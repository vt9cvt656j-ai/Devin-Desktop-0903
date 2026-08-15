//! Referrals: an invite code, a link, and a share of what the people you bring in spend.
//!
//! The `commissions` ledger predates this module and stays exactly as it was — a row is
//! still an auditable claim an admin can settle or reject. What was missing was everything
//! that fills it: a code to give out, a record of who used it, and a hook on the payment
//! path that turns a payment into a claim. An admin creating rows by hand is a fallback,
//! not a referral programme.
//!
//! **Terms are frozen when a referral is claimed, not read at payout.** Someone shared
//! their link on the promise of 30% for three months; cutting the rate next month must not
//! reach backwards and rewrite what they were already owed. Changing the settings changes
//! the deal for referrals made afterwards and for nobody else — which is also why the admin
//! screen says so rather than leaving it to be discovered.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

use sqlx::Acquire;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::field_crypto;
use crate::AppState;

/// 落库加密的 context（= 列身份，绑进 AAD）。见 field_crypto.rs。提现账户和收款码是
/// 支付路由信息，拖库即泄露；它们从不按值查，只回显给本人和管理员，适合随机加密。
const WD_ACCOUNT_CTX: &str = "withdrawals.account";
const WD_QR_CTX: &str = "withdrawals.qr";

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// Code alphabet, minus everything that gets misread aloud or off a screenshot.
///
/// No 0/O, no 1/I/L. These codes are read out on calls, typed off phone photos and pasted
/// with the wrong case; every confusable pair removed here is a support message that never
/// gets sent. Digits 2-9 and 22 letters give 30^8 — collisions are not the constraint.
const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
const CODE_LEN: usize = 8;

fn new_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..CODE_LEN)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

/// The terms in force right now, for referrals claimed from here on.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Terms {
    pub rate_bps: i32,
    pub window_days: i32,
    pub enabled: bool,
    /// true = settle on the spot as account credit; false = raise it pending for a human.
    pub auto_settle: bool,
    /// 冻结期天数：佣金审核通过之后，要等这么久才允许进打款批次。挡的是退款和拒付。
    pub hold_days: i32,
    /// 提现门槛，分。同一个推荐人攒够这个数才发一笔，免得手续费吃掉小额佣金。
    pub min_payout_cents: i64,
    /// 定时批量打款的总开关。关着的时候，提现仍然是用户自己点、系统当场转。
    pub batch_enabled: bool,
}

impl Default for Terms {
    fn default() -> Self {
        // Matches the migration's defaults. Used only if the settings row cannot be read,
        // where refusing to attribute at all would lose referrals silently.
        // batch_enabled 独自例外：其他几项猜错只是少记一笔佣金，这一项猜错会让服务器
        // 在没人授意的情况下自己往外转钱。所以它永远默认关。
        Terms {
            rate_bps: 3000,
            window_days: 90,
            enabled: true,
            auto_settle: false,
            hold_days: 14,
            min_payout_cents: 5000,
            batch_enabled: false,
        }
    }
}

pub async fn terms(db: &sqlx::PgPool) -> Terms {
    sqlx::query_as::<_, (i32, i32, bool, bool, i32, i64, bool)>(
        "SELECT referral_rate_bps, referral_window_days, referral_enabled, referral_auto_settle, \
                referral_hold_days, referral_min_payout_cents, referral_batch_enabled \
         FROM app_settings WHERE id = 1",
    )
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
    .map(
        |(rate_bps, window_days, enabled, auto_settle, hold_days, min_payout_cents, batch_enabled)| {
            Terms {
                rate_bps,
                window_days,
                enabled,
                auto_settle,
                hold_days,
                min_payout_cents,
                batch_enabled,
            }
        },
    )
    .unwrap_or_default()
}

// ---------------------------------------------------------------------------------------
// The customer's side
// ---------------------------------------------------------------------------------------

/// This account's code, or None if it has not been assigned one.
///
/// Its own function because the read happens twice — once up front, once after losing the
/// race to insert — and the two copies were where the bug lived: both spelled `$1` and
/// neither bound anything, so every call died with "bind message supplies 0 parameters,
/// but prepared statement requires 1". Granting a privilege and opening the referral page
/// both went through here, so both were 500s. One copy cannot disagree with itself.
async fn read_code(state: &AppState, uid: uuid::Uuid) -> ApiResult<Option<String>> {
    Ok(sqlx::query_scalar::<_, Option<String>>("SELECT referral_code FROM users WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .flatten())
}

/// This account's code, minted on first look.
///
/// Lazily rather than at sign-up so accounts that never open the screen never take a code,
/// and so this did not need a backfill over every existing user. The loop retries on the
/// unique index rather than checking first: two people opening the screen at the same
/// moment would both pass a check and one insert would still lose.
async fn code_for(state: &AppState, uid: uuid::Uuid) -> ApiResult<String> {
    if let Some(existing) = read_code(state, uid).await? {
        return Ok(existing);
    }

    for _ in 0..6 {
        let candidate = new_code();
        let done = sqlx::query(
            "UPDATE users SET referral_code = $2 WHERE id = $1 AND referral_code IS NULL",
        )
        .bind(uid)
        .bind(&candidate)
        .execute(&state.db)
        .await;
        match done {
            Ok(r) if r.rows_affected() > 0 => return Ok(candidate),
            // Either somebody else assigned this account a code first, or the candidate
            // collided. Re-read: the first case is already finished.
            _ => {
                if let Some(existing) = read_code(state, uid).await? {
                    return Ok(existing);
                }
            }
        }
    }
    Err(AppError::internal("邀请码生成失败，请重试"))
}

/// `GET /api/referral/me` — my code, my link, and what it has earned.
pub async fn me(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let t = terms(&state.db).await;

    // Not an error — "you are not in the programme" is a normal answer, and the account
    // page needs to render something for it. Returning 403 would make an ordinary state
    // look like a fault.
    let granted: bool = sqlx::query_scalar("SELECT referral_enabled FROM users WHERE id = $1")
        .bind(uid)
        .fetch_optional(&state.db)
        .await?
        .unwrap_or(false);
    if !granted {
        return Ok(Json(json!({
            "granted": false,
            "auto_settle": t.auto_settle,
            // 自动打款开着时，用户端要把「提现」入口藏掉：钱由系统按冻结期和门槛自己转，
            // 没有可提的动作。开户入口不在那一页，所以藏掉它不会挡住收款账户的绑定。
            "batch_enabled": t.batch_enabled,
            "pending_withdrawals": 0,
            "rate_bps": t.rate_bps,
            "window_days": t.window_days,
            "enabled": t.enabled,
        })));
    }

    // Minted only now, so an account that was never granted never takes a code.
    let code = code_for(&state, uid).await?;

    let invited: i64 =
        sqlx::query_scalar("SELECT count(*) FROM referrals WHERE referrer_user_id = $1")
            .bind(uid)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    // Split by status so "earned" never quietly includes claims that were rejected.
    let (pending, settled): (i64, i64) = sqlx::query_as(
        "SELECT \
           COALESCE(SUM(commission_cents) FILTER (WHERE status = 'pending'), 0)::bigint, \
           COALESCE(SUM(commission_cents) FILTER (WHERE status = 'settled'), 0)::bigint \
         FROM commissions WHERE referrer_user_id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or((0, 0));

    // Same rule as the admin nav: no withdrawal screen under automatic settlement, unless
    // this person has a request still outstanding from before the switch.
    let my_pending: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM withdrawals WHERE user_id = $1 AND status = 'pending'",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(json!({
        "granted": true,
        "auto_settle": t.auto_settle,
            // 自动打款开着时，用户端要把「提现」入口藏掉：钱由系统按冻结期和门槛自己转，
            // 没有可提的动作。开户入口不在那一页，所以藏掉它不会挡住收款账户的绑定。
            "batch_enabled": t.batch_enabled,
        "pending_withdrawals": my_pending,
        "code": code,
        "link": format!("{}/gate?ref={}", state.cfg.public_base.trim_end_matches('/'), code),
        "rate_bps": t.rate_bps,
        "window_days": t.window_days,
        "enabled": t.enabled,
        "invited": invited,
        "pending_cents": pending,
        "settled_cents": settled,
    })))
}

#[derive(Deserialize)]
pub struct ClaimReq {
    pub code: String,
}

/// `POST /api/referral/claim` — attach a referrer to this account.
///
/// Called by the sign-in page after registration, using the `ref` it kept from the URL.
/// Separate from registration rather than a parameter on it because there are three ways
/// into an account — password, GitHub, Google — and a parameter would have to be threaded
/// through all three and kept across a provider round trip.
///
/// Five things it refuses, each of them a way to be paid for nothing: referring yourself,
/// being referred twice, an unknown code, attaching a referrer to an account that has
/// already bought something, and — the one the product rule turns on — attaching one to an
/// account that is not newly registered.
pub async fn claim(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<ClaimReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;
    let t = terms(&state.db).await;
    if !t.enabled {
        return Err(AppError::bad("推荐计划当前未开放"));
    }

    let code = req.code.trim().to_string();
    if code.is_empty() {
        return Err(AppError::bad("请填写邀请码"));
    }

    // `AND referral_enabled` rather than a second query: a code belonging to an account
    // whose privilege was withdrawn must read as an invalid code, not as a different kind
    // of refusal that tells a stranger something about somebody else's account.
    let referrer: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE lower(referral_code) = lower($1) AND referral_enabled",
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await?;
    let Some(referrer) = referrer else {
        return Err(AppError::bad("邀请码无效"));
    };
    if referrer == uid {
        return Err(AppError::bad("不能使用自己的邀请码"));
    }

    /*
     * 只有**新注册**的账号能绑推荐人。
     *
     * 规则是产品定的：已经有账号的人拿一个邀请码来登录，不算谁的推荐用户。这条闸挡住的
     * 是两种真实情况：
     *
     *   · 共用电脑 / 老用户帮朋友点了一下邀请链接 —— gate 在跳转前就把码存下了，而绑定
     *     只看「有没有令牌」，于是这个码被烧在一个老账号上，朋友的推荐永久没了
     *     （推荐人名额只有一个，绑过就不能改）；
     *   · 老用户自己找个码绑上，接下来的消费凭空给别人分成。
     *
     * 用注册时间而不是「有没有付过款」：后者只挡住已经花过钱的人，一个注册半年、一分没花
     * 的老账号照样能绑。窗口给到 24 小时而不是几分钟 —— 正常注册流程里绑定发生在几秒内，
     * 但从桌面端注册、第二天才打开网页控制台的人也该算数。
     */
    // 让 SQL 直接给一个布尔，而不是取出秒数再在 Rust 里算：EXTRACT 在 PG14+ 返回
    // NUMERIC，取成 f64 会在运行时解码失败 —— 编译期没有任何提示，只有真调用到才 500。
    let is_new: Option<bool> = sqlx::query_scalar(
        "SELECT created_at > now() - interval '24 hours' FROM users WHERE id = $1",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await?
    .flatten();
    if !is_new.unwrap_or(false) {
        return Err(AppError::bad("邀请码只能在注册时使用，已有账号无法再绑定邀请人"));
    }

    // 已经付过款的更不用说了：那笔消费和推荐人毫无关系。
    let already_paid: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM orders WHERE user_id = $1 AND status = 'paid'",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    if already_paid > 0 {
        return Err(AppError::bad("账号已有付款记录，不能再绑定邀请人"));
    }

    let inserted = sqlx::query(
        "INSERT INTO referrals (referrer_user_id, referred_user_id, code, source, rate_bps, expires_at) \
         VALUES ($1,$2,$3,$4,$5, now() + make_interval(days => $6)) \
         ON CONFLICT (referred_user_id) DO NOTHING",
    )
    .bind(referrer)
    .bind(uid)
    .bind(&code)
    .bind("code")
    .bind(t.rate_bps)
    .bind(t.window_days)
    .execute(&state.db)
    .await?;

    if inserted.rows_affected() == 0 {
        return Err(AppError::bad("该账号已经绑定过邀请人"));
    }

    Ok(Json(json!({
        "ok": true,
        "rate_bps": t.rate_bps,
        "window_days": t.window_days,
    })))
}

