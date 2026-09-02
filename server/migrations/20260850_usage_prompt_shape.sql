-- 各家 usage 回执里 prompt_tokens 的含义**不一样**，而下游（结算 API、IDE 的缓存命中率）
-- 一直拿它当同一个东西用：
--   · Anthropic 形状：prompt_tokens **不含**缓存读取（单列 cache_read_input_tokens）
--   · OpenAI / DeepSeek / GLM 形状：prompt_tokens **含**缓存读取
-- 于是「缓存命中率」的分母对 Claude 少算了命中的那部分，被结构性顶到 100%（线上实测
-- claude-sonnet-5 1463%、claude-fable-5 1332%），而同一个仪表上 GPT 报的是老实的低值。
-- 用户看着这两个不可比的数，得出「只有 Claude 有缓存」。
--
-- 形状只有**收到回执的那一刻**知道，事后从数字反推不出来（cached < prompt 时两种形状同形）。
-- 所以在这里落一位。默认 true = OpenAI 形状：历史行绝大多数是这一族，而且这个默认
-- 让老行的行为和修复前一致（分母 = prompt_tokens），不制造一次静默的历史数据跳变。
ALTER TABLE model_usage
  ADD COLUMN IF NOT EXISTS prompt_includes_cached BOOLEAN NOT NULL DEFAULT TRUE;
