# 跨目录感知机制诊断报告

> 诊断目标：找出 AI "瞎猜路径" 的根因，只读调查，不改代码。
> 主文件：`src/main.js`（行号基于 grep 实际定位）

---

## 一、现有机制全景

### 1.1 AI 获取项目结构的方式：快照 + 按需 list_dir 两层结合

| 层级 | 机制 | 函数 | 行号 |
|------|------|------|------|
| **L1 系统快照** | `_workspaceTreeSnapshot()` | main.js:18916 | 每轮对话前自动注入 system prompt |
| **L2 按需探索** | `list_dir` 工具 | main.js:41055 | AI 主动调用，列出任意目录 |
| **L3 语义索引** | BM25 倒排索引 | main.js:30805-30855 | 后台构建，按查询相关度返回代码片段 |
| **L4 符号索引** | `_buildRepoMap()` | main.js:18812 | Aider 风格，按文件列出符号定义 |

### 1.2 快照实现细节（`_workspaceTreeSnapshot`）

- **深度**：默认 `maxDepth=3`，大上下文档位（`_contextBudgetScale()>1`）时 `maxDepth=4`（main.js:19188）
- **行数上限**：默认 `maxLines=160`，大档位时 `640`（main.js:19188）
- **跳过目录**：`node_modules`, `.git`, `dist`, `build`, `.next`, `target`, `__pycache__` 等 14 个（main.js:18902-18906）
- **隐藏文件**：默认跳过，仅保留 `.env`, `.env.local`, `.env.development`, `.github`
- **输出格式**：`📁 src/` + 缩进子项，每行含**相对路径**（相对于 root）
- **截断提示**：超限时追加 `…（只展示前 N 项；需要更深目录时用 list_dir 精确查看）`

### 1.3 缓存机制（`_agentContextCache`）

- **声明**：main.js:16428 `{ root, rootsKey, activeKey, ts, data }`
- **失效策略**：
  - 文件变更时 `handleFsChanges` 将 `ts=0`（main.js:8331）
  - 5 分钟 TTL 安全网（main.js:19152）
  - 顶层目录指纹对比（`rootFp`）：文件名+目录标记排序拼接，变化即重建（main.js:19172）
  - 规模突变检测：文件数/大小从有到零或缩小 80%+ 强制刷新（main.js:19163-19165）
- **每轮核对**：`_agentContextSnapshotForTurn` 在用户每次发消息时执行一次 `readDir(root)` 顶层指纹对比（main.js:19316-19324）

### 1.4 多根（Multi-root）感知

- **`_allRoots()`**（main.js:39616）：收集所有打开的工作区根目录，active 优先
- **`_relCandidates()`**（main.js:39656）：相对路径解析时尝试所有根，支持 root-name-qualified 路径（如 `proj2/src/x`）
- **注入提示**：多根时在上下文中注入警告（main.js:19237）：
  > "现在打开了 N 个工作区文件夹。跨目录操作时**用绝对路径，或在相对路径前加目标文件夹名**"
- **其他根的树**：每个额外根目录单独生成快照（`maxLines=40, maxDepth=2`），拼入上下文（main.js:19191-19192, 19238-19241）

---

## 二、盲区诊断：AI 为什么会"瞎猜路径"

### 2.1 盲区 A：快照深度不够，深层文件不可见

**根因**：`maxDepth=3`（甚至大档位也只有 4 层），超过此深度的文件对 AI **完全不可见**。

**影响场景**：
- 深层嵌套目录（如 `src/components/ui/atoms/Button.tsx`）在 4 层快照中勉强可见，但 `src-tauri/src/handlers/auth/mod.rs` 这类 5 层+路径会被截断
- 截断时只给出 `…（只展示前 N 项；需要更深目录时用 list_dir 精确查看）`，但 AI 不一定知道要去 list_dir

**证据**：main.js:18920 `maxDepth = Math.max(1, Math.min(5, Number(options.maxDepth) || 3))`，硬上限 5。

### 2.2 盲区 B：AI 无法主动请求"完整目录树"

