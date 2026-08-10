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
/// The client's own id when it sent one. Otherwise the shape of the request it arrived
/// on, which is the best available guess for anything that signed in before device ids
/// existed. That fallback is imperfect in both directions — two laptops behind one office
/// NAT running the same browser version look identical, and a phone that changes networks
/// looks like two devices — but it only ever applies to rows old enough to predate the
/// id, and it is much closer to the truth than listing one laptop once per sign-in.
///
/// NUL separates the parts so a User-Agent containing the separator cannot be crafted to
/// collide with a different (kind, ip) pair.
fn device_group(device_id: &str, kind: &str, user_agent: &str, ip: &str) -> String {
    if device_id.is_empty() {
        format!("ua\u{0}{kind}\u{0}{user_agent}\u{0}{ip}")
    } else {
        format!("id\u{0}{device_id}")
    }
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
    for row in &rows {
        let key = device_group(&row.6, &row.1, &row.2, &row.3);
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

    // Scoped to the caller. Someone else's session id matches no row and gets the same
    // "not found" as an id that never existed, which is also what stops this being a
    // probe for whether a given session exists.
    let target: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT device_id, kind, user_agent, ip FROM sessions \
         WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(id)
    .bind(uid)
    .fetch_optional(&state.db)
    .await?;
    let Some((device_id, kind, user_agent, ip)) = target else {
        return Err(AppError::bad("该登录不存在或已失效"));
    };

    // Same two cases as the list groups on, so what disappears from the page is exactly
    // what stopped working.
    let done = if device_id.is_empty() {
        sqlx::query(
            "UPDATE sessions SET revoked_at = now() \
             WHERE user_id = $1 AND revoked_at IS NULL AND device_id = '' \
               AND kind = $2 AND user_agent = $3 AND ip = $4",
        )
        .bind(uid)
        .bind(&kind)
        .bind(&user_agent)
        .bind(&ip)
        .execute(&state.db)
        .await?
    } else {
        sqlx::query(
            "UPDATE sessions SET revoked_at = now() \
             WHERE user_id = $1 AND revoked_at IS NULL AND device_id = $2",
        )
        .bind(uid)
        .bind(&device_id)
        .execute(&state.db)
        .await?
    };

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
        // The three "Chrome · macOS" rows at one IP that started all this: same browser,
        // same address, three sign-ins, and nothing but this to tie them together.
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
