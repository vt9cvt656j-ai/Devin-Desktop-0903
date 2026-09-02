-- 把「这一笔到底谁付的钱」从推算变成记录。
--
-- 起因是对账页的收入列。它现在这么算：`endpoint_model_usage.revenue_micro_usd`
-- （售价，美元）÷ `channel_rates.usd_per_cny`（那个站的**进货折扣**）。后者根本不是
-- 汇率：线上 16 行里只有 4 行填的 0.14 碰巧等于真实汇率（¥7.14/美元），其余是
-- 1 / 1.02 / 3 / 5 / 5.3 / 7 / 10，也就是「¥1 在那家能买到多少面值」。
-- 实测 7 天，同一批流水两把尺子折出来的收入：
--     api.hao.ai              ¥7891.29  vs  ¥7846.48   （1.0×，碰巧对）
--     polly.modelbridge.cc    ¥ 380.79  vs  ¥2704.48   （7.1×）
--     zyz.qingyanzhiying.top  ¥  25.12  vs  ¥ 945.46   （37.6×）
--     合计                    ¥9777.57  vs  ¥16396.86  （低画 40%）
-- 「碰巧对」是最坏的一种对：Claude 那几条线路的数字一直是准的，于是没人怀疑过这一列。
--
-- 真正的修法不是换一把尺子，是**别再折算**。用户被扣了多少人民币，`bill_inner` 当场
-- 就知道（`charge.wallet_cents` / `charge.quota_cents`），把它记下来即可，谁的汇率都不用问。
--
-- 三列各答一个问题，合起来才说得清一次调用的钱去哪了：
--   wallet_cents  真金白银（余额）
--   quota_cents   套餐额度里划走的
--   endpoint_id   哪个出口服务的 —— 对账页按出口分组，而这张表此前只有线路 id
--
-- 加上本来就有的 free_milli_points_spent，四个来源相加应当等于 cost_cents；
-- 差额就是**订阅吸收**（配额窗口尾巴上运营方替用户吃掉的那一段，7 天约 ¥584），
-- 那笔钱此前在任何一张报表上都不存在。
--
-- endpoint_id **故意不建外键**。model_usage 在这件事上有过教训：model_id 当初带着
-- ON DELETE SET NULL 的外键，后台删掉一条线路之后，那条线路上还没结算完的每一笔
-- 都撞外键插不进去、补扣重试到上限，钱就静悄悄没了（线上抓到过 settlement 2fa0de51）。
-- 出口被删是常规操作，结算绝不能因此失败。
ALTER TABLE model_usage
  ADD COLUMN IF NOT EXISTS endpoint_id  uuid,
  ADD COLUMN IF NOT EXISTS wallet_cents bigint NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS quota_cents  bigint NOT NULL DEFAULT 0;

-- 对账按 (出口, 时间窗) 取数，这张表已经 12 万行且每次调用都在长。
CREATE INDEX IF NOT EXISTS idx_model_usage_endpoint
  ON model_usage (endpoint_id, created_at DESC)
  WHERE endpoint_id IS NOT NULL;

-- **不回填历史。** 旧行的 wallet_cents 会是 0，而那不是「没扣钱」的意思，是「当时没记」。
-- 报表必须能区分这两件事，所以判据用 endpoint_id IS NULL（＝这一行在新列上线之前写的），
-- 而不是拿 0 当缺失值。回填只能靠 cost_cents 反推，可 cost_cents 恰恰是把四个来源
-- 揉在一起的那个数 —— 用它反推等于把要修的错当成事实种进去。
