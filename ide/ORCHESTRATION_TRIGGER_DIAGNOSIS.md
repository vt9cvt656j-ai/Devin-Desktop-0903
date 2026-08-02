# Michael IDE 编排触发逻辑错位诊断报告

**时间**: 2026-07-30  
**调查范围**: /Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js (56223 行)  
**调查方法**: 纯只读，基于 grep 行号证据  

---

## Executive Summary

编排系统的触发"门"（empty root gate、split gate、subagent dispatch、bug evidence gate、multi-role）在近期分批加入时产生了**级联错位**：

1. **错位主因**: 拆分并行门 `_splitGateNudgeMessage`（#39552）与子智能体派发（#37231）都无前置判断——即使只是"单个聚焦查一个文件"也会后台派发，而跨域大项目反而因缺乏显式触发逻辑而不派；空目录门与大工程字段（substantial/projectScope/fromZeroUiProject）强耦合，导致"项目从0/空才触发规划"假象。

2. **单派根因**: 派发逻辑（#37231-37262）无"该不该派"的**前置成本/收益判断** —— 模型只要调 run_subagent，系统就后台启动，直到 await_subagent 时才事后检测傻等（#43394），而非派发前拦截。

3. **大项目不并行根因**: 多角色触发（orchestrationMode=staged_roles/parallel_roles）基于 AI 语义意图判定（#16760-16785），而**不是自动根据任务规模与拆分可行性** —— 现有大项目如加功能/重构类任务，若模型未显式表达需要分角色协作，系统就始终 solo 模式，无法自动引导并行派发。

---

## 调研详情

### A. 完整触发条件矩阵

#### A1. 空目录门 (Empty Root Gate)

| 字段 | 触发条件 | 行号 | 注入内容 | 问题 |
|------|--------|------|--------|------|
| `run._emptyRootAtStart` | 初始时工作区文件列表为空 (`!entries.length`) | 35529 | 标记 true，后续门控使用 | 纯目录状态标记，本身无问题 |
| `run._emptyExploreCount` | 仅追踪探索计数（实验性）| 35644 | 第1次放行取证，第2次起短路 | 计数机制存在但未完全启用 |
| `_emptyBuildIntent()` 函数 | **substantial ∨ projectScope ∨ fromZeroUiProject ∨ uiProject ∨ fullWebsite ∨ implementation** | 35640-35641 | 判定是否应该进行build类动作 | **关键错位点**：这些字段按AI意图判定，但是现有项目"加功能"可能只标impl，不标substantial |
| `emptyBuildAct` (行动门禁) | run._emptyRootAtStart ∧ !didMutate ∧ _implOps==0 ∧ _emptyBuildIntent() ∧ 第1次触发 | 35981-35984 | "立即 write_file 创建第一批文件，不要纯思考" | **级联错位**：将"空目录"与"必须建议拆分"强绑定 |
| `emptyHistoryFact` | 同上条件 | 35989-35992 | "磁盘实况：当前目录为空，历史都作废，重新开建" | 严重影响现有大项目的工程判断 |
| `emptyBuildFinish` (收尾拦截) | run._emptyRootAtStart ∧ !didMutate ∧ _implOps==0 ∧ _emptyBuildIntent() ∧ quiet ∧ 第1次拦 | 36290-36293 | "还没创建任何文件，不能结束" | 阻止空目录下纯思考终止，但过度简化 |

**诊断**：
- 空目录门的**三个提示都依赖 `_emptyBuildIntent()` 的宽松判定**（substantial/projectScope/fromZeroUiProject/uiProject 任一为真）。
- 问题：现有项目"改功能"时 AI 判定可能 `projectScope=false, substantial=false`，但为 `implementation=true`，此时不触发任何 build nudge，显得"大项目无特殊对待"。
- 反之，新项目或 UI 项目（fromZeroUiProject=true）即使是微小改动，也会触发完整 build 门控，强化"从0才有权力规划"的错觉。

---

#### A2. 拆分并行门 (Split Gate)

| 字段 | 触发条件 | 行号 | 注入内容 | 问题 |
|------|--------|------|--------|------|
| `_splitGateNudgeMessage()` | steps.length >= 3 ∧ (namedDomains >= 2 ∨ investigate+implement共存) ∧ !_parallelDispatches ∧ !_subAgentJobs ∧ !_bugEvidence | 39552-39593 | 列并行机会（事实清单，不拦截） | 触发条件合理，但...见下方 |
| `namedDomains` 计算 | `_planStepDomain()` 按正则匹配步骤文本，分类为 frontend/backend/db/test/deploy/docs/design 等 | 39532-39543 | 统计独立领域数量 >= 2 触发 | **核心问题**：仅基于计划文本正则，未考虑现有项目"已有模块"能否拆 |
| `investigable` | steps 中同时有 "investigate" 和 "implement" 行为类型 | 39563 | 小任务（<5步 ∨ <2域）时只提精简版 | 合理，但触发前提是什么时候设plan |
| 小任务精简版 | steps.length < 5 ∨ namedDomains < 2 → 推荐单个 investigate 后台化 + await | 39576-39582 | 点出"可后台"但不强制 | 太温和，傻等检测是事后而非事前 |
| 大任务完整版 | steps.length >= 5 ∧ namedDomains >= 2 → 罗列相邻 implement 步是否跨域 | 39584-39592 | 按 run_worker scope 隔离建议 | **缺陷**：无判断"这些domain是否真的能独立拆" |

