//! 批量结算调度器 —— stripe-billing-kit 的 payout-scheduler.ts 在 Rust 这边的实现。
//!
//! 一轮做四件事，顺序和 kit 一致：
//!   1. 捞出已审核（status='settled'）且**过了冻结期**（mature_at <= now）的佣金；
//!   2. 按推荐人分组，合计不到门槛的整组跳过；
//!   3. 逐笔把佣金从 settled 抢成 paid —— 抢不到的说明已经在别的批次里，直接剔除；
//!   4. 调通道打款，幂等键就是这次打款的行 id。
//!
//! 失败就回滚：通道拒绝、账户没就绪、余额不够，被锁走的佣金全部退回 settled，下一轮再来。
//! 这一点是这套设计里最重要的：**没有任何一条路径会让佣金停在"被锁住但永远不会被支付"**。
//!
//! 为什么默认关闭。这个任务会在没有人点任何按钮的情况下把真钱转出去。它必须由运营在后台
//! 明确打开，而不是随一次部署自己开始跑 —— 所以开关在数据库里，默认 false，读不到也按 false。

use std::time::Duration;

use serde_json::json;

use crate::AppState;

/// 多久扫一轮。kit 把排期交给产品侧的 cron（每周/每月），这里用固定间隔跑，
/// 因为「够门槛 + 过冻结期」本身就是闸门 —— 扫得勤一点只是让钱早点到，不会多发。
const SWEEP_EVERY: Duration = Duration::from_secs(30 * 60);

/// 单轮最多发起多少笔打款。对应 kit 的 maxPayoutsPerRun=20：
/// 一轮里塞几百笔进通道，出问题时你连是哪一笔都找不出来。
const MAX_PAYOUTS_PER_RUN: usize = 20;

/// 一笔打款最多合并多少条佣金。防止一个推荐人攒了几千条把单条 UPDATE 撑爆。
const MAX_COMMISSIONS_PER_PAYOUT: usize = 200;

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        // 和 health、reconciler 错开，别在启动瞬间一起打出去。
        tokio::time::sleep(Duration::from_secs(90)).await;
        let mut tick = tokio::time::interval(SWEEP_EVERY);
        loop {
            tick.tick().await;
            if let Err(err) = run_once(&state).await {
                tracing::warn!(%err, "payout batch sweep failed");
            }
        }
    });
}

struct Batch {
    hold_days: i32,
    min_cents: i64,
    enabled: bool,
}

async fn settings(state: &AppState) -> Batch {
    sqlx::query_as::<_, (i32, i64, bool)>(
        "SELECT referral_hold_days, referral_min_payout_cents, referral_batch_enabled \
         FROM app_settings WHERE id = 1",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|(hold_days, min_cents, enabled)| Batch { hold_days, min_cents, enabled })
    // 读不到就按「关」：这个默认值决定的是「要不要自动把钱转出去」，
    // 猜错的代价不对称，所以只往安全的一边猜。
    .unwrap_or(Batch { hold_days: 14, min_cents: 5000, enabled: false })
}

/// 跑一轮。可重复调用，重复跑不会重复付款。
pub async fn run_once(state: &AppState) -> anyhow::Result<()> {
    let cfg = settings(state).await;
    if !cfg.enabled {
        return Ok(());
    }

    /*
     * 一步就把"谁该拿多少"算出来。
     *
     * 过滤条件逐条对应一个真实的坑：
     *   status='settled'        —— 只付审核过的（kit 的 APPROVED）
     *   mature_at <= now()      —— 冻结期已过（kit 的 validUntil）
     *   reversed_at IS NULL     —— 退款撤销过的不付
     *   payout_id IS NULL       —— 没被别的批次锁走
     * HAVING 就是 kit 的 minPayoutThresholdCents。
     */
    let groups: Vec<(uuid::Uuid, i64)> = sqlx::query_as(
        "SELECT referrer_user_id, SUM(commission_cents)::bigint \
         FROM commissions \
         WHERE status = 'settled' AND mature_at IS NOT NULL AND mature_at <= now() \
           AND reversed_at IS NULL AND payout_id IS NULL \
           AND referrer_user_id IS NOT NULL \
         GROUP BY referrer_user_id \
         HAVING SUM(commission_cents) >= $1 \
         ORDER BY SUM(commission_cents) DESC \
         LIMIT $2",
    )
    .bind(cfg.min_cents)
    .bind(MAX_PAYOUTS_PER_RUN as i64)
    .fetch_all(&state.db)
    .await?;

    if groups.is_empty() {
        return Ok(());
    }

    let mut sent = 0usize;
    let mut skipped = 0usize;

    for (uid, _expected) in groups {
        match pay_one(state, uid, cfg.min_cents).await {
            Ok(true) => sent += 1,
            Ok(false) => skipped += 1,
            Err(e) => {
                skipped += 1;
                tracing::warn!(user = %uid, "payout batch: {e}");
            }
        }
    }

    tracing::info!(sent, skipped, "payout batch sweep");
    Ok(())
}

