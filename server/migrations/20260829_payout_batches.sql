-- 批量结算：冻结期 + 提现门槛 + 定时打款。
--
-- 对应 stripe-billing-kit 的 payout-scheduler.ts 与规范 Phase 3。原来的模型是「谁想提现谁点
-- 一下，能提多少提多少，立刻转账」，缺的正是两道最要紧的闸：
--
--   · 冻结期。佣金一到手就能变现，而退款和拒付是几十天后才发生的事。钱转出去之后再退款，
--     你赔的是这笔销售、拒付手续费，外加一笔已经付出去、代码里没有任何路径能追回的佣金。
--   · 门槛。一笔 $1.05 的佣金单独转一次，Stripe 的固定费用就把它吃掉大半。攒够再发。
--
-- 状态机（左边是我们的列值，右边是 kit 里的名字）：
--
--   pending   PENDING    刚记下，等审核
--   settled   APPROVED   审核通过，进入冻结期，到期后可以进批次
--   paid      PAID       已被某一批次锁定，钱正在路上
--   rejected  REJECTED   作废
--   reversed  REFUNDED   退款/拒付追回
--
-- 打款失败或被冲回时，paid 回滚成 settled，下一轮重新参加 —— 和 kit 的
-- rollbackCommissions 是同一件事。

-- 冻结期的终点，记在佣金行上而不是每次现算：条款是发生时冻结的，之后改设置不该动到
-- 已经记下的佣金（和 rate_bps / expires_at 一个道理）。
ALTER TABLE commissions
    ADD COLUMN IF NOT EXISTS mature_at TIMESTAMPTZ,
    -- 这笔佣金被哪一次打款锁走了。回滚和对账都靠它。
    ADD COLUMN IF NOT EXISTS payout_id UUID REFERENCES withdrawals (id) ON DELETE SET NULL;

-- 已有的行没有冻结期概念，按"立即到期"处理：它们是在这套机制之前记下的，
-- 凭空给它们加 14 天等于把已经承诺的东西往后推。
UPDATE commissions SET mature_at = COALESCE(settled_at, created_at) WHERE mature_at IS NULL;

-- 调度器每轮的主查询：按状态和到期时间捞。
CREATE INDEX IF NOT EXISTS idx_commissions_payable
    ON commissions (mature_at) WHERE status = 'settled';

CREATE INDEX IF NOT EXISTS idx_commissions_payout ON commissions (payout_id)
    WHERE payout_id IS NOT NULL;

ALTER TABLE app_settings
    -- 冻结期天数。kit 的默认是 14。
    ADD COLUMN IF NOT EXISTS referral_hold_days INTEGER NOT NULL DEFAULT 14
        CHECK (referral_hold_days >= 0 AND referral_hold_days <= 180),
    -- 提现门槛，分。kit 的默认是 5000 = $50。
    ADD COLUMN IF NOT EXISTS referral_min_payout_cents BIGINT NOT NULL DEFAULT 5000
        CHECK (referral_min_payout_cents > 0),
    -- 定时批量打款的总开关。
    --
    -- 默认关闭，而且是刻意的：这个开关一打开，服务器就会在没有任何人点击的情况下往外转
    -- 真钱。它必须是运营明确打开的，不能因为一次部署就自己开始跑。
    ADD COLUMN IF NOT EXISTS referral_batch_enabled BOOLEAN NOT NULL DEFAULT false;
