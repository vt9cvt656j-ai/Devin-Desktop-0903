//! 跑**项目自己的** linter，有界、无 shell。
//!
//! # 为什么单独开一条路
//!
//! 写时正确性此前只有一条腿：Monaco 的内建 worker（JS/TS 的语法+类型）加 LSP 诊断。
//! 那条腿认的是**语法和类型**——「这行编译不过」。而用户实际吃亏的那一类是别的：
//! 漏 await 的 promise、比较用了 ==、React hook 依赖不全、拿到异常吞掉不处理、
//! 未使用的变量掩盖的拼写错误。这些在语法上全对，类型检查也全过，只有项目自己配的
//! linter 认得——而那份配置就躺在仓库里没人读。
//!
//! 纯 JS 项目更极端：没有类型检查，那条腿只剩语法，等于**唯一的写时正确性门几乎是空的**。
//!
//! # 为什么不复用 bash 工具
//!
//! bash 工具是给模型用的，要过审批门、要进终端、是 PTY 交互式的。这里要的是相反的东西：
//! 每批改动之后**静默**跑一次、几秒内必须回来、结果只喂给 harness 自己的阻断门。
//!
//! # 安全边界
//!
//! 不走 shell（没有 `sh -c`，因此没有命令拼接、没有通配符展开、没有环境变量替换），
//! 程序名走白名单，参数由调用方按结构传。这条命令能做的事只有「跑一个 linter」。

use std::time::Duration;

/// 一次 lint 的墙钟上限。交错验证整体只有几秒预算，一个卡住的 linter 不该把它吃光。
const LINT_TIMEOUT: Duration = Duration::from_millis(8000);

/// 允许被这条命令启动的程序。
///
/// 白名单是这条命令**唯一**的安全边界，所以判据必须是精确相等，不是「包含」也不是
/// 「以…结尾」：`npx` 通过而 `/tmp/evil/npx` 不通过，`eslint` 通过而 `eslint;rm` 不通过。
/// 路径分隔符一律拒绝——真正要跑项目本地那份 eslint 时走 `npx`，由它去解析
/// node_modules/.bin，不需要我们拼路径。
const ALLOWED: &[&str] = &[
    // JS/TS：npx 会优先用项目本地的那份（node_modules/.bin），版本和配置都跟仓库走。
    "npx",
    "eslint",
    "biome",
    "oxlint",
    // Python
    "ruff",
    "flake8",
    // Go
    "go",
    "golangci-lint",
    // Rust：只用于 `cargo clippy --message-format=json`，调用方自己决定值不值得等。
    "cargo",
];

