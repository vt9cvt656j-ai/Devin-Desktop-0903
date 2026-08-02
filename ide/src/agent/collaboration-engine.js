/**
 * CollaborationEngine - 多智能体智能协同引擎 (P1 Implementation)
 * 
 * 支持三种协作模式:
 * 1. SharedStore: 基于共享状态的数据交换
 * 2. EventBus: 事件驱动的异步通信
 * 3. Lead-Follower: 主从协调模式
 * 
 * 包含上下文增强和 Token 预算控制
 */

import getSharedStore from './shared-store.js';

class CollaborationEngine {
  constructor(options = {}) {
    this.store = options.store || getSharedStore();
    this.mode = options.mode || 'shared_store';
    this.config = {
      broadcastThreshold: 5, // 每 N 条 findings 广播一次
      maxContextSize: 8000,  // 最大上下文大小 (bytes)
      fileSnippetsCount: 3,  // 每次返回的片段数
      tokenBudget: options.tokenBudget || 100000, // Token 预算
      warningThreshold: options.warningThreshold || 80, // 警告阈值 %
      ...options.config
    };
    
    this.activeCollaborations = new Map();
    this.eventSubscriptions = new Map();
    this.tokenTracking = new Map();
    
    console.log(`[CollaborationEngine] Initialized with mode: ${this.mode}`);
  }
  
  /**
   * 初始化协作会话
   */
  startSession(sessionId, jobIds, config = {}) {
    const sessionConfig = {
      mode: this.mode,
      createdAt: Date.now(),
      jobIds,
      status: 'active',
      tokensUsed: 0,
      contextData: {},
      ...config
    };
    
    this.store.set(`collab_sessions.${sessionId}`, sessionConfig);
    this.activeCollaborations.set(sessionId, sessionConfig);
    
    // 根据模式设置对应的协作规则
    switch (this.mode) {
      case 'shared_store':
        this.setupSharedStoreCollaboration(sessionId, jobIds);
        break;
        
      case 'eventbus':
        this.setupEventBusCollaboration(sessionId, jobIds);
        break;
        
      case 'lead_follower':
        this.setupLeadFollowerCollaboration(sessionId, jobIds, config);
        break;
    }
    
    return sessionId;
  }
  
  /**
   * ========== 模式 1: SharedStore (数据共享) ==========
   */
  setupSharedStoreCollaboration(sessionId, jobIds) {
    console.log(`[SharedStore] Setting up collaboration for ${jobIds.length} jobs`);
    
    jobIds.forEach(sourceJobId => {
      // 监听每个 job 的 findings 更新
      this.store.on(`jobs.${sourceJobId}.findings`, (findings) => {
        if (!Array.isArray(findings) || findings.length === 0) return;
        
        // 只广播最新的关键信息
        const latestFindings = findings.slice(-this.config.broadcastThreshold);
        
        if (!latestFindings.length) return;
        
        // 广播给其他所有 jobs
        jobIds.forEach(targetJobId => {
          if (targetJobId !== sourceJobId) {
            this.store.appendFinding(targetJobId, {
              source: sourceJobId,
              channel: 'shared_store',
              data: latestFindings.map(f => ({
                type: f.type,
                content: f.content?.slice(0, 200), // 限制内容长度
                timestamp: f.timestamp
              })),
              isExternal: true,
              sessionId
            });
          }
        });
        
        // 跟踪 token 使用
        this.trackTokenUsage(sessionId, latestFindings.length * 50);
      });
    });
  }
  
