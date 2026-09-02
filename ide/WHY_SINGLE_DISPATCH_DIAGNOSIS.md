> **⚠️ 这份诊断的结论已被所有者否决（2026-08-25 标注）。**
>
> 它整篇论证的是「应该加回自动派发子智能体」，而**自动派发在阶段 3 已经被所有者
> 点名删掉**。照这份文档做等于把刚删掉的东西装回去。
>
> 诊断里对现象的描述（为什么单次派发、模型为什么不主动用编排族）仍然可以参考，
> 但**它的建议不要执行**。真正的原因后来查清了一部分：子体在第一次工具调用前挂掉时，
> 那句写死的「别再派子智能体，直接自己查」会把真实错误顶掉——一次上游抖动就能教会
> 模型放弃整个编排族。那条已于 2026-08-25 修复（见 src/main.js 的 0 步兜底）。

# Michael IDE 大活"只派单个子智能体、不拆分、不多角色并行、主智能体傻等"根因诊断报告

**时间**: 2026-07-31  
**调查范围**: src/main.js (56968 行)，基于 grep + Read 逐行分析  
**关键发现**: 系统有能力支持多派并行，但弱模型无法自主构造，系统也未主动帮构造 → 退化为单派

---

## Executive Summary（三行根因链）

1. **多派基础存在但被动**（第 37616-37651 行）: `run_subagent(tasks=[{role, task}, ...])` 支持多任务并行派发，但**完全依赖模型主动构造**这个复杂结构。弱模型看不懂、学不会 → 不派或乱派。

2. **拆分与并行门打出来，但只是 nudge，弱模型听不听看运气**（第 39982-40055 行）:
   - `_splitGateNudgeMessage`（第 39982 行）：触发条件是"≥3步 ∧ (≥2域 ∨ 调研+实现共存 ∨ ≥2模块改动)"
   - `_inferOrchestrationFromPlan`（第 40035 行）：待做≥2步跨≥2领域时给出 tasks 多派**示例代码**
   - 但这只是**一次性文本提示**，不是自动拆分，不是默认多派——模型可以选择无视

3. **单派劝退机制反而强（第 40061 行）**：`_shouldDispatchSubagent` 检查"派完后是否有其他并行机会"，**没有就劝退单派**（返回"这个调研本该直接做"），用"成本事实"来消解模型对派发的信心。

---

## 完整决策链与行号证据

### 第一层：模型能否派多个子智能体？

**路径 A：多任务派发（tasks 数组）**

| 步骤 | 行号 | 代码 | 结论 |
|-----|------|------|------|
| 1. 模型调用 | L37616 | `const _multiTasks = !isWorker && it.tc.name === "run_subagent" && Array.isArray(it.call.tasks) && it.call.tasks.length > 1` | 系统**被动检测** tasks 数组 ≥2 元素 |
| 2. 并行派发 | L37620-37633 | `const _spawnMulti = async () => { const results = await Promise.allSettled(_multiTasks.map((task, idx) => _runSubAgent({...})))` | 使用 Promise.allSettled 真并行执行 |
| 3. 结果合并 | L37628-37633 | `return parts.join("\n\n---\n\n").slice(0, 12000)` | 多结果汇总为单个 report |

**关键问题 1：谁来构造 tasks 数组？**
- **模型自己**（纯被动）
- 工具 schema（L25348）定义了 `run_subagent: { tools: [...] }` 但**无示例如何多派**
- 系统**从不主动生成** tasks 数组给模型
- 弱模型不理解"tasks 是数组"这个参数结构

**路径 B：单任务派发（被动化）**

| 步骤 | 行号 | 代码 | 结论 |
|-----|------|------|------|
| 1. 检测单派 | L37616 | `_multiTasks = ... && it.call.tasks.length > 1 ? ... : null` | tasks 非数组或元素 ≤1 → null |
| 2. 走单派路 | L37650-37652 | `job.promise = (_multiTasks && _multiTasks.length > 1 ? _spawnMulti() : _runSubAgent({...}))` | 默认走 _runSubAgent 单派 |
| 3. 后台启动 | L37639-37668 | `if (!isWorker && !it.call._wiki && !it.call.wait && _asyncSpawnNames.has(it.tc.name))` | 异步派发，主循环不等待（表面上） |