/// 注册那一刻绑推荐人。
///
/// 和 `claim` 是同一套规则，少一道账号年龄的闸 —— 这个账号是这一刻刚建出来的，年龄检查
/// 没有意义。同样会拒绝：程序未开放、码无效或对方资格被收回、绑自己、重复绑定。
///
/// 永不返回错误。账号已经建好了，一个绑不上的推荐关系不该让注册失败：那会把一个真实用户
/// 挡在门外，去换一笔本来就没有的佣金。返回推荐人的邮箱只是为了写进事件流，方便排查。
pub async fn bind_at_signup(state: &AppState, uid: uuid::Uuid, code: &str) -> Option<String> {
    let t = terms(&state.db).await;
    if !t.enabled {
        return None;
    }
    let found: Option<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, email FROM users WHERE lower(referral_code) = lower($1) AND referral_enabled",
    )
    .bind(code)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();
    let (referrer, referrer_email) = found?;
    if referrer == uid {
        return None;
    }

    let done = sqlx::query(
        "INSERT INTO referrals (referrer_user_id, referred_user_id, code, source, rate_bps, expires_at) \
         VALUES ($1,$2,$3,'signup',$4, now() + make_interval(days => $5)) \
         ON CONFLICT (referred_user_id) DO NOTHING",
    )
    .bind(referrer)
    .bind(uid)
    .bind(code)
    .bind(t.rate_bps)
    .bind(t.window_days)
    .execute(&state.db)
    .await;

    match done {
        Ok(r) if r.rows_affected() > 0 => {
            tracing::info!(%uid, code, "referral bound at signup");
            Some(referrer_email)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------
// The payment hook
// ---------------------------------------------------------------------------------------

/// Turn a paid order into a commission claim, if somebody is owed one.
///
/// Called from inside the Stripe fulfilment transaction, after the order is claimed and the
/// grant is made. Sharing that transaction is what makes it exactly-once: the same `UPDATE
/// … WHERE status <> 'paid'` that stops a duplicate webhook granting twice also stops it
/// paying a commission twice, and the partial unique index on `(order_id) WHERE source =
/// 'referral'` is the belt to that braces.
///
/// Never returns an error into the payment path. A referral programme is a nice thing to
/// have; a payment that is refused because the nice thing broke is not. Failures are logged
/// and the payment stands.
pub async fn award(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    buyer: uuid::Uuid,
    order_id: Option<uuid::Uuid>,
    amount_cents: i64,
    currency: &str,
    // paid_at_unix: Stripe 说这笔钱是什么时候到的（秒）。0 表示不知道，退回 now()。
    paid_at_unix: i64,
) {
    if amount_cents <= 0 {
        return;
    }

    /*
     * The basis and the payout must be the same money.
     *
     * `amount_cents` is whatever Stripe charged, in whatever currency it charged it —
     * `amount_subtotal` on a session, `subtotal` on an invoice — and nothing downstream
     * carries a currency: `commissions` has no currency column and `connect::pay` sends the
     * literal "usd". A CNY-base price (which this catalogue has: prices.amount_cents is fen,
     * and stripe.rs::display_amount exists because a live price really did move to a CNY
     * base) would take ¥188.00 = 18800 fen ≈ US$26, compute 5640, and transfer US$56.40 —
     * over twice the gross of the sale, with no error anywhere, because a USD transfer from
     * a USD balance is a perfectly valid request.
     *
     * Skipping is the right failure: a commission not recorded is a support ticket, a
     * commission paid at 2x gross is money gone. Loud, because it means a price is
     * misconfigured and every sale on it is silently earning nobody anything.
     */
    /*
     * 换算成美元记账，而不是跳过。
     *
     * 上一版这里遇到非美元销售直接 return —— 当时是对的（账本没有币种列，connect::pay
     * 又写死美元，不挡就会把 18800 分人民币当成 $188 付佣金）。但中国区开始按人民币收款
     * 之后，那等于每一笔中国销售的推荐人都拿不到钱，而且只在日志里出现。
     *
     * 账本继续以美元计量（十几处跨行 SUM 和 Stripe 转账都依赖这一点），换算在写入时做一次，
     * 用的汇率钉在行上（fx_bps）—— 以后调汇率不会改写已经记下的佣金。
     */
    let ccy = currency.trim().to_ascii_lowercase();
    let (basis_usd, fx_bps) = match ccy.as_str() {
        "usd" => (amount_cents, 10_000i64),
        "cny" => {
            let bps = crate::settings::usd_per_cny_bps();
            (amount_cents * bps / 10_000, bps)
        }
        _ => {
            tracing::error!(
                order = ?order_id, currency, amount_cents,
                "commission skipped: 不认识的销售币种，账本只支持 usd / cny"
            );
            return;
        }
    };
    if basis_usd <= 0 {
        return;
    }

    /*
     * 两个时间条件，都以**付款时刻**为准，而不是这段代码跑起来的时刻。
     *
     * expires_at > 付款时间：webhook 可能晚到几小时甚至几天（Stripe 会重投三天），对账扫描
     * 更是十分钟一轮。按 now() 判，一笔明明在窗口内付的钱会因为送达晚了而拿不到佣金。
     *
     * created_at < 付款时间：推荐关系必须早于这笔付款。claim() 那边只挡了「已经有 status='paid'
     * 的订单」，但订单是下单时就以 pending 写进去的、要到 webhook 才翻成 paid —— 中间这段时间
     * 里绑一个推荐人，就能对一笔已经付掉的钱抽成。这一条把那扇门关上。
     */
    let paid_at = if paid_at_unix > 0 { paid_at_unix } else { chrono::Utc::now().timestamp() };
    let found: Result<Option<(uuid::Uuid, i32, String)>, _> = sqlx::query_as(
        "SELECT r.referrer_user_id, r.rate_bps, u.email \
         FROM referrals r JOIN users u ON u.id = r.referrer_user_id \
         WHERE r.referred_user_id = $1 \
           AND r.expires_at > to_timestamp($2) \
           AND r.created_at < to_timestamp($2)",
    )
    .bind(buyer)
    .bind(paid_at as f64)
    .fetch_optional(&mut **tx)
    .await;

    let Ok(Some((referrer, rate_bps, referrer_email))) = found else {
        return;
    };

    // Integer arithmetic, rounded down. A half-cent that cannot be paid should not be
    // recorded as owed — the ledger settles in whole cents.
    let commission_cents = basis_usd * rate_bps as i64 / 10_000;
    if commission_cents <= 0 {
        return;
    }

    let buyer_email: String =
        sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(buyer)
            .fetch_optional(&mut **tx)
            .await
            .ok()
            .flatten()
            .unwrap_or_default();

    /*
     * `auto` 现在只决定一件事：这笔佣金要不要人工审核。
     *
     * 它**不再**往 users.credits_cents 里写钱。旧写法把一笔美元分的佣金原样加进那一列，
     * 而那一列是网关的原始计费单位（约 663 个单位 = $1.00），推荐人实际拿到的只有应得的
     * 六分之一；而且当场到账意味着退款发生时钱早就出去了，没有任何追回的路径。
     *
     * 现在两种模式都只是写一条账，钱统一由打款批次在冻结期结束、攒够门槛之后用 Stripe
     * Connect 转真金。对应 kit 的 reviewStatus：auto=true 即 AUTO_APPROVED。
     *
     * 在事务里读而不是走 terms()（那个要连接池）：这段跑在 Stripe 履约路径上，手上只有
     * 事务。读不到就按「要人审」处理 —— 那是不会自己把钱送出去的那一边。
     */
    let auto: bool = sqlx::query_scalar("SELECT referral_auto_settle FROM app_settings WHERE id = 1")
        .fetch_optional(&mut **tx)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);
    let status = if auto { "settled" } else { "pending" };

    /*
     * 冻结期在这里冻结，不是打款时现算。
     *
     * 和 rate_bps、expires_at 一个道理：条款是发生当时定下的。之后运营把冻结期从 14 天改成
     * 30 天，不该把已经记下的佣金一起往后推 —— 那是事后改承诺。
     */
    let hold_days: i32 = sqlx::query_scalar("SELECT referral_hold_days FROM app_settings WHERE id = 1")
        .fetch_optional(&mut **tx)
        .await
        .ok()
        .flatten()
        .unwrap_or(14);

    /*
     * Everything below runs on a SAVEPOINT.
     *
     * `award` swallows its errors so a broken referral programme cannot refuse a payment —
     * but swallowing a sqlx error does not un-abort a Postgres transaction. Once any
     * statement here fails, the ENCLOSING fulfilment transaction is poisoned: the order
     * claim, the entitlement grant and the stripe_events dedupe row all vanish, `commit()`
     * returns a ROLLBACK command tag that sqlx reports as success, and the handler answers
     * 200. The customer has paid, has nothing, and Stripe never retries.
     *
     * That is not hypothetical. `commissions` carries TWO overlapping partial unique indexes
     * on (order_id): `idx_commissions_referral_order` WHERE source='referral', which the
     * ON CONFLICT below infers, and `idx_commissions_order_payable` WHERE status <> 'rejected',
     * which it cannot infer (status is a bind parameter, so the planner cannot prove the
     * predicate). A manual commission already on the order therefore raises an unhandled
     * 23505 — verified against the live database.
     *
     * The savepoint contains the damage: a failed commission rolls back to here and the
     * payment commits normally.
     */
    let mut sp = match tx.as_mut().begin().await {
        Ok(sp) => sp,
        Err(e) => {
            tracing::warn!("referral commission skipped, savepoint failed: {e}");
            return;
        }
    };

    let done = sqlx::query(
        "INSERT INTO commissions (referrer_user_id, referrer_email, customer_user_id, \
             customer_email, order_id, source, amount_cents, rate_bps, commission_cents, \
             status, note, settled_at, settled_by, mature_at, \
             sale_currency, sale_amount_cents, fx_bps) \
         VALUES ($1,$2,$3,$4,$5,'referral',$6,$7,$8,$9,'', \
                 CASE WHEN $9 = 'settled' THEN now() ELSE NULL END, \
                 CASE WHEN $9 = 'settled' THEN 'auto' ELSE '' END, \
                 now() + make_interval(days => $10), $11, $12, $13) \
         ON CONFLICT (order_id) WHERE source = 'referral' AND order_id IS NOT NULL DO NOTHING",
    )
    .bind(referrer)
    .bind(&referrer_email)
    .bind(buyer)
    .bind(&buyer_email)
    .bind(order_id)
    // amount_cents 这一列存的是**折算后的美元基数**，跨行合计才有意义；
    // 原币种和原始金额进 sale_* 两列。
    .bind(basis_usd)
    .bind(rate_bps)
    .bind(commission_cents)
    .bind(status)
    .bind(hold_days)
    .bind(&ccy)
    .bind(amount_cents)
    .bind(fx_bps as i32)
    .execute(&mut *sp)
    .await;

    let wrote = match &done {
        Ok(r) => r.rows_affected() > 0,
        Err(e) => {
            // The savepoint is now aborted; roll it back so the PAYMENT can still commit.
            tracing::error!(
                order = ?order_id,
                "referral commission not recorded (payment unaffected): {e}"
            );
            let _ = sp.rollback().await;
            return;
        }
    };

    // 这里曾经有一段「自动结算就往余额加钱」。它被删掉了，不是搬走了：佣金现在只由
    // 打款批次以现金形式支付一次。见本函数上方和 20260830 迁移。
    let _ = wrote;

    if let Err(e) = sp.commit().await {
        tracing::error!(order = ?order_id, "referral commission rolled back: {e}");
    }
}

/// The money came back. Stop paying commission on it.
///
/// Called from the Stripe webhook when a charge is refunded or disputed, inside the same
/// transaction as everything else that event does.
///
/// THE HARD CASE is a commission that has already been handed over. Reversing a ledger row
/// does not un-send money, and a system that silently debits somebody's balance months
/// later — possibly below zero, possibly after they have spent it — does more damage than
/// the refund did. So the rule is: reverse what has not been paid out, and for the rest
/// write down what happened and let a person decide.
///
///   * `pending`  → reversed. Nobody has been paid; nothing to recover.
///   * `settled`, `settled_by = 'auto'` → flagged only. Automatic settlement credits the
///     referrer's balance the moment the commission is written, so the value is already
///     with them.
///   * `settled` by hand → reversed only if the referrer's paid-out withdrawals still fit
///     inside what remains settled afterwards. If reversing would put them "overdrawn",
///     the money has demonstrably left, so it is flagged instead.
///
/// Flagged rows keep `status = 'settled'` and gain `reversed_at` + a reason, which is what
/// the console's 结算记录 screen shows in red. Never returns an error into the payment path:
/// a webhook that 500s is retried forever, and this is bookkeeping.
pub async fn reverse(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order_id: uuid::Uuid,
    reason: &str,
    // ratio_bps: 这笔销售退回了多少，万分比。10000 = 整笔。规范 7.3 的 clawbackRatio ——
    // 部分退款只追回相应比例，而不是把整笔佣金抹掉。
    ratio_bps: i64,
) {
    let ratio_bps = ratio_bps.clamp(0, 10_000);
    if ratio_bps == 0 {
        return;
    }
    let partial = ratio_bps < 10_000;
    // Unpaid commission: nothing has moved, so this is the one case that can be made exactly
    // right. A full refund reverses the row; a partial one just shrinks what is owed.
    let cleared = if partial {
        // 还没进批次的都能按比例扣：pending（待审）和 settled（已审但没被锁走）。
        // 自动审核打开时根本不会有 pending 行，只匹配 pending 等于这条路永远走不到，
        // 于是部分退款掉进下面的整笔标记 —— $500 的订单退 $10，$150 佣金全没了。
        sqlx::query(
            "UPDATE commissions SET \
                 commission_cents = commission_cents - (commission_cents * $3 / 10000), \
                 note = concat_ws(' ', NULLIF(note, ''), $2 || ' 部分退款，已按比例扣减'), \
                 updated_at = now() \
             WHERE order_id = $1 AND source = 'referral' \
               AND status IN ('pending', 'settled') AND payout_id IS NULL \
               AND reversed_at IS NULL",
        )
        .bind(order_id)
        .bind(reason)
        .bind(ratio_bps)
        .execute(&mut **tx)
        .await
    } else {
        sqlx::query(
            "UPDATE commissions SET status = 'reversed', reversed_at = now(), \
                 reversal_reason = $2, updated_at = now() \
             WHERE order_id = $1 AND source = 'referral' AND status = 'pending'",
        )
        .bind(order_id)
        .bind(reason)
        .execute(&mut **tx)
        .await
    };
    let cleared = cleared.map(|r| r.rows_affected()).unwrap_or(0);

    /*
     * Settled by hand, and still covered after taking this one back.
     *
     * The test is per referrer, not per row: withdrawals are amounts, not links to
     * particular commissions, so "was THIS one paid out" is not a question the schema can
     * answer. What it can answer is whether the person's settled total would still cover
     * everything already sent to them. If yes, nothing has left on account of this
     * commission and reversing is safe.
     *
     * `settled_by <> 'auto'` excludes automatic settlement, where the credit was applied
     * at the moment the row was written and no withdrawal exists to measure.
     */
    // 部分退款就到此为止：已结算的钱要么已经进了余额、要么已经转出去，按比例往回抠需要
    // 决定「从哪一笔里扣」，那是人的判断，不是这里该替他做的。标注出来。
    if partial {
        let flagged = sqlx::query(
            "UPDATE commissions SET reversed_at = now(), \
                 reversal_reason = $2 || ' (partial)', updated_at = now() \
             WHERE order_id = $1 AND source = 'referral' \
               AND status IN ('settled', 'paid') AND reversed_at IS NULL",
        )
        .bind(order_id)
        .bind(reason)
        .execute(&mut **tx)
        .await;
        let n = flagged.map(|r| r.rows_affected()).unwrap_or(0);
        tracing::info!(order = %order_id, ratio_bps, reduced = cleared, flagged = n, "partial refund");
        return;
    }

    let recovered = sqlx::query(
        "UPDATE commissions c SET status = 'reversed', reversed_at = now(), \
             reversal_reason = $2, updated_at = now() \
         WHERE c.order_id = $1 AND c.source = 'referral' AND c.status = 'settled' \
           AND c.settled_by <> 'auto' \
           AND (SELECT COALESCE(SUM(s.commission_cents), 0)::bigint FROM commissions s \
                WHERE s.referrer_user_id = c.referrer_user_id AND s.status = 'settled') \
               - c.commission_cents \
             >= (SELECT COALESCE(SUM(w.amount_cents), 0)::bigint FROM withdrawals w \
                 WHERE w.user_id = c.referrer_user_id AND w.status = 'paid')",
    )
    .bind(order_id)
    .bind(reason)
    .execute(&mut **tx)
    .await;
    let recovered = recovered.map(|r| r.rows_affected()).unwrap_or(0);

    // Everything still settled after those two passes is money that is genuinely gone.
    // Mark it so it surfaces, but leave the status alone — it was paid, and saying
    // otherwise in the ledger would be a lie about what happened.
    /*
     * 'paid' 也要盖到。
     *
     * 自动打款开着的时候，佣金从 settled 进批次那一刻就变成 'paid'，而且成功之后永远停在
     * 'paid'。原来三段 UPDATE 全部只匹配 pending / settled，于是一笔退款落在已进批次的佣金上
     * 时**什么都没发生** —— 连 reversed_at 都没写。之后打款失败回滚（payout::rollback）或者
     * 转账被冲回（webhook），这一行干干净净地回到 settled，下一轮批次照付不误，付的是一笔
     * 已经退了款的销售。
     */
    let flagged = sqlx::query(
        "UPDATE commissions SET reversed_at = now(), \
             reversal_reason = $2, updated_at = now() \
         WHERE order_id = $1 AND source = 'referral' \
           AND status IN ('settled', 'paid') AND reversed_at IS NULL",
    )
    .bind(order_id)
    .bind(reason)
    .execute(&mut **tx)
    .await;
    let flagged = flagged.map(|r| r.rows_affected()).unwrap_or(0);

    if cleared + recovered + flagged > 0 {
        tracing::info!(
            order = %order_id,
            cleared, recovered, flagged,
            reason,
            "referral commission reversed after refund"
        );
    }
}

/// The dispute was won: the money never left, so the reversal was wrong.
///
/// Scoped to reversals THIS dispute caused. `reverse()` stamps the event name into
/// `reversal_reason`, and without matching on it a won dispute would also undo an unrelated
/// refund on the same order — restoring commission on money that genuinely went back.
///
/// Reversed rows go back to `pending` rather than to whatever they were before, because the
/// previous status is not recorded and guessing at it is how a commission gets paid twice.
/// `pending` is the conservative landing place — it is owed again, and an operator (or the
/// auto-settle path on the next sale) decides what happens next. Rows that were only
/// flagged, because the money had already gone out, simply lose the flag: they were never
/// taken off anybody.
pub async fn unreverse(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, order_id: uuid::Uuid) {
    let done = sqlx::query(
        "UPDATE commissions SET \
             status = CASE WHEN status = 'reversed' THEN 'pending' ELSE status END, \
             reversed_at = NULL, reversal_reason = '', updated_at = now() \
         WHERE order_id = $1 AND source = 'referral' AND reversed_at IS NOT NULL \
           AND reversal_reason = 'charge.dispute.created'",
    )
    .bind(order_id)
    .execute(&mut **tx)
    .await;
    if let Ok(r) = done {
        if r.rows_affected() > 0 {
            tracing::info!(order = %order_id, rows = r.rows_affected(), "dispute won — commission restored");
        }
    }
}

// ---------------------------------------------------------------------------------------
// Who I brought in
// ---------------------------------------------------------------------------------------

#[derive(Serialize, sqlx::FromRow)]
pub struct MyReferral {
    /// Masked. See `mask_email`.
    pub who: String,
    pub source: String,
    pub rate_bps: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub active: bool,
    pub earned_cents: i64,
}

/// `a***z@example.com` — enough to recognise someone you invited, not enough to harvest.
///
/// A referrer knows who they invited, so hiding the address entirely would be theatre. But
/// this endpoint would otherwise hand out a customer's full address to anyone who got them
/// to click a link, which is a different thing from them knowing their own friend's email.
pub(crate) fn mask_email(addr: &str) -> String {
    let Some((local, domain)) = addr.split_once('@') else {
        return "—".to_string();
    };
    // 头尾留一点，中间遮掉 —— 和 `138****8888` 是同一个思路。
    //
    // 这里改过两次，两次都是被真实的坏处推着走的：
    //
    // 1. 最早按长度分档，`a@x.io` 遮成 `a*@x.io`、`ab@x.io` 遮成 `a*b@x.io` —— 短用户名
    //    整个露在外面，而短的恰恰最好猜。
    // 2. 于是改成只露首字符 `a***@x.io`。安全了，但**两个不同账号会渲染成同一个样子**
    //    （排行榜上真出现过：两行都是 `3***@qq.com`，其实是两个人）。遮到认不出彼此，
    //    这一栏就失去意义了。
    //
    // 现在按长度决定露几个，但**星号个数恒为 4**：够长的两端各留一点，短的只留头部。
    // 星号不跟着长度走是有意的 —— 否则遮挡本身就在报长度，那是白送的线索。
    let chars: Vec<char> = local.chars().collect();
    let n = chars.len();
    let take = |k: usize| -> String { chars[..k.min(n)].iter().collect() };
    let tail = |k: usize| -> String { chars[n.saturating_sub(k)..].iter().collect() };
    let masked = match n {
        // 太短，露一个字符就已经是相当大的一部分了 —— 尾部一概不露。
        0 => "****".to_string(),
        1..=3 => format!("{}****", take(1)),
        4..=6 => format!("{}****", take(2)),
        // 够长才两端都留：头 3 尾 2，中间无论多长都是 4 颗星。
        _ => format!("{}****{}", take(3), tail(2)),
    };
    format!("{masked}@{domain}")
}

/// `GET /api/referral/referrals` — the people this account brought in.
pub async fn my_referrals(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Vec<MyReferral>>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    type Row = (String, String, i32, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>, bool, i64);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT cu.email, r.source, r.rate_bps, r.created_at, r.expires_at, \
                (r.expires_at > now()) AS active, \
                COALESCE(( \
                  SELECT SUM(c.commission_cents) FROM commissions c \
                  WHERE c.referrer_user_id = r.referrer_user_id \
                    AND c.customer_user_id = r.referred_user_id \
                    AND c.status NOT IN ('rejected', 'reversed') \
                ), 0)::bigint \
         FROM referrals r JOIN users cu ON cu.id = r.referred_user_id \
         WHERE r.referrer_user_id = $1 \
         ORDER BY r.created_at DESC LIMIT 200",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|(email, source, rate_bps, created_at, expires_at, active, earned_cents)| {
                MyReferral {
                    who: mask_email(&email),
                    source,
                    rate_bps,
                    created_at,
                    expires_at,
                    active,
                    earned_cents,
                }
            })
            .collect(),
    ))
}

