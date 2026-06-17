//! Lightweight Git integration: repository status and per-file diffs.
//!
//! Rather than linking a Git library we shell out to the user's `git`
//! executable. This keeps the dependency surface small and matches whatever
//! Git the user already has configured (hooks, includes, credentials, etc.).

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A single changed path reported by `git status`.
#[derive(Serialize)]
pub struct GitFile {
    /// Absolute path on disk (the new path for renames).
    path: String,
    /// File name only, for icon + display.
    name: String,
    /// Path relative to the repository root.
    rel: String,
    /// Two-letter porcelain code, e.g. " M", "A ", "??", "MM".
    code: String,
    /// Human label: "Modified", "Added", "Deleted", "Renamed", "Untracked"…
    label: String,
    /// Whether the change is (at least partly) staged in the index.
    staged: bool,
    /// True when the worktree file no longer exists (deleted).
    deleted: bool,
}

/// Result of `git_status`: branch name plus the list of changed files.
#[derive(Serialize)]
pub struct GitStatus {
    /// Whether `root` is inside a Git work tree at all.
    is_repo: bool,
    /// Current branch (or a short detached-HEAD description).
    branch: String,
    files: Vec<GitFile>,
}

fn run_git(root: &str, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        // Never block waiting for an interactive credential prompt — fail fast
        // instead. This keeps network commands like `git push` from hanging
        // indefinitely on an auth prompt when no credential helper is set.
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|e| format!("failed to run git: {e}"))
}

/// Map a porcelain status code to a friendly label.
fn label_for(code: &str) -> &'static str {
    let bytes = code.as_bytes();
    if code == "??" {
        return "Untracked";
    }
    if code == "!!" {
        return "Ignored";
    }
    // Prefer the index (staged) status, falling back to the worktree status.
    let c = if bytes[0] != b' ' && bytes[0] != b'?' {
        bytes[0]
    } else {
        bytes[1]
    };
    match c {
        b'M' => "Modified",
        b'A' => "Added",
        b'D' => "Deleted",
        b'R' => "Renamed",
        b'C' => "Copied",
        b'U' => "Conflict",
        b'T' => "Type changed",
        _ => "Changed",
    }
}

/// Report the status of the repository containing `root`.
///
/// Returns `is_repo: false` (rather than an error) when `root` is not inside a
/// Git work tree, so the frontend can show a tidy empty state.
#[tauri::command]
pub fn git_status(root: String) -> Result<GitStatus, String> {
    let inside = run_git(&root, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() {
        return Ok(GitStatus {
            is_repo: false,
            branch: String::new(),
            files: Vec::new(),
        });
    }

    let branch_out = run_git(&root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let mut branch = String::from_utf8_lossy(&branch_out.stdout)
        .trim()
        .to_string();
    if branch == "HEAD" {
        // Detached HEAD: show the short commit instead.
        if let Ok(short) = run_git(&root, &["rev-parse", "--short", "HEAD"]) {
            let s = String::from_utf8_lossy(&short.stdout).trim().to_string();
            if !s.is_empty() {
                branch = format!("detached @ {s}");
            }
        }
    }
    if branch.is_empty() {
        branch = "(no commits yet)".to_string();
    }

    let status_out = run_git(&root, &["status", "--porcelain=v1"])?;
    if !status_out.status.success() {
        let err = String::from_utf8_lossy(&status_out.stderr)
            .trim()
            .to_string();
        return Err(if err.is_empty() {
            "git status failed".into()
        } else {
            err
        });
    }

    let root_path = PathBuf::from(&root);
    let text = String::from_utf8_lossy(&status_out.stdout);
    let mut files: Vec<GitFile> = Vec::new();
    for line in text.lines() {
        if line.len() < 4 {
            continue;
        }
        let code = line[..2].to_string();
        // Path starts at column 3 (after "XY ").
        let mut rel = line[3..].to_string();
        // Renames/copies are reported as "old -> new"; keep the new path.
        if let Some(idx) = rel.find(" -> ") {
            rel = rel[idx + 4..].to_string();
        }
        let bytes = code.as_bytes();
        let staged = bytes[0] != b' ' && bytes[0] != b'?';
        let abs = root_path.join(&rel);
        let deleted = !abs.exists();
        let name = Path::new(&rel)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| rel.clone());
        files.push(GitFile {
            path: abs.to_string_lossy().to_string(),
            name,
            rel,
            label: label_for(&code).to_string(),
            code,
            staged,
            deleted,
        });
    }
    files.sort_by_key(|f| f.rel.to_lowercase());

    Ok(GitStatus {
        is_repo: true,
        branch,
        files,
    })
}

/// Return the contents of `rel` as of `HEAD`.
///
/// Used as the "original" side of a diff. Returns an empty string when the
/// file does not exist at HEAD (e.g. a newly added or untracked file).
#[tauri::command]
pub fn git_file_head(root: String, rel: String) -> Result<String, String> {
    let spec = format!("HEAD:{rel}");
    let out = run_git(&root, &["show", &spec])?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        // No such path at HEAD → treat as empty (added file).
        Ok(String::new())
    }
}

/// Run a git command and map a non-zero exit into a readable `Err`.
fn run_git_checked(root: &str, args: &[&str]) -> Result<String, String> {
    let out = run_git(root, args)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if err.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err
        };
        Err(if msg.is_empty() {
            format!("git {} failed", args.first().copied().unwrap_or("command"))
        } else {
            msg
        })
    }
}

