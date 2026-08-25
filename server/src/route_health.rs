//! 一条线路是不是真的在工作 —— 用**真实流量的结局**说话，而不是敲上游的前门。
//!
//! ## 它要修的东西
//!
//! `health.rs` 的探针对线路的 `base_url` 发一个不带凭据的 GET，任何回应都算健康。而十条
//! 线路共用同一个上游域名，所以它其实是把同一次 TCP 握手做了十遍，记录 1–10ms、全绿。
//! 2026-08-19 那次事故里，「Claude 强力版」连续 44 小时零成功，面板从头到尾报 `ok=t 1ms`,
//! **监控一次都没响过**。
//!
//! ## 为什么不是「成功率 + 时间窗」
//!
//! 这是设计里最要紧的一处，第一版就栽在这儿。按成功率判定需要样本量，而这台机器实测
//! 约 1,540 次成功/天，摊到 8–9 条有流量的线路上，**平均每条每小时只有个位数**。于是
//! 「近 60 分钟至少 20 个样本」这类门槛几乎永远够不到，判定只能退到更长的窗；而回退窗
//! 里装的是**故障之前**的成功，它只会把结论往好看的方向拉。按那套规则算一遍这次事故：
//! 一条彻底死掉的线路要 1.2 小时才离开绿色、12 小时才跌破告警线。那不是修好监控，
//! 是把 44 小时换成 12 小时。
//!
//! 所以这里换了口径：**连败次数** 和 **上一次真正成功是什么时候**。这两个量与样本量无关，
//! 每天 4 次请求也能定性 —— 强力版当时是 34 连败，第 5 次就该报出来。
//!
//! ## 为什么放 Redis 而不是建表
//!
//! 需要的状态是「每条线路一行」，不是流水：连败数、上次成功时刻、上次尝试时刻。
//! 十个键，Redis 的 INCR/SET 是原子的、亚毫秒、不占连接池、没有行锁 —— 而这个项目
//! 有过教训：门禁往 users 表写，36 万次 UPDATE 把同一用户的并发请求串行化了。
//! 观测不该和计费（`bill_inner` 跨 BEGIN/UPDATE/COMMIT 持一条连接，失败就是真金）
//! 抢同一个连接池。也不需要保留期、不需要清理、不占盘 —— 这台机器有过被构建塞满盘的
//! 记录，而盘满时 Postgres 拒写等于每个 chat 请求都失败。
//!
//! ## 为什么没有 Drop guard
//!
//! 用 Drop 落库的话，客户端在 `req.send()` 期间断开时 handler future 被丢弃、guard 带着
//! 一个**没查明的初值**落库：记成失败就是把「用户点了停止」算成线路故障，记成成功就是
//! 重新造出这里要杀掉的假绿灯。这里只在**结局确实已知**的四个点显式记一次，客户端取消
//! 时什么都不执行 —— 不写，就不会写错。

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::AppState;

/// 连续多少次非成功就判定这条线路坏了。
///
/// 与样本量无关，这正是它在「每天 4 次请求」的量级上仍然管用的原因。
///
/// 取 5 而不是 3：这台机器的硬失败率常态在 16–20%，3 连败的自然概率约 0.6%（每 170 组
/// 就撞一次），会造成误报；5 连败约 0.02%，而真坏掉的线路一分钟内就能攒够。
const FAILING_STREAK: i64 = 5;

/// 成功多久之内才算「现在是好的」。
///
/// 超过这个时间没有新的成功，并不代表坏了 —— 也可能只是没人用。所以它不产生「坏」，
/// 只是让状态退回「不知道」。**绝不能因为没有证据就报绿**，那正是探针在做的事。
const OK_FRESH_SECS: i64 = 15 * 60;

/// 键的存活期。线路被删或长期不用时不留垃圾；30 天远长于任何判定窗口。
const KEY_TTL_SECS: i64 = 30 * 24 * 3600;

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn key(route_id: Uuid, field: &str) -> String {
    format!("rh:{route_id}:{field}")
}

/// 一条线路当前的健康事实。全部来自真实流量。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RouteHealth {
    /// 连续非成功次数。一次成功清零。
    pub consecutive_failures: i64,
    /// 上一次拿到成功响应的 unix 秒。None = 有记录以来从没成功过。
    pub last_ok_at: Option<i64>,
    /// 上一次**结局已知**的尝试。None = 这条线路根本没被真实流量碰过。
    pub last_attempt_at: Option<i64>,
    /// 上一次失败时上游给的状态码，便于面板直接说清原因。
    pub last_fail_status: Option<i64>,
}

/// 把事实翻译成面板上的状态词。
///
/// **刻意只用 health.rs 现有的那四个词**（ok / degraded / error / unknown）。前端把状态词
/// 查一张四键表来上色，多一个词就是一颗无字无色的空药丸；而且这一屏挂在 /dashboard 上、
/// 所有登录用户都看得到，内部诊断词不该漏给客户。要表达的东西这四个词够用。
///
/// 顺序是判定的一部分，有测试正面钉着：**先判坏，再判好**。反过来的话，小样本全失败
/// 会被「样本不足」一类的中性结论吞掉 —— 那恰好是这次事故的形态。
pub fn classify(h: &RouteHealth, now: i64) -> &'static str {
    // 1) 连败达标 → 坏。与样本量、时间窗都无关，这是唯一能在低流量下有界时间内定性的规则。
    if h.consecutive_failures >= FAILING_STREAK {
        return "error";
    }
    // 2) 试过、但从来没成功过 → 坏。强力版就是这个形状：34 次尝试、0 次成功。
    if h.last_attempt_at.is_some() && h.last_ok_at.is_none() {
        return "error";
    }
    // 3) 根本没被碰过 → 不知道。**不是绿**。
    //    这是真实流量口径的固有盲区：没有请求就没有证据。宁可说不知道，也不要替它担保。
    let Some(last_ok) = h.last_ok_at else {
        return "unknown";
    };
    // 4) 最近成功过，且没在连败 → 好。
    if now.saturating_sub(last_ok) <= OK_FRESH_SECS {
        return if h.consecutive_failures > 0 { "degraded" } else { "ok" };
    }
    // 5) 上次成功已经旧了：坏消息新于好消息就报降级，否则只是没人用 → 不知道。
    if h.consecutive_failures > 0 {
        return "degraded";
    }
    "unknown"
}

