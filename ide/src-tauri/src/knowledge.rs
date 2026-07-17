use chrono::{DateTime, SecondsFormat, Utc};
use futures_util::stream::{FuturesUnordered, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

type DeepSearchHit = (String, String, String, &'static str);
type DeepSearchFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Vec<DeepSearchHit>> + Send>>;
type CommunityAdapterOutput = (&'static str, &'static str, Result<String, String>);
type CommunityAdapterFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = CommunityAdapterOutput> + Send>>;
type CommunitySearchOutput = (&'static str, &'static str, CommunitySearchOutcome);
type CommunitySearchFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = CommunitySearchOutput> + Send>>;
type CommunitySearchResponse = (&'static str, &'static str, CommunitySearchOutcome, String);

const COMMUNITY_SOURCE_TIMEOUT: Duration = Duration::from_secs(12);
const GITHUB_TRENDING_EMPTY_NOTICE: &str =
    "search_status: empty\nNo trending repositories were parsed from the successful GitHub response.\n";

#[derive(Debug)]
enum CommunitySearchOutcome {
    Finished(Result<String, String>),
    TimedOut { after: Duration },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommunitySourceStatus {
    Success,
    Empty,
    RateLimited,
    Failed,
    Timeout,
}

impl CommunitySourceStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Empty => "empty",
            Self::RateLimited => "rate-limited",
            Self::Failed => "failed",
            Self::Timeout => "timeout",
        }
    }
}

#[derive(Clone, Copy)]
struct DiscourseSource {
    key: &'static str,
    label: &'static str,
    base_url: &'static str,
}

const RUST_USERS_DISCOURSE: DiscourseSource = DiscourseSource {
    key: "rust_users",
    label: "Rust Users Forum",
    base_url: "https://users.rust-lang.org",
};
const PYTHON_DISCOURSE: DiscourseSource = DiscourseSource {
    key: "python_discussions",
    label: "Python Discussions",
    base_url: "https://discuss.python.org",
};
const SWIFT_DISCOURSE: DiscourseSource = DiscourseSource {
    key: "swift_forums",
    label: "Swift Forums",
    base_url: "https://forums.swift.org",
};
const KOTLIN_DISCOURSE: DiscourseSource = DiscourseSource {
    key: "kotlin_discussions",
    label: "Kotlin Discussions",
    base_url: "https://discuss.kotlinlang.org",
};

const DEVELOPER_COMMUNITY_SOURCES: &[(&str, &str)] = &[
    ("github", "GitHub"),
    ("github_discussions", "GitHub Discussions"),
    ("stackoverflow", "Stack Overflow"),
    ("hackernews", "Hacker News"),
    ("reddit", "Reddit"),
    ("lobsters", "Lobsters"),
    ("devto", "DEV Community"),
    ("juejin", "掘金"),
    ("v2ex", "V2EX"),
    ("segmentfault", "SegmentFault"),
    ("rust_users", "Rust Users Forum"),
    ("python_discussions", "Python Discussions"),
    ("swift_forums", "Swift Forums"),
    ("kotlin_discussions", "Kotlin Discussions"),
    ("gitlab", "GitLab"),
    ("gitee", "Gitee"),
    ("codeberg", "Codeberg"),
    ("sourcegraph", "Sourcegraph"),
    ("bestofjs", "Best of JS"),
    ("github_trending", "GitHub Trending"),
    ("producthunt", "Product Hunt"),
    ("freecodecamp", "freeCodeCamp"),
    ("infoq", "InfoQ"),
    ("hackernoon", "HackerNoon"),
];

fn canonical_community_source(source: &str) -> Option<&'static str> {
    let normalized = source.trim().to_lowercase().replace([' ', '-', '.'], "_");
    match normalized.as_str() {
        "github" | "gh" => Some("github"),
        "github_discussions" | "gh_discussions" | "discussions" => Some("github_discussions"),
        "stackoverflow" | "stack_overflow" | "so" => Some("stackoverflow"),
        "hackernews" | "hacker_news" | "hn" => Some("hackernews"),
        "reddit" => Some("reddit"),
        "lobsters" => Some("lobsters"),
        "devto" | "dev_to" | "dev" => Some("devto"),
        "juejin" | "掘金" => Some("juejin"),
        "v2ex" => Some("v2ex"),
        "segmentfault" | "segment_fault" | "思否" => Some("segmentfault"),
        "rust" | "rust_users" | "rust_discourse" | "rust_forum" | "users_rust" => {
            Some("rust_users")
        }
        "python" | "python_discussions" | "python_discourse" | "python_forum"
        | "discuss_python" => Some("python_discussions"),
        "swift" | "swift_discourse" | "swift_forums" | "swift_forum" => Some("swift_forums"),
        "kotlin" | "kotlin_discourse" | "kotlin_discussions" | "kotlin_forum"
        | "discuss_kotlin" => Some("kotlin_discussions"),
        "gitlab" => Some("gitlab"),
        "gitee" | "码云" => Some("gitee"),
        "codeberg" => Some("codeberg"),
        "sourcegraph" => Some("sourcegraph"),
        "bestofjs" | "best_of_js" => Some("bestofjs"),
        "github_trending" | "trending" => Some("github_trending"),
        "producthunt" | "product_hunt" => Some("producthunt"),
        "freecodecamp" | "free_code_camp" => Some("freecodecamp"),
        "infoq" => Some("infoq"),
        "hackernoon" | "hacker_noon" => Some("hackernoon"),
        _ => None,
    }
}

fn select_developer_sources(
    scope: Option<&str>,
    requested: Option<&[String]>,
) -> Result<Vec<&'static str>, String> {
    if let Some(requested) = requested.filter(|items| !items.is_empty()) {
        let mut selected = Vec::new();
        let mut unknown = Vec::new();
        for source in requested {
            match canonical_community_source(source) {
                Some(name) if !selected.contains(&name) => selected.push(name),
                Some(_) => {}
                None => unknown.push(source.trim().to_string()),
            }
        }
        if !unknown.is_empty() {
            let supported = DEVELOPER_COMMUNITY_SOURCES
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "Unsupported developer sources: {}. Supported sources: {supported}",
                unknown.join(", ")
            ));
        }
        return Ok(selected);
    }

    let selected = match scope.unwrap_or("all").trim().to_lowercase().as_str() {
        "all" | "" => DEVELOPER_COMMUNITY_SOURCES
            .iter()
            .map(|(name, _)| *name)
            .collect(),
        "code" | "opensource" | "open_source" => vec![
            "github",
            "github_discussions",
            "gitlab",
            "gitee",
            "codeberg",
            "sourcegraph",
            "bestofjs",
            "github_trending",
        ],
        "forums" | "forum" | "qa" => vec![
            "stackoverflow",
            "hackernews",
            "reddit",
            "lobsters",
            "github_discussions",
            "v2ex",
            "segmentfault",
            "rust_users",
            "python_discussions",
            "swift_forums",
            "kotlin_discussions",
        ],
        "chinese" | "zh" | "cn" => {
            vec!["gitee", "juejin", "v2ex", "segmentfault", "infoq"]
        }
        "articles" | "article" | "media" => vec![
            "devto",
            "freecodecamp",
            "infoq",
            "hackernoon",
            "producthunt",
        ],
        other => {
            return Err(format!(
                "Unsupported scope '{other}'. Use all, code, forums, chinese, or articles"
            ))
        }
    };
    Ok(selected)
}

async fn public_site_search(
    query: &str,
    site: &str,
    required_path: Option<&str>,
    label: &str,
    max_results: u32,
    direct_search_url: &str,
) -> Result<String, String> {
    let search_query = format!("site:{site} {query}");
    let hits = ddg_surface_checked(&search_query)
        .await
        .map_err(|error| format!("{label} public search: {error}"))?;
    let retrieved = retrieved_at();
    let mut out = format!(
        "{label} pages for '{query}' (via public web search, not an official {label} API):\nretrieved_at: {retrieved}\n\n"
    );
    let mut count = 0usize;
    for (title, url, snippet, _) in hits {
        if !url.contains(site) || required_path.is_some_and(|path| !url.contains(path)) {
            continue;
        }
        count += 1;
        out.push_str(&format!(
            "{}. {}\n   {}\n   published_date: unknown\n   updated_date: unknown\n   last_activity_date: unknown\n   retrieved_at: {}\n   {}\n\n",
            count,
            title,
            trunc(&snippet, 180),
            retrieved,
            url,
        ));
        if count >= max_results as usize {
            break;
        }
    }
    if count == 0 {
        out.push_str(
            "search_status: empty\nThe public-search response contained no usable matching page. This can mean no match, an index coverage gap, or an unrecognized result layout; it does not prove the site has no matching content.\n\n",
        );
    }
    out.push_str(&format!("Direct search: {direct_search_url}\n"));
    Ok(out)
}

fn url_query_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn kclient() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Michael-IDE/1.0")
        .build()
        .map_err(|e| format!("HTTP client: {e}"))
}

fn trunc(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn retrieved_at() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn unix_time_rfc3339(value: Option<&Value>) -> Option<String> {
    let seconds = value.and_then(|value| match value {
        Value::Number(value) => value.as_i64(),
        Value::String(value) => value.trim().parse::<i64>().ok(),
        _ => None,
    })?;
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|value| value.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn value_or_unknown(value: Option<&str>) -> &str {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
}

fn github_api_url(owner: &str, repo: &str, suffix: &str) -> String {
    let owner = url_query_component(owner);
    let repo = url_query_component(repo);
    if suffix.is_empty() {
        format!("https://api.github.com/repos/{owner}/{repo}")
    } else {
        format!("https://api.github.com/repos/{owner}/{repo}/{suffix}")
    }
}

fn github_text_value(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn repo_text_value(value: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(value) = value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return value.to_string();
        }
    }
    "unknown".to_string()
}

fn github_repo_ref(owner: &str, repo: &str) -> Result<(String, String), String> {
    let clean_owner = owner.trim().trim_matches('/');
    let clean_repo = repo.trim().trim_matches('/');
    if clean_owner.is_empty() || clean_repo.is_empty() {
        return Err("github_repo 需要 owner 和 repo，例如 owner=vercel repo=next.js".into());
    }
    if clean_owner.contains('/') || clean_repo.contains('/') {
        return Err(
            "github_repo 的 owner/repo 必须分开传，不要把 owner/repo 写在同一个字段里".into(),
        );
    }
    Ok((clean_owner.to_string(), clean_repo.to_string()))
}

fn hosted_repo_ref(
    tool: &str,
    owner: &str,
    repo: &str,
    allow_nested_owner: bool,
) -> Result<(String, String), String> {
    let clean_owner = owner.trim().trim_matches('/');
    let clean_repo = repo.trim().trim_matches('/');
    if clean_owner.is_empty() || clean_repo.is_empty() {
        return Err(format!(
            "{tool} 需要 owner 和 repo，例如 owner=gitlab-org repo=gitlab"
        ));
    }
    if clean_repo.contains('/') || (!allow_nested_owner && clean_owner.contains('/')) {
        return Err(format!(
            "{tool} 的 owner/repo 必须分开传；GitLab 子组可写在 owner（如 group/subgroup）"
        ));
    }
    Ok((clean_owner.to_string(), clean_repo.to_string()))
}

fn provider_date_value(value: Option<(&str, &str)>) -> String {
    match value {
        Some((value, field)) if !value.trim().is_empty() => {
            format!("{} (provider field: {field})", trunc(value.trim(), 80))
        }
        _ => "unknown".to_string(),
    }
}

fn provider_date_lines(
    published: Option<(&str, &str)>,
    created: Option<(&str, &str)>,
    updated: Option<(&str, &str)>,
    last_activity: Option<(&str, &str)>,
    retrieved: &str,
) -> String {
    format!(
        concat!(
            "   published_date: {}\n",
            "   created_date: {}\n",
            "   updated_date: {}\n",
            "   last_activity_date: {}\n",
            "   retrieved_at: {}\n",
        ),
        provider_date_value(published),
        provider_date_value(created),
        provider_date_value(updated),
        provider_date_value(last_activity),
        retrieved,
    )
}

fn freecodecamp_published_date(hit: &Value) -> Option<(String, &'static str)> {
    hit.get("publishedAt")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| (value.to_string(), "publishedAt"))
        .or_else(|| {
            unix_time_rfc3339(hit.get("publishedAtTimestamp"))
                .map(|value| (value, "publishedAtTimestamp"))
        })
}

fn repository_date_lines(
    provider: &str,
    item: &Value,
    last_activity_field: Option<&str>,
    retrieved: &str,
) -> String {
    let created = value_or_unknown(item.get("created_at").and_then(Value::as_str));
    let updated = value_or_unknown(item.get("updated_at").and_then(Value::as_str));
    let (last_activity, last_activity_note) = match last_activity_field {
        Some(field) => (
            value_or_unknown(item.get(field).and_then(Value::as_str)),
            format!(" (provider field: {field})"),
        ),
        None => (
            "unknown",
            format!(" ({provider} repository response did not expose a last-activity field)"),
        ),
    };
    format!(
        concat!(
            "   published_date: unknown ({} repository search does not expose a publication date)\n",
            "   created_date: {} (provider field: created_at)\n",
            "   updated_date: {} (provider field: updated_at)\n",
            "   last_activity_date: {}{}\n",
            "   retrieved_at: {}\n",
        ),
        provider,
        created,
        updated,
        last_activity,
        last_activity_note,
        retrieved,
    )
}

fn community_result_status(outcome: &CommunitySearchOutcome) -> CommunitySourceStatus {
    match outcome {
        CommunitySearchOutcome::TimedOut { .. } => CommunitySourceStatus::Timeout,
        CommunitySearchOutcome::Finished(result) => match result {
            Err(error) => {
                let error = error.to_ascii_lowercase();
                if error.contains("429")
                    || error.contains("rate limit")
                    || error.contains("rate-limit")
                    || error.contains("too many requests")
                    || error.contains("quota exceeded")
                {
                    CommunitySourceStatus::RateLimited
                } else {
                    CommunitySourceStatus::Failed
                }
            }
            Ok(content) => {
                let normalized = content.trim().to_ascii_lowercase();
                if normalized.is_empty()
                    || normalized.contains("search_status: empty")
                    || normalized.starts_with("no ")
                    || normalized.contains("\nno matching page was returned")
                    || normalized.contains("\n  no results found")
                    || normalized.contains("\n  no matching projects")
                {
                    CommunitySourceStatus::Empty
                } else {
                    CommunitySourceStatus::Success
                }
            }
        },
    }
}

async fn community_source_with_timeout(
    source_key: &'static str,
    source_label: &'static str,
    adapter: CommunityAdapterFuture,
    timeout: Duration,
) -> CommunitySearchOutput {
    match tokio::time::timeout(timeout, adapter).await {
        Ok((key, label, result)) => (key, label, CommunitySearchOutcome::Finished(result)),
        Err(_) => (
            source_key,
            source_label,
            CommunitySearchOutcome::TimedOut { after: timeout },
        ),
    }
}

fn aggregate_trending_language(query: &str) -> Option<&'static str> {
    match query.trim().to_ascii_lowercase().as_str() {
        "c" => Some("c"),
        "c++" | "cpp" => Some("c++"),
        "c#" | "csharp" => Some("c#"),
        "dart" => Some("dart"),
        "elixir" => Some("elixir"),
        "go" | "golang" => Some("go"),
        "haskell" => Some("haskell"),
        "java" => Some("java"),
        "javascript" | "js" => Some("javascript"),
        "julia" => Some("julia"),
        "kotlin" => Some("kotlin"),
        "lua" => Some("lua"),
        "objective-c" | "objective_c" => Some("objective-c"),
        "php" => Some("php"),
        "python" | "py" => Some("python"),
        "r" => Some("r"),
        "ruby" => Some("ruby"),
        "rust" => Some("rust"),
        "scala" => Some("scala"),
        "shell" | "bash" => Some("shell"),
        "swift" => Some("swift"),
        "typescript" | "ts" => Some("typescript"),
        _ => None,
    }
}

fn github_trending_url(language: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse("https://github.com/trending")
        .map_err(|error| format!("GitHub Trending URL: {error}"))?;
    let language = language.trim();
    if !language.is_empty() && !language.eq_ignore_ascii_case("all") {
        let language = language.to_ascii_lowercase().replace(' ', "-");
        url.path_segments_mut()
            .map_err(|_| "GitHub Trending URL cannot accept path segments".to_string())?
            .push(&language);
    }
    url.query_pairs_mut().append_pair("since", "weekly");
    Ok(url.into())
}

// ── Academic papers (Semantic Scholar) ──────────────────────────────

#[tauri::command]
pub async fn academic_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(8).min(20);

    let resp = c
        .get("https://api.semanticscholar.org/graph/v1/paper/search")
        .query(&[
            ("query", query.as_str()),
            ("limit", &limit.to_string()),
            (
                "fields",
                "title,abstract,year,citationCount,url,authors,externalIds",
            ),
        ])
        .send()
        .await
        .map_err(|e| format!("Semantic Scholar: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Semantic Scholar returned {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| format!("JSON: {e}"))?;
    let total = json["total"].as_u64().unwrap_or(0);
    let mut out = format!("Found {total} papers (showing top {limit}):\n\n");

    if let Some(papers) = json["data"].as_array() {
        for (i, p) in papers.iter().enumerate() {
            let title = p["title"].as_str().unwrap_or("?");
            let year = p["year"]
                .as_u64()
                .map(|y| y.to_string())
                .unwrap_or_default();
            let cites = p["citationCount"].as_u64().unwrap_or(0);
            let url = p["url"].as_str().unwrap_or("");
            let abs = p["abstract"].as_str().unwrap_or("(no abstract)");

            let authors: Vec<&str> = p["authors"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect())
                .unwrap_or_default();
            let auth_str = if authors.len() > 3 {
                format!("{}, {} et al.", authors[0], authors[1])
            } else {
                authors.join(", ")
            };

            let arxiv = p["externalIds"]["ArXiv"]
                .as_str()
                .map(|id| format!(" | arXiv: https://arxiv.org/abs/{id}"))
                .unwrap_or_default();

            out.push_str(&format!(
                "{}. {} ({})\n   Authors: {}\n   Citations: {}{}\n   {}\n   {}\n\n",
                i + 1,
                title,
                year,
                auth_str,
                cites,
                arxiv,
                url,
                trunc(abs, 300)
            ));
        }
    }
    Ok(out)
}

// ── Package registries (npm / crates.io / PyPI / HuggingFace / pub.dev / Conda / CocoaPods / Hex) ──

#[tauri::command]
pub async fn package_search(
    query: String,
    ecosystem: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(8).min(20);
    let eco = ecosystem.as_deref().unwrap_or("npm");

    match eco {
        "npm" => search_npm(&c, &query, limit).await,
        "pypi" | "python" => search_pypi(&c, &query).await,
        "crates" | "rust" => search_crates(&c, &query, limit).await,
        "huggingface" | "hf" => search_hf(&c, &query, limit).await,
        "dart" | "flutter" | "pub" => search_pub(&c, &query, limit).await,
        "conda" | "anaconda" => search_conda(&c, &query, limit).await,
        "swift" | "cocoapods" | "ios" => search_cocoapods(&c, &query, limit).await,
        "elixir" | "hex" | "erlang" => search_hex(&c, &query, limit).await,
        _ => Err(format!(
            "Unknown ecosystem '{eco}'. Use: npm, pypi, crates, huggingface, dart, conda, cocoapods, hex"
        )),
    }
}

async fn search_npm(c: &Client, q: &str, limit: u32) -> Result<String, String> {
    let exact = npm_exact_summary(c, q).await.ok().flatten();
    let resp = match c
        .get("https://registry.npmjs.org/-/v1/search")
        .query(&[("text", q), ("size", &limit.to_string())])
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            if let Some(info) = exact {
                return Ok(format!("npm packages:\n\n{info}"));
            }
            return Err(format!("npm: {e}"));
        }
    };
    let json: Value = match resp.json().await {
        Ok(json) => json,
        Err(e) => {
            if let Some(info) = exact {
                return Ok(format!("npm packages:\n\n{info}"));
            }
            return Err(format!("npm JSON: {e}"));
        }
    };
    let mut out = String::from("npm packages:\n\n");
    if let Some(info) = exact {
        out.push_str(&info);
        out.push('\n');
    }
    if let Some(objs) = json["objects"].as_array() {
        for (i, o) in objs.iter().enumerate() {
            let p = &o["package"];
            out.push_str(&format!(
                "{}. {} v{}\n   {}\n   https://www.npmjs.com/package/{}\n\n",
                i + 1,
                p["name"].as_str().unwrap_or("?"),
                p["version"].as_str().unwrap_or("?"),
                p["description"].as_str().unwrap_or(""),
                p["name"].as_str().unwrap_or(""),
            ));
        }
    }
    Ok(out)
}

fn npm_exact_query(q: &str) -> bool {
    let q = q.trim();
    !q.is_empty()
        && q.len() <= 214
        && !q.chars().any(char::is_whitespace)
        && q.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '/' | '-' | '_' | '.' | '~'))
}

fn npm_registry_path(name: &str) -> String {
    let mut out = String::new();
    for (part_index, part) in name.trim().split('/').filter(|part| !part.is_empty()).enumerate() {
        if part_index > 0 {
            out.push_str("%2F");
        }
        for byte in part.as_bytes() {
            match *byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*byte as char),
                other => out.push_str(&format!("%{other:02X}")),
            }
        }
    }
    out
}

fn npm_value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        Value::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "?".to_string()),
    }
}

fn npm_map_text(value: Option<&Value>, max: usize) -> Option<String> {
    let obj = value.and_then(Value::as_object)?;
    if obj.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for (key, value) in obj.iter().take(max) {
        parts.push(format!("{key}: {}", npm_value_text(value)));
    }
    if obj.len() > max {
        parts.push(format!("… +{} more", obj.len() - max));
    }
    Some(parts.join(", "))
}

fn npm_recent_versions(data: &Value, max: usize) -> Vec<String> {
    let Some(versions) = data.get("versions").and_then(Value::as_object) else {
        return Vec::new();
    };
    let time = data.get("time").and_then(Value::as_object);
    let mut rows: Vec<(String, String)> = versions
        .keys()
        .map(|version| {
            let published = time
                .and_then(|items| items.get(version))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            (version.clone(), published)
        })
        .collect();
    if time.is_some() {
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    } else {
        rows.sort_by(|a, b| b.0.cmp(&a.0));
    }
    rows.into_iter().take(max).map(|row| row.0).collect()
}

async fn npm_exact_summary(c: &Client, q: &str) -> Result<Option<String>, String> {
    if !npm_exact_query(q) {
        return Ok(None);
    }
    let url = format!("https://registry.npmjs.org/{}", npm_registry_path(q));
    let resp = c
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("npm exact: {e}"))?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let data: Value = resp.json().await.map_err(|e| format!("npm exact JSON: {e}"))?;
    let name = data.get("name").and_then(Value::as_str).unwrap_or(q.trim());
    let latest = data
        .get("dist-tags")
        .and_then(|tags| tags.get("latest"))
        .and_then(Value::as_str)
        .unwrap_or("?");
    let latest_manifest = data
        .get("versions")
        .and_then(|versions| versions.get(latest));
    let versions_count = data
        .get("versions")
        .and_then(Value::as_object)
        .map(|versions| versions.len())
        .unwrap_or(0);
    let recent = npm_recent_versions(&data, 10);
    let modified = data
        .get("time")
        .and_then(|time| time.get("modified"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut out = format!(
        "Exact npm registry metadata:\n   Package: {name}@{latest}\n   Modified: {modified}\n   Registry: https://www.npmjs.com/package/{name}\n"
    );
    if let Some(tags) = npm_map_text(data.get("dist-tags"), 8) {
        out.push_str(&format!("   dist-tags: {tags}\n"));
    }
    if recent.is_empty() {
        out.push_str(&format!("   Versions: {versions_count} total\n"));
    } else {
        out.push_str(&format!(
            "   Versions: {versions_count} total; recent: {}\n",
            recent.join(", ")
        ));
    }
    for (label, value, max) in [
        ("engines", latest_manifest.and_then(|m| m.get("engines")), 6),
        ("peerDependencies", latest_manifest.and_then(|m| m.get("peerDependencies")), 8),
        ("dependencies", latest_manifest.and_then(|m| m.get("dependencies")), 10),
        (
            "optionalDependencies",
            latest_manifest.and_then(|m| m.get("optionalDependencies")),
            6,
        ),
    ] {
        if let Some(text) = npm_map_text(value, max) {
            out.push_str(&format!("   {label}: {text}\n"));
        }
    }
    if let Some(deprecated) = latest_manifest
        .and_then(|m| m.get("deprecated"))
        .and_then(Value::as_str)
    {
        out.push_str(&format!("   Deprecated: {}\n", trunc(deprecated, 300)));
    }
    Ok(Some(out))
}

async fn search_pypi(c: &Client, q: &str) -> Result<String, String> {
    let resp = c
        .get(format!("https://pypi.org/pypi/{q}/json"))
        .send()
        .await
        .map_err(|e| format!("PyPI: {e}"))?;
    if !resp.status().is_success() {
        return Ok(format!(
            "Package '{q}' not found on PyPI. Note: PyPI only supports exact name lookup, not search. \
             Try the exact package name, or use web_search to discover Python packages."
        ));
    }
    let json: Value = resp.json().await.map_err(|e| format!("PyPI JSON: {e}"))?;
    let info = &json["info"];
    Ok(format!(
        "PyPI package:\n\n{} v{}\n{}\nAuthor: {}\nLicense: {}\nPython: {}\nhttps://pypi.org/project/{}/\n",
        info["name"].as_str().unwrap_or("?"),
        info["version"].as_str().unwrap_or("?"),
        info["summary"].as_str().unwrap_or(""),
        info["author"].as_str().unwrap_or("?"),
        info["license"].as_str().unwrap_or("?"),
        info["requires_python"].as_str().unwrap_or("any"),
        info["name"].as_str().unwrap_or(""),
    ))
}

async fn search_crates(c: &Client, q: &str, limit: u32) -> Result<String, String> {
    let resp = c
        .get("https://crates.io/api/v1/crates")
        .query(&[
            ("q", q),
            ("per_page", &limit.to_string()),
            ("sort", "downloads"),
        ])
        .send()
        .await
        .map_err(|e| format!("crates.io: {e}"))?;
    let json: Value = resp.json().await.map_err(|e| format!("crates JSON: {e}"))?;
    let mut out = String::from("crates.io packages:\n\n");
    if let Some(crates) = json["crates"].as_array() {
        for (i, cr) in crates.iter().enumerate() {
            let name = cr["name"].as_str().unwrap_or("?");
            out.push_str(&format!(
                "{}. {} v{}\n   {}\n   Downloads: {} | https://crates.io/crates/{}\n\n",
                i + 1,
                name,
                cr["max_version"].as_str().unwrap_or("?"),
                cr["description"].as_str().unwrap_or("").trim(),
                cr["downloads"].as_u64().unwrap_or(0),
                name,
            ));
        }
    }
    Ok(out)
}

async fn search_hf(c: &Client, q: &str, limit: u32) -> Result<String, String> {
    let resp = c
        .get("https://huggingface.co/api/models")
        .query(&[
            ("search", q),
            ("limit", &limit.to_string()),
            ("sort", "downloads"),
            ("direction", "-1"),
        ])
        .send()
        .await
        .map_err(|e| format!("HuggingFace: {e}"))?;
    let json: Value = resp.json().await.map_err(|e| format!("HF JSON: {e}"))?;
    let mut out = String::from("HuggingFace models:\n\n");
    if let Some(models) = json.as_array() {
        for (i, m) in models.iter().enumerate() {
            let id = m["modelId"].as_str().unwrap_or("?");
            out.push_str(&format!(
                "{}. {}\n   Pipeline: {} | Downloads: {} | Likes: {}\n   https://huggingface.co/{}\n\n",
                i + 1,
                id,
                m["pipeline_tag"].as_str().unwrap_or("?"),
                m["downloads"].as_u64().unwrap_or(0),
                m["likes"].as_u64().unwrap_or(0),
                id,
            ));
        }
    }
    Ok(out)
}

async fn search_pub(c: &Client, q: &str, limit: u32) -> Result<String, String> {
    let resp = c
        .get("https://pub.dev/api/search")
        .query(&[("q", q)])
        .send()
        .await
        .map_err(|e| format!("pub.dev: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("pub.dev returned {}", resp.status()));
    }
    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("pub.dev JSON: {e}"))?;
    let mut out = String::from("pub.dev (Dart/Flutter) packages:\n\n");
    if let Some(packages) = json["packages"].as_array() {
        for (i, p) in packages.iter().take(limit as usize).enumerate() {
            let name = p["package"].as_str().unwrap_or("?");
            if let Ok(dr) = c
                .get(format!("https://pub.dev/api/packages/{name}"))
                .send()
                .await
            {
                if let Ok(d) = dr.json::<Value>().await {
                    let ps = &d["latest"]["pubspec"];
                    out.push_str(&format!(
                        "{}. {} v{}\n   {}\n   https://pub.dev/packages/{}\n\n",
                        i + 1,
                        name,
                        ps["version"].as_str().unwrap_or("?"),
                        trunc(ps["description"].as_str().unwrap_or("").trim(), 200),
                        name,
                    ));
                    continue;
                }
            }
            out.push_str(&format!(
                "{}. {}\n   https://pub.dev/packages/{}\n\n",
                i + 1,
                name,
                name
            ));
        }
    }
    Ok(out)
}

async fn search_conda(c: &Client, q: &str, limit: u32) -> Result<String, String> {
    let resp = c
        .get("https://api.anaconda.org/search")
        .query(&[("name", q)])
        .send()
        .await
        .map_err(|e| format!("Anaconda: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Anaconda returned {}", resp.status()));
    }
    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("Anaconda JSON: {e}"))?;
    let mut out = String::from("Anaconda/Conda packages:\n\n");
    if let Some(pkgs) = json.as_array() {
        for (i, p) in pkgs.iter().take(limit as usize).enumerate() {
            out.push_str(&format!(
                "{}. {}/{} v{}\n   {}\n   Downloads: {} | https://anaconda.org/{}/{}\n\n",
                i + 1,
                p["owner"].as_str().unwrap_or("?"),
                p["name"].as_str().unwrap_or("?"),
                p["version"].as_str().unwrap_or("?"),
                trunc(p["summary"].as_str().unwrap_or(""), 200),
                p["ndownloads"].as_u64().unwrap_or(0),
                p["owner"].as_str().unwrap_or("_"),
                p["name"].as_str().unwrap_or(""),
            ));
        }
    }
    Ok(out)
}

