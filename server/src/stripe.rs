//! Stripe Checkout and webhook fulfilment.
//!
//! Two endpoints:
//!   * `POST /api/billing/checkout` — signed-in user picks a `lookup_key`, gets back a
//!     Stripe-hosted Checkout URL. No card data ever touches this server.
//!   * `POST /api/webhooks/stripe`  — Stripe tells us the money arrived; we grant the
//!     plan or credits through the same `codes::apply_*` helpers a redeem code and an
//!     admin grant use, so every path into a user's balance behaves identically.
//!
//! What the catalogue is: rows in `prices`. The Rust here never decides what a product
//! costs or grants — it reads `stripe_price_id`, `plan`, `duration_days`,
//! `credits_cents` off the row. Editing a product is a row update, not a redeploy.
//!
//! Trust boundary: the browser sends a `lookup_key` and nothing else. Amounts, plans
//! and credit grants are all read server-side from the matching row, so a tampered
//! request can at worst buy a different listed product at its real price.

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use std::time::{Duration, Instant};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

const STRIPE_API: &str = "https://api.stripe.com/v1";

/// Stripe signatures older than this are refused, so a captured webhook body cannot be
/// replayed at leisure. Stripe's own libraries default to the same window.
const SIGNATURE_TOLERANCE_SECS: i64 = 300;

/// Secrets come from the environment like everything else in `config.rs`. They are read
/// per call rather than cached in `Config` so that adding them to the container is a
/// restart, not a rebuild — and so a deploy without them still boots and serves every
/// other route, with only billing reporting itself as unconfigured.
fn secret_key() -> Option<String> {
    std::env::var("STRIPE_SECRET_KEY").ok().filter(|s| !s.trim().is_empty())
}

fn webhook_secret() -> Option<String> {
    std::env::var("STRIPE_WEBHOOK_SECRET").ok().filter(|s| !s.trim().is_empty())
}

