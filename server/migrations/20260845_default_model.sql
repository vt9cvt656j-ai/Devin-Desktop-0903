-- 新装客户端开箱用哪个模型。
--
-- 在这之前，客户端取的是 /api/models 返回的**第一个** —— 而那个顺序是路线的
-- enabled_models 按字母排出来的，于是新用户永远落在 claude-fable-5 上。实测（2026-08-19，
-- 40 小时）它的硬失败率 18.8%，是在售模型里最高的一档；而 claude-opus-5 是 3.6%、
-- glm-5.3 是 0%。也就是说「模型老是用不了」这件事，对每一个新用户都是开箱即得的。
--
-- 做成配置而不是写死在客户端：模型名一年换好几茬（这张目录里已经有 52 个用过的名字），
-- 写死意味着每次换默认都要发一次桌面版。空串 = 沿用旧行为（取第一个）。
ALTER TABLE app_settings ADD COLUMN IF NOT EXISTS default_model text NOT NULL DEFAULT '';
