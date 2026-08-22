//! Where this account is signed in, and how to sign one of them out.
//!
//! The rows are written by `auth::start_session` on every sign-in. Nothing here trusts
//! the client: both handlers scope every query to the caller's own `user_id`, so an id
//! belonging to somebody else's session simply does not match.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// 「这张令牌背后的那次登录还在不在」—— 认证链上共用的那一条判定，只此一份文本。
///
/// `$2::uuid IS NULL` 那一支放行 sessions 表出现之前签发的老令牌：它们没有 sid，没有行
/// 可查，按行判会把这批人全部登出。其余一律要求 `revoked_at IS NULL`，并且那一行必须属于
/// 这个账号 —— 少了 `user_id = $1`，别人的一条活会话就能替这张令牌作证。用户行本身没了
/// （销号）时整条 SELECT 返回零行，调用方一并判死。
///
/// 抽成常量而不是各处照抄，是因为照抄这件事在这个仓库已经出过两次事，且两次都不是逻辑
/// 写错、而是「同一条规则有多份实现」：`user_from_jwt` 那份当初根本没写这条判定，于是
/// 用户点完「注销该设备」，`/api/me` 老实回 401，同一张令牌却还能继续烧他的额度到 30 天
/// 期满；`/ws` 那份也没写，被注销的管理员 socket 照样把全站邮箱和订单/佣金金额推满 30 天。
///
/// 收编进度（2026-08-22）：`realtime` 的复查已经改调 `session_is_live`。`auth.rs` 里
/// `user_from_jwt` 和 `claims_from_jwt` 那两份仍是手抄的字面量 —— auth.rs 不在这轮的改动
/// 范围内，所以先用 `liveness_tests` 逐字钉住，各自只差一行就能改成调这里。`Claims` 提取器
/// 那一份**不打算**收编：它把同一条判定嵌在一条顺带读 role、顺带刷 last_seen_at 的语句里，
/// 拆出来等于每个已认证请求多一次数据库往返，那条语句每天跑几十万次。
pub(crate) const SESSION_LIVE_SQL: &str = "SELECT ($2::uuid IS NULL OR EXISTS ( \
     SELECT 1 FROM sessions \
     WHERE id = $2 AND user_id = $1 AND revoked_at IS NULL \
 )) AS live \
 FROM users WHERE id = $1";

/// 查不出来就算「不在」。
///
/// 数据库抖一下、超时、连接池排空 —— 这里都返回 false，向安全侧倒。放行的代价是被注销的
/// 令牌多活一个复查周期（`/ws` 是 5 分钟），拒绝的代价只是调用方重连或重新登录一次。
/// `auth::user_from_jwt` / `claims_from_jwt` 用 `.ok()?` 表达的是同一件事。
pub(crate) async fn session_is_live(
    db: &sqlx::PgPool,
    uid: uuid::Uuid,
    sid: Option<uuid::Uuid>,
) -> bool {
    let live: Option<(bool,)> = sqlx::query_as(SESSION_LIVE_SQL)
        .bind(uid)
        .bind(sid)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
    matches!(live, Some((true,)))
}

/// A readable name for a browser, from its User-Agent.
///
/// Deliberately short and deliberately incomplete. The point is to help someone answer
/// "is that me?", which needs "Chrome on macOS", not a version string. Order matters:
/// Edge and Chrome both claim Safari, and Edge also claims Chrome, so the most specific
/// has to be tested first or every browser reports as Safari.
fn browser_of(ua: &str) -> Option<&'static str> {
    let ua = ua.to_ascii_lowercase();
    for (needle, name) in [
        ("edg/", "Edge"),
        ("opr/", "Opera"),
        ("firefox/", "Firefox"),
        ("chrome/", "Chrome"),
        ("safari/", "Safari"),
    ] {
        if ua.contains(needle) {
            return Some(name);
        }
    }
    None
}

fn platform_of(ua: &str) -> Option<&'static str> {
    let ua = ua.to_ascii_lowercase();
    for (needle, name) in [
        // Before "mac", because an iPad's User-Agent contains "Macintosh" too.
        ("iphone", "iPhone"),
        ("ipad", "iPad"),
        ("android", "Android"),
        ("windows", "Windows"),
        ("mac os", "macOS"),
        ("macintosh", "macOS"),
        ("linux", "Linux"),
    ] {
        if ua.contains(needle) {
            return Some(name);
        }
    }
    None
}

/// What the row is called in the list: "Chrome on macOS", "Desktop app on Windows", or
/// just the kind when the User-Agent says nothing useful.
pub(crate) fn label_for(kind: &str, ua: &str) -> String {
    let what = match kind {
        "desktop" => "Desktop app".to_owned(),
        "mobile" => browser_of(ua).map(str::to_owned).unwrap_or_else(|| "Mobile app".to_owned()),
        _ => browser_of(ua).map(str::to_owned).unwrap_or_else(|| "Web".to_owned()),
    };
    match platform_of(ua) {
        Some(os) => format!("{what} on {os}"),
        None => what,
    }
}

