/**
 * SharedStore - 主子智能体间共享状态空间
 * 
 * 这是一个类似 Redis 的内存键值存储，用于:
 * 1. 子智能体之间交换信息 (findings, context)
 * 2. 主智能体追踪子智能体进度 (status, progress)
 * 3. 双向通信通道 (publish/subscribe)
 * 
 * 设计原则:
 * - 线程安全 (单线程 JS + Event Loop)
 * - 事件驱动 (on/off/unsubscribe)
 * - 数据隔离 (job-specific namespacing)
 * - 自动清理 (LRU 淘汰过期数据)
 */

class SharedStore {
  constructor(options = {}) {
    this.memory = new Map();
    this.listeners = new Map(); // key -> Set<callback>
    this.ttlMap = new Map(); // key -> expiryTimestamp
    
    // 配置项
    this.maxEntries = options.maxEntries || 10000;
    this.defaultTTL = options.defaultTTL || 3600000; // 1 小时默认过期
    this.cleanupInterval = options.cleanupInterval || 60000; // 每分钟清理
    
    // LRU 跟踪
    this.accessOrder = [];
    
    // 启动后台清理任务
    this._startCleanupTicker();
    
    console.log('[SharedStore] Initialized with maxEntries:', this.maxEntries);
  }

  /**
   * 设置键值对
   * @param {string} key - 键名 (支持点分隔路径：jobs.123.findings)
   * @param {*} value - 值 (自动深拷贝)
   * @param {number} ttl - 过期时间 (毫秒)，默认使用 defaultTTL
   */
  set(key, value, ttl = this.defaultTTL) {
    const fullPath = this._normalizeKey(key);
    
    // 深拷贝避免引用泄漏
    const clonedValue = JSON.parse(JSON.stringify(value));
    
    // Replacing a value must not remove subscribers. The old implementation
    // dropped listeners on every update, so a collaboration channel silently
    // stopped receiving findings after its first message.
    const oldValue = this.memory.get(fullPath);
    if (oldValue !== undefined) this._updateLRU(fullPath, -1); // 从 LRU 移除
    
    // 插入新数据
    this.memory.set(fullPath, clonedValue);
    this.ttlMap.set(fullPath, Date.now() + ttl);
    
    // 更新 LRU (移动到末尾 = 最新访问)
    this._updateLRU(fullPath, 1);
    
    // 触发监听器
    this._notifyListeners(fullPath, clonedValue);
    
    // 检查容量，必要时清理
    if (this.memory.size > this.maxEntries) {
      this._evictOldest();
    }
    
    return this;
  }

  /**
   * 获取值
   * @param {string} key 
   * @param {*} fallback - 默认值
   * @returns {*}
   */
  get(key, fallback) {
    const fullPath = this._normalizeKey(key);
    const value = this.memory.get(fullPath);
    
    if (value !== undefined) {
      // 更新 LRU
      this._updateLRU(fullPath, 1);
      
      // 检查是否过期
      const expiry = this.ttlMap.get(fullPath);
      if (expiry && Date.now() > expiry) {
        this.delete(fullPath);
        return fallback;
      }
      
      return value;
    }
    
    return fallback;
  }

  /**
   * 判断键是否存在，并与 get() 使用相同的 TTL 语义。
   * 直接暴露 Map.has 会把已经过期、但尚未等到后台清理的记录误报为存在。
   */
  has(key) {
    const fullPath = this._normalizeKey(key);
    if (!this.memory.has(fullPath)) return false;
    const expiry = this.ttlMap.get(fullPath);
    if (expiry && Date.now() > expiry) {
      this.delete(fullPath);
      return false;
    }
    this._updateLRU(fullPath, 1);
    return true;
  }

  /**
   * 删除键
   * @param {string|Array<string>} keys 
   */
  delete(keys) {
    const keyList = Array.isArray(keys) ? keys : [keys];
    let deletedCount = 0;
    
    for (const key of keyList) {
      const fullPath = this._normalizeKey(key);

      // A record may have been written using either `jobs.id` or
      // `jobs.id.status`. Deleting the record must remove both forms so stale
      // child fields cannot resurrect an old status on the next read.
      const keysToDelete = [...this.memory.keys()].filter((candidate) =>
        candidate === fullPath || candidate.startsWith(`${fullPath}.`));
      for (const candidate of keysToDelete) {
        this.ttlMap.delete(candidate);
        this._updateLRU(candidate, -1);
        this.memory.delete(candidate);
        deletedCount++;
      }

      // Subscribers commonly listen to a child path such as
      // `jobs.<id>.findings` while the coherent job record itself is stored only
      // at `jobs.<id>`. Deriving listener cleanup from memory keys therefore
      // leaves those callbacks alive after TTL/history cleanup, retaining the
      // whole collaboration closure. Clear the listener namespace directly.
      for (const listenerKey of [...this.listeners.keys()]) {
        if (listenerKey === fullPath || listenerKey.startsWith(`${fullPath}.`)) {
          this._removeListeners(listenerKey);
        }
      }
    }
    
    if (deletedCount > 0) {
      console.log(`[SharedStore] Deleted ${deletedCount} entries`);
    }
    
    return this;
  }