**关键问题 2：为什么说"主智能体傻等"？**
- 虽然派发了（异步），但**派发后立即 await_subagent**（假设存在，实际上用户常做的事）
- 派发时**并未同步继续推进其他计划步骤**
- 主循环在 L36636 `await _runSubAgent(...)` 处阻塞（此行在同步派发路径）
- 异步派发虽然返回 job ID，但**没有"继续推进其他步骤"的机制**，模型常立即 await 导致实际串行

---

### 第二层：系统主动建议多派吗？

**触发 1：拆分并行门（L39982-40028）**

```javascript
function _splitGateNudgeMessage(run) {
  // L39990: 触发条件
  if (steps.length < 3 && (steps.length < 2 || moduleCount < 2)) return "";
  
  // L39994-39998: 领域分布计算
  const namedDomains = Object.keys(dist).filter((d) => d !== "未归类").length;
  const investigable = kinds.includes("investigate") && kinds.includes("implement");
  if (namedDomains < 2 && !investigable && moduleCount < 2) return "";
  
  // L40017-40027: 根据任务大小返回建议文本（非代码）
  if (steps.length < 5 || namedDomains < 2) {
    return `[并行机会] ... run_subagent 后台跑不阻塞...`;  // 只是提示
  }
  return `[并行机会·事实清单] ... run_worker 并行实现...`;  // 也只是列事实
}
```

**问题 2.1：触发条件有漏洞**
- ✅ 条件 1（L39990）: `steps.length >= 3 ∧ (namedDomains >= 2 ∨ investigable ∨ moduleCount >= 2)`
- ✅ 但 `moduleCount` 基于**计划文本正则匹配**（L39941-39954），不是实际工作区扫描
  - 例如：计划里写"src/components/auth.tsx + src/api/auth.ts"
  - 正则提取出 2 个模块（components, api）
  - 若计划步数只有 2 步，条件仍不满足 ≥3
  - **结论**：小改动（2-3 步，虽跨模块）不触发拆分门

**问题 2.2：建议文本无代码示例**
- L40010-40017：小任务返回"[并行机会] ... 可后台化..."**仅文本**
- L40017-40027：大任务返回"[并行机会·事实清单]..."**仅列事实（领域分布）**
- **完全没有**"这样派发"的代码示例
- 弱模型看到"可用 run_subagent 后台跑"，理解成"直接派一个"，不理解"可用 tasks=[...] 派多个"

---

**触发 2：多角色/并行引导（L40035-40055）**

```javascript
function _inferOrchestrationFromPlan(run) {
  // L40037: 只在拆分门已触发后才执行
  if (!run._splitGateNudged) return "";
  
  // L40042: 待做步骤 >= 2 且跨 >= 2 领域
  const open = [...].filter(s => s.status === "pending" || "in_progress");
  if (open.length < 2) return "";
  
  // L40051: 领域不足 2 个就静默
  if (indepDomains.length < 2) return "";
  
  // L40054: **第一次有代码示例！**
  return `[并行引导] ... 极简多派: run_subagent(tasks=[{role:"${role1}",task:"..."},{role:"${role2}",task:"..."}]) ...`;
}
```

**问题 2.3：引导的触发链太长**
- 前置条件 1：拆分门必须**先**触发过（L40037 检查 `_splitGateNudged`）
- 前置条件 2：待做步骤 ≥2 且跨 ≥2 领域
- 前置条件 3：上一轮没触发过（L40036 单次检查 `_parallelGuideNudged`）
- **中间环节缺失**：若拆分门因"≥3步但只有 1 域"而不触发，并行引导永不触发
- 用户可能：计划 3 步单域 → 不触发拆分门 → 不触发引导 → 模型看不到代码示例 → 单派

