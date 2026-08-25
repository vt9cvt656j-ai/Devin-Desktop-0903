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

    // **只有 lead_follower 一种模式。**
    //
    // 这里原来是个三分支 switch（shared_store / eventbus / lead_follower），而生产里唯一的
    // 调用点写死 `{ mode: "lead_follower" }`——另外两条从来没被执行过一次。
    // 更要紧的是 shared_store 那条是**第二套实现**：子体之间共享发现这件事，main.js 早就
    // 直接做了（派发时给每个子体一个收件箱键、互为同伴，见 mainlink.js）。两套实现里
    // 死的那套还带着一整套测试，测得全绿——那正是"测试在测产品不跑的代码"。
    // 连同它的 token 预算那一族（trackTokenUsage / triggerTokenWarning / getTokenStats /
    // autoOptimizeForTokenBudget / applyOptimization）一起删了，那五个也是零调用点。
    this.setupLeadFollowerCollaboration(sessionId, jobIds, config);

    return sessionId;
  }
  
  /**
   * ========== Lead-Follower：唯一在生产里跑的协同模式 ==========
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
    
    // 这里原来还挂着 registerFollowerRequestHandler —— 给每个 follower 订阅
    // `jobs.<id>.request_guidance`，号称"follower 可以向 Lead 请求指导"。
    //
    // **删了，因为那条通道从来没有写入端。** 全仓 grep `request_guidance` 只有那一行订阅：
    // 没有任何地方 set/publish 这个键，子体也没有任何工具或提示词知道有这个入口。
    // 而 SharedStore 的通知只在 set/publish 时按精确键或 `<父>.*` 通配触发，follower 的
    // 记录是整条对象写在 `jobs.<id>`，永远命中不到 `jobs.<id>.request_guidance`。
    // 也就是说这不是"模型不会用"，是**根本没有入口**——而它还给每个 follower 挂一条
    // 订阅，一直挂到 endSession。
    //
    // lead_follower 是唯一在生产里跑的模式，在它身上留一个假的一半，比没有更坏：
    // 下一个人会以为双向已经通了，去查"为什么 follower 不求助"。
    //
    // 真要做的话缺的是三样，缺一不可：子体侧一个能写这个键的工具或约定、提示词里
    // 告诉它有这条路、以及**主智能体能在自己那一轮之外响应**——最后这样现在不成立，
    // 主循环没有可被打断的结构。所以不是补个订阅就能通的事。
    // 反方向（主 → 子）是通的：见 src/agent/mainlink.js 的收件箱。
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
    
    // filesToInclude 这个参数在生产里**恒为空数组**（main.js 那个唯一调用点写死了 []），
    // 所以配套的 extractFileSnippets / readFileSync / generateLineSummary 三个方法一次都
    // 没被执行过，构造器里注入的 readFile 也一直没有消费方。三个方法连同注入一起删了。
    // 哪天真要给子体喂文件片段，从这里重新长出来，别把死代码留着当"以后可能用得上"。
    
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
/**
   * 触发 Token 警告
   */
/**
   * 获取 Token 使用统计
   */
  
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

  _trackSessionSubscription(sessionId, unsubscribe) {
    if (typeof unsubscribe !== 'function') return;
    if (!this.sessionUnsubscribers.has(sessionId)) this.sessionUnsubscribers.set(sessionId, []);
    this.sessionUnsubscribers.get(sessionId).push(unsubscribe);
  }
  
  /**
   * 广播消息给特定频道
   */
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

// ===== Singleton Pattern =====
let collaborationEngineInstance = null;

export function getCollaborationEngine(options = {}) {
  if (!collaborationEngineInstance) {
    collaborationEngineInstance = new CollaborationEngine(options);
  }
  return collaborationEngineInstance;
}

export default CollaborationEngine;
