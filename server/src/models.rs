use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// Shared, pooled HTTP client for upstream model calls. Building a fresh
/// `reqwest::Client` per request (the old behaviour) forced a brand-new TCP+TLS
/// handshake to the provider on every call — a large chunk of the "feels slow"
/// latency, and it compounds badly for an agent firing many sequential requests.
/// One pooled client keeps connections warm (keep-alive), so only the first call
/// to a host pays the handshake. No global timeout: streamed chat responses are
/// open-ended; only the connect phase is bounded (per-request timeouts are added
/// for the non-streaming calls that need them).
static GW_HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(16)
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

const CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE: u32 = 6;
const CHAT_UPSTREAM_ROUTE_COOLDOWN: Duration = Duration::from_secs(20);

static CHAT_UPSTREAM_ROUTE_COOLDOWNS: LazyLock<Mutex<HashMap<uuid::Uuid, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static I18N_PACK_CACHE: LazyLock<Mutex<HashMap<String, serde_json::Value>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn chat_upstream_retry_base_delay_ms(attempt: u32) -> u64 {
    match attempt {
        0 => 250,
        1 => 650,
        2 => 1_300,
        3 => 2_500,
        _ => 4_000,
    }
}

fn chat_upstream_retry_delay(attempt: u32) -> Duration {
    let jitter_ms = rand::thread_rng().gen_range(0..=175);
    Duration::from_millis(chat_upstream_retry_base_delay_ms(attempt) + jitter_ms)
}

fn route_cooldown_remaining(id: uuid::Uuid, now: Instant) -> Option<Duration> {
    let mut guard = CHAT_UPSTREAM_ROUTE_COOLDOWNS.lock().ok()?;
    match guard.get(&id).copied() {
        Some(until) if until > now => Some(until - now),
        Some(_) => {
            guard.remove(&id);
            None
        }
        None => None,
    }
}

fn mark_route_cooldown(id: uuid::Uuid) {
    if let Ok(mut guard) = CHAT_UPSTREAM_ROUTE_COOLDOWNS.lock() {
        guard.insert(id, Instant::now() + CHAT_UPSTREAM_ROUTE_COOLDOWN);
    }
}

fn chat_upstream_attempt_suffix(route_count: usize, attempts: u32, last_status: u16) -> String {
    if route_count <= 1 {
        format!("（已请求 {attempts} 次；当前只有 1 条同模型线路；最后状态 {last_status}）")
    } else {
        format!("（已请求 {attempts} 次 / {route_count} 条同模型线路；最后状态 {last_status}）")
    }
}

fn safe_upstream_error_excerpt(low: &str) -> String {
    let mut text = low
        .replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for marker in ["sk-", "sk_live_", "sk-proj-"] {
        if let Some(pos) = text.find(marker) {
            let end = (pos + 48).min(text.len());
            text.replace_range(pos..end, "[redacted-key]");
        }
    }
    text.chars().take(220).collect()
}

/// Wrap an upstream byte stream with an IDLE timeout: if the provider (zyz et al.)
/// goes silent mid-response for too long (it occasionally stalls a stream), we
/// gracefully END the stream instead of leaving the IDE frozen forever. The client
/// then hits EOF, finalizes whatever it has, and unblocks — far better than an
/// infinite "跑着跑着卡住" hang. Generic over the byte type so we don't need to name
/// `bytes::Bytes` directly.
#[allow(dead_code)]
fn idle_guarded_stream<B, S>(
    upstream: S,
) -> impl futures_util::Stream<Item = Result<B, std::io::Error>> + Send + 'static
where
    S: futures_util::Stream<Item = reqwest::Result<B>> + Send + 'static,
    B: Send + 'static,
{
    use futures_util::StreamExt;
    // 180s: a "thinking" model can pause far longer than 30s — it reasons silently, or
    // composes a long tool-call argument (a full-file write) the relay forwards in bursts.
    // The old 30s cut those mid-stream (→ truncated tool call → empty write "内容为空").
    // 180s still bounds a truly-hung upstream so the client eventually auto-retries.
    let idle = std::time::Duration::from_secs(180);
    let upstream = Box::pin(upstream);
    futures_util::stream::unfold(upstream, move |mut s| async move {
        match tokio::time::timeout(idle, s.next()).await {
            Ok(Some(Ok(chunk))) => Some((Ok(chunk), s)),
            // upstream finished, errored, or went idle past the timeout → end here
            _ => None,
        }
    })
}

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// Normalize an OpenAI-compatible base URL: ensure it ends with a `/v1` segment
/// (so `https://gateway.example` becomes `https://gateway.example/v1`). If the
/// caller already included `/v1` (or any `/v1/...`), leave it untouched.
fn api_base(base: &str) -> String {
    let b = base.trim().trim_end_matches('/');
    if b.ends_with("/v1") || b.contains("/v1/") {
        b.to_string()
    } else {
        format!("{}/v1", b)
    }
}

#[derive(sqlx::FromRow, Clone)]
pub struct Model {
    pub id: uuid::Uuid,
    pub label: String,
    pub provider: String,
    pub base_url: String,
    pub model_id: Option<String>,
    pub api_key: String,
    pub price_cents: i64,
    pub rate: f64,
    /// USD per 1,000,000 INPUT tokens (real-API unit). 0 = not set → bill the flat `rate`.
    pub input_price: f64,
    /// USD per 1,000,000 OUTPUT tokens. 0 = not set → bill the flat `rate`.
    pub output_price: f64,
    /// Per 1M CACHE-READ tokens (cheap). 0 = not set → fall back to 0.1× input_price.
    pub cache_read_price: f64,
    /// Per 1M CACHE-CREATE/write tokens (premium). 0 = not set → fall back to 1.25× input_price.
    pub cache_create_price: f64,
    /// Optional admin blurb shown in the IDE picker's hover card.
    pub description: String,
    pub active: bool,
    pub sort: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub enabled_models: Vec<String>,
    /// Billing mode: "rate" (token×price×倍率, default) or "per_call" (flat fee/call).
    pub billing_mode: String,
    /// Flat fee per call in cents, used only when billing_mode = "per_call".
    pub per_call_cents: i64,
    /// Friendly display-name overrides: { raw_model_id → label shown in the IDE }.
    /// The IDE still sends the raw id upstream; this only renames the picker entry.
    pub model_names: serde_json::Value,
    /// Per-MODEL price overrides: { raw_model_id → {"in": usd_per_1M, "out": usd_per_1M} }.
    /// When an entry is set (in>0 or out>0) it WINS over the built-in official catalog for
    /// that model; empty → fall back to official, then the connection-level input/output
    /// price. Lets the admin price each enabled model individually. (倍率 still applies on top.)
    pub model_prices: serde_json::Value,
    /// Upstream wire protocol: "anthropic" (native /v1/messages) or "openai" (/chat/completions
    /// compat). When "anthropic", the gateway translates the OpenAI request/response ⇄ Anthropic.
    pub protocol: String,
}

/// Per-MODEL (input, output) USD/1M price override from a connection's model_prices map.
/// Returns (0.0, 0.0) when this model has no override — compute_cost then uses the built-in
/// official price, then the connection-level fallback. Admin per-model prices beat both.
fn model_price_override(model_prices: &serde_json::Value, model_id: &str) -> (f64, f64) {
    match model_prices.get(model_id) {
        Some(p) => (
            p.get("in").and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0),
            p.get("out")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .max(0.0),
        ),
        None => (0.0, 0.0),
    }
}

/// True for any image-GENERATION model (bills PER-IMAGE, not per-token) across vendors:
/// OpenAI gpt-image / DALL·E, Google gemini *image* (gemini-3.1-flash-image-preview),
/// gpt-4o-image, etc. Guarantees image calls never fall through to $0 token billing.
/// Text/vision models never contain these substrings, so it won't misfire on them.
fn is_image_gen_model(model_id: &str) -> bool {
    let m = model_id.to_lowercase();
    m.contains("gpt-image")
        || m.contains("dall-e")
        || m.contains("dall_e")
        || m.contains("-image")
        || m.contains("image-preview")
        || m.contains("image-generation")
}

/// Look up a friendly display name for `mid` in a connection's model_names map,
/// falling back to the raw id when there's no override.
fn display_name_for(model_names: &serde_json::Value, mid: &str) -> String {
    model_names
        .get(mid)
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| mid.to_string())
}

/// Pick the final cost for one successful upstream call based on the connection's
/// billing mode. "per_call" → flat per_call_cents (token-count independent);
/// otherwise → real token billing via compute_cost. Centralized so EVERY billing
/// site (chat stream/non-stream, legacy chat, responses) stays consistent.
#[allow(clippy::too_many_arguments)]
fn resolve_cost(
    billing_mode: &str,
    per_call_cents: i64,
    usage: Option<&serde_json::Value>,
    model_id: &str,
    rate: f64,
    admin_in: f64,
    admin_out: f64,
    cache_read_price: f64,
    cache_create_price: f64,
    model_in: f64,
    model_out: f64,
) -> i64 {
    if billing_mode == "per_call" {
        let c = per_call_cents.max(0);
        tracing::info!("[billing] model={} mode=per_call → {}¢", model_id, c);
        return c;
    }
    compute_cost(
        usage,
        model_id,
        rate,
        admin_in,
        admin_out,
        cache_read_price,
        cache_create_price,
        model_in,
        model_out,
    )
}

fn allowed_ids(m: &Model) -> Vec<String> {
    if !m.enabled_models.is_empty() {
        return m.enabled_models.clone();
    }
    match &m.model_id {
        Some(s) if !s.is_empty() => vec![s.clone()],
        _ => vec![],
    }
}

#[derive(Deserialize)]
pub struct I18nPackReq {
    pub locale: String,
    pub source_locale: Option<String>,
    pub entries: HashMap<String, String>,
}

fn i18n_pack_cache_key(locale: &str, entries: &HashMap<String, String>) -> String {
    let mut pairs: Vec<_> = entries.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    let mut h = std::collections::hash_map::DefaultHasher::new();
    locale.hash(&mut h);
    for (k, v) in pairs {
        k.hash(&mut h);
        v.hash(&mut h);
    }
    format!("{}:{:016x}", locale, h.finish())
}

fn json_object_from_model_text(text: &str) -> Option<serde_json::Value> {
    let mut s = text.trim();
    if s.starts_with("```") {
        if let Some(pos) = s.find('\n') {
            s = &s[pos + 1..];
        }
        if let Some(pos) = s.rfind("```") {
            s = &s[..pos];
        }
    }
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    serde_json::from_str(&s[start..=end]).ok()
}

fn i18n_pack_payload(
    model_id: &str,
    source_locale: &str,
    locale: &str,
    entries: &HashMap<String, String>,
) -> serde_json::Value {
    json!({
        "model": model_id,
        "temperature": 0.1,
        "stream": false,
        "messages": [
            {
                "role": "system",
                "content": "You are a professional software UI localization engine. Return ONLY valid JSON. Translate UI strings accurately and naturally. Preserve placeholders like {name}, {count}, {path}, punctuation that belongs to variables, product names (Michael IDE, Git, MCP, Skills), code identifiers, file paths, shortcuts, and HTML/Markdown markers. Keep keys unchanged. Do not add explanations."
            },
            {
                "role": "user",
                "content": format!(
                    "Translate this Michael IDE UI language pack from {} to locale {}. Return JSON exactly as {{\"translations\":{{\"key\":\"translated text\"}}}}. Entries JSON:\n{}",
                    source_locale,
                    locale,
                    serde_json::to_string(entries).unwrap_or_else(|_| "{}".into())
                )
            }
        ]
    })
}

fn i18n_out_from_raw(
    entries: &HashMap<String, String>,
    raw: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (k, original) in entries {
        if let Some(text) = raw.get(k).and_then(|v| v.as_str()) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.insert(k.clone(), json!(trimmed));
                continue;
            }
        }
        out.insert(k.clone(), json!(original));
    }
    out
}

fn i18n_pack_body(
    locale: &str,
    source_locale: &str,
    translations: serde_json::Map<String, serde_json::Value>,
    source: &str,
) -> serde_json::Value {
    json!({
        "locale": locale,
        "source_locale": source_locale,
        "translations": translations,
        "source": source,
    })
}

async fn i18n_pack_from_model(
    m: &Model,
    model_id: &str,
    source_locale: &str,
    locale: &str,
    entries: &HashMap<String, String>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let payload = i18n_pack_payload(model_id, source_locale, locale, entries);
    let url = format!("{}/chat/completions", api_base(&m.base_url));
    let resp = GW_HTTP
        .post(url)
        .header("Authorization", format!("Bearer {}", m.api_key))
        .json(&payload)
        .timeout(Duration::from_secs(90))
        .send()
        .await
        .map_err(|e| format!("{} / {} 请求失败: {e}", m.label, model_id))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "{} / {} 上游错误 {}: {}",
            m.label,
            model_id,
            status.as_u16(),
            safe_upstream_error_excerpt(&text)
        ));
    }
    let data: serde_json::Value = serde_json::from_str(&text)
        .map_err(|_| format!("{} / {} 返回非 JSON", m.label, model_id))?;
    let content = data
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let parsed = json_object_from_model_text(content)
        .ok_or_else(|| format!("{} / {} 没有返回可解析语言包 JSON", m.label, model_id))?;
    let raw = parsed
        .get("translations")
        .and_then(|v| v.as_object())
        .or_else(|| parsed.as_object())
        .ok_or_else(|| format!("{} / {} 语言包缺少 translations 对象", m.label, model_id))?;
    Ok(i18n_out_from_raw(entries, raw))
}

fn google_translate_locale(locale: &str) -> String {
    match locale.trim().replace('_', "-").as_str() {
        "zh-CN" | "zh-Hans" => "zh-CN".to_string(),
        "zh-TW" | "zh-Hant" => "zh-TW".to_string(),
        other => other.to_string(),
    }
}

fn google_translate_text(data: &serde_json::Value) -> Option<String> {
    let parts = data.get(0)?.as_array()?;
    let mut out = String::new();
    for part in parts {
        if let Some(text) = part.get(0).and_then(|v| v.as_str()) {
            out.push_str(text);
        }
    }
    if out.trim().is_empty() {
        None
    } else {
        Some(out)
    }
}

async fn google_translate_joined(
    source_locale: &str,
    locale: &str,
    joined: &str,
) -> Result<String, String> {
    let resp = GW_HTTP
        .get("https://translate.googleapis.com/translate_a/single")
        .query(&[
            ("client", "gtx"),
            ("sl", source_locale),
            ("tl", locale),
            ("dt", "t"),
            ("q", joined),
        ])
        .timeout(Duration::from_secs(25))
        .send()
        .await
        .map_err(|e| format!("公共翻译请求失败: {e}"))?;
    let status = resp.status();
    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("公共翻译返回非 JSON: {e}"))?;
    if !status.is_success() {
        return Err(format!("公共翻译错误 {}: {}", status.as_u16(), data));
    }
    google_translate_text(&data).ok_or_else(|| "公共翻译没有返回文本".to_string())
}

async fn google_translate_batch(
    source_locale: &str,
    locale: &str,
    texts: &[String],
) -> Result<Vec<String>, String> {
    if texts.is_empty() {
        return Ok(vec![]);
    }
    let marker = "<<<MICHAEL_I18N_SPLIT>>>";
    let joined = texts.join(&format!("\n{marker}\n"));
    let translated = google_translate_joined(source_locale, locale, &joined).await?;
    let parts: Vec<String> = translated
        .split(marker)
        .map(|s| s.trim_matches(['\n', '\r']).trim().to_string())
        .collect();
    if parts.len() == texts.len() {
        return Ok(parts);
    }
    if texts.len() == 1 {
        return Ok(vec![translated.trim().to_string()]);
    }

    let mut one_by_one = Vec::with_capacity(texts.len());
    for text in texts {
        let single = google_translate_joined(source_locale, locale, text).await?;
        let cleaned = single.trim();
        one_by_one.push(if cleaned.is_empty() {
            text.clone()
        } else {
            cleaned.to_string()
        });
    }
    Ok(one_by_one)
}

async fn i18n_pack_from_public_translate(
    source_locale: &str,
    locale: &str,
    entries: &HashMap<String, String>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let tl = google_translate_locale(locale);
    let mut pairs: Vec<(&String, &String)> = entries.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));

    let mut out = serde_json::Map::new();
    let mut batch_keys: Vec<String> = Vec::new();
    let mut batch_texts: Vec<String> = Vec::new();
    let mut batch_len = 0usize;
    for (key, text) in pairs {
        let projected = batch_len + text.len() + 32;
        if !batch_texts.is_empty() && projected > 3200 {
            let translated = google_translate_batch(source_locale, &tl, &batch_texts).await?;
            for (k, v) in batch_keys.drain(..).zip(translated.into_iter()) {
                out.insert(k, json!(if v.trim().is_empty() { "" } else { v.trim() }));
            }
            batch_texts.clear();
            batch_len = 0;
        }
        batch_keys.push(key.clone());
        batch_texts.push(text.clone());
        batch_len += text.len() + 32;
    }
    if !batch_texts.is_empty() {
        let translated = google_translate_batch(source_locale, &tl, &batch_texts).await?;
        for (k, v) in batch_keys.drain(..).zip(translated.into_iter()) {
            out.insert(k, json!(if v.trim().is_empty() { "" } else { v.trim() }));
        }
    }

    Ok(i18n_out_from_raw(entries, &out))
}

/// POST /api/i18n/pack — generate a UI language pack for any BCP-47 locale.
///
/// The IDE ships core packs (zh/en/ja) locally. For every other selected language
/// it posts the English key-value base here; the server asks an active configured
/// model to translate it into the requested locale, caches the result in memory,
/// and returns a plain `{ translations: { key: text } }` object. This gives every
/// language in the picker a real loading path without bundling hundreds of huge
/// hand-maintained JSON files into the desktop app.
pub async fn i18n_pack(
    State(state): State<AppState>,
    Json(req): Json<I18nPackReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let locale = req.locale.trim().replace('_', "-");
    if locale.is_empty() || locale.len() > 32 {
        return Err(AppError::bad("locale 不能为空"));
    }
    let source_locale = req.source_locale.unwrap_or_else(|| "en".to_string());
    let entries: HashMap<String, String> = req
        .entries
        .into_iter()
        .filter(|(k, v)| {
            !k.trim().is_empty() && k.len() <= 96 && !v.trim().is_empty() && v.len() <= 900
        })
        .take(700)
        .collect();
    if entries.is_empty() {
        return Err(AppError::bad("entries 不能为空"));
    }

    let cache_key = i18n_pack_cache_key(&locale, &entries);
    if let Ok(cache) = I18N_PACK_CACHE.lock() {
        if let Some(v) = cache.get(&cache_key) {
            return Ok(Json(v.clone()));
        }
    }

    let models = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true AND api_key <> '' ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;

    let mut failures: Vec<String> = Vec::new();
    for m in &models {
        let mut ids = allowed_ids(m);
        if ids.is_empty() {
            if let Some(id) = &m.model_id {
                if !id.trim().is_empty() {
                    ids.push(id.clone());
                }
            }
        }
        ids.sort();
        ids.dedup();
        if ids.is_empty() {
            failures.push(format!("{} 未配置 model_id", m.label));
            continue;
        }
        for model_id in ids {
            match i18n_pack_from_model(m, &model_id, &source_locale, &locale, &entries).await {
                Ok(out) => {
                    let body =
                        i18n_pack_body(&locale, &source_locale, out, "model_generated_cached");
                    if let Ok(mut cache) = I18N_PACK_CACHE.lock() {
                        cache.insert(cache_key, body.clone());
                    }
                    return Ok(Json(body));
                }
                Err(e) => failures.push(e),
            }
        }
    }

    let out = i18n_pack_from_public_translate(&source_locale, &locale, &entries)
        .await
        .map_err(|e| AppError {
            status: StatusCode::BAD_GATEWAY,
            msg: format!(
                "语言包生成失败；模型线路失败 {} 条，公共翻译也失败: {}",
                failures.len(),
                e
            ),
        })?;
    let body = i18n_pack_body(&locale, &source_locale, out, "public_translate_cached");
    if let Ok(mut cache) = I18N_PACK_CACHE.lock() {
        cache.insert(cache_key, body.clone());
    }
    Ok(Json(body))
}

/// Mask a secret for display: keep the last 4 chars.
fn mask(key: &str) -> String {
    if key.len() <= 4 {
        return "••••".into();
    }
    format!("••••{}", &key[key.len() - 4..])
}

// ---------- admin: list / create / delete ----------
/// GET /api/admin/models — full list for management (api_key masked).
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, Model>("SELECT * FROM models ORDER BY sort, created_at")
        .fetch_all(&state.db)
        .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|m| {
            json!({
                "id": m.id, "label": m.label, "provider": m.provider, "base_url": m.base_url,
                "model_id": m.model_id, "api_key_masked": mask(&m.api_key), "has_key": !m.api_key.is_empty(),
                "price_cents": m.price_cents, "rate": m.rate, "active": m.active, "sort": m.sort, "created_at": m.created_at,
                "input_price": m.input_price, "output_price": m.output_price,
                "cache_read_price": m.cache_read_price, "cache_create_price": m.cache_create_price,
                "description": m.description,
                "enabled_models": m.enabled_models,
                "billing_mode": m.billing_mode, "per_call_cents": m.per_call_cents,
                "model_names": m.model_names,
                "model_prices": m.model_prices,
                "protocol": m.protocol,
            })
        })
        .collect();
    Ok(Json(json!(list)))
}

#[derive(Deserialize)]
pub struct ModelReq {
    pub label: String,
    pub provider: Option<String>,
    pub base_url: String,
    pub model_id: Option<String>,
    pub api_key: String,
    pub rate: Option<f64>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub cache_read_price: Option<f64>,
    pub cache_create_price: Option<f64>,
    pub description: Option<String>,
    pub sort: Option<i32>,
    pub billing_mode: Option<String>,
    pub per_call_cents: Option<i64>,
}

