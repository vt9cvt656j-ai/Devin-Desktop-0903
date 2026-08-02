# Michael Gateway — Review Against the Real Production Source

**Reviewed:** 2026-08-01
**Source:** `Devin-Desktop/server/` (local) — verified **byte-identical** to production
`/opt/michael-ide-deploy/server` across all 20 modules (sha-compared per file)
**Size:** 24,022 lines / 20 modules — vs the 8,684 lines / 16 modules in the stale tree

> Supersedes the status column in [SERVER_BACKEND_REVIEW.md](SERVER_BACKEND_REVIEW.md), which
> reviewed `/root/Michael-IDE/Devin-Desktop/server` — a tree production does not build from.
> The detailed *analysis* in that document is still useful; its *status* was wrong.

---

## Headline: 6 of 14 findings were already fixed in production

The production tree is materially more hardened than the stale copy. Several fixes are better
than the ones I wrote, and in two cases the existing code documents the exact attack I described.
My earlier "still present in production" spot-check was done with greps that under-detected —
it missed `require_paid_access` entirely, and it mistook a WebSocket handshake for an
unauthenticated feed. Everything below was re-verified by reading the code.

| # | Finding | Status in production | Evidence |
|---|---|---|---|
| 1 | `/ws` feed unauthenticated | ✅ **Fixed** | `realtime.rs:75-101` |
| 2 | `game.rs` unmetered spend | ✅ **Fixed** | `models.rs:2895` |
| 3 | transcription discards user | ✅ **Fixed** | `models.rs:4099` |
| 4 | role trusted from JWT | ✅ **Fixed** (one residual → **F15**) | `auth.rs:70-86` |
| 5 | decompression bomb | ✅ **Fixed** | `deploy.rs:265-322` |
| 10 | subdomain squatting | ✅ **Fixed** | `deploy.rs:171-190` |
| 11 | `bill()` swallows DB errors | ✅ **Fixed** | `models.rs` `bill()` |
| 6 | quota gate not atomic | ⚠️ **Partly** — charge is now atomic, gate→charge is not | `models.rs:4273`, `3131` |
| 7 | cache key global + `DefaultHasher` | ❌ Present | `models.rs`, `gw_cache_key` |
| 8 | raw `sqlx` errors to client | ❌ Present | `error.rs:39-51` |
| 9 | no login rate limit | ❌ Present | `auth.rs` `login` |
| 12 | `online:count` drifts | ❌ Present | `realtime.rs:50-57` |
| 13 | code burned before dup check | ❌ Present | `auth.rs` `register` |
| 14 | stale `.bak` copies | ❌ Present (5 files) | `src/*.pre-*` |

### How the already-fixed ones were done (worth knowing — several beat my patch)

**#1 `/ws`.** `handle_socket` requires the *first frame* to be
`{"type":"auth","token":"<jwt>"}` with `role == "admin"`, on a 10-second timeout, and closes with
an `auth_error` frame otherwise. The comment explains they deliberately rejected a query
parameter because *"putting the token in the query string would write it into nginx's access
log."* My patch used `?token=` — it would have leaked every admin JWT into
`/var/log/nginx/access.log`. **Good thing it was never applied.**

My earlier "verified live: returns 101" was a bad inference. The upgrade *is* expected to
succeed; authentication happens on the first frame immediately after. I proved the handshake
worked, not that data flowed.

**#2 and #3.** A shared `require_paid_access()` gates auth + active plan/quota-or-credits, then
applies `asset_gen_charge_budget` — a 60-per-hour per-user ceiling. Its doc comment states the
problem almost verbatim: *"`auth_any_user` alone only proves 'some registered account', which let
any free signup burn the operator's third-party balance without limit."*

**#4.** Rather than patching 13 call sites (what I did), the fix is in the `Claims` **extractor
itself** — it re-reads `role` from `users` on every request and overwrites the token's claim,
failing closed if the row is gone. Every admin gate and every deleted account is handled at once.

**#5.** `MAX_UNPACKED_BYTES = 200MB`, accumulated from each entry's declared header size *before*
unpacking, so the bomb is rejected before any bytes land.

