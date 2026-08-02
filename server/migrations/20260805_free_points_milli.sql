-- Sub-point precision for the free pool.
--
-- Two integer floors made small per-call fees impossible:
--   1. per_call_cents is whole cents, so a $0.003 fee rounded to 0 and the model became
--      genuinely free — the admin form appeared to have a $0.01 minimum.
--   2. points were whole 点, and the deduction rounded UP, so ANY non-zero cost cost 1 点.
--      A ¥2 (40 点) allowance was therefore always exactly 40 calls, whatever the fee.
--
-- The pool now stores MILLI-点 (1 点 = 1000), and per-model fees are stored as micro-USD
-- (1 cent = 10 000 micro-USD) in the model_billing map. Both are integers — no floats in the
-- money path — but fine enough that a $0.003 call costs 0.06 点 instead of a whole one.
--
-- Existing balances are scaled, not reset: a user mid-day keeps what they had.
UPDATE users SET free_points = free_points * 1000;

-- Spend recorded per usage row, also in milli-点, so 账单 can show fractional spend.
ALTER TABLE model_usage
  ADD COLUMN IF NOT EXISTS free_milli_points_spent BIGINT NOT NULL DEFAULT 0;
UPDATE model_usage
   SET free_milli_points_spent = free_points_spent * 1000
 WHERE free_points_spent > 0 AND free_milli_points_spent = 0;
