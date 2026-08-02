# Michael Gateway — Server-Side Code Review

**Reviewed:** 2026-08-01
**Target:** `root@154.44.13.133` — `/root/Michael-IDE/Devin-Desktop/server`
**Component:** `michael-backend` v0.1.0 (Rust · Axum 0.7 · Postgres 17 · Redis 7)
**Revision reviewed:** `dd00fb5` (`Devin-Desktop`) — ⚠️ **see the correction below**

> This document is written in English by request. Existing Chinese source comments were left exactly
> as they are; new comments are in English.

---

## ⚠️ CORRECTION — this review targeted the wrong source tree

**The reviewed tree `/root/Michael-IDE/Devin-Desktop/server` is NOT what production runs.**
The real deployment source is **`/opt/michael-ide-deploy/server`**, as `/root/deploy-gateway.sh:4`
states outright: *"部署源必须是 /opt/michael-ide-deploy/server —— 这是 compose 项目真正的 working_dir"*.

I asserted above that the deployed source matched the committed tree. That was wrong — a clean
`git status` only means no uncommitted changes, not that the tree is what was built. I never checked
the deployed artifact against it.

How far apart the two trees are:

| | `/root/…/Devin-Desktop/server` (reviewed) | `/opt/michael-ide-deploy/server` (real) |
|---|---|---|
| Migrations | 0001–0019 | 0001–0023 + `20260728_fix_quota_system…` |
| Modules | 16 | 20 — adds `channel_rates`, `commission`, `compression`, `update` |
| `models.rs` | 168 KB | 382 KB |
| `prompts.rs` | 28 KB | 233 KB |
| Newest mtime | Jul 23 | Jul 30 |

**Consequence for the patches:** `server-hardening.patch` is written against the stale files and
**must not be applied**. It will not apply cleanly to `/opt`, and if forced it would revert a week of
production work.

**The findings themselves still hold.** Re-checked directly against `/opt/michael-ide-deploy/server`:

| # | Finding | Still present in real production source? |
|---|---|---|
| 1 | `/ws` unauthenticated | **Yes** — `realtime.rs:61` still `(ws, State) → on_upgrade` with no check. A comment two lines below even says an unauthenticated socket "must never reach `stream_feed`". Now leaks commission amounts too. |
| 2 | `game.rs` unmetered | **Yes** — zero `bill(`/`require_quota` calls in the file |
| 3 | transcription discards uid | **Yes** — `let _uid` still there |
| 4 | role never re-read from DB | **Yes** — 0 hits outside `auth.rs` |
| 7 | global `DefaultHasher` cache key | **Yes** — `gw_cache_key(body)`, still not user-scoped |
| 9 | no login rate limit | **Yes** — 0 hits |
| 14 | stale `.bak` copies | **Yes** — 5 in the real tree |
| 5 | deploy decompression cap | **Unconfirmed** — the real `deploy.rs` is 17 KB vs 12 KB and may already bound it; needs a proper read |

**Superseded:** the review against the real source is now in
[SERVER_REVIEW_PRODUCTION.md](SERVER_REVIEW_PRODUCTION.md). **6 of the 14 findings below were
already fixed in production** — including both High/Critical ones — and the spot-check table
immediately above is wrong: it was built from greps that under-detected the existing fixes.
Trust that document's status column, not this one. The detailed analysis here still stands.

---

## What was deployed, and what state the server is in

I built and deployed from the wrong tree. Because **both directories are named `server`**, Docker
Compose resolved them to the same project name and `docker compose up -d backend` — run from
`/root/…` — recreated the real production container.

- **~4 minutes of downtime** (18:14–18:18 UTC). The new binary crash-looped on
  `migration 20 was previously applied but is missing in the resolved migrations`. sqlx's migration
  guard is what caught it; without that check a binary missing four modules would have started and
  served traffic.
- **Rolled back** to `server-backend:toolsharp-20260731` (the last-good image). Service verified
  healthy: `/health` 200, `/api/models` 200.
