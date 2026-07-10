use futures_util::stream::{FuturesUnordered, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

type DeepSearchHit = (String, String, String, &'static str);
type DeepSearchFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Vec<DeepSearchHit>> + Send>>;
type CommunitySearchOutput = (&'static str, &'static str, Result<String, String>);
type CommunitySearchFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = CommunitySearchOutput> + Send>>;

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
    let hits = ddg_surface(&search_query).await;
    let mut out = format!(
        "{label} pages for '{query}' (via public web search, not an official {label} API):\n\n"
    );
    let mut count = 0usize;
    for (title, url, snippet, _) in hits {
        if !url.contains(site) || required_path.is_some_and(|path| !url.contains(path)) {
            continue;
        }
        count += 1;
        out.push_str(&format!(
            "{}. {}\n   {}\n   {}\n\n",
            count,
            title,
            trunc(&snippet, 180),
            url,
        ));
        if count >= max_results as usize {
            break;
        }
    }
    if count == 0 {
        out.push_str(
            "No matching page was returned. This can mean no match or that the public search endpoint was unavailable.\n\n",
        );
    }
    out.push_str(&format!("Direct search: {direct_search_url}\n"));
    Ok(out)
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
    let resp = c
        .get("https://registry.npmjs.org/-/v1/search")
        .query(&[("text", q), ("size", &limit.to_string())])
        .send()
        .await
        .map_err(|e| format!("npm: {e}"))?;
    let json: Value = resp.json().await.map_err(|e| format!("npm JSON: {e}"))?;
    let mut out = String::from("npm packages:\n\n");
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
        .query(&[
            ("q", query.as_str()),
            ("per_page", &limit.to_string()),
            ("sort", "stars"),
        ])
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
    let mut out = format!("GitHub {stype}: {total} results\n\n");

    if let Some(items) = json["items"].as_array() {
        for (i, item) in items.iter().enumerate() {
            if stype == "repositories" {
                out.push_str(&format!(
                    "{}. {} ({}★)\n   {}\n   Language: {} | Forks: {} | Updated: {}\n   {}\n\n",
                    i + 1,
                    item["full_name"].as_str().unwrap_or("?"),
                    item["stargazers_count"].as_u64().unwrap_or(0),
                    item["description"].as_str().unwrap_or(""),
                    item["language"].as_str().unwrap_or("?"),
                    item["forks_count"].as_u64().unwrap_or(0),
                    &item["updated_at"]
                        .as_str()
                        .unwrap_or("")
                        .get(..10)
                        .unwrap_or(""),
                    item["html_url"].as_str().unwrap_or(""),
                ));
            } else {
                out.push_str(&format!(
                    "{}. {}\n   {}\n\n",
                    i + 1,
                    item["full_name"]
                        .as_str()
                        .or(item["name"].as_str())
                        .unwrap_or("?"),
                    item["html_url"].as_str().unwrap_or(""),
                ));
            }
        }
    }
    Ok(out)
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
    let mut out = String::from("Stack Overflow results:\n\n");

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

            out.push_str(&format!(
                "{}. {} {}\n   Score: {} | Answers: {} | Views: {} | Tags: [{}]\n   {}\n\n",
                i + 1,
                if answered { "✅" } else { "❓" },
                title,
                score,
                answers,
                views,
                tags.join(", "),
                link,
            ));
        }
        if items.is_empty() {
            out.push_str("(no results)\n");
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

    let json: Value = resp.json().await.map_err(|e| format!("HN JSON: {e}"))?;
    let total = json["nbHits"].as_u64().unwrap_or(0);
    let mut out = format!("Hacker News: {total} results\n\n");

    if let Some(hits) = json["hits"].as_array() {
        for (i, h) in hits.iter().enumerate() {
            let title = h["title"].as_str().unwrap_or("?");
            let points = h["points"].as_u64().unwrap_or(0);
            let comments = h["num_comments"].as_u64().unwrap_or(0);
            let author = h["author"].as_str().unwrap_or("?");
            let date = h["created_at"]
                .as_str()
                .and_then(|s| s.get(..10))
                .unwrap_or("?");
            let url = h["url"].as_str().unwrap_or("");
            let hn_id = h["objectID"].as_str().unwrap_or("");

            out.push_str(&format!(
                "{}. {} ({}pts, {}comments)\n   By: {} | Date: {}\n   {}\n   HN: https://news.ycombinator.com/item?id={}\n\n",
                i + 1, title, points, comments, author, date, url, hn_id,
            ));
        }
    }
    Ok(out)
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
        let q = query.clone();
        let future: CommunitySearchFuture = match *source {
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
                (
                    "github_trending",
                    "GitHub Trending",
                    github_trending(q, Some(limit)).await,
                )
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
        pending.push(future);
    }

    let mut responses = Vec::with_capacity(selected.len());
    while let Some(response) = pending.next().await {
        responses.push(response);
    }
    responses.sort_by_key(|(key, _, _)| {
        selected
            .iter()
            .position(|selected_key| selected_key == key)
            .unwrap_or(usize::MAX)
    });

    let completed = responses
        .iter()
        .filter(|(_, _, result)| result.is_ok())
        .count();
    let failed = responses.len().saturating_sub(completed);
    let mut out = format!(
        "Developer community live search\nQuery: {query}\nRequested sources: {}; completed searches: {completed}; failed requests: {failed}.\n\
         A completed search may use a source API or a clearly labelled public site search, and may return no matches. It does not make community posts verified facts; inspect original pages and cross-check important claims.\n",
        responses.len()
    );

    for (_, label, result) in responses {
        match result {
            Ok(content) => out.push_str(&format!(
                "\n## {label} [search completed]\n{}\n",
                trunc(&content, 900)
            )),
            Err(error) => out.push_str(&format!("\n## {label} [failed]\n{}\n", trunc(&error, 500))),
        }
    }
    Ok(out)
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
            ("order_by", &"stars_count".to_string()),
            ("sort", &"desc".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("GitLab: {e}"))?;
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("GitLab parse: {e}"))?;
    let arr = data.as_array().ok_or("GitLab: unexpected response")?;
    if arr.is_empty() {
        return Ok(format!("No GitLab projects found for '{query}'"));
    }
    let mut out = format!("GitLab projects for '{query}':\n\n");
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
        let updated = r["last_activity_at"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. {}\n   {}\n   Stars: {} | Forks: {} | Topics: {}\n   Updated: {}\n   {}\n\n",
            i + 1,
            name,
            trunc(desc, 150),
            stars,
            forks,
            if lang.is_empty() { "-" } else { &lang },
            &updated[..updated.len().min(10)],
            url
        ));
    }
    Ok(out)
}

