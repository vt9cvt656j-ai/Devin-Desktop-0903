-- 2026Q3 价格方案。
--
-- 单位：额度 = 用户看到的美元。换算 raw = 额度 × 663（settings.rs 的面值分母；
-- plan_quotas 里 'ceshi' 那行 6630 raw 正好显示为 $10.00，除数由此坐实）。
-- 人民币金额按 7.10 折算，只用于展示；真正收多少由 Stripe 上的 currency_options 决定。
--
--   名称   key               美元    人民币    额度    折合 raw
--   日卡   daily_trial       $1      ¥7.10     5       3315（配额）
--   入门   starter_monthly   $20     ¥100      60      39780（配额）
--   主力   power_monthly     $60     ¥295      210     139230（配额）
--   尊享   elite_monthly     $200    ¥980      750     497250（配额）
--   包A    credit_a          $20     ¥142      50      33150（钱包）
--   包B    credit_b          $100    ¥710      300     198900（钱包）
--   包C    credit_c          $500    ¥3550     1600    1060800（钱包）
--   自定义 credit_custom     $1/份   ¥7.10     2.5/份  1658（钱包）
--
-- 一律 UPDATE，绝不 DELETE、也绝不把 lookup_key 挪到新行：orders.price_id 是续费找回
-- 商品的唯一线索，once_per_account 的计数和目录的"已购买"标记也都绑在行 id 上。

-- ---------- 套餐配额 ----------
-- 只 UPDATE。settings.rs 启动时整表读入，少一行会让 plan_spec 返回 None，
-- 于是发放走到「发套餐但没有配额」那条分支，而且等级标记会被任何后续购买覆盖。
UPDATE plan_quotas SET total_cents =   3315, window_cents =  3315 WHERE plan = 'trial';
UPDATE plan_quotas SET total_cents =  39780, window_cents =  3000 WHERE plan = 'basic';
UPDATE plan_quotas SET total_cents = 139230, window_cents = 15000 WHERE plan = 'power';
UPDATE plan_quotas SET total_cents = 497250, window_cents = 30000 WHERE plan = 'ultra';
-- 'pro'(65000) 和 'ceshi'(6630) 原样留着当墓碑：还有人挂在这两个等级上，
-- 删掉他们的配额就查不出来了。它们只是不再售卖。

-- ---------- 商品 ----------
UPDATE prices SET
  label='日卡', kind='plan', plan='trial', duration_days=1,
  credits_cents=NULL, unit_credits_cents=NULL,
  amount_usd_cents=100, amount_cents=710,
  stripe_price_id='price_1U3DtD7htKzrqxMwrIwPdvLf',
  recurring=false, once_per_account=true, active=true, sort=10,
  blurb='整整一天的付费额度。每个账号限一次，不会自动续费。'
WHERE lookup_key='daily_trial';

UPDATE prices SET
  label='入门', kind='plan', plan='basic', duration_days=30,
  credits_cents=NULL, unit_credits_cents=NULL,
  amount_usd_cents=2000, amount_cents=10000,
  stripe_price_id='price_1U3Dzi7htKzrqxMwgoLhIXzY',
  recurring=true, once_per_account=false, active=true, sort=20,
  blurb='日常够用：每月 $60 额度，按窗口滚动恢复。'
WHERE lookup_key='starter_monthly';

UPDATE prices SET
  label='主力', kind='plan', plan='power', duration_days=30,
  credits_cents=NULL, unit_credits_cents=NULL,
  amount_usd_cents=6000, amount_cents=29500,
  stripe_price_id='price_1U3EN87htKzrqxMwY7jETpO5',
  recurring=true, once_per_account=false, active=true, sort=30,
  blurb='长时间跑 Agent：每月 $210 额度，单次爆发窗口也更大。'
WHERE lookup_key='power_monthly';

UPDATE prices SET
  label='尊享', kind='plan', plan='ultra', duration_days=30,
  credits_cents=NULL, unit_credits_cents=NULL,
  amount_usd_cents=20000, amount_cents=98000,
  stripe_price_id='price_1U3ENz7htKzrqxMwEof6sfiL',
  recurring=true, once_per_account=false, active=true, sort=40,
  blurb='最大额度，每月 $750，不用盯着表跑长任务。'
WHERE lookup_key='elite_monthly';

UPDATE prices SET
  label='加油包 A', kind='credits', plan=NULL, duration_days=NULL, unit_credits_cents=NULL,
  credits_cents=33150, amount_usd_cents=2000, amount_cents=14200,
  stripe_price_id='price_1U3DbO7htKzrqxMwJoupHs65',
  recurring=false, once_per_account=false, active=true, sort=50,
  blurb='$50 额度，永不过期，任何套餐都能用。'
WHERE lookup_key='credit_a';

UPDATE prices SET
  label='加油包 B', kind='credits', plan=NULL, duration_days=NULL, unit_credits_cents=NULL,
  credits_cents=198900, amount_usd_cents=10000, amount_cents=71000,
  stripe_price_id='price_1U3Dak7htKzrqxMw0yOmVzML',
  recurring=false, once_per_account=false, active=true, sort=60,
  blurb='$300 额度，比 A 包更划算。'
WHERE lookup_key='credit_b';

UPDATE prices SET
  label='加油包 C', kind='credits', plan=NULL, duration_days=NULL, unit_credits_cents=NULL,
  credits_cents=1060800, amount_usd_cents=50000, amount_cents=355000,
  stripe_price_id='price_1U3DXn7htKzrqxMwKVyeUNv9',
  recurring=false, once_per_account=false, active=true, sort=70,
  blurb='$1600 额度，单价最低。'
WHERE lookup_key='credit_c';

-- 自定义充值：单价 $1，买几份就是几美元，每份 2.5 额度。
--
-- 换了 Stripe 价格 id：原来那个是「自定义金额」类型（买家在 Stripe 页面自己填，最低 $100），
-- 和这里「数量 × 每份额度」的发放模型对不上 —— 付多少和发多少完全脱钩。
UPDATE prices SET
  label='自定义充值', kind='credits', plan=NULL, duration_days=NULL,
  credits_cents=NULL, unit_credits_cents=1658,
  amount_usd_cents=100, amount_cents=710,
  stripe_price_id='price_1U3FEc7htKzrqxMwW9uFS1qL',
  recurring=false, once_per_account=false, active=true, sort=80,
  blurb='$1 = 2.5 额度，想充多少充多少。'
WHERE lookup_key='credit_custom';

-- 停售旧的两行。不删：它们身上可能挂着历史订单。
UPDATE prices SET active=false WHERE lookup_key IS NULL OR lookup_key = 'ceshi_key';