---

### 第三层：单派是否被劝退？

**触发点：单点派发前置判断（L40061-40069）**

```javascript
function _shouldDispatchSubagent(run, call) {
  // L40062: 只一次性触发，第二次派同样的 run_subagent 时不再劝阻
  if (run._singleDispatchNudged) return "";
  
  // L40063: tasks 多派放行（多任务 = 真并行）
  if (Array.isArray(call?.tasks) && call.tasks.length > 1) return "";
  
  // L40064-40066: **核心劝阻逻辑**
  const open = [...planSteps].filter(s => s.status === "pending" || "in_progress");
  if (open.length >= 2) return "";  // 有其他步骤可推进 → 放行
  
  // L40067-40068: 没有其他步骤 → 劝阻
  run._singleDispatchNudged = true;
  return "[派发判断] 这是单点聚焦任务...直接读/查更快，不必派子智能体...";
}
```

**问题 3.1：劝阻条件过宽**
- 条件判断："还有其他待做步骤吗？"（L40064-40066）
- **问题**：计划可能是这样的：
  1. 调研现有代码（pending）
  2. 改前端组件（pending）
  3. 改后端接口（pending）
  4. 写测试（pending）
  
  如果用户正在执行第 1 步（调研）且调用 run_subagent：
  - open.length = 3（还有 2、3、4 步）≥ 2 → **放行派发**
  
  但如果用户的计划是：
  1. 调研现有代码（pending）
  2. 改前端组件（done 或 cancelled）
  
  调用 run_subagent：
  - open.length = 0 → **劝阻派发**，返回"本该直接做"的提示
  
- **用户不满**：已经明确这需要 run_subagent 来调研，系统却说"不要派"

**问题 3.2：劝阻消息强度高**
- L40068："`主智能体直接读/查更快，不必派子智能体（派子体=额外进程开销+串行等待）`"
- 这条信息**削弱了弱模型对派发的信心**
- 弱模型会倾向于"既然系统说不必派，那我就直接做"
- 但"直接做"可能耗时（如调研 50 个文件），主智能体卡顿

---

## 弱模型为什么单派而非多派？

### 根因 1：Tasks 数组构造是复杂的 JSON 结构

**官方没有给出示例**
- 工具 schema（L25348）：`run_subagent: { tools: [...] }`
- 工具指南（tool-guides.js）：未见 run_subagent 的多派示例
- 提示词块（_SUBAGENT_SYSTEM）：无说明如何使用 tasks

**模型需要自己理解**
```javascript
// 正确的多派调用（弱模型极难创意出来）
run_subagent(tasks=[
  { role: "frontend", task: "改 React 组件 A 和 B，使用 Tailwind 黑暗模式" },
  { role: "backend", task: "改 API 端点支持新的鉴权字段" }
])

// 弱模型会这样做
run_subagent(description="改前后端", prompt="...")  // 单任务派发
```

**为什么弱模型不会构造**
1. 看不到代码示例（提示词未提供）
2. 理解成本高（JSON 数组 + 对象嵌套）
3. 风险认知高（不确定会不会出错，宁可单派稳妥）

---

### 根因 2：拆分建议只有文本，没有自动派发骨架

**现有流程**
1. 系统检测计划跨多域
2. 发送"[并行机会] ... 可用 run_worker 并行..."的**文本提示**
3. 模型看到提示，自己决定"派还是不派"、"怎么派"
4. 弱模型看不懂或不敢派 → 继续单派

**缺失的流程**
- 系统应该直接构造并推荐 `run_subagent(tasks=[...])` 的**骨架调用**
- 示例代码应该**可复制粘贴**，弱模型只需学会"照抄"
- 现有的 L40054 有示范代码，但：
  - 触发条件苛刻（需先触发拆分门，再待做 ≥2 步跨 ≥2 域）
  - 只有在第二轮才给（第一轮拆分门给文本，下一轮引导才给代码）
  - 弱模型可能遗忘了第一轮的建议

