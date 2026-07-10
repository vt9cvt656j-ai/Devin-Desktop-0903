# DevOps: Deploy & Operations

Battle-tested cheat-sheet for shipping and running apps. Each `##` section is self-contained: read only the one you need. Snippets are copy-paste starting points, not gospel — adapt to the stack. The golden rule throughout: **fail fast, small blast radius, no secrets in the wrong place, never break running traffic.**

---

## Docker: Production Images

Goal: small, reproducible, secure images that build fast and leak nothing.

**Multi-stage builds** — compile/install in a fat builder stage, copy only artifacts into a slim runtime. This drops a 1.2 GB image to ~150 MB.

```dockerfile
# ---- builder ----
FROM node:20.11.1-bookworm-slim AS builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci                      # full deps incl. devDependencies for build
COPY . .
RUN npm run build               # produces /app/dist

# ---- runtime ----
FROM node:20.11.1-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production
COPY package.json package-lock.json ./
RUN npm ci --omit=dev && npm cache clean --force
COPY --from=builder /app/dist ./dist
USER node                       # non-root (see below)
EXPOSE 3000
CMD ["node", "dist/server.js"]
```

**Layer caching order — copy dependency manifests FIRST.** Docker caches each layer; a layer rebuilds only if it or an earlier layer changed. `COPY package.json` + `RUN npm ci` *before* `COPY . .` means dependency install is cached and skipped unless the manifest changes. Copy source first and every code edit re-runs a full install (slow CI, slow local). Same pattern: `requirements.txt`/`poetry.lock` (Python), `go.mod`/`go.sum` (Go), `Cargo.toml`/`Cargo.lock` (Rust), `pom.xml` (Java).

**.dockerignore — always have one.** Without it, `COPY . .` ships `.git`, `node_modules`, secrets, and bloats the build context (slow uploads to the daemon). Minimum:

```
.git
.gitignore
node_modules
npm-debug.log
.env
.env.*
*.md
dist
build
coverage
.vscode
.idea
Dockerfile
.dockerignore
```

**Pin base image versions — never `FROM node:latest`.** `latest` is a moving target: a rebuild months later silently pulls a new major version and breaks you. Pin to a specific tag (`node:20.11.1-bookworm-slim`); for true reproducibility pin the digest (`node:20.11.1-bookworm-slim@sha256:...`). Prefer `-slim` or `-alpine` (smaller attack surface) but know alpine uses musl libc — some native deps misbehave; `distroless` is even smaller but harder to debug (no shell).

**Run as non-root.** Containers run as root by default; a container escape then has root on shared kernel resources. Create or use an unprivileged user.

```dockerfile
# debian/ubuntu base
RUN groupadd -r app && useradd -r -g app app
USER app
# alpine
RUN addgroup -S app && adduser -S app -G app
USER app
```

Many official images ship a user already (`node` → uid 1000). Files copied in are root-owned, so `chown` what the app must write, or use `COPY --chown=app:app`.

**EXPOSE / CMD vs ENTRYPOINT.**
- `EXPOSE 3000` is documentation/metadata only — it does NOT publish the port. You still need `-p 3000:3000` or compose `ports:`.
- `CMD` = the default command, easily overridden at `docker run image somethingelse`.
- `ENTRYPOINT` = the fixed executable; `CMD` becomes its default args. Use `ENTRYPOINT` when the image *is* one tool and you want args appended; use plain `CMD` for app servers. Use **exec form** (`CMD ["node","app.js"]`, JSON array) not shell form (`CMD node app.js`) — shell form wraps in `/bin/sh -c`, which does NOT forward signals, so SIGTERM never reaches your process and graceful shutdown breaks (see Deployment Strategies).
- For PID-1 signal/zombie reaping issues, add `--init` or `tini` as entrypoint.

**Never bake secrets into images.** Image layers are immutable and inspectable: `docker history`, `docker save | tar -x`, or pulling from the registry reveals any `ENV API_KEY=...`, copied `.env`, or secret used in a `RUN`. Even if a later layer deletes the file, the earlier layer still contains it. Inject secrets at **runtime** via env vars / mounted files / orchestrator secrets. For build-time secrets (private package registry token) use BuildKit `--mount=type=secret` so it never lands in a layer:

```dockerfile
# syntax=docker/dockerfile:1
RUN --mount=type=secret,id=npmtoken \
    NPM_TOKEN=$(cat /run/secrets/npmtoken) npm ci
```

**COMMON PITFALLS**
- `FROM ...:latest` everywhere → non-reproducible builds that rot.
- No `.dockerignore` → secrets and `node_modules` baked in, huge context.
- Copying source before deps → cache busted on every code change.
- Running as root → privilege escalation surface.
- `ENV SECRET=...` or `COPY .env` → secret permanently in image history.
- Single giant stage → 1 GB+ image with compilers and dev deps shipped to prod.
- `apt-get install` without `&& rm -rf /var/lib/apt/lists/*` in the same RUN → cache bloat.
- Shell-form `CMD` → signals not forwarded, no graceful shutdown.

---

## docker-compose: Local & Small Deployments

Compose is for local dev and small single-host deployments. For real multi-host orchestration use Kubernetes/Nomad/ECS — but compose patterns still teach the right defaults.

```yaml
services:
  app:
    build: .
    image: myapp:${TAG:-dev}
    env_file: .env                 # load non-secret config from file
    environment:
      - DATABASE_URL=postgres://app:${DB_PASSWORD}@db:5432/app
    ports:
      - "3000:3000"
    depends_on:
      db:
        condition: service_healthy # wait for DB to be HEALTHY, not just started
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/healthz"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 20s            # grace window before failures count

  db:
    image: postgres:16.2-bookworm
    environment:
      POSTGRES_USER: app
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: app
    volumes:
      - pgdata:/var/lib/postgresql/data   # NAMED volume = data survives recreate
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U app"]
      interval: 10s
      timeout: 5s
      retries: 5

volumes:
  pgdata:
```

**Service deps & startup ordering.** `depends_on` alone only waits for the container to *start*, not to be *ready*. Use `condition: service_healthy` with a healthcheck on the dependency, or make your app retry the DB connection on boot (more robust — never assume ordering, networks flap).

**Healthchecks** let compose/orchestrators know a container is actually serving, and gate dependent services. Hit a real endpoint (`/healthz`) or a real check (`pg_isready`), not just `true`. `start_period` prevents slow-booting apps from being marked unhealthy during startup.

**Named volumes for persistent data.** `pgdata:/var/lib/postgresql/data` survives `docker compose down` and `up`. A bind mount (`./data:/var/lib/...`) ties data to host paths and causes permission/uid mismatches. An *anonymous* volume or no volume = **data lost on container recreation** — the classic "my database reset itself" bug. NEVER store a production DB's data only inside the container layer.

**env_file vs environment.** `env_file: .env` bulk-loads key=values; `environment:` sets/overrides specific ones and can interpolate (`${DB_PASSWORD}`). Keep `.env` out of git (see Environment Config). For secrets prefer Docker/Swarm `secrets:` (mounted files) over env when you can.

**Restart policies.** `restart: unless-stopped` (restart on crash and on daemon start, but respect a manual stop) is the sane default for services. `always` ignores manual stops; `on-failure` restarts only on non-zero exit; `no` never restarts. Without a policy a crashed container stays dead.

**COMMON PITFALLS**
- No volume / anonymous volume on a database → silent data loss on recreate.
- `depends_on` without `service_healthy` → app starts before DB is ready, crashes.
- Secrets committed in `.env` tracked by git.
- No `restart:` policy → one crash and the service is down until you notice.
- Using compose as your production orchestrator across many hosts (no scheduling, no rolling updates).
- Hardcoding host ports that collide on shared hosts.

---

## Environment Config: 12-Factor

**Config lives in the environment, not in code.** Anything that differs between dev/staging/prod — DB URLs, credentials, feature flags, log level, external endpoints — is config and belongs in env vars (or a secrets manager), never hardcoded or committed.

**Strict separation of config from code.** The same built artifact must run in every environment with only env vars changing (see CI/CD "build once, promote"). A litmus test: could you open-source the repo right now without leaking a single credential? If not, secrets are in the code.

