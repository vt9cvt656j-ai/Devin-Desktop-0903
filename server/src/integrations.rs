//! Linking a GitHub or GitLab account, so the IDE can offer someone's own repositories
//! behind `@github:` and `@gitlab:`.
//!
//! Three things decide the shape of this module.
//!
//! **The tokens are the crown jewels.** A GitHub token with `repo` scope is read/write
//! access to every private repository the person owns. So: no endpoint here ever returns
//! one, they are stored server-side and only ever spent by this server calling the
//! provider on the person's behalf, and the repo listing goes through us rather than
//! handing the browser a token to call GitHub with directly. And **at rest they are
//! encrypted** (see field_crypto.rs): a database dump alone yields `fc1:...`, not a
//! working token, because the decryption key lives in the process env, not the DB.
//!
//! **The callback is a plain browser redirect**, which means it arrives with no
//! Authorization header — the provider sends the person's browser to us. So the `state`
//! parameter has to carry who started the flow, and it has to be unforgeable, or anyone
//! could complete an OAuth dance and have the resulting token attached to *someone
//! else's* account. It is a short-lived JWT signed with the same secret as a login
//! token, and it is checked for provider mismatch as well as signature.
//!
//! **Credentials belong to the operator.** The OAuth app is registered under their
//! GitHub/GitLab identity, so OAuth ships with no defaults and reports
//! `oauth_configured: false` until the environment carries an id and secret.
//!
//! **But a provider without OAuth still connects.** Requiring the operator to register
//! an OAuth app before anyone can link anything makes the button dead on a fresh
//! deployment. A personal access token needs nothing registered — the person creates one
//! in their own account settings and pastes it — so that path is always open, and the
//! OAuth button appears alongside it once credentials exist.

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::field_crypto;
use crate::AppState;

/// 落库加密的 context（= 列身份，绑进 AAD）。见 field_crypto.rs。
const ACCESS_CTX: &str = "connected_accounts.access_token";
const REFRESH_CTX: &str = "connected_accounts.refresh_token";

/// Long enough to sign in at the provider and approve; short enough that a `state`
/// captured from a browser history or a proxy log is useless by the time it is found.
const STATE_TTL_SECS: i64 = 600;

/// GitHub rejects API requests without one, with a 403 that says nothing useful.
const UA: &str = "MrDayOne-Gateway";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    GitHub,
    GitLab,
}

impl Provider {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "github" => Some(Self::GitHub),
            "gitlab" => Some(Self::GitLab),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::GitLab => "gitlab",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
        }
    }

    fn credentials(self, state: &AppState) -> (String, String) {
        match self {
            Self::GitHub => (
                state.cfg.github_client_id.clone(),
                state.cfg.github_client_secret.clone(),
            ),
            Self::GitLab => (
                state.cfg.gitlab_client_id.clone(),
                state.cfg.gitlab_client_secret.clone(),
            ),
        }
    }

    /// Least privilege that still does the job.
    ///
    /// GitHub's `repo` is coarse — it is the only scope that can see private
    /// repositories, and it grants write with it. `read:user` is separate so the page
    /// can say who is connected. GitLab splits read from write properly, so it gets
    /// read-only scopes.
    fn scope(self) -> &'static str {
        match self {
            Self::GitHub => "repo read:user",
            Self::GitLab => "read_api read_user read_repository",
        }
    }

    /// Where the person goes to create a personal access token by hand.
    fn token_create_url(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/settings/tokens/new?scopes=repo&description=Mr.day%20One",
            Self::GitLab => "https://gitlab.com/-/user_settings/personal_access_tokens",
        }
    }

    /// Which boxes to tick on that page. Getting this wrong is the most likely reason a
    /// pasted token is refused, so the page says it rather than leaving it to be guessed.
    fn token_hint(self) -> &'static str {
        match self {
            Self::GitHub => "repo",
            Self::GitLab => "read_api, read_repository",
        }
    }

    fn authorize_url(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/login/oauth/authorize",
            Self::GitLab => "https://gitlab.com/oauth/authorize",
        }
    }

    fn token_url(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/login/oauth/access_token",
            Self::GitLab => "https://gitlab.com/oauth/token",
        }
    }
}

