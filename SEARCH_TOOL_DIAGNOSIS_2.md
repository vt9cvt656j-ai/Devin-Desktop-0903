# Search Tool 诊断报告：`api.cursor.sh` 无匹配 vs `__kcUpdateBlocker` 6 处匹配

## 结论摘要

**搜索工具对含特殊字符的查询没有 bug。** Rust 后端在 literal 模式下使用 `regex::escape()` 正确转义了所有正则元字符（`.`、`/`、`*` 等）。两个查询结果不同的唯一原因是：`__kcUpdateBlocker` 当时存在于项目文件中，而 `api.cursor.sh` 不存在。

**当前代码库验证**：`grep -r` 确认两个字符串在当前项目中均不存在（文件可能已被修改/删除）。

---

## 调查路径

### 1. 搜索工具调用链（前端 → 后端）

| 层级 | 文件 | 行号 | 关键代码 |
|------|------|------|----------|
| 工具定义 | `src/main.js` | 25335 | `search` 工具 schema，`mode` 默认 `literal` |
| 参数解析 | `src/main.js` | 26563 | `case "search"` → 构造 `{ type: "search", query, searchPath, mode }` |
| 执行入口 | `src/main.js` | 41481-41539 | `call.type === "search"` 分支，调用 `backend.searchInProject()` |
| Tauri 桥接 | `src/main.js` | 309-313 | `core.invoke("search_in_project", { root, query, caseSensitive, mode })` |
| Rust 命令 | `src-tauri/src/files.rs` | 1803-1812 | `search_in_project()` → `search_project_scope()` |
| 匹配器构建 | `src-tauri/src/files.rs` | 1597-1625 | `build_search_matcher()` — **核心** |
| 行内匹配 | `src-tauri/src/files.rs` | 1627-1636 | `find_matches_in_line()` 使用 `Regex::find_iter` |

### 2. 特殊字符处理分析

**`build_search_matcher`** (`files.rs:1606-1619`)：

```rust
let pattern = match mode.unwrap_or("literal").trim().to_ascii_lowercase().as_str() {
    "literal" => regex::escape(query),   // ← 所有元字符被转义
    "regex" => query.to_string(),         // ← 原始正则
    other => return Err(...),
};
```

- `api.cursor.sh` → 被转义为 `api\.cursor\.sh` → 作为字面量匹配，**行为正确**
- `__kcUpdateBlocker` → 无元字符，转义后不变 → **行为正确**

**结论**：Rust 后端不存在 glob 匹配问题、分词问题或路径解析问题。`regex::escape()` 是标准库函数，对 `.`、`/`、`|` 等所有元字符均正确转义。

### 3. 搜索范围与过滤规则

`search_project_scope` (`files.rs:1643-1795`) 的过滤规则：

| 过滤条件 | 行号 | 说明 |
|----------|------|------|
| `IGNORED_DIRS` | 1692-1698 | 跳过 `node_modules`、`target`、`dist`、`build`、`out`、`vendor`、`__pycache__`、`coverage` |
| 点目录 | 1709-1711 | 跳过所有以 `.` 开头的目录（如 `.git`、`.claude`） |
| 文件大小 | 1716 | 跳过 > 2 MiB 的文件 |
| 二进制文件 | 1724-1726 | 跳过前 8000 字节含 `NUL (0x00)` 的文件 |
| 非 UTF-8 | 1727-1730 | 跳过无法解析为 UTF-8 的文件 |
| 符号链接 | 1688-1690 | 跳过符号链接 |

**注意**：`dist-web/` 不在 `IGNORED_DIRS` 中（只有 `dist` 被忽略），所以 `dist-web/` 内的打包文件会被搜索。

### 4. 唯一发现的实际问题：模糊失败搜索去重

**位置**：`src/main.js:37301-37323`

**机制**：当一个搜索返回"无匹配"后，系统会对后续相似查询进行去重跳过。

**规范化逻辑**（行 37308）：
```js
const _qN = _qRaw.toLowerCase()
  .replace(/[|\\.*+?^${}()[\]]/g, " ")  // 元字符替换为空格
  .replace(/\s+/g, " ").trim();
```

**去重判定**（行 37313-37314）：
```js
_qN === entry.query || _qN.includes(entry.query) || entry.query.includes(_qN)
```

**潜在问题**：`includes` 子串匹配过于宽松。

| 场景 | 规范化后 | 是否被误拦 |
|------|----------|------------|
| 先搜 `api`（无匹配），再搜 `api.cursor.sh` | `"api"` vs `"api cursor sh"` | **是** — `"api cursor sh".includes("api")` 为 true |
| 先搜 `cursor`（无匹配），再搜 `api.cursor.sh` | `"cursor"` vs `"api cursor sh"` | **是** — `"api cursor sh".includes("cursor")` 为 true |
| 先搜 `__kcUpdateBlocker`（有匹配） | 不进入失败队列 | 不影响 |

**影响**：如果 Agent 先搜索了一个短词（如 `api`）且无匹配，后续搜索 `api.cursor.sh` 会被误判为"相似搜索"而跳过，返回"无匹配"——即使该文本实际存在。这可以解释用户观察到的行为。

**但**：这个去重只在 Agent 工具调用路径中生效（行 37305），侧边栏 UI 搜索不受影响。如果用户截图是 Agent 搜索结果，此 bug 可能是根因之一。

---

## 根因判定

| 假设 | 验证结果 |
|------|----------|
| 特殊字符导致正则编译失败 | **否** — `regex::escape()` 正确处理 |
| glob 匹配不支持 `.`/`/` | **否** — 使用 literal 模式，非 glob |
| 分词导致 `api.cursor.sh` 被拆分 | **否** — 无分词步骤 |
| 路径解析把 query 当 path | **否** — query 和 path 是独立参数 |
| 文本确实不存在于搜索范围 | **最可能** — `grep -r` 确认当前代码库无此字符串 |
| 模糊去重误拦 | **可能**（仅 Agent 路径）— 若先前搜过短词且失败 |

---

## 修复建议

### P0：模糊去重子串匹配过于宽松

**文件**：`src/main.js`，行 37313-37314

**现状**：`_qN.includes(entry.query) || entry.query.includes(_qN)` — 任意子串关系即视为相同搜索

**建议**：改为编辑距离或词集合交叠判定，避免 `"api"` 拦截 `"api.cursor.sh"`：

```js
// 方案 A：要求长度比 > 0.7（短词不拦截长词）
const ratio = Math.min(_qN.length, entry.query.length) / Math.max(_qN.length, entry.query.length);
if (ratio > 0.7 && (_qN === entry.query || _qN.includes(entry.query) || entry.query.includes(_qN))) { ... }

// 方案 B：仅精确匹配去重
if (_qN === entry.query) { ... }
```

### P1：无需修改

搜索核心实现（Rust `build_search_matcher` + `search_project_scope`）逻辑正确，不需要修改。

---

## 关键文件索引

| 文件 | 行号 | 内容 |
|------|------|------|
| `src/main.js` | 25335 | search 工具 schema 定义 |
| `src/main.js` | 26563 | 工具参数解析 |
| `src/main.js` | 41481-41539 | search 执行分支 |
| `src/main.js` | 37301-37323 | 模糊失败搜索去重（**有 bug**） |
| `src-tauri/src/files.rs` | 1597-1625 | `build_search_matcher` — literal 模式转义 |
| `src-tauri/src/files.rs` | 1643-1795 | `search_project_scope` — 目录遍历+匹配 |
| `src-tauri/src/files.rs` | 28-37 | `IGNORED_DIRS` 列表 |