**Never commit `.env`.** Add to `.gitignore` on day one. Commit a `.env.example` with keys and dummy/empty values as documentation. If a secret was ever committed, it is compromised — **rotate it**; deleting the file does not remove it from git history (`git log -p`, forks, mirrors keep it).

```
# .gitignore
.env
.env.*
!.env.example
```

```bash
# .env.example  (committed — documents required config, NO real values)
DATABASE_URL=
REDIS_URL=
JWT_SECRET=
LOG_LEVEL=info
PORT=3000
```

**Fail fast on missing/invalid required config.** Validate at startup and **crash immediately** with a clear message if a required var is missing — do not boot half-configured and fail mysteriously on the first request (or worse, silently fall back to a dev default in prod). Centralize parsing/validation in one module.

```js
// config.js — single source of truth, validated once at boot
function required(name) {
  const v = process.env[name];
  if (v === undefined || v === "") {
    throw new Error(`Missing required env var: ${name}`);
  }
  return v;
}
module.exports = {
  databaseUrl: required("DATABASE_URL"),
  jwtSecret:   required("JWT_SECRET"),
  port:        Number(process.env.PORT ?? 3000),   // sane default for non-secret
  logLevel:    process.env.LOG_LEVEL ?? "info",
  env:         process.env.NODE_ENV ?? "development",
};
```

**Sane defaults — but only for non-sensitive, non-environment-specific values.** Default `PORT=3000`, `LOG_LEVEL=info`: fine. Defaulting `JWT_SECRET` to `"changeme"` or a DB password to anything: dangerous — the default ships to prod and becomes a backdoor. Secrets must have **no** default; their absence must crash the app.

**Separate dev/staging/prod cleanly.** Distinct credentials, distinct datastores, distinct external API keys per environment. Staging should mirror prod's topology as closely as budget allows so deploys are tested realistically. Never let a dev/test run point at the production database.

**COMMON PITFALLS**
- `.env` committed (or in Docker image) → credential leak.
- Secret with a hardcoded default fallback → silent insecure prod.
- Booting with missing config and failing later at request time → hard-to-trace outages.
- Config scattered across files via raw `process.env.X` reads → impossible to audit what's required.
- Same API keys/DB across dev and prod → a dev mistake nukes prod data.
- "We'll rotate the leaked key later" → it's already scraped; rotate now.

---

## CI/CD Pipelines

Every push runs the same gates; the pipeline is the source of truth for "is this shippable."

**Run lint + tests + build on every push and PR. Fail the pipeline on any error.** A red build must block merge/deploy — a pipeline that goes green while tests fail is worse than no pipeline (false confidence). Set `exit 1` semantics; don't `|| true` away failures.

**Cache dependencies** to keep pipelines fast (cache keyed on the lockfile hash so it invalidates correctly).

**Build the artifact ONCE, then promote the *same* artifact across environments.** Build the Docker image (or bundle) a single time, tag it with the immutable commit SHA, push to a registry, and deploy that exact digest to staging → prod. Rebuilding per environment means prod runs a different binary than the one you tested in staging — non-reproducible and a classic source of "worked in staging." Promote by re-tagging/re-deploying the tested digest, never by rebuilding.

**Tag images by commit SHA (and optionally semver), not `latest`.** Immutable tags make rollbacks trivial (`deploy myapp:9f3a1c`) and auditable.

Minimal GitHub Actions workflow (test/lint/build on every push; build-and-push image once on main):

```yaml
name: ci
on:
  push:
    branches: [main]
  pull_request:

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: "20.11.1"
          cache: "npm"               # caches ~/.npm keyed on package-lock.json
      - run: npm ci
      - run: npm run lint
      - run: npm test                # non-zero exit fails the job → blocks merge
      - run: npm run build

  build-image:
    needs: test                      # only if tests pass
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v6
        with:
          push: true
          tags: ghcr.io/${{ github.repository }}:${{ github.sha }}
          cache-from: type=gha       # BuildKit layer cache in Actions
          cache-to: type=gha,mode=max
```