/// What the `state` parameter carries across the redirect.
///
/// `nonce` 同时被写进一颗只有发起方浏览器拿得到的 cookie，回调时必须对上。签名只能证明
/// 「这个 state 是我们签的」，证明不了「是给这个浏览器签的」—— 而这里的 `sub` 直接写着
/// 发起者的用户 id，所以少了这一半的后果是：攻击者用自己账号发起、扣下回调 URL、诱导受害者
/// 打开，**受害者授权出去的代码托管令牌会存进攻击者的行**，攻击者随后就能读他的仓库。
#[derive(Debug, Serialize, Deserialize)]
struct StateClaims {
    sub: String,
    provider: String,
    nonce: String,
    exp: i64,
}

fn redirect_uri(state: &AppState, provider: Provider) -> String {
    format!(
        "{}/api/integrations/{}/callback",
        state.cfg.ide_update_public_base.trim_end_matches('/'),
        provider.key()
    )
}

/// Back to the page the person started from, with a word about how it went. The console
/// reads these and shows a message; it never needs to be told the token exists.
fn console_redirect(state: &AppState, outcome: &str) -> Redirect {
    Redirect::to(&format!(
        "{}/dashboard?integration={}#integrations",
        state.cfg.ide_update_public_base.trim_end_matches('/'),
        outcome
    ))
}

fn provider_or_400(raw: &str) -> ApiResult<Provider> {
    Provider::parse(raw).ok_or_else(|| AppError::bad("未知的代码托管平台"))
}

// ── GET /api/integrations ────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct ConnectionRow {
    provider: String,
    account_login: String,
    account_name: String,
    avatar_url: String,
    connected_at: chrono::DateTime<chrono::Utc>,
}

/// Everything the Integrations page needs: which providers this deployment can offer,
/// and which of them this person has linked. Deliberately says nothing about tokens.
pub async fn list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let rows = sqlx::query_as::<_, ConnectionRow>(
        "SELECT provider, account_login, account_name, avatar_url, connected_at \
         FROM connected_accounts WHERE user_id = $1",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    let providers: Vec<serde_json::Value> = [Provider::GitHub, Provider::GitLab]
        .iter()
        .map(|p| {
            let (id, secret) = p.credentials(&state);
            let configured = !id.trim().is_empty() && !secret.trim().is_empty();
            let linked = rows.iter().find(|r| r.provider == p.key());
            json!({
                "provider": p.key(),
                "label": p.label(),
                // Whether the OAuth *button* can be offered. Token linking never depends
                // on it, so the card is never dead.
                "oauth_configured": configured,
                "token_url": p.token_create_url(),
                "token_hint": p.token_hint(),
                "connected": linked.is_some(),
                "account_login": linked.map(|r| r.account_login.clone()),
                "account_name": linked.map(|r| r.account_name.clone()),
                "avatar_url": linked.map(|r| r.avatar_url.clone()),
                "connected_at": linked.map(|r| r.connected_at),
            })
        })
        .collect();

    Ok(Json(json!({ "providers": providers })))
}

// ── GET /api/integrations/:provider/start ────────────────────────────────────────────

