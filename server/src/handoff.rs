//! 桌面端 → 网页 的登录交接。
//!
//! 目的：用户已经在桌面 App 里登录了，网页登录页不该再让他输一遍密码。
//!
//! **为什么不走 loopback。** 以前是网页去打 `http://127.0.0.1:47821` 问 App。那条路做不成 ——
//! 见 `ide/src/main.js` 里的记录：App 在跑、在听、CORS 预检正确返回了
//! `Access-Control-Allow-Private-Network`、本地网络权限也授予了，Chrome 依然以一句
//! `TypeError: Failed to fetch` 拒绝。HTTPS 页面访问明文 loopback 端口正是浏览器在关掉的形状。
//!
//! # 方向：桌面端发起，不是网页发起
//!
//! 第一版把方向搞反了，代价是一个账号接管漏洞。当时的流程是「网页生成 nonce+secret →
//! 桌面端替这个 nonce 认领 → 网页凭 secret 取走令牌」，而 `start` 按设计是公开的（调用它的
//! 人正是还没登录的那个页面）。于是任何一个网站都能：
//!
//! ```text
//!   1. 自己调 start 拿到 {nonce, secret}      ← 公开接口，谁都能调
//!   2. 让受害者的浏览器打开 mrday://signin?nonce=…
//!   3. 受害者的桌面端二话不说，带着他的令牌去认领这个 nonce
//!   4. 攻击者用自己手里的 secret 轮询，取走受害者完整的 30 天会话
//! ```
//!
//! 受害者全程无感 —— `claim` 建的是新会话，不顶掉旧的。
//!
//! **校验 Origin 补不上这个洞。** 攻击者可以在自己服务器上用 curl 完成第 1 步和第 4 步，
//! curl 能伪造任何请求头，而受害者的浏览器从头到尾不需要碰 `start`。服务端也无法区分
//! 「攻击者生成的 nonce」和「用户自己那个」—— 两者来源完全一样。
//!
//! 根因是**谁持有密钥、谁决定结果送去哪**。所以把方向反过来：
//!
//! ```text
//!   网页                        网关                       桌面 App
//!    │  打开 mrday://signin ────────────────────────────────▶ │  不带任何凭据，就是喊一声
//!    │                            │ ◀──── offer + App 自己的令牌
//!    │                            │  签一张网页会话，存到一次性 code 名下
//!    │                            │ ─────── code ──────────▶ │
//!    │ ◀───────────────────────────────────  桌面端亲自打开浏览器到 /gate?handoff=code
//!    │  redeem(code) ───────────▶ │
//!    │ ◀──────────────── token ── │  取走即焚
//! ```
//!
//! 现在攻击者触发深链能造成的唯一后果，是受害者自己的浏览器上真的登录了他自己的账号 ——
//! 那正是他点那个按钮想要的。**code 从桌面端直接进入用户自己的浏览器，从不经过发起方**，
//! 所以攻击者没有任何位置可以插进来。也因此不需要弹确认框。
//!
//! 令牌只能被取走一次，一分钟不取就过期。

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// 交接码的有效期。桌面端拿到 code 之后立刻就去开浏览器，正常路径是秒级完成；给一分钟是
/// 为了容忍冷启动的浏览器。再长只是把一张能换会话的票在服务器上多留一会儿。
const TTL_SECS: i64 = 60;