/// POST /api/admin/models — create a provider connection (admin). model_id is
/// optional; the exposed models are chosen later via the edit/enabled set.
pub async fn admin_create(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<ModelReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if req.label.trim().is_empty() || req.base_url.trim().is_empty() {
        return Err(AppError::bad("名称 / baseUrl 不能为空"));
    }
    let bmode = match req.billing_mode.as_deref() {
        Some("per_call") => "per_call",
        _ => "rate",
    };
    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO models (label, provider, base_url, model_id, api_key, rate, input_price, output_price, description, sort, billing_mode, per_call_cents, cache_read_price, cache_create_price) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING id",
    )
    .bind(req.label.trim())
    .bind(req.provider.unwrap_or_default())
    .bind(req.base_url.trim().trim_end_matches('/'))
    .bind(req.model_id.unwrap_or_default().trim())
    .bind(req.api_key.trim())
    .bind(req.rate.unwrap_or(1.0).max(0.0))
    .bind(req.input_price.unwrap_or(0.0).max(0.0))
    .bind(req.output_price.unwrap_or(0.0).max(0.0))
    .bind(req.description.unwrap_or_default().trim())
    .bind(req.sort.unwrap_or(0))
    .bind(bmode)
    .bind(req.per_call_cents.unwrap_or(0).max(0))
    .bind(req.cache_read_price.unwrap_or(0.0).max(0.0))
    .bind(req.cache_create_price.unwrap_or(0.0).max(0.0))
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "ok": true, "id": id })))
}

/// DELETE /api/admin/models/:id (admin).
pub async fn admin_delete(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM models WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("模型不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

/// GET /api/admin/models/:id/available — proxy the provider's model catalogue
/// (OpenAI-compatible GET /models) using this connection's key.
pub async fn admin_available(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let m = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型连接不存在"))?;
    if m.api_key.is_empty() {
        return Err(AppError::bad("该连接未配置 API Key"));
    }
    let url = format!("{}/models", api_base(&m.base_url));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", m.api_key))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("拉取模型列表失败: {e}")))?;
    let status = resp.status();
    let data: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        return Err(AppError {
            status: axum::http::StatusCode::BAD_GATEWAY,
            msg: format!("供应商错误 {}: {}", status.as_u16(), data),
        });
    }
    let ids: Vec<String> = data
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.get("id").and_then(|i| i.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(json!({ "models": ids, "enabled": m.enabled_models })))
}

#[derive(Deserialize)]
pub struct UpdateReq {
    pub label: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>, // empty/missing = keep existing
    pub rate: Option<f64>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub cache_read_price: Option<f64>,
    pub cache_create_price: Option<f64>,
    pub description: Option<String>,
    pub active: Option<bool>,
    pub sort: Option<i32>,
    pub enabled_models: Option<Vec<String>>,
    pub billing_mode: Option<String>,
    pub per_call_cents: Option<i64>,
    /// { raw_model_id → friendly display name }. Replaces the whole map when present.
    pub model_names: Option<serde_json::Value>,
    /// { raw_model_id → {"in", "out"} } per-model price overrides. Replaces the whole map.
    pub model_prices: Option<serde_json::Value>,
    /// "anthropic" | "openai" — upstream wire protocol for this connection.
    pub protocol: Option<String>,
}

/// POST /api/admin/models/:id — update a connection (incl. enabled model set). admin.
pub async fn admin_update(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<UpdateReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let m = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型连接不存在"))?;
    let label = req
        .label
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(m.label);
    let provider = req.provider.unwrap_or(m.provider);
    let base_url = req
        .base_url
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(m.base_url);
    let api_key = match req.api_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => m.api_key,
    };
    let rate = req.rate.unwrap_or(m.rate).max(0.0);
    let input_price = req.input_price.unwrap_or(m.input_price).max(0.0);
    let output_price = req.output_price.unwrap_or(m.output_price).max(0.0);
    let cache_read_price = req.cache_read_price.unwrap_or(m.cache_read_price).max(0.0);
    let cache_create_price = req
        .cache_create_price
        .unwrap_or(m.cache_create_price)
        .max(0.0);
    let description = req
        .description
        .map(|s| s.trim().to_string())
        .unwrap_or(m.description);
    let active = req.active.unwrap_or(m.active);
    let sort = req.sort.unwrap_or(m.sort);
    let enabled = req.enabled_models.unwrap_or(m.enabled_models);
    let billing_mode = match req.billing_mode.as_deref() {
        Some("per_call") => "per_call".to_string(),
        Some("rate") => "rate".to_string(),
        _ => m.billing_mode, // unspecified → keep existing
    };
    let per_call_cents = req.per_call_cents.unwrap_or(m.per_call_cents).max(0);
    // model_names / model_prices: replace the whole map when the client sends one; keep existing otherwise.
    let model_names = req
        .model_names
        .filter(|v| v.is_object())
        .unwrap_or(m.model_names);
    let model_prices = req
        .model_prices
        .filter(|v| v.is_object())
        .unwrap_or(m.model_prices);
    let protocol = match req.protocol.as_deref() {
        Some("openai") => "openai".to_string(),
        Some("anthropic") => "anthropic".to_string(),
        _ => m.protocol, // unspecified → keep existing
    };
    sqlx::query("UPDATE models SET label=$1, provider=$2, base_url=$3, api_key=$4, rate=$5, active=$6, sort=$7, enabled_models=$8, input_price=$9, output_price=$10, description=$11, billing_mode=$12, per_call_cents=$13, model_names=$14, cache_read_price=$15, cache_create_price=$16, model_prices=$17, protocol=$18 WHERE id=$19")
        .bind(&label)
        .bind(&provider)
        .bind(&base_url)
        .bind(&api_key)
        .bind(rate)
        .bind(active)
        .bind(sort)
        .bind(&enabled)
        .bind(input_price)
        .bind(output_price)
        .bind(&description)
        .bind(&billing_mode)
        .bind(per_call_cents)
        .bind(&model_names)
        .bind(cache_read_price)
        .bind(cache_create_price)
        .bind(&model_prices)
        .bind(&protocol)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true })))
}

// ---------- IDE-facing: list active models (safe fields, no secrets) ----------
/// GET /api/models — active models for the IDE (no api_key / base_url leaked).
pub async fn list_for_client(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let mut list = Vec::new();
    for m in &rows {
        for mid in allowed_ids(m) {
            let name = display_name_for(&m.model_names, &mid);
            let (model_in, model_out) = model_price_override(&m.model_prices, &mid);
            let (input_price, output_price, price_source) = if model_in > 0.0 || model_out > 0.0 {
                (model_in, model_out, "model_override")
            } else if m.input_price > 0.0 || m.output_price > 0.0 {
                (m.input_price, m.output_price, "backend")
            } else if let Some((official_in, official_out)) = official_price(&mid) {
                (official_in, official_out, "catalog")
            } else {
                (0.0, 0.0, "unset")
            };
            list.push(json!({
                "conn_id": m.id,
                "group": m.label,
                "provider": m.provider,
                "model_id": mid.clone(),
                "name": name,
                "price_cents": m.price_cents,
                // Expose the display price the admin configured for the IDE picker.
                // No api_key/base_url is leaked; just the model's visible input/output
                // price so the client can show exactly what the backend is using.
                "input_price": input_price,
                "output_price": output_price,
                "price_source": price_source,
                "rate": m.rate,
                "description": m.description,
            }));
        }
    }
    Ok(Json(json!(list)))
}

// ---------- IDE-facing: proxy a chat completion, billing credits ----------
/// POST /api/models/:id/chat — forwards an OpenAI-style chat request to the
/// model's provider, deducts the model's price from the caller's credits, and
/// returns the upstream JSON. Non-streaming.
pub async fn chat(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(mut body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = $1 AND active = true")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::bad("模型不存在或已停用"))?;

    // pre-check: need a positive balance when the model isn't free. per_call mode
    // (with per_call_cents > 0) also requires balance even if rate/io-price are 0.
    let not_free = model.rate > 0.0
        || model.input_price > 0.0
        || model.output_price > 0.0
        || (model.billing_mode == "per_call" && model.per_call_cents > 0);
    if not_free {
        let bal: i64 = sqlx::query_scalar("SELECT credits_cents FROM users WHERE id = $1")
            .bind(uid)
            .fetch_one(&state.db)
            .await?;
        if bal <= 0 {
            return Err(AppError {
                status: axum::http::StatusCode::PAYMENT_REQUIRED,
                msg: "额度不足，请充值".into(),
            });
        }
    }

    // forward to the provider (OpenAI-compatible /chat/completions)
    if !body.is_object() {
        return Err(AppError::bad("请求体需为 JSON 对象"));
    }
    // honour the requested model when it's in this connection's enabled set
    let allowed = allowed_ids(&model);
    let requested = body.get("model").and_then(|v| v.as_str()).map(String::from);
    let chosen = match requested {
        Some(r) if allowed.contains(&r) => r,
        _ => allowed.first().cloned().unwrap_or_default(),
    };
    if chosen.is_empty() {
        return Err(AppError::bad("该连接未开放任何模型，请在后台编辑勾选"));
    }
    body["model"] = json!(chosen);

    // Weak-vision models (deepseek/minimax/glm/…) can't read images well. If the
    // request has images and the chosen model isn't vision-native, let gpt-5.5
    // describe the images first, then hand the text to the chosen model.
    if needs_vision_help(&chosen) {
        vision_preprocess(&state, &mut body).await;
    }

    let url = format!("{}/chat/completions", api_base(&model.base_url));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", model.api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("模型调用失败: {e}")))?;
    let status = resp.status();
    let data: serde_json::Value = resp
        .json()
        .await
        .unwrap_or_else(|_| json!({ "error": "上游返回非 JSON" }));
    if !status.is_success() {
        return Err(AppError {
            status: axum::http::StatusCode::BAD_GATEWAY,
            msg: format!("模型供应商错误 {}: {}", status.as_u16(), data),
        });
    }

    // bill on success: per_call flat fee, or real token usage × official price × 倍率.
    let (model_in, model_out) = model_price_override(&model.model_prices, &chosen);
    let usage_val = data.get("usage");
    let has_real = usage_val.is_some_and(|u| {
        u.get("prompt_tokens").and_then(|x| x.as_i64()).unwrap_or(0) > 0
            || u.get("input_tokens").and_then(|x| x.as_i64()).unwrap_or(0) > 0
    });
    let (effective_usage, is_est) = if has_real {
        (usage_val.cloned().unwrap_or_else(|| json!({})), false)
    } else {
        let est_in = estimate_input_tokens(&body);
        let resp_str = serde_json::to_string(&data).unwrap_or_default();
        let est_out = (resp_str.len() as i64) / 4;
        tracing::warn!(
            "[billing] legacy chat no usage, estimating: in={} out={} model={}",
            est_in,
            est_out,
            chosen
        );
        (estimated_usage(est_in, est_out), true)
    };
    let cost = resolve_cost(
        &model.billing_mode,
        model.per_call_cents,
        Some(&effective_usage),
        &chosen,
        model.rate,
        model.input_price,
        model.output_price,
        model.cache_read_price,
        model.cache_create_price,
        model_in,
        model_out,
    );
    let tokens = extract_bill_tokens(Some(&effective_usage), &chosen, is_est);
    bill(&state, uid, model.id, cost, false, &tokens).await;
    Ok(Json(data))
}

// ---------- admin: usage stats ----------
/// GET /api/admin/model-usage — recent usage + totals (admin).
pub async fn admin_usage(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let calls: i64 = sqlx::query_scalar("SELECT count(*) FROM model_usage")
        .fetch_one(&state.db)
        .await?;
    let spent: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(cost_cents),0)::bigint FROM model_usage")
            .fetch_one(&state.db)
            .await?;
    Ok(Json(json!({ "calls": calls, "spent_cents": spent })))
}

/// GET /api/usage — a logged-in user's own recent usage + current balance.
pub async fn user_usage(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    type UsageRow = (
        i64,
        i64,
        i64,
        i64,
        String,
        bool,
        chrono::DateTime<chrono::Utc>,
    );
    let rows: Vec<UsageRow> =
        sqlx::query_as(
            "SELECT cost_cents, prompt_tokens, completion_tokens, cached_tokens, model_name, estimated, created_at \
             FROM model_usage WHERE user_id = $1 ORDER BY created_at DESC LIMIT 200",
        )
        .bind(uid)
        .fetch_all(&state.db)
        .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            json!({
                "cost_cents": r.0, "prompt_tokens": r.1, "completion_tokens": r.2,
                "cached_tokens": r.3, "model": r.4, "estimated": r.5, "time": r.6,
            })
        })
        .collect();
    let (credits, plan): (i64, String) =
        sqlx::query_as("SELECT credits_cents, plan FROM users WHERE id = $1")
            .bind(uid)
            .fetch_one(&state.db)
            .await?;
    let total_spent: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(cost_cents),0)::bigint FROM model_usage WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({
        "credits_cents": credits,
        "plan": plan,
        "total_spent_cents": total_spent,
        "recent": list,
    })))
}

// ================= Michael API keys + OpenAI-compatible gateway =================

#[derive(sqlx::FromRow)]
struct ApiKeyRow {
    id: uuid::Uuid,
    label: String,
    api_key: String,
    email: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn gen_api_key() -> String {
    let mut rng = rand::thread_rng();
    let hex: String = (0..40)
        .map(|_| std::char::from_digit(rng.gen_range(0..16), 16).unwrap())
        .collect();
    format!("sk-michael-{hex}")
}

fn mask_key(k: &str) -> String {
    if k.len() <= 8 {
        return "••••".into();
    }
    format!("{}…{}", &k[..11.min(k.len())], &k[k.len() - 4..])
}

#[derive(Deserialize)]
pub struct ApiKeyReq {
    pub label: Option<String>,
    pub email: Option<String>,
}

/// POST /api/admin/apikeys — generate a gateway key for the admin (or a given user's email).
pub async fn admin_create_apikey(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<ApiKeyReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let uid = match req
        .email
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        Some(email) => sqlx::query_scalar::<_, uuid::Uuid>("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::bad("用户不存在"))?,
        None => {
            uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?
        }
    };
    let key = gen_api_key();
    sqlx::query("INSERT INTO api_keys (user_id, api_key, label) VALUES ($1,$2,$3)")
        .bind(uid)
        .bind(&key)
        .bind(req.label.unwrap_or_default())
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "ok": true, "api_key": key })))
}

/// GET /api/admin/apikeys — list keys (masked) with their owner email.
pub async fn admin_list_apikeys(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, ApiKeyRow>(
        "SELECT k.id, k.label, k.api_key, u.email, k.created_at, k.last_used_at \
         FROM api_keys k LEFT JOIN users u ON u.id = k.user_id ORDER BY k.created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;
    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| json!({ "id": r.id, "label": r.label, "email": r.email, "key_masked": mask_key(&r.api_key), "created_at": r.created_at, "last_used_at": r.last_used_at }))
        .collect();
    Ok(Json(json!(list)))
}

/// DELETE /api/admin/apikeys/:id
pub async fn admin_delete_apikey(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("密钥不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

/// GET /api/ide-key — return a stable API key bound to THE LOGGED-IN USER (creating
/// it once), so the IDE can auto-configure a per-user key. REQUIRES a valid login JWT
/// (the `Claims` extractor 401s otherwise). This is deliberate: previously this was
/// public and returned the *first admin's* key — anyone could fetch it (full-gateway
/// leak) and every anonymous caller's usage billed the admin. Now each caller gets
/// THEIR OWN key, billed to THEIR account. The desktop IDE already authenticates chat
/// with the login JWT directly; this endpoint is for clients that want a stable key.
pub async fn ide_key(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let existing: Option<String> = sqlx::query_scalar("SELECT api_key FROM api_keys WHERE user_id = $1 AND label = 'ide-auto' ORDER BY created_at LIMIT 1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?;
    let key = match existing {
        Some(k) => k,
        None => {
            let k = gen_api_key();
            sqlx::query(
                "INSERT INTO api_keys (user_id, api_key, label) VALUES ($1, $2, 'ide-auto')",
            )
            .bind(uid)
            .bind(&k)
            .execute(&state.db)
            .await?;
            k
        }
    };
    Ok(Json(json!({ "api_key": key })))
}

/// A model id whose vision is weak/absent → route images through gpt-5.5 first.
fn needs_vision_help(model_id: &str) -> bool {
    let m = model_id.to_lowercase();
    let native = m.contains("gpt")
        || m.contains("gemini")
        || m.contains("claude")
        || m.contains("vision")
        || m.contains("-vl")
        || m.contains("image")
        || m.contains("o3")
        || m.contains("o4");
    !native
}

/// If the request carries images, ask gpt-5.5 to describe them, then rewrite the
/// messages to plain text (description injected) so a non-vision model can work
/// from it. No-op if there are no images or no gpt-5.5 connection is configured.
async fn vision_preprocess(state: &AppState, body: &mut serde_json::Value) {
    let mut images: Vec<serde_json::Value> = Vec::new();
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        for m in msgs {
            if let Some(arr) = m.get("content").and_then(|c| c.as_array()) {
                for part in arr {
                    if part.get("type").and_then(|t| t.as_str()) == Some("image_url") {
                        images.push(part.clone());
                    }
                }
            }
        }
    }
    if images.is_empty() {
        return;
    }
    // best-effort: have gpt-5.5 describe the images (may fail → we still strip them)
    let mut desc: Option<String> = None;
    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    if let Some(vconn) = conns.into_iter().find(|m| {
        allowed_ids(m)
            .iter()
            .any(|id| id.eq_ignore_ascii_case("gpt-5.5"))
    }) {
        let mut vcontent = vec![json!({
            "type": "text",
            "text": "请详细、客观地描述这些图片的全部内容（文字、数据、图表、代码、界面元素、布局、配色等），让一个无法读图的模型也能据此完成工作。只输出描述本身。"
        })];
        vcontent.extend(images.clone());
        let payload = json!({ "model": "gpt-5.5", "messages": [{ "role": "user", "content": vcontent }], "stream": false });
        if let Ok(client) = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
        {
            let url = format!("{}/chat/completions", api_base(&vconn.base_url));
            if let Ok(r) = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", vconn.api_key))
                .json(&payload)
                .send()
                .await
            {
                if let Ok(d) = r.json::<serde_json::Value>().await {
                    if let Some(s) = d["choices"][0]["message"]["content"].as_str() {
                        if !s.trim().is_empty() {
                            desc = Some(s.to_string());
                        }
                    }
                }
            }
        }
    }
    let note = match desc {
        Some(d) => format!("【图片内容（由 GPT-5.5 视觉识别）】：\n{}", d),
        None => "【图片】（视觉识别暂不可用，无法读取图片内容）".to_string(),
    };
    // ALWAYS strip images → plain text so a non-vision model never chokes on them.
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        let mut last_img: Option<usize> = None;
        for (i, m) in msgs.iter_mut().enumerate() {
            if let Some(arr) = m.get("content").and_then(|c| c.as_array()).cloned() {
                let mut text = String::new();
                let mut had = false;
                for part in &arr {
                    match part.get("type").and_then(|t| t.as_str()) {
                        Some("text") => {
                            if let Some(t) = part.get("text").and_then(|x| x.as_str()) {
                                if !text.is_empty() {
                                    text.push('\n');
                                }
                                text.push_str(t);
                            }
                        }
                        Some("image_url") => had = true,
                        _ => {}
                    }
                }
                m["content"] = json!(text);
                if had {
                    last_img = Some(i);
                }
            }
        }
        if let Some(idx) = last_img {
            if let Some(m) = msgs.get_mut(idx) {
                let cur = m
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
                m["content"] = json!(format!("{}\n\n{}", cur, note));
            }
        }
    }
}

/// Cents to bill from an upstream `usage` object, CACHE-AWARE. `rate` is cents per
/// 1000 tokens (same unit as the non-streaming path). Cached (read) input tokens are
/// charged at CACHE_READ_FACTOR of the rate so caching savings reach the user;
/// Anthropic cache-CREATION at CACHE_WRITE_FACTOR. Handles both usage shapes —
/// OpenAI/DeepSeek: `prompt_tokens` INCLUDES cached; Anthropic: `input_tokens`
/// EXCLUDES cached (cache_read/creation reported separately). Returns None when the
/// upstream reported no usable token counts, so the caller falls back to a flat fee.
#[allow(dead_code)] // kept for an optional token-based billing mode (currently flat)
fn cost_from_usage(u: &serde_json::Value, rate: f64) -> Option<i64> {
    const CACHE_READ_FACTOR: f64 = 0.1; // cached reads ~10% of input price
    const CACHE_WRITE_FACTOR: f64 = 1.25; // Anthropic cache creation ~125%
                                          // Sanity ceiling: a malformed/huge upstream usage must never saturate to i64::MAX
                                          // and zero out a user's balance. No single call legitimately costs $10k.
    const COST_CEILING: f64 = 1_000_000.0;
    let completion = u
        .get("completion_tokens")
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("output_tokens").and_then(|v| v.as_f64()));
    let prompt = u
        .get("prompt_tokens")
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("input_tokens").and_then(|v| v.as_f64()));
    let (completion, prompt) = match (completion, prompt) {
        (Some(c), Some(p)) => (c, p),
        // Some providers report only total_tokens — bill that flat (matches the
        // non-streaming path's formula).
        _ => {
            let total = u.get("total_tokens").and_then(|v| v.as_f64())?;
            return Some((total / 1000.0 * rate).round().clamp(0.0, COST_CEILING) as i64);
        }
    };
    let cache_read = u.get("cache_read_input_tokens").and_then(|v| v.as_f64()); // Anthropic
    let cache_creation = u
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let cached = u
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("prompt_cache_hit_tokens").and_then(|v| v.as_f64())) // DeepSeek
        .or(cache_read)
        .unwrap_or(0.0);
    let billable_input = if cache_read.is_some() {
        // Anthropic shape: input_tokens EXCLUDES cached.
        prompt + cached * CACHE_READ_FACTOR + cache_creation * CACHE_WRITE_FACTOR
    } else {
        // OpenAI/DeepSeek shape: prompt_tokens INCLUDES cached.
        (prompt - cached).max(0.0) + cached * CACHE_READ_FACTOR
    };
    Some(
        ((billable_input + completion) / 1000.0 * rate)
            .round()
            .clamp(0.0, COST_CEILING) as i64,
    )
}