**Secrets in CI** come from the platform's secret store (`${{ secrets.X }}`), masked in logs — never hardcoded in the workflow YAML, never echoed. Scope tokens to least privilege.

**COMMON PITFALLS**
- Pipeline green while tests fail (`|| true`, ignored exit codes, no required checks) → broken code merges.
- Rebuilding the image for each environment → prod ≠ what you tested.
- `latest` tags → can't roll back, can't tell what's deployed.
- No dependency cache → slow pipelines, devs start skipping CI.
- Secrets pasted into workflow files or printed in logs.
- Running deploy before tests, or deploying on red.
- No lint/format gate → style churn and avoidable bugs slip through.

---

## Deployment Strategies & Migrations

Goal: ship without dropping a single request, and evolve the schema without breaking the running version.

**Zero-downtime: rolling or blue-green.**
- *Rolling*: replace instances a few at a time behind a load balancer; new instances must pass health checks before old ones drain and stop. Needs N+1 capacity and backward/forward-compatible changes (old and new run simultaneously).
- *Blue-green*: stand up the full new version (green) alongside current (blue), health-check it, then flip the LB/router to green; keep blue warm for instant rollback. More resources, simplest rollback.
- *Canary*: route a small % of traffic to the new version, watch error/latency metrics, then ramp. Best risk control; needs good observability.