/// Returns the URL to send the browser to, rather than redirecting itself: this endpoint
/// is called with a Bearer token from the console, and a redirect chain cannot carry one.
/// The console navigates to what it gets back.
pub async fn start(
    State(state): State<AppState>,
    claims: Claims,
    Path(provider): Path<String>,
) -> ApiResult<axum::response::Response> {
    use axum::response::IntoResponse;
    let provider = provider_or_400(&provider)?;
    let (client_id, secret) = provider.credentials(&state);
    if client_id.trim().is_empty() || secret.trim().is_empty() {
        return Err(AppError::bad(format!(
            "{} 尚未在本服务器上配置",
            provider.label()
        )));
    }

    let nonce = uuid::Uuid::new_v4().to_string();
    let state_token = encode(
        &Header::default(),
        &StateClaims {
            sub: claims.sub.clone(),
            provider: provider.key().to_owned(),
            nonce: nonce.clone(),
            exp: chrono::Utc::now().timestamp() + STATE_TTL_SECS,
        },
        &EncodingKey::from_secret(state.cfg.jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::internal("无法签发 state"))?;

    // Built through the URL type rather than format!: `state` is a JWT and `scope` has
    // spaces in it, and hand-assembling those is how a redirect_uri quietly stops
    // matching the one registered at the provider.
    let url = reqwest::Url::parse_with_params(
        provider.authorize_url(),
        &[
            ("client_id", client_id.as_str()),
            ("redirect_uri", redirect_uri(&state, provider).as_str()),
            ("scope", provider.scope()),
            ("state", state_token.as_str()),
            ("response_type", "code"),
        ],
    )
    .map_err(|_| AppError::internal("无法构造授权地址"))?;

    // cookie 和 state 各持 nonce 的一半，回调时比对。SameSite=Lax：回调是从 provider 过来的
    // 顶层 GET 导航，Strict 会把它扣下，正常流程就永远走不通。
    Ok((
        [(
            axum::http::header::SET_COOKIE,
            format!(
                "{NONCE_COOKIE}={nonce}; Path=/api/integrations; Max-Age={STATE_TTL_SECS}; \
                 HttpOnly; Secure; SameSite=Lax"
            ),
        )],
        Json(json!({ "url": url.to_string() })),
    )
        .into_response())
}

/// 发起方浏览器持有的那半 nonce。
const NONCE_COOKIE: &str = "mide_integ_nonce";

fn nonce_cookie(headers: &axum::http::HeaderMap) -> Option<String> {
    let raw = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| k.trim() == NONCE_COOKIE)
        .map(|(_, v)| v.trim().to_owned())
}

/// 常量时间比较。
fn nonce_matches(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// ── GET /api/integrations/:provider/callback ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    /// Providers send this when the person declines.
    error: Option<String>,
}

/// The provider sends the browser here. No Authorization header exists on this request —
/// `state` is the only thing tying it to an account, which is why it is signed.
pub async fn callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(q): Query<CallbackQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    let Some(provider) = Provider::parse(&provider) else {
        return console_redirect(&state, "error").into_response();
    };
    if q.error.is_some() {
        // Declining is a normal outcome, not a failure worth a scary message.
        return console_redirect(&state, "cancelled").into_response();
    }
    let (Some(code), Some(state_token)) = (q.code, q.state) else {
        return console_redirect(&state, "error").into_response();
    };

    match finish(&state, provider, &code, &state_token, &headers).await {
        Ok(()) => console_redirect(&state, provider.key()).into_response(),
        Err(e) => {
            // The person sees only "it failed"; the detail goes to the log, because it
            // can contain provider messages that are not theirs to read.
            tracing::warn!("{} OAuth callback failed: {e:?}", provider.label());
            console_redirect(&state, "error").into_response()
        }
    }
}

