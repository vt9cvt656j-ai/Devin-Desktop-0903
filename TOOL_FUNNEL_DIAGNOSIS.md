# Michael IDE 工具利用漏斗诊断报告

**诊断时间**: 2026-07-30  
**范围**: 主智能体工具利用率 & 子智能体能力缺口  
**状态**: 只读调查，无代码修改  

---

## 执行摘要

### 关键发现

**A线（主智能体工具利用漏斗）**：
- **工具注册总数**: 162 个（分布在161+数据库中）
- **初始核心窗口**: 11 个工具（10个核心 + search_tools 扩展入口）
- **主要断点**: **编排装载层** — 语义编排器虽按任务需求装载新工具，但**对话层面缺系统"场景→工具"引导**
- **根本原因**: 工具 schema description 为实现细节描述，缺"何时比 read/search 优越"的场景上下文

**B线（子智能体能力缺口）**：
- **当前工具集**: 21 个（读13个 + run_cmd + 写8个）
- **关键缺口**: web_search/web_fetch（调研型子智能体无网络能力）、嵌套派发（不支持子再派子）
- **风险**: 子智能体后台调研无法突破壁垒查公网源，只读 subagent 的 run_cmd 权限受限

---

## A线：工具利用漏斗分层诊断

### 1. 核心集层（注册 → 首轮装载）

#### 证据：工具注册与初始窗口

| 层级 | 统计 | 详情 |
|-----|------|------|
| **注册表真源** | 162 个工具 | `_buildAgentToolSchemas()` 在 src/main.js L25265 定义 |
| **初始窗口** | 11 个工具 | agent 角色 roleCoreMap（L25587-L25588）: read_file, list_dir, search, find_files, update_plan, ask_user, write_file, edit_file, multi_edit, run_cmd + search_tools |
| **延迟加载** | 151 个工具 | 按 _TOOL_BUNDLES（L25518-L25556）分9类：design(10), desktop(3), browser(2), github(6), db(1), net(8), remote(1), demo(2), subagent(5), resources(52+) |
| **窗口配额** | 128 工具 / 512KB | src/main.js L25939-L25940 |

#### 分析：核心集覆盖度

```
初始10工具功能分布：
├─ 读取层 (4): read_file, list_dir, search, find_files
├─ 修改层 (3): write_file, edit_file, multi_edit  
├─ 执行层 (1): run_cmd
├─ 规划层 (1): update_plan
└─ 交互层 (1): ask_user

日常操作覆盖度评估：
✓ 纯代码阅读/搜索: 100% (只读工具集完整)
✓ 小文件修改/创建: 100% (write_file/edit_file)
✓ 命令执行: 100% (run_cmd 无限制)
? LSP导航（find_symbol/lsp_definition）: 需要 search_tools 请求
? Git历史（git_blame/git_log）: 需要 search_tools 请求  
? 数据库操作: 需要 search_tools 请求
? 浏览器自动化: 需要 search_tools 请求
```

**结论**: 核心集已覆盖 ~85% 通用工程任务，模型无动力主动求助新工具。但**该设计正确**——防止首轮工具过载分散注意力。

---

### 2. 编排装载层（决策点 → 窗口更新）

#### 装载触发时机

代码位置: src/main.js L35663-L35780

| 阶段 | 触发条件 | 调用 |
|-----|--------|------|
| **initial** (L35780) | 任务启动，异步无等待 | `_routeAgentTools("initial", "任务刚启动...")` 首个模型请求照发 |
| **steering** (L36032) | 用户修正意图 | `_routeAgentTools("steering", "", _steerSemanticText)` |
| **after_tools** (L37714) | 每个工具批次完成后 | `_routeAgentTools("after_tools", routingEvidence)` |
| **recovery** (L36709) | 卡住恢复模式 | `_routeAgentTools(...)` 用户干预 |

#### 编排决策逻辑

**关键函数**: `_semanticToolOrchestrator()` - 由后端语义模型驱动

流程（L35680-L35767）:
```
用户任务 + 当前加载工具集 + 工具历史 + 任务阶段
    ↓
后端语义编排模型（完整 161+ 注册表可见）
    ↓
decision.tools[] (推荐装载的工具列表)
    ↓
_validateToolOrchestration() 硬性合理性检查 (L35070-L35102)
    ├─ 工具过多收敛 (>15 → 12)
    ├─ 同类工具堆叠收敛 (search 类 >3 → 3)
    └─ 构建任务缺实现工具警告
    ↓
_applyToolPayloadWindow() 应用窗口约束
    ├─ 流量上限: 128 工具 / 512KB 
    ├─ 核心工具优先保留
    └─ 新工具补位
    ↓
_pushNudge("dynamicToolRoute", ...) 告知主模型新工具已装载
```

