# find_files 工具全面失效 — 只读诊断报告

> 诊断时间: 2026-07-31 | 诊断范围: `src/main.js` (56583 行) + `src-tauri/src/files.rs` | 只读，未改代码

---

## 一、实现链定位（行号均以 grep 实查为准）

| 环节 | 位置 | 行号 |
|------|------|------|
| **工具 schema 定义** | `_buildAgentToolSchemas` 内 | **L25336** |
| **调用映射** | `_mapToolCall` 内 `case "find_files"` | **L26564** |
| **执行入口** | `_executeToolStep` 内 `call.type === "find"` | **L41400–L41413** |
| **核心执行函数** | `_agentFindFiles(root, pattern)` | **L30975–L31006** |
| **glob→RegExp 转换** | `_globToRegExp(glob)` | **L30540–L30554** |
| **后端 readDir (Tauri)** | `tauriBackend()` 返回 | **L287** |
| **后端 readDir (Remote)** | `_remote.active` 分支 | **L210–L217** |
| **Rust read_dir** | `src-tauri/src/files.rs` | **L402–L422** |

---

## 二、根因诊断（逐项检查 + 代码证据）

### 2.1 glob 匹配逻辑 — ✅ 基本正确，但有局限

**实现**: 手写 `_globToRegExp` (L30540–L30554)，将 glob 转为 RegExp。

**转换逻辑**:
- `*` → `[^/]*`（单星不跨目录）
- `**` → `.*`（双星跨目录）
- `?` → `[^/]`（单字符不跨目录）
- 无 `*?` 的纯文本 → 子串匹配（非锚定）
- 最终正则锚定: `(^|/)...$`，flag `i`

**实测推演** (以 `*.yml` 为例):
```
glob = "*.yml"
→ re = "[^/]*\\.yml"
→ RegExp("(^|/)[^/]*\\.yml$", "i")
```
- 匹配 `config.yml` ✅（`^` + `config` + `.yml` + `$`）
- 匹配 `src/config.yml` ✅（位置 3 的 `/` 命中 `(^|/)` 的 `/` 分支）

**结论**: 对 `*.yml`、`*.config.*`、`README*` 这三类常见模式，正则本身**是正确的**。

**局限**: 不支持字符类 `[abc]`、交替 `{a,b}`、取反 `!`。但用户报告的模式不涉及这些。

### 2.2 ⭐ 根因 #1 — 隐藏文件/目录过滤过激（L30994）

```javascript
// L30994
if (!name || name.startsWith(".")) continue;
```

**问题**: 这一行同时过滤了**文件和目录**。所有以 `.` 开头的条目被无条件跳过：
- **文件**: `.env`, `.eslintrc.json`, `.prettierrc`, `.editorconfig`, `.gitignore`, `.npmrc` 等全部不可见
- **目录**: `.claude`, `.github`, `.vscode`, `.idea` 等整个子树不被遍历

**对比**: 同文件的 `_workspaceTreeSnapshot` (L18904) 用了更精细的白名单:
```javascript
// L18904 — 允许特定 dotfiles
if (!options.includeHidden && name.startsWith(".") 
    && ![".env", ".env.local", ".env.development", ".github"].includes(name)) 
    return false;
```

**影响**: 如果用户的项目配置文件大多是 dotfiles（非常常见），find_files 会表现为"什么都找不到"。

### 2.3 ⭐ 根因 #2 — `e.is_dir` 检查脆弱，与项目其他函数不一致（L30996）

```javascript
// L30996 — _agentFindFiles 的写法
if (e.is_dir) {
```

**对比**: 同文件其他遍历函数用的是兼容性更好的 helper:
```javascript
// L18882–L18884
function _agentDirEntryIsDir(entry) {
  return !!(entry?.is_dir || entry?.isDir || entry?.kind === "dir" || entry?.type === "dir");
}
```