**根因**：AI 没有"给我完整树"的工具。`list_dir` 只能一次列一个目录，且 AI 不知道哪些深层目录值得展开。

**影响场景**：
- AI 看到 `src/` 下有 `components/`，但不知道里面有什么，只能猜文件名去 `read_file`
- 没有 `find_files` 的语义变体（按内容/功能描述找文件），只有 glob/文件名匹配

**证据**：工具 schema（main.js:25375）`list_dir` 只接受一个 `path` 参数，没有 `recursive` 或 `depth` 选项。

### 2.3 盲区 C：路径建议时只有文件名，缺少完整路径

**根因**：`list_dir` 返回结果**只含文件名**（main.js:41092），不含完整相对路径。AI 必须自己拼接路径。

**影响场景**：
- AI 看到 `list_dir("src")` 返回 `handlers/`，但不知道 `handlers/` 下有什么
- 需要多次 `list_dir` 才能拼出完整路径，但 AI 经常偷懒直接猜

**证据**：main.js:41090-41092 返回的 listing 只含 `e.name`，不含相对路径。

### 2.4 盲区 D：没有项目结构索引（类似 ctags）

**根因**：虽然有 BM25 索引（main.js:30805）和符号索引（main.js:18812），但这些都是**内容级**索引，不是**结构级**索引。

**缺失**：
- 没有"文件功能摘要"索引（如 `src/auth/` = "认证模块，处理登录/注册/token"）
- 没有跨目录的文件关联图（哪些文件 import 了哪些）
- BM25 索引按 80 行分块，丢失文件整体结构

### 2.5 盲区 E：快照只覆盖当前工作区根

**根因**：`_workspaceTreeSnapshot` 只遍历传入的 `root`（main.js:18916），多根时其他根只有 40 行/2 层的简略快照（main.js:19191-19192）。

**影响场景**：
- 主工作区有 640 行配额，其他工作区只有 40 行，深层结构完全不可见
- 跨工作区文件引用时 AI 无法知道目标工作区的完整结构

### 2.6 盲区 F：缓存指纹只看顶层

**根因**：缓存失效指纹 `rootFp` 只对比**根目录顶层**文件名（main.js:19168-19173），深层目录变化不触发刷新。

**影响场景**：
- 在 `src/deep/nested/` 下新建文件，顶层指纹不变，缓存不刷新
- AI 继续用旧快照，不知道新文件存在，直到 5 分钟 TTL 过期

---

## 三、修复方向建议（仅建议，不实施）

### 3.1 增强快照深度（推荐优先级：高）

**方案**：将 `maxDepth` 从固定 3-5 改为**自适应**：
- 小项目（<200 文件）：递归到全部（无深度限制）
- 中型项目（200-1000 文件）：`maxDepth=5-6`
- 大型项目（>1000 文件）：保持当前 `maxDepth=3-4`，但增加"目录名列表"（只列子目录名不列文件）

**实现要点**：
- `_workspaceTreeSnapshot` 的 `maxDepth` 参数（main.js:18920）改为根据文件总数动态计算
- 增加 `visited` 计数器提前终止时的"目录骨架"模式：只输出目录名，不输出文件

### 3.2 增加 `list_dir` 递归模式（推荐优先级：高）

**方案**：给 `list_dir` 工具增加 `depth` 参数（默认 1，可设 2-5 或 "all"）。

**实现要点**：
- 工具 schema（main.js:25375）增加 `depth` 参数
- 执行逻辑（main.js:41055）传递 depth 到递归遍历
- 返回结果改为**带缩进的树形**（类似 `_workspaceTreeSnapshot` 的格式），而非扁平文件名列表

### 3.3 增加"项目结构摘要"索引（推荐优先级：中）

**方案**：类似 `_buildRepoMap`（main.js:18812），但按**目录**而非文件聚合：
- 每个子目录一行摘要：`src/auth/ → 认证模块（login, logout, token, middleware）`
- 基于符号索引 `_symbolIndex` 聚合，零额外 IO

