-- 充值比例：人民币进去，多少余额出来。
--
-- # 为什么这不是一个「汇率」
--
-- 实测（2026-08-25，线上三家 sub2api）：前端的 currency 分包只是个格式化工具
-- （货币符号表 + Intl.NumberFormat，默认 CNY），**没有任何汇率常量**。
-- 比例是**逐套餐**的：每档充值各自定价（¥50 → 多少余额），所以要存的是套餐表，
-- 不是一个数。存成单一汇率的话，改了某一档的定价就再也对不上。
CREATE TABLE IF NOT EXISTS endpoint_topup_plan (
    endpoint_id   UUID NOT NULL,
    -- 上游给这档套餐的 id / 名字。没有 id 就用名字，两者都没有就用序号。
    plan_key      TEXT NOT NULL,
    plan_name     TEXT NOT NULL DEFAULT '',
    -- 付多少。币种跟 currency 走，**不假设是人民币** —— 有的站按美元定价。
    price         DOUBLE PRECISION NOT NULL,
    currency      TEXT NOT NULL DEFAULT 'CNY',
    -- 到账多少余额（上游自己的余额单位；对 sub2api 就是美元）。
    -- NULL = 套餐表里没给这个数，比例算不出来 —— **不能当 0**。
    granted       DOUBLE PRECISION,
    -- granted / price。NULL = 上面那个是 NULL，或者 price <= 0。
    rate          DOUBLE PRECISION,
    -- 认不出字段时把上游原文留一段。「这家结构和我们猜的不一样」和「这家没有套餐」
    -- 必须能分开 —— 前者一眼能看出该怎么改，后者才是真没有。
    raw           TEXT NOT NULL DEFAULT '',
    fetched_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (endpoint_id, plan_key)
);

-- 余额跳升 = 一次充值。**这是没有套餐表时唯一的兜底**，也是套餐表的交叉验证：
-- 到账金额对不上任何一档套餐，说明套餐表过期了或者你走的是站外充值。
CREATE TABLE IF NOT EXISTS endpoint_topup_event (
    id          BIGSERIAL PRIMARY KEY,
    endpoint_id UUID NOT NULL,
    route_id    UUID NOT NULL,
    -- 余额从多少涨到多少（上游余额单位）。
    before_bal  DOUBLE PRECISION NOT NULL,
    after_bal   DOUBLE PRECISION NOT NULL,
    -- 到账金额 = after - before。
    granted     DOUBLE PRECISION NOT NULL,
    -- 匹配到的套餐（按到账金额就近匹配）。空 = 没匹配上，多半是站外充值。
    matched_plan TEXT NOT NULL DEFAULT '',
    -- 匹配到套餐时带出来的付款金额和币种；没匹配上就留空**而不是猜一个**。
    price       DOUBLE PRECISION,
    currency    TEXT NOT NULL DEFAULT '',
    at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_topup_event_time ON endpoint_topup_event (at DESC);
