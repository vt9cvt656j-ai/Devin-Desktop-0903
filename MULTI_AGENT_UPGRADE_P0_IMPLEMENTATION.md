# Michael IDE 多智能体协作升级 - P0 实施说明

## 目标摘要

将当前**同步阻塞式**子智能体系统升级为**异步作业队列 + 共享存储 + 协同引擎**架构，实现:

1. ✅ 并发派多个子智能体 (后端/前端/测试各司其职)
2. ✅ 主智能体不等待，并行做其他任务
3. ✅ 实时 UI 显示集群进度
4. ✅ 子智能体间可通过 SharedStore 交换信息

---

## 已创建核心文件

### 1. `/src/agent/shared-store.js` (已完成 ✅)

共享状态空间，类似 Redis but 基于内存:
- 键值存储 + TTL 过期
- 事件订阅/发布机制 (on/off/publish)
- LRU 淘汰策略
- job-specific namespace (`jobs.{jobId}.*`)

**使用示例**:
```javascript
import getSharedStore from './agent/shared-store.js';

const store = getSharedStore();

// 创建 job 记录
const jobId = store.createJob({ 
  role: 'backend', 
  task: 'Implement API' 
});

// 更新进度
store.updateJobStatus(jobId, 'running', 50);

// 添加发现
store.appendFinding(jobId, {
  type: 'discovery',
  content: 'Found authentication flow in src/auth.js'
});

// 监听变化
store.on(`jobs.${jobId}.status`, (status) => {
  console.log('Status changed:', status);
});
```

### 2. `/src/agent/job-queue.js` (已完成 ✅)

异步作业调度器:
- 提交立即返回 jobId(非阻塞)
- 后台并发执行 N 个子智能体
- token 消耗监控与告警
- 事件总线 (started/completed/error)

**使用示例**:
```javascript
import getJobQueue from './agent/job-queue.js';

const queue = getJobQueue({ 
  maxConcurrent: 5,        // 最大并发数
  tokenWarningThreshold: 80 // token 警告阈值 %
});

// 提交 job (非阻塞)
const jobId = await queue.submit({
  id: 'custom-id', // 可选，自动生成
  tool: 'run_worker',
  args: {
    description: 'Build backend API',
    prompt: 'Implement REST endpoints...',
    scope: ['src/api'],
    role: 'backend'
  },
  onProgress: ({ step, progress }) => {
    console.log(`Step ${step}: ${progress}%`);
  },
  onComplete: async (result) => {
    console.log('Job done:', result);
    // 将结果注入 messages 数组供下一轮主循环使用
    mainAgent.messages.push({
      role: 'user',
      content: `[Worker ${jobId} 完成]\n${result.summary}`
    });
  }
});

console.log('Job submitted:', jobId);
// 可以立即做其他事，不需要 await!

// 如果需要等待某个特定 job
const result = await queue.waitForJob(jobId, 60000);
```

---

## 待修改的核心逻辑

### 位置 1: `/src/main.js` - `_runSubAgent` 函数改造

**问题**: 当前直接 `await _runSubAgent({...})` 阻塞主循环

**修改方案**:

找到 `_runSubAgent` 的定义处 (~line 33542),在函数开头添加:

```javascript
async function _runSubAgent({ config, description, prompt, root, container, run, write = false, scope = [], role = "", depth = 1, onMutation = null }) {
  // === NEW: 检查是否为异步模式 ===
  const isAsyncMode = true; // 可从配置读取
  
  if (isAsyncMode && depth === 0) {
    // 主循环派发 → 交给 JobQueue
    import getJobQueue from './agent/job-queue.js';
    const queue = getJobQueue();
    
    const jobId = await queue.submit({
      tool: 'run_subagent',
      args: { description, prompt, role, scope, write },
      runnerConfig: { maxSteps: SUB_MAX || 20 },
      onProgress: ({ step, progress }) => {
        // UI 实时更新
        if (container) {
          const statusEl = container.querySelector('.agent-tool-step--subagent .status');
          if (statusEl) statusEl.textContent = `执行中 ${progress}%`;
        }
      },
      onComplete: async (result) => {
        // 完成后注入 messages
        if (run && run.messages) {
          run.messages.push({
            role: 'user',
            content: `[子智能体完成] ${description}\n\n${result.summary}`
          });
        }
      }
    });
    
    // ⭐ 关键：不等待，直接返回 jobId
    return `【已派发至后台】job_${jobId}`;
  }
  
  // === ORIGINAL LOGIC (保留作为 fallback) ===
  // ... 原有的完整实现保持不动
```

