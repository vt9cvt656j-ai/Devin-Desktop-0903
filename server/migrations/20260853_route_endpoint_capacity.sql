-- 出口能扛多少。
--
-- # 它只在「首选被限流、要挑替补」时起作用
--
-- 平时所有流量都走最便宜那个（进价折扣是每次调用都复现的真金，主动分散在成本上从不占优）。
-- 只有首选正在让位时，才需要在幸存出口里挑一个 —— 而那时只看价格是不够的：
-- 一个 20 RPM 的小转卖和一个 4000 RPM 的官方直连，光按价格权重会把溢出全灌给小的，
-- 它立刻也满了，于是雪崩推到下一跳。
--
-- # 为什么可空，以及空了怎么办
--
-- 这个数要去每家转卖商那儿查，多数人不会填，填错的比不填的还多。所以：
--   · 全都没填 → 一律按 1 算，行为和加这一列之前完全一样；
--   · 有人填了、有人没填 → 没填的按**已填里的最小值**算。
-- 保守方向：不知道能扛多少，就当它是最不能扛的那个。反过来（当成最大）会让一个
-- 没人填过的出口吃掉全部溢出。
--
-- 单位随你，只要同一条线路下的几个出口用同一把尺（RPM、并发数、随便一个相对值都行）——
-- 算法只看它们之间的**比值**。
ALTER TABLE route_endpoints
    ADD COLUMN IF NOT EXISTS capacity DOUBLE PRECISION;

-- 0 和负数没有意义（会让这个出口永远拿不到溢出，那该用「停用」表达，不是填 0）。
-- 用 NOT VALID 加约束：蓝绿重叠期新旧两版共用一个库，老版本不知道这一列，
-- 立刻校验存量会在滚动过程中卡住写入。
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'route_endpoints_capacity_positive'
    ) THEN
        ALTER TABLE route_endpoints
            ADD CONSTRAINT route_endpoints_capacity_positive
            CHECK (capacity IS NULL OR capacity > 0) NOT VALID;
    END IF;
END $$;
