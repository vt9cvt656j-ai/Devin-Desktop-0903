-- Stripe card checkout, replacing manual order confirmation.
--
-- The catalogue stays in `prices` so the admin console remains the single place a
-- product is edited; these columns only add what Stripe needs in order to charge for
-- a row, and what the billing page needs in order to describe it. Nothing here is
-- hardcoded in Rust: changing a price, a plan mapping or a credit grant is a row
-- update, not a redeploy.

ALTER TABLE prices
    ADD COLUMN IF NOT EXISTS stripe_price_id  TEXT,
    -- Stripe's own lookup_key. The client asks for a product by this, never by
    -- price id, so a rotated price in Stripe does not break the buttons.
    ADD COLUMN IF NOT EXISTS lookup_key       TEXT,
    -- amount_cents holds RMB fen (the existing row is ¥9.90 = 990 despite the 2026-06
    -- column comment claiming USD). USD is listed separately because the operator sets
    -- each tier's dollar price by hand rather than converting at a fixed rate.
    ADD COLUMN IF NOT EXISTS amount_usd_cents BIGINT,
    ADD COLUMN IF NOT EXISTS recurring        BOOLEAN NOT NULL DEFAULT false,
    -- Day passes are sold once per account and never renewed.
    ADD COLUMN IF NOT EXISTS once_per_account BOOLEAN NOT NULL DEFAULT false,
    -- Set only for the pay-what-you-want top-up, where the buyer picks a quantity and
    -- each unit grants this many credits_cents.
    ADD COLUMN IF NOT EXISTS unit_credits_cents BIGINT,
    ADD COLUMN IF NOT EXISTS blurb            TEXT NOT NULL DEFAULT '';

-- Partial unique: legacy rows have no lookup_key and must stay insertable.
CREATE UNIQUE INDEX IF NOT EXISTS idx_prices_lookup_key
    ON prices (lookup_key) WHERE lookup_key IS NOT NULL;

ALTER TABLE users ADD COLUMN IF NOT EXISTS stripe_customer_id TEXT;
CREATE INDEX IF NOT EXISTS idx_users_stripe_customer ON users (stripe_customer_id);

ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS stripe_session_id      TEXT,
    ADD COLUMN IF NOT EXISTS stripe_subscription_id TEXT,
    ADD COLUMN IF NOT EXISTS quantity               INTEGER NOT NULL DEFAULT 1;

-- One order per Checkout Session: the webhook and the browser return can both try to
-- fulfil, and a retried webhook delivery must not create a second order.
CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_stripe_session
    ON orders (stripe_session_id) WHERE stripe_session_id IS NOT NULL;

-- Webhook idempotency. Stripe retries until it gets a 2xx, so "have I already acted on
-- this event id" has to be a durable fact, not an in-memory set — otherwise a restart
-- mid-retry grants a plan twice.
CREATE TABLE IF NOT EXISTS stripe_events (
    id          TEXT PRIMARY KEY,
    type        TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- Catalogue seed.
--
-- Plan rows map onto the tiers codes::apply_plan already knows (trial/basic/pro/
-- power/ultra) so a Stripe purchase grants exactly what a redeem code or an admin
-- grant does. The operator-facing names differ from the internal tier names:
--   Daily Pass -> trial (1 day)   Starter -> basic   Power -> power   Elite -> ultra
--
-- Credit rows: the operator prices credits at 22 per US dollar, and the gateway
-- stores balances in raw cents where 663 = $1.00, so one credit is 663/22 ≈ 30.14
-- raw cents. Each pack's credit count is taken from the operator's own table (the
-- larger packs include a bonus over the flat rate) and converted at that rate.
-- ---------------------------------------------------------------------------

INSERT INTO prices
    (label, kind, plan, duration_days, credits_cents, amount_cents, amount_usd_cents,
     stripe_price_id, lookup_key, recurring, once_per_account, unit_credits_cents, blurb, active, sort)
VALUES
    ('Daily Pass', 'plan', 'trial', 1, NULL, 990, 149,
     'price_1Tqry67htKzrqxMwRVc6gIsR', 'daily_trial', false, true, NULL,
     'A full day of the paid quota. One per account, never auto-renews.', true, 10),

    ('Starter', 'plan', 'basic', 30, NULL, 8800, 1299,
     'price_1Tqs0P7htKzrqxMw4V1XP5yj', 'starter_monthly', true, false, NULL,
     'For everyday work: a monthly quota that refills through the day.', true, 20),

    ('Power', 'plan', 'power', 30, NULL, 18800, 2799,
     'price_1Tqs1S7htKzrqxMwvHAGWYLv', 'power_monthly', true, false, NULL,
     'For heavy sessions: several times the Starter quota and a larger burst window.', true, 30),

    ('Elite', 'plan', 'ultra', 30, NULL, 48800, 7199,
     'price_1Tqs1y7htKzrqxMwz05dLxxl', 'elite_monthly', true, false, NULL,
     'The largest quota, for running long agent jobs without watching the meter.', true, 40),

    ('Package A', 'credits', NULL, NULL, 3014, 3000, 450,
     'price_1Tqs3b7htKzrqxMwkQdo4a9E', 'credit_a', false, false, NULL,
     '100 credits. Never expires, usable on any plan.', true, 50),

    ('Package B', 'credits', NULL, NULL, 10548, 10000, 1499,
     'price_1Tqs4A7htKzrqxMwOCLvjx6w', 'credit_b', false, false, NULL,
     '350 credits — better rate than Package A.', true, 60),

    ('Package C', 'credits', NULL, NULL, 45205, 40000, 5999,
     'price_1Tqs5Z7htKzrqxMwL6qDDp9Y', 'credit_c', false, false, NULL,
     '1500 credits — the best rate of the three packs.', true, 70),

    -- Quantity-based: Stripe charges ¥1 per unit and the buyer picks how many.
    ('Custom top-up', 'credits', NULL, NULL, NULL, 100, 15,
     'price_1Tqs9f7htKzrqxMwOPF9QhBE', 'credit_custom', false, false, 99,
     'Choose your own amount. Every ¥1 adds 3.3 credits.', true, 80)
ON CONFLICT DO NOTHING;
