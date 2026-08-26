//! 套餐额度体检 —— 「这个套餐给多少额度合适」的数据底座。
//!
//! # 它替掉了什么
//!
//! 老的「定价试算」是**正向**的：运营输入「给多少额度 + 卖多少钱」，它算保本倍率。
//! 问题是它要人先猜一个额度，而且成本那一步要人**手选一个渠道**——于是随手选中一个
//! 便宜渠道，算出来的成本就漂亮得不像话。线上真实情况是：60% 的流量跑在 usd_per_cny=1
//! 的站上，而页面默认那条是 usd_per_cny=10，两者差十倍。
//!
//! 这里反过来：把**真实用量**和**真实渠道构成**都从库里算出来，然后回答
//! 「现有这几个套餐，额度是偏紧还是偏大方、按现在的价卖亏不亏」。
//!
//! # 三个数据源，都不是假设
//!
//! * `model_usage` —— 每一次真实请求扣了多少真实分。用来算「一个付费用户一天烧多少」。
//! * `models` × `channel_rates` —— 每条连接指向哪个中转、那个中转 ¥1 买多少美元额度。
//!   用来把真实分折成人民币。
//! * `plan_quotas` × `prices` —— 每个套餐给多少额度、卖多少钱。
//!
//! # 单位链（错一位就差一百倍，所以写在这里）
//!
//! `plan_quotas.total_cents`、`users.quota_total_cents`、`model_usage.cost_cents` 是**同一个单位**：
//! 真实计费分。面值额度（用户看到的那个 $）= 真实分 ÷ `raw_cents_per_credit_usd`（默认 663，
//! 从 app_settings 取，本文件不许出现这个字面量）。人民币成本 = 真实分 ÷ 100 ÷ usd_per_cny。
//!
//! 实测对得上线上那张截图：面值 $271 × 663 / 100 = $1796.73 真实，÷10 = ¥179.67。

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// 用量画像取最近多少天。两个月是这套库现在全部的数据量，再长没有意义；
/// 再短则赶上「大部分用户只活跃两天」这个分布，样本会碎掉。
const WINDOW_DAYS: i64 = 60;

/// 一个「有效样本」至少要有多少个付费用户。低于它就不给百分位——三五个人的
/// p90 是噪声，把它印在定价页上比不印更糟。
const MIN_PAYERS: i64 = 5;

#[derive(Serialize)]
struct Burn {
    p50: f64,
    p75: f64,
    p90: f64,
    max: f64,
    active_days_p50: f64,
    payers: i64,
    requests: i64,
}

#[derive(Serialize)]
struct ChannelMix {
    name: String,
    host: String,
    usd_per_cny: f64,
    requests: i64,
    raw_cents: i64,
    cny: Option<f64>,
    /// 这条渠道占总真实消耗的比例。定价要看它：一条 usd_per_cny=1 的线只要占了
    /// 六成流量，整体成本就跟着它走，而不是跟着最便宜那条走。
    share: f64,
}