/// 记一次**成功**：上游收下请求并开始回话。
///
/// 口径是「这条线路接得通、认得了凭据、开始出字」，不是「这一轮流式完整结束」。
/// 两者刻意分开：流中途断掉在一个 agentic IDE 里多半是用户按了停止，把它算成线路故障
/// 会把好线路刷成红的，然后告警疲劳 —— 那是这次事故的真正成因，不能用另一种方式复制。
pub async fn record_ok(state: &AppState, route_id: Uuid) {
    let mut conn = state.redis.clone();
    let now = now_secs();
    let _: Result<(), _> = redis::pipe()
        .cmd("SET").arg(key(route_id, "ok_at")).arg(now).arg("EX").arg(KEY_TTL_SECS).ignore()
        .cmd("SET").arg(key(route_id, "last_at")).arg(now).arg("EX").arg(KEY_TTL_SECS).ignore()
        .cmd("DEL").arg(key(route_id, "fails")).ignore()
        .query_async(&mut conn)
        .await;
}

/// 记一次**失败**：上游明确报错、卡死不回话、或传输层出错。
///
/// 客户端主动取消**不走这里**：那种情况下 handler future 直接被丢弃，这个函数根本不会被
/// 调用。不写就不会写错，这是不用 Drop guard 换来的。
pub async fn record_fail(state: &AppState, route_id: Uuid, status: u16) {
    let mut conn = state.redis.clone();
    let now = now_secs();
    let _: Result<(), _> = redis::pipe()
        .cmd("INCR").arg(key(route_id, "fails")).ignore()
        .cmd("EXPIRE").arg(key(route_id, "fails")).arg(KEY_TTL_SECS).ignore()
        .cmd("SET").arg(key(route_id, "last_at")).arg(now).arg("EX").arg(KEY_TTL_SECS).ignore()
        .cmd("SET").arg(key(route_id, "fail_status")).arg(status as i64).arg("EX").arg(KEY_TTL_SECS).ignore()
        .query_async(&mut conn)
        .await;
}

/// 记一次成功，**不等它写完**。
///
/// 派单路径上一个 await 都不加：观测失败绝不能让用户多等一毫秒，也绝不能把一次请求
/// 拖垮。Redis 写不进去的后果只是这条线路暂时"不知道"——而"不知道"不会被判成绿。
pub fn spawn_ok(state: &AppState, route_id: Uuid) {
    let st = state.clone();
    tokio::spawn(async move { record_ok(&st, route_id).await });
}

/// 记一次失败，同样不等。
pub fn spawn_fail(state: &AppState, route_id: Uuid, status: u16) {
    let st = state.clone();
    tokio::spawn(async move { record_fail(&st, route_id, status).await });
}