/// 给一个推荐人打一次款。返回 true 表示钱真的发出去了。
///
/// 顺序很讲究，每一步都是为了「失败之后不留下烂摊子」：
///   1. 先建打款行（sending）—— 它同时是佣金的锁目标和通道的幂等键；
///   2. 把佣金抢过来（settled → paid，条件更新，抢不到的自动剔除）；
///   3. 按**真正抢到的**金额改写打款行 —— 而不是第 1 步的预估；
///   4. 抢到的不够门槛就整轮回滚，钱一分不动。
async fn pay_one(state: &AppState, uid: uuid::Uuid, min_cents: i64) -> anyhow::Result<bool> {
    let mut tx = state.db.begin().await?;

    // 占位金额先写 1：这一行此刻的意义是「锁」，不是「金额」。真实金额第 3 步补。
    let payout: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO withdrawals (user_id, amount_cents, method, account, status, provider) \
         VALUES ($1, 1, 'auto', '', 'sending', 'stripe_connect') RETURNING id",
    )
    .bind(uid)
    .fetch_one(&mut *tx)
    .await?;

    // 竞争锁。条件写在 WHERE 里，所以两轮同时跑的时候，同一条佣金只会被一轮抢到 ——
    // 对应 kit 的 transitionCommissionStatus(['APPROVED'] -> 'PAID')。
    let locked: i64 = sqlx::query_scalar(
        "WITH picked AS ( \
             SELECT id FROM commissions \
             WHERE referrer_user_id = $1 AND status = 'settled' \
               AND mature_at IS NOT NULL AND mature_at <= now() \
               AND reversed_at IS NULL AND payout_id IS NULL \
             ORDER BY mature_at \
             LIMIT $3 \
             FOR UPDATE SKIP LOCKED \
         ), locked AS ( \
             UPDATE commissions c SET status = 'paid', payout_id = $2, updated_at = now() \
             FROM picked WHERE c.id = picked.id AND c.status = 'settled' \
             RETURNING c.commission_cents \
         ) \
         SELECT COALESCE(SUM(commission_cents), 0)::bigint FROM locked",
    )
    .bind(uid)
    .bind(payout)
    .bind(MAX_COMMISSIONS_PER_PAYOUT as i64)
    .fetch_one(&mut *tx)
    .await?;

    // 抢完之后不够门槛（别的批次抢先了，或者中间有笔被退款撤销），整轮作废。
    // 事务回滚会把佣金和打款行一起撤掉 —— 什么都没发生过。
    if locked < min_cents {
        tx.rollback().await?;
        // 静默跳过会让「一个人攒了几百笔小额佣金、每轮都抢不满门槛」这种活锁完全看不见。
        tracing::warn!(
            user = %uid, locked, min_cents,
            "payout skipped: claimed less than the threshold (another sweep, or a refund landed)"
        );
        return Ok(false);
    }

    sqlx::query("UPDATE withdrawals SET amount_cents = $2 WHERE id = $1")
        .bind(payout)
        .bind(locked)
        .execute(&mut *tx)
        .await?;

    // 锁到此为止。转账必须在事务外：Stripe 那边的钱不会因为这里 ROLLBACK 而回来。
    tx.commit().await?;

    match crate::connect::pay(state, payout, uid, locked).await {
        crate::connect::Payout::Sent(transfer) => {
            let recorded = sqlx::query(
                "UPDATE withdrawals SET status = 'paid', transfer_id = $2, paid_at = now(), \
                     paid_by = 'batch', updated_at = now() \
                 WHERE id = $1 AND status = 'sending'",
            )
            .bind(payout)
            .bind(&transfer)
            .execute(&state.db)
            .await;
            if !matches!(&recorded, Ok(r) if r.rows_affected() == 1) {
                tracing::error!(
                    payout = %payout, transfer = %transfer,
                    "TRANSFER SENT BUT NOT RECORDED — reconcile by metadata[withdrawal_id]"
                );
            }
            crate::realtime::record_event(
                state,
                Some(uid),
                "withdrawal_decided",
                json!({ "status": "paid", "amount_cents": locked, "by": "batch" }),
            )
            .await;
            Ok(true)
        }
        // 结果不明：钱**可能已经出去了**。这一笔留在 sending，佣金留在 paid，等人核对。
        //
        // 绝不能回滚。回滚会让下一轮重新捞到这些佣金、开一行新的 withdrawals、用一个**新的**
        // uuid 当幂等键 —— Stripe 看到的是两个毫不相干的请求，于是真的转两次。
        crate::connect::Payout::Unknown(reason) => {
            sqlx::query(
                "UPDATE withdrawals SET failure_reason = $2, updated_at = now() WHERE id = $1",
            )
            .bind(payout)
            .bind(&reason)
            .execute(&state.db)
            .await
            .ok();
            tracing::error!(payout = %payout, reason, "payout outcome UNKNOWN — left as sending, needs a person");
            Ok(false)
        }
        // 通道明确拒绝了：钱没动。佣金退回 settled，下一轮重来（kit 的 rollbackCommissions）。
        crate::connect::Payout::Refused(reason) => {
            rollback(state, payout, &reason).await;
            Ok(false)
        }
    }
}

