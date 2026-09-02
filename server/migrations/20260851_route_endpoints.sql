-- 一条线路可以挂多个上游（多路由）。
--
-- # 为什么是新表，而不是在 models 里多开几行
--
-- models 里多开几行本来是能跑的：chat 的候选池就是「enabled_models 里含这个模型」的所有
-- 行，故障转移、冷却、健康判定都现成。但计费读的是**真正答复的那一行**
-- （models.rs 里 `match (success, selected_conn)`），而价格字段是**每行各一份**。
-- 于是同一个模型，用户被扣多少钱要看当时哪家转卖商先答 —— 运维每加一个上游，就多一次
-- 把某些用户悄悄按另一个价计费的机会，而界面上看不出来。
--
-- 所以这里把两件事拆开：**价格、开放模型、用量归属留在 models**（一条线路一个身份），
-- 这张表只装「往哪儿发、用哪个密钥、我进价多少」。失败转移换的只是出口，
-- 换不动账单 —— 这是结构上的保证，不是靠运维记得把几行的价格填成一样。
--
-- # cost_ratio 是进价折扣，不是卖价
--
-- 转卖商报价的方式就是折扣（「三折」= 0.3），而且对全部模型同时成立，所以一个数就够，
-- 不需要按模型逐个填。它**只参与排序**，一分钱都不进用户账单：卖价在 models 上。
-- 默认 1.0 = 原价，正好让线路自带的那个地址在没填时不占便宜也不吃亏。
--
-- # 探测结果为什么落库而不是只放 Redis
--
-- route_health 那套（连败数、上次成功时刻）是**真实流量的结局**，放 Redis 是对的：
-- 每条线路一行、亚毫秒、不占连接池。这里存的是另一回事 —— 「运维点了测一下，
-- 结果是什么」，要在后台列表里看得见、要能解释为什么这个上游被排到后面，
-- 而且加完一个上游隔一天回来还得看得到。这是配置的一部分，不是观测流水。
CREATE TABLE IF NOT EXISTS route_endpoints (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- 线路被删时端点跟着走：留着的话就是一堆指不到任何线路的密钥。
    route_id   UUID NOT NULL REFERENCES models (id) ON DELETE CASCADE,
    -- 给人看的备注，例如「转卖A / 备用」。空着也能用。
    label      TEXT NOT NULL DEFAULT '',
    base_url   TEXT NOT NULL,
    -- 和 models.api_key 同一套加密（context = models.api_key），一次轮换覆盖两边。
    api_key    TEXT NOT NULL DEFAULT '',
    -- 进价折扣：0.3 = 三折。只排序，不计费。
    cost_ratio DOUBLE PRECISION NOT NULL DEFAULT 1.0 CHECK (cost_ratio > 0),
    active     BOOLEAN NOT NULL DEFAULT true,
    note       TEXT NOT NULL DEFAULT '',
    -- 最近一次探测。NULL = 还没测过，和「测过并且失败」是两回事，
    -- 所以用可空布尔，不用 false 兼作「没测过」。
    probe_ok   BOOLEAN,
    probe_at   TIMESTAMPTZ,
    probe_ms   INTEGER,
    probe_note TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 取候选时的形状就是「这条线路的、还开着的、按进价从便宜到贵」。
CREATE INDEX IF NOT EXISTS idx_route_endpoints_pick
    ON route_endpoints (route_id, active, cost_ratio);

-- 同一条线路下同一个地址只留一份：手滑粘两遍会让同一个上游占掉两个尝试位，
-- 而每个请求最多只试两次出口（CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED），
-- 等于把故障转移悄悄变成没有。
CREATE UNIQUE INDEX IF NOT EXISTS idx_route_endpoints_unique_url
    ON route_endpoints (route_id, lower(base_url));
