# Payment Systems & Banking

## Money Representation (THE #1 Rule)

```javascript
// NEVER
const total = 19.99 + 0.01;  // → 20.000000000000004

// ALWAYS — store cents/smallest unit as integers
const totalCents = 1999 + 1;  // → 2000 = $20.00
const display = (totalCents / 100).toFixed(2);  // → "20.00"

// Or use Decimal libraries
import Decimal from 'decimal.js';
const total = new Decimal('19.99').plus('0.01');  // → 20.00 exact
```

**Currency precision by type:**
| Currency | Smallest Unit | Decimal Places |
|----------|--------------|----------------|
| USD/EUR/GBP | cent | 2 |
| JPY/KRW | yen/won | 0 |
| BHD/KWD | fils | 3 |
| Crypto (BTC) | satoshi | 8 |

## Double-Entry Bookkeeping

### Ledger Schema
```sql
CREATE TABLE accounts (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('asset','liability','equity','revenue','expense')),
    currency CHAR(3) NOT NULL DEFAULT 'USD',
    balance BIGINT NOT NULL DEFAULT 0  -- in smallest currency unit
);

CREATE TABLE journal_entries (
    id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    description TEXT NOT NULL,
    reference_id TEXT,  -- external reference (order_id, payment_id)
    idempotency_key TEXT UNIQUE  -- prevent double-posting
);

CREATE TABLE ledger_lines (
    id UUID PRIMARY KEY,
    journal_entry_id UUID NOT NULL REFERENCES journal_entries(id),
    account_id UUID NOT NULL REFERENCES accounts(id),
    amount BIGINT NOT NULL,  -- positive = debit, negative = credit
    CHECK (amount != 0)
);

-- CRITICAL: every journal entry MUST balance to zero
CREATE OR REPLACE FUNCTION check_journal_balance()
RETURNS TRIGGER AS $$
BEGIN
    IF (SELECT SUM(amount) FROM ledger_lines WHERE journal_entry_id = NEW.journal_entry_id) != 0
    THEN RAISE EXCEPTION 'Journal entry does not balance';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

### Debit/Credit Rules
| Account Type | Debit (positive) | Credit (negative) |
|-------------|------------------|-------------------|
| Asset | Increase | Decrease |
| Liability | Decrease | Increase |
| Equity | Decrease | Increase |
| Revenue | Decrease | Increase |
| Expense | Increase | Decrease |

## Payment Processing (Stripe Pattern)

### Payment Intent Flow
```javascript
// 1. Create PaymentIntent server-side
const paymentIntent = await stripe.paymentIntents.create({
  amount: 2000,  // $20.00 in cents
  currency: 'usd',
  metadata: { order_id: order.id },
  idempotency_key: `order_${order.id}_payment`,  // ALWAYS use
});

// 2. Client confirms with payment method
// 3. Handle webhook (the SOURCE OF TRUTH — not the client callback)
app.post('/webhooks/stripe', async (req, res) => {
  const sig = req.headers['stripe-signature'];
  const event = stripe.webhooks.constructEvent(req.body, sig, webhookSecret);

  switch (event.type) {
    case 'payment_intent.succeeded':
      await fulfillOrder(event.data.object.metadata.order_id);
      break;
    case 'payment_intent.payment_failed':
      await handleFailedPayment(event.data.object);
      break;
  }
  res.json({ received: true });  // ALWAYS return 200 quickly
});
```

### Webhook Processing Rules
1. Return 200 immediately, process asynchronously
2. Handle out-of-order delivery (check current state before transitioning)
3. Idempotent processing (use event.id as dedup key)
4. Verify webhook signature ALWAYS
5. Store raw webhook payload for debugging

## PCI-DSS Compliance Checklist

```
NEVER:
- Store full card numbers (PAN) in your database
- Log card numbers, CVV, or PIN in any log
- Send card data in URL parameters
- Store CVV/CVC after authorization (even encrypted)
- Process cards on servers without PCI compliance

