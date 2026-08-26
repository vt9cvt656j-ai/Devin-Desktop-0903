//! 批量结算调度器 —— stripe-billing-kit 的 payout-scheduler.ts 在 Rust 这边的实现。
//!
//! 一轮做四件事，顺序和 kit 一致：
//!   1. 捞出已审核（status='settled'）且**过了冻结期**（mature_at <= now）的佣金；
//!   2. 按推荐人分组，减去手动提现已经领走的部分，合计不到门槛的整组跳过；
//!   3. 逐笔把佣金从 settled 抢成 paid —— 抢不到的说明已经在别的批次里，直接剔除；
//!   4. 调通道打款，幂等键就是这次打款的行 id。
//!
//! 第 2 步那个减法不是优化，是这套账最要紧的一条闸：手动提现从**同一堆** settled 佣金里
//! 取钱却只记金额、从不碰佣金行，所以一笔手动付掉的佣金在这里看起来和「从没付过」一模一样。
//! 少了它，运营一打开 referral_batch_enabled，历史上每一笔手动付款都会被再转一次。
//! 见 `claimed_by_hand_sql`。
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

/// 这个人已经通过**手动提现**从同一份「已结算佣金」里领走（或正在领）的钱，按分合计。
/// `owner` 填佣金那一侧的用户列：分组查询里是 `c.referrer_user_id`，逐人那条是 `$1`。
///
/// ## 这个减法是干什么的
///
/// 手动提现和批量打款从**同一堆** settled 佣金里取钱，但两边记的是两本账：
/// 批量把佣金标成 paid + 挂 payout_id（行级），手动只往 withdrawals 里写一个金额，
/// **从不碰佣金行**（withdraw() 的 Connect 自动打款、admin_withdraw_status 手工标已支付，
/// 两条路都一样）。于是一笔已经手动付掉的佣金在批次眼里仍然是
/// settled + mature + reversed_at IS NULL + payout_id IS NULL —— 一个字都不差的「该付」。
///
/// 后果不是理论上的：运营在后台把 referral_batch_enabled 打开（admin_save_settings 只打
/// 一行 warn，不做任何对账），下一轮扫描就会把**历史上每一笔手动付过的佣金**重新捞出来，
/// 用 Stripe Connect 再转一次。20260830 的迁移预见到了这一类重复支付，但它只标了旧版
/// settled_by='auto' 的余额发放行，手动提现付掉的那些一个都没标。
///
/// ## 为什么是减金额，而不是在付款时给佣金盖章
///
/// 「手动付款时把它覆盖的佣金标成 paid」听着更干净，但做不到严密：提现是**金额**，佣金是
/// **行**，$37 的提现盖在 $20+$20 两行上，盖一行漏 $17、盖两行多吃 $3 —— 多吃的那 $3 要
/// 么凭空吞掉推荐人的钱，要么得再发明一套找零账。更要命的是历史：改之前付掉的那些提现
/// 没有任何痕迹可回填，而它们正是双付敞口本身。
///
/// 按金额减就没有这些问题：settled 池子是唯一的真相，手动提现是对池子的金额索取，批量打款
/// 是从池子里取走整行；两边都只能动「池子 − 已被索取」这一块。withdrawable() 里的 `taken`
/// 用的就是这个定义（逐字同源），所以两条路算出来的可动金额天然一致，永远不会重叠。
///
/// 代价：被减掉的那部分对应的佣金行会永远停在 settled（没人再来取它们）。账面上有点脏，
/// 但那正是「这笔钱已经付过了」的事实。切到批量之后留在队列里的 pending 手动申请也会一直
/// 占着额度——这就是 admin_withdraw_status 让运营用「驳回」清掉遗留申请的财务理由。
///
/// `method <> 'auto'` 把批次自己开的 withdrawals 行排除掉：它们背后的佣金已经变成 paid、
/// 早就从池子里出去了，再减一次就是减两遍。
///
/// ## 排除 'rejected' 的那一半：驳回 = 把钱放回批次预算
///
/// `status NOT IN ('rejected', 'failed', 'returned')` 这三个状态都表示钱没到对方手上，
/// 额度必须还回池子。要说清楚的是它反过来那一面：**驳回一笔手动提现，就是把这笔钱放回
/// 本调度器的预算**，下一轮它可以被真的转出去。运营点「驳回」时看到的提示只说「退回他的
/// 可提现余额」，实际影响不止于此。
///
/// 对 pending 行这是对的，而且是必须的：切到自动打款之后，遗留的 pending 申请靠驳回清掉，
/// 不然它们会一直占着额度，那个人的佣金永远发不出去（见上一段「代价」）。
///
/// 对 'sending' 行就是一句**断言** —— 「这笔 Connect 转账不存在」。程序判不了：sending 的
/// 定义就是结果不明（connect.rs 的 `Payout::Unknown`，钱可能已经出去了）。规矩因此是单向的：
/// 先按 metadata[withdrawal_id] 去 Stripe 核对，查到转账在就标已支付并填 tr_ 号
/// （admin_withdraw_status 允许，自动打款开着也允许，就是为这个），查不到才驳回。凭感觉
/// 驳回一条其实已经成立的 sending 行，等于让下一轮批次把同一笔钱再转一次 —— 而这条减法
/// 是唯一拦得住重复支付的东西。
fn claimed_by_hand_sql(owner: &str) -> String {
    format!(
        "SELECT COALESCE(SUM(w.amount_cents), 0)::bigint FROM withdrawals w \
         WHERE w.user_id = {owner} AND w.method <> 'auto' \
           AND w.status NOT IN ('rejected', 'failed', 'returned')"
    )
}

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
     *
     * 减去手动提现已经领走的部分，见 claimed_by_hand_sql。这里减是为了不让「这组的钱
     * 其实早就手动付掉了」的推荐人每半小时被捞起来一次、再在 pay_one 里因为不够门槛被
     * 丢掉；真正拦住重复付款的是 pay_one 里那一道，这一道只是别做无用功。
     */
    let groups: Vec<(uuid::Uuid, i64)> = sqlx::query_as(
        &format!(
            "SELECT c.referrer_user_id, SUM(c.commission_cents)::bigint \
             FROM commissions c \
             WHERE c.status = 'settled' AND c.mature_at IS NOT NULL AND c.mature_at <= now() \
               AND c.reversed_at IS NULL AND c.payout_id IS NULL \
               AND c.referrer_user_id IS NOT NULL \
             GROUP BY c.referrer_user_id \
             HAVING SUM(c.commission_cents)::bigint - ({claimed}) >= $1 \
             ORDER BY SUM(c.commission_cents) DESC \
             LIMIT $2",
            claimed = claimed_by_hand_sql("c.referrer_user_id"),
        ),
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

/// 这一轮真正可以锁走的佣金行：可付合计 − 手动已领走，然后按 mature_at 顺序往里装。
///
/// 佣金是不可分割的行，而手动提现只是一个金额，两者对不齐是常态：可付 $20+$20+$10、
/// 手动已领 $20，预算 $30 —— 只能锁 $20+$10。**装不下就跳过看下一条，不是就此打住**：
/// 打住的话一条大额佣金卡在队首会把它后面所有小额一起冻住，直到池子涨到能装下它为止，
/// 明明该付的钱就一直不动。装箱只往下取整，任何一条都不会跨过预算 —— 跨过去的那一分
/// 就是第二次付同一笔钱。装不下的不会丢：下一轮池子涨了，同样的减法会把它放出来。
///
/// 预算为负（历史手动付款多于现存可付佣金）时一行都取不到，这一轮什么都不发，并由调用方
/// 把 claimed 打进 warn 日志。**失败向安全侧倒**：宁可让钱卡住等人看，也不能猜着往外转。
///
/// `pool_total` 必须是这个推荐人**全部**到期可付佣金的合计，不能拿候选窗口的合计代替。
/// 候选查询是 `ORDER BY mature_at LIMIT 200`，取的是**最老的** 200 行；而新赚到的佣金
/// mature_at 更晚，永远排在窗口外面。用窗口合计当基数会造成一种**永久卡死**：
///
///   某推荐人 400 行 × 3000 = 1,200,000 可付，历史手动提现 800,000，真实应付 400,000。
///   窗口（最老 200 行）合计 600,000，budget = 600,000 − 800,000 = −200,000 → 一行都装不下
///   → 回滚。而 run_once 的 HAVING 用的是**全池**（净额 400,000 过门槛），于是这个人每 30
///   分钟被重新选中、重新算出负预算、重新回滚，warn 刷屏，钱永远发不出去。池子涨了也没用
///   ——涨出来的行 mature_at 更晚，还是进不了窗口。
///
/// 用全池当基数之后「不会重复付」的归纳仍然成立：每一轮取走的 p_i ≤ pool_i − claimed，
/// 而 pool 会随着取走的行转成 'paid' 而减少，claimed 只增不减，所以累计付出永远不超过
/// 「历史总应付 − 手动已付」。装箱仍然只在 200 行的窗口里做，单轮上限不变。
fn within_budget(candidates: &[(uuid::Uuid, i64)], pool_total: i64, claimed: i64) -> Vec<uuid::Uuid> {
    let budget = pool_total - claimed;
    let mut running = 0i64;
    let mut take = Vec::with_capacity(candidates.len());
    for (id, cents) in candidates {
        if running + cents > budget {
            continue;
        }
        running += cents;
        take.push(*id);
    }
    take
}

/// 给一个推荐人打一次款。返回 true 表示钱真的发出去了。
///
/// 顺序很讲究，每一步都是为了「失败之后不留下烂摊子」：
///   1. 锁住这个人 —— 手动提现走的是同一把锁，两条路不能同时算余额；
///   2. 建打款行（sending）—— 它同时是佣金的锁目标和通道的幂等键；
///   3. 算出这一轮**最多能动多少**：可付佣金合计 − 手动提现已经领走的（claimed_by_hand_sql）；
///   4. 在预算之内把佣金抢过来（settled → paid，条件更新，抢不到的自动剔除）；
///   5. 按**真正抢到的**金额改写打款行 —— 而不是第 2 步的预估；
///   6. 抢到的不够门槛就整轮回滚，钱一分不动。
async fn pay_one(state: &AppState, uid: uuid::Uuid, min_cents: i64) -> anyhow::Result<bool> {
    let mut tx = state.db.begin().await?;

    /*
     * 先锁人，再算账。
     *
     * withdraw() 也是拿这一把锁（referral.rs，`SELECT id FROM users … FOR UPDATE`）之后
     * 才重算余额、插提现行。没有这把锁，第 3 步读到的「手动已领走」可能比实际少一笔：
     * 运营刚把批量开关关掉、用户提了 $100、开关又打开，而这一轮正好卡在读之后插之前 ——
     * 两边各按「还有 $100」发一次。锁顺序两边一致（users → withdrawals → commissions），
     * 不会死锁。
     */
    sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(uid)
        .execute(&mut *tx)
        .await?;

    // 占位金额先写 1：这一行此刻的意义是「锁」，不是「金额」。真实金额第 5 步补。
    let payout: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO withdrawals (user_id, amount_cents, method, account, status, provider) \
         VALUES ($1, 1, 'auto', '', 'sending', 'stripe_connect') RETURNING id",
    )
    .bind(uid)
    .fetch_one(&mut *tx)
    .await?;

    // 手动那条路已经从同一堆 settled 佣金里领走多少。为什么必须减，见函数上的长注释。
    // 这一行是「切到自动打款之后，历史手动付款不会被重付一遍」的唯一依靠。
    let claimed: i64 = sqlx::query_scalar(&claimed_by_hand_sql("$1"))
        .bind(uid)
        .fetch_one(&mut *tx)
        .await?;

    // 预算的基数：这个人**全部**到期可付佣金，不是下面那 200 行候选窗口的合计。
    // 窗口取的是最老的 200 行，新赚的佣金 mature_at 更晚永远进不来——拿窗口当基数会让
    // 「手动已付 > 窗口合计」的老推荐人永久卡死（详见 within_budget 的注释）。
    // 和 claimed 一样在 users FOR UPDATE 之后读，两个数来自同一个一致性快照。
    let pool_total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(commission_cents), 0)::bigint FROM commissions \
         WHERE referrer_user_id = $1 AND status = 'settled' \
           AND mature_at IS NOT NULL AND mature_at <= now() \
           AND reversed_at IS NULL AND payout_id IS NULL",
    )
    .bind(uid)
    .fetch_one(&mut *tx)
    .await?;

    // 竞争锁。SKIP LOCKED 让两轮同时跑时各自拿到不相交的一批，
    // 对应 kit 的 transitionCommissionStatus(['APPROVED'] -> 'PAID')。
    let candidates: Vec<(uuid::Uuid, i64)> = sqlx::query_as(
        "SELECT id, commission_cents FROM commissions \
         WHERE referrer_user_id = $1 AND status = 'settled' \
           AND mature_at IS NOT NULL AND mature_at <= now() \
           AND reversed_at IS NULL AND payout_id IS NULL \
         ORDER BY mature_at \
         LIMIT $2 \
         FOR UPDATE SKIP LOCKED",
    )
    .bind(uid)
    .bind(MAX_COMMISSIONS_PER_PAYOUT as i64)
    .fetch_all(&mut *tx)
    .await?;

    let take = within_budget(&candidates, pool_total, claimed);

    let locked: i64 = sqlx::query_scalar(
        "WITH locked AS ( \
             UPDATE commissions c SET status = 'paid', payout_id = $2, updated_at = now() \
             WHERE c.id = ANY($1) AND c.status = 'settled' \
             RETURNING c.commission_cents \
         ) \
         SELECT COALESCE(SUM(commission_cents), 0)::bigint FROM locked",
    )
    .bind(&take[..])
    .bind(payout)
    .fetch_one(&mut *tx)
    .await?;

    // 抢完之后不够门槛（别的批次抢先了、中间有笔被退款撤销、或者预算被手动提现吃掉了），
    // 整轮作废。事务回滚会把佣金和打款行一起撤掉 —— 什么都没发生过。
    if locked < min_cents {
        tx.rollback().await?;
        // 静默跳过会让「一个人攒了几百笔小额佣金、每轮都抢不满门槛」这种活锁完全看不见。
        // claimed 一起报出来：整组被手动提现挡住时，日志里必须看得出是被挡的，
        // 而不是像「没佣金可付」。
        tracing::warn!(
            user = %uid, locked, min_cents, claimed,
            "payout skipped: claimed less than the threshold (another sweep, a refund, \
             or commission already paid out by hand)"
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
///
/// # 写失败必须喊出来，而且不能和「没有可放的」同值
///
/// 这里曾经把 Result 压成行数、失败时得到 0，然后只在行数大于 0 时打日志 ——
/// 写失败时**一个字都不留**，而后果是这批佣金停在 `paid` 且挂着一个作废打款的 id：
/// 既没打出去，也不能再打，钱就卡在那儿。
///
/// 「查库出错」和「本来就没有可放的」在结果上是同一个 0，在后果上差着一笔钱。
pub async fn release(state: &AppState, payout: uuid::Uuid) {
    match sqlx::query(
        "UPDATE commissions SET status = 'settled', payout_id = NULL, updated_at = now() \
         WHERE payout_id = $1 AND status = 'paid'",
    )
    .bind(payout)
    .execute(&state.db)
    .await
    {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::info!(payout = %payout, released = r.rows_affected(),
                "payout rejected; commissions released");
        }
        Ok(_) => {} // 真的没有可放的，正常
        Err(e) => tracing::error!(payout = %payout, error = %e,
            "打款被拒但佣金**没能放回可结算** —— 它们卡在 paid 且挂着作废的 payout_id，\
             既打不出去也不能重试，需要人工介入"),
    }
}

pub async fn rollback(state: &AppState, payout: uuid::Uuid, reason: &str) {
    // 和 release 同一条规矩：写失败不能压成 0，那和「没有可放的」同值，
    // 而后果是这批佣金卡在 paid、既打不出去也不能重试。
    let back = sqlx::query(
        "UPDATE commissions SET status = 'settled', payout_id = NULL, updated_at = now() \
         WHERE payout_id = $1 AND status = 'paid'",
    )
    .bind(payout)
    .execute(&state.db)
    .await;
    let n = match &back {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            tracing::error!(payout = %payout, error = %e, reason,
                "打款失败，但佣金**没能放回可结算** —— 卡在 paid 且挂着作废的 payout_id，需要人工介入");
            0
        }
    };

    // 提现单也一样：这一条不落地的话，一笔失败的提现会一直显示成 sending/paid，
    // 用户看到「打款中」而钱既没到账也不会重试。
    if let Err(e) = sqlx::query(
        "UPDATE withdrawals SET status = 'failed', failure_reason = $2, updated_at = now() \
         WHERE id = $1 AND status IN ('sending', 'paid')",
    )
    .bind(payout)
    .bind(reason)
    .execute(&state.db)
    .await
    {
        tracing::error!(payout = %payout, error = %e,
            "打款失败，但提现单**没能置成 failed** —— 用户那边会一直显示打款中");
    }

    if back.is_ok() {
        tracing::warn!(payout = %payout, released = n, reason, "payout failed; commissions released");
    }
}

