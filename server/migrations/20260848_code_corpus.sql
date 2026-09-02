-- 自有代码语料库：把「全网公开库的真实 API」变成**这台服务器自己的**索引。
--
-- # 为什么要有它
--
-- 现有三层各有各的边界：
--   · knowledge_search —— 65 篇手写语料，讲经验和坑，不是 API 手册，也追不上全网；
--   · package_source   —— 只读**本机装了的**包。没装就明确回一句「未安装」，然后没有下文；
--   · package_search   —— 注册表元数据，按它自己的说明「returns NO signatures」。
-- 于是「这个没装的库，某个函数真实签名是什么」在整套工具里**无解**，模型只能靠训练记忆猜——
-- 那正是它编 API 的地方。
--
-- # 为什么不是镜像全网
--
-- 一台 97G 的机器装不下 GitHub。但模型要的从来不是整包，是 **API 表面**：导出了什么、
-- 签名长什么样、文档注释怎么说。这部分一个包几十到几百 KB，一万个包也就 1~2G——存得下，
-- 而且信噪比远高于整份源码。
--
-- # 「继承」在哪
--
-- 语料按**真实需求**生长：谁问到一个还没有的包，就现拉、抽取、入库，此后**永久留下**。
-- 用得越久覆盖越全，而且这份索引长在自己机器上，不依赖任何第三方服务。
--
-- # 检索为什么用内置全文 + trigram
--
-- 这台 Postgres 是 postgres:17-alpine，`pg_available_extensions` 里只有 pg_trgm 和 unaccent，
-- **没有 pgvector**。所以 v1 用内置 tsvector（英文配置，API 文本基本是英文）做相关性，
-- 外加 pg_trgm 做符号名的模糊匹配（记岔一两个字母也能命中）。
-- 将来要上向量检索，换 pgvector 镜像后加一列即可，这张表的其它部分不用动。

CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE IF NOT EXISTS code_corpus (
    id          BIGSERIAL PRIMARY KEY,
    -- 'package_api' = 某个导出符号的签名+文档；'package_readme' = 包的说明节选。
    -- 留成文本而不是枚举：以后加 'doc' / 'guide' 这类通用知识时不用改表。
    kind        TEXT NOT NULL,
    ecosystem   TEXT NOT NULL,              -- npm | pypi | crates | ...
    name        TEXT NOT NULL,              -- 包名，按 import 写法
    version     TEXT NOT NULL DEFAULT '',   -- 抽取时的确切版本；空=未取到
    symbol      TEXT NOT NULL DEFAULT '',   -- 导出符号名；'' = 包级条目（README/导出清单）
    title       TEXT NOT NULL DEFAULT '',
    body        TEXT NOT NULL,              -- 签名 + 文档注释，或 README 的一节
    source_url  TEXT NOT NULL DEFAULT '',   -- 出处，便于人工核对
    fetched_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- 相关性用内置英文配置；title 权重高于 body。
    tsv tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(name, '') || ' ' || coalesce(symbol, '')), 'A')
        || setweight(to_tsvector('english', coalesce(title, '')), 'B')
        || setweight(to_tsvector('english', coalesce(body, '')), 'C')
    ) STORED
);

-- 同一个包同一个版本的同一个符号只留一条：重复入库是 no-op，重抓新版本才新增。
CREATE UNIQUE INDEX IF NOT EXISTS idx_code_corpus_identity
    ON code_corpus (ecosystem, name, version, kind, symbol);

CREATE INDEX IF NOT EXISTS idx_code_corpus_tsv ON code_corpus USING GIN (tsv);
-- 符号名模糊匹配：`useQeury` 也要能找到 `useQuery`。
CREATE INDEX IF NOT EXISTS idx_code_corpus_symbol_trgm
    ON code_corpus USING GIN (symbol gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_code_corpus_name_trgm
    ON code_corpus USING GIN (name gin_trgm_ops);
-- 「这个包收录了没有、什么时候抓的」——按需入库那条路每次都要问一次。
CREATE INDEX IF NOT EXISTS idx_code_corpus_pkg ON code_corpus (ecosystem, name, fetched_at DESC);

-- 抓取台账：成功、失败、空包都留痕。没有它，一个抓不到的包会被反复重抓，
-- 而失败原因（包不存在 / 没有类型声明 / 体积超限）只会在日志里一闪而过。
CREATE TABLE IF NOT EXISTS code_corpus_fetches (
    ecosystem   TEXT NOT NULL,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL DEFAULT '',
    ok          BOOLEAN NOT NULL,
    entries     INT NOT NULL DEFAULT 0,     -- 这次入库多少条
    bytes       BIGINT NOT NULL DEFAULT 0,  -- 下载了多少字节
    error       TEXT,
    fetched_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (ecosystem, name, version)
);
CREATE INDEX IF NOT EXISTS idx_code_corpus_fetches_at ON code_corpus_fetches (fetched_at DESC);
