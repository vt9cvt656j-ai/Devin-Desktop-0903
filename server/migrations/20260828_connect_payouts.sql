-- Paying a referrer without a person in the loop.
--
-- Until now the last step was always human: an operator read an Alipay account off the
-- screen, sent money by hand, and came back to tick a box. This adds the machine path —
-- Stripe Connect Transfers to the referrer's own connected account — and keeps the manual
-- one for everybody who has not connected an account.

-- Which Stripe connected account a referrer gets paid into. NULL until they go through
-- onboarding. Nothing about their identity, bank details or KYC is stored here: that all
-- lives at Stripe, and this column is only the pointer to it.
ALTER TABLE users ADD COLUMN IF NOT EXISTS stripe_connect_account_id TEXT;

-- One account per user, and one user per account. Two rows pointing at one connected
-- account would let a payout land on the wrong person's balance.
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_connect_account
    ON users (stripe_connect_account_id) WHERE stripe_connect_account_id IS NOT NULL;

ALTER TABLE withdrawals
    -- 'manual' (a human sent it) or 'stripe_connect' (a transfer did).
    ADD COLUMN IF NOT EXISTS provider       TEXT NOT NULL DEFAULT 'manual',
    -- Stripe's `tr_…` id. The receipt: it is what a transfer can be looked up by, reversed
    -- by, and reconciled against later.
    ADD COLUMN IF NOT EXISTS transfer_id    TEXT,
    -- Why an automatic attempt did not go through, in Stripe's words. Left on the row so
    -- the request can fall back to the manual queue with the reason attached rather than
    -- silently looking like nobody has got to it yet.
    ADD COLUMN IF NOT EXISTS failure_reason TEXT NOT NULL DEFAULT '';

-- A transfer id may appear on at most one withdrawal. This is the database half of the
-- idempotency story: the API half is the Idempotency-Key on transfers.create (the
-- withdrawal id), and between them a retry cannot pay twice.
CREATE UNIQUE INDEX IF NOT EXISTS idx_withdrawals_transfer
    ON withdrawals (transfer_id) WHERE transfer_id IS NOT NULL;

-- `status` gains two terminal values beyond pending/paid/rejected:
--
--   'failed'   — the transfer was refused (no balance, account not ready). The money was
--                never sent, so it must go back to the withdrawable balance.
--   'returned' — the transfer went out and was later reversed by Stripe. Same accounting
--                consequence, different story, and worth telling apart when someone asks
--                what happened to their payout.
--
-- `withdrawable` counts anything not in (rejected, failed, returned) as already spoken
-- for, so both of these release the money again — see referral.rs.
CREATE INDEX IF NOT EXISTS idx_withdrawals_needs_attention
    ON withdrawals (created_at DESC) WHERE status IN ('failed', 'returned');
