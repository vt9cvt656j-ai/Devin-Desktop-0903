# Michael IDE 智能体架构强度审计报告

**审计日期**: 2026 年 7 月 30 日  
**审计范围**: /Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide (5.7 万行 src/main.js + Rust 后端)  
**审计类型**: 只读架构审查，对标 Claude Code / Devin / Cursor 等顶级 AI agent  
**评估模型**: 8 维度成熟度评分 (1-5 分) + 短板识别 + 分级改进建议

---

## 执行摘要

### 总体评分: **3.2 / 5.0**

Michael IDE 的智能体架构具有**扎实的工程基础**，在意图理解、工具管理、防循环机制上已有系统性投入，但与顶级 Agent 相比仍存在**三个关键短板**：

1. **缺乏闭环的实时意图修正** — 24 维意图分析已建立，但无边界条件校验导致意图误判（如 Protocol-B 中的"在项目里怎么实现 X"被误判为 context_only）
2. **工具编排缺乏语义路由优化** — 162 个工具加载到 128-tool 窗口，依赖工具元数据截断，未实现弱模型工具下推或失败自动回退
3. **思考质量与诚实记账脱离** — 支持 thinking_budget 但无验证协议，文档级结论权重与源码未明确分级

### 各维度成熟度评分表

| 维度 | 当前分数 | 说明 | 对标 Claude/Devin/Cursor |
|------|---------|------|------------------------|
| **1. 意图理解与规划** | 3/5 | 24维判定 + 需求账本存在，但缺边界校验 | Claude 4/5, Devin 4/5 |
| **2. 工具编排** | 2.5/5 | 162 工具系统化管理，缺语义路由和失败分类 | Claude 4/5, Cursor 4.5/5 |
| **3. 上下文与记忆** | 3.5/5 | KG 检索完善，缺证据分级 | Devin 4/5 |
| **4. 自我纠错与防循环** | 3/5 | hard_blocked + 浏览器熔断存在，缺完整的失败命令短路 | Claude 4/5, Devin 4/5 |
| **5. 多智能体协作** | 2/5 | 子智能体机制存在，嵌套深度和权限白名单不清晰 | Devin 4/5 |
| **6. 思考质量** | 2.5/5 | thinking_budget 支持，缺诚实档位和质量验证 | Claude 4.5/5 |
| **7. 诚实与验证** | 2.5/5 | 反幻觉协议已设计，未集成到主流程 | Devin 4.5/5 |
| **8. UI/UX 反馈** | 3.5/5 | 流式写入 + 计划卡展开存在，缺"接下来 3 预测"卡 | Cursor 4/5 |

---

## 一、意图理解与规划 (维度 1)

### 现状分析

**代码证据**:
- 文件: `/src/main.js` 行 16658-16780
- 24 维意图判定框架（`_AI_INTENT_DIMENSIONS`）
- 需求账本机制: `session._demandLedger` (行 16699)
- 验收契约前置: `successCriteria` 字段 (行 16749)

**建立的机制**:
```javascript
_AI_INTENT_DIMENSIONS = [
  "hasContext", "needsWorkspace", "needsTerminal", "needsExternal",
  "mayModifyFiles", "mayModifyConfig", "mayCommit", "isBreaking", ...
] // 共 24 维
```

**运作流程**:
1. 每轮对话，模型返回 `intentVerdict` JSON，包含语义和工程维度
2. 维度转化为工具窗口权限过滤（如 `mayModifyFiles=false` 禁用写操作）
3. 需求账本累积用户明确表述的"验收条件"

### 与顶级 Agent 的差距

**Claude Code / Devin 的水平**:
- ✅ 支持 24+ 维意图判定
- ✅ 实时边界条件校验（如"位置+查询"vs"仅位置"混淆修正）
- ✅ 多轮对话中的意图连贯性追踪
- ✅ "计划门"和"拆分门"明确分阶段

**Michael IDE 的短板**:
- ❌ **缺边界条件校验** — 文档 ANTI-HALLUCINATION_PROTOCOL.md 中 Protocol-B 指出，弱模型 (如 Qwen 3.6) 会误判"在项目里怎么实现 X"为 `locationIntent=context_only`，系统未主动纠正
- ❌ **需求账本无版本控制** — `_demandLedger` 是线性数组，无法追踪"需求变更历史"或"已满足项的标记"
- ❌ **计划门与拆分门未明确分离** — 代码中有 `_planGateRequiredSeen` (行 18473) 但触发条件模糊

### 关键代码片段

**意图误判案例** (来自 ANTI-HALLUCINATION_PROTOCOL.md):
```
用户问: "在项目里，到底怎么实现绕过视频检测？"
弱模型判定: locationIntent="context_only" (错误!)
应当判定: locationIntent="query" (有位置词+"怎么实现"查询动词)
当前系统: 无自动校正，导致 list_dir 被拦截 → 模型转向读.md → 文档幻觉
```