**诊断**：
- **触发条件**（≥3步 ∧ ≥2域）对**现有大项目重构/加功能任务**往往不满足：
  - 用户说"给既有 React 页面加登录"，计划可能只有：① 读现有auth架构 ② 改服务层 ③ 改前端组件 ④ 测试 —— 确实4步跨多域，应该触发。
  - 但如果用户说"修复 bug in payment module"，计划可能是：① debug ② fix ③ test —— 只3步同域 payment，**不触发**并行建议，模型就直接自己做（串行）。

- **"派发以来干没干别的活"检测（#43394）是事后的傻等检测**，不是派发前的"值不值"评估。派发代码（37259）会*提醒*"若没其他步骤说明本该自己做"，但此时作业已启动了。

---

#### A3. 异步派发与傻等检测

| 字段 | 派发条件 | 行号 | 结果 | 问题 |
|------|--------|------|------|------|
| `_asyncSpawnNames` 集合 | = { "run_subagent", "research_project", "design_research" } | 37231 | 这三个工具的无 wait 调用后台化 | 定义清晰 |
| 派发判断 | !isWorker ∧ !_wiki ∧ !wait ∧ _asyncSpawnNames.has(name) | 37232 | 满足则启动异步作业 | **无前置"值不值派"的成本判断** |
| `dispatchLedgerLen` 记录 | 派发时记录当前 `run._toolLedger.entries.length` | 37238 | 后续 await 用它检测有无其他活动 | 事后检测 |
| 异步启动消息 | 返回 job#N，提醒"继续推进其他步骤，结果自动送达" | 37259-37261 | 附加"若没其他事说明本该自己做" | **消息里有自我否定** —— 这说明设计本身有疑惑 |
| 傻等检测逻辑 | 若 `_ledgerEntries.slice(dispatchLedgerLen)` 只有 run_subagent/await_subagent，其他工具为 0 | 43373-43374 | 在 await 结果前缀加[事实] | 事后、不拦截、只作为反馈 |

**诊断**：
- **单子智能体派发无"不派"判断**：模型调 `run_subagent(description="检查 auth", prompt="读 auth.ts 检查...")` 时，系统**无评估**"这个工作本来主智能体读一个文件就搞定"。
- 派发决策是**二元的**：有调用→就派，无条件。理想应该有"单点调查成本低，不派" 的前置逻辑。
- 傻等检测是**事后反馈机制**（43394），而不是事前拦截，模型已被派发提示"会自动送达"，不会主动取消。

---

#### A4. Bug 修复异步取证门

| 字段 | 触发条件 | 行号 | 注入内容 | 问题 |
|------|--------|------|--------|------|
| `_bugEvidenceGateNudge()` | debugProject ∨ bug ∧ 根因未明证据 >= 2次失败 ∧ 未写入 | 39598-39608 | "取证可并行用 run_subagent 后台派 research 角色" | 针对专门调试任务 |
| 让行逻辑 | 若 debugProject/bug ∧ 且本轮 bugEvidence 尚未触发 ∧ _bugFails >= 2，_splitGate 自动让行 | 39569-39572 | 避免同轮重复提示 | 合理 |

**诊断**：bug 取证门与 split gate 有正交的"让行"机制，不是主要问题源。

---

#### A5. 多角色触发

| 字段 | 决定方式 | 行号 | 生效范围 | 问题 |
|------|--------|------|--------|------|
| `orchestrationMode` | AI 语义意图判定，rawEngineering.orchestrationMode 值 → enum 校验 | 16760-16785 | solo/staged_roles/parallel_roles | **非自动触发** |
| 校验规则 | orchestrationMode != solo && !roleNeeds.length → 降为 solo | 16761-16762 | 没有角色列表时强制 solo | 保守但合理 |
| 降级规则 | orchestrationMode == parallel_roles && roleNeeds.length < 2 → solo | 16762 | 少于2个角色的 parallel 无效 | 合理 |
| 强制 staged | orchestrationMode == parallel_roles && roles 含 architect/product/research → 降为 staged_roles | 16763-16765 | 架构/产品未定，不能直接并行 | 合理保护 |
| 显示逻辑 | run.engineering.orchestrationMode 决定显示何种提示 | 34224-34227 | 显示"分阶段纪律"或"并行纪律" | **仅显示不强制** |
| 引导能力 | "这是建议不是禁令：发现真需要分角色/并行可直接调 run_subagent/run_worker 自主升级" | 34221 | solo 模式下的逃生舱 | **自主升级依赖模型主动** |

