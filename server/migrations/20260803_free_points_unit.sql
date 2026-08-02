-- Denominate the free pool in POINTS, not cents, and record points spent per usage row.
--
-- The operator's unit is 点: ¥0.5 = 10 点, so 1 点 = ¥0.05 and the ¥2 daily allowance is
-- exactly 40 点. The previous column stored raw provider cents, which the client then
-- rendered through the 663-raw-cents-per-credit-dollar denominator and showed as "$3.02" —
-- a real number in the wrong unit. Points are the unit the operator prices in, so store them.
--
-- Values are reset rather than converted: the old column held a different unit, and the pool
-- is granted fresh each day anyway, so a conversion would be fiction.
ALTER TABLE users DROP COLUMN IF EXISTS free_points_cents;
ALTER TABLE users ADD COLUMN IF NOT EXISTS free_points BIGINT NOT NULL DEFAULT 0;

-- Points charged for this call, so 账单 can show free-model spend beside paid spend instead
-- of a row that silently reads $0.00.
ALTER TABLE model_usage ADD COLUMN IF NOT EXISTS free_points_spent BIGINT NOT NULL DEFAULT 0;
