//! Paying referrers automatically, through Stripe Connect.
//!
//! The manual path — operator reads an account off the screen, sends money, ticks a box —
//! stays exactly where it was. This is the other half: a referrer who connects a Stripe
//! account gets paid by a Transfer the moment they ask, with nobody in the loop.
//!
//! WHAT THIS SERVICE NEVER TOUCHES. No bank details, no ID documents, no KYC answers.
//! Onboarding happens on Stripe's own hosted pages; all that comes back here is an
//! `acct_…` id, and everything about whether that account may receive money is asked of
//! Stripe at the moment of paying rather than cached and trusted.
//!
//! THE THREE WAYS A PAYOUT DOES NOT HAPPEN, all of them normal:
//!   * the referrer has not connected an account — nothing to transfer to;
//!   * they connected one but have not finished onboarding — Stripe refuses;
//!   * the platform's *available* balance is short. This is the one that surprises people:
//!     transfers come out of available balance, and money from a sale sits in `pending` for
//!     days first. A transfer against a pending balance fails with `balance_insufficient`.
//!
//! In all three the withdrawal request survives as a pending row with the reason on it, and
//! the operator can pay it by hand. A failed automatic payout must never look like a
//! request nobody has got to yet.

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

const STRIPE_API: &str = "https://api.stripe.com/v1";

