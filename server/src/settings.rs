//! 运营参数的唯一真相。
//!
//! 在这个模块之前，三个数字散在四个文件里：`models.rs` 的面值分母 6.63、
//! `models.rs` 的 `FREE_POINTS_DAILY`、`codes.rs` 的 `plan_spec`；面值分母另有三份
//! 前端副本（`Customers.tsx`、`Billing.tsx`、`static/admin.html`）和一份客户端副本
//! （`ide/src/main.js`）。其中三份在写路径上——管理员输入的美元由前端乘 663 变成
//! 存库的真实分，服务端不做二次换算。只把其中一处改成可配置，等于让"发出去多少"
//! 和"显示多少"当场对不上，而且对不上的地方没有任何报错。
//!
//! 所以这里的规则是：数据库是唯一定义，Rust 启动时读一次进内存，写入后失效重载，
//! 所有前端从接口取值。任何一处再出现字面量 663 都是 bug。
//!
//! 为什么用内存缓存而不是每次查库：面值分母被展示路径大量使用，套餐额度在发放路径
//! 上使用，而中转请求本身已经有三次同步数据库往返。多一次 SELECT 不会压垮它，但也
//! 没有必要——这些值一天改不了一次。缓存用的是本仓库既有的写法
//! （`update.rs`、`knowledge.rs`、`agent_trace.rs` 都是 `LazyLock<RwLock<..>>` 静态量）。

use std::sync::{LazyLock, RwLock};

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// 面值分母的合法区间，与迁移里的 CHECK 一致。下限是 1 而不是 0：这个数在展示路径上
/// 做除数，0 会把每一个余额变成除零。
pub const MIN_RAW_CENTS_PER_CREDIT_USD: i64 = 1;
pub const MAX_RAW_CENTS_PER_CREDIT_USD: i64 = 100_000;
pub const MIN_FREE_POINTS_DAILY: i64 = 0;
pub const MAX_FREE_POINTS_DAILY: i64 = 1_000_000;

/// 兜底值。逐字对应落库前 `models.rs` 里的常量，所以在迁移跑完之前（或数据库暂时读
/// 不到时）读到的行为与改造前完全一致——这次改造本身不改变任何金额。
pub const DEFAULT_RAW_CENTS_PER_CREDIT_USD: i64 = 663;
pub const DEFAULT_FREE_POINTS_DAILY: i64 = 40;
/// 7.10 CNY/USD。只用于把人民币销售折成美元记佣金，不参与任何展示。
pub const DEFAULT_USD_PER_CNY_BPS: i64 = 1408;
pub const MIN_USD_PER_CNY_BPS: i64 = 100;
pub const MAX_USD_PER_CNY_BPS: i64 = 10_000;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Settings {
    pub raw_cents_per_credit_usd: i64,
    pub free_points_daily: i64,
    /// 1 人民币分 折合多少美元分，万分比。佣金账本以美元计量，人民币销售在记账时折一次。
    pub usd_per_cny_bps: i64,
    /// 缓存计费「便宜模式」。默认 false = 按真实价（缓存写照收贵的那笔）。
    /// true = 灰产/便宜渠道用：把最贵的缓存写入降到和缓存读同一个便宜价，普通输入照常。
    /// 语义见 compute_cost 里的用法：cheap 时 write_price = read_price。
    pub cache_billing_cheap: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            raw_cents_per_credit_usd: DEFAULT_RAW_CENTS_PER_CREDIT_USD,
            free_points_daily: DEFAULT_FREE_POINTS_DAILY,
            usd_per_cny_bps: DEFAULT_USD_PER_CNY_BPS,
            cache_billing_cheap: false,
        }
    }
}

/// 套餐额度。字段单位与 `users.quota_*_cents` 一致：真实计费分。
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PlanQuota {
    pub plan: String,
    pub total_cents: i64,
    pub window_cents: i64,
    pub weekly_cents: i64,
    pub days: i32,
    pub rank: i32,
}

/// 兜底套餐表，逐字对应改造前的 `plan_spec`。
fn default_plans() -> Vec<PlanQuota> {
    [
        ("trial", 5_000, 5_000, 500, 1, 1),
        ("basic", 33_000, 3_000, 5_000, 30, 2),
        ("pro", 65_000, 6_000, 10_000, 30, 3),
        ("power", 180_000, 15_000, 30_000, 30, 4),
        ("ultra", 500_000, 30_000, 80_000, 30, 5),
    ]
    .into_iter()
    .map(
        |(plan, total_cents, window_cents, weekly_cents, days, rank)| PlanQuota {
            plan: plan.to_string(),
            total_cents,
            window_cents,
            weekly_cents,
            days,
            rank,
        },
    )
    .collect()
}