/// Official public list prices (USD per 1,000,000 tokens) per model — the REAL cost basis
/// for billing (the default when no per-model override is set). Per-MODEL, not per-connection,
/// because one connection (e.g. the zyz aggregator) exposes many models at different prices.
/// Matched by FAMILY substring so date/`-preview` suffixes still resolve (e.g.
/// `claude-haiku-4-5-20251001`, `gemini-3.1-pro-preview`). Sources: vendor pricing pages, 2026-07
/// (Anthropic prices from the claude-api skill; Gemini/GPT/DeepSeek/MiniMax from vendor pages).
/// Returns (input, output). None → caller falls back to the connection-level price, then 0.
fn official_price(model_id: &str) -> Option<(f64, f64)> {
    let m = model_id.to_lowercase();
    // ---- Anthropic Claude (official list price) ----
    if m.contains("claude") {
        if m.contains("fable-5") || m.contains("mythos-5") {
            return Some((10.0, 50.0));
        }
        if m.contains("opus-4") {
            return Some((5.0, 25.0));
        } // opus 4.6 / 4.7 / 4.8
          // sonnet 5 standard $3/$15 (intro $2/$10 through 2026-08-31; use the durable list price);
          // sonnet 4.6 is also $3/$15.
        if m.contains("sonnet-5") || m.contains("sonnet-4") {
            return Some((3.0, 15.0));
        }
        if m.contains("haiku-4") {
            return Some((1.0, 5.0));
        } // haiku 4.5 (+date suffix)
    }
    // ---- Google Gemini (official list price, standard ≤200K-context tier) ----
    if m.contains("gemini") {
        if m.contains("image") {
            return None;
        } // image gen → billed per-image, not per-token
        if m.contains("3.5-flash") || m.contains("3-5-flash") {
            return Some((1.5, 9.0));
        }
        if m.contains("pro") {
            return Some((2.0, 12.0));
        } // gemini 3.1 pro (-preview)
        if m.contains("flash") {
            return Some((0.5, 3.0));
        } // gemini 3 flash tier
    }
    // ---- Z.ai GLM (official list price, USD/1M, docs.z.ai 2026-07) ----
    if m.contains("glm") {
        // 顺序敏感：更具体的变体（airx/air/x/flashx）必须先于裸版本号匹配
        if m.contains("5.2") || m.contains("5.1") {
            return Some((1.40, 4.40));
        }
        if m.contains("glm-5") || m.contains("glm5") {
            return Some((1.00, 3.20));
        }
        if m.contains("flashx") {
            return Some((0.07, 0.40));
        }
        if m.contains("airx") {
            return Some((1.10, 4.50));
        }
        if m.contains("air") {
            return Some((0.20, 1.10));
        }
        if m.contains("4.5-x") || m.contains("4.5x") {
            return Some((2.20, 8.90));
        }
        if m.contains("4.7") || m.contains("4.6") || m.contains("4.5") {
            return Some((0.60, 2.20));
        }
        return Some((0.60, 2.20)); // 其余 GLM 变体按 4.x 主档兜底
    }
    // ---- xAI Grok (official list price, USD/1M, x.ai 2026-07) ----
    if m.contains("grok") {
        if m.contains("code-fast") || m.contains("code_fast") {
            return Some((0.20, 1.50));
        }
        if m.contains("4.20") {
            return Some((2.0, 6.0));
        }
        if m.contains("4.5") {
            return Some((2.0, 6.0));
        }
        if m.contains("4.3") {
            return Some((1.25, 2.50));
        }
        if m.contains("4.1") || m.contains("4-fast") || m.contains("fast") {
            return Some((0.20, 0.50)); // grok-4.1 / grok-4.1-fast / grok-4-fast 量产档
        }
        if m.contains("build") {
            return Some((1.0, 2.0));
        }
        if m.contains("3-mini") || m.contains("3 mini") {
            return Some((0.30, 0.50));
        }
        if m.contains("grok-4") || m.contains("grok-3") {
            return Some((3.0, 15.0)); // grok-4 / grok-3 旗舰旧档
        }
        return Some((2.0, 6.0)); // 未知新 Grok 按当前旗舰档兜底
    }
    // ---- OpenAI GPT / DeepSeek / MiniMax (exact ids as the aggregator exposes them) ----
    let p = match m.as_str() {
        "gpt-5.5" => (5.0, 30.0),
        "gpt-5.4" => (2.5, 15.0),
        "deepseek-v4-flash" => (0.14, 0.28),
        "deepseek-v4-pro" => (0.435, 0.87),
        "minimax-m3" => (0.30, 1.20),
        "minimax-m2.7" | "minimax-m2.7-highspeed" => (0.25, 1.00),
        "minimax-m2.5" | "minimax-m2.5-highspeed" => (0.30, 1.20),
        "minimax-m2.1" | "minimax-m2.1-highspeed" => (0.15, 0.60),
        "minimax-m2" => (0.10, 0.40),
        _ => return None,
    };
    Some(p)
}

/// REAL billing — actual token usage × the model's REAL (official) price × the
/// connection's 倍率 (markup multiplier):
///   cost_cents = (input_tok·off_in + output_tok·off_out) / 1e6 · 100 · rate
/// `off_in/off_out` come from the per-model official catalog, falling back to the admin's
/// per-connection input/output price override when a model isn't catalogued. `rate` is the
/// connection's 倍率 (e.g. 3 = bill 3× the real cost; the operator's margin, hidden from
/// users). Uses ONLY the upstream's authoritative `usage`; no usage / no price → 0 (never
/// guesses). Cache-aware (cached input 0.1×). Hard $50/call ceiling.
#[allow(clippy::too_many_arguments)]
fn compute_cost(
    usage: Option<&serde_json::Value>,
    model_id: &str,
    rate: f64,
    admin_in: f64,
    admin_out: f64,
    cache_read_price: f64,
    cache_create_price: f64,
    model_in: f64,
    model_out: f64,
) -> i64 {
    const CACHE_READ_FACTOR: f64 = 0.1;
    const CACHE_WRITE_FACTOR: f64 = 1.25;
    const COST_CEILING_CENTS: f64 = 5000.0; // $50/call backstop — no legit single call hits this
    let u = match usage {
        Some(u) if u.is_object() => u,
        _ => return 0,
    };
    let completion = u
        .get("completion_tokens")
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("output_tokens").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    let prompt = u
        .get("prompt_tokens")
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("input_tokens").and_then(|v| v.as_f64()))
        // Only total_tokens reported → the non-output remainder is input.
        .or_else(|| {
            u.get("total_tokens")
                .and_then(|v| v.as_f64())
                .map(|t| (t - completion).max(0.0))
        })
        .unwrap_or(0.0);
    if prompt <= 0.0 && completion <= 0.0 {
        return 0;
    }
    // Price priority: admin's PER-MODEL override (model_in/out, set in the backend per enabled
    // model) wins; else the built-in official catalog; else the connection-level input/output
    // price. This lets each checked model be priced individually while keeping the catalog default.
    let (off_in, off_out) = if model_in > 0.0 || model_out > 0.0 {
        (model_in, model_out)
    } else {
        official_price(model_id).unwrap_or((admin_in, admin_out))
    };
    if off_in <= 0.0 && off_out <= 0.0 {
        return 0; // no known price for this model → can't compute a real cost
    }
    let cache_read = u.get("cache_read_input_tokens").and_then(|v| v.as_f64()); // Anthropic
    let cache_creation = u
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let cached = u
        .get("prompt_tokens_details")
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_f64())
        .or_else(|| u.get("prompt_cache_hit_tokens").and_then(|v| v.as_f64())) // DeepSeek
        .or(cache_read)
        .unwrap_or(0.0);
    // Per-token CACHE prices: admin's explicit price if set (>0), else the old factor off
    // input. cache READ is cheap, cache CREATE/write is a premium — billed separately now.
    let read_price = if cache_read_price > 0.0 {
        cache_read_price
    } else {
        off_in * CACHE_READ_FACTOR
    };
    let write_price = if cache_create_price > 0.0 {
        cache_create_price
    } else {
        off_in * CACHE_WRITE_FACTOR
    };
    // Split input into plain (full price) + cache-read + cache-create, bill each at its own
    // unit price; output at off_out. Then × 倍率. Anthropic reports input EXCLUDING cached;
    // OpenAI/DeepSeek report prompt INCLUDING cached reads (and no separate write count).
    let (plain_input, read_tok, write_tok) = if cache_read.is_some() {
        (prompt, cached, cache_creation) // Anthropic shape
    } else {
        ((prompt - cached).max(0.0), cached, 0.0) // OpenAI / DeepSeek shape
    };
    let usd = (plain_input * off_in
        + read_tok * read_price
        + write_tok * write_price
        + completion * off_out)
        / 1_000_000.0;
    let cents = (usd * 100.0 * rate.max(0.0))
        .round()
        .clamp(0.0, COST_CEILING_CENTS) as i64;
    // Detailed breakdown so we can trace "why was this call charged X" — appears
    // in `docker logs server-backend-1` at INFO level.
    tracing::info!(
        "[billing] model={} prompt={} completion={} cache_read={} cache_create={} | in_price={} read_price={:.4} write_price={:.4} out_price={} → usd={:.6} rate={} → {}¢",
        model_id, prompt as i64, completion as i64, read_tok as i64, write_tok as i64,
        off_in, read_price, write_price, off_out, usd, rate, cents
    );
    cents
}

/// Pull the final `usage` object out of an accumulated OpenAI-style SSE stream. With
/// `stream_options.include_usage` the upstream emits a trailing `data:` chunk whose
/// `usage` carries the real prompt/completion token counts; we scan every `data:` line
/// and keep the LAST one that actually has token fields. None if the stream never
/// reported usage (caller then bills the flat fee).
fn parse_usage_from_sse(acc: &[u8]) -> Option<serde_json::Value> {
    let text = String::from_utf8_lossy(acc);
    let mut last: Option<serde_json::Value> = None;
    for line in text.lines() {
        let payload = match line.trim_start().strip_prefix("data:") {
            Some(p) => p.trim(),
            None => continue,
        };
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
            if let Some(u) = v.get("usage") {
                let has_tokens = [
                    "prompt_tokens",
                    "completion_tokens",
                    "total_tokens",
                    "input_tokens",
                    "output_tokens",
                ]
                .iter()
                .any(|k| u.get(*k).and_then(|x| x.as_f64()).is_some());
                if has_tokens {
                    last = Some(u.clone());
                }
            }
        }
    }
    last
}

/// Incrementally validates OpenAI-compatible SSE before each upstream chunk is
/// forwarded. A terminal marker alone is insufficient when an earlier frame was
/// malformed: that frame may contain the missing suffix of a file-writing tool.
#[derive(Clone, Debug, Default)]
struct ToolArgumentRules {
    required: Vec<String>,
    min_lengths: std::collections::HashMap<String, usize>,
}

fn validate_streamed_tool_arguments(
    provider: &str,
    name: &str,
    raw_arguments: &str,
    rules: Option<&ToolArgumentRules>,
) -> Result<String, String> {
    let arguments = if raw_arguments.trim().is_empty() {
        "{}".to_string()
    } else {
        raw_arguments.to_string()
    };
    let parsed: serde_json::Value = serde_json::from_str(&arguments).map_err(|error| {
        format!("{provider} tool call {name:?} produced incomplete arguments JSON: {error}")
    })?;
    let object = parsed
        .as_object()
        .ok_or_else(|| format!("{provider} tool call {name:?} arguments must be a JSON object"))?;
    if let Some(rules) = rules {
        let missing = rules
            .required
            .iter()
            .filter(|key| !object.contains_key(key.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "{provider} tool call {name:?} is missing required arguments: {}",
                missing.join(", ")
            ));
        }
        for (key, min_length) in &rules.min_lengths {
            let Some(value) = object.get(key) else {
                continue;
            };
            let text = value.as_str().ok_or_else(|| {
                format!("{provider} tool call {name:?} argument {key:?} must be a string")
            })?;
            if text.chars().count() < *min_length {
                return Err(format!(
                    "{provider} tool call {name:?} argument {key:?} is shorter than minLength {min_length}"
                ));
            }
        }
    }
    Ok(arguments)
}

#[derive(Default)]
struct OpenAiToolStream {
    name: String,
    arguments: String,
}

#[derive(Default)]
struct OpenAiSseValidator {
    buf: Vec<u8>,
    done_seen: bool,
    tool_calls: std::collections::HashMap<(u64, u64), OpenAiToolStream>,
    tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
}

impl OpenAiSseValidator {
    fn with_tool_argument_rules(
        tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
    ) -> Self {
        Self {
            tool_argument_rules,
            ..Self::default()
        }
    }

    fn record_tool_calls(&mut self, event: &serde_json::Value) -> Result<(), String> {
        let Some(choices) = event.get("choices").and_then(|value| value.as_array()) else {
            return Ok(());
        };
        for (choice_position, choice) in choices.iter().enumerate() {
            let choice_index = choice
                .get("index")
                .and_then(|value| value.as_u64())
                .unwrap_or(choice_position as u64);
            let calls = choice
                .pointer("/delta/tool_calls")
                .or_else(|| choice.pointer("/message/tool_calls"));
            let Some(calls) = calls.and_then(|value| value.as_array()) else {
                continue;
            };
            for (call_position, call) in calls.iter().enumerate() {
                let tool_index = call
                    .get("index")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(call_position as u64);
                let stream = self
                    .tool_calls
                    .entry((choice_index, tool_index))
                    .or_default();
                let Some(function) = call.get("function") else {
                    continue;
                };
                let function = function
                    .as_object()
                    .ok_or_else(|| "OpenAI SSE tool call function must be an object".to_string())?;
                if let Some(name) = function.get("name") {
                    let name = name.as_str().ok_or_else(|| {
                        "OpenAI SSE tool call function.name must be a string".to_string()
                    })?;
                    if !name.is_empty() {
                        stream.name = name.to_string();
                    }
                }
                if let Some(arguments) = function.get("arguments") {
                    let arguments = arguments.as_str().ok_or_else(|| {
                        "OpenAI SSE tool call function.arguments must be a string".to_string()
                    })?;
                    stream.arguments.push_str(arguments);
                }
            }
        }
        Ok(())
    }

    fn validate_tool_calls(&self) -> Result<(), String> {
        for stream in self.tool_calls.values() {
            if stream.name.is_empty() {
                return Err("OpenAI SSE tool call ended without function.name".to_string());
            }
            validate_streamed_tool_arguments(
                "OpenAI",
                &stream.name,
                &stream.arguments,
                self.tool_argument_rules.get(&stream.name),
            )?;
        }
        Ok(())
    }

    fn push(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.buf.extend_from_slice(bytes);
        while let Some(newline) = self.buf.iter().position(|&byte| byte == b'\n') {
            let raw: Vec<u8> = self.buf.drain(..=newline).collect();
            let line = std::str::from_utf8(&raw)
                .map_err(|error| format!("OpenAI SSE contains invalid UTF-8: {error}"))?
                .trim();
            let Some(payload) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if payload.is_empty() {
                continue;
            }
            if payload == "[DONE]" {
                if self.done_seen {
                    return Err(
                        "OpenAI SSE contains more than one terminal data: [DONE]".to_string()
                    );
                }
                // Validate before the caller forwards the chunk containing [DONE]. This
                // prevents clients from observing a successful terminal event for a
                // truncated tool call and also keeps that response out of the cache.
                self.validate_tool_calls()?;
                self.done_seen = true;
                continue;
            }
            if self.done_seen {
                return Err("OpenAI SSE contains data after terminal data: [DONE]".to_string());
            }
            let event = serde_json::from_str::<serde_json::Value>(payload)
                .map_err(|error| format!("OpenAI SSE contains malformed JSON: {error}"))?;
            self.record_tool_calls(&event)?;
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), String> {
        if !self.buf.iter().all(u8::is_ascii_whitespace) {
            return Err("OpenAI upstream stream ended with an incomplete SSE frame".to_string());
        }
        if !self.done_seen {
            return Err("OpenAI upstream stream ended without terminal data: [DONE]".to_string());
        }
        self.validate_tool_calls()
    }
}

#[cfg(test)]
fn validate_openai_sse_eof(bytes: &[u8]) -> Result<(), String> {
    let mut validator = OpenAiSseValidator::default();
    validator.push(bytes)?;
    validator.finish()
}

#[cfg(test)]
fn validate_openai_sse_with_rules(
    bytes: &[u8],
    rules: std::collections::HashMap<String, ToolArgumentRules>,
) -> Result<(), String> {
    let mut validator = OpenAiSseValidator::with_tool_argument_rules(rules);
    validator.push(bytes)?;
    validator.finish()
}

/// Strip ALL `cache_control` before forwarding. PROVEN via per-call fingerprints that
/// the [tools+system] prefix is byte-IDENTICAL on every call (16+ consecutive calls,
/// same sys_hash + tools_hash) — yet the relay (zyz) still bills cache CREATION (a 1.25×
/// write premium) on nearly every call and serves reads only sporadically (its prompt
/// cache appears per-instance behind a load balancer, so identical calls keep missing).
/// So on this relay cache_control is, on average, a pure write premium. Stripping it →
/// flat 1× billing. The real win (write-once, then 0.1× reads) needs a RELIABLE-caching
/// upstream (Anthropic / Bedrock direct, or LiteLLM), not this relay.
fn strip_cache_control(body: &mut serde_json::Value) {
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        for m in msgs.iter_mut() {
            if let Some(content) = m.get_mut("content") {
                if let Some(blocks) = content.as_array_mut() {
                    for b in blocks.iter_mut() {
                        if let Some(o) = b.as_object_mut() {
                            o.remove("cache_control");
                        }
                    }
                }
            }
        }
    }
    if let Some(tools) = body.get_mut("tools").and_then(|t| t.as_array_mut()) {
        for t in tools.iter_mut() {
            if let Some(o) = t.as_object_mut() {
                o.remove("cache_control");
            }
        }
    }
}

/// Deterministic cache key for a chat request: hashes the full request body (model
/// + messages + params). serde_json serializes Map keys sorted, so it's stable.
///
/// 128-bit (two seeded hashes) so a collision — which would serve a WRONG cached
/// response — is negligible.
fn gw_cache_key(body: &serde_json::Value) -> String {
    use std::hash::{Hash, Hasher};
    let bytes = serde_json::to_vec(body).unwrap_or_default();
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h1);
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    0x9e37_79b9_7f4a_7c15u64.hash(&mut h2);
    bytes.hash(&mut h2);
    format!("gwc:{:016x}{:016x}", h1.finish(), h2.finish())
}

fn response_cache_safe(bytes: &[u8]) -> bool {
    // Tool-call arguments contain the user's full tracking number. The native tool
    // masks its result, but caching the model response would retain the original
    // argument in Redis. A false positive only costs one cache miss.
    !bytes
        .windows(b"track_shipment".len())
        .any(|window| window == b"track_shipment")
}

/// POST /v1/chat/completions — OpenAI-compatible gateway. Auth via a Michael API
/// key (Bearer). Resolves `model` to the connection that exposes it, forwards
/// the request (streaming passthrough), and bills the key owner's credits.
/// Repair malformed `tool_calls[*].function.arguments` strings from upstream relays.
/// Specifically targets the `'{}'` + `'{...}'` concatenation bug seen on Claude-via-
/// OpenAI-compat relays, where the placeholder `{}` is glued to the real args JSON
/// instead of replaced. We detect this exact pattern and keep only the trailing JSON.
fn fix_tool_call_arguments(data: &mut serde_json::Value) {
    let choices = match data.get_mut("choices").and_then(|c| c.as_array_mut()) {
        Some(c) => c,
        None => return,
    };
    for ch in choices {
        let tcs = match ch
            .pointer_mut("/message/tool_calls")
            .and_then(|t| t.as_array_mut())
        {
            Some(t) => t,
            None => continue,
        };
        for tc in tcs {
            let args_val = match tc.pointer_mut("/function/arguments") {
                Some(v) => v,
                None => continue,
            };
            let s = match args_val.as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            // Strip a literal leading `{}` followed by another JSON object — that's
            // the exact concatenation bug. Don't touch valid single-object strings.
            let trimmed = s.trim_start();
            if let Some(rest) = trimmed.strip_prefix("{}") {
                let rest = rest.trim_start();
                if rest.starts_with('{') && serde_json::from_str::<serde_json::Value>(rest).is_ok()
                {
                    *args_val = serde_json::Value::String(rest.to_string());
                    continue;
                }
            }
            // Fallback: try to parse; if it fails, attempt to locate the last valid
            // JSON object in the string (handles `xxx{...}` garbage prefix).
            if serde_json::from_str::<serde_json::Value>(&s).is_err() {
                if let Some(last_open) = s.rfind('{') {
                    let candidate = &s[last_open..];
                    if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                        *args_val = serde_json::Value::String(candidate.to_string());
                    }
                }
            }
        }
    }
}

/// Resolve a caller to a user id from either an api_key or a login JWT (Bearer).
/// Used by free, auth-gated endpoints (knowledge base) that need a valid user but
/// don't bill.
pub(crate) async fn auth_any_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<uuid::Uuid, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    match sqlx::query_scalar::<_, uuid::Uuid>("SELECT user_id FROM api_keys WHERE api_key = $1")
        .bind(&token)
        .fetch_optional(&state.db)
        .await?
    {
        Some(u) => Ok(u),
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效")),
    }
}

/// POST /api/knowledge/search — agentic-RAG retrieval over the curated domain
/// knowledge corpus. Body: { query, domain?, top_k? }. Free (no billing); auth
/// only to prevent open abuse. Returns the most relevant best-practice sections.
pub async fn knowledge_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    auth_any_user(&state, &headers).await?;
    let query = body
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if query.is_empty() {
        return Err(AppError::bad("缺少 query"));
    }
    let domain = body
        .get("domain")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty());
    let top_k = body.get("top_k").and_then(|v| v.as_u64()).unwrap_or(6) as usize;
    let hits = crate::knowledge::search(query, domain, top_k);
    Ok(Json(json!({ "results": hits })))
}

/// GET /api/knowledge/domains — list the available knowledge domains + their topics
/// so the agent (or the IDE) can see what's covered.
pub async fn knowledge_domains(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    auth_any_user(&state, &headers).await?;
    let idx = crate::knowledge::get();
    let domains: Vec<_> = idx
        .domains
        .iter()
        .map(|(d, t)| json!({ "domain": d, "topics": t }))
        .collect();
    Ok(Json(json!({ "domains": domains })))
}

/// Per-call token detail for the model_usage audit trail.
#[derive(Clone, Default)]
struct BillTokens {
    prompt: i64,
    completion: i64,
    cached: i64,
    model_name: String,
    estimated: bool,
}

