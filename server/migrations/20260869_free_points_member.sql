-- 每日赠送分两档：会员一档，非会员一档。
--
-- 在这之前只有 free_points_daily 一个数，绑在四个「懒发放」点上（auth.rs 的 /api/me，
-- models.rs 的 free_points_balance / spend_free_points / try_spend_free_points）。
-- 加一档必须四处一起改：漏一处的症状不是报错，是某条路径上的会员静默地拿回旧额度。
--
-- 为什么可空，而不是 NOT NULL DEFAULT 40：
--   NULL = 「没单独配」= 跟随 free_points_daily，也就是**今天的行为逐字不变**。
--   线上这个数已经被运营改成 100 了。给新列一个 40 的 NOT NULL 默认值，等于在迁移
--   跑完的那一刻把会员从 100 降到 40 —— 一次没人要求过的降级，而且不报错。
--   可空还盖住另外两个窗口：新二进制先上、这张表还没加列时读不到；以及这一列读取
--   失败时。两种情况服务端都当「没配」处理，两档合一，仍然是今天的行为。
--
-- 也**故意不做** `UPDATE ... SET free_points_daily_member = free_points_daily` 这种
-- 「种下当前值」的回填：那会把两档的联动一次性冻结，之后运营把普通档改到 200，会员
-- 还停在 100，而后台上看不出这是「跟随」还是「配过」。NULL 一直答得对这个问题。
--
-- 为什么是绝对值而不是倍数：倍数下把 free_points_daily 改成 0（＝关掉免费额度）会
-- 连带把会员福利清零，而后台上会员那格还写着「×3」，看不出来。
--
-- 区间与 free_points_daily 那一列一致（0 合法 = 关掉会员的免费额度；上限防手滑多打零）。
ALTER TABLE app_settings
  ADD COLUMN IF NOT EXISTS free_points_daily_member integer
    CHECK (free_points_daily_member IS NULL
           OR free_points_daily_member BETWEEN 0 AND 1000000);