pub async fn admin_plan_health(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }

    // 面值分母只有一个源（settings.rs），本文件不许出现那个字面量 ——
    // 有一条测试在守它（the_credit_denominator_has_exactly_one_source）。
    let denominator = crate::settings::raw_cents_per_credit_usd();

    // ---- 1. 付费用户的真实日耗 ----
    //
    // 判据是「付过费」而不是「现在挂着套餐」：套餐会过期，而过期用户当时的用量
    // 同样是付费用户的用量。只看当前挂套餐的，样本会少掉一大半。
    //
    // 按「用户 × 有活跃的那一天」聚合再取百分位，而不是直接对请求取百分位：
    // 定价关心的是「一个人一天烧多少」，不是「一次请求花多少」。
    let burn = sqlx::query_as::<_, (Option<i64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<i64>)>(
        "WITH payer AS ( \
           SELECT DISTINCT u.id FROM users u \
           WHERE (u.plan <> '' AND u.plan <> 'none') \
              OR EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id AND o.status = 'paid') \
         ), d AS ( \
           SELECT m.user_id, m.created_at::date AS day, sum(m.cost_cents) AS c, count(*) AS reqs \
           FROM model_usage m JOIN payer p ON p.id = m.user_id \
           WHERE m.created_at > now() - ($1 || ' days')::interval \
           GROUP BY 1, 2 \
         ), pu AS ( \
           SELECT user_id, count(*)::float8 AS active_days, avg(c) AS per_day, sum(reqs) AS reqs \
           FROM d GROUP BY 1 \
         ) \
         SELECT count(*)::bigint, \
                percentile_cont(0.5) WITHIN GROUP (ORDER BY per_day), \
                percentile_cont(0.75) WITHIN GROUP (ORDER BY per_day), \
                percentile_cont(0.9) WITHIN GROUP (ORDER BY per_day), \
                max(per_day), \
                percentile_cont(0.5) WITHIN GROUP (ORDER BY active_days), \
                sum(reqs)::bigint \
         FROM pu",
    )
    .bind(WINDOW_DAYS.to_string())
    .fetch_one(&state.db)
    .await?;

    let payers = burn.0.unwrap_or(0);
    let enough = payers >= MIN_PAYERS;
    let burn = Burn {
        p50: burn.1.unwrap_or(0.0),
        p75: burn.2.unwrap_or(0.0),
        p90: burn.3.unwrap_or(0.0),
        max: burn.4.unwrap_or(0.0),
        active_days_p50: burn.5.unwrap_or(0.0),
        payers,
        requests: burn.6.unwrap_or(0),
    };

    // ---- 2. 真实流量落在哪些渠道上 ----
    //
    // 这一段是整个页面的关键。成本不该由「运营手选的那个渠道」决定，而该由
    // **流量实际落在哪** 决定。join 走 base_url 的 host —— 和 channel_rates.host 同一个键。
    let mix = sqlx::query_as::<_, (String, Option<String>, Option<f64>, i64, Option<i64>)>(
        "WITH u AS ( \
           SELECT mu.cost_cents, m.label, substring(m.base_url from '://([^/]+)') AS host \
           FROM model_usage mu JOIN models m ON m.id::text = mu.model_id::text \
           WHERE mu.created_at > now() - ($1 || ' days')::interval \
         ) \
         SELECT u.label, u.host, max(cr.usd_per_cny), count(*)::bigint, sum(u.cost_cents)::bigint \
         FROM u LEFT JOIN channel_rates cr ON cr.host = u.host \
         GROUP BY 1, 2 ORDER BY 5 DESC NULLS LAST",
    )
    .bind(WINDOW_DAYS.to_string())
    .fetch_all(&state.db)
    .await?;

    let total_raw: i64 = mix.iter().map(|r| r.4.unwrap_or(0)).sum();
    // 折人民币时**只算得出价的那部分**：一条没填购买价的线，它的消耗不能当成 0 块钱，
    // 否则整体成本会被系统性低估。它单独计数，前端要把这个缺口说出来。
    let mut priced_raw: i64 = 0;
    let mut priced_cny = 0.0_f64;
    let channels: Vec<ChannelMix> = mix
        .iter()
        .map(|(label, host, rate, reqs, raw)| {
            let raw = raw.unwrap_or(0);
            let cny = rate.filter(|r| *r > 0.0).map(|r| raw as f64 / 100.0 / r);
            if let Some(c) = cny {
                priced_raw += raw;
                priced_cny += c;
            }
            ChannelMix {
                name: label.clone(),
                host: host.clone().unwrap_or_default(),
                usd_per_cny: rate.unwrap_or(0.0),
                requests: *reqs,
                raw_cents: raw,
                cny,
                share: if total_raw > 0 { raw as f64 / total_raw as f64 } else { 0.0 },
            }
        })
        .collect();

    // 综合购买价：真实消耗的美元额度 ÷ 真实花掉的人民币。它才是「¥1 实际买到多少美元额度」。
    let blended = if priced_cny > 0.0 { Some(priced_raw as f64 / 100.0 / priced_cny) } else { None };
    let best = channels.iter().map(|c| c.usd_per_cny).fold(0.0_f64, f64::max);
    let worst = channels
        .iter()
        .filter(|c| c.usd_per_cny > 0.0)
        .map(|c| c.usd_per_cny)
        .fold(f64::INFINITY, f64::min);

    // ---- 3. 套餐表：额度、售价、成本、能撑多久 ----
    let plans = sqlx::query_as::<_, (String, i64, i64, i32, Option<i64>, Option<String>)>(
        "SELECT q.plan, q.total_cents, q.window_cents, q.days, \
                (SELECT p.amount_cents FROM prices p \
                  WHERE p.plan = q.plan AND p.kind = 'plan' \
                  ORDER BY p.active DESC, p.sort LIMIT 1), \
                (SELECT p.label FROM prices p \
                  WHERE p.plan = q.plan AND p.kind = 'plan' \
                  ORDER BY p.active DESC, p.sort LIMIT 1) \
         FROM plan_quotas q ORDER BY q.rank, q.total_cents",
    )
    .fetch_all(&state.db)
    .await?;

    let rows: Vec<serde_json::Value> = plans
        .iter()
        .map(|(plan, total, window, days, price_fen, label)| {
            let cost_at = |rate: f64| if rate > 0.0 { Some(*total as f64 / 100.0 / rate) } else { None };
            let cost_blended = blended.and_then(cost_at);
            let price_cny = price_fen.map(|f| f as f64 / 100.0);
            // 「能撑几个活跃日」：额度 ÷ 这一档用户的日耗。样本不够就不给数，
            // 三五个人的 p90 是噪声，印在定价页上比不印更糟。
            let lasts = |per_day: f64| {
                if enough && per_day > 0.0 { Some(*total as f64 / per_day) } else { None }
            };
            json!({
                "plan": plan,
                "label": label,
                "total_cents": total,
                "window_cents": window,
                "days": days,
                "visible_usd": *total as f64 / denominator as f64,
                "price_cny": price_cny,
                "cost_best": cost_at(best),
                "cost_blended": cost_blended,
                "cost_worst": if worst.is_finite() { cost_at(worst) } else { None },
                "margin_blended": match (price_cny, cost_blended) {
                    (Some(p), Some(c)) if p > 0.0 => Some((p - c) / p * 100.0),
                    _ => None,
                },
                "lasts_p50_days": lasts(burn.p50),
                "lasts_p90_days": lasts(burn.p90),
            })
        })
        .collect();

    Ok(Json(json!({
        "denominator": denominator,
        "window_days": WINDOW_DAYS,
        "enough_sample": enough,
        "min_payers": MIN_PAYERS,
        "burn": burn,
        "channels": channels,
        "blended_usd_per_cny": blended,
        "best_usd_per_cny": if best > 0.0 { Some(best) } else { None },
        "worst_usd_per_cny": if worst.is_finite() { Some(worst) } else { None },
        "unpriced_raw_cents": total_raw - priced_raw,
        "plans": rows,
    })))
}