  /**
   * ========== 模式 2: EventBus (事件驱动) ==========
   */
  setupEventBusCollaboration(sessionId, jobIds) {
    console.log(`[EventBus] Setting up collaboration for ${jobIds.length} jobs`);
    
    // 为每个 job 创建独立的频道
    jobIds.forEach(jobId => {
      const channel = `collab:${sessionId}:${jobId}`;
      
      // 订阅该频道的所有事件
      this.eventSubscriptions.set(channel, (event) => {
        // 广播给其他 jobs
        jobIds.forEach(otherJobId => {
          if (otherJobId !== jobId) {
            const otherChannel = `collab:${sessionId}:${otherJobId}`;
            
            this.store.publish(otherChannel, {
              type: event.type,
              payload: event.payload,
              sourceJobId: jobId,
              timestamp: Date.now()
            });
          }
        });
      });
    });
    
    // 提供发布 API
    this.activeCollaborations.get(sessionId).publish = (jobId, eventType, payload) => {
      const channel = `collab:${sessionId}:${jobId}`;
      this.store.publish(channel, { type: eventType, payload, timestamp: Date.now() });
    };
  }
  
  /**
   * ========== 模式 3: Lead-Follower (主从协调) ==========
   */
  setupLeadFollowerCollaboration(sessionId, jobIds, config = {}) {
    if (!config.leadJobId) {
      throw new Error('Lead-Follower 模式必须指定 leadJobId');
    }
    
    const leadJobId = config.leadJobId;
    const followerJobIds = jobIds.filter(id => id !== leadJobId);
    
    console.log(`[Lead-Follower] Lead: ${leadJobId}, Followers: ${followerJobIds.length}`);
    
    // 监听 lead job 的决策和状态变化
    this.store.on(`jobs.${leadJobId}.status`, (status) => {
      // 当 lead job 完成某个关键阶段时，通知 followers
      if (status === 'phase_complete' || status === 'completed') {
        const decision = this.store.get(`jobs.${leadJobId}.decision`);
        
        if (decision) {
          followerJobIds.forEach(followerId => {
            // 继承 lead 的决策
            this.store.updateJobContext(followerId, {
              parentDecision: decision,
              inheritFrom: leadJobId,
              updatedAt: Date.now()
            });
            
            // 发送通知
            this.store.appendFinding(followerId, {
              source: leadJobId,
              channel: 'lead_decision',
              data: decision,
              isExternal: true
            });
          });
        }
      }
    });
    
    // Follower 可以向 Lead 请求指导
    this.registerFollowerRequestHandler(sessionId, leadJobId, followerJobIds);
  }
  
  registerFollowerRequestHandler(sessionId, leadJobId, followerJobIds) {
    // 监听 follower 的请求
    followerJobIds.forEach(followerId => {
      this.store.on(`jobs.${followerId}.request_guidance`, (request) => {
        // Lead job 接收请求并提供指导
        this.store.appendFinding(leadJobId, {
          type: 'guidance_request',
          from: followerId,
          data: request,
          timestamp: Date.now()
        });
      });
    });
  }
  
  /**
   * ========== 上下文增强 ==========
   */
  
  /**
   * 增强子智能体的上下文，包含文件片段
   */
  async enhanceContext(jobId, baseContext, filesToInclude = []) {
    const enhanced = {
      ...baseContext,
      _enhancedAt: Date.now(),
      _source: 'CollaborationEngine'
    };
    
    // 添加相关文件的内容片段
    if (filesToInclude.length > 0) {
      const snippets = await this.extractFileSnippets(filesToInclude);
      enhanced.fileSnippets = snippets;
      enhanced.totalSnippetBytes = snippets.reduce((sum, s) => sum + s.content.length, 0);
    }
    
    // 添加相关 findings (来自其他协作的 jobs)
    const relatedFindings = await this.collectRelatedFindings(jobId);
    enhanced.relatedFindings = relatedFindings.slice(-10); // 最新 10 条
    
    // 添加共享知识
    const sharedKnowledge = this.store.get(`collab_sessions.*.knowledge`, {});
    if (Object.keys(sharedKnowledge).length > 0) {
      enhanced.sharedKnowledge = sharedKnowledge;
    }
    
    // 限制总大小
    const totalSize = JSON.stringify(enhanced).length;
    if (totalSize > this.config.maxContextSize) {
      console.warn(`[CollaborationEngine] Context too large (${totalSize} bytes), truncating`);
      enhanced._truncated = true;
      enhanced._originalSize = totalSize;
    }
    
    return enhanced;
  }
  