// ---------------------------------------------------------------------------------------
// Settlement records
// ---------------------------------------------------------------------------------------

/// Rows per page on both settlement screens.
const SETTLEMENTS_PER_PAGE: i64 = 20;

#[derive(Serialize, sqlx::FromRow)]
pub struct Settlement {
    pub id: uuid::Uuid,
    /// Whose commission this was. Only on the admin screen — the user's own screen knows.
    pub referrer_email: String,
    /// Whose payment produced it.
    pub customer_email: String,
    pub amount_cents: i64,
    pub rate_bps: i32,
    pub commission_cents: i64,
    /// 'auto', an operator's address, or empty for rows settled before this was recorded.
    pub settled_by: String,
    pub settled_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 'settled', or 'reversed' when a refund took the sale back before anyone was paid.
    pub status: String,
    /// Set when the underlying payment was refunded or disputed. A row that is BOTH
    /// 'settled' and reversed is the case needing a person: the money had already gone out.
    pub reversed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub reversal_reason: String,
}

#[derive(Deserialize)]
pub struct SettlementQuery {
    pub page: Option<i64>,
}

/// `GET /api/admin/settlements?page=` — every commission that has been settled.
///
/// Both routes in one list rather than two screens: the question an operator asks is "what
/// has been paid out", and splitting it by mechanism would mean reading two lists and
/// adding them up. `settled_by` says which was which.
pub async fn admin_settlements(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<SettlementQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    // `total_cents` counts only rows still standing: a reversed commission is not money
    // owed or paid, and rolling it into the total would overstate what the programme cost.
    // `flagged` is the follow-up queue — settled, then refunded, so the money is already out.
    let (total, auto, manual, total_cents, flagged): (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT count(*)::bigint, \
                count(*) FILTER (WHERE settled_by = 'auto' AND status = 'settled')::bigint, \
                count(*) FILTER (WHERE settled_by <> 'auto' AND settled_by <> '' \
                                   AND status = 'settled')::bigint, \
                COALESCE(SUM(commission_cents) FILTER (WHERE status = 'settled'), 0)::bigint, \
                count(*) FILTER (WHERE status = 'settled' AND reversed_at IS NOT NULL)::bigint \
         FROM commissions WHERE status IN ('settled', 'reversed')",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, 0, 0, 0, 0));

    let pages = ((total + SETTLEMENTS_PER_PAGE - 1) / SETTLEMENTS_PER_PAGE).max(1);
    let page = q.page.unwrap_or(1).clamp(1, pages);

    let rows = sqlx::query_as::<_, Settlement>(
        "SELECT id, referrer_email, customer_email, amount_cents, rate_bps, \
                commission_cents, settled_by, settled_at, created_at, \
                status, reversed_at, reversal_reason \
         FROM commissions WHERE status IN ('settled', 'reversed') \
         ORDER BY (status = 'settled' AND reversed_at IS NOT NULL) DESC, \
                  settled_at DESC NULLS LAST, created_at DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(SETTLEMENTS_PER_PAGE)
    .bind((page - 1) * SETTLEMENTS_PER_PAGE)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "rows": rows,
        "page": page,
        "pages": pages,
        "total": total,
        "auto_count": auto,
        "manual_count": manual,
        "total_cents": total_cents,
        "flagged": flagged,
        "per_page": SETTLEMENTS_PER_PAGE,
    })))
}