fn rand_hex(bytes: usize) -> String {
    use rand::Rng;
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill(&mut buf[..]);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// 交接码的格式：32 字节十六进制。既当校验，也把它能出现的字符钉死 —— 它要拼进 URL。
fn valid_code(code: &str) -> bool {
    code.len() == 64 && code.chars().all(|c| c.is_ascii_hexdigit())
}

/// `POST /api/auth/handoff/offer` — 桌面 App 用自己的登录态换一张网页会话，装进一次性交接码。
///
/// **要带 App 自己的令牌**，所以「谁被登录进网页」完全由 App 当前登录的是谁决定；请求体里
/// 没有任何身份字段，调用方不能指定用户。
///
/// 返回的 `url` 是给桌面端直接打开的 —— 它必须由桌面端亲自打开，这是整个设计的安全基础：
/// 交接码只走「网关 → 桌面端 → 用户自己的浏览器」，任何第三方都不在这条路径上。
pub async fn offer(
    State(state): State<AppState>,
    claims: Claims,
    headers: axum::http::HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let user = sqlx::query_as::<_, crate::auth::User>("SELECT * FROM users WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::unauthorized("账号不存在"))?;

    // 新会话，走和登录完全相同的那条路 —— 这样设备列表、撤销、过期全都一致。
    // device 记成 web：这张令牌是给浏览器用的，不是给 App 自己用的。
    let token = crate::auth::start_session(&state, &user, &headers, Some("web"), None).await?;

    let code = rand_hex(32);
    let mut conn = state.redis.clone();
    let _: () = redis::cmd("SET")
        .arg(format!("handoff:code:{code}"))
        .arg(&token)
        .arg("EX")
        .arg(TTL_SECS)
        .query_async(&mut conn)
        .await?;

    let base = state.cfg.ide_update_public_base.trim_end_matches('/');
    tracing::info!(%uid, "desktop handoff offered");
    Ok(Json(json!({
        "code": code,
        "url": format!("{base}/gate?handoff={code}"),
        "expires_in": TTL_SECS,
    })))
}

#[derive(Deserialize)]
pub struct RedeemReq {
    pub code: String,
}

/// `POST /api/auth/handoff/redeem` — 网页用交接码取走会话。取走即焚。
///
/// 公开接口：调用它的正是还没登录的那个页面。它不需要再校验别的东西 —— 能拿到 code 就说明
/// 你就是桌面端亲自打开的那个浏览器，而 code 从没离开过这条路径。
pub async fn redeem(
    State(state): State<AppState>,
    Json(req): Json<RedeemReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let code = req.code.trim();
    if !valid_code(code) {
        return Err(AppError::bad("交接码格式不对"));
    }
    let mut conn = state.redis.clone();

    // GETDEL：取值和删除是同一条命令，两个浏览器同时拿同一个 code 也只有一个能换到令牌。
    // 分成 GET + DEL 会留出一个窗口，两边都读到值。
    let token: Option<String> = redis::cmd("GETDEL")
        .arg(format!("handoff:code:{code}"))
        .query_async(&mut conn)
        .await?;

    let Some(token) = token else {
        return Err(AppError::bad("交接已过期或已被使用，请回到 App 重试"));
    };
    Ok(Json(json!({ "ready": true, "token": token })))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 被登录进网页的是谁，只由 App 的令牌决定。
    #[test]
    fn the_web_session_belongs_to_whoever_the_app_is_signed_in_as() {
        let src = include_str!("handoff.rs");
        let f = src.split("pub async fn offer(").nth(1).expect("offer");
        let body = &f[..f.find("\n#[derive").unwrap_or(f.len())];
        assert!(
            body.contains("claims: Claims") && body.contains("uuid::Uuid::parse_str(&claims.sub)"),
            "用户身份必须来自调用方的令牌",
        );
        assert!(
            !body.contains("req.email") && !body.contains("req.user_id"),
            "请求体绝不能携带身份字段",
        );
    }

    /// 一个交接码只能换一次会话。
    ///
    /// 用 GETDEL 而不是 GET 再 DEL：分成两步会留出一个窗口，同一个 code 被两个浏览器同时
    /// 读到，两边都能换走令牌。
    #[test]
    fn a_handoff_code_is_single_use_and_atomic() {
        let src = include_str!("handoff.rs");
        let f = src.split("pub async fn redeem(").nth(1).expect("redeem");
        assert!(
            f.contains(r#"redis::cmd("GETDEL")"#),
            "取走和删除必须是同一条命令",
        );
        assert!(
            !f.contains(r#"redis::cmd("GET")"#),
            "GET + DEL 两步有竞态，同一个 code 可能被换走两次",
        );
    }

    /// 网页不能自己发起交接 —— 这正是上一版被接管的原因。
    ///
    /// 只要存在一个公开的、由调用方生成密钥的入口，任何网站都能生成一对密钥、诱导受害者的
    /// 桌面端去认领，然后用自己手里的那半换走受害者的会话。所以 offer 必须要求登录态，
    /// 而且整个模块里不能再有 start / claim / poll 这类由网页发起的入口。
    #[test]
    fn there_is_no_caller_initiated_entry_point() {
        // 只扫非测试部分：断言里写着这些函数名的字面量，连自己一起扫就永远为真。
        let src = include_str!("handoff.rs");
        let src = &src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())];
        for gone in ["pub async fn start(", "pub async fn claim(", "pub async fn poll("] {
            assert!(
                !src.contains(gone),
                "{gone} 是网页发起式交接的入口，它的存在就是那个接管漏洞",
            );
        }
        let f = src.split("pub async fn offer(").nth(1).expect("offer");
        let sig = &f[..f.find(") ->").unwrap_or(f.len())];
        assert!(
            sig.contains("claims: Claims"),
            "offer 必须要求登录态；不要求就等于把发起权还给了任何调用方",
        );
    }

    #[test]
    fn code_format_is_pinned() {
        assert!(valid_code(&"a".repeat(64)));
        assert!(valid_code(&"0123456789abcdef".repeat(4)));
        // 长度不对、含非十六进制字符、空 —— 全拒。它要拼进 URL。
        assert!(!valid_code(&"a".repeat(63)));
        assert!(!valid_code(&"a".repeat(65)));
        assert!(!valid_code(&"g".repeat(64)));
        assert!(!valid_code(""));
        assert!(!valid_code("../../etc/passwd"));
    }
}
