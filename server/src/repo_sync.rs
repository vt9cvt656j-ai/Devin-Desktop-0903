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

/// 测试模块里「写成测试的样子却没有 `#[test]`」的函数，一个都不许有。
///
/// **这不是假想的风险，本仓已经吃过两次。** `drained_pool_makes_the_gate_reachable_again`
/// 和 `the_topup_rate_is_per_plan_not_a_single_exchange_rate` 各自漏了 `#[test]`，于是它们
/// 只是普通私有函数：`cargo test` 全绿、`--list` 里一次都不出现。前者是「用户没钱还能用」
/// 那个修复的**全部行为证据**——也就是说那个修复上线时，守卫是不存在的。
///
/// 两次的成因相同：新测试插在旧测试上方时，`#[test]` 和文档注释交错了，一个函数头上
/// 落了两个属性（同一条因此注册两遍），而下一个函数一个都没有。全仓当时有 8 处这种叠加。
///
/// 编译器帮不上忙：私有函数只要在同模块里被引用得到就不算 dead_code，引用不到也只是
/// 一条 warning，混在几十条里没人看见。
///
/// 判据刻意收窄成「无参 + 函数体里有断言 + 前面没有 `#[test]`」。辅助函数
/// （`src()`、`fn_body()`、`code_of()` 这类）几乎都带参数或没有断言，不会误报；
/// 真漏了属性的测试全都长这个样子。宁可漏报也不误报——误报多了这道闸会被关掉。
#[test]
fn every_assertion_bearing_test_function_is_actually_registered() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut orphans: Vec<String> = Vec::new();
    let mut scanned = 0usize;

    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("read src/")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "rs").unwrap_or(false))
        .collect();
    files.sort();

    for path in files {
        let src = std::fs::read_to_string(&path).expect("read source");
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let bytes: Vec<char> = src.chars().collect();
        let mut idx = 0usize;
        // 逐行找 `fn foo() {`（无参），再取它的函数体。
        for (lineno, line) in src.lines().enumerate() {
            let t = line.trim_start();
            let sig = t.strip_prefix("pub ").unwrap_or(t);
            let Some(rest) = sig.strip_prefix("fn ") else {
                idx += line.chars().count() + 1;
                continue;
            };
            // 无参**且无返回值**。加「无返回值」这一条是因为辅助函数常常也没有参数、
            // body 里也带断言（`per_call_readonly_types() -> HashSet<String>` 就在断言
            // 自己的解析器没坏），而 `#[test]` 函数按惯例返回 `()`。
            // 代价是漏掉 `fn x() -> Result<(), E>` 形状的孤儿；本仓两次真事故都不是那种，
            // 而误报会让这道闸被关掉，所以往窄了收。
            let sig_ok = rest
                .split_once('(')
                .map(|(_, a)| a.trim_start().starts_with(')'))
                .unwrap_or(false)
                && !rest.contains("->");
            if !sig_ok {
                idx += line.chars().count() + 1;
                continue;
            }
            // 函数体：从这一行的 `{` 起按花括号配平。
            let body = {
                let start = idx;
                let mut depth = 0i32;
                let mut end = start;
                let mut seen = false;
                for i in start..bytes.len() {
                    match bytes[i] {
                        '{' => { depth += 1; seen = true; }
                        '}' => {
                            depth -= 1;
                            if seen && depth == 0 { end = i; break; }
                        }
                        _ => {}
                    }
                }
                bytes[start..end.max(start)].iter().collect::<String>()
            };
            if !body.contains("assert") {
                idx += line.chars().count() + 1;
                continue;
            }
            scanned += 1;
            // 紧邻这个 fn 的属性/注释块（到上一个空行为止）里有没有 #[test]。
            // **必须先剥掉注释**：本仓的测试文档里就写着「少了 `#[test]`」这句话，
            // 不剥的话判据被自己的说明喂饱 —— 那正是这类闸门最常见的失效形状。
            let before: String = src.lines().take(lineno).collect::<Vec<_>>().join("\n");
            let block = before.rsplit("\n\n").next().unwrap_or("").to_string();
            let code: String = block
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            if !code.contains("#[test]") && !code.contains("#[tokio::test]") {
                let fname = rest.split('(').next().unwrap_or(rest);
                orphans.push(format!("{name}:{}  {fname}()", lineno + 1));
            }
            idx += line.chars().count() + 1;
        }
    }

    // 阳性对照：判据必须真的看见了一批测试。数到 0 个的话上面的扫描坏了，
    // 而它坏掉的表现恰好是「一切正常」。
    assert!(
        scanned >= 200,
        "只扫到 {scanned} 个带断言的无参函数 —— 扫描器坏了，这条断言不作数"
    );
    assert!(
        orphans.is_empty(),
        "下面这些函数有断言、却没有 #[test]，它们一次都不会跑：\n  {}\n\
         补上 #[test]；如果它其实是辅助函数，给它加个参数或改名以示区别。",
        orphans.join("\n  ")
    );
}
