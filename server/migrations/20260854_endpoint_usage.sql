-- 每个出口每天用掉多少。**纯观测，不参与任何计费**。
--
-- # 为什么另起一张表，而不是往 model_usage 加一列
--
-- model_usage 是计费链路的一部分：它那条插入跑在结算事务里，`model_id` 有外键、
-- 归属被测试钉死（换出口不许换归属，否则用量会静默记成 NULL）。为了看一眼「哪个
-- 中转站花得多」去改那条链路的函数签名，是拿钱的路径冒险换一个报表。
--
-- 这张表反过来：写它是火后不管的，丢几条无所谓，看板少算一点没有任何后果。
--
-- # 主键是 (天, 出口)
--
-- 存流水的话，一天一千多次调用、十个出口，一年就是几十万行，而看板只会问
-- 「今天/最近七天各花了多少」。按天聚合，一个出口一天一行，查询是主键命中。
CREATE TABLE IF NOT EXISTS endpoint_usage (
    day            DATE   NOT NULL,
    -- 出口 id；线路自带的那个地址用线路 id（和调度里的 health_id 同一套命名空间）。
    -- 不加外键：出口被删之后历史用量仍然有意义，而且这张表不该有能力阻止删除。
    endpoint_id    UUID   NOT NULL,
    route_id       UUID   NOT NULL,
    calls          BIGINT NOT NULL DEFAULT 0,
    -- 按扣给用户的钱记，微美元。不是进价——进价折扣只在选路时用，从不落库。
    cost_micro_usd BIGINT NOT NULL DEFAULT 0,
    prompt_tokens     BIGINT NOT NULL DEFAULT 0,
    completion_tokens BIGINT NOT NULL DEFAULT 0,
    cached_tokens     BIGINT NOT NULL DEFAULT 0,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (day, endpoint_id)
);

-- 看板按线路分组展示，所以按 route_id 也要能快速取一段时间。
CREATE INDEX IF NOT EXISTS idx_endpoint_usage_route_day
    ON endpoint_usage (route_id, day DESC);
