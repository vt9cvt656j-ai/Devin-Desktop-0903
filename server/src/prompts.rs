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
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

// The bundled registry currently contains 118 tools. Keep a bounded margin for
// additions while allowing the IDE to send its complete static selection.
const MAX_STATIC_TOOLS_PER_REQUEST: usize = 128;
// L0 defense: the desktop can aggregate tools from several runtime/MCP services before this
// request reaches the server. Bound the final array after every merge so one noisy service cannot
// create an unbounded upstream payload. This limit is the complete compact JSON array, including
// brackets and commas, measured as serialized UTF-8 bytes.
const MAX_FINAL_TOOLS_PER_REQUEST: usize = 128;
const MAX_FINAL_TOOL_SCHEMA_BYTES: usize = 512 * 1024;

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
            "web_search" | "web_fetch" | "knowledge_search" | "local_discovery" | "ask_user"
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
                | "local_discovery"
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
                | "local_discovery"
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

fn tool_function_name(tool: &serde_json::Value) -> Option<&str> {
    tool.pointer("/function/name")
        .and_then(|name| name.as_str())
}

/// Keep the first occurrence of each function name and retain candidates in their input order.
/// Runtime/MCP tools are already at the front of `body.tools`, with requested static tools appended
/// afterward, so applying the final budget here gives runtime capabilities deterministic priority.
fn enforce_final_tool_budget(body: &mut serde_json::Value) -> (usize, usize) {
    let Some(tools) = body.get_mut("tools").and_then(|tools| tools.as_array_mut()) else {
        return (0, 0);
    };

    let candidates = std::mem::take(tools);
    let candidate_count = candidates.len();
    let mut bounded = Vec::with_capacity(candidate_count.min(MAX_FINAL_TOOLS_PER_REQUEST));
    let mut names = HashSet::new();
    // The serialized representation of an empty JSON array is `[]`.
    let mut serialized_bytes = 2usize;

    for tool in candidates {
        let name = tool_function_name(&tool).map(str::to_string);
        if name.as_ref().is_some_and(|name| names.contains(name)) {
            continue;
        }
        if bounded.len() >= MAX_FINAL_TOOLS_PER_REQUEST {
            continue;
        }

        let Ok(tool_bytes) = serde_json::to_vec(&tool) else {
            continue;
        };
        let separator_bytes = usize::from(!bounded.is_empty());
        let Some(next_bytes) = serialized_bytes
            .checked_add(separator_bytes)
            .and_then(|bytes| bytes.checked_add(tool_bytes.len()))
        else {
            continue;
        };
        if next_bytes > MAX_FINAL_TOOL_SCHEMA_BYTES {
            continue;
        }

        if let Some(name) = name {
            names.insert(name);
        }
        serialized_bytes = next_bytes;
        bounded.push(tool);
    }

    *tools = bounded;
    (candidate_count, serialized_bytes)
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

const USER_REQUEST_MARKER: &str = "📌 **用户这次的请求（请正面、直接回应这一条本身）**：";
const USER_STEERING_MARKER: &str = "[MICHAEL_USER_STEERING]";
const AUTO_KNOWLEDGE_MIN_QUERY_CHARS: usize = 12;
const AUTO_KNOWLEDGE_MAX_QUERY_CHARS: usize = 1200;
const AUTO_KNOWLEDGE_MAX_HITS: usize = 2;
const AUTO_KNOWLEDGE_MIN_SCORE: f64 = 3.0;

/// Flatten the textual content of one user message, including multimodal text parts.
fn user_message_text(message: &serde_json::Value) -> Option<String> {
    match message.get("content") {
        Some(serde_json::Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        Some(serde_json::Value::Array(parts)) => {
            let text = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
                .collect::<Vec<_>>()
                .join(" ");
            (!text.trim().is_empty()).then_some(text)
        }
        _ => None,
    }
}

/// The IDE appends the real request after a large, dynamic project-context preamble. Extract only
/// the text after that stable marker so README content, paths, and prior errors cannot dominate
/// knowledge retrieval. Plain clients without the marker continue to use their complete message.
fn extract_real_user_request(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    for marker in [USER_STEERING_MARKER, USER_REQUEST_MARKER] {
        if let Some((_, marked_tail)) = text.rsplit_once(marker) {
            if let Some((_, request)) = marked_tail.split_once("\n\n") {
                let request = request.trim();
                if !request.is_empty() {
                    return Some(request.to_string());
                }
            }
        }
    }
    Some(text.to_string())
}

/// Prefer the most recent explicitly-marked original request or real-time user steering over
/// orchestration nudges. Verification and continuation messages also have role=user, so treating
/// unmarked messages as a new task would make retrieval drift away from the person's request.
fn latest_user_request(body: &serde_json::Value) -> Option<String> {
    let msgs = body.get("messages")?.as_array()?;
    let mut latest_plain = None;
    for m in msgs.iter().rev() {
        if m.get("role").and_then(|r| r.as_str()) != Some("user") {
            continue;
        }
        let Some(text) = user_message_text(m) else {
            continue;
        };
        if latest_plain.is_none() {
            latest_plain = extract_real_user_request(&text);
        }
        if text.contains(USER_STEERING_MARKER) || text.contains(USER_REQUEST_MARKER) {
            return extract_real_user_request(&text);
        }
    }
    latest_plain
}

/// Keep automatic retrieval limited to concrete engineering work. This intentionally requires
/// both an action and a code/project object: long casual questions and generic discussion do not
/// receive an unrelated best-practice dump.
fn looks_like_coding_task(query: &str) -> bool {
    if query.chars().count() < AUTO_KNOWLEDGE_MIN_QUERY_CHARS {
        return false;
    }
    let lower = query.to_lowercase();
    const ACTIONS: &[&str] = &[
        "fix",
        "implement",
        "build",
        "create",
        "add",
        "change",
        "update",
        "refactor",
        "debug",
        "migrate",
        "optimize",
        "integrate",
        "develop",
        "deploy",
        "write",
        "test",
        "修复",
        "解决",
        "实现",
        "开发",
        "新增",
        "添加",
        "修改",
        "改造",
        "重构",
        "迁移",
        "优化",
        "调试",
        "排查",
        "接入",
        "编写",
        "搭建",
        "构建",
        "部署",
        "补测试",
        "改代码",
    ];
    const ENGINEERING_OBJECTS: &[&str] = &[
        "code",
        "project",
        "repository",
        "repo",
        "bug",
        "error",
        "api",
        "frontend",
        "backend",
        "database",
        "component",
        "function",
        "module",
        "service",
        "endpoint",
        "schema",
        "query",
        "architecture",
        "rust",
        "python",
        "javascript",
        "typescript",
        "react",
        "vue",
        "代码",
        "项目",
        "代码库",
        "仓库",
        "报错",
        "错误",
        "接口",
        "前端",
        "后端",
        "数据库",
        "组件",
        "函数",
        "模块",
        "服务",
        "架构",
        "页面",
        "网站",
        "应用",
        "脚本",
        "依赖",
        "构建",
        "测试",
        "性能",
        "安全",
        "鉴权",
        "登录",
        "部署",
        "github",
        "社区",
    ];
    let contains_term = |term: &&str| {
        if term.is_ascii() {
            lower
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
                .any(|word| word == *term)
        } else {
            lower.contains(*term)
        }
    };
    ACTIONS.iter().any(&contains_term) && ENGINEERING_OBJECTS.iter().any(contains_term)
}

fn valid_iana_timezone(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.len() > 64 || (!name.contains('/') && name != "UTC" && name != "GMT")
    {
        return false;
    }
    name.split('/').all(|part| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'_' | b'-' | b'+' | b'.'))
    })
}

