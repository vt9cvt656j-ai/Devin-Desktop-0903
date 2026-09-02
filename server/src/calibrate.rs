//! 用**余额差**反推一个出口的真实进价。
//!
//! # 为什么需要它
//!
//! 中转的价目只有三种来源，前两种经常拿不到：
//!
//!   1. 面板的价目接口 —— 站长可以关，实测线上六家里五家关了或被人机校验挡住；
//!   2. `/v1/models` 里的 pricing 字段 —— 实测那六家一个都没带（teamorouter 回了
//!      39 个模型、ohub 34 个、wecodex 9 个，全都只有 id 没有价）；
//!   3. 手工录 —— 要人去抄。
//!
//! 但还有第四种，而且它**不依赖对方公布任何东西**：花掉一点钱，看余额掉了多少。
//! 这条路只需要两件事 —— 能发请求、能查余额 —— 而这两件本来就是这个出口在用的能力。
//!
//! # 它比公布的价目更准
//!
//! 量到的是**实际扣的钱**，天然含了分组倍率、活动折扣、按次计费、隐藏加价这些
//! 在价目表上看不见的东西。价目表说多少不重要，账上掉多少才是成本。
//!
//! # 判据与拒绝
//!
//! 两次标定解一个二元一次方程组：
//!
//! ```text
//!   Δ1 = 输入token1 × 输入价 + 输出token1 × 输出价
//!   Δ2 = 输入token2 × 输入价 + 输出token2 × 输出价
//! ```
//!
//! token 数一律取**回执里上游自己报的**，不是我们请求时想要的 —— 两者经常不同
//! （系统提示、模板、思考 token）。
//!
//! 下面每一条不满足就**明确拒绝并说明**，绝不给一个「大概是这个数」：
//!
//!   · 余额读不到 → 这条路本来就走不通；
//!   · 余额变化小于精度门槛 → 量不出来（余额只有四位小数，掉 0.0001 和噪声分不开）；
//!   · 行列式接近 0 → 两次标定的输入输出配比太像，方程组解不稳；
//!   · 解出负价 → 一定不对。最常见的原因是**期间有真实用户流量也在花同一个账户**，
//!     那笔钱混进了 Δ 里。这种情况报出来让人重试，不是四舍五入成 0。

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use std::time::Duration;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// 标定请求的超时。比探测长：这里发的是**真实生成请求**，输出几百个 token 要时间。
const CALL_TIMEOUT: Duration = Duration::from_secs(120);

/// 余额至少要掉这么多才算量到了。
///
/// 线上余额是四位小数（`82.4588`）。掉 0.0001 和读数噪声分不开，掉 0.005 就稳了。
/// 量不到就明说量不到 —— 拿一个噪声级的差值去除以 token 数，会得到一个
/// 数量级随机的「价格」，而它看起来完全正常。
const MIN_DELTA: f64 = 0.005;

/// 两次标定的配比必须差得够开，否则方程组病态。
///
/// 归一化行列式低于这个值就拒绝：此时解对 Δ 的一点点噪声极度敏感，
/// 算出来的价可以差几个数量级。
const MIN_CONDITION: f64 = 0.25;

#[derive(Serialize)]
pub struct ModelCalibration {
    pub model: String,
    /// 量出来的每百万 token 美元价（这家中转自己的余额单位）。失败时为 None。
    pub input_per_mtok: Option<f64>,
    pub output_per_mtok: Option<f64>,
    /// 两次标定的原始数据，**一定回给界面**：价对不对，人得能自己复核这几个数。
    pub samples: Vec<CalSample>,
    /// 失败原因。成功时为空。
    pub why: String,
}

#[derive(Serialize)]
pub struct CalSample {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub balance_before: f64,
    pub balance_after: f64,
    pub delta: f64,
}

#[derive(serde::Deserialize)]
pub struct CalibrateReq {
    pub endpoint_id: uuid::Uuid,
    /// 要标定哪些模型。空 = 拒绝（不替用户决定花多少钱）。
    pub models: Vec<String>,
}

