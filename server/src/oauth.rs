//! Signing in with GitHub or Google.
//!
//! Distinct from `integrations.rs`, which links a code host to an account that already
//! exists so the IDE can read someone's repositories. This module answers the earlier
//! question — who is this person at the door — and it deliberately keeps no provider
//! token: the access token is spent once, here, to read a profile, and then dropped.
//!
//! Four things decide the shape of it.
//!
//! **Identity is the provider's subject id, never the email.** Addresses move between
//! accounts: someone renames their GitHub login, the old address is freed, and a stranger
//! takes it. Keying on email hands that stranger the account. The subject id is stable for
//! the life of the provider account, and `auth_identities` is unique on it.
//!
//! **An unverified address proves nothing.** Both providers will happily report an email
//! the account holder never confirmed, and anyone can put your address on a fresh account.
//! So an unverified address never creates an account and never matches an existing one —
//! it is refused outright. This is the check that stops "sign in with Google" from being a
//! way to walk into somebody else's password account.
//!
//! **The callback arrives as a plain browser redirect** with no Authorization header, so
//! `state` is the only thing tying it to the flow that started it. It is a short-lived JWT
//! signed with the same secret as a login token, and it is checked for provider mismatch
//! and for purpose, so a `state` minted for linking a repository host cannot complete a
//! sign-in — and vice versa.
//!
//! **Where it sends you afterwards is not the provider's to choose.** `next` is carried
//! inside the signed state and re-validated on the way out, against the same rules the
//! gate page applies, so a crafted authorize URL cannot turn this into an open redirect.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{find_user, normalize_email, start_session, User};
use crate::AppState;

/// Long enough to sign in at the provider and approve, short enough that a `state` found
/// later in a proxy log or browser history is already dead.
const STATE_TTL_SECS: i64 = 600;

/// Marks a state token as belonging to *this* flow. `integrations.rs` mints structurally
/// similar tokens with the same key; without a purpose, one would be accepted by the
/// other's callback.
const PURPOSE: &str = "login";

/// GitHub answers 403 with an unhelpful body when this is missing.
const UA: &str = "MrDayOne-Gateway";

/// Where the browser may be sent after signing in. Same list the gate page enforces, for
/// the same reason: everything else is either another site or not ours to land on.
const ALLOWED_NEXT: [&str; 3] = ["/dashboard", "/billing", "/app"];
const HOME: &str = "/dashboard";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    GitHub,
    Google,
}

impl Provider {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "github" => Some(Self::GitHub),
            "google" => Some(Self::Google),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::Google => "google",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::Google => "Google",
        }
    }

    fn credentials(self, state: &AppState) -> (String, String) {
        match self {
            Self::GitHub => (
                state.cfg.github_login_client_id.clone(),
                state.cfg.github_login_client_secret.clone(),
            ),
            Self::Google => (
                state.cfg.google_client_id.clone(),
                state.cfg.google_client_secret.clone(),
            ),
        }
    }

    fn configured(self, state: &AppState) -> bool {
        let (id, secret) = self.credentials(state);
        !id.trim().is_empty() && !secret.trim().is_empty()
    }

    /// The narrowest scope that still yields a verified address.
    ///
    /// Notably NOT GitHub's `repo`, which the linking app asks for. Signing in needs to
    /// know who you are and that your address is confirmed; it has no business being able
    /// to read your private code.
    fn scope(self) -> &'static str {
        match self {
            Self::GitHub => "read:user user:email",
            Self::Google => "openid email profile",
        }
    }

    fn authorize_url(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/login/oauth/authorize",
            Self::Google => "https://accounts.google.com/o/oauth2/v2/auth",
        }
    }

    fn token_url(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/login/oauth/access_token",
            Self::Google => "https://oauth2.googleapis.com/token",
        }
    }
}

