# Application Security

Defensive cheat-sheet for writing safe application code. Each `##` section is self-contained. Rule of thumb: **all input is hostile until proven otherwise**, **trust nothing from the client**, and **fail closed** (deny by default). When unsure, prefer the SAFE pattern even if it's more verbose.

## Threat Model Mindset (read first)

- Treat every byte that crosses a trust boundary as attacker-controlled: HTTP params, headers, cookies, JSON bodies, file uploads, webhook payloads, DB rows written by other users, env from less-trusted services, even filenames.
- The client is the attacker's machine. JavaScript validation, hidden fields, disabled buttons, and `role` checks in the SPA are UX, not security. Re-check everything server-side.
- Fail closed: default-deny authz, default-escape output, default-reject malformed input. An exception/early-return must never leave the user authenticated or authorized.
- Defense in depth: parameterized queries AND least-privilege DB user AND input validation. One control failing shouldn't be game over.
- Don't invent crypto, auth, or sanitizers. Use vetted libraries (argon2, libsodium, your framework's ORM/escaper, DOMPurify). Hand-rolled = bugs.
- Least information: error messages, timing, and response differences all leak. Generic responses to untrusted callers.

## Injection: SQL

Never build queries by concatenating or interpolating user input. Use parameterized queries / prepared statements / bound parameters. This is non-negotiable and covers ~all SQLi.

VULNERABLE — string concatenation/interpolation:
```js
// node + pg  — attacker sends email = "' OR '1'='1' --"
const q = `SELECT * FROM users WHERE email = '${email}'`;
await db.query(q);

// python
cur.execute("SELECT * FROM users WHERE id = %s" % user_id)   // % formatting = injection
cur.execute(f"SELECT * FROM users WHERE name = '{name}'")     // f-string = injection
```

SAFE — bound parameters (driver does the escaping):
```js
// node + pg
await db.query('SELECT * FROM users WHERE email = $1', [email]);

// python (note: comma, not %)
cur.execute("SELECT * FROM users WHERE id = %s", (user_id,))
cur.execute("SELECT * FROM users WHERE name = %s", (name,))

// Go database/sql
db.Query("SELECT * FROM users WHERE email = ?", email)

// Java JDBC
PreparedStatement ps = c.prepareStatement("SELECT * FROM users WHERE email = ?");
ps.setString(1, email);
```

- ORMs (Prisma, SQLAlchemy, ActiveRecord, Hibernate, Sequelize): safe **when you pass values as arguments**. They become unsafe the moment you drop to raw SQL with string building (`.raw()`, `text()`, `whereRaw()`, `extra()`). If you must use raw, still bind params.
- **Identifiers (table/column/sort/direction) cannot be parameterized.** If a column name or `ORDER BY ... ASC/DESC` comes from the user, validate against a hardcoded allowlist — never interpolate it.
  ```js
  const SORT = { name: 'name', created: 'created_at' };           // allowlist
  const col = SORT[req.query.sort] ?? 'created_at';
  const dir = req.query.dir === 'asc' ? 'ASC' : 'DESC';          // map, don't pass through
  db.query(`SELECT * FROM users ORDER BY ${col} ${dir} LIMIT $1`, [limit]);
  ```
- `LIMIT`/`OFFSET`: bind them or coerce to integers; don't interpolate raw strings.
- Don't rely on escaping functions (`mysql_real_escape_string`-style) or blocklists of keywords/quotes — they get bypassed. Parameterize.
- Stored procedures help only if they themselves use bound params internally.

## Injection: NoSQL

- MongoDB & friends are injectable via **operator injection** when user input becomes a query object. JSON bodies let attackers smuggle `$gt`, `$ne`, `$where`, `$regex`.

VULNERABLE — login bypass with `{ "password": { "$ne": null } }`:
```js
// req.body = { user: "admin", password: { "$gt": "" } }
db.users.findOne({ user: req.body.user, password: req.body.password });
// query becomes password > "" → matches, auth bypassed
```

SAFE — coerce to expected primitive types and validate before querying:
```js
const user = String(req.body.user);
const password = String(req.body.password);     // now "$gt" object → "[object Object]", harmless
const u = await db.users.findOne({ user });
if (!u || !(await verifyHash(password, u.passwordHash))) return fail();
```

- Use a schema validator (zod/Joi/ajv/Mongoose with strict types) so `password` must be a `string`, not an object.
- Never pass `$where`, `$function`, or `mapReduce` with user-influenced JS strings — that's server-side JS eval (RCE-adjacent).
- For `$regex`, treat user input as a literal: escape regex metacharacters or use anchored exact match; an unescaped user regex enables ReDoS and unintended matches.
- Same idea for other engines: don't let raw user input define query structure; coerce types and validate.

## Injection: OS Command

Best rule: **don't call the shell at all.** Use language/library APIs (file ops, HTTP clients, native bindings) instead of shelling out. If you must run a subprocess, pass an **argument array** with the shell disabled, never an interpolated command string.

