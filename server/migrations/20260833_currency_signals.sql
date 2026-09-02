-- 中国区判定 + 人民币收款所需要的列。
--
-- 三个月付套餐在 Stripe 上本来就带人民币价格（currency_options），所以中国区不需要另建
-- 商品行 —— 只要结账时把币种指定成 cny。这条迁移准备的是「判定依据留痕」和「佣金换算」。

-- 结算汇率：1 人民币分 折合多少美元分，万分比。7.10 CNY/USD → 10000/7.10 ≈ 1408。
--
-- 不复用 channel_rates.usd_per_cny：那个数活在原始计费单位的世界里（默认 6.63），
-- 和这里的钱不是一个量纲，混用会差两个数量级。
ALTER TABLE app_settings
    ADD COLUMN IF NOT EXISTS usd_per_cny_bps INTEGER NOT NULL DEFAULT 1408
        CHECK (usd_per_cny_bps BETWEEN 100 AND 10000);

-- 佣金账本继续以美元记账，另外留下原币种的审计三列。
--
-- 为什么不把 commission_cents 改成多币种：referral.rs 里十几处 SUM(commission_cents) 是
-- 跨行合计的，connect::pay 也只能从美元余额转账。一旦这一列混了币种，那些合计全部失去
-- 意义，而且是静默的。所以换算在写入时做一次，汇率钉在行上，事后改汇率不会改写历史。
ALTER TABLE commissions
    ADD COLUMN IF NOT EXISTS sale_currency     TEXT    NOT NULL DEFAULT 'usd',
    ADD COLUMN IF NOT EXISTS sale_amount_cents BIGINT,
    ADD COLUMN IF NOT EXISTS fx_bps            INTEGER NOT NULL DEFAULT 10000;

-- 这笔订单当时是按什么判成中国区的。三个信号原样留档：定价争议只能靠它回答。
ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS resolved_currency     TEXT,
    ADD COLUMN IF NOT EXISTS signal_country        TEXT,
    ADD COLUMN IF NOT EXISTS signal_language       TEXT,
    ADD COLUMN IF NOT EXISTS signal_timezone       TEXT,
    ADD COLUMN IF NOT EXISTS signal_offset_minutes INTEGER;

-- 删商品会让老订单的 price_id 变成 NULL，而续费正是靠原始订单的 price_id 找回商品
-- （fulfil_renewal 要求 price_id IS NOT NULL）。于是删一个商品 = 它名下所有订阅的续费
-- 从此静默失效：不发货、不报错、Stripe 收到 200 不再重投。
--
-- 改成 RESTRICT：有订单引用就删不掉，逼人去用「下架」（active=false），那本来就是对的做法。
ALTER TABLE orders DROP CONSTRAINT IF EXISTS orders_price_id_fkey;
ALTER TABLE orders ADD CONSTRAINT orders_price_id_fkey
    FOREIGN KEY (price_id) REFERENCES prices (id) ON DELETE RESTRICT;
