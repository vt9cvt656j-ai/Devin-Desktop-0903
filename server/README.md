# Michael 总后台 — central real-time backend

The central server for the Michael platform: shared accounts, a real-time admin
dashboard, and an API the IDE talks to (replacing the IDE's local-only SQLite auth).

## Stack (chosen for a large, high-traffic project)

| Layer | Choice | Why |
|---|---|---|
| Database | **PostgreSQL 17** | Modern default; far better than MySQL for write-heavy, concurrent, JSON, analytical workloads; scales via partitioning / Citus |
| Backend | **Rust + Axum** | Highest raw throughput; same language as the IDE backend (sqlx shared) |
| Real-time | **WebSocket + Redis Pub/Sub** | Stateless app nodes, fanned out via a Redis backplane → horizontal scale |
| Cache/Sessions | **Redis** | Verification-code TTL, online presence, future rate-limiting |
| Auth | **JWT (HS256) + bcrypt** | Stateless tokens; password hashing |

Everything is config-driven (env / `.env`); **no secrets in code or git**.

## API

```
POST /api/auth/check-email   { email }            -> { exists }
POST /api/auth/send-code     { email }            -> { sent }       # emails a 6-digit code
POST /api/auth/register      { email, password, code } -> { token, user }
POST /api/auth/login         { email, password }  -> { token, user }
POST /api/auth/verify-code   { email, code }      -> { token, user } # passwordless login
GET  /api/me                 (Bearer token)       -> user
GET  /api/admin/users        (admin)              -> [user]
GET  /api/admin/stats        (admin)              -> { total_users, today_users, online }
GET  /ws                     -> WebSocket live event feed (register/login/…)
GET  /health                 -> "ok"
```

## Run locally / on the server (Docker)

```bash
cp .env.example .env      # fill JWT_SECRET, POSTGRES_PASSWORD, QQ_SMTP_* …
docker compose up -d --build
curl localhost:8080/health
```

Postgres + Redis + the backend come up together. Without `QQ_SMTP_*` set, codes
are printed to the server log (dev mode) instead of emailed.

## Production operations

The production compose file binds the backend to loopback. Nginx terminates TLS on
ports 443 and 8443; PostgreSQL and Redis are never published to the host network.

Deploy from this directory:

```bash
SERVER_HOST=154.44.13.133 \
SERVER_KEY=~/.ssh/michael_server \
./deploy.sh
```

The script creates a database/site backup first, syncs source without touching
the server `.env`, validates Compose, rebuilds, and waits for `/health`.

Install the tracked Nginx configuration and daily backup timer once:

```bash
sudo install -m 0644 nginx/michael-backend.conf /etc/nginx/sites-available/michael-backend
sudo install -m 0644 nginx/michaelide-sites.conf /etc/nginx/sites-available/michaelide-sites
sudo install -m 0644 systemd/michael-db-backup.* /etc/systemd/system/
sudo nginx -t && sudo systemctl reload nginx
sudo systemctl daemon-reload
sudo systemctl enable --now michael-db-backup.timer
```

Backups are written atomically to `/var/backups/michael-ide`, verified with
`pg_restore --list`, checksummed, and retained for 14 days by default. A separate
off-host snapshot should still be configured for disaster recovery.

## Effective-context evaluation

`scripts/context-eval.mjs` measures the 1M/2M/5M compression tiers with exact
needles near 5%, 50%, and 95% plus facts that require multi-hop recall. It uploads
history incrementally below the 12 MiB request limit and follows the gateway's
prefix contract. The default is a no-cost dry run:

```bash
node scripts/context-eval.mjs --tier 1m
```

Paid requests require a dedicated evaluation account key and an explicit flag:

```bash
MICHAEL_EVAL_API_KEY=... node scripts/context-eval.mjs --tier 1m --execute
```

For the PostgreSQL recovery check, also set `MICHAEL_EVAL_SSH_TARGET`,
`MICHAEL_EVAL_SSH_KEY`, and `MICHAEL_EVAL_COMPOSE_DIR`, then add
`--verify-pg-recovery`. That mode reads the evaluation prefix record from
PostgreSQL, deletes only its related Redis prefix/summary/archive/search keys,
and performs recall through the same prefix. Secrets and raw prefix tokens are
never written to the report.

Use the same `--run-id` for sequential 2M and 5M runs. Their fixture prefix is
byte-stable, so the 5M run reuses the 2M segment cache and only compresses newly
added history. This lowers evaluation cost while testing the production cache
contract.

These are effective-context tests, not a claim that compression gives a model a
native full-attention 1M/2M/5M window. A pass proves durable storage, retrieval,
and end-model recall for the tested facts and workload.

## Make yourself an admin

After registering once:

```sql
UPDATE users SET role = 'admin' WHERE email = 'you@qq.com';
```

## Security

- Real `.env` is git-ignored; never commit secrets.
- Nginx terminates TLS; the backend is published only on `127.0.0.1:8080`.
- Use SSH keys (not the root password) and a firewall on the VPS.
