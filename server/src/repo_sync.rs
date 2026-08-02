//! Guard against the dual-tracking drift hazard.
//!
//! `ide/` is a nested git repository with its own history, and THIS repository also
//! tracks plain copies of the same files (it is not a submodule). Work committed inside
//! `ide/` therefore leaves this repo's committed copies silently stale — which already
//! happened once: this repo's HEAD held a pre-fix `main.js` (a write-gate escape and a
//! verification gate that accepted failing builds) while the inner repo had the fixes.
//! Anyone building from this repo shipped the unfixed code.
//!
//! The comparison is HEAD-to-HEAD — the inner repo's committed state against this
//! repo's committed copy — deliberately NOT the working tree, so a developer mid-edit
//! never sees a red suite. The alarm fires exactly when a commit lands on one side
//! without its mirror on the other, and stays red until the copies are reconciled.
//!
//! Skips cleanly when either side is absent (fresh clone without the nested `.git`,
//! or a vendored build without the outer repo): absence is not drift.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn repo_root() -> PathBuf {
        // server/ -> repo root
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
    }

    /// `git -C <dir> show HEAD:<rel>` — a repo's committed version of one file.
    fn head_copy(dir: &Path, rel: &str) -> Option<Vec<u8>> {
        let out = Command::new("git")
            .arg("-C").arg(dir)
            .arg("show").arg(format!("HEAD:{rel}"))
            .output().ok()?;
        if !out.status.success() { return None; }
        Some(out.stdout)
    }

    #[test]
    fn ide_copies_match_the_inner_repos_head() {
        let root = repo_root();
        let ide = root.join("ide");
        if !ide.join(".git").exists() || !root.join(".git").exists() {
            eprintln!("repo_sync: one side absent — skipping (absence is not drift)");
            return;
        }
        // Every file the INNER repo has committed. -z avoids quoting surprises.
        let ls = Command::new("git")
            .arg("-C").arg(&ide)
            .args(["ls-tree", "-r", "--name-only", "-z", "HEAD"])
            .output().expect("git ls-tree");
        assert!(ls.status.success(), "git ls-tree failed in ide/");
        let listing = String::from_utf8_lossy(&ls.stdout);

        let mut drifted = Vec::new();
        let mut compared = 0usize;
        for rel in listing.split('\0').filter(|s| !s.is_empty()) {
            let Some(inner) = head_copy(&ide, rel) else { continue };
            // Only files the OUTER repo also tracks can drift; the rest are inner-only.
            let Some(outer) = head_copy(&root, &format!("ide/{rel}")) else { continue };
            compared += 1;
            if inner != outer {
                drifted.push(rel.to_string());
            }
        }
        assert!(compared > 0, "repo_sync: compared zero files — the guard is not guarding");
        assert!(
            drifted.is_empty(),
            "committed ide/ copies have DIVERGED between the two repos for {} file(s):\n  {}\n\
             A commit landed in one repo without its mirror in the other. Reconcile:\n\
               inner ahead → cd <root> && git add ide/<files> && git commit  (sync the outer copy)\n\
               outer ahead → cd ide && git add <files> && git commit          (sync the inner repo)\n\
             Two tracked copies of the same source exist (ide/ is a nested repo, not a\n\
             submodule); this guard keeps their drift loud until that duplication is removed.",
            drifted.len(),
            drifted.join("\n  ")
        );
    }
}