static CACHE: LazyLock<RwLock<Settings>> = LazyLock::new(|| RwLock::new(Settings::default()));
static PLANS: LazyLock<RwLock<Vec<PlanQuota>>> = LazyLock::new(|| RwLock::new(default_plans()));

/// 测试专用：把套餐缓存换成给定的一组，返回原来的那组以便还原。
///
/// 存在的理由很具体：`plan_quotas` 是运营在后台加套餐的地方，而校验一度查的是代码里
/// 写死的五元组。两者在**默认配置下恰好相等**，所以任何不改这份缓存的测试都无法区分
/// 「按配置校验」和「按硬编码校验」——测试会两种实现都通过，等于没守。
/// 换过 PLANS 的测试和读 PLANS 的测试必须串行。
///
/// PLANS 是**进程级**缓存，而 cargo 默认并行跑测试：一个测试把它换成一组只含
/// "yunying-xinzeng" 的假配置期间，另外几个正在读真配置的测试就会 unwrap 到 None。
/// 症状是这几条时红时绿、每次红的还不一定是同一条——最难查的那种。
/// 中毒了也照用（`into_inner`）：一次 panic 不该让后面每条都变成"锁中毒"。
#[cfg(test)]
pub static PLANS_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub fn plans_test_guard() -> std::sync::MutexGuard<'static, ()> {
    PLANS_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
pub fn replace_plans_for_test(plans: Vec<PlanQuota>) -> Vec<PlanQuota> {
    let mut guard = PLANS.write().expect("plans lock");
    std::mem::replace(&mut *guard, plans)
}

/// 读缓存。锁中毒时退回默认值而不是 panic——这些是展示与发放参数，让整个网关因为一把
/// 读锁挂掉是更坏的结果。
pub fn current() -> Settings {
    CACHE.read().map(|g| *g).unwrap_or_default()
}

/// 面值分母：多少真实计费分等于客户看到的 $1.00 额度。
/// 缓存计费是否走「便宜模式」（灰产渠道用）。true = 缓存写降到缓存读价。
pub fn cache_billing_cheap() -> bool {
    CACHE.read().map(|g| g.cache_billing_cheap).unwrap_or(false)
}

pub fn raw_cents_per_credit_usd() -> i64 {
    current()
        .raw_cents_per_credit_usd
        .clamp(MIN_RAW_CENTS_PER_CREDIT_USD, MAX_RAW_CENTS_PER_CREDIT_USD)
}

/// 1 人民币分 折合多少美元分，万分比。夹在合理区间内：这个数是佣金的乘数，
/// 一个离谱的值会静默地把每一笔中国销售的佣金放大或抹平。
pub fn usd_per_cny_bps() -> i64 {
    current()
        .usd_per_cny_bps
        .clamp(MIN_USD_PER_CNY_BPS, MAX_USD_PER_CNY_BPS)
}

/// 同一个数的浮点形式，供利润测算用（`models.rs` 原先的 6.63）。
pub fn raw_usd_per_visible_usd() -> f64 {
    raw_cents_per_credit_usd() as f64 / 100.0
}

/// 每日赠送点数（整点）。
pub fn free_points_daily() -> i64 {
    current()
        .free_points_daily
        .clamp(MIN_FREE_POINTS_DAILY, MAX_FREE_POINTS_DAILY)
}

/// 每日赠送的毫点。池子按毫点存储，见 `models.rs::MILLI` 的说明。
pub fn free_milli_points_daily() -> i64 {
    free_points_daily() * crate::models::MILLI
}

pub fn plans() -> Vec<PlanQuota> {
    PLANS
        .read()
        .map(|g| g.clone())
        .unwrap_or_else(|_| default_plans())
}

/// 与改造前的 `codes::plan_spec` 同签名同语义：(总额度, 时段上限, 周上限, 天数)。
pub fn plan_spec(plan: &str) -> Option<(i64, i64, i64, i32)> {
    plans()
        .into_iter()
        .find(|p| p.plan == plan)
        .map(|p| (p.total_cents, p.window_cents, p.weekly_cents, p.days))
}

/// 套餐高低次序，未知套餐为 0。
pub fn plan_rank(plan: &str) -> i32 {
    plans()
        .into_iter()
        .find(|p| p.plan == plan)
        .map(|p| p.rank)
        .unwrap_or(0)
}