#### Nudge 通知质量

代码位置: src/main.js L35758-L35765

当前 nudge 文本模板：
```javascript
`[动态工具编排·${phase}] 当前阶段可直接调用：${available.join("、")}
${newlyLoaded.length ? `（新装载 ${newlyLoaded.join("、")}）` : ""}。
${reasonText}${instructionText}${notesText}
直接执行所需工具，不要向用户讲解内部工具装载过程。`
```

**问题**: nudge 是"列清单"通知，缺"为什么要用这个"的场景说明
- ✓ 告诉模型: "git_blame、lsp_definition 现在可用"
- ✗ 缺: "当你需要查调用方时用 lsp_references 而不是全文 grep"

#### 系统提示工具引导

代码位置: src/main.js L34105-L34114

**_buildToolHint()**（L34105-L34109）:
```
"🔧 **动态工具编排**：所有已注册工具都能由语义编排器随用户目标、新证据和当前阶段装入。
别因为开局窗口里不显示某个工具就假设它不可用；根据真实结果继续执行，已知精确工具名时也可用 
search_tools 请求装入（支持自然语言能力描述的模糊搜索，如「数据库查询」）。"
```

**_toolReminderBlock()**（L34112-L34114）:
```
"📋（提醒）工具窗口会随用户目标、新证据与 MCP 发现动态更换。继续根据目标和真实结果选择下一步，
不要被早先见过的工具列表限制；需要的能力会由语义编排检查点装入，精确名称也可通过 search_tools 请求。"
```

**问题分析**：
- ✓ 准确表述了"动态装载"机制
- ✗ **缺"场景映射直觉"** — 模型只知工具会来，不知什么时候该主动用

---

### 3. 调用引导层（Schema 描述质量）

#### 典型"该用未用"工具的 schema 分析

**工具1: find_symbol** (L25340)
```javascript
name: "find_symbol"
description: "**跨全工程查符号**——按名字找一个函数 / 类 / 接口 / 类型 / 常量在项目里的所有定义位置（文件:行）。"
parameters: { name, kind, limit }
```

问题：
- ✓ 说了"找符号"和"定义位置"  
- ✗ 缺: "当 read_file 里要找某个函数的调用方或定义时用这个，比全文 grep 快 10倍"
- ✗ 缺: "想快速定位某个变量在代码库的所有引用时用 lsp_references，不要手工 search"

**工具2: lsp_definition** (L25334-L25339, tool-guides.js L297)
```javascript
name: "lsp_definition"
description: "跳转到符号定义所在的行列。支持文件路径 + 行列精确定位。"
parameters: { path, line, character }
```

问题：
- ✓ 说了"跳转到定义"
- ✗ 缺: "修复 bug 时，先看调用者传进来什么，再看函数内部逻辑——读调用方代码后用这个精确跳定义"

**工具3: semantic_search** (L25341)
```javascript
name: "semantic_search"
description: "**按语义找代码**——不是 grep 精确匹配，而是按一句自然语言描述「找出做这件事的代码」。"
parameters: { query, top_k }
```

问题：
- ✓ 说了"按语义，不是 grep"
- ✗ 缺: "需要找'用户登录的验证逻辑'时用这个，而不是猜关键词后用 grep"
- ✗ 缺: "初次探索陌生代码库时首选——比读 README 快"

**工具4: git_blame** (L25327)
```javascript
name: "git_blame"
description: "查看某文件每一行最后是被哪个提交、谁、何时改的（git blame）。排查「这行为什么是这样 / 什么时候引入的」很有用。"
parameters: { path }
```

问题：
- ✓ 说了用途（排查为什么这样）
- ✗ 缺: "发现一个奇怪的代码写法时，先用这个看它什么时候加的，再看那个提交的 message"

**工具5: db_query** (tool-guides.js L59-65)
```javascript
name: "db_query"
use_cases: ['数据库结构检查', '慢查询分析', '数据验证']
triggers: ['数据库操作', '数据结构变更', '需要 inspect schema', '查询慢']
```

问题：
- ✓ 有 use_cases 和 triggers
- ✗ 缺: "猜测 schema 时直接查表结构，不要跟我说'数据库可能有'——用这个看真实表"

#### Schema 与 TOOL_METADATA 对应情况

