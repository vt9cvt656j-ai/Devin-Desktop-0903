-- Referrals: who brought whom, and for how long that earns.
--
-- The `commissions` ledger already existed but only an admin could write to it, one row at
-- a time, by hand. This is the half that was missing — the attribution that says a payment
-- is owed to somebody, so the ledger fills itself.

-- Terms, alongside the other operating numbers. Read at attribution time rather than
-- cached: this is consulted once per sign-up and once per payment, not per request.
ALTER TABLE app_settings
  -- Basis points, so 30% is 3000 and a half-percent is expressible. Capped at 100%:
  -- paying out more than came in is not a rate, it is a bug with a number in it.
  ADD COLUMN IF NOT EXISTS referral_rate_bps integer NOT NULL DEFAULT 3000
    CHECK (referral_rate_bps BETWEEN 0 AND 10000),

  -- How long a referred customer keeps earning for whoever brought them. 90 days ≈ three
  -- months. 0 is legal and means "the first payment only", which is a real policy rather
  -- than a broken one — the window is checked as `>` against now(), so a zero-day window
  -- has expired by the time the second payment arrives.
  ADD COLUMN IF NOT EXISTS referral_window_days integer NOT NULL DEFAULT 90
    CHECK (referral_window_days BETWEEN 0 AND 3650),

  -- Off means new referrals stop being recorded. Existing ones keep paying out: turning a
  -- programme off should stop new promises, not break the ones already made.
  ADD COLUMN IF NOT EXISTS referral_enabled boolean NOT NULL DEFAULT true;

-- Everyone's invite code.
--
-- On `users` rather than its own table because it is one per account and never more: a
-- second code for the same person would split their own statistics against them. Assigned
-- lazily the first time someone opens the referral screen, so accounts that never use it
-- do not consume codes from the space.
ALTER TABLE users
  ADD COLUMN IF NOT EXISTS referral_code TEXT;

-- Case-insensitively unique: codes get typed by hand, re-typed from screenshots, and
-- pasted with the wrong capitalisation. Two codes differing only in case would be two
-- different people's earnings resolved by the shift key.
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_referral_code
  ON users (lower(referral_code)) WHERE referral_code IS NOT NULL;

CREATE TABLE IF NOT EXISTS referrals (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referrer_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    -- UNIQUE, not just indexed: a person has exactly one referrer, permanently. Without
    -- this, a second claim would create a second row and one payment would owe two people.
    referred_user_id UUID NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE,

    -- What was actually used, kept verbatim. The referrer can change nothing about it
    -- afterwards, and "which code did this come from" is the question asked when someone
    -- disputes an attribution.
    code             TEXT NOT NULL DEFAULT '',
    -- 'code' typed in, or 'link' followed. Worth separating: they answer different
    -- questions about where growth is actually coming from.
    source           TEXT NOT NULL DEFAULT 'code',

    -- Terms FROZEN at the moment of claiming, not read from settings at payout.
    --
    -- This is the whole reason both columns exist here rather than being looked up. Someone
    -- was told "30% for three months" when they shared their link; dropping the rate to 10%
    -- next month must not reach backwards and rewrite what they were promised. Changing the
    -- settings changes the deal for referrals made after the change, and for nobody else.
    rate_bps         INTEGER NOT NULL DEFAULT 3000,
    expires_at       TIMESTAMPTZ NOT NULL,

    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_referrals_referrer ON referrals (referrer_user_id, created_at DESC);
-- The lookup on the payment path: one referred user, is their window still open.
CREATE INDEX IF NOT EXISTS idx_referrals_referred ON referrals (referred_user_id, expires_at);

-- Commissions raised by the system rather than by hand need to be told apart from the
-- manual ones, and an order must not be able to pay out twice.
--
-- Partial, so the manual rows the ledger already holds — which may legitimately share an
-- order id with a system row, or carry none at all — are untouched by it.
CREATE UNIQUE INDEX IF NOT EXISTS idx_commissions_referral_order
  ON commissions (order_id) WHERE source = 'referral' AND order_id IS NOT NULL;