async fn finish(
    state: &AppState,
    provider: Provider,
    code: &str,
    state_token: &str,
    headers: &axum::http::HeaderMap,
) -> anyhow::Result<()> {
    let data = decode::<StateClaims>(
        state_token,
        &DecodingKey::from_secret(state.cfg.jwt_secret.as_bytes()),
        &Validation::default(),
    )?;
    // A valid signature for *a* flow is not a valid signature for *this* one: without
    // this check a state minted for GitHub would complete a GitLab link.
    if data.claims.provider != provider.key() {
        anyhow::bail!("state provider mismatch");
    }
    // 完成回调的必须是发起的那个浏览器。
    //
    // 签名只证明「这个 state 是我们签的」—— 攻击者手里那个也是我们签的，因为那确实是给
    // **他**签的，而 `sub` 里写的就是他的用户 id。少了这一半，他就能扣下自己的回调 URL
    // 诱导受害者打开：受害者在 provider 那边点了同意，令牌落进攻击者的行。
    let held = nonce_cookie(headers).unwrap_or_default();
    if !nonce_matches(&held, &data.claims.nonce) {
        anyhow::bail!("state was not issued to this browser");
    }
    let uid = uuid::Uuid::parse_str(&data.claims.sub)?;

    let (client_id, client_secret) = provider.credentials(state);
    let redirect = redirect_uri(state, provider);

    let token: serde_json::Value = match provider {
        Provider::GitHub => {
            state
                .update_http
                .post(provider.token_url())
                .header("Accept", "application/json")
                .header("User-Agent", UA)
                .form(&[
                    ("client_id", client_id.as_str()),
                    ("client_secret", client_secret.as_str()),
                    ("code", code),
                    ("redirect_uri", redirect.as_str()),
                ])
                .send()
                .await?
                .json()
                .await?
        }
        Provider::GitLab => {
            state
                .update_http
                .post(provider.token_url())
                .form(&[
                    ("grant_type", "authorization_code"),
                    ("code", code),
                    ("client_id", client_id.as_str()),
                    ("client_secret", client_secret.as_str()),
                    ("redirect_uri", redirect.as_str()),
                ])
                .send()
                .await?
                .json()
                .await?
        }
    };

    let access = token
        .get("access_token")
        .and_then(|v| v.as_str())
        // 只报「没有」，不报「响应体是什么」。
        //
        // `token` 是 provider 令牌端点的完整响应。交换失败时 GitHub 会把请求参数回显在
        // 错误里 —— 其中包含 **client_secret**。而这个 anyhow 错误在上面的 callback 里被
        // `tracing::warn!("{e:?}")` 打进日志，于是平台密钥就落在了日志文件里，等着被
        // 任何能读日志的人、或任何一次日志外传捡走。
        //
        // oauth.rs:599 的登录流是刻意避开这一点的，这里漏了。响应体要看就去看 provider
        // 那边的记录，不要经由我们的日志。
        .ok_or_else(|| anyhow::anyhow!("provider returned no access_token"))?
        .to_owned();
    let refresh = token
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let expires_at = token
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .map(|secs| chrono::Utc::now() + chrono::Duration::seconds(secs));
    let scopes = token
        .get("scope")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();

    let who = fetch_account(state, provider, &access).await?;

    // 落库前加密。fetch_account 上面用的是明文 access（要拿它去 provider 查账号），
    // 只有**存进库**的这一份变成密文。
    let enc_access = field_crypto::encrypt(&access, ACCESS_CTX);
    let enc_refresh: Option<String> =
        refresh.as_deref().map(|r| field_crypto::encrypt(r, REFRESH_CTX));

    sqlx::query(
        "INSERT INTO connected_accounts \
           (user_id, provider, account_login, account_name, avatar_url, access_token, \
            refresh_token, token_expires_at, scopes, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9, now()) \
         ON CONFLICT (user_id, provider) DO UPDATE SET \
           account_login = EXCLUDED.account_login, \
           account_name  = EXCLUDED.account_name, \
           avatar_url    = EXCLUDED.avatar_url, \
           access_token  = EXCLUDED.access_token, \
           refresh_token = EXCLUDED.refresh_token, \
           token_expires_at = EXCLUDED.token_expires_at, \
           scopes        = EXCLUDED.scopes, \
           updated_at    = now()",
    )
    .bind(uid)
    .bind(provider.key())
    .bind(&who.login)
    .bind(&who.name)
    .bind(&who.avatar)
    .bind(&enc_access)
    .bind(enc_refresh.as_deref())
    .bind(expires_at)
    .bind(&scopes)
    .execute(&state.db)
    .await?;

    Ok(())
}

struct Account {
    login: String,
    name: String,
    avatar: String,
}

