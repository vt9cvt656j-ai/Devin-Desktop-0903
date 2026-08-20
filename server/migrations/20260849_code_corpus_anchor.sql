-- 无符号条目（文档节、README 节）在唯一索引里全撞成一条。
--
-- 原索引是 (ecosystem, name, version, kind, symbol)，而文档节和 README 节的 symbol 恒为空
-- ——于是同一个源的几百上千节共用同一个键，`ON CONFLICT DO NOTHING` 只留下第一条。
-- 实测：react 文档抽出 970 节，实际入库 **1 条**；七个文档源各 1 条。包的 README 同理。
--
-- 而且它当时看不出来：入库计数用的是 `res.is_ok()`，而冲突跳过时 sqlx 照样返回 Ok，
-- 于是日志理直气壮地报 "sections=970"。数字对不上是查库才发现的。
--
-- 修法是给每条一个**稳定的段内锚点**：有符号的用符号，没符号的用小节标题。
-- 单独一列而不是塞进 symbol：symbol 上挂着 trigram 索引做符号名模糊匹配，
-- 把整句标题塞进去会把那个索引搅浑。

ALTER TABLE code_corpus ADD COLUMN IF NOT EXISTS anchor TEXT NOT NULL DEFAULT '';

-- 回填历史行。旧索引保证了 (…, symbol) 唯一，而新键是它的超集，所以回填不会撞。
UPDATE code_corpus
   SET anchor = CASE WHEN symbol <> '' THEN symbol ELSE left(title, 200) END
 WHERE anchor = '';

DROP INDEX IF EXISTS idx_code_corpus_identity;
CREATE UNIQUE INDEX IF NOT EXISTS idx_code_corpus_identity
    ON code_corpus (ecosystem, name, version, kind, anchor);
