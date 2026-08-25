//! 多路由：一条线路挂多个上游出口，按进价从便宜到贵用，坏的自动排后面。
//!
//! # 它和 models 那张表的分工
//!
//! `models` 里一行是**一条线路**：一个身份。用户在 IDE 里看到的名字、开放哪些模型、
//! 按什么价扣钱、用量算到谁头上，全在那一行。
//!
//! 这张表一行是**一个出口**：往哪个地址发、用哪个密钥、我进价几折。仅此而已。
//!
//! 这么切不是为了整洁，是因为计费读的是**真正答复的那一行**（`models.rs` 里
//! `match (success, selected_conn)`）。要是多个上游各占一行 `models`，价格字段就各有
//! 一份，同一个模型用户被扣多少钱要看当时哪家转卖商先答；运维每加一个上游，就多一次
//! 悄悄按另一个价计费的机会。出口换来换去换不动账单，靠的是账单字段根本不在这张表里，
//! 而不是靠运维记得把几行价格填成一样。
//!
//! # 为什么排序是「进价升序」而不是让运维排
//!
//! 线路之间的次序（`models.sort`）是运维的意图：它决定用户看到哪个名字、按哪个价。
//! 但同一条线路下的几个出口对用户是**完全等价**的 —— 同样的模型、同样的账单，
//! 只有我的进价不同。既然等价，就没有任何理由让人手排：便宜的先用是唯一正确答案。
//!
//! 折扣（0.3 = 三折）而不是绝对价：转卖商就是这么报价的，而且对全部模型同时成立，
//! 一个数就够。它只进排序，不进账单。
//!
//! # 「自动测」为什么是发一次真请求
//!
//! `health.rs` 那个探针的教训就在隔壁：它对 `base_url` 发一个不带凭据的 GET，任何回应
//! 都算健康 —— 于是十条线路共用一个域名时，它把同一次 TCP 握手做了十遍、全绿，而一条
//! 连续 44 小时零成功的线路从头到尾报 `ok=t 1ms`。
//!
//! 一个出口会坏在四个地方：域名没了、密钥不对、这家没有这个模型、能连但不出货。
//! 只有前一个能靠握手看出来。所以这里发一次**真的**对话请求（`max_tokens` 取 1），
//! 看它是不是回了一个形状对的响应。烧的 token 是个位数，但它是唯一能同时验到那四件事
//! 的办法。
//!
//! # 探测失败为什么只是排后面，不是停用
//!
//! 一次探测是一个样本，不是判决 —— 上游抖一下、我这边网络抖一下，都会得到失败。
//! 拿一个样本去停用出口，就是把「可能还能用」变成「肯定不能用」；而如果所有出口
//! 恰好在同一分钟各抖了一次，整条线路就没有出口可用了。所以探测结果只改次序：
//! 没问题的排前面，有问题的留在后面兜底。真正确定坏了的（密钥被拒）由
//! `models.rs` 里已有的 `mark_route_cooldown_auth` 冷却掉，那是**执行事实**，不是探测。
//!
//! # 一个必须知道的上限
//!
//! `CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED = 2`：一个请求最多换两个出口就收手（再多
//! 客户端就等不起了）。所以挂十个上游不等于有十次机会 —— **只有排在最前面的两个真的
//! 会被用到**。这正是排序必须同时看进价和健康的原因：排序不对，多挂的那八个就只是
//! 躺在库里。

use std::collections::HashMap;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::models::Model;
use crate::AppState;

/// 探测的死线。比 chat 的表头预算短得多：探测是运维在后台等一个结果，不是用户在等回答，
/// 而一个要 20 秒才回表头的出口，本来就不该排在前面。
const PROBE_TIMEOUT_SECS: u64 = 20;

/// 后台自动重测的间隔。
///
/// 取 15 分钟而不是 1 分钟：每次探测烧真 token，而出口的状态不会分钟级变化。
/// 真正的实时判据是 `route_health` 那套（真实流量的结局），探测只负责覆盖
/// 「这个出口今天还没人用过，我不知道它行不行」。
const PROBE_EVERY_SECS: u64 = 15 * 60;

const MAX_LABEL: usize = 60;
const MAX_NOTE: usize = 200;
const MAX_URL: usize = 400;

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

#[derive(sqlx::FromRow, Clone)]
pub struct Endpoint {
    pub id: uuid::Uuid,
    pub route_id: uuid::Uuid,
    pub label: String,
    pub base_url: String,
    pub api_key: String,
    pub cost_ratio: f64,
    pub active: bool,
    pub note: String,
    /// 这个出口实际有哪些模型。空 = 线路的全部。
    #[sqlx(default)]
    pub enabled_models: Vec<String>,
    /// 这个出口说什么协议。空 = 跟线路一样。
    #[sqlx(default)]
    pub protocol: String,
    /// 这个出口能扛多少（相对值，同线路下用同一把尺）。None = 没填。
    /// 只在「首选被限流、要挑替补」时参与权重，平时一点作用都没有。
    #[sqlx(default)]
    pub capacity: Option<f64>,
    pub probe_ok: Option<bool>,
    pub probe_at: Option<chrono::DateTime<chrono::Utc>>,
    pub probe_ms: Option<i32>,
    pub probe_note: String,
}

/// 出口在候选池里的排序键。小的先用。
///
/// 两级：**先看探测结论，再看进价**。反过来（先便宜）会让一个已知打不通的便宜出口
/// 稳定占掉两个尝试位里的一个 —— 每个请求都先去撞它一次，用户每次都多等一个来回。
/// 便宜是省钱，能用是前提。
///
/// `None`（还没测过）排在「测过并且成功」之后、「测过并且失败」之前：没有证据不等于
/// 坏，但也不该越过有证据能用的那个。这和 `route_health` 里「绝不因为没有证据就报绿」
/// 是同一条规矩。
pub fn order_key(probe_ok: Option<bool>, cost_ratio: f64) -> (u8, f64) {
    let tier = match probe_ok {
        Some(true) => 0,
        None => 1,
        Some(false) => 2,
    };
    (tier, cost_ratio)
}

/// 线路自带地址的排序键。
///
/// 它不能走 `order_key(None, 1.0)`。那样它会永远停在「还没测过」那一档 —— 这张表里
/// 没有它的行，探测结论无处可存，所以它**结构上**升不到第 0 档。后果是：加一个原价的
/// 备用中转，只要它测通，就会把直连整个顶掉 —— 同样的价钱，凭空多一跳、多一个第三方。
///
/// 直连是**在任的那个**，不是一个待评估的候选：今天所有流量都从它走。所以它按第 0 档算，
/// 和同价位测通的出口打平，稳定排序让它留在前面；真比它便宜的出口照样越过它。
///
/// 这不违反「没有证据不报绿」—— 那条规矩管的是面板和告警怎么**说**，不是先敲哪扇门。
/// 直连真坏了的时候，冷却、卡顿、连败那套（`route_goes_to_the_back`）会把它往后压，
/// 那走的是执行事实，比任何探测都硬。
pub fn own_order_key() -> (u8, f64) {
    (0, 1.0)
}

/// 这条线路**连同它的出口**一共能提供哪些模型。
///
/// 出口可以带来线路本身没有的模型：你新挂一个中转，它那儿多了两款货，那两款就该出现在
/// IDE 的模型列表里。所以这里是**并集**，不是线路自己那一份。
///
/// 但有一条闸：能不能开放给用户，还要看这个模型**算不算得出价格**（见 `priceable`）。
/// 算不出价格的模型如果开放出去，用户被扣 0、上游照收你的钱 —— 那不是功能，是漏洞。
pub fn effective_models(route: &Model, outlets: &[Endpoint]) -> Vec<String> {
    let mut all = crate::models::allowed_ids(route);
    for e in outlets.iter().filter(|e| e.active) {
        for m in &e.enabled_models {
            if !all.iter().any(|x| x == m) {
                all.push(m.clone());
            }
        }
    }
    all
}

/// 这个模型在这条线路上算不算得出价格。
///
/// 三条来源，任一条有就行：每模型覆盖 → 实时目录 → 线路自己的兜底价。
/// 三条都没有时 `compute_cost` 会算出 0 —— 用户一分不付，而上游照收你的钱。
/// 所以算不出价的模型**不开放**，宁可它不出现在列表里，也不能让它静默地白送。
pub fn priceable(route: &Model, model_id: &str) -> bool {
    let (mi, mo) = crate::models::model_price_override(&route.model_prices, model_id);
    if mi > 0.0 || mo > 0.0 {
        return true;
    }
    if crate::models::official_price(model_id).is_some() {
        return true;
    }
    // 线路兜底价。实测线上这几条都是 0，所以这一支基本等于「没有」，
    // 但配了的话就该认。
    route.input_price > 0.0 || route.output_price > 0.0
}

/// 把「线路」展开成「实际要发请求的出口」。/// 把「线路」展开成「实际要发请求的出口」。
///
/// 每条线路自带的 `base_url` / `api_key` 也算一个出口，而且是**成本 1.0 的那个**：
/// 它是原价直连，运维加的转卖出口只要填了折扣就自动排到它前面。这样「不配任何多路由」
/// 与今天的行为完全一致 —— 一条线路展开成一个出口，顺序不变。
///
/// 展开出来的每一项都是线路本身的克隆，只换了 `base_url`、`api_key`，并记下
/// `endpoint_id`。所以价格、开放模型、协议、计费模式全部原样跟着线路走：
/// **换出口换不动账单**。
pub fn expand(
    routes: &[Model],
    by_route: &HashMap<uuid::Uuid, Vec<Endpoint>>,
    model_id: &str,
) -> Vec<Model> {
    let mut out = Vec::with_capacity(routes.len());
    for r in routes {
        // (排序键, 线路克隆)
        let mut targets: Vec<((u8, f64), Model)> = Vec::new();

        // 线路自带的地址只在**它自己**有这个模型时才算候选。
        //
        // 出口能带来线路本身没有的模型（新挂的中转多了两款货）。那种模型的请求派给
        // 线路自带地址只会撞一个 404 —— 而每个请求只有两次机会，白撞一次就浪费掉一半。
        let own_has = model_id.is_empty()
            || crate::models::allowed_ids(r).iter().any(|x| x == model_id);
        // 线路自带的地址：在任的那个。见 own_order_key —— 同价位它留在前面，
        // 真便宜的出口才越得过它。
        let mut own = r.clone();
        own.endpoint_id = None;
        own.endpoint_label = String::new();
        own.endpoint_cost = Some(1.0);
        if own_has {
            targets.push((own_order_key(), own));
        }

        for e in by_route.get(&r.id).into_iter().flatten() {
            if !e.active || e.base_url.trim().is_empty() {
                continue;
            }
            // 这个出口没有这个模型就别派给它。
            //
            // 转卖商之间的货不一样：同一条 Claude 线路的三个出口，可能只有一个真有 opus-5。
            // 不筛的话，opus-5 的请求会被派到没有它的出口上撞一个 404 —— 而每个请求只有
            // 两次机会（CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED），这一撞就浪费掉一半。
            //
            // 空 = 承载线路的全部模型，也就是不填时和以前完全一样。
            // 空 = 承载**线路自己**开放的那些（不是并集 —— 别的出口带来的货，
            // 这个出口未必有）。非空 = 就这几款。
            let serves = if e.enabled_models.is_empty() {
                crate::models::allowed_ids(r).iter().any(|x| x == model_id)
            } else {
                e.enabled_models.iter().any(|x| x == model_id)
            };
            if !model_id.is_empty() && !serves {
                continue;
            }
            let mut m = r.clone();
            m.base_url = e.base_url.clone();
            // 协议是「这条线怎么说话」，可以和线路不同：官方直连走 Anthropic 原生，
            // 而最便宜的那批转卖往往只提供 OpenAI 兼容。
            if !e.protocol.trim().is_empty() {
                m.protocol = e.protocol.clone();
            }
            // 出口没填密钥就沿用线路的：同一家转卖商换个入口地址是常见配置，
            // 逼人把同一个密钥抄一遍只会抄错。
            if !e.api_key.trim().is_empty() {
                m.api_key = e.api_key.clone();
            }
            m.endpoint_id = Some(e.id);
            m.endpoint_label = e.label.clone();
            m.endpoint_cost = Some(e.cost_ratio);
            m.endpoint_capacity = e.capacity;
            targets.push((order_key(e.probe_ok, e.cost_ratio), m));
        }

        // 稳定排序：进价和探测结论都相同时，保持「线路自带的在前、其余按建立次序」，
        // 免得每次请求随机换一个出口 —— 那会把上游的提示词缓存全部打散。
        targets.sort_by(|a, b| {
            a.0 .0
                .cmp(&b.0 .0)
                .then(a.0 .1.partial_cmp(&b.0 .1).unwrap_or(std::cmp::Ordering::Equal))
        });
        out.extend(targets.into_iter().map(|(_, m)| m));
    }
    out
}

// ---------------------------------------------------------------- 分配

/// 粘性键：同一个用户（同一段对话）稳定地映射到同一个出口。
///
/// # 为什么必须带 uid，而且带盐
///
/// 这个键只在**溢出**时用（首选出口被限流了，得挑一个替补）。它要满足两件事：
/// 同一段对话每次挑到同一个替补（否则每换一次出口，上游那份提示词缓存就全部重来，
/// 而那笔钱是**用户**在付 —— `effective_cache_prices` 把缓存折扣直接算进了用户账单）；
/// 不同用户挑到不同替补（否则替补立刻变成下一个热点）。
///
/// 阶梯：会话 id → run id → 只有 uid。**每一级都掺 uid**，因为 run id 是客户端给的，
/// 不掺的话两个用户可以撞同一个键、被钉在同一个出口上。
///
/// 掺服务端盐：不掺的话，任何持 API key 的人都能离线枚举 run id，把自己钉在最便宜的
/// 出口上。收益不大但成本更小，掺上。
///
/// **不复用 `openai_prompt_cache_key`**：那个函数在没有 run id 时退回「模型 + 首条 system」，
/// 键里根本没有 uid —— 拿它做分配会把所有用户的同类请求钉在同一个出口上，
/// 正好是这里要避免的事。
pub fn sticky_key(uid: &uuid::Uuid, scopes: &[Option<&str>], secret: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(secret);
    h.update([0u8]);
    h.update(uid.as_bytes());
    // 第一个通过归一化的 scope 胜出；一个都没有就只用 uid。
    for (level, scope) in scopes.iter().enumerate() {
        if let Some(v) = scope.and_then(|v| normalise_scope(v)) {
            h.update([0u8, level as u8 + 1]);
            h.update(v.as_bytes());
            break;
        }
    }
    h.finalize().into()
}

/// 客户端给的 scope 得先洗一遍。
///
/// 客户端那道白名单不合法时是**静默不发**，所以网关不能假设收到的一定合法 ——
/// 一个带空格或超长的值混进哈希，效果等同于「这个用户每次都换一个键」，粘性直接失效。
fn normalise_scope(v: &str) -> Option<&str> {
    let t = v.trim();
    let ok = (8..=128).contains(&t.len())
        && t.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    ok.then_some(t)
}

/// 溢出时每个出口该分到多少。**只看进价，不看健康。**
///
/// 健康信号（探测结论、连败、冷却、卡顿）一律不进权重，全部留在选完之后那道重排里。
/// 理由是那些信号都是**阶跃**的：探测是单样本 0/1、`classify` 是四值词、进程内的记号
/// 在发版后全空。把阶跃量折进连续权重，一次抖动就会让过半在途对话集体迁走，
/// 而这正是粘性要防的事。排除坏出口是排除，不是降权。
///
/// γ 取 2 而不是更大：溢出集合里最便宜那个已经被摘掉了，剩下的价差本来就小；
/// γ 太大会退化成「第二便宜的吃全部」，也就是把雪崩推到下一跳。
pub fn overflow_weight(cost_ratio: f64, capacity: f64) -> f64 {
    const GAMMA: f64 = 2.0;
    const MIN: f64 = 1e-6;
    if !cost_ratio.is_finite() || cost_ratio <= 0.0 || !capacity.is_finite() || capacity <= 0.0 {
        return MIN;
    }
    let w = capacity * (1.0 / cost_ratio).powf(GAMMA);
    if w.is_finite() && w > MIN {
        w
    } else {
        MIN
    }
}

/// 把一池出口的容量补齐：没填的按**已填里的最小值**算，全没填就一律 1。
///
/// 不补的话会出一个很难查的错：一个填了 600（RPM）、一个没填按 1 算，
/// 后者拿到的溢出是前者的六百分之一 —— 运维只是"没填"，却等于把那个出口关掉了。
///
/// 补成最小值是保守方向：不知道能扛多少，就当它是最不能扛的那个。反过来（补成最大）
/// 会让一个没人填过的出口吃掉全部溢出，而那正是容量这一列想避免的事。
pub fn fill_capacities(declared: &[Option<f64>]) -> Vec<f64> {
    let floor = declared
        .iter()
        .filter_map(|c| c.filter(|v| v.is_finite() && *v > 0.0))
        .fold(f64::INFINITY, f64::min);
    let floor = if floor.is_finite() { floor } else { 1.0 };
    declared
        .iter()
        .map(|c| match c {
            Some(v) if v.is_finite() && *v > 0.0 => *v,
            _ => floor,
        })
        .collect()
}