/// 从数据库装载进缓存。启动时调用一次，每次写入后再调用一次。
/// 读不到就保留当前缓存（首次即默认值），不让网关起不来。
pub async fn load(db: &sqlx::PgPool) {
    match sqlx::query_as::<_, (i32, i32, i32, bool)>(
        "SELECT raw_cents_per_credit_usd, free_points_daily, usd_per_cny_bps, \
                COALESCE(cache_billing_cheap, false) \
         FROM app_settings WHERE id = 1",
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some((raw, free, fx, cheap))) => {
            let next = Settings {
                raw_cents_per_credit_usd: (raw as i64)
                    .clamp(MIN_RAW_CENTS_PER_CREDIT_USD, MAX_RAW_CENTS_PER_CREDIT_USD),
                free_points_daily: (free as i64)
                    .clamp(MIN_FREE_POINTS_DAILY, MAX_FREE_POINTS_DAILY),
                usd_per_cny_bps: (fx as i64).clamp(MIN_USD_PER_CNY_BPS, MAX_USD_PER_CNY_BPS),
                cache_billing_cheap: cheap,
            };
            if let Ok(mut g) = CACHE.write() {
                *g = next;
            }
        }
        Ok(None) => tracing::warn!("app_settings 无数据行，沿用默认值"),
        Err(e) => tracing::warn!("app_settings 读取失败，沿用当前值: {e}"),
    }

    match sqlx::query_as::<_, PlanQuota>(
        "SELECT plan, total_cents, window_cents, weekly_cents, days, rank \
         FROM plan_quotas ORDER BY rank",
    )
    .fetch_all(db)
    .await
    {
        Ok(rows) if !rows.is_empty() => {
            if let Ok(mut g) = PLANS.write() {
                *g = rows;
            }
        }
        Ok(_) => tracing::warn!("plan_quotas 为空，沿用默认套餐"),
        Err(e) => tracing::warn!("plan_quotas 读取失败，沿用当前值: {e}"),
    }
}

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// 套餐列表，外加一个 `is_default`：这一行是否还等于代码里的出厂值。
///
/// 加这个字段是因为一次真实的误会：设置页把种子值填进可编辑输入框里，看上去就像是
/// 运营自己配过的数字，于是「我从没设过这么高的值」变成了一句无法自证的话。标出哪些
/// 行没人动过，这个问题就不会再出现。
fn plans_json() -> Vec<serde_json::Value> {
    let defaults = default_plans();
    plans()
        .into_iter()
        .map(|p| {
            let is_default = defaults.iter().any(|d| {
                d.plan == p.plan
                    && d.total_cents == p.total_cents
                    && d.window_cents == p.window_cents
                    && d.weekly_cents == p.weekly_cents
                    && d.days == p.days
            });
            json!({
                "plan": p.plan,
                "total_cents": p.total_cents,
                "window_cents": p.window_cents,
                "weekly_cents": p.weekly_cents,
                "days": p.days,
                "rank": p.rank,
                "is_default": is_default,
            })
        })
        .collect()
}

pub async fn admin_get(claims: Claims) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let s = current();
    Ok(Json(json!({
        "raw_cents_per_credit_usd": s.raw_cents_per_credit_usd,
        "free_points_daily": s.free_points_daily,
        "plans": plans_json(),
        "limits": {
            "raw_cents_per_credit_usd": [MIN_RAW_CENTS_PER_CREDIT_USD, MAX_RAW_CENTS_PER_CREDIT_USD],
            "free_points_daily": [MIN_FREE_POINTS_DAILY, MAX_FREE_POINTS_DAILY],
        },
        // 只读：由 663 与 ¥7.2/点价 ¥0.05 手工推导，是编译期常量，不在这一屏改。
        "raw_cents_per_point": crate::models::RAW_CENTS_PER_POINT,
    })))
}

#[derive(Debug, Deserialize)]
pub struct PlanPatch {
    pub plan: String,
    pub total_cents: i64,
    pub window_cents: i64,
    pub weekly_cents: i64,
    pub days: i32,
}

#[derive(Debug, Deserialize)]
pub struct SettingsPatch {
    pub raw_cents_per_credit_usd: Option<i64>,
    pub free_points_daily: Option<i64>,
    pub cache_billing_cheap: Option<bool>,
    pub plans: Option<Vec<PlanPatch>>,
}