#[derive(serde::Serialize, Debug, Clone)]
pub struct LintOutput {
    /// 进程退出码。被超时杀掉时为 -1。
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    /// 只在**跑不成**时非空：程序不在白名单、启动失败、超时。
    ///
    /// 「没跑成」和「跑了、没发现问题」在返回值里必须可分：混为一谈的话，一个没装
    /// eslint 的项目会得到「零错误」，而那正是最需要提醒的情形。
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl LintOutput {
    fn failed(note: impl Into<String>) -> Self {
        Self {
            code: -1,
            stdout: String::new(),
            stderr: String::new(),
            note: note.into(),
        }
    }
}

fn is_allowed(program: &str) -> bool {
    !program.contains('/') && !program.contains('\\') && ALLOWED.contains(&program)
}

/// 跑一次 linter 并把原始输出交回去。**不解析、不判断**——解析在 JS 侧
/// （src/agent/project-lint.js），那里是纯函数、可测、可变异验证。
///
/// `async` 不是可选的：这条命令有 8 秒硬超时，而 Tauri 的**同步**命令跑在主线程上，
/// 也就是 UI 线程。同步版本会让每一批改动之后整个界面冻住最多 8 秒——比它要解决的
/// 问题还严重。标了 async 之后 Tauri 把它派到异步运行时，界面不受影响。
#[tauri::command(async)]
pub fn project_lint_run(program: String, args: Vec<String>, cwd: String) -> LintOutput {
    if !is_allowed(&program) {
        return LintOutput::failed(format!("程序不在 lint 白名单里：{program}"));
    }
    if cwd.trim().is_empty() {
        return LintOutput::failed("没有工作目录");
    }
    // **必须走 process_util，不能用裸 Command::new。** 这是本仓已经踩过并修好的坑，
    // 它同时打掉过 LSP、MCP 和 F5 调试三块，而这里是个新调用点、没接上：
    //
    //   · PATH：GUI 启动的应用拿到的 PATH 很窄（Finder 里双击打开时基本只有
    //     /usr/bin:/bin:/usr/sbin:/sbin），nvm / volta / homebrew / 用户级 npm 前缀
    //     全不在里面 —— 也就是**打包发出去之后**这条腿必然一个 linter 都起不来，
    //     而开发机上从终端 `npm run tauri dev` 起的时候一切正常，看不出来。
    //   · PATHEXT：Windows 的 CreateProcessW 只给无扩展名的名字补 `.exe`、不查 PATHEXT，
    //     而 npm 装出来的是 `npx.cmd` —— eslint/biome/oxlint 三条 JS 腿在 Windows 上
    //     结构性哑掉。resolve_command 就是为这件事写的。
    //
    // 白名单在**解析之前**判（判的是调用方给的裸名字），所以解析出绝对路径不会绕过它。
    let resolved = crate::process_util::resolve_command(&program, Some(cwd.as_str()));
    // 参数一律原样传给 exec，不经过 shell。所以这里不需要转义，也**不能**有转义——
    // 加转义反而会让 `--ext=.ts,.tsx` 这种带逗号的参数被改写。
    let mut command = crate::process_util::command(&resolved);
    command
        .args(&args)
        .current_dir(&cwd)
        .env("PATH", crate::process_util::augmented_path(Some(cwd.as_str())))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // 自成进程组：超时要杀的是**整棵子树**。`npx eslint` 起的是 node，真正干活的是孙进程，
    // 只 kill 直接子进程的话 npx 退出而 eslint 还在跑，读管道的两个线程也跟着挂住。
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Safe：pre_exec 里只调 setsid()，异步信号安全，不分配、不加锁。
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    let child = match command.spawn() {
        Ok(child) => child,
        Err(err) => return LintOutput::failed(format!("启动失败：{err}")),
    };
    #[cfg(unix)]
    let pgid = child.id() as i32;
    match crate::env_probe::wait_with_timeout_pub(child, LINT_TIMEOUT) {
        Some(out) => LintOutput {
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            note: String::new(),
        },
        None => {
            // wait_with_timeout 只 kill 了直接子进程。整个进程组也要收掉，
            // 否则一个卡住的 eslint 会一直占着 CPU 和文件锁，而我们已经不看它了。
            #[cfg(unix)]
            unsafe {
                libc::killpg(pgid, libc::SIGKILL);
            }
            LintOutput::failed(format!("超时（{}ms）", LINT_TIMEOUT.as_millis()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 白名单是这条命令唯一的安全边界，所以它必须是**精确相等**。
    ///
    /// 「包含」或「以…结尾」那种写法看着也能挡住明显的坏名字，实际上
    /// `/tmp/evil/npx`、`npx../../evil` 全都能过——而这条命令不走 shell 的好处会被
    /// 一个能指定任意路径的程序名整个抵消掉。
    #[test]
    fn the_allowlist_is_exact_and_rejects_paths() {
        assert!(is_allowed("npx"));
        assert!(is_allowed("ruff"));
        assert!(!is_allowed("/usr/local/bin/npx"), "带路径的一律拒绝");
        assert!(!is_allowed("./npx"));
        assert!(!is_allowed("..\\npx"));
        assert!(!is_allowed("npx;rm -rf /"), "命令拼接不该被当成程序名");
        assert!(!is_allowed("bash"));
        assert!(!is_allowed("sh"));
        assert!(!is_allowed("node"));
        assert!(!is_allowed(""));
    }

    /// 不在白名单 / 没有工作目录时，返回的必须是**note 非空**的失败，而不是
    /// 「code=0、输出为空」——后者和「跑过了、干干净净」字节级不可区分，
    /// 调用方会把一次没跑成当成一次通过。
    #[test]
    fn a_refusal_is_distinguishable_from_a_clean_run() {
        let refused = project_lint_run("bash".into(), vec!["-c".into(), "echo hi".into()], "/tmp".into());
        assert_eq!(refused.code, -1);
        assert!(!refused.note.is_empty(), "拒绝必须自报，否则会被当成「零问题」");
        assert!(refused.stdout.is_empty());

        let no_cwd = project_lint_run("ruff".into(), vec!["--version".into()], "  ".into());
        assert!(!no_cwd.note.is_empty());
    }

    /// 不走 shell 这件事必须留在源码里：一旦有人改成 `sh -c`，白名单就只挡程序名、
    /// 参数里的一切都会被 shell 解释，这条命令的安全边界当场失效。
    #[test]
    fn it_never_goes_through_a_shell() {
        // 断言只跑在**代码行**上：这个文件的文档注释里逐字写着 `sh -c`（在解释为什么
        // 不用它），不剥注释的话断言会匹配到自己的说明文字，永远红。
        let src = include_str!("lint.rs");
        let prod = src[..src.find("#[cfg(test)]").unwrap_or(src.len())]
            .lines()
            .filter(|line| {
                let t = line.trim_start();
                !t.starts_with("//") && !t.starts_with("/*") && !t.starts_with("*")
            })
            .collect::<Vec<_>>()
            .join("\n");
        for shell in ["Command::new(\"sh\")", "Command::new(\"bash\")", "Command::new(\"cmd\")", "sh -c"] {
            assert!(
                !prod.contains(shell),
                "lint 执行路径经过了 shell（{shell}）—— 白名单只管程序名，参数会被 shell 重新解释"
            );
        }
        assert!(
            prod.contains("crate::process_util::command(&resolved)"),
            "没走 process_util —— GUI 启动时 PATH 很窄，打包发出去之后一个 linter 都起不来"
        );
        assert!(
            prod.contains("crate::process_util::resolve_command(&program"),
            "没解析命令名 —— Windows 上 npx.cmd 找不到，三条 JS 腿结构性哑掉"
        );
        assert!(
            prod.contains("#[tauri::command(async)]"),
            "同步命令跑在 UI 线程上，8 秒超时会把界面冻住"
        );
    }

    /// 真跑一次白名单里的程序，确认输出和退出码是原样交回的。
    ///
    /// 用 `go`：它在 CI 和开发机上都存在的概率最高；不在时这条测试自己跳过，
    /// 而不是假装通过（「没探到」不等于「探到了没有」）。
    #[cfg(unix)]
    #[test]
    fn a_real_run_returns_the_process_output_verbatim() {
        let out = project_lint_run("go".into(), vec!["version".into()], "/tmp".into());
        if out.code == -1 && out.note.starts_with("启动失败") {
            return; // 这台机器上没有 go，跳过（不是通过）
        }
        assert!(out.note.is_empty(), "跑起来了就不该有 note：{}", out.note);
        assert_eq!(out.code, 0);
        assert!(out.stdout.contains("go version"), "输出没有原样交回：{:?}", out.stdout);
    }
}
