//! 中转站适配器：**认出这家跑的是什么软件，然后按它自己的接口把真实价目和余额拉下来。**
//!
//! # 为什么必须自动拉，而不是让运维手填单价
//!
//! 手填一个 `$/百万token` 在**形状**上就装不下真实计费。实测线上（2026-08-25）：
//!
//!   · sub2api 的一次扣费 = 单价 × 分组倍率 × (高峰时段 ? 高峰倍率 : 1)，
//!     而线上那家的分组倍率从 **0.07 到 1.0**，差 14 倍；
//!   · new-api 一族 = (输入 + 输出×补全倍率) × 模型倍率 × 分组倍率 ÷ 每单位额度，
//!     四个乘数，每一个都能被站长单独改。
//!
//! 手填等于把这些乘数在**填写那一刻**的乘积拍扁成一个标量。中转改一次倍率、或者把你
//! 的令牌挪一个分组，那个数就静默失效 —— 而对账这一页存在的全部意义就是发现这种事。
//!
//! # 认不出来时**必须**报「未知」
//!
//! 这个模块最容易造成的伤害不是「拉不到价」，是**拉到一个错的价还很自信**。
//! 一个猜出来的单价会让对账页显示一个精确的、错的毛利，而空白至少会让人去查。
//! 所以：指纹不确凿就是 `Unknown`，字段对不上就是 `None`，绝不用「差不多是这家」凑数。

use std::time::Duration;

/// 探测一个中转要多久放弃。
///
/// 8 秒：这些请求都是小 JSON，正常一秒内回。给到 8 秒是为了容忍跨境链路的抖动，
/// 再长会让「一家挂掉的中转」把整轮探测拖死 —— 而探测是定时跑的，拖死就等于不跑。
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

/// 中转软件家族。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Family {
    /// Wei-Shaw/sub2api。直接按美元计价，分组倍率是线性折扣。
    Sub2Api,
    /// QuantumNous/new-api（One API 分支）。额度制 + 四组倍率。
    NewApi,
    /// songquanpeng/one-api。额度制，但**没有**公开价目接口。
    OneApi,
    /// one-api 系的其它分支（Veloera / done-hub / VoAPI / shell-api …）。
    /// 带上认出来的牌子名 —— 它决定 `/api/user/self` 要哪个 user-id 头。
    OneApiFork(String),
    /// OpenRouter。价目公开，余额用调用密钥。
    OpenRouter,
    /// 自研网关（不是任何开源面板的分支）。带上认出来的牌子 —— 来自它自己在
    /// 401 文案里点名的密钥前缀（`sk-teamo-*` 这种），那是编译进去的常量。
    ///
    /// 这一类**没有**公开价目接口可言（每家自己写的），所以价目一律手工录。
    /// 但认出来仍然有意义：界面上「自研网关 / teamo」和「未知」是两句话 ——
    /// 前者是「这家就是这样，别再等它自动」，后者是「我没探明白，可能还有救」。
    Custom(String),
    /// 探过了，但对不上任何已知指纹。**不是「还没探」**，见 `Detection::note`。
    Unknown,
}

impl Family {
    pub fn label(&self) -> String {
        match self {
            Family::Sub2Api => "sub2api".into(),
            Family::NewApi => "new-api".into(),
            Family::OneApi => "one-api".into(),
            Family::OneApiFork(n) => format!("one-api 系 / {n}"),
            Family::OpenRouter => "openrouter".into(),
            Family::Custom(n) => format!("自研网关 / {n}"),
            Family::Unknown => "未知".into(),
        }
    }

    /// 从存库的家族名反解回来。
    ///
    /// 同步任务已经探过一次并把结果落了库，查余额时没必要再探一遍 —— 那是两个
    /// 多余的往返，而且**探测结果可能和适配器页显示的不一致**，于是同一条线路
    /// 在两个页面上是两个家族。认不出的名字回 Unknown，不猜。
    pub fn from_label(s: &str) -> Family {
        match s {
            "sub2api" => Family::Sub2Api,
            "new-api" => Family::NewApi,
            "one-api" => Family::OneApi,
            "openrouter" => Family::OpenRouter,
            _ if s.starts_with("one-api 系 / ") => {
                Family::OneApiFork(s.trim_start_matches("one-api 系 / ").to_string())
            }
            _ if s.starts_with("自研网关 / ") => {
                Family::Custom(s.trim_start_matches("自研网关 / ").to_string())
            }
            _ => Family::Unknown,
        }
    }

    /// 这一家有没有**专用**价目接口。
    ///
    /// `false` 不等于「拉不到」：`fetch_pricing` 对每一家都还会再走一条通用路
    /// （带调用密钥问 `/v1/models`，按 OpenRouter 约定读 pricing）。这个函数只
    /// 决定界面上那句话该怎么说 —— 「有接口但没拉到」和「本来就没有专用接口」
    /// 是两句不同的话，前者该去找站长开，后者该去手工录。
    pub fn can_fetch_pricing(&self) -> bool {
        matches!(self, Family::Sub2Api | Family::NewApi | Family::OpenRouter)
    }
}

/// 一次探测的结论。
#[derive(Clone, Debug, serde::Serialize)]
pub struct Detection {
    pub family: Family,
    /// 命中的是哪一条指纹 —— 出错时唯一能看出「它凭什么这么判」的地方。
    pub matched_by: String,
    /// 探不出来时的说明。`Unknown` 时**一定**非空。
    pub note: String,
    /// 额度换美元的除数（one-api 系才有）。**从 /api/status 现读，不写死。**
    ///
    /// 调研查到 metapi 那个项目把 Veloera 的除数写成了 1000000（正确是 500000），
    /// 全站成本差一倍而没人发现 —— 因为那个数字看起来完全正常。
    pub quota_per_unit: Option<f64>,
    pub detected_at: i64,
}

