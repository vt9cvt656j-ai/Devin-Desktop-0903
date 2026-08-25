//! 对账：**每个中转站收我们多少，我们从用户身上收回来多少，差额是正是负。**
//!
//! # 两侧的数从哪来
//!
//! 收入那一侧是现成的：`endpoint_usage.cost_micro_usd` 记的就是「按这个出口发出去的
//! 请求，一共扣了用户多少钱」，逐笔累加，和计费同源。
//!
//! 成本那一侧没有任何接口能直接回答。这里给两个数，**它们互为校验**：
//!
//! · **估算成本** = 收入 × `cost_ratio / rate`。这个恒等式成立是因为两边同底：
//!   用户被扣的是 `tokens × 官方价 × rate`，我们付上游的是 `tokens × 官方价 × cost_ratio`，
//!   官方价和 tokens 一约就没了。好处是**部署当天就有数**，不用等快照攒够。
//!   它的前提是运维把 `cost_ratio` 填对了 —— 填错就只是把一个错误的假设算得很精确。
//!
//! · **实测成本** = 中转账户余额（或「已用」）在这段时间里的变化量。这是真金白银，
//!   不依赖任何人填对什么。代价是要等两次快照，而且有些中转根本不给余额接口。
//!
//! 两个数差得远，说明 `cost_ratio` 填错了，或者上游在按和你以为的不同的价目表收费。
//! 这正是对账要抓的东西 —— 所以两个都显示，不合并成一个。
//!
//! # 为什么优先用「已用」而不是「余额」
//!
//! 余额会被充值打断：期间充了 100 刀，`first - last` 会算出一个负成本，然后面板显示
//! 这条线路在给你送钱。「已用」是单调递增的，充值不影响它。只有在这家不给「已用」时
//! 才退回余额，并且**一旦发现余额上升就整段作废**，而不是把负数当成利润报上去。

use axum::extract::{Query, State};
use axum::Json;
use std::collections::HashMap;
use std::time::Duration;

use crate::auth::Claims;
use crate::error::{AppError, ApiResult};
use crate::models::Model;
use crate::AppState;

/// 多久拍一次余额快照。
///
/// 30 分钟：对账看的是天/周级别的差额，再密只是多打网络往返；再疏则「今天花了多少」
/// 会只剩一两个采样点，一次网络抖动就让当天没有数。
const SNAPSHOT_EVERY: Duration = Duration::from_secs(30 * 60);
/// 查一次余额的耐心。三种形态各试一次，所以给得比单次请求宽一点。
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(20);

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

// ---------------------------------------------------------------- 快照

/// 拍一轮：每条线路自带地址 + 它挂的每个出口，各存一行。
pub async fn snapshot_once(state: &AppState) {
    let Ok(http) = reqwest::Client::builder().timeout(SNAPSHOT_TIMEOUT).build() else {
        return;
    };
    let routes: Vec<Model> =
        match sqlx::query_as("SELECT * FROM models WHERE active = true ORDER BY sort, created_at")
            .fetch_all(&state.db)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "余额快照：线路读不出来，本轮跳过");
                return;
            }
        };
    let eps = crate::route_endpoints::load_for_routes(
        &state.db,
        &routes.iter().map(|m| m.id).collect::<Vec<_>>(),
    )
    .await;

    for r in &routes {
        let mut targets: Vec<(uuid::Uuid, String, String)> =
            vec![(r.id, r.base_url.clone(), r.api_key.clone())];
        for e in eps.get(&r.id).into_iter().flatten().filter(|e| e.active) {
            let key = if e.api_key.trim().is_empty() { r.api_key.clone() } else { e.api_key.clone() };
            targets.push((e.id, e.base_url.clone(), key));
        }
        for (id, base, key) in targets {
            if base.trim().is_empty() || key.trim().is_empty() {
                continue;
            }
            let Some(b) = crate::route_endpoints::read_balance(
                &http,
                &base,
                &crate::models::model_key(&key),
            )
            .await
            else {
                // 查不到就**不写行**。写一行 NULL 会让「这家没有余额接口」和
                // 「这次网络抖了一下」长得一模一样，而前者不该每半小时占一行。
                continue;
            };
            let _ = sqlx::query(
                "INSERT INTO endpoint_balance (endpoint_id, route_id, remaining_usd, used_usd, raw) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(r.id)
            .bind(b.remaining_usd)
            .bind(b.used_usd)
            .bind(&b.text)
            .execute(&state.db)
            .await;
        }
    }
}

/// 起后台任务。
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        // 开机就拍一张：否则重启之后要等半小时才有第一个采样点，
        // 而重启往往正发生在「出事了在查」的时候。
        tokio::time::sleep(Duration::from_secs(45)).await;
        loop {
            snapshot_once(&state).await;
            tokio::time::sleep(SNAPSHOT_EVERY).await;
        }
    });
}

