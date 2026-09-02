-- 同一个中转地址下的**多个账号**要能各挂一个出口。
--
-- # 原来那条唯一索引挡错了东西
--
-- 20260851 建的 `idx_route_endpoints_unique_url (route_id, lower(base_url))` 想挡的是
-- 「手滑把同一个上游粘了两遍」——那确实有害：每个请求最多试两次出口
-- （CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED），两个尝试位被同一个上游占满，
-- 等于把故障转移悄悄变成没有。
--
-- 但它把「同地址、不同密钥」也一起挡掉了，而那是**完全正当**的：同一家转卖商开几个
-- 账号是常态，每个账号有自己的余额、自己的限速、自己的封禁状态。它们是真正独立的
-- 上游 —— 恰恰是故障转移最有价值的那种（额度耗尽、密钥失效都是**按密钥**发生的，
-- 换一个密钥就好了，而换地址救不了）。运维手上有几个同址账号就一个都用不上。
--
-- # 判据改成「地址 + 密钥」
--
-- 真正的重复是「同地址**且**同密钥」。密钥列是密文，而 field_crypto 用的是随机 nonce
-- （每次加密结果不同），所以唯一索引建在 `api_key` 上永远不会命中 —— 必须另存一个
-- **确定性指纹**。`key_fp` 就是它：明文密钥的 sha256（带域分隔前缀）取前 32 位十六进制，
-- 由写入路径计算，见 route_endpoints.rs 的 `key_fingerprint`。
--
-- 空密钥（= 沿用线路自己那把）的指纹是空串，所以「同地址 + 都不填密钥」仍然被当成重复
-- 挡下 —— 那确实是同一个上游粘了两遍。
--
-- # 为什么现在建索引是安全的
--
-- 存量行 key_fp 一律是 ''，而它们本来就满足 (route_id, lower(base_url)) 唯一，
-- 所以 (route_id, lower(base_url), '') 同样唯一，建索引不会失败。
-- 存量行的真实指纹由启动期的一次性回填补上（route_endpoints::spawn_key_fp_backfill）：
-- 它要解密，SQL 做不了。回填之前那些行只是「暂时按空指纹参与去重」，
-- 而它们两两之间地址本就不同，不会误判。

ALTER TABLE route_endpoints ADD COLUMN IF NOT EXISTS key_fp TEXT NOT NULL DEFAULT '';

DROP INDEX IF EXISTS idx_route_endpoints_unique_url;

CREATE UNIQUE INDEX IF NOT EXISTS idx_route_endpoints_unique_url_key
    ON route_endpoints (route_id, lower(base_url), key_fp);