VULNERABLE — shell string with user input (`; rm -rf /`, `$(...)`, backticks, `&&`, `|`):
```js
exec(`convert ${userFile} out.png`);             // node: shell=true, injectable
os.system("ping -c 1 " + host)                   // python: shell, injectable
subprocess.run(f"git clone {url}", shell=True)   // shell=True + interpolation = RCE
```

SAFE — arg array, no shell:
```js
// node: execFile/spawn with array; shell NOT invoked
execFile('convert', [userFile, 'out.png']);
spawn('git', ['clone', '--', url]);              // '--' stops option injection

// python: list args, shell=False (default)
subprocess.run(["ping", "-c", "1", host], shell=False)
subprocess.run(["git", "clone", "--", url])

// Go
exec.Command("git", "clone", "--", url)          // never pass through "sh -c"
```

- Arg arrays still let a value *look like* a flag (`--upload-pack=...`, `-o ProxyCommand=...`). Use `--` to terminate options, and/or allowlist/validate values that could be flags.
- If a shell is truly unavoidable, allowlist input to `[A-Za-z0-9._/-]` and reject everything else — but prefer the array form. Quoting/escaping by hand is error-prone.
- This extends beyond `sh`: `eval`, `Function()`, `pickle.loads`, `yaml.load` (use `safe_load`), `child_process`, template engines with code eval, and deserializers are all "code injection" surfaces. Never feed them untrusted data.
- Path/argument context: a filename like `-rf` or `../../etc/passwd` is injection too — see Input Validation / path traversal.

## Injection: LDAP, XPath, headers, templates

- **LDAP**: special chars `( ) * \ NUL /` alter filters. Escape per RFC 4515 using your LDAP library's escape function; better, use parameterized search APIs. `(&(uid=*)(...))` lets `*` enumerate. Never concatenate raw DN/filter input.
  ```
  // VULNERABLE: filter = "(uid=" + user + ")"   → user = "*)(uid=*"
  // SAFE: ldap.escape(user) then build, or use a binding API
  ```
- **XPath injection**: same shape as SQLi (`' or '1'='1`). Use variable binding (`XPathExpression` with a resolver) instead of string-built XPath.
- **HTTP header / response splitting**: never put raw user input into response headers, `Location` redirects, `Set-Cookie`, or log lines without stripping CR/LF (`\r\n`). Use framework setters that reject control chars.
- **SMTP / email header injection**: strip CR/LF from any user-supplied value going into `To`, `Subject`, `From`, etc.
- **Server-side template injection (SSTI)**: never `render_template_string(user_input)` / build templates from user input (Jinja2, Twig, Handlebars, EJS). That's RCE. Pass user data as *template variables*, keep the template static.
- **Log injection**: user input in logs can forge log lines or inject into log viewers — strip newlines, and don't log raw HTML into a system that renders it.

## XSS: Cross-Site Scripting

XSS = attacker-controlled data is interpreted as code (HTML/JS) in a victim's browser. Defense: **contextual output encoding by default**, plus a strong CSP as backstop. The browser, not the server, executes it — so escaping at output time is what matters.

- **Use a framework that auto-escapes** (React `{value}`, Vue `{{ }}`, Angular interpolation, Django/Jinja2 autoescape ON, Rails ERB). These escape text-context HTML for you. Don't fight them.
- Escaping is **context-dependent**: HTML body, HTML attribute, JS string, URL, and CSS each need different encoding. A value safe in HTML text is unsafe inside `<script>` or an `href`.

VULNERABLE:
```jsx
// React: bypasses auto-escaping with raw HTML
<div dangerouslySetInnerHTML={{ __html: userBio }} />
```
```js
// vanilla: innerHTML with user data
el.innerHTML = "Hello " + username;               // <img src=x onerror=alert(1)>
document.write(location.hash);                      // DOM XSS
```
```html
<!-- server templates with escaping disabled -->
{{ user_comment | safe }}          {# Jinja2: 'safe' disables escaping #}
<%== user_comment %>               <%# ERB raw #>
{!! $comment !!}                   {{-- Blade unescaped --}}
```

SAFE:
```jsx
<div>{userBio}</div>                               // React escapes text
```
```js
el.textContent = "Hello " + username;              // textContent never parses HTML
el.setAttribute('title', username);                // attribute via API, not string-built markup
```
```html
{{ user_comment }}                 {# autoescaped #}
```

- If you genuinely must render user HTML (rich text), **sanitize with a vetted allowlist sanitizer** right before render: DOMPurify (`DOMPurify.sanitize(html)`), `sanitize-html`, OWASP Java HTML Sanitizer, Bleach (Python). Never write your own. Sanitize on output/render, and re-sanitize even if you also sanitized on input (transforms can re-introduce vectors).
- **URL context**: validate scheme before using user URLs in `href`/`src`. Block `javascript:`, `data:`, `vbscript:`. Allowlist `https:`/`http:`/`mailto:`.
  ```js
  const u = new URL(userUrl, base);
  if (!['http:', 'https:'].includes(u.protocol)) reject();
  ```