/// `GET /api/referral/settlements?page=` — this account's own settled commissions.
pub async fn my_settlements(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<SettlementQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let (total, total_cents): (i64, i64) = sqlx::query_as(
        "SELECT count(*)::bigint, \
                COALESCE(SUM(commission_cents) FILTER (WHERE status = 'settled'), 0)::bigint \
         FROM commissions WHERE referrer_user_id = $1 AND status IN ('settled', 'reversed')",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, 0));

    let pages = ((total + SETTLEMENTS_PER_PAGE - 1) / SETTLEMENTS_PER_PAGE).max(1);
    let page = q.page.unwrap_or(1).clamp(1, pages);

    // `referrer_email` is this account's own address, so it is selected as '' rather than
    // repeated on every row — the screen already knows whose it is.
    let rows = sqlx::query_as::<_, Settlement>(
        "SELECT id, '' AS referrer_email, customer_email, amount_cents, rate_bps, \
                commission_cents, settled_by, settled_at, created_at, \
                status, reversed_at, reversal_reason \
         FROM commissions WHERE referrer_user_id = $1 AND status IN ('settled', 'reversed') \
         ORDER BY settled_at DESC NULLS LAST, created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(uid)
    .bind(SETTLEMENTS_PER_PAGE)
    .bind((page - 1) * SETTLEMENTS_PER_PAGE)
    .fetch_all(&state.db)
    .await?;

    // The customer's address belongs to somebody else, so it is masked here exactly as it
    // is on the referrals screen. The admin list above is not masked — that screen is for
    // the operator, who can already see every account.
    let rows: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "customer_email": mask_email(&r.customer_email),
                "amount_cents": r.amount_cents,
                "rate_bps": r.rate_bps,
                "commission_cents": r.commission_cents,
                "settled_by": if r.settled_by == "auto" { "auto" } else { "manual" },
                "settled_at": r.settled_at,
                "created_at": r.created_at,
            })
        })
        .collect();

    Ok(Json(json!({
        "rows": rows,
        "page": page,
        "pages": pages,
        "total": total,
        "total_cents": total_cents,
        "per_page": SETTLEMENTS_PER_PAGE,
    })))
}

// ---------------------------------------------------------------------------------------
// Getting paid
// ---------------------------------------------------------------------------------------

/// Below this, a request is not worth anyone's time to process by hand.
const MIN_WITHDRAWAL_CENTS: i64 = 1_000;

const METHODS: [&str; 4] = ["alipay", "wechat", "bank", "paypal"];

/// What this account may still ask for.
///
/// Settled commission, minus everything already requested that has not been rejected. A
/// pending request counts against the balance from the moment it is made — otherwise the
/// same money can be requested twice before the first request is paid, and the operator
/// discovers it while sending the second.
async fn withdrawable(state: &AppState, uid: uuid::Uuid) -> ApiResult<i64> {
    /*
     * 两个排除条件，各堵一条重复支付的路。
     *
     * settled_by <> 'auto'：自动结算在写这条佣金的同时，已经把等额的钱加进了
     * users.credits_cents（见 award）。如果这里还把它算作可提现，同一笔佣金就被发了两次 ——
     * 一次进余额，一次转成现金。20260823 的迁移注释写着「自动结算之后没有可提的东西」，
     * 但在这之前，全服务器没有一行代码执行这句话：侧栏只是把入口藏起来了，接口照收。
     *
     * reversed_at IS NULL：订单退款之后，reverse() 对已经付过的那些行只打标记、
     * 保留 status='settled'（因为钱确实已经出去了，改账本不会把钱变回来）。但"已经付过"
     * 不等于"还能再提一次"。少了这个条件，一笔退掉的订单，佣金还能原样提走。
     */
    let settled: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(commission_cents), 0)::bigint FROM commissions \
         WHERE referrer_user_id = $1 AND status = 'settled' \
           AND reversed_at IS NULL",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // 'failed' 和 'returned' 的钱从来没到对方手上（或者已经被 Stripe 冲回来了），
    // 必须重新算作可提现 —— 否则一次转账失败等于把这笔钱永远锁死。
    let taken: i64 = sqlx::query_scalar(
        // method='auto' 是批量打款自己开的行（payout.rs 是唯一写这个值的地方，手动那条路
        // 只接受 alipay/wechat/bank/paypal）。它背后的佣金已经从 status='settled' 变成
        // 'paid'、从上面那个 settled 合计里出去了 —— 在这里再扣一次就是扣两遍。
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM withdrawals \
         WHERE user_id = $1 AND status NOT IN ('rejected', 'failed', 'returned') \
           AND method <> 'auto'",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok((settled - taken).max(0))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Withdrawal {
    pub id: uuid::Uuid,
    pub amount_cents: i64,
    pub method: String,
    pub account: String,
    pub status: String,
    pub note: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `GET /api/referral/withdrawals` — what I can ask for, and what I have asked for.
pub async fn withdrawals(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    let mut rows = sqlx::query_as::<_, Withdrawal>(
        "SELECT id, amount_cents, method, account, status, note, created_at, paid_at \
         FROM withdrawals WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(uid)
    .fetch_all(&state.db)
    .await?;
    // 存的是密文（fc1:...）或遗留明文，回显前解开。用 decrypt_or_raw：这是纯展示，
    // 万一解不开也让页面照常渲染（顶多显示原值），别为一行坏数据把整个列表 500。
    for w in &mut rows {
        w.account = field_crypto::decrypt_or_raw(&w.account, WD_ACCOUNT_CTX);
    }

    // Shown alongside the balance so "why is this less than my earnings" has an answer on
    // screen: commission that is still pending has not been approved and cannot be drawn.
    let pending_commission: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(commission_cents), 0)::bigint FROM commissions \
         WHERE referrer_user_id = $1 AND status = 'pending'",
    )
    .bind(uid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(json!({
        "available_cents": withdrawable(&state, uid).await?,
        "pending_commission_cents": pending_commission,
        "min_cents": MIN_WITHDRAWAL_CENTS,
        "methods": METHODS,
        "rows": rows,
    })))
}

#[derive(Deserialize)]
pub struct WithdrawReq {
    pub amount_cents: i64,
    pub method: String,
    pub account: String,
    /// Optional payment QR, as a base64 `data:` image. Validated by the same rule as a
    /// profile picture, so SVG and remote URLs cannot get in.
    pub qr: Option<String>,
}

/// `POST /api/referral/withdraw` — ask to be paid.
///
/// Records a request. It does not move money, and nothing downstream of it does either —
/// an operator reads the queue and pays by whatever means they actually use, then marks it
/// paid. The screen says so rather than letting "提现" imply a transfer is on its way.
pub async fn withdraw(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<WithdrawReq>,
) -> ApiResult<Json<serde_json::Value>> {
    let uid = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::unauthorized("令牌损坏"))?;

    /*
     * 开了批量结算，这条手动路就必须关掉。
     *
     * 两条路都从同一份「已结算佣金」里取钱，但各自记账：用户在这里提走 $50 的同时，
     * 调度器可能正好把同一批佣金锁进一次打款。批量那条路会把佣金标成 paid，手动这条路
     * 只看合计金额 —— 两边都认为自己有权发这笔钱。
     *
     * 所以不是"两个功能并存"，而是"谁来付"这一件事只能有一个答案。
     */
    let t = terms(&state.db).await;
    if t.batch_enabled {
        return Err(AppError::bad(
            "已开启自动结算打款：佣金过了冻结期、攒够门槛后会自动转出，无需手动申请",
        ));
    }

    let method = req.method.trim().to_ascii_lowercase();
    if !METHODS.contains(&method.as_str()) {
        return Err(AppError::bad("收款方式不支持"));
    }
    let account = req.account.trim();
    if account.is_empty() || account.chars().count() > 200 {
        return Err(AppError::bad("请填写收款账号"));
    }
    if req.amount_cents < MIN_WITHDRAWAL_CENTS {
        return Err(AppError::bad(format!(
            "单次提现不少于 ${:.2}",
            MIN_WITHDRAWAL_CENTS as f64 / 100.0
        )));
    }

    // Checked here rather than trusted from the client, obviously — but also re-checked
    // against the same definition the balance is displayed with, so the number someone
    // sees and the number they are held to cannot drift apart.
    let available = withdrawable(&state, uid).await?;
    if req.amount_cents > available {
        return Err(AppError::bad(format!(
            "可提现余额只有 ${:.2}",
            available as f64 / 100.0
        )));
    }

    /*
     * 上面那次检查只是给用户一个体面的报错。真正算数的是下面这段。
     *
     * 之前的写法是「先查余额、再插一行」，两条独立的池连接，中间没有事务也没有锁。五个并发
     * 请求同时通过 $50 的检查，就会插出五行、发出 $250 —— 而且 withdrawable 末尾的 .max(0)
     * 会把透支结果压成 0，账面上看不出来。withdrawals 上唯一的唯一索引是 transfer_id，这里
     * 用不上：每个并发请求各自生成一个新的行 id，而那个 id 正是 connect::pay 的幂等键，
     * 所以 Stripe 看到的是五笔货真价实的不同转账。
     *
     * 用户行的 FOR UPDATE 把同一个账号的提现串行化，余额在锁内重算一次。
     */
    let mut tx = state.db.begin().await?;
    sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(uid)
        .execute(&mut *tx)
        .await?;

    let settled: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(commission_cents), 0)::bigint FROM commissions \
         WHERE referrer_user_id = $1 AND status = 'settled' \
           AND reversed_at IS NULL",
    )
    .bind(uid)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);
    let taken: i64 = sqlx::query_scalar(
        // method='auto' 是批量打款自己开的行（payout.rs 是唯一写这个值的地方，手动那条路
        // 只接受 alipay/wechat/bank/paypal）。它背后的佣金已经从 status='settled' 变成
        // 'paid'、从上面那个 settled 合计里出去了 —— 在这里再扣一次就是扣两遍。
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM withdrawals \
         WHERE user_id = $1 AND status NOT IN ('rejected', 'failed', 'returned') \
           AND method <> 'auto'",
    )
    .bind(uid)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);
    if req.amount_cents > (settled - taken).max(0) {
        return Err(AppError::bad("可提现余额不足，请刷新后重试"));
    }

    // Same validator the avatar upload uses: raster only, size-capped, base64 checked.
    // Reused rather than re-derived so the rules cannot drift into disagreeing.
    let qr = match req.qr.as_deref() {
        Some(raw) if !raw.trim().is_empty() => crate::auth::clean_avatar(raw)?,
        _ => None,
    };

    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO withdrawals (user_id, amount_cents, method, account, qr) \
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(uid)
    .bind(req.amount_cents)
    .bind(&method)
    .bind(field_crypto::encrypt(account, WD_ACCOUNT_CTX))
    .bind(qr.as_deref().map(|q| field_crypto::encrypt(q, WD_QR_CTX)))
    .fetch_one(&mut *tx)
    .await?;

    // 锁到这里为止。转账必须在事务之外 —— Stripe 那边的钱不会因为这里 ROLLBACK 而退回来。
    tx.commit().await?;

    /*
     * Try to pay it right now.
     *
     * This is what "fully automatic" means in practice: a referrer with a connected Stripe
     * account is paid the moment they ask, and nobody ticks a box. Everyone else lands in
     * the same queue as before, with the reason written on the row so a request that could
     * not be paid automatically is distinguishable from one nobody has looked at yet.
     *
     * The transfer is deliberately NOT inside a database transaction. Money moving at
     * Stripe cannot be rolled back by a ROLLBACK here, so the honest order is: record the
     * request, attempt the transfer, then record what happened. If the last write is lost
     * the row stays pending with Stripe's transfer already made — which the unique index on
     * transfer_id and the idempotency key together stop from ever being sent twice.
     */
    /*
     * 先把这一行标成 sending，再去调 Stripe。
     *
     * 最可能的坏情况不是转账被拒，而是转账已经在 Stripe 那边建好了、我们这边超时收不到回应
     * （connect.rs 会返回 Skipped("Stripe 不可达")）。如果这一行还停在 pending，它和一笔
     * 「没人处理过的申请」在运营队列里长得一模一样，运营会再手工转一次 —— 同一笔钱付两遍。
     *
     * admin_withdraw_status 只接受 pending，所以 sending 的行人工也点不动：进程中途死掉，
     * 这行会停在 sending 等人来看，而不是被悄悄再付一次。
     */
    sqlx::query(
        "UPDATE withdrawals SET status = 'sending', provider = 'stripe_connect', \
             updated_at = now() WHERE id = $1 AND status = 'pending'",
    )
    .bind(id)
    .execute(&state.db)
    .await
    .ok();

    let paid = match crate::connect::pay(&state, id, uid, req.amount_cents).await {
        crate::connect::Payout::Sent(transfer) => {
            let recorded = sqlx::query(
                "UPDATE withdrawals SET status = 'paid', provider = 'stripe_connect', \
                     transfer_id = $2, paid_at = now(), paid_by = 'stripe-connect', \
                     updated_at = now() \
                 WHERE id = $1 AND status = 'sending'",
            )
            .bind(id)
            .bind(&transfer)
            .execute(&state.db)
            .await;
            // 钱已经出去了。这条 UPDATE 要是没落上，必须喊出来 —— 否则这行会停在 sending，
            // 而 Stripe 那边的转账已经成立。
            if !matches!(&recorded, Ok(r) if r.rows_affected() == 1) {
                tracing::error!(
                    withdrawal = %id, transfer = %transfer,
                    "TRANSFER SENT BUT NOT RECORDED — reconcile by metadata[withdrawal_id]"
                );
            }
            Some(transfer)
        }
        crate::connect::Payout::Unknown(reason) => {
            // 结果不明：留在 sending 等人核对。绝不能回到 pending —— 那等于允许再付一次。
            let unsure = true;
            let _ = &reason;
            sqlx::query(
                "UPDATE withdrawals SET failure_reason = $2, \
                     status = CASE WHEN $3 THEN 'sending' ELSE 'pending' END, \
                     updated_at = now() \
                 WHERE id = $1 AND status = 'sending'",
            )
            .bind(id)
            .bind(&reason)
            .bind(unsure)
            .execute(&state.db)
            .await
            .ok();
            if unsure {
                tracing::error!(withdrawal = %id, "payout outcome unknown — left as sending");
            }
            None
        }
        crate::connect::Payout::Refused(reason) => {
            // 明确被拒：钱没动，回到 pending 交给人工。
            sqlx::query(
                "UPDATE withdrawals SET failure_reason = $2, status = 'pending', \
                     updated_at = now() \
                 WHERE id = $1 AND status = 'sending'",
            )
            .bind(id)
            .bind(&reason)
            .execute(&state.db)
            .await
            .ok();
            None
        }
    };

    crate::realtime::record_event(
        &state,
        Some(uid),
        "withdrawal_requested",
        json!({ "amount_cents": req.amount_cents, "method": method, "auto_paid": paid.is_some() }),
    )
    .await;

    Ok(Json(json!({
        "id": id,
        "auto_paid": paid.is_some(),
        "transfer_id": paid,
        "available_cents": withdrawable(&state, uid).await?,
    })))
}

