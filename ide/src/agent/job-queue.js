/**
 * JobQueue - 异步作业调度系统
 * 
 * 解决主循环阻塞问题：
 * 1. 提交 job 后立即返回 jobId (非阻塞)
 * 2. 后台并发执行多个子智能体
 * 3. 通过事件通知完成状态
 * 4. token 消耗监控与钳位
 */

import getSharedStore from './shared-store.js';

class JobQueue {
  constructor(options = {}) {
    this.jobs = new Map(); // jobId -> JobConfig
    this.pendingPromises = new Map(); // jobId -> Set<{resolve,reject}>
    this.maxConcurrent = options.maxConcurrent || parseInt(import.meta.env?.MAX_SUBAGENTS_CONCURRENT || '5');
    this.tokenBudget = Math.max(1, Number(options.tokenBudget || 100000));
    this.tokenWarningThreshold = options.tokenWarningThreshold || 80;
    this.runner = typeof options.runner === 'function' ? options.runner : null;
    this.historyTTL = Math.max(0, Number(options.historyTTL ?? 3600000));
    
    this.sharedStore = options.sharedStore || getSharedStore();
    
    this.activeCount = 0;
    this.totalExecuted = 0;
    this.totalTokens = 0;
    
    // 事件总线
    this.eventHandlers = new Map();
    
    // 启动监控器
    this._startMonitoring();
    
    console.log('[JobQueue] Created with maxConcurrent:', this.maxConcurrent);
  }

