//! OS-level confinement for agent-run shell commands.
//!
//! The workspace boundary in `files.rs` only guards the STRUCTURED file tools
//! (`write_file`, `delete_path`, …). Command execution — `task_run_capture` and the PTY —
//! never consulted it, so `run_cmd("echo … >> ~/.zshrc")` was completely unbounded while
//! `write_file("~/.zshrc")` was correctly refused. A lock on the front door of a building
//! whose side door is the shell.
//!
//! This module closes that by asking the OS to enforce the same boundary on the command and
//! every process it spawns:
//!
//!   * macOS  — Seatbelt (`sandbox-exec`) with a generated SBPL profile.
//!   * Linux  — bubblewrap (`bwrap`) with a read-only root and read-write binds.
//!
//! # What it does and does not do
//!
//! This is **write confinement**, deliberately, not a full jail:
//!
//!   * WRITES are denied everywhere except the workspace, temp dirs, and a curated set of
//!     package-manager caches. That is what stops persistence — shell rc files, LaunchAgents,
//!     ssh `authorized_keys`, another repo's git hooks — and destructive writes outside the
//!     project.
//!   * READS stay open. Confining them breaks git-over-ssh (`~/.ssh`), toolchains, and
//!     compilers in ways that would get the sandbox switched off, which is strictly worse
//!     than a narrower sandbox that stays on.
//!   * NETWORK stays open. Egress filtering needs a proxy with a domain allowlist; that is a
//!     separate piece of work and is the natural follow-on, because without it a sandboxed
//!     command can still exfiltrate anything it can read.
//!
//! So: this stops an injected command from *persisting* or *destroying*. It does not yet stop
//! one from *reading* a secret and *sending* it somewhere. Both remaining halves are the
//! network layer's job.
//!
//! # Failure posture
//!
//! Every entry point degrades to "no sandbox" rather than "no command". A missing
//! `sandbox-exec`, an unusable `bwrap`, an unresolvable workspace path — all return `None`
//! and the caller runs the command unconfined, exactly as it did before this module existed.
//! Silently refusing to run commands would be a far worse regression than the status quo, and
//! the caller reports which mode was used so the UI never implies protection it did not get.

use std::path::{Path, PathBuf};

/// A command rewritten to run under OS confinement.
#[derive(Debug, Clone)]
pub struct SandboxPlan {
    /// The sandbox launcher to exec (`sandbox-exec` / `bwrap`).
    pub program: String,
    /// Its arguments, ending with the shell + command to run inside.
    pub args: Vec<String>,
    /// Which mechanism this plan uses — surfaced to the UI so "sandboxed" is never a guess.
    pub kind: &'static str,
}

/// Directories every sandboxed command may write to regardless of workspace, because the
/// dominant toolchains are unusable without them (`npm install` fails outright on a read-only
/// `~/.npm`). These are caches and build artifacts: poisoning one is a real but far lower
/// severity than the persistence vectors this module exists to block, and none of them is a
/// credential store or an autostart location.
const HOME_WRITABLE_RELATIVE: &[&str] = &[
    ".npm",
    ".cache",
    ".cargo",
    ".rustup",
    ".gradle",
    ".m2",
    ".pnpm-store",
    ".yarn",
    ".bun",
    ".deno",
    ".pub-cache",
    ".nuget",
    ".dotnet",
    ".gem",
    ".composer",
    ".stack",
    ".ivy2",
    ".sbt",
    "go/pkg",
    "Library/Caches",
    "Library/pnpm",
];

