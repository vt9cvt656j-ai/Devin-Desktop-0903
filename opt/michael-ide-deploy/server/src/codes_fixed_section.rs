// FIX: Replaced window-based quota system with weekly cap mechanism
// Original buggy version gave users only 9.2% of purchased value due to 5h30m window error
pub(crate) fn plan_spec(plan: &str) -> Option<(i64, i64, i64, i32)> {
    match plan {
        "trial" => Some((5_000, 0, 500, 1)),      // (total_cents, window_cap=0禁用，weekly_cap_cents, duration_days)
        "basic" => Some((33_000, 0, 5_000, 30)),  // $330 total, $50/week cap, 30 days, ¥88
        "pro" => Some((65_000, 0, 10_000, 30)),  // $650 total, $100/week cap, 30 days, ¥188
        "power" => Some((180_000, 0, 30_000, 30)), // $1800 total, $300/week cap, 30 days, ¥488
        "ultra" => Some((500_000, 0, 80_000, 30)), // $5000 total, $800/week cap, 30 days
        _ => None,
    }
}

/// Ordering of the built-in plans, so a grant never silently downgrades a user who
/// still holds a better plan. Unknown/absent plans rank 0.
pub(crate) fn plan_rank(plan: &str) -> i32 {
    match plan {
        "trial" => 1,
        "basic" => 2,
        "pro" => 3,
        "power" => 4,
        "ultra" => 5,
        _ => 0,
    }
}

pub(crate) async fn apply_plan(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    uid: uuid::Uuid,
    plan: &str,
    days: i32,
) -> ApiResult<()> {
    if let Some((total, _window_disabled, weekly, spec_days)) = plan_spec(plan) {
        let dur = if days > 0 { days } else { spec_days };
        // Read the current state under a row lock: the new pools are derived from the
        // old ones, and two concurrent redeems must not both compute from the same base.
        let (cur_plan, cur_total, _old_window_cap, cur_window, cur_weekly): (
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
        // Don't reset window caps anymore - we're deprecating this mechanism
        let new_window = cur_window.max(0).min(new_total);  // Just keep what's available
        // 0 means "no weekly cap", so it wins over any finite cap.
        let new_weekly = if cur_weekly == 0 || weekly == 0 {
            0
        } else {
            cur_weekly.max(weekly)
        };
        let new_plan = if plan_rank(plan) >= plan_rank(&cur_plan) {
            plan.to_string()
        } else {
            cur_plan
        };

        // Note: We NO LONGER reset quota_window_reset_at - removing the buggy 5h30m logic
        sqlx::query(
            "UPDATE users SET plan = $1, \
             plan_expires_at = GREATEST(COALESCE(plan_expires_at, now()), now()) + ($2 * interval '1 day'), \
             quota_total_cents = $3, quota_window_cents = $4, \
             quota_weekly_cap_cents = $5, \
             updated_at = now() WHERE id = $6",
        )
        .bind(&new_plan)
        .bind(dur)
        .bind(new_total)
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