/// Where Stripe sends the buyer back to. Defaults to the gateway's own dashboard.
fn public_base() -> String {
    std::env::var("PUBLIC_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "https://code.mrday.one".to_string())
        .trim_end_matches('/')
        .to_string()
}

/// One purchasable row, as the billing page needs it.
#[derive(Debug, sqlx::FromRow)]
struct CatalogRow {
    id: uuid::Uuid,
    label: String,
    kind: String,
    plan: Option<String>,
    duration_days: Option<i32>,
    credits_cents: Option<i64>,
    amount_cents: i64,
    amount_usd_cents: Option<i64>,
    stripe_price_id: Option<String>,
    lookup_key: Option<String>,
    recurring: bool,
    once_per_account: bool,
    unit_credits_cents: Option<i64>,
    blurb: String,
}

/// What Stripe says a price actually is. Fetched by lookup key, never typed by hand.
#[derive(Clone, Debug, Default)]
pub struct LivePrice {
    /// Stripe's own id for the price this lookup key currently points at.
    pub id: String,
    /// Minor units in the price's own currency (fen for cny, cents for usd).
    pub cny_minor: Option<i64>,
    pub usd_minor: Option<i64>,
    /// The price's base currency. What Checkout charges when the buyer's currency has no
    /// entry in `currency_options` — so it decides what the card is honest to display.
    pub currency: String,
    /// The product's name and description. These are what the operator edits in Stripe,
    /// and the card shows them verbatim rather than keeping a second copy in the database
    /// that nobody remembers to update.
    pub name: Option<String>,
    pub description: Option<String>,
    /// Per-language overrides read from the Stripe product's metadata, keyed by BCP-47
    /// tag. The operator adds `name_ja` / `description_de` there and the console picks
    /// them up on the next cache refresh — no deploy, and Stripe stays the one place a
    /// product is described.
    pub names: serde_json::Map<String, serde_json::Value>,
    pub descriptions: serde_json::Map<String, serde_json::Value>,
    /// Stripe's `recurring` is null on a one-time price. This decides the checkout mode,
    /// and getting it wrong is a hard 400 from Stripe, not a cosmetic slip.
    pub recurring: bool,
}

static PRICE_CACHE: LazyLock<RwLock<Option<(Instant, HashMap<String, LivePrice>)>>> =
    LazyLock::new(|| RwLock::new(None));
/// Long enough that the billing page is not making an API call per view; short enough
/// that editing a price in Stripe shows up without a deploy.
const PRICE_CACHE_TTL: Duration = Duration::from_secs(120);

/// Read every price this catalogue references straight from Stripe.
///
/// The amounts and the one-time/recurring flag used to be typed into the `prices` table
/// by hand, which meant two sources of truth for the same fact. They drift, and both ways
/// of drifting are bad: a wrong amount advertises a price you do not charge, and a wrong
/// `recurring` flag makes Stripe reject the checkout outright with "You must provide at
/// least one recurring price in subscription mode" — which is exactly how the test plan
/// broke. Asking Stripe removes the second copy.
///
/// Returns an empty map on any failure. Every caller falls back to the stored columns, so
/// Stripe being unreachable degrades the page to its previous behaviour rather than
/// emptying the shop.
async fn live_prices(state: &AppState) -> HashMap<String, LivePrice> {
    if let Some((at, cached)) = PRICE_CACHE.read().ok().and_then(|g| g.clone()) {
        if at.elapsed() < PRICE_CACHE_TTL {
            return cached;
        }
    }
    let Some(key) = secret_key() else {
        return HashMap::new();
    };

    // `currency_options` carries the per-currency amounts of a multi-currency price, and
    // `product` the name and description shown on the card. Neither is returned unless
    // expanded.
    let res = state
        .update_http
        .get(format!("{STRIPE_API}/prices"))
        .bearer_auth(&key)
        .query(&[
            ("limit", "100"),
            ("active", "true"),
            ("expand[]", "data.currency_options"),
            ("expand[]", "data.product"),
        ])
        .send()
        .await;
    let Ok(res) = res else { return HashMap::new() };
    let body: serde_json::Value = res.json().await.unwrap_or_else(|_| json!({}));
    let Some(list) = body.get("data").and_then(|v| v.as_array()) else {
        return HashMap::new();
    };

    let mut out: HashMap<String, LivePrice> = HashMap::new();
    for p in list {
        if let Some((lookup, live)) = parse_price(p) {
            out.insert(lookup, live);
        }
    }

    if let Ok(mut g) = PRICE_CACHE.write() {
        *g = Some((Instant::now(), out.clone()));
    }
    out
}

/// One entry of Stripe's `/v1/prices` list. Split out from the fetch so the shape it
/// expects can be tested against real Stripe payloads without a network call.
///
/// Returns None for a price with no `lookup_key`: the catalogue is keyed by it, and a
/// price that has none is one this gateway was never told to sell.
fn parse_price(p: &serde_json::Value) -> Option<(String, LivePrice)> {
    let lookup = p.get("lookup_key").and_then(|v| v.as_str())?;
    let base_ccy = p.get("currency").and_then(|v| v.as_str()).unwrap_or("");
    let base_amount = p.get("unit_amount").and_then(|v| v.as_i64());
    // A multi-currency price lists its other currencies here; the base one is not
    // repeated, so it has to be folded in by hand.
    let opt = |c: &str| {
        p.pointer(&format!("/currency_options/{c}/unit_amount"))
            .and_then(|v| v.as_i64())
    };
    // Blank strings are Stripe's "unset", not a name — treat them as absent so the card
    // falls back to the stored label instead of rendering an empty heading.
    let text = |path: &str| {
        p.pointer(path)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    Some((
        lookup.to_owned(),
        LivePrice {
            id: p.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_owned(),
            cny_minor: opt("cny").or(if base_ccy == "cny" { base_amount } else { None }),
            usd_minor: opt("usd").or(if base_ccy == "usd" { base_amount } else { None }),
            currency: base_ccy.to_owned(),
            name: text("/product/name"),
            description: text("/product/description"),
            names: localized(p, "name"),
            descriptions: localized(p, "description"),
            recurring: p.get("recurring").map(|v| !v.is_null()).unwrap_or(false),
        },
    ))
}

/// Pull `name_xx` / `description_xx` out of a Stripe product's metadata.
///
/// Stripe metadata keys cannot contain a hyphen in practice for these, so `zh_CN` and
/// `zh-CN` are both accepted and normalised to the BCP-47 form the client asks for.
/// Blank values are dropped so an empty metadata field falls back rather than rendering
/// an empty product name.
fn localized(price: &serde_json::Value, field: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    let Some(meta) = price.pointer("/product/metadata").and_then(|v| v.as_object()) else {
        return out;
    };
    let prefix = format!("{field}_");
    for (k, v) in meta {
        let Some(tag) = k.strip_prefix(&prefix) else { continue };
        let Some(text) = v.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        out.insert(tag.replace('_', "-"), json!(text));
    }
    out
}

/// What a card should print: the amount, and the currency it is honest to print it in.
///
/// Checkout charges a price's base currency unless the buyer's currency has an entry in
/// `currency_options`, so quoting USD is only truthful when Stripe actually carries a USD
/// amount. Falling back to the stored USD column field-by-field is what produced a card
/// reading "¥4 · $0.15": Stripe had moved that price to ¥4 while the column still held the
/// $0.15 from when it was ¥1.
///
/// `stored` is (cny_minor, usd_minor) from the database, used only when Stripe said
/// nothing at all about this price.
fn display_amount(
    live: Option<&LivePrice>,
    stored: (i64, Option<i64>),
) -> (Option<i64>, Option<i64>, String, Option<i64>) {
    let (cny, usd, currency) = match live {
        Some(p) => (
            p.cny_minor,
            p.usd_minor,
            if p.usd_minor.is_some() { "usd".to_owned() } else { p.currency.clone() },
        ),
        None => (Some(stored.0), stored.1, "usd".to_owned()),
    };
    let minor = match currency.as_str() {
        "usd" => usd,
        "cny" => cny,
        _ => None,
    };
    (cny, usd, currency, minor)
}

/// `GET /api/admin/stripe/payments` — what Stripe says happened, for the console.
///
/// The console used to list the local `orders` table and offer a "confirm payment" button
/// beside each pending row. That table records checkout sessions we *created*, not money
/// that *arrived*, so an abandoned checkout sat there looking like an invoice awaiting
/// approval. Stripe is the only thing that knows whether a card was charged, so the
/// console asks Stripe.
///
/// Each row is reconciled against the local order so the operator can see the case that
/// actually matters: Stripe took the money and this gateway granted nothing.
pub async fn admin_payments(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    let Some(key) = secret_key() else {
        return Ok(Json(json!({ "configured": false, "payments": [], "unfulfilled": 0 })));
    };

    // Sessions rather than payment intents: a session carries the buyer's email, the
    // amount, the payment status and the id this gateway stored on its own order, so one
    // list answers every column the console shows.
    let res = state
        .update_http
        .get(format!("{STRIPE_API}/checkout/sessions"))
        .bearer_auth(&key)
        .query(&[("limit", "100")])
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Stripe 无法访问：{e}")))?;
    let body: serde_json::Value = res.json().await.unwrap_or_else(|_| json!({}));
    if let Some(err) = body.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
        return Err(AppError::internal(format!("Stripe 返回错误：{err}")));
    }
    let sessions = body.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    // One query for every session on the page, not one per row.
    let ids: Vec<String> = sessions
        .iter()
        .filter_map(|s| s.get("id").and_then(|v| v.as_str()).map(str::to_owned))
        .collect();
    let local: std::collections::HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT stripe_session_id, status FROM orders WHERE stripe_session_id = ANY($1)",
    )
    .bind(&ids)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect();

    let mut unfulfilled = 0;
    let payments: Vec<serde_json::Value> = sessions
        .iter()
        .map(|s| {
            let id = s.get("id").and_then(|v| v.as_str()).unwrap_or_default();
            let payment_status = s.get("payment_status").and_then(|v| v.as_str()).unwrap_or("");
            let paid = payment_status == "paid" || payment_status == "no_payment_required";
            let order_status = local.get(id).cloned();
            // The case worth alarming about: Stripe took the money, the local order never
            // reached 'paid'. Everything else is an abandoned checkout, which is normal.
            let needs_attention = paid && order_status.as_deref() != Some("paid");
            if needs_attention {
                unfulfilled += 1;
            }
            json!({
                "session_id": id,
                "created": s.get("created").and_then(|v| v.as_i64()),
                "amount": s.get("amount_total").and_then(|v| v.as_i64()),
                "currency": s.get("currency").and_then(|v| v.as_str()),
                "email": s.pointer("/customer_details/email").and_then(|v| v.as_str()),
                "status": s.get("status").and_then(|v| v.as_str()),
                "payment_status": payment_status,
                "paid": paid,
                "payment_intent": s.get("payment_intent").and_then(|v| v.as_str()),
                // None when Stripe knows about a session this gateway never recorded.
                "order_status": order_status,
                "needs_attention": needs_attention,
            })
        })
        .collect();

    Ok(Json(json!({
        "configured": true,
        "payments": payments,
        "unfulfilled": unfulfilled,
    })))
}

/// Which currency to show first, from where the request came from.
///
/// Cloudflare sits in front of this origin and stamps `CF-IPCountry` on every request,
/// so the lookup is free and needs no geo database. The header is advisory only: it
/// picks the default tab, never what Stripe charges — the price id decides that, and a
/// spoofed header cannot change it.
fn currency_for(headers: &HeaderMap) -> (&'static str, Option<String>) {
    let country = headers
        .get("cf-ipcountry")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| s.len() == 2 && s != "XX" && s != "T1");
    match country.as_deref() {
        Some("CN") => ("cny", country),
        _ => ("usd", country),
    }
}

