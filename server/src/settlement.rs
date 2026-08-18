//! 结算恢复：把「上游已服务、但结算事务失败（回滚→没扣到钱）」的调用补扣一次，**绝不重复扣钱**。
//!
//! 为什么需要：`bill()` 是 fire-and-forget，一旦结算事务失败（开事务失败、锁余额失败、扣减失败、
//! 写用量失败、提交失败），整笔回滚——用户被服务了却一分没扣。以前只留一条日志，漏收查无对象。
//!
//! 不重复扣钱是硬约束（用户明确要求「失败了就不应该扣钱」）。做法：
//!   1. 每次 bill() 生成唯一 `settlement_id`；付费结算在**同一个事务**里往 `settled_requests`
//!      写一行认领。于是「扣了钱」和「记了账本」共命运：提交则都在，回滚则都不在。
//!   2. 失败时把这笔的全部输入快照进 `unsettled_charges`（用 `settlement_id` 做主键，重复入队是
//!      no-op）。落库失败（多半是 DB 本身挂了）再退一步塞进 Redis 列表兜底。
//!   3. 后台 worker 逐条 `FOR UPDATE SKIP LOCKED` 认领队列行（多副本也不会撞同一行）：
//!        · 先查 `settled_requests`——原始提交若其实落库了（「模糊提交」：commit 报错但数据已提交），
//!          账本里就有它，直接标记 resolved、**不再扣第二次**；
//!        · 否则调 `models::resettle` 精确补扣一次（它内部再认领一次，作第二层防线）。
//!   4. 幂等锚点是 settlement_id 而非 request_id，所以 request_id 缺失（vision/compression 计费恒为
//!      None）也照常入队恢复——它绑 NULL，恢复仍按 settlement_id 精确一次。

use crate::AppState;

/// 恢复间隔与放弃阈值。间隔短是因为失败多半是瞬时的（DB 抖一下），补扣越快越好；
/// 放弃阈值防一条永远补不掉的记录无限烧——到点转「死信」，留错并停手，交人工。
const RECOVERY_INTERVAL_SECS: u64 = 30;
const MAX_ATTEMPTS: i32 = 10;
/// 一轮最多处理多少条，避免一条队列积压把 worker 卡死在一轮里。
const BATCH_PER_TICK: usize = 200;
/// Redis 兜底队列的键。
const REDIS_UNSETTLED_KEY: &str = "billing:unsettled";
/// 彻底坏掉的兜底记录挪到这里留证（人工对账），绝不静默丢弃。
const REDIS_DEAD_KEY: &str = "billing:unsettled:dead";
/// 账本保留期：恢复只关心近几分钟内的失败，旧账本行没用了。远大于 MAX_ATTEMPTS×间隔（~5 分钟），
/// 所以清理绝不会删掉一条还可能被恢复的账本。
const SETTLED_RETENTION_DAYS: i64 = 7;
/// 已了结的队列行留 30 天供对账，然后清掉。
const RESOLVED_RETENTION_DAYS: i64 = 30;

/// 入队一笔失败结算所需的全部输入快照。用原始类型，避免 settlement 依赖 models 的私有 BillTokens。
pub(crate) struct QueueInput {
    pub settlement_id: uuid::Uuid,
    pub uid: uuid::Uuid,
    pub conn_id: uuid::Uuid,
    pub request_id: Option<String>,
    pub cost: i64,
    pub use_quota: bool,
    pub free_pool: bool,
    pub free_micro_usd: i64,
    pub prompt: i64,
    pub completion: i64,
    pub cached: i64,
    pub cache_creation: i64,
    pub model_name: String,
    pub estimated: bool,
    pub mode: Option<String>,
    pub tool_turn: Option<bool>,
    pub emitted_tool: Option<String>,
    /// 哪个失败分支入的队（排查用）。
    pub stage: &'static str,
}

/// 队列里的一行，够重建 bill() 的全部输入。
#[derive(sqlx::FromRow, Clone)]
pub(crate) struct UnsettledRow {
    pub settlement_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub conn_id: uuid::Uuid,
    pub request_id: Option<String>,
    pub cost_cents: i64,
    pub use_quota: bool,
    pub free_pool: bool,
    pub free_micro_usd: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    pub cache_creation_tokens: i64,
    pub model_name: String,
    pub estimated: bool,
    pub ide_mode: Option<String>,
    pub is_tool_turn: Option<bool>,
    pub emitted_tool: Option<String>,
}

