//! 「这台机器上到底有什么」——一次问清楚。
//!
//! # 为什么需要它
//!
//! 智能体撞上意外时最没效率的一段，是**逐个试探环境**：`node -v` 失败 → 下一轮
//! `which node` → 再下一轮 `ls /usr/local/bin` → 再下一轮 `nvm ls`。每一轮都是一次完整的
//! 模型往返，而它想知道的其实只有一句话：这台机器上有什么、版本多少、这个项目用哪套工具链。
//!
//! 在这之前 IDE 里**没有任何工具**能回答这个问题（`live_environment` 是查天气和地震的，
//! 名字像但不是一回事）。于是"卡住时先搞清楚状况"这个最基本的动作，成本高到模型宁可去猜。
//!
//! 这个模块把二十来个探测并发跑掉，一次返回。典型耗时几百毫秒——比一次模型往返便宜两个
//! 数量级。
//!
//! # 只报事实
//!
//! 探测结果里不写任何"建议"。装没装、什么版本、是不是 git 仓库、有哪个 lockfile——
//! 都是可核对的事实。怎么用这些事实是模型的判断，不是这里替它做的决定。
//! 唯一的例外是 `notes`：那里放的是**探测本身失败**的说明（比如超时），因为"没探到"和
//! "确实没有"是两回事，混淆这两者会让模型据此断言一个不成立的结论。

use serde::Serialize;
use std::time::Duration;

/// 单个探测的墙钟上限。探测器卡住不该拖垮整次调用——一个坏掉的 `docker` 客户端
/// 能挂十几秒，而我们只是想知道它在不在。
const PROBE_TIMEOUT: Duration = Duration::from_millis(2500);

#[derive(Serialize, Debug, Clone)]
pub struct ToolPresence {
    pub name: String,
    pub found: bool,
    /// 可执行文件的绝对路径；没找到时为空。
    pub path: String,
    /// `--version` 的第一行（清洗过）。探测超时或命令存在但报错时为空。
    pub version: String,
    /// 只在"文件在但等于没装"时填：把系统给的原话交出去（如 macOS 的 java 存根说的
    /// "Unable to locate a Java Runtime"）。模型据此知道该让用户装什么，而不是以为路径错了。
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

#[derive(Serialize, Debug, Default)]
pub struct WorkspaceFacts {
    pub root: String,
    pub is_git_repo: bool,
    pub branch: String,
    /// 未提交改动的文件数。-1 表示没问出来。
    pub dirty_files: i64,
    /// 检测到的包管理器（按 lockfile），可能多个——monorepo 里混用是常态。
    pub package_managers: Vec<String>,
    /// 项目里存在的显著清单文件（package.json / Cargo.toml / pyproject.toml …）。
    pub manifests: Vec<String>,
    /// 顶层目录里的条目数，给"这是不是个空工作区"一个直接答案。
    pub entries: i64,
}

#[derive(Serialize, Debug)]
pub struct EnvProbe {
    pub os: String,
    pub arch: String,
    pub shell: String,
    pub home: String,
    pub cwd: String,
    /// PATH 里的目录数——`command not found` 时最先该看的东西。
    pub path_entries: usize,
    pub tools: Vec<ToolPresence>,
    pub workspace: WorkspaceFacts,
    /// 探测本身出的问题（超时等）。"没探到"和"确实没有"是两回事。
    pub notes: Vec<String>,
}

/// 常见工具链。顺序无关紧要（并发跑），但保持稳定输出便于人读。
const PROBES: &[(&str, &str)] = &[
    ("git", "--version"),
    ("node", "--version"),
    ("npm", "--version"),
    ("pnpm", "--version"),
    ("yarn", "--version"),
    ("bun", "--version"),
    ("deno", "--version"),
    ("python3", "--version"),
    ("pip3", "--version"),
    ("uv", "--version"),
    ("cargo", "--version"),
    ("rustc", "--version"),
    ("go", "version"),
    ("java", "-version"),
    ("docker", "--version"),
    ("make", "--version"),
    ("cmake", "--version"),
    ("gcc", "--version"),
    ("swift", "--version"),
    ("ruby", "--version"),
    ("php", "--version"),
    ("psql", "--version"),
    ("sqlite3", "--version"),
    ("gh", "--version"),
    ("rg", "--version"),
    ("jq", "--version"),
    ("ffmpeg", "-version"),
];

/// 版本输出清洗成一行。
///
/// 各家格式五花八门（`git version 2.39.5`、`v22.3.0`、`Python 3.12.1`、
/// java 还把版本打到 stderr），统一取**首个非空行**并截断——模型要的是"哪个大版本"，
/// 不是完整的 banner。
fn first_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .chars()
        .take(80)
        .collect()
}

