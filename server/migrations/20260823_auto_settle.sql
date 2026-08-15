-- How commission is paid out: by hand, or as account credit.
--
-- `false` — manual. A commission is raised as `pending`, an operator settles it, and the
-- referrer asks to be paid; money leaves by whatever means the operator actually uses.
-- That is what exists today and stays the default, because switching an existing
-- programme's payout method underneath people is not something a migration should do.
--
-- `true` — automatic. The commission is settled the moment it is raised and the same
-- amount is added to the referrer's credit balance, inside the transaction that records
-- the payment. There is then nothing to withdraw, which is why the withdrawal screens
-- disappear in this mode.
--
-- The trade-off is worth stating plainly, because it is not reversible per-commission:
-- automatic hands out the credit before anyone can look at it. If the customer's payment
-- is later refunded or charged back, the credit has already been granted and the operator
-- has no reject step to catch it. Manual keeps that step at the cost of doing the work.
ALTER TABLE app_settings
  ADD COLUMN IF NOT EXISTS referral_auto_settle BOOLEAN NOT NULL DEFAULT false;