ALWAYS:
- Use Stripe Elements / PayPal SDK (card data never touches your server)
- Use tokenization (card → token on client, send token to server)
- HTTPS everywhere (TLS 1.2+)
- Restrict access to cardholder data (need-to-know)
- Mask PAN in any display: **** **** **** 4242
```

## Multi-Currency Handling

```python
from decimal import Decimal, ROUND_HALF_EVEN

class Money:
    def __init__(self, amount: int, currency: str):
        self.amount = amount  # smallest unit (cents)
        self.currency = currency.upper()

    def convert(self, target_currency: str, rate: Decimal) -> 'Money':
        """Convert to target currency using the given rate."""
        if self.currency == target_currency:
            return Money(self.amount, self.currency)
        converted = (Decimal(self.amount) * rate).quantize(
            Decimal('1'), rounding=ROUND_HALF_EVEN
        )
        return Money(int(converted), target_currency)

    def __add__(self, other):
        if self.currency != other.currency:
            raise ValueError(f"Cannot add {self.currency} and {other.currency}")
        return Money(self.amount + other.amount, self.currency)

# Exchange rate rules:
# 1. Always quote: 1 BASE = X QUOTE (e.g., 1 USD = 0.92 EUR)
# 2. Use mid-market rate for display, bid/ask for transactions
# 3. Cache rates with TTL (5min for display, real-time for transactions)
# 4. Store the rate used in the transaction record (for audit)
# 5. Handle triangulation: USD→JPY via USD→EUR→JPY if direct rate unavailable
```

## Reconciliation

```python
def reconcile(internal_txns, bank_statement, tolerance_cents=0):
    """Three-way reconciliation: match internal records to bank statement."""
    matched = []
    unmatched_internal = list(internal_txns)
    unmatched_bank = list(bank_statement)

    # Pass 1: exact match on amount + date + reference
    for bank_txn in list(unmatched_bank):
        for int_txn in list(unmatched_internal):
            if (abs(bank_txn.amount - int_txn.amount) <= tolerance_cents
                and bank_txn.date == int_txn.date
                and bank_txn.reference == int_txn.reference):
                matched.append((int_txn, bank_txn))
                unmatched_internal.remove(int_txn)
                unmatched_bank.remove(bank_txn)
                break

    # Pass 2: fuzzy match on amount + date (within 2 days)
    for bank_txn in list(unmatched_bank):
        for int_txn in list(unmatched_internal):
            if (abs(bank_txn.amount - int_txn.amount) <= tolerance_cents
                and abs((bank_txn.date - int_txn.date).days) <= 2):
                matched.append((int_txn, bank_txn, 'fuzzy'))
                unmatched_internal.remove(int_txn)
                unmatched_bank.remove(bank_txn)
                break

    return {
        'matched': matched,
        'unmatched_internal': unmatched_internal,  # missing from bank
        'unmatched_bank': unmatched_bank,          # missing from system
    }
```

## IBAN / BIC Validation

```javascript
function validateIBAN(iban) {
  const clean = iban.replace(/\s/g, '').toUpperCase();
  if (!/^[A-Z]{2}\d{2}[A-Z0-9]{4,30}$/.test(clean)) return false;

  // Move first 4 chars to end, convert letters to numbers (A=10, B=11...)
  const rearranged = clean.slice(4) + clean.slice(0, 4);
  const numeric = rearranged.replace(/[A-Z]/g, c => c.charCodeAt(0) - 55);

  // Mod 97 check
  let remainder = BigInt(numeric) % 97n;
  return remainder === 1n;
}

function validateBIC(bic) {
  // BIC/SWIFT: 8 or 11 chars — BANKCCLL[BBB]
  return /^[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}([A-Z0-9]{3})?$/.test(bic.toUpperCase());
}
```

## Fraud Detection Rules (Minimum Set)

```
1. Velocity checks: > 3 transactions in 1 minute from same user → flag
2. Amount anomaly: transaction > 3× user's average → flag
3. Geography: transaction from new country within 1 hour of last → flag
4. Card testing: multiple small amounts ($0.50-$2.00) in quick succession → block
5. Device fingerprint: new device + high-value transaction → step-up auth
6. Time-based: transactions at unusual hours (2-5 AM local time) → flag
```