async fn fetch_account(
    state: &AppState,
    provider: Provider,
    access: &str,
) -> anyhow::Result<Account> {
    let url = match provider {
        Provider::GitHub => "https://api.github.com/user",
        Provider::GitLab => "https://gitlab.com/api/v4/user",
    };
    let body: serde_json::Value = state
        .update_http
        .get(url)
        .header("User-Agent", UA)
        .bearer_auth(access)
        .send()
        .await?
        .json()
        .await?;
    Ok(Account {
        // GitHub calls the handle `login`; GitLab calls it `username`.
        login: body
            .get("login")
            .or_else(|| body.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        name: body
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        avatar: body
            .get("avatar_url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
    })
}

// ── POST /api/integrations/:provider/token ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct TokenReq {
    pub token: String,
}

/// Link by personal access token instead of OAuth.
///
/// This is what makes the button work on a deployment where no OAuth app has been
/// registered — which is every deployment until the operator registers one. The person
/// creates a token in their own account settings and pastes it; nothing has to exist on
/// our side first.
///
/// The token is verified before it is stored, by spending it on the provider's "who am
/// I" endpoint. A typo, a token for the wrong host, or one whose scopes are too narrow
/// fails here with a message, rather than being saved and then failing later at the
/// point someone types `@github:` and gets an empty list.
pub async fn connect_token(
    State(state): State<AppState>,
    claims: Claims,
    Path(provider): Path<String>,
    Json(req): Json<TokenReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let provider = provider_or_400(&provider)?;
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let token = req.token.trim().to_owned();
    if token.is_empty() {
        return Err(AppError::bad("请填写访问令牌"));
    }
    // Length only — the formats are not stable enough to pattern-match, and the real
    // check is the API call below.
    if token.len() > 512 {
        return Err(AppError::bad("这不像是一个访问令牌"));
    }

    let who = fetch_account(&state, provider, &token)
        .await
        .map_err(|_| AppError::bad(format!("{} 拒绝了这个令牌，请检查是否填错或权限不足", provider.label())))?;
    if who.login.is_empty() {
        return Err(AppError::bad(format!(
            "{} 没有认出这个令牌对应的账号",
            provider.label()
        )));
    }

    sqlx::query(
        "INSERT INTO connected_accounts \
           (user_id, provider, account_login, account_name, avatar_url, access_token, \
            refresh_token, token_expires_at, scopes, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,NULL,NULL,'pat', now()) \
         ON CONFLICT (user_id, provider) DO UPDATE SET \
           account_login = EXCLUDED.account_login, \
           account_name  = EXCLUDED.account_name, \
           avatar_url    = EXCLUDED.avatar_url, \
           access_token  = EXCLUDED.access_token, \
           refresh_token = NULL, \
           token_expires_at = NULL, \
           scopes        = 'pat', \
           updated_at    = now()",
    )
    .bind(uid)
    .bind(provider.key())
    .bind(&who.login)
    .bind(&who.name)
    .bind(&who.avatar)
    .bind(field_crypto::encrypt(&token, ACCESS_CTX))
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "ok": true, "account_login": who.login })))
}

// ── DELETE /api/integrations/:provider ───────────────────────────────────────────────

/// Forgets the token. The grant still exists at the provider until it is revoked there
/// too, which the response says so nobody assumes this was a full revocation.
pub async fn disconnect(
    State(state): State<AppState>,
    claims: Claims,
    Path(provider): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let provider = provider_or_400(&provider)?;
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    sqlx::query("DELETE FROM connected_accounts WHERE user_id = $1 AND provider = $2")
        .bind(uid)
        .bind(provider.key())
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "ok": true,
        "revoke_at_provider": match provider {
            Provider::GitHub => "https://github.com/settings/applications",
            Provider::GitLab => "https://gitlab.com/-/user_settings/applications",
        }
    })))
}

// ── GET /api/integrations/:provider/repos ────────────────────────────────────────────

/// What `@github:` and `@gitlab:` offer. Proxied rather than letting the IDE hold the
    /// 令牌端点的响应体绝不能进日志。
    ///
    /// 交换失败时 provider 会把请求参数回显在错误里，其中包含 client_secret；而这个错误
    /// 会被 callback 的 `tracing::warn!("{e:?}")` 打进日志文件。平台密钥落在日志里，等于