async fn search_cocoapods(c: &Client, q: &str, limit: u32) -> Result<String, String> {
    let resp = c
        .get("https://search.cocoapods.org/api/v1/pods.flat.hash.json")
        .query(&[("query", q), ("amount", &limit.to_string())])
        .send()
        .await
        .map_err(|e| format!("CocoaPods: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CocoaPods returned {}", resp.status()));
    }
    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("CocoaPods JSON: {e}"))?;
    let mut out = String::from("CocoaPods (iOS/macOS) pods:\n\n");
    if let Some(pods) = json.as_array() {
        for (i, p) in pods.iter().enumerate() {
            let id = p["id"].as_str().unwrap_or("?");
            out.push_str(&format!(
                "{}. {} v{}\n   {}\n   {} | https://cocoapods.org/pods/{}\n\n",
                i + 1,
                id,
                p["version"].as_str().unwrap_or("?"),
                trunc(p["summary"].as_str().unwrap_or(""), 200),
                p["source"]
                    .as_object()
                    .and_then(|s| s.get("git"))
                    .and_then(|g| g.as_str())
                    .unwrap_or(""),
                id,
            ));
        }
    }
    Ok(out)
}

async fn search_hex(c: &Client, q: &str, limit: u32) -> Result<String, String> {
    let resp = c
        .get("https://hex.pm/api/packages")
        .query(&[("search", q), ("sort", "downloads")])
        .send()
        .await
        .map_err(|e| format!("Hex.pm: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Hex.pm returned {}", resp.status()));
    }
    let json: Value = resp.json().await.map_err(|e| format!("Hex JSON: {e}"))?;
    let mut out = String::from("Hex.pm (Elixir/Erlang) packages:\n\n");
    if let Some(pkgs) = json.as_array() {
        for (i, p) in pkgs.iter().take(limit as usize).enumerate() {
            let name = p["name"].as_str().unwrap_or("?");
            let desc = p["meta"]["description"].as_str().unwrap_or("");
            let dl = p["downloads"]["all"].as_u64().unwrap_or(0);
            out.push_str(&format!(
                "{}. {}\n   {}\n   Downloads: {} | https://hex.pm/packages/{}\n\n",
                i + 1,
                name,
                trunc(desc, 200),
                dl,
                name,
            ));
        }
    }
    Ok(out)
}

// ── GitHub search ──────────────────────────────────────────────────

fn format_github_search_item(
    item: &Value,
    search_type: &str,
    index: usize,
    retrieved: &str,
) -> String {
    if search_type == "repositories" {
        let dates = repository_date_lines("GitHub", item, Some("pushed_at"), retrieved);
        return format!(
            "{}. {} ({}★)\n   {}\n   Language: {} | Forks: {}\n{}   {}\n\n",
            index + 1,
            item["full_name"].as_str().unwrap_or("?"),
            item["stargazers_count"].as_u64().unwrap_or(0),
            item["description"].as_str().unwrap_or(""),
            item["language"].as_str().unwrap_or("?"),
            item["forks_count"].as_u64().unwrap_or(0),
            dates,
            item["html_url"].as_str().unwrap_or(""),
        );
    }

    let (created, updated) = if search_type == "issues" {
        (
            value_or_unknown(item.get("created_at").and_then(Value::as_str)),
            value_or_unknown(item.get("updated_at").and_then(Value::as_str)),
        )
    } else {
        ("unknown", "unknown")
    };
    let label = item["full_name"]
        .as_str()
        .or(item["name"].as_str())
        .or(item["title"].as_str())
        .or(item["path"].as_str())
        .unwrap_or("?");
    format!(
        "{}. {}\n   published_date: unknown (GitHub {search_type} search does not expose a publication date)\n   created_date: {created}{}\n   updated_date: {updated}{}\n   last_activity_date: unknown (GitHub {search_type} search does not expose a last-activity field)\n   retrieved_at: {retrieved}\n   {}\n\n",
        index + 1,
        label,
        if search_type == "issues" {
            " (provider field: created_at)"
        } else {
            ""
        },
        if search_type == "issues" {
            " (provider field: updated_at)"
        } else {
            ""
        },
        item["html_url"].as_str().unwrap_or(""),
    )
}

#[tauri::command]
pub async fn github_search(
    query: String,
    search_type: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(30);
    let stype = search_type.as_deref().unwrap_or("repositories");

    let url = format!("https://api.github.com/search/{stype}");
    let resp = c
        .get(&url)
        .query(&[("q", query.as_str()), ("per_page", &limit.to_string())])
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("GitHub: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("GitHub error: {body}"));
    }

    let json: Value = resp.json().await.map_err(|e| format!("GitHub JSON: {e}"))?;
    let total = json["total_count"].as_u64().unwrap_or(0);
    let retrieved = retrieved_at();
    let mut out = format!("GitHub {stype}: {total} results\nretrieved_at: {retrieved}\n\n");
    if total == 0 {
        out.push_str("search_status: empty\n");
    }

    if let Some(items) = json["items"].as_array() {
        for (i, item) in items.iter().enumerate() {
            out.push_str(&format_github_search_item(item, stype, i, &retrieved));
        }
    }
    Ok(out)
}

