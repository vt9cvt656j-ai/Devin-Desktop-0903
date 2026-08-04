//! Linking a GitHub or GitLab account, so the IDE can offer someone's own repositories
//! behind `@github:` and `@gitlab:`.
//!
//! Three things decide the shape of this module.
//!
//! **The tokens are the crown jewels.** A GitHub token with `repo` scope is read/write
//! access to every private repository the person owns. So: no endpoint here ever returns
//! one, they are stored server-side and only ever spent by this server calling the
//! provider on the person's behalf, and the repo listing goes through us rather than
//! handing the browser a token to call GitHub with directly.
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
use crate::AppState;

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

/// What the `state` parameter carries across the redirect. `nonce` is not checked
/// against a store — the signature plus the short expiry is what makes it unforgeable —
/// but it keeps two flows started in the same second from producing an identical string.
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
) -> ApiResult<Json<serde_json::Value>> {
    let provider = provider_or_400(&provider)?;
    let (client_id, secret) = provider.credentials(&state);
    if client_id.trim().is_empty() || secret.trim().is_empty() {
        return Err(AppError::bad(format!(
            "{} 尚未在本服务器上配置",
            provider.label()
        )));
    }

    let state_token = encode(
        &Header::default(),
        &StateClaims {
            sub: claims.sub.clone(),
            provider: provider.key().to_owned(),
            nonce: uuid::Uuid::new_v4().to_string(),
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

    Ok(Json(json!({ "url": url.to_string() })))
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

    match finish(&state, provider, &code, &state_token).await {
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
        .ok_or_else(|| anyhow::anyhow!("no access_token in response: {token}"))?
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
    .bind(&access)
    .bind(refresh.as_deref())
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
    .bind(&token)
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