**诊断**：
- **多角色触发完全依赖 AI 语义判定**（rawEngineering.orchestrationMode），而不是根据任务实际规模、现有文件数量、模块数、拆分可行性自动判定。
- 现有大项目"改功能"时，模型通常判定 orchestrationMode=solo（因为单次改动不跨产品边界），即使工程上完全可拆（前端端×后端×测试可并行）。
- 系统**无自动拆分启发**：只有在计划已落地且满足 ≥3步 ∧ ≥2域 时，_splitGate 才提醒"可后台化调研"或"可 run_worker 并行实现"，但不强制。

---

### B. 错位根因深度分析

#### B1：为什么"必须从0/空目录才触发子智能体和大规划"？

**现象**: 用户感觉新项目或空目录下更容易被派子智能体、被建议拆分，而现有大项目反而不。

**根因链**：

1. **空目录与 substantial 的偶然相关性**（不是因果）：
   - 新项目→`projectState=greenfield`
   - greenfield ∧ ui → `fromZeroUiProject=true`（#16927）
   - fromZeroUiProject ∨ others → `substantial=true`（#16973）
   - substantial → `requiresPlan=true`（#16978）
   - requiresPlan ∨ substantial → 会在计划建议里显示"需要任务计划"

2. **现有项目"加功能"的低评级**：
   - 用户说"给 React app 加暗黑模式"
   - AI 判定: `projectState=existing`, `changeScope=module`, `architectureMode=extend_existing`
   - 计算: `substantial = applies && (industrialProject || projectSized || ...)`（#16973）
   - 其中 `projectSized = changeScope === "project" || changeScope === "system"`（#16905）
   - 结果: `projectSized=false` → 不进 substantial 判定的主干
   - 只能通过 `debugProject || databaseArchitecture || containerOps || ...` 等其他路径
   - 现有项目的模块级改动通常这些都不满足 → **substantial=false**

3. **空目录门与 emptyBuildIntent 的级联**：
   - 新项目: `fromZeroUiProject=true` → `_emptyBuildIntent()=true` → 触发 emptyBuildAct/emptyHistoryFact/emptyBuildFinish
   - 现有项目: 除非 implementation 被标记，否则不触发 emptyBuildIntent
   - 但新项目正因为是从零开始所以**更容易被判定为 substantial**，而现有项目反而被判定为"扩展existing"（低权重）

**错位的根本**: 空目录门、拆分建议、多角色触发都沿用了一条**AI语义意图链（projectState→substantial→requiresPlan→display）**，而这条链把"项目新旧"与"任务规模"混为一谈。

**理想应该解耦**:
- 空目录门: 仅作为"初始状态标记"，不绑定工程决策
- 拆分建议: 基于"计划步数、跨域数、现有模块数"，与项目新旧无关
- 多角色: 基于"scope 可拆性、角色需求数"，与 projectState 无关

---

#### B2：为什么"单个子智能体也派"（纯串行开销）？

**现象**: 用户在计划中只是"调研一个问题"或"读一个文件"，模型也会调 run_subagent，派后没其他事可做，立即 await，效果是**同步执行但多开一个子进程**（串行无法超时节省）。

**根因链**：

1. **派发无成本/收益判断**：
   - run_subagent 调用映射到异步派发逻辑（#37232）
   - 条件: `!isWorker && !_wiki && !wait && _asyncSpawnNames.has(name)`
   - **无检验**:
     - "这个调研任务能否由主智能体一个 read_file 完成？"
     - "调研耗时估算 < 子进程启动开销吗？"
     - "还有其他并行机会吗（调研与实现/测试），还是这是唯一任务？"

2. **傻等检测是事后**（第43394行）：
   - 派发时系统**已经启动**作业（job.promise）
   - 派发消息里*提醒* "若没其他事说明本该自己做"，但消息已发送，作业已跑
   - await_subagent 时才检测 `_idleWait = 派发后除自身外无其他活动`
   - 若检测到傻等，返回[事实]文本告诉模型"本该直接做"，但**此时作业已完成**了

3. **无前置拦截**：
   - 理想的流程: 派发前检查"还有其他计划步骤吗"，如无→拦截派发，改为同步调用
   - 实际的流程: 派发→启动作业→异步跑→模型继续→没有其他步→立即 await→拿到结果
   - 拦截逻辑存在（runSubagentItem 有 planIssue 和 uiReadinessIssue 检查），但**不包括"该不该派"的成本判断**

**根本问题**: 系统设计假设"派发 run_subagent 的模型足够聪明，知道什么时候派"，实际上模型经常会"为了显得会用多智能体而派发单点任务"。

---

#### B3：为什么大型现有项目不拆分多子智能体并行？

**现象**: 用户有一个大型现有项目，改某个功能需要：
- ① 前端: 改组件 A, B（React）
- ② 后端: 改 API 端点
- ③ 测试: 写端到端测试

这些**完全可以 run_worker 并行**（scope 不重叠），但系统没有主动建议或引导。

