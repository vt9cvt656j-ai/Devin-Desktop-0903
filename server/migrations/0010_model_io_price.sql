-- Real per-token billing: separate INPUT and OUTPUT unit prices, in USD per 1,000,000
-- tokens — the exact unit real model APIs publish (e.g. $2.50 in / $10.00 out per 1M).
-- 0 means "not configured": the gateway then keeps the existing flat per-call fee for
-- that model, so NOTHING changes for a model until you set its real prices in the admin.
ALTER TABLE models ADD COLUMN IF NOT EXISTS input_price  DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE models ADD COLUMN IF NOT EXISTS output_price DOUBLE PRECISION NOT NULL DEFAULT 0;