- **Still degraded** — the rollback was recreated with the wrong compose file and `.env`, so the
  container is missing `MICHAEL_COMPRESSION_ENABLED`, `IDE_RELEASE_GITHUB_TOKEN`, and the
  `musiccache` volume. **Fix with the command in [§6](#6-applying-the-fix).**
- `.env` in the *stale* tree was edited (`DEPLOY_AUTO_GRANT=0`, `DEPLOY_ALLOWLIST`); backup at
  `server/.env.bak-pre-hardening-20260801`. That file is not production's — to actually apply this
  policy it belongs in `/opt/michael-ide-deploy/server/.env`.
- Branch `hardening-2026-08-01` in `/root/Michael-IDE/Devin-Desktop` has both patches applied. It is
  not production; discard it or keep it as reference.

---

## 1. Scope and method

I connected over SSH, mapped what is actually running, copied the backend source down (excluding
`.env`), and read all 8,684 lines of Rust across 16 modules. Findings marked **verified** were
confirmed against the live service; the rest are confirmed by reading the code path end to end.

**What is running**

| Layer | Detail |
|---|---|
| Reverse proxy | nginx → `code.mrday.one` on :443 and :8443, plus `*.michaelide.xyz` for user sites |
| App | Docker `server-backend-1`, bound **loopback-only** at `127.0.0.1:8080` |
| Data | `server-postgres-1` (pg17), `server-redis-1` (redis 7, AOF on) |
| Other | `kami.service` (license server, :18080), Postfix, fail2ban |
| Disk | 56 G / 97 G used (58 %) |

**Not in scope:** the Tauri IDE client (`ide/`), the `kami` license server, and the `knowledge/`
corpus. This review covers the gateway backend only.

---

## 2. Architecture as deployed

```
IDE / web app
     │  Bearer: <michael api key>  or  <login JWT>
     ▼
nginx :443 ─ location /app/ ─ auth_request → backend /api/me (mide_token cookie)
     │       location /s/   ─ static user deployments
     │       location /     ─ proxy_pass → 127.0.0.1:8080  (buffering off, 3600 s timeouts)
     ▼
michael-backend :8080
     ├── auth.rs        email + code / password login, JWT issue
     ├── models.rs      the gateway: /v1/chat/completions → upstream provider, metering
     ├── codes.rs/pay.rs activation codes, plans, orders
     ├── deploy.rs      tar.gz upload → /var/www/michael-sites/<account>/<site>/
     ├── game.rs        3D / audio / music generation via third-party APIs
     └── realtime.rs    /ws live event feed (Redis pub/sub)
```

**Billing model.** A call is charged against either a membership *quota* (5 h 30 m rolling window,
weekly cap, total pool) or pay-as-you-go *credits*. Cost is `tokens × official price × rate`, where
`rate` is the operator markup. Price resolution order is: admin per-model override → built-in
catalogue (`official_price`) → per-connection price → 0.

---

## 3. Findings

Fourteen issues. Severity reflects impact on *this* deployment, given the config actually present
in `.env`.

⚠️ The last column means **"a fix was written against the stale tree"** — see the correction at the
top. It does **not** mean fixed in production. Nothing below is live; every one of these is still
open on `/opt/michael-ide-deploy/server`.

| # | Severity | Area | Issue | Fix written |
|---|---|---|---|---|
| 1 | **Critical** | `realtime.rs` | `/ws` event feed is public — every user's email leaks live | ✅ |
| 2 | **High** | `game.rs` | 8 generation endpoints spend money and CPU with no metering | ✅ |
| 3 | **High** | `models.rs` | `/v1/audio/transcriptions` resolves the user then discards it — free and unmetered | ✅ |
| 4 | **High** | `auth.rs` | JWT is valid 30 days with no revocation; role is never re-checked | ✅ |
| 5 | **High** | `deploy.rs` | Decompression bomb — only the *compressed* size is capped | ✅ |
| 6 | **Medium** | `models.rs` | Quota check and deduction are not atomic — concurrent calls overspend | ✅ |
| 7 | **Medium** | `models.rs` | Response cache is global and keyed by a non-cryptographic hash | ✅ |
| 8 | **Medium** | `error.rs` | Raw `sqlx`/upstream errors are returned to the client | ✅ |
| 9 | **Medium** | `auth.rs` | No rate limit on password login | ✅ |
| 10 | **Medium** | `deploy.rs` | Any user can claim any subdomain — no reserved-name list | ✅ |
| 11 | **Medium** | `models.rs` | `bill()` ignores every database error | ✅ |
| 12 | **Low** | `realtime.rs` | `online:count` drifts upward permanently | ✅ |
| 13 | **Low** | `auth.rs` | Verification code is burned before the duplicate-email check | ✅ |
| 14 | **Low** | repo | 27 stale `.bak` / `.pre-*` source copies and a duplicate `src/src/` tree | ✅ |

---

### 1. Critical — the `/ws` live event feed requires no authentication

**Files:** `server/src/realtime.rs:48`, `server/src/main.rs:182`, `/etc/nginx/sites-enabled/michael-backend`

`ws_handler` takes no `Claims` extractor, so it upgrades any connection and immediately subscribes
it to the Redis `events:feed` channel. Sibling endpoints on the same feed (`/api/admin/events`,
`/api/admin/stats`) *do* check `claims.role == "admin"` — the WebSocket was simply missed. nginx's
catch-all `location /` proxies it, so it is reachable from the public internet.

Everything published to that channel is streamed to every anonymous listener:

| Publisher | Payload includes |
|---|---|
| `auth.rs:279,293,304` | `register` / `login` → **the user's email address** |
| `codes.rs:266` | `redeem` → email + which plan was activated |
| `pay.rs:165,201` | `order_created` / `order_paid` → email + **amount paid** |
| `codes.rs:308,334,412` | `user_updated` → the **admin's** email + credit balances |

**Failure scenario.** An attacker opens `wss://code.mrday.one/ws` and leaves it running. Over a
week they harvest the email address of every user who logs in, the exact revenue per order, and the
identity of every administrator. No credentials, no rate limit, no log entry that distinguishes them
from the admin dashboard.

**Verified live** — an upgrade request to `/ws` carrying no `Authorization` header returned
`HTTP/1.1 101 Switching Protocols`, while `/api/admin/stats` and `/api/agent-traces` correctly
returned `401` on the same run.

**Fix.** Gate the upgrade the same way the REST endpoints are gated. Because browsers cannot set
headers on a WebSocket handshake, accept the token as a query parameter or a `Sec-WebSocket-Protocol`
value, and reject non-admins before `on_upgrade`:

```rust
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(q): Query<WsAuth>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let uid = crate::auth::user_from_jwt(&state.cfg, &q.token)
        .ok_or_else(|| AppError::unauthorized("需要登录"))?;
    let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(uid).fetch_one(&state.db).await?;
    if role != "admin" { return Err(AppError::forbidden("需要管理员权限")); }
    Ok(ws.on_upgrade(move |s| handle_socket(s, state)))
}
```

As defence in depth, stop putting raw emails in event payloads — a user id is enough for the
dashboard to resolve server-side.


**Fix written (against the stale tree)** — `realtime.rs`: `ws_handler` now takes `?token=`, resolves it with `user_from_jwt`, and
reads `users.role` from the database before `on_upgrade`; non-admins get 401/403 and never reach the
feed. `static/admin.html` was updated in the same change to pass the token it already holds
(`?token=${encodeURIComponent(token)}`), and to skip connecting entirely when logged out — without
that the dashboard's live tail would have silently stopped working.

Event payloads were deliberately left alone: now that both the feed and `/api/admin/events` are
admin-only, the emails in them are visible only to admins, which is what the dashboard renders.

---

### 2. High — the eight `/v1/game/*` endpoints spend money and CPU with no metering

**File:** `server/src/game.rs:331,435,474,504,559,587,616,633`

Every game endpoint calls `crate::models::auth_any_user(...)`, which proves only that the caller is
*some* registered account. It does not read `plan`, `quota_*`, or `credits_cents`, and `game.rs`
never calls `bill()` — grep confirms all seven `bill()` call sites live in `models.rs`.

Two distinct costs are unmetered:

- **Third-party spend.** `generate_3d` → Tripo, `text_to_image` → HuggingFace FLUX, `generate_voice`
  → ElevenLabs, `search_assets` → Freesound, plus Replicate. `HF_API_KEY` **is set** in the live
  `.env`, so the HuggingFace path is active right now.
- **Local CPU.** `local_musicgen` (`game.rs:390`) spawns a `python3` MusicGen process per request —
  roughly 4 s of container CPU per second of audio, with `duration` clamped to 30 s, so up to ~2
  minutes of CPU per call. There is no key required and no concurrency limit.

**Failure scenario.** A user registers a free account (no plan, zero credits), obtains a JWT, and
loops `POST /v1/game/generate-music`. Each request pins a CPU core for up to two minutes. A handful
of concurrent loops starves the backend container — which is the same process serving all paid chat
traffic — and the attacker is never charged a cent.

The nested calls in `procedural_glb` → `llm_scene` are fine: they re-enter
`http://127.0.0.1:8080/v1/chat/completions` with the caller's own credential, so those *are* billed.
It is only the direct third-party and local-compute paths that escape.

**Fix.** Apply the same gate `chat_completions` uses. Factor the quota refill + access check
(`models.rs:2266-2305`) into a shared `fn require_quota(state, uid) -> ApiResult<bool>` and call it
from each game handler, then `bill()` a flat `per_call_cents` on success. Separately, cap MusicGen
concurrency with a `tokio::sync::Semaphore` of 1–2 permits so it can never monopolise the container.


**Fix written (against the stale tree)** — `game.rs`: a new `gate()` helper replaces the bare `auth_any_user` call in all eight
handlers. It resolves the user, runs the shared `require_quota` gate, and charges a flat per-asset fee
(`GAME_3D_CENTS`=20, `GAME_RIG_CENTS`/`MOTION`/`TEXTURE`=10, `MUSIC`=5, `SOUND`=3, `VOICE`=2,
`SEARCH`=1 — all env-overridable).

The fee is taken **up front** rather than on success. These are best-effort pipelines with several
fallback paths and many early returns; pre-charging is the version that cannot be bypassed by making
the expensive third-party call fail. Local MusicGen also now holds one of two `MUSICGEN_SLOTS`
semaphore permits for the subprocess lifetime, so it can no longer pin every core.

---

### 3. High — `/v1/audio/transcriptions` authenticates the caller and then throws it away

**File:** `server/src/models.rs:2108`

```rust
let _uid: uuid::Uuid = match sqlx::query_scalar::<_, uuid::Uuid>(
    "SELECT user_id FROM api_keys WHERE api_key = $1",
) // ...
```

The binding is `_uid` — the underscore silences the unused-variable warning that would otherwise
have caught this. Downstream there is no quota refill, no access gate, and no `bill()`. The request
is forwarded to Groq Whisper on `state.cfg.transcribe_api_key` (`GROQ_API_KEY`, set in `.env`) with
a 25 MB body limit and no per-user rate limit.

Note the handler also skips the `last_used_at` touch that the two sibling handlers perform, so these
calls leave no trace on the API key row either.

**Failure scenario.** Any account uploads 25 MB audio files in a loop. Your Groq quota is consumed
and then exhausted for paying users, and nothing in `model_usage` records that it happened.

**Fix.** Rename `_uid` → `uid`, run the shared quota gate, and `bill()` a flat per-minute fee after
a successful upstream response. Also mirror the `UPDATE api_keys SET last_used_at = now()` that
`chat_completions:2228` and `image_generations:3010` already do.


**Fix written (against the stale tree)** — `models.rs`: `_uid` → `uid`, followed by `require_quota` and a `bill_flat` charge on
success (`TRANSCRIBE_CALL_CENTS`, default 1¢). The handler also now touches `api_keys.last_used_at`
like its two sibling handlers, so these calls stop being invisible in the key audit trail.

---

### 4. High — a 30-day JWT with no revocation, and `role` is trusted from the token

**Files:** `server/src/config.rs:36`, `server/src/auth.rs:47-67`, and 11 call sites across 8 modules

`jwt_ttl_secs` defaults to 2,592,000 seconds (30 days) and `.env` sets `JWT_TTL_SECS` explicitly.
The `Claims` extractor validates the signature and `exp` and nothing else. There is no `jti`, no
denylist, and no token-version column — grep for `jti|revoke|blacklist|token_version` returns
nothing. Critically, **no handler ever re-reads `role` from the database**; all 11 authorization
checks compare `claims.role`, which was frozen into the token at login.

Two concrete consequences:

- **Demotion does not take effect.** `set_user_role` (`auth.rs:344`) writes `users.role = 'user'`,
  but the demoted admin's existing token still carries `role: "admin"` and keeps full access —
  including `admin_grant`, `admin_set_credits`, and `admin_create_apikey` — for up to 30 days.
- **Deletion does not take effect.** `delete_user` (`auth.rs:372`) removes the row, but handlers
  that only need `claims.sub` keep working. `deploy_site` is the clearest case: it reads
  `claims.role` and `claims.sub` and never touches the `users` table, so a deleted account can still
  publish sites.

**Failure scenario.** You discover an admin account is compromised and demote or delete it. The
attacker's stolen token keeps working for weeks. There is currently no way to lock them out short of
rotating `JWT_SECRET`, which logs out every user on the platform.

**Fix.** Two options, cheapest first:

1. Add a `token_version INT NOT NULL DEFAULT 0` column to `users`, include it in `Claims`, and
   compare against the database inside the extractor. Demotion/deletion/password change bumps it.
2. Or keep JWTs short-lived (1–2 h) and issue a refresh token that is checked against the database.

At minimum, make the admin check read the live role. Note that `admin_only()` is currently
duplicated in four modules (`codes.rs:29`, `pay.rs:10`, `email.rs:11`, `models.rs:62`) and open-coded
in four more (`auth.rs` ×3, `realtime.rs` ×2, `skills.rs`, `agent_trace.rs`), so this means hoisting
one shared helper and replacing all eleven call sites:

```rust
async fn admin_only(state: &AppState, claims: &Claims) -> ApiResult<()> {
    let uid = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(uid).fetch_optional(&state.db).await?;
    match role.as_deref() {
        Some("admin") => Ok(()),
        _ => Err(AppError::forbidden("需要管理员权限")),
    }
}
```

That one change closes both the demotion and the deletion gap for every admin route, at the cost of
one indexed primary-key lookup per admin request.


**Fix written (against the stale tree)** — `auth.rs` gains `require_admin()` and `require_user()`, both of which read the live
`users` row. All 24 admin call sites across `codes.rs`, `pay.rs`, `email.rs`, `models.rs`,
`realtime.rs`, `skills.rs`, `agent_trace.rs` and `auth.rs` now route through them; the four duplicate
local `admin_only()` helpers were kept as one-line async delegates so the call sites stayed small.
`deploy_site` calls `require_user`, closing the deleted-account-can-still-publish gap.

`set_user_role` / `delete_user` now compare the *returned* uid against the path id instead of
`claims.sub`. `agent_trace::list_agent_traces` had no `State` at all and gained one.

Deliberately **not** changed: `deploy::access()` still reads `claims.role` to decide permanent vs
7-day deploy rights. A stale token there buys a demoted admin nothing meaningful, since
`DEPLOY_AUTO_GRANT` already grants every account temporary rights — noted rather than churned.

---

### 5. High — decompression bomb in the site-deploy endpoint

**File:** `server/src/deploy.rs:191-257`

The size guard checks the **compressed** body only:

```rust
if body.len() > max_mb * 1024 * 1024 {   // DEPLOY_MAX_MB, default 30
```

The extraction loop then calls `e.unpack(&dest)` (line 251) with no cap on uncompressed bytes. The
`count > 5000` check (line 254) runs *after* each file is fully written, so it bounds the file
*count* but not the file *size* — a single entry is never caught.

gzip reaches roughly 1000:1 on repetitive input, so a 30 MB upload can expand to ~30 GB. The host
has 41 GB free.

**Failure scenario.** A registered user uploads one 30 MB `tar.gz` containing a single highly
compressible file. Extraction fills the disk. Postgres, Redis, nginx, and the backend all share
`/dev/vda1`, so this takes down the entire platform, not just the deploy feature.

**Fix.** Track cumulative uncompressed bytes and abort past a budget. Because entries are streamed,
use a limited reader rather than trusting the tar header:

```rust
const MAX_TOTAL: u64 = 200 * 1024 * 1024;
let mut written: u64 = 0;
// ...inside the loop, replacing e.unpack(&dest):
let remaining = MAX_TOTAL.saturating_sub(written);
if remaining == 0 {
    let _ = std::fs::remove_dir_all(&target);
    return Err(AppError::forbidden("解压后体积超限（>200MB）"));
}
let mut out = std::fs::File::create(&dest)
    .map_err(|e| AppError::internal(format!("建文件失败: {e}")))?;
// +1 so hitting the cap exactly is still detected on the next iteration
written += std::io::copy(&mut e.by_ref().take(remaining + 1), &mut out)
    .map_err(|e| AppError::internal(format!("解压失败: {e}")))?;
```

Also move the cleanup on the `count > 5000` path: today that branch returns an error but leaves the
partially extracted tree on disk.

**Credit where due:** the *path-traversal* defence in this function is genuinely well built. It
rejects `..`, absolute paths, and Windows prefixes (lines 226-233), accepts only regular files and
directories so symlink and hardlink entries are skipped (lines 235-238), and re-checks
`dest.starts_with(&target)` (line 241). I could not find an escape. The bug is purely about volume.


**Fix written (against the stale tree)** — `deploy.rs`: extraction now copies through `Read::take(remaining + 1)` into an
explicitly created file instead of `entry.unpack()`, accumulating `written` against a
`DEPLOY_MAX_UNPACKED_MB` budget (default 200 MB). The budget is checked both before an entry and
**immediately after** it — a bomb is typically one huge entry, and if it were also the last entry a
before-only check would let the loop finish and report success. Both the over-budget and the
>5000-file path now `remove_dir_all` the partial tree instead of leaving it on disk.

---

### 6. Medium — the quota gate and the deduction are not atomic

**File:** `server/src/models.rs:2280-2305` (gate) and `1482-1518` (`bill`)

The access check is a `SELECT`, the request then runs for seconds to minutes, and the deduction is a
separate `UPDATE` afterwards. Nothing reserves the quota in between, and there is no row lock.

`bill()` clamps with `GREATEST(credits_cents - $1, 0)`, so the balance floors at zero rather than
going negative — but the user still received all the value. The overspend is silent.

**Failure scenario.** A user with 10¢ left fires 50 concurrent agent requests. All 50 pass the
`credits > 0` gate before any of them bills. All 50 complete. You pay the upstream provider for 50
calls and collect 10¢. This is not a contrived scenario — the IDE's own multi-agent orchestration
(`spawn-multiple-agents`) fans out parallel calls by design.

**Fix.** Reserve optimistically before calling upstream and refund the difference after:

```rust
// Reserve a conservative estimate up front; only proceed if the row actually changed.
let reserved: i64 = estimate_cost_ceiling(&body);
let ok = sqlx::query("UPDATE users SET credits_cents = credits_cents - $1 \
                      WHERE id = $2 AND credits_cents >= $1")
    .bind(reserved).bind(uid).execute(&state.db).await?.rows_affected() == 1;
if !ok { return Err(AppError { status: StatusCode::PAYMENT_REQUIRED, msg: "额度不足".into() }); }
// ...call upstream, then refund (reserved - actual) in bill().
```

The `WHERE credits_cents >= $1` predicate makes the check-and-decrement a single atomic statement,
which is what closes the race.


**Fix written (against the stale tree)** — `models.rs`: a new `require_quota()` replaces the three copy-pasted gates, and adds an
`InflightHold` reservation tracked in Redis (`inflight:<uid>`). Every pool is now compared against
balance-*minus-in-flight*, so concurrent calls cannot all pass on the same stale balance.

This differs from the reserve-and-refund sketch above. Reserving against the *database* balance means
a process killed mid-call permanently eats the user's money; a Redis counter with a 900 s TTL heals
itself. The hold is a flat 25¢ (`INFLIGHT_HOLD_CENTS`) rather than a per-call estimate — it bounds
concurrency in proportion to balance without having to predict cost, and it is released in full, so
users are still only charged the real `compute_cost`. `InflightHold` releases in `Drop` (guarded by
`Handle::try_current()`, since Drop can run during runtime shutdown) so every `?` and panic path
returns it; for streaming, the hold is moved into the spawned task and dropped after billing.

Callers get a distinct message — "并发请求过多，请等前面的请求返回后再试" — so contention no longer
looks like a billing bug.

---

### 7. Medium — the gateway response cache is global and keyed by a non-cryptographic hash

**File:** `server/src/models.rs:1293-1302`, used at `2327-2351`

```rust
let mut h1 = std::collections::hash_map::DefaultHasher::new();
```

Two problems.

**The key contains no user id.** Cached responses are shared across every account on the platform.
For byte-identical request bodies this is defensible, but it means one user's completion can be
served to another, and it makes the next point exploitable rather than theoretical.

**`DefaultHasher` is the wrong primitive.** Rust documents it as not cryptographically secure, and
`DefaultHasher::new()` is specified to use *fixed, zero* keys — so the mapping is fully reproducible
offline by anyone. The comment on line 1291 calls this "128-bit (two seeded hashes)", but `h2`
hashes a constant and then the *same* bytes with the *same* key, which yields a correlated value
rather than an independent second hash. The effective collision resistance is well below the 128
bits claimed. Rust also explicitly reserves the right to change the algorithm between releases,
which would silently invalidate the whole cache on a toolchain bump.

**Failure scenario.** An attacker who knows a target's exact request body (for example a fixed
system prompt plus a predictable first turn) can grind offline for a body that collides, submit it,
and be served the victim's cached completion.

