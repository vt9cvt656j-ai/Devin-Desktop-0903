//! michael-compression — 让任意模型获得 1M / 2M / 5M 的有效上下文。
//!
//! # 它解决什么
//!
//! 模型的原生窗口是固定的（Claude 200K、GPT-5 400K、Gemini 1M）。这个模块坐在网关的
//! 聊天链路上：接受远超原生窗口的对话，把**较早的部分**压成摘要，只把摘要 + 最近的原文
//! 交给上游，从而对外呈现一个大得多的窗口。
//!
//! # 为什么必须是「压缩**缓存**」
//!
//! 朴素的做法（客户端现有的 `_compactHistoryIfHuge` 就是这样）是每次超限时把整段历史
//! 重新压一遍。那样每一轮都要为**同样的旧内容**重新付一次 LLM 费用，成本随对话长度线性
//! 增长——在 5M 这个量级上完全不可行。
//!
//! 这里的关键是 **前缀稳定分段（prefix-stable segmentation）**：分段边界从对话的
//! **开头**按 token 预算贪心切分，因此往末尾追加消息**永远不会改变已有段的内容**。段的
//! 缓存键是其内容的哈希，所以：
//!
//! - 第 N 轮压缩了段 0..k
//! - 第 N+1 轮只有新增内容形成新段 k+1，段 0..k 的摘要**直接命中缓存**
//!
//! 每轮的压缩成本因此正比于**新增内容**，而不是历史总量。这也是它能和上游的 prompt
//! caching 叠加的原因——两者都依赖同一个「稳定前缀」性质。
//!
//! # 不做什么
//!
//! - 不改写最近的对话：尾部若干消息始终逐字透传，模型的近期记忆不受损。
//! - 不猜测 token：估算器只用于**预算规划**，真实计费永远以上游返回的 usage 为准。

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// 对外提供的上下文档位：接受多少**原始输入** token。
///
/// 档位只决定「接受多少」，不决定「压多狠」——压缩比由目标模型的原生窗口反推。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    /// 1M 原始输入
    M1,
    /// 2M 原始输入
    M2,
    /// 5M 原始输入
    M5,
}

impl Tier {
    /// 该档位接受的原始输入 token 上限。
    pub fn max_input_tokens(self) -> usize {
        match self {
            Tier::M1 => 1_000_000,
            Tier::M2 => 2_000_000,
            Tier::M5 => 5_000_000,
        }
    }

    /// 档位的对外标识，用于请求头 / 计费记录 / 日志。
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::M1 => "1m",
            Tier::M2 => "2m",
            Tier::M5 => "5m",
        }
    }

    /// 解析对外标识。大小写不敏感，容忍 `1M` / `1m` / `1000k` 这类写法。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "1m" | "1000k" | "1000000" => Some(Tier::M1),
            "2m" | "2000k" | "2000000" => Some(Tier::M2),
            "5m" | "5000k" | "5000000" => Some(Tier::M5),
            _ => None,
        }
    }

    /// 全部档位，从小到大。供模型目录对外声明可用档位，以及测试遍历用。
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn all() -> [Tier; 3] {
        [Tier::M1, Tier::M2, Tier::M5]
    }
}

/// 会员档位 → 允许使用的最大上下文档位。
///
/// 压缩层每压一段都要真金白银打一次上游，所以它是**付费能力**，跟着会员走：
///
/// | 套餐 | 最大档位 |
/// |---|---|
/// | ultra / power | 5M |
/// | pro | 2M |
/// | basic / trial | 1M |
/// | 无套餐但有余额（按量付费） | 1M |
/// | 都没有 | 不可用 |
///
/// 返回 `None` 表示该用户完全不能用这个特性。注意聊天入口的额度闸门已经保证了调用者
/// 至少有余额或有效套餐，所以实践中只有"套餐过期且余额恰好耗尽"才会走到 `None`。
pub fn max_tier_for_plan(plan: &str, plan_active: bool, credits_cents: i64) -> Option<Tier> {
    if plan_active {
        return match plan {
            "ultra" | "power" => Some(Tier::M5),
            "pro" => Some(Tier::M2),
            "basic" | "trial" => Some(Tier::M1),
            // 未知的自定义套餐名：按最低档给，不猜它对应哪一级。
            _ => Some(Tier::M1),
        };
    }
    (credits_cents > 0).then_some(Tier::M1)
}

