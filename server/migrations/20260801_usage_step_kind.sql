-- Step-type instrumentation for model_usage.
--
-- Goal: decide model routing from real traffic instead of guesswork. Today we can see
-- WHAT was spent (tokens, cost, model) but not WHAT KIND OF WORK the call did, so there
-- is no way to tell how many expensive calls were mechanical tool dispatch that a cheap
-- tier could have handled.
--
-- Every column is NULLable with no default backfill: adding them cannot fail on existing
-- rows, and a gateway that has not been redeployed yet simply writes NULLs.
--
-- NOTE ON THE FILENAME: this is date-prefixed on purpose. sqlx orders migrations by the
-- integer parsed from the prefix and REFUSES to start if it finds an unapplied migration
-- sorting before an applied one. The DB already has 20260728_fix_quota_system…, so a
-- `0024_*` file (version 24 < 20260728) would crash the backend on boot.

ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS ide_mode     TEXT;   -- agent | chat | explorer | plan | reviewer
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS is_tool_turn BOOLEAN;-- true = continuation of an agent loop (last input msg was a tool result)
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS emitted_tool TEXT;   -- first tool the model called back, NULL = prose answer

-- Routing analysis always slices by (model, kind); index for it.
CREATE INDEX IF NOT EXISTS idx_model_usage_step
  ON model_usage (model_name, ide_mode, emitted_tool, created_at DESC);
