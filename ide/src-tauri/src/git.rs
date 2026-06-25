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

/// Unified diff of the working tree (or the staged index when `staged=true`)
/// against HEAD, optionally scoped to one path. Returns the raw unified-diff
/// text — empty when there are no changes. Output is capped so a huge diff can't
/// blow up the IPC payload; note that `git diff` never lists *untracked* files
/// (use `git_status` for those).
#[tauri::command]
pub fn git_diff(root: String, rel: Option<String>, staged: Option<bool>) -> Result<String, String> {
    let mut args: Vec<&str> = vec!["diff"];
    if staged.unwrap_or(false) {
        args.push("--cached");
    }
    let rel = rel.unwrap_or_default();
    let rel = rel.trim().to_string();
    if !rel.is_empty() {
        args.push("--");
        args.push(&rel);
    }
    let out = run_git(&root, &args)?;
    if out.status.success() {
        let mut text = String::from_utf8_lossy(&out.stdout).to_string();
        const MAX: usize = 200_000;
        if text.len() > MAX {
            // Truncate on a char boundary.
            let mut end = MAX;
            while end > 0 && !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
            text.push_str("\n…（diff 过大已截断）");
        }
        Ok(text)
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            "git diff failed".into()
        } else {
            err
        })
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
    /// Parent commit hashes (empty for root commit, 2+ for merge commits).
    parents: Vec<String>,
    /// Ref decorations: branch names, tags, HEAD.
    refs: Vec<String>,
}

