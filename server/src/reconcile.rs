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

    // 「问了几个」和「问到几个」都要报出来。
    //
    // 只在失败时打日志的话，「跑了一轮但一家都不给余额接口」和「这个任务压根没起来」
    // 在日志里长得一模一样 —— 而它们要做的事完全相反（前者去换个中转或手工记账，
    // 后者去查任务为什么没跑）。实测线上第一轮就是 0 行，当时无从判断是哪一种。
    let mut tried = 0usize;
    let mut got = 0usize;

    for r in &routes {
        // (出口 id, 地址, 调用密钥, 余额令牌)
        let mut targets: Vec<(uuid::Uuid, String, String, String)> =
            vec![(r.id, r.base_url.clone(), r.api_key.clone(), r.balance_token.clone())];
        for e in eps.get(&r.id).into_iter().flatten().filter(|e| e.active) {
            let key = if e.api_key.trim().is_empty() { r.api_key.clone() } else { e.api_key.clone() };
            // 出口没配令牌就用线路的 —— 同一个中转账号挂几个入口是常见配置。
            let btok = if e.balance_token.trim().is_empty() { r.balance_token.clone() } else { e.balance_token.clone() };
            targets.push((e.id, e.base_url.clone(), key, btok));
        }
        for (id, base, key, btok) in targets {
            // 两种凭据都空才跳过：只配了令牌没配密钥是合法的。
            if base.trim().is_empty() || (key.trim().is_empty() && btok.trim().is_empty()) {
                continue;
            }
            tried += 1;
            let Some(b) = crate::route_endpoints::read_balance(
                &http,
                &base,
                &crate::models::model_key(&key),
                &crate::models::model_key(&btok),
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
            got += 1;
        }
    }

    if got == 0 && tried > 0 {
        // 一个都问不到不是错误 —— 多数国内中转的 /api/user/self 认的是控制台登录令牌，
        // 不是 sk- 开头的调用密钥。但它必须说出来，否则对账页的「成本(实测)」会永远
        // 空着而没人知道为什么。
        tracing::info!(
            tried,
            "余额快照：问了 {tried} 个出口，没有一个给出可识别的余额 ——              对账页的「成本(实测)」会一直空着，只能用估算列"
        );
    } else {
        tracing::info!(tried, got, "余额快照完成");
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
struct ModelUsage {
    endpoint_id: uuid::Uuid,
    model_id: String,
    calls: i64,
    revenue_micro: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    cached_tokens: i64,
}

#[derive(sqlx::FromRow, Clone)]
pub struct ModelPrice {
    pub endpoint_id: uuid::Uuid,
    pub model_id: String,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cached_per_mtok: Option<f64>,
    pub note: String,
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

/// 一个出口上某个模型这段时间的账。
#[derive(serde::Serialize)]
pub struct ModelRow {
    pub model_id: String,
    pub calls: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    pub revenue_usd: f64,
    /// token × 录入单价。None = **这个模型的进价还没录**，不是 0。
    pub cost_usd: Option<f64>,
    pub margin_usd: Option<f64>,
    /// 当前录着的单价，回给界面直接填进输入框。
    pub input_per_mtok: Option<f64>,
    pub output_per_mtok: Option<f64>,
    pub cached_per_mtok: Option<f64>,
    pub price_note: String,
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
    pub calls: i64,
    pub revenue_usd: f64,
    /// 真实成本 = Σ(真实 token × 录入单价)。
    ///
    /// **只有这个出口用过的模型全都录了价才有值。** 少录一个就是 None ——
    /// 把没录价的那部分按 0 加进来，得到的是一个看起来很精确的错数字。
    pub cost_usd: Option<f64>,
    pub margin_usd: Option<f64>,
    pub margin_pct: Option<f64>,
    /// 用过但还没录价的模型。非空 = 上面三个数都是 None。
    pub unpriced_models: Vec<String>,
    /// 余额读数差出来的成本。和上面那个各自独立，用来**互相印证** ——
    /// 两个都有而且对不上，说明单价录错了或者中转在按另一份价目表收费。
    pub cost_by_balance_usd: Option<f64>,
    pub balance_basis: Option<&'static str>,
    pub balance_note: String,
    /// 按模型展开。界面点开一行就是它，也是录价的地方。
    pub models: Vec<ModelRow>,
}

#[derive(serde::Deserialize)]
pub struct ReconQuery {
    /// 看最近几天。默认 7。
    #[serde(default)]
    pub days: Option<i64>,
}

/// 一个模型这一段的真实成本。
///
/// # 缓存 token 必须先从输入里减出来
///
/// `prompt_tokens` 是**含**缓存命中的总输入，各家的 usage 帧都是这个口径。
/// 不减直接乘输入价，命中率高的模型成本会被高估好几倍 —— 而缓存价通常只有输入价
/// 的十分之一，正是它让「同样的对话第二轮便宜得多」这件事成立。
///
/// `cached_per_mtok` 没录时按输入价算：那等于「这家不给缓存折扣」，是**保守**的方向
/// （宁可把成本算高一点，也不要把毛利算得比实际好看）。
fn model_cost_usd(u: &ModelUsage, p: &ModelPrice) -> f64 {
    let cached = u.cached_tokens.max(0).min(u.prompt_tokens.max(0));
    let fresh = (u.prompt_tokens.max(0) - cached) as f64;
    let cached_price = p.cached_per_mtok.unwrap_or(p.input_per_mtok);
    (fresh * p.input_per_mtok
        + cached as f64 * cached_price
        + u.completion_tokens.max(0) as f64 * p.output_per_mtok)
        / 1_000_000.0
}

/// `GET /api/admin/reconciliation?days=7`
///
/// 一行一个出口：收了多少、真实花了多少、差额多少。**没有估算**。
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

    let usage: Vec<ModelUsage> = sqlx::query_as(
        "SELECT endpoint_id, model_id, \
                SUM(calls)::bigint             AS calls, \
                SUM(revenue_micro_usd)::bigint AS revenue_micro, \
                SUM(prompt_tokens)::bigint     AS prompt_tokens, \
                SUM(completion_tokens)::bigint AS completion_tokens, \
                SUM(cached_tokens)::bigint     AS cached_tokens \
         FROM endpoint_model_usage \
         WHERE day > current_date - $1::int \
         GROUP BY endpoint_id, model_id",
    )
    .bind(days as i32)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let prices: Vec<ModelPrice> = sqlx::query_as("SELECT * FROM endpoint_model_price")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    let price_of: HashMap<(uuid::Uuid, String), ModelPrice> = prices
        .into_iter()
        .map(|p| ((p.endpoint_id, p.model_id.clone()), p))
        .collect();

    let mut by_ep: HashMap<uuid::Uuid, Vec<ModelUsage>> = HashMap::new();
    for u in usage {
        by_ep.entry(u.endpoint_id).or_default().push(u);
    }

    // 余额那条独立的证据链，留着和单价算出来的数互相印证。
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
        let mut targets: Vec<(uuid::Uuid, String, bool, bool)> =
            vec![(r.id, format!("{}（自带地址）", r.label), true, true)];
        for e in eps.get(&r.id).into_iter().flatten() {
            let label = if e.label.trim().is_empty() { "未命名出口".to_string() } else { e.label.clone() };
            targets.push((e.id, label, false, e.active));
        }

        for (id, label, is_own, active) in targets {
            let used = by_ep.get(&id).map(|v| v.as_slice()).unwrap_or(&[]);
            let mut models: Vec<ModelRow> = Vec::new();
            let mut unpriced: Vec<String> = Vec::new();
            let mut revenue = 0.0_f64;
            let mut cost = 0.0_f64;

            for u in used {
                let rev = u.revenue_micro as f64 / 1_000_000.0;
                revenue += rev;
                let p = price_of.get(&(id, u.model_id.clone()));
                let c = p.map(|p| model_cost_usd(u, p));
                match &c {
                    Some(v) => cost += v,
                    None => unpriced.push(u.model_id.clone()),
                }
                models.push(ModelRow {
                    model_id: u.model_id.clone(),
                    calls: u.calls,
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    cached_tokens: u.cached_tokens,
                    revenue_usd: rev,
                    cost_usd: c,
                    margin_usd: c.map(|c| rev - c),
                    input_per_mtok: p.map(|p| p.input_per_mtok),
                    output_per_mtok: p.map(|p| p.output_per_mtok),
                    cached_per_mtok: p.and_then(|p| p.cached_per_mtok),
                    price_note: p.map(|p| p.note.clone()).unwrap_or_default(),
                });
            }
            // 贵的排前面：一个出口上二十个模型时，该先看哪个由金额决定。
            models.sort_by(|a, b| b.revenue_usd.partial_cmp(&a.revenue_usd).unwrap_or(std::cmp::Ordering::Equal));

            // 少录一个模型的价，整行的成本就是未知 —— 把没录的那部分按 0 加进来，
            // 得到的是一个看起来很精确的错数字，而且它会让这一行显示成高毛利。
            let row_cost = unpriced.is_empty().then_some(cost).filter(|_| !used.is_empty());
            let margin = row_cost.map(|c| revenue - c);
            let (bal_cost, bal_basis, _samples, bal_note) = balance_cost(edges.get(&id));

            rows.push(ReconRow {
                endpoint_id: id,
                route_id: r.id,
                route_label: r.label.clone(),
                label,
                vendor,
                is_own,
                active,
                calls: used.iter().map(|u| u.calls).sum(),
                revenue_usd: revenue,
                cost_usd: row_cost,
                margin_usd: margin,
                // 收入是 0 时毛利率没有意义（分母为零），不报 0% —— 那读起来像「打平」。
                margin_pct: margin.filter(|_| revenue > 0.0).map(|m| m / revenue * 100.0),
                unpriced_models: unpriced,
                cost_by_balance_usd: bal_cost,
                balance_basis: bal_basis,
                balance_note: bal_note,
                models,
            });
        }
    }

    // 亏得最狠的排最前：这一页是用来发现问题的，不是用来浏览的。
    // 没有毛利数的（价没录全）沉到最后，而不是混在中间冒充打平。
    rows.sort_by(|a, b| match (a.margin_usd, b.margin_usd) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    let revenue_total: f64 = rows.iter().map(|r| r.revenue_usd).sum();
    // 合计只加**算得出成本**的那些行，并把加了几行报出去 —— 否则总额会把
    // 「还没录价的出口」当成零成本，合计毛利凭空变好看。
    let counted: Vec<&ReconRow> = rows.iter().filter(|r| r.cost_usd.is_some()).collect();
    let cost_total: f64 = counted.iter().filter_map(|r| r.cost_usd).sum();
    let counted_revenue: f64 = counted.iter().map(|r| r.revenue_usd).sum();
    // 还差多少个模型没录价 —— 这是「离能看真数还有多远」的唯一进度条。
    let unpriced_total: usize = rows.iter().map(|r| r.unpriced_models.len()).sum();

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
            "unpriced_models": unpriced_total,
        },
    })))
}

