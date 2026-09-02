# Michael IDE 安全审计报告

**审计日期**: 2026-07-20  
**审计范围**: Tauri 配置、Rust 后端代码、文件操作、命令执行、认证存储  
**审计方法**: 静态代码分析 + 配置审查

---

## 🔴 高危漏洞 (Critical)

### 1. 路径穿越 — 文件系统完全暴露

**位置**: `src-tauri/tauri.conf.json:18`

**问题**:
```json
"assetProtocol": { 
  "enable": true, 
  "scope": ["**", "/**", "$HOME/**"] 
}
```

- `"**"` 和 `"/**"` 允许前端通过 `convertFileSrc()` 读取**整个文件系统**
- 攻击向量: `asset://localhost/etc/passwd`、`asset://localhost/Users/xxx/.ssh/id_rsa`
- 绕过了后端 `require_inside_workspace()` 的校验

**影响**: 
- 读取系统密钥、SSH 私钥、浏览器 cookie、其他应用数据
- 本地权限提升

**修复建议**:
```json
"assetProtocol": { 
  "enable": true, 
  "scope": ["$RESOURCE/**", "$APPDATA/**", "$TEMP/**"]
}
```

**验证方法**:
```javascript
// 前端测试（修复前应该失败）
const url = window.__TAURI__.convertFileSrc('/etc/passwd');
fetch(url).then(r => r.text()).then(console.log);
```

---

### 2. 命令注入风险 — 缺少参数白名单

**位置**: 
- `src-tauri/src/browser.rs:1097` — `Command::new("kill").args(["-9", pid])`
- `src-tauri/src/process_util.rs:20` — Shell 命令执行

**问题**:
- 多处 `Command::new()` 直接传递外部输入
- 虽然 `pid` 来自 `pgrep` 输出，但如果进程名可控（通过其他漏洞注入），仍有风险
- Git 操作如果路径拼接不当，可能执行任意命令

**潜在攻击**:
```rust
// 如果某处用户能控制路径
git_clone("; rm -rf /", target)  
```

**修复建议**:
1. 所有外部命令参数用**白名单验证**
2. 路径参数用 `Path::canonicalize()` + 前缀校验
3. Git 参数用 `libgit2` 库代替 shell 调用

**验证方法**:
```rust
#[test]
fn test_command_injection() {
    let malicious_path = "; rm -rf /tmp/test";
    assert!(git_clone(malicious_path, "/tmp/out").is_err());
}
```

---

### 3. 凭据明文存储

**位置**: 
- `src-tauri/src/auth.rs:255` — `~/.michael_ide/smtp.env` 明文存储
- SQLite 数据库未加密

**问题**:
```rust
if let Ok(txt) = std::fs::read_to_string(format!("{home}/.michael_ide/smtp.env")) {
    // 直接读取明文密码
}
```

**影响**:
- 本地恶意软件直接读取 `~/.michael-ide/*.db` 和 `.env` 获取所有 API 密钥
- 横向渗透到其他服务

**修复建议**:
- **macOS**: 用 `security` 命令存取 Keychain
- **Windows**: 用 Windows Credential Manager
- **Linux**: 用 `libsecret` 或加密存储

```rust
// macOS 示例
use security_framework::passwords::*;
set_generic_password("Michael IDE", "smtp_password", password.as_bytes())?;
```

---

## 🟡 中危问题 (High)

### 4. XSS 风险 — CSP 允许 blob: 协议

**位置**: `src-tauri/tauri.conf.json:19`

**问题**:
```
script-src 'self' blob:
```

- 允许 `blob:` 很危险 — 攻击者可通过 `URL.createObjectURL(new Blob([evil]))` 绕过 CSP
- 如果有 HTML 预览功能，打开恶意 HTML 会执行脚本

**修复建议**:
```
script-src 'self';  # 移除 blob:
```
如果确实需要 Worker，用 `worker-src 'self' blob:` 单独控制

---

### 5. TOCTOU 竞态条件

**位置**: `src-tauri/src/files.rs:125-152`

**问题**:
```rust
let temporary_path = stage_text_file(path, content, original_permissions)?;
// ⏰ 时间窗口：恶意进程可修改目标文件
if let Err(error) = atomic_replace_file(&temporary_path, path) {
    let _ = std::fs::remove_file(&temporary_path);  // 💥 失败时临时文件泄露
```

**影响**:
- 并发写入时数据损坏
- `/tmp` 目录泄露敏感文件

**修复建议**:
```rust
use tempfile::NamedTempFile;
let tmp = NamedTempFile::new()?;
tmp.write_all(content.as_bytes())?;
tmp.persist(path)?;  // 原子操作
```

---

### 6. DoS — 无限制文件读取

**位置**: `src-tauri/src/files.rs:566`

**问题**:
```rust
// read_file_data_url 没有大小限制
let bytes = std::fs::read(&path).map_err(...)?;
```

**攻击**:
- 打开 10GB 文件会 OOM 崩溃

**修复建议**:
```rust
const MAX_DATA_URL_SIZE: u64 = 10 * 1024 * 1024;  // 10MB
let meta = std::fs::metadata(&path)?;
if meta.len() > MAX_DATA_URL_SIZE {
    return Err("file too large for data URL".into());
}
```

---

### 7. 信息泄露 — Panic 日志暴露路径

**位置**: `src-tauri/src/lib.rs:71-88`