/// GET /api/billing/catalog — the Stripe-purchasable products, plus whether billing is
/// actually wired up. The page renders straight from this; it holds no prices of its own.
pub async fn catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let rows = sqlx::query_as::<_, CatalogRow>(
        "SELECT id, label, kind, plan, duration_days, credits_cents, amount_cents, \
         amount_usd_cents, stripe_price_id, lookup_key, recurring, once_per_account, \
         unit_credits_cents, blurb \
         FROM prices \
         WHERE active = true AND lookup_key IS NOT NULL AND stripe_price_id IS NOT NULL \
         ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;

    // A day pass is sold once per account; say so up front rather than letting the buyer
    // reach Stripe and get refused at fulfilment.
    let spent_once: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT p.lookup_key FROM orders o JOIN prices p ON p.id = o.price_id \
         WHERE o.user_id = $1 AND o.status = 'paid' AND p.once_per_account = true \
           AND p.lookup_key IS NOT NULL",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Stripe is the authority on price and on one-time-vs-subscription; the columns are
    // only a fallback for when it cannot be reached.
    let live = live_prices(&state).await;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let key = r.lookup_key.clone().unwrap_or_default();
            let lp = live.get(&key);
            // What a plan actually grants, from the same table apply_plan uses — so a
            // card can say "$49.77 included" instead of only quoting a price, and can
            // never drift from what the purchase really delivers.
            let spec = r.plan.as_deref().and_then(crate::settings::plan_spec);

            let (cny_minor, usd_minor, display_currency, display_minor) =
                display_amount(lp, (r.amount_cents, r.amount_usd_cents));

            json!({
                "lookup_key": key,
                // Stripe's product name and description win: they are what the operator
                // edits, and the stored copies are only there for when Stripe is silent.
                "label": lp.and_then(|p| p.name.clone()).unwrap_or_else(|| r.label.clone()),
                "kind": r.kind,
                "plan": r.plan,
                "included_cents": spec.map(|s| s.0),
                "window_cap_cents": spec.map(|s| s.1),
                "weekly_cap_cents": spec.map(|s| s.2),
                "duration_days": r.duration_days,
                "credits_cents": r.credits_cents,
                "amount_cents": cny_minor,
                "amount_usd_cents": usd_minor,
                // What the card should print, and in which currency — decided here so the
                // page never has to guess which of the two amounts Stripe will honour.
                "display_currency": display_currency,
                "display_minor": display_minor,
                "recurring": lp.map(|p| p.recurring).unwrap_or(r.recurring),
                "once_per_account": r.once_per_account,
                "unit_credits_cents": r.unit_credits_cents,
                "blurb": lp.and_then(|p| p.description.clone()).unwrap_or_else(|| r.blurb.clone()),
                // Whole maps, not a pre-picked language: the catalogue is cached for two
                // minutes and shared by every reader, so the language has to be chosen
                // per request — which the client does anyway, from its own setting.
                "labels": lp.map(|p| p.names.clone()).unwrap_or_default(),
                "blurbs": lp.map(|p| p.descriptions.clone()).unwrap_or_default(),
                "already_purchased": spent_once.contains(&key),
            })
        })
        .collect();

    let (currency, country) = currency_for(&headers);

    Ok(Json(json!({
        "enabled": secret_key().is_some(),
        "raw_cents_per_credit_usd": crate::settings::raw_cents_per_credit_usd(),
        "currency": currency,
        "country": country,
        "items": items,
    })))
}

#[derive(Deserialize)]
pub struct CheckoutReq {
    pub lookup_key: String,
    /// Only meaningful for the quantity-priced top-up; clamped below.
    #[serde(default)]
    pub quantity: Option<i64>,
}

/// POST /api/billing/checkout — create a Stripe Checkout Session and hand back its URL.
pub async fn checkout(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CheckoutReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let key = secret_key().ok_or_else(|| {
        AppError::bad("支付尚未配置：网关缺少 STRIPE_SECRET_KEY")
    })?;
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let row = sqlx::query_as::<_, CatalogRow>(
        "SELECT id, label, kind, plan, duration_days, credits_cents, amount_cents, \
         amount_usd_cents, stripe_price_id, lookup_key, recurring, once_per_account, \
         unit_credits_cents, blurb \
         FROM prices WHERE lookup_key = $1 AND active = true",
    )
    .bind(&req.lookup_key)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::bad("商品不存在或已下架"))?;

    // Resolved by lookup key, not from the stored column. A Stripe price is immutable:
    // changing an amount means creating a NEW price and moving the lookup key onto it.
    // Reading the id from the key means that move is all it takes — no row to update, and
    // no window where the database points at the old price while Stripe has the new one.
    // The stored column stays as the fallback for when Stripe cannot be reached.
    let live = live_prices(&state).await;
    let live_entry = live.get(&req.lookup_key);
    let price_id = live_entry
        .map(|p| p.id.clone())
        .filter(|id| !id.is_empty())
        .or_else(|| row.stripe_price_id.clone())
        .ok_or_else(|| AppError::bad("该商品未绑定 Stripe 价格"))?;

    // Quantity only applies where the row prices per unit; everything else is one.
    let quantity = if row.unit_credits_cents.is_some() {
        req.quantity.unwrap_or(1).clamp(1, 100_000)
    } else {
        1
    };

    if row.once_per_account {
        let already: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM orders WHERE user_id = $1 AND price_id = $2 AND status = 'paid'",
        )
        .bind(uid)
        .bind(row.id)
        .fetch_one(&state.db)
        .await?;
        if already > 0 {
            return Err(AppError::bad("该商品每个账号仅限购买一次"));
        }
    }

    // Ask Stripe what this price is rather than trusting the stored flag. A row that
    // says "recurring" over a one-time price makes Stripe reject the session outright
    // ("You must provide at least one recurring price in subscription mode"), which is a
    // Subscribe button that silently does nothing. Stripe is the only thing that
    // actually knows, so it is the thing that decides.
    let recurring = live_entry.map(|p| p.recurring).unwrap_or(row.recurring);
    let mode = if recurring { "subscription" } else { "payment" };
    let base = public_base();

    // Form-encoded: Stripe's API does not take JSON.
    let mut form: Vec<(String, String)> = vec![
        ("mode".into(), mode.into()),
        ("line_items[0][price]".into(), price_id),
        ("line_items[0][quantity]".into(), quantity.to_string()),
        (
            "success_url".into(),
            format!("{base}/billing?paid={{CHECKOUT_SESSION_ID}}"),
        ),
        ("cancel_url".into(), format!("{base}/billing?canceled=1")),
        ("client_reference_id".into(), uid.to_string()),
        ("customer_email".into(), claims.email.clone()),
        // Echoed back on the webhook. The webhook re-reads the row anyway, but carrying
        // the ids makes a delivery self-describing when reading Stripe's event log.
        ("metadata[user_id]".into(), uid.to_string()),
        ("metadata[lookup_key]".into(), req.lookup_key.clone()),
        ("metadata[price_row]".into(), row.id.to_string()),
        ("metadata[quantity]".into(), quantity.to_string()),
    ];
    if row.unit_credits_cents.is_some() {
        // Let the buyer change their mind on the Stripe page itself.
        form.push(("line_items[0][adjustable_quantity][enabled]".into(), "true".into()));
        form.push(("line_items[0][adjustable_quantity][minimum]".into(), "1".into()));
        form.push(("line_items[0][adjustable_quantity][maximum]".into(), "100000".into()));
    }

    let res = state
        .update_http
        .post(format!("{STRIPE_API}/checkout/sessions"))
        .bearer_auth(&key)
        // Retrying a failed create must not open two sessions for the same intent — but
        // the key has to move whenever the request does, and it must not outlive the
        // attempt it belongs to. The old key was (user, product, quantity) and broke both
        // ways:
        //
        //   * Stripe remembers a key for 24h and rejects it outright if the parameters
        //     differ. Fixing this product's mode from subscription to payment therefore
        //     locked the buyer out for a day with an idempotency_error, not a price
        //     error — the Subscribe button simply kept failing.
        //   * Buying the same product twice inside 24h replayed the FIRST session instead
        //     of opening a new one, handing the buyer a checkout they had already used.
        //
        // So: everything that shapes the request goes in, plus a 5-minute bucket. A
        // network retry seconds later still collapses onto one session; a genuine second
        // purchase gets a genuine second session.
        .header(
            "Idempotency-Key",
            format!(
                "co_{uid}_{}_{quantity}_{mode}_{}",
                req.lookup_key,
                chrono::Utc::now().timestamp() / 300
            ),
        )
        .form(&form)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("Stripe 不可达：{e}")))?;

    let status = res.status();
    let body: serde_json::Value = res.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        let msg = body
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("Stripe 拒绝了这次结账请求");
        tracing::warn!("Stripe checkout failed ({status}): {body}");
        return Err(AppError::bad(msg.to_string()));
    }

    let url = body
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::internal("Stripe 未返回结账地址"))?;
    let session_id = body.get("id").and_then(|v| v.as_str()).unwrap_or_default();

    // Record the intent now so an abandoned checkout is still visible in the admin
    // order list; the webhook flips it to 'paid'.
    let credits = row
        .unit_credits_cents
        .map(|u| u.saturating_mul(quantity))
        .or(row.credits_cents);
    sqlx::query(
        "INSERT INTO orders (user_id, email, price_id, kind, plan, duration_days, credits_cents, \
         amount_cents, method, status, stripe_session_id, quantity) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'stripe','pending',$9,$10) \
         ON CONFLICT (stripe_session_id) WHERE stripe_session_id IS NOT NULL DO NOTHING",
    )
    .bind(uid)
    .bind(&claims.email)
    .bind(row.id)
    .bind(&row.kind)
    .bind(&row.plan)
    .bind(row.duration_days)
    .bind(credits)
    .bind(row.amount_cents.saturating_mul(quantity))
    .bind(session_id)
    .bind(quantity as i32)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "url": url, "session_id": session_id })))
}