#[derive(serde::Deserialize)]
pub struct ReadQuery {
    pub owner: String,
    pub repo: String,
    /// overview | readme | tree | file — 与桌面端 `github_repo` 工具的动作同名。
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

/// `GET /api/integrations/:provider/read` — 用服务端保管的令牌代读仓库内容。
///
/// **为什么要有这条。** 桌面端的 `github_repo` 工具直连 GitHub，而它的鉴权
/// （`ide/src-tauri/src/knowledge.rs` 的 `github_auth_header`）**只读进程环境变量
/// `GITHUB_TOKEN`** —— 用户在网页后台用 OAuth 连好的那份令牌它根本不知道。于是每一次读仓库
/// 都是匿名请求，配额 60 次/小时；读几个文件就打光，然后开始报错、重试，看起来就是"变笨了"。
///
/// 修法不是把令牌下发给客户端 —— 那正好抹掉"令牌只存服务端"的全部意义。改成让读取绕一下
/// 网关：令牌一步都不离开这里，客户端拿到的只有内容。已连接的账号因此享有认证配额
/// （5000 次/小时），而不是匿名的 60 次。
///
/// 只放行读取类动作，且 owner/repo/path 都要过白名单 —— 它们要拼进上游 URL。
pub async fn read(
    State(state): State<AppState>,
    claims: Claims,
    Path(provider): Path<String>,
    axum::extract::Query(q): axum::extract::Query<ReadQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let provider = provider_or_400(&provider)?;
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    // 拼进 URL 的每一段都要钉死字符集，否则 `..` 或 `?` 能把请求指到别处。
    let safe = |v: &str| {
        !v.is_empty()
            && v.len() <= 128
            && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    };
    if !safe(&q.owner) || !safe(&q.repo) {
        return Err(AppError::bad("owner/repo 格式不对"));
    }
    let action = q.action.as_deref().unwrap_or("overview");
    // 白名单，不是黑名单：新增动作必须显式想过一遍。
    if !matches!(action, "overview" | "readme" | "tree" | "file") {
        return Err(AppError::bad("只支持 overview/readme/tree/file"));
    }
    let path = q.path.as_deref().unwrap_or("");
    if !path.is_empty()
        && (path.contains("..") || path.starts_with('/') || path.len() > 400
            || !path.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/')))
    {
        return Err(AppError::bad("path 格式不对"));
    }

    let access: Option<String> = sqlx::query_scalar(
        "SELECT access_token FROM connected_accounts WHERE user_id = $1 AND provider = $2",
    )
    .bind(uid)
    .bind(provider.key())
    .fetch_optional(&state.db)
    .await?;
    let access = access.ok_or_else(|| AppError::bad(format!("尚未连接 {}", provider.label())))?;
    // 存的是密文（fc1:...）或遗留明文；解开再拿去认证 provider。解不开是配置事故
    // （密钥没配却存了密文、或密钥换了），明确报错，别把 fc1:... 当令牌发出去。
    let access = field_crypto::decrypt(&access, ACCESS_CTX)
        .map_err(|_| AppError::internal("连接令牌解密失败，请重新连接"))?;

    let (owner, repo) = (&q.owner, &q.repo);
    let url = match (provider, action) {
        (Provider::GitHub, "overview") => format!("https://api.github.com/repos/{owner}/{repo}"),
        (Provider::GitHub, "readme") => format!("https://api.github.com/repos/{owner}/{repo}/readme"),
        (Provider::GitHub, "tree") => {
            format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}")
        }
        (Provider::GitHub, _) => format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}"),
        (Provider::GitLab, "overview") => {
            format!("https://gitlab.com/api/v4/projects/{owner}%2F{repo}")
        }
        (Provider::GitLab, _) => format!(
            "https://gitlab.com/api/v4/projects/{owner}%2F{repo}/repository/tree?path={path}"
        ),
    };

    let res = state
        .update_http
        .get(&url)
        .header("User-Agent", UA)
        // readme/file 要正文而不是 base64 包装
        .header("Accept", "application/vnd.github.raw+json")
        .bearer_auth(&access)
        .send()
        .await
        .map_err(|e| {
            // 报错里不带 URL：reqwest 的 Display 会把上游主机拼进去。
            tracing::warn!(%e, "code host unreachable");
            AppError::internal(format!("{} 暂时不可达", provider.label()))
        })?;

    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(AppError::bad(format!(
            "{} 返回 {}（{}）",
            provider.label(),
            status.as_u16(),
            if status.as_u16() == 404 { "路径不存在" } else { "读取失败" }
        )));
    }
    // 原样回传正文；桌面端按它自己的格式渲染。
    Ok(Json(json!({ "provider": provider.key(), "action": action, "content": text })))
}

    /// 每一次日志外传都是一次密钥外传。
    #[test]
    fn the_token_endpoint_body_never_reaches_a_log() {
        let src = include_str!("integrations.rs");
        let src = &src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())];
        let f = src.split("let access = token").nth(1).expect("token exchange");
        let f = &f[..f.find("let refresh").unwrap_or(f.len())];
        assert!(
            !f.contains("{token}"),
            "响应体被插进了错误信息；交换失败时它含 client_secret",
        );
        assert!(
            f.contains("provider returned no access_token"),
            "应当只报缺失，不报内容",
        );
    }

