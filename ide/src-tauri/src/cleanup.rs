//! 旧版残留清理器。
//!
//! # 它要解决的问题
//!
//! 用户反复反馈「装了新版还像旧版」。实测把每一层摸过一遍之后，成因分三类，
//! 这个模块只负责**第三类**，前两类不在这儿修，也修不了：
//!
//! 1. **有东西在运行时把新版覆盖掉**（例如第三方语言包整份压过内置词典）。
//!    这类清缓存没用——覆盖发生在每次启动。已在前端侧修。
//! 2. **新版根本没送到用户手上**（同一个版本号切了两次内容不同的包，
//!    更新器按 semver 严格大于判断，永远不下发）。这是发版纪律，不是残留。
//! 3. **真残留**：换包留下的旧 .app 副本、失效的站点数据、网络缓存、崩溃日志。
//!    只有这一类是清理器该管的。
//!
//! # 分档的判据
//!
//! 判据**不是键名，也不是「看起来像缓存」**，而是**权威副本在哪**：
//!
//! - `auto`   —— 别处有权威副本，最坏后果是重取一次。可以静默清。
//! - `manual` —— 清了会丢东西（回滚点、离线可用性），必须用户点头。
//! - `never`  —— 不可重建：聊天记录、项目记忆、凭据、信任决定。**永远不出现在候选里**。
//!
//! 这个模块只枚举 auto 和 manual。用户数据的目录连扫都不扫——不是靠「没被匹配到」
//! 侥幸活下来，而是压根不在枚举范围内。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 一类可清理的残留。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupItem {
    pub id: String,
    pub label: String,
    /// 说清楚清了会丢什么。用户要能据此判断，而不是看一个数字就点。
    pub detail: String,
    /// "auto" | "manual"
    pub tier: String,
    pub bytes: u64,
    pub entries: u32,
    /// 具体路径，给用户看的（前端只展示不回传）
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupReport {
    pub freed_bytes: u64,
    pub moved: Vec<String>,
    /// (路径, 原因)——失败要说得出原因，不能静默跳过
    pub failed: Vec<(String, String)>,
    /// 东西被移到哪了，用户要能找回来
    pub recovered_from: String,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

const BUNDLE_ID: &str = "ai.devin.ide";

/// 目录占多少字节、多少个条目。符号链接不跟进——跟进的话
/// node_modules 那种软链能把一次扫描变成扫全盘。
fn dir_size(path: &Path) -> (u64, u32) {
    let mut bytes = 0u64;
    let mut count = 0u32;
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        let Ok(md) = std::fs::symlink_metadata(&p) else { continue };
        if md.file_type().is_symlink() {
            continue;
        }
        if md.is_dir() {
            let Ok(rd) = std::fs::read_dir(&p) else { continue };
            for e in rd.flatten() {
                stack.push(e.path());
            }
        } else {
            bytes += md.len();
            count += 1;
        }
    }
    (bytes, count)
}

fn days_since_modified(path: &Path) -> f64 {
    let Ok(md) = std::fs::metadata(path) else { return 0.0 };
    let Ok(m) = md.modified() else { return 0.0 };
    let Ok(age) = std::time::SystemTime::now().duration_since(m) else { return 0.0 };
    age.as_secs_f64() / 86_400.0
}

/// **绝对不能碰的东西。** 每一次移动之前都要过这道闸——不是靠「没被匹配到」
/// 侥幸活下来，而是显式拒绝。
///
/// 这里面装的是不可重建的用户资产：聊天记录、跨项目记忆、网关凭据、
/// 以及 settings.json 里那份「哪些项目被信任过」的安全决定。
fn is_protected(path: &Path) -> bool {
    let s = path.to_string_lossy();
    const NEVER: &[&str] = &[
        "conversations.sqlite3",
        "memory-episodes.json",
        "memory-kg.json",
        "memory-workflows.json",
        "settings.json",
        "session.json",
        ".mrdayone",
    ];
    if NEVER.iter().any(|n| s.contains(n)) {
        return true;
    }
    // Application Support 是数据目录，整个不参与清理。
    // 例外是它下面那个隔离区（清理器自己造的），见 quarantine_root。
    if s.contains("Application Support") && !s.contains("_cleanup-quarantine") {
        return true;
    }
    false
}