fn format_github_repo_overview(repo: &Value, retrieved: &str) -> String {
    let full_name = github_text_value(repo, "full_name");
    let dates = repository_date_lines("GitHub", repo, Some("pushed_at"), retrieved);
    format!(
        concat!(
            "GitHub repo overview: {}\n",
            "search_status: success\n",
            "source: GitHub REST API /repos/{{owner}}/{{repo}}\n",
            "retrieved_at: {}\n\n",
            "Description: {}\n",
            "Language: {}\n",
            "Stars: {}\n",
            "Forks: {}\n",
            "Open issues: {}\n",
            "Default branch: {}\n",
            "License: {}\n",
            "Homepage: {}\n",
            "URL: {}\n",
            "{}"
        ),
        full_name,
        retrieved,
        repo.get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        github_text_value(repo, "language"),
        repo.get("stargazers_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        repo.get("forks_count").and_then(Value::as_u64).unwrap_or(0),
        repo.get("open_issues_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        github_text_value(repo, "default_branch"),
        repo.pointer("/license/spdx_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        github_text_value(repo, "homepage"),
        github_text_value(repo, "html_url"),
        dates,
    )
}

fn decode_github_base64(value: &str) -> Result<Vec<u8>, String> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(value.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0u8;
    for byte in value.bytes() {
        let val = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\r' | b'\n' | b'\t' | b' ' => continue,
            _ => return Err("GitHub 文件内容不是合法 base64".into()),
        };
        debug_assert_eq!(TABLE[val as usize], byte);
        buf = (buf << 6) | val as u32;
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
        buf &= if bits == 0 { 0 } else { (1u32 << bits) - 1 };
    }
    Ok(out)
}

fn github_auth_header(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match std::env::var("GITHUB_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        Some(token) => req.header("Authorization", format!("Bearer {token}")),
        None => req,
    }
}

fn gitlab_auth_header(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let token = std::env::var("GITLAB_TOKEN")
        .or_else(|_| std::env::var("GITLAB_PERSONAL_ACCESS_TOKEN"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    match token {
        Some(token) => req.header("PRIVATE-TOKEN", token),
        None => req,
    }
}

fn codeberg_auth_header(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let token = std::env::var("CODEBERG_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    match token {
        Some(token) => req.header("Authorization", format!("token {token}")),
        None => req,
    }
}

fn with_gitee_token(url: String) -> String {
    let Some(token) = std::env::var("GITEE_ACCESS_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return url;
    };
    let sep = if url.contains('?') { '&' } else { '?' };
    format!("{url}{sep}access_token={}", url_query_component(&token))
}

async fn api_get_json(
    c: &Client,
    url: &str,
    label: &str,
    decorate: impl Fn(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
) -> Result<Value, String> {
    let resp = decorate(c.get(url))
        .send()
        .await
        .map_err(|e| format!("{label}: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("{label} returned {status}: {}", trunc(&body, 800)));
    }
    resp.json().await.map_err(|e| format!("{label} JSON: {e}"))
}

async fn api_get_text(
    c: &Client,
    url: &str,
    label: &str,
    decorate: impl Fn(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
) -> Result<String, String> {
    let resp = decorate(c.get(url))
        .send()
        .await
        .map_err(|e| format!("{label}: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("{label} returned {status}: {}", trunc(&body, 800)));
    }
    resp.text().await.map_err(|e| format!("{label} text: {e}"))
}

async fn github_get_json(c: &Client, url: &str) -> Result<Value, String> {
    api_get_json(c, url, "GitHub", |req| {
        github_auth_header(req.header("Accept", "application/vnd.github+json"))
    })
    .await
}

fn repo_reader_action(value: Option<String>) -> String {
    let action = value.unwrap_or_else(|| "overview".into());
    match action
        .trim()
        .to_lowercase()
        .replace(['-', ' '], "_")
        .as_str()
    {
        "summary" | "info" | "repo" | "overview" => "overview".into(),
        "readme" | "read_me" => "readme".into(),
        "tree" | "list" | "dir" | "directory" | "contents" => "tree".into(),
        "file" | "read_file" | "content" => "file".into(),
        "releases" | "release" | "versions" => "releases".into(),
        "issues" | "issue" => "issues".into(),
        "pulls" | "pull_requests" | "prs" | "pr" | "merge_requests" | "mrs" | "mr" => {
            "pulls".into()
        }
        other => other.to_string(),
    }
}

fn github_repo_action(value: Option<String>) -> String {
    repo_reader_action(value)
}

/// Read a real GitHub repository through the GitHub REST API.
///
/// This is intentionally separate from `github_search`: search finds candidates; this tool reads
/// the selected repo's README/tree/file/releases/issues so the model can reason from repository
/// evidence instead of titles and snippets.
#[tauri::command]
pub async fn github_repo(
    owner: String,
    repo: String,
    action: Option<String>,
    path: Option<String>,
    branch: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let (owner, repo) = github_repo_ref(&owner, &repo)?;
    let action = github_repo_action(action);
    let c = kclient()?;
    let retrieved = retrieved_at();
    let limit = max_results.unwrap_or(20).min(100);
    let branch = branch.unwrap_or_default();
    let branch = branch.trim();

    match action.as_str() {
        "overview" => {
            let url = github_api_url(&owner, &repo, "");
            let json = github_get_json(&c, &url).await?;
            Ok(format_github_repo_overview(&json, &retrieved))
        }
        "readme" => {
            let mut url = github_api_url(&owner, &repo, "readme");
            if !branch.is_empty() {
                url.push_str(&format!("?ref={}", url_query_component(branch)));
            }
            let json = github_get_json(&c, &url).await?;
            let encoded = json.get("content").and_then(Value::as_str).unwrap_or("");
            let bytes = decode_github_base64(encoded)?;
            let text = String::from_utf8_lossy(&bytes);
            let file_path = json.get("path").and_then(Value::as_str).unwrap_or("README");
            Ok(format!(
                "GitHub repo README: {owner}/{repo}/{file_path}\nsearch_status: success\nsource: GitHub REST API /readme\nretrieved_at: {retrieved}\nencoding: {}\nsize_bytes: {}\nhtml_url: {}\n\n{}",
                github_text_value(&json, "encoding"),
                bytes.len(),
                github_text_value(&json, "html_url"),
                trunc(&text, 60_000),
            ))
        }
        "tree" => {
            let clean_path = path.unwrap_or_default().trim().trim_matches('/').to_string();
            let mut url = github_api_url(
                &owner,
                &repo,
                &format!("contents/{}", url_query_component(&clean_path).replace("%2F", "/")),
            );
            if !branch.is_empty() {
                url.push_str(&format!("?ref={}", url_query_component(branch)));
            }
            let json = github_get_json(&c, &url).await?;
            let mut out = format!(
                "GitHub repo tree: {owner}/{repo}/{}\nsearch_status: success\nsource: GitHub REST API /contents\nretrieved_at: {retrieved}\n\n",
                if clean_path.is_empty() { "." } else { clean_path.as_str() },
            );
            if let Some(items) = json.as_array() {
                for (i, item) in items.iter().take(limit as usize).enumerate() {
                    out.push_str(&format!(
                        "{}. {} {} ({} bytes)\n   path: {}\n   url: {}\n",
                        i + 1,
                        item.get("type").and_then(Value::as_str).unwrap_or("?"),
                        item.get("name").and_then(Value::as_str).unwrap_or("?"),
                        item.get("size").and_then(Value::as_u64).unwrap_or(0),
                        item.get("path").and_then(Value::as_str).unwrap_or("?"),
                        item.get("html_url").and_then(Value::as_str).unwrap_or(""),
                    ));
                }
                if items.len() > limit as usize {
                    out.push_str(&format!(
                        "\ntruncated: true (showing {limit} of {} entries; pass max_results for more)\n",
                        items.len()
                    ));
                }
            } else {
                out.push_str("The requested path is not a directory; use action=file to read it.\n");
            }
            Ok(out)
        }
        "file" => {
            let clean_path = path
                .map(|p| p.trim().trim_matches('/').to_string())
                .filter(|p| !p.is_empty())
                .ok_or_else(|| "github_repo action=file 需要 path，例如 src/main.ts".to_string())?;
            let mut url = github_api_url(
                &owner,
                &repo,
                &format!("contents/{}", url_query_component(&clean_path).replace("%2F", "/")),
            );
            if !branch.is_empty() {
                url.push_str(&format!("?ref={}", url_query_component(branch)));
            }
            let json = github_get_json(&c, &url).await?;
            if json.as_array().is_some() {
                return Err("github_repo action=file 收到目录；请改用 action=tree".into());
            }
            let encoded = json.get("content").and_then(Value::as_str).unwrap_or("");
            let bytes = decode_github_base64(encoded)?;
            let text = String::from_utf8_lossy(&bytes);
            Ok(format!(
                "GitHub repo file: {owner}/{repo}/{clean_path}\nsearch_status: success\nsource: GitHub REST API /contents\nretrieved_at: {retrieved}\nencoding: {}\nsize_bytes: {}\nsha: {}\nhtml_url: {}\n\n{}",
                github_text_value(&json, "encoding"),
                bytes.len(),
                github_text_value(&json, "sha"),
                github_text_value(&json, "html_url"),
                trunc(&text, 80_000),
            ))
        }
        "releases" => {
            let url = github_api_url(&owner, &repo, "releases");
            let json = github_get_json(&c, &format!("{url}?per_page={limit}")).await?;
            let mut out = format!(
                "GitHub repo releases: {owner}/{repo}\nsearch_status: success\nsource: GitHub REST API /releases\nretrieved_at: {retrieved}\n\n"
            );
            if let Some(items) = json.as_array() {
                if items.is_empty() {
                    out.push_str("search_status: empty\nNo releases returned by GitHub.\n");
                }
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&format!(
                        "{}. {}{}\n   published_date: {}\n   created_date: {}\n   prerelease: {} | draft: {}\n   url: {}\n   notes: {}\n\n",
                        i + 1,
                        item.get("tag_name").and_then(Value::as_str).unwrap_or("?"),
                        item.get("name").and_then(Value::as_str).map(|name| format!(" — {name}")).unwrap_or_default(),
                        provider_date_value(item.get("published_at").and_then(Value::as_str).map(|v| (v, "published_at"))),
                        provider_date_value(item.get("created_at").and_then(Value::as_str).map(|v| (v, "created_at"))),
                        item.get("prerelease").and_then(Value::as_bool).unwrap_or(false),
                        item.get("draft").and_then(Value::as_bool).unwrap_or(false),
                        item.get("html_url").and_then(Value::as_str).unwrap_or(""),
                        trunc(item.get("body").and_then(Value::as_str).unwrap_or(""), 1200),
                    ));
                }
            }
            Ok(out)
        }
        "issues" | "pulls" => {
            let endpoint = if action == "pulls" { "pulls" } else { "issues" };
            let url = github_api_url(&owner, &repo, endpoint);
            let json = github_get_json(&c, &format!("{url}?state=open&per_page={limit}")).await?;
            let mut out = format!(
                "GitHub repo {endpoint}: {owner}/{repo}\nsearch_status: success\nsource: GitHub REST API /{endpoint}\nretrieved_at: {retrieved}\nstate: open\n\n"
            );
            if let Some(items) = json.as_array() {
                if items.is_empty() {
                    out.push_str("search_status: empty\nNo open items returned by GitHub.\n");
                }
                for (i, item) in items.iter().enumerate() {
                    out.push_str(&format!(
                        "{}. #{} {}\n   created_date: {}\n   updated_date: {}\n   user: {}\n   comments: {}\n   url: {}\n   body: {}\n\n",
                        i + 1,
                        item.get("number").and_then(Value::as_u64).unwrap_or(0),
                        item.get("title").and_then(Value::as_str).unwrap_or("?"),
                        provider_date_value(item.get("created_at").and_then(Value::as_str).map(|v| (v, "created_at"))),
                        provider_date_value(item.get("updated_at").and_then(Value::as_str).map(|v| (v, "updated_at"))),
                        item.pointer("/user/login").and_then(Value::as_str).unwrap_or("unknown"),
                        item.get("comments").and_then(Value::as_u64).unwrap_or(0),
                        item.get("html_url").and_then(Value::as_str).unwrap_or(""),
                        trunc(item.get("body").and_then(Value::as_str).unwrap_or(""), 900),
                    ));
                }
            }
            Ok(out)
        }
        other => Err(format!(
            "unknown github_repo action={other}; allowed: overview, readme, tree, file, releases, issues, pulls"
        )),
    }
}

fn repo_array_items(json: &Value) -> Option<&Vec<Value>> {
    json.as_array()
        .or_else(|| json.get("data").and_then(Value::as_array))
}

fn repo_item_url(item: &Value) -> String {
    repo_text_value(item, &["html_url", "web_url", "url", "download_url"])
}

fn repo_item_number(item: &Value) -> String {
    item.get("number")
        .and_then(Value::as_u64)
        .map(|value| format!("#{value}"))
        .or_else(|| {
            item.get("iid")
                .and_then(Value::as_u64)
                .map(|value| format!("!{value}"))
        })
        .or_else(|| {
            item.get("number")
                .and_then(Value::as_str)
                .map(|value| format!("#{value}"))
        })
        .unwrap_or_default()
}

fn repo_u64_value(item: &Value, keys: &[&str]) -> u64 {
    for key in keys {
        if let Some(value) = item.get(*key).and_then(Value::as_u64) {
            return value;
        }
    }
    0
}

fn repo_topics(item: &Value) -> String {
    item.get("topics")
        .or_else(|| item.get("tag_list"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".into())
}

fn format_hosted_repo_tree(
    provider: &str,
    repo_name: &str,
    path: &str,
    source: &str,
    retrieved: &str,
    json: &Value,
    limit: u32,
) -> String {
    let shown_path = if path.is_empty() { "." } else { path };
    let mut out = format!(
        "{provider} repo tree: {repo_name}/{shown_path}\nsearch_status: success\nsource: {source}\nretrieved_at: {retrieved}\n\n",
    );
    if let Some(items) = json.as_array() {
        for (i, item) in items.iter().take(limit as usize).enumerate() {
            let size = item
                .get("size")
                .and_then(Value::as_u64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".into());
            out.push_str(&format!(
                "{}. {} {}\n   path: {}\n   size_bytes: {}\n   sha/id: {}\n   url: {}\n",
                i + 1,
                item.get("type").and_then(Value::as_str).unwrap_or("?"),
                item.get("name").and_then(Value::as_str).unwrap_or("?"),
                item.get("path").and_then(Value::as_str).unwrap_or("?"),
                size,
                repo_text_value(item, &["sha", "id"]),
                repo_item_url(item),
            ));
        }
        if items.len() > limit as usize {
            out.push_str(&format!(
                "\ntruncated: true (showing {limit} of {} entries; pass max_results for more)\n",
                items.len()
            ));
        }
    } else {
        out.push_str("The requested path is not a directory; use action=file to read it.\n");
    }
    out
}

fn format_hosted_repo_file(
    provider: &str,
    repo_name: &str,
    path: &str,
    source: &str,
    retrieved: &str,
    encoding: &str,
    bytes: &[u8],
    sha: &str,
    html_url: &str,
) -> String {
    let text = String::from_utf8_lossy(bytes);
    format!(
        "{provider} repo file: {repo_name}/{path}\nsearch_status: success\nsource: {source}\nretrieved_at: {retrieved}\nencoding: {encoding}\nsize_bytes: {}\nsha/id: {sha}\nhtml_url: {html_url}\n\n{}",
        bytes.len(),
        trunc(&text, 80_000),
    )
}

fn decode_repo_content_base64(provider: &str, encoded: &str) -> Result<Vec<u8>, String> {
    decode_github_base64(encoded)
        .map_err(|_| format!("{provider} 文件内容不是合法 base64 或接口未返回 content 字段"))
}

fn format_hosted_repo_releases(
    provider: &str,
    repo_name: &str,
    source: &str,
    retrieved: &str,
    json: &Value,
) -> String {
    let mut out = format!(
        "{provider} repo releases: {repo_name}\nsearch_status: success\nsource: {source}\nretrieved_at: {retrieved}\n\n"
    );
    if let Some(items) = repo_array_items(json) {
        if items.is_empty() {
            out.push_str("search_status: empty\nNo releases returned by the provider.\n");
        }
        for (i, item) in items.iter().enumerate() {
            out.push_str(&format!(
                "{}. {}{}\n   published_date: {}\n   created_date: {}\n   updated_date: {}\n   prerelease: {} | draft: {}\n   url: {}\n   notes: {}\n\n",
                i + 1,
                repo_text_value(item, &["tag_name", "tag"]),
                item.get("name")
                    .and_then(Value::as_str)
                    .map(|name| format!(" — {name}"))
                    .unwrap_or_default(),
                provider_date_value(item.get("released_at")
                    .or_else(|| item.get("published_at"))
                    .and_then(Value::as_str)
                    .map(|value| (value, if item.get("released_at").is_some() { "released_at" } else { "published_at" }))),
                provider_date_value(item.get("created_at").and_then(Value::as_str).map(|value| (value, "created_at"))),
                provider_date_value(item.get("updated_at").and_then(Value::as_str).map(|value| (value, "updated_at"))),
                item.get("prerelease").and_then(Value::as_bool).unwrap_or(false),
                item.get("draft").and_then(Value::as_bool).unwrap_or(false),
                repo_item_url(item),
                trunc(item.get("description")
                    .or_else(|| item.get("body"))
                    .and_then(Value::as_str)
                    .unwrap_or(""), 1200),
            ));
        }
    }
    out
}

fn format_hosted_repo_items(
    provider: &str,
    repo_name: &str,
    label: &str,
    source: &str,
    retrieved: &str,
    json: &Value,
) -> String {
    let mut out = format!(
        "{provider} repo {label}: {repo_name}\nsearch_status: success\nsource: {source}\nretrieved_at: {retrieved}\nstate: open\n\n"
    );
    if let Some(items) = repo_array_items(json) {
        if items.is_empty() {
            out.push_str("search_status: empty\nNo open items returned by the provider.\n");
        }
        for (i, item) in items.iter().enumerate() {
            let number = repo_item_number(item);
            out.push_str(&format!(
                "{}. {} {}\n   created_date: {}\n   updated_date: {}\n   user: {}\n   comments: {}\n   url: {}\n   body: {}\n\n",
                i + 1,
                number,
                item.get("title").and_then(Value::as_str).unwrap_or("?"),
                provider_date_value(item.get("created_at").and_then(Value::as_str).map(|value| (value, "created_at"))),
                provider_date_value(item.get("updated_at").and_then(Value::as_str).map(|value| (value, "updated_at"))),
                item.pointer("/author/username")
                    .or_else(|| item.pointer("/author/name"))
                    .or_else(|| item.pointer("/user/login"))
                    .or_else(|| item.pointer("/user/username"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                repo_u64_value(item, &["comments", "comments_count"]),
                repo_item_url(item),
                trunc(item.get("description")
                    .or_else(|| item.get("body"))
                    .and_then(Value::as_str)
                    .unwrap_or(""), 900),
            ));
        }
    }
    out
}

fn format_gitlab_repo_overview(repo: &Value, retrieved: &str) -> String {
    let full_name = repo_text_value(repo, &["path_with_namespace", "name_with_namespace"]);
    let dates = repository_date_lines("GitLab", repo, Some("last_activity_at"), retrieved);
    format!(
        concat!(
            "GitLab repo overview: {}\n",
            "search_status: success\n",
            "source: GitLab REST API /projects/{{urlencoded path}}\n",
            "retrieved_at: {}\n\n",
            "Description: {}\n",
            "Topics: {}\n",
            "Stars: {}\n",
            "Forks: {}\n",
            "Open issues: {}\n",
            "Default branch: {}\n",
            "Visibility: {}\n",
            "URL: {}\n",
            "{}"
        ),
        full_name,
        retrieved,
        repo.get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        repo_topics(repo),
        repo_u64_value(repo, &["star_count", "stars_count", "stargazers_count"]),
        repo_u64_value(repo, &["forks_count"]),
        repo_u64_value(repo, &["open_issues_count"]),
        repo_text_value(repo, &["default_branch"]),
        repo_text_value(repo, &["visibility"]),
        repo_text_value(repo, &["web_url", "html_url"]),
        dates,
    )
}

fn format_gitee_repo_overview(repo: &Value, retrieved: &str) -> String {
    let full_name = repo_text_value(repo, &["full_name", "human_name", "name"]);
    let dates = repository_date_lines("Gitee", repo, Some("pushed_at"), retrieved);
    format!(
        concat!(
            "Gitee repo overview: {}\n",
            "search_status: success\n",
            "source: Gitee REST API /repos/{{owner}}/{{repo}}\n",
            "retrieved_at: {}\n\n",
            "Description: {}\n",
            "Language: {}\n",
            "Stars: {}\n",
            "Forks: {}\n",
            "Open issues: {}\n",
            "Default branch: {}\n",
            "URL: {}\n",
            "{}"
        ),
        full_name,
        retrieved,
        repo.get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        repo_text_value(repo, &["language"]),
        repo_u64_value(repo, &["stargazers_count", "stars_count"]),
        repo_u64_value(repo, &["forks_count"]),
        repo_u64_value(repo, &["open_issues_count"]),
        repo_text_value(repo, &["default_branch"]),
        repo_text_value(repo, &["html_url", "web_url"]),
        dates,
    )
}

fn format_codeberg_repo_overview(repo: &Value, retrieved: &str) -> String {
    let full_name = repo_text_value(repo, &["full_name", "name"]);
    let dates = repository_date_lines("Codeberg", repo, None, retrieved);
    format!(
        concat!(
            "Codeberg repo overview: {}\n",
            "search_status: success\n",
            "source: Codeberg/Gitea REST API /repos/{{owner}}/{{repo}}\n",
            "retrieved_at: {}\n\n",
            "Description: {}\n",
            "Language: {}\n",
            "Stars: {}\n",
            "Forks: {}\n",
            "Open issues: {}\n",
            "Default branch: {}\n",
            "URL: {}\n",
            "{}"
        ),
        full_name,
        retrieved,
        repo.get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        repo_text_value(repo, &["language"]),
        repo_u64_value(repo, &["stars_count", "stargazers_count"]),
        repo_u64_value(repo, &["forks_count"]),
        repo_u64_value(repo, &["open_issues_count"]),
        repo_text_value(repo, &["default_branch"]),
        repo_text_value(repo, &["html_url", "web_url"]),
        dates,
    )
}

fn gitlab_project_id(owner: &str, repo: &str) -> String {
    url_query_component(&format!("{owner}/{repo}"))
}

fn gitlab_project_url(owner: &str, repo: &str, suffix: &str) -> String {
    let id = gitlab_project_id(owner, repo);
    if suffix.is_empty() {
        format!("https://gitlab.com/api/v4/projects/{id}")
    } else {
        format!("https://gitlab.com/api/v4/projects/{id}/{suffix}")
    }
}

async fn gitlab_get_json(c: &Client, url: &str) -> Result<Value, String> {
    api_get_json(c, url, "GitLab", gitlab_auth_header).await
}

async fn gitlab_read_raw_file(
    c: &Client,
    owner: &str,
    repo: &str,
    path: &str,
    branch: &str,
) -> Result<String, String> {
    let url = gitlab_project_url(
        owner,
        repo,
        &format!(
            "repository/files/{}/raw?ref={}",
            url_query_component(path),
            url_query_component(branch),
        ),
    );
    api_get_text(c, &url, "GitLab", gitlab_auth_header).await
}

/// Read a real GitLab.com public or token-authorized repository.
#[tauri::command]
pub async fn gitlab_repo(
    owner: String,
    repo: String,
    action: Option<String>,
    path: Option<String>,
    branch: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let (owner, repo) = hosted_repo_ref("gitlab_repo", &owner, &repo, true)?;
    let action = repo_reader_action(action);
    let c = kclient()?;
    let retrieved = retrieved_at();
    let limit = max_results.unwrap_or(20).min(100);
    let branch = branch.unwrap_or_default();
    let branch = branch.trim();

    match action.as_str() {
        "overview" => {
            let json = gitlab_get_json(&c, &gitlab_project_url(&owner, &repo, "")).await?;
            Ok(format_gitlab_repo_overview(&json, &retrieved))
        }
        "readme" => {
            let overview = gitlab_get_json(&c, &gitlab_project_url(&owner, &repo, "")).await?;
            let default_branch = repo_text_value(&overview, &["default_branch"]);
            let branch = if branch.is_empty() {
                default_branch.as_str()
            } else {
                branch
            };
            let mut last_error = String::new();
            for candidate in ["README.md", "README.rst", "README.txt", "README", "readme.md"] {
                match gitlab_read_raw_file(&c, &owner, &repo, candidate, branch).await {
                    Ok(text) => {
                        return Ok(format!(
                            "GitLab repo README: {owner}/{repo}/{candidate}\nsearch_status: success\nsource: GitLab REST API /repository/files/raw\nretrieved_at: {retrieved}\nencoding: utf-8/raw\nsize_bytes: {}\nhtml_url: https://gitlab.com/{owner}/{repo}/-/blob/{}/{}\n\n{}",
                            text.len(),
                            url_query_component(branch).replace("%2F", "/"),
                            url_query_component(candidate).replace("%2F", "/"),
                            trunc(&text, 60_000),
                        ));
                    }
                    Err(error) => last_error = error,
                }
            }
            Err(format!(
                "GitLab README not found via common names on branch {branch}: {last_error}"
            ))
        }
        "tree" => {
            let clean_path = path.unwrap_or_default().trim().trim_matches('/').to_string();
            let mut url = gitlab_project_url(
                &owner,
                &repo,
                &format!("repository/tree?per_page={limit}"),
            );
            if !branch.is_empty() {
                url.push_str(&format!("&ref={}", url_query_component(branch)));
            }
            if !clean_path.is_empty() {
                url.push_str(&format!("&path={}", url_query_component(&clean_path)));
            }
            let json = gitlab_get_json(&c, &url).await?;
            Ok(format_hosted_repo_tree(
                "GitLab",
                &format!("{owner}/{repo}"),
                &clean_path,
                "GitLab REST API /repository/tree",
                &retrieved,
                &json,
                limit,
            ))
        }
        "file" => {
            let clean_path = path
                .map(|p| p.trim().trim_matches('/').to_string())
                .filter(|p| !p.is_empty())
                .ok_or_else(|| "gitlab_repo action=file 需要 path，例如 src/main.ts".to_string())?;
            let overview = gitlab_get_json(&c, &gitlab_project_url(&owner, &repo, "")).await?;
            let default_branch = repo_text_value(&overview, &["default_branch"]);
            let branch = if branch.is_empty() {
                default_branch.as_str()
            } else {
                branch
            };
            let text = gitlab_read_raw_file(&c, &owner, &repo, &clean_path, branch).await?;
            Ok(format!(
                "GitLab repo file: {owner}/{repo}/{clean_path}\nsearch_status: success\nsource: GitLab REST API /repository/files/raw\nretrieved_at: {retrieved}\nencoding: utf-8/raw\nsize_bytes: {}\nsha/id: unknown\nhtml_url: https://gitlab.com/{owner}/{repo}/-/blob/{}/{}\n\n{}",
                text.len(),
                url_query_component(branch).replace("%2F", "/"),
                url_query_component(&clean_path).replace("%2F", "/"),
                trunc(&text, 80_000),
            ))
        }
        "releases" => {
            let url = gitlab_project_url(&owner, &repo, &format!("releases?per_page={limit}"));
            let json = gitlab_get_json(&c, &url).await?;
            Ok(format_hosted_repo_releases(
                "GitLab",
                &format!("{owner}/{repo}"),
                "GitLab REST API /releases",
                &retrieved,
                &json,
            ))
        }
        "issues" | "pulls" => {
            let (endpoint, label) = if action == "pulls" {
                ("merge_requests", "merge_requests")
            } else {
                ("issues", "issues")
            };
            let state = if action == "pulls" { "opened" } else { "opened" };
            let url = gitlab_project_url(
                &owner,
                &repo,
                &format!("{endpoint}?state={state}&per_page={limit}"),
            );
            let json = gitlab_get_json(&c, &url).await?;
            Ok(format_hosted_repo_items(
                "GitLab",
                &format!("{owner}/{repo}"),
                label,
                &format!("GitLab REST API /{endpoint}"),
                &retrieved,
                &json,
            ))
        }
        other => Err(format!(
            "unknown gitlab_repo action={other}; allowed: overview, readme, tree, file, releases, issues, pulls"
        )),
    }
}

fn gitee_api_url(owner: &str, repo: &str, suffix: &str) -> String {
    let owner = url_query_component(owner);
    let repo = url_query_component(repo);
    let url = if suffix.is_empty() {
        format!("https://gitee.com/api/v5/repos/{owner}/{repo}")
    } else {
        format!("https://gitee.com/api/v5/repos/{owner}/{repo}/{suffix}")
    };
    with_gitee_token(url)
}

async fn gitee_get_json(c: &Client, url: &str) -> Result<Value, String> {
    api_get_json(c, url, "Gitee", |req| req).await
}

fn gitee_ref_query(branch: &str) -> String {
    if branch.trim().is_empty() {
        String::new()
    } else {
        format!("ref={}", url_query_component(branch.trim()))
    }
}

/// Read a real Gitee public or token-authorized repository.
#[tauri::command]
pub async fn gitee_repo(
    owner: String,
    repo: String,
    action: Option<String>,
    path: Option<String>,
    branch: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let (owner, repo) = hosted_repo_ref("gitee_repo", &owner, &repo, false)?;
    let action = repo_reader_action(action);
    let c = kclient()?;
    let retrieved = retrieved_at();
    let limit = max_results.unwrap_or(20).min(100);
    let branch = branch.unwrap_or_default();
    let branch = branch.trim();

    match action.as_str() {
        "overview" => {
            let json = gitee_get_json(&c, &gitee_api_url(&owner, &repo, "")).await?;
            Ok(format_gitee_repo_overview(&json, &retrieved))
        }
        "readme" => {
            let ref_query = gitee_ref_query(branch);
            let suffix = if ref_query.is_empty() {
                "readme".to_string()
            } else {
                format!("readme?{ref_query}")
            };
            let json = gitee_get_json(&c, &gitee_api_url(&owner, &repo, &suffix)).await?;
            let encoded = json.get("content").and_then(Value::as_str).unwrap_or("");
            let bytes = decode_repo_content_base64("Gitee", encoded)?;
            let file_path = json.get("path").and_then(Value::as_str).unwrap_or("README");
            Ok(format!(
                "Gitee repo README: {owner}/{repo}/{file_path}\nsearch_status: success\nsource: Gitee REST API /readme\nretrieved_at: {retrieved}\nencoding: {}\nsize_bytes: {}\nhtml_url: {}\n\n{}",
                repo_text_value(&json, &["encoding"]),
                bytes.len(),
                repo_text_value(&json, &["html_url"]),
                trunc(&String::from_utf8_lossy(&bytes), 60_000),
            ))
        }
        "tree" => {
            let clean_path = path.unwrap_or_default().trim().trim_matches('/').to_string();
            let encoded_path = url_query_component(&clean_path).replace("%2F", "/");
            let mut suffix = if clean_path.is_empty() {
                "contents/".to_string()
            } else {
                format!("contents/{encoded_path}")
            };
            let ref_query = gitee_ref_query(branch);
            if !ref_query.is_empty() {
                suffix.push('?');
                suffix.push_str(&ref_query);
            }
            let json = gitee_get_json(&c, &gitee_api_url(&owner, &repo, &suffix)).await?;
            Ok(format_hosted_repo_tree(
                "Gitee",
                &format!("{owner}/{repo}"),
                &clean_path,
                "Gitee REST API /contents",
                &retrieved,
                &json,
                limit,
            ))
        }
        "file" => {
            let clean_path = path
                .map(|p| p.trim().trim_matches('/').to_string())
                .filter(|p| !p.is_empty())
                .ok_or_else(|| "gitee_repo action=file 需要 path，例如 src/main.ts".to_string())?;
            let mut suffix = format!(
                "contents/{}",
                url_query_component(&clean_path).replace("%2F", "/")
            );
            let ref_query = gitee_ref_query(branch);
            if !ref_query.is_empty() {
                suffix.push('?');
                suffix.push_str(&ref_query);
            }
            let json = gitee_get_json(&c, &gitee_api_url(&owner, &repo, &suffix)).await?;
            if json.as_array().is_some() {
                return Err("gitee_repo action=file 收到目录；请改用 action=tree".into());
            }
            let encoded = json.get("content").and_then(Value::as_str).unwrap_or("");
            let bytes = decode_repo_content_base64("Gitee", encoded)?;
            Ok(format_hosted_repo_file(
                "Gitee",
                &format!("{owner}/{repo}"),
                &clean_path,
                "Gitee REST API /contents",
                &retrieved,
                &repo_text_value(&json, &["encoding"]),
                &bytes,
                &repo_text_value(&json, &["sha"]),
                &repo_text_value(&json, &["html_url"]),
            ))
        }
        "releases" => {
            let json = gitee_get_json(
                &c,
                &gitee_api_url(&owner, &repo, &format!("releases?per_page={limit}")),
            )
            .await?;
            Ok(format_hosted_repo_releases(
                "Gitee",
                &format!("{owner}/{repo}"),
                "Gitee REST API /releases",
                &retrieved,
                &json,
            ))
        }
        "issues" | "pulls" => {
            let endpoint = if action == "pulls" { "pulls" } else { "issues" };
            let json = gitee_get_json(
                &c,
                &gitee_api_url(
                    &owner,
                    &repo,
                    &format!("{endpoint}?state=open&per_page={limit}"),
                ),
            )
            .await?;
            Ok(format_hosted_repo_items(
                "Gitee",
                &format!("{owner}/{repo}"),
                endpoint,
                &format!("Gitee REST API /{endpoint}"),
                &retrieved,
                &json,
            ))
        }
        other => Err(format!(
            "unknown gitee_repo action={other}; allowed: overview, readme, tree, file, releases, issues, pulls"
        )),
    }
}

fn codeberg_api_url(owner: &str, repo: &str, suffix: &str) -> String {
    let owner = url_query_component(owner);
    let repo = url_query_component(repo);
    if suffix.is_empty() {
        format!("https://codeberg.org/api/v1/repos/{owner}/{repo}")
    } else {
        format!("https://codeberg.org/api/v1/repos/{owner}/{repo}/{suffix}")
    }
}

async fn codeberg_get_json(c: &Client, url: &str) -> Result<Value, String> {
    api_get_json(c, url, "Codeberg", codeberg_auth_header).await
}

fn codeberg_ref_query(branch: &str) -> String {
    if branch.trim().is_empty() {
        String::new()
    } else {
        format!("ref={}", url_query_component(branch.trim()))
    }
}

async fn codeberg_read_content(
    c: &Client,
    owner: &str,
    repo: &str,
    path: &str,
    branch: &str,
) -> Result<Value, String> {
    let mut suffix = format!("contents/{}", url_query_component(path).replace("%2F", "/"));
    let ref_query = codeberg_ref_query(branch);
    if !ref_query.is_empty() {
        suffix.push('?');
        suffix.push_str(&ref_query);
    }
    codeberg_get_json(c, &codeberg_api_url(owner, repo, &suffix)).await
}

/// Read a real Codeberg/Gitea public or token-authorized repository.
#[tauri::command]
pub async fn codeberg_repo(
    owner: String,
    repo: String,
    action: Option<String>,
    path: Option<String>,
    branch: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let (owner, repo) = hosted_repo_ref("codeberg_repo", &owner, &repo, false)?;
    let action = repo_reader_action(action);
    let c = kclient()?;
    let retrieved = retrieved_at();
    let limit = max_results.unwrap_or(20).min(100);
    let branch = branch.unwrap_or_default();
    let branch = branch.trim();

    match action.as_str() {
        "overview" => {
            let json = codeberg_get_json(&c, &codeberg_api_url(&owner, &repo, "")).await?;
            Ok(format_codeberg_repo_overview(&json, &retrieved))
        }
        "readme" => {
            let overview = codeberg_get_json(&c, &codeberg_api_url(&owner, &repo, "")).await?;
            let default_branch = repo_text_value(&overview, &["default_branch"]);
            let branch = if branch.is_empty() {
                default_branch.as_str()
            } else {
                branch
            };
            let mut last_error = String::new();
            for candidate in ["README.md", "README.rst", "README.txt", "README", "readme.md"] {
                match codeberg_read_content(&c, &owner, &repo, candidate, branch).await {
                    Ok(json) => {
                        let encoded = json.get("content").and_then(Value::as_str).unwrap_or("");
                        let bytes = decode_repo_content_base64("Codeberg", encoded)?;
                        return Ok(format!(
                            "Codeberg repo README: {owner}/{repo}/{candidate}\nsearch_status: success\nsource: Codeberg/Gitea REST API /contents\nretrieved_at: {retrieved}\nencoding: {}\nsize_bytes: {}\nhtml_url: {}\n\n{}",
                            repo_text_value(&json, &["encoding"]),
                            bytes.len(),
                            repo_text_value(&json, &["html_url"]),
                            trunc(&String::from_utf8_lossy(&bytes), 60_000),
                        ));
                    }
                    Err(error) => last_error = error,
                }
            }
            Err(format!(
                "Codeberg README not found via common names on branch {branch}: {last_error}"
            ))
        }
        "tree" => {
            let clean_path = path.unwrap_or_default().trim().trim_matches('/').to_string();
            let mut suffix = if clean_path.is_empty() {
                "contents/".to_string()
            } else {
                format!(
                    "contents/{}",
                    url_query_component(&clean_path).replace("%2F", "/")
                )
            };
            let ref_query = codeberg_ref_query(branch);
            if !ref_query.is_empty() {
                suffix.push('?');
                suffix.push_str(&ref_query);
            }
            let json = codeberg_get_json(&c, &codeberg_api_url(&owner, &repo, &suffix)).await?;
            Ok(format_hosted_repo_tree(
                "Codeberg",
                &format!("{owner}/{repo}"),
                &clean_path,
                "Codeberg/Gitea REST API /contents",
                &retrieved,
                &json,
                limit,
            ))
        }
        "file" => {
            let clean_path = path
                .map(|p| p.trim().trim_matches('/').to_string())
                .filter(|p| !p.is_empty())
                .ok_or_else(|| "codeberg_repo action=file 需要 path，例如 src/main.ts".to_string())?;
            let json = codeberg_read_content(&c, &owner, &repo, &clean_path, branch).await?;
            if json.as_array().is_some() {
                return Err("codeberg_repo action=file 收到目录；请改用 action=tree".into());
            }
            let encoded = json.get("content").and_then(Value::as_str).unwrap_or("");
            let bytes = decode_repo_content_base64("Codeberg", encoded)?;
            Ok(format_hosted_repo_file(
                "Codeberg",
                &format!("{owner}/{repo}"),
                &clean_path,
                "Codeberg/Gitea REST API /contents",
                &retrieved,
                &repo_text_value(&json, &["encoding"]),
                &bytes,
                &repo_text_value(&json, &["sha"]),
                &repo_text_value(&json, &["html_url"]),
            ))
        }
        "releases" => {
            let json = codeberg_get_json(
                &c,
                &codeberg_api_url(&owner, &repo, &format!("releases?limit={limit}")),
            )
            .await?;
            Ok(format_hosted_repo_releases(
                "Codeberg",
                &format!("{owner}/{repo}"),
                "Codeberg/Gitea REST API /releases",
                &retrieved,
                &json,
            ))
        }
        "issues" | "pulls" => {
            let endpoint = if action == "pulls" { "pulls" } else { "issues" };
            let json = codeberg_get_json(
                &c,
                &codeberg_api_url(
                    &owner,
                    &repo,
                    &format!("{endpoint}?state=open&limit={limit}"),
                ),
            )
            .await?;
            Ok(format_hosted_repo_items(
                "Codeberg",
                &format!("{owner}/{repo}"),
                endpoint,
                &format!("Codeberg/Gitea REST API /{endpoint}"),
                &retrieved,
                &json,
            ))
        }
        other => Err(format!(
            "unknown codeberg_repo action={other}; allowed: overview, readme, tree, file, releases, issues, pulls"
        )),
    }
}

// ── CVE / NVD vulnerability database ───────────────────────────────

#[tauri::command]
pub async fn cve_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(20);

    let resp = c
        .get("https://services.nvd.nist.gov/rest/json/cves/2.0")
        .query(&[
            ("keywordSearch", query.as_str()),
            ("resultsPerPage", &limit.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("NVD: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("NVD returned {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| format!("NVD JSON: {e}"))?;
    let total = json["totalResults"].as_u64().unwrap_or(0);
    let mut out = format!("NVD CVE: {total} results\n\n");

    if let Some(vulns) = json["vulnerabilities"].as_array() {
        for (i, v) in vulns.iter().enumerate() {
            let cve = &v["cve"];
            let id = cve["id"].as_str().unwrap_or("?");
            let published = cve["published"]
                .as_str()
                .and_then(|s| s.get(..10))
                .unwrap_or("?");
            let desc = cve["descriptions"]
                .as_array()
                .and_then(|d| d.iter().find(|x| x["lang"].as_str() == Some("en")))
                .and_then(|x| x["value"].as_str())
                .unwrap_or("(no description)");

            let metrics = &cve["metrics"];
            let cvss_arr = metrics["cvssMetricV31"]
                .as_array()
                .or_else(|| metrics["cvssMetricV30"].as_array());
            let (score, severity) = cvss_arr
                .and_then(|m| m.first())
                .map(|m| {
                    let s = m["cvssData"]["baseScore"]
                        .as_f64()
                        .map(|v| format!("{v:.1}"))
                        .unwrap_or_else(|| "?".into());
                    let sev = m["cvssData"]["baseSeverity"]
                        .as_str()
                        .unwrap_or("?")
                        .to_string();
                    (s, sev)
                })
                .unwrap_or(("N/A".into(), "?".into()));

            out.push_str(&format!(
                "{}. {} (CVSS {score} {severity})\n   Published: {published}\n   {}\n   https://nvd.nist.gov/vuln/detail/{id}\n\n",
                i + 1,
                id,
                trunc(desc, 200),
            ));
        }
    }
    Ok(out)
}

// ── Wikipedia ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn wiki_search(
    query: String,
    lang: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(5).min(10);
    let l = lang.as_deref().unwrap_or("en");

    let base = format!("https://{l}.wikipedia.org/w/api.php");
    let resp = c
        .get(&base)
        .query(&[
            ("action", "query"),
            ("list", "search"),
            ("srsearch", &query),
            ("srlimit", &limit.to_string()),
            ("format", "json"),
            ("utf8", "1"),
        ])
        .send()
        .await
        .map_err(|e| format!("Wikipedia: {e}"))?;

    let json: Value = resp.json().await.map_err(|e| format!("Wiki JSON: {e}"))?;
    let total = json["query"]["searchinfo"]["totalhits"]
        .as_u64()
        .unwrap_or(0);
    let mut out = format!("Wikipedia ({l}): {total} results\n\n");

    if let Some(results) = json["query"]["search"].as_array() {
        for (i, r) in results.iter().enumerate() {
            let title = r["title"].as_str().unwrap_or("?");
            let snippet = strip_html(r["snippet"].as_str().unwrap_or(""));
            let slug = title.replace(' ', "_");
            out.push_str(&format!(
                "{}. {title}\n   {snippet}\n   https://{l}.wikipedia.org/wiki/{slug}\n\n",
                i + 1,
            ));
        }

        if let Some(first) = results.first() {
            let title = first["title"].as_str().unwrap_or("");
            if !title.is_empty() {
                if let Ok(extract) = fetch_wiki_extract(&c, &base, title).await {
                    out.push_str(&format!("--- Extract: {title} ---\n{extract}\n"));
                }
            }
        }
    }
    Ok(out)
}

async fn fetch_wiki_extract(c: &Client, base: &str, title: &str) -> Result<String, String> {
    let resp = c
        .get(base)
        .query(&[
            ("action", "query"),
            ("titles", title),
            ("prop", "extracts"),
            ("exintro", "1"),
            ("explaintext", "1"),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let pages = &json["query"]["pages"];
    if let Some(obj) = pages.as_object() {
        if let Some((_, page)) = obj.iter().next() {
            if let Some(ext) = page["extract"].as_str() {
                return Ok(trunc(ext, 800).to_string());
            }
        }
    }
    Err("no extract".into())
}

// strip_html moved to bottom of file (with entity decoding)

// ── Stack Overflow (Stack Exchange API) ────────────────────────────

#[tauri::command]
pub async fn stackoverflow_search(
    query: String,
    max_results: Option<u32>,
    tag: Option<String>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(8).min(20);

    let mut params: Vec<(&str, String)> = vec![
        ("q", query.clone()),
        ("site", "stackoverflow".into()),
        ("sort", "relevance".into()),
        ("order", "desc".into()),
        ("pagesize", limit.to_string()),
        ("filter", "!nNPvSNdWme".into()),
    ];
    if let Some(t) = &tag {
        params.push(("tagged", t.clone()));
    }

    let resp = c
        .get("https://api.stackexchange.com/2.3/search/advanced")
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("StackOverflow: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("StackOverflow returned {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| format!("SO JSON: {e}"))?;
    let retrieved = retrieved_at();
    let mut out = format!("Stack Overflow results:\nretrieved_at: {retrieved}\n\n");

    if let Some(items) = json["items"].as_array() {
        for (i, item) in items.iter().enumerate() {
            let title = strip_html(item["title"].as_str().unwrap_or("?"));
            let answered = item["is_answered"].as_bool().unwrap_or(false);
            let answers = item["answer_count"].as_u64().unwrap_or(0);
            let score = item["score"].as_i64().unwrap_or(0);
            let views = item["view_count"].as_u64().unwrap_or(0);
            let tags: Vec<&str> = item["tags"]
                .as_array()
                .map(|t| t.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let link = item["link"].as_str().unwrap_or("");
            let published = unix_time_rfc3339(item.get("creation_date"))
                .unwrap_or_else(|| "unknown".to_string());
            let updated = unix_time_rfc3339(item.get("last_edit_date"))
                .unwrap_or_else(|| "unknown".to_string());
            let last_activity = unix_time_rfc3339(item.get("last_activity_date"))
                .unwrap_or_else(|| "unknown".to_string());

            out.push_str(&format!(
                "{}. {} {}\n   Score: {} | Answers: {} | Views: {} | Tags: [{}]\n   published_date: {}\n   updated_date: {}\n   last_activity_date: {}\n   retrieved_at: {}\n   {}\n\n",
                i + 1,
                if answered { "✅" } else { "❓" },
                title,
                score,
                answers,
                views,
                tags.join(", "),
                published,
                updated,
                last_activity,
                retrieved,
                link,
            ));
        }
        if items.is_empty() {
            out.push_str("search_status: empty\n(no results)\n");
        }
    }
    let quota = json["quota_remaining"].as_u64().unwrap_or(0);
    if quota < 50 {
        out.push_str(&format!("⚠️ API quota remaining: {quota}/300\n"));
    }
    Ok(out)
}

// ── Hacker News (Algolia API) ─────────────────────────────────────

#[tauri::command]
pub async fn hackernews_search(
    query: String,
    max_results: Option<u32>,
    sort: Option<String>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(30);
    let endpoint = match sort.as_deref() {
        Some("date") | Some("new") => "search_by_date",
        _ => "search",
    };

    let resp = c
        .get(format!("https://hn.algolia.com/api/v1/{endpoint}"))
        .query(&[
            ("query", query.as_str()),
            ("tags", "story"),
            ("hitsPerPage", &limit.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("HN: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HN returned {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| format!("HN JSON: {e}"))?;
    let total = json["nbHits"].as_u64().unwrap_or(0);
    let retrieved = retrieved_at();
    let mut out = format!("Hacker News: {total} results\nretrieved_at: {retrieved}\n\n");
    if total == 0 {
        out.push_str("search_status: empty\n");
    }

    if let Some(hits) = json["hits"].as_array() {
        for (i, h) in hits.iter().enumerate() {
            let title = h["title"].as_str().unwrap_or("?");
            let points = h["points"].as_u64().unwrap_or(0);
            let comments = h["num_comments"].as_u64().unwrap_or(0);
            let author = h["author"].as_str().unwrap_or("?");
            let published = value_or_unknown(h["created_at"].as_str());
            let url = h["url"].as_str().unwrap_or("");
            let hn_id = h["objectID"].as_str().unwrap_or("");

            out.push_str(&format!(
                "{}. {} ({}pts, {}comments)\n   By: {}\n   published_date: {}\n   updated_date: unknown\n   last_activity_date: unknown\n   retrieved_at: {}\n   {}\n   HN: https://news.ycombinator.com/item?id={}\n\n",
                i + 1, title, points, comments, author, published, retrieved, url, hn_id,
            ));
        }
    }
    Ok(out)
}

// ── Official language-community Discourse forums ───────────────────

fn parse_discourse_search(
    payload: &Value,
    source: DiscourseSource,
    query: &str,
    limit: u32,
    retrieved: &str,
) -> Result<String, String> {
    if let Some(error) = payload
        .pointer("/grouped_search_result/error")
        .and_then(Value::as_str)
        .filter(|error| !error.trim().is_empty())
    {
        return Err(format!("{} search error: {error}", source.label));
    }

    let posts = payload
        .get("posts")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{}: response did not contain a posts array", source.label))?;
    let topics = payload
        .get("topics")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{}: response did not contain a topics array", source.label))?;
    let topics_by_id: HashMap<u64, &Value> = topics
        .iter()
        .filter_map(|topic| {
            topic
                .get("id")
                .and_then(Value::as_u64)
                .map(|id| (id, topic))
        })
        .collect();

    let mut out = format!(
        "{} official Discourse search for '{query}':\nsource: {}\nretrieved_at: {retrieved}\n\n",
        source.label, source.key,
    );
    if posts.is_empty() {
        out.push_str("search_status: empty\nNo matching public posts were returned.\n");
        return Ok(out);
    }

    for (index, post) in posts.iter().take(limit as usize).enumerate() {
        let topic_id = post.get("topic_id").and_then(Value::as_u64);
        let topic = topic_id.and_then(|id| topics_by_id.get(&id).copied());
        let title = topic
            .and_then(|topic| topic.get("title").and_then(Value::as_str))
            .or_else(|| post.get("topic_title").and_then(Value::as_str))
            .unwrap_or("(title unavailable)");
        let slug = topic
            .and_then(|topic| topic.get("slug").and_then(Value::as_str))
            .unwrap_or("");
        let post_number = post.get("post_number").and_then(Value::as_u64).unwrap_or(1);
        let url = match topic_id {
            Some(id) if !slug.is_empty() => {
                format!("{}/t/{slug}/{id}/{post_number}", source.base_url)
            }
            Some(id) => format!("{}/t/{id}/{post_number}", source.base_url),
            None => source.base_url.to_string(),
        };
        let author = post
            .get("username")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let blurb = strip_html(post.get("blurb").and_then(Value::as_str).unwrap_or(""));
        let published = value_or_unknown(post.get("created_at").and_then(Value::as_str));
        let updated = value_or_unknown(post.get("updated_at").and_then(Value::as_str));
        let last_activity = value_or_unknown(
            topic.and_then(|topic| topic.get("last_posted_at").and_then(Value::as_str)),
        );
        let replies = topic
            .and_then(|topic| topic.get("reply_count").and_then(Value::as_u64))
            .or_else(|| {
                topic
                    .and_then(|topic| topic.get("posts_count").and_then(Value::as_u64))
                    .map(|posts| posts.saturating_sub(1))
            })
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        out.push_str(&format!(
            "{}. {}\n   author: {} | replies: {}\n   {}\n   published_date: {}\n   updated_date: {}\n   last_activity_date: {}\n   retrieved_at: {}\n   {}\n\n",
            index + 1,
            strip_html(title),
            author,
            replies,
            trunc(&blurb, 240),
            published,
            updated,
            last_activity,
            retrieved,
            url,
        ));
    }
    Ok(out)
}

async fn discourse_search(
    query: String,
    max_results: Option<u32>,
    source: DiscourseSource,
) -> Result<String, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err(format!(
            "{} search requires a non-empty query",
            source.label
        ));
    }
    let client = kclient()?;
    let response = client
        .get(format!("{}/search.json", source.base_url))
        .query(&[("q", query)])
        .send()
        .await
        .map_err(|error| format!("{}: {error}", source.label))?;
    let status = response.status();
    if status.as_u16() == 429 {
        return Err(format!("{} rate limited (HTTP 429)", source.label));
    }
    if !status.is_success() {
        return Err(format!("{} returned HTTP {status}", source.label));
    }
    let payload: Value = response
        .json()
        .await
        .map_err(|error| format!("{} JSON: {error}", source.label))?;
    parse_discourse_search(
        &payload,
        source,
        query,
        max_results.unwrap_or(10).clamp(1, 20),
        &retrieved_at(),
    )
}

#[tauri::command]
pub async fn rust_users_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    discourse_search(query, max_results, RUST_USERS_DISCOURSE).await
}

#[tauri::command]
pub async fn python_discussions_search(
    query: String,
    max_results: Option<u32>,
) -> Result<String, String> {
    discourse_search(query, max_results, PYTHON_DISCOURSE).await
}

#[tauri::command]
pub async fn swift_forums_search(
    query: String,
    max_results: Option<u32>,
) -> Result<String, String> {
    discourse_search(query, max_results, SWIFT_DISCOURSE).await
}

#[tauri::command]
pub async fn kotlin_discussions_search(
    query: String,
    max_results: Option<u32>,
) -> Result<String, String> {
    discourse_search(query, max_results, KOTLIN_DISCOURSE).await
}

// ── Federated developer-community search ───────────────────────────

/// Search the supported developer communities concurrently. A failed source is
/// reported alongside successful sources instead of failing or overstating the
/// entire search. "all" means every adapter listed in
/// `DEVELOPER_COMMUNITY_SOURCES`, not every developer site on the internet.
#[tauri::command]
pub async fn developer_community_search(
    query: String,
    scope: Option<String>,
    sources: Option<Vec<String>>,
    max_per_source: Option<u32>,
) -> Result<String, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("developer_community_search requires a non-empty query".into());
    }

    let selected = select_developer_sources(scope.as_deref(), sources.as_deref())?;
    let limit = max_per_source.unwrap_or(3).clamp(1, 5);
    let mut pending: FuturesUnordered<CommunitySearchFuture> = FuturesUnordered::new();

    for source in &selected {
        let source_key = *source;
        let source_label = DEVELOPER_COMMUNITY_SOURCES
            .iter()
            .find_map(|(key, label)| (*key == source_key).then_some(*label))
            .expect("selected developer source must have a label");
        let q = query.clone();
        let adapter: CommunityAdapterFuture = match source_key {
            "github" => Box::pin(async move {
                (
                    "github",
                    "GitHub",
                    github_search(q, Some("repositories".into()), Some(limit)).await,
                )
            }),
            "github_discussions" => Box::pin(async move {
                (
                    "github_discussions",
                    "GitHub Discussions",
                    github_discussions_search(q, Some(limit)).await,
                )
            }),
            "stackoverflow" => Box::pin(async move {
                (
                    "stackoverflow",
                    "Stack Overflow",
                    stackoverflow_search(q, Some(limit), None).await,
                )
            }),
            "hackernews" => Box::pin(async move {
                (
                    "hackernews",
                    "Hacker News",
                    hackernews_search(q, Some(limit), None).await,
                )
            }),
            "reddit" => Box::pin(async move {
                (
                    "reddit",
                    "Reddit",
                    reddit_search(q, None, Some(limit)).await,
                )
            }),
            "lobsters" => Box::pin(async move {
                (
                    "lobsters",
                    "Lobsters",
                    lobsters_search(q, Some(limit)).await,
                )
            }),
            "devto" => {
                Box::pin(
                    async move { ("devto", "DEV Community", devto_search(q, Some(limit)).await) },
                )
            }
            "juejin" => {
                Box::pin(async move { ("juejin", "掘金", juejin_search(q, Some(limit)).await) })
            }
            "v2ex" => Box::pin(async move { ("v2ex", "V2EX", v2ex_search(q, Some(limit)).await) }),
            "segmentfault" => Box::pin(async move {
                (
                    "segmentfault",
                    "SegmentFault",
                    segmentfault_search(q, Some(limit)).await,
                )
            }),
            "rust_users" => Box::pin(async move {
                (
                    "rust_users",
                    "Rust Users Forum",
                    rust_users_search(q, Some(limit)).await,
                )
            }),
            "python_discussions" => Box::pin(async move {
                (
                    "python_discussions",
                    "Python Discussions",
                    python_discussions_search(q, Some(limit)).await,
                )
            }),
            "swift_forums" => Box::pin(async move {
                (
                    "swift_forums",
                    "Swift Forums",
                    swift_forums_search(q, Some(limit)).await,
                )
            }),
            "kotlin_discussions" => Box::pin(async move {
                (
                    "kotlin_discussions",
                    "Kotlin Discussions",
                    kotlin_discussions_search(q, Some(limit)).await,
                )
            }),
            "gitlab" => {
                Box::pin(async move { ("gitlab", "GitLab", gitlab_search(q, Some(limit)).await) })
            }
            "gitee" => {
                Box::pin(async move { ("gitee", "Gitee", gitee_search(q, Some(limit)).await) })
            }
            "codeberg" => Box::pin(async move {
                (
                    "codeberg",
                    "Codeberg",
                    codeberg_search(q, Some(limit)).await,
                )
            }),
            "sourcegraph" => Box::pin(async move {
                (
                    "sourcegraph",
                    "Sourcegraph",
                    sourcegraph_search(q, Some(limit)).await,
                )
            }),
            "bestofjs" => Box::pin(async move {
                (
                    "bestofjs",
                    "Best of JS",
                    bestofjs_search(q, Some(limit)).await,
                )
            }),
            "github_trending" => Box::pin(async move {
                let result = match aggregate_trending_language(&q) {
                    Some(language) => github_trending(language.to_string(), Some(limit)).await,
                    None => Ok(
                        "search_status: empty\nGitHub Trending is language discovery, not keyword search. The aggregate skipped it because this query is not a recognized single language; no invalid trending URL was requested."
                            .to_string(),
                    ),
                };
                ("github_trending", "GitHub Trending", result)
            }),
            "producthunt" => Box::pin(async move {
                (
                    "producthunt",
                    "Product Hunt",
                    producthunt_search(q, Some(limit)).await,
                )
            }),
            "freecodecamp" => Box::pin(async move {
                (
                    "freecodecamp",
                    "freeCodeCamp",
                    freecodecamp_search(q, Some(limit)).await,
                )
            }),
            "infoq" => {
                Box::pin(async move { ("infoq", "InfoQ", infoq_search(q, Some(limit)).await) })
            }
            "hackernoon" => Box::pin(async move {
                (
                    "hackernoon",
                    "HackerNoon",
                    hackernoon_search(q, Some(limit)).await,
                )
            }),
            _ => continue,
        };
        pending.push(Box::pin(community_source_with_timeout(
            source_key,
            source_label,
            adapter,
            COMMUNITY_SOURCE_TIMEOUT,
        )));
    }

    let mut responses = Vec::with_capacity(selected.len());
    while let Some(response) = pending.next().await {
        responses.push((response.0, response.1, response.2, retrieved_at()));
    }
    Ok(format_developer_community_results(
        &query,
        &selected,
        responses,
        &retrieved_at(),
    ))
}

fn format_developer_community_results(
    query: &str,
    selected: &[&'static str],
    mut responses: Vec<CommunitySearchResponse>,
    aggregate_retrieved_at: &str,
) -> String {
    responses.sort_by_key(|(key, _, _, _)| {
        selected
            .iter()
            .position(|selected_key| selected_key == key)
            .unwrap_or(usize::MAX)
    });

    let success = responses
        .iter()
        .filter(|(_, _, result, _)| {
            community_result_status(result) == CommunitySourceStatus::Success
        })
        .count();
    let empty = responses
        .iter()
        .filter(|(_, _, result, _)| community_result_status(result) == CommunitySourceStatus::Empty)
        .count();
    let rate_limited = responses
        .iter()
        .filter(|(_, _, result, _)| {
            community_result_status(result) == CommunitySourceStatus::RateLimited
        })
        .count();
    let failed = responses
        .iter()
        .filter(|(_, _, result, _)| {
            community_result_status(result) == CommunitySourceStatus::Failed
        })
        .count();
    let timed_out = responses
        .iter()
        .filter(|(_, _, result, _)| {
            community_result_status(result) == CommunitySourceStatus::Timeout
        })
        .count();
    let completed = success + empty;
    let failed_requests = failed + rate_limited + timed_out;
    let mut out = format!(
        "Developer community search\nQuery: {query}\nretrieved_at: {aggregate_retrieved_at}\nRequested sources: {}; completed searches: {completed}; failed requests: {failed_requests}.\n\
         Status counts: success={success}; empty={empty}; rate-limited={rate_limited}; failed={failed}; timeout={timed_out}.\n\
         The five source statuses are success, empty, rate-limited, failed, and timeout; timeout is reported separately and counts as a failed request. Sources run concurrently with an independent {} ms hard deadline, so source collection is bounded to about 12 seconds plus scheduling and formatting overhead.\n\
         Per-source retrieved_at is when that adapter finished, failed, or reached the local hard deadline. It is not published_date, created_date, updated_date, or last_activity_date. Missing provider dates remain unknown.\n\
         This request executed now, but source content may come from provider indexes or caches and is not necessarily current. A successful search may use a source API or a clearly labelled public site search. Community posts are not verified facts; inspect original pages and cross-check important claims.\n",
        responses.len(),
        COMMUNITY_SOURCE_TIMEOUT.as_millis(),
    );

    for (_, label, result, source_retrieved_at) in responses {
        let status = community_result_status(&result);
        let legacy_status = match status {
            CommunitySourceStatus::Success | CommunitySourceStatus::Empty => "search completed",
            CommunitySourceStatus::RateLimited
            | CommunitySourceStatus::Failed
            | CommunitySourceStatus::Timeout => "failed",
        };
        out.push_str(&format!(
            "\n## {label} [{legacy_status}; status={}]\nretrieved_at: {source_retrieved_at}\n",
            status.as_str(),
        ));
        match result {
            CommunitySearchOutcome::Finished(Ok(content)) => {
                out.push_str(&format!("{}\n", trunc(&content, 1200)))
            }
            CommunitySearchOutcome::Finished(Err(error)) => {
                out.push_str(&format!("{}\n", trunc(&error, 500)))
            }
            CommunitySearchOutcome::TimedOut { after } => out.push_str(&format!(
                "search_status: timeout\ntimeout_ms: {}\nThe source did not finish within its per-source hard deadline; pending work was cancelled.\n",
                after.as_millis(),
            )),
        }
    }
    out
}

// ── PubMed (NCBI E-utilities) ─────────────────────────────────────

#[tauri::command]
pub async fn pubmed_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(8).min(20);

    let search_resp = c
        .get("https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi")
        .query(&[
            ("db", "pubmed"),
            ("term", &query),
            ("retmax", &limit.to_string()),
            ("retmode", "json"),
            ("sort", "relevance"),
        ])
        .send()
        .await
        .map_err(|e| format!("PubMed search: {e}"))?;

    let sj: Value = search_resp
        .json()
        .await
        .map_err(|e| format!("PubMed JSON: {e}"))?;
    let ids: Vec<&str> = sj["esearchresult"]["idlist"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    if ids.is_empty() {
        return Ok("PubMed: no results found.\n".into());
    }

    let id_str = ids.join(",");
    let total = sj["esearchresult"]["count"].as_str().unwrap_or("?");
    let sum_resp = c
        .get("https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi")
        .query(&[("db", "pubmed"), ("id", &id_str), ("retmode", "json")])
        .send()
        .await
        .map_err(|e| format!("PubMed summary: {e}"))?;

    let dj: Value = sum_resp
        .json()
        .await
        .map_err(|e| format!("PubMed JSON: {e}"))?;
    let mut out = format!("PubMed: {total} results (showing {}):\n\n", ids.len());

    if let Some(result) = dj["result"].as_object() {
        for (i, pmid) in ids.iter().enumerate() {
            let p = &result[*pmid];
            let title = strip_html(p["title"].as_str().unwrap_or("?"));
            let source = p["source"].as_str().unwrap_or("?");
            let pubdate = p["pubdate"].as_str().unwrap_or("?");
            let authors: Vec<&str> = p["authors"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect())
                .unwrap_or_default();
            let auth_str = if authors.len() > 3 {
                format!("{}, {} et al.", authors[0], authors[1])
            } else {
                authors.join(", ")
            };

            out.push_str(&format!(
                "{}. {}\n   Authors: {}\n   Journal: {} | Date: {}\n   PMID: {} | https://pubmed.ncbi.nlm.nih.gov/{}/\n\n",
                i + 1, title, auth_str, source, pubdate, pmid, pmid,
            ));
        }
    }
    Ok(out)
}

// ── arXiv ─────────────────────────────────────────────────────────

fn xml_tag_value<'a>(xml: &'a str, tag: &str) -> &'a str {
    let open = format!("<{tag}");
    if let Some(start) = xml.find(&open) {
        let after = &xml[start + open.len()..];
        if let Some(gt) = after.find('>') {
            let content = &after[gt + 1..];
            let close = format!("</{tag}>");
            if let Some(end) = content.find(&close) {
                return content[..end].trim();
            }
        }
    }
    ""
}

fn xml_tag_attr<'a>(xml: &'a str, tag: &str, attr: &str) -> &'a str {
    let open = format!("<{tag} ");
    if let Some(start) = xml.find(&open) {
        let after = &xml[start..];
        let needle = format!("{attr}=\"");
        if let Some(a) = after.find(&needle) {
            let val_start = &after[a + needle.len()..];
            if let Some(end) = val_start.find('"') {
                return &val_start[..end];
            }
        }
    }
    ""
}

#[tauri::command]
pub async fn arxiv_search(
    query: String,
    category: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(30);

    let search_q = if let Some(cat) = &category {
        format!("all:{query} AND cat:{cat}")
    } else {
        format!("all:{query}")
    };

    let resp = c
        .get("http://export.arxiv.org/api/query")
        .query(&[
            ("search_query", search_q.as_str()),
            ("start", "0"),
            ("max_results", &limit.to_string()),
            ("sortBy", "relevance"),
            ("sortOrder", "descending"),
        ])
        .send()
        .await
        .map_err(|e| format!("arXiv: {e}"))?;

    let xml = resp.text().await.map_err(|e| format!("arXiv text: {e}"))?;
    let total_str = xml_tag_value(&xml, "opensearch:totalResults");
    let mut out = format!("arXiv: {total_str} results\n\n");

    for (i, entry) in xml.split("<entry>").skip(1).enumerate() {
        let title = xml_tag_value(entry, "title").replace('\n', " ");
        let summary = xml_tag_value(entry, "summary").replace('\n', " ");
        let published = xml_tag_value(entry, "published").get(..10).unwrap_or("?");
        let id_url = xml_tag_value(entry, "id");
        let category = xml_tag_attr(entry, "arxiv:primary_category", "term");

        let mut authors = Vec::new();
        for author_block in entry.split("<author>").skip(1) {
            let name = xml_tag_value(author_block, "name");
            if !name.is_empty() {
                authors.push(name);
            }
        }
        let auth_str = if authors.len() > 3 {
            format!("{}, {} et al.", authors[0], authors[1])
        } else {
            authors.join(", ")
        };

        let pdf = id_url.replace("/abs/", "/pdf/");
        out.push_str(&format!(
            "{}. {}\n   Authors: {}\n   Category: {} | Published: {}\n   {}\n   PDF: {}\n   {}\n\n",
            i + 1,
            title.trim(),
            auth_str,
            category,
            published,
            id_url,
            pdf,
            trunc(summary.trim(), 300),
        ));
    }
    Ok(out)
}

// ── CrossRef (DOI / citation metadata) ───────────────────────────

#[tauri::command]
pub async fn crossref_search(
    query: String,
    search_type: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(20);
    let stype = search_type.as_deref().unwrap_or("works");

    let url = format!("https://api.crossref.org/{stype}");
    let resp = c
        .get(&url)
        .query(&[
            ("query", query.as_str()),
            ("rows", &limit.to_string()),
            ("sort", "relevance"),
        ])
        .header(
            "User-Agent",
            "Michael-IDE/1.0 (mailto:contact@michaelide.xyz)",
        )
        .send()
        .await
        .map_err(|e| format!("CrossRef: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("CrossRef returned {}", resp.status()));
    }

    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("CrossRef JSON: {e}"))?;
    let total = json["message"]["total-results"].as_u64().unwrap_or(0);
    let mut out = format!("CrossRef: {total} results\n\n");

    if let Some(items) = json["message"]["items"].as_array() {
        for (i, item) in items.iter().enumerate() {
            let title = item["title"]
                .as_array()
                .and_then(|t| t.first())
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let doi = item["DOI"].as_str().unwrap_or("?");
            let pub_type = item["type"].as_str().unwrap_or("?");
            let cited = item["is-referenced-by-count"].as_u64().unwrap_or(0);
            let container = item["container-title"]
                .as_array()
                .and_then(|t| t.first())
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let year = item["published"]["date-parts"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_u64())
                .map(|y| y.to_string())
                .unwrap_or_default();

            let authors: Vec<String> = item["author"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|x| {
                            format!(
                                "{} {}",
                                x["given"].as_str().unwrap_or(""),
                                x["family"].as_str().unwrap_or("")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let auth_str = if authors.len() > 3 {
                format!("{}, {} et al.", authors[0], authors[1])
            } else {
                authors.join(", ")
            };

            out.push_str(&format!(
                "{}. {}\n   Authors: {}\n   {} ({}) | Cited: {} | Type: {}\n   DOI: {} | https://doi.org/{}\n\n",
                i + 1, title, auth_str, container, year, cited, pub_type, doi, doi,
            ));
        }
    }
    Ok(out)
}

// ── OpenAlex (open scholarly graph, 250M+ works) ─────────────────

#[tauri::command]
pub async fn openalex_search(
    query: String,
    entity_type: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(25);
    let etype = entity_type.as_deref().unwrap_or("works");

    let url = format!("https://api.openalex.org/{etype}");
    let resp = c
        .get(&url)
        .query(&[
            ("search", query.as_str()),
            ("per_page", &limit.to_string()),
            ("mailto", "contact@michaelide.xyz"),
        ])
        .send()
        .await
        .map_err(|e| format!("OpenAlex: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("OpenAlex returned {}", resp.status()));
    }

    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("OpenAlex JSON: {e}"))?;
    let total = json["meta"]["count"].as_u64().unwrap_or(0);
    let mut out = format!("OpenAlex ({etype}): {total} results\n\n");

    if let Some(results) = json["results"].as_array() {
        match etype {
            "works" => {
                for (i, w) in results.iter().enumerate() {
                    let title = w["title"].as_str().unwrap_or("?");
                    let year = w["publication_year"]
                        .as_u64()
                        .map(|y| y.to_string())
                        .unwrap_or_default();
                    let cited = w["cited_by_count"].as_u64().unwrap_or(0);
                    let oa = w["open_access"]["is_oa"].as_bool().unwrap_or(false);
                    let doi = w["doi"].as_str().unwrap_or("");
                    let venue = w["primary_location"]["source"]["display_name"]
                        .as_str()
                        .unwrap_or("");
                    let w_type = w["type"].as_str().unwrap_or("?");

                    let authors: Vec<&str> = w["authorships"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x["author"]["display_name"].as_str())
                                .collect()
                        })
                        .unwrap_or_default();
                    let auth_str = if authors.len() > 3 {
                        format!("{}, {} et al.", authors[0], authors[1])
                    } else {
                        authors.join(", ")
                    };

                    let topics: Vec<&str> = w["topics"]
                        .as_array()
                        .map(|t| {
                            t.iter()
                                .take(3)
                                .filter_map(|x| x["display_name"].as_str())
                                .collect()
                        })
                        .unwrap_or_default();

                    out.push_str(&format!(
                        "{}. {} ({})\n   Authors: {}\n   Venue: {} | Cited: {} | OA: {} | Type: {}\n   Topics: [{}]\n   {}\n\n",
                        i + 1, title, year, auth_str, venue, cited,
                        if oa { "Yes" } else { "No" }, w_type,
                        topics.join(", "), doi,
                    ));
                }
            }
            "authors" => {
                for (i, a) in results.iter().enumerate() {
                    let name = a["display_name"].as_str().unwrap_or("?");
                    let works = a["works_count"].as_u64().unwrap_or(0);
                    let cited = a["cited_by_count"].as_u64().unwrap_or(0);
                    let inst = a["last_known_institutions"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|x| x["display_name"].as_str())
                        .unwrap_or("?");
                    out.push_str(&format!(
                        "{}. {}\n   Institution: {} | Works: {} | Cited: {}\n\n",
                        i + 1,
                        name,
                        inst,
                        works,
                        cited,
                    ));
                }
            }
            _ => {
                for (i, r) in results.iter().enumerate() {
                    let name = r["display_name"].as_str().unwrap_or("?");
                    let works = r["works_count"].as_u64().unwrap_or(0);
                    out.push_str(&format!("{}. {} (works: {})\n\n", i + 1, name, works));
                }
            }
        }
    }
    Ok(out)
}

// ── PubChem (chemical compounds) ─────────────────────────────────

#[tauri::command]
pub async fn pubchem_search(query: String, search_type: Option<String>) -> Result<String, String> {
    let c = kclient()?;
    let stype = search_type.as_deref().unwrap_or("compound");

    match stype {
        "compound" => {
            let auto_resp = c
                .get(format!(
                    "https://pubchem.ncbi.nlm.nih.gov/rest/autocomplete/{stype}/{query}/json"
                ))
                .query(&[("limit", "8")])
                .send()
                .await
                .map_err(|e| format!("PubChem autocomplete: {e}"))?;

            let auto_json: Value = auto_resp
                .json()
                .await
                .map_err(|e| format!("PubChem JSON: {e}"))?;
            let names: Vec<&str> = auto_json["dictionary_terms"]["compound"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            if names.is_empty() {
                return Ok(format!("PubChem: no compounds found for '{query}'.\n"));
            }

            let first = names[0];
            let prop_resp = c
                .get(format!(
                    "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/name/{first}/property/MolecularFormula,MolecularWeight,IUPACName,InChIKey,XLogP,ExactMass,Charge/JSON"
                ))
                .send()
                .await
                .map_err(|e| format!("PubChem props: {e}"))?;

            let mut out = format!("PubChem matches: {}\n\n", names.join(", "));

            if prop_resp.status().is_success() {
                let pj: Value = prop_resp
                    .json()
                    .await
                    .map_err(|e| format!("PubChem JSON: {e}"))?;
                if let Some(props) = pj["PropertyTable"]["Properties"].as_array() {
                    if let Some(p) = props.first() {
                        let cid = p["CID"].as_u64().unwrap_or(0);
                        out.push_str(&format!(
                            "Compound: {}\n  CID: {}\n  IUPAC: {}\n  Formula: {}\n  MW: {} g/mol\n  Exact Mass: {}\n  XLogP: {}\n  InChIKey: {}\n  Charge: {}\n  https://pubchem.ncbi.nlm.nih.gov/compound/{}\n",
                            first,
                            cid,
                            p["IUPACName"].as_str().unwrap_or("?"),
                            p["MolecularFormula"].as_str().unwrap_or("?"),
                            p["MolecularWeight"].as_f64().map(|v| format!("{v:.2}")).unwrap_or("?".into()),
                            p["ExactMass"].as_f64().map(|v| format!("{v:.4}")).unwrap_or("?".into()),
                            p["XLogP"].as_f64().map(|v| format!("{v:.1}")).unwrap_or("N/A".into()),
                            p["InChIKey"].as_str().unwrap_or("?"),
                            p["Charge"].as_i64().unwrap_or(0),
                            cid,
                        ));
                    }
                }
            }
            Ok(out)
        }
        _ => Err(format!(
            "Unknown PubChem search_type '{stype}'. Use: compound"
        )),
    }
}

// ── ClinicalTrials.gov (clinical studies) ─────────────────────────

#[tauri::command]
pub async fn clinical_trials_search(
    query: String,
    status: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(20);

    let mut params: Vec<(&str, String)> = vec![
        ("query.term", query.clone()),
        ("pageSize", limit.to_string()),
        ("format", "json".into()),
        ("sort", "LastUpdatePostDate:desc".into()),
    ];
    if let Some(s) = &status {
        params.push(("filter.overallStatus", s.clone()));
    }

    let resp = c
        .get("https://clinicaltrials.gov/api/v2/studies")
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("ClinicalTrials: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("ClinicalTrials returned {}", resp.status()));
    }

    let json: Value = resp.json().await.map_err(|e| format!("CT JSON: {e}"))?;
    let total = json["totalCount"].as_u64().unwrap_or(0);
    let mut out = format!("ClinicalTrials.gov: {total} studies\n\n");

    if let Some(studies) = json["studies"].as_array() {
        for (i, s) in studies.iter().enumerate() {
            let proto = &s["protocolSection"];
            let id_mod = &proto["identificationModule"];
            let status_mod = &proto["statusModule"];
            let design_mod = &proto["designModule"];

            let nct_id = id_mod["nctId"].as_str().unwrap_or("?");
            let title = id_mod["briefTitle"].as_str().unwrap_or("?");
            let overall_status = status_mod["overallStatus"].as_str().unwrap_or("?");
            let start_date = status_mod["startDateStruct"]["date"]
                .as_str()
                .unwrap_or("?");
            let study_type = design_mod["studyType"].as_str().unwrap_or("?");
            let phases: Vec<&str> = design_mod["phases"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            let conditions: Vec<&str> = proto["conditionsModule"]["conditions"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let interventions: Vec<String> = proto["armsInterventionsModule"]["interventions"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .take(3)
                        .map(|x| {
                            format!(
                                "{}: {}",
                                x["type"].as_str().unwrap_or("?"),
                                x["name"].as_str().unwrap_or("?")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();

            out.push_str(&format!(
                "{}. [{}] {} ({})\n   {}\n   Conditions: {}\n   Interventions: {}\n   Phase: {} | Start: {}\n   https://clinicaltrials.gov/study/{}\n\n",
                i + 1,
                overall_status,
                nct_id,
                study_type,
                trunc(title, 200),
                conditions.join(", "),
                interventions.join("; "),
                if phases.is_empty() { "N/A" } else { phases[0] },
                start_date,
                nct_id,
            ));
        }
    }
    Ok(out)
}

// ── Docker Hub ────────────────────────────────────────────────────

#[tauri::command]
pub async fn dockerhub_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(25);

    let resp = c
        .get("https://hub.docker.com/v2/search/repositories/")
        .query(&[("query", query.as_str()), ("page_size", &limit.to_string())])
        .send()
        .await
        .map_err(|e| format!("Docker Hub: {e}"))?;

    let json: Value = resp.json().await.map_err(|e| format!("DH JSON: {e}"))?;
    let total = json["count"].as_u64().unwrap_or(0);
    let mut out = format!("Docker Hub: {total} images\n\n");

    if let Some(results) = json["results"].as_array() {
        for (i, r) in results.iter().enumerate() {
            let name = r["repo_name"].as_str().unwrap_or("?");
            let desc = r["short_description"].as_str().unwrap_or("");
            let stars = r["star_count"].as_u64().unwrap_or(0);
            let pulls = r["pull_count"].as_u64().unwrap_or(0);
            let official = r["is_official"].as_bool().unwrap_or(false);

            out.push_str(&format!(
                "{}. {}{}\n   {}\n   Stars: {} | Pulls: {}\n   https://hub.docker.com/{}{}\n\n",
                i + 1,
                name,
                if official { " [OFFICIAL]" } else { "" },
                trunc(desc, 150),
                stars,
                pulls,
                if name.contains('/') { "r/" } else { "_/" },
                name,
            ));
        }
    }
    Ok(out)
}

// ── GitLab ────────────────────────────────────────────────────────

#[tauri::command]
pub async fn gitlab_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://gitlab.com/api/v4/projects")
        .query(&[
            ("search", &query),
            ("per_page", &n.to_string()),
            ("order_by", &"star_count".to_string()),
            ("sort", &"desc".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("GitLab: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 429 {
        return Err("GitLab rate limited (HTTP 429)".into());
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "GitLab returned HTTP {status}: {}",
            trunc(body.trim(), 240)
        ));
    }
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("GitLab parse: {e}"))?;
    let arr = data.as_array().ok_or("GitLab: unexpected response")?;
    if arr.is_empty() {
        return Ok(format!("No GitLab projects found for '{query}'"));
    }
    let retrieved = retrieved_at();
    let mut out = format!("GitLab projects for '{query}':\nretrieved_at: {retrieved}\n\n");
    for (i, r) in arr.iter().enumerate() {
        let name = r["path_with_namespace"].as_str().unwrap_or("?");
        let desc = r["description"].as_str().unwrap_or("");
        let stars = r["star_count"].as_u64().unwrap_or(0);
        let forks = r["forks_count"].as_u64().unwrap_or(0);
        let lang = r["topics"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let url = r["web_url"].as_str().unwrap_or("");
        let created = value_or_unknown(r["created_at"].as_str());
        let updated = value_or_unknown(r["updated_at"].as_str());
        let last_activity = value_or_unknown(r["last_activity_at"].as_str());
        out.push_str(&format!(
            "{}. {}\n   {}\n   Stars: {} | Forks: {} | Topics: {}\n   published_date: unknown (GitLab project search does not expose a publication date)\n   created_date: {}\n   updated_date: {}\n   last_activity_date: {}\n   retrieved_at: {}\n   {}\n\n",
            i + 1,
            name,
            trunc(desc, 150),
            stars,
            forks,
            if lang.is_empty() { "-" } else { &lang },
            created,
            updated,
            last_activity,
            retrieved,
            url
        ));
    }
    Ok(out)
}

// ── Gitee ─────────────────────────────────────────────────────────

fn format_gitee_repository_item(item: &Value, index: usize, retrieved: &str) -> String {
    let name = item["full_name"]
        .as_str()
        .unwrap_or(item["name"].as_str().unwrap_or("?"));
    let dates = repository_date_lines("Gitee", item, Some("pushed_at"), retrieved);
    format!(
        "{}. {}\n   {}\n   Stars: {} | Forks: {} | Lang: {}\n{}   {}\n\n",
        index + 1,
        name,
        trunc(item["description"].as_str().unwrap_or(""), 150),
        item["stargazers_count"].as_u64().unwrap_or(0),
        item["forks_count"].as_u64().unwrap_or(0),
        item["language"].as_str().unwrap_or("-"),
        dates,
        item["html_url"].as_str().unwrap_or(""),
    )
}

#[tauri::command]
pub async fn gitee_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://gitee.com/api/v5/search/repositories")
        .query(&[
            ("q", &query),
            ("per_page", &n.to_string()),
            ("sort", &"stars_count".to_string()),
            ("order", &"desc".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Gitee: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 429 {
        return Err("Gitee rate limited (HTTP 429)".into());
    }
    if !status.is_success() {
        return Err(format!("Gitee returned HTTP {status}"));
    }
    let data: Value = resp.json().await.map_err(|e| format!("Gitee parse: {e}"))?;
    let arr = data.as_array().ok_or("Gitee: unexpected response")?;
    let retrieved = retrieved_at();
    if arr.is_empty() {
        return Ok(format!(
            "Gitee repos for '{query}':\nretrieved_at: {retrieved}\nsearch_status: empty\nNo matching repositories were returned."
        ));
    }
    let mut out = format!("Gitee repos for '{query}':\nretrieved_at: {retrieved}\n\n");
    for (i, r) in arr.iter().enumerate() {
        out.push_str(&format_gitee_repository_item(r, i, &retrieved));
    }
    Ok(out)
}

// ── Maven Central ─────────────────────────────────────────────────

#[tauri::command]
pub async fn maven_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://search.maven.org/solrsearch/select")
        .query(&[
            ("q", &query),
            ("rows", &n.to_string()),
            ("wt", &"json".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Maven: {e}"))?;
    let data: Value = resp.json().await.map_err(|e| format!("Maven parse: {e}"))?;
    let docs = data["response"]["docs"]
        .as_array()
        .ok_or("Maven: no docs")?;
    if docs.is_empty() {
        return Ok(format!("No Maven packages found for '{query}'"));
    }
    let mut out = format!("Maven Central results for '{query}':\n\n");
    for (i, d) in docs.iter().enumerate() {
        let group = d["g"].as_str().unwrap_or("?");
        let artifact = d["a"].as_str().unwrap_or("?");
        let version = d["latestVersion"]
            .as_str()
            .unwrap_or(d["v"].as_str().unwrap_or("?"));
        let packaging = d["p"].as_str().unwrap_or("jar");
        let timestamp = d["timestamp"].as_u64().unwrap_or(0);
        let date = if timestamp > 0 {
            let secs = timestamp / 1000;
            let days = secs / 86400;
            let y = 1970 + days / 365;
            format!("~{y}")
        } else {
            "-".to_string()
        };
        out.push_str(&format!(
            "{}. {}:{} (v{})\n   Packaging: {} | Updated: {}\n   https://search.maven.org/artifact/{}/{}/{}/jar\n\n",
            i + 1, group, artifact, version, packaging, date, group, artifact, version
        ));
    }
    Ok(out)
}

// ── Packagist (PHP) ───────────────────────────────────────────────

#[tauri::command]
pub async fn packagist_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://packagist.org/search.json")
        .query(&[("q", &query), ("per_page", &n.to_string())])
        .send()
        .await
        .map_err(|e| format!("Packagist: {e}"))?;
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Packagist parse: {e}"))?;
    let results = data["results"].as_array().ok_or("Packagist: no results")?;
    if results.is_empty() {
        return Ok(format!("No Packagist packages found for '{query}'"));
    }
    let mut out = format!("Packagist (PHP) results for '{query}':\n\n");
    for (i, r) in results.iter().take(n as usize).enumerate() {
        let name = r["name"].as_str().unwrap_or("?");
        let desc = r["description"].as_str().unwrap_or("");
        let url = r["url"].as_str().unwrap_or("");
        let downloads = r["downloads"].as_u64().unwrap_or(0);
        let favers = r["favers"].as_u64().unwrap_or(0);
        out.push_str(&format!(
            "{}. {}\n   {}\n   Favorites: {} | Downloads: {}\n   {}\n\n",
            i + 1,
            name,
            trunc(desc, 150),
            favers,
            downloads,
            url
        ));
    }
    Ok(out)
}

// ── RubyGems ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn rubygems_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20) as usize;
    let resp = client
        .get("https://rubygems.org/api/v1/search.json")
        .query(&[("query", &query)])
        .send()
        .await
        .map_err(|e| format!("RubyGems: {e}"))?;
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("RubyGems parse: {e}"))?;
    let arr = data.as_array().ok_or("RubyGems: unexpected response")?;
    if arr.is_empty() {
        return Ok(format!("No RubyGems found for '{query}'"));
    }
    let mut out = format!("RubyGems results for '{query}':\n\n");
    for (i, r) in arr.iter().take(n).enumerate() {
        let name = r["name"].as_str().unwrap_or("?");
        let version = r["version"].as_str().unwrap_or("?");
        let info = r["info"].as_str().unwrap_or("");
        let downloads = r["downloads"].as_u64().unwrap_or(0);
        let url = r["project_uri"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. {} (v{})\n   {}\n   Downloads: {}\n   {}\n\n",
            i + 1,
            name,
            version,
            trunc(info, 150),
            downloads,
            url
        ));
    }
    Ok(out)
}

// ── NuGet (.NET) ──────────────────────────────────────────────────

#[tauri::command]
pub async fn nuget_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://azuresearch-usnc.nuget.org/query")
        .query(&[("q", &query), ("take", &n.to_string())])
        .send()
        .await
        .map_err(|e| format!("NuGet: {e}"))?;
    let data: Value = resp.json().await.map_err(|e| format!("NuGet parse: {e}"))?;
    let items = data["data"].as_array().ok_or("NuGet: no data")?;
    if items.is_empty() {
        return Ok(format!("No NuGet packages found for '{query}'"));
    }
    let mut out = format!("NuGet (.NET) results for '{query}':\n\n");
    for (i, r) in items.iter().enumerate() {
        let id = r["id"].as_str().unwrap_or("?");
        let version = r["version"].as_str().unwrap_or("?");
        let desc = r["description"].as_str().unwrap_or("");
        let downloads = r["totalDownloads"].as_u64().unwrap_or(0);
        let verified = r["verified"].as_bool().unwrap_or(false);
        let authors = r["authors"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| r["authors"].as_str().unwrap_or("").to_string());
        out.push_str(&format!(
            "{}. {} (v{}){}\n   {}\n   Authors: {} | Downloads: {}\n   https://www.nuget.org/packages/{}\n\n",
            i + 1,
            id,
            version,
            if verified { " [verified]" } else { "" },
            trunc(desc, 150),
            authors,
            downloads,
            id
        ));
    }
    Ok(out)
}

// ── Homebrew ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn homebrew_search(query: String) -> Result<String, String> {
    let client = kclient()?;
    let slug = query.to_lowercase().replace(' ', "-");

    // Try direct formula lookup
    let url = format!("https://formulae.brew.sh/api/formula/{slug}.json");
    if let Ok(r) = client.get(&url).send().await {
        if r.status().is_success() {
            if let Ok(data) = r.json::<Value>().await {
                let name = data["name"].as_str().unwrap_or("?");
                let desc = data["desc"].as_str().unwrap_or("");
                let version = data["versions"]["stable"].as_str().unwrap_or("?");
                let homepage = data["homepage"].as_str().unwrap_or("");
                let deps = data["dependencies"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                return Ok(format!(
                    "Homebrew formula: {} (v{})\n{}\nHomepage: {}\nInstall: brew install {}\nDependencies: {}\nhttps://formulae.brew.sh/formula/{}",
                    name, version, desc, homepage, name,
                    if deps.is_empty() { "none".to_string() } else { deps },
                    name
                ));
            }
        }
    }

    // Try cask lookup
    let url2 = format!("https://formulae.brew.sh/api/cask/{slug}.json");
    if let Ok(r2) = client.get(&url2).send().await {
        if r2.status().is_success() {
            if let Ok(data) = r2.json::<Value>().await {
                let token = data["token"].as_str().unwrap_or("?");
                let desc = data["desc"].as_str().unwrap_or("");
                let version = data["version"].as_str().unwrap_or("?");
                let homepage = data["homepage"].as_str().unwrap_or("");
                return Ok(format!(
                    "Homebrew cask: {} (v{})\n{}\nHomepage: {}\nInstall: brew install --cask {}\nhttps://formulae.brew.sh/cask/{}",
                    token, version, desc, homepage, token, token
                ));
            }
        }
    }

    Ok(format!(
        "No Homebrew formula or cask found for '{query}'. Try an exact package name (e.g. 'ffmpeg', 'visual-studio-code')."
    ))
}

// ── MDN Web Docs ──────────────────────────────────────────────────

#[tauri::command]
pub async fn mdn_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://developer.mozilla.org/api/v1/search")
        .query(&[("q", &query), ("size", &n.to_string())])
        .send()
        .await
        .map_err(|e| format!("MDN: {e}"))?;
    let data: Value = resp.json().await.map_err(|e| format!("MDN parse: {e}"))?;
    let docs = data["documents"].as_array().ok_or("MDN: no documents")?;
    if docs.is_empty() {
        return Ok(format!("No MDN docs found for '{query}'"));
    }
    let mut out = format!("MDN Web Docs for '{query}':\n\n");
    for (i, d) in docs.iter().enumerate() {
        let title = d["title"].as_str().unwrap_or("?");
        let summary = d["summary"].as_str().unwrap_or("");
        let mdn_slug = d["slug"].as_str().unwrap_or("");
        let locale = d["locale"].as_str().unwrap_or("en-US");
        out.push_str(&format!(
            "{}. {}\n   {}\n   https://developer.mozilla.org/{}/docs/{}\n\n",
            i + 1,
            title,
            trunc(summary, 200),
            locale,
            mdn_slug
        ));
    }
    Ok(out)
}

// ── cdnjs (frontend CDN libraries) ────────────────────────────────

#[tauri::command]
pub async fn cdnjs_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://api.cdnjs.com/libraries")
        .query(&[
            ("search", query.as_str()),
            ("fields", "description,version,homepage,keywords"),
            ("limit", &n.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("cdnjs: {e}"))?;
    let data: Value = resp.json().await.map_err(|e| format!("cdnjs parse: {e}"))?;
    let results = data["results"].as_array().ok_or("cdnjs: no results")?;
    if results.is_empty() {
        return Ok(format!("No cdnjs libraries found for '{query}'"));
    }
    let mut out = format!("cdnjs libraries for '{query}':\n\n");
    for (i, r) in results.iter().enumerate() {
        let name = r["name"].as_str().unwrap_or("?");
        let version = r["version"].as_str().unwrap_or("?");
        let desc = r["description"].as_str().unwrap_or("");
        let homepage = r["homepage"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. {} (v{})\n   {}\n   CDN: https://cdnjs.cloudflare.com/ajax/libs/{}/{}/{}.min.js\n   {}\n\n",
            i + 1, name, version, trunc(desc, 150), name, version, name, homepage
        ));
    }
    Ok(out)
}

// ── Bundlephobia (NPM package bundle size) ────────────────────────

#[tauri::command]
pub async fn bundlephobia_search(package: String) -> Result<String, String> {
    let client = kclient()?;
    let url = format!(
        "https://bundlephobia.com/api/size?package={}",
        package.trim()
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Bundlephobia: {e}"))?;
    if !resp.status().is_success() {
        return Ok(format!(
            "Package '{}' not found on Bundlephobia. Try exact npm package name.",
            package
        ));
    }
    let d: Value = resp
        .json()
        .await
        .map_err(|e| format!("Bundlephobia parse: {e}"))?;
    let name = d["name"].as_str().unwrap_or("?");
    let version = d["version"].as_str().unwrap_or("?");
    let size = d["size"].as_u64().unwrap_or(0);
    let gzip = d["gzip"].as_u64().unwrap_or(0);
    let dep_count = d["dependencyCount"].as_u64().unwrap_or(0);
    let has_side_effects = d["hasSideEffects"].as_bool().unwrap_or(true);
    fn fmt_size(b: u64) -> String {
        if b >= 1_000_000 {
            format!("{:.1} MB", b as f64 / 1_000_000.0)
        } else if b >= 1_000 {
            format!("{:.1} KB", b as f64 / 1_000.0)
        } else {
            format!("{b} B")
        }
    }
    let mut out = format!(
        "📦 {} v{}\n   Minified: {} | Gzipped: {}\n   Dependencies: {} | Tree-shakeable: {}\n   https://bundlephobia.com/package/{}@{}\n",
        name, version, fmt_size(size), fmt_size(gzip), dep_count,
        if !has_side_effects { "Yes" } else { "No" },
        name, version
    );
    if let Some(deps) = d["dependencySizes"].as_array() {
        if !deps.is_empty() {
            out.push_str("\n   Top dependencies by size:\n");
            for dep in deps.iter().take(5) {
                let dn = dep["name"].as_str().unwrap_or("?");
                let ds = dep["approximateSize"].as_u64().unwrap_or(0);
                out.push_str(&format!("   · {} ({})\n", dn, fmt_size(ds)));
            }
        }
    }
    Ok(out)
}

// ── Dev.to (developer articles) ───────────────────────────────────

#[tauri::command]
pub async fn devto_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let n = max_results.unwrap_or(10).min(20);
    let direct = format!("https://dev.to/search?q={}", query.replace(' ', "%20"));
    public_site_search(&query, "dev.to", None, "DEV Community", n, &direct).await
}

// ── Reddit (discussions) ──────────────────────────────────────────

#[tauri::command]
pub async fn reddit_search(
    query: String,
    subreddit: Option<String>,
    max_results: Option<u32>,
) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(25);
    let url = match &subreddit {
        Some(sub) => format!("https://www.reddit.com/r/{}/search.json", sub),
        None => "https://www.reddit.com/search.json".to_string(),
    };
    let mut params: Vec<(&str, String)> = vec![
        ("q", query.clone()),
        ("limit", n.to_string()),
        ("sort", "relevance".to_string()),
    ];
    if subreddit.is_some() {
        params.push(("restrict_sr", "1".to_string()));
    }
    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .map_err(|e| format!("Reddit: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Reddit returned {}", resp.status()));
    }
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Reddit parse: {e}"))?;
    let posts = data["data"]["children"]
        .as_array()
        .ok_or("Reddit: no data")?;
    if posts.is_empty() {
        return Ok(format!("No Reddit posts found for '{query}'"));
    }
    let retrieved = retrieved_at();
    let mut out = format!("Reddit results for '{query}':\nretrieved_at: {retrieved}\n\n");
    for (i, p) in posts.iter().enumerate() {
        let d = &p["data"];
        let title = d["title"].as_str().unwrap_or("?");
        let sub = d["subreddit"].as_str().unwrap_or("?");
        let score = d["score"].as_i64().unwrap_or(0);
        let comments = d["num_comments"].as_u64().unwrap_or(0);
        let author = d["author"].as_str().unwrap_or("?");
        let selftext = d["selftext"].as_str().unwrap_or("");
        let permalink = d["permalink"].as_str().unwrap_or("");
        let published =
            unix_time_rfc3339(d.get("created_utc")).unwrap_or_else(|| "unknown".to_string());
        let updated = unix_time_rfc3339(d.get("edited")).unwrap_or_else(|| "unknown".to_string());
        out.push_str(&format!(
            "{}. [r/{}] {}\n   by u/{} | ⬆️{} | 💬{}\n   {}\n   published_date: {}\n   updated_date: {}\n   last_activity_date: unknown\n   retrieved_at: {}\n   https://reddit.com{}\n\n",
            i + 1,
            sub,
            title,
            author,
            score,
            comments,
            trunc(selftext, 150),
            published,
            updated,
            retrieved,
            permalink
        ));
    }
    Ok(out)
}

// ── Deal / second-hand marketplace public searches ───────────────

#[tauri::command]
pub async fn smzdm_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("smzdm_search requires a non-empty query".into());
    }
    let n = max_results.unwrap_or(10).min(20);
    let direct = format!(
        "https://search.smzdm.com/?c=home&s={}",
        url_query_component(q)
    );
    public_site_search(q, "smzdm.com", None, "什么值得买/SMZDM", n, &direct).await
}

#[tauri::command]
pub async fn xianyu_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("xianyu_search requires a non-empty query".into());
    }
    let n = max_results.unwrap_or(10).min(20);
    let direct = format!(
        "https://www.goofish.com/search?q={}",
        url_query_component(q)
    );
    public_site_search(q, "goofish.com", None, "闲鱼/Goofish", n, &direct).await
}

#[tauri::command]
pub async fn zhuanzhuan_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("zhuanzhuan_search requires a non-empty query".into());
    }
    let n = max_results.unwrap_or(10).min(20);
    let direct = format!(
        "https://www.zhuanzhuan.com/search?keyword={}",
        url_query_component(q)
    );
    public_site_search(q, "zhuanzhuan.com", None, "转转/Zhuanzhuan", n, &direct).await
}

// ── Steam (game search) ──────────────────────────────────────────

#[tauri::command]
pub async fn steam_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://store.steampowered.com/api/storesearch/")
        .query(&[("term", query.as_str()), ("l", "schinese"), ("cc", "CN")])
        .send()
        .await
        .map_err(|e| format!("Steam: {e}"))?;
    let data: Value = resp.json().await.map_err(|e| format!("Steam parse: {e}"))?;
    let items = data["items"].as_array().ok_or("Steam: no items")?;
    if items.is_empty() {
        return Ok(format!("No Steam games found for '{query}'"));
    }
    let mut out = format!("Steam games for '{query}':\n\n");
    for (i, g) in items.iter().take(n as usize).enumerate() {
        let name = g["name"].as_str().unwrap_or("?");
        let appid = g["id"].as_u64().unwrap_or(0);
        let price = g["price"]
            .as_object()
            .map(|p| {
                let final_price = p.get("final").and_then(|v| v.as_u64()).unwrap_or(0);
                if final_price == 0 {
                    "Free".to_string()
                } else {
                    format!("¥{:.2}", final_price as f64 / 100.0)
                }
            })
            .unwrap_or_else(|| "N/A".to_string());
        let platforms = {
            let mut pl = Vec::new();
            if g["platforms"]["windows"].as_bool().unwrap_or(false) {
                pl.push("Win");
            }
            if g["platforms"]["mac"].as_bool().unwrap_or(false) {
                pl.push("Mac");
            }
            if g["platforms"]["linux"].as_bool().unwrap_or(false) {
                pl.push("Linux");
            }
            pl.join("/")
        };
        out.push_str(&format!(
            "{}. {} (AppID: {})\n   Price: {} | Platforms: {}\n   https://store.steampowered.com/app/{}\n\n",
            i + 1, name, appid, price, platforms, appid
        ));
    }
    Ok(out)
}

// ── Iconify (icon search across 200+ sets) ───────────────────────

#[tauri::command]
pub async fn iconify_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(20).min(50);
    let resp = client
        .get("https://api.iconify.design/search")
        .query(&[("query", &query), ("limit", &n.to_string())])
        .send()
        .await
        .map_err(|e| format!("Iconify: {e}"))?;
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("Iconify parse: {e}"))?;
    let icons = data["icons"].as_array().ok_or("Iconify: no icons")?;
    if icons.is_empty() {
        return Ok(format!("No icons found for '{query}'"));
    }
    let total = data["total"].as_u64().unwrap_or(0);
    let mut out = format!("Icons for '{}' ({} total):\n\n", query, total);
    for (i, icon_name) in icons.iter().enumerate() {
        let name = icon_name.as_str().unwrap_or("?");
        let parts: Vec<&str> = name.splitn(2, ':').collect();
        let (prefix, icon) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("?", name)
        };
        out.push_str(&format!(
            "{}. {} (set: {})\n   SVG: https://api.iconify.design/{}/{}.svg\n   Use: <iconify-icon icon=\"{}\"></iconify-icon>\n\n",
            i + 1, icon, prefix, prefix, icon, name
        ));
    }
    Ok(out)
}

// ── Color palettes (ColourLovers) ─────────────────────────────────

#[tauri::command]
pub async fn color_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let client = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = client
        .get("https://www.colourlovers.com/api/palettes")
        .query(&[
            ("keywords", &query),
            ("format", &"json".to_string()),
            ("numResults", &n.to_string()),
            ("orderCol", &"numVotes".to_string()),
            ("sortBy", &"DESC".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("ColourLovers: {e}"))?;
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("ColourLovers parse: {e}"))?;
    let palettes = data.as_array().ok_or("ColourLovers: unexpected response")?;
    if palettes.is_empty() {
        return Ok(format!("No color palettes found for '{query}'"));
    }
    let mut out = format!("Color palettes for '{query}':\n\n");
    for (i, p) in palettes.iter().enumerate() {
        let title = p["title"].as_str().unwrap_or("?");
        let user = p["userName"].as_str().unwrap_or("?");
        let votes = p["numVotes"].as_u64().unwrap_or(0);
        let colors = p["colors"]
            .as_array()
            .map(|c| {
                c.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| format!("#{s}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        let url = p["url"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. {} (by {})\n   Colors: {}\n   Votes: {} | {}\n\n",
            i + 1,
            title,
            user,
            colors,
            votes,
            url
        ));
    }
    Ok(out)
}

// ── Lobsters (curated tech community) ─────────────────────────────

#[tauri::command]
pub async fn lobsters_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let limit = max_results.unwrap_or(10).min(25);
    let direct = format!("https://lobste.rs/search?q={}", query.replace(' ', "+"));
    public_site_search(&query, "lobste.rs", Some("/s/"), "Lobsters", limit, &direct).await
}

// ── 掘金 / Juejin (Chinese developer community) ──────────────────

#[tauri::command]
pub async fn juejin_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let limit = max_results.unwrap_or(10).min(20);
    let body = serde_json::json!({
        "search_type": 2,
        "keyword": query,
        "cursor": "0",
        "limit": limit,
        "search_id": ""
    });
    let resp = c
        .post("https://api.juejin.cn/search_api/v1/search")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("掘金: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("掘金 returned {}", resp.status()));
    }
    let json: Value = resp.json().await.map_err(|e| format!("掘金 JSON: {e}"))?;
    let data = json["data"]
        .as_array()
        .ok_or("掘金: response did not contain a data array")?;
    let retrieved = retrieved_at();
    let mut out = format!("掘金 (Juejin) articles:\nretrieved_at: {retrieved}\n\n");
    if data.is_empty() {
        out.push_str("search_status: empty\nThe endpoint returned no public articles.\n");
        return Ok(out);
    }
    for (i, item) in data.iter().take(limit as usize).enumerate() {
        let rm = &item["result_model"];
        let info = &rm["article_info"];
        let author = rm["author_user_info"]["user_name"].as_str().unwrap_or("?");
        let title = info["title"].as_str().unwrap_or("?");
        let brief = info["brief_content"].as_str().unwrap_or("");
        let aid = info["article_id"].as_str().unwrap_or("");
        let views = info["view_count"].as_u64().unwrap_or(0);
        let likes = info["digg_count"].as_u64().unwrap_or(0);
        let published =
            unix_time_rfc3339(info.get("ctime")).unwrap_or_else(|| "unknown".to_string());
        let updated = unix_time_rfc3339(info.get("mtime")).unwrap_or_else(|| "unknown".to_string());
        out.push_str(&format!(
            "{}. {} (by {})\n   {}\n   Views: {} | Likes: {}\n   published_date: {}\n   updated_date: {}\n   last_activity_date: unknown\n   retrieved_at: {}\n   https://juejin.cn/post/{}\n\n",
            i + 1,
            title,
            author,
            trunc(brief, 200),
            views,
            likes,
            published,
            updated,
            retrieved,
            aid,
        ));
    }
    Ok(out)
}

// ── HTML helpers for WP REST / scraping ──────────────────────────────

fn strip_html(s: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in s.chars() {
        match c {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#8217;", "\u{2019}")
        .replace("&#8211;", "\u{2013}")
        .replace("&#8230;", "\u{2026}")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

async fn wp_search(
    c: &Client,
    base: &str,
    query: &str,
    n: u32,
    label: &str,
) -> Result<String, String> {
    let url = format!("{}/wp-json/wp/v2/posts", base);
    let resp = c
        .get(&url)
        .query(&[
            ("search", query),
            ("per_page", &n.to_string()),
            ("_fields", "title,link,excerpt,date"),
        ])
        .send()
        .await
        .map_err(|e| format!("{label}: {e}"))?;
    let items: Vec<Value> = resp.json().await.unwrap_or_default();
    if items.is_empty() {
        return Ok(format!("No {label} articles found for '{query}'"));
    }
    let mut out = format!("{label} articles for '{query}':\n\n");
    for (i, item) in items.iter().take(n as usize).enumerate() {
        let title = strip_html(item["title"]["rendered"].as_str().unwrap_or("?"));
        let link = item["link"].as_str().unwrap_or("");
        let excerpt = strip_html(item["excerpt"]["rendered"].as_str().unwrap_or(""));
        let date = item["date"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. {}\n   {}\n   {} | {}\n\n",
            i + 1,
            title,
            trunc(&excerpt, 200),
            &date[..date.len().min(10)],
            link,
        ));
    }
    Ok(out)
}

// ── Codrops (creative CSS/JS demos & experiments) ───────────────────

#[tauri::command]
pub async fn codrops_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    wp_search(&c, "https://tympanus.net/codrops", &query, n, "Codrops").await
}

// ── Smashing Magazine (web design & UX articles) ────────────────────

#[tauri::command]
pub async fn smashingmag_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20) as usize;
    let resp = c
        .get("https://www.smashingmagazine.com/feed/")
        .send()
        .await
        .map_err(|e| format!("SmashingMag RSS: {e}"))?;
    let xml = resp
        .text()
        .await
        .map_err(|e| format!("SmashingMag RSS: {e}"))?;
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();
    let mut out = format!("Smashing Magazine articles matching '{query}':\n\n");
    let mut count = 0;
    let mut pos = 0;
    while count < n {
        let Some(item_start) = xml[pos..].find("<item>") else {
            break;
        };
        let Some(item_end) = xml[pos + item_start..].find("</item>") else {
            break;
        };
        let item = &xml[pos + item_start..pos + item_start + item_end + 7];
        pos = pos + item_start + item_end + 7;
        let title = extract_xml_tag(item, "title");
        let link = extract_xml_tag(item, "link");
        let desc = extract_xml_tag(item, "description");
        let date = extract_xml_tag(item, "pubDate");
        let haystack = format!("{} {} {}", title, desc, link).to_lowercase();
        if !keywords.is_empty() && !keywords.iter().any(|k| haystack.contains(k)) {
            continue;
        }
        count += 1;
        out.push_str(&format!(
            "{}. {}\n   {}\n   {} | {}\n\n",
            count,
            strip_html(&title),
            trunc(&strip_html(&desc), 200),
            &date[..date.len().min(16)],
            link,
        ));
    }
    if count == 0 {
        return Ok(format!("No Smashing Magazine articles found matching '{query}'. RSS feed only has recent articles; for older content use web_search."));
    }
    Ok(out)
}

fn extract_xml_tag(xml: &str, tag: &str) -> String {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        let after_tag = start + open.len();
        let content_start = xml[after_tag..]
            .find('>')
            .map(|i| after_tag + i + 1)
            .unwrap_or(after_tag);
        if let Some(end) = xml[content_start..].find(&close) {
            let raw = &xml[content_start..content_start + end];
            return raw
                .trim_start_matches("<![CDATA[")
                .trim_end_matches("]]>")
                .trim()
                .to_string();
        }
    }
    String::new()
}

// ── CSS-Tricks (CSS tutorials & techniques) ─────────────────────────

#[tauri::command]
pub async fn css_tricks_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    match wp_search(&c, "https://css-tricks.com", &query, n, "CSS-Tricks").await {
        Ok(r) => Ok(r),
        Err(_) => {
            Ok("CSS-Tricks search unavailable. Try web_search for CSS-Tricks content.".into())
        }
    }
}

// ── CodePen (real UI component implementations) ─────────────────────

#[tauri::command]
pub async fn codepen_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let n = max_results.unwrap_or(10).min(20);
    let direct = format!(
        "https://codepen.io/search/pens?q={}",
        query.replace(' ', "+")
    );
    public_site_search(&query, "codepen.io", Some("/pen/"), "CodePen", n, &direct).await
}

// ── Dribbble (professional UI design inspiration) ───────────────────

#[tauri::command]
pub async fn dribbble_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let n = max_results.unwrap_or(10).min(20);
    let direct = format!("https://dribbble.com/search/{}", query.replace(' ', "-"));
    public_site_search(
        &query,
        "dribbble.com",
        Some("/shots/"),
        "Dribbble",
        n,
        &direct,
    )
    .await
}

