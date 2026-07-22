-- Per-connection billing mode. Two modes:
--   'rate'      — existing behavior: real token usage × official price × 倍率 (rate).
--   'per_call'  — flat fee per successful upstream call, regardless of token count.
--                 The fee is `per_call_cents` (in cents). Cache hits still bill 0
--                 (no upstream call was made). Image/responses paths are unaffected
--                 (those already bill per-image).
-- Default 'rate' so EVERY existing connection keeps its current billing untouched.
ALTER TABLE models ADD COLUMN IF NOT EXISTS billing_mode  TEXT   NOT NULL DEFAULT 'rate';
ALTER TABLE models ADD COLUMN IF NOT EXISTS per_call_cents BIGINT NOT NULL DEFAULT 0;
