use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDefinition {
    id: String,
    label: String,
    command: String,
    cwd: String,
    source: String,
    group: String,
    problem_matcher: Option<String>,
}

/// 超时被杀之后，给模型的那句建议。抽成独立函数只为一件事：能被测试直接打。
///
/// 判据**输出优先**。原来只按命令字符串猜（dev/serve/watch/start…），用户实拍的那次
/// 跑的是 `python3 main.py`，一个关键词都不沾，于是走进"这是会退出的命令"那一支——而
/// 那一支明确写着「不要改用 run_in_terminal」。模型照做了：反复重跑、查 sys.path、查
/// pip list，就是不换终端。而证据一直在手里：输出里白纸黑字写着
/// `Uvicorn running on http://0.0.0.0:8000` 和 `Application startup complete`。
/// 服务起没起来，看它自己说了什么最准——命令可以叫任何名字，启动横幅骗不了人。
/// 命令失败在"东西没装"上时，直接把下一步说清楚。
///
/// 用户实拍：让它跑一个 FastAPI 项目 → `ModuleNotFoundError: No module named 'fastapi'`
/// → 它去查 `pip list`、查 `sys.path`、反复重跑，就是没去装。用户的话：「有没有安装
/// 东西那些各种 lsp 之类不知道？」
///
/// 判断规则是**按项目自己的依赖清单**走，不靠猜：
/// - 清单里有这一项（requirements.txt / package.json / Cargo.toml / go.mod）→ 那就是
///   项目本来就要的，直接按清单装完继续，别停下来问；问了才是浪费用户时间。
/// - 清单里没有 → 那是**新增依赖**，属于改项目，要先问过用户。
///
/// 这句话挂在工具结果上，而不是塞进常驻提示词：只有真撞上才出现，每轮不花一个 token。
fn missing_dependency_advice(stdout: &str, stderr: &str) -> Option<&'static str> {
    let out = format!("{stdout}{stderr}").to_ascii_lowercase();
    let missing_module = out.contains("modulenotfounderror")
        || out.contains("no module named")
        || out.contains("cannot find module")
        || out.contains("importerror")
        || out.contains("command not found")
        || out.contains("is not recognized as an internal or external command")
        || out.contains("no such file or directory: 'python")
        || out.contains("could not find a version that satisfies")
        || out.contains("unresolved import");
    if !missing_module {
        return None;
    }
    Some(
        "缺的是依赖，不是代码问题。先看项目自己的依赖清单（requirements.txt / package.json / \
         Cargo.toml / go.mod / pyproject.toml）：\
         ① 清单里**有**这一项 → 直接按清单装（pip install -r requirements.txt、npm install、\
         cargo fetch、go mod download 之类），装完把刚才那条命令重跑一遍，别停下来问用户；\
         ② 清单里**没有** → 那是给项目新增依赖，先问用户要不要加，别自作主张装。\
         不要用 pip list / sys.path 反复确认——装一次再跑一次，结果自己会说话。",
    )
}

fn timeout_advice_for(command: &str, stdout: &str, stderr: &str) -> &'static str {
    let out_lower = format!("{stdout}{stderr}").to_ascii_lowercase();
    let output_says_server = [
        "running on http://",
        "listening on",
        "server running at",
        "press ctrl+c to quit",
        "application startup complete",
        "started server process",
        "local:   http://",
        "ready in ",
        "compiled successfully",
        "watching for file changes",
    ]
    .iter()
    .any(|marker| out_lower.contains(marker));
    let looks_like_service = output_says_server || {
        let c = command.to_ascii_lowercase();
        c.contains("dev")
            || c.contains("serve")
            || c.contains("watch")
            || c.contains("start")
            || c.contains("nodemon")
            || c.contains("http-server")
            || c.contains("tail -f")
    };
    if looks_like_service {
        "这是长驻服务。**它已经被杀掉了，现在没有在运行、端口也没在监听**——刚才那几行启动日志\
         说明它起来过，不代表它现在还活着。所以：不要对用户说「服务已启动」，也不要给他地址让他去打开。\
         要让它真正持续运行，用 run_in_terminal 在真实终端里重新起（那条路进程不会被杀），\
         再用 read_terminal / background_monitor 看输出和端口。"
    } else {
        "这看起来是一条会退出的命令，只是没跑完。**不要**因此改用 run_in_terminal——那是给服务用的，\
         而且长命令拿不到退出码，你要的验证结论就没了。直接重跑并把 timeout_secs 调大（上限 600）。"
    }
}