/// 解那个二元一次方程组。分出来当独立函数是为了能单测 —— 拒绝的判据全在这里。
///
/// 回 (输入价每token, 输出价每token) 或者拒绝的原因。
pub fn solve(a: &CalSample, b: &CalSample) -> Result<(f64, f64), String> {
    let (ia, oa, da) = (a.input_tokens as f64, a.output_tokens as f64, a.delta);
    let (ib, ob, db) = (b.input_tokens as f64, b.output_tokens as f64, b.delta);

    if da < MIN_DELTA || db < MIN_DELTA {
        return Err(format!(
            "余额变化太小（{da:.4} / {db:.4}），低于 {MIN_DELTA} 的精度门槛 —— 量不出来。\
             这家的余额只有四位小数，再小就和读数噪声分不开了"
        ));
    }
    let det = ia * ob - ib * oa;
    // 归一化：行列式要和两项乘积的量级比，否则大 token 数会让任何配比都「看起来」可解。
    let scale = (ia * ob).abs().max((ib * oa).abs()).max(1.0);
    if (det / scale).abs() < MIN_CONDITION {
        return Err(format!(
            "两次标定的输入/输出配比太接近（{ia:.0}/{oa:.0} 对 {ib:.0}/{ob:.0}），\
             方程组解不稳 —— 换更悬殊的配比再试"
        ));
    }
    let p_in = (da * ob - db * oa) / det;
    let p_out = (ia * db - ib * da) / det;
    if !p_in.is_finite() || !p_out.is_finite() || p_in < 0.0 || p_out < 0.0 {
        return Err(format!(
            "解出负价（输入 {:.9}、输出 {:.9}）—— 多半是标定期间有真实用户流量在花同一个账户，\
             那笔钱混进了余额差里。等空闲一点再标一次",
            p_in, p_out
        ));
    }
    Ok((p_in, p_out))
}

/// 每百万 token。库里那几列是这个单位。
pub fn per_mtok(v: f64) -> f64 {
    v * 1_000_000.0
}

/// 造一段**不会命中缓存**的提示词。
///
/// 前缀带随机数：中转和上游都会做提示词缓存，命中了的话这一发的价是缓存价，
/// 而我们要量的是新鲜输入价 —— 量错的方向还正好是「便宜得离谱」。
fn filler(nonce: u128, approx_tokens: usize) -> String {
    let mut s = format!("cal-{nonce:x} ");
    // 一个词大约一个 token，用递增数字避免任何形式的重复压缩。
    for i in 0..approx_tokens {
        s.push_str(&format!("{} ", (i as u64).wrapping_mul(2_654_435_761) % 100_000));
    }
    s
}