// ── Awwwards (award-winning website designs) ────────────────────────

#[tauri::command]
pub async fn awwwards_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20) as usize;
    let resp = c
        .get("https://www.awwwards.com/websites/")
        .query(&[("q", query.as_str())])
        .header("Accept", "text/html")
        .send()
        .await
        .map_err(|e| format!("Awwwards: {e}"))?;
    let html = resp.text().await.map_err(|e| format!("Awwwards: {e}"))?;
    let mut out = format!("Awwwards sites for '{query}':\n\n");
    let mut count = 0;
    let mut pos = 0;
    let mut seen = std::collections::HashSet::new();
    while count < n && pos < html.len() {
        let Some(idx) = html[pos..].find("/sites/") else {
            break;
        };
        let start = pos + idx;
        let end = html[start + 7..]
            .find(['"', '\'', '<', ' ', '?'])
            .unwrap_or(0);
        let path = &html[start..start + 7 + end.min(200)];
        let slug = path.split('/').rfind(|s| !s.is_empty()).unwrap_or("");
        pos = start + 7 + end;
        if slug.is_empty() || slug == "sites" || slug == "new" || slug.len() < 3 {
            continue;
        }
        if !seen.insert(slug.to_string()) {
            continue;
        }
        let title = slug.replace('-', " ");
        count += 1;
        out.push_str(&format!(
            "{}. {}\n   https://www.awwwards.com{}\n\n",
            count, title, path,
        ));
    }
    if count == 0 {
        return Ok(format!("No Awwwards sites found for '{query}'"));
    }
    Ok(out)
}

