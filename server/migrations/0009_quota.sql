-- Plan-based quota system (all amounts in USD cents, like credits).
-- total: lifetime pool from the plan; never auto-refills (when 0 → stop).
-- window: per-5h30m allowance; refills to LEAST(window_cap, total) each window.
-- weekly: per-7-day cap (0 = unlimited; counter still resets weekly).
ALTER TABLE users ADD COLUMN IF NOT EXISTS quota_total_cents      BIGINT      NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS quota_window_cap_cents BIGINT      NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS quota_window_cents     BIGINT      NOT NULL DEFAULT 0;  -- current window remaining
ALTER TABLE users ADD COLUMN IF NOT EXISTS quota_window_reset_at  TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS quota_weekly_cap_cents BIGINT      NOT NULL DEFAULT 0;  -- 0 = unlimited
ALTER TABLE users ADD COLUMN IF NOT EXISTS quota_week_used_cents  BIGINT      NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS quota_week_reset_at    TIMESTAMPTZ;