/// 读一条线路的当前事实。Redis 读不到就返回全空 —— 全空经 `classify` 得到 "unknown"，
/// 不会变成绿灯。
pub async fn snapshot(state: &AppState, route_id: Uuid) -> RouteHealth {
    let mut conn = state.redis.clone();
    let got: Result<(Option<i64>, Option<i64>, Option<i64>, Option<i64>), _> = redis::cmd("MGET")
        .arg(key(route_id, "fails"))
        .arg(key(route_id, "ok_at"))
        .arg(key(route_id, "last_at"))
        .arg(key(route_id, "fail_status"))
        .query_async(&mut conn)
        .await;
    match got {
        Ok((fails, ok_at, last_at, fail_status)) => RouteHealth {
            consecutive_failures: fails.unwrap_or(0),
            last_ok_at: ok_at,
            last_attempt_at: last_at,
            last_fail_status: fail_status,
        },
        Err(err) => {
            tracing::warn!(%err, %route_id, "线路健康读取失败，按「不知道」处理");
            RouteHealth::default()
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 金丝雀：给**没有真实流量**的线路一个证据来源
// ─────────────────────────────────────────────────────────────────────────────
//
// 真实流量口径有一个固有盲区：没有请求就没有证据，那条线路只能显示「不知道」。
// 这比原来的假绿灯诚实，但覆盖不到事故里最危险的一类 —— 实测「Claude 强力版」168 小时里
// 只有 3 小时有流量，Kimi 建库至今零调用。它们坏了也没人会知道，直到某个用户点中它。
//
// 所以对**近期没有证据**的线路，自己发一次最小的真实请求：max_tokens=1、两个 token 的提示。
// 三条纪律：
//   · 只探没有新鲜证据的线路 —— 忙碌的线路本来就有真实流量，一分钱都不该花；
//   · 每轮有条数上限，避免某天多配了几十条线路时一次烧穿；
//   · 有开关（ROUTE_CANARY=0），因为它花的是真钱。
//
// **必须按线路的协议分支。** 直接照 model_probe.rs 那样只发 OpenAI 形状的话，所有
// anthropic 线路都会探测失败 —— 而假红比假绿更糟：它会把好线路报成坏的，然后告警被静音。

/// 多久跑一轮。
const CANARY_EVERY: Duration = Duration::from_secs(15 * 60);
/// 这条线路多久之内有过证据就不探 —— 有真实流量时一分钱都不花。
const CANARY_SKIP_IF_FRESH_SECS: i64 = 15 * 60;
/// 一轮最多探几条。防止线路数量变多时一次烧穿。
const CANARY_MAX_PER_ROUND: usize = 4;
/// 单次探测的耐心。远短于派单路径的 57 秒：这里只问「接不接得通」，不等模型思考。
const CANARY_TIMEOUT: Duration = Duration::from_secs(20);

fn canary_enabled() -> bool {
    std::env::var("ROUTE_CANARY").ok().as_deref() != Some("0")
}

/// 对一条线路发一次最小真实请求。
///
/// 返回 `None` = **这一次没有产生任何证据**，调用方必须什么都不记。
///
/// 这个区分是必须显式表达的：第一版这里在「无从探起」时返回了 `(true, 0)`，而调用方看到
/// `ok=true` 就 `record_ok` —— 凭空造了一次成功、把连败计数清零、点亮绿灯。注释当时写的是
/// 「不产生证据，也不产生结论」，代码做的却是相反的事。这正是这套监控要消灭的东西
/// （没有证据不许报绿），结果在它自己身上重演了一遍。
///
/// 探哪个模型要和**派单口径一致**（`allowed_ids`）：`enabled_models` 为空时派单会回落到
/// `model_id`，那种线路照样在接真实流量，不能因为第一个字段是空的就当它不存在。
async fn canary_once(m: &crate::models::Model) -> Option<(bool, u16)> {
    let http = reqwest::Client::builder().timeout(CANARY_TIMEOUT).build().ok()?;
    let ids = crate::models::allowed_ids(m);
    let model_id = ids.first()?;
    let key = crate::models::model_key(&m.api_key);
    let base = crate::models::api_base(&m.base_url);
    let req = if m.protocol == "anthropic" {
        http.post(format!("{base}/messages"))
            .header("x-api-key", &key)
            .header("anthropic-version", "2023-06-01")
            .json(&serde_json::json!({
                "model": model_id,
                "max_tokens": 1,
                "messages": [{ "role": "user", "content": "hi" }],
            }))
    } else {
        http.post(format!("{base}/chat/completions"))
            .header("Authorization", format!("Bearer {key}"))
            .json(&serde_json::json!({
                "model": model_id,
                "max_tokens": 1,
                "messages": [{ "role": "user", "content": "hi" }],
            }))
    };
    match req.send().await {
        Ok(r) => {
            let s = r.status().as_u16();
            Some((r.status().is_success(), s))
        }
        // 超时/连不上：和派单路径上的卡死是同一种坏，用同一个码，面板上读起来一致。
        Err(_) => Some((false, 504)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 卡死恢复探针：停机期间不让用户当探针
// ─────────────────────────────────────────────────────────────────────────────
//
// 派单路径上一条线路卡满表头预算后会记一个 120 秒的卡死记号（`models::mark_route_stall`）。
// 在这之前，记号只有两种退出方式：用户的真实请求真的拿到表头，或者 120 秒自然过期。
// 两种都由用户付费：记号在世时每个落上去的请求挂 25 秒；过期后下一个用户再挂满 57 秒；
// 停机持续多久就循环多久 —— 44 小时事故就是这个形状。而上面的巡检金丝雀反而不探它：
// 每次失败都刷新 last_attempt_at，被当成「有新鲜证据」跳过。
//
// 对用户请求做并发赛马是不行的：首字节=全文的中转在回表头前已经在跑模型，双发就是
// 双计费（models.rs 里「一次用户发送只对应一次上游调用」那条不变量）。所以赛的是探针：
// 卡死后按线路起一个后台任务，每 30 秒发一次 1-token 的最小真实请求（复用 `canary_once`
// 的协议分支）。失败就把记号续上 —— 停机期间线路持续降权、持续短预算，而不是 120 秒后
// 让用户去撞；成功就撤记号、撤冷却、记一次真实成功，任务退出。
//
// 纪律：
//   · 只对有记号的线路跑，记号一消失（真实流量拿到表头、或兜底过期）任务立刻停；
//   · 同一条线路只有一个任务；并发任务总数有上限 —— 超出的线路退回 120 秒过期的老路；
//   · 受同一个 ROUTE_CANARY 开关约束，因为它花的也是真钱；
//   · 「无从探起」（None）什么都不记、任务退出，和巡检金丝雀一样不伪造证据。

/// 两次恢复探测之间的间隔。必须明显短于卡死记号的有效期，否则记号会在两次探测之间
/// 过期、线路回到排头、用户又成了探针。
const STALL_RECOVERY_EVERY: Duration = Duration::from_secs(30);
/// 同时在跑的恢复任务上限。一次把整批线路都卡死（上游整体故障）时，不让探针本身
/// 变成一次小型压测；超出上限的线路退回 120 秒自然过期那条老路。
const STALL_RECOVERY_MAX_CONCURRENT: usize = 4;

/// 正在跑恢复任务的线路。进程内存即可：记号本身也在进程内存里，发版两边一起清零。
static STALL_RECOVERY_ACTIVE: LazyLock<Mutex<HashSet<Uuid>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// 这条线路现在能不能起一个恢复任务。返回 true 表示**已经占了名额**，调用方必须保证
/// 任务结束时 `stall_recovery_release`。
fn stall_recovery_admit(route_id: Uuid) -> bool {
    let Ok(mut active) = STALL_RECOVERY_ACTIVE.lock() else {
        return false;
    };
    if active.contains(&route_id) || active.len() >= STALL_RECOVERY_MAX_CONCURRENT {
        return false;
    }
    active.insert(route_id);
    true
}

fn stall_recovery_release(route_id: Uuid) {
    if let Ok(mut active) = STALL_RECOVERY_ACTIVE.lock() {
        active.remove(&route_id);
    }
}

/// 任务无论怎么结束（正常退出、panic、运行时关闭时被丢弃）都把名额还回去。
struct StallRecoverySlot(Uuid);
impl Drop for StallRecoverySlot {
    fn drop(&mut self) {
        stall_recovery_release(self.0);
    }
}

/// 一条线路刚刚卡满表头预算 —— 起一个后台任务替用户去探它什么时候恢复。
///
/// 调用时机是派单路径上 `mark_route_stall` 之后。派单路径上一个 await 都不加：这里只
/// 占名额、spawn，立刻返回。
pub fn spawn_stall_recovery(state: &AppState, m: crate::models::Model) {
    if !canary_enabled() {
        return;
    }
    if !stall_recovery_admit(m.health_id()) {
        return;
    }
    let st = state.clone();
    tokio::spawn(async move {
        let _slot = StallRecoverySlot(m.health_id());
        loop {
            tokio::time::sleep(STALL_RECOVERY_EVERY).await;
            // 记号没了 —— 要么真实流量已经拿到表头（clear_route_stall），要么兜底过期。
            // 两种都不该再花钱探。
            if !crate::models::route_recently_stalled(m.health_id(), Instant::now()) {
                tracing::info!(route = %m.label, "卡死记号已撤，恢复探针退出");
                return;
            }
            match canary_once(&m).await {
                Some((true, status)) => {
                    crate::models::clear_route_stall(m.health_id());
                    crate::models::clear_route_cooldown(m.health_id());
                    record_ok(&st, m.health_id()).await;
                    tracing::info!(route = %m.label, status, "卡死线路已由后台探针确认恢复，回到轮换");
                    return;
                }
                Some((false, status)) => {
                    // 还没好：把记号续上，让它在停机期间持续降权、持续短预算。
                    crate::models::mark_route_stall(m.health_id());
                    record_fail(&st, m.health_id(), status).await;
                    tracing::warn!(route = %m.label, status, "卡死线路仍未恢复（后台探针）");
                }
                // 无从探起（没有任何可用模型 id）：什么都不记，退出。记号按 120 秒自然过期。
                None => {
                    tracing::warn!(route = %m.label, "恢复探针无从探起：这条线路没有任何可用模型 id");
                    return;
                }
            }
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// 告警：坏了要有人知道
// ─────────────────────────────────────────────────────────────────────────────
//
// 这次事故里监控从头到尾没报过警，而全仓搜不到任何针对健康的阈值或通知代码 ——
// 不是阈值设错，是**根本没有**。面板改准了，如果没人去看，44 小时还是 44 小时。
//
// 收件人取 `role='admin'` 的邮箱，不新造一个密钥：自配置、可发现，加一个运维进来就自动
// 收到。邮件开着却一个管理员都没有时，启动会明确报错 —— 一个没有收件人的告警系统，
// 和没有告警系统是一回事，但更危险，因为它看起来像有。

/// 连续判坏多久才发。给一次抖动留出自愈的时间，也避免部署瞬间的空窗触发。
const ALARM_AFTER_SECS: i64 = 5 * 60;
/// 同一条线路两次通知之间的最小间隔。告警疲劳是这次事故没人看的真正成因。
const ALARM_COOLDOWN_SECS: i64 = 6 * 3600;

/// 收件人只认**看起来像邮箱**的那些。
///
/// `users.email` 这一列并不保证是邮箱：线上实测有一个 admin 的值是 `fendoushaonian`
/// —— 一个用户名，14 个字符、连 @ 都没有。原来不筛就直接发，结果每一轮巡检都往
/// 邮件服务打一次必然失败的请求，日志里稳定刷 `email is not valid in to`，
/// 真正的发送失败反而被埋在这堆噪声里。
fn looks_like_email(s: &str) -> bool {
    let s = s.trim();
    let Some((user, domain)) = s.split_once('@') else {
        return false;
    };
    !user.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && s.len() <= 254
        && !s.contains(char::is_whitespace)
        && s.matches('@').count() == 1
}

async fn alarm_recipients(state: &AppState) -> Vec<String> {
    let all = sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE role = 'admin'")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    let (good, bad): (Vec<_>, Vec<_>) = all.into_iter().partition(|e| looks_like_email(e));
    if !bad.is_empty() {
        // 不打印地址本身，只报数量：这行会进日志，而管理员的联系方式不该躺在那里。
        tracing::warn!(
            skipped = bad.len(),
            usable = good.len(),
            "有 admin 账号的 email 字段不是邮箱地址，已跳过（那是用户名，不是收件人）"
        );
    }
    good
}

/// 给管理员发一封。别的模块要发告警时走这里，**不要**去改 `notify` 的签名 ——
/// 下面有一条源码断言逐字钉着那一行（它守的是「告警必须真发出去才算发过」）。
pub(crate) async fn notify_admins(state: &AppState, subject: &str, body: &str) -> bool {
    notify(state, subject, body).await
}

/// 发出去。返回**是否至少有一封成功** —— 调用方靠它决定要不要保留冷却。
async fn notify(state: &AppState, subject: &str, body: &str) -> bool {
    if !state.cfg.mail_enabled() {
        tracing::error!(subject, "线路告警无法发出：邮件未配置（brevo_api_key / mail_from 为空）");
        return false;
    }
    let to = alarm_recipients(state).await;
    if to.is_empty() {
        tracing::error!(subject, "线路告警无处可发：没有 email 字段是有效邮箱的 admin 账号");
        return false;
    }
    let mut any_ok = false;
    for addr in to {
        match crate::email::send_mail(&state.cfg, &addr, subject, body, false).await {
            Ok(()) => any_ok = true,
            // 发不出去也要留痕：静默失败等于没有告警，而这正是要修的东西。
            Err(err) => tracing::error!(reason = %err.msg, subject, "线路告警发送失败"),
        }
    }
    any_ok
}

/// `POST /api/admin/route-health/test-alarm` —— 往真实收件人发一封测试告警。
///
/// # 为什么需要这个按钮
///
/// 「地址在收件人列表里」和「这封信真能到」是两件事。QQ 邮箱对陌生发件域尤其严 ——
/// 可能静默丢掉，也可能进垃圾箱，而两种在服务端看都是「已发送」。线路真挂掉那天
/// 才发现收不到，就晚了。
///
/// 所以这里发一封真的：走和真告警**完全同一条路**（同一个收件人清单、同一个发信通道），
/// 只是内容写明是测试。它逐个报告每个地址成没成功，失败原因原样带出来。
pub async fn test_alarm(
    axum::extract::State(state): axum::extract::State<AppState>,
    claims: crate::auth::Claims,
) -> crate::error::ApiResult<axum::Json<serde_json::Value>> {
    if claims.role != "admin" {
        return Err(crate::error::AppError::forbidden("需要管理员权限"));
    }
    if !state.cfg.mail_enabled() {
        return Err(crate::error::AppError::bad(
            "邮件没配置（brevo_api_key / mail_from 为空），任何告警都发不出去",
        ));
    }
    let all = sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE role = 'admin'")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    let (good, bad): (Vec<_>, Vec<_>) = all.into_iter().partition(|e| looks_like_email(e));
    if good.is_empty() {
        return Err(crate::error::AppError::bad(
            "没有一个 admin 账号的 email 字段是邮箱地址 —— 线路挂了不会有任何人收到通知",
        ));
    }

    let mut results = Vec::new();
    for addr in &good {
        let r = crate::email::send_mail(
            &state.cfg,
            addr,
            "[测试] Mr. Day One 线路告警自检",
            "这是一封测试信，用来确认线路告警发得到你这儿。\n\n             收到了就说明真出问题时你也会收到。没收到的话先翻垃圾箱；\n             还是没有的话，是发件域在这家邮箱那边没过，得去配 SPF/DKIM。",
            false,
        )
        .await;
        results.push(serde_json::json!({
            "to": addr,
            "ok": r.is_ok(),
            "error": r.err().map(|e| e.msg),
        }));
    }
    Ok(axum::Json(serde_json::json!({
        "sent": results,
        // 填了用户名而不是邮箱的那些：它们永远收不到，得让人看见。
        "skipped": bad.len(),
    })))
}

/// 判定一条线路要不要发告警 / 恢复通知。状态存 Redis，不存进程内存。/// 判定一条线路要不要发告警 / 恢复通知。状态存 Redis，不存进程内存。
///
/// 进程内存在这里是错的：发一次版就清零，而蓝绿切换时新旧两版还会各记各的 ——
/// 一次部署就能把「已经坏了 30 分钟」重置成「刚刚开始坏」，告警永远攒不满。
async fn evaluate_alarm(state: &AppState, route_id: Uuid, label: &str, word: &str, h: &RouteHealth) {
    let mut conn = state.redis.clone();
    let since_key = key(route_id, "alarm_since");
    let now = now_secs();

    if word != "error" {
        // 恢复了：只有真发过通知才补一封「恢复」，否则一次短暂抖动会产生一封莫名其妙的邮件。
        let had: Option<i64> = redis::cmd("GET").arg(&since_key).query_async(&mut conn).await.unwrap_or(None);
        if had.is_some() {
            let sent: Option<i64> = redis::cmd("GET")
                .arg(key(route_id, "alarm_sent"))
                .query_async(&mut conn)
                .await
                .unwrap_or(None);
            let _: Result<(), _> = redis::cmd("DEL")
                .arg(&since_key)
                .arg(key(route_id, "alarm_sent"))
                .query_async(&mut conn)
                .await;
            if sent.is_some() {
                let _ = notify(
                    state,
                    &format!("[恢复] 线路「{label}」又能用了"),
                    &format!("线路：{label}\n当前判定：{word}\n连败计数已清零。"),
                )
                .await;
            }
        }
        return;
    }

    // 第一次判坏：记下起点，先不发 —— 给抖动留 ALARM_AFTER_SECS 的自愈时间。
    // NX 让「起点」只被写一次：后续每一轮都读回同一个值，所以「已经坏了多久」是连续的，
    // 不会被每轮刷新重置成 0（那样告警永远攒不满 5 分钟，一封都发不出去）。
    let claimed_start: Option<String> = redis::cmd("SET")
        .arg(&since_key)
        .arg(now)
        .arg("NX")
        .arg("EX")
        .arg(KEY_TTL_SECS)
        .query_async(&mut conn)
        .await
        .unwrap_or(None);
    let started = if claimed_start.is_some() {
        now
    } else {
        redis::cmd("GET")
            .arg(&since_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(None)
            .unwrap_or(now)
    };
    if now.saturating_sub(started) < ALARM_AFTER_SECS {
        return;
    }

    // 抢占发送权。SET NX 是原子的，蓝绿重叠那几十秒里两个进程只有一个抢得到，
    // 所以不会两边各发一封；冷却期同时由这把锁的 TTL 表达。
    let claimed: Option<String> = redis::cmd("SET")
        .arg(key(route_id, "alarm_sent"))
        .arg(now)
        .arg("NX")
        .arg("EX")
        .arg(ALARM_COOLDOWN_SECS)
        .query_async(&mut conn)
        .await
        .unwrap_or(None);
    if claimed.is_none() {
        return;
    }

    let last_ok = h
        .last_ok_at
        .map(|t| format!("{:.1} 小时前", (now - t) as f64 / 3600.0))
        .unwrap_or_else(|| "有记录以来从未成功".into());
    let delivered = notify(
        state,
        &format!("[告警] 线路「{label}」判定为不可用"),
        &format!(
            "线路：{label}\n\
             判定：error（连续 {} 次非成功）\n\
             上次成功：{last_ok}\n\
             上次失败状态码：{}\n\
             已持续：{} 分钟\n\n\
             判据是真实流量的结局（连败次数 + 上次成功时刻），不是探针 —— 探针只测上游前门，\n\
             十条线路共用同一个域名，它测不出这条线路能不能用。",
            h.consecutive_failures,
            h.last_fail_status.map(|s| s.to_string()).unwrap_or_else(|| "—".into()),
            (now - started) / 60,
        ),
    )
    .await;

    // 一封都没发出去 → **把发送权还回去**，让下一轮再试。
    //
    // 原来是「先抢占再发」，发失败了冷却照样挂满 6 小时 —— 于是一次投递故障就能让这条
    // 线路静音一整个冷却期，而运维那边什么都收不到。那正是这套东西要修的形态
    //（「看起来有告警、其实一封都发不出」），不能在告警自己身上再造一遍。
    if !delivered {
        let _: Result<(), _> = redis::cmd("DEL")
            .arg(key(route_id, "alarm_sent"))
            .query_async(&mut conn)
            .await;
        tracing::error!(route = label, "线路告警一封都没送达，已释放冷却，下一轮重试");
    }
}

/// 后台任务：给没有证据的线路补一次真实探测，然后评估告警。
///
/// **单独一个任务，不挂在 health.rs 的探针 tick 上。** 那个循环是串行的、每条线路
/// 10 秒超时，一轮最坏 100 秒；把探测和告警叠上去会让两件事互相拖延，而告警恰恰是
/// 最不能被拖的那个。
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        // 起步先让服务把自己启动完，也避开部署瞬间那段必然「没有证据」的窗口。
        tokio::time::sleep(Duration::from_secs(90)).await;

        if state.cfg.mail_enabled() && alarm_recipients(&state).await.is_empty() {
            tracing::error!(
                "线路告警没有收件人：邮件已配置但没有 role='admin' 的用户。\
                 现在的状态是「看起来有告警，实际一封都发不出去」——比没有告警更危险。"
            );
        }

        let mut tick = tokio::time::interval(CANARY_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            let routes = match sqlx::query_as::<_, crate::models::Model>(
                "SELECT * FROM models WHERE active = true ORDER BY sort, created_at",
            )
            .fetch_all(&state.db)
            .await
            {
                Ok(r) => r,
                Err(err) => {
                    tracing::warn!(%err, "线路健康巡检：读线路失败，这一轮跳过");
                    continue;
                }
            };

            let mut probed = 0usize;
            for m in &routes {
                let h = snapshot(&state, m.id).await;
                let fresh = h
                    .last_attempt_at
                    .is_some_and(|t| now_secs().saturating_sub(t) < CANARY_SKIP_IF_FRESH_SECS);

                // 有新鲜的真实流量就不探 —— 那是免费且更真实的证据。
                if !fresh && canary_enabled() && probed < CANARY_MAX_PER_ROUND {
                    probed += 1;
                    match canary_once(m).await {
                        Some((ok, status)) => {
                            if ok {
                                record_ok(&state, m.id).await;
                            } else {
                                record_fail(&state, m.id, status).await;
                            }
                            tracing::info!(route = %m.label, ok, status, "线路探活（最小真实请求）");
                            // 探活的结果不用在这儿回读：下面的 best_word 会重新取一次
                            // 快照（它还要同时看这条线路挂的多路由出口）。
                        }
                        // 无从探起（这条线路一个模型都没开）——什么都不记。
                        // 记成功就是伪造证据，记失败就是诬告一条没被用到的线路。
                        None => tracing::warn!(
                            route = %m.label,
                            "线路探活跳过：这条线路没有任何可用模型 id，本轮不产生证据"
                        ),
                    }
                }

                // 告警看的是「这条线路还能不能服务」，所以要把它挂的多路由出口一起算进来。
                //
                // 健康是按出口记的（一个坏出口不该拖垮同线路的好出口），而流量大多走最便宜
                // 那个出口 —— 只看线路自带地址的记录，出口连败就永远进不了告警。那正是这次
                // 事故的形状：面板全绿、监控一次没响、44 小时。
                //
                // 取所有出口里**最好**的结论：还有一个能服务就不该报警，全坏了才是真坏了。
                let (word, which, h) =
                    crate::route_endpoints::best_word(&state, m.id, now_secs()).await;
                // 指名道姓：收到「线路 X 坏了」却发现直连是好的，下一次就没人看告警了。
                let label = match which {
                    Some(ep) => format!("{}（出口 {})", m.label, &ep.to_string()[..8]),
                    None => m.label.clone(),
                };
                evaluate_alarm(&state, m.id, &label, word, &h).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    /// 这一组的验收标准就是那次真实事故：强力版 44 小时零成功、34 次尝试。
    #[test]
    fn the_incident_route_is_called_broken() {
        // 34 次尝试、一次都没成功过 —— 无论连败阈值定多少都必须判坏。
        let never_worked = RouteHealth {
            consecutive_failures: 34,
            last_ok_at: None,
            last_attempt_at: Some(NOW - 60),
            last_fail_status: Some(504),
        };
        assert_eq!(classify(&never_worked, NOW), "error");

        // 更早的形态：它曾经好过（44 小时前），之后一路失败。
        let died_after_working = RouteHealth {
            consecutive_failures: 34,
            last_ok_at: Some(NOW - 44 * 3600),
            last_attempt_at: Some(NOW - 60),
            last_fail_status: Some(504),
        };
        assert_eq!(classify(&died_after_working, NOW), "error");

        // 关键：第 5 次就要报，不能等到攒够统计样本。
        let just_broke = RouteHealth {
            consecutive_failures: FAILING_STREAK,
            last_ok_at: Some(NOW - 120),
            last_attempt_at: Some(NOW - 5),
            last_fail_status: Some(502),
        };
        assert_eq!(
            classify(&just_broke, NOW),
            "error",
            "连败达标就该报，哪怕两分钟前还成功过 —— 这正是低流量下唯一有界的判据",
        );
    }

    /// **没有证据不许报绿。** 这是整套东西存在的理由：探针的病就是拿「敲得通前门」
    /// 冒充「模型能用」。
    #[test]
    fn absence_of_evidence_is_never_green() {
        assert_eq!(classify(&RouteHealth::default(), NOW), "unknown", "从没被碰过 ≠ 健康");

        // 曾经成功，但已经很久没有新证据了 —— 只是没人用，不能继续挂绿灯。
        let stale = RouteHealth {
            consecutive_failures: 0,
            last_ok_at: Some(NOW - OK_FRESH_SECS - 1),
            last_attempt_at: Some(NOW - OK_FRESH_SECS - 1),
            last_fail_status: None,
        };
        assert_eq!(classify(&stale, NOW), "unknown");
    }

    /// 判定顺序：先判坏、再判好。反过来的话小样本全失败会被中性结论吞掉。
    #[test]
    fn bad_news_is_evaluated_before_good_news() {
        // 刚成功过，但紧接着连败达标 → 仍然是坏。
        let fresh_ok_then_broke = RouteHealth {
            consecutive_failures: FAILING_STREAK + 3,
            last_ok_at: Some(NOW - 1),
            last_attempt_at: Some(NOW),
            last_fail_status: Some(503),
        };
        assert_eq!(classify(&fresh_ok_then_broke, NOW), "error");

        // 少量失败 + 新鲜成功 → 降级，不是绿。
        let flaky = RouteHealth {
            consecutive_failures: 1,
            last_ok_at: Some(NOW - 30),
            last_attempt_at: Some(NOW),
            last_fail_status: Some(502),
        };
        assert_eq!(classify(&flaky, NOW), "degraded");

        // 干净且新鲜 → 绿。这是唯一一条通往绿灯的路。
        let healthy = RouteHealth {
            consecutive_failures: 0,
            last_ok_at: Some(NOW - 30),
            last_attempt_at: Some(NOW - 30),
            last_fail_status: None,
        };
        assert_eq!(classify(&healthy, NOW), "ok");
    }

    /// 状态词必须落在 health.rs 前端已经认识的那四个里 —— 多一个就是一颗空药丸。
    #[test]
    fn only_the_four_words_the_frontend_knows() {
        let allowed = ["ok", "degraded", "error", "unknown"];
        let cases = [
            RouteHealth::default(),
            RouteHealth { consecutive_failures: 99, last_ok_at: None, last_attempt_at: Some(NOW), last_fail_status: Some(500) },
            RouteHealth { consecutive_failures: 0, last_ok_at: Some(NOW), last_attempt_at: Some(NOW), last_fail_status: None },
            RouteHealth { consecutive_failures: 2, last_ok_at: Some(NOW - 99_999), last_attempt_at: Some(NOW), last_fail_status: Some(429) },
            RouteHealth { consecutive_failures: 1, last_ok_at: Some(NOW - 10), last_attempt_at: Some(NOW), last_fail_status: Some(502) },
        ];
        for c in cases {
            let w = classify(&c, NOW);
            assert!(allowed.contains(&w), "冒出了前端不认识的状态词：{w}");
        }
    }

    /// 金丝雀**必须按线路协议分支**。
    ///
    /// 照 model_probe.rs 那样只发 OpenAI 形状的话，所有 anthropic 线路（Claude 一族、
    /// 免费智普、Kimi）都会探测失败 —— 而假红比假绿更糟：它把好线路报成坏的，
    /// 运维几次之后就把告警静音，下一次真事故照样没人看。
    #[test]
    fn the_canary_speaks_both_protocols() {
        let src = include_str!("route_health.rs");
        let body = src
            .split("async fn canary_once")
            .nth(1)
            .and_then(|s| s.split("\n}").next())
            .expect("canary_once 不见了");
        assert!(body.contains("anthropic"), "没有按协议分支");
        assert!(body.contains("x-api-key") && body.contains("anthropic-version"),
            "anthropic 分支缺鉴权头，那条路上的线路会被全部误判成坏的");
        assert!(body.contains("/messages") && body.contains("/chat/completions"),
            "两种协议的端点必须各走各的");
        // 最小请求：只问「接不接得通」，不让它真去生成。
        assert!(body.contains("\"max_tokens\": 1"), "探测请求不是最小的，会白烧 token");
    }



    /// 花钱的东西必须能关，而且默认不该在忙碌线路上花。
    #[test]
    fn the_canary_is_cheap_by_construction() {
        assert!(
            CANARY_SKIP_IF_FRESH_SECS > 0,
            "没有「有新鲜证据就跳过」的话，忙碌线路也会被白探一遍",
        );
        assert!(CANARY_MAX_PER_ROUND <= 8, "一轮探太多，线路变多时会一次烧穿");
        assert!(
            CANARY_TIMEOUT < Duration::from_secs(60),
            "探活只问接不接得通，不该等模型思考",
        );
        // 开关存在，且默认开（用户要的是覆盖零流量线路）。
        let src = include_str!("route_health.rs");
        assert!(src.contains("ROUTE_CANARY"), "没有关掉它的开关，而它花的是真钱");
    }

    /// 告警的两个时间常数必须站得住：够久到不被抖动触发，够短到还有意义。
    #[test]
    fn alarm_timing_is_neither_jumpy_nor_useless() {
        assert!(
            ALARM_AFTER_SECS >= 60,
            "太短的话一次部署空窗就会发一封，几次之后告警就被静音——那正是这次事故没人看的成因",
        );
        assert!(
            ALARM_AFTER_SECS <= 30 * 60,
            "太长的话它救不了这次事故（44 小时里前半小时就该有人知道）",
        );
        assert!(
            ALARM_COOLDOWN_SECS > ALARM_AFTER_SECS,
            "冷却必须长于判定时长，否则同一次故障会连着发",
        );
        // 巡检间隔要能在判定时长内至少跑到两轮，否则「持续 5 分钟」这句话没有测量精度。
        assert!(
            CANARY_EVERY.as_secs() as i64 <= ALARM_AFTER_SECS * 3,
            "巡检太稀疏，「已经坏了多久」量不准",
        );
    }

    /// 「坏了多久」的起点必须只写一次。
    ///
    /// 每轮都刷新起点的话，`now - started` 永远接近 0，5 分钟的门槛**永远攒不满**，
    /// 一封都发不出去 —— 而那正是这次事故的形态，不能用另一种方式复制。
    #[test]
    fn the_alarm_clock_is_not_reset_every_round() {
        let src = include_str!("route_health.rs");
        let body = src
            .split("async fn evaluate_alarm")
            .nth(1)
            .expect("evaluate_alarm 不见了");
        let head = &body[..body.find("fn ").unwrap_or(body.len().min(4000))];
        assert!(
            head.contains("alarm_since"),
            "起点没有落到持久存储上；存进程内存的话，发一次版就清零、蓝绿两版还各记各的",
        );
        assert!(head.contains("\"NX\""), "起点不是用 NX 写的，会被每轮刷新重置");
    }

    /// 「没东西可探」绝不能被记成一次成功。
    ///
    /// 第一版在这里返回 `(true, 0)`，调用方看到 ok=true 就 record_ok —— 凭空造一次成功、
    /// 把连败清零、点亮绿灯。而 `enabled_models` 为空的线路**照样在接真实流量**
    /// （派单的 allowed_ids 会回落到 model_id），所以它可以是真坏的。
    /// 这正是这套监控要消灭的东西（没有证据不许报绿），当时在它自己身上重演了一遍。
    #[test]
    fn nothing_to_probe_must_not_be_recorded_as_success() {
        let src = include_str!("route_health.rs");
        let body = src
            .split("async fn canary_once")
            .nth(1)
            .and_then(|s| s.split("\n// ").next())
            .expect("canary_once 不见了");

        assert!(
            body.contains("-> Option<(bool, u16)>"),
            "返回类型必须能表达「这一次没有证据」，否则调用方只能在成功和失败里二选一",
        );
        assert!(
            !body.contains("return (true,"),
            "又把「无从探起」当成功返回了——那是伪造证据",
        );
        // 探的模型必须和派单口径一致，否则会漏掉 enabled_models 为空、但正在接流量的线路。
        assert!(
            body.contains("allowed_ids"),
            "探测用的模型 id 和派单不是同一个口径",
        );

        // 调用方必须对 None 什么都不记。
        let loop_src = src
            .split("pub fn spawn(")
            .nth(1)
            .expect("spawn 不见了");
        assert!(
            loop_src.contains("None =>") && !loop_src.contains("None => record_ok"),
            "调用方没有为「没有证据」留一条什么都不做的分支",
        );
    }

    /// 卡死线路的恢复判定由后台探针接管，用户的真实请求不再当探针。
    ///
    /// 节奏必须钉住：探测间隔短于记号有效期，否则记号在两次探测之间过期、线路回到排头、
    /// 下一个用户又去撞满 57 秒 —— 正是这条要消灭的形态。
    #[test]
    fn stall_recovery_probes_faster_than_the_mark_expires() {
        assert!(
            STALL_RECOVERY_EVERY * 2 <= crate::models::CHAT_UPSTREAM_STALL_MEMORY,
            "探测间隔 {STALL_RECOVERY_EVERY:?} 太稀疏，记号会在两次探测之间过期",
        );
        assert!(STALL_RECOVERY_EVERY >= Duration::from_secs(10), "探得太密，停机期间白烧钱");
        assert!(
            CANARY_TIMEOUT <= STALL_RECOVERY_EVERY,
            "单次探测耐心超过间隔，探针会自己叠自己",
        );
        assert!(STALL_RECOVERY_MAX_CONCURRENT >= 1 && STALL_RECOVERY_MAX_CONCURRENT <= 8);
    }

    /// 同一条线路只许一个任务，总数有上限，名额用完能还。
    #[test]
    fn stall_recovery_admission_is_bounded() {
        // 别的测试也可能占着名额：先把这组用到的 id 清干净，结束时再还回去。
        let ids: Vec<Uuid> = (0..STALL_RECOVERY_MAX_CONCURRENT + 1).map(|_| Uuid::new_v4()).collect();
        let baseline = STALL_RECOVERY_ACTIVE.lock().map(|a| a.len()).unwrap_or(0);
        let room = STALL_RECOVERY_MAX_CONCURRENT.saturating_sub(baseline);
        if room == 0 {
            // 并发跑的别的用例把名额占满了，本用例的判定没意义；只验证拒绝。
            assert!(!stall_recovery_admit(ids[0]));
            return;
        }
        assert!(stall_recovery_admit(ids[0]), "空闲时必须放行");
        assert!(!stall_recovery_admit(ids[0]), "同一条线路第二次必须拒绝 —— 一条线一个任务");
        for id in ids.iter().skip(1).take(room - 1) {
            assert!(stall_recovery_admit(*id));
        }
        assert!(
            !stall_recovery_admit(ids[room]),
            "超过 {STALL_RECOVERY_MAX_CONCURRENT} 个并发任务必须拒绝",
        );
        stall_recovery_release(ids[0]);
        assert!(stall_recovery_admit(ids[room]), "释放后名额要能再用");
        for id in ids.iter().take(room + 1) {
            stall_recovery_release(*id);
        }
        assert!(!STALL_RECOVERY_ACTIVE.lock().unwrap().contains(&ids[0]));
    }

    /// 任务的结构必须是：只对有记号的线路跑、恢复即停、失败续记号、受开关约束、None 不记账。
    ///
    /// 钉的是实现特征（调用点），不是文案。需要的串拼出来找，避免本测试自己喂绿自己。
    #[test]
    fn stall_recovery_task_stops_when_the_mark_is_gone_and_refreshes_it_on_failure() {
        let src = include_str!("route_health.rs");
        let body = src
            .split("pub fn spawn_stall_recovery(")
            .nth(1)
            .and_then(|s| s.split("\n}\n").next())
            .expect("spawn_stall_recovery 不见了");
        let stalled_read = format!("{}(m.health_id(), Instant::now())", "route_recently_stalled");
        assert!(
            body.contains(&format!("if !crate::models::{stalled_read}")),
            "任务没有在每轮前检查记号是否还在 —— 线路恢复后探针不会停",
        );
        assert!(
            body.contains(&format!("{}(m.health_id())", "clear_route_stall"))
                && body.contains(&format!("{}(m.health_id())", "clear_route_cooldown")),
            "探通之后没有撤记号/撤冷却，线路回不到排头",
        );
        assert!(
            body.contains(&format!("{}(m.health_id())", "mark_route_stall")),
            "失败没有续记号 —— 120 秒后记号过期，用户又成了探针",
        );
        assert!(body.contains("canary_enabled()"), "恢复探针花的是真钱，必须受 ROUTE_CANARY 约束");
        assert!(body.contains("stall_recovery_admit(m.health_id())"), "没有并发上限");
        assert!(
            body.contains("None =>") && !body.contains("None => record_ok"),
            "「无从探起」必须什么都不记",
        );
        // record_ok 只能出现在探针成功那一支里。
        let ok_arm = body.split("Some((true,").nth(1).and_then(|s| s.split("Some((false,").next()).unwrap_or("");
        assert!(ok_arm.contains("record_ok("), "探通了却不记成功，面板看不到恢复");
        let fail_arm = body.split("Some((false,").nth(1).unwrap_or("");
        assert!(!fail_arm.contains("record_ok("), "失败支里记了成功");
        // 派单路径必须真的起它 —— 写好了零调用点是这个仓库反复出现的失败模式。
        let models_src = include_str!("models.rs");
        let stall_site = models_src
            .split(&format!("{}(candidate.health_id());", "mark_route_stall"))
            .nth(1)
            .expect("派单路径上的 mark_route_stall 不见了");
        let after = &stall_site[..stall_site.len().min(600)];
        assert!(
            after.contains("spawn_stall_recovery(&state, candidate.clone())"),
            "卡死之后没有起恢复探针，恢复判定仍由用户的真实请求付费",
        );
    }

    /// `users.email` 并不保证是邮箱 —— 线上有个 admin 的值是用户名。
    #[test]
    fn only_real_addresses_are_treated_as_recipients() {
        assert!(looks_like_email("ops@example.com"));
        assert!(looks_like_email("a.b+tag@mail.co.uk"));
        // 线上实测的那一个：14 个字符、没有 @。往它发就是每轮白打一次必然失败的请求，
        // 而真正的发送失败会被埋在这堆噪声里。
        assert!(!looks_like_email("fendoushaonian"));
        assert!(!looks_like_email(""));
        assert!(!looks_like_email("@example.com"));
        assert!(!looks_like_email("ops@localhost"));
        assert!(!looks_like_email("ops@ example.com"));
        assert!(!looks_like_email("a@b@c.com"));
    }

    /// 发送失败必须把冷却还回去。
    ///
    /// 原来是「先抢占再发」：一次投递故障就让这条线路静音整整 6 小时，而运维什么都收不到
    /// —— 正是这套东西要修的那个形态，不能在告警自己身上再造一遍。
    #[test]
    fn a_failed_send_releases_the_cooldown() {
        let src = include_str!("route_health.rs");
        let body = src
            .split("async fn evaluate_alarm")
            .nth(1)
            .expect("evaluate_alarm 不见了");
        assert!(
            body.contains("if !delivered {"),
            "没有「一封都没送达就释放冷却」的分支",
        );
        let release = body.split("if !delivered {").nth(1).unwrap_or("");
        assert!(
            release.contains("DEL") && release.contains("alarm_sent"),
            "释放分支没有真的把发送权删掉，冷却仍然会挂满",
        );
        // notify 必须回报结果，否则上面那个判断永远拿不到真相。
        assert!(
            src.contains("async fn notify(state: &AppState, subject: &str, body: &str) -> bool"),
            "notify 不回报是否送达，调用方只能假设成功",
        );
    }

    /// 阈值本身要站得住：3 连败在 16–20% 的常态失败率下会误报，5 连败不会。
    #[test]
    fn the_streak_threshold_survives_the_background_failure_rate() {
        let background = 0.20_f64;
        let false_alarm = background.powi(FAILING_STREAK as i32);
        assert!(
            false_alarm < 0.001,
            "连败阈值 {FAILING_STREAK} 在 20% 的常态失败率下误报概率 {false_alarm:.4}，太高了",
        );
        assert!(FAILING_STREAK >= 3, "太小的话一次偶发抖动就报警，几次之后告警就被静音");
    }
}
