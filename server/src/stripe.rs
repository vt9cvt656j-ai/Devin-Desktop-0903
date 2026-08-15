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
    want: &str,
) -> (Option<i64>, Option<i64>, String, Option<i64>) {
    let (cny, usd, currency) = match live {
        Some(p) => {
            // 买家要人民币、而这个价格在 Stripe 上确实有人民币金额，才显示人民币。
            // 否则退回美元 —— 日卡和三个加油包没有人民币价，中国买家看到的就是美元，
            // 那也是他们实际会被收的币种，卡上和结账页因此永远一致。
            let ccy = if want == "cny" && p.cny_minor.is_some() {
                "cny".to_owned()
            } else if p.usd_minor.is_some() {
                "usd".to_owned()
            } else {
                p.currency.clone()
            };
            (p.cny_minor, p.usd_minor, ccy)
        }
        None => (
            Some(stored.0),
            stored.1,
            if want == "cny" { "cny".to_owned() } else { "usd".to_owned() },
        ),
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

/// 这次请求按哪个币种定价，以及判定所依据的三个信号。
pub struct Buyer {
    pub currency: &'static str,
    pub country: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
    pub offset_minutes: Option<i32>,
}

/// 中国大陆时区名。乌鲁木齐/喀什也算：它们在中国境内，只是历史上曾用过 +6。
const CN_ZONES: &[&str] = &[
    "asia/shanghai", "asia/chongqing", "asia/chungking", "asia/harbin",
    "asia/urumqi", "asia/kashgar", "asia/macau", "prc",
];

/// 简体中文标签。刻意不含 zh-TW / zh-HK / zh-Hant —— 那是港澳台，不按大陆价。
fn language_says_cn(tag: &str) -> bool {
    let t = tag.trim().to_ascii_lowercase();
    let t = t.split(|c| c == ';' || c == ',').next().unwrap_or("").trim();
    matches!(t, "zh" | "zh-cn" | "zh-hans" | "zh-hans-cn" | "zh-sg") || t.starts_with("zh-hans")
}

/// 判定这次请求是不是中国区买家。
///
/// 规则按需求原话实现：**语言和时区都指向中国，就算中国用户，哪怕 IP 不是**。
/// 换句话说 IP 只是三个信号之一，它单独为真也算（人在国内直连），但它为假否决不了
/// 另外两个都为真的情况（人在国内挂了代理，或者出差在外仍按国内价）。
///
///     is_cn = (语言=简中 且 时区=中国) 或 IP=CN
///
/// 时区那一腿要过 `validated_user_timezone`：客户端同时报时区名和 UTC 偏移，那个函数会
/// 检查「这个时区此刻的真实偏移」和声称的偏移是否一致，所以随手编一个 Asia/Shanghai
/// 但偏移填 -300 会被否掉。
///
/// **这条规则是可以被自己声明的**，而且这是需求本身要求的：把系统语言和时区都设成中国，
/// 就能拿到人民币价（¥100 ≈ US$14，比 $20 便宜三成）。这是「ip不对 那也是中国用户」
/// 这句话的必然代价。为此三个原始信号会原样写进订单行，事后有争议能查。
pub fn buyer_currency(headers: &HeaderMap) -> Buyer {
    let h = |k: &str| headers.get(k).and_then(|v| v.to_str().ok()).map(|s| s.trim().to_string());

    let country = h("cf-ipcountry")
        .map(|s| s.to_ascii_uppercase())
        .filter(|s| s.len() == 2 && s != "XX" && s != "T1");

    // 优先用 IDE/前端显式上报的语言；退回浏览器每个请求都会带的 Accept-Language。
    let language = h("x-ide-language").or_else(|| h("accept-language"));
    let timezone = h("x-ide-timezone");
    let offset_minutes = h("x-ide-utc-offset-minutes").and_then(|s| s.parse::<i32>().ok());

    let lang_cn = language.as_deref().map(language_says_cn).unwrap_or(false);
    let tz_cn = match (timezone.as_deref(), offset_minutes) {
        (Some(name), Some(off)) => {
            crate::prompts::validated_user_timezone(name, off, chrono::Utc::now()).is_some()
                && CN_ZONES.contains(&name.trim().to_ascii_lowercase().as_str())
        }
        _ => false,
    };
    let ip_cn = country.as_deref() == Some("CN");

    let currency = if (lang_cn && tz_cn) || ip_cn { "cny" } else { "usd" };
    Buyer { currency, country, language, timezone, offset_minutes }
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

    // 先定币种，再画卡片。原来这一句在下面 items 组装**之后**才调用，
    // 所以它算出来的答案根本没机会影响任何一张卡的价格。
    let buyer = buyer_currency(&headers);

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
                display_amount(lp, (r.amount_cents, r.amount_usd_cents), buyer.currency);

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

    Ok(Json(json!({
        "enabled": secret_key().is_some(),
        "raw_cents_per_credit_usd": crate::settings::raw_cents_per_credit_usd(),
        "currency": buyer.currency,
        "country": buyer.country,
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
    // Json 必须在最后：它是消费请求体的提取器，顺序错了编译就过不去。
    headers: HeaderMap,
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

    /*
     * 中国区按人民币收，其余按美元。
     *
     * 判定条件用的是**这个价格在 Stripe 上有没有人民币金额**（currency_options），而不是
     * 库里的 amount_cents —— 现在每一行都填了人民币展示价，但日卡和三个加油包在 Stripe 上
     * 并没有人民币价，照着库里的列去指定 cny 会被 Stripe 直接 400，买按钮就死了。
     *
     * live_prices 任何失败都返回空表，所以 Stripe 抖动时会退回美元 —— 往安全的一边退。
     */
    let buyer = buyer_currency(&headers);
    let charge_ccy = if buyer.currency == "cny"
        && live_entry.and_then(|p| p.cny_minor).is_some()
    {
        "cny"
    } else {
        "usd"
    };

    // Form-encoded: Stripe's API does not take JSON.
    let mut form: Vec<(String, String)> = vec![
        ("mode".into(), mode.into()),
        ("line_items[0][price]".into(), price_id),
        ("line_items[0][quantity]".into(), quantity.to_string()),
        (
            "success_url".into(),
            // 复用 /billing：nginx 里 /billing 和 /dashboard 是逐条写死的 location，
            // 新加一个路径要改 nginx，而 nginx 配置来自仓库、手改会被下次部署覆盖。
            // 带上 session id，前端据此渲染支付成功页而不是商品列表。
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
    if charge_ccy == "cny" {
        form.push(("currency".into(), "cny".into()));
    }
    /*
     * 这里曾经打开 adjustable_quantity，让买家在 Stripe 页面上改数量。它必须关掉。
     *
     * 发放走的是 metadata[quantity]（fulfil_session 读它，再乘 unit_credits_cents），而
     * adjustable_quantity 允许买家在**付款前**把数量调下去。于是：下单时填 100000，到
     * Stripe 页面改成 1，付一份的钱，拿到 100000 份的额度。
     *
     * 按当前的每份面额，这是花 $0.15 拿走约 $15,000 的余额；按新价率（每 $1 = 2.5 额度、
     * 每份 1658）是 $25 万。要让它安全，得在履约时改从 line_items 读**实际结算数量**，
     * 那要多一次 API 调用和一条新的失败路径；关掉这三行没有任何新的失败模式，
     * 想买多少在我们自己的页面上填。
     */

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
                "co_{uid}_{}_{quantity}_{mode}_{charge_ccy}_{}",
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
        // 三个原始信号一起落库：定价争议（「为什么我看到的是美元」）只能靠它回答。
        "INSERT INTO orders (user_id, email, price_id, kind, plan, duration_days, credits_cents, \
         amount_cents, method, status, stripe_session_id, quantity, \
         resolved_currency, signal_country, signal_language, signal_timezone, \
         signal_offset_minutes) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'stripe','pending',$9,$10,$11,$12,$13,$14,$15) \
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
    .bind(charge_ccy)
    .bind(&buyer.country)
    .bind(&buyer.language)
    .bind(&buyer.timezone)
    .bind(buyer.offset_minutes)
    .execute(&state.db)
    .await?;

    // 币种回给前端：卡上写的和实际要收的一旦不一致，页面能当场发现。
    Ok(Json(json!({ "url": url, "session_id": session_id, "currency": charge_ccy })))
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
        // BOTH names, because Stripe sends two events for one paid invoice and an endpoint
        // receives only what it is subscribed to. This endpoint's subscription list has
        // `invoice.payment_succeeded` and NOT `invoice.paid` — checked against the live
        // account on 2026-08-11 — so listening for `invoice.paid` alone meant renewals
        // never reached this code at all: no extra month granted, no commission, nothing.
        // No subscription had renewed yet, so nothing was actually lost.
        //
        // Handling both is safe rather than merely tolerable: the two events carry the same
        // invoice, and `fulfil_renewal` claims on the invoice id, so whichever arrives second
        // finds the claim taken and stops. Being subscribed to both is now the harmless case
        // instead of the double-grant it would have been before that claim existed.
        "invoice.paid" | "invoice.payment_succeeded" => {
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
        // The money went back. A refunded or disputed purchase must stop paying its
        // referrer — until this existed, a customer could buy, generate a commission, and
        // be refunded in full while the commission stayed on the books as earned.
        //
        // Entitlement is deliberately NOT revoked here. Refunds are issued for all sorts of
        // reasons, some of them goodwill, and cutting off access on the strength of a
        // webhook is the kind of thing that hurts the wrong person. Commission is the part
        // that is unambiguous: no sale, no commission.
        "charge.refunded" | "charge.dispute.created" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            /*
             * 按退款比例追回，而不是一律整笔作废（规范 7.3：
             * clawbackRatio = refund.amount / order.amount_total）。
             *
             * charge.refunded 在部分退款时也会触发，amount_refunded < amount。之前这里
             * 不看金额，客户退 $10 就把整笔佣金抹掉 —— 少付推荐人。
             *
             * 拒付（dispute）没有部分之说，对象上也没有 amount_refunded，按全额处理。
             */
            let ratio_bps = if event_type == "charge.refunded" {
                let amount = obj.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
                let refunded = obj.get("amount_refunded").and_then(|v| v.as_i64()).unwrap_or(0);
                if amount > 0 && refunded > 0 {
                    ((refunded.min(amount) * 10_000) / amount).clamp(0, 10_000)
                } else {
                    10_000
                }
            } else {
                10_000
            };
            if let Some(order_id) = order_for_reversal(&state, &mut tx, &obj).await? {
                // 先把退款记在订单上。退过款的 Checkout Session 在 Stripe 那边仍然报
                // payment_status: paid，所以没有这个标记，履约和计佣还能再跑一遍。
                let _ = sqlx::query(
                    "UPDATE orders SET refunded_at = COALESCE(refunded_at, now()) WHERE id = $1",
                )
                .bind(order_id)
                .execute(&mut *tx)
                .await;
                crate::referral::reverse(&mut tx, order_id, event_type, ratio_bps).await;
            }
        }
        // Nobody paid, and now nobody can. Closes the order so 「待付款」 means something and
        // the reconciler's queue does not grow forever. The reconciler catches these too, on
        // a ten-minute delay; this is the same job done immediately.
        "checkout.session.expired" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            if let Some(sid) = obj.get("id").and_then(|v| v.as_str()) {
                let _ = sqlx::query(
                    "UPDATE orders SET status = 'canceled' \
                     WHERE stripe_session_id = $1 AND status = 'pending'",
                )
                .bind(sid)
                .execute(&mut *tx)
                .await;
            }
        }
        // A referrer's connected account changed — usually onboarding finishing, sometimes
        // Stripe withdrawing a capability. Nothing is cached from it (readiness is asked
        // fresh at payout time), so this exists to tell the browser to re-read the screen
        // the person is very likely staring at, having just come back from Stripe.
        "account.updated" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            if let Some(acct) = obj.get("id").and_then(|v| v.as_str()) {
                let uid: Option<uuid::Uuid> = sqlx::query_scalar(
                    "SELECT id FROM users WHERE stripe_connect_account_id = $1",
                )
                .bind(acct)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);
                if let Some(uid) = uid {
                    post_commit.push((
                        uid,
                        "connect_updated",
                        json!({
                            "payouts_enabled": obj.get("payouts_enabled").and_then(|v| v.as_bool()),
                            "details_submitted": obj.get("details_submitted").and_then(|v| v.as_bool()),
                        }),
                    ));
                }
            }
        }
        // A payout we sent came back. The referrer never got the money, so it has to return
        // to their withdrawable balance — `withdrawable` excludes 'returned' for exactly
        // this. Told apart from 'failed' on purpose: this one did leave and came back, and
        // "what happened to my payout" deserves the true answer.
        "transfer.reversed" | "transfer.failed" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            if let Some(tr) = obj.get("id").and_then(|v| v.as_str()) {
                /*
                 * 只有整笔冲回才把这条提现放回余额。
                 *
                 * Stripe 的 transfer 可以被部分冲回：一笔 $50 的转账冲回 $10，事件同样是
                 * transfer.reversed。之前这里只看事件名，直接把整行标成 returned，而
                 * withdrawable() 是整行剔除的 —— $50 全部回到可提现余额，用户再提一次，
                 * 于是 $40 被付了两遍。
                 *
                 * `reversed` 是 Stripe 自己给的「是否已全额冲回」，比自己拿
                 * amount_reversed 和金额相比更可靠（金额单位、币种都不用我们再判断一次）。
                 */
                let fully = obj
                    .get("reversed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(event_type == "transfer.failed");
                if !fully {
                    tracing::warn!(
                        transfer = %tr,
                        amount_reversed = ?obj.get("amount_reversed"),
                        "partial transfer reversal — withdrawal left as paid, needs a person"
                    );
                }
                let row: Option<(uuid::Uuid, i64)> = sqlx::query_as(
                    "UPDATE withdrawals SET status = $2, updated_at = now(), \
                         failure_reason = $3 \
                     WHERE transfer_id = $1 AND status = 'paid' AND $4 \
                     RETURNING user_id, amount_cents",
                )
                .bind(tr)
                .bind(if event_type == "transfer.reversed" { "returned" } else { "failed" })
                .bind(event_type)
                .bind(fully)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);
                if let Some((uid, cents)) = row {
                    tracing::warn!(transfer = %tr, %uid, cents, "payout came back");
                    /*
                     * 钱回来了，被这次打款锁走的佣金也必须一起放回去 —— kit 在
                     * PayoutService 里对 FAILED/RETURNED 做的就是这件事。
                     *
                     * 少了这一步，佣金会永远停在 paid：不会被支付（打款已经失败），也不会
                     * 再被下一轮扫到（扫的是 settled）。推荐人的钱凭空消失，而且没有任何报错。
                     */
                    let released: Option<i64> = sqlx::query_scalar(
                        "WITH back AS ( \
                             UPDATE commissions SET status = 'settled', payout_id = NULL, \
                                 updated_at = now() \
                             WHERE payout_id = (SELECT id FROM withdrawals WHERE transfer_id = $1) \
                               AND status = 'paid' \
                             RETURNING 1 \
                         ) SELECT count(*)::bigint FROM back",
                    )
                    .bind(tr)
                    .fetch_optional(&mut *tx)
                    .await
                    .unwrap_or(None);
                    if let Some(n) = released {
                        if n > 0 {
                            tracing::warn!(transfer = %tr, released = n, "commissions released back to settled");
                        }
                    }
                    post_commit.push((
                        uid,
                        "withdrawal_decided",
                        json!({ "status": "returned", "amount_cents": cents, "by": "stripe" }),
                    ));
                }
            }
        }
        // The dispute is over. If we won it, the money stayed with us all along and the
        // commission we reversed on `charge.dispute.created` should not have been.
        "charge.dispute.closed" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            let won = obj.get("status").and_then(|v| v.as_str()) == Some("won");
            if won {
                if let Some(order_id) = order_for_reversal(&state, &mut tx, &obj).await? {
                    let _ = sqlx::query("UPDATE orders SET refunded_at = NULL WHERE id = $1")
                        .bind(order_id)
                        .execute(&mut *tx)
                        .await;
                    crate::referral::unreverse(&mut tx, order_id).await;
                }
            }
        }
        // A renewal charge failed. Deliberately does NOT revoke: Stripe retries on its
        // own schedule for days, and cutting a paying customer off on the first failed
        // attempt would be wrong. Record it so it is visible, and let the terminal
        // events above do the revoking if it never recovers.
        "invoice.payment_failed" => {
            let obj = event.pointer("/data/object").cloned().unwrap_or(json!({}));
            if let Some(sub) = subscription_id_of(&obj) {
                if let Some(uid) = user_for_subscription(&mut tx, &sub).await? {
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
    .bind(&sub)
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
/// Which of our orders a refunded charge or a dispute belongs to.
///
/// Two shapes arrive here. A `charge` carries `invoice` when it came from a subscription
/// renewal, which maps straight onto the order that renewal wrote. Everything else is
/// matched through the payment intent back to the Checkout session, the same route the
/// `payment_intent.succeeded` fallback already uses.
///
/// A dispute object has no `invoice`, so a disputed *renewal* falls through and is logged
/// rather than guessed at. Renewals before the invoice id existed have no `stripe_invoice_id`
/// either and land in the same place. Both are visible in the log rather than silently
/// reversing whatever order happened to be nearest.
/// 返回 `Err` 表示**没查成**（数据库出错、Stripe 不可达）—— 那会让整个 webhook 事务回滚，
/// Stripe 重投，退款下次再处理。返回 `Ok(None)` 才是「查过了，确实不是我们的订单」。
///
/// 以前这里是 `Option`，两种情况长得一模一样：一次网络抖动会被当成「与我无关」，事件被标记
/// 处理完毕并提交，Stripe 收到 200 再也不重投 —— 那笔退款就永远没人追了。
async fn order_for_reversal(
    state: &AppState,
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    obj: &serde_json::Value,
) -> ApiResult<Option<uuid::Uuid>> {
    if let Some(invoice) = obj.get("invoice").and_then(|v| v.as_str()) {
        // `?` 而不是 .ok()：库出错要往上抛，让事务回滚重投。
        let found: Option<(uuid::Uuid,)> =
            sqlx::query_as("SELECT id FROM orders WHERE stripe_invoice_id = $1")
                .bind(invoice)
                .fetch_optional(&mut **tx)
                .await?;
        if let Some((id,)) = found {
            return Ok(Some(id));
        }
    }

    let Some(pi) = obj.get("payment_intent").and_then(|v| v.as_str()) else {
        tracing::warn!("refund/dispute carries no payment_intent; nothing to match");
        return Ok(None);
    };
    let Some(session) = session_for_payment_intent(state, pi).await else {
        // 这里分不出「Stripe 说没有」和「问不到 Stripe」，所以按问不到处理：宁可重投一次。
        return Err(AppError::internal(format!(
            "refund for {pi}: could not resolve the checkout session"
        )));
    };
    let Some(session_id) = session.get("id").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    let found: Option<(uuid::Uuid,)> =
        sqlx::query_as("SELECT id FROM orders WHERE stripe_session_id = $1")
            .bind(session_id)
            .fetch_optional(&mut **tx)
            .await?;
    if found.is_none() {
        tracing::warn!("refund/dispute for {pi} matched no order; commission left as is");
    }
    Ok(found.map(|(id,)| id))
}

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

    // 结账时定下的数量。adjustable_quantity 已经关掉（见 checkout），所以 Stripe 页面上
    // 改不了它 —— metadata 里的这个数就是实际付款的份数。
    //
    // 以前这里的注释写着「metadata 是下限，所以不会多发」，那是反的：买家能往**下**调，
    // 而发放照着 metadata 走，少付多拿。
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
    // `charged_*` is what Stripe says it took. `amount_cents` on this row came from the
    // catalogue and is the CNY shelf price — see migration 20260827.
    let charged = session.get("amount_total").and_then(|v| v.as_i64());
    let charged_ccy = session.get("currency").and_then(|v| v.as_str()).unwrap_or_default();

    let claimed = sqlx::query(
        "UPDATE orders SET status = 'paid', paid_at = now(), stripe_subscription_id = $2, \
             charged_cents = $3, charged_currency = $4 \
         WHERE stripe_session_id = $1 AND status <> 'paid'",
    )
    .bind(session_id)
    .bind(&subscription)
    .bind(charged)
    .bind(charged_ccy)
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
             stripe_subscription_id, quantity, paid_at, charged_cents, charged_currency) \
             VALUES ($1,'',$2,$3,$4,$5,$6,$7,'stripe','paid',$8,$9,$10, now(),$11,$12) \
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
        .bind(charged)
        .bind(charged_ccy)
        .execute(&mut **tx)
        .await?;
        if inserted.rows_affected() == 0 {
            tracing::info!("Stripe session {session_id} already fulfilled; skipping");
            return Ok(None);
        }
    }

    grant(tx, uid, &row, quantity).await?;

    // Whoever referred this buyer, if their window is still open.
    //
    // Inside this transaction on purpose: the `UPDATE … WHERE status <> 'paid'` above is
    // what makes a duplicate webhook harmless, and putting the commission behind the same
    // claim gives it the same exactly-once property for free. `award` never returns an
    // error — a payment must not fail because a referral could not be recorded.
    let paid_order: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM orders WHERE stripe_session_id = $1")
            .bind(session_id)
            .fetch_optional(&mut **tx)
            .await
            .unwrap_or(None);
    crate::referral::award(
        tx,
        uid,
        paid_order,
        commission_basis(session, &row, quantity),
        charged_ccy,
        // Checkout Session 上没有 paid_at；created 是发起结账的时刻，仍然比「webhook 什么时候
        // 送到」接近事实得多。
        session.get("created").and_then(|v| v.as_i64()).unwrap_or(0),
    )
    .await;

    if let Some(cust) = session.get("customer").and_then(|v| v.as_str()) {
        let _ = sqlx::query("UPDATE users SET stripe_customer_id = $1 WHERE id = $2")
            .bind(cust)
            .bind(uid)
            .execute(&mut **tx)
            .await;
    }

    Ok(Some((uid, row.label.clone(), quantity)))
}

/// The subscription an invoice belongs to, across Stripe API versions.
///
/// THIS IS WHY RENEWALS DID NOTHING. `invoice.subscription` was removed in
/// `2025-04-30.basil` and moved under `parent.subscription_details`. This webhook endpoint
/// is pinned to **2026-06-24.dahlia** — checked against the live account — so the old field
/// has not been present on a single invoice Stripe has ever sent here. `fulfil_renewal`
/// read only that field and returned `Ok(())`, which means every renewal was a silent no-op:
/// no order row, no `grant()`, so the customer's plan lapses while their card is still being
/// charged, and no `award()`, so the referrer earns nothing after month one. Stripe got a
/// 200 every time and never retried, and the reconciler only sweeps pending *Checkout*
/// orders, so nothing anywhere would have caught it.
///
/// New shape first, legacy second — the same order the reference kit uses
/// (packages/core/src/commission/events.ts:60-67).
fn subscription_id_of(invoice: &serde_json::Value) -> Option<String> {
    invoice
        .pointer("/parent/subscription_details/subscription")
        .and_then(|v| v.as_str())
        .or_else(|| invoice.get("subscription").and_then(|v| v.as_str()))
        .or_else(|| {
            invoice
                .pointer("/lines/data/0/parent/subscription_item_details/subscription")
                .and_then(|v| v.as_str())
        })
        .map(|s| s.to_string())
}

/// What a commission is a percentage OF: the money Stripe actually collected.
///
/// Not the catalogue price. `prices` carries two figures — `amount_cents`, which is the
/// display price in CNY fen, and `amount_usd_cents` — and the commission used to be taken
/// from the first. For 「Power」 that is 18800, so a 30% commission came out as 5640 and was
/// then rendered, credited and paid as **$56.40**, against a sale that charged US$34.99.
/// Roughly six and a half times too much, in the referrer's favour, silently.
///
/// Nor `amount_usd_cents`: for that same plan the catalogue says 2799 while the live Stripe
/// price charges 3499. Neither local column is the truth. The only figure that is, is the
/// one on the object Stripe just told us it charged.
///
/// 基数 = amount_subtotal 减掉 total_details.amount_discount。
///
/// Stripe 的 `amount_subtotal` 是**折扣前**的合计 —— 这里以前的注释写反了，说它「已经扣过
/// 折扣」。按原样用，一张五折券会按原价付佣金；一张全免券会在实收 $0 的订单上付出真钱。
/// 税不算在内：抽税款的成，等于抽一笔本来就不属于我们的钱。
fn commission_basis(session: &serde_json::Value, row: &CatalogRow, quantity: i64) -> i64 {
    // amount_subtotal 是**折扣前**的合计（Stripe 的定义，和这里以前的注释相反）。
    // 折扣要自己减掉，否则一张五折券会按原价付佣金，一张全免券会在零收入上付真金。
    let discount = session
        .pointer("/total_details/amount_discount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        .max(0);
    session
        .get("amount_subtotal")
        .and_then(|v| v.as_i64())
        .map(|c| (c - discount).max(0))
        .or_else(|| session.get("amount_total").and_then(|v| v.as_i64()))
        .filter(|c| *c > 0)
        // Last resort only. Wrong currency, but a commission recorded low is a bug someone
        // reports; one recorded 6× high is money that leaves before anybody notices.
        .unwrap_or_else(|| row.amount_usd_cents.unwrap_or(row.amount_cents) * quantity)
}

/// A subscription renewed. Extend the plan by another period, and pay the referrer.
///
/// The first invoice of a subscription is skipped: the Checkout session already granted
/// that period, and granting again here would hand out two months for one payment.
///
/// ORDER OF OPERATIONS, and why it changed. This used to grant first and insert the order
/// row afterwards with no unique key, so "have I already handled this invoice" was answered
/// only by the per-event dedupe in `stripe_events`. That covers a redelivery of the same
/// event id and nothing else. The Checkout path does it the other way round — claim, then
/// grant — precisely so a second delivery finds the claim taken. Renewals now do the same,
/// keyed on the invoice id: the INSERT is the claim, and if it conflicts there is nothing
/// left to do. That protects the grant and the commission with one guard.
async fn fulfil_renewal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    invoice: &serde_json::Value,
) -> ApiResult<()> {
    let reason = invoice
        .get("billing_reason")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if reason != "subscription_cycle" {
        // 不放宽这个闸：fulfil_renewal 是按**原始订单**解析商品的，放 subscription_update
        // 进来会照着旧套餐再发一次。但也不能一声不响 —— 在 Stripe 后台改了订阅之后，
        // 客户为什么没拿到东西，只能靠这行日志回答。
        if !reason.is_empty() && reason != "subscription_create" {
            tracing::warn!(
                billing_reason = reason,
                invoice = ?invoice.get("id").and_then(|v| v.as_str()),
                "invoice ignored: only subscription_cycle renews"
            );
        }
        return Ok(());
    }
    let Some(sub) = subscription_id_of(invoice) else {
        // 取不到订阅 id 就什么都做不了，但这必须是响的：这正是让每一次续费空转的那个分支。
        tracing::error!(
            invoice = ?invoice.get("id").and_then(|v| v.as_str()),
            "renewal ignored: no subscription id on the invoice"
        );
        return Ok(());
    };
    let invoice_id = invoice.get("id").and_then(|v| v.as_str()).unwrap_or_default();
    if invoice_id.is_empty() {
        // Without it there is no claim to make, and granting unguarded is how a retry
        // hands out two months. Stripe always sends one; if it did not, do nothing.
        tracing::warn!("Stripe renewal for {sub} carried no invoice id; skipping");
        return Ok(());
    }

    // Find the original order for this subscription; it names the product AND how many
    // seats were bought. The seat count used to be hardcoded to 1 here, so a subscription
    // bought as a 3-seat plan renewed as a 1-seat plan every month after the first.
    let found: Option<(uuid::Uuid, uuid::Uuid, i32)> = sqlx::query_as(
        "SELECT user_id, price_id, quantity FROM orders \
         WHERE stripe_subscription_id = $1 AND user_id IS NOT NULL AND price_id IS NOT NULL \
         ORDER BY created_at LIMIT 1",
    )
    .bind(&sub)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((uid, price_id, quantity)) = found else {
        tracing::warn!("Stripe renewal for unknown subscription {sub}");
        return Ok(());
    };
    let quantity = quantity.max(1) as i64;

    let row = sqlx::query_as::<_, CatalogRow>(
        "SELECT id, label, kind, plan, duration_days, credits_cents, amount_cents, \
         amount_usd_cents, stripe_price_id, lookup_key, recurring, once_per_account, \
         unit_credits_cents, blurb FROM prices WHERE id = $1",
    )
    .bind(price_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else { return Ok(()) };

    // The claim. `RETURNING id` also hands us the order the commission hangs off, so
    // there is no second lookup and no chance of attaching it to the wrong row.
    let claimed: Option<(uuid::Uuid,)> = sqlx::query_as(
        "INSERT INTO orders (user_id, email, price_id, kind, plan, duration_days, \
         credits_cents, amount_cents, method, status, stripe_subscription_id, \
         stripe_invoice_id, quantity, paid_at, charged_cents, charged_currency) \
         VALUES ($1,'',$2,$3,$4,$5,$6,$7,'stripe','paid',$8,$9,$10, now(),$11,$12) \
         ON CONFLICT (stripe_invoice_id) WHERE stripe_invoice_id IS NOT NULL DO NOTHING \
         RETURNING id",
    )
    .bind(uid)
    .bind(row.id)
    .bind(&row.kind)
    .bind(&row.plan)
    .bind(row.duration_days)
    .bind(row.unit_credits_cents.map(|u| u * quantity).or(row.credits_cents))
    .bind(row.amount_cents * quantity)
    .bind(&sub)
    .bind(invoice_id)
    .bind(quantity as i32)
    .bind(invoice.get("amount_paid").and_then(|v| v.as_i64()))
    .bind(invoice.get("currency").and_then(|v| v.as_str()).unwrap_or_default())
    .fetch_optional(&mut **tx)
    .await?;

    let Some((order_id,)) = claimed else {
        tracing::info!("Stripe invoice {invoice_id} already fulfilled; skipping");
        return Ok(());
    };

    grant(tx, uid, &row, quantity).await?;

    // The referrer gets paid for renewals inside the window, not just the first month.
    //
    // Basis is what Stripe actually collected for the goods: `subtotal` is after discounts
    // and before tax, which is the number a percentage of revenue should be taken from —
    // paying commission on sales tax would be paying it on money that was never ours. Falls
    // back to the catalogue price if the invoice does not carry one.
    // Same rule as the Checkout path — see `commission_basis`. An invoice calls its
    // pre-tax, post-discount figure `subtotal` where a session calls it `amount_subtotal`.
    // 同上：invoice 的 subtotal 也是发票级折扣之前的数。
    let inv_discount: i64 = invoice
        .get("total_discount_amounts")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|d| d.get("amount").and_then(|v| v.as_i64())).sum())
        .unwrap_or(0)
        .max(0);
    let basis = invoice
        .get("subtotal")
        .and_then(|v| v.as_i64())
        .map(|c| (c - inv_discount).max(0))
        .filter(|c| *c > 0)
        .unwrap_or_else(|| row.amount_usd_cents.unwrap_or(row.amount_cents) * quantity);
    crate::referral::award(
        tx,
        uid,
        Some(order_id),
        basis,
        invoice.get("currency").and_then(|v| v.as_str()).unwrap_or_default(),
        invoice
            .pointer("/status_transitions/paid_at")
            .and_then(|v| v.as_i64())
            .or_else(|| invoice.get("created").and_then(|v| v.as_i64()))
            .unwrap_or(0),
    )
    .await;

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
                src.contains(&format!("\"{event}\" =>"))
                    || src.contains(&format!("\"{event}\" | "))
                    || src.contains(&format!("| \"{event}\"")),
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
        let (_, _, currency, minor) = display_amount(Some(&live), (100, Some(15)), "usd");
        assert_eq!(currency, "cny");
        assert_eq!(minor, Some(400), "must quote Stripe's ¥4, not the stale ¥1");
    }

    /// 买家不是中国区时，有美元价就报美元。中国区的行为另有一组测试。
    #[test]
    fn dollars_win_when_stripe_has_a_usd_amount() {
        let live = LivePrice {
            cny_minor: Some(8800),
            usd_minor: Some(1299),
            currency: "cny".into(),
            ..Default::default()
        };
        let (_, _, currency, minor) = display_amount(Some(&live), (8800, Some(9999)), "usd");
        assert_eq!(currency, "usd");
        assert_eq!(minor, Some(1299), "Stripe's USD, never the stored column");
    }

    #[test]
    fn the_stored_columns_are_used_only_when_stripe_said_nothing() {
        // No key configured, or Stripe unreachable: the shop degrades to its old
        // behaviour rather than emptying itself.
        let (cny, usd, currency, minor) = display_amount(None, (8800, Some(1299)), "usd");
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
        let (_, _, currency, minor) = display_amount(Some(&live), (8800, Some(1299)), "usd");
        assert_eq!(currency, "eur");
        assert_eq!(minor, None, "no EUR amount was parsed, so there is nothing honest to print");
    }

    fn hdr(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    /// 现在中国的 UTC 偏移。validated_user_timezone 会拿声称的偏移和时区真实偏移比对，
    /// 所以测试必须给一个对的数，否则时区那一腿直接不成立。
    fn cn_offset() -> String {
        use chrono::Offset;
        let tz: chrono_tz::Tz = "Asia/Shanghai".parse().unwrap();
        (chrono::Utc::now().with_timezone(&tz).offset().fix().local_minus_utc() / 60).to_string()
    }

    /// 需求原话：「时区语言和时区时间对得上，ip 不对 那也是中国用户」。
    #[test]
    fn language_and_timezone_together_beat_a_foreign_ip() {
        let h = hdr(&[
            ("cf-ipcountry", "US"),
            ("x-ide-language", "zh-CN"),
            ("x-ide-timezone", "Asia/Shanghai"),
            ("x-ide-utc-offset-minutes", &cn_offset()),
        ]);
        assert_eq!(
            buyer_currency(&h).currency,
            "cny",
            "语言和时区都指向中国时，IP 说是美国也仍然按中国区定价",
        );
    }

    /// IP 单独为真也算 —— 人在国内直连，系统却是英文的。
    #[test]
    fn a_chinese_ip_alone_is_enough() {
        assert_eq!(buyer_currency(&hdr(&[("cf-ipcountry", "CN")])).currency, "cny");
        assert_eq!(
            buyer_currency(&hdr(&[("cf-ipcountry", "cn")])).currency,
            "cny",
            "大小写不能影响判定",
        );
    }

    /// 只满足一条不算。否则一个在上海出差的美国用户会拿到人民币价。
    #[test]
    fn one_signal_alone_is_not_enough() {
        assert_eq!(
            buyer_currency(&hdr(&[
                ("cf-ipcountry", "US"),
                ("x-ide-timezone", "Asia/Shanghai"),
                ("x-ide-utc-offset-minutes", &cn_offset()),
            ]))
            .currency,
            "usd",
            "只有时区在中国、语言不是简中 —— 不算",
        );
        assert_eq!(
            buyer_currency(&hdr(&[
                ("cf-ipcountry", "US"),
                ("x-ide-language", "zh-CN"),
                ("x-ide-timezone", "America/New_York"),
                ("x-ide-utc-offset-minutes", "-300"),
            ]))
            .currency,
            "usd",
            "只有语言是简中、时区在纽约 —— 不算",
        );
    }

    /// 时区名和声称的偏移必须自洽，随手编一个挡不住。
    #[test]
    fn a_timezone_whose_offset_does_not_match_is_rejected() {
        assert_eq!(
            buyer_currency(&hdr(&[
                ("x-ide-language", "zh-CN"),
                ("x-ide-timezone", "Asia/Shanghai"),
                ("x-ide-utc-offset-minutes", "-300"),
            ]))
            .currency,
            "usd",
            "声称在上海却报纽约的偏移 —— 这一腿不成立",
        );
    }

    /// 港澳台不按大陆价。
    #[test]
    fn traditional_chinese_is_not_mainland() {
        assert_eq!(
            buyer_currency(&hdr(&[
                ("x-ide-language", "zh-TW"),
                ("x-ide-timezone", "Asia/Shanghai"),
                ("x-ide-utc-offset-minutes", &cn_offset()),
            ]))
            .currency,
            "usd",
        );
    }

    /// 浏览器自带的 Accept-Language 也要认 —— 网页端不会主动发 x-ide-language。
    #[test]
    fn accept_language_is_a_valid_source() {
        let h = hdr(&[
            ("accept-language", "zh-CN,zh;q=0.9,en;q=0.8"),
            ("x-ide-timezone", "Asia/Shanghai"),
            ("x-ide-utc-offset-minutes", &cn_offset()),
        ]);
        assert_eq!(buyer_currency(&h).currency, "cny");
    }

    /// 中国买家看没有人民币价的商品时，卡上必须显示美元 —— 那才是实际会收的币种。
    #[test]
    fn a_cny_buyer_sees_usd_when_the_price_has_no_cny_amount() {
        let live = LivePrice {
            cny_minor: None,
            usd_minor: Some(2000),
            currency: "usd".into(),
            ..Default::default()
        };
        let (_, _, currency, minor) = display_amount(Some(&live), (14200, Some(2000)), "cny");
        assert_eq!(currency, "usd");
        assert_eq!(minor, Some(2000), "卡上和结账页必须一致，否则就是一笔拒付");
    }

    /// 有人民币价时，中国买家看到人民币。
    #[test]
    fn a_cny_buyer_sees_cny_when_stripe_carries_it() {
        let live = LivePrice {
            cny_minor: Some(29500),
            usd_minor: Some(6000),
            currency: "usd".into(),
            ..Default::default()
        };
        let (_, _, currency, minor) = display_amount(Some(&live), (29500, Some(6000)), "cny");
        assert_eq!(currency, "cny");
        assert_eq!(minor, Some(29500));
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

#[cfg(test)]
mod renewal_and_refund_tests {
    /// Renewals inside the window must pay the referrer.
    ///
    /// This was the whole defect: `award` was reachable only from the Checkout path, so a
    /// programme advertising a rate "for three months" paid on month 1 and nothing after.
    #[test]
    fn a_renewal_pays_the_referrer() {
        let src = include_str!("stripe.rs");
        let f = src
            .split("async fn fulfil_renewal(")
            .nth(1)
            .expect("the renewal path must exist");
        let body = &f[..f.find("\n/// ").unwrap_or(f.len())];
        assert!(
            body.contains("crate::referral::award("),
            "a renewal must award commission — paying only on the first invoice breaks the \
             terms the programme advertises",
        );
        assert!(
            body.contains("subtotal"),
            "commission must be taken from the invoice subtotal: after discounts, before \
             tax. A percentage of sales tax is a percentage of money that was never ours.",
        );
    }

    /// The claim comes first, and it is the invoice.
    ///
    /// The renewal path used to grant and then insert an unkeyed order row, leaning entirely
    /// on per-event dedupe. One redelivery with a new event id — or a retry after a partial
    /// failure — was two months of plan for one payment, and now would also be two
    /// commissions.
    #[test]
    fn a_redelivered_invoice_cannot_grant_or_pay_twice() {
        let src = include_str!("stripe.rs");
        let f = src
            .split("async fn fulfil_renewal(")
            .nth(1)
            .expect("renewal");
        let body = &f[..f.find("\n/// ").unwrap_or(f.len())];

        let claim = body
            .find("ON CONFLICT (stripe_invoice_id)")
            .expect("the renewal order must be claimed on the invoice id");
        let grant = body.find("grant(tx,").expect("a renewal must grant");
        let award = body.find("crate::referral::award(").expect("a renewal must award");
        assert!(
            claim < grant && claim < award,
            "the claim must come BEFORE the grant and the award, or a redelivery does both \
             again before finding out it was already handled",
        );
        assert!(
            body.contains("tracing::info!(\"Stripe invoice {invoice_id} already fulfilled"),
            "a losing claim must return, not carry on",
        );
    }

    /// Seats bought once are seats renewed.
    #[test]
    fn a_renewal_keeps_the_seat_count() {
        let src = include_str!("stripe.rs");
        let f = src.split("async fn fulfil_renewal(").nth(1).expect("renewal");
        let body = &f[..f.find("\n/// ").unwrap_or(f.len())];
        assert!(
            body.contains("SELECT user_id, price_id, quantity FROM orders"),
            "the renewal must read the seat count off the original order",
        );
        assert!(
            !body.contains("grant(tx, uid, &row, 1)"),
            "the seat count was hardcoded to 1: a 3-seat subscription renewed as 1 seat \
             every month after the first",
        );
    }

    /// Refunds and disputes have to reach the ledger.
    #[test]
    fn a_refund_reaches_the_commission_ledger() {
        let src = include_str!("stripe.rs");
        for event in ["charge.refunded", "charge.dispute.created"] {
            assert!(
                src.contains(&format!("\"{event}\"")),
                "no handler for {event} — a refunded sale would keep paying commission",
            );
        }
        let arm = src
            .split("\"charge.refunded\" | \"charge.dispute.created\" => {")
            .nth(1)
            .expect("the refund arm");
        assert!(
            arm[..arm.find("\"checkout.session.expired\"").unwrap_or(arm.len())]
                .contains("crate::referral::reverse("),
            "the refund arm must reverse the commission",
        );
        assert!(
            arm[..arm.find("\"checkout.session.expired\"").unwrap_or(arm.len())]
                .contains("ratio_bps"),
            "clawback must be pro-rata (spec 7.3): a $10 refund on a $100 sale takes back \
             a tenth of the commission, not all of it",
        );
        // Entitlement is deliberately untouched here; see the comment on the arm.
        let refund_arm = &arm[..arm.find("\"checkout.session.expired\"").unwrap_or(arm.len())];
        assert!(
            !refund_arm.contains("end_subscription") && !refund_arm.contains("apply_plan"),
            "a refund must not revoke access as a side effect — that is a separate decision",
        );
    }
}

#[cfg(test)]
mod renewal_event_name_tests {
    /// Renewals must be listened for under BOTH names Stripe uses.
    ///
    /// A paid invoice produces `invoice.paid` AND `invoice.payment_succeeded`, and an endpoint
    /// only receives the ones it subscribes to. The live endpoint had the second and not the
    /// first, so a handler matching `invoice.paid` alone was never going to run: no renewed
    /// month, no commission, and no error anywhere to notice it by.
    #[test]
    fn renewals_listen_for_both_names_stripe_uses() {
        let src = include_str!("stripe.rs");
        let arm = src
            .find("\"invoice.paid\" | \"invoice.payment_succeeded\" =>")
            .or_else(|| src.find("\"invoice.payment_succeeded\" | \"invoice.paid\" =>"))
            .expect(
                "renewals must match both invoice.paid and invoice.payment_succeeded — an \
                 endpoint receives only what it subscribes to, and subscribing to the other \
                 one silently disables renewals entirely",
            );
        assert!(
            src[arm..arm + 300].contains("fulfil_renewal"),
            "both names must reach the renewal path",
        );
    }
}

#[cfg(test)]
mod commission_basis_tests {
    use super::*;

    fn row() -> CatalogRow {
        // 「Power」 as it really sits in the catalogue: ¥188.00 display, US$27.99 recorded,
        // and the live Stripe price charging US$34.99 — three different numbers.
        CatalogRow {
            id: uuid::Uuid::nil(),
            label: "Power".into(),
            kind: "plan".into(),
            plan: Some("pro".into()),
            duration_days: Some(30),
            credits_cents: None,
            amount_cents: 18800,
            amount_usd_cents: Some(2799),
            stripe_price_id: None,
            lookup_key: Some("power_monthly".into()),
            recurring: true,
            once_per_account: false,
            unit_credits_cents: None,
            blurb: String::new(),
        }
    }

    /// The percentage comes off what Stripe charged, not off the CNY shelf price.
    #[test]
    fn commission_follows_the_money_stripe_actually_took() {
        let session = serde_json::json!({ "amount_subtotal": 3499, "amount_total": 3499 });
        assert_eq!(
            commission_basis(&session, &row(), 1),
            3499,
            "must use the session's own figure — 18800 is CNY fen and would pay a 30% \
             commission of $56.40 on a $34.99 sale",
        );
    }

    /// Discounts count, tax does not.
    #[test]
    fn commission_is_taken_before_tax_and_after_discount() {
        let session = serde_json::json!({ "amount_subtotal": 3000, "amount_total": 3600 });
        assert_eq!(
            commission_basis(&session, &row(), 1),
            3000,
            "subtotal wins: a share of sales tax is a share of money that was never ours",
        );
    }

    /// With nothing from Stripe to go on, fall back to the USD column, never the CNY one.
    #[test]
    fn the_fallback_is_never_the_cny_price() {
        let bare = serde_json::json!({});
        assert_eq!(commission_basis(&bare, &row(), 1), 2799);
        assert_eq!(commission_basis(&bare, &row(), 3), 2799 * 3, "seats still multiply");
        assert_ne!(
            commission_basis(&bare, &row(), 1),
            18800,
            "falling back to amount_cents pays a commission in the wrong currency",
        );
    }
}

// ---------------------------------------------------------------------------------------
// The backstop: payments the webhook never told us about
// ---------------------------------------------------------------------------------------

/// How often to sweep. Long enough that Stripe's own retry schedule (which runs for up to
/// three days) usually wins first, short enough that a customer who paid and got nothing is
/// waiting minutes rather than discovering it themselves.
const RECONCILE_EVERY: Duration = Duration::from_secs(10 * 60);

/// Reconcile payments Stripe accepted but this service never heard about.
///
/// WHY THIS EXISTS. Every grant in this system hangs off one webhook delivery. That is a
/// single point of failure with no second chance: if `checkout.session.completed` is not
/// delivered — the endpoint was down through a deploy, the event type was not on the
/// subscription list, Stripe exhausted its retries, a 500 escaped the handler — then the
/// customer has paid, the order sits `pending` forever, and nothing in the system will ever
/// notice. There is no error to see, because from here it looks exactly like a checkout
/// nobody completed. That is the worst shape a payment bug can take.
///
/// Stripe is the source of truth, so reconciling is just asking it. Every pending order
/// with a session id gets checked against the session itself: paid means fulfil, expired
/// means close it out, anything else means leave it alone and look again next sweep.
///
/// SAFE TO RACE with a late webhook. `fulfil_session` claims the order with
/// `UPDATE … WHERE status <> 'paid'` before granting anything, so whichever of the two gets
/// there first does the work and the other finds nothing to claim. The commission hangs off
/// the same claim, so it cannot double-pay either.
pub fn spawn_reconciler(state: AppState) {
    tokio::spawn(async move {
        // Let the service finish starting before adding outbound traffic, and stagger away
        // from the health prober so a restart does not fire everything at once.
        tokio::time::sleep(Duration::from_secs(45)).await;
        let mut tick = tokio::time::interval(RECONCILE_EVERY);
        loop {
            tick.tick().await;
            if let Err(err) = reconcile_once(&state).await {
                // Never fatal. The next sweep is ten minutes away and a database or network
                // blip must not take the backstop down for the life of the process.
                tracing::warn!(%err, "stripe reconcile sweep failed");
            }
        }
    });
}

async fn reconcile_once(state: &AppState) -> anyhow::Result<()> {
    let Some(key) = secret_key() else { return Ok(()) };

    // Seven days back: Stripe sessions expire after 24h, so anything older is settled one
    // way or the other and re-asking every ten minutes forever would be pure noise. The cap
    // keeps one bad sweep from making a hundred API calls.
    let pending: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, stripe_session_id FROM orders \
         WHERE status = 'pending' AND stripe_session_id IS NOT NULL \
           AND created_at > now() - interval '7 days' \
         ORDER BY created_at LIMIT 50",
    )
    .fetch_all(&state.db)
    .await?;

    if pending.is_empty() {
        return Ok(());
    }

    let mut rescued = 0usize;
    let mut expired = 0usize;

    for (order_id, session_id) in pending {
        let res = state
            .update_http
            .get(format!("{STRIPE_API}/checkout/sessions/{session_id}"))
            .bearer_auth(&key)
            .send()
            .await;
        let Ok(res) = res else { continue };
        if !res.status().is_success() {
            continue;
        }
        let Ok(session) = res.json::<serde_json::Value>().await else { continue };

        let paid = session
            .get("payment_status")
            .and_then(|v| v.as_str())
            .map(|s| s == "paid" || s == "no_payment_required")
            .unwrap_or(false);

        if paid {
            let mut tx = state.db.begin().await?;
            match fulfil_session(&mut tx, &session).await {
                Ok(Some((uid, label, quantity))) => {
                    tx.commit().await?;
                    rescued += 1;
                    tracing::warn!(
                        order = %order_id, session = %session_id, %label,
                        "reconciler fulfilled a paid order the webhook never delivered"
                    );
                    crate::realtime::record_event(
                        state,
                        Some(uid),
                        "order_paid",
                        json!({ "via": "stripe-reconcile", "product": label, "quantity": quantity }),
                    )
                    .await;
                }
                // Already fulfilled between the query and now — the webhook won the race,
                // which is the normal case and not worth a line in the log.
                Ok(None) => {
                    tx.rollback().await.ok();
                }
                Err(err) => {
                    tx.rollback().await.ok();
                    tracing::warn!(order = %order_id, err = %err.msg, "reconciler could not fulfil");
                }
            }
        } else if session.get("status").and_then(|v| v.as_str()) == Some("expired") {
            // Nobody paid and nobody can now. Close it so the queue does not grow forever
            // and so 「待付款」 on the billing screen means something.
            let done = sqlx::query(
                "UPDATE orders SET status = 'canceled' WHERE id = $1 AND status = 'pending'",
            )
            .bind(order_id)
            .execute(&state.db)
            .await;
            if matches!(&done, Ok(r) if r.rows_affected() > 0) {
                expired += 1;
            }
        }
    }

    if rescued > 0 || expired > 0 {
        tracing::info!(rescued, expired, "stripe reconcile sweep");
    }
    Ok(())
}

#[cfg(test)]
mod reconcile_tests {
    /// A payment must survive the webhook not arriving.
    ///
    /// Before this, every grant hung off one delivery of one event. A missed delivery —
    /// endpoint down through a deploy, event type absent from the subscription, Stripe out
    /// of retries — meant the customer paid, the order stayed `pending`, and nothing ever
    /// looked again. It is indistinguishable from an abandoned checkout, so nobody finds it.
    #[test]
    fn a_missed_webhook_is_recovered_not_lost() {
        let src = include_str!("stripe.rs");
        let f = src
            .split("async fn reconcile_once(")
            .nth(1)
            .expect("the backstop must exist");
        let body = &f[..f.find("\n#[cfg(test)]").unwrap_or(f.len())];

        assert!(
            body.contains("status = 'pending'") && body.contains("stripe_session_id IS NOT NULL"),
            "the sweep must look for orders that were started and never confirmed",
        );
        assert!(
            body.contains("fulfil_session(&mut tx, &session)"),
            "recovery must go through the SAME fulfilment path as the webhook — a second \
             implementation is a second set of bugs, and it would not share the claim",
        );
        assert!(
            body.contains("payment_status") && body.contains("no_payment_required"),
            "only fulfil what Stripe says was actually paid",
        );
    }

    /// The backstop must not become a second way to pay a commission or grant a plan twice.
    #[test]
    fn reconciling_cannot_double_grant() {
        let src = include_str!("stripe.rs");
        // fulfil_session claims before granting; that claim is what makes the race safe.
        let f = src.split("async fn fulfil_session(").nth(1).expect("fulfil_session");
        let claim = f.find("WHERE stripe_session_id = $1 AND status <> 'paid'")
            .expect("the claim must still be conditional on not-already-paid");
        let grant = f.find("grant(tx,").expect("grant");
        assert!(
            claim < grant,
            "the claim must precede the grant, or the reconciler and a late webhook can \
             both grant before either discovers the other",
        );

        let r = src.split("async fn reconcile_once(").nth(1).expect("reconcile");
        let body = &r[..r.find("\n#[cfg(test)]").unwrap_or(r.len())];
        assert!(
            body.contains("Ok(None) =>"),
            "losing the race must be handled quietly, not treated as a failure",
        );
        assert!(
            body.contains("tx.rollback()"),
            "a transaction that fulfilled nothing must not be committed",
        );
    }

    /// Recording a payment must record the payment, not the shelf price.
    #[test]
    fn a_paid_order_records_what_stripe_charged() {
        let src = include_str!("stripe.rs");
        let f = src.split("async fn fulfil_session(").nth(1).expect("fulfil_session");
        let body = &f[..f.find("\n/// ").unwrap_or(f.len())];
        assert!(
            body.contains("charged_cents = $3") && body.contains("charged_currency = $4"),
            "the claim must write what Stripe actually took — orders.amount_cents is the \
             CNY shelf price and reporting it as USD overstates revenue five-fold",
        );
        assert!(
            body.contains(r#"session.get("amount_total")"#),
            "the amount must come off the Stripe session, not the catalogue row",
        );
    }
}

#[cfg(test)]
mod payout_callback_tests {
    /// Every event that changes money or entitlement must have a home.
    ///
    /// The list is the point: an event Stripe sends and this service ignores is a state
    /// change that happened in the world and not here. `invoice.paid` was exactly that for
    /// months, under a name nobody checked against the subscription.
    #[test]
    fn the_events_that_matter_all_have_handlers() {
        let src = include_str!("stripe.rs");
        for event in [
            "checkout.session.completed",
            "checkout.session.expired",
            "payment_intent.succeeded",
            "invoice.paid",
            "invoice.payment_succeeded",
            "invoice.payment_failed",
            "customer.subscription.deleted",
            "customer.subscription.updated",
            "charge.refunded",
            "charge.dispute.created",
            "charge.dispute.closed",
            "transfer.reversed",
            "transfer.failed",
            "account.updated",
        ] {
            let quoted = format!("\"{event}\"");
            assert!(
                src.contains(&format!("{quoted} =>"))
                    || src.contains(&format!("{quoted} | "))
                    || src.contains(&format!("| {quoted}")),
                "no handler for {event}",
            );
        }
    }

    /// A payout that comes back must return to the balance it came out of.
    #[test]
    fn a_reversed_payout_gives_the_money_back() {
        let src = include_str!("stripe.rs");
        let arm = src
            .split("\"transfer.reversed\" | \"transfer.failed\" => {")
            .nth(1)
            .expect("the reversal arm");
        // 按下一个分支切，不要按固定字节数：这一段注释是中文，1400 字节远不到 1400 个字。
        let body = &arm[..arm.find("\"charge.dispute.closed\"").unwrap_or(arm.len())];
        assert!(
            body.contains("WHERE transfer_id = $1 AND status = 'paid' AND $4"),
            "only a payout recorded as paid can come back, found by the transfer id — and \
             the $4 gate is what keeps a PARTIAL reversal from releasing the whole row",
        );
        assert!(
            body.contains("let fully = obj") && body.contains(r#".get("reversed")"#),
            "a partial reversal must be told apart from a full one, or a $50 transfer \
             reversed by $10 gives the referrer the whole $50 back to withdraw again",
        );
        assert!(
            body.contains("returned") && body.contains("failed"),
            "'left and came back' and 'never left' are different answers to \
             『我的钱呢』 and must not be collapsed",
        );

        // The accounting half: withdrawable has to exclude both, or the money stays locked.
        let r = include_str!("referral.rs");
        let w = r.split("async fn withdrawable").nth(1).expect("withdrawable");
        assert!(
            w[..w.find("\n#[derive").unwrap_or(w.len())]
                .contains("status NOT IN ('rejected', 'failed', 'returned')"),
            "a failed or returned payout must stop counting against the balance",
        );
    }

    /// Winning a dispute must undo the reversal that opening it caused.
    #[test]
    fn a_won_dispute_restores_the_commission() {
        let src = include_str!("stripe.rs");
        let arm = src
            .split("\"charge.dispute.closed\" => {")
            .nth(1)
            .expect("the dispute-closed arm");
        let body = &arm[..arm.find("\"invoice.payment_failed\"").unwrap_or(arm.len())];
        assert!(
            body.contains(r#"== Some("won")"#),
            "only a WON dispute restores anything — a lost one is a real refund",
        );
        assert!(
            body.contains("referral::unreverse"),
            "the commission reversed when the dispute opened must be undone",
        );
    }
}

#[cfg(test)]
mod refund_marker_tests {
    /// 「查不到」和「没有」必须分开。
    ///
    /// 以前两者都返回 None：一次数据库抖动或者 Stripe 不可达，会被当成「这笔退款与我们无关」，
    /// 事件被记为处理完毕并提交，Stripe 收到 200 再也不重投 —— 那笔退款就永远没人追了。
    #[test]
    fn a_failed_lookup_is_not_mistaken_for_no_match() {
        let src = include_str!("stripe.rs");
        let f = src.split("async fn order_for_reversal(").nth(1).expect("fn");
        let sig = &f[..f.find(" {").unwrap_or(200)];
        assert!(
            sig.contains("ApiResult<Option<uuid::Uuid>>"),
            "查不到要能往上抛，让整个 webhook 事务回滚重投",
        );
        let body = &f[..f.find("\nasync fn session_for_payment_intent").unwrap_or(f.len())];
        assert!(
            !body.contains(".ok()\n                .flatten()"),
            "数据库错误不能再被 .ok() 吞掉",
        );
    }

    /// 退款要记在订单上，否则退过款的订单还能再被履约一次。
    #[test]
    fn a_refunded_order_is_marked_on_the_order() {
        let src = include_str!("stripe.rs");
        assert!(
            src.contains("UPDATE orders SET refunded_at = COALESCE(refunded_at, now())"),
            "退过款的 Checkout Session 在 Stripe 那边仍然报 payment_status: paid —— \
             没有这个标记，履约和计佣还能再跑一遍",
        );
        assert!(
            src.contains("UPDATE orders SET refunded_at = NULL"),
            "拒付打赢了要把标记清掉：钱其实从来没走",
        );
    }
}

/// `GET /api/billing/session/:id` — 这一笔到底买到了什么。
///
/// 支付成功页用它。有两件事让它不能只是一次简单的查询：
///
/// **一、跳回来的那一刻，webhook 很可能还没到。** Stripe 先把浏览器重定向回来，webhook 是
/// 另一条链路，晚几百毫秒到几秒都正常。如果这里只读订单，用户会看到「未支付」——他刚付完钱。
/// 所以订单还挂着的时候，这里直接去问 Stripe：真付了就当场走**和 webhook 完全相同**的履约
/// 路径。`fulfil_session` 先认领后发放，所以和 webhook 抢也不会发两次。
///
/// **二、只能看自己的。** session id 出现在跳转地址里，是会被复制、被分享的。查询按
/// `user_id = 调用者` 过滤，拿到别人的 id 也只会得到 404。
pub async fn session_result(
    State(state): State<AppState>,
    claims: Claims,
    axum::extract::Path(sid): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let found: Option<(String, String, Option<String>, Option<i32>, Option<i64>, i64, Option<i64>, Option<String>, Option<uuid::Uuid>)> =
        sqlx::query_as(
            "SELECT status, kind, plan, duration_days, credits_cents, amount_cents, \
                    charged_cents, charged_currency, price_id \
             FROM orders WHERE stripe_session_id = $1 AND user_id = $2",
        )
        .bind(&sid)
        .bind(uid)
        .fetch_optional(&state.db)
        .await?;
    let Some((mut status, kind, plan, duration_days, credits, amount_cents, charged, charged_ccy, price_id)) = found
    else {
        return Err(AppError::bad("找不到这笔订单"));
    };

    // 还没到账就去问 Stripe。付了就当场发，用户不用等 webhook。
    if status != "paid" {
        if let Some(key) = secret_key() {
            let res = state
                .update_http
                .get(format!("{STRIPE_API}/checkout/sessions/{sid}"))
                .bearer_auth(&key)
                .send()
                .await;
            if let Ok(res) = res {
                if res.status().is_success() {
                    if let Ok(session) = res.json::<serde_json::Value>().await {
                        let paid = session
                            .get("payment_status")
                            .and_then(|v| v.as_str())
                            .map(|s| s == "paid" || s == "no_payment_required")
                            .unwrap_or(false);
                        if paid {
                            let mut tx = state.db.begin().await?;
                            match fulfil_session(&mut tx, &session).await {
                                Ok(Some(_)) => {
                                    tx.commit().await?;
                                    status = "paid".into();
                                    tracing::info!(session = %sid, "success page fulfilled ahead of the webhook");
                                }
                                // webhook 抢先了，也算成功。
                                Ok(None) => {
                                    tx.rollback().await.ok();
                                    status = "paid".into();
                                }
                                Err(e) => {
                                    tx.rollback().await.ok();
                                    tracing::warn!(session = %sid, err = %e.msg, "success page could not fulfil");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 发放之后的实际状态，从账号本身读 —— 而不是复述订单里写了什么。
    let (user_plan, plan_expires, credits_now, quota_total): (String, Option<chrono::DateTime<chrono::Utc>>, i64, i64) =
        sqlx::query_as(
            "SELECT plan, plan_expires_at, credits_cents, quota_total_cents FROM users WHERE id = $1",
        )
        .bind(uid)
        .fetch_one(&state.db)
        .await?;

    let label: Option<String> = match price_id {
        Some(pid) => sqlx::query_scalar("SELECT label FROM prices WHERE id = $1")
            .bind(pid)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None),
        None => None,
    };

    Ok(Json(json!({
        "paid": status == "paid",
        "kind": kind,
        "label": label,
        "plan": plan,
        "duration_days": duration_days,
        // 这一单买到的额度，和账号当前总余额分开报：用户想看到的是「这次拿到了什么」。
        "credits_cents": credits,
        "amount_cents": amount_cents,
        "charged_cents": charged,
        "charged_currency": charged_ccy,
        "raw_cents_per_credit_usd": crate::settings::raw_cents_per_credit_usd(),
        "account": {
            "plan": user_plan,
            "plan_expires_at": plan_expires,
            "credits_cents": credits_now,
            "quota_total_cents": quota_total,
        },
    })))
}

#[cfg(test)]
mod bind_arity_tests {
    /// 每条 SQL 用到的最大 `$n`，必须有同样多的 `.bind(...)` 跟在后面。
    ///
    /// 这个错在本次改动里犯了第二次：漏一个 `.bind` 编译期毫无提示，跑起来才报
    /// 「bind message supplies 1 parameters, but prepared statement requires 2」，
    /// 而且只在那一条路径被真正走到时才出现 —— 支付成功页正是那种平时没人点的路径。
    /// referral.rs 有一份同样的守卫，这是 stripe.rs 的那份。
    #[test]
    fn every_query_binds_what_its_sql_asks_for() {
        let src = include_str!("stripe.rs");
        let mut checked = 0;
        for (idx, _) in src.match_indices("sqlx::query") {
            let tail = &src[idx..];
            let end = tail.find(".await").unwrap_or(tail.len().min(3000));
            let stmt = &tail[..end];

            // 只看 SQL 字符串字面量本身。整段扫会把注释里的金额（"$100"）当成占位符。
            let Some(q0) = stmt.find('"') else { continue };
            let rest = &stmt[q0 + 1..];
            let mut sql_len = 0usize;
            let bytes = rest.as_bytes();
            while sql_len < bytes.len() {
                if bytes[sql_len] == b'\\' {
                    sql_len += 2;
                    continue;
                }
                if bytes[sql_len] == b'"' {
                    break;
                }
                sql_len += 1;
            }
            let sql = &rest[..sql_len.min(rest.len())];

            let mut max_n = 0usize;
            let b = sql.as_bytes();
            for i in 0..b.len() {
                if b[i] == b'$' {
                    let mut j = i + 1;
                    let mut n = 0usize;
                    while j < b.len() && b[j].is_ascii_digit() {
                        n = n * 10 + (b[j] - b'0') as usize;
                        j += 1;
                    }
                    if j > i + 1 && n <= 40 {
                        max_n = max_n.max(n);
                    }
                }
            }
            if max_n == 0 {
                continue;
            }
            let binds = stmt[q0 + sql_len..].matches(".bind(").count();
            checked += 1;
            assert!(
                binds >= max_n,
                "一条 SQL 用到了 ${max_n}，但只跟了 {binds} 个 .bind —— 漏绑在编译期查不出来，\
                 只有那条路径被走到时才 500。语句片段：\n{}",
                &sql[..sql.len().min(240)],
            );
        }
        assert!(checked > 15, "扫到的语句太少（{checked}），这个断言可能没在检查什么");
    }
}