**注意**: 这只是示意代码，需要根据实际代码结构调整。关键是:
1. 深度为 0(主智能体) 时走 JobQueue
2. 不 await，直接返回 jobId
3. 通过 `onComplete`回调将结果注入`messages`数组

---

### 位置 2: 工具定义 - 新增批量 spawns

在 `/src/tools/` 目录或主文件的工具 schema 定义处 (~line 25400 附近),添加新工具:

```javascript
{
  type: "function",
  function: {
    name: "spawn_multiple_agents",
    description: "并发派发给多个角色专属的子智能体，每个独立执行并定期汇报进度。适合大型任务需要并行开发场景。",
    parameters: {
      type: "object",
      properties: {
        task: {
          type: "string",
          description: "整体任务描述"
        },
        agents: {
          type: "array",
          items: {
            type: "object",
            properties: {
              role: { type: "string" },
              focus: { type: "string" },
              tools: { type: "array", items: { type: "string" } },
              priority: { type: "integer", default: 1 }
            }
          }
        },
        collaborationMode: {
          type: "string",
          enum: ["independent", "shared_store", "eventbus"],
          default: "shared_store"
        }
      },
      required: ["task", "agents"]
    }
  }
},
{
  type: "function",
  function: {
    name: "sync_subagent_results",
    description: "等待一个或多个后台子智能体作业完成并汇总结果",
    parameters: {
      type: "object",
      properties: {
        jobs: {
          type: "array",
          items: { type: "string" },
          description: "作业 ID 列表，默认等所有活跃作业"
        }
      }
    }
  }
}
```

需要在工具处理逻辑中添加对应的 handler(~line 26668):

```javascript
case "spawn_multiple_agents": {
  import getJobQueue from './agent/job-queue.js';
  const queue = getJobQueue();
  
  const { task, agents, collaborationMode } = args;
  const jobIds = [];
  
  for (const agentSpec of agents) {
    const jobId = await queue.submit({
      tool: 'run_worker', // 或 run_subagent
      args: {
        description: `${task} (${agentSpec.role})`,
        prompt: `${task} - Focus on: ${agentSpec.focus}`,
        role: agentSpec.role,
        scope: [] // 根据需求设置
      },
      // onProgress/onComplete...
    });
    jobIds.push(jobId);
  }
  
  // 如果使用了 shared_store 模式，设置协同规则
  if (collaborationMode === 'shared_store') {
    import getSharedStore from './agent/shared-store.js';
    const store = getSharedStore();
    
    jobIds.forEach(jobId => {
      store.on(`jobs.${jobId}.findings`, (findings) => {
        // 广播到其他 job
        jobIds
          .filter(id => id !== jobId)
          .forEach(otherId => {
            store.appendFinding(otherId, {
              source: jobId,
              channel: 'collaboration',
              data: findings.slice(-3) // 最新 3 条
            });
          });
      });
    });
  }
  
  return `【已并发派发】${jobIds.length} 个子智能体开始工作\nJobs: ${jobIds.join(', ')}`;
}

case "sync_subagent_results": {
  import getJobQueue from './agent/job-queue.js';
  const queue = getJobQueue();
  
  const jobs = args.jobs || null;
  const results = [];
  
  if (jobs) {
    for (const jobId of jobs) {
      const result = await queue.waitForJob(jobId);
      results.push(result);
    }
  } else {
    // 等所有活跃作业
    const activeJobs = queue.getActiveJobs();
    const promises = activeJobs.map(j => queue.waitForJob(j.jobId));
    const resultsArray = await Promise.all(promises);
  }
  
  return results.map(r => r.summary).join('\n\n');
}
```

---

### 位置 3: UI 组件 - SubagentCluster (新组件)

