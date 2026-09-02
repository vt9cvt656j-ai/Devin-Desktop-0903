//! Whether a desktop app is currently signed in, asked of the server rather than of the
//! computer.
//!
//! The original design had the browser open `http://127.0.0.1:47821` and ask the app
//! directly. That cannot be made to work reliably: a page served over HTTPS reaching a
//! plaintext loopback port is exactly the shape browsers have spent years closing down.
//! Observed on Chrome 150 with the app running, listening, and answering the preflight
//! correctly — including `Access-Control-Allow-Private-Network` — and with the
//! local-network-access permission *granted*: the fetch still failed with a bare
//! `TypeError: Failed to fetch`. There is nothing left to fix on the app's side, and a
//! page cannot argue with its browser.
//!
//! So the direction is reversed. The app already talks to this gateway, authenticated,
//! every time it is used. It now also says "still here" on a timer, and the console asks
//! the gateway. No loopback, no browser permission, no localhost at all.
//!
//! Presence lives in Redis with a TTL rather than in Postgres: it is worthless a minute
//! after it is written, and expiry is the whole semantic — a key that is gone *is* an app
//! that stopped. Nothing has to notice a crash or run a sweep.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// Comfortably longer than the client's interval, so one dropped request on a bad network
/// does not blink the badge off. Short enough that quitting the app shows up quickly.
const PRESENCE_TTL_SECS: usize = 90;

/// What the client is asked to send at, in seconds. Published in the status response so
/// the two cannot drift apart silently.
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

fn key(uid: &uuid::Uuid) -> String {
    format!("desktop:{uid}")
}

#[derive(Deserialize)]
pub struct HeartbeatReq {
    /// Shown in the console so it is obvious which build is running.
    pub version: Option<String>,
}

/// `POST /api/desktop/heartbeat` — the app saying it is running and signed in.
///
/// Authenticated with the app's own login token, so it can only ever mark *its own*
/// account present. There is no id in the body and none would be honoured.
pub async fn heartbeat(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<HeartbeatReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    // Version is display-only, but it is written into a value this server later hands
    // back to a browser, so it is bounded and stripped of anything that is not a plain
    // version string.
    let version: String = req
        .version
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        .take(32)
        .collect();

    let payload = json!({ "version": version, "at": chrono::Utc::now().timestamp() }).to_string();

    let mut conn = state.redis.clone();
    let _: Result<(), _> = redis::cmd("SET")
        .arg(key(&uid))
        .arg(payload)
        .arg("EX")
        .arg(PRESENCE_TTL_SECS)
        .query_async(&mut conn)
        .await;

    Ok(Json(json!({ "ok": true, "interval_secs": HEARTBEAT_INTERVAL_SECS })))
}

/// `GET /api/desktop/status` — what the console shows.
///
/// Says "a desktop app is signed in to this account", not "on this computer". Those are
/// different claims and only the first one is knowable from here; the wording in the
/// console matches.
pub async fn status(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let mut conn = state.redis.clone();
    let raw: Option<String> = redis::cmd("GET")
        .arg(key(&uid))
        .query_async(&mut conn)
        .await
        .unwrap_or(None);

    let Some(raw) = raw else {
        // The key expired or was never written: no app has checked in recently.
        return Ok(Json(json!({
            "online": false,
            "version": null,
            "seconds_ago": null,
            "interval_secs": HEARTBEAT_INTERVAL_SECS,
        })));
    };

    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|_| json!({}));
    let at = parsed.get("at").and_then(|v| v.as_i64()).unwrap_or(0);
    let seconds_ago = (chrono::Utc::now().timestamp() - at).max(0);

    Ok(Json(json!({
        "online": true,
        "version": parsed.get("version").and_then(|v| v.as_str()).filter(|s| !s.is_empty()),
        "seconds_ago": seconds_ago,
        "interval_secs": HEARTBEAT_INTERVAL_SECS,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_outlives_a_single_missed_heartbeat() {
        // If the TTL were not comfortably longer than the interval, one dropped request
        // on a flaky network would blink the badge off while the app was running fine.
        assert!(
            PRESENCE_TTL_SECS as u64 > HEARTBEAT_INTERVAL_SECS * 2,
            "TTL {PRESENCE_TTL_SECS}s must survive at least two missed beats at {HEARTBEAT_INTERVAL_SECS}s"
        );
    }

    #[test]
    fn the_key_is_scoped_to_one_account() {
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        assert_ne!(key(&a), key(&b));
        assert!(key(&a).starts_with("desktop:"));
    }
}