/// 一个模型在这家中转上的**真实单价**，已经归一成「美元每 token」。
///
/// 各家的原始表达完全不同（sub2api 直接给美元、new-api 给倍率+额度单位），
/// 归一放在适配器里做，下游只认这一种形状 —— 否则每加一家中转，对账那边就要
/// 再长一个分支，而那些分支没有一个是能测的。
#[derive(Clone, Debug, serde::Serialize)]
pub struct UnitPrices {
    pub input: f64,
    pub output: f64,
    /// 命中缓存的输入。None = 这家不单独计缓存价，按 input 算（保守方向）。
    pub cache_read: Option<f64>,
    /// 写入缓存。None 同上。
    pub cache_write: Option<f64>,
    /// 按次计费的单价（美元/次）。Some 时上面几个不参与计算。
    pub per_request: Option<f64>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct RelayPrice {
    pub model: String,
    pub prices: UnitPrices,
    /// 这个价属于哪个分组。None = 这家没有分组概念。
    pub group: Option<String>,
    /// 分组倍率。**已经乘进 prices 里了**，这里保留是为了让界面能解释
    /// 「为什么这家比官网便宜 14 倍」。
    pub group_multiplier: f64,
    /// 从哪个接口拉的 —— 价目不对时第一件事就是回去看这个接口。
    pub source: String,
}

/// 余额读数。数值**不做任何单位猜测**：各家单位不同，猜错的代价是一个
/// 差 50 万倍却看起来完全正常的数字。
#[derive(Clone, Debug, serde::Serialize)]
pub struct Balance {
    pub text: String,
    pub remaining_usd: Option<f64>,
    pub used_usd: Option<f64>,
    pub source: String,
}

fn client() -> Option<reqwest::Client> {
    reqwest::Client::builder().timeout(PROBE_TIMEOUT).build().ok()
}

/// 去掉尾部的 `/` 和 `/v1`，得到站点根。
///
/// 后台里线路地址两种写法都有（`https://x.com` 和 `https://x.com/v1`），
/// 而 `/api/...` 这些控制台接口挂在**根**上，不在 `/v1` 下面。不剥的话
/// 探测会全部打到 `/v1/api/status` 上，然后一路 404 —— 表现是「全都认不出来」。
fn site_root(base_url: &str) -> String {
    base_url.trim_end_matches('/').trim_end_matches("/v1").to_string()
}

/// 拿一个 URL 的 (状态码, 正文)。失败回 None —— 和「拿到了但内容不对」分开。
async fn get(http: &reqwest::Client, url: &str, auth: Option<&str>) -> Option<(u16, String)> {
    let mut req = http.get(url);
    if let Some(a) = auth {
        req = req.header("authorization", format!("Bearer {a}"));
    }
    let r = req.send().await.ok()?;
    let code = r.status().as_u16();
    let body = r.text().await.unwrap_or_default();
    Some((code, body))
}

/// 认出这家跑的是什么。**全程不需要真凭据。**
///
/// 探测顺序按「指纹独特性」排，不按「哪家常见」排：先打最不可能误判的那一发。
/// 常见度排序会让一个罕见但指纹独特的家族被前面某个宽松判据抢走。
pub async fn detect(base_url: &str) -> Detection {
    let now = chrono::Utc::now().timestamp();
    let unknown = |note: &str| Detection {
        family: Family::Unknown,
        matched_by: String::new(),
        note: note.to_string(),
        quota_per_unit: None,
        detected_at: now,
    };
    let Some(http) = client() else {
        return unknown("建不出 HTTP 客户端");
    };
    let root = site_root(base_url);
    if root.is_empty() {
        return unknown("线路没填地址");
    }

    // ── 1. sub2api ────────────────────────────────────────────────────────
    //
    // 判据是 /v1/usage 未鉴权时那句**逐字硬编码**的错误文案。它同时点名了三种
    // 接受的鉴权头，全网只有 sub2api 这么写。比 /api/v1/settings/public 的键集
    // 更硬：键集会随版本增删，这句话是编译进去的。
    // 这一份回应下面还要用（认自研网关靠的就是它 401 里点名的密钥前缀），所以留着。
    let usage = get(&http, &format!("{root}/v1/usage"), None).await;
    if let Some((code, body)) = usage.as_ref() {
        if *code == 401 && body.contains("\"API_KEY_REQUIRED\"") && body.contains("x-goog-api-key")
        {
            return Detection {
                family: Family::Sub2Api,
                matched_by: "/v1/usage 回 API_KEY_REQUIRED（sub2api 独有文案）".into(),
                note: String::new(),
                quota_per_unit: None,
                detected_at: now,
            };
        }
    }

    // ── 2. OpenRouter ─────────────────────────────────────────────────────
    if root.contains("openrouter.ai") {
        return Detection {
            family: Family::OpenRouter,
            matched_by: "域名是 openrouter.ai".into(),
            note: String::new(),
            quota_per_unit: None,
            detected_at: now,
        };
    }

    // ── 3. one-api 血统：/api/status 带 quota_per_unit ─────────────────────
    //
    // 这是整个 one-api 家族唯一稳定的免鉴权入口，one-api / new-api / Veloera /
    // done-hub / shell-api 全都有且字段名一致。拿到之后再按 data 的键集分家。
    let status = get(&http, &format!("{root}/api/status"), None).await;
    // SPA 兜底路由会让任何未知路径回 200 HTML。**200 不代表接口存在**，
    // 这一条是实测踩出来的（线上三家都这样），所以要真解析成 JSON 才算数。
    let sv = match &status {
        Some((200, b)) => serde_json::from_str::<serde_json::Value>(b).ok(),
        _ => None,
    };
    let data = sv
        .as_ref()
        .and_then(|v| v.get("data").cloned())
        .unwrap_or(serde_json::Value::Null);
    let qpu = data.get("quota_per_unit").and_then(|x| x.as_f64());

    // **不是 one-api 系 —— 但别在这里就放弃。**
    //
    // 上一版走到这里一律回「未知」，于是线上两家明明有很硬的指纹却被归到认不出：
    //   · llm.ohub.vip：/api/* 全部 403 `{"success":false,"message":"无效的请求，验证码错误"}`
    //     —— 那个信封就是 one-api 系的标准形状，只是面板接口被验证码挡住了；
    //   · api.teamorouter.com：自研网关，401 文案里点名 `sk-teamo-*`，还带 trace_id。
    //
    // 「认不出」和「认出来了但拉不到价目」是两句完全不同的话：前者让人以为还有救、
    // 会反复去点重探；后者直接告诉他去手工录，或者去把那道验证码关掉。
    if qpu.is_none() {
        if let Some(d) = gated_one_api(&status, now) {
            return d;
        }
        if let Some(d) = custom_gateway(usage.as_ref(), now) {
            return d;
        }
        if let Some(d) = openai_compatible(&http, &root, now).await {
            return d;
        }
        return unknown(
            "既不是 sub2api（/v1/usage 没回它那句独有文案），/api/status 也拿不到、\
             /v1/models 认不出牌子 —— 这家的价目要手工录",
        );
    }
    let has = |k: &str| data.get(k).is_some();

    // 分支牌子：/api/user/self 未鉴权时的报错会**点名它要哪个 user-id 头**，
    // 那个头名是编译期常量、站长改不了，是这一族里最硬的品牌指纹。
    if let Some((_, ubody)) = get(&http, &format!("{root}/api/user/self"), None).await {
        for (needle, brand) in [
            ("Veloera-User", "Veloera"),
            ("voapi-user", "VoAPI"),
            ("Rix-Api-User", "Rix-API"),
            ("neo-api-user", "neo-API"),
            ("X-Api-User", "v-api"),
        ] {
            if ubody.contains(needle) {
                return Detection {
                    family: Family::OneApiFork(brand.into()),
                    matched_by: format!("/api/user/self 报错点名了 {needle}"),
                    note: String::new(),
                    quota_per_unit: qpu,
                    detected_at: now,
                };
            }
        }
        // New-API-User 是 new-api 自己的头名，归到 NewApi 而不是分支。
        if ubody.contains("New-API-User") {
            return Detection {
                family: Family::NewApi,
                matched_by: "/api/user/self 报错点名了 New-API-User".into(),
                note: String::new(),
                quota_per_unit: qpu,
                detected_at: now,
            };
        }
    }

    // new-api：/api/pricing 存在（one-api 没这条路由）。
    if let Some((code, pbody)) = get(&http, &format!("{root}/api/pricing"), None).await {
        if code == 200 && pbody.contains("\"model_ratio\"") {
            return Detection {
                family: Family::NewApi,
                matched_by: "/api/pricing 返回带 model_ratio 的价目（one-api 无此路由）".into(),
                note: String::new(),
                quota_per_unit: qpu,
                detected_at: now,
            };
        }
        // 403 + 那句中文本身就是 new-api 的确证（倍率接口默认关闭）。
        if code == 403 && pbody.contains("倍率") {
            return Detection {
                family: Family::NewApi,
                matched_by: "/api/pricing 回 403 倍率相关文案".into(),
                note: "这站把价目接口关了，倍率拉不到 —— 要么请中转商打开，要么手工录".into(),
                quota_per_unit: qpu,
                detected_at: now,
            };
        }
    }

    // new-api 的 /api/status 独有键（价目接口被关掉时的兜底判据）。
    if has("quota_display_type") || has("custom_currency_symbol") || has("self_use_mode_enabled") {
        return Detection {
            family: Family::NewApi,
            matched_by: "/api/status 带 new-api 独有键".into(),
            note: "价目接口没探到，倍率可能要手工录".into(),
            quota_per_unit: qpu,
            detected_at: now,
        };
    }
    // one-api 独有键。
    if has("lark_client_id") || has("top_up_link") || has("oidc_well_known") {
        return Detection {
            family: Family::OneApi,
            matched_by: "/api/status 带 one-api 独有键".into(),
            // 这不是缺陷也不是没做完：one-api 上游**根本没有**公开价目接口。
            note: "one-api 没有公开的价目接口，这家的倍率只能手工录".into(),
            quota_per_unit: qpu,
            detected_at: now,
        };
    }

    Detection {
        family: Family::OneApiFork("未识别分支".into()),
        matched_by: "/api/status 有 quota_per_unit，但认不出具体是哪个分支".into(),
        note: "能确定是 one-api 系，但分不清牌子 —— 余额可能查得到，倍率多半要手工录".into(),
        quota_per_unit: qpu,
        detected_at: now,
    }
}

/// one-api 系，但**面板接口被挡住了**。
///
/// 线上 llm.ohub.vip 就是这个形状：`/api/status`、`/api/pricing`、`/api/user/self`
/// 一律 403 `{"success":false,"message":"无效的请求，验证码错误"}`。那个
/// `success + message` 信封是 one-api 全家的标准回包形状（前面几步已经把 sub2api、
/// OpenRouter、new-api 都排除掉了，走到这里还回这个信封的基本就是这一族）。
///
/// 判成「未知」是浪费信息：它其实是可识别的，只是**价目要人去把那道验证码关掉**
/// 或者手工录。把原话原样带出来，让人一眼看见拦住的是什么。
fn gated_one_api(status: &Option<(u16, String)>, now: i64) -> Option<Detection> {
    let (code, body) = status.as_ref()?;
    if *code == 200 {
        return None;
    }
    let v = serde_json::from_str::<serde_json::Value>(body).ok()?;
    if v.get("success")?.as_bool()? {
        return None;
    }
    let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("").trim();
    if msg.is_empty() {
        return None;
    }
    Some(Detection {
        family: Family::OneApiFork("面板被挡住".into()),
        matched_by: format!("/api/status 回 {code}，信封是 one-api 系的 success/message 形状"),
        note: format!(
            "面板接口被挡住了（原话：{msg}）—— 价目和余额都读不到。\
             多半是站点开了人机校验；关掉它、或者手工录价目",
        ),
        quota_per_unit: None,
        detected_at: now,
    })
}

/// 自研网关：靠它自己在 401 文案里点名的**密钥前缀**认牌子。
///
/// 线上 api.teamorouter.com：
/// `{"error":{"message":"Missing auth credential. Provide x-api-key: sk-teamo-* or ...",
///   "type":"missing_auth_credential"},"trace_id":"..."}`
///
/// `sk-<牌子>-` 是编译进那家网关的常量（密钥都得带这个前缀才认），站长改不了，
/// 和 one-api 系那几个 user-id 头一样硬。认出来不能自动拉价目 —— 自研的每家都不一样，
/// 没有共同接口可拉 —— 但能把「未知」换成一句确定的话。
fn custom_gateway(usage: Option<&(u16, String)>, now: i64) -> Option<Detection> {
    let (_, body) = usage?;
    // 得是 JSON 错误信封，不是随便一个含 sk- 的 HTML 页 —— 下面这串 `?` 就是判据：
    // 解析不了 JSON、或者没有 error.message，一律不认。
    let v = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let text = v.get("error")?.get("message")?.as_str()?;
    let brand = key_prefix_brand(text)?;
    Some(Detection {
        family: Family::Custom(brand.clone()),
        matched_by: format!("/v1/usage 的报错点名密钥前缀 sk-{brand}-（自研网关的编译期常量）"),
        note: "自研网关，没有面板价目接口".into(),
        quota_per_unit: None,
        detected_at: now,
    })
}

/// 从一句报错里抠出 `sk-<牌子>-` 的牌子部分。
///
/// 只认 `sk-x-` 这种**两段**的（`sk-teamo-*`）。单段的 `sk-xxxx` 是 OpenAI 通用形状，
/// 抠出来的会是别人的随机串，那比认不出更糟 —— 会给一个自信的错牌子。
fn key_prefix_brand(text: &str) -> Option<String> {
    let at = text.find("sk-")?;
    let rest = &text[at + 3..];
    let brand: String = rest
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect();
    // 后面必须紧跟第二个连字符，否则它就是普通的 sk-<随机串>，认不得牌子。
    if brand.is_empty() || brand.len() > 24 || !rest[brand.len()..].starts_with('-') {
        return None;
    }
    Some(brand)
}

/// 最后一档：至少确认它是个 OpenAI 兼容网关，牌子取模型的 `owned_by`。
///
/// `/v1/models` 是这一行唯一的通用入口。拿到就说明「能对话、但面板认不出」，
/// 比「未知」多一句确定的话；拿不到才是真的什么都不知道。
async fn openai_compatible(http: &reqwest::Client, root: &str, now: i64) -> Option<Detection> {
    let (code, body) = get(http, &format!("{root}/v1/models"), None).await?;
    if code != 200 {
        return None;
    }
    let v = serde_json::from_str::<serde_json::Value>(&body).ok()?;
    if v.get("object")?.as_str()? != "list" {
        return None;
    }
    let brand = v
        .get("data")?
        .as_array()?
        .first()?
        .get("owned_by")
        .and_then(|x| x.as_str())
        .filter(|x| !x.is_empty() && x.len() <= 24)
        .unwrap_or("未标牌子")
        .to_string();
    Some(Detection {
        family: Family::Custom(brand),
        matched_by: "/v1/models 回 OpenAI 形状的模型表，牌子取 owned_by".into(),
        note: "只认出是个 OpenAI 兼容网关，面板接口认不出 —— 价目要手工录".into(),
        quota_per_unit: None,
        detected_at: now,
    })
}

// ---------------------------------------------------------------- 价目

/// 按家族把真实价目拉下来，归一成「美元每 token」。
///
/// 拉不到就回空 Vec，**不回猜的值**。调用方据此退回手填，界面上会写明原因。
/// 回 (价目, 通用退路没拿到价的原因)。
///
/// 那句原因不是装饰：「密钥被拒」「回了 40 个模型但一个 pricing 字段都没有」
/// 「有 pricing 但数字对不上每 token 的量级」是三件完全不同的事 ——
/// 第一件去换密钥、第二件去手工录、第三件是这边解析要改。分不清就只能瞎试。
pub async fn fetch_pricing(
    det: &Detection,
    base_url: &str,
    api_key: &str,
    console_token: &str,
) -> (Vec<RelayPrice>, String) {
    let Some(http) = client() else { return (Vec::new(), "建不出 HTTP 客户端".into()) };
    let root = site_root(base_url);
    let by_family = match &det.family {
        Family::Sub2Api => sub2api_pricing(&http, &root, console_token).await,
        // one-api 系的分支多数是 new-api 的下游，`/api/pricing` 那条路照样可能通。
        // 试一次的成本是一个往返，形状对不上就回空 —— 而不试就是白白放弃一整族。
        Family::NewApi | Family::OneApi | Family::OneApiFork(_) => {
            new_api_pricing(&http, &root, det.quota_per_unit.unwrap_or(500_000.0), console_token)
                .await
        }
        // OpenRouter 传原始 base_url，别传 site_root —— 理由见 openrouter_pricing。
        Family::OpenRouter => openrouter_pricing(&http, base_url).await,
        Family::Custom(_) | Family::Unknown => Vec::new(),
    };
    if !by_family.is_empty() {
        return (by_family, String::new());
    }
    // 专用接口没拿到 —— **再走一条通用的**：带上调用密钥问 `/v1/models`。
    //
    // 这一条对自研网关是唯一的路（每家自己写的面板没有共同接口），对面板被关掉
    // 或被人机校验挡住的站也是退路。OpenRouter 的 `/v1/models` 回包里带 pricing，
    // 这个约定被抄得很广 —— 抄字段名的基本也抄了单位（美元每 token）。
    openai_models_pricing(&http, &root, api_key).await
}

/// 通用退路：`GET /v1/models`，按 OpenRouter 的约定读价。
///
/// # 只认得出单位的才认，认不出的一律不要
///
/// 各家字段名五花八门：`input_price`、`prompt_price`、`price_in`……而它们的**单位**
/// 有的是每 token、有的是每百万 token，从字段名根本分不出来。猜错就是差一百万倍
/// 而看起来完全正常的数字 —— 那比没有价目糟得多，因为对账会一脸自信地报出来。
///
/// 所以这里只认 OpenRouter 那套确切的字段名（`pricing.prompt` / `pricing.completion`
/// 及其 input/output 别名），它们的单位是**美元每 token**，抄这个字段名的基本
/// 也在抄这个单位。再加一道量级闸：每 token 单价必须小于 0.001 美元（＝每百万
/// token 一千美元）。一个每百万 token 计价的数（比如 5.0）会被这道闸直接挡掉，
/// 而不是被当成天价悄悄记进去。
async fn openai_models_pricing(
    http: &reqwest::Client,
    root: &str,
    api_key: &str,
) -> (Vec<RelayPrice>, String) {
    let mut out = Vec::new();
    if api_key.trim().is_empty() {
        return (out, "这个出口没配调用密钥，/v1/models 问不了".into());
    }
    let resp = get(http, &format!("{root}/v1/models"), Some(api_key)).await;
    let Some((code, body)) = resp else {
        return (out, "/v1/models 连不上".into());
    };
    if code != 200 {
        return (out, format!("/v1/models 回 {code}（密钥被拒或这家没这条路由）"));
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else {
        return (out, "/v1/models 回的不是 JSON".into());
    };
    let Some(items) = v.get("data").and_then(|d| d.as_array()) else {
        return (out, "/v1/models 回的 JSON 里没有 data 数组".into());
    };
    // 数一数「有 pricing 字段但被单位闸挡掉」的有几个 —— 这一类和「压根没有 pricing」
    // 要分开报：前者说明这家其实给了价，只是形状／单位我们还认不出，值得来改解析。
    let mut has_field = 0usize;
    let mut unit_rejected = 0usize;

    for it in items {
        let Some(name) = it.get("id").and_then(|x| x.as_str()) else { continue };
        let Some(pr) = it.get("pricing") else { continue };
        has_field += 1;
        let input = per_token(pr, &["prompt", "input"]);
        let output = per_token(pr, &["completion", "output"]);
        if input.is_none() || output.is_none() {
            unit_rejected += 1;
        }
        // **输出价必须是正数**才收。回包里 pricing 字段存在但全是 0 的站不少
        // （字段占位、没填），照收就是把成本记成零 —— 那正是「亏损显示成盈利」
        // 的那个方向。宁可这家没有价，也不要一份零。
        let (Some(input), Some(output)) = (input, output) else { continue };
        if !(output > 0.0) {
            continue;
        }
        out.push(RelayPrice {
            model: name.to_string(),
            prices: UnitPrices {
                input,
                output,
                cache_read: per_token(pr, &["input_cache_read", "cache_read"]),
                cache_write: per_token(pr, &["input_cache_write", "cache_write"]),
                per_request: None,
            },
            group: None,
            group_multiplier: 1.0,
            source: "/v1/models 的 pricing 字段（OpenRouter 约定，美元每 token）".into(),
        });
    }
    let why = if !out.is_empty() {
        String::new()
    } else if has_field == 0 {
        format!("/v1/models 回了 {} 个模型，但一个都没带 pricing 字段", items.len())
    } else if unit_rejected > 0 {
        format!(
            "/v1/models 里有 {has_field} 个模型带 pricing，但数字对不上「美元每 token」的量级 —— \
             多半是每百万 token 计价。没有换算依据，不猜"
        )
    } else {
        format!("/v1/models 里 {has_field} 个模型的 pricing 输出价都是 0，当作没有价")
    };
    (out, why)
}

/// 从 pricing 对象里按名字取一个**每 token 美元**的价。
///
/// 字符串和数字都收（OpenRouter 回的是字符串）。超出量级闸的一律当认不出 ——
/// 那多半是每百万 token 的数字，收进来会差一百万倍。
fn per_token(pr: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    /// 每 token 一千分之一美元 = 每百万 token 一千美元。真实模型不可能到这个价，
    /// 到了就说明这个数不是「每 token」。
    const MAX_PER_TOKEN_USD: f64 = 0.001;
    for k in keys {
        // 这里必须 `continue` 而不是 `?`：`?` 是从**整个函数**返回，第一个别名不存在
        // 就再也轮不到第二个 —— 别名那条路会整个失效，而且不报错。
        let Some(raw) = pr.get(*k) else { continue };
        let n = raw
            .as_f64()
            .or_else(|| raw.as_str().and_then(|s| s.trim().parse::<f64>().ok()));
        if let Some(n) = n {
            if n.is_finite() && (0.0..MAX_PER_TOKEN_USD).contains(&n) {
                return Some(n);
            }
            // 有这个字段但数字对不上单位 —— **别退而求其次去试下一个别名**，
            // 那会拿另一个字段的数配上这个字段的语义。整项作废。
            return None;
        }
    }
    None
}

/// sub2api：价目在「模型广场」，**站长开了就完全公开**，一个凭据都不用。
///
/// 实测（2026-08-25）：api.hanhegufei.online 开着，53KB 结构化价目；
/// zyz.qingyanzhiying.top 和 polly.modelbridge.cc 关着，回一个**JSON 404**
/// `{"code":404,"message":"Model plaza is not enabled"}` —— 注意是 JSON 不是 SPA 的
/// HTML 兜底页，这本身也是 sub2api 的一条指纹。关着的那两家退回 /api/v1/groups/rates，
/// 那个要控制台令牌。
async fn sub2api_pricing(http: &reqwest::Client, root: &str, token: &str) -> Vec<RelayPrice> {
    let mut out = Vec::new();
    let plaza = get(http, &format!("{root}/api/v1/model-plaza"), None).await;
    if let Some((200, body)) = plaza {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            collect_sub2api_groups(&v, "/api/v1/model-plaza", &mut out);
            if !out.is_empty() {
                return out;
            }
        }
    }
    // 广场关了：退回要鉴权的那条。没有令牌就只能空手回 —— 界面会说清楚。
    if token.trim().is_empty() {
        return out;
    }
    if let Some((200, body)) = get(http, &format!("{root}/api/v1/groups/rates"), Some(token)).await {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
            collect_sub2api_groups(&v, "/api/v1/groups/rates", &mut out);
        }
    }
    out
}