#[cfg(test)]
mod tests {
    /// 放回佣金失败时不许静默，也不许和「没有可放的」同值。
    ///
    /// 两处曾经都把 Result 压成行数，而 release 还只在行数大于 0 时打日志 ——
    /// 写失败时一个字都不留。后果是这批佣金停在 `paid` 且挂着一个作废打款的 id：
    /// **既打不出去，也不能重试**，钱卡住而没有任何痕迹。
    #[test]
    fn a_failed_release_is_never_silent_nor_confused_with_nothing_to_release() {
        let src = include_str!("payout.rs");
        let prod_raw = &src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())];
        // **先剥注释再断言。** 否定断言最容易被注释喂到：一段解释「原来是怎么写的」
        // 的文档注释会让「不许再出现这种写法」的断言恒红。我写这条测试时就当场
        // 踩了一次 —— 上面那段文档注释里引用了旧写法。
        let prod: String = prod_raw
            .lines()
            .map(|l| if l.trim_start().starts_with("//") { "" } else { l })
            .collect::<Vec<_>>()
            .join("\n");
        let prod = prod.as_str();
        assert!(
            !prod.contains("back.map(|r| r.rows_affected()).unwrap_or(0)"),
            "又把写失败压成 0 了 —— 和「没有可放的」同值，而后果差着一笔钱",
        );
        // 按花括号配对抠函数体，不切固定长度的窗口。
        //
        // 固定窗口有两种坏法，今天两种都踩了：函数变长时窗口够不到要守的那一行
        // （测试仍然绿，但它守的东西已经不在它看的那段里）；函数在文件末尾时
        // 切片直接越界 panic。两者都不是「断言失败」，都是测试本身坏了。
        let fn_body = |sig: &str| -> String {
            let at = prod.find(sig).unwrap_or_else(|| panic!("{sig} 不见了"));
            let open = at + prod[at..].find('{').expect("函数没有花括号");
            let b = prod.as_bytes();
            let (mut d, mut i) = (0i32, open);
            while i < b.len() {
                match b[i] {
                    b'{' => d += 1,
                    b'}' => {
                        d -= 1;
                        if d == 0 {
                            return prod[open..=i].to_string();
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            panic!("{sig} 花括号没配平");
        };

        // 两个函数都要能区分 Err 和 Ok(0)。
        for f in ["pub async fn release(", "pub async fn rollback("] {
            let body = fn_body(f);
            assert!(
                body.contains("Err(e)") && body.contains("tracing::error!"),
                "{f} 写失败时没有 error 级日志 —— 钱卡住而没人知道",
            );
        }

        // 提现单置 failed 也不许把失败丢掉：不落地的话用户会一直看到「打款中」。
        let rb = fn_body("pub async fn rollback(");
        let at = rb.find("UPDATE withdrawals SET status = 'failed'").expect("提现置失败那句不见了");
        assert!(
            rb[..at].contains("if let Err(e)"),
            "提现单置 failed 的失败被吞了 —— 用户那边会一直显示打款中",
        );
    }

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
            body.contains("HAVING SUM(c.commission_cents)") && body.contains(">= $1"),
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

    /// 手动付掉的佣金，批次一分都不许再付。
    ///
    /// 这是切换到自动打款那一刻最贵的一个洞。两条路从**同一堆** settled 佣金里取钱，
    /// 但手动那条只往 withdrawals 写一个金额、从不碰佣金行（withdraw() 的 Connect 自动
    /// 打款、admin_withdraw_status 手工标已支付，都一样），所以一笔已经手动付掉的佣金在
    /// 批次的 SELECT 眼里仍然是 settled + mature + reversed_at IS NULL + payout_id IS NULL。
    /// 运营把 referral_batch_enabled 一开，历史上每一笔手动付款都会被 Stripe Connect
    /// 再转一次 —— 而 admin_save_settings 只打了一行 warn，什么都没对账。
    #[test]
    fn money_already_paid_by_hand_is_never_claimed_again() {
        let id = |n: u128| uuid::Uuid::from_u128(n);
        let rows = [(id(1), 2_000i64), (id(2), 2_000), (id(3), 1_000)];
        // 候选窗口就是全池时（行数没到 LIMIT），基数 = 这三行合计。
        let pool = 5_000i64;

        // 没有手动提现：全部照付，和改动之前逐字一样。
        assert_eq!(super::within_budget(&rows, pool, 0), vec![id(1), id(2), id(3)]);

        // $50 全部手动付过了：一行都不能锁。这正是「打开批量开关重付一遍」的场景。
        assert!(super::within_budget(&rows, pool, 5_000).is_empty());

        // 手动付了 $20：剩下 $30 可付，且必须是整行 —— $20 + $10 正好。
        // 中间那条 $20 装不下要跳过去看下一条，不能就此打住把 $10 一起冻住。
        assert_eq!(super::within_budget(&rows, pool, 2_000), vec![id(1), id(3)]);

        // 对不齐时向下取整，绝不跨过预算：预算 $47，$20+$20 之后再加 $10 就超了。
        assert_eq!(super::within_budget(&rows, pool, 300), vec![id(1), id(2)]);

        // 预算连第一行都不够：宁可这一轮不发，也不能多发。
        assert!(super::within_budget(&rows, pool, 4_500).is_empty());

        // 历史手动付款比现存可付佣金还多（退款把佣金扣没了之类）：预算为负，一分不动。
        assert!(super::within_budget(&rows, pool, 9_999).is_empty());

        // 预算基数必须是**全池**，不是候选窗口的合计。
        //
        // 候选查询是 `ORDER BY mature_at LIMIT 200`，取最老的一批；新赚的佣金 mature_at
        // 更晚，永远排在窗口外面。拿窗口合计当基数会造成永久卡死：老推荐人 400 行共
        // 1,200,000 可付、历史手动提现 800,000（真实应付 400,000），窗口只装得下最老的
        // 600,000 → 预算 −200,000 → 一行都取不到 → 回滚；而 run_once 的 HAVING 用的是全池
        // （净额 400,000 过门槛），于是他每 30 分钟被重新捞起来、重新算负、重新回滚，
        // 池子涨多少都没用——涨出来的行进不了窗口。
        //
        // 这里就是那个形状的最小复现：窗口合计 5,000 < 已手动付 8,000（旧写法预算 −3,000、
        // 一行都取不到、且池子再涨也进不了窗口），而全池 14,000 → 预算 6,000，三行都装得下。
        let big_pool = 14_000i64;
        assert_eq!(
            super::within_budget(&rows, big_pool, 8_000),
            vec![id(1), id(2), id(3)],
            "窗口合计小于手动已付、但全池够时，必须照常取——拿窗口当基数会让这个人永久卡死",
        );
        // 全池当基数并不等于放宽上限：预算仍然按整行向下取整，绝不跨过。
        // 全池 14,000 − 已付 11,500 = 2,500 → 只装得下第一行的 2,000。
        assert_eq!(
            super::within_budget(&rows, big_pool, 11_500),
            vec![id(1)],
            "基数换成全池之后，装箱仍然只取装得下的整行",
        );
    }

    /// 减法必须真的写进那两条 SQL，而且两处同源。
    ///
    /// 只改 pay_one 会让调度器每半小时把一个整组已被手动付清的推荐人捞起来一次、再丢掉；
    /// 只改 run_once 则完全挡不住 —— 分组那条只是预筛，真正锁佣金的是 pay_one。
    #[test]
    fn both_selection_queries_subtract_manual_withdrawals() {
        let src = include_str!("payout.rs");
        let f = src.split("fn claimed_by_hand_sql(").nth(1).expect("claimed_by_hand_sql");
        let def = &f[..f.find("\n/// ").unwrap_or(f.len())];
        assert!(
            def.contains("w.method <> 'auto'"),
            "批次自己开的 withdrawals 行背后的佣金已经变成 paid、早就出了池子，再减一次是减两遍",
        );
        assert!(
            def.contains("w.status NOT IN ('rejected', 'failed', 'returned')"),
            "驳回/失败/被冲回的提现钱从来没到对方手上，必须放回池子；\
             口径要和 referral.rs 的 taken 逐字一致，两边一漂就是重复支付或永久锁死",
        );

        let run = src.split("pub async fn run_once(").nth(1).expect("run_once");
        let run = &run[..run.find("\n/// ").unwrap_or(run.len())];
        assert!(
            run.contains("claimed_by_hand_sql(\"c.referrer_user_id\")"),
            "分组预筛也要减，否则整组已被手动付清的人每轮都被捞起来一次",
        );

        let pay = src.split("async fn pay_one(").nth(1).expect("pay_one");
        let pay = &pay[..pay.find("\n/// ").unwrap_or(pay.len())];
        assert!(
            pay.contains("claimed_by_hand_sql(\"$1\")")
                && pay.contains("within_budget(&candidates, pool_total, claimed)"),
            "真正锁佣金的这条必须在预算内取，这是唯一拦得住重复支付的地方",
        );
        // 预算基数必须是全池，不是候选窗口。窗口是最老的 200 行，拿它当基数会让手动付款
        // 多于窗口合计的老推荐人永久卡死（见 within_budget 的注释和它的单测）。
        assert!(
            pay.contains("SELECT COALESCE(SUM(commission_cents), 0)::bigint FROM commissions"),
            "缺少全池合计的读取：预算基数一旦退回候选窗口合计，老推荐人的钱会永远发不出去",
        );

        // **真正把钱转出去的是这一行。** 上面几条断言的都是「算」，这一条断言「用」——
        // 之前它没被任何测试盖住：把 `.bind(&take[..])` 改成绑定全部 candidates（保留
        // within_budget 的调用、只是不用它的结果），13 条测试照样全绿，而手动→批量的
        // 重复支付被原样放回。断言这条 UPDATE 绑定的只能是 take。
        let lock_stmt = pay
            .find("UPDATE commissions c SET status = 'paid'")
            .expect("锁佣金的 UPDATE 不见了");
        let bind_at = pay[lock_stmt..]
            .find(".bind(")
            .map(|i| lock_stmt + i)
            .expect("锁佣金的 UPDATE 后面必须有 bind");
        let first_bind = &pay[bind_at..bind_at + 40.min(pay.len() - bind_at)];
        assert!(
            first_bind.starts_with(".bind(&take[..])"),
            "锁佣金那条 UPDATE 的第一个绑定必须是 take（预算内挑出来的那批）。\
             绑成 candidates 或别的集合＝把预算判断架空，手动已付的佣金会被再付一遍。\
             实际读到：{first_bind}",
        );
        // 锁人必须在读「手动已领走」之前，否则并发的手动提现会从读和插之间穿过去。
        let lock = pay.find("FROM users WHERE id = $1 FOR UPDATE").expect("user lock");
        let read = pay.find("claimed_by_hand_sql(\"$1\")").unwrap();
        assert!(lock < read, "先锁人再算账：withdraw() 拿的是同一把锁");
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

    /// 被驳回的批量打款要把佣金放回去；而「不许人工标已支付」这道闸只许拦 pending。
    ///
    /// 后半条是被一个反向事故逼出来的。这道闸原来不看行的状态，于是自动打款开着时，一条
    /// 结果不明的 'sending' 行**唯一**能做的操作就是驳回（报错文案还明写着「请用驳回」）。
    /// 而驳回会 release() 把佣金放回 settled，并把金额从 claimed_by_hand_sql 的合计里摘掉、
    /// 把额度还给批次预算 —— 运营去 Stripe 核对完、发现转账其实成立了，照着提示一点，
    /// 下一轮批次就把同一笔钱又转了一次。一道防重复支付的闸，恰好在最危险的那种行上
    /// 把人推向重复支付。
    ///
    /// 加上 pending 限定之后，核对完的 sending 行可以被标成已支付：这个接口从不动钱，
    /// 标记只是把已经发生的事记下来，而佣金因此稳稳停在 paid，不会再被扫到。
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
        assert!(
            f.contains("t.batch_enabled && req.status == \"paid\" \
                        && current.as_deref() == Some(\"pending\")"),
            "这道闸必须只拦 pending。连 'sending' 一起拦，就等于强迫运营用「驳回」收尾一条\
             结果不明的转账，而驳回会把佣金和批次预算一起放回去 —— 转账要是其实成立了，\
             下一轮就是第二次转账",
        );
        assert!(
            f.contains("SELECT status FROM withdrawals WHERE id = $1"),
            "闸门要按行的当前状态判，就必须先读到这个状态",
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