fn group_for_name(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if matches!(lower.as_str(), "build" | "compile" | "bundle") {
        "build"
    } else if matches!(lower.as_str(), "test" | "check" | "lint") || lower.contains("test") {
        "test"
    } else if matches!(lower.as_str(), "dev" | "start" | "serve" | "run") {
        "run"
    } else {
        "custom"
    }
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':' | '='))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn push_task(
    out: &mut Vec<TaskDefinition>,
    root: &Path,
    source: &str,
    label: impl Into<String>,
    command: impl Into<String>,
    group: impl Into<String>,
    problem_matcher: Option<String>,
) {
    let label = label.into();
    let source_id = source.to_lowercase().replace(' ', "-");
    let task_id = label
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    out.push(TaskDefinition {
        id: format!("{source_id}:{task_id}"),
        label,
        command: command.into(),
        cwd: root.to_string_lossy().to_string(),
        source: source.to_string(),
        group: group.into(),
        problem_matcher,
    });
}

fn add_package_tasks(root: &Path, out: &mut Vec<TaskDefinition>) {
    let path = root.join("package.json");
    let Ok(raw) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) else {
        return;
    };
    let mut names: Vec<&String> = scripts.keys().collect();
    names.sort();
    for name in names {
        let group = group_for_name(name);
        let matcher = match group {
            "build" | "test" => Some("$tsc".to_string()),
            _ => None,
        };
        push_task(
            out,
            root,
            "npm",
            format!("npm: {name}"),
            format!("npm run {}", shell_quote(name)),
            group,
            matcher,
        );
    }
}

fn add_cargo_tasks(root: &Path, out: &mut Vec<TaskDefinition>) {
    if !root.join("Cargo.toml").is_file() {
        return;
    }
    for (label, command, group) in [
        ("cargo: check", "cargo check", "test"),
        ("cargo: build", "cargo build", "build"),
        ("cargo: test", "cargo test", "test"),
        ("cargo: run", "cargo run", "run"),
    ] {
        push_task(
            out,
            root,
            "cargo",
            label,
            command,
            group,
            Some("$rustc".into()),
        );
    }
}

fn add_make_tasks(root: &Path, out: &mut Vec<TaskDefinition>) {
    if root.join("Makefile").is_file() || root.join("makefile").is_file() {
        push_task(out, root, "make", "make", "make", "build", None);
    }
}

fn json_array_strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn add_configured_tasks(root: &Path, rel: &str, source: &str, out: &mut Vec<TaskDefinition>) {
    let path = root.join(rel);
    let Ok(raw) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let Some(tasks) = json.get("tasks").and_then(|v| v.as_array()) else {
        return;
    };
    for task in tasks {
        let label = task
            .get("label")
            .and_then(|v| v.as_str())
            .or_else(|| task.get("taskName").and_then(|v| v.as_str()));
        let command = task.get("command").and_then(|v| v.as_str());
        let (Some(label), Some(command)) = (label, command) else {
            continue;
        };
        let args = json_array_strings(task.get("args").unwrap_or(&serde_json::Value::Null));
        let mut full_command = command.to_string();
        for arg in args {
            full_command.push(' ');
            full_command.push_str(&shell_quote(&arg));
        }
        let group = task
            .get("group")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| group_for_name(label));
        let matcher = task
            .get("problemMatcher")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        push_task(out, root, source, label, full_command, group, matcher);
    }
}

fn discover_tasks(root: &Path) -> Vec<TaskDefinition> {
    let mut out = Vec::new();
    // 目录改名（.michael → .mrdayone）后两处都发现：任务清单是**加法**，不像权限规则那样
    // 有"旧的压住新的"的风险，所以不做二选一——旧文件里的任务不会因为改名就从面板消失。
    add_configured_tasks(root, ".mrdayone/tasks.json", "Mr. Day One", &mut out);
    add_configured_tasks(root, ".michael/tasks.json", "Mr. Day One", &mut out);
    add_configured_tasks(root, ".vscode/tasks.json", "VS Code", &mut out);
    add_package_tasks(root, &mut out);
    add_cargo_tasks(root, &mut out);
    add_make_tasks(root, &mut out);
    out.sort_by(|a, b| a.source.cmp(&b.source).then(a.label.cmp(&b.label)));
    out
}

#[tauri::command]
pub fn tasks_list(root: String) -> Result<Vec<TaskDefinition>, String> {
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err("workspace root is not a directory".into());
    }
    Ok(discover_tasks(&root))
}