// ── Gitee ─────────────────────────────────────────────────────────

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
    let data: Value = resp.json().await.map_err(|e| format!("Gitee parse: {e}"))?;
    let arr = data.as_array().ok_or("Gitee: unexpected response")?;
    if arr.is_empty() {
        return Ok(format!("No Gitee repos found for '{query}'"));
    }
    let mut out = format!("Gitee repos for '{query}':\n\n");
    for (i, r) in arr.iter().enumerate() {
        let name = r["full_name"]
            .as_str()
            .unwrap_or(r["name"].as_str().unwrap_or("?"));
        let desc = r["description"].as_str().unwrap_or("");
        let stars = r["stargazers_count"].as_u64().unwrap_or(0);
        let forks = r["forks_count"].as_u64().unwrap_or(0);
        let lang = r["language"].as_str().unwrap_or("-");
        let url = r["html_url"].as_str().unwrap_or("");
        let updated = r["updated_at"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. {}\n   {}\n   Stars: {} | Forks: {} | Lang: {}\n   Updated: {}\n   {}\n\n",
            i + 1,
            name,
            trunc(desc, 150),
            stars,
            forks,
            lang,
            &updated[..updated.len().min(10)],
            url
        ));
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
    let mut out = format!("Reddit results for '{query}':\n\n");
    for (i, p) in posts.iter().enumerate() {
        let d = &p["data"];
        let title = d["title"].as_str().unwrap_or("?");
        let sub = d["subreddit"].as_str().unwrap_or("?");
        let score = d["score"].as_i64().unwrap_or(0);
        let comments = d["num_comments"].as_u64().unwrap_or(0);
        let author = d["author"].as_str().unwrap_or("?");
        let selftext = d["selftext"].as_str().unwrap_or("");
        let permalink = d["permalink"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. [r/{}] {}\n   by u/{} | ⬆️{} | 💬{}\n   {}\n   https://reddit.com{}\n\n",
            i + 1,
            sub,
            title,
            author,
            score,
            comments,
            trunc(selftext, 150),
            permalink
        ));
    }
    Ok(out)
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
    let mut out = String::from("掘金 (Juejin) articles:\n\n");
    if let Some(data) = json["data"].as_array() {
        for (i, item) in data.iter().take(limit as usize).enumerate() {
            let rm = &item["result_model"];
            let info = &rm["article_info"];
            let author = rm["author_user_info"]["user_name"].as_str().unwrap_or("?");
            let title = info["title"].as_str().unwrap_or("?");
            let brief = info["brief_content"].as_str().unwrap_or("");
            let aid = info["article_id"].as_str().unwrap_or("");
            let views = info["view_count"].as_u64().unwrap_or(0);
            let likes = info["digg_count"].as_u64().unwrap_or(0);
            out.push_str(&format!(
                "{}. {} (by {})\n   {}\n   Views: {} | Likes: {} | https://juejin.cn/post/{}\n\n",
                i + 1,
                title,
                author,
                trunc(brief, 200),
                views,
                likes,
                aid,
            ));
        }
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
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = c
        .get("https://dev.to/api/articles")
        .query(&[
            ("per_page", n.to_string()),
            ("tag", "codepen".into()),
            ("top", "365".into()),
        ])
        .send()
        .await;
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();
    let mut out = format!("CodePen-related articles & pens for '{query}':\n\n");
    let mut count = 0;
    if let Ok(r) = resp {
        if let Ok(items) = r.json::<Vec<Value>>().await {
            for a in &items {
                let title = a["title"].as_str().unwrap_or("");
                let desc = a["description"].as_str().unwrap_or("");
                let haystack = format!("{} {}", title, desc).to_lowercase();
                if !keywords.is_empty() && !keywords.iter().any(|k| haystack.contains(k)) {
                    continue;
                }
                count += 1;
                let url = a["url"].as_str().unwrap_or("");
                let user = a["user"]["username"].as_str().unwrap_or("?");
                out.push_str(&format!(
                    "{}. {} (by @{})\n   {}\n   {}\n\n",
                    count,
                    title,
                    user,
                    trunc(desc, 150),
                    url,
                ));
                if count >= n as usize {
                    break;
                }
            }
        }
    }
    out.push_str(&format!(
        "Direct CodePen search: https://codepen.io/search/pens?q={}\nTip: use web_search with 'site:codepen.io {}' for more results, then web_fetch any pen URL to study its code.\n",
        query.replace(' ', "+"), query,
    ));
    Ok(out)
}