/// 从 sub2api 的分组结构里把每个模型的实际单价抠出来。
///
/// 结构（实测）：`data.groups[] { name, rate_multiplier, peak_*, models[] { name, pricing } }`，
/// 而 `pricing.input_price` 已经是**美元每 token**（`5e-06` = $5/百万），不需要任何换算。
///
/// **分组倍率要乘进去。** 线上那家的倍率从 0.07 到 1.0 —— 不乘的话，claude_kiro 分组的
/// 成本会被算成实际的 14 倍，对账页上每一行 Claude 都会显示成巨亏。
fn collect_sub2api_groups(v: &serde_json::Value, source: &str, out: &mut Vec<RelayPrice>) {
    let groups = v
        .get("data")
        .and_then(|d| d.get("groups"))
        .and_then(|g| g.as_array());
    let Some(groups) = groups else { return };
    for g in groups {
        let gname = g.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
        // 倍率缺省按 1.0：缺字段时**不能**当成 0，那会让整组成本变成零、显示成 100% 毛利。
        let mult = g.get("rate_multiplier").and_then(|x| x.as_f64()).unwrap_or(1.0);
        let Some(models) = g.get("models").and_then(|m| m.as_array()) else { continue };
        for m in models {
            let Some(name) = m.get("name").and_then(|x| x.as_str()) else { continue };
            let p = m.get("pricing").unwrap_or(&serde_json::Value::Null);
            let f = |k: &str| p.get(k).and_then(|x| x.as_f64());
            // 输入和输出都拿不到就跳过：只有一半的价算出来的成本是偏低的**真数字**，
            // 比空白更危险 —— 它看起来有效。
            let (Some(input), Some(output)) = (f("input_price"), f("output_price")) else {
                continue;
            };
            out.push(RelayPrice {
                model: name.to_string(),
                prices: UnitPrices {
                    input: input * mult,
                    output: output * mult,
                    cache_read: f("cache_read_price").map(|x| x * mult),
                    cache_write: f("cache_write_price").map(|x| x * mult),
                    per_request: f("per_request_price").map(|x| x * mult),
                },
                group: (!gname.is_empty()).then(|| gname.clone()),
                group_multiplier: mult,
                source: source.to_string(),
            });
        }
    }
}