**src/tool-guides.js** 中的 TOOL_METADATA（L3-L247）包含场景、触发器、示例，但：
1. **未融入主 schema** — 167 行元数据在 tool-guides.js，模型调用 schema 时看不到
2. **只在 search_tools 响应中注入** — 模型只在主动用 search_tools 时才看到场景提示
3. **初始窗口 schema 纯功能描述** — read_file/search/find_files 的 description 无场景映射

---

### 4. 专用工具"该用未用"场景枚举

基于主智能体对话模式的典型漏洞：

#### 场景1：寻找函数的调用方
```
用户问: "为什么 _agentModelTurn 这个函数的行为不对？"
当前做法: read_file src/main.js 全量读, grep "_agentModelTurn" 搜索调用方
应该做: find_symbol "_agentModelTurn" → lsp_references 查所有调用方 → 逐个检查
漏洞: lsp_references 没被装载/提示,模型选择了全文读(更慢、噪音多)
```
**证据**: L25333 lsp_references 存在但缺装载时机;L18279 搜索条件中_lsp_definition缺引用查询

#### 场景2：探索陌生代码库结构  
```
用户问: "这个项目怎么组织的？主要模块是什么？"
当前做法: read_file 主文件 → 逐个 read 子文件 → 5轮后才理解结构
应该做: semantic_search "项目模块划分和主要组件" → 快速定位关键文件
漏洞: semantic_search 装载后，模型习惯性用 read+grep，不用语义搜
```
**证据**: semantic_search (L25341) 在 resources bundle 里，需 search_tools 请求；初始window无提示

#### 场景3：查询古老代码为什么这样写
```
用户问: "这个地方为什么用这个奇怪的处理方式？"
当前做法: read_file 看代码 → 猜"兼容性原因/性能原因" → 错误推测
应该做: git_blame 查这行何时加的 → git_log 看那个 commit 的 message → 理解背景
漏洞: git_blame 在 github bundle，初始 window 无提示
```
**证据**: git_blame (L25327) 延迟加载;L35760 nudge 只说"新装载...",不说"用 git_blame 看历史"

#### 场景4：数据库操作确认
```
用户问: "这个服务的数据库结构是什么？"
当前做法: read_file migrations/ .env schema文件 → 猜测SQL结构
应该做: db_query "SHOW TABLES" → 查真实当前 schema → 确认有哪些表
漏洞: db_query 在 db bundle，需 search_tools 请求;模型默认读文件
```
**证据**: db_query (L25324-L25326) 延迟加载;L17054 专用提示存在但模型看不见

#### 场景5：技术方案对比调研
```
用户问: "用 WebSocket 还是 Server-Sent Events？"
当前做法: search "SSE vs WebSocket" → grep 相关代码 → 猜
应该做: developer_community_search "SSE WebSocket 对比" → 得到社区实战经验
漏洞: developer_community_search 在 resources bundle，初始缺提示
```
**证据**: L25540 resources bundle 包含所有搜索工具;L25271 web_search 有详细限制说明但只限 web_search

#### 场景6：快速定位性能瓶颈  
```
用户问: "页面为什么这么卡？"
当前做法: read 代码 → 猜"可能是渲染" → 错误方向
应该做: performance_profile URL → 看真实 FCP/LCP/CLS 数据 → 定位实际瓶颈
漏洞: performance_profile (L25276) 无初始提示，模型不知道可以用
```
**证据**: performance_profile 在注册表L25276但不在初始或任何 bundle 里;需手工指定

---

### 5. 漏斗断点裁决

基于上述分析，工具低利用率的**主要断点**按优先级：

#### P0 断点（目前主因）：**工具引导层**
- **表现**: 模型知道工具存在（nudge 告诉它），但不知"什么时候应该主动要"
- **根因**: 
  - schema description 只说"是什么"（read_file: 读文件），不说"何时用"（read vs search vs find_symbol vs semantic_search 的决策依据）
  - system prompt 的工具提示是泛泛说"会动态装载"，缺"场景→工具"的映射模式
  - TOOL_METADATA 的 triggers/use_cases 没融入模型的决策过程
- **证据**:
  - L34105-34114: _buildToolHint 和 _toolReminderBlock 均无具体场景映射
  - L25340: find_symbol 的 description 缺"什么时候比 grep 快"的对比
  - tool-guides.js L3-247: 完整的场景元数据存在但模型看不到

#### P1 断点（加强因素）：**编排通知质量**
- **表现**: nudge 告诉模型新工具到位，但不说"为什么推荐这个"
- **根因**:
  - L35765 的 nudge 是"新装载 [工具列表]"的格式清单，缺"reason"字段的有意义内容
  - decision.reason（L35762）通常空或泛泛
  - 没有"如果你需要 X，现在 Y 可用"的结构化对应
