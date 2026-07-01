use std::path::{Path, PathBuf};
use std::process::Command;

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
    add_configured_tasks(root, ".michael/tasks.json", "Michael", &mut out);
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
}

const MAX_TASK_OUTPUT: usize = 2 * 1024 * 1024;
/// Kill a captured command after this long so a server/watch/blocked command
/// can't hang the caller forever (long but enough for slow builds/installs).
const TASK_TIMEOUT_SECS: u64 = 300;

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

#[cfg(not(windows))]
fn augmented_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let extra = format!(
        "{home}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:{home}/go/bin:{home}/.local/bin:/usr/bin:/bin"
    );
    match std::env::var("PATH") {
        Ok(p) if !p.is_empty() => format!("{extra}:{p}"),
        _ => extra,
    }
}

/// Run a discovered task to completion and capture stdout/stderr so the
/// frontend can feed it through a problem matcher into the Problems panel.
/// This is the non-interactive complement to running a task in the terminal.
#[tauri::command]
pub async fn task_run_capture(cwd: String, command: String) -> Result<TaskRunResult, String> {
    // Run the blocking spawn + wait loop on the blocking pool, NOT the Tauri
    // event-loop thread. A sync command here blocks that thread for the command's
    // whole duration (up to the 300s timeout), freezing the whole IDE — the cause
    // of "调用终端容易卡死一会". spawn_blocking keeps the UI responsive throughout.
    tauri::async_runtime::spawn_blocking(move || task_run_capture_inner(cwd, command))
        .await
        .map_err(|e| format!("task thread join failed: {e}"))?
}

fn task_run_capture_inner(cwd: String, command: String) -> Result<TaskRunResult, String> {
    let dir = PathBuf::from(&cwd);
    if !dir.is_dir() {
        return Err("task working directory is not a directory".into());
    }
    if command.trim().is_empty() {
        return Err("empty task command".into());
    }

    #[cfg(windows)]
    let mut cmd = {
        // run_cmd executes through cmd.exe (COMSPEC). Two Windows-only fixes so
        // the agent can actually *read* what happened:
        //   1. `chcp 65001` switches the console to UTF-8 for this child, so
        //      non-ASCII output (Chinese paths / error text) comes back as UTF-8
        //      instead of OEM-codepage (GBK/936) mojibake we can't decode.
        //   2. PYTHONUTF8 / PYTHONIOENCODING make Python tooling emit UTF-8 too.
        // raw_arg keeps cmd metacharacters (& | > "") intact — std's normal arg
        // quoting would mangle them for cmd.exe.
        use std::os::windows::process::CommandExt;
        let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());
        let mut c = crate::process_util::command(shell);
        c.raw_arg(format!("/C chcp 65001>nul & {command}"));
        c.current_dir(&dir)
            .env("PYTHONUTF8", "1")
            .env("PYTHONIOENCODING", "utf-8");
        c
    };
    #[cfg(not(windows))]
    let mut cmd = {
        // A login shell loads the user's profile so cargo/npm/go resolve, and we
        // prepend the usual toolchain dirs as a belt-and-suspenders fallback.
        let mut c = crate::process_util::command(task_shell());
        c.arg("-lc")
            .arg(&command)
            .current_dir(&dir)
            .env("PATH", augmented_path())
            .env("CI", "1")
            .env("TERM", "dumb");
        c
    };
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

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
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(TASK_TIMEOUT_SECS);
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
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
    let mut stdout = String::from_utf8_lossy(&out_bytes).into_owned();
    let mut stderr = String::from_utf8_lossy(&err_bytes).into_owned();
    let mut truncated = truncate_on_boundary(&mut stdout, MAX_TASK_OUTPUT)
        | truncate_on_boundary(&mut stderr, MAX_TASK_OUTPUT);
    if timed_out {
        truncated = true;
        stderr.push_str(&format!(
            "\n[已超时 {TASK_TIMEOUT_SECS}s，命令被终止。长时间运行的命令（如启动服务器）请在终端里手动运行。]"
        ));
    }
    let combined = format!("{stdout}{stderr}");
    Ok(TaskRunResult {
        code,
        stdout,
        stderr,
        combined,
        truncated,
    })
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

    #[cfg(not(windows))]
    #[test]
    fn capture_runs_command_and_collects_output() {
        let root = temp_root("capture");
        let result = task_run_capture_inner(
            root.to_string_lossy().to_string(),
            "echo michael-ide".into(),
        )
        .expect("task should run");
        assert_eq!(result.code, 0);
        assert!(result.combined.contains("michael-ide"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(not(windows))]
    #[test]
    fn capture_reports_nonzero_exit() {
        let root = temp_root("capture-fail");
        let result = task_run_capture_inner(root.to_string_lossy().to_string(), "exit 3".into())
            .expect("task should run");
        assert_eq!(result.code, 3);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn capture_rejects_bad_dir() {
        let err = task_run_capture_inner("/nonexistent-michael-ide-dir-xyz".into(), "echo hi".into());
        assert!(err.is_err());
    }
}