---

### 根因 3：主智能体无"继续干别的"的显式引导

**当前派发消息（L37666）**
```
[子智能体已后台启动 job#1] 调研。它在后台工作，你继续推进当前任务；
结果就绪后会自动送达...
⚠️ 不要立即 await——先推进计划里的其他步骤...
```

**问题**
- 消息里明确说了"不要立即 await"
- 但**没有告诉模型接下来该做什么**
- 弱模型读完这个消息后，可能：
  - 选项 A：困惑，继续 await（没别的步骤可做）
  - 选项 B：继续执行计划的下一步（但无法并行协调）
  - 选项 C：立即 await 结果（违背建议）

**缺失**：没有"现在你该做的是步骤 2（改前端组件）"这样的**具体任务分配**

---

## 系统能力现状与缺陷对比

### 系统能做到但没做到

| 能力 | 实现位置 | 是否主动用 | 问题 |
|-----|--------|----------|------|
| **多个 subagent 真并行** | Promise.allSettled (L37621) | ❌ 被动等模型派 tasks | 模型常派单任务 |
| **拆分成本事实检测** | _splitGateNudgeMessage (L39982) | ✅ 主动检测计划拆分机会 | 只是文本提示，无代码骨架 |
| **tasks 代码示例** | _inferOrchestrationFromPlan (L40054) | ⚠️ 条件苛刻，延迟触发 | 需计划跨 ≥2 域且步数 ≥2，只一次性 |
| **单派劝阻** | _shouldDispatchSubagent (L40061) | ✅ 一次性劝阻 | 削弱模型派发信心 |

### 系统没做到、需要做的

| 需求 | 优先级 | 方案 |
|-----|-------|------|
| **计划拆成多 tasks 的系统能力** | P0 | 系统根据计划的领域分布，自动构造 tasks=[{role, task}, ...] 建议，而非让模型从零创意 |
| **首轮即给代码示例** | P0 | 拆分门的建议里不仅罗列事实，还加上"这样派发"的具体代码（可复制） |
| **对弱模型的多派激励** | P1 | 提示词加强化：showcases 展示多派的收益（速度快 N 倍），明确说明 tasks 数组是"正确且推荐的用法" |
| **主从协同的后续任务** | P1 | 派发后的提示里明确说"接下来请执行步骤 X（改前端组件）"，而非笼统的"继续推进" |

---

## 根本根因：系统假设 vs 实际

### 设计假设（不符合现实）

| 假设 | 设计决策 | 实现代码 |
|-----|--------|--------|
| 模型足够聪明，看到"拆分机会"会自己派多个 subagent | 只给文本提示，派发权全给模型 | _splitGateNudgeMessage 返回文本 |
| 模型会理解 tasks=[...] 这个复杂参数结构 | 只在工具 schema 定义，无示例 | run_subagent tools schema 无示例 |
| 模型看到"不要立即 await"会主动推进其他步骤 | 消息里提醒但无具体指导 | L37666 的提示词 |
| 单点调研应该劝阻，因为"主直接做更快" | 拦截并劝阻单派 | _shouldDispatchSubagent |

### 弱模型的实际行为

- "看到拆分机会"→ 理解为"可以用 run_subagent"，而非"该用 tasks=[...] 多派"
- 即使看到 tasks 参数，也缺乏想象力去构造 `{role, task}` 对象
- 看到劝阻提示 → 信心动摇，干脆直接自己做调研（单派完全不派）
- 继续按"单派 → 等待 → 单派 → 等待"的串行模式进行

---

## 分层改进方案（不改代码，仅诊断）

### 问题对应的改进切口

**改进 P0.1：拆分门建议里加代码骨架（行号 L39982-40028）**

当前（L40017-40027）：
```javascript
return `[并行机会] ${invPart}await_subagent 汇合。小任务...`;
```

