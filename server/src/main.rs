mod agent_trace;
mod auth;
mod channel_rates;
mod codes;
mod commission;
mod compression;
mod config;
mod console_session;
mod deploy;
mod desktop;
mod email;
mod error;
mod game;
mod integrations;
mod knowledge;
mod models;
mod pay;
mod procedural_3d;
mod prompts;
mod realtime;
mod receipt;
mod repo_sync;
mod sessions;
mod settings;
mod skills;
mod stripe;
mod update;

use std::sync::Arc;

use axum::response::{Html, IntoResponse};
use axum::routing::{delete, get, post};
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultOnFailure, TraceLayer};

/// Shared application state. Cloned cheaply into every handler.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::ConnectionManager, // multiplexed conn for normal commands
    pub redis_client: redis::Client,          // for pub/sub (needs its own connection)
    pub cfg: Arc<config::Config>,
    pub update_http: reqwest::Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn,tower_http=info".into()),
        )
        .init();

    let cfg = config::Config::from_env()?;

    let db = PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .connect(&cfg.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("postgres connected, migrations applied");

    // 运营参数进内存。必须在建路由之前——面值分母在展示路径上是除数，套餐额度在发放
    // 路径上使用，两者都不能等到第一个请求才有值。读不到会沿用与改造前一致的默认值。
    settings::load(&db).await;
    {
        let s = settings::current();
        tracing::info!(
            raw_cents_per_credit_usd = s.raw_cents_per_credit_usd,
            free_points_daily = s.free_points_daily,
            "运营参数已装载"
        );
    }

    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis = redis::aio::ConnectionManager::new(redis_client.clone()).await?;
    tracing::info!("redis connected");

    let bind_addr = cfg.bind_addr.clone();
    let state = AppState {
        db,
        redis,
        redis_client,
        cfg: Arc::new(cfg),
        update_http: reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(12))
            .user_agent("Michael-IDE-Update-Service/1")
            .build()?,
    };

    let app = Router::new()
        .route("/", get(root_redirect))
        .route("/register", get(register_page))
        .route("/api/logo.png", get(logo_png))
        .route("/health", get(|| async { "ok" }))
        .route("/api/agent-traces", get(agent_trace::list_agent_traces))
        .route("/api/auth/check-email", post(auth::check_email))
        .route("/api/auth/send-code", post(auth::send_code))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/verify-code", post(auth::verify_code))
        .route("/api/me", get(auth::me))
        // 只读门禁：nginx 的 auth_request 打这里，而不是 /api/me。
        // /api/me 每次跑两条 UPDATE，而 auth_request 是每个静态资源都触发一次。
        .route("/api/authz", get(auth::authz))
        // Desktop presence, reported by the app rather than probed over loopback: a
        // browser cannot reliably reach 127.0.0.1 from an HTTPS page (see desktop.rs).
        .route("/api/desktop/heartbeat", post(desktop::heartbeat))
        .route("/api/desktop/status", get(desktop::status))
        // Linked code hosts. `/callback` is the only one without a Bearer token — the
        // provider redirects the browser to it, and the signed `state` is what ties the
        // request to an account (see integrations.rs).
        .route("/api/integrations", get(integrations::list))
        .route("/api/integrations/:provider/start", get(integrations::start))
        .route(
            "/api/integrations/:provider/callback",
            get(integrations::callback),
        )
        .route(
            "/api/integrations/:provider",
            axum::routing::delete(integrations::disconnect),
        )
        .route("/api/integrations/:provider/repos", get(integrations::repos))
        // Linking by pasted token: the path that works before any OAuth app exists.
        .route(
            "/api/integrations/:provider/token",
            post(integrations::connect_token),
        )
        // Body limit covers the inline avatar data URL, which the browser has already
        // resized; the handler caps it far lower still.
        // Where this account is signed in, and signing one of those devices out.
        .route("/api/sessions", get(sessions::list))
        .route("/api/sessions/:id", axum::routing::delete(sessions::revoke))
        .route(
            "/api/me/profile",
            post(auth::update_profile).layer(axum::extract::DefaultBodyLimit::max(1024 * 1024)),
        )
        .route(
            "/api/skills",
            get(skills::get_skills).put(skills::put_skills),
        )
        .route("/api/admin/skills", get(skills::admin_list_skills))
        .route(
            "/api/deploy",
            post(deploy::deploy_site).layer(axum::extract::DefaultBodyLimit::max(35 * 1024 * 1024)),
        )
        .route("/api/admin/users", get(auth::admin_users))
        .route("/api/admin/users/:id/role", post(auth::set_user_role))
        .route("/api/admin/users/:id/grant", post(codes::admin_grant))
        .route(
            "/api/admin/users/:id/credits",
            post(codes::admin_set_credits),
        )
        .route("/api/admin/users/:id/plan", post(codes::admin_set_plan))
        .route(
            "/api/admin/users/:id/cancel-plan",
            post(codes::admin_cancel_plan),
        )
        .route("/api/admin/users/:id", delete(auth::delete_user))
        .route("/api/redeem", post(codes::redeem))
        .route(
            "/api/admin/codes",
            get(codes::admin_list).post(codes::admin_generate),
        )
        .route("/api/admin/codes/:id", delete(codes::admin_delete))
        .route("/api/admin/notify", post(email::notify))
        .route("/api/admin/email-logs", get(email::logs))
        .route("/api/prices", get(pay::list_prices_public))
        .route(
            "/api/admin/prices",
            get(pay::admin_list_prices).post(pay::admin_create_price),
        )
        .route("/api/admin/prices/:id", delete(pay::admin_delete_price))
        .route("/api/billing/catalog", get(stripe::catalog))
        .route("/api/billing/checkout", post(stripe::checkout))
        // Unauthenticated by design: Stripe proves itself with the signature.
        .route("/api/webhooks/stripe", post(stripe::webhook))
        .route("/api/orders", post(pay::create_order))
        .route("/api/admin/orders", get(pay::admin_list_orders))
        .route(
            "/api/admin/orders/:id/confirm",
            post(pay::admin_confirm_order),
        )
        .route(
            "/api/admin/orders/:id/cancel",
            post(pay::admin_cancel_order),
        )
        .route(
            "/api/admin/commissions",
            get(commission::admin_list_commissions).post(commission::admin_create_commission),
        )
        .route(
            "/api/admin/commissions/:id/status",
            post(commission::admin_update_commission_status),
        )
        .route(
            "/api/admin/commissions/:id",
            delete(commission::admin_delete_commission),
        )
        .route(
            "/api/admin/channel-rates",
            get(channel_rates::admin_list).post(channel_rates::admin_create),
        )
        .route(
            "/api/admin/channel-rates/:id",
            post(channel_rates::admin_update).delete(channel_rates::admin_delete),
        )
        .route(
            "/api/admin/settings",
            get(settings::admin_get).post(settings::admin_put),
        )
        // 管理后台门禁：nginx 的 auth_request 打 /api/admin/authz，
        // 登录页拿到管理员 Bearer 后调 /api/admin/session 换 HttpOnly cookie。
        .route("/api/admin/authz", get(console_session::authz))
        .route("/api/admin/session", post(console_session::create_session))
        .route(
            "/api/admin/session/logout",
            post(console_session::destroy_session),
        )
        .route("/api/models", get(models::list_for_client))
        .route("/api/ide-key", get(models::ide_key))
        .route("/api/ide-prompts", get(prompts::ide_prompts))
        .route("/api/ide/update", get(update::latest))
        .route(
            "/api/ide/update/download/:tag/:file",
            get(update::download_asset),
        )
        .route("/api/admin/ide-releases", get(update::admin_release_status))
        .route(
            "/api/admin/ide-releases/dispatch",
            post(update::admin_dispatch_release),
        )
        .route(
            "/api/admin/ide-releases/publish",
            post(update::admin_publish_release),
        )
        .route(
            "/api/admin/ide-releases/runs/:run_id/cancel",
            post(update::admin_cancel_release_run),
        )
        .route("/api/i18n/pack", post(models::i18n_pack))
        .route(
            "/api/models/:id/chat",
            post(models::chat).layer(axum::extract::DefaultBodyLimit::max(12 * 1024 * 1024)),
        )
        .route(
            "/api/admin/models",
            get(models::admin_list).post(models::admin_create),
        )
        .route(
            "/api/admin/models/:id",
            delete(models::admin_delete).post(models::admin_update),
        )
        .route(
            "/api/admin/models/:id/available",
            get(models::admin_available),
        )
        .route(
            "/api/admin/model-estimate",
            post(models::admin_model_estimate),
        )
        .route(
            "/api/admin/quota-estimate",
            post(models::admin_quota_estimate),
        )
        .route("/api/usage", get(models::user_usage))
        .route(
            "/api/usage/settlement/:request_id",
            get(models::usage_settlement),
        )
        .route("/api/admin/model-usage", get(models::admin_usage))
        .route(
            "/api/admin/apikeys",
            get(models::admin_list_apikeys).post(models::admin_create_apikey),
        )
        .route(
            "/api/admin/apikeys/:id",
            delete(models::admin_delete_apikey),
        )
        .route(
            "/v1/chat/completions",
            post(models::chat_completions)
                .layer(axum::extract::DefaultBodyLimit::max(12 * 1024 * 1024)),
        )
        .route(
            "/chat/completions",
            post(models::chat_completions)
                .layer(axum::extract::DefaultBodyLimit::max(12 * 1024 * 1024)),
        )
        .route(
            "/v1/audio/transcriptions",
            post(models::audio_transcriptions)
                .layer(axum::extract::DefaultBodyLimit::max(25 * 1024 * 1024)),
        )
        .route(
            "/audio/transcriptions",
            post(models::audio_transcriptions)
                .layer(axum::extract::DefaultBodyLimit::max(25 * 1024 * 1024)),
        )
        .route("/v1/images/generations", post(models::image_generations))
        .route(
            "/v1/game/generate-3d",
            post(game::generate_3d).layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/v1/game/generate-sound", post(game::generate_sound))
        .route("/v1/game/generate-music", post(game::generate_music))
        .route("/v1/game/generate-voice", post(game::generate_voice))
        .route(
            "/v1/game/auto-rig",
            post(game::auto_rig).layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/v1/game/generate-motion", post(game::generate_motion))
        .route("/v1/game/generate-texture", post(game::generate_texture))
        .route("/v1/game/search-assets", post(game::search_assets))
        .route("/v1/responses", post(models::responses_proxy))
        .route("/responses", post(models::responses_proxy))
        .route("/api/knowledge/search", post(models::knowledge_search))
        .route("/api/knowledge/domains", get(models::knowledge_domains))
        .route("/api/admin/events", get(realtime::recent_events))
        .route("/api/admin/stats", get(realtime::stats))
        .route("/ws", get(realtime::ws_handler))
        .layer(
            TraceLayer::new_for_http()
                .on_failure(DefaultOnFailure::new().level(tracing::Level::WARN)),
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Michael 总后台 listening on http://{bind_addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// The admin dashboard SPA, baked into the binary (no separate build/deploy).
/// `no-store` so browsers always fetch the latest panel after a redeploy — the
/// HTML is baked into the binary, so a new deploy always means new markup.
/// 旧版管理后台（static/admin.html，2091 行）已经整个删掉了。
///
/// 它是第二套能改余额、发兑换码、改套餐的界面，和新控制台各写各的：面值分母 663 在它
/// 里面又是一份独立的硬编码副本，而且它此前对**任何匿名访客**公开，等于把整张运营接口
/// 地图挂在网上。留着它意味着每加一个防护都要在两处各做一遍，而漏掉的那一处就是入口。
///
/// 根路径改成跳转到新控制台。真正的鉴权在 /console/ 的门禁上，这里只是个指路牌。
async fn root_redirect() -> axum::response::Response {
    axum::response::Redirect::temporary("/console/").into_response()
}

/// Public user registration page (email + verification code).
async fn register_page() -> Html<&'static str> {
    Html(include_str!("../static/register.html"))
}

/// Brand logo PNG — served publicly so verification-email clients can load it (baked into the binary
/// via include_bytes!, so it ships with the image and needs no separate asset hosting).
async fn logo_png() -> impl axum::response::IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "image/png"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=604800"),
        ],
        axum::body::Bytes::from_static(include_bytes!("../static/logo.png")),
    )
}
