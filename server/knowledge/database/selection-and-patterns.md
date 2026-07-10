# Database Selection & Usage Patterns

Opinionated guide for choosing a database, picking an ORM/driver, and applying database-specific patterns. Each `##` section is self-contained. Companion to `schema-and-queries.md` and `optimization.md`.

## Database Selection Decision Tree

Follow top-down. Stop at the first match.

```
START
 |
 +--> Embedded/mobile/desktop/CLI app with NO remote server?
 |     YES --> SQLite (WAL mode). Done.
 |
 +--> Primary need is caching, sessions, rate limiting, or ephemeral queues?
 |     YES --> Redis (or Valkey). NOT as primary datastore. Done.
 |
 +--> Need fully-managed BaaS with auth + real-time, team has <3 backend engineers?
 |     YES --> Supabase (Postgres under the hood) or Firebase (document DB). Done.
 |
 +--> Data is truly schemaless, varies per-document, never needs cross-document
 |    transactions or complex joins?
 |     YES --> MongoDB. But most "schemaless" data has a schema you haven't
 |             written down yet. Done.
 |
 +--> Existing MySQL/MariaDB database you must integrate with?
 |     YES --> MySQL. Don't migrate for the sake of it. Done.
 |
 +--> Everything else:
       USE POSTGRESQL. Handles JSONB, full-text search, geospatial (PostGIS),
       vector embeddings (pgvector), time-series, graph queries (recursive CTEs).
       Scales to hundreds of millions of rows before you need anything exotic.
```

**When in doubt, PostgreSQL.** Managed options ranked by ease: Supabase > Neon (serverless) > Railway > RDS > self-hosted.

PITFALLS:
- Choosing MongoDB because "we don't know the schema yet" -- use Postgres with a JSONB column for the flexible parts.
- Using Redis as a primary database. One OOM kill and your data is gone.
- Running SQLite in a web server with multiple workers writing concurrently. SQLite is single-writer.


## ORM & Driver Selection by Language

### Python -- Default: SQLAlchemy 2.0

```python
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column

engine = create_async_engine("postgresql+asyncpg://user:pw@localhost/db")
Session = async_sessionmaker(engine, expire_on_commit=False)

class Base(DeclarativeBase):
    pass

class User(Base):
    __tablename__ = "users"
    id: Mapped[int] = mapped_column(primary_key=True)
    email: Mapped[str] = mapped_column(unique=True, index=True)
    name: Mapped[str]

async with Session() as s:
    user = (await s.execute(select(User).where(User.email == email))).scalar_one_or_none()
```

Alternatives: **asyncpg** (raw speed), **SQLModel** (simple CRUD + Pydantic), **psycopg3** (scripts/pipelines).

### JavaScript/TypeScript -- Default: Drizzle ORM

```typescript
import { drizzle } from "drizzle-orm/node-postgres";
import { pgTable, serial, text, timestamp } from "drizzle-orm/pg-core";
import { eq } from "drizzle-orm";

const users = pgTable("users", {
  id: serial("id").primaryKey(),
  email: text("email").notNull().unique(),
  name: text("name").notNull(),
  createdAt: timestamp("created_at").defaultNow(),
});

const db = drizzle(process.env.DATABASE_URL!);
const user = await db.select().from(users).where(eq(users.email, email));
```

Alternatives: **Prisma** (great DX but heavy, clunky raw SQL), **Knex** (query builder, no types), **pg** (raw driver for scripts/lambdas).

### Rust -- Default: sqlx

```rust
use sqlx::PgPool;

#[derive(sqlx::FromRow)]
struct User { id: i64, email: String, name: String }

let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
let user = sqlx::query_as::<_, User>("SELECT id, email, name FROM users WHERE email = $1")
    .bind(&email).fetch_optional(&pool).await?;
```

Alternatives: **Diesel** (full ORM, compile-time DSL), **SeaORM** (async ActiveRecord-style).

### Go -- Default: sqlc

