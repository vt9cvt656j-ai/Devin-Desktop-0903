-- 探活自己烧掉的 token。
--
-- # 为什么必须记
--
-- `canary_once` 每 15 分钟对每条线路发一次**真实推理请求**（max_tokens:1）——
-- 那是真花钱的，但它从不进 `endpoint_usage`（那张表只记用户流量）。
--
-- 实测（2026-08-25 23:00 前后）：19:02 之后零用户流量，而账户余额仍在下降。
-- 差额的嫌疑就是探活。不记的话有两个后果：
--   · 对账里永远有一块解释不了的差额，久了就会有人默认「对账本来就对不齐」，
--     那样整页就废了；
--   · 更要命的是「按余额差反推单价」那条路会被污染 —— 探活的钱被摊到用户 token 上，
--     算出来的单价偏高，而它看起来完全正常。
--
-- 这张表是纯观测，不参与计费，也不产生收入 —— 探活没有用户为它付钱，
-- 所以它只有成本没有收入，这正是它必须单独一张表而不是塞进 endpoint_usage 的原因。
CREATE TABLE IF NOT EXISTS endpoint_probe_usage (
    day               DATE   NOT NULL,
    -- 探的是线路自带地址，所以这里就是线路 id（health_id 命名空间）。
    endpoint_id       UUID   NOT NULL,
    route_id          UUID   NOT NULL,
    model_id          TEXT   NOT NULL,
    calls             BIGINT NOT NULL DEFAULT 0,
    prompt_tokens     BIGINT NOT NULL DEFAULT 0,
    completion_tokens BIGINT NOT NULL DEFAULT 0,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (day, endpoint_id, model_id)
);
