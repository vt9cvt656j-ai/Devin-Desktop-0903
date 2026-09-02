//! 前缀在哪一步断的。
//!
//! # 为什么需要它
//!
//! OpenAI / xAI 那一族是**严格前缀**的自动缓存：只要某一条消息和上一次请求不一样，
//! 从那一点往后全部进不了缓存。线上实测（2026-09-02，24h）：
//!
//!   · gpt-5.6-luna 大回合（>40k）命中 27.8%，平均每轮缓存 3.1 万 / 请求 11.3 万
//!   · 同为 OpenAI 形状的 deepseek-v4-pro 大回合能到 53.5%
//!   · 命中率**与间隔无关**：<1 分钟 26.9%、1–5 分钟 26.6%、首轮 23.6%
//!
//! 与间隔无关就排除了"缓存过期"；装配出来的系统块和工具块逐字节相同（有测试钉着），
//! 也排除了"前缀本身在变"。剩下的只能是**消息数组中段**在变 —— 而缓存量约等于
//! 「静态头 + 一点点早期对话」，正是"公共前缀断在很靠前的地方"的形状。
//!
//! 到这一步继续猜没有意义。这个模块把它变成一次测量：按 run 记住上一次请求每条消息的
//! 指纹，下一次进来时报出**从第几条开始不一样**、以及那条消息的角色和长度变化。
//!
//! 只写日志，不改任何行为 —— 诊断代码不该有让请求失败的能力。
//! 只留指纹（16 字节哈希）和长度，**不留任何内容**：这条链路上流的是用户的代码和对话。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// 一条消息的指纹：角色 + 内容哈希 + 字节数。内容本身不留。
#[derive(Clone, PartialEq, Eq)]
struct MsgPrint {
    role: [u8; 12],
    hash: u64,
    bytes: u32,
}

struct RunPrints {
    prints: Vec<MsgPrint>,
    at: Instant,
}

/// 按 run 存上一次的指纹。上限 512 条 run，超了清掉最旧的一半 ——
/// 诊断结构不能把内存吃掉。
static LAST: LazyLock<Mutex<HashMap<String, RunPrints>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const MAX_RUNS: usize = 512;
const STALE: Duration = Duration::from_secs(3600);

fn print_of(m: &serde_json::Value) -> MsgPrint {
    use std::hash::{Hash, Hasher};
    let role_str = m.get("role").and_then(|v| v.as_str()).unwrap_or("");
    let mut role = [0u8; 12];
    for (i, b) in role_str.bytes().take(12).enumerate() {
        role[i] = b;
    }
    // content 可能是字符串、也可能是块数组（工具结果、图片）。一律按序列化后的字节算，
    // 这样"同样的内容换了形状"也会被当成不同 —— 那对前缀缓存来说本来就是不同。
    let body = m.get("content").map(|c| c.to_string()).unwrap_or_default();
    let extra = m.get("tool_calls").map(|c| c.to_string()).unwrap_or_default();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    body.hash(&mut h);
    extra.hash(&mut h);
    MsgPrint {
        role,
        hash: h.finish(),
        bytes: (body.len() + extra.len()) as u32,
    }
}

/// 记下这一次的消息指纹，并报出它和上一次**从第几条开始不一样**。
///
/// 返回 `None` 表示这是这个 run 的第一次请求（没有可比的）。
/// 返回 `Some((idx, total_prev, total_now))`：idx 是首个不同的位置。
/// idx == total_prev 表示**纯追加**（前缀完整保留），那是缓存最想要的形状。
pub fn diverged_at(run_id: &str, messages: &[serde_json::Value]) -> Option<(usize, usize, usize)> {
    if run_id.is_empty() {
        return None;
    }
    let now: Vec<MsgPrint> = messages.iter().map(print_of).collect();
    let mut guard = LAST.lock().ok()?;
    if guard.len() > MAX_RUNS {
        let cutoff = Instant::now() - STALE;
        guard.retain(|_, v| v.at > cutoff);
        if guard.len() > MAX_RUNS {
            guard.clear();
        }
    }
    let prev = guard.insert(
        run_id.to_string(),
        RunPrints { prints: now.clone(), at: Instant::now() },
    )?;
    let idx = prev
        .prints
        .iter()
        .zip(now.iter())
        .position(|(a, b)| a != b)
        .unwrap_or(prev.prints.len().min(now.len()));
    Some((idx, prev.prints.len(), now.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn msgs(items: &[(&str, &str)]) -> Vec<serde_json::Value> {
        items.iter().map(|(r, c)| json!({"role": r, "content": c})).collect()
    }

    #[test]
    fn a_pure_append_reports_the_whole_previous_prefix_as_intact() {
        let run = "run-append";
        assert!(diverged_at(run, &msgs(&[("system", "s"), ("user", "a")])).is_none());
        // 追加两条：首个不同的位置 == 上一次的长度，也就是"前缀一个字节没变"。
        let (idx, prev_len, now_len) =
            diverged_at(run, &msgs(&[("system", "s"), ("user", "a"), ("assistant", "b")])).unwrap();
        assert_eq!(idx, 2, "纯追加却报出中途就断了");
        assert_eq!((prev_len, now_len), (2, 3));
    }

    #[test]
    fn a_changed_message_in_the_middle_is_pinpointed() {
        // 这正是要抓的东西：中段某条被换掉，缓存从那一点起全部作废。
        let run = "run-mid";
        diverged_at(run, &msgs(&[("system", "s"), ("user", "a"), ("user", "nudge-v1")]));
        let (idx, _, _) =
            diverged_at(run, &msgs(&[("system", "s"), ("user", "a"), ("user", "nudge-v2"), ("assistant", "b")]))
                .unwrap();
        assert_eq!(idx, 2, "中段被换掉了却没报出位置");
    }

    #[test]
    fn a_removed_message_shortens_the_common_prefix() {
        // nudge 被移除再追加是同一个形状：从被移除的位置起就对不上了。
        let run = "run-drop";
        diverged_at(run, &msgs(&[("system", "s"), ("user", "a"), ("user", "nudge"), ("assistant", "b")]));
        let (idx, prev_len, now_len) =
            diverged_at(run, &msgs(&[("system", "s"), ("user", "a"), ("assistant", "b")])).unwrap();
        assert_eq!(idx, 2);
        assert_eq!((prev_len, now_len), (4, 3));
    }

    #[test]
    fn nothing_of_the_content_is_retained() {
        // 这条链路上流的是用户的代码和对话。指纹只留哈希和长度。
        let src = include_str!("prefix_probe.rs");
        let struct_body = src
            .split("struct MsgPrint {")
            .nth(1)
            .and_then(|t| t.split_once('}').map(|(a, _)| a))
            .expect("结构体没找到");
        assert!(!struct_body.contains("String"), "指纹里存了字符串——那就是把内容留下来了");
        assert!(struct_body.contains("hash") && struct_body.contains("bytes"));
    }

    #[test]
    fn an_empty_run_id_is_ignored_rather_than_bucketed_together() {
        // 没有 run id 的请求（老客户端、单发测试）不该互相当成同一条对话比较。
        assert!(diverged_at("", &msgs(&[("user", "a")])).is_none());
        assert!(diverged_at("", &msgs(&[("user", "b")])).is_none());
    }
}
