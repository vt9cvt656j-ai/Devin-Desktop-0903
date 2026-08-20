-- 死信只该叫一次，不该每 30 秒叫一遍到世界末日。
--
-- 原来 prune() 每个 tick 都 COUNT 一遍「attempts 到顶且未了结」的行，然后打一条 ERROR。
-- 而这些行是**故意不删**的（留给人工对账），于是一笔卡住的账就会每 30 秒叫一次、跨容器
-- 重启继续叫 —— 实测线上一笔 12 分的死账已经这么叫了一整天，约 2880 条/天。
--
-- 更糟的是它只报一个数字：真出现**新的**死信时，日志上只是从 count=1 变成 count=2，
-- 淹在两千多条重复里根本看不出来。也就是说这条告警不但吵，而且**已经不起作用了**。
--
-- 加一列「叫过了没」。叫过的不再叫，于是一条 ERROR 就精确对应一笔新死信。
ALTER TABLE unsettled_charges ADD COLUMN IF NOT EXISTS alerted_at TIMESTAMPTZ;

-- 已经存在的死信补一个时间戳：上线后不要为历史遗留再叫一轮。
-- 它们仍然 resolved_at IS NULL、仍然等着人工对账，只是不再刷屏。
UPDATE unsettled_charges
   SET alerted_at = now()
 WHERE resolved_at IS NULL
   AND alerted_at IS NULL
   AND attempts >= 10;

-- 告警扫描只看这三列，给它一个部分索引。
CREATE INDEX IF NOT EXISTS idx_unsettled_dead_unalerted
    ON unsettled_charges (created_at)
 WHERE resolved_at IS NULL AND alerted_at IS NULL;