// ---------------------------------------------------------------------------------------
// The admin's side
// ---------------------------------------------------------------------------------------

#[derive(Serialize, sqlx::FromRow)]
pub struct AdminWithdrawal {
    pub id: uuid::Uuid,
    pub email: String,
    /// The name the account holder set, joined for display. Empty when never set — the
    /// address is what identifies the account either way.
    pub name: String,
    pub amount_cents: i64,
    pub method: String,
    /// Where to send it, as typed. Payout data: admin-gated, never on a public route.
    pub account: String,
    /// A `data:` image, or null. Rendered as a thumbnail in the queue.
    pub qr: Option<String>,
    pub status: String,
    pub note: String,
    /// So the operator can see whether the balance still covers it before paying.
    pub settled_cents: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub paid_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Which admin marked it paid, and the transfer's own reference. Empty until then.
    pub paid_by: String,
    pub reference: String,
    /// 'manual' or 'stripe_connect'; Stripe's `tr_…` once one exists; and why an automatic
    /// attempt did not go through. A row in `sending` with a reason is one where the outcome
    /// is genuinely unknown — do not pay it by hand without checking Stripe first.
    pub provider: String,
    pub transfer_id: Option<String>,
    pub failure_reason: String,
}

#[derive(Deserialize)]
pub struct AdminWithdrawQuery {
    /// pending | paid | rejected | all. Defaults to the queue that needs work.
    pub status: Option<String>,
    /// 1-based. Out of range is clamped rather than refused — a stale link should land
    /// somewhere real, not on an error.
    pub page: Option<i64>,
}

/// Rows per page of the payout queue. Larger than a marketing leaderboard because this is
/// a work queue: an operator paying people wants to see the batch, not six of it.
const PAYOUTS_PER_PAGE: i64 = 20;

/// `GET /api/admin/withdrawals?status=` — who has asked to be paid.
///
/// Oldest first when pending: this is a queue, and the person who has waited longest is
/// the one to deal with next. Everything else newest first, because that is a history.
pub async fn admin_withdrawals(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<AdminWithdrawQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let status = q.status.unwrap_or_else(|| "pending".into());
    let wanted = if ["pending", "paid", "rejected"].contains(&status.as_str()) {
        status.clone()
    } else {
        String::new() // 'all'
    };

    // Counted before the page is cut, so the pager has a denominator that does not move
    // as the operator works through the queue.
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM withdrawals WHERE $1 = '' OR status = $1",
    )
    .bind(&wanted)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let pages = ((total + PAYOUTS_PER_PAGE - 1) / PAYOUTS_PER_PAGE).max(1);
    let page = q.page.unwrap_or(1).clamp(1, pages);
    let offset = (page - 1) * PAYOUTS_PER_PAGE;

    let mut rows = sqlx::query_as::<_, AdminWithdrawal>(
        "SELECT w.id, u.email, \
                btrim(concat_ws(' ', NULLIF(u.first_name, ''), NULLIF(u.last_name, ''))) AS name, \
                w.amount_cents, w.method, w.account, w.qr, w.status, w.note, \
                COALESCE(( \
                  SELECT SUM(c.commission_cents) FROM commissions c \
                  WHERE c.referrer_user_id = w.user_id AND c.status = 'settled' \
                    AND c.reversed_at IS NULL AND c.payout_id IS NULL \
                ), 0)::bigint AS settled_cents, \
                w.created_at, w.paid_at, w.paid_by, w.reference, \
                w.provider, w.transfer_id, w.failure_reason \
         FROM withdrawals w JOIN users u ON u.id = w.user_id \
         WHERE $1 = '' OR w.status = $1 \
         ORDER BY CASE WHEN w.status = 'pending' THEN 0 ELSE 1 END, \
                  CASE WHEN w.status = 'pending' THEN w.created_at END ASC, \
                  w.created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(&wanted)
    .bind(PAYOUTS_PER_PAGE)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;
    // 管理员靠这一页的 account/qr 手动打款。解开再显示（decrypt_or_raw：展示路径，
    // 解不开也让页面渲染，人会注意到异常）。
    for w in &mut rows {
        w.account = field_crypto::decrypt_or_raw(&w.account, WD_ACCOUNT_CTX);
        w.qr = w.qr.as_deref().map(|q| field_crypto::decrypt_or_raw(q, WD_QR_CTX));
    }

    let pending_total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_cents), 0)::bigint FROM withdrawals WHERE status = 'pending'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(json!({
        "rows": rows,
        // The page actually served, which is not always the one asked for.
        "page": page,
        "pages": pages,
        "total": total,
        "per_page": PAYOUTS_PER_PAGE,
        // Of everything pending, not of this page — it is the money still owed.
        "pending_total_cents": pending_total,
    })))
}

#[derive(Deserialize)]
pub struct WithdrawStatusReq {
    /// paid | rejected. There is no way back to pending: marking something paid is a
    /// statement that money left, and un-saying it in software does not un-send it.
    pub status: String,
    pub note: Option<String>,
    /// The transfer's own identifier — a bank reference, an Alipay order number, whatever
    /// the sending end produced. This is the only thing that ties a row in this table to a
    /// real movement of money, which is what makes the row worth anything three months
    /// later when somebody asks whether they were paid.
    pub reference: Option<String>,
}

/// `POST /api/admin/withdrawals/:id/status`
///
/// Records what the operator did after paying — or refusing to. It does not move money
/// either; nothing in this service does.
///
/// Rejecting returns the amount to the person's withdrawable balance, because `withdrawable`
/// counts everything that is not rejected. That is the intended behaviour and the reason
/// rejecting is safe: it un-reserves the money rather than destroying it.
pub async fn admin_withdraw_status(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<WithdrawStatusReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if !["paid", "rejected"].contains(&req.status.as_str()) {
        return Err(AppError::bad("状态只能是 paid 或 rejected"));
    }

    /*
     * 开着自动打款时不许人工标「已支付」。
     *
     * withdraw() 只拦了新申请，这个接口原来完全不看这个开关 —— 于是切换之前留下的那些
     * pending 行仍然能被手工付掉，而它们背后的佣金正等着被批次锁走。同一笔钱两条路各发一次。
     *
     * 「驳回」照旧允许：那正是把遗留队列清干净的办法。
     */
    let t = terms(&state.db).await;
    if t.batch_enabled && req.status == "paid" {
        return Err(AppError::bad(
            "已开启自动打款：佣金会由系统转出，不要再人工标记已支付。要清掉这条遗留申请请用「驳回」",
        ));
    }

    // Only a pending request can be decided. Without this, a double-click marks an
    // already-paid request paid again and its timestamp moves — which is the audit trail
    // for "when did we actually send this" quietly changing.
    // Who did it and what they sent, recorded in the same statement that flips the status —
    // an audit trail written afterwards is one that can be missing. `paid_by` comes from the
    // token, never from the body: it is a record of who acted, so the caller does not get to
    // choose what it says.
    /*
     * 'sending' 也必须能被处理，否则它是个死胡同。
     *
     * 结果不明的那些行就停在 sending：钱可能已经出去了，程序不敢猜。人去 Stripe 里按
     * metadata[withdrawal_id] 查完之后，得有地方把答案写回来 —— 查到转账成功就标已支付
     * （必须带上 tr_ 号），查到根本没建就驳回，驳回会把佣金放回去重来。
     * 原来这个接口只接受 pending，于是这些行永远卡住，背后的佣金也永远停在 paid。
     */
    let row = sqlx::query_as::<_, (uuid::Uuid, i64)>(
        "UPDATE withdrawals SET status = $2, \
             note = CASE WHEN $3::text = '' THEN note ELSE $3 END, \
             paid_at = CASE WHEN $2 = 'paid' THEN now() ELSE NULL END, \
             paid_by = CASE WHEN $2 = 'paid' THEN $4 ELSE '' END, \
             reference = CASE WHEN $2 = 'paid' THEN $5 ELSE '' END, \
             updated_at = now() \
         WHERE id = $1 AND status IN ('pending', 'sending') \
         RETURNING user_id, amount_cents",
    )
    .bind(id)
    .bind(&req.status)
    .bind(req.note.unwrap_or_default().trim())
    .bind(&claims.email)
    .bind(req.reference.unwrap_or_default().trim())
    .fetch_optional(&state.db)
    .await?;

    let Some((uid, amount_cents)) = row else {
        return Err(AppError::bad("该申请已经处理过了"));
    };

    // 驳回一笔批量打款，等于宣布这次转账没成立 —— 被它锁走的佣金必须回到可结算，
    // 否则它们会永远停在 paid：不会被支付，也不会再被下一轮扫到。
    if req.status == "rejected" {
        crate::payout::release(&state, id).await;
    }

    crate::realtime::record_event(
        &state,
        Some(uid),
        "withdrawal_decided",
        json!({ "status": req.status, "amount_cents": amount_cents, "by": claims.email }),
    )
    .await;

    Ok(Json(json!({ "ok": true, "status": req.status })))
}

/// `GET /api/admin/referral/settings`
pub async fn admin_settings(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let t = terms(&state.db).await;
    let referrals: i64 = sqlx::query_scalar("SELECT count(*) FROM referrals")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let active: i64 = sqlx::query_scalar("SELECT count(*) FROM referrals WHERE expires_at > now()")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    // Whether the payout screens should be offered at all. Automatic settlement means
    // there is nothing to withdraw — but a request made before the switch is somebody
    // still waiting for money, so the screen stays reachable until the queue is empty.
    let pending_withdrawals: i64 =
        sqlx::query_scalar("SELECT count(*)::bigint FROM withdrawals WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let (holding, ready): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(commission_cents) FILTER (WHERE mature_at > now()), 0)::bigint, \
                COALESCE(SUM(commission_cents) FILTER (WHERE mature_at <= now()), 0)::bigint \
         FROM commissions \
         WHERE status = 'settled' AND reversed_at IS NULL \
           AND payout_id IS NULL AND mature_at IS NOT NULL",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, 0));

    Ok(Json(json!({
        "rate_bps": t.rate_bps,
        "window_days": t.window_days,
        "enabled": t.enabled,
        "auto_settle": t.auto_settle,
            // 自动打款开着时，用户端要把「提现」入口藏掉：钱由系统按冻结期和门槛自己转，
            // 没有可提的动作。开户入口不在那一页，所以藏掉它不会挡住收款账户的绑定。
            "batch_enabled": t.batch_enabled,
        "hold_days": t.hold_days,
        "min_payout_cents": t.min_payout_cents,
        "batch_enabled": t.batch_enabled,
        "referrals": referrals,
        "active": active,
        "pending_withdrawals": pending_withdrawals,
        // 现在有多少钱卡在冻结期里、多少已经到期在等门槛。运营开这个开关之前，
        // 应该先看得见它到底会付出去多少。
        "holding_cents": holding,
        "ready_cents": ready,
    })))
}

#[derive(Deserialize)]
pub struct SettingsReq {
    pub rate_bps: Option<i32>,
    pub window_days: Option<i32>,
    pub enabled: Option<bool>,
    pub auto_settle: Option<bool>,
    /// 冻结期天数。只影响此后新记下的佣金 —— 已经记下的把到期时间冻在了行上。
    pub hold_days: Option<i32>,
    /// 提现门槛，分。
    pub min_payout_cents: Option<i64>,
    /// 定时批量打款的总开关。开了之后服务器会自动往外转钱。
    pub batch_enabled: Option<bool>,
}