/// Captured result of running a task to completion (non-interactive).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRunResult {
    code: i32,
    stdout: String,
    stderr: String,
    combined: String,
    truncated: bool,
    timed_out: bool,
    /// Which confinement actually applied: `seatbelt`, `bubblewrap`, or `none`. Reported so
    /// the UI states what happened instead of implying protection the command never got —
    /// an unavailable sandbox degrades to running unconfined, it never blocks the command.
    sandbox: String,
    /// The command failed AND the output looks like the sandbox refusing a write. Lets the
    /// caller offer "re-run this one unconfined" instead of leaving the model to thrash on
    /// what reads like a mysterious permissions bug.
    sandbox_denied: bool,
}

const MAX_TASK_OUTPUT: usize = 2 * 1024 * 1024;
/// Kill a captured command after this long so a server/watch/blocked command
/// can't hang the caller forever (long but enough for slow builds/installs).
const TASK_TIMEOUT_SECS: u64 = 600;

/// Truncate a UTF-8 string to at most `max` bytes without splitting a code
/// point. Returns true when truncation happened.
fn truncate_on_boundary(s: &mut String, max: usize) -> bool {
    if s.len() <= max {
        return false;
    }
    let mut idx = max;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    s.truncate(idx);
    true
}

#[cfg(not(windows))]
fn task_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
}

/// Run a discovered task to completion and capture stdout/stderr so the
/// frontend can feed it through a problem matcher into the Problems panel.
/// This is the non-interactive complement to running a task in the terminal.
#[tauri::command]
/// `sandbox` confines the command to the workspace at the OS level and defaults to ON: an
/// agent-run command is the one place a prompt injection turns into persistence. The caller
/// passes `false` only for an explicit, user-approved escape (see `sandbox_denied`).
pub async fn task_run_capture(
    cwd: String,
    command: String,
    timeout_secs: Option<u64>,
    sandbox: Option<bool>,
) -> Result<TaskRunResult, String> {
    // Run the blocking spawn + wait loop on the blocking pool, NOT the Tauri
    // event-loop thread. A sync command here blocks that thread for the command's
    // whole duration (up to the command timeout), freezing the whole IDE — the cause
    // of "调用终端容易卡死一会". spawn_blocking keeps the UI responsive throughout.
    tauri::async_runtime::spawn_blocking(move || {
        task_run_capture_inner(cwd, command, timeout_secs, sandbox.unwrap_or(true))
    })
    .await
    .map_err(|e| format!("task thread join failed: {e}"))?
}

