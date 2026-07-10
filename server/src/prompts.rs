//! Cloud prompt registry.
//!
//! The big static system-prompt blobs that used to be hard-coded in the IDE
//! bundle (`ide/src/main.js`) now live here, in `./prompts/<name>.txt`, and are
//! served to authenticated IDE clients. Two wins:
//!   1. They're no longer trivially extractable from the shipped desktop app.
//!   2. They can be improved without reshipping the app — edit a file + restart
//!      the backend container and every client picks up the new prompt.
//!
//! Read on each request (the files are tiny relative to a chat request) so a file
//! edit hot-updates all clients. `version` is a content hash the IDE caches on, so
//! it only swaps prompts when they actually change.

use crate::agent_trace::{record_agent_trace, AgentTraceInput};
use crate::auth::Claims;
use crate::error::ApiResult;
use axum::http::HeaderMap;
use axum::Json;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

// The bundled registry currently contains 109 tools. Keep a bounded margin for
// additions while allowing the IDE to send its complete static selection.
const MAX_STATIC_TOOLS_PER_REQUEST: usize = 128;

fn prompt_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("prompts")
        .join(format!("{name}.txt"))
}

fn tools_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("prompts")
        .join("tools.json")
}

fn read_tools_file() -> Result<String, String> {
    let path = tools_path();
    std::fs::read_to_string(&path).map_err(|err| {
        tracing::warn!(path = %path.display(), %err, "failed to load prompts/tools.json");
        format!("tools.json load failed: {err}")
    })
}

/// Read one prompt file, with path-traversal protection (name must be [A-Za-z0-9_]).
fn read_prompt(name: &str) -> Result<String, String> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        tracing::warn!(prompt = name, "rejected invalid prompt name");
        return Err("invalid prompt name".to_string());
    }
    let path = prompt_path(name);
    std::fs::read_to_string(&path).map_err(|err| {
        tracing::warn!(prompt = name, path = %path.display(), %err, "failed to load prompt file");
        format!("prompt {name} load failed: {err}")
    })
}

fn allowed_static_tool(mode: &str, name: &str) -> bool {
    match mode {
        // Plain chat should not silently grow into an autonomous tool-using agent.
        "chat" => matches!(
            name,
            "web_search" | "web_fetch" | "knowledge_search" | "ask_user"
        ),
        "plan" => matches!(
            name,
            "update_plan"
                | "ask_user"
                | "list_dir"
                | "read_file"
                | "find_files"
                | "search"
                | "lsp_symbols"
                | "lsp_definition"
                | "lsp_references"
                | "knowledge_search"
                | "web_search"
                | "web_fetch"
                | "research_project"
                | "run_subagent"
        ),
        "explorer" | "reviewer" => matches!(
            name,
            "ask_user"
                | "list_dir"
                | "read_file"
                | "find_files"
                | "search"
                | "lsp_symbols"
                | "find_symbol"
                | "lsp_definition"
                | "lsp_references"
                | "get_diagnostics"
                | "git_status"
                | "git_diff"
                | "git_log"
                | "git_blame"
                | "git_conflicts"
                | "knowledge_search"
                | "web_search"
                | "web_fetch"
                | "research_project"
                | "run_subagent"
        ),
        // Agent mode can request mutating tools, but still goes through a server-side cap.
        "agent" | "ui" => true,
        other => {
            tracing::warn!(
                mode = other,
                "unknown IDE mode; static tool injection disabled"
            );
            false
        }
    }
}

fn requested_static_tools(mode: &str, names: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();

    for name in names.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if !seen.insert(name.to_string()) {
            continue;
        }
        if allowed_static_tool(mode, name) {
            accepted.push(name.to_string());
        } else {
            rejected.push(name.to_string());
        }
    }

    if accepted.len() > MAX_STATIC_TOOLS_PER_REQUEST {
        tracing::warn!(
            mode,
            requested = accepted.len(),
            cap = MAX_STATIC_TOOLS_PER_REQUEST,
            "too many static tools requested; truncating"
        );
        accepted.truncate(MAX_STATIC_TOOLS_PER_REQUEST);
    }
    if !rejected.is_empty() {
        tracing::warn!(mode, tools = ?rejected, "rejected static tools for IDE mode");
    }

    accepted
}