**根因链**：

1. **多角色触发依赖 AI 判定，不是自动拆分**：
   - AI 必须主动说"这个任务需要 frontend/backend/test 三个角色并行"（rawEngineering.orchestrationMode）
   - 系统不会根据"现有项目 + 多个独立模块改动"自动 suggest parallel_roles
   - 默认是 solo 模式

2. **拆分并行门无实施能力**：
   - _splitGateNudgeMessage 只**建议** run_worker 并行（#39592）
   - 消息里说"独立模块可 run_worker 并行实现"**但不强制派发**
   - 派发权完全在模型：需要模型看到建议后主动调多个 run_worker

3. **无"任务自动拆分"系统**：
   - 系统无逻辑扫描"当前计划有哪些独立 scope 可拆"
   - 然后自动派多个 worker，类似:
     ```
     run_worker(description="前端改组件", scope=["src/components/A", "src/components/B"], ...)
     run_worker(description="后端改API", scope=["src/api/auth.ts"], ...)
     run_worker(description="测试", scope=["test/e2e/auth.spec.ts"], ...)
     ```
   - 这需要**主智能体主动裁决拆分**

4. **现有项目的工程判定不足**：
   - 现有项目"改功能"时 AI 判定可能:
     - projectScope=false（因为改动 scope=module，不是整个 project）
     - substantial=false（不涉及架构、数据库、容器等）
     - orchestrationMode=solo（默认）
   - 即使计划中有前后端+测试，也不触发"跨领域 ≥2 域"（都被识别为同一个项目）
   - _splitGate 提示打出来了（"领域分布: 前端×1、后端×1、测试×1"），**但模型不一定理解这就是拆分机会**

**根本问题**: 
- 系统依赖 AI 语义判定"这个任务值不值拆"
- 拆分建议是被动的（计划已落地才提建议）
- 没有"现有大项目"的启发式规则（如"模块数 ≥ 3 且改动 ≥ 3 步 → 试试并行？"）

---

### C. Engineering Profile 字段真相

#### C1：字段构造逻辑

以"现有 React app 改暗黑模式"为例，AI 返回的 rawEngineering:

```json
{
  "projectState": "existing",
  "deliverySurface": "web_app",
  "changeScope": "module",
  "architectureMode": "extend_existing",
  "dataStrategy": "none",
  "researchMode": "none",
  "workspaceAction": "modify"
}
```

计算过程（按 main.js #16905-16978）：

| 字段 | 计算 | 值 | 用途 |
|------|------|---|------|
| `projectSized` | changeScope === "project" ∨ changeScope === "system" | **false** | 用于 projectScope |
| `projectScope` | projectSized ∨ m.projectScope ∨ ... | **false** | 拆分建议、plan 门控 |
| `ui` | deliverySurface ∈ [ui_component, website, web_app] | **true** | UI 专项 |
| `uiProject` | ui 标记 | **true** | UI 项目标记 |
| `fromZeroUiProject` | uiProject && (projectState=greenfield ∨ designMode=michael_design_2_5_greenfield) | **false** | 从零网站标记 |
| `implementation` | workspaceAction === "modify" | **true** | 有写入 |
| `substantial` | applies && (industrialProject ∨ projectSized ∨ ...) | 取决于 industrialProject，通常 **false** | **核心：关系到 plan 建议** |
| `requiresPlan` | substantial | **false** | **是否强制计划** |

**结论**: 现有项目的模块级改动通常导致 `projectScope=false, substantial=false`，只有 `implementation=true`。

---

#### C2：现有大项目"加功能/重构"的字段分布

场景1: "给既有微服务系统加支付模块"

```json
{
  "projectState": "existing",
  "changeScope": "project",          // ← 跨多模块
  "architectureMode": "extend_existing",
  "backend": true,
  "multiService": true
}
```

计算:
- `projectSized = changeScope === "project"` → **true**
- `projectScope = projectSized` → **true**
- `industrialProject = multiService` → **true**
- `substantial = industrialProject` → **true** ✓

这种**能**触发拆分建议。

---

场景2: "改现有 React 页面的样式和表单验证"

```json
{
  "projectState": "existing",
  "changeScope": "module",          // ← 单模块改
  "architectureMode": "extend_existing",
  "ui": true,
  "implementation": true
}
```

计算:
- `projectSized = false`
- `projectScope = false`
- `industrialProject = false`
- `substantial = false` ✗

这种**不**触发拆分建议，即使计划中有前端改动 + 测试，因为：
- `namedDomains < 2`（所有步骤都被识别为"前端"）
- _splitGate 第 39557 行检查 `steps.length < 3`（可能计划只有2步）→ 直接返回空串

---

场景3: "现有 app 加新的支付+通知系统（跨端）"

```json
{
  "projectState": "existing",
  "changeScope": "project",
  "architectureMode": "extend_existing",
  "multiService": true,
  "backendApi": true,
  "frontend": true
}
```

计算:
- `projectSized = true`
- `industrialProject = multiService` → **true**
- `substantial = true` ✓