// ── V2EX (Chinese developer community via SOV2EX search) ──────────

fn ascii_search_terms(text: &str) -> HashSet<String> {
    text.to_ascii_lowercase()
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || character == '+' || character == '#')
        })
        .filter(|term| !term.is_empty())
        .map(str::to_string)
        .collect()
}

fn v2ex_hit_matches_language_anchor(query: &str, source: &Value) -> bool {
    const LANGUAGE_GROUPS: &[(&[&str], &[&str])] = &[
        (&["rust"], &["rust", "cargo", "tokio"]),
        (
            &["python"],
            &["python", "pytest", "django", "flask", "fastapi"],
        ),
        (&["swift"], &["swift", "swiftui"]),
        (&["kotlin"], &["kotlin", "ktor"]),
        (&["go", "golang"], &["go", "golang", "gopher"]),
        (&["javascript", "js"], &["javascript", "js", "nodejs"]),
        (&["typescript", "ts"], &["typescript", "ts"]),
        (&["java"], &["java", "spring"]),
        (&["ruby"], &["ruby", "rails"]),
        (&["php"], &["php", "laravel"]),
        (&["c#", "csharp"], &["c#", "csharp", "dotnet"]),
        (&["c++", "cpp"], &["c++", "cpp"]),
    ];
    let query_terms = ascii_search_terms(query);
    let active_groups = LANGUAGE_GROUPS
        .iter()
        .filter(|(query_aliases, _)| {
            query_aliases
                .iter()
                .any(|alias| query_terms.contains(*alias))
        })
        .collect::<Vec<_>>();
    if active_groups.is_empty() {
        return true;
    }

    let searchable = format!(
        "{} {}",
        source.get("title").and_then(Value::as_str).unwrap_or(""),
        source.get("content").and_then(Value::as_str).unwrap_or("")
    );
    let hit_terms = ascii_search_terms(&searchable);
    active_groups
        .iter()
        .any(|(_, hit_aliases)| hit_aliases.iter().any(|alias| hit_terms.contains(*alias)))
}