/// `PUT /api/admin/referral/settings`
///
/// Applies to referrals claimed from now on. Existing ones keep the rate and expiry they
/// were created with — see the module note.
pub async fn admin_save_settings(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<SettingsReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    // Checked here as well as in the schema so the operator gets a sentence rather than a
    // constraint violation.
    if let Some(r) = req.rate_bps {
        if !(0..=10_000).contains(&r) {
            return Err(AppError::bad("返佣比例只能在 0% 到 100% 之间"));
        }
    }
    if let Some(d) = req.window_days {
        if !(0..=3650).contains(&d) {
            return Err(AppError::bad("返佣期限只能在 0 到 3650 天之间"));
        }
        // 0 天等于第一笔付款到达时窗口已经过期 —— 一分钱也不会记，而且不报错。
        if d == 0 {
            return Err(AppError::bad("返佣期限不能是 0 天：那样一笔佣金也不会记录"));
        }
    }
    if let Some(h) = req.hold_days {
        if !(0..=180).contains(&h) {
            return Err(AppError::bad("冻结期只能在 0 到 180 天之间"));
        }
    }
    if let Some(m) = req.min_payout_cents {
        if m <= 0 {
            return Err(AppError::bad("提现门槛必须大于 0"));
        }
    }

    sqlx::query(
        "UPDATE app_settings SET \
           referral_rate_bps = COALESCE($1, referral_rate_bps), \
           referral_window_days = COALESCE($2, referral_window_days), \
           referral_enabled = COALESCE($3, referral_enabled), \
           referral_auto_settle = COALESCE($4, referral_auto_settle), \
           referral_hold_days = COALESCE($6, referral_hold_days), \
           referral_min_payout_cents = COALESCE($7, referral_min_payout_cents), \
           referral_batch_enabled = COALESCE($8, referral_batch_enabled), \
           updated_at = now(), updated_by = $5 \
         WHERE id = 1",
    )
    .bind(req.rate_bps)
    .bind(req.window_days)
    .bind(req.enabled)
    .bind(req.auto_settle)
    .bind(&claims.sub)
    .bind(req.hold_days)
    .bind(req.min_payout_cents)
    .bind(req.batch_enabled)
    .execute(&state.db)
    .await?;

    let t = terms(&state.db).await;
    if req.batch_enabled == Some(true) {
        // 这个开关一开，服务器就会自己往外转钱。留一条记录，说明是谁开的。
        tracing::warn!(by = %claims.email, "automatic batch payouts ENABLED");
    }
    Ok(Json(json!({
        "rate_bps": t.rate_bps,
        "window_days": t.window_days,
        "enabled": t.enabled,
        "auto_settle": t.auto_settle,
            // 自动打款开着时，用户端要把「提现」入口藏掉：钱由系统按冻结期和门槛自己转，
            // 没有可提的动作。开户入口不在那一页，所以藏掉它不会挡住收款账户的绑定。
            "batch_enabled": t.batch_enabled,
        "hold_days": t.hold_days,
        "min_payout_cents": t.min_payout_cents,
        "batch_enabled": t.batch_enabled,
    })))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ReferralRow {
    pub referrer_email: String,
    pub referred_email: String,
    pub code: String,
    pub source: String,
    pub rate_bps: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub active: bool,
    /// What this pairing has actually produced, so a list of names is also a list of
    /// results — a referrer with 40 sign-ups and no revenue is a different story from one
    /// with three.
    pub earned_cents: i64,
}

/// `GET /api/admin/referral/list` — who referred whom, newest first.
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<Vec<ReferralRow>>> {
    admin_only(&claims)?;
    let rows = sqlx::query_as::<_, ReferralRow>(
        "SELECT ru.email AS referrer_email, cu.email AS referred_email, r.code, r.source, \
                r.rate_bps, r.created_at, r.expires_at, \
                (r.expires_at > now()) AS active, \
                COALESCE(( \
                  SELECT SUM(c.commission_cents) FROM commissions c \
                  WHERE c.referrer_user_id = r.referrer_user_id \
                    AND c.customer_user_id = r.referred_user_id \
                    AND c.status NOT IN ('rejected', 'reversed') \
                ), 0)::bigint AS earned_cents \
         FROM referrals r \
         JOIN users ru ON ru.id = r.referrer_user_id \
         JOIN users cu ON cu.id = r.referred_user_id \
         ORDER BY r.created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ReferrerRow {
    pub id: uuid::Uuid,
    pub email: String,
    /// Whether this account may recruit. See `admin_grant`.
    pub referral_enabled: bool,
    pub code: String,
    pub invited: i64,
    /// Of those, how many are still inside their window and can still earn.
    pub active: i64,
    pub pending_cents: i64,
    pub settled_cents: i64,
    pub last_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub struct ReferrerQuery {
    /// Substring of the address, case-insensitive. Empty matches everyone.
    pub q: Option<String>,
    /// 1-based. Clamped rather than refused, so a stale page number lands somewhere real.
    pub page: Option<i64>,
}

/// Accounts per page. Matches the payout queue: this is a list an operator works through,
/// and twenty rows is about a screenful without becoming a scroll of its own.
const REFERRERS_PER_PAGE: i64 = 20;

/// `GET /api/admin/referral/referrers?q=` — every account, and where it stands.
///
/// All users, not only those who have referred somebody: this is the screen where the
/// privilege is handed out, so the people who do not have it yet are the whole point of
/// looking. Granted accounts sort to the top, then by what they have earned — the list is
/// long and the interesting end of it is short.
///
/// The subselect is not decoration. `ORDER BY (pending_cents + settled_cents)` against the
/// flat query resolved those names against `users`, where they do not exist, and the
/// endpoint answered 500 on every call. An alias is only usable as a bare `ORDER BY` term,
/// never inside an expression — wrapping makes them real columns.
pub async fn admin_referrers(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<ReferrerQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let needle = q.q.unwrap_or_default().trim().to_string();

    // Both counts respect the search, so the header describes the list on screen rather
    // than the whole table. Counted server-side because the page no longer holds every
    // row — deriving "1 / 124 已开通" from the twenty rows fetched would just be wrong.
    let (total, granted): (i64, i64) = sqlx::query_as(
        "SELECT count(*)::bigint, \
                count(*) FILTER (WHERE referral_enabled)::bigint \
         FROM users WHERE $1 = '' OR email ILIKE '%' || $1 || '%'",
    )
    .bind(&needle)
    .fetch_one(&state.db)
    .await
    .unwrap_or((0, 0));

    let pages = ((total + REFERRERS_PER_PAGE - 1) / REFERRERS_PER_PAGE).max(1);
    let page = q.page.unwrap_or(1).clamp(1, pages);
    let offset = (page - 1) * REFERRERS_PER_PAGE;

    let rows = sqlx::query_as::<_, ReferrerRow>(
        "SELECT * FROM ( \
           SELECT u.id, u.email, u.referral_enabled, \
                  COALESCE(u.referral_code, '') AS code, \
                  (SELECT count(*) FROM referrals r WHERE r.referrer_user_id = u.id)::bigint AS invited, \
                  (SELECT count(*) FROM referrals r \
                    WHERE r.referrer_user_id = u.id AND r.expires_at > now())::bigint AS active, \
                  COALESCE((SELECT SUM(c.commission_cents) FROM commissions c \
                    WHERE c.referrer_user_id = u.id AND c.status = 'pending'), 0)::bigint AS pending_cents, \
                  COALESCE((SELECT SUM(c.commission_cents) FROM commissions c \
                    WHERE c.referrer_user_id = u.id AND c.status = 'settled'), 0)::bigint AS settled_cents, \
                  (SELECT max(r.created_at) FROM referrals r WHERE r.referrer_user_id = u.id) AS last_at, \
                  u.created_at \
           FROM users u \
           WHERE $1 = '' OR u.email ILIKE '%' || $1 || '%' \
         ) t \
         ORDER BY referral_enabled DESC, (pending_cents + settled_cents) DESC, \
                  invited DESC, created_at DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(&needle)
    .bind(REFERRERS_PER_PAGE)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "rows": rows,
        // The page actually served, so a clamped request shows where it really landed.
        "page": page,
        "pages": pages,
        "total": total,
        "granted": granted,
        "per_page": REFERRERS_PER_PAGE,
    })))
}

#[derive(Deserialize)]
pub struct GrantReq {
    pub enabled: bool,
}

