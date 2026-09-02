-- 模型能力目录（实时抓来的，不是代码里写死的那张表）。
--
-- 为什么要落库而不是只放内存：网关重启、或者上游目录源当时不可达时，仍然要有**上一次
-- 抓到的真实值**可用。只有内存缓存的话，一次重启就退回硬编码表，而硬编码表实测 13 款里
-- 错了 6 款（deepseek-v4-flash 128K vs 真实 1.05M，少 88%）。
--
-- norm_id 是归一化后的模型名（小写、去掉 - . _），因为中转商的命名和目录源的命名对不齐：
-- 我们卖的叫 `claude-opus-4-6`，目录源叫 `anthropic/claude-opus-4.6`。
CREATE TABLE IF NOT EXISTS model_catalog (
  norm_id     text PRIMARY KEY,
  source_id   text        NOT NULL,
  -- 全部原生上下文档位（升序）。是列表不是单值：Sonnet 4 真的同时提供 200K 和 1M，
  -- 塌缩成一个数就把一个真实的选择藏掉了。
  contexts    jsonb       NOT NULL DEFAULT '[]'::jsonb,
  -- 该模型支持的推理档位，如 ["max","xhigh","high","medium","low"]。
  -- 空数组 = 这个模型根本不吃推理档位（实测 glm-5 就是），客户端不该给它显示档位选择。
  efforts     jsonb       NOT NULL DEFAULT '[]'::jsonb,
  default_effort text,
  max_output  bigint,
  -- 单位统一 USD / 1M tokens（和 models 表、official_price 一致）。
  -- NULL = 目录源没给这一项，**不是 0**：0 会被下游当成免费。
  input_price       double precision,
  output_price      double precision,
  -- 缓存读/写的真实价。以前是按输入价 ×0.1 / ×1.25 推算的，实测偏差很大
  -- （deepseek-v4-flash 缓存读真实 0.0123、推算 0.0061，少算一半 = 实际多付）。
  cache_read_price  double precision,
  cache_write_price double precision,
  updated_at  timestamptz NOT NULL DEFAULT now()
);
