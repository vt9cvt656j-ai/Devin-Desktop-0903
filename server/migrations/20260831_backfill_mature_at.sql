-- 补上没有冻结期终点的佣金行。
--
-- mature_at 没有数据库默认值，而打款批次的两条查询都要求它非空（payout.rs）。任何一条写入
-- 佣金时忘了带上它的路径，都会留下一行「界面显示已结算、调度器永远看不见」的记录 —— 钱一分
-- 出不去，而且没有任何报错。admin_create_commission 就漏过一次，已在同一批修复里补上。
--
-- 这里按「立即到期」回填：这些行是在冻结期机制之外产生的，凭空给它们加 14 天等于把已经
-- 承诺的东西往后推。
UPDATE commissions
   SET mature_at = COALESCE(settled_at, created_at)
 WHERE mature_at IS NULL;