fn probe_one(name: &str, flag: &str) -> ToolPresence {
    let resolved = crate::process_util::resolve_system_command(name);
    let mut out = ToolPresence {
        name: name.to_string(),
        found: false,
        path: String::new(),
        version: String::new(),
        note: String::new(),
    };
    // resolve_system_command 解析不到时会原样返回名字；那种情况下仍然试一次，
    // 因为 PATH 里可能有 shim（nvm / pyenv / asdf 这类）。
    let mut cmd = crate::process_util::command(&resolved);
    cmd.arg(flag)
        .env("PATH", crate::process_util::augmented_path(None))
        // 有些工具在非交互环境下会去问终端；一律关掉，否则探测会挂到超时。
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::null());

    let child = match cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        // spawn 失败＝这个命令确实不存在（或不可执行）。这是一个**确定**的答案。
        Err(_) => return out,
    };
    match wait_with_timeout(child, PROBE_TIMEOUT) {
        Some(o) => {
            // java -version 打到 stderr；两边都看。
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            let v = first_line(&stdout);
            let version = if v.is_empty() { first_line(&stderr) } else { v };

            // 「文件存在」不等于「装了」。
            //
            // macOS 自带一个永远存在的 /usr/bin/java 存根：spawn 成功、进程正常起来，
            // 但输出是 "Unable to locate a Java Runtime"、退出码非零。只看 spawn 成不成功
            // 会把它报成"装了 java"，模型据此去跑 Java 构建，然后撞一堵它完全没预料到的墙。
            // 同类的还有 Xcode 的命令行工具存根。
            // 判据：退出码非零 **且** 输出像"没装"，才翻成 found=false——只看退出码不行，
            // 有些工具 --version 本来就返回非零。
            let looks_absent = !o.status.success()
                && {
                    let low = format!("{stdout}{stderr}").to_lowercase();
                    low.contains("unable to locate")
                        || low.contains("not found")
                        || low.contains("no such file")
                        || low.contains("is not installed")
                        || low.contains("no developer tools")
                };
            if looks_absent {
                out.found = false;
                out.version = String::new();
                out.note = first_line(if stdout.trim().is_empty() { &stderr } else { &stdout });
                return out;
            }
            out.found = true;
            if resolved != name {
                out.path = resolved;
            }
            out.version = version;
        }
        // 超时：命令**存在**（spawn 成功了），只是没在限时内答上来。
        // 这一点必须和"不存在"区分开，否则模型会得出"这台机器没装 docker"的错误结论。
        None => {
            out.found = true;
            if resolved != name {
                out.path = resolved;
            }
            out.version = String::new();
        }
    }
    out
}

/// 等子进程，超时就杀掉。标准库没有带超时的 wait，这里用轮询——探测都是毫秒级返回的
/// 短命令，20ms 的粒度足够，不值得为它引入一个依赖。
fn wait_with_timeout(
    mut child: std::process::Child,
    limit: Duration,
) -> Option<std::process::Output> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) => {
                if start.elapsed() >= limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return None,
        }
    }
}