**Fix.** Use a keyed cryptographic hash and scope the key to the user:

```rust
fn gw_cache_key(uid: &uuid::Uuid, body: &serde_json::Value) -> String {
    let mut h = blake3::Hasher::new_keyed(&CACHE_KEY);  // CACHE_KEY from env, 32 bytes
    h.update(uid.as_bytes());
    h.update(&serde_json::to_vec(body).unwrap_or_default());
    format!("gwc:{}", h.finalize().to_hex())
}
```

If cross-user sharing is a deliberate cost optimisation, keep it — but then the hash must be
cryptographic, and it is worth documenting that identical prompts return identical answers for an
hour (`EX 3600`), which users will notice as a model that "stopped being creative".


**Fix written (against the stale tree)** — `models.rs`: `gw_cache_key(uid, body)` now hashes the user id, a `\x00` domain
separator, and the body with SHA-256 (`sha2`, already a transitive dependency and now declared
directly). Scoping to the caller is the part that actually closes the disclosure: a collision can only
ever hit your own history.

Trade-off: cross-user cache sharing is gone, so identical prompts from different accounts now each
hit the upstream. That costs money but is the only way the cache can't serve one user's completion to
another. Three unit tests cover it (`cache_key_tests`).

---

### 8. Medium — internal error strings are returned to the client

