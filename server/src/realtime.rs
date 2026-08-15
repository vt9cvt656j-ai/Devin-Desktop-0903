use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::StreamExt;
use serde_json::json;

use crate::error::{ApiResult, AppError};
use crate::AppState;

const FEED_CHANNEL: &str = "events:feed";

/// Persist an event and publish it to the live feed. Best-effort: a telemetry
/// failure must never break the request that triggered it.
pub async fn record_event(
    state: &AppState,
    user_id: Option<uuid::Uuid>,
    kind: &str,
    data: serde_json::Value,
) {
    let res: ApiResult<()> = async {
        sqlx::query("INSERT INTO events (user_id, kind, data) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(kind)
            .bind(&data)
            .execute(&state.db)
            .await?;
        let payload = json!({
            "kind": kind,
            "user_id": user_id,
            "data": data,
            "ts": chrono::Utc::now().to_rfc3339(),
        })
        .to_string();
        let mut conn = state.redis.clone();
        let _: () = redis::cmd("PUBLISH")
            .arg(FEED_CHANNEL)
            .arg(payload)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }
    .await;
    if let Err(e) = res {
        tracing::warn!("record_event({kind}) failed: {}", e.msg);
    }
}

/// Presence is one short-lived key PER CONNECTION, refreshed while the socket is open.
///
/// The previous single `online:count` INCRBY/DECRBY counter drifted upward forever: a
/// container restart, a panic, or a killed task skipped the decrement, and the key had no
/// TTL to heal it — so "online" only ever grew and became meaningless. Per-connection keys
/// expire on their own when a process dies.
const PRESENCE_PREFIX: &str = "ws:online:";
const PRESENCE_TTL_SECS: i64 = 45;
const PRESENCE_REFRESH: std::time::Duration = std::time::Duration::from_secs(15);

async fn touch_presence(state: &AppState, key: &str) {
    let mut conn = state.redis.clone();
    let _: Result<(), _> = redis::cmd("SET")
        .arg(key)
        .arg(1)
        .arg("EX")
        .arg(PRESENCE_TTL_SECS)
        .query_async(&mut conn)
        .await;
}

async fn drop_presence(state: &AppState, key: &str) {
    let mut conn = state.redis.clone();
    let _: Result<(), _> = redis::cmd("DEL").arg(key).query_async(&mut conn).await;
}

/// Count live presence keys. SCAN, not KEYS — KEYS blocks Redis for the whole sweep.
async fn online_count(state: &AppState) -> i64 {
    let mut conn = state.redis.clone();
    let pattern = format!("{PRESENCE_PREFIX}*");
    let mut cursor: u64 = 0;
    let mut n: i64 = 0;
    loop {
        let res: Result<(u64, Vec<String>), _> = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(500)
            .query_async(&mut conn)
            .await;
        match res {
            Ok((next, keys)) => {
                n += keys.len() as i64;
                cursor = next;
                if cursor == 0 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    n
}

/// GET /ws — upgrade to a WebSocket that streams the live event feed (fanned out
/// across all backend instances via Redis pub/sub) and tracks online presence.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// How long a freshly upgraded socket may stay silent before we hang up on it.
/// The feed carries every user's email plus order/grant/commission amounts, so an
/// unauthenticated socket must never reach `stream_feed`.
const WS_AUTH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// How often a still-open feed socket re-checks that its owner is still an admin.
/// Without this, a socket opened while the user WAS an admin keeps streaming for as
/// long as it stays connected, which for this feed can be days.
const WS_ROLE_RECHECK: std::time::Duration = std::time::Duration::from_secs(300);

/// Is this user an admin *right now*, according to the users table?
///
/// The token is trusted for identity, never for privilege — the same rule the `Claims`
/// extractor enforces. `claims_from_jwt` only decodes, so checking `claims.role` here
/// would have left `/ws` as the one place where a 30-day token still carried privilege:
/// a demoted or deleted admin would keep streaming every user's email, order amounts
/// and commissions until the token expired.
async fn is_admin_now(state: &AppState, uid: uuid::Uuid) -> bool {
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();
    role.as_deref() == Some("admin")
}

/// Authenticate a WebSocket from its first frame: `{"type":"auth","token":"<jwt>"}`.
///
/// The browser cannot set an Authorization header on a WebSocket, and putting the
/// token in the query string would write it into nginx's access log, so the token
/// travels in the first message instead. Returns the admin's user id on success.
async fn ws_authenticate(socket: &mut WebSocket, state: &AppState) -> Option<uuid::Uuid> {
    let first = match tokio::time::timeout(WS_AUTH_TIMEOUT, socket.recv()).await {
        Ok(Some(Ok(Message::Text(text)))) => text,
        _ => return None,
    };
    let frame = serde_json::from_str::<serde_json::Value>(&first).ok()?;
    if frame.get("type").and_then(|v| v.as_str()) != Some("auth") {
        return None;
    }
    let token = frame.get("token").and_then(|v| v.as_str())?;
    let claims = crate::auth::claims_from_jwt(&state.db, &state.cfg, token).await?;
    let uid = uuid::Uuid::parse_str(&claims.sub).ok()?;
    is_admin_now(state, uid).await.then_some(uid)
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let Some(uid) = ws_authenticate(&mut socket, &state).await else {
        let _ = socket
            .send(Message::Text(
                json!({ "type": "auth_error", "error": "需要管理员权限" }).to_string(),
            ))
            .await;
        let _ = socket.close().await;
        return;
    };
    let _ = socket
        .send(Message::Text(json!({ "type": "auth_ok" }).to_string()))
        .await;
    let presence = format!("{}{}", PRESENCE_PREFIX, uuid::Uuid::new_v4());
    touch_presence(&state, &presence).await;
    let result = stream_feed(&mut socket, &state, uid, &presence).await;
    if let Err(e) = result {
        tracing::debug!("ws closed: {}", e.msg);
    }
    drop_presence(&state, &presence).await;
}

async fn stream_feed(
    socket: &mut WebSocket,
    state: &AppState,
    uid: uuid::Uuid,
    presence: &str,
) -> ApiResult<()> {
    let mut pubsub = state.redis_client.get_async_pubsub().await?;
    pubsub.subscribe(FEED_CHANNEL).await?;
    let mut messages = pubsub.on_message();
    let mut recheck = tokio::time::interval(WS_ROLE_RECHECK);
    recheck.tick().await; // the first tick fires immediately; auth just succeeded
    let mut heartbeat = tokio::time::interval(PRESENCE_REFRESH);
    heartbeat.tick().await; // ditto — touch_presence already ran
    loop {
        tokio::select! {
            _ = heartbeat.tick() => { touch_presence(state, presence).await; }
            _ = recheck.tick() => {
                if !is_admin_now(state, uid).await {
                    let _ = socket.send(Message::Text(
                        json!({ "type": "auth_error", "error": "权限已变更" }).to_string(),
                    )).await;
                    break;
                }
            }
            maybe = messages.next() => {
                match maybe {
                    Some(msg) => {
                        let payload: String = msg.get_payload().unwrap_or_default();
                        if socket.send(Message::Text(payload)).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Ping(p))) => { let _ = socket.send(Message::Pong(p)).await; }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
    Ok(())
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct Event {
    pub id: i64,
    pub user_id: Option<uuid::Uuid>,
    pub kind: String,
    pub data: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/events — recent activity for the dashboard's initial load
/// (the live tail then arrives over /ws). Admin only.
pub async fn recent_events(
    State(state): State<AppState>,
    claims: crate::auth::Claims,
) -> ApiResult<Json<Vec<Event>>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    let rows = sqlx::query_as::<_, Event>(
        "SELECT id, user_id, kind, data, created_at FROM events ORDER BY id DESC LIMIT 50",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

/// GET /api/admin/stats — headline numbers for the dashboard (admin only).
pub async fn stats(
    State(state): State<AppState>,
    claims: crate::auth::Claims,
) -> ApiResult<Json<serde_json::Value>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    let today: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM users WHERE created_at >= date_trunc('day', now())",
    )
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({
        "total_users": total,
        "today_users": today,
        "online": online_count(&state).await,
    })))
}