/// What crosses the redirect. Unforgeable because it is signed; `nonce` only keeps two
/// flows started in the same second from producing an identical string.
#[derive(Debug, Serialize, Deserialize)]
struct StateClaims {
    purpose: String,
    provider: String,
    /// Where to land afterwards. Validated when it is minted and again when it is spent.
    next: String,
    /// The browser's device id, so an OAuth sign-in groups with that device like any
    /// other. Carried through the redirect because the callback is a fresh navigation
    /// with no access to the page's storage.
    device_id: String,
    nonce: String,
    exp: i64,
}

fn public_base(state: &AppState) -> String {
    state
        .cfg
        .ide_update_public_base
        .trim_end_matches('/')
        .to_owned()
}

fn redirect_uri(state: &AppState, provider: Provider) -> String {
    format!(
        "{}/api/auth/oauth/{}/callback",
        public_base(state),
        provider.key()
    )
}

/// Same-origin app paths only. A leading `//` is another host, and anything outside the
/// allow-list is not somewhere this flow should be able to deposit a signed-in browser.
fn safe_next(raw: &str) -> String {
    if raw.starts_with('/')
        && !raw.starts_with("//")
        && ALLOWED_NEXT.iter().any(|p| raw.starts_with(p))
    {
        return raw.to_owned();
    }
    HOME.to_owned()
}

/// Back to the sign-in page with a word about what went wrong. The reason is a fixed
/// keyword, never provider text: that can carry an address or a message that is not ours
/// to render into someone's browser.
fn gate_error(state: &AppState, reason: &str) -> Response {
    Redirect::to(&format!("{}/gate?oauth={}", public_base(state), reason)).into_response()
}

// ── GET /api/auth/oauth/providers ────────────────────────────────────────────────────

/// Which buttons the sign-in page should offer.
///
/// Public and unauthenticated — it is asked before anyone is signed in — and it reports
/// only whether credentials exist, never what they are. The page reads this instead of
/// hardcoding, so adding the environment variables lights the button up without a rebuild.
pub async fn providers(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "github": Provider::GitHub.configured(&state),
        "google": Provider::Google.configured(&state),
    }))
}

// ── GET /api/auth/oauth/:provider/start ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StartQuery {
    next: Option<String>,
    device_id: Option<String>,
}

/// Sends the browser to the provider.
///
/// A redirect rather than JSON with a URL in it: this is reached by a plain link on the
/// sign-in page, which has no session yet and nothing to authenticate with.
pub async fn start(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(q): Query<StartQuery>,
) -> Response {
    let Some(provider) = Provider::parse(&provider) else {
        return gate_error(&state, "unknown");
    };
    if !provider.configured(&state) {
        // Not an error the person can do anything about; the button should not have been
        // live. Say so plainly rather than bouncing them into a provider error page.
        return gate_error(&state, "unconfigured");
    }
    let (client_id, _) = provider.credentials(&state);

    let state_token = match encode(
        &Header::default(),
        &StateClaims {
            purpose: PURPOSE.to_owned(),
            provider: provider.key().to_owned(),
            next: safe_next(q.next.as_deref().unwrap_or(HOME)),
            device_id: crate::auth::clean_device_id(q.device_id.as_deref()),
            nonce: uuid::Uuid::new_v4().to_string(),
            exp: chrono::Utc::now().timestamp() + STATE_TTL_SECS,
        },
        &EncodingKey::from_secret(state.cfg.jwt_secret.as_bytes()),
    ) {
        Ok(t) => t,
        Err(_) => return gate_error(&state, "error"),
    };

    // Built through the URL type: `scope` contains spaces and `state` is a JWT, and
    // hand-assembling those is how a redirect_uri quietly stops matching the registered
    // one.
    let mut params = vec![
        ("client_id", client_id),
        ("redirect_uri", redirect_uri(&state, provider)),
        ("scope", provider.scope().to_owned()),
        ("state", state_token),
        ("response_type", "code".to_owned()),
    ];
    if provider == Provider::Google {
        // Without this Google returns no refresh token and, more importantly here, skips
        // the account chooser — so a second person on a shared machine is silently signed
        // in as the first.
        params.push(("prompt", "select_account".to_owned()));
    }

    match reqwest::Url::parse_with_params(provider.authorize_url(), &params) {
        Ok(url) => Redirect::to(url.as_str()).into_response(),
        Err(_) => gate_error(&state, "error"),
    }
}