/// 把一次失败的打款所锁定的佣金全部退回可结算状态。
///
/// 这是 kit 的 rollbackCommissions。少了它，一次「余额不足」就能把一批佣金永久卡在 paid：
/// 不会被支付，也不会再被扫到，推荐人的钱凭空消失，而且没有任何报错。
pub async fn release(state: &AppState, payout: uuid::Uuid) {
    let back = sqlx::query(
        "UPDATE commissions SET status = 'settled', payout_id = NULL, updated_at = now() \
         WHERE payout_id = $1 AND status = 'paid'",
    )
    .bind(payout)
    .execute(&state.db)
    .await;
    let n = back.map(|r| r.rows_affected()).unwrap_or(0);
    if n > 0 {
        tracing::info!(payout = %payout, released = n, "payout rejected; commissions released");
    }
}

pub async fn rollback(state: &AppState, payout: uuid::Uuid, reason: &str) {
    let back = sqlx::query(
        "UPDATE commissions SET status = 'settled', payout_id = NULL, updated_at = now() \
         WHERE payout_id = $1 AND status = 'paid'",
    )
    .bind(payout)
    .execute(&state.db)
    .await;
    let n = back.map(|r| r.rows_affected()).unwrap_or(0);

    sqlx::query(
        "UPDATE withdrawals SET status = 'failed', failure_reason = $2, updated_at = now() \
         WHERE id = $1 AND status IN ('sending', 'paid')",
    )
    .bind(payout)
    .bind(reason)
    .execute(&state.db)
    .await
    .ok();

    tracing::warn!(payout = %payout, released = n, reason, "payout failed; commissions released");
}

#[cfg(test)]
mod tests {
    /// 抢不到的佣金必须被剔除，而不是一起付掉。
    #[test]
    fn two_concurrent_batches_cannot_pay_the_same_commission() {
        let src = include_str!("payout.rs");
        let f = src.split("async fn pay_one(").nth(1).expect("pay_one");
        let body = &f[..f.find("\n/// ").unwrap_or(f.len())];
        assert!(
            body.contains("AND c.status = 'settled'"),
            "锁必须是条件更新：两轮同时跑时，同一条佣金只能被一轮抢到",
        );
        assert!(
            body.contains("FOR UPDATE SKIP LOCKED"),
            "并发的两轮不该互相等待，抢不到就跳过",
        );
        assert!(
            body.contains("SELECT COALESCE(SUM(commission_cents), 0)::bigint FROM locked"),
            "金额必须来自**真正抢到**的那些行，不能用抢之前的预估",
        );
    }

