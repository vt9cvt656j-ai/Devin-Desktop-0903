/**
 * WebSocketRealtime - WebSocket 实时数据推送 (P2 Implementation)
 * 
 * 功能:
 * - 建立与后端的 WebSocket 连接
 * - 实时接收子智能体进度更新
 * - 流式渲染 findings
 * - Token 消耗实时统计
 */

export class WebSocketRealtime {
  constructor(options = {}) {
    this.url = options.url || `ws://${window.location.host}/ws/subagents`;
    this.reconnectDelay = options.reconnectDelay || 3000;
    this.maxReconnectAttempts = options.maxReconnectAttempts || 10;
    
    this.socket = null;
    this.reconnectAttempts = 0;
    this.isConnected = false;
    this.messageHandlers = new Map();
    this.eventListeners = new Map();
    
    // 订阅的频道
    this.subscriptions = new Set();
    
    console.log('[WebSocketRealtime] Initialized, connecting to:', this.url);
  }
  
  /**
   * 连接到 WebSocket 服务器
   */
  connect() {
    if (this.isConnected || this.socket?.readyState === WebSocket.OPEN) {
      console.log('[WebSocketRealtime] Already connected');
      return;
    }
    
    try {
      this.socket = new WebSocket(this.url);
      
      this.socket.onopen = () => {
        console.log('[WebSocketRealtime] Connected successfully');
        this.isConnected = true;
        this.reconnectAttempts = 0;
        this.onOpen();
      };
      
      this.socket.onmessage = (event) => {
        const data = JSON.parse(event.data);
        this.handleMessage(data);
      };
      
      this.socket.onerror = (error) => {
        console.error('[WebSocketRealtime] Error:', error);
        this.onError(error);
      };
      
      this.socket.onclose = (event) => {
        console.log('[WebSocketRealtime] Connection closed', event.code, event.reason);
        this.isConnected = false;
        this.onClose(event);
        
        // 尝试重连
        if (event.wasClean || event.code === 1000) {
          // 正常关闭，不重连
          return;
        }
        
        this.scheduleReconnect();
      };
      
    } catch (error) {
      console.error('[WebSocketRealtime] Connection failed:', error);
      this.scheduleReconnect();
    }
  }
  
  /**
   * 安排重连
   */
  scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('[WebSocketRealtime] Max reconnection attempts reached');
      return;
    }
    
    this.reconnectAttempts++;
    const delay = Math.min(
      this.reconnectDelay * this.reconnectAttempts,
      30000 // 最大 30 秒
    );
    
    console.log(`[WebSocketRealtime] Scheduling reconnect in ${delay}ms (attempt ${this.reconnectAttempts})`);
    
    setTimeout(() => {
      this.connect();
    }, delay);
  }
  
  /**
   * 发送消息到服务器
   */
  send(channel, data) {
    if (!this.isConnected || !this.socket) {
      console.warn('[WebSocketRealtime] Not connected, cannot send message');
      return false;
    }
    
    try {
      this.socket.send(JSON.stringify({ channel, ...data }));
      return true;
    } catch (error) {
      console.error('[WebSocketRealtime] Send error:', error);
      return false;
    }
  }
  
  /**
   * 订阅频道
   */
  subscribe(channel) {
    if (!this.subscriptions.has(channel)) {
      this.subscriptions.add(channel);
      this.send('subscribe', { channel });
      console.log(`[WebSocketRealtime] Subscribed to ${channel}`);
    }
  }
  
  /**
   * 取消订阅
   */
  unsubscribe(channel) {
    if (this.subscriptions.has(channel)) {
      this.subscriptions.delete(channel);
      this.send('unsubscribe', { channel });
      console.log(`[WebSocketRealtime] Unsubscribed from ${channel}`);
    }
  }
  
  /**
   * 处理接收到的消息
   */
  handleMessage(data) {
    const { type, payload, channel } = data;
    
    console.log('[WebSocketRealtime] Received', type, 'from', channel);
    
    // 调用对应的处理器
    if (this.messageHandlers.has(type)) {
      this.messageHandlers.get(type)(payload, channel);
    }
    
    // 触发监听事件
    this.dispatchEvent(type, { payload, channel });
  }
  
  /**
   * 注册消息类型处理器
   */
  onMessage(type, handler) {
    this.messageHandlers.set(type, handler);
  }
  
  /**
   * 添加事件监听器
   */
  addEventListener(eventType, callback) {
    if (!this.eventListeners.has(eventType)) {
      this.eventListeners.set(eventType, []);
    }
    this.eventListeners.get(eventType).push(callback);
  }
  
  /**
   * 移除事件监听器
   */
  removeEventListener(eventType, callback) {
    const callbacks = this.eventListeners.get(eventType);
    if (callbacks) {
      const idx = callbacks.indexOf(callback);
      if (idx > -1) {
        callbacks.splice(idx, 1);
      }
    }
  }
  
  /**
   * 触发事件
   */
  dispatchEvent(eventType, data) {
    const callbacks = this.eventListeners.get(eventType) || [];
    callbacks.forEach(callback => {
      try {
        callback(data);
      } catch (error) {
        console.error('[WebSocketRealtime] Event handler error:', error);
      }
    });
  }
  
  /**
   * 连接打开时的回调
   */
  onOpen() {
    // 重新订阅之前订阅的频道
    this.subscriptions.forEach(channel => {
      this.subscribe(channel);
    });
    
    // 通知所有监听器
    this.dispatchEvent('connected', {});
  }
  
  /**
   * 连接错误的回调
   */
  onError(error) {
    this.dispatchEvent('error', { error });
  }
  
  /**
   * 连接关闭时的回调
   */
  onClose(event) {
    this.dispatchEvent('disconnected', { event });
  }
  
  /**
   * 断开连接
   */
  disconnect() {
    if (this.socket) {
      this.socket.close(1000, 'User disconnected');
      this.socket = null;
    }
    
    this.isConnected = false;
    this.subscriptions.clear();
    this.dispatchEvent('closed', {});
  }
  
  /**
   * 获取连接状态
   */
  getConnectionState() {
    if (!this.socket) return 'disconnected';
    if (this.socket.readyState === WebSocket.CONNECTING) return 'connecting';
    if (this.socket.readyState === WebSocket.OPEN) return 'connected';
    if (this.socket.readyState === WebSocket.CLOSING) return 'closing';
    return 'disconnected';
  }
}

