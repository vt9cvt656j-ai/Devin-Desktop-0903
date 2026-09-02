# Michael-Design 触发面过窄诊断报告

## 调查对象
- 客户端：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide` (src/main.js)
- 服务端：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/server` (src/prompts.rs)
- 权威知识源：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/server/prompts/design_system.txt`

## 执行摘要
**根本问题**：michael-design 知识库的注入触发链路分为客户端意图判定→服务端语义路由两层，但两层都过于严格地依赖关键词匹配和特定标记，导致"写软件 UI"等合法场景被漏掉。特别是：
1. 客户端意图判定（src/main.js L16661-16945）判断 `m.ui/m.uiProject` 依赖 `deliverySurface=website/web_app` 或对话含前端代码迹象，软件应用 UI 场景（deliverySurface=desktop/mixed）判不出来
2. 服务端网关（prompts.rs L3661-3665）有事实门控 `looks_like_ui_task()` 依赖关键词集合（包含"网站""网页""dashboard"等），但"做个软件界面""应用 UI"等措辞未完全覆盖
3. 两层都未建立"任何 UI 设计/改界面"就触发的统一通道

---

## 一、触发链路图解

```
客户端（ide/src/main.js）
    ↓
意图判定（AI模型，16808行 _aiIntentProfile）
    ↓ 生成 verdict.engineering.designMode
    ↓ 派生维度 ui/uiProject/fullWebsite/designKnowledgeRequired
    ↓ 发 x-ide-semantic-profile 头（537行）
    ↓
网关服务端（server/prompts.rs L3202）
    ↓ 接收 x-ide-semantic-profile / x-ide-ui 头部
    ↓ 语义路由 semantic("design")（3322-3325行）
    ↓ 
决策树
├─ ui_env="0"：禁用设计系统（MICHAEL_UI_GUIDE=0）
├─ ui_env="always"：强制启用
└─ 默认：semantic("design") && (mode=="agent" || mode=="plan")
    ↓