#[tauri::command]
pub async fn v2ex_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = c
        .get("https://www.sov2ex.com/api/search")
        .query(&[
            ("q", query.as_str()),
            ("size", &n.to_string()),
            ("sort", "sumup"),
        ])
        .send()
        .await
        .map_err(|e| format!("V2EX (SOV2EX): {e}"))?;
    let status = resp.status();
    if status.as_u16() == 429 {
        return Err("V2EX (SOV2EX) rate limited (HTTP 429)".into());
    }
    if !status.is_success() {
        return Err(format!("V2EX (SOV2EX) returned HTTP {status}"));
    }
    let data: Value = resp.json().await.map_err(|e| format!("V2EX parse: {e}"))?;
    let total = data["total"].as_u64().unwrap_or(0);
    let hits = data["hits"].as_array();
    if total == 0 || hits.is_none() {
        return Ok(format!("No V2EX discussions found for '{query}'"));
    }
    let retrieved = retrieved_at();
    let mut out = format!(
        "V2EX discussions for '{}' ({} total, via SOV2EX third-party index):\nretrieved_at: {retrieved}\n\n",
        query, total
    );
    let relevant_hits = hits
        .unwrap()
        .iter()
        .filter(|hit| v2ex_hit_matches_language_anchor(&query, &hit["_source"]))
        .take(n as usize)
        .collect::<Vec<_>>();
    if relevant_hits.is_empty() {
        out.push_str("search_status: empty\nThe third-party index returned hits, but none matched the explicit programming-language term in the query.\n");
        return Ok(out);
    }
    for (i, hit) in relevant_hits.into_iter().enumerate() {
        let src = &hit["_source"];
        let id = src["id"].as_u64().unwrap_or(0);
        let title = strip_html(src["title"].as_str().unwrap_or("?"));
        let member = src["member"].as_str().unwrap_or("?");
        let replies = src["replies"].as_u64().unwrap_or(0);
        let created = value_or_unknown(src["created"].as_str());
        let content = src["content"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. {} (by @{}, {} replies)\n   {}\n   published_date: {}\n   updated_date: unknown\n   last_activity_date: unknown\n   retrieved_at: {}\n   https://www.v2ex.com/t/{}\n\n",
            i + 1,
            title,
            member,
            replies,
            trunc(content, 200),
            created,
            retrieved,
            id,
        ));
    }
    Ok(out)
}

// ── SegmentFault / 思否 (Chinese developer Q&A) ───────────────────

#[tauri::command]
pub async fn segmentfault_search(
    query: String,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20) as usize;
    let resp = c
        .get("https://api.segmentfault.com/search")
        .query(&[("q", query.as_str()), ("page", "1")])
        .send()
        .await
        .map_err(|e| format!("SegmentFault: {e}"))?;
    let status = resp.status();
    if status.as_u16() == 429 {
        return Err("SegmentFault rate limited (HTTP 429)".into());
    }
    if !status.is_success() {
        return Err(format!("SegmentFault returned HTTP {status}"));
    }
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("SegmentFault parse: {e}"))?;
    let rows = data["data"]["rows"].as_array();
    if rows.is_none() || rows.unwrap().is_empty() {
        return Ok(format!("No SegmentFault results found for '{query}'"));
    }
    let content_rows = rows
        .unwrap()
        .iter()
        .filter(|row| matches!(row["type"].as_str(), Some("question" | "article")))
        .take(n)
        .collect::<Vec<_>>();
    if content_rows.is_empty() {
        return Ok(format!(
            "SegmentFault (思否) results for '{query}':\nsearch_status: empty\nThe endpoint returned no public question or article results."
        ));
    }
    let retrieved = retrieved_at();
    let mut out = format!(
        "SegmentFault (思否) question and article results for '{query}':\nretrieved_at: {retrieved}\n\n"
    );
    for (i, row) in content_rows.into_iter().enumerate() {
        let rtype = row["type"].as_str().unwrap_or("article");
        let title = row["title"].as_str().unwrap_or("?");
        let excerpt = row["excerpt"].as_str().unwrap_or("");
        let path = row["url"].as_str().unwrap_or("");
        let votes = row["votes"].as_i64().unwrap_or(0);
        let user = row["user"]["name"].as_str().unwrap_or("?");
        let type_label = if rtype == "question" {
            "Q&A"
        } else {
            "Article"
        };
        let published = row["createdDate"]
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| unix_time_rfc3339(row.get("created")))
            .unwrap_or_else(|| "unknown".to_string());
        out.push_str(&format!(
            "{}. [{}] {} (by @{}, votes: {})\n   result_type: {}\n   {}\n   published_date: {}\n   updated_date: unknown\n   last_activity_date: unknown\n   retrieved_at: {}\n   https://segmentfault.com{}\n\n",
            i + 1,
            type_label,
            title,
            user,
            votes,
            rtype,
            trunc(excerpt, 200),
            published,
            retrieved,
            path,
        ));
    }
    Ok(out)
}

// ── GitHub Discussions (open-source project discussions) ───────────

#[tauri::command]
pub async fn github_discussions_search(
    query: String,
    max_results: Option<u32>,
) -> Result<String, String> {
    let n = max_results.unwrap_or(10).min(20);
    let direct = format!(
        "https://github.com/search?q={}&type=discussions",
        query.replace(' ', "+")
    );
    public_site_search(
        &query,
        "github.com",
        Some("/discussions/"),
        "GitHub Discussions",
        n,
        &direct,
    )
    .await
}

// ── ProductHunt (discover developer tools & products) ─────────────

#[tauri::command]
pub async fn producthunt_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let n = max_results.unwrap_or(10).min(20);
    let direct = format!(
        "https://www.producthunt.com/search?q={}",
        query.replace(' ', "+")
    );
    public_site_search(
        &query,
        "producthunt.com",
        Some("/posts/"),
        "Product Hunt",
        n,
        &direct,
    )
    .await
}

#[tauri::command]
pub async fn freecodecamp_search(
    query: String,
    max_results: Option<u32>,
) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let body = serde_json::json!({
        "query": query,
        "hitsPerPage": n,
    });
    let resp = c
        .post("https://QMJYL5WYTI-dsn.algolia.net/1/indexes/news/query")
        .header("X-Algolia-Application-Id", "QMJYL5WYTI")
        .header("X-Algolia-API-Key", "89770b24481654192d7a5c402c6ad9a0")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    ensure_provider_http_success("freeCodeCamp", resp.status())?;
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let retrieved = retrieved_at();
    let mut out = format!("FreeCodeCamp articles for '{query}':\nretrieved_at: {retrieved}\n\n");
    let mut count = 0usize;
    if let Some(hits) = data["hits"].as_array() {
        for (i, h) in hits.iter().take(n as usize).enumerate() {
            let title = h["title"].as_str().unwrap_or("");
            let url = h["url"].as_str().unwrap_or("");
            let author = h["author"]["name"]
                .as_str()
                .or_else(|| h["author"].as_str())
                .unwrap_or("?");
            let tags: Vec<&str> = h["tags"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let tag_str = if tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", tags.join(", "))
            };
            let published = freecodecamp_published_date(h);
            let dates = provider_date_lines(
                published
                    .as_ref()
                    .map(|(value, field)| (value.as_str(), *field)),
                None,
                None,
                None,
                &retrieved,
            );
            count += 1;
            out.push_str(&format!(
                "{}. {} (by {}){}\n{}   {}\n\n",
                i + 1,
                title,
                author,
                tag_str,
                dates,
                url
            ));
        }
    }
    if count == 0 {
        out.push_str("search_status: empty\nNo results found.\n");
    }
    Ok(out)
}