/// Whether the repo has at least one commit (i.e. `HEAD` resolves).
///
/// `git restore --staged` / `git reset` need a HEAD to diff the index against;
/// a freshly `git init`-ed repo has none, so we fall back to `git rm --cached`.
fn has_head(root: &str) -> bool {
    run_git(root, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Stage a single path (`git add -- <rel>`). Works for new, modified, and
/// deleted files alike.
#[tauri::command]
pub fn git_stage(root: String, rel: String) -> Result<(), String> {
    run_git_checked(&root, &["add", "--", &rel]).map(|_| ())
}

/// Unstage a single path, leaving the worktree changes intact.
///
/// Uses `git restore --staged` normally, but `git rm --cached` when the repo
/// has no commits yet (there is no HEAD to restore from).
#[tauri::command]
pub fn git_unstage(root: String, rel: String) -> Result<(), String> {
    if has_head(&root) {
        run_git_checked(&root, &["restore", "--staged", "--", &rel]).map(|_| ())
    } else {
        run_git_checked(&root, &["rm", "--cached", "--quiet", "--", &rel]).map(|_| ())
    }
}

/// Stage every change in the worktree (`git add -A`).
#[tauri::command]
pub fn git_stage_all(root: String) -> Result<(), String> {
    run_git_checked(&root, &["add", "-A"]).map(|_| ())
}

/// Unstage everything currently in the index.
///
/// Uses `git reset` normally, falling back to `git rm -r --cached .` for a repo
/// with no commits (no HEAD to reset to).
#[tauri::command]
pub fn git_unstage_all(root: String) -> Result<(), String> {
    if has_head(&root) {
        run_git_checked(&root, &["reset", "--quiet"]).map(|_| ())
    } else {
        run_git_checked(&root, &["rm", "-r", "--cached", "--quiet", "."]).map(|_| ())
    }
}

/// Commit the staged changes with `message`. Returns the short hash + subject.
#[tauri::command]
pub fn git_commit(root: String, message: String) -> Result<String, String> {
    let msg = message.trim();
    if msg.is_empty() {
        return Err("Commit message is empty.".into());
    }
    run_git_checked(&root, &["commit", "-m", msg])?;
    // Report the new commit so the UI can show feedback.
    run_git_checked(&root, &["log", "-1", "--pretty=%h %s"])
}

/// Push the current branch to its upstream (`git push`).
///
/// Returns combined stdout/stderr because git writes its progress to stderr
/// even on success.
#[tauri::command]
pub fn git_push(root: String) -> Result<String, String> {
    let out = run_git(&root, &["push"])?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}").trim().to_string();
    if out.status.success() {
        Ok(if combined.is_empty() {
            "Pushed.".into()
        } else {
            combined
        })
    } else {
        Err(if combined.is_empty() {
            "git push failed".into()
        } else {
            combined
        })
    }
}

/// Local branches plus the name of the current branch, for the branch picker.
#[derive(Serialize)]
pub struct GitBranches {
    /// Current branch (empty on a detached HEAD or a repo with no commits).
    current: String,
    /// Local branch names, in `git branch` order.
    branches: Vec<String>,
}

/// List local branches and report which one is checked out.
#[tauri::command]
pub fn git_branches(root: String) -> Result<GitBranches, String> {
    let head = run_git(&root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let mut current = String::from_utf8_lossy(&head.stdout).trim().to_string();
    if current == "HEAD" {
        // Detached HEAD has no branch name.
        current = String::new();
    }
    let out = run_git(&root, &["branch", "--format=%(refname:short)"])?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "git branch failed".into()
        } else {
            err
        });
    }
    let branches = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    Ok(GitBranches { current, branches })
}

/// Switch to `branch`, optionally creating it first (`git checkout [-b]`).
#[tauri::command]
pub fn git_checkout(root: String, branch: String, create: bool) -> Result<(), String> {
    let branch = branch.trim();
    if branch.is_empty() {
        return Err("Branch name is empty.".into());
    }
    if create {
        run_git_checked(&root, &["checkout", "-b", branch]).map(|_| ())
    } else {
        run_git_checked(&root, &["checkout", branch]).map(|_| ())
    }
}

/// A single commit from the log.
#[derive(Serialize)]
pub struct GitLogEntry {
    hash: String,
    short_hash: String,
    author: String,
    date: String,
    message: String,
}

/// Return the last N commits (default 50) from the current branch.
#[tauri::command]
pub fn git_log(root: String, count: Option<usize>) -> Result<Vec<GitLogEntry>, String> {
    let n = count.unwrap_or(50).min(200);
    let format = "%H%n%h%n%an%n%ar%n%s";
    let out = run_git_checked(
        &root,
        &["log", &format!("-{n}"), &format!("--format={format}")],
    )?;
    let lines: Vec<&str> = out.lines().collect();
    let mut entries = Vec::new();
    let mut i = 0;
    while i + 4 < lines.len() {
        entries.push(GitLogEntry {
            hash: lines[i].to_string(),
            short_hash: lines[i + 1].to_string(),
            author: lines[i + 2].to_string(),
            date: lines[i + 3].to_string(),
            message: lines[i + 4].to_string(),
        });
        i += 5;
    }
    Ok(entries)
}

/// Pull the current branch from its upstream (`git pull`).
///
/// Returns combined stdout/stderr because git reports progress on stderr even
/// on success.
#[tauri::command]
pub fn git_pull(root: String) -> Result<String, String> {
    let out = run_git(&root, &["pull"])?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{stdout}\n{stderr}").trim().to_string();
    if out.status.success() {
        Ok(if combined.is_empty() {
            "Already up to date.".into()
        } else {
            combined
        })
    } else {
        Err(if combined.is_empty() {
            "git pull failed".into()
        } else {
            combined
        })
    }
}
