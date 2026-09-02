# Backend Error Handling & Resilience Patterns

## Structured Error Response Contract

Every API error response MUST follow this envelope:
```json
{
  "status": "error",
  "error_type": "validation|auth|rate_limit|timeout|server|not_found",
  "retryable": false,
  "retry_after_ms": null,
  "message": "Human-readable description",
  "request_id": "uuid"
}
```

Rules:
- 4xx errors are non-retryable (except 429 which includes `retry_after_ms`)
- 5xx errors are retryable
- Never return bare HTTP status codes without a body
- Always include `request_id` for correlation

## Retry Strategy

```
wait_time = (base_delay × 2^attempt) + random(0, jitter_max)
```

| Failure Type | Base Delay | Max Retries | Notes |
|-------------|-----------|-------------|-------|
| 429 Rate Limit | 1-2s | 5-7 | Use `retry_after_ms` from response |
| 5xx Server Error | 2s | 3-5 | Exponential backoff |
| Timeout | 5s fixed | 2-3 | Don't exponential — already slow |
| Auth (401/403) | — | 0 | Never retry |
| Validation (400) | — | 0 | Never retry |

Rules:
- Use tenacity/backoff library — never hand-roll retry loops
- Add jitter to prevent thundering herd
- Log every retry with attempt number and wait time

## Circuit Breaker (Three-State)

```
Closed (normal) → Open (fail fast) → Half-Open (probe)
```

- Track failure rate over sliding window of 10 requests
- Open breaker when failure rate > 50%
- In Open: return fallback immediately, do NOT queue
- After 30s: transition to Half-Open, allow 1 test request
- If test succeeds: close; if fails: re-open for 30s
- Expose breaker state as a metric

## Idempotency

- Every write operation accepts an idempotency key
- If key exists in Redis/DB → return cached result
- Key format: `${user_id}:${operation}:${content_hash}`
- TTL: 24 hours for most operations

## Fault-Tolerant Pipeline

1. Multi-step workflows MUST checkpoint state after each step
2. On failure: resume from last checkpoint, never restart
3. After 3 failed retries → route to dead letter queue with full context
4. Dead letter items include: original input, checkpoint data, error details, retry history

## Logging Contract

Every log entry:
```json
{
  "trace_id": "string",
  "span_id": "string",
  "timestamp": "ISO8601-UTC",
  "service": "string",
  "level": "info|warn|error",
  "message": {},
  "duration_ms": 42
}
```

Rules:
- Structured JSON, not string concatenation
- OpenTelemetry SDK for instrumentation
- Propagate W3C Trace Context headers across all service boundaries
- Paired start/complete events for async operations