/// Canonicalize when possible, else fall back to the path as given. Seatbelt matches on real
/// paths, so `/tmp/x` must become `/private/tmp/x` or the rule silently never fires.
fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// Paths that must stay read-only even though they sit INSIDE one of the writable roots above.
///
/// SBPL is last-match-wins and bubblewrap's later binds win, so these are emitted after the
/// allow block and carve holes back out of it.
///
/// Why they are needed: the comment on `HOME_WRITABLE_RELATIVE` claims "none of them is a
/// credential store or an autostart location". That was not true. `~/.cargo/bin`, `~/.bun/bin`,
/// `~/.deno/bin`, `~/.yarn/bin`, `~/.dotnet/tools` and `~/.composer/vendor/bin` are on a
/// developer's PATH — dropping a file there is the same persistence this module exists to block,
/// just one directory over from `~/.local/bin`. `~/.cargo/config.toml` is worse than a PATH
/// entry: `rustc-wrapper` / `[target.*.runner]` / `[alias]` in it are executed by the NEXT
/// `cargo build`, no PATH lookup involved. `~/.gradle/init.d/*.gradle` is run by every gradle
/// invocation. `~/.m2/settings.xml` redirects Maven to whatever repository it names.
///
/// None of these is written by the flows the writable list exists for: `npm install`,
/// `cargo build`, `gradle build` and `mvn package` touch caches, not PATH directories or the
/// config that drives them. Installing a global binary or rewriting your cargo config from
/// inside a sandboxed agent command should not be silent — run it outside the sandbox.
const HOME_DENY_RELATIVE: &[&str] = &[
    ".cargo/bin",
    ".cargo/config.toml",
    ".cargo/config",
    ".cargo/env",
    ".bun/bin",
    ".deno/bin",
    ".yarn/bin",
    ".dotnet/tools",
    ".composer/vendor/bin",
    ".gem/bin",
    ".gradle/init.d",
    ".gradle/init.gradle",
    ".m2/settings.xml",
];

/// Whether a path is too broad to be handed to `(subpath …)` as a write root.
///
/// `/` is the one that actually happened: a caller passing `cwd = "/"` produced a profile whose
/// first rule was `(allow file-write* (subpath "/"))` — everything writable, `~/.ssh` included —
/// while the plan still reported `kind: "seatbelt"`. A sandbox that lies about confining is worse
/// than no sandbox, because the label is what the rest of the app reasons about.
///
/// This file's own `credential_and_autostart_locations_are_never_writable` test already asserted
/// `/` must never become a writable root; it just never passed `/` in as the workspace.
fn too_broad_for_write_root(p: &Path) -> bool {
    if p.parent().is_none() {
        return true; // filesystem root
    }
    if let Some(home) = home_dir() {
        if real(&home) == *p {
            return true; // the home directory itself contains every path we protect
        }
    }
    matches!(
        p.to_string_lossy().as_ref(),
        "/Users" | "/home" | "/private" | "/var" | "/opt" | "/usr" | "/etc" | "/Library"
            | "/Applications" | "/System" | "/private/var"
    )
}