impl Tier {
    /// 档位序（越大越高），用于比较与钳位。
    fn rank(self) -> u8 {
        match self {
            Tier::M1 => 1,
            Tier::M2 => 2,
            Tier::M5 => 3,
        }
    }
}

/// 把请求的档位钳到会员允许的范围内。
///
/// 刻意**下调而不是拒绝**：一个长对话跑到一半才发现档位不够就直接 402，用户体验是灾难，
/// 而且他本来就该拿到"他付费买到的那部分"能力。实际生效的档位会记进日志与响应头。
pub fn clamp_tier(requested: Tier, allowed: Option<Tier>) -> Option<Tier> {
    let allowed = allowed?;
    Some(if requested.rank() <= allowed.rank() {
        requested
    } else {
        allowed
    })
}

/// 一个分段的 token 预算。
///
/// 段太小 → 摘要调用次数多、每次开销摊不平；段太大 → 单次压缩慢，且末尾未满的那一段
/// 迟迟不能定型、反复重压。20K 是这两者的折中：对 200K 原生窗口的模型，压缩后的前缀
/// 大约由 (raw/20K) 条摘要组成，每条摘要目标 ~600 token。
pub const SEGMENT_TOKENS: usize = 20_000;

/// 每段摘要的目标长度（token）。压缩比约 33:1。
pub const SEGMENT_SUMMARY_TOKENS: usize = 600;

/// 尾部始终逐字保留的 token 数。
///
/// 模型对「刚刚发生了什么」最敏感，把近期对话压掉会直接伤害续写质量，所以这部分永不压缩。
pub const VERBATIM_TAIL_TOKENS: usize = 32_000;

/// 规划时给原生窗口留的余量：模型还要写输出，且我们的 token 估算是近似值。
pub const WINDOW_SAFETY: f64 = 0.75;

/// 粗略 token 估算。
///
/// **只用于预算规划**，不用于计费。CJK 每字约 1 token，拉丁文约 4 字符 1 token；混合
/// 文本按字符分类加权，比 `len/4` 在中文场景准得多（中文按 len/4 会低估 4 倍，导致规划
/// 出来的上下文实际超出窗口）。
pub fn estimate_tokens(text: &str) -> usize {
    let mut cjk = 0usize;
    let mut other = 0usize;
    for ch in text.chars() {
        // CJK 统一表意文字、假名、谚文
        let c = ch as u32;
        let is_cjk = (0x4E00..=0x9FFF).contains(&c)
            || (0x3040..=0x30FF).contains(&c)
            || (0xAC00..=0xD7AF).contains(&c)
            || (0x3400..=0x4DBF).contains(&c);
        if is_cjk {
            cjk += 1;
        } else {
            other += 1;
        }
    }
    cjk + other.div_ceil(4)
}

/// 一条参与压缩规划的消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Msg {
    pub role: String,
    pub text: String,
    pub tokens: usize,
}

impl Msg {
    pub fn new(role: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let tokens = estimate_tokens(&text);
        Self {
            role: role.into(),
            text,
            tokens,
        }
    }
}

/// 一个前缀稳定的分段：消息下标区间 `[start, end)`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start: usize,
    pub end: usize,
    pub tokens: usize,
}

/// 把消息切成前缀稳定的分段。
///
/// **从头贪心累加**：只要当前段的 token 数达到 `segment_tokens` 就封段。这保证了
/// 「同一个对话前缀 → 同一组分段」，与后面还会追加什么无关——正是缓存能命中的前提。
///
/// 反过来说，任何**从末尾**往回切的方案（比如「保留最后 N 条」）都会让每一轮的分段边界
/// 整体平移，缓存永远打不中。这是这个函数唯一重要的性质。
///
/// 最后一个不满额的段也会被返回；调用方通过 `Plan` 决定它是否参与压缩（不满额的段通常
/// 留在逐字尾部，等它长满再定型，避免反复压缩一个还在增长的段）。
pub fn segment_messages(msgs: &[Msg], segment_tokens: usize) -> Vec<Segment> {
    let segment_tokens = segment_tokens.max(1);
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut acc = 0usize;
    for (i, m) in msgs.iter().enumerate() {
        acc += m.tokens;
        if acc >= segment_tokens {
            out.push(Segment {
                start,
                end: i + 1,
                tokens: acc,
            });
            start = i + 1;
            acc = 0;
        }
    }
    if start < msgs.len() {
        out.push(Segment {
            start,
            end: msgs.len(),
            tokens: acc,
        });
    }
    out
}

