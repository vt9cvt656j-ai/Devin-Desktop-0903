mod agent_trace;
mod auth;
mod codes;
mod commission;
mod config;
mod deploy;
mod email;
mod error;
mod game;
mod knowledge;
mod models;
mod pay;
mod procedural_3d;
mod prompts;
mod realtime;
mod skills;

use std::sync::Arc;

use axum::response::Html;
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

    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis = redis::aio::ConnectionManager::new(redis_client.clone()).await?;
    tracing::info!("redis connected");

    let bind_addr = cfg.bind_addr.clone();
    let state = AppState {
        db,
        redis,
        redis_client,
        cfg: Arc::new(cfg),
    };

    let app = Router::new()
        .route("/", get(admin_page))
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
        .route("/api/models", get(models::list_for_client))
        .route("/api/ide-key", get(models::ide_key))
        .route("/api/ide-prompts", get(prompts::ide_prompts))
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
        .route("/api/usage", get(models::user_usage))
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
async fn admin_page() -> impl axum::response::IntoResponse {
    (
        [(
            axum::http::header::CACHE_CONTROL,
            "no-store, must-revalidate",
        )],
        Html(include_str!("../static/admin.html")),
    )
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