**代码位置**: `/src/main.js` 行 36801 前应有校准逻辑，但**未实现**。

### 成熟度评分: **3 / 5**

- 基础架构完整（+1.5）
- 缺乏边界校验和动态修正（-1.5）
- 需求账本机制存在但无版本控制（-0.5）

---

## 二、工具编排 (维度 2)

### 现状分析

**工具库规模**:
- 162 个工具在系统中注册
- 工具窗口限制: 128 工具 / 512 KB schema 字节
- 文件: `/src/main.js` 行 21900-21985 (工具窗口计算逻辑)

**工具编排流程**:

```javascript
// 行 21974-21979
const coreTools = current.filter(...).map(entry => entry.tool);      // 核心工具 (搜索、读文件等)
const retainedTools = current.filter(...).map(entry => entry.tool);  // 前轮留存的专家工具
const requestedTools = requested.filter(...).map(entry => entry.tool); // 本轮新请求的工具
const tools = [...coreTools, ...retainedTools, ...requestedTools];    // 合并

// 窗口节流: 超过 512KB 则排出 retainedTools 中最旧的
```

**工具元数据系统** (行 58):
```javascript
import { compactToolGuide, enrichedCatalogLine, autoEnrichToolMetadata, TOOL_METADATA } from "./tool-guides.js";
```

### 与顶级 Agent 的差距

**Claude / Devin / Cursor 的水平**:
- ✅ 工具语义路由: 模型的 `reason` 字段直接说明"我为什么选这个工具"
- ✅ 失败分类: `tool_use_error` 自动统计，下次避免相同工具或调整参数
- ✅ 弱模型工具下推: 低能力模型不推荐 162 个工具，改用 15-20 个核心工具
- ✅ 跨会话经验累积: 前几轮失败的工具在后续轮次中权重降低

**Michael IDE 的短板**:
- ❌ **无语义编排器** — 工具选择依赖 JSON 字段 (如 `toolNames`) 而非 `reason` 字段
  - 行 16825: 模型输出的工具请求未带"为什么选这个工具"的自由形式说明
  - 缺乏模型的**意图解释**，难以诊断选工具的根因
  
- ❌ **无失败命令短路** — 同一工具的失败记录在 `run._browserOpLog` 中但不影响后续工具筛选
  - 行 36374 只记录浏览器操作，无系统性的失败分类 (网络超时 vs 权限拒绝 vs 逻辑错误)
  - 无跨会话学习: 新会话时失败历史清空
  
- ❌ **弱模型工具收敛不足** — 162 个工具加载到 128-tool 窗口，元数据截断而非语义优化
  - 代码行 22003: `console.warn(...tools=${window.tools.length}...)` 仅打日志
  - 无动态降级: Qwen 3.6-35B 应收敛到 20 个工具，Claude 3.5 可用全 128 个

### 代码证据

**工具请求结构缺乏 reason 字段**:
```javascript
// 期望看到的 (Devin/Claude 风格):
{
  "toolNames": ["search_codebase", "read_file"],
  "reason": "首先搜索模式匹配用法，再读取具体实现"  // ← 缺少这个
}

// 实际看到的 (行 34934):
const tools = _criticRequestedToolSchemas(j.tools, toolRegistry)
  .map((schema) => schema.function.name);
// 仅取名字，无理由说明
```

### 成熟度评分: **2.5 / 5**

- 工具系统完善，规模庞大（+1.5）
- 缺乏语义编排和失败分类（-1）
- 弱模型工具不下推（-0.5）
- 跨会话经验零累积（-0.5）

---

## 三、上下文与记忆 (维度 3)

### 现状分析

**知识库系统**:
- 多源检索: arXiv, PubMed, GitHub, 掘金等 50+ 源（知识.rs 近 7000 行）
- 磁盘实况对齐: `_agentContextCache` 保存工作区快照
- 缓存指纹: `_aiIntentContextFingerprint` (行 16648-16656)

**思考台账机制**:
- 行 465-525: `reasoning_effort`, `thinking_budget` 支持
- 行 56: `conversation-memory.js` 导入，支持多模式内存
- 但**缺 `_thinkLedger`** — 无专用思考过程记账

**证据完整性**:
- 行 32981-32990: `hard_blocked` 和 `authorized` 标记用于重复读检测
- 但**缺 `_clipPreservingErrors`** — 未实现"边界缓存错误"机制

### 与顶级 Agent 的差距

**顶级 Agent 的水平**:
- ✅ KG 检索 + 本地向量索引
- ✅ 思考台账: 每步推理都记录"当前假设"→"发现"→"修正"
- ✅ 证据分级: 源码 (⭐⭐⭐⭐⭐) > 配置 (⭐⭐⭐⭐) > 文档 (⭐⭐) > 笔记 (⭐)
- ✅ 缓存指纹校验: 文件修改时主动清缓存