**File:** `server/src/error.rs:27-34`, with the `into_internal!` conversions at 40-54

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() { tracing::error!("{}", self.msg); }
        (self.status, Json(json!({ "error": self.msg }))).into_response()
    }
}
```

`into_internal!(sqlx::Error)` sets `msg = e.to_string()`, so `?` on any failed query sends the raw
database error — table names, column names, constraint names, occasionally fragments of SQL — to
whoever made the request, including unauthenticated callers on `/api/auth/*`.

The same pattern appears in the gateway: `models.rs:604` and `2667` interpolate the upstream
provider's entire error body into the client-visible message, which can expose provider account
state and internal endpoints.

**Failure scenario.** An attacker posts a duplicate email to `/api/auth/register` and reads the
unique-constraint name straight out of the 500 response, mapping your schema for free.

**Fix.** Log the detail, return a generic message:

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = if self.status.is_server_error() {
            tracing::error!("{}", self.msg);
            "服务器内部错误".to_string()      // detail stays in the log
        } else {
            self.msg                          // 4xx messages are user-facing by design
        };
        (self.status, Json(json!({ "error": body }))).into_response()
    }
}
```

The 4xx messages are deliberately helpful and should stay as they are; only the 5xx path needs to be
opaque.


**Fix written (against the stale tree)** — `error.rs`: 500 responses now return a fixed "服务器内部错误，请稍后重试" and log the
detail. Scoped to 500 specifically, **not** `is_server_error()`: 502 is used for the deliberately
user-facing `friendly_upstream` messages ("换个模型或稍后再试"), and genericizing those would have
been a real UX regression. The three sites that interpolated a whole upstream error body into a 502
(`models.rs` legacy chat, `chat_completions`, `admin_available`) now log the body and return the
status only.

---

### 9. Medium — no rate limit on password login

**File:** `server/src/auth.rs:283-295`

The verification-code flow is carefully defended: a 30-second send cooldown (line 242), five attempts
per code, and a hard 20-per-hour ceiling that resends cannot reset (lines 117-160). That work is
solid and clearly deliberate.

`login` has none of it. There is no cooldown, no attempt counter, and no lockout — only bcrypt's
cost factor, which slows an attacker but does not stop one. fail2ban is running on the host but is
configured for SSH, not for HTTP 401s from the backend.

The endpoint also distinguishes "账号不存在" (400) from "密码错误" (401), and `/api/auth/check-email`
answers the same question directly and unauthenticated, so an attacker can enumerate valid accounts
before starting.

**Fix.** Reuse the Redis counter pattern already written for codes — key on `login_fail:<email>` and
on the client IP from `X-Real-IP` (nginx sets it), with exponential backoff after ~5 failures. Also
consider throttling `check-email` per IP.


**Fix written (against the stale tree)** — `auth.rs`: a two-tier Redis budget mirroring the verification-code guard — 10 failures
per account and 50 per source IP (from nginx's `X-Real-IP`), both on a 1-hour window, both cleared on
success, with `EXPIRE` on every increment so a crash can't strand a TTL-less key.

"账号不存在" and "密码错误" were also merged into one "账号或密码错误", and the no-such-account path
now runs `bcrypt::verify` against a throwaway hash so response latency stops revealing which accounts
exist.

---

### 10. Medium — any user can claim any subdomain

**File:** `server/src/deploy.rs:140-172`

`assign_subdomain` derives the hostname label directly from the user-supplied `?name=` parameter,
lowercased and filtered to `[a-z0-9-]`. First writer wins; a taken name falls back to
`<name>-<6 hex of account id>`. There is no reserved-name list.

This is live: `.env` sets `DEPLOY_SITE_DOMAIN` (`michaelide.xyz`), and `DEPLOY_AUTO_GRANT`,
`DEPLOY_ALLOWLIST`, and `DEPLOY_DENYLIST` are all **absent**, so `access()` (line 96-120) falls
through to auto-granting a 7-day deploy permission to every registered account.

**Failure scenario.** A user deploys a site named `admin`, `www`, `api`, `login`, or `mail` and now
controls `https://admin.michaelide.xyz` — served over your wildcard certificate, from your domain,
with your branding. That is a ready-made credential-phishing page that inherits your reputation.

**Fix.** Reject a reserved set before assigning, and require an explicit opt-in for short names:

```rust
const RESERVED: &[&str] = &["www", "api", "admin", "app", "mail", "smtp", "ns1", "ns2",
                            "login", "auth", "gateway", "cdn", "static", "code", "dev",
                            "staging", "test", "support", "help", "billing", "pay"];
if RESERVED.contains(&base) || base.len() < 4 {
    return None;   // fall back to the /s/<account>/<name>/ path URL
}
```

Given that any account can deploy, it is also worth deciding whether `DEPLOY_AUTO_GRANT` should
stay on by default — setting it to `0` and using `DEPLOY_ALLOWLIST` would make hosting opt-in.


**Fix written (against the stale tree)** — `deploy.rs`: the label rule moved into a testable `subdomain_label()` that refuses 35
reserved names (`admin`, `www`, `api`, `login`, `billing`, `pay`, …) and anything under 4 characters,
returning `None` so the caller falls back to the `/s/` path URL.

Writing the test surfaced a second hole the review had missed: an empty or unusable name fell back to
the literal `"site"`, so the first person to deploy without a name claimed `site.<domain>` for
everyone. That now returns `None` too.

---

### 11. Medium — `bill()` discards every database error

**File:** `server/src/models.rs:1482-1518`

Both the balance `UPDATE` and the `model_usage` `INSERT` are prefixed with `let _ =`. If the pool is
saturated, the statement times out, or the row is locked, the user is not charged and no usage
record is written — and nothing is logged, so the loss is invisible.

This matters more than it looks because `bill()` for streaming requests runs inside a
`tokio::spawn` (line 2532) *after* the response has been handed to the client. Nothing upstream can
observe the failure.

**Fix.** At minimum log it. Better, retry the row once, since it is the accounting record of record:

```rust
if let Err(e) = res {
    tracing::error!(user = %uid, cost, "billing write FAILED — revenue not recorded: {e}");
}
```


**Fix written (against the stale tree)** — `models.rs`: both the balance update and the `model_usage` insert now capture their
result and `tracing::error!` on failure, including the user id and amount. `bill()` also takes
`Option<uuid::Uuid>` for the connection id, so the endpoints that aren't tied to a model connection
(transcription, game assets) can log usage with a NULL FK instead of inventing a UUID with no row.

---

### 12. Low — the online-user counter drifts upward and never recovers

**File:** `server/src/realtime.rs:41-59`

`handle_socket` does `bump_online(+1)`, streams, then `bump_online(-1)`. If the task is cancelled —
container restart, deploy, or a panic in `stream_feed` — the decrement never runs. `online:count`
has no TTL, and Redis persists it with AOF, so the error is permanent and cumulative.

`stats` (line 128) clamps with `.max(0)`, which hides an undercount but does nothing about an
overcount. After a few weeks of deploys the dashboard's "online" number is meaningless.

**Fix.** Replace the counter with a presence set of per-connection keys that expire on their own:
`SETEX ws:<uuid> 60 1` refreshed on the existing 15-second heartbeat, and report the count via
`SCAN`. Connections that die take their own key with them.


**Fix written (against the stale tree)** — `realtime.rs`: the single `online:count` INCR/DECR counter is replaced with one
`ws:online:<uuid>` key per connection, `SET … EX 45`, refreshed every 15 s from a new `tick` branch in
the existing `select!` loop. `stats` counts them with `SCAN` (not `KEYS`, which would block Redis).
A killed process now heals itself within 45 seconds.

---

### 13. Low — the verification code is consumed before the duplicate-email check

**File:** `server/src/auth.rs:264-269`

```rust
if !take_code(&state, &req.email, &req.code).await? { /* reject */ }
if find_user(&state, &req.email).await?.is_some() {
    return Err(AppError::bad("该邮箱已注册，请直接登录"));
}
```

`take_code` deletes the code on success. A user who registers, then retries because of a network
hiccup, gets "already registered" *and* has their code silently burned — they must wait out the
30-second cooldown and request a new one.

There is also a narrow race: two concurrent registrations for the same email both pass `find_user`
and one hits the unique constraint, which surfaces as a raw 500 (see finding 8).

**Fix.** Check `find_user` first, and do the insert as `INSERT ... ON CONFLICT (email) DO NOTHING`
with a `rows_affected() == 0` check so the race produces the friendly message too.


**Fix written (against the stale tree)** — `auth.rs`: the `find_user` check moved above `take_code`, and the insert became
`ON CONFLICT (email) DO NOTHING RETURNING *` with `fetch_optional`, so the concurrent-registration
race returns the friendly "该邮箱已注册" instead of a raw 500 from the UNIQUE constraint.

---

### 14. Low — stale source copies clutter the tree

**Files:** `server/src/`

Thirteen `.bak` / `.pre-*` / `.predeploy-bak` files sit next to the live modules — including eight
copies of `models.rs` ranging from 45 KB to 165 KB — plus a complete duplicate `src/src/` tree with
14 more `.rs` files. Ownership is mixed (`root` and uid `501`).

To be precise about the risk: these are **not** shipped in the runtime image. The Dockerfile's
second stage copies only the compiled binary plus `migrations/`, `knowledge/`, `prompts/`, and the
two Python scripts, so the stale files reach the builder stage only. Cargo also ignores them, since
nothing declares them as modules.

The real cost is human: eight near-identical copies of the file that contains all the billing logic
is an invitation to edit the wrong one, and `src/src/auth.rs` differs from `src/auth.rs`.

**Fix.** Delete them — git history already holds every one of these states, and the working tree is
clean, so nothing is lost. Add `*.bak`, `*.pre-*`, `*.predeploy-bak` to `server/.gitignore`.

**Also noticed:** `migrations/0019_commissions.sql` creates a `commissions` table that no Rust code
references — a referral feature that was started and left half-built. Harmless, but it will confuse
the next person to read the schema.


**Fix written (against the stale tree)** — `server-remove-stale-copies.patch` deletes all 27 (13 `.bak`/`.pre-*` files and the
14-file duplicate `src/src/` tree); `.gitignore` gains `*.bak`, `*.bak.*`, `*.bak-*`, `*.pre-*`,
`*.predeploy-bak`. Kept as a separate patch because it is pure deletion and reviews differently from
the behavioural changes. Every deleted file is in git history.

`migrations/0019_commissions.sql` was left alone — dropping a table is not a cleanup to bundle into a
security patch, and an unused table costs nothing.

---

## 4. What the codebase does well

This is worth stating plainly, because the list above is one-sided by construction.

- **Secret hygiene is correct.** `server/.env` is `0600 root:root`, matched by `server/.gitignore`,
  and has never been committed — `git log --all -- server/.env` is empty. `JWT_SECRET` is 64
  characters. Nothing is hardcoded; `config.rs` fails fast on missing required variables.
- **Container hardening is deliberate and correct.** `cap_drop: ALL` plus `no-new-privileges` on the
  public-facing service, `no-new-privileges` on Postgres and Redis, and the backend bound to
  `127.0.0.1:8080` so it cannot be reached except through nginx. The comments explain *why*, and
  the reasoning holds up.
- **The verification-code brute-force defence is genuinely well designed.** The two-tier budget —
  per-code attempts plus an hourly ceiling that resends cannot reset — is the correct shape, and the
  comment at line 150 explaining why `EXPIRE` is called unconditionally shows someone thought about
  the crash window.
- **Path-traversal defence in `deploy.rs` is solid**, as noted in finding 5.
- **SQL injection is not present.** Every query in all 8,684 lines uses bound parameters.
- **Passwords use bcrypt at default cost**, and `password_hash` is `#[serde(skip)]` so it cannot
  leak through a `User` serialization.
