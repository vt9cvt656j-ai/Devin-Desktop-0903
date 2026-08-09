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

/// Every directory a sandboxed command may write to.
fn writable_roots(workspace: &Path, extra: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = vec![real(workspace)];
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
        roots.push(real(p));
    }
    roots.sort();
    roots.dedup();
    roots
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
        let out = std::process::Command::new("bwrap")
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
            std::process::Command::new(&plan.program)
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