**风险**: 如果后端返回的字段名不是 `is_dir`（例如 remote daemon 返回 `isDir`），`e.is_dir` 为 `undefined`（falsy），**所有目录都会被当作文件处理**:
- 目录不会被压入 stack → **不递归子目录**
- 目录名会被当文件名测试 regex → 几乎不可能匹配
- 结果: 只搜索根目录层的文件，子目录全部跳过

**当前 Tauri 后端确实返回 `is_dir`**，所以本地模式不受影响。但 remote 模式、以及任何后端变更都会触发此 bug。

### 2.4 ⭐ 根因 #3 — 静默吞错导致"无匹配"无任何诊断信息（L30989）

```javascript
// L30989
try { entries = await backend.readDir(dir); } catch { continue; }
```

**问题**: 如果 `backend.readDir` 抛异常，错误被**完全吞掉**，循环跳到下一个目录。如果所有 readDir 都失败（例如 `require_inside_workspace` 因路径问题拒绝），函数会安静地返回 `(无匹配文件)`，没有任何错误提示。

**可能的失败场景**:
- 工作区根目录未正确注册到 Rust 后端的 `ALLOWED_ROOTS`
- 路径含中文/特殊字符导致 `canonicalize` 失败
- 符号链接解析后落在工作区外
- Tauri IPC 通信异常

**对比**: `_agentFindProjectFiles` (L18955) 同样静默吞错，但 `_workspaceTreeSnapshot` (L18898) 也是。这是项目的通用模式，但对于 find_files 这个**用户直接面对的工具**，应该有错误反馈。

### 2.5 目录遍历 — ✅ 基本正确

- 使用 stack-based DFS（`stack.pop()`），递归遍历子目录
- 排除集合 `IGNORED` (L30981): `.git`, `node_modules`, `target`, `dist`, `build`, `.next`, `.venv`, `__pycache__`, `.cache`, `vendor`
- 上限: MAX=200 个结果, MAX_SCAN=8000 个条目
- 注意: `IGNORED` 中的 `.git`/`.next`/`.venv`/`.cache` 已被 `startsWith(".")` 提前过滤，属于冗余

### 2.6 路径解析 — ✅ 正确

- `root` 来自 `_executeToolStep` 的参数，是已验证的工作区绝对路径
- 子目录路径拼接: `dir + "/" + name`，macOS/Linux 下正确
- Rust 后端 `require_inside_workspace` (files.rs L314) 支持 HOME 目录下任意路径

### 2.7 返回格式 — ✅ 正确

```javascript
// L31004–L31005
const text = out.length ? out.join("\n") + (...) : "(无匹配文件)";
return { count: out.length, text, files: out };
```

返回相对路径列表，换行分隔。格式正确。

---

## 三、与用户期望的差距

| 用户期望 | 现有实现 | 差距 |
|----------|----------|------|
| 按 glob 递归搜工作区 | ✅ 支持递归 + 基本 glob | 无 |
| 返回匹配文件列表（相对路径） | ✅ 返回相对路径列表 | 无 |
| 能找到 dotfiles（`.env` 等） | ❌ `startsWith(".")` 全部跳过 | **过激过滤** |
| 能找到 `.github/` 下的文件 | ❌ 隐藏目录不遍历 | **过激过滤** |
| 搜索失败时有诊断信息 | ❌ 静默返回"无匹配" | **无错误反馈** |
| 支持 `*.yml` 等常见模式 | ✅ 正则逻辑正确 | 无 |
| 健壮的后端字段兼容 | ❌ 硬编码 `e.is_dir` | **脆弱** |

---

## 四、重写建议

### 4.1 glob 匹配

- **推荐**: 继续使用手写转换（零新增依赖），但补全字符类支持
- **备选**: 引入 `minimatch`（~10KB gzipped）— 但项目风格偏好零新增依赖
- 转换结果改为匹配 **basename** 而非完整相对路径（见 4.4）

### 4.2 遍历策略