// ===== 预定义的消息类型处理器 =====
export function createDefaultMessageHandlers(realtimeInstance) {
  const handlers = {
    // 子智能体进度更新
    subagent_progress: (payload) => {
      console.log('[Handler] Subagent progress:', payload);
      
      realtimeInstance.dispatchEvent('progress', {
        jobId: payload.jobId,
        step: payload.step,
        progress: payload.progress,
        timestamp: payload.timestamp
      });
    },
    
    // 新 findings 产生
    new_finding: (payload) => {
      console.log('[Handler] New finding:', payload);
      
      realtimeInstance.dispatchEvent('finding', {
        jobId: payload.jobId,
        finding: payload.finding,
        timestamp: payload.timestamp
      });
    },
    
    // Job 完成
    job_completed: (payload) => {
      console.log('[Handler] Job completed:', payload);
      
      realtimeInstance.dispatchEvent('completed', {
        jobId: payload.jobId,
        result: payload.result,
        duration: payload.duration,
        timestamp: payload.timestamp
      });
      
      // 自动从活跃列表中移除
      if (typeof window._globalSharedStore !== 'undefined') {
        window._globalSharedStore.updateJobStatus(payload.jobId, 'completed', 100);
      }
    },
    
    // Token 使用统计
    token_usage_update: (payload) => {
      console.log('[Handler] Token usage update:', payload);
      
      realtimeInstance.dispatchEvent('token_usage', {
        sessionId: payload.sessionId,
        used: payload.used,
        limit: payload.limit,
        percentage: payload.percentage,
        status: payload.status
      });
    },
    
    // 协作事件
    collaboration_event: (payload) => {
      console.log('[Handler] Collaboration event:', payload);
      
      realtimeInstance.dispatchEvent('collaboration', {
        sessionId: payload.sessionId,
        eventType: payload.eventType,
        data: payload.data,
        sourceJobId: payload.sourceJobId,
        timestamp: payload.timestamp
      });
    },
    
    // 错误/警告
    error: (payload) => {
      console.error('[Handler] Error:', payload);
      
      realtimeInstance.dispatchEvent('error', {
        jobId: payload.jobId,
        error: payload.error,
        context: payload.context
      });
    }
  };
  
  Object.entries(handlers).forEach(([type, handler]) => {
    realtimeInstance.onMessage(type, handler);
  });
  
  return handlers;
}

// ===== Singleton Pattern =====
let websocketRealtimeInstance = null;

export function getWebSocketRealtime(options = {}) {
  if (!websocketRealtimeInstance) {
    websocketRealtimeInstance = new WebSocketRealtime(options);
    
    // 默认启用所有处理器
    createDefaultMessageHandlers(websocketRealtimeInstance);
    
    // 自动尝试连接
    websocketRealtimeInstance.connect();
  }
  
  return websocketRealtimeInstance;
}

// ===== 自动导出到全局 (用于调试) =====
if (typeof window !== 'undefined') {
  window.WebSocketRealtime = WebSocketRealtime;
  window.getWebSocketRealtime = getWebSocketRealtime;
}

export default WebSocketRealtime;
