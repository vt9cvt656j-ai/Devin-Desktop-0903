-- Membership plan + credit balance on the (IDE-shared) user accounts.
ALTER TABLE users ADD COLUMN IF NOT EXISTS plan            TEXT        NOT NULL DEFAULT 'none';   -- none|trial|basic|pro|power|ultra
ALTER TABLE users ADD COLUMN IF NOT EXISTS plan_expires_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS credits_cents   BIGINT      NOT NULL DEFAULT 0;        -- USD balance, in cents

-- Activation codes: either grant a membership plan (for N days) or top up credits.
CREATE TABLE IF NOT EXISTS activation_codes (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code           TEXT NOT NULL UNIQUE,
    kind           TEXT NOT NULL,                     -- 'plan' | 'credits'
    plan           TEXT,                              -- trial|basic|pro|power|ultra  (when kind='plan')
    duration_days  INTEGER,                           -- membership length           (when kind='plan')
    credits_cents  BIGINT,                            -- USD amount in cents          (when kind='credits')
    note           TEXT NOT NULL DEFAULT '',
    status         TEXT NOT NULL DEFAULT 'unused',    -- 'unused' | 'used'
    used_by        UUID REFERENCES users (id) ON DELETE SET NULL,
    used_at        TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_codes_created_at ON activation_codes (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_codes_status     ON activation_codes (status);