/// Which rows are the same machine.
///
/// The client's own id when it sent one. Otherwise what the row *displays* — browser,
/// platform and address — which is the best available guess for anything that signed in
/// before device ids existed.
///
/// The fallback deliberately ignores the version numbers in the User-Agent. Grouping on
/// the raw string looks stricter but produces exactly the bug it is meant to fix: this
/// account had four live sessions at one address and two User-Agents, because Chrome
/// updated itself between sign-ins. Two rows that both read "Chrome · macOS" at the same
/// IP are one laptop as far as anyone reading the page is concerned, and showing them
/// separately is the page contradicting itself.
///
/// It is imperfect in both directions — two laptops behind one office NAT running the
/// same browser look identical, and a phone that changes networks looks like two devices
/// — but it only ever applies to rows old enough to predate the id, and it is much closer
/// to the truth than listing one laptop once per sign-in.
///
/// NUL separates the parts so a value containing the separator cannot be crafted to
/// collide with a different combination.
fn device_group(device_id: &str, kind: &str, user_agent: &str, ip: &str) -> String {
    if device_id.is_empty() {
        format!(
            "ua\u{0}{kind}\u{0}{}\u{0}{}\u{0}{ip}",
            browser_of(user_agent).unwrap_or(""),
            platform_of(user_agent).unwrap_or(""),
        )
    } else {
        format!("id\u{0}{device_id}")
    }
}

/*
 * 把一批会话行折成设备键：返回值和入参一一对应，第 i 个键就是第 i 行所属的那台设备。
 *
 * 行必须按 created_at 倒序传进来 —— 两个处理器都是这么读表的，别名表也依赖这个顺序。
 *
 * 分组键有两套：有 device_id 的按 id 分，没有的按「类型+浏览器+平台+IP」这个指纹分。
 * device_id 是后来才加的，所以同一个浏览器里早期登录没有它、现在的登录有 —— 两把钥匙
 * 落进两个组，设备列表上就出现两行一模一样的 Chrome·macOS、同一个 IP，其中一行的
 * 「最近活跃」永远停在几天前。别名表把它们并起来：倒序意味着带 id 的那条（较新）先到，
 * 先登记自己的指纹，后面指纹相同的老会话就并进它。折叠只改「怎么归成一台设备」，
 * 不撤销任何会话。
 *
 * **这段必须只有一份实现。** 列表页用它决定「哪些行画成同一台设备」，「注销此设备」用它
 * 决定「哪些行一起撤销」。两边各写一遍的代价已经付过了：revoke 那份漏了别名这一步，
 * 只按 device_group 比，于是被并进当前设备的那条老会话撤不掉 —— 页面上设备已经消失，
 * 它的令牌还能再用满 30 天（JWT 30 天，device_id 才上线 8 天）。2026-08-22 线上有 5 个
 * 账号正处在这个状态。只要这两个答案不是同一段代码算出来的，它们就会再次分叉。
 */
fn fold_device_keys<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a str, &'a str, &'a str)>,
) -> Vec<String> {
    let mut alias: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut keys: Vec<String> = Vec::new();
    for (device_id, kind, user_agent, ip) in rows {
        let fingerprint = device_group("", kind, user_agent, ip);
        keys.push(if device_id.is_empty() {
            // 老会话：能认领就认领，认不到就自成一组。
            alias.get(&fingerprint).cloned().unwrap_or(fingerprint)
        } else {
            let k = device_group(device_id, kind, user_agent, ip);
            alias.entry(fingerprint).or_insert_with(|| k.clone());
            k
        });
    }
    keys
}

/// 撤销时读出来的一行：id、kind、user_agent、ip、device_id。
type LiveRow = (uuid::Uuid, String, String, String, String);

/// 和 `id` 属于同一台设备的所有行 —— 也就是列表页会画成同一行的那一组。
///
/// `None` 表示这个 id 不在给进来的活会话里。行同样必须按 created_at 倒序，和 `list`
/// 读表的顺序一致：别名表认的是「先到的那条带 id 的行」，顺序换了折出来的组也会换。
fn same_device(rows: &[LiveRow], id: uuid::Uuid) -> Option<Vec<uuid::Uuid>> {
    let keys = fold_device_keys(
        rows.iter()
            .map(|r| (r.4.as_str(), r.1.as_str(), r.2.as_str(), r.3.as_str())),
    );
    let at = rows.iter().position(|r| r.0 == id)?;
    let wanted = &keys[at];
    Some(
        rows.iter()
            .zip(&keys)
            .filter(|&(_, k)| k == wanted)
            .map(|(r, _)| r.0)
            .collect(),
    )
}