/// Extract BillTokens from a provider usage JSON (OpenAI or Anthropic shape).
fn extract_bill_tokens(
    usage: Option<&serde_json::Value>,
    model_name: &str,
    estimated: bool,
) -> BillTokens {
    let u = match usage.and_then(|v| if v.is_object() { Some(v) } else { None }) {
        Some(v) => v,
        None => {
            return BillTokens {
                model_name: model_name.to_string(),
                estimated,
                ..Default::default()
            }
        }
    };
    let gi = |keys: &[&str]| -> i64 {
        for k in keys {
            if let Some(n) = u.get(*k).and_then(|x| x.as_i64()) {
                return n;
            }
        }
        0
    };
    let cached = gi(&["cache_read_input_tokens"])
        .max(
            u.pointer("/prompt_tokens_details/cached_tokens")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
        )
        .max(gi(&["prompt_cache_hit_tokens"]));
    BillTokens {
        prompt: gi(&["prompt_tokens", "input_tokens"]),
        completion: gi(&["completion_tokens", "output_tokens"]),
        cached,
        model_name: model_name.to_string(),
        estimated,
    }
}

/// CJK-aware rough token estimate from a serialized JSON body (input side).
fn estimate_input_tokens(body: &serde_json::Value) -> i64 {
    let s = serde_json::to_string(body).unwrap_or_default();
    let mut cjk = 0i64;
    for c in s.chars() {
        if ('\u{2E80}'..='\u{9FFF}').contains(&c)
            || ('\u{AC00}'..='\u{D7A3}').contains(&c)
            || ('\u{F900}'..='\u{FAFF}').contains(&c)
            || ('\u{FF00}'..='\u{FFEF}').contains(&c)
        {
            cjk += 1;
        }
    }
    cjk + (s.len() as i64 - cjk) / 4
}

/// Rough output token estimate from accumulated SSE response bytes.
fn estimate_output_tokens(response_bytes: usize) -> i64 {
    (response_bytes as f64 * 0.6 / 4.0).round().max(0.0) as i64
}

/// Build a synthetic usage JSON from estimates (fallback when provider reports nothing).
fn estimated_usage(input_tok: i64, output_tok: i64) -> serde_json::Value {
    json!({ "prompt_tokens": input_tok, "completion_tokens": output_tok, "total_tokens": input_tok + output_tok })
}

