//! 每家中转的**充值汇率**，以及「谁真的便宜」这个问题的唯一答案。
//!
//! # 倍率不可跨站比较
//!
//! `route_endpoints.cost_ratio` 是「相对官方价的倍数」，看起来是个纯数，其实带单位：
//! **那家中转自己的余额单位**。而一块钱余额要花多少人民币，各家差几十倍。
//!
//! 线上现成的例子：梦幻API 的出口写着 0.05 倍，hanhegufei 的自带地址是 1.0 倍。
//! 按倍率排是二十倍差距；把充值汇率算进来完全可能反过来。而选路**一直**按 cost_ratio
//! 排序 —— 也就是说它一直在按一个不可比的数挑「最便宜的门」。
//!
//! 唯一可比的量是：**每一美元官方价，实际要花多少人民币**
//!
//! ```text
//! 人民币 / 官方美元 = cost_ratio ÷ usd_per_cny
//! ```
//!
//! # 不知道就不换算
//!
//! 没填汇率的站，这个量算不出来。这时**不能**拿 1.0 顶上去 —— 那是把「不知道」
//! 当成「一比一」，会让一个没填汇率的站凭空排到前面去。判据是全有全无：
//! 一条线路下所有候选都知道汇率，才按人民币排；只要缺一个，整条线路退回按倍率排
//! （＝和今天一模一样）。见 `expand`。
//!
//! 这条规矩和这个仓库里另外两条是同一条：「没查到 ≠ 没有」、「枚举没判据就恒等于默认值」。

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// host（小写，不含协议和路径）→ ¥1 能买到多少上游余额单位。
///
/// 进内存的理由和 `settings` 一样：选路路径上每个请求都要读它，而它一天改不了一次。
static RATES: LazyLock<RwLock<HashMap<String, f64>>> = LazyLock::new(Default::default);

/// 汇率的合法区间。
///
/// 上限一百万不是随手写的：有的中转按「1 人民币 = 50 万额度」标价（额度不是美元，
/// 是它自己的计价单位）。真正要挡的只有 0 和负数 —— 它们会让下面那个除法炸掉或反号。
const MAX_RATE: f64 = 1_000_000.0;

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

/// 从 base_url 取站点主机名。
///
/// 比价的粒度是**站**，不是出口：一家站底下挂着好几条线路的好几个出口，
/// 而「充值汇率」是站的属性 —— 你在那家站充一次钱，它底下所有出口一起花。
/// 按出口存的话，同一家站要填好几遍，填错一个就有一个出口排错位置。
pub fn host_of(base_url: &str) -> String {
    let s = base_url.trim();
    let s = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s);
    // 端口保留：换端口通常还是同一家，但真要区分时也不该被我们悄悄合并。
    s.split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

/// 这个地址所在的站，¥1 买到多少上游余额单位。没填过就是 None。
pub fn usd_per_cny(base_url: &str) -> Option<f64> {
    let host = host_of(base_url);
    if host.is_empty() {
        return None;
    }
    RATES
        .read()
        .ok()?
        .get(&host)
        .copied()
        .filter(|r| r.is_finite() && *r > 0.0)
}

/// **每一美元官方价，实际要花多少人民币。** 小的便宜。
///
/// 这是跨中转唯一可比的量。返回 None 就是「这个站的汇率没填，算不出来」——
/// 调用方必须把它当成「不知道」，不许兜底成一个数。
pub fn cny_per_official_usd(cost_ratio: f64, usd_per_cny: f64) -> Option<f64> {
    if !cost_ratio.is_finite() || cost_ratio < 0.0 {
        return None;
    }
    if !usd_per_cny.is_finite() || usd_per_cny <= 0.0 {
        return None;
    }
    Some(cost_ratio / usd_per_cny)
}

/// 启动时装载一次；每次写入后重新装载。
pub async fn load(db: &sqlx::PgPool) {
    match sqlx::query_as::<_, (String, f64)>(
        "SELECT lower(host), usd_per_cny FROM channel_rates WHERE host <> ''",
    )
    .fetch_all(db)
    .await
    {
        Ok(rows) => {
            let map: HashMap<String, f64> = rows
                .into_iter()
                .filter(|(h, r)| !h.is_empty() && r.is_finite() && *r > 0.0)
                .collect();
            let n = map.len();
            if let Ok(mut g) = RATES.write() {
                *g = map;
            }
            tracing::info!(sites = n, "中转充值汇率已装载");
        }
        // host 这一列是后加的：迁移还没跑的老库读它会报错。那就当成「一个都没填」——
        // 与改造前完全一致的行为（全部按倍率排），而不是让整个启动挂掉。
        Err(e) => tracing::warn!("充值汇率读取失败，选路沿用「只按倍率排」的旧行为: {e}"),
    }
}

/// 只给测试用：直接摆一份汇率表进缓存。
#[cfg(test)]
pub fn set_for_test(pairs: &[(&str, f64)]) {
    if let Ok(mut g) = RATES.write() {
        *g = pairs
            .iter()
            .map(|(h, r)| (h.to_ascii_lowercase(), *r))
            .collect();
    }
}

// ---------- 管理接口 ----------

/// 一家站的全貌：谁在用它、我填的汇率、自动抓到的套餐、换算出来的真实成本。
#[derive(Debug, serde::Serialize)]
pub struct SiteRow {
    pub host: String,
    /// 用到这家站的线路/出口。
    pub users: Vec<SiteUser>,
    /// 我手填的汇率（¥1 买多少上游余额单位）。
    pub usd_per_cny: Option<f64>,
    pub note: String,
    /// 自动抓到的充值套餐推算出来的汇率区间。空 = 一档都没抓到。
    pub auto_rates: Vec<f64>,
    /// 这家站最便宜的那个出口，折算成「每一美元官方价花多少人民币」。
    /// None = 汇率没填，算不出来。
    pub cny_per_official_usd: Option<f64>,
    /// 这家站上最低的倍率（用来算上面那个数的）。
    pub best_ratio: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct SiteUser {
    pub route_label: String,
    pub cost_ratio: f64,
    pub is_own: bool,
    pub endpoint_label: String,
}

/// GET /api/admin/relay-rates
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    // 站点名单来自**正在用的地址**，不是来自汇率表：没填过汇率的站恰恰是最需要
    // 出现在这一屏上的。只列汇率表里有的，等于只显示已经解决的问题。
    let owns: Vec<(String, String)> =
        sqlx::query_as("SELECT base_url, label FROM models WHERE active AND base_url <> ''")
            .fetch_all(&state.db)
            .await?;
    let outlets: Vec<(String, String, f64, String)> = sqlx::query_as(
        "SELECT e.base_url, m.label, e.cost_ratio, e.label \
         FROM route_endpoints e JOIN models m ON m.id = e.route_id \
         WHERE e.active AND e.base_url <> ''",
    )
    .fetch_all(&state.db)
    .await?;

