-- 免费模型落到付费路径之后的**亚分零头**。
--
-- 钱包和会员额度都是整分（credits_cents / quota_*_cents），而免费模型常常按次计价到
-- 亚分（实测 $0.003/次 = 3000 micro-USD）。免费池空了以后这类调用落到付费路径，
-- requested_cost 换算成整分是 0 —— 于是两边都不扣，模型变成真正的无限免费。
--
-- 四舍五入到 1 分是 3.3 倍溢价，不收是白送。所以把零头累计起来：攒够一分才真的扣一分，
-- 余下的留到下一次。单位是 micro-USD（1 分 = 10000），非负。
ALTER TABLE users ADD COLUMN IF NOT EXISTS micro_usd_carry BIGINT NOT NULL DEFAULT 0;