/// Deduct cost from the user's quota/credits and log the model_usage row with token detail.
/// Module-scope so chat_completions, responses_proxy, and image_generations all share it.
async fn bill(
    state: &AppState,
    uid: uuid::Uuid,
    conn_id: uuid::Uuid,
    cost: i64,
    use_quota: bool,
    tokens: &BillTokens,
) {
    if cost > 0 {
        if use_quota {
            let _ = sqlx::query(
                "UPDATE users SET quota_total_cents = GREATEST(quota_total_cents - $1, 0), \
                 quota_window_cents = GREATEST(quota_window_cents - $1, 0), \
                 quota_week_used_cents = quota_week_used_cents + $1 WHERE id = $2",
            )
            .bind(cost)
            .bind(uid)
            .execute(&state.db)
            .await;
        } else {
            let _ = sqlx::query(
                "UPDATE users SET credits_cents = GREATEST(credits_cents - $1, 0) WHERE id = $2",
            )
            .bind(cost)
            .bind(uid)
            .execute(&state.db)
            .await;
        }
    }
    let _ = sqlx::query(
        "INSERT INTO model_usage (user_id, model_id, cost_cents, prompt_tokens, completion_tokens, cached_tokens, model_name, estimated) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(uid)
    .bind(conn_id)
    .bind(cost)
    .bind(tokens.prompt)
    .bind(tokens.completion)
    .bind(tokens.cached)
    .bind(&tokens.model_name)
    .bind(tokens.estimated)
    .execute(&state.db)
    .await;
}

// ============ Anthropic protocol bridge (OpenAI ⇄ Anthropic Messages API) ============
// A connection with protocol="anthropic" talks the NATIVE Anthropic /v1/messages API instead
// of the OpenAI-compat /chat/completions wrapper. Native = reliable prompt caching (0.1× reads,
// proven working on this upstream) + correct tool-call streaming (the compat wrapper stalled /
// garbled Claude tool writes). The IDE still speaks OpenAI, so the gateway translates the
// request → Anthropic and the response (streaming + non-streaming) → OpenAI. protocol="openai"
// paths are completely untouched.

/// Flatten an OpenAI message `content` (string OR array of parts) to plain text.
fn oai_content_text(content: Option<&serde_json::Value>) -> String {
    match content {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter_map(|p| {
                if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                    p.get("text").and_then(|t| t.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// OpenAI user `content` → Anthropic content (plain string, or blocks incl. images).
fn oai_content_to_anthropic(content: Option<&serde_json::Value>) -> serde_json::Value {
    match content {
        Some(serde_json::Value::Array(parts)) => {
            let mut blocks: Vec<serde_json::Value> = Vec::new();
            for p in parts {
                match p.get("type").and_then(|t| t.as_str()) {
                    Some("text") => {
                        if let Some(t) = p.get("text").and_then(|v| v.as_str()) {
                            blocks.push(json!({"type":"text","text":t}));
                        }
                    }
                    Some("image_url") => {
                        if let Some(u) = p.pointer("/image_url/url").and_then(|v| v.as_str()) {
                            if let Some(rest) = u.strip_prefix("data:") {
                                if let Some((meta, data)) = rest.split_once(',') {
                                    let media = meta.split(';').next().unwrap_or("image/png");
                                    blocks.push(json!({"type":"image","source":{"type":"base64","media_type":media,"data":data}}));
                                }
                            } else {
                                blocks
                                    .push(json!({"type":"image","source":{"type":"url","url":u}}));
                            }
                        }
                    }
                    _ => {}
                }
            }
            if blocks.is_empty() {
                json!("")
            } else {
                json!(blocks)
            }
        }
        Some(serde_json::Value::String(s)) => json!(s.clone()),
        _ => json!(""),
    }
}

/// Extended-thinking config for a Claude model on the native Anthropic path, or None.
/// Adaptive thinking (Opus/Sonnet 4.x+, Fable, Mythos) is Anthropic's smartest mode and
/// auto-scales depth (minimal on trivial calls, deep on hard ones) — the single biggest IQ
/// lever for the coding agent. Verified live against the upstream: adaptive is accepted, and
/// replayed tool_use turns WITHOUT preserved thinking blocks are tolerated (200, not 400), so
/// no thinking-signature round-trip through the OpenAI-format history is needed.
/// Haiku stays fast/cheap (no thinking); 3.7 uses the older explicit-budget form; 3.5 none.
/// Respects the client's per-model control: `reasoning_effort` present = thinking ON, absent /
/// "off" = OFF (the IDE defaults Claude to "medium" and drops the field on "off").
/// Master off-switch: env MICHAEL_ANTHROPIC_THINKING=0.
fn anthropic_thinking(model: &str, effort: Option<&str>) -> Option<serde_json::Value> {
    if std::env::var("MICHAEL_ANTHROPIC_THINKING").ok().as_deref() == Some("0") {
        return None;
    }
    let eff = match effort {
        Some(e) if !e.is_empty() && e != "off" => e,
        _ => return None,
    };
    let m = model.to_lowercase();
    if m.contains("haiku") {
        return None;
    } // fast tier → keep it fast
    if m.contains("claude-3-5") || m.contains("claude-3.5") {
        return None;
    } // pre-thinking
    if m.contains("claude-3-7") || m.contains("claude-3.7") {
        // 3.7 → explicit budget
        let budget = match eff {
            "low" => 4000,
            "high" | "max" => 12000,
            _ => 8000,
        };
        return Some(json!({"type":"enabled","budget_tokens":budget}));
    }
    if m.contains("claude") || m.contains("fable") || m.contains("mythos") {
        // 4.x+/Fable/Mythos 一律用显式预算：聚合上游（zyz 等）对 {"type":"adaptive"}
        // 静默忽略——请求 200 但一个 thinking_delta 都不回，IDE 的思考卡永远是空的；
        // 换成 enabled+budget_tokens 后实测同一路线能正常回思考流。
        let budget = match eff {
            "low" => 4096,
            "high" => 24000,
            "max" | "xhigh" => 32000,
            _ => 12000,
        };
        return Some(json!({"type":"enabled","budget_tokens":budget}));
    }
    None
}

/// OpenAI /chat/completions body → Anthropic /v1/messages body.
fn oai_to_anthropic(body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let mut system_parts: Vec<String> = Vec::new();
    let mut messages: Vec<serde_json::Value> = Vec::new();
    if let Some(msgs) = body.get("messages").and_then(|m| m.as_array()) {
        for m in msgs {
            match m.get("role").and_then(|r| r.as_str()).unwrap_or("user") {
                "system" => {
                    let s = oai_content_text(m.get("content"));
                    if !s.is_empty() {
                        system_parts.push(s);
                    }
                }
                "tool" => {
                    // OpenAI tool result → Anthropic user turn w/ a tool_result block. Consecutive
                    // tool results MUST be grouped into one user turn (Anthropic requirement).
                    let tcid = m.get("tool_call_id").and_then(|v| v.as_str()).unwrap_or("");
                    let block = json!({"type":"tool_result","tool_use_id":tcid,"content":oai_content_text(m.get("content"))});
                    let can_group = messages.last().is_some_and(|last| {
                        last.get("role").and_then(|r| r.as_str()) == Some("user")
                            && last
                                .get("content")
                                .and_then(|c| c.as_array())
                                .is_some_and(|a| {
                                    a.iter().all(|b| {
                                        b.get("type").and_then(|t| t.as_str())
                                            == Some("tool_result")
                                    })
                                })
                    });
                    if can_group {
                        if let Some(arr) = messages
                            .last_mut()
                            .and_then(|l| l.get_mut("content"))
                            .and_then(|c| c.as_array_mut())
                        {
                            arr.push(block);
                        }
                    } else {
                        messages.push(json!({"role":"user","content":[block]}));
                    }
                }
                "assistant" => {
                    let mut blocks: Vec<serde_json::Value> = Vec::new();
                    let s = oai_content_text(m.get("content"));
                    if !s.is_empty() {
                        blocks.push(json!({"type":"text","text":s}));
                    }
                    if let Some(tcs) = m.get("tool_calls").and_then(|t| t.as_array()) {
                        for tc in tcs {
                            let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            let name = tc
                                .pointer("/function/name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let args = tc.pointer("/function/arguments").ok_or_else(|| {
                                format!(
                                    "assistant tool call {name:?} (id {id:?}) is missing function.arguments"
                                )
                            })?;
                            let input = match args {
                                serde_json::Value::String(args) => serde_json::from_str(args)
                                    .map_err(|err| {
                                        format!(
                                            "assistant tool call {name:?} (id {id:?}) has malformed function.arguments JSON: {err}"
                                        )
                                    })?,
                                serde_json::Value::Object(_) => args.clone(),
                                _ => {
                                    return Err(format!(
                                        "assistant tool call {name:?} (id {id:?}) has non-object function.arguments"
                                    ));
                                }
                            };
                            if !input.is_object() {
                                return Err(format!(
                                    "assistant tool call {name:?} (id {id:?}) function.arguments must decode to a JSON object"
                                ));
                            }
                            blocks
                                .push(json!({"type":"tool_use","id":id,"name":name,"input":input}));
                        }
                    }
                    if blocks.is_empty() {
                        blocks.push(json!({"type":"text","text":"(no content)"}));
                    }
                    messages.push(json!({"role":"assistant","content":blocks}));
                }
                _ => messages.push(
                    json!({"role":"user","content":oai_content_to_anthropic(m.get("content"))}),
                ),
            }
        }
    }
    let mut out = serde_json::Map::new();
    if let Some(model) = body.get("model") {
        out.insert("model".into(), model.clone());
    }
    out.insert("messages".into(), json!(messages));
    if !system_parts.is_empty() {
        out.insert("system".into(), json!(system_parts.join("\n\n")));
    }
    // Extended thinking — ALWAYS use the gateway's model-aware config, never the client's
    // `thinking` field. Newer models (Sonnet 5, Opus 4.7/4.8, Fable 5) REJECT the old
    // `{"type":"enabled","budget_tokens":N}` format with a 400/502 — they require
    // `{"type":"adaptive"}` + `output_config.effort`. The IDE client may still send the old
    // format; the gateway normalises it here per-model.
    let model_str = body.get("model").and_then(|v| v.as_str()).unwrap_or("");
    let effort = body
        .get("reasoning_effort")
        .and_then(|v| v.as_str())
        .or_else(|| {
            // 客户端只发 thinking:{budget_tokens} 不发 reasoning_effort（IDE Claude 族的
            // 真实形状）时，按预算推档——以前这里一律写死 "high"，用户转盘上的
            // low/medium/max 全被压平成 high，max 的 64K 输出余量也永远打不中。
            // 档位边界与 IDE budgets{low:4096, medium:12000, high:24000, max:32000} 对齐。
            body.get("thinking").map(|t| {
                match t.get("budget_tokens").and_then(|v| v.as_i64()).unwrap_or(0) {
                    b if b > 24000 => "max",
                    b if b > 12000 => "high",
                    b if b > 4096 => "medium",
                    b if b > 0 => "low",
                    _ => "high", // 无预算的裸 thinking 开关（Kimi/GLM 形状）保持旧行为
                }
            })
        });
    let thinking = anthropic_thinking(model_str, effort);
    let thinking_on = thinking.is_some();
    // Anthropic REQUIRES max_tokens. Map from OpenAI, else a generous default.
    let mut max_tokens = body
        .get("max_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| body.get("max_completion_tokens").and_then(|v| v.as_i64()))
        .filter(|n| *n > 0)
        .unwrap_or(8192);
    // For adaptive thinking: no budget_tokens, just ensure a generous max_tokens.
    // For budget-based (3.7): ensure max_tokens > budget_tokens.
    if thinking_on {
        let budget = thinking
            .as_ref()
            .and_then(|t| t.get("budget_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        // Give the deepest effort real headroom to think long. Both "high" and "max" map to
        // output_config.effort="high" (Anthropic's top knob), so the ONLY thing that makes
        // "max" deeper than "high" is more max_tokens room for adaptive thinking to stretch.
        // Without this the top UI dial is a no-op. Gated by effort so low/medium stay lean;
        // weak/fast models never reach here (thinking is None for haiku/3.5/non-Claude).
        let floor = match effort {
            Some("max") => 64000,
            Some("high") => 40000,
            _ => 32000,
        };
        let min_mt = (budget + 8000).max(floor);
        if max_tokens < min_mt {
            max_tokens = min_mt;
        }
    }
    let max_tokens = max_tokens.clamp(1, 128000);
    out.insert("max_tokens".into(), json!(max_tokens));
    if let Some(t) = &thinking {
        out.insert("thinking".into(), t.clone());
        // 不发 output_config.effort：实测聚合上游（zyz）一旦收到 effort 就把思考流
        // 换成一句 "Compatibility reasoning summary." 摘要，完整原始思考全部丢失；
        // 只发 thinking.budget_tokens 时上游按原文回思考流。思考深度已由
        // budget_tokens + max_tokens 下限（见上）控制，effort 不再需要。
    }
    // stream/stop always pass through; temperature/top_p are INCOMPATIBLE with thinking
    // (Anthropic rejects non-default values), so copy them only when thinking is OFF.
    for k in ["stream", "stop"] {
        if let Some(v) = body.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    if !thinking_on {
        for k in ["temperature", "top_p"] {
            if let Some(v) = body.get(k) {
                out.insert(k.to_string(), v.clone());
            }
        }
    }
    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        let atools: Vec<serde_json::Value> = tools
            .iter()
            .filter_map(|t| {
                let f = t.get("function")?;
                let name = f.get("name")?.as_str()?;
                let mut a = serde_json::Map::new();
                a.insert("name".into(), json!(name));
                if let Some(d) = f.get("description") {
                    a.insert("description".into(), d.clone());
                }
                a.insert(
                    "input_schema".into(),
                    f.get("parameters")
                        .cloned()
                        .unwrap_or_else(|| json!({"type":"object","properties":{}})),
                );
                Some(serde_json::Value::Object(a))
            })
            .collect();
        if !atools.is_empty() {
            out.insert("tools".into(), json!(atools));
        }
    }
    if let Some(tc) = body.get("tool_choice") {
        let atc = match tc.as_str() {
            Some("auto") => Some(json!({"type":"auto"})),
            Some("required") => Some(json!({"type":"any"})),
            Some("none") => None,
            _ => tc
                .pointer("/function/name")
                .and_then(|n| n.as_str())
                .map(|n| json!({"type":"tool","name":n})),
        };
        if let Some(v) = atc {
            out.insert("tool_choice".into(), v);
        }
    }
    Ok(serde_json::Value::Object(out))
}

/// Anthropic usage → an object carrying BOTH Anthropic token names (so compute_cost bills
/// cache-correctly) and OpenAI names (so OpenAI clients read prompt/completion tokens).
fn anthropic_usage_merged(au: &serde_json::Value) -> serde_json::Value {
    let g = |k: &str| au.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
    let (it, ot) = (g("input_tokens"), g("output_tokens"));
    json!({
        "input_tokens": it, "output_tokens": ot,
        "cache_read_input_tokens": g("cache_read_input_tokens"),
        "cache_creation_input_tokens": g("cache_creation_input_tokens"),
        "prompt_tokens": it, "completion_tokens": ot, "total_tokens": it + ot,
    })
}

/// Anthropic non-streaming response → OpenAI /chat/completions response.
fn anthropic_to_oai(av: &serde_json::Value, model: &str) -> serde_json::Value {
    let mut text = String::new();
    let mut tool_calls: Vec<serde_json::Value> = Vec::new();
    if let Some(content) = av.get("content").and_then(|c| c.as_array()) {
        for b in content {
            match b.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                        text.push_str(t);
                    }
                }
                Some("tool_use") => {
                    let input = b.get("input").cloned().unwrap_or_else(|| json!({}));
                    tool_calls.push(json!({
                        "id": b.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        "type": "function",
                        "function": {"name": b.get("name").and_then(|v| v.as_str()).unwrap_or(""), "arguments": serde_json::to_string(&input).unwrap_or_else(|_| "{}".into())}
                    }));
                }
                _ => {}
            }
        }
    }
    let finish = match av.get("stop_reason").and_then(|v| v.as_str()) {
        Some("tool_use") => "tool_calls",
        Some("max_tokens") => "length",
        _ => "stop",
    };
    let mut message = serde_json::Map::new();
    message.insert("role".into(), json!("assistant"));
    message.insert(
        "content".into(),
        if text.is_empty() && !tool_calls.is_empty() {
            serde_json::Value::Null
        } else {
            json!(text)
        },
    );
    if !tool_calls.is_empty() {
        message.insert("tool_calls".into(), json!(tool_calls));
    }
    json!({
        "id": av.get("id").cloned().unwrap_or_else(|| json!("chatcmpl-anthropic")),
        "object": "chat.completion", "model": model,
        "choices": [{"index": 0, "message": serde_json::Value::Object(message), "finish_reason": finish}],
        "usage": anthropic_usage_merged(av.get("usage").unwrap_or(&json!({}))),
    })
}

fn tool_argument_rules(
    body: &serde_json::Value,
) -> std::collections::HashMap<String, ToolArgumentRules> {
    body.get("tools")
        .and_then(|tools| tools.as_array())
        .into_iter()
        .flatten()
        .filter_map(|tool| {
            let function = tool.get("function")?;
            let name = function.get("name")?.as_str()?.to_string();
            let required = function
                .pointer("/parameters/required")
                .and_then(|required| required.as_array())
                .into_iter()
                .flatten()
                .filter_map(|key| key.as_str().map(str::to_string))
                .collect::<Vec<_>>();
            let min_lengths = function
                .pointer("/parameters/properties")
                .and_then(|properties| properties.as_object())
                .into_iter()
                .flatten()
                .filter_map(|(key, schema)| {
                    schema
                        .get("minLength")
                        .and_then(|value| value.as_u64())
                        .and_then(|value| usize::try_from(value).ok())
                        .map(|value| (key.clone(), value))
                })
                .collect();
            Some((
                name,
                ToolArgumentRules {
                    required,
                    min_lengths,
                },
            ))
        })
        .collect()
}

/// Stateful converter: Anthropic Messages SSE stream → OpenAI chat.completions SSE stream.
/// Fed raw upstream bytes via `push` (handles chunk-split events); emits ready-to-forward
/// OpenAI `data:` lines. Accumulates usage for billing. `finish` emits the terminal chunks.
struct AnthToolStream {
    tool_index: i64,
    name: String,
    arguments: String,
    stopped: bool,
}

struct AnthSse {
    buf: Vec<u8>,
    model: String,
    role_sent: bool,
    next_tool_idx: i64,
    tool_blocks: std::collections::HashMap<i64, AnthToolStream>,
    tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
    message_stop_seen: bool,
    input_tokens: i64,
    output_tokens: i64,
    cache_read: i64,
    cache_create: i64,
    stop_reason: String,
    out_bytes: usize, // incremental output byte counter for fallback estimation
}
impl AnthSse {
    #[cfg(test)]
    fn new(model: &str) -> Self {
        Self::with_tool_argument_rules(model, std::collections::HashMap::new())
    }

    #[cfg(test)]
    fn with_required_tool_args(
        model: &str,
        required_tool_args: std::collections::HashMap<String, Vec<String>>,
    ) -> Self {
        let rules = required_tool_args
            .into_iter()
            .map(|(name, required)| {
                (
                    name,
                    ToolArgumentRules {
                        required,
                        min_lengths: std::collections::HashMap::new(),
                    },
                )
            })
            .collect();
        Self::with_tool_argument_rules(model, rules)
    }

    fn with_tool_argument_rules(
        model: &str,
        tool_argument_rules: std::collections::HashMap<String, ToolArgumentRules>,
    ) -> Self {
        AnthSse {
            buf: Vec::new(),
            model: model.to_string(),
            role_sent: false,
            next_tool_idx: 0,
            tool_blocks: std::collections::HashMap::new(),
            tool_argument_rules,
            message_stop_seen: false,
            input_tokens: 0,
            output_tokens: 0,
            cache_read: 0,
            cache_create: 0,
            stop_reason: "stop".into(),
            out_bytes: 0,
        }
    }

    fn validated_tool_arguments(&self, block: &AnthToolStream) -> Result<String, String> {
        validate_streamed_tool_arguments(
            "Anthropic",
            &block.name,
            &block.arguments,
            self.tool_argument_rules.get(&block.name),
        )
    }
    fn chunk(&self, delta: serde_json::Value, finish: Option<&str>) -> Vec<u8> {
        let choice = json!({"index":0,"delta":delta,"finish_reason": match finish { Some(f) => json!(f), None => serde_json::Value::Null }});
        format!(
            "data: {}\n\n",
            json!({"object":"chat.completion.chunk","model":self.model,"choices":[choice]})
        )
        .into_bytes()
    }
    fn ensure_role(&mut self, out: &mut Vec<u8>) {
        if !self.role_sent {
            out.extend(self.chunk(json!({"role":"assistant","content":""}), None));
            self.role_sent = true;
        }
    }
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        self.buf.extend_from_slice(bytes);
        let mut out: Vec<u8> = Vec::new();
        while let Some(nl) = self.buf.iter().position(|&b| b == b'\n') {
            let raw: Vec<u8> = self.buf.drain(0..=nl).collect();
            let line = std::str::from_utf8(&raw)
                .map_err(|err| format!("Anthropic SSE contains invalid UTF-8: {err}"))?
                .trim();
            let data = match line.strip_prefix("data:") {
                Some(d) => d.trim(),
                None => continue,
            };
            if data.is_empty() {
                continue;
            }
            let ev: serde_json::Value = serde_json::from_str(data)
                .map_err(|err| format!("invalid Anthropic SSE JSON: {err}"))?;
            match ev.get("type").and_then(|t| t.as_str()) {
                Some("message_start") => {
                    if let Some(u) = ev.pointer("/message/usage") {
                        self.input_tokens =
                            u.get("input_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                        self.cache_read = u
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        self.cache_create = u
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                    }
                    self.ensure_role(&mut out);
                }
                Some("content_block_start") => {
                    let idx = ev.get("index").and_then(|v| v.as_i64()).ok_or_else(|| {
                        "Anthropic content_block_start is missing a numeric index".to_string()
                    })?;
                    let cb = ev.get("content_block");
                    if cb.and_then(|c| c.get("type")).and_then(|t| t.as_str()) == Some("tool_use") {
                        if self.tool_blocks.contains_key(&idx) {
                            return Err(format!(
                                "Anthropic tool_use reused content block index {idx}"
                            ));
                        }
                        let ti = self.next_tool_idx;
                        self.next_tool_idx += 1;
                        let id = cb
                            .and_then(|c| c.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let name = cb
                            .and_then(|c| c.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let input = cb
                            .and_then(|c| c.get("input"))
                            .cloned()
                            .unwrap_or_else(|| json!({}));
                        let initial_arguments = match input.as_object() {
                            Some(input) if input.is_empty() => String::new(),
                            Some(_) => serde_json::to_string(&input).map_err(|err| {
                                format!(
                                    "Anthropic tool_use {name:?} contains unserializable input: {err}"
                                )
                            })?,
                            None => {
                                return Err(format!(
                                    "Anthropic tool_use {name:?} input must be a JSON object"
                                ));
                            }
                        };
                        self.tool_blocks.insert(
                            idx,
                            AnthToolStream {
                                tool_index: ti,
                                name: name.to_string(),
                                arguments: initial_arguments.clone(),
                                stopped: false,
                            },
                        );
                        self.ensure_role(&mut out);
                        out.extend(self.chunk(json!({"tool_calls":[{"index":ti,"id":id,"type":"function","function":{"name":name,"arguments":initial_arguments}}]}), None));
                    }
                }
                Some("content_block_delta") => {
                    match ev.pointer("/delta/type").and_then(|t| t.as_str()) {
                        Some("text_delta") => {
                            if let Some(t) = ev.pointer("/delta/text").and_then(|v| v.as_str()) {
                                self.out_bytes += t.len();
                                self.ensure_role(&mut out);
                                out.extend(self.chunk(json!({"content": t}), None));
                            }
                        }
                        Some("thinking_delta") => {
                            if let Some(t) = ev.pointer("/delta/thinking").and_then(|v| v.as_str())
                            {
                                self.out_bytes += t.len();
                                self.ensure_role(&mut out);
                                out.extend(self.chunk(json!({"reasoning_content": t}), None));
                            }
                        }
                        Some("input_json_delta") => {
                            let idx = ev.get("index").and_then(|v| v.as_i64()).ok_or_else(|| {
                                "Anthropic input_json_delta is missing a numeric content block index"
                                    .to_string()
                            })?;
                            let pj = ev
                                .pointer("/delta/partial_json")
                                .and_then(|v| v.as_str())
                                .ok_or_else(|| {
                                    format!(
                                        "Anthropic input_json_delta for index {idx} is missing partial_json"
                                    )
                                })?;
                            let block = self.tool_blocks.get_mut(&idx).ok_or_else(|| {
                                format!(
                                    "Anthropic input_json_delta references unknown content block index {idx}"
                                )
                            })?;
                            if block.stopped {
                                return Err(format!(
                                    "Anthropic input_json_delta arrived after content_block_stop for index {idx}"
                                ));
                            }
                            block.arguments.push_str(pj);
                            let ti = block.tool_index;
                            out.extend(self.chunk(
                                json!({"tool_calls":[{"index":ti,"function":{"arguments": pj}}]}),
                                None,
                            ));
                        }
                        _ => {}
                    }
                }
                Some("message_delta") => {
                    if let Some(sr) = ev.pointer("/delta/stop_reason").and_then(|v| v.as_str()) {
                        self.stop_reason = match sr {
                            "tool_use" => "tool_calls",
                            "max_tokens" => "length",
                            _ => "stop",
                        }
                        .into();
                    }
                    if let Some(v) = ev.pointer("/usage/output_tokens").and_then(|v| v.as_i64()) {
                        self.output_tokens = v;
                    }
                    if let Some(v) = ev.pointer("/usage/input_tokens").and_then(|v| v.as_i64()) {
                        if v > 0 {
                            self.input_tokens = v;
                        }
                    }
                    if let Some(v) = ev
                        .pointer("/usage/cache_read_input_tokens")
                        .and_then(|v| v.as_i64())
                    {
                        if v > 0 {
                            self.cache_read = v;
                        }
                    }
                }
                Some("content_block_stop") => {
                    if let Some(idx) = ev.get("index").and_then(|v| v.as_i64()) {
                        if let Some(block) = self.tool_blocks.get(&idx) {
                            if block.stopped {
                                return Err(format!(
                                    "Anthropic content block index {idx} stopped more than once"
                                ));
                            }
                            let arguments = self.validated_tool_arguments(block)?;
                            let emit_empty_object = block.arguments.trim().is_empty();
                            let ti = block.tool_index;
                            if emit_empty_object {
                                out.extend(self.chunk(json!({"tool_calls":[{"index":ti,"function":{"arguments":arguments}}]}), None));
                            }
                        }
                        if let Some(block) = self.tool_blocks.get_mut(&idx) {
                            block.stopped = true;
                        }
                    }
                }
                Some("message_stop") => {
                    self.message_stop_seen = true;
                }
                Some("error") => {
                    let message = ev
                        .pointer("/error/message")
                        .and_then(|value| value.as_str())
                        .unwrap_or("unknown Anthropic streaming error");
                    return Err(format!("Anthropic streaming error: {message}"));
                }
                _ => {} // ping / text block stops → emit nothing
            }
        }
        Ok(out)
    }
    fn usage(&self) -> serde_json::Value {
        let ot = if self.output_tokens > 0 {
            self.output_tokens
        } else if self.out_bytes > 0 {
            // Stream broke before message_delta reported output_tokens.
            // Estimate from the content bytes we DID forward (~4 chars/token).
            (self.out_bytes as i64 / 4).max(1)
        } else {
            0
        };
        json!({
            "input_tokens": self.input_tokens, "output_tokens": ot,
            "cache_read_input_tokens": self.cache_read, "cache_creation_input_tokens": self.cache_create,
            "prompt_tokens": self.input_tokens, "completion_tokens": ot,
            "total_tokens": self.input_tokens + ot,
        })
    }
    fn finish(&self) -> Result<Vec<u8>, String> {
        if !self.buf.iter().all(u8::is_ascii_whitespace) {
            return Err("Anthropic stream ended with an incomplete SSE frame".to_string());
        }
        if !self.message_stop_seen {
            return Err("Anthropic stream ended before message_stop".to_string());
        }
        for block in self.tool_blocks.values() {
            if !block.stopped {
                return Err(format!(
                    "Anthropic stream ended before tool_use {:?} completed",
                    block.name
                ));
            }
            self.validated_tool_arguments(block)?;
        }
        let mut out = self.chunk(json!({}), Some(&self.stop_reason));
        out.extend(format!("data: {}\n\n", json!({"object":"chat.completion.chunk","model":self.model,"choices":[],"usage":self.usage()})).into_bytes());
        out.extend_from_slice(b"data: [DONE]\n\n");
        Ok(out)
    }
}

/// POST /v1/audio/transcriptions — OpenAI-compatible speech-to-text for the IDE's voice input.
/// Auth via a Michael API key (or the login JWT), same as chat. Forwards the uploaded clip to the
/// configured Whisper upstream (Groq's free whisper-large-v3 by default) and returns its JSON
/// verbatim. Does NOT use the DB `models` connections — those aggregators don't do audio (404).
pub async fn audio_transcriptions(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    // ---- auth (mirror chat_completions: api_keys row, else login JWT) ----
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let _uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM api_keys WHERE api_key = $1",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    {
        Some(u) => u,
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };

    if state.cfg.transcribe_api_key.is_empty() {
        return Err(AppError::bad("转写服务未配置"));
    }

    // ---- read the multipart form: file (required) + optional language ----
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name = "speech.m4a".to_string();
    let mut content_type = "audio/mp4".to_string();
    let mut language: Option<String> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad(format!("表单解析失败: {e}")))?
    {
        match field.name().unwrap_or("") {
            "file" => {
                if let Some(n) = field.file_name() {
                    if !n.is_empty() {
                        file_name = n.to_string();
                    }
                }
                if let Some(ct) = field.content_type() {
                    if ct.contains('/') {
                        content_type = ct.to_string();
                    }
                }
                file_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| AppError::bad(format!("读取音频失败: {e}")))?
                        .to_vec(),
                );
            }
            "language" => language = field.text().await.ok().filter(|s| !s.is_empty()),
            _ => {
                let _ = field.bytes().await;
            }
        }
    }
    let file_bytes = file_bytes.ok_or_else(|| AppError::bad("缺少音频文件"))?;
    if file_bytes.len() < 256 {
        return Err(AppError::bad("音频太短或为空"));
    }

    // ---- forward to the Whisper upstream ----
    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str(&content_type)
        .map_err(|e| AppError::bad(format!("音频类型无效: {e}")))?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", state.cfg.transcribe_model.clone())
        .text("response_format", "json");
    if let Some(l) = language {
        form = form.text("language", l);
    }

    let resp = GW_HTTP
        .post(&state.cfg.transcribe_url)
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", state.cfg.transcribe_api_key),
        )
        .multipart(form)
        .send()
        .await
        .map_err(|e| AppError::bad(format!("转写上游连接失败: {e}")))?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let body = resp
        .bytes()
        .await
        .map_err(|e| AppError::bad(format!("转写上游读取失败: {e}")))?;

    Ok(Response::builder()
        .status(status)
        .header(axum::http::header::CONTENT_TYPE, ctype)
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response()))
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM api_keys WHERE api_key = $1",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    {
        Some(u) => {
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE api_key = $1")
                .bind(&token)
                .execute(&state.db)
                .await;
            u
        }
        // Also accept the login JWT directly (the IDE authenticates with it).
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };

    if !body.is_object() {
        return Err(AppError::bad("请求体需为 JSON 对象"));
    }
    // Strip cache_control: fingerprints proved the prefix is byte-stable yet this relay
    // still bills cache WRITES (1.25×) almost every call → it's a pure premium here.
    // Flat 1× is cheaper. (Real fix = a reliable-caching upstream; see fn doc.)
    strip_cache_control(&mut body);
    // L0 server-side assembly: when the IDE opts in (x-ide-mode header), inject the system
    // prompt + requested tool schemas from the registry HERE, so the client ships neither.
    // No header → no-op (existing behavior untouched).
    crate::prompts::assemble_into(&headers, &mut body);
    let model_id = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::bad("缺少 model"))?;

    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let candidates: Vec<Model> = conns
        .into_iter()
        .filter(|m| allowed_ids(m).contains(&model_id))
        .collect();
    let route_count = candidates.len();
    let primary_conn = candidates
        .first()
        .cloned()
        .ok_or_else(|| AppError::bad(format!("模型 {model_id} 不可用")))?;

    // Refill the 5h30m window + reset the weekly counter when due.
    sqlx::query(
        "UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '5 hours 30 minutes' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1",
    )
    .bind(uid)
    .execute(&state.db)
    .await?;

    // Access gate: active-membership quota (window/total/weekly) OR pay-as-you-go credits.
    let (plan, plan_exp, q_total, q_window, q_weekly_cap, q_week_used, credits): (String, Option<chrono::DateTime<chrono::Utc>>, i64, i64, i64, i64, i64) =
        sqlx::query_as("SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, quota_weekly_cap_cents, quota_week_used_cents, credits_cents FROM users WHERE id = $1")
            .bind(uid)
            .fetch_one(&state.db)
            .await?;
    let plan_active = plan != "none" && plan_exp.is_none_or(|e| e > chrono::Utc::now());
    let quota_ok = plan_active
        && q_total > 0
        && q_window > 0
        && (q_weekly_cap == 0 || q_week_used < q_weekly_cap);
    let use_quota = quota_ok;
    if !quota_ok && credits <= 0 {
        let msg = if plan_active && q_total <= 0 {
            "总额度已用完"
        } else if plan_active && q_window <= 0 {
            "本时段额度已用完，请等待刷新（每 5.5 小时）"
        } else if plan_active && q_weekly_cap > 0 && q_week_used >= q_weekly_cap {
            "本周额度已用完"
        } else {
            "请先开通会员或充值额度"
        };
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: msg.into(),
        });
    }

    let streaming = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // Ensure the upstream emits a final usage chunk so streaming billing can read
    // real (cache-discounted) tokens instead of falling back to a flat fee. Only add
    // it when the client didn't set stream_options itself (the IDE already does; this
    // covers third-party OpenAI-compatible clients of this gateway).
    if streaming {
        if let Some(obj) = body.as_object_mut() {
            obj.entry("stream_options")
                .or_insert_with(|| serde_json::json!({ "include_usage": true }));
        }
    }
    // ── Gateway response cache ────────────────────────────────────────────────
    // Identical request (same model + messages + params) → serve the stored
    // response: NO upstream call, 0 cost. Real caching the user controls, working
    // for EVERY model regardless of whether the upstream caches. Best-effort: any
    // Redis hiccup or miss just falls through to a normal upstream call. The quota
    // gate already ran above, so a hit still requires access — it just costs nothing.
    let ckey = gw_cache_key(&body);
    {
        let mut rconn = state.redis.clone();
        let hit: Option<Vec<u8>> = redis::cmd("GET")
            .arg(&ckey)
            .query_async(&mut rconn)
            .await
            .ok()
            .flatten();
        if let Some(bytes) = hit.filter(|bytes| response_cache_safe(bytes)) {
            bill(
                &state,
                uid,
                primary_conn.id,
                0,
                use_quota,
                &BillTokens {
                    model_name: model_id.clone(),
                    ..Default::default()
                },
            )
            .await; // record a 0-cost cache hit
            let ct = if streaming {
                "text/event-stream"
            } else {
                "application/json"
            };
            return Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, ct)
                .header("x-gateway-cache", "hit")
                .header("cache-control", "no-cache")
                .body(Body::from(bytes))
                .map_err(|e| AppError::internal(e.to_string()));
        }
    }
    // ── max_tokens guardrail for thinking (all protocols) ───────────────────
    // Chinese aggregators (zyz etc.) convert reasoning_effort / thinking to Anthropic thinking
    // with budget_tokens; if max_tokens < budget_tokens the upstream rejects. The native
    // Anthropic path (oai_to_anthropic) handles this, but OpenAI-protocol connections pass
    // body through unchanged — so bump max_tokens here before the fork.
    {
        let has_thinking = body.get("thinking").is_some()
            || body
                .get("reasoning_effort")
                .and_then(|v| v.as_str())
                .is_some_and(|e| !e.is_empty() && e != "off");
        if has_thinking {
            let budget = body
                .pointer("/thinking/budget_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let min_mt = (budget + 8000).max(32000);
            let cur_mt = body.get("max_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
            if cur_mt < min_mt {
                if let Some(obj) = body.as_object_mut() {
                    obj.insert("max_tokens".into(), json!(min_mt.min(128000)));
                }
            }
        }
    }
    let tool_argument_rules = tool_argument_rules(&body);
    // Pooled client (warm keep-alive connections) instead of a fresh handshake
    // per request. Streaming stays open-ended; non-streaming gets a sane cap.
    //
    // Upstream providers (zyz et al.) intermittently return 502/503/504/429 or
    // drop a kept-alive connection mid-flight — the user just sees "网关又出问题".
    // Retry such *transient* failures with bounded exponential backoff so a
    // short provider flap is absorbed instead of surfaced. We only retry BEFORE
    // streaming the body has started (a send error or a bad status line), so no
    // half-streamed response is ever double-sent, and billing still happens once,
    // after success. Failed routes get a short in-memory cooldown so the next
    // request prefers another same-model route when the admin has configured one.
    let model_name = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("该模型")
        .to_string();
    // Map an upstream error to a friendly, actionable Chinese message.
    fn friendly_upstream(status: u16, low: &str) -> String {
        if low.contains("forbidden") || low.contains("未授权") {
            "上游暂不可用（供应商未授权 / 账户异常）。请换个模型，或联系模型供应商开通 / 续费。"
                .into()
        } else if low.contains("no available") || low.contains("没有可用") {
            "上游暂无可用账号。请换个模型，或稍后再试。".into()
        } else if status == 429
            || low.contains("rate")
            || low.contains("frequent")
            || low.contains("过于频繁")
        {
            "请求过于频繁，请稍后再试。".into()
        } else if status == 401 || low.contains("unauthorized") || low.contains("invalid api key") {
            "上游密钥无效。请在后台「模型系统」更新该连接的 API Key。".into()
        } else if status == 400 {
            let detail = safe_upstream_error_excerpt(low);
            if detail.is_empty() {
                "上游拒绝了请求（400），但没有返回更细原因。".into()
            } else {
                format!("上游拒绝了请求（400）：{detail}")
            }
        } else {
            "上游暂时不可用，请换个模型或稍后再试。".into()
        }
    }
    // Send with retry — but ONLY retry *transient* failures (502/503/504/429 or a
    // dropped connection). "forbidden / unauthorized / no available account" is
    // PERSISTENT: retrying just makes the user wait ~15s for the same error, so we
    // fail FAST with a friendly message. Billing only happens after a success.
    let (resp, conn) = {
        let mut success = None;
        let mut err_status = 502u16;
        let mut err_low = String::new();
        let mut selected_conn = None;
        let mut attempted_sends = 0u32;
        let now = Instant::now();
        let mut ordered_candidates: Vec<&Model> = Vec::with_capacity(candidates.len());
        let mut cooled_candidates: Vec<&Model> = Vec::new();
        for candidate in &candidates {
            if route_count > 1 && route_cooldown_remaining(candidate.id, now).is_some() {
                cooled_candidates.push(candidate);
            } else {
                ordered_candidates.push(candidate);
            }
        }
        ordered_candidates.extend(cooled_candidates);

        'routes: for candidate in ordered_candidates {
            // protocol="anthropic" → native /v1/messages with translated OpenAI⇄Anthropic body;
            // else OpenAI-compat /chat/completions passthrough. Multiple active connections may
            // expose the same model id; try the next line when the current one is dead instead of
            // failing the whole IDE request on the first 502.
            let candidate_anthropic = candidate.protocol == "anthropic";
            let candidate_url = if candidate_anthropic {
                format!("{}/messages", api_base(&candidate.base_url))
            } else {
                format!("{}/chat/completions", api_base(&candidate.base_url))
            };
            let candidate_upstream_body = if candidate_anthropic {
                match oai_to_anthropic(&body) {
                    Ok(v) => v,
                    Err(err) => {
                        err_status = 400;
                        err_low =
                            format!("Anthropic request conversion failed: {err}").to_lowercase();
                        continue;
                    }
                }
            } else {
                serde_json::Value::Null
            };
            let mut route_attempts = 0u32;
            let mut route_failed_transient = false;
            for attempt in 0u32..CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE {
                let req0 = GW_HTTP.post(&candidate_url);
                let mut req = if candidate_anthropic {
                    req0.header("x-api-key", &candidate.api_key)
                        .header("anthropic-version", "2023-06-01")
                        .json(&candidate_upstream_body)
                } else {
                    req0.header("Authorization", format!("Bearer {}", candidate.api_key))
                        .json(&body)
                };
                if !streaming {
                    req = req.timeout(Duration::from_secs(120));
                }
                route_attempts += 1;
                attempted_sends += 1;
                match req.send().await {
                    Ok(r) if r.status().is_success() => {
                        success = Some(r);
                        selected_conn = Some(candidate.clone());
                        break 'routes;
                    }
                    Ok(r) => {
                        err_status = r.status().as_u16();
                        err_low = r.text().await.unwrap_or_default().to_lowercase();
                        let persistent = err_status == 401
                            || err_status == 403
                            || err_low.contains("forbidden")
                            || err_low.contains("unauthorized")
                            || err_low.contains("invalid api key")
                            || err_low.contains("未授权")
                            || err_low.contains("no available")
                            || err_low.contains("没有可用");
                        let transient = matches!(err_status, 502 | 503 | 504 | 429);
                        if persistent || !transient {
                            break;
                        }
                        if attempt + 1 >= CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE {
                            route_failed_transient = true;
                            break;
                        }
                        tokio::time::sleep(chat_upstream_retry_delay(attempt)).await;
                    }
                    // A send error means the request almost certainly never reached the
                    // server (incl. a stale pooled connection) — safe to re-send.
                    Err(e) => {
                        err_status = 502;
                        err_low = e.to_string().to_lowercase();
                        if attempt + 1 >= CHAT_UPSTREAM_MAX_ATTEMPTS_PER_ROUTE {
                            route_failed_transient = true;
                            break;
                        }
                        tokio::time::sleep(chat_upstream_retry_delay(attempt)).await;
                    }
                }
            }
            if route_failed_transient {
                mark_route_cooldown(candidate.id);
                tracing::warn!(
                    model = %model_name,
                    provider = %candidate.provider,
                    label = %candidate.label,
                    attempts = route_attempts,
                    status = err_status,
                    "chat upstream route exhausted transient retries; cooling route"
                );
            }
        }
        match (success, selected_conn) {
            (Some(r), Some(c)) => (r, c),
            (None, _) => {
                let msg = format!(
                    "【{model_name}】{}{}",
                    friendly_upstream(err_status, &err_low),
                    chat_upstream_attempt_suffix(route_count, attempted_sends, err_status)
                );
                if headers.contains_key("x-ide-mode") {
                    return Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .header(
                            axum::http::header::CONTENT_TYPE,
                            "text/plain; charset=utf-8",
                        )
                        .body(Body::from(msg))
                        .map_err(|e| AppError::internal(e.to_string()));
                }
                return Err(AppError {
                    status: StatusCode::BAD_GATEWAY,
                    msg,
                });
            }
            _ => unreachable!("success response and selected connection are set together"),
        }
    };
    let status = resp.status();
    let anthropic = conn.protocol == "anthropic";

    if streaming {
        // Pre-compute input token estimate BEFORE the spawn (body is consumed afterwards).
        let est_in_tok = estimate_input_tokens(&body);
        // 深思考请求（xhigh/max/带 thinking 预算）静默期可超 3 分钟：固定 180s 的上游
        // 空闲斩会在客户端窗口放宽后成为顶层杀手，这里跟档位一起放宽。
        let deep_thinking = body
            .get("reasoning_effort")
            .and_then(|v| v.as_str())
            .map(|e| e.eq_ignore_ascii_case("max") || e.eq_ignore_ascii_case("xhigh"))
            .unwrap_or(false)
            || body
                .get("thinking")
                .and_then(|t| t.get("budget_tokens"))
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                > 0;
        // Tee the upstream SSE: forward bytes to the client UNCHANGED while
        // accumulating the full stream so a complete response can be cached. Billing is
        // REAL: the trailing include_usage chunk gives true token counts → official price
        // × 倍率 (see compute_cost). Cache hits bill 0 (handled at the cache-hit return
        // above). 180s idle guard preserved inline.
        let ct = resp
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/event-stream")
            .to_string();
        let st = state.clone();
        let cid = conn.id;
        let rate = conn.rate;
        let admin_in = conn.input_price;
        let admin_out = conn.output_price;
        let cache_read_price = conn.cache_read_price;
        let cache_create_price = conn.cache_create_price;
        let bmode = conn.billing_mode.clone();
        let percall = conn.per_call_cents;
        let req_model = model_id.clone();
        let (model_in, model_out) = model_price_override(&conn.model_prices, &model_id);
        let ckey_task = ckey.clone();
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(32);
        tokio::spawn(async move {
            use futures_util::StreamExt;
            let mut upstream = Box::pin(resp.bytes_stream());
            // 180s (was 30s): a thinking model pauses to reason / composes a long file write
            // silently → the 30s guard cut the stream mid-tool-call (truncated args → empty
            // write "内容为空"). 180s lets those through while still bounding a real hang.
            // 深思考档（xhigh/max/thinking 预算）放宽到 600s——否则客户端 180s 窗和这里打平，
            // 超 3 分钟的静默深思仍会被网关先掐。
            let idle = std::time::Duration::from_secs(if deep_thinking { 600 } else { 180 });
            let mut acc: Vec<u8> = Vec::new(); // OpenAI-shape SSE bytes, for the response cache (capped 1MB)
                                               // Bounded tail for OpenAI usage extraction (the include_usage chunk is the LAST event;
                                               // a >1MB response would miss it in the capped acc). Unused on the anthropic path — there
                                               // usage comes from the converter's accumulated counts.
            let mut tail: Vec<u8> = Vec::new();
            let mut complete = false;
            let mut client_closed = false;
            let mut stream_failure: Option<String> = None;
            // anthropic connections: translate the upstream Anthropic SSE → OpenAI SSE on the fly.
            let mut conv = if anthropic {
                Some(AnthSse::with_tool_argument_rules(
                    &req_model,
                    tool_argument_rules.clone(),
                ))
            } else {
                None
            };
            let mut openai_validator = if anthropic {
                None
            } else {
                Some(OpenAiSseValidator::with_tool_argument_rules(
                    tool_argument_rules,
                ))
            };
            // SSE heartbeat: Chinese carrier NATs kill TCP connections idle >30-60s.
            // During model "thinking" the upstream is silent → zero bytes flow to the
            // client → NAT drops it → "网络波动". Fix: send an SSE comment (`: ping\n\n`)
            // every 15s of upstream silence. SSE comments are ignored by compliant parsers.
            let hb_interval = std::time::Duration::from_secs(15);
            let mut last_data = tokio::time::Instant::now();
            loop {
                match tokio::time::timeout(hb_interval, upstream.next()).await {
                    Ok(Some(Ok(chunk))) => {
                        last_data = tokio::time::Instant::now();
                        let fwd: Vec<u8> = match conv.as_mut() {
                            Some(c) => match c.push(chunk.as_ref()) {
                                Ok(fwd) => fwd,
                                Err(err) => {
                                    stream_failure = Some(err);
                                    break;
                                }
                            },
                            None => {
                                if let Some(validator) = openai_validator.as_mut() {
                                    if let Err(err) = validator.push(chunk.as_ref()) {
                                        stream_failure = Some(err);
                                        break;
                                    }
                                }
                                chunk.to_vec()
                            }
                        };
                        if !fwd.is_empty() {
                            if acc.len() < 1_000_000 {
                                acc.extend_from_slice(&fwd);
                            }
                            if conv.is_none() {
                                tail.extend_from_slice(&fwd);
                                if tail.len() > 131_072 {
                                    let cut = tail.len() - 65_536;
                                    tail.drain(0..cut);
                                }
                            }
                            if tx.send(Ok(axum::body::Bytes::from(fwd))).await.is_err() {
                                client_closed = true;
                                break;
                            }
                        }
                    }
                    Ok(None) => {
                        if conv.is_some() {
                            // Anthropic completion is validated by AnthSse::finish below.
                            complete = true;
                        } else {
                            match openai_validator
                                .as_ref()
                                .expect("OpenAI validator")
                                .finish()
                            {
                                Ok(()) => complete = true,
                                Err(err) => stream_failure = Some(err),
                            }
                        }
                        break;
                    }
                    Ok(Some(Err(err))) => {
                        stream_failure = Some(format!("upstream stream read failed: {err}"));
                        break;
                    }
                    Err(_elapsed) => {
                        if last_data.elapsed() >= idle {
                            stream_failure = Some(format!(
                                "upstream stream stalled for {} seconds",
                                idle.as_secs()
                            ));
                            break; // real stall — upstream dead for 180s
                        }
                        // Send SSE heartbeat to keep the client connection alive
                        if tx
                            .send(Ok(axum::body::Bytes::from_static(b": ping\n\n")))
                            .await
                            .is_err()
                        {
                            client_closed = true;
                            break;
                        }
                    }
                }
            }
            // anthropic: flush the terminal OpenAI chunks (finish_reason + usage + [DONE]) and bill
            // from the converter's accumulated (cache-aware) usage. openai: bill from the trailing
            // include_usage chunk. per_call mode ignores usage; rate mode → 0 if no usage/prices.
            // FALLBACK: when the provider reports no usage (stream broke, no trailing chunk), estimate
            // from the request body and accumulated response bytes so calls are never silently free.
            let (usage, is_estimated) = if let Some(c) = conv.as_ref() {
                if complete {
                    match c.finish() {
                        Ok(fin) => {
                            if acc.len() < 1_000_000 {
                                acc.extend_from_slice(&fin);
                            }
                            if tx.send(Ok(axum::body::Bytes::from(fin))).await.is_err() {
                                client_closed = true;
                            }
                        }
                        Err(err) => {
                            complete = false;
                            stream_failure = Some(err);
                        }
                    }
                } else if stream_failure.is_none() && !client_closed {
                    stream_failure = Some(
                        "Anthropic upstream stream ended before protocol completion".to_string(),
                    );
                }
                (c.usage(), false)
            } else {
                match parse_usage_from_sse(&tail) {
                    Some(u) => (u, false),
                    None if !acc.is_empty() => {
                        let est_out = estimate_output_tokens(acc.len());
                        tracing::warn!(
                            "[billing] no usage from provider, estimating: in={} out={} model={}",
                            est_in_tok,
                            est_out,
                            req_model
                        );
                        (estimated_usage(est_in_tok, est_out), true)
                    }
                    None => (json!({}), false), // truly empty response, bill 0
                }
            };
            if let Some(err) = stream_failure.take() {
                complete = false;
                if !client_closed {
                    tracing::warn!(model = %req_model, error = %err, "upstream model stream failed protocol validation");
                    let _ = tx
                        .send(Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            err,
                        )))
                        .await;
                }
            }
            let cost = resolve_cost(
                &bmode,
                percall,
                Some(&usage),
                &req_model,
                rate,
                admin_in,
                admin_out,
                cache_read_price,
                cache_create_price,
                model_in,
                model_out,
            );
            let tokens = extract_bill_tokens(Some(&usage), &req_model, is_estimated);
            // Cache the FULL (OpenAI-shape) stream for identical future requests (only when complete).
            if complete && !acc.is_empty() && acc.len() < 1_000_000 && response_cache_safe(&acc) {
                let mut rconn = st.redis.clone();
                let _: Result<(), redis::RedisError> = redis::cmd("SET")
                    .arg(&ckey_task)
                    .arg(acc)
                    .arg("EX")
                    .arg(3600i64)
                    .query_async(&mut rconn)
                    .await;
            }
            bill(&st, uid, cid, cost, use_quota, &tokens).await;
        });
        let body_stream = futures_util::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        });
        let out = Response::builder()
            .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
            .header(axum::http::header::CONTENT_TYPE, ct)
            .header("cache-control", "no-cache")
            .body(Body::from_stream(body_stream))
            .map_err(|e| AppError::internal(e.to_string()))?;
        Ok(out)
    } else {
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({ "error": "上游返回非 JSON" }));
        if !status.is_success() {
            return Err(AppError {
                status: StatusCode::BAD_GATEWAY,
                msg: format!("模型供应商错误 {}: {}", status.as_u16(), raw),
            });
        }
        // Anthropic native response → OpenAI shape for the IDE (usage kept in a form compute_cost bills).
        let mut data = if anthropic {
            anthropic_to_oai(&raw, &model_id)
        } else {
            raw
        };
        // Repair upstream's malformed `tool_calls[*].function.arguments`. Some relays
        // (Claude→OpenAI-compat translators) concat the initial empty-arg placeholder
        // `"{}"` with the actual JSON, producing `'{}{"path":"."}'` which clients then
        // parse as `{}` (silent empty args). Strip leading `{}` when followed by `{`.
        fix_tool_call_arguments(&mut data);
        // Cache the successful response for identical future requests.
        if let Ok(bytes) = serde_json::to_vec(&data) {
            if !bytes.is_empty() && bytes.len() < 1_000_000 && response_cache_safe(&bytes) {
                let mut rconn = state.redis.clone();
                let _: Result<(), redis::RedisError> = redis::cmd("SET")
                    .arg(&ckey)
                    .arg(bytes)
                    .arg("EX")
                    .arg(3600i64)
                    .query_async(&mut rconn)
                    .await;
            }
        }
        let (cost, tokens) = if is_image_gen_model(&model_id) {
            let per = if conn.per_call_cents > 0 {
                conn.per_call_cents
            } else {
                (30.0 * conn.rate).round() as i64
            };
            (
                per.clamp(0, 5000),
                BillTokens {
                    model_name: model_id.clone(),
                    ..Default::default()
                },
            )
        } else {
            let usage_val = data.get("usage");
            let is_est = usage_val.is_none()
                || usage_val.is_some_and(|u| {
                    u.get("prompt_tokens").and_then(|x| x.as_i64()).unwrap_or(0) == 0
                        && u.get("completion_tokens")
                            .and_then(|x| x.as_i64())
                            .unwrap_or(0)
                            == 0
                        && u.get("input_tokens").and_then(|x| x.as_i64()).unwrap_or(0) == 0
                });
            let effective_usage = if is_est {
                let est_in = estimate_input_tokens(&body);
                let resp_str = serde_json::to_string(&data).unwrap_or_default();
                let est_out = (resp_str.len() as i64) / 4;
                tracing::warn!(
                    "[billing] non-stream no usage, estimating: in={} out={} model={}",
                    est_in,
                    est_out,
                    model_id
                );
                estimated_usage(est_in, est_out)
            } else {
                usage_val.cloned().unwrap_or_else(|| json!({}))
            };
            let (model_in, model_out) = model_price_override(&conn.model_prices, &model_id);
            let cost = resolve_cost(
                &conn.billing_mode,
                conn.per_call_cents,
                Some(&effective_usage),
                &model_id,
                conn.rate,
                conn.input_price,
                conn.output_price,
                conn.cache_read_price,
                conn.cache_create_price,
                model_in,
                model_out,
            );
            (
                cost,
                extract_bill_tokens(Some(&effective_usage), &model_id, is_est),
            )
        };
        bill(&state, uid, conn.id, cost, use_quota, &tokens).await;
        Ok(Json(data).into_response())
    }
}