/// 一个段的缓存键。
///
/// 键里必须包含**压缩器模型**和**目标长度**：换了压缩模型或改了摘要长度，旧摘要就不再
/// 是这次请求想要的东西，必须重算而不是复用。
pub fn segment_cache_key(text: &str, compressor_model: &str, summary_tokens: usize) -> String {
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    let payload = (text, compressor_model, summary_tokens);
    payload.hash(&mut h1);
    0x9e37_79b9_7f4a_7c15u64.hash(&mut h2);
    payload.hash(&mut h2);
    format!("mc:v1:{:016x}{:016x}", h1.finish(), h2.finish())
}

/// 把一个段的消息拼成送去压缩的文本。
///
/// 带上角色前缀，摘要器才能分清「用户要求」和「助手做过的事」——这两类在摘要里的保留
/// 优先级完全不同。
pub fn segment_text(msgs: &[Msg], seg: &Segment) -> String {
    msgs[seg.start..seg.end]
        .iter()
        .map(|m| format!("[{}] {}", m.role, m.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 压缩规划：哪些段要压、哪些逐字保留。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// 需要压缩成摘要的段（按顺序）。
    pub compress: Vec<Segment>,
    /// 从这条消息开始逐字保留。
    pub verbatim_from: usize,
    /// 规划后预计送给上游的 token 数。
    pub projected_tokens: usize,
    /// 输入的原始 token 数。
    pub raw_tokens: usize,
}

/// 规划一次压缩。
///
/// - `native_window`：目标模型的原生上下文窗口
/// - `verbatim_tail_tokens`：尾部逐字保留的预算
///
/// 规则：从后往前累加逐字尾部，直到吃满 `verbatim_tail_tokens`；剩下的前缀按段压缩。
/// 若压缩后仍然超出窗口，则继续把最老的逐字消息也纳入压缩范围——但**永远至少保留最后
/// 一条消息逐字**，否则模型会收到一个没有当前问题的请求。
pub fn plan(
    msgs: &[Msg],
    native_window: usize,
    verbatim_tail_tokens: usize,
    segment_tokens: usize,
) -> Plan {
    let raw_tokens: usize = msgs.iter().map(|m| m.tokens).sum();
    let budget = ((native_window as f64) * WINDOW_SAFETY) as usize;

    // 没超窗口就什么都不做——压缩本身要花钱，能不压就不压。
    if msgs.is_empty() || raw_tokens <= budget {
        return Plan {
            compress: Vec::new(),
            verbatim_from: 0,
            projected_tokens: raw_tokens,
            raw_tokens,
        };
    }

    // 从末尾往回吃出逐字尾部，至少保留一条。
    let mut verbatim_from = msgs.len().saturating_sub(1);
    let mut tail = msgs[verbatim_from].tokens;
    while verbatim_from > 0 {
        let next = msgs[verbatim_from - 1].tokens;
        if tail + next > verbatim_tail_tokens {
            break;
        }
        tail += next;
        verbatim_from -= 1;
    }

    // 前缀按稳定边界分段。注意分段是对**整个消息序列**做的，所以边界与
    // verbatim_from 无关——这正是缓存能跨轮命中的原因。
    let all_segments = segment_messages(msgs, segment_tokens);
    let mut compress: Vec<Segment> = all_segments
        .into_iter()
        .filter(|s| s.end <= verbatim_from)
        .collect();

    // 落在 verbatim_from 之前、但被段边界截断的零头也要压掉，否则会漏内容。
    let covered = compress.last().map(|s| s.end).unwrap_or(0);
    if covered < verbatim_from {
        let tokens = msgs[covered..verbatim_from].iter().map(|m| m.tokens).sum();
        compress.push(Segment {
            start: covered,
            end: verbatim_from,
            tokens,
        });
    }

    let summary_cost = compress.len() * SEGMENT_SUMMARY_TOKENS;
    let projected = summary_cost + tail;

    Plan {
        compress,
        verbatim_from,
        projected_tokens: projected,
        raw_tokens,
    }
}

/// 送给压缩模型的系统提示。
///
/// 和客户端 `_compactHistoryIfHuge` 的提示词同源，但这里压的是**一个段**而不是整段
/// 历史，所以要求它保留可被后续段引用的锚点（文件路径、决定、未完成项）。
pub fn segment_compress_prompt(summary_tokens: usize) -> String {
    format!(
        "你是对话压缩引擎。下面是一段较早的对话片段（不是全部）。把它压成不超过约 {summary_tokens} token 的要点，供模型后续继续这个任务时参考。\n\n\
**必须保留：**\n\
• 用户在这段里提出的需求和明确要求（原话核心）\n\
• 每个被修改/创建/删除的文件路径 + 具体改了什么\n\
• 每个错误的根因和最终解法\n\
• 做出的技术决定与其理由\n\
• 这段结束时**尚未完成**的事项\n\n\
**可以丢弃：**\n\
• 工具的原始输出（只留结论）\n\
• 重复的探索和试错过程（折叠成一句）\n\
• 寒暄、确认性回复\n\n\
只输出要点本身，中文，分条，不要加标题或前言。"
    )
}

/// 前缀续传：把「已压缩的历史」留在网关，客户端只发一个引用。
///
/// # 为什么需要它
///
/// 只做段缓存还不够：**客户端每轮仍要把完整历史发过来**，网关才能分段。5M token 的对话
/// 是 17–25MB（实测：纯中文 17.2MB / 纯英文 23MB / 中英混合 25.3MB），而网关的 body 上限
/// 是 12 MiB。就算把上限提到 32MB，也意味着每一轮都要重传 20MB+ —— 在国内网络上每轮几
/// 分钟，等于不可用。
///
/// 突破口在于：**5M 的对话从来不是一次性攒出来的，它是一轮轮长出来的**。压好的摘要本来
/// 就躺在网关的 Redis 里，客户端没有任何理由把它们再传一遍。于是：
///
/// - 网关压完一批段后，签发一个 `PrefixRecord`，把这些段的键按顺序记下来
/// - 下一轮客户端只发 `mc_prefix` 引用 + **未被覆盖的那些消息**
/// - 网关按引用取回摘要，拼上新消息，继续压新长出来的段，再签发一个更长的引用
///
/// 线路体积因此正比于**新增内容**而不是对话总长——1M 和 50M 的每轮传输量是一样的。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrefixRecord {
    /// 签发给谁。**必须校验**：这是一个指向对话内容的 bearer 引用，拿到别人的 token 就
    /// 等于读到别人的历史摘要。
    pub uid: String,
    /// 组成该前缀的段缓存键，按对话顺序。
    pub segment_keys: Vec<String>,
    /// 这个前缀覆盖了原始消息序列的前多少条。客户端据此知道自己该从第几条开始发。
    pub covered_msgs: usize,
    /// 被覆盖部分的原始 token 数，仅用于日志与用量展示。
    pub raw_tokens: usize,
}

/// 前缀引用的存活时间。比单段摘要短：它是会话级的续传凭据，不是内容缓存。
pub const PREFIX_TTL_SECS: u64 = 7 * 24 * 3600;

/// 生成一个不可猜测的前缀引用。
pub fn new_prefix_token() -> String {
    format!("mcp_{}", uuid::Uuid::new_v4().simple())
}

/// 前缀引用在 Redis 里的键。
pub fn prefix_redis_key(token: &str) -> String {
    format!("mc:prefix:{token}")
}

/// 校验一个前缀引用是否属于该用户。
///
/// 分开成独立函数是为了让「越权访问别人的前缀」这条有测试可打。
pub fn prefix_belongs_to(record: &PrefixRecord, uid: &str) -> bool {
    record.uid == uid
}

/// 摘要在 Redis 里的存活时间。
///
/// 段是内容寻址的，所以过期只影响成本不影响正确性：过期后重算一次即可。30 天足以覆盖
/// 一个长期项目反复被续写的场景。
pub const SUMMARY_TTL_SECS: u64 = 30 * 24 * 3600;

/// 单次请求最多现算多少个新摘要。
///

/// 从 Redis 取一个段的摘要。
pub async fn cached_summary(
    redis: &mut redis::aio::ConnectionManager,
    key: &str,
) -> Option<String> {
    redis::cmd("GET")
        .arg(key)
        .query_async::<Option<String>>(redis)
        .await
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
}

/// 回填一个段的摘要。
pub async fn store_summary(redis: &mut redis::aio::ConnectionManager, key: &str, summary: &str) {
    let _: Result<(), redis::RedisError> = redis::cmd("SET")
        .arg(key)
        .arg(summary)
        .arg("EX")
        .arg(SUMMARY_TTL_SECS)
        .query_async(redis)
        .await;
}

/// 组装压缩后的消息序列。
///
/// 摘要以一条 `system` 消息注入，并明确告诉模型这是被压缩过的早期上下文——否则模型可能
/// 把摘要当成用户刚说的话。
/// 摘要注入用的 system 文本。`None` 表示没有摘要要注入。
///
/// 单独抽出来是为了让调用方能自己拼装消息数组：`Msg` 是**规划用**的有损类型
/// （只有 role/text/tokens），拿它重建线路消息会把 `tool_calls`、`tool_call_id`、
/// `name`、图片块全部丢掉 —— agent 模式发的正是这些，上游会直接拒收。
/// 真正上线路的必须是原始 JSON 消息对象。
pub fn summary_system_text(summaries: &[String]) -> Option<String> {
    if summaries.is_empty() {
        return None;
    }
    let joined = summaries
        .iter()
        .enumerate()
        .map(|(i, s)| format!("【早期片段 {}】\n{}", i + 1, s.trim()))
        .collect::<Vec<_>>()
        .join("\n\n");
    Some(format!(
        "以下是本次对话**较早部分**的压缩记录（由 michael-compression 生成，非用户新发言）。\
         请把它当作已经发生过的事实来延续任务；如果其中的信息与后面的原文冲突，以原文为准。\n\n{joined}"
    ))
}

/// 组装后的预计 token 数：摘要占位 + 逐字尾部的真实 token。
///
/// 有了它才能在压缩完成后**校验结果真的塞得进窗口**。此前没有任何环节做这件事：
/// 撞到单轮新算上限就 break，剩下的段留在原文里，于是第一轮压缩仍然发出一个远超
/// 窗口的请求 —— 而客户端因为看到档位已经关掉了自己的裁剪，没有任何兜底。
pub fn projected_tokens_for(msgs: &[Msg], verbatim_from: usize, summary_count: usize) -> usize {
    let tail: usize = msgs
        .get(verbatim_from..)
        .map(|s| s.iter().map(|m| m.tokens).sum())
        .unwrap_or(0);
    summary_count * SEGMENT_SUMMARY_TOKENS + tail
}

/// 送给上游的 token 预算（留出安全边际）。
pub fn window_budget(native_window: usize) -> usize {
    ((native_window as f64) * WINDOW_SAFETY) as usize
}

/// 仅测试使用：生产路径改走 `summary_system_text` + 原始 JSON 拼接
/// （见 models.rs 的 `compression_write_back`），因为 `Msg` 会丢掉 tool_calls。
#[cfg(test)]
pub fn assemble(msgs: &[Msg], plan: &Plan, summaries: &[String]) -> Vec<Msg> {
    // 判据是「有没有摘要要注入」，不是「这一轮压了几段」。前缀续传时这一轮可能一段都
    // 没压，但手上仍握着上几轮的摘要——按 plan.compress 判会把它们整个丢掉，模型就
    // 凭空失忆了。
    if summaries.is_empty() {
        return msgs.to_vec();
    }
    let mut out = Vec::with_capacity(msgs.len() - plan.verbatim_from + 1);
    if let Some(text) = summary_system_text(summaries) {
        out.push(Msg::new("system", text));
    }
    out.extend_from_slice(&msgs[plan.verbatim_from..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msgs(specs: &[(&str, usize)]) -> Vec<Msg> {
        specs
            .iter()
            .map(|(role, toks)| Msg {
                role: (*role).into(),
                text: "x".repeat(*toks * 4),
                tokens: *toks,
            })
            .collect()
    }

    #[test]
    fn tier_round_trips_and_accepts_common_spellings() {
        for t in Tier::all() {
            assert_eq!(Tier::parse(t.as_str()), Some(t));
        }
        assert_eq!(Tier::parse("1M"), Some(Tier::M1));
        assert_eq!(Tier::parse(" 5m "), Some(Tier::M5));
        assert_eq!(Tier::parse("2000k"), Some(Tier::M2));
        assert_eq!(Tier::parse("3m"), None);
        assert_eq!(Tier::M1.max_input_tokens(), 1_000_000);
        assert_eq!(Tier::M5.max_input_tokens(), 5_000_000);
    }

    /// CJK 按 len/4 估算会低估约 4 倍，规划出来的上下文就会真的超窗口。
    #[test]
    fn token_estimate_does_not_undercount_chinese() {
        let zh = "这是一段中文对话内容";
        assert_eq!(estimate_tokens(zh), zh.chars().count());
        let en = "abcdefgh";
        assert_eq!(estimate_tokens(en), 2);
        // 混合文本两部分分别计。
        assert_eq!(estimate_tokens("中文abcd"), 2 + 1);
    }

    #[test]
    fn tier_entitlement_follows_the_plan() {
        assert_eq!(max_tier_for_plan("ultra", true, 0), Some(Tier::M5));
        assert_eq!(max_tier_for_plan("power", true, 0), Some(Tier::M5));
        assert_eq!(max_tier_for_plan("pro", true, 0), Some(Tier::M2));
        assert_eq!(max_tier_for_plan("basic", true, 0), Some(Tier::M1));
        assert_eq!(max_tier_for_plan("trial", true, 0), Some(Tier::M1));
        // 未知套餐名按最低档，不去猜它值多少钱。
        assert_eq!(max_tier_for_plan("enterprise-x", true, 0), Some(Tier::M1));
        // 套餐过期 → 只看余额。
        assert_eq!(max_tier_for_plan("ultra", false, 500), Some(Tier::M1));
        assert_eq!(max_tier_for_plan("ultra", false, 0), None);
        assert_eq!(max_tier_for_plan("none", false, 0), None);
    }

    /// 超出权限时下调而不是拒绝：长对话跑到一半被 402 掉是灾难性体验。
    #[test]
    fn requesting_above_the_plan_clamps_instead_of_failing() {
        assert_eq!(clamp_tier(Tier::M5, Some(Tier::M2)), Some(Tier::M2));
        assert_eq!(clamp_tier(Tier::M2, Some(Tier::M5)), Some(Tier::M2), "没到上限就按请求的来");
        assert_eq!(clamp_tier(Tier::M1, Some(Tier::M1)), Some(Tier::M1));
        assert_eq!(clamp_tier(Tier::M5, None), None, "完全无权限就是不可用");
    }

    /// 前缀引用是指向对话内容的 bearer 凭据：拿到别人的 token 就等于读到别人的历史。
    #[test]
    fn prefix_reference_is_bound_to_its_owner() {
        let rec = PrefixRecord {
            uid: "user-a".into(),
            segment_keys: vec!["mc:v1:aaa".into()],
            covered_msgs: 12,
            raw_tokens: 240_000,
        };
        assert!(prefix_belongs_to(&rec, "user-a"));
        assert!(!prefix_belongs_to(&rec, "user-b"), "别人的前缀必须拒绝");
    }

    #[test]
    fn prefix_tokens_are_unguessable_and_unique() {
        let a = new_prefix_token();
        let b = new_prefix_token();
        assert_ne!(a, b);
        assert!(a.starts_with("mcp_"));
        // uuid simple 形式是 32 位十六进制，加前缀共 36。
        assert_eq!(a.len(), 36);
    }

    #[test]
    fn prefix_record_round_trips_through_json() {
        let rec = PrefixRecord {
            uid: "u".into(),
            segment_keys: vec!["k1".into(), "k2".into()],
            covered_msgs: 40,
            raw_tokens: 800_000,
        };
        let json = serde_json::to_string(&rec).expect("serialize");
        let back: PrefixRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rec, back);
        // 顺序必须保住：摘要是按对话顺序拼回去的。
        assert_eq!(back.segment_keys, vec!["k1".to_string(), "k2".to_string()]);
    }

    /// 这是整个模块最重要的性质：追加消息不得改变已有分段。
    #[test]
    fn segmentation_is_prefix_stable() {
        let base = msgs(&[("user", 12_000), ("assistant", 9_000), ("user", 15_000)]);
        let first = segment_messages(&base, SEGMENT_TOKENS);

        let mut grown = base.clone();
        grown.push(Msg::new("assistant", "更多内容"));
        grown.push(Msg::new("user", "接着做"));
        let second = segment_messages(&grown, SEGMENT_TOKENS);

        // 已封口的段必须逐字节相同，否则它们的缓存键会变、旧摘要全部作废。
        let sealed = first.len().saturating_sub(1);
        assert!(sealed > 0, "测试数据应至少产生一个封口段");
        assert_eq!(&first[..sealed], &second[..sealed]);
    }

    #[test]
    fn segments_cover_every_message_without_overlap() {
        let m = msgs(&[("user", 8_000), ("assistant", 8_000), ("user", 8_000), ("assistant", 3_000)]);
        let segs = segment_messages(&m, SEGMENT_TOKENS);
        assert_eq!(segs.first().unwrap().start, 0);
        assert_eq!(segs.last().unwrap().end, m.len());
        for w in segs.windows(2) {
            assert_eq!(w[0].end, w[1].start, "分段之间不能有空隙或重叠");
        }
    }

    /// 同一段内容必须得到同一个键（缓存才能命中），不同压缩器/长度必须得到不同的键
    /// （否则会复用一个不是这次想要的摘要）。
    #[test]
    fn cache_key_is_content_addressed() {
        let a = segment_cache_key("同样的内容", "haiku", 600);
        assert_eq!(a, segment_cache_key("同样的内容", "haiku", 600));
        assert_ne!(a, segment_cache_key("别的内容", "haiku", 600));
        assert_ne!(a, segment_cache_key("同样的内容", "sonnet", 600));
        assert_ne!(a, segment_cache_key("同样的内容", "haiku", 900));
        assert!(a.starts_with("mc:v1:"));
    }

    #[test]
    fn short_conversations_are_left_alone() {
        let m = msgs(&[("user", 500), ("assistant", 800)]);
        let p = plan(&m, 200_000, VERBATIM_TAIL_TOKENS, SEGMENT_TOKENS);
        assert!(p.compress.is_empty(), "没超窗口就不该花钱压缩");
        assert_eq!(p.verbatim_from, 0);
        assert_eq!(p.projected_tokens, p.raw_tokens);
    }

    #[test]
    fn oversized_conversation_is_planned_into_the_native_window() {
        // 60 条 × 20K = 1.2M 原始输入，目标是 200K 原生窗口。
        let m = msgs(&vec![("user", 20_000); 60]);
        let p = plan(&m, 200_000, VERBATIM_TAIL_TOKENS, SEGMENT_TOKENS);
        assert!(!p.compress.is_empty());
        assert!(
            p.projected_tokens <= (200_000.0 * WINDOW_SAFETY) as usize,
            "规划后 {} token 仍超出预算",
            p.projected_tokens
        );
        assert!(p.raw_tokens > 1_000_000);
    }

    /// 压缩范围必须严格在逐字尾部之前，且必须完整覆盖，不能漏消息。
    #[test]
    fn compression_covers_the_whole_prefix_and_never_touches_the_tail() {
        let m = msgs(&vec![("user", 15_000); 40]);
        let p = plan(&m, 200_000, VERBATIM_TAIL_TOKENS, SEGMENT_TOKENS);
        assert_eq!(p.compress.first().unwrap().start, 0, "必须从第一条开始覆盖");
        assert_eq!(
            p.compress.last().unwrap().end,
            p.verbatim_from,
            "压缩范围必须正好接上逐字尾部"
        );
        for w in p.compress.windows(2) {
            assert_eq!(w[0].end, w[1].start, "压缩段之间不能漏消息");
        }
        assert!(p.verbatim_from < m.len(), "必须留下逐字尾部");
    }

    /// 极端情况：单条消息就撑爆窗口。仍必须至少逐字保留最后一条，否则模型收到的请求里
    /// 没有当前这个问题。
    #[test]
    fn always_keeps_at_least_the_final_message_verbatim() {
        let m = msgs(&[("user", 500_000), ("user", 500_000)]);
        let p = plan(&m, 200_000, VERBATIM_TAIL_TOKENS, SEGMENT_TOKENS);
        assert_eq!(p.verbatim_from, m.len() - 1);
    }

    /// 前缀续传：这一轮一段都没压，但手上有上几轮的摘要，必须照样注入。
    #[test]
    fn carried_summaries_survive_a_turn_that_compressed_nothing() {
        let m = msgs(&[("user", 100), ("assistant", 100)]);
        let empty_plan = Plan {
            compress: Vec::new(),
            verbatim_from: 0,
            projected_tokens: 200,
            raw_tokens: 200,
        };
        let out = assemble(&m, &empty_plan, &["早期要点".to_string()]);
        assert_eq!(out.len(), m.len() + 1, "摘要必须被注入");
        assert_eq!(out[0].role, "system");
        assert!(out[0].text.contains("早期要点"));
        assert_eq!(&out[1..], &m[..], "逐字部分不受影响");
    }

    /// 组装出来的序列必须完整保留逐字尾部，并且把摘要标成 system 而不是用户发言。
    #[test]
    fn assembly_keeps_the_tail_and_labels_the_summary() {
        let m = msgs(&vec![("user", 20_000); 40]);
        let p = plan(&m, 200_000, VERBATIM_TAIL_TOKENS, SEGMENT_TOKENS);
        let summaries: Vec<String> = p.compress.iter().map(|_| "要点若干".to_string()).collect();
        let out = assemble(&m, &p, &summaries);

        assert_eq!(out[0].role, "system", "摘要必须是 system，不能被当成用户新发言");
        assert!(out[0].text.contains("michael-compression"));
        assert!(out[0].text.contains("以原文为准"), "冲突时的优先级要写清楚");
        // 逐字尾部必须原样在后面。
        assert_eq!(out.len(), 1 + (m.len() - p.verbatim_from));
        assert_eq!(&out[1..], &m[p.verbatim_from..]);
    }

    /// 没有需要压缩的段时，assemble 必须原样返回，不能凭空插入一条 system。
    #[test]
    fn assembly_is_a_no_op_when_nothing_was_compressed() {
        let m = msgs(&[("user", 100), ("assistant", 100)]);
        let p = plan(&m, 200_000, VERBATIM_TAIL_TOKENS, SEGMENT_TOKENS);
        assert!(p.compress.is_empty());
        assert_eq!(assemble(&m, &p, &[]), m);
    }

    /// 跨轮复用：第二轮只有新增内容需要压缩，旧段的键必须原样命中。
    #[test]
    fn later_turns_only_pay_for_new_content() {
        let turn1 = msgs(&vec![("user", 20_000); 40]);
        let p1 = plan(&turn1, 200_000, VERBATIM_TAIL_TOKENS, SEGMENT_TOKENS);
        let keys1: Vec<String> = p1
            .compress
            .iter()
            .map(|s| segment_cache_key(&segment_text(&turn1, s), "haiku", SEGMENT_SUMMARY_TOKENS))
            .collect();

        let mut turn2 = turn1.clone();
        turn2.extend(msgs(&[("assistant", 20_000), ("user", 20_000)]));
        let p2 = plan(&turn2, 200_000, VERBATIM_TAIL_TOKENS, SEGMENT_TOKENS);
        let keys2: Vec<String> = p2
            .compress
            .iter()
            .map(|s| segment_cache_key(&segment_text(&turn2, s), "haiku", SEGMENT_SUMMARY_TOKENS))
            .collect();

        let reused = keys1.iter().filter(|k| keys2.contains(k)).count();
        assert!(
            reused >= keys1.len() - 1,
            "第二轮应复用几乎全部旧摘要，实际只命中 {reused}/{}",
            keys1.len()
        );
        let fresh = keys2.len() - reused;
        assert!(fresh <= 2, "第二轮不该产生 {fresh} 个新段");
    }
}
