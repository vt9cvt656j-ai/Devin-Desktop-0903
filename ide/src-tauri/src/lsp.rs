use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Stdio};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use crate::process_util;

/// 送到前端的事件。
///
/// `Error` 这一支以前是**声明了从来没构造过**的死变体（`#[allow(dead_code)]` 就是证据）：
/// 语言服务器的 stderr 只进了 `tracing::debug!`，默认日志级别下谁都看不见。于是
/// jdtls 因为找不到 JDK 起来就退、pyright 因为 node 版本太老崩掉，用户看到的都是
/// 一句光秃秃的「已停止」，真正的原因被丢掉了。现在 stderr 走这一支上来，
/// `Stopped` 也带上进程死前的最后几行。allow(dead_code) 一并撤掉——让编译器替我们
/// 盯着这个变体别再变回死代码。
#[derive(Serialize, Clone)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LspEvent {
    Message { data: String },
    Started { lang: String },
    Error { message: String },
    Stopped { lang: String, tail: Vec<String> },
}

struct LspProcess {
    child: Child,
    stdin_tx: std::sync::mpsc::Sender<String>,
}

// LSP servers (rust-analyzer/gopls/tsserver…) are heavy — hundreds of MB each.
// Killing on drop ensures clearing the map (reload / exit) reaps them and their
// reader threads (which exit on EOF) instead of leaving them resident forever.
impl Drop for LspProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// map 是 `Arc` 的，因为**读线程要能把自己摘掉**。
///
/// 原来读循环遇到一个非法帧就 break，然后发一个 Stopped 事件走人——子进程既不 kill、
/// 也不从这个 map 里移除。于是用户再打开一个同语言文件：ensureServer → lsp_start →
/// prune_stopped 看见进程还活着就保留 → `contains_key` 命中 → 返回
/// 「LSP for 'x' is already running」→ 前端那条 alreadyRunning 分支**静默 return**。
/// 结果整个会话里这门语言再也没有补全/诊断/跳转，界面上一个字都没有；而那个
/// 几百 MB 的语言服务器进程一直挂着，还没人读它的 stdout，管道写满后它自己也卡死。
#[derive(Default)]
pub struct LspManager {
    inner: Arc<Mutex<HashMap<String, LspProcess>>>,
}

use std::sync::Arc;

/// 把一门语言从 map 里摘掉并结束它的进程。
///
/// **先移除、再在锁外 drop**：LspProcess::drop 会 kill + 阻塞 wait，抱着锁做这件事会
/// 让编辑器的所有诊断/跳转请求排在它后面（lsp_stop 那边早就是这么写的，这里跟上）。
fn reap(map: &Arc<Mutex<HashMap<String, LspProcess>>>, lang: &str) {
    let removed = match map.lock() {
        Ok(mut inner) => inner.remove(lang),
        Err(_) => None,
    };
    drop(removed);
}

