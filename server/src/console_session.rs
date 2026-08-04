//! 管理后台门禁。
//!
//! 在此之前，`/console/` 是**完全公开**的：任何人 `GET https://code.mrday.one/console/`
//! 都能拿到整个管理台的 HTML 和 JS。唯一的"你是不是管理员"判断跑在访客自己的浏览器里
//! （`App.tsx` 拿 `/api/me` 看 role），也就是跑在攻击者机器上的一段可以随手改掉的 JS。
//! 接口本身是有服务端鉴权的，所以这不是直接的数据泄露，但它把整张管理接口地图、字段名、
//! 以及一个不限次数的登录表单，白送给了每一个扫到这台机器的人。
//!
//! ## 为什么需要一个新的凭据，而不是复用已有的
//!
//! 浏览器地址栏里敲一个 URL，请求里**不会**带 Authorization 头；控制台的令牌又存在
//! localStorage 里，nginx 读不到。所以在"把 HTML 交出去"的那一刻，边缘层没有任何东西
//! 可以用来判断来访者是谁。这正是这个页面一直裸奔的原因。
//!
//! 现成的两个东西都不能用：
//!   - `mide_token` cookie 是网页版门禁用的，由页面 JS 自己写（gate.html），所以**不可能**
//!     是 HttpOnly；用它当强门禁的凭据等于把门禁降级到那个弱 cookie 的强度。
//!   - 已有的 `/_app_authz` 子请求打的是 `/api/me`，而 `/api/me` 对**任何**登录用户都返回
//!     200，表达不了"仅限管理员"。它还会跑两条 UPDATE，不适合每个请求都触发一次。
//!
//! ## 这里的做法
//!
//! 由服务端签发一个独立的 HttpOnly 会话 cookie：
//!
//!   1. **它不是 JWT。** 是一串随机 id，真正的会话存在 Redis 里。偷到 cookie 只能换到
//!      管理台的静态文件，换不到接口权限——没有任何 handler 接受这个 cookie 当身份。
//!      顺带补上了这套系统一直没有的东西：服务端注销、以及会话吊销。
//!   2. **由后端下发**，所以 HttpOnly 是真能生效的：页面上的 XSS 读不走它。
//!   3. **每次子请求都回数据库读 role**，和 `auth.rs` 的 Claims 提取器同一个口径。
//!      降权或删号在下一个请求就生效，不会留一个到 cookie 过期为止的窗口。
//!
//! ## 为什么这不会让现有接口变得可以被跨站触发（CSRF）
//!
//! 关键在于：**除了门禁子请求之外，没有任何接口读这个 cookie**。所有 `/api/*` 仍然只认
//! Authorization 头，而 Authorization 头不会被浏览器自动带上。所以即使 cookie 被自动
//! 附加到跨站请求上，它也只能换来一个静态文件的 200，动不了任何数据。再加上
//! `SameSite=Strict`，跨站导航连 cookie 都带不过来。

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// 刻意不叫 `mide_token`：那个 cookie 由页面 JS 赋值，永远不可能是 HttpOnly。
pub const CONSOLE_COOKIE: &str = "mide_console";

/// 8 小时。运营台是坐下来用的，不是常驻登录；短一点，丢一台机器的代价就小一点。
const CONSOLE_TTL_SECS: i64 = 8 * 3600;

fn sess_key(tok: &str) -> String {
    format!("console_sess:{tok}")
}

/// 会话 id：两个 v4 UUID 拼成 64 个十六进制字符（约 244 位随机）。
/// 用 uuid 是因为它已经是依赖且底层就是 OS 的 CSPRNG，不必为此多引一个包。
fn new_session_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|p| p.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.to_string())
}

fn set_cookie(value: &str, max_age: i64) -> String {
    // Secure：只走 HTTPS。HttpOnly：JS 读不到。SameSite=Strict：跨站导航不带过来，
    // 所以别的站点没法把访客"拐"进一个已登录的后台。
    format!(
        "{CONSOLE_COOKIE}={value}; Path=/; Max-Age={max_age}; Secure; HttpOnly; SameSite=Strict"
    )
}

/// cookie → Redis 会话 → users.role。role 每次都从行里读，和 `auth.rs` 的 Claims
/// 提取器同一个口径：降权、删号立刻生效。
pub async fn is_admin_request(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(tok) = cookie_value(headers, CONSOLE_COOKIE) else {
        return false;
    };
    // 先做形状检查再打 Redis：不让任意长度的垃圾字符串变成一次网络往返。
    if tok.len() != 64 || !tok.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    let mut conn = state.redis.clone();
    let uid: Option<String> = redis::cmd("GET")
        .arg(sess_key(&tok))
        .query_async(&mut conn)
        .await
        .ok()
        .flatten();
    let Some(uid) = uid.and_then(|s| uuid::Uuid::parse_str(&s).ok()) else {
        return false;
    };
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();
    role.as_deref() == Some("admin")
}

