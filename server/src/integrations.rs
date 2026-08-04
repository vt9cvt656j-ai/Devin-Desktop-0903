//! Linking a GitHub or Gitee account, so the IDE can offer someone's own repositories
//! behind `@github:` and `@gitee:`.
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
//! GitHub/Gitee identity, so this ships with no defaults and reports `configured: false`
//! until the environment carries an id and secret. A provider that is not configured is
//! not offered — better than a button that leads to a provider error page.

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
    Gitee,
}

impl Provider {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "github" => Some(Self::GitHub),
            "gitee" => Some(Self::Gitee),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::Gitee => "gitee",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::Gitee => "Gitee",
        }
    }

    fn credentials(self, state: &AppState) -> (String, String) {
        match self {
            Self::GitHub => (
                state.cfg.github_client_id.clone(),
                state.cfg.github_client_secret.clone(),
            ),
            Self::Gitee => (
                state.cfg.gitee_client_id.clone(),
                state.cfg.gitee_client_secret.clone(),
            ),
        }
    }

    /// Least privilege that still does the job.
    ///
    /// GitHub's `repo` is coarse — it is the only scope that can see private
    /// repositories, and it grants write with it. `read:user` is separate so the page
    /// can say who is connected. Gitee splits them properly.
    fn scope(self) -> &'static str {
        match self {
            Self::GitHub => "repo read:user",
            Self::Gitee => "projects user_info",
        }
    }

    fn authorize_url(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/login/oauth/authorize",
            Self::Gitee => "https://gitee.com/oauth/authorize",
        }
    }

    fn token_url(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/login/oauth/access_token",
            Self::Gitee => "https://gitee.com/oauth/token",
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

    let providers: Vec<serde_json::Value> = [Provider::GitHub, Provider::Gitee]
        .iter()
        .map(|p| {
            let (id, secret) = p.credentials(&state);
            let configured = !id.trim().is_empty() && !secret.trim().is_empty();
            let linked = rows.iter().find(|r| r.provider == p.key());
            json!({
                "provider": p.key(),
                "label": p.label(),
                "configured": configured,
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
    // this check a state minted for GitHub would complete a Gitee link.
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
        Provider::Gitee => {
            // Gitee takes these as query parameters, not a form body.
            state
                .update_http
                .post(provider.token_url())
                .query(&[
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
        Provider::Gitee => "https://gitee.com/api/v5/user",
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
        login: body
            .get("login")
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
            Provider::Gitee => "https://gitee.com/oauth/applications",
        }
    })))
}

// ── GET /api/integrations/:provider/repos ────────────────────────────────────────────

/// What `@github:` and `@gitee:` offer. Proxied rather than letting the IDE hold the
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
        Provider::Gitee => "https://gitee.com/api/v5/user/repos?per_page=100&sort=updated",
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

    let repos: Vec<serde_json::Value> = list
        .iter()
        .map(|r| {
            json!({
                "full_name": r.get("full_name").and_then(|v| v.as_str()).unwrap_or_default(),
                "name": r.get("name").and_then(|v| v.as_str()).unwrap_or_default(),
                "private": r.get("private").and_then(|v| v.as_bool()).unwrap_or(false),
                "default_branch": r.get("default_branch").and_then(|v| v.as_str()).unwrap_or("main"),
                "description": r.get("description").and_then(|v| v.as_str()).unwrap_or_default(),
                "clone_url": r.get("clone_url").and_then(|v| v.as_str()).unwrap_or_default(),
                "html_url": r.get("html_url").and_then(|v| v.as_str()).unwrap_or_default(),
                "updated_at": r.get("updated_at").and_then(|v| v.as_str()).unwrap_or_default(),
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
        assert_eq!(Provider::parse("gitee"), Some(Provider::Gitee));
        // Path segments come straight off the URL; anything else must not reach a
        // credential lookup or a database write.
        assert_eq!(Provider::parse("GitHub"), None);
        assert_eq!(Provider::parse("gitlab"), None);
        assert_eq!(Provider::parse("../github"), None);
        assert_eq!(Provider::parse(""), None);
    }

    #[test]
    fn scopes_stay_least_privilege() {
        // A widening here is a real escalation across every linked account, so it should
        // have to break a test to happen.
        assert_eq!(Provider::GitHub.scope(), "repo read:user");
        assert_eq!(Provider::Gitee.scope(), "projects user_info");
        for p in [Provider::GitHub, Provider::Gitee] {
            assert!(
                !p.scope().contains("delete") && !p.scope().contains("admin"),
                "{} must not ask for destructive scopes",
                p.label()
            );
        }
    }
}