/// Weak / small / fast-tier models follow a long, densely-caveated prompt poorly — they under-use
/// tools, skip verification, and drown in the ~7K-token `agent` prompt. Serve THEM the tighter,
/// directive `agent_lite` instead (agent mode only; other modes are already short). Conservative
/// DENYLIST: anything not matched here keeps the full prompt, so frontier models never regress.
/// Reasoning models keep the full prompt even if the family name looks small. Env override:
/// `MICHAEL_LITE_PROMPT=0` disables it entirely, `=all` forces lite for every model.
fn use_lite_agent_prompt(model: &str) -> bool {
    match std::env::var("MICHAEL_LITE_PROMPT").ok().as_deref() {
        Some("0") => return false,
        Some("all") => return true,
        _ => {}
    }
    let m = model.to_lowercase();
    // Reasoning models digest the full prompt fine regardless of tier naming.
    if m.contains("reasoner") || m.contains("-r1") || m.contains("qwq") || m.contains("think") {
        return false;
    }
    m.contains("deepseek-v")
        || m.contains("deepseek-chat")
        || m.contains("minimax")
        || m.contains("flash")
        || m.contains("haiku")
        || m.contains("-mini")
        || m.contains("-lite")
        || m.contains("-small")
        || m.contains("-air")
        || m.contains("glm-4-flash")
        || m.contains("qwen-turbo")
        || m.contains("qwen-plus")
        || m.contains("ernie")
        || m.contains("hunyuan-lite")
        || m.contains("doubao-lite")
        || m.contains("gpt-4o-mini")
        || m.contains("gpt-5-mini")
        || m.contains("o1-mini")
        || m.contains("o3-mini")
        || m.contains("o4-mini")
}

/// The most recent user message's text (concatenating the text parts of a multimodal content
/// array). Used to auto-inject relevant knowledge for weak models.
fn latest_user_text(body: &serde_json::Value) -> Option<String> {
    let msgs = body.get("messages")?.as_array()?;
    for m in msgs.iter().rev() {
        if m.get("role").and_then(|r| r.as_str()) != Some("user") {
            continue;
        }
        match m.get("content") {
            Some(serde_json::Value::String(s)) if !s.trim().is_empty() => return Some(s.clone()),
            Some(serde_json::Value::Array(parts)) => {
                let text: String = parts
                    .iter()
                    .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join(" ");
                if !text.trim().is_empty() {
                    return Some(text);
                }
            }
            _ => {}
        }
    }
    None
}

/// Broad, generous "is this a web-UI task?" check on the user's latest message, used to
/// decide whether to inject the deep design guide. Enrichment, not a restriction gate:
/// a false positive only adds design guidance (mild token cost), it never blocks anything.
/// The point is that the deepest design guidance (`ui_design_guide`/`css_concrete_tokens`)
/// was previously gated behind an `x-ide-ui` header that NOTHING emits — so it never
/// reached the model. This makes it fire whenever the work is plausibly UI/frontend.
fn looks_like_ui_task(q: &str) -> bool {
    let l = q.to_lowercase();
    // ASCII terms (matched against the lowercased query)
    const ASCII_KW: &[&str] = &[
        "website",
        "web page",
        "webpage",
        "web app",
        "frontend",
        "front-end",
        "landing page",
        "landing",
        "dashboard",
        "navbar",
        "hero section",
        "ui design",
        "ui/ux",
        "tailwind",
        "shadcn",
        "css",
        "html",
        "react",
        "vue",
        "svelte",
        "next.js",
        "nextjs",
        "responsive",
        "dark mode",
        "portfolio",
        "homepage",
        "styling",
        "stylesheet",
    ];
    // CJK terms (matched against the raw query)
    const CJK_KW: &[&str] = &[
        "网页",
        "网站",
        "前端",
        "页面",
        "界面",
        "组件",
        "布局",
        "落地页",
        "官网",
        "主页",
        "首页",
        "仪表盘",
        "后台管理",
        "表单",
        "导航栏",
        "响应式",
        "深色模式",
        "暗色模式",
        "配色",
        "字体排版",
        "设计稿",
        "设计图",
        "ui设计",
        "建站",
        "美化界面",
        "做界面",
        "做网页",
        "做个站",
        "切图",
        "样式",
        "视觉稿",
        "交互动效",
    ];
    ASCII_KW.iter().any(|k| l.contains(k)) || CJK_KW.iter().any(|k| q.contains(k))
}

