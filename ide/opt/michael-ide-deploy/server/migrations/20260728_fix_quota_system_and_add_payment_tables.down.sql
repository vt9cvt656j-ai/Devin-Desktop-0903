-- Migration Rollback: Revert quota and payment tables changes
-- Date: 2026-07-28
-- Purpose: Emergency rollback if fixes cause issues

-- Drop new tables
DROP TABLE IF EXISTS payment_intents CASCADE;
DROP TABLE IF EXISTS audit_logs CASCADE;
DROP TABLE IF EXISTS refresh_tokens CASCADE;

-- Remove newly added columns (keep original window-based system)
ALTER TABLE users DROP COLUMN IF EXISTS quota_weekly_cap_cents;

-- Restore old behavior comments in code