#[derive(serde::Deserialize)]
pub struct SavePriceReq {
    pub endpoint_id: uuid::Uuid,
    pub model_id: String,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    #[serde(default)]
    pub cached_per_mtok: Option<f64>,
    #[serde(default)]
    pub note: String,
}

/// `POST /api/admin/endpoint-prices` —— 录一个出口上某个模型的真实进价。
pub async fn admin_save_price(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<SavePriceReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if req.model_id.trim().is_empty() {
        return Err(AppError::bad("缺少模型"));
    }
    // 负价一定是填错了。放进去的话成本会变成负数，那一行会显示成「毛利高于收入」。
    // 0 是合法的（确实有白送的模型），负数不是。
    for (v, what) in [
        (req.input_per_mtok, "输入价"),
        (req.output_per_mtok, "输出价"),
        (req.cached_per_mtok.unwrap_or(0.0), "缓存价"),
    ] {
        if !v.is_finite() || v < 0.0 {
            return Err(AppError::bad(format!("{what}不能是负数或非数字")));
        }
    }
    sqlx::query(
        "INSERT INTO endpoint_model_price \
           (endpoint_id, model_id, input_per_mtok, output_per_mtok, cached_per_mtok, note) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (endpoint_id, model_id) DO UPDATE SET \
           input_per_mtok = EXCLUDED.input_per_mtok, \
           output_per_mtok = EXCLUDED.output_per_mtok, \
           cached_per_mtok = EXCLUDED.cached_per_mtok, \
           note = EXCLUDED.note, \
           updated_at = now()",
    )
    .bind(req.endpoint_id)
    .bind(req.model_id.trim())
    .bind(req.input_per_mtok)
    .bind(req.output_per_mtok)
    .bind(req.cached_per_mtok)
    .bind(req.note.trim())
    .execute(&state.db)
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// 从首尾两条余额读数算成本。
///
/// 返回 (成本, 口径, 采样点数, 说明)。成本为 None 时说明里一定有一句话 ——
/// 「这里没有数字」和「为什么没有」必须一起给，否则面板上一片横杠没人知道该做什么。
fn balance_cost(e: Option<&BalanceEdge>) -> (Option<f64>, Option<&'static str>, i64, String) {
    let Some(e) = e else {
        return (None, None, 0, "这个出口还没有余额读数（多数中转的余额接口认的是控制台令牌，不是调用密钥）".into());
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

    fn src() -> String {
        let all = include_str!("reconcile.rs");
        all.split("\n#[cfg(test)]").next().unwrap().to_string()
    }

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

    fn usage(prompt: i64, cached: i64, completion: i64) -> ModelUsage {
        ModelUsage {
            endpoint_id: uuid::Uuid::nil(),
            model_id: "m".into(),
            calls: 1,
            revenue_micro: 0,
            prompt_tokens: prompt,
            completion_tokens: completion,
            cached_tokens: cached,
        }
    }

    fn price(inp: f64, out: f64, cached: Option<f64>) -> ModelPrice {
        ModelPrice {
            endpoint_id: uuid::Uuid::nil(),
            model_id: "m".into(),
            input_per_mtok: inp,
            output_per_mtok: out,
            cached_per_mtok: cached,
            note: String::new(),
        }
    }

    /// 缓存 token 必须先从输入里减出来。
    ///
    /// `prompt_tokens` 是**含**缓存命中的总输入（各家 usage 帧都是这个口径）。
    /// 不减直接乘输入价，命中率高的模型成本会被高估好几倍 —— 而缓存价通常只有输入价
    /// 的十分之一，正是它让「同一段对话第二轮便宜得多」成立。
    #[test]
    fn cached_input_is_billed_at_the_cache_rate_not_the_input_rate() {
        // 100 万输入里 90 万命中缓存，输入 $3/M、缓存 $0.3/M、输出 $15/M，输出 10 万。
        let u = usage(1_000_000, 900_000, 100_000);
        let c = model_cost_usd(&u, &price(3.0, 15.0, Some(0.3)));
        // 10 万新输入 × 3 + 90 万缓存 × 0.3 + 10 万输出 × 15，全部 ÷ 1e6
        let want = (100_000.0 * 3.0 + 900_000.0 * 0.3 + 100_000.0 * 15.0) / 1_000_000.0;
        assert!((c - want).abs() < 1e-9, "算出 {c}，应为 {want}");

        // 不减缓存的话会算成这个数 —— 差 2.4 倍，而且是往高了算。
        let naive = (1_000_000.0 * 3.0 + 100_000.0 * 15.0) / 1_000_000.0;
        assert!(naive > c * 2.0, "这个测试本身失去意义了：两种算法差别太小");
    }

    /// 没录缓存价时按输入价算 —— 保守方向，宁可把成本算高也不要把毛利算好看。
    #[test]
    fn a_missing_cache_price_falls_back_to_the_input_price() {
        let u = usage(1_000_000, 900_000, 0);
        let c = model_cost_usd(&u, &price(3.0, 15.0, None));
        assert!((c - 3.0).abs() < 1e-9, "应当整段按输入价算，得到 3.0，实得 {c}");
    }

    /// cached 比 prompt 还大时不能算出负的新增输入。
    ///
    /// 上游偶尔会回不自洽的 usage（cached 超过 prompt）。不夹的话 `fresh` 变负数，
    /// 成本被减掉一块，那一行的毛利凭空变高 —— 而且没有任何迹象。
    #[test]
    fn an_inconsistent_usage_frame_cannot_produce_negative_input() {
        let u = usage(100, 5_000, 0);
        let c = model_cost_usd(&u, &price(3.0, 15.0, Some(0.3)));
        assert!(c >= 0.0, "算出了负成本：{c}");
        // 全部按缓存价算：100 × 0.3 / 1e6
        assert!((c - 100.0 * 0.3 / 1_000_000.0).abs() < 1e-12, "夹取之后的口径不对：{c}");
    }

    /// 少录一个模型的价，整行成本就是未知，不能把那部分按 0 加进来。
    #[test]
    fn one_unpriced_model_makes_the_whole_row_unknown() {
        let s = src();
        assert!(
            s.contains("let row_cost = unpriced.is_empty().then_some(cost)"),
            "行成本没有要求「全部模型都录了价」—— 漏一个就会得到一个看着很精确的错数字",
        );
        // 合计也必须排掉这些行。
        assert!(
            s.contains("filter(|r| r.cost_usd.is_some())"),
            "合计把没录价的行按零成本计入了，毛利会凭空变好看",
        );
    }

    /// 没有用量的出口不该显示成「成本 0、毛利 0」。
    #[test]
    fn an_idle_endpoint_has_no_cost_at_all() {
        let s = src();
        assert!(
            s.contains(".filter(|_| !used.is_empty())"),
            "一个这段时间没跑过的出口会显示成成本 0 —— 那读起来像「白用」，其实是「没用过」",
        );
    }

    /// 负单价一定是填错了，必须在入口拦掉。
    #[test]
    fn a_negative_price_is_refused_at_the_door() {
        let s = src();
        let i = s.find("pub async fn admin_save_price(").expect("录价入口不见了");
        let body = &s[i..];
        assert!(
            body.contains("!v.is_finite() || v < 0.0"),
            "没拦负数/NaN —— 成本会变成负的，那一行显示成毛利高于收入",
        );
        assert!(body.contains("cached_per_mtok"), "缓存价没参与校验");
    }

    /// 充值不能变成负成本（余额那条独立证据链）。
    #[test]
    fn a_top_up_never_becomes_negative_cost() {
        let (cost, basis, _, note) = balance_cost(Some(&edge(Some(10.0), Some(110.0), None, None, 5)));
        assert!(cost.is_none(), "充值被算成了负成本");
        assert!(basis.is_none());
        assert!(note.contains("充过值"), "没说清为什么没有数字：{note}");
    }

    /// 有「已用」时优先用它 —— 它不被充值打断。
    #[test]
    fn used_wins_over_remaining() {
        let (cost, basis, _, _) =
            balance_cost(Some(&edge(Some(10.0), Some(110.0), Some(3.0), Some(8.0), 5)));
        assert_eq!(basis, Some("used"), "有已用却退回去用余额了");
        assert!((cost.unwrap() - 5.0).abs() < 1e-9, "成本算错：{cost:?}");
    }

    /// 一个采样点算不出差额，而且必须说清楚。
    #[test]
    fn one_sample_is_not_a_delta() {
        let (cost, _, n, note) = balance_cost(Some(&edge(Some(10.0), Some(10.0), None, None, 1)));
        assert!(cost.is_none());
        assert_eq!(n, 1);
        assert!(!note.is_empty(), "没有数字也没有原因，面板上就是一片横杠");
    }

    /// 完全没有读数 ≠ 成本为零。
    #[test]
    fn no_reading_is_not_zero_cost() {
        let (cost, _, n, note) = balance_cost(None);
        assert!(cost.is_none(), "没有读数被当成了零成本 —— 那会让毛利凭空变好看");
        assert_eq!(n, 0);
        assert!(!note.is_empty());
    }

    /// 「只给上限」既不是余额也不是已用，不许拿来算。
    #[test]
    fn a_hard_limit_is_not_a_balance() {
        let (cost, basis, _, note) = balance_cost(Some(&edge(None, None, None, None, 4)));
        assert!(cost.is_none() && basis.is_none());
        assert!(note.contains("上限"), "{note}");
    }

    /// 这一页不许再出现「按收入反推成本」那套估算。
    ///
    /// 用户点名要真实计算。估算那版是 `收入 × cost_ratio / rate` —— 它的前提是
    /// `cost_ratio` 填对了，而那是个**选路旋钮**，不是价格。留着它就会有人去看它。
    #[test]
    fn there_is_no_estimate_left_anywhere() {
        let s = src();
        for banned in ["cost_est", "cost_ratio / r.rate", "revenue * cost_ratio"] {
            assert!(!s.contains(banned), "估算又回来了：{banned}");
        }
    }
}