创建 `/src/components/SubagentCluster.jsx`:

```jsx
function SubagentCluster({ jobIds, autoRefresh = true }) {
  const [state, setState] = React.useState({
    jobs: {},
    total: jobIds?.length || 0,
    completed: 0,
    running: 0
  });
  const [expanded, setExpanded] = React.useState(false);
  
  React.useEffect(() => {
    // 如果是外部传入 jobIds，动态管理
    if (jobIds) {
      setState(prev => ({
        ...prev,
        jobs: jobIds.reduce((acc, id) => ({ ...acc, [id]: { status: 'pending' } }), {})
      }));
    }
    
    if (!autoRefresh) return;
    
    // 订阅全局事件
    import EventBus from './utils/event-bus.js';
    
    const unsub = EventBus.subscribe('job_progress', (data) => {
      setState(prev => {
        const updated = { ...prev.jobs, [data.jobId]: {
          ...prev.jobs[data.jobId],
          status: data.status,
          progress: data.progress,
          step: data.step
        }};
        
        const completed = Object.values(updated).filter(s => s.status === 'completed').length;
        const running = Object.values(updated).filter(s => s.status === 'running').length;
        
        return { ...prev, jobs: updated, completed, running };
      });
    });
    
    return () => unsub();
  }, []);
  
  const handleExpand = () => setExpanded(!expanded);
  
  const estTime = Math.ceil(state.running * 30); // 粗略估算
  
  return (
    <div className="subagent-cluster">
      {/* Header */}
      <div className="cluster-header" onClick={handleExpand}>
        <svg width="16" height="16" viewBox="0 0 16 16">
          <path d="M8 1l3 3H5L8 1z" />
          <path d="M8 13l-3-3h6l-3 3z" />
          <circle cx="8" cy="8" r="3" />
        </svg>
        <span>{state.running} 个子智能体正在并行工作</span>
        <span className={`status-badge status-${state.running > 0 ? 'running' : 'completed'}`}>
          {state.running > 0 ? 'Running' : 'Done'}
        </span>
      </div>
      
      {/* Expanded Grid */}
      {expanded && (
        <div className="cluster-grid">
          {Object.entries(state.jobs).map(([jobId, job]) => (
            <div key={jobId} className="subagent-card">
              <div className="card-header">
                <strong>Job: {jobId}</strong>
                <span className={`badge badge-${job.status}`}>{job.status}</span>
              </div>
              <div className="card-body">
                <div className="progress-bar">
                  <div 
                    className="progress-fill" 
                    style={{ width: job.progress || 0 + '%' }}
                  />
                </div>
                <div className="meta">
                  Progress: {job.progress || 0}% | Step: {job.step || 0}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
      
      {/* Footer Summary */}
      <div className="summary-bar">
        <strong>预计完成:</strong> ~{estTime}s
        <button onClick={handleExpand}>
          {expanded ? '收起' : '查看详情'}
        </button>
      </div>
    </div>
  );
}

export default SubagentCluster;
```

---

### 位置 4: CSS 样式

在 `/src/styles/app.css` 末尾追加:

```css
/* ===== Subagent Cluster UI ===== */
.subagent-cluster {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  margin: 12px 0;
  background: var(--bg-secondary);
  overflow: hidden;
}

.cluster-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px;
  cursor: pointer;
  font-weight: 600;
  color: var(--text-primary);
  border-bottom: 1px solid var(--border-color);
  transition: background 0.2s;
}

.cluster-header:hover {
  background: var(--bg-hover);
}

.status-badge {
  margin-left: auto;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 600;
}

.status-badge.status-running {
  background: rgba(59, 130, 246, 0.2);
  color: #3b82f6;
}

.status-badge.status-completed {
  background: rgba(34, 197, 94, 0.2);
  color: #22c55e;
}

.cluster-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 12px;
  padding: 12px;
}

.subagent-card {
  background: var(--bg-tertiary);
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 10px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.progress-bar {
  height: 8px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 6px;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #3b82f6, #22c55e);
  transition: width 0.3s;
}

.meta {
  font-size: 12px;
  color: var(--text-secondary);
}

.summary-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border-color);
  font-size: 14px;
}

.summary-bar button {
  padding: 6px 12px;
  background: var(--primary-color);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 600;
}

.summary-bar button:hover {
  opacity: 0.9;
}
```

