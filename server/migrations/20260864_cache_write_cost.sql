-- 缓存写入是成本大头，而对账一直把它当 0。
--
-- # 实测
--
-- 2026-08-26 04:26:46 的一次 claude-opus-5 调用：新鲜输入 381 个 token、
-- **缓存写入 61,634 个**、输出 1152 个。上游按输入价的 1.25 倍收缓存写入
-- （$18.75/M），这一笔的成本几乎全在写入上 —— $1.16 里有 $1.156 是它。
--
-- 而 `endpoint_model_usage` 连这一列都没有，`model_cost_usd` 也只算
-- 「新鲜输入 + 缓存读 + 输出」。后果是**「中转收了」被系统性低估、毛利被高估**，
-- 而且低估的幅度取决于缓存命中率 —— 命中率越高的模型，账越漂亮，实际越亏。
--
-- 用户那一侧一直是对的（`compute_cost` 一直在算 cache_creation），
-- 错的只有对账这一侧：收入算了、成本没算。
ALTER TABLE endpoint_model_usage
    ADD COLUMN IF NOT EXISTS cache_creation_tokens BIGINT NOT NULL DEFAULT 0;
ALTER TABLE endpoint_usage
    ADD COLUMN IF NOT EXISTS cache_creation_tokens BIGINT NOT NULL DEFAULT 0;

-- 手录进价也要能填缓存写入价。自动抓来的那张表（endpoint_auto_price）本来就有
-- cache_write_per_mtok，手录这张一直没有 —— 于是手录价永远算不出写入成本。
-- NULL = 没录，按「输入价 × 1.25」推（上游普遍的倍数），而不是按 0。
-- 按 0 算就是这次这个 bug 的原样重演。
ALTER TABLE endpoint_model_price
    ADD COLUMN IF NOT EXISTS cache_write_per_mtok DOUBLE PRECISION;