  /**
   * 提取文件片段
   */
  async extractFileSnippets(filePaths) {
    const snippets = [];
    
    for (const filePath of filePaths) {
      try {
        // 读取文件内容 (实际应用中应该从 backend 获取)
        const content = await this.readFileSync(filePath);
        
        if (!content) continue;
        
        const lines = content.split('\n');
        
        snippets.push({
          path: filePath,
          totalLines: lines.length,
          linesRead: Math.min(lines.length, 100), // 最多读 100 行
          content: content.slice(0, 2000), // 最多 2KB
          summary: this.generateLineSummary(lines)
        });
        
      } catch (error) {
        console.error(`[CollaborationEngine] Failed to read ${filePath}:`, error);
      }
    }
    
    return snippets.slice(0, this.config.fileSnippetsCount);
  }
  
  generateLineSummary(lines) {
    if (!lines || lines.length === 0) return null;
    
    const firstFew = lines.slice(0, 5).join(' ');
    const lastFew = lines.slice(-5).join(' ');
    
    return `First: ${firstFew}...\nLast: ...${lastFew}`;
  }
  
  async readFileSync(path) {
    // 在实际应用中，这里应该调用 Tauri/Backend API
    // 为了演示，返回 mock 数据
    return null;
  }
  
  /**
   * 收集相关的 findings (来自其他协作的 jobs)
   */
  async collectRelatedFindings(currentJobId) {
    const allFindings = [];
    
    // 查询 SharedStore 中与该 job 相关的发现
    const relatedJobs = this.store.query('jobs.*');
    
    for (const { key, value } of relatedJobs) {
      if (key.includes('.findings.') && key !== `jobs.${currentJobId}.findings`) {
        const findings = Array.isArray(value) ? value : [];
        allFindings.push(...findings.map(f => ({
          ...f,
          sourceJobId: key.split('.')[1]
        })));
      }
    }
    
    return allFindings.sort((a, b) => (b.timestamp || 0) - (a.timestamp || 0));
  }
  
  /**
   * ========== Token 预算管理 ==========
   */
  
  /**
   * 跟踪 token 使用
   */
  trackTokenUsage(sessionId, count) {
    if (!this.tokenTracking.has(sessionId)) {
      this.tokenTracking.set(sessionId, { used: 0, sessions: [] });
    }
    
    const tracking = this.tokenTracking.get(sessionId);
    tracking.used += count;
    
    // 更新 store
    this.store.set(`token_usage.${sessionId}`, {
      total: tracking.used,
      limit: this.config.tokenBudget,
      percentage: (tracking.used / this.config.tokenBudget * 100).toFixed(2) + '%',
      warningThreshold: this.config.warningThreshold + '%'
    });
    
    // 检查是否超过阈值
    const usagePercentage = tracking.used / this.config.tokenBudget * 100;
    if (usagePercentage >= this.config.warningThreshold) {
      this.triggerTokenWarning(sessionId, tracking.used, this.config.tokenBudget);
    }
  }
  
  /**
   * 触发 Token 警告
   */
  triggerTokenWarning(sessionId, used, limit) {
    console.warn(`[TokenWarning] Session ${sessionId} using ${used.toLocaleString()} / ${limit.toLocaleString()} tokens (${((used/limit*100)).toFixed(1)}%)`);
    
    // 发布警告事件
    this.store.publish('token_warning', {
      sessionId,
      used,
      limit,
      percentage: (used / limit * 100).toFixed(1)
    });
    
    // 添加到 session 记录
    const session = this.activeCollaborations.get(sessionId);
    if (session) {
      session.tokensUsed = used;
      session.tokenWarning = true;
      this.store.set(`collab_sessions.${sessionId}`, session);
    }
  }
  
  /**
   * 获取 Token 使用统计
   */
  getTokenStats(sessionId) {
    const usage = this.tokenTracking.get(sessionId);
    const limit = this.config.tokenBudget;
    
    if (!usage) {
      return { used: 0, limit, percentage: '0%', status: 'normal' };
    }
    
    const percentage = (usage.used / limit * 100).toFixed(1);
    let status = 'normal';
    
    if (percentage >= 90) status = 'critical';
    else if (percentage >= this.config.warningThreshold) status = 'warning';
    
    return {
      used: usage.used,
      limit,
      remaining: limit - usage.used,
      percentage: percentage + '%',
      status
    };
  }
  
