# MSE-1 — Mr.day Sealed Envelope, version 1

Application-layer encryption for every request and response between our own clients
(官网 / 用户后台 / 管理后台 / 桌面端) and the gateway.

This document is normative. `server/src/mse.rs` and `server/web-shared/mse.ts` implement
it; `server/testdata/mse-vectors.json` pins the byte-level agreement between them.

---

## 1. What this actually protects, and what it does not

Write this down first, because the whole design follows from it.

**It removes plaintext from every hop between the browser and the axum process.** Today
TLS terminates at Cloudflare, then again at nginx. Both see every request body, every
response body, every query string in the clear — as does anyone with a corporate root CA
installed on the user's machine, and anything that ever reads an nginx access log or a
Cloudflare trace. After MSE-1 all of those see a fixed-shape blob.

**With key pinning it also stops an active man-in-the-middle**, including one holding a
CA the browser trusts. A pinned client refuses to talk to a gateway whose static key it
does not recognise, and there is no plaintext path to fall back to.

**It does not, and cannot, hide data from the user running the client.** The browser
holds the session key because the browser has to render the data. DevTools shows the
decrypted object; the bundle can be read. Raising that cost is worth doing (see §9) but
"impossible" is not on the menu for code that runs on someone else's computer. Anything
that genuinely must not reach a user must not be *sent* to that user — that is a change
to what the handlers return, not to how they are encrypted.

**It does not hide traffic shape.** Method, path, status code, timing and approximate
size stay visible, deliberately: nginx routes and rate-limits on the path (the per-IP
bcrypt limiter on `/api/auth/` is real protection we are not giving up), and blind
error-rate monitoring is how outages get missed. `MSE_MASK_STATUS=1` collapses the outer
status to 200 for the deployments that would rather trade the observability.

---

## 2. Cipher suite

`MSE1-P384-HKDF-SHA384-AES256GCM` — CNSA 1.0 / NSA Suite B at the TOP SECRET level, and
natively available in both WebCrypto and RustCrypto, so no third-party crypto ships in
the bundle.

| role | algorithm |
| --- | --- |
| key agreement | ECDH, NIST P-384 (secp384r1), 48-byte shared secret |
| KDF | HKDF-SHA-384 |
| AEAD | AES-256-GCM, 96-bit nonce, 128-bit tag |
| hash / key id | SHA-384 |

The construction is HPKE base mode (RFC 9180) in shape: DHKEM(P-384) to a static
receiver key, HKDF for the traffic keys, AES-GCM per message. Nothing here is invented.

---

## 3. Keys

### 3.1 Server static key

A long-term P-384 keypair held by the gateway.

- `MSE_SERVER_KEY` — base64 (standard, padded) PKCS#8 DER of the private key.
- `MSE_SERVER_KEY_PREV` — the key being rotated out. Accepted for sealing/opening,
  never advertised as current. Drop it once no client is pinned to it.
- Unset: the process generates one at boot and logs it, with a warning. Traffic works
  (clients fetch the key), but pinning cannot work and every restart invalidates every
  client session, so treat the warning as a deploy defect.

`kid` = first 24 characters of `base64url(SHA-384(SPKI DER of the public key))`. Deriving
the id from the key means it cannot be mislabelled.

Generate one — the `pkcs8 -topk8` hop is not optional on OpenSSL 3.x, whose
`genpkey -outform DER` emits SEC1 rather than PKCS#8 (the server accepts both, but every
other tool you might reach for assumes PKCS#8):

```bash
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-384 | openssl pkcs8 -topk8 -nocrypt -outform DER | openssl base64 -A
```

### 3.2 Session keys

Never stored, never transmitted, never written to disk.

The client generates an ephemeral P-384 keypair per page load. Both sides compute the
same two traffic keys from ECDH; the server re-derives on demand and keeps the result in
a process-local cache keyed by the ephemeral public key's hash. A restart, a second
instance, or an eviction costs one extra ECDH — never a failure, never a re-handshake.

That cache is hard-capped at 4096 entries (~1 MB) and evicts nearest-to-expiry first.
Sweeping expired entries alone would not bound it: minting a session needs no credentials,
`location /` deliberately carries no rate limit, and every fresh ephemeral key inserts an
entry whose expiry is 30 minutes out — so under a flood nothing is expired and nothing
gets swept. Evicting a live session is harmless; the next request re-derives it.

