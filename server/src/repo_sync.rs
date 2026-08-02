//! Guard against the dual-tracking drift hazard.
//!
//! `ide/` is a nested git repository with its own history, and THIS repository also
//! tracks plain copies of the same files (it is not a submodule). Work committed inside
//! `ide/` therefore leaves this repo's committed copies silently stale — which already
//! happened once: this repo's HEAD held a pre-fix `main.js` (a write-gate escape and a
//! verification gate that accepted failing builds) while the inner repo had the fixes.
//!
//! The comparison is one `git ls-tree -r HEAD` listing per side — path + blob SHA —
//! diffed as sets. That makes ADDS and DELETES visible (the first version compared
//! only content of mutually-tracked files and was blind to both — it stayed green
//! while `src/_serve.mjs` existed on one side only), costs two subprocesses instead of
//! two per file (the per-file version spawned ~780 `git show`s, re-read multi-MB blobs
//! every run, and tripled suite wall time), and never reads file contents at all:
//! equal SHAs mean equal bytes.
//!
//! HEAD-to-HEAD, deliberately not the working tree, so a developer mid-edit never sees
//! a red suite; the alarm fires exactly when a commit lands on one side without its
//! mirror on the other. Skips cleanly when either `.git` is absent OR the `git` binary
//! itself is unavailable (minimal CI container): absence of tooling is not drift, and
//! the first version's panic on a missing binary contradicted its own skip philosophy.
//!
//! Files only ONE side tracks are reported as drift too — with one carve-out: paths the
//! other side explicitly .gitignores are that repo's deliberate choice, not drift.

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn repo_root() -> PathBuf {
        // server/ -> repo root
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
    }

    /// `git -C <dir> ls-tree -r HEAD [prefix]` → path (relative to `strip`) → blob SHA.
    /// None = git missing or command failed; treated as "cannot compare", not drift.
    fn head_listing(dir: &Path, prefix: Option<&str>, strip: &str) -> Option<BTreeMap<String, String>> {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(dir).args(["ls-tree", "-r", "-z", "HEAD"]);
        if let Some(p) = prefix {
            cmd.arg(p);
        }
        let out = cmd.output().ok()?;
        if !out.status.success() {
            return None;
        }
        // Entry shape: "<mode> <type> <sha>\t<path>\0". Non-UTF8 paths survive lossy
        // conversion identically on both sides, so they still compare equal.
        let text = String::from_utf8_lossy(&out.stdout);
        let mut map = BTreeMap::new();
        for entry in text.split('\0').filter(|s| !s.is_empty()) {
            let Some((meta, path)) = entry.split_once('\t') else { continue };
            let sha = meta.split_whitespace().nth(2).unwrap_or("");
            let rel = path.strip_prefix(strip).unwrap_or(path);
            map.insert(rel.to_string(), sha.to_string());
        }
        Some(map)
    }

    /// Is `rel` ignored by the repo at `dir`? Used for the one-side-only carve-out.
    fn is_ignored(dir: &Path, rel: &str) -> bool {
        Command::new("git")
            .arg("-C").arg(dir)
            .args(["check-ignore", "-q", rel])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[test]
    fn ide_copies_match_the_inner_repos_head() {
        let root = repo_root();
        let ide = root.join("ide");
        if !ide.join(".git").exists() || !root.join(".git").exists() {
            eprintln!("repo_sync: one side absent — skipping (absence is not drift)");
            return;
        }
        let (Some(inner), Some(outer)) = (
            head_listing(&ide, None, ""),
            head_listing(&root, Some("ide/"), "ide/"),
        ) else {
            eprintln!("repo_sync: git unavailable — skipping (tooling absence is not drift)");
            return;
        };

        let mut diverged = Vec::new(); // both sides, different blob
        let mut inner_only = Vec::new();
        let mut outer_only = Vec::new();
        for (rel, sha) in &inner {
            match outer.get(rel) {
                Some(other) if other != sha => diverged.push(rel.clone()),
                Some(_) => {}
                None => {
                    // The outer repo deliberately ignoring a path is a choice, not drift.
                    if !is_ignored(&root, &format!("ide/{rel}")) {
                        inner_only.push(rel.clone());
                    }
                }
            }
        }
        for rel in outer.keys() {
            if !inner.contains_key(rel) && !is_ignored(&ide, rel) {
                outer_only.push(rel.clone());
            }
        }

        assert!(!inner.is_empty() && !outer.is_empty(), "repo_sync: empty listing — the guard is not guarding");
        let clean = diverged.is_empty() && inner_only.is_empty() && outer_only.is_empty();
        assert!(
            clean,
            "committed ide/ state has DIVERGED between the two repos.\n\
             content differs ({}): {}\n\
             inner repo only ({}): {}\n\
             outer repo only ({}): {}\n\
             A commit landed in one repo without its mirror in the other. Reconcile:\n\
               inner ahead → cd <root> && git add ide/<files> && git commit\n\
               outer ahead → cd ide && git add <files> && git commit\n\
             (Two tracked copies of the same source exist — ide/ is a nested repo, not a\n\
             submodule; this guard keeps their drift loud until that duplication is removed.)",
            diverged.len(), diverged.join(", "),
            inner_only.len(), inner_only.join(", "),
            outer_only.len(), outer_only.join(", "),
        );
    }
}