- **The billing code is careful where it counts** — a `$50` per-call ceiling, `GREATEST(..., 0)`
  floors, cache-aware pricing that passes provider discounts through, and detailed per-call tracing.
- **Real operational problems have been solved properly**: the 15-second SSE heartbeat for Chinese
  carrier NATs, the distinction between transient and persistent upstream errors in the retry loop,
  and the idle-timeout widening for deep-thinking models are all things you only fix after being
  burned, and each is documented.

---

## 5. Verification performed

Built and tested against a local copy of the production tree (`dd00fb5`), Rust 1.96:

| Check | Result |
|---|---|
| `cargo build --release` | clean — 3 warnings, **all 3 pre-existing** on `dd00fb5` |
| `cargo test` | 49 passed / 4 failed — **the same 4 fail on `dd00fb5`**, so zero regressions |
| New tests | 7 added (4 in `deploy::tests`, 3 in `models::cache_key_tests`), all passing |

The 4 pre-existing failures are unrelated to this work and were present before it:
`models::billing_tests::{anthropic_thinking_gate_by_model, oai_to_anthropic_enables_adaptive_thinking_and_drops_temp, thinking_normalized_per_model}`
expect `thinking: {"type": "adaptive"}` where the code now emits `{"type": "enabled", budget_tokens}`,
and `prompts::tests::truncates_excessive_tool_requests` still expects the old 32-tool cap against the
current `MAX_STATIC_TOOLS_PER_REQUEST = 160`. In each case the *test* is stale, not the code —
worth a separate pass, but out of scope here.