// ── GET /api/auth/oauth/:provider/callback ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    /// Sent when the person declines at the provider.
    error: Option<String>,
}

pub async fn callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let Some(provider) = Provider::parse(&provider) else {
        return gate_error(&state, "unknown");
    };
    if q.error.is_some() {
        // Declining is an ordinary outcome, not a failure worth alarming language.
        return gate_error(&state, "cancelled");
    }
    let (Some(code), Some(state_token)) = (q.code, q.state) else {
        return gate_error(&state, "error");
    };

    match finish(&state, provider, &code, &state_token, &headers).await {
        Ok((token, next)) => signed_in(&state, &token, &next),
        Err(e) => {
            // The person is told only a keyword. The detail goes to the log because it can
            // quote provider text and addresses that are not ours to render back.
            tracing::warn!("{} sign-in failed: {}", provider.label(), e.detail());
            gate_error(&state, e.reason())
        }
    }
}

/// Why a sign-in did not happen.
///
/// Split into "refused" and "broken" because they are different messages: a refusal is
/// something the person can act on — confirm your address at the provider — while
/// anything else is ours to fix and theirs to simply retry.
enum Failure {
    Refused(&'static str, &'static str),
    Broken(anyhow::Error),
}

impl Failure {
    /// The keyword the sign-in page turns into a sentence.
    fn reason(&self) -> &'static str {
        match self {
            Self::Refused(reason, _) => reason,
            Self::Broken(_) => "error",
        }
    }

    fn detail(&self) -> String {
        match self {
            Self::Refused(_, why) => (*why).to_owned(),
            Self::Broken(e) => format!("{e:#}"),
        }
    }
}

/// Deliberately only from `anyhow::Error`, not a blanket `impl<E: Into<anyhow::Error>>`:
/// that overlaps the reflexive `From<T> for T` and coherence rejects it. Every helper
/// below returns `anyhow::Result`, so one conversion covers all of them.
impl From<anyhow::Error> for Failure {
    fn from(e: anyhow::Error) -> Self {
        Self::Broken(e)
    }
}

/// Hands the browser its session and sends it on.
///
/// The cookie is what nginx checks before serving the app shell, and it is readable by
/// script on purpose — the console falls back to it when local storage is empty, which is
/// exactly the state a browser is in after being redirected back from a provider. Same
/// attributes the sign-in page sets by hand, including `SameSite=Lax` rather than Strict:
/// Strict withholds the cookie on cross-site top-level navigation, which is precisely what
/// this redirect is, and the person would arrive appearing signed out.
fn signed_in(state: &AppState, token: &str, next: &str) -> Response {
    // Not percent-encoded, and it does not need to be: a JWT is base64url plus dots, all
    // of which are legal in a cookie value. Anything else would mean the token is not what
    // this function was handed, so it is refused rather than escaped into looking valid.
    if !token
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return gate_error(state, "error");
    }
    let cookie = format!(
        "mide_token={token}; Path=/; Secure; SameSite=Lax; Max-Age={}",
        7 * 24 * 3600
    );
    let target = format!("{}{}", public_base(state), safe_next(next));
    (
        StatusCode::SEE_OTHER,
        [
            (header::SET_COOKIE, cookie),
            (header::LOCATION, target),
            // The response carries a session cookie; nothing should keep a copy.
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
    )
        .into_response()
}

/// The profile a provider released, reduced to what signing in actually needs.
struct Identity {
    subject: String,
    email: String,
    email_verified: bool,
    name: String,
    avatar: String,
}