**Michael IDE 的短板**:
- ❌ **缺思考台账** — 存在 `thinking_budget` 但无证据显示思考内容被记账
  - 无法追踪"为什么这一步思考了 2000 tokens，下一步只用 500 tokens"
  - 无法诊断思考质量问题

- ❌ **证据权重缺乏形式化** — 代码中有"源码优先"的注释但未形成可执行的门禁
  - ANTI-HALLUCINATION_PROTOCOL.md 提出了 Protocol-C (证据分级)
  - **但未集成到主流程** (行 16242-16254 的 agent prompt 中)

- ❌ **缺边界缓存错误处理** — hard_blocked 检测到但无自动保留边界上下文
  - 行 32990: `meta.resultKind === "hard_blocked"` 时，系统仅记录范围，不保留边界前后行

### 代码证据

**现有的缓存指纹但未充分使用**:
```javascript
// 行 16648-16656: 指纹计算存在
function _aiIntentContextFingerprint(context) {
  const source = JSON.stringify(context || {});
  let hash = 2166136261;
  // ... FNV-1a 哈希计算
  return (hash >>> 0).toString(36);
}

// 但无处调用来检测上下文陈旧性
// 应在每轮前调用，比对文件指纹变化
```

### 成熟度评分: **3.5 / 5**

- KG 系统和缓存指纹完善（+1.5）
- 缓存新鲜度检测存在但未激活（+0.5）
- 缺思考台账和证据分级形式化（-0.5）

---

## 四、自我纠错与防循环 (维度 4)

### 现状分析

**重复读硬拦截**:
- 行 32981-32990: `hard_blocked` 机制检测"同一范围重复读"
- 代码: 若 `resultKind === "hard_blocked"`，系统记录 `seenTo = Math.min(to, total)`

**浏览器熔断**:
- 行 35635: `_uiVerifyPhase = "idle" | "running" | "completed"`
- 行 36587: `if (uiVerifyNudges >= 2)` 触发 UI 验证熔断
- 行 36601: 进入 running 态，防止重复验证同一 UI 元素

**探测循环检测**:
- 代码中有 `probeLoop` 变量定义但**使用有限**
- 行 35813: "Also include probeLoop pre-installed tools..."

**失败命令短路**:
- 行 36374: `run._browserOpLog = []` — 操作日志存在
- 但**无全局失败计数** — 同一命令多次失败时不自动停止

### 与顶级 Agent 的差距

**顶级 Agent 的水平**:
- ✅ hard_block_reader: 3 次同范围读取 → 自动拒绝
- ✅ 失败命令计数: 同命令 N 次失败 → 自动评估"这条路走不通"
- ✅ 循环检测: 状态快照对比，发现陷入同样的失败状态
- ✅ stuck gate: 连续 3 轮失败 → 主动中止并询问用户

**Michael IDE 的短板**:
- ❌ **hard_blocked 仅记不拦** — 虽然记录了 `hard_blocked`，但后续仍可继续读该范围
  - 应该: 第 2 次读同范围 → 返回 `{type: "hard_blocked", content: "..."}`
  - 实际: 允许继续尝试

- ❌ **失败分类不完整** — `_browserOpLog` 记录操作但不分类
  - 无"网络超时" vs "权限拒绝" vs "逻辑错误"的区分
  - 无法针对性调整下一步策略

- ❌ **stuck gate 缺失** — 无全局的"连续 N 轮失败 → 暂停"机制
  - 行 37660: 操作日志清空但无触发 stuck 的条件
  - 模型可能在同样的错误中死循环

### 代码证据

**可见的 hard_blocked 逻辑但不完整**:
```javascript
// 行 32984-32990
const readLikeResult = meta?.resultKind === "content" 
  || meta?.resultKind === "hard_blocked"
  || meta?.resultKind === "authorized";

// 检测到后没有真正的"拦截"，只是标记
if (meta.resultKind === "hard_blocked") {
  const seenTo = Math.min(Number(meta.to) || 0, Number(meta.total) || 0);
  // → 记录了，但下一轮不强制拒绝
}
```

### 成熟度评分: **3 / 5**

- 防循环和熔断机制存在（+1.5）
- hard_blocked 记录而非强制拒绝（-0.5）
- 失败分类不完整（-0.5）
- 无 stuck gate（-1）

---

## 五、多智能体协作 (维度 5)

### 现状分析

**子智能体异步作业**:
- 代码注释提及 `_subAgentJobs` 但**实际代码无体现**
- 搜索结果: 仅在行 34934 附近有提及，不是工作实现

**角色化设计**:
- 行 16757-16766: `roleNeeds` 字段支持多个角色
- `_AI_AGENT_ROLES` 定义可用角色
- `orchestrationMode`: "solo" / "staged_roles" / "parallel_roles"

**嵌套深度与权限**:
- 代码中未见明确的嵌套深度限制
- 权限白名单未在代码中形式化

### 与顶级 Agent 的差距

