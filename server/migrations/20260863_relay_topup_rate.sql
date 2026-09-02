-- 每家中转的充值汇率：人民币进去，多少「上游余额」出来。
--
-- # 为什么这一列必须存在
--
-- `route_endpoints.cost_ratio` 是**相对官方价的倍数**，但那个倍数的单位是
-- **那家中转自己的余额单位**。而一块钱余额值多少人民币，各家差几十倍。
-- 于是「0.05 倍」和「1.0 倍」谁便宜，光看倍率根本答不出来 —— 那是拿两种货币比大小。
--
-- 线上就有这个形状：mhapi 的出口写着 0.05 倍，hanhegufei 的自带地址是 1.0 倍。
-- 看倍率是二十倍差距；换算成人民币之后完全可能反过来。选路一直按倍率排序，
-- 也就是说**它一直在按一个不可比的数挑最便宜的门**。
--
-- # 为什么加在 channel_rates 上，而不是新开一张表
--
-- 这个量已经有了：`channel_rates.usd_per_cny`（1 人民币买到多少上游美元），
-- 定价试算那一屏在用。差别只在于它的行是「假想渠道」，没有绑到真实站点。
-- 新开一张表就是同一个物理量的第二份实现 —— 这个仓库里那种东西每次都会分叉，
-- 而且用得少的那份坏了没人发现。加一列把它绑到站点上，一张表一个含义。
ALTER TABLE channel_rates ADD COLUMN IF NOT EXISTS host TEXT NOT NULL DEFAULT '';

-- 一个站点只能有一条汇率。空 host 是「假想渠道」，可以有任意多条，所以是部分索引。
CREATE UNIQUE INDEX IF NOT EXISTS idx_channel_rates_host_unique
    ON channel_rates (lower(host)) WHERE host <> '';