    let mut by_host: HashMap<String, Vec<SiteUser>> = HashMap::new();
    for (url, route) in owns {
        by_host.entry(host_of(&url)).or_default().push(SiteUser {
            route_label: route,
            cost_ratio: 1.0,
            is_own: true,
            endpoint_label: String::new(),
        });
    }
    for (url, route, ratio, elabel) in outlets {
        by_host.entry(host_of(&url)).or_default().push(SiteUser {
            route_label: route,
            cost_ratio: ratio,
            is_own: false,
            endpoint_label: elabel,
        });
    }

    let saved: Vec<(String, f64, String)> =
        sqlx::query_as("SELECT lower(host), usd_per_cny, note FROM channel_rates WHERE host <> ''")
            .fetch_all(&state.db)
            .await?;
    let saved: HashMap<String, (f64, String)> =
        saved.into_iter().map(|(h, r, n)| (h, (r, n))).collect();
    // 汇率表里有、但已经没有任何线路在用的站也要列出来 —— 否则它会变成一条
    // 删不掉也看不见的记录。
    for h in saved.keys() {
        by_host.entry(h.clone()).or_default();
    }

    // 自动抓到的套餐比例，按站归拢。
    let plans: Vec<(String, f64)> = sqlx::query_as(
        "SELECT e.base_url, p.rate FROM endpoint_topup_plan p \
         JOIN route_endpoints e ON e.id = p.endpoint_id WHERE p.rate IS NOT NULL",
    )
    .fetch_all(&state.db)
    .await?;
    let mut auto: HashMap<String, Vec<f64>> = HashMap::new();
    for (url, rate) in plans {
        if rate.is_finite() && rate > 0.0 {
            auto.entry(host_of(&url)).or_default().push(rate);
        }
    }

    let mut rows: Vec<SiteRow> = by_host
        .into_iter()
        .map(|(host, mut users)| {
            users.sort_by(|a, b| {
                a.cost_ratio
                    .partial_cmp(&b.cost_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.route_label.cmp(&b.route_label))
            });
            let (rate, note) = saved
                .get(&host)
                .map(|(r, n)| (Some(*r), n.clone()))
                .unwrap_or((None, String::new()));
            // 「这家站有多便宜」看它最便宜的那个出口 —— 选路本来就会挑那个。
            let best_ratio = users
                .iter()
                .map(|u| u.cost_ratio)
                .filter(|r| r.is_finite() && *r > 0.0)
                .fold(None::<f64>, |acc, r| Some(acc.map_or(r, |a: f64| a.min(r))));
            let real = match (best_ratio, rate) {
                (Some(br), Some(r)) => cny_per_official_usd(br, r),
                _ => None,
            };
            let mut ar = auto.remove(&host).unwrap_or_default();
            ar.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            SiteRow {
                host,
                users,
                usd_per_cny: rate,
                note,
                auto_rates: ar,
                cny_per_official_usd: real,
                best_ratio,
            }
        })
        .collect();

    // 算得出真实成本的排前面（便宜的更前），算不出的垫底 —— 那一批才是要你去填的。
    rows.sort_by(|a, b| match (a.cny_per_official_usd, b.cny_per_official_usd) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.host.cmp(&b.host),
    });

    // 选路当下到底按什么排，必须让人看得见 —— 否则「填了汇率有没有生效」只能靠猜。
    let known = rows.iter().filter(|r| r.usd_per_cny.is_some()).count();
    Ok(Json(json!({
        "rows": rows,
        "sites": rows.len(),
        "with_rate": known,
        "all_known": known == rows.len() && !rows.is_empty(),
    })))
}

#[derive(Debug, Deserialize)]
pub struct SaveReq {
    pub host: String,
    /// 空 / 0 / 负数 = 把这家站的汇率清掉（回到「按倍率排」）。
    pub usd_per_cny: Option<f64>,
    pub note: Option<String>,
}