/// token: the token stays on this server, and the IDE gets a list of names.
pub async fn repos(
    State(state): State<AppState>,
    claims: Claims,
    Path(provider): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let provider = provider_or_400(&provider)?;
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let access: Option<String> = sqlx::query_scalar(
        "SELECT access_token FROM connected_accounts WHERE user_id = $1 AND provider = $2",
    )
    .bind(uid)
    .bind(provider.key())
    .fetch_optional(&state.db)
    .await?;
    let access = access.ok_or_else(|| AppError::bad(format!("尚未连接 {}", provider.label())))?;
    // 存的是密文（fc1:...）或遗留明文；解开再拿去认证 provider。解不开是配置事故
    // （密钥没配却存了密文、或密钥换了），明确报错，别把 fc1:... 当令牌发出去。
    let access = field_crypto::decrypt(&access, ACCESS_CTX)
        .map_err(|_| AppError::internal("连接令牌解密失败，请重新连接"))?;

    let url = match provider {
        Provider::GitHub => "https://api.github.com/user/repos?per_page=100&sort=updated",
        Provider::GitLab => "https://gitlab.com/api/v4/projects?membership=true&per_page=100&order_by=updated_at",
    };
    let body: serde_json::Value = state
        .update_http
        .get(url)
        .header("User-Agent", UA)
        .bearer_auth(&access)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("{} 不可达：{e}", provider.label())))?
        .json()
        .await
        .map_err(|_| AppError::internal("代码托管平台返回了无法解析的响应"))?;

    // An expired or revoked token comes back as an object with a message, not a list.
    let Some(list) = body.as_array() else {
        return Err(AppError::bad(format!(
            "{} 拒绝了这次请求，可能需要重新连接",
            provider.label()
        )));
    };

    // The two APIs describe the same thing with different words, and the editor should
    // not have to care which host a repository came from.
    let str_of = |r: &serde_json::Value, keys: &[&str]| -> String {
        keys.iter()
            .find_map(|k| r.get(*k).and_then(|v| v.as_str()))
            .unwrap_or_default()
            .to_owned()
    };
    let repos: Vec<serde_json::Value> = list
        .iter()
        .map(|r| {
            // Computed outside the macro: json! takes expressions, not blocks.
            let branch = {
                let b = str_of(r, &["default_branch"]);
                if b.is_empty() { "main".to_owned() } else { b }
            };
            // GitHub has a boolean; GitLab has visibility public/internal/private.
            let private = r.get("private").and_then(|v| v.as_bool()).unwrap_or_else(|| {
                r.get("visibility").and_then(|v| v.as_str()).unwrap_or("private") != "public"
            });
            json!({
                "full_name": str_of(r, &["full_name", "path_with_namespace"]),
                "name": str_of(r, &["name"]),
                "private": private,
                "default_branch": branch,
                "description": str_of(r, &["description"]),
                "clone_url": str_of(r, &["clone_url", "http_url_to_repo"]),
                "html_url": str_of(r, &["html_url", "web_url"]),
                "updated_at": str_of(r, &["updated_at", "last_activity_at"]),
            })
        })
        .collect();

    Ok(Json(json!({ "provider": provider.key(), "repos": repos })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_two_known_providers_are_accepted() {
        assert_eq!(Provider::parse("github"), Some(Provider::GitHub));
        assert_eq!(Provider::parse("gitlab"), Some(Provider::GitLab));
        // Path segments come straight off the URL; anything else must not reach a
        // credential lookup or a database write.
        assert_eq!(Provider::parse("GitHub"), None);
        assert_eq!(Provider::parse("gitee"), None);
        assert_eq!(Provider::parse("../github"), None);
        assert_eq!(Provider::parse(""), None);
    }

    #[test]
    fn scopes_stay_least_privilege() {
        // A widening here is a real escalation across every linked account, so it should
        // have to break a test to happen.
        assert_eq!(Provider::GitHub.scope(), "repo read:user");
        assert_eq!(Provider::GitLab.scope(), "read_api read_user read_repository");
        for p in [Provider::GitHub, Provider::GitLab] {
            assert!(
                !p.scope().contains("delete") && !p.scope().contains("admin"),
                "{} must not ask for destructive scopes",
                p.label()
            );
        }
    }
}
