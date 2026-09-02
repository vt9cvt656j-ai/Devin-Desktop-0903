-- Asking to be paid what the commission ledger says you are owed.
--
-- A request, not a payment. Nothing in this service moves money to a person — settling a
-- commission marks it dealt with, and paying it happens by whatever means the operator
-- actually uses. This table is the bridge between those two facts: it records who asked,
-- for how much, and where they want it sent, so the operator has something to work from and
-- the referrer can see that they asked.
CREATE TABLE IF NOT EXISTS withdrawals (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,

    -- Whole cents, and greater than zero. A request for nothing is not a request.
    amount_cents BIGINT NOT NULL CHECK (amount_cents > 0),

    -- 'alipay' | 'wechat' | 'bank' | 'paypal'. Kept as written rather than normalised:
    -- what matters later is "how was this person actually paid".
    method       TEXT NOT NULL,

    -- Where to send it, as the person typed it. Deliberately free text — an Alipay
    -- account, a PayPal address and a bank account have nothing in common structurally,
    -- and a schema that pretended otherwise would reject somebody's real details.
    --
    -- This is payout information: it identifies a person and their account. It is stored
    -- as written, readable by anyone with database access, and is not something to expose
    -- on any endpoint that is not the owner's own or an admin's.
    account      TEXT NOT NULL,

    -- 'pending' | 'paid' | 'rejected'. 'paid' means a human sent the money and said so.
    status       TEXT NOT NULL DEFAULT 'pending',
    note         TEXT NOT NULL DEFAULT '',

    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    paid_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_withdrawals_user ON withdrawals (user_id, created_at DESC);
-- The operator's queue: everything still waiting, oldest first.
CREATE INDEX IF NOT EXISTS idx_withdrawals_pending
  ON withdrawals (created_at) WHERE status = 'pending';