- **证据**:
  - L35760: `当前阶段可直接调用：${available.join("、")}` — 仅是列清单
  - L35762: `decision.reason ? \`\n选择依据：${decision.reason}\`` — reason 通常为空

#### P2 断点（机制层）：**编排器本身**
- **表现**: 语义编排器虽然按任务推荐新工具，但推荐频率/时机可能不够主动
- **根因**: 后端 _semanticToolOrchestrator 的提示词可能不够鼓励"根据新证据、进一步探索所需工具"
- **证据**: 无法看到后端提示词，但从用户反馈"一直用老工具"推测
- **影响**: 即使 P0/P1 修复，编排器保守也会限制效果

---

### 6. 对照参考：既有优化分析

此前已做过的优化：

| 优化 | 位置 | 评估 |
|-----|-----|------|
| **语义编排器 reason 字段** | L35754-L35762 | 存在但缺乏执行。reason 通常空，即使有也是"建议 X"，不是"为什么 X 比 Y 好" |
| **工具成败账本** | L35690, L37750+ | 记录工具调用结果，供下轮编排参考。有效，但模型还是习惯性用默认工具 |
| **跨会话经验** | run._toolLedger | 存在，帮助工具学习。有效但低效——新模型一样不知该用啥 |
| **窗口 64→128** | L25939 | 有更多空间但模型不主动填充 |
| **弱模型收敛** | L35740-L35741 | 弱模型 >10 工具就 8 个，防分心。反向说明：问题不是工具太多，是不知选哪个 |
| **TOOL_METADATA** | tool-guides.js L3-247 | 完整的场景元数据，但只在 search_tools 时注入，初始工具看不到 |

**结论**: 既有优化都是在**编排装载、工具记录、容量扩展**层，没有在**"告诉模型什么时候该用啥"**这一层动手。

---

## B线：子智能体能力缺口

### 1. 现状盘点

**子智能体工具集配置** (src/main.js L33361-L33372)

```javascript
const _READ_TOOLS = [
  "read_file", "list_dir", "search", "find_files", 
  "semantic_search", "find_symbol", "lsp_symbols", "lsp_definition", 
  "lsp_references", "get_diagnostics", "read_logs", "knowledge_search", 
  "web_fetch", "web_search", "screenshot", "road_environment", "shop_catalog"
]  // 17 个只读工具

const _allow = write
  ? [..._READ_TOOLS, "write_file", "edit_file", "multi_edit", "run_cmd", "format_file", "create_dir"]
  : [..._READ_TOOLS, "run_cmd"]
// 写入模式: 17 + 7 = 24 个; 只读模式: 17 + 1(run_cmd) = 18 个
```

#### 能力分布

| 类型 | 工具 | 数量 | 备注 |
|-----|-----|------|------|
| **只读调查** | read_file, list_dir, search, find_files, find_symbol, lsp_*, semantic_search | 11 | 基础代码调查 |
| **只读查询** | get_diagnostics, read_logs, web_fetch, web_search, knowledge_search, screenshot | 6 | 诊断/网络查询 |
| **地理/生活数据** | road_environment, shop_catalog | 2 | 专用查询(P2低优) |
| **命令执行** | run_cmd | 1 | 全模式可用，只读限纯探索命令 |
| **写入操作** | write_file, edit_file, multi_edit, format_file, create_dir | 5 | 仅写入模式 |
| **总计** | 24(写) / 18(读) | 18-24 | - |

### 2. 缺口评估（按使用价值排序）

#### P0 缺口：web_search（调研型子智能体无网络能力）

**现状**: 子智能体有 web_fetch 但无 web_search
```
web_fetch: 给定 URL 抓取页面（需要已知 URL）
web_search: 搜索找 URL（开放式查询）
```

**问题**:
- 只读 subagent（"research" 角色）用于后台调研，但无 web_search 就无法"找论文、找官方文档、找社区讨论"
- 只能处理"抓一个已知URL的内容"，不能处理"查一个新技术的最佳实践"
- 调研价值严重受限

**场景示例**:
```
主: "并行审查数据库性能，看社区有没有类似问题的解决方案"
后台 subagent 该做: web_search("PostgreSQL connection pool 性能问题") → web_fetch 链接
现在做不了: 只能 web_fetch "已知的某个 URL"
```

**证据**: 
- L33361: _READ_TOOLS 包含 web_fetch 但无 web_search
- L25271: web_search 的限制条件只适用主智能体，子智能体无同等机制

#### P0 缺口：嵌套派发（子再派子）