/// 发一发**真实**请求，回上游自己报的 (输入token, 输出token)。
///
/// token 数一定取回执里的，不取我们请求时想要的：系统提示、模板、思考 token
/// 都会让两者对不上，而方程组的系数就是这两个数 —— 用错就整条标定作废。
async fn call_once(
    base_url: &str,
    api_key: &str,
    protocol: &str,
    model: &str,
    prompt: &str,
    max_tokens: i64,
) -> Result<(i64, i64), String> {
    let http = reqwest::Client::builder()
        .timeout(CALL_TIMEOUT)
        .build()
        .map_err(|_| "建不出 HTTP 客户端".to_string())?;
    let wire = crate::models::Wire::of(protocol);
    let anthropic = wire == crate::models::Wire::Anthropic;
    let base = crate::models::api_base(base_url);
    let url = format!("{base}{}", wire.path());
    // Responses 的最小请求体是另一套名字。用 chat/completions 那套去发，上游要么
    // 400、要么当成别的东西处理 —— 两种都会让这一发的钱白花。
    let body = match wire {
        crate::models::Wire::XaiResponses => serde_json::json!({
            "model": model,
            "max_output_tokens": max_tokens,
            "input": [{ "role": "user", "content": prompt }],
        }),
        _ => serde_json::json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{ "role": "user", "content": prompt }],
        }),
    };
    let req = http.post(&url).json(&body);
    let req = if anthropic {
        req.header("x-api-key", api_key).header("anthropic-version", "2023-06-01")
    } else {
        req.header("authorization", format!("Bearer {api_key}"))
    };
    let resp = req.send().await.map_err(|e| {
        // 错误原文不能带出去：reqwest 的错误链会带完整 URL，查询串里可能有密钥。
        if e.is_timeout() { "超时".to_string() } else { "请求没发出去".to_string() }
    })?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(format!("上游返回 {status}"));
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "回的不是 JSON".to_string())?;
    let u = v.get("usage").ok_or("回执里没有 usage，量不出 token 数")?;
    let g = |a: &str, b: &str| {
        u.get(a)
            .and_then(|x| x.as_i64())
            .or_else(|| u.get(b).and_then(|x| x.as_i64()))
            .unwrap_or(0)
    };
    // Anthropic 的 input_tokens 不含缓存读，OpenAI 的 prompt_tokens 含 —— 这里两边
    // 都加上缓存读，让「输入 token」在两种形状下是同一个东西。填充串是随机的，
    // 正常情况下缓存读就是 0；不为 0 说明缓存意外命中了，下面会因此拒绝。
    let cache_read = u
        .get("cache_read_input_tokens")
        .and_then(|x| x.as_i64())
        .or_else(|| {
            u.get("prompt_tokens_details")
                .and_then(|d| d.get("cached_tokens"))
                .and_then(|x| x.as_i64())
        })
        .unwrap_or(0);
    let inp = g("prompt_tokens", "input_tokens");
    let outp = g("completion_tokens", "output_tokens");
    let inp = if u.get("cache_read_input_tokens").is_some() { inp + cache_read } else { inp };
    if inp <= 0 || outp <= 0 {
        return Err(format!("上游报的 token 数不可用（输入 {inp}、输出 {outp}）"));
    }
    if cache_read > 0 {
        return Err(format!(
            "这一发命中了提示词缓存（{cache_read} 个 token）—— 量到的会是缓存价不是输入价，作废重来"
        ));
    }
    Ok((inp, outp))
}

/// `POST /api/admin/endpoint-prices/calibrate`
///
/// **这个接口会花钱。** 每个模型发两发真实请求（一发几千输入、一发几百输出），
/// 所以只在用户点了按钮时跑，绝不进定时任务。花掉的钱就是量出来的那两个 Δ，
/// 一并回给界面 —— 花了多少必须看得见。
pub async fn admin_calibrate(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<CalibrateReq>,
) -> ApiResult<Json<serde_json::Value>> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    if req.models.is_empty() {
        return Err(AppError::bad("没有选要标定哪些模型 —— 这个操作会花钱，不替你决定标哪些"));
    }
    // 一次最多几个：每个模型两发真实请求，几十个模型排下去既慢又花钱。
    if req.models.len() > 12 {
        return Err(AppError::bad("一次最多标定 12 个模型 —— 每个都要发两发真实请求"));
    }

    let (base_url, api_key_enc, protocol, console_token) = load_endpoint(&state, req.endpoint_id)
        .await
        .ok_or_else(|| AppError::bad("找不到这个出口"))?;
    let api_key = crate::models::model_key(&api_key_enc);
    if api_key.trim().is_empty() {
        return Err(AppError::bad("这个出口没配调用密钥，发不了标定请求"));
    }

    let read_balance = || async {
        crate::relay_sync::balance_now(&state, req.endpoint_id, &base_url, &api_key, &console_token)
            .await
            .and_then(|b| b.used_usd.map(|u| (u, true)).or(b.remaining_usd.map(|r| (r, false))))
    };
    // 先确认余额读得到。读不到就整批不发 —— 发了也算不出价，纯粹白花钱。
    if read_balance().await.is_none() {
        return Err(AppError::bad(
            "这个出口的余额读不到，标定没法进行（标定靠的就是余额差）。\
             先把控制台令牌配上，或者确认这家有余额接口",
        ));
    }

    let mut out: Vec<ModelCalibration> = Vec::new();
    for model in &req.models {
        out.push(calibrate_model(&state, &req, &base_url, &api_key, &protocol, &console_token, model).await);
    }
    let ok = out.iter().filter(|x| x.input_per_mtok.is_some()).count();
    Ok(Json(serde_json::json!({ "calibrated": ok, "results": out })))
}

