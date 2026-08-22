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
//!    │                            │  把「是谁」存到一次性 code 名下
//!    │                            │ ─────── code ──────────▶ │
//!    │ ◀───────────────────────────────────  桌面端亲自打开浏览器到 /gate?handoff=code
//!    │  redeem(code) ───────────▶ │  取走即焚，然后**才**签会话
//!    │ ◀──────────────── token ── │
//! ```
//!
//! 注意 code 名下存的是用户 id，不是一张已经签好的令牌 —— 会话在 redeem 里才建。信任方向
//! 没变（身份仍然只由 App 的令牌决定，请求体里没有任何身份字段），变的是「会话什么时候
//! 存在、按谁的请求头记」：没人来换的 code 不再留下一台幽灵设备，换走的那张也终于记的是
//! 真正在用的那个浏览器的 IP 和 User-Agent，而不是桌面 App 的。
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

/// `POST /api/auth/handoff/offer` — 桌面 App 用自己的登录态换一张一次性交接码。
///
/// **要带 App 自己的令牌**，所以「谁被登录进网页」完全由 App 当前登录的是谁决定；请求体里
/// 没有任何身份字段，调用方不能指定用户。code 名下存的就是这个身份，网页会话要等
/// `redeem` 真的来换才签 —— 理由见下面的注释。
///
/// 返回的 `url` 是给桌面端直接打开的 —— 它必须由桌面端亲自打开，这是整个设计的安全基础：
/// 交接码只走「网关 → 桌面端 → 用户自己的浏览器」，任何第三方都不在这条路径上。
pub async fn offer(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    // code 名下存的是**身份**，不是一张已经签好的令牌。会话在 redeem 里才建。
    //
    // 原来这里就调 start_session 把网页会话签出来了，两个后果：
    //   1. 没人来换的 code（用户取消、浏览器冷启动超过一分钟、App 重复 offer 一次）—— Redis
    //      键一过期，那张令牌谁也拿不到了，可 sessions 行还活着：设备列表上多出一台
    //      「最近活跃」永远停在当时的幽灵 Web 设备，要挂满 30 天令牌寿命。
    //   2. 就算交接成功，会话记的也是**桌面 App 这次请求**的 IP 和 User-Agent —— 一行标着
    //      Web 的设备，指纹却是 App 的，而用户真正在用的是另一个浏览器。
    //
    // 身份仍然只由 claims 决定（调用方不能指定用户），所以信任方向没有变。这里不再查 users
    // 表：提取器已经确认过这个账号还在，而真正要用到 user 行的是 redeem。
    let code = rand_hex(32);
    let mut conn = state.redis.clone();
    // 键名跟着值的含义一起改：`handoff:code:` 存的是签好的 JWT，`handoff:uid:` 存的是用户 id。
    //
    // 只换值不换键名会在**蓝绿重叠期**造出一个比原 bug 更糟的结果。rollout.sh 让新旧两版
    // 同时连着同一个 Redis 最多 120 秒（DRAIN_TRIES=60 × sleep 2），两个方向都会发生：
    //   · 旧 offer → 新 redeem：值是 JWT，parse 成 uuid 失败 → 「交接已过期」。可接受。
    //   · 新 offer → **旧** redeem：旧代码不解析、把取到的字符串**原样当令牌发回去**
    //     （`{"ready":true,"token":"<uuid>"}`），gate.html 拿它写进 .mrday.one 的会话
    //     cookie 然后跳进 /app —— nginx 的 auth_request 当然不认，用户被弹回来，手里还
    //     多了一个垃圾 cookie。这是这次修复自己带出来的回归。
    // 换了键名之后两个方向都落进「取不到 → 已过期，回 App 重试」这一条已经处理好的路。
    let _: () = redis::cmd("SET")
        .arg(format!("handoff:uid:{code}"))
        .arg(uid.to_string())
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
///
/// 会话在这里签，不在 offer 里签：只有真的被换走了才存在一行，而且那一行记的是**这个
/// 浏览器**的 IP 和 User-Agent。
pub async fn redeem(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RedeemReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let code = req.code.trim();
    if !valid_code(code) {
        return Err(AppError::bad("交接码格式不对"));
    }
    let mut conn = state.redis.clone();

    // GETDEL：取值和删除是同一条命令，两个浏览器同时拿同一个 code 也只有一个能换到令牌。
    // 分成 GET + DEL 会留出一个窗口，两边都读到值。
    //
    // 一次性语义仍然由这一步兜底：code 在签会话之前就已经消失了，所以下面任何一步失败都
    // 只是这次交接作废（用户回 App 再点一次），绝不会变成同一个 code 能换两张会话。
    let holder: Option<String> = redis::cmd("GETDEL")
        .arg(format!("handoff:uid:{code}"))
        .query_async(&mut conn)
        .await?;

    let Some(holder) = holder else {
        return Err(AppError::bad("交接已过期或已被使用，请回到 App 重试"));
    };
    // 兼容性靠的是**键名**，不是下面这个 uuid 解析。
    //
    // 旧版本用的键是 `handoff:code:`，值是签好的 JWT。发版那一刻在飞的 code 最多活 60 秒
    // （TTL_SECS），而蓝绿重叠期新旧两版共用同一个 Redis：
    //   · 旧 offer 写的 `handoff:code:` 这里根本读不到 → 直接落进上面那句「已过期」；
    //   · 新 offer 写的 `handoff:uid:` 旧版也读不到 → 它同样回「已过期」。
    // 两个方向都收敛到同一条用户可自愈的路（回 App 再点一次）。如果只换值不换键名，
    // 后一个方向会让旧版把这里的 uuid 原样当令牌发给浏览器，那比失败糟得多（见 offer 的注释）。
    //
    // uuid 解析保留作双保险：万一有谁把旧格式写进了新键名，也不会被当成身份用。
    let uid = uuid::Uuid::parse_str(holder.trim())
        .map_err(|_| AppError::bad("交接已过期或已被使用，请回到 App 重试"))?;
    let user = sqlx::query_as::<_, crate::auth::User>("SELECT * FROM users WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::unauthorized("账号不存在"))?;

    // 走和登录完全相同的那条路 —— 设备列表、撤销、过期全都一致。
    // device 记成 web：这张令牌是给浏览器用的，不是给 App 自己用的。
    // device_id 传 None：换取的请求体里只有 code，浏览器那个 localStorage 里的 id 还没有
    // 办法送过来，所以这一行落成空 device_id，按指纹和这个浏览器自己的登录并成一台设备
    // （见 sessions.rs 的折叠）。要让它带上 id，得先给 /gate 那个页面加字段。
    let token = crate::auth::start_session(&state, &user, &headers, Some("web"), None).await?;
    tracing::info!(%uid, "desktop handoff redeemed");
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

    /// 剥掉注释再断言：注释里写着被修掉的旧写法（「原来这里就调 start_session」），
    /// 连注释一起扫的话，把 bug 放回去测试照样是绿的。
    fn code_of(src: &str) -> String {
        src.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 值的含义变了，键名必须跟着变。
    ///
    /// 这条钉的是一个只在**蓝绿重叠期**出现、而且比原 bug 更糟的回归。rollout.sh 让新旧两版
    /// 同时连同一个 Redis 最多 120 秒。会话改成 redeem 时才签之后，这个键存的从 JWT 变成了
    /// 用户 id；键名若不变，新版 offer 写下的 uuid 会被**旧版** redeem 原样当作令牌发回
    /// 浏览器（旧代码不解析、直接 `{"ready":true,"token":<取到的字符串>}`），gate.html 把它
    /// 写进 .mrday.one 的会话 cookie 再跳进 /app，nginx 当然不认——用户被弹回来，还落下一个
    /// 垃圾 cookie。换了键名之后两个方向都只会「取不到 → 已过期，回 App 重试」。
    #[test]
    fn the_redis_key_name_changed_with_the_value_format() {
        let src = include_str!("handoff.rs");
        let code = code_of(&src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())]);
        assert!(
            !code.contains("handoff:code:"),
            "还在用旧键名 handoff:code:。值已经从 JWT 换成了用户 id，键名不换的话，\
             蓝绿重叠期里旧版会把这个 uuid 当令牌发给浏览器",
        );
        assert_eq!(
            code.matches("handoff:uid:").count(),
            2,
            "offer 存和 redeem 取必须是同一个新键名，各一处",
        );
    }

    /// 网页会话必须由**换取它的那个浏览器**那次请求签出来，不能在 offer 里提前签好。
    ///
    /// 提前签有两个真实后果：
    ///   1. 没人来换的 code —— Redis 键 60 秒后过期，那张令牌谁也拿不到了，可 sessions 行
    ///      还活着，设备列表上多出一台挂满 30 天的幽灵 Web 设备。
    ///   2. 就算交接成功，会话记的也是桌面 App 那次请求的 IP 和 User-Agent —— 一行标着
    ///      Web 的设备，指纹却是 App 的。
    ///
    /// 一次性语义不受影响：GETDEL 仍然在签会话之前，签失败只是这次交接作废。
    #[test]
    fn the_session_is_minted_by_the_browser_that_redeems_not_by_the_offer() {
        let src = code_of(include_str!("handoff.rs"));
        let src = &src[..src.find("#[cfg(test)]").unwrap_or(src.len())];

        let offer = src.split("pub async fn offer(").nth(1).expect("offer");
        let offer = &offer[..offer.find("#[derive").unwrap_or(offer.len())];
        assert!(
            !offer.contains("start_session("),
            "offer 里签会话：没被换走的 code 会留下一台挂满 30 天的幽灵设备",
        );
        assert!(
            offer.contains(".arg(uid.to_string())"),
            "code 名下存的必须是身份，不是一张已经签好的令牌",
        );

        let redeem = src.split("pub async fn redeem(").nth(1).expect("redeem");
        assert!(
            redeem.contains("headers: axum::http::HeaderMap"),
            "会话要按换取方的请求头记，否则设备列表上是 App 的 IP 和 User-Agent",
        );
        assert!(
            redeem.contains("start_session(&state, &user, &headers,"),
            "会话在 redeem 里签，用的是这个浏览器的头",
        );
        let getdel = redeem.find("GETDEL").expect("取走即焚");
        let mint = redeem.find("start_session(").expect("签会话");
        assert!(
            getdel < mint,
            "先取走即焚再签会话；反过来会在失败路径上留下一次没被换走的会话",
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