**#11.** `bill()` now opens a transaction, takes `SELECT … FOR UPDATE` on the balance row, and
logs every failure path with `tracing::error!`.

---

## F15 — the WebSocket path still trusts the JWT's `role`  ✅ FIXED

**Severity:** Medium · **Fixed in** `server-f15-commission.patch` · **Files:** `auth.rs:118-127` (`claims_from_jwt`), `realtime.rs:89`

Finding #4 was fixed inside the `Claims` extractor. The WebSocket feed cannot use that extractor —
there is no request to extract from once the socket is upgraded, so it calls `claims_from_jwt`,
which **only decodes the token**:

```rust
pub fn claims_from_jwt(cfg: &Config, token: &str) -> Option<Claims> {
    decode::<Claims>(token, &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
                     &Validation::default()).ok().map(|data| data.claims)
}
```

and `ws_authenticate` decides on that value:

```rust
crate::auth::claims_from_jwt(&state.cfg, token).is_some_and(|c| c.role == "admin")
```

So `/ws` is the one surviving place where the 30-day token is trusted for **privilege**, not just
identity — exactly what the extractor's own comment says must never happen. A demoted or deleted
admin keeps the live feed (every user's email, order amounts, grants, commissions) until their
token expires.

**Failure scenario.** You demote a compromised admin. Every REST endpoint locks them out
immediately. Their WebSocket keeps streaming your entire user and revenue feed for up to 30 days,
and nothing in the logs distinguishes it from the real dashboard.

**Fix applied.** `ws_authenticate` now returns `Option<uuid::Uuid>` instead of a bool: it
decodes the frame as before, then resolves the role from `users` via a new `is_admin_now()`
helper, returning the id only for a current admin. `stream_feed` takes that id and re-runs
`is_admin_now` every 5 minutes (`WS_ROLE_RECHECK`) on a `tokio::time::interval` branch inside
the existing `select!`, closing the socket with an `auth_error` frame the moment the role
changes — otherwise a socket opened while the user *was* an admin would survive the demotion
for as long as it stayed connected, which on this feed can be days.

---

## Money modules — reviewed (was the main coverage gap)

### `channel_rates.rs` — clean, nothing to report

All four handlers admin-only; rate validated `is_finite() && > 0 && <= 1e6` (so NaN and
Infinity are rejected, not just negatives); name/note length-bounded by *characters* not
bytes; the unique-violation SQLSTATE `23505` is mapped to a friendly message instead of a
500; every query parameterised; two unit tests.

I initially flagged `f64` for an exchange rate as a money-math risk and was wrong twice
over. First, it is not display-only — `models.rs:1041` feeds it into
`project_quota_package` for real pricing decisions. Second, that function's only caller
validates every input before the call (`sales_cny > 0`, `target_margin_percent` in
`[0, 100)` — note the **exclusive** upper bound, `multiplier > 0`), so none of its six
divisions can hit zero and none can produce NaN or Infinity. No finding.

### `commission.rs` — two real issues, both fixed

Otherwise good: basis-point arithmetic through an `i128` intermediate so there is no
float drift on money, an explicit `commission_cents <= amount_cents` ceiling, bounded
`source`/`note`, and unit tests covering the rounding.

**C1 — settlement timestamps were destroyed on re-settle** (Medium)

```sql
settled_at = CASE WHEN $2 = 'settled' THEN now() ELSE NULL END
```

Re-submitting `status: "settled"` on an already-settled row — which is what happens when
an admin edits the note — silently overwrote `settled_at` with the current time. On a
payout ledger that is the loss of an audit fact: after one note edit you can no longer say
when a commission was actually settled. Fixed with `COALESCE(settled_at, now())`, so the
first settlement time sticks and only a move *away* from settled clears it.

**C2 — nothing stopped two commissions for the same order** (Medium)

`0019_commissions.sql` creates `idx_commissions_order` as a **plain** index, not unique, and
`admin_create_commission` never checked. A double-submitted form, or two admins working the
same backlog, produced two payable rows for one order and paid the referrer twice. Fixed
with a pre-insert check that ignores `rejected` rows, so a mistake can still be voided and
re-entered.

**Deliberately not a migration.** A partial unique index is the stronger fix and the table
is currently empty (0 rows), so it would apply safely — but see the trap below. The
application check is race-prone in theory; in practice this is admin-only manual data entry,
so two truly-simultaneous inserts are not a realistic path.

> ### ⚠️ Migration-ordering trap — read before adding any migration
>
> The database has already applied `20260728_fix_quota_system_and_add_payment_tables`.
> sqlx orders migrations by the integer parsed from the filename, and **rejects
> out-of-order application**. So a new `0024_*.sql` (version `24 < 20260728`) would be seen
> as an unapplied migration sorting *before* an applied one, and the backend would refuse to
> start — the exact `migration 20 was previously applied but is missing…` crash-loop that
> took the service down earlier today, in a different form.
>
> Any new migration must sort after the largest applied version, i.e. use a date prefix:
> `20260801_commission_order_unique.sql`.

---

## Also found: production is running uncommitted code

`git status` in `Devin-Desktop` (branch `devin/initial-implementation`) shows
`server/src/codes.rs` and `server/src/prompts.rs` modified in the working tree — before I
touched anything. Since the local tree is byte-identical to `/opt`, production is running
those uncommitted edits. They are not in git on any branch, so there is nothing to roll back
to and nothing recording what changed or why. Worth committing.

Related: `prompts::tests::ordinary_agent_assembly_stays_within_a_compact_attention_budget`
fails on the current tree. I confirmed it fails with my changes stashed, so it is
pre-existing — quite possibly tied to those uncommitted `prompts.rs` edits.

---

## Verification of this patch

| Check | Result |
|---|---|
| `cargo check` | clean — verified genuine by injecting a deliberate type error and confirming cargo caught it at `commission.rs:329` |
| `cargo test` | 194 passed / 1 failed — the failure is **pre-existing** (reproduced with my changes stashed) |
| `admin.html` | script extracted and `node --check`ed — syntax OK |
| Patch apply | `git apply --check` against a **fresh rsync of `/opt`** — applies cleanly |

`static/admin.html` is compiled into the binary with `include_str!`, so the dashboard change
ships with the same rebuild — no separate asset deploy.

**Dashboard behaviour change.** The client already spoke the first-frame auth protocol, so no
protocol change was needed. But once the server can now revoke a live socket, the existing
`onclose` handler would have reconnected every 3 seconds forever, toasting each time. The
patch adds a `wsAuthDenied` flag so an auth rejection stops the auto-reconnect, while an
explicit `connectWs()` (login or page load) clears it and retries.

---

## Deploying this patch

```bash
cd /opt/michael-ide-deploy/server && git apply /root/server-f15-commission.patch && bash /root/deploy-gateway.sh
```

Use `deploy-gateway.sh`, never a bare `docker compose` from another directory — both
`/root/Michael-IDE/Devin-Desktop/server` and `/opt/michael-ide-deploy/server` are named
`server`, so Compose resolves them to the same project and will deploy whichever one you
happen to be standing in.

Smoke test after restart: open the dashboard and confirm the live event tail still arrives
(exercises the first-frame auth), then demote a test account and confirm its socket drops
within 5 minutes.

---

## Still-open findings, re-confirmed against production

**#6 quota gate not atomic** — `bill()` now locks the row `FOR UPDATE`, so concurrent charges can
no longer lose updates or corrupt a balance. But the *gate* (`models.rs:4273`, `5350`, `2900`) is
still a plain `SELECT` compared against the balance, with the charge happening later. N concurrent
requests still all pass on the same reading, so a user with 10¢ still gets N calls' value for 10¢.
Severity is lower than originally stated — this is lost revenue, not data corruption.

**#7 response cache** — `gw_cache_key(body)` is unchanged: still `DefaultHasher` (fixed zero keys,
not collision-resistant, algorithm explicitly unstable across Rust releases) and still **not
scoped to the user**, so one account's cached completion can be served to another. There is a new
`response_cache_safe()` guard that refuses to cache responses whose tool-call arguments contain
tracking numbers — a targeted PII fix that does not address the cross-user or collision issue.