async fn calibrate_model(
    state: &AppState,
    req: &CalibrateReq,
    base_url: &str,
    api_key: &str,
    protocol: &str,
    console_token: &str,
    model: &str,
) -> ModelCalibration {
    let mut r = ModelCalibration {
        model: model.to_string(),
        input_per_mtok: None,
        output_per_mtok: None,
        samples: Vec::new(),
        why: String::new(),
    };
    // 两次的配比要悬殊：一次几乎全是输入，一次尽量多输出。配比相近的话
    // 行列式接近 0，解会被 Δ 上的一点噪声放大到没有意义。
    let plans = [(6000usize, 16i64), (40usize, 600i64)];
    for (i, (fill, max_tok)) in plans.iter().enumerate() {
        // 随机前缀防缓存。用请求 id 当种子，不用时间 —— 同一批里两发也不能撞。
        let nonce = uuid::Uuid::new_v4().as_u128();
        let before = match read_used(state, req.endpoint_id, base_url, api_key, console_token).await {
            Some(v) => v,
            None => {
                r.why = "标定途中余额读不到了".into();
                return r;
            }
        };
        let (inp, outp) =
            match call_once(base_url, api_key, protocol, model, &filler(nonce, *fill), *max_tok).await {
                Ok(v) => v,
                Err(e) => {
                    r.why = format!("第 {} 发标定失败：{e}", i + 1);
                    return r;
                }
            };
        let after = match read_used(state, req.endpoint_id, base_url, api_key, console_token).await {
            Some(v) => v,
            None => {
                r.why = "标定途中余额读不到了".into();
                return r;
            }
        };
        r.samples.push(CalSample {
            input_tokens: inp,
            output_tokens: outp,
            balance_before: before,
            balance_after: after,
            delta: (after - before).abs(),
        });
    }
    match solve(&r.samples[0], &r.samples[1]) {
        Ok((p_in, p_out)) => {
            r.input_per_mtok = Some(per_mtok(p_in));
            r.output_per_mtok = Some(per_mtok(p_out));
            if let Err(e) = save(state, req.endpoint_id, model, per_mtok(p_in), per_mtok(p_out)).await {
                r.why = format!("量出来了但没存进去：{e}");
            }
        }
        Err(e) => r.why = e,
    }
    r
}

/// 读一个「单调不减」的花费口径。
///
/// 优先「已用」：它只增不减，中途充值不会打断。只有余额的话取负数方向 ——
/// `balance_now` 已经把两者归一到 `used_usd` / `remaining_usd`，这里统一成
/// 「越大花得越多」，让上面的减法一个分支就够。
async fn read_used(
    state: &AppState,
    endpoint_id: uuid::Uuid,
    base_url: &str,
    api_key: &str,
    console_token: &str,
) -> Option<f64> {
    let b = crate::relay_sync::balance_now(state, endpoint_id, base_url, api_key, console_token).await?;
    b.used_usd.or_else(|| b.remaining_usd.map(|r| -r))
}

async fn load_endpoint(
    state: &AppState,
    id: uuid::Uuid,
) -> Option<(String, String, String, String)> {
    // 出口和线路自带地址在同一套 id 命名空间里，两张表都要找。
    if let Ok(row) = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT e.base_url, e.api_key, \
                CASE WHEN e.protocol = '' THEN m.protocol ELSE e.protocol END, \
                COALESCE(e.balance_token, '') \
         FROM route_endpoints e JOIN models m ON m.id = e.route_id WHERE e.id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    {
        return Some(row);
    }
    sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT base_url, api_key, protocol, '' FROM models WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .ok()
}

