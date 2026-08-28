-- 每条线路 / 每个模型的**真实结果**：成不成、多快。按天汇总。
--
-- # 为什么必须有这张表
--
-- 在它之前，「这条线路好不好」只有两个来源，两个都不回答那个问题：
--
--   1. `model_health` —— 对 base_url 发一个**不带凭据的 GET**，问的是「门在不在」。
--      密钥过期、额度用尽、模型下架，它一律报绿。实测「Claude 强力版」24 小时前门可达率
--      99.93%、平均 179ms，而它**真实成功是 43 小时前**。假绿灯。
--   2. Redis 里的连败计数 —— 按线路，没有模型维度、没有历史、30 天 TTL。
--      算不出成功率，也说不出「昨天比今天差」。
--
-- 成功那一半其实一直有（model_usage 每次扣费都写一行），但**失败从不落库**，
-- 于是分母永远缺一块，成功率无从谈起。这张表把两半记在一起。
--
-- # 为什么按天汇总而不是一行一次
--
-- 派单路径上不能多一次同步写。这里和 endpoint_model_usage 同一个形状：
-- 一次 upsert、tokio::spawn 出去、写失败不影响请求。
--
-- endpoint_id 沿用 `health_id()` 的两个命名空间（出口用 route_endpoints.id，
-- 线路自带地址用 models.id），和 endpoint_balance 一致。
CREATE TABLE IF NOT EXISTS route_attempt (
  day          date        NOT NULL DEFAULT current_date,
  endpoint_id  uuid        NOT NULL,
  model_id     text        NOT NULL,
  ok_calls     bigint      NOT NULL DEFAULT 0,
  fail_calls   bigint      NOT NULL DEFAULT 0,
  -- 最后一次失败的状态码。不做直方图：排查时想知道的是「现在是 401 还是 502」。
  last_status  int,
  -- 表头往返耗时，只在成功时累加。用 sum/n 而不是存每一次：
  -- 均值够回答「这条比那条慢多少」，而存每一次要另一张按次表。
  ttfb_ms_sum  bigint      NOT NULL DEFAULT 0,
  ttfb_ms_n    bigint      NOT NULL DEFAULT 0,
  updated_at   timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (day, endpoint_id, model_id)
);

CREATE INDEX IF NOT EXISTS route_attempt_day_idx ON route_attempt (day DESC);