**#8 error leakage** — `error.rs:39-51` still returns `self.msg` verbatim at every status, and
`into_internal!(sqlx::Error)` still fills that with raw DB error text. 502/503/504 now log at
`warn` instead of `error`, but nothing is redacted.

**#9 login** — no cooldown, no attempt counter, no lockout. Still returns `"账号不存在"` (400) vs
`"密码错误"` (401), and skips bcrypt entirely on the unknown-account path, so both the message and
the response time are account-enumeration oracles. The emailed-code path remains well defended by
contrast.

**#12 `online:count`** — still a bare `INCRBY`/`DECRBY` with no TTL, so a crash or restart between
the two leaks the count upward permanently.

**#13 register** — `take_code` still runs *before* the duplicate-email check, so retrying a
registration that already succeeded burns the user's code. `normalize_email` was added since, which
is a genuine improvement to the UNIQUE guarantee, but the ordering is untouched.

---

## Coverage — what this review did and did not cover

Being explicit, because 24,022 lines is more than one pass can responsibly claim.

| Module | Lines | Depth |
|---|---|---|
| `realtime.rs`, `error.rs`, `auth.rs`, `deploy.rs`, `game.rs` | 1,487 | **Read in full** |
| `update.rs` | 1,016 | **Reviewed** — handler auth map + the public `download_asset` proxy |
| `models.rs` | 9,205 | **Targeted** — auth, billing, quota gates, cache key, transcription |
| `prompts.rs` | 6,357 | **Targeted** — keyword matchers only (from the earlier task) |
| `compression.rs` | 1,738 | ⚠️ **Not reviewed** |
| `commission.rs` | 309 | ⚠️ **Not reviewed** |
| `channel_rates.rs` | 181 | ⚠️ **Not reviewed** |
| `knowledge.rs`, `procedural_3d.rs`, others | ~1,700 | ⚠️ **Not reviewed** |