/// `GET /api/sessions` — the account's signed-in devices, most recent first.
///
/// One row per device, not per sign-in. Signing in again on a device the account is
/// already signed in on now replaces that device's session, but rows written before that
/// was true are still out there in threes, so they are collapsed here as well.
///
/// Revoked rows are excluded, and so are ones older than a token can live: a session
/// whose token expired weeks ago is not somewhere you are still signed in, and listing
/// it would invite people to "revoke" something that already stopped working.
pub async fn list(State(state): State<AppState>, claims: Claims) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let current = claims.sid.as_deref().and_then(|s| uuid::Uuid::parse_str(s).ok());

    type Row = (
        uuid::Uuid,
        String,
        String,
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
        String,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, kind, user_agent, ip, created_at, last_seen_at, device_id \
         FROM sessions \
         WHERE user_id = $1 AND revoked_at IS NULL \
           AND created_at > now() - make_interval(secs => $2) \
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(uid)
    .bind(state.cfg.jwt_ttl_secs as f64)
    .fetch_all(&state.db)
    .await?;

    // Rows arrive newest first, so the first one seen for a group is the sign-in that
    // minted the token still in use — that is the one the row is built from. The others
    // only contribute their activity: a device is "last active" at the latest moment any
    // of its live tokens was used, not whenever the newest one happened to be created.
    let mut order: Vec<String> = Vec::new();
    let mut groups: std::collections::HashMap<String, (&Row, chrono::DateTime<chrono::Utc>, bool)> =
        std::collections::HashMap::new();
    // 折叠规则在 `fold_device_keys` 里，撤销走的是同一段代码 —— 见那里的注释。
    let keys = fold_device_keys(
        rows.iter()
            .map(|r| (r.6.as_str(), r.1.as_str(), r.2.as_str(), r.3.as_str())),
    );
    for (row, key) in rows.iter().zip(keys) {
        let is_current = Some(row.0) == current;
        match groups.get_mut(&key) {
            Some(entry) => {
                if row.5 > entry.1 {
                    entry.1 = row.5;
                }
                // The device you are reading this from stays marked as such even when the
                // row shown is one of its older sign-ins, and revoking targets the id that
                // is actually current so the warning and the effect agree.
                if is_current {
                    entry.0 = row;
                    entry.2 = true;
                }
            }
            None => {
                order.push(key.clone());
                groups.insert(key, (row, row.5, is_current));
            }
        }
    }

    let sessions: Vec<serde_json::Value> = order
        .iter()
        .filter_map(|key| groups.get(key))
        .map(|(r, last_seen, is_current)| {
            json!({
                "id": r.0,
                "kind": r.1,
                // Still sent for any older client, but the console composes its own from
                // the two parts below: a finished English string cannot be translated,
                // which is why "Desktop app on macOS" stayed English in every language.
                "label": label_for(&r.1, &r.2),
                // Proper nouns, deliberately not translated — "Chrome" and "macOS" are
                // the same words everywhere. Only the connector and the fallback nouns
                // are the client's to localise.
                "browser": browser_of(&r.2),
                "platform": platform_of(&r.2),
                "ip": r.3,
                "created_at": r.4,
                "last_seen_at": last_seen,
                // Lets the page mark the row you are reading it from, and warn before
                // you sign yourself out of the page you are standing on.
                "current": is_current,
            })
        })
        .collect();

    Ok(Json(json!({
        "sessions": sessions,
        // Tokens minted before sessions existed carry no sid, so they are not in the
        // list and cannot be revoked one at a time. The page says so rather than
        // implying the list is the whole truth.
        "current_tracked": current.is_some(),
    })))
}

