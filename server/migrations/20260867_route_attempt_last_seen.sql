-- 每个出口**最近一次真实成功／失败**的时刻。
--
-- 排序原来只看探测（`availability_tier`），探测失败就一律打成最差档，哪怕这个出口
-- 五分钟前刚成功服务过用户。线上实测「梦幻API」三个出口探测全部 20 秒超时被判死，
-- 而它同一天接了 241 次真实请求全部成功 —— 而它们恰好是最便宜的几个（进价系数
-- 0.15 / 0.24，还活着的那些是 0.6）。于是多路由省钱的效果基本没兑现。
--
-- 真实成功比探测硬：这条规矩本仓库别处已经写着（`own_order_key` 那段注释
-- 「那走的是执行事实，比任何探测都硬」），这里把它接到出口这一侧。
--
-- 两列都要。只记成功会变成棘轮：出口一旦失败被埋，就再也拿不到流量，也就再也
-- 刷新不了「最近成功」，于是永远埋着。两列都在，最近一次真实结果才说得出这个
-- 出口现在到底是活是死。
--
-- 单独开一个文件而不是改 20260866：那个已经在线上应用过了，改它等于改校验和，
-- sqlx 会直接拒绝启动（实测过：backend 反复重启，报 "was previously applied but
-- has been modified"）。
ALTER TABLE route_attempt ADD COLUMN IF NOT EXISTS last_ok_at   timestamptz;
ALTER TABLE route_attempt ADD COLUMN IF NOT EXISTS last_fail_at timestamptz;
