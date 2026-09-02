-- 后台按模型手填的能力兜底。
--
-- 实时目录（OpenRouter）没收录的模型——比如 glm-5.3，目录里只有 5.1/5.2/5-turbo——
-- 在硬编码表删掉之后就没有任何窗口数据了。兜底本身是需要的，只是**不该由代码编**：
-- 那张被删的表实测在售 13 款里错了 6 款，它的问题正是"没人知道它错了还在自信地用"。
--
-- 由运维在后台填就没有这个问题：谁填的、什么时候填的、对不对，填的人自己清楚，
-- 而且随时能改，不用发版。
--
-- 形状和 model_prices 一致：{ "glm-5.3": { "contexts": [128000], "max_output": 64000 } }
ALTER TABLE models
  ADD COLUMN IF NOT EXISTS model_caps jsonb NOT NULL DEFAULT '{}'::jsonb;
