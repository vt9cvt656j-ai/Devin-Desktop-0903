# E-Commerce: Catalog, Cart & Order Management

## Product Catalog Schema

### Core Data Model
```sql
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sku TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    -- CHECK (status IN ('draft','active','archived','out_of_stock'))
    category_id UUID REFERENCES categories(id),
    brand TEXT,
    base_price BIGINT NOT NULL,  -- cents, smallest currency unit
    currency CHAR(3) NOT NULL DEFAULT 'USD',
    weight_grams INT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Variants (size, color, etc.)
CREATE TABLE product_variants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products(id),
    sku TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,         -- "Large / Red"
    price_override BIGINT,     -- null = use base_price
    stock_quantity INT NOT NULL DEFAULT 0,
    attributes JSONB NOT NULL,  -- {"size": "L", "color": "red"}
    weight_grams INT,
    UNIQUE(product_id, attributes)
);

-- Category tree (closure table for arbitrary depth)
CREATE TABLE categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    parent_id UUID REFERENCES categories(id)
);
CREATE TABLE category_closure (
    ancestor_id UUID REFERENCES categories(id),
    descendant_id UUID REFERENCES categories(id),
    depth INT NOT NULL,
    PRIMARY KEY (ancestor_id, descendant_id)
);
```

### Search & Filtering
```javascript
// Elasticsearch product index mapping
const productMapping = {
  properties: {
    name: { type: 'text', analyzer: 'standard', fields: { keyword: { type: 'keyword' } } },
    description: { type: 'text' },
    category_path: { type: 'keyword' },  // "Electronics > Phones > Smartphones"
    price: { type: 'scaled_float', scaling_factor: 100 },
    attributes: { type: 'nested' },       // for faceted filtering
    in_stock: { type: 'boolean' },
    created_at: { type: 'date' },
  }
};

// Faceted search query
function buildSearchQuery(filters) {
  const must = [{ match: { name: filters.query } }];
  const filter = [];

  if (filters.category) filter.push({ term: { category_path: filters.category } });
  if (filters.price_min || filters.price_max) {
    filter.push({ range: { price: { gte: filters.price_min, lte: filters.price_max } } });
  }
  if (filters.in_stock) filter.push({ term: { in_stock: true } });

  return { bool: { must, filter } };
}
```

## Shopping Cart

### Cart Architecture (Redis + DB)
```python
import json, hashlib

class Cart:
    """Hybrid: Redis for active carts (fast), DB for persistence (recovery)"""

    def __init__(self, redis, db, user_id=None, session_id=None):
        self.key = f"cart:{user_id or session_id}"
        self.redis = redis
        self.db = db

    def add_item(self, variant_id, quantity=1):
        # Atomic increment
        self.redis.hincrby(self.key, variant_id, quantity)
        self.redis.expire(self.key, 86400 * 30)  # 30-day TTL
        self._sync_to_db()

    def get_items(self):
        raw = self.redis.hgetall(self.key)
        return {vid: int(qty) for vid, qty in raw.items()}

    def validate_stock(self):
        """Check all items still available — call before checkout"""
        items = self.get_items()
        issues = []
        for variant_id, qty in items.items():
            stock = self.db.get_stock(variant_id)
            if stock == 0:
                issues.append({'variant_id': variant_id, 'issue': 'out_of_stock'})
            elif stock < qty:
                issues.append({'variant_id': variant_id, 'issue': 'insufficient', 'available': stock})
        return issues

    def merge_guest_to_user(self, session_id, user_id):
        """On login: merge guest cart into user cart"""
        guest_key = f"cart:{session_id}"
        user_key = f"cart:{user_id}"
        guest_items = self.redis.hgetall(guest_key)
        for vid, qty in guest_items.items():
            self.redis.hincrby(user_key, vid, int(qty))
        self.redis.delete(guest_key)
```

## Order State Machine

### Order Lifecycle
```
                    ┌─────────────────┐
                    │    CREATED      │
                    └────────┬────────┘
                             │ payment_initiated
                    ┌────────▼────────┐
                    │ PAYMENT_PENDING │──── payment_failed ──→ CANCELLED
                    └────────┬────────┘
                             │ payment_confirmed
                    ┌────────▼────────┐
                    │   CONFIRMED     │──── cancel_requested ──→ CANCELLED (+ refund)
                    └────────┬────────┘
                             │ items_picked
                    ┌────────▼────────┐
                    │   PROCESSING    │
                    └────────┬────────┘
                             │ shipped
                    ┌────────▼────────┐
                    │    SHIPPED      │──── tracking_number assigned
                    └────────┬────────┘
                             │ delivered
                    ┌────────▼────────┐
                    │   DELIVERED     │──── return_requested ──→ RETURN_PENDING
                    └────────┬────────┘
                             │ (after return window)
                    ┌────────▼────────┐
                    │    COMPLETED    │
                    └─────────────────┘
```

