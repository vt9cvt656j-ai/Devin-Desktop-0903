mod agent_trace;
mod api_key_store;
mod auth;
mod changelog;
mod channel_rates;
mod codes;
mod commission;
mod compression;
mod config;
mod connect;
mod console_session;
mod deploy;
mod desktop;
mod docs;
mod field_backfill;
mod field_crypto;
mod email;
mod error;
mod game;
mod handoff;
mod health;
mod shutdown;
mod route_health;
mod integrations;
mod knowledge;
mod model_catalog;
mod model_probe;
mod models;
mod mse;
mod oauth;
mod pay;
mod payout;
mod procedural_3d;
mod prompts;
mod rankings;
mod realtime;
mod receipt;
mod referral;
mod repo_sync;
mod sessions;
mod settings;
mod settlement;
mod skills;
mod stripe;
mod update;

use std::sync::Arc;

use axum::response::IntoResponse;
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
    /// 应用层加密。持有静态密钥和派生缓存 —— 缓存必须跨请求活着，所以它在这里而不是
    /// 在中间件里现造。见 mse.rs。
    pub mse: Arc<mse::Mse>,
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

    // 落库字段加密的密钥。配错（不是 32 字节 base64）要在这里就把进程拦下来，别等到
    // 第一条写入时才发现——那时候一半数据加密、一半没有，最难收拾。没配则 passthrough
    // （敏感字段以明文存库，会打一条 warn）。见 field_crypto.rs。
    field_crypto::init()?;

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
    // 密钥解析失败要在这里就把进程拦下来。配错了却继续启动，表现是所有加密客户端
    // 一起 409，而原因藏在一行启动日志里。
    let mse = Arc::new(mse::Mse::new(&cfg)?);
    let state = AppState {
        db,
        redis,
        redis_client,
        mse,
        cfg: Arc::new(cfg),
        update_http: reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(12))
            .user_agent("Michael-IDE-Update-Service/1")
            .build()?,
    };

    // Start measuring model reachability. Runs on its own timer and writes to
    // model_health; nothing in the request path waits on it.
    health::spawn(state.clone());
    // 线路健康巡检：给没有真实流量的线路补一次最小真实请求，然后评估告警。
    // 单独一个任务，不挂在探针 tick 上——那个循环串行、每条 10 秒超时，一轮最坏 100 秒。
    route_health::spawn(state.clone());

    // Catch payments the webhook never delivered. Every grant in this service hangs off a
    // single webhook call; without this, one missed delivery means a customer paid and got
    // nothing, and nothing would ever notice. See stripe::spawn_reconciler.
    stripe::spawn_reconciler(state.clone());

    // 批量结算：冻结期到了、够门槛了才发，失败自动回滚。默认关闭，由后台开关控制 ——
    // 它会在没有人点任何按钮的情况下往外转真钱。见 payout.rs。
    payout::spawn(state.clone());

    // 把存量明文敏感字段（上游 key、OAuth 令牌、提现账户/QR）加密回去。只在配了
    // FIELD_ENC_KEY 时跑，幂等，逐行条件更新。见 field_backfill.rs。
    field_backfill::spawn(state.clone());
    // 存量 api_key 明文的清除。默认**不跑**——只有显式设了 API_KEY_PURGE_PLAINTEXT=1
    // 的那一次部署才会执行，而且只清已经补齐哈希+密文的行。见 docs/OPERATIONS.md。
    api_key_store::spawn_purge(state.clone());
    // 结算恢复：补扣「已服务却因结算失败没扣到钱」的调用，幂等、绝不双扣。
    settlement::spawn(state.clone());
    // 模型能力目录：实时抓上下文档位和推理档位，取代手写表。
    // 抓不到不影响启动——它有三级降级（内存 → 库里上次的值 → 硬编码表）。
    model_catalog::spawn(state.clone());

    let app = Router::new()
        .route("/", get(root_redirect))
        .route("/api/logo.png", get(logo_png))
        .route("/health", get(|| async { "ok" }))
        // 加密层的引导。这两条永远明文、永远不要求加密 —— 给它们加密是循环依赖。
        // 拿公钥不需要登录：公钥是公开的，而任何客户端在有会话之前就得先能加密。
        .route("/api/crypto/pubkey", get(mse::pubkey))
        .route("/api/crypto/handshake", post(mse::handshake))
        .route("/api/agent-traces", get(agent_trace::list_agent_traces))
        .route("/api/auth/check-email", post(auth::check_email))
        .route("/api/auth/send-code", post(auth::send_code))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/verify-code", post(auth::verify_code))
        // Signing in with a provider. All three are public: they are reached before
        // anyone has a session, and the callback arrives as a bare browser redirect
        // from GitHub or Google with no header of ours on it (see oauth.rs).
        .route("/api/auth/oauth/providers", get(oauth::providers))
        // 桌面端 → 网页 的登录交接，由**桌面端**发起：offer 要带 App 自己的令牌，redeem 用一次性 code 取走。
        // 上一版是网页发起（start/poll/claim），任何网站都能生成一对密钥、诱导受害者的桌面端
        // 去认领，再用自己那半换走对方的会话 —— 见 handoff.rs 顶部。那三条路由已删除。
        .route("/api/auth/handoff/offer", post(handoff::offer))
        .route("/api/auth/handoff/redeem", post(handoff::redeem))
        // Public, names only. The marketing site reads it so its tool list is the
        // gateway's actual catalog rather than a copy baked in at build time.
        .route("/api/tools/catalog", get(prompts::tools_catalog))
        // The public changelog the website renders. Published entries only.
        .route("/api/changelog", get(changelog::list))
        // 用户文档。公开只读；列表不含正文，正文按 slug 单取。写在管理台，见 docs.rs。
        .route("/api/docs", get(docs::list))
        .route("/api/docs/:slug", get(docs::get))
        // Reached by clicking a link in an email, so it cannot require a session — the
        // signature in the URL is what authorises it (see email.rs).
        .route("/api/unsubscribe", get(email::unsubscribe))
        // Who consumed the most, by money and by free points. Signed in only: it
        // ranks accounts and reports their spend (see rankings.rs).
        .route("/api/rankings", get(rankings::list))
        // Writing to it is publishing, so the rest is admin-gated.
        .route(
            "/api/admin/changelog",
            get(changelog::admin_list).post(changelog::admin_create),
        )
        .route("/api/admin/changelog/:id", delete(changelog::admin_delete))
        // 文档的写入口。全部 admin-only（handler 里再查一次 role，不只靠路由分组）。
        .route(
            "/api/admin/docs",
            get(docs::admin_list).post(docs::admin_save),
        )
        .route("/api/admin/docs/:slug", get(docs::admin_get))
        .route("/api/admin/docs/id/:id", delete(docs::admin_delete))
        .route("/api/auth/oauth/:provider/start", get(oauth::start))
        .route("/api/auth/oauth/:provider/callback", get(oauth::callback))
        .route("/api/me", get(auth::me))
        // 退出登录要真的作废这一次登录，否则残留在另一个源 localStorage 里的令牌副本
        // 还能继续换到会话 —— 那正是官网退出后登录页无限刷新的成因，见 auth::logout。
        .route("/api/auth/logout", post(auth::logout))
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
        // 用服务端保管的令牌代读仓库内容。桌面端的 github_repo 工具只认环境变量
        // GITHUB_TOKEN，因此一直在匿名请求（60 次/小时），读几个文件就被限流。
        // 令牌不下发给客户端 —— 那会抹掉"只存服务端"的意义。见 integrations::read。
        .route("/api/integrations/:provider/read", get(integrations::read))
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
        // Mail to the customer base. `audience` only counts, so the operator sees the size
        // of the decision before making it; `send` starts a background campaign and
        // returns a receipt rather than holding the request open for the whole run.
        // Referrals. `me` and `claim` are the customer's side — a code, a link, and
        // binding the person who invited them. The commission a payment raises is created
        // on the Stripe fulfilment path, not here (see referral.rs).
        .route("/api/referral/me", get(referral::me))
        .route("/api/referral/claim", post(referral::claim))
        .route("/api/referral/referrals", get(referral::my_referrals))
        .route("/api/referral/settlements", get(referral::my_settlements))
        // Asking to be paid. `withdraw` records a request; nothing here moves money.
        .route("/api/referral/withdrawals", get(referral::withdrawals))
        .route("/api/referral/withdraw", post(referral::withdraw))
        .route(
            "/api/admin/referral/settings",
            get(referral::admin_settings).put(referral::admin_save_settings),
        )
        // The same relationships from both ends: `list` is one row per referred customer,
        // `referrers` is one row per person doing the referring.
        .route("/api/admin/referral/list", get(referral::admin_list))
        .route("/api/admin/referral/referrers", get(referral::admin_referrers))
        // Referring is granted per account, not held by everyone (see referral.rs).
        .route("/api/admin/referral/grant/:id", post(referral::admin_grant))
        // The payout queue. Carries payment details and QR images, so admin-gated like
        // everything else under /api/admin.
        // Every commission that has been settled, automatically or by hand.
        .route("/api/admin/settlements", get(referral::admin_settlements))
        .route("/api/admin/withdrawals", get(referral::admin_withdrawals))
        .route(
            "/api/admin/withdrawals/:id/status",
            post(referral::admin_withdraw_status),
        )
        .route("/api/admin/email/audience", post(email::audience))
        .route("/api/admin/email/send", post(email::send))
        .route("/api/admin/email/campaigns", get(email::campaigns))
        .route("/api/admin/email-logs", get(email::logs))
        .route("/api/prices", get(pay::list_prices_public))
        .route(
            "/api/admin/prices",
            get(pay::admin_list_prices).post(pay::admin_create_price),
        )
        .route("/api/admin/prices/:id", delete(pay::admin_delete_price))
        // What Stripe says happened, for the console's payments page.
        .route("/api/admin/stripe/payments", get(stripe::admin_payments))
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
        // 只看 mide_token 的 role，给 /console/login 当门禁 —— 登录页本身也不该公开。
        .route("/api/admin/ide-authz", get(console_session::ide_authz))
        .route("/api/admin/session", post(console_session::create_session))
        .route(
            "/api/admin/session/logout",
            post(console_session::destroy_session),
        )
        .route("/api/models", get(models::list_for_client))
        // Reachability of each configured route, measured by the prober in health.rs.
        // Signed in only: it names the deployment's model routes.
        .route("/api/models/status", get(health::status))
        .route("/api/ide-key", get(models::ide_key))
        .route("/api/ide-prompts", get(prompts::ide_prompts))
        .route("/api/ide/update", get(update::latest))
        .route(
            "/api/ide/update/download/:tag/:file",
            get(update::download_asset),
        )
        // What a visitor can install today, read from the release's own assets rather
        // than from latest.json — installing and auto-updating are different questions.
        .route("/api/ide/downloads", get(update::downloads))
        // The published changelog, for the console's Update log page.
        .route("/api/ide/releases", get(update::releases)
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
        // Display grouping: show one route's models under another's name. Changes only
        // the picker's heading — never where a request goes (see models.rs).
        .route("/api/admin/models/:id/group", post(models::admin_group))
        // 自动打款：推荐人自己去 Stripe 开户，之后提现不再需要人工确认。
        // 支付成功页：这一笔买到了什么。订单还没到账时会主动向 Stripe 核实并当场发放。
        .route("/api/billing/session/:id", get(stripe::session_result))
        .route("/api/referral/connect", get(connect::status))
        .route("/api/referral/connect/start", post(connect::start))
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
        // 层序（后加的在外）：CORS 最外，然后 Trace，然后 MSE，最里面才是路由。
        //
        // MSE 在 Trace 之内：这样 access 日志记的是**外层**那个不带 query 的 URI，
        // 而不是解密后的完整地址 —— 把 query 从中间人那里藏起来、却自己原样写进日志，
        // 等于白做。Trace 在 CORS 之内没有影响，预检请求由 CORS 直接短路掉。
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            mse::middleware,
        ))
        .layer(
            TraceLayer::new_for_http()
                .on_failure(DefaultOnFailure::new().level(tracing::Level::WARN)),
        )
        // 跨源必须显式暴露 X-Mse-* 响应头。官网在 mrday.one，网关在 code.mrday.one；
        // 不暴露的话浏览器会把这些头对脚本藏起来，官网每一个请求都解不开 —— 而且症状
        // 是「解密失败」，不是「CORS 错误」，查起来会绕很远。
        .layer(CorsLayer::permissive().expose_headers(
            mse::EXPOSED_HEADERS
                .iter()
                .filter_map(|h| h.parse::<axum::http::HeaderName>().ok())
                .collect::<Vec<_>>(),
        ))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("Michael 总后台 listening on http://{bind_addr}");
    // 两段等待，缺一不可（理由见 shutdown.rs 顶部）：
    //   · with_graceful_shutdown 停止接新连接、等在途 HTTP 连接结束；
    //   · drain 再等**结算任务**——它们是 spawn 出去的，不挂在任何连接上，
    //     服务器关掉之后照样在跑，而进程一 return 就全没了，那一笔的计费就此丢失。
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal())
        .await?;
    shutdown::drain(std::time::Duration::from_secs(20)).await;
    tracing::info!("已优雅退出");
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
    // 官网，不是管理后台。
    //
    // 以前这里跳 /console/ —— 任何人输一下域名，浏览器就把管理后台的位置指了出来，
    // 而那正是最不该主动广播的东西。这个域名对外的身份是网关，人来了就送去官网。
    axum::response::Redirect::temporary("https://mrday.one/").into_response()
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
