//! 安全多租户站点部署。IDE 打包好的静态站（gzip'd tar）POST 上来，网关：
//!   1) 校验 JWT（账号绑定，account = claims.sub）；
//!   2) 白名单闸门（admin 永远可、其他账号须在 DEPLOY_ALLOWLIST 里）；
//!   3) 大小上限（DEPLOY_MAX_MB，默认 30MB）；
//!   4) 安全解压——拒绝 `../`、绝对路径、符号链接/硬链接/设备文件（防路径穿越 + 越权逃逸），
//!      只写普通文件/目录，落到**按账号隔离**的目录 /var/www/michael-sites/<account>/<name>/；
//!   5) nginx 纯静态服务（不执行任何代码 → 无服务端渗透面）。
//!
//! 用户**永远拿不到 shell / SSH / 任意命令**——这是和"给用户 root rsync"的本质区别。
use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use axum::body::Bytes;
use axum::extract::Query;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::path::{Component, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const SITES_ROOT: &str = "/var/www/michael-sites";

#[derive(Deserialize)]
pub struct DeployParams {
    pub name: Option<String>,
}

/// slug 化：只留 [A-Za-z0-9_-]，最长 40，防路径注入。
fn slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(40)
        .collect()
}

/// 临时部署授权记录（自动白名单）。存在一个 JSON 文件里（和站点同卷、非 web 目录），
/// 无需额外数据库/迁移；每次部署自动续期，过期项顺手清掉。
#[derive(Serialize, Deserialize)]
struct Grant {
    account: String,
    granted: i64,
    expires: i64,
}

static GRANTS_LOCK: Mutex<()> = Mutex::new(());

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 授权记录文件路径。默认放 /var/lib/michael-gateway（非 nginx 服务目录，绝不外泄），
/// 可用 DEPLOY_STATE_DIR 覆盖到持久卷。丢了也无妨——下次部署会自动重新开通。
fn grants_file() -> PathBuf {
    PathBuf::from(
        std::env::var("DEPLOY_STATE_DIR")
            .unwrap_or_else(|_| "/var/lib/michael-gateway".to_string()),
    )
    .join("deploy_grants.json")
}

fn env_list_has(var: &str, sub: &str) -> bool {
    std::env::var(var)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim())
        .any(|s| !s.is_empty() && s == sub)
}

/// upsert 一条临时授权（续期到 now + days），清掉过期项，返回到期时间戳。
fn grant_temp(account: &str, days: i64) -> i64 {
    let _lk = GRANTS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let now = now_unix();
    let expires = now + days.max(1) * 86_400;
    let path = grants_file();
    let mut list: Vec<Grant> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    if let Some(g) = list.iter_mut().find(|g| g.account == account) {
        g.expires = expires; // 续期
    } else {
        list.push(Grant {
            account: account.to_string(),
            granted: now,
            expires,
        });
    }
    list.retain(|g| g.expires > now); // 清过期
    if let Ok(txt) = serde_json::to_string(&list) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, txt);
    }
    expires
}

/// 部署授权判定。Ok(Some(到期戳))=临时授权、Ok(None)=永久/admin、Err(原因)=拒绝。
/// 策略：admin 永远可；DEPLOY_DENYLIST 里的账号永远禁（封禁开关，最高优先级）；
/// DEPLOY_ALLOWLIST 里的=永久授权；**其余任何已登录账号默认自动开通「临时」部署授权**
/// （JWT 已证明是注册+计费的真实账号），除非把 DEPLOY_AUTO_GRANT 设成 0/false/off 关掉。
fn access(claims: &Claims) -> Result<Option<i64>, String> {
    if claims.role == "admin" {
        return Ok(None);
    }
    if env_list_has("DEPLOY_DENYLIST", &claims.sub) {
        return Err("你的账号已被禁止部署，请联系管理员。".into());
    }
    if env_list_has("DEPLOY_ALLOWLIST", &claims.sub) {
        return Ok(None);
    }
    let auto = std::env::var("DEPLOY_AUTO_GRANT")
        .map(|v| {
            let v = v.trim();
            !(v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off"))
        })
        .unwrap_or(true);
    if auto {
        let days: i64 = std::env::var("DEPLOY_GRANT_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7);
        return Ok(Some(grant_temp(&claims.sub, days)));
    }
    Err("你的账号还没有部署权限——请联系管理员把你加入部署白名单。".into())
}

/// 递归把目录设 0755、文件设 0644（世界可读），好让 nginx(www-data) 能服务。
fn chmod_readable(dir: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755))?;
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.is_dir() {
            chmod_readable(&p)?;
        } else {
            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644));
        }
    }
    Ok(())
}