/// OpenAI Responses API proxy — forwards POST /v1/responses to the upstream that
/// owns the requested model. Used by the IDE's image-generation fallback chain:
/// 中转站 like LaoZhang/Codex wrap gpt-image-2 behind ChatGPT Plus accounts via
/// this endpoint with the image_generation built-in tool.
///
/// Smart model rewrite: if the IDE sends `model=gpt-image-2`, we route to the
/// matching connection (UI生图密钥) BUT swap body.model to `gpt-5.4` before
/// forwarding — because the Responses API requires a mainline text model in
/// the `model` field (the image model itself is fixed by `tools.image_generation`).
pub async fn responses_proxy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM api_keys WHERE api_key = $1",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    {
        Some(u) => {
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE api_key = $1")
                .bind(&token)
                .execute(&state.db)
                .await;
            u
        }
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };

    let model_id = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::bad("缺少 model"))?;

    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let conn = conns
        .into_iter()
        .find(|m| allowed_ids(m).contains(&model_id))
        .ok_or_else(|| AppError::bad(format!("模型 {model_id} 不可用")))?;

    // Same quota refill + check as image_generations.
    sqlx::query(
        "UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '5 hours 30 minutes' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1",
    )
    .bind(uid)
    .execute(&state.db)
    .await?;

    let (plan, plan_exp, q_total, q_window, q_weekly_cap, q_week_used, credits): (
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, \
         quota_weekly_cap_cents, quota_week_used_cents, credits_cents FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    let plan_active = plan != "none" && plan_exp.is_none_or(|e| e > chrono::Utc::now());
    let quota_ok = plan_active
        && q_total > 0
        && q_window > 0
        && (q_weekly_cap == 0 || q_week_used < q_weekly_cap);
    let use_quota = quota_ok;
    if !quota_ok && credits <= 0 {
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: "请先开通会员或充值额度".into(),
        });
    }

    // Always ensure image_generation tool is present for image models.
    let is_image_model = model_id.to_lowercase().contains("gpt-image")
        || model_id.to_lowercase().contains("dall-e")
        || model_id.to_lowercase().contains("dall_e");
    if is_image_model {
        if let Some(obj) = body.as_object_mut() {
            let has_image_tool = obj
                .get("tools")
                .and_then(|t| t.as_array())
                .map(|a| a.iter().any(|t| t["type"] == "image_generation"))
                .unwrap_or(false);
            if !has_image_tool {
                let mut tools = obj
                    .get("tools")
                    .and_then(|t| t.as_array())
                    .cloned()
                    .unwrap_or_default();
                tools.push(serde_json::json!({"type": "image_generation"}));
                obj.insert("tools".into(), serde_json::json!(tools));
            }
        }
    }

    let url = format!("{}/responses", api_base(&conn.base_url));

    // Two-stage attempt for image models:
    //   stage 1: forward AS-IS — relay routes to real gpt-image-2 (full HD output).
    //   stage 2: when stage 1 fails with "no Plus OAuth account" (relay's HD account
    //   pool is empty), swap model → "gpt-5.4" and retry — relay's mainline-wrap
    //   path doesn't need a Plus account but caps output at ~940×627.
    // For non-image models, stage 2 is skipped (no model swap makes sense).
    async fn send_once(
        url: &str,
        api_key: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, (u16, String)> {
        for attempt in 0u32..3 {
            match GW_HTTP
                .post(url)
                .header("Authorization", format!("Bearer {api_key}"))
                .json(body)
                .timeout(std::time::Duration::from_secs(180))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => return Ok(r),
                Ok(r) => {
                    let st = r.status().as_u16();
                    let txt = r.text().await.unwrap_or_default();
                    let transient = matches!(st, 502 | 503 | 504 | 429);
                    if !transient || attempt == 2 {
                        return Err((st, txt));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    if attempt == 2 {
                        return Err((0, e.to_string()));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
        Err((0, "exhausted".into()))
    }

    let resp = match send_once(&url, &conn.api_key, &body).await {
        Ok(r) => r,
        Err((st, msg)) if is_image_model && msg.to_lowercase().contains("no active plus oauth") => {
            // HD pool empty → fall back to mainline-wrap (model=gpt-5.4) for low-res but functional output.
            tracing::info!(
                "[responses] {model_id} HD pool empty, falling back to gpt-5.4 mainline-wrap"
            );
            if let Some(obj) = body.as_object_mut() {
                obj.insert("model".into(), serde_json::json!("gpt-5.4"));
            }
            match send_once(&url, &conn.api_key, &body).await {
                Ok(r) => r,
                Err((st2, msg2)) => {
                    return Err(AppError {
                        status: StatusCode::BAD_GATEWAY,
                        msg: format!(
                            "【{model_id}】responses 双路径都失败: HD={} | mainline={}: {}",
                            st,
                            st2,
                            msg2.chars().take(150).collect::<String>()
                        ),
                    });
                }
            }
        }
        Err((st, msg)) => {
            return Err(AppError {
                status: StatusCode::BAD_GATEWAY,
                msg: format!(
                    "【{model_id}】responses 上游不可用 ({}): {}",
                    st,
                    msg.chars().take(200).collect::<String>()
                ),
            });
        }
    };

    let data: serde_json::Value = resp
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({"error": "上游返回非 JSON"}));
    let has_error = data.get("error").is_some();

    // Bill: image models = per-image (per_call_cents if set, else 30分×倍率), text = per-token.
    if !has_error {
        if is_image_gen_model(&model_id) {
            let mut n_images = data
                .get("output")
                .and_then(|o| o.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter(|i| i["type"] == "image_generation_call")
                        .count() as f64
                })
                .unwrap_or(0.0);
            // Non-OpenAI image models (e.g. gemini-*-image) may not emit `image_generation_call`;
            // a successful image call still costs → bill at least 1 image, never $0.
            if n_images == 0.0 {
                n_images = 1.0;
            }
            let cost = if conn.per_call_cents > 0 {
                (conn.per_call_cents as f64 * n_images).round().min(5000.0) as i64
            } else {
                (30.0 * n_images * conn.rate).round().min(5000.0) as i64
            };
            bill(
                &state,
                uid,
                conn.id,
                cost,
                use_quota,
                &BillTokens {
                    model_name: model_id.clone(),
                    ..Default::default()
                },
            )
            .await;
        } else {
            let (model_in, model_out) = model_price_override(&conn.model_prices, &model_id);
            let cost = resolve_cost(
                &conn.billing_mode,
                conn.per_call_cents,
                data.get("usage"),
                &model_id,
                conn.rate,
                conn.input_price,
                conn.output_price,
                conn.cache_read_price,
                conn.cache_create_price,
                model_in,
                model_out,
            );
            let tokens = extract_bill_tokens(data.get("usage"), &model_id, false);
            bill(&state, uid, conn.id, cost, use_quota, &tokens).await;
        }
    }

    Ok(Json(data).into_response())
}

/// Image generation endpoint — proxies to upstream /images/generations.
/// Same auth + quota as chat_completions; bills per-image (official price × 倍率).
pub async fn image_generations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Response, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::unauthorized("缺少 API Key"))?;
    let uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM api_keys WHERE api_key = $1",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await?
    {
        Some(u) => {
            let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE api_key = $1")
                .bind(&token)
                .execute(&state.db)
                .await;
            u
        }
        None => crate::auth::user_from_jwt(&state.cfg, &token)
            .ok_or_else(|| AppError::unauthorized("登录已失效或密钥无效"))?,
    };

    let model_id = body
        .get("model")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| AppError::bad("缺少 model"))?;

    let conns = sqlx::query_as::<_, Model>(
        "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let conn = conns
        .into_iter()
        .find(|m| allowed_ids(m).contains(&model_id))
        .ok_or_else(|| AppError::bad(format!("模型 {model_id} 不可用")))?;

    // Quota refill + check (same as chat_completions).
    sqlx::query(
        "UPDATE users SET \
         quota_window_cents = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN LEAST(quota_window_cap_cents, quota_total_cents) ELSE quota_window_cents END, \
         quota_window_reset_at = CASE WHEN (quota_window_reset_at IS NULL OR quota_window_reset_at <= now()) AND quota_total_cents > 0 THEN now() + interval '5 hours 30 minutes' ELSE quota_window_reset_at END, \
         quota_week_used_cents = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN 0 ELSE quota_week_used_cents END, \
         quota_week_reset_at = CASE WHEN quota_week_reset_at IS NULL OR quota_week_reset_at <= now() THEN now() + interval '7 days' ELSE quota_week_reset_at END \
         WHERE id = $1",
    )
    .bind(uid)
    .execute(&state.db)
    .await?;

    let (plan, plan_exp, q_total, q_window, q_weekly_cap, q_week_used, credits): (
        String,
        Option<chrono::DateTime<chrono::Utc>>,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT plan, plan_expires_at, quota_total_cents, quota_window_cents, \
         quota_weekly_cap_cents, quota_week_used_cents, credits_cents FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    let plan_active = plan != "none" && plan_exp.is_none_or(|e| e > chrono::Utc::now());
    let quota_ok = plan_active
        && q_total > 0
        && q_window > 0
        && (q_weekly_cap == 0 || q_week_used < q_weekly_cap);
    let use_quota = quota_ok;
    if !quota_ok && credits <= 0 {
        let msg = if plan_active && q_total <= 0 {
            "总额度已用完"
        } else if plan_active && q_window <= 0 {
            "本时段额度已用完，请等待刷新（每 5.5 小时）"
        } else if plan_active && q_weekly_cap > 0 && q_week_used >= q_weekly_cap {
            "本周额度已用完"
        } else {
            "请先开通会员或充值额度"
        };
        return Err(AppError {
            status: StatusCode::PAYMENT_REQUIRED,
            msg: msg.into(),
        });
    }

    // Proxy to upstream /images/generations with retry for transient failures.
    let url = format!("{}/images/generations", api_base(&conn.base_url));
    let resp = {
        let mut success = None;
        let mut last_err = String::new();
        for attempt in 0u32..3 {
            match GW_HTTP
                .post(&url)
                .header("Authorization", format!("Bearer {}", conn.api_key))
                .json(&body)
                .timeout(std::time::Duration::from_secs(120))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => {
                    success = Some(r);
                    break;
                }
                Ok(r) => {
                    let st = r.status().as_u16();
                    last_err = r.text().await.unwrap_or_default();
                    let transient = matches!(st, 502 | 503 | 504 | 429);
                    if !transient || attempt == 2 {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    last_err = e.to_string();
                    if attempt == 2 {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
        match success {
            Some(r) => r,
            None => {
                return Err(AppError {
                    status: StatusCode::BAD_GATEWAY,
                    msg: format!(
                        "【{model_id}】生图上游不可用: {}",
                        last_err.chars().take(200).collect::<String>()
                    ),
                });
            }
        }
    };

    let mut data: serde_json::Value = resp
        .json()
        .await
        .unwrap_or_else(|_| json!({"error": "上游返回非 JSON"}));

    // Async task support: some upstreams queue large-size requests and return a task_id.
    if data.get("status").and_then(|s| s.as_str()) == Some("queued")
        || data.get("status").and_then(|s| s.as_str()) == Some("running")
    {
        if let Some(task_id) = data
            .get("task_id")
            .and_then(|t| t.as_str())
            .map(String::from)
        {
            let poll_url = format!(
                "{}/images/generations/{}",
                api_base(&conn.base_url),
                task_id
            );
            for _ in 0..60 {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                if let Ok(pr) = GW_HTTP
                    .get(&poll_url)
                    .header("Authorization", format!("Bearer {}", conn.api_key))
                    .timeout(std::time::Duration::from_secs(15))
                    .send()
                    .await
                {
                    if let Ok(pv) = pr.json::<serde_json::Value>().await {
                        let st = pv.get("status").and_then(|s| s.as_str()).unwrap_or("");
                        if st == "failed" {
                            data = pv;
                            break;
                        }
                        if st == "completed" {
                            data = pv;
                            break;
                        }
                    }
                }
            }
        }
    }

    // Fix relative URLs: some upstreams return "/api/v1/gen/..." instead of full URLs.
    if let Some(arr) = data.get_mut("data").and_then(|d| d.as_array_mut()) {
        let origin = conn.base_url.trim_end_matches('/');
        for item in arr.iter_mut() {
            if let Some(u) = item.get("url").and_then(|v| v.as_str()).map(String::from) {
                if u.starts_with('/') {
                    item["url"] = json!(format!("{}{}", origin, u));
                }
            }
        }
    }

    let has_error = data.get("error").is_some()
        || data.get("status").and_then(|s| s.as_str()) == Some("failed");

    // Bill per image: per_call_cents × n_images (if set), else 30分 × n_images × 倍率.
    let n_images = data
        .get("data")
        .and_then(|d| d.as_array())
        .map(|a| a.len() as f64)
        .unwrap_or(0.0);
    if !has_error && n_images > 0.0 {
        let cost = if conn.per_call_cents > 0 {
            (conn.per_call_cents as f64 * n_images).round().min(5000.0) as i64
        } else {
            (30.0 * n_images * conn.rate).round().min(5000.0) as i64
        };
        if cost > 0 {
            bill(
                &state,
                uid,
                conn.id,
                cost,
                use_quota,
                &BillTokens {
                    model_name: model_id.clone(),
                    ..Default::default()
                },
            )
            .await;
        }
    }

    Ok(Json(data).into_response())
}

#[cfg(test)]
mod billing_tests {
    use super::{
        anthropic_thinking, anthropic_to_oai, chat_upstream_attempt_suffix,
        chat_upstream_retry_base_delay_ms, compute_cost, is_image_gen_model, model_price_override,
        oai_to_anthropic, official_price, parse_usage_from_sse, resolve_cost, response_cache_safe,
        tool_argument_rules, validate_openai_sse_eof, validate_openai_sse_with_rules, AnthSse,
        OpenAiSseValidator,
    };
    use serde_json::json;

    #[test]
    fn shipment_tool_calls_are_never_response_cached() {
        assert!(response_cache_safe(
            br#"data: {\"content\":\"ordinary answer\"}"#
        ));
        assert!(!response_cache_safe(
            br#"data: {\"name\":\"track_shipment\",\"arguments\":\"{\\\"tracking_number\\\":\\\"1Z999AA10123456784\\\"}\"}"#
        ));
    }

    #[test]
    fn chat_gateway_transient_retry_backoff_is_bounded() {
        assert_eq!(chat_upstream_retry_base_delay_ms(0), 250);
        assert_eq!(chat_upstream_retry_base_delay_ms(1), 650);
        assert_eq!(chat_upstream_retry_base_delay_ms(2), 1_300);
        assert_eq!(chat_upstream_retry_base_delay_ms(3), 2_500);
        assert_eq!(chat_upstream_retry_base_delay_ms(4), 4_000);
        assert_eq!(chat_upstream_retry_base_delay_ms(99), 4_000);
    }

    #[test]
    fn chat_gateway_error_suffix_reports_single_route_retries() {
        assert_eq!(
            chat_upstream_attempt_suffix(1, 6, 502),
            "（已请求 6 次；当前只有 1 条同模型线路；最后状态 502）"
        );
        assert_eq!(
            chat_upstream_attempt_suffix(3, 12, 504),
            "（已请求 12 次 / 3 条同模型线路；最后状态 504）"
        );
    }

    // per_call mode bills the flat fee, ignoring token usage entirely.
    #[test]
    fn per_call_mode_flat_fee() {
        let usage = json!({"prompt_tokens": 999999, "completion_tokens": 50000});
        // Huge usage, but per_call mode → exactly per_call_cents regardless.
        assert_eq!(
            resolve_cost(
                "per_call",
                20,
                Some(&usage),
                "claude-opus-4-8",
                5.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            20
        );
        // Even with no usage at all, per_call still charges the flat fee.
        assert_eq!(
            resolve_cost(
                "per_call",
                35,
                None,
                "claude-opus-4-8",
                5.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            35
        );
        // Negative per_call_cents floored to 0.
        assert_eq!(
            resolve_cost("per_call", -5, None, "x", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            0
        );
    }

    // rate mode delegates to compute_cost (real token billing), unchanged.
    #[test]
    fn rate_mode_delegates_to_token_billing() {
        let usage =
            json!({"prompt_tokens": 22000, "completion_tokens": 2000, "total_tokens": 24000});
        // (22000·5 + 2000·25)/1e6 = $0.16 = 16¢ × 1.0 rate.
        assert_eq!(
            resolve_cost(
                "rate",
                999,
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        );
        // per_call_cents is IGNORED in rate mode.
        assert_eq!(
            resolve_cost(
                "rate",
                999,
                Some(&usage),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            48
        );
        // Empty/unknown mode string → treated as rate (safe default).
        assert_eq!(
            resolve_cost(
                "",
                999,
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        );
    }

    // The per-model official catalog returns the published $/1M prices; unknown → None.
    #[test]
    fn official_catalog() {
        assert_eq!(official_price("claude-opus-4-8"), Some((5.0, 25.0)));
        assert_eq!(official_price("CLAUDE-OPUS-4-6"), Some((5.0, 25.0))); // case-insensitive
        assert_eq!(official_price("gpt-5.5"), Some((5.0, 30.0)));
        assert_eq!(official_price("gpt-5.4"), Some((2.5, 15.0)));
        assert_eq!(official_price("deepseek-v4-flash"), Some((0.14, 0.28)));
        assert_eq!(official_price("minimax-m3"), Some((0.30, 1.20)));
        assert_eq!(official_price("some-unknown-model"), None);
    }

    // REAL billing = (in·off_in + out·off_out)/1e6 · 100 · 倍率. Normal agent turn on
    // Claude Opus ($5/$25), 22k in + 2k out:
    //   (22000·5 + 2000·25)/1e6 = $0.16 = 16¢ real cost. × 倍率 3 → 48¢ billed.
    #[test]
    fn real_cost_times_rate() {
        let usage =
            json!({"prompt_tokens": 22000, "completion_tokens": 2000, "total_tokens": 24000});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            48
        ); // ×3
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        ); // ×1 = real cost
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                2.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            32
        ); // ×2
    }

    // gpt-5.5 ($5/$30), 22k+2k, ×1: (110000+60000)/1e6 = $0.17 = 17¢.
    #[test]
    fn gpt55_real_cost() {
        let usage = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        assert_eq!(
            compute_cost(Some(&usage), "gpt-5.5", 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            17
        );
    }

    // Cheap model on a SMALL call rounds toward 0; the SAME model on a big agentic call
    // bills real money. deepseek-v4-flash ($0.14/$0.28), ×1:
    //   22k+2k  → $0.00364 → 0¢ (sub-cent).   200k+10k → $0.0308 → 3¢.
    #[test]
    fn cheap_model_scales_with_size() {
        let small = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        let big = json!({"prompt_tokens": 200000, "completion_tokens": 10000});
        assert_eq!(
            compute_cost(
                Some(&small),
                "deepseek-v4-flash",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            0
        );
        assert_eq!(
            compute_cost(
                Some(&big),
                "deepseek-v4-flash",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            3
        );
    }

    // An uncatalogued model falls back to the admin's per-connection input/output price.
    //   admin $2/$10, 22k+2k, ×1: (44000+20000)/1e6 = $0.064 = 6.4¢ → 6¢.
    #[test]
    fn admin_override_fallback() {
        let usage = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "mystery-model",
                1.0,
                2.0,
                10.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            6
        );
        // No catalog AND no admin price → can't know the real cost → 0.
        assert_eq!(
            compute_cost(
                Some(&usage),
                "mystery-model",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            0
        );
    }

    // A PER-MODEL price override WINS over the built-in official catalog. claude-opus-4-8's
    // catalog price is $5/$25, but with a per-model override of $1/$2 the bill uses $1/$2:
    //   22k·$1 + 2k·$2 = 26000/1e6 = $0.026 = 2.6¢ → 3¢ (×1). Catalog would give 16¢.
    #[test]
    fn per_model_price_override_wins() {
        let usage = json!({"prompt_tokens": 22000, "completion_tokens": 2000});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                2.0
            ),
            3
        );
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
                2.0
            ),
            8
        ); // ×3 → 7.8→8
           // No override (0,0) → catalog price used (16¢), proving the override is what changed it.
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        );
    }

    // model_price_override reads {in,out} from the connection map; missing/empty → (0,0).
    #[test]
    fn model_price_override_reads_map() {
        let mp = json!({"claude-opus-4-8": {"in": 1.5, "out": 2.5}, "gpt-5.5": {}});
        assert_eq!(model_price_override(&mp, "claude-opus-4-8"), (1.5, 2.5));
        assert_eq!(model_price_override(&mp, "gpt-5.5"), (0.0, 0.0)); // empty entry → no override
        assert_eq!(model_price_override(&mp, "absent"), (0.0, 0.0));
        assert_eq!(model_price_override(&json!({}), "anything"), (0.0, 0.0));
    }

    // Image-gen models (any vendor) must be detected so they bill per-image, never $0-tokens.
    #[test]
    fn image_gen_models_detected_across_vendors() {
        for id in [
            "gpt-image-1",
            "gpt-image-2",
            "dall-e-3",
            "gemini-3.1-flash-image-preview",
            "gpt-4o-image",
        ] {
            assert!(is_image_gen_model(id), "should be image: {id}");
        }
        // text / vision models must NOT be treated as image-gen (else they'd bill a flat image fee):
        for id in [
            "claude-opus-4-8",
            "gemini-3.5-flash",
            "gemini-3.1-pro-preview",
            "gpt-5.5",
            "deepseek-v4-pro",
        ] {
            assert!(!is_image_gen_model(id), "should NOT be image: {id}");
        }
    }

    // ---- Anthropic protocol bridge ----
    #[test]
    fn oai_to_anthropic_translates_system_tools_and_toolcalls() {
        let body = json!({
            "model": "claude-haiku-4-5-20251001", "max_tokens": 100,
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "read foo"},
                {"role": "assistant", "content": "ok", "tool_calls": [{"id": "c1", "type": "function", "function": {"name": "read_file", "arguments": "{\"path\":\"foo\"}"}}]},
                {"role": "tool", "tool_call_id": "c1", "content": "file body"}
            ],
            "tools": [{"type": "function", "function": {"name": "read_file", "description": "read", "parameters": {"type": "object", "properties": {"path": {"type": "string"}}}}}]
        });
        let a = oai_to_anthropic(&body).unwrap();
        assert_eq!(a["system"], json!("You are helpful.")); // system hoisted out of messages
        assert_eq!(a["max_tokens"], json!(100)); // haiku (fast tier) → no thinking bump
        assert!(
            a.get("thinking").is_none(),
            "haiku stays fast — no extended thinking"
        );
        let msgs = a["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3); // system removed; user, assistant, tool-result-user
        assert_eq!(msgs[1]["role"], "assistant");
        assert_eq!(msgs[1]["content"][1]["type"], "tool_use");
        assert_eq!(msgs[1]["content"][1]["name"], "read_file");
        assert_eq!(msgs[1]["content"][1]["input"]["path"], "foo"); // arguments string parsed to object
        assert_eq!(msgs[2]["role"], "user");
        assert_eq!(msgs[2]["content"][0]["type"], "tool_result");
        assert_eq!(msgs[2]["content"][0]["tool_use_id"], "c1");
        assert_eq!(a["tools"][0]["name"], "read_file");
        assert!(a["tools"][0]["input_schema"]["properties"]["path"].is_object()); // parameters → input_schema
        assert!(a["tools"][0].get("parameters").is_none());
    }

    #[test]
    fn oai_to_anthropic_rejects_malformed_historical_tool_arguments() {
        let error = oai_to_anthropic(&json!({
            "model": "claude-sonnet-5",
            "messages": [{
                "role": "assistant",
                "tool_calls": [{
                    "id": "call_write_1",
                    "type": "function",
                    "function": {
                        "name": "write_file",
                        "arguments": "{\"path\":\"server/index.js\",\"content\":"
                    }
                }]
            }]
        }))
        .unwrap_err();

        assert!(error.contains("write_file"));
        assert!(error.contains("call_write_1"));
        assert!(error.contains("malformed"));
    }

    #[test]
    fn oai_to_anthropic_enables_thinking_and_drops_temp() {
        // Opus 4.x + reasoning_effort → explicit-budget thinking on; temperature/top_p dropped;
        // max_tokens gets headroom; output_config.effort must NOT be sent (it collapses the
        // upstream thinking stream into a one-line summary).
        let body = json!({
            "model": "claude-opus-4-8", "max_tokens": 4096, "temperature": 0.7, "top_p": 0.9,
            "reasoning_effort": "high",
            "messages": [{"role": "user", "content": "hi"}]
        });
        let a = oai_to_anthropic(&body).unwrap();
        assert_eq!(
            a["thinking"],
            json!({"type":"enabled","budget_tokens":24000})
        );
        assert!(
            a.get("output_config").is_none(),
            "output_config.effort must be omitted or upstream returns summarized thinking"
        );
        assert_eq!(a["max_tokens"], json!(40000)); // high effort gets extra thinking headroom
        assert!(
            a.get("temperature").is_none(),
            "temperature must be dropped when thinking is on"
        );
        assert!(
            a.get("top_p").is_none(),
            "top_p must be dropped when thinking is on"
        );

        // Fable → enabled+budget too.
        assert_eq!(
            oai_to_anthropic(
                &json!({"model":"claude-fable-5","reasoning_effort":"medium","messages":[]})
            )
            .unwrap()["thinking"],
            json!({"type":"enabled","budget_tokens":12000})
        );

        // No reasoning_effort (user chose "off" → IDE drops the field) → NO thinking; temp passes through.
        let off = oai_to_anthropic(&json!({
            "model":"claude-opus-4-8","max_tokens":4096,"temperature":0.5,"messages":[]
        }))
        .unwrap();
        assert!(off.get("thinking").is_none());
        assert_eq!(off["max_tokens"], json!(4096));
        assert_eq!(off["temperature"], json!(0.5));
    }

    #[test]
    fn thinking_normalized_per_model() {
        // Opus 4.8 with reasoning_effort: gateway normalizes to enabled+budget mapped from
        // the effort dial (aggregator upstreams silently ignore "adaptive").
        let a = oai_to_anthropic(&json!({
            "model": "claude-opus-4-8",
            "reasoning_effort": "max",
            "thinking": {"type": "enabled", "budget_tokens": 32000},
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .unwrap();
        assert_eq!(
            a["thinking"],
            json!({"type":"enabled","budget_tokens":32000})
        );
        assert!(a["max_tokens"].as_i64().unwrap() >= 32000);
        assert!(a.get("output_config").is_none()); // effort knob dropped to keep raw thinking

        // Sonnet 5: enabled+budget too (high → 24000)
        let s5 = oai_to_anthropic(&json!({
            "model": "claude-sonnet-5",
            "reasoning_effort": "high",
            "thinking": {"type": "enabled", "budget_tokens": 16000},
            "messages": []
        }))
        .unwrap();
        assert_eq!(
            s5["thinking"],
            json!({"type":"enabled","budget_tokens":24000})
        );

        // Claude 3.7: explicit budget is correct (gateway generates it, not client).
        let b = oai_to_anthropic(&json!({
            "model": "claude-3-7-sonnet-20250219",
            "reasoning_effort": "high",
            "messages": []
        }))
        .unwrap();
        assert_eq!(b["thinking"]["type"], "enabled");
        assert!(b["thinking"]["budget_tokens"].as_i64().unwrap() > 0);
        assert!(b["max_tokens"].as_i64().unwrap() >= 32000);

        // Haiku: no thinking even with effort
        let h = oai_to_anthropic(&json!({
            "model": "claude-haiku-4-5",
            "reasoning_effort": "high",
            "messages": []
        }))
        .unwrap();
        assert!(
            h.get("thinking").is_none(),
            "haiku should not have thinking"
        );
    }

    #[test]
    fn anthropic_thinking_gate_by_model() {
        // effort present → on for capable Claude; mapped to enabled + explicit budget.
        assert_eq!(
            anthropic_thinking("claude-opus-4-8", Some("medium")),
            Some(json!({"type":"enabled","budget_tokens":12000}))
        );
        assert_eq!(
            anthropic_thinking("claude-sonnet-4-6", Some("high")),
            Some(json!({"type":"enabled","budget_tokens":24000}))
        );
        assert_eq!(
            anthropic_thinking("claude-fable-5", Some("low")),
            Some(json!({"type":"enabled","budget_tokens":4096}))
        );
        assert_eq!(
            anthropic_thinking("claude-haiku-4-5-20251001", Some("high")),
            None
        ); // fast tier
        assert_eq!(anthropic_thinking("gpt-5.5", Some("high")), None); // non-Claude
                                                                       // effort absent / "off" → thinking off (respect the user's control).
        assert_eq!(anthropic_thinking("claude-opus-4-8", None), None);
        assert_eq!(anthropic_thinking("claude-opus-4-8", Some("off")), None);
    }

    #[test]
    fn anthropic_to_oai_maps_content_tools_usage() {
        let av = json!({
            "id": "msg_1",
            "content": [{"type": "text", "text": "Hello"}, {"type": "tool_use", "id": "t1", "name": "get_time", "input": {"tz": "Asia/Tokyo"}}],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5, "cache_read_input_tokens": 3, "cache_creation_input_tokens": 0}
        });
        let o = anthropic_to_oai(&av, "claude-opus-4-8");
        assert_eq!(o["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(o["choices"][0]["message"]["content"], "Hello");
        assert_eq!(o["choices"][0]["message"]["tool_calls"][0]["id"], "t1");
        assert_eq!(
            o["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "get_time"
        );
        assert!(
            o["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"]
                .as_str()
                .unwrap()
                .contains("Asia/Tokyo")
        );
        assert_eq!(o["usage"]["input_tokens"], 10); // Anthropic name (compute_cost reads this)
        assert_eq!(o["usage"]["prompt_tokens"], 10); // OpenAI name (clients read this)
        assert_eq!(o["usage"]["cache_read_input_tokens"], 3);
    }

    #[test]
    fn anth_sse_converts_stream_to_openai() {
        // Event shapes copied verbatim from a real zyz streaming response (tool call).
        let mut c = AnthSse::new("claude-opus-4-8");
        let mut out: Vec<u8> = Vec::new();
        out.extend(c.push(b"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m1\",\"usage\":{\"input_tokens\":15,\"cache_read_input_tokens\":46,\"cache_creation_input_tokens\":0,\"output_tokens\":0}}}\n\n").unwrap());
        out.extend(
            c.push(b"event: ping\ndata: {\"type\":\"ping\"}\n\n")
                .unwrap(),
        );
        out.extend(c.push(b"event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"tooluse_1\",\"name\":\"get_time\",\"input\":{}}}\n\n").unwrap());
        out.extend(c.push(b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"tz\\\": \\\"As\"}}\n\n").unwrap());
        out.extend(c.push(b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"ia/Tokyo\\\"}\"}}\n\n").unwrap());
        out.extend(c.push(b"event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n").unwrap());
        out.extend(c.push(b"event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":18,\"input_tokens\":15,\"cache_read_input_tokens\":46}}\n\n").unwrap());
        out.extend(
            c.push(b"event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n")
                .unwrap(),
        );
        out.extend(c.finish().unwrap());
        // Parse the emitted OpenAI SSE back (no key-order assumptions).
        let s = String::from_utf8(out).unwrap();
        let (mut role, mut id, mut name, mut args, mut finish, mut done, mut idx) = (
            false,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            false,
            -1i64,
        );
        for line in s.lines() {
            let d = match line.strip_prefix("data:") {
                Some(x) => x.trim(),
                None => continue,
            };
            if d == "[DONE]" {
                done = true;
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(d).unwrap();
            let delta = &v["choices"][0]["delta"];
            if delta["role"] == "assistant" {
                role = true;
            }
            if let Some(tcs) = delta["tool_calls"].as_array() {
                for tc in tcs {
                    if let Some(i) = tc["index"].as_i64() {
                        idx = i;
                    }
                    if let Some(x) = tc["id"].as_str() {
                        if !x.is_empty() {
                            id = x.into();
                        }
                    }
                    if let Some(n) = tc["function"]["name"].as_str() {
                        if !n.is_empty() {
                            name = n.into();
                        }
                    }
                    if let Some(a) = tc["function"]["arguments"].as_str() {
                        args.push_str(a);
                    }
                }
            }
            if let Some(f) = v["choices"][0]["finish_reason"].as_str() {
                finish = f.into();
            }
        }
        assert!(role, "role bootstrap chunk emitted");
        assert_eq!(id, "tooluse_1");
        assert_eq!(name, "get_time");
        assert_eq!(idx, 0);
        assert_eq!(args, "{\"tz\": \"Asia/Tokyo\"}"); // input_json_delta pieces concatenated
        assert_eq!(finish, "tool_calls");
        assert!(done);
        let u = c.usage(); // accumulated for billing (cache-aware)
        assert_eq!(u["input_tokens"], 15);
        assert_eq!(u["output_tokens"], 18);
        assert_eq!(u["cache_read_input_tokens"], 46);
    }

    #[test]
    fn anth_sse_preserves_non_empty_tool_input_from_block_start() {
        let required = std::collections::HashMap::from([(
            "write_file".to_string(),
            vec!["path".to_string(), "content".to_string()],
        )]);
        let mut c = AnthSse::with_required_tool_args("claude-sonnet-5", required);
        let mut out = c
            .push(b"data: {\"type\":\"content_block_start\",\"index\":2,\"content_block\":{\"type\":\"tool_use\",\"id\":\"write_1\",\"name\":\"write_file\",\"input\":{\"path\":\"server/index.js\",\"content\":\"module.exports = {};\"}}}\n\n")
            .unwrap();
        out.extend(
            c.push(b"data: {\"type\":\"content_block_stop\",\"index\":2}\n\n")
                .unwrap(),
        );
        out.extend(
            c.push(
                b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"}}\n\n",
            )
            .unwrap(),
        );
        out.extend(c.push(b"data: {\"type\":\"message_stop\"}\n\n").unwrap());
        out.extend(c.finish().unwrap());

        let mut arguments = String::new();
        for line in String::from_utf8(out).unwrap().lines() {
            let Some(data) = line.strip_prefix("data:").map(str::trim) else {
                continue;
            };
            if data == "[DONE]" {
                continue;
            }
            let event: serde_json::Value = serde_json::from_str(data).unwrap();
            if let Some(fragment) =
                event["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"].as_str()
            {
                arguments.push_str(fragment);
            }
        }
        let parsed: serde_json::Value = serde_json::from_str(&arguments).unwrap();
        assert_eq!(parsed["path"], "server/index.js");
        assert_eq!(parsed["content"], "module.exports = {};");
    }

    #[test]
    fn anth_sse_rejects_unknown_tool_delta_index() {
        let mut c = AnthSse::new("claude-sonnet-5");
        let error = c
            .push(b"data: {\"type\":\"content_block_delta\",\"index\":7,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{}\"}}\n\n")
            .unwrap_err();
        assert!(error.contains("unknown content block index 7"));
    }

    #[test]
    fn anth_sse_rejects_clean_eof_without_message_stop() {
        let mut c = AnthSse::new("claude-sonnet-5");
        c.push(
            b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1}}}\n\n",
        )
        .unwrap();
        c.push(b"data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n")
            .unwrap();
        let error = c.finish().unwrap_err();
        assert!(error.contains("before message_stop"));
    }

    #[test]
    fn anth_sse_rejects_incomplete_or_missing_required_tool_arguments() {
        let required = std::collections::HashMap::from([(
            "write_file".to_string(),
            vec!["path".to_string(), "content".to_string()],
        )]);
        let mut incomplete = AnthSse::with_required_tool_args("claude-sonnet-5", required.clone());
        incomplete
            .push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"w1\",\"name\":\"write_file\",\"input\":{}}}\n\n")
            .unwrap();
        incomplete
            .push(b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\\\"server/index.js\\\"\"}}\n\n")
            .unwrap();
        let error = incomplete
            .push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n")
            .unwrap_err();
        assert!(error.contains("incomplete arguments JSON"));

        let mut missing = AnthSse::with_required_tool_args("claude-sonnet-5", required);
        missing
            .push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"w2\",\"name\":\"write_file\",\"input\":{}}}\n\n")
            .unwrap();
        let error = missing
            .push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n")
            .unwrap_err();
        assert!(error.contains("missing required arguments: path, content"));
    }

    #[test]
    fn anth_sse_rejects_empty_schema_constrained_tool_arguments() {
        let body = json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "minLength": 1},
                            "content": {"type": "string", "minLength": 1}
                        },
                        "required": ["path", "content"]
                    }
                }
            }]
        });
        let mut stream =
            AnthSse::with_tool_argument_rules("claude-sonnet-5", tool_argument_rules(&body));
        stream
            .push(b"data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"w3\",\"name\":\"write_file\",\"input\":{\"path\":\"src/a.js\",\"content\":\"\"}}}\n\n")
            .unwrap();
        let error = stream
            .push(b"data: {\"type\":\"content_block_stop\",\"index\":0}\n\n")
            .unwrap_err();
        assert!(error.contains("argument \"content\" is shorter than minLength 1"));
    }

    #[test]
    fn anth_sse_rejects_invalid_utf8_even_when_message_stop_follows() {
        let mut c = AnthSse::new("claude-sonnet-5");
        let mut bytes = b"data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"".to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b"\"}}\n\ndata: {\"type\":\"message_stop\"}\n\n");

        let error = c.push(&bytes).unwrap_err();

        assert!(error.contains("invalid UTF-8"));
        assert!(c.finish().is_err());
    }

    // The official catalog must cover every token model live on the gateway, matched by
    // family so date/`-preview` suffixes still resolve. (Image models → per-image, None here.)
    #[test]
    fn official_catalog_covers_live_models() {
        assert_eq!(official_price("claude-fable-5"), Some((10.0, 50.0)));
        assert_eq!(official_price("claude-opus-4-8"), Some((5.0, 25.0)));
        assert_eq!(official_price("claude-opus-4-6"), Some((5.0, 25.0)));
        assert_eq!(official_price("claude-sonnet-5"), Some((3.0, 15.0)));
        assert_eq!(official_price("claude-sonnet-4-6"), Some((3.0, 15.0)));
        assert_eq!(
            official_price("claude-haiku-4-5-20251001"),
            Some((1.0, 5.0))
        ); // date suffix matches
        assert_eq!(official_price("gemini-3.1-pro-preview"), Some((2.0, 12.0)));
        assert_eq!(official_price("gemini-3.5-flash"), Some((1.5, 9.0)));
        assert_eq!(official_price("gemini-3.1-flash-image-preview"), None); // image → per-image billing
        assert_eq!(official_price("gpt-5.5"), Some((5.0, 30.0)));
        assert_eq!(official_price("gpt-5.4"), Some((2.5, 15.0)));
        assert_eq!(official_price("deepseek-v4-flash"), Some((0.14, 0.28)));
        assert_eq!(official_price("deepseek-v4-pro"), Some((0.435, 0.87)));
        assert_eq!(official_price("MiniMax-M3"), Some((0.30, 1.20))); // case-insensitive
        assert_eq!(official_price("MiniMax-M2.7-highspeed"), Some((0.25, 1.00)));
        assert_eq!(official_price("MiniMax-M2.1"), Some((0.15, 0.60)));
        assert_eq!(official_price("MiniMax-M2"), Some((0.10, 0.40)));
        assert_eq!(official_price("some-unknown-model"), None); // → connection fallback, then 0
    }

    // GLM / Grok 走"透传任意 id"的连接（连接价 0、无按模型覆盖），此前不在目录里 → 一直按
    // 0 计费。默认定价 = 官方牌价进目录（docs.z.ai / x.ai，2026-07），连接倍率照常乘在上面。
    #[test]
    fn official_catalog_covers_glm_and_grok_families() {
        assert_eq!(official_price("glm-5.2"), Some((1.40, 4.40)));
        assert_eq!(official_price("GLM-5.1"), Some((1.40, 4.40))); // case-insensitive
        assert_eq!(official_price("glm-5"), Some((1.00, 3.20)));
        assert_eq!(official_price("glm-4.7-flashx"), Some((0.07, 0.40)));
        assert_eq!(official_price("glm-4.7"), Some((0.60, 2.20)));
        assert_eq!(official_price("glm-4.6"), Some((0.60, 2.20)));
        assert_eq!(official_price("glm-4.5-airx"), Some((1.10, 4.50))); // airx 先于 air
        assert_eq!(official_price("glm-4.5-air"), Some((0.20, 1.10)));
        assert_eq!(official_price("glm-4.5-x"), Some((2.20, 8.90)));
        assert_eq!(official_price("glm-4.5"), Some((0.60, 2.20)));
        assert_eq!(official_price("glm-next"), Some((0.60, 2.20))); // 未知变体按 4.x 主档兜底

        assert_eq!(official_price("grok-code-fast-1"), Some((0.20, 1.50)));
        assert_eq!(official_price("grok-4.20"), Some((2.0, 6.0)));
        assert_eq!(official_price("grok-4.5"), Some((2.0, 6.0)));
        assert_eq!(official_price("grok-4.3"), Some((1.25, 2.50)));
        assert_eq!(official_price("grok-4.1-fast"), Some((0.20, 0.50)));
        assert_eq!(official_price("grok-4-fast"), Some((0.20, 0.50)));
        assert_eq!(official_price("grok-build-0.1"), Some((1.0, 2.0)));
        assert_eq!(official_price("grok-3-mini"), Some((0.30, 0.50)));
        assert_eq!(official_price("grok-4-0709"), Some((3.0, 15.0)));
        assert_eq!(official_price("grok-3"), Some((3.0, 15.0)));
        assert_eq!(official_price("grok-x-new"), Some((2.0, 6.0))); // 未知新 Grok 按旗舰档兜底
    }

    // No usage reported → 0 (never guesses token counts).
    #[test]
    fn no_usage_is_zero() {
        assert_eq!(
            compute_cost(None, "claude-opus-4-8", 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            0
        );
        assert_eq!(
            compute_cost(
                Some(&json!({})),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            0
        );
    }

    // Anthropic-style field names (input_tokens/output_tokens) are honored.
    #[test]
    fn anthropic_field_names() {
        let usage = json!({"input_tokens": 22000, "output_tokens": 2000});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            16
        );
    }

    // OpenAI cached-prompt shape: cached input billed at 0.1×. opus, prompt 10000 (8000
    // cached), completion 0, ×1: billable = 2000 + 800 = 2800; 2800·5/1e6 = $0.014 → 1¢.
    #[test]
    fn cached_input_discount() {
        let usage = json!({"prompt_tokens": 10000, "completion_tokens": 0,
                           "prompt_tokens_details": {"cached_tokens": 8000}});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                1.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            1
        );
    }

    // A malformed/huge usage can never drain a balance — capped at $50 (5000¢).
    #[test]
    fn ceiling_caps_runaway() {
        let usage = json!({"prompt_tokens": 999_999_999i64, "completion_tokens": 999_999_999i64});
        assert_eq!(
            compute_cost(
                Some(&usage),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            5000
        );
    }

    // Pull the trailing usage chunk out of a real-shaped SSE stream and bill it.
    #[test]
    fn sse_usage_extraction() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
                   data: {\"choices\":[],\"usage\":{\"prompt_tokens\":22000,\"completion_tokens\":2000,\"total_tokens\":24000}}\n\n\
                   data: [DONE]\n\n";
        let u = parse_usage_from_sse(sse.as_bytes()).expect("usage present");
        assert_eq!(u.get("prompt_tokens").and_then(|v| v.as_i64()), Some(22000));
        assert_eq!(
            compute_cost(
                Some(&u),
                "claude-opus-4-8",
                3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0
            ),
            48
        );
    }

    // The 64KB usage tail can begin MID-LINE (cut from a bigger stream): leading garbage
    // is skipped and the trailing usage still extracted.
    #[test]
    fn sse_usage_from_truncated_tail() {
        let tail = "ent\":\" tokens\"}}]}\n\n\
                    data: {\"choices\":[],\"usage\":{\"prompt_tokens\":50000,\"completion_tokens\":3000}}\n\n\
                    data: [DONE]\n\n";
        let u = parse_usage_from_sse(tail.as_bytes()).expect("usage present in tail");
        assert_eq!(
            u.get("completion_tokens").and_then(|v| v.as_i64()),
            Some(3000)
        );
    }

    #[test]
    fn openai_sse_clean_eof_without_done_is_incomplete() {
        let partial = b"data: {\"choices\":[{\"delta\":{\"content\":\"partial\"}}]}\n\n\
                        data: {\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":1}}\n\n";
        let error = validate_openai_sse_eof(partial).unwrap_err();
        assert!(error.contains("without terminal data: [DONE]"));

        // A marker mentioned inside JSON content is not an SSE terminal event.
        let embedded = b"data: {\"choices\":[{\"delta\":{\"content\":\"data: [DONE]\"}}]}\n\n";
        assert!(validate_openai_sse_eof(embedded).is_err());
    }

    #[test]
    fn openai_sse_done_line_marks_clean_eof_complete() {
        let complete = b"data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\r\n\r\n\
                         data:[DONE]\r\n\r\n";
        assert_eq!(validate_openai_sse_eof(complete), Ok(()));
    }

    #[test]
    fn openai_sse_rejects_malformed_json_before_done() {
        let stream = b"data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{}\"}}]}}]}\n\n\
                       data: {malformed\n\n\
                       data: [DONE]\n\n";

        let error = validate_openai_sse_eof(stream).unwrap_err();

        assert!(error.contains("malformed JSON"));
    }

    #[test]
    fn openai_sse_rejects_incomplete_missing_and_empty_required_tool_arguments() {
        let body = json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "minLength": 1},
                            "content": {"type": "string", "minLength": 1}
                        },
                        "required": ["path", "content"]
                    }
                }
            }]
        });
        let rules = tool_argument_rules(&body);

        let incomplete = b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\",\\\"content\\\":\"}}]}}]}\n\ndata: [DONE]\n\n";
        let error = validate_openai_sse_with_rules(incomplete, rules.clone()).unwrap_err();
        assert!(error.contains("incomplete arguments JSON"));

        let missing = b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\"}\"}}]}}]}\n\ndata: [DONE]\n\n";
        let error = validate_openai_sse_with_rules(missing, rules.clone()).unwrap_err();
        assert!(error.contains("missing required arguments: content"));

        let empty = b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\",\\\"content\\\":\\\"\\\"}\"}}]}}]}\n\ndata: [DONE]\n\n";
        let error = validate_openai_sse_with_rules(empty, rules).unwrap_err();
        assert!(error.contains("argument \"content\" is shorter than minLength 1"));
    }

    #[test]
    fn openai_sse_rejects_terminal_event_before_incomplete_tool_call_can_complete() {
        let body = json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "content": {"type": "string"}
                        },
                        "required": ["path", "content"]
                    }
                }
            }]
        });
        let mut validator =
            OpenAiSseValidator::with_tool_argument_rules(tool_argument_rules(&body));
        validator
            .push(b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\",\\\"content\\\":\"}}]}}]}\n\n")
            .unwrap();

        // The streaming caller validates a chunk before forwarding it, so this
        // error keeps [DONE] from reaching the client or being cached as success.
        let error = validator.push(b"data: [DONE]\n\n").unwrap_err();

        assert!(error.contains("incomplete arguments JSON"));
        assert!(!validator.done_seen);
    }

    #[test]
    fn openai_sse_accumulates_complete_tool_argument_fragments() {
        let body = json!({
            "tools": [{
                "type": "function",
                "function": {
                    "name": "write_file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "minLength": 1},
                            "content": {"type": "string", "minLength": 1}
                        },
                        "required": ["path", "content"]
                    }
                }
            }]
        });
        let stream = b"data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"write_file\",\"arguments\":\"{\\\"path\\\":\\\"src/a.js\\\",\"}}]}}]}\n\ndata: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"content\\\":\\\"ok\\\"}\"}}]}}]}\n\ndata: [DONE]\n\n";

        assert_eq!(
            validate_openai_sse_with_rules(stream, tool_argument_rules(&body)),
            Ok(())
        );
    }

    #[test]
    fn openai_sse_rejects_invalid_utf8_before_done() {
        let mut stream = b"data: {\"choices\":[{\"delta\":{\"content\":\"".to_vec();
        stream.push(0xff);
        stream.extend_from_slice(b"\"}}]}\n\ndata: [DONE]\n\n");

        let error = validate_openai_sse_eof(&stream).unwrap_err();

        assert!(error.contains("invalid UTF-8"));
    }

    // A stream that never reported usage → None → caller bills 0.
    #[test]
    fn sse_no_usage() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n";
        assert!(parse_usage_from_sse(sse.as_bytes()).is_none());
    }
}