/// Constant-time compare so a wrong signature leaks nothing through timing.
fn secure_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Verify Stripe's `Stripe-Signature` header against the raw body.
///
/// The body must be the bytes as received — parsing to JSON and re-serialising changes
/// whitespace and key order, and the signature would never match again.
fn verify_signature(secret: &str, header: &str, body: &[u8]) -> Result<(), String> {
    let mut timestamp: Option<i64> = None;
    let mut signatures: Vec<&str> = Vec::new();
    for part in header.split(',') {
        let Some((k, v)) = part.trim().split_once('=') else { continue };
        match k {
            "t" => timestamp = v.parse().ok(),
            // v1 is the current scheme; v0 is a test-mode artefact and is not accepted.
            "v1" => signatures.push(v),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or("签名缺少时间戳")?;
    if signatures.is_empty() {
        return Err("签名缺少 v1".into());
    }

    let age = chrono::Utc::now().timestamp() - timestamp;
    if age.abs() > SIGNATURE_TOLERANCE_SECS {
        return Err(format!("签名时间戳超出容忍窗口（{age}s）"));
    }

    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes())
        .map_err(|_| "Webhook 密钥无效".to_string())?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex_lower(&expected);

    if signatures
        .iter()
        .any(|s| secure_eq(s.as_bytes(), expected_hex.as_bytes()))
    {
        Ok(())
    } else {
        Err("签名不匹配".into())
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// POST /api/webhooks/stripe — the only endpoint that may grant a plan or credits
/// without an admin. Unauthenticated by design: Stripe proves who it is with the
/// signature, so an unsigned or stale request is rejected before anything is read.
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    let secret = webhook_secret()
        .ok_or_else(|| AppError::bad("Webhook 未配置：网关缺少 STRIPE_WEBHOOK_SECRET"))?;
    let sig = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("缺少 Stripe-Signature"))?;

    verify_signature(&secret, sig, &body).map_err(|e| {
        tracing::warn!("Stripe webhook rejected: {e}");
        AppError::unauthorized(format!("Stripe 签名校验失败：{e}"))
    })?;

    let event: serde_json::Value =
        serde_json::from_slice(&body).map_err(|_| AppError::bad("事件体不是 JSON"))?;
    let event_id = event.get("id").and_then(|v| v.as_str()).unwrap_or_default();
    let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or_default();
    if event_id.is_empty() {
        return Err(AppError::bad("事件缺少 id"));
    }

    // Idempotency gate — and the grant it guards — in ONE transaction.
    //
    // This used to claim the event on its own connection, commit, and only then fulfil.
    // That combination loses paid orders in silence: if fulfilment then failed for any
    // transient reason (deadlock, pool timeout, a dropped connection), the handler
    // returned 500, Stripe retried, and the retry found the id already present and
    // answered `200 duplicate` — so Stripe stopped retrying and the grant never
    // happened. The customer was charged and received nothing, and both systems
    // reported success. Nothing else would have caught it: there is no reconciliation
    // job, and `invoice.paid` never fires for a one-off purchase.
    //
    // Sharing one transaction makes the claim conditional on the grant. A failure rolls
    // BOTH back, so Stripe's next delivery genuinely re-runs the work. The "give up"
    // paths below (unknown product, unusable user id) deliberately return Ok and let the
    // claim commit — retrying those cannot change the outcome, so they must not loop.
    let mut tx = state.db.begin().await?;
    let claimed =
        sqlx::query("INSERT INTO stripe_events (id, type) VALUES ($1,$2) ON CONFLICT (id) DO NOTHING")
            .bind(event_id)
            .bind(event_type)
            .execute(&mut *tx)
            .await?;
    if claimed.rows_affected() == 0 {
        return Ok(Json(json!({ "ok": true, "duplicate": true })));
    }

    // Anything user-visible is recorded AFTER the commit — an event announced from
    // inside a transaction that later rolls back is a lie told to the admin console.
    let mut post_commit: Vec<(uuid::Uuid, &'static str, serde_json::Value)> = Vec::new();

    match event_type {
        // Covers both one-off payments and the first period of a subscription.
        "checkout.session.completed" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            // Only fulfil once the money is actually there. Async methods can complete
            // the session while payment is still pending.
            let paid = obj
                .get("payment_status")
                .and_then(|v| v.as_str())
                .map(|s| s == "paid" || s == "no_payment_required")
                .unwrap_or(false);
            if paid {
                if let Some((uid, label, quantity)) = fulfil_session(&mut tx, &obj).await? {
                    post_commit.push((
                        uid,
                        "order_paid",
                        json!({ "via": "stripe", "product": label, "quantity": quantity }),
                    ));
                }
            }
        }
        // Fallback for an endpoint that was never subscribed to
        // checkout.session.completed. That is the only event carrying who bought what, so
        // without it the money lands and nothing is granted — which is exactly what
        // happened here: charge.succeeded and payment_intent.succeeded arrived, the
        // session event did not, and the buyer got nothing.
        //
        // Stripe can map a payment intent back to its session, so the missing event can be
        // reconstructed. Double-granting is prevented by the session claim inside
        // fulfil_session, not by hoping only one of the two events is subscribed.
        "payment_intent.succeeded" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            let pi = obj.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_owned();
            if !pi.is_empty() {
                if let Some(session) = session_for_payment_intent(&state, &pi).await {
                    if let Some((uid, label, quantity)) = fulfil_session(&mut tx, &session).await? {
                        tracing::info!("fulfilled {pi} via payment_intent fallback");
                        post_commit.push((
                            uid,
                            "order_paid",
                            json!({ "via": "stripe", "product": label, "quantity": quantity }),
                        ));
                    }
                }
            }
        }
        // Renewals. The first invoice of a subscription arrives here too, but the
        // session already granted it and `stripe_events` keeps that from doubling up
        // only per-event — so renewals are matched on the subscription id instead.
        "invoice.paid" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            fulfil_renewal(&mut tx, &obj).await?;
        }
        // The subscription is over — cancelled, or dunning finally gave up. Until this
        // was handled, cancelling in Stripe never reached this database at all: the row
        // kept its plan and its quota, so a cancelled subscriber went on being served
        // forever. This is the event that actually ends a paid relationship.
        "customer.subscription.deleted" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            if let Some(uid) = end_subscription(&mut tx, &obj).await? {
                post_commit.push((uid, "user_updated", json!({ "by": "stripe", "action": "subscription_deleted" })));
            }
        }
        // Mid-life changes. Only the terminal statuses act: `cancel_at_period_end` is
        // NOT one of them — that subscriber has paid through the end of the period and
        // keeps everything until `deleted` arrives. Revoking here would be taking away
        // time they already bought.
        "customer.subscription.updated" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            let status = obj.get("status").and_then(|v| v.as_str()).unwrap_or_default();
            if is_terminal_subscription_status(status) {
                if let Some(uid) = end_subscription(&mut tx, &obj).await? {
                    post_commit.push((uid, "user_updated", json!({ "by": "stripe", "action": "subscription_ended", "status": status })));
                }
            }
        }
        // A renewal charge failed. Deliberately does NOT revoke: Stripe retries on its
        // own schedule for days, and cutting a paying customer off on the first failed
        // attempt would be wrong. Record it so it is visible, and let the terminal
        // events above do the revoking if it never recovers.
        "invoice.payment_failed" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            if let Some(sub) = obj.get("subscription").and_then(|v| v.as_str()) {
                if let Some(uid) = user_for_subscription(&mut tx, sub).await? {
                    post_commit.push((uid, "payment_failed", json!({ "via": "stripe", "subscription": sub })));
                }
            }
        }
        _ => {}
    }

    tx.commit().await?;

    for (uid, kind, payload) in post_commit {
        crate::realtime::record_event(&state, Some(uid), kind, payload).await;
    }

    Ok(Json(json!({ "ok": true })))
}