若触发，根据维度分支装配设计模块
├─ design_knowledge_block()(L1826）注入蓝本
├─ design_implementation(L3334)
├─ design_scaffold(L3337)
└─ design_motion(L3349)
    ↓
system prompt + design_system.txt 注入模型
```

---

## 二、客户端判定层分析

### 当前 UI 意图判定位置
**文件**：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js`  
**行号**：L16758-16945

### 关键判定逻辑
```javascript
// L16909
let designMode = engineering?.designMode || "none";

// L16932-16933：uiProject 由服务端 deliverySurface 推导
m.uiProject = !!(m.uiProject || uiSurface);
m.fullWebsite = !!(m.fullWebsite || deliverySurface === "website" || deliverySurface === "web_app");

// L16941-16945：设计知识库触发条件
if (m.ui && workspaceAction !== "none" && designMode === "none") {
    designMode = m.fromZeroUiProject ? "michael_design_2_5_greenfield" : "michael_design_2_5_existing";
}
m.designMode = designMode;
m.designKnowledgeRequired = designMode !== "none";
```

### 问题根源 1：deliverySurface 的狭隘定义
**工程字段定义**（L16833 prompt）：
```
deliverySurface=answer/code/ui_component/website/web_app/backend/data/cli/desktop/automation/mixed
designMode=none/michael_design_2_5_existing/michael_design_2_5_greenfield
```

**当前触发条件**（L16941-16945）：
- michael-design **仅在** `m.ui && workspaceAction !== "none"` 时才会自动升级为 greenfield/existing
- 但 `m.ui` 由以下任一条件触发（L17013）：
  ```javascript
  add("design", p.designKnowledgeRequired || p.ui || p.uiProject);
  ```
- 而 `p.ui` 的来源是 semantic("design") 返回值

**症状**：
- ✅ `deliverySurface=website` → michael-design 注入
- ✅ `deliverySurface=web_app` → michael-design 注入  
- ❌ `deliverySurface=desktop` 应用 UI → **不注入**（被判为后端/系统软件）
- ❌ `deliverySurface=mixed` 既有后端又有 UI → **不注入**（不确定）
- ❌ `deliverySurface=code` + "写个 React 组件" → **不注入**（语义不明确）

### 问题根源 2：服务端语义门控过严

**文件**：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/server/src/prompts.rs`  
**行号**：L3661-3665（重点）

```rust
let design = (mode == "agent" || mode == "plan")
    && (routed.get("x-ide-ui").is_some()
        || ((current_or_continuation_user_text_any(body, looks_like_ui_task)
            || conversation_shows_frontend_work(body))
            && (!automation || explicit_design)));
```

**三个事实门：**
1. `routed.get("x-ide-ui").is_some()` → 客户端显式打了 UI 标记（目前仅在测试中用）
2. `looks_like_ui_task()` → L2411-2573，关键词匹配
3. `conversation_shows_frontend_work()` → L979-1013，扫历史消息找 `.tsx/.jsx/.tailwind/shadcn` 等代码标记

**looks_like_ui_task 关键词集合**（L2414-2507）：
```rust
ASCII_KW: ["website", "web page", "webpage", "web app", "frontend", "dashboard", 
           "navbar", "hero section", "ui design", "tailwind", "shadcn", "react", ...]
CJK_KW:  ["网页", "网站", "前端", "页面", "布局", "按钮", "落地页", "仪表盘", 
           "建站", "做界面", "做网页", "界面设计", ...]
```

**症状**：
- ✅ 用户说"写一个网站" → 命中 looks_like_ui_task
- ✅ 前两轮代码中有 `.tsx` + 当前说"改界面" → conversation_shows_frontend_work 捕获
- ❌ 用户说"做个软件应用的设置页面" → `"软件"/"应用"` 未在关键词表，可能漏掉
- ❌ 用户说"给这个 Electron 应用加个新 UI" → `"Electron"` 未列，关键词不匹配
- ❌ 用户说"设计一个漂亮的配置界面" → `"配置"` 虽在表里，但若消息太简短可能不足以触发

### 问题根源 3：客户端无反向馈送

客户端的 `_aiIntentProfile()` 调用 AI 模型判定 `designMode`，但模型输出 `designMode=none` 时，客户端未在后续提醒用户或允许手工回溯。特别是当模型把 UI 意图判成了 `deliverySurface=code` 而非 `deliverySurface=ui_component/website` 时，michael-design 完全被跳过。

---

## 三、设计系统的权威位置

**唯一真源**：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/server/prompts/design_system.txt` (66 行)

**关键声明**（L5-9）：
```
- **建站/界面意图无条件适用本体系**，不管用户措辞是否含"网站/网页"字样；
  只要交付物是页面/界面就按这套来。
```

**所以设计系统的设计已经是"任何 UI 都该用"，但触发门控没有对齐这个意图。**

---

## 四、根因剖析（分层）

### 层级 1：客户端 AI 判定（src/main.js L16820+）

**判定逻辑**（prompt 要求）：
```
设计模式定义（L16838）：
4. 任何实际 UI/网站的新建、修改、评审都使用 michael-design 2.5。
   现有网站/现有组件用 michael_design_2_5_existing：保留品牌、现有架构和可用组件。
   真正绿地 UI 才用 michael_design_2_5_greenfield。
```

**问题**：
- AI 判定受 `deliverySurface` 枚举制约：如果用户说"做个应用"，AI 再聪明也无法输出一个不存在的 `deliverySurface` 值
- 这要求工程字段的词汇表本身就包含"UI 应该被识别"的所有情景
- 当前词汇表缺"应用 UI"这条通道（只有 website/web_app，没有 desktop_ui/app_interface 等）

### 层级 2：服务端网关边界（prompts.rs L3661-3665）

**事实门控**：语义路由依赖落地检测（looks_like_ui_task、conversation_shows_frontend_work）
- 用户没说"网页/网站"但说了"软件界面" → 可能漏掉
- 用户说"改 UI"但历史消息中没有代码 → conversation_shows_frontend_work 无法证实

**关键字漏洞**：
- `looks_like_ui_task` 未覆盖所有 UI 表述（软件应用、客户端、GUI、界面等）
- `frontend_work_signal` (L958-976) 只检查代码标记，不检查"改界面""调样式"这类对话信号

### 层级 3：prompt cache 稳定性约束

目前 michael-design 注入在 system prompt（缓存最热路径），若触发条件变化频繁会破坏前缀稳定性。

---

## 五、过窄的根本原因陈述

用户诉求："**凡涉及 UI 设计都该用 michael-design，除非确实不适用**"

当前实现："**只有被客户端+服务端同时识别为网站/网页类 UI 的意图才注入 michael-design**"

**差异**：
1. **delivery surface 遗漏** → 应用 UI (desktop/cli with UI/mixed) 被归到非网站类，michael-design 完全旁落
2. **关键词黑名单** → 只要用户措辞不在关键词表里，就被当作不是 UI 任务
3. **往一处集中** → 所有判定都集中在"看起来像网站吗？"，没有"这个交付物有 UI 吗？"的独立通道

---

## 六、放宽方案（分层设计）

### 方案 A：客户端低风险放宽（推荐优先）

**改动位置**：`ide/src/main.js` L16941-16945

**原理**：将 michael-design 的触发从 `deliverySurface` 的狭义理解扩展到任何包含 UI 的场景

**具体改动**：
```javascript
// 当前（仅website/web_app）
if (m.ui && workspaceAction !== "none" && designMode === "none") {
    designMode = m.fromZeroUiProject ? "michael_design_2_5_greenfield" : "michael_design_2_5_existing";
}

// 改为（任何有 UI 的交付物）
if ((m.ui || /^(desktop|app|automation|mixed|cli|ui_component)$/.test(deliverySurface)) 
    && workspaceAction !== "none" && designMode === "none") {
    designMode = m.fromZeroUiProject ? "michael_design_2_5_greenfield" : "michael_design_2_5_existing";
}
```

**风险等级**：⭐ 低（仅客户端本地推导，不改网关逻辑）

**优点**：
- ✅ 无须服务端改动与重部署
- ✅ prompt cache 前缀稳定（server/prompts.rs 层检测逻辑不变）
- ✅ 若 AI 判定出 designMode != "none"，客户端照样发出去
- ✅ 后备：即使客户端没升级，服务端网关仍有 looks_like_ui_task 兜底

**缺点**：
- ⚠️   硬编码 regex，若新增 deliverySurface 值需再改
- ⚠️  不处理服务端关键词漏洞（但可作为阶段 1）

**代码位置**：
- `/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js` 行 16941-16945

### 方案 B：服务端事实门放宽（中期）

**改动位置**：`server/src/prompts.rs` L2411-2573 (looks_like_ui_task)

**改动内容**：在 `looks_like_ui_task()` 的关键词集合中补充：
- 应用 UI 相关：`"应用界面", "应用 UI", "软件界面", "软件 UI", "GUI", "客户端界面", "客户端UI", "Electron", "Tauri", "PyQt", "Swing"`
- 通用设计：`"界面设计", "做界面", "改界面", "调界面", "美化", "重新设计界面", "界面优化"`
- 内容管理：`"后台 UI", "管理后台界面", "控制面板"`

**风险等级**：⭐⭐ 中低（改动服务端检测函数，需重部署）

**优点**：
- ✅ 捕获更多真实 UI 表述
- ✅ 不依赖客户端版本

**缺点**：
- ⚠️  关键词匹配本质上是穷举法，新措辞仍可能漏掉
- ⚠️  需服务端重启+容器重建镜像（154.44.13.133）
- ⚠️  与"语义判断禁用正则"的长期原则冲突（参见 memory: 语义判断禁用正则）

### 方案 C：长期：从关键词到 AI 语义判定（高风险）

**根本解决**：删掉 `looks_like_ui_task()` 的关键词匹配，改为 AI 模型判定

**改动位置**：`server/src/prompts.rs` L3661-3665

```rust
// 当前（关键词匹配）
let design = (mode == "agent" || mode == "plan")
    && (routed.get("x-ide-ui").is_some()
        || ((current_or_continuation_user_text_any(body, looks_like_ui_task)
            || conversation_shows_frontend_work(body))
            && (!automation || explicit_design)));

// 改为（取客户端 AI 判定结果）
let design = (mode == "agent" || mode == "plan")
    && (routed.get("x-ide-ui").is_some()
        || semantic_profile_contains(&semantic_profile, "design"));
```

**风险等级**：⭐⭐⭐⭐ 极高（需客户端、服务端协同改，prompt cache 前缀变化大）

**阻塞因素**：
- ⚠️ 需确保客户端 AI 判定层的 `designMode` 字段能通过某种编码传给网关
- ⚠️ 修改后 prompt cache 前缀可能变化 → 缓存命中率下降
- ⚠️ 依赖客户端更新发布周期

---

## 七、诊断总结

### 触发面过窄的具体证据

| 场景 | 用户表述 | 当前结果 | 根本原因 |
|------|--------|--------|--------|
| 网站建设 | "做一个官网" | ✅ 触发 | website 在白名单 |
| Web 应用 | "写个 React 仪表板" | ✅ 触发 | dashboard 在白名单 |
| 桌面应用 UI | "给 Electron 应用加个设置页" | ❌ 不触发 | deliverySurface=desktop，关键词无 Electron |
| 软件界面 | "做个漂亮的配置界面" | 🟡 可能漏掉 | "配置"未必触发 looks_like_ui_task |
| CLI 应用美化 | "改进这个命令行工具的 TUI" | ❌ 不触发 | 关键词无 TUI/CLI UI |
| 移动应用 | "设计一个移动应用首页" | 🟡 可能漏掉 | "移动"不在关键词，仅 responsive |
| 现有项目改界面 | 前文改过后端，现在说"改个界面" | 🟡 见代码迹象 | 仅 conversation_shows_frontend_work 可救 |

### 关键发现

1. **客户端层**：意图判定 AI 模型有能力识别 UI，但客户端在将结果映射为 `designMode` 时只考虑了 website/web_app 的 deliverySurface
2. **服务端层**：网关有事实门 looks_like_ui_task 做兜底，但关键词集合不完整
3. **两层都缺**：没有"任何 UI"的统一通道，都是基于具体措辞和代码迹象的被动识别

---

## 八、建议优先级

1. **立即（阶段 1，客户端无部署成本）**
   - 在客户端 L16941-16945 放宽 designMode 自动升级条件
   - 使 desktop/app/mixed/cli 等 deliverySurface 也能触发 michael-design
   - 改动风险最低，可验证客户端补全对用户体验的影响

2. **短期（阶段 2，服务端需重启）**
   - 补全 looks_like_ui_task 关键词表
   - 涵盖应用、桌面、客户端等表述
   - 作为降级防线，即使客户端未更新也能捕获

3. **中期（阶段 3，需重构）**
   - 考虑让客户端 AI 判定结果通过标准编码传递给服务端（而非仅依赖关键词）
   - 但需评估 prompt cache 前缀稳定性影响
   - 暂不推荐，因为涉及协议改动

---

## 九、实施路线与风险

### 路线 A（推荐）：客户端先行 + 服务端兜底
```
Week 1: 客户端 L16941-16945 放宽 designMode 自动升级
        ├─ 条件从 `m.ui` 放宽为 `m.ui || uiSurface`
        ├─ 无部署成本
        └─ 验证：UI 类任务是否都能注入 michael-design

Week 2: 服务端 looks_like_ui_task 补关键词
        ├─ 需服务端重启 + 容器重建（154.44.13.133）
        ├─ 只要客户端未改或改不及时，网关仍能兜底
        └─ GitHub 解耦的容器镜像需重构、推送、重启
```

### 风险清单

| 风险 | 等级 | 应对 |
|------|------|------|
| 无关任务被误判为 UI | ⭐⭐ | 关键词表精心审视，必须排除"说到界面就注入"的虚假触发 |
| prompt cache 命中率下降 | ⭐⭐⭐ | 阶段 1（客户端）不改 server/prompts.rs，cache 前缀稳定 |
| 新增 designMode 值未同步 | ⭐ | 维护 design_modes 枚举的同步检查表 |
| 服务端部署失败 | ⭐ | 阶段 1 不需部署；阶段 2 需容器测试 |

---

## 十、代码改动清单（具体行号）

### 客户端改动
**文件**：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js`

**行 16941-16945**（放宽 designMode 自动升级）：
```javascript
// 改前
if (m.ui && workspaceAction !== "none" && designMode === "none") {
    designMode = m.fromZeroUiProject ? "michael_design_2_5_greenfield" : "michael_design_2_5_existing";
}

// 改后（任何 UI deliverySurface 都触发）
const uiRelatedSurface = /^(desktop|app|automation|mixed|cli|ui_component|website|web_app)$/.test(deliverySurface);
if ((m.ui || uiRelatedSurface) && workspaceAction !== "none" && designMode === "none") {
    designMode = m.fromZeroUiProject ? "michael_design_2_5_greenfield" : "michael_design_2_5_existing";
}
```

### 服务端改动（可选，阶段 2）
**文件**：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/server/src/prompts.rs`

**行 2448-2507**（补充 CJK 关键词）：
```rust
const CJK_KW: &[&str] = &[
    // 现有词...
    // 新增：应用 UI 相关
    "应用界面", "应用 UI", "软件界面", "软件 UI", "GUI", 
    "客户端界面", "客户端 UI", "Electron", "Tauri", "PyQt", "Swing",
    // 新增：通用设计
    "改界面", "调界面", "重新设计", "界面美化", "界面优化",
    // 新增：后台管理
    "后台界面", "管理后台", "控制面板",
];
```

---

## 十一、验证计划

### 验收标准

实施后应验证以下场景都能触发 michael-design 注入：

```javascript
// 场景 1：桌面应用
{ deliverySurface: "desktop", designMode: "michael_design_2_5_*" }

// 场景 2：CLI 应用 UI
{ deliverySurface: "cli", designMode: "michael_design_2_5_*" }

// 场景 3：混合应用
{ deliverySurface: "mixed", designMode: "michael_design_2_5_*" }

// 场景 4：服务端检测（网关转发后）
语义路由 semantic("design") === true for 用户说"做个漂亮的 Python 应用界面"
```

### 测试用例
1. 用户："给这个 Tauri 应用改个新的设置页面"
   - 预期：michael-design 蓝本应被注入
   
2. 用户："做个 CLI 工具的彩色菜单界面"
   - 预期：design_knowledge_block 应被注入

3. 用户："改一下软件的配色方案，用 michael-design"
   - 预期：design_system.txt 应被注入

---

## 十二、相关代码路径汇总

### 知识库与系统提示
- 权威设计体系：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/server/prompts/design_system.txt`
- AI 意图判定提示（客户端）：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js` L16820-16850
- 工程字段定义：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js` L16833-16844

### 客户端意图判定
- 维度定义：L16562-16579
- AI 判定主函数：L16808-16870
- verdict 规范化：L16717-16789
- designMode 映射：L16909-16945

### 服务端语义路由
- 网关入口：`/Users/michael/Desktop/Michael-IDE/Devin-Desktop/server/src/prompts.rs` L3202
- UI 意图判定：L3661-3665（核心决策树）
- looks_like_ui_task 关键词表：L2411-2573
- design_knowledge_block 注入：L1826-1897
- ensure_design_backbone_hits 保底：L1903-1944

### 蓝本与知识检索
- 知识库检索入口：L1946-2010
- 蓝本格式化：L1869-1879
- 编排骨干保底逻辑：L1899-1944

---

## 附录：前缀稳定性影响评估

### 当前 cache 前缀构成
- agent.txt base（常驻，~12KB）
- 条件模块（engineering/research/automation 等，按 semantic_profile 加载）
- design_knowledge_block（仅 ui_intent=true 时，~4KB）

### 阶段 1 改动的影响
- **改客户端 L16941-16945**：仅改本地推导逻辑，不改网关检测
- 发送给网关的 x-ide-semantic-profile 头可能变化（新场景会标 design=true）
- **影响**：desktop/mixed 类任务的 designMode 从 "none" 变为 "michael_design_2_5_*"
- **结果**：这些任务的 system prompt 会加 design_knowledge_block，与网站任务一致
- **缓存影响**：desktop/mixed 用户第一次命中后缓存得分下降（新 prefix），但后续 desktop 用户共享缓存

### 建议
- 阶段 1 改动**不会破坏** website/web_app 用户的缓存（最大用户群）
- 仅新增 desktop/mixed 用户的缓存段
- 若有性能忧虑，可在阶段 2 评估是否值得

---

## 结论与建议

**根本原因**：michael-design 的注入触发依赖两层关键词和 deliverySurface 的白名单，导致"网站"以外的 UI 场景被遗漏。

**立即行动**：
1. 执行方案 A（客户端放宽）→ 0 部署成本，低风险，高收益
2. 补充关键词（方案 B）→ 作为事实门控的降级防线

**长期方向**：
- 建立"任何 UI"的统一识别通道，而非逐个 deliverySurface 白名单
- 考虑让服务端直接信任客户端 AI 判定（不再靠关键词），但需评估 cache 影响

