# Backend & API Design

Battle-tested best practices for building backends and HTTP APIs. Each `##` section is self-contained. Code is JS/TS-first; "Other langs" notes map the idea to Python/Go/Java/Ruby. Default stance: **validate everything, trust nothing from the client, fail loud server-side and quiet client-side.**

## REST resource naming and URL structure

Design URLs around **nouns (resources)**, not verbs (actions). The HTTP method is the verb.

Do:
- Use plural nouns for collections: `/users`, `/orders`, `/orders/42/items`.
- Nest to show ownership, but stop at 2 levels: `/users/42/orders` is fine; `/users/42/orders/7/items/3/reviews` is not — link instead.
- Use `kebab-case` or flat lowercase in paths: `/shipping-addresses`. Never `camelCase` or `snake_case` in the path.
- Put identifiers in the path; put options in the query string: `/orders/42?expand=customer`.
- Keep the resource identity stable. `GET /orders/42` should always mean the same order.

Don't:
- `POST /createUser`, `GET /getUserById?id=5`, `POST /users/5/delete` — verbs in the URL are an RPC smell, not REST.
- Mix singular and plural (`/user` vs `/orders`). Pick plural and be consistent.
- Leak storage details: `/tbl_users`, `/v2_orders_final`.

Actions that don't map to CRUD (e.g. "publish", "cancel") are the pragmatic exception. Two accepted patterns:
- Sub-resource as state: `POST /orders/42/cancellation` or `PUT /orders/42/status` with `{ "status": "cancelled" }`.
- Controller verb (last resort): `POST /orders/42/cancel`. Acceptable for genuinely non-CRUD operations; don't abuse it for everything.

```
GET    /articles            # list
POST   /articles            # create
GET    /articles/123        # read one
PUT    /articles/123        # full replace
PATCH  /articles/123        # partial update
DELETE /articles/123        # delete
GET    /articles/123/comments      # nested collection
POST   /articles/123/publish       # non-CRUD action (last resort)
```

COMMON PITFALLS:
- Verbs in paths (`/getUser`) — the #1 amateur tell.
- Deep nesting that forces clients to know the whole hierarchy. Expose `/comments/55` directly so it's addressable.
- Trailing-slash inconsistency (`/users` vs `/users/`) causing 301s or cache misses. Pick one (usually no trailing slash) and normalize.
- Encoding actions in query params (`/users?action=delete`).

## HTTP methods and idempotency

Each method has a contract. Clients, proxies, and CDNs rely on it.

| Method | Purpose | Safe (no side effects) | Idempotent (repeat = same result) | Body |
|--------|---------|------------------------|-----------------------------------|------|
| GET | Read | Yes | Yes | No |
| HEAD | Read headers only | Yes | Yes | No |
| POST | Create / non-idempotent action | No | **No** | Yes |
| PUT | Create-or-replace at a known URL | No | **Yes** | Yes |
| PATCH | Partial update | No | Not required (make it so if you can) | Yes |
| DELETE | Remove | No | **Yes** | Usually no |

- **GET must be side-effect-free.** Never mutate state on GET. Search-engine crawlers, prefetchers, and `<link rel=prefetch>` will fire your GETs; if `GET /orders/42/delete` deletes the order, a crawler will wipe your DB.
- **PUT is idempotent**: `PUT /users/42` with the same body 5 times leaves one user in the same state. Use PUT when the client controls the resource ID and sends the full representation.
- **POST is not idempotent**: two `POST /orders` = two orders. This is why retries on POST are dangerous — see the idempotency-key pattern in the async section.
- **PATCH** sends only changed fields. JSON Merge Patch (`{ "email": "new@x.com" }`, `null` means delete) is simplest; JSON Patch (RFC 6902, op arrays) is more powerful but rarely needed.
- **DELETE is idempotent in effect**: deleting an already-deleted resource should return 204 or 404, not error. Don't 500 on a double-delete.

Other langs: Express `app.get/post/put/patch/delete`; FastAPI `@app.get/post/...`; Go `r.Method == http.MethodGet` or chi/gorilla routers; Spring `@GetMapping/@PostMapping`. Semantics are identical across all of them — the framework doesn't change the contract.

COMMON PITFALLS:
- Using POST for everything ("POST tunneling") because it's easy — you lose caching, idempotency, and clarity.
- Mutating data in GET handlers (analytics counters, "mark as read" on view) — at minimum make it an explicit POST/PATCH.
- Returning different results for repeated PUT/DELETE and calling it a bug when a retry "fails."

## HTTP status codes — correct usage

The status code is the API's primary signal. Getting it wrong breaks clients, retries, and monitoring.

2xx success:
- `200 OK` — general success with a body (GET, PATCH, action results).
- `201 Created` — resource created. **Include a `Location` header** pointing to the new resource, and usually return the created body.
- `202 Accepted` — accepted for async processing, not yet done. Return a status URL.
- `204 No Content` — success, no body (common for DELETE and some PUTs). Do **not** send a body with 204.

3xx:
- `301/308` permanent redirect, `302/307` temporary. `304 Not Modified` for conditional GETs (ETag/If-None-Match).