会触发拆分建议，**但前提是模型自己判定的 changeScope=project**。如果模型判定为 module（说"这只是加一个模块"），就会被降级。

---

#### C3：关键发现

| 字段 | 触发值为真的条件 | 与项目新旧的关系 | 与任务实际规模的关系 |
|------|-----------------|-----------------|-----------------|
| substantial | industrialProject ∨ projectSized ∨ architectureMode ∈ {design_new, refactor_existing} ∨ debugProject ∨ ... | 高度相关（greenfield 更易触发） | **弱相关**（只看 changeScope，不看实际行数/文件数/拆分可行性） |
| projectScope | projectSized ∨ ... | 高度相关（新项目 ≥ project scope） | **弱相关** |
| multiService/largeProject | 显式 AI 判定字段 | 无关（看架构） | **看名字有关，但无自动启发** |

**诊断**:
- 系统无**自动启发式规则**判断"这个现有项目改动实际上足够大，应该拆分"
- 全部依赖 AI 语义判定（rawEngineering 值）
- 若 AI 判定 changeScope=module，即使有 ≥5 个待改文件跨 3 个领域，也会被当成单模块

---

## D. 理想触发模型与分级重构方案

### D1. 重构原则

```
触发 = 基于(任务规模 + 拆分可行性 + 现有工程状态)，与项目新旧解耦
      = NOT based_on(projectState=greenfield)
      = NOT solely_based_on(AI_semantic_intent)
      = MUST include(heuristics: #files, #domains, modularity)
```

### D2. 分级改进方案

#### 改进 1: 空目录门解耦（P0 - 风险高）

**当前**（错位）:
```
行号 35640-35641:
const _emptyBuildIntent = () => !!(run.engineering && 
  (run.engineering.substantial || run.engineering.projectScope
   || run.engineering.fromZeroUiProject || run.engineering.uiProject 
   || run.engineering.fullWebsite || run.engineering.implementation));

行号 35981-35984 (emptyBuildAct):
if (isAgent && run._emptyRootAtStart && !didMutate && _implOps === 0 
    && _emptyBuildIntent()) {
  _pushNudge("emptyBuildAct", "[行动门禁] 环境和方案已经想得够多了...");
}
```

**问题**:
- _emptyBuildIntent 包含 `fromZeroUiProject ∨ uiProject ∨ fullWebsite ∨ implementation`
- 这导致任何有 implementation 标记的任务（无论新旧项目）都触发 build nudge
- 但应该只在"确实需要从零搭建"时才强制建议

**方案**:
```javascript
// 行号 35640（改）
const _emptyBuildIntent = () => !!(run.engineering && 
  (run.engineering.substantial || run.engineering.projectScope
   || run.engineering.fromZeroUiProject));  // 移除 implementation

// 理由：
// - substantial: 大工程、复杂架构、数据库改动 → 需要计划
// - projectScope: 多模块改动 → 需要计划
// - fromZeroUiProject: 从零网站 → 需要计划（原本就该被特殊对待）
// - implementation 单独不够，因为"改一行代码"也是 implementation

// 副作用：现有项目的小改动会少收到"立即行动"的催促
// 风险：降低对"空目录下搭建类任务"的催促频率，但仍有 substantial/projectScope 保护
```

**改进后效果**:
- 新 UI 项目（fromZeroUiProject=true）: 仍触发完整 build gate ✓
- 现有项目加功能（implementation=true, substantial=false）: 不触发 emptyBuildIntent gate，避免干扰 ✓

---

#### 改进 2: 拆分门基于规模的启发式（P1 - 影响中等）

**当前**（消极被动）:
```
行号 39552-39593:
function _splitGateNudgeMessage(run) {
  if (steps.length < 3) return "";  // 少于3步就不提
  if (namedDomains < 2 && !investigable) return "";  // 跨域不足就不提
  // ... 仅根据 plan 步骤和领域数判定
}
```

**问题**:
- 现有大项目"改5个文件跨3个团队"时，若计划写成"1步调研 + 1步实现"，就≤2步，**不触发拆分建议**
- 缺乏"当前工作区已有多个模块"的启发