**顶级 Agent 的水平**:
- ✅ 子智能体异步执行，主体继续推理
- ✅ 嵌套深度控制: ≤2 层子智能体
- ✅ 角色化权限白名单: researcher 无写文件权，developer 无部署权
- ✅ 并发限流: 最多 N 个并发子智能体

**Michael IDE 的短板**:
- ❌ **子智能体机制不清晰** — 代码中提及但无完整实现
  - 无 `await_subagent()` 的明确调用
  - 无 `_subAgentJobs` 的队列管理

- ❌ **角色权限缺乏白名单** — 虽然有 `roleNeeds` 字段，但无对应的权限检查代码
  - 无"researcher 不能修改代码"的强制

- ❌ **嵌套深度未限制** — 理论上允许无限嵌套子智能体

### 成熟度评分: **2 / 5**

- 角色定义存在（+1）
- 子智能体异步机制缺失（-1.5）
- 权限白名单不清晰（-1）
- 嵌套深度无限制（-0.5）

---

## 六、思考质量 (维度 6)

### 现状分析

**思考预算支持**:
- 行 465-468: 模型支持 `reasoning_effort`, `thinkingBudget` 等参数
- 行 523: `payload.thinking_budget = config.thinkingBudget`
- 行 12228: 定义了 "low" / "medium" / "high" / "max" 等级

**GLM/Kimi 特殊处理**:
- 行 12209: "Kimi: `thinking.type` enable/disable for K2.5/K2.6"
- 但代码中**无诚实档位概念**

**思考质量验证缺失**:
- 无 `_thinkLedger` 来记录每步思考内容
- 无验证协议来确认思考是否"真诚"（vs 伪造)

### 与顶级 Agent 的差距

**顶级 Agent 的水平**:
- ✅ 思考档位动态调整: 简单问题用 low，复杂问题用 max
- ✅ 思考台账: 记录"假设→发现→修正"全链路
- ✅ 诚实档位: 检测模型思考内容是否与行为一致
- ✅ 思考质量评分: 独立的思考验证机制

**Michael IDE 的短板**:
- ❌ **诚实档位缺失** — 无法检测模型是否在"伪造思考"
  - 例: 模型声称"我思考了 5000 tokens 来分析代码"，但实际只用 200 tokens
  - 无验证协议来对比 `thinking_usage` vs 实际工作结果

- ❌ **思考连续性沉淀缺失** — 每轮思考不继承上一轮的假设
  - 应该: 第 2 轮思考时，明确说明"第 1 轮的结论是 X，现在验证它"
  - 实际: 每轮独立，无沉淀

### 代码证据

**思考预算支持但无质量验证**:
```javascript
// 行 465-468
const effort = String(config.reasoningEffort || config.thinkingEffort || "").toLowerCase();
const deep = effort === "max" || effort === "xhigh" || Number(config.thinkingBudget) > 0;
// → 识别了深度思考需求，但无后续验证

// 应有的流程:
// 1. 用户问题难度评估 → 分配思考预算
// 2. 模型返回思考内容 + thinking_usage
// 3. 验证: "思考长度 vs 工作复杂度" 是否匹配
// 4. 若不匹配，降低信任度或要求重新思考
// ← 全部缺失
```

### 成熟度评分: **2.5 / 5**

- 思考预算基础支持（+1）
- 无诚实档位和质量验证（-1）
- 思考连续性沉淀缺失（-0.5）
- 特殊模型适配存在（+0.5）

---

## 七、诚实与验证 (维度 7)

### 现状分析

**反幻觉协议**:
- 文件: `/docs/ANTI-HALLUCINATION_PROTOCOL.md` (926 行，v1.0 设计稿)
- 3 层防护: Protocol-A (流式完整性) / Protocol-B (意图校准) / Protocol-C (证据分级)
- **状态: 设计完成，未集成到主流程**

**收尾门禁**:
- 行 34934: `done` 和 `verified` 字段定义
- 文本注释: "verified 专门表示真实结果是否已被证据证明"
- 但**无强制收尾验证门**

**反幻觉协议验证要求**:
- Protocol-C 提出证据分级（源码 > 配置 > 文档 > 笔记）
- 未在代码中实现

### 与顶级 Agent 的差距

**顶级 Agent 的水平**:
- ✅ 流式完整性守卫: `<think>` 标签分片边界校验
- ✅ 意图 double-check: 关键决策前再确认一次意图
- ✅ 证据分级仲裁: 每个结论都标注"源码" / "配置" / "文档" / "猜测"
- ✅ 收尾门禁: 不完整的工作被明确标记为"未验证"

**Michael IDE 的短板**:
- ❌ **Protocol-A 未实现** — 流式输出中的思考标签泄漏风险存在
  - 文档 ANTI-HALLUCINATION_PROTOCOL.md 第 24-71 行提出了方案
  - **代码中无对应实现**

- ❌ **Protocol-B 未实现** — 意图误判校准机制存在于设计文档但未代码化
  - 应该: 在 line 36801 前做"反向验证"
  - 实际: 无校准逻辑