/// 加权 rendezvous：在幸存出口里稳定地挑一个，命中概率正比于权重。
///
/// 用 `w / -ln(u)` 这个形式（u 是 (0,1) 上的均匀量），它的最大值恰好以 wᵢ/Σw 的概率
/// 落在第 i 个 —— 这是加权 rendezvous 的标准构造。
///
/// 它比「按权重划分区间」多一条要紧的性质：**集合变化时扰动最小**。移走一个出口，
/// 只有原本落在它上面的那些对话会重新分配，其余一个都不动 —— 而按区间划分会让所有人
/// 集体平移，也就是所有人的缓存同时作废。
///
/// 哈希必须是 SHA-256，不能用 `DefaultHasher`：Rust 保留跨版本换算法的权利，
/// 换一次全网粘性静默清零，而且不报错。
pub fn hrw_pick(key: &[u8; 32], set: &[(uuid::Uuid, f64, f64)]) -> Option<usize> {
    use sha2::{Digest, Sha256};
    let mut best: Option<(f64, uuid::Uuid, usize)> = None;
    for (i, (id, cost, cap)) in set.iter().enumerate() {
        let mut h = Sha256::new();
        h.update(key);
        h.update(id.as_bytes());
        let d: [u8; 32] = h.finalize().into();
        // 取高 53 位映射到 (0,1)：+0.5 保证严格大于 0，-ln(u) 因而不会是 inf。
        let bits = u64::from_be_bytes(d[..8].try_into().unwrap()) >> 11;
        let u = (bits as f64 + 0.5) / (1u64 << 53) as f64;
        let score = overflow_weight(*cost, *cap) / -u.ln();
        // 分数相同时按 uuid 定，保证同一份输入永远得到同一个答案。
        let better = match best {
            None => true,
            Some((bs, bid, _)) => score > bs || (score == bs && *id > bid),
        };
        if better && score.is_finite() {
            best = Some((score, *id, i));
        }
    }
    // 全部非有限（理论上到不了，overflow_weight 有下限）时退回第一个，绝不返回空。
    best.map(|(_, _, i)| i).or(if set.is_empty() { None } else { Some(0) })
}

/// 取一批线路的出口，按 `route_id` 分好。
pub async fn load_for_routes(
    db: &sqlx::PgPool,
    route_ids: &[uuid::Uuid],
) -> HashMap<uuid::Uuid, Vec<Endpoint>> {
    if route_ids.is_empty() {
        return HashMap::new();
    }
    let rows: Vec<Endpoint> = sqlx::query_as(
        "SELECT * FROM route_endpoints WHERE route_id = ANY($1) AND active = true \
         ORDER BY cost_ratio, created_at",
    )
    .bind(route_ids)
    .fetch_all(db)
    .await
    .unwrap_or_else(|e| {
        // 取不到出口不该让请求失败：退回「只用线路自带的地址」就是今天的行为。
        // 多路由是加成，不是依赖。
        tracing::warn!(error = %e, "route_endpoints 读取失败，本轮只用线路自带地址");
        Vec::new()
    });
    let mut map: HashMap<uuid::Uuid, Vec<Endpoint>> = HashMap::new();
    for r in rows {
        map.entry(r.route_id).or_default().push(r);
    }
    map
}

// ---------------------------------------------------------------- 观测

/// 记一次出口用量。**火后不管**：丢几条对看板毫无影响，而阻塞一次真实回答的代价是实打实的。
///
/// 归属用 `health_id`（出口，或线路自带地址），和计费的 `model_id`（线路）刻意分开 ——
/// 计费归属换不得，那会让用量静默记成 NULL。
pub fn note_endpoint_usage(
    state: &AppState,
    endpoint_id: uuid::Uuid,
    route_id: uuid::Uuid,
    cost_cents: i64,
    // 直接收三个数，不收计费那边的内部类型 —— 观测不该有能力碰到计费的结构。
    prompt: i64,
    completion: i64,
    cached: i64,
) {
    let db = state.db.clone();
    // 分转微美元。看板要看得见「三分钱」这种量级，按分存会全是 0。
    let micro = cost_cents.max(0).saturating_mul(10_000);
    tokio::spawn(async move {
        let _ = sqlx::query(
            "INSERT INTO endpoint_usage \
               (day, endpoint_id, route_id, calls, cost_micro_usd, \
                prompt_tokens, completion_tokens, cached_tokens) \
             VALUES (current_date, $1, $2, 1, $3, $4, $5, $6) \
             ON CONFLICT (day, endpoint_id) DO UPDATE SET \
               calls = endpoint_usage.calls + 1, \
               cost_micro_usd = endpoint_usage.cost_micro_usd + EXCLUDED.cost_micro_usd, \
               prompt_tokens = endpoint_usage.prompt_tokens + EXCLUDED.prompt_tokens, \
               completion_tokens = endpoint_usage.completion_tokens + EXCLUDED.completion_tokens, \
               cached_tokens = endpoint_usage.cached_tokens + EXCLUDED.cached_tokens, \
               updated_at = now()",
        )
        .bind(endpoint_id)
        .bind(route_id)
        .bind(micro)
        .bind(prompt)
        .bind(completion)
        .bind(cached)
        .execute(&db)
        .await;
    });
}

/// 问一个中转「我还剩多少额度」。
///
/// # 没有标准，所以是尽力而为
///
/// 各家中转的余额接口互不相同，也没有任何一个标准。这里按三种线上最常见的形态各试一次：
///   · One API / New API 那一族（国内转卖用得最多）：`/api/user/self` → `quota`/`used_quota`
///   · OpenRouter：`/api/v1/auth/key` → `limit_remaining`
///   · OpenAI 官方那套：`/dashboard/billing/subscription`
///
/// **查不到就明确回「查不到」，绝不猜、绝不填 0。** 一个显示成 0 的余额会让人以为
/// 没钱了去充值，而实际可能只是这家没有这个接口 —— 报错的信息量为零，误导的代价却是真的。
/// 一次余额读数。
///
/// `text` 是给人看的，`remaining_usd` / `used_usd` 是给对账算的。两者必须同源 ——
/// 面板显示一个数、成本按另一个数算，是最难发现的一类错。
#[derive(Clone, Debug)]
pub struct BalanceReading {
    pub text: String,
    /// 还剩多少美元。None = 这家只给了「已用」或只给了上限。
    pub remaining_usd: Option<f64>,
    /// 累计已用多少美元。None = 这家不给。
    ///
    /// **算成本时优先用它**：余额会被充值打断（充一次就变成负成本），
    /// 而「已用」是单调递增的，充值不影响。
    pub used_usd: Option<f64>,
}

/// 给面板用的那层：只要展示串。
pub(crate) async fn query_balance(
    http: &reqwest::Client,
    base_url: &str,
    key: &str,
) -> Option<String> {
    read_balance(http, base_url, key).await.map(|b| b.text)
}

pub(crate) async fn read_balance(
    http: &reqwest::Client,
    base_url: &str,
    key: &str,
) -> Option<BalanceReading> {
    let root = base_url.trim_end_matches('/').trim_end_matches("/v1").to_string();
    let bearer = format!("Bearer {key}");

    // 形态一：One API / New API
    if let Ok(r) = http
        .get(format!("{root}/api/user/self"))
        .header("authorization", &bearer)
        .send()
        .await
    {
        if r.status().is_success() {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                let d = v.get("data").unwrap_or(&v);
                if let Some(q) = d.get("quota").and_then(|x| x.as_f64()) {
                    let used = d.get("used_quota").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    // 这一族的 quota 是「点」，惯例 500000 点 = 1 美元。
                    let left = q / 500_000.0;
                    let spent = used / 500_000.0;
                    return Some(BalanceReading {
                        text: format!("${left:.2}（已用 ${spent:.2}）"),
                        remaining_usd: Some(left),
                        used_usd: Some(spent),
                    });
                }
            }
        }
    }

    // 形态二：OpenRouter
    if let Ok(r) = http
        .get(format!("{root}/api/v1/auth/key"))
        .header("authorization", &bearer)
        .send()
        .await
    {
        if r.status().is_success() {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                let d = v.get("data").unwrap_or(&v);
                let used = d.get("usage").and_then(|x| x.as_f64());
                let left = d.get("limit_remaining").and_then(|x| x.as_f64());
                match (left, used) {
                    (Some(l), u) => {
                        return Some(BalanceReading {
                            text: format!("${l:.2}"),
                            remaining_usd: Some(l),
                            used_usd: u,
                        })
                    }
                    (None, Some(u)) => {
                        return Some(BalanceReading {
                            text: format!("已用 ${u:.2}（未设上限）"),
                            remaining_usd: None,
                            used_usd: Some(u),
                        })
                    }
                    _ => {}
                }
            }
        }
    }

    // 形态三：OpenAI 官方
    if let Ok(r) = http
        .get(format!("{root}/dashboard/billing/subscription"))
        .header("authorization", &bearer)
        .send()
        .await
    {
        if r.status().is_success() {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                if let Some(x) = v.get("hard_limit_usd").and_then(|x| x.as_f64()) {
                    // 上限**不是**余额，也不是已用。拿它去算成本会算出一个纯属虚构的数字，
                    // 所以这里只给展示串，两个数值都留空。
                    return Some(BalanceReading {
                        text: format!("上限 ${x:.2}"),
                        remaining_usd: None,
                        used_usd: None,
                    });
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------- 调度

/// 下架的持久化前缀。和让位分开存：两者的恢复方式完全不同 ——
/// 让位是**到点自己回来**（时长由上游给），下架是**试通了才回来**（时长不知道）。
const DELIST_KEY_PREFIX: &str = "rh:delist:";
/// 下架状态在 Redis 里最多留多久。比最长退避（1 小时）长一截，
/// 但不能无限 —— 一个被删掉的出口不该在库里留一辈子。
const DELIST_TTL_SECS: i64 = 6 * 3600;

/// 调度器多久扫一轮。
///
/// 30 秒：最短的退避是 60 秒，扫得比它快一档就够，再快只是空转。
/// 它不发请求，只是看一眼有没有到点的 —— 到点了才去探。
const SCHEDULER_TICK: Duration = Duration::from_secs(30);

/// 把下架落一份到 Redis，发版后能承接。火后不管。
pub fn persist_delisting(state: &AppState, id: uuid::Uuid, why: crate::models::Delisted) {
    let mut conn = state.redis.clone();
    let word = why.word().to_string();
    tokio::spawn(async move {
        let key = format!("{DELIST_KEY_PREFIX}{id}");
        let _: Result<(), _> = redis::cmd("SET")
            .arg(&key)
            .arg(&word)
            .arg("EX")
            .arg(DELIST_TTL_SECS)
            .query_async(&mut conn)
            .await;
    });
}

async fn forget_delisting(state: &AppState, id: uuid::Uuid) {
    let mut conn = state.redis.clone();
    let _: Result<(), _> = redis::cmd("DEL")
        .arg(format!("{DELIST_KEY_PREFIX}{id}"))
        .query_async(&mut conn)
        .await;
}

/// 启动时承接上一个进程的下架名单。
///
/// 不承接的话，发版后第一批请求会把流量铺回一个明知道没额度的出口，
/// 每个都白烧一个来回 —— 而蓝绿切换那几秒正好是流量最集中的时候。
pub async fn restore_delisting(state: &AppState) {
    let mut conn = state.redis.clone();
    let mut cursor: u64 = 0;
    let mut n = 0usize;
    loop {
        let res: Result<(u64, Vec<String>), _> = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(format!("{DELIST_KEY_PREFIX}*"))
            .arg("COUNT")
            .arg(200)
            .query_async(&mut conn)
            .await;
        let (next, keys) = match res {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "下架名单没读回来，本进程从空名单开始");
                return;
            }
        };
        for key in keys {
            let word: Option<String> = redis::cmd("GET").arg(&key).query_async(&mut conn).await.ok();
            let Some(id) = key
                .strip_prefix(DELIST_KEY_PREFIX)
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
            else {
                continue;
            };
            let why = match word.as_deref() {
                Some("auth") => crate::models::Delisted::AuthRejected,
                Some("no_quota") => crate::models::Delisted::OutOfQuota,
                _ => continue,
            };
            crate::models::delist_endpoint(id, why);
            n += 1;
        }
        cursor = next;
        if cursor == 0 {
            break;
        }
    }
    if n > 0 {
        tracing::info!(delisted = n, "下架名单已从上一个进程承接");
    }
}

/// 调度器：什么时候该动什么。
///
/// # 它只管一件事：把下架的出口试回来
///
/// 别的状态都有自己的到期机制，不需要人管：
///   · **让位**（429）—— 上游在 Retry-After 里说了多久，到点自己回来；
///   · **冷却**（502/503/504）—— 20 秒后自然过期；
///   · **卡死** —— 120 秒记号 + 已有的 `spawn_stall_recovery` 探针，通了自己撤记号。
///
/// 只有**下架**不一样：没额度、密钥被拒，都不知道什么时候好，时间到了也不会自己好。
/// 所以只有这一种需要「定期去敲门」，也就是这个调度器存在的全部理由。
///
/// # 为什么用真请求去试，而不是等下一个用户去撞
///
/// 等用户撞的代价是：恢复判定由用户的请求付费（他多等一个来回），而且流量越少
/// 恢复越慢 —— 一个半夜充了钱的出口可能到早上才被发现能用。
/// 主动探一次只花个位数 token。
pub fn spawn_scheduler(state: AppState) {
    tokio::spawn(async move {
        // 起步先让服务起完；也避开部署瞬间那段状态刚承接完的窗口。
        tokio::time::sleep(Duration::from_secs(45)).await;
        let mut tick = tokio::time::interval(SCHEDULER_TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            if let Err(e) = sweep_delisted(&state).await {
                tracing::warn!(error = %e, "下架恢复这一轮没跑完");
            }
        }
    });
}

async fn sweep_delisted(state: &AppState) -> anyhow::Result<()> {
    let due = crate::models::delisted_due(std::time::Instant::now());
    if due.is_empty() {
        return Ok(());
    }
    // 出口和线路一次性取回来，别在循环里逐个查库。
    let eps: Vec<Endpoint> = sqlx::query_as("SELECT * FROM route_endpoints")
        .fetch_all(&state.db)
        .await?;
    let routes: Vec<Model> = sqlx::query_as("SELECT * FROM models")
        .fetch_all(&state.db)
        .await?;
    let by_route: HashMap<uuid::Uuid, &Model> = routes.iter().map(|m| (m.id, m)).collect();
    let by_ep: HashMap<uuid::Uuid, &Endpoint> = eps.iter().map(|e| (e.id, e)).collect();

    for (id, why) in due {
        // id 可能是一个出口，也可能是线路自带的地址（health_id 两者共用一个命名空间）。
        let (route, base, key_raw, proto, only) = if let Some(e) = by_ep.get(&id) {
            let Some(r) = by_route.get(&e.route_id) else {
                // 线路没了 → 这条下架记录也没意义了。
                crate::models::relist_endpoint(id);
                forget_delisting(state, id).await;
                continue;
            };
            let k = if e.api_key.trim().is_empty() { &r.api_key } else { &e.api_key };
            (*r, e.base_url.clone(), k.clone(), e.protocol.clone(), e.enabled_models.clone())
        } else if let Some(r) = by_route.get(&id) {
            (*r, r.base_url.clone(), r.api_key.clone(), String::new(), Vec::new())
        } else {
            crate::models::relist_endpoint(id);
            forget_delisting(state, id).await;
            continue;
        };

        let out = probe_once(
            &probe_client(),
            route,
            &base,
            &crate::models::model_key(&key_raw),
            &proto,
            &only,
        )
        .await;
        if out.ok {
            crate::models::relist_endpoint(id);
            forget_delisting(state, id).await;
            tracing::info!(
                endpoint = %id,
                why = why.word(),
                ms = out.ms,
                "下架的出口试通了，已恢复"
            );
        } else {
            crate::models::defer_relist(id);
            tracing::info!(
                endpoint = %id,
                why = why.word(),
                note = %out.note,
                "下架的出口还是不通，退避加长"
            );
        }
        // 别把一堆探测同时打到同一家转卖商头上。
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    Ok(())
}

/// 让位状态的跨进程承接。
///
/// # 为什么要落 Redis，又为什么**不在派单路径读**
///
/// 让位状态活在进程内（一次哈希、一把短锁，派单路径上零 I/O，这是刻意的）。代价是
/// 发版后新进程那张表是空的 —— 它会把流量直接铺回一个可能还在限流窗口里的出口，
/// 撞一次 429 才重新学到。蓝绿切换那几秒正好是流量最集中的时候。
///
/// 所以写一份到 Redis（火后不管，不阻塞请求），**启动时读回来**一次。
/// 读只发生在启动，派单路径一个 await 都没加。
///
/// TTL 就设成让位时长本身：到期键自己没了，不需要任何人去清，也不会有陈旧值。
const SAT_KEY_PREFIX: &str = "rh:sat:";

/// 记一次让位到 Redis。调用方已经在进程内记过了，这里只管持久化，失败就算了 ——
/// 掉一次的后果是「发版后可能多撞一个 429」，不值得为它让用户的请求等。
pub fn persist_saturation(state: &AppState, id: uuid::Uuid, how_long: std::time::Duration) {
    let mut conn = state.redis.clone();
    let secs = how_long.as_secs().max(1) as i64;
    tokio::spawn(async move {
        let key = format!("{SAT_KEY_PREFIX}{id}");
        let _: Result<(), _> = redis::cmd("SET")
            .arg(&key)
            .arg(secs)
            .arg("EX")
            .arg(secs)
            .query_async(&mut conn)
            .await;
    });
}

/// 启动时把还没到期的让位读回进程内。
///
/// 用 SCAN 而不是 KEYS：KEYS 在大库上会阻塞整个 Redis，而这台机器上 Redis 还扛着
/// 会话和健康数据。这里的键最多几十个，SCAN 一轮就完。
pub async fn restore_saturation(state: &AppState) {
    let mut conn = state.redis.clone();
    let mut cursor: u64 = 0;
    let mut restored = 0usize;
    loop {
        let res: Result<(u64, Vec<String>), _> = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(format!("{SAT_KEY_PREFIX}*"))
            .arg("COUNT")
            .arg(200)
            .query_async(&mut conn)
            .await;
        let (next, keys) = match res {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "让位状态没读回来，本进程从空表开始");
                return;
            }
        };
        for key in keys {
            // 剩余 TTL 才是真正还要让位多久 —— 存进去的那个时长早就走掉一截了。
            let ttl: Option<i64> = redis::cmd("TTL").arg(&key).query_async(&mut conn).await.ok();
            let Some(ttl) = ttl.filter(|t| *t > 0) else { continue };
            let Some(id) = key
                .strip_prefix(SAT_KEY_PREFIX)
                .and_then(|s| uuid::Uuid::parse_str(s).ok())
            else {
                continue;
            };
            crate::models::mark_endpoint_saturated(id, std::time::Duration::from_secs(ttl as u64));
            restored += 1;
        }
        cursor = next;
        if cursor == 0 {
            break;
        }
    }
    if restored > 0 {
        tracing::info!(restored, "让位状态已从上一个进程承接");
    }
}