fn user_local_time_block_at(headers: &HeaderMap, now_utc: chrono::DateTime<chrono::Utc>) -> String {
    use chrono::{Datelike, Timelike};

    let timezone = headers
        .get("x-ide-timezone")
        .and_then(|value| value.to_str().ok())
        .map(str::trim);
    let offset_minutes = headers
        .get("x-ide-utc-offset-minutes")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<i32>().ok());
    let (timezone, offset_minutes) = match (timezone, offset_minutes) {
        (Some(timezone), Some(offset))
            if valid_iana_timezone(timezone) && (-840..=840).contains(&offset) =>
        {
            (timezone, offset)
        }
        _ => ("UTC", 0),
    };
    let local = now_utc + chrono::Duration::minutes(i64::from(offset_minutes));
    let weekday = ["周日", "周一", "周二", "周三", "周四", "周五", "周六"]
        [local.weekday().num_days_from_sunday() as usize];
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let absolute_offset = offset_minutes.abs();

    format!(
        "【当前真实时间·用户本地】今天是 {}-{:02}-{:02} {} {:02}:{:02}（{}，UTC{}{:02}:{:02}）。凡涉及\"今天/现在/最新/几号/星期几/距某天还有多久\"直接用这个时间算，别猜、别报训练数据里的旧时间。",
        local.year(),
        local.month(),
        local.day(),
        weekday,
        local.hour(),
        local.minute(),
        timezone,
        sign,
        absolute_offset / 60,
        absolute_offset % 60,
    )
}

fn bounded_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

/// Build one bounded, model-independent knowledge block for a concrete agent coding task.
fn auto_knowledge_block(mode: &str, user_request: Option<&str>) -> Option<String> {
    if mode != "agent" {
        return None;
    }
    let request = user_request?.trim();
    if !looks_like_coding_task(request) {
        return None;
    }
    let query = bounded_chars(request, AUTO_KNOWLEDGE_MAX_QUERY_CHARS);
    let hits = crate::knowledge::search(&query, None, AUTO_KNOWLEDGE_MAX_HITS)
        .into_iter()
        .filter(|hit| hit.score >= AUTO_KNOWLEDGE_MIN_SCORE)
        .take(AUTO_KNOWLEDGE_MAX_HITS)
        .collect::<Vec<_>>();
    if hits.is_empty() {
        return None;
    }

    let mut sections = Vec::with_capacity(hits.len());
    for (index, hit) in hits.iter().enumerate() {
        sections.push(format!(
            "【{}｜{}/{} · {}｜相关度 {:.3}】\n{}",
            index + 1,
            hit.domain,
            hit.topic,
            hit.section,
            hit.score,
            hit.text
        ));
    }
    Some(format!(
        "--- 平台知识库·与真实用户请求相关的工程参考（自动检索，最多 {AUTO_KNOWLEDGE_MAX_HITS} 段）---\n\
         这些内容用于提醒常见工程约束，不替代当前项目源码、项目约定和真实构建/测试结果；发生冲突时以后者为准。\n\n{}",
        sections.join("\n\n———\n\n")
    ))
}

/// Decide whether the user's real request needs the UI specialization. Keep generic engineering
/// terms out: a false positive adds several prompt blocks and can steer a backend task toward a
/// frontend stack even though the tool/runtime capabilities themselves remain unchanged.
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
        "布局",
        "按钮",
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
        "优化界面",
        "界面设计",
        "界面样式",
        "界面布局",
        "做界面",
        "做网页",
        "做个站",
        "切图",
        "样式",
        "视觉",
        "视觉稿",
        "交互动效",
    ];
    let contextual_component = q.contains("组件")
        && [
            "ui", "前端", "网页", "页面", "界面", "样式", "布局", "视觉", "按钮", "表单", "react",
            "vue", "svelte", "tailwind", "shadcn",
        ]
        .iter()
        .any(|context| l.contains(context));
    ASCII_KW.iter().any(|k| l.contains(k))
        || CJK_KW.iter().any(|k| q.contains(k))
        || contextual_component
}

fn looks_like_research_task(q: &str) -> bool {
    let lower = q.to_lowercase();
    const ASCII: &[&str] = &[
        "research",
        "latest",
        "current version",
        "compare libraries",
        "compare frameworks",
        "community",
        "forum",
        "github resources",
        "open source",
        "paper",
        "academic",
        "cve",
        "security research",
        "crawler",
        "scraper",
        "scraping",
        "reverse engineer",
        "tor",
        "dark web",
        "game asset",
        "package recommendation",
        "nearby",
        "near me",
        "local food",
        "restaurant",
        "where to eat",
        "travel itinerary",
        "tourist attraction",
    ];
    const CJK: &[&str] = &[
        "调研",
        "研究",
        "最新",
        "现状",
        "社区",
        "论坛",
        "开源资源",
        "代码库资源",
        "选库",
        "技术选型",
        "论文",
        "学术",
        "漏洞情报",
        "安全研究",
        "爬虫",
        "采集",
        "逆向",
        "深网",
        "暗网",
        "游戏资产",
        "资源搜索",
        "比价",
        "二手",
        "附近",
        "周边",
        "好吃",
        "餐厅",
        "旅游",
        "景点",
        "去哪玩",
        "当地美食",
        "行程推荐",
    ];
    let github_research = lower.contains("github")
        && [
            "resource",
            "repository",
            "repositories",
            "repo",
            "project",
            "资源",
            "仓库",
            "项目",
            "开源",
            "推荐",
            "寻找",
            "搜索",
            "找",
        ]
        .iter()
        .any(|term| lower.contains(term));
    github_research
        || ASCII.iter().any(|term| lower.contains(term))
        || CJK.iter().any(|term| q.contains(term))
}