impl LspManager {
    /// Kill every language server and clear the map — reaps a previous page
    /// session on webview reload and on app exit.
    pub fn stop_all(&self) {
        // Reap on a DETACHED thread: each LspProcess::drop does kill()+blocking wait(),
        // and heavy servers (rust-analyzer/gopls, hundreds of MB) reaped serially stalled
        // the caller for seconds. This runs from cleanup_stale on boot (which used to
        // freeze the window) and on app exit; draining + dropping off-thread returns at once.
        let drained: Vec<LspProcess> = match self.inner.lock() {
            Ok(mut inner) => inner.drain().map(|(_, v)| v).collect(),
            Err(_) => return,
        };
        if !drained.is_empty() {
            std::thread::spawn(move || drop(drained));
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct LspServerConfig {
    pub lang: String,
    pub command: String,
    pub args: Vec<String>,
    pub root_uri: String,
    /// 是否允许使用**工作区提供的**语言服务器二进制（`node_modules/.bin`、`.venv/bin`）。
    ///
    /// 用项目自己的 TypeScript / pyright 版本是真实且必要的功能，但它同时意味着：打开
    /// 一个仓库里的 .ts 文件，就会执行那个仓库自带的可执行文件。所以这条能力跟着工作区
    /// 信任走 —— 未信任的工作区只用系统安装的语言服务器，功能降级但不执行仓库的东西。
    ///
    /// 缺省 `false`：老客户端/未传该字段时按不信任处理（fail closed）。
    #[serde(default)]
    pub trust_workspace_binaries: bool,
}

const KNOWN_SERVERS: &[(&str, &str, &[&str])] = &[
    ("typescript", "typescript-language-server", &["--stdio"]),
    ("javascript", "typescript-language-server", &["--stdio"]),
    ("rust", "rust-analyzer", &[]),
    ("python", "pyright-langserver", &["--stdio"]),
    ("go", "gopls", &["serve"]),
    ("c", "clangd", &[]),
    ("cpp", "clangd", &[]),
    ("objective-c", "clangd", &[]),
    ("html", "vscode-html-language-server", &["--stdio"]),
    ("css", "vscode-css-language-server", &["--stdio"]),
    ("json", "vscode-json-language-server", &["--stdio"]),
    // Extended language coverage. Each only activates when its server binary is
    // installed; otherwise the UI offers a one-click install hint.
    ("java", "jdtls", &[]),
    ("ruby", "solargraph", &["stdio"]),
    ("php", "intelephense", &["--stdio"]),
    ("lua", "lua-language-server", &[]),
    ("shell", "bash-language-server", &["start"]),
    ("yaml", "yaml-language-server", &["--stdio"]),
    ("csharp", "omnisharp", &["-lsp"]),
    ("kotlin", "kotlin-language-server", &[]),
    ("swift", "sourcekit-lsp", &[]),
    ("dart", "dart", &["language-server", "--protocol=lsp"]),
    ("elixir", "elixir-ls", &[]),
    ("clojure", "clojure-lsp", &[]),
    ("scala", "metals", &[]),
    ("hcl", "terraform-ls", &["serve"]),
    ("graphql", "graphql-lsp", &["server", "-m", "stream"]),
    ("dockerfile", "docker-langserver", &["--stdio"]),
    ("vue", "vue-language-server", &["--stdio"]),
];

fn find_server(lang: &str) -> Option<(&'static str, &'static [&'static str])> {
    KNOWN_SERVERS
        .iter()
        .find(|(l, _, _)| *l == lang)
        .map(|(_, cmd, args)| (*cmd, *args))
}

fn prune_stopped(inner: &mut HashMap<String, LspProcess>) {
    inner.retain(|_, proc| match proc.child.try_wait() {
        Ok(Some(_)) => false,
        Ok(None) => true,
        Err(_) => false,
    });
}

fn extract_method(json: &str) -> String {
    if let Some(start) = json.find("\"method\"") {
        let rest = &json[start + 9..];
        if let Some(q1) = rest.find('"') {
            let inner = &rest[q1 + 1..];
            if let Some(q2) = inner.find('"') {
                return inner[..q2].to_string();
            }
        }
    }
    if json.contains("\"result\"") {
        return "response".to_string();
    }
    "?".to_string()
}

fn encode_lsp_message(content: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", content.len(), content)
}

/// 把 `file://` 根 URI 还原成真实路径。
///
/// Windows 上这里以前是**每一个语言服务器都起不来**的原因，而且和"装没装"毫无关系：
/// 前端给的是 monaco 的 `monaco.Uri.file(root).toString()`，形如
/// `file:///c%3A/Users/me/proj`。
/// 剥掉 `file://`、百分号解码之后得到 `/c:/Users/me/proj` —— **多一个前导斜杠**，不是合法
/// 的 Windows 路径。它随后被交给 `current_dir()`，spawn 当场失败。于是：只要打开了工作区
/// 文件夹，补全、跳转定义、悬停、诊断全部消失；反而"不打开文件夹、单独开一个文件"时是好的
/// ——因为那条路不设 current_dir。
///
/// 修法是按 URI 规范来：`file:///C:/x` 里第三个斜杠是**路径的起始分隔符**，对 Windows 这种
/// 带盘符的路径要去掉它。顺带把正斜杠换成反斜杠（Windows API 两种都收，但拼进错误信息和
/// 日志时反斜杠才是用户认得的样子）。
fn workspace_dir_from_uri(uri: &str) -> Option<String> {
    let trimmed = uri.strip_prefix("file://").unwrap_or(uri);
    if trimmed.is_empty() {
        return None;
    }
    let decoded = percent_decode(trimmed);
    if decoded.is_empty() {
        return None;
    }
    Some(normalize_uri_path(&decoded))
}

/// `/C:/x` → `C:\x`（Windows）；其余平台原样返回。
///
/// 判据是"斜杠后面跟着 <单字母><冒号>"，不是"当前编译目标是 Windows"——一台 mac 上如果
/// 真收到这种 URI，那也是一个 Windows 路径，按 Windows 还原才对。这样这段逻辑在 mac 上
/// 也能测，不必靠交叉编译。
fn normalize_uri_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let drive_letter_at_start = bytes.len() >= 3
        && bytes[0] == b'/'
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':';
    if !drive_letter_at_start {
        return path.to_string();
    }
    path[1..].replace('/', "\\")
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| input.to_string())
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[tauri::command(async)]
pub fn lsp_start(
    state: State<LspManager>,
    config: LspServerConfig,
    on_event: Channel<LspEvent>,
) -> Result<(), String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    prune_stopped(&mut inner);

    if inner.contains_key(&config.lang) {
        return Err(format!("LSP for '{}' is already running", config.lang));
    }
    if inner.len() >= process_util::MAX_CHILD_PROCESSES {
        return Err("too many language servers running; stop one first".into());
    }

    let command = if config.command.is_empty() {
        let (cmd, _) = find_server(&config.lang).ok_or_else(|| {
            format!(
                "no known LSP server for '{}'; provide a custom command",
                config.lang
            )
        })?;
        cmd.to_string()
    } else {
        config.command.clone()
    };
    let args: Vec<String> = if config.command.is_empty() {
        let (_, default_args) = find_server(&config.lang).ok_or_else(|| {
            format!(
                "no known LSP server for '{}'; provide a custom command",
                config.lang
            )
        })?;
        default_args.iter().map(|arg| (*arg).to_string()).collect()
    } else {
        config.args.clone()
    };

    let ws = workspace_dir_from_uri(&config.root_uri);
    // 未信任的工作区：解析时不把工作区目录算进 PATH，于是仓库自带的
    // `node_modules/.bin/typescript-language-server` 不会被选中，只会用系统安装的那个。
    let bin_scope = if config.trust_workspace_binaries {
        ws.as_deref()
    } else {
        None
    };
    // 两个平台统一走 resolve_command。
    //
    // Windows 分支原来是 `command.clone()`——裸名字直接交给 spawn。可 Rust 的 Command 在
    // Windows 上走 CreateProcessW，**只补 .exe，不查 PATHEXT**（process_util 里那段注释
    // 自己写着）。而 npm 装出来的语言服务器全是 *.cmd：typescript-language-server、
    // bash-language-server、yaml-language-server、docker-langserver、vue-language-server、
    // intelephense、graphql-lsp —— 一个都起不来。
    //
    // 更难查的是它的表现：lsp_check_available 的 Windows 分支**会**扫 .exe/.cmd/.bat，
    // 于是返回「装了」，前端走的是「启动失败」而不是「去装一个」。用户看到的是
    // 「明明装好了，却没有补全、没有跳转，也没人告诉我为什么」。
    // mcp.rs 早就为同一个坑打过补丁并写明后果，lsp.rs 没跟上。
    let resolved = process_util::resolve_command(&command, bin_scope);

    // Detect Node.js shebang scripts and run them through node directly,
    // because the kernel's shebang handler uses the parent process PATH
    // which is minimal when launched from macOS Finder.
    #[cfg(not(windows))]
    let (actual_cmd, extra_args) = {
        if let Ok(content) = std::fs::read_to_string(&resolved) {
            if content.starts_with("#!/usr/bin/env node")
                || content.starts_with("#!/usr/bin/env -S node")
            {
                let node = process_util::resolve_command("node", bin_scope);
                (node, vec![resolved.clone()])
            } else {
                (resolved.clone(), vec![])
            }
        } else {
            (resolved.clone(), vec![])
        }
    };
    #[cfg(windows)]
    let (actual_cmd, extra_args) = (resolved.clone(), Vec::<String>::new());

    let mut builder = crate::process_util::command(&actual_cmd);
    builder
        .args(&extra_args)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // PATH 两个平台都要设。这里以前整段关在 cfg(not(windows)) 里，理由大概是"Windows 会
    // 自己按 PATH 找"——可子进程继承的是**这个进程**的 PATH，而 GUI 启动的应用拿到的那份
    // 很窄：nvm / volta / 用户级 npm 前缀全不在里面。于是语言服务器就算被拉起来了，它自己
    // 再去找 node 也找不到，表现同样是"装了却没有补全"。
    //
    // 未信任的工作区不把它的目录放进 PATH：否则语言服务器再去 PATH 里找工具时会命中仓库
    // 自带的版本——那是 clone 下来的别人的可执行文件。
    if let Some(ref ws_dir) = ws {
        builder.current_dir(ws_dir);
        builder.env(
            "PATH",
            process_util::augmented_path(config.trust_workspace_binaries.then_some(ws_dir)),
        );
    } else {
        builder.env("PATH", process_util::augmented_path(None));
    }
    tracing::info!(
        "[lsp] spawning: cmd={actual_cmd:?} extra={extra_args:?} args={args:?} resolved={resolved:?}"
    );
    let mut child = builder.spawn().map_err(|e| {
        format!(
            "failed to start '{}' (resolved={}, actual={}): {}",
            command, resolved, actual_cmd, e
        )
    })?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    // 进程死前的最后几行 stderr —— 「java 停了」和「java 停了：找不到 JDK」的差别。
    const STDERR_TAIL_LINES: usize = 12;
    let stderr_tail = std::sync::Arc::new(Mutex::new(std::collections::VecDeque::<String>::new()));
    let tail_for_stop = std::sync::Arc::clone(&stderr_tail);

    let (stdin_tx, stdin_rx) = std::sync::mpsc::channel::<String>();

    let mut stdin_handle = child.stdin.take().ok_or("no stdin")?;
    let send_lang = config.lang.clone();
    std::thread::spawn(move || {
        while let Ok(msg) = stdin_rx.recv() {
            let method = extract_method(&msg);
            tracing::debug!("[lsp-{send_lang}] → {method}");
            let encoded = encode_lsp_message(&msg);
            if stdin_handle.write_all(encoded.as_bytes()).is_err() {
                tracing::warn!("[lsp-{send_lang}] stdin write failed");
                break;
            }
            if stdin_handle.flush().is_err() {
                break;
            }
        }
    });

    let lang = config.lang.clone();
    let evt = on_event.clone();
    let reap_map = Arc::clone(&state.inner);
    let mut frame_error = String::new();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let body = match crate::content_length_frame::read_frame(&mut reader) {
                Ok(Some(body)) => body,
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!("[lsp-{lang}] invalid protocol frame: {error}");
                    // 非法帧不是「这门语言坏了」，是「这一条读不懂」——但从这里往后
                    // 流已经错位，只能收摊。把原因带出去，别让用户面对一句「已停止」。
                    frame_error = format!("协议帧读不懂：{error}（服务器往 stdout 写了非 LSP 内容，常见于包装脚本打印下载进度）");
                    break;
                }
            };
            let data = String::from_utf8_lossy(&body).to_string();
            let recv_method = extract_method(&data);
            tracing::debug!("[lsp-{lang}] ← {recv_method}");
            if evt.send(LspEvent::Message { data }).is_err() {
                tracing::warn!("[lsp-{lang}] channel send failed");
                break;
            }
        }
        // stdout EOF 是进程死掉的可靠信号，但 stderr 那边可能还没读完最后几行。
        // 等一小下再取尾巴——不等的话，最能说明死因的那几行恰好赶不上这班车。
        std::thread::sleep(std::time::Duration::from_millis(150));
        let mut tail = tail_for_stop
            .lock()
            .map(|t| t.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        if !frame_error.is_empty() {
            tail.push(frame_error);
        }
        // **必须摘掉自己**：不摘的话 map 里那条记录会永远挡住这门语言的下一次启动
        // （contains_key → "already running"），而进程本身还活着、还占着几百 MB。
        reap(&reap_map, &lang);
        let _ = evt.send(LspEvent::Stopped { lang, tail });
    });

    let lang2 = config.lang.clone();
    let evt_err = on_event.clone();
    let tail2 = std::sync::Arc::clone(&stderr_tail);
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        // 转发有上限：rust-analyzer / metals 这类会往 stderr 刷进度，全转会把前端的
        // 日志缓冲冲垮。前 STDERR_FORWARD_MAX 行全转（启动失败必在这一段），之后只转
        // 看着像错误的那些。环形缓冲不受上限影响——它要保证"死前最后几行"永远是最新的。
        const STDERR_FORWARD_MAX: usize = 200;
        let mut forwarded = 0usize;
        for line in reader.lines() {
            let l = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            tracing::debug!("[lsp-{}] {}", lang2, l);
            if let Ok(mut t) = tail2.lock() {
                t.push_back(l.clone());
                while t.len() > STDERR_TAIL_LINES {
                    t.pop_front();
                }
            }
            let looks_bad = {
                let low = l.to_ascii_lowercase();
                low.contains("error")
                    || low.contains("exception")
                    || low.contains("fatal")
                    || low.contains("panic")
                    || low.contains("not found")
                    || low.contains("cannot find")
                    || low.contains("traceback")
            };
            if forwarded < STDERR_FORWARD_MAX || looks_bad {
                forwarded += 1;
                if evt_err
                    .send(LspEvent::Error {
                        message: format!("[stderr] {}", l.chars().take(600).collect::<String>()),
                    })
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    let _ = on_event.send(LspEvent::Started {
        lang: config.lang.clone(),
    });

    inner.insert(config.lang, LspProcess { child, stdin_tx });
    Ok(())
}

#[tauri::command(async)]
pub fn lsp_send(state: State<LspManager>, lang: String, message: String) -> Result<(), String> {
    let inner = state.inner.lock().map_err(|e| e.to_string())?;
    let proc = inner
        .get(&lang)
        .ok_or_else(|| format!("no LSP for '{lang}'"))?;
    proc.stdin_tx
        .send(message)
        .map_err(|e| format!("failed to send to LSP: {e}"))
}

#[tauri::command(async)]
pub fn lsp_stop(state: State<LspManager>, lang: String) -> Result<(), String> {
    let removed = {
        let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
        inner.remove(&lang)
    };
    if let Some(mut proc) = removed {
        let _ = proc.child.kill();
        // LspProcess::drop performs a blocking wait; release the manager lock
        // first so diagnostics and editor requests are not serialized behind it.
        drop(proc);
    }
    Ok(())
}

#[tauri::command(async)]
/// 「这台机器上有没有这门语言的服务器」。
///
/// 必须和 `lsp_start` **用同一个作用域**去找。原来这里恒定传 None，而 lsp_start 在
/// 工作区受信任时会把工作区目录算进 PATH——于是装在项目 `node_modules/.bin` 或
/// `.venv/bin` 里的语言服务器，启动得起来，这里却一律判「没装」。
///
/// 后果是那张安装进度卡**必定超时**：它每 2.5 秒问一次这个函数「装好了没」，
/// 用户 `npm i -D typescript-language-server` 明明装成功了，卡片还是转满 90 秒然后
/// 告诉他「安装超时」。缺省仍然 fail closed（不传 = 不信任 = 只看系统装的那份）。
pub fn lsp_check_available(
    lang: String,
    workspace: Option<String>,
    trust_workspace_binaries: Option<bool>,
) -> bool {
    let (cmd, _) = match find_server(&lang) {
        Some(pair) => pair,
        None => return false,
    };
    let ws = workspace.filter(|w| !w.is_empty());
    let scope = if workspace_trusted(trust_workspace_binaries) {
        ws.as_deref()
    } else {
        None
    };
    #[cfg(not(windows))]
    {
        let resolved = process_util::resolve_command(cmd, scope);
        resolved != cmd || std::path::Path::new(cmd).exists()
    }
    #[cfg(windows)]
    {
        if std::path::Path::new(cmd).exists() {
            return true;
        }
        let path = process_util::augmented_path(scope);
        for dir in path.split(';').filter(|d| !d.is_empty()) {
            for ext in ["", ".exe", ".cmd", ".bat"] {
                if std::path::Path::new(&format!("{dir}\\{cmd}{ext}")).exists() {
                    return true;
                }
            }
        }
        false
    }
}

/// 工作区信任门在这些"取环境符号"的路上一直是**缺的**。
///
/// `lsp_start` 那边把它写得很仔细（未信任就不把工作区目录算进 PATH，只用系统装的语言
/// 服务器），可同一个进程里另外四条路绕过了它：`lsp_detect_python` 会直接执行
/// `<工作区>/.venv/bin/python`，`lsp_node_env_symbols` 会 `require()` 工作区
/// `node_modules` 里的包（等于跑它的顶层代码）。也就是说：clone 一个别人的仓库、
/// 点开任意一个 .py 或 .ts 文件，仓库自带的可执行文件就跑起来了。
///
/// 缺省 fail closed —— 参数是 `Option<bool>`，老客户端不传就按不信任处理。
/// 不信任不等于功能没了，是降级：Python 用系统解释器（pyright 照常工作，只是看不到
/// venv 里装的包），Node 照常列包名（读目录名是安全的）但不去 require 它们。
fn workspace_trusted(flag: Option<bool>) -> bool {
    flag.unwrap_or(false)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonEnvInfo {
    pub python_path: String,
    pub site_packages: Vec<String>,
    /// 工作区未信任 → 用的是系统解释器而不是仓库的 venv。前端据此告诉用户
    /// 「venv 里的包看不到，信任这个工作区就能看到」，而不是让他对着
    /// 「import X could not be resolved」发呆。
    pub untrusted_fallback: bool,
}

/// Pick the interpreter to introspect: the project's venv python if one exists (so pyright resolves
/// the packages the user installed THERE — requests, debugpy, …), else the resolved system python3.
/// Detecting only system python was why "import X could not be resolved" persisted across reopens
/// even though the venv had X.
fn pick_python(workspace: Option<&str>) -> String {
    if let Some(ws) = workspace.filter(|w| !w.is_empty()) {
        // venv 的布局两个平台不一样：POSIX 是 <venv>/bin/python，Windows 是
        // <venv>\Scripts\python.exe。这里以前只列了 POSIX 那四条，于是 Windows 上
        // **一条都命中不了**，pyright 拿不到 venv 解释器——项目里 pip 装过的每一个包，
        // import 那一行全是红波浪线，而 IDE 一句错都不报。
        for rel in VENV_PYTHON_RELATIVE_PATHS {
            let p = format!("{ws}/{rel}");
            if std::path::Path::new(&p).exists() {
                return p;
            }
        }
    }
    // 兜底也不能写死 python3：Windows 上 python.org 的安装包只产出 python.exe，
    // 而同名的 python3.exe 是微软商店的「应用执行别名」——跑它会弹出商店页面。
    for name in DEFAULT_PYTHON_NAMES {
        let resolved = process_util::resolve_command(name, workspace);
        if resolved != *name {
            return resolved;
        }
    }
    process_util::resolve_command(DEFAULT_PYTHON_NAMES[0], workspace)
}

/// venv 里解释器的相对位置，按平台。
#[cfg(windows)]
const VENV_PYTHON_RELATIVE_PATHS: &[&str] = &[
    ".venv/Scripts/python.exe",
    "venv/Scripts/python.exe",
];
#[cfg(not(windows))]
const VENV_PYTHON_RELATIVE_PATHS: &[&str] = &[
    ".venv/bin/python3",
    ".venv/bin/python",
    "venv/bin/python3",
    "venv/bin/python",
];

/// 没有 venv 时按什么名字找解释器。Windows 上 `python3` 通常不存在（存在的那个多半是
/// 微软商店的别名），所以 `python` 排在前面。
#[cfg(windows)]
const DEFAULT_PYTHON_NAMES: &[&str] = &["python", "py"];
#[cfg(not(windows))]
const DEFAULT_PYTHON_NAMES: &[&str] = &["python3", "python"];

#[tauri::command(async)]
pub fn lsp_detect_python(
    workspace: Option<String>,
    trust_workspace_binaries: Option<bool>,
) -> Result<PythonEnvInfo, String> {
    let trusted = workspace_trusted(trust_workspace_binaries);
    let ws = workspace.as_deref();
    // 不信任就整个不看工作区：既不挑它的 venv 解释器，也不把它算进 PATH。
    let scope = if trusted { ws } else { None };
    let python = pick_python(scope);
    let aug_path = process_util::augmented_path(scope);
    let untrusted_fallback = !trusted && ws.is_some_and(|w| !w.is_empty());
    let output = crate::process_util::command(&python)
        .args(["-c", "import sys,site,json;p=list(site.getsitepackages());p.append(site.getusersitepackages());print(json.dumps({'exec':sys.executable,'paths':p}))"])
        .env("PATH", &aug_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("failed to run python3: {e}"))?;
    if !output.status.success() {
        return Ok(PythonEnvInfo {
            python_path: python.to_string(),
            site_packages: vec![],
            untrusted_fallback,
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("failed to parse python output: {e}"))?;
    let exec_path = parsed["exec"].as_str().unwrap_or(&python).to_string();
    let paths: Vec<String> = parsed["paths"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(PythonEnvInfo {
        python_path: exec_path,
        site_packages: paths,
        untrusted_fallback,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonModuleSymbols {
    pub modules: Vec<String>,
    pub symbols: HashMap<String, Vec<String>>,
    pub cached: bool,
}

use std::sync::OnceLock;
use std::time::Instant;

struct PythonModuleCache {
    modules: Vec<String>,
    fetched_at: Instant,
    symbol_cache: HashMap<String, Vec<String>>,
}

static PY_CACHE: OnceLock<Mutex<Option<PythonModuleCache>>> = OnceLock::new();

fn py_cache() -> &'static Mutex<Option<PythonModuleCache>> {
    PY_CACHE.get_or_init(|| Mutex::new(None))
}

/// `scope` 是**已经过信任门**的工作区目录（不信任就传 None）。
///
/// 原来这里恒定 `pick_python(None)`，跑的是系统 python3；而 pyright 拿到的是
/// `lsp_detect_python` 用工作区 venv 挑出来的那个解释器。于是同一个编辑器里出现两套
/// 互相矛盾的「这个包存不存在」：项目 venv 里 pip 装的 requests / pandas，pyright 的
/// 诊断认得，而模块名补全和 `import requests` 之后的属性补全一片空白——因为
/// `pkgutil.iter_modules()` 是在系统解释器下跑的。
fn run_python_script(script: &str, extra_args: &[&str], scope: Option<&str>) -> Option<String> {
    // 走和 pick_python 同一套按平台的名字，别在这里再写死一个 python3。
    let python = pick_python(scope);
    let aug_path = process_util::augmented_path(scope);
    let mut cmd = crate::process_util::command(&python);
    cmd.args(["-c", script])
        .env("PATH", &aug_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for a in extra_args {
        cmd.arg(a);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command(async)]
pub fn lsp_python_env_symbols(
    modules: Vec<String>,
    workspace: Option<String>,
    trust_workspace_binaries: Option<bool>,
) -> Result<PythonModuleSymbols, String> {
    let ws = workspace.filter(|w| !w.is_empty());
    let scope = if workspace_trusted(trust_workspace_binaries) {
        ws.as_deref()
    } else {
        None
    };
    let mut guard = py_cache().lock().map_err(|e| e.to_string())?;
    let now = Instant::now();

    let cache_valid = guard
        .as_ref()
        .is_some_and(|c| now.duration_since(c.fetched_at).as_secs() < 300);

    let all_modules = if cache_valid {
        guard.as_ref().unwrap().modules.clone()
    } else {
        let script = "import json,pkgutil;print(json.dumps(sorted(set(m.name for m in pkgutil.iter_modules() if not m.name.startswith('_')))))";
        let mods: Vec<String> = run_python_script(script, &[], scope)
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let c = guard.get_or_insert_with(|| PythonModuleCache {
            modules: vec![],
            fetched_at: now,
            symbol_cache: HashMap::new(),
        });
        // **失败不落缓存**。脚本跑挂（解释器不在、venv 刚建还没装好）时 mods 是空的，
        // 原来照样写 fetched_at —— 于是一张空模块表被当成有效缓存钉住 300 秒，
        // 这五分钟里补全一个模块名都给不出，而且没有任何迹象说明为什么。
        if !mods.is_empty() {
            c.modules = mods.clone();
            c.fetched_at = now;
        }
        mods
    };

    let mut need_fetch: Vec<String> = Vec::new();
    let mut symbols: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(ref c) = *guard {
        for m in &modules {
            if let Some(cached) = c.symbol_cache.get(m) {
                symbols.insert(m.clone(), cached.clone());
            } else {
                need_fetch.push(m.clone());
            }
        }
    } else {
        need_fetch = modules.clone();
    }

    if !need_fetch.is_empty() {
        let script = r#"import json,sys,importlib
r={}
for n in sys.argv[1:]:
 try:
  m=importlib.import_module(n);r[n]=[a for a in dir(m) if not a.startswith('_')][:500]
 except: pass
print(json.dumps(r))"#;
        let args: Vec<&str> = need_fetch.iter().map(|s| s.as_str()).collect();
        if let Some(out) = run_python_script(script, &args, scope) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&out) {
                if let Some(obj) = parsed.as_object() {
                    let c = guard.get_or_insert_with(|| PythonModuleCache {
                        modules: vec![],
                        fetched_at: now,
                        symbol_cache: HashMap::new(),
                    });
                    for (k, v) in obj {
                        if let Some(arr) = v.as_array() {
                            let syms: Vec<String> = arr
                                .iter()
                                .filter_map(|s| s.as_str().map(String::from))
                                .collect();
                            c.symbol_cache.insert(k.clone(), syms.clone());
                            symbols.insert(k.clone(), syms);
                        }
                    }
                }
            }
        }
    }

    Ok(PythonModuleSymbols {
        modules: all_modules,
        symbols,
        cached: cache_valid,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeEnvSymbols {
    pub packages: Vec<String>,
    pub exports: HashMap<String, Vec<String>>,
}

#[tauri::command(async)]
pub fn lsp_node_env_symbols(
    project_dir: String,
    modules: Vec<String>,
    trust_workspace_binaries: Option<bool>,
) -> Result<NodeEnvSymbols, String> {
    let trusted = workspace_trusted(trust_workspace_binaries);
    let node = process_util::resolve_command("node", None);
    let aug_path = process_util::augmented_path(None);

    let mut packages = Vec::new();
    let pkg_path = std::path::Path::new(&project_dir).join("package.json");
    if let Ok(content) = std::fs::read_to_string(&pkg_path) {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
            for section in &["dependencies", "devDependencies"] {
                if let Some(obj) = parsed[section].as_object() {
                    for k in obj.keys() {
                        if !k.starts_with('@') || k.contains('/') {
                            packages.push(k.clone());
                        }
                    }
                }
            }
        }
    }

    let node_mods = std::path::Path::new(&project_dir).join("node_modules");
    if packages.is_empty() && node_mods.exists() {
        if let Ok(entries) = std::fs::read_dir(&node_mods) {
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') && !name.starts_with('_') {
                    packages.push(name);
                }
            }
        }
    }

    let mut exports: HashMap<String, Vec<String>> = HashMap::new();
    // require(n) 会跑那个包的顶层代码。列包名（上面读目录）是安全的，require 不是。
    if !modules.is_empty() && trusted {
        let script = r#"const r={};for(const n of process.argv.slice(1)){try{const m=require(n);r[n]=Object.getOwnPropertyNames(m).filter(k=>!k.startsWith('_')).slice(0,500)}catch{}};console.log(JSON.stringify(r))"#.to_string();
        let mut cmd = crate::process_util::command(&node);
        cmd.args(["-e", &script])
            .env("PATH", &aug_path)
            .env("NODE_PATH", node_mods.to_str().unwrap_or(""))
            .current_dir(&project_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for m in &modules {
            cmd.arg(m);
        }
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
                    if let Some(obj) = parsed.as_object() {
                        for (k, v) in obj {
                            if let Some(arr) = v.as_array() {
                                exports.insert(
                                    k.clone(),
                                    arr.iter()
                                        .filter_map(|s| s.as_str().map(String::from))
                                        .collect(),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(NodeEnvSymbols { packages, exports })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoEnvSymbols {
    pub packages: Vec<String>,
}

#[tauri::command(async)]
pub fn lsp_go_env_symbols(
    project_dir: String,
    trust_workspace_binaries: Option<bool>,
) -> Result<GoEnvSymbols, String> {
    // go.mod 里的 `toolchain` 指令会让 go 去下载并执行另一个工具链——在别人的仓库里
    // 跑 `go list` 不是纯读操作。
    if !workspace_trusted(trust_workspace_binaries) {
        return Ok(GoEnvSymbols { packages: vec![] });
    }
    let go_cmd = process_util::resolve_command("go", None);
    let aug_path = process_util::augmented_path(None);

    let output = crate::process_util::command(&go_cmd)
        .args(["list", "-m", "all"])
        .env("PATH", &aug_path)
        .current_dir(&project_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();

    let mut packages = Vec::new();
    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if let Some(pkg) = parts.first() {
                    let name = pkg.rsplit('/').next().unwrap_or(pkg);
                    if !name.is_empty() && !name.starts_with('_') {
                        packages.push(name.to_string());
                    }
                }
            }
        }
    }

    let output2 = crate::process_util::command(&go_cmd)
        .args(["list", "std"])
        .env("PATH", &aug_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    if let Ok(out) = output2 {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let name = line.rsplit('/').next().unwrap_or(line);
                if !name.is_empty() && !name.starts_with('_') && !name.contains("internal") {
                    packages.push(name.to_string());
                }
            }
        }
    }

    packages.sort();
    packages.dedup();
    Ok(GoEnvSymbols { packages })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LangEnvSymbols {
    pub symbols: Vec<String>,
    pub api_symbols: HashMap<String, Vec<String>>,
}

fn run_cmd_collect(cmd_name: &str, args: &[&str], cwd: Option<&str>) -> Vec<String> {
    let resolved = process_util::resolve_command(cmd_name, None);
    let aug_path = process_util::augmented_path(None);
    let mut cmd = crate::process_util::command(&resolved);
    cmd.args(args)
        .env("PATH", &aug_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .collect(),
        _ => vec![],
    }
}

#[tauri::command(async)]
pub fn lsp_lang_env_symbols(
    lang: String,
    project_dir: String,
    modules: Vec<String>,
    trust_workspace_binaries: Option<bool>,
) -> Result<LangEnvSymbols, String> {
    // 这里几条（dart pub deps 之类）是在**项目目录里**跑工具链的。解释器本身取的是系统
    // 那个（run_cmd_collect 用 None 作用域），但工作目录是别人的仓库。
    if !workspace_trusted(trust_workspace_binaries) {
        return Ok(LangEnvSymbols { symbols: vec![], api_symbols: HashMap::new() });
    }
    let mut symbols = Vec::new();
    let mut api_symbols: HashMap<String, Vec<String>> = HashMap::new();

    match lang.as_str() {
        "lua" => {
            let script = r#"
local r={}
for k,_ in pairs(package.loaded) do r[#r+1]=k end
for k,_ in pairs(_G) do if type(k)=="string" and not k:match("^_") then r[#r+1]=k end end
table.sort(r)
for _,v in ipairs(r) do print(v) end"#;
            symbols = run_cmd_collect("lua", &["-e", script.trim()], None);
            if symbols.is_empty() {
                symbols = run_cmd_collect("lua5.4", &["-e", script.trim()], None);
            }
            if symbols.is_empty() {
                symbols = run_cmd_collect("luajit", &["-e", script.trim()], None);
            }
            for m in &modules {
                let mod_script = format!(
                    "local ok,mod=pcall(require,'{}');if ok and type(mod)=='table' then for k,_ in pairs(mod) do if type(k)=='string' and not k:match('^_') then print(k) end end end",
                    m
                );
                let syms = run_cmd_collect("lua", &["-e", &mod_script], None);
                if !syms.is_empty() {
                    api_symbols.insert(m.clone(), syms);
                }
            }
        }
        "ruby" => {
            let lines = run_cmd_collect(
                "ruby",
                &["-e", "puts Gem::Specification.map(&:name).sort.uniq"],
                None,
            );
            symbols.extend(lines);
            let builtins = run_cmd_collect("ruby", &["-e", "puts Object.constants.sort"], None);
            symbols.extend(builtins);
            for m in &modules {
                let script = format!(
                    "begin;require '{}';m=Object.const_get('{}');puts m.instance_methods(false).sort rescue puts m.public_methods(false).sort;rescue=>e;end",
                    m, m.chars().next().unwrap_or('X').to_uppercase().to_string() + &m[1..]
                );
                let syms = run_cmd_collect("ruby", &["-e", &script], None);
                if !syms.is_empty() {
                    api_symbols.insert(m.clone(), syms);
                }
            }
        }
        "php" => {
            let exts = run_cmd_collect("php", &["-m"], None);
            symbols.extend(exts.iter().filter(|e| !e.starts_with('[')).cloned());
            let fns = run_cmd_collect(
                "php",
                &[
                    "-r",
                    "echo implode(\"\\n\",array_slice(get_defined_functions()['internal'],0,500));",
                ],
                None,
            );
            symbols.extend(fns);
        }
        "dart" => {
            let deps = run_cmd_collect(
                "dart",
                &["pub", "deps", "--style=compact"],
                Some(&project_dir),
            );
            for line in &deps {
                if let Some(name) = line.split_whitespace().next() {
                    if name.chars().next().is_some_and(|c| c.is_alphabetic()) {
                        symbols.push(name.to_string());
                    }
                }
            }
        }
        "kotlin" | "java" => {
            let script = r#"import java.util.jar.*;import java.io.*;public class _Ls{public static void main(String[] a){for(String p:System.getProperty("java.class.path","").split(File.pathSeparator)){try{JarFile j=new JarFile(p);j.stream().filter(e->e.getName().endsWith(".class")).forEach(e->{String n=e.getName().replace('/','.');n=n.substring(0,n.length()-6);String s=n.contains(".")?n.substring(n.lastIndexOf('.')+1):n;if(!s.isEmpty()&&!s.startsWith("_"))System.out.println(s);});j.close();}catch(Exception ex){}}}}"#;
            let _ = script;
            let common = vec![
                "String",
                "Integer",
                "Long",
                "Double",
                "Float",
                "Boolean",
                "Character",
                "Byte",
                "Short",
                "ArrayList",
                "LinkedList",
                "HashMap",
                "TreeMap",
                "HashSet",
                "TreeSet",
                "LinkedHashMap",
                "Collections",
                "Arrays",
                "Objects",
                "Optional",
                "Stream",
                "Collectors",
                "List",
                "Map",
                "Set",
                "Queue",
                "Deque",
                "Iterator",
                "Iterable",
                "Comparable",
                "Thread",
                "Runnable",
                "Callable",
                "Future",
                "CompletableFuture",
                "ExecutorService",
                "IOException",
                "Exception",
                "RuntimeException",
                "NullPointerException",
                "StringBuilder",
                "StringBuffer",
                "Scanner",
                "Random",
                "BigDecimal",
                "BigInteger",
                "File",
                "Path",
                "Paths",
                "Files",
                "InputStream",
                "OutputStream",
                "Reader",
                "Writer",
                "BufferedReader",
                "BufferedWriter",
                "FileReader",
                "FileWriter",
                "PrintWriter",
                "Socket",
                "ServerSocket",
                "URL",
                "URI",
                "HttpURLConnection",
                "Pattern",
                "Matcher",
                "DateTimeFormatter",
                "LocalDate",
                "LocalDateTime",
                "Instant",
                "System",
                "Math",
                "Class",
                "Object",
                "Enum",
                "Annotation",
                "Override",
                "Deprecated",
            ];
            symbols.extend(common.into_iter().map(String::from));
        }
        "swift" => {
            let common = vec![
                "String",
                "Int",
                "Double",
                "Float",
                "Bool",
                "Array",
                "Dictionary",
                "Set",
                "Optional",
                "print",
                "debugPrint",
                "fatalError",
                "precondition",
                "assert",
                "struct",
                "class",
                "enum",
                "protocol",
                "extension",
                "func",
                "var",
                "let",
                "guard",
                "UIView",
                "UIViewController",
                "UILabel",
                "UIButton",
                "UITableView",
                "UICollectionView",
                "URLSession",
                "URLRequest",
                "JSONDecoder",
                "JSONEncoder",
                "Codable",
                "Decodable",
                "Encodable",
                "DispatchQueue",
                "OperationQueue",
                "NotificationCenter",
                "UserDefaults",
                "Bundle",
                "CGFloat",
                "CGPoint",
                "CGSize",
                "CGRect",
                "NSObject",
                "NSError",
                "SwiftUI",
                "View",
                "Text",
                "Button",
                "NavigationView",
                "List",
                "VStack",
                "HStack",
                "ZStack",
                "State",
                "Binding",
                "ObservableObject",
                "Published",
                "EnvironmentObject",
            ];
            symbols.extend(common.into_iter().map(String::from));
        }
        _ => {}
    }

    symbols.sort();
    symbols.dedup();
    Ok(LangEnvSymbols {
        symbols,
        api_symbols,
    })
}

#[derive(Serialize)]
pub struct LspInfo {
    lang: String,
    running: bool,
}

#[tauri::command(async)]
pub fn lsp_list(state: State<LspManager>) -> Result<Vec<LspInfo>, String> {
    let mut inner = state.inner.lock().map_err(|e| e.to_string())?;
    prune_stopped(&mut inner);
    let known: Vec<&str> = KNOWN_SERVERS.iter().map(|(l, _, _)| *l).collect();
    let mut out: Vec<LspInfo> = known
        .into_iter()
        .map(|l| LspInfo {
            lang: l.to_string(),
            running: inner.contains_key(l),
        })
        .collect();
    for key in inner.keys() {
        if !out.iter().any(|i| i.lang == *key) {
            out.push(LspInfo {
                lang: key.clone(),
                running: true,
            });
        }
    }
    Ok(out)
}
#[cfg(test)]
mod windows_path_tests {
    use super::*;

    /// Windows 上「打开文件夹之后每个语言服务器都起不来」的根因。
    ///
    /// 前端给的是 monaco 的 `Uri.file(root).toString()`：`file:///c%3A/Users/me/proj`。
    /// 剥前缀 + 解码之后是 `/c:/Users/me/proj`——多一个前导斜杠，不是合法 Windows 路径，
    /// 交给 current_dir() 当场失败。反而"不开文件夹、单开一个文件"是好的，因为那条路
    /// 不设 current_dir——这也是为什么这个 bug 看起来像"时好时坏"。
    #[test]
    fn a_windows_file_uri_becomes_a_real_path() {
        assert_eq!(
            workspace_dir_from_uri("file:///c%3A/Users/me/proj").unwrap(),
            "c:\\Users\\me\\proj",
        );
        assert_eq!(
            workspace_dir_from_uri("file:///D%3A/work/%E9%A1%B9%E7%9B%AE").unwrap(),
            "D:\\work\\项目",
            "中文目录名解码之后也要按 Windows 还原",
        );
        // POSIX 路径一个字都不能动。
        assert_eq!(
            workspace_dir_from_uri("file:///Users/me/proj").unwrap(),
            "/Users/me/proj",
        );
        assert_eq!(
            workspace_dir_from_uri("file:///home/me/%E9%A1%B9%E7%9B%AE").unwrap(),
            "/home/me/项目",
        );
        assert_eq!(workspace_dir_from_uri("file://"), None);
    }

    /// 判据是"斜杠后面跟着 <单字母><冒号>"，不是编译目标——所以这段在 mac 上也测得了，
    /// 不必靠交叉编译（本仓库的交叉编译卡在 C 依赖上）。
    #[test]
    fn only_a_drive_letter_triggers_the_windows_rewrite() {
        assert_eq!(normalize_uri_path("/c:/x"), "c:\\x");
        assert_eq!(normalize_uri_path("/Z:/a/b"), "Z:\\a\\b");
        // 这些都不是盘符，原样留着。
        for keep in ["/usr/local/bin", "/ab:/x", "/1:/x", "/", "relative/path"] {
            assert_eq!(normalize_uri_path(keep), keep, "{keep} 被误改了");
        }
    }
}


/// 「取环境符号」那四条路的工作区信任门。
///
/// `lsp_start` 把它写得很仔细，可同一个进程里另外四条路绕过了它：`lsp_detect_python`
/// 会直接执行 `<工作区>/.venv/bin/python`，`lsp_node_env_symbols` 会 `require()`
/// 工作区 `node_modules` 里的包（等于跑它的顶层代码）。也就是说：clone 一个别人的
/// 仓库、点开任意一个 .py 或 .ts 文件，仓库自带的可执行文件就跑起来了。
///
/// 这两条测试是**真的去执行**：造一个会留下痕迹的假解释器/假包，然后断言那个痕迹
/// 不存在。断言"参数传对了"是守不住的——门漏没漏，只有让它真跑一次才知道。
#[cfg(test)]
mod trust_gate_tests {
    use super::*;

    /// 这几条测试会**真的起进程**（假解释器、node、被 kill 的 sleep）。测试运行器默认
    /// 并行，几条同时 spawn 在负载高时会波及别的用例——加进去当天就见过一次 archive
    /// 那三条莫名其妙一起红（单独跑、单线程跑都是绿的）。起进程的用例没必要并行，
    /// 用一把锁串起来；比留一个偶发的红更值。
    pub(super) static SPAWN_LOCK: Mutex<()> = Mutex::new(());
    pub(super) fn spawn_guard() -> std::sync::MutexGuard<'static, ()> {
        SPAWN_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn fake_repo_with_venv(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mrday-trustgate-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        let bin = dir.join(".venv").join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let py = bin.join("python3");
        std::fs::write(
            &py,
            format!(
                "#!/bin/sh\necho '{{\"exec\":\"{}\",\"paths\":[\"/pwned\"]}}'\n",
                py.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&py, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        dir
    }

    #[test]
    #[cfg(unix)]
    fn untrusted_workspace_never_runs_its_own_interpreter() {
        let _g = spawn_guard();
        let dir = fake_repo_with_venv("untrusted");
        let ws = dir.to_string_lossy().to_string();

        // 不信任，以及老客户端那种「压根不传这个参数」，都不许碰仓库里的解释器。
        for flag in [None, Some(false)] {
            let info = lsp_detect_python(Some(ws.clone()), flag).unwrap();
            assert!(
                !info.python_path.contains("mrday-trustgate"),
                "未信任的工作区里那个解释器被执行了：{}",
                info.python_path
            );
            assert!(
                !info.site_packages.iter().any(|p| p == "/pwned"),
                "输出来自仓库自带的解释器 —— 说明它真的跑了"
            );
            assert!(info.untrusted_fallback, "降级了却没报出来，前端无从告诉用户");
        }

        // 信任：功能照旧。这条不能被这次修改弄死。
        let info = lsp_detect_python(Some(ws.clone()), Some(true)).unwrap();
        assert!(
            info.python_path.contains("mrday-trustgate"),
            "信任的工作区反而用不上自己的 venv 了 —— 那是把功能修没了：{}",
            info.python_path
        );
        assert!(!info.untrusted_fallback, "信任时不该标降级");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn untrusted_workspace_never_requires_its_node_modules() {
        let _g = spawn_guard();
        let dir = std::env::temp_dir().join("mrday-trustgate-node");
        let _ = std::fs::remove_dir_all(&dir);
        let m = dir.join("node_modules").join("evil");
        std::fs::create_dir_all(&m).unwrap();
        std::fs::write(m.join("package.json"), "{\"name\":\"evil\",\"main\":\"index.js\"}").unwrap();
        // require 一旦发生顶层代码就跑了；用"写一个标记文件"留证据。
        let marker = dir.join("REQUIRED");
        std::fs::write(
            m.join("index.js"),
            format!(
                "require('fs').writeFileSync({:?}, 'x');\nmodule.exports = {{ a: 1 }};",
                marker.to_string_lossy()
            ),
        )
        .unwrap();
        let ws = dir.to_string_lossy().to_string();

        let out = lsp_node_env_symbols(ws.clone(), vec!["evil".into()], Some(false)).unwrap();
        assert!(
            !marker.exists(),
            "未信任的工作区里那个包被 require 了 —— 它的顶层代码跑了"
        );
        assert!(
            out.packages.iter().any(|p| p == "evil"),
            "把「列包名」也一起关掉了 —— 读目录名是安全的，不该跟着降级"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// 读线程退出时必须把自己从 map 里摘掉。
///
/// 不摘的后果是一条**静默**的死亡：进程还活着 → prune_stopped 保留它 → 下次
/// `contains_key` 命中 → lsp_start 返回「already running」→ 前端那条分支静默 return。
/// 整个会话里这门语言再也没有补全/诊断/跳转，界面上一个字都没有，而那个几百 MB 的
/// 进程一直挂着、没人读它的 stdout，管道写满后它自己也卡死。
#[cfg(test)]
mod reap_tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn reap_removes_the_entry_and_kills_the_child() {
        let _g = super::trust_gate_tests::spawn_guard();
        let map: Arc<Mutex<HashMap<String, LspProcess>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(Stdio::piped())
            .spawn()
            .expect("spawn sleep");
        let pid = child.id();
        // stdin_tx 只是个占位：reap 走的是 remove + drop，不碰它。
        let (tx, _rx) = std::sync::mpsc::channel::<String>();
        let _ = child.stdin.take();
        map.lock().unwrap().insert(
            "rust".to_string(),
            LspProcess {
                child,
                stdin_tx: tx,
            },
        );
        assert!(map.lock().unwrap().contains_key("rust"));

        reap(&map, "rust");

        assert!(
            !map.lock().unwrap().contains_key("rust"),
            "读线程退出后那条记录还在 —— 下次 lsp_start 会撞上 already running，这门语言整个会话再也起不来"
        );
        // 进程真的被结束了：kill -0 探测不到就是没了。
        std::thread::sleep(std::time::Duration::from_millis(120));
        let alive = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(!alive, "进程没被结束 —— 几百 MB 的语言服务器会一直挂到应用退出");
    }

    #[test]
    fn reap_on_a_missing_lang_is_a_no_op() {
        let map: Arc<Mutex<HashMap<String, LspProcess>>> = Arc::new(Mutex::new(HashMap::new()));
        reap(&map, "nope"); // 不许 panic
        assert!(map.lock().unwrap().is_empty());
    }
}
