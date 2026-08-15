-- 自动结算的含义改了：从「立刻用余额付掉」改成「免人工审核」。
--
-- 原来的 referral_auto_settle=true 做两件事：把佣金直接记成已结算，**并且**把等额的数字
-- 加进 users.credits_cents。两件事都有问题：
--
--   1. 单位是错的。credits_cents 是网关的原始计费单位，约 663 个单位 = 客户看到的 $1.00
--      （见 settings.rs 的面值分母，以及 Package A：发 3014 个单位换 $4.50）。award() 把
--      一笔 1049 美元分的佣金原样写进去，推荐人屏幕上显示的是 $1.58，而他该得 $10.49。
--      少付 6.6 倍，而且两边都没有任何提示。
--   2. 没有冻结期。钱当场就到对方手上，而退款和拒付是几十天后的事 —— 一旦发生，你赔的是
--      这笔销售、拒付手续费，外加一笔已经送出去、代码里没有任何路径能追回的佣金。
--
-- 新含义对应 kit 的 reviewStatus='AUTO_APPROVED'（规范 7.1 三层审核的 Level 1）：
--   auto_settle = true   → 佣金直接进 settled（APPROVED），不需要人点审核
--   auto_settle = false  → 佣金停在 pending，等人审
-- 两种情况都照样走冻结期 → 门槛 → Stripe Connect 转账。钱只从一个口子出去，而且是真钱。
--
-- 结果是这套系统第一次做到「全自动且安全」：没有任何一步需要人点确认，同时退款有 14 天的
-- 缓冲期，佣金按美元真实金额支付。

-- 保险起见：把这条迁移之前用旧含义结算掉的佣金标记出来。
--
-- 那些行的钱已经以余额的形式给出去了（哪怕给少了），所以它们绝不能再被新的打款批次捡走 ——
-- 那才是真的付两次。withdrawable() 和批量调度器都跳过 reversed_at 非空的行，所以打上这个
-- 标记就够了，不需要动它们的状态，账面上仍然看得出它们曾经结算过。
--
-- 执行时 commissions 表为空，所以这是个空操作；写在这里是因为它必须在表非空时也正确。
UPDATE commissions
   SET reversed_at = COALESCE(reversed_at, now()),
       reversal_reason = CASE WHEN reversal_reason = '' THEN 'legacy-credit-settlement'
                              ELSE reversal_reason END,
       note = btrim(note || ' 旧版自动结算：已按余额发放，不参与现金打款')
 WHERE settled_by = 'auto'
   AND status = 'settled'
   AND reversed_at IS NULL;