// ---------------------------------------------------------------- 探测

/// 一次探测的结论。
pub struct ProbeOutcome {
    pub ok: bool,
    pub ms: i32,
    pub note: String,
}

/// 对一个出口发一次最小的真实请求。
///
/// `base_url` / `api_key` 由调用方给出（可能来自出口，也可能是线路自带的）。
///
/// `protocol` 空 = 跟线路一样。`only_models` 空 = 线路的全部 —— 探一个只有 sonnet 的
/// 出口时必须拿 sonnet 去探，拿线路的第一个模型（可能是 opus）去探会得到一个 404，
/// 然后把一个好出口判成坏的。
pub async fn probe_once(
    http: &reqwest::Client,
    route: &Model,
    base_url: &str,
    api_key_plain: &str,
    protocol: &str,
    only_models: &[String],
) -> ProbeOutcome {
    let started = std::time::Instant::now();
    let ms = |s: std::time::Instant| s.elapsed().as_millis().min(i32::MAX as u128) as i32;

    let pool: Vec<String> = if only_models.is_empty() {
        crate::models::allowed_ids(route)
    } else {
        only_models.to_vec()
    };
    let Some(model_id) = pool.into_iter().next() else {
        return ProbeOutcome {
            ok: false,
            ms: 0,
            note: "这条线路一个开放模型都没配，没有可探的模型".into(),
        };
    };

    let anthropic = if protocol.is_empty() {
        route.protocol == "anthropic"
    } else {
        protocol == "anthropic"
    };
    let base = crate::models::api_base(base_url);
    let url = if anthropic {
        format!("{base}/messages")
    } else {
        format!("{base}/chat/completions")
    };
    // max_tokens 取 1：验的是「这条路通不通」，不是它会说什么。
    let body = serde_json::json!({
        "model": model_id,
        "max_tokens": 1,
        "messages": [{ "role": "user", "content": "hi" }],
    });

    let mut req = http.post(&url).json(&body);
    req = if anthropic {
        req.header("x-api-key", api_key_plain)
            .header("anthropic-version", "2023-06-01")
    } else {
        req.header("authorization", format!("Bearer {api_key_plain}"))
    };

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            // 连不上／超时／TLS 坏了。这里**不能**把错误原文塞进 note：reqwest 的错误链
            // 会带上完整 URL，而查询串里可能有人把密钥写在了地址上。
            let why = if e.is_timeout() {
                format!("超过 {PROBE_TIMEOUT_SECS} 秒没有回应")
            } else if e.is_connect() {
                "连不上（域名或端口不对）".to_string()
            } else {
                "请求没发出去".to_string()
            };
            return ProbeOutcome { ok: false, ms: ms(started), note: why };
        }
    };

    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    let elapsed = ms(started);

    if !(200..300).contains(&status) {
        let why = match status {
            401 | 403 => "密钥被拒（401/403）".to_string(),
            404 => format!("这家没有 {model_id}（404）"),
            429 => "被限流（429）".to_string(),
            402 => "余额不足（402）".to_string(),
            500..=599 => format!("上游自己出错（{status}）"),
            _ => format!("上游返回 {status}"),
        };
        return ProbeOutcome { ok: false, ms: elapsed, note: why };
    }

    // 2xx 还不够。转卖网关会用 200 包一个错误体，也会回一个空壳 —— 这正是
    // model_probe.rs 里记着的那条教训：拿「没报错」当「能用」会得出荒唐的结论。
    // 所以要求响应里确实有生成内容的那几个字段之一。
    let looks_real = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .is_some_and(|v| {
            v.get("content").is_some_and(|c| c.is_array())
                || v.get("choices").is_some_and(|c| c.is_array())
                || v.get("usage").is_some()
        });
    if !looks_real {
        return ProbeOutcome {
            ok: false,
            ms: elapsed,
            note: "回了 200 但不是对话响应（可能是转卖网关的错误页）".into(),
        };
    }
    ProbeOutcome { ok: true, ms: elapsed, note: String::new() }
}

fn probe_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
        .unwrap_or_default()
}

/// 探一个出口并把结论写回库。
async fn probe_and_store(state: &AppState, ep: &Endpoint, route: &Model) -> ProbeOutcome {
    let key = if ep.api_key.trim().is_empty() {
        crate::models::model_key(&route.api_key)
    } else {
        crate::models::model_key(&ep.api_key)
    };
    let out = probe_once(
        &probe_client(),
        route,
        &ep.base_url,
        &key,
        &ep.protocol,
        &ep.enabled_models,
    )
    .await;
    let _ = sqlx::query(
        "UPDATE route_endpoints SET probe_ok = $2, probe_at = now(), probe_ms = $3, \
         probe_note = $4, updated_at = now() WHERE id = $1",
    )
    .bind(ep.id)
    .bind(out.ok)
    .bind(out.ms)
    .bind(&out.note)
    .execute(&state.db)
    .await;
    out
}

/// 后台自动重测。
///
/// 只测「最近没有真实流量证明过」的出口：真实流量的结局比探测更准，也不花钱。
/// 这既省钱，也避免把探测流量算进上游的限流额度。
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        // 启动后先等一会儿：刚起来时迁移、连接池、缓存都在忙，探测不着急。
        tokio::time::sleep(std::time::Duration::from_secs(90)).await;
        loop {
            if let Err(e) = sweep(&state).await {
                tracing::warn!(error = %e, "多路由自动探测这一轮没跑完");
            }
            tokio::time::sleep(std::time::Duration::from_secs(PROBE_EVERY_SECS)).await;
        }
    });
}

async fn sweep(state: &AppState) -> anyhow::Result<()> {
    let eps: Vec<Endpoint> =
        sqlx::query_as("SELECT * FROM route_endpoints WHERE active = true ORDER BY created_at")
            .fetch_all(&state.db)
            .await?;
    if eps.is_empty() {
        return Ok(());
    }
    let routes: Vec<Model> = sqlx::query_as("SELECT * FROM models WHERE active = true")
        .fetch_all(&state.db)
        .await?;
    let by_id: HashMap<uuid::Uuid, Model> = routes.into_iter().map(|m| (m.id, m)).collect();

    let mut probed = 0usize;
    for ep in eps {
        let Some(route) = by_id.get(&ep.route_id) else {
            continue;
        };
        // 真实流量最近成功过就别浪费 token —— 那是比探测更硬的证据。
        let health = crate::route_health::snapshot(state, ep.id).await;
        let now = chrono::Utc::now().timestamp();
        if crate::route_health::classify(&health, now) == "ok" {
            continue;
        }
        probe_and_store(state, &ep, route).await;
        probed += 1;
        // 别把一堆探测同时打到同一家转卖商头上。
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    if probed > 0 {
        tracing::info!(probed, "多路由自动探测完成");
    }
    Ok(())
}

/// 这条线路是哪家的模型，用来给后台挑一个厂商图标。
///
/// **判据是模型 id，不是 `provider` 列。** 线上实测那一列并不可靠：「免费智普」和「Grok」
/// 的 provider 都填的是 `other`，而它们的模型 id（`glm-5.3` / `grok-4.6`）一眼就能认出来。
/// `protocol` 更不能用 —— 它是传输协议，deepseek、智谱、Grok 三条线路都写着 `openai`。
///
/// 放服务端算而不是让前端猜：判据只有一份，改一次两边都对，而且能测。
/// 认不出来就回空串，前端画一个中性图标 —— 猜错一个厂商比不猜更糟。
///
/// # 次序就是判据
///
/// 这张表**从上往下**匹配，所以排在前面的必须更 specific。两处真实的坑：
///   · `claude` 要在 `bedrock` / `vertex` 前面 —— AWS 上的 id 是
///     `anthropic.claude-3-5-sonnet`，那确实是 Claude，画 Claude 的标才对；
///   · `gpt` 要靠后 —— 一堆转卖商会把别家模型起名成 `xxx-gpt-*`。
///
/// 短词一律不用（`yi`、`nova` 这种），它们会撞进别人的模型名里。宁可漏认一家画中性图标，
/// 也不能给智谱画上 OpenAI 的标。
pub fn vendor_of(provider: &str, models: &[String], base_url: &str) -> &'static str {
    let hay = format!(
        "{} {}",
        provider.to_ascii_lowercase(),
        models.join(" ").to_ascii_lowercase()
    );
    for (needle, vendor) in NEEDLES {
        if hay.contains(needle) {
            return vendor;
        }
    }
    // 模型名认不出来时，再看这条线路指向哪儿。
    //
    // 次序不能反：模型比管道重要。一条指向 openrouter 但跑 claude-opus 的线路该显示
    // Claude —— 运维想知道的是「这条线路卖的是谁家的模型」，不是「它从哪个中间商买的」。
    // 反过来，「牛来」那种自起名字的（stealth/ox-alpha）模型名什么都说明不了，
    // 这时它的地址 openrouter.ai 就是唯一有信息量的东西。
    let host = base_url.to_ascii_lowercase();
    for (needle, vendor) in HOSTS {
        if host.contains(needle) {
            return vendor;
        }
    }
    ""
}

/// (出现在 base_url 里的片段, 厂商)。只在模型名认不出来时才轮到它。
const HOSTS: &[(&str, &str)] = &[
    ("openrouter", "openrouter"),
    ("siliconflow", "siliconcloud"),
    ("siliconcloud", "siliconcloud"),
    ("deepinfra", "deepinfra"),
    ("groq.com", "groq"),
    ("together.", "together"),
    ("fireworks", "fireworks"),
    ("replicate", "replicate"),
    ("huggingface", "huggingface"),
    ("novita", "novita"),
    ("hyperbolic", "hyperbolic"),
    ("cerebras", "cerebras"),
    ("sambanova", "sambanova"),
    ("baseten", "baseten"),
    ("nebius", "nebius"),
    ("featherless", "featherless"),
    ("lepton", "leptonai"),
    ("ppio", "ppio"),
    ("gitee", "giteeai"),
    ("aihubmix", "aihubmix"),
    ("burncloud", "burncloud"),
    ("cometapi", "cometapi"),
    ("302.ai", "ai302"),
    ("poe.com", "poe"),
    ("monica", "monica"),
    ("venice", "venice"),
    ("zenmux", "zenmux"),
    ("sophnet", "sophnet"),
    ("straico", "straico"),
    ("qiniu", "qiniu"),
    ("jina.ai", "jina"),
    ("voyageai", "voyage"),
    ("dashscope", "bailian"),
    ("aliyuncs", "alibabacloud"),
    ("volces", "volcengine"),
    ("volcengine", "volcengine"),
    ("bigmodel", "zhipu"),
    ("moonshot", "moonshot"),
    ("baidubce", "baiducloud"),
    ("tencentcloudapi", "tencentcloud"),
    ("myhuaweicloud", "huaweicloud"),
    ("xf-yun", "iflytekcloud"),
    ("azure", "azure"),
    ("amazonaws", "bedrock"),
    ("googleapis", "googlecloud"),
    ("cloudflare", "cloudflare"),
    ("localhost", "ollama"),
    ("127.0.0.1", "ollama"),
    ("11434", "ollama"),
];

/// (在模型 id 或 provider 里出现的片段, 厂商)。从上往下匹配。
const NEEDLES: &[(&str, &str)] = &[
    ("claude", "anthropic"),
    ("anthropic", "anthropic"),
    ("deepseek", "deepseek"),
    ("glm", "zhipu"),
    ("chatglm", "zhipu"),
    ("zhipu", "zhipu"),
    ("grok", "xai"),
    ("gemini", "google"),
    ("gemma", "google"),
    ("qwen", "qwen"),
    ("qwq", "qwen"),
    ("kimi", "moonshot"),
    ("moonshot", "moonshot"),
    ("llama", "meta"),
    ("mistral", "mistral"),
    ("mixtral", "mistral"),
    ("magistral", "mistral"),
    ("minimax", "minimax"),
    ("abab", "minimax"),
    ("baichuan", "baichuan"),
    ("hunyuan", "hunyuan"),
    ("doubao", "doubao"),
    ("volc", "volcengine"),
    ("ernie", "wenxin"),
    ("wenxin", "wenxin"),
    ("internlm", "internlm"),
    ("sensechat", "sensenova"),
    ("sensenova", "sensenova"),
    ("skywork", "skywork"),
    ("command-r", "cohere"),
    ("cohere", "cohere"),
    ("jamba", "ai21"),
    ("sonar", "perplexity"),
    ("perplexity", "perplexity"),
    ("nemotron", "nvidia"),
    ("nvidia", "nvidia"),
    ("phi-", "microsoft"),
    ("openrouter", "openrouter"),
    ("fireworks", "fireworks"),
    ("groq", "groq"),
    ("together", "together"),
    ("ollama", "ollama"),
    ("bedrock", "bedrock"),
    ("vertex", "vertexai"),
    ("azure", "azure"),
    // 「01.AI / 零一万物」的 id 是 yi-large / yi-lightning 这一族。
    // 只写 "yi" 会撞进一堆别的名字里（例如任何含 "yi" 的拼音品牌），所以逐个列。
    ("yi-large", "zeroone"),
    ("yi-lightning", "zeroone"),
    ("yi-vision", "zeroone"),
    ("yi-medium", "zeroone"),
    ("zeroone", "zeroone"),
    // 讯飞星火。放在最后是因为 "spark" 也可能出现在别的地方（如 sparkdesk 之外的品牌名）。
    ("sparkdesk", "spark"),
    ("spark-", "spark"),
    ("step-", "stepfun"),
    ("stepfun", "stepfun"),
    ("gpt", "openai"),
    ("o3-", "openai"),
    ("o4-", "openai"),
    ("openai", "openai"),
];

/// 「能不能服务」的排序：越小越好。
///
/// 用 `route_health::classify` 那套词（ok / degraded / unknown / error），不新造词 ——
/// 面板和告警都按那几个词分支，多一个词就是一条走不到的分支。
fn serve_rank(word: &str) -> u8 {
    match word {
        "ok" => 0,
        "degraded" => 1,
        // 「不知道」排在 error 前面：没有证据不等于坏。它不会触发告警，
        // 也不会让一条真坏了的线路显示成绿的 —— 两头都不冤枉。
        "unknown" => 2,
        _ => 3,
    }
}

/// 这条线路所有出口里**最好**的那个结论，以及它是谁。
///
/// 加多路由之前，「线路健康」和「那个地址健康」是同一件事。现在不是了：健康按出口记
/// （一个坏出口不该拖垮同线路的好出口），而流量大多走最便宜那个出口 —— 只看线路自带
/// 地址的记录，面板上最忙的线路反而会显示成「不知道」，**告警更是永远看不到出口的连败**。
/// 那正好是这台机器出过的那次事故的形状：面板全绿、监控一次没响、44 小时。
///
/// 取「最好」而不是「最坏」，是因为这两处要回答的都是**用户此刻能不能用**：只要还有一个
/// 出口能服务，请求就会成功，不该报警、也不该标红。全部出口都判坏了才是真的坏了。
///
/// 返回的第二个值是判据来自哪个出口：`None` = 线路自带地址。告警文案要指名道姓，
/// 否则运维收到「线路 X 坏了」却发现直连是好的，下一次就不看告警了。
pub async fn best_word(
    state: &AppState,
    route_id: uuid::Uuid,
    now: i64,
) -> (&'static str, Option<uuid::Uuid>, crate::route_health::RouteHealth) {
    let own = crate::route_health::snapshot(state, route_id).await;
    let mut best = (
        crate::route_health::classify(&own, now),
        None::<uuid::Uuid>,
        own,
    );
    if serve_rank(best.0) == 0 {
        return best;
    }
    let ids: Vec<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM route_endpoints WHERE route_id = $1 AND active = true",
    )
    .bind(route_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    for id in ids {
        let h = crate::route_health::snapshot(state, id).await;
        let w = crate::route_health::classify(&h, now);
        if serve_rank(w) < serve_rank(best.0) {
            best = (w, Some(id), h);
            if serve_rank(w) == 0 {
                break;
            }
        }
    }
    best
}

/// 面板上那一格。`best_word` 的第一个值。
pub async fn aggregate_live(state: &AppState, route_id: uuid::Uuid, now: i64) -> &'static str {
    best_word(state, route_id, now).await.0
}

// ---------------------------------------------------------------- 后台接口

#[derive(Serialize)]
pub struct EndpointOut {
    pub id: uuid::Uuid,
    pub route_id: uuid::Uuid,
    pub label: String,
    pub base_url: String,
    /// 只回「有没有配密钥」，永远不回密钥本身 —— 后台页面也不例外。
    pub has_key: bool,
    pub cost_ratio: f64,
    pub active: bool,
    pub note: String,
    pub probe_ok: Option<bool>,
    pub probe_at: Option<chrono::DateTime<chrono::Utc>>,
    pub probe_ms: Option<i32>,
    pub probe_note: String,
    /// 这个出口实际有哪些模型。空 = 线路的全部。
    pub enabled_models: Vec<String>,
    /// 这个出口的协议。空 = 跟线路一样。
    pub protocol: String,
    /// 能扛多少（相对值）。null = 没填。
    pub capacity: Option<f64>,
    /// 调度器眼里它现在是什么状态：live / saturated / no_quota / auth。
    pub sched: &'static str,
    /// 下架的话，还有多少秒去试下一次。
    pub retry_in: Option<u64>,
    /// 真实流量的结论：ok / degraded / error / unknown。和探测是两个来源，都要看得见。
    pub live: String,
}

