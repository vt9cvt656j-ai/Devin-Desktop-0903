//! 流断了以后**接着写**，让用户看不出中间换过出口。
//!
//! # 为什么放在这里而不是各个协议的转换器里
//!
//! 上游有三种协议（OpenAI / Anthropic / xAI Responses），但**发给客户端的一律是
//! OpenAI SSE** —— 三种都在转换器里归一过了。所以「已经吐出去多少字」只要在
//! 转发那一处数一次，三条路自动都覆盖。放进各自的转换器要写三遍，还会漂。
//!
//! # 什么时候可以续写，什么时候绝对不行
//!
//! * **开始过工具调用 → 不续。** 那时候断在半截 JSON 里，续写拼出来的参数
//!   可能是合法 JSON 却是错的意思 —— 而工具调用会**真的执行**。宁可让这一次失败。
//! * **一个字都没吐出去 → 直接重发**，没有拼接问题，风险最小。
//! * **吐了一部分 → 带着已生成的内容重发**，只把新增的部分接着推给客户端。
//!
//! # 重复是怎么防的
//!
//! 「带着已生成内容接着写」在 Anthropic 那边是原生的（assistant 预填），
//! 在 OpenAI 兼容那边不保证 —— 有的模型会**从头再说一遍**。所以续写回来的头几百字
//! 要和已经发出去的尾巴对一遍，重叠的砍掉（`strip_overlap`）。
//! 不砍的话用户会看到一段话说了两遍，那比断掉更像 bug。

use serde_json::Value;

/// 续写最多做几次。
///
/// 一次就够覆盖「上游抖一下」；再多的话，一个持续抽风的上游会让**一次**用户请求
/// 变成三四次真实生成 —— 那是实打实的钱，而且用户等的时间也没省下来。
pub const MAX_CONTINUATIONS: u8 = 1;

/// 拿来找重叠的尾巴取多长（字符）。
///
/// 太短会漏掉「从头再说一遍」里较长的重复；太长会把正常的重复用语误当成重叠砍掉。
/// 400 字覆盖得住常见的重述开头，又短到不会误伤。
const OVERLAP_WINDOW: usize = 400;

/// 从一段发给客户端的 OpenAI SSE 里，把正文增量追加到 `acc`。
///
/// 只认 `choices[].delta.content`。思考内容（`reasoning_content`）**不算** ——
/// 续写时要交给模型的是「你已经说出口的话」，思考不是。
pub fn absorb_text(sse: &[u8], acc: &mut String) {
    for frame in split_frames(sse) {
        let Ok(v) = serde_json::from_str::<Value>(&frame) else { continue };
        let Some(choices) = v.get("choices").and_then(|c| c.as_array()) else { continue };
        for c in choices {
            if let Some(t) = c.get("delta").and_then(|d| d.get("content")).and_then(|x| x.as_str())
            {
                acc.push_str(t);
            }
        }
    }
}

/// 这段 SSE 里有没有开始工具调用。开始了就**不许续写**。
pub fn saw_tool_call(sse: &[u8]) -> bool {
    for frame in split_frames(sse) {
        let Ok(v) = serde_json::from_str::<Value>(&frame) else {
            // 解析不了的帧里出现 tool_calls 字样也当成有 —— 这一处宁可保守：
            // 判错成「有工具调用」只是少续一次，判错成「没有」会拼出错的工具参数。
            if frame.contains("\"tool_calls\"") {
                return true;
            }
            continue;
        };
        let Some(choices) = v.get("choices").and_then(|c| c.as_array()) else { continue };
        for c in choices {
            if c.get("delta").and_then(|d| d.get("tool_calls")).is_some() {
                return true;
            }
        }
    }
    false
}

/// 把 SSE 字节切成一个个 `data:` 帧的正文。`[DONE]` 和注释行跳过。
fn split_frames(sse: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(sse);
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("data:") else { continue };
        let rest = rest.trim();
        if rest.is_empty() || rest == "[DONE]" {
            continue;
        }
        out.push(rest.to_string());
    }
    out
}

