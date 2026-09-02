-- Per-model billing overrides + a daily free-points pool.
--
-- WHY model_billing: a `models` row is a CONNECTION (one base_url + key, `enabled_models`
-- holding many model ids). `model_prices` is already a per-model map, but billing_mode /
-- per_call_cents are single columns on the connection — so "make THIS model per-call" was
-- impossible; the switch applied to the whole channel. This map is the same shape as
-- model_prices and overrides the connection default per model id:
--   { "<model_id>": { "mode": "rate" | "per_call" | "free", "per_call_cents": 5 } }
--
-- WHY free_points: models flagged "free" bill against a daily points pool instead of the
-- user's quota/wallet. Points are cents so every existing cost calculation applies unchanged;
-- only the deduction target differs.
ALTER TABLE models ADD COLUMN IF NOT EXISTS model_billing JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE users ADD COLUMN IF NOT EXISTS free_points_cents BIGINT NOT NULL DEFAULT 0;
-- The day the pool was last granted. Reset is lazy (compared on read/spend) rather than a
-- cron sweep: no scheduler to fail, and a user who never logs in costs nothing.
ALTER TABLE users ADD COLUMN IF NOT EXISTS free_points_date DATE;
