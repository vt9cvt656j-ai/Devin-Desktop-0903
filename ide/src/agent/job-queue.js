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
    this.pendingPromises = new Map(); // jobId -> Promise + resolve/reject
    this.maxConcurrent = options.maxConcurrent || parseInt(import.meta.env?.MAX_SUBAGENTS_CONCURRENT || '5');
    this.tokenWarningThreshold = options.tokenWarningThreshold || 80;
    
    this.sharedStore = getSharedStore();
    
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
    });
    
    const jobConfig = {
      id: jobId,
      tool,
      args,
      role,
      priority,
      onProgress,
      onComplete,
      agentTools,
      runnerConfig,
      submittedAt: Date.now(),
      startedAt: null,
      completedAt: null,
      tokensUsed: 0,
      status: 'pending'
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
    
    job.abort = true;
    this.sharedStore.updateJobStatus(jobId, 'aborted');
    
    // 解析 pending promise
    const pending = this.pendingPromises.get(jobId);
    if (pending) {
      pending.reject?.(new Error('Job aborted by user'));
      this.pendingPromises.delete(jobId);
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
      
      const timeoutId = timeout ? setTimeout(() => {
        reject(new Error(`Job ${jobId} timed out after ${timeout}ms`));
      }, timeout) : null;
      
      // 注册一次性回调
      this._onCompleteOnce(jobId, (result) => {
        clearTimeout(timeoutId);
        resolve(result);
      }, reject);
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
    const active = this.getActiveJobs();
    const toDelete = Array.from(this.jobs.keys())
      .filter(id => !active.find(j => j.jobId === id));
    
    this.delete(toDelete);
    return toDelete.length;
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
    this.activeCount++;
    jobConfig.status = 'running';
    jobConfig.startedAt = Date.now();
    
    this.sharedStore.updateJobStatus(jobConfig.id, 'running', 0);
    this.publish('job_started', jobConfig);
    
    try {
      // 这里应该调用实际的 agent runner
      // 由于是纯 JS 实现，我们先模拟一个流程
      const result = await this._runAgent(jobConfig);
      
      jobConfig.status = 'completed';
      jobConfig.completedAt = Date.now();
      jobConfig.result = result;
      jobConfig.tokensUsed = this.totalTokens;
      
      this.sharedStore.updateJobStatus(jobConfig.id, 'completed', 100);
      this.sharedStore.appendFinding(jobConfig.id, {
        source: 'agent',
        type: 'completion',
        data: { summary: result.summary, findingsCount: result.findings?.length || 0 }
      });
      
      this.totalExecuted++;
      
      this.publish('job_completed', {
        id: jobConfig.id,
        duration: jobConfig.completedAt - jobConfig.startedAt,
        result
      });
      
      // 触发回调
      if (jobConfig.onComplete) {
        try {
          await jobConfig.onComplete(result);
        } catch (err) {
          console.error('[JobQueue] onComplete callback error:', err);
        }
      }
      
      // 通知等待该 job 的 promise
      const pending = this.pendingPromises.get(jobConfig.id);
      if (pending && pending.resolve) {
        pending.resolve(result);
        this.pendingPromises.delete(jobConfig.id);
      }
      
    } catch (error) {
      jobConfig.status = 'failed';
      jobConfig.error = error.message;
      
      this.sharedStore.updateJobStatus(jobConfig.id, 'failed');
      this.sharedStore.appendFinding(jobConfig.id, {
        source: 'agent',
        type: 'error',
        data: error.message
      });
      
      console.error('[JobQueue] Job failed:', jobConfig.id, error);
      
      const pending = this.pendingPromises.get(jobConfig.id);
      if (pending && pending.reject) {
        pending.reject(error);
        this.pendingPromises.delete(jobConfig.id);
      }
      
    } finally {
      this.activeCount--;
      this.jobs.delete(jobConfig.id); // 完成后删除，避免内存泄漏
    }
  }

  async _runAgent(jobConfig) {
    /**
     * ⚠️ 这是一个 stub 实现
     * 真实逻辑需要从 Rust 端调用 AgentRunner
     * 
     * 简化模拟流程:
     * 1. 每步更新进度
     * 2. 收集 findings
     * 3. 最后汇总报告
     */
    
    const maxSteps = jobConfig.runnerConfig.maxSteps || 20;
    let findings = [];
    
    for (let step = 1; step <= maxSteps; step++) {
      // 检查是否取消
      if (jobConfig.abort) {
        throw new Error('Job aborted');
      }
      
      // 模拟进度
      const progress = Math.round((step / maxSteps) * 100);
      this.sharedStore.updateJobStatus(jobConfig.id, 'running', progress);
      
      // 模拟生成一些 findings
      if (step % 2 === 0) {
        const finding = {
          step,
          type: 'discovery',
          content: `Analyzing ${jobConfig.role} task at step ${step}...`,
          confidence: 0.7 + Math.random() * 0.3
        };
        
        findings.push(finding);
        this.sharedStore.appendFinding(jobConfig.id, finding);
        
        if (jobConfig.onProgress) {
          jobConfig.onProgress({
            step,
            progress,
            findings: [finding]
          });
        }
      }
      
      // 模拟 token 消耗
      this.totalTokens += Math.floor(Math.random() * 100) + 50;
      jobConfig.tokensUsed = this.totalTokens;
      
      // 检查 token 阈值
      if (this.totalTokens > 100000 && this.totalTokens / 100000 >= this.tokenWarningThreshold / 100) {
        console.warn(`[JobQueue] Token warning: ${this.totalTokens.toLocaleString()} tokens used`);
        this.publish('token_warning', { total: this.totalTokens, threshold: this.tokenWarningThreshold });
      }
      
      // 模拟延迟 (实际应该更快)
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    
    return {
      summary: `Completed ${jobConfig.role} task after ${maxSteps} steps`,
      findings: findings.slice(-20), // 返回最后 20 条
      stepsExecuted: maxSteps,
      tokensUsed: jobConfig.tokensUsed
    };
  }

  _startMonitoring() {
    // 每分钟输出统计
    setInterval(() => {
      const stats = this.stats();
      console.log('[JobQueue] Stats:', stats);
    }, 60000);
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

  _onCompleteOnce(jobId, resolve, reject) {
    const handler = (data) => {
      if (data.id === jobId) {
        this._off('job_completed', handler);
        resolve(data.result);
      }
    };
    
    const errorHandler = (data) => {
      if (data.id === jobId) {
        this._off('job_error', errorHandler);
        reject(new Error(data.error));
      }
    };
    
    this._on('job_completed', handler);
    this._on('job_failed', errorHandler);
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
