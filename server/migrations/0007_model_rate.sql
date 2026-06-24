-- Billing multiplier per connection: credits deducted = total_tokens/1000 * rate (USD cents).
ALTER TABLE models ADD COLUMN IF NOT EXISTS rate DOUBLE PRECISION NOT NULL DEFAULT 1.0;
-- model_id is now optional (a connection exposes its `enabled_models` set instead).
ALTER TABLE models ALTER COLUMN model_id DROP NOT NULL;
ALTER TABLE models ALTER COLUMN model_id SET DEFAULT '';