fn task_run_capture_inner(
    cwd: String,
    command: String,
    timeout_secs: Option<u64>,
    sandbox: bool,
) -> Result<TaskRunResult, String> {
    let dir = PathBuf::from(&cwd);
    if !dir.is_dir() {
        // 带上路径和**身份**。原文是一句不带任何路径的 "task working directory is not a
        // directory"，前端把它当普通 stderr 拼成 `[ERROR] run_cmd 退出 1`，而失败分型的
        // 四条规则全部落空，只补一句通用的"按上面真实输出定位根因"，外层恢复又给出
        // "判断是路径、参数、依赖、端口还是权限错误"——正好把模型引向**命令本身**。
        // 于是它会改命令、换路径、反复重试，而真相是**整个工作区根目录没了**
        // （被改名、移动，或所在卷已卸载），所有 run_cmd 和文件读写都会因此失败。
        return Err(format!(
            "[WORKSPACE_GONE] 工作目录不存在：{cwd}\n\
             这是**当前工作区的根目录**，不是命令里写的路径——所以换命令、换路径都没用，\
             这个会话里后续所有 run_cmd 和文件操作都会同样失败。\
             多半是这个文件夹被改名、移动了，或所在的卷已卸载。\
             停下来告诉用户重新打开这个文件夹（或说清它被移到哪了），不要继续重试。"
        ));
    }
    if command.trim().is_empty() {
        return Err("empty task command".into());
    }

    #[cfg(windows)]
    let mut cmd = {
        // 解释器由 shell_env 决定：装了 Git for Windows 就走 bash，模型写的 POSIX 直接是对的；
        // 没装才降级到 cmd.exe。降级不是静悄悄的——前端会把 kind 告诉模型，也会提示用户。
        //
        // 两个 Windows 专属的编码修正保留：
        //   1. `chcp 65001` 把这个子进程的控制台切到 UTF-8，中文路径 / 报错才不会以
        //      OEM 代码页（GBK/936）回来变成我们解不了的乱码。
        //   2. PYTHONUTF8 / PYTHONIOENCODING 让 Python 工具链也吐 UTF-8。
        let plan = crate::shell_env::plan();
        let mut c = crate::process_util::command(&plan.program);
        if plan.kind == "cmd" {
            // raw_arg 保住 cmd 的元字符（& | > ""）——std 正常的参数引用会把它们改写掉。
            use std::os::windows::process::CommandExt;
            c.raw_arg(format!("/C chcp 65001>nul & {command}"));
        } else {
            // bash 用普通 argv 传参才是对的，不需要 raw_arg。
            c.args(&plan.oneshot).arg(&command);
        }
        c.current_dir(&dir)
            // 外部改的环境变量在这里当场生效。Windows 永远不会改写运行中进程的环境块，
            // 只往注册表写再广播一条消息；不主动读注册表，用户 setx 出来的东西这个进程
            // 到死都看不见。
            .envs(crate::shell_env::registry_env())
            // 这个分支以前**从来没有设过 PATH**（设 PATH 那一行在 cfg(not(windows)) 里面），
            // 于是工作区的 node_modules\.bin、venv\Scripts 一个都进不来。
            .env("PATH", crate::process_util::augmented_path(Some(&cwd)))
            .env("PYTHONUTF8", "1")
            .env("PYTHONIOENCODING", "utf-8");
        for (k, v) in crate::shell_env::posix_shim_env(&plan.kind, &plan.program) {
            c.env(k, v);
        }
        c
    };
    #[cfg(windows)]
    let sandbox_kind = "none";

    #[cfg(not(windows))]
    let (mut cmd, sandbox_kind) = {
        // A login shell loads the user's profile so cargo/npm/go resolve. Use the SHARED
        // process_util::augmented_path(cwd) — it prepends the workspace's `.venv/bin` + `venv/bin` +
        // node_modules/.bin and the user's real login-shell PATH. (A private helper here used to omit
        // all of those, so an AI-installed venv was invisible → "环境丢失、重装" loop.)
        //
        // When confinement is available the same shell invocation runs under it instead. The
        // plan is `None` on an unsupported platform or an unresolvable workspace, and then
        // this behaves exactly as it did before the sandbox existed — degrade to unconfined,
        // never to "command refused".
        let shell = task_shell();
        let plan = if sandbox {
            crate::sandbox::wrap(&shell, &["-lc"], &command, &dir, &[])
        } else {
            None
        };
        let kind = plan.as_ref().map(|p| p.kind).unwrap_or("none");
        let mut c = match &plan {
            Some(p) => {
                let mut c = crate::process_util::command(&p.program);
                c.args(&p.args);
                c
            }
            None => {
                let mut c = crate::process_util::command(&shell);
                c.arg("-lc").arg(&command);
                c
            }
        };
        c.current_dir(&dir)
            .env("PATH", crate::process_util::augmented_path(Some(&cwd)))
            .env("CI", "1")
            .env("TERM", "dumb");
        // 从 Finder 启动的应用一个 locale 变量都没有，子进程于是跑在 C locale 下，
        // 按字符处理的工具（awk / wc -m / sort / ls）全部退化成按字节 —— 中文就烂了。
        // 用户自己配过就不动（见 utf8_locale_env）。
        for (k, v) in crate::process_util::utf8_locale_env() {
            c.env(k, v);
        }
        // Auto-activate a project venv so bare `python`/`pip`/`pytest` resolve INTO it and an installed
        // environment PERSISTS across restarts (previously they hit the system interpreter).
        for name in [".venv", "venv"] {
            let venv = dir.join(name);
            if venv.join("bin/activate").exists() {
                c.env("VIRTUAL_ENV", venv.to_string_lossy().to_string());
                c.env_remove("PYTHONHOME");
                break;
            }
        }
        (c, kind)
    };
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Make the shell a process-group leader so a timeout can terminate npm,
        // cargo, test runners, and every grandchild instead of only the wrapper shell.
        cmd.process_group(0);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to run task: {e}"))?;

    // Drain stdout/stderr on threads (a full pipe buffer would otherwise deadlock
    // the child), capping each so a flood can't exhaust memory.
    let mut out_pipe = child.stdout.take().unwrap();
    let mut err_pipe = child.stderr.take().unwrap();
    let (tx_o, rx_o) = std::sync::mpsc::channel::<Vec<u8>>();
    let (tx_e, rx_e) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut b = Vec::new();
        read_capped(&mut out_pipe, &mut b, MAX_TASK_OUTPUT);
        let _ = tx_o.send(b);
    });
    std::thread::spawn(move || {
        let mut b = Vec::new();
        read_capped(&mut err_pipe, &mut b, MAX_TASK_OUTPUT);
        let _ = tx_e.send(b);
    });

    // Wait with a timeout and kill a command that runs too long (a dev server, a
    // watch, or one blocked on input) instead of hanging the caller forever.
    let timeout_secs = timeout_secs
        .unwrap_or(TASK_TIMEOUT_SECS)
        .clamp(1, TASK_TIMEOUT_SECS);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    terminate_task_tree(&mut child);
                    timed_out = true;
                    break -1;
                }
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
            Err(_) => break -1,
        }
    };

    let out_bytes = rx_o
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap_or_default();
    let err_bytes = rx_e
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap_or_default();

    // 收尾：把整个进程组清干净。
    //
    // 之前只有超时分支会 terminate_task_tree。但包装 shell **正常退出**并不代表它启动的
    // 东西也退出了：`npm run dev &`、`nohup ... &` 这类命令会让 shell 立刻返回 0，而孙
    // 进程留在进程组里永久活着——没有任何人再持有它的句柄，也就永远不会被回收。同时它
    // 继承了 stdout/stderr 管道写端，导致上面那两个 reader 线程被永久钉在 read() 上
    // （`recv_timeout` 只是让我们别等它，线程本身并没有结束）。
    //
    // run_cmd 的语义就是「一次性命令 + 真实退出码」，跑完还活着的东西按定义就是泄漏；
    // 需要常驻服务的场景有专门的 run_in_terminal（它有自己的终端页签和生命周期）。
    // 已经拿到退出码和输出之后再清理，所以不影响任何正常命令的结果。
    if !timed_out {
        terminate_task_tree(&mut child);
    }
    // 不能用 from_utf8_lossy：Windows 的命令行工具往**管道**里写的是 ANSI 代码页
    // 字节（简体中文机器是 GBK），`chcp 65001` 只改控制台不改管道，于是每一个汉字
    // 都被换成 `�` 直接送进模型上下文。decode_process_output 先按 UTF-8 严格解，
    // 解不动才按系统代码页解，并且会把末尾被 8KB 分块/2MB 上限切断的半截字符去掉。
    let mut stdout = crate::process_util::decode_process_output(&out_bytes);
    let mut stderr = crate::process_util::decode_process_output(&err_bytes);
    let mut truncated = truncate_on_boundary(&mut stdout, MAX_TASK_OUTPUT)
        | truncate_on_boundary(&mut stderr, MAX_TASK_OUTPUT);
    if timed_out {
        truncated = true;
        // 这句话原来只有一个版本：「长时间运行的命令（如启动服务器）请在终端里手动运行」。
        //
        // 但绝大多数超时不是"你启了个服务"，而是"这条一次性命令就是慢"——冷编一个
        // Rust/Tauri 工程、docker build、make -j8、一整套集成测试。把它们诊断成"该去终端跑"
        // 是把模型主动推向错的方向：run_in_terminal 对长命令返回的是 running:true /
        // exitCode:null，而 purpose:"verify" 那条路要的正是退出码。于是模型照做，然后
        // 永远拿不到它需要的结论。
        //
        // 现在按命令形态分两种说法，并且明确告诉它"可以把 timeout_secs 调大"——这个参数
        // 后端一直支持（TASK_TIMEOUT_SECS 上限 600），只是以前没有暴露给模型。
        let advice = timeout_advice_for(&command, &stdout, &stderr);
        stderr.push_str(&format!(
            "\n[已超时 {timeout_secs}s，命令及其子进程已被终止。{advice}]"
        ));
    }
    if code != 0 {
        if let Some(dep) = missing_dependency_advice(&stdout, &stderr) {
            stderr.push_str(&format!("\n[{dep}]"));
        }
    }
    let combined = format!("{stdout}{stderr}");
    // Only claim a denial when the command actually FAILED and ran confined. A successful
    // command whose output happens to mention "operation not permitted" (a test asserting an
    // error message, a log replay) must not be reported as sandbox-blocked — that would send
    // the model chasing a permission problem that does not exist.
    let sandbox_denied =
        code != 0 && sandbox_kind != "none" && crate::sandbox::looks_like_denial(&combined);
    Ok(TaskRunResult {
        code,
        stdout,
        stderr,
        combined,
        truncated,
        timed_out,
        sandbox: sandbox_kind.to_string(),
        sandbox_denied,
    })
}