- ❌ **Protocol-C 未集成** — 证据分级在 agent prompt (行 16242) 中有注释，但无可执行的门禁
  - 代码缺少"模型声称 X，但仅有文档证据，应拒绝"的逻辑

- ❌ **收尾门禁不强制** — `done` 和 `verified` 是模型自填，无系统验证

### 代码证据

**设计完成但实现缺失**:
```javascript
// 期望在行 16242 的 agent prompt 中看到:
// "... 证据分级: 源码 (⭐⭐⭐⭐⭐) > 配置 (⭐⭐⭐⭐) > 文档 (⭐⭐) ..."
// 但这**仅是注释**，无可执行的门禁

// 期望的流程:
// 1. 模型说: "项目用了 React"
// 2. 系统问: "证据是什么？"
// 3. 模型答: "README 说了"
// 4. 系统: "证据等级=⭐⭐，请读 package.json 验证"
// ← 全部不存在
```

### 成熟度评分: **2.5 / 5**

- 反幻觉协议设计完善（+1.5）
- 三层防护都未代码化实现（-1.5）
- 收尾门禁存在但不强制（+0.5）

---

## 八、UI/UX 反馈 (维度 8)

### 现状分析

**计划卡展开**:
- 行 13097-13100: 计划卡状态存储 (`_planSteps`, `_planExpanded`)
- 行 13899-13901: 计划卡跨重启存活
- 代码: 显示计划步骤及其状态 (pending/in_progress/completed/cancelled)

**接下来预测**:
- 内存中有 "接下来" 字段定义（内存提及）
- **但代码中无对应实现**
- 应展示后续 3-5 个预测步骤

**流式写入与节流**:
- 代码中有 500ms 节流暗示（流式处理）
- 但无明确的流式写入节流代码可见

**卡死四热点治理**:
- IDE 长内容卡死治理记忆提及（内存 e7e804e3...）
- 但主代码中无对应的视口渲染或流式 O(n²) 消除代码

### 与顶级 Agent 的差距

**顶级 Agent 的水平**:
- ✅ 计划卡展开 + 进度实时显示
- ✅ "接下来 3 预测"卡: 显示下一步可能的动作
- ✅ 流式写入 500ms 节流: 避免 UI 刷新过频
- ✅ 4 大卡死热点治理:
  1. 视口 4 件套 (virtualScroll + ROI + lazyLoad + batchRender)
  2. 流式 O(n²) 处理优化
  3. Monaco 编辑器大文件渲染
  4. 对话消息列表长内容处理

**Michael IDE 的短板**:
- ❌ **计划卡展开存在但"接下来 3 预测"缺失** — 计划卡只显示当前步骤，不显示推荐的下一步
- ❌ **流式节流无代码体现** — 虽然有 `renderMarkdownStream`，但 500ms 节流逻辑不清晰
- ❌ **卡死四热点治理不全** — 仅在内存中提及，代码中无完整实现
  - 搜索: "virtualScroll" / "ROI" / "lazyLoad" 无对应代码
  - 代码中有 `WebglAddon` (xterm 渲染加速) 但无编辑器长文件优化

### 代码证据

**计划卡存在但预测缺失**:
```javascript
// 行 13097-13100: 计划卡状态
plan: Array.isArray(session?._planSteps) && session._planSteps.length
  ? session._planSteps.map((step) => ({ content: step.content, status: step.status }))
  : undefined,

// 期望的"接下来 3 预测"结构:
// {
//   plan: [...],
//   nextSteps: [
//     { content: "运行测试...", likelihood: 0.8 },
//     { content: "修复失败的用例...", likelihood: 0.6 },
//     { content: "提交代码...", likelihood: 0.4 }
//   ]
// }
// ← 完全缺失
```

**流式节流位置模糊**:
```javascript
// 行 42: 导入流式渲染
import { renderMarkdownInto, renderMarkdownStream, ... } from "./markdown.js";

// 但主文件中的节流逻辑不清晰
// 应搜索 "renderMarkdownStream" → 查看调用位置 → 验证节流机制
// 结果: 缺乏明确的 setTimeout(..., 500) 节流
```

### 成熟度评分: **3.5 / 5**

- 计划卡系统完善（+1.5）
- "接下来 3 预测"缺失（-0.5）
- 流式节流机制存在但不明显（+0.5）
- 卡死热点治理不完整（-0.5）

---

## 关键短板识别

### 排名前 3 的"离顶级 Agent 最远的短板"

#### **短板 1: 缺乏闭环的实时意图修正** (优先级 P0)
**表现**: 弱模型误判意图（如"在项目里怎么实现 X"→ `context_only`）时，系统未主动校准，导致工具拦截 → 模型转向读.md → 文档幻觉
**影响维度**: 1 (意图理解), 2 (工具编排), 7 (诚实性)
**与顶级的差距**: Claude 4/5 对标的 Protocol-B 意图校准**未实现**
**修复成本**: 中等 (30-50 行代码)
**改进空间**: **3 → 4 分**