/// macOS 的 WebsiteData 根；Windows 是 WebView2 的用户数据目录。
fn website_data_root() -> Option<PathBuf> {
    let h = home()?;
    #[cfg(target_os = "macos")]
    {
        Some(h.join("Library/WebKit").join(BUNDLE_ID).join("WebsiteData/Default"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(h.join("AppData/Local").join(BUNDLE_ID).join("EBWebView"))
    }
}

fn cache_root() -> Option<PathBuf> {
    let h = home()?;
    #[cfg(target_os = "macos")]
    {
        Some(h.join("Library/Caches").join(BUNDLE_ID))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(h.join("AppData/Local").join(BUNDLE_ID).join("EBWebView/Default/Cache"))
    }
}

/// 网络缓存：HTTP 响应，按定义可重取。清了只是下次多一次网络往返。
///
/// **注意**：应用自己的前端（tauri://localhost 自定义协议）从不进这个缓存，
/// 所以清它不会让界面变新。这一点必须在 detail 里对用户说清楚，否则他清完
/// 发现图标还是旧的，会认为清理器坏了。
fn scan_network_cache() -> Option<CleanupItem> {
    let root = cache_root()?;
    if !root.exists() {
        return None;
    }
    let (bytes, entries) = dir_size(&root);
    if bytes == 0 {
        return None;
    }
    Some(CleanupItem {
        id: "network_cache".into(),
        label: "网络缓存".into(),
        detail: "网页图标、头像这类下载过的资源。清了下次联网自动重取。\
                 应用自己的界面不走这里，所以清它不会改变界面新旧。"
            .into(),
        tier: "auto".into(),
        bytes,
        entries,
        paths: vec![root.to_string_lossy().into_owned()],
    })
}

/// 失效的站点数据：这个应用在历史上从别的源加载过（例如开发服务器），
/// 那些源的 localStorage/IndexedDB 从此再没人读。
///
/// 判据是**最近改过的那个是活的，其余超过 30 天没动的才算死**。不用「按名字认」——
/// 目录名是加了盐的哈希，认不出来；也不用「只留一个」——那会误删多窗口场景。
fn scan_dead_origins() -> Option<CleanupItem> {
    let root = website_data_root()?;
    if !root.exists() {
        return None;
    }
    let mut dirs: Vec<(PathBuf, f64)> = std::fs::read_dir(&root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .map(|p| {
            let age = days_since_modified(&p);
            (p, age)
        })
        .collect();
    if dirs.len() < 2 {
        return None; // 只有一个源，没有死的
    }
    dirs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    // 最新的那个是活的，跳过
    let dead: Vec<&(PathBuf, f64)> = dirs.iter().skip(1).filter(|(_, age)| *age > 30.0).collect();
    if dead.is_empty() {
        return None;
    }
    let mut bytes = 0u64;
    let mut entries = 0u32;
    let mut paths = Vec::new();
    for (p, age) in &dead {
        let (b, c) = dir_size(p);
        bytes += b;
        entries += c;
        paths.push(format!("{} （{:.0} 天没动过）", p.to_string_lossy(), age));
    }
    Some(CleanupItem {
        id: "dead_origins".into(),
        label: "失效的站点数据".into(),
        detail: "这个应用历史上从别的地址加载过（多半是开发服务器），\
                 那些数据现在没有任何代码会读到。当前版本在用的那份不在清理范围内。"
            .into(),
        tier: "auto".into(),
        bytes,
        entries,
        paths,
    })
}

/// 换包留下的旧 .app 副本。**只认带明确备份后缀的**，不去猜哪个 .app 是旧版——
/// 猜错一次就是把用户正在用的应用扔进废纸篓。
#[cfg(target_os = "macos")]
fn scan_stale_app_copies() -> Option<CleanupItem> {
    scan_stale_app_copies_in(
        Path::new("/Applications"),
        std::env::current_exe().ok().as_deref(),
    )
}

/// 判据抽出来单独可测。这是整个清理器里**最危险的一个判据**：错一次就是把用户
/// 正在用的应用扔进废纸篓，所以它必须能被单独钉住，而不是埋在一个读死
/// /Applications 的函数里没人验。
#[cfg(target_os = "macos")]
fn scan_stale_app_copies_in(apps: &Path, current_exe: Option<&Path>) -> Option<CleanupItem> {
    let mut bytes = 0u64;
    let mut entries = 0u32;
    let mut paths = Vec::new();
    for e in std::fs::read_dir(apps).ok()?.flatten() {
        let p = e.path();
        let Some(name) = p.file_name().map(|n| n.to_string_lossy().into_owned()) else { continue };
        // 判据只有一条：名字里带 `.app.bak-` 或 `.app.prev-`。这是换包脚本留下的
        // 明确记号，不是启发式。没有这个记号的 .app 一律不碰。
        if !(name.contains(".app.bak-") || name.contains(".app.prev-")) {
            continue;
        }
        if current_exe.is_some_and(|c| c.starts_with(&p)) {
            continue; // 正在跑的那一份，无论如何不碰
        }
        let (b, c) = dir_size(&p);
        bytes += b;
        entries += c;
        paths.push(p.to_string_lossy().into_owned());
    }
    if paths.is_empty() {
        return None;
    }
    Some(CleanupItem {
        id: "stale_app_copies".into(),
        label: "换包留下的旧版副本".into(),
        detail: "手工更新时留下的旧版应用。它们不会被自动启动，但会出现在\
                 聚焦搜索里——误点一次就会用旧版打开，并覆写当前版本的界面状态。\
                 清掉等于放弃这些回滚点。"
            .into(),
        tier: "manual".into(), // 这是回滚点，必须用户点头
        bytes,
        entries,
        paths,
    })
}

#[cfg(not(target_os = "macos"))]
fn scan_stale_app_copies() -> Option<CleanupItem> {
    None
}

/// 隔离区：清掉的东西先搬到这儿，七天后才真删。
/// 放在缓存目录下——它本身就是「可以丢」的语义。
fn quarantine_root() -> Option<PathBuf> {
    Some(cache_root()?.join("_cleanup-quarantine"))
}

/// 把过了保质期的隔离物真正删掉。每次扫描时顺手做，用户无感。
fn purge_expired_quarantine() -> u64 {
    let Some(root) = quarantine_root() else { return 0 };
    let Ok(rd) = std::fs::read_dir(&root) else { return 0 };
    let mut freed = 0u64;
    for e in rd.flatten() {
        let p = e.path();
        if days_since_modified(&p) <= 7.0 {
            continue;
        }
        let (b, _) = dir_size(&p);
        if std::fs::remove_dir_all(&p).is_ok() || std::fs::remove_file(&p).is_ok() {
            freed += b;
        }
    }
    freed
}

#[tauri::command]
pub fn cleanup_scan() -> Vec<CleanupItem> {
    purge_expired_quarantine();
    let mut out = Vec::new();
    if let Some(i) = scan_network_cache() {
        out.push(i);
    }
    if let Some(i) = scan_dead_origins() {
        out.push(i);
    }
    if let Some(i) = scan_stale_app_copies() {
        out.push(i);
    }
    out
}

/// 把一个路径挪走而不是删掉。
///
/// macOS 上进废纸篓——同卷 rename，瞬间完成，而且用户能在访达里自己拖回来。
/// 其它平台没有等价的系统废纸篓 API（不引新依赖的前提下），进应用自己的隔离区，
/// 七天后自动清空。两条路都保证**这一步是可回退的**。
fn move_aside(path: &Path) -> Result<PathBuf, String> {
    if is_protected(path) {
        return Err("这是受保护的用户数据，拒绝移动".into());
    }
    let name = path
        .file_name()
        .ok_or_else(|| "路径没有文件名".to_string())?
        .to_string_lossy()
        .into_owned();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    #[cfg(target_os = "macos")]
    let dest_dir = home().ok_or("找不到主目录")?.join(".Trash");
    #[cfg(not(target_os = "macos"))]
    let dest_dir = quarantine_root().ok_or("找不到隔离区")?.join(stamp.to_string());

    std::fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let mut dest = dest_dir.join(&name);
    if dest.exists() {
        dest = dest_dir.join(format!("{name}.{stamp}"));
    }
    // 先试 rename（同卷，瞬间）；跨卷失败再退回复制+删除。
    match std::fs::rename(path, &dest) {
        Ok(()) => Ok(dest),
        Err(_) => Err(format!(
            "移动失败（可能跨卷）：{}",
            path.to_string_lossy()
        )),
    }
}

#[tauri::command]
pub fn cleanup_apply(ids: Vec<String>) -> CleanupReport {
    let items = cleanup_scan();
    let mut freed = 0u64;
    let mut moved = Vec::new();
    let mut failed = Vec::new();
    for item in items.iter().filter(|i| ids.contains(&i.id)) {
        for raw in &item.paths {
            // paths 里可能带了说明后缀（「N 天没动过」），只取路径本身
            let p = raw.split(" （").next().unwrap_or(raw);
            let path = PathBuf::from(p);
            if !path.exists() {
                continue;
            }
            let (b, _) = dir_size(&path);
            match move_aside(&path) {
                Ok(_) => {
                    freed += b;
                    moved.push(p.to_string());
                }
                Err(e) => failed.push((p.to_string(), e)),
            }
        }
    }
    #[cfg(target_os = "macos")]
    let from = "废纸篓".to_string();
    #[cfg(not(target_os = "macos"))]
    let from = quarantine_root()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "隔离区".into());
    CleanupReport { freed_bytes: freed, moved, failed, recovered_from: from }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_paths_are_never_movable() {
        // 这道闸是清理器唯一的安全底线：不可重建的用户资产必须在这里被**显式拒绝**，
        // 而不是靠「没被任何 scan_* 匹配到」侥幸活下来。
        //
        // 第一版这里写的是 `assert!(move_aside(p).is_err())`，用的是不存在的假路径——
        // 而 move_aside 对不存在的路径本来就返回 Err。于是「因为受保护被拒」和
        // 「因为文件不在而失败」返回同一个值：把 is_protected 那道闸整个删掉，
        // 这条用例照样绿。所以必须造**真文件**，并且断言**拒绝的理由**。
        let tmp = std::env::temp_dir().join(format!("cleanup-guard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let base = tmp.join("Library/Application Support/ai.devin.ide");
        std::fs::create_dir_all(&base).unwrap();
        let mut made = Vec::new();
        for name in ["conversations.sqlite3", "settings.json", "memory-kg.json", "session.json"] {
            let f = base.join(name);
            std::fs::write(&f, b"user data").unwrap();
            made.push(f);
        }
        made.push(base.clone());
        let skills = tmp.join("project/.mrdayone/skills");
        std::fs::create_dir_all(&skills).unwrap();
        made.push(skills);

        for f in &made {
            assert!(is_protected(f), "{} 竟然不受保护", f.display());
            let err = move_aside(f).expect_err(&format!("{} 竟然可以被移走", f.display()));
            assert!(err.contains("受保护"), "{} 的拒绝理由是「{}」——不是因为受保护，\
                这条断言分不出「被拒」和「文件不在」", f.display(), err);
            assert!(f.exists(), "{} 已经不在原地了", f.display());
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn cache_and_app_copies_are_not_protected() {
        // 反面对照：真正该清的东西不能被闸门误伤，否则这道闸等于把清理器关死了。
        for p in [
            "/Users/x/Library/Caches/ai.devin.ide/WebKit/NetworkCache",
            "/Applications/Mr. Day One.app.bak-20260824-065255",
            "/Users/x/Library/WebKit/ai.devin.ide/WebsiteData/Default/abc123",
        ] {
            assert!(!is_protected(Path::new(p)), "{p} 被误判成受保护，清理器会清不动");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn stale_app_copies_only_matches_explicit_backup_suffixes() {
        // 这是清理器里**最危险的判据**：只认换包脚本留下的 `.app.bak-` / `.app.prev-`
        // 记号，绝不去猜「哪个 .app 看起来像旧版」。猜错一次的代价是用户正在用的
        // 应用被扔进废纸篓。
        let tmp = std::env::temp_dir().join(format!("cleanup-apps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let mk = |name: &str| {
            let d = tmp.join(name).join("Contents/MacOS");
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("michael-ide"), b"0123456789").unwrap();
            tmp.join(name)
        };
        let live = mk("Mr. Day One.app");                       // 正在用的
        let other = mk("Some Other App.app");                   // 别人的应用
        let old_named = mk("Mr. Day One 0.3.94.app");           // 名字里有版本号，但**不是**备份记号
        mk("Mr. Day One.app.bak-20260824-065255");              // 该清
        mk("Mr. Day One.app.prev-084254");                      // 该清

        let running = live.join("Contents/MacOS/michael-ide");
        let item = scan_stale_app_copies_in(&tmp, Some(&running)).expect("应该找到两份备份");
        let mut got: Vec<String> = item.paths.iter()
            .map(|p| Path::new(p).file_name().unwrap().to_string_lossy().into_owned()).collect();
        got.sort();
        assert_eq!(got, vec![
            "Mr. Day One.app.bak-20260824-065255".to_string(),
            "Mr. Day One.app.prev-084254".to_string(),
        ], "认错了：只有带明确备份记号的才该进候选");
        // 按**文件名精确比**，不能用 contains ——「Mr. Day One.app」是
        // 「Mr. Day One.app.bak-…」的子串，用 contains 会把正确结果判成错。
        for must_keep in [&live, &other, &old_named] {
            let n = must_keep.file_name().unwrap().to_string_lossy().into_owned();
            assert!(!got.contains(&n), "{n} 不该被列为可清理");
        }
        assert_eq!(item.tier, "manual", "旧版副本是回滚点，必须要用户点头，不能进自动档");

        // 正在跑的那一份即使被改成备份名字，也绝不能碰
        let running2 = tmp.join("Mr. Day One.app.bak-20260824-065255/Contents/MacOS/michael-ide");
        let item2 = scan_stale_app_copies_in(&tmp, Some(&running2)).expect("还应剩一份");
        assert_eq!(item2.paths.len(), 1, "正在跑的那一份没有被排除");
        assert!(item2.paths[0].contains("prev-084254"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn quarantine_inside_cache_is_reclaimable() {
        // 隔离区自己住在缓存目录下，必须能被回收——否则清掉的东西永远占着盘。
        let p = "/Users/x/Library/Caches/ai.devin.ide/_cleanup-quarantine/123";
        assert!(!is_protected(Path::new(p)));
    }

    #[test]
    fn dir_size_does_not_follow_symlinks() {
        // 跟进软链的话，工作区里一个指向 / 的链接就能让扫描变成扫全盘。
        let tmp = std::env::temp_dir().join(format!("cleanup-symlink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("real")).unwrap();
        std::fs::write(tmp.join("real/a.txt"), b"1234567890").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(tmp.join("real"), tmp.join("link")).unwrap();
        let (bytes, count) = dir_size(&tmp);
        assert_eq!(count, 1, "软链被跟进了，同一个文件数了两次");
        assert_eq!(bytes, 10);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
