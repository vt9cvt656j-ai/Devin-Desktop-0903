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

use std::time::{SystemTime, UNIX_EPOCH};

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