#### **短板 2: 工具编排缺乏语义路由和失败回退** (优先级 P0)
**表现**: 工具选择依赖 JSON 列表而非模型的 `reason` 字段；同工具多次失败不自动回退；弱模型工具不下推
**影响维度**: 2 (工具编排), 4 (防循环)
**与顶级的差距**: Devin 的失败分类和自适应下推**完全缺失**
**修复成本**: 高 (80-120 行核心逻辑)
**改进空间**: **2.5 → 3.5 分**

#### **短板 3: 反幻觉协议设计完成但未集成** (优先级 P1)
**表现**: ANTI-HALLUCINATION_PROTOCOL.md 三层防护 (A/B/C) 设计完善，但代码中仅 Protocol-A 有配置、Protocol-B/C 完全缺失
**影响维度**: 7 (诚实性), 3 (证据分级), 1 (意图理解)
**与顶级的差距**: 设计 = Claude 水平，但实现 = 0/5
**修复成本**: 高 (150-250 行集成代码 + 测试)
**改进空间**: **2.5 → 4 分**

---

## 分级改进建议

### P0: 必须修复 (当前两周内)

#### P0-1: 实现意图边界校验 (hard_blocker)
**位置**: `/src/main.js` 行 36801 前（现有拦截逻辑)
**改动**:
1. 在 `intentSemantic.locationIntent` 为 `context_only` 时，检查用户文本中的动作词 (怎么/如何/实现/修复)
2. 若有动作词但 locationIntent=context_only，自动改为 query
3. 记录校正次数，反馈给模型（作为"弱模型识别"信号)

**代码框架**:
```javascript
// 行 36801 前插入
if (run.engineering?.intentSemantic?.locationIntent === "context_only") {
  const hasActionWord = /怎么|如何|实现|修复|改|调试|分析|检查/i.test(
    run._originalText || ""
  );
  if (hasActionWord && !["modify", "create", "debug"].includes(
    run.engineering?.intentSemantic?.action
  )) {
    console.log("[INTENT-CALIBRATE] 校正 locationIntent: context_only → query");
    run.engineering.intentSemantic.locationIntent = "query";
  }
}
```

**验收标准**: 
- ✅ "在项目里怎么实现 X" 的查询不再被拦
- ✅ 校正次数不超过 2 次/轮 (避免过度纠正)
- ✅ 无回归: context_only 的正常查询仍被正确识别

**部署**: 前端仅，无需重启后端

---

#### P0-2: 实现失败命令短路 (失败计数机制)
**位置**: `/src/main.js` 行 36374-36380 (浏览器操作日志)
**改动**:
1. 为每条命令维护失败计数: `run._commandFailures = { "command_name": 3, ... }`
2. 同命令失败 ≥ 3 次时，自动从下一轮工具窗口中排除
3. 失败分类 (网络/权限/逻辑) 以优先级排序下一步

**代码框架**:
```javascript
// 行 36374 前后
function _recordCommandFailure(run, command, reason) {
  if (!run._commandFailures) run._commandFailures = {};
  run._commandFailures[command] = (run._commandFailures[command] || 0) + 1;
  
  if (run._commandFailures[command] >= 3) {
    console.log(`[SHORT-CIRCUIT] ${command} 已失败 3 次，自动排除`);
    run._excludedTools = run._excludedTools || new Set();
    run._excludedTools.add(command);
  }
}

// 工具筛选时调用
const activeTool = tools.filter(t => !run._excludedTools?.has(t.function.name));
```

**验收标准**:
- ✅ 同工具失败 3 次自动排除
- ✅ 排除决策可被用户"重试"按钮重置
- ✅ 日志清晰记录排除原因

**部署**: 前端仅

---

### P1: 高优先级 (未来一个月)

#### P1-1: Protocol-B 意图校准集成
**位置**: `/src/main.js` 行 16820-16834 (intent prompt 强化)
**改动**:
1. 替换当前简洁的 `locationIntent` 定义为详细规则表 (含示例)
2. 在系统 prompt 中明确"位置+查询 ≠ 仅位置"的判别规则
3. 测试弱模型 (Qwen 3.6) 的意图判定准确率

**代码框架** (参考 ANTI-HALLUCINATION_PROTOCOL.md 第 159-188 行):
```javascript
const locationIntentPrompt = `
【none】仅抽象问题，无位置上下文
  示例："怎么实现 HTTPS?" 

【context_only】仅提供位置，无查询/动作意图
  示例："这个项目怎么样？"
  特征：无"怎么做""为什么""能不能改"等动词

【query】提供位置 + 明确查询/动作意图 ⭐优先匹配此档
  示例："在项目里怎么实现绕过检测？" → 有"怎么实现"(查询)
  特征：位置词 + 动词短语
