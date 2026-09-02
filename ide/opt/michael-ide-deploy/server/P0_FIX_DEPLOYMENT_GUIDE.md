# Phase 1 P0 Emergency Fixes - Deployment Guide

## Overview
This document describes the deployment process for critical security and business logic fixes.

## Changes Summary

### 1. Quota System Fix (5h30m Window Bug)
**Problem**: Users only received 9.2% of purchased budget due to incorrect window calculation
**Solution**: Replaced window-based system with weekly caps, users can now access 100% of their budget
**Impact**: All付费用户 benefit from correct quota allocation

### 2. Payment Gateway Integration
**Problem**: Manual order confirmation allowed internal fraud
**Solution**: Stripe webhook integration for automatic payment confirmation
**Impact**: Eliminates manual intervention in payment flow

### 3. Security Improvements
- Pre-commit hook prevents accidental secret commits
- .env.example template provided (no real secrets)
- Docker secrets structure created

### 4. JWT Refresh Mechanism
**Problem**: 30-day token lifetime made revocation slow
**Solution**: Added refresh token rotation and short-lived access tokens (1 hour)

## Deployment Steps

### Step 1: Database Migration (Run first!)
```bash
cd /opt/michael-ide-deploy/server
docker exec -i michael-postgres-1 psql -U michael -d michael < migrations/20260728_fix_quota_system_and_add_payment_tables.up.sql
```

Verify migration:
```bash
docker exec -it michael-postgres-1 psql -U michael -d michael -c "\dt payment_intents"
docker exec -it michael-postgres-1 psql -U michael -d michael -c "SELECT column_name FROM information_schema.columns WHERE table_name = 'users' AND column_name = 'quota_weekly_cap_cents'"
```

### Step 2: Code Changes Deployment

The following files need to be updated on the server:

1. **codes.rs**: Replace `plan_spec()` function with new logic
2. **pay.rs**: Add webhook endpoint handler  
3. **auth.rs**: Add Redis blacklist check (future phase)
4. **main.rs**: Register new routes

Copy modified files to server:
```bash
scp src/codes.rs root@154.44.13.133:/opt/michael-ide-deploy/server/src/
scp src/pay_webhook.rs root@154.44.13.133:/opt/michael-ide-deploy/server/src/
```

### Step 3: Install Dependencies

Add missing Rust crates to `Cargo.toml`:
```toml
[dependencies]
# ... existing deps ...
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
subtle = "2.5"
```

Build and deploy:
```bash
cd /opt/michael-ide-deploy/server
cargo build --release
systemctl restart michael-backend  # Or docker compose restart backend
```

### Step 4: Configure Stripe Webhooks

1. Log into Stripe Dashboard
2. Navigate to Developers → Webhooks
3. Add endpoint: `https://code.mrday.one/api/webhooks/stripe`
4. Select events:
   - `payment_intent.succeeded`
   - `payment_intent.payment_failed`
   - `payment_intent.canceled`
5. Copy webhook signing secret to `.env` or secrets manager

### Step 5: Install Pre-commit Hook

```bash
cd /opt/michael-ide-deploy
cp hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

### Step 6: Verify Everything

Test quota fix:
```bash
curl -X POST https://code.mrday.one/api/admin/users/{user_id}/grant \
  -H "Authorization: Bearer {admin_token}" \
  -H "Content-Type: application/json" \
  -d '{
    "kind": "plan",
    "plan": "basic",
    "duration_days": 30
  }' | jq '.user.quota_total_cents, .user.quota_weekly_cap_cents'
```

Expected: `quota_total_cents: 33000`, `quota_weekly_cap_cents: 5000`

## Rollback Plan

If issues occur:

```bash
# 1. Revert code changes
cd /opt/michael-ide-deploy
git checkout HEAD~1

# 2. Run database rollback
docker exec -i michael-postgres-1 psql -U michael -d michael < migrations/20260728_fix_quota_system_and_add_payment_tables.down.sql

# 3. Restart services
docker compose restart backend
```

## Monitoring

Watch for errors:
```bash
# Backend logs
tail -f /var/log/michael-backend/error.log

# Database queries
docker exec -it michael-postgres-1 psql -U michael -d michael -c "SELECT count(*) FROM payment_intents WHERE status = 'pending';"

# Audit log (manual confirmations)
docker exec -it michael-postgres-1 psql -U michael -d michael -c "SELECT action, count(*) FROM audit_logs GROUP BY action;"
```

## Success Criteria

✅ Users can spend 100% of plan budget within 30 days
✅ No manual order confirmations needed (>99% automated via webhooks)
✅ Zero failed secret commits since pre-commit hook installation
✅ JWT revocation takes effect within 1 minute (when Redis blacklist implemented)

## Next Steps

After P0 fixes are stable:
1. Implement Phase 1.2: Redis token blacklist for instant JWT revocation
2. Implement Phase 2.1: Fix prompt cache poisoning bug
3. Implement Phase 2.2: Force tool schema budget enforcement
4. Add comprehensive tests for quota calculation

## Contact

For issues or questions:
- Primary: [Your name/email]
- Secondary: DevOps team
- Emergency: PagerDuty channel #michael-ide-critical
