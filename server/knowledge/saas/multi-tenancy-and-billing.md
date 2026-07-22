# SaaS: Multi-Tenancy, Billing & Platform Infrastructure

## Multi-Tenancy Patterns

### Pattern 1: Shared Schema with tenant_id (Most Common)
```sql
-- Every table has tenant_id; RLS enforces isolation
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free',
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_projects_tenant ON projects(tenant_id);

-- Row-Level Security (PostgreSQL)
ALTER TABLE projects ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON projects
    USING (tenant_id = current_setting('app.current_tenant')::uuid);

-- Set tenant context per request (middleware sets this)
SET app.current_tenant = 'tenant-uuid-here';
-- All queries now automatically filtered — no WHERE clause needed
```

### Middleware Pattern (Node.js / Express)
```javascript
function tenantMiddleware(req, res, next) {
  const tenantId = req.headers['x-tenant-id'] || extractFromSubdomain(req);
  if (!tenantId) return res.status(400).json({ error: 'Tenant required' });

  req.tenantId = tenantId;
  req.db = db.withContext({ tenant_id: tenantId });
  next();
}

function extractFromSubdomain(req) {
  const host = req.hostname; // acme.app.com
  const sub = host.split('.')[0];
  return tenantSlugToId(sub); // cached lookup
}
```

### Django Middleware (Schema-per-Tenant)
```python
from django.db import connection

class TenantMiddleware:
    def __init__(self, get_response):
        self.get_response = get_response

    def __call__(self, request):
        tenant = self._resolve_tenant(request)
        if not tenant:
            return JsonResponse({'error': 'Unknown tenant'}, status=400)

        # Schema-per-tenant: SET search_path
        connection.cursor().execute(
            "SET search_path TO %s, shared", [tenant.schema_name]
        )
        request.tenant = tenant
        return self.get_response(request)

    def _resolve_tenant(self, request):
        host = request.get_host().split(':')[0]
        subdomain = host.split('.')[0]
        return Tenant.objects.filter(slug=subdomain).first()
```

### Pattern Selection Guide
| Pattern | Tenant Count | Data Isolation | Complexity | Cost |
|---------|-------------|---------------|-----------|------|
| Shared schema + RLS | 10-10,000+ | Logical (row-level) | Low | Low |
| Schema-per-tenant | 10-500 | Strong (schema-level) | Medium | Medium |
| Database-per-tenant | 1-50 | Complete (physical) | High | High |

### PgBouncer Gotcha (Transaction Mode)
```
# In transaction pooling mode (default for SaaS scale):
# SET commands are LOCAL to the transaction — they reset when the connection returns to pool

# WRONG: SET then query in separate statements
SET app.current_tenant = 'abc';  -- lost when connection returns to pool
SELECT * FROM projects;          -- might get a different connection!

# RIGHT: SET LOCAL inside a transaction
BEGIN;
SET LOCAL app.current_tenant = 'abc';
SELECT * FROM projects;  -- same connection, tenant context preserved
COMMIT;

# Or: use session-level GUC with a dedicated pool (more expensive)
```

## Billing & Subscription

### Stripe Webhook Handler
```javascript
const stripe = require('stripe')(process.env.STRIPE_SECRET_KEY);

async function handleWebhook(req, res) {
  const sig = req.headers['stripe-signature'];
  let event;
  try {
    event = stripe.webhooks.constructEvent(req.rawBody, sig, process.env.STRIPE_WEBHOOK_SECRET);
  } catch (err) {
    return res.status(400).send(`Webhook Error: ${err.message}`);
  }

  switch (event.type) {
    case 'checkout.session.completed': {
      const session = event.data.object;
      await db.query(
        `UPDATE tenants SET stripe_customer_id = $1, plan = $2, subscription_id = $3 WHERE id = $4`,
        [session.customer, session.metadata.plan, session.subscription, session.metadata.tenant_id]
      );
      break;
    }
    case 'invoice.payment_succeeded': {
      const invoice = event.data.object;
      await db.query(
        `INSERT INTO billing_events (tenant_id, type, amount, currency, stripe_invoice_id, created_at)
         VALUES ($1, 'payment', $2, $3, $4, NOW())`,
        [lookupTenant(invoice.customer), invoice.amount_paid, invoice.currency, invoice.id]
      );
      break;
    }
    case 'invoice.payment_failed': {
      const invoice = event.data.object;
      const tenant = await lookupTenant(invoice.customer);
      if (invoice.attempt_count >= 3) {
        await db.query(`UPDATE tenants SET plan = 'suspended' WHERE id = $1`, [tenant]);
        await sendEmail(tenant, 'payment_failed_final');
      } else {
        await sendEmail(tenant, 'payment_failed_retry');
      }
      break;
    }
    case 'customer.subscription.deleted': {
      const sub = event.data.object;
      await db.query(
        `UPDATE tenants SET plan = 'free', subscription_id = NULL WHERE stripe_customer_id = $1`,
        [sub.customer]
      );
      break;
    }
  }
  res.json({ received: true });
}
```

