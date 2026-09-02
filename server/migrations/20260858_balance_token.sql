-- 查余额用的凭据，和调用密钥分开存。
--
-- # 为什么必须分开
--
-- 实测（2026-08-25，线上那三家中转都是同一套自建系统）：余额在
-- `/api/v1/auth/me` 和 `/api/v1/subscriptions/summary`，而它们要的是**控制台登录令牌**，
-- 不是 `sk-` 开头的调用密钥 —— 两者是不同的凭据体系。拿调用密钥去问，7 个出口
-- 一个都查不到，而对账页的成本那一侧就永远空着。
--
-- 空 = 没配，那就退回去用调用密钥试（有些中转两者通用）。两条都试不出来才算查不到。
--
-- # 和调用密钥同一套加密
--
-- 用 MODEL_KEY_CTX 加密，解密走 models::model_key。换一个 context 的话密钥轮换
-- 会漏掉这一列，症状是「某天所有余额同时查不到」——而那时候没人会想到是轮换。
ALTER TABLE route_endpoints ADD COLUMN IF NOT EXISTS balance_token TEXT NOT NULL DEFAULT '';
ALTER TABLE models          ADD COLUMN IF NOT EXISTS balance_token TEXT NOT NULL DEFAULT '';
