//! Lightweight Git integration: repository status and per-file diffs.
//!
//! Rather than linking a Git library we shell out to the user's `git`
//! executable. This keeps the dependency surface small and matches whatever
//! Git the user already has configured (hooks, includes, credentials, etc.).

use crate::files::require_inside_workspace;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

/// Verify that joining `root` and `rel` stays inside `root` after resolving
/// `..` and other components.  Prevents path-traversal writes via crafted `rel`.
fn require_rel_inside_root(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let joined = root.join(rel);
    let mut resolved = root.to_path_buf();
    for component in joined
        .strip_prefix(root)
        .unwrap_or(Path::new(rel))
        .components()
    {
        match component {
            Component::Normal(s) => resolved.push(s),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.starts_with(root) || resolved == root.to_path_buf() {
                    return Err(format!(
                        "path traversal denied: '{rel}' escapes repository root"
                    ));
                }
                resolved.pop();
                if !resolved.starts_with(root) {
                    return Err(format!(
                        "path traversal denied: '{rel}' escapes repository root"
                    ));
                }
            }
            _ => return Err(format!("disallowed path component in '{rel}'")),
        }
    }
    if !resolved.starts_with(root) {
        return Err(format!(
            "path traversal denied: '{rel}' escapes repository root"
        ));
    }
    Ok(resolved)
}

/// Reject a `root` that is not inside ALLOWED_ROOTS before any Git operation
/// runs. This prevents the frontend from pointing `git -C` at an arbitrary
/// directory on the user's machine.
fn require_git_root(root: &str, is_write: bool) -> Result<(), String> {
    require_inside_workspace(root, is_write)?;
    Ok(())
}

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
    // Resolve `git` through the augmented PATH (Homebrew /opt/homebrew/bin, /usr/local/bin,
    // Xcode) and pass that PATH to the child — so a Finder/Dock-launched app (whose inherited
    // PATH is the minimal /usr/bin:/bin) can still FIND git AND its credential helper
    // (git-credential-osxkeychain). Every other subprocess (lsp/dap/capture) already does this;
    // run_git was the one git-critical caller spawning bare "git" → commit/push "传不过去" on
    // Macs where git lives only under Homebrew.
    // 用 system 解析：git 是 IDE 自己的基础设施，不能被仓库里的 node_modules/.bin/git
    // 顶掉（那等于打开一个文件夹就执行它带的程序）。项目自带工具链的解析不走这条路。
    let git = crate::process_util::resolve_system_command("git");
    let mut command = crate::process_util::command(&git);
    command
        .arg("-C")
        .arg(root)
        .args(args)
        // 子进程的 PATH 同样不含工作区目录：git 会去 PATH 里找 credential helper、
        // ssh、以及 core.pager 之类，仓库不该有机会替换掉其中任何一个。
        .env("PATH", crate::process_util::augmented_path(None))
        // Never block waiting for an interactive credential prompt — fail fast
        // instead. This keeps network commands like `git push` from hanging
        // indefinitely on an auth prompt when no credential helper is set.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "Never")
        // 锁英文。git 会**本地化**它的 fatal 信息——中文环境下 `not a git repository`
        // 印出来的是「不是 git 仓库」。任何拿英文子串去区分"不是仓库"和"git 跑不起来"的
        // 判断，在中文机器上都会全线失配，把每个正常的非仓库目录报成"git 坏了"。
        .env("LC_ALL", "C")
        .env("LC_MESSAGES", "C");
    // SSH has its own prompts, independent of GIT_TERMINAL_PROMPT. Keep any
    // user-supplied identity/wrapper command, but append non-interactive options.
    // `accept-new` permits first contact while still rejecting changed host keys.
    let ssh = std::env::var("GIT_SSH_COMMAND").unwrap_or_else(|_| "ssh".into());
    command.env(
        "GIT_SSH_COMMAND",
        format!("{ssh} -oBatchMode=yes -oStrictHostKeyChecking=accept-new"),
    );
    command
        .output()
        .map_err(|e| format!("failed to run git: {e}"))
}