### Usage-Based Billing
```sql
CREATE TABLE usage_events (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    metric TEXT NOT NULL,       -- 'api_calls', 'storage_bytes', 'compute_seconds'
    quantity BIGINT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    idempotency_key TEXT UNIQUE  -- prevent double-counting on retry
);

CREATE TABLE usage_aggregates (
    tenant_id UUID NOT NULL,
    metric TEXT NOT NULL,
    period TEXT NOT NULL,       -- '2024-01' (monthly billing period)
    total BIGINT NOT NULL DEFAULT 0,
    last_event_id BIGINT,
    PRIMARY KEY (tenant_id, metric, period)
);

-- Aggregate on insert (or batch job)
-- Report to Stripe: stripe.subscriptionItems.createUsageRecord(si_id, { quantity, timestamp, action: 'set' })
```

### Credits / Prepaid Ledger
```sql
-- Append-only ledger (never UPDATE/DELETE rows)
CREATE TABLE credits_ledger (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL,
    amount BIGINT NOT NULL,         -- positive = credit, negative = debit
    balance_after BIGINT NOT NULL,  -- running balance (denormalized for speed)
    reason TEXT NOT NULL,           -- 'purchase', 'usage', 'refund', 'promo', 'expiry'
    reference_id TEXT,              -- Stripe charge ID, usage event ID, etc.
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Debit with balance check (atomic)
CREATE OR REPLACE FUNCTION debit_credits(p_tenant UUID, p_amount BIGINT, p_reason TEXT, p_ref TEXT)
RETURNS BIGINT AS $$
DECLARE
    current_bal BIGINT;
BEGIN
    SELECT balance_after INTO current_bal
    FROM credits_ledger WHERE tenant_id = p_tenant
    ORDER BY id DESC LIMIT 1 FOR UPDATE;

    current_bal := COALESCE(current_bal, 0);
    IF current_bal < p_amount THEN
        RAISE EXCEPTION 'insufficient_credits';
    END IF;

    INSERT INTO credits_ledger (tenant_id, amount, balance_after, reason, reference_id)
    VALUES (p_tenant, -p_amount, current_bal - p_amount, p_reason, p_ref);

    RETURN current_bal - p_amount;
END;
$$ LANGUAGE plpgsql;
```

### Seat-Based Licensing
```javascript
async function addSeat(tenantId, userId) {
  const tenant = await db.getTenant(tenantId);
  const currentSeats = await db.count('memberships', { tenant_id: tenantId, active: true });

  if (currentSeats >= tenant.seat_limit) {
    if (tenant.auto_expand_seats) {
      await stripe.subscriptionItems.update(tenant.seat_si_id, {
        quantity: currentSeats + 1,
        proration_behavior: 'create_prorations',
      });
    } else {
      throw new Error('Seat limit reached. Upgrade your plan.');
    }
  }

  await db.insert('memberships', { tenant_id: tenantId, user_id: userId, active: true });
}
```

## User Management & RBAC

### Org / Team Hierarchy
```sql
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    name TEXT NOT NULL
);

CREATE TABLE memberships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    org_id UUID NOT NULL REFERENCES organizations(id),
    role TEXT NOT NULL DEFAULT 'member',
    -- roles: 'owner','admin','member','viewer','billing'
    invited_by UUID REFERENCES users(id),
    accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, org_id)
);
```

### RBAC with Permission Inheritance
```javascript
const ROLE_HIERARCHY = {
  owner:   ['admin', 'member', 'viewer', 'billing'],
  admin:   ['member', 'viewer'],
  billing: ['viewer'],
  member:  ['viewer'],
  viewer:  [],
};

const ROLE_PERMISSIONS = {
  owner:   ['*'],
  admin:   ['project:create','project:edit','project:delete','member:invite','member:remove','settings:edit'],
  billing: ['billing:view','billing:edit','invoice:download'],
  member:  ['project:create','project:edit','project:view'],
  viewer:  ['project:view'],
};

function hasPermission(userRole, requiredPermission) {
  const perms = ROLE_PERMISSIONS[userRole] || [];
  if (perms.includes('*')) return true;
  if (perms.includes(requiredPermission)) return true;
  // Check inherited roles
  const inherited = ROLE_HIERARCHY[userRole] || [];
  return inherited.some(r => hasPermission(r, requiredPermission));
}
```