  /**
   * 订阅某个 key 的变化
   * @param {string} key 
   * @param {Function} callback 
   * @returns {Function} - unsubscribe 函数
   */
  on(key, callback) {
    const fullPath = this._normalizeKey(key);
    
    if (!this.listeners.has(fullPath)) {
      this.listeners.set(fullPath, new Set());
    }
    
    this.listeners.get(fullPath).add(callback);
    
    // 返回 unsubscribe 函数
    return () => this.off(key, callback);
  }

  /**
   * 取消订阅
   * @param {string} key 
   * @param {Function} callback 
   */
  off(key, callback) {
    const fullPath = this._normalizeKey(key);
    const listeners = this.listeners.get(fullPath);
    
    if (listeners) {
      listeners.delete(callback);
      
      // 如果没监听了，清理掉整个 entry
      if (listeners.size === 0) {
        this.listeners.delete(fullPath);
      }
    }
  }

  /**
   * 发布消息到所有订阅者
   * @param {string} key 
   * @param {*} data 
   */
  publish(key, data) {
    const fullPath = this._normalizeKey(key);
    this._notifyListeners(fullPath, data);
  }

  /**
   * 批量设置
   */
  setMany(pairs) {
    pairs.forEach(([key, value]) => this.set(key, value));
    return this;
  }

  /**
   * 批量获取
   */
  getMany(keys, fallback) {
    return keys.map(key => this.get(key, fallback ?? null));
  }

  /**
   * 查询所有匹配的 key (支持 glob 模式)
   */
  query(pattern) {
    const regex = this._patternToRegex(pattern);
    const results = [];
    
    for (const key of this.memory.keys()) {
      if (regex.test(key)) {
        results.push({ key, value: this.get(key) });
      }
    }
    
    return results;
  }

  /**
   * 获取活跃 job 列表
   */
  getActiveJobs() {
    const ids = new Set();
    for (const key of this.memory.keys()) {
      const match = /^jobs\.([^.]+)/.exec(key);
      if (match) ids.add(match[1]);
    }
    return [...ids].map((jobId) => ({ jobId, ...this._readRecord(`jobs.${jobId}`) }))
      .filter((job) => job.status === 'running' || job.status === 'pending');
  }

  /**
   * 创建新的 job 记录
   */
  createJob(jobMeta, requestedJobId = '') {
    const jobId = String(requestedJobId || `job_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`);
    
    this.set(`jobs.${jobId}`, {
      ...jobMeta,
      status: 'pending',
      progress: 0,
      findings: [],
      createdAt: Date.now(),
      updatedAt: Date.now()
    });
    
    // 触发全局事件
    this.publish('jobs.created', { jobId, meta: jobMeta });
    
    return jobId;
  }

  /**
   * 更新 job 状态
   */
  updateJobStatus(jobId, status, progress) {
    const key = `jobs.${jobId}`;
    const record = this._readRecord(key) || {};
    const next = { ...record, status, updatedAt: Date.now() };
    if (progress !== undefined) next.progress = progress;
    this.set(key, next);
    // Keep path-level subscriptions working while storing one coherent record.
    this._notifyListeners(`${key}.status`, status);
    if (progress !== undefined) this._notifyListeners(`${key}.progress`, progress);
    this._notifyListeners(`${key}.updatedAt`, next.updatedAt);
    
    // 触发事件
    this.publish('jobs.statusChanged', { jobId, status, progress });
    
    return this;
  }

  /**
   * 添加 finding 到指定 job
   */
  appendFinding(jobId, finding) {
    const key = `jobs.${jobId}`;
    const record = this._readRecord(key) || {};
    const findings = Array.isArray(record.findings) ? [...record.findings] : [];
    
    // 添加 timestamp 和元数据
    const enrichedFinding = {
      ...finding,
      source: finding.source === jobId ? jobId : (finding.isExternal ? String(finding.source || '') : jobId),
      timestamp: Date.now(),
      jobId // 反溯来源
    };
    
    findings.push(enrichedFinding);
    
    // 限制保留最近 100 条
    if (findings.length > 100) {
      findings.shift();
    }
    
    const next = { ...record, findings, updatedAt: Date.now() };
    this.set(key, next);
    this._notifyListeners(`${key}.findings`, findings);
    
    // 每 10 条 triggering 一次聚合事件
    if (findings.length % 10 === 0) {
      this.publish('jobs.findingsUpdated', { 
        jobId, 
        count: findings.length,
        latest: findings.slice(-5) // 最新 5 条
      });
    }
    
    return this;
  }

  /**
   * 广播到多个 job
   */
  broadcastToJobs(excludeJobId, channel, data) {
    const relatedJobs = this.getActiveJobs().filter(j => j.jobId !== excludeJobId);
    
    relatedJobs.forEach(job => {
      this.appendFinding(job.jobId, {
        source: excludeJobId,
        channel,
        data,
        isExternal: true
      });
    });
  }

