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
    this.readFile = typeof options.readFile === 'function' ? options.readFile : null;
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
    this.sessionUnsubscribers = new Map();
    this.tokenTracking = new Map();
    
  }
  
  /**
   * 初始化协作会话
   */
  startSession(sessionId, jobIds, config = {}) {
    const mode = config.mode || this.mode;
    const sessionConfig = {
      mode,
      createdAt: Date.now(),
      jobIds,
      status: 'active',
      tokensUsed: 0,
      contextData: {},
      knowledge: {},
      ...config
    };

    this.store.set(`collab_sessions.${sessionId}`, sessionConfig);
    this.activeCollaborations.set(sessionId, sessionConfig);

    switch (mode) {
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
    const seenCounts = new Map(jobIds.map((jobId) => [
      jobId,
      Array.isArray(this.store.get(`jobs.${jobId}`)?.findings)
        ? this.store.get(`jobs.${jobId}`).findings.length
        : 0,
    ]));
    
    jobIds.forEach(sourceJobId => {
      // 监听每个 job 的 findings 更新
      const unsubscribe = this.store.on(`jobs.${sourceJobId}.findings`, (findings) => {
        if (!Array.isArray(findings) || findings.length === 0) return;

        // SharedStore notifies with the complete findings array. Broadcast only
        // newly appended local findings; rebroadcasting the external copies would
        // bounce them between peers recursively until the stack overflowed.
        const previousCount = Math.min(seenCounts.get(sourceJobId) || 0, findings.length);
        seenCounts.set(sourceJobId, findings.length);
        const latestFindings = findings
          .slice(previousCount)
          .filter((finding) => !finding?.isExternal)
          .slice(-this.config.broadcastThreshold);
        
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
      this._trackSessionSubscription(sessionId, unsubscribe);
    });
  }
  
  /**
   * ========== 模式 2: EventBus (事件驱动) ==========
   */
  setupEventBusCollaboration(sessionId, jobIds) {

    // Publish directly to every peer inbox. The old implementation only put
    // callbacks in a Map and never subscribed them to SharedStore, so no event
    // could ever leave its source job.
    this.activeCollaborations.get(sessionId).publish = (jobId, eventType, payload) => {
      jobIds.forEach(otherJobId => {
        if (otherJobId === jobId) return;
        this.store.publish(`collab:${sessionId}:${otherJobId}`, {
          type: eventType,
          payload,
          sourceJobId: jobId,
          timestamp: Date.now(),
        });
      });
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
    
    
    const unsubscribe = this.store.on(`jobs.${leadJobId}`, (jobData) => {
      const status = jobData?.status;
      if (status === 'phase_complete' || status === 'completed') {
        const decision = this.store.get(`jobs.${leadJobId}`)?.decision || this.store.get(`jobs.${leadJobId}.decision`);
        
        if (decision) {
          followerJobIds.forEach(followerId => {
            // 继承 lead 的决策
            const followerJob = this.store.get(`jobs.${followerId}`) || {};
            followerJob.parentDecision = decision;
            followerJob.inheritFrom = leadJobId;
            followerJob.updatedAt = Date.now();
            this.store.set(`jobs.${followerId}`, followerJob);
            
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
    this._trackSessionSubscription(sessionId, unsubscribe);
    
    // Follower 可以向 Lead 请求指导
    this.registerFollowerRequestHandler(sessionId, leadJobId, followerJobIds);
  }
  
  registerFollowerRequestHandler(sessionId, leadJobId, followerJobIds) {
    // 监听 follower 的请求
    followerJobIds.forEach(followerId => {
      const unsubscribe = this.store.on(`jobs.${followerId}.request_guidance`, (request) => {
        // Lead job 接收请求并提供指导
        this.store.appendFinding(leadJobId, {
          type: 'guidance_request',
          from: followerId,
          data: request,
          timestamp: Date.now()
        });
      });
      this._trackSessionSubscription(sessionId, unsubscribe);
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
    // collectRelatedFindings 返回的是**降序**（新的在前，见它末尾的 sort）。
    // 这里原来写 `.slice(-10)` —— 对降序数组取末尾 10 条，拿到的正是**最旧**的 10 条；
    // main.js 那边再 `.slice(0, 6)`，于是新派的子智能体永远看到同伴最早的几条发现，
    // 刚查出来的结论一条都进不去。注释还写着"最新 10 条"，和代码正好相反。
    enhanced.relatedFindings = relatedFindings.slice(0, 10); // 降序数组的前 10 条 = 最新 10 条
    
    // 添加共享知识
    const sharedKnowledge = {};
    for (const [activeSessionId, session] of this.activeCollaborations.entries()) {
      if (session?.knowledge && Object.keys(session.knowledge).length) {
        sharedKnowledge[activeSessionId] = session.knowledge;
      }
    }
    if (Object.keys(sharedKnowledge).length > 0) {
      enhanced.sharedKnowledge = sharedKnowledge;
    }
    
    // 限制总大小
    const totalSize = JSON.stringify(enhanced).length;
    if (totalSize > this.config.maxContextSize) {
      console.warn(`[CollaborationEngine] Context too large (${totalSize} bytes), truncating`);
      enhanced._truncated = true;
      enhanced._originalSize = totalSize;
      if (Array.isArray(enhanced.fileSnippets)) {
        enhanced.fileSnippets = enhanced.fileSnippets.map((snippet) => ({
          ...snippet,
          content: String(snippet.content || '').slice(0, 800),
        }));
      }
      if (Array.isArray(enhanced.relatedFindings)) {
        // 同样是降序数组，取末尾就是取**最旧**的 5 条。上面那处已经改成 slice(0,10)
        // 并在注释里写明了原因，而这条裁剪分支一直没跟上——于是"上下文太大"时的降级
        // 恰好把最新的发现全丢了，只留下最早的几条。降级本来就是有损的，
        // 但不该反着损。
        enhanced.relatedFindings = enhanced.relatedFindings.slice(0, 5).map((finding) => ({
          sourceJobId: finding.sourceJobId,
          type: finding.type,
          channel: finding.channel,
          content: String(finding.content || finding.data || '').slice(0, 300),
          timestamp: finding.timestamp,
        }));
      }
      if (JSON.stringify(enhanced).length > this.config.maxContextSize) {
        delete enhanced.sharedKnowledge;
      }
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
    if (!this.readFile) throw new Error('CollaborationEngine requires an injected workspace reader');
    return this.readFile(path);
  }
  
  /**
   * 收集相关的 findings (来自其他协作的 jobs)
   */
  async collectRelatedFindings(currentJobId) {
    const allFindings = [];
    
    // **只收同一次派发里的同伴，不是黑板上所有人。**
    //
    // 这里原来无差别扫 `jobs.*`。而 SharedStore 是**全局**的、跨 run 也跨标签页：
    // 上一轮任务的子体、另一个项目窗口里正在跑的子体，findings 全都躺在同一块黑板上。
    // 于是新派的子智能体一开工就被喂进一段「其他角色已发现：…」，内容却来自一个
    // 它从来没参与过的任务——它会把那些结论当成本次调查的既有证据接着往下推。
    //
    // 键的形状是 `sm_<runToken>_<jobId>`（见 main.js 的 _smRunToken：runToken 是 run 内
    // 唯一的前缀，正是为了让两个标签页的 job#1 不串台）。同一次派发的同伴共享这个前缀。
    const cur = String(currentJobId);
    const runPrefix = /^(sm_[^_]+)_/.exec(cur)?.[1] || '';
    // 会话里登记过的同伴优先——那是最准的一份名单。
    const peers = new Set();
    for (const session of this.activeCollaborations.values()) {
      const ids = Array.isArray(session?.jobIds) ? session.jobIds.map(String) : [];
      if (ids.includes(cur)) for (const id of ids) peers.add(id);
    }
    const relatedJobs = this.store.query('jobs.*');
    
    for (const { key, value } of relatedJobs) {
      const match = /^jobs\.([^.]+)$/.exec(key);
      if (!match || match[1] === cur) continue;
      const id = match[1];
      if (peers.size) {
        if (!peers.has(id)) continue;
      } else if (runPrefix) {
        // 没登记会话时退回按 run 前缀：仍然把别的 run / 别的标签页挡在外面。
        if (!id.startsWith(runPrefix + '_')) continue;
      } else {
        // 连前缀都认不出来（老键或测试造的键）就不猜了——宁可不给上下文，
        // 也不要把别人的结论当成同伴发现喂进去。
        continue;
      }
      const findings = Array.isArray(value?.findings) ? value.findings : [];
      allFindings.push(...findings.map(f => ({ ...f, sourceJobId: id })));
    }
    
    // 排序按 **seq 优先**，timestamp 兜底。
    //
    // 只按 timestamp 排是不可靠的：Date.now() 的分辨率是毫秒，而子体在一轮里连着写
    // 十几条发现是常态——它们的 timestamp 完全相同，sort 稳定于是保持**插入顺序**，
    // 也就是升序。下游按"这是降序"去取前 N 条，拿到的正好是**最旧**的 N 条，
    // 而注释和变量名都在说"最新"。这个坑在 enhanceContext 里已经以两种形态各出现过一次。
    // seq 是写入端的单调计数（见 shared-store.appendFinding），同一毫秒也能定序。
    return allFindings.sort((a, b) => {
      const sa = Number(a?.seq), sb = Number(b?.seq);
      if (Number.isFinite(sa) && Number.isFinite(sb) && sa !== sb) return sb - sa;
      return (b.timestamp || 0) - (a.timestamp || 0);
    });
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
    
    if (percentage >= (this.config.criticalThreshold || 90)) status = 'critical';
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
    for (const unsubscribe of this.sessionUnsubscribers.get(sessionId) || []) {
      try { unsubscribe(); } catch {}
    }
    this.sessionUnsubscribers.delete(sessionId);
    this.activeCollaborations.delete(sessionId);
    
    return true;
  }
  
  /**
   * 获取所有活跃会话
   */
  getActiveSessions() {
    return Array.from(this.activeCollaborations.entries())
      .filter(([, session]) => session.status === 'active')
      .map(([sessionId, session]) => ({ sessionId, ...session }));
  }

  _trackSessionSubscription(sessionId, unsubscribe) {
    if (typeof unsubscribe !== 'function') return;
    if (!this.sessionUnsubscribers.has(sessionId)) this.sessionUnsubscribers.set(sessionId, []);
    this.sessionUnsubscribers.get(sessionId).push(unsubscribe);
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
      // 这里原来只给 `data: { key, value }`，没有 `content`。而收件箱那边读的是
      // `String(finding.content ?? finding.data ?? "")` —— 对一个对象求 String 就是
      // 字面的 "[object Object]"，于是子智能体的上下文里每次广播都多出一行
      // `· shared_knowledge: [object Object]`。占着 token，还让模型以为自己漏看了什么。
      this.store.appendFinding(jobId, {
        source: 'shared_knowledge',
        channel: 'knowledge_update',
        content: `${key}：${typeof value === 'string' ? value : JSON.stringify(value)}`,
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