- **JS context**: never inject server data straight into a `<script>` block by string. Pass data via `JSON.stringify` into a `data-*` attribute or a JSON `<script type="application/json">`, and read it from JS. Beware `</script>` sequences — JSON-encode and escape `<`.
- **Content-Security-Policy** as defense-in-depth (stops injected inline script from running):
  ```
  Content-Security-Policy: default-src 'self'; script-src 'self' 'nonce-{random}';
    object-src 'none'; base-uri 'none'; frame-ancestors 'none'
  ```
  Avoid `'unsafe-inline'`/`'unsafe-eval'`; use per-request nonces or hashes. `frame-ancestors 'none'` also handles clickjacking (replaces `X-Frame-Options`).
- Set `HttpOnly` on session cookies so XSS can't read them via `document.cookie` (limits damage).
- React-specific gotchas: `href={userUrl}` can still be `javascript:` (validate scheme); `dangerouslySetInnerHTML`, `ref` + `innerHTML`, and third-party markdown renderers are the usual leaks.

## Authentication: Password Storage & Verification

- **Hash passwords with a slow, salted, memory-hard KDF.** Use **argon2id** (preferred), or **bcrypt**, or **scrypt**, or PBKDF2 (last resort, high iterations). Each generates and stores a unique salt automatically.
- **NEVER** use MD5, SHA-1, SHA-256/512 (raw), or any fast/general hash for passwords, and never store plaintext or reversible encryption. Fast hashes are brute-forced at billions/sec on GPUs.

VULNERABLE:
```js
const hash = crypto.createHash('sha256').update(password).digest('hex'); // fast, unsalted
if (user.password === inputPassword) { ... }                              // plaintext compare
md5(password)                                                              // catastrophic
```

SAFE:
```js
// node, argon2id
import argon2 from 'argon2';
const hash = await argon2.hash(password, { type: argon2.argon2id });
const ok = await argon2.verify(hash, password);   // verify embeds salt+params, constant-time-ish

// bcrypt (cost >= 12; note 72-byte input truncation — pre-hash long inputs if needed)
import bcrypt from 'bcryptjs';
const hash = await bcrypt.hash(password, 12);
const ok  = await bcrypt.compare(password, hash);
```
```python
# python, argon2-cffi
from argon2 import PasswordHasher
ph = PasswordHasher()
hash = ph.hash(password)
ph.verify(hash, password)   # raises on mismatch
```