```
z    = ECDH(server_static_priv, client_eph_pub)              // 48 bytes
prk  = HKDF-Extract(salt = "", ikm = z)                       // SHA-384
tx   = SHA-384(client_eph_spki || server_static_spki)         // transcript binding
k_c2s = HKDF-Expand(prk, "MSE1/v1|c2s|" || kid || "|" || tx, 32)
k_s2c = HKDF-Expand(prk, "MSE1/v1|s2c|" || kid || "|" || tx, 32)
sid   = base64url( SHA-384(client_eph_spki)[0..18] )          // 24 chars, deterministic
```

Separate keys per direction, so a response can never be replayed as a request. `tx` binds
both public keys into the derivation, so a swapped server key yields a different key
rather than a working session.

### 3.3 Forward secrecy — the static key signs, it does not do ECDH

The scheme above, on its own, has no forward secrecy against **server** key compromise: the
client ephemeral public travels in the clear (`X-Mse-Epk`), so an attacker who records
ciphertext and *later* steals the static private key recomputes `z = ECDH(static, client_eph)`
and decrypts everything recorded. That is the classic static-DH weakness.

The fix (TLS 1.3 / Signal shape): the static key stops doing ECDH and instead **signs** a
rotating server **ephemeral** key. ECDH runs client-ephemeral × server-ephemeral. The static
key is now only a *trust anchor* — it authenticates the ephemeral so an active MITM can't
substitute one.

- `/api/crypto/pubkey` carries an `eph` object: `{ id, pub, exp, sig }`, where `sig` is
  `ECDSA-P384-SHA384(static_priv, "MSE-EPH-v1\0" || eph_spki || exp_ms_be)` in raw r‖s (96 B,
  the form WebCrypto verifies). `id` is `kid_of(eph_spki)`.
- The client pins the **static** kid (unchanged), verifies `sig` against it, checks `exp`, and
  then does ECDH with the ephemeral. `X-Mse-Kid` now carries the **ephemeral** id.
- The server rotates the ephemeral every `MSE_EPHEMERAL_TTL_SECS` (default 600 s) and keeps
  each ephemeral private only until `created_at + ephemeral_ttl + session_ttl` — long enough
  that a mid-session client can still re-derive, short enough that older privates are gone.
  `exp` on the wire equals that retention deadline, so a client never uses an ephemeral whose
  private the server has discarded.

Result: stealing the static private key later decrypts **nothing** — it never held a DH secret.
A live-RAM compromise still exposes only the traffic of ephemerals not yet discarded (bounded by
`ephemeral_ttl + session_ttl`, ~40 min) — unavoidable, since a running server must hold current
session keys. This is proven by `forward_secrecy_static_key_alone_cannot_derive_the_session` in
`mse.rs`, and cross-language (Rust signs, WebCrypto verifies) end to end.