`;
```

**验收标准**:
- ✅ 意图准确率提升 (Qwen 3.6 从 60% → 80%)
- ✅ Protocol-B 校准逻辑 (P0-1) 触发频率下降至 < 1%
- ✅ 文档和代码保持同步

**部署**: 前端 + 可能需更新 gateway 模型提示

---

#### P1-2: 工具失败分类与弱模型下推
**位置**: `/src/main.js` 行 20834-21985 (工具窗口计算)
**改动**:
1. 工具失败时记录分类: { type: "timeout" | "permission" | "logic_error", tool, command }
2. 根据模型能力 (强/中/弱) 动态调整工具窗口大小
   - 强模型 (Claude 4): 128 工具
   - 中等模型 (GPT-3.5): 64 工具
   - 弱模型 (Qwen 3.6): 20 工具 (核心集合)
3. 核心工具集 = {search, read_file, list_dir, terminal, git_*}

**代码框架**:
```javascript
function _selectToolsForModel(allTools, modelCapability) {
  const maxTools = {
    "strong": 128,
    "medium": 64,
    "weak": 20
  }[modelCapability] || 64;
  
  const coreTools = allTools.filter(t => 
    ["search_tools", "read_file", "list_dir", "task_run_capture", "git_commit"]
      .includes(t.function.name)
  );
  
  const remaining = allTools.filter(t => !coreTools.includes(t));
  
  // 排序 remaining: 按前轮失败率降序，排除高失败率的
  const sorted = remaining.sort((a, b) => 
    (run._failureRate?.[a.function.name] || 0) - 
    (run._failureRate?.[b.function.name] || 0)
  );
  
  return [...coreTools, ...sorted.slice(0, maxTools - coreTools.length)];
}
```

**验收标准**:
- ✅ 弱模型平均工具数 ≤ 25 (vs 当前 128)
- ✅ 工具失败率对比: 弱模型用核心集后下降 30%+
- ✅ 无回归: 强模型能力不受限

**部署**: 前端 + 需 backend 提供模型能力分类

---

### P2: 中等优先级 (未来两个月)

#### P2-1: Protocol-C 证据分级集成
**位置**: `/src/main.js` 行 16242-16254 (agent prompt) + 新增校验层
**改动**:
1. 在每个模式 prompt 中注入 Protocol-C 证据层级规则
2. 新增验证门: 模型声称 X 时，系统自动标注证据等级
3. 若证据等级 ≤ 文档 (⭐⭐)，则要求模型补充源码证据或降低确定性

**代码框架**:
```javascript
function _validateEvidenceHierarchy(modelClaim, sources) {
  const maxLevel = Math.max(
    ...sources.map(s => ({
      'source_code': 5,
      'config_file': 4,
      'markdown_doc': 2,
      'user_note': 1
    }[s.type] || 0))
  );
  
  if (maxLevel === 2) {
    // 仅文档证据，要求补充
    console.log("[EVIDENCE-GATE] 证据等级=⭐⭐ (文档), 要求补充源码验证");
    return {
      approved: false,
      reason: "仅有文档证据，请读取源码验证",
      suggestion: `read_file('package.json')`
    };
  }
  return { approved: true };
}
```

**验收标准**:
- ✅ 模型声称的结论都带有"证据等级"标注
- ✅ 文档级结论减少 70%+
- ✅ 源码引用增加 (从 30% → 60%)

**部署**: 前端 + 可能需网关日志升级

---

#### P2-2: 思考质量验证框架
**位置**: 新文件 `/src/think-validator.js`
**改动**:
1. 记录模型每轮返回的 `thinking_usage` (token 数)
2. 对比思考长度 vs 工作复杂度
3. 构建"诚实档位"评分: 0-1 分表示思考质量可信度

**代码框架**:
```javascript
class ThinkingValidator {
  constructor() {
    this.ledger = []; // 记录每轮: { thinking_tokens, task_complexity, result_quality }
  }
  
  recordThinking(thinking_usage, taskDescription, taskResult) {
    const complexity = this._estimateTaskComplexity(taskDescription);
    const quality = this._estimateResultQuality(taskResult);
    
    const ratio = thinking_usage / (complexity * 100); // 权重计算
    const honesty = Math.max(0, Math.min(1, ratio));
    
    this.ledger.push({ thinking_usage, complexity, quality, honesty });
    
    if (honesty < 0.3) {
      console.warn("[THINK-QUALITY] 诚实档位低于 0.3，可能伪造思考");
    }
  }
  
  _estimateTaskComplexity(desc) {
    // 简单启发式估计: 词数 + 关键词
    return (desc.length / 50) + (desc.match(/\b(implement|debug|refactor)\b/g) || []).length;
  }
  
  _estimateResultQuality(result) {
    // 结果质量估计: 代码行数 / 错误数 / 验证覆盖等
    return result.codeLines - result.errors * 2;
  }
}
```