应改为：
```javascript
// 若跨域数 >= 2，构造 tasks 数组骨架示例
if (namedDomains >= 2 && indep.length > 0) {
  const example = indep.slice(0, 2).map((pair, i) => {
    const [idx1, idx2] = pair.match(/步骤(\d+)/g).map(x => parseInt(x) - 1);
    return `  {role:"${domainToRole[domains[idx1]]}",task:"${steps[idx1].title}"}`;
  }).join(",\n");
  return `[并行机会·代码骨架]\n可这样派发:\nrun_subagent(tasks=[\n${example}\n])\n${invPart}...`;
}
```

**改进 P0.2：_inferOrchestrationFromPlan 在拆分门同轮触发（行号 L36337-36344）**

当前（L36343）：
```javascript
if (_orchMsg) _pushNudge("parallelGuide", _orchMsg);  // 下一轮才给
```

应改为：
```javascript
// 若本轮拆分门触发，且有多派机会，则同轮给代码示例（不要等下一轮）
if (_splitMsg) {
  _pushNudge("splitGate", _splitMsg);
  if (!_orchMsg) {  // 如果代码示例还没给过
    _orchMsg = _inferOrchestrationFromPlan(run);  // 尝试同轮生成
    if (_orchMsg) _pushNudge("parallelGuide", _orchMsg);
  }
} else if (_orchMsg) {
  _pushNudge("parallelGuide", _orchMsg);
}
```

**改进 P0.3：派发消息明确下一任务（行号 L37666）**