/// `POST /api/admin/referral/grant/:id` — let this account recruit, or stop it.
///
/// Granting mints the code straight away rather than waiting for the person to open their
/// account page: the operator is usually granting it *in order to* send someone their link,
/// and a blank code column would mean granting, telling them to log in, then coming back.
///
/// Revoking leaves every referral this account already made intact and still paying out.
/// The promise was made when each one was bound; withdrawing the privilege stops new
/// recruiting, it does not claw back what was earned.
pub async fn admin_grant(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<GrantReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    let done = sqlx::query("UPDATE users SET referral_enabled = $2, updated_at = now() WHERE id = $1")
        .bind(id)
        .bind(req.enabled)
        .execute(&state.db)
        .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::bad("账号不存在"));
    }

    let code = if req.enabled {
        Some(code_for(&state, id).await?)
    } else {
        None
    };

    Ok(Json(json!({ "enabled": req.enabled, "code": code })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_cannot_contain_a_character_people_misread() {
        let alphabet = std::str::from_utf8(ALPHABET).unwrap();
        // The pairs that get confused reading a code off a screenshot or hearing it aloud.
        for bad in ['0', 'O', '1', 'I', 'L'] {
            assert!(
                !alphabet.contains(bad),
                "`{bad}` is confusable and must not appear in an invite code"
            );
        }
        let code = new_code();
        assert_eq!(code.len(), CODE_LEN);
        assert!(code.chars().all(|c| alphabet.contains(c)));
    }

    /// Two codes in a row being equal would mean the generator is not random, and every
    /// account would end up sharing one code — which is to say, one person's earnings.
    #[test]
    fn codes_differ() {
        let a = new_code();
        assert!((0..20).any(|_| new_code() != a));
    }

    /// The rate is applied to whole cents and rounded down; the ledger cannot settle a
    /// fraction of a cent, so recording one as owed would never be payable.
    #[test]
    fn commission_is_a_whole_number_of_cents() {
        let owed = |amount: i64, bps: i64| amount * bps / 10_000;
        assert_eq!(owed(10_000, 3000), 3_000); // $100 at 30% = $30
        assert_eq!(owed(999, 3000), 299); // 299.7 rounds down
        assert_eq!(owed(1, 3000), 0); // a cent at 30% owes nothing
        assert_eq!(owed(10_000, 0), 0); // programme at 0% owes nothing
        assert_eq!(owed(10_000, 10_000), 10_000); // 100% never exceeds the payment
    }

    /// The promise made to a referrer must not be rewritten by a later settings change.
    #[test]
    fn terms_are_frozen_onto_the_referral_row() {
        let src = include_str!("referral.rs");
        let claim = src.split("pub async fn claim").nth(1).expect("claim");
        let claim = &claim[..claim.find("\n// ---").unwrap_or(claim.len())];
        assert!(
            claim.contains("rate_bps, expires_at"),
            "claiming must stamp the rate and expiry onto the referral"
        );

        let award = src.split("pub async fn award").nth(1).expect("award");
        let award = &award[..award.find("\n// ---").unwrap_or(award.len())];
        assert!(
            award.contains("r.rate_bps"),
            "payout must use the rate stored on the referral, not the current setting"
        );
        /*
         * The rate and the window must not be re-read at payout — they were promised.
         *
         * Checked by column name rather than by looking for `terms(`, which also matched
         * a comment that merely mentioned the function and failed for no reason. Naming
         * the columns says what is actually forbidden: `award` does read one setting,
         * `referral_auto_settle`, and that is correct. How the operator chooses to pay is
         * not part of the deal made with the referrer; the rate and the expiry are.
         */
        assert!(
            !award.contains("referral_rate_bps") && !award.contains("referral_window_days"),
            "reading the live rate or window at payout would rewrite what was promised"
        );
        assert!(
            !award.contains("terms(&"),
            "payout must not resolve the terms afresh; they are on the referral row"
        );
    }

    /// Paying twice for one order is the failure that costs money rather than merely
    /// looking wrong.
    #[test]
    fn one_order_can_only_pay_out_once() {
        let src = include_str!("referral.rs");
        assert!(
            src.contains("ON CONFLICT (order_id) WHERE source = 'referral'"),
            "a duplicate webhook must not raise a second commission for the same order"
        );
        let migration = include_str!("../migrations/20260819_referrals.sql");
        assert!(
            migration.contains("idx_commissions_referral_order"),
            "the ON CONFLICT above needs the matching partial unique index to exist"
        );
    }

    /// The same money must not be requested twice.
    ///
    /// A pending request counts against the balance the moment it is made. Without that,
    /// someone can ask for their whole balance twice before the first is paid, and the
    /// operator finds out while sending the second payment.
    #[test]
    fn a_pending_request_already_counts_against_the_balance() {
        let src = include_str!("referral.rs");
        let f = src.split("async fn withdrawable").nth(1).expect("withdrawable");
        let f = &f[..f.find("\n#[derive").unwrap_or(f.len())];
        // 排除名单里必须有 rejected —— 驳回要把钱放回去。另外两个（failed / returned）
        // 是自动打款失败或被冲回，钱同样没到对方手上，见 20260828。
        assert!(
            f.contains("status NOT IN ('rejected', 'failed', 'returned')"),
            "rejected/failed/returned all mean the money never reached anyone, so none of \
             them may keep counting against the balance"
        );
        assert!(
            f.contains("status = 'settled'"),
            "only settled commission is withdrawable — pending is not yet owed"
        );
        assert!(
            f.contains(".max(0)"),
            "a negative balance would render as a minus sign next to a withdraw button"
        );

        // The request path must re-check against the same function, not trust the client.
        let w = src.split("pub async fn withdraw(").nth(1).expect("withdraw");
        let w = &w[..w.find("\n// ---").unwrap_or(w.len())];
        assert!(
            w.contains("withdrawable(&state, uid)"),
            "the amount must be re-checked server-side against the same definition"
        );
    }

    /// Automatic settlement must pay exactly once, and never on a duplicate webhook.
    ///
    /// The conflict clause makes a repeated Stripe event insert nothing; the credit has to
    /// be gated on the same fact, or the second delivery would top up the balance again
    /// for a commission that already exists.
    #[test]
    fn automatic_settlement_credits_only_when_a_row_was_written() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn award").nth(1).expect("award");
        let f = &f[..f.find("\n// ---").unwrap_or(f.len())];

        // 20260830 之后，award() 不再以任何形式发钱 —— 它只记账。
        assert!(
            f.contains("Ok(r) => r.rows_affected() > 0"),
            "`wrote` 仍要来自 INSERT 自己的影响行数，不能靠猜",
        );
        assert!(
            !f.contains("credits_cents = credits_cents +"),
            "佣金绝不能再以余额形式发放：credits_cents 是网关原始计费单位（约 663 = $1.00），\
             把美元分原样写进去等于按六分之一支付；而且当场到账就没有退款缓冲期了。\
             钱只由打款批次以现金支付一次。",
        );
        // The mode is read inside the payment transaction, and defaults to the mode that
        // hands out nothing by itself.
        assert!(f.contains("referral_auto_settle FROM app_settings"));
        assert!(f.contains(".unwrap_or(false)"), "an unreadable setting must mean manual");
        let migration = include_str!("../migrations/20260823_auto_settle.sql");
        assert!(
            migration.contains("referral_auto_settle BOOLEAN NOT NULL DEFAULT false"),
            "switching an existing programme's payout method must not happen by migration"
        );
    }

    /// The pager's denominator must not move while the operator works.
    ///
    /// Counted before the page is cut, and the served page clamped rather than refused —
    /// a stale link to page 9 of a 2-page queue should land on page 2, not on an error.
    #[test]
    fn the_payout_queue_pages_without_losing_the_tail() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn admin_withdrawals").nth(1).expect("fn");
        let f = &f[..f.find("\n#[derive").unwrap_or(f.len())];
        assert!(f.contains("count(*)::bigint FROM withdrawals"), "pages needs a real total");
        assert!(f.contains(".clamp(1, pages)"), "an out-of-range page must clamp");
        assert!(
            f.contains("LIMIT $2 OFFSET $3"),
            "the page must be cut in SQL, not by fetching everything and slicing"
        );
        assert!(
            !f.contains("LIMIT 200"),
            "the old silent cap must be gone — it dropped the tail without saying so"
        );
    }

    /// Deciding a request must be a one-way door from `pending`.
    ///
    /// Without the guard, a double-click marks an already-paid request paid a second time
    /// and moves `paid_at` — which is the record of when money actually left.
    #[test]
    fn a_withdrawal_can_only_be_decided_once() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn admin_withdraw_status").nth(1).expect("fn");
        let f = &f[..f.find("\n/// `GET").unwrap_or(f.len())];
        assert!(
            f.contains("WHERE id = $1 AND status IN ('pending', 'sending')"),
            "只有还没有结论的申请能被处理（pending 或结果不明的 sending）"
        );
        assert!(
            f.contains("已经处理过了"),
            "a second decision must say so rather than silently succeeding"
        );
        // Rejecting has to un-reserve the money; `withdrawable` excludes only rejected.
        let w = src.split("async fn withdrawable").nth(1).expect("withdrawable");
        assert!(
            w[..w.find("\n#[derive").unwrap_or(w.len())]
                .contains("status NOT IN ('rejected', 'failed', 'returned')"),
            "a rejected request must return the amount to the balance"
        );
    }

    /// The QR is an uploaded image on a payout record. It goes through the same validator
    /// as a profile picture rather than a second, laxer copy of the rules.
    #[test]
    fn an_uploaded_qr_is_validated_like_any_other_image() {
        let src = include_str!("referral.rs");
        let w = src.split("pub async fn withdraw(").nth(1).expect("withdraw");
        let w = &w[..w.find("\n// ---").unwrap_or(w.len())];
        assert!(
            w.contains("crate::auth::clean_avatar"),
            "SVG and remote URLs must be refused by the shared image validator"
        );
    }

    /// A referrer knows who they invited; everyone else does not need their address.
    ///
    /// 三条性质缺一不可，而它们互相拉扯 —— 这条规则改过两次，每次都是因为丢了其中一条：
    ///
    /// 1. **短用户名不能整个露出来。** 最早按长度分档，`a@x.io` 遮成 `a*@x.io`、
    ///    `ab@x.io` 遮成 `a*b@x.io` —— 等于没遮，而短的恰恰最好猜。
    /// 2. **不同账号必须看起来不同。** 后来改成只露首字符，安全了，但排行榜上真出现过
    ///    两行都是 `3***@qq.com` 而其实是两个人 —— 遮到认不出彼此，这一栏就没用了。
    /// 3. **星号个数不能透露长度。** 否则遮挡本身就是一条线索。
    #[test]
    fn a_referred_address_is_masked() {
        // 够长的：头 3 尾 2，中间恒 4 颗星。
        assert_eq!(mask_email("husainazam0@gmail.com"), "hus****m0@gmail.com");
        assert_eq!(mask_email("3491274438@qq.com"), "349****38@qq.com");
        assert_eq!(mask_email("kynexic@qq.com"), "kyn****ic@qq.com");
        // 中等长度：只留头部两位，尾部一概不露。
        assert_eq!(mask_email("441740@qq.com"), "44****@qq.com");
        assert_eq!(mask_email("abcd@x.io"), "ab****@x.io");
        // 极短：只留一位，且绝不因为"露了首尾就等于全露"。
        assert_eq!(mask_email("ab@x.io"), "a****@x.io");
        assert_eq!(mask_email("a@x.io"), "a****@x.io");

        // 性质 1：中间那段一定有东西被藏住。
        for addr in ["ab@x.io", "abc@x.io", "abcd@x.io", "abcdefgh@x.io"] {
            let local = addr.split('@').next().unwrap();
            let shown: String = mask_email(addr)
                .split('@').next().unwrap().chars().filter(|c| *c != '*').collect();
            assert!(shown.len() < local.len(), "{addr} 被完整露出来了：{shown}");
        }
        // 性质 2：不同账号渲染不同。
        assert_ne!(mask_email("3491274438@qq.com"), mask_email("303813717@qq.com"));
        // 性质 3：星号个数与长度无关。
        let stars = |a: &str| mask_email(a).matches('*').count();
        assert_eq!(stars("a@x.io"), stars("averyverylongmailbox@x.io"));
        assert_eq!(stars("441740@qq.com"), stars("3491274438@qq.com"));

        // Nothing usable, and nothing that panics.
        assert_eq!(mask_email("not-an-address"), "—");
        assert_eq!(mask_email("@x.io"), "****@x.io");
    }

    /// Every query in this file must bind as many values as its SQL asks for.
    ///
    /// This shipped broken twice over: two copies of the same one-parameter read had no
    /// `.bind` at all, so Postgres refused them with "bind message supplies 0 parameters,
    /// but prepared statement requires 1" and both granting a privilege and opening the
    /// referral page answered 500. Nothing caught it because the mismatch is only visible
    /// to a real database, which the unit tests never touch — so the check is on the source.
    ///
    /// Counts the highest `$n` in each query string and compares it with the number of
    /// `.bind(` calls that follow it. Crude, and it only sees this module, but it is the
    /// difference between finding this at compile time and finding it in production.
    #[test]
    fn every_query_binds_what_its_sql_asks_for() {
        let src = include_str!("referral.rs");
        // Skip the test module itself, which contains SQL-looking strings in assertions.
        let code = &src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())];

        for (i, chunk) in code.split("sqlx::query").enumerate().skip(1) {
            // The query text runs to the first `.` call after the closing paren of the
            // string; taking everything up to the first `.await` is close enough and
            // always contains both the SQL and its binds.
            let stmt = &chunk[..chunk.find(".await").unwrap_or(chunk.len())];

            let highest = (1..=9)
                .filter(|n| stmt.contains(&format!("${n}")))
                .max()
                .unwrap_or(0);
            let binds = stmt.matches(".bind(").count();

            assert!(
                binds >= highest,
                "query #{i} uses ${highest} but only binds {binds} value(s):\n{}",
                stmt.lines().take(6).collect::<Vec<_>>().join("\n")
            );
        }
    }

    /// This shipped broken: `ORDER BY (pending_cents + settled_cents)` over a flat SELECT
    /// resolves those names against `users`, where they do not exist, so the endpoint
    /// answered 500 on every call. An alias is only usable as a bare ORDER BY term, never
    /// inside an expression — the subselect is what makes them real columns.
    #[test]
    fn the_referrer_list_orders_on_columns_that_exist() {
        let src = include_str!("referral.rs");
        let body = src.split("pub async fn admin_referrers").nth(1).expect("fn");
        let body = &body[..body.find("\n#[derive").unwrap_or(body.len())];
        assert!(
            body.contains("SELECT * FROM ( \\"),
            "the aggregate must be wrapped so ORDER BY can do arithmetic on its aliases"
        );
        let order = body.split("ORDER BY").nth(1).expect("ORDER BY");
        assert!(
            order.contains("(pending_cents + settled_cents)"),
            "sorting by total earnings is the point of the wrapping"
        );

        // Paged, and the header counts come from the database rather than from the rows
        // on screen — with only twenty fetched, "1 / 124 已开通" cannot be derived here.
        assert!(
            body.contains("LIMIT $2 OFFSET $3"),
            "the account list must be paged in SQL, not truncated"
        );
        assert!(!body.contains("LIMIT 200"), "the silent 200-row cap must be gone");
        assert!(
            body.contains("count(*) FILTER (WHERE referral_enabled)"),
            "the granted count must be counted over the whole list, not the page"
        );
        // Both counts filter on the search, or the header would describe a different
        // list from the one underneath it.
        let counts = body.split("let (total, granted)").nth(1).expect("counts");
        assert!(
            counts[..counts.find("fetch_one").unwrap_or(counts.len())]
                .contains("email ILIKE '%' || $1 || '%'"),
            "the counts must respect the search filter"
        );
    }

    /// Referring pays real money, so it is handed out rather than assumed.
    #[test]
    fn recruiting_requires_a_granted_privilege() {
        let src = include_str!("referral.rs");
        let claim = src.split("pub async fn claim").nth(1).expect("claim");
        let claim = &claim[..claim.find("\n// ---").unwrap_or(claim.len())];
        assert!(
            claim.contains("AND referral_enabled"),
            "a code from an account without the privilege must not bind a referral"
        );

        let me = src.split("pub async fn me(").nth(1).expect("me");
        let me = &me[..me.find("\n#[derive").unwrap_or(me.len())];
        assert!(
            me.contains("SELECT referral_enabled FROM users"),
            "an account with no privilege must not be handed a code"
        );

        let migration = include_str!("../migrations/20260820_referral_grant.sql");
        assert!(
            migration.contains("referral_enabled BOOLEAN NOT NULL DEFAULT false"),
            "the privilege must default to off, or every account silently has it"
        );
    }

    /// Self-referral and after-the-fact attachment are the two ways to be paid for
    /// bringing in nobody.
    #[test]
    fn a_claim_refuses_the_obvious_abuses() {
        let src = include_str!("referral.rs");
        let claim = src.split("pub async fn claim").nth(1).expect("claim");
        let claim = &claim[..claim.find("\n// ---").unwrap_or(claim.len())];
        assert!(claim.contains("referrer == uid"), "self-referral must be refused");
        assert!(
            claim.contains("status = 'paid'"),
            "an account that already paid must not be able to acquire a referrer"
        );
        assert!(
            claim.contains("ON CONFLICT (referred_user_id) DO NOTHING"),
            "a second referrer for one account must not be creatable"
        );
    }
}

