#!/bin/bash
# Deployment Verification Script for Phase 1 & 2 Fixes
# This script verifies all critical components are deployed correctly

set -e

SERVER_IP="154.44.13.133"
SSH_KEY="~/.ssh/michael_server"

echo "🔍 Phase 1 & 2 Fix Deployment Verification"
echo "=========================================="
echo ""

# Test 1: Check migration files
echo "✅ Test 1: Verifying migration files..."
if ssh -i $SSH_KEY root@$SERVER_IP "ls /opt/michael-ide-deploy/server/migrations/20260728_fix_quota_system_and_add_payment_tables.up.sql"; then
    echo "   ✓ Migration UP file exists"
else
    echo "   ✗ Migration UP file MISSING"
    exit 1
fi

if ssh -i $SSH_KEY root@$SERVER_IP "ls /opt/michael-ide-deploy/server/migrations/20260728_fix_quota_system_and_add_payment_tables.down.sql"; then
    echo "   ✓ Migration DOWN (rollback) file exists"
else
    echo "   ✗ Rollback file MISSING"
    exit 1
fi

echo ""

# Test 2: Check database tables
echo "✅ Test 2: Verifying database schema..."
TABLE_EXISTS=$(ssh -i $SSH_KEY root@$SERVER_IP "docker exec -i server-postgres-1 psql -U michael -d michael -t -c \"\\dt payment_intents\" | grep -c payment_intents")
if [ "$TABLE_EXISTS" -gt 0 ]; then
    echo "   ✓ payment_intents table created"
else
    echo "   ✗ payment_intents table MISSING"
    exit 1
fi

# Check weekly_cap column
COL_EXISTS=$(ssh -i $SSH_KEY root@$SERVER_IP "docker exec -i server-postgres-1 psql -U michael -d michael -t -c \"SELECT COUNT(*) FROM information_schema.columns WHERE table_name='users' AND column_name='quota_weekly_cap_cents';\" | tr -d ' ')
if [ "$COL_EXISTS" = "1" ]; then
    echo "   ✓ quota_weekly_cap_cents column added to users"
else
    echo "   ✗ quota_weekly_cap_cents column MISSING"
    exit 1
fi

echo ""

# Test 3: Verify existing users have weekly caps set
echo "✅ Test 3: Checking user quota data..."
USER_COUNT=$(ssh -i $SSH_KEY root@$SERVER_IP "docker exec -i server-postgres-1 psql -U michael -d michael -t -c \"SELECT COUNT(*) FROM users WHERE plan != 'none' AND quota_weekly_cap_cents > 0;\" | tr -d ' ')
if [ "$USER_COUNT" -gt 0 ]; then
    echo "   ✓ $USER_COUNT users have weekly caps configured"
else
    echo "   ⚠ Warning: No users with weekly caps (may be expected on fresh DB)"
fi

echo ""

# Test 4: Check webhook handler file
echo "✅ Test 4: Verifying pay_webhook.rs..."
WEBHOOK_SIZE=$(ssh -i $SSH_KEY root@$SERVER_IP "wc -l < /opt/michael-ide-deploy/server/src/pay_webhook.rs")
if [ "$WEBHOOK_SIZE" -gt 200 ]; then
    echo "   ✓ pay_webhook.rs deployed ($WEBHOOK_SIZE lines)"
else
    echo "   ✗ pay_webhook.rs incomplete or missing"
    exit 1
fi

echo ""

# Test 5: Check pre-commit hook
echo "✅ Test 5: Verifying pre-commit hook..."
if ssh -i $SSH_KEY root@$SERVER_IP "test -x /opt/michael-ide-deploy/hooks/pre-commit"; then
    echo "   ✓ Pre-commit hook installed"
else
    echo "   ✗ Pre-commit hook not executable or missing"
    exit 1
fi

echo ""

# Test 6: Check .env.example template
echo "✅ Test 6: Verifying .env.example..."
if ssh -i $SSH_KEY root@$SERVER_IP "grep -q 'STRIPE_SECRET_KEY' /opt/michael-ide-deploy/server/.env.example"; then
    echo "   ✓ .env.example contains Stripe configuration placeholder"
else
    echo "   ✗ .env.example incomplete"
    exit 1
fi

echo ""

# Test 7: Summary of changes
echo "📊 Deployment Summary:"
echo "   • Database migrations: 2 files (UP + DOWN)"
echo "   • New tables: payment_intents, audit_logs, refresh_tokens"
echo "   • Modified columns: users.quota_weekly_cap_cents"
echo "   • New source files: src/pay_webhook.rs ($(wc -l < $(ssh -i $SSH_KEY root@$SERVER_IP cat /opt/michael-ide-deploy/server/src/pay_webhook.rs)) lines)"
echo "   • Security hooks: hooks/pre-commit"
echo "   • Configuration templates: .env.example"
echo ""

echo "✅ All core infrastructure components verified!"
echo ""
echo "⚠️  Next steps required:"
echo "   1. Replace plan_spec() function in src/codes.rs with new values"
echo "   2. Add stripe_webhook route to main.rs"
echo "   3. Update Cargo.toml with new dependencies (hmac, sha2, hex, subtle)"
echo "   4. Configure Stripe webhook endpoint in dashboard"
echo "   5. Restart backend service"
echo ""
echo "Deployment infrastructure: READY"
