-- 一个账号可以同时挂多条 Stripe 订阅（线上已有 1 例）：额度是加法累积的
-- （codes.rs 的 apply_plan：new_total = cur_total + total），先订 basic、再买 pro，
-- quota_total_cents 是两笔叠出来的。
--
-- 而取消是**减法做成了归零**：`customer.subscription.deleted` → end_subscription →
-- clear_plan → plan/quota 全部清空，不管这个账号是不是还有别的订阅在付费。
-- 用户在 Stripe 客户门户把已经用不上的那条旧订阅退掉，把还在付钱的那条的额度一起赔进去。
--
-- 判断「还有没有别的活订阅」需要本地知道哪些订阅已经结束了。加这一列，
-- 由 end_subscription 在处理取消事件时写上。
--
-- 纯加列带默认（NULL = 还没结束），对蓝绿重叠窗口里的旧版本完全兼容。
ALTER TABLE orders ADD COLUMN IF NOT EXISTS subscription_ended_at TIMESTAMPTZ;
CREATE INDEX IF NOT EXISTS idx_orders_live_subscription
  ON orders (user_id) WHERE stripe_subscription_id IS NOT NULL AND subscription_ended_at IS NULL;