/// Every directory a sandboxed command may write to.
fn writable_roots(workspace: &Path, extra: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    // A too-broad workspace is dropped rather than making the whole plan bail out: the command
    // still runs confined (tmp + caches writable, credentials and autostart denied) and a write
    // it genuinely needed fails visibly with EPERM. Bailing out would run it with no confinement
    // at all, which is the strictly worse of the two.
    let ws = real(workspace);
    if !too_broad_for_write_root(&ws) {
        roots.push(ws);
    }
    roots.push(real(&std::env::temp_dir()));
    for fixed in ["/tmp", "/private/tmp", "/var/folders", "/private/var/folders"] {
        let p = PathBuf::from(fixed);
        if p.exists() {
            roots.push(real(&p));
        }
    }
    if let Some(home) = home_dir() {
        for rel in HOME_WRITABLE_RELATIVE {
            roots.push(home.join(rel));
        }
    }
    for p in extra {
        let p = real(p);
        // `extra` comes from callers too (extra workspace roots), so it needs the same guard —
        // otherwise the hole just moves one argument over.
        if !too_broad_for_write_root(&p) {
            roots.push(p);
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

/// Paths carved back out of the writable set (see `HOME_DENY_RELATIVE`).
fn readonly_holes() -> Vec<PathBuf> {
    let Some(home) = home_dir() else { return Vec::new() };
    HOME_DENY_RELATIVE.iter().map(|rel| home.join(rel)).collect()
}

/// Escape a path for an SBPL string literal.
fn escape_sbpl(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

/// Build the Seatbelt profile.
///
/// SBPL is last-match-wins, so the order is: allow everything, then deny all writes, then
/// re-allow the specific write targets. Starting from `(allow default)` keeps this a WRITE
/// confinement — process exec, mach lookups, signals and reads all stay untouched, which is
/// what lets real toolchains run inside it.
fn seatbelt_profile(writable: &[PathBuf]) -> String {
    let mut profile = String::from("(version 1)\n(allow default)\n(deny file-write*)\n(allow file-write*\n");
    for root in writable {
        profile.push_str(&format!("  (subpath \"{}\")\n", escape_sbpl(root)));
    }
    // Character devices a shell pipeline needs. `/dev/fd` is a subpath because redirections
    // materialise numbered entries under it.
    profile.push_str("  (subpath \"/dev/fd\")\n");
    for dev in [
        "/dev/null",
        "/dev/zero",
        "/dev/random",
        "/dev/urandom",
        "/dev/stdin",
        "/dev/stdout",
        "/dev/stderr",
        "/dev/tty",
        "/dev/dtracehelper",
    ] {
        profile.push_str(&format!("  (literal \"{dev}\")\n"));
    }
    profile.push_str(")\n");
    // ioctl/write on ttys and other char devices, without granting node creation under /dev.
    profile.push_str("(allow file-write-data (subpath \"/dev\"))\n");
    // …then carve the PATH directories and auto-executed configs back out. Last match wins, so
    // this must come after every allow above — including the `/dev` one, or a future writable
    // root added below this line would silently re-open the hole.
    let holes = readonly_holes();
    if !holes.is_empty() {
        profile.push_str("(deny file-write*\n");
        for hole in &holes {
            let p = escape_sbpl(hole);
            // Both forms: `subpath` covers the directory entries (~/.cargo/bin/…), `literal`
            // covers the single-file ones (~/.cargo/config.toml) and the creation of a
            // directory entry at exactly that name. Emitting both means the list does not
            // have to know which of the two each path is.
            profile.push_str(&format!("  (subpath \"{p}\")\n  (literal \"{p}\")\n"));
        }
        profile.push_str(")\n");
    }
    profile
}

#[cfg(target_os = "macos")]
fn seatbelt_available() -> bool {
    Path::new("/usr/bin/sandbox-exec").exists()
}

/// Probe bubblewrap ONCE and cache the verdict.
///
/// `bwrap` can be installed yet unusable — unprivileged user namespaces disabled, a
/// restrictive AppArmor profile, an unusual container runtime. Checking only for the binary
/// would make every command fail. Actually running a trivial sandbox is the only honest test.
#[cfg(target_os = "linux")]
fn bubblewrap_available() -> bool {
    static PROBE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *PROBE.get_or_init(|| {
        let out = crate::process_util::command("bwrap")
            .args([
                "--ro-bind", "/", "/",
                "--dev", "/dev",
                "--proc", "/proc",
                "--",
                "/bin/true",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        matches!(out, Ok(status) if status.success())
    })
}

/// Whether this platform can confine commands at all.
pub fn available() -> bool {
    #[cfg(target_os = "macos")]
    {
        seatbelt_available()
    }
    #[cfg(target_os = "linux")]
    {
        bubblewrap_available()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

// NOTE: there is deliberately no `mechanism()` accessor here. Callers that need to report the
// confinement in effect read `Plan::kind` off the plan they actually ran (see `tasks.rs`), which
// is what happened rather than what is merely available — the two differ whenever a caller passes
// `sandbox: false`, and an accessor that re-probes would quietly contradict the result it labels.

/// Rewrite `shell -lc <command>` into a confined invocation.
///
/// Returns `None` when confinement is unavailable — the caller then runs the command exactly
/// as before. `None` must never be treated as "refuse to run".
pub fn wrap(
    shell: &str,
    shell_args: &[&str],
    command: &str,
    workspace: &Path,
    extra_writable: &[PathBuf],
) -> Option<SandboxPlan> {
    if !available() {
        return None;
    }
    // A workspace that does not resolve to a real directory would produce a profile whose
    // write rules match nothing — every write inside the project would fail. Refuse to build
    // a plan rather than hand back a sandbox that breaks the task.
    let ws = real(workspace);
    if !ws.is_dir() {
        return None;
    }
    let writable = writable_roots(&ws, extra_writable);

    #[cfg(target_os = "macos")]
    {
        let profile = seatbelt_profile(&writable);
        let mut args = vec!["-p".to_string(), profile, shell.to_string()];
        args.extend(shell_args.iter().map(|s| s.to_string()));
        args.push(command.to_string());
        return Some(SandboxPlan {
            program: "/usr/bin/sandbox-exec".to_string(),
            args,
            kind: "seatbelt",
        });
    }

    #[cfg(target_os = "linux")]
    {
        let mut args: Vec<String> = vec![
            "--ro-bind".into(), "/".into(), "/".into(),
            "--dev".into(), "/dev".into(),
            "--proc".into(), "/proc".into(),
        ];
        for root in &writable {
            // Bind only what exists: bwrap aborts on a missing source, which would turn one
            // absent cache directory into "no commands run at all".
            if root.exists() {
                let p = root.to_string_lossy().to_string();
                args.push("--bind".into());
                args.push(p.clone());
                args.push(p);
            }
        }
        // Carve the PATH directories and auto-executed configs back out. Later binds win in
        // bwrap, so this must come after every `--bind` above — same invariant as the SBPL
        // deny block on macOS.
        for hole in readonly_holes() {
            if hole.exists() {
                let p = hole.to_string_lossy().to_string();
                args.push("--ro-bind".into());
                args.push(p.clone());
                args.push(p);
            }
        }
        args.push("--chdir".into());
        args.push(ws.to_string_lossy().to_string());
        args.push("--".into());
        args.push(shell.to_string());
        args.extend(shell_args.iter().map(|s| s.to_string()));
        args.push(command.to_string());
        return Some(SandboxPlan { program: "bwrap".to_string(), args, kind: "bubblewrap" });
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (shell, shell_args, command, writable);
        None
    }
}

/// Does this output look like the sandbox refusing a write, rather than an ordinary failure?
///
/// Used to tell the model (and the user) that retrying verbatim is pointless and the real
/// choice is "narrow the write target, or re-run this one command unconfined" — otherwise a
/// blocked `npm install -g` reads as a mysterious permissions bug and the agent thrashes.
pub fn looks_like_denial(output: &str) -> bool {
    let o = output.to_ascii_lowercase();
    o.contains("operation not permitted")
        || o.contains("sandbox-exec")
        || o.contains("deny file-write")
        || o.contains("bwrap:")
        || o.contains("read-only file system")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profile_allows_the_workspace_and_denies_everything_else_by_default() {
        let profile = seatbelt_profile(&[PathBuf::from("/Users/t/proj")]);
        // Order matters and is the whole correctness argument: allow-all, then deny writes,
        // then re-allow specific targets. Any other order silently allows every write.
        let allow_default = profile.find("(allow default)").unwrap();
        let deny_writes = profile.find("(deny file-write*)").unwrap();
        let allow_writes = profile.find("(allow file-write*").unwrap();
        assert!(allow_default < deny_writes && deny_writes < allow_writes);
        assert!(profile.contains("(subpath \"/Users/t/proj\")"));
    }

    #[test]
    fn profile_paths_are_escaped_so_a_quote_cannot_break_out_of_the_rule() {
        // A workspace directory may legitimately contain a quote or backslash. Unescaped, it
        // would terminate the SBPL string early and change the meaning of the profile.
        let evil = PathBuf::from("/tmp/we\"ird\\path");
        let profile = seatbelt_profile(&[evil.clone()]);
        assert!(profile.contains(r#"(subpath "/tmp/we\"ird\\path")"#));
        assert_eq!(escape_sbpl(&evil), r#"/tmp/we\"ird\\path"#);
    }

    #[test]
    fn package_manager_caches_are_writable_or_npm_install_cannot_run() {
        let home = match home_dir() {
            Some(h) => h,
            None => return,
        };
        let roots = writable_roots(Path::new("/"), &[]);
        for rel in [".npm", ".cargo", ".cache"] {
            assert!(
                roots.contains(&home.join(rel)),
                "{rel} must stay writable — npm/cargo fail outright without it"
            );
        }
    }

    #[test]
    fn a_root_workspace_does_not_make_everything_writable() {
        // 这是真发生过的那条：调用方传 cwd = "/"（市场里"查看仓库"那个按钮就是），
        // profile 的第一条规则于是变成 (allow file-write* (subpath "/"))——全盘可写，
        // ~/.ssh 在内——而 plan 仍然报 kind: "seatbelt"。会撒谎的沙箱比没有沙箱更糟，
        // 因为应用其余部分是照着那个标签做判断的。
        let roots = writable_roots(Path::new("/"), &[]);
        assert!(!roots.iter().any(|r| r == Path::new("/")), "/ 成了可写根");
        // 但沙箱本身不该塌掉：临时目录照旧可写，命令还能跑。
        assert!(!roots.is_empty());
        assert!(roots.iter().any(|r| r.starts_with("/private/tmp") || r.starts_with("/tmp")));
    }

    #[test]
    fn a_home_workspace_does_not_make_everything_writable() {
        let Some(home) = home_dir() else { return };
        let roots = writable_roots(&home, &[]);
        assert!(!roots.iter().any(|r| r == &real(&home)), "$HOME 整个成了可写根");
        // extra 也走同一道闸——否则这个洞只是挪到另一个参数上。
        let roots = writable_roots(&home.join("code/p"), &[PathBuf::from("/")]);
        assert!(!roots.iter().any(|r| r == Path::new("/")), "extra 里的 / 溜进去了");
    }

    #[test]
    fn path_directories_inside_the_writable_caches_are_carved_back_out() {
        let Some(home) = home_dir() else { return };
        let profile = seatbelt_profile(&writable_roots(&home.join("code/p"), &[]));
        // ~/.cargo 整个可写，意味着 ~/.cargo/bin（在 PATH 上）和 ~/.cargo/config.toml
        // （里面的 rustc-wrapper / [alias] 会被下一次 cargo build 执行）都可写——
        // 这正是这个模块声称要挡的持久化。
        let deny_at = profile.rfind("(deny file-write*\n").expect("没有 deny 收尾块");
        let allow_at = profile.find("(allow file-write*").unwrap();
        // 最后匹配者胜：deny 必须排在所有 allow 之后，否则等于没写。
        assert!(deny_at > allow_at, "deny 块排在 allow 前面，SBPL 里等于无效");
        assert!(deny_at > profile.find("(allow file-write-data").unwrap());
        for rel in [".cargo/bin", ".cargo/config.toml", ".gradle/init.d", ".bun/bin", ".m2/settings.xml"] {
            let p = home.join(rel);
            let p = escape_sbpl(&p);
            assert!(profile[deny_at..].contains(&format!("(subpath \"{p}\")")), "{rel} 没被挡");
            assert!(profile[deny_at..].contains(&format!("(literal \"{p}\")")), "{rel} 缺 literal 形式");
        }
        // 缓存本身还得可写，否则 npm install / cargo build 直接跑不了。
        assert!(profile[..deny_at].contains(&format!("(subpath \"{}\")", escape_sbpl(&home.join(".cargo")))));
    }

    #[test]
    fn credential_and_autostart_locations_are_never_writable() {
        let home = match home_dir() {
            Some(h) => h,
            None => return,
        };
        // A realistic workspace, not `/` — the workspace itself is writable by design, so
        // passing a root that CONTAINS the credential paths would only prove the obvious.
        let roots = writable_roots(&home.join("code/project"), &[]);
        // The persistence and credential paths this module exists to block. None of them may
        // appear as a writable root, and none may be a PARENT of one either — a writable
        // `~/.ssh/x` would be just as bad as a writable `~/.ssh`.
        for rel in [".ssh", ".gnupg", ".aws", ".zshrc", ".bashrc", ".profile", "Library/LaunchAgents"] {
            let forbidden = home.join(rel);
            assert!(
                !roots.iter().any(|r| r == &forbidden || r.starts_with(&forbidden)),
                "{rel} must never be writable inside the sandbox"
            );
        }
        // …and no rule may be broad enough to swallow them by accident.
        for broad in ["/", "/usr", "/etc", "/Library"] {
            assert!(
                !roots.iter().any(|r| r == Path::new(broad)),
                "{broad} must never become a writable root"
            );
        }
        assert!(
            !roots.iter().any(|r| r == &home),
            "HOME itself must never be writable — only the curated caches under it"
        );
    }

    #[test]
    fn a_nonexistent_workspace_yields_no_plan_rather_than_a_profile_that_blocks_the_project() {
        let plan = wrap(
            "/bin/sh",
            &["-lc"],
            "echo hi",
            Path::new("/definitely/not/a/real/directory/xyzzy"),
            &[],
        );
        assert!(plan.is_none());
    }

    #[test]
    fn denial_detection_recognizes_the_real_messages_and_ignores_ordinary_failures() {
        assert!(looks_like_denial("/bin/sh: /Users/m/.zshrc: Operation not permitted"));
        assert!(looks_like_denial("bwrap: Can't bind mount"));
        assert!(looks_like_denial("EROFS: read-only file system"));
        assert!(!looks_like_denial("npm ERR! 404 Not Found"));
        assert!(!looks_like_denial("error[E0308]: mismatched types"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_generated_plan_actually_confines_writes_on_this_machine() {
        // End-to-end: build a plan exactly the way the caller does and run it. A profile that
        // parses but does not confine is the failure mode unit tests on strings cannot catch.
        if !available() {
            return;
        }
        let ws = std::env::temp_dir().join(format!("michael-sbtest-{}", std::process::id()));
        std::fs::create_dir_all(&ws).unwrap();
        let outside = std::env::temp_dir().join(format!("michael-sbtest-outside-{}", std::process::id()));
        let _ = std::fs::remove_file(&outside);

        let run = |script: &str| {
            let plan = wrap("/bin/sh", &["-c"], script, &ws, &[]).expect("a plan on macOS");
            crate::process_util::command(&plan.program)
                .args(&plan.args)
                // The profile grants the workspace by absolute path; the callers all set cwd
                // there too, and a relative write in the script needs it to resolve inside.
                .current_dir(&ws)
                .output()
                .expect("sandbox-exec runs")
        };

        // Inside the workspace: allowed.
        let inside = run("echo ok > inside.txt && cat inside.txt");
        assert!(inside.status.success(), "a write inside the workspace must succeed");
        assert_eq!(String::from_utf8_lossy(&inside.stdout).trim(), "ok");

        // Outside it: refused. `/var/folders` is writable wholesale, so probe a path that is
        // genuinely out of bounds — the user's HOME.
        let home_probe = home_dir().unwrap().join(".michael-sbtest-should-not-exist");
        let _ = std::fs::remove_file(&home_probe);
        let escaped = run(&format!("echo pwned > {}", home_probe.display()));
        assert!(!escaped.status.success(), "a write to HOME must be refused");
        assert!(!home_probe.exists(), "and must not have landed");
        assert!(looks_like_denial(&String::from_utf8_lossy(&escaped.stderr)));

        // Reads stay open — this is write confinement, not a jail.
        let read = run("head -c 3 /etc/hosts > read.txt && echo READ-OK");
        assert!(read.status.success(), "reads outside the workspace must still work");

        let _ = std::fs::remove_dir_all(&ws);
        let _ = std::fs::remove_file(&outside);
    }
}