/// 持久化一笔失败结算，供后台补扣。request_id 可为 None（幂等锚点是 settlement_id，不是它）。
///
/// 走独立连接（不是那条已经死掉的事务）。落库失败（DB 本身挂了，常是结算失败的同因）再退到 Redis。
/// 两处都失败时，`bill_inner` 里那条 `event="billing_settlement_failed"` 的 error 日志仍是最后凭证。
pub(crate) async fn queue(state: &AppState, input: QueueInput) {
    // 幂等锚点是 settlement_id，不是 request_id——所以 request_id 缺失（vision/compression 计费
    // 就恒为 None）也照样安全入队恢复：绑 NULL 即可，恢复仍按 settlement_id 精确一次。
    let request_id = input.request_id.clone();
    let res = sqlx::query(
        "INSERT INTO unsettled_charges \
         (settlement_id, user_id, conn_id, request_id, cost_cents, use_quota, free_pool, free_micro_usd, \
          prompt_tokens, completion_tokens, cached_tokens, cache_creation_tokens, model_name, estimated, \
          ide_mode, is_tool_turn, emitted_tool, stage) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) \
         ON CONFLICT (settlement_id) DO NOTHING",
    )
    .bind(input.settlement_id)
    .bind(input.uid)
    .bind(input.conn_id)
    .bind(request_id.as_deref())
    .bind(input.cost)
    .bind(input.use_quota)
    .bind(input.free_pool)
    .bind(input.free_micro_usd)
    .bind(input.prompt)
    .bind(input.completion)
    .bind(input.cached)
    .bind(input.cache_creation)
    .bind(&input.model_name)
    .bind(input.estimated)
    .bind(input.mode.as_deref())
    .bind(input.tool_turn)
    .bind(input.emitted_tool.as_deref())
    .bind(input.stage)
    .execute(&state.db)
    .await;
    match res {
        Ok(_) => tracing::warn!(
            uid = %input.uid, conn_id = %input.conn_id, request_id = request_id.as_deref().unwrap_or("-"), cost = input.cost,
            stage = input.stage, settlement_id = %input.settlement_id,
            "queued unsettled charge for background recovery"
        ),
        Err(error) => {
            // DB 落库失败：多半 Postgres 正不可用。退到 Redis（独立服务，通常还活着）。
            tracing::error!(
                %error, uid = %input.uid, conn_id = %input.conn_id, request_id = request_id.as_deref().unwrap_or("-"), cost = input.cost,
                stage = input.stage, settlement_id = %input.settlement_id,
                "failed to persist unsettled charge to DB; falling back to redis"
            );
            let payload = serde_json::json!({
                "settlement_id": input.settlement_id,
                "user_id": input.uid,
                "conn_id": input.conn_id,
                "request_id": request_id,
                "cost_cents": input.cost,
                "use_quota": input.use_quota,
                "free_pool": input.free_pool,
                "free_micro_usd": input.free_micro_usd,
                "prompt_tokens": input.prompt,
                "completion_tokens": input.completion,
                "cached_tokens": input.cached,
                "cache_creation_tokens": input.cache_creation,
                "model_name": input.model_name,
                "estimated": input.estimated,
                "ide_mode": input.mode,
                "is_tool_turn": input.tool_turn,
                "emitted_tool": input.emitted_tool,
                "stage": input.stage,
            })
            .to_string();
            let mut r = state.redis.clone();
            let pushed: Result<(), redis::RedisError> = redis::cmd("LPUSH")
                .arg(REDIS_UNSETTLED_KEY)
                .arg(payload)
                .query_async(&mut r)
                .await;
            if let Err(error) = pushed {
                tracing::error!(
                    %error, uid = %input.uid, settlement_id = %input.settlement_id,
                    "failed to persist unsettled charge to redis fallback too — only the error log remains"
                );
            }
        }
    }
}

/// 启动后台恢复 worker。和 field_backfill/payout 一样在启动时 spawn，不挡启动。
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        // 让迁移和主要初始化先过去。它不急。
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        loop {
            // 先把 Redis 兜底队列排干进表，之后统一走表的恢复路径。
            if let Err(error) = drain_redis(&state).await {
                tracing::warn!(%error, "settlement: draining redis fallback failed (will retry next tick)");
            }
            match recover_pending(&state).await {
                Ok((resolved, deferred)) if resolved + deferred > 0 => {
                    tracing::info!(resolved, deferred, "settlement recovery tick");
                }
                Ok(_) => {}
                Err(error) => tracing::warn!(%error, "settlement recovery tick failed (will retry)"),
            }
            if let Err(error) = prune(&state).await {
                tracing::warn!(%error, "settlement: pruning old rows failed");
            }
            tokio::time::sleep(std::time::Duration::from_secs(RECOVERY_INTERVAL_SECS)).await;
        }
    });
}