/// 造续写请求体：原样的请求 + 一条装着「已经说到这里」的 assistant 消息。
///
/// Anthropic 原生支持这个形状（assistant 预填，模型会**接着**这段往下写）。
/// OpenAI 兼容那边不保证，所以回来的内容还要过一遍 `strip_overlap`。
///
/// **不加任何「请接着写」之类的指令。** 那会进上下文、影响模型说什么，
/// 而用户根本没说过那句话 —— 换出口不该改变回答的内容。
pub fn continuation_body(original: &Value, partial: &str) -> Option<Value> {
    if partial.trim().is_empty() {
        return None;
    }
    let mut body = original.clone();
    let msgs = body.get_mut("messages")?.as_array_mut()?;
    msgs.push(serde_json::json!({ "role": "assistant", "content": partial }));
    Some(body)
}

/// 续写回来的内容里，开头有多少字节和「已经发出去的尾巴」重叠。
///
/// 返回的是 `next` 里**应该跳过**的字节数。
///
/// 做法：拿已发内容的最后 `OVERLAP_WINDOW` 个字符当尾巴，从长到短找
/// 「尾巴的某个后缀 == next 的前缀」。找最长的那个 —— 短的重叠可能是巧合
/// （比如两边都以「。」结尾开头），长的才是真的重述。
pub fn strip_overlap(already: &str, next: &str) -> usize {
    if already.is_empty() || next.is_empty() {
        return 0;
    }
    // 按**字符**取窗口，不按字节 —— 按字节切会切在汉字中间。
    let tail: String = {
        let n = already.chars().count();
        already.chars().skip(n.saturating_sub(OVERLAP_WINDOW)).collect()
    };
    let tail_chars: Vec<char> = tail.chars().collect();
    let next_chars: Vec<char> = next.chars().collect();
    let max = tail_chars.len().min(next_chars.len());
    // 从最长的重叠往下试。
    for len in (1..=max).rev() {
        if tail_chars[tail_chars.len() - len..] == next_chars[..len] {
            let bytes: usize = next_chars[..len].iter().map(|c| c.len_utf8()).sum();
            // 太短的当巧合，不砍。门槛同时看**字数和字节数**：
            // 纯按字数定的话，中文六个字（「疑是地上霜。」）已经是很硬的重叠却会被放过，
            // 而英文六个字母（"the ca"）撞上纯属常事。字节数正好是信息量的近似 ——
            // 一个汉字三字节，四个汉字就到 12 字节；英文要八个字母才到同一条线。
            if len < 4 || bytes < 8 {
                return 0;
            }
            return bytes;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sse(frames: &[&str]) -> Vec<u8> {
        frames.iter().map(|f| format!("data: {f}\n\n")).collect::<String>().into_bytes()
    }

    #[test]
    fn it_counts_only_the_text_the_client_actually_saw() {
        let mut acc = String::new();
        absorb_text(
            &sse(&[
                r#"{"choices":[{"delta":{"role":"assistant"}}]}"#,
                r#"{"choices":[{"delta":{"content":"你好"}}]}"#,
                r#"{"choices":[{"delta":{"content":"，世界"}}]}"#,
                "[DONE]",
            ]),
            &mut acc,
        );
        assert_eq!(acc, "你好，世界");

        // 思考内容不算：续写要交给模型的是「你已经说出口的话」，思考不是。
        let mut acc = String::new();
        absorb_text(&sse(&[r#"{"choices":[{"delta":{"reasoning_content":"想一想"}}]}"#]), &mut acc);
        assert_eq!(acc, "", "思考被当成正文了");

        // 心跳注释和坏帧都不该炸，也不该算进去。
        let mut acc = String::new();
        absorb_text(b": ping\n\ndata: {\"broken\n\n", &mut acc);
        assert_eq!(acc, "");
    }

    /// 开始过工具调用就**绝对不许**续写。
    ///
    /// 那时候断在半截 JSON 里，续写拼出来的参数可能是合法 JSON 却是错的意思，
    /// 而工具调用是**会真的执行**的。宁可让这一次失败。
    #[test]
    fn a_tool_call_forbids_continuation() {
        assert!(saw_tool_call(&sse(&[
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"bash"}}]}}]}"#
        ])));
        // 半截帧解析不出来时也按「有」算 —— 这一处宁可保守。
        assert!(saw_tool_call(b"data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"inde"));
        // 纯文本的不该误判。
        assert!(!saw_tool_call(&sse(&[r#"{"choices":[{"delta":{"content":"讲讲 tool_calls 是什么"}}]}"#])));
    }

    /// 续写请求只加一条 assistant 消息，**不加任何指令**。
    ///
    /// 加「请接着写」之类的话会进上下文、影响模型说什么，而用户根本没说过那句 ——
    /// 换出口不该改变回答的内容。
    #[test]
    fn the_continuation_adds_nothing_but_what_was_already_said() {
        let orig = serde_json::json!({
            "model": "claude-opus-5",
            "messages": [{"role": "user", "content": "写首诗"}],
            "temperature": 0.7
        });
        let b = continuation_body(&orig, "床前明月光，").expect("该造得出来");
        let msgs = b["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1]["role"], "assistant");
        assert_eq!(msgs[1]["content"], "床前明月光，");
        // 原来的参数一个字不能动。
        assert_eq!(b["temperature"], 0.7);
        assert_eq!(b["model"], "claude-opus-5");
        assert_eq!(msgs[0]["content"], "写首诗");
        // 没有正文可续时不造 —— 那种情况该走「直接重发」，不是续写。
        assert!(continuation_body(&orig, "   ").is_none());
    }

    /// 续写回来重说一遍的部分要砍掉。
    ///
    /// Anthropic 那边 assistant 预填是原生的、会接着写；OpenAI 兼容那边不保证，
    /// 有的模型会从头再说一遍。不砍的话用户会看到一段话说了两遍 ——
    /// 那比断掉更像 bug。
    #[test]
    fn a_restated_prefix_is_stripped() {
        let already = "床前明月光，疑是地上霜。";
        // 模型从头重说：整段重叠，全砍。
        let skip = strip_overlap(already, "疑是地上霜。举头望明月。");
        assert_eq!(&"疑是地上霜。举头望明月。"[skip..], "举头望明月。");
        // 正常接着写：没有重叠，一个字都不砍。
        assert_eq!(strip_overlap(already, "举头望明月，低头思故乡。"), 0);
        // 巧合的短重合不砍 —— 中文里一两个字重合太常见。
        assert_eq!(strip_overlap("……结束了。", "。然后呢"), 0);
        // 门槛同时看字数和字节数：英文四个字母（"the "）是常事，不砍；
        // 中文四个字（12 字节）就是硬重叠，砍。
        assert_eq!(strip_overlap("I saw the ", "the cat sat"), 0, "英文短重合被误砍了");
        let skip = strip_overlap("他说过这四个字", "这四个字后面还有");
        assert!(skip > 0, "中文四字重叠没被认出来");
        assert_eq!(&"这四个字后面还有"[skip..], "后面还有");
        // 空的两边都不炸。
        assert_eq!(strip_overlap("", "abc"), 0);
        assert_eq!(strip_overlap("abc", ""), 0);
    }

    /// 砍重叠必须按**字符**切，不能按字节 —— 按字节会切在汉字中间直接 panic。
    #[test]
    fn overlap_never_splits_a_character() {
        let already = "一".repeat(1000);
        let next = format!("{}后面的新内容", "一".repeat(50));
        let skip = strip_overlap(&already, &next);
        // 切点必须落在字符边界上，否则下面这一句就 panic 了。
        let _ = &next[skip..];
        assert!(skip > 0, "整段重复没被认出来");

        // 窗口本身也必须按字符取。按字节 `&already[n..]` 切会在汉字中间断开直接 panic，
        // 而 1000 个汉字的窗口边界（3000-400=2600）恰好就不在字符边界上。
        // 这一条钉的是写法：行为上「安全版的按字节切」和按字符切碰巧同结果，
        // 测不出来，但那个写法离 panic 只差一个 `.get()`。
        let all = include_str!("failover.rs");
        let me = &all[..all.find("\n#[cfg(test)]").unwrap_or(all.len())];
        assert!(
            me.contains("already.chars().skip(n.saturating_sub(OVERLAP_WINDOW)).collect()"),
            "取尾巴窗口没按字符切 —— 中文源码里这会 panic",
        );
    }

    /// 续写次数必须有上限。
    ///
    /// 一个持续抽风的上游会让**一次**用户请求变成三四次真实生成 —— 那是实打实的钱，
    /// 而且用户等的时间也没省下来。
    #[test]
    fn continuation_is_capped() {
        assert_eq!(MAX_CONTINUATIONS, 1);
        let all = include_str!("failover.rs");
        let me = &all[..all.find("\n#[cfg(test)]").unwrap_or(all.len())];
        assert!(me.contains("pub const MAX_CONTINUATIONS: u8 = 1;"));
    }
}
