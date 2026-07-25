-- Admin-managed exchange rates used by the sales profitability calculator.
-- usd_per_cny means: 1 CNY buys this many raw channel USD.
CREATE TABLE IF NOT EXISTS channel_rates (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    usd_per_cny DOUBLE PRECISION NOT NULL CHECK (usd_per_cny > 0),
    note        TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_channel_rates_name_unique
    ON channel_rates (lower(name));
CREATE INDEX IF NOT EXISTS idx_channel_rates_created
    ON channel_rates (created_at, name);