/// Statuses from which a subscription never comes back, so entitlement should end.
///
/// `past_due` is absent on purpose: Stripe is still retrying the card, and the
/// subscriber has not lost anything yet. `trialing` and `active` are obviously alive.
/// `paused` is absent too — it resumes.
fn is_terminal_subscription_status(status: &str) -> bool {
    matches!(status, "canceled" | "unpaid" | "incomplete_expired")
}

/// Which account a Stripe subscription belongs to, via the order that created it.
async fn user_for_subscription(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    sub: &str,
) -> ApiResult<Option<uuid::Uuid>> {
    let found: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM orders \
         WHERE stripe_subscription_id = $1 AND user_id IS NOT NULL \
         ORDER BY created_at LIMIT 1",
    )
    .bind(sub)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(found.map(|(uid,)| uid))
}

/// End a subscription: the plan and every quota column go back to nothing, through the
/// same `codes::clear_plan` an operator cancel uses.
async fn end_subscription(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subscription: &serde_json::Value,
) -> ApiResult<Option<uuid::Uuid>> {
    let Some(sub) = subscription.get("id").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    let Some(uid) = user_for_subscription(tx, sub).await? else {
        tracing::warn!("Stripe cancellation for unknown subscription {sub}");
        return Ok(None);
    };
    crate::codes::clear_plan(tx, uid).await?;
    Ok(Some(uid))
}

/// Grant what the purchased row says, in the caller's transaction — the same one that
/// claimed the event — so the grant and the claim commit or fail together.
///
/// `Ok(None)` means "nothing to announce, and do not retry": the payload named a product
/// or a user we cannot resolve, and a redelivery would reach the identical conclusion.
/// Real failures propagate with `?` and take the event claim down with them.
/// On success returns (user, product label, quantity) for the caller to record post-commit.
/// Find the Checkout Session a payment intent belongs to.
///
/// Used only by the fallback path above. Returns None on any failure — a fallback that
/// cannot reach Stripe simply does not fire, and the primary event (or Stripe's retry)
/// remains the path that matters.
async fn session_for_payment_intent(
    state: &AppState,
    payment_intent: &str,
) -> Option<serde_json::Value> {
    let key = secret_key()?;
    let body: serde_json::Value = state
        .update_http
        .get(format!("{STRIPE_API}/checkout/sessions"))
        .bearer_auth(&key)
        .query(&[("payment_intent", payment_intent), ("limit", "1")])
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let session = body.get("data")?.as_array()?.first()?.clone();
    // Only fulfil once the money is actually there, same rule as the primary path.
    let paid = session
        .get("payment_status")
        .and_then(|v| v.as_str())
        .map(|s| s == "paid" || s == "no_payment_required")
        .unwrap_or(false);
    paid.then_some(session)
}