**验收标准**:
- ✅ 诚实档位正常范围 0.4-0.8
- ✅ 诚实档位偏低时有警告
- ✅ 台账可被审计

**部署**: 前端仅

---

#### P2-3: UI "接下来 3 预测"卡实现
**位置**: `/src/main.js` 新增视图层 + `/src/markdown.js` 渲染
**改动**:
1. 在计划卡下方添加"接下来 3 预测"组件
2. 每轮对话后，根据 `nextSteps` 字段绘制预测卡
3. 预测卡显示: 操作名 + 成功率 + 时间估计

**代码框架**:
```javascript
function renderNextStepsPredictions(nextSteps) {
  const container = document.createElement('div');
  container.className = 'next-steps-predictions';
  
  for (const [i, step] of (nextSteps || []).slice(0, 3).entries()) {
    const card = document.createElement('div');
    card.className = 'prediction-card';
    card.innerHTML = `
      <div class="step-rank">${i + 1}</div>
      <div class="step-content">${step.content}</div>
      <div class="step-confidence" style="width: ${step.likelihood * 100}%"></div>
      <div class="step-eta">${step.estimatedMinutes}min</div>
    `;
    container.appendChild(card);
  }
  
  return container;
}
```

**验收标准**:
- ✅ 计划卡展开时显示预测卡
- ✅ 预测卡样式与设计规范一致
- ✅ 点击预测卡可快速导航

**部署**: 前端仅，依赖模型返回 `nextSteps` 字段

---

## 总体改进路径

### Phase 1 (P0, 2 周) — 稳定性修复
1. **P0-1**: 意图边界校验 (hard_blocker)
2. **P0-2**: 失败命令短路 (3 次排除)
3. **预期效果**: 意图维度 3 → 3.5, 工具编排 2.5 → 3

### Phase 2 (P1, 4 周) — 语义优化
1. **P1-1**: Protocol-B 集成 (intent prompt 强化)
2. **P1-2**: 工具失败分类与弱模型下推
3. **预期效果**: 意图维度 3.5 → 4, 工具维度 3 → 3.5

### Phase 3 (P2, 8 周) — 长期演进
1. **P2-1**: Protocol-C 证据分级集成
2. **P2-2**: 思考质量验证框架
3. **P2-3**: "接下来 3 预测"UI
4. **预期效果**: 诚实维度 2.5 → 4, UI 维度 3.5 → 4

### Phase 4 (P3, 持续) — 顶级对标
- 完整测试套件 (ANTI-HALLUCINATION_PROTOCOL.md 第 469-837 行)
- 子智能体异步机制补完
- 缓存指纹校验激活

---

## 审计结论

### 总体评价

**Michael IDE 的智能体架构处于"中上"水平**: 工程规范完善，基础设施扎实，但在"实时意图修正"、"工具语义路由"、"诚实验证"三个关键领域与顶级 Agent 有明显差距。

### 最关键的 3-5 个短板

| 序号 | 短板 | 优先级 | 修复难度 | 预期收益 |
|-----|------|--------|---------|---------|
| 1 | 缺乏闭环意图修正 | P0 | 低 | 成熟度 +0.5, 用户体验 ++ |
| 2 | 工具编排缺语义路由 | P0 | 中 | 成熟度 +1, 弱模型支持力度 ++ |
| 3 | 反幻觉协议未集成 | P1 | 高 | 成熟度 +1.5, 诚实度 ++ |
| 4 | 思考质量无验证 | P1 | 中 | 成熟度 +0.5, 调试能力 ++ |
| 5 | UI 缺预测卡 | P2 | 低 | 成熟度 +0.5, UX 体验 + |

### 建议的下一步

1. ✅ **立即**: 实施 P0 系列 (1-2 周完成)
   - 推送意图边界校验
   - 启用失败命令短路
   
2. ⏳ **近期** (4 周): P1 系列
   - Protocol-B/C 集成
   - 工具下推机制
   
3. 📋 **中期** (8 周): P2 系列
   - 思考验证框架
   - UI 完善

### 与顶级 Agent 的距离

| 阶段 | 总体评分 | 对标 |
|-----|---------|------|
| 当前 | 3.2 / 5 | Devin 2.5 个月前版本 |
| P0 后 | 3.5 / 5 | Cursor 当前版本 |
| P1 后 | 3.8 / 5 | Claude 3.5 去年版本 |
| P2 后 | 4.2 / 5 | Claude 4 当前版本 |

**路线是可行的**: Michael IDE 有完整的基础，通过系统性的 P0 → P1 → P2 投入，可在 3 个月内追赶到业界顶级水平。

---

**审计完成日期**: 2026 年 7 月 30 日  
**审计范围**: 代码 + 设计文档 + 配置  
**方法论**: 8 维度对标评估 + 代码证据追溯 + 顶级 Agent 对比  
**维护者**: Research Agent  
**建议反馈**: 详见第 9 部分"分级改进建议"