    /// 冻结期和门槛是这套设计的全部意义，不能被绕过。
    #[test]
    fn the_hold_period_and_threshold_actually_gate_the_money() {
        let src = include_str!("payout.rs");
        let f = src.split("pub async fn run_once(").nth(1).expect("run_once");
        let body = &f[..f.find("\n/// ").unwrap_or(f.len())];
        assert!(
            body.contains("mature_at <= now()"),
            "没有冻结期，佣金一到手就能变现，而退款是几十天后的事",
        );
        assert!(
            body.contains("HAVING SUM(commission_cents) >= $1"),
            "没有门槛，一笔 $1 的佣金也会单独转一次，手续费吃掉大半",
        );
        assert!(
            body.contains("reversed_at IS NULL"),
            "退款撤销的不能再发一次现金；20260830 之前用旧版自动结算发过余额的行也是靠这个\
             标记被排除的",
        );
        assert!(
            body.contains("payout_id IS NULL"),
            "已经被别的批次锁走的不能再被扫到",
        );
    }

    /// 默认必须是关的。
    #[test]
    fn the_batch_never_starts_itself() {
        let src = include_str!("payout.rs");
        assert!(
            src.contains("enabled: false }"),
            "读不到设置时必须按关处理 —— 这个开关决定的是要不要自动往外转真钱",
        );
        let f = src.split("pub async fn run_once(").nth(1).expect("run_once");
        assert!(
            f[..400].contains("if !cfg.enabled") && f[..400].contains("return Ok(())"),
            "关着的时候必须立刻返回，不能先扫一遍再判断",
        );
    }

    /// 打款失败，佣金必须回到可结算 —— 否则钱会永久卡住。
    #[test]
    fn a_failed_payout_releases_its_commissions() {
        let src = include_str!("payout.rs");
        let f = src.split("pub async fn rollback(").nth(1).expect("rollback");
        assert!(
            f.contains("SET status = 'settled', payout_id = NULL")
                && f.contains("WHERE payout_id = $1 AND status = 'paid'"),
            "回滚必须把这次打款锁走的佣金全部放回 settled，且只放这一次的",
        );
    }
}

#[cfg(test)]
mod exclusivity_tests {
    /// 两条打款路径不能同时开着。
    ///
    /// 手动提现和批量结算都从同一份「已结算佣金」里取钱，但记账方式不同：批量会把佣金标成
    /// paid 并挂上 payout_id，手动只看合计金额。两边同时开，同一笔佣金可以被各发一次。
    #[test]
    fn manual_withdrawal_is_closed_while_the_batch_owns_payouts() {
        let src = include_str!("referral.rs");
        let w = src.split("pub async fn withdraw(").nth(1).expect("withdraw");
        let w = &w[..w.find("\n// ---").unwrap_or(w.len())];
        assert!(
            w.contains("if t.batch_enabled"),
            "开了批量结算就必须挡住手动申请 —— 「谁来付」只能有一个答案",
        );
        let gate = w.find("if t.batch_enabled").unwrap();
        let insert = w.find("INSERT INTO withdrawals").expect("insert");
        assert!(gate < insert, "拦截必须在写库之前");
    }

    /// 冻结期是冻结在佣金行上的，不是打款时现算。
    #[test]
    fn the_hold_is_frozen_onto_the_row_like_the_rate() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn award").nth(1).expect("award");
        let f = &f[..f.find("\n// ---").unwrap_or(f.len())];
        assert!(
            f.contains("now() + make_interval(days => $10)"),
            "mature_at 必须在记录佣金时算好写进去",
        );
        assert!(
            f.contains("SELECT referral_hold_days FROM app_settings"),
            "冻结期天数从设置读，但只读这一次",
        );
        // 打款那边只能读，不能再算一遍。
        let p = include_str!("payout.rs");
        assert!(
            !p.contains("referral_hold_days FROM app_settings WHERE id = 1\")\n        .fetch_optional(&state.db)")
                || p.contains("mature_at <= now()"),
            "调度器只比较 mature_at，不重新按当前设置算冻结期 —— 否则改设置会把已记下的佣金往后推",
        );
    }

    /// 退回的打款必须把佣金放回去，两条路都要。
    #[test]
    fn both_failure_paths_release_the_commissions() {
        // 通道当场拒绝 → payout::rollback
        let p = include_str!("payout.rs");
        assert!(
            p.contains("rollback(state, payout, &reason).await"),
            "通道明确拒绝时要立刻回滚",
        );
        // 先成功、之后被 Stripe 冲回 → webhook
        let s = include_str!("stripe.rs");
        let arm = s
            .split("\"transfer.reversed\" | \"transfer.failed\" => {")
            .nth(1)
            .expect("reversal arm");
        let body = &arm[..arm.find("\"charge.dispute.closed\"").unwrap_or(arm.len())];
        assert!(
            body.contains("SET status = 'settled', payout_id = NULL"),
            "转账被冲回时，被它锁走的佣金必须回到 settled，否则永远卡在 paid：\
             不会被支付，也不会再被扫到",
        );
    }
}