- 递归 DFS，保持现有 stack-based 模式
- 深度限制: 默认无限制，但受 MAX_SCAN 约束
- **排除规则精简为**:
  ```javascript
  const IGNORED = new Set([
    "node_modules", ".git", "target", "dist", "build",
    ".next", ".nuxt", ".svelte-kit", ".venv", "venv",
    "__pycache__", ".cache", ".turbo", ".vite", "coverage",
    ".idea", ".vscode", "vendor"
  ]);
  ```
- **dotfiles 处理**: 不再用 `startsWith(".")` 一刀切。改为：
  - 隐藏**目录**仍跳过（`.git` 等已在 IGNORED 中）
  - 隐藏**文件**允许匹配（`.env`, `.eslintrc.json` 等）
  - 如需限制，用白名单: `[".env", ".env.*", ".gitignore", ".eslintrc*", ".prettierrc*"]`

### 4.3 健壮性

- 用 `_agentDirEntryIsDir(entry)` 替代 `e.is_dir`
- 用 `entry.path` 构造子目录绝对路径（而非手动拼接 `dir + "/" + name`）
- readDir 失败时记录错误，超过 N 次连续失败提前终止并返回错误信息

### 4.4 匹配策略优化

当前: regex 匹配完整相对路径 `childRel`
建议: regex 匹配 **basename**（文件名部分），`**` 模式才匹配路径

```
*.yml     → 匹配 basename 以 .yml 结尾的文件
README*   → 匹配 basename 以 README 开头的文件
src/**/*.ts → 匹配路径以 src/ 开头、basename 以 .ts 结尾的文件
```

这样更符合用户直觉，也与其他 IDE 的文件搜索行为一致。

### 4.5 返回格式

- 相对路径列表，换行分隔
- 最多返回 200 个（保持现有 MAX）
- 超过上限提示"更多结果已截断"
- **新增**: 无匹配时返回工作区根目录的文件样本（帮助用户调整 pattern）

### 4.6 项目风格对齐

- 中文注释
- 下划线私有函数（`_agentFindFiles`）
- 零新增依赖优先
- 使用现有 helper: `_agentDirEntryName`, `_agentDirEntryIsDir`, `_normalizeFsPath`

---

## 五、结论

**find_files "全面失效"的根因是多重缺陷叠加**:

1. **`name.startsWith(".")` 过激过滤** (L30994) — 跳过所有 dotfiles 和隐藏目录，这是用户感知"什么都找不到"的最直接原因。现代项目的配置文件大量是 dotfiles（`.env`, `.eslintrc`, `.prettierrc` 等），全部被过滤。

2. **`e.is_dir` 脆弱检查** (L30996) — 在非 Tauri 后端（remote 模式）下可能导致目录不被递归，搜索退化为仅扫描根目录。

3. **静默吞错** (L30989) — readDir 失败时无任何反馈，用户只看到"无匹配"，无法区分"真的没有匹配文件"和"搜索过程本身出错了"。

glob→regexp 转换逻辑本身**是正确的**，`*.yml` / `*.config.*` / `README*` 的正则都能正确匹配。问题不在匹配算法，而在**遍历层把候选文件过滤光了**。

---

## 附录: 关键行号速查

| 功能 | 行号 |
|------|------|
| `_globToRegExp` | 30540–30554 |
| `_agentFindFiles` | 30975–31006 |
| `name.startsWith(".")` 过激过滤 | 30994 |
| `e.is_dir` 脆弱检查 | 30996 |
| `backend.readDir` 静默吞错 | 30989 |
| `IGNORED` 集合 | 30981 |
| `_agentDirEntryIsDir` (应使用的 helper) | 18882–18884 |
| `_agentDirEntryName` (应使用的 helper) | 18878–18880 |
| `_mapToolCall` find_files 分支 | 26564 |
| `_executeToolStep` find 分支 | 41400–41413 |
| 工具 schema | 25336 |
| Rust `read_dir` | files.rs L402–422 |
| Rust `require_inside_workspace` | files.rs L314–388 |