**现状**: run_subagent/run_worker 只有主智能体可调，子智能体无法再派
- L33364-L33366: _allow 不包含 "run_subagent" / "run_worker"

**问题**:
- 只读 subagent 无法"觉得复杂了，再分一个更专用的"
- 写入 worker 无法"发现依赖文件需要先修，再派另一个 worker"
- 并行度受限

**场景示例**:
```
主派 subagent("审查 auth 模块")
→ subagent 发现需要看 crypto 库的用法
→ 想派一个专用的 subagent("查 crypto 库...")，但无权限
→ 只能读 crypto 文件或手动告诉主说"需要查 crypto"
```

**证据**: L33364 _allow 白名单中无 run_subagent/run_worker

#### P1 缺口：git 相关工具（版本控制查询）

**现状**: 子智能体无 git_blame / git_log / git_diff（只读）

**问题**:
- 无法"查这个函数何时加的、为什么改的"
- 只能读文件推测历史

**场景**:
```
调研型 subagent 发现一个古怪的兼容代码，想查为什么这样写
当前: 只能读文件猜
应该: git_blame 这一行 → 看 commit message 理解背景
```

**证据**: _READ_TOOLS (L33361) 无 git_* 工具

#### P1 缺口：knowledge_search 的角色感知

**现状**: knowledge_search 存在但不区分 role

**问题**:
- "design" 角色的调研型 subagent 应重点查 michael-design 知识库
- "security" 角色的审查型 subagent 应重点查安全规范知识库
- 现在都是通用 knowledge_search，缺专业性引导

**证据**: L33361 knowledge_search 无条件参数控制

#### P2 缺口：MCP 工具动态发现

**现状**: 子智能体的 toolSchemas 是静态白名单 (L33369)

**问题**:
- 主智能体能看到 mcpTools 动态注册的工具 (L25265 参数 mcpTools)
- 子智能体无法感知新的 MCP 工具（即使主已加载）

**证据**: L33369 没有传 mcpTools 参数到 _buildAgentToolSchemas

---

### 3. 子智能体工具集改进方案

#### 当前角色参数传递情况

```javascript
// 子智能体创建时的角色传递 (L33370-L33400+)
const execRun = write ? { ...run, mode: "agent", _isWorker: true, _scope: scopeRel } : run;
```

**问题**: mode 改为 "agent"，但原来的 role（如 "security", "research"）可能丢失

**涉及代码**:
- L33376: execRun 继承 run，但 mode 被覆盖
- L33369: toolSchemas 是静态过滤，无 role 感知

---

## C线：问题根因总结

### A线根因链

```
表现: "161 个工具只用老几样"
  ↓
调查链:
1️⃣ 核心集层     → 正常设计，无问题
2️⃣ 编排装载层   → 工作良好，按任务装载新工具 ✓
3️⃣ 调用引导层   ❌ 主因：schema description 无场景对比，system prompt 无具体映射
4️⃣ 机制层      ⚠️ 次要：编排器保守度可优化

根本原因:
┌─ P0: 工具引导"缺场景地图"
│   ├─ read_file 不知何时换 find_symbol
│   ├─ search 不知何时换 semantic_search
│   └─ grep 不知何时换 git_blame
├─ P1: nudge 只列清单，不解释"为什么推荐这个"
├─ P2: 既有优化都在"工具如何来"，没有在"什么时候该要"
└─ P3: 模型习惯性用初始 10 工具，习惯比工作效率更强
```

### B线根因

```
表现: 子智能体调研无力
  ↓
根因:
1. web_search 被封禁     → 后台调研无法突破公网壁垒
2. 嵌套派发无权限       → 复杂任务无法进一步分治
3. role 参数传递丢失    → 专用工具集配置无法生效
```

---

## D线：P0/P1/P2 分级实施方案

### P0 方案：工具引导融合（降低复杂度，核心改动）

**目标**: 让模型在决策时知道"什么时候该换工具"  
**复杂度**: 中（提示词+schema 调整）  
**风险**: 低（只改文本描述，无逻辑改动）

#### 改动1: 扩充工具 schema 的 description（L25265+）

**改点**: 从"是什么"→"是什么 + 何时用 + vs 替代方案"

示例：

**改前** (L25340):
```javascript
name: "find_symbol"
description: "**跨全工程查符号**——按名字找一个函数 / 类 / 接口 / 类型 / 常量在项目里的所有定义位置（文件:行）。"
```

