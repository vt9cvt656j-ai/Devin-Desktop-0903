# Database: Schema & Queries

Battle-tested, opinionated best practices for designing and using relational databases. Each `##` section is self-contained. Default dialect is PostgreSQL; MySQL/SQLite differences are called out. When in doubt, prefer correctness and explicitness over cleverness.

## Quick Reference: The 10 Rules That Prevent 90% of Disasters

- ALWAYS use parameterized queries. NEVER string-concatenate user input into SQL. This is the #1 security hole (SQL injection).
- Index every foreign key. Databases do NOT do this automatically (except MySQL/InnoDB, which auto-indexes FKs).
- Store money as integer minor units (cents) or `DECIMAL/NUMERIC`. NEVER `float`/`double` — `0.1 + 0.2 != 0.3`.
- Every table gets a primary key, `created_at`, and `updated_at`. No exceptions for real tables.
- Use proper column types. Not everything is `TEXT`. Dates are `timestamptz`, booleans are `boolean`, numbers are numeric.
- Add `NOT NULL` to everything that should never be null. Null is not "empty" — it is "unknown" and breaks comparisons.
- Watch for N+1 queries. One query in a loop = N+1. Use JOINs, eager loading, or batched `IN (...)`.
- All schema changes go through migration files. NEVER hand-edit production schema with ad-hoc `ALTER`.
- Wrap multi-statement invariants in a transaction. Keep transactions SHORT. No network/LLM calls inside a transaction.
- `EXPLAIN (ANALYZE)` any query that feels slow before guessing. A `Seq Scan` on a big table in a hot path is a red flag.

COMMON PITFALL: Treating the database as a dumb key-value bucket for JSON blobs. You lose constraints, types, indexing, and query power. Model your data.

## Schema Design: Normalization

- Normalize first; denormalize later with evidence. Default target is 3rd Normal Form (3NF): every non-key column depends on the key, the whole key, and nothing but the key.
- 1NF: atomic columns, no repeating groups. DON'T store `"red,green,blue"` in one column — use a child table or (Postgres) an array/`jsonb` only if you never query inside it.
- 2NF/3NF in practice: if a fact about entity A is stored on entity B's rows, it belongs in A's table. Example: don't put `customer_email` on every `order` row — put it on `customers` and reference `customer_id`.
- Each distinct concept = its own table. Users, orders, order_items, products — separate tables joined by keys.
- Many-to-many always needs a junction table (a.k.a. join/bridge table):

```sql
CREATE TABLE student_courses (
  student_id BIGINT NOT NULL REFERENCES students(id) ON DELETE CASCADE,
  course_id  BIGINT NOT NULL REFERENCES courses(id)  ON DELETE CASCADE,
  enrolled_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (student_id, course_id)   -- composite PK prevents duplicate enrollment
);
```

When to DENORMALIZE (deliberately, with a comment explaining why):
- Read-heavy aggregates that are expensive to compute every request (e.g. `comment_count` on `posts`). Keep it in sync via trigger, transaction, or scheduled job — and accept it can drift.
- Reporting/analytics tables (star schema, materialized views) separate from the transactional (OLTP) schema.
- Caching a foreign value to avoid a hot-path JOIN — only after profiling proves the JOIN is the bottleneck.

COMMON PITFALLS:
- Denormalizing prematurely "for performance" before any measurement. You create write-time bugs (two copies disagree) for speed you didn't need.
- Storing CSV/JSON lists you later need to filter or join on. You will regret it; querying inside them is slow and unindexable in most engines.
- One giant "god table" with 80 nullable columns covering 4 different entity types. Split it.

## Schema Design: Choosing Column Types

- Use the most specific type that fits. Types are free documentation AND enforced constraints AND storage/perf wins.
- Text: prefer `TEXT` (Postgres) or `VARCHAR(n)` where `n` is a real business limit. In Postgres `TEXT` and `VARCHAR` perform identically — pick `VARCHAR(n)` only to enforce a max length. `CHAR(n)` is almost never right (pads with spaces).
- Integers: `SMALLINT` (±32k), `INTEGER` (±2.1B), `BIGINT` (±9.2e18). Use `BIGINT` for any id/counter that could ever grow — the 2006 "ran out of INT ids" outage is a classic.
- Money: `NUMERIC(19,4)` / `DECIMAL` for exact decimals, OR integer minor units (cents). NEVER `REAL`/`FLOAT`/`DOUBLE` for money.
- Dates/times: `TIMESTAMPTZ` (timestamp with time zone) in Postgres — store UTC, convert at the edge. `DATE` for date-only. NEVER store datetimes as strings or epoch-in-a-VARCHAR.
- Booleans: native `BOOLEAN`. Not `'Y'/'N'`, not `0/1` in an INT, not `'true'` strings.
- Enums/status: a `VARCHAR` + `CHECK (status IN (...))`, or a lookup table, or Postgres `ENUM`. Lookup table is most flexible (add values without migration); Postgres `ENUM` is fast but altering it is a migration. Avoid magic integers with no meaning.
- UUIDs: native `UUID` type, not `VARCHAR(36)` (half the storage, validated).
- JSON: Postgres `JSONB` (binary, indexable, dedup keys) over `JSON` (raw text). Use for genuinely schemaless/sparse data — not as an excuse to skip modeling.
- IP/network, MAC, geo: Postgres has `INET`, `CIDR`, `MACADDR`, PostGIS. Use them over text when you query them.

