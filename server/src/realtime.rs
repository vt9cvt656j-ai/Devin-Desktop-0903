use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::StreamExt;
use serde_json::json;

use crate::error::{ApiResult, AppError};
use crate::AppState;

const FEED_CHANNEL: &str = "events:feed";
const ONLINE_KEY: &str = "online:count";

/// Persist an event and publish it to the live feed. Best-effort: a telemetry
/// failure must never break the request that triggered it.
pub async fn record_event(state: &AppState, user_id: Option<uuid::Uuid>, kind: &str, data: serde_json::Value) {
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
        let _: () = redis::cmd("PUBLISH").arg(FEED_CHANNEL).arg(payload).query_async(&mut conn).await?;
        Ok(())
    }
    .await;
    if let Err(e) = res {
        tracing::warn!("record_event({kind}) failed: {}", e.msg);
    }
}

async fn bump_online(state: &AppState, delta: i64) {
    let mut conn = state.redis.clone();
    let _: Result<i64, _> = redis::cmd("INCRBY").arg(ONLINE_KEY).arg(delta).query_async(&mut conn).await;
}

/// GET /ws — upgrade to a WebSocket that streams the live event feed (fanned out
/// across all backend instances via Redis pub/sub) and tracks online presence.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    bump_online(&state, 1).await;
    let result = stream_feed(&mut socket, &state).await;
    if let Err(e) = result {
        tracing::debug!("ws closed: {}", e.msg);
    }
    bump_online(&state, -1).await;
}

async fn stream_feed(socket: &mut WebSocket, state: &AppState) -> ApiResult<()> {
    let mut pubsub = state.redis_client.get_async_pubsub().await?;
    pubsub.subscribe(FEED_CHANNEL).await?;
    let mut messages = pubsub.on_message();
    loop {
        tokio::select! {
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

/// GET /api/admin/stats — headline numbers for the dashboard (admin only).
pub async fn stats(State(state): State<AppState>, claims: crate::auth::Claims) -> ApiResult<Json<serde_json::Value>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM users").fetch_one(&state.db).await?;
    let today: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE created_at >= date_trunc('day', now())")
        .fetch_one(&state.db)
        .await?;
    let mut conn = state.redis.clone();
    let online: Option<i64> = redis::cmd("GET").arg(ONLINE_KEY).query_async(&mut conn).await?;
    Ok(Json(json!({
        "total_users": total,
        "today_users": today,
        "online": online.unwrap_or(0).max(0),
    })))
}
