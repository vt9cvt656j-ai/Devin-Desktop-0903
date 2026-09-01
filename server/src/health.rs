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
use axum::http::StatusCode;
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

/// `GET /health` — 容器活性探针 + 一小撮进程内计数器。
///
/// 部署脚本和 Docker healthcheck 只看 HTTP 200（`curl -fsS` / `urllib.urlopen`），
/// 不解析 body，所以从纯文本 "ok" 换成 JSON 不改任何探活语义；`"status":"ok"`
/// 让人肉 curl 的读感也不变。响应缓存三计数器挂在这里：它是唯一免鉴权、每台实例
/// 各自应答的端点，正适合回答「没人命中还是根本没在记」。进程内计数，重启归零。
pub async fn liveness() -> Json<serde_json::Value> {
    let (hit, miss, store) = crate::models::response_cache_counters();
    Json(json!({
        "status": "ok",
        "response_cache": { "hit": hit, "miss": miss, "store": store },
    }))
}

/// `GET /health/reports` — 部署后真的把money报表的查询跑一遍。
///
/// 起因：`/api/admin/plan-health` 在 2026-08-31 整天返回 500，而每一次部署都打印
/// 「deployment healthy」。因为探活只 `curl /health`，那个端点连数据库都不碰。
///
/// 这一类错**编译期和空表上都看不见**：sqlx 解码要求列的 PG 类型和 Rust 类型严格
/// 匹配，而两边写在不同地方（SQL 在字符串里、类型在 `let x: (f64, …)` 上）。对不上
/// 时唯一的表现是接口 500，**且只在查询真的返回了行的时候**。plan-health 那条就是
/// 这样：余额探针 2026-08-25 才上线，在那之前它恒返回 0 行、走 `None` 分支，绿了几个月。
///
/// 所以这里跑的是**同一份代码**，不是抄一份 SQL —— 抄的副本对「被抄那边改了类型
/// 转换」结构性地看不见，等于自检自己。
///
/// **只回名字和成败，不回任何数据**：这个端点和 `/health` 一样免鉴权（部署脚本要在
/// 容器里 curl 它，那时还没有任何凭据）。真实报错写进日志，不进响应体 —— 报错文本
/// 里会带列名和表名。
pub async fn reports(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    let mut checks = Vec::new();
    let mut all_ok = true;
    let mut note = |name: &str, err: Option<String>| {
        if let Some(e) = &err {
            tracing::error!(check = %name, error = %e, event = "health_report_failed",
                "报表查询跑不通 —— 对应的后台页面现在就是 500");
        }
        all_ok &= err.is_none();
        checks.push(json!({ "name": name, "ok": err.is_none() }));
    };

    // 套餐健康页最外层那条：五列里有一列是 EXTRACT(...)/3600.0，PG 14 起回 numeric。
    note(
        "plan_health.measured_upstream",
        crate::plan_health::measured_upstream_per_visible_usd(&state)
            .await
            .err()
            .map(|e| format!("{} {}", e.status, e.msg)),
    );

    // 总览页那条「白送出去的调用」。走的是 stats 用的同一个函数。
    // 它内部 unwrap_or_default，查询炸了会静默变成空 —— 所以这里额外看一眼形状：
    // 缺 key 就说明那条 SQL 没跑通，而页面上只会表现为「今天没有漏收」，
    // 也就是**报表的失败和好消息长得一模一样**，这正是要拦的。
    let zp = crate::realtime::zero_priced_24h(&state).await;
    note(
        "stats.zero_priced_24h",
        (!zp.get("models").map(|m| m.is_array()).unwrap_or(false))
            .then(|| "zero_priced_24h 没有返回 models 数组".to_string()),
    );

    // 免费额度池那一栏。同样走 stats 用的那个函数。
    let fp = crate::realtime::free_pool_value_24h(&state).await;
    note(
        "stats.free_pool_24h",
        (!fp.get("models").map(|m| m.is_array()).unwrap_or(false))
            .then(|| "free_pool_24h 没有返回 models 数组".to_string()),
    );

    let code = if all_ok { StatusCode::OK } else { StatusCode::INTERNAL_SERVER_ERROR };
    (code, Json(json!({ "ok": all_ok, "checks": checks })))
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
        let front_door_word = match latest {
            None => "unknown",
            Some((false, _, _)) => "error",
            Some((true, Some(ms), _)) if (*ms as i64) > SLOW_MS => "degraded",
            Some((true, _, _)) => "ok",
        };

        // **面板上的状态改由真实流量决定，探针只留作「前门通不通」。**
        //
        // 上面那个 `front_door_word` 是探针的结论，而探针发的是一个不带凭据的 GET，
        // 十条线路又共用同一个上游域名 —— 它测的是同一次 TCP 握手，测十遍。
        // 2026-08-19 那次事故里，「Claude 强力版」连续 44 小时零成功，这个词一直是 ok。
        //
        // route_health 用的是这条线路真实请求的结局：连败次数 + 上次成功时刻。
        // 刻意不用「成功率 + 时间窗」：这台机器每条线路每小时只有个位数请求，任何
        // 带样本量门槛的判据都会退到更长的窗，而长窗里装的是故障**之前**的成功，
        // 只会把结论往好看的方向拉。
        let rh = crate::route_health::snapshot(&state, id).await;
        // 多路由之后「线路健康」是它所有出口的并集：健康按出口记（一个坏出口不该拖垮
        // 同线路的好出口），而流量大多走最便宜那个出口，只看线路自带地址的记录，
        // 最忙的线路反而会显示成「不知道」。见 route_endpoints::aggregate_live。
        let state_word =
            crate::route_endpoints::aggregate_live(&state, id, chrono::Utc::now().timestamp())
                .await;

        cards.push(json!({
            "id": id,
            "label": label,
            "provider": provider,
            "model": model_id.unwrap_or_default(),
            "state": state_word,
            "ping_ms": ping_ms,
            // 只增不改：既有的键含义一个都没动（ping_ms 仍是探针握手耗时，availability
            // 仍是探针可达率），新的事实全部走新键，这样没升级的前端 bundle 不会把
            // 秒级数字渲染进「Endpoint response」那一格。
            "front_door": front_door_word,
            "consecutive_failures": rh.consecutive_failures,
            "last_ok_at": rh.last_ok_at,
            "last_attempt_at": rh.last_attempt_at,
            "last_fail_status": rh.last_fail_status,
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
    } else if cards.iter().any(|c| c["state"] == "unknown") {
        // 有一条说不清就整体说不清。原来是「只要不全是 unknown 就报 ok」——
        // 9 条不知道 + 1 条好，整体报好，这正是「没有证据当成好消息」的老毛病。
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

    /// /health 必须暴露响应缓存三计数器，且保持 "ok" 语义（部署脚本按 200 探活）。
    #[tokio::test]
    async fn liveness_reports_response_cache_counters() {
        let v = super::liveness().await.0;
        assert_eq!(v["status"], "ok");
        for key in ["hit", "miss", "store"] {
            assert!(
                v["response_cache"][key].is_u64(),
                "缺了 response_cache.{key} 计数器"
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