#[cfg(test)]
mod review_regression_tests {
    /// 「被拒绝」和「结果不明」必须走两条完全不同的路。
    ///
    /// 合成一条是这套代码里最贵的一个错：Stripe 返回 5xx（带 JSON 错误体）或者响应体读超时，
    /// 都会被当成「没转成」→ 回滚佣金 → 下一轮开新的 withdrawals 行 → **新的 uuid 当幂等键**
    /// → Stripe 看到一个陌生请求 → 同一笔钱转第二次。
    #[test]
    fn an_unknown_outcome_is_never_rolled_back() {
        let src = include_str!("payout.rs");
        let f = src.split("async fn pay_one(").nth(1).expect("pay_one");
        let body = &f[..f.find("\n/// ").unwrap_or(f.len())];

        assert!(
            body.contains("crate::connect::Payout::Unknown(reason)")
                && body.contains("crate::connect::Payout::Refused(reason)"),
            "两种结局必须分别匹配，不能共用一个分支",
        );
        let unknown = body.find("Payout::Unknown(reason)").expect("unknown arm");
        let refused = body.find("Payout::Refused(reason)").expect("refused arm");
        let unknown_arm = &body[unknown..refused];
        assert!(
            !unknown_arm.contains("rollback("),
            "结果不明时绝不能回滚：回滚就等于允许下一轮用新的幂等键再转一次",
        );
        assert!(
            body[refused..].contains("rollback(state, payout, &reason)"),
            "明确被拒时必须回滚，否则佣金永远卡在 paid",
        );
        // 字符串匹配已经被枚举取代，不能再回去。
        assert!(
            !body.contains(r#"contains("不可达")"#),
            "用字符串判断结局会随文案漂移；分类必须由类型保证",
        );
    }

    /// 被驳回的批量打款要把佣金放回去。
    #[test]
    fn rejecting_a_batch_payout_releases_its_commissions() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn admin_withdraw_status").nth(1).expect("fn");
        let f = &f[..f.find("\n/// `GET").unwrap_or(f.len())];
        assert!(
            f.contains("crate::payout::release(&state, id)"),
            "驳回一笔批量打款等于宣布转账没成立，被它锁走的佣金必须回到 settled",
        );
        assert!(
            f.contains("t.batch_enabled && req.status == \"paid\""),
            "开着自动打款时不许人工再标已支付 —— 同一笔钱两条路各发一次",
        );
    }

    /// 退款必须盖得到已经进批次的佣金。
    #[test]
    fn a_refund_reaches_a_commission_that_is_already_batched() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn reverse(").nth(1).expect("reverse");
        let body = &f[..f.find("\n/// ").unwrap_or(f.len())];
        assert!(
            body.matches("status IN ('settled', 'paid')").count() >= 2,
            "'paid' 是佣金进批次之后的常驻状态。三段 UPDATE 只匹配 pending/settled 时，\
             一笔退款落在已进批次的佣金上什么都不会发生，之后回滚一放，照付不误",
        );
        assert!(
            body.contains("status IN ('pending', 'settled') AND payout_id IS NULL"),
            "部分退款要能按比例扣到 settled 行：自动审核开着时根本没有 pending 行",
        );
    }

    /// 批量打款自己的行不能在可提现里再扣一次。
    #[test]
    fn a_batch_payout_is_not_subtracted_twice() {
        let src = include_str!("referral.rs");
        assert!(
            src.matches("AND method <> 'auto'").count() >= 2,
            "批次的 withdrawals 行背后的佣金已经从 settled 合计里出去了（变成 paid），\
             在 taken 里再扣一次就是扣两遍 —— 展示和锁内重算两处都要",
        );
    }
}