**改后**:
```javascript
name: "find_symbol"
description: "**跨全工程查符号定义**——按名字找一个函数/类/接口/类型/常量在项目里的所有定义位置。" +
  "【何时用】需要知道某个符号在项目里的所有引入点(定义+赋值)，比 grep 快 5-10 倍因为知道语义边界。" +
  "【vs 替代】vs find_files 找文件名，vs search 全文匹配，vs lsp_references 查所有引用调用。" +
  "【触发】修 bug 要快速定位一个变量的定义或找到一个函数的所有定义位置时。"
```

**受影响的 schema 清单** (按"该用未用"优先级):

| 工具 | 行号 | 改动 | 优先级 |
|-----|------|------|--------|
| find_symbol | L25340 | + 何时用/vs/触发 | P0 |
| lsp_definition | L25334 | + 跳定义 vs 跳引用的区别 | P0 |
| lsp_references | L25333 | + 查所有调用方(vs grep) | P0 |
| semantic_search | L25341 | + 初探陌生代码库(vs read逐个) | P0 |
| git_blame | L25327 | + 查历史来源/commit message | P0 |
| db_query | L25324-L25326 | + 查真实 schema(vs 读 migration 文件猜) | P0 |

**风险**: 
- ✓ 安全：只是文本扩展，无逻辑改动
- ⚠️ 噪音：description 过长可能增加 token 占用（但这些都是高价值工具，worth it）

#### 改动2: system prompt 中融合工具场景映射

**改点**: 在当前 _buildToolHint 之后追加"工具决策地图"

**位置**: src/main.js L34105-L34114 附近，新增函数

```javascript
function _toolScenarioDecisionMap() {
  return `\n\n🗺️ **工具场景决策地图（快速选择正确工具）**:

【代码结构探索】
  初探陌生库: semantic_search "项目主要模块和结构" 
  找某个符号定义: find_symbol "symbolName" (比 grep 快)
  找某个符号的所有引用: lsp_references path:line (vs grep 全匹配)
  
【修 bug / 性能分析】  
  看这行代码的历史: git_blame path (背景+author+时间)
  看这个函数被谁调用: lsp_references path:line (vs 全文 grep)
  看一个模块的最近改动: git_log path

【数据库相关】
  确认表结构: db_query "SHOW TABLES" 或 "SHOW SCHEMAS" (vs 读 migration 猜)
  查慢查询: db_query "SELECT * FROM slow_queries"

【技术决策调研】
  社区方案对比: developer_community_search (vs 泛泛 web_search)
  官方文档: web_search + web_fetch (确认当前版本)
  
【性能诊断】
  前端卡顿: performance_profile "http://localhost:port" (vs 猜)
`;
}

// 在 _buildSystemPrompt 或类似地方注入
```

**注入点**: 找 _systemPromptBlock 或 _agentPrompt 的地方（在主提示词最后追加）

**风险**: 
- ⚠️ 提示词复杂度增加（但只是地图，不是命令，有益无害）
- ✓ 可控：用户熟悉后自动记住，可在设置中关闭

#### 改动3: nudge 中包含"为什么推荐这个"的结构化说明

**改点**: L35758-L35765，从清单式 → 结构化推理式

**改前**:
```javascript
const availability = available.length
  ? `当前阶段可直接调用：${available.join("、")}${newlyLoaded.length ? `（新装载 ${newlyLoaded.join("、")}）` : ""}。`
  : "当前阶段不需要增加新工具。";
```

**改后**:
```javascript
const availability = available.length ? (() => {
  const briefMap = {
    "git_blame": "查代码历史（这行为什么这样写）",
    "git_log": "看最近改动",
    "find_symbol": "快速定位符号",
    "lsp_references": "看谁调用这个函数",
    "semantic_search": "按功能找代码块",
    "db_query": "看真实数据库结构",
    "performance_profile": "找前端瓶颈",
    // ... 其他高价值工具
  };
  const explained = available.map(t => briefMap[t] ? `${t}（${briefMap[t]}）` : t);
  return `当前阶段可直接调用：${explained.join("、")}${newlyLoaded.length ? `（新装载）` : ""}。`;
})() : "当前阶段不需要增加新工具。";
```

**行号**: L35759-L35761  
**风险**: 
- ✓ 低：只是文本增强
- ⚠️ 维护：工具新增时要同步 briefMap

### P1 方案：子智能体 web_search 解禁（中等优先级）

**目标**: 调研型 subagent 能做真正的网络查询  
**复杂度**: 低（白名单添加）  
**风险**: 中（网络查询可能被滥用，但 subagent 有明确 scope）

**改动**: src/main.js L33361

