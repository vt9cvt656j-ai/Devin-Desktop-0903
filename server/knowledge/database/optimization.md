# Database Optimization & Design

## Schema Design Process
```
1. Produce conceptual model (entities, attributes, relationships)
2. Review for normalization issues BEFORE writing SQL
3. Generate DDL
4. Generate test cases (INSERT/UPDATE/DELETE/SELECT) alongside schema
5. Validate FK consistency and PK coverage
```

## Query Optimization Checklist

Before writing SQL for large datasets:
- Are there unnecessary JOINs?
- Can subqueries be replaced with window functions?
- Is there a missing WHERE clause causing full table scan?
- Check EXPLAIN plan
- Suggest indexes for WHERE/JOIN columns

### Index Strategy
```sql
-- Composite indexes: left-to-right prefix rule
CREATE INDEX idx_users_status_created ON users(status, created_at);
-- Covers: WHERE status = 'active'
-- Covers: WHERE status = 'active' AND created_at > '2024-01-01'
-- Does NOT cover: WHERE created_at > '2024-01-01' (alone)

-- Covering indexes (include non-indexed columns)
CREATE INDEX idx_orders_user ON orders(user_id) INCLUDE (total, status);
```

### Window Functions (Prefer Over Self-Joins)
```sql
-- Running total
SELECT *, SUM(amount) OVER (PARTITION BY user_id ORDER BY created_at) AS running_total
FROM transactions;

-- Row number for pagination
SELECT * FROM (
  SELECT *, ROW_NUMBER() OVER (ORDER BY created_at DESC) AS rn
  FROM posts
) t WHERE rn BETWEEN 21 AND 40;

-- Lag/Lead for comparisons
SELECT *,
  amount - LAG(amount) OVER (PARTITION BY user_id ORDER BY month) AS diff
FROM monthly_spending;
```

## PostgreSQL Specifics
- Use `CREATE INDEX CONCURRENTLY` for zero-downtime index creation
- Use `EXPLAIN (ANALYZE, BUFFERS)` for query analysis
- `pg_stat_statements` for identifying slow queries
- Connection pooling: PgBouncer in transaction mode
- Vacuuming: autovacuum ON, tune `autovacuum_vacuum_scale_factor` for large tables

## Migration Safety Rules

1. NEVER add NOT NULL without default in one migration
   → Add nullable → backfill in batches → add constraint
2. NEVER rename columns directly
   → Add new → copy data → update code → drop old
3. NEVER drop columns without verifying zero usage
4. Large table ALTER: use `pg_repack` or online DDL tools
5. Every migration has a rollback script
6. Test against staging copy of prod data before merging

## SQL Injection Prevention

- ALL queries use parameterized statements / prepared statements
- NEVER concatenate user input into SQL
- NEVER pass LLM output into SQL without parameterization
- Use ORM query builders for complex queries
- Validate input types at API boundary (not in SQL)

## Redis Patterns

```
# Cache-aside
GET key → if nil → query DB → SET key value EX ttl

# Rate limiting (sliding window)
MULTI
  ZADD key timestamp timestamp
  ZREMRANGEBYSCORE key 0 (now - window)
  ZCARD key
EXEC

# Distributed lock
SET lock_key unique_id NX EX 30
# Release: Lua script checking unique_id before DEL
```

## Data Pipeline Patterns

### ETL/ELT Structure
```
1. Extract: source query with incremental markers (updated_at > last_run)
2. Validate: null checks, type validation, range constraints, uniqueness
3. Transform: clean, enrich, aggregate
4. Load: upsert (ON CONFLICT DO UPDATE) for idempotency
5. Verify: row counts, checksums, sample comparison
```

### Spark Best Practices
- Partition by: frequently filtered columns, ~128MB per partition
- Broadcast joins for small tables (< 100MB)
- Cache intermediate DataFrames used in multiple paths
- Never `.collect()` on large datasets
- Check explain plan for unnecessary shuffles
