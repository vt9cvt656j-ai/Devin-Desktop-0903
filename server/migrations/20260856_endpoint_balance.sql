-- 中转账户余额的定时快照。**对账里「成本」那一侧的唯一真实来源。**
--
-- # 为什么必须存快照，不能只读当前值
--
-- 「这个中转这周花了我多少钱」没有任何接口能直接回答 —— 各家只告诉你**现在还剩多少**。
-- 花掉的钱只能由两次读数相减得出，所以余额必须按时间存下来。只读当前值的话，
-- 面板永远只能显示「还剩 X」，回答不了「亏没亏钱」。
--
-- # remaining_usd 可以为 NULL，而且 NULL ≠ 0
--
-- 有些中转根本没有余额接口。把查不到记成 0，下一次真实读数一减就会算出一个巨大的
-- 「成本」，然后对账页显示这条线路在疯狂亏钱 —— 一个纯粹由缺失数据造出来的结论。
-- 查不到就存 NULL，算成本时整段跳过。
--
-- # 主键是 (taken_at, endpoint_id)
--
-- 一个出口一次快照一行。半小时一次、十个出口，一年约十七万行 —— 可以接受，
-- 而按天聚合会把「今天充了值」这种事抹平，那正是成本计算必须看见的东西。
CREATE TABLE IF NOT EXISTS endpoint_balance (
    taken_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- 出口 id；线路自带地址用线路 id（和 endpoint_usage / health_id 同一套命名空间）。
    endpoint_id   UUID NOT NULL,
    route_id      UUID NOT NULL,
    -- 还剩多少美元。NULL = 这家没有可识别的余额接口，或者这次没查成 —— 不是 0。
    remaining_usd DOUBLE PRECISION,
    -- 有些家（One API 一族）同时给「已用」。它比余额更适合算成本：充值不会打断它。
    used_usd      DOUBLE PRECISION,
    -- 原始展示串，排查用。解析错了的时候，这里是唯一能看出「上游到底回了什么」的地方。
    raw           TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (taken_at, endpoint_id)
);

-- 对账查的是「某个出口在某段时间里的第一条和最后一条」，按这个顺序建索引。
CREATE INDEX IF NOT EXISTS idx_endpoint_balance_ep_time
    ON endpoint_balance (endpoint_id, taken_at DESC);
