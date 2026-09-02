use axum::extract::{Path, State};
use crate::auth::QUOTA_WINDOW_REFRESH;
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// Membership tiers sold on the platform.
/// 内置套餐名。**只作为默认值和测试基线**，不是校验用的白名单——真源是后台可编辑的
/// `plan_quotas` 表（`settings::plans()`）。用它做校验会让两条发放路径给出相反结果：
/// 运营在后台新建一个套餐，Stripe 付款能正常发放（走 `plan_spec` 查表就找得到），
/// 后台手动发放却被判"套餐无效"。线上已经有一个这样的套餐（ceshi）。
pub const PLANS: [&str; 5] = ["trial", "basic", "pro", "power", "ultra"];

/// 这个套餐名现在真的可以发放吗——以后台配置为准。
///
/// 判据就是"它有没有额度规格"：能查到规格，Stripe 就能按它分配额度，后台自然也该能。
pub(crate) fn plan_is_grantable(plan: &str) -> bool {
    plan_spec(plan).is_some()
}

/// 拒绝时把后台**当前真实配置**的套餐列出来，而不是一句写死的 "trial / basic / pro …"。
pub(crate) fn grantable_plans_hint() -> String {
    let mut names: Vec<String> = crate::settings::plans().into_iter().map(|p| p.plan).collect();
    names.sort();
    names.join(" / ")
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Code {
    pub id: uuid::Uuid,
    pub code: String,
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
    pub note: String,
    pub status: String,
    pub used_by: Option<uuid::Uuid>,
    pub used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// A readable random code like `AB3K-9F2M-7QX4-WPED`.
fn gen_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // no easily-confused chars
    let mut rng = rand::thread_rng();
    let mut s = String::with_capacity(19);
    for g in 0..4 {
        if g > 0 {
            s.push('-');
        }
        for _ in 0..4 {
            s.push(CHARS[rng.gen_range(0..CHARS.len())] as char);
        }
    }
    s
}

// ---------- admin: generate ----------
#[derive(Deserialize)]
pub struct GenReq {
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
    pub count: i32,
    pub note: Option<String>,
}

pub async fn admin_generate(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<GenReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let count = req.count.clamp(1, 500);
    match req.kind.as_str() {
        "plan" => {
            let plan = req.plan.as_deref().unwrap_or("");
            if !plan_is_grantable(plan) {
                return Err(AppError::bad(format!(
                    "套餐无效（后台当前可用: {}）",
                    grantable_plans_hint()
                )));
            }
            if req.duration_days.unwrap_or(0) <= 0 {
                return Err(AppError::bad("时长(天)需大于 0"));
            }
        }
        "credits" => {
            if req.credits_cents.unwrap_or(0) <= 0 {
                return Err(AppError::bad("额度需大于 0"));
            }
        }
        _ => return Err(AppError::bad("类型只能是 plan 或 credits")),
    }
    let note = req.note.unwrap_or_default();
    let mut codes = Vec::with_capacity(count as usize);
    for _ in 0..count {
        // Retry a few times on the (very unlikely) unique collision.
        let mut attempt = 0;
        loop {
            let code = gen_code();
            let res = sqlx::query(
                "INSERT INTO activation_codes (code, kind, plan, duration_days, credits_cents, note) VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(&code)
            .bind(&req.kind)
            .bind(&req.plan)
            .bind(req.duration_days)
            .bind(req.credits_cents)
            .bind(&note)
            .execute(&state.db)
            .await;
            match res {
                Ok(_) => {
                    codes.push(code);
                    break;
                }
                Err(e) => {
                    attempt += 1;
                    if attempt >= 5 {
                        return Err(e.into());
                    }
                }
            }
        }
    }
    Ok(Json(json!({ "codes": codes, "count": codes.len() })))
}

// ---------- admin: list ----------
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Vec<Code>>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, Code>(
        "SELECT * FROM activation_codes ORDER BY created_at DESC LIMIT 1000",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

// ---------- admin: delete ----------
pub async fn admin_delete(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let res = sqlx::query("DELETE FROM activation_codes WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::bad("激活码不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

// ---------- apply a grant to a user (shared by redeem + admin grant) ----------
/// Per-plan quota spec: (total_cents, window_cap_cents, weekly_cap_cents, duration_days).
/// Amounts are USD cents; weekly 0 = unlimited.
///
/// window_cap must stay NON-ZERO on every plan. It is not a "disable the window" switch:
/// the refill sets `quota_window_cents = LEAST(quota_window_cap_cents, quota_total_cents)`,
/// so a cap of 0 refills the window balance to 0, and the serve gate requires
/// `q_window > 0` (models.rs, `quota_ok`). A zero cap therefore does not widen the limit —
/// it locks the plan out permanently, and the user is shown "本时段额度已用完，请等待刷新"
/// for a refresh that can never raise the balance above zero.
///
/// Moving to a weekly-primary scheme is a real design change, not a retuning: it needs
/// `quota_ok` and the four refill sites to treat a 0 cap as "unlimited window" first.
/// See `plan_spec_window_cap_is_never_zero` below.
///
/// 额度值原先写死在这里的 match 分支里，改一次要重新编译部署；现在唯一定义在
/// `plan_quotas` 表（见 `settings.rs`），种子值与原分支逐字相同。单位不变：真实计费分
/// （trial 的 5_000 = 上游 $50 成本，客户端按面值分母折算后显示）。
///
/// 编辑套餐**不会**改写已订阅用户：额度在兑换时由 `apply_plan` 写进 users 表，
/// 之后没有任何地方重新推导。唯一的例外是 `admin_set_plan` 的 `reset_quotas`。
pub(crate) fn plan_spec(plan: &str) -> Option<(i64, i64, i64, i32)> {
    crate::settings::plan_spec(plan)
}

/// Ordering of the built-in plans, so a grant never silently downgrades a user who
/// still holds a better plan. Unknown/absent plans rank 0.
pub(crate) fn plan_rank(plan: &str) -> i32 {
    crate::settings::plan_rank(plan)
}

/// 合并周额度上限。这个函数存在的唯一理由是：`0` 在这一列里有两种截然相反的含义，
/// 而原来的写法把它们当成了一种。
///
/// `users.quota_weekly_cap_cents` 的建表默认值是 0（0009_quota.sql，注释写着「0 = 不限」），
/// 所以一个从没被发放过套餐的账号，这一列必然是 0 —— 那是「还没配过」，不是任何人做出的
/// 「不限」选择。原来这里是 `if cur_weekly == 0 || weekly == 0 { 0 }`：第一次发放时基线的
/// 0 直接压过套餐给出的有限值，写回去仍然是 0，于是下一次发放再读到 0、再写回 0 ——
/// 这道闸**在 redeem / Stripe / 支付宝三条路上永远立不起来**。admin_set_plan 不受影响，
/// 它是把套餐的值直接写进去的，所以后台改过的账号看起来是对的，更掩盖了这个问题。
///
/// 判据换成「**手上这个套餐的规格**本身是不是不限」：
///   * cur_weekly > 0            → 已经有有限上限，取两者较高的（权益不缩水，原行为）。
///   * cur_weekly == 0 且当前套餐规格 weekly == 0 → 这个 0 是真实选择，保留不限。
///   * cur_weekly == 0 且当前套餐规格 weekly > 0，或压根没有套餐（'none'、后台已删）
///                               → 这个 0 是默认值/历史遗留，让新套餐的有限值立起来。
///
/// 用「当前套餐的规格」而不是「cur_plan != 'none'」，是因为后者只修得了首次发放：一个
/// 存量 basic 用户续费时 cur_plan 仍是 basic，运营新填的有限周上限还是落不下去，控制项
/// 照样是死的。
///
/// **今天线上是空操作**：plan_quotas 六个套餐的 weekly_cents 全是 0（运营的配置选择），
/// 所以首次发放取到的仍是 0、持有套餐的仍判为不限，写入值逐字不变。这条改的是「控制项
/// 失灵」本身 —— 运营哪天在后台填上一个有限周上限，存量订阅者和新订阅者才会真的受它约束。
///
/// 生效时机要说清楚，别当成「改完就全员生效」：这个函数只在**发放**时被调到（redeem /
/// Stripe / 支付宝三条路），所以后台填上上限之后，一个存量订阅者要等到**下一次续费或
/// 再发放**才会被写上。最长可以差一整个计费周期。想立刻全员生效只有两条路，都得人来
/// 决定，所以这里刻意不做：
///   · 写一条回填 migration —— 但今天没有值可回填（六个套餐都是 0），提前写等于替运营
///     做了一个他还没做的定价决定；
///   · 后台「保存周上限」时顺手对存量订阅者跑一次批量写入 —— 那是产品行为，不是这条
///     缺陷的范围。
/// 另有 5 个老账号（pro 10000、ultra 80000×3、trial 500）身上还挂着更早版本的默认上限，
/// 正被限着，而同档甚至更高档的新用户不被限。这批不对称同样只能靠上面两条之一抹平。
fn merge_weekly_cap(cur_plan: &str, cur_weekly: i64, plan_weekly: i64) -> i64 {
    // 新套餐自己声明不限：不限赢过任何有限值，和原来一样（买了更高的套餐不该被旧上限捆住）。
    if plan_weekly <= 0 {
        return 0;
    }
    if cur_weekly > 0 {
        return cur_weekly.max(plan_weekly);
    }
    // cur_weekly == 0 有两种完全相反的含义，必须去问用户**当前持有的套餐**才能分开：
    //   · 他现在持有的套餐自己就声明不限 → 这个 0 是一次真实的选择，保留不限。
    //   · 他现在没套餐、或持有的套餐是有限档但这一列还是建表默认值 → 这个 0 只是「还没配」，
    //     不能拿它去压掉新套餐的有限上限，否则第一次开通就把周上限永久做成不限
    //     （写回 0 之后，下一次授予再读到 0，再写回 0，永远建立不起来）。
    let cur_is_unlimited = plan_spec(cur_plan).is_some_and(|(_, _, weekly, _)| weekly <= 0);
    if cur_is_unlimited {
        return 0;
    }
    // 「还没配」这条路上仍然要和当前套餐自己的档位取大，不能直接写新套餐的值。
    // 上面 cur_weekly > 0 那条分支用的就是 max —— 上限是权益，叠加时保留更好的那个
    // （见 apply_plan 里 window_cap 的同款注释）。这里直接返回 plan_weekly 的话，
    // 一个 ultra 用户（周上限 80000，但这一列因为历史原因是 0）去兑换一张 basic 码
    // （5000），周上限会被按 basic 定死，等于降级。
    let cur_plan_weekly = plan_spec(cur_plan).map_or(0, |(_, _, weekly, _)| weekly);
    plan_weekly.max(cur_plan_weekly)
}

pub(crate) async fn apply_plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
    plan: &str,
    days: i32,
) -> ApiResult<()> {
    if let Some((total, window, weekly, spec_days)) = plan_spec(plan) {
        let dur = if days > 0 { days } else { spec_days };
        // Read the current state under a row lock: the new pools are derived from the
        // old ones, and two concurrent redeems must not both compute from the same base.
        let (cur_plan, cur_total, cur_window_cap, cur_window, cur_weekly): (
            String,
            i64,
            i64,
            i64,
            i64,
        ) = sqlx::query_as(
            "SELECT plan, quota_total_cents, quota_window_cap_cents, quota_window_cents, \
             quota_weekly_cap_cents FROM users WHERE id = $1 FOR UPDATE",
        )
        .bind(uid)
        .fetch_one(&mut **tx)
        .await?;

        // Quota is a balance, not a setting: grants ADD. Overwriting meant redeeming a
        // cheap code on top of an expensive plan destroyed the paid remainder (ultra
        // with $4000 left + a $50 trial code = $50), and stacking two of the same plan
        // only ever delivered one plan's worth.
        let new_total = cur_total.max(0).saturating_add(total);
        // Caps are entitlements: keep the better one rather than the newest one.
        let new_window_cap = cur_window_cap.max(window);
        // Don't shrink what's left in the live window; top it up toward the new cap.
        let new_window = cur_window.max(window.min(new_total)).min(new_window_cap);
        // 0 在这一列里既可能是「不限」也可能是「还没配过」，判据见 merge_weekly_cap。
        let new_weekly = merge_weekly_cap(&cur_plan, cur_weekly, weekly);
        let new_plan = if plan_rank(plan) >= plan_rank(&cur_plan) {
            plan.to_string()
        } else {
            cur_plan
        };

        // Note what is deliberately NOT reset here: quota_week_used_cents and the
        // window/week reset timestamps. Zeroing them on every grant let a user clear a
        // spent weekly cap (or refresh the 30-minute window) by redeeming any cheap code.
        // The access gate already rolls both windows over when their deadline passes.
        sqlx::query(
            &format!("UPDATE users SET plan = $1, \
             plan_expires_at = GREATEST(COALESCE(plan_expires_at, now()), now()) + ($2 * interval '1 day'), \
             quota_total_cents = $3, quota_window_cap_cents = $4, quota_window_cents = $5, \
             quota_window_reset_at = COALESCE(quota_window_reset_at, now() + interval '{QUOTA_WINDOW_REFRESH}'), \
             quota_weekly_cap_cents = $6, \
             quota_week_reset_at = COALESCE(quota_week_reset_at, now() + interval '7 days'), \
             updated_at = now() WHERE id = $7"),
        )
        .bind(&new_plan)
        .bind(dur)
        .bind(new_total)
        .bind(new_window_cap)
        .bind(new_window)
        .bind(new_weekly)
        .bind(uid)
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE users SET plan = $1, \
             plan_expires_at = GREATEST(COALESCE(plan_expires_at, now()), now()) + ($2 * interval '1 day'), \
             updated_at = now() WHERE id = $3",
        )
        .bind(plan)
        .bind(days)
        .bind(uid)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub(crate) async fn apply_credits(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
    cents: i64,
) -> ApiResult<()> {
    sqlx::query(
        "UPDATE users SET credits_cents = credits_cents + $1, updated_at = now() WHERE id = $2",
    )
    .bind(cents)
    .bind(uid)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn user_summary(state: &AppState, uid: uuid::Uuid) -> ApiResult<serde_json::Value> {
    let row = sqlx::query_as::<_, (String, Option<chrono::DateTime<chrono::Utc>>, i64)>(
        "SELECT plan, plan_expires_at, credits_cents FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await?;
    Ok(json!({ "plan": row.0, "plan_expires_at": row.1, "credits_cents": row.2 }))
}

// ---------- user: redeem (the IDE-facing endpoint) ----------
#[derive(Deserialize)]
pub struct RedeemReq {
    pub code: String,
}

pub async fn redeem(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<RedeemReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let input = req.code.trim().to_uppercase();
    if input.is_empty() {
        return Err(AppError::bad("请输入激活码"));
    }

    let mut tx = state.db.begin().await?;
    let code =
        sqlx::query_as::<_, Code>("SELECT * FROM activation_codes WHERE code = $1 FOR UPDATE")
            .bind(&input)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::bad("激活码无效"))?;
    if code.status != "unused" {
        return Err(AppError::bad("激活码已被使用"));
    }
    let granted = if code.kind == "plan" {
        let plan = code.plan.clone().unwrap_or_else(|| "none".into());
        apply_plan(&mut tx, uid, &plan, code.duration_days.unwrap_or(0)).await?;
        json!({ "kind": "plan", "plan": plan, "duration_days": code.duration_days })
    } else {
        apply_credits(&mut tx, uid, code.credits_cents.unwrap_or(0)).await?;
        json!({ "kind": "credits", "credits_cents": code.credits_cents })
    };
    sqlx::query(
        "UPDATE activation_codes SET status = 'used', used_by = $1, used_at = now() WHERE id = $2",
    )
    .bind(uid)
    .bind(code.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    crate::realtime::record_event(
        &state,
        Some(uid),
        "redeem",
        json!({ "email": claims.email, "grant": granted }),
    )
    .await;
    Ok(Json(
        json!({ "ok": true, "granted": granted, "user": user_summary(&state, uid).await? }),
    ))
}

// ---------- admin: manually grant a plan / credits to a user ----------
#[derive(Deserialize)]
pub struct GrantReq {
    pub kind: String,
    pub plan: Option<String>,
    pub duration_days: Option<i32>,
    pub credits_cents: Option<i64>,
}

pub async fn admin_grant(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<GrantReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let mut tx = state.db.begin().await?;
    match req.kind.as_str() {
        "plan" => {
            let plan = req.plan.as_deref().unwrap_or("");
            if !plan_is_grantable(plan) {
                return Err(AppError::bad(format!(
                    "套餐无效（后台当前可用: {}）",
                    grantable_plans_hint()
                )));
            }
            if req.duration_days.unwrap_or(0) <= 0 {
                return Err(AppError::bad("时长(天)需大于 0"));
            }
            apply_plan(&mut tx, id, plan, req.duration_days.unwrap_or(0)).await?;
        }
        "credits" => {
            if req.credits_cents.unwrap_or(0) == 0 {
                return Err(AppError::bad("额度不能为 0"));
            }
            apply_credits(&mut tx, id, req.credits_cents.unwrap_or(0)).await?;
        }
        _ => return Err(AppError::bad("类型只能是 plan 或 credits")),
    }
    tx.commit().await?;
    let summary = user_summary(&state, id).await?;
    crate::realtime::record_event(
        &state,
        Some(id),
        "user_updated",
        json!({ "by": claims.email, "action": "grant", "kind": req.kind, "user": summary.clone() }),
    )
    .await;
    Ok(Json(json!({ "ok": true, "user": summary })))
}

// ---------- admin: SET (not add) credits to an exact balance ----------
#[derive(Deserialize)]
pub struct SetCreditsReq {
    pub credits_cents: i64,
}

pub async fn admin_set_credits(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<SetCreditsReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if req.credits_cents < 0 {
        return Err(AppError::bad("额度不能为负数"));
    }
    sqlx::query("UPDATE users SET credits_cents = $1, updated_at = now() WHERE id = $2")
        .bind(req.credits_cents)
        .bind(id)
        .execute(&state.db)
        .await?;
    let summary = user_summary(&state, id).await?;
    crate::realtime::record_event(&state, Some(id), "user_updated", json!({ "by": claims.email, "action": "set_credits", "credits_cents": req.credits_cents, "user": summary.clone() })).await;
    Ok(Json(json!({ "ok": true, "user": summary })))
}

// ---------- admin: SET plan + expiry to specific absolute values ----------
#[derive(Deserialize)]
pub struct SetPlanReq {
    pub plan: String,
    /// ISO-8601 timestamp for when the plan expires. If null/missing → plan stays
    /// active indefinitely (treated as far-future).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// When true, also reset the per-window / weekly quota counters to the new
    /// plan's caps (a fresh start). Defaults to true since admins usually want
    /// the user to feel the plan change immediately.
    #[serde(default = "default_true")]
    pub reset_quotas: bool,
}

fn default_true() -> bool {
    true
}

/// 这个用户当前的总额度（真实计费分）。
///
/// 单独一个查询是为了让"不重置额度"这条路能先看一眼：没有额度可保持时，
/// "保持不动"就等于发一个空壳会员。
async fn quota_total_cents(state: &AppState, uid: uuid::Uuid) -> ApiResult<i64> {
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT quota_total_cents FROM users WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await?
            .unwrap_or(0),
    )
}

pub async fn admin_set_plan(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<SetPlanReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let plan = req.plan.trim();
    if plan != "none" && !plan_is_grantable(plan) {
        return Err(AppError::bad(format!(
            "套餐无效（后台当前可用: {} / none）",
            grantable_plans_hint()
        )));
    }
    if plan == "none" {
        // Treat plan="none" as a cancel — clear membership.
        sqlx::query(
            "UPDATE users SET plan = 'none', plan_expires_at = NULL, \
             quota_total_cents = 0, quota_window_cents = 0, quota_window_cap_cents = 0, \
             quota_weekly_cap_cents = 0, quota_week_used_cents = 0, \
             updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&state.db)
        .await?;
    } else if req.reset_quotas {
        if let Some((total, window, weekly, _)) = plan_spec(plan) {
            sqlx::query(
                &format!("UPDATE users SET plan = $1, plan_expires_at = $2, \
                 quota_total_cents = $3, quota_window_cap_cents = $4, quota_window_cents = LEAST($4, $3), \
                 quota_window_reset_at = now() + interval '{QUOTA_WINDOW_REFRESH}', \
                 quota_weekly_cap_cents = $5, quota_week_used_cents = 0, quota_week_reset_at = now() + interval '7 days', \
                 updated_at = now() WHERE id = $6"),
            )
            .bind(plan)
            .bind(req.expires_at)
            .bind(total)
            .bind(window)
            .bind(weekly)
            .bind(id)
            .execute(&state.db)
            .await?;
        } else {
            // Unknown plan_spec — at least set plan + expiry; quotas left as-is.
            sqlx::query("UPDATE users SET plan = $1, plan_expires_at = $2, updated_at = now() WHERE id = $3")
                .bind(plan)
                .bind(req.expires_at)
                .bind(id)
                .execute(&state.db)
                .await?;
        }
    } else if quota_total_cents(&state, id).await? <= 0 {
        // 不勾"重置额度"本意是"只改标签、别动人家已有的余额"。但用户**本来就没有额度**时，
        // 这条路会发出一个空壳会员：套餐名在、到期时间在、四个额度字段全是 0——界面显示
        // "pro 会员 还有 31 天"，实际一次调用都发不出去（付费门看的是 quota_total > 0）。
        // 线上真出现过一个（gqunzhu@gmail.com：pro / 31 天 / 总额度 0）。
        //
        // 所以这里不是"保持不动"，而是"没有可保持的东西"：按套餐规格发一份，和勾了重置、
        // 也和 Stripe 付款（apply_plan 在 cur=0 时累加出来的正是同一组数）完全一致。
        // 已经有额度的用户仍然一分不动——那才是这个选项存在的意义。
        if let Some((total, window, weekly, _)) = plan_spec(plan) {
            sqlx::query(
                &format!("UPDATE users SET plan = $1, plan_expires_at = $2, \
                 quota_total_cents = $3, quota_window_cap_cents = $4, quota_window_cents = LEAST($4, $3), \
                 quota_window_reset_at = COALESCE(quota_window_reset_at, now() + interval '{QUOTA_WINDOW_REFRESH}'), \
                 quota_weekly_cap_cents = $5, \
                 quota_week_reset_at = COALESCE(quota_week_reset_at, now() + interval '7 days'), \
                 updated_at = now() WHERE id = $6"),
            )
            .bind(plan)
            .bind(req.expires_at)
            .bind(total)
            .bind(window)
            .bind(weekly)
            .bind(id)
            .execute(&state.db)
            .await?;
        } else {
            sqlx::query("UPDATE users SET plan = $1, plan_expires_at = $2, updated_at = now() WHERE id = $3")
                .bind(plan)
                .bind(req.expires_at)
                .bind(id)
                .execute(&state.db)
                .await?;
        }
    } else {
        // Keep existing quotas, just retag the plan + expiry.
        sqlx::query(
            "UPDATE users SET plan = $1, plan_expires_at = $2, updated_at = now() WHERE id = $3",
        )
        .bind(plan)
        .bind(req.expires_at)
        .bind(id)
        .execute(&state.db)
        .await?;
    }
    let summary = user_summary(&state, id).await?;
    crate::realtime::record_event(&state, Some(id), "user_updated", json!({ "by": claims.email, "action": "set_plan", "plan": plan, "expires_at": req.expires_at, "user": summary.clone() })).await;
    Ok(Json(json!({ "ok": true, "user": summary })))
}

// ---------- admin: CANCEL a user's membership ----------
/// Strip a plan back to nothing, in a caller-supplied transaction.
///
/// One definition, deliberately. There are now two ways a plan ends — an operator
/// cancelling it in the console, and Stripe reporting the subscription gone — and if
/// they each carried their own UPDATE they would drift the moment a quota column is
/// added. The Stripe path must also be able to run inside the webhook's transaction,
/// which is why this takes a `Transaction` rather than the pool.
pub async fn clear_plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
) -> ApiResult<()> {
    sqlx::query(
        "UPDATE users SET plan = 'none', plan_expires_at = NULL, \
         quota_total_cents = 0, quota_window_cents = 0, quota_window_cap_cents = 0, \
         quota_weekly_cap_cents = 0, quota_week_used_cents = 0, \
         updated_at = now() WHERE id = $1",
    )
    .bind(uid)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn admin_cancel_plan(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let mut tx = state.db.begin().await?;
    clear_plan(&mut tx, id).await?;
    tx.commit().await?;
    let summary = user_summary(&state, id).await?;
    crate::realtime::record_event(
        &state,
        Some(id),
        "user_updated",
        json!({ "by": claims.email, "action": "cancel_plan", "user": summary.clone() }),
    )
    .await;
    Ok(Json(json!({ "ok": true, "user": summary })))
}

#[cfg(test)]
mod weekly_cap_tests {
    use super::merge_weekly_cap;
    use crate::settings::{swap_plans_for_test, PlanQuota};

    fn plan(name: &str, weekly: i64, rank: i32) -> PlanQuota {
        PlanQuota {
            plan: name.to_string(),
            total_cents: 33_000,
            window_cents: 3_000,
            weekly_cents: weekly,
            days: 30,
            rank,
        }
    }

    /// 周上限这道闸必须能从零立起来。
    ///
    /// 老写法 `if cur_weekly == 0 || weekly == 0 { 0 }` 把建表默认值 0（「还没配过」）
    /// 当成了套餐做出的「不限」选择：第一次发放写回 0，之后每一次发放都再读到 0 再写回 0，
    /// 于是 redeem / Stripe / 支付宝三条路上这个控制项永远是死的，只有 admin_set_plan
    /// 直写才看得到有限值。前两条断言就是那个 bug 的形状。
    #[test]
    fn a_zero_weekly_cap_only_means_unlimited_when_the_held_plan_says_so() {
        // 后台把 basic 配成有限周上限，另有一个规格本身就是不限的套餐。
        let _swap = swap_plans_for_test(vec![
            plan("basic", 5_000, 2),
            plan("unlimited", 0, 5),
            // 一个上限更高的有限档，用来钉「0 不代表可以按新套餐重定上限」（见末尾）。
            plan("big", 30_000, 4),
        ]);

        // ① 全新账号：plan='none'，周上限是建表默认的 0 —— 没人选过「不限」。
        assert_eq!(merge_weekly_cap("none", 0, 5_000), 5_000);
        // ② 存量订阅者：手上是 basic，列里还是历史遗留的 0，而 basic 现在配了有限值。
        //    续费/再发放必须让新配置落地，否则「后台能填」只是个摆设。
        assert_eq!(merge_weekly_cap("basic", 0, 5_000), 5_000);
        // ③ 手上套餐的规格本身就是不限：这个 0 是一次真实选择，不许收紧成有限值。
        assert_eq!(merge_weekly_cap("unlimited", 0, 5_000), 0);
        // ④ 新套餐声明不限：不限照旧赢过任何有限基线（买了更高的套餐不该被旧上限捆住）。
        assert_eq!(merge_weekly_cap("basic", 5_000, 0), 0);
        // ⑤ 两边都有限：取高的，权益不缩水。
        assert_eq!(merge_weekly_cap("basic", 5_000, 10_000), 10_000);
        assert_eq!(merge_weekly_cap("basic", 30_000, 10_000), 30_000);

        // 「这一列还是 0」不等于「可以按新套餐重定上限」。上限是权益，叠加时保留更好的
        // 那个（和上面 cur_weekly > 0 那条分支、以及 apply_plan 里的 window_cap 同一条规矩）。
        // 持有 big（30000）的人因为历史原因这一列还是 0，去兑一张 basic 码（5000）时，
        // 周上限不该被按 basic 定死 —— 那是降级。
        assert_eq!(
            merge_weekly_cap("big", 0, 5_000),
            30_000,
            "持有高档套餐的人兑一张低档码，周上限被降到了低档那一档",
        );
    }

    /// 今天线上六个套餐的 weekly_cents 全是 0（运营的配置选择），这个改动在那份配置下
    /// 必须逐字不改变写入值 —— 修的是控制项失灵，不是现在的额度。
    #[test]
    fn todays_all_zero_configuration_writes_the_same_value_as_before() {
        let _swap = swap_plans_for_test(vec![plan("basic", 0, 2), plan("power", 0, 4)]);
        for cur_plan in ["none", "basic", "power"] {
            assert_eq!(merge_weekly_cap(cur_plan, 0, 0), 0, "{cur_plan}");
        }
    }
}

#[cfg(test)]
mod plan_spec_tests {
    use super::{plan_spec, PLANS};

    /// A zero window_cap is a lockout, not a wider limit.
    ///
    /// The refill is `quota_window_cents = LEAST(quota_window_cap_cents, quota_total_cents)`
    /// (auth.rs + three sites in models.rs) and the serve gate is
    /// `plan_active && q_total > 0 && q_window > 0 && ...`. With a cap of 0 the window
    /// refills to 0, `quota_ok` is false forever, and every member on that plan is refused
    /// with "本时段额度已用完，请等待刷新（每 30 分钟）" — a refresh that can never help.
    ///
    /// This landed once already, on all five plans at once, and would have taken down
    /// every paying account on deploy. The invariant is cheap to assert, so assert it.
    /// 能不能发放，以**后台配置**为准，不是代码里写死的五元组。
    ///
    /// 两条发放路径必须给出同一个答案：Stripe 付款走 apply_plan → plan_spec 查表，查得到
    /// 就照发；后台手动发放曾经查硬编码 PLANS，于是运营在后台新建的套餐（线上真有一个叫
    /// ceshi 的）付款能发、手动发不了，同一份配置两种结果。
    ///
    /// 关键：必须先把配置换成一组**含内置列表之外套餐**的，否则默认配置下两种实现恰好
    /// 等价，测试会两边都过、等于没守。
    #[test]
    fn grantable_follows_the_configured_plans_not_the_builtin_list() {
        // 这条会把进程级 PLANS 换掉，必须和读 PLANS 的那几条串行——串行锁由 swap 自己
        // 持有，并在离开作用域时把表写回，断言中途红了也一样。
        use crate::settings::PlanQuota;
        let custom = PlanQuota {
            plan: "yunying-xinzeng".to_string(),
            total_cents: 12_345,
            window_cents: 1_234,
            weekly_cents: 0,
            days: 30,
            rank: 9,
        };
        let _swap = crate::settings::swap_plans_for_test(vec![custom]);

        // 后台新加的套餐：不在 PLANS 里，但必须可发放——Stripe 就是这么发的。
        assert!(!PLANS.contains(&"yunying-xinzeng"));
        assert!(
            super::plan_is_grantable("yunying-xinzeng"),
            "后台配了额度规格的套餐，手动发放也必须认"
        );
        // 反过来：内置名字被运营从后台删掉后，就不该再能发放。
        assert!(
            !super::plan_is_grantable("pro"),
            "校验必须跟着配置走，不能回落到硬编码列表"
        );
        // 判据与 Stripe 那条完全同源。
        for name in ["yunying-xinzeng", "pro", "nonexistent", ""] {
            assert_eq!(super::plan_is_grantable(name), plan_spec(name).is_some());
        }
        // 拒绝文案要报后台当前的套餐。
        let hint = super::grantable_plans_hint();
        assert!(hint.contains("yunying-xinzeng"), "提示应报当前配置，实际: {hint}");
        assert!(!hint.contains("pro"), "提示不该再报已经不存在的套餐");
    }

    /// 有套餐就必须有额度——「保存套餐」不许发出空壳会员。
    ///
    /// 不勾"重置额度"本意是"别动人家已有的余额"。可用户本来就没有额度时，这条路只改
    /// 套餐名和到期时间，结果是界面显示"pro 会员 还有 31 天"、四个额度字段全是 0，
    /// 一次调用都发不出去（付费门要求 quota_total > 0）。线上真出现过一个。
    ///
    /// 这里对源码断言：那条分支必须先查当前总额度，为 0 时按套餐规格发一份。
    #[test]
    fn saving_a_plan_never_leaves_a_member_with_zero_quota() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/codes.rs"))
            .expect("read codes.rs");
        let start = src
            .find("pub async fn admin_set_plan(")
            .expect("admin_set_plan 必须存在");
        let body: String = src[start..].chars().take(6_000).collect();

        let zero_branch = body
            .find("quota_total_cents(&state, id).await? <= 0")
            .expect("没有额度时必须走补发分支，否则就是空壳会员");
        let retag_only = body
            .find("// Keep existing quotas, just retag the plan + expiry.")
            .expect("只换标签那条分支必须还在——已有额度的用户不该被动");
        assert!(
            zero_branch < retag_only,
            "零额度判断必须排在'只换标签'之前，否则永远走不到"
        );

        // 补发用的必须是套餐规格，而不是随手写的数——要和 Stripe 同源。
        let seg: String = body[zero_branch..].chars().take(1_400).collect();
        assert!(
            seg.contains("plan_spec(plan)"),
            "补发的额度必须来自 plan_spec，才和 Stripe 那条一致"
        );
        assert!(
            seg.contains("quota_total_cents = $3"),
            "补发必须真的写入总额度"
        );
        // 已经有额度的用户，这条路一分都不能动。
        let tail: String = body[retag_only..].chars().take(500).collect();
        assert!(
            !tail.contains("quota_total_cents ="),
            "只换标签的分支不得改写额度"
        );
    }

    #[test]
    fn plan_spec_window_cap_is_never_zero() {
        // 这条测试在 HEAD 上**漏了 `#[test]`**，所以从来没跑过。补上之后它立刻红了，
        // 但红的不是 window_cap —— 是 `plan_spec(plan)` 返回 None。原因是隔壁一批用例
        // 会用 `PlansSwap` 临时换掉 PLANS 表，而这条没有像它的同族
        // `plan_spec_window_cap_never_exceeds_total` 那样先拿串行锁，于是并行跑时读到
        // 的是别人换上去的假表。补锁，不是改断言 —— 断言本身是对的。
        let _g = crate::settings::plans_test_guard();
        for plan in PLANS {
            let (total, window_cap, _weekly, days) =
                plan_spec(plan).unwrap_or_else(|| panic!("{plan} must have a spec"));
            assert!(
                window_cap > 0,
                "{plan}: window_cap must be > 0 — a 0 cap refills the window to \
                 LEAST(0, total) = 0 and permanently locks the plan out, it does NOT \
                 disable the window. Rework quota_ok before trying weekly-primary quotas."
            );
            assert!(total > 0, "{plan}: total must be > 0");
            assert!(days > 0, "{plan}: duration must be > 0 days");
        }
    }

    /// The window is a per-30-minute slice of the total, so a cap above the total is a
    /// typo — LEAST() would silently clamp it and the advertised figure would be fiction.
    #[test]
    fn plan_spec_window_cap_never_exceeds_total() {
        let _g = crate::settings::plans_test_guard();
        for plan in PLANS {
            let (total, window_cap, _weekly, _days) = plan_spec(plan).unwrap();
            assert!(
                window_cap <= total,
                "{plan}: window_cap {window_cap} exceeds total {total}; LEAST() would clamp it"
            );
        }
    }

    /// Grants must never silently downgrade: rank and price must move together.
    #[test]
    fn plan_spec_totals_increase_with_rank() {
        let _g = crate::settings::plans_test_guard();
        let mut prev = 0i64;
        for plan in PLANS {
            let (total, ..) = plan_spec(plan).unwrap();
            assert!(total > prev, "{plan}: total {total} must exceed previous {prev}");
            prev = total;
        }
    }

    #[test]
    fn plan_spec_rejects_unknown_plans() {
        assert!(plan_spec("none").is_none());
        assert!(plan_spec("").is_none());
        assert!(plan_spec("PRO").is_none(), "plan matching is case-sensitive");
    }
}