// ── Dribbble (professional UI design inspiration) ───────────────────

#[tauri::command]
pub async fn dribbble_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20);
    let resp = c
        .get("https://dev.to/api/articles")
        .query(&[
            ("per_page", n.to_string()),
            ("tag", "design".into()),
            ("top", "365".into()),
        ])
        .send()
        .await;
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();
    let mut out = format!("UI design articles & inspiration for '{query}':\n\n");
    let mut count = 0;
    if let Ok(r) = resp {
        if let Ok(items) = r.json::<Vec<Value>>().await {
            for a in &items {
                let title = a["title"].as_str().unwrap_or("");
                let desc = a["description"].as_str().unwrap_or("");
                let tags = a["tag_list"]
                    .as_array()
                    .map(|t| {
                        t.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let haystack = format!("{} {} {}", title, desc, tags).to_lowercase();
                if !keywords.is_empty() && !keywords.iter().any(|k| haystack.contains(k)) {
                    continue;
                }
                count += 1;
                let url = a["url"].as_str().unwrap_or("");
                let user = a["user"]["username"].as_str().unwrap_or("?");
                out.push_str(&format!(
                    "{}. {} (by @{})\n   {}\n   {}\n\n",
                    count,
                    title,
                    user,
                    trunc(desc, 150),
                    url,
                ));
                if count >= n as usize {
                    break;
                }
            }
        }
    }
    out.push_str(&format!(
        "Dribbble search: https://dribbble.com/search/{}\nTip: use web_search with 'site:dribbble.com {}' for Dribbble shots, or 'UI design {}' for broader inspiration.\n",
        query.replace(' ', "-"), query, query,
    ));
    Ok(out)
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
    let data: Value = resp.json().await.map_err(|e| format!("V2EX parse: {e}"))?;
    let total = data["total"].as_u64().unwrap_or(0);
    let hits = data["hits"].as_array();
    if total == 0 || hits.is_none() {
        return Ok(format!("No V2EX discussions found for '{query}'"));
    }
    let mut out = format!("V2EX discussions for '{}' ({} total):\n\n", query, total);
    for (i, hit) in hits.unwrap().iter().take(n as usize).enumerate() {
        let src = &hit["_source"];
        let id = src["id"].as_u64().unwrap_or(0);
        let title = strip_html(src["title"].as_str().unwrap_or("?"));
        let member = src["member"].as_str().unwrap_or("?");
        let replies = src["replies"].as_u64().unwrap_or(0);
        let created = src["created"].as_str().unwrap_or("");
        let content = src["content"].as_str().unwrap_or("");
        out.push_str(&format!(
            "{}. {} (by @{}, {} replies)\n   {}\n   {} | https://www.v2ex.com/t/{}\n\n",
            i + 1,
            title,
            member,
            replies,
            trunc(content, 200),
            &created[..created.len().min(10)],
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
        .query(&[("q", query.as_str()), ("type", "article"), ("page", "1")])
        .send()
        .await
        .map_err(|e| format!("SegmentFault: {e}"))?;
    let data: Value = resp
        .json()
        .await
        .map_err(|e| format!("SegmentFault parse: {e}"))?;
    let rows = data["data"]["rows"].as_array();
    if rows.is_none() || rows.unwrap().is_empty() {
        return Ok(format!("No SegmentFault results found for '{query}'"));
    }
    let mut out = format!("SegmentFault (思否) results for '{}':\n\n", query);
    for (i, row) in rows.unwrap().iter().take(n).enumerate() {
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
        out.push_str(&format!(
            "{}. [{}] {} (by @{}, votes: {})\n   {}\n   https://segmentfault.com{}\n\n",
            i + 1,
            type_label,
            title,
            user,
            votes,
            trunc(excerpt, 200),
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
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let mut out = format!("FreeCodeCamp articles for '{query}':\n\n");
    if let Some(hits) = data["hits"].as_array() {
        for (i, h) in hits.iter().enumerate() {
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
            out.push_str(&format!(
                "{}. {} (by {}){}\n   {}\n\n",
                i + 1,
                title,
                author,
                tag_str,
                url
            ));
        }
    }
    if out.ends_with(":\n\n") {
        out.push_str("  No results found.\n");
    }
    Ok(out)
}

#[tauri::command]
pub async fn github_trending(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(15).min(25) as usize;
    let lang = query.trim();
    let url = if lang.is_empty() || lang == "all" {
        "https://github.com/trending?since=weekly".to_string()
    } else {
        format!(
            "https://github.com/trending/{}?since=weekly",
            lang.to_lowercase().replace(' ', "-")
        )
    };
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
    let mut out = format!(
        "GitHub Trending repos (weekly, {}):\n\n",
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
        out.push_str(&format!("   https://github.com/{}\n\n", repo));
    }
    if count == 0 {
        out.push_str("  No trending repos found for this language.\n");
    }
    out.push_str(&format!("Browse: {}\n", url));
    Ok(out)
}

#[tauri::command]
pub async fn infoq_search(query: String, max_results: Option<u32>) -> Result<String, String> {
    let c = kclient()?;
    let n = max_results.unwrap_or(10).min(20) as usize;
    let html = c
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
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let mut out = format!("InfoQ articles for '{query}':\n\n");
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
                "{}. {}\n   https://www.infoq.com{}\n\n",
                count, title, path
            ));
        }
    }
    if count == 0 {
        out.push_str("  No results found.\n");
    }
    out.push_str(&format!(
        "InfoQ search: https://www.infoq.com/search/?q={}\n",
        query.replace(' ', "+"),
    ));
    Ok(out)
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
    let hits = ddg_surface(&search_query).await;
    let mut out = format!(
        "HackerNoon pages for '{query}' (via public web search, not a HackerNoon API):\n\n"
    );
    let mut count = 0usize;
    for (title, url, snippet, _) in hits {
        if !url.contains("hackernoon.com") {
            continue;
        }
        count += 1;
        out.push_str(&format!(
            "{}. {}\n   {}\n   {}\n\n",
            count,
            title,
            trunc(&snippet, 180),
            url,
        ));
        if count >= n as usize {
            break;
        }
    }
    if count == 0 {
        out.push_str(
            "No matching page was returned. This can mean no match or that the public search endpoint was unavailable.\n\n",
        );
    }
    out.push_str(&format!(
        "HackerNoon search page: https://hackernoon.com/search?query={}\n",
        query.replace(' ', "+"),
    ));
    Ok(out)
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
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let mut out = format!("Codeberg repos for '{query}':\n\n");
    if let Some(repos) = data["data"].as_array() {
        for (i, r) in repos.iter().enumerate() {
            let name = r["full_name"].as_str().unwrap_or("?");
            let desc = r["description"].as_str().unwrap_or("");
            let stars = r["stars_count"].as_u64().unwrap_or(0);
            let lang = r["language"].as_str().unwrap_or("");
            let url = r["html_url"].as_str().unwrap_or("");
            let topics: Vec<&str> = r["topics"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let lang_tag = if lang.is_empty() {
                String::new()
            } else {
                format!(" [{}]", lang)
            };
            let topic_str = if topics.is_empty() {
                String::new()
            } else {
                format!(" ({})", topics.join(", "))
            };
            out.push_str(&format!(
                "{}. {} ★{}{}{}\n   {}\n   {}\n\n",
                i + 1,
                name,
                stars,
                lang_tag,
                topic_str,
                trunc(desc, 150),
                url,
            ));
        }
    }
    if out.ends_with(":\n\n") {
        out.push_str("  No results found.\n");
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
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();
    let mut out = format!("Best of JS projects for '{query}':\n\n");
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
            out.push_str(&format!(
                "{}. {} ★{}{}\n   {}\n   {}\n\n",
                count,
                name,
                stars,
                tag_str,
                trunc(desc, 150),
                url,
            ));
        }
    }
    if count == 0 {
        out.push_str("  No matching projects.\n");
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
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    let result_count = data["data"]["search"]["results"]["resultCount"]
        .as_u64()
        .unwrap_or(0);
    let mut out = format!("Sourcegraph code search for '{query}' ({result_count} results):\n\n");
    let mut count = 0;
    if let Some(results) = data["data"]["search"]["results"]["results"].as_array() {
        for r in results {
            count += 1;
            let typename = r["__typename"].as_str().unwrap_or("");
            match typename {
                "Repository" => {
                    let name = r["name"].as_str().unwrap_or("?");
                    let desc = r["description"].as_str().unwrap_or("");
                    out.push_str(&format!("{}. [REPO] {}\n", count, name));
                    if !desc.is_empty() {
                        out.push_str(&format!("   {}\n", trunc(desc, 150)));
                    }
                    out.push_str(&format!("   https://sourcegraph.com/{}\n\n", name));
                }
                "FileMatch" => {
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
                    out.push_str(&format!(
                        "   https://sourcegraph.com/{}/-/blob/{}\n\n",
                        repo, path
                    ));
                }
                _ => {}
            }
        }
    }
    if count == 0 {
        out.push_str("  No results found.\n");
    }
    out.push_str(&format!(
        "Sourcegraph: https://sourcegraph.com/search?q={}\n",
        query.replace(' ', "+"),
    ));
    Ok(out)
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

async fn ddg_surface(q: &str) -> Vec<(String, String, String, &'static str)> {
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
    let html = match resp {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return vec![],
    };
    parse_ddg_html(&html, "明网")
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
    let mut out = Vec::new();
    for d in dorks {
        let mut r = ddg_query(&d, "定向(dork)").await;
        out.append(&mut r);
    }
    out
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

#[tauri::command]
pub async fn deep_search(query: String, max_results: Option<usize>) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("空搜索词".into());
    }
    // Best-effort: if Tor is down, kick a background start so the .onion layers self-heal
    // (this run or the next). Non-blocking — the clearnet layers never wait on Tor.
    tokio::spawn(async {
        let _ = crate::net::ensure_tor().await;
    });
    let limit = max_results.unwrap_or(24).min(60);
    let domain = looks_like_domain(q);

    let mut racing: FuturesUnordered<DeepSearchFuture> = FuturesUnordered::new();
    // Keyword layers — always run: surface + dorks (pastes/leaks/files) + 4 dark-web engines.
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
        racing.push(Box::pin(async move { ddg_onion(&q).await }));
    }
    {
        let q = q.to_string();
        racing.push(Box::pin(async move { torch_search_layer(&q).await }));
    }
    {
        let q = q.to_string();
        racing.push(Box::pin(async move { haystak_layer(&q).await }));
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

    let mut all: Vec<(String, String, String, &str)> = Vec::new();
    let mut seen_urls = std::collections::HashSet::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(40);

    while let Ok(Some(batch)) = tokio::time::timeout_at(deadline, racing.next()).await {
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
            "深层搜索「{q}」未找到结果（Tor 可能未运行：brew services start tor）。"
        ));
    }

    let mut out = format!(
        "🔍 深层情报搜索「{q}」— {} 条结果（跨明网+定向dork+存档+子域名+暗网多引擎）：\n",
        all.len().min(limit)
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
         提示：明网/存档 URL 用 web_fetch 读；.onion URL 用 tor_request 读（Tor 未运行则暗网层为空：brew services start tor）。存档快照能读到原网站已删除的内容。",
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    #[ignore = "calls live third-party developer communities"]
    async fn developer_community_live_smoke_lists_every_supported_source() {
        let out = developer_community_search(
            "rust async error handling".into(),
            Some("all".into()),
            None,
            Some(1),
        )
        .await
        .unwrap();
        println!("{out}");
        assert!(out.contains(&format!(
            "Requested sources: {}",
            DEVELOPER_COMMUNITY_SOURCES.len()
        )));
        assert_eq!(
            out.matches("\n## ").count(),
            DEVELOPER_COMMUNITY_SOURCES.len()
        );
    }
}
