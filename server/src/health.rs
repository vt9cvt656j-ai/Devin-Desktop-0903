//! Whether each configured model route is reachable, and how fast it answers.
//!
//! Built because nothing recorded it. `model_usage` holds tens of thousands of rows of
//! cost and tokens with no latency and no outcome, so "is this provider up" had no answer
//! anywhere in the system — and a status page whose numbers are not measured is worse
//! than no status page.
//!
//! **What this measures, and what it does not.** A probe is one HTTP request to the
//! route's own base URL: it costs nothing, needs no credentials, and tells you the network
//! path and the provider's front door are alive. It is NOT conversation latency. Measuring
//! that honestly means paying for a completion against every model on every cycle, which
//! is a standing bill for a dashboard; if that is ever wanted it belongs behind an
//! explicit setting, not switched on by default.
//!
//! **Reachable is not 200.** A provider answering 401 or 404 to an unauthenticated GET has
//! spoken, so the route is up. Only a refused connection, a TLS failure or a timeout counts
//! as down. Treating a 401 as an outage would show every correctly-secured provider as
//! broken.

use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::Json;
use serde_json::json;

use crate::auth::Claims;
use crate::error::ApiResult;
use crate::AppState;

/// How often every active route is probed. One request per model per cycle; at a handful
/// of models this is negligible traffic, and it is what sets the resolution of the
/// "last 60 samples" strip — an hour of history at this interval.
const PROBE_EVERY: Duration = Duration::from_secs(60);

/// A probe that has not answered by now is down for practical purposes. Deliberately
/// shorter than the interval, so a hung endpoint cannot stack probes on top of each other.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Samples older than this are dropped. The longest window the page offers is 30 days.
const KEEP_DAYS: i64 = 31;

/// Above this, a reachable route is reported as degraded rather than healthy.
const SLOW_MS: i64 = 2_000;

/// Start the background prober. Called once at boot; returns immediately.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        // Let the rest of the service finish starting before adding outbound traffic.
        tokio::time::sleep(Duration::from_secs(15)).await;
        let mut tick = tokio::time::interval(PROBE_EVERY);
        loop {
            tick.tick().await;
            if let Err(err) = probe_once(&state).await {
                // A failed cycle is not fatal: the next one is a minute away, and a
                // database blip must not take the prober down for the life of the process.
                tracing::warn!(%err, "model health probe cycle failed");
            }
        }
    });
}