  /**
   * 提交新 job
   * @param {Object} config 
   * @returns {string} jobId
   */
  async submit(config) {
    const {
      id: providedId,
      tool,
      args,
      role,
      priority = 1,
      onProgress,
      onComplete,
      tools: agentTools,
      sharedContext,
      runner: jobRunner,
      runnerConfig: explicitRunnerConfig,
      ...runnerConfig
    } = config;
    
    // 生成或接受自定义 jobId
    const jobId = providedId || `job_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    
    // 在 SharedStore 创建记录
    this.sharedStore.createJob({
      tool,
      role,
      priority,
      args: JSON.stringify(args),
      agentTools,
      sharedContext: sharedContext ? Object.keys(sharedContext).slice(0, 10).join(', ') : 'none'
    }, jobId);
    
    const jobConfig = {
      id: jobId,
      tool,
      args,
      role,
      priority,
      onProgress,
      onComplete,
      agentTools,
      runner: typeof jobRunner === 'function' ? jobRunner : this.runner,
      runnerConfig: { ...(runnerConfig || {}), ...(explicitRunnerConfig || {}) },
      submittedAt: Date.now(),
      startedAt: null,
      completedAt: null,
      tokensUsed: 0,
      status: 'pending',
      abortController: typeof AbortController === 'function' ? new AbortController() : null,
      waiters: new Set()
    };
    
    this.jobs.set(jobId, jobConfig);
    
    // 立即返回，不 await
    this._scheduleJob(jobConfig);
    
    return jobId;
  }

  /**
   * 取消 job
   */
  async abort(jobId) {
    const job = this.jobs.get(jobId);
    if (!job) return false;
    if (['completed', 'failed', 'aborted'].includes(job.status)) return false;
    job.abort = true;
    try { job.abortController?.abort(); } catch {}
    // Queued jobs have not entered the runner yet and can settle immediately.
    // Running jobs settle after the injected runner returns so the concurrency
    // counter remains truthful while it releases resources.
    if (job.status === 'pending') this._settleJob(job, 'aborted', null, new Error('Job aborted by user'));
    return true;
  }

  /**
   * 扩展 job steps
   */
  extendJob(jobId, extraSteps) {
    const job = this.jobs.get(jobId);
    if (!job || job.status !== 'running') return false;
    
    // 向 SharedStore 发送信号
    this.sharedStore.appendFinding(jobId, {
      source: 'user',
      type: 'extension_request',
      data: { extraSteps },
      isExternal: false
    });
    
    return true;
  }

  /**
   * 获取所有活跃 jobs
   */
  getActiveJobs() {
    return Array.from(this.jobs.values()).filter(
      job => job.status === 'running' || job.status === 'pending'
    );
  }

  /**
   * 等待特定 job 完成
   */
  waitForJob(jobId, timeout) {
    return new Promise((resolve, reject) => {
      const job = this.jobs.get(jobId);
      if (!job) {
        reject(new Error(`Job ${jobId} not found`));
        return;
      }
      
      if (job.status === 'completed') {
        resolve(job.result);
        return;
      }
      if (['failed', 'aborted'].includes(job.status)) {
        reject(job.error instanceof Error ? job.error : new Error(String(job.error || `Job ${jobId} ${job.status}`)));
        return;
      }

      const waiter = { resolve, reject, timer: null };
      if (timeout) {
        waiter.timer = setTimeout(() => {
          job.waiters.delete(waiter);
          reject(new Error(`Job ${jobId} timed out after ${timeout}ms`));
        }, Math.max(1, Number(timeout)));
      }
      job.waiters.add(waiter);
      this.pendingPromises.set(jobId, job.waiters);
    });
  }

  /**
   * 获取统计信息
   */
  stats() {
    return {
      totalJobs: this.jobs.size,
      activeJobs: this.getActiveJobs().length,
      completedJobs: this.totalExecuted,
      currentConcurrency: this.activeCount,
      maxConcurrency: this.maxConcurrent,
      totalTokens: this.totalTokens,
      tokenBudget: this.tokenBudget,
      tokenWarningThreshold: this.tokenWarningThreshold + '%'
    };
  }

  /**
   * 停止所有 jobs
   */
  async stopAll() {
    const promises = [];
    for (const [jobId] of this.jobs.entries()) {
      promises.push(this.abort(jobId));
    }
    await Promise.all(promises);
    return true;
  }

  /**
   * 清空历史数据
   */
  clearHistory() {
    const active = new Set(this.getActiveJobs().map((job) => job.id));
    let removed = 0;
    for (const [jobId] of this.jobs.entries()) {
      if (!active.has(jobId)) {
        this.jobs.delete(jobId);
        this.pendingPromises.delete(jobId);
        this.sharedStore.delete(`jobs.${jobId}`);
        removed++;
      }
    }
    return removed;
  }

  // ==================== 私有方法 ====================

  _scheduleJob(jobConfig) {
    // 检查是否需要等待
    if (this.activeCount >= this.maxConcurrent) {
      console.log(`[JobQueue] Queue full (${this.activeCount}/${this.maxConcurrent}), waiting...`);
      
      // 等待有空闲槽位
      this._waitForSlot().then(() => {
        this._executeJob(jobConfig);
      });
      return;
    }
    
    // 立即执行
    this._executeJob(jobConfig);
  }

  async _waitForSlot() {
    return new Promise(resolve => {
      const check = () => {
        if (this.activeCount < this.maxConcurrent) {
          resolve();
        } else {
          setTimeout(check, 100);
        }
      };
      check();
    });
  }

  async _executeJob(jobConfig) {
    if (jobConfig.abort) {
      this._settleJob(jobConfig, 'aborted', null, new Error('Job aborted before start'));
      return;
    }
    this.activeCount++;
    jobConfig.status = 'running';
    jobConfig.startedAt = Date.now();
    
    this.sharedStore.updateJobStatus(jobConfig.id, 'running', 0);
    this.publish('job_started', jobConfig);
    
    try {
      const result = await this._runAgent(jobConfig);
      if (jobConfig.abort) {
        this._settleJob(jobConfig, 'aborted', null, new Error('Job aborted by user'));
      } else {
        this._settleJob(jobConfig, 'completed', result);
        if (jobConfig.onComplete) {
          try { await jobConfig.onComplete(result); }
          catch (err) { console.error('[JobQueue] onComplete callback error:', err); }
        }
      }
    } catch (error) {
      console.error('[JobQueue] Job failed:', jobConfig.id, error);
      this._settleJob(jobConfig, 'failed', null, error);
    } finally {
      this.activeCount--;
    }
  }

  async _runAgent(jobConfig) {
    const runner = jobConfig.runner;
    if (typeof runner !== 'function') {
      throw new Error('JobQueue requires an injected runner; refusing to simulate an agent job');
    }
    const reportProgress = (update = {}) => {
      const progress = Number.isFinite(Number(update.progress))
        ? Math.max(0, Math.min(100, Number(update.progress)))
        : undefined;
      if (progress !== undefined) this.sharedStore.updateJobStatus(jobConfig.id, 'running', progress);
      if (update.finding) this.sharedStore.appendFinding(jobConfig.id, update.finding);
      if (typeof jobConfig.onProgress === 'function') {
        try { jobConfig.onProgress(update); }
        catch (error) { console.error('[JobQueue] onProgress callback error:', error); }
      }
    };
    const result = await runner(jobConfig, {
      args: jobConfig.args,
      config: jobConfig.runnerConfig,
      signal: jobConfig.abortController?.signal,
      isAborted: () => !!jobConfig.abort,
      reportProgress,
    });
    const used = Number(result?.tokensUsed);
    if (Number.isFinite(used) && used >= 0) {
      jobConfig.tokensUsed = used;
      this.totalTokens += used;
      if (this.totalTokens >= this.tokenBudget * this.tokenWarningThreshold / 100) {
        this.publish('token_warning', { total: this.totalTokens, threshold: this.tokenWarningThreshold });
      }
    }
    return result;
  }

  _settleJob(jobConfig, status, result = null, error = null) {
    if (!jobConfig || ['completed', 'failed', 'aborted'].includes(jobConfig.status)) return;
    jobConfig.status = status;
    jobConfig.completedAt = Date.now();
    if (status === 'completed') {
      jobConfig.result = result;
      this.totalExecuted++;
      this.sharedStore.updateJobStatus(jobConfig.id, 'completed', 100);
      this.sharedStore.appendFinding(jobConfig.id, {
        source: 'agent', type: 'completion',
        data: { summary: result?.summary || '', findingsCount: result?.findings?.length || 0 }
      });
      this.publish('job_completed', {
        id: jobConfig.id,
        duration: jobConfig.completedAt - jobConfig.startedAt,
        result
      });
    } else {
      jobConfig.error = error instanceof Error ? error : new Error(String(error || status));
      this.sharedStore.updateJobStatus(jobConfig.id, status);
      this.sharedStore.appendFinding(jobConfig.id, {
        source: 'agent', type: 'error', data: jobConfig.error.message
      });
      this.publish(status === 'aborted' ? 'job_aborted' : 'job_failed', {
        id: jobConfig.id, error: jobConfig.error.message
      });
    }
    const waiters = jobConfig.waiters || this.pendingPromises.get(jobConfig.id) || [];
    for (const waiter of waiters) {
      if (waiter.timer) clearTimeout(waiter.timer);
      if (status === 'completed') waiter.resolve(result);
      else waiter.reject(jobConfig.error || new Error(`Job ${jobConfig.id} ${status}`));
    }
    jobConfig.waiters?.clear();
    this.pendingPromises.delete(jobConfig.id);
    if (this.historyTTL > 0) {
      const timer = setTimeout(() => {
        const current = this.jobs.get(jobConfig.id);
        if (current === jobConfig && ['completed', 'failed', 'aborted'].includes(current.status)) this.jobs.delete(jobConfig.id);
      }, this.historyTTL);
      if (typeof timer?.unref === 'function') timer.unref();
    }
  }

  _startMonitoring() {
    // 每分钟输出统计
    const timer = setInterval(() => {
      const stats = this.stats();
      console.log('[JobQueue] Stats:', stats);
    }, 60000);
    if (typeof timer?.unref === 'function') timer.unref();
  }

  // 事件系统
  _on(event, handler) {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, []);
    }
    this.eventHandlers.get(event).push(handler);
  }

  _off(event, handler) {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      const idx = handlers.indexOf(handler);
      if (idx > -1) handlers.splice(idx, 1);
    }
  }

  publish(event, data) {
    const handlers = this.eventHandlers.get(event) || [];
    handlers.forEach(handler => {
      try {
        handler(data);
      } catch (err) {
        console.error('[JobQueue] Event handler error:', err);
      }
    });
  }

}

// ==================== 全局实例 ====================

let jobQueueInstance = null;

function getJobQueue(options = {}) {
  if (!jobQueueInstance) {
    jobQueueInstance = new JobQueue(options);
  }
  return jobQueueInstance;
}

export { JobQueue };
export default getJobQueue;