```sql
-- name: GetUserByEmail :one
SELECT id, email, name FROM users WHERE email = $1;
```
```go
user, err := queries.GetUserByEmail(ctx, email)  // generated, type-safe
```

Alternatives: **GORM** (hides SQL, surprising queries), **database/sql + pgx** (raw driver).

COMMON MISTAKES:
- Using an ORM to avoid learning SQL. You WILL need raw queries for reports, migrations, and debugging.
- Not enabling connection pooling in ORM config. Every ORM above supports it; configure `pool_size` explicitly.


## PostgreSQL Power Features

### JSONB

Use for: user preferences, API response caching, event metadata -- anything where shape varies per-row. Use proper columns for anything you filter, sort, join, or enforce constraints on.

```sql
CREATE TABLE events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    event_type TEXT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES users(id),
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_events_metadata ON events USING GIN (metadata);

SELECT * FROM events WHERE metadata @> '{"source": "api"}';       -- containment
SELECT metadata->>'ip_address' AS ip FROM events WHERE event_type = 'login';  -- extract
```

### Full-Text Search

Good enough for search within your app up to ~10M rows. Switch to Meilisearch/Typesense for typo tolerance, faceted search, or when search is the core feature.

```sql
ALTER TABLE articles ADD COLUMN search_vector tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('english', coalesce(title, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(body, '')), 'B')
  ) STORED;
CREATE INDEX idx_articles_search ON articles USING GIN (search_vector);

SELECT id, title, ts_rank(search_vector, query) AS rank
FROM articles, plainto_tsquery('english', 'database optimization') AS query
WHERE search_vector @@ query ORDER BY rank DESC LIMIT 20;
```

### Key Extensions

```sql
CREATE EXTENSION IF NOT EXISTS pg_trgm;   -- fuzzy search: WHERE name % 'jonh' finds "john"
CREATE EXTENSION IF NOT EXISTS vector;     -- pgvector: AI embeddings, semantic search
CREATE EXTENSION IF NOT EXISTS postgis;    -- geospatial: ST_DWithin, ST_Distance
CREATE EXTENSION IF NOT EXISTS pgcrypto;   -- gen_random_uuid(), crypt(), hmac()
```

### Advanced Queries

```sql
-- LATERAL join: top-N per group without subquery tricks
SELECT u.id, u.email, latest.*
FROM users u LEFT JOIN LATERAL (
    SELECT created_at, total FROM orders
    WHERE user_id = u.id ORDER BY created_at DESC LIMIT 3
) latest ON true;

-- Advisory locks: application-level mutex
SELECT pg_advisory_lock(hashtext('process-invoices'));

-- LISTEN/NOTIFY: lightweight pub/sub
LISTEN new_order;
NOTIFY new_order, '{"order_id": 42}';
```

PITFALLS:
- Using `->` (returns JSON) when you need `->>` (returns text). `WHERE data->'age' = '25'` silently fails.
- Indexing every JSONB key individually instead of a single GIN index with `@>`.
- Running pgvector IVFFlat on fewer rows than the `lists` count. Use HNSW for small datasets.


## SQLite Patterns

### Essential Configuration (Every Connection)

```sql
PRAGMA journal_mode = WAL;       -- concurrent reads + writes
PRAGMA busy_timeout = 5000;      -- wait 5s on lock instead of instant fail
PRAGMA synchronous = NORMAL;     -- safe with WAL
PRAGMA foreign_keys = ON;        -- OFF by default (!)
PRAGMA cache_size = -64000;      -- 64MB page cache
```

### Concurrency: Unlimited Readers, ONE Writer

WAL mode: readers never block writers, writers never block readers. But two writes serialize. Fine for: desktop, mobile, CLI, dev/test, single-process web, edge workers. Breaks at: multiple processes doing frequent writes -- use Postgres.

### When SQLite Beats Postgres

- **Embedded**: no server, single file, zero-config
- **Read-heavy**: no network round-trip = faster simple reads
- **Dev/test**: copy a file = fresh database per test run
- **Edge**: Turso/D1 replicate SQLite to the edge
- **Single binary**: ship app + data as one artifact