**问题**:
```rust
let msg = format!(
    "\n===== PANIC (unix {ts}) =====\n{info}\nat: {loc}\n{:#?}\n",
    std::backtrace::Backtrace::force_capture()  // 完整堆栈 + 路径
);
```

**影响**:
- 堆栈包含完整文件路径（暴露用户真实姓名、项目名）
- 如果 `crash.log` 被恶意扩展读取，泄露系统结构

**修复建议**:
```rust
let loc = info.location()
    .map(|l| {
        let file = l.file().replace(&home, "~");  // 去除 HOME 前缀
        format!("{}:{}:{}", file, l.line(), l.column())
    })
    .unwrap_or_default();
```

---

### 8. macOS 私有 API 使用

**位置**: `src-tauri/tauri.conf.json:12`

**问题**:
```json
"macOSPrivateApi": true
```

**影响**:
- 可能绕过沙箱限制
- App Store 审核会拒绝
- 未来 macOS 更新可能破坏功能

**修复建议**:
- 除非确实需要，改为 `false`
- 如果需要，在 README 说明原因

---

## ✅ 做得好的地方

1. **路径校验** — `files.rs` 有 `require_inside_workspace()` 防护
2. **原子写入** — 用临时文件 + 硬链接避免部分写入
3. **进程清理** — `cleanup_stale()` 防止僵尸进程
4. **CSP 启用** — 虽然有 blob: 漏洞，但总比没有好

---

## 📋 修复优先级

| 优先级 | 漏洞 | 工作量 | 影响 | 风险评分 |
|--------|------|--------|------|---------|
| **P0** | 路径穿越 (assetProtocol) | 5 分钟 | 整个文件系统可读 | 9.5/10 |
| **P1** | 凭据明文存储 | 1-2 天 | 密钥泄露 | 8.5/10 |
| **P1** | 命令注入 | 2-3 天 | RCE 风险 | 8.0/10 |
| **P2** | XSS (CSP blob:) | 30 分钟 | 窃取 cookie | 7.0/10 |
| **P2** | DoS (文件大小) | 1 小时 | 崩溃 | 6.5/10 |
| **P3** | TOCTOU 竞态 | 1 天 | 数据损坏 | 5.5/10 |
| **P3** | 信息泄露 (crash.log) | 30 分钟 | 隐私泄露 | 4.0/10 |
| **P3** | macOS 私有 API | 5 分钟 | 审核拒绝 | 3.0/10 |

---

## 🔧 快速修复（30 分钟内可完成）

### 立即修改 `src-tauri/tauri.conf.json`:

```diff
  "security": {
-   "assetProtocol": { "enable": true, "scope": ["**", "/**", "$HOME/**"] },
+   "assetProtocol": { "enable": true, "scope": ["$RESOURCE/**", "$APPDATA/**", "$TEMP/**"] },
-   "csp": "default-src 'self'; script-src 'self' blob:; ..."
+   "csp": "default-src 'self'; script-src 'self'; worker-src 'self' blob:; ..."
  },
  "app": {
-   "macOSPrivateApi": true,
+   "macOSPrivateApi": false,
```

### 在 `files.rs` 开头加限制:

```rust
const MAX_DATA_URL_SIZE: u64 = 10 * 1024 * 1024;

#[tauri::command]
pub fn read_file_data_url(path: String) -> Result<String, String> {
    let resolved = require_inside_workspace(&path)?;
    let meta = std::fs::metadata(&resolved).map_err(|e| e.to_string())?;
+   if meta.len() > MAX_DATA_URL_SIZE {
+       return Err("file too large for data URL (> 10 MB)".into());
+   }
    let bytes = std::fs::read(&resolved).map_err(...)?;
    ...
}
```

---

## 📚 深度修复建议（需要架构调整）

### 1. 使用 `libgit2` 代替 shell 调用

```toml
[dependencies]
git2 = "0.18"
```

```rust
use git2::Repository;

pub fn git_clone_safe(url: &str, path: &Path) -> Result<(), String> {
    // 不会有命令注入风险
    Repository::clone(url, path).map_err(|e| e.to_string())?;
    Ok(())
}
```

### 2. 集成系统密钥存储

```toml
[dependencies]
keyring = "2.0"  # 跨平台密钥存储
```

```rust
use keyring::Entry;

pub fn save_api_key(key: &str) -> Result<(), String> {
    let entry = Entry::new("Michael IDE", "ai_api_key")
        .map_err(|e| e.to_string())?;
    entry.set_password(key).map_err(|e| e.to_string())
}
```

---

## 🧪 安全测试清单

- [ ] 尝试读取 `/etc/passwd` 通过 asset protocol
- [ ] 尝试注入 `; rm -rf /` 到 Git 命令
- [ ] 打开 10GB 文件看是否崩溃
- [ ] 检查 `~/.michael-ide/` 目录权限
- [ ] 运行 `cargo audit` 检查依赖漏洞
- [ ] 运行 `cargo clippy -- -W clippy::all` 抓代码问题

---

## 📖 参考资料

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Tauri Security Best Practices](https://tauri.app/v1/references/security/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [CWE-22: Path Traversal](https://cwe.mitre.org/data/definitions/22.html)
- [CWE-78: Command Injection](https://cwe.mitre.org/data/definitions/78.html)

---

## 联系信息

如有疑问，联系安全团队或提交 issue。