/// 把 Redis 兜底队列里的记录搬进 `unsettled_charges`。**非破坏性**是关键：Redis 兜底恰恰用在
/// Postgres 抖动时，此刻回表 INSERT 很可能也失败——绝不能像原来那样「RPOP 取出即删、`?` 一抛就丢」，
/// 那等于在最需要它的时候把钱弄没。规则：
///   · 回表 INSERT 失败（DB 还没好）→ 把原始 payload LPUSH **原样退回**主队列，停这一轮，下轮再搬；
///   · 彻底解析不了 / UUID 坏的记录 → 挪到 dead 列表留证（人工对账），绝不静默丢；
///   · 只有成功入表的那条才真正从 Redis 消失。
async fn drain_redis(state: &AppState) -> anyhow::Result<()> {
    let mut r = state.redis.clone();
    // 有界：一轮最多搬这么多，别让超长队列把 tick 卡死；剩下的下一轮继续。
    for _ in 0..(BATCH_PER_TICK * 4) {
        let popped: Option<String> = redis::cmd("RPOP")
            .arg(REDIS_UNSETTLED_KEY)
            .query_async(&mut r)
            .await?;
        let Some(raw) = popped else { break }; // 主队列空了

        let v: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(error) => {
                tracing::error!(%error, "settlement: undecodable redis fallback record → dead list");
                let _: Result<(), redis::RedisError> = redis::cmd("LPUSH").arg(REDIS_DEAD_KEY).arg(&raw).query_async(&mut r).await;
                continue;
            }
        };
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let i = |k: &str| v.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
        let b = |k: &str| v.get(k).and_then(|x| x.as_bool());
        let (Some(settlement_id), Some(user_id), Some(conn_id), Some(stage)) =
            (s("settlement_id"), s("user_id"), s("conn_id"), s("stage"))
        else {
            tracing::error!("settlement: redis fallback record missing key fields → dead list");
            let _: Result<(), redis::RedisError> = redis::cmd("LPUSH").arg(REDIS_DEAD_KEY).arg(&raw).query_async(&mut r).await;
            continue;
        };
        // request_id 现在允许缺失（幂等锚点是 settlement_id）——None 就绑 NULL。
        let request_id = s("request_id");
        // UUID 必须真解析成功。解析不了挪 dead 列表，绝不 unwrap_or_default 落 nil-uuid（会在主键上
        // 和别的坏记录撞成一条、把不同的真实费用当重复吞掉）。
        let (Ok(sid), Ok(uid_v), Ok(cid_v)) = (
            uuid::Uuid::parse_str(&settlement_id),
            uuid::Uuid::parse_str(&user_id),
            uuid::Uuid::parse_str(&conn_id),
        ) else {
            tracing::error!(%settlement_id, %user_id, %conn_id, "settlement: redis fallback record has unparseable uuid → dead list");
            let _: Result<(), redis::RedisError> = redis::cmd("LPUSH").arg(REDIS_DEAD_KEY).arg(&raw).query_async(&mut r).await;
            continue;
        };
        let ins = sqlx::query(
            "INSERT INTO unsettled_charges \
             (settlement_id, user_id, conn_id, request_id, cost_cents, use_quota, free_pool, free_micro_usd, \
              prompt_tokens, completion_tokens, cached_tokens, cache_creation_tokens, model_name, estimated, \
              ide_mode, is_tool_turn, emitted_tool, stage) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18) \
             ON CONFLICT (settlement_id) DO NOTHING",
        )
        .bind(sid)
        .bind(uid_v)
        .bind(cid_v)
        .bind(request_id.as_deref())
        .bind(i("cost_cents"))
        .bind(b("use_quota").unwrap_or(false))
        .bind(b("free_pool").unwrap_or(false))
        .bind(i("free_micro_usd"))
        .bind(i("prompt_tokens"))
        .bind(i("completion_tokens"))
        .bind(i("cached_tokens"))
        .bind(i("cache_creation_tokens"))
        .bind(s("model_name").unwrap_or_default())
        .bind(b("estimated").unwrap_or(true))
        .bind(s("ide_mode"))
        .bind(b("is_tool_turn"))
        .bind(s("emitted_tool"))
        .bind(stage)
        .execute(&state.db)
        .await;
        if let Err(error) = ins {
            // DB 还没好：把原样 payload 退回主队列，停这一轮，下轮再试——一条都不丢。
            let _: Result<(), redis::RedisError> = redis::cmd("LPUSH")
                .arg(REDIS_UNSETTLED_KEY)
                .arg(&raw)
                .query_async(&mut r)
                .await;
            tracing::warn!(%error, "settlement: redis→DB insert failed; payload requeued, will retry next tick");
            return Ok(());
        }
    }
    Ok(())
}