fn looks_like_desktop_automation_task(q: &str) -> bool {
    let lower = q.to_lowercase();
    const ASCII: &[&str] = &[
        "desktop automation",
        "browser automation",
        "rpa",
        "control my computer",
        "control the computer",
        "click through",
        "record workflow",
        "replay workflow",
        "operate the app",
        "open a web page",
        "open the website",
        "log into the website",
        "login to the website",
        "fill out the form",
        "submit the form",
        "screen automation",
        "gui automation",
        "packet capture",
    ];
    const CJK: &[&str] = &[
        "桌面自动化",
        "浏览器自动化",
        "控制电脑",
        "操作电脑",
        "操作软件",
        "打开网页",
        "打开网站",
        "操作网页",
        "操作网站",
        "登录网页",
        "登录网站",
        "填写表单",
        "提交表单",
        "点击界面",
        "录制流程",
        "回放流程",
        "工作流录制",
        "抓包",
        "鼠标键盘",
        "读屏",
        "界面自动化",
    ];
    ASCII.iter().any(|term| lower.contains(term)) || CJK.iter().any(|term| q.contains(term))
}

/// Keep the proven full prompt as the source of truth, but assemble only its stable engineering
/// core. Research, automation, and UI use compact shared blocks
/// assembled below for every model tier. Their legacy monolithic chapters are deliberately omitted.
fn routed_full_agent_prompt(full: &str) -> (String, Vec<&'static str>) {
    let Some(domain_start) = full.find("\n# 九、") else {
        return (full.to_string(), vec!["agent_full_fallback"]);
    };
    let Some(automation_start_rel) = full[domain_start..].find("\n# 十、") else {
        return (full.to_string(), vec!["agent_full_fallback"]);
    };
    let automation_start = domain_start + automation_start_rel;
    let Some(ui_start_rel) = full[automation_start..].find("\n# 十一、") else {
        return (full.to_string(), vec!["agent_full_fallback"]);
    };
    let ui_start = automation_start + ui_start_rel;
    let Some(tail_start_rel) = full[ui_start..].find("\n# 十二、") else {
        return (full.to_string(), vec!["agent_full_fallback"]);
    };
    let tail_start = ui_start + tail_start_rel;

    let mut out = String::with_capacity(full.len());
    let blocks = vec!["agent_core"];
    out.push_str(&full[..domain_start]);
    out.push_str(&full[tail_start..]);
    (out, blocks)
}