#[cfg(test)]
mod cache_price_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn explicit_cache_prices_used() {
        // Anthropic shape: 1000 plain input + 2000 cache_read + 500 cache_create + 300 output
        let u = json!({"input_tokens":1000,"output_tokens":300,"cache_read_input_tokens":2000,"cache_creation_input_tokens":500});
        // off_in=5, off_out=25 (official claude). explicit read=0.5, create=6.5. rate=1.
        // usd = (1000*5 + 2000*0.5 + 500*6.5 + 300*25)/1e6 = (5000+1000+3250+7500)/1e6 = 16750/1e6
        // cents = 16750/1e6 *100 *1 = 1.675 → round 2
        let c = compute_cost(
            Some(&u),
            "claude-opus-4-6",
            1.0,
            0.0,
            0.0,
            0.5,
            6.5,
            0.0,
            0.0,
        );
        assert_eq!(c, 2, "explicit cache prices: got {}", c);
        // with cache prices = 0 → falls back to factors (read 0.1*5=0.5, write 1.25*5=6.25)
        // usd = (1000*5 + 2000*0.5 + 500*6.25 + 300*25)/1e6 = (5000+1000+3125+7500)/1e6=16625 → 1.66 → 2
        let c2 = compute_cost(
            Some(&u),
            "claude-opus-4-6",
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert_eq!(c2, 2, "factor fallback: got {}", c2);
    }
}