  /**
   * 自动优化 Token 使用
   */
  autoOptimizeForTokenBudget(sessionId, aggressive = false) {
    const stats = this.getTokenStats(sessionId);
    
    if (stats.status === 'normal') return;
    
    console.log(`[TokenOptimization] Session ${sessionId} at ${stats.percentage}, mode: ${aggressive ? 'aggressive' : 'conservative'}`);
    
    // 采取行动减少后续 token 消耗
    const optimizations = [
      { action: 'reduceBroadcastFrequency', threshold: 80 },
      { action: 'limitContextSize', threshold: 90, aggressive },
      { action: 'skipLowPriorityTasks', threshold: 95, aggressive }
    ];
    
    const applicableOptimizations = optimizations.filter(
      opt => stats.percentage.replace('%', '') >= opt.threshold
    );
    
    applicableOptimizations.forEach(opt => {
      console.log(`[TokenOptimization] Applying: ${opt.action}`);
      this.applyOptimization(sessionId, opt.action, opt.aggressive);
    });
  }
  
  applyOptimization(sessionId, action, aggressive) {
    switch (action) {
      case 'reduceBroadcastFrequency':
        this.config.broadcastThreshold = Math.max(1, Math.floor(this.config.broadcastThreshold / 2));
        break;
        
      case 'limitContextSize':
        this.config.maxContextSize = aggressive ? 4000 : 6000;
        break;
        
      case 'skipLowPriorityTasks':
        // Mark low priority tasks as skipped
        const session = this.activeCollaborations.get(sessionId);
        if (session) {
          session.optimizationApplied = action;
        }
        break;
    }
  }
  
  /**
   * ========== 实用工具 ==========
   */
  
  /**
   * 结束协作会话
   */
  endSession(sessionId) {
    const session = this.activeCollaborations.get(sessionId);
    if (!session) return false;
    
    session.status = 'completed';
    session.completedAt = Date.now();
    session.finalStats = {
      jobsCollaborated: session.jobIds.length,
      tokensUsed: session.tokensUsed || 0,
      duration: session.completedAt - session.createdAt
    };
    
    this.store.set(`collab_sessions.${sessionId}`, session);
    this.activeCollaborations.delete(sessionId);
    
    return true;
  }
  
  /**
   * 获取所有活跃会话
   */
  getActiveSessions() {
    return Array.from(this.activeCollaborations.values())
      .filter(s => s.status === 'active')
      .map(s => ({
        sessionId: Object.keys(this.activeCollaborations).find(key => 
          this.activeCollaborations.get(key) === s
        ),
        ...s
      }));
  }
  
  /**
   * 广播消息给特定频道
   */
  broadcast(channel, message) {
    this.store.publish(channel, message);
  }
  
  /**
   * 添加共享知识
   */
  addSharedKnowledge(sessionId, key, value) {
    const session = this.activeCollaborations.get(sessionId);
    if (!session) return false;
    
    if (!session.knowledge) session.knowledge = {};
    session.knowledge[key] = value;
    
    this.store.set(`collab_sessions.${sessionId}.knowledge.${key}`, value);
    
    // 广播给所有相关 jobs
    session.jobIds.forEach(jobId => {
      this.store.appendFinding(jobId, {
        source: 'shared_knowledge',
        channel: 'knowledge_update',
        data: { key, value },
        isExternal: true
      });
    });
    
    return true;
  }
}

// ===== Factory Function =====
export function createCollaborationEngine(options = {}) {
  return new CollaborationEngine(options);
}

// ===== Singleton Pattern =====
let collaborationEngineInstance = null;

export function getCollaborationEngine(options = {}) {
  if (!collaborationEngineInstance) {
    collaborationEngineInstance = new CollaborationEngine(options);
  }
  return collaborationEngineInstance;
}

export default CollaborationEngine;