**实现要点**：
- 在 `_agentContextForQuery`（main.js:19084）中增加目录级摘要段
- 利用现有 `_symbolIndex` 按目录分组统计符号名

### 3.4 缓存指纹增强（推荐优先级：中）

**方案**：指纹不仅对比顶层，还对比**一级子目录**的文件数：
- `src/:42, test/:15, package.json:1` 这样的紧凑指纹
- 任何子目录文件数变化即触发刷新

**实现要点**：
- main.js:19168-19173 的指纹构建逻辑扩展为遍历一级子目录
- 保持 O(n) 复杂度（只统计数量，不列全部文件名）

### 3.5 list_dir 返回完整相对路径（推荐优先级：低）

**方案**：`list_dir` 结果中每个文件名前加相对路径前缀：
- 当前：`Button.tsx`
- 改为：`atoms/Button.tsx`（当 list_dir 目标是 `src/components` 时返回 `src/components/atoms/Button.tsx`）

**实现要点**：
- main.js:41090-41092 的 listing 构建逻辑改为输出相对路径
- 注意不要破坏现有 AI 对 list_dir 输出格式的隐式依赖

### 3.6 增加 `get_project_tree` 专用工具（推荐优先级：低）

**方案**：新增一个专门的全局树工具，返回完整项目结构（类似 VS Code 的文件树）：
- 不受 `maxDepth` 限制
- 自动跳过 `node_modules` 等
- 输出紧凑的树形文本

**权衡**：增加工具数量会增加 AI 的选择负担，可能不如增强现有 `list_dir` 的 depth 参数。

---

## 四、结论

### "瞎猜路径"根因排序

| 优先级 | 根因 | 影响程度 |
|--------|------|----------|
| **P0** | 快照深度只有 3-4 层，深层文件不可见 | AI 完全不知道深层文件存在，只能猜 |
| **P0** | `list_dir` 返回只有文件名无路径 | AI 看到文件但不知道完整路径，拼接出错 |
| **P1** | 没有递归 list_dir，AI 无法一次展开多层 | AI 需要多次调用才能探索深层，经常放弃 |
| **P1** | 缓存指纹只看顶层，深层变化不感知 | 外部修改深层文件后 AI 用旧快照 |
| **P2** | 缺少目录级功能摘要 | AI 不知道每个目录"是干什么的" |
| **P2** | 多根时其他根只有 40 行快照 | 跨工作区操作时结构信息严重不足 |

### 核心矛盾

**现有架构已经做了"快照 + 按需 + 缓存"三层设计，但每层都有信息损失**：
- 快照层：深度截断（3-4 层）
- 按需层：只返回文件名不含路径
- 缓存层：指纹只看顶层

**AI 的"瞎猜"本质上是信息缺失下的合理行为**——它看不到完整结构，只能根据文件名模式猜测。修复方向应该是**让真实结构对 AI 可见**，而不是靠提示词禁止猜测。

---

## 五、关键代码位置索引

| 功能 | 函数/变量 | 行号 |
|------|-----------|------|
| 目录树快照 | `_workspaceTreeSnapshot()` | 18916 |
| 跳过目录集合 | `_AGENT_CONTEXT_SKIP_DIRS` | 18902 |
| 上下文缓存 | `_agentContextCache` | 16428 |
| 缓存失效（文件变更） | `handleFsChanges` 中 `ts=0` | 8331 |
| 每轮快照核对 | `_agentContextSnapshotForTurn()` | 19301 |
| 顶层指纹对比 | `rootFp` 构建 | 19168-19173 |
| 多根收集 | `_allRoots()` | 39616 |
| 相对路径候选 | `_relCandidates()` | 39656 |
| list_dir 执行 | `call.type === "list"` | 41055 |
| list_dir 返回格式 | listing 构建 | 41090-41092 |
| BM25 索引 | `_bm25Index` / `buildBM25Index()` | 30805 / 30841 |
| 符号 RepoMap | `_buildRepoMap()` | 18812 |
| 上下文预算缩放 | `_contextBudgetScale()` | 11694 |
| 工具 schema（list_dir） | `_buildAgentToolSchemas` | 25375 |