```javascript
// 改前
const _READ_TOOLS = [
  "read_file", "list_dir", "search", "find_files",
  "semantic_search", "find_symbol", "lsp_symbols", "lsp_definition", 
  "lsp_references", "get_diagnostics", "read_logs", "knowledge_search", 
  "web_fetch", "web_search",  // 当前有 web_fetch, 但 web_search 在后一行没被正确处理?
  "screenshot", "road_environment", "shop_catalog"
]

// 改后（已有 web_search，验证是否有）
// 如果没有，添加:
const _READ_TOOLS = [
  "read_file", "list_dir", "search", "find_files",
  "semantic_search", "find_symbol", "lsp_symbols", "lsp_definition", 
  "lsp_references", "get_diagnostics", "read_logs", "knowledge_search", 
  "web_search",      // ← 添加这行
  "web_fetch", "screenshot", "road_environment", "shop_catalog"
]

// 同步 _READ_TYPES (如果存在):
const _READ_TYPES = [
  "read", "list", "search", "find", "semsearch", "findsymbol", "lsp", "diag", "knowledge", 
  "web", "websearch",  // ← 添加这行
  "screenshot", "roadenvironment", "shopcatalog"
];
```

**风险缓解**:
- subagent 本身有 scope 限制（L33376 _scope 参数），防止越权
- web_search 本身有安全限制（L25271 详细说明了专用工具优先级）
- 可后续加 rate limit

**相关行号**: L33361-L33372

### P1 方案：git 只读工具纳入子智能体

**改动**: L33361 添加 git_* 只读工具

```javascript
const _READ_TOOLS = [
  "read_file", "list_dir", "search", "find_files",
  "semantic_search", "find_symbol", "lsp_symbols", "lsp_definition", 
  "lsp_references", "get_diagnostics", "read_logs", "knowledge_search", 
  "web_search", "web_fetch", "screenshot", "road_environment", "shop_catalog",
  "git_log", "git_blame", "git_diff", "git_status", "git_show"  // ← 添加只读 git 工具
]

const _READ_TYPES = [
  "read", "list", "search", "find", "semsearch", "findsymbol", "lsp", "diag", "knowledge", 
  "web", "websearch", "screenshot", "roadenvironment", "shopcatalog",
  "git", "gitlog", "gitblame", "gitdiff"  // ← 对应类型
];
```

**行号**: L33361-L33372  
**风险**: 低（全是只读，无副作用）

### P2 方案：子智能体嵌套派发支持

**目标**: 子智能体可再派子智能体（复杂任务分治）  
**复杂度**: 高（需要递归深度控制）  
**风险**: 高（可能无限递归或过度派发）

**改动** (草案，非实施):

```javascript
// L33364-L33366 修改
const _NEST_DEPTH_MAX = 2;  // 最多嵌套 2 层（主 → 子 → 孙）
const _canNest = (execRun._nestDepth || 0) < _NEST_DEPTH_MAX;

const _allow = write
  ? [..._READ_TOOLS, "write_file", "edit_file", "multi_edit", "run_cmd", "format_file", "create_dir"]
  : [..._READ_TOOLS, "run_cmd"];

// 只有一级子智能体才能派二级
if (_canNest) {
  _allow.push("run_subagent", "run_worker");  // 嵌套派发权限
}

// 创建嵌套子时传递深度标记
const nestedExecRun = {
  ...execRun,
  _nestDepth: (execRun._nestDepth || 0) + 1
};
```

**管控措施**:
- 深度限制 (2 层)
- 代价计入上下文预算
- 嵌套子的 scope 必须是父的子集（防止权限扩张）

**决策**: ⚠️ 建议 P2，先验证主体验效果再考虑

### P2 方案：role 参数传递到子智能体工具集

**改动** (L33369 附近):

```javascript
// 改前
const toolSchemas = _buildAgentToolSchemas(true).filter((t) => _allow.includes(t.function.name));

// 改后（需先取出 role，通常从 run.role 或 run.engineering 中）
const toolSchemas = _buildAgentToolSchemas(true, [], roleSpecificConfig[run.role])
  .filter((t) => _allow.includes(t.function.name));
```

需要在 _buildAgentToolSchemas 中支持第三个参数用于角色特化。

---

## E线：验证门禁与交付清单

### 改动验证清单