// ---------------------------------------------------------------- 对账

#[derive(sqlx::FromRow)]
struct UsageAgg {
    endpoint_id: uuid::Uuid,
    calls: i64,
    cost_micro: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    cached_tokens: i64,
}

#[derive(sqlx::FromRow)]
struct BalanceEdge {
    endpoint_id: uuid::Uuid,
    first_remaining: Option<f64>,
    last_remaining: Option<f64>,
    first_used: Option<f64>,
    last_used: Option<f64>,
    samples: i64,
}

#[derive(serde::Serialize)]
pub struct ReconRow {
    pub endpoint_id: uuid::Uuid,
    pub route_id: uuid::Uuid,
    pub route_label: String,
    pub label: String,
    pub vendor: &'static str,
    pub is_own: bool,
    pub active: bool,
    pub cost_ratio: f64,
    pub rate: f64,
    pub calls: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    /// 从用户身上收到的（美元）。
    pub revenue_usd: f64,
    /// 按 cost_ratio/rate 推出来的成本。`rate <= 0` 时为 None —— 除不了，不猜。
    pub cost_est_usd: Option<f64>,
    /// 余额/已用实际掉了多少。None = 采样不足、或期间充过值。
    pub cost_real_usd: Option<f64>,
    /// 实测值是按哪个口径算的："used" / "remaining"。None = 没有实测值。
    pub cost_real_basis: Option<&'static str>,
    /// 有几个余额采样点。1 或 0 就算不出差额。
    pub balance_samples: i64,
    /// 毛利 = 收入 − 成本（**优先用实测成本**，没有才退回估算）。
    pub margin_usd: Option<f64>,
    pub margin_pct: Option<f64>,
    /// 成本口径。"real" / "est" / None。
    pub margin_basis: Option<&'static str>,
    /// 为什么没有实测成本 —— 空 = 有。
    pub note: String,
}

#[derive(serde::Deserialize)]
pub struct ReconQuery {
    /// 看最近几天。默认 7。
    #[serde(default)]
    pub days: Option<i64>,
}