---

## 测试步骤

### 1. 基本功能验证
```bash
# 启动本地开发服务器
cd /Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide
npm run dev

# 在浏览器中测试
```

**测试用例**:
- 打开开发者控制台，输入 `getSharedStore().stats()` 应输出统计信息
- 输入 `getJobQueue().stats()` 应输出 Queue 状态
- 手动提交一个 job 并验证进度更新

### 2. 并发测试
编写一个简单的测试脚本 `/test/subagent-concurrency.mjs`:

```javascript
import getJobQueue from '../src/agent/job-queue.js';

const queue = getJobQueue({ maxConcurrent: 3 });

// 同时派 5 个 job
const jobIds = [];
for (let i = 0; i < 5; i++) {
  const id = await queue.submit({
    tool: 'run_worker',
    args: { role: 'tester', description: `Test job ${i}` }
  });
  jobIds.push(id);
}

console.log('Submitted:', jobIds);

// 等待所有完成
for (const id of jobIds) {
  const result = await queue.waitForJob(id);
  console.log(`Job ${id} done:`, result.summary);
}

console.log('All done!');
process.exit(0);
```

运行:
```bash
node /test/subagent-concurrency.mjs
```

预期:
- 前 3 个立即执行
- 后 2 个排队等待
- 每个 job 打印进度更新
- 最后汇总结果

### 3. UI 集成测试
在主界面中调用:
```javascript
spawn_multiple_agents({
  task: 'Build a todo app with backend + frontend',
  agents: [
    { role: 'backend', focus: 'REST API design', tools: ['run_worker'] },
    { role: 'frontend', focus: 'UI components', tools: ['run_worker'] },
    { role: 'test', focus: 'E2E test coverage', tools: ['run_subagent'] }
  ],
  collaborationMode: 'shared_store'
})
```

观察:
- UI 是否显示 3 个并行卡片
- 每个卡片是否有实时进度
- 是否能在 SharedStore 中看到相互 broadcast 的 findings

---

## 后续优化方向

### P1 - 智能协同 (5-7 天)
1. **CollaborationEngine** 实现三种模式
2. **上下文传递增强** - 增加 fileSnippets 内容片段
3. **成本钳位** - token 计数器 + UI 告警

### P2 - 高级功能 (可选)
1. **双向通信** - 主子智能体互相发消息
2. **中途干预** - abort, extend_steps, change_focus
3. **递归派生** - 允许子智能体再派 (需权衡)

---

## 风险提示

### 风险 1: Token 爆炸
**缓解措施**:
- 默认限制并发数为 5
- 每个 subagent 最大 steps 降至 15
- 成本计算器 UI

### 风险 2: UI 复杂度过高
**缓解措施**:
- 默认折叠集群卡片
- 渐进式展开
- 提供"简略模式"开关

### 风险 3: 上下文混乱
**缓解措施**:
- 强制隔离各 job 的 context
- 只广播必要的 findings(经过摘要)
- 提供 clear_context 工具重置

---

## 验收标准

### 功能✅
- [ ] 能同时派 3+ 个不同角色的子智能体
- [ ] 主智能机等待期间可以继续写文档
- [ ] UI 实时显示每个子智能体进度
- [ ] 子智能体之间能通过 SharedStore 交换信息
- [ ] token 超 80% 时 UI 警告

### 性能⚡
- [ ] 并发 5 个子智能体不卡死主线程
- [ ] SharedStore 内存占用 < 50MB
- [ ] 每次会话最多 20 个活跃子智能体

### 用户体验👍
- [ ] 用户能看到"N 个子智能体并行中"提示
- [ ] 点击展开看每个详情
- [ ] 完成后汇总成统一报告

---

## 下一步行动

如果你对这个方案满意，我可以:

1. **立即开始**: 修改 `/src/main.js` 中的 `_runSubAgent` 函数
2. **测试驱动**: 先写测试确保兼容性
3. **分步提交**: 每完成一个模块就提交一次 git commit

告诉我你想怎么推进！