#[derive(Serialize)]
pub struct RouteOut {
    pub id: uuid::Uuid,
    pub label: String,
    pub protocol: String,
    /// 厂商标识（anthropic / openai / deepseek / …），前端据此挑图标。空 = 认不出来。
    pub vendor: &'static str,
    pub base_url: String,
    pub active: bool,
    pub model_count: usize,
    /// 这条线路开放的模型 id。
    pub models: Vec<String>,
    /// 这条线路怎么计费。出口窗口里**只读显示** —— 加一个出口时你要知道它的流量
    /// 会被按什么价计费，但计费是线路的属性，不能在出口这一层改。
    pub billing_mode: String,
    pub rate: f64,
    pub cache_disabled: bool,
    /// 单模型定价和显示名（线路上的那一份），出口窗口里可以就地编辑。
    pub model_prices: serde_json::Value,
    pub model_names: serde_json::Value,
    /// 线路自带那个地址的调度状态（它也是一个出口）。
    pub sched: &'static str,
    pub retry_in: Option<u64>,
    pub live: String,
    pub endpoints: Vec<EndpointOut>,
}

/// 调度器眼里这个出口现在是什么状态。
///
/// 三个词各对应一种「现在别用它」的理由，恢复方式完全不同 —— 所以界面上必须分开显示，
/// 混成一个「不可用」的话，运维看到红点不知道该去充值、去换密钥、还是什么都不用做。
fn sched_word(id: uuid::Uuid) -> &'static str {
    if let Some(r) = crate::models::endpoint_delisted(id) {
        return r.why.word();
    }
    if crate::models::endpoint_saturated(id, std::time::Instant::now(), Duration::ZERO) {
        return "saturated";
    }
    "live"
}

fn retry_in_secs(id: uuid::Uuid) -> Option<u64> {
    crate::models::endpoint_delisted(id).map(|r| {
        r.next_probe
            .saturating_duration_since(std::time::Instant::now())
            .as_secs()
    })
}

/// `POST /api/admin/route-endpoints/:id/relist` —— 手动把一个下架的出口放回去。
///
/// 充完钱不想等调度器那一轮时用。放回去之后它就是普通候选，真不行会立刻再被下架 ——
/// 所以这个按钮不会造成任何持久的坏状态。
pub async fn admin_relist(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let was = crate::models::relist_endpoint(id);
    if was {
        forget_delisting(&state, id).await;
    }
    Ok(Json(serde_json::json!({ "relisted": was })))
}

#[derive(Serialize)]
pub struct HealthRow {
    pub endpoint_id: uuid::Uuid,
    pub route_id: uuid::Uuid,
    pub route_label: String,
    pub vendor: &'static str,
    /// 出口备注；线路自带地址回「直连」。
    pub label: String,
    pub base_url: String,
    pub is_own: bool,
    pub active: bool,
    pub cost_ratio: f64,
    pub capacity: Option<f64>,
    /// 调度状态：live / saturated / no_quota / auth
    pub sched: &'static str,
    pub retry_in: Option<u64>,
    /// 真实流量的结论：ok / degraded / error / unknown
    pub live: String,
    pub consecutive_failures: i64,
    pub last_ok_secs_ago: Option<i64>,
    /// 最近一次主动探测
    pub probe_ok: Option<bool>,
    pub probe_ms: Option<i32>,
    pub probe_note: String,
    /// 用量：今天 / 最近 7 天
    pub calls_today: i64,
    pub cost_today_usd: f64,
    pub calls_7d: i64,
    pub cost_7d_usd: f64,
    pub cached_tokens_7d: i64,
    /// 余额。null = 这家没有可识别的余额接口，或者查失败 —— **不是 0**。
    pub balance: Option<String>,
    /// 我们声明开放、而上游清单里没有的模型。这些请求会撞 404。
    ///
    /// 空数组和 `manifest_note` 非空是两件事：前者是「比对过，没缺货」，
    /// 后者是「没比对成」。都塌成空数组的话，一家不提供 /models 的中转
    /// 看起来会和一家完全正常的一模一样。
    pub missing_models: Vec<String>,
    /// 没比对成时的原因。空 = 比对出结论了。
    pub manifest_note: String,
}

#[derive(sqlx::FromRow)]
struct UsageRow {
    endpoint_id: uuid::Uuid,
    calls_today: i64,
    cost_today: i64,
    calls_7d: i64,
    cost_7d: i64,
    cached_7d: i64,
}

/// `GET /api/admin/route-health` —— 健康面板要的全部事实。
///
/// 一次把「它现在什么状态、最近成不成、花了多少、还剩多少钱」凑齐。分散在几个接口里
/// 的话，页面要串行等好几轮，而这一页的用途正是「出事时快速看一眼」。
///
/// `?balance=1` 才去问上游余额 —— 那是几个网络往返，不该让每次刷新都付这个钱。
pub async fn admin_health(
    State(state): State<AppState>,
    claims: Claims,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let want_balance = q.get("balance").map(|v| v == "1").unwrap_or(false);

    let routes: Vec<Model> = sqlx::query_as("SELECT * FROM models ORDER BY sort, created_at")
        .fetch_all(&state.db)
        .await?;
    let eps: Vec<Endpoint> = sqlx::query_as("SELECT * FROM route_endpoints ORDER BY cost_ratio")
        .fetch_all(&state.db)
        .await?;
    // 用量一次查完，别在循环里逐个查。
    let usage: Vec<UsageRow> = sqlx::query_as(
        "SELECT endpoint_id, \
            COALESCE(SUM(calls) FILTER (WHERE day = current_date), 0)::bigint AS calls_today, \
            COALESCE(SUM(cost_micro_usd) FILTER (WHERE day = current_date), 0)::bigint AS cost_today, \
            COALESCE(SUM(calls), 0)::bigint AS calls_7d, \
            COALESCE(SUM(cost_micro_usd), 0)::bigint AS cost_7d, \
            COALESCE(SUM(cached_tokens), 0)::bigint AS cached_7d \
         FROM endpoint_usage WHERE day >= current_date - 6 GROUP BY endpoint_id",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let by_ep: HashMap<uuid::Uuid, &UsageRow> =
        usage.iter().map(|u| (u.endpoint_id, u)).collect();

    let now_ts = chrono::Utc::now().timestamp();
    let http = probe_client();
    let mut rows: Vec<HealthRow> = Vec::new();

    for r in &routes {
        let vendor = vendor_of(&r.provider, &crate::models::allowed_ids(r), &r.base_url);
        // 线路自带的地址也是一个出口，必须出现在面板里 —— 它往往是最常出问题的那个。
        let mut entries: Vec<(uuid::Uuid, String, String, bool, f64, Option<f64>, bool, Option<bool>, Option<i32>, String, String)> =
            vec![(
                r.id,
                "直连".into(),
                r.base_url.clone(),
                true,
                1.0,
                None,
                r.active,
                None,
                None,
                String::new(),
                r.api_key.clone(),
            )];
        for e in eps.iter().filter(|e| e.route_id == r.id) {
            let key = if e.api_key.trim().is_empty() { r.api_key.clone() } else { e.api_key.clone() };
            entries.push((
                e.id,
                if e.label.trim().is_empty() { "未命名出口".into() } else { e.label.clone() },
                e.base_url.clone(),
                false,
                e.cost_ratio,
                e.capacity,
                e.active,
                e.probe_ok,
                e.probe_ms,
                e.probe_note.clone(),
                key,
            ));
        }

        for (id, label, base, is_own, cost, cap, active, pok, pms, pnote, key) in entries {
            let h = crate::route_health::snapshot(&state, id).await;
            let u = by_ep.get(&id);
            let mf = crate::manifest_check::report_for(id);
            let balance = if want_balance && !key.trim().is_empty() {
                query_balance(&http, &base, &crate::models::model_key(&key)).await
            } else {
                None
            };
            rows.push(HealthRow {
                endpoint_id: id,
                route_id: r.id,
                route_label: r.label.clone(),
                vendor,
                label,
                base_url: base,
                is_own,
                active,
                cost_ratio: cost,
                capacity: cap,
                sched: sched_word(id),
                retry_in: retry_in_secs(id),
                live: crate::route_health::classify(&h, now_ts).to_string(),
                consecutive_failures: h.consecutive_failures,
                last_ok_secs_ago: h.last_ok_at.map(|t| now_ts.saturating_sub(t)),
                probe_ok: pok,
                probe_ms: pms,
                probe_note: pnote,
                calls_today: u.map(|x| x.calls_today).unwrap_or(0),
                cost_today_usd: u.map(|x| x.cost_today as f64 / 1_000_000.0).unwrap_or(0.0),
                calls_7d: u.map(|x| x.calls_7d).unwrap_or(0),
                cost_7d_usd: u.map(|x| x.cost_7d as f64 / 1_000_000.0).unwrap_or(0.0),
                cached_tokens_7d: u.map(|x| x.cached_7d).unwrap_or(0),
                balance,
                missing_models: mf.as_ref().map(|r| r.missing.clone()).unwrap_or_default(),
                // 还没轮到它比对时，note 说清楚是「还没测」，而不是留空冒充「没问题」。
                manifest_note: match &mf {
                    Some(r) => r.note.clone(),
                    None => "还没比对过".to_string(),
                },
            });
        }
    }

    // 告警收件人：一个 admin 把 email 填成用户名，就永远收不到线路告警，
    // 而这件事只在启动日志里闪一下。放到面板上。
    let admins = sqlx::query_scalar::<_, String>("SELECT email FROM users WHERE role = 'admin'")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    let usable = admins.iter().filter(|e| e.contains('@') && e.len() > 3).count();

    Ok(Json(serde_json::json!({
        "rows": rows,
        "alarm": { "usable": usable, "total": admins.len() },
        "balance_included": want_balance,
        // 有几个出口正在缺货。只数「比对出结论且真的缺」的，没比对成的不算。
        "missing_endpoints": crate::manifest_check::missing_endpoint_count(),
    })))
}

/// `GET /api/admin/route-endpoints` —— 每条线路 + 它挂了哪些出口。/// `GET /api/admin/route-endpoints` —— 每条线路 + 它挂了哪些出口。
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let routes: Vec<Model> =
        sqlx::query_as("SELECT * FROM models ORDER BY sort, created_at").fetch_all(&state.db).await?;
    let eps: Vec<Endpoint> = sqlx::query_as(
        "SELECT * FROM route_endpoints ORDER BY cost_ratio, created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let now = chrono::Utc::now().timestamp();

    let mut by_route: HashMap<uuid::Uuid, Vec<Endpoint>> = HashMap::new();
    for e in eps {
        by_route.entry(e.route_id).or_default().push(e);
    }

    let mut out = Vec::with_capacity(routes.len());
    for r in &routes {
        // 被分组到别处的线路仍然是独立线路（分组只动显示），所以照样列出来 ——
        // 它有自己的密钥和账单，也就该能挂自己的出口。
        let mut list = Vec::new();
        for e in by_route.remove(&r.id).unwrap_or_default() {
            let h = crate::route_health::snapshot(&state, e.id).await;
            list.push(EndpointOut {
                id: e.id,
                route_id: e.route_id,
                label: e.label,
                base_url: e.base_url,
                has_key: !e.api_key.trim().is_empty(),
                cost_ratio: e.cost_ratio,
                active: e.active,
                note: e.note,
                probe_ok: e.probe_ok,
                probe_at: e.probe_at,
                probe_ms: e.probe_ms,
                probe_note: e.probe_note,
                enabled_models: e.enabled_models,
                protocol: e.protocol,
                capacity: e.capacity,
                sched: sched_word(e.id),
                retry_in: retry_in_secs(e.id),
                live: crate::route_health::classify(&h, now).to_string(),
            });
        }
        out.push(RouteOut {
            id: r.id,
            label: r.label.clone(),
            protocol: r.protocol.clone(),
            vendor: vendor_of(&r.provider, &crate::models::allowed_ids(r), &r.base_url),
            base_url: r.base_url.clone(),
            active: r.active,
            model_count: crate::models::allowed_ids(r).len(),
            models: crate::models::allowed_ids(r),
            billing_mode: r.billing_mode.clone(),
            rate: r.rate,
            cache_disabled: r.cache_disabled,
            model_prices: r.model_prices.clone(),
            model_names: r.model_names.clone(),
            sched: sched_word(r.id),
            retry_in: retry_in_secs(r.id),
            live: aggregate_live(&state, r.id, now).await.to_string(),
            endpoints: list,
        });
    }
    Ok(Json(serde_json::json!({ "routes": out })))
}

/// 前端送来的保存请求。
///
/// # 每个字段都要能吃 `null`
///
/// `#[serde(default)]` 只管**字段缺失**，管不了字段在、值是 `null`。而前端里
/// `x ? f(x) : null`、以及 `Number(...)` 出 NaN 被 `JSON.stringify` 写成 `null`，
/// 都太常见了 —— 一个显式 null 打在 `String` 或 `f64` 上，请求会在**进入处理函数
/// 之前**被提取器拒掉，报一句英文 serde 错，服务端一行日志都没有。
///
/// 那正是「点保存没反应、查不出原因」的形状。所以这里一律用 `null_as_*`：
/// null 一律当成没填，真正的校验交给下面那几个 `clean_*`，它们会说人话。
#[derive(Deserialize)]
pub struct SaveReq {
    #[serde(default)]
    pub id: Option<uuid::Uuid>,
    pub route_id: uuid::Uuid,
    #[serde(default, deserialize_with = "null_as_default")]
    pub label: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub base_url: String,
    /// 空字符串 = 不改（改地址时不用把密钥再抄一遍）。
    #[serde(default, deserialize_with = "null_as_default")]
    pub api_key: String,
    #[serde(default = "one", deserialize_with = "null_as_one")]
    pub cost_ratio: f64,
    #[serde(default = "yes", deserialize_with = "null_as_yes")]
    pub active: bool,
    #[serde(default, deserialize_with = "null_as_default")]
    pub note: String,
    /// 这个出口实际有哪些模型。空数组 = 线路的全部。
    #[serde(default, deserialize_with = "null_as_default")]
    pub enabled_models: Vec<String>,
    /// 空串 = 跟线路一样。只收 anthropic / openai。
    #[serde(default, deserialize_with = "null_as_default")]
    pub protocol: String,
    /// 能扛多少（相对值）。None / 0 = 不填。
    #[serde(default)]
    pub capacity: Option<f64>,
    /// 顺手改的单模型定价，形状 `{ "模型id": {"in": 3.0, "out": 15.0} }`。
    ///
    /// **写到线路上，不写到出口上。** 价格是线路的属性，同一条线路的几个出口共用一份。
    /// 放在这个窗口里只是因为「发现新模型」和「给它定价」是同一件事的两半 ——
    /// 让人跑去另一页再回来，多数人会直接放弃，然后那个模型就永远开放不了。
    #[serde(default)]
    pub model_prices: Option<serde_json::Value>,
    /// 单模型显示名，同上，也写到线路。
    #[serde(default)]
    pub model_names: Option<serde_json::Value>,
}

/// 把 `null` 当成「没填」。见 SaveReq 上面那段。
fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

fn null_as_one<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    Ok(Option::<f64>::deserialize(d)?.unwrap_or(1.0))
}

fn null_as_yes<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(Option::<bool>::deserialize(d)?.unwrap_or(true))
}

fn one() -> f64 {
    1.0
}
fn yes() -> bool {
    true
}

/// 折扣的合理区间。
///
/// 上限 1.0：比原价还贵的转卖没有存在意义，填出来只会是小数点点错（10 当成十倍折扣）。
/// 下限不设 0：`cost_ratio > 0` 由表上的 CHECK 兜着，而 0 会让「免费」永远排第一，
/// 那反而是对的 —— 真有免费额度的出口就该先用。
fn clean_ratio(v: f64) -> ApiResult<f64> {
    if !v.is_finite() || v <= 0.0 {
        return Err(AppError::bad("进价折扣要是个大于 0 的数（0.3 = 三折）"));
    }
    if v > 1.0 {
        return Err(AppError::bad("进价折扣不能大于 1.0（1.0 就是原价）"));
    }
    Ok(v)
}

/// 协议只认这两个。
///
/// 不认识的字符串会一路带到发请求那一步，然后走进「不是 anthropic 就当 openai」的分支 ——
/// 拼出一个 /chat/completions 打给一个只认 /v1/messages 的上游，报一个看不懂的 404。
/// 在入口挡住，错误就停在填表的人面前。
fn clean_protocol(v: &str) -> ApiResult<String> {
    let p = v.trim().to_ascii_lowercase();
    if p.is_empty() || p == "anthropic" || p == "openai" {
        Ok(p)
    } else {
        Err(AppError::bad("上游协议只能是 anthropic 或 openai（留空 = 跟线路一样）"))
    }
}

/// 容量的合理区间。
///
/// 只做「是不是个正常的正数」这一层校验，不猜单位 —— 算法只看同一条线路下几个出口
/// 之间的比值，填 RPM、并发数还是 1/2/3 都行。0 和负数拒掉：那该用「停用」表达。
fn clean_capacity(v: Option<f64>) -> ApiResult<Option<f64>> {
    match v {
        None => Ok(None),
        // 前端空输入会送 0 过来，当成「没填」而不是报错。
        Some(x) if x == 0.0 => Ok(None),
        Some(x) if !x.is_finite() || x < 0.0 => {
            Err(AppError::bad("容量要是个大于 0 的数，留空表示不填"))
        }
        Some(x) if x > 1_000_000.0 => Err(AppError::bad("容量填得太大了，检查一下是不是多打了几个零")),
        Some(x) => Ok(Some(x)),
    }
}

fn clean_url(v: &str) -> ApiResult<String> {
    let u = v.trim().trim_end_matches('/').to_string();
    if u.is_empty() {
        return Err(AppError::bad("中转地址不能为空"));
    }
    if u.len() > MAX_URL {
        return Err(AppError::bad("中转地址太长了"));
    }
    // 只收 http(s)。别的协议进到这里只会在发请求时报一个看不懂的错。
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        return Err(AppError::bad("中转地址要以 http:// 或 https:// 开头"));
    }
    Ok(u)
}