#[tauri::command]
pub async fn github_trending(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(15).min(25) as usize;
    let lang = query.trim();
    let url = github_trending_url(lang)?;
    let mut html = String::new();
    let mut last_err = String::from("unknown");
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(800 * attempt as u64)).await;
        }
        match c.get(&url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if status == 200 {
                    html = resp.text().await.map_err(|e| e.to_string())?;
                    break;
                } else if status == 429 && attempt < 2 {
                    last_err = format!("rate limited ({})", status);
                    continue;
                } else {
                    return Err(format!("GitHub returned HTTP {}", status));
                }
            }
            Err(e) if attempt < 2 => { last_err = e.to_string(); continue; }
            Err(e) => return Err(format!("request failed after retries: {}", e)),
        }
    }
    if html.is_empty() {
        return Err(format!("all retries exhausted: {}", last_err));
    }
    let retrieved = retrieved_at();
    let mut out = format!(
        "GitHub Trending repos (weekly, {}):\nretrieved_at: {retrieved}\n\n",
        if lang.is_empty() || lang == "all" {
            "all languages"
        } else {
            lang
        }
    );
    let mut count = 0;
    for chunk in html.split("Box-row") {
        if count >= n {
            break;
        }
        // Extract repo href: href="/owner/repo"
        let repo = {
            let marker = "href=\"/";
            if let Some(pos) = chunk.find(marker) {
                let rest = &chunk[pos + 7..];
                if let Some(end) = rest.find('"') {
                    let path = &rest[..end];
                    if path.contains('/')
                        && !path.contains("login")
                        && !path.contains("signup")
                        && path.split('/').count() == 2
                    {
                        Some(path.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };
        let repo = match repo {
            Some(r) => r,
            None => continue,
        };
        // Extract description from <p class="col-9
        let desc = {
            let marker = "col-9";
            if let Some(pos) = chunk.find(marker) {
                let rest = &chunk[pos..];
                if let Some(gt) = rest.find('>') {
                    let inner = &rest[gt + 1..];
                    if let Some(end) = inner.find("</p>") {
                        let raw = inner[..end].trim();
                        let clean: String = raw.chars().filter(|c| *c != '\n').collect();
                        let clean = clean.trim().to_string();
                        if clean.is_empty() {
                            String::new()
                        } else {
                            trunc(&clean, 200).to_string()
                        }
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        };
        // Extract language
        let plang = {
            let marker = "programmingLanguage\">";
            if let Some(pos) = chunk.find(marker) {
                let rest = &chunk[pos + marker.len()..];
                rest.split('<').next().unwrap_or("").trim().to_string()
            } else {
                String::new()
            }
        };
        // Extract stars this week
        let stars = {
            let marker = "stars this";
            if let Some(pos) = chunk.find(marker) {
                let before = &chunk[..pos];
                let num: String = before
                    .chars()
                    .rev()
                    .take_while(|c| c.is_ascii_digit() || *c == ',')
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                if num.is_empty() {
                    String::new()
                } else {
                    format!(" +{} stars/week", num.trim())
                }
            } else {
                String::new()
            }
        };
        count += 1;
        let lang_tag = if plang.is_empty() {
            String::new()
        } else {
            format!(" [{}]", plang)
        };
        out.push_str(&format!("{}. {}{}{}\n", count, repo, lang_tag, stars));
        if !desc.is_empty() {
            out.push_str(&format!("   {}\n", desc));
        }
        out.push_str(&provider_date_lines(None, None, None, None, &retrieved));
        out.push_str(&format!("   https://github.com/{}\n\n", repo));
    }
    if count == 0 {
        out.push_str(GITHUB_TRENDING_EMPTY_NOTICE);
    }
    out.push_str(&format!("Browse: {}\n", url));
    Ok(out)
}

#[tauri::command]
pub async fn infoq_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20) as usize;
    let resp = c
        .get("https://www.infoq.com/search.action")
        .query(&[
            ("queryString", query.as_str()),
            ("page", "0"),
            ("searchOrder", "relevance"),
        ])
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;
    ensure_provider_http_success("InfoQ", resp.status())?;
    let html = resp.text().await.map_err(|e| e.to_string())?;
    let retrieved = retrieved_at();
    let mut out = format!("InfoQ articles for '{query}':\nretrieved_at: {retrieved}\n\n");
    let mut count = 0;
    // Parse search result links: <a href="/articles/..." or <a href="/news/..."
    let patterns = ["href=\"/articles/", "href=\"/news/"];
    let mut seen = std::collections::HashSet::new();
    for pat in &patterns {
        for piece in html.split(pat) {
            if count >= n {
                break;
            }
            // Extract path
            let path = if let Some(end) = piece.find('"') {
                let slug = &piece[..end];
                if slug.len() < 3 || slug.contains('<') || slug.contains('>') || slug.len() > 200 {
                    continue;
                }
                let prefix = if pat.contains("articles") {
                    "/articles/"
                } else {
                    "/news/"
                };
                format!("{}{}", prefix, slug)
            } else {
                continue;
            };
            if seen.contains(&path) {
                continue;
            }
            seen.insert(path.clone());
            // Extract title: next > then text until <
            let title = if let Some(gt) = piece.find('>') {
                let rest = &piece[gt + 1..];
                if let Some(lt) = rest.find('<') {
                    let t = rest[..lt].trim();
                    if t.is_empty() || t.len() < 3 {
                        continue;
                    }
                    html_decode(t)
                } else {
                    continue;
                }
            } else {
                continue;
            };
            count += 1;
            out.push_str(&format!(
                "{}. {}\n{}   https://www.infoq.com{}\n\n",
                count,
                title,
                provider_date_lines(None, None, None, None, &retrieved),
                path
            ));
        }
    }
    if count == 0 {
        out.push_str("search_status: empty\nNo results found in the successful InfoQ response.\n");
    }
    out.push_str(&format!(
        "InfoQ search: https://www.infoq.com/search/?q={}\n",
        query.replace(' ', "+"),
    ));
    Ok(out)
}

fn ensure_provider_http_success(provider: &str, status: reqwest::StatusCode) -> Result<(), String> {
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        Err(format!("{provider} rate-limited (HTTP 429)"))
    } else if !status.is_success() {
        Err(format!("{provider} returned HTTP {status}"))
    } else {
        Ok(())
    }
}

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

#[tauri::command]
pub async fn hackernoon_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let n = max_results.unwrap_or(10).min(20);
    let search_query = format!("site:hackernoon.com {query}");
    let hits = ddg_surface_checked(&search_query)
        .await
        .map_err(|error| format!("HackerNoon public search: {error}"))?;
    let retrieved = retrieved_at();
    let mut out = format!(
        "HackerNoon pages for '{query}' (via public web search, not a HackerNoon API):\nretrieved_at: {retrieved}\n\n"
    );
    let mut count = 0usize;
    for (title, url, snippet, _) in hits {
        if !url.contains("hackernoon.com") {
            continue;
        }
        count += 1;
        out.push_str(&format!(
            "{}. {}\n   {}\n   published_date: unknown\n   updated_date: unknown\n   last_activity_date: unknown\n   retrieved_at: {}\n   {}\n\n",
            count,
            title,
            trunc(&snippet, 180),
            retrieved,
            url,
        ));
        if count >= n as usize {
            break;
        }
    }
    if count == 0 {
        out.push_str(
            "search_status: empty\nThe public-search response contained no usable matching page. This can mean no match, an index coverage gap, or an unrecognized result layout; it does not prove HackerNoon has no matching content.\n\n",
        );
    }
    out.push_str(&format!(
        "HackerNoon search page: https://hackernoon.com/search?query={}\n",
        query.replace(' ', "+"),
    ));
    Ok(out)
}

fn format_codeberg_repository_item(item: &Value, index: usize, retrieved: &str) -> String {
    let name = item["full_name"].as_str().unwrap_or("?");
    let description = item["description"].as_str().unwrap_or("");
    let stars = item["stars_count"].as_u64().unwrap_or(0);
    let language = item["language"].as_str().unwrap_or("");
    let topics: Vec<&str> = item["topics"]
        .as_array()
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let language_tag = if language.is_empty() {
        String::new()
    } else {
        format!(" [{language}]")
    };
    let topic_text = if topics.is_empty() {
        String::new()
    } else {
        format!(" ({})", topics.join(", "))
    };
    let dates = repository_date_lines("Codeberg", item, None, retrieved);
    format!(
        "{}. {} ★{}{}{}\n   {}\n{}   {}\n\n",
        index + 1,
        name,
        stars,
        language_tag,
        topic_text,
        trunc(description, 150),
        dates,
        item["html_url"].as_str().unwrap_or(""),
    )
}

#[tauri::command]
pub async fn codeberg_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(25) as usize;
    let resp = c
        .get("https://codeberg.org/api/v1/repos/search")
        .query(&[
            ("q", query.as_str()),
            ("sort", "stars"),
            ("limit", &n.to_string()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if status.as_u16() == 429 {
        return Err("Codeberg rate limited (HTTP 429)".into());
    }
    if !status.is_success() {
        return Err(format!("Codeberg returned HTTP {status}"));
    }
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let retrieved = retrieved_at();
    let mut out = format!("Codeberg repos for '{query}':\nretrieved_at: {retrieved}\n\n");
    if let Some(repos) = data["data"].as_array() {
        for (i, r) in repos.iter().enumerate() {
            out.push_str(&format_codeberg_repository_item(r, i, &retrieved));
        }
    }
    if data["data"].as_array().is_none_or(Vec::is_empty) {
        out.push_str("search_status: empty\nNo matching repositories were returned.\n");
    }
    out.push_str(&format!(
        "Codeberg search: https://codeberg.org/explore/repos?q={}\n",
        query.replace(' ', "+")
    ));
    Ok(out)
}

#[tauri::command]
pub async fn bestofjs_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(30) as usize;
    let resp = c
        .get("https://bestofjs-static-api.vercel.app/projects.json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    ensure_provider_http_success("Best of JS", resp.status())?;
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let retrieved = retrieved_at();
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();
    let mut out = format!("Best of JS projects for '{query}':\nretrieved_at: {retrieved}\n\n");
    let mut count = 0;
    if let Some(projects) = data["projects"].as_array() {
        for p in projects {
            if count >= n {
                break;
            }
            let name = p["name"].as_str().unwrap_or("");
            let desc = p["description"].as_str().unwrap_or("");
            let full_name = p["full_name"].as_str().unwrap_or("");
            let tags: Vec<&str> = p["tags"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let haystack =
                format!("{} {} {} {}", name, desc, full_name, tags.join(" ")).to_lowercase();
            if !keywords.is_empty() && !keywords.iter().all(|k| haystack.contains(k)) {
                continue;
            }
            count += 1;
            let stars = p["stars"].as_u64().unwrap_or(0);
            let url = if full_name.is_empty() {
                p["url"].as_str().unwrap_or("").to_string()
            } else {
                format!("https://github.com/{}", full_name)
            };
            let tag_str = if tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", tags.join(", "))
            };
            let created = p
                .get("created_at")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let pushed = p
                .get("pushed_at")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            out.push_str(&format!(
                "{}. {} ★{}{}\n   {}\n{}   {}\n\n",
                count,
                name,
                stars,
                tag_str,
                trunc(desc, 150),
                provider_date_lines(
                    None,
                    created.map(|value| (value, "created_at")),
                    None,
                    pushed.map(|value| (value, "pushed_at")),
                    &retrieved,
                ),
                url,
            ));
        }
    }
    if count == 0 {
        out.push_str("search_status: empty\nNo matching projects.\n");
    }
    out.push_str(&format!(
        "Best of JS: https://bestofjs.org/projects?query={}\n",
        query.replace(' ', "+")
    ));
    Ok(out)
}

#[tauri::command]
pub async fn sourcegraph_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let gql = format!(
        r#"query{{search(query:"{} count:{}",version:V3){{results{{resultCount results{{__typename ... on Repository{{name description}} ... on FileMatch{{repository{{name}} file{{path}} lineMatches{{preview}}}}}}}}}}}}"#,
        query.replace('"', "\\\""),
        n,
    );
    let body = serde_json::json!({ "query": gql });
    let resp = c
        .post("https://sourcegraph.com/.api/graphql")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    ensure_provider_http_success("Sourcegraph", resp.status())?;
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let retrieved = retrieved_at();
    let (result_count, results) = sourcegraph_graphql_results(&data)?;
    let mut out = format!(
        "Sourcegraph code search for '{query}' ({result_count} results):\nretrieved_at: {retrieved}\n\n"
    );
    let mut count = 0usize;
    for r in results {
        if count >= n as usize {
            break;
        }
        let typename = r["__typename"].as_str().unwrap_or("");
        match typename {
            "Repository" => {
                count += 1;
                let name = r["name"].as_str().unwrap_or("?");
                let desc = r["description"].as_str().unwrap_or("");
                out.push_str(&format!("{}. [REPO] {}\n", count, name));
                if !desc.is_empty() {
                    out.push_str(&format!("   {}\n", trunc(desc, 150)));
                }
                out.push_str(&provider_date_lines(None, None, None, None, &retrieved));
                out.push_str(&format!("   https://sourcegraph.com/{}\n\n", name));
            }
            "FileMatch" => {
                count += 1;
                let repo = r["repository"]["name"].as_str().unwrap_or("?");
                let path = r["file"]["path"].as_str().unwrap_or("?");
                let preview = r["lineMatches"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|m| m["preview"].as_str())
                    .unwrap_or("");
                out.push_str(&format!("{}. {} / {}\n", count, repo, path));
                if !preview.is_empty() {
                    out.push_str(&format!("   {}\n", trunc(preview.trim(), 120)));
                }
                out.push_str(&provider_date_lines(None, None, None, None, &retrieved));
                out.push_str(&format!(
                    "   https://sourcegraph.com/{}/-/blob/{}\n\n",
                    repo, path
                ));
            }
            _ => {}
        }
    }
    if count == 0 {
        out.push_str("search_status: empty\nNo results found.\n");
    }
    out.push_str(&format!(
        "Sourcegraph: https://sourcegraph.com/search?q={}\n",
        query.replace(' ', "+"),
    ));
    Ok(out)
}

fn sourcegraph_graphql_results(data: &Value) -> Result<(u64, &[Value]), String> {
    if let Some(errors) = data.get("errors").filter(|errors| !errors.is_null()) {
        let errors = errors
            .as_array()
            .ok_or_else(|| "Sourcegraph GraphQL errors field is malformed".to_string())?;
        if !errors.is_empty() {
            let summary = errors
                .iter()
                .take(3)
                .filter_map(|error| error.get("message").and_then(Value::as_str))
                .map(|message| trunc(message.trim(), 160))
                .filter(|message| !message.is_empty())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(if summary.is_empty() {
                "Sourcegraph GraphQL returned errors without messages".to_string()
            } else {
                format!("Sourcegraph GraphQL returned errors: {summary}")
            });
        }
    }

    let results = data
        .pointer("/data/search/results")
        .ok_or_else(|| "Sourcegraph GraphQL response missing data.search.results".to_string())?;
    let result_count = results
        .get("resultCount")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Sourcegraph GraphQL response missing numeric resultCount".to_string())?;
    let items = results
        .get("results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| "Sourcegraph GraphQL response missing results array".to_string())?;
    Ok((result_count, items))
}

// ── Deep search (surface + Tor + onion engines) ───────────────────

fn tor_client(timeout_secs: u64) -> Result<Client, String> {
    let proxy =
        reqwest::Proxy::all("socks5h://127.0.0.1:9050").map_err(|e| format!("Tor proxy: {e}"))?;
    Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; rv:128.0) Gecko/20100101 Firefox/128.0")
        .build()
        .map_err(|e| format!("Tor client: {e}"))
}

async fn ddg_surface_checked(
    q: &str,
) -> Result<Vec<(String, String, String, &'static str)>, String> {
    let client = kclient()?;
    let resp = client
        .post("https://html.duckduckgo.com/html/")
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .form(&[("q", q), ("kl", "wt-wt")])
        .send()
        .await
        .map_err(|error| format!("DuckDuckGo request: {error}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("DuckDuckGo returned HTTP {status}"));
    }
    let html = resp
        .text()
        .await
        .map_err(|error| format!("DuckDuckGo response body: {error}"))?;
    Ok(parse_ddg_html(&html, "明网"))
}

async fn ddg_surface(q: &str) -> Vec<(String, String, String, &'static str)> {
    ddg_surface_checked(q).await.unwrap_or_default()
}

async fn ddg_onion(q: &str) -> Vec<(String, String, String, &'static str)> {
    let client = match tor_client(20) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let resp = client
        .post("https://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/html/")
        .form(&[("q", q), ("kl", "wt-wt")])
        .send()
        .await;
    let html = match resp {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return vec![],
    };
    parse_ddg_html(&html, "Tor匿名")
}

async fn ahmia_search_layer(q: &str) -> Vec<(String, String, String, &'static str)> {
    let client = match kclient() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let url = format!("https://ahmia.fi/search/?q={}", q.replace(' ', "+"));
    let resp = match client.get(&url).send().await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return vec![],
    };
    let mut results = Vec::new();
    for chunk in resp.split("search-result") {
        let url = match chunk.find("href=\"") {
            Some(i) => {
                let s = &chunk[i + 6..];
                match s.find('"') {
                    Some(j) => s[..j].to_string(),
                    None => continue,
                }
            }
            None => continue,
        };
        if !url.contains(".onion") && !url.starts_with("http") {
            continue;
        }
        let title = chunk
            .find("<h4")
            .and_then(|i| chunk[i..].find('>').map(|j| i + j + 1))
            .and_then(|start| {
                chunk[start..]
                    .find("</")
                    .map(|end| html_decode(&chunk[start..start + end]).trim().to_string())
            })
            .unwrap_or_default();
        let snippet = chunk
            .find("<p")
            .and_then(|i| chunk[i..].find('>').map(|j| i + j + 1))
            .and_then(|start| {
                chunk[start..]
                    .find("</p")
                    .map(|end| html_decode(&chunk[start..start + end]).trim().to_string())
            })
            .unwrap_or_default();
        if !title.is_empty() || !snippet.is_empty() {
            results.push((title, url, trunc(&snippet, 200).to_string(), "暗网(Ahmia)"));
        }
    }
    results
}

async fn torch_search_layer(q: &str) -> Vec<(String, String, String, &'static str)> {
    let client = match tor_client(25) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let url = format!(
        "http://xmh57jrknzkhv6y3ls3ubitzfqnkrwxhopf5aygthi7d6rplyvk3noyd.onion/cgi-bin/omega/omega?P={}&DEFAULTOP=and",
        q.replace(' ', "+")
    );
    let resp = match client.get(&url).send().await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return vec![],
    };
    let mut results = Vec::new();
    for chunk in resp.split("<b>") {
        if !chunk.contains(".onion") {
            continue;
        }
        let url = chunk
            .find("href=\"")
            .and_then(|i| {
                let s = &chunk[i + 6..];
                s.find('"').map(|j| s[..j].to_string())
            })
            .unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        let title = match chunk.find("</b>") {
            Some(i) => html_decode(&chunk[..i]).trim().to_string(),
            None => continue,
        };
        results.push((title, url, String::new(), "暗网(Torch)"));
    }
    results.truncate(8);
    results
}

fn parse_ddg_html(html: &str, source: &'static str) -> Vec<(String, String, String, &'static str)> {
    let mut results = Vec::new();
    for chunk in html.split("result__body") {
        let url = chunk
            .find("result__url")
            .and_then(|i| chunk[i..].find("href=\"").map(|j| i + j + 6))
            .and_then(|start| {
                chunk[start..].find('"').map(|end| {
                    let raw = &chunk[start..start + end];
                    if raw.starts_with("//") {
                        format!("https:{raw}")
                    } else {
                        raw.to_string()
                    }
                })
            })
            .unwrap_or_default();
        let title = chunk
            .find("result__a")
            .and_then(|i| chunk[i..].find('>').map(|j| i + j + 1))
            .and_then(|start| {
                chunk[start..]
                    .find("</a")
                    .map(|end| html_decode(&chunk[start..start + end]).trim().to_string())
            })
            .unwrap_or_default();
        let snippet = chunk
            .find("result__snippet")
            .and_then(|i| chunk[i..].find('>').map(|j| i + j + 1))
            .and_then(|start| {
                chunk[start..]
                    .find("</a")
                    .or_else(|| chunk[start..].find("</td"))
                    .map(|end| html_decode(&chunk[start..start + end]).trim().to_string())
            })
            .unwrap_or_default();
        if !url.is_empty() && !title.is_empty() {
            results.push((title, url, trunc(&snippet, 200).to_string(), source));
        }
    }
    results
}

// ── Extra intelligence layers (reach content the surface web hides) ───────

/// True (with the bare host) when the query looks like a domain or URL, so we can
/// fire the domain-oriented OSINT layers (subdomains, deleted-page archives).
fn looks_like_domain(q: &str) -> Option<String> {
    let s = q
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    let host = s.split('/').next().unwrap_or(s);
    if host.is_empty() || host.contains(' ') || !host.contains('.') {
        return None;
    }
    let tld_ok = host
        .rsplit('.')
        .next()
        .is_some_and(|t| t.len() >= 2 && t.chars().all(|c| c.is_ascii_alphabetic()));
    if host.split('.').count() >= 2
        && tld_ok
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        Some(host.to_lowercase())
    } else {
        None
    }
}

/// A DuckDuckGo HTML query with an arbitrary source label — used for dork variants.
async fn ddg_query(q: &str, label: &'static str) -> Vec<(String, String, String, &'static str)> {
    let client = match kclient() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let resp = client
        .post("https://html.duckduckgo.com/html/")
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .form(&[("q", q), ("kl", "wt-wt")])
        .send()
        .await;
    match resp {
        Ok(r) => parse_ddg_html(&r.text().await.unwrap_or_default(), label),
        Err(_) => vec![],
    }
}

/// Dork the surface web for where hidden content actually lives: paste sites, code
/// hosts, and sensitive file types. DuckDuckGo honours `site:` / `filetype:`.
async fn dork_layer(q: &str) -> Vec<(String, String, String, &'static str)> {
    let dorks = [
        format!("{q} site:pastebin.com OR site:ghostbin.com OR site:rentry.co OR site:justpaste.it OR site:controlc.com"),
        format!("{q} site:gist.github.com OR site:github.com OR site:gitlab.com"),
        format!("{q} filetype:log OR filetype:sql OR filetype:env OR filetype:json OR filetype:txt"),
    ];
    let (mut paste, mut code, mut files) = tokio::join!(
        ddg_query(&dorks[0], "定向(dork)"),
        ddg_query(&dorks[1], "定向(dork)"),
        ddg_query(&dorks[2], "定向(dork)"),
    );
    paste.append(&mut code);
    paste.append(&mut files);
    paste
}

/// Wayback CDX: every archived URL under a domain — including pages DELETED from the
/// live web. The archived snapshot is still readable, so this reaches "removed" content.
async fn wayback_layer(domain: &str) -> Vec<(String, String, String, &'static str)> {
    let client = match kclient() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let url = format!(
        "http://web.archive.org/cdx/search/cdx?url={domain}*&output=json&fl=original,timestamp&collapse=urlkey&limit=50"
    );
    let txt = match client.get(&url).send().await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return vec![],
    };
    let rows: Vec<Vec<String>> = serde_json::from_str(&txt).unwrap_or_default();
    let mut out = Vec::new();
    for row in rows.into_iter().skip(1) {
        if row.len() < 2 {
            continue;
        }
        let orig = row[0].clone();
        let ts = row[1].clone();
        out.push((
            orig.clone(),
            format!("https://web.archive.org/web/{ts}/{orig}"),
            format!("存档于 {ts}——原页面即使已被删除，这个快照仍可 web_fetch 读到"),
            "存档(Wayback)",
        ));
    }
    out
}

/// crt.sh certificate transparency: enumerate subdomains / internal hosts of a domain
/// that normal search never surfaces (dev, staging, admin, api-internal, …).
async fn crtsh_layer(domain: &str) -> Vec<(String, String, String, &'static str)> {
    let client = match kclient() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let url = format!("https://crt.sh/?q=%25.{domain}&output=json");
    let txt = match client
        .get(&url)
        .timeout(Duration::from_secs(20))
        .send()
        .await
    {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return vec![],
    };
    let arr: Value = serde_json::from_str(&txt).unwrap_or(Value::Null);
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    if let Some(items) = arr.as_array() {
        for it in items {
            if let Some(nv) = it.get("name_value").and_then(|v| v.as_str()) {
                for host in nv.split('\n') {
                    let h = host.trim().trim_start_matches("*.").to_lowercase();
                    if h.is_empty() || h.contains(' ') || h.contains('*') || seen.contains(&h) {
                        continue;
                    }
                    seen.insert(h.clone());
                    out.push((
                        h.clone(),
                        format!("https://{h}"),
                        "证书透明记录里发现的主机".into(),
                        "子域名(crt.sh)",
                    ));
                    if out.len() >= 40 {
                        return out;
                    }
                }
            }
        }
    }
    out
}

/// Pull every distinct `.onion` link (+ anchor text) out of a dark-web engine's HTML.
/// Parser-agnostic on purpose so it survives the frequent layout churn of onion engines.
fn extract_onion_links(
    html: &str,
    label: &'static str,
) -> Vec<(String, String, String, &'static str)> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for part in html.split("href=\"").skip(1) {
        let end = match part.find('"') {
            Some(e) => e,
            None => continue,
        };
        let href = &part[..end];
        if !href.contains(".onion") || !href.starts_with("http") {
            continue;
        }
        let url = href.to_string();
        if seen.contains(&url) {
            continue;
        }
        seen.insert(url.clone());
        let title = part[end..]
            .find('>')
            .and_then(|g| {
                let s = &part[end + g + 1..];
                s.find("</a")
                    .map(|e2| html_decode(&s[..e2]).trim().to_string())
            })
            .unwrap_or_default();
        out.push((
            if title.is_empty() { url.clone() } else { title },
            url,
            String::new(),
            label,
        ));
        if out.len() >= 12 {
            break;
        }
    }
    out
}

/// Haystak — a 4th dark-web engine (over Tor) on top of DDG-onion / Ahmia / Torch.
async fn haystak_layer(q: &str) -> Vec<(String, String, String, &'static str)> {
    let client = match tor_client(25) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let url = format!(
        "http://haystak5njsmn2hqkewecpaxetahtwhsbsa64jom2k22z5afxhnpxfid.onion/?q={}",
        q.replace(' ', "+")
    );
    match client.get(&url).send().await {
        Ok(r) => extract_onion_links(&r.text().await.unwrap_or_default(), "暗网(Haystak)"),
        Err(_) => vec![],
    }
}