### Replication

**Litestream**: continuous backup to S3 (disaster recovery). **Turso/libSQL**: distributed read replicas. **LiteFS**: FUSE-based primary-replica (Fly.io).

PITFALLS:
- Forgetting `PRAGMA foreign_keys = ON`. SQLite silently ignores FK violations by default.
- Using SQLite over NFS/SMB. It requires POSIX file locking; network filesystems break this. Database will corrupt.
- Not setting `busy_timeout`. Default is 0 -- instant `SQLITE_BUSY` error on any contention.


## MongoDB Patterns

### Embed vs Reference

**Embed** when: child data always read with parent, belongs to one parent (1:1 or 1:few), array won't grow unbounded. **Reference** when: shared across parents (many-to-many), queried independently, array could grow to thousands.

```javascript
// EMBED: user with addresses (1:few)
{ _id: ObjectId("..."), name: "Alice",
  addresses: [{ type: "home", street: "123 Main St", city: "Portland" }] }

// REFERENCE: orders -> products (many-to-many, products queried alone)
{ _id: ObjectId("..."), user_id: ObjectId("..."),
  items: [{ product_id: ObjectId("..."), quantity: 2, price_at_purchase: 1999 }] }
```

### Indexes

```javascript
db.orders.createIndex({ user_id: 1, created_at: -1 });           // compound
db.articles.createIndex({ title: "text", body: "text" });         // text search
db.sessions.createIndex({ expires_at: 1 }, { expireAfterSeconds: 0 });  // TTL auto-delete
```

### Aggregation Pipeline

```javascript
db.orders.aggregate([
  { $match: { created_at: { $gte: new Date(Date.now() - 30*86400000) } } },
  { $unwind: "$items" },
  { $lookup: { from: "products", localField: "items.product_id",
               foreignField: "_id", as: "product" } },
  { $unwind: "$product" },
  { $group: { _id: "$product.category",
              revenue: { $sum: { $multiply: ["$items.quantity", "$items.price_at_purchase"] } } } },
  { $sort: { revenue: -1 } }
]);
```

COMMON MISTAKES:
- Unbounded array growth. An embedded array growing with every action hits 16MB document limit.
- Using MongoDB to avoid SQL, then rebuilding joins with `$lookup` and manual reference integrity.
- Assuming sharding makes it "web scale" automatically. A single Postgres handles more load than an untuned Mongo cluster.


## Redis Patterns

Use alongside your primary database, not instead of it.

### Data Structure Selection

| Need | Structure | Example |
|------|-----------|---------|
| Simple cache | String | `SET user:42 "{...}" EX 3600` |
| Object with fields | Hash | `HSET user:42 name Alice email a@b.com` |
| Unique membership | Set | `SADD online_users 42` / `SISMEMBER` |
| Ranked leaderboard | Sorted Set | `ZADD leaderboard 9500 "alice"` |
| Unique counting | HyperLogLog | `PFADD pageviews:2024-01 user42` (~0.81% error, 12KB) |
| Event log / queue | Stream | `XADD orders * item widget qty 2` |

### Caching: cache-aside is the default

```
GET cache_key -> HIT: return -> MISS: query DB, SET cache_key value EX ttl, return
```

TTL on everything. Data without TTL is a memory leak. Start at 1 hour, tune from there.

### Rate Limiting (Sliding Window)

```python
def is_rate_limited(r: redis.Redis, key: str, limit: int, window_secs: int) -> bool:
    now = time.time()
    pipe = r.pipeline()
    pipe.zremrangebyscore(key, 0, now - window_secs)
    pipe.zadd(key, {str(now): now})
    pipe.zcard(key)
    pipe.expire(key, window_secs)
    return pipe.execute()[2] > limit
```

### Pub/Sub vs Streams

**Pub/Sub**: fire-and-forget, no persistence. Fine for real-time notifications where missing a message is OK. **Streams**: persistent, replayable, consumer groups with ACK. Use for job queues and event sourcing.