/// new-api：`/api/pricing` 给倍率，换算成美元要除以 `quota_per_unit`。
///
/// 公式（源码核对过）：`美元/token = model_ratio × group_ratio ÷ quota_per_unit`，
/// 输出再乘 `completion_ratio`。默认 quota_per_unit = 500000，也就是
/// `model_ratio × 2e-6` —— 和 new-api 前端显示的 `model_ratio × 2 USD/1M` 对得上。
///
/// **quota_per_unit 必须按站取，不能写死 500000。** 调研发现 metapi 那个项目把某个
/// 分支的除数写成了 1000000，全站成本差一倍 —— 而那个数字看起来完全正常。
async fn new_api_pricing(
    http: &reqwest::Client,
    root: &str,
    quota_per_unit: f64,
    console_token: &str,
) -> Vec<RelayPrice> {
    let mut out = Vec::new();
    if quota_per_unit <= 0.0 {
        return out; // 除数无效，宁可不给价
    }
    // 先不带凭据问一次（多数站是公开的），403 再带控制台令牌重问一次。
    //
    // 线上 wecodex.lol 就卡在这里：不带凭据回 `{"message":"pricing is disabled"}`，
    // 而那道开关拦的是**匿名**访问 —— 登录态下同一条路可能是通的。不重试一次
    // 就等于把一家有价目的站当成没有。
    let mut resp = get(http, &format!("{root}/api/pricing"), None).await;
    if !matches!(resp, Some((200, _))) && !console_token.trim().is_empty() {
        resp = get(http, &format!("{root}/api/pricing"), Some(console_token)).await;
    }
    let Some((200, body)) = resp else {
        return out;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else { return out };

    // 分组倍率表。取最小的那个当默认 —— 我们的密钥属于哪个分组这里看不到，
    // 而**低估成本比高估危险**：低估会让亏损的行显示成盈利。所以这里取最贵的
    // （倍率最大的）来算，宁可把成本算高。
    let group_ratio = v.get("group_ratio").and_then(|x| x.as_object());
    let worst = group_ratio
        .map(|m| {
            m.values()
                .filter_map(|x| x.as_f64())
                .fold(f64::NEG_INFINITY, f64::max)
        })
        .filter(|x| x.is_finite())
        .unwrap_or(1.0);

    let Some(items) = v.get("data").and_then(|d| d.as_array()) else { return out };
    for it in items {
        let Some(name) = it.get("model_name").and_then(|x| x.as_str()) else { continue };
        let f = |k: &str| it.get(k).and_then(|x| x.as_f64());
        // quota_type 1 = 按次计费，倍率不参与。
        if it.get("quota_type").and_then(|x| x.as_i64()) == Some(1) {
            if let Some(price) = f("model_price") {
                out.push(RelayPrice {
                    model: name.into(),
                    prices: UnitPrices {
                        input: 0.0,
                        output: 0.0,
                        cache_read: None,
                        cache_write: None,
                        per_request: Some(price * worst),
                    },
                    group: None,
                    group_multiplier: worst,
                    source: "/api/pricing (按次)".into(),
                });
            }
            continue;
        }
        let Some(ratio) = f("model_ratio") else { continue };
        let base = ratio * worst / quota_per_unit;
        let completion = f("completion_ratio").unwrap_or(1.0);
        out.push(RelayPrice {
            model: name.into(),
            prices: UnitPrices {
                input: base,
                output: base * completion,
                cache_read: f("cache_ratio").map(|c| base * c),
                cache_write: f("create_cache_ratio").map(|c| base * c),
                per_request: None,
            },
            group: None,
            group_multiplier: worst,
            source: "/api/pricing".into(),
        });
    }
    out
}

/// OpenRouter：`/api/v1/models` 公开，`pricing.prompt` / `pricing.completion`
/// 是**十进制字符串**形式的「美元每 token」，直接可用。
async fn openrouter_pricing(http: &reqwest::Client, api_base: &str) -> Vec<RelayPrice> {
    let mut out = Vec::new();
    // **这里用的是 base_url 本身，不是 site_root。**
    //
    // OpenRouter 的地址形如 `https://openrouter.ai/api/v1`，而它的接口就挂在这个
    // 前缀下面（`{base}/models`）。走 site_root 的话 `/v1` 被剥掉只剩 `/api`，
    // 再拼 `/api/v1/models` 就成了 `openrouter.ai/api/api/v1/models` —— 404，
    // 表现是「这家一条价都拉不到」而不是任何报错。线上实测踩到过这一发。
    let base = api_base.trim_end_matches('/');
    let Some((200, body)) = get(http, &format!("{base}/models"), None).await else {
        return out;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else { return out };
    let Some(items) = v.get("data").and_then(|d| d.as_array()) else { return out };
    for it in items {
        let Some(name) = it.get("id").and_then(|x| x.as_str()) else { continue };
        let p = it.get("pricing").unwrap_or(&serde_json::Value::Null);
        // 字符串形式，别当数字读 —— as_f64() 对 "0.0000001" 返回 None。
        let f = |k: &str| {
            p.get(k)
                .and_then(|x| x.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v >= 0.0)
        };
        let (Some(input), Some(output)) = (f("prompt"), f("completion")) else { continue };
        out.push(RelayPrice {
            model: name.into(),
            prices: UnitPrices {
                input,
                output,
                cache_read: f("input_cache_read"),
                cache_write: f("input_cache_write"),
                per_request: f("request"),
            },
            group: None,
            group_multiplier: 1.0,
            source: "/api/v1/models".into(),
        });
    }
    out
}

// ---------------------------------------------------------------- 充值套餐

/// 一档充值套餐：付多少，到账多少。
#[derive(Clone, Debug, serde::Serialize)]
pub struct TopupPlan {
    pub key: String,
    pub name: String,
    pub price: f64,
    pub currency: String,
    /// 到账余额。None = 套餐表里没给这个数 —— **不能当 0**，那会让比例算成无穷大。
    pub granted: Option<f64>,
    /// 认不出字段时的原文片段。空 = 认出来了。
    pub raw: String,
}

impl TopupPlan {
    /// 一块钱买到多少余额。None = 缺 granted 或 price 非正。
    pub fn rate(&self) -> Option<f64> {
        let g = self.granted?;
        (self.price > 0.0 && g.is_finite()).then(|| g / self.price)
    }
}

/// 拉这家的充值套餐表。
///
/// # 为什么不是「取一个汇率」
///
/// 实测线上三家 sub2api：前端的 currency 分包只是格式化工具（货币符号表 +
/// Intl.NumberFormat，默认 CNY），**整个前端没有汇率常量**。比例是逐套餐定的
/// （¥50 一档、¥200 一档，各自的到账金额不成比例是常事），所以能取的只有套餐表。
///
/// # 要控制台令牌
///
/// `/api/v1/payment/plans` 未鉴权时回 401（三家都是）。没令牌就空手回 ——
/// 那时靠余额跳升兜底，见 `endpoint_topup_event`。
pub async fn fetch_topup_plans(
    det: &Detection,
    base_url: &str,
    console_token: &str,
) -> (Vec<TopupPlan>, String) {
    // 空手回的时候必须说清是**哪一条**路径断的。
    //
    // 五条失败路径原来一律 `return Vec::new()`，一行日志都不留，于是
    // `endpoint_topup_plan` 是空表 —— 而空表有五种完全不同的原因，处置也完全不同：
    // 「没配控制台令牌」是运营去后台填一下，「接口回的形状变了」是我们要改代码。
    // 分不出来的时候，两种都只能干等着。
    if det.family != Family::Sub2Api {
        return (Vec::new(), format!("这家是 {} 不是 sub2api，没有充值套餐接口", det.family.label()));
    }
    if console_token.trim().is_empty() {
        return (Vec::new(), "没配控制台令牌 —— 充值套餐接口要它，配上就能自动算出真实进价".into());
    }
    let Some(http) = client() else {
        return (Vec::new(), "HTTP 客户端建不起来".into());
    };
    let root = site_root(base_url);
    let got = get(&http, &format!("{root}/api/v1/payment/plans"), Some(console_token.trim())).await;
    let body = match got {
        Some((200, b)) => b,
        Some((code, _)) => {
            return (
                Vec::new(),
                match code {
                    401 | 403 => format!("控制台令牌被拒（HTTP {code}），多半过期了"),
                    404 => "这家没有 /api/v1/payment/plans 这个接口".into(),
                    _ => format!("充值套餐接口回 HTTP {code}"),
                },
            );
        }
        None => return (Vec::new(), "充值套餐接口连不上".into()),
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) else {
        return (Vec::new(), "充值套餐接口回的不是 JSON".into());
    };
    // 套餐数组可能在 data、data.plans、data.items 或顶层。逐个试 ——
    // 写死一条路径的话，换个版本就整表拉不到，而表现只是「没有套餐」。
    let arr = ["plans", "items", "list"]
        .iter()
        .find_map(|k| v.get("data").and_then(|d| d.get(*k)).and_then(|x| x.as_array()))
        .or_else(|| v.get("data").and_then(|d| d.as_array()))
        .or_else(|| v.as_array());
    let Some(arr) = arr else {
        return (Vec::new(), "回的 JSON 里找不到套餐数组（这家换版本了？）".into());
    };

    let mut out = Vec::new();
    for (i, p) in arr.iter().enumerate() {
        let num = |names: &[&str]| -> Option<f64> {
            names.iter().find_map(|n| {
                p.get(*n).and_then(|x| {
                    x.as_f64().or_else(|| x.as_str().and_then(|t| t.trim().parse().ok()))
                })
            })
        };
        let text = |names: &[&str]| -> Option<String> {
            names.iter().find_map(|n| p.get(*n).and_then(|x| x.as_str()).map(str::to_string))
        };
        // 付款金额认不出来就整档跳过：没有分母算不出比例，而留一档 price=0
        // 会让比例变成无穷大，那比没有更糟。
        let Some(price) = num(&["price", "amount", "cost", "cny", "money"]) else {
            continue;
        };
        let granted = num(&["balance", "granted", "credit", "credits", "quota", "value", "give"]);
        let key = text(&["id", "plan_id", "key", "code"])
            .or_else(|| text(&["name", "title"]))
            .unwrap_or_else(|| format!("#{i}"));
        out.push(TopupPlan {
            key,
            name: text(&["name", "title", "label"]).unwrap_or_default(),
            price,
            currency: text(&["currency", "currency_code"]).unwrap_or_else(|| "CNY".into()),
            granted,
            // 到账金额认不出来时，把这一档的原文留下 —— 「字段名和我们猜的不一样」
            // 和「这档本来就不送余额」在结果上都是 None，在处理上完全不同。
            raw: if granted.is_none() {
                p.to_string().chars().take(200).collect()
            } else {
                String::new()
            },
        });
    }
    // 一档都没解析出来也要说清楚：接口通、JSON 也对，但每一档都缺付款金额，
    // 那是字段名对不上，不是「这家没有充值套餐」。
    let reason = if out.is_empty() {
        format!("接口通，但 {} 档里一档都没认出付款金额（字段名换了？）", arr.len())
    } else {
        String::new()
    };
    (out, reason)
}

// ---------------------------------------------------------------- 余额

/// 按家族查余额。
///
/// 实测的关键结论：**sub2api 的余额用调用密钥就能查**（`/v1/usage`），
/// 不需要控制台令牌。线上那三家全是 sub2api，所以它们不用配任何新东西。
pub async fn fetch_balance(
    det: &Detection,
    base_url: &str,
    api_key: &str,
    console_token: &str,
) -> Option<Balance> {
    let http = client()?;
    let root = site_root(base_url);
    match det.family {
        Family::Sub2Api => {
            // 先用调用密钥走 /v1/usage —— 实测这条对线上三家都通，不用配任何新东西。
            if let Some((200, body)) = get(&http, &format!("{root}/v1/usage"), Some(api_key)).await
            {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(b) = pick_money(&v, "/v1/usage") {
                        return Some(b);
                    }
                }
            }
            // 那条被关掉/改过时的退路：控制台面。没有令牌就只能空手回。
            //
            // 留这条不是防御性编程：`/v1/usage` 是**站长可关**的，而一旦关掉，
            // 没有退路就意味着这家的余额从此永久空白，且看不出是被关了还是没实现。
            if console_token.trim().is_empty() {
                return None;
            }
            for path in ["/api/v1/auth/me", "/api/v1/subscriptions/summary"] {
                if let Some((200, body)) =
                    get(&http, &format!("{root}{path}"), Some(console_token)).await
                {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(b) = pick_money(&v, path) {
                            return Some(b);
                        }
                    }
                }
            }
            None
        }
        Family::OpenRouter => {
            // 同上：接口在 base_url 前缀下面，不在站点根下。
            let base = base_url.trim_end_matches('/');
            let (code, body) = get(&http, &format!("{base}/auth/key"), Some(api_key)).await?;
            if code != 200 {
                return None;
            }
            let v = serde_json::from_str::<serde_json::Value>(&body).ok()?;
            pick_money(&v, "/api/v1/auth/key")
        }
        // one-api 系：OpenAI 兼容那条用调用密钥就行，比 /api/user/self 少一个头。
        Family::NewApi | Family::OneApi | Family::OneApiFork(_) => {
            let _ = console_token; // 这一族的余额用调用密钥就够，见下面那条 OpenAI 兼容路径
            let qpu = det.quota_per_unit.unwrap_or(500_000.0);
            if let Some((200, body)) =
                get(&http, &format!("{root}/api/usage/token"), Some(api_key)).await
            {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    let d = v.get("data").unwrap_or(&v);
                    let g = |k: &str| d.get(k).and_then(|x| x.as_f64());
                    if let Some(left) = g("total_available") {
                        let used = g("total_used");
                        return Some(Balance {
                            text: format!(
                                "${:.4}{}",
                                left / qpu,
                                used.map(|u| format!("（已用 ${:.4}）", u / qpu)).unwrap_or_default()
                            ),
                            remaining_usd: Some(left / qpu),
                            used_usd: used.map(|u| u / qpu),
                            source: "/api/usage/token".into(),
                        });
                    }
                }
            }
            None
        }
        // 自研网关：**试一次 `/v1/usage`**，别直接放弃。
        //
        // 这条路是 OpenAI 生态的既成惯例，很多自研网关照着实现了（sub2api 那一支
        // 就是这么查的），而它只要一个调用密钥、我们本来就有。拿不到就如实空手回 ——
        // 认不出面板不等于余额一定查不到，反过来也一样。
        Family::Custom(_) => {
            let mut resp = get(&http, &format!("{root}/v1/usage"), Some(api_key)).await?;
            // 400 = 路由在、密钥认了，但请求缺东西。OpenAI 那套 `/v1/usage` 要日期区间，
            // 抄这条路由的网关多半也要。线上 teamorouter 就卡在这里：不带参数回 400，
            // 而它是这家唯一的账户接口（其余 /v1/balance、/v1/credits 全是 404）。
            if resp.0 == 400 {
                let today = chrono::Utc::now().date_naive();
                let from = today - chrono::Duration::days(30);
                let url = format!("{root}/v1/usage?start_date={from}&end_date={today}");
                if let Some(r2) = get(&http, &url, Some(api_key)).await {
                    resp = r2;
                }
            }
            let (code, body) = resp;
            if code != 200 {
                tracing::info!(host = %root, %code, "[balance-shape] 自研网关 /v1/usage 不是 200（带日期区间也重试过了）");
                return None;
            }
            let v = serde_json::from_str::<serde_json::Value>(&body).ok()?;
            let got = pick_money(&v, "/v1/usage");
            if got.is_none() {
                // 200 了却认不出金额。把**字段名**记下来（只记名字，不记值 ——
                // 值里可能有账号信息）。下一步该怎么解析，全看这一行。
                // 认不出就说认不出，而不是让这家的余额永远空白且看不出为什么。
                let keys: Vec<&str> = v
                    .as_object()
                    .map(|o| o.keys().map(|k| k.as_str()).collect())
                    .unwrap_or_default();
                tracing::info!(
                    host = %root,
                    keys = %keys.join(","),
                    "[balance-shape] 自研网关 /v1/usage 回了 200 但认不出金额字段"
                );
            }
            got
        }
        Family::Unknown => None,
    }
}