- Suggested params: argon2id ~64MB memory, ≥3 iterations, parallelism ~1-4 (tune to ~250-500ms); bcrypt cost 12+; PBKDF2-HMAC-SHA256 ≥600k iterations. Increase over time.
- Enforce length over complexity: min 8-12 chars, allow long passphrases (cap ~64-128 to avoid DoS), allow all Unicode/spaces. Check against breached-password lists (HaveIBeenPwned k-anonymity API). Don't force arbitrary composition rules or frequent rotation.
- On login, **always run the verify even if the user doesn't exist** (compare against a dummy hash) so response timing doesn't reveal valid accounts.
- Store only the hash. Never log the password, never email it, never include it in API responses, exclude `passwordHash` from serializers by default.
- For API keys/tokens you generate: store a hash of them too (they're high-entropy, so a single SHA-256 is acceptable since brute force is infeasible), and compare in constant time.

## Authentication: Constant-Time Comparison

- Comparing secrets (tokens, HMACs, signatures, API keys, password-reset tokens) with `==`/`===`/`strcmp` leaks length and byte-by-byte match position via timing → attacker can forge byte by byte.

VULNERABLE:
```js
if (providedToken === storedToken) grant();        // early-exit, timing leak
if (hmac === expectedHmac) ...                      // signature bypass via timing
```

SAFE:
```js
import crypto from 'crypto';
const a = Buffer.from(providedToken), b = Buffer.from(storedToken);
// guard length first (timingSafeEqual throws on length mismatch)
const ok = a.length === b.length && crypto.timingSafeEqual(a, b);
```
```python
import hmac
ok = hmac.compare_digest(provided_token, stored_token)
```
```go
import "crypto/subtle"
ok := subtle.ConstantTimeCompare(a, b) == 1
```

- Prefer comparing fixed-length hashes (e.g. HMAC the inputs first) so length itself isn't a side channel.
- Use the library verify functions (`argon2.verify`, `bcrypt.compare`) rather than comparing hashes yourself.

## Authentication: Sessions & Tokens

- **Session IDs / tokens must be cryptographically random and high-entropy** (≥128 bits): `crypto.randomBytes(32)`, `secrets.token_urlsafe(32)`. Never use sequential IDs, timestamps, `Math.random()`, or guessable values.
- **Session cookies**: `HttpOnly` (no JS access), `Secure` (HTTPS only), `SameSite=Lax` or `Strict`, scoped `Path`, sensible `Max-Age`. Don't put session data *in* the cookie unsigned.
  ```
  Set-Cookie: sid=...; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=3600
  ```
- **Regenerate the session ID on privilege change** (login, role elevation) to prevent session fixation. Invalidate server-side on logout (don't just clear the cookie). Support "log out all sessions."
- Set absolute + idle timeouts. Bind sessions server-side so revocation works.
- **JWTs** (only if you need stateless):
  - Pin the algorithm; reject `alg: none` and never let the token choose the alg. Use a strong secret (HS256) or proper keys (RS/ES256).
  - Validate `exp`, `iss`, `aud`, signature — always, on every request. A library that "decodes" without verifying signature is a footgun (`jwt.decode` vs `jwt.verify`).
  - Keep access tokens short-lived (minutes); use rotating refresh tokens stored server-side and revocable. JWTs can't be easily revoked — that's their main downside; don't use them for "log out everywhere" without a server-side denylist/version.
  - Don't store secrets/PII in the (unencrypted, base64) payload. Anyone can read it.
  - Store tokens in `HttpOnly` cookies (then add CSRF defense) rather than `localStorage` (XSS-readable). Pick one trade-off deliberately.
- OAuth/OIDC: use a library, validate `state` (CSRF for the callback) and `nonce`, use PKCE for public clients, validate redirect URIs against an exact allowlist.
- Email verification / password reset tokens: single-use, short TTL, high-entropy, invalidated after use or password change. Compare in constant time.

## Authentication: Account Enumeration & Brute Force

- **Account enumeration**: don't reveal whether a username/email exists via login errors, signup ("already registered"), password reset, or response timing.
  - Login: same generic message + similar timing for "no such user" and "wrong password": *"Invalid email or password."* (Run the password verify against a dummy hash when the user is missing.)
  - Password reset / signup: respond identically whether or not the account exists (*"If an account exists, we've sent an email"*). Send mail out-of-band.
- **Brute-force / credential-stuffing protection** (layered):
  - Rate-limit auth endpoints per IP **and** per account (e.g. token bucket). Add exponential backoff.
  - Temporary lockout or step-up (CAPTCHA / MFA challenge) after N failures; auto-unlock after a cooldown to avoid lockout-DoS.
  - Detect distributed attempts (many accounts from one IP/ASN, one account from many IPs).
  - Offer/encourage **MFA**; it defeats most credential stuffing.
  - Watch other brute-forceable endpoints too: OTP/2FA codes (rate-limit + few attempts + short TTL), reset tokens, coupon/gift-card codes, API keys.
- Log auth failures and lockouts (without logging the attempted password). Alert on spikes.

VULNERABLE:
```js
if (!user) return res.status(404).json({ error: 'No account with that email' });
if (!passwordOk) return res.status(401).json({ error: 'Wrong password' });
// → enumerate users + unlimited guesses
```
SAFE:
```js
await limiter.consume(ip + ':' + emailKey);          // throws/429 when exceeded
const user = await findUser(email);
const ok = await verify(user?.passwordHash ?? DUMMY_HASH, password); // constant work
if (!user || !ok) return res.status(401).json({ error: 'Invalid email or password' });
```

## Authorization: Check on Every Request (IDOR is the #1 bug)

**Every request that touches data must verify, server-side, that the authenticated caller is allowed to perform that action on that specific resource.** Authentication ("who are you") is not authorization ("are you allowed"). This is the most common serious real-world vuln.

- **IDOR / BOLA (Broken Object Level Authorization)**: never fetch a resource by an ID from the request and return/mutate it without checking ownership/permission. A logged-in user changing `/orders/1001` to `/orders/1002` must not see someone else's order.

VULNERABLE:
```js
app.get('/api/orders/:id', auth, async (req, res) => {
  const order = await db.orders.findById(req.params.id);   // no ownership check!
  res.json(order);                                         // any user reads any order
});
app.post('/api/users/:id/email', auth, async (req, res) => {
  await db.users.update(req.params.id, { email: req.body.email }); // edit anyone
});
```

SAFE — scope the query to the caller, or check ownership explicitly:
```js
app.get('/api/orders/:id', auth, async (req, res) => {
  // scope by owner in the query — best: not-yours is indistinguishable from not-found
  const order = await db.orders.findOne({ id: req.params.id, userId: req.user.id });
  if (!order) return res.sendStatus(404);
  res.json(order);
});

// or explicit check
const doc = await db.docs.findById(id);
if (!doc) return res.sendStatus(404);
if (doc.ownerId !== req.user.id && !req.user.isAdmin) return res.sendStatus(403);
```

- **Prefer scoping queries by the owner** (`WHERE user_id = :caller`) over fetch-then-compare: it's harder to forget and avoids leaking existence. Return `404` (not `403`) for not-owned to avoid confirming the resource exists.
- **Check authz at the function/data layer, not just in route middleware or the UI.** A shared service method called from multiple places must enforce its own checks. Don't rely on "this endpoint is only linked from the admin page."
- **Function-level / Broken Function Level Authorization**: admin/privileged endpoints must verify role/permission server-side — not just be hidden in the UI. Attackers hit `/api/admin/...` directly. Deny by default; allowlist who can call what.
- **Mass assignment / over-posting**: don't bind request bodies straight to models — a user can set `isAdmin`, `role`, `balance`, `ownerId`. Use an explicit allowlist of writable fields (DTO/serializer/`pick`), never `Object.assign(model, req.body)` or `Model(**req.body)`.
  ```js
  const { name, bio } = req.body;                 // allowlist
  await db.users.update(req.user.id, { name, bio });   // never spread req.body
  ```
- Enforce authz on **every** verb and nested route: list, read, create, update, delete, and bulk/GraphQL/`include` expansions. GraphQL: check per-field/per-node, not just at the root.
- **Principle of least privilege** everywhere: each user/role/service/token gets the minimum scope. DB user for the app shouldn't be superuser/owner. API tokens scoped to specific actions. Background jobs run as constrained identities. Cloud IAM roles narrowly scoped (no `*:*`).
- Centralize policy (a single `can(user, action, resource)` / policy objects / RBAC-ABAC layer) so checks are consistent and auditable, not copy-pasted and drifting.
- Re-validate authorization on **state-changing** multi-step flows (don't trust a "step 1 said OK" token the client holds).

## Authorization: Server-Side Trust Boundary

- The client sends **requests**, not **decisions**. Prices, totals, discounts, user IDs, roles, quotas, "isAdmin", feature flags that gate access — recompute/verify server-side. Never trust amounts or entitlements posted by the client.
  ```js
  // VULNERABLE: trust client total
  charge(req.body.totalCents);
  // SAFE: recompute from server-side prices
  const total = items.reduce((s, i) => s + price(i.sku) * qty(i), 0);
  charge(total);
  ```
- Hidden form fields, JWT claims set by the client, `X-User-Id` headers, and query params are all attacker-controlled. Derive identity from the verified session/token, not from a header the client supplies.
- Disabled/absent UI controls are not access control. Assume every endpoint is called directly with arbitrary params.

## Secrets Management

- **Never hardcode secrets** (API keys, DB passwords, JWT/signing keys, private keys, OAuth client secrets, encryption keys) in source. Never commit them. Once committed, consider them compromised — **rotate**, don't just delete (git history keeps them).

VULNERABLE:
```js
const stripe = new Stripe('sk_live_51H...real_key');   // in repo forever
const DB_URL = 'hardcoded database URL';
```
SAFE:
```js
const stripe = new Stripe(process.env.STRIPE_SECRET_KEY);
if (!process.env.STRIPE_SECRET_KEY) throw new Error('STRIPE_SECRET_KEY not set');
const DB_URL = process.env.DATABASE_URL;
```

- Load from environment / a secrets manager (Vault, AWS/GCP Secrets Manager, Doppler, SOPS-encrypted files). In cloud, prefer workload identity / IAM roles over long-lived static keys.
- Keep secrets out of the repo: `.env` in `.gitignore`, commit a `.env.example` with **placeholder** values only. Add a pre-commit secret scanner (gitleaks, trufflehog, `detect-secrets`).
- **Don't log secrets, tokens, passwords, full card numbers, or PII.** Redact `Authorization` headers, cookies, query strings with tokens, and request bodies before logging. Watch for secrets leaking into error trackers, APM, analytics, and stack traces.
  ```js
  logger.info('charge', { userId, amount });                 // OK
  logger.info('req', { headers: req.headers });              // LEAKS Authorization/Cookie
  ```
- Don't expose secrets to the frontend. Anything in client JS / `NEXT_PUBLIC_*` / mobile app binaries is public. Server-side only for real secrets.
- Rotate on a schedule and immediately on suspected exposure or staff offboarding. Prefer short-lived credentials. Scope each secret to least privilege. Use distinct secrets per environment (dev/staging/prod).
- Encrypt secrets at rest; restrict who/what can read them. Don't pass secrets on the command line (visible in `ps`/shell history) — use env or files.

## CSRF: Cross-Site Request Forgery

CSRF matters when the browser **auto-attaches credentials** — i.e. **cookie/session-based auth** (also HTTP Basic, client certs). A malicious site triggers a state-changing request to your app and the browser sends the victim's cookies. Token-in-`Authorization`-header APIs are largely immune (the attacker site can't read/set that header cross-origin), so the highest risk is cookie-authenticated form/AJAX endpoints.

- **Primary defense: `SameSite` cookies.** `SameSite=Lax` (default in modern browsers) blocks cookies on cross-site POST/PUT/DELETE; `Strict` is stronger but breaks top-level inbound links. Set it explicitly; don't rely on browser defaults alone.
- **Add anti-CSRF tokens** for cookie-auth state-changing requests (defense in depth, and covers older browsers / same-site subdomain risks):
  - **Synchronizer token**: server issues a random token tied to the session, embedded in the form/page; verified on POST.
  - **Double-submit cookie**: token in a cookie + mirrored in a header/body, server checks they match (sign it to prevent subdomain tampering).
  ```html
  <form method="post" action="/transfer">
    <input type="hidden" name="_csrf" value="{{ csrfToken }}">
  </form>
  ```
  ```js
  // server verifies header/body token matches session token, constant-time
  if (!safeEqual(req.body._csrf, req.session.csrfToken)) return res.sendStatus(403);
  ```
- Use your framework's built-in CSRF protection (Django, Rails, Spring Security, Laravel, `csurf`-style middleware). Don't roll your own.
- **GET/HEAD must be side-effect-free.** Never mutate state on GET — links/images can trigger them and CSRF tokens don't apply.
- For SPA + cookie auth: send a custom header (e.g. `X-CSRF-Token`) the server requires; cross-site forms can't add custom headers, and ensure CORS doesn't blanket-allow it.
- For JSON APIs, requiring `Content-Type: application/json` + a custom header raises the bar (simple cross-site form posts can't set those), but still pair with SameSite.
- Verify `Origin`/`Referer` on sensitive POSTs as an extra check.
- After login, **rotate the session/CSRF token**.

## Input Validation & SSRF

**Validate all input against an allowlist of what's expected** (type, length, format, range, set) and reject the rest. Blocklists miss cases. Validate, then escape/encode for the destination context (the two are different layers).

- Define a schema at every boundary (zod/Joi/ajv, Pydantic, Bean Validation, struct tags) — types, required fields, bounds, enums, regex with anchors and length caps. Reject unknown fields (prevents mass assignment).
- Validate semantics, not just type: email format, positive quantity, allowed enum, ID belongs to caller (authz), file size/type limits.
- Numeric: enforce ranges; beware integer overflow and negative values (negative price/quantity → free money). Money in integer minor units, not floats.
- Strings: cap length (DoS), normalize Unicode where relevant, watch for null bytes and control chars.
- **Regex DoS (ReDoS)**: avoid catastrophic backtracking (nested quantifiers like `(a+)+`). Use linear-time engines (RE2) or anchored, simple patterns; bound input length; timeout matching.
- **Path traversal**: never join user input into a filesystem path without containment. Strip/reject `..`, absolute paths, and after resolving, assert the path stays under the intended base dir.
  ```js
  const base = '/var/app/uploads';
  const p = path.resolve(base, req.params.name);
  if (p !== base && !p.startsWith(base + path.sep)) return res.sendStatus(400);
  ```
  Prefer mapping user input to an ID → known safe path, rather than using user-supplied filenames.
- **File uploads**: validate type by content/magic bytes (not just extension/Content-Type), cap size, generate your own storage filename, store outside webroot or in object storage, set non-executable perms, never serve uploads from a path where they can run as code, scan if feasible. Strip/ignore client path in the filename.
- **Open redirect**: don't redirect to a user-supplied URL unless it's a relative path or on an allowlist of hosts. Attackers use it for phishing and to bounce OAuth tokens.
  ```js
  // SAFE: only allow same-site relative redirects
  const to = req.query.next;
  if (!to || !to.startsWith('/') || to.startsWith('//')) return res.redirect('/');
  res.redirect(to);
  ```

**SSRF (Server-Side Request Forgery)**: when your server fetches a user-supplied URL (webhooks, image/URL preview, importers, PDF render, proxy), an attacker can target your internal network or cloud metadata.

VULNERABLE:
```js
const r = await fetch(req.body.url);     // user controls url → http://169.254.169.254/...
```
SAFE — allowlist + block internal ranges, resolve-then-pin, no redirects to internal:
```js
const u = new URL(req.body.url);
if (!['http:', 'https:'].includes(u.protocol)) reject();      // no file:, gopher:, ftp:, dict:
// resolve DNS, then check EVERY resolved IP before connecting (defeats DNS rebinding)
const addrs = await dns.lookup(u.hostname, { all: true });
for (const { address } of addrs) if (isPrivate(address)) reject();
// connect to the validated IP (pin it), disable redirects, set timeouts & size cap
```
- Block (for outbound user-driven fetches): `127.0.0.0/8`, `::1`, `10/8`, `172.16/12`, `192.168/16`, `169.254/16` (incl. **cloud metadata `169.254.169.254`** / `fd00:ec2::254`), `0.0.0.0`, link-local, ULA `fc00::/7`, and DNS names resolving to them.
- **Prefer an allowlist** of permitted hosts/domains over a blocklist.
- **Defeat DNS rebinding**: resolve, validate the IP, then connect to that exact IP (don't re-resolve). Re-check on each redirect; ideally disable redirects.
- Restrict schemes to http/https; block `file://`, `gopher://`, `ftp://`, `dict://`, `data:`.
- Set timeouts and response-size limits; don't reflect the fetched body verbatim (can leak internal responses).
- Network-layer backstop: run the fetcher with egress firewall rules / no route to metadata and internal subnets; use IMDSv2 (token-required) on AWS.

## Transport & Data Protection

- **HTTPS/TLS everywhere.** Redirect HTTP→HTTPS; send **HSTS** (`Strict-Transport-Security: max-age=63072000; includeSubDomains; preload`). No mixed content. Verify certs on outbound calls (never disable TLS verification / `rejectUnauthorized:false` / `verify=False` in prod).
- **Cookies**: `Secure` + `HttpOnly` + `SameSite` on session/auth cookies (recap from Sessions/CSRF). Don't store sensitive data in non-HttpOnly cookies or `localStorage`.
- **Encrypt sensitive data at rest**: full-disk/volume encryption, plus app-level encryption (AEAD: AES-GCM or libsodium/`crypto_secretbox`) for highly sensitive fields. Use a KMS for key management and key rotation; never hardcode keys (see Secrets).
- **Crypto hygiene**: use AEAD; never ECB; use random unique IV/nonce per message; never reuse a nonce with the same key; use HKDF for key derivation; use a CSPRNG (`crypto.randomBytes`, `secrets`), never `Math.random()`/`rand()` for tokens/keys/IVs. Don't invent protocols — use libsodium/your platform's vetted primitives.
- **Don't leak data in errors/responses**: no stack traces, SQL text, file paths, framework versions, or internal hostnames to clients. Return generic messages + a correlation ID; log details server-side. Disable debug mode in prod. Set `Server`/`X-Powered-By` to nothing useful.
  ```js
  // VULNERABLE
  res.status(500).json({ error: err.stack });
  // SAFE
  logger.error({ err, reqId });
  res.status(500).json({ error: 'Internal error', reqId });
  ```
- **PII / sensitive data handling**: collect the minimum; define retention and delete on schedule; mask in UI/logs (show last 4 only); restrict access (least privilege) and audit it; never put PII/secrets in URLs (they hit logs, history, Referer); honor deletion/export requests (GDPR/CCPA); encrypt in transit and at rest. Never log full card numbers (PCI), SSNs, tokens, or passwords. Tokenize card data; offload to a PCI-compliant processor rather than storing PAN.
- **Security headers** (set globally; `helmet`/equivalent): `Content-Security-Policy`, `Strict-Transport-Security`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer` or `strict-origin-when-cross-origin`, `Cache-Control: no-store` for sensitive responses, `Permissions-Policy` to drop unneeded features, `frame-ancestors 'none'` (or `X-Frame-Options: DENY`) against clickjacking.
- **CORS**: don't reflect arbitrary `Origin` or use `Access-Control-Allow-Origin: *` with credentials. Allowlist exact origins; never combine `*` with `Allow-Credentials: true`. Keep allowed methods/headers minimal.

## Dependencies & Supply Chain

- **Pin and lock** dependencies: commit `package-lock.json` / `yarn.lock` / `poetry.lock` / `Cargo.lock` / `go.sum`; install with `npm ci` (not `npm install`) in CI for reproducible builds.
- **Audit regularly**: `npm audit` / `pip-audit` / `cargo audit` / `govulncheck`; enable Dependabot/Renovate + secret/SCA scanning. Patch known-vuln transitive deps. Don't ignore high/critical advisories.
- **Vet before adding**: prefer widely-used, maintained packages; check for typosquats (`expresss`, `loadsh`, `colour`), recent ownership changes, and suspicious install scripts. Fewer dependencies = smaller attack surface.
- **Don't auto-run untrusted code/scripts**: be wary of postinstall scripts (`npm ci --ignore-scripts` where viable), `curl | bash`, and copy-pasted snippets. Never `eval`/import code fetched at runtime from untrusted sources.
- **Integrity**: use lockfile hashes / Subresource Integrity (SRI) for CDN `<script>`/`<link>`; verify checksums/signatures of downloaded binaries; prefer signed releases.
- **Build/CI security**: least-privilege CI tokens, don't echo secrets in logs, protect the pipeline (it can push to prod). Beware compromised actions/plugins; pin third-party CI actions to a commit SHA, not a mutable tag.
- Keep runtimes/base images patched; scan container images; prefer minimal/distroless bases. Remove dev tooling and secrets from production images.
- Track your dependencies (SBOM) so you can respond fast when a new CVE drops.

## OWASP Top 10 (2021) Quick Reference

| # | Category | Watch for | Primary fix |
|---|----------|-----------|-------------|
| A01 | Broken Access Control | IDOR/BOLA, missing function-level checks, path traversal, mass assignment, CORS | Server-side authz on every request; scope queries to caller; deny by default; least privilege |
| A02 | Cryptographic Failures | Plaintext/weak hashes, no TLS, hardcoded keys, weak RNG, ECB, leaked PII | TLS+HSTS; argon2id/bcrypt; AEAD; CSPRNG; KMS; encrypt at rest |
| A03 | Injection | SQL/NoSQL/command/LDAP/XPath/SSTI, XSS | Parameterize; arg arrays no-shell; contextual output encoding; allowlist validate |
| A04 | Insecure Design | Missing rate limits, no threat model, business-logic abuse (negative qty, race) | Threat-model; secure defaults; abuse-case tests; limits/quotas |
| A05 | Security Misconfiguration | Debug on, default creds, verbose errors, open buckets, missing headers, dir listing | Hardened defaults; least functionality; security headers; config review |
| A06 | Vulnerable & Outdated Components | Unpatched deps, no inventory, EOL runtimes | Pin+lock; audit/SCA; patch; SBOM |
| A07 | Identification & Auth Failures | Weak passwords, no MFA, session fixation, enumeration, no brute-force protection | Strong KDF; MFA; rotate session on login; rate-limit/lockout; generic errors |
| A08 | Software & Data Integrity Failures | Unsigned updates, insecure deserialization, unverified CI plugins | Verify signatures/SRI; safe deserializers; protect pipeline; pin actions |
| A09 | Logging & Monitoring Failures | No audit logs, no alerting, logging secrets/PII | Log security events (redacted); alert; retain; monitor |
| A10 | SSRF | Server fetches user URLs to internal/metadata | Allowlist hosts; block private IPs/metadata; pin resolved IP; no internal redirects |

(API-specific: see OWASP API Security Top 10 — BOLA/A01, broken auth, BOPLA/property-level authz, unrestricted resource consumption, broken function-level authz.)

## Common Amateur Mistakes (anti-pattern checklist)

Fast scan before shipping — each maps to a section above.

- **Trusting client-side validation only** → re-validate and re-authorize on the server; the JS check is UX only.
- **No authorization check / IDOR** → on every request, verify the resource belongs to the caller; scope queries by owner; deny by default. (The single most common serious bug.)
- **Plaintext or weak password hashing** (md5/sha/unsalted/`==`) → argon2id/bcrypt + library verify + constant-time compares.
- **String-concatenated SQL / building queries from input** → parameterized queries; allowlist identifiers/sort columns.
- **Shelling out with interpolated strings** → arg arrays, no shell, `--` to stop option injection; better, don't shell out.
- **Reflecting user input unescaped** → contextual output encoding by default; sanitize HTML with DOMPurify; never `dangerouslySetInnerHTML` with user data; CSP backstop.
- **Secrets in code/logs/responses/`localStorage`/client bundles** → env/secret manager; redact logs; rotate on exposure; server-side only.
- **No rate limiting / lockout on auth** (login, OTP, reset) → per-IP + per-account limits, backoff, lockout/CAPTCHA, MFA.
- **Account enumeration** via distinct errors/timing → generic messages, constant-time work, out-of-band reset email.
- **Verbose error leaks** (stack traces, SQL, paths, versions, debug mode in prod) → generic message + correlation ID; log server-side.
- **Mass assignment** (`Object.assign(model, req.body)`, `Model(**body)`) → explicit writable-field allowlist.
- **Trusting client-supplied amounts/IDs/roles/headers** (`X-User-Id`, prices, `isAdmin`) → derive from verified session; recompute server-side.
- **SSRF**: fetching user URLs without allowlist / blocking metadata IP → allowlist + private-IP block + pin resolved IP.
- **Disabling TLS verification** (`verify=False`, `rejectUnauthorized:false`) in prod → never; fix the cert chain instead.
- **State-changing GET requests** → use POST/PUT/DELETE; keep GET side-effect-free (CSRF/caching).
- **Unsafe deserialization / `eval` / `yaml.load` / `pickle` on untrusted data** → safe loaders; never eval untrusted input.
- **Missing `Secure`/`HttpOnly`/`SameSite` on auth cookies** → set all three.
- **`Access-Control-Allow-Origin: *` with credentials, or reflecting Origin** → exact-origin allowlist.
- **Logging tokens/passwords/PII** → redact before logging; keep out of URLs.
- **Ignoring dependency advisories / unpinned deps** → lockfiles, `npm ci`, audit, patch.

### Pre-ship security checklist
1. Is every data-access path authorized server-side, scoped to the caller (IDOR)?
2. Are all queries parameterized and all subprocess calls shell-free?
3. Is all output contextually encoded; any user HTML sanitized; CSP set?
4. Passwords via argon2id/bcrypt; secrets compared constant-time?
5. Auth endpoints rate-limited; errors generic (no enumeration)?
6. Any secrets in code/logs/responses? Any debug/verbose errors exposed?
7. User-supplied URLs allowlisted and internal IPs/metadata blocked (SSRF)?
8. Cookies `HttpOnly`+`Secure`+`SameSite`; CSRF handled for cookie auth?
9. Inputs schema-validated/allowlisted; mass assignment prevented?
10. TLS enforced + HSTS; deps pinned and audited?
