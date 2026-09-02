//! Who consumed the most, ranked.
//!
//! Two currencies, so two columns rather than one merged order: `cost_cents` is money drawn
//! from a balance or plan, `free_milli_points_spent` is the free daily pool. Adding them
//! would invent an exchange rate that does not exist, and ranking on money alone would bury
//! anyone who runs the free models heavily behind people who spent a dollar once.
//!
//! **Signed in only.** This ranks people, not models, and it reports what they spent — so
//! unlike the model leaderboard it replaced, it is not something to hand to an anonymous
//! visitor. Note what that does and does not buy: every signed-in account can see every
//! other account's spend. That is the shape of a leaderboard and it was asked for
//! deliberately; if it should be narrower, gate the route on `claims.role == "admin"` and
//! the whole exposure closes in one line.
//!
//! **没填资料的人显示的是遮挡后的邮箱**（`349****38@qq.com`），不是完整地址。
//!
//! 这里的取舍变过两次，值得记下来。最早显示的是按账号 id 拼的 `User 71548c` —— 那样的一列
//! 谁也认不出谁，等于没有排行榜。于是改成显示完整地址，代价是把每个没填资料的人的邮箱交给
//! 了**每一个登录用户**。现在取中间：首字符 + 域名，认得出是个账号、本人也认得出自己，但
//! 地址不再是可以抄走的。
//!
//! 遮挡用的是 `referral::mask_email`，和佣金那边同一个函数 —— 不另写一套，两份规则迟早会漂。
//! 会话要求仍然必须保留：它挡住的是"任何人都能拿到这张消费清单"。
//!
//! Avatars are the same picture the account console shows: a `data:` URL the account holder
//! uploaded, fetched only for the rows that survived truncation so the response carries at
//! most `2 * TOP_N` inline images rather than one per account in the window.

use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// How many people a page of a column shows.
///
/// Paging is server-side rather than "fetch everything and slice in the browser" because
/// each row can carry an inline picture of roughly 30 KB. Slicing client-side would mean
/// shipping every active account's photo to render six of them.
const PER_PAGE: usize = 6;

/// 1 点 is stored as 1000, so spend can be fractional (see 20260805_free_points_milli).
const MILLI: f64 = 1000.0;

#[derive(Deserialize)]
pub struct RankQuery {
    /// day | week | month. Anything else falls back to the week.
    window: Option<String>,
    /// 1-based page of the money column. Out of range is clamped, never an error.
    money_page: Option<i64>,
    /// 1-based page of the points column. The two columns page independently: they are
    /// different lengths, and one pager would strand the shorter column on a blank page.
    points_page: Option<i64>,
}

/// Where a page starts, and how many pages there are.
///
/// Clamped rather than validated: a page number is a navigation detail, and answering
/// "page 900 does not exist" with an error would turn a stale link into a broken screen.
fn page_bounds(requested: Option<i64>, total: usize) -> (usize, usize, usize) {
    let pages = total.div_ceil(PER_PAGE).max(1);
    let page = requested.unwrap_or(1).clamp(1, pages as i64) as usize;
    ((page - 1) * PER_PAGE, page, pages)
}

fn days_for(window: &str) -> (i32, &'static str) {
    match window {
        "day" => (1, "day"),
        "month" => (30, "month"),
        _ => (7, "week"),
    }
}

/// What to call someone on this page.
///
/// Their own name if they set one, otherwise a **masked** address (`349****38@qq.com` —— 见模块
/// 顶部的说明）。The id fragment is a last resort
/// no live row should reach: the column is NOT NULL UNIQUE, so it exists only so a corrupt
/// row degrades to a label rather than an empty cell.
fn display_name(id: Uuid, first: &str, last: &str, email: &str) -> String {
    let full = format!("{} {}", first.trim(), last.trim()).trim().to_string();
    if !full.is_empty() {
        return full;
    }
    if !email.trim().is_empty() {
        // 遮挡，不是原样。
        //
        // 这一栏最初是刻意展示完整地址的 —— 理由是「一个全是 User 71548c 的排行榜没法看」。
        // 那个理由成立，但代价被低估了：这个页面对**每一个登录用户**开放，于是每个人都能把
        // 没填过资料的人的邮箱抄走。首字符 + 域名足够让人认出"这是个账号"，也够本人认出自己，
        // 而地址本身不再是可复制的。
        //
        // 复用 referral.rs 那一个，不另写一套：两份遮挡规则迟早会漂，而漂的那一天不会有人发现。
        return crate::referral::mask_email(email.trim());
    }
    format!("User {}", &id.simple().to_string()[..6])
}