/// 从一份形状未知的 JSON 里挑出金额。**不做单位换算** —— 各家单位不同，
/// 猜错会得到一个差几个数量级却看起来完全正常的数字。
fn pick_money(v: &serde_json::Value, source: &str) -> Option<Balance> {
    let scopes: Vec<&serde_json::Value> = std::iter::once(v)
        .chain(["data", "user", "result"].iter().filter_map(|k| v.get(*k)))
        .collect();
    let num = |o: &serde_json::Value, names: &[&str]| -> Option<(String, f64)> {
        for n in names {
            if let Some(x) = o.get(*n) {
                let f = x.as_f64().or_else(|| x.as_str().and_then(|t| t.trim().parse().ok()));
                if let Some(f) = f.filter(|f: &f64| f.is_finite()) {
                    return Some(((*n).to_string(), f));
                }
            }
        }
        None
    };
    for o in scopes {
        let left = num(o, &["balance", "remaining", "limit_remaining", "credit", "credits"]);
        // `total_usage` 是 OpenAI 那套 /v1/usage 的字段名（单位是**分**），
        // 抄这条路由的网关多半也抄了字段名。放在最后：前面几个都是余额单位，
        // 只有认不出时才轮到它。
        let used = num(o, &["used", "usage", "total_used", "used_quota", "consumed", "total_usage"]);
        if left.is_none() && used.is_none() {
            continue;
        }
        let text = match (&left, &used) {
            (Some((ln, lv)), Some((un, uv))) => format!("{ln} {lv:.4}（{un} {uv:.4}）"),
            (Some((ln, lv)), None) => format!("{ln} {lv:.4}"),
            (None, Some((un, uv))) => format!("已用 {un} {uv:.4}"),
            (None, None) => unreachable!(),
        };
        return Some(Balance {
            text,
            remaining_usd: left.map(|(_, v)| v),
            used_usd: used.map(|(_, v)| v),
            source: source.to_string(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> String {
        let all = include_str!("relay_adapter.rs");
        all.split("\n#[cfg(test)]").next().unwrap().to_string()
    }

    /// 按花括号配对抠出一个函数体。
    ///
    /// 不用「从函数名切固定长度」：函数一长，切片就越过函数尾巴、把**下一个函数**
    /// 的代码也框进来 —— 于是一条「这个函数里不许出现 X」的断言会被隔壁函数里
    /// 合法的 X 触发。这个坑刚踩过一次（quota 那条断言撞上了 fetch_balance 的兜底值）。
    fn fn_body(src: &str, sig: &str) -> String {
        let at = src.find(sig).unwrap_or_else(|| panic!("找不到 {sig}"));
        let open = at + src[at..].find('{').expect("函数没有花括号");
        let bytes = src.as_bytes();
        let (mut depth, mut i) = (0i32, open);
        while i < bytes.len() {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return src[open..=i].to_string();
                    }
                }
                _ => {}
            }
            i += 1;
        }
        panic!("{sig} 的花括号没配平");
    }

    /// `/v1` 必须从站点根里剥掉。
    ///
    /// 后台里两种写法都有。不剥的话所有控制台探针会打到 `/v1/api/status`，
    /// 一路 404 —— 表现是「全网都认不出来」，而原因只是多了三个字符。
    #[test]
    fn the_v1_suffix_is_stripped_before_probing_console_paths() {
        assert_eq!(site_root("https://x.com/v1"), "https://x.com");
        assert_eq!(site_root("https://x.com/v1/"), "https://x.com");
        assert_eq!(site_root("https://x.com/"), "https://x.com");
        assert_eq!(site_root("https://x.com"), "https://x.com");
        // 只剥尾部：路径里带 v1 的不能被误伤。
        assert_eq!(site_root("https://x.com/v1/gw"), "https://x.com/v1/gw");
    }

    /// 分组倍率必须乘进单价里。
    ///
    /// 线上实测那家的倍率是 0.07 —— 不乘的话，claude_kiro 分组每一行的成本
    /// 都会是实际的 14 倍，整页 Claude 显示成巨亏，而那是纯粹算出来的假象。
    #[test]
    fn the_group_multiplier_is_folded_into_the_unit_price() {
        let v: serde_json::Value = serde_json::json!({
            "data": { "groups": [{
                "name": "claude_kiro",
                "rate_multiplier": 0.07,
                "models": [{
                    "name": "claude-opus-4-6",
                    "pricing": {
                        "input_price": 5e-06, "output_price": 2.5e-05,
                        "cache_read_price": 5e-07, "cache_write_price": 6.25e-06
                    }
                }]
            }]}
        });
        let mut out = Vec::new();
        collect_sub2api_groups(&v, "t", &mut out);
        assert_eq!(out.len(), 1);
        let p = &out[0];
        assert!((p.prices.input - 5e-06 * 0.07).abs() < 1e-15, "输入价没乘倍率");
        assert!((p.prices.output - 2.5e-05 * 0.07).abs() < 1e-15, "输出价没乘倍率");
        assert!((p.group_multiplier - 0.07).abs() < 1e-12, "倍率没留痕，界面解释不了折扣");
        assert_eq!(p.group.as_deref(), Some("claude_kiro"));
    }

    /// 倍率字段缺失时按 1.0，**不能**按 0。
    ///
    /// 按 0 的话整组成本变成零，对账页显示成 100% 毛利 —— 一个由缺失字段
    /// 凭空造出来的「这条线路白赚」。
    #[test]
    fn a_missing_multiplier_defaults_to_one_not_zero() {
        let v: serde_json::Value = serde_json::json!({
            "data": { "groups": [{
                "name": "g",
                "models": [{ "name": "m", "pricing": { "input_price": 1e-06, "output_price": 2e-06 }}]
            }]}
        });
        let mut out = Vec::new();
        collect_sub2api_groups(&v, "t", &mut out);
        assert_eq!(out.len(), 1);
        assert!((out[0].prices.input - 1e-06).abs() < 1e-15, "缺倍率时把价算没了");
        assert!((out[0].group_multiplier - 1.0).abs() < 1e-12);
    }

    /// 只有一半价格的模型必须整个跳过。
    ///
    /// 只取到输入价、把输出价当 0，会算出一个**偏低的真数字** —— 它比空白危险，
    /// 因为它看起来是有效的，而且会让那一行显示成高毛利。
    #[test]
    fn a_half_priced_model_is_skipped_entirely() {
        let v: serde_json::Value = serde_json::json!({
            "data": { "groups": [{
                "name": "g", "rate_multiplier": 1,
                "models": [
                    { "name": "half", "pricing": { "input_price": 1e-06 }},
                    { "name": "full", "pricing": { "input_price": 1e-06, "output_price": 2e-06 }}
                ]
            }]}
        });
        let mut out = Vec::new();
        collect_sub2api_groups(&v, "t", &mut out);
        assert_eq!(out.len(), 1, "只有一半价格的模型被收进来了");
        assert_eq!(out[0].model, "full");
    }

    /// OpenRouter 的价是**字符串**，不能当数字读。
    #[test]
    fn openrouter_prices_are_decimal_strings() {
        let p = serde_json::json!({ "prompt": "0.0000001", "completion": "0.0000004" });
        // 直接 as_f64 是拿不到的 —— 这条钉住「必须先 as_str 再 parse」。
        assert!(p.get("prompt").unwrap().as_f64().is_none(), "OpenRouter 改成数字了？那这条要重写");
        let s = src();
        assert!(
            s.contains(".and_then(|x| x.as_str())") && s.contains("parse::<f64>()"),
            "OpenRouter 的价改成按数字读了 —— 会整份拿不到价，而且是静默的",
        );
    }

    /// 认不出来时必须是 Unknown **且带原因**，不能落到某个「差不多」的家族上。
    #[test]
    fn an_unrecognised_relay_is_unknown_with_a_reason() {
        let s = src();
        // 每一条 unknown(...) 都得带一句话。
        let mut at = 0usize;
        let mut n = 0usize;
        while let Some(i) = s[at..].find("unknown(") {
            let j = at + i + "unknown(".len();
            // 参数可能换行写。跳过空白再看第一个字符是不是引号 ——
            // 只认同一行的话，rustfmt 换个行这条守卫就假红。
            let rest = s[j..].trim_start();
            assert!(
                rest.starts_with('"') && !rest.starts_with("\"\""),
                "有一处 unknown() 没给原因 —— 面板上会是一片空白，没人知道该做什么",
            );
            n += 1;
            at = j;
        }
        assert!(n >= 3, "只找到 {n} 处 unknown()，这条守卫多半没在看真正的分支");
    }

    /// 对**线上真实中转**跑一遍识别和拉价。
    ///
    /// `#[ignore]` 是刻意的：它要联网，跑在 CI 里会因为对方抖动而假红，而假红的
    /// 守卫最后都会被人加 `--skip` 绕过去。手动跑：
    ///
    /// ```text
    /// cargo test --offline relay_adapter::tests::live -- --ignored --nocapture
    /// ```
    ///
    /// 单元测试只证明逻辑自洽 —— 一个能通过全部单元测试、却把真实中转认错的
    /// 检测器是完全无用的，而那种错只有打真实站点才看得出来。
    #[tokio::test]
    #[ignore]
    async fn live_relays_are_identified_and_priced() {
        // OpenRouter 必须在名单里：它的地址形状（`/api/v1` 结尾）和别家不同，
        // 上一版就是因为它没被覆盖，一个拼错 URL 的 bug 一路到了线上才被发现。
        for (url, want) in [
            ("https://api.hanhegufei.online", Family::Sub2Api),
            ("https://zyz.qingyanzhiying.top", Family::Sub2Api),
            ("https://polly.modelbridge.cc/v1", Family::Sub2Api),
            ("https://openrouter.ai/api/v1", Family::OpenRouter),
        ] {
            let det = detect(url).await;
            println!("{url}\n  家族: {}  凭据: {}", det.family.label(), det.matched_by);
            if !det.note.is_empty() {
                println!("  说明: {}", det.note);
            }
            assert_eq!(det.family, want, "{url} 认错了");

            let (prices, _why) = fetch_pricing(&det, url, "", "").await;
            println!("  无凭据拉到 {} 条价目", prices.len());
            // OpenRouter 的价目是公开的，拉到 0 条一定是我们这边拼错了 URL。
            if want == Family::OpenRouter {
                assert!(!prices.is_empty(), "OpenRouter 一条价都没拉到 —— 多半是 URL 拼错");
            }
            for p in prices.iter().take(3) {
                println!(
                    "    {:<28} in={:.3e} out={:.3e} 分组={} ×{}",
                    p.model,
                    p.prices.input,
                    p.prices.output,
                    p.group.clone().unwrap_or_default(),
                    p.group_multiplier
                );
            }
            // 价目全部要是正数且有限 —— 0 或 NaN 会让成本算成 0，显示成 100% 毛利。
            for p in &prices {
                assert!(p.prices.input.is_finite() && p.prices.input >= 0.0, "{} 输入价异常", p.model);
                assert!(p.prices.output.is_finite() && p.prices.output >= 0.0, "{} 输出价异常", p.model);
            }
        }
    }

    /// 充值比例是**逐套餐**的，不能塌成一个汇率。
    ///
    /// 实测：这几家中转的前端整个没有汇率常量（currency 分包只是格式化工具）。
    /// 每档充值各自定价，¥50 和 ¥200 两档的到账金额常常不成比例 —— 取平均会把
    /// 「哪一档划算」这件事整个抹平，而那正是这张表唯一的用途。
    /// 拉不到充值套餐时，**必须说清是哪一条路径断的**。
    ///
    /// 五条失败路径原来一律 `return Vec::new()`，一行日志都不留，于是
    /// `endpoint_topup_plan` 是空表 —— 而空表有五种完全不同的原因，处置也完全不同：
    /// 「没配控制台令牌」是运营去后台填一下，「接口回的形状变了」是我们改代码。
    /// 分不出来的时候两种都只能干等着，而这一格空着就意味着人民币成本只能手填。
    #[test]
    fn every_way_the_topup_fetch_can_fail_says_which_one() {
        let src = include_str!("relay_adapter.rs");
        let at = src
            .find("pub async fn fetch_topup_plans(")
            .expect("fetch_topup_plans 改名了");
        let end = src[at..]
            .find("\n// ---------------------------------------------------------------- 余额")
            .map(|i| at + i)
            .expect("找不到函数结尾");
        // **先把注释剥掉再断言。** 函数体里那段解释性注释本身就写着「没配控制台令牌」，
        // 不剥的话，把真正的提示语删空这条测试照样绿 —— 变异测试当场翻出来的。
        let body: String = src[at..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let body = body.as_str();
        // 返回类型必须带上原因，否则调用方无从记录。
        assert!(
            body.contains("-> (Vec<TopupPlan>, String)"),
            "空手回的时候没带上原因",
        );
        // 五条路径逐条点名。少一条就是又多了一种「空表，不知道为什么」。
        for (needle, what) in [
            ("不是 sub2api", "家族不对"),
            ("没配控制台令牌", "缺令牌"),
            ("HTTP", "接口非 200"),
            ("不是 JSON", "回的不是 JSON"),
            ("找不到套餐数组", "JSON 形状变了"),
        ] {
            assert!(body.contains(needle), "{what}这条失败路径没说明原因");
        }
        // 光返回不算数，得真的落到库里让人看见。
        let sync = include_str!("relay_sync.rs");
        assert!(
            sync.contains("topup_reason = EXCLUDED.topup_reason"),
            "原因没写进 endpoint_adapter，页面上还是看不见",
        );
        let ui = include_str!("../admin-ui/src/pages/Adapters.tsx");
        assert!(ui.contains("充值套餐没拉到："), "页面没把原因显示出来");
    }

    /// 充值比例是**按档**的，不能取一个平均汇率。
    ///
    /// 少了 `#[test]`，这条从来没跑过（`cargo test -- --list` 里找不到它）。
    #[test]
    fn the_topup_rate_is_per_plan_not_a_single_exchange_rate() {
        let cheap = TopupPlan {
            key: "a".into(), name: "小额".into(), price: 50.0, currency: "CNY".into(),
            granted: Some(6.5), raw: String::new(),
        };
        let bulk = TopupPlan {
            key: "b".into(), name: "大额".into(), price: 200.0, currency: "CNY".into(),
            granted: Some(30.0), raw: String::new(),
        };
        // 两档的比例不同 —— 这正是不能取平均的理由。
        assert!((cheap.rate().unwrap() - 0.13).abs() < 1e-9);
        assert!((bulk.rate().unwrap() - 0.15).abs() < 1e-9);
        assert!(bulk.rate() > cheap.rate(), "大额档没有更划算，这个测试的前提要重看");
    }

    /// 到账金额认不出来时，比例必须是 None，不能当 0。
    ///
    /// 当 0 的话 `granted / price` = 0，界面会显示「1 元买到 $0.0000」——
    /// 一个看起来确定的、错的结论。而真相是「这家的字段名和我们猜的不一样」。
    #[test]
    fn a_plan_without_a_granted_amount_has_no_rate() {
        let p = TopupPlan {
            key: "x".into(), name: String::new(), price: 100.0, currency: "CNY".into(),
            granted: None, raw: "{\"price\":100,\"gift\":7}".into(),
        };
        assert!(p.rate().is_none(), "缺到账金额却算出了比例");
        assert!(!p.raw.is_empty(), "认不出字段时没留原文 —— 没法知道该往哪儿改");

        // price 为 0 或负也不许出比例（会得到无穷大）。
        let free = TopupPlan { price: 0.0, granted: Some(5.0), ..p.clone() };
        assert!(free.rate().is_none(), "付款金额为 0 时算出了无穷大的比例");
    }

    /// new-api 的额度单位不许写死。
    #[test]
    fn the_quota_unit_is_read_per_site_never_hardcoded() {
        let s = src();
        let sig = "async fn new_api_pricing(";
        assert!(s.contains("quota_per_unit: f64"), "价目函数不再按站接收额度单位了");
        let body = fn_body(&s, sig);
        let body = body.as_str();
        assert!(
            body.contains("/ quota_per_unit"),
            "换算没用按站取的除数 —— 站长改过这个值的话全站成本差一个倍数",
        );
        assert!(
            !body.contains("500_000.0") && !body.contains("500000.0"),
            "函数体里写死了额度单位",
        );
    }

    /// 分组倍率取不准时要往**贵**了算。
    ///
    /// 我们看不到自己的密钥属于哪个分组。低估成本会让亏损的行显示成盈利，
    /// 而这一页的用途正是发现亏损 —— 所以取倍率表里最大的那个。
    #[test]
    fn an_unknown_group_is_costed_at_the_most_expensive_ratio() {
        let s = src();
        let body = fn_body(&s, "async fn new_api_pricing(");
        assert!(
            body.contains("fold(f64::NEG_INFINITY, f64::max)"),
            "分组倍率没取最大值 —— 低估成本会把亏损显示成盈利",
        );
    }

    /// 「认不出」和「认出来了但拉不到」是两句不同的话。
    ///
    /// 上一版走到 `/api/status` 拿不到就一律回未知，线上两家因此被归到认不出：
    /// llm.ohub.vip 的 /api/* 全被人机校验挡住，api.teamorouter.com 是自研网关。
    /// 两家都有很硬的指纹 —— 判成未知会让人以为还有救，反复去点重探。
    #[test]
    fn a_gated_one_api_panel_is_still_identified() {
        // 线上 llm.ohub.vip 的原样回包。
        let st = Some((403u16, r#"{"success":false,"message":"无效的请求，验证码错误"}"#.to_string()));
        let d = gated_one_api(&st, 0).expect("被挡住的 one-api 面板没认出来");
        assert!(matches!(d.family, Family::OneApiFork(_)));
        // 拦住的原话必须原样带出来，否则运维不知道该去关哪道开关。
        assert!(d.note.contains("验证码错误"), "拦截原话被吞了");
        assert!(!d.matched_by.is_empty(), "没说靠什么认的");

        // 200 的走正常那条路，不该被这一条截胡。
        assert!(gated_one_api(&Some((200, r#"{"success":true}"#.into())), 0).is_none());
        // success 为真 = 不是错误信封。
        assert!(gated_one_api(&Some((403, r#"{"success":true,"message":"x"}"#.into())), 0).is_none());
        // 没有 message 的空信封认不出内容，不硬认。
        assert!(gated_one_api(&Some((403, r#"{"success":false}"#.into())), 0).is_none());
        // HTML 兜底页不是 JSON。
        assert!(gated_one_api(&Some((403, "<html>403</html>".into())), 0).is_none());
        assert!(gated_one_api(&None, 0).is_none());
    }

    /// 自研网关靠它自己点名的密钥前缀认牌子。
    #[test]
    fn a_custom_gateway_is_named_by_its_key_prefix() {
        // 线上 api.teamorouter.com 的原样回包。
        let u = (
            401u16,
            r#"{"error":{"message":"Missing auth credential. Provide x-api-key: sk-teamo-* or Authorization: Bearer <token>.","type":"missing_auth_credential","code":401},"trace_id":"x"}"#
                .to_string(),
        );
        let d = custom_gateway(Some(&u), 0).expect("自研网关没认出来");
        assert_eq!(d.family, Family::Custom("teamo".into()));
        assert_eq!(d.family.label(), "自研网关 / teamo");
        // 存库再读回来必须还是同一个牌子，否则两个页面上会是两个家族。
        assert_eq!(Family::from_label(&d.family.label()), d.family);

        // 不是 JSON 错误信封的一律不认。
        assert!(custom_gateway(Some(&(401, "sk-teamo-abc".into())), 0).is_none());
        assert!(custom_gateway(Some(&(401, r#"{"detail":"nope"}"#.into())), 0).is_none());
        // **合法 JSON、也含 sk-xxx-，但不是错误信封** —— 这一条才真正拦住「随便在
        // 哪个字段里看到密钥前缀就认牌子」。少了它，判据可以退化成全文找 sk-，
        // 而那会在别人的文档页、示例响应上认出一个自信的错牌子。
        assert!(
            custom_gateway(Some(&(200, r#"{"detail":"请用 sk-teamo- 开头的密钥"}"#.into())), 0)
                .is_none(),
            "不是错误信封也认了牌子",
        );
        assert!(custom_gateway(None, 0).is_none());
    }

    /// 抠牌子只认 `sk-<牌子>-` 这种两段的。
    ///
    /// 单段的 `sk-xxxxxxxx` 是 OpenAI 的通用密钥形状，抠出来是别人的随机串 ——
    /// 那会给出一个**自信的错牌子**，比认不出更糟：页面上「靠什么认的」这一列
    /// 存在的全部意义就是让人能判断结论可不可信。
    #[test]
    fn only_a_two_segment_key_prefix_names_a_brand() {
        assert_eq!(key_prefix_brand("Provide x-api-key: sk-teamo-* or ..."), Some("teamo".into()));
        assert_eq!(key_prefix_brand("需要 sk-abc123- 开头"), Some("abc123".into()));
        // 单段：OpenAI 通用形状，不认。
        assert_eq!(key_prefix_brand("your key sk-proj9x8y7z should start"), None);
        assert_eq!(key_prefix_brand("sk-"), None);
        // 没有 sk- 的不认。
        assert_eq!(key_prefix_brand("missing credential"), None);
        // 太长的不是牌子，是随机串。
        assert_eq!(key_prefix_brand("sk-aaaaaaaaaaaaaaaaaaaaaaaaaaaa-x"), None);
    }

    /// 认不出的时候，`Unknown` 那句话必须说清楚**试过什么**。
    ///
    /// 这一列不是装饰：认错比认不出更糟，因为认错会拉到一份别家的价目还很自信。
    /// 说清试过什么，才能判断结论可不可信。
    #[test]
    fn the_detection_ladder_does_not_give_up_early() {
        let me = include_str!("relay_adapter.rs");
        // 三条新指纹必须真的挂在阶梯上 —— 写了函数没接进去等于没写。
        for shape in [
            "if let Some(d) = gated_one_api(&status, now) {",
            "if let Some(d) = custom_gateway(usage.as_ref(), now) {",
            "if let Some(d) = openai_compatible(&http, &root, now).await {",
        ] {
            assert!(me.contains(shape), "新指纹没接进阶梯（缺 `{shape}`）");
        }
        // 老那句「/api/status 也拿不到 —— 这家的价目要手工录」不能再是**第一时间**
        // 就回的：它现在只该出现在三条都没中之后。
        let at = me.find("if qpu.is_none() {").expect("阶梯的分叉点不见了");
        let tail = &me[at..];
        let give_up = tail.find("return unknown(").expect("兜底那句不见了");
        for shape in ["gated_one_api(", "custom_gateway(", "openai_compatible("] {
            let pos = tail.find(shape).unwrap_or(usize::MAX);
            assert!(pos < give_up, "`{shape}` 排在了放弃之后 —— 它永远不会被走到");
        }
    }

    /// 400 要带日期区间重试一次。
    ///
    /// OpenAI 那套 `/v1/usage` 要 start_date/end_date，抄这条路由的网关多半也要。
    /// 线上 teamorouter 就卡在这儿：路由在（无效密钥回 401 不是 404）、真密钥回 400，
    /// 而它是这家**唯一**的账户接口（/v1/balance、/v1/credits、/v1/me 全是 404）。
    /// 不重试就等于这家的余额永远读不到，而余额读不到就没法标定进价。
    #[test]
    fn a_400_is_retried_with_a_date_range() {
        let all = include_str!("relay_adapter.rs");
        let me = &all[..all.find("\n#[cfg(test)]").unwrap_or(all.len())];
        assert!(
            me.contains("if resp.0 == 400 {"),
            "400 没有带日期区间重试 —— 这家的余额会永远读不到",
        );
        assert!(
            me.contains("/v1/usage?start_date={from}&end_date={today}"),
            "重试时没带 OpenAI 那套日期参数",
        );
        // OpenAI 的 /v1/usage 回的是 total_usage，得认。
        assert!(
            me.contains("\"total_usage\""),
            "不认 total_usage —— 抄 OpenAI 那条路由的网关回的就是它",
        );
    }

    /// 单位认不出来就宁可不要。
    ///
    /// 各家 `/v1/models` 里价目字段的名字五花八门，而**单位**从名字分不出来：
    /// 有的每 token、有的每百万 token。猜错是差一百万倍、却看起来完全正常的数字 ——
    /// 比没有价目糟得多，因为对账会一脸自信地报出来。
    #[test]
    fn a_price_whose_unit_is_unclear_is_dropped() {
        let per_tok = serde_json::json!({"prompt": "0.000005", "completion": "0.000025"});
        assert_eq!(per_token(&per_tok, &["prompt", "input"]), Some(0.000005));
        assert_eq!(per_token(&per_tok, &["completion", "output"]), Some(0.000025));
        // 数字型也收（不是所有站都回字符串）。
        assert_eq!(per_token(&serde_json::json!({"input": 1.5e-6}), &["prompt", "input"]), Some(1.5e-6));
        // 别名：没有 prompt 就看 input。
        assert_eq!(per_token(&serde_json::json!({"input": "0.000003"}), &["prompt", "input"]), Some(0.000003));
        // 免费模型是 0，合法。
        assert_eq!(per_token(&serde_json::json!({"prompt": "0"}), &["prompt"]), Some(0.0));

        // **每百万 token 的数字被挡掉**，不会被当成天价每 token 悄悄记进去。
        assert_eq!(per_token(&serde_json::json!({"prompt": 5.0}), &["prompt"]), None);
        assert_eq!(per_token(&serde_json::json!({"prompt": "15"}), &["prompt"]), None);
        // 负数、非数、缺字段。
        assert_eq!(per_token(&serde_json::json!({"prompt": -1.0}), &["prompt"]), None);
        assert_eq!(per_token(&serde_json::json!({"prompt": "免费"}), &["prompt"]), None);
        assert_eq!(per_token(&serde_json::json!({"other": 1.0}), &["prompt"]), None);

        // 第一个别名存在但单位不对时**不许退到第二个别名** —— 那是拿另一个字段的数
        // 配上这个字段的语义，两个都错还互相掩护。
        assert_eq!(
            per_token(&serde_json::json!({"prompt": 5.0, "input": 0.000005}), &["prompt", "input"]),
            None,
            "第一个别名单位不对时退到了第二个",
        );
    }

    /// 通用取价必须真的挂在分派里，而且**每一家**都走得到。
    ///
    /// 这条路对自研网关是唯一的路（每家自己写的面板没有共同接口），
    /// 对面板被关掉或被人机校验挡住的站是退路。写了函数没接进去等于没写。
    #[test]
    fn every_family_falls_back_to_the_generic_price_probe() {
        let me = include_str!("relay_adapter.rs");
        let at = me.find("pub async fn fetch_pricing(").expect("分派函数不见了");
        let body = &me[at..at + me[at..].find("\n}\n").expect("函数没结尾")];
        // 退路在专用接口之后无条件走一遍，不是挂在某个 family 分支下面。
        assert!(
            body.contains("if !by_family.is_empty() {\n        return (by_family, String::new());\n    }")
                && body.contains("openai_models_pricing(&http, &root, api_key).await"),
            "通用取价没有作为兜底无条件走一遍 —— 自研网关会一直没有价",
        );
        // one-api 系也要试一次 /api/pricing，不能整族放弃。
        // 钉的是**这条 arm 真的去取价**，不只是 arm 头存在：只匹配 arm 头的话，
        // 把它的实现换成 `Vec::new()` 测试照样绿，而那一族就再也拿不到价了。
        assert!(
            body.contains(
                "Family::NewApi | Family::OneApi | Family::OneApiFork(_) => {\n            \
                 new_api_pricing(&http, &root, det.quota_per_unit.unwrap_or(500_000.0), console_token)"
            ),
            "one-api 系没真去问 /api/pricing —— 那一族的分支多数是 new-api 下游",
        );
        // 密钥必须真的传进去，否则 /v1/models 一律 401。
        assert!(
            me.contains("get(http, &format!(\"{root}/v1/models\"), Some(api_key)).await"),
            "问 /v1/models 时没带调用密钥 —— 拿到的只会是 401",
        );
    }

    /// pricing 字段全是 0 的站不许收。
    ///
    /// 「字段占位但没填」在中转里很常见。照收就是把成本记成零，而那正是
    /// 「亏损显示成盈利」的那个方向 —— 单向偏差，比缺数据危险。
    #[test]
    fn a_zero_output_price_is_not_a_price() {
        let me = include_str!("relay_adapter.rs");
        assert!(
            me.contains("if !(output > 0.0) {\n            continue;\n        }"),
            "输出价为 0 的模型被收进来了 —— 成本会被记成零",
        );
    }
}