async fn tor_search_layers(q: &str) -> Vec<(String, String, String, &'static str)> {
    // Fast paths must not wait through a full cold Tor bootstrap. Six seconds
    // is enough when Tor is already running or nearly ready; ensure_tor starts
    // the detached process, so a later search can use it if bootstrap is slower.
    match tokio::time::timeout(Duration::from_secs(6), crate::net::ensure_tor()).await {
        Ok(Ok(())) => {}
        _ => return Vec::new(),
    }
    let mut racing: FuturesUnordered<DeepSearchFuture> = FuturesUnordered::new();
    for source in 0..3 {
        let query = q.to_string();
        racing.push(Box::pin(async move {
            match source {
                0 => ddg_onion(&query).await,
                1 => torch_search_layer(&query).await,
                _ => haystak_layer(&query).await,
            }
        }));
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut results = Vec::new();
    while let Ok(Some(mut batch)) = tokio::time::timeout_at(deadline, racing.next()).await {
        results.append(&mut batch);
    }
    results
}

#[tauri::command]
pub async fn deep_search(query: String, max_results: Option<usize>) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("空搜索词".into());
    }
    let limit = max_results.unwrap_or(24).min(60);
    let domain = looks_like_domain(q);

    let mut racing: FuturesUnordered<DeepSearchFuture> = FuturesUnordered::new();
    // Keyword layers run concurrently. Tor-backed engines share one readiness
    // gate so a cold bootstrap cannot hold every public source for 30-40s.
    {
        let q = q.to_string();
        racing.push(Box::pin(async move { ddg_surface(&q).await }));
    }
    {
        let q = q.to_string();
        racing.push(Box::pin(async move { dork_layer(&q).await }));
    }
    {
        let q = q.to_string();
        racing.push(Box::pin(async move { ahmia_search_layer(&q).await }));
    }
    {
        let q = q.to_string();
        racing.push(Box::pin(async move { tor_search_layers(&q).await }));
    }
    // Domain/OSINT layers — only when the query is a domain or URL: hidden subdomains + deleted-page archives.
    if let Some(d) = domain.clone() {
        {
            let d = d.clone();
            racing.push(Box::pin(async move { crtsh_layer(&d).await }));
        }
        {
            let d = d.clone();
            racing.push(Box::pin(async move { wayback_layer(&d).await }));
        }
    }

    let source_tasks = racing.len();
    let mut completed_tasks = 0usize;
    let mut all: Vec<(String, String, String, &str)> = Vec::new();
    let mut seen_urls = std::collections::HashSet::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);

    while let Ok(Some(batch)) = tokio::time::timeout_at(deadline, racing.next()).await {
        completed_tasks += 1;
        for item in batch {
            let norm = item.1.trim_end_matches('/').to_lowercase();
            if seen_urls.contains(&norm) {
                continue;
            }
            seen_urls.insert(norm);
            all.push(item);
        }
    }

    if all.is_empty() {
        return Ok(format!(
            "深层搜索「{q}」未找到结果；本轮完成 {completed_tasks}/{source_tasks} 个来源任务。空结果可能表示无匹配、来源拒绝/超时，或 Tor 尚在冷启动。"
        ));
    }

    let mut out = format!(
        "🔍 深层情报搜索「{q}」— {} 条结果；本轮完成 {completed_tasks}/{source_tasks} 个来源任务（明网+定向dork+存档+子域名+暗网多引擎）：\n",
        all.len().min(limit),
    );
    for (i, (title, url, snippet, source)) in all.iter().take(limit).enumerate() {
        out.push_str(&format!(
            "\n{}. [{}] {}\n   {}\n",
            i + 1,
            source,
            title,
            url,
        ));
        if !snippet.is_empty() {
            out.push_str(&format!("   {}\n", trunc(snippet, 240)));
        }
    }
    out.push_str(
        "\n来源层：明网(DuckDuckGo) + 定向dork(paste/leak/filetype) + 存档(Wayback,含已删除页) + 子域名(crt.sh) + 暗网四引擎(DDG-onion/Ahmia/Torch/Haystak)\n\
         完成来源任务只表示请求已结束，不等于该层返回了结果；空层可能是无匹配、被拒绝、超时或 Tor 尚未就绪。明网/存档 URL 用 web_fetch 读，.onion URL 用 tor_request 读。",
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marketplace_search_urls_percent_encode_cjk_queries() {
        assert_eq!(
            url_query_component("iPhone 16 优惠"),
            "iPhone%2016%20%E4%BC%98%E6%83%A0"
        );
        assert_eq!(
            url_query_component("二手 3080 显卡"),
            "%E4%BA%8C%E6%89%8B%203080%20%E6%98%BE%E5%8D%A1"
        );
    }

    #[test]
    fn developer_source_scopes_are_explicit_and_bounded() {
        let all = select_developer_sources(Some("all"), None).unwrap();
        assert_eq!(all.len(), DEVELOPER_COMMUNITY_SOURCES.len());
        assert!(all.contains(&"github"));
        assert!(all.contains(&"stackoverflow"));
        assert!(all.contains(&"v2ex"));

        let forums = select_developer_sources(Some("forums"), None).unwrap();
        assert!(forums.contains(&"reddit"));
        assert!(forums.contains(&"github_discussions"));
        assert!(forums.contains(&"rust_users"));
        assert!(forums.contains(&"python_discussions"));
        assert!(forums.contains(&"swift_forums"));
        assert!(forums.contains(&"kotlin_discussions"));
        assert!(!forums.contains(&"gitlab"));
    }

    #[test]
    fn developer_source_aliases_deduplicate_and_reject_unknowns() {
        let requested = vec![
            "Stack Overflow".to_string(),
            "so".to_string(),
            "dev.to".to_string(),
            "码云".to_string(),
        ];
        let selected = select_developer_sources(None, Some(&requested)).unwrap();
        assert_eq!(selected, vec!["stackoverflow", "devto", "gitee"]);

        let invalid = vec!["a-community-without-an-adapter".to_string()];
        let error = select_developer_sources(None, Some(&invalid)).unwrap_err();
        assert!(error.contains("Unsupported developer sources"));
    }

    #[test]
    fn official_discourse_aliases_resolve_without_duplicates() {
        let requested = vec![
            "rust".to_string(),
            "rust discourse".to_string(),
            "python".to_string(),
            "Swift Forums".to_string(),
            "kotlin_forum".to_string(),
        ];
        let selected = select_developer_sources(None, Some(&requested)).unwrap();
        assert_eq!(
            selected,
            vec![
                "rust_users",
                "python_discussions",
                "swift_forums",
                "kotlin_discussions"
            ]
        );
    }

    #[test]
    fn github_issue_and_code_fixtures_do_not_turn_creation_into_publication() {
        let issue = serde_json::json!({
            "title": "Fix async cancellation",
            "created_at": "2026-01-02T03:04:05Z",
            "updated_at": "2026-02-03T04:05:06Z",
            "html_url": "https://github.com/example/project/issues/7"
        });
        let issue_output = format_github_search_item(&issue, "issues", 0, "2026-07-12T18:00:00Z");
        assert!(issue_output.contains("published_date: unknown"));
        assert!(issue_output
            .contains("created_date: 2026-01-02T03:04:05Z (provider field: created_at)"));
        assert!(issue_output
            .contains("updated_date: 2026-02-03T04:05:06Z (provider field: updated_at)"));
        assert!(issue_output.contains("last_activity_date: unknown"));
        assert!(!issue_output.contains("published_date: 2026-01-02T03:04:05Z"));

        let code = serde_json::json!({
            "name": "handler.rs",
            "path": "src/handler.rs",
            "html_url": "https://github.com/example/project/blob/main/src/handler.rs"
        });
        let code_output = format_github_search_item(&code, "code", 0, "2026-07-12T18:00:00Z");
        assert!(code_output.contains("published_date: unknown"));
        assert!(code_output.contains("created_date: unknown"));
        assert!(code_output.contains("updated_date: unknown"));
        assert!(code_output.contains("last_activity_date: unknown"));
    }

    #[test]
    fn github_repo_helpers_normalize_actions_and_decode_content() {
        assert_eq!(
            github_repo_ref("vercel", "next.js").unwrap(),
            ("vercel".to_string(), "next.js".to_string())
        );
        assert!(github_repo_ref("vercel/next.js", "").is_err());
        assert_eq!(github_repo_action(Some("read-file".into())), "file");
        assert_eq!(github_repo_action(Some("pull requests".into())), "pulls");
        assert_eq!(repo_reader_action(Some("merge requests".into())), "pulls");
        assert_eq!(
            hosted_repo_ref("gitlab_repo", "group/subgroup", "project", true).unwrap(),
            ("group/subgroup".to_string(), "project".to_string())
        );
        assert!(hosted_repo_ref("gitee_repo", "group/subgroup", "project", false).is_err());
        assert_eq!(
            String::from_utf8(decode_github_base64("SGVsbG8sIHJlcG8h\n").unwrap()).unwrap(),
            "Hello, repo!"
        );
    }

    #[test]
    fn github_repo_overview_keeps_provider_date_semantics() {
        let repo = serde_json::json!({
            "full_name": "example/project",
            "description": "Example project",
            "language": "Rust",
            "stargazers_count": 42,
            "forks_count": 3,
            "open_issues_count": 7,
            "default_branch": "main",
            "license": {"spdx_id": "MIT"},
            "homepage": "https://example.com",
            "html_url": "https://github.com/example/project",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2025-02-02T00:00:00Z",
            "pushed_at": "2026-03-03T00:00:00Z"
        });
        let output = format_github_repo_overview(&repo, "2026-07-12T18:00:00Z");
        assert!(output.contains("GitHub repo overview: example/project"));
        assert!(output.contains("Stars: 42"));
        assert!(output.contains("License: MIT"));
        assert!(output.contains("created_date: 2024-01-01T00:00:00Z (provider field: created_at)"));
        assert!(output.contains("updated_date: 2025-02-02T00:00:00Z (provider field: updated_at)"));
        assert!(
            output.contains("last_activity_date: 2026-03-03T00:00:00Z (provider field: pushed_at)")
        );
    }

    #[test]
    fn hosted_repo_overviews_keep_provider_date_semantics() {
        let gitlab = serde_json::json!({
            "path_with_namespace": "gitlab-org/gitlab",
            "description": "GitLab",
            "topics": ["ruby", "vue"],
            "star_count": 10,
            "forks_count": 2,
            "open_issues_count": 3,
            "default_branch": "master",
            "visibility": "public",
            "web_url": "https://gitlab.com/gitlab-org/gitlab",
            "created_at": "2020-01-01T00:00:00Z",
            "updated_at": "2021-01-01T00:00:00Z",
            "last_activity_at": "2022-01-01T00:00:00Z"
        });
        let gitlab_output = format_gitlab_repo_overview(&gitlab, "2026-07-12T18:00:00Z");
        assert!(gitlab_output.contains("GitLab repo overview: gitlab-org/gitlab"));
        assert!(gitlab_output.contains(
            "last_activity_date: 2022-01-01T00:00:00Z (provider field: last_activity_at)"
        ));

        let gitee = serde_json::json!({
            "full_name": "oschina/git-osc",
            "description": "Gitee Feedback",
            "language": "Ruby",
            "stargazers_count": 9,
            "forks_count": 4,
            "open_issues_count": 5,
            "default_branch": "master",
            "html_url": "https://gitee.com/oschina/git-osc",
            "created_at": "2020-01-01T00:00:00Z",
            "updated_at": "2021-01-01T00:00:00Z",
            "pushed_at": "2022-01-01T00:00:00Z"
        });
        let gitee_output = format_gitee_repo_overview(&gitee, "2026-07-12T18:00:00Z");
        assert!(gitee_output.contains("Gitee repo overview: oschina/git-osc"));
        assert!(gitee_output
            .contains("last_activity_date: 2022-01-01T00:00:00Z (provider field: pushed_at)"));

        let codeberg = serde_json::json!({
            "full_name": "forgejo/forgejo",
            "description": "Forgejo",
            "language": "Go",
            "stars_count": 11,
            "forks_count": 6,
            "open_issues_count": 7,
            "default_branch": "forgejo",
            "html_url": "https://codeberg.org/forgejo/forgejo",
            "created_at": "2020-01-01T00:00:00Z",
            "updated_at": "2021-01-01T00:00:00Z"
        });
        let codeberg_output = format_codeberg_repo_overview(&codeberg, "2026-07-12T18:00:00Z");
        assert!(codeberg_output.contains("Codeberg repo overview: forgejo/forgejo"));
        assert!(codeberg_output.contains("last_activity_date: unknown (Codeberg repository response did not expose a last-activity field)"));
    }

    #[test]
    fn provider_date_lines_only_use_explicit_fields() {
        let output = provider_date_lines(
            Some(("2026-02-20T15:10:00.015Z", "publishedAt")),
            Some(("2020-03-20", "created_at")),
            None,
            Some(("2024-08-09", "pushed_at")),
            "2026-07-12T18:00:00Z",
        );

        assert!(output
            .contains("published_date: 2026-02-20T15:10:00.015Z (provider field: publishedAt)"));
        assert!(output.contains("created_date: 2020-03-20 (provider field: created_at)"));
        assert!(output.contains("updated_date: unknown"));
        assert!(output.contains("last_activity_date: 2024-08-09 (provider field: pushed_at)"));
        assert!(output.contains("retrieved_at: 2026-07-12T18:00:00Z"));
        for field in [
            "published_date:",
            "created_date:",
            "updated_date:",
            "last_activity_date:",
            "retrieved_at:",
        ] {
            assert_eq!(output.matches(field).count(), 1, "{field}: {output}");
        }
    }

    #[test]
    fn freecodecamp_publication_uses_provider_field_and_timestamp_fallback() {
        let explicit = serde_json::json!({
            "publishedAt": "2026-02-20T15:10:00.015Z",
            "publishedAtTimestamp": 1
        });
        assert_eq!(
            freecodecamp_published_date(&explicit),
            Some(("2026-02-20T15:10:00.015Z".to_string(), "publishedAt"))
        );

        let timestamp_only = serde_json::json!({ "publishedAtTimestamp": 1_771_600_200 });
        assert_eq!(
            freecodecamp_published_date(&timestamp_only),
            Some(("2026-02-20T15:10:00Z".to_string(), "publishedAtTimestamp"))
        );
        assert_eq!(freecodecamp_published_date(&serde_json::json!({})), None);
    }

    #[test]
    fn provider_http_statuses_and_trending_empty_status_are_explicit() {
        for provider in ["InfoQ", "freeCodeCamp", "Best of JS", "Sourcegraph"] {
            assert_eq!(
                ensure_provider_http_success(provider, reqwest::StatusCode::OK),
                Ok(())
            );

            let rate_limited =
                ensure_provider_http_success(provider, reqwest::StatusCode::TOO_MANY_REQUESTS)
                    .expect_err("HTTP 429 must not be parsed as an empty provider response");
            assert_eq!(rate_limited, format!("{provider} rate-limited (HTTP 429)"));
            assert_eq!(
                community_result_status(&CommunitySearchOutcome::Finished(Err(rate_limited))),
                CommunitySourceStatus::RateLimited
            );

            let failed =
                ensure_provider_http_success(provider, reqwest::StatusCode::SERVICE_UNAVAILABLE)
                    .expect_err("non-success provider responses must fail");
            assert_eq!(
                failed,
                format!("{provider} returned HTTP 503 Service Unavailable")
            );
            assert_eq!(
                community_result_status(&CommunitySearchOutcome::Finished(Err(failed))),
                CommunitySourceStatus::Failed
            );
        }

        assert!(GITHUB_TRENDING_EMPTY_NOTICE.contains("search_status: empty"));
        assert_eq!(
            community_result_status(&CommunitySearchOutcome::Finished(Ok(
                GITHUB_TRENDING_EMPTY_NOTICE.to_string()
            ))),
            CommunitySourceStatus::Empty
        );
    }

    #[test]
    fn sourcegraph_graphql_fixture_rejects_errors_and_missing_result_count() {
        let graphql_error = serde_json::json!({
            "errors": [{ "message": "query syntax rejected" }],
            "data": {
                "search": { "results": { "resultCount": 0, "results": [] } }
            }
        });
        let error = sourcegraph_graphql_results(&graphql_error)
            .expect_err("GraphQL errors must fail even when HTTP status was successful");
        assert_eq!(
            error,
            "Sourcegraph GraphQL returned errors: query syntax rejected"
        );
        assert_eq!(
            community_result_status(&CommunitySearchOutcome::Finished(Err(error))),
            CommunitySourceStatus::Failed
        );

        let missing_count = serde_json::json!({
            "data": { "search": { "results": { "results": [] } } }
        });
        let error = sourcegraph_graphql_results(&missing_count)
            .expect_err("missing resultCount must not become a genuine empty result");
        assert_eq!(
            error,
            "Sourcegraph GraphQL response missing numeric resultCount"
        );
        assert_eq!(
            community_result_status(&CommunitySearchOutcome::Finished(Err(error))),
            CommunitySourceStatus::Failed
        );

        let genuine_empty = serde_json::json!({
            "data": {
                "search": { "results": { "resultCount": 0, "results": [] } }
            }
        });
        let (count, items) = sourcegraph_graphql_results(&genuine_empty).unwrap();
        assert_eq!(count, 0);
        assert!(items.is_empty());
    }

    #[test]
    fn repository_fixtures_keep_gitee_and_codeberg_dates_distinct() {
        let gitee = serde_json::json!({
            "full_name": "example/project",
            "description": "Example",
            "stargazers_count": 5,
            "forks_count": 2,
            "language": "Rust",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2026-01-02T00:00:00Z",
            "pushed_at": "2026-01-03T00:00:00Z",
            "html_url": "https://gitee.com/example/project"
        });
        let gitee_output = format_gitee_repository_item(&gitee, 0, "2026-07-12T18:00:00Z");
        assert!(gitee_output.contains("published_date: unknown"));
        assert!(gitee_output
            .contains("created_date: 2024-01-01T00:00:00Z (provider field: created_at)"));
        assert!(gitee_output
            .contains("updated_date: 2026-01-02T00:00:00Z (provider field: updated_at)"));
        assert!(gitee_output
            .contains("last_activity_date: 2026-01-03T00:00:00Z (provider field: pushed_at)"));
        assert!(gitee_output.contains("retrieved_at: 2026-07-12T18:00:00Z"));

        let codeberg = serde_json::json!({
            "full_name": "example/project",
            "created_at": "2025-02-01T02:24:04+01:00",
            "updated_at": "2026-03-19T03:46:53+01:00",
            "html_url": "https://codeberg.org/example/project"
        });
        let codeberg_output = format_codeberg_repository_item(&codeberg, 0, "2026-07-12T18:00:00Z");
        assert!(codeberg_output.contains("published_date: unknown"));
        assert!(codeberg_output
            .contains("created_date: 2025-02-01T02:24:04+01:00 (provider field: created_at)"));
        assert!(codeberg_output
            .contains("updated_date: 2026-03-19T03:46:53+01:00 (provider field: updated_at)"));
        assert!(codeberg_output.contains("last_activity_date: unknown"));
        assert!(codeberg_output.contains("retrieved_at: 2026-07-12T18:00:00Z"));
    }

    #[test]
    fn v2ex_language_anchor_filters_obviously_unrelated_hits() {
        let go_hit = serde_json::json!({
            "title": "Error handling in Go",
            "content": "A Go 2 proposal about errors"
        });
        let rust_hit = serde_json::json!({
            "title": "Async error handling in Rust",
            "content": "Using Tokio and Result"
        });
        assert!(!v2ex_hit_matches_language_anchor(
            "rust async error handling",
            &go_hit
        ));
        assert!(v2ex_hit_matches_language_anchor(
            "rust async error handling",
            &rust_hit
        ));
        assert!(v2ex_hit_matches_language_anchor(
            "async error handling",
            &go_hit
        ));
    }

    #[test]
    fn discourse_fixture_keeps_date_semantics_distinct() {
        let fixture = serde_json::json!({
            "posts": [{
                "id": 11,
                "username": "alice",
                "created_at": "2026-05-01T10:00:00.000Z",
                "blurb": "A real <b>async</b> answer",
                "post_number": 2,
                "topic_id": 7
            }],
            "topics": [{
                "id": 7,
                "title": "Async error handling",
                "slug": "async-error-handling",
                "posts_count": 3,
                "last_posted_at": "2026-06-02T12:30:00.000Z"
            }],
            "grouped_search_result": { "error": null }
        });
        let output = parse_discourse_search(
            &fixture,
            RUST_USERS_DISCOURSE,
            "async error",
            5,
            "2026-07-12T18:00:00Z",
        )
        .unwrap();

        assert!(output.contains("published_date: 2026-05-01T10:00:00.000Z"));
        assert!(output.contains("updated_date: unknown"));
        assert!(output.contains("last_activity_date: 2026-06-02T12:30:00.000Z"));
        assert!(output.contains("retrieved_at: 2026-07-12T18:00:00Z"));
        assert!(output.contains("replies: 2"));
        assert!(!output.contains("updated_date: 2026-06-02T12:30:00.000Z"));
        assert!(!output.contains("published_date: 2026-06-02T12:30:00.000Z"));
    }

    #[test]
    fn discourse_fixture_preserves_provider_updated_date() {
        let fixture = serde_json::json!({
            "posts": [{
                "username": "bob",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-03T00:00:00Z",
                "post_number": 1,
                "topic_id": 9
            }],
            "topics": [{
                "id": 9,
                "title": "Structured concurrency",
                "slug": "structured-concurrency",
                "reply_count": 4,
                "last_posted_at": "2026-02-01T00:00:00Z"
            }],
            "grouped_search_result": { "error": null }
        });
        let output = parse_discourse_search(
            &fixture,
            SWIFT_DISCOURSE,
            "concurrency",
            1,
            "2026-07-12T18:00:00Z",
        )
        .unwrap();

        assert!(output.contains("published_date: 2026-01-01T00:00:00Z"));
        assert!(output.contains("updated_date: 2026-01-03T00:00:00Z"));
        assert!(output.contains("last_activity_date: 2026-02-01T00:00:00Z"));
    }

    #[test]
    fn discourse_empty_fixture_and_aggregate_statuses_are_explicit() {
        let fixture = serde_json::json!({
            "posts": [],
            "topics": [],
            "grouped_search_result": { "error": null }
        });
        let output = parse_discourse_search(
            &fixture,
            PYTHON_DISCOURSE,
            "no-match",
            3,
            "2026-07-12T18:00:00Z",
        )
        .unwrap();
        assert!(output.contains("search_status: empty"));
        assert_eq!(
            community_result_status(&CommunitySearchOutcome::Finished(Ok(output))),
            CommunitySourceStatus::Empty
        );
        assert_eq!(
            community_result_status(&CommunitySearchOutcome::Finished(Err(
                "HTTP 429 rate limit".into()
            ))),
            CommunitySourceStatus::RateLimited
        );
        assert_eq!(
            community_result_status(&CommunitySearchOutcome::Finished(Err("HTTP 503".into()))),
            CommunitySourceStatus::Failed
        );
        assert_eq!(
            community_result_status(&CommunitySearchOutcome::Finished(Ok(
                "one real result".into()
            ))),
            CommunitySourceStatus::Success
        );
    }

    #[tokio::test]
    async fn community_source_timeout_is_explicit_and_cancels_pending_work() {
        let adapter: CommunityAdapterFuture = Box::pin(std::future::pending());
        let outcome =
            community_source_with_timeout("slow", "Slow source", adapter, Duration::from_millis(5))
                .await;

        assert_eq!(outcome.0, "slow");
        assert_eq!(outcome.1, "Slow source");
        assert_eq!(
            community_result_status(&outcome.2),
            CommunitySourceStatus::Timeout
        );
        assert!(matches!(
            outcome.2,
            CommunitySearchOutcome::TimedOut { after }
                if after == Duration::from_millis(5)
        ));
    }

    #[tokio::test]
    async fn community_timeout_summary_preserves_fast_results_and_is_bounded() {
        let fast: CommunityAdapterFuture =
            Box::pin(async { ("fast", "Fast source", Ok("one verified result".to_string())) });
        let stalled: CommunityAdapterFuture = Box::pin(std::future::pending());
        let mut pending: FuturesUnordered<CommunitySearchFuture> = FuturesUnordered::new();
        pending.push(Box::pin(community_source_with_timeout(
            "fast",
            "Fast source",
            fast,
            Duration::from_millis(10),
        )));
        pending.push(Box::pin(community_source_with_timeout(
            "stalled",
            "Stalled source",
            stalled,
            Duration::from_millis(10),
        )));

        let responses = tokio::time::timeout(Duration::from_secs(1), async move {
            let mut responses = Vec::new();
            while let Some((key, label, outcome)) = pending.next().await {
                responses.push((key, label, outcome, "2026-07-12T18:00:00Z".to_string()));
            }
            responses
        })
        .await
        .expect("concurrent per-source deadlines must bound the aggregate");

        let output = format_developer_community_results(
            "test query",
            &["fast", "stalled"],
            responses,
            "2026-07-12T18:00:01Z",
        );
        assert!(output.contains("Requested sources: 2; completed searches: 1; failed requests: 1"));
        assert!(output
            .contains("Status counts: success=1; empty=0; rate-limited=0; failed=0; timeout=1"));
        assert!(output.contains(
            "The five source statuses are success, empty, rate-limited, failed, and timeout"
        ));
        assert!(output.contains("independent 12000 ms hard deadline"));
        assert!(output.contains("## Fast source [search completed; status=success]"));
        assert!(output.contains("one verified result"));
        assert!(output.contains("## Stalled source [failed; status=timeout]"));
        assert!(output.contains("search_status: timeout"));
        assert!(output.contains("timeout_ms: 10"));
    }

    #[test]
    fn aggregate_trending_only_accepts_single_known_languages() {
        assert_eq!(aggregate_trending_language("Rust"), Some("rust"));
        assert_eq!(aggregate_trending_language("golang"), Some("go"));
        assert_eq!(aggregate_trending_language("c#"), Some("c#"));
        assert_eq!(aggregate_trending_language("rust async errors"), None);
        assert_eq!(
            aggregate_trending_language("an-unrecognized-language"),
            None
        );

        assert_eq!(
            github_trending_url("c#").unwrap(),
            "https://github.com/trending/c%23?since=weekly"
        );
        assert_eq!(
            github_trending_url("c++").unwrap(),
            "https://github.com/trending/c++?since=weekly"
        );
        assert_eq!(
            github_trending_url("all").unwrap(),
            "https://github.com/trending?since=weekly"
        );
    }

    #[tokio::test]
    #[ignore = "calls live third-party developer communities"]
    async fn developer_community_live_smoke_lists_every_supported_source() {
        let started = std::time::Instant::now();
        let out = developer_community_search(
            "rust async error handling".into(),
            Some("all".into()),
            None,
            Some(1),
        )
        .await
        .unwrap();
        println!("{out}");
        let aggregate_header = out.split("\n## ").next().unwrap_or_default();
        assert!(aggregate_header.contains("\nretrieved_at: "));
        assert!(aggregate_header.contains("published_date, created_date, updated_date"));
        assert!(out.contains(&format!(
            "Requested sources: {}",
            DEVELOPER_COMMUNITY_SOURCES.len()
        )));
        assert_eq!(
            out.matches("\n## ").count(),
            DEVELOPER_COMMUNITY_SOURCES.len()
        );
        assert!(
            started.elapsed() < COMMUNITY_SOURCE_TIMEOUT + Duration::from_secs(3),
            "all-source aggregation exceeded its per-source deadline envelope: {:?}",
            started.elapsed(),
        );
    }

    #[tokio::test]
    #[ignore = "calls four live official Discourse developer forums"]
    async fn official_discourse_live_smoke_has_results_and_truthful_dates() {
        let sources = [
            ("rust_users", "Rust Users Forum"),
            ("python_discussions", "Python Discussions"),
            ("swift_forums", "Swift Forums"),
            ("kotlin_discussions", "Kotlin Discussions"),
        ];
        let out = developer_community_search(
            "rust async error handling".into(),
            None,
            Some(sources.iter().map(|(key, _)| (*key).to_string()).collect()),
            Some(1),
        )
        .await
        .unwrap();
        println!("{out}");
        assert!(out.starts_with("Developer community search\n"));
        assert!(!out.starts_with("Developer community live search\n"));
        assert!(out.contains("source content may come from provider indexes or caches"));

        for (key, label) in sources {
            let marker = format!("\n## {label} [");
            let section = out
                .split(&marker)
                .nth(1)
                .unwrap_or_else(|| panic!("missing aggregate section for {label}"))
                .split("\n## ")
                .next()
                .unwrap_or_default();
            assert!(section.contains("status=success"), "{label}: {section}");
            assert!(
                section.contains(&format!("source: {key}")),
                "{label}: {section}"
            );
            for field in ["published_date:", "last_activity_date:", "retrieved_at:"] {
                assert!(
                    section.contains(field),
                    "{label} missing {field}: {section}"
                );
            }
        }
    }

    #[tokio::test]
    #[ignore = "calls live surface, archive, certificate, and Tor search sources"]
    async fn deep_search_live_smoke_is_bounded_and_reports_coverage() {
        let started = std::time::Instant::now();
        let out = deep_search("rust async error handling".into(), Some(5))
            .await
            .unwrap();
        println!("{out}");
        assert!(out.contains("本轮完成"));
        assert!(started.elapsed() < Duration::from_secs(20));
    }
}