fn routed_agent_core(prompt_name: &str, loaded: &str) -> (String, Vec<&'static str>) {
    match prompt_name {
        "agent" => routed_full_agent_prompt(loaded),
        "agent_lite" => (loaded.to_string(), vec!["agent_lite_core"]),
        _ => (loaded.to_string(), vec!["agent_full_fallback"]),
    }
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
    // Snapshot the person's real request before mutating messages. The IDE wraps it in a large
    // dynamic project preamble; retrieval must not search that entire blob.
    let user_request = latest_user_request(body);
    let loaded_prompt = read_prompt(prompt_name)
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
    let (mut sys, routed_blocks) = if mode == "agent" && !loaded_prompt.is_empty() {
        routed_agent_core(prompt_name, &loaded_prompt)
    } else {
        (loaded_prompt, vec![prompt_name])
    };
    if !sys.is_empty() {
        prompt_blocks.extend(routed_blocks);
    }
    // One shared evidence policy covers every IDE mode. Keeping it separate from
    // tone/personality prompts prevents model-specific style tuning from turning
    // guesses or partial integrations into confident product claims.
    let truthfulness = read_prompt("truthfulness").unwrap_or_default();
    if !truthfulness.is_empty() {
        prompt_blocks.push("truthfulness");
        sys.push_str("\n\n");
        sys.push_str(&truthfulness);
    }
    // Research is a shared task specialization, not a model-tier prompt fork. The compact block
    // describes evidence discipline and how to use the capabilities actually present this turn;
    // the old full-prompt chapter embedded a static catalog of every possible tool.
    let research_intent = mode == "agent"
        && user_request
            .as_deref()
            .is_some_and(looks_like_research_task);
    if research_intent {
        let research = read_prompt("agent_research").unwrap_or_default();
        if !research.is_empty() {
            prompt_blocks.push("agent_research");
            sys.push_str("\n\n");
            sys.push_str(&research);
        }
    }
    let automation_intent = mode == "agent"
        && user_request
            .as_deref()
            .is_some_and(looks_like_desktop_automation_task);
    if automation_intent {
        let automation = read_prompt("agent_automation").unwrap_or_default();
        if !automation.is_empty() {
            prompt_blocks.push("agent_automation");
            sys.push_str("\n\n");
            sys.push_str(&automation);
        }
    }
    // Inject the design guide on any UI/frontend task — not just when the (never-emitted)
    // x-ide-ui header is present. `MICHAEL_UI_GUIDE=0` disables; `=always` forces it on.
    let ui_env = std::env::var("MICHAEL_UI_GUIDE").ok();
    let ui_intent = ui_env.as_deref() == Some("always")
        || (ui_env.as_deref() != Some("0")
            && (hdr("x-ide-ui").is_some()
                || user_request
                    .as_deref()
                    .map(|request| {
                        looks_like_ui_task(request) && !looks_like_desktop_automation_task(request)
                    })
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
    // Keep every orchestration block in the leading system message. Inserting a system message
    // near the tail can split assistant(tool_calls) from its required tool response on later
    // agent turns, which OpenAI-compatible APIs correctly reject.
    let needs_reasoning_checkpoint =
        mode == "agent" && user_request.as_deref().is_some_and(looks_like_coding_task);
    if needs_reasoning_checkpoint {
        prompt_blocks.push("reasoning_checkpoint");
        sys.push_str("\n\n⚠️ 强制推理检查点：下一步前先在脑子里快速过一遍——① 我真的理解了吗？② 还缺什么关键信息？③ 这步要拿到什么？④ 可能出什么岔子？除非是显而易见的一步操作（读明确指定的文件、改一行明确的代码），否则先想清楚再动手。");
    }
    if let Some(growth) = hdr("x-ide-growth").map(str::trim).filter(|g| !g.is_empty()) {
        prompt_blocks.push("growth_final_only");
        sys.push_str(&format!(
            "\n\n--- 因人而教（只作用于最终收尾总结）---\n{growth}\n\n执行任务、选择工具、修改代码、验证结果时忽略本段；只在最终回复里用它调整解释深度。"
        ));
    }
    // Model-independent engineering retrieval. Every agent model gets the same bounded
    // reference block for a concrete coding task; prompt tier only changes presentation density.
    // Env MICHAEL_AUTO_KNOWLEDGE=0 remains an operational kill switch.
    if std::env::var("MICHAEL_AUTO_KNOWLEDGE").ok().as_deref() != Some("0") {
        if let Some(block) = auto_knowledge_block(mode, user_request.as_deref()) {
            sys.push_str("\n\n");
            sys.push_str(&block);
            prompt_blocks.push("auto_knowledge");
            tracing::info!(mode, "auto-injecting bounded engineering knowledge");
        }
    }
    // Use the user's browser-provided IANA zone label and current UTC offset. The
    // offset performs the arithmetic (including DST); invalid/missing context falls
    // back to UTC rather than silently reporting the server's or Beijing time.
    if !sys.is_empty() {
        sys = format!(
            "{}\n\n{}",
            user_local_time_block_at(headers, chrono::Utc::now()),
            sys
        );
    }
    let prompt_bytes = sys.len();
    if let Some(msgs) = body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        if !sys.is_empty() {
            msgs.insert(0, serde_json::json!({ "role": "system", "content": sys }));
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
                        let mut catalog: HashMap<String, serde_json::Value> = all
                            .into_iter()
                            .filter_map(|tool| {
                                tool_function_name(&tool)
                                    .map(str::to_string)
                                    .map(|name| (name, tool))
                            })
                            .collect();
                        // Resolve from the ordered request, not registry order. This makes the
                        // behavior stable even if tools.json is reorganized later.
                        let picked: Vec<serde_json::Value> = want
                            .iter()
                            .filter_map(|name| catalog.remove(name))
                            .collect();
                        if !picked.is_empty() {
                            // MERGE, don't overwrite: the client may ship MCP/runtime tools
                            // in body.tools that we have no schema for — keep those, append
                            // the static schemas we injected. The final L0 budget below dedupes
                            // the complete list while preserving runtime priority.
                            let mut merged =
                                match body.get_mut("tools").and_then(|t| t.as_array_mut()) {
                                    Some(arr) => std::mem::take(arr),
                                    None => Vec::new(),
                                };
                            merged.extend(picked);
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
    // Always re-check opted-in requests, including those without x-ide-tools. The client can send
    // runtime/MCP schemas directly, and those need the same cross-service aggregate defense.
    let (candidate_tool_count, budgeted_tool_schema_bytes) = enforce_final_tool_budget(body);
    let final_tool_count = body
        .get("tools")
        .and_then(|tools| tools.as_array())
        .map_or(0, Vec::len);
    let final_message_count = body
        .get("messages")
        .and_then(|messages| messages.as_array())
        .map_or(0, Vec::len);
    let tool_schema_bytes = body
        .get("tools")
        .and_then(|tools| serde_json::to_vec(tools).ok())
        .map_or(0, |tools| tools.len());
    debug_assert!(
        body.get("tools")
            .and_then(|tools| tools.as_array())
            .is_none()
            || tool_schema_bytes == budgeted_tool_schema_bytes
    );
    let request_json_bytes = serde_json::to_vec(body).map_or(0, |request| request.len());
    tracing::info!(
        mode,
        prompt_blocks = ?prompt_blocks,
        requested_tool_count,
        candidate_tool_count,
        final_tool_count,
        final_message_count,
        prompt_bytes,
        tool_schema_bytes,
        request_json_bytes,
        "assembled IDE prompt request"
    );
    record_agent_trace(AgentTraceInput {
        mode: mode.to_string(),
        prompt_blocks: prompt_blocks.into_iter().map(str::to_string).collect(),
        requested_tool_count,
        injected_tool_count: final_tool_count,
        missing_tool_count: requested_tool_count.saturating_sub(final_tool_count),
        final_message_count,
        prompt_bytes,
        tool_schema_bytes,
        request_json_bytes,
    });
}

/// Static prompt blobs migrated out of the client. Order is fixed so the version
/// hash is stable for identical content.
const PROMPT_NAMES: &[&str] = &[
    "agent",
    "agent_lite",
    "agent_research",
    "agent_automation",
    "truthfulness",
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

    fn test_tool(name: &str, description: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": name,
                "description": description,
                "parameters": {"type": "object", "properties": {}}
            }
        })
    }

    #[test]
    fn bundled_prompts_are_not_empty() {
        for name in [
            "agent",
            "agent_lite",
            "agent_research",
            "agent_automation",
            "truthfulness",
            "chat",
            "plan",
            "explorer",
            "reviewer",
        ] {
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
    fn write_file_schema_requires_non_empty_path_and_content() {
        let text = read_tools_file().expect("tools.json should be readable");
        let tools: serde_json::Value =
            serde_json::from_str(&text).expect("tools.json should be valid JSON");
        let write = tools
            .as_array()
            .and_then(|items| {
                items.iter().find(|tool| {
                    tool.pointer("/function/name").and_then(|v| v.as_str()) == Some("write_file")
                })
            })
            .expect("write_file schema should exist");
        let required = write
            .pointer("/function/parameters/required")
            .and_then(|value| value.as_array())
            .expect("write_file should declare required arguments");
        assert!(required.iter().any(|value| value == "path"));
        assert!(required.iter().any(|value| value == "content"));
        assert_eq!(
            write.pointer("/function/parameters/properties/path/minLength"),
            Some(&serde_json::json!(1))
        );
        assert_eq!(
            write.pointer("/function/parameters/properties/content/minLength"),
            Some(&serde_json::json!(1))
        );
        assert_eq!(
            write.pointer("/function/parameters/additionalProperties"),
            Some(&serde_json::json!(false))
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
    fn developer_community_tools_have_cloud_schemas() {
        let text = read_tools_file().expect("tools.json should be readable");
        let tools: Vec<serde_json::Value> =
            serde_json::from_str(&text).expect("tools.json should be valid JSON");
        let names = tools
            .iter()
            .filter_map(|tool| {
                tool.pointer("/function/name")
                    .and_then(|name| name.as_str())
            })
            .collect::<std::collections::HashSet<_>>();
        for required in [
            "developer_community_search",
            "github_search",
            "stackoverflow_search",
            "hackernews_search",
            "devto_search",
            "reddit_search",
            "gitlab_search",
            "gitee_search",
        ] {
            assert!(
                names.contains(required),
                "missing cloud schema for {required}"
            );
        }
    }

    #[test]
    fn git_clone_has_a_complete_agent_only_cloud_schema() {
        let text = read_tools_file().expect("tools.json should be readable");
        let tools: Vec<serde_json::Value> =
            serde_json::from_str(&text).expect("tools.json should be valid JSON");
        let clone = tools
            .iter()
            .find(|tool| {
                tool.pointer("/function/name")
                    .and_then(|name| name.as_str())
                    == Some("git_clone")
            })
            .expect("git_clone should have a cloud schema");
        let required = clone
            .pointer("/function/parameters/required")
            .and_then(|required| required.as_array())
            .expect("git_clone should declare required parameters");
        assert!(required
            .iter()
            .any(|value| value.as_str() == Some("source")));
        assert!(required
            .iter()
            .any(|value| value.as_str() == Some("target")));
        assert_eq!(requested_static_tools("agent", "git_clone"), ["git_clone"]);
        assert!(requested_static_tools("plan", "git_clone").is_empty());
    }

    #[test]
    fn truthfulness_policy_rejects_partial_success_claims() {
        let policy = read_prompt("truthfulness").expect("truthfulness prompt should load");
        assert!(policy.contains("已验证事实"));
        assert!(policy.contains("不等于“接入成功"));
        assert!(policy.contains("逐项报告"));
        assert!(policy.contains("不可信数据"));
        assert!(policy.contains("绝不执行"));
        assert!(policy.contains("搜索是取证手段，不是思考的替代品"));
        assert!(policy.contains("检索轮次不固定"));
        assert!(policy.contains("一轮没有带来新的独立来源"));
        assert!(policy.contains("不因为事实难听"));
        assert!(policy.contains("不得提供可直接用于入侵"));
        assert!(policy.contains("source_statuses[].status == success"));
        assert!(policy.contains("retrieved_at"));
        assert!(policy.contains("source_statuses[].data_as_of"));
        assert!(policy.contains("weather.observed_at"));
        assert!(policy.contains("opening_hours"));
        assert!(policy.contains("缺失的 `rating`、`price`、`open_now` 必须保持未知"));
        assert!(policy.contains("不得把全部结构化地理数据统称为“实时数据”"));
    }

    #[test]
    fn local_discovery_schema_and_mode_access_are_real() {
        let tools: serde_json::Value = serde_json::from_str(&read_tools_file().unwrap()).unwrap();
        let local = tools
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| {
                tool.pointer("/function/name")
                    .and_then(|name| name.as_str())
                    == Some("local_discovery")
            })
            .expect("local_discovery should have a cloud schema");
        let description = local
            .pointer("/function/description")
            .and_then(|value| value.as_str())
            .expect("local_discovery should describe its real data contract");
        for expected in [
            "先使用 Nominatim，无法接受时才后备到 ArcGIS World Geocoding",
            "OpenStreetMap Overpass",
            "Open-Meteo",
            "Haversine 直线距离",
            "source_statuses[].status=success",
            "retrieved_at 是 IDE 本次请求完成时间",
            "source_statuses[].data_as_of（存在时）只是提供方暴露的数据集/快照时间",
            "weather.observed_at 是提供方报告的天气观测时间",
            "缺失的 rating、price、open_now 必须保持未知",
        ] {
            assert!(
                description.contains(expected),
                "local_discovery description should contain {expected}"
            );
        }
        assert_eq!(
            local
                .pointer("/function/parameters/properties/radius_m/maximum")
                .and_then(|value| value.as_u64()),
            Some(20_000)
        );
        assert_eq!(
            local
                .pointer("/function/parameters/properties/latitude/minimum")
                .and_then(|value| value.as_i64()),
            Some(-90)
        );
        assert_eq!(
            local
                .pointer("/function/parameters/anyOf/0/required/0")
                .and_then(|value| value.as_str()),
            Some("near")
        );
        assert_eq!(
            local
                .pointer("/function/parameters/anyOf/1/required/1")
                .and_then(|value| value.as_str()),
            Some("longitude")
        );
        assert_eq!(
            requested_static_tools("chat", "local_discovery"),
            ["local_discovery"]
        );
        assert_eq!(
            requested_static_tools("plan", "local_discovery"),
            ["local_discovery"]
        );
        assert!(looks_like_research_task("我在东京附近想找不只网红店的早餐"));
        assert!(looks_like_research_task(
            "plan a travel itinerary near Kyoto"
        ));
    }

    #[test]
    fn server_assembly_injects_truthfulness_and_community_tools() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        headers.insert(
            "x-ide-tools",
            "developer_community_search,github_search,stackoverflow_search"
                .parse()
                .unwrap(),
        );
        let mut body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "查 Rust async 错误处理"}]
        });

        assemble_into(&headers, &mut body);

        let system = body["messages"][0]["content"]
            .as_str()
            .expect("assembled request should start with a system prompt");
        assert!(system.contains("真实性与证据纪律"));
        let names = body["tools"]
            .as_array()
            .expect("assembled request should contain tools")
            .iter()
            .filter_map(|tool| {
                tool.pointer("/function/name")
                    .and_then(|name| name.as_str())
            })
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(names.len(), 3);
        assert!(names.contains("developer_community_search"));
        assert!(names.contains("github_search"));
        assert!(names.contains("stackoverflow_search"));
    }

    #[test]
    fn server_assembly_preserves_dynamic_mcp_tools_while_adding_selected_static_tools() {
        let mut headers = HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        headers.insert("x-ide-tools", "read_file".parse().unwrap());
        let mut body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "读取当前项目文件"}],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "mcp__memory__read",
                    "description": "workspace MCP tool",
                    "parameters": {"type": "object", "properties": {}}
                }
            }]
        });

        assemble_into(&headers, &mut body);

        let names = body["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| {
                tool.pointer("/function/name")
                    .and_then(|name| name.as_str())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            names
                .iter()
                .filter(|name| **name == "mcp__memory__read")
                .count(),
            1
        );
        assert_eq!(names.iter().filter(|name| **name == "read_file").count(), 1);
        assert_eq!(
            names.len(),
            2,
            "only the selected static schema should be added"
        );
    }

    #[test]
    fn final_budget_caps_runtime_and_static_tools_by_count_and_json_bytes() {
        let bundled: Vec<serde_json::Value> =
            serde_json::from_str(&read_tools_file().expect("tools.json should be readable"))
                .expect("tools.json should be valid JSON");
        let static_names = bundled
            .iter()
            .filter_map(tool_function_name)
            .collect::<Vec<_>>()
            .join(",");
        assert!(
            bundled.len() + 20 > MAX_FINAL_TOOLS_PER_REQUEST,
            "fixture must exercise the aggregate count limit"
        );

        let runtime = (0..20)
            .map(|index| test_tool(&format!("runtime_{index}"), "runtime MCP schema"))
            .collect::<Vec<_>>();
        let mut headers = HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        headers.insert("x-ide-tools", static_names.parse().unwrap());
        let mut body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "check tool budget"}],
            "tools": runtime
        });

        assemble_into(&headers, &mut body);

        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), MAX_FINAL_TOOLS_PER_REQUEST);
        assert!(serde_json::to_vec(tools).unwrap().len() <= MAX_FINAL_TOOL_SCHEMA_BYTES);
        for (index, tool) in tools.iter().take(20).enumerate() {
            assert_eq!(
                tool_function_name(tool),
                Some(format!("runtime_{index}").as_str()),
                "runtime tools must retain priority over appended static tools"
            );
        }
    }

    #[test]
    fn final_budget_measures_unicode_json_bytes_and_skips_oversized_items() {
        let oversized_description = "界".repeat(MAX_FINAL_TOOL_SCHEMA_BYTES);
        let mut headers = HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        let mut body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "check unicode budget"}],
            "tools": [
                test_tool("before", "真实 runtime 工具"),
                test_tool("oversized", &oversized_description),
                test_tool("after", "超大单项之后仍应保留")
            ]
        });

        assemble_into(&headers, &mut body);

        let tools = body["tools"].as_array().unwrap();
        let names = tools
            .iter()
            .filter_map(tool_function_name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["before", "after"]);
        assert!(serde_json::to_vec(tools).unwrap().len() <= MAX_FINAL_TOOL_SCHEMA_BYTES);
    }

    #[test]
    fn final_budget_deduplicates_function_names_and_preserves_requested_static_order() {
        let mut headers = HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        headers.insert("x-ide-tools", "read_file,write_file".parse().unwrap());
        let mut body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "check duplicates"}],
            "tools": [
                test_tool("read_file", "runtime wins"),
                test_tool("read_file", "duplicate runtime loses")
            ]
        });

        assemble_into(&headers, &mut body);

        let tools = body["tools"].as_array().unwrap();
        let names = tools
            .iter()
            .filter_map(tool_function_name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["read_file", "write_file"]);
        assert_eq!(
            tools[0]
                .pointer("/function/description")
                .and_then(|v| v.as_str()),
            Some("runtime wins")
        );

        let mut ordered_headers = HeaderMap::new();
        ordered_headers.insert("x-ide-mode", "agent".parse().unwrap());
        ordered_headers.insert("x-ide-tools", "write_file,read_file".parse().unwrap());
        let mut ordered_body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "check static order"}]
        });
        assemble_into(&ordered_headers, &mut ordered_body);
        let ordered_names = ordered_body["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(tool_function_name)
            .collect::<Vec<_>>();
        assert_eq!(ordered_names, ["write_file", "read_file"]);
    }

    #[test]
    fn opted_in_request_without_static_header_still_gets_final_tool_cap() {
        let tools = (0..(MAX_FINAL_TOOLS_PER_REQUEST + 12))
            .map(|index| test_tool(&format!("runtime_{index}"), "runtime MCP schema"))
            .collect::<Vec<_>>();
        let mut headers = HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        let mut body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "cap runtime tools"}],
            "tools": tools
        });

        assemble_into(&headers, &mut body);

        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), MAX_FINAL_TOOLS_PER_REQUEST);
        assert!(serde_json::to_vec(tools).unwrap().len() <= MAX_FINAL_TOOL_SCHEMA_BYTES);
        assert_eq!(tool_function_name(&tools[0]), Some("runtime_0"));
        assert_eq!(tool_function_name(&tools[127]), Some("runtime_127"));
    }

    #[test]
    fn non_opted_in_request_is_not_modified_by_final_tool_budget() {
        let tools = (0..(MAX_FINAL_TOOLS_PER_REQUEST + 12))
            .map(|index| test_tool(&format!("runtime_{index}"), "runtime MCP schema"))
            .collect::<Vec<_>>();
        let mut body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "leave unchanged"}],
            "tools": tools
        });
        let original = body.clone();

        assemble_into(&HeaderMap::new(), &mut body);

        assert_eq!(body, original);
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
        assert!(!lite.contains("# UI 组件库栈"));
        assert!(!lite.contains("联网研究协议"));
        assert!(lite.contains("UI / 研究 / 自动化专项由系统按用户真实请求动态注入"));
        let (routed_lite, lite_blocks) = routed_agent_core("agent_lite", &lite);
        assert_eq!(routed_lite, lite);
        assert_eq!(lite_blocks, vec!["agent_lite_core"]);
    }

    #[test]
    fn full_agent_prompt_is_routed_by_task_without_losing_the_core() {
        let full = read_prompt("agent").expect("full agent prompt should load");
        let (coding, coding_blocks) = routed_full_agent_prompt(&full);
        assert!(coding.contains("# 一、最高准则"));
        assert!(coding.contains("# 四、写代码的纪律"));
        assert!(coding.contains("# 十二、纪律"));
        assert!(!coding.contains("# 九、领域任务"));
        assert!(!coding.contains("# 十、自动化"));
        assert!(!coding.contains("# 十一、UI / 界面"));
        assert_eq!(coding_blocks, vec!["agent_core"]);
        assert!(
            coding.len() * 5 < full.len() * 3,
            "routine coding prompt should omit at least 40% of irrelevant bytes: {} vs {}",
            coding.len(),
            full.len()
        );

        let (research, research_blocks) = routed_full_agent_prompt(&full);
        assert!(!research.contains("# 九、领域任务"));
        assert_eq!(research_blocks, vec!["agent_core"]);
        assert!(!research.contains("# 十、自动化"));

        let (automation, automation_blocks) = routed_full_agent_prompt(&full);
        assert!(!automation.contains("# 十、自动化"));
        assert_eq!(automation_blocks, vec!["agent_core"]);
        assert!(!automation.contains("# 九、领域任务"));
    }

    #[test]
    fn research_specialization_is_task_routed_for_every_model_tier() {
        for model in ["gpt-5-mini", "gpt-5.5"] {
            let mut headers = HeaderMap::new();
            headers.insert("x-ide-mode", "agent".parse().unwrap());
            let mut body = serde_json::json!({
                "model": model,
                "messages": [{
                    "role": "user",
                    "content": "调研最新开发者社区和 GitHub 资源，比较当前技术选型"
                }]
            });
            assemble_into(&headers, &mut body);
            let system = body["messages"][0]["content"].as_str().unwrap();
            assert!(
                system.contains("# 按任务加载：研究、社区与当前事实"),
                "{model}"
            );
            assert!(!system.contains("# 九、领域任务"), "{model}");
            assert!(!system.contains("开发者资源与专业数据源"), "{model}");

            let mut github_body = serde_json::json!({
                "model": model,
                "messages": [{
                    "role": "user",
                    "content": "找一些 GitHub 上能用的仓库和项目"
                }]
            });
            assemble_into(&headers, &mut github_body);
            assert!(
                github_body["messages"][0]["content"]
                    .as_str()
                    .unwrap()
                    .contains("# 按任务加载：研究、社区与当前事实"),
                "standalone Chinese GitHub request should route research for {model}"
            );

            let mut local_body = serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": "我想知道附近有什么好吃的本地小店"}]
            });
            assemble_into(&headers, &mut local_body);
            let local_system = local_body["messages"][0]["content"].as_str().unwrap();
            assert!(
                local_system.contains("# 按任务加载：研究、社区与当前事实"),
                "{model}"
            );
            assert!(local_system.contains("local_discovery"), "{model}");
        }

        let mut headers = HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        let mut ordinary = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "修复 Rust 服务的空值错误并补测试"}]
        });
        assemble_into(&headers, &mut ordinary);
        assert!(!ordinary["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("# 按任务加载：研究、社区与当前事实"));
    }

    #[test]
    fn automation_specialization_is_task_routed_for_every_model_tier() {
        for model in ["gpt-5-mini", "gpt-5.5"] {
            let mut headers = HeaderMap::new();
            headers.insert("x-ide-mode", "agent".parse().unwrap());
            let mut body = serde_json::json!({
                "model": model,
                "messages": [{
                    "role": "user",
                    "content": "操作桌面软件，自动点击界面并录制回放工作流"
                }]
            });
            assemble_into(&headers, &mut body);
            let system = body["messages"][0]["content"].as_str().unwrap();
            assert!(
                system.contains("# 按任务加载：浏览器与桌面自动化"),
                "{model}"
            );
            assert!(!system.contains("# 十、自动化"), "{model}");
            assert!(!system.contains("# UI 设计 token 与组件契约"), "{model}");
        }

        let mut headers = HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        let mut browser_action = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{
                "role": "user",
                "content": "打开网页帮我登录网站，填写并提交表单"
            }]
        });
        assemble_into(&headers, &mut browser_action);
        let system = browser_action["messages"][0]["content"].as_str().unwrap();
        assert!(system.contains("# 按任务加载：浏览器与桌面自动化"));
        assert!(!system.contains("# UI 设计 token 与组件契约"));
    }

    #[test]
    fn shared_ui_contract_replaces_the_legacy_full_ui_chapter() {
        let full = read_prompt("agent").unwrap();
        let (routed, _) = routed_full_agent_prompt(&full);
        assert!(!routed.contains("# 十一、UI / 界面"));

        let contract = read_prompt("css_concrete_tokens").unwrap();
        assert!(contract.contains("已有项目：现有设计系统是唯一真源"));
        assert!(contract.contains("按需使用 shadcn/ui"));
        assert!(contract.contains("shadcn-vue"));
        assert!(contract.contains("1440x900"));
        assert!(contract.contains("390x844"));
        assert!(!contract.contains("npx shadcn"));
    }

    #[test]
    fn latest_user_request_extracts_plain_and_multimodal_text() {
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
        assert_eq!(latest_user_request(&b).as_deref(), Some("hello world"));
        let b2 = serde_json::json!({"messages":[{"role":"user","content":"only"}]});
        assert_eq!(latest_user_request(&b2).as_deref(), Some("only"));
        assert!(latest_user_request(&serde_json::json!({"messages":[]})).is_none());
        assert!(latest_user_request(&serde_json::json!({})).is_none());
    }

    #[test]
    fn latest_user_request_ignores_dynamic_context_and_later_agent_nudges() {
        let real_request = "实现 Rust Tokio 并发任务，修复 MutexGuard 跨 await，并补充测试";
        let wrapped = format!(
            "--- 项目上下文 ---\nREADME 说这是 React 数据库项目，包含很多无关代码。\n\n\
             ━━━━━━━━━━━━━━━━━━━━━━━━\n{USER_REQUEST_MARKER}上面的项目上下文只是背景参考。\n\n{real_request}"
        );
        let body = serde_json::json!({"messages":[
            {"role":"user","content":wrapped},
            {"role":"assistant","content":"working"},
            {"role":"user","content":"自动验证失败，请继续修复上一条命令"}
        ]});

        assert_eq!(latest_user_request(&body).as_deref(), Some(real_request));
    }

    #[test]
    fn latest_user_request_prefers_real_time_user_steering_over_original_request() {
        let real_request = "修复 Rust 服务的并发错误";
        let wrapped = format!(
            "--- 项目上下文 ---\nREADME 背景。\n\n{USER_REQUEST_MARKER}上面的项目上下文只是背景参考。\n\n{real_request}"
        );
        let steering = "先停下后端工作，改为调研 GitHub 上可用的前端组件仓库";
        let body = serde_json::json!({"messages":[
            {"role":"user","content":wrapped},
            {"role":"assistant","content":"working"},
            {"role":"user","content":format!("{USER_STEERING_MARKER}\n\n{steering}")},
            {"role":"assistant","content":"redirecting"},
            {"role":"user","content":"[系统：继续完成当前任务并执行验证]"}
        ]});

        assert_eq!(latest_user_request(&body).as_deref(), Some(steering));
    }

    #[test]
    fn automatic_engineering_knowledge_is_identical_for_all_model_tiers() {
        let real_request = "实现 Rust Tokio 并发任务，修复 MutexGuard 跨 await，并补充错误处理测试";
        let wrapped = format!(
            "--- 项目上下文 ---\npackage.json 和 README 的大段动态内容。\n\n\
             ━━━━━━━━━━━━━━━━━━━━━━━━\n{USER_REQUEST_MARKER}上面的项目上下文只是背景参考。\n\n{real_request}"
        );
        let mut blocks = Vec::new();

        for model in ["gpt-5-mini", "gpt-5.5", "provider/new-coder-2027"] {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("x-ide-mode", "agent".parse().unwrap());
            let mut body = serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": wrapped}]
            });

            assemble_into(&headers, &mut body);
            let system = body["messages"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|message| message.get("content").and_then(|content| content.as_str()))
                .find(|content| content.contains("--- 平台知识库·与真实用户请求相关"))
                .unwrap_or_else(|| panic!("{model} should receive automatic engineering knowledge"))
                .to_string();
            let marker = system
                .find("--- 平台知识库·与真实用户请求相关")
                .expect("knowledge marker should be in the leading system prompt");
            let block = system[marker..].to_string();
            let hit_count = block.matches("\n【").count();
            assert!((1..=AUTO_KNOWLEDGE_MAX_HITS).contains(&hit_count));
            blocks.push(block);
        }

        assert_eq!(
            blocks[0], blocks[1],
            "weak and frontier models should get the same block"
        );
        assert_eq!(
            blocks[1], blocks[2],
            "unknown models should get the same block too"
        );
    }

    #[test]
    fn orchestration_blocks_never_split_tool_call_adjacency() {
        let real_request = "实现 Rust Tokio 并发任务，修复 MutexGuard 跨 await，并补充错误处理测试";
        let wrapped = format!(
            "--- 项目上下文 ---\nREADME 里的动态内容。\n\n━━━━━━━━━━━━━━━━━━━━━━━━\n{USER_REQUEST_MARKER}上面的项目上下文只是背景参考。\n\n{real_request}"
        );
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-ide-mode", "agent".parse().unwrap());
        headers.insert("x-ide-growth", "summarize concisely".parse().unwrap());
        let mut body = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [
                {"role":"user","content":"earlier 1"},
                {"role":"assistant","content":"reply 1"},
                {"role":"user","content":"earlier 2"},
                {"role":"assistant","content":"reply 2"},
                {"role":"user","content":"earlier 3"},
                {"role":"assistant","content":"reply 3"},
                {"role":"user","content":"earlier 4"},
                {"role":"assistant","content":"reply 4"},
                {"role":"user","content":wrapped},
                {"role":"assistant","content":"","tool_calls":[{"id":"call_1","type":"function","function":{"name":"read_file","arguments":"{\\\"path\\\":\\\"src/lib.rs\\\"}"}}]},
                {"role":"tool","tool_call_id":"call_1","content":"source"}
            ]
        });

        assemble_into(&headers, &mut body);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        let system = messages[0]["content"].as_str().unwrap();
        assert!(system.contains("平台知识库·与真实用户请求相关"));
        assert!(system.contains("强制推理检查点"));
        assert!(system.contains("因人而教"));
        assert_eq!(
            messages
                .iter()
                .filter(|message| message["role"] == "system")
                .count(),
            1,
            "all orchestration content belongs in the leading system message"
        );
        let assistant_index = messages
            .iter()
            .position(|message| message.get("tool_calls").is_some())
            .expect("tool-calling assistant should remain present");
        assert_eq!(messages[assistant_index + 1]["role"], "tool");
        assert_eq!(messages[assistant_index + 1]["tool_call_id"], "call_1");
    }

    #[test]
    fn automatic_knowledge_requires_a_concrete_coding_task() {
        assert!(!looks_like_coding_task(
            "latest React version and current pricing"
        ));
        assert!(!looks_like_coding_task(
            "请简单解释一下这个概念，只回答问题即可"
        ));
        assert!(looks_like_coding_task(
            "请实现 React 登录组件并修复现有项目的鉴权错误"
        ));
        assert!(auto_knowledge_block(
            "chat",
            Some("请实现 React 登录组件并修复现有项目的鉴权错误")
        )
        .is_none());
    }

    #[test]
    fn user_local_time_uses_la_offset_across_previous_date() {
        use chrono::TimeZone;

        let mut headers = HeaderMap::new();
        headers.insert("x-ide-timezone", "America/Los_Angeles".parse().unwrap());
        headers.insert("x-ide-utc-offset-minutes", "-480".parse().unwrap());
        let utc = chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 3, 30, 0)
            .single()
            .unwrap();

        let block = user_local_time_block_at(&headers, utc);
        assert!(block.contains("2025-12-31 周三 19:30"));
        assert!(block.contains("America/Los_Angeles，UTC-08:00"));
    }

    #[test]
    fn user_local_time_uses_beijing_offset_across_next_date() {
        use chrono::TimeZone;

        let mut headers = HeaderMap::new();
        headers.insert("x-ide-timezone", "Asia/Shanghai".parse().unwrap());
        headers.insert("x-ide-utc-offset-minutes", "480".parse().unwrap());
        let utc = chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 20, 30, 0)
            .single()
            .unwrap();

        let block = user_local_time_block_at(&headers, utc);
        assert!(block.contains("2026-01-02 周五 04:30"));
        assert!(block.contains("Asia/Shanghai，UTC+08:00"));
    }

    #[test]
    fn user_local_time_falls_back_to_utc_for_missing_or_invalid_headers() {
        use chrono::TimeZone;

        let utc = chrono::Utc
            .with_ymd_and_hms(2026, 7, 11, 12, 5, 0)
            .single()
            .unwrap();
        let missing = user_local_time_block_at(&HeaderMap::new(), utc);
        assert!(missing.contains("2026-07-11 周六 12:05（UTC，UTC+00:00）"));

        let mut invalid = HeaderMap::new();
        invalid.insert("x-ide-timezone", "../../UTC".parse().unwrap());
        invalid.insert("x-ide-utc-offset-minutes", "900".parse().unwrap());
        let invalid = user_local_time_block_at(&invalid, utc);
        assert!(invalid.contains("2026-07-11 周六 12:05（UTC，UTC+00:00）"));
    }

    #[test]
    fn reasoning_checkpoint_requires_agent_coding_request_not_history_length() {
        let mut agent_headers = HeaderMap::new();
        agent_headers.insert("x-ide-mode", "agent".parse().unwrap());
        let mut first_turn = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{
                "role": "user",
                "content": "请重构整个 Rust 后端认证架构，修复并发错误并补充集成测试"
            }]
        });
        assemble_into(&agent_headers, &mut first_turn);
        assert!(first_turn["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("⚠️ 强制推理检查点"));

        let mut long_chat = serde_json::json!({
            "model": "gpt-5.5",
            "messages": (0..12).map(|index| serde_json::json!({
                "role": if index % 2 == 0 { "user" } else { "assistant" },
                "content": if index == 10 { "请聊聊你最喜欢的电影和音乐，不需要做任何项目" } else { "普通聊天" }
            })).collect::<Vec<_>>()
        });
        assemble_into(&agent_headers, &mut long_chat);
        assert!(!long_chat["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("⚠️ 强制推理检查点"));

        let mut plan_headers = HeaderMap::new();
        plan_headers.insert("x-ide-mode", "plan".parse().unwrap());
        let mut plan = serde_json::json!({
            "model": "gpt-5.5",
            "messages": [{"role": "user", "content": "请实现 Rust 后端认证模块并补测试"}]
        });
        assemble_into(&plan_headers, &mut plan);
        assert!(!plan["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("⚠️ 强制推理检查点"));
    }

    #[test]
    fn chinese_ui_terms_trigger_design_guidance() {
        for request in [
            "优化表单布局和按钮样式",
            "修复手机端视觉与交互动效",
            "调整后台管理页面配色",
        ] {
            assert!(looks_like_ui_task(request), "missed UI request: {request}");
        }
        assert!(looks_like_ui_task("优化 React 登录组件的样式和布局"));
        assert!(!looks_like_ui_task("修复 Rust 服务组件的并发锁错误"));
    }

    #[test]
    fn prompt_catalog_versions_every_routed_prompt_block() {
        for required in [
            "agent",
            "agent_lite",
            "agent_research",
            "agent_automation",
            "ui_design_flow",
            "css_concrete_tokens",
        ] {
            assert!(
                PROMPT_NAMES.contains(&required),
                "missing prompt catalog entry: {required}"
            );
        }
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
