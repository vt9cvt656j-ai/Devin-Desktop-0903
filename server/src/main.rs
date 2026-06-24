mod auth;
mod codes;
mod config;
mod email;
mod error;
mod models;
mod pay;
mod realtime;

use std::sync::Arc;

use axum::response::Html;
use axum::routing::{delete, get, post};
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Shared application state. Cloned cheaply into every handler.
#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::ConnectionManager,    // multiplexed conn for normal commands
    pub redis_client: redis::Client,             // for pub/sub (needs its own connection)
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
    let state = AppState { db, redis, redis_client, cfg: Arc::new(cfg) };

    let app = Router::new()
        .route("/", get(admin_page))
        .route("/register", get(register_page))
        .route("/health", get(|| async { "ok" }))
        .route("/api/auth/check-email", post(auth::check_email))
        .route("/api/auth/send-code", post(auth::send_code))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/verify-code", post(auth::verify_code))
        .route("/api/me", get(auth::me))
        .route("/api/admin/users", get(auth::admin_users))
        .route("/api/admin/users/:id/role", post(auth::set_user_role))
        .route("/api/admin/users/:id/grant", post(codes::admin_grant))
        .route("/api/admin/users/:id", delete(auth::delete_user))
        .route("/api/redeem", post(codes::redeem))
        .route("/api/admin/codes", get(codes::admin_list).post(codes::admin_generate))
        .route("/api/admin/codes/:id", delete(codes::admin_delete))
        .route("/api/admin/notify", post(email::notify))
        .route("/api/admin/email-logs", get(email::logs))
        .route("/api/prices", get(pay::list_prices_public))
        .route("/api/admin/prices", get(pay::admin_list_prices).post(pay::admin_create_price))
        .route("/api/admin/prices/:id", delete(pay::admin_delete_price))
        .route("/api/orders", post(pay::create_order))
        .route("/api/admin/orders", get(pay::admin_list_orders))
        .route("/api/admin/orders/:id/confirm", post(pay::admin_confirm_order))
        .route("/api/admin/orders/:id/cancel", post(pay::admin_cancel_order))
        .route("/api/models", get(models::list_for_client))
        .route("/api/models/:id/chat", post(models::chat))
        .route("/api/admin/models", get(models::admin_list).post(models::admin_create))
        .route("/api/admin/models/:id", delete(models::admin_delete).post(models::admin_update))
        .route("/api/admin/models/:id/available", get(models::admin_available))
        .route("/api/admin/model-usage", get(models::admin_usage))
        .route("/api/admin/events", get(realtime::recent_events))
        .route("/api/admin/stats", get(realtime::stats))
        .route("/ws", get(realtime::ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Michael 总后台 listening on http://{bind_addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// The admin dashboard SPA, baked into the binary (no separate build/deploy).
async fn admin_page() -> Html<&'static str> {
    Html(include_str!("../static/admin.html"))
}

/// Public user registration page (email + verification code).
async fn register_page() -> Html<&'static str> {
    Html(include_str!("../static/register.html"))
}
