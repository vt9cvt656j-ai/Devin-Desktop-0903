-- Referring is a privilege that is granted, not something every account has.
--
-- Until now anyone who opened the referral screen was handed a code. That is the wrong
-- default for a programme that pays out real money: it means a throwaway account created
-- five minutes ago can start recruiting, and the first time you learn who is in the
-- programme is when a commission appears. The operator picks who can refer.
--
-- DEFAULT false, and deliberately no backfill to true. Nobody has referred anyone yet, so
-- switching the default costs nothing today and would be a much harder decision later —
-- revoking a privilege people have already used is a different conversation from never
-- having handed it out.
--
-- Revoking does NOT stop existing referrals paying out, for the same reason the
-- programme-wide switch does not: the promise was made when the referral was bound, and
-- `award` looks at the referral row, not at this column. What this gates is *new*
-- bindings — see referral.rs.
ALTER TABLE users
  ADD COLUMN IF NOT EXISTS referral_enabled BOOLEAN NOT NULL DEFAULT false;

-- The admin list sorts granted accounts to the top and searches by address; both benefit
-- from not scanning every user row to answer "who is in the programme".
CREATE INDEX IF NOT EXISTS idx_users_referral_enabled
  ON users (referral_enabled) WHERE referral_enabled;