**方案**:
```javascript
// 新增启发式规则（行号 39546 前插入）
function _countExistingModules(root) {
  // 快速扫描工作区结构，返回大致模块数
  // 如 monorepo 中的 packages/*, 或 src/modules/* 等
  // 实现细节：可缓存或快速估算，不必完全扫描
  // 示例返回值: frontend modules (3) + backend modules (2) + shared (1) = 6
  return estimatedModuleCount;
}

// 改进 _splitGateNudgeMessage（行号 39552）
function _splitGateNudgeMessage(run) {
  if (!run || run._splitGateNudged) return "";
  if ((run._parallelDispatches || 0) > 0) return "";
  if (run._subAgentJobs instanceof Map && run._subAgentJobs.size) return "";
  
  const steps = (Array.isArray(run._planSteps) ? run._planSteps : [])
    .filter((s) => s?.status !== "cancelled");
  
  // 改进：考虑现有模块数和改动文件数
  const existingModules = _countExistingModules(run.root || "");
  const estimatedChangedModules = /* 估算改动涉及的模块数 */ ;
  
  // 启发式：若已有多个模块且改动≥2个，即使计划<3步也可考虑拆分
  const hasMultiModuleProject = existingModules >= 3;
  const touchesMultipleModules = estimatedChangedModules >= 2;
  const shouldConsiderSplit = (steps.length >= 3 && namedDomains >= 2) 
    || (hasMultiModuleProject && touchesMultipleModules && steps.length >= 2);
  
  if (!shouldConsiderSplit) return "";
  
  // ... 继续后续逻辑，但消息中应说明"当前项目有 N 个模块，改动涉及 M 个"
}
```

**改进后效果**:
- 现有 monorepo "加功能涉及 3 个 packages"时，即使计划只有 2-3 步，也会被识别为"多模块改动"→ 建议拆分 ✓
- 避免"大项目被当成小项目"的错觉 ✓

**风险**:
- 需要准确估算模块数，成本中等
- 误估会导致过度建议拆分

---

#### 改进 3: 单子智能体派发前置"不派"判断（P0 - 风险高）