/// `DELETE /api/sessions/:id` — sign one device out.
///
/// Every live token belonging to that device goes, not just the one named. The row stands
/// for a device, so revoking one of three sign-ins from the same laptop and leaving the
/// other two working would be the button lying about what it did.
///
/// Takes effect on that device's very next request: the Claims extractor checks
/// `revoked_at` on every authenticated call.
pub async fn revoke(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    // Scoped to the caller. Someone else's session id matches no row, so it gets the same
    // "not found" as an id that never existed — which is also what stops this being a
    // probe for whether a given session exists.
    //
    // The whole live set is read rather than the one row, because which rows belong to
    // this device is decided by the folding: half of that answer comes from parsing a
    // User-Agent, and the other half from an alias table that only exists once the set is
    // in memory. Expressing it in SQL would mean a second, hand-maintained copy of the
    // rule that could disagree with the list.
    //
    // 「两边不许分歧」不是靠都记得写一样的逻辑，而是靠**调同一个函数**：分组交给
    // `same_device`，这里不再自己比 device_group。上一版注释就写着不能分歧，可它自己漏了
    // 别名折叠 —— 页面把那条 device_id 为空的老会话并进了当前设备，撤销却按裸 device_group
    // 比，撤不到它。用户点完「注销此设备」，设备从列表上消失了，那条老会话的令牌还能再用
    // 到 30 天期满。2026-08-22 线上有 5 个账号正处在这个状态。
    //
    // 这里读到的比列表页多一点：列表按令牌寿命过滤掉了 created_at 太老的行，这里不过滤，
    // 所以同一台设备上「已过期但没标撤销」的老行也会一起盖上 revoked_at。那些令牌本来就
    // 已经不能用了，多撤一条只是把状态弄干净；漏撤一条还能用的才是事故。
    //
    // **结果集的另一处变化，比上面那条重要得多：折叠现在决定的是「撤销谁」，不再只是
    // 「画成几行」，于是指纹回落的假阳性从显示层跟进了撤销层。** `device_group` 的回落
    // 分支自己就写着它不准（见 sessions.rs 上面那段：同一个办公室 NAT 出口 IP、同一款被
    // 抹掉版本号的浏览器，两台笔记本长得一模一样）。从前这只是页面上多并了一行；现在从
    // 笔记本 B（device_id='B'）点「注销此设备」，会把笔记本 A 那条 2026-08-14 之前的
    // 老行（device_id=''、同一个出口 IP、同一款浏览器）一起盖上 revoked_at —— 用户没碰过
    // A，A 却被登出了。
    //
    // 明知有假阳还是这么做，三条理由，按份量排：
    //   1. 页面本来就把这两行画成一台设备。撤销集合必须恰好等于页面合并集合，否则就是
    //      刚修掉的那个 bug 换个方向再来一遍（按钮说注销了一台设备，实际留了活令牌）。
    //      要收窄只能去收窄 `fold_device_keys` 本身，而那会把「Chrome 自己升级了一版就
    //      多出一行」的重复行原样放回来 —— 那正是这套折叠的来由。
    //   2. 两个失败方向不对等：多撤一条 = 用户在自己另一台设备上重新登录一次；少撤一条 =
    //      用户明确想杀掉的令牌还能活满 30 天。安全侧在多撤这一边。
    //   3. 会被误伤的只可能是 device_id 为空的行 —— 带 id 的行永远按 id 分组（fold_device_keys
    //      对它们一律用 `id\0{device_id}` 做键），两台各自带 id 的机器不可能被并到一起。
    //
    //      空 id 有**两个**来源，而且第二个是长期的：
    //        · 2026-08-14 之前的登录（那时还没有这一列）。JWT 30 天，这批 2026-09-13
    //          前后自然过期，之后它们即使被指纹并进来、被盖上 revoked_at，背后也已经没有
    //          能用的令牌了。
    //        · 桌面端→浏览器的登录交接：handoff.rs 的 redeem 调 start_session 时
    //          device_id 传的就是 None（浏览器那个 localStorage 里的 id 送不过来），
    //          而且它**指望**被指纹折叠进同一台设备 —— handoff.rs 自己的注释就是这么写的。
    //          这条路只要还在，空 id 的行就会一直产生。
    //
    //      所以令牌过期买到的不是一个截止日期，而是更小的爆炸半径：要被误伤，得同一个账号、
    //      同一个出口 IP、同一款粗化后的浏览器，并且其中一行是从 /gate 进来的。这道决定
    //      真正立在 (1) 和 (2) 上，(3) 只是说明「谁可能被误伤」，不是说它会自己消失。
    // Ordered like the list, so the bound takes the same rows the page showed rather than
    // an arbitrary 500 — otherwise a row could be visible and still not be revocable, and
    // the alias folding needs the same newest-first order the list folds in.
    let rows: Vec<LiveRow> = sqlx::query_as(
        "SELECT id, kind, user_agent, ip, device_id FROM sessions \
         WHERE user_id = $1 AND revoked_at IS NULL \
         ORDER BY created_at DESC LIMIT 500",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    let doomed = same_device(&rows, id).ok_or_else(|| AppError::bad("该登录不存在或已失效"))?;

    let done = sqlx::query(
        "UPDATE sessions SET revoked_at = now() \
         WHERE id = ANY($1) AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(&doomed)
    .bind(uid)
    .execute(&state.db)
    .await?;

    if done.rows_affected() == 0 {
        return Err(AppError::bad("该登录不存在或已失效"));
    }

    crate::realtime::record_event(
        &state,
        Some(uid),
        "session_revoked",
        json!({ "email": claims.email, "session": id }),
    )
    .await;

    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::device_kind;

    const CHROME_MAC: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36";
    const SAFARI_IPHONE: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
    const EDGE_WIN: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0";

    #[test]
    fn a_browser_is_named_by_its_most_specific_claim() {
        // Every one of these also claims Safari, and Edge also claims Chrome.
        assert_eq!(browser_of(EDGE_WIN), Some("Edge"));
        assert_eq!(browser_of(CHROME_MAC), Some("Chrome"));
        assert_eq!(browser_of(SAFARI_IPHONE), Some("Safari"));
        assert_eq!(browser_of("something unrecognised"), None);
    }

    #[test]
    fn an_ipad_is_not_reported_as_a_mac() {
        // iPadOS puts "Macintosh" in its User-Agent, so order decides this one.
        let ipad = "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X) AppleWebKit/605.1.15";
        assert_eq!(platform_of(ipad), Some("iPad"));
        assert_eq!(platform_of(CHROME_MAC), Some("macOS"));
        assert_eq!(platform_of(EDGE_WIN), Some("Windows"));
    }

    #[test]
    fn labels_read_the_way_a_person_would_say_them() {
        assert_eq!(label_for("web", CHROME_MAC), "Chrome on macOS");
        assert_eq!(label_for("desktop", CHROME_MAC), "Desktop app on macOS");
        assert_eq!(label_for("mobile", SAFARI_IPHONE), "Safari on iPhone");
    }

    #[test]
    fn an_unreadable_user_agent_still_produces_a_name() {
        assert_eq!(label_for("web", ""), "Web");
        assert_eq!(label_for("desktop", ""), "Desktop app");
        assert_eq!(label_for("mobile", ""), "Mobile app");
    }

    #[test]
    fn the_clients_hint_beats_sniffing() {
        // A Tauri window reports the system webview's User-Agent, so without the hint
        // every desktop sign-in would be filed as a browser.
        assert_eq!(device_kind(Some("desktop"), CHROME_MAC), "desktop");
        assert_eq!(device_kind(Some("mobile"), CHROME_MAC), "mobile");
        assert_eq!(device_kind(Some(" DESKTOP "), CHROME_MAC), "desktop");
    }

    #[test]
    fn a_device_id_groups_rows_no_matter_what_else_changed() {
        // The point of the id: a laptop that moved networks, or reconnected on a new
        // Chrome version, is still one laptop.
        let a = device_group("dev-1", "web", CHROME_MAC, "1.1.1.1");
        let b = device_group("dev-1", "web", EDGE_WIN, "9.9.9.9");
        assert_eq!(a, b);
        assert_ne!(a, device_group("dev-2", "web", CHROME_MAC, "1.1.1.1"));
    }

    #[test]
    fn rows_without_an_id_fall_back_to_the_shape_of_the_request() {
        // The repeated "Chrome · macOS" rows at one IP that started all this: same
        // browser, same address, several sign-ins, and nothing but this to tie them
        // together.
        let first = device_group("", "web", CHROME_MAC, "165.254.118.214");
        let again = device_group("", "web", CHROME_MAC, "165.254.118.214");
        assert_eq!(first, again);
        // A different address, browser or kind is a different row — the fallback never
        // merges beyond what it can actually observe.
        assert_ne!(first, device_group("", "web", CHROME_MAC, "10.0.0.1"));
        assert_ne!(first, device_group("", "web", EDGE_WIN, "165.254.118.214"));
        assert_ne!(first, device_group("", "desktop", CHROME_MAC, "165.254.118.214"));
    }

    #[test]
    fn a_browser_that_updated_itself_is_still_one_device() {
        // Live data from the account that reported this: four sessions at one address,
        // two User-Agents, because Chrome updated between sign-ins. Grouping on the raw
        // string would leave two rows on screen that print identical text — which is the
        // complaint, not the fix.
        let older = CHROME_MAC.replace("140.0.0.0", "139.0.0.0");
        assert_eq!(
            device_group("", "web", CHROME_MAC, "165.254.118.214"),
            device_group("", "web", &older, "165.254.118.214")
        );
    }

    #[test]
    fn two_rows_that_read_the_same_group_together() {
        // The rule the fallback actually follows: rows group exactly when the page would
        // render them the same way. Anything else puts a visible contradiction on screen.
        let rows = [
            ("web", CHROME_MAC, "1.1.1.1"),
            ("web", &CHROME_MAC.replace("140.0.0.0", "138.0.1.2")[..], "1.1.1.1"),
            ("web", EDGE_WIN, "1.1.1.1"),
        ];
        for (kind, ua, ip) in rows {
            for (kind2, ua2, ip2) in rows {
                let same_text = label_for(kind, ua) == label_for(kind2, ua2) && ip == ip2;
                let same_group =
                    device_group("", kind, ua, ip) == device_group("", kind2, ua2, ip2);
                assert_eq!(same_text, same_group, "{ua} vs {ua2}");
            }
        }
    }

    #[test]
    fn an_id_and_a_user_agent_can_never_collide() {
        // Both branches share one key space, so the prefixes have to keep them apart:
        // without them a device_id could be chosen to match some other row's fallback key
        // and quietly join that group.
        assert_ne!(
            device_group("x", "web", "", ""),
            device_group("", "web", "x", "")
        );
    }

    #[test]
    fn without_a_hint_the_user_agent_decides_and_defaults_to_web() {
        assert_eq!(device_kind(None, CHROME_MAC), "web");
        assert_eq!(device_kind(None, SAFARI_IPHONE), "mobile");
        assert_eq!(device_kind(None, "MrDayOne/1.2 Tauri"), "desktop");
        // An unrecognised hint must not be honoured as a kind of its own.
        assert_eq!(device_kind(Some("toaster"), CHROME_MAC), "web");
        assert_eq!(device_kind(None, ""), "web");
    }
}

#[cfg(test)]
mod liveness_tests {
    /// 剥掉注释再断言。注释里会引用被修掉的旧写法（上面那段常量注释就写着
    /// 「user_from_jwt 那份当初根本没写这条判定」），连注释一起扫的话断言会被注释喂到。
    ///
    /// **两边都要剥。** 上一版只剥了 realtime 那一侧、auth 那一侧喂的是原文；碰巧无害
    /// （SQL 字面量里出现不了 `//` 开头的行），但那是巧合不是保证，而这条测试的全部价值
    /// 就在于两边输入被同样处理 —— 处理方式一旦不对称，比对的就不是同一种东西了。
    fn code_of(src: &str) -> String {
        src.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn normalise(sql: &str) -> String {
        sql.split_whitespace()
            .filter(|t| *t != "\\")
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// 把一段源码里那条会话存活判定抠出来并归一化空白。
    fn liveness_sql(src: &str) -> String {
        let start = src
            .find("SELECT ($2::uuid IS NULL OR EXISTS")
            .expect("这里必须有那条会话存活判定");
        let rest = &src[start..];
        let tail = "FROM users WHERE id = $1";
        let end = rest.find(tail).expect("判定必须落在这个账号自己的行上") + tail.len();
        normalise(&rest[..end])
    }

    /// 存活判定只能有一种文本，auth.rs 里那两份手抄件必须和它逐字一致。
    ///
    /// 钉的是「连接」，不是「实现」：SQL 想怎么改都行，但四个入口要一起改。它们各自漂走
    /// 的代价这个仓库付过两次 —— `user_from_jwt` 缺这条判定，用户点完「注销该设备」之后
    /// `/api/me` 老实回 401、同一张令牌却还在烧额度；`/ws` 缺这条判定，被注销的管理员
    /// socket 把全站邮箱和订单/佣金金额推满 30 天。
    ///
    /// **这条红了，多半要改的是 auth.rs，不是这个文件。** 断言消息会说清是哪一份对不上。
    #[test]
    fn the_liveness_predicate_has_one_text_and_auth_rs_copies_still_match_it() {
        let canon = normalise(super::SESSION_LIVE_SQL);
        let auth = code_of(include_str!("auth.rs"));

        for name in ["pub async fn user_from_jwt", "pub async fn claims_from_jwt"] {
            let f = auth.split(name).nth(1).unwrap_or_else(|| panic!("{name} 不见了"));
            assert_eq!(
                liveness_sql(f),
                canon,
                "auth.rs 的 `{name}` 里那份手抄的存活判定和 \
                 crate::sessions::SESSION_LIVE_SQL 不一样了。要改判定，两处一起改；\
                 更好的做法是把那段字面量换成 crate::sessions::session_is_live(db, uid, sid)，\
                 这份副本就不用再手工对齐了",
            );
        }

        // realtime 已经收编：它不许再持有自己的副本（realtime.rs 里另有一条守着）。
        let rt = code_of(include_str!("realtime.rs"));
        let rt = &rt[..rt.find("#[cfg(test)]").unwrap_or(rt.len())];
        assert!(
            !rt.contains("SELECT ($2::uuid IS NULL OR EXISTS"),
            "realtime.rs 又抄回了一份存活判定 —— 改调 crate::sessions::session_is_live()",
        );

        // `Claims` 提取器那一份不收编（拆出来等于每个已认证请求多一次往返，理由见常量
        // 上面的注释），所以它只能钉形状：同一条规则嵌在那条顺带读 role 的语句里，
        // 三个要件一个都不能少。这是整条认证链上跑得最频繁的一处判定，也是唯一没人
        // 逐字钉过的一处。
        let extractor = auth
            .split("impl FromRequestParts<AppState> for Claims")
            .nth(1)
            .expect("Claims 提取器");
        let extractor =
            &extractor[..extractor.find("pub(crate) fn device_kind").unwrap_or(extractor.len())];
        let inline = normalise(extractor);
        for (needle, why) in [
            (
                "WHERE id = $2 AND user_id = $1 AND revoked_at IS NULL",
                "提取器不再查 revoked_at / 不再限定本账号：被注销的令牌又能过每一个已认证接口了",
            ),
            (
                "($2::uuid IS NULL OR EXISTS (SELECT 1 FROM live))",
                "sessions 表出现之前签发的老令牌没有 sid，少了这一支会把这批人全部登出",
            ),
        ] {
            assert!(inline.contains(needle), "{why}");
        }
    }
}

#[cfg(test)]
mod grouping_tests {
    use super::*;

    /// device_id 出现之前的老会话，必须能并进同一台设备，而不是自成一行。
    ///
    /// 这是用户实际看到的样子：同一台 Mac 的 Chrome，同一个 IP，设备列表里出现两行，
    /// 其中一行标着「当前设备」，另一行的「最近活跃」永远停在几天前 —— 因为两把分组键
    /// 不一样（一个 id、一个指纹），而它们其实是同一个浏览器。
    #[test]
    fn a_legacy_session_folds_into_the_device_it_belongs_to() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
                  (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
        let with_id = device_group("f1b54f1f-cafe", "web", ua, "165.254.118.214");
        let legacy = device_group("", "web", ua, "165.254.118.214");
        assert_ne!(
            with_id, legacy,
            "两把键本来就不同，这正是重复行的来源；list() 靠别名表把它们并起来",
        );
        // 指纹这一半必须只由「类型+浏览器+平台+IP」决定，别名表才对得上。
        assert_eq!(
            legacy,
            device_group("", "web", ua, "165.254.118.214"),
            "同样的浏览器和 IP 必须算出同一个指纹",
        );
    }

    /// 不同 IP 的老会话不能被并进来 —— 那是另一个地方登录的。
    #[test]
    fn a_different_ip_stays_its_own_device() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Chrome/141.0.0.0";
        assert_ne!(
            device_group("", "web", ua, "165.254.118.214"),
            device_group("", "web", ua, "8.8.8.8"),
        );
    }

    /// 桌面端和浏览器即便同机同 IP，也必须分开：它们是两个可以各自撤销的登录。
    #[test]
    fn desktop_and_browser_are_never_merged() {
        let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Chrome/141.0.0.0";
        assert_ne!(
            device_group("", "web", ua, "1.2.3.4"),
            device_group("", "desktop", ua, "1.2.3.4"),
        );
    }

    const CHROME: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
                          (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

    fn row(id: uuid::Uuid, ip: &str, device_id: &str) -> LiveRow {
        (id, "web".to_owned(), CHROME.to_owned(), ip.to_owned(), device_id.to_owned())
    }

    /// 「注销此设备」必须撤掉页面并进这台设备的每一行，包括那条 device_id 为空的老会话。
    ///
    /// 事故长这样：list 用别名表把老会话并进了带 id 的那台设备（页面上一台设备一行），
    /// revoke 却只按裸 device_group 比 —— 老会话的键是指纹，永远不等于 `id\0D`，于是撤不到。
    /// 用户点完注销，设备从列表上消失了，那条老会话的令牌还能再用到 30 天期满。
    /// 2026-08-22 线上有 5 个账号正处在这个状态。这里钉住的是：**撤销的集合恰好等于
    /// 页面合并的集合**。
    #[test]
    fn signing_out_a_device_also_revokes_the_legacy_session_folded_into_it() {
        let newer = uuid::Uuid::from_u128(1);
        let legacy = uuid::Uuid::from_u128(2);
        let elsewhere = uuid::Uuid::from_u128(3);
        // 倒序，和两个处理器读表的顺序一致：带 id 的那条较新，排在前面。
        let rows = vec![
            row(newer, "1.2.3.4", "D"),
            row(legacy, "1.2.3.4", ""),
            row(elsewhere, "8.8.8.8", ""),
        ];

        // 页面把前两行画成一台设备 —— 撤销必须照着这个答案走。
        let keys = fold_device_keys(
            rows.iter()
                .map(|r| (r.4.as_str(), r.1.as_str(), r.2.as_str(), r.3.as_str())),
        );
        assert_eq!(keys[0], keys[1], "老会话必须折进带 id 的那台设备");
        assert_ne!(keys[0], keys[2], "另一个 IP 是另一台设备");

        let both = {
            let mut v = vec![newer, legacy];
            v.sort();
            v
        };
        let mut from_new = same_device(&rows, newer).expect("target row");
        from_new.sort();
        assert_eq!(from_new, both, "从当前这条注销，被并进来的老会话必须一起撤");

        // 反向同样成立：用户点的可能正是那条老会话的行。
        let mut from_legacy = same_device(&rows, legacy).expect("target row");
        from_legacy.sort();
        assert_eq!(from_legacy, both, "从老会话那一行注销，带 id 的那条也必须撤");

        // 别处的登录不能被牵连 —— 撤多了等于把用户从他没动过的设备上踢下线。
        assert_eq!(same_device(&rows, elsewhere).unwrap(), vec![elsewhere]);
        // 不在活会话里的 id 什么也不撤（调用方据此回「不存在或已失效」）。
        assert!(same_device(&rows, uuid::Uuid::from_u128(9)).is_none());
    }

    /// 桌面端和浏览器同 IP 时，指纹不同（kind 不同），不能被别名表并到一起。
    #[test]
    fn the_alias_never_folds_across_kinds() {
        let web = uuid::Uuid::from_u128(11);
        let desktop = uuid::Uuid::from_u128(12);
        let rows = vec![
            row(web, "1.2.3.4", "D"),
            (desktop, "desktop".to_owned(), CHROME.to_owned(), "1.2.3.4".to_owned(), String::new()),
        ];
        assert_eq!(same_device(&rows, web).unwrap(), vec![web]);
        assert_eq!(same_device(&rows, desktop).unwrap(), vec![desktop]);
    }

    /// 指纹回落的假阳性会跟着一起撤 —— 这是权衡后**故意**接受的，钉在这里免得被「顺手收窄」。
    ///
    /// 场景：办公室一个 NAT 出口，两台 Mac。B 是 2026-08-14 之后登录的，有 device_id；
    /// A 是那之前登录的老会话，device_id 为空，而且它那次登录时 Chrome 还是旧版本。
    /// 指纹把版本号抹平（`device_group` 只取「浏览器·平台」，不取版本），出口 IP 又一样，
    /// 于是这两行在折叠眼里是同一台机器 —— 从 B 点「注销此设备」把 A 也登出了。
    ///
    /// 之所以不收窄：页面已经把这两行画成一台设备，撤销集合必须等于页面合并集合；要收窄
    /// 只能改 `device_group` 的指纹（比如把原始 User-Agent 加回去），而那会让「Chrome 自己
    /// 升了一版就多出一行」的重复行回来。失败方向也不对等：多撤 = 重新登录一次，少撤 =
    /// 用户想杀掉的令牌活满 30 天。谁要改这里，先改 revoke 上面那段注释里的三条理由。
    #[test]
    fn a_fingerprint_false_positive_is_signed_out_along_with_the_device() {
        let mine = uuid::Uuid::from_u128(21);
        let neighbour = uuid::Uuid::from_u128(22);
        let older_chrome = CHROME.replace("141.0.0.0", "138.0.1.2");
        // 倒序：带 id 的那条较新，排在前面，先把指纹登记进别名表。
        let rows = vec![
            row(mine, "165.254.118.214", "D"),
            (
                neighbour,
                "web".to_owned(),
                older_chrome,
                "165.254.118.214".to_owned(),
                String::new(),
            ),
        ];
        let mut swept = same_device(&rows, mine).expect("target row");
        swept.sort();
        assert_eq!(
            swept,
            vec![mine, neighbour],
            "同 IP、同「浏览器·平台」的老会话必须跟着一起撤 —— 哪怕它其实是隔壁那台机器。\
             这条红了说明有人收窄了指纹：页面会重新出现重复行，而 revoke 上面那段注释\
             （第 1、3 条理由）就成了假话",
        );
    }

    /// revoke 不许自己算分组键。
    ///
    /// 行为测试拦不住这一条：把老逻辑抄回 revoke 里，`same_device` 和它的测试照样是绿的。
    /// 分歧的根源就是「同一个问题有两份实现」，所以这里钉的是「只有一份」。
    #[test]
    fn revoke_delegates_grouping_to_the_same_folding_the_list_uses() {
        let src = include_str!("sessions.rs");
        let src = &src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())];
        let body = src.split("pub async fn revoke(").nth(1).expect("revoke");
        // 注释里会引用被修掉的旧写法，连注释一起扫断言就永远为真。
        let body: String = body
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            body.contains("same_device(&rows, id)"),
            "撤销的分组必须走 list 用的那份折叠",
        );
        assert!(
            !body.contains("device_group("),
            "revoke 自己比 device_group 就漏掉了别名折叠 —— 那条老会话撤不掉",
        );
    }
}