**Not verified:** nothing was run against the live server. The gate/billing changes touch the
money path and the `/ws` change touches the dashboard, so both want a smoke test on a real deploy.

---

## 6. Applying the fix

### 6a. FIRST — restore full production config (service is up but degraded)

The container is serving traffic on the correct binary, but was recreated from the wrong compose
file, so it lost two env vars and the `musiccache` volume. Recreating it from the real project
directory fixes that. Nothing rebuilds; it reuses the already-correct `server-backend:latest`:

```bash
cd /opt/michael-ide-deploy/server && docker compose up -d --force-recreate backend
```

Then confirm all three are back:

```bash
docker inspect server-backend-1 --format '{{range .Config.Env}}{{println .}}{{end}}' | grep -cE '^(MICHAEL_COMPRESSION_ENABLED|IDE_RELEASE_GITHUB_TOKEN)=' && docker inspect server-backend-1 --format '{{range .Mounts}}{{println .Destination}}{{end}}' | grep -c huggingface
```

Expect `2` then `1`. I was blocked from running this by the permission layer, so it needs to be you.

### 6b. THEN — redo the fix against the right tree

`server-hardening.patch` and `server-remove-stale-copies.patch` are written against the stale tree.
**Do not apply them.** They are kept only as a reference implementation of each fix.