### Order Schema
```sql
CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_number TEXT UNIQUE NOT NULL,  -- human-readable: ORD-20240115-A7X3
    user_id UUID NOT NULL,
    status TEXT NOT NULL DEFAULT 'created',
    subtotal BIGINT NOT NULL,
    tax_amount BIGINT NOT NULL DEFAULT 0,
    shipping_amount BIGINT NOT NULL DEFAULT 0,
    discount_amount BIGINT NOT NULL DEFAULT 0,
    total BIGINT NOT NULL,
    currency CHAR(3) NOT NULL DEFAULT 'USD',
    shipping_address JSONB NOT NULL,
    billing_address JSONB NOT NULL,
    payment_intent_id TEXT,  -- Stripe reference
    idempotency_key TEXT UNIQUE NOT NULL,  -- prevent double orders
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE order_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES orders(id),
    variant_id UUID NOT NULL,
    sku TEXT NOT NULL,
    name TEXT NOT NULL,       -- snapshot at purchase time
    quantity INT NOT NULL,
    unit_price BIGINT NOT NULL,  -- price at purchase time (NEVER reference current price)
    total BIGINT NOT NULL
);

-- CRITICAL: decrement stock atomically on order confirmation
-- Use SELECT ... FOR UPDATE or advisory locks to prevent overselling
```

### Inventory Management
```python
def reserve_stock(variant_id, quantity, order_id):
    """Optimistic locking: prevent overselling"""
    result = db.execute("""
        UPDATE product_variants
        SET stock_quantity = stock_quantity - %(qty)s
        WHERE id = %(vid)s AND stock_quantity >= %(qty)s
        RETURNING stock_quantity
    """, {'vid': variant_id, 'qty': quantity})

    if result.rowcount == 0:
        raise InsufficientStockError(variant_id)

    # Record reservation for potential rollback
    db.execute("""
        INSERT INTO stock_reservations (order_id, variant_id, quantity, reserved_at)
        VALUES (%(oid)s, %(vid)s, %(qty)s, NOW())
    """, {'oid': order_id, 'vid': variant_id, 'qty': quantity})
```

## Pricing & Promotions

### Discount Engine
```python
class DiscountEngine:
    def apply_discounts(self, cart_items, user, promo_codes):
        discounts = []

        for code in promo_codes:
            promo = self.validate_promo(code, user)
            if not promo:
                continue

            if promo.type == 'percentage':
                amount = sum(i.total for i in cart_items) * promo.value / 100
                amount = min(amount, promo.max_discount or float('inf'))
            elif promo.type == 'fixed_amount':
                amount = promo.value
            elif promo.type == 'buy_x_get_y':
                amount = self._calc_bxgy(cart_items, promo)
            elif promo.type == 'free_shipping':
                amount = cart_items.shipping_cost

            discounts.append({'code': code, 'amount': int(amount), 'promo_id': promo.id})

        # Stack rules: only one percentage + one fixed allowed
        return self._apply_stacking_rules(discounts)

    def validate_promo(self, code, user):
        promo = db.get_promo(code)
        if not promo or promo.expired:
            return None
        if promo.max_uses and promo.current_uses >= promo.max_uses:
            return None
        if promo.max_uses_per_user and self._user_usage(user, promo) >= promo.max_uses_per_user:
            return None
        if promo.min_order_amount and cart_total < promo.min_order_amount:
            return None
        return promo
```

### Tax Calculation
```javascript
// Use tax API (TaxJar, Avalara) for production — these are the patterns
async function calculateTax(order) {
  // Nexus rules: only collect in states where you have presence
  const nexusStates = await getNexusStates();
  if (!nexusStates.includes(order.shipping_address.state)) {
    return 0;
  }

  // Tax rates vary by: state, county, city, product category
  const rate = await taxApi.getRateForAddress(order.shipping_address);

  // Some items are tax-exempt (groceries in some states, clothing in PA/NJ)
  const taxableItems = order.items.filter(i => !isExempt(i.category, order.shipping_address.state));
  const taxableAmount = taxableItems.reduce((sum, i) => sum + i.total, 0);

  return Math.round(taxableAmount * rate);
}
```

## Checkout Flow

### Critical Rules
```
1. NEVER charge before stock validation (validate → reserve → charge → confirm)
2. ALWAYS use idempotency keys on payment creation
3. ALWAYS snapshot prices into order_items (prices change, orders don't)
4. ALWAYS validate addresses server-side (USPS/Google Address Validation API)
5. NEVER store full card numbers (use Stripe Elements / tokenization)
6. ALWAYS handle partial failures (payment succeeded but order save failed → queue for recovery)
7. ALWAYS calculate totals server-side (client can display, but server is authoritative)
8. Rate-limit checkout endpoint (prevent card testing attacks)
```

### Shipping Rate Calculation
```javascript
async function getShippingRates(items, destination) {
  const totalWeight = items.reduce((sum, i) => sum + (i.weight_grams * i.quantity), 0);
  const packages = packItems(items);  // bin-packing algorithm

  const rates = await Promise.all([
    carrier.getRates('usps', packages, destination),
    carrier.getRates('ups', packages, destination),
    carrier.getRates('fedex', packages, destination),
  ]);

  return rates
    .flat()
    .sort((a, b) => a.price - b.price)
    .map(r => ({
      carrier: r.carrier,
      service: r.service,  // 'ground', 'express', '2day'
      price: r.price,
      estimated_days: r.transit_days,
    }));
}
```