/// Return the last N commits (default 50) from all branches.
#[tauri::command]
pub fn git_log(root: String, count: Option<usize>) -> Result<Vec<GitLogEntry>, String> {
    let n = count.unwrap_or(50).min(200);
    // %P = parent hashes (space-separated), %D = ref names
    let format = "%H%n%h%n%an%n%ar%n%s%n%P%n%D";
    let out = run_git_checked(
        &root,
        &[
            "log",
            "--all",
            &format!("-{n}"),
            &format!("--format={format}"),
        ],
    )?;
    let lines: Vec<&str> = out.lines().collect();
    let mut entries = Vec::new();
    let mut i = 0;
    // Each commit is 7 lines (%H..%D). The 5 mandatory fields (hash..subject) must
    // be present; the trailing parents/refs lines are optional because
    // `run_git_checked` trims the output's trailing newline, so the OLDEST commit
    // in the page loses its empty `%D` (and empty `%P`) line — without this the
    // loop would silently drop that commit.
    while i + 4 < lines.len() {
        let parents: Vec<String> = lines
            .get(i + 5)
            .map(|l| {
                l.split_whitespace()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let refs: Vec<String> = lines
            .get(i + 6)
            .map(|l| {
                l.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        entries.push(GitLogEntry {
            hash: lines[i].to_string(),
            short_hash: lines[i + 1].to_string(),
            author: lines[i + 2].to_string(),
            date: lines[i + 3].to_string(),
            message: lines[i + 4].to_string(),
            parents,
            refs,
        });
        i += 7;
    }
    Ok(entries)
}

/// List files with merge conflicts (unmerged entries in `git status`).
#[derive(Serialize)]
pub struct ConflictFile {
    path: String,
    rel: String,
    name: String,
}

#[tauri::command]
pub fn git_conflicts(root: String) -> Result<Vec<ConflictFile>, String> {
    let out = run_git(&root, &["diff", "--name-only", "--diff-filter=U"])?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let root_path = PathBuf::from(&root);
    let text = String::from_utf8_lossy(&out.stdout);
    let files: Vec<ConflictFile> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|rel| {
            let abs = root_path.join(rel);
            ConflictFile {
                path: abs.to_string_lossy().to_string(),
                name: Path::new(rel)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| rel.to_string()),
                rel: rel.to_string(),
            }
        })
        .collect();
    Ok(files)
}

/// Get the base, ours, and theirs versions for a conflicted file.
#[derive(Serialize)]
pub struct MergeVersions {
    base: String,
    ours: String,
    theirs: String,
    merged: String,
}

#[tauri::command]
pub fn git_merge_versions(root: String, rel: String) -> Result<MergeVersions, String> {
    let base = run_git(&root, &["show", &format!(":1:{rel}")])
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let ours = run_git(&root, &["show", &format!(":2:{rel}")])
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let theirs = run_git(&root, &["show", &format!(":3:{rel}")])
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let root_path = PathBuf::from(&root);
    let merged = std::fs::read_to_string(root_path.join(&rel)).unwrap_or_default();
    Ok(MergeVersions {
        base,
        ours,
        theirs,
        merged,
    })
}

/// Accept one side of a merge conflict for a file.
#[tauri::command]
pub fn git_resolve_conflict(root: String, rel: String, resolution: String) -> Result<(), String> {
    let root_path = PathBuf::from(&root);
    let file_path = root_path.join(&rel);
    match resolution.as_str() {
        "ours" => {
            let ours = run_git(&root, &["show", &format!(":2:{rel}")])
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .map_err(|e| format!("cannot get ours: {e}"))?;
            std::fs::write(&file_path, &ours).map_err(|e| e.to_string())?;
        }
        "theirs" => {
            let theirs = run_git(&root, &["show", &format!(":3:{rel}")])
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .map_err(|e| format!("cannot get theirs: {e}"))?;
            std::fs::write(&file_path, &theirs).map_err(|e| e.to_string())?;
        }
        "manual" => {
            // File was manually edited; just mark as resolved
        }
        _ => return Err(format!("unknown resolution: {resolution}")),
    }
    run_git_checked(&root, &["add", "--", &rel]).map(|_| ())
}

/// Stash the current working directory changes.
#[tauri::command]
pub fn git_stash(root: String) -> Result<String, String> {
    let out = run_git(&root, &["stash", "push", "-m", "Michael IDE stash"])?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() {
        Ok(if text.is_empty() {
            "No local changes to stash.".into()
        } else {
            text
        })
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Pop a stash entry, applying it and removing it from the stash list.
///
/// Defaults to the most recent stash (`stash@{0}`) when no index is given.
#[tauri::command]
pub fn git_stash_pop(root: String, index: Option<usize>) -> Result<String, String> {
    let spec = index.map(|i| format!("stash@{{{i}}}"));
    let mut args: Vec<&str> = vec!["stash", "pop"];
    if let Some(ref s) = spec {
        args.push(s.as_str());
    }
    let out = run_git(&root, &args)?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        Ok(if text.is_empty() {
            "Stash applied.".into()
        } else {
            text
        })
    } else {
        Err(if err.is_empty() {
            "git stash pop failed".into()
        } else {
            err
        })
    }
}

/// Apply a stash entry without removing it from the stash list.
#[tauri::command]
pub fn git_stash_apply(root: String, index: usize) -> Result<String, String> {
    let spec = format!("stash@{{{index}}}");
    let out = run_git(&root, &["stash", "apply", &spec])?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        Ok(if text.is_empty() {
            "Stash applied.".into()
        } else {
            text
        })
    } else {
        Err(if err.is_empty() {
            "git stash apply failed".into()
        } else {
            err
        })
    }
}

/// Drop (delete) a stash entry without applying it.
#[tauri::command]
pub fn git_stash_drop(root: String, index: usize) -> Result<String, String> {
    let spec = format!("stash@{{{index}}}");
    let out = run_git(&root, &["stash", "drop", &spec])?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        Ok(if text.is_empty() {
            "Stash dropped.".into()
        } else {
            text
        })
    } else {
        Err(if err.is_empty() {
            "git stash drop failed".into()
        } else {
            err
        })
    }
}

/// List stash entries.
#[tauri::command]
pub fn git_stash_list(root: String) -> Result<Vec<String>, String> {
    let out = run_git(&root, &["stash", "list"])?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect())
}

/// Show git blame for a file (line-by-line author + commit info).
#[derive(Serialize)]
pub struct BlameLine {
    pub commit: String,
    pub author: String,
    pub date: String,
    pub line: usize,
}

#[tauri::command]
pub fn git_blame(root: String, rel: String) -> Result<Vec<BlameLine>, String> {
    let out = run_git(&root, &["blame", "--porcelain", "--", &rel])?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut result = Vec::new();
    let mut cur_commit = String::new();
    let mut cur_author = String::new();
    let mut cur_date = String::new();
    let mut cur_line = 0usize;
    for line in text.lines() {
        if let Some(a) = line.strip_prefix("author ") {
            cur_author = a.to_string();
        } else if let Some(d) = line.strip_prefix("author-time ") {
            cur_date = d.to_string();
        } else if !line.starts_with('\t') && !line.is_empty() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0].len() == 40 {
                cur_commit = parts[0].to_string();
                cur_line = parts[2].parse().unwrap_or(0);
            }
        } else if line.starts_with('\t') && !cur_commit.is_empty() {
            result.push(BlameLine {
                commit: cur_commit[..8.min(cur_commit.len())].to_string(),
                author: cur_author.clone(),
                date: cur_date.clone(),
                line: cur_line,
            });
        }
    }
    Ok(result)
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