/// `GET /api/admin/reconciliation?days=7`
///
/// 一行一个出口：收了多少、花了多少、差额多少。
pub async fn admin_reconciliation(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<ReconQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    // 上下界都钳住：0 天什么都查不出来，而不限上界会让一次点击扫全表。
    let days = q.days.unwrap_or(7).clamp(1, 90);

    let routes: Vec<Model> =
        sqlx::query_as("SELECT * FROM models WHERE active = true ORDER BY sort, created_at")
            .fetch_all(&state.db)
            .await?;
    let eps = crate::route_endpoints::load_for_routes(
        &state.db,
        &routes.iter().map(|m| m.id).collect::<Vec<_>>(),
    )
    .await;

    let usage: Vec<UsageAgg> = sqlx::query_as(
        "SELECT endpoint_id, \
                SUM(calls)::bigint             AS calls, \
                SUM(cost_micro_usd)::bigint    AS cost_micro, \
                SUM(prompt_tokens)::bigint     AS prompt_tokens, \
                SUM(completion_tokens)::bigint AS completion_tokens, \
                SUM(cached_tokens)::bigint     AS cached_tokens \
         FROM endpoint_usage \
         WHERE day > current_date - $1::int \
         GROUP BY endpoint_id",
    )
    .bind(days as i32)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let usage: HashMap<uuid::Uuid, UsageAgg> =
        usage.into_iter().map(|u| (u.endpoint_id, u)).collect();

    // 每个出口在窗口内的第一条和最后一条读数。
    //
    // 用 DISTINCT ON 而不是把整段拉回来在内存里算：一个出口一天 48 行、七天 336 行、
    // 十个出口三千多行 —— 拉回来只为取首尾两条是纯浪费，而且行数还会随天数线性涨。
    let edges: Vec<BalanceEdge> = sqlx::query_as(
        "WITH w AS ( \
             SELECT * FROM endpoint_balance WHERE taken_at > now() - ($1::int * INTERVAL '1 day') \
         ), \
         f AS (SELECT DISTINCT ON (endpoint_id) endpoint_id, remaining_usd, used_usd \
               FROM w ORDER BY endpoint_id, taken_at ASC), \
         l AS (SELECT DISTINCT ON (endpoint_id) endpoint_id, remaining_usd, used_usd \
               FROM w ORDER BY endpoint_id, taken_at DESC), \
         c AS (SELECT endpoint_id, count(*)::bigint AS samples FROM w GROUP BY endpoint_id) \
         SELECT c.endpoint_id, \
                f.remaining_usd AS first_remaining, l.remaining_usd AS last_remaining, \
                f.used_usd      AS first_used,      l.used_usd      AS last_used, \
                c.samples \
         FROM c JOIN f ON f.endpoint_id = c.endpoint_id JOIN l ON l.endpoint_id = c.endpoint_id",
    )
    .bind(days as i32)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let edges: HashMap<uuid::Uuid, BalanceEdge> =
        edges.into_iter().map(|e| (e.endpoint_id, e)).collect();

    let mut rows: Vec<ReconRow> = Vec::new();
    for r in &routes {
        let vendor = crate::route_endpoints::vendor_of(
            &r.provider,
            &crate::models::allowed_ids(r),
            &r.base_url,
        );
        let mut targets: Vec<(uuid::Uuid, String, bool, f64, bool)> =
            vec![(r.id, format!("{}（自带地址）", r.label), true, 1.0, true)];
        for e in eps.get(&r.id).into_iter().flatten() {
            let label = if e.label.trim().is_empty() { "未命名出口".to_string() } else { e.label.clone() };
            targets.push((e.id, label, false, e.cost_ratio, e.active));
        }

        for (id, label, is_own, cost_ratio, active) in targets {
            let u = usage.get(&id);
            let revenue = u.map(|x| x.cost_micro as f64 / 1_000_000.0).unwrap_or(0.0);
            // 倍率是 0 或负数时除不出东西来。不猜，留空。
            let cost_est = (r.rate > 0.0).then(|| revenue * cost_ratio / r.rate);
            let (cost_real, basis, samples, note) = real_cost(edges.get(&id));
            let cost_used = cost_real.or(cost_est);
            let margin = cost_used.map(|c| revenue - c);

            rows.push(ReconRow {
                endpoint_id: id,
                route_id: r.id,
                route_label: r.label.clone(),
                label,
                vendor,
                is_own,
                active,
                cost_ratio,
                rate: r.rate,
                calls: u.map(|x| x.calls).unwrap_or(0),
                prompt_tokens: u.map(|x| x.prompt_tokens).unwrap_or(0),
                completion_tokens: u.map(|x| x.completion_tokens).unwrap_or(0),
                cached_tokens: u.map(|x| x.cached_tokens).unwrap_or(0),
                revenue_usd: revenue,
                cost_est_usd: cost_est,
                cost_real_usd: cost_real,
                cost_real_basis: basis,
                balance_samples: samples,
                margin_usd: margin,
                // 收入是 0 时毛利率没有意义（分母为零），不报 0% —— 那读起来像「打平」。
                margin_pct: margin.filter(|_| revenue > 0.0).map(|m| m / revenue * 100.0),
                margin_basis: cost_real.map(|_| "real").or(cost_est.map(|_| "est")),
                note,
            });
        }
    }

    // 亏得最狠的排最前：这一页是用来发现问题的，不是用来浏览的。
    // 没有毛利数的（还没攒够采样）沉到最后，而不是混在中间冒充打平。
    rows.sort_by(|a, b| match (a.margin_usd, b.margin_usd) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    let revenue_total: f64 = rows.iter().map(|r| r.revenue_usd).sum();
    // 合计只加**有成本数**的那些行，并把加了几行报出去 —— 否则总额会把
    // 「还没有数据的出口」当成零成本，合计毛利凭空变好看。
    let counted: Vec<&ReconRow> = rows.iter().filter(|r| r.margin_usd.is_some()).collect();
    let cost_total: f64 = counted
        .iter()
        .map(|r| r.cost_real_usd.or(r.cost_est_usd).unwrap_or(0.0))
        .sum();
    let counted_revenue: f64 = counted.iter().map(|r| r.revenue_usd).sum();

    Ok(Json(serde_json::json!({
        "days": days,
        "rows": rows,
        "totals": {
            "revenue_usd": revenue_total,
            "counted_revenue_usd": counted_revenue,
            "cost_usd": cost_total,
            "margin_usd": counted_revenue - cost_total,
            "counted_rows": counted.len(),
            "total_rows": rows.len(),
        },
    })))
}

/// 从首尾两条读数算实测成本。
///
/// 返回 (成本, 口径, 采样点数, 说明)。成本为 None 时说明里一定有一句话 ——
/// 「这里没有数字」和「为什么没有」必须一起给，否则面板上一片横杠没人知道该做什么。
fn real_cost(e: Option<&BalanceEdge>) -> (Option<f64>, Option<&'static str>, i64, String) {
    let Some(e) = e else {
        return (None, None, 0, "这个出口还没有余额读数（可能这家不提供余额接口）".into());
    };
    if e.samples < 2 {
        return (None, None, e.samples, "余额只采到一个点，算不出差额（再等半小时）".into());
    }
    // 优先「已用」：它单调递增，充值不打断。
    if let (Some(a), Some(b)) = (e.first_used, e.last_used) {
        let d = b - a;
        if d >= 0.0 {
            return (Some(d), Some("used"), e.samples, String::new());
        }
        // 已用回退：多半是换了密钥（换成另一个账号了），这段没法比。
        return (
            None,
            None,
            e.samples,
            "「已用」比上次小了 —— 多半是换了密钥，这段时间没法比".into(),
        );
    }
    if let (Some(a), Some(b)) = (e.first_remaining, e.last_remaining) {
        let d = a - b;
        if d >= 0.0 {
            return (Some(d), Some("remaining"), e.samples, String::new());
        }
        // 余额涨了 = 期间充过值。**不能报成负成本**，那会显示成中转在给你送钱。
        return (
            None,
            None,
            e.samples,
            "期间余额上升（充过值），这段的成本算不出来".into(),
        );
    }
    (None, None, e.samples, "这家只给了额度上限，既不是余额也不是已用".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(fr: Option<f64>, lr: Option<f64>, fu: Option<f64>, lu: Option<f64>, n: i64) -> BalanceEdge {
        BalanceEdge {
            endpoint_id: uuid::Uuid::nil(),
            first_remaining: fr,
            last_remaining: lr,
            first_used: fu,
            last_used: lu,
            samples: n,
        }
    }

    /// 充值不能变成负成本。
    ///
    /// 这是这张表最容易得出荒谬结论的地方：余额从 10 涨到 110（充了 100），
    /// `first - last` = -100，直接报上去就是「这个中转这周倒贴了你 100 刀」，
    /// 而且它会排在列表最前面（按毛利升序），把真正亏钱的那行挤下去。
    #[test]
    fn a_top_up_never_becomes_negative_cost() {
        let (cost, basis, _, note) = real_cost(Some(&edge(Some(10.0), Some(110.0), None, None, 5)));
        assert!(cost.is_none(), "充值被算成了负成本");
        assert!(basis.is_none());
        assert!(note.contains("充过值"), "没说清为什么没有数字：{note}");
    }

    /// 有「已用」时优先用它 —— 它不被充值打断。
    #[test]
    fn used_wins_over_remaining() {
        // 余额从 10 涨到 110（充值），但已用从 3 涨到 8 —— 真实成本是 5。
        let (cost, basis, _, _) =
            real_cost(Some(&edge(Some(10.0), Some(110.0), Some(3.0), Some(8.0), 5)));
        assert_eq!(basis, Some("used"), "有已用却退回去用余额了");
        assert!((cost.unwrap() - 5.0).abs() < 1e-9, "成本算错：{cost:?}");
    }

    /// 一个采样点算不出差额，而且必须说清楚。
    #[test]
    fn one_sample_is_not_a_delta() {
        let (cost, _, n, note) = real_cost(Some(&edge(Some(10.0), Some(10.0), None, None, 1)));
        assert!(cost.is_none());
        assert_eq!(n, 1);
        assert!(!note.is_empty(), "没有数字也没有原因，面板上就是一片横杠");
    }

    /// 完全没有读数 ≠ 成本为零。
    #[test]
    fn no_reading_is_not_zero_cost() {
        let (cost, _, n, note) = real_cost(None);
        assert!(cost.is_none(), "没有读数被当成了零成本 —— 那会让毛利凭空变好看");
        assert_eq!(n, 0);
        assert!(!note.is_empty());
    }

    /// 「只给上限」既不是余额也不是已用，不许拿来算。
    #[test]
    fn a_hard_limit_is_not_a_balance() {
        let (cost, basis, _, note) = real_cost(Some(&edge(None, None, None, None, 4)));
        assert!(cost.is_none() && basis.is_none());
        assert!(note.contains("上限"), "{note}");
    }

    /// 估算成本的恒等式：收入 × cost_ratio / rate。
    #[test]
    fn the_estimate_shares_a_base_with_revenue() {
        // 用户按官方价 ×5 收，我们按 ×0.5 付 → 成本应当是收入的十分之一。
        let revenue: f64 = 100.0;
        let (rate, cost_ratio): (f64, f64) = (5.0, 0.5);
        let est = revenue * cost_ratio / rate;
        assert!((est - 10.0).abs() < 1e-9, "估算恒等式算错：{est}");
        // 倍率为 0 时除不了 —— 调用方必须留空而不是产出 inf。
        let guarded = (0.0_f64 > 0.0).then(|| revenue * cost_ratio / 0.0);
        assert!(guarded.is_none(), "倍率为 0 时产出了一个无穷大的成本");
    }
}