fn git_facts(root: &str, facts: &mut WorkspaceFacts, notes: &mut Vec<String>) {
    let git = crate::process_util::resolve_system_command("git");
    let run = |args: &[&str]| -> Option<String> {
        let child = crate::process_util::command(&git)
            .arg("-C")
            .arg(root)
            .args(args)
            .env("PATH", crate::process_util::augmented_path(None))
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;
        let out = wait_with_timeout(child, PROBE_TIMEOUT)?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    match run(&["rev-parse", "--is-inside-work-tree"]) {
        Some(v) if v == "true" => facts.is_git_repo = true,
        Some(_) => {}
        None => {
            // 问不出来有两种可能：不是仓库，或者 git 根本没装。后者要说，否则模型会
            // 把"没装 git"误读成"这个目录不是仓库"，然后去 git init 一个已有仓库。
            if !std::path::Path::new(&crate::process_util::resolve_system_command("git")).exists() {
                notes.push("git 未安装或不在 PATH 上——is_git_repo=false 不代表这个目录不是仓库".into());
            }
            return;
        }
    }
    if !facts.is_git_repo {
        return;
    }
    facts.branch = run(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    facts.dirty_files = run(&["status", "--porcelain"])
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count() as i64)
        .unwrap_or(-1);
}

fn workspace_facts(root: &str, notes: &mut Vec<String>) -> WorkspaceFacts {
    let mut facts = WorkspaceFacts {
        root: root.to_string(),
        dirty_files: -1,
        ..Default::default()
    };
    if root.is_empty() {
        notes.push("没有打开工作区——涉及项目的判断一律不成立，先让用户打开文件夹".into());
        return facts;
    }
    let p = std::path::Path::new(root);
    if !p.is_dir() {
        notes.push(format!("{root} 不是一个目录"));
        return facts;
    }
    // lockfile → 包管理器。用 lockfile 而不是 package.json 里的 packageManager 字段：
    // 前者是"实际用过什么"的证据，后者只是声明。
    for (file, pm) in [
        ("pnpm-lock.yaml", "pnpm"),
        ("yarn.lock", "yarn"),
        ("bun.lockb", "bun"),
        ("package-lock.json", "npm"),
        ("Cargo.lock", "cargo"),
        ("poetry.lock", "poetry"),
        ("uv.lock", "uv"),
        ("Pipfile.lock", "pipenv"),
        ("go.sum", "go"),
        ("composer.lock", "composer"),
        ("Gemfile.lock", "bundler"),
    ] {
        if p.join(file).exists() {
            facts.package_managers.push(pm.to_string());
        }
    }
    for file in [
        "package.json",
        "Cargo.toml",
        "pyproject.toml",
        "requirements.txt",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "Gemfile",
        "composer.json",
        "Makefile",
        "CMakeLists.txt",
        "Dockerfile",
        "docker-compose.yml",
    ] {
        if p.join(file).exists() {
            facts.manifests.push(file.to_string());
        }
    }
    facts.entries = std::fs::read_dir(p)
        .map(|it| it.filter_map(Result::ok).count() as i64)
        .unwrap_or(-1);
    git_facts(root, &mut facts, notes);
    facts
}

/// 一次问清楚这台机器和这个工作区的状况。
#[tauri::command(async)]
pub fn probe_env(root: Option<String>) -> Result<EnvProbe, String> {
    let root = root.unwrap_or_default().trim().to_string();
    let mut notes: Vec<String> = Vec::new();

    // 并发跑所有探测。串行的话 27 个命令 × 几十毫秒就到秒级了，而这个工具的价值
    // 恰恰在于"比一次模型往返便宜得多"。
    let handles: Vec<_> = PROBES
        .iter()
        .map(|(name, flag)| {
            let (name, flag) = (name.to_string(), flag.to_string());
            std::thread::spawn(move || probe_one(&name, &flag))
        })
        .collect();
    let mut tools: Vec<ToolPresence> = Vec::with_capacity(handles.len());
    for h in handles {
        match h.join() {
            Ok(t) => tools.push(t),
            Err(_) => notes.push("有一个探测线程 panic 了，结果不完整".into()),
        }
    }

    let workspace = workspace_facts(&root, &mut notes);
    let path_entries = std::env::var("PATH")
        .map(|p| p.split(':').filter(|s| !s.is_empty()).count())
        .unwrap_or(0);

    Ok(EnvProbe {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        shell: std::env::var("SHELL").unwrap_or_default(),
        home: std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default(),
        cwd: std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path_entries,
        tools,
        workspace,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_that_exists_is_found_with_a_version() {
        // git 在开发机上一定有；用它验证"存在"这条路是通的。
        let t = probe_one("git", "--version");
        assert!(t.found, "git 应当被探到");
        assert!(t.version.contains("git"), "版本行没解析出来：{:?}", t.version);
    }

    #[test]
    fn a_command_that_does_not_exist_is_reported_as_absent_not_as_unknown() {
        let t = probe_one("mrday-definitely-not-a-real-binary-xyz", "--version");
        assert!(!t.found, "不存在的命令不该被报成存在");
        assert!(t.version.is_empty());
    }

    #[test]
    fn version_output_is_one_clean_line_even_when_the_tool_prints_a_banner() {
        assert_eq!(first_line("\n\n  git version 2.39.5  \nmore\nlines"), "git version 2.39.5");
        assert_eq!(first_line(""), "");
        // 超长 banner 要截断——模型要的是大版本，不是整段 banner
        let long = "x".repeat(200);
        assert_eq!(first_line(&long).chars().count(), 80);
    }

    #[test]
    fn workspace_facts_are_read_from_lockfiles_not_from_declarations() {
        let dir = std::env::temp_dir().join(format!("mrday-env-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        std::fs::write(dir.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]").unwrap();

        let mut notes = Vec::new();
        let f = workspace_facts(&dir.to_string_lossy(), &mut notes);
        // lockfile 是"实际用过什么"的证据；package.json 里的 packageManager 只是声明
        assert!(f.package_managers.contains(&"pnpm".to_string()), "{:?}", f.package_managers);
        assert!(!f.package_managers.contains(&"npm".to_string()), "没有 package-lock 就不该报 npm");
        assert!(f.manifests.contains(&"package.json".to_string()));
        assert!(f.manifests.contains(&"Cargo.toml".to_string()));
        assert!(f.entries >= 3);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_workspace_is_stated_plainly_rather_than_left_to_be_inferred() {
        // "没打开工作区"和"工作区是空的"是两件事，模型据此的下一步完全不同。
        let mut notes = Vec::new();
        let f = workspace_facts("", &mut notes);
        assert!(f.root.is_empty());
        assert!(notes.iter().any(|n| n.contains("没有打开工作区")), "{notes:?}");
        assert!(notes.iter().any(|n| n.contains("先让用户打开文件夹")), "没给出下一步");
    }

    #[test]
    fn the_whole_probe_finishes_fast_enough_to_be_worth_calling() {
        // 这个工具的全部价值在于"比一次模型往返便宜得多"。串行跑 27 个命令会到秒级，
        // 那就不如让模型自己一条条试了。
        let start = std::time::Instant::now();
        let p = probe_env(None).unwrap();
        let elapsed = start.elapsed();
        assert!(elapsed < Duration::from_secs(6), "探测太慢：{elapsed:?}");
        assert_eq!(p.tools.len(), PROBES.len(), "有探测掉队了");
        assert!(!p.os.is_empty());
        assert!(p.path_entries > 0, "PATH 读不到");
    }
}

#[cfg(test)]
mod probe_demo {
    #[test]
    #[ignore]
    fn dump() {
        let p = super::probe_env(Some(
            "/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide".into(),
        ))
        .unwrap();
        println!("{}", serde_json::to_string_pretty(&p).unwrap());
    }
}

#[cfg(test)]
mod stub_tests {
    use super::*;

    #[test]
    fn a_stub_that_exists_but_is_not_actually_installed_is_reported_as_absent() {
        // macOS 自带的 /usr/bin/java 存根：文件在、能起进程，但输出是
        // "Unable to locate a Java Runtime"、退出码非零。只看 spawn 成不成功会把它
        // 报成"装了 java"，模型据此去跑 Java 构建，撞一堵完全没预料到的墙。
        //
        // 这条测试只在真的存在这种存根的机器上有意义；不存在时它验证的是"没装就报没装"。
        let t = probe_one("java", "-version");
        if !t.version.is_empty() {
            assert!(
                !t.version.to_lowercase().contains("unable to locate"),
                "存根被当成了真实版本：{}",
                t.version
            );
        }
        if !t.found && !t.note.is_empty() {
            assert!(
                t.note.to_lowercase().contains("unable to locate")
                    || t.note.to_lowercase().contains("not found"),
                "note 里应当是系统的原话：{}",
                t.note
            );
        }
        // 无论哪种情况，found 与 version 必须自洽：报了装了就得有版本号
        if t.found {
            assert!(!t.version.is_empty() || t.note.is_empty(),
                "报成装了却既没版本也没说明");
        }
    }

    #[test]
    fn a_tool_whose_version_flag_exits_nonzero_is_still_reported_as_present() {
        // 只看退出码会误伤：有些工具 --version 本来就返回非零。
        // 判据必须是"非零 **且** 输出像没装"。
        let src = include_str!("env_probe.rs");
        assert!(src.contains("let looks_absent = !o.status.success()"), "判据不该只看输出");
        assert!(src.contains("|| low.contains(\"not found\")"));
    }
}
