# Microservice Architecture Patterns

## Anti-Pattern Checklist (Review Before Finalizing)

| Anti-Pattern | Check |
|-------------|-------|
| Wrong Cuts | Each service owns a single bounded context? |
| Shared Persistence | Each service has its own database? |
| No API Versioning | All endpoints versioned (URL or header)? |
| Hard-Coded Endpoints | All service URLs configurable (env vars)? |
| No Health Check | Every service exposes `/health`? |
| No API Gateway | Single entry point for external traffic? |
| Cyclic Dependencies | No circular service-to-service calls? |
| Shared Libraries | No cross-service code coupling via shared libs? |

## Service Decomposition Rules

1. Keep core business logic monolithic for low latency
2. Extract: data enrichment, analytics, external integrations as separate services
3. Each service owns its data store — no shared databases
4. Communication: gRPC for internal (40-60% less overhead), REST for external/public
5. Use an orchestrator pattern for multi-step workflows

## Inter-Service Communication

```
Synchronous: gRPC (internal), REST (external)
Asynchronous: Message queue (RabbitMQ/NATS) for event-driven
```

- gRPC: define `.proto` files first, HTTP/2 multiplexing, binary serialization
- Events: at-least-once delivery + idempotent consumers
- Timeouts: always set, cascade-aware (inner < outer)

## Caching Architecture

```
Client → CDN → API Gateway → Redis (L1) → PostgreSQL (L2)
```

| Data Type | Cache TTL | Strategy |
|-----------|-----------|----------|
| User profiles | 5 min | Cache-aside |
| Product catalog | 1 hour | Cache-aside |
| Config/settings | 24 hours | Read-through |
| Auth responses | Never cache | — |
| Search results | 2-5 min | Time-based |

Rules:
- Cache-aside pattern: read cache → on miss query DB → populate cache
- Invalidate on writes via pub/sub
- Add jitter to TTLs: `TTL + random(0, TTL * 0.1)` to prevent thundering herd
- Monitor hit rate — target > 80% for read-heavy endpoints

## Database Migration Safety

1. NEVER add NOT NULL columns without a default in a single migration
   → Add nullable first → backfill → then add constraint
2. Always use `CREATE INDEX CONCURRENTLY` (PostgreSQL)
3. Every migration MUST have a rollback
4. Flag ALTER TABLE on tables > 1M rows for manual review
5. Test migrations against staging copy of production data before merging

## GraphQL Rules

- Every field and type MUST have a description
- Use semantic names, not abbreviations
- Enable introspection
- Version with inline `@deprecated(reason:)`, not URL versioning
- Each resolver self-contained — never chain two tools for one logical operation