async fn fulfil_session(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: &serde_json::Value,
) -> ApiResult<Option<(uuid::Uuid, String, i64)>> {
    let session_id = session.get("id").and_then(|v| v.as_str()).unwrap_or_default();
    let uid_str = session
        .get("client_reference_id")
        .and_then(|v| v.as_str())
        .or_else(|| session.pointer("/metadata/user_id").and_then(|v| v.as_str()))
        .unwrap_or_default();
    let Ok(uid) = uuid::Uuid::parse_str(uid_str) else {
        tracing::warn!("Stripe session {session_id} has no usable user id");
        return Ok(None);
    };
    let lookup_key = session
        .pointer("/metadata/lookup_key")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let subscription = session
        .get("subscription")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let row = sqlx::query_as::<_, CatalogRow>(
        "SELECT id, label, kind, plan, duration_days, credits_cents, amount_cents, \
         amount_usd_cents, stripe_price_id, lookup_key, recurring, once_per_account, \
         unit_credits_cents, blurb FROM prices WHERE lookup_key = $1",
    )
    .bind(&lookup_key)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        tracing::warn!("Stripe session {session_id} references unknown product {lookup_key}");
        return Ok(None);
    };

    // What the buyer asked for at create time. If they raised it on Stripe's own page
    // the adjusted count arrives on the line items, which are not expanded here — the
    // metadata figure is the floor, so a top-up can never grant more than was charged.
    let quantity = session
        .pointer("/metadata/quantity")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);

    // Claim the SESSION before granting anything.
    //
    // `stripe_events` dedupes by event id, which is not the same thing: one payment can
    // arrive as two different events — checkout.session.completed and, on endpoints
    // subscribed to it, payment_intent.succeeded. Two ids, two passes through here, and
    // with grant() first that is two grants for one payment. Ordering the claim first
    // makes the session itself the unit of "already done".
    let claimed = sqlx::query(
        "UPDATE orders SET status = 'paid', paid_at = now(), stripe_subscription_id = $2 \
         WHERE stripe_session_id = $1 AND status <> 'paid'",
    )
    .bind(session_id)
    .bind(&subscription)
    .execute(&mut **tx)
    .await?;

    if claimed.rows_affected() == 0 {
        // Nothing pending to flip. Either the buyer never went through our checkout (no
        // row at all), or this session was already fulfilled. The insert decides which:
        // the unique index on stripe_session_id makes it atomic, so a conflict means
        // somebody else got there first and we must not grant again.
        let inserted = sqlx::query(
            "INSERT INTO orders (user_id, email, price_id, kind, plan, duration_days, \
             credits_cents, amount_cents, method, status, stripe_session_id, \
             stripe_subscription_id, quantity, paid_at) \
             VALUES ($1,'',$2,$3,$4,$5,$6,$7,'stripe','paid',$8,$9,$10, now()) \
             ON CONFLICT (stripe_session_id) WHERE stripe_session_id IS NOT NULL DO NOTHING",
        )
        .bind(uid)
        .bind(row.id)
        .bind(&row.kind)
        .bind(&row.plan)
        .bind(row.duration_days)
        .bind(row.unit_credits_cents.map(|u| u * quantity).or(row.credits_cents))
        .bind(row.amount_cents * quantity)
        .bind(session_id)
        .bind(&subscription)
        .bind(quantity as i32)
        .execute(&mut **tx)
        .await?;
        if inserted.rows_affected() == 0 {
            tracing::info!("Stripe session {session_id} already fulfilled; skipping");
            return Ok(None);
        }
    }

    grant(tx, uid, &row, quantity).await?;

    if let Some(cust) = session.get("customer").and_then(|v| v.as_str()) {
        let _ = sqlx::query("UPDATE users SET stripe_customer_id = $1 WHERE id = $2")
            .bind(cust)
            .bind(uid)
            .execute(&mut **tx)
            .await;
    }

    Ok(Some((uid, row.label.clone(), quantity)))
}

/// A subscription renewed. Extend the plan by another period.
///
/// The first invoice of a subscription is skipped: the Checkout session already granted
/// that period, and granting again here would hand out two months for one payment.
async fn fulfil_renewal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    invoice: &serde_json::Value,
) -> ApiResult<()> {
    let reason = invoice
        .get("billing_reason")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if reason != "subscription_cycle" {
        return Ok(());
    }
    let Some(sub) = invoice.get("subscription").and_then(|v| v.as_str()) else {
        return Ok(());
    };

    // Find the original order for this subscription; it names the product.
    let found: Option<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
        "SELECT user_id, price_id FROM orders \
         WHERE stripe_subscription_id = $1 AND user_id IS NOT NULL AND price_id IS NOT NULL \
         ORDER BY created_at LIMIT 1",
    )
    .bind(sub)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((uid, price_id)) = found else {
        tracing::warn!("Stripe renewal for unknown subscription {sub}");
        return Ok(());
    };

    let row = sqlx::query_as::<_, CatalogRow>(
        "SELECT id, label, kind, plan, duration_days, credits_cents, amount_cents, \
         amount_usd_cents, stripe_price_id, lookup_key, recurring, once_per_account, \
         unit_credits_cents, blurb FROM prices WHERE id = $1",
    )
    .bind(price_id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = row {
        grant(tx, uid, &row, 1).await?;
        let _ = sqlx::query(
            "INSERT INTO orders (user_id, email, price_id, kind, plan, duration_days, \
             credits_cents, amount_cents, method, status, stripe_subscription_id, paid_at) \
             VALUES ($1,'',$2,$3,$4,$5,$6,$7,'stripe','paid',$8, now())",
        )
        .bind(uid)
        .bind(row.id)
        .bind(&row.kind)
        .bind(&row.plan)
        .bind(row.duration_days)
        .bind(row.credits_cents)
        .bind(row.amount_cents)
        .bind(sub)
        .execute(&mut **tx)
        .await;
    }
    Ok(())
}