/// POST /api/admin/relay-rates
pub async fn admin_save(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<SaveReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let host = host_of(&req.host);
    if host.is_empty() {
        return Err(AppError::bad("站点地址不能为空"));
    }
    let note = req.note.unwrap_or_default().trim().to_string();
    if note.chars().count() > 500 {
        return Err(AppError::bad("备注不能超过 500 个字符"));
    }

    match req.usd_per_cny.filter(|r| r.is_finite() && *r > 0.0) {
        None => {
            // 清掉。留着一条 0 或者 NULL 比删掉更糟：`usd_per_cny > 0` 是表上的 CHECK，
            // 而「填了个 0」和「没填」在选路那边必须是同一件事。
            sqlx::query("DELETE FROM channel_rates WHERE lower(host) = $1")
                .bind(&host)
                .execute(&state.db)
                .await?;
        }
        Some(rate) => {
            if rate > MAX_RATE {
                return Err(AppError::bad("充值汇率大得不像真的 —— 小数点是不是点错了"));
            }
            // 先改后插，不用 ON CONFLICT：这张表上有**两个**唯一索引（name 和 host），
            // 而 ON CONFLICT 只能盯住一个。盯 host 的话，名字撞车会直接抛出去，
            // 保存表现为一句「渠道名称已存在」——而这一屏根本没有名字这个字段，
            // 那句话在这儿是天书。
            let upd = sqlx::query(
                "UPDATE channel_rates SET usd_per_cny = $2, note = $3, updated_at = now() \
                 WHERE lower(host) = $1",
            )
            .bind(&host)
            .bind(rate)
            .bind(&note)
            .execute(&state.db)
            .await?;
            if upd.rows_affected() == 0 {
                // 还没有这一行。名字默认用站点名；万一定价试算那边已经占了这个名字，
                // 加个后缀重试 —— 名字只是给人看的，host 才是这一屏认的键。
                let mut name = host.clone();
                let mut placed = false;
                for attempt in 1..=5 {
                    let r = sqlx::query(
                        "INSERT INTO channel_rates (name, usd_per_cny, note, host) \
                         VALUES ($1, $2, $3, $4)",
                    )
                    .bind(&name)
                    .bind(rate)
                    .bind(&note)
                    .bind(&host)
                    .execute(&state.db)
                    .await;
                    match r {
                        Ok(x) if x.rows_affected() == 1 => {
                            placed = true;
                            break;
                        }
                        Ok(_) => break,
                        Err(e)
                            if e.as_database_error().and_then(|d| d.code()).as_deref()
                                == Some("23505")
                                && attempt < 5 =>
                        {
                            name = format!("{host} ({})", attempt + 1);
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                // 「保存成功」这句话只许来自执行事实。写不进去还回 ok 的话，
                // 界面会显示新汇率、选路继续按旧的排，两边都不报错。
                if !placed {
                    tracing::error!(host = %host, "充值汇率没写进去");
                    return Err(AppError::bad("没保存上，请重试"));
                }
            }
        }
    }

    // 缓存必须当场重载：不重载的话，界面上写着新汇率，选路还在按旧的排，
    // 而两者都不会报错 —— 这正是最难查的那一类。
    load(&state.db).await;
    Ok(Json(json!({ "ok": true, "host": host })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_host_is_the_site_not_the_url() {
        // 同一家站的三种写法必须归成一个 —— 否则同一家要填三遍汇率，
        // 而填漏一个就有一个出口按「没填」处理，整条线路退回按倍率排。
        for u in [
            "https://mhapi.net",
            "https://mhapi.net/v1",
            "http://MHAPI.net/v1/",
            "mhapi.net/api/v1",
        ] {
            assert_eq!(host_of(u), "mhapi.net", "{u} 没归到同一个站");
        }
        // 不同站不许合并。
        assert_eq!(host_of("https://openrouter.ai/api/v1"), "openrouter.ai");
        assert_eq!(host_of("https://us.neuracraft.org/v1"), "us.neuracraft.org");
        assert_eq!(host_of("   "), "");
    }

    #[test]
    fn the_comparable_number_is_cny_per_official_dollar() {
        // 线上那两家：hanhegufei ¥1 买 10 额度、梦幻 ¥1 买 0.14 美元。
        let hanhe = cny_per_official_usd(1.0, 10.0).unwrap();
        let meng = cny_per_official_usd(0.05, 0.14).unwrap();
        assert!((hanhe - 0.1).abs() < 1e-12);
        assert!((meng - 0.35714285714285715).abs() < 1e-12);
        // 倍率说梦幻便宜二十倍，人民币说贵三倍半。这就是这个模块存在的全部理由。
        assert!(0.05 < 1.0, "倍率给出的结论");
        assert!(meng > hanhe, "换算成人民币之后结论必须反过来");

        // 免费出口（0 倍）仍然是最便宜的，不是「算不出来」。
        assert_eq!(cny_per_official_usd(0.0, 7.0), Some(0.0));
        // 汇率非法 = 算不出来，绝不兜底成一个数。
        assert_eq!(cny_per_official_usd(1.0, 0.0), None);
        assert_eq!(cny_per_official_usd(1.0, -3.0), None);
        assert_eq!(cny_per_official_usd(1.0, f64::NAN), None);
        assert_eq!(cny_per_official_usd(f64::NAN, 7.0), None);
    }

    /// 「不知道」和「一比一」必须是两件事。
    ///
    /// 缓存里没有这家站，`usd_per_cny` 要返回 None。返回 Some(1.0) 的话，
    /// 一个纯粹没填汇率的站会被当成「¥1 买 1 美元额度」—— 那是个极好的汇率，
    /// 它会凭空排到所有人前面，而且没有任何地方会报错。
    #[test]
    fn an_unset_site_reads_as_unknown_not_as_one() {
        set_for_test(&[("mhapi.net", 0.14)]);
        assert_eq!(usd_per_cny("https://mhapi.net/v1"), Some(0.14));
        assert_eq!(usd_per_cny("https://never-configured.example.com"), None);
        // 0 和负数存进去也算没填 —— 表上有 CHECK，但缓存不该指望它。
        set_for_test(&[("zero.example.com", 0.0), ("neg.example.com", -1.0)]);
        assert_eq!(usd_per_cny("https://zero.example.com"), None);
        assert_eq!(usd_per_cny("https://neg.example.com"), None);
        set_for_test(&[]);
    }
}

// ---------- 逐模型比价 ----------
//
// 站级的「最低倍率」只够回答「这家整体便宜不便宜」，而钱是**按模型**花的：
// 同一家中转，claude-opus-5 可能便宜、gpt-5.6-sol 可能贵。而且 `cost_ratio` 是手填的
// 近似值，`endpoint_auto_price` 才是从上游价目表真抓下来的逐模型单价 —— 有它就没有
// 理由再用前者去猜。

#[derive(Debug, serde::Serialize)]
pub struct Offer {
    /// 去重键，前端拿它当 React key。
    pub key: String,
    pub host: String,
    /// 走这一家、这一档价的线路有哪几条。**同一家同一个价被三条线路各挂一次
    /// 不是三个选择**，是一个 —— 把它们排成一低/二低/三低是噪音，还会让人
    /// 以为「换一个能省钱」。
    pub via: Vec<String>,
    pub endpoint_id: uuid::Uuid,
    /// 上游自己标的价（它自己的余额单位，每百万 token）。
    pub input_raw: f64,
    pub output_raw: f64,
    pub group_name: String,
    pub group_multiplier: f64,
    /// 价是抓来的还是手录的。
    pub source: &'static str,
    /// 换算成人民币的每百万 token。None = 这家站没填汇率，算不出来。
    pub input_cny: Option<f64>,
    pub output_cny: Option<f64>,
    /// 按这个模型的真实 token 配比混出来的一个数，排名次用的就是它。
    pub blended_cny: Option<f64>,
    /// 名次：1 = 最低。只有算得出人民币的才有名次。
    pub rank: Option<usize>,
    pub probe_ms: Option<i32>,
    pub probe_ok: Option<bool>,
    /// 这个模型的候选里最快的那个。
    pub fastest: bool,
    /// 慢得离谱（判据和选路同一套：比最快的慢三倍以上且自己超过五秒）。
    pub slow: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct ModelRow {
    pub model_id: String,
    /// 这个模型现在开放给用户没有。没开放的也列出来 —— 那正是「该不该开」的依据。
    pub open: bool,
    /// 排名用的配比**是怎么来的**。这一栏必须回给界面：一个混合价如果不说清楚
    /// 是按什么配比混的，它就是个没有单位的数。
    pub mix_source: &'static str,
    pub mix_in: f64,
    pub mix_cached: f64,
    pub mix_out: f64,
    pub mix_calls: i64,
    pub offers: Vec<Offer>,
    /// 第一低比第二低便宜多少（百分比）。差得少就没必要为它牺牲速度。
    pub gap_pct: Option<f64>,
}

/// 这个模型的真实 token 配比 →（普通输入, 缓存输入, 输出）三个权重。
///
/// # 为什么权重必须来自真实用量
///
/// 一个「混合价」如果不说清楚按什么配比混的，它就是个没有单位的数。随手拍一个
/// 「输入 3 输出 1」看起来很合理，而线上 grok-4.6 的真实形状是
/// **输入 24% / 缓存 74% / 输出 2%** —— 缓存占了四分之三。按 3:1 排出来的名次
/// 和按真实配比排出来的完全可能是两个答案，而前者看起来一样自信。
///
/// `cached` 夹在 `[0, prompt]` 里：它**包含在** prompt 里（见 endpoint_model_usage
/// 的注释）。不夹的话，上游多报一点就会算出负的普通输入权重，混合价直接变成负数 ——
/// 那会让这家站稳稳排第一低。
pub fn mix_weights(prompt: i64, completion: i64, cached: i64) -> Option<(f64, f64, f64)> {
    let total = prompt.saturating_add(completion);
    if total <= 0 {
        return None;
    }
    let total = total as f64;
    let cached = cached.clamp(0, prompt) as f64;
    let plain = (prompt as f64 - cached).max(0.0);
    Some((plain / total, cached / total, completion.max(0) as f64 / total))
}

/// GET /api/admin/relay-model-prices
pub async fn admin_model_prices(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    let rates: HashMap<String, f64> = sqlx::query_as::<_, (String, f64)>(
        "SELECT lower(host), usd_per_cny FROM channel_rates WHERE host <> ''",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .filter(|(_, r)| r.is_finite() && *r > 0.0)
    .collect();

    // 抓来的价。倍率已经乘进单价里了（见 endpoint_auto_price 的注释），
    // 留着 group_multiplier 只是为了让界面能解释「为什么这家比官网便宜十几倍」。
    type PriceRow = (
        uuid::Uuid,
        String,
        String,
        String,
        String,
        f64,
        f64,
        Option<f64>,
        String,
        f64,
        Option<i32>,
        Option<bool>,
    );
    // **两套命名空间都要捞。**
    //
    // `endpoint_auto_price.endpoint_id` 对出口是 route_endpoints.id，对线路自带地址是
    // models.id（和 endpoint_usage / health_id 同一套约定）。上一版只 JOIN 了
    // route_endpoints —— 线上 682 条真实价格里**只捞到 106 条**，576 条被静默丢掉，
    // 而丢掉的恰恰是最大的两份目录（openrouter 412 个模型、hanhegufei 164 个）。
    // 表现不是报错，是「翻来覆去只有那两家」，看起来像数据本来就少。
    let auto: Vec<PriceRow> = sqlx::query_as(
        "SELECT e.id, p.model_id, e.base_url, m.label, e.label, \
                p.input_per_mtok, p.output_per_mtok, p.cached_per_mtok, \
                p.group_name, p.group_multiplier, e.probe_ms, e.probe_ok \
         FROM endpoint_auto_price p \
         JOIN route_endpoints e ON e.id = p.endpoint_id \
         JOIN models m ON m.id = e.route_id \
         WHERE e.active \
         UNION ALL \
         SELECT m.id, p.model_id, m.base_url, m.label, '', \
                p.input_per_mtok, p.output_per_mtok, p.cached_per_mtok, \
                p.group_name, p.group_multiplier, \
                NULL::int, NULL::boolean \
         FROM endpoint_auto_price p \
         JOIN models m ON m.id = p.endpoint_id \
         WHERE m.active",
    )
    .fetch_all(&state.db)
    .await?;

    // 手录的价。**只在没有抓来的价时才用** —— 和对账那边同一条优先级（自动覆盖手工），
    // 两处不一致的话，这一屏说的最便宜和账单算的最便宜会是两家。
    let manual: Vec<PriceRow> = sqlx::query_as(
        "SELECT e.id, p.model_id, e.base_url, m.label, e.label, \
                p.input_per_mtok, p.output_per_mtok, p.cached_per_mtok, \
                '', 1.0, e.probe_ms, e.probe_ok \
         FROM endpoint_model_price p \
         JOIN route_endpoints e ON e.id = p.endpoint_id \
         JOIN models m ON m.id = e.route_id \
         WHERE e.active \
         UNION ALL \
         SELECT m.id, p.model_id, m.base_url, m.label, '', \
                p.input_per_mtok, p.output_per_mtok, p.cached_per_mtok, \
                '', 1.0, NULL::int, NULL::boolean \
         FROM endpoint_model_price p \
         JOIN models m ON m.id = p.endpoint_id \
         WHERE m.active",
    )
    .fetch_all(&state.db)
    .await?;

    // 真实 token 配比。**这不是我拍的权重，是这个模型自己最近 30 天跑出来的。**
    // 一个混合价必须说得出它按什么配比混的，否则它就是个没有单位的数。
    let mix: Vec<(String, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT model_id, SUM(prompt_tokens)::bigint, SUM(completion_tokens)::bigint, \
                SUM(cached_tokens)::bigint, SUM(calls)::bigint \
         FROM endpoint_model_usage WHERE day >= current_date - 30 GROUP BY model_id",
    )
    .fetch_all(&state.db)
    .await?;
    let mix: HashMap<String, (i64, i64, i64, i64)> = mix
        .into_iter()
        .map(|(m, p, c, ca, n)| (m, (p, c, ca, n)))
        .collect();

    let open: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT unnest(enabled_models) FROM models WHERE active",
    )
    .fetch_all(&state.db)
    .await?;
    let open: std::collections::HashSet<String> = open.into_iter().collect();

    // 按 (出口, 模型) 归拢，自动价盖住手录价。
    let mut by_key: HashMap<(uuid::Uuid, String), (PriceRow, &'static str)> = HashMap::new();
    for row in manual {
        by_key.insert((row.0, row.1.clone()), (row, "manual"));
    }
    for row in auto {
        by_key.insert((row.0, row.1.clone()), (row, "auto"));
    }

    let mut by_model: HashMap<String, Vec<Offer>> = HashMap::new();
    for ((_, model_id), (row, source)) in by_key {
        let (
            eid,
            _,
            base_url,
            route_label,
            endpoint_label,
            input_raw,
            output_raw,
            cached_raw,
            group_name,
            group_multiplier,
            probe_ms,
            probe_ok,
        ) = row;
        let host = host_of(&base_url);
        let rate = rates.get(&host).copied();
        let cny = |v: f64| rate.and_then(|r| cny_per_official_usd(v, r));
        // 缓存价没录就按输入价算 —— 和对账那边同一个方向：**保守**，宁可高估成本。
        let cached_eff = cached_raw.unwrap_or(input_raw);
        let via = if endpoint_label.is_empty() {
            route_label.clone()
        } else {
            format!("{route_label} · {endpoint_label}")
        };
        // 去重键：同一家站、同一个分组、同一份价 —— 这三样一样就是同一个选择。
        // 价进键里是因为同一家站的不同分组价钱不同，而分组名有时是空的。
        let key = format!("{host}|{group_name}|{input_raw:.6}|{output_raw:.6}");
        by_model.entry(model_id).or_default().push(Offer {
            key,
            host,
            via: vec![via],
            endpoint_id: eid,
            input_raw,
            output_raw,
            group_name,
            group_multiplier,
            source,
            input_cny: cny(input_raw),
            output_cny: cny(output_raw),
            // 先占位，配比拿到之后再算。
            blended_cny: cny(cached_eff),
            rank: None,
            probe_ms,
            probe_ok,
            fastest: false,
            slow: false,
        });
    }

    let mut rows: Vec<ModelRow> = Vec::with_capacity(by_model.len());
    for (model_id, mut offers) in by_model {
        // 配比：这个模型最近 30 天真实跑出来的。没有用量就退回「只按输入价排」，
        // 并且**把这件事说出来** —— 编一个默认配比会让排名看起来有依据而其实没有。
        let (mix_source, w_in, w_cache, w_out, calls) = match mix
            .get(&model_id)
            .and_then(|(p, c, ca, n)| mix_weights(*p, *c, *ca).map(|w| (w, *n)))
        {
            Some(((wi, wc, wo), n)) => ("usage", wi, wc, wo, n),
            // 没有真实用量就**说出来**，按输入价排。编一个默认配比会让排名
            // 看起来有依据而其实没有。
            None => ("input_only", 1.0, 0.0, 0.0, 0),
        };

        for o in offers.iter_mut() {
            // blended_cny 上面暂存的是「缓存价的人民币」，这里重算成真正的混合价。
            let rate = rates.get(&o.host).copied();
            o.blended_cny = match (rate, o.input_cny, o.output_cny) {
                (Some(_), Some(i), Some(out)) => {
                    let cached_cny = o.blended_cny.unwrap_or(i);
                    Some(w_in * i + w_cache * cached_cny + w_out * out)
                }
                _ => None,
            };
        }

        // 合并同键。**同一家站、同一个分组、同一份价，被三条线路各挂一次不是三个选择。**
        // 不合并的话，一个模型会排出一低/二低/三低而三行数字一模一样 —— 那不是排名，
        // 是噪音，还会让人以为「换一个能省钱」。
        let mut merged: Vec<Offer> = Vec::new();
        for o in offers {
            match merged.iter_mut().find(|m| m.key == o.key) {
                Some(m) => {
                    for v in o.via {
                        if !m.via.contains(&v) {
                            m.via.push(v);
                        }
                    }
                    // 速度取这一组里**最快**的那条：同一家站同一份价，走哪条线路
                    // 到达它是我们自己的事，用户体验按最好的那条算。
                    m.probe_ms = match (m.probe_ms, o.probe_ms) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                    // 有一条测通就算通：探测是逐出口发的，同一家站的两条只是凭据不同。
                    if o.probe_ok == Some(true) {
                        m.probe_ok = Some(true);
                    }
                }
                None => merged.push(o),
            }
        }
        let mut offers = merged;
        for o in offers.iter_mut() {
            o.via.sort();
        }

        // 名次：**并列同名次**。价钱一样就是一样，硬分先后是编出来的信息。
        // 只有算得出人民币的才排；算不出的没有名次，而不是排到最后假装有。
        let mut vals: Vec<f64> = offers.iter().filter_map(|o| o.blended_cny).collect();
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // 按「六位小数相同就算同一个价」去重 —— 浮点尾巴不该制造出一个假的名次差。
        vals.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for o in offers.iter_mut() {
            o.rank = o.blended_cny.map(|v| {
                vals.iter()
                    .position(|x| (v - *x).abs() < 1e-6)
                    .unwrap_or(vals.len())
                    + 1
            });
        }
        // 省多少：拿**第一低和第二低**比，也就是去重之后真正不同的两个价。
        // 全都一样价时没有 gap，那时候「省 0%」比不显示更误导。
        let gap_pct = match (vals.first(), vals.get(1)) {
            (Some(x), Some(y)) if *y > 0.0 => Some((y - x) / y * 100.0),
            _ => None,
        };

        // 快慢。判据和选路那边同一个函数 —— 两处各写一份必然分叉，而这一屏
        // 说的「流畅」就会和真正被优先敲门的那个对不上。
        let best_ms = offers
            .iter()
            .filter(|o| o.probe_ok != Some(false))
            .filter_map(|o| o.probe_ms.map(|v| v as f64))
            .filter(|v| *v > 0.0)
            .fold(None::<f64>, |acc, v| Some(acc.map_or(v, |a: f64| a.min(v))));
        for o in offers.iter_mut() {
            o.fastest = best_ms.is_some_and(|b| o.probe_ms.map(|v| v as f64) == Some(b));
            o.slow = crate::route_endpoints::is_egregiously_slow(o.probe_ms, best_ms);
        }

        offers.sort_by(|a, b| match (a.rank, b.rank) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.host.cmp(&b.host),
        });

        rows.push(ModelRow {
            open: open.contains(&model_id),
            model_id,
            mix_source,
            mix_in: w_in,
            mix_cached: w_cache,
            mix_out: w_out,
            mix_calls: calls,
            offers,
            gap_pct,
        });
    }

    // 已开放的排前面（那是正在花钱的），其余按名字。
    rows.sort_by(|a, b| b.open.cmp(&a.open).then_with(|| a.model_id.cmp(&b.model_id)));

    // 「有价」和「比得了」是两件事，必须分开数。
    //
    // 线上 478 个模型有真实单价，但其中只有 17 个在**两家以上**有价 —— 其余 461 个
    // 只有一家，说它「最低」什么都没说。把这两个数合成一个，界面就会写出
    // 「478 个模型算得出最便宜」这种听起来很强、实际是废话的句子。
    let priced = rows
        .iter()
        .filter(|r| r.offers.iter().any(|o| o.blended_cny.is_some()))
        .count();
    let comparable = rows.iter().filter(|r| r.offers.len() > 1).count();
    Ok(Json(json!({
        "rows": rows,
        "models": rows.len(),
        "priced": priced,
        "comparable": comparable,
        "open_models": rows.iter().filter(|r| r.open).count(),
    })))
}

#[cfg(test)]
mod ratio_sync_tests {
    /// 同步倍率**不许替用户做主**，而且写入判据要和手填那条路一致。
    ///
    /// # 为什么这条比它看起来重要
    ///
    /// 「真实倍率」的两个来源 2026-08-26 实测都不能无条件相信：
    ///
    ///   · 公开价目看不见**私有/自建分组**。线上 Claude 的 key 在 `CCMAX（自建）1x`，
    ///     而公开价目里那几个模型只出现在 `claude_kiro 0.07x` —— 照抄就是把成本
    ///     算成十四分之一，**正好是「让你以为在赚钱」的方向**。
    ///   · 余额反推要求 token 记录完整，而按模型记账当天 05:48 才修好。
    ///
    /// 所以：有歧义时取**最大**（宁可高估成本），可信度必须回给界面，
    /// 而且**只有 `ok` 那一档前端才默认勾上**。
    ///
    /// **少填一个倍率最坏是排序不准；填错一个是账目错到反向。**
    #[test]
    fn ratio_sync_never_decides_for_the_user() {
        let src = include_str!("relay_rates.rs");
        // **按行首锚定。** 这个测试模块排在被测函数**前面**（函数是后追加到文件末尾的），
        // 所以裸 find 会先命中测试自己写的这句字面量，切片从测试内部开始，
        // 下面每条断言都在自己身上命中 —— 恒真。今天这个坑踩到第四次了。
        let at = src
            .find("\npub async fn admin_ratio_preview(")
            .expect("同步倍率的预览接口不见了");
        let rest = &src[at..];
        let end = rest[1..]
            .find("\n#[derive")
            .map(|i| i + 1)
            .unwrap_or(rest.len());
        let body = &rest[..end];

        assert!(
            !body.contains("fn ratio_sync_never_decides_for_the_user"),
            "切片切到测试自己身上了 —— 下面的断言会恒真，等于什么都没测",
        );

        // 四档可信度都要有，缺一档就会有一类情况被悄悄当成另一类。
        for c in ["\"ok\"", "\"partial\"", "\"ambiguous\"", "\"none\""] {
            assert!(body.contains(c), "少了 {c} 这一档可信度");
        }
        // 有歧义取最大 —— 高估成本让毛利难看，低估会让亏损看起来像盈利。
        assert!(
            body.contains("Some(a.map_or(v, |a: f64| a.max(v)))"),
            "有歧义时不再取最大值 —— 取小的那个会让亏损看起来像盈利",
        );
        // 前端只默认勾 ok。
        let ui = include_str!("../admin-ui/src/components/RatioSync.tsx");
        assert!(
            ui.contains("x.confidence === \"ok\""),
            "界面默认勾选的不再只是「可信」那一档 —— 等于替用户做了一个可能反向的决定",
        );
        assert!(
            ui.contains("私有 / 自建分组中转不公开"),
            "界面没有说明「看不到」是怎么回事 —— 用户会以为那几条不用管",
        );

        // 写入判据必须和手填那条路一致（0 < x < 10）。两处各写一套，
        // 这条路就会放进手填路挡下来的值。
        let ap = src
            .find("\npub async fn admin_ratio_apply(")
            .expect("同步倍率的写入接口不见了");
        let apbody = &src[ap..];
        assert!(
            apbody.contains("it.ratio <= 0.0 || it.ratio >= 10.0"),
            "写入没有沿用手填那条路的合法区间",
        );
        assert!(
            apbody.contains("r.rows_affected() != 1"),
            "没检查每一条真的写进去了 —— 少写一条就是一条线路的倍率没同步，而界面会说成功",
        );
    }
}

#[cfg(test)]
mod price_scan_tests {
    /// 逐模型价必须**两套命名空间都捞**。
    ///
    /// # 这条守的是一次真实的静默丢数
    ///
    /// `endpoint_auto_price.endpoint_id` 有两种含义：对出口是 `route_endpoints.id`，
    /// 对线路自带地址是 `models.id`（和 `endpoint_usage` / `health_id` 同一套约定）。
    /// 上一版只 JOIN 了 `route_endpoints` —— 线上 682 条真实价格**只捞到 106 条**，
    /// 576 条（85%）被丢掉，而丢掉的恰恰是最大的两份目录：openrouter 412 个模型、
    /// hanhegufei 164 个。
    ///
    /// 它不报错。表现是界面上「翻来覆去只有那两家」，看起来像数据本来就少 ——
    /// 这类 bug 只有拿真实计数去对才发现得了。
    #[test]
    fn per_model_prices_cover_both_id_namespaces() {
        let src = include_str!("relay_rates.rs");
        // 直接锁定那个函数，不做「切到第一个测试模块为止」那种切法。
        //
        // 这个文件里测试模块夹在生产代码**中间**（`mod tests` 在第 386 行，而
        // `admin_model_prices` 在 526 行），所以任何「切到第一个 #[cfg(test)]」的写法
        // 都会把要查的 SQL 切没，于是测试红在一个根本不存在的问题上 —— 前两版都这么红过。
        // 锁定函数同时也解决自我印证：断言的字面量写在测试模块里，不在这个区间内。
        let at = src
            .find("pub async fn admin_model_prices(")
            .expect("逐模型比价的接口不见了");
        let rest = &src[at..];
        let end = rest.find("\n#[cfg(test)]").unwrap_or(rest.len());
        let prod = &rest[..end];
        // 出口那一半。
        assert!(
            prod.contains("JOIN route_endpoints e ON e.id = p.endpoint_id"),
            "出口那一半的价没了",
        );
        // 线路自带地址那一半 —— 这是被丢掉过的那半。
        assert!(
            prod.contains("JOIN models m ON m.id = p.endpoint_id"),
            "线路自带地址的逐模型价又被丢掉了 —— 线上那是 85% 的数据，\
             而且不会报错，只表现为「只有那两家」",
        );
        // 两张价表（抓来的 + 手录的）都要覆盖两套命名空间，各两条 JOIN。
        assert_eq!(
            prod.matches("JOIN models m ON m.id = p.endpoint_id").count(),
            2,
            "抓来的价和手录的价，只有一张表捞了自带地址 —— 另一张会缺一大块",
        );
        assert_eq!(
            prod.matches("UNION ALL").count(),
            2,
            "UNION 少了一处：某一张价表退回了只看出口",
        );
    }
}

#[cfg(test)]
mod mix_tests {
    use super::*;

    /// 配比必须来自真实用量，而且缓存要从普通输入里减出来。
    #[test]
    fn the_blend_weights_come_from_real_tokens() {
        // 线上 grok-4.6 的真实形状（30 天）：输入 5,060,042、输出 89,620、缓存 3,808,192。
        // 缓存占了四分之三 —— 随手拍的「输入 3 输出 1」在这个模型上离谱得没边。
        let (wi, wc, wo) = mix_weights(5_060_042, 89_620, 3_808_192).unwrap();
        assert!((wi + wc + wo - 1.0).abs() < 1e-9, "三个权重必须加起来是 1");
        assert!((wi - 0.2431).abs() < 0.001, "普通输入 ~24%，实得 {wi}");
        assert!((wc - 0.7395).abs() < 0.001, "缓存 ~74%，实得 {wc}");
        assert!((wo - 0.0174).abs() < 0.001, "输出 ~1.7%，实得 {wo}");

        // 没有用量 → None。调用方据此说「按输入价排」，而不是编一个配比。
        assert_eq!(mix_weights(0, 0, 0), None);
        assert_eq!(mix_weights(-5, -5, 0), None);
    }

    /// 缓存数比输入还大时不许算出负权重。
    ///
    /// cached **包含在** prompt 里。上游多报一点（或者我们某次没减干净）就会让
    /// 普通输入权重变成负数，混合价跟着变负 —— 而负数会让这家站稳稳排「一低」。
    /// 一个因为脏数据而永远排第一的推荐，比没有推荐糟得多。
    #[test]
    fn a_cached_count_larger_than_prompt_never_makes_a_negative_price() {
        let (wi, wc, wo) = mix_weights(1_000, 100, 9_999).unwrap();
        assert!(wi >= 0.0 && wc >= 0.0 && wo >= 0.0, "权重出现负数：{wi} {wc} {wo}");
        assert!((wi + wc + wo - 1.0).abs() < 1e-9);
        assert_eq!(wi, 0.0, "缓存吃满时普通输入应当是 0，不是负数");

        // 拿这组权重去混三个正价，结果必须仍然是正的。
        let blended = wi * 3.0 + wc * 0.3 + wo * 15.0;
        assert!(blended > 0.0, "混合价变成了 {blended} —— 负价会让这家永远排第一低");
    }
}

// ---------- 倍率同步 ----------
//
// # 为什么这不是一个「一键自动写」的按钮
//
// 「真实倍率」有两个来源，2026-08-26 实测两个都不能无条件相信：
//
//   1. **公开价目里的分组倍率**。中转把每个分组的倍率写在价目表里，我们抓得到。
//      但**私有/自建分组不在公开价目里** —— 线上 Claude 那条的 key 在
//      `CCMAX（自建）1x`，而公开价目里那几个模型只出现在 `claude_kiro 0.07x`。
//      照抄就是把成本算成十四分之一，**正好是「让你以为在赚钱」的方向**。
//
//   2. **余额反推**（余额掉的钱 ÷ 按官方价算的成本）。这是执行事实，最硬。
//      但它要求 token 记录是完整的，而按模型记账 2026-08-26 05:48 才修好 ——
//      在那之前分子完整、分母残缺，实测反推出 1.58 倍，比任何一个分组都高。
//
// 所以这里回的是**对照表加证据**，不是一个替用户做主的数字。可信的那些前端默认勾上，
// 看不见分组的明说「你的 key 在私有分组，价目里看不到」。
//
// 少填一个倍率，最坏是排序不准；**填错一个倍率，是账目错到反向**。

#[derive(Debug, serde::Serialize)]
pub struct RatioRow {
    pub endpoint_id: uuid::Uuid,
    pub route_label: String,
    pub outlet_label: String,
    pub host: String,
    pub is_own: bool,
    pub current: f64,
    /// 从公开价目推出来的倍率。None = 这条线路开放的模型一个都不在价目里。
    pub from_catalog: Option<f64>,
    /// 推它的时候命中了哪些分组。多于一个 = 有歧义。
    pub groups: Vec<String>,
    pub matched_models: i64,
    pub total_models: i64,
    /// 可信度：`ok`（单一分组、全部命中）/ `partial`（只命中一部分）/
    /// `ambiguous`（多个分组）/ `none`（价目里查不到）。
    pub confidence: &'static str,
    pub reason: String,
}

/// GET /api/admin/ratio-sync
pub async fn admin_ratio_preview(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    // (出口 id, 线路名, 出口名, 地址, 是不是自带地址, 当前倍率, 这个出口开放的模型)
    type Target = (uuid::Uuid, String, String, String, bool, f64, Vec<String>);
    let owns: Vec<(uuid::Uuid, String, String, Vec<String>)> = sqlx::query_as(
        "SELECT id, label, base_url, enabled_models FROM models WHERE active AND base_url <> ''",
    )
    .fetch_all(&state.db)
    .await?;
    let outlets: Vec<(uuid::Uuid, String, String, String, f64, Vec<String>, Vec<String>)> =
        sqlx::query_as(
            "SELECT e.id, m.label, e.label, e.base_url, e.cost_ratio, e.enabled_models, \
                    m.enabled_models \
             FROM route_endpoints e JOIN models m ON m.id = e.route_id \
             WHERE e.active AND e.base_url <> ''",
        )
        .fetch_all(&state.db)
        .await?;

    let mut targets: Vec<Target> = Vec::new();
    for (id, label, url, models) in owns {
        targets.push((id, label, "自带地址".into(), url, true, 1.0, models));
    }
    for (id, route, olabel, url, ratio, own_models, route_models) in outlets {
        // 出口没勾模型 = 承载线路开放的那些。判据和 expand 一致。
        let models = if own_models.is_empty() { route_models } else { own_models };
        let olabel = if olabel.trim().is_empty() { "未命名出口".into() } else { olabel };
        targets.push((id, route, olabel, url, false, ratio, models));
    }

    // 价目：(出口, 模型) → (分组名, 分组倍率)
    let prices: Vec<(uuid::Uuid, String, String, f64)> = sqlx::query_as(
        "SELECT endpoint_id, model_id, group_name, group_multiplier FROM endpoint_auto_price",
    )
    .fetch_all(&state.db)
    .await?;
    let mut by_ep: HashMap<uuid::Uuid, HashMap<String, (String, f64)>> = HashMap::new();
    for (e, m, g, mult) in prices {
        by_ep.entry(e).or_default().insert(m, (g, mult));
    }

    let mut rows: Vec<RatioRow> = Vec::new();
    for (id, route_label, outlet_label, url, is_own, current, models) in targets {
        let cat = by_ep.get(&id);
        let mut groups: Vec<String> = Vec::new();
        let mut mults: Vec<f64> = Vec::new();
        for m in &models {
            if let Some((g, mult)) = cat.and_then(|c| c.get(m)) {
                if !groups.contains(g) {
                    groups.push(g.clone());
                }
                mults.push(*mult);
            }
        }
        groups.sort();
        let matched = mults.len() as i64;
        let total = models.len() as i64;
        // 有歧义时取**最大**的那个：高估成本让毛利难看，低估会让亏损看起来像盈利。
        // 这条方向和这个仓库里其它几处「宁可保守」是同一条。
        let from_catalog = mults
            .iter()
            .copied()
            .fold(None::<f64>, |a, v| Some(a.map_or(v, |a: f64| a.max(v))));
        let (confidence, reason) = if matched == 0 {
            (
                "none",
                "这条线路开放的模型一个都不在公开价目里 —— 多半是你的 key 在私有/自建分组，\
                 那种分组中转不公开。只能照着中转后台手填。"
                    .to_string(),
            )
        } else if groups.len() > 1 {
            (
                "ambiguous",
                format!("命中 {} 个分组（{}），取了最贵的那个 —— 少收比多收危险，\
                         但请对照中转后台确认你的 key 在哪个分组。", groups.len(), groups.join("、")),
            )
        } else if matched < total {
            (
                "partial",
                format!("{total} 个模型里只有 {matched} 个在价目里（分组 {}）。\
                         没命中的那些多半在私有分组，确认一下。", groups.join("、")),
            )
        } else {
            (
                "ok",
                format!("这条线路开放的 {total} 个模型全部落在分组「{}」里。", groups.join("、")),
            )
        };
        rows.push(RatioRow {
            endpoint_id: id,
            route_label,
            outlet_label,
            host: host_of(&url),
            is_own,
            current,
            from_catalog,
            groups,
            matched_models: matched,
            total_models: total,
            confidence,
            reason,
        });
    }

    rows.sort_by(|a, b| {
        a.route_label
            .cmp(&b.route_label)
            .then_with(|| b.is_own.cmp(&a.is_own))
            .then_with(|| a.outlet_label.cmp(&b.outlet_label))
    });
    // 自带地址没有 cost_ratio 这一列，改不了 —— 前端据此禁掉它们的勾选框。
    let changeable = rows
        .iter()
        .filter(|r| !r.is_own && r.from_catalog.is_some_and(|v| (v - r.current).abs() > 1e-9))
        .count();
    Ok(Json(json!({ "rows": rows, "changeable": changeable })))
}

#[derive(Debug, Deserialize)]
pub struct RatioApplyReq {
    pub items: Vec<RatioApplyItem>,
}

#[derive(Debug, Deserialize)]
pub struct RatioApplyItem {
    pub endpoint_id: uuid::Uuid,
    pub ratio: f64,
}

/// POST /api/admin/ratio-sync —— 把用户勾中的那几条写进去。
///
/// **只改出口**（`route_endpoints.cost_ratio`）。线路自带地址没有这一列，它恒等于 1.0 ——
/// 那是「我按官方价进货」的定义，不是一个可调的旋钮。
///
/// 一个事务，全写或全不写：写一半的话次序和成本都会是两次意图的混合体。
pub async fn admin_ratio_apply(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<RatioApplyReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    if req.items.is_empty() {
        return Err(AppError::bad("没有勾选任何一条"));
    }
    if req.items.len() > 500 {
        return Err(AppError::bad("一次改不了这么多"));
    }
    for it in &req.items {
        // 判据和手填那条路**同一套**：大于 0、小于 10（点错小数点的护栏）。
        // 两处各写一套的话，这条路会放进手填路挡下来的值。
        if !it.ratio.is_finite() || it.ratio <= 0.0 || it.ratio >= 10.0 {
            return Err(AppError::bad("倍率要是个 0 到 10 之间的数"));
        }
    }
    let mut tx = state.db.begin().await?;
    let mut n = 0u64;
    for it in &req.items {
        let r = sqlx::query("UPDATE route_endpoints SET cost_ratio = $2 WHERE id = $1")
            .bind(it.endpoint_id)
            .bind(it.ratio)
            .execute(&mut *tx)
            .await?;
        // 自带地址不在这张表里 —— 勾到它就是前端出了 bug，整批回滚而不是悄悄少写一条。
        if r.rows_affected() != 1 {
            return Err(AppError::bad(
                "有一条不是出口（线路自带地址的倍率恒为 1，改不了），整批没保存",
            ));
        }
        n += 1;
    }
    tx.commit().await?;
    tracing::info!(changed = n, "倍率已按价目同步");
    Ok(Json(json!({ "ok": true, "changed": n })))
}