#[cfg(test)]
mod refund_tests {
    /// A refund must never silently take money out of somebody's balance.
    ///
    /// The three passes in `reverse` are ordered and mutually exclusive, and each one exists
    /// for a case the others get wrong. Losing any of them turns a refund into either a free
    /// commission or a surprise debit, so the shape is asserted rather than trusted.
    #[test]
    fn a_refund_reverses_only_what_has_not_been_paid_out() {
        let src = include_str!("referral.rs");
        let f = src
            .split("pub async fn reverse(")
            .nth(1)
            .expect("the reversal path must exist");
        let body = &f[..f.find("\n// ---").unwrap_or(f.len())];

        assert!(
            body.contains("status = 'pending'"),
            "unpaid commission must be reversible — that is the case with no downside",
        );
        assert!(
            body.contains("c.settled_by <> 'auto'"),
            "automatic settlement credits the balance as the row is written, so those \
             cannot be reversed by flipping a status; they must fall through to flagging",
        );
        assert!(
            body.contains("WHERE w.user_id = c.referrer_user_id AND w.status = 'paid'"),
            "a settled commission may only be reversed while the referrer's PAID withdrawals \
             are still covered without it — otherwise the money has demonstrably left",
        );
        assert!(
            body.contains("reversed_at IS NULL"),
            "the flagging pass must skip rows the earlier passes already handled, or it \
             overwrites their reason",
        );
        // The one thing that must NOT be here.
        assert!(
            !body.contains("credits_cents"),
            "reversal must never debit a balance: it can go negative, and the person may \
             already have spent it. Flag it and let an operator decide.",
        );
    }

    /// A reversed commission is not owed, not settled, and not "earned".
    ///
    /// Every total in this file filters on an explicit status, which is what lets a new one
    /// be added safely — but the two subselects that answer "how much has this referral
    /// produced" used `<> 'rejected'`, which would have counted reversals as earnings.
    #[test]
    fn reversed_commission_counts_towards_nothing() {
        let src = include_str!("referral.rs");
        let body = &src[..src.find("mod refund_tests").unwrap_or(src.len())];
        assert_eq!(
            body.matches("AND c.status NOT IN ('rejected', 'reversed')").count(),
            2,
            "both 已产生佣金 subselects (my_referrals and admin_referred) must exclude reversals",
        );
        // withdrawable() is the one that decides what can be asked for.
        let w = body
            .split("async fn withdrawable")
            .nth(1)
            .expect("withdrawable");
        assert!(
            w[..w.find("\n#[derive").unwrap_or(w.len())].contains("status = 'settled'"),
            "withdrawable must count settled only, so a reversed row drops out on its own",
        );
    }

    /// Marking a payout paid has to record who and what, in the same statement.
    #[test]
    fn a_payout_records_who_sent_it_and_what_reference() {
        let src = include_str!("referral.rs");
        let f = src
            .split("pub async fn admin_withdraw_status")
            .nth(1)
            .expect("fn");
        let body = &f[..f.find("\n/// `GET").unwrap_or(f.len())];
        assert!(
            body.contains("paid_by = CASE WHEN $2 = 'paid'") && body.contains("reference = CASE"),
            "who paid and the transfer reference must be written by the same UPDATE that \
             flips the status — an audit trail written afterwards can be missing",
        );
        assert!(
            body.contains(".bind(&claims.email)"),
            "paid_by must come from the token, never from the request body: it records who \
             acted, so the caller does not get to choose what it says",
        );
        assert!(
            body.contains("WHERE id = $1 AND status IN ('pending', 'sending')"),
            "只有还没有结论的申请能被处理。'sending' 也在里面是刻意的：那些是结果不明、\
             等人去 Stripe 核对的行 —— 不让处理它们，佣金就永远卡在 paid。\
             已经 paid/rejected 的仍然改不动，双击不会把付款时间挪走。",
        );
    }
}

#[cfg(test)]
mod audit_regression_tests {
    /// 自动结算已经把钱发过一次了，不能再让它变成可提现的现金。
    ///
    /// 这是整套佣金里最贵的一个洞：award() 在写 status='settled' 的同时把等额的钱加进了
    /// users.credits_cents，而 withdrawable() 又把所有 settled 的行算作可提现。30% 的方案
    /// 实际按 60% 支付，每一笔销售、每一次续费都重复一遍。20260823 的迁移用文字写明了
    /// 「自动结算之后没有可提的东西」，但在这条断言之前，服务端没有一行代码执行它。
    #[test]
    fn commission_is_never_paid_twice_nor_as_credit() {
        let src = include_str!("referral.rs");
        let f = src.split("async fn withdrawable").nth(1).expect("withdrawable");
        let f = &f[..f.find("\n#[derive").unwrap_or(f.len())];
        assert!(
            f.contains("reversed_at IS NULL"),
            "退款撤销过的、以及旧版自动结算已经用余额发过的，都不能再提走",
        );
        // 提现请求那条路必须用同一个口径重算，不能只在展示时过滤。
        let w = src.split("pub async fn withdraw(").nth(1).expect("withdraw");
        let w = &w[..w.find("\n// ---").unwrap_or(w.len())];
        assert!(
            w.contains("reversed_at IS NULL"),
            "锁内重算必须和 withdrawable 用同一个口径，否则并发那条路会绕开它",
        );
        // 真正的防线：整个 award 路径不许碰余额。
        let a = src.split("pub async fn award").nth(1).expect("award");
        let a = &a[..a.find("\n// ---").unwrap_or(a.len())];
        assert!(
            !a.contains("credits_cents = credits_cents +"),
            "同一笔佣金一旦既进余额又能提现，就是付两次 —— 现在只留现金这一条路",
        );
    }

    /// 同一个账号的提现必须串行。
    #[test]
    fn concurrent_withdrawals_cannot_overdraw() {
        let src = include_str!("referral.rs");
        let w = src.split("pub async fn withdraw(").nth(1).expect("withdraw");
        let w = &w[..w.find("\n// ---").unwrap_or(w.len())];
        assert!(
            w.contains("FOR UPDATE"),
            "没有行锁，五个并发请求会在同一份余额上各插一行 —— 每行自带不同的 id，而那个 id \
             正是 Stripe 的幂等键，所以 Stripe 看到的是五笔不同的转账",
        );
        let lock = w.find("FOR UPDATE").unwrap();
        let insert = w.find("INSERT INTO withdrawals").expect("insert");
        assert!(lock < insert, "锁必须在插入之前拿到");
        assert!(
            w.contains("tx.commit().await?;"),
            "锁要在调 Stripe 之前释放：转账不能待在事务里，ROLLBACK 不会把钱要回来",
        );
    }

    /// 佣金写失败不能把客户的付款一起带走。
    #[test]
    fn a_failed_commission_cannot_discard_the_payment() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn award").nth(1).expect("award");
        let f = &f[..f.find("\n// ---").unwrap_or(f.len())];
        assert!(
            f.contains("tx.as_mut().begin().await"),
            "award 的写入必须在 SAVEPOINT 上：吞掉 sqlx 错误并不能让 Postgres 事务恢复，\
             外层的订单认领、权益发放、事件去重会一起回滚，而 commit() 返回 ROLLBACK \
             命令标签、sqlx 报成功 —— 客户付了钱，什么也没拿到",
        );
        assert!(
            f.contains("sp.rollback()") && f.contains("sp.commit()"),
            "两条路都要收尾，否则 savepoint 悬在那里",
        );
    }

    /// 人民币销售要换算，不能按美元原样记账，也不能悄悄跳过。
    ///
    /// 这条断言换过一次方向。上一版是「非美元一律跳过」—— 当时账本没有币种概念，不挡就会
    /// 把 18800 分人民币当成 $188 付佣金。中国区开始按人民币收款之后，跳过等于每一笔中国
    /// 销售的推荐人都拿不到钱。现在的做法是写入时折一次，汇率钉在行上。
    #[test]
    fn a_cny_sale_is_converted_not_skipped_and_not_taken_at_face_value() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn award").nth(1).expect("award");
        let f = &f[..f.find("\n// ---").unwrap_or(f.len())];

        assert!(
            f.contains(r#""cny" => {"#) && f.contains("crate::settings::usd_per_cny_bps()"),
            "人民币销售必须按结算汇率折成美元，而不是跳过",
        );
        assert!(
            f.contains("let commission_cents = basis_usd * rate_bps"),
            "佣金必须从折算后的美元基数算，从原币金额算就是按面值付钱",
        );
        assert!(
            f.contains(".bind(&ccy)") && f.contains(".bind(fx_bps as i32)"),
            "原币种和当时的汇率要钉在行上：以后调汇率不能改写已经记下的佣金",
        );
        // 不认识的币种仍然必须拒绝 —— 那才是当初那道闸的价值所在。
        assert!(
            f.contains("不认识的销售币种"),
            "usd / cny 之外的币种没有换算依据，只能跳过并喊出来",
        );
    }

    /// 拒付打赢了只该撤销拒付造成的那次。
    #[test]
    fn a_won_dispute_does_not_undo_an_unrelated_refund() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn unreverse").nth(1).expect("unreverse");
        let f = &f[..f.find("\n// ---").unwrap_or(f.len())];
        assert!(
            f.contains("reversal_reason = 'charge.dispute.created'"),
            "没有这个条件，赢了拒付会把同一订单上因退款撤销的佣金也一起还原",
        );
    }
}

#[cfg(test)]
mod attribution_tests {
    /// 窗口按付款时刻判，不按 webhook 送达时刻。
    ///
    /// Stripe 会重投三天，对账扫描十分钟一轮 —— 按 now() 判，一笔明明在窗口内付的钱会
    /// 因为送达晚了而一分佣金拿不到，而且没有任何提示。
    #[test]
    fn the_window_is_measured_at_payment_time() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn award").nth(1).expect("award");
        let f = &f[..f.find("\n// ---").unwrap_or(f.len())];
        assert!(
            f.contains("r.expires_at > to_timestamp($2)"),
            "窗口要和付款时刻比，不能和 now() 比",
        );
        assert!(
            !f.contains("r.expires_at > now()"),
            "旧的 now() 判法不能留着",
        );
    }

    /// 推荐关系必须早于这笔付款。
    ///
    /// claim() 只挡了「已经有 status='paid' 的订单」，但订单是下单时以 pending 写进去的，
    /// 要到 webhook 才翻成 paid。中间那段时间绑一个推荐人，就能对一笔已经付掉的钱抽成。
    #[test]
    fn a_referrer_bound_after_the_charge_earns_nothing() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn award").nth(1).expect("award");
        let f = &f[..f.find("\n// ---").unwrap_or(f.len())];
        assert!(
            f.contains("r.created_at < to_timestamp($2)"),
            "绑定时间必须早于付款时间，否则 claim() 那道闸有一个和订单状态一样宽的缝",
        );
    }
}

#[cfg(test)]
mod signup_binding_tests {
    /// 已有账号不能靠邀请码变成谁的推荐用户。
    ///
    /// 挡的是两种真实情况：老用户帮朋友点了一下邀请链接（码被烧在老账号上，朋友的推荐永久
    /// 没了，因为推荐人名额只有一个），以及老用户自己找个码绑上、让之后的消费凭空给别人分成。
    #[test]
    fn only_a_newly_registered_account_can_be_referred() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn claim(").nth(1).expect("claim");
        let f = &f[..f.find("\n/// 注册那一刻").unwrap_or(f.len())];
        assert!(
            f.contains("created_at > now() - interval '24 hours'"),
            "必须按注册时间判断。用「有没有付过款」只能挡住花过钱的人 —— \
             一个注册半年、一分没花的老账号照样能绑",
        );
        // 比较交给 SQL 做，不要把秒数取回 Rust 再算：EXTRACT 在 PG14+ 返回 NUMERIC，
        // 解成 f64 会在运行时炸，而编译期毫无提示。
        assert!(
            !f.contains("EXTRACT(EPOCH"),
            "别再用 EXTRACT 取秒数了 —— 那条路踩过一次 500",
        );
        assert!(
            f.contains("邀请码只能在注册时使用"),
            "拒绝时要说清是为什么，否则用户只会以为码坏了",
        );
    }

    /// 注册那一刻就能绑，且绑不上不能让注册失败。
    #[test]
    fn signup_binding_never_blocks_registration() {
        let src = include_str!("referral.rs");
        let f = src.split("pub async fn bind_at_signup(").nth(1).expect("bind_at_signup");
        let f = &f[..f.find("\n// ---").unwrap_or(f.len())];
        // 返回 Option 而不是 Result —— 类型上就没有「往外抛错」这个可能。
        // （函数体里的 `found?` 是 Option 的 ?，走的是返回 None。）
        assert!(
            f.contains("-> Option<String>") && !f.contains("return Err("),
            "这个函数不能往外抛错：账号已经建好了，一个绑不上的推荐关系不该把真实用户挡在门外",
        );
        assert!(
            f.contains("referrer == uid"),
            "自己不能推荐自己，注册这条路同样要挡",
        );
        assert!(
            f.contains("ON CONFLICT (referred_user_id) DO NOTHING"),
            "推荐人名额只有一个，重复绑定要由数据库挡住",
        );
        assert!(
            f.contains("referral_enabled"),
            "资格被收回的人，他的码就该当作无效",
        );
    }

    /// 注册接口要收邀请码 —— 桌面端只有这一条路。
    #[test]
    fn registration_accepts_a_code() {
        let a = include_str!("auth.rs");
        assert!(
            a.contains("pub referral_code: Option<String>"),
            "注册请求要能带邀请码，否则「点链接→下载 App→在 App 里注册」整条推荐会丢",
        );
        assert!(
            a.contains("crate::referral::bind_at_signup(&state, user.id, code)"),
            "建号之后要立刻绑",
        );
    }
}