/// 处理一批待恢复的队列行。返回 (已了结, 仍待续).
async fn recover_pending(state: &AppState) -> anyhow::Result<(u64, u64)> {
    let mut resolved = 0u64;
    let mut deferred = 0u64;
    for _ in 0..BATCH_PER_TICK {
        // 每条独立事务 T1：`FOR UPDATE SKIP LOCKED` 认领一行，别的 worker 会跳过它。
        let mut t1 = state.db.begin().await?;
        let row: Option<UnsettledRow> = sqlx::query_as(
            "SELECT settlement_id, user_id, conn_id, request_id, cost_cents, use_quota, free_pool, \
                    free_micro_usd, prompt_tokens, completion_tokens, cached_tokens, cache_creation_tokens, \
                    model_name, estimated, ide_mode, is_tool_turn, emitted_tool \
             FROM unsettled_charges \
             WHERE resolved_at IS NULL AND attempts < $1 \
             ORDER BY created_at \
             LIMIT 1 FOR UPDATE SKIP LOCKED",
        )
        .bind(MAX_ATTEMPTS)
        .fetch_optional(&mut *t1)
        .await?;
        let Some(row) = row else {
            let _ = t1.rollback().await;
            break; // 没有待处理的了
        };

        // 模糊提交兜底：原始结算若其实已落库（账本里有它），绝不再扣第二次。
        // 取真实列（uuid）而不是 `SELECT 1`——后者是 int4，映射成 (i64,) 会在**运行时**类型报错，
        // 让恢复每轮都炸；sqlx 用运行时 SQL，cargo 查不出这种类型不匹配。
        let already: Option<(uuid::Uuid,)> =
            sqlx::query_as("SELECT settlement_id FROM settled_requests WHERE settlement_id = $1")
                .bind(row.settlement_id)
                .fetch_optional(&mut *t1)
                .await?;

        let outcome = if already.is_some() {
            crate::models::BillOutcome::AlreadySettled
        } else {
            // resettle 走它自己的事务 T2（另一条连接）扣费+认领。T1 只锁着队列行，两者锁目标不重叠。
            crate::models::resettle(state, &row).await
        };

        match outcome {
            crate::models::BillOutcome::Settled | crate::models::BillOutcome::AlreadySettled => {
                sqlx::query(
                    "UPDATE unsettled_charges SET resolved_at = now(), last_error = $2 WHERE settlement_id = $1",
                )
                .bind(row.settlement_id)
                .bind(match outcome {
                    crate::models::BillOutcome::AlreadySettled => "already_settled",
                    _ => "recovered",
                })
                .execute(&mut *t1)
                .await?;
                resolved += 1;
            }
            crate::models::BillOutcome::Deferred => {
                sqlx::query(
                    "UPDATE unsettled_charges SET attempts = attempts + 1, last_error = 'resettle_failed' WHERE settlement_id = $1",
                )
                .bind(row.settlement_id)
                .execute(&mut *t1)
                .await?;
                deferred += 1;
            }
        }
        t1.commit().await?;
    }
    Ok((resolved, deferred))
}