### Invitation Flow
```javascript
async function inviteUser(orgId, email, role, invitedBy) {
  const token = crypto.randomBytes(32).toString('hex');
  const tokenHash = crypto.createHash('sha256').update(token).digest('hex');

  await db.insert('invitations', {
    org_id: orgId,
    email,
    role,
    token_hash: tokenHash,  // NEVER store raw token
    invited_by: invitedBy,
    expires_at: new Date(Date.now() + 7 * 86400000),
  });

  await sendEmail(email, 'invitation', {
    link: `${BASE_URL}/invite/${token}`,
    org_name: (await db.getOrg(orgId)).name,
    inviter_name: (await db.getUser(invitedBy)).name,
  });
}

async function acceptInvite(token) {
  const tokenHash = crypto.createHash('sha256').update(token).digest('hex');
  const invite = await db.query(
    `DELETE FROM invitations WHERE token_hash = $1 AND expires_at > NOW() RETURNING *`,
    [tokenHash]
  );
  if (!invite) throw new Error('Invalid or expired invitation');

  await db.insert('memberships', {
    user_id: currentUser.id,
    org_id: invite.org_id,
    role: invite.role,
    invited_by: invite.invited_by,
    accepted_at: new Date(),
  });
}
```

### SSO / SAML Integration
```
SAML Flow:
1. User visits app → app redirects to IdP (Okta/Azure AD) with AuthnRequest
2. IdP authenticates user → sends SAMLResponse (signed XML assertion)
3. App validates signature + conditions → extracts NameID + attributes
4. App creates/updates local user, maps to org based on IdP entity_id
5. Issues session token

SCIM (System for Cross-domain Identity Management):
- IdP pushes user provisioning/deprovisioning to app
- Endpoints: POST /scim/v2/Users, PATCH /scim/v2/Users/:id, DELETE /scim/v2/Users/:id
- Auto-deactivate users removed from IdP (compliance critical)
```

## Feature Flags

### Evaluation Engine
```javascript
async function evaluateFlag(flagKey, context) {
  const flag = await getFlag(flagKey); // cached in Redis
  if (!flag || !flag.enabled) return flag?.default_value ?? false;

  // Priority chain: user override > tenant > plan > percentage > default
  for (const rule of flag.rules) {
    if (rule.type === 'user' && rule.user_ids.includes(context.userId)) {
      return rule.value;
    }
    if (rule.type === 'tenant' && rule.tenant_ids.includes(context.tenantId)) {
      return rule.value;
    }
    if (rule.type === 'plan' && rule.plans.includes(context.plan)) {
      return rule.value;
    }
    if (rule.type === 'percentage') {
      const hash = murmurhash3(flagKey + context.userId) % 100;
      if (hash < rule.percentage) return rule.value;
    }
  }
  return flag.default_value;
}

// Schema
// flags: { key, enabled, default_value, rules: [{type, condition, value}], created_at, updated_at }
```

### Plan-Based Feature Gating
```javascript
const PLAN_FEATURES = {
  free:       { max_projects: 3, max_members: 5, api_rate_limit: 100, features: ['basic'] },
  pro:        { max_projects: 50, max_members: 25, api_rate_limit: 1000, features: ['basic','advanced','api'] },
  enterprise: { max_projects: null, max_members: null, api_rate_limit: 10000, features: ['basic','advanced','api','sso','audit','sla'] },
};

function checkFeature(tenant, feature) {
  const plan = PLAN_FEATURES[tenant.plan];
  if (!plan) return false;
  return plan.features.includes(feature);
}

function checkLimit(tenant, limitKey, currentCount) {
  const plan = PLAN_FEATURES[tenant.plan];
  const limit = plan[limitKey];
  return limit === null || currentCount < limit; // null = unlimited
}
```

## API Versioning

### URL-Based Versioning
```javascript
// /api/v1/projects, /api/v2/projects
app.use('/api/v1', v1Router);
app.use('/api/v2', v2Router);

// Sunset header — warn clients their version is deprecated
app.use('/api/v1', (req, res, next) => {
  res.set('Sunset', 'Sat, 01 Mar 2025 00:00:00 GMT');
  res.set('Deprecation', 'true');
  res.set('Link', '</api/v2>; rel="successor-version"');
  next();
});
```