  /**
   * 清空过期数据
   */
  cleanupExpired() {
    const now = Date.now();
    const toDelete = [];
    
    for (const [key, expiry] of this.ttlMap.entries()) {
      if (now > expiry) {
        toDelete.push(key);
      }
    }
    
    if (toDelete.length > 0) {
      this.delete(toDelete);
      console.log(`[SharedStore] Cleaned up ${toDelete.length} expired entries`);
    }
    
    return toDelete.length;
  }

  /**
   * 获取统计信息
   */
  stats() {
    return {
      totalEntries: this.memory.size,
      totalListeners: this.listenerCount(),
      ttlEntries: this.ttlMap.size,
      lruSize: this.accessOrder.length,
      activeJobs: this.getActiveJobs().length
    };
  }

  /**
   * 获取监听器总数
   */
  listenerCount() {
    let count = 0;
    for (const set of this.listeners.values()) {
      count += set.size;
    }
    return count;
  }

  // ==================== 私有方法 ====================

  _normalizeKey(key) {
    if (typeof key !== 'string') {
      throw new Error('Key must be a string');
    }
    // 统一为点分隔格式
    return key.replace(/[/\\]/g, '.');
  }

  _getValueAtPath(obj, path) {
    return path.split('.').reduce((acc, part) => {
      return acc?.[part];
    }, obj);
  }

  // Read a record written as one object and transparently merge legacy
  // path-style entries (`jobs.id.status`) that may still exist in memory.
  // The object entry wins over legacy children, so an update cannot be
  // overwritten by a stale field left by an older build.
  _readRecord(prefix) {
    const prefixDot = `${prefix}.`;
    const record = {};
    let found = false;
    for (const [key, value] of this.memory.entries()) {
      if (!key.startsWith(prefixDot)) continue;
      const path = key.slice(prefixDot.length).split('.');
      this._setValueAtPath(record, path, value);
      found = true;
    }
    const base = this.memory.get(prefix);
    if (base !== undefined) {
      if (base && typeof base === 'object' && !Array.isArray(base)) Object.assign(record, base);
      else return base;
      found = true;
    }
    return found ? record : undefined;
  }

  _setValueAtPath(target, path, value) {
    if (!path.length) return;
    let cursor = target;
    for (const part of path.slice(0, -1)) {
      if (!cursor[part] || typeof cursor[part] !== 'object') cursor[part] = {};
      cursor = cursor[part];
    }
    cursor[path[path.length - 1]] = value;
  }

  _notifyListeners(key, value) {
    const callbacks = this.listeners.get(key) || new Set();
    
    // 也要通知通配符订阅
    const wildcardKey = key.substring(0, key.lastIndexOf('.'));
    const wildcardCallbacks = this.listeners.get(wildcardKey + '.*') || new Set();
    
    [...callbacks, ...wildcardCallbacks].forEach(cb => {
      try {
        cb(value, key);
      } catch (err) {
        console.error('[SharedStore] Listener error:', err);
      }
    });
  }

  _removeListeners(key) {
    this.listeners.delete(key);
  }

  _updateLRU(key, delta) {
    const idx = this.accessOrder.indexOf(key);
    
    if (delta === 1) {
      // 加入或移到末尾
      if (idx > -1) {
        this.accessOrder.splice(idx, 1);
      }
      this.accessOrder.push(key);
    } else if (delta === -1 && idx > -1) {
      // 移除
      this.accessOrder.splice(idx, 1);
    }
  }

  _evictOldest() {
    // LRU 淘汰: 从最早访问的开始删
    while (this.memory.size > this.maxEntries && this.accessOrder.length > 0) {
      const oldestKey = this.accessOrder.shift();
      this.memory.delete(oldestKey);
      this.ttlMap.delete(oldestKey);
      for (const listenerKey of [...this.listeners.keys()]) {
        if (listenerKey === oldestKey || listenerKey.startsWith(`${oldestKey}.`)) {
          this._removeListeners(listenerKey);
        }
      }
      console.log('[SharedStore] Evicted oldest entry:', oldestKey);
    }
  }

  _startCleanupTicker() {
    const timer = setInterval(() => {
      this.cleanupExpired();
    }, this.cleanupInterval);
    // A store used by tests or a short-lived preview must not keep Node alive.
    if (typeof timer?.unref === 'function') timer.unref();
  }

  _patternToRegex(pattern) {
    // 简单实现：* -> .*
    const escaped = pattern.replace(/[+?{}()\[\]\\|^$/]/g, '\\$&');
    const regexStr = '^' + escaped.replace(/\*/g, '.*') + '$';
    return new RegExp(regexStr);
  }
}

// ==================== 单例导出 ====================

let sharedStoreInstance = null;

function getSharedStore(options = {}) {
  if (!sharedStoreInstance) {
    sharedStoreInstance = new SharedStore(options);
  }
  return sharedStoreInstance;
}

// 也支持导入类直接实例化（测试用）
export { SharedStore };
export default getSharedStore;