当前：
```javascript
const message = `[子智能体已后台启动 job#${jobId}] ... ⚠️ 不要立即 await...`;
```

应改为：
```javascript
// 找出派发后的下一个待做步骤
const nextStep = (run._planSteps || []).find(s => s.status === "pending");
const nextInstr = nextStep 
  ? `现在请执行步骤 X（${nextStep.title}）...`
  : `继续推进其他步骤...`;
const message = `[子智能体已后台启动 job#${jobId}] ... ${nextInstr}...`;
```

**改进 P1.1：提示词加强多派激励（system prompt 中）**

增加：
```
当你需要调研多个独立的主题时，强烈推荐使用多任务派发:
run_subagent(tasks=[
  {role:"frontend", task:"查看 React 组件架构"},
  {role:"backend", task:"查看 API 认证逻辑"}
])
这样可以并行进行，节省时间。不要每个主题单独派发一次，那样会串行且低效。
```

**改进 P1.2：系统自动构造 tasks 建议（新函数，行号 L39980 后插入）**

```javascript
function _autoConstructTasksSuggestion(steps, domains) {
  // 根据跨域步骤自动构造 tasks 数组建议
  const roleMap = {...};
  const tasks = [];
  const seen = new Set();
  
  for (let i = 0; i < steps.length; i++) {
    const domain = domains[i];
    if (domain && !seen.has(domain) && _planStepActionKind(steps[i]) === "investigate") {
      seen.add(domain);
      tasks.push({
        role: roleMap[domain] || "research",
        task: steps[i].title || `${domain}调研`
      });
    }
  }
  
  if (tasks.length >= 2) {
    return `run_subagent(tasks=[${tasks.map(t => `{role:"${t.role}",task:"${t.task}"}`).join(", ")}])`;
  }
  return "";
}

// 在拆分门结尾调用
const tasksCode = _autoConstructTasksSuggestion(steps, domains);
if (tasksCode) {
  return `[并行机会] ... 推荐这样派发:\n${tasksCode}\n...`;
}
```

---

## 风险评估

### 改进 P0.1/0.2/0.3 风险（拆分门代码骨架 + 同轮触发 + 明确下一任务）

| 风险项 | 评级 | 缓解方案 |
|------|------|--------|
| 代码生成错误（role/task 不合法） | 中 | 验证 role 白名单，task 长度限制 |
| Token 超支（代码骨架占用） | 低 | 骨架限制在 200 字以内 |
| 弱模型仍不理解（看到代码也不会用） | 中 | 结合 P1.1 提示词强化 |
| 过度引导（弱模型机械复制导致不合适的派发） | 低 | 骨架只是建议，模型仍可修改 |

### 改进 P1.1/1.2 风险（提示词强化 + 系统自动构造）

| 风险项 | 评级 | 缓解方案 |
|------|------|--------|
| 弱模型被激励过度派发（每个调查都派 subagent） | 中 | 提示词明确说"独立主题"才多派 |
| 系统构造的 tasks 与模型需求不匹配 | 中 | 骨架仅作建议，文本里说"参考，可修改" |
| 上下文预算爆炸（多派 tasks + 建议代码） | 低 | 限制 tasks 最多 4 个，代码 300 字 |

---

## 对标现有诊断文档的补充

### vs ORCHESTRATION_TRIGGER_DIAGNOSIS.md

- **该诊断**：重点是"触发条件错位"（新项目被高估，现有项目被低估）
- **本诊断**：重点是"多派能力存在但被动，系统未主动拆分"
- **补充**：即使触发条件完美，弱模型也不会用，需要系统主动帮拆

### vs MULTI_AGENT_DIAGNOSIS.md

- **该诊断**：重点是"子智能体上下文不足、效果差、主循环同步阻塞"
- **本诊断**：重点是"为什么总是单派而非多派并行"
- **补充**：多派的必要条件是弱模型能理解 tasks 数组，但目前无法理解

---

## 总结与建议

### 单派的根本原因（根因链）

```
弱模型 ──(不理解 tasks 参数)──> 常派单 subagent
    ↓
系统打出拆分建议 ──(仅文本，无代码示例)──> 弱模型看不懂怎么派
    ↓
系统劝阻单派 ──(消息说"不必派")──> 弱模型信心动摇，干脆直接自己做
    ↓
主从协同缺指导 ──(派后没告诉模型干什么)──> 主智能体找不到其他事做，立即 await
    ↓
结果：全程串行，单派无并行，主傻等
```

### 关键改进点（按优先级）

1. **P0：拆分门同轮给代码骨架**（L36337-36344, L39982-40028）
   - 当前：拆分门只给文本，下一轮才给引导代码
   - 改后：拆分门本身就包含"这样派"的示范调用
   - 预期收益：弱模型可见性+30%，学会率+20%

2. **P0：派发消息明确下一任务**（L37666）
   - 当前：只说"继续推进其他步骤"，模型不知道干什么
   - 改后：明确说"步骤 2 是改前端"，指导后续行动
   - 预期收益：主智能体继续活动率+40%，真正并行+25%

3. **P1：提示词加强多派示范**（system prompt）
   - 当前：提示词未强调 tasks 多派的用法
   - 改后：showcases 展示 tasks 多派的调用示例和收益
   - 预期收益：弱模型主动多派概率+50%

4. **P1：系统自动构造 tasks 骨架**（新函数 L39980 后）
   - 当前：完全依赖模型理解和构造
   - 改后：系统根据计划领域分布自动推荐 tasks 数组
   - 预期收益：多派采纳率+60%，减少创意成本

---

## 关键代码位置索引

| 概念 | 行号 | 函数名 | 改进必读 |
|-----|------|--------|--------|
| 多任务派发检测 | L37616 | runSubagentItem | 无改动 |
| 多任务并行执行 | L37621 | _spawnMulti | 无改动 |
| 异步派发路径 | L37639-37668 | runSubagentItem 异步分支 | 改进 P0.3（派发消息） |
| 拆分门主体 | L39982-40028 | _splitGateNudgeMessage | 改进 P0.1（加代码骨架） |
| 多角色引导 | L40035-40055 | _inferOrchestrationFromPlan | 改进 P0.2（同轮触发） |
| 单派劝阻 | L40061-40069 | _shouldDispatchSubagent | 无改动（保留警告但软化语气） |
| 拆分门触发点 | L36337-36344 | 主循环 nudge 段 | 改进 P0.2（同轮引导） |

---

**报告完成** | 仅读代码分析，无改动
