-- 真实成本要的两样东西：**按模型分开的用量**，和**中转的真实单价**。
--
-- # 为什么不能沿用 endpoint_usage
--
-- 那张表的主键是 `(day, endpoint_id)` —— 一个出口一天一行，所有模型的 token 混在一起。
-- 成本是 `token × 该模型的单价`，而同一个出口上 opus 和 haiku 的单价差两个量级，
-- 混着的总 token 乘任何一个单价都得不到真实数字。所以必须有模型这一维。
--
-- endpoint_usage 保留不动：健康面板在用它，而且它有历史。两张表由同一个调用点同时写，
-- 数字同源；重复的代价是一次多余的 upsert，换的是不动一块正在工作的屏幕。
CREATE TABLE IF NOT EXISTS endpoint_model_usage (
    day               DATE   NOT NULL,
    -- 出口 id；线路自带地址用线路 id（和 endpoint_usage / health_id 同一套命名空间）。
    endpoint_id       UUID   NOT NULL,
    route_id          UUID   NOT NULL,
    -- 模型名（`claude-opus-5` 这种），不是线路 id。单价是按它定的。
    model_id          TEXT   NOT NULL,
    calls             BIGINT NOT NULL DEFAULT 0,
    -- 从用户身上收到的，微美元。收入这一侧，和计费同源。
    revenue_micro_usd BIGINT NOT NULL DEFAULT 0,
    -- 真实 token。上游自己在 usage 帧里报的，我们扣用户的钱就是按它算的。
    prompt_tokens     BIGINT NOT NULL DEFAULT 0,
    completion_tokens BIGINT NOT NULL DEFAULT 0,
    -- 命中缓存的那部分输入 token。它**包含在** prompt_tokens 里，算钱时要减出来
    -- 单独按缓存价乘 —— 不减的话，缓存命中率高的模型成本会被高估好几倍。
    cached_tokens     BIGINT NOT NULL DEFAULT 0,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (day, endpoint_id, model_id)
);

CREATE INDEX IF NOT EXISTS idx_endpoint_model_usage_day
    ON endpoint_model_usage (day DESC, endpoint_id);

-- 这个出口卖给我们的真实单价。**运维从中转后台的价目页抄进来的事实，不是推算。**
--
-- # 为什么价定在「出口」上而不是「线路」上
--
-- 多路由的全部意义就是同一个模型在不同出口有不同进价 —— 那正是要比的东西。
-- 定在线路上的话，三个出口共用一份价，对账就退化回「所有出口一样贵」，
-- 而这恰好是它要证伪的那个假设。
--
-- # 单位是「每百万 token 多少美元」
--
-- 和目录价、和界面上显示的售价同一个单位。换算只在一个地方做（除以 1e6），
-- 两种单位混着走是这类表最经典的错法。
--
-- # 没录价 ≠ 免费
--
-- 查不到单价时对账页显示「未知」并把这一行排除在合计之外，绝不按 0 计入。
-- 按 0 算的话，没录价的出口会显示成 100% 毛利，而它可能正是亏得最狠的那个。
CREATE TABLE IF NOT EXISTS endpoint_model_price (
    endpoint_id     UUID NOT NULL,
    model_id        TEXT NOT NULL,
    -- 每百万输入 token 多少美元。
    input_per_mtok  DOUBLE PRECISION NOT NULL,
    -- 每百万输出 token 多少美元。
    output_per_mtok DOUBLE PRECISION NOT NULL,
    -- 每百万「命中缓存的输入 token」多少美元。NULL = 这家不单独计缓存价，按输入价算。
    cached_per_mtok DOUBLE PRECISION,
    -- 抄自哪儿 / 什么时候确认的。价格会变，而变了之后历史成本是错的 —— 这一栏是
    -- 唯一能看出「这个数字有多旧」的地方。
    note            TEXT NOT NULL DEFAULT '',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (endpoint_id, model_id)
);