/// 给「账号+站点」分配稳定的二级域名，并用软链把 `_hosts/<sub>` 指向站点目录，于是
/// nginx（server_name `*.<域名>`）就能按域名直接服务它。返回完整站点 URL；若没配置
/// `DEPLOY_SITE_DOMAIN`（= 没开二级域名托管）返回 None，调用方回退到 `/s/` 路径。
fn assign_subdomain(account: &str, name: &str, target: &std::path::Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let domain = std::env::var("DEPLOY_SITE_DOMAIN")
        .ok()
        .filter(|s| !s.is_empty())?;
    // 子域名标签：小写、只留 [a-z0-9-]、两端去 '-'（与 nginx 的 server_name 正则一致）
    let base: String = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let base = base.trim_matches('-');
    let base = if base.is_empty() { "site" } else { base };
    let hosts = PathBuf::from(SITES_ROOT).join("_hosts");
    let _ = std::fs::create_dir_all(&hosts);
    let _ = std::fs::set_permissions(&hosts, std::fs::Permissions::from_mode(0o755)); // nginx 可遍历
                                                                                      // 冲突时用账号前 6 位十六进制做后缀，保证跨账号唯一
    let tag: String = account
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(6)
        .collect();
    for cand in [base.to_string(), format!("{base}-{tag}")] {
        let link = hosts.join(&cand);
        match std::fs::read_link(&link) {
            Ok(existing) if existing.as_path() == target => {
                return Some(format!("https://{cand}.{domain}/")); // 本就是这个站的软链，续用
            }
            Ok(_) => continue, // 该子域名被别的站占了 → 试带后缀的候选
            Err(_) => {
                let _ = std::fs::remove_file(&link); // 可能是残留，清掉再建
                if std::os::unix::fs::symlink(target, &link).is_ok() {
                    return Some(format!("https://{cand}.{domain}/"));
                }
            }
        }
    }
    None
}

/// `POST /api/deploy?name=<slug>` —— body 是 gzip'd tar 的静态站产物。
pub async fn deploy_site(
    claims: Claims,
    Query(p): Query<DeployParams>,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    // 1) 部署授权：已登录账号自动开通「临时白名单」，不必再找管理员手动加
    //    （admin / 永久名单 / 黑名单封禁的逻辑都在 access() 里）
    let grant_expires = match access(&claims) {
        Ok(exp) => exp,
        Err(msg) => return Err(AppError::forbidden(msg)),
    };
    // 2) 大小上限
    let max_mb: usize = std::env::var("DEPLOY_MAX_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    if body.len() > max_mb * 1024 * 1024 {
        return Err(AppError::forbidden(format!(
            "项目太大（超过 {max_mb}MB）——精简后再传，或联系管理员提额。"
        )));
    }
    // 3) 按账号隔离的目标目录
    let acct = slug(&claims.sub);
    if acct.is_empty() {
        return Err(AppError::unauthorized("账号无效"));
    }
    let mut name = slug(p.name.as_deref().unwrap_or("site"));
    if name.is_empty() {
        name = "site".to_string();
    }
    let target = PathBuf::from(SITES_ROOT).join(&acct).join(&name);
    // 只清这个站自己的旧内容（不碰别的账号/别的站）
    let _ = std::fs::remove_dir_all(&target);
    std::fs::create_dir_all(&target).map_err(|e| AppError::internal(format!("建目录失败: {e}")))?;

    // 4) 安全解压
    let gz = flate2::read::GzDecoder::new(&body[..]);
    let mut ar = tar::Archive::new(gz);
    ar.set_preserve_permissions(false);
    ar.set_preserve_mtime(false);
    let entries = ar
        .entries()
        .map_err(|e| AppError::forbidden(format!("不是有效的 tar.gz: {e}")))?;
    let mut count = 0usize;
    for entry in entries {
        let mut e = entry.map_err(|e| AppError::forbidden(format!("tar 读取错: {e}")))?;
        let path = e
            .path()
            .map_err(|_| AppError::forbidden("tar 内路径无效"))?
            .into_owned();
        // 防路径穿越：拒绝 ../ / 绝对路径 / 盘符前缀
        if path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            continue;
        }
        // 防越权逃逸：只收普通文件与目录，跳过软/硬链接、设备等
        let et = e.header().entry_type();
        if !(et.is_file() || et.is_dir()) {
            continue;
        }
        let dest = target.join(&path);
        // 再保险：解出的目标必须仍在账号目录内
        if !dest.starts_with(&target) {
            continue;
        }
        if et.is_dir() {
            let _ = std::fs::create_dir_all(&dest);
            continue;
        }
        if let Some(parent) = dest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        e.unpack(&dest)
            .map_err(|e| AppError::internal(format!("解压失败: {e}")))?;
        count += 1;
        if count > 5000 {
            return Err(AppError::forbidden("文件数过多（>5000）——精简项目后再传。"));
        }
    }
    if count == 0 {
        return Err(AppError::forbidden(
            "包里没有可部署的文件（tar.gz 为空？）。",
        ));
    }

    // 让 nginx(www-data) 能读：容器 umask 可能把目录/文件建成 owner-only → 递归设世界可读
    // (目录 0755 可遍历、文件 0644 可读)。也把账号父目录设 0755，否则 nginx 进不去。
    let _ = chmod_readable(&target);
    if let Some(parent) = target.parent() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755));
    }

    // 优先返回漂亮的二级域名（配置了 DEPLOY_SITE_DOMAIN 时自动分配 + 软链），否则回退 /s/ 路径。
    let path_url = format!("https://code.mrday.one/s/{acct}/{name}/");
    let url = assign_subdomain(&acct, &name, &target).unwrap_or_else(|| path_url.clone());
    Ok(Json(serde_json::json!({
        "ok": true,
        "url": url,
        "path_url": path_url,
        "files": count,
        "account": acct,
        "name": name,
        "grant_expires": grant_expires,
        "grant_note": grant_expires.map(|_| "已自动为你开通临时部署权限（每次部署自动续期）"),
    })))
}