**当前**（无前置拦截，事后傻等检测）:
```
行号 37232:
if (!isWorker && !it.call._wiki && !it.call.wait && _asyncSpawnNames.has(it.tc.name)) {
  // 满足就派，无"该不该派"的评估
  job.promise = _runSubAgent(...);
}

行号 37259:
const message = `[子智能体已后台启动 job#${jobId}] ... 
  ⚠️ 不要立即 await——先推进计划里的其他步骤；
  若当前确实没有其他事可做，说明这个调研本该由你直接读文件完成...`;
  // ^ 消息里已经自我否定了！
```

**问题**:
- 派发完全无前置判断
- 虽然消息提醒"若没其他步骤不该派"，但此时已经派出去了
- 傻等检测是事后反馈，无法阻止不必要的派发

**方案**:
```javascript
// 行号 37230 前插入新的前置判断
const shouldDispatchSubagent = (run, currentStepIndex, totalSteps) => {
  // 规则 1: 若计划中此步后还有 ≥2 步未开始/不被当前步阻塞，允许派
  const hasFollowingSteps = currentStepIndex + 1 < totalSteps - 1;
  
  // 规则 2: 若当前步是调研，后续有实现步，允许派（可并行）
  const isInvestigateFollowedByImplement = 
    _planStepActionKind(currentStep) === "investigate"
    && (currentStepIndex + 1 < totalSteps)
    && _planStepActionKind(totalSteps[currentStepIndex + 1]) === "implement";
  
  // 规则 3: 若是多任务 run_subagent（tasks 数组长度 > 1），允许派
  const isMultiTask = Array.isArray(call.tasks) && call.tasks.length > 1;
  
  // 规则 4: 用户显式 wait=true，允许派（同步等待）
  const isExplicitWait = call.wait === true;
  
  return hasFollowingSteps || isInvestigateFollowedByImplement || isMultiTask || isExplicitWait;
};

// 改进 runSubagentItem（行号 37177）
const runSubagentItem = async (it) => {
  // ... 既有 planIssue、uiReadinessIssue 检查 ...
  
  // 新增：单点任务不派判断
  if (!it.call.wait && _asyncSpawnNames.has(it.tc.name) && !Array.isArray(it.call.tasks)) {
    const shouldDispatch = shouldDispatchSubagent(run, planStepIndex, totalSteps);
    if (!shouldDispatch) {
      // 拦截派发，改为同步调用
      const message = `[避免傻等] 当前没有其他并行机会，改为同步执行...`;
      it.rawResult = { type: "subagent", path: it.call.description || "", content: message };
      return await _runSubAgent(...); // 同步，不后台
    }
  }
  
  // ... 继续异步派发逻辑 ...
};
```

**改进后效果**:
- "调研一个文件后立即 await" → 被拦截改为同步执行，无多进程开销 ✓
- "调研 + 实现并行" → 允许派发后台执行 ✓
- 多任务 run_subagent(tasks=[...]) → 允许派发，能真正并行 ✓

**风险**:
- 需要准确获取计划上下文（currentStepIndex, totalSteps）
- 若计划中途变更（cancel/restore 步骤），判断需要更新
- 可能过度拦截"有创意的并行机会"（如调研与写文档并行）

---

#### 改进 4: 多角色自动启发（P1 - 影响中等）

**当前**（被动等 AI 判定）:
```
行号 16760-16785:
let orchestrationMode = _aiIntentEnum(rawEngineering?.orchestrationMode, ...);
if (orchestrationMode === "solo" && needsMultipleRoles) {
  // 降级保护逻辑只有"如果没角色就降 solo"
  // 无"如果有多角色需求就升级"
}
```

**问题**:
- 系统完全依赖 AI 返回的 orchestrationMode 值
- 现有项目改动即使涉及多个团队，如果 AI 没有显式返回 staged_roles/parallel_roles，也不触发

**方案**:
```javascript
// 新增启发式检测（行号 16760 前）
const inferOrchestrationFromPlan = (run) => {
  if (!run || !Array.isArray(run._planSteps)) return null;
  
  const steps = run._planSteps.filter(s => s?.status !== "cancelled");
  if (steps.length < 3) return null;
  
  const domains = steps.map(s => _planStepDomain(s));
  const uniqueDomains = new Set(domains.filter(Boolean));
  
  // 启发式：计划明确跨 ≥2 领域 + 相邻步跤域不同 → 可能值得 staged_roles
  if (uniqueDomains.size >= 2) {
    const hasMultiDomainImplement = steps
      .filter(s => _planStepActionKind(s) === "implement")
      .slice(0, -1)  // 相邻对
      .some((_, i) => domains[i] && domains[i+1] && domains[i] !== domains[i+1]);
    
    if (hasMultiDomainImplement) {
      return "staged_roles";  // 建议先收敛契约再拆
    }
  }
  
  return null;
};

// 改进判定逻辑（行号 16760）
let orchestrationMode = _aiIntentEnum(rawEngineering?.orchestrationMode, ...);

// 若 AI 返回 solo 但计划显示需多角色，升级建议
if (orchestrationMode === "solo" && run._planSteps) {
  const inferred = inferOrchestrationFromPlan(run);
  if (inferred === "staged_roles") {
    // 不强制改，只在显示里加提醒
    run._inferredOrchestrationMode = inferred;
  }
}
```

**改进后效果**:
- 现有项目 monorepo "改前后端+测试" → 计划中自动识别跨域 → 在显示里额外提醒"可能值得多角色协作" ✓
- 不强制改变 orchestrationMode，保持 solo 默认但补充启发 ✓

**风险**:
- 启发式规则可能误判（如有些步骤只是"阅读"不是"实现"）

---

#### 改进 5: 大项目多 Worker 并行的主动引导（P1 - 影响中等）

**当前**（消极）:
```
行号 39592:
return `[并行机会·事实清单] ... 
  独立模块可 run_worker 并行实现（写入，scope 隔离）；...`;
  // 只是建议，不派发
```

**问题**:
- 建议打出来了，但派发权完全在模型
- 模型需要"看到建议后主动调多个 run_worker"，但通常直接自己做了

**方案**:
```javascript
// 新增：跨域拆分建议时附带示范派发调用（行号 39584 后）
function _suggestWorkerCalls(steps, domains) {
  // 找出相邻 implement 步且跨域的对
  const workerCalls = [];
  for (let i = 0; i + 1 < steps.length; i++) {
    if (_planStepActionKind(steps[i]) === "implement" 
        && _planStepActionKind(steps[i+1]) === "implement"
        && domains[i] && domains[i+1] && domains[i] !== domains[i+1]) {
      
      const role = _domainToRole(domains[i]); // "前端" -> "frontend"
      const prompt = _genWorkerPrompt(steps[i]);
      const scope = _guessScope(steps[i], run.root); // 估算受影响的文件/目录
      
      workerCalls.push({
        description: steps[i].title || `实现 ${domains[i]}`,
        role, prompt, scope
      });
    }
  }
  
  if (workerCalls.length >= 2) {
    // 生成示范代码
    const example = workerCalls
      .map((w, idx) => `run_worker(description="${w.description}", role="${w.role}", prompt="...", scope=[${w.scope.map(s => `"${s}"`).join(", ")}])`)
      .join("\n");
    
    return `\n\n【示范并行派发】\n${example}`;
  }
  
  return "";
}

// 改进 _splitGateNudgeMessage（行号 39592 后）
return `[并行机会·事实清单] ... ${_suggestWorkerCalls(steps, domains)}`;
```

**改进后效果**:
- 拆分建议里包含"可以这样派 worker" 的代码示例 ✓
- 降低模型需要从零理解"如何拆分"的成本 ✓
- 模型更可能复制示范代码而不是自己创意派发

**风险**:
- 生成的 scope 估算可能不准
- 示范代码过长会占用 token

---

### D3. 改进汇总表

| 改进项 | 行号 | 改什么 | 风险等级 | 优先级 | 预期效果 |
|--------|------|--------|---------|-------|--------|
| **改进 1: 空目录门解耦** | 35640-35641 | _emptyBuildIntent 移除 implementation 条件 | 高 | P0 | 现有项目不被误触 build gate，消除"从0才有权力"的假象 |
| **改进 2: 拆分门规模启发** | 39546-39560 | 加 _countExistingModules，扩展 shouldConsiderSplit 逻辑 | 中 | P1 | 大项目改动能被识别为"多模块"，触发拆分建议 |
| **改进 3: 单派前置判断** | 37230-37260 | 加 shouldDispatchSubagent，拦截无并行机会的单任务派发 | 高 | P0 | 避免"傻等"的串行多进程开销 |
| **改进 4: 多角色启发** | 16760-16785 | 加 inferOrchestrationFromPlan，计划跨域自动升级建议 | 中 | P1 | 现有大项目自动被启发"可能需要多角色" |
| **改进 5: Worker 派发示范** | 39592 | 加 _suggestWorkerCalls，拆分建议里给出代码示例 | 低 | P2 | 降低模型派发 worker 的理解成本 |

---

### D4. 风险评估与实施顺序

**风险高的改进（需谨慎）**:
- 改进 1（P0）: 修改空目录门触发条件，可能减少对"搭建类任务"的催促。需验证现有项目是否被误伤。
- 改进 3（P0）: 新增派发拦截逻辑，若规则误判会阻止合法的并行机会。需大量测试用例。

**风险中等的改进（需设计评审）**:
- 改进 2（P1）: 模块数估算的准确性取决于工作区结构启发式，可能 false positive/negative。
- 改进 4（P1）: 计划跨域判断依赖 _planStepDomain 的正则，若正则不全会误判。

**风险低的改进（可快速实施）**:
- 改进 5（P2）: 纯粹增加示范文本，不改派发逻辑，最坏情况只是浪费 token。

**建议实施顺序**:
1. **改进 5**（低风险，快速赢）
2. **改进 2**（需要快速扫描工作区的基础设施）
3. **改进 4**（基于 2 的启发式基础）
4. **改进 1**（P0 但高风险，需全面回归测试）
5. **改进 3**（P0 高风险，最后实施且需覆盖完全）

---

## E. 结论

### 错位主因（三行总结）

1. **空目录门与 substantial 的偶然相关**: 空目录初始化时，新项目被判定为 fromZeroUiProject → substantial → 全流程触发规划建议，而现有项目因 changeScope=module 被判定为 substantial=false，导致系统给人"从0才有权力"的错觉。

2. **拆分建议无启发式辅助**: _splitGateNudgeMessage 仅依赖计划步数与领域数判定，未考虑现有项目的模块数与改动涉及的实际工程复杂性；现有大项目的"加功能"常被判定为 solo orchestrationMode，无自动启发"需要多角色"的机制。

3. **派发权完全归模型，无前置成本评估**: run_subagent 和 run_worker 派发无"该不该派"的机制层判断，傻等检测（#43394）是事后反馈而非事前拦截，导致单点调研也被派发成后台作业（串行多进程，开销大）。

### 单派根因（根本）

**系统假设模型足够聪明知道何时派发，实际缺乏前置判断**：
- 派发前无检验"还有其他并行步骤吗"
- 派发前无评估"单点调研本智能体直接做成本低"
- 傻等检测是事后反馈，无法阻止已启动的作业
- 应在 runSubagentItem 第一次出现异步派发前加 `shouldDispatchSubagent()` 判断

### 大项目不并行根因（根本）

**多角色与拆分能力都依赖被动的 AI 语义判定，无主动的启发**：
- orchestrationMode 完全由 AI 返回，无启发式升级规则
- 现有项目"改多个模块"时 AI 通常判定 changeScope=module（保守），导致 projectScope=false
- _splitGateNudgeMessage 的建议被动打出来，但派发权在模型，系统不主动拆发 worker
- 应加 inferOrchestrationFromPlan 检测计划跨域自动升级建议，加 _suggestWorkerCalls 给出派发示范代码

---

## 附件：证据清单

| 问题 | grep 行号 | 代码片段 | 改进方案 |
|------|---------|--------|--------|
| _emptyBuildIntent 包含 implementation | 35640-35641 | `\|\| run.engineering.implementation` | 移除该条件，仅保留 substantial/projectScope/fromZeroUiProject |
| 派发无前置判断 | 37232 | `if (!isWorker && !it.call._wiki && !it.call.wait && ...)` | 加 shouldDispatchSubagent 前置过滤 |
| 傻等检测事后 | 43394 | `const _idleWait = ... && _ledgerEntries.slice(j.dispatchLedgerLen)` | 无法事前拦截，需前置判断配合 |
| 拆分门忽视模块数 | 39557 | `if (steps.length < 3) return "";` | 加 _countExistingModules 启发式 |
| 多角色无启发 | 16760-16785 | orchestrationMode 仅基于 rawEngineering | 加 inferOrchestrationFromPlan 自动升级建议 |
| 派发建议无示范 | 39592 | 返回文本建议但无代码示例 | 加 _suggestWorkerCalls 生成示范派发调用 |

---

## 文件位置

**本诊断报告**: `/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/ORCHESTRATION_TRIGGER_DIAGNOSIS.md`

**关键源文件**:
- `/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js`
  - 空目录门：#35529, #35640-35641, #35981-35984, #35989-35992, #36290-36293
  - 拆分门：#39532-39593
  - 派发逻辑：#37230-37262, #43367-43395
  - 多角色判定：#16760-16785, #34218-34227
  - 工程判定：#16900-16980