**Backward compatibility.** Old clients (desktop builds that can't auto-update) seal to the
**static** kid; the server still serves those via static ECDH — they work, without forward
secrecy. New clients seal to the ephemeral id. The server tells them apart by `X-Mse-Kid`.

**Downgrade floor — `requireFs`.** The `eph` object is an unauthenticated optional field, so an
*active* MITM can **strip it with no key** and force a client onto the static path; that recorded
session then becomes decryptable *if the static key is later stolen* — the very thing FS closes.
A signature over the ephemeral key alone doesn't stop this (the attack deletes the offer, it
doesn't forge one). So the client carries a `requireFs` policy (build flag `VITE_MSE_REQUIRE_FS=1`):
when set, a missing or unverifiable `eph` is a **hard error** (`code: "fs"`), never a silent
static fallback. This converts the downgrade from invisible to a visible connection failure — an
active MITM can still deny service (they always could, by dropping packets) but can no longer
downgrade-then-decrypt.

The SPAs build with `requireFs` on. The two login pages deliberately leave it **off**: they are
the only door in, a stripped `eph` there still keeps the password confidential *now* (it's sealed
to the pinned static key, which the MITM cannot read), and availability outweighs the narrow
"MITM now **and** static-key theft later" residual. `requireFs` needs a gateway that actually
serves `eph` — ours always does, so deploy the server (which offers `eph`) before shipping
`requireFs` frontends.

The client's ephemeral key still gives forward secrecy against compromise of the *client*, and
rotation (§3.1) still bounds the static key's exposure as an authenticator.

---

## 4. Bootstrap

### `GET /api/crypto/pubkey`

Always plaintext, always available, cacheable for 300s.

```json
{
  "v": 1,
  "suite": "MSE1-P384-HKDF-SHA384-AES256GCM",
  "kid": "…",
  "pub": "<base64url SPKI DER>",
  "prev": { "kid": "…", "pub": "…" },
  "mode": "off | optional | required",
  "session_ttl": 1800,
  "max_skew_ms": 120000,
  "server_time": 1755100000000
}
```

`server_time` is not decoration: a client with a wrong clock would otherwise fail every
request forever with a skew rejection. Clients record `server_time - Date.now()` and add
it to every `X-Mse-Ts` they send.

### `POST /api/crypto/handshake`

Optional. Sealing needs no round trip — the client can derive keys from the pubkey alone
and seal its very first request. This endpoint exists to warm the server's derivation
cache and to give a client a clean "am I actually able to talk to this gateway" probe.

Request `{ "v":1, "kid":"…", "epk":"<base64url SPKI>" }` →
response `{ "v":1, "sid":"…", "kid":"…", "expires_in":1800, "server_time":… }`.

No key material crosses the wire in either direction.

---

## 5. Wire format

### 5.1 Sealed body

Binary, `Content-Type: application/mse-sealed`:

```
offset 0      : 0x01                       format version
offset 1..13  : nonce, 12 random bytes      (CSPRNG, fresh per message)
offset 13..   : AES-256-GCM ciphertext || 16-byte tag
```

Random 96-bit nonces with a session lifetime measured in minutes stay far below the
birthday bound; the sequence number in the AAD is what actually orders messages.

### 5.2 Request headers

| header | value |
| --- | --- |
| `X-Mse-V` | `1` |
| `X-Mse-Kid` | server key id the client sealed to |
| `X-Mse-Epk` | base64url SPKI of the client's ephemeral public key |
| `X-Mse-Sid` | session id (§3.2) — must equal the hash of `X-Mse-Epk` |
| `X-Mse-Seq` | decimal u64, strictly increasing from 1 within a session |
| `X-Mse-Ts` | decimal unix milliseconds, skew-corrected |
| `X-Mse-Stream` | `0` to opt out of sealed `text/event-stream`; anything else (or absent) means seal it |

Streams are sealed by default rather than on request. A client that seals its requests is
using our library, and our library reads sealed streams — making it opt-in would mean the
common case silently returns a plaintext stream, which is the failure mode worth designing
out. A hand-rolled client that cannot read sealed frames sends `X-Mse-Stream: 0` and gets
a plaintext stream carrying `X-Mse-Downgrade: stream-opt-out`, so the downgrade is never
silent.

### 5.3 Response headers

`X-Mse-V: 1`, `X-Mse-Sid`, `X-Mse-Seq` (echoing the request's), and either
`Content-Type: application/mse-sealed` or, for a sealed stream,
`Content-Type: text/event-stream` plus `X-Mse-Stream: 1`.

**`Set-Cookie` stays on the outer response.** It is the one header that cannot be sealed: a
cookie is only stored when it arrives on a real network response, and the client reconstructs
the sealed reply with `new Response(...)` in JavaScript, which never touches the cookie jar.
Sealing it would silently discard it — `/api/admin/session` would return 200 while the admin
console never receives its gate cookie, and the login would fail with no visible cause. Nothing
is lost by leaving it outside: `mide_token` already rides every same-origin request as a plain
cookie header because nginx's `auth_request` gate has to read it.

`Access-Control-Expose-Headers` must list every `X-Mse-*` header — the marketing site is
on `mrday.one` and the gateway on `code.mrday.one`, so without it the browser hides them
from the client and every cross-origin call fails to decrypt.

### 5.4 Associated data

NUL-separated so no field can be shifted into another:

```
request : "MSE1/req" \0 sid \0 seq \0 ts \0 METHOD \0 path
response: "MSE1/res" \0 sid \0 seq \0 path
sse     : "MSE1/sse" \0 sid \0 seq \0 frame_index
```

`path` is the URI path with no query string (the query travels sealed, §5.5). Binding
method and path means a captured envelope cannot be replayed onto another route.

The response AAD deliberately omits the status code. The status lives in the sealed
plaintext's `s` field, where the GCM tag already authenticates it, and the client trusts
only that one. Putting it in the AAD as well would contradict `MSE_MASK_STATUS=1`: with
the outer status rewritten to 200 the client cannot know the real one, so it could not
build the AAD it needs to decrypt — the switch would break every response.

### 5.5 Sealed plaintext

Request:

```json
{ "q": "page=2&sort=desc", "b": { "…": "original JSON body" }, "raw": null, "ct": null }
```

- `q` — query string without `?`; `""` when there is none. This is why query strings stop
  appearing in nginx access logs.
- `b` — the original body as JSON, or `null`.
- `raw` + `ct` — base64 of the original bytes and their content type, for bodies that are
  not JSON (`/api/deploy` posts a raw archive). Exactly one of `b` / `raw` is non-null.
- `h` — request headers moved inside the envelope, so they stop appearing on the wire. The
  client seals `authorization` plus the three `x-ide-*` region signals.

**`h` is applied against a strict allowlist** (`SEALABLE_REQUEST_HEADERS` in `mse.rs`).
This is not defence in depth, it is load-bearing: minting a valid envelope needs no
credentials, so without the allowlist any anonymous caller could seal
`{"h":{"x-real-ip":"1.2.3.4"}}` and overwrite what nginx wrote. `auth.rs`'s `client_ip()`
reads `x-real-ip` first, and both the login-failure counter and the verification-code
limiter key off it — a fresh IP per request and neither limit exists any more. Before
adding a name to that list, ask whether any handler makes a security decision on it.

Response:

```json
{ "s": 200, "b": { "…": "handler JSON" }, "raw": null, "ct": "application/json",
  "h": { "x-request-id": "…" } }
```

`h` carries the small allowlist of response headers worth passing through. Everything
else the handler set is dropped, which is the point — headers leak as readily as bodies.

### 5.6 Sealed streams

`text/event-stream` responses are sealed frame by frame when the request carried
`X-Mse-Stream: 1`. Each upstream SSE event becomes one line:

```
data: <base64url(0x01 || nonce || ciphertext||tag)>
```

with `frame_index` counting from 0 in the AAD. The final frame seals the literal
`{"__mse_eos":true}`. A stream that ends without it was truncated in transit, and the
client raises rather than silently returning a short answer — the failure mode a plain
SSE proxy cannot detect at all.

---

## 6. Errors

Always plaintext JSON, so a client whose crypto state is wrong can still read why:

```json
{ "error": "human text", "mse": "rekey" }
```

| `mse` | status | client action |
| --- | --- | --- |
| `rekey` | 409 | refetch `/api/crypto/pubkey`, re-derive, retry once |
| `replay` | 409 | advance `seq` past the server's `seen`, retry once |
| `skew` | 409 | adopt `server_time` offset, retry once |
| `malformed` | 400 | do not retry; report |
| `required` | 400 | plaintext refused on a protected route; seal and retry once |
| `unavailable` | 503 | `MSE_MODE=off` but a sealed request arrived |

At most one retry per class and two per request, or a bad clock becomes a request storm.

---

## 7. Policy: which routes are sealed

`MSE_MODE`:

- `off` — middleware inert. Sealed requests get 503.
- `optional` (default) — sealed accepted, plaintext accepted. A sealed request always
  gets a sealed response. This is the only safe setting during a rollout, and the reason
  it is the default: installed desktop builds cannot auto-update, so flipping straight to
  `required` would lock out every client already in the field.
- `required` — plaintext to a protected route is refused with `mse: "required"`.

Never protected, in any mode — each for a reason that will not change:

| route | why |
| --- | --- |
| `/api/crypto/*` | bootstrap; sealing it would be circular |
| `/health`, `/`, `/api/logo.png` | no payload; probed by things that are not our clients |
| `/api/webhooks/stripe` | Stripe signs and sends it; we do not control the sender |
| `/api/auth/oauth/*/callback`, `/api/integrations/*/callback` | a bare browser redirect arrives from the provider with no header of ours on it |
| `/api/unsubscribe` | reached by clicking a link in an email |
| `/api/authz`, `/api/admin/authz`, `/api/admin/ide-authz` | nginx `auth_request` subrequests; nginx cannot seal |
| `/v1/*`, `/chat/completions`, `/audio/transcriptions`, `/responses` | the OpenAI-compatible contract third-party clients rely on |
| `/ws` | websocket upgrade |
| `/api/ide/update/download/*` | large binary asset served to an updater |

Everything else under `/api/` is protected.

---

## 8. Replay and freshness

- `X-Mse-Ts` outside `±MSE_MAX_SKEW_MS` (default 120 000) → `skew`.
- `SET mse:r:{sid}:{seq} 1 NX PX (2×skew)` in Redis. Key already present → `replay`.
  One small non-secret key per request, expiring inside the acceptance window; nothing
  secret is ever written to Redis or its append-only file.
- Redis unreachable → reject, unless `MSE_REPLAY_FAIL_OPEN=1`. Failing open turns a Redis
  blip into a replay window, so it is off by default.

---

## 9. Raising the reverse-engineering cost

Honest framing: these raise cost, they do not create impossibility (§1).

- **Pin the key.** Build the frontends with `VITE_MSE_PIN=<kid>[,<kid>]`. A pinned client
  refuses an unrecognised gateway key outright — this is the step that turns MSE-1 from
  "opaque to passive intermediaries" into "opaque to an active MITM with a trusted CA".
- **Build with `VITE_MSE_MODE=require`** so the client has no plaintext code path to
  force it down.
- **Ship no source maps** for production bundles.
- **Error messages carry codes, not descriptions.** The client's `MseError` messages are
  short tokens (`e.len`, `e.dec`, …), never prose like "envelope too short" — those would
  redraw the wire format for anyone reading the bundle. All logic branches on `.code`, so
  the messages are free to be opaque. Keep it that way.
- **The two login-page bundles are control-flow obfuscated.** `scripts/sync-mse-client.mjs`
  minifies the standalone browser bundle with esbuild and then runs it through
  `javascript-obfuscator` (control-flow flattening + base64 string-array). This targets
  exactly the small self-contained client a passer-by would screenshot. It is **not** run
  on the three SPAs — obfuscating a 500 KB React bundle triples its size and makes
  production errors unreadable, for no real gain since the SPA's only sensitive part is the
  same crypto client and the protocol is public anyway (§1). `self-defending` and
  `debug-protection` are deliberately **off**: on the login page — the one page that must
  never break — a false trip locks real users out, and control-flow flattening already
  meets the "painful to skim" bar. Set `MSE_NO_OBFUSCATE=1` to skip obfuscation while
  iterating locally. The obfuscator is a **build-time devDependency** (in account-ui) and
  never enters any frontend's runtime dependency tree.
- Whatever must never reach a user: stop returning it. §1.

Honest framing, unchanged from §1: all of this raises the cost of reverse-engineering. None
of it makes the client secret — it runs on the user's machine, the protocol is documented
here on purpose, and the pin is a public fingerprint. Obfuscation stops the casual reader,
not the determined one.

---

## 10. Configuration

| variable | default | meaning |
| --- | --- | --- |
| `MSE_MODE` | `optional` | `off` / `optional` / `required` |
| `MSE_SERVER_KEY` | — | base64 PKCS#8 DER, P-384. Generated at boot if unset (logged, with a warning) |
| `MSE_SERVER_KEY_PREV` | — | previous key, accepted during rotation |
| `MSE_SESSION_TTL_SECS` | `1800` | advertised session lifetime and derivation-cache TTL |
| `MSE_MAX_SKEW_MS` | `120000` | accepted `X-Mse-Ts` deviation |
| `MSE_MAX_SEALED_BYTES` | `67108864` | 64 MiB — see below |
| `MSE_REPLAY_FAIL_OPEN` | `0` | accept when Redis cannot answer |
| `MSE_MASK_STATUS` | `0` | collapse the outer status to 200 (§1) |

`MSE_MAX_SEALED_BYTES` is derived from `/api/deploy`, not picked as a round number. A non-JSON
body travels base64-encoded in `raw`, so that route's 35 MiB archive becomes ~46.7 MiB on the
wire. A 36 MiB cap would have rejected any deploy over ~27 MiB, and the error would have
surfaced somewhere that says nothing about size. JSON bodies are unaffected — they ride as an
object in `b` and are never base64'd. nginx's `client_max_body_size 55m` remains the real outer
gate.

Per-route `DefaultBodyLimit`s still apply as before: the middleware hands the handler the
reconstructed *plaintext* body, so the 12 MiB chat limit and the 35 MiB deploy limit measure
what they always measured.

Frontends: `VITE_MSE_PIN`, `VITE_MSE_MODE` (`auto` / `require`), `VITE_MSE_BASE`.

---

## 10a. The two hand-written login pages

`ide/gate/gate.html` and `server/console-login/login.js` have no build step — they are
copied to the server as-is. They also submit the most sensitive payloads on the whole
surface: passwords, emailed verification codes, and the admin password. They seal too.

Rather than hand-write a second implementation (three copies of crypto, one of which
eventually gets a fix the others miss), `scripts/sync-mse-client.mjs` compiles the same
`web-shared/mse.ts` into a self-contained IIFE exposing `window.MSE`, using the esbuild
that already lives in the frontends' `node_modules`. It then:

- inlines it into `gate.html` between the `MSE-BUNDLE-BEGIN` / `MSE-BUNDLE-END` markers
  (that page's CSP allows inline script), and
- writes `server/console-login/mse.js`, loaded by a `<script src>` before `login.js`
  (that page's CSP is `script-src 'self'`, so it cannot inline).

Both pages fall back to plaintext `fetch` if anything about sealing fails. That is
deliberate: these are the only doors into the system, and locking everyone out is a worse
outcome than one unsealed sign-in. It also means **`MSE_MODE=required` must not be flipped
until both pages are confirmed sealing in production** — under `required` the fallback
request is refused and nobody can log in.

**Deploying `mse.js` is a manual step.** No script references `michael-console-login`;
those files are copied by hand. `mse.js` must land next to `login.js` in
`/var/www/michael-console-login/`, and `nginx/michael-backend.conf` carries a
`location = /console/mse.js` block mirroring the `login.js` one. Ship the two together —
if `mse.js` 404s, the login page silently reverts to submitting the admin password in
plaintext.

## 11. Rollout

1. Deploy with `MSE_MODE=optional` and a persisted `MSE_SERVER_KEY`. Nothing changes for
   any existing client.
2. Publish the three frontends. They seal automatically; verify in DevTools that request
   and response bodies read as `application/mse-sealed`.
3. Watch `mse.sealed` / `mse.plain` request counters until the plaintext share on
   protected routes is only the desktop builds that cannot update.
4. Ship a desktop release that seals. Confirm both login pages (§10a) are sealing in
   production — under `required` their plaintext fallback is refused, and that fallback is
   the only thing standing between a bad deploy and nobody being able to sign in.
5. Flip `MSE_MODE=required`.

## 12. Rotating the key — all five places

The `kid` is pinned in **five** places. Miss one and that surface hard-fails; miss a login
page and nobody can sign in. Do them in this order:

1. New key into `MSE_SERVER_KEY`, old key into `MSE_SERVER_KEY_PREV`, redeploy the
   backend. Both keys now work, so nothing is broken yet — this is the safe window.
2. Rebuild and publish the three SPAs with `VITE_MSE_PIN=<new-kid>,<old-kid>` (both, so
   the build works before *and* after step 4).
3. Update the hard-coded pin in `ide/gate/gate.html` and `server/console-login/login.js`
   to list both kids, run `scripts/sync-mse-client.mjs`, and publish both pages.
   `login.js` and `mse.js` ship together — see §10a.
4. Confirm nothing in the field still pins only the old kid, then clear
   `MSE_SERVER_KEY_PREV` and rebuild the four frontend surfaces with the new kid alone.

Deployed value as of 2026-08-15: `yddBgF9eaS-gVWQtCJKoTwoM`.

**Why the login pages are the dangerous ones.** They refuse to fall back to plaintext on a
`pin` or `downgrade` failure — deliberately, because falling back there would hand a
password to whoever substituted the key. So a stale pin on those two pages does not
degrade quietly; it stops sign-in outright. Every other failure mode (gateway without
MSE, no WebCrypto) still falls back and keeps the door open.