/// `GET /api/admin/authz` —— nginx `auth_request` 的目标。204 = 是管理员，403 = 不是。
///
/// 只读：一次 Redis GET + 一次带索引的 SELECT。它每个受控请求都会跑一次，所以绝不能
/// 换成 `/api/me`（那个会跑两条 UPDATE，而且对任何登录用户都返回 200）。
pub async fn authz(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if is_admin_request(&state, &headers).await {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

/// `POST /api/admin/session` —— 用管理员的 Bearer 令牌换一张门禁 cookie。
///
/// 要求 Authorization 头（`Claims` 提取器），所以它不可能被跨站用"顺带发过去的"凭据
/// 触发——浏览器不会自动附加 Authorization 头。
pub async fn create_session(State(state): State<AppState>, claims: Claims) -> ApiResult<Response> {
    // Claims 提取器已经把 role 从数据库重读过了，这里拿到的不是令牌里那份可能过期的。
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    let tok = new_session_token();
    let mut conn = state.redis.clone();
    let _: () = redis::cmd("SET")
        .arg(sess_key(&tok))
        .arg(&claims.sub)
        .arg("EX")
        .arg(CONSOLE_TTL_SECS)
        .query_async(&mut conn)
        .await?;
    Ok((
        [(header::SET_COOKIE, set_cookie(&tok, CONSOLE_TTL_SECS))],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}

/// `POST /api/admin/session/logout` —— 服务端注销：会话从 Redis 删掉，cookie 立刻作废。
/// 这套系统此前只有"前端把 token 丢掉"这种注销。
pub async fn destroy_session(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    if let Some(tok) = cookie_value(&headers, CONSOLE_COOKIE) {
        if tok.len() == 64 && tok.bytes().all(|b| b.is_ascii_hexdigit()) {
            let mut conn = state.redis.clone();
            let _: Result<i64, _> = redis::cmd("DEL")
                .arg(sess_key(&tok))
                .query_async(&mut conn)
                .await;
        }
    }
    Ok((
        [(header::SET_COOKIE, set_cookie("", 0))],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with(cookie: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, cookie.parse().unwrap());
        h
    }

    #[test]
    fn cookie_is_parsed_out_of_a_crowded_header() {
        let h = headers_with("a=1; mide_console=deadbeef; mide_token=xyz");
        assert_eq!(cookie_value(&h, CONSOLE_COOKIE).as_deref(), Some("deadbeef"));
        assert_eq!(cookie_value(&h, "mide_token").as_deref(), Some("xyz"));
        assert_eq!(cookie_value(&h, "nope"), None);
    }

    /// 前缀不能冒充：`xmide_console` 不是 `mide_console`。
    #[test]
    fn a_similarly_named_cookie_is_not_accepted() {
        let h = headers_with("xmide_console=evil");
        assert_eq!(cookie_value(&h, CONSOLE_COOKIE), None);
    }

    /// 门禁 cookie 的属性是安全性的全部：少一个 HttpOnly，页面上的 XSS 就能读走它；
    /// 少一个 Secure，明文连接就会泄露；少一个 SameSite=Strict，别的站点就能把访客
    /// 带着 cookie 拐进后台。所以这几个字段用测试钉死。
    #[test]
    fn the_cookie_carries_every_protective_attribute() {
        let c = set_cookie("abc", CONSOLE_TTL_SECS);
        assert!(c.contains("HttpOnly"), "{c}");
        assert!(c.contains("Secure"), "{c}");
        assert!(c.contains("SameSite=Strict"), "{c}");
        assert!(c.starts_with("mide_console=abc;"), "{c}");
    }

    /// 注销必须发一张立刻过期的空 cookie，而且属性一个都不能少 —— 否则浏览器会
    /// 认为这是另一张 cookie，旧的那张原样留着。
    #[test]
    fn logout_expires_the_cookie_in_place() {
        let c = set_cookie("", 0);
        assert!(c.contains("Max-Age=0"), "{c}");
        assert!(c.contains("HttpOnly") && c.contains("Secure") && c.contains("SameSite=Strict"));
    }

    /// 会话 id 必须是 64 位十六进制，且每次都不一样 —— `is_admin_request` 的形状
    /// 检查就是按这个来的，两边不能各说各话。
    #[test]
    fn session_tokens_are_64_hex_chars_and_unique() {
        let a = new_session_token();
        let b = new_session_token();
        assert_eq!(a.len(), 64, "{a}");
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()), "{a}");
        assert_ne!(a, b);
    }
}
