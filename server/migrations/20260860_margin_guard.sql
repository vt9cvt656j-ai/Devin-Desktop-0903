-- 看守判据从「涨幅百分比」换成「会不会亏钱」，并默认打开。
--
-- # 为什么百分比是错的判据
--
-- 涨 200% 的模型如果毛利有 10 倍，照样赚钱；涨 20% 的薄利模型可能当场变亏本。
-- 按百分比停既误杀又漏杀，而误杀的代价是停掉一条本来在赚钱的线路。
--
-- 换成「按新价重算成本，和同期实收比」之后，「不是恶意涨价的就没事」不再需要
-- 一条额外规则 —— 一次不威胁毛利的涨价按定义就不触发。
--
-- # 为什么默认开
--
-- 上一版默认关，理由是「停用可能断服，取舍该由人做」。那个理由在**负毛利**下不成立：
-- 每一次调用都在赔钱时，不停才是持续伤害。断服你当天就会发现，慢性失血不会。
ALTER TABLE endpoint_adapter ALTER COLUMN auto_guard SET DEFAULT true;
UPDATE endpoint_adapter SET auto_guard = true WHERE auto_guard = false;

-- 毛利低于这个比例就算「危险」，0 = 只在真亏了才动手。
-- 做成每出口可调：有人愿意用薄利换市场，有人不愿意。
ALTER TABLE endpoint_adapter
    ADD COLUMN IF NOT EXISTS margin_floor_pct DOUBLE PRECISION NOT NULL DEFAULT 0;
