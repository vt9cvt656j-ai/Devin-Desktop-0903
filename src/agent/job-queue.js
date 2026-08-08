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
    const configuredConcurrency = Number(options.maxConcurrent ?? import.meta.env?.MAX_SUBAGENTS_CONCURRENT ?? 5);
    this.maxConcurrent = Number.isFinite(configuredConcurrency)
      ? Math.max(1, Math.floor(configuredConcurrency))
      : 5;
    this.tokenBudget = Math.max(1, Number(options.tokenBudget || 100000));
    this.tokenWarningThreshold = options.tokenWarningThreshold || 80;
    this.runner = typeof options.runner === 'function' ? options.runner : null;
    this.historyTTL = Math.max(0, Number(options.historyTTL ?? 3600000));
    
    this.sharedStore = options.sharedStore || getSharedStore();
    
    this.activeCount = 0;
    this.pendingJobs = [];
    this.totalExecuted = 0;
    this.totalTokens = 0;
    this.stopped = false;
    
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
    if (this.stopped) {
      throw new Error('JobQueue is stopped; no new submissions accepted');
    }
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
    if (job.status === 'pending') {
      const pendingIndex = this.pendingJobs.indexOf(job);
      if (pendingIndex >= 0) this.pendingJobs.splice(pendingIndex, 1);
      this._settleJob(job, 'aborted', null, new Error('Job aborted by user'));
    } else if (job.executionPromise) {
      // A third-party runner may ignore AbortSignal forever. _runAgent races the
      // signal, so this waits only for our queue wrapper to release its slot.
      await job.executionPromise;
    }
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
    this.stopped = true;
    if (this.monitorTimer) {
      clearInterval(this.monitorTimer);
      this.monitorTimer = null;
    }
    const promises = [];
    for (const [jobId] of this.jobs.entries()) {
      promises.push(this.abort(jobId));
    }
    await Promise.all(promises);
    this.pendingJobs.length = 0;
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
    this.pendingJobs.push(jobConfig);
    this._drainQueue();
  }

  _drainQueue() {
    while (!this.stopped && this.activeCount < this.maxConcurrent && this.pendingJobs.length) {
      const jobConfig = this.pendingJobs.shift();
      if (!jobConfig || jobConfig.abort || jobConfig.status !== 'pending') continue;
      // Reserve the slot synchronously. If several jobs are queued, no two
      // continuations can observe the same free slot and oversubscribe it.
      this.activeCount++;
      const execution = this._executeJob(jobConfig);
      jobConfig.executionPromise = execution;
      void execution.finally(() => {
        if (jobConfig.executionPromise === execution) jobConfig.executionPromise = null;
      });
    }
  }

  async _executeJob(jobConfig) {
    if (jobConfig.abort) {
      this._settleJob(jobConfig, 'aborted', null, new Error('Job aborted before start'));
      this.activeCount--;
      this._drainQueue();
      return;
    }
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
          Promise.resolve().then(() => jobConfig.onComplete(result))
            .catch((err) => console.error('[JobQueue] onComplete callback error:', err));
        }
      }
    } catch (error) {
      if (jobConfig.abort) {
        this._settleJob(jobConfig, 'aborted', null, error);
      } else {
        console.error('[JobQueue] Job failed:', jobConfig.id, error);
        this._settleJob(jobConfig, 'failed', null, error);
      }
    } finally {
      this.activeCount--;
      this._drainQueue();
    }
  }

  async _runAgent(jobConfig) {
    const runner = jobConfig.runner;
    if (typeof runner !== 'function') {
      throw new Error('JobQueue requires an injected runner; refusing to simulate an agent job');
    }
    const reportProgress = (update = {}) => {
      if (jobConfig.abort || ['completed', 'failed', 'aborted'].includes(jobConfig.status)) return;
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
    const runnerPromise = Promise.resolve().then(() => runner(jobConfig, {
      args: jobConfig.args,
      config: jobConfig.runnerConfig,
      signal: jobConfig.abortController?.signal,
      isAborted: () => !!jobConfig.abort,
      reportProgress,
    }));
    const signal = jobConfig.abortController?.signal;
    let result;
    if (!signal) {
      result = await runnerPromise;
    } else if (signal.aborted) {
      throw new Error('Job aborted by user');
    } else {
      let onAbort;
      const aborted = new Promise((_, reject) => {
        onAbort = () => reject(new Error('Job aborted by user'));
        signal.addEventListener('abort', onAbort, { once: true });
      });
      try {
        result = await Promise.race([runnerPromise, aborted]);
      } finally {
        signal.removeEventListener('abort', onAbort);
      }
    }
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
    this.monitorTimer = setInterval(() => {
      const stats = this.stats();
      console.log('[JobQueue] Stats:', stats);
    }, 60000);
    if (typeof this.monitorTimer?.unref === 'function') this.monitorTimer.unref();
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