/// One row per account, already aggregated over the window:
/// id, first name, last name, address, cents, milli-points, paying calls, free calls.
///
/// **两个调用数，不是一个。** 以前这里只有一个 `count(*)`，两栏共用 —— 于是"按花钱排"
/// 那一栏里写的是这个账号在窗口内的**全部**调用，包含一次钱都没花的免费调用；而同一个
/// 数字又原样出现在"按点数排"那一栏。403 次调用配 $41.50，读起来像"这 403 次花掉了
/// $41.50"，实际上其中可能只有几十次是付费的。各栏只数各自那种消耗。
type Spender = (Uuid, String, String, String, i64, i64, i64, i64);

/// `GET /api/rankings?window=day|week|month`
///
/// Requires a session. Returns two rankings of accounts over the window — by money spent
/// and by free points spent — plus the totals each column is a share of.
pub async fn list(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<RankQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let me = Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let (days, window) = days_for(q.window.as_deref().unwrap_or("week"));

    // The join drops usage whose account has been deleted (`user_id` is ON DELETE SET
    // NULL), which is correct: those calls have nobody left to rank.
    let rows: Vec<Spender> = sqlx::query_as(
        "SELECT u.id, u.first_name, u.last_name, u.email, \
                COALESCE(SUM(mu.cost_cents), 0)::bigint, \
                COALESCE(SUM(mu.free_milli_points_spent), 0)::bigint, \
                (count(*) FILTER (WHERE mu.cost_cents > 0))::bigint, \
                (count(*) FILTER (WHERE mu.free_milli_points_spent > 0))::bigint \
         FROM model_usage mu \
         JOIN users u ON u.id = mu.user_id \
         WHERE mu.created_at > now() - make_interval(days => $1) \
         GROUP BY u.id, u.first_name, u.last_name, u.email",
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;

    // `amount` picks the currency this column ranks on; a row with none of it is not in
    // the column at all, so "spent nothing" never occupies a rank above someone who did.
    // The total is of the whole column, taken before the page is cut out of it, so a share
    // means "of everything spent this window" rather than "of the six rows on screen".
    let column = |amount: fn(&Spender) -> i64,
                  requested: Option<i64>|
     -> (Vec<&Spender>, i64, usize, usize, usize) {
        let mut group: Vec<&Spender> = rows.iter().filter(|r| amount(r) > 0).collect();
        group.sort_by(|a, b| amount(b).cmp(&amount(a)));
        let total: i64 = group.iter().map(|r| amount(r)).sum();
        let count = group.len();

        let (start, page, pages) = page_bounds(requested, count);
        let slice = group.into_iter().skip(start).take(PER_PAGE).collect();
        (slice, total, page, pages, count)
    };

    let (money_rows, total_cents, money_page, money_pages, money_count) =
        column(|r| r.4, q.money_page);
    let (points_rows, total_milli, points_page, points_pages, points_count) =
        column(|r| r.5, q.points_page);

    // Pictures for the rows on these two pages, and only those. Each is an inline `data:`
    // URL of roughly 30 KB, so selecting them alongside the aggregate would have put one
    // in the response for every account active in the window — hundreds of images to draw
    // a dozen. Bounded at 2 * PER_PAGE, minus whoever appears in both columns, which is
    // why the ids are deduplicated first.
    let mut shown: Vec<Uuid> = money_rows
        .iter()
        .chain(points_rows.iter())
        .map(|r| r.0)
        .collect();
    shown.sort();
    shown.dedup();

    let avatars: HashMap<Uuid, String> =
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, avatar FROM users WHERE id = ANY($1) AND avatar IS NOT NULL AND avatar <> ''",
        )
        .bind(&shown)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();

    // `start` is where this page begins in the whole column, so rank 7 on page two is
    // labelled 7 rather than restarting at 1.
    // `calls` is passed in alongside `amount` so each column reports the calls that spent
    // *its* currency. One shared count made the same number appear in both columns, next to
    // two different figures, describing neither.
    let render = |group: Vec<&Spender>,
                  total: i64,
                  start: usize,
                  amount: fn(&Spender) -> i64,
                  calls: fn(&Spender) -> i64|
     -> Vec<Value> {
        group
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let (id, first, last, email, cents, milli, _, _) = r;
                json!({
                    "rank": start + i + 1,
                    "name": display_name(*id, first, last, email),
                    "avatar": avatars.get(id),
                    // So the viewer can find themselves without the page having to
                    // match on anything identifying.
                    "you": *id == me,
                    "cents": cents,
                    "points": (*milli as f64) / MILLI,
                    "calls": calls(r),
                    "share": if total > 0 { (amount(r) as f64) * 100.0 / total as f64 } else { 0.0 },
                })
            })
            .collect()
    };

    let money = render(
        money_rows,
        total_cents,
        (money_page - 1) * PER_PAGE,
        |r| r.4,
        |r| r.6,
    );
    let points = render(
        points_rows,
        total_milli,
        (points_page - 1) * PER_PAGE,
        |r| r.5,
        |r| r.7,
    );

    Ok(Json(json!({
        "window": window,
        "days": days,
        // `page` is the page actually served, which is not always the one asked for —
        // the client reads it back so a clamped request leaves the pager showing where
        // it really is rather than where it tried to go.
        "money": { "rows": money, "page": money_page, "pages": money_pages, "total": money_count },
        "points": { "rows": points, "page": points_page, "pages": points_pages, "total": points_count },
        "per_page": PER_PAGE,
        "total_cents": total_cents,
        "total_points": (total_milli as f64) / MILLI,
        // 换算分母，和 /api/me、/api/billing/catalog 下发的是同一个。
        //
        // 这个网关里所有 `*_cents` 都是**原始计费分**，一美元等于 663 个（运营可调），
        // 不是 100。排行榜页面此前按 100 除，于是每一个金额都被放大了 6.63 倍 ——
        // $7.54 印成 $50.02。跟着数据一起下发而不是让页面写死：这是一个运营设置，
        // account-ui 的 format.ts 为此专门写了「永远不要写死这个分母」。
        "raw_cents_per_credit_usd": crate::settings::raw_cents_per_credit_usd(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_falls_back_to_a_week() {
        assert_eq!(days_for("day"), (1, "day"));
        assert_eq!(days_for("week"), (7, "week"));
        assert_eq!(days_for("month"), (30, "month"));
        // Anything unrecognised, including an empty or hostile value.
        assert_eq!(days_for("year"), (7, "week"));
        assert_eq!(days_for(""), (7, "week"));
        assert_eq!(days_for("'; DROP TABLE model_usage; --"), (7, "week"));
    }

    /// A ranking of people that anyone could fetch would be a spend report on the
    /// customer base. The session requirement is the whole protection.
    #[test]
    fn the_ranking_requires_a_session() {
        let src = include_str!("rankings.rs");
        let sig = src.split("pub async fn list(").nth(1).expect("list");
        // Up to the `) ->` that closes the parameter list — not the first `)`, which
        // belongs to the `State(state)` pattern.
        let sig = &sig[..sig.find(") ->").unwrap_or(sig.len())];
        assert!(
            sig.contains("claims: Claims"),
            "listing who spent what must extract Claims, or the route is public"
        );
    }

    /// A name the account holder chose always wins over their address. This is the whole
    /// of what limits the exposure the module note describes: someone who fills in their
    /// profile stops being listed by address, and that must not silently regress.
    #[test]
    fn a_chosen_name_is_preferred_to_the_address() {
        let id = Uuid::parse_str("4f2a91c3-0000-0000-0000-000000000000").unwrap();
        let addr = "someone@example.com";
        assert_eq!(display_name(id, "Michael", "Hu", addr), "Michael Hu");
        assert_eq!(display_name(id, "Michael", "", addr), "Michael");
        assert_eq!(display_name(id, "", "Hu", addr), "Hu");
        // 没填名字时退到**遮挡后**的地址，不是原样地址。
        assert_eq!(display_name(id, "", "", addr), "som****ne@example.com");
        assert_eq!(display_name(id, "  ", " ", addr), "som****ne@example.com");
        // 完整地址绝不能出现在这一栏 —— 这个页面对每一个登录用户开放。
        // 完整地址绝不能出现在这一栏 —— 这个页面对每一个登录用户开放。
        assert!(!display_name(id, "", "", addr).contains("someone"));
        // 也不能遮到两个账号看起来一样。
        assert_ne!(
            display_name(id, "", "", "3491274438@qq.com"),
            display_name(id, "", "", "303813717@qq.com"),
        );
        // 星号个数与长度无关，否则遮挡本身就在报长度。
        let stars = |a: &str| display_name(id, "", "", a).matches('*').count();
        assert_eq!(stars("a@x.io"), stars("averylongmailbox@x.io"));
        // Last resort only: the column is NOT NULL, so no live row reaches this.
        assert_eq!(display_name(id, "", "", ""), "User 4f2a91");
    }

    /// 页面上的金额是这个接口的 `cents` 除以这个接口的 `raw_cents_per_credit_usd` 得来的。
    /// 少下发这个字段，页面就只能自己猜一个分母 —— 上一次猜的是 100，把每一笔金额都放大
    /// 了 6.63 倍（真实 $7.54 印成 $50.02）。
    #[test]
    fn the_response_carries_the_dollar_divisor() {
        let src = include_str!("rankings.rs");
        let body = src.split("pub async fn list(").nth(1).expect("list");
        let body = &body[..body.find("\n#[cfg(test)]").unwrap_or(body.len())];
        assert!(
            body.contains(r#""raw_cents_per_credit_usd": crate::settings::raw_cents_per_credit_usd()"#),
            "必须下发换算分母，且取自 settings —— 写死一个常数就是又开一个会漂的副本",
        );
        // 100 是"普通美分"的分母。这个网关里没有任何一个 *_cents 是普通美分。
        assert!(
            !body.contains("/ 100") && !body.contains("/100"),
            "服务端只出原始计费分，任何除 100 都说明这里混进了另一套单位",
        );
    }

    /// 两栏各数各自的调用。
    ///
    /// 以前是一个 `count(*)` 两栏共用：于是"按花钱排"里印的是这个账号窗口内的**全部**
    /// 调用（含一分钱没花的免费调用），同一个数字又原样出现在"按点数排"里。403 次调用
    /// 配 $41.50，读起来像是这 403 次花掉了 $41.50。
    #[test]
    fn each_column_counts_only_the_calls_that_spent_its_currency() {
        let src = include_str!("rankings.rs");
        let body = src.split("pub async fn list(").nth(1).expect("list");
        let body = &body[..body.find("\n#[cfg(test)]").unwrap_or(body.len())];
        assert!(
            body.contains("count(*) FILTER (WHERE mu.cost_cents > 0)")
                && body.contains("count(*) FILTER (WHERE mu.free_milli_points_spent > 0)"),
            "两栏要各有一个带 FILTER 的计数",
        );
        // 光有两个计数还不够：得真的分别用上，两栏取的是不同的字段。
        assert!(
            body.contains("|r| r.6,") && body.contains("|r| r.7,"),
            "钱那栏取 r.6、点数那栏取 r.7；两栏取同一个字段就等于回到共用一个计数",
        );
    }

    /// A page number is navigation, not input to validate: a stale or hand-typed one
    /// should land somewhere sensible rather than produce an error screen.
    #[test]
    fn a_page_out_of_range_lands_on_a_real_page() {
        // 15 rows over 6 per page is three pages, the last one short.
        assert_eq!(page_bounds(Some(1), 15), (0, 1, 3));
        assert_eq!(page_bounds(Some(2), 15), (6, 2, 3));
        assert_eq!(page_bounds(Some(3), 15), (12, 3, 3));
        // Past the end clamps to the last page; before the start clamps to the first.
        assert_eq!(page_bounds(Some(900), 15), (12, 3, 3));
        assert_eq!(page_bounds(Some(0), 15), (0, 1, 3));
        assert_eq!(page_bounds(Some(-5), 15), (0, 1, 3));
        assert_eq!(page_bounds(None, 15), (0, 1, 3));
        // Exactly full pages must not produce a trailing empty one.
        assert_eq!(page_bounds(Some(2), 12), (6, 2, 2));
        // An empty column is one page, not zero — the pager has to render something.
        assert_eq!(page_bounds(Some(1), 0), (0, 1, 1));
        assert_eq!(page_bounds(Some(4), 0), (0, 1, 1));
    }

    /// Pictures are fetched for the rows on screen, not for everyone in the window. Each
    /// is a ~30 KB inline image, so the difference is a response of tens of kilobytes
    /// versus one of many megabytes on a busy month.
    #[test]
    fn pictures_are_fetched_only_for_the_rows_shown() {
        let src = include_str!("rankings.rs");
        let body = src.split("pub async fn list(").nth(1).expect("list");
        let body = &body[..body.find("\n#[cfg(test)]").unwrap_or(body.len())];
        let aggregate = body.split("let column =").next().unwrap_or("");
        assert!(
            !aggregate.contains("avatar"),
            "the aggregate query must not carry a picture for every active account"
        );
        assert!(
            body.contains("shown.dedup()"),
            "a person in both columns must not have their picture sent twice"
        );
    }
}