**Health checks gate every rollout.** Distinguish:
- **Liveness** — "is the process wedged?" Restart if it fails. Keep it cheap and dependency-free (don't fail liveness because the DB is down — that just causes restart storms).
- **Readiness** — "can it serve traffic right now?" If it fails, the LB removes the instance but does NOT kill it (e.g. still warming caches, dependency briefly unavailable). Roll-out waits for readiness before sending traffic.

**Graceful shutdown — handle SIGTERM and drain.** On deploy/scale-down the orchestrator sends `SIGTERM`, waits a grace period, then `SIGKILL`. Your app must: stop accepting new connections, fail readiness (so the LB stops routing), finish in-flight requests, close DB/queue connections, then exit. No handler = in-flight requests dropped on every deploy.

```js
const server = app.listen(port);
let shuttingDown = false;

function shutdown(signal) {
  if (shuttingDown) return;
  shuttingDown = true;
  console.log(`${signal} received, draining...`);
  // 1) fail readiness so LB stops sending new traffic
  // 2) stop accepting, let in-flight finish
  server.close(async () => {
    await db.end();        // close pools/queues
    process.exit(0);
  });
  // hard cap so a stuck request can't block SIGKILL forever
  setTimeout(() => process.exit(1), 25_000).unref();
}
process.on("SIGTERM", () => shutdown("SIGTERM"));
process.on("SIGINT",  () => shutdown("SIGINT"));
```

(Make sure SIGTERM actually reaches the process: exec-form `CMD`, no `sh -c` wrapper, PID-1 init for reaping — see Docker section.)

**Database migrations: expand/contract, never destructive in the same deploy.** The hard rule: during a rolling deploy, **old and new code run against the same schema simultaneously**, so the schema must be compatible with BOTH. Destructive changes (drop/rename column, drop table, narrow a type, add `NOT NULL` without default) in the same deploy that ships the code break the old instances mid-rollout and are near-impossible to roll back.

Use the **expand → migrate → contract** sequence across *separate* deploys:
1. **Expand** (backward-compatible): add the new nullable column / new table / new index. Old code ignores it.
2. **Deploy code** that writes to both old and new (dual-write) and can read either.
3. **Backfill** existing rows in batches (not one giant locking `UPDATE`).
4. **Deploy code** that reads/writes only the new shape.
5. **Contract**: in a *later* deploy, once nothing references the old column, drop it.

To "rename" a column: add new → backfill → switch code → drop old (4+ steps), never `ALTER ... RENAME` in place. Run migrations as an explicit, ordered, idempotent step (a migration tool with up/down + a tracked version), gated before/around the app rollout. Take a backup before any contract/destructive step. Add indexes concurrently where supported (`CREATE INDEX CONCURRENTLY`) to avoid table locks.

**COMMON PITFALLS**
- Deploying by `down` then `up` (stop-the-world) → user-visible downtime every release.
- No SIGTERM handler → dropped in-flight requests on every deploy/scale event.
- Liveness check that depends on the DB → cascading restart loops when the DB hiccups.
- Dropping/renaming a column in the same deploy as the code → old instances 500 mid-rollout, no clean rollback.
- One massive `UPDATE`/`ALTER` that locks the table → app stalls under the lock.
- Migrations that aren't idempotent or version-tracked → double-applied or skipped.
- No tested rollback path → a bad deploy becomes an incident instead of a one-command revert.

---

## Observability: Logs, Metrics, Health

You cannot operate what you cannot see. Three pillars: logs, metrics, traces — plus health endpoints and error tracking.

**Structured logging: JSON with levels.** Emit one JSON object per line to **stdout/stderr** (12-factor: the platform handles routing/aggregation — don't manage log files inside the app). Structured logs are queryable/filterable; freeform strings are not at scale. Include a correlation/request ID to follow a request across services.

```json
{"ts":"2026-06-27T10:00:00Z","level":"error","msg":"db query failed","request_id":"a1b2","route":"/orders","err":"timeout","duration_ms":5021}
```

- **Levels**: `debug` (dev detail), `info` (normal milestones), `warn` (recoverable oddity), `error` (needs attention). Make level configurable via env (`LOG_LEVEL`); run `info`+ in prod.
- **Never log secrets or PII.** No passwords, tokens, API keys, full card numbers, auth headers, or raw request bodies that may contain personal data. Redact at the logging boundary (`Authorization: [REDACTED]`). Leaked secrets in logs are a breach and logs are widely accessible/retained. Be mindful of GDPR/PII regulations.

**Metrics: the four golden signals** — **latency, traffic, errors, saturation** (RED: Rate/Errors/Duration for request-driven services; USE: Utilization/Saturation/Errors for resources). Expose counters/histograms/gauges (e.g. a Prometheus `/metrics` endpoint) and alert on symptoms users feel (error rate up, p99 latency up) rather than on every cause.

**Health & readiness endpoints** (also consumed by the orchestrator — see Deployment):
- `GET /healthz` (liveness): process is up; cheap; no external deps.
- `GET /readyz` (readiness): can serve now; may check critical dependencies (DB ping) but with a timeout, and degrade gracefully.
Return `200` healthy / non-2xx unhealthy with a tiny JSON body.

**Error tracking.** Send exceptions to a tracker (Sentry/Rollbar/etc.) with stack trace, release/commit, and request context — aggregated, deduplicated, alertable. Tag with the deploy SHA so you can see "errors started at release X." Don't rely on grepping logs for exceptions.

**COMMON PITFALLS**
- Plain-text/freeform logs → unsearchable, useless during an incident.
- Logging secrets, tokens, full request bodies, or PII → breach + compliance violation.
- Writing logs to files inside the container → lost on restart, disk fills up.
- No request/correlation ID → can't trace a request across services.
- One log level (everything `info`, or `console.log` everywhere) → noise drowns signal.
- No metrics → you learn about outages from users, not dashboards.
- Health endpoint that returns 200 unconditionally → orchestrator can't detect a sick instance.

---

## Reverse Proxy & TLS

Never expose an app server directly to the internet. Put nginx or Caddy in front for TLS termination, compression, security headers, buffering, and a stable entry point.

**Caddy** — simplest; automatic HTTPS via Let's Encrypt out of the box:

```caddyfile
app.example.com {
    encode gzip zstd
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "no-referrer"
        -Server
    }
    reverse_proxy app:3000
}
```

**nginx** — more control; you manage certs (use certbot or a companion):

```nginx
server {
    listen 443 ssl http2;
    server_name app.example.com;

    ssl_certificate     /etc/letsencrypt/live/app.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/app.example.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;          # no SSLv3/TLS1.0/1.1

    gzip on;
    gzip_types text/plain text/css application/json application/javascript;

    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    add_header Referrer-Policy "no-referrer" always;

    location / {
        proxy_pass http://app:3000;
        proxy_http_version 1.1;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;   # app must trust this for HTTPS detection
        proxy_set_header Upgrade           $http_upgrade;     # websockets
        proxy_set_header Connection        "upgrade";
        proxy_read_timeout 60s;
    }
}
# redirect plain HTTP → HTTPS
server { listen 80; server_name app.example.com; return 301 https://$host$request_uri; }
```

**HTTPS always, in front of everything public.** No plaintext HTTP for anything carrying credentials, tokens, or user data. Redirect 80→443, enable HSTS so browsers refuse downgrade. Let's Encrypt makes certs free and auto-renewing — there's no excuse to skip TLS; **set up renewal** (Caddy auto; nginx via certbot timer) so certs don't silently expire and take the site down.

**Forwarded headers.** Behind a proxy, the app sees the proxy's IP and `http` scheme unless you forward `X-Forwarded-For`/`X-Forwarded-Proto` AND configure the app to trust them (e.g. `app.set('trust proxy', 1)` in Express, `ForwardedHeaders` in others). Otherwise "secure cookie" logic, rate limiting by IP, and redirect-to-HTTPS all misbehave. Only trust these headers from known proxies (a public-facing app trusting client-supplied `X-Forwarded-For` can be spoofed).

**Security headers** (cheap, high value): HSTS, `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY` (or CSP `frame-ancestors`), `Referrer-Policy`, and a real `Content-Security-Policy` for HTML apps. Strip the `Server` version banner.

**Also at the proxy**: gzip/zstd compression, sensible timeouts, max body size limits, and basic rate limiting to blunt abuse before it hits the app.

**COMMON PITFALLS**
- App server directly on :80/:443 to the internet → no TLS, no buffering, slowloris exposure.
- Plain HTTP for a public app → credentials/tokens in cleartext.
- Expired cert because renewal wasn't automated → hard outage.
- Forgetting `X-Forwarded-Proto` → infinite HTTPS-redirect loops, insecure cookies.
- Trusting client `X-Forwarded-For` from the open internet → spoofed IPs bypass rate limits.
- Weak TLS (TLS 1.0/1.1, old ciphers) → fails audits, downgrade attacks.
- No body-size limit → memory-exhaustion via huge uploads.

---

## Reliability: Timeouts, Retries, Degradation

Distributed systems fail partially and constantly. Assume every network call can hang, fail, or be slow, and contain the blast radius.

**Set timeouts on EVERY external call** — HTTP clients, DB queries, cache, queue, RPC. The default for most clients is *infinite*; one slow dependency then exhausts your connection/thread pool and the whole service hangs (cascading failure). Use connect + read timeouts, and an overall deadline.

```js
const ctrl = new AbortController();
const t = setTimeout(() => ctrl.abort(), 3000);   // 3s budget
try {
  const res = await fetch(url, { signal: ctrl.signal });
} finally { clearTimeout(t); }
```

**Retry transient failures with exponential backoff + jitter — and a cap.** Retry only idempotent operations and only transient errors (timeouts, 503, connection reset). Backoff prevents hammering a struggling dependency; jitter prevents synchronized retry storms (the "thundering herd").

```js
async function withRetry(fn, { tries = 3, base = 200, cap = 2000 } = {}) {
  let lastErr;
  for (let i = 0; i < tries; i++) {
    try { return await fn(); }
    catch (e) {
      lastErr = e;
      if (!isTransient(e) || i === tries - 1) throw e;
      const backoff = Math.min(cap, base * 2 ** i);
      const jitter  = Math.random() * backoff;     // full jitter
      await sleep(backoff / 2 + jitter / 2);
    }
  }
  throw lastErr;
}
```

Do NOT retry non-idempotent writes blindly (you may double-charge/double-create) — see idempotency. Do NOT retry 4xx (your request is wrong; retrying won't help).

**Circuit breaker.** When a dependency keeps failing, stop calling it for a cooldown: after N consecutive failures the breaker *opens* and calls fail fast (no waiting on timeouts); after a cooldown it goes *half-open* and lets a probe through; success *closes* it. This prevents you from piling load onto a down service and from blocking all your workers on doomed calls. Retries + breaker together: bounded retries for blips, breaker for sustained outages.

**Idempotency.** Make operations safe to repeat so retries (and at-least-once delivery from queues/webhooks) don't cause duplicates. Have clients send an `Idempotency-Key`; store keys with their result and return the prior result on replay. Use unique constraints / upserts so a duplicated create is a no-op. Critical for payments, order creation, anything money- or side-effect-bearing.

**Graceful degradation.** When a non-critical dependency is down, degrade rather than fail the whole request: serve stale cache, hide an optional widget, queue the work for later, return a sensible default. Decide per-dependency what is critical (DB usually) vs optional (recommendations, avatars). Combine with timeouts so a slow optional call can't drag down the core response. Use bulkheads (separate connection pools per dependency) so one saturated dependency can't consume all resources.

**COMMON PITFALLS**
- No timeouts → one hung dependency takes down the whole service (cascading failure).
- Infinite/unbounded retries with no backoff → you DDoS your own failing dependency.
- Retrying non-idempotent writes → duplicate charges/orders/emails.
- Retrying 4xx/permanent errors → wasted load, no recovery.
- Synchronized retries without jitter → thundering herd when a dependency recovers.
- Treating an optional dependency as critical → unnecessary full-request failures.
- No circuit breaker → workers all block on a dead dependency until everything times out.

---

## Common Amateur Mistakes (Quick Audit)

Run this checklist before shipping. Each maps to a section above.

- **Huge Docker images** (1 GB+, dev deps and compilers in prod). → Multi-stage builds, slim base, `.dockerignore`. (Docker)
- **Secrets in the image or repo** (`ENV SECRET=`, committed `.env`, copied key files). Inspectable forever in history. → Runtime injection; rotate anything leaked. (Docker, Environment Config)
- **No healthchecks** → orchestrator/LB can't tell a sick instance from a healthy one; rollouts and restarts fly blind. → Liveness + readiness endpoints. (Compose, Deployment, Observability)
- **Running containers as root** → container escape = host-level privilege. → `USER app`, non-root. (Docker)
- **No graceful shutdown** (no SIGTERM handler, shell-form CMD) → every deploy drops in-flight requests. → Trap SIGTERM, drain, exec-form CMD. (Deployment)
- **Destructive migrations in the deploy** (drop/rename/NOT NULL alongside the code) → old instances 500 mid-rollout, no rollback. → Expand/contract across deploys, backfill in batches. (Migrations)
- **`latest` tags everywhere** → non-reproducible builds, can't roll back, can't tell what's running. → Pin bases; tag artifacts by commit SHA. (Docker, CI/CD)
- **Logging secrets / PII / full request bodies** → breach + compliance violation; logs are broadly accessible. → Redact at the boundary; structured JSON. (Observability)
- **No timeouts on external calls** → one slow dependency hangs the whole service. → Timeouts + bounded retries with jittered backoff + circuit breaker. (Reliability)
- **Public app with no TLS/reverse proxy** → cleartext credentials, no buffering, direct exposure of the app server. → nginx/Caddy in front, HTTPS + HSTS, security headers, auto-renew certs. (Reverse Proxy & TLS)
- **Rebuilding per environment** → prod runs a different binary than you tested. → Build once, promote the same digest. (CI/CD)
- **Pipeline green on failing tests** (`|| true`, ignored exit codes) → broken code merges with false confidence. → Fail the pipeline on any error; required checks. (CI/CD)
- **Config with insecure defaults** (`JWT_SECRET="changeme"`) or **missing-config that boots anyway** → silent insecure prod / mystery failures. → No defaults for secrets; fail fast on missing required config. (Environment Config)
- **Database data only in the container layer / anonymous volume** → wiped on recreate. → Named/persistent volumes; backups. (Compose)
- **No rollback plan** → a bad deploy becomes a multi-hour incident. → Immutable tags + blue-green/keep-previous so rollback is one command. (Deployment)

When in doubt: small images, secrets out of code, health checks on, drain on shutdown, non-destructive migrations, timeouts everywhere, TLS in front, build once. These prevent the large majority of production incidents.