async fn probe_once(state: &AppState) -> anyhow::Result<()> {
    let routes: Vec<(uuid::Uuid, String)> =
        sqlx::query_as("SELECT id, base_url FROM models WHERE active = true")
            .fetch_all(&state.db)
            .await?;

    for (id, base_url) in routes {
        let (ok, latency_ms, status_code, error) = probe(state, &base_url).await;
        // A failed insert for one route must not skip the rest of the cycle.
        if let Err(err) = sqlx::query(
            "INSERT INTO model_health (model_id, ok, latency_ms, status_code, error) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(id)
        .bind(ok)
        .bind(latency_ms)
        .bind(status_code)
        .bind(&error)
        .execute(&state.db)
        .await
        {
            tracing::warn!(%err, %id, "could not record a health sample");
        }
    }

    // Cheap enough to run every cycle, and it keeps the table from growing without bound
    // if the service runs for months.
    let _ = sqlx::query("DELETE FROM model_health WHERE checked_at < now() - make_interval(days => $1)")
        .bind(KEEP_DAYS as i32)
        .execute(&state.db)
        .await;

    Ok(())
}

/// One request to the route's front door. Returns (reachable, ms, status, error).
async fn probe(state: &AppState, base_url: &str) -> (bool, Option<i32>, Option<i32>, String) {
    let url = base_url.trim();
    if url.is_empty() || !url.starts_with("http") {
        return (false, None, None, "route has no usable base URL".to_owned());
    }

    let started = Instant::now();
    let result = state
        .update_http
        .get(url)
        .timeout(PROBE_TIMEOUT)
        // No credentials on purpose: this asks "are you there", not "will you serve me".
        .send()
        .await;
    let elapsed = started.elapsed().as_millis().min(i32::MAX as u128) as i32;

    match result {
        // Any answer at all means the path is alive — see the note at the top of the file.
        Ok(response) => (true, Some(elapsed), Some(response.status().as_u16() as i32), String::new()),
        Err(err) => {
            // Bounded and stripped of the URL: this text is rendered in the console, and
            // the base URL is part of the routing configuration, not something to leak
            // into a page any signed-in user can open.
            let reason = if err.is_timeout() {
                "timed out".to_owned()
            } else if err.is_connect() {
                "connection refused".to_owned()
            } else {
                "unreachable".to_owned()
            };
            (false, None, None, reason)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct StatusQuery {
    /// Availability window in days. Clamped to the three the page offers.
    days: Option<i64>,
}

/// `GET /api/models/status` — one card's worth of truth per configured route.
///
/// Signed in only. It names every model route the deployment has, which is operational
/// detail rather than public information — and it deliberately returns no base URL and no
/// API key, only what a card shows.
pub async fn status(
    State(state): State<AppState>,
    _claims: Claims,
    Query(q): Query<StatusQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let days = match q.days.unwrap_or(7) {
        d if d <= 7 => 7,
        d if d <= 15 => 15,
        _ => 30,
    };

    type Row = (uuid::Uuid, String, String, Option<String>, bool);
    let routes: Vec<Row> = sqlx::query_as(
        "SELECT id, label, provider, model_id, active FROM models \
         WHERE active = true ORDER BY sort, label",
    )
    .fetch_all(&state.db)
    .await?;

    let mut cards = Vec::with_capacity(routes.len());
    for (id, label, provider, model_id, _active) in routes {
        // Newest first, so the client reverses for a left-to-right "past → now" strip.
        let samples: Vec<(bool, Option<i32>, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
            "SELECT ok, latency_ms, checked_at FROM model_health \
             WHERE model_id = $1 ORDER BY checked_at DESC LIMIT 60",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?;

        let window: Option<(i64, i64)> = sqlx::query_as(
            "SELECT count(*), count(*) FILTER (WHERE ok) FROM model_health \
             WHERE model_id = $1 AND checked_at > now() - make_interval(days => $2)",
        )
        .bind(id)
        .bind(days as i32)
        .fetch_optional(&state.db)
        .await?;

        let (total, up) = window.unwrap_or((0, 0));
        // Null rather than 100%: a route nobody has probed yet has unknown availability,
        // and printing a perfect score for it would be inventing the number.
        let availability = if total > 0 {
            Some((up as f64) * 100.0 / (total as f64))
        } else {
            None
        };

        let latest = samples.first();
        let ping_ms = latest.and_then(|s| s.1);
        let state_word = match latest {
            None => "unknown",
            Some((false, _, _)) => "error",
            Some((true, Some(ms), _)) if (*ms as i64) > SLOW_MS => "degraded",
            Some((true, _, _)) => "ok",
        };

        cards.push(json!({
            "id": id,
            "label": label,
            "provider": provider,
            "model": model_id.unwrap_or_default(),
            "state": state_word,
            "ping_ms": ping_ms,
            "availability": availability,
            "window_days": days,
            "checked_at": latest.map(|s| s.2),
            // Oldest → newest, which is the order the strip is drawn in.
            "samples": samples
                .iter()
                .rev()
                .map(|(ok, ms, _)| json!({ "ok": ok, "ms": ms }))
                .collect::<Vec<_>>(),
        }));
    }

    // The header pill: the worst state on the page, because that is what an operator
    // needs to see without reading every card.
    let overall = if cards.iter().any(|c| c["state"] == "error") {
        "error"
    } else if cards.iter().any(|c| c["state"] == "degraded") {
        "degraded"
    } else if cards.iter().all(|c| c["state"] == "unknown") {
        "unknown"
    } else {
        "ok"
    };

    Ok(Json(json!({
        "overall": overall,
        "window_days": days,
        "probe_every_secs": PROBE_EVERY.as_secs(),
        "models": cards,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The probe must never be turned into "did it return 200".
    #[test]
    fn a_provider_that_answers_at_all_is_reachable() {
        let src = include_str!("health.rs");
        let body = src
            .split("async fn probe(")
            .nth(1)
            .expect("probe must exist");
        let body = &body[..body.find("\n#[derive").unwrap_or(body.len())];
        assert!(
            body.contains("Ok(response) => (true,"),
            "any HTTP answer counts as reachable — a 401 from a secured provider is not an outage"
        );
        assert!(
            !body.contains("is_success()"),
            "success-only probing would report every correctly-secured route as down"
        );
    }

    /// Nothing about the route's credentials or address may reach the client.
    #[test]
    fn the_status_payload_carries_no_secrets() {
        let src = include_str!("health.rs");
        let body = src.split("pub async fn status(").nth(1).expect("status");
        let body = &body[..body.find("\n#[cfg(test)]").unwrap_or(body.len())];
        for leaked in ["api_key", "base_url"] {
            assert!(
                !body.contains(leaked),
                "the status payload must not expose `{leaked}`"
            );
        }
    }

    /// An unprobed route reports unknown, not perfect.
    #[test]
    fn availability_is_null_until_something_has_been_measured() {
        let src = include_str!("health.rs");
        assert!(
            src.contains("if total > 0 {") && src.contains("None\n        };"),
            "availability must be null with no samples rather than defaulting to 100%"
        );
    }
}