/// The single place a Stripe purchase turns into entitlement. Deliberately routed
/// through `codes::apply_*` so a card payment, a redeem code and an admin grant all
/// stack quota the same way.
async fn grant(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
    row: &CatalogRow,
    quantity: i64,
) -> ApiResult<()> {
    if row.kind == "plan" {
        crate::codes::apply_plan(
            tx,
            uid,
            row.plan.as_deref().unwrap_or("none"),
            row.duration_days.unwrap_or(0),
        )
        .await
    } else {
        let cents = row
            .unit_credits_cents
            .map(|u| u.saturating_mul(quantity))
            .or(row.credits_cents)
            .unwrap_or(0);
        crate::codes::apply_credits(tx, uid, cents).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The claim on `stripe_events` must never commit independently of the grant.
    ///
    /// This is a source-level assertion because the failure it guards needs a database
    /// that fails halfway, which this suite has no way to stage. The bug it pins was
    /// real: the INSERT ran on `state.db` (its own auto-committed connection) and the
    /// fulfilment opened a separate transaction afterwards. A transient failure between
    /// them left the event claimed and nothing granted, and because the retry then saw
    /// the id already present and answered `200 duplicate`, Stripe stopped retrying — a
    /// paid customer, no entitlement, and success reported on both sides.
    ///
    /// Written against the source rather than behaviour so that reintroducing the shape
    /// fails, not just reintroducing the symptom.
    #[test]
    fn the_event_claim_shares_the_grants_transaction() {
        let src = include_str!("stripe.rs");
        let claim = src
            .split_once("INSERT INTO stripe_events")
            .expect("the idempotency INSERT should still exist")
            .1;
        // The executor is named a few lines below the SQL, after the binds.
        let executor = claim
            .split_once(".execute(")
            .expect("the INSERT should be executed")
            .1;
        let executor: String = executor.chars().take(40).collect();
        assert!(
            executor.contains("tx"),
            "the stripe_events claim must run inside the fulfilment transaction, \
             but it executes against `{}` — committing the claim on its own connection \
             silently drops paid orders when fulfilment then fails",
            executor.trim()
        );
        assert!(
            !executor.contains("state.db"),
            "the stripe_events claim must not run on the pool directly: {}",
            executor.trim()
        );
    }

    /// `idx_orders_stripe_session` is intentionally partial so legacy rows with no
    /// Stripe session remain valid. PostgreSQL can only infer that index when the
    /// conflict target repeats its predicate; omitting it causes every checkout
    /// pre-write and webhook fallback to fail with 42P10.
    #[test]
    fn stripe_session_conflicts_match_the_partial_unique_index() {
        let src = include_str!("stripe.rs");
        let conflict_target =
            "ON CONFLICT (stripe_session_id) WHERE stripe_session_id IS NOT NULL DO NOTHING";
        let production = src
            .split_once("#[cfg(test)]")
            .expect("the test module must follow the Stripe SQL")
            .0;
        assert_eq!(
            production.matches(conflict_target).count(),
            2,
            "both Stripe order INSERTs must infer the partial session index"
        );

        let migration = include_str!("../migrations/20260808_stripe_billing.sql");
        assert!(
            migration.contains("ON orders (stripe_session_id) WHERE stripe_session_id IS NOT NULL"),
            "the regression test must stay aligned with the already-applied partial index"
        );

        let checkout_write = production
            .split_once("// Record the intent now")
            .expect("checkout pending-order write")
            .1
            .split_once("Ok(Json")
            .expect("checkout response follows the pending-order write")
            .0;
        assert!(
            checkout_write.contains(".await?;"),
            "checkout must surface a failed pending-order write"
        );
        assert!(
            !checkout_write.contains("let _ = sqlx::query"),
            "checkout must not silently discard a pending-order write failure"
        );
    }

    /// Only genuinely dead subscriptions revoke. `past_due` means Stripe is still
    /// retrying the card — cutting that customer off would be taking away a period
    /// they may yet pay for.
    #[test]
    fn only_terminal_statuses_end_a_subscription() {
        for dead in ["canceled", "unpaid", "incomplete_expired"] {
            assert!(is_terminal_subscription_status(dead), "{dead} should revoke");
        }
        for alive in ["active", "trialing", "past_due", "paused", "incomplete", ""] {
            assert!(
                !is_terminal_subscription_status(alive),
                "{alive} must NOT revoke — the subscriber has not lost the period they paid for"
            );
        }
    }

    /// Every event the fulfilment logic depends on must actually be handled. Adding a
    /// branch is cheap; noticing months later that cancellations were never wired up is
    /// not — that gap let a cancelled subscriber keep their plan indefinitely.
    #[test]
    fn the_lifecycle_events_are_all_handled() {
        let src = include_str!("stripe.rs");
        for event in [
            "checkout.session.completed",
            "invoice.paid",
            "customer.subscription.deleted",
            "customer.subscription.updated",
            "invoice.payment_failed",
        ] {
            assert!(
                src.contains(&format!("\"{event}\" =>")),
                "no match arm for {event}"
            );
        }
    }

    /// A real Stripe header, signed with a known secret, must verify — and every way of
    /// tampering with it must not.
    fn sign(secret: &str, ts: i64, body: &[u8]) -> String {
        let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(ts.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        hex_lower(&mac.finalize().into_bytes())
    }

    #[test]
    fn a_correctly_signed_payload_verifies() {
        let secret = "whsec_test_abc";
        let body = br#"{"id":"evt_1","type":"checkout.session.completed"}"#;
        let ts = chrono::Utc::now().timestamp();
        let header = format!("t={ts},v1={}", sign(secret, ts, body));
        assert!(verify_signature(secret, &header, body).is_ok());
    }

    #[test]
    fn a_tampered_body_is_refused() {
        let secret = "whsec_test_abc";
        let body = br#"{"id":"evt_1","amount":100}"#;
        let ts = chrono::Utc::now().timestamp();
        let header = format!("t={ts},v1={}", sign(secret, ts, body));
        let tampered = br#"{"id":"evt_1","amount":999999}"#;
        assert!(
            verify_signature(secret, &header, tampered).is_err(),
            "a rewritten payload must not pass with the original signature"
        );
    }

    #[test]
    fn the_wrong_secret_is_refused() {
        let body = br#"{"id":"evt_1"}"#;
        let ts = chrono::Utc::now().timestamp();
        let header = format!("t={ts},v1={}", sign("whsec_real", ts, body));
        assert!(verify_signature("whsec_attacker", &header, body).is_err());
    }

    #[test]
    fn a_stale_signature_is_refused() {
        let secret = "whsec_test_abc";
        let body = br#"{"id":"evt_1"}"#;
        let ts = chrono::Utc::now().timestamp() - (SIGNATURE_TOLERANCE_SECS + 60);
        let header = format!("t={ts},v1={}", sign(secret, ts, body));
        assert!(
            verify_signature(secret, &header, body).is_err(),
            "an old capture must not be replayable"
        );
    }

    #[test]
    fn a_header_missing_its_parts_is_refused() {
        let body = br#"{"id":"evt_1"}"#;
        assert!(verify_signature("s", "", body).is_err());
        assert!(verify_signature("s", "t=123", body).is_err(), "no v1");
        let ts = chrono::Utc::now().timestamp();
        assert!(
            verify_signature("s", &format!("v1={}", sign("s", ts, body)), body).is_err(),
            "no timestamp"
        );
    }

    /// Stripe sends `v0` alongside `v1` in some test payloads; only `v1` counts.
    #[test]
    fn a_v0_signature_alone_is_refused() {
        let secret = "whsec_test_abc";
        let body = br#"{"id":"evt_1"}"#;
        let ts = chrono::Utc::now().timestamp();
        let header = format!("t={ts},v0={}", sign(secret, ts, body));
        assert!(verify_signature(secret, &header, body).is_err());
    }

    /// Multiple v1s appear while a signing secret is being rotated; any valid one wins.
    #[test]
    fn one_valid_signature_among_several_is_enough() {
        let secret = "whsec_test_abc";
        let body = br#"{"id":"evt_1"}"#;
        let ts = chrono::Utc::now().timestamp();
        let header = format!("t={ts},v1=deadbeef,v1={}", sign(secret, ts, body));
        assert!(verify_signature(secret, &header, body).is_ok());
    }

    /// Shaped like a real `/v1/prices` entry with `currency_options` and `product`
    /// expanded, which is how the gateway asks for them.
    fn stripe_price(extra: serde_json::Value) -> serde_json::Value {
        let mut base = json!({
            "id": "price_123",
            "lookup_key": "starter_monthly",
            "currency": "cny",
            "unit_amount": 8800,
            "recurring": { "interval": "month" },
            "product": { "name": "Starter", "description": "For everyday work." }
        });
        let (serde_json::Value::Object(b), serde_json::Value::Object(e)) = (&mut base, extra) else {
            unreachable!()
        };
        for (k, v) in e {
            b.insert(k, v);
        }
        base
    }

    #[test]
    fn product_metadata_supplies_per_language_names() {
        let (_, live) = parse_price(&stripe_price(json!({
            "product": {
                "name": "Starter",
                "description": "For everyday work.",
                "metadata": {
                    "name_ja": "スターター",
                    "name_zh_CN": "入门版",
                    "description_de": "Für die tägliche Arbeit.",
                    // Blank must not shadow the English original with an empty heading.
                    "name_es": "   ",
                    // Not one of ours; must not be mistaken for a language.
                    "internal_note": "do not ship"
                }
            }
        })))
        .unwrap();
        assert_eq!(live.names.get("ja").and_then(|v| v.as_str()), Some("スターター"));
        // Stripe metadata keys use underscores; the client asks in BCP-47.
        assert_eq!(live.names.get("zh-CN").and_then(|v| v.as_str()), Some("入门版"));
        assert_eq!(live.descriptions.get("de").and_then(|v| v.as_str()), Some("Für die tägliche Arbeit."));
        assert!(!live.names.contains_key("es"), "a blank override must fall back");
        assert!(!live.names.contains_key("internal-note"));
        assert_eq!(live.name.as_deref(), Some("Starter"), "English is still the base");
    }

    #[test]
    fn a_product_with_no_metadata_yields_no_overrides() {
        let (_, live) = parse_price(&stripe_price(json!({}))).unwrap();
        assert!(live.names.is_empty());
        assert!(live.descriptions.is_empty());
    }

    #[test]
    fn a_price_without_a_lookup_key_is_not_ours_to_sell() {
        let mut p = stripe_price(json!({}));
        p.as_object_mut().unwrap().remove("lookup_key");
        assert!(parse_price(&p).is_none());
    }

    #[test]
    fn the_base_currency_amount_is_folded_in() {
        // Stripe does not repeat the base currency inside currency_options, so a CNY-only
        // price would otherwise parse as having no amount at all.
        let (key, live) = parse_price(&stripe_price(json!({}))).unwrap();
        assert_eq!(key, "starter_monthly");
        assert_eq!(live.cny_minor, Some(8800));
        assert_eq!(live.usd_minor, None);
        assert_eq!(live.currency, "cny");
        assert!(live.recurring);
    }

    #[test]
    fn a_multi_currency_price_reports_both() {
        let (_, live) = parse_price(&stripe_price(json!({
            "currency_options": { "usd": { "unit_amount": 1299 } }
        })))
        .unwrap();
        assert_eq!(live.cny_minor, Some(8800));
        assert_eq!(live.usd_minor, Some(1299));
    }

    #[test]
    fn a_one_time_price_is_not_recurring() {
        // Stripe sends `recurring: null`, and mistaking it for a subscription is a hard
        // 400 at checkout, not a cosmetic slip.
        let (_, live) = parse_price(&stripe_price(json!({ "recurring": serde_json::Value::Null }))).unwrap();
        assert!(!live.recurring);
    }

    #[test]
    fn a_blank_product_name_falls_back_rather_than_rendering_empty() {
        let (_, live) = parse_price(&stripe_price(json!({
            "product": { "name": "   ", "description": "" }
        })))
        .unwrap();
        assert_eq!(live.name, None);
        assert_eq!(live.description, None);
    }

    #[test]
    fn the_product_name_and_description_come_through() {
        let (_, live) = parse_price(&stripe_price(json!({}))).unwrap();
        assert_eq!(live.name.as_deref(), Some("Starter"));
        assert_eq!(live.description.as_deref(), Some("For everyday work."));
    }

    #[test]
    fn dollars_are_quoted_only_when_stripe_carries_them() {
        let live = LivePrice {
            cny_minor: Some(400),
            usd_minor: None,
            currency: "cny".into(),
            ..Default::default()
        };
        // The stored column still says $0.15 from when this price was ¥1. Quoting it
        // would advertise a figure nobody is charged.
        let (_, _, currency, minor) = display_amount(Some(&live), (100, Some(15)));
        assert_eq!(currency, "cny");
        assert_eq!(minor, Some(400), "must quote Stripe's ¥4, not the stale ¥1");
    }

    #[test]
    fn dollars_win_when_stripe_has_a_usd_amount() {
        let live = LivePrice {
            cny_minor: Some(8800),
            usd_minor: Some(1299),
            currency: "cny".into(),
            ..Default::default()
        };
        let (_, _, currency, minor) = display_amount(Some(&live), (8800, Some(9999)));
        assert_eq!(currency, "usd");
        assert_eq!(minor, Some(1299), "Stripe's USD, never the stored column");
    }

    #[test]
    fn the_stored_columns_are_used_only_when_stripe_said_nothing() {
        // No key configured, or Stripe unreachable: the shop degrades to its old
        // behaviour rather than emptying itself.
        let (cny, usd, currency, minor) = display_amount(None, (8800, Some(1299)));
        assert_eq!(cny, Some(8800));
        assert_eq!(usd, Some(1299));
        assert_eq!(currency, "usd");
        assert_eq!(minor, Some(1299));
    }

    #[test]
    fn an_unsupported_currency_quotes_nothing_rather_than_a_wrong_number() {
        let live = LivePrice {
            cny_minor: None,
            usd_minor: None,
            currency: "eur".into(),
            ..Default::default()
        };
        let (_, _, currency, minor) = display_amount(Some(&live), (8800, Some(1299)));
        assert_eq!(currency, "eur");
        assert_eq!(minor, None, "no EUR amount was parsed, so there is nothing honest to print");
    }

    fn hdr(country: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Some(c) = country {
            h.insert("cf-ipcountry", c.parse().unwrap());
        }
        h
    }

    #[test]
    fn mainland_visitors_see_yuan_and_everyone_else_dollars() {
        assert_eq!(currency_for(&hdr(Some("CN"))).0, "cny");
        assert_eq!(currency_for(&hdr(Some("cn"))).0, "cny", "the header case must not matter");
        assert_eq!(currency_for(&hdr(Some("US"))).0, "usd");
        assert_eq!(currency_for(&hdr(Some("HK"))).0, "usd");
        assert_eq!(currency_for(&hdr(Some("GB"))).0, "usd");
    }

    #[test]
    fn an_unusable_country_falls_back_to_dollars() {
        // No Cloudflare in front (direct origin hit), and the two values Cloudflare
        // itself uses when it cannot place the client.
        assert_eq!(currency_for(&hdr(None)), ("usd", None));
        assert_eq!(currency_for(&hdr(Some("XX"))), ("usd", None));
        assert_eq!(currency_for(&hdr(Some("T1"))), ("usd", None));
        assert_eq!(currency_for(&hdr(Some("nonsense"))), ("usd", None));
        assert_eq!(currency_for(&hdr(Some(""))), ("usd", None));
    }

    #[test]
    fn secure_eq_matches_only_identical_slices() {
        assert!(secure_eq(b"abc", b"abc"));
        assert!(!secure_eq(b"abc", b"abd"));
        assert!(!secure_eq(b"abc", b"ab"));
    }

    #[test]
    fn hex_is_lowercase_and_padded() {
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xff]), "000fff");
    }
}
