-- Sub-cent per-call fee at the CONNECTION level.
--
-- 20260805 gave per-MODEL overrides micro-USD precision, but the connection-wide field kept
-- whole cents: entering 0.0055 computed Math.round(0.55) = 1 cent and the form redisplayed
-- it as "0.010", which reads as the value silently reverting. Same floor, different field.
ALTER TABLE models ADD COLUMN IF NOT EXISTS per_call_micro_usd BIGINT NOT NULL DEFAULT 0;
-- Backfill so existing per-call channels keep charging exactly what they charge today.
UPDATE models SET per_call_micro_usd = per_call_cents * 10000
 WHERE per_call_micro_usd = 0 AND per_call_cents > 0;
