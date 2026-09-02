# Secure Coding Practices (OWASP 2025)

## Core Principle: Treat ALL LLM Output as Untrusted Input

LLM-generated output passed to downstream systems MUST be validated exactly like external user input.

## Seven-Layer Output Validation

```
1. Type check — verify expected data types
2. Length check — enforce maximum lengths
3. Format check — validate against regex (emails, URLs, IDs)
4. Encoding — apply context-specific encoding
5. Sanitization — strip HTML tags, control characters
6. Allowlist — only permit known-safe values for enums
7. Audit log — record raw + sanitized output
```

## OWASP Top 10 for LLM Applications 2025

| # | Risk | Prevention |
|---|------|-----------|
| 1 | Prompt Injection | Input validation + output encoding + sandboxing |
| 2 | Sensitive Info Disclosure | Redact PII from training/context; output filtering |
| 3 | Supply Chain | Pin dependencies; audit third-party models |
| 4 | Data Poisoning | Validate training data; use trusted sources only |
| 5 | Improper Output Handling | See seven-layer validation above |
| 6 | Excessive Agency | Least privilege; human-in-loop for destructive actions |
| 7 | System Prompt Leakage | Don't embed secrets in prompts |
| 8 | Vector/Embedding Vulnerabilities | Validate RAG retrieval results |
| 9 | Misinformation | Ground outputs in verified data |
| 10 | Unbounded Consumption | Rate limit; token budgets; circuit breakers |

## SQL Injection Prevention
```python
# ALWAYS parameterized
cursor.execute("SELECT * FROM users WHERE id = %s", (user_id,))

# NEVER string concatenation
cursor.execute(f"SELECT * FROM users WHERE id = {user_id}")  # VULNERABLE
```

## XSS Prevention
```javascript
// ALWAYS encode output
element.textContent = userInput;  // Safe

// NEVER insert raw HTML
element.innerHTML = userInput;  // VULNERABLE
```

## Command Injection Prevention
```python
# ALWAYS use argument lists
subprocess.run(["ls", "-la", path], check=True)

# NEVER pass through shell
subprocess.run(f"ls -la {path}", shell=True)  # VULNERABLE
os.system(f"rm {path}")  # VULNERABLE
```

## Input Validation Rules
- Validate at system boundaries (API handlers), not deep in business logic
- Allowlist > denylist
- Use Pydantic/Zod models with Field constraints
- Reject unknown fields (`extra = "forbid"`)
- Length limits on all string inputs
- Rate limit per user/IP/API key

## Secrets Management
- Never hardcode secrets in source code
- Use environment variables or secret managers (Vault, AWS SM)
- Rotate credentials regularly
- .env files: never commit, always in .gitignore
- API keys: hash stored keys (SHA-256), prefix for identification
- Logs: mask all but prefix + last 4 chars of keys

## Headers
```
Strict-Transport-Security: max-age=63072000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Content-Security-Policy: default-src 'self'
Referrer-Policy: strict-origin-when-cross-origin
```

## Rate Limiting
- Per-IP: 100 req/min on public endpoints
- Per-user: 1000 req/hr on authenticated endpoints
- Per-endpoint: 10 req/min for expensive operations
- Use sliding window counters in Redis
- Return 429 with Retry-After header
- Alert when any client hits > 50% of their limit
