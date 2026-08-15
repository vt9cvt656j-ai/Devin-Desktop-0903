-- 订单被退款/拒付的事实，记在订单自己身上。
--
-- 之前退款只写佣金那一侧。订单本身没有任何痕迹，于是 fulfil_session 的认领条件
-- （status <> 'paid'）和对账扫描仍然认为这是一笔正常的已付订单 —— 而一笔退过款的
-- Checkout Session 在 Stripe 那边**依然报 payment_status: paid**。结果是退款之后，
-- 履约和计佣还能再跑一遍。
ALTER TABLE orders ADD COLUMN IF NOT EXISTS refunded_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_orders_refunded ON orders (refunded_at)
    WHERE refunded_at IS NOT NULL;
