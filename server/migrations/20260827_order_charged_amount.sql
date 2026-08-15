-- What Stripe actually charged, recorded next to what the catalogue says it costs.
--
-- `orders.amount_cents` is copied from `prices.amount_cents`, which is the CNY display
-- price in fen: 「Power」 is 18800. Every screen renders orders as USD, so one Power sale
-- was reported as "$188.00" against a charge of US$34.99 — revenue overstated five to six
-- times over, with nothing anywhere to contradict it.
--
-- The catalogue column is not the bug and is not being changed: ¥188.00 is a real, correct
-- shelf price, and the billing page is right to show it. The bug is that a *record of a
-- payment* was carrying a shelf price instead of a payment. These two columns carry the
-- payment — the amount and the currency Stripe reports on the session or invoice — so the
-- number in the ledger is the number that moved.
--
-- Nullable, and left NULL for everything that already exists: those orders predate the
-- column and there is no honest value to backfill. A screen showing money must be able to
-- tell "we charged nothing" apart from "we did not record what we charged", and NULL is
-- how it tells. `stripe.rs` fills both in at fulfilment from that point on.
ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS charged_cents    BIGINT,
    ADD COLUMN IF NOT EXISTS charged_currency TEXT;