fn terminate_task_tree(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        // Safe because the child was placed in its own process group above. A
        // negative pid targets that group and cannot hit the IDE's process group.
        let _ = libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = crate::process_util::command("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Read a stream into `out`, but stop storing past `cap` bytes (keep draining so
/// the child process isn't blocked on a full pipe).
fn read_capped<R: std::io::Read>(r: &mut R, out: &mut Vec<u8>, cap: usize) {
    let mut buf = [0u8; 8192];
    loop {
        match r.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if out.len() < cap {
                    let take = (cap - out.len()).min(n);
                    out.extend_from_slice(&buf[..take]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    /// 用户实拍：跑项目撞上 `ModuleNotFoundError: No module named 'fastapi'`，它却去查
    /// pip list、查 sys.path、反复重跑，就是不去装。缺依赖时下一步必须当场说清楚。
    #[test]
    fn missing_dependency_gets_a_concrete_next_step() {
        let advice = missing_dependency_advice(
            "",
            "ModuleNotFoundError: No module named 'fastapi'\n",
        )
        .expect("撞上缺模块却没给下一步");
        assert!(advice.contains("requirements.txt"), "{advice}");
        assert!(advice.contains("别停下来问用户"), "清单里有的依赖不该反问用户：{advice}");
        assert!(advice.contains("先问用户"), "清单里没有的属于新增依赖，必须问：{advice}");
        assert!(
            advice.contains("不要用 pip list"),
            "不写这句它就会反复确认而不是直接装：{advice}"
        );
    }

    /// node / 可执行文件缺失也算同一类。
    #[test]
    fn missing_dependency_covers_node_and_missing_binaries() {
        assert!(missing_dependency_advice("", "Error: Cannot find module 'express'").is_some());
        assert!(missing_dependency_advice("", "bash: uvicorn: command not found").is_some());
    }

    /// 普通失败（测试挂了、编译错误）不该被扣上"缺依赖"的帽子——那会把模型推去装东西，
    /// 而真正该做的是读错误。
    #[test]
    fn ordinary_failures_get_no_dependency_advice() {
        assert!(missing_dependency_advice("", "AssertionError: expected 1 got 2").is_none());
        assert!(missing_dependency_advice("", "error[E0308]: mismatched types").is_none());
    }

    /// 用户实拍：`python3 main.py` 起了个 FastAPI 服务，10 秒被杀，而超时建议却说
    /// 「这看起来是一条会退出的命令，**不要**改用 run_in_terminal」——模型照做，于是
    /// 反复重跑、查 sys.path、查 pip list，就是不换终端。判据只按命令名猜，而命令名里
    /// 一个关键词都没有；证据其实在输出里。
    #[test]
    fn timeout_advice_reads_the_output_not_just_the_command_name() {
        let out = timeout_advice_for(
            "cd /x && python3 main.py",
            "INFO:     Uvicorn running on http://0.0.0.0:8000 (Press CTRL+C to quit)\nINFO:     Application startup complete.\n",
            "",
        );
        assert!(
            out.contains("长驻服务"),
            "输出里明说了服务已启动，还被判成一次性命令：{out}"
        );
        assert!(
            out.contains("已经被杀掉"),
            "必须点明它现在没在运行，否则模型会说「服务已启动」并把地址给用户：{out}"
        );
        assert!(
            out.contains("run_in_terminal"),
            "必须指路到 run_in_terminal：{out}"
        );
    }

    /// 反过来也要成立：真正会退出的长命令（冷编译、跑测试）不许被误判成服务，
    /// 否则模型改用 run_in_terminal，就永远拿不到它要的退出码。
    #[test]
    fn slow_one_shot_command_is_not_mistaken_for_a_service() {
        let out = timeout_advice_for(
            "cargo build --release",
            "   Compiling serde v1.0\n   Compiling tokio v1.0\n",
            "",
        );
        assert!(out.contains("会退出的命令"), "{out}");
        assert!(out.contains("timeout_secs"), "应该提示调大超时：{out}");
    }
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("michael-ide-{name}-{suffix}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn discovers_package_scripts() {
        let root = temp_root("npm");
        std::fs::write(
            root.join("package.json"),
            r#"{"scripts":{"dev":"vite","build":"vite build","test":"vitest"}}"#,
        )
        .unwrap();

        let tasks = discover_tasks(&root);
        let labels: Vec<&str> = tasks.iter().map(|task| task.label.as_str()).collect();
        assert!(labels.contains(&"npm: build"));
        assert!(labels.contains(&"npm: dev"));
        assert!(labels.contains(&"npm: test"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_cargo_tasks() {
        let root = temp_root("cargo");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='demo'\nversion='0.1.0'\n",
        )
        .unwrap();

        let tasks = discover_tasks(&root);
        assert!(tasks.iter().any(|task| task.command == "cargo check"));
        assert!(tasks.iter().any(|task| task.command == "cargo test"));
        let _ = std::fs::remove_dir_all(root);
    }

    /// 目录改名之后，新目录里的任务必须被发现——否则用户按提示把 tasks.json 放进
    /// `.mrdayone/`，面板里什么都不出现，而且不报错。
    #[test]
    fn discovers_configured_tasks_in_renamed_dir() {
        let root = temp_root("configured-renamed");
        std::fs::create_dir_all(root.join(".mrdayone")).unwrap();
        std::fs::write(
            root.join(".mrdayone/tasks.json"),
            r#"{"tasks":[{"label":"Lint","command":"npm","args":["run","lint"]}]}"#,
        )
        .unwrap();

        let tasks = discover_tasks(&root);
        let task = tasks.iter().find(|task| task.label == "Lint").unwrap();
        assert_eq!(task.command, "npm run lint");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_configured_tasks_with_args() {
        let root = temp_root("configured");
        std::fs::create_dir_all(root.join(".michael")).unwrap();
        std::fs::write(
            root.join(".michael/tasks.json"),
            r#"{"tasks":[{"label":"Type Check","command":"npm","args":["run","typecheck"],"group":"test","problemMatcher":"$tsc"}]}"#,
        )
        .unwrap();

        let tasks = discover_tasks(&root);
        let task = tasks
            .iter()
            .find(|task| task.label == "Type Check")
            .unwrap();
        assert_eq!(task.command, "npm run typecheck");
        assert_eq!(task.problem_matcher.as_deref(), Some("$tsc"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        // Each "é" is two bytes; truncating at an odd byte must not panic.
        let mut s = "é".repeat(10);
        let truncated = truncate_on_boundary(&mut s, 5);
        assert!(truncated);
        assert!(s.len() <= 5);
        // The result must still be valid UTF-8 (no partial code point).
        assert!(std::str::from_utf8(s.as_bytes()).is_ok());
    }

    #[test]
    fn truncate_noop_when_short() {
        let mut s = "hello".to_string();
        assert!(!truncate_on_boundary(&mut s, 100));
        assert_eq!(s, "hello");
    }

    // Every capture test runs WITH the sandbox on — the default path, so ordinary commands
    // must keep working under confinement. A regression that breaks normal execution inside
    // the sandbox fails these rather than surfacing later as "the terminal stopped working".
    #[cfg(not(windows))]
    #[test]
    fn capture_runs_command_and_collects_output() {
        let root = temp_root("capture");
        let result = task_run_capture_inner(
            root.to_string_lossy().to_string(),
            "echo michael-ide".into(),
            None,
            true,
        )
        .expect("task should run");
        assert_eq!(result.code, 0);
        assert!(result.combined.contains("michael-ide"));
        assert!(!result.sandbox_denied);
        let _ = std::fs::remove_dir_all(root);
    }

    /// 接线守卫：捕获路径必须**真的**把 utf8_locale_env() 的结果塞给子进程。
    ///
    /// 这里刻意不去断言"子进程的 LANG 是 UTF-8"——那条断言在我这台机器上不删改动也
    /// 照样绿（macOS 的 /etc/zprofile 里有 `if [ -z "$LANG" ]; then export
    /// LANG=C.UTF-8`，登录 zsh 自己就把 LANG 补上了），是个假守卫。
    /// 真正会断的接线只有一处：调用点被删掉。所以直接断言源码里有这个调用。
    #[cfg(not(windows))]
    #[test]
    fn the_capture_path_injects_the_utf8_locale() {
        // 只看测试模块**以前**的那部分源码。第一版直接搜整个文件，结果搜到的是这条断言
        // 自己写的那个字符串字面量——把真正的调用点删掉它照样绿。
        let src = include_str!("tasks.rs");
        let production = &src[..src.find("mod tests").expect("tests module")];
        assert!(
            production.contains("process_util::utf8_locale_env()"),
            "捕获路径不再注入 locale —— bash 登录 shell 下 `ls` 中文目录会打成 ????"
        );
    }

    /// 端到端：中文文件名一路走下来必须完好。
    ///
    /// 覆盖两件事：子进程的输出字节被正确解码（decode_process_output），以及路径里的中文
    /// 不会在 shell / 沙箱包装那一层被弄坏。
    #[cfg(not(windows))]
    #[test]
    fn chinese_filenames_survive_the_whole_capture_path() {
        let root = temp_root("capture-cjk");
        let sub = root.join("中文目录");
        std::fs::create_dir_all(&sub).expect("create dir");
        std::fs::write(sub.join("你好世界.txt"), b"hi").expect("write file");
        let result = task_run_capture_inner(
            root.to_string_lossy().to_string(),
            "ls 中文目录".into(),
            None,
            true,
        )
        .expect("task should run");
        assert_eq!(result.code, 0, "stderr={}", result.stderr);
        assert!(
            result.stdout.contains("你好世界.txt"),
            "中文文件名没能原样回来：{:?}",
            result.stdout
        );
        assert!(
            !result.combined.contains('\u{fffd}'),
            "输出里出现了替换字符 U+FFFD——解码环节把中文弄坏了：{:?}",
            result.combined
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(not(windows))]
    #[test]
    fn capture_reports_nonzero_exit() {
        let root = temp_root("capture-fail");
        let result =
            task_run_capture_inner(root.to_string_lossy().to_string(), "exit 3".into(), None, true)
                .expect("task should run");
        assert_eq!(result.code, 3);
        // A plain nonzero exit is not a sandbox denial; mislabelling it would send the model
        // hunting for a permissions problem that does not exist.
        assert!(!result.sandbox_denied);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn capture_rejects_bad_dir() {
        let err = task_run_capture_inner(
            "/nonexistent-michael-ide-dir-xyz".into(),
            "echo hi".into(),
            None,
            true,
        );
        assert!(err.is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn capture_timeout_terminates_the_command_tree() {
        let root = temp_root("capture-timeout");
        let started = std::time::Instant::now();
        let result = task_run_capture_inner(
            root.to_string_lossy().to_string(),
            "sleep 10 & wait".into(),
            Some(1),
            true,
        )
        .expect("timed command should return a result");
        assert_eq!(result.code, -1);
        assert!(result.timed_out);
        assert!(started.elapsed() < std::time::Duration::from_secs(4));
        let _ = std::fs::remove_dir_all(root);
    }

    // The whole point, end to end through the real command path: a write inside the
    // workspace lands, the same write to HOME does not, and the refusal is reported as a
    // sandbox denial so the caller can offer an explicit escape instead of leaving the model
    // to retry a command that can never succeed.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_sandboxed_command_cannot_write_outside_the_workspace() {
        if !crate::sandbox::available() {
            return;
        }
        let root = temp_root("capture-sandbox");
        let inside = task_run_capture_inner(
            root.to_string_lossy().to_string(),
            "echo ok > inside.txt".into(),
            None,
            true,
        )
        .expect("task should run");
        assert_eq!(inside.code, 0, "writes inside the workspace must still work");
        assert_eq!(inside.sandbox, "seatbelt");

        let home = std::env::var("HOME").expect("HOME");
        let probe = std::path::PathBuf::from(&home).join(".michael-tasks-sbtest-probe");
        let _ = std::fs::remove_file(&probe);
        let escaped = task_run_capture_inner(
            root.to_string_lossy().to_string(),
            format!("echo pwned > {}", probe.display()),
            None,
            true,
        )
        .expect("task should run");
        assert_ne!(escaped.code, 0, "a write to HOME must fail");
        assert!(!probe.exists(), "and must not have landed");
        assert!(escaped.sandbox_denied, "and must be reported as a sandbox denial");

        // The explicit escape hatch still works, so a legitimate outside-write is one
        // approval away rather than impossible.
        let allowed = task_run_capture_inner(
            root.to_string_lossy().to_string(),
            format!("echo ok > {}", probe.display()),
            None,
            false,
        )
        .expect("task should run");
        assert_eq!(allowed.code, 0);
        assert_eq!(allowed.sandbox, "none");
        assert!(probe.exists());

        let _ = std::fs::remove_file(&probe);
        let _ = std::fs::remove_dir_all(root);
    }
}