### Header-Based Versioning
```javascript
function versionMiddleware(req, res, next) {
  const version = req.headers['api-version'] || req.query.api_version || 'latest';
  const resolved = version === 'latest' ? '2024-06-01' : version;

  // Version-specific response transformations
  req.apiVersion = resolved;
  const originalJson = res.json.bind(res);
  res.json = (data) => {
    const transformed = applyVersionTransform(data, resolved, req.path);
    originalJson(transformed);
  };
  next();
}
```

## Analytics & Usage Tracking

### Event Schema
```sql
CREATE TABLE analytics_events (
    id BIGSERIAL,
    tenant_id UUID NOT NULL,
    user_id UUID,
    event_name TEXT NOT NULL,   -- 'page_view', 'feature_used', 'api_call'
    properties JSONB,
    session_id TEXT,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
) PARTITION BY RANGE (created_at);

-- Monthly partitions for performance
CREATE TABLE analytics_events_2024_01 PARTITION OF analytics_events
    FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');

-- Retention: drop old partitions instead of DELETE (instant, no vacuum)
DROP TABLE analytics_events_2023_01;
```

### Tracking Client
```javascript
class Analytics {
  constructor(tenantId, userId) {
    this.tenantId = tenantId;
    this.userId = userId;
    this.queue = [];
    this.flushInterval = setInterval(() => this.flush(), 5000);
  }

  track(eventName, properties = {}) {
    this.queue.push({
      event_name: eventName,
      properties,
      timestamp: Date.now(),
      session_id: this.sessionId,
    });
    if (this.queue.length >= 20) this.flush();
  }

  async flush() {
    if (!this.queue.length) return;
    const batch = this.queue.splice(0);
    await fetch('/api/analytics/batch', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ events: batch }),
    }).catch(() => this.queue.unshift(...batch)); // retry on failure
  }
}
```

## Trial & Onboarding

### Trial Management
```javascript
async function startTrial(tenantId, plan, durationDays = 14) {
  // Abuse prevention
  const existing = await db.query(
    `SELECT COUNT(*) FROM trial_history WHERE tenant_id = $1`, [tenantId]
  );
  if (existing.count > 0) throw new Error('Trial already used');

  await db.query(
    `UPDATE tenants SET plan = $1, trial_ends_at = NOW() + interval '${durationDays} days' WHERE id = $2`,
    [plan, tenantId]
  );
  await db.insert('trial_history', { tenant_id: tenantId, plan, started_at: new Date() });

  // Schedule reminders: day 7 (midpoint), day 12 (2 days left), day 14 (expired)
  await scheduleEmail(tenantId, 'trial_midpoint', durationDays / 2);
  await scheduleEmail(tenantId, 'trial_ending', durationDays - 2);
  await scheduleEmail(tenantId, 'trial_expired', durationDays);
}

// Cron: check expired trials
async function expireTrials() {
  await db.query(
    `UPDATE tenants SET plan = 'free' WHERE trial_ends_at < NOW() AND plan != 'free'`
  );
}
```

### Onboarding State Machine
```javascript
const ONBOARDING_STEPS = [
  { key: 'profile',     required: true,  check: t => !!t.company_name },
  { key: 'invite_team', required: false, check: t => t.member_count > 1 },
  { key: 'first_project', required: true, check: t => t.project_count > 0 },
  { key: 'integration', required: false, check: t => t.integrations.length > 0 },
  { key: 'billing',     required: true,  check: t => !!t.stripe_customer_id },
];

function getOnboardingProgress(tenant) {
  const steps = ONBOARDING_STEPS.map(s => ({
    key: s.key,
    completed: s.check(tenant),
    required: s.required,
  }));
  const requiredDone = steps.filter(s => s.required && s.completed).length;
  const requiredTotal = steps.filter(s => s.required).length;
  return { steps, progress: requiredDone / requiredTotal, complete: requiredDone === requiredTotal };
}
```

## Common LLM Mistakes in SaaS
```
1. Forgetting tenant_id in queries (data leak across tenants)
2. Not using RLS or equivalent — relying on app-level WHERE clauses alone
3. Storing Stripe webhook secrets in client-side code
4. Processing webhooks without signature verification
5. Using floating-point for money (use integer cents or DECIMAL)
6. Not handling subscription state transitions (active→past_due→canceled)
7. Hardcoding plan limits instead of config-driven feature flags
8. Missing idempotency on billing events (double-charging on webhook retry)
9. Exposing internal tenant IDs in URLs (use slugs or public IDs)
10. Not rate-limiting per-tenant (one tenant can DoS the whole platform)
```
