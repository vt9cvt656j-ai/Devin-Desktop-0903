# 多智能体系统集成完成 - 使用指南

> 本轮已把此前创建的所有模块**真正接入** IDE 主链路，构建通过 (vite build ✓)、
> 全部 573 个单元测试通过 (npm test ✓)。

## 本轮真实落地的内容

### 1. spawn_multiple_agents 工具 (全链路注册，走真实派发)

一次并发派发 2-5 个只读子智能体，**复用 IDE 原生的 `run._subAgentJobs` 后台作业台账**
(同一套 5min 超时/代际取消/await_subagent 汇合机制)，不是模拟器。

注册位置 (src/main.js):
- 工具 schema (`name: "spawn_multiple_agents"`，在 await_subagent 之后)
- 按需加载工具组 `subagent: { tools: [...] }`
- `_KNOWN_TOOLS` 白名单
- 弱模型别名 (`spawnagents` / `spawn_agents` / `spawnmultipleagents`)
- `_mapToolCall` → `type: "spawnmulti"`
- 主循环 `runSubagentItem` 执行器 (真实派发 N 个 `_runSubAgent`)
- `canRunInReadSegment` 并行段 (派发本身不阻塞同批只读工具)

模型侧用法:
```
spawn_multiple_agents(
  task="重构认证系统",
  agents=[
    {role: "security",  focus: "审计现有 auth 流程漏洞，输出 file:line 证据"},
    {role: "architect", focus: "梳理模块边界与迁移路径"},
    {role: "research",  focus: "调研 session vs JWT 在本项目的适配性"}
  ],
  collaboration="shared_store"   // 可选：independent=完全隔离
)
// → 立即返回 job#N 列表，主智能体继续干活
// → await_subagent(job="all") 随时汇合
```

shared_store 协同：每个作业落定时，报告摘要 (400 字) 自动广播给其他并行作业在
SharedStore 中的记录，控制台面板可见；同时给 CollaborationEngine 记 token 账。

### 2. UI 层（已按需求全部移除）

悬浮"子智能体控制台"面板、右下角悬浮按钮、SubagentControlPanel、SubagentCluster、
TokenBudgetDashboard 均已删除——作业状态用 IDE 原生的子智能体卡片与 `await_subagent`
台账摘要查看即可，不另起一套 UI。

### 3. 全局实例 (main.js 启动即就绪)

| 全局变量 | 模块 | 用途 |
|---|---|---|
| `window.collaborationEngine` | collaboration-engine.js | 三种协作策略 + token 追踪 |
| `window.realtime` | websocket-realtime.js | WS 实时推送 (无服务器时自动重试 10 次后静默) |

### 4. 修复的问题 (本轮验证中发现)

| 问题 | 修复 |
|---|---|
| `collaboration-engine.js` 导入路径写错 (`./agent/...`) 导致构建失败 | 改为 `./shared-store.js` |
| 3 个组件从 `components/` 引 `./agent/...` 路径错误 | 改为 `../agent/...` |
| Dashboard/ControlPanel 构造函数调用不存在的 `this.render()` | 改为 `this.updateUI()` |
| Dashboard 初始状态缺 `remaining` 字段导致首渲染 TypeError | 补默认值 |
| ControlPanel 终止作业调用了不存在的 `store.abort()` | 改为 `getJobQueue().abort()` |
| 上一轮把 `awaitsubagent` 执行器改成只查 JobQueue 模拟层，真实作业等不到 | **恢复 `run._subAgentJobs` 真实台账等待**，SharedStore 仅作面板展示镜像 |
| 上一轮 `_runSubAgent` 开头的 JobQueue stub 分支会把真实调研路由到模拟执行器 | **已移除**，真实异步派发由原生作业台账完成 |
| 测试断言旧的工具组内容 | 同步更新 2 处断言 |

## 验证记录

```
npx vite build   → ✓ built in 21.22s (仅既有 chunk 大小警告)
npm test         → 573 tests / 573 pass / 0 fail
node --check     → main.js / agent-console.js / 全部新模块通过
```

## 架构备注

- **真实执行层**：`run._subAgentJobs` (原生台账) 是子智能体作业的唯一真源；
  `await_subagent` 只等这里的 Promise。
- **展示层**：SharedStore 中 `jobs.sm_*` 记录是控制台面板的镜像数据，落定即更新。
- **JobQueue (`src/agent/job-queue.js`)**：其 `_runAgent` 目前是模拟 stub，
  仅供 `test-subagent.mjs` 单元测试与未来 Rust 端接入用，**不在真实派发链路上**。
- **CollaborationEngine**：shared_store 广播逻辑已接入 spawn_multiple_agents 落定回调；
  eventbus / lead_follower 两种模式基建就绪，可按需 startSession 使用。