4xx — client's fault (do not retry without changing the request):
- `400 Bad Request` — malformed syntax or failed validation.
- `401 Unauthorized` — **not authenticated** (missing/invalid credentials). Misnamed in the spec; it means "unauthenticated."
- `403 Forbidden` — authenticated but **not allowed** to do this.
- `404 Not Found` — resource doesn't exist (or you're hiding its existence from this caller).
- `405 Method Not Allowed` — wrong verb on a valid path; include an `Allow` header.
- `409 Conflict` — state conflict (duplicate unique key, edit conflict, version mismatch).
- `410 Gone` — existed, permanently removed.
- `415 Unsupported Media Type` — wrong `Content-Type`.
- `422 Unprocessable Entity` — syntactically valid but semantically invalid (popular for validation failures; `400` is also fine — pick one convention and stick to it).
- `429 Too Many Requests` — rate limited. Include `Retry-After`.

5xx — server's fault (client may retry idempotent calls):
- `500 Internal Server Error` — unhandled exception. Generic; never leak details.
- `502/503/504` — bad gateway / service unavailable / gateway timeout. Use `503` with `Retry-After` for maintenance and overload.

```ts
// Express
app.post("/orders", async (req, res) => {
  const order = await createOrder(req.body);
  res.status(201).location(`/orders/${order.id}`).json(order);
});

app.delete("/orders/:id", async (req, res) => {
  await deleteOrder(req.params.id); // idempotent: ok if already gone
  res.status(204).end();
});
```

