-- Products / price list the admin sells (a plan tier for N days, or a credit pack).
CREATE TABLE IF NOT EXISTS prices (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    label         TEXT NOT NULL,
    kind          TEXT NOT NULL,                 -- 'plan' | 'credits'
    plan          TEXT,                          -- trial|basic|pro|power|ultra (kind='plan')
    duration_days INTEGER,                       -- (kind='plan')
    credits_cents BIGINT,                        -- credits granted (kind='credits')
    amount_cents  BIGINT NOT NULL,               -- price the buyer pays, USD cents
    active        BOOLEAN NOT NULL DEFAULT true,
    sort          INTEGER NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_prices_active ON prices (active, sort);

-- Orders. A user buys a product; on payment (gateway callback or admin manual
-- confirm) the plan/credits are granted and status flips to 'paid'.
CREATE TABLE IF NOT EXISTS orders (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID REFERENCES users (id) ON DELETE SET NULL,
    email         TEXT NOT NULL DEFAULT '',      -- buyer email snapshot
    price_id      UUID REFERENCES prices (id) ON DELETE SET NULL,
    kind          TEXT NOT NULL,                 -- 'plan' | 'credits'
    plan          TEXT,
    duration_days INTEGER,
    credits_cents BIGINT,
    amount_cents  BIGINT NOT NULL,               -- amount due, USD cents
    status        TEXT NOT NULL DEFAULT 'pending', -- pending | paid | canceled
    method        TEXT NOT NULL DEFAULT 'manual',  -- manual | stripe | epay | alipay | wechat
    note          TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    paid_at       TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_orders_created ON orders (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_orders_status  ON orders (status);
