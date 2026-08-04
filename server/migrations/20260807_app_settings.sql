-- 运营参数落库。在此之前，三个数字被硬编码在四个文件里（models.rs 的 6.63、
-- FREE_POINTS_DAILY=40、codes.rs 的 plan_spec），前端另有三份 663 的独立副本。
-- 改一处不改其余，管理端发出的额度、客户端显示的余额、免费点的定价会互相打架。
-- 这张表是唯一真相；Rust 启动时读一次进内存，写入后失效重载。

CREATE TABLE IF NOT EXISTS app_settings (
  -- 单行表：id 只能是 1。用 CHECK 而不是靠约定，避免出现第二行后读到哪一行全凭运气。
  id smallint PRIMARY KEY DEFAULT 1 CHECK (id = 1),

  -- 面值分母：663 真实计费分 = 客户看到的 $1.00 额度。这是"混合汇率"，
  -- 不是人民币汇率——它是卖出的 1 美元额度对应的上游真实成本美元数。
  -- 下限 1：这个数在展示路径上做除数，0 会让每个余额变成除零。
  raw_cents_per_credit_usd integer NOT NULL DEFAULT 663
    CHECK (raw_cents_per_credit_usd BETWEEN 1 AND 100000),

  -- 每日赠送点数。0 合法（等于关掉免费额度），上限防手滑多打几个零。
  free_points_daily integer NOT NULL DEFAULT 40
    CHECK (free_points_daily BETWEEN 0 AND 1000000),

  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by text NOT NULL DEFAULT ''
);

INSERT INTO app_settings (id) VALUES (1) ON CONFLICT (id) DO NOTHING;

-- 套餐额度。原先是 codes.rs::plan_spec 里的 match 分支，改一次要重新编译部署。
-- 单位与 users.quota_*_cents / credits_cents 一致：真实计费分（不是面值分）。
CREATE TABLE IF NOT EXISTS plan_quotas (
  plan text PRIMARY KEY,

  total_cents  bigint NOT NULL CHECK (total_cents  >= 0),

  -- 严格大于 0。plan_spec 的注释写明：quota_ok 要求 q_window > 0，所以 0 上限不是
  -- "不限"，而是把套餐永久锁死，用户只会看到一个永远刷不出额度的提示。
  -- 这条 CHECK 就是把那段注释变成数据库拦得住的规则。
  window_cents bigint NOT NULL CHECK (window_cents >  0),

  weekly_cents bigint NOT NULL CHECK (weekly_cents >= 0),
  days integer NOT NULL CHECK (days > 0),

  -- 套餐高低次序，发放时不会把持有更好套餐的用户降级。
  rank integer NOT NULL CHECK (rank > 0),

  updated_at timestamptz NOT NULL DEFAULT now()
);

-- 种子值逐字对应 plan_spec 当时的取值，迁移后行为不变。
INSERT INTO plan_quotas (plan, total_cents, window_cents, weekly_cents, days, rank) VALUES
  ('trial',   5000,  5000, 0,  1, 1),
  ('basic',  33000,  3000, 0, 30, 2),
  ('pro',    65000,  6000, 0, 30, 3),
  ('power', 180000, 15000, 0, 30, 4),
  ('ultra', 500000, 30000, 0, 30, 5)
ON CONFLICT (plan) DO NOTHING;