/// Server-side assembly (L0 — "airtight"): if the IDE asks for it via headers, inject the
/// system prompt + the requested tool schemas HERE, just before forwarding upstream — so the
/// client never ships the prompts or the tool definitions (the real anti-reverse-engineering
/// win; client-side encryption/obfuscation only raises the bar). Fully gated: a request with
/// no `x-ide-mode` header is left UNCHANGED (existing behavior), so this can't affect any
/// traffic that doesn't opt in.
///   x-ide-mode:  agent | chat | plan | explorer | reviewer  → prepend that mode's system prompt
///   x-ide-ui:    (present) → also append the UI flow + guide
///   x-ide-tools: comma-separated tool names → inject those tools' schemas from tools.json
pub fn assemble_into(headers: &HeaderMap, body: &mut serde_json::Value) {
    let hdr = |k: &str| headers.get(k).and_then(|v| v.to_str().ok());
    let mode = match hdr("x-ide-mode") {
        Some(m) if !m.is_empty() => m,
        _ => return, // not opted in → leave the request exactly as the client sent it
    };
    if !body.is_object() {
        return;
    }
    let requested_tool_count = hdr("x-ide-tools")
        .map(|names| {
            names
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .collect::<HashSet<_>>()
                .len()
        })
        .unwrap_or(0);
    let mut prompt_blocks: Vec<&str> = Vec::new();
    // 1) prepend the execution prompt (mode + optional UI guides) as messages[0].
    // User-growth coaching is injected separately and scoped to final replies so it does
    // not compete with tool selection, autonomy, or verification rules during execution.
    // Model-tier-adaptive: weak models get the tighter `agent_lite`; everyone else the full prompt.
    // Compute the weakness flag in its own scope so the `&str` model borrow of `body` is released
    // before we mutate `body.messages` below. Falls back to the full `agent` prompt if `agent_lite`
    // is missing, so a partial deploy can't leave a weak model prompt-less.
    let is_weak_model = {
        let model = body.get("model").and_then(|m| m.as_str()).unwrap_or("");
        mode == "agent" && use_lite_agent_prompt(model)
    };
    let prompt_name: &str = if is_weak_model { "agent_lite" } else { mode };
    // Snapshot the user's latest query (owned) now — used below to auto-inject a knowledge
    // cheatsheet for weak models after `messages` has been mutated.
    let user_query = latest_user_text(body);
    let mut sys = read_prompt(prompt_name)
        .or_else(|_| {
            if prompt_name != mode {
                read_prompt(mode)
            } else {
                Err(String::new())
            }
        })
        .unwrap_or_else(|_| {
            tracing::error!(
                mode,
                prompt_name,
                "failed to load mode prompt, degrading to empty"
            );
            String::new()
        });
    if !sys.is_empty() {
        prompt_blocks.push(prompt_name);
    }
    // Inject the design guide on any UI/frontend task — not just when the (never-emitted)
    // x-ide-ui header is present. `MICHAEL_UI_GUIDE=0` disables; `=always` forces it on.
    let ui_env = std::env::var("MICHAEL_UI_GUIDE").ok();
    let ui_intent = ui_env.as_deref() == Some("always")
        || (ui_env.as_deref() != Some("0")
            && (hdr("x-ide-ui").is_some()
                || user_query
                    .as_deref()
                    .map(looks_like_ui_task)
                    .unwrap_or(false)));
    if ui_intent {
        // Flow + the copy-paste concrete tokens are compact and high-signal → inject for all.
        for name in ["ui_design_flow", "css_concrete_tokens"] {
            let block = read_prompt(name).unwrap_or_default();
            if !block.is_empty() {
                prompt_blocks.push(name);
                sys.push_str("\n\n");
                sys.push_str(&block);
            }
        }
        // The deep guide is large (~14KB); skip it for weak models to avoid drowning them,
        // give it to capable models where the extra depth pays off.
        if !is_weak_model {
            let guide = read_prompt("ui_design_guide").unwrap_or_default();
            if !guide.is_empty() {
                prompt_blocks.push("ui_design_guide");
                sys.push_str("\n\n");
                sys.push_str(&guide);
            }
        }
    }
    // 始终把真实当前时间(北京时间 UTC+8)注入系统提示词顶部——模型老"忘记时间/引用训练截止的旧
    // 时间"，有这行它张口就知道今天几号，不必再去调 current_time（用户报"时间工具也不行老忘记"）。
    if !sys.is_empty() {
        use chrono::{Datelike, Timelike};
        let bj = chrono::Utc::now() + chrono::Duration::hours(8);
        let wd = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"]
            [bj.weekday().num_days_from_sunday() as usize];
        sys = format!(
            "【当前真实时间·北京时间 UTC+8】今天是 {}-{:02}-{:02} {} {:02}:{:02}。凡涉及\"今天/现在/最新/几号/星期几/距某天还有多久\"直接用这个时间算，别猜、别报训练数据里的旧时间。\n\n{}",
            bj.year(), bj.month(), bj.day(), wd, bj.hour(), bj.minute(), sys
        );
    }
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        if !sys.is_empty() {
            msgs.insert(0, serde_json::json!({ "role": "system", "content": sys }));
        }

        // Long-session reminder: if the conversation has grown beyond ~10 turns, inject a
        // SHORT system reminder before the user's latest message to re-anchor the model's
        // reasoning discipline. Without this, models drift and stop reasoning carefully in long
        // sessions (user-reported issue: "跑时间长就不思考了"). Keep it terse so it
        // doesn't bloat context; the full rules are already in messages[0].
        const LONG_SESSION_THRESHOLD: usize = 10;
        if msgs.len() >= LONG_SESSION_THRESHOLD {
            let reminder = "⚠️ 强制推理检查点：下一步前先在脑子里快速过一遍——① 我真的理解了吗？② 还缺什么关键信息？③ 这步要拿到什么？④ 可能出什么岔子？除非是显而易见的一步操作（读明确指定的文件、改一行明确的代码），否则先想清楚再动手。";
            // Insert before the LAST message (which is the user's newest prompt).
            let insert_pos = msgs.len().saturating_sub(1);
            msgs.insert(
                insert_pos,
                serde_json::json!({ "role": "system", "content": reminder }),
            );
        }

        if let Some(growth) = hdr("x-ide-growth").map(str::trim).filter(|g| !g.is_empty()) {
            prompt_blocks.push("growth_final_only");
            msgs.push(serde_json::json!({
                "role": "system",
                "content": format!(
                    "--- 因人而教（只作用于最终收尾总结）---\n{growth}\n\n执行任务、选择工具、修改代码、验证结果时忽略本段；只在最终回复里用它调整解释深度。"
                ),
            }));
        }
    }
    // 1b) Auto-inject a knowledge-base cheatsheet for WEAK models: they benefit most from the
    // curated corpus but won't call `knowledge_search` themselves. Only when the user's query
    // strongly matches (BM25 score floor) so trivial asks ("改个名") get nothing. Frontier models
    // are untouched (they self-serve knowledge_search when useful). Env MICHAEL_AUTO_KNOWLEDGE=0 off.
    if is_weak_model && std::env::var("MICHAEL_AUTO_KNOWLEDGE").ok().as_deref() != Some("0") {
        if let Some(q) = user_query.as_deref().filter(|q| q.chars().count() >= 12) {
            if let Some(h) = crate::knowledge::search(q, None, 1)
                .into_iter()
                .find(|h| h.score >= 3.0)
            {
                tracing::info!(domain = %h.domain, topic = %h.topic, score = h.score, "auto-injecting knowledge for weak model");
                let block = format!(
                    "--- 平台知识库·相关最佳实践（{} / {} — {}）：动手前照着来，别凭印象编 ---\n{}",
                    h.domain, h.topic, h.section, h.text
                );
                if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
                    let pos = msgs.len().saturating_sub(1); // before the user's newest message
                    msgs.insert(
                        pos,
                        serde_json::json!({ "role": "system", "content": block }),
                    );
                    prompt_blocks.push("auto_knowledge");
                }
            }
        }
    }
    // 2) inject the requested tool schemas from tools.json (client sends only the NAMES it
    //    selected via its lightweight bundle/catalog logic — never the heavy schema text).
    if let Some(names) = hdr("x-ide-tools") {
        let want = requested_static_tools(mode, names);
        if !want.is_empty() {
            if let Ok(text) = read_tools_file() {
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(serde_json::Value::Array(all)) => {
                        let want: HashSet<&str> = want.iter().map(String::as_str).collect();
                        let picked: Vec<serde_json::Value> = all
                            .into_iter()
                            .filter(|t| {
                                t.get("function")
                                    .and_then(|f| f.get("name"))
                                    .and_then(|n| n.as_str())
                                    .is_some_and(|n| want.contains(n))
                            })
                            .collect();
                        if !picked.is_empty() {
                            // MERGE, don't overwrite: the client may ship MCP/runtime tools
                            // in body.tools that we have no schema for — keep those, append
                            // the static schemas we injected, deduped by function name (so a
                            // name the client already sent is never doubled).
                            let mut merged =
                                match body.get_mut("tools").and_then(|t| t.as_array_mut()) {
                                    Some(arr) => std::mem::take(arr),
                                    None => Vec::new(),
                                };
                            let mut have: HashSet<String> = merged
                                .iter()
                                .filter_map(|t| {
                                    t.get("function")
                                        .and_then(|f| f.get("name"))
                                        .and_then(|n| n.as_str())
                                        .map(|s| s.to_string())
                                })
                                .collect();
                            for t in picked {
                                let name = t
                                    .get("function")
                                    .and_then(|f| f.get("name"))
                                    .and_then(|n| n.as_str())
                                    .map(str::to_string);
                                if name.as_ref().is_some_and(|n| have.insert(n.clone())) {
                                    merged.push(t);
                                }
                            }
                            tracing::info!(
                                mode,
                                requested_tool_count,
                                accepted_static_tool_count = want.len(),
                                final_tool_count = merged.len(),
                                "assembled IDE tools"
                            );
                            body["tools"] = serde_json::Value::Array(merged);
                        } else {
                            tracing::warn!(mode, requested = ?want, "no matching static tools found");
                        }
                    }
                    Ok(_) => tracing::warn!("prompts/tools.json is not a JSON array"),
                    Err(err) => tracing::warn!(%err, "failed to parse prompts/tools.json"),
                }
            }
        }
    }
    let final_tool_count = body
        .get("tools")
        .and_then(|tools| tools.as_array())
        .map_or(0, Vec::len);
    let final_message_count = body
        .get("messages")
        .and_then(|messages| messages.as_array())
        .map_or(0, Vec::len);
    tracing::info!(
        mode,
        prompt_blocks = ?prompt_blocks,
        requested_tool_count,
        final_tool_count,
        final_message_count,
        "assembled IDE prompt request"
    );
    record_agent_trace(AgentTraceInput {
        mode: mode.to_string(),
        prompt_blocks: prompt_blocks.into_iter().map(str::to_string).collect(),
        requested_tool_count,
        injected_tool_count: final_tool_count,
        missing_tool_count: requested_tool_count.saturating_sub(final_tool_count),
        final_message_count,
    });
}