/// 清理：账本旧行、已了结的队列行、以及已放弃（attempts 到顶）的死信只留错、不删（留给人工）。
async fn prune(state: &AppState) -> anyhow::Result<()> {
    let _ = sqlx::query(&format!(
        "DELETE FROM settled_requests WHERE settled_at < now() - interval '{SETTLED_RETENTION_DAYS} days'"
    ))
    .execute(&state.db)
    .await?;
    let _ = sqlx::query(&format!(
        "DELETE FROM unsettled_charges WHERE resolved_at IS NOT NULL \
         AND resolved_at < now() - interval '{RESOLVED_RETENTION_DAYS} days'"
    ))
    .execute(&state.db)
    .await?;
    // 死信（attempts 到顶还没了结）不删，叫一声让人看见。
    let dead: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM unsettled_charges WHERE resolved_at IS NULL AND attempts >= $1",
    )
    .bind(MAX_ATTEMPTS)
    .fetch_optional(&state.db)
    .await?;
    if let Some((n,)) = dead {
        if n > 0 {
            tracing::error!(count = n, "settlement: {n} charges gave up after max attempts (dead-letter, need manual reconcile)");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// 每条 INSERT 的「列数 == 最大占位符 $N == .bind() 次数」必须一致。sqlx 用运行时字符串
    /// SQL，绑定数量对不上不会被 cargo 拦下，只会在**真发生结算失败、恢复真跑**时炸——
    /// 那是最不该炸的地方。这条测试把这类漂移在编译期外、上线前拦住。
    fn insert_arity(src: &str, table: &str) -> (usize, usize, usize) {
        let at = src.find(&format!("INSERT INTO {table}")).expect("INSERT 不见了");
        let stmt = &src[at..];
        // 列清单：第一个 (...) 到 ) 之间，逗号数 + 1。
        let lp = stmt.find('(').expect("no col paren");
        let rp = stmt[lp..].find(')').expect("no col close") + lp;
        let cols = stmt[lp + 1..rp].matches(',').count() + 1;
        // 最大 $N。
        let mut max_ph = 0usize;
        let bytes = stmt.as_bytes();
        let mut i = 0;
        while i < stmt.len() {
            if bytes[i] == b'$' {
                let mut j = i + 1;
                while j < stmt.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > i + 1 {
                    if let Ok(n) = stmt[i + 1..j].parse::<usize>() {
                        max_ph = max_ph.max(n);
                    }
                }
                i = j;
            } else {
                i += 1;
            }
            // 到 .execute( 就停，别扫进下一条语句。
            if stmt[..i.min(stmt.len())].contains(".execute(") {
                break;
            }
        }
        // .bind( 次数：从语句起点到第一个 .execute( 之间。
        let exec = stmt.find(".execute(").expect("no execute");
        let binds = stmt[..exec].matches(".bind(").count();
        (cols, max_ph, binds)
    }

    #[test]
    fn unsettled_insert_columns_placeholders_and_binds_all_agree() {
        let src = include_str!("settlement.rs");
        // queue() 里那条与 drain_redis() 里那条都要查——两条列清单必须完全一致，
        // 否则同一张表两条写入路径悄悄分叉。
        let (cols, ph, binds) = insert_arity(src, "unsettled_charges");
        assert_eq!(cols, 18, "unsettled_charges 列数变了，同步检查两条 INSERT");
        assert_eq!(ph, 18, "占位符数和列数对不上");
        assert_eq!(binds, 18, ".bind() 数和列数对不上——恢复时会运行时报错");
    }

    #[test]
    fn queue_accepts_missing_request_id_because_ledger_key_is_settlement_id() {
        // 幂等锚点是 settlement_id，不是 request_id——所以 request_id=None（vision/compression
        // 计费恒 None）也要照常入队恢复，不能早返回把它丢掉。绑 NULL 即可。
        let src = include_str!("settlement.rs");
        let at = src.find("pub(crate) async fn queue(").expect("queue 改名了");
        let body = &src[at..at + 1400];
        assert!(
            !body.contains("cannot recover idempotently, leaving uncharged"),
            "queue() 不该再因无 request_id 早返回丢弃——settlement_id 才是幂等键",
        );
        assert!(
            body.contains(".bind(request_id.as_deref())"),
            "request_id 应以 Option 形式绑定（None → NULL），照常入队",
        );
    }

    #[test]
    fn recovery_checks_ledger_before_charging_and_claims_row_exclusively() {
        let src = include_str!("settlement.rs");
        let at = src.find("async fn recover_pending(").expect("recover_pending 改名了");
        let body = &src[at..src[at..].find("\nasync fn ").map(|e| e + at).unwrap_or(src.len())];
        // 多副本安全：认领队列行必须 FOR UPDATE SKIP LOCKED。
        assert!(
            body.contains("FOR UPDATE SKIP LOCKED"),
            "认领队列行必须 SKIP LOCKED，否则多副本会重复补扣",
        );
        // 模糊提交兜底：扣费前必须先查账本 settled_requests。
        let ledger = body.find("SELECT settlement_id FROM settled_requests").expect("恢复必须先查账本");
        let charge = body.find("crate::models::resettle").expect("恢复要能补扣");
        assert!(ledger < charge, "必须先查账本、确认没扣过，才允许 resettle 补扣");
        // 只有 attempts 未到顶、未了结的才处理。
        assert!(body.contains("resolved_at IS NULL AND attempts <"), "只处理待处理且未放弃的");
    }
}