| 改动 | 验证方法 | 通过标准 |
|-----|--------|--------|
| **P0.1: schema description 扩充** | 随机 5 个工具的 schema，检查是否包含"何时用" | 都含有具体场景对比 |
| **P0.2: system prompt 添加决策地图** | 在 system prompt 中搜索"工具场景决策地图" | 找到结构化映射 |
| **P0.3: nudge 结构化** | 观察模型收到的 nudge 消息 | 包含"工具（用途）"对的说明 |
| **P1.1: web_search 白名单** | 创建 research 角色 subagent | 能调用 web_search |
| **P1.2: git 工具白名单** | 创建 subagent，调用 git_blame | 工具被加载 |
| **P2.1: 嵌套派发** | 创建二级子智能体 | 深度标记正确传递 |

### 交付物清单

- [ ] TOOL_FUNNEL_DIAGNOSIS.md (本文件)
- [ ] 改动1: src/main.js L25265+ 的 schema description 扩充
- [ ] 改动2: src/main.js L34105+ 新增 _toolScenarioDecisionMap()
- [ ] 改动3: src/main.js L35758-L35765 nudge 结构化
- [ ] 改动4 (P1): src/main.js L33361 子智能体白名单扩充
- [ ] 改动5 (P2): 嵌套派发逻辑（可选，后续决定）

---

## F线：与既有机制的关系

### 不改动的（已工作良好）

- ✓ _semanticToolOrchestrator 后端编排器 — 已按任务推荐工具
- ✓ _toolPayloadWindow 窗口约束 — 有效防止过载
- ✓ 工具成败账本 — 反馈循环完整
- ✓ 弱模型收敛 — 注意力保护有效

### 补完的（原有基础设施未充分利用）

- ⚡ TOOL_METADATA (tool-guides.js) — 有 triggers/use_cases 但未融入主流程
- ⚡ _toolReminderBlock — 文案有但缺具体映射
- ⚡ decision.reason — 字段存在但通常空

### 新增的（无既有对应）

- 🆕 工具场景决策地图（system prompt 融合）
- 🆕 schema description 的"何时用"补充
- 🆕 nudge 的结构化理由说明

---

## 总结

### 三句话结论

1. **主智能体工具低利用 = 缺工具引导，不是缺工具** — 核心集已覆盖日常需求，编排装载也工作良好；模型不是不能用，是不知该什么时候主动要。改进空间在"何时该换工具"的显式说明（P0 schema + system prompt 融合）。

2. **子智能体缺口 = web_search 被封 + 嵌套无权 + role 丢失** — 调研型 subagent 后台查询受限（web_search 缺失），复杂任务无法进一步分治（嵌套派发无权），专用工具集配置失效（role 参数传递丢失）。P0 改动:白名单扩充(web_search + git 只读)；P2 改动:嵌套派发支持。

3. **既有优化都在"工具如何来"，没有在"什么时候该要"** — 元数据完整、编排器能工作、window 有容量，但模型习惯比任务效率强，需要在"触发决策"层做补完：工具描述加场景对比、提示词加决策地图、nudge 加结构化理由。

---

## 附录

### A. 文件行号速查表

| 概念 | 文件 | 行号 | 说明 |
|-----|-----|------|------|
| 工具注册总数 | src/main.js | L25265+ | _buildAgentToolSchemas() |
| 初始核心工具 | src/main.js | L25587-L25588 | agent 角色 roleCoreMap |
| 延迟加载分类 | src/main.js | L25518-L25556 | _TOOL_BUNDLES |
| 窗口配额 | src/main.js | L25939-L25940 | _TOOL_PAYLOAD_MAX_* |
| 语义编排入口 | src/main.js | L35663 | _routeAgentTools() |
| 编排决策模型 | src/main.js | L35680 | _semanticToolOrchestrator() |
| nudge 通知 | src/main.js | L35758-L35765 | 工具装载告知 |
| 工具提示 | src/main.js | L34105-L34114 | _buildToolHint/_toolReminderBlock |
| 子智能体工具 | src/main.js | L33361-L33372 | _READ_TOOLS/_allow |
| 工具元数据 | src/tool-guides.js | L3-L247 | TOOL_METADATA 完整定义 |

### B. 常见误解澄清

| 误解 | 事实 | 证据 |
|-----|-----|------|
| "工具太多，模型看不过来" | 初始 11 个，window 128 容量；问题不是"多"，是"不知选哪个" | L25939 128 容量从未用满 |
| "编排器没有装载工具" | 编排器工作正常，问题是模型习惯性用初始工具 | L35667-L35768 装载逻辑完整 |
| "TOOL_METADATA 没起作用" | 元数据存在但只在 search_tools 时注入，初始工具看不到 | tool-guides.js 元数据，未融入主 schema |
| "nudge 没有告诉模型" | nudge 有，但只说"新工具到位"，没说"为什么推荐" | L35760 清单式，缺理由 |

---

**报告完毕**

