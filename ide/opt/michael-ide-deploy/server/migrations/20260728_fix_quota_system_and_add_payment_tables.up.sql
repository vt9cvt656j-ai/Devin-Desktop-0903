-- Migration: Add quota weekly cap and fix plan budget logic
-- Date: 2026-07-28
-- Purpose: Fix the mathematical error in quota window system where users could only access 9.2% of their purchased budget

-- Step 1: Add weekly_cap_cents column if it doesn't exist
ALTER TABLE users 
ADD COLUMN IF NOT EXISTS quota_weekly_cap_cents BIGINT DEFAULT 0;

-- Step 2: Set reasonable weekly caps for existing users based on their plans
UPDATE users SET quota_weekly_cap_cents = 
  CASE plan
    WHEN 'trial' THEN 500           -- $5/week
    WHEN 'basic' THEN 5000          -- $50/week  
    WHEN 'pro' THEN 10000           -- $100/week
    WHEN 'power' THEN 30000         -- $300/week
    WHEN 'ultra' THEN 80000         -- $800/week
    ELSE 0
  END
WHERE plan != 'none';

-- Step 3: Mark that we're deprecating the window-based refill mechanism
-- Keep the columns for backward compatibility but set them to zero
UPDATE users SET 
    quota_window_reset_at = NULL,
    quota_window_cents = quota_total_cents  -- Set to total so no artificial limit
WHERE plan != 'none';

-- Step 4: Create indexes for payment_intents table (for Phase 1.2)
CREATE TABLE IF NOT EXISTS payment_intents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID REFERENCES orders(id) ON DELETE CASCADE,
    customer_id TEXT NOT NULL,
    payment_intent_id TEXT NOT NULL UNIQUE,
    amount_cents INT NOT NULL,
    currency TEXT DEFAULT 'usd',
    status TEXT NOT NULL DEFAULT 'created',
    received_webhook_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_payment_intents_order ON payment_intents(order_id);
CREATE INDEX IF NOT EXISTS idx_payment_intents_pi_id ON payment_intents(payment_intent_id);
CREATE INDEX IF NOT EXISTS idx_payment_intents_status ON payment_intents(status) WHERE status = 'pending';

-- Step 5: Create audit_logs table for admin action tracking
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    action TEXT NOT NULL,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created ON audit_logs(created_at DESC);

-- Step 6: Create refresh_tokens table for JWT rotation (Phase 1.4)
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    used_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user ON refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires ON refresh_tokens(expires_at) WHERE expires_at > NOW();

-- Step 7: Clean up old test data (optional, run with caution)
-- DELETE FROM activation_codes WHERE created_at < NOW() - INTERVAL '30 days';
