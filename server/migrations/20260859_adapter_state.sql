-- 网关适配器的状态、自动拉到的价、以及价格变动史。
--
-- # 为什么状态要落库，不只放内存
--
-- 「这家认出来了吗、价拉到了几条、为什么被停」是**运维要看的事实**，不是缓存。
-- 放内存的话每次重启就空一次，而重启往往正发生在出事时；更要命的是「被自动停用」
-- 这件事必须能追溯到当时的判据，否则第二天没人知道它为什么被停。
CREATE TABLE IF NOT EXISTS endpoint_adapter (
    endpoint_id      UUID PRIMARY KEY,
    route_id         UUID NOT NULL,
    -- sub2api / new-api / one-api / one-api 系分支 / openrouter / 未知
    family           TEXT NOT NULL DEFAULT '',
    -- 命中的是哪一条指纹。认错时唯一能看出「它凭什么这么判」的地方。
    matched_by       TEXT NOT NULL DEFAULT '',
    -- 认不出来、或者认出来但拉不到价时的原因。**空字符串才表示没问题。**
    note             TEXT NOT NULL DEFAULT '',
    quota_per_unit   DOUBLE PRECISION,
    priced_models    INT  NOT NULL DEFAULT 0,
    balance_ok       BOOLEAN NOT NULL DEFAULT false,
    balance_text     TEXT NOT NULL DEFAULT '',
    -- 这家能不能做到**真实记账**：拉得到价、或者手填补齐了。
    -- false 意味着对账页上这一行的成本永远是未知 —— 那是要摆到人脸上的事。
    accounting_ready BOOLEAN NOT NULL DEFAULT false,
    -- 被自动停用的原因。空 = 没被停。
    blocked_reason   TEXT NOT NULL DEFAULT '',
    -- 涨价时要不要自动停用这条线路。**默认关**，理由见 relay_sync.rs 的模块注释：
    -- 停用唯一一条能服务某个模型的线路，等于用「不亏钱」换「直接断服」，
    -- 那个取舍必须由人来做。
    auto_guard       BOOLEAN NOT NULL DEFAULT false,
    synced_at        TIMESTAMPTZ,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 适配器自动拉到的真实进价，已归一成「美元每百万 token」。
--
-- 和手填的 endpoint_model_price 并存，**自动的优先**。手填降级成兜底：
-- 拉不到价的家族（one-api 没有公开价目接口）、或者站长关了价目广场时才用。
CREATE TABLE IF NOT EXISTS endpoint_auto_price (
    endpoint_id          UUID NOT NULL,
    model_id             TEXT NOT NULL,
    input_per_mtok       DOUBLE PRECISION NOT NULL,
    output_per_mtok      DOUBLE PRECISION NOT NULL,
    cached_per_mtok      DOUBLE PRECISION,
    cache_write_per_mtok DOUBLE PRECISION,
    per_request          DOUBLE PRECISION,
    -- 分组名和倍率。倍率**已经乘进上面的单价里**，留着是为了让界面能解释
    -- 「为什么这家比官网便宜 14 倍」——不留的话那个数字看起来像 bug。
    group_name           TEXT NOT NULL DEFAULT '',
    group_multiplier     DOUBLE PRECISION NOT NULL DEFAULT 1,
    source               TEXT NOT NULL DEFAULT '',
    fetched_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (endpoint_id, model_id)
);

-- 价格变动史。**抓恶意涨价靠它，不靠对比当前值。**
--
-- 只存当前价的话，涨价这件事在数据里根本不存在 —— 你只会看到一个新的价，
-- 而它看起来和一直就是这个价没有任何区别。
CREATE TABLE IF NOT EXISTS endpoint_price_change (
    id          BIGSERIAL PRIMARY KEY,
    endpoint_id UUID NOT NULL,
    model_id    TEXT NOT NULL,
    old_input   DOUBLE PRECISION,
    new_input   DOUBLE PRECISION,
    old_output  DOUBLE PRECISION,
    new_output  DOUBLE PRECISION,
    -- 涨幅，按输入输出里涨得更狠的那个算。负数 = 降价（也记，降价同样是信息）。
    pct         DOUBLE PRECISION NOT NULL,
    -- 当时做了什么：none / alarm / disabled / kept_last_route
    acted       TEXT NOT NULL DEFAULT '',
    at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_price_change_time ON endpoint_price_change (at DESC);
