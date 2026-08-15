-- Who settled a commission, and therefore how.
--
-- `status = 'settled'` already recorded *that* it happened and `settled_at` recorded when,
-- but not by what route — and the two routes are meaningfully different. Automatic means
-- the gateway credited the referrer's balance at the moment of payment, with no human
-- involved. Manual means a named operator looked at it and decided.
--
-- 'auto' for the first, the operator's address for the second. A settlement record that
-- cannot tell them apart is a list of amounts with no account of how the money moved.
--
-- Empty for the rows settled before this column existed: unknown is the honest value, and
-- backfilling a guess would put a claim in the audit trail that nobody made.
ALTER TABLE commissions
  ADD COLUMN IF NOT EXISTS settled_by TEXT NOT NULL DEFAULT '';

-- The settlement-records screens read this end of the table, newest first.
CREATE INDEX IF NOT EXISTS idx_commissions_settled
  ON commissions (settled_at DESC) WHERE status = 'settled';