### Job Queues

Use **BullMQ** (Node.js) or **RQ** (Python), not raw Redis. They handle retries, dead-letter, concurrency.

### Persistence

**RDB** (snapshots): fast restarts, lose data since last snapshot. **AOF**: logs every write, more durable (`appendfsync everysec`). **Both**: recommended for production.

PITFALLS:
- Not setting `maxmemory` + `maxmemory-policy`. Redis grows until OOM kills it. Use `allkeys-lru` for caches.
- Using `KEYS *` in production. Blocks the single-threaded event loop. Use `SCAN`.
- Pub/Sub without persistence: consumer down = message gone forever. Use Streams for reliability.


## Connection Management

### Pool Sizing

**Formula**: `connections = (physical_cores * 2) + effective_spindle_count`. For SSD: 4-core server = ~9-10 connections. More connections = more contention, NOT more throughput.

```python
engine = create_engine("postgresql://...",
    pool_size=10, max_overflow=5, pool_timeout=30,
    pool_recycle=1800, pool_pre_ping=True)
```

### PgBouncer

Use when app instances exceed Postgres `max_connections` (default: 100). **Transaction mode** (recommended): connection assigned per-transaction, returned after COMMIT. Session mode is safe but less efficient.

### Serverless Connections

Serverless functions create a connection per invocation, exploding connection counts. Solutions: **Neon** (built-in pooler + HTTP), **Supabase Pooler**, **PlanetScale** (HTTP driver).

```typescript
import { neon } from "@neondatabase/serverless";
const sql = neon(process.env.DATABASE_URL!);
const users = await sql`SELECT * FROM users WHERE id = ${userId}`;
```

COMMON MISTAKES:
- Setting `pool_size=100` thinking more is faster. Each connection costs ~10MB RAM. Optimal pool is surprisingly small.
- PgBouncer transaction mode with prepared statements. Not supported across transactions; set `prepared_statements=false`.
- Serverless connecting directly to Postgres without a pooler. Traffic spike = thousands of connections = database crash.


## Migration Best Practices

### Zero-Downtime: Expand, Migrate, Contract

Never make a breaking schema change in one step:

```
EXPAND:   Add new column (nullable). Deploy code that writes to BOTH old and new.
MIGRATE:  Backfill existing data. Verify parity.
CONTRACT: Deploy code using only new column. Drop old column.
```

### Migration Tools

| Stack | Tool | Notes |
|-------|------|-------|
| TypeScript/Prisma | `prisma migrate` | Auto-generates SQL from schema diff |
| TypeScript/Drizzle | `drizzle-kit` | `push` for dev, `generate`+`migrate` for prod |
| Python/SQLAlchemy | Alembic | `alembic revision --autogenerate` then review |
| Rust/sqlx | `sqlx-cli` | `sqlx migrate add` / `sqlx migrate run` |
| Go | golang-migrate | Plain SQL up/down files |

### Large Table Backfills

Never update millions of rows in one transaction. Batch:

```sql
DO $$
DECLARE batch_size INT := 5000; rows_updated INT;
BEGIN
    LOOP
        UPDATE users SET full_name = name
        WHERE full_name IS NULL AND id IN (
            SELECT id FROM users WHERE full_name IS NULL LIMIT batch_size);
        GET DIAGNOSTICS rows_updated = ROW_COUNT;
        COMMIT;
        EXIT WHEN rows_updated = 0;
        PERFORM pg_sleep(0.1);
    END LOOP;
END $$;
```

### Rollback Planning

Every migration needs a rollback plan BEFORE running. Expand = safe (drop new column). Migrate = revert code. Contract = dangerous -- verify with query logs that nothing reads old columns before dropping.

PITFALLS:
- `ALTER TABLE ADD COLUMN NOT NULL` without default on a large table. PG11+ is instant WITH a default; without one it fails on existing NULLs.
- Auto-generated migrations that drop+recreate columns. Always review generated SQL.
- Backfilling in a single transaction. 10-minute backfill = 10-minute lock = all writes queued.