fn secret_key() -> Option<String> {
    std::env::var("STRIPE_SECRET_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn account_base() -> String {
    std::env::var("PUBLIC_ACCOUNT_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://my.mrday.one".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// Readiness, straight from Stripe. Deliberately not cached: `payouts_enabled` can be
/// withdrawn by Stripe at any time (a failed verification, a document expiring), and a
/// stale "ready" here means a transfer attempt that fails in front of the user.
async fn account_state(state: &AppState, acct: &str) -> Option<(bool, bool)> {
    let key = secret_key()?;
    let res = state
        .update_http
        .get(format!("{STRIPE_API}/accounts/{acct}"))
        .bearer_auth(&key)
        .send()
        .await
        .ok()?;
    if !res.status().is_success() {
        return None;
    }
    let a: serde_json::Value = res.json().await.ok()?;
    Some((
        a.get("payouts_enabled").and_then(|v| v.as_bool()).unwrap_or(false),
        a.get("details_submitted").and_then(|v| v.as_bool()).unwrap_or(false),
    ))
}

async fn account_of(state: &AppState, uid: uuid::Uuid) -> ApiResult<Option<String>> {
    Ok(sqlx::query_scalar::<_, Option<String>>(
        "SELECT stripe_connect_account_id FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await?
    .flatten())
}

/// `GET /api/referral/connect` — can this account be paid automatically, and if not, why.
pub async fn status(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    // Whether the feature can work at all, so the UI can say "not available" rather than
    // offering a button that will fail.
    let configured = secret_key().is_some();
    let Some(acct) = account_of(&state, uid).await? else {
        return Ok(Json(json!({
            "configured": configured,
            "connected": false,
            "ready": false,
            "missing": ["connect_onboarding"],
        })));
    };

    let (payouts_enabled, details_submitted) =
        account_state(&state, &acct).await.unwrap_or((false, false));
    let mut missing: Vec<&str> = Vec::new();
    if !details_submitted {
        missing.push("details_submitted");
    }
    if !payouts_enabled {
        missing.push("payouts_enabled");
    }

    Ok(Json(json!({
        "configured": configured,
        "connected": true,
        "ready": missing.is_empty(),
        "missing": missing,
    })))
}

/// `POST /api/referral/connect/start` — begin or resume Stripe's onboarding.
///
/// Creates the connected account on first call and reuses it forever after: a second
/// account for the same person would split their payout history and strand whatever was
/// sent to the first. The returned link is single-use and short-lived, which is Stripe's
/// design — it is generated fresh on every call rather than stored.
pub async fn start(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let key = secret_key().ok_or_else(|| AppError::bad("尚未配置 Stripe，无法开通自动打款"))?;

    // Only people actually in the programme. Otherwise this is an open endpoint for
    // creating Stripe accounts on the platform's tab.
    let granted: Option<String> =
        sqlx::query_scalar::<_, Option<String>>("SELECT referral_code FROM users WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await?
            .flatten();
    if granted.is_none() {
        return Err(AppError::forbidden("还没有开通分销资格"));
    }

    let acct = match account_of(&state, uid).await? {
        Some(a) => a,
        None => {
            let res = state
                .update_http
                .post(format!("{STRIPE_API}/accounts"))
                // Same person, same account, however many times they click. Stripe holds an
                // idempotency key for 24h, which covers the double-click this is for.
                .header("Idempotency-Key", format!("acct_{uid}"))
                .bearer_auth(&key)
                .form(&[
                    ("type", "express"),
                    ("email", claims.email.as_str()),
                    ("capabilities[transfers][requested]", "true"),
                    ("metadata[user_id]", &uid.to_string()),
                ])
                .send()
                .await
                .map_err(|e| AppError::internal(format!("Stripe 不可达：{e}")))?;
            let body: serde_json::Value = res.json().await.unwrap_or_else(|_| json!({}));
            let Some(id) = body.get("id").and_then(|v| v.as_str()) else {
                let msg = body
                    .pointer("/error/message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Stripe 拒绝了开户请求");
                return Err(AppError::bad(msg.to_string()));
            };
            // Written before the link is handed out. If this fails we would create a fresh
            // account on the next click and orphan this one at Stripe.
            sqlx::query("UPDATE users SET stripe_connect_account_id = $2 WHERE id = $1")
                .bind(uid)
                .bind(id)
                .execute(&state.db)
                .await?;
            id.to_string()
        }
    };

    let base = account_base();
    let res = state
        .update_http
        .post(format!("{STRIPE_API}/account_links"))
        .bearer_auth(&key)
        .form(&[
            ("account", acct.as_str()),
            ("type", "account_onboarding"),
            // Stripe sends the browser here when the link expires unused, and here again
            // when onboarding finishes. Both are the same screen; it re-reads status.
            ("refresh_url", &format!("{base}/withdraw?connect=refresh")),
            ("return_url", &format!("{base}/withdraw?connect=done")),
        ])
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Stripe 不可达：{e}")))?;
    let body: serde_json::Value = res.json().await.unwrap_or_else(|_| json!({}));
    let Some(url) = body.get("url").and_then(|v| v.as_str()) else {
        let msg = body
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("Stripe 没有返回开户链接");
        return Err(AppError::bad(msg.to_string()));
    };

    Ok(Json(json!({ "url": url })))
}

/// 打款的三种结局。三种，不是两种 —— 这是这个模块最要紧的一个区分。
///
/// 「被拒绝」和「不知道」必须分开，因为它们的后续动作完全相反：被拒绝可以安全地把佣金放回去
/// 重来，不知道则绝对不能。把两者合成一个 Skipped，就等于在 Stripe 返回 500（它带 JSON 错误
/// 体）或者读响应体超时的时候，把一笔**可能已经转出去的钱**判成"没转成"，然后下一轮用一个
/// **新的**幂等键再转一次 —— Stripe 看到的是两个互不相干的请求，于是真的付两次。
pub enum Payout {
    /// 钱动了。带着 Stripe 的 `tr_…`。
    Sent(String),
    /// 明确被拒绝，钱没动。可以回滚重试。
    Refused(String),
    /// 结果不明：可能已经成功了。绝不能重试，交给人核对。
    Unknown(String),
}

/// Send one withdrawal through Stripe, or explain why it could not go.
///
/// IDEMPOTENCY. The withdrawal id is the Idempotency-Key, so a retry — a timeout that
/// actually succeeded, a double submit, a redeploy mid-flight — returns Stripe's original
/// transfer instead of making a second one. The unique index on `withdrawals.transfer_id`
/// is the second half of that guarantee.
///
/// Never returns Err. A payout that cannot go is not an error in the request that triggered
/// it: the withdrawal has been recorded either way, and the fallback is the queue that
/// existed before any of this.
pub async fn pay(state: &AppState, withdrawal: uuid::Uuid, uid: uuid::Uuid, cents: i64) -> Payout {
    let Some(key) = secret_key() else {
        return Payout::Refused("Stripe 未配置".into());
    };
    let acct = match account_of(state, uid).await {
        Ok(Some(a)) => a,
        _ => return Payout::Refused("未连接收款账户".into()),
    };

    // Asked now, not remembered. See `account_state`.
    match account_state(state, &acct).await {
        Some((true, true)) => {}
        Some((payouts, details)) => {
            return Payout::Refused(format!(
                "收款账户尚未就绪（payouts_enabled={payouts}, details_submitted={details}）"
            ));
        }
        // 读不到账户状态就没发过转账请求，安全，可重试。
        None => return Payout::Refused("无法读取收款账户状态".into()),
    }

    let res = state
        .update_http
        .post(format!("{STRIPE_API}/transfers"))
        .bearer_auth(&key)
        .header("Idempotency-Key", withdrawal.to_string())
        .form(&[
            ("amount", cents.to_string().as_str()),
            // Transfers must match the balance they are drawn from. The platform account is
            // USD and every price is charged in USD or CNY through it; a currency mismatch
            // is refused by Stripe rather than silently converted.
            ("currency", "usd"),
            ("destination", acct.as_str()),
            ("metadata[withdrawal_id]", &withdrawal.to_string()),
            ("metadata[user_id]", &uid.to_string()),
        ])
        .send()
        .await;

    let Ok(res) = res else {
        // 连请求都没发出去/没收到回应。转账可能已经在 Stripe 那边建好了。
        return Payout::Unknown("Stripe 不可达".into());
    };

    // 状态码要在消费 body 之前拿到 —— res.json() 会把 res 吃掉。
    let status = res.status();

    // 5xx 和 429：Stripe 自己也不保证这次请求有没有落地。当作不明，不当作拒绝。
    if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Payout::Unknown(format!("Stripe 返回 {status}，结果不明"));
    }

    // 响应体读不出来（截断、超时打在读 body 上）同样是不明：HTTP 200 也可能走到这里。
    let Ok(body) = res.json::<serde_json::Value>().await else {
        return Payout::Unknown("Stripe 响应无法解析，结果不明".into());
    };

    match body.get("id").and_then(|v| v.as_str()) {
        Some(id) => Payout::Sent(id.to_string()),
        // 到这里才是真正的拒绝：4xx，带着 Stripe 自己的说法。
        None => Payout::Refused(
            body.pointer("/error/message")
                .and_then(|v| v.as_str())
                .unwrap_or("Stripe 拒绝了这次转账")
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    /// A payout that could not be sent must never be recorded as sent.
    #[test]
    fn a_transfer_is_only_claimed_when_stripe_returns_an_id() {
        let src = include_str!("connect.rs");
        let f = src.split("pub async fn pay(").nth(1).expect("pay");
        let body = &f[..f.find("\n#[cfg(test)]").unwrap_or(f.len())];
        assert!(
            body.contains("match body.get(\"id\").and_then(|v| v.as_str())"),
            "Sent must be gated on Stripe returning a transfer id, not on the HTTP status",
        );
        assert!(
            body.contains("Idempotency-Key") && body.contains("withdrawal.to_string()"),
            "the withdrawal id must be the idempotency key, or a retried timeout pays twice",
        );
        assert!(
            !body.contains("return Err("),
            "paying must not fail the caller: the withdrawal is recorded either way and \
             falls back to the manual queue",
        );
        assert!(
            body.contains("status.is_server_error()") && body.contains("TOO_MANY_REQUESTS"),
            "5xx/429 必须判成 Unknown：Stripe 在这些情况下不保证请求有没有落地，\
             判成拒绝就会在下一轮用新的幂等键再转一次 —— 真的付两次",
        );
        assert!(
            body.contains("let status = res.status();"),
            "状态码必须在消费 body 之前取，否则拿不到",
        );
        assert!(
            !body.contains("unwrap_or_else(|_| json!({}))"),
            "响应体读不出来是「结果不明」，不能吞成空对象再判成拒绝",
        );
    }

    /// Readiness must be asked of Stripe every time, never remembered.
    #[test]
    fn readiness_is_never_cached() {
        let src = include_str!("connect.rs");
        let f = src.split("pub async fn pay(").nth(1).expect("pay");
        let body = &f[..f.find("\n#[cfg(test)]").unwrap_or(f.len())];
        assert!(
            body.contains("account_state(state, &acct).await"),
            "Stripe can withdraw payouts_enabled at any time; a cached 'ready' is a \
             transfer that fails in front of the user",
        );
    }
}
