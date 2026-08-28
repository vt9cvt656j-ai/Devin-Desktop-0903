-- 这一份用量里，`prompt_tokens` 到底含不含缓存读取。
--
-- 两家的回执形状不一样，而且**事后从数字反推不出来**（cached < prompt 时两种形状
-- 完全同形）：Anthropic 单列 cache_read_input_tokens、prompt 不含它；
-- OpenAI/DeepSeek/GLM 的 prompt 含。计费那边早就有这一位（BillTokens 的
-- prompt_includes_cached，取 `cache_read_input_tokens` 在不在），也一路存进了
-- model_usage —— 唯独没有带到 endpoint_model_usage，而对账页读的正是这张表。
--
-- # 漏掉它的后果
--
-- 对账那边只好硬夹一刀：`cached = min(cached, prompt)`。对 OpenAI 形状是对的，
-- 对 Anthropic 形状就把超出的缓存读**整段丢掉**。线上实测最近 7 天：
--   claude-fable-5  输入 764,233   缓存读 10,818,782 → 丢掉 10,054,549
--   claude-opus-5   输入 569,259   缓存读  6,436,239 → 丢掉  5,866,980
-- 一共 1590 万个缓存读 token 从成本里凭空消失，而且顺带把那 30 个新鲜输入
-- 也按缓存价算了。方向是单向的：**成本低估、毛利高估**，缓存命中率越高错得越狠。
--
-- 回填按模型名从 model_usage 取，用 bool_and —— 和 models.rs 里那句
-- `COALESCE(bool_and(prompt_includes_cached), true)` 同一个口径：只要有一次是
-- Anthropic 形状就按 Anthropic 算。反过来（bool_or）会把混合情况错判成含缓存，
-- 又变成低估成本，正是这里要修的那个方向。
ALTER TABLE endpoint_model_usage ADD COLUMN IF NOT EXISTS prompt_includes_cached BOOLEAN;

UPDATE endpoint_model_usage e
   SET prompt_includes_cached = s.v
  FROM (SELECT model_name, bool_and(prompt_includes_cached) AS v
          FROM model_usage
         WHERE model_name <> ''
         GROUP BY model_name) s
 WHERE s.model_name = e.model_id
   AND e.prompt_includes_cached IS NULL;
