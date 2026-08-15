-- Three holes in the commission programme, all on the Stripe side.
--
-- 1. RENEWALS PAID NOTHING. `award` was only ever called from the Checkout path, so a
--    monthly subscriber earned their referrer a commission in month 1 and nothing in
--    months 2 and 3 — against terms that promise a rate for a whole window. Renewals now
--    award too, which needs a way to say "I have already handled this invoice".
--
--    The renewal path had no such key at all: it INSERTed an order row with no unique
--    column, relying entirely on the per-event dedupe in `stripe_events`. That is thinner
--    than the Checkout path, where the `UPDATE … WHERE status <> 'paid'` claim is what
--    makes a redelivery harmless. An invoice id gives renewals the same property, and it
--    protects the grant as much as the commission.
ALTER TABLE orders ADD COLUMN IF NOT EXISTS stripe_invoice_id TEXT;

-- One order per invoice. Existing renewal rows have NULL here and are unaffected —
-- Postgres treats NULLs as distinct, so they neither collide nor get claimed.
CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_stripe_invoice
    ON orders (stripe_invoice_id) WHERE stripe_invoice_id IS NOT NULL;

-- 2. REFUNDS DID NOT CLAW BACK. A refunded or disputed purchase left its commission on
--    the books as if the money had stayed. `reversed` joins pending/settled/rejected as a
--    status; every sum in referral.rs filters on an explicit status, so a reversed row
--    drops out of both "owed" and "settled" without any of them being touched.
--
--    `reversed_at` and the reason are recorded separately from the status because of the
--    case the status cannot express: a commission that was ALREADY PAID OUT when the
--    refund arrived. Those keep status = 'settled' — the money really did leave, and
--    rewriting the ledger would not bring it back — and carry a reversal note so the
--    operator can see what happened and decide. See `referral::reverse`.
ALTER TABLE commissions
    ADD COLUMN IF NOT EXISTS reversed_at     TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS reversal_reason TEXT NOT NULL DEFAULT '';

-- The operator's follow-up queue: refunds that landed after the money was gone.
CREATE INDEX IF NOT EXISTS idx_commissions_reversed
    ON commissions (reversed_at DESC) WHERE reversed_at IS NOT NULL;

-- 3. "PAID" WAS A FLAG, NOT A RECORD. Marking a withdrawal paid set a status and a
--    timestamp and nothing else: not who did it, not what was actually sent, not any
--    reference that could be matched against a bank or Alipay statement later. For the
--    one step in this system where real money leaves, that is the wrong amount of
--    evidence. Nothing here moves money — it still does not — but what a human did is now
--    written down.
ALTER TABLE withdrawals
    ADD COLUMN IF NOT EXISTS paid_by   TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS reference TEXT NOT NULL DEFAULT '';