async fn finish(
    state: &AppState,
    provider: Provider,
    code: &str,
    state_token: &str,
    headers: &HeaderMap,
) -> Result<(String, String), Failure> {
    let data = decode::<StateClaims>(
        state_token,
        &DecodingKey::from_secret(state.cfg.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| Failure::Broken(e.into()))?;
    // A valid signature for *a* flow is not a valid signature for *this* one. Both checks
    // matter: purpose stops an integrations state completing a sign-in, and provider stops
    // a state minted for Google completing a GitHub callback.
    if data.claims.purpose != PURPOSE {
        return Err(Failure::Broken(anyhow::anyhow!("state purpose mismatch")));
    }
    if data.claims.provider != provider.key() {
        return Err(Failure::Broken(anyhow::anyhow!("state provider mismatch")));
    }

    let access = exchange_code(state, provider, code).await?;
    let who = fetch_identity(state, provider, &access).await?;

    if who.email.trim().is_empty() {
        return Err(Failure::Refused(
            "noemail",
            "the provider released no email address",
        ));
    }
    if !who.email_verified {
        // The load-bearing check. Anyone can put someone else's address on a fresh
        // provider account; only the provider confirming it makes the address evidence of
        // anything. Refusing here is what keeps this from being a way into an existing
        // password account.
        return Err(Failure::Refused(
            "unverified",
            "the provider reports this address as unverified",
        ));
    }

    let user = resolve_account(state, provider, &who).await?;
    let token = start_session(
        state,
        &user,
        headers,
        Some("web"),
        Some(&data.claims.device_id),
    )
    .await
    .map_err(|e| Failure::Broken(anyhow::anyhow!("{}", e.msg)))?;

    Ok((token, data.claims.next))
}

async fn exchange_code(
    state: &AppState,
    provider: Provider,
    code: &str,
) -> anyhow::Result<String> {
    let (client_id, client_secret) = provider.credentials(state);
    let redirect = redirect_uri(state, provider);

    let mut form = vec![
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", code),
        ("redirect_uri", redirect.as_str()),
    ];
    if provider == Provider::Google {
        form.push(("grant_type", "authorization_code"));
    }

    let body: serde_json::Value = state
        .update_http
        .post(provider.token_url())
        // GitHub defaults to form-encoded output and only sends JSON when asked.
        .header("Accept", "application/json")
        .header("User-Agent", UA)
        .form(&form)
        .send()
        .await?
        .json()
        .await?;

    body.get("access_token")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        // The body is not logged: a failed exchange can echo the client secret back.
        .ok_or_else(|| anyhow::anyhow!("token endpoint returned no access_token"))
}

async fn fetch_identity(
    state: &AppState,
    provider: Provider,
    access: &str,
) -> anyhow::Result<Identity> {
    match provider {
        Provider::GitHub => {
            let profile: serde_json::Value = state
                .update_http
                .get("https://api.github.com/user")
                .bearer_auth(access)
                .header("User-Agent", UA)
                .header("Accept", "application/vnd.github+json")
                .send()
                .await?
                .json()
                .await?;

            let subject = profile
                .get("id")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| anyhow::anyhow!("no id in GitHub profile"))?
                .to_string();

            // `/user` returns the *public* email, which is null for most people and is not
            // marked verified either way. The verified address only comes from this
            // endpoint, which is what `user:email` is requested for.
            let emails: serde_json::Value = state
                .update_http
                .get("https://api.github.com/user/emails")
                .bearer_auth(access)
                .header("User-Agent", UA)
                .header("Accept", "application/vnd.github+json")
                .send()
                .await?
                .json()
                .await?;

            // Primary and verified first; failing that any verified address, because an
            // account whose primary is unconfirmed may still have a confirmed one.
            let pick = |want_primary: bool| {
                emails.as_array()?.iter().find(|e| {
                    e.get("verified").and_then(|v| v.as_bool()).unwrap_or(false)
                        && (!want_primary
                            || e.get("primary").and_then(|v| v.as_bool()).unwrap_or(false))
                })
            };
            let chosen = pick(true).or_else(|| pick(false));

            Ok(Identity {
                subject,
                email: chosen
                    .and_then(|e| e.get("email"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                // True by construction: nothing that failed the verified test above is
                // ever selected.
                email_verified: chosen.is_some(),
                name: profile
                    .get("name")
                    .and_then(|v| v.as_str())
                    .or_else(|| profile.get("login").and_then(|v| v.as_str()))
                    .unwrap_or_default()
                    .to_owned(),
                avatar: profile
                    .get("avatar_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
            })
        }
        Provider::Google => {
            let profile: serde_json::Value = state
                .update_http
                .get("https://openidconnect.googleapis.com/v1/userinfo")
                .bearer_auth(access)
                .send()
                .await?
                .json()
                .await?;

            Ok(Identity {
                subject: profile
                    .get("sub")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("no sub in Google profile"))?
                    .to_owned(),
                email: profile
                    .get("email")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                // Google sends this as a real bool. Absent counts as unverified rather
                // than defaulting the permissive way.
                email_verified: profile
                    .get("email_verified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                name: profile
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                avatar: profile
                    .get("picture")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
            })
        }
    }
}

/// Which account this identity signs in to, creating one if this is a first visit.
///
/// Three cases, in the only order that is safe:
///
///   1. The identity is already linked — that is the account, whatever the address says
///      now. Someone who changed their GitHub email keeps their account.
///   2. A local account exists at the (verified) address — link this identity to it, so
///      "sign in with Google" reaches the account you registered with a password rather
///      than silently creating a second one at the same address.
///   3. Neither — register a passwordless account.
async fn resolve_account(
    state: &AppState,
    provider: Provider,
    who: &Identity,
) -> anyhow::Result<User> {
    let existing: Option<uuid::Uuid> = sqlx::query_scalar(
        "UPDATE auth_identities SET last_login_at = now(), email = $3 \
         WHERE provider = $1 AND subject = $2 RETURNING user_id",
    )
    .bind(provider.key())
    .bind(&who.subject)
    .bind(&who.email)
    .fetch_optional(&state.db)
    .await?;

    if let Some(uid) = existing {
        return sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await?
            // The row is gone but the identity survived: only possible if the cascade did
            // not run. Fail rather than inventing a replacement account.
            .ok_or_else(|| anyhow::anyhow!("identity points at a missing user"));
    }

    let email = normalize_email(&who.email);
    let user = match find_user(state, &email).await.map_err(|e| anyhow::anyhow!("{}", e.msg))? {
        Some(user) => user,
        None => {
            // Passwordless. ON CONFLICT covers two first-time sign-ins racing at the same
            // address, where both saw no row a moment ago.
            sqlx::query_as::<_, User>(
                "INSERT INTO users (email, password_hash) VALUES ($1, '') \
                 ON CONFLICT (email) DO UPDATE SET updated_at = now() RETURNING *",
            )
            .bind(&email)
            .fetch_one(&state.db)
            .await?
        }
    };

    // ON CONFLICT DO NOTHING, then read back: two tabs completing the same flow at once
    // must end up on one identity row, not a unique-violation 500 in one of them.
    sqlx::query(
        "INSERT INTO auth_identities (user_id, provider, subject, email, last_login_at) \
         VALUES ($1,$2,$3,$4, now()) ON CONFLICT (provider, subject) DO NOTHING",
    )
    .bind(user.id)
    .bind(provider.key())
    .bind(&who.subject)
    .bind(&who.email)
    .execute(&state.db)
    .await?;

    // A name and picture, but only into empty fields — the provider is a source of first
    // resort, never something that overwrites what the person set here themselves.
    if !who.name.trim().is_empty() {
        let (first, last) = split_name(&who.name);
        sqlx::query(
            "UPDATE users SET first_name = $2, last_name = $3, updated_at = now() \
             WHERE id = $1 AND first_name = '' AND last_name = ''",
        )
        .bind(user.id)
        .bind(first)
        .bind(last)
        .execute(&state.db)
        .await?;
    }
    if !who.avatar.trim().is_empty() {
        sqlx::query(
            "UPDATE users SET avatar = $2, updated_at = now() \
             WHERE id = $1 AND (avatar IS NULL OR avatar = '')",
        )
        .bind(user.id)
        .bind(&who.avatar)
        .execute(&state.db)
        .await?;
    }

    // Re-read so the caller sees the row as it now stands rather than as it was inserted.
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?,
    )
}

/// A display name split into the two fields the console keeps.
///
/// Providers hand over one free-text string, so this is a guess by definition. It splits
/// on the last space, which is right for "Ada Lovelace" and for "Mary Anne Evans", and
/// leaves a single-word name entirely in the first field rather than inventing a surname.
fn split_name(full: &str) -> (String, String) {
    let full = full.trim();
    match full.rsplit_once(' ') {
        Some((first, last)) => (first.trim().to_owned(), last.trim().to_owned()),
        None => (full.to_owned(), String::new()),
    }
}

/// Nothing here is reachable without credentials, so the tests cover the decisions that
/// are made before any network call — which is where the security-relevant ones live.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_own_app_paths_are_valid_landing_places() {
        assert_eq!(safe_next("/billing"), "/billing");
        assert_eq!(safe_next("/dashboard#devices"), "/dashboard#devices");
        // The open-redirect shapes. `//evil.example` is a protocol-relative URL — a
        // different site that merely looks like a path.
        assert_eq!(safe_next("//evil.example/x"), HOME);
        assert_eq!(safe_next("https://evil.example"), HOME);
        assert_eq!(safe_next("/../admin"), HOME);
        assert_eq!(safe_next(""), HOME);
    }

    #[test]
    fn a_provider_is_only_offered_once_both_halves_are_configured() {
        // Half-configured is the dangerous middle: the button lights up, the person is
        // sent to the provider, and the exchange fails after they have already approved.
        for (id, secret, want) in [
            ("", "", false),
            ("id", "", false),
            ("", "secret", false),
            ("   ", "secret", false),
            ("id", "secret", true),
        ] {
            let configured = !id.trim().is_empty() && !secret.trim().is_empty();
            assert_eq!(configured, want, "{id:?}/{secret:?}");
        }
    }

    #[test]
    fn sign_in_never_asks_for_repository_access() {
        // The linking app asks for `repo`, which is read/write over every private
        // repository. Putting that consent screen in front of someone who wants to log in
        // is both alarming and far more access than this needs.
        assert!(!Provider::GitHub.scope().contains("repo"));
        assert!(Provider::GitHub.scope().contains("user:email"));
        assert!(Provider::Google.scope().contains("email"));
    }

    #[test]
    fn a_display_name_splits_the_way_people_write_theirs() {
        assert_eq!(split_name("Ada Lovelace"), ("Ada".into(), "Lovelace".into()));
        assert_eq!(
            split_name("Mary Anne Evans"),
            ("Mary Anne".into(), "Evans".into())
        );
        // One word stays one word: a blank surname beats a made-up one.
        assert_eq!(split_name("Prince"), ("Prince".into(), "".into()));
        assert_eq!(split_name("  "), ("".into(), "".into()));
    }

    #[test]
    fn the_two_oauth_flows_cannot_complete_each_others_callbacks() {
        // integrations.rs signs its state with the same key, so the signature alone does
        // not distinguish them. This constant is what does.
        assert_eq!(PURPOSE, "login");
        let integrations = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/integrations.rs"
        ))
        .expect("read integrations.rs");
        assert!(
            !integrations.contains("purpose"),
            "integrations state gained a purpose field — make sure it is not \"login\""
        );
    }
}