COMMON PITFALLS (the cardinal sins):
- **Returning `200 OK` with `{ "error": "..." }` in the body.** Clients, load balancers, retry libraries, and dashboards all read the status code. A 200 means success; an error in a 200 body is invisible to everything except hand-written client code. **The single most common amateur mistake.**
- Using `200` instead of `201` for creation, or `200` with an empty body instead of `204`.
- Confusing 401 (who are you?) and 403 (you can't do that).
- Returning `500` for client mistakes (bad input is `400`, not `500`) — this pollutes error budgets and pages on-call for non-incidents.
- Using `404` when you mean `403`, or `400` when you mean `409`. Wrong-but-4xx is better than 200, but precise codes let clients react correctly.

## Pagination

**Never return an unbounded list.** A `GET /events` that returns all 2M rows will OOM the server, time out the client, and fall over the day the table grows.

- **Always cap and default `limit`** (e.g. default 20, max 100). Clamp server-side; ignore absurd client values.
- Return enough metadata to fetch the next page and (ideally) total count.

Two strategies:

**Offset/limit** — simple, supports jump-to-page, but slow and inconsistent on large/changing data (`OFFSET 1000000` scans a million rows; rows shift if items are inserted mid-scan).
```
GET /orders?limit=20&offset=40
{ "data": [...], "pagination": { "limit": 20, "offset": 40, "total": 1234 } }
```

**Cursor (keyset)** — scales to huge tables and is stable under inserts. Encode the last-seen sort key as an opaque cursor. Preferred default for large or high-write datasets.
```
GET /orders?limit=20
{ "data": [...], "next_cursor": "eyJpZCI6MTAyfQ==" }
GET /orders?limit=20&cursor=eyJpZCI6MTAyfQ==
```
```ts
// Keyset pagination: order by an indexed, unique, monotonic column (e.g. id or created_at,id)
const limit = Math.min(Number(req.query.limit) || 20, 100);
const afterId = decodeCursor(req.query.cursor); // null on first page
const rows = await db.query(
  `SELECT * FROM orders WHERE ($1::int IS NULL OR id > $1) ORDER BY id ASC LIMIT $2`,
  [afterId, limit + 1] // fetch one extra to know if there's a next page
);
const hasMore = rows.length > limit;
const data = hasMore ? rows.slice(0, limit) : rows;
const next_cursor = hasMore ? encodeCursor(data[data.length - 1].id) : null;
```

Other langs: same pattern everywhere — Django `Paginator`/`CursorPagination`, SQLAlchemy `.limit().offset()` or keyset, Go append `LIMIT $n` to the query. The DB does the work; the framework just shapes it.

COMMON PITFALLS:
- No pagination at all (returning 100k+ rows) — the classic OOM/timeout bug.
- Trusting client `limit` without a max — `?limit=10000000` becomes a denial-of-service.
- Offset pagination on a multi-million-row table — fine at page 1, dies at page 5000.
- `COUNT(*)` on every request for `total` — expensive on big tables; make it optional or approximate.
- Unstable sort (sorting by a non-unique column with no tiebreaker) — rows duplicate or vanish across pages. Always add a unique tiebreaker (e.g. `ORDER BY created_at DESC, id DESC`).

## Filtering, sorting, and field selection

Use the query string. Keep it predictable and bounded.

- Filter by equality: `GET /orders?status=paid&customer_id=42`.
- Ranges: `GET /events?since=2026-01-01&until=2026-02-01` (or `price[gte]=10&price[lte]=50`).
- Sorting: `GET /orders?sort=-created_at,total` (leading `-` = descending). **Whitelist sortable fields** — never interpolate a client string into `ORDER BY`.
- Sparse fieldsets: `GET /users?fields=id,name,email` to shrink payloads.
- Search: `GET /products?q=keyboard`.

```ts
const SORTABLE = new Set(["created_at", "total", "id"]); // whitelist!
const sort = String(req.query.sort ?? "-created_at")
  .split(",")
  .map(s => {
    const dir = s.startsWith("-") ? "DESC" : "ASC";
    const col = s.replace(/^-/, "");
    if (!SORTABLE.has(col)) throw new BadRequest(`Cannot sort by ${col}`);
    return `${col} ${dir}`; // col is from a fixed allow-list, safe to interpolate
  })
  .join(", ");
```

COMMON PITFALLS:
- Interpolating `sort`/`filter` field names straight into SQL — **SQL injection via column name**, which parameterized values can't protect against. Whitelist identifiers.
- Unbounded `OR`/`LIKE '%term%'` full-table scans with no index — slow query DoS.
- Allowing filters on unindexed columns at scale.
- Reinventing a query DSL in the URL; if you need GraphQL-level flexibility, use GraphQL, don't build a half one.

## API versioning

You will need to make breaking changes. Plan for it from v1.

- **URL path versioning** (`/v1/users`) — most common, explicit, easy to route and cache. Recommended default.
- **Header versioning** (`Accept: application/vnd.myapi.v2+json`) — cleaner URLs, harder to test/debug, easy to forget.
- **Query param** (`/users?version=2`) — works but pollutes caching and feels bolt-on.

Rules:
- Version from day one; ship `/v1` even for your first release.
- A **breaking change** = removing/renaming a field, changing a type, making an optional field required, changing semantics. These require a new version.
- **Additive changes are not breaking**: adding a new optional field or endpoint. Clients must tolerate unknown fields (don't break on extra JSON keys).
- Keep old versions alive during a published deprecation window. Announce via `Deprecation` and `Sunset` headers and changelogs.
- Don't proliferate versions (`/v1`...`/v7`). Each live version is maintenance and security surface.

```ts
app.use("/v1", v1Router);
app.use("/v2", v2Router);
// v2 reuses v1 handlers where unchanged; only diverges where it must
```

COMMON PITFALLS:
- No versioning, then a breaking change ships and every client breaks at once.
- Breaking v1 in place "because it's a small change" — small to you, fatal to a client parsing a removed field.
- Maintaining 6 versions forever. Deprecate and sunset on a schedule.

## Request validation — validate everything at the boundary

**Never trust the client.** Every byte from a request (body, query, params, headers, cookies) is attacker-controlled until validated. Validate at the edge, before any business logic touches it, and reject with a clear `400`/`422`.

- Use a schema validator: **zod** or **valibot** (TS), **joi**/**yup** (JS), **pydantic** (Python), **go-playground/validator** (Go), **Bean Validation/Hibernate Validator** (Java), **dry-validation** (Ruby).
- Validate type, presence, format, length, range, and enum membership. Coerce/parse into typed values — don't pass raw strings deeper.
- **Strip unknown fields** (allow-list), don't just ignore them. Otherwise mass-assignment lets a user POST `{ "role": "admin", "isVerified": true }` and silently set privileged fields.
- Return a structured error listing **which fields failed and why** — don't make the client guess.
- Validate query/path params too (`page`, `id`), not just bodies. `/users/not-a-number` should be a clean 400, not a 500 from a parse error.

```ts
import { z } from "zod";

const CreateUser = z.object({
  email: z.string().email(),
  password: z.string().min(12).max(200),
  age: z.number().int().min(13).max(120).optional(),
  role: z.enum(["member", "editor"]).default("member"), // never accept "admin" from client
}).strict(); // .strict() rejects unknown keys -> blocks mass assignment

app.post("/users", (req, res, next) => {
  const parsed = CreateUser.safeParse(req.body);
  if (!parsed.success) {
    return res.status(400).json({
      error: { code: "VALIDATION_ERROR", message: "Invalid request body",
        details: parsed.error.issues.map(i => ({ path: i.path.join("."), message: i.message })) }
    });
  }
  createUser(parsed.data).then(u => res.status(201).json(u)).catch(next);
});
```

Python (pydantic): a `BaseModel` with typed fields validates and coerces automatically; FastAPI does it per-route from the type hints and auto-returns 422 with details.

COMMON PITFALLS:
- No validation at all — the root cause of injection, crashes, and corrupt data.
- Validating only the happy-path field and trusting the rest.
- Accepting and persisting unknown fields (mass assignment → privilege escalation).
- Validating in the frontend only — the client is not a security boundary; anyone can hit the API directly with curl.
- Enforcing length/format in the DB constraint only, so the failure surfaces as an ugly 500 instead of a clean 400.
- Using validation messages that echo raw input back unescaped into HTML (reflected XSS in error pages).

## Authentication vs authorization

These are different. Conflating them causes real breaches.

- **Authentication (authN)** = *who are you?* Verifying identity (login, token, API key). Failure → **401**.
- **Authorization (authZ)** = *what are you allowed to do?* Checking permissions on a specific resource/action. Failure → **403**.

You always do authN first, then authZ. A valid token (authN passed) does **not** mean the user may touch this object (authZ still required).

- Do authZ on **every** protected request, against the **authenticated identity from the token/session**, never against an ID the client supplies.
- Check ownership/role at the resource level: "can *this* user read *this* order?" — not just "is this user logged in?"

```ts
// BROKEN: trusts client-supplied userId -> IDOR, anyone reads anyone's data
app.get("/orders", (req, res) => getOrdersForUser(req.query.userId));

// CORRECT: identity comes from the verified token; authZ checks ownership
app.get("/orders/:id", requireAuth, async (req, res) => {
  const order = await getOrder(req.params.id);
  if (!order) return res.sendStatus(404);
  if (order.userId !== req.auth.userId && req.auth.role !== "admin")
    return res.sendStatus(403); // authenticated but not authorized
  res.json(order);
});
```

COMMON PITFALLS:
- **Trusting client-supplied `user_id`/`role`/`account_id`** from body, query, or a custom header. The biggest authZ bug class: **IDOR / broken object-level authorization**. Identity must come from the server-verified token/session only.
- Checking "is logged in" but not "owns this resource" — every authenticated user can read every record by changing the ID.
- Doing authZ in the frontend (hiding a button) without enforcing it server-side.
- Role checks scattered and inconsistent — centralize in middleware/policies.
- Returning 200 for an unauthorized read (leaks data) — fail with 403/404.

## JWT vs sessions — and where to store tokens

**Stateful sessions**: server stores session state (in Redis/DB), client holds an opaque session ID in a cookie. Easy to revoke (delete the row). Needs a session store. Great default for first-party web apps.

**JWT (stateless tokens)**: signed token carrying claims; server verifies the signature, stores nothing. Scales horizontally with no shared store, good for microservices/mobile/third-party APIs. **Downside: hard to revoke before expiry** — a stolen token is valid until it expires. Mitigate with short lifetimes + a refresh-token + revocation list.

Pick:
- First-party web app, single backend → **sessions** are simpler and revocable.
- Multiple services / mobile / stateless scaling → **JWT** (short-lived access + refresh).
- Don't store sensitive or large data in a JWT — it's signed, **not encrypted**; anyone can base64-decode and read the claims.

**Where to store the token in a browser — the key security tradeoff:**
- **`httpOnly`, `Secure`, `SameSite` cookie** — JavaScript cannot read it, so **XSS can't steal it**. This is the recommended store for web. Cost: cookies are sent automatically, so you must defend **CSRF** (`SameSite=Lax/Strict` + CSRF token for state-changing requests).
- **`localStorage`/`sessionStorage`** — readable by any JS on the page, so **any XSS exfiltrates the token instantly**. Convenient for SPAs/attaching `Authorization` headers, but a single XSS = full account takeover. Avoid for anything sensitive.

```ts
res.cookie("session", token, {
  httpOnly: true,   // JS can't read it -> XSS-resistant
  secure: true,     // HTTPS only
  sameSite: "lax",  // CSRF mitigation
  maxAge: 1000 * 60 * 15, // short-lived access token (15 min)
  path: "/",
});
```

JWT verification (always verify; never decode-and-trust):
```ts
import jwt from "jsonwebtoken";
const payload = jwt.verify(token, process.env.JWT_SECRET!, {
  algorithms: ["HS256"],     // pin the algorithm
  issuer: "myapp", audience: "myapp-web",
  maxAge: "15m",
});
```

COMMON PITFALLS:
- **Storing JWTs in `localStorage`** and getting wrecked by XSS. Prefer httpOnly cookies for browsers.
- **The `alg: "none"` attack** and algorithm confusion: never accept the token's self-declared algorithm. Pin `algorithms: ["HS256"]` (or RS256) server-side.
- Forgetting to verify `exp`, `iss`, `aud` — accepting expired or foreign tokens.
- Putting secrets/PII in JWT claims (it's readable).
- Long-lived (days/weeks) access tokens you can't revoke — use short access + refresh.
- Using a weak/short HS256 secret. Use a long random secret or RS256 keypair.

## Password hashing and credential storage

**Never store passwords in plaintext. Never hash them with MD5/SHA-1/SHA-256 alone.** Fast hashes are trivially brute-forced by GPUs (billions/sec).

- Use a **slow, salted, memory-hard** password hash: **argon2id** (best modern choice), **bcrypt** (battle-tested, fine), or **scrypt**. Each salts automatically and embeds the salt + cost in the output.
- bcrypt: cost factor ≥ 12. Note its 72-byte input truncation; pre-hash with SHA-256 if you must allow longer passwords.
- argon2id: tune memory/iterations to your hardware; it resists GPU/ASIC attacks better.
- **Use constant-time comparison** (the library's `verify` does this) to avoid timing attacks. Never `===` on hashes.
- Never log, return, or include the password or its hash in any response or error.
- Enforce a sane minimum length (12+); allow long passphrases; don't impose dumb composition rules that hurt usability.
- Store password reset tokens **hashed** too, with a short TTL and single use.

```ts
import bcrypt from "bcrypt";
const hash = await bcrypt.hash(password, 12);           // on signup; salt is automatic + embedded
const ok   = await bcrypt.compare(password, user.hash); // on login; constant-time
```
```py
# Python
from argon2 import PasswordHasher
ph = PasswordHasher()
hash_ = ph.hash(password)
ph.verify(hash_, password)  # raises on mismatch; constant-time
```

Other langs: Go `golang.org/x/crypto/bcrypt` or `argon2`; Java Spring `BCryptPasswordEncoder`/`Argon2PasswordEncoder`; Ruby `bcrypt` gem / `has_secure_password`.

COMMON PITFALLS:
- Plaintext storage (catastrophic and shockingly common in amateur code).
- `md5(password)` / `sha256(password)` — fast hashes, instantly cracked with rainbow tables/GPUs.
- Hand-rolling your own salt+hash scheme — use a vetted library.
- A single global salt, or no salt — identical passwords hash identically; one rainbow table cracks all.
- Comparing hashes with `==`/`===` (timing leak) instead of the library verify.
- Reflecting password values in logs, stack traces, or validation errors.

## Refresh tokens and session lifecycle

Short access tokens limit blast radius; refresh tokens keep users logged in without re-entering credentials.

- **Access token**: short-lived (5–15 min), sent on every request. If stolen, it expires fast.
- **Refresh token**: long-lived (days–weeks), used **only** at a dedicated `/auth/refresh` endpoint to mint a new access token. Store it httpOnly + Secure; ideally keep a server-side record so you can revoke.
- **Rotate** refresh tokens on every use: issue a new one, invalidate the old. If an old (already-used) refresh token is presented again, treat it as theft → revoke the whole token family and force re-login (reuse detection).
- Support **logout = server-side revocation** (delete the session/refresh record). Pure stateless JWT can't truly log out before expiry — that's why you keep a refresh/blocklist store.
- Bind tokens to context where feasible (device, client id) and expire idle sessions.

```ts
app.post("/auth/refresh", async (req, res) => {
  const rt = req.cookies.refresh_token;
  const rec = await findRefreshToken(rt);
  if (!rec || rec.revoked) return res.sendStatus(401);
  if (rec.used) { await revokeFamily(rec.familyId); return res.sendStatus(401); } // reuse = theft
  await markUsed(rec.id);
  const newRefresh = await issueRefresh(rec.userId, rec.familyId); // rotation
  const access = signAccessToken(rec.userId);
  setAuthCookies(res, access, newRefresh);
  res.sendStatus(204);
});
```

COMMON PITFALLS:
- Long-lived access tokens with no refresh — either annoying re-logins or huge theft windows.
- Refresh token usable as an access token on normal endpoints — restrict it to `/auth/refresh`.
- No rotation/reuse detection — a stolen refresh token works indefinitely.
- No revocation path, so "log out everywhere" and incident response are impossible.
- Refresh token in localStorage — XSS steals long-lived access.

## Error handling and consistent error shape

Errors are part of your API contract. Make them **consistent, machine-readable, and leak-free.**

- Define **one error envelope** and use it everywhere:
```json
{ "error": { "code": "ORDER_NOT_FOUND", "message": "Order 42 does not exist", "details": [], "request_id": "req_abc123" } }
```
- Use a **stable `code`** (string enum) for programmatic handling; `message` is human-facing; `details` carries field errors; `request_id` ties it to logs. Consider RFC 9457 (`application/problem+json`) as a standard shape.
- **Match the HTTP status to the error** (validation→400/422, auth→401/403, missing→404, conflict→409, server→500). The status is the primary signal; the body adds detail.
- **Never leak internals to clients**: no stack traces, SQL, file paths, dependency versions, raw exception messages, or hostnames. These hand attackers a map. Log them server-side; return a generic message + `request_id`.
- Have **one global error handler** so no route can leak an unformatted error. Catch async errors too.
- Distinguish **expected errors** (throw typed `AppError` with a status + code) from **unexpected** ones (bugs → 500, generic message).

```ts
class AppError extends Error {
  constructor(public status: number, public code: string, msg: string, public details: unknown[] = []) {
    super(msg);
  }
}

// Global error handler — MUST be last, MUST have 4 args in Express
app.use((err: unknown, req: Request, res: Response, _next: NextFunction) => {
  const requestId = req.headers["x-request-id"] ?? crypto.randomUUID();
  if (err instanceof AppError) {
    return res.status(err.status).json({
      error: { code: err.code, message: err.message, details: err.details, request_id: requestId },
    });
  }
  logger.error({ err, requestId, path: req.path }); // full detail to logs only
  res.status(500).json({
    error: { code: "INTERNAL_ERROR", message: "An unexpected error occurred", request_id: requestId },
  });
});
```

Other langs: FastAPI `exception_handler` / custom `HTTPException`; Go middleware that recovers panics and writes a JSON error; Spring `@ControllerAdvice` + `@ExceptionHandler`; Rails `rescue_from`. Same idea: centralize, normalize, hide internals.

COMMON PITFALLS:
- Returning `200` with an error body (see status-codes section — the cardinal sin).
- Leaking stack traces / SQL / exception text to clients (info disclosure; e.g. a DB error exposing table names).
- Inconsistent shapes per endpoint — clients can't write one handler.
- Swallowing errors silently (`catch (e) {}`) so failures vanish with no log and a misleading success.
- Forgetting async errors leak past `try/catch` in callbacks/promises if not awaited — unhandled rejection crashes the process or hangs the request.
- Putting the global handler before routes, or omitting the 4th `next` arg in Express (it won't register as an error handler).

## Logging and observability

You can't fix what you can't see. Log enough to debug, never enough to breach privacy.

- **Structured (JSON) logs**, not `console.log("user " + id + " did thing")`. Use pino/winston (JS), structlog/`logging` (Py), zap/zerolog (Go), SLF4J/Logback (Java).
- Attach a **`request_id`/correlation ID** to every log line in a request (propagate it across services). This is how you trace one request through the whole system.
- Log levels: `error` (needs attention), `warn`, `info` (request start/finish, key events), `debug` (off in prod). Don't `error`-log expected 4xx — that's noise that drowns real incidents.
- **Never log secrets or sensitive PII**: passwords, tokens, API keys, full card numbers, auth headers, raw request bodies of auth endpoints. Redact them.
- Log the **outcome** of each request: method, path, status, latency, user id (or anon), request id.
- Emit metrics (request rate, error rate, p50/p95/p99 latency) and traces for hot paths. Alert on error rate and latency, not just "is it up."

```ts
import pino from "pino";
const logger = pino({ redact: ["req.headers.authorization", "*.password", "*.token"] });
logger.info({ requestId, userId, method: req.method, path: req.path, status: res.statusCode, ms }, "request");
```

COMMON PITFALLS:
- `console.log` everywhere with no structure, level, or correlation id — ungreppable, unparseable.
- Logging secrets/tokens/PII (now your logs are a breach surface and a compliance problem).
- No request id — impossible to correlate a user's failed request with server logs.
- Logging at `error` for routine 404s/validation — real errors get buried.
- Logging huge payloads/PII bodies that blow up log volume and cost.

## Async, the event loop, and not blocking

In single-threaded runtimes (Node.js), **blocking the event loop blocks every concurrent request**. One slow synchronous operation freezes the whole server.

- **Never do sync I/O or heavy CPU on the event loop.** No `fs.readFileSync`, `crypto.pbkdf2Sync`, big `JSON.parse` of huge payloads, synchronous `bcrypt`, or tight CPU loops in a request handler.
- Use async I/O (`await fs.promises.readFile`, async DB drivers). Offload CPU-bound work (hashing, image/video, compression, crypto) to **worker threads**, a queue, or a separate service.
- Don't `await` independent operations serially — run them concurrently with `Promise.all`. But don't fire thousands at once; bound concurrency (e.g. `p-limit`).
- Always `await` promises (or handle rejections). A floating promise = swallowed errors and unhandled rejections.

```ts
// BAD: serial awaits triple the latency; and a sync hash blocks the loop
const a = await getUser(id);
const b = await getOrders(id);
const hash = bcrypt.hashSync(pw, 12); // BLOCKS every other request

// GOOD: concurrent independent I/O + async CPU offload
const [user, orders] = await Promise.all([getUser(id), getOrders(id)]);
const hash = await bcrypt.hash(pw, 12); // async, yields to the loop
```

Other langs: Python — don't run blocking calls in an `async def` without `run_in_executor`/`asyncio.to_thread`; mixing sync DB drivers into async frameworks stalls the loop. Go — goroutines are cheap, but still bound concurrency and protect shared state. Java — use thread pools; don't block request threads on slow downstreams without timeouts.

COMMON PITFALLS:
- Sync FS/crypto/`*Sync` calls in handlers — server "randomly" hangs under load.
- Heavy CPU (parsing, hashing, image work) on the main thread — throughput collapses, p99 latency spikes.
- Floating promises / missing `await` — errors vanish, ordering breaks.
- `await` in a `for` loop over a large array when the calls are independent (serial when it should be parallel).
- Unbounded `Promise.all` over 10k items — exhausts file descriptors/connections.

## Connection pooling and database access

Opening a DB/HTTP connection per request is slow and exhausts limits. **Pool and reuse.**

- Create **one shared connection pool** at startup; reuse it for all requests. Never `new Pool()` / connect-per-request.
- Size the pool sensibly (e.g. 10–20 per instance) and **cap total connections across all instances** below the DB's `max_connections`. N instances × pool size can blow past Postgres limits and cause "too many connections."
- **Always release connections** back to the pool (use the framework's managed query API; if you check one out manually, release in `finally`). A leaked connection per request drains the pool until everything hangs.
- Set **statement/query timeouts** so one slow query can't pin a connection forever.
- Use transactions for multi-step writes; keep them short — long transactions hold locks and connections.
- Use a separate, larger pool tier or a proxy (PgBouncer) if you have many app instances.

```ts
import { Pool } from "pg";
const pool = new Pool({ max: 15, idleTimeoutMillis: 30_000, connectionTimeoutMillis: 5_000,
  statement_timeout: 10_000 }); // created ONCE, module scope

// pool.query checks out + releases automatically:
const { rows } = await pool.query("SELECT 1");

// manual checkout MUST release:
const client = await pool.connect();
try { await client.query("BEGIN"); /* ... */ await client.query("COMMIT"); }
catch (e) { await client.query("ROLLBACK"); throw e; }
finally { client.release(); } // ALWAYS, even on error
```

COMMON PITFALLS:
- New connection per request — latency and connection exhaustion.
- Leaking connections (no `release()`/`finally`) — pool drains, app deadlocks under load.
- Pool size × instance count > DB max connections — intermittent "too many clients" outages.
- No statement timeout — one runaway query holds a connection until it's killed manually.
- Long-running transactions holding locks — write contention and deadlocks.

## Timeouts, retries, and idempotency for external calls

Every network call can hang or fail. **Put a timeout on every external call**, and make retries safe.

- **Set an explicit timeout on every outbound call** (HTTP, DB, cache, queue). Many clients default to *infinite* — one stuck downstream then exhausts your threads/connections and cascades into a full outage.
- Set both connect and overall/read timeouts. Keep them shorter than your own request budget so you can fail fast and return a clean 504.
- **Retry only safe (idempotent) operations** (GET, PUT, DELETE) with **exponential backoff + jitter** and a small cap (2–3). Don't retry POSTs that create things unless you have an idempotency key — you'll double-charge/double-create.
- Use **idempotency keys** for non-idempotent operations so client retries are safe: client sends `Idempotency-Key: <uuid>`; server records it, and a repeat with the same key returns the original result instead of doing the work twice (the Stripe pattern).
- Add a **circuit breaker** for chronically failing dependencies — stop hammering a dead service; fail fast and recover.
- Distinguish retryable (timeout, 502/503/504, connection reset) from non-retryable (400/401/403/404/422) — never retry a 4xx; the request is wrong.

```ts
// Always timeout outbound HTTP
const ctrl = new AbortController();
const t = setTimeout(() => ctrl.abort(), 3000); // 3s budget
try {
  const r = await fetch(url, { signal: ctrl.signal });
} finally { clearTimeout(t); }

// Idempotency key on a charge so a retry doesn't double-charge
app.post("/charges", async (req, res) => {
  const key = req.header("Idempotency-Key");
  if (!key) return res.status(400).json({ error: { code: "IDEMPOTENCY_KEY_REQUIRED" } });
  const existing = await getIdempotentResult(key);
  if (existing) return res.status(existing.status).json(existing.body); // replay original
  const result = await chargeCard(req.body);
  await saveIdempotentResult(key, 201, result);
  res.status(201).json(result);
});
```

Other langs: Python `requests`/`httpx` — pass `timeout=` (requests defaults to **no timeout**!); Go — use `context.WithTimeout` and `http.Client{Timeout: ...}`; Java — set connect/read timeouts on the client and use Resilience4j for retries/breakers.

COMMON PITFALLS:
- **No timeout on outbound calls** (the silent killer) — one hung dependency cascades into total failure. `requests.get(url)` with no `timeout` can hang forever.
- Retrying non-idempotent POSTs with no idempotency key — duplicate orders, double charges.
- Retrying 4xx — wastes time; the request won't get more valid by repeating.
- Retries with no backoff/jitter — a thundering herd that DoSes the recovering service.
- Client timeout shorter than server's, so the client retries while the server is still processing the first attempt (compounding load).

## Rate limiting and abuse protection

Without limits, one client (or attacker) can exhaust your capacity, brute-force passwords, or run up your bill.

- **Limit per identity** (API key / authenticated user) for normal traffic, and **per IP** for unauthenticated endpoints. Combine both.
- Apply **stricter limits to sensitive/expensive endpoints**: login, signup, password reset, OTP, search, file upload, anything that costs money or sends email/SMS.
- Algorithms: **token bucket** (allows bursts, refills steadily — good default) or **sliding window** (smoother). Fixed windows allow 2x bursts at the boundary.
- Store counters in a **shared store (Redis)** so the limit holds across all instances — in-memory counters reset per process and are bypassable behind a load balancer.
- On limit, return **`429 Too Many Requests`** with a **`Retry-After`** header. Expose `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset` so good clients can self-throttle.
- Behind a proxy/CDN, derive the client IP from the **trusted** `X-Forwarded-For` (and configure `trust proxy`), not the raw socket — and don't trust client-set IP headers blindly (they're spoofable).
- Add specific brute-force defenses on auth: progressive delays, lockout/CAPTCHA after N failures, and per-account + per-IP login limits.

```ts
// express-rate-limit + Redis store (shared across instances)
import rateLimit from "express-rate-limit";
const loginLimiter = rateLimit({
  windowMs: 15 * 60 * 1000, limit: 5,            // 5 login attempts / 15 min / key
  standardHeaders: true, legacyHeaders: false,
  keyGenerator: (req) => req.ip,                 // or user id / api key
  handler: (req, res) => res.status(429).set("Retry-After", "900")
    .json({ error: { code: "RATE_LIMITED", message: "Too many attempts, try later" } }),
});
app.post("/auth/login", loginLimiter, loginHandler);
```

Other langs: Python `slowapi`/`django-ratelimit`; Go `golang.org/x/time/rate` or a Redis limiter; Java Bucket4j / Spring Cloud Gateway; or do it at the gateway (NGINX `limit_req`, API gateway, Cloudflare).

COMMON PITFALLS:
- No rate limiting on login/signup/reset — open to credential-stuffing and email/SMS-bombing.
- In-memory counters behind multiple instances — limits don't actually hold; trivially bypassed.
- Returning `503`/`403` instead of `429`, or omitting `Retry-After` — clients can't back off correctly.
- Limiting by raw socket IP behind a proxy (everyone shares the proxy IP) or trusting a spoofable `X-Forwarded-For` without a trusted-proxy config.
- Forgetting cost-based limits — one user running 10k expensive queries within the "request count" limit still melts the DB.

## Secrets and configuration (12-factor)

Config that varies per environment, and all secrets, belong in the **environment**, not the code.

- **Never commit secrets** (API keys, DB passwords, JWT secrets, private keys) to git. They live forever in history and leak via forks, CI logs, and screenshots.
- Read config from **environment variables** (or a secrets manager). Provide a checked-in **`.env.example`** with keys and dummy values; **gitignore the real `.env`**.
- In production, inject secrets via a **secrets manager** (Vault, AWS/GCP Secrets Manager, Doppler) or the platform's encrypted env, not a plaintext file on disk.
- **Validate config at startup** and **fail fast** if a required variable is missing or malformed — crash on boot, don't discover it on the first request in prod.
- Keep config **per-environment** with no code differences between dev/stage/prod — only env values change (12-factor). No `if (env === "prod")` branches scattered through business logic.
- **Rotate** secrets periodically and immediately on suspected exposure. If a secret hits git, rotate it — deleting the commit is not enough; assume it's compromised.
- Separate credentials per environment (dev keys ≠ prod keys) and scope them to least privilege.

```ts
import { z } from "zod";
const Env = z.object({
  DATABASE_URL: z.string().url(),
  JWT_SECRET: z.string().min(32),
  PORT: z.coerce.number().default(3000),
  NODE_ENV: z.enum(["development", "test", "production"]),
});
export const env = Env.parse(process.env); // throws at startup if anything is missing/invalid
```
```gitignore
.env
.env.local
*.pem
```

Other langs: Python `os.environ` + pydantic-settings; Go `os.Getenv` + a config struct/validation; Java Spring `application.yml` with `${ENV_VAR}` placeholders + profiles. Same rule everywhere: secrets out of source, validated at boot.

COMMON PITFALLS:
- **Hardcoding secrets in source** (`const JWT_SECRET = "supersecret"`) — leaks via the repo, the #1 way credentials end up on the public internet.
- Committing a real `.env`, then "removing" it later — it's already in git history and on every clone; rotate it.
- No startup validation — the app boots fine and 500s on the first request because `DATABASE_URL` is undefined.
- Logging the full config/env at boot (prints secrets to logs).
- Sharing one set of credentials across dev/stage/prod, or giving prod keys full admin scope.
- Baking secrets into Docker images or build args (they persist in image layers).

## SQL injection and safe database queries

**Never build queries by string concatenation/interpolation with untrusted input.** This is SQL injection — among the oldest and most devastating vulnerabilities (data theft, deletion, auth bypass, RCE).

- **Always use parameterized queries / prepared statements.** The driver sends the SQL and the values separately, so input can never change the query structure.
- Use an ORM/query builder (Prisma, Drizzle, SQLAlchemy, ActiveRecord, GORM) which parameterizes by default — but stay alert to their raw-query escape hatches.
- **Parameters bind values, not identifiers.** Table/column names, `ORDER BY` directions, and `LIMIT` keywords can't be parameterized — **whitelist** those against a fixed allow-list (see filtering section).
- Apply least-privilege DB users (the app role shouldn't be able to `DROP TABLE` or read other schemas).
- Validate/normalize input before it ever reaches the query (defense in depth), but parameterization is the actual fix — not input "sanitization" by escaping quotes.

```ts
const email = req.query.email;

// CATASTROPHIC: string interpolation -> SQL injection
// input  ' OR '1'='1  ->  returns/auth-bypasses everything
await pool.query(`SELECT * FROM users WHERE email = '${email}'`); // NEVER

// CORRECT: parameterized ($1 is bound as a value, not SQL)
await pool.query("SELECT * FROM users WHERE email = $1", [email]);
```
```py
# CORRECT (psycopg) — pass params, never use f-strings/% formatting for values
cur.execute("SELECT * FROM users WHERE email = %s", (email,))
```

Other langs: Go `db.Query("... WHERE id = $1", id)`; Java `PreparedStatement` with `setString`; Ruby `User.where(email: email)` (never string-interpolate into `where("email = '#{email}'")`). NoSQL isn't immune — Mongo query injection happens when you pass raw user objects as query filters; validate types so `{ "$gt": "" }` can't sneak in.

COMMON PITFALLS:
- **Building SQL with `+`, template literals, f-strings, or `.format()`** on user input — the textbook vulnerability.
- Thinking an ORM makes you immune, then using `raw()`/`query()` with interpolation.
- Trying to "escape" quotes manually instead of parameterizing — incomplete and bypassable.
- Interpolating column/table names from user input (parameters can't help here) — whitelist them.
- Passing un-typed user JSON straight into a NoSQL query filter (operator injection).

## Amateur-mistake checklist (quick scan before shipping)

A fast audit of the bugs that mark inexperienced backend code. If any are true, fix before shipping.

- [ ] **No input validation** — request bodies/params/query trusted as-is. → validate at the boundary with a schema, reject with 400/422.
- [ ] **SQL built by string concatenation/interpolation** — injection. → parameterized queries everywhere; whitelist identifiers.
- [ ] **Returning `200` on failure** (error in a 200 body) — invisible to clients/monitoring. → correct 4xx/5xx status codes.
- [ ] **No timeouts on external calls** — one hung dependency cascades. → explicit connect+read timeout on every outbound call.
- [ ] **Blocking the event loop** (sync FS/crypto/CPU in handlers) — server hangs under load. → async I/O; offload CPU to workers/queues.
- [ ] **Secrets in code/committed `.env`** — credential leak. → env vars / secrets manager; gitignore `.env`; rotate exposed secrets.
- [ ] **No pagination** — endpoints return 100k+ rows, OOM/timeout. → always cap+default `limit`; cursor pagination at scale.
- [ ] **Trusting client-supplied `user_id`/`role`/`account_id`** — IDOR / privilege escalation. → identity from the verified token; authZ on every resource.
- [ ] **Plaintext or fast-hash (md5/sha) passwords** — instantly cracked. → argon2id/bcrypt with salt; constant-time verify.
- [ ] **JWT in localStorage** — XSS = account takeover. → httpOnly+Secure+SameSite cookies; defend CSRF.
- [ ] **Leaking stack traces/SQL/internals to clients** — info disclosure. → generic message + request_id to client; full detail to logs only.
- [ ] **No global error handler / swallowed errors** (`catch (e) {}`) — failures vanish or leak. → centralize; normalize; never swallow.
- [ ] **New DB connection per request / leaked connections** — exhaustion and deadlock. → shared pool; release in `finally`; statement timeouts.
- [ ] **No rate limiting on auth/expensive endpoints** — brute force, bill blowups. → per-key + per-IP limits in a shared store; 429 + Retry-After.
- [ ] **Mass assignment** (persisting unknown body fields) — privilege escalation. → strict schemas; allow-list fields; never accept `role` from client.
- [ ] **Mutating state on GET** — crawlers/prefetch trigger it. → GET is read-only; use POST/PATCH/DELETE for changes.
- [ ] **Retrying non-idempotent POSTs without an idempotency key** — duplicates/double charges. → idempotency keys; retry only safe ops with backoff+jitter.
- [ ] **No request size/upload limits** — memory exhaustion DoS. → cap body size; stream/validate uploads; limit file types.
- [ ] **CORS set to `*` with credentials, or reflecting any Origin** — cross-site data theft. → explicit allow-list of trusted origins.
- [ ] **No HTTPS / sending tokens over plaintext** — interception. → TLS everywhere; `Secure` cookies; HSTS.