```sql
-- Good: precise types, constraints baked in
CREATE TABLE invoices (
  id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  amount_cents BIGINT  NOT NULL CHECK (amount_cents >= 0),  -- money as integer cents
  currency     CHAR(3) NOT NULL DEFAULT 'USD',
  status       VARCHAR(20) NOT NULL DEFAULT 'pending'
                 CHECK (status IN ('pending','paid','void','refunded')),
  due_on       DATE NOT NULL,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

COMMON PITFALLS:
- `TEXT`-for-everything. You lose validation, indexing efficiency, and accidentally store `"true"`, `"N/A"`, `"01/02/03"` (which date format?!).
- Floats for money or quantities that must sum exactly. Rounding errors compound and accountants notice.
- `VARCHAR(255)` cargo-culted everywhere. Pick a real limit or use `TEXT`.

## Schema Design: Primary Keys

- Every table needs a primary key. It is the row's identity, the default clustering, and what FKs point at.
- Prefer a synthetic (surrogate) key over a natural one (email, SSN, slug). Natural keys change; changing a PK cascades pain. Keep natural keys as `UNIQUE` constraints instead.
- Two solid choices:
  - `BIGINT GENERATED ALWAYS AS IDENTITY` (SQL-standard auto-increment). Small, fast, sequential, great for indexes and locality. Downside: guessable/enumerable, leaks row counts, coordination needed across shards.
  - `UUID` — globally unique, generatable client-side (no round-trip), non-enumerable, merge-friendly across systems. Downside: 16 bytes, and random UUIDv4 hurts index locality / insert performance on big tables.
- UUID guidance: prefer **time-ordered UUIDv7** (or ULID) over random v4 when you'll have high insert volume — keeps B-tree inserts sequential and avoids index fragmentation/page splits.

```sql
-- Postgres identity (preferred over the older SERIAL)
id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY

-- UUID v4 (needs pgcrypto or gen_random_uuid in PG13+)
id UUID PRIMARY KEY DEFAULT gen_random_uuid()
```

- Composite PKs are correct for junction tables and natural compound identities (see normalization section). Otherwise a single surrogate key is simpler for ORMs and FKs.
- MySQL note: InnoDB clusters the table on the PK. A random UUID PK there is especially costly — use `BIGINT AUTO_INCREMENT` PK and keep the UUID as a secondary `UNIQUE` column, or use ordered UUIDs.
- SQLite note: an `INTEGER PRIMARY KEY` aliases the internal `rowid` (fast). Anything else creates a separate index.

COMMON PITFALLS:
- Using a mutable natural key (email) as PK — then the user changes it and every FK row is orphaned or must cascade.
- `SERIAL` in modern Postgres — prefer `GENERATED ... AS IDENTITY` (cleaner ownership/permissions, standard).
- No PK at all ("it's just a log table") — replication, dedup, and `UPDATE`/`DELETE` targeting all suffer. Add one.
- Exposing sequential integer ids in public URLs — leaks volume and invites enumeration. Use UUIDs or hashids for public surfaces.

## Schema Design: Foreign Keys & ON DELETE Behavior

- Declare foreign keys. They enforce referential integrity — the DB refuses to create orphan rows. App-level "we'll keep it consistent" always eventually fails.
- Always pick an explicit `ON DELETE` (and usually `ON UPDATE`) action; the default `NO ACTION` will block deletes and surprise you.

`ON DELETE` options:
- `CASCADE` — delete children when parent is deleted. Right for owned/dependent rows (order_items when order dies, a post's comments). DANGER: a cascade can wipe huge subtrees; know your graph.
- `RESTRICT` / `NO ACTION` — prevent deleting a parent that still has children. Safe default for important references (don't delete a `customer` who has `orders`).
- `SET NULL` — null the FK on child (column must be nullable). Good for optional links (set `assigned_to = NULL` when a user is removed).
- `SET DEFAULT` — set to a default row (rare; e.g. reassign to a "deleted user" placeholder).

```sql
CREATE TABLE order_items (
  id       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  order_id BIGINT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,   -- owned by order
  product_id BIGINT NOT NULL REFERENCES products(id) ON DELETE RESTRICT, -- don't delete a sold product
  qty INTEGER NOT NULL CHECK (qty > 0)
);
```

- INDEX YOUR FOREIGN KEYS. Postgres/SQLite do NOT auto-create an index on the referencing column. Without it: slow JOINs AND every parent delete/update does a full scan of the child table to check the constraint (and can take a heavy lock). MySQL/InnoDB auto-creates this index; the others do not.
- Prefer soft delete (`deleted_at TIMESTAMPTZ`) over hard delete when you need history/audit/undo — but then you must filter `WHERE deleted_at IS NULL` everywhere (a partial index helps).
- Deferrable constraints (`DEFERRABLE INITIALLY DEFERRED`, Postgres) let you insert circular references within one transaction.

COMMON PITFALLS:
- No FK at all → orphan rows accumulate silently; reports double-count or crash on missing joins.
- No index on the FK column → mysteriously slow JOINs and `DELETE` storms on the parent table.
- Blanket `ON DELETE CASCADE` everywhere → one `DELETE FROM users WHERE id=1` nukes years of data. Reserve cascade for truly owned data.

## Schema Design: NOT NULL, Defaults, and CHECK Constraints

- Default to `NOT NULL`. Make nullability a deliberate decision. `NULL` means "unknown," propagates through expressions (`NULL = NULL` is `NULL`, not true), and silently breaks aggregates, joins, and `WHERE` logic.
- Pair `NOT NULL` with a sensible `DEFAULT` so inserts don't have to specify everything: `is_active BOOLEAN NOT NULL DEFAULT true`.
- Don't use sentinel values to dodge NULL (`-1`, `''`, `'1970-01-01'`, `9999-12-31`). They corrupt `MIN/MAX/AVG` and comparisons. Either the value is truly unknown (`NULL`) or it has a real default.
- `CHECK` constraints enforce business rules at the data layer (the only place they can't be bypassed): `CHECK (price >= 0)`, `CHECK (end_at > start_at)`, `CHECK (email LIKE '%@%')`.
- `UNIQUE` constraints prevent duplicates (`UNIQUE (email)`, `UNIQUE (tenant_id, slug)`). Note: in standard SQL, `NULL`s are distinct, so multiple NULLs are allowed in a UNIQUE column (Postgres `NULLS NOT DISTINCT` since PG15 changes this).
- Adding `NOT NULL` to a populated table needs care (see migrations/expand-contract): backfill first, then constrain. In older Postgres a bare `ALTER ... SET NOT NULL` scans/locks the whole table.

```sql
CREATE TABLE users (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  email      TEXT    NOT NULL UNIQUE,
  is_active  BOOLEAN NOT NULL DEFAULT true,
  login_count INTEGER NOT NULL DEFAULT 0 CHECK (login_count >= 0),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

COMMON PITFALLS:
- Leaving columns nullable "just in case" → app code is littered with null checks, and a missing one becomes a NullPointer/`None` crash or a wrong query result.
- `COUNT(col)` skips NULLs while `COUNT(*)` doesn't — silent off-by-N in reports.
- Relying on the app to validate instead of `CHECK`/`UNIQUE` → a second writer (script, admin console, migration) inserts garbage the app would have rejected.

## Schema Design: Timestamps (created_at / updated_at)

- Add `created_at TIMESTAMPTZ NOT NULL DEFAULT now()` to essentially every table. It's free, and you ALWAYS end up needing it for debugging, sorting, retention, and audits.
- Add `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()` and keep it current on every update.
- Store UTC (`TIMESTAMPTZ` in Postgres stores an absolute instant). Convert to the user's zone at display time. NEVER store naive local times without a zone for events.
- `updated_at` is NOT auto-updated by the column default — `DEFAULT` only fires on INSERT. Three ways to maintain it:
  - ORM hooks (Rails/Django/Prisma do this) — fine, but a raw SQL update bypasses it.
  - A database trigger — authoritative, catches every writer:

```sql
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN NEW.updated_at = now(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_users_updated_at
  BEFORE UPDATE ON users
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();
```

  - Explicitly set it in every `UPDATE` (error-prone; you'll forget one).
- MySQL shortcut: `updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP`.
- SQLite: no `ON UPDATE`; use a trigger or set it in the query.
- Consider `deleted_at TIMESTAMPTZ NULL` for soft deletes, and audit columns (`created_by`, `updated_by`) when you need accountability.

COMMON PITFALLS:
- Trusting the column `DEFAULT` to bump `updated_at` on updates — it doesn't. Use a trigger or ORM hook.
- Using `TIMESTAMP` without time zone and feeding it local times — DST and multi-region servers produce off-by-an-hour bugs and unorderable rows.
- No timestamps at all, then a data incident hits and you can't tell what changed when.

## Indexing: What to Index

Index columns that the database has to search, match, or sort by:
- Every FOREIGN KEY column (JOINs + constraint checks). Top priority.
- Columns in `WHERE` predicates that are selective (filter to a small fraction of rows).
- Columns used in `JOIN ... ON`.
- Columns in `ORDER BY` / `GROUP BY` for large result sets (an index can supply pre-sorted rows and skip a sort).
- Columns with `UNIQUE` constraints (the DB indexes these automatically).

```sql
CREATE INDEX idx_orders_customer_id ON orders (customer_id);          -- FK
CREATE INDEX idx_orders_status      ON orders (status);                -- WHERE
CREATE INDEX idx_orders_created_at  ON orders (created_at DESC);       -- ORDER BY recent
```

Specialized indexes (Postgres):
- Partial index — index only the rows you query: `CREATE INDEX ON orders (created_at) WHERE status = 'open';` Smaller, faster, perfect for soft-delete (`WHERE deleted_at IS NULL`).
- Expression index — when you query a transformed value: `CREATE INDEX ON users (lower(email));` then query `WHERE lower(email) = $1`.
- GIN index — for `JSONB` containment, arrays, and full-text search.
- `BRIN` — tiny index for huge, naturally-ordered tables (append-only by time).

- Default index type is a B-tree: great for `=`, `<`, `>`, `BETWEEN`, prefix `LIKE 'abc%'`, and ordering. It can NOT help with leading-wildcard `LIKE '%abc'` (use trigram/GIN or full-text).

COMMON PITFALLS:
- Forgetting indexes on FK columns — the single most common cause of slow JOINs and delete storms.
- Indexing a column you never filter/sort/join on — pure write overhead, no benefit.
- Assuming a `UNIQUE` constraint and an index are different things — the constraint already creates the index; don't duplicate it.

## Indexing: Composite Index Column Order, Covering Indexes

- A composite (multi-column) index on `(a, b, c)` can serve queries filtering on a leading prefix: `(a)`, `(a, b)`, `(a, b, c)`. It does NOT help a query that filters only on `b` or `c`. Order matters enormously.
- Rule of thumb for ordering: **equality columns first, then the range/sort column last.** A query `WHERE tenant_id = $1 AND created_at > $2 ORDER BY created_at` wants `(tenant_id, created_at)`.
- Put the most selective equality column early when several are equalities — but the equality-before-range rule dominates: a range column kills the usability of any column after it in the index.

```sql
-- Serves: WHERE customer_id = ? [AND status = ?] [ORDER BY created_at]
CREATE INDEX idx_orders_cust_status_created
  ON orders (customer_id, status, created_at DESC);
```

- COVERING INDEX (index-only scan): include every column the query needs so the DB never touches the table heap. Postgres `INCLUDE` adds non-key payload columns:

```sql
-- SELECT status, total FROM orders WHERE customer_id = ?  -> index-only scan
CREATE INDEX idx_orders_cust_cover
  ON orders (customer_id) INCLUDE (status, total);
```

- For sort + pagination, match the index's column order AND direction to the `ORDER BY` to get a pre-sorted scan and avoid a sort step.
- Don't over-stack columns: a 6-column composite index is large and only earns its keep if real queries use that prefix shape. One well-ordered composite often replaces several single-column indexes.

COMMON PITFALLS:
- Wrong column order: index `(created_at, customer_id)` when the query filters by `customer_id` — the index can't be used for that lookup.
- Putting a range/inequality column before an equality column — everything after the range is unusable for seeking.
- Creating `(a)`, `(a,b)`, and `(a,b,c)` separately — the last one already covers the prefixes; the redundant ones just slow writes.

## Indexing: When NOT to Index, and Spotting Missing Indexes

When NOT to add an index:
- Write-heavy / high-ingest tables: every index must be updated on each `INSERT`/`UPDATE`/`DELETE`. On a hot write path, each extra index is a tax. Index only what reads truly need.
- Low-cardinality columns alone (e.g. `boolean is_active`, a status with 3 values): a plain index barely narrows the scan and the planner may ignore it. A PARTIAL index on the rare value (`WHERE is_active = false`) is far better.
- Small tables (a few thousand rows): a sequential scan is often faster than an index lookup; the planner knows this and may skip the index.
- Columns you only ever `SELECT`, never filter/join/sort on.
- Redundant prefixes of an existing composite index.

Costs of over-indexing: slower writes, more storage, more to keep in cache, longer `VACUUM`/maintenance, and the planner can pick a worse plan when overwhelmed with options.

Spotting a MISSING index with `EXPLAIN`:
- Run `EXPLAIN (ANALYZE, BUFFERS) SELECT ...` (Postgres). `ANALYZE` actually executes and shows real timing/rows; `BUFFERS` shows I/O.
- Red flags:
  - `Seq Scan` on a large table when you filtered to a few rows → likely missing index on the filter column.
  - Big gap between `rows` estimated and actual → stale stats; run `ANALYZE table;`.
  - `Sort` consuming lots of time/memory for an `ORDER BY` that an index could pre-sort.
  - A `Nested Loop` repeatedly seeking a child table with no index → the N+1 of query plans.

```sql
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM orders WHERE customer_id = 42;
-- BAD:  Seq Scan on orders ... rows=2 (filtered from 5,000,000)
-- FIX:  CREATE INDEX idx_orders_customer_id ON orders(customer_id);
-- GOOD: Index Scan using idx_orders_customer_id ...
```

- MySQL: `EXPLAIN`/`EXPLAIN ANALYZE`; watch for `type: ALL` (full scan) and `key: NULL` (no index used).
- After creating indexes or big data changes, refresh planner stats (`ANALYZE` / `ANALYZE TABLE`).

COMMON PITFALLS:
- "Add an index to fix slowness" without reading the plan — you may add one the planner won't use, or one already covered.
- Indexing a boolean and expecting a speedup — too low cardinality; use a partial index on the selective value.
- Death by a thousand indexes on a write-hot table — throughput tanks while most indexes are never used. Drop unused ones (check `pg_stat_user_indexes` for `idx_scan = 0`).

## The N+1 Query Problem (the #1 ORM Performance Killer)

What it is: you run 1 query to fetch a list of N parents, then — usually because lazy-loading a relationship inside a loop — run 1 more query per parent to fetch its children. Total: 1 + N queries. With N=500 that's 501 round-trips, each with network + parse + plan overhead. The app looks fine on 5 test rows and dies in production.

How to SPOT it:
- Logs show the same query repeated with different ids: `SELECT * FROM comments WHERE post_id = 1`, `... = 2`, `... = 3`, ...
- Query count scales with result-set size. Turn on SQL logging (Rails `ActiveRecord` logger, Django `django.db.backends` DEBUG, Prisma `log: ['query']`, Hibernate `show_sql`) and count.
- Use a detector: Rails `bullet` gem, Django `nplusone` / Debug Toolbar, an APM (Datadog/NewRelic) flame graph showing hundreds of identical spans.

How to FIX:
1. Eager-load the association (the ORM emits a JOIN or a second batched query):
   - Rails: `Post.includes(:comments)` (or `preload`/`eager_load`).
   - Django: `.select_related('author')` (FK, single JOIN) / `.prefetch_related('comments')` (M2M/reverse FK, batched `IN`).
   - SQLAlchemy: `selectinload(Post.comments)` / `joinedload(...)`.
   - Prisma: `include: { comments: true }`. EF Core: `.Include(p => p.Comments)`.
2. Or write the JOIN yourself and read children off the joined rows.
3. Or batch manually: collect all parent ids, fetch children in ONE query, group in memory.

```sql
-- N+1 (conceptually): 1 + N
SELECT id FROM posts LIMIT 100;            -- 1 query
SELECT * FROM comments WHERE post_id = ?;  -- run 100 times

-- Fixed: 2 queries total (batched IN)
SELECT id FROM posts LIMIT 100;
SELECT * FROM comments WHERE post_id IN (/* the 100 ids */);

-- Or 1 query with a JOIN
SELECT p.*, c.*
FROM posts p
LEFT JOIN comments c ON c.post_id = p.id
WHERE p.id IN (/* ... */);
```

- Beware the JOIN-explosion trade-off: a single JOIN across multiple one-to-many relations multiplies rows (cartesian-ish) and ships duplicate parent data. For multiple collections, `prefetch`/`selectinload` (separate batched queries) is often better than one giant JOIN.
- N+1 also hides in serializers/GraphQL resolvers (each field resolves a query). Use DataLoader-style batching there.

COMMON PITFALLS:
- "It's fast locally" — local DB has 10 rows and ~0ms latency. Prod has 1M rows and 1–5ms network per query; 500 queries = seconds.
- Adding eager-loading but still triggering lazy loads on a relation you forgot to include — profile after the fix.
- Eager-loading EVERYTHING always — you fetch data you don't use. Load what the request needs.

## Migrations: Always Use Migration Files

- ALL schema changes (and structural data changes) go through versioned migration files checked into the repo. Never run ad-hoc `ALTER`/`CREATE`/`DROP` against production by hand.
- Why: reproducible across dev/staging/prod, code-reviewed, rollback-able, ordered, and the schema's history is documented. Hand edits drift environments and are invisible to teammates.
- Use a real migration tool: Rails ActiveRecord migrations, Django migrations, Alembic (SQLAlchemy), Prisma Migrate, Flyway, Liquibase, golang-migrate, Knex, Sqitch. The tool tracks which migrations have run in a `schema_migrations`/`migrations` table.
- One logical change per migration; small and focused. Name them descriptively (`add_status_to_orders`).
- Test the migration on a production-like copy (real data volume) before prod — especially for index builds and `NOT NULL`/type changes that can lock big tables.
- Long index builds: Postgres `CREATE INDEX CONCURRENTLY` (no full-table write lock; can't run inside a transaction). MySQL: prefer online DDL / `pt-online-schema-change`/`gh-ost` for big tables.
- NEVER edit a migration that has already run in any shared environment — write a new one. Editing applied migrations desyncs everyone.
- Keep DDL out of application startup code; run migrations as an explicit, gated deploy step.

COMMON PITFALLS:
- Hand-running `ALTER TABLE` in a prod console "just this once" → next deploy's migrations conflict, or another env is missing the change.
- A blocking migration on a huge table during peak traffic → table lock, request timeouts, outage. Use concurrent/online DDL and off-peak windows.
- Committing the migration but not the corresponding model/code change (or vice versa) → broken deploy.

## Migrations: Reversibility, Backward-Compatible Deploys (Expand/Contract), Seed Data

- Make migrations reversible. Provide a `down`/rollback (Rails `change`/`down`, Alembic `downgrade`). Test the down. Some ops are irreversible (dropping a column loses data) — call that out explicitly and snapshot/backup first.
- Decouple schema changes from code deploys. In rolling/zero-downtime deploys, old and new app code run SIMULTANEOUSLY against ONE schema. Every migration must be compatible with the currently-running code.

EXPAND / CONTRACT (a.k.a. parallel change) — how to make breaking changes safely across multiple deploys:
1. EXPAND: add the new structure additively (new nullable column / new table / new index). Old code ignores it; nothing breaks.
2. MIGRATE + DUAL-WRITE: deploy code that writes BOTH old and new, and backfill existing rows (batched, throttled). Reads can start preferring new.
3. SWITCH READS: deploy code that reads from the new structure.
4. CONTRACT: once nothing references the old structure, a later migration drops the old column/table/constraint.

Examples of changes that REQUIRE expand/contract (never do in one step on a live system):
- Renaming a column → add new, dual-write, backfill, switch reads, drop old. (A raw `RENAME COLUMN` instantly breaks any running old code referencing the old name.)
- Adding a `NOT NULL` column to a big/used table → add nullable + default, backfill, then `SET NOT NULL` (validate). A bare `NOT NULL` add can lock and/or reject old inserts.
- Changing a column type → add new typed column, dual-write/backfill, switch, drop.
- Splitting/merging tables.

```sql
-- EXPAND: additive, safe, non-blocking
ALTER TABLE users ADD COLUMN email_verified BOOLEAN NOT NULL DEFAULT false;
-- (PG11+ adds a constant-default column without a full rewrite)

-- Backfill in batches to avoid one giant locking UPDATE:
UPDATE users SET email_verified = true
WHERE id BETWEEN 1 AND 10000 AND legacy_verified = 1;
-- repeat across id ranges

-- CONTRACT (a later, separate migration, after code no longer uses it):
ALTER TABLE users DROP COLUMN legacy_verified;
```

- Adding a `NOT NULL`/`CHECK`/FK on existing data: add it `NOT VALID` first (fast, no full scan/lock), then `VALIDATE CONSTRAINT` separately (scans without blocking writes) — Postgres.
- SEED DATA: keep reference/lookup data (roles, countries, plan tiers) in idempotent seed scripts or data migrations using `UPSERT` (`INSERT ... ON CONFLICT DO UPDATE`) so re-running is safe. Keep seeds separate from schema migrations; never seed large fixture data into prod by accident.

COMMON PITFALLS:
- `RENAME COLUMN` / `DROP COLUMN` in a single deploy on a live app → old pods 500 instantly. Use expand/contract.
- One unbounded `UPDATE` to backfill millions of rows → long lock, replication lag, bloat. Batch + throttle.
- Irreversible migration with no backup → a bad deploy can't be rolled back and data is gone.
- Non-idempotent seeds → re-running duplicates rows. Use UPSERT and natural keys.

## Transactions: ACID, When to Use, Keeping Them Short

- A transaction groups statements into an all-or-nothing unit. Use one whenever multiple writes must succeed or fail TOGETHER to preserve an invariant (the classic money transfer: debit one account, credit another).
- ACID:
  - Atomicity — all statements commit or none do (`ROLLBACK` undoes the batch).
  - Consistency — constraints (FK/UNIQUE/CHECK/NOT NULL) hold at commit.
  - Isolation — concurrent transactions don't see each other's uncommitted, partial state (governed by the isolation level).
  - Durability — once committed, it survives a crash.

```sql
BEGIN;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;
UPDATE accounts SET balance = balance + 100 WHERE id = 2;
-- both succeed or neither does
COMMIT;   -- ROLLBACK on any error
```

- Isolation levels (weak→strong): `READ COMMITTED` (Postgres default — no dirty reads), `REPEATABLE READ` (Postgres also prevents non-repeatable reads; MySQL/InnoDB default), `SERIALIZABLE` (strongest; behaves as if transactions ran one at a time — may abort with a serialization error you must retry). Raise the level only for the operations that need it.
- Keep transactions SHORT. A transaction holds locks and (in Postgres MVCC) pins old row versions, blocking `VACUUM` and bloating the DB. Long transactions cause lock pileups and replication lag.
  - Do all reads/computation/validation you can BEFORE `BEGIN`.
  - NEVER do slow/external work inside a transaction: no HTTP/API calls, no LLM calls, no file uploads, no `sleep`, no waiting on user input. Keep the open window to the minimum SQL.
  - Don't hold a transaction open across an ORM lazy-load that fans out into many queries.
- One unit of work = one connection's transaction. Don't interleave unrelated work.
- Autocommit: outside an explicit `BEGIN`, each statement is its own transaction. Fine for single-statement writes; wrap multi-statement invariants explicitly.

COMMON PITFALLS:
- Doing a payment-gateway/HTTP call inside `BEGIN ... COMMIT` → the transaction stays open for the call's full latency, holding locks and exhausting the connection pool under load.
- "Transaction" that's actually several separate auto-committed statements → a mid-sequence failure leaves half-applied, inconsistent data.
- Cranking everything to `SERIALIZABLE` and not handling the serialization-failure retry → random user-facing errors.

## Transactions: Deadlocks and the Read-Modify-Write Race

DEADLOCK: two transactions each hold a lock the other needs, so the DB kills one (`deadlock detected`). Avoid:
- Always acquire locks/rows in a CONSISTENT ORDER across the codebase (e.g. always update the lower account id first). Out-of-order locking is the #1 deadlock cause.
- Keep transactions short and touch as few rows as possible.
- Use a single atomic statement instead of multiple where possible.
- Expect occasional deadlocks anyway — wrap the unit of work in a RETRY with small backoff (also handles `SERIALIZABLE` serialization failures).

THE READ-MODIFY-WRITE RACE (lost update): read a value, change it in app code, write it back — two concurrent runs both read the old value and one overwrites the other's update.

```sql
-- BROKEN under concurrency: two sessions both read 5, both write 4 -> lost a decrement
SELECT stock FROM products WHERE id = 1;   -- app sees 5
-- ... app computes 5 - 1 = 4 ...
UPDATE products SET stock = 4 WHERE id = 1;
```

Three correct fixes:
1. ATOMIC UPDATE — let the DB do the arithmetic in one statement (best when you can express it in SQL):
```sql
UPDATE products SET stock = stock - 1
WHERE id = 1 AND stock >= 1;   -- also enforces "no oversell"; check affected rows = 1
```
2. PESSIMISTIC LOCK — `SELECT ... FOR UPDATE` locks the row until commit so the second reader waits:
```sql
BEGIN;
SELECT stock FROM products WHERE id = 1 FOR UPDATE;  -- row locked
-- compute new value in app
UPDATE products SET stock = $new WHERE id = 1;
COMMIT;
```
3. OPTIMISTIC LOCK — a `version`/`updated_at` column; the write only applies if the row is unchanged, else retry:
```sql
UPDATE products SET stock = $new, version = version + 1
WHERE id = 1 AND version = $version_read;   -- 0 rows affected => someone else won => retry
```
- Pessimistic = fewer retries, more blocking/contention. Optimistic = no locks held, but you must handle the retry. Atomic single-statement is simplest when applicable.

COMMON PITFALLS:
- Counters/balances/inventory updated via read-then-write in app code → lost updates, oversells, double-spends under load.
- `SELECT FOR UPDATE` without a transaction (autocommit) → the lock releases immediately, doing nothing.
- Inconsistent lock ordering across endpoints → intermittent deadlocks that only appear under concurrency.
- No retry on `deadlock detected` / serialization failure → user sees a 500 for a transient, retryable condition.

## Query Safety: Always Parameterize (Never String-Concat — SQL Injection)

- ALWAYS pass user/untrusted input as bound PARAMETERS, never by building SQL strings. This is the single most important security rule. String concatenation = SQL injection = full DB read/write/destroy.

```python
# CATASTROPHIC: string interpolation -> SQL injection
cur.execute(f"SELECT * FROM users WHERE email = '{email}'")
#   email = "x' OR '1'='1' --"  dumps every user
#   email = "x'; DROP TABLE users; --"  ...

# CORRECT: parameterized (driver sends value separately; it can never be parsed as SQL)
cur.execute("SELECT * FROM users WHERE email = %s", (email,))   # psycopg
```

- Parameter placeholders vary by driver: `?` (SQLite, JDBC, many), `%s` (psycopg/MySQL Python), `$1, $2` (Postgres native / Go `pgx`), `:name` (named). Use whatever your driver wants — the point is the value travels OUT of the SQL string.
- This applies to EVERY value: `WHERE`, `INSERT`/`UPDATE` values, `LIMIT`, `LIKE` patterns, `IN (...)` lists. For `IN`, generate one placeholder per element (`IN ($1,$2,$3)`) or use an array param (`= ANY($1)` in Postgres) — do NOT concatenate the list.
- IDENTIFIERS (table/column names, `ASC/DESC`, sort columns) usually can't be bound as parameters. NEVER interpolate them raw from user input. Validate against a strict ALLOWLIST of known-good names, then inline only the allowlisted value (or use the driver's safe identifier quoting, e.g. psycopg `sql.Identifier`).
- ORMs and query builders parameterize for you — but `raw`/`exec`/string-fragment escape hatches do NOT. The moment you build raw SQL with `+`/f-strings/template literals, you own the injection risk.
- Defense in depth: least-privilege DB user (the app role shouldn't be a superuser), but parameterization is the actual fix — input validation alone is not.

COMMON PITFALLS:
- "I escaped the quotes myself" → escaping is error-prone and bypassable (encodings, unicode, comment tricks). Let the driver bind parameters.
- ORDER BY / column name taken from a query string and concatenated → injectable. Allowlist it.
- Building a dynamic `IN` clause by joining strings → injectable and unparameterized. Use placeholders/array params.

## Query Performance: Avoid SELECT *, LIMIT Big Tables, No Functions on Indexed Columns

- Select only the columns you need. `SELECT *`:
  - Ships unused data (wide rows, large `TEXT`/`JSONB`/`BLOB`) over the wire.
  - Defeats covering/index-only scans (forces a heap fetch for columns the index didn't have).
  - Breaks silently when columns are added/reordered, and makes ORM hydration heavier.
  - Use explicit column lists in application queries. (`SELECT *` is fine for quick ad-hoc exploration.)

```sql
-- DON'T
SELECT * FROM users WHERE id = $1;
-- DO
SELECT id, email, display_name FROM users WHERE id = $1;
```

- LIMIT result sets on large tables. Never fetch "all rows" into the app for a list view. Always paginate.
  - Prefer KEYSET (seek) pagination over large `OFFSET`: `OFFSET 1000000` still scans and discards a million rows.
```sql
-- Slow on deep pages:
SELECT id, title FROM posts ORDER BY id LIMIT 20 OFFSET 1000000;
-- Fast keyset: pass the last id you saw
SELECT id, title FROM posts WHERE id > $last_id ORDER BY id LIMIT 20;
```
- Don't wrap an INDEXED column in a function/cast/expression in `WHERE` — it makes the index unusable (the planner can't match `f(col)` to an index on `col`):
```sql
-- Index on email NOT used (function on the column):
WHERE lower(email) = 'a@b.com'
-- Fix A: create an EXPRESSION index to match the query
CREATE INDEX idx_users_lower_email ON users (lower(email));
-- Fix B: store/compare in a normalized form, or use citext
```
- Same trap for dates: `WHERE date(created_at) = '2026-06-27'` ignores an index on `created_at`. Use a RANGE instead:
```sql
WHERE created_at >= '2026-06-27' AND created_at < '2026-06-28'
```
- Leading-wildcard `LIKE '%term%'` can't use a B-tree — use trigram (`pg_trgm` GIN) or full-text search. Prefix `LIKE 'term%'` CAN use a B-tree.
- Avoid implicit type casts in predicates (`WHERE varchar_col = 123`) — they can suppress index use and cause surprising comparisons. Pass the matching type.
- `OR` across different columns often can't use a single index; sometimes a `UNION` of two index-friendly queries is faster.

COMMON PITFALLS:
- `SELECT *` in hot endpoints → bloated payloads, no index-only scans, broken assumptions when schema changes.
- Functions/casts on indexed columns in `WHERE` → silent full scans despite "having an index."
- Deep `OFFSET` pagination → gets slower the further the user pages. Use keyset.
- Fetching everything then filtering/sorting in the app → moves the DB's job into slow application memory.

## Postgres vs MySQL vs SQLite: When Each, and What Bites

WHEN TO USE:
- PostgreSQL — the default for new server applications. Richest features (advanced types, `JSONB`, arrays, CTEs, window functions, partial/expression/GIN indexes, strict standards compliance, robust concurrency via MVCC, extensions like PostGIS). Reach for it unless you have a specific reason not to.
- MySQL / MariaDB — mature, ubiquitous, great read scaling and replication ecosystem; fine for typical web apps, especially where the team/host already runs it. InnoDB is the engine to use (transactional). Historically simpler feature set than Postgres (catching up).
- SQLite — an embedded, single-file, serverless library. Ideal for local dev, tests, CLI tools, desktop/mobile apps, edge, and low-to-moderate-concurrency single-node services. NOT for high write concurrency (a write takes a database-level lock) or multi-server setups sharing one file.

KEY DIFFERENCES THAT BITE:
- Types & strictness:
  - SQLite has dynamic typing ("type affinity") — by default it will happily store a string in an `INTEGER` column. Use `STRICT` tables (modern SQLite) to enforce types.
  - SQLite has no real `BOOLEAN` (uses 0/1), limited `ALTER TABLE` (can't drop/alter many things historically — you rebuild the table), and stores dates as TEXT/REAL/INT (no native date type).
  - MySQL: watch `utf8` — it's only 3-byte and breaks emoji/some characters. ALWAYS use `utf8mb4`. Beware historic implicit truncation/zero-dates unless `STRICT`/`sql_mode` is set properly.
- Auto-increment / identity: Postgres `GENERATED AS IDENTITY` (or `SERIAL`), MySQL `AUTO_INCREMENT`, SQLite `INTEGER PRIMARY KEY [AUTOINCREMENT]`.
- Upsert syntax: Postgres/SQLite `INSERT ... ON CONFLICT (...) DO UPDATE`; MySQL `INSERT ... ON DUPLICATE KEY UPDATE` (or `INSERT IGNORE`). Not portable — don't assume.
- Case sensitivity: Postgres string comparison is case-sensitive by default (use `citext`/`lower()`); MySQL's default collations are often case-INSENSITIVE — the same query can return different results across engines. MySQL identifiers' case sensitivity also depends on the OS filesystem.
- Booleans: Postgres real `BOOLEAN`; MySQL `BOOL` is an alias for `TINYINT(1)`; SQLite uses 0/1.
- DDL transactions: Postgres can run most DDL inside a transaction (and roll it back). MySQL implicitly commits on DDL (no rollback of an `ALTER`). Plan migrations accordingly.
- FK indexing: MySQL/InnoDB auto-indexes FK columns; Postgres and SQLite do NOT — you must add them.
- Concurrency model: Postgres/MySQL-InnoDB use MVCC (readers don't block writers). SQLite serializes writers (one writer at a time); enable WAL mode to let reads proceed during a write.
- FK enforcement in SQLite is OFF by default — you must `PRAGMA foreign_keys = ON;` per connection or constraints are ignored.

COMMON PITFALLS:
- Writing SQLite in tests but Postgres in prod and relying on engine-specific behavior (type leniency, different collation/casing, upsert syntax) → "works on my machine," fails in prod. Test against the prod engine for anything non-trivial.
- MySQL `utf8` (3-byte) silently mangling emoji/4-byte chars → use `utf8mb4` everywhere.
- Assuming Postgres-style `ON CONFLICT` works on MySQL (it doesn't) or that DDL is transactional on MySQL (it isn't).
- Forgetting `PRAGMA foreign_keys=ON` in SQLite → FK constraints silently do nothing.

## Common Amateur Mistakes (Checklist to Audit Any Schema/Query)

Run through this list on any database you design or review:
- [ ] FOREIGN KEYS not indexed → slow JOINs + delete storms. Add an index on every FK column (except MySQL/InnoDB, which does it for you).
- [ ] STRING-CONCATENATED queries → SQL injection. Parameterize everything; allowlist identifiers.
- [ ] N+1 QUERIES (query in a loop) → use eager loading / JOIN / batched `IN`. The top ORM perf killer.
- [ ] MONEY AS FLOAT → rounding errors. Use integer minor units (cents) or `DECIMAL/NUMERIC`.
- [ ] NO MIGRATIONS / hand-edited prod schema → drift and broken deploys. Use versioned, reversible migration files.
- [ ] `SELECT *` EVERYWHERE → bloated payloads, no index-only scans, fragile to schema changes. List columns explicitly.
- [ ] MISSING `NOT NULL` → null bugs and broken aggregates. Default columns to `NOT NULL` + sensible `DEFAULT`.
- [ ] NO `created_at`/`updated_at` → can't debug or audit. Add both (`updated_at` via trigger/ORM hook, not just `DEFAULT`).
- [ ] `ALTER`-ING PROD DIRECTLY / breaking change in one deploy → outage. Use expand/contract and online/concurrent DDL.
- [ ] EVERYTHING IS `TEXT` → no validation, bad indexing. Use precise types (timestamptz, boolean, numeric, uuid, jsonb).
- [ ] MUTABLE NATURAL KEY (email) as PK → cascade chaos when it changes. Use a surrogate PK (bigint identity / uuid); keep natural keys `UNIQUE`.
- [ ] NO FOREIGN KEY CONSTRAINTS → orphan rows and corrupt joins. Declare FKs with explicit `ON DELETE`.
- [ ] READ-MODIFY-WRITE on counters/balances → lost updates. Use atomic `SET x = x - 1`, `SELECT FOR UPDATE`, or optimistic version checks.
- [ ] LONG TRANSACTIONS / external calls inside `BEGIN..COMMIT` → lock pileups, pool exhaustion, bloat. Keep transactions short; no HTTP/LLM/IO inside.
- [ ] UNBOUNDED QUERIES (no `LIMIT`) / deep `OFFSET` pagination → memory blowups and slow deep pages. Paginate with keyset.
- [ ] FUNCTIONS/CASTS ON INDEXED COLUMNS in `WHERE` (`lower(email)`, `date(created_at)`) → index ignored. Use expression indexes or range predicates.
- [ ] STORING LISTS as CSV/JSON you later filter on → unindexable, slow. Use a child table.
- [ ] OVER-INDEXING a write-hot table / indexing low-cardinality booleans → slow writes for no read benefit. Index by evidence (`EXPLAIN`), drop unused indexes.
- [ ] NO BACKUPS / untested restores, and irreversible migrations with no snapshot → unrecoverable data loss. Back up and test restores.
- [ ] STORING SECRETS/PASSWORDS IN PLAINTEXT → hash passwords (bcrypt/argon2), encrypt sensitive fields. (Not strictly schema, but ships constantly.)