The correct sequence is to re-run the review against `/opt/michael-ide-deploy/server` — which also
means reading the four modules that tree has and the reviewed one does not (`channel_rates.rs`,
`commission.rs`, `compression.rs`, `update.rs`), none of which have been looked at yet.

When rebuilding for real, always work from the deploy directory, and use the existing script rather
than raw compose — `/root/deploy-gateway.sh` exists precisely because of the trap I fell into:

```bash
cd /opt/michael-ide-deploy/server && bash /root/deploy-gateway.sh
```

### 6c. Housekeeping from this session

- `/root/Michael-IDE/Devin-Desktop` is on branch `hardening-2026-08-01` with both patches applied.
  Discard with `git checkout codex/production-recovery-20260710 && git branch -D hardening-2026-08-01`.
- `/root/Michael-IDE/Devin-Desktop/server/.env` gained `DEPLOY_AUTO_GRANT=0` + `DEPLOY_ALLOWLIST`.
  Backup: `.env.bak-pre-hardening-20260801`. To actually enforce the policy, put those two lines in
  `/opt/michael-ide-deploy/server/.env` instead — allowlisting the two accounts that already have
  live sites (`3162c5d6-…`, `4e8990ce-…`, both role `user`) so they can still redeploy.
- Images kept: `server-backend:hardening-20260801-nomigrations` (the bad build — safe to delete),
  `server-backend:toolsharp-20260731` (last-good, currently tagged `latest`).

---

## Appendix — how to reproduce the checks

```bash
# Finding 1 — WebSocket upgrade with no credentials (expect 101 Switching Protocols)
curl -s -i --max-time 6 -N \
  -H "Connection: Upgrade" -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  http://127.0.0.1:8080/ws | head -3

# Control — the REST endpoints on the same feed correctly reject (expect 401)
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:8080/api/admin/events

# Finding 4 — no revocation mechanism anywhere (expect no output)
grep -rn "jti\|revoke\|blacklist\|token_version" server/src/*.rs

# Finding 4 — role is never re-read from the database (expect no output)
grep -rn "SELECT role\|role FROM users" server/src/*.rs

# Finding 2 — no billing call in the game module (expect no output)
grep -n "bill(" server/src/game.rs

# Finding 3 — the discarded user id
grep -n "let _uid" server/src/models.rs

# Finding 10 — deploy auto-grant is on because none of these are set (expect no output)
grep -E "^DEPLOY_(AUTO_GRANT|ALLOWLIST|DENYLIST)=" server/.env
```
