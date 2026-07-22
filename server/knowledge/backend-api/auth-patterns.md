# Authentication & Authorization Patterns

## JWT Implementation Checklist

1. Access token TTL: **15 minutes max**
2. Refresh token TTL: **7 days** with rotation (new refresh token on each use)
3. Store tokens in **HttpOnly, Secure, SameSite=Strict cookies** — NEVER localStorage
4. Always validate the `alg` header server-side — reject `none`
5. Use **PKCE** for all OAuth2 authorization code flows (OAuth 2.1 mandate)
6. Required claims: `iss`, `aud`, `exp`, `iat`, `sub`
7. Validate against JWKS endpoint, not hardcoded secrets

## Managed Auth (Preferred)

NEVER implement password hashing, session management, or MFA from scratch.
Use Auth0 / Cognito / Clerk / Supabase Auth.

Backend's only job:
```
1. Extract Bearer token from Authorization header
2. Validate signature against provider's JWKS endpoint
3. Check exp, iss, aud
4. Attach decoded claims to request context
5. Done — no custom session logic
```

## RBAC Pattern

```
User → Role(s) → Permission(s) → Resource.Action
```

- Roles: admin, editor, viewer (not fine-grained per-endpoint)
- Permissions: `resource:action` format (e.g., `posts:write`, `users:delete`)
- Check at middleware level, not in business logic
- Superuser bypass: single check at the top, never scattered

## API Key Security

- Prefix keys for identification: `sk-live-`, `sk-test-`
- Hash stored keys (SHA-256) — never store plaintext
- Rate limit per key, not just per IP
- Rotate mechanism: create new → migrate → revoke old
- Never log full keys — mask all but prefix + last 4 chars

## OAuth2 Flows

| Flow | Use Case |
|------|----------|
| Authorization Code + PKCE | Web apps, SPAs, mobile |
| Client Credentials | Server-to-server |
| Device Code | CLI tools, IoT |
| NEVER use Implicit flow | Deprecated in OAuth 2.1 |

## Session Security Headers

```
Strict-Transport-Security: max-age=63072000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Content-Security-Policy: default-src 'self'
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
```