pub async fn admin_put(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<SettingsPatch>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    // 先把范围校验做在这里，让管理员看到中文原因，而不是数据库 CHECK 冒出来的 500。
    if let Some(v) = req.raw_cents_per_credit_usd {
        if !(MIN_RAW_CENTS_PER_CREDIT_USD..=MAX_RAW_CENTS_PER_CREDIT_USD).contains(&v) {
            return Err(AppError::bad(format!(
                "面值分母需在 {MIN_RAW_CENTS_PER_CREDIT_USD}~{MAX_RAW_CENTS_PER_CREDIT_USD} 之间"
            )));
        }
    }
    if let Some(v) = req.free_points_daily {
        if !(MIN_FREE_POINTS_DAILY..=MAX_FREE_POINTS_DAILY).contains(&v) {
            return Err(AppError::bad(format!(
                "每日赠送需在 {MIN_FREE_POINTS_DAILY}~{MAX_FREE_POINTS_DAILY} 之间"
            )));
        }
    }
    if let Some(list) = &req.plans {
        for p in list {
            if p.total_cents < 0 || p.weekly_cents < 0 {
                return Err(AppError::bad(format!("{}：额度不能为负", p.plan)));
            }
            // 0 时段上限不是"不限"，是把套餐锁死——quota_ok 要求 q_window > 0。
            if p.window_cents <= 0 {
                return Err(AppError::bad(format!(
                    "{}：时段上限必须大于 0，填 0 会让这个套餐永远刷不出额度",
                    p.plan
                )));
            }
            if p.days <= 0 {
                return Err(AppError::bad(format!("{}：有效天数必须大于 0", p.plan)));
            }
        }
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| AppError::internal(format!("开启事务失败: {e}")))?;

    if req.raw_cents_per_credit_usd.is_some() || req.free_points_daily.is_some()
        || req.cache_billing_cheap.is_some()
    {
        sqlx::query(
            "UPDATE app_settings SET \
               raw_cents_per_credit_usd = COALESCE($1, raw_cents_per_credit_usd), \
               free_points_daily = COALESCE($2, free_points_daily), \
               cache_billing_cheap = COALESCE($4, cache_billing_cheap), \
               updated_at = now(), updated_by = $3 \
             WHERE id = 1",
        )
        .bind(req.raw_cents_per_credit_usd.map(|v| v as i32))
        .bind(req.free_points_daily.map(|v| v as i32))
        .bind(&claims.sub)
        .bind(req.cache_billing_cheap)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::internal(format!("写入设置失败: {e}")))?;
    }

    if let Some(list) = &req.plans {
        for p in list {
            // 只更新既有套餐，不新增：套餐名散在兑换码、订单、compression 分级里，
            // 从这一屏凭空造一个新名字出来只会得到一个没人认识的套餐。
            sqlx::query(
                "UPDATE plan_quotas SET total_cents = $2, window_cents = $3, \
                   weekly_cents = $4, days = $5, updated_at = now() WHERE plan = $1",
            )
            .bind(&p.plan)
            .bind(p.total_cents)
            .bind(p.window_cents)
            .bind(p.weekly_cents)
            .bind(p.days)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::internal(format!("写入套餐 {} 失败: {e}", p.plan)))?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| AppError::internal(format!("提交失败: {e}")))?;

    load(&state.db).await;

    let s = current();
    tracing::info!(
        admin = %claims.sub,
        raw_cents_per_credit_usd = s.raw_cents_per_credit_usd,
        free_points_daily = s.free_points_daily,
        "运营参数已更新"
    );

    Ok(Json(json!({
        "ok": true,
        "raw_cents_per_credit_usd": s.raw_cents_per_credit_usd,
        "free_points_daily": s.free_points_daily,
        "plans": plans_json(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 兜底值必须逐字等于改造前 models.rs / codes.rs 里的常量，否则"迁移前后行为不变"
    /// 这句话就是假的。
    ///
    /// weekly_cents 是迁移之后才加的周上限，不在"行为不变"这句话的范围里，所以它钉的是
    /// 这一列自己该有的值。这个测试之前一直红着：周上限上线时没有人回来改这里，于是
    /// 它从"迁移守卫"变成了"套餐表被人动过就报错"的噪音——两个失败的测试摆在那里，
    /// 只会教人把整个套件的红色当背景。走库里的 plans 表时这些值用不上；它们是查库
    /// 失败时的兜底，所以必须是真能收费的数字，不能是 0。
    #[test]
    fn defaults_match_the_constants_they_replaced() {
        assert_eq!(DEFAULT_RAW_CENTS_PER_CREDIT_USD, 663);
        assert_eq!(DEFAULT_FREE_POINTS_DAILY, 40);
        assert_eq!(
            default_plans()
                .into_iter()
                .map(|p| (p.plan, p.total_cents, p.window_cents, p.weekly_cents, p.days))
                .collect::<Vec<_>>(),
            vec![
                ("trial".to_string(), 5_000, 5_000, 500, 1),
                ("basic".to_string(), 33_000, 3_000, 5_000, 30),
                ("pro".to_string(), 65_000, 6_000, 10_000, 30),
                ("power".to_string(), 180_000, 15_000, 30_000, 30),
                ("ultra".to_string(), 500_000, 30_000, 80_000, 30),
            ]
        );
    }

    /// 每一个兜底套餐的时段上限都必须 > 0。0 会让 quota_ok 永远为假，用户被锁在一个
    /// 刷不出额度的提示里——这是 plan_spec 原注释专门警告过的事。
    #[test]
    fn default_plan_window_caps_are_never_zero() {
        for p in default_plans() {
            assert!(p.window_cents > 0, "{} 的时段上限为 0", p.plan);
        }
    }

    /// 面值分母永远不会返回 0，即使缓存被写进了非法值——它在展示路径上是除数。
    #[test]
    fn denominator_is_never_zero() {
        if let Ok(mut g) = CACHE.write() {
            g.raw_cents_per_credit_usd = 0;
        }
        assert!(raw_cents_per_credit_usd() >= MIN_RAW_CENTS_PER_CREDIT_USD);
        assert!(raw_usd_per_visible_usd() > 0.0);
        if let Ok(mut g) = CACHE.write() {
            *g = Settings::default();
        }
    }

    /// 面值分母只能有一个真相。
    ///
    /// 这个数曾经在四个文件里各有一份副本，其中三份在写路径上——管理员输入的美元乘以
    /// 它变成存库的真实分，服务端不做二次换算。任何一份重新变回硬编码，"发出去多少额度"
    /// 就会和"显示多少"对不上，而且不会有任何报错，只能靠对账发现。
    ///
    /// 所以这里逐个文件检查两件事：拿到了服务端下发的值，且没有自己声明一份常量。
    #[test]
    fn the_credit_denominator_has_exactly_one_source() {
        // 第四份副本原本在 static/admin.html 里。那个旧后台已经整个删掉了 —— 它是第二套
        // 能改余额的界面，对匿名访客公开，而且面值分母在里面又是一份独立硬编码。删掉比
        // 接上服务端更彻底，所以这里连带钉死"它不能回来"。
        assert!(
            !std::path::Path::new("static/admin.html").exists(),
            "旧管理后台 static/admin.html 又出现了：它是第二套能改余额的界面，\
             面值分母在里面是独立的一份，留着就迟早和服务端对不上"
        );

        // (文件, 必须存在的"从服务端取值"的写法)
        let wired = [
            ("admin-ui/src/lib/settings.ts", "raw_cents_per_credit_usd"),
            ("admin-ui/src/pages/Customers.tsx", "creditCentsFromRaw"),
            ("admin-ui/src/pages/Billing.tsx", "creditCentsFromRaw"),
            ("../ide/src/main.js", "_setCreditDenominator(_michaelUser"),
        ];
        for (path, needle) in wired {
            let src = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("读不到 {path}: {e}"));
            assert!(
                src.contains(needle),
                "{path} 不再从服务端取面值分母（缺少 `{needle}`）"
            );
        }

        // 只有明确标注为兜底的那一行可以出现字面量；其余一律不许再声明常量。
        // 这些文件里连兜底都不该有——它们全部经由 lib/settings.ts 取值。
        for path in [
            "admin-ui/src/pages/Customers.tsx",
            "admin-ui/src/pages/Billing.tsx",
        ] {
            let src = std::fs::read_to_string(path).unwrap();
            for (i, line) in src.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                assert!(
                    !code.contains("= 663"),
                    "{path}:{} 又硬编码了一份面值分母：{}",
                    i + 1,
                    line.trim()
                );
            }
        }
    }

    /// 未知套餐既没有额度也没有等级，和改造前一致。
    #[test]
    fn unknown_plan_has_no_spec_and_rank_zero() {
        let _g = super::plans_test_guard();
        assert!(plan_spec("no-such-plan").is_none());
        assert_eq!(plan_rank("no-such-plan"), 0);
        assert_eq!(plan_rank("basic"), 2);
        // 第三位是周上限，0 表示不限；basic 是 5_000。codes.rs 的 0 == 不限那条规则，
        // 意味着把它写回 0 不会让测试失败，只会悄悄给所有 basic 用户解除周上限。
        assert_eq!(plan_spec("basic"), Some((33_000, 3_000, 5_000, 30)));
    }
}