async fn save(
    state: &AppState,
    endpoint_id: uuid::Uuid,
    model: &str,
    input_per_mtok: f64,
    output_per_mtok: f64,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO endpoint_model_price \
           (endpoint_id, model_id, input_per_mtok, output_per_mtok, note) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (endpoint_id, model_id) DO UPDATE SET \
           input_per_mtok = EXCLUDED.input_per_mtok, \
           output_per_mtok = EXCLUDED.output_per_mtok, \
           note = EXCLUDED.note, updated_at = now()",
    )
    .bind(endpoint_id)
    .bind(model)
    .bind(input_per_mtok)
    .bind(output_per_mtok)
    // 来源写清楚：这个价是**量出来的**，不是抄来的也不是推算的。
    // 后面有人看见它和中转价目表对不上时，得知道该信哪个。
    .bind("标定：发两发真实请求，按余额差反推")
    .execute(&state.db)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(inp: i64, outp: i64, delta: f64) -> CalSample {
        CalSample { input_tokens: inp, output_tokens: outp, balance_before: 0.0, balance_after: delta, delta }
    }

    /// 解得出来的时候要解对。
    #[test]
    fn it_solves_the_two_by_two() {
        // 造一组：输入 5 元/百万、输出 25 元/百万（每 token 5e-6 / 25e-6）。
        let (pi, po) = (5e-6, 25e-6);
        let a = s(6000, 16, 6000.0 * pi + 16.0 * po);
        let b = s(40, 600, 40.0 * pi + 600.0 * po);
        let (gi, go) = solve(&a, &b).expect("这组应该解得出来");
        assert!((per_mtok(gi) - 5.0).abs() < 1e-6, "输入价解错了：{}", per_mtok(gi));
        assert!((per_mtok(go) - 25.0).abs() < 1e-6, "输出价解错了：{}", per_mtok(go));
    }

    /// 余额精度不够时**明确拒绝**，不给一个噪声级的「价格」。
    ///
    /// 线上余额只有四位小数。拿 0.0001 的差值去除以 token 数，会得到一个
    /// 数量级随机、却看起来完全正常的价格 —— 那比没有价目危险得多。
    #[test]
    fn a_delta_below_the_noise_floor_is_refused() {
        let a = s(6000, 16, 0.0001);
        let b = s(40, 600, 0.0002);
        let e = solve(&a, &b).unwrap_err();
        assert!(e.contains("余额变化太小"), "拒绝的理由不对：{e}");
        // 只有一发太小也要拒 —— 两个系数里有一个是噪声，解就整个是噪声。
        let big = s(6000, 16, 5.0);
        assert!(solve(&big, &s(40, 600, 0.0001)).is_err());
        assert!(solve(&s(6000, 16, 0.0001), &big).is_err());
    }

    /// 两次配比太像 → 方程组病态 → 拒绝。
    ///
    /// 不拒的话解会被 Δ 上的一点点噪声放大几个数量级，而结果照样是个正常模样的数。
    #[test]
    fn an_ill_conditioned_pair_is_refused() {
        // 两发几乎一样的配比。
        let a = s(1000, 100, 1.0);
        let b = s(1010, 101, 1.01);
        let e = solve(&a, &b).unwrap_err();
        assert!(e.contains("配比太接近"), "拒绝的理由不对：{e}");
    }

    /// 解出负价一定是量岔了 —— 报出来，不四舍五入成 0。
    ///
    /// 最常见的原因是标定期间有真实用户流量也在花同一个账户，那笔钱混进了 Δ 里。
    /// 悄悄取 0 的话，这个模型的成本会被记成零，而那正是「亏损显示成盈利」的方向。
    #[test]
    fn a_negative_price_is_reported_not_clamped() {
        // 第二发被别人的流量顶高了 Δ，解出来输入价为负。
        let a = s(6000, 16, 0.03);
        let b = s(40, 600, 9.9);
        let e = solve(&a, &b).unwrap_err();
        assert!(e.contains("负价"), "拒绝的理由不对：{e}");
        assert!(e.contains("真实用户流量"), "没说清最可能的原因：{e}");
    }

    /// 花钱的操作必须先问过人。
    ///
    /// 这不是客套：标定给每个模型发两发真实请求，一次点下去就是实打实的钱。
    /// 没有确认框的话，一个手滑就把十几个模型的钱花了 —— 而且看不出花了多少。
    #[test]
    fn the_console_asks_before_spending() {
        let ui = include_str!("../admin-ui/src/pages/Reconcile.tsx");
        // 钉整条守卫的形状，不只是「出现过 window.confirm」：在它前面加一个
        // `false &&` 就能让确认框永远不弹，而只匹配函数名的断言照样绿。
        assert!(
            ui.contains("    if (\n      !window.confirm(\n"),
            "标定按钮的确认框不是那条守卫本身 —— 一点就花钱",
        );
        // 而且用户点「取消」必须真的什么都不做。
        let at = ui.find("!window.confirm(").expect("确认框不见了");
        assert!(
            ui[at..].contains("      return;\n    }"),
            "点了取消还是会往下发请求",
        );
        // 确认框里要说清楚**会花钱**和**发几发**，不能只写「确定吗」。
        assert!(
            ui.contains("这些请求要花钱") && ui.contains("各发两发真实请求"),
            "确认框没说清会花钱、发几发",
        );
        // 花掉多少要回显。花了多少必须看得见。
        assert!(
            ui.contains("这两发花了 ${x.samples.map((v) => v.delta.toFixed(4)).join(\" + \")}"),
            "花掉的余额差没回显 —— 花了多少看不见",
        );
        // 只标真的跑过的模型：没跑过的标了也用不上，纯白花钱。
        assert!(
            ui.contains("const models = row.models.map((m) => m.model_id).slice(0, 12);"),
            "标定的模型范围不对 —— 会给没跑过的模型白花钱，或者超过服务端那道 12 个的闸",
        );
    }

    /// 标定**绝不能**进定时任务：它花的是真钱。
    #[test]
    fn calibration_is_never_scheduled() {
        // **先把测试模块剥掉。** 不剥的话这段断言会匹配到它自己下面那张禁用词表，
        // 于是它永远红 —— 而且看起来像发现了真问题。踩过一次。
        let all = include_str!("calibrate.rs");
        let me = &all[..all.find("\n#[cfg(test)]").unwrap_or(all.len())];
        for banned in ["tokio::spawn", "spawn_scheduler", "interval(", "tokio::time::sleep"] {
            assert!(
                !me.contains(banned),
                "标定里出现了 `{banned}` —— 这个操作发真实请求、花真钱，只能由人点按钮触发",
            );
        }
        // 路由挂上了，而且只有 POST。
        let main = include_str!("main.rs");
        assert!(main.contains("post(calibrate::admin_calibrate)"), "标定路由没挂上");
    }

    /// 命中缓存的那一发必须作废重来，不能拿来解方程。
    ///
    /// 缓存读通常是输入价的十分之一。混进去量到的价会**便宜十倍**，
    /// 而这个价一旦写进库，派单会以为这个出口极便宜、把流量全导过去 ——
    /// 错的方向正好是最贵的那个方向。
    ///
    /// `call_once` 是网络函数，单测碰不到，所以钉源码形状。
    #[test]
    fn a_cache_hit_invalidates_the_sample() {
        let all = include_str!("calibrate.rs");
        let me = &all[..all.find("\n#[cfg(test)]").unwrap_or(all.len())];
        assert!(
            me.contains("if cache_read > 0 {"),
            "命中缓存的标定没被作废 —— 量到的会是缓存价，便宜十倍",
        );
        // 而且要真的返回错误，不是只记一笔。
        let at = me.find("if cache_read > 0 {").unwrap();
        let seg = &me[at..];
        let end = seg.find("\n    }").unwrap_or(seg.len());
        assert!(
            seg[..end].contains("return Err("),
            "发现缓存命中却没有作废这一发",
        );
        // 两个形状的缓存字段都要认：Anthropic 单列 cache_read_input_tokens，
        // OpenAI 放在 prompt_tokens_details.cached_tokens 里。少认一个等于对那一族没设防。
        assert!(
            me.contains("\"cache_read_input_tokens\"")
                && me.contains("\"prompt_tokens_details\"")
                && me.contains("\"cached_tokens\""),
            "只认了一种形状的缓存字段 —— 另一族的缓存命中会漏过去",
        );
    }

    /// 填充串必须每次不同，否则第二发会命中提示词缓存。
    ///
    /// 命中缓存量到的是缓存价（通常是输入价的十分之一），而错的方向正好是
    /// 「便宜得离谱」—— 会让一个贵出口显示成极便宜，然后派单把流量全导过去。
    #[test]
    fn the_filler_is_never_the_same_twice() {
        let a = filler(1, 50);
        let b = filler(2, 50);
        assert_ne!(a, b, "两次填充串一样 —— 第二发会命中缓存");
        assert!(a.len() > 100 && b.len() > 100);
        // 长度大致跟着要的 token 数走。
        assert!(filler(1, 500).len() > filler(1, 50).len() * 5);
    }
}