/// Map a porcelain status code to a friendly label.
/// 还原 git 的引号路径（`core.quotepath` 或路径含特殊字符时出现）。
///
/// 形如 `"a\tb\344\270\255.txt"`：外层双引号 + C 风格转义 + 非 ASCII 字节的八进制。
/// 非引号形式原样返回。
fn unquote_git_path(raw: &str) -> String {
    if !(raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2) {
        return raw.to_string();
    }
    let body = &raw[1..raw.len() - 1];
    let mut out: Vec<u8> = Vec::with_capacity(body.len());
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        i += 1;
        if i >= bytes.len() {
            break;
        }
        match bytes[i] {
            b'a' => {
                out.push(0x07);
                i += 1;
            }
            b'b' => {
                out.push(0x08);
                i += 1;
            }
            b'f' => {
                out.push(0x0c);
                i += 1;
            }
            b'n' => {
                out.push(b'\n');
                i += 1;
            }
            b'r' => {
                out.push(b'\r');
                i += 1;
            }
            b't' => {
                out.push(b'\t');
                i += 1;
            }
            b'v' => {
                out.push(0x0b);
                i += 1;
            }
            b'"' => {
                out.push(b'"');
                i += 1;
            }
            b'\\' => {
                out.push(b'\\');
                i += 1;
            }
            b'0'..=b'7' => {
                // 三位八进制，逐字节还原后整体按 UTF-8 解释。
                let mut v = 0u32;
                let mut n = 0;
                while n < 3 && i < bytes.len() && (b'0'..=b'7').contains(&bytes[i]) {
                    v = v * 8 + u32::from(bytes[i] - b'0');
                    i += 1;
                    n += 1;
                }
                out.push((v & 0xff) as u8);
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

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
// Every Git command below launches the synchronous git executable. Force Tauri's
// async dispatch so filesystem scans, hooks, credential prompts, and network
// operations never run on the window event thread.
#[tauri::command(async)]
pub fn git_status(root: String) -> Result<GitStatus, String> {
    require_git_root(&root, false)?;
    let inside = run_git(&root, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() {
        // 「git 说这里不是仓库」和「git 根本跑不起来」是两件完全不同的事，而这里原来
        // 一律报成 is_repo:false。后果是：模型收到"这不是 Git 仓库"，于是它要么去
        // git init 一个**已经存在**的仓库，要么在报告里写下"当前目录没有 Git 仓库"
        // 这个**假结论**——而真相是这台机器上 git 没装、或者 xcode-select 没配好。
        // 重试任何 git_* 工具都没有意义，而模型不知道这一点。
        let stderr = String::from_utf8_lossy(&inside.stderr).to_lowercase();
        let really_not_a_repo = stderr.contains("not a git repository")
            || stderr.contains("not a git repo")
            // 上面 run_git 锁了 LC_ALL=C，正常情况下拿到的一定是英文；这两条是给
            // LC_ALL 被外部环境强行覆盖的极端情况留的兜底。
            || stderr.contains("不是 git 仓库")
            || stderr.contains("不是git仓库");
        if !really_not_a_repo {
            let detail = String::from_utf8_lossy(&inside.stderr).trim().to_string();
            return Err(format!(
                "[GIT_UNAVAILABLE] git 在这台机器上跑不起来（原文：{}）。\
                 这**不是**仓库的问题——重试 git_* 工具没有意义。\
                 先 run_cmd 跑 `git --version` 确认；macOS 上多半要 `xcode-select --install`。\
                 装好之前用 read_file / search 做调查，收尾时明说 Git 相关的验证没做成。",
                if detail.is_empty() { "无输出".into() } else { detail }
            ));
        }
        return Ok(GitStatus {
            is_repo: false,
            branch: String::new(),
            files: Vec::new(),
        });
    }

    // `rev-parse --abbrev-ref HEAD` exits 128 and prints the literal "HEAD" on
    // an unborn branch. symbolic-ref reads the ref itself, so it reports the real
    // branch name both before and after the first commit.
    let symbolic = run_git(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    let branch = if symbolic.status.success() {
        let name = String::from_utf8_lossy(&symbolic.stdout).trim().to_string();
        if name.is_empty() {
            "(no commits yet)".to_string()
        } else {
            name
        }
    } else {
        // A non-symbolic HEAD is detached. Show the commit when it resolves.
        let short = run_git(&root, &["rev-parse", "--verify", "--short", "HEAD"])?;
        let commit = String::from_utf8_lossy(&short.stdout).trim().to_string();
        if short.status.success() && !commit.is_empty() {
            format!("detached @ {commit}")
        } else {
            "(no commits yet)".to_string()
        }
    };

    // `core.quotepath=false` 让 git 直接输出 UTF-8 路径，而不是 `"\344\270\255..."`
    // 这种八进制转义形式。含引号/反斜杠/控制字符的路径仍会被引号包起来，由下面的
    // unquote_git_path 兜底。
    let status_out = run_git(
        &root,
        &["-c", "core.quotepath=false", "status", "--porcelain=v1"],
    )?;
    if !status_out.status.success() {
        // 和 stash 那四处同一个形状：只看 stderr，stdout 里的东西被整个扔掉。
        let err = String::from_utf8_lossy(&status_out.stderr)
            .trim()
            .to_string();
        let text = String::from_utf8_lossy(&status_out.stdout).trim().to_string();
        return Err(git_failure_text(&text, &err, "git status failed"));
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
        // git 会把含特殊字符的路径用引号包起来并做 C 风格转义。不解开的话拼出来的绝对
        // 路径根本不存在 → 下面的 `!abs.exists()` 判成"已删除"，而且前端拿这个带引号的
        // 路径去暂存/看 diff 也一律失败。
        rel = unquote_git_path(&rel);
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
#[tauri::command(async)]
pub fn git_file_head(root: String, rel: String) -> Result<String, String> {
    require_git_root(&root, false)?;
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
#[tauri::command(async)]
pub fn git_diff(root: String, rel: Option<String>, staged: Option<bool>) -> Result<String, String> {
    require_git_root(&root, false)?;
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
        let low = err.to_lowercase();
        if low.contains("not a git repository") || low.contains("--no-index") {
            return Err(
                "当前目录不是 Git 仓库（没有 .git）。要用版本控制先在此目录执行 `git init`。"
                    .into(),
            );
        }
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Err(git_failure_text(&text, &err, "git diff failed"))
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

fn validated_clone_source(source: &str) -> Result<String, String> {
    let source = source.trim();
    if source.is_empty() {
        return Err("Clone source is empty.".into());
    }
    if source.chars().any(char::is_control) || source.starts_with('-') {
        return Err("Clone source contains unsafe characters.".into());
    }

    let local = Path::new(source);
    if local.is_absolute() {
        return std::fs::canonicalize(local)
            .map(|path| path.to_string_lossy().into_owned())
            .map_err(|e| format!("Local clone source does not exist or is inaccessible: {e}"));
    }

    if source.chars().any(char::is_whitespace) || source.contains("::") {
        return Err(
            "Clone source must be a supported URL, SSH path, or absolute local path.".into(),
        );
    }
    let lower = source.to_ascii_lowercase();
    if ["https://", "http://", "ssh://", "git://", "file://"]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return Ok(source.to_string());
    }
    if source.contains("://") {
        return Err("Unsupported Git clone URL scheme.".into());
    }

    // Git's SCP-style SSH syntax: [user@]host:path. Restrict the host side to
    // hostname characters so remote-helper syntax cannot be smuggled through.
    if let Some((host, path)) = source.split_once(':') {
        let valid_host = !host.is_empty()
            && host
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'.' | b'-' | b'_' | b'@'));
        if valid_host && !path.is_empty() {
            return Ok(source.to_string());
        }
    }
    Err("Clone source must be a supported URL, SSH path, or absolute local path.".into())
}

fn validated_clone_target(target: &str) -> Result<PathBuf, String> {
    let target = target.trim();
    if target.is_empty() {
        return Err("Clone target is empty.".into());
    }
    if target.chars().any(char::is_control) {
        return Err("Clone target contains unsafe characters.".into());
    }
    let target = PathBuf::from(target);
    if !target.is_absolute() {
        return Err("Clone target must be an absolute path.".into());
    }
    ensure_clone_target_absent(&target)?;
    let name = match target.components().next_back() {
        Some(Component::Normal(name)) => name.to_owned(),
        _ => return Err("Clone target must name a new directory.".into()),
    };
    let parent = target
        .parent()
        .ok_or_else(|| "Clone target has no parent directory.".to_string())?;
    let parent = std::fs::canonicalize(parent)
        .map_err(|e| format!("Clone target parent does not exist or is inaccessible: {e}"))?;
    if !parent.is_dir() {
        return Err("Clone target parent is not a directory.".into());
    }
    let normalized = parent.join(name);
    ensure_clone_target_absent(&normalized)?;
    Ok(normalized)
}

fn ensure_clone_target_absent(target: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(target) {
        Ok(_) => Err("Clone target already exists; choose a new directory.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Unable to inspect clone target: {error}")),
    }
}

/// Clone a repository into a new, explicitly chosen absolute directory.
///
/// The parent must already exist and the target itself must not. Sources are
/// limited to normal Git network URLs, SCP-style SSH paths, and existing absolute
/// local paths; dangerous remote-helper schemes are rejected before Git runs.
#[tauri::command(async)]
pub fn git_clone(source: String, target: String) -> Result<String, String> {
    require_inside_workspace(&target, true)?;
    let source = validated_clone_source(&source)?;
    let target = validated_clone_target(&target)?;
    let parent = target
        .parent()
        .ok_or_else(|| "Clone target has no parent directory.".to_string())?;
    let parent = parent.to_string_lossy().into_owned();
    let target_arg = target.to_string_lossy().into_owned();
    run_git_checked(
        &parent,
        &[
            "-c",
            "protocol.ext.allow=never",
            "clone",
            "--",
            &source,
            &target_arg,
        ],
    )?;
    let inside = run_git(&target_arg, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() || String::from_utf8_lossy(&inside.stdout).trim() != "true" {
        return Err("Git clone completed without creating a valid working tree.".into());
    }
    Ok(target_arg)
}

fn select_push_remote(remotes: &[String]) -> Result<String, String> {
    if remotes.iter().any(|remote| remote == "origin") {
        return Ok("origin".into());
    }
    match remotes {
        [] => Err("Current branch has no upstream and this repository has no remotes.".into()),
        [only] => Ok(only.clone()),
        many => Err(format!(
            "Current branch has no upstream and no `origin` remote; choose one of: {}",
            many.join(", ")
        )),
    }
}

/// best-of-N 隔离：建一个 git worktree，让一个并行候选在独立工作树里改仓库、不碰主 checkout。
/// 返回 worktree 的绝对路径。工作树放在 `<root>/.mrdayone/worktrees/<name>`，挂一条临时分支
/// `michael/bon-<name>`（HEAD 派生）。同名残留先强制清掉。**撤销该候选 = git_worktree_remove。**
/// 注意：worktree 不含被 gitignore 的依赖（如 node_modules）——上层若要在里面跑测试，需自行
/// symlink/复用主仓库的依赖（前端 flow 已写明）。
#[tauri::command(async)]
pub fn git_worktree_add(root: String, name: String) -> Result<String, String> {
    require_git_root(&root, true)?;
    let safe: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if safe.is_empty() {
        return Err("invalid worktree name (need [A-Za-z0-9_-])".into());
    }
    let inside = run_git(&root, &["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() {
        return Err("不是 git 仓库——best-of-N 的并行隔离需要 git worktree；先 `git init` 或改用基于 checkpoint 的顺序尝试。".into());
    }
    let abs = format!("{}/.mrdayone/worktrees/{}", root.trim_end_matches('/'), safe);
    let branch = format!("michael/bon-{safe}");

    // 同名残留原来是这么"清掉"的：
    //     let _ = run_git(&root, &["worktree", "remove", "--force", &abs]);
    //     let _ = run_git(&root, &["branch", "-D", &branch]);
    // 注释写的是「避免 add 失败」，实际做的是**无声销毁上一个同名候选**——`--force` 连
    // 未提交的改动一起删，`-D` 连没合并的提交一起丢。而重名恰恰是最常发生的事：模型重试
    // 一步、或者换个思路重来，都会拿同一个 slug（cand-a）再 add 一次，几十分钟的工作就
    // 这么没了，还返回成功。best-of-N 的整个价值就是"候选留在那儿等你挑"，这段代码把它
    // 反过来了。
    //
    // 现在：能复用就复用，不能复用就报错，一律不删任何文件。
    // prune 只清理"登记还在、目录已经没了"的簿记，不碰磁盘上的东西，是安全的。
    let _ = run_git(&root, &["worktree", "prune"]);

    if std::path::Path::new(&abs).exists() {
        let listed = run_git(&root, &["worktree", "list", "--porcelain"])
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let registered = listed
            .lines()
            .filter_map(|l| l.strip_prefix("worktree "))
            .any(|p| p.trim() == abs);
        if registered {
            return Ok(abs); // 上次那个候选还在——交回去接着做，别推倒重来
        }
        return Err(format!(
            "{abs} 已经存在，但它不是一个 git worktree。没有动它——里面可能有要留的东西。\
             换一个候选名，或者自己确认过之后再删掉这个目录。"
        ));
    }

    // 目录不在，但分支可能还在（上一个候选被 remove 掉了，分支删除是 best-effort）。
    // 这时候用 `-b` 会因为分支已存在直接失败，用 `-B` 会把那个分支上已有的提交丢掉。
    // 挂上已有分支：上次的提交还在，接着往下做。
    let branch_exists = run_git(&root, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
        .map(|o| o.status.success())
        .unwrap_or(false);
    if branch_exists {
        run_git_checked(&root, &["worktree", "add", &abs, &branch])?;
    } else {
        run_git_checked(&root, &["worktree", "add", "-b", &branch, &abs, "HEAD"])?;
    }
    Ok(abs)
}

/// 列出当前所有 worktree（porcelain 文本）。
#[tauri::command(async)]
pub fn git_worktree_list(root: String) -> Result<String, String> {
    require_git_root(&root, false)?;
    run_git_checked(&root, &["worktree", "list", "--porcelain"])
}

/// 看一个**历史提交**：提交本身，或者某个文件在那个提交时的样子。
///
/// 缺了这个，整套 git 工具只能看「现在」。定位一个回归的标准路径是
/// git_log 找到可疑提交 → 看那个提交改了什么 → 看改之前那个文件长什么样，
/// 而这条路走到第二步就断头了：git_diff 只比较工作区，git_blame 只给行级归属。
/// 模型于是只能绕道 run_cmd 跑 `git show`——那条路要过审批、输出不受这里的截断保护，
/// 而且它本来就该是个一等工具。
///
/// `rev` 允许分支名和 tag（不只是十六进制哈希）：定位回归时 `main~3`、`v1.2.0`
/// 和 `HEAD^` 一样常用。但仍然逐字符白名单，杜绝把参数喂成 git 的选项或 shell 元字符。
#[tauri::command(async)]
pub fn git_show(root: String, rev: String, rel: Option<String>) -> Result<String, String> {
    require_git_root(&root, false)?;
    let rev = rev.trim().to_string();
    if rev.is_empty() {
        return Err("需要给出提交（哈希 / 分支名 / tag / HEAD~2 之类）".into());
    }
    // 白名单而不是黑名单：允许 [A-Za-z0-9_./-] 加 ^~@{}，其余一律拒。
    // 特别要挡住开头的 '-'，否则 rev 会被 git 当成选项。
    if rev.starts_with('-')
        || !rev
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./-^~@{}".contains(c))
    {
        return Err(format!(
            "非法的版本号「{rev}」。只允许字母数字和 _ . / - ^ ~ @ {{ }}（例：a1b2c3d、main~3、v1.2.0、HEAD^）"
        ));
    }

    let rel = rel.unwrap_or_default().trim().to_string();
    let out = if rel.is_empty() {
        // 提交本身：元信息 + 每文件统计 + 完整 patch。
        run_git(&root, &["show", "--stat", "--patch", "--no-color", &rev])?
    } else {
        if rel.contains("..") {
            return Err("路径不能包含 ..".into());
        }
        run_git(&root, &["show", "--no-color", &format!("{rev}:{rel}")])?
    };

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        // git 的原话对模型比"失败了"有用得多，但要补上下一步。
        return Err(if rel.is_empty() {
            format!("{err}\n（先用 git_log 拿到真实的提交哈希；分支名和 tag 也行。）")
        } else {
            format!("{err}\n（路径要相对仓库根，且必须是那个提交里**当时**存在的路径——文件后来改过名的话，用 git_log 看重命名历史。）")
        });
    }

    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    const MAX: usize = 200_000;
    if text.len() > MAX {
        let mut end = MAX;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n…（内容过大已截断；想看单个文件就带上 rel 参数）");
    }
    Ok(text)
}

/// best-of-N 的候选工作树都建在这里（见 `git_worktree_add`）。remove 只允许动这个目录。
const BON_WORKTREE_DIR: &str = ".mrdayone/worktrees";

/// 把调用方给的 path 收敛成「**我们自己建的**那一个候选工作树」，否则拒绝。
///
/// 原来 remove 是把 path 原样交给 `git worktree remove --force`，一点校验都没有。
/// 而这个工具在策略表里不需要审批、path 又来自模型（仓库里的文本能诱导它），于是
/// 「用户自己 `git worktree add ../feature-x` 挂的、里面有一天没提交的改动」的那棵树，
/// 一句 --force 就没了 —— `--force` 的语义正是「有未提交改动也删」。
///
/// add 那边只会建在 `{root}/.mrdayone/worktrees/{[A-Za-z0-9_-]+}`，所以 remove 的合法
/// 输入集合就这么大。两边用同一条规则，不给它们漂开的机会。
fn bon_worktree_path(root: &str, path: &str) -> Result<(String, String), String> {
    let seg = path
        .trim_end_matches('/')
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("");
    let safe: String = seg
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if safe.is_empty() || safe != seg {
        return Err(format!(
            "只能移除本工具自己建的候选工作树（{BON_WORKTREE_DIR}/<名字>），拒绝：'{path}'"
        ));
    }
    let root_trimmed = root.trim_end_matches('/');
    let expected = format!("{root_trimmed}/{BON_WORKTREE_DIR}/{safe}");
    // 允许调用方传绝对路径或只传名字，但两者都必须归到同一个位置上。
    let given = if path.contains('/') || path.contains('\\') {
        path.trim_end_matches('/').replace('\\', "/")
    } else {
        expected.clone()
    };
    if given != expected {
        return Err(format!(
            "只能移除本工具自己建的候选工作树（{BON_WORKTREE_DIR}/<名字>），拒绝：'{path}'"
        ));
    }
    Ok((expected, safe))
}

/// 移除一个 worktree（并尽力删掉它的临时分支）= 丢弃这个候选。
#[tauri::command(async)]
pub fn git_worktree_remove(root: String, path: String) -> Result<String, String> {
    require_git_root(&root, true)?;
    let (abs, safe) = bon_worktree_path(&root, &path)?;
    run_git_checked(&root, &["worktree", "remove", "--force", &abs])?;
    // 临时分支名由**校验过的**末段推回，而不是从原始输入里切。
    let _ = run_git(&root, &["branch", "-D", &format!("michael/bon-{safe}")]);
    Ok("removed".into())
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
#[tauri::command(async)]
pub fn git_stage(root: String, rel: String) -> Result<(), String> {
    require_git_root(&root, true)?;
    run_git_checked(&root, &["add", "--", &rel]).map(|_| ())
}

/// Unstage a single path, leaving the worktree changes intact.
///
/// Uses `git restore --staged` normally, but `git rm --cached` when the repo
/// has no commits yet (there is no HEAD to restore from).
#[tauri::command(async)]
pub fn git_unstage(root: String, rel: String) -> Result<(), String> {
    require_git_root(&root, true)?;
    if has_head(&root) {
        run_git_checked(&root, &["restore", "--staged", "--", &rel]).map(|_| ())
    } else {
        run_git_checked(&root, &["rm", "--cached", "--quiet", "--", &rel]).map(|_| ())
    }
}

/// Stage every change in the worktree (`git add -A`).
#[tauri::command(async)]
pub fn git_stage_all(root: String) -> Result<(), String> {
    require_git_root(&root, true)?;
    run_git_checked(&root, &["add", "-A"]).map(|_| ())
}

/// Unstage everything currently in the index.
///
/// Uses `git reset` normally, falling back to `git rm -r --cached .` for a repo
/// with no commits (no HEAD to reset to).
#[tauri::command(async)]
pub fn git_unstage_all(root: String) -> Result<(), String> {
    require_git_root(&root, true)?;
    if has_head(&root) {
        run_git_checked(&root, &["reset", "--quiet"]).map(|_| ())
    } else {
        run_git_checked(&root, &["rm", "-r", "--cached", "--quiet", "."]).map(|_| ())
    }
}

/// Commit the staged changes with `message`. Returns the short hash + subject.
#[tauri::command(async)]
pub fn git_commit(root: String, message: String) -> Result<String, String> {
    require_git_root(&root, true)?;
    let msg = message.trim();
    if msg.is_empty() {
        return Err("Commit message is empty.".into());
    }
    run_git_checked(&root, &["commit", "-m", msg])?;
    // Report the new commit so the UI can show feedback.
    run_git_checked(&root, &["log", "-1", "--pretty=%h %s"])
}

/// Push the current branch to its upstream. When the branch has no upstream,
/// prefer `origin`, otherwise use the repository's sole remote, and establish
/// tracking with `--set-upstream`.
///
/// Returns combined stdout/stderr because git writes its progress to stderr
/// even on success.
#[tauri::command(async)]
pub fn git_push(root: String) -> Result<String, String> {
    require_git_root(&root, true)?;
    let symbolic = run_git(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    let branch = String::from_utf8_lossy(&symbolic.stdout).trim().to_string();
    if !symbolic.status.success() || branch.is_empty() {
        return Err("Cannot push from detached HEAD; check out a branch first.".into());
    }

    let upstream = run_git(
        &root,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    )?;
    let out = if upstream.status.success()
        && !String::from_utf8_lossy(&upstream.stdout).trim().is_empty()
    {
        run_git(&root, &["push"])?
    } else {
        let remote_out = run_git(&root, &["remote"])?;
        if !remote_out.status.success() {
            let error = String::from_utf8_lossy(&remote_out.stderr)
                .trim()
                .to_string();
            return Err(if error.is_empty() {
                "Unable to list Git remotes before push.".into()
            } else {
                error
            });
        }
        let remotes: Vec<String> = String::from_utf8_lossy(&remote_out.stdout)
            .lines()
            .map(str::trim)
            .filter(|remote| !remote.is_empty())
            .map(str::to_string)
            .collect();
        let remote = select_push_remote(&remotes)?;
        run_git(&root, &["push", "--set-upstream", "--", &remote, &branch])?
    };
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
#[tauri::command(async)]
pub fn git_branches(root: String) -> Result<GitBranches, String> {
    require_git_root(&root, false)?;
    let head = run_git(&root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let mut current = String::from_utf8_lossy(&head.stdout).trim().to_string();
    if current == "HEAD" {
        // Detached HEAD has no branch name.
        current = String::new();
    }
    let out = run_git(&root, &["branch", "--format=%(refname:short)"])?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        return Err(git_failure_text(&text, &err, "git branch failed"));
    }
    let branches = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    Ok(GitBranches { current, branches })
}

/// Switch to `branch`, optionally creating it first (`git checkout [-b]`).
#[tauri::command(async)]
pub fn git_checkout(root: String, branch: String, create: bool) -> Result<(), String> {
    require_git_root(&root, true)?;
    let branch = branch.trim();
    if branch.is_empty() {
        return Err("Branch name is empty.".into());
    }
    if create {
        run_git_checked(&root, &["checkout", "-b", branch, "--"]).map(|_| ())
    } else {
        // Trailing `--` marks the end of revisions so a branch whose name also
        // matches a file in the tree resolves to the branch (git errors on the
        // ambiguous `git checkout <name>` form otherwise).
        run_git_checked(&root, &["checkout", branch, "--"]).map(|_| ())
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

/// Return the last N commits (default 50).
///
/// `all` 默认 **false**，只看当前分支。原来这里写死 `--all`，于是提交历史是
/// 「所有分支按时间穿插」的一张表——分支装饰只出现在各分支的顶端，中间的行
/// 长得一模一样。画分支图的面板要的正是这个，但智能体问「刚才这条线上改了什么」
/// 「这个回归是哪一笔引进来的」时，它拿到的是一份混着别人分支的清单，挑出来的
/// 提交可能根本不是 HEAD 的祖先——接着 git_show 它、照着不在工作区里的代码推理。
/// 默认取安全的那个：图形面板显式传 all=true，其余调用方忘了传也不会被带偏。
#[tauri::command(async)]
pub fn git_log(
    root: String,
    count: Option<usize>,
    all: Option<bool>,
) -> Result<Vec<GitLogEntry>, String> {
    require_git_root(&root, false)?;
    let n = count.unwrap_or(50).min(200);
    // %P = parent hashes (space-separated), %D = ref names
    let format = "%H%n%h%n%an%n%ar%n%s%n%P%n%D";
    let mut args: Vec<String> = vec!["log".into()];
    if all.unwrap_or(false) {
        args.push("--all".into());
    }
    args.push(format!("-{n}"));
    args.push(format!("--format={format}"));
    let out = run_git_checked(&root, &args.iter().map(|s| s.as_str()).collect::<Vec<_>>())?;
    Ok(parse_log_entries(&out))
}

/// Parse `--format=%H%n%h%n%an%n%ar%n%s%n%P%n%D` output into entries.
///
/// Shared by `git_log` and `git_file_log` so the timeline and the history list can
/// never disagree about how a commit is read.
fn parse_log_entries(out: &str) -> Vec<GitLogEntry> {
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
    entries
}

/// The contents of one file at one revision — what the Timeline opens when a commit
/// is clicked.
///
/// `git_file_head` only ever reads HEAD, so it cannot show an older version. A path
/// that did not exist at that revision is not an error here: it is how an "added"
/// commit looks, and the diff pane renders it as an empty left-hand side.
#[tauri::command(async)]
pub fn git_file_at(root: String, rel: String, rev: String) -> Result<String, String> {
    require_git_root(&root, false)?;
    if rel.trim().is_empty() || rev.trim().is_empty() {
        return Ok(String::new());
    }
    // `rev` comes from a commit hash we ourselves printed, but treat it as untrusted
    // anyway: anything outside hex/^~ can't be a revision we produced, and refusing
    // early keeps shell-ish surprises out of the argument list.
    if !rev
        .chars()
        .all(|c| c.is_ascii_hexdigit() || c == '^' || c == '~' || c.is_ascii_digit())
    {
        return Err("非法的版本号".into());
    }
    let out = run_git(&root, &["show", &format!("{rev}:{rel}")])?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Ok(String::new())
    }
}

/// History of ONE file — what the Timeline shows for whatever is open in the editor.
///
/// `git_log` is repo-wide and cannot answer "what happened to this file"; passing a
/// pathspec to it would also lose renames. `--follow` keeps the trail across renames,
/// which is the whole point of a per-file timeline: a file that was moved still shows
/// the commits it carried under its old name.
///
/// `--` separates the pathspec from revisions. Without it a file whose name also
/// matches a branch or tag makes git fail with "ambiguous argument".
#[tauri::command(async)]
pub fn git_file_log(
    root: String,
    rel: String,
    count: Option<usize>,
) -> Result<Vec<GitLogEntry>, String> {
    require_git_root(&root, false)?;
    if rel.trim().is_empty() {
        return Ok(Vec::new());
    }
    let n = count.unwrap_or(30).min(200);
    let format = "%H%n%h%n%an%n%ar%n%s%n%P%n%D";
    let out = run_git_checked(
        &root,
        &[
            "log",
            "--follow",
            &format!("-{n}"),
            &format!("--format={format}"),
            "--",
            &rel,
        ],
    )?;
    Ok(parse_log_entries(&out))
}

/// 冲突这条路上，所有 `rel` 的锚点是**仓库顶层**，不是用户打开的那个文件夹。
///
/// `root` 只被 `require_git_root` 要求"在工作区里"，从来不要求它是仓库顶层——打开
/// `<repo>/sub` 是很常见的用法（monorepo 里只开一个包）。而这条路上两头用的坐标系不一样：
///
/// - `git diff --name-only --diff-filter=U`（git_conflicts 的数据源）无论从哪个子目录跑，
///   印的都是**相对顶层**的路径，而且列的是**整个仓库**的冲突，不只是当前目录底下的
///   （实测：`git -C <repo>/sub` 照样印出 `top.txt` 和 `sub/bar.txt`）。
/// - 而 pathspec 是**相对 CWD** 解析的。`git -C <repo>/sub ls-files -u -- ':(literal)sub/bar.txt'`
///   实测一行都不匹配。
///
/// 这个错位以前是"静默走偏"：老代码把 `<repo>/top.txt` 写成了 `<repo>/sub/top.txt`
/// ——一个凭空多出来的野文件——然后 `git add` 它、返回 Ok(())。上一轮修复补了
/// "索引里没有未合并项就报错"，于是它变成了"每一次解冲突都失败"，而且失败信息说的是
/// 「当前不是冲突状态」，与面板上一行还把它列成冲突文件的事实正面矛盾。
///
/// 所以两头一起校到顶层：pathspec 加 `top` magic（`:(top,literal)`，相对顶层解析），
/// 落盘/读盘的绝对路径也从这里拼。`git show :N:<rel>` 不用管——它本来就是相对顶层解析的
/// （实测 `git -C <repo>/sub show :2:sub/bar.txt` 出内容，`:2:bar.txt` 报 fatal）。
fn repo_top_level(root: &str) -> Result<PathBuf, String> {
    let top = run_git_checked(root, &["rev-parse", "--show-toplevel"])?;
    if top.is_empty() {
        // 裸仓库 / `--show-toplevel` 印不出东西：这两种情况下根本不会有工作区冲突，
        // 但绝不能退回用 `root` 当顶层——那正是上面那个错位，静默走偏比报错糟得多。
        return Err(format!(
            "cannot locate the repository top level for '{root}' (git rev-parse --show-toplevel printed nothing)"
        ));
    }
    Ok(PathBuf::from(top))
}

/// List files with merge conflicts (unmerged entries in `git status`).
#[derive(Serialize)]
pub struct ConflictFile {
    path: String,
    rel: String,
    name: String,
}

#[tauri::command(async)]
pub fn git_conflicts(root: String) -> Result<Vec<ConflictFile>, String> {
    require_git_root(&root, false)?;
    // core.quotepath=false：否则中文文件名会被 git 转义成 `"\344\275\240\345\245\275.txt"`，
    // 冲突列表里显示的就是这串八进制。git status 那边早就加了，这里漏了。
    let out = run_git(
        &root,
        &[
            "-c",
            "core.quotepath=false",
            "diff",
            "--name-only",
            "--diff-filter=U",
        ],
    )?;
    if !out.status.success() {
        return Ok(Vec::new());
    }
    // 绝对路径从顶层拼，不是从 `root` 拼（见 repo_top_level）。面板拿 `path` 去
    // read_text_file / 手动解决时写回，按打开的文件夹拼就会指向一个不存在的
    // `<repo>/sub/sub/bar.txt`。放在 diff 成功之后：不是仓库时上面已经返回空列表了。
    let top = repo_top_level(&root)?;
    let text = String::from_utf8_lossy(&out.stdout);
    let files: Vec<ConflictFile> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|rel| {
            let abs = top.join(rel);
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

#[tauri::command(async)]
pub fn git_merge_versions(root: String, rel: String) -> Result<MergeVersions, String> {
    require_git_root(&root, false)?;
    // 三个 `git show :N:{rel}` 本来就按顶层解析，只有这条读磁盘的路以前按打开的文件夹
    // 拼——子目录工作区里「合并结果」那一栏于是永远是空的（read_to_string 落在一个
    // 不存在的 `<repo>/sub/sub/bar.txt` 上，被 unwrap_or_default 吞成空串）。
    let top = repo_top_level(&root)?;
    let safe_path = require_rel_inside_root(&top, &rel)?;
    // 锚点从「打开的文件夹」挪到「仓库顶层」之后，这条读盘的路也**当场再查一次**边界，
    // 理由和 `git_resolve_conflict` 那边一模一样，只是方向是读：`require_git_root` 查的是
    // `root`，而 `top` 可以是 `root` 的任意祖先，于是 rel 能把 read_to_string 引到打开的
    // 工作区之外去 —— 内容会原样出现在合并面板里（以及模型的上下文里）。
    //
    // 这不是「本来就有的老洞」，是**这次改锚点新开的**：从前拼的是 `<root>/{rel}`，
    // 越界路径落在一个不存在的位置上、被 unwrap_or_default 吞成空串。
    //
    // 用读的那一档（`false`）而不是写的那一档：HOME 是读根，所以 HOME 底下的仓库
    // 一切照旧、这条检查一个用例都不影响；被挡住的只有「仓库顶层在 HOME 之外、
    // 而你打开的是它的子目录」那一种 —— 那正是不该把顶层旁边的文件读出来的场合。
    // 报错而不是继续给空串：空串就是这次修的那个 bug 本身（面板上一栏莫名其妙是空的）。
    require_inside_workspace(&safe_path.to_string_lossy(), false).map_err(|e| {
        format!(
            "「{rel}」在这个仓库里，但不在你打开的工作区内，读不到它的工作副本（{e}）。\
             把仓库顶层 {} 作为文件夹打开再试。",
            top.display()
        )
    })?;
    let base = run_git(&root, &["show", &format!(":1:{rel}")])
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let ours = run_git(&root, &["show", &format!(":2:{rel}")])
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let theirs = run_git(&root, &["show", &format!(":3:{rel}")])
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let merged = std::fs::read_to_string(&safe_path).unwrap_or_default();
    Ok(MergeVersions {
        base,
        ours,
        theirs,
        merged,
    })
}

/// 索引里 `pathspec` 真正存着哪几个合并 stage，以及每个 stage 的 blob oid
/// （1=base，2=ours，3=theirs）。
///
/// 冲突不一定三个 stage 都齐：modify/delete 冲突里被删掉的那一侧**根本没有 stage**。
/// `git ls-files -u` 每行形如 `100644 <sha> 2\t<path>`：模式、oid、stage 号、TAB、路径
/// （路径可能被 git 加引号转义，但我们只取 TAB 前那段，所以不受影响）。
///
/// **判据必须来自索引，不能来自 `git show` 的退出码。** 实测：`git show ':3:a*.txt'` 在
/// stage 3 不存在时**退出 0 且 stdout 为空**（路径里的 `*` 让它走了通配匹配那条路），
/// 而同样情况下 `git show ':3:foo.txt'` 是退出 128。也就是说"查退出码"这一条对带通配
/// 字符的文件名当场失效，零字节文件照样被写下去。拿到 oid 之后改用
/// `git cat-file blob <oid>`：不再有任何路径/revision 解析，退出码也就可信了。
fn conflict_stages(root: &str, pathspec: &str) -> Result<Vec<(u8, String)>, String> {
    let out = run_git_checked(root, &["ls-files", "-u", "--", pathspec])?;
    let mut stages: Vec<(u8, String)> = Vec::new();
    for line in out.lines() {
        let meta = match line.split_once('\t') {
            Some((meta, _)) => meta,
            None => continue,
        };
        let mut fields = meta.split_whitespace();
        let _mode = fields.next();
        let oid = match fields.next() {
            // oid 会被原样当作 `git cat-file` 的实参，所以只接受纯十六进制——git 自己的
            // 输出本来就是，但一个非 hex 的串（比如以 `-` 开头）会变成选项注入。
            Some(o) if !o.is_empty() && o.chars().all(|c| c.is_ascii_hexdigit()) => o,
            _ => continue,
        };
        let stage = match fields.next().and_then(|s| s.parse::<u8>().ok()) {
            Some(s) => s,
            None => continue,
        };
        if !stages.iter().any(|(s, _)| *s == stage) {
            stages.push((stage, oid.to_string()));
        }
    }
    Ok(stages)
}

/// Accept one side of a merge conflict for a file.
///
/// 这个函数踩过三个坑，三个都以「它照样返回 Ok」收场：
///
/// ① **不看退出码。** `run_git` 对任何非零退出都返回 `Ok(Output)`（只有 spawn 失败才 Err），
///    所以原来那句 `.map_err(...)?` 从来不会触发。modify/delete 冲突（porcelain 的 UD / DU）
///    里被删的那一侧没有对应 stage，`git show :3:rel` 以 128 退出、stdout 是空的，
///    于是**一个 0 字节的文件**被写下去、`git add` 进索引，函数报成功。用户点的是
///    「接受对方的删除」，拿到的却是一个空文件——而且它已经暂存好了，下一步就提交进去。
/// ② **没有 blob 时该删文件，不是写空文件。** 「接受删了它的那一侧」在语义上就是接受删除。
/// ③ **走了 from_utf8_lossy。** 冲突的是一张 PNG / 一个 .so 的时候，每个非 UTF-8 字节被换成
///    U+FFFD，暂存进去的是一份烂掉的二进制，同样报成功。现在按**原始字节**读写。
///
/// 修法上有一处值得记下来：**光加一句"查退出码"是不够的**。实测 `git show ':3:a*.txt'`
/// 在 stage 3 不存在时退出 0、stdout 为空（文件名里的 `*` 让它走了通配那条路），
/// 零字节文件照样会被写下去。所以判据整个搬到索引上：先 `git ls-files -u` 问出这个路径
/// 到底有哪几个 stage 和对应的 oid，再用 `git cat-file blob <oid>` 取内容——那条路上
/// 没有任何路径/revision 解析，退出码才真的可信。
///
/// 本文件里其它每一个会改东西的 git 包装（stage / commit / stash / branch）都查退出码，
/// 只有这里没查；`git_merge_versions` 用 `unwrap_or_default()` 是对的——它只用于**显示**。
#[tauri::command(async)]
pub fn git_resolve_conflict(root: String, rel: String, resolution: String) -> Result<(), String> {
    require_git_root(&root, true)?;
    let top = repo_top_level(&root)?;
    let file_path = require_rel_inside_root(&top, &rel)?;
    // 锚点从「打开的文件夹」挪到「仓库顶层」之后，写入边界要在这里**当场再查一次**：
    // `require_git_root` 查的是 `root`，而现在动的文件可能在 `root` 之外。
    // HOME 本身就是一个仓库（dotfiles 仓库，`git init ~`）的人是真实存在的，那种机器上
    // 顶层等于 HOME，一个叫 `.ssh/authorized_keys` 的未合并条目就能顺着这条路被写出去。
    // require_inside_workspace(写) 要求路径落在**显式打开过**的工作区里（HOME 不算），
    // 正好挡住这一类。副作用是：打开 `<repo>/sub` 时，仓库里 `sub/` 之外的那些冲突
    // 在面板上看得见但解不了——报的是「不在打开的工作区内、去打开仓库顶层」，
    // 这句话是真的，比原来那句「不是冲突状态」强。
    require_inside_workspace(&file_path.to_string_lossy(), true).map_err(|e| {
        format!(
            "「{rel}」在这个仓库里，但不在你打开的工作区内，无法在这里解决（{e}）。\
             把仓库顶层 {} 作为文件夹打开再试。",
            top.display()
        )
    })?;
    // `--` 只挡住「路径被当成选项」，挡不住**通配**：`git rm -- 'a*.txt'` 会把 ab.txt 一起
    // 删掉（实测过）。rel 来自 git status，是一条真实路径，但文件名里带 `*` / `?` / `[]`
    // 在 Unix 上完全合法。`:(literal)` 是 git 的 pathspec magic，关掉这一条的通配解释。
    // 解冲突这条路上现在有 `git rm`，误伤的代价是别人的文件被删，所以三处 pathspec 一起收。
    // 前面那个 `top` 是第二条 magic：pathspec 默认相对 CWD 解析，而 rel 是相对顶层的，
    // 打开子目录时两者对不上（见 repo_top_level）。两条 magic 必须同时在。
    let pathspec = format!(":(top,literal){rel}");
    let stage: u8 = match resolution.as_str() {
        "ours" => 2,
        "theirs" => 3,
        "manual" => {
            // File was manually edited; just mark as resolved
            return run_git_checked(&root, &["add", "--", &pathspec]).map(|_| ());
        }
        _ => return Err(format!("unknown resolution: {resolution}")),
    };

    // 先问索引，再取内容。顺序不能反：`git show` 的失败**不可靠**（见 conflict_stages），
    // 而索引里有没有这个 stage 是个确定的事实。
    let stages = conflict_stages(&root, &pathspec)?;
    if stages.is_empty() {
        return Err(format!(
            "「{rel}」当前不是冲突状态（索引里没有未合并的暂存项），无法按 {resolution} 解决"
        ));
    }

    let oid = match stages.iter().find(|(s, _)| *s == stage) {
        Some((_, oid)) => oid.clone(),
        None => {
            // 选中的那一侧**没有这个文件**（modify/delete 冲突里被删掉的那一边）。
            // 「接受这一侧」在语义上就是接受删除，所以真的删掉并把删除暂存起来——
            // 旧代码在这里写了个 0 字节文件然后 git add，把"删除"解决成了"空文件"。
            // `-f` 是必须的：路径处于未合并状态时 git rm 会拒绝，而工作区里那份正是
            // 用户此刻明确要丢弃的另一侧内容。
            return run_git_checked(&root, &["rm", "-q", "-f", "--", &pathspec]).map(|_| ());
        }
    };

    let out = run_git(&root, &["cat-file", "blob", &oid])?;
    if !out.status.success() {
        // 索引说这个 stage 在，却读不出对象 —— 仓库真的坏了。必须报出去，而不是把空的
        // stdout 当成内容写进用户的文件。
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!(
            "cannot get {resolution} for '{rel}'{}",
            if err.is_empty() { String::new() } else { format!(": {err}") }
        ));
    }
    // 原始字节原样落盘：这一侧的内容可能是二进制，也可能是这个仓库里合法但非 UTF-8
    // 的文本，任何转码都是在替用户改内容。
    std::fs::write(&file_path, &out.stdout).map_err(|e| e.to_string())?;
    run_git_checked(&root, &["add", "--", &pathspec]).map(|_| ())
}

/// git 失败时的回执：**stdout 也要带上**。
///
/// `git stash pop` 撞冲突时退出码是 1，而整份冲突报告——哪个文件冲突、
/// 「The stash entry is kept in case you need it again」——全在 **stdout**，
/// stderr 是空的。原来四处都写成 `if err.is_empty() { "…failed" } else { err }`，
/// 于是这份唯一有用的东西被整个扔掉，调用方只拿到五个词的 "git stash pop failed"：
/// 不知道冲突在哪、也不知道 stash 还留着，接着就会去重复 pop 或直接 drop。
fn git_failure_text(text: &str, err: &str, fallback: &str) -> String {
    match (text.is_empty(), err.is_empty()) {
        (true, true) => fallback.to_string(),
        (true, false) => err.to_string(),
        (false, true) => text.to_string(),
        (false, false) => format!("{text}\n{err}"),
    }
}

/// Stash the current working directory changes.
#[tauri::command(async)]
pub fn git_stash(root: String) -> Result<String, String> {
    require_git_root(&root, true)?;
    let out = run_git(&root, &["stash", "push", "-m", "Mr. Day One stash"])?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        Ok(if text.is_empty() {
            "No local changes to stash.".into()
        } else {
            text
        })
    } else {
        Err(git_failure_text(&text, &err, "git stash push failed"))
    }
}

/// Pop a stash entry, applying it and removing it from the stash list.
///
/// Defaults to the most recent stash (`stash@{0}`) when no index is given.
#[tauri::command(async)]
pub fn git_stash_pop(root: String, index: Option<usize>) -> Result<String, String> {
    require_git_root(&root, true)?;
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
        Err(git_failure_text(&text, &err, "git stash pop failed"))
    }
}

/// Apply a stash entry without removing it from the stash list.
#[tauri::command(async)]
pub fn git_stash_apply(root: String, index: usize) -> Result<String, String> {
    require_git_root(&root, true)?;
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
        Err(git_failure_text(&text, &err, "git stash apply failed"))
    }
}

/// Drop (delete) a stash entry without applying it.
#[tauri::command(async)]
pub fn git_stash_drop(root: String, index: usize) -> Result<String, String> {
    require_git_root(&root, true)?;
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
        Err(git_failure_text(&text, &err, "git stash drop failed"))
    }
}

/// List stash entries.
#[tauri::command(async)]
pub fn git_stash_list(root: String) -> Result<Vec<String>, String> {
    require_git_root(&root, false)?;
    let out = run_git(&root, &["stash", "list"])?;
    // 命令失败绝不能吞成空列表。
    //
    // 原来这里 `return Ok(Vec::new())`，前端于是印「(stash 堆栈为空)」——而真实情况可能是
    // 仓库损坏、index.lock 被占、权限不足。用户刚 stash 完切分支回来，看到「为空」会以为
    // 改动丢了；更糟的是模型据此判定「没有需要恢复的东西」，跳过 stash_pop 直接在工作区上
    // 继续写——改动还在 stash 里，但再没人去取。
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "git stash list 失败（exit {}）：{}",
            out.status.code().unwrap_or(-1),
            err.trim()
        ));
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

#[tauri::command(async)]
pub fn git_blame(root: String, rel: String) -> Result<Vec<BlameLine>, String> {
    require_git_root(&root, false)?;
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
            // `author-time` 是**裸 Unix 时间戳**（1787290406）。原样交出去，模型要么
            // 照抄给用户，要么自己去换算——而 git_log 那条早就用 %ar 给的是「3 小时前」。
            // 同一个仓库里两种口径，这里对齐成人能读的日期。
            cur_date = d
                .trim()
                .parse::<i64>()
                .ok()
                .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| d.to_string());
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

/// Pull the current branch from its upstream with an explicit merge strategy.
///
/// Returns combined stdout/stderr because git reports progress on stderr even
/// on success.
#[tauri::command(async)]
pub fn git_pull(root: String) -> Result<String, String> {
    require_git_root(&root, true)?;
    // Newer Git versions refuse a divergent pull unless the user configured a
    // reconciliation strategy. The IDE's button is explicitly “拉取并合并”, so
    // pass the strategy every time and disable the merge-message editor.
    let out = run_git(&root, &["pull", "--no-rebase", "--no-edit"])?;
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

#[cfg(test)]
mod tests {
    /// `--force` 只许落在本工具自己建的候选工作树上。
    ///
    /// 原来 remove 把模型给的 path 原样交给 `git worktree remove --force`，零校验；
    /// 而这个工具不需要审批、path 又可以被仓库里的文本诱导。用户自己
    /// `git worktree add ../feature-x` 挂的、里面有一天没提交改动的那棵树，一句就没了
    /// ——`--force` 的语义正是「有未提交改动也删」。
    #[test]
    fn worktree_remove_only_accepts_worktrees_this_tool_created() {
        let root = "/repo";
        let good = format!("{root}/.mrdayone/worktrees/cand1");

        // add 那边建出来的形状：放行，并且末段原样带出去给分支名用。
        let (abs, safe) = super::bon_worktree_path(root, &good).expect("自己建的应当放行");
        assert_eq!(abs, good);
        assert_eq!(safe, "cand1");
        // 只给名字也接受，归到同一个位置。
        assert_eq!(super::bon_worktree_path(root, "cand1").unwrap().0, good);

        // 用户自己挂的工作树、以及各种越界写法：一律拒绝。
        for bad in [
            "../feature-x",
            "/Users/someone/work/feature-x",
            "/repo/../feature-x",
            "/repo/.mrdayone/worktrees/../../feature-x",
            "/repo/src",
            "",
            "/repo/.mrdayone/worktrees/",
        ] {
            assert!(
                super::bon_worktree_path(root, bad).is_err(),
                "这个路径本不该被 --force 删掉：{bad}",
            );
        }
    }

    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempGitRoot(PathBuf);

    impl TempGitRoot {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "michael-ide-git-roundtrip-{}-{suffix}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempGitRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn path_string(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    fn configure_identity(root: &str) {
        run_git_checked(root, &["config", "user.name", "Mr. Day One Test"]).unwrap();
        run_git_checked(root, &["config", "user.email", "ide-test@example.invalid"]).unwrap();
    }

    #[test]
    fn status_diff_stage_commit_push_clone_roundtrip() {
        let temp = TempGitRoot::new();
        let base = path_string(&temp.0);
        let source_path = temp.0.join("source");
        let remote_path = temp.0.join("origin.git");
        let clone_path = temp.0.join("clone");
        let source = path_string(&source_path);
        let remote = path_string(&remote_path);
        let clone = path_string(&clone_path);

        run_git_checked(&base, &["init", "--bare", &remote]).unwrap();
        run_git_checked(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]).unwrap();
        run_git_checked(&base, &["init", &source]).unwrap();
        run_git_checked(&source, &["checkout", "-b", "main"]).unwrap();
        configure_identity(&source);

        fs::write(source_path.join("README.md"), "v1\n").unwrap();
        let status = git_status(source.clone()).unwrap();
        assert!(status.is_repo);
        assert_eq!(status.branch, "main");
        assert_eq!(status.files.len(), 1);
        assert_eq!(status.files[0].rel, "README.md");
        assert_eq!(status.files[0].code, "??");
        assert!(!status.files[0].staged);

        git_stage(source.clone(), "README.md".into()).unwrap();
        let staged_status = git_status(source.clone()).unwrap();
        assert_eq!(staged_status.files[0].code, "A ");
        assert!(staged_status.files[0].staged);
        let staged = git_diff(source.clone(), Some("README.md".into()), Some(true)).unwrap();
        assert!(staged.contains("+v1"));
        let first_commit = git_commit(source.clone(), "initial commit".into()).unwrap();
        assert!(first_commit.contains("initial commit"));
        assert_eq!(git_status(source.clone()).unwrap().branch, "main");

        run_git_checked(&source, &["remote", "add", "upstream", &remote]).unwrap();
        git_push(source.clone()).unwrap();
        assert_eq!(
            run_git_checked(
                &source,
                &[
                    "rev-parse",
                    "--abbrev-ref",
                    "--symbolic-full-name",
                    "@{upstream}",
                ],
            )
            .unwrap(),
            "upstream/main"
        );

        let source_head = run_git_checked(&source, &["rev-parse", "HEAD"]).unwrap();
        let remote_head = run_git_checked(&remote, &["rev-parse", "refs/heads/main"]).unwrap();
        assert_eq!(source_head, remote_head);

        assert!(git_clone("ext::sh -c echo unsafe".into(), clone.clone()).is_err());
        assert!(git_clone(remote.clone(), "relative/clone".into()).is_err());
        assert!(git_clone(remote.clone(), base.clone()).is_err());
        #[cfg(unix)]
        {
            let dangling = temp.0.join("dangling-clone");
            std::os::unix::fs::symlink(temp.0.join("missing-target"), &dangling).unwrap();
            assert!(git_clone(remote.clone(), path_string(&dangling)).is_err());
            fs::remove_file(dangling).unwrap();
        }
        let cloned_to = git_clone(remote.clone(), clone.clone()).unwrap();
        assert_eq!(
            PathBuf::from(cloned_to),
            std::fs::canonicalize(&temp.0).unwrap().join("clone")
        );
        configure_identity(&clone);
        assert_eq!(
            fs::read_to_string(clone_path.join("README.md")).unwrap(),
            "v1\n"
        );
        let cloned_status = git_status(clone.clone()).unwrap();
        assert!(cloned_status.is_repo);
        assert_eq!(cloned_status.branch, "main");
        assert!(cloned_status.files.is_empty());

        fs::write(clone_path.join("README.md"), "v2\n").unwrap();
        let changed = git_status(clone.clone()).unwrap();
        assert_eq!(changed.files.len(), 1);
        assert_eq!(changed.files[0].code, " M");
        assert!(!changed.files[0].staged);
        let unstaged = git_diff(clone.clone(), Some("README.md".into()), Some(false)).unwrap();
        assert!(unstaged.contains("-v1"));
        assert!(unstaged.contains("+v2"));

        git_stage(clone.clone(), "README.md".into()).unwrap();
        let staged_status = git_status(clone.clone()).unwrap();
        assert_eq!(staged_status.files[0].code, "M ");
        assert!(staged_status.files[0].staged);
        let staged = git_diff(clone.clone(), Some("README.md".into()), Some(true)).unwrap();
        assert!(staged.contains("-v1"));
        assert!(staged.contains("+v2"));
        let second_commit = git_commit(clone.clone(), "update from clone".into()).unwrap();
        assert!(second_commit.contains("update from clone"));
        git_push(clone.clone()).unwrap();

        let clone_head = run_git_checked(&clone, &["rev-parse", "HEAD"]).unwrap();
        let remote_head = run_git_checked(&remote, &["rev-parse", "refs/heads/main"]).unwrap();
        let remote_content =
            run_git_checked(&remote, &["show", "refs/heads/main:README.md"]).unwrap();
        assert_eq!(clone_head, remote_head);
        assert_eq!(remote_content, "v2");
        assert!(git_status(clone.clone()).unwrap().files.is_empty());

        run_git_checked(&clone, &["checkout", "--detach", "HEAD"]).unwrap();
        let detached_error = git_push(clone).unwrap_err();
        assert!(detached_error.contains("detached HEAD"));
    }

    #[test]
    fn push_remote_selection_is_deterministic() {
        assert_eq!(
            select_push_remote(&["backup".into(), "origin".into()]).unwrap(),
            "origin"
        );
        assert_eq!(select_push_remote(&["company".into()]).unwrap(), "company");
        assert!(select_push_remote(&[]).unwrap_err().contains("no remotes"));
        let error = select_push_remote(&["one".into(), "two".into()]).unwrap_err();
        assert!(error.contains("no `origin` remote"));
        assert!(error.contains("one, two"));
    }

    #[test]
    fn pull_uses_merge_strategy_without_opening_an_editor() {
        let temp = TempGitRoot::new();
        let base = path_string(&temp.0);
        let source_path = temp.0.join("source");
        let remote_path = temp.0.join("origin.git");
        let clone_path = temp.0.join("clone");
        let source = path_string(&source_path);
        let remote = path_string(&remote_path);
        let clone = path_string(&clone_path);

        run_git_checked(&base, &["init", "--bare", &remote]).unwrap();
        run_git_checked(&remote, &["symbolic-ref", "HEAD", "refs/heads/main"]).unwrap();
        run_git_checked(&base, &["init", &source]).unwrap();
        run_git_checked(&source, &["checkout", "-b", "main"]).unwrap();
        configure_identity(&source);

        fs::write(source_path.join("README.md"), "base\n").unwrap();
        git_stage(source.clone(), "README.md".into()).unwrap();
        git_commit(source.clone(), "base".into()).unwrap();
        run_git_checked(&source, &["remote", "add", "origin", &remote]).unwrap();
        git_push(source.clone()).unwrap();

        git_clone(remote.clone(), clone.clone()).unwrap();
        configure_identity(&clone);

        fs::write(source_path.join("remote.txt"), "remote\n").unwrap();
        git_stage(source.clone(), "remote.txt".into()).unwrap();
        git_commit(source.clone(), "remote change".into()).unwrap();
        git_push(source.clone()).unwrap();

        fs::write(clone_path.join("local.txt"), "local\n").unwrap();
        git_stage(clone.clone(), "local.txt".into()).unwrap();
        git_commit(clone.clone(), "local change".into()).unwrap();

        let out = git_pull(clone.clone()).unwrap();
        assert!(
            out.contains("Merge") || out.contains("files changed") || out.contains("file changed"),
            "unexpected pull output: {out}"
        );
        assert_eq!(
            fs::read_to_string(clone_path.join("remote.txt")).unwrap(),
            "remote\n"
        );
        assert_eq!(
            fs::read_to_string(clone_path.join("local.txt")).unwrap(),
            "local\n"
        );
        let parents = run_git_checked(&clone, &["show", "-s", "--format=%P", "HEAD"]).unwrap();
        assert!(
            parents.split_whitespace().count() >= 2,
            "pull should create a merge commit for divergent branches, got parents: {parents}"
        );
    }
}

#[cfg(test)]
mod git_path_tests {
    use super::*;

    /// git 对含非 ASCII 的路径会输出 `"\344\270\255文.txt"` 这种形式。不解开的话拼出来
    /// 的绝对路径不存在 → 被判成"已删除"，而且暂存/看 diff 一律失败。
    #[test]
    fn octal_escaped_utf8_paths_round_trip() {
        assert_eq!(
            unquote_git_path("\"\\344\\270\\255\\346\\226\\207.txt\""),
            "中文.txt"
        );
        assert_eq!(unquote_git_path("\"src/\\344\\270\\255.rs\""), "src/中.rs");
    }

    #[test]
    fn c_style_escapes_are_decoded() {
        assert_eq!(unquote_git_path("\"a\\tb.txt\""), "a\tb.txt");
        assert_eq!(unquote_git_path("\"back\\\\slash\""), "back\\slash");
    }

    /// 绝大多数路径不带引号，必须原样返回——包括含空格的（git 不会为空格加引号）。
    #[test]
    fn unquoted_paths_are_untouched() {
        assert_eq!(unquote_git_path("src/main.rs"), "src/main.rs");
        assert_eq!(unquote_git_path("my file.txt"), "my file.txt");
        assert_eq!(unquote_git_path(""), "");
    }
}

#[cfg(test)]
mod git_show_tests {
    use super::*;

    /// 在一个真造出来的仓库上跑，不是断言字符串。
    fn tmp_repo(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mrday-show-{}-{}", tag, std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let git = crate::process_util::resolve_system_command("git");
        let run = |args: &[&str]| {
            crate::process_util::command(&git)
                .arg("-C").arg(&dir).args(args)
                .env("PATH", crate::process_util::augmented_path(None))
                .env("GIT_AUTHOR_NAME", "t").env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t").env("GIT_COMMITTER_EMAIL", "t@t")
                .output().unwrap()
        };
        run(&["init", "-q", "."]);
        std::fs::write(dir.join("app.js"), "const VERSION = 1;\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "第一版"]);
        std::fs::write(dir.join("app.js"), "const VERSION = 2;\n").unwrap();
        run(&["commit", "-q", "-am", "把版本号改成 2"]);
        dir
    }

    #[test]
    fn git_show_reaches_into_history_which_diff_and_blame_cannot() {
        let dir = tmp_repo("hist");
        let root = dir.to_string_lossy().to_string();
        crate::files::register_workspace_root(root.clone()).ok();

        // ① 看提交本身：要有提交信息、文件统计和 patch
        let commit = git_show(root.clone(), "HEAD".into(), None).expect("看 HEAD 应当成功");
        assert!(commit.contains("把版本号改成 2"), "没有提交信息：{commit}");
        assert!(commit.contains("app.js"), "没有文件统计");
        assert!(commit.contains("+const VERSION = 2;"), "没有 patch");

        // ② 看**改之前**那个文件的全文——这正是 git_diff / git_blame 做不到的那一步
        let before = git_show(root.clone(), "HEAD~1".into(), Some("app.js".into())).expect("看历史文件应当成功");
        assert_eq!(before.trim(), "const VERSION = 1;", "拿到的不是那次提交时的内容");

        // ③ 分支名 / 相对引用要能用，不是只认十六进制哈希
        assert!(git_show(root.clone(), "HEAD^".into(), None).is_ok());

        // ④ 非法 rev 必须拒，且不能被当成 git 的选项
        for bad in ["--upload-pack=touch /tmp/pwned", "-x", "a;b", "a b", "$(id)"] {
            let e = git_show(root.clone(), bad.into(), None).unwrap_err();
            assert!(e.contains("非法的版本号"), "{bad} 没被拒：{e}");
        }

        // ⑤ 查不到的提交要给出可执行的下一步，不是干巴巴一句失败
        let e = git_show(root.clone(), "deadbeef".into(), None).unwrap_err();
        assert!(e.contains("git_log"), "错误信息没告诉模型下一步该干嘛：{e}");

        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod git_failure_text_tests {
    use super::git_failure_text;

    /// 函数写对了但没接上，等于没写。实测：把 git_stash_pop 那处接线退回
    /// `if err.is_empty() { … } else { err }`，455 个测试全绿——守卫测的是 helper 的
    /// 纯逻辑，覆盖不到「四个失败分支有没有用它」。
    #[test]
    fn all_four_stash_failure_branches_are_wired() {
        let src = include_str!("git.rs");
        let code: String = src
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(src)
            .lines()
            .filter(|l| !l.trim_start().starts_with("//") && !l.trim_start().starts_with("///"))
            .collect::<Vec<_>>()
            .join("\n");
        for fallback in [
            "git stash push failed",
            "git stash pop failed",
            "git stash apply failed",
            "git stash drop failed",
        ] {
            assert!(
                code.contains(&format!("git_failure_text(&text, &err, \"{fallback}\")")),
                "「{fallback}」那条失败分支没走 git_failure_text —— stdout 里的冲突报告会被整个扔掉"
            );
        }
        // 而且不许再出现「只看 err、扔掉 stdout」的老形状。
        // 收窄到「扔掉 stdout」那个确切形状：`Err(if err.is_empty() { … })`。
        // run_git_checked 里的 `let msg = if err.is_empty() { …stdout… }` 是对的
        // ——它 stderr 空时会退回 stdout，不在此列。
        assert!(
            !code.contains("Err(if err.is_empty() {"),
            "又有分支退回了「err 为空就用兜底文案」—— 那正是把 stdout 整个扔掉的写法"
        );
        // 这条 bug 不止 stash 有：git_status / git_diff / git_branches 当初是同一个形状。
        for fallback in ["git status failed", "git diff failed", "git branch failed"] {
            assert!(
                code.contains(&format!("git_failure_text(&text, &err, \"{fallback}\")")),
                "「{fallback}」这条也会把 stdout 扔掉 —— 同一个形状，一起修的"
            );
        }
    }

    /// `git stash pop` 撞冲突：退出码 1，冲突报告全在 stdout，stderr 空。
    /// 原来的 `if err.is_empty() { fallback } else { err }` 把它整个扔掉。
    #[test]
    fn conflict_report_on_stdout_survives_a_nonzero_exit() {
        let stdout = "Auto-merging f.txt\nCONFLICT (content): Merge conflict in f.txt\n\
                      The stash entry is kept in case you need it again.";
        let got = git_failure_text(stdout, "", "git stash pop failed");
        assert!(got.contains("CONFLICT (content): Merge conflict in f.txt"));
        assert!(
            got.contains("The stash entry is kept"),
            "「stash 还留着」被丢了，调用方会以为它已经没了"
        );
        assert_ne!(got, "git stash pop failed");
    }

    /// 越界的 stash 索引反过来：stderr 有话、stdout 空。
    #[test]
    fn stderr_only_failures_still_report_stderr() {
        assert_eq!(
            git_failure_text("", "error: stash@{9} is not a valid reference", "git stash drop failed"),
            "error: stash@{9} is not a valid reference"
        );
    }

    #[test]
    fn both_streams_are_kept_and_a_silent_failure_still_says_something() {
        assert_eq!(git_failure_text("out", "err", "fallback"), "out\nerr");
        assert_eq!(git_failure_text("", "", "fallback"), "fallback");
    }
}

#[cfg(test)]
mod git_log_behaviour_tests {
    /// 源码 grep 挡不住「空 if + 无条件 push」这种**等价于旧 bug** 的变体——
    /// 实测：那么改一遍，455 个测试全绿。所以这条跑真 git。
    fn sh(dir: &std::path::Path, args: &[&str]) {
        let out = crate::process_util::command("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("git 跑不起来");
        assert!(
            out.status.success(),
            "git {args:?} 失败: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn defaults_to_the_current_branch_and_all_is_opt_in() {
        let dir = std::env::temp_dir().join(format!("mrday-gitlog-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        sh(&dir, &["init", "-q", "-b", "main"]);
        sh(&dir, &["config", "user.email", "t@t"]);
        sh(&dir, &["config", "user.name", "t"]);
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        sh(&dir, &["add", "."]);
        sh(&dir, &["commit", "-qm", "main-1"]);
        sh(&dir, &["checkout", "-q", "-b", "side"]);
        std::fs::write(dir.join("b.txt"), "b").unwrap();
        sh(&dir, &["add", "."]);
        sh(&dir, &["commit", "-qm", "side-only"]);
        sh(&dir, &["checkout", "-q", "main"]);

        let root = dir.to_string_lossy().to_string();
        let mine = super::git_log(root.clone(), Some(20), None).expect("git_log 失败");
        let subjects: Vec<&str> = mine.iter().map(|e| e.message.as_str()).collect();
        assert!(
            !subjects.contains(&"side-only"),
            "默认就带上了别的分支的提交：{subjects:?} —— 模型会挑一个根本不是 HEAD 祖先的提交去推理"
        );
        assert!(subjects.contains(&"main-1"), "当前分支自己的提交反而没了：{subjects:?}");

        let all = super::git_log(root.clone(), Some(20), Some(true)).expect("git_log(all) 失败");
        let all_subjects: Vec<&str> = all.iter().map(|e| e.message.as_str()).collect();
        assert!(
            all_subjects.contains(&"side-only"),
            "显式要 all 也拿不到别的分支——分支图会退化成一条直线：{all_subjects:?}"
        );

        // 第一条的 refs 必须带 `HEAD -> <分支>`：前端靠它报分支名（比读全局 DOM 节点准）。
        assert!(
            mine[0].refs.iter().any(|r| r.contains("HEAD -> main")),
            "HEAD 装饰没了，前端就没法从这份结果里认出分支：{:?}",
            mine[0].refs
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod git_log_scope_tests {
    /// `--all` 必须是**显式传进来**才加。默认取安全的那个：画分支图的面板要跨分支，
    /// 智能体问「这条线上刚改了什么」时要的是当前分支，忘了传的调用方不该被带偏。
    #[test]
    fn all_is_opt_in_not_hardcoded() {
        let src = include_str!("git.rs");
        let body = src
            .split("pub fn git_log(")
            .nth(1)
            .and_then(|s| s.split("\nfn parse_log_entries").next())
            .expect("git_log 的函数体不见了");
        assert!(
            body.contains("if all.unwrap_or(false)"),
            "git_log 又把 --all 写死了"
        );
        let arg_line = body
            .lines()
            .find(|l| l.contains("\"--all\""))
            .expect("--all 整个没了 —— 分支图会退化成一条直线");
        assert!(
            arg_line.trim().starts_with("args.push"),
            "--all 回到了固定参数表里：{arg_line}"
        );
    }
}

#[cfg(test)]
mod git_conflict_resolution_tests {
    use super::*;

    /// 跑真 git。源码 grep 挡不住「换个写法、同样不看退出码」的变体，而这两个 bug 的
    /// 全部形状都在 git 的索引状态里（缺 stage、二进制 blob），造不出来就测不到。
    struct Repo {
        dir: std::path::PathBuf,
    }

    impl Repo {
        fn new(tag: &str) -> Repo {
            let dir = std::env::temp_dir().join(format!(
                "mrday-conflict-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::remove_dir_all(&dir).ok();
            std::fs::create_dir_all(&dir).unwrap();
            let repo = Repo { dir };
            repo.git(&["init", "-q", "."]);
            repo.git(&["config", "user.name", "t"]);
            repo.git(&["config", "user.email", "t@t"]);
            crate::files::register_workspace_root(repo.root()).ok();
            repo
        }

        fn root(&self) -> String {
            self.dir.to_string_lossy().to_string()
        }

        fn git(&self, args: &[&str]) -> std::process::Output {
            let git = crate::process_util::resolve_system_command("git");
            crate::process_util::command(&git)
                .arg("-C")
                .arg(&self.dir)
                .args(args)
                .env("PATH", crate::process_util::augmented_path(None))
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .env("LC_ALL", "C")
                .output()
                .expect("git 跑不起来")
        }

        fn out(&self, args: &[&str]) -> String {
            String::from_utf8_lossy(&self.git(args).stdout)
                .trim()
                .to_string()
        }

        fn branch(&self) -> String {
            self.out(&["rev-parse", "--abbrev-ref", "HEAD"])
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.dir).ok();
        }
    }

    /// modify/delete 冲突：HEAD 改了它，对面删了它 → porcelain 的 `UD`，索引里**没有 stage 3**。
    ///
    /// 老代码在这里 `git show :3:foo.txt` 拿到 128 + 空 stdout，却因为 run_git 对非零退出
    /// 返回 Ok 而毫无察觉，于是写下一个 **0 字节的 foo.txt** 并 `git add`，最后返回 Ok(())。
    /// 用户点的是「接受对方的删除」，得到的是一个已经暂存好的空文件——下一次提交就把它
    /// 固化进历史，而且看不出哪里错了。
    #[test]
    fn accepting_the_deleting_side_removes_the_file_instead_of_staging_an_empty_one() {
        let repo = Repo::new("moddel");
        std::fs::write(repo.dir.join("foo.txt"), "base\n").unwrap();
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "base"]);
        let main = repo.branch();

        repo.git(&["checkout", "-q", "-b", "deleter"]);
        repo.git(&["rm", "-q", "foo.txt"]);
        repo.git(&["commit", "-q", "-m", "删掉 foo"]);

        repo.git(&["checkout", "-q", &main]);
        std::fs::write(repo.dir.join("foo.txt"), "ours change\n").unwrap();
        repo.git(&["commit", "-q", "-am", "改了 foo"]);
        let merge = repo.git(&["merge", "deleter"]);
        assert!(!merge.status.success(), "这一步本该冲突，测试前提没造出来");
        assert!(
            repo.out(&["ls-files", "-u", "--", "foo.txt"])
                .lines()
                .all(|l| !l.contains("\tfoo.txt") || !l.split('\t').next().unwrap().ends_with(" 3")),
            "前提不成立：索引里居然有 stage 3"
        );

        git_resolve_conflict(repo.root(), "foo.txt".into(), "theirs".into())
            .expect("接受对方的删除应当成功");

        assert!(
            !repo.dir.join("foo.txt").exists(),
            "文件还在——说明写下去的是个空文件，而不是把删除这件事解决掉"
        );
        // 索引里必须是**已暂存的删除**（porcelain `D `），不是一个 0 字节的暂存内容。
        let status = repo.out(&["status", "--porcelain", "--", "foo.txt"]);
        assert_eq!(status, "D  foo.txt", "索引状态不对：{status:?}");
        assert!(
            repo.out(&["ls-files", "-u"]).is_empty(),
            "冲突没被真正解决，索引里还留着未合并的项"
        );
    }

    /// 二进制冲突：`from_utf8_lossy` 会把每个非 UTF-8 字节换成 U+FFFD（EF BF BD），
    /// 于是「接受我方」暂存进去的是一张**烂掉的图**，而函数照样报成功。
    /// 现在按原始字节读写，选中那一侧的 blob 必须**一个字节不差**地落到磁盘和索引里。
    #[test]
    fn a_conflicted_binary_file_is_restored_byte_for_byte() {
        let repo = Repo::new("binary");
        let mut base = b"\x89PNG\r\n\x1a\n".to_vec();
        base.extend_from_slice(&[0x00, 0x01, 0xFF, 0xFE, b'b', b'a', b's', b'e']);
        std::fs::write(repo.dir.join("logo.png"), &base).unwrap();
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "base"]);
        let main = repo.branch();

        repo.git(&["checkout", "-q", "-b", "other"]);
        let mut theirs = b"\x89PNG\r\n\x1a\n".to_vec();
        theirs.extend_from_slice(&[0x00, 0x02, 0xC3, 0x28, b't', b'h', b'e', b'i', b'r', b's']);
        std::fs::write(repo.dir.join("logo.png"), &theirs).unwrap();
        repo.git(&["commit", "-q", "-am", "对面改图"]);

        repo.git(&["checkout", "-q", &main]);
        // 0x80 / 0x81 是**孤立的 UTF-8 续字节**，from_utf8_lossy 一定会把它们吃掉。
        let mut ours = b"\x89PNG\r\n\x1a\n".to_vec();
        ours.extend_from_slice(&[0x00, 0x03, 0x80, 0x81, 0xFF, b'o', b'u', b'r', b's']);
        std::fs::write(repo.dir.join("logo.png"), &ours).unwrap();
        repo.git(&["commit", "-q", "-am", "我方改图"]);
        let merge = repo.git(&["merge", "other"]);
        assert!(!merge.status.success(), "这一步本该冲突，测试前提没造出来");

        git_resolve_conflict(repo.root(), "logo.png".into(), "ours".into())
            .expect("接受我方应当成功");

        let on_disk = std::fs::read(repo.dir.join("logo.png")).unwrap();
        assert_eq!(
            on_disk, ours,
            "落盘的不是原始字节 —— 非 UTF-8 的部分被 U+FFFD 替掉了，图就此损坏"
        );
        assert!(
            !on_disk.windows(3).any(|w| w == [0xEF, 0xBF, 0xBD]),
            "文件里出现了 U+FFFD 的字节序列，说明还是走了 from_utf8_lossy"
        );
        // 暂存的 blob 也要是同一份字节，不能磁盘对了索引却是坏的。
        let staged = repo.git(&["show", ":0:logo.png"]).stdout;
        assert_eq!(staged, ours, "索引里暂存的 blob 和磁盘不一致");
    }

    /// 带通配字符的文件名，是这条修复路上最阴的一格，它同时踩两个雷：
    ///
    /// 雷一：`git show ':3:a*.txt'` 在 stage 3 不存在时**退出 0、stdout 为空**（`*` 让它
    /// 走了通配匹配那条路），所以"补一句查退出码"根本挡不住零字节文件——这也是为什么
    /// 判据搬去了 `ls-files -u` + `cat-file blob <oid>`。
    ///
    /// 雷二：`--` 只保证路径不被当成选项，**不关通配**：`git rm -- 'a*.txt'` 会连 ab.txt
    /// 一起删（真 git 上实测过）。Unix 下这种文件名完全合法，而 rel 一路是从 git status
    /// 抄下来的真实路径。修法是 `:(literal)` pathspec magic。这一格比原 bug 更凶：
    /// 丢的是一个跟这次冲突毫无关系的文件。
    #[test]
    fn resolving_a_path_with_glob_characters_does_not_take_its_neighbours_with_it() {
        let repo = Repo::new("glob");
        std::fs::write(repo.dir.join("a*.txt"), "base\n").unwrap();
        std::fs::write(repo.dir.join("ab.txt"), "innocent\n").unwrap();
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "base"]);
        let main = repo.branch();

        repo.git(&["checkout", "-q", "-b", "deleter"]);
        repo.git(&["rm", "-q", "--", ":(literal)a*.txt"]);
        repo.git(&["commit", "-q", "-m", "删掉带星号的那个"]);

        repo.git(&["checkout", "-q", &main]);
        std::fs::write(repo.dir.join("a*.txt"), "ours change\n").unwrap();
        repo.git(&["commit", "-q", "-am", "改了带星号的那个"]);
        assert!(
            !repo.git(&["merge", "deleter"]).status.success(),
            "这一步本该冲突，测试前提没造出来"
        );

        git_resolve_conflict(repo.root(), "a*.txt".into(), "theirs".into())
            .expect("接受对方的删除应当成功");

        assert!(
            !repo.dir.join("a*.txt").exists(),
            "该删的没删掉 —— 大概率是 git show 退出 0 + 空 stdout，又写了个零字节文件"
        );
        assert_eq!(
            repo.out(&["status", "--porcelain", "--", ":(literal)a*.txt"]),
            "D  a*.txt",
            "没有解决成「已暂存的删除」"
        );
        assert!(
            repo.dir.join("ab.txt").exists(),
            "ab.txt 被 `git rm -- 'a*.txt'` 的通配一起删了 —— 用户丢的是一个完全无关的文件"
        );
        assert_eq!(
            std::fs::read_to_string(repo.dir.join("ab.txt")).unwrap(),
            "innocent\n"
        );
    }

    /// 没有冲突的时候必须**报错**，不能悄悄写点什么再 add。
    /// 这是①那条的另一半：不看退出码时，这种调用同样会写一个空文件然后返回 Ok。
    #[test]
    fn resolving_a_file_that_is_not_conflicted_fails_loudly() {
        let repo = Repo::new("noconflict");
        std::fs::write(repo.dir.join("a.txt"), "hello\n").unwrap();
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "base"]);

        let err = git_resolve_conflict(repo.root(), "a.txt".into(), "theirs".into())
            .expect_err("没有冲突却报成功 —— 那意味着刚往文件里写了空内容");
        assert!(err.contains("不是冲突状态"), "错误信息没说清原因：{err}");
        assert_eq!(
            std::fs::read_to_string(repo.dir.join("a.txt")).unwrap(),
            "hello\n",
            "文件被动过了"
        );
    }

    /// 打开的文件夹是仓库的**子目录**时，整条冲突链路要落在正确的文件上。
    ///
    /// 这一条钉的是两个坐标系的错位（见 repo_top_level）：`git diff --diff-filter=U`
    /// 给的 rel 相对**仓库顶层**，而 pathspec 相对 **CWD**。打开 `<repo>/sub` 时：
    /// 老代码把 `<repo>/sub/bar.txt` 的内容写到了 `<repo>/sub/sub/bar.txt`（凭空多一层），
    /// 再 `git add` 那个野文件、返回 Ok(())；加了索引判据之后它改成每次都失败，
    /// 报的却是「当前不是冲突状态」——而同一个面板上一行正把它显示为冲突文件。
    #[test]
    fn a_conflict_resolves_when_the_opened_folder_is_a_subdirectory_of_the_repo() {
        let repo = Repo::new("subdir");
        std::fs::create_dir_all(repo.dir.join("sub")).unwrap();
        std::fs::write(repo.dir.join("sub/bar.txt"), "base\n").unwrap();
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "base"]);
        let main = repo.branch();

        repo.git(&["checkout", "-q", "-b", "other"]);
        std::fs::write(repo.dir.join("sub/bar.txt"), "theirs\n").unwrap();
        repo.git(&["commit", "-q", "-am", "对方改了"]);

        repo.git(&["checkout", "-q", &main]);
        std::fs::write(repo.dir.join("sub/bar.txt"), "ours\n").unwrap();
        repo.git(&["commit", "-q", "-am", "我方改了"]);
        assert!(
            !repo.git(&["merge", "other"]).status.success(),
            "这一步本该冲突，测试前提没造出来"
        );

        // 用户打开的是子目录，后端拿到的 root 就是它。
        let sub = repo.dir.join("sub").to_string_lossy().to_string();
        crate::files::register_workspace_root(sub.clone()).ok();

        let listed = git_conflicts(sub.clone()).expect("列冲突不该失败");
        assert_eq!(
            listed.iter().map(|c| c.rel.as_str()).collect::<Vec<_>>(),
            vec!["sub/bar.txt"],
            "git diff 印的一直是相对仓库顶层的路径，即使从子目录里跑"
        );
        assert!(
            Path::new(&listed[0].path).exists(),
            "面板给出的绝对路径不存在：{} —— 从打开的文件夹拼就会多出一层 sub/",
            listed[0].path
        );

        git_resolve_conflict(sub, "sub/bar.txt".into(), "theirs".into())
            .expect("子目录工作区里解冲突失败了 —— pathspec 还在按 CWD 解析");

        assert_eq!(
            std::fs::read_to_string(repo.dir.join("sub/bar.txt")).unwrap(),
            "theirs\n",
            "内容没落到真正那个冲突文件上"
        );
        assert_eq!(
            repo.out(&["status", "--porcelain", "--", ":(top,literal)sub/bar.txt"]),
            "M  sub/bar.txt",
            "没有解决成「已暂存的修改」"
        );
        assert!(
            !repo.dir.join("sub/sub").exists(),
            "在打开的文件夹底下又拼了一层 sub/ —— 那是老代码写出来的野文件"
        );
    }

    /// 同一个锚点错误的**另一半**：`git_merge_versions` 也从「打开的文件夹」拼路径。
    ///
    /// 上面那条测的是解冲突（写），这条测的是三栏对比里的「合并结果」（读）。两处各自
    /// 有一行 `require_rel_inside_root(...)`，改对一处、漏掉另一处，测试全绿 —— 这条
    /// 就是补那个缺口：把 1192 行的 `&top` 换回 `&PathBuf::from(&root)`，这里必须红。
    ///
    /// 症状不是报错，是**静悄悄的空白**：路径拼成不存在的 `<repo>/sub/sub/bar.txt`，
    /// `read_to_string(...).unwrap_or_default()` 把 io 错误吞成空串，面板上那一栏就
    /// 永远是空的 —— 用户会以为「合并结果真的没内容」。所以断言必须落在**内容**上，
    /// 只断言 Ok 是抓不到的。
    #[test]
    fn merge_versions_reads_the_working_copy_when_the_opened_folder_is_a_subdirectory() {
        let repo = Repo::new("subdir-merge-versions");
        std::fs::create_dir_all(repo.dir.join("sub")).unwrap();
        std::fs::write(repo.dir.join("sub/bar.txt"), "base\n").unwrap();
        repo.git(&["add", "-A"]);
        repo.git(&["commit", "-q", "-m", "base"]);
        let main = repo.branch();

        repo.git(&["checkout", "-q", "-b", "other"]);
        std::fs::write(repo.dir.join("sub/bar.txt"), "theirs\n").unwrap();
        repo.git(&["commit", "-q", "-am", "对方改了"]);

        repo.git(&["checkout", "-q", &main]);
        std::fs::write(repo.dir.join("sub/bar.txt"), "ours\n").unwrap();
        repo.git(&["commit", "-q", "-am", "我方改了"]);
        assert!(
            !repo.git(&["merge", "other"]).status.success(),
            "这一步本该冲突，测试前提没造出来"
        );

        let sub = repo.dir.join("sub").to_string_lossy().to_string();
        crate::files::register_workspace_root(sub.clone()).ok();

        let v = git_merge_versions(sub, "sub/bar.txt".into()).expect("取三方版本不该失败");

        // 索引那三栏一直是按顶层解析的，本来就对——先钉住，免得下面那条红了以后
        // 有人误以为是 git show 坏了。
        assert_eq!(v.base, "base\n", "stage 1 不是基线");
        assert_eq!(v.ours, "ours\n", "stage 2 不是我方");
        assert_eq!(v.theirs, "theirs\n", "stage 3 不是对方");

        // 而这一栏读的是磁盘：锚点错了它就是空串。
        assert!(
            !v.merged.is_empty(),
            "「合并结果」是空的 —— 路径又从打开的文件夹拼了，多出一层 sub/"
        );
        assert!(
            v.merged.contains("<<<<<<<") && v.merged.contains("ours") && v.merged.contains("theirs"),
            "读到的不是那个带冲突标记的工作副本：{:?}",
            v.merged
        );
        assert!(
            !repo.dir.join("sub/sub").exists(),
            "又在打开的文件夹底下拼了一层 sub/"
        );
    }

    /// 读盘这一步要不要再查一次工作区边界 —— 这条是源码断言，不是行为断言，因为
    /// `Repo` 造在 `std::env::temp_dir()` 里，而临时目录是 `has_safe_prefix` 明写的
    /// 例外，在那里边界检查恒为 Ok，行为上造不出被挡的那一侧。
    ///
    /// 挡的是这个形状：仓库顶层在 HOME 之外（外接盘 / /opt 下的仓库），用户打开的是它的
    /// 子目录，于是 `top` 落在工作区外，一条 `../` 之外的 rel 就能把任意文件的内容读进
    /// 合并面板。写那一侧（`git_resolve_conflict`）早就查了，读这一侧不查就是不对称。
    #[test]
    fn merge_versions_still_checks_the_workspace_boundary_after_moving_the_anchor() {
        let src = include_str!("git.rs");
        let body = src
            .split("pub fn git_merge_versions(")
            .nth(1)
            .and_then(|s| s.split("\n#[tauri::command").next())
            .expect("git_merge_versions 的函数体不见了");
        // 先剥注释：上面那几段注释里逐字写着被修掉的旧代码和这里要断言的调用名，
        // 不剥的话断言会被自己的注释喂饱，函数体删空了都还是绿的。
        let code: String = body
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code.contains("require_rel_inside_root(&top, &rel)"),
            "锚点又回到了打开的文件夹上：{code}"
        );
        assert!(
            code.contains("require_inside_workspace(&safe_path.to_string_lossy(), false)"),
            "读盘前不再查工作区边界了 —— 顶层在工作区之外时，任意文件会被读进合并面板：{code}"
        );
    }
}