/// `POST /api/admin/route-endpoints` —— 新增或修改一个出口。
pub async fn admin_save(
    State(state): State<AppState>,
    claims: Claims,
    // 收原始字节自己解，不用 `Json<SaveReq>`。
    //
    // 这是被一次真实故障逼出来的：控制台连着 5 次 400，而 axum 的提取器在字段类型
    // 对不上时**先于处理函数**就把请求拒了 —— 那种 400 是英文的 serde 报错、不进
    // 任何日志、也不经过这里加的任何一行代码。于是「它到底为什么不让我存」在服务端
    // 一点线索都没有，只能靠猜。
    //
    // 自己解之后：解不出来是一句说得清的中文 + 一条带字段名的日志。多一次
    // from_slice 的开销，换掉一整类查不出来的失败。
    body: axum::body::Bytes,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let req: SaveReq = serde_json::from_slice(&body).map_err(|e| {
        tracing::warn!(
            error = %e,
            bytes = body.len(),
            "出口保存：请求体解不出来（字段类型对不上，或者前端发了个意外的形状）"
        );
        AppError::bad(format!("请求格式不对：{e}"))
    })?;

    // 被拒的保存要留痕。
    //
    // 这一段是真实故障逼出来的：控制台连着 5 次 400，而 400 不进错误日志、响应又走
    // MSE 加密（nginx 记的是密文长度），于是「它到底为什么不让我存」在服务端**一点
    // 线索都没有**。校验失败是运维每天都会撞到的事，不是异常，所以它该有日志 ——
    // 带够定位用的字段，唯独不带密钥。
    let reject = |why: AppError| -> AppError {
        tracing::warn!(
            route_id = %req.route_id,
            base_url = %req.base_url,
            models = req.enabled_models.len(),
            protocol = %req.protocol,
            editing = req.id.is_some(),
            reason = %why.msg,
            "出口没存成"
        );
        why
    };

    let base_url = clean_url(&req.base_url).map_err(reject)?;
    let cost_ratio = clean_ratio(req.cost_ratio).map_err(reject)?;
    let label: String = req.label.trim().chars().take(MAX_LABEL).collect();
    let note: String = req.note.trim().chars().take(MAX_NOTE).collect();
    let protocol = clean_protocol(&req.protocol).map_err(reject)?;
    let capacity = clean_capacity(req.capacity).map_err(reject)?;

    let route: Option<Model> = sqlx::query_as("SELECT * FROM models WHERE id = $1")
        .bind(req.route_id)
        .fetch_optional(&state.db)
        .await?;
    let Some(route) = route else {
        return Err(reject(AppError::bad("线路不存在")));
    };

    // 先把这次顺手填的定价并进线路，再做价格闸校验。
    //
    // 顺序不能反：新模型的价就是在这个窗口里填的，先校验的话它永远查不到价、永远存不上，
    // 而报错还让人「去线路那页填」—— 那页根本没有这个模型。
    let mut route = route;
    if req.model_prices.is_some() || req.model_names.is_some() {
        merge_route_pricing(&state, &mut route, &req).await?;
    }

    // 出口可以带来线路本身没有的模型 —— 新挂一个中转，它那儿多了两款货，
    // 那两款就该出现在 IDE 的列表里。
    //
    // 但有一条闸：**算不出价格的不许开放**。价格有三条来源（每模型覆盖 → 实时目录 →
    // 线路兜底价），三条都没有时 `compute_cost` 会算出 0，用户一分不付而上游照收你的钱。
    // 那不是功能，是漏洞。所以这里拒掉，并在报错里说清楚该去哪儿补价。
    let allowed = crate::models::allowed_ids(&route);
    let mut enabled_models: Vec<String> = req
        .enabled_models
        .iter()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .collect();
    enabled_models.sort();
    enabled_models.dedup();
    if let Some(bad) = enabled_models
        .iter()
        .find(|m| !allowed.contains(m) && !priceable(&route, m))
    {
        return Err(reject(AppError::bad(format!(
            "「{bad}」是这条线路没有的新模型，但算不出它的价格 —— 开放出去用户一分不付、\
             上游照收你的钱。去线路那页给它填一个单模型价，或者先别勾它。"
        ))));
    }
    // 「正好等于线路那一份」和「不选」是同一件事，都存成空：这样以后线路加了新模型，
    // 出口会自动跟着有，而不是停在保存那天的那一份名单上。
    //
    // 判据必须是**集合相等**，不能是长度相等。出口现在能带来线路没有的模型 ——
    // 线路有 6 个，勾了 4 个原有 + 2 个新的也是 6 个，按长度判会把整份选择清空，
    // 那两个新模型**静默消失**：存的时候不报错，只是它们再也不会被派到这个出口。
    let is_exactly_the_routes_own = enabled_models.len() == allowed.len()
        && enabled_models.iter().all(|m| allowed.contains(m));
    if is_exactly_the_routes_own {
        enabled_models.clear();
    }

    let id = match req.id {
        Some(id) => {
            // 密钥空着 = 沿用原值。这一步必须在 UPDATE 之外先取出来，
            // 不然一次「只改地址」的保存会把密钥清成空。
            let keep: Option<String> =
                sqlx::query_scalar("SELECT api_key FROM route_endpoints WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&state.db)
                    .await?;
            let Some(keep) = keep else {
                return Err(AppError::bad("这个出口不存在"));
            };
            let stored = if req.api_key.trim().is_empty() {
                keep
            } else {
                crate::field_crypto::encrypt(req.api_key.trim(), crate::models::MODEL_KEY_CTX)
            };
            sqlx::query(
                "UPDATE route_endpoints SET route_id = $2, label = $3, base_url = $4, \
                 api_key = $5, cost_ratio = $6, active = $7, note = $8, \
                 enabled_models = $9, protocol = $10, capacity = $11, updated_at = now() \
                 WHERE id = $1",
            )
            .bind(id)
            .bind(req.route_id)
            .bind(&label)
            .bind(&base_url)
            .bind(&stored)
            .bind(cost_ratio)
            .bind(req.active)
            .bind(&note)
            .bind(&enabled_models)
            .bind(&protocol)
            .bind(capacity)
            .execute(&state.db)
            .await
            .map_err(dup_url)?;
            id
        }
        None => {
            let stored =
                crate::field_crypto::encrypt(req.api_key.trim(), crate::models::MODEL_KEY_CTX);
            sqlx::query_scalar(
                "INSERT INTO route_endpoints (route_id, label, base_url, api_key, cost_ratio, \
                 active, note, enabled_models, protocol, capacity) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id",
            )
            .bind(req.route_id)
            .bind(&label)
            .bind(&base_url)
            .bind(&stored)
            .bind(cost_ratio)
            .bind(req.active)
            .bind(&note)
            .bind(&enabled_models)
            .bind(&protocol)
            .bind(capacity)
            .fetch_one(&state.db)
            .await
            .map_err(dup_url)?
        }
    };

    // 存完立刻探一次。加一个出口最想知道的就是「这个密钥对不对」，
    // 而等 15 分钟后的后台轮次才告诉你填错了，那期间它一直在候选池里。
    let ep: Option<Endpoint> = sqlx::query_as("SELECT * FROM route_endpoints WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let probe = match &ep {
        Some(e) => {
            let out = probe_and_store(&state, e, &route).await;
            serde_json::json!({ "ok": out.ok, "ms": out.ms, "note": out.note })
        }
        None => serde_json::Value::Null,
    };

    tracing::info!(route_id = %req.route_id, endpoint = %id, "出口已保存");
    Ok(Json(serde_json::json!({ "id": id, "probe": probe })))
}

/// 把这次填的单模型定价合并进**线路**。
///
/// # 为什么不存到出口上
///
/// 价格是线路的属性 —— 同一条线路的几个出口对用户完全等价，只有我的进价不同。
/// 要是每个出口各存一份价，同一个模型用户被扣多少钱就要看当时哪家先答；这正是整套
/// 多路由设计第一天就堵死的那个洞，不能从这个窗口再开一次。
///
/// 所以这里是「在出口窗口里编辑线路的定价」，不是「给出口定价」。合并而不是覆盖：
/// 这个窗口只列了这一家有的那些模型，整份覆盖会把线路上别的模型的价抹掉。
async fn merge_route_pricing(
    state: &AppState,
    route: &mut Model,
    req: &SaveReq,
) -> ApiResult<()> {
    fn merge(base: &serde_json::Value, patch: &serde_json::Value) -> serde_json::Value {
        let mut out = base.as_object().cloned().unwrap_or_default();
        if let Some(p) = patch.as_object() {
            for (k, v) in p {
                // null / 空对象 = 把这一条删掉，而不是写一个空进去。
                if v.is_null() {
                    out.remove(k);
                } else {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        serde_json::Value::Object(out)
    }

    let prices = match &req.model_prices {
        Some(p) => merge(&route.model_prices, p),
        None => route.model_prices.clone(),
    };
    let names = match &req.model_names {
        Some(n) => merge(&route.model_names, n),
        None => route.model_names.clone(),
    };
    sqlx::query("UPDATE models SET model_prices = $2, model_names = $3 WHERE id = $1")
        .bind(route.id)
        .bind(&prices)
        .bind(&names)
        .execute(&state.db)
        .await?;
    route.model_prices = prices;
    route.model_names = names;
    Ok(())
}

/// 唯一索引撞了就说人话。原始报错里有表名、索引名和列值，对运维没用。
fn dup_url(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &e {
        if db.code().as_deref() == Some("23505") {
            return AppError::bad("这条线路下已经有同样的中转地址了");
        }
    }
    AppError::from(e)
}

/// `POST /api/admin/route-endpoints/:id/probe` —— 手动测一个出口。
pub async fn admin_probe(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let ep: Option<Endpoint> = sqlx::query_as("SELECT * FROM route_endpoints WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let Some(ep) = ep else {
        return Err(AppError::not_found("这个出口不存在"));
    };
    let route: Option<Model> = sqlx::query_as("SELECT * FROM models WHERE id = $1")
        .bind(ep.route_id)
        .fetch_optional(&state.db)
        .await?;
    let Some(route) = route else {
        return Err(AppError::bad("这个出口挂的线路已经不在了"));
    };
    let out = probe_and_store(&state, &ep, &route).await;
    Ok(Json(
        serde_json::json!({ "ok": out.ok, "ms": out.ms, "note": out.note }),
    ))
}

/// `POST /api/admin/route-endpoints/:id/probe-route` —— 测线路自带的那个地址。
///
/// 线路自带的地址也是一个出口，而且是默认那个。只能测转卖出口、测不了它，
/// 等于最常出问题的那个反而看不见。它的结论不落 `route_endpoints`（那里没有它的行），
/// 只即时回给页面。
pub async fn admin_probe_route(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let route: Option<Model> = sqlx::query_as("SELECT * FROM models WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let Some(route) = route else {
        return Err(AppError::not_found("线路不存在"));
    };
    let key = crate::models::model_key(&route.api_key);
    // 线路自带的地址：协议和模型都用线路自己的。
    let out = probe_once(&probe_client(), &route, &route.base_url, &key, "", &[]).await;
    Ok(Json(
        serde_json::json!({ "ok": out.ok, "ms": out.ms, "note": out.note }),
    ))
}

#[derive(Deserialize)]
pub struct AvailableReq {
    pub route_id: uuid::Uuid,
    /// 还没保存的出口也要能拉 —— 否则运维得先存一个可能是错的配置才知道它有什么货。
    pub base_url: String,
    /// 空 = 用这个出口已存的密钥（改地址时不用重抄），再空 = 用线路的。
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub id: Option<uuid::Uuid>,
}

/// `POST /api/admin/route-endpoints/available` —— 问这个中转「你有哪些模型」。
///
/// 和线路那边的「拉取可用模型」是同一件事，但这里必须能在**保存之前**拉：出口的价值
/// 就在于「这家有没有我要的那几个模型」，先存再看等于先把一个不知道行不行的出口放进
/// 候选池。
///
/// 回四组，不是交集。**这里曾经只回交集，后来推翻了**：出口的价值有一半就在于
/// 「这家多了两款线路没有的货」，只回交集等于把那一半藏起来，运维在界面上永远勾不到。
///
///   · `here`           —— 线路开放 ∩ 这家有
///   · `missing`        —— 线路开放，但这家没有（派过去只会撞 404）
///   · `extra`          —— 这家有、线路没有，且**算得出价格** → 勾上就新增到 IDE 列表
///   · `extra_no_price` —— 同上但算不出价格
///
/// `extra` 和 `extra_no_price` 分两堆，是因为算不出价的开放出去等于白送：用户一分不付，
/// 上游照收你的钱（见 `priceable`）。分好之后界面把后者标红、勾不动，并在同一行给一个
/// 填价框 —— 而不是等运维保存时才报一句错，或者更糟：让那个模型凭空消失。
pub async fn admin_available(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<AvailableReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let base_url = clean_url(&req.base_url)?;
    let route: Option<Model> = sqlx::query_as("SELECT * FROM models WHERE id = $1")
        .bind(req.route_id)
        .fetch_optional(&state.db)
        .await?;
    let Some(route) = route else {
        return Err(AppError::bad("线路不存在"));
    };

    let key = if !req.api_key.trim().is_empty() {
        req.api_key.trim().to_string()
    } else {
        let stored: Option<String> = match req.id {
            Some(id) => sqlx::query_scalar("SELECT api_key FROM route_endpoints WHERE id = $1")
                .bind(id)
                .fetch_optional(&state.db)
                .await?,
            None => None,
        };
        match stored.filter(|k| !k.trim().is_empty()) {
            Some(k) => crate::models::model_key(&k),
            None => crate::models::model_key(&route.api_key),
        }
    };

    let url = format!("{}/models", crate::models::api_base(&base_url));
    let resp = probe_client()
        .get(&url)
        .header("authorization", format!("Bearer {key}"))
        .header("x-api-key", &key)
        .send()
        .await
        // 和探测同一条规矩：不回显 reqwest 的错误原文，它带完整 URL，
        // 而有些转卖商要求把密钥写在查询串里。
        .map_err(|_| AppError::bad("连不上这个地址（域名、端口或网络不对）"))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(AppError::bad(match status {
            401 | 403 => "密钥被拒（401/403）".to_string(),
            404 => "这个地址没有 /models 接口（有些中转不提供，可以直接手动勾选）".to_string(),
            _ => format!("上游返回 {status}"),
        }));
    }
    let data: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let ids: Vec<String> = data
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.get("id").and_then(|i| i.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let allowed = crate::models::allowed_ids(&route);
    let here: Vec<String> = allowed.iter().filter(|m| ids.contains(m)).cloned().collect();
    let missing: Vec<String> = allowed.iter().filter(|m| !ids.contains(m)).cloned().collect();
    // 这家有、而线路没有的 —— 勾上就会**新增**到 IDE 的模型列表里。
    //
    // 分成能开放和不能开放两堆：算不出价格的开放出去，用户一分不付而上游照收你的钱。
    // 所以这里先替运维把这件事分好，而不是等他保存时才报错。
    let (extra_ok, extra_no_price): (Vec<String>, Vec<String>) = ids
        .iter()
        .filter(|m| !allowed.contains(m))
        .cloned()
        .partition(|m| priceable(&route, m));
    Ok(Json(serde_json::json!({
        "here": here,
        "missing": missing,
        "extra": extra_ok,
        "extra_no_price": extra_no_price,
        "upstream_total": ids.len(),
    })))
}

/// `DELETE /api/admin/route-endpoints/:id`
pub async fn admin_delete(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let n = sqlx::query("DELETE FROM route_endpoints WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();
    Ok(Json(serde_json::json!({ "deleted": n })))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 这个模块的源码（不含测试），用来钉住那些「读起来对、但一改就静默失效」的地方。
    fn src() -> String {
        let all = include_str!("route_endpoints.rs");
        // 断言字面量本身出现在测试里，扫描时必须先把测试段切掉，否则测试在自我印证。
        all.split("\n#[cfg(test)]").next().unwrap().to_string()
    }

    fn model(protocol: &str) -> Model {
        // 只填排序和展开会读到的字段，其余走 Default —— 这里测的是选路，不是计费。
        Model {
            id: uuid::Uuid::new_v4(),
            base_url: "https://own.example.com".into(),
            api_key: "own-key".into(),
            protocol: protocol.into(),
            enabled_models: vec!["claude-opus-5".into()],
            ..Model::blank()
        }
    }

    fn ep(cost: f64, probe: Option<bool>, url: &str) -> Endpoint {
        Endpoint {
            id: uuid::Uuid::new_v4(),
            route_id: uuid::Uuid::nil(),
            label: String::new(),
            base_url: url.into(),
            api_key: "ep-key".into(),
            cost_ratio: cost,
            active: true,
            note: String::new(),
            probe_ok: probe,
            probe_at: None,
            probe_ms: None,
            probe_note: String::new(),
            enabled_models: Vec::new(),
            protocol: String::new(),
            capacity: None,
        }
    }

    #[test]
    fn cheaper_endpoint_goes_first_but_only_when_it_works() {
        // 便宜且能用 → 排第一。
        assert!(order_key(Some(true), 0.3) < order_key(None, 1.0));
        // 便宜但已知打不通 → 排到没测过的后面。这是最要紧的一条：反过来的话，
        // 每个请求都会先去撞那个便宜的死出口，而一个请求只有两次机会。
        assert!(order_key(Some(false), 0.1) > order_key(None, 1.0));
        assert!(order_key(Some(false), 0.1) > order_key(Some(true), 1.0));
        // 同一档里才比价钱。
        assert!(order_key(Some(true), 0.3) < order_key(Some(true), 0.5));
    }

    /// 同价位时直连要留在前面。
    ///
    /// 这一条钉的是一个真出现过的判据错误：线路自带地址在这张表里没有行，探测结论
    /// 无处可存，于是它曾经永远停在「还没测过」那一档 —— 加一个**原价**的备用中转，
    /// 只要它测通就把直连整个顶掉，白多一跳、多一个第三方，而界面上看不出为什么。
    #[test]
    fn a_same_price_relay_does_not_displace_the_direct_connection() {
        let r = model("anthropic");
        let mut map = HashMap::new();
        map.insert(
            r.id,
            vec![
                // 原价、测通 —— 不该越过直连。
                ep(1.0, Some(true), "https://same-price.example.com"),
                // 便宜、测通 —— 应该越过直连。
                ep(0.4, Some(true), "https://cheaper.example.com"),
            ],
        );
        let urls: Vec<String> = expand(&[r], &map, "claude-opus-5").into_iter().map(|m| m.base_url).collect();
        assert_eq!(
            urls,
            vec![
                "https://cheaper.example.com",   // 真便宜，越过直连
                "https://own.example.com",       // 直连：同价位在任者优先
                "https://same-price.example.com" // 原价转卖：没有理由排到直连前面
            ]
        );
    }

    /// 直连不会因为「没测过」就被判到失败出口后面。
    #[test]
    fn the_direct_connection_outranks_a_broken_relay_however_cheap() {
        let r = model("anthropic");
        let mut map = HashMap::new();
        map.insert(r.id, vec![ep(0.05, Some(false), "https://broken-but-cheap.example.com")]);
        let urls: Vec<String> = expand(&[r], &map, "claude-opus-5").into_iter().map(|m| m.base_url).collect();
        assert_eq!(urls[0], "https://own.example.com", "一折但打不通的出口抢到了第一位");
    }

    #[test]
    fn expanding_a_route_without_endpoints_changes_nothing() {
        // 没配多路由的线路必须和今天一模一样：展开成一个出口，就是它自己。
        let r = model("anthropic");
        let out = expand(&[r.clone()], &HashMap::new(), "claude-opus-5");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].base_url, "https://own.example.com");
        assert_eq!(out[0].api_key, "own-key");
        assert!(out[0].endpoint_id.is_none());
    }

    #[test]
    fn endpoints_never_change_the_price_fields() {
        // 这是整张表存在的理由：换出口不能换账单。
        let mut r = model("anthropic");
        r.input_price = 3.0;
        r.output_price = 15.0;
        r.rate = 2.0;
        r.billing_mode = "rate".into();
        r.per_call_cents = 7;
        let mut map = HashMap::new();
        map.insert(r.id, vec![ep(0.2, Some(true), "https://cheap.example.com")]);

        for m in expand(&[r.clone()], &map, "claude-opus-5") {
            assert_eq!(m.input_price, 3.0, "出口改动了输入价");
            assert_eq!(m.output_price, 15.0, "出口改动了输出价");
            assert_eq!(m.rate, 2.0, "出口改动了倍率");
            assert_eq!(m.billing_mode, "rate", "出口改动了计费模式");
            assert_eq!(m.per_call_cents, 7, "出口改动了每次调用价");
            assert_eq!(m.id, r.id, "出口把线路身份换掉了——用量会记到别处");
            assert_eq!(m.enabled_models, r.enabled_models, "出口改动了开放模型");
        }
    }

    #[test]
    fn cheap_working_endpoint_outranks_the_routes_own_address() {
        let r = model("anthropic");
        let mut map = HashMap::new();
        map.insert(
            r.id,
            vec![
                ep(0.5, Some(true), "https://mid.example.com"),
                ep(0.2, Some(true), "https://cheap.example.com"),
                ep(0.1, Some(false), "https://broken.example.com"),
            ],
        );
        let out = expand(&[r], &map, "claude-opus-5");
        let urls: Vec<&str> = out.iter().map(|m| m.base_url.as_str()).collect();
        assert_eq!(
            urls,
            vec![
                "https://cheap.example.com",  // 最便宜且测过能用
                "https://mid.example.com",    // 次便宜且能用
                "https://own.example.com",    // 线路自带，没测过
                "https://broken.example.com", // 最便宜但测过是坏的 → 兜底
            ]
        );
    }

    #[test]
    fn an_endpoint_without_its_own_key_borrows_the_routes() {
        let r = model("anthropic");
        let mut e = ep(0.2, Some(true), "https://cheap.example.com");
        e.api_key = "   ".into();
        let mut map = HashMap::new();
        map.insert(r.id, vec![e]);
        let out = expand(&[r], &map, "claude-opus-5");
        let cheap = out.iter().find(|m| m.base_url.contains("cheap")).unwrap();
        assert_eq!(cheap.api_key, "own-key");
    }

    /// 出口只承载它真有的那几个模型。
    ///
    /// 转卖商之间的货不一样。不筛的话，opus 的请求会被派到一个只有 sonnet 的出口上
    /// 撞 404 —— 而每个请求只有两次机会，这一撞就浪费掉一半，用户看到的是变慢。
    #[test]
    fn an_endpoint_only_serves_the_models_it_actually_has() {
        let mut r = model("anthropic");
        r.enabled_models = vec!["claude-opus-5".into(), "claude-sonnet-5".into()];
        let mut only_sonnet = ep(0.2, Some(true), "https://sonnet-only.example.com");
        only_sonnet.enabled_models = vec!["claude-sonnet-5".into()];
        let mut map = HashMap::new();
        map.insert(r.id, vec![only_sonnet]);

        // 要 sonnet：那个便宜出口能用，排第一。
        let urls: Vec<String> = expand(&[r.clone()], &map, "claude-sonnet-5")
            .into_iter()
            .map(|m| m.base_url)
            .collect();
        assert_eq!(urls[0], "https://sonnet-only.example.com");

        // 要 opus：它根本不该出现在候选里。
        let urls: Vec<String> = expand(&[r], &map, "claude-opus-5")
            .into_iter()
            .map(|m| m.base_url)
            .collect();
        assert_eq!(
            urls,
            vec!["https://own.example.com"],
            "一个没有 opus 的出口被派了 opus 的请求"
        );
    }

    /// 出口可以带来线路本身没有的模型。
    #[test]
    fn an_endpoint_can_bring_models_the_route_never_had() {
        let mut r = model("anthropic");
        r.enabled_models = vec!["claude-opus-5".into()];
        let mut e = ep(0.3, Some(true), "https://extra.example.com");
        e.enabled_models = vec!["claude-opus-5".into(), "claude-haiku-9".into()];
        let all = effective_models(&r, &[e]);
        assert!(all.contains(&"claude-haiku-9".to_string()), "出口带来的新模型没进并集");
        assert!(all.contains(&"claude-opus-5".to_string()));
        assert_eq!(all.len(), 2, "并集里出现了重复");
        // 停用的出口不该贡献模型。
        let mut off = ep(0.3, Some(true), "https://off.example.com");
        off.enabled_models = vec!["ghost-model".into()];
        off.active = false;
        assert!(!effective_models(&r, &[off]).contains(&"ghost-model".to_string()));
    }

    /// 线路自带的地址没有那款货时，不能把请求派给它。
    ///
    /// 派过去只会撞一个 404，而每个请求只有两次机会 —— 白撞一次就浪费掉一半。
    #[test]
    fn the_direct_address_is_skipped_for_a_model_it_does_not_have() {
        let mut r = model("anthropic");
        r.enabled_models = vec!["claude-opus-5".into()];
        let mut e = ep(0.3, Some(true), "https://extra.example.com");
        e.enabled_models = vec!["claude-haiku-9".into()];
        let mut map = HashMap::new();
        map.insert(r.id, vec![e]);

        // 只有出口有的那款：候选里不该出现线路自带地址
        let urls: Vec<String> = expand(&[r.clone()], &map, "claude-haiku-9")
            .into_iter()
            .map(|m| m.base_url)
            .collect();
        assert_eq!(urls, vec!["https://extra.example.com"], "把新模型派给了没有它的直连");

        // 线路自己那款：出口没有它，所以只剩直连
        let urls: Vec<String> = expand(&[r], &map, "claude-opus-5")
            .into_iter()
            .map(|m| m.base_url)
            .collect();
        assert_eq!(urls, vec!["https://own.example.com"]);
    }

    /// 算不出价格的模型不许开放。
    ///
    /// 价格有三条来源：每模型覆盖 → 实时目录 → 线路兜底价。三条都没有时
    /// `compute_cost` 会算出 0 —— 用户一分不付，而上游照收你的钱。这不是少收了点钱，
    /// 是每一次调用都在白送，而且账面上完全看不出来。
    #[test]
    fn a_model_with_no_resolvable_price_is_refused() {
        let mut r = model("anthropic");
        // 线路兜底价为 0（线上实测就是这样），目录里也没有这个自造的名字
        r.input_price = 0.0;
        r.output_price = 0.0;
        assert!(!priceable(&r, "some-relay-private-name-v9"), "查不到价却说能开放");

        // 填了单模型价就能开放
        r.model_prices = serde_json::json!({ "some-relay-private-name-v9": { "in": 3.0, "out": 15.0 } });
        assert!(priceable(&r, "some-relay-private-name-v9"));

        // 线路兜底价也算一条来源
        let mut r2 = model("anthropic");
        r2.input_price = 2.0;
        assert!(priceable(&r2, "anything-at-all"));
    }

    /// 「全选归一成空」的判据必须是集合相等，不能是长度相等。
    ///
    /// 出口能带来线路没有的模型之后，长度判就错了：线路 6 个，勾 4 个原有 + 2 个新的
    /// 也是 6 个 —— 按长度判会把整份选择清空，那两个新模型**静默消失**。
    /// 保存不报错，只是它们再也不会被派到这个出口，而界面上还显示勾着。
    #[test]
    fn the_normalise_to_empty_rule_compares_sets_not_lengths() {
        let s = src();
        let i = s.find("pub async fn admin_save(").expect("保存入口不见了");
        let body = &s[i..];
        assert!(
            body.contains("enabled_models.iter().all(|m| allowed.contains(m))"),
            "还在按长度判：勾了同样多但含新模型时，那几个新模型会被静默清掉"
        );
        // 纯逻辑复现一遍，防止实现改成别的等价写法后这条断言失去意义。
        let allowed = vec!["a".to_string(), "b".into(), "c".into()];
        let same_len_but_different = vec!["a".to_string(), "b".into(), "新模型".into()];
        let exactly = vec!["a".to_string(), "b".into(), "c".into()];
        let judge = |sel: &Vec<String>| {
            sel.len() == allowed.len() && sel.iter().all(|m| allowed.contains(m))
        };
        assert!(!judge(&same_len_but_different), "含新模型的选择被当成了「就是线路那一份」");
        assert!(judge(&exactly));
    }

    /// 在出口窗口里填的价，必须写到**线路**上，不能写到出口上。
    ///
    /// 这是新开的一条写入路径，也是最容易把不变量弄丢的地方：每个出口各存一份价的话，
    /// 同一个模型用户被扣多少钱就要看当时哪家先答 —— 那正是整套多路由第一天堵死的洞。
    #[test]
    fn prices_edited_in_the_outlet_dialog_are_stored_on_the_route() {
        let s = src();
        let i = s.find("async fn merge_route_pricing(").expect("合并函数不见了");
        let body = &s[i..s[i..].find("\n/// 唯一索引").map(|j| i + j).unwrap_or(s.len())];
        assert!(
            body.contains("UPDATE models SET model_prices"),
            "定价没写到 models 表 —— 写到出口上就等于每个出口一份价"
        );
        assert!(
            !body.contains("UPDATE route_endpoints"),
            "定价被写到出口表上了：同一个模型的账单会随出口变"
        );
        // route_endpoints 表结构里也不许出现价格列。
        for mig in [
            include_str!("../migrations/20260851_route_endpoints.sql"),
            include_str!("../migrations/20260852_route_endpoint_scope.sql"),
            include_str!("../migrations/20260853_route_endpoint_capacity.sql"),
        ] {
            for col in ["input_price", "output_price", "model_prices", "rate ", "billing_mode"] {
                assert!(
                    !mig.contains(&format!("ADD COLUMN IF NOT EXISTS {col}"))
                        && !mig.contains(&format!("    {col}")),
                    "出口表上出现了计价列 {col} —— 换出口就会换账单"
                );
            }
        }
    }

    /// 合并，不是覆盖。
    ///
    /// 出口窗口只列了这一家有的那几个模型。整份覆盖会把线路上别的模型的价**抹掉**，
    /// 而那个后果要等到别人用那个模型时才显现：突然一分钱不收。
    #[test]
    fn merging_prices_never_wipes_the_rest() {
        let s = src();
        let i = s.find("async fn merge_route_pricing(").expect("合并函数不见了");
        let body = &s[i..];
        assert!(
            body.contains("fn merge(base: &serde_json::Value, patch: &serde_json::Value)"),
            "不是合并了 —— 整份覆盖会把线路上别的模型的价抹掉"
        );
        assert!(
            body.contains("out.remove(k)"),
            "没有删除语义：想去掉某个模型的价就只能留一个空对象在那儿"
        );
    }

    /// 先合并定价，再做价格闸校验。
    ///
    /// 顺序反了的话，新模型的价明明就在这次请求里，却因为「查不到价」被拒 ——
    /// 而报错还让人去线路那页填，那页根本没有这个模型。死循环。
    #[test]
    fn pricing_is_merged_before_the_price_gate_runs() {
        let s = src();
        let i = s.find("pub async fn admin_save(").expect("保存入口不见了");
        let body = &s[i..];
        let merged = body.find("merge_route_pricing(&state, &mut route, &req)").expect("没合并定价");
        let gate = body.find("!allowed.contains(m) && !priceable(&route, m)").expect("价格闸不见了");
        assert!(
            merged < gate,
            "价格闸跑在合并之前：这次填的价还没落库，新模型必然被判成「查不到价」"
        );
    }

    /// 保存出口时，新模型必须先有价格。
    #[test]
    fn saving_an_unpriceable_new_model_is_rejected() {
        let s = src();
        let i = s.find("pub async fn admin_save(").expect("保存入口不见了");
        let body = &s[i..];
        assert!(
            body.contains("!allowed.contains(m) && !priceable(&route, m)"),
            "价格闸没了 —— 出口能开放一个用户不付钱、你照付的模型"
        );
    }

    /// 不填模型 = 承载线路的全部。不填时行为必须和加这个功能之前一模一样。
    #[test]
    fn an_endpoint_with_no_model_list_serves_everything() {
        let mut r = model("anthropic");
        r.enabled_models = vec!["a".into(), "b".into()];
        let mut map = HashMap::new();
        map.insert(r.id, vec![ep(0.2, Some(true), "https://all.example.com")]);
        for want in ["a", "b"] {
            let urls: Vec<String> = expand(&[r.clone()], &map, want)
                .into_iter()
                .map(|m| m.base_url)
                .collect();
            assert_eq!(urls[0], "https://all.example.com", "模型 {want} 漏了这个出口");
        }
    }

    /// 出口可以用另一种协议，而且换协议不能顺带换掉别的。
    #[test]
    fn an_endpoint_can_speak_a_different_protocol() {
        let r = model("anthropic");
        let mut e = ep(0.2, Some(true), "https://openai-style.example.com");
        e.protocol = "openai".into();
        let mut map = HashMap::new();
        map.insert(r.id, vec![e]);
        let out = expand(&[r.clone()], &map, "claude-opus-5");
        let relay = out.iter().find(|m| m.base_url.contains("openai-style")).unwrap();
        assert_eq!(relay.protocol, "openai", "出口的协议没生效");
        assert_eq!(relay.input_price, r.input_price, "换协议顺带改了价");
        assert_eq!(relay.id, r.id, "换协议顺带换了线路身份");
        // 线路自带的那份不受影响。
        let own = out.iter().find(|m| m.base_url.contains("own")).unwrap();
        assert_eq!(own.protocol, "anthropic");
    }

    /// 协议只认两个值。
    #[test]
    fn protocol_is_one_of_two_words() {
        assert_eq!(clean_protocol("").ok(), Some(String::new()));
        assert_eq!(clean_protocol(" Anthropic ").ok(), Some("anthropic".into()));
        assert_eq!(clean_protocol("openai").ok(), Some("openai".into()));
        // 不认识的值会一路带到发请求那步，走进「不是 anthropic 就当 openai」的分支，
        // 拼出一个 /chat/completions 打给只认 /v1/messages 的上游，报一个看不懂的 404。
        assert!(clean_protocol("gemini").is_err());
        assert!(clean_protocol("anthropic-v2").is_err());
    }

    /// 全选要归一成空。
    ///
    /// 不归一的话，线路以后加了新模型，这个出口会停在保存那天的名单上 —— 而运维
    /// 当初勾的是「全部」，不是「这七个」。
    #[test]
    fn selecting_everything_is_stored_as_empty() {
        let s = src();
        let i = s.find("pub async fn admin_save(").expect("保存入口不见了");
        let body = &s[i..];
        assert!(
            body.contains("if is_exactly_the_routes_own {"),
            "全选没有归一成空 —— 线路以后加了新模型，这个出口会停在保存那天的名单上"
        );
    }

    #[test]
    fn inactive_endpoints_are_not_expanded() {
        let r = model("anthropic");
        let mut e = ep(0.1, Some(true), "https://off.example.com");
        e.active = false;
        let mut map = HashMap::new();
        map.insert(r.id, vec![e]);
        let out = expand(&[r], &map, "claude-opus-5");
        assert_eq!(out.len(), 1);
        assert!(out[0].base_url.contains("own"));
    }

    #[test]
    fn ratio_must_be_a_discount() {
        assert!(clean_ratio(0.3).is_ok());
        assert!(clean_ratio(1.0).is_ok());
        // 小数点点错成 10（想写 1.0）不能进库：它会让这个出口永远排最后，
        // 而运维以为自己配的是「十倍便宜」。
        assert!(clean_ratio(10.0).is_err());
        assert!(clean_ratio(0.0).is_err());
        assert!(clean_ratio(-1.0).is_err());
        assert!(clean_ratio(f64::NAN).is_err());
        // NaN 尤其要挡：它参与排序时所有比较都返回 false，会让次序变成
        // 「取决于库里的行序」——一个看起来随机、永远查不出来的 bug。
    }

    /// 前端真实发出的那个载荷，必须能反序列化。
    ///
    /// 这一条钉的是一个真实故障：控制台点「保存」连着 5 次 400，而 `admin_save` 自己的
    /// 任何一条校验文案都对不上响应体长度 —— 说明请求**在进入处理函数之前**就被
    /// 提取器拒了。这种 400 不进错误日志、文案是英文的 serde 报错，最难查。
    ///
    /// 载荷逐字抄自 admin-ui 的 save()：新建时 `id` 是 undefined，JSON.stringify 会
    /// 把这个键整个丢掉。
    #[test]
    fn the_exact_payload_the_console_sends_deserialises() {
        // 新建：没有 id，capacity 是 null
        let create = serde_json::json!({
            "route_id": "11111111-1111-1111-1111-111111111111",
            "label": "转卖A",
            "base_url": "https://relay.example.com/v1",
            "api_key": "sk-test",
            "cost_ratio": 0.3,
            "note": "",
            "protocol": "",
            "active": true,
            "enabled_models": ["claude-opus-5"],
            "capacity": serde_json::Value::Null,
        });
        let got = serde_json::from_value::<SaveReq>(create);
        assert!(got.is_ok(), "新建的载荷反序列化失败：{:?}", got.err().map(|e| e.to_string()));

        // 编辑：带 id，capacity 是个数
        let update = serde_json::json!({
            "id": "22222222-2222-2222-2222-222222222222",
            "route_id": "11111111-1111-1111-1111-111111111111",
            "label": "", "base_url": "https://relay.example.com/v1", "api_key": "",
            "cost_ratio": 1, "note": "", "protocol": "openai", "active": true,
            "enabled_models": [], "capacity": 600,
        });
        assert!(
            serde_json::from_value::<SaveReq>(update).is_ok(),
            "编辑的载荷反序列化失败"
        );

        // 最小载荷：只有必填的两个。其余都该有默认值，
        // 否则一个旧版前端就会把整条链路打成 400。
        let minimal = serde_json::json!({
            "route_id": "11111111-1111-1111-1111-111111111111",
            "base_url": "https://relay.example.com/v1",
        });
        let got = serde_json::from_value::<SaveReq>(minimal);
        assert!(got.is_ok(), "最小载荷反序列化失败：{:?}", got.err().map(|e| e.to_string()));
    }

    /// 每个字段被显式打成 `null` 时都不能把请求打死。
    ///
    /// 这一条钉的是「点保存没反应、服务端查不出原因」那类故障的根：`#[serde(default)]`
    /// 只管字段**缺失**，管不了值是 `null`。而前端里 `x ? f(x) : null`、以及
    /// `Number("abc")` 出 NaN 被 JSON.stringify 写成 `null`，随手就会产生一个显式 null。
    /// 那种请求在**进入处理函数之前**就被提取器拒了，报英文 serde 错、不进任何日志。
    #[test]
    fn an_explicit_null_on_any_field_never_kills_the_request() {
        let fields = [
            "id", "label", "base_url", "api_key", "cost_ratio", "active", "note",
            "enabled_models", "protocol", "capacity", "model_prices", "model_names",
        ];
        for f in fields {
            let mut v = serde_json::json!({
                "route_id": "11111111-1111-1111-1111-111111111111",
                "base_url": "https://relay.example.com/v1",
            });
            v[f] = serde_json::Value::Null;
            let got = serde_json::from_value::<SaveReq>(v);
            assert!(
                got.is_ok(),
                "字段 {f} 被打成 null 就解不出来了：{:?}",
                got.err().map(|e| e.to_string())
            );
        }
        // null 要落到「没填」，不是落到别的值上。
        let v = serde_json::json!({
            "route_id": "11111111-1111-1111-1111-111111111111",
            "base_url": "https://a.example.com/v1",
            "cost_ratio": serde_json::Value::Null,
            "active": serde_json::Value::Null,
            "protocol": serde_json::Value::Null,
        });
        let r = serde_json::from_value::<SaveReq>(v).expect("解不出来");
        assert_eq!(r.cost_ratio, 1.0, "null 折扣该落到默认的原价");
        assert!(r.active, "null 该落到「投入轮转」");
        assert_eq!(r.protocol, "", "null 协议该落到「跟线路一样」");
    }

    /// 请求体由处理函数自己解，不交给提取器。
    ///
    /// 交给 `Json<SaveReq>` 的话，解析失败是一句英文 serde 错，发生在这个函数之前 ——
    /// 服务端没有任何日志，运维只能看到一个 400。这一整类失败查不出来。
    #[test]
    fn the_handler_parses_the_body_itself_so_failures_are_visible() {
        let s = src();
        let i = s.find("pub async fn admin_save(").expect("保存入口不见了");
        let head = &s[i..s[i..].find("admin_only").map(|j| i + j).unwrap_or(s.len())];
        assert!(
            head.contains("body: axum::body::Bytes"),
            "又交回给提取器了：解析失败会变成一个查不出原因的 400"
        );
        let body = &s[i..];
        assert!(
            body.contains("请求格式不对："),
            "解析失败没有转成中文报错"
        );
    }

    #[test]
    fn url_must_be_http() {
        assert!(clean_url("https://a.example.com/v1/").is_ok());
        // AppError 没实现 Debug，所以不能 unwrap。
        assert_eq!(
            clean_url("https://a.example.com/v1/").ok().as_deref(),
            Some("https://a.example.com/v1"),
            "结尾的斜杠没削掉：拼出来会是 //messages",
        );
        assert!(clean_url("a.example.com").is_err());
        assert!(clean_url("file:///etc/passwd").is_err());
        assert!(clean_url("  ").is_err());
    }

    #[test]
    fn probe_never_reports_ok_on_a_bare_200() {
        // 这一条钉的是 model_probe.rs 里记着的那条教训：转卖网关会用 200 包错误页。
        let s = src();
        let i = s.find("pub async fn probe_once(").expect("探测函数不见了");
        let body = &s[i..];
        assert!(
            body.contains("looks_real"),
            "探测不再检查响应形状了——那就退回成「只要不报错就算好」"
        );
        assert!(
            body.contains(r#"v.get("content")"#) && body.contains(r#"v.get("choices")"#),
            "响应形状的判据被改窄了"
        );
    }

    #[test]
    fn the_probe_error_path_never_echoes_the_url() {
        // reqwest 的错误链带完整 URL，而有些转卖商要求把密钥写在查询串里。
        let s = src();
        let i = s.find("pub async fn probe_once(").expect("探测函数不见了");
        let body = &s[i..s[i..].find("fn probe_client").map(|j| i + j).unwrap_or(s.len())];
        assert!(
            !body.contains("{e}") && !body.contains("e.to_string()"),
            "把 reqwest 的错误原文放进了 note —— 那可能把密钥写进后台页面和日志"
        );
    }

    #[test]
    fn the_key_is_never_returned_to_the_browser() {
        let s = src();
        let i = s.find("pub struct EndpointOut").expect("出参结构不见了");
        // 按**结构体边界**切，不用定长窗口：这个结构会长，而定长窗口既会把新字段挤出
        // 检查范围（漏判），也会在中文注释上切到半个汉字里直接 panic —— 两种都发生过。
        let body = &s[i..s[i..].find("\n}").map(|j| i + j).unwrap_or(s.len())];
        assert!(body.contains("has_key"), "改成回密钥本身了");
        assert!(
            !body.contains("pub api_key"),
            "EndpointOut 带上了 api_key —— 后台页面也不该拿到密钥"
        );
    }

    #[test]
    fn saving_without_a_key_keeps_the_old_one() {
        // 「只改地址」的保存必须留住密钥，否则一次改地址就把出口打瘸，
        // 而错误要等到下一个请求打过去才出现。
        let s = src();
        let i = s.find("pub async fn admin_save(").expect("保存入口不见了");
        let body = &s[i..];
        assert!(
            body.contains("SELECT api_key FROM route_endpoints WHERE id = $1"),
            "不再先取旧密钥了"
        );
        assert!(
            body.contains("if req.api_key.trim().is_empty()"),
            "空密钥的分支没了 —— 会把密钥清空"
        );
    }

    #[test]
    fn keys_use_the_same_crypto_context_as_route_keys() {
        // 用另一个 context 存的话，密钥轮换会漏掉这张表，而症状是「某天所有多路由同时挂」。
        let s = src();
        // 按判据数，不按次数数：每一处加密都得带这个 context，一处都不能漏。
        assert_eq!(
            s.matches("crate::field_crypto::encrypt(").count(),
            s.matches("crate::models::MODEL_KEY_CTX").count(),
            "有加密调用没带线路那套 context —— 密钥轮换会漏掉这张表，\
             症状是「某天所有多路由同时挂」"
        );
        assert!(
            s.contains("crate::field_crypto::encrypt("),
            "端点密钥不再加密存了"
        );
        // 解密只许走 model_key（它内部就是这个 context），不许自己再拼一个。
        assert!(
            s.contains("crate::models::model_key(") && !s.contains("field_crypto::decrypt("),
            "端点密钥的解密绕开了 model_key"
        );
        assert!(
            !s.contains("route_endpoints.api_key\""),
            "给端点密钥另起了一个加密 context"
        );
    }

    /// 「还能不能服务」的排序：好的在前，`unknown` 必须排在 `error` 前面。
    ///
    /// 反了会同时造出两种错：把没人用过的出口当成坏的报警（告警疲劳，正是上次事故里
    /// 没人看告警的成因），或者把真坏了的出口当成不知道而不报警。
    /// 厂商判定要认得出线上那七条线路。
    ///
    /// 这七组是从生产库里抄出来的真值，不是我编的形状 —— 其中两条（智谱、Grok）的
    /// `provider` 列填的都是 `other`，正是「不能信那一列」的证据。
    #[test]
    fn the_real_routes_are_all_recognised() {
        let cases: [(&str, &[&str], &str); 7] = [
            ("claude", &["claude-opus-5", "claude-fable-5"], "anthropic"),
            ("gpt", &["gpt-5.6-sol", "gpt-5.6-terra"], "openai"),
            ("deepseek", &["deepseek-v4-flash"], "deepseek"),
            // provider = other，只能靠模型 id 认出来。
            ("other", &["glm-5.2", "glm-5.3"], "zhipu"),
            ("other", &["grok-4.6", "grok-4.5"], "xai"),
            // 自起的名字，模型名什么都说明不了 —— 这一条靠域名兜底，见下面那个测试。
            ("other", &["stealth/ox-alpha"], ""),
            ("", &[], ""),
        ];
        for (provider, models, want) in cases {
            let owned: Vec<String> = models.iter().map(|s| s.to_string()).collect();
            assert_eq!(
                vendor_of(provider, &owned, ""),
                want,
                "provider={provider} models={models:?}"
            );
        }
    }

    /// 一大批真实模型 id 各自该认成谁。
    ///
    /// 这张表钉的是**次序**。厂商表是从上往下匹配的，加一条短词就可能把后面几家全抢走，
    /// 而症状只是「某条线路的图标变了」—— 没有任何测试会自然发现。所以每加一家，
    /// 就往这里加一行。
    #[test]
    fn a_pile_of_real_model_ids_map_to_the_right_vendor() {
        let cases: &[(&str, &str)] = &[
            ("claude-opus-5", "anthropic"),
            // AWS 上的 Claude：id 带 anthropic. 前缀，认成 Claude 才对，不是 bedrock。
            ("anthropic.claude-3-5-sonnet-v2", "anthropic"),
            ("gpt-5.6-sol", "openai"),
            ("o3-mini", "openai"),
            ("deepseek-v4-pro", "deepseek"),
            ("glm-5.3", "zhipu"),
            ("grok-4.6", "xai"),
            ("gemini-3-pro", "google"),
            ("gemma-3-27b", "google"),
            ("qwen3-max", "qwen"),
            ("qwq-32b", "qwen"),
            ("kimi-k2", "moonshot"),
            ("moonshot-v1-128k", "moonshot"),
            ("llama-4-scout", "meta"),
            ("mistral-large-2411", "mistral"),
            ("abab6.5s-chat", "minimax"),
            ("minimax-m2", "minimax"),
            ("baichuan4-turbo", "baichuan"),
            ("hunyuan-turbos", "hunyuan"),
            ("doubao-pro-32k", "doubao"),
            ("ernie-4.5-turbo", "wenxin"),
            ("internlm3-8b", "internlm"),
            ("sensechat-5", "sensenova"),
            ("skywork-13b", "skywork"),
            ("command-r-plus", "cohere"),
            ("jamba-1.5-large", "ai21"),
            ("sonar-pro", "perplexity"),
            ("nemotron-4-340b", "nvidia"),
            ("phi-4", "microsoft"),
            ("yi-lightning", "zeroone"),
            ("sparkdesk-v4", "spark"),
            ("step-2-16k", "stepfun"),
        ];
        for (id, want) in cases {
            assert_eq!(
                vendor_of("", &[id.to_string()], ""),
                *want,
                "{id} 认错了厂商"
            );
        }
    }

    /// 每一家都得有图标，否则判定认出来了、界面还是画中性图。
    ///
    /// 两边是两个文件（Rust 的厂商表、前端的图标表），加一家很容易只加一边 ——
    /// 而漏的那一边不会报错，只是图标默默变回灰色。
    #[test]
    fn every_vendor_has_an_icon_on_the_front_end() {
        let icons = include_str!("../admin-ui/src/components/VendorMark.tsx");
        let mut missing = Vec::new();
        for (_, vendor) in NEEDLES.iter().chain(HOSTS.iter()) {
            if !icons.contains(&format!("\n  {vendor}: {{")) {
                missing.push(*vendor);
            }
        }
        missing.sort_unstable();
        missing.dedup();
        assert!(missing.is_empty(), "这些厂商在前端没有图标：{missing:?}");
    }

    /// 模型名认不出来时，看这条线路指向哪儿。
    ///
    /// 线上「牛来」那条就是这个形状：模型 id 是 `stealth/ox-alpha`（自起的名字，
    /// 什么都说明不了），而它的地址是 openrouter.ai —— 那才是唯一有信息量的东西。
    #[test]
    fn the_address_is_the_fallback_when_the_model_name_says_nothing() {
        assert_eq!(
            vendor_of("other", &["stealth/ox-alpha".into()], "https://openrouter.ai/api/v1"),
            "openrouter"
        );
        assert_eq!(vendor_of("", &[], "https://api.siliconflow.cn/v1"), "siliconcloud");
        assert_eq!(vendor_of("", &[], "http://localhost:11434/v1"), "ollama");
        // 两边都认不出来，还是不猜。
        assert_eq!(vendor_of("", &["mystery".into()], "https://relay.example.com"), "");
    }

    /// 模型比管道重要：地址只在模型名说不清时才轮到。
    ///
    /// 一条指向 openrouter 但跑 claude-opus 的线路该显示 Claude —— 运维想知道的是
    /// 「这条线路卖的是谁家的模型」，不是「从哪个中间商买的」。次序反了就全反了。
    #[test]
    fn the_model_wins_over_the_pipe() {
        assert_eq!(
            vendor_of("", &["claude-opus-5".into()], "https://openrouter.ai/api/v1"),
            "anthropic"
        );
        assert_eq!(
            vendor_of("", &["deepseek-v4".into()], "https://api.siliconflow.cn/v1"),
            "deepseek"
        );
    }

    /// 不认识就回空串，绝不猜。
    ///
    /// 给一条智谱线路画上 OpenAI 的标，比不画标糟得多：不画只是朴素，画错是错误信息，
    /// 而运维扫一眼图标就以为自己看懂了这条线路是谁家的。
    #[test]
    fn an_unknown_vendor_is_never_guessed() {
        assert_eq!(vendor_of("some-reseller", &["mystery-model-v9".into()], ""), "");
        assert_eq!(vendor_of("other", &["ox-alpha".into()], ""), "");
    }

    fn uid() -> uuid::Uuid {
        uuid::Uuid::from_u128(7)
    }

    /// 粘性键每一级都必须带 uid。
    ///
    /// run id 是客户端给的。不掺 uid 的话，两个用户完全可以撞同一个 run id，
    /// 被钉在同一个出口上 —— 那正好是这个键要避免的事。
    #[test]
    fn the_sticky_key_always_carries_the_uid() {
        let a = uuid::Uuid::from_u128(1);
        let b = uuid::Uuid::from_u128(2);
        let same_scope = [Some("run-abcdefgh")];
        assert_ne!(
            sticky_key(&a, &same_scope, b"salt"),
            sticky_key(&b, &same_scope, b"salt"),
            "两个用户带同一个 run id 得到了同一个键"
        );
        // 没有任何 scope 时也要能用（只靠 uid），不能退化成常量。
        assert_ne!(sticky_key(&a, &[None], b"salt"), sticky_key(&b, &[None], b"salt"));
        // 盐要真的起作用。
        assert_ne!(sticky_key(&a, &[None], b"s1"), sticky_key(&a, &[None], b"s2"));
    }

    /// 不合法的 scope 要掉级，而不是原样进哈希。
    ///
    /// 客户端那道白名单不合法时是静默不发，所以网关收到什么都有可能。一个带空格或
    /// 超长的值混进去，效果等同于「这个用户每次换一个键」—— 粘性直接没了，
    /// 而且不会有任何报错。
    #[test]
    fn a_malformed_scope_falls_through_instead_of_poisoning_the_key() {
        let u = uid();
        let bare = sticky_key(&u, &[None], b"salt");
        for bad in ["", "  ", "short", "has space", &"x".repeat(200), "semi;colon"] {
            assert_eq!(
                sticky_key(&u, &[Some(bad)], b"salt"),
                bare,
                "不合法的 scope「{bad}」没有掉级"
            );
        }
        // 合法的要真的参与。
        assert_ne!(sticky_key(&u, &[Some("run-abcdefgh")], b"salt"), bare);
    }

    /// 同一个键永远挑到同一个出口。
    #[test]
    fn one_conversation_always_lands_on_the_same_endpoint() {
        let pool: Vec<(uuid::Uuid, f64, f64)> = (1..=4)
            .map(|i| (uuid::Uuid::from_u128(i), 0.2 * i as f64, 1.0))
            .collect();
        let k = sticky_key(&uid(), &[Some("run-abcdefgh")], b"salt");
        let first = hrw_pick(&k, &pool).unwrap();
        for _ in 0..200 {
            assert_eq!(hrw_pick(&k, &pool).unwrap(), first, "同一个键挑出了不同的出口");
        }
    }

    /// 移走一个**没被选中**的出口，选择不变。
    ///
    /// 这是加权 rendezvous 相对「按权重划分区间」的关键优势：集合变化时扰动最小。
    /// 区间划分会让所有人集体平移 —— 也就是所有人的上游缓存同时作废。
    #[test]
    fn removing_an_unchosen_endpoint_moves_nobody() {
        let pool: Vec<(uuid::Uuid, f64, f64)> = (1..=5)
            .map(|i| (uuid::Uuid::from_u128(i), 0.15 * i as f64, 1.0))
            .collect();
        let mut unchanged = 0;
        for n in 0..400u128 {
            let k = sticky_key(&uuid::Uuid::from_u128(n), &[None], b"salt");
            let chosen = pool[hrw_pick(&k, &pool).unwrap()].0;
            // 去掉一个不是它选中的出口
            let drop = pool.iter().find(|(id, _, _)| *id != chosen).unwrap().0;
            let smaller: Vec<_> = pool.iter().filter(|(id, _, _)| *id != drop).cloned().collect();
            if smaller[hrw_pick(&k, &smaller).unwrap()].0 == chosen {
                unchanged += 1;
            }
        }
        assert_eq!(unchanged, 400, "移走一个没被选中的出口，却有人被迫换了地方");
    }

    /// 溢出时按权重铺开，便宜的分得多 —— 但不是全拿。
    #[test]
    fn overflow_spreads_by_price_not_winner_take_all() {
        // 三折 vs 六折：γ=2 → 权重比 (1/0.3)² : (1/0.6)² = 4:1
        let pool = vec![
            (uuid::Uuid::from_u128(11), 0.3, 1.0),
            (uuid::Uuid::from_u128(22), 0.6, 1.0),
        ];
        let mut cheap = 0;
        const N: u128 = 4000;
        for n in 0..N {
            let k = sticky_key(&uuid::Uuid::from_u128(n), &[None], b"salt");
            if pool[hrw_pick(&k, &pool).unwrap()].0 == uuid::Uuid::from_u128(11) {
                cheap += 1;
            }
        }
        let share = cheap as f64 / N as f64;
        assert!(
            (0.76..0.84).contains(&share),
            "三折应拿到约 80%（4:1），实际 {share:.3} —— 权重函数被改动了"
        );
    }

    /// 垃圾进价不能让排序 panic。
    ///
    /// Rust 1.81 起，不自洽的比较器是 **panic** 而不是乱序 —— 一个 NaN 进到权重里，
    /// 整个网关的选路会直接崩。
    #[test]
    fn garbage_prices_never_panic_the_picker() {
        let pool = vec![
            (uuid::Uuid::from_u128(1), f64::NAN, 1.0),
            (uuid::Uuid::from_u128(2), 0.0, 1.0),
            (uuid::Uuid::from_u128(3), -1.0, 1.0),
            (uuid::Uuid::from_u128(4), f64::INFINITY, 1.0),
            (uuid::Uuid::from_u128(5), 0.5, 1.0),
        ];
        let k = sticky_key(&uid(), &[None], b"salt");
        let got = hrw_pick(&k, &pool);
        assert!(got.is_some(), "全是垃圾价时没挑出任何出口");
        for bad in [f64::NAN, 0.0, -1.0, f64::INFINITY] {
            // 价格和容量两边各喂一遍垃圾，都不许产出非有限权重。
            assert!(overflow_weight(bad, 1.0).is_finite() && overflow_weight(bad, 1.0) > 0.0);
            assert!(overflow_weight(0.5, bad).is_finite() && overflow_weight(0.5, bad) > 0.0);
        }
        assert!(hrw_pick(&k, &[]).is_none(), "空集合应当回 None");
    }

    /// 没填容量的按池内最小值兜底，不是按 1。
    ///
    /// 按 1 的话，「一个填了 600、一个没填」会差六百倍 —— 运维只是没填，
    /// 却等于把那个出口关掉了，而且完全看不出来。
    #[test]
    fn an_undeclared_capacity_falls_back_to_the_smallest_declared_one() {
        assert_eq!(fill_capacities(&[None, None]), vec![1.0, 1.0], "全没填该一律 1");
        assert_eq!(
            fill_capacities(&[Some(600.0), None]),
            vec![600.0, 600.0],
            "只有一个填了，没填的该跟它齐平，而不是掉到 1"
        );
        assert_eq!(
            fill_capacities(&[Some(600.0), Some(20.0), None]),
            vec![600.0, 20.0, 20.0],
            "没填的该按已填里的最小值 —— 不知道能扛多少就当它最不能扛"
        );
        // 垃圾值当成没填。
        assert_eq!(
            fill_capacities(&[Some(f64::NAN), Some(50.0), Some(-3.0)]),
            vec![50.0, 50.0, 50.0]
        );
    }

    /// 容量真的参与溢出分配。
    #[test]
    fn a_bigger_endpoint_takes_more_of_the_overflow() {
        // 同价，容量 10:1 → 份额也该接近 10:1
        let pool = vec![
            (uuid::Uuid::from_u128(11), 0.5, 10.0),
            (uuid::Uuid::from_u128(22), 0.5, 1.0),
        ];
        let mut big = 0;
        const N: u128 = 4000;
        for n in 0..N {
            let k = sticky_key(&uuid::Uuid::from_u128(n), &[None], b"salt");
            if pool[hrw_pick(&k, &pool).unwrap()].0 == uuid::Uuid::from_u128(11) {
                big += 1;
            }
        }
        let share = big as f64 / N as f64;
        assert!(
            (0.87..0.95).contains(&share),
            "容量 10:1 应拿到约 91%，实际 {share:.3} —— 容量没进权重"
        );
    }

    /// 承接只在启动时做，派单路径上一个 await 都不许加。
    ///
    /// 这是整套设计的地基：让位判定必须是纯内存的一次哈希 + 一把短锁。往里加一次
    /// Redis 往返，几千 QPS 下就是几千次网络等待 —— 而这个功能的目的正是不卡顿。
    #[test]
    fn saturation_is_restored_at_boot_never_on_the_dispatch_path() {
        let s = src();
        // 写是火后不管（tokio::spawn），不阻塞请求。
        let i = s.find("pub fn persist_saturation(").expect("持久化函数不见了");
        let body = &s[i..s[i..].find("\n/// 启动时").map(|j| i + j).unwrap_or(s.len())];
        assert!(
            body.contains("tokio::spawn("),
            "落 Redis 变成同步的了 —— 每个 429 都会让用户多等一次网络往返"
        );
        assert!(
            body.contains("EX"),
            "没设 TTL：键会永远留着，出口再也回不来"
        );
        // 读只在启动。派单路径（chat_completions）里不许出现承接调用。
        let gw = include_str!("models.rs");
        let prod = gw.split("\n#[cfg(test)]").next().unwrap();
        assert!(
            !prod.contains("restore_saturation("),
            "承接跑进派单路径了 —— 那是一次 SCAN，绝不能在每个请求上做"
        );
    }

    /// 承接读的是**剩余** TTL，不是当初写进去的时长。
    #[test]
    fn restore_uses_the_remaining_ttl_not_the_original_window() {
        let s = src();
        let i = s.find("pub async fn restore_saturation(").expect("承接函数不见了");
        let body = &s[i..];
        assert!(
            body.contains("redis::cmd(\"TTL\")"),
            "没读剩余 TTL：一个写的时候是 300 秒、已经走了 290 秒的键，会被当成又要让位 300 秒"
        );
        assert!(
            body.contains("redis::cmd(\"SCAN\")") && !body.contains("redis::cmd(\"KEYS\")"),
            "用了 KEYS —— 它会阻塞整个 Redis，而这台机器上 Redis 还扛着会话和健康数据"
        );
    }

    /// 权重里不许出现任何健康信号。
    ///
    /// 健康是阶跃量（探测是单样本 0/1、进程内记号发版后全空）。折进连续权重的话，
    /// 一次抖动就让过半在途对话集体迁走 —— 而粘性存在的意义正是防这件事。
    /// 排除坏出口是**排除**，在选完之后那道重排里做，不是降权。
    #[test]
    fn the_overflow_weight_never_looks_at_health() {
        let s = src();
        let i = s.find("pub fn overflow_weight(").expect("权重函数不见了");
        let body = &s[i..s[i..].find("\npub fn hrw_pick").map(|j| i + j).unwrap_or(s.len())];
        for banned in [
            "route_cooldown_remaining",
            "route_recently_stalled",
            "route_mutes_thinking",
            "probe_ok",
            "route_health::",
        ] {
            assert!(
                !body.contains(banned),
                "权重里读了 {banned} —— 阶跃信号折进连续权重会让粘性在抖动时集体失效"
            );
        }
    }

    /// 哈希必须是 SHA-256。
    #[test]
    fn the_hash_is_stable_across_rust_versions() {
        // 先剥注释：这个文件的注释里就写着「不能用 DefaultHasher」，不剥的话
        // 断言会被说明文字喂绿（或者像这次一样，被喂红）。
        let s: String = src()
            .lines()
            .map(|l| {
                let t = l.trim_start();
                if t.starts_with("//") { "" } else { l }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !s.contains("DefaultHasher"),
            "用了 DefaultHasher —— Rust 保留换算法的权利，换一次全网粘性静默清零且不报错"
        );
        // 按判据数，不按次数数：两个函数各自都得用 SHA-256。
        for (name, end) in [
            ("pub fn sticky_key(", "\nfn normalise_scope"),
            ("pub fn hrw_pick(", "\n/// 取一批线路的出口"),
        ] {
            let i = s.find(name).unwrap_or_else(|| panic!("{name} 不见了"));
            let body = &s[i..s[i..].find(end).map(|j| i + j).unwrap_or(s.len())];
            assert!(body.contains("Sha256"), "{name} 没用 SHA-256");
        }
    }

    #[test]
    fn serving_rank_puts_no_evidence_before_bad_evidence() {
        assert!(serve_rank("ok") < serve_rank("degraded"));
        assert!(serve_rank("degraded") < serve_rank("unknown"));
        assert!(serve_rank("unknown") < serve_rank("error"));
        // 词表之外的一律当最坏，不会被当成绿的。
        assert_eq!(serve_rank("随便什么"), serve_rank("error"));
    }

    /// 聚合出来的词必须还在 `route_health::classify` 那套词表里。
    ///
    /// 自己编一个新词（比如 "bad"）不会报错，只会让面板和告警落进各自的 `_ =>` 分支：
    /// 一个显示成灰点，一个当成「不是 error」而永远不报警。
    #[test]
    fn the_aggregate_never_invents_a_new_word() {
        let s = src();
        let i = s.find("pub async fn best_word(").expect("聚合函数不见了");
        let body = &s[i..s[i..].find("\n/// 面板上那一格").map(|j| i + j).unwrap_or(s.len())];
        assert!(
            !body.contains("\"bad\"") && !body.contains("\"down\""),
            "聚合造了一个 classify 里没有的词"
        );
        assert!(
            body.contains("crate::route_health::classify("),
            "聚合不再用 classify 定词了 —— 两套词表迟早对不上"
        );
    }

    /// 告警必须把多路由出口算进去。
    ///
    /// 健康是按出口记的，而流量大多走最便宜那个出口。告警要是只看线路自带地址的记录，
    /// 出口连败就永远进不了告警 —— 面板全绿、监控一次没响，正是这台机器出过的那次事故。
    #[test]
    fn the_alarm_sees_endpoint_failures_too() {
        let health = include_str!("route_health.rs");
        let prod = health.split("\n#[cfg(test)]").next().unwrap();
        assert!(
            prod.contains("crate::route_endpoints::best_word(&state, m.id, now_secs())"),
            "告警又只看线路自带地址了：挂在它下面的出口全坏光也不会有人知道"
        );
        assert!(
            !prod.contains("let word = classify(&h, now_secs());"),
            "告警绕开聚合，直接拿线路自己那条记录定词了"
        );
    }

    #[test]
    fn the_background_sweep_skips_endpoints_real_traffic_already_proved() {
        let s = src();
        let i = s.find("async fn sweep(").expect("轮次函数不见了");
        let body = &s[i..];
        assert!(
            body.contains(r#"classify(&health, now) == "ok""#),
            "自动探测不再跳过真实流量证明过的出口 —— 白烧 token，还占上游限流额度"
        );
    }

    /// 前端在「加一个出口」里发的那份 JSON，后端必须原样解得出来。
    ///
    /// 这条是踩出来的，不是防御性的：出口保存连着 5 次 400，而 400 是 axum 的
    /// **提取器**在进 handler 之前吐的 —— handler 里一行日志都不会打，网关日志干干净净，
    /// 从服务端完全看不出发生过什么。查了很久才定位到「前端多发/少发了一个字段」这一类。
    ///
    /// 所以判据要跨语言对齐：直接读前端源码里那个对象字面量的键，逐个查后端认不认。
    /// 手写一份期望清单没用 —— 它和真正被发出去的东西是两个东西，会各自漂移。
    #[test]
    fn 前端发的每个字段后端都认识() {
        let ui = include_str!("../admin-ui/src/pages/RouteEndpoints.tsx");
        // 定位真正的保存调用，而不是同文件里别的 post。
        let at = ui
            .find(r#""/api/admin/route-endpoints",
        {"#)
            .expect("保存调用的形状变了 —— 这条测试已经不在看真正发出去的东西了");
        let body = &ui[at..];
        let end = body.find("\n      );").expect("找不到调用的收尾");
        let body = &body[..end];

        // 对象字面量的顶层键：行首缩进恰好 10 空格的 `名字:`。
        let sent: Vec<&str> = body
            .lines()
            .filter_map(|l| {
                let k = l.strip_prefix("          ")?;
                if k.starts_with(' ') || k.starts_with("//") {
                    return None;
                }
                let name = k.split(':').next()?.trim();
                (!name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()))
                .then_some(name)
            })
            .collect();
        assert!(
            sent.len() >= 12,
            "只认出 {} 个字段（{sent:?}）—— 解析规则和前端排版对不上了，\
             这时候测试会**恒真**，比失败还危险",
            sent.len()
        );

        // 后端认识的字段：SaveReq 里的 `pub 名字:`。
        let me = include_str!("route_endpoints.rs");
        let sat = me.find("pub struct SaveReq {").expect("SaveReq 改名了");
        let sblock = &me[sat..sat + me[sat..].find("\n}").expect("SaveReq 没有收尾")];
        let known: Vec<&str> = sblock
            .lines()
            .filter_map(|l| l.trim().strip_prefix("pub "))
            .filter_map(|l| l.split(':').next())
            .map(str::trim)
            .collect();
        assert!(known.len() >= 12, "SaveReq 的字段没解出来：{known:?}");

        let unknown: Vec<&&str> = sent.iter().filter(|k| !known.contains(k)).collect();
        assert!(
            unknown.is_empty(),
            "前端发了后端不认识的字段：{unknown:?}。\n\
             serde 默认会忽略多余字段，所以这**不一定**报错 —— 更常见的是静默丢掉，\
             用户填了东西、保存成功、结果没生效。"
        );
    }

    /// 新建出口那一发（没有 id、capacity 显式 null、两个空对象）必须能解开。
    ///
    /// `#[serde(default)]` 治的是「字段缺失」，治不了「字段是 null」—— 这两件事在
    /// serde 里是两条不同的路径，而前端 `capacity: x.trim() ? Number(x) : null`
    /// 发的恰好是后者。
    #[test]
    fn 新建出口的那份请求体解得开() {
        let rid = uuid::Uuid::new_v4();
        let raw = format!(
            r#"{{"route_id":"{rid}","label":"","base_url":"https://x.com/v1",
               "api_key":"sk-1","cost_ratio":1,"note":"","protocol":"",
               "active":true,"enabled_models":[],"capacity":null,
               "model_prices":{{}},"model_names":{{}}}}"#
        );
        let got: SaveReq = serde_json::from_str(&raw)
            .expect("新建出口的请求体解不开 —— 这就是那 5 次 400 的形状");
        assert_eq!(got.route_id, rid);
        assert_eq!(got.id, None, "没带 id 应该当成新建");
        assert_eq!(got.cost_ratio, 1.0);
        assert!(got.active);
        assert_eq!(got.capacity, None, "显式 null 必须当成没填，而不是解析失败");
    }

    /// 每个可空字段单独喂 null，逐个确认。
    ///
    /// 上面那条只覆盖了「前端今天恰好这么发」。前端改一行、或者中间层把空串规整成
    /// null，就会换成别的组合 —— 而每一种组合都是一次 400，症状还是同一个「点了没反应」。
    #[test]
    fn 任何一个字段是null都不会让请求失败() {
        let rid = uuid::Uuid::new_v4();
        let nullable = [
            "id",
            "label",
            "base_url",
            "api_key",
            "cost_ratio",
            "active",
            "note",
            "enabled_models",
            "protocol",
            "capacity",
            "model_prices",
            "model_names",
        ];
        for f in nullable {
            let raw = format!(r#"{{"route_id":"{rid}","{f}":null}}"#);
            let got: Result<SaveReq, _> = serde_json::from_str(&raw);
            assert!(
                got.is_ok(),
                "字段 `{f}` 是 null 就整发请求失败 —— 用户看到的是「点了没反应」，\
                 而 400 由提取器吐出，服务端不留任何日志"
            );
        }
        // 反面：route_id 是**唯一**必须有的字段，缺了就该失败。
        // 没有这一半的话，上面那圈断言用「什么都接受」也能全过。
        assert!(
            serde_json::from_str::<SaveReq>(r#"{"label":"x"}"#).is_err(),
            "route_id 都没有也能解开 —— 那这个结构体就不再校验任何东西了"
        );
    }
}