#[cfg(test)]
mod estimation_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn estimate_input_tokens_counts_cjk_and_ascii() {
        let body = json!({"messages": [{"role": "user", "content": "你好世界 hello"}]});
        let est = estimate_input_tokens(&body);
        assert!(est > 10, "should estimate > 10 tokens, got {}", est);
    }

    #[test]
    fn estimate_output_tokens_from_sse_bytes() {
        let est = estimate_output_tokens(4000);
        assert!(est > 0 && est < 4000, "should be reasonable, got {}", est);
        assert_eq!(estimate_output_tokens(0), 0);
    }

    #[test]
    fn estimated_usage_produces_valid_json() {
        let u = estimated_usage(1000, 200);
        assert_eq!(u["prompt_tokens"], 1000);
        assert_eq!(u["completion_tokens"], 200);
        assert_eq!(u["total_tokens"], 1200);
    }

    #[test]
    fn extract_bill_tokens_openai_shape() {
        let u = json!({"prompt_tokens": 500, "completion_tokens": 100,
                        "prompt_tokens_details": {"cached_tokens": 50}});
        let bt = extract_bill_tokens(Some(&u), "gpt-5.5", false);
        assert_eq!(bt.prompt, 500);
        assert_eq!(bt.completion, 100);
        assert_eq!(bt.cached, 50);
        assert_eq!(bt.model_name, "gpt-5.5");
        assert!(!bt.estimated);
    }

    #[test]
    fn extract_bill_tokens_anthropic_shape() {
        let u = json!({"input_tokens": 800, "output_tokens": 300,
                        "cache_read_input_tokens": 200});
        let bt = extract_bill_tokens(Some(&u), "claude-opus-4-8", false);
        assert_eq!(bt.prompt, 800);
        assert_eq!(bt.completion, 300);
        assert_eq!(bt.cached, 200);
    }

    #[test]
    fn extract_bill_tokens_none_returns_zeros() {
        let bt = extract_bill_tokens(None, "test", true);
        assert_eq!(bt.prompt, 0);
        assert_eq!(bt.completion, 0);
        assert!(bt.estimated);
    }

    #[test]
    fn anth_sse_fallback_output_estimation() {
        let mut c = AnthSse::new("claude-opus-4-8");
        // Simulate receiving text deltas without a final message_delta (stream broke)
        let bytes = b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":1000}}}\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello world, this is a test response with some content.\"}}\n";
        c.push(bytes).unwrap();
        let u = c.usage();
        assert_eq!(u["input_tokens"], 1000);
        // output_tokens should be estimated from out_bytes since message_delta never arrived
        assert!(
            u["output_tokens"].as_i64().unwrap_or(0) > 0,
            "should estimate output from forwarded bytes, got {}",
            u["output_tokens"]
        );
    }

    #[test]
    fn fallback_estimation_produces_nonzero_cost() {
        let est = estimated_usage(50000, 500);
        // Claude Opus: $5/1M in, $25/1M out, rate=1
        let cost = compute_cost(
            Some(&est),
            "claude-opus-4-8",
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        );
        assert!(
            cost > 0,
            "estimated usage should produce nonzero cost, got {}",
            cost
        );
    }
}
