-- API key 不再以明文存库。
--
-- 原表是 `api_key TEXT NOT NULL UNIQUE` + 一条建在明文上的索引。任何一次只读的库暴露
-- （备份、只读副本、pg_dump 误配、一条 SQL 注入）都直接产出一批**可立即使用**的网关
-- 凭据：持有者以受害者身份跑 /v1/chat/completions，费用记受害者钱包，还允许记成负债。
-- 而同一份代码对上游供应商的 key 一直是加密存的（field_crypto，MODEL_KEY_CTX），
-- 管理端也做了 mask —— 标准不一致本身就是这条的证据。
--
-- 为什么不能只存哈希：GET /api/ide-key 要**把同一把 key 原样还给登录用户**（IDE 自动
-- 配置用，跨设备跨会话必须稳定）。哈希是单向的，只存哈希这个接口就废了。所以两列：
--
--   api_key_sha256  校验用。确定性 → 可建索引 → 鉴权仍是一次索引命中，不用全表解密。
--                   它是单向的，库泄漏拿不到能用的凭据。
--   api_key_enc     回显用。field_crypto 加密（随机 nonce，不可索引），只有 ide-key
--                   这条路会解它。没有 MSE 密钥就解不开。
--
-- **本次迁移只加列、不删列，且新列可空**：回滚到旧二进制时旧代码照常读 api_key，
-- 线上用户无感。明文的清除由 API_KEY_PURGE_PLAINTEXT=1 单独一次部署完成（见
-- docs/OPERATIONS.md），确认无恙之后再做。

ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS api_key_sha256 TEXT;
ALTER TABLE api_keys ADD COLUMN IF NOT EXISTS api_key_enc    TEXT;

-- 明文列改为可空：清除阶段要把它置 NULL，而原定义是 NOT NULL。
-- 唯一约束一并去掉（它建在明文上；唯一性改由 sha256 保证）。
ALTER TABLE api_keys ALTER COLUMN api_key DROP NOT NULL;
ALTER TABLE api_keys DROP CONSTRAINT IF EXISTS api_keys_api_key_key;

-- 鉴权走这条索引。唯一：同一把 key 不该对应两个账号。
CREATE UNIQUE INDEX IF NOT EXISTS idx_api_keys_sha256 ON api_keys (api_key_sha256);

-- 明文那条索引留到清除阶段再删：过渡期回退查询还要用它。
