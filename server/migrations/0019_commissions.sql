-- Commission / affiliate payout ledger for admin operations.
-- A row is an auditable commission claim, usually created after an order is paid,
-- but it can also be created manually for offline sales or partner adjustments.
CREATE TABLE IF NOT EXISTS commissions (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referrer_user_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    referrer_email    TEXT NOT NULL DEFAULT '',
    customer_user_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    customer_email    TEXT NOT NULL DEFAULT '',
    order_id          UUID REFERENCES orders(id) ON DELETE SET NULL,
    source            TEXT NOT NULL DEFAULT 'manual',
    amount_cents      BIGINT NOT NULL DEFAULT 0,
    rate_bps          INTEGER NOT NULL DEFAULT 0,
    commission_cents  BIGINT NOT NULL DEFAULT 0,
    status            TEXT NOT NULL DEFAULT 'pending', -- pending | settled | rejected
    note              TEXT NOT NULL DEFAULT '',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    settled_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_commissions_created ON commissions (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_commissions_status ON commissions (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_commissions_referrer ON commissions (referrer_email, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_commissions_order ON commissions (order_id);