/// Static prompt blobs migrated out of the client. Order is fixed so the version
/// hash is stable for identical content.
const PROMPT_NAMES: &[&str] = &[
    "agent",
    "chat",
    "plan",
    "explorer",
    "reviewer",
    "ui_design_guide",
    "ui_design_flow",
    "css_concrete_tokens",
    // tail (subagent task/system prompts, git guide, small inline utility prompts)
    "subagent_system",
    "worker_system",
    "git_guide",
    "research_prompt",
    "design_research_prompt",
    "next_action",
    "compact",
    "edit_rewrite",
    "edit_transform",
];

/// `GET /api/ide-prompts` — returns `{ version, prompts: { <name>: <text>, ... } }`.
///
/// Gated by the `Claims` extractor (same as `/api/ide-key`): a missing/invalid JWT
/// 401s, so the prompts are only handed to logged-in IDE clients, not the open net.
/// A missing prompt file degrades to an empty string for that key — the IDE falls
/// back to its built-in minimal prompt per key, so a partial deploy can't brick it.
pub async fn ide_prompts(_claims: Claims) -> ApiResult<Json<serde_json::Value>> {
    let mut map = serde_json::Map::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for name in PROMPT_NAMES {
        let text = read_prompt(name).unwrap_or_default();
        text.hash(&mut hasher);
        map.insert((*name).to_string(), serde_json::Value::String(text));
    }
    let version = format!("{:x}", hasher.finish());
    // Also serve the tool schemas (the ~37KB of tool + parameter descriptions) so the IDE
    // can fetch them at runtime instead of shipping them in its bundle — the same migration
    // as the prompts above. Falls back to an empty array if the file is missing, so the IDE
    // keeps its built-in tool fallback and a partial deploy can't break tool-calling.
    let tools = read_tools_file()
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .unwrap_or_else(|| serde_json::Value::Array(vec![]));
    Ok(Json(
        serde_json::json!({ "version": version, "prompts": map, "tools": tools }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_prompts_are_not_empty() {
        for name in ["agent", "chat", "plan", "explorer", "reviewer"] {
            let result = read_prompt(name);
            assert!(result.is_ok(), "prompt {name} should load successfully");
            assert!(!result.unwrap().trim().is_empty(), "prompt {name} is empty");
        }
    }

    #[test]
    fn bundled_tools_are_valid_and_non_empty() {
        let text = read_tools_file().expect("tools.json should be readable");
        let tools: serde_json::Value =
            serde_json::from_str(&text).expect("tools.json should be valid JSON");
        assert!(
            tools.as_array().is_some_and(|items| !items.is_empty()),
            "tools.json should contain at least one tool schema"
        );
    }

    #[test]
    fn rejects_path_traversal_in_prompt_name() {
        let malicious = read_prompt("../../../etc/passwd");
        assert!(malicious.is_err(), "should reject path traversal");

        let malicious2 = read_prompt("../../Cargo.toml");
        assert!(malicious2.is_err(), "should reject relative paths");
    }

    #[test]
    fn truncates_excessive_tool_requests() {
        let many_tools = (0..160)
            .map(|i| format!("tool_{i}"))
            .collect::<Vec<_>>()
            .join(",");
        let result = requested_static_tools("agent", &many_tools);
        assert_eq!(
            result.len(),
            MAX_STATIC_TOOLS_PER_REQUEST,
            "should truncate to cap"
        );
    }

    #[test]
    fn bundled_tool_registry_fits_within_request_cap() {
        let text = read_tools_file().expect("tools.json should be readable");
        let tools: Vec<serde_json::Value> =
            serde_json::from_str(&text).expect("tools.json should be valid JSON");
        assert!(
            tools.len() <= MAX_STATIC_TOOLS_PER_REQUEST,
            "{} bundled tools exceed the request cap of {}",
            tools.len(),
            MAX_STATIC_TOOLS_PER_REQUEST
        );
    }

    #[test]
    fn handles_invalid_mode_gracefully() {
        let result = requested_static_tools("invalid_mode", "read_file,write_file");
        assert!(
            result.is_empty(),
            "unknown mode should return empty tool list"
        );
    }

    #[test]
    fn deduplicates_requested_tools() {
        let result = requested_static_tools("agent", "read_file,read_file,write_file,read_file");
        assert_eq!(result.len(), 2, "should deduplicate tool names");
        assert!(result.contains(&"read_file".to_string()));
        assert!(result.contains(&"write_file".to_string()));
    }

    #[test]
    fn handles_empty_tool_requests() {
        let result = requested_static_tools("agent", "");
        assert!(result.is_empty());

        let result2 = requested_static_tools("agent", "  ,  , ");
        assert!(result2.is_empty(), "should handle whitespace-only input");
    }

    #[test]
    fn chat_mode_restricts_tools_correctly() {
        let result = requested_static_tools("chat", "web_search,web_fetch,read_file,write_file");
        assert!(result.contains(&"web_search".to_string()));
        assert!(result.contains(&"web_fetch".to_string()));
        assert!(
            !result.contains(&"read_file".to_string()),
            "chat should not allow read_file"
        );
        assert!(
            !result.contains(&"write_file".to_string()),
            "chat should not allow write_file"
        );
    }

    #[test]
    fn agent_mode_allows_all_tools() {
        let result = requested_static_tools("agent", "read_file,write_file,run_cmd,git_commit");
        assert_eq!(result.len(), 4, "agent mode should allow all tools");
    }

    #[test]
    fn lite_agent_prompt_exists_and_is_tighter() {
        let full = read_prompt("agent").expect("agent prompt should load");
        let lite = read_prompt("agent_lite").expect("agent_lite prompt should load");
        assert!(!lite.trim().is_empty(), "agent_lite must not be empty");
        assert!(
            lite.len() < full.len(),
            "agent_lite ({}) should be tighter than the full agent prompt ({})",
            lite.len(),
            full.len()
        );
    }

    #[test]
    fn latest_user_text_extracts_string_and_multimodal() {
        let b = serde_json::json!({"messages":[
            {"role":"system","content":"sys"},
            {"role":"user","content":"first"},
            {"role":"assistant","content":"ok"},
            {"role":"user","content":[
                {"type":"text","text":"hello"},
                {"type":"image_url","image_url":{"url":"x"}},
                {"type":"text","text":"world"}
            ]}
        ]});
        assert_eq!(latest_user_text(&b).as_deref(), Some("hello world"));
        let b2 = serde_json::json!({"messages":[{"role":"user","content":"only"}]});
        assert_eq!(latest_user_text(&b2).as_deref(), Some("only"));
        assert!(latest_user_text(&serde_json::json!({"messages":[]})).is_none());
        assert!(latest_user_text(&serde_json::json!({})).is_none());
    }

    #[test]
    fn weak_models_get_lite_frontier_and_reasoners_get_full() {
        // weak / small / fast tiers → lite prompt
        for m in [
            "deepseek-v4-pro",
            "deepseek-chat",
            "gemini-3.5-flash",
            "minimax-m2.5",
            "claude-haiku-4-5-20251001",
            "gpt-5-mini",
            "glm-4-flash",
            "qwen-turbo",
        ] {
            assert!(use_lite_agent_prompt(m), "{m} should use the lite prompt");
        }
        // frontier / reasoners → full prompt (no regression)
        for m in [
            "claude-opus-4-8",
            "claude-fable-5",
            "claude-sonnet-4-6",
            "claude-sonnet-5",
            "gpt-5.5",
            "gemini-3-pro",
            "deepseek-reasoner",
            "deepseek-r1",
        ] {
            assert!(!use_lite_agent_prompt(m), "{m} should keep the full prompt");
        }
    }
}
