-- 换上新的 Stripe 价格 id（日卡 + 三个额度包）。
--
-- 这四个价格在 Stripe 上已经带着各自的 lookup_key，而结账时用的 price 是**按 lookup_key
-- 现查**的（live_entry 优先，库里这一列只在 Stripe 查不到时兜底）。也就是说它们已经在收钱了，
-- 这条迁移是把兜底那一路也对齐 —— 否则哪天 Stripe 拉取失败，就会兜底到已经作废的旧价格。
--
-- 日卡同时拿到了人民币价 ¥7.00，展示列跟着改。
-- 三个额度包在 Stripe 上的人民币是自动换算出来的小数（如包 A 是 ¥134.9194，
-- unit_amount 为 null），取不到整数金额，所以中国用户看到和支付的都是美元 —— 与价格表
-- 里「额度包没有人民币价」一致，卡上和结账页也因此永远同币种。
UPDATE prices SET stripe_price_id='price_1U3Fcw7htKzrqxMwiVuHrTQV', amount_cents=700
 WHERE lookup_key='daily_trial';
UPDATE prices SET stripe_price_id='price_1U3G0F7htKzrqxMwtHvvNjfP' WHERE lookup_key='credit_a';
UPDATE prices SET stripe_price_id='price_1U3Fxi7htKzrqxMwOsDVlOBC' WHERE lookup_key='credit_b';
UPDATE prices SET stripe_price_id='price_1U3FuY7htKzrqxMwmnarnY00' WHERE lookup_key='credit_c';
