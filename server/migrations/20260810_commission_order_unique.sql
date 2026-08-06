-- 一个订单最多一条应付佣金 —— 由数据库保证，而不是由一次 SELECT 保证。
--
-- commission.rs 里已经有一段"先查有没有、没有再插"的防重复逻辑，注释写明它要挡的是
-- 「表单被点了两次，或者两个运营在处理同一批积压」。问题在于这两件事恰好都是**并发**的：
-- 两个请求各自的 SELECT 都在对方 INSERT 落地之前跑完，于是两条都插进去，推荐人被付两次。
-- 双击表单是它列出的第一个场景，也正是它挡不住的那个。
--
-- 唯一索引让并发情形下的第二条直接被数据库拒绝，与请求的时序无关。
--
-- 条件索引，两个条件都必要：
--   · status <> 'rejected' —— 保留原有语义：作废的记录不占位，改错了还能重开。
--   · order_id IS NOT NULL —— 手工录入的佣金没有订单号。Postgres 的唯一索引本就认为
--     NULL 互不相同，写出来是为了让意图明确，不用读的人回忆这条规则。
--
-- 建索引前已确认线上没有重复行（commissions 表当前为空），所以不会建不起来。
CREATE UNIQUE INDEX IF NOT EXISTS idx_commissions_order_payable
    ON commissions (order_id)
    WHERE order_id IS NOT NULL AND status <> 'rejected';