`update.rs` looked well built and I found nothing to report: the public `download_asset` proxy
requires a configured token, whitelists both path segments via `safe_release_path_segment`, refuses
draft releases, and resolves filenames through the release's own asset map rather than accepting a
path — so it is scoped to published assets of the configured repo, not an open GitHub proxy.
`validate_manifest` requires an HTTPS URL and a non-empty signature on every platform entry.

**`commission.rs` and `channel_rates.rs` are the notable gap** — both are money-handling modules
that have never been looked at by anyone in this review, and `0019_commissions.sql` was a migration
I previously flagged as an unused half-built feature. That was wrong too: it is live code in the
real tree.

---

## Suggested order

1. **F15** — WS role check. Small, closes the last hole in an otherwise-complete fix (#4).
2. **#8** — redact 500 bodies. One function, stops leaking schema to anonymous callers.
3. **#9** — login throttle. The Redis pattern already exists in `take_code`; reuse it.
4. **#7** — scope the cache key to the user and use a real hash.
5. **#6** — reserve against the balance before the upstream call.
6. **#12**, **#13**, **#14** — low severity, cheap.
7. **Review `commission.rs`, `channel_rates.rs`, `compression.rs`** before trusting the money path.

No patch is included this time. The last one was written against the wrong tree; this one should
be written against `Devin-Desktop/server/` (which mirrors production), built with
`cd /opt/michael-ide-deploy/server && bash /root/deploy-gateway.sh`, and never with a bare
`docker compose` from another directory — both directories are named `server`, so Compose resolves
them to the same project and will happily deploy the wrong one.

---

# Addendum — 2026-08-01: the production hang

The user reported the IDE freezing and correctly judged it was a bug, not congestion. It was.
Server load at the time was 0.08.

## Root cause

Gateway logs, six-hour window:

```
model=claude-sonnet-5 upstream_status=400 downstream_status=502
error_excerpt={"message":"\"thinking.type.enabled\" is not supported for this model.
 use \"thinking.type.adaptive\" and \"output_config.effort\" to control thinking behavior."}
attempted_sends=1 route_count=1
```

29 of these in six hours, at 21:18:50 / :52 / :54 / :57 / :59 — one every ~2.2 seconds.

Note `attempted_sends=1`. The gateway was **not** the retry loop; it gave up correctly after a
single send. The amplifier was the client. `upstream_failure_status()` mapped every unrecognised
status through a `_ => BAD_GATEWAY` catch-all, so a permanent 400 reached the IDE dressed as a
transient 502 — and `_isRetryableAiGatewayStatus()` in `main.js` is `[500, 502, 503, 504]` with
backoff `[600, 1200]`. The IDE therefore re-sent a request that could never succeed, forever,
and the user saw a spinner.

Three separate defects had to line up:

1. **`anthropic_thinking()` sent a rejected wire shape.** A workaround assumed aggregator
   upstreams silently ignore `{"type":"adaptive"}` (200 OK, zero `thinking_delta`) and applied
   `enabled`+`budget_tokens` to *every* Claude model. But Opus 4.7/4.8/5, Sonnet 5, Fable 5 and
   Mythos 5 **removed** the explicit-budget form and reject it with a hard 400.
2. **A permanent error was classified as retryable**, which handed the retry decision to a client
   that reasonably trusts the status code.
3. **Three unit tests had been rewritten to assert the buggy shape**, so the suite defended the
   defect instead of catching it.

## Fixes (deployed)

| File | Change |
|---|---|
| `models.rs` `anthropic_thinking()` | 4.6 family keeps `enabled`+budget; everything newer returns `{"type":"adaptive"}`. 3.7 and 3.5/haiku paths unchanged. |
| `models.rs` `upstream_failure_status()` | `400 → 400`, `413 → 413`, `422 → 422` instead of falling through to 502. Access/billing text still maps to 424, so the IDE's "switch account / top up" path is intact. |
| `models.rs` chat route loop | A 400 naming the request as the problem breaks `'routes` instead of replaying an identical body against every remaining candidate. |
| `models.rs` `request_is_deep_thinking()` | Extracted and **fixed**: see below. |
| `main.js` `_browserAiStreamTimeouts()` | Same omission, client side: `adaptive` now counts as a thinking request. |

### The fix that nearly broke something else

`deep_thinking` was computed from `thinking.budget_tokens > 0`. Switching to `adaptive` —
which carries **no** budget field — would have silently dropped every thinking request from the
deep transport budget (10s headers / 600s idle) to the standard one (7s / 180s). That is an
invisible downgrade: nothing errors, the deadline is just wrong, and it surfaces later as a 504
under load. It would have traded a 400-hang for a 504-hang.

The predicate is now a named function that recognises all three wire shapes, with a regression
test asserting both directions (thinking counts as deep; a plain chat does not).

## Verification

Probed the live `claude-sonnet-5` route (`zyz.qingyanzhiying.top`, protocol `anthropic`) with
both shapes, using the stored credential server-side:

```
adaptive        => HTTP 200 | thinking_delta=24 | text_delta=2
enabled+budget  => HTTP 200 | thinking_delta=13 | text_delta=2
```

Two things follow. First, the original workaround's premise is **obsolete** — this aggregator
now honours `adaptive`, and returns *more* thinking with it, not less. Second, `enabled` did
**not** reproduce the 400 on demand: a subsequent 8× run returned 200 every time for both shapes.

So the 400 is **intermittent** — consistent with the aggregator load-balancing across backends
where only some reject the legacy shape. That is precisely why fix #2 matters more than fix #1:
whatever makes a permanent error appear, the client must never be told to retry it.

## Not fixed / follow-ups

- `prompts.rs::ordinary_agent_assembly_stays_within_a_compact_attention_budget` fails at
  **9,275 bytes against its own 9,000-byte ceiling**. Pre-existing, from commit `d01ea1f`,
  unrelated to this outage. Left failing deliberately rather than raising the threshold — the
  guardrail is correct and the prompt should come down to meet it.
- **9,841 failed SSH auth attempts** in the last 24h. `fail2ban` is active and was rate-limiting
  legitimate connections during this work. Worth moving sshd off :22 or going key-only.
- `/opt` still runs code that exists in no git branch (`compression.rs`, `update.rs`). This
  remains the largest standing risk: there is no way to diff or roll back to a known revision.
  Rollback for this deploy is the image tag `server-backend:rollback-20260801` and the source
  copy `/root/models.rs.bak-20260801-thinking`.
