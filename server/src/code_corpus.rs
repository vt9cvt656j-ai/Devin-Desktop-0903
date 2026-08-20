//! 自有代码语料库：把公开生态的**真实 API 表面**抽进这台服务器自己的索引。
//!
//! # 它补的洞
//!
//! `package_source` 只读本机装了的包，没装就是一句「未安装」；`package_search` 只有注册表
//! 元数据、没有任何签名。于是「这个没装的库，某个函数真实签名是什么」在整套工具里无解，
//! 模型只能靠训练记忆猜——那正是它编 API 的地方。
//!
//! # 只抽 API 表面，不存整包
//!
//! 一台机器装不下全网源码，但模型要的从来不是整包：是导出了什么、签名什么样、文档怎么说。
//! npm 包的 `.d.ts` 恰好就是这份东西的官方形态——它是**包自己声明的**类型契约，比从 JS
//! 实现里猜准得多，而且体积只有源码的零头。
//!
//! # 生长方式
//!
//! 按真实需求抓：谁问到一个还没收录的包，就现拉、抽取、入库，此后永久留下。
//! 用得越久覆盖越全，而且索引长在自己机器上，不依赖任何第三方服务。

use anyhow::{anyhow, Context, Result};
use std::io::Read;

/// 单个 tarball 的下载上限。npm 上有几百 MB 的怪物包（含预编译二进制），
/// 抽 API 完全用不到那些，超限直接放弃并在台账里留因由。
const MAX_TARBALL_BYTES: u64 = 24 * 1024 * 1024;
/// 解包后单个文件的上限。一个 .d.ts 超过这个尺寸多半是打包产物而不是声明。
const MAX_ENTRY_BYTES: usize = 2 * 1024 * 1024;
/// 一个包最多抽多少条。防一个巨型 SDK 把整张表灌满。
const MAX_ENTRIES_PER_PACKAGE: usize = 400;
/// 正文截断长度。签名 + 文档注释，超过这个长度的多半是把实现也带进来了。
const MAX_BODY_CHARS: usize = 4000;
const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// 允许抓取的主机。**白名单是硬要求**：tarball 地址来自注册表返回的 JSON，
/// 那是外部输入；不校验就等于让任何人指使这台服务器去访问任意内网地址（SSRF）。
fn host_allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false; // 明文 http 一律不要
    };
    let host = rest.split('/').next().unwrap_or("");
    // 端口不允许自定义：`registry.npmjs.org:2375` 这种写法要挡掉。
    if host.contains(':') {
        return false;
    }
    host == "registry.npmjs.org"
        || host == "registry.yarnpkg.com"
        || host.ends_with(".npmjs.org")
        || host.ends_with(".npmjs.com")
        // PyPI：元数据在 pypi.org，产物在 files.pythonhosted.org
        || host == "pypi.org"
        || host == "files.pythonhosted.org"
        // crates.io：元数据在 crates.io，.crate 在 static.crates.io
        || host == "crates.io"
        || host == "static.crates.io"
}

/// 生态。每一支的差别只在「去哪拿包」和「从什么文件里抽签名」，其余共用。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Eco {
    Npm,
    PyPI,
    Crates,
}

impl Eco {
    pub fn as_str(self) -> &'static str {
        match self {
            Eco::Npm => "npm",
            Eco::PyPI => "pypi",
            Eco::Crates => "crates",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "npm" => Some(Eco::Npm),
            "pypi" | "pip" | "python" => Some(Eco::PyPI),
            "crates" | "cargo" | "rust" => Some(Eco::Crates),
            _ => None,
        }
    }
}

#[derive(Debug, Default)]
pub struct IngestReport {
    pub name: String,
    pub version: String,
    pub entries: usize,
    pub bytes: u64,
}

/// 抽出来的一条：一个导出符号，或包级条目（symbol 为空）。
struct Entry {
    kind: &'static str,
    symbol: String,
    title: String,
    body: String,
}

fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .user_agent("michael-ide-code-corpus/1.0")
        .build()
        .context("build http client")
}

/// npm 包名是否合法。挡掉路径穿越和注入到 URL 里的花样。
fn valid_npm_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 214 || name.starts_with('.') || name.starts_with('_') {
        return false;
    }
    let body = name.strip_prefix('@').unwrap_or(name);
    // scope 形式只允许一个斜杠：@scope/pkg
    let mut parts = body.split('/');
    let (Some(first), second, None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let segs = if name.starts_with('@') {
        match second {
            Some(s) => vec![first, s],
            None => return false,
        }
    } else {
        if second.is_some() {
            return false;
        }
        vec![first]
    };
    segs.iter().all(|seg| {
        !seg.is_empty()
            && seg.len() <= 128
            && seg
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.'))
    })
}

/// 从注册表拿 tarball 地址和确切版本。
async fn npm_dist(client: &reqwest::Client, name: &str, want: Option<&str>) -> Result<(String, String)> {
    let url = format!("https://registry.npmjs.org/{name}");
    if !host_allowed(&url) {
        return Err(anyhow!("registry host not allowed"));
    }
    let meta: serde_json::Value = client
        .get(&url)
        .send()
        .await
        .context("fetch registry metadata")?
        .error_for_status()
        .context("registry returned an error status")?
        .json()
        .await
        .context("registry metadata is not JSON")?;

    let version = match want {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => meta
            .pointer("/dist-tags/latest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("registry metadata has no dist-tags.latest"))?
            .to_string(),
    };
    let tarball = meta
        .pointer(&format!("/versions/{}/dist/tarball", version.replace('~', "~0").replace('/', "~1")))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("no tarball for {name}@{version}"))?
        .to_string();
    if !host_allowed(&tarball) {
        // 注册表理论上只会给自己的地址，但它是外部输入，按外部输入对待。
        return Err(anyhow!("tarball host not allowed: {tarball}"));
    }
    Ok((tarball, version))
}

/// 下载并解包，返回 (相对路径, 文本内容) 列表。只留下抽 API 用得上的那几类文件。
async fn fetch_and_unpack(client: &reqwest::Client, tarball: &str) -> Result<(Vec<(String, String)>, u64)> {
    let resp = client
        .get(tarball)
        .send()
        .await
        .context("download tarball")?
        .error_for_status()
        .context("tarball download returned an error status")?;
    if let Some(len) = resp.content_length() {
        if len > MAX_TARBALL_BYTES {
            return Err(anyhow!("tarball is {len} bytes, over the {MAX_TARBALL_BYTES} cap"));
        }
    }
    let bytes = resp.bytes().await.context("read tarball body")?;
    // Content-Length 可以缺席或撒谎，所以拿到实体之后再量一次。
    if bytes.len() as u64 > MAX_TARBALL_BYTES {
        return Err(anyhow!("tarball is {} bytes, over the cap", bytes.len()));
    }
    let downloaded = bytes.len() as u64;

    // 解包是 CPU 活，别占着 async 执行器。
    let files = tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>> {
        let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(bytes));
        let mut archive = tar::Archive::new(gz);
        let mut out: Vec<(String, String)> = Vec::new();
        for entry in archive.entries().context("read tar entries")? {
            let mut entry = entry.context("bad tar entry")?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let path = entry.path().context("bad entry path")?.to_string_lossy().to_string();
            // npm tarball 里所有东西都在 `package/` 下。
            let rel = path.strip_prefix("package/").unwrap_or(&path).to_string();
            if !wanted_file(&rel) {
                continue;
            }
            let size = entry.header().size().unwrap_or(0) as usize;
            if size > MAX_ENTRY_BYTES {
                continue;
            }
            let mut buf = String::new();
            // 二进制文件读成字符串会失败——跳过即可，那本来就不是我们要的。
            if entry.read_to_string(&mut buf).is_err() {
                continue;
            }
            out.push((rel, buf));
            if out.len() > 600 {
                break; // 一个包里有用的声明文件不会有这么多
            }
        }
        Ok(out)
    })
    .await
    .context("unpack task panicked")??;

    Ok((files, downloaded))
}

/// 这个文件抽 API 用得上吗。
fn wanted_file(rel: &str) -> bool {
    if rel == "package.json" {
        return true;
    }
    let lower = rel.to_ascii_lowercase();
    if lower.starts_with("readme") && (lower.ends_with(".md") || lower.ends_with(".markdown")) {
        return true;
    }
    // 类型声明就是包自己声明的 API 契约——比从实现里猜准得多。
    lower.ends_with(".d.ts") && !lower.contains("/test/") && !lower.contains("/__tests__/")
}

/// 取紧贴在声明前面的 `/** ... */` 文档注释。
fn doc_comment_before(src: &str, decl_start: usize) -> String {
    let head = &src[..decl_start];
    let trimmed = head.trim_end();
    if !trimmed.ends_with("*/") {
        return String::new();
    }
    let Some(open) = trimmed.rfind("/**") else {
        return String::new();
    };
    let raw = &trimmed[open..];
    // 剥掉注释框架，只留文字。
    raw.lines()
        .map(|l| {
            l.trim()
                .trim_start_matches("/**")
                .trim_start_matches("*/")
                .trim_start_matches('*')
                .trim()
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从一段 `.d.ts` 里抽导出声明。
///
/// 不引入 TS 解析器：这里要的是**签名那一行（或几行）加它的文档注释**，不是完整 AST。
/// 逐行扫 `export` 开头的声明，把签名读到分号或第一个 `{` 为止——这对
/// `export declare function f(...): T;` / `export interface X {` / `export class Y {` 都成立。
fn extract_dts(src: &str, out: &mut Vec<Entry>) {
    const KEYWORDS: &[&str] = &[
        "function", "class", "interface", "type", "const", "enum", "abstract class", "namespace",
    ];
    let bytes = src.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let line_end = src[i..].find('\n').map(|e| i + e).unwrap_or(bytes.len());
        let line = &src[i..line_end];
        let t = line.trim_start();
        if t.starts_with("export ") {
            let after = t.trim_start_matches("export ").trim_start();
            let after = after.strip_prefix("declare ").unwrap_or(after).trim_start();
            if let Some(kw) = KEYWORDS.iter().find(|k| after.starts_with(**k)) {
                // 名字：关键字之后的第一个标识符。
                let rest = after[kw.len()..].trim_start();
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
                    .collect();
                if !name.is_empty() {
                    // 签名读到分号或 `{`（含它自己那一行），最多跨 12 行——
                    // 多行参数表很常见，而读到实现体里去没有意义。
                    let mut sig_end = line_end;
                    let mut lines_used = 0;
                    while sig_end < bytes.len() && lines_used < 12 {
                        let seg = &src[..sig_end];
                        if seg[i..].contains(';') || seg[i..].contains('{') {
                            break;
                        }
                        let nxt = src[sig_end + 1..]
                            .find('\n')
                            .map(|e| sig_end + 1 + e)
                            .unwrap_or(bytes.len());
                        sig_end = nxt;
                        lines_used += 1;
                    }
                    let signature = src[i..sig_end.min(bytes.len())].trim().to_string();
                    let doc = doc_comment_before(src, i);
                    let mut body = signature;
                    if !doc.is_empty() {
                        body.push_str("\n\n");
                        body.push_str(&doc);
                    }
                    if body.chars().count() > MAX_BODY_CHARS {
                        body = body.chars().take(MAX_BODY_CHARS).collect();
                    }
                    out.push(Entry {
                        kind: "package_api",
                        symbol: name.clone(),
                        title: format!("{kw} {name}"),
                        body,
                    });
                }
            }
        }
        if out.len() >= MAX_ENTRIES_PER_PACKAGE {
            return;
        }
        i = line_end + 1;
    }
}

/// README 按 `##` 切节，和手写语料库同一套切法。
fn extract_readme(src: &str, out: &mut Vec<Entry>) {
    let mut section = String::new();
    let mut title = String::from("README");
    let mut push = |title: &str, body: &str, out: &mut Vec<Entry>| {
        let body = body.trim();
        if body.len() < 40 || out.len() >= MAX_ENTRIES_PER_PACKAGE {
            return;
        }
        let body = if body.chars().count() > MAX_BODY_CHARS {
            body.chars().take(MAX_BODY_CHARS).collect()
        } else {
            body.to_string()
        };
        out.push(Entry {
            kind: "package_readme",
            symbol: String::new(),
            title: title.to_string(),
            body,
        });
    };
    for line in src.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            push(&title, &section, out);
            section.clear();
            title = h.trim().to_string();
        } else {
            section.push_str(line);
            section.push('\n');
        }
    }
    push(&title, &section, out);
}

/// 抓一个 npm 包并入库。已经收录过同一版本时是 no-op（唯一索引兜底）。
pub async fn ingest_npm(
    db: &sqlx::PgPool,
    name: &str,
    version: Option<&str>,
) -> Result<IngestReport> {
    if !valid_npm_name(name) {
        return Err(anyhow!("not a valid npm package name"));
    }
    let client = http()?;
    let (tarball, version) = npm_dist(&client, name, version).await?;
    let (files, bytes) = fetch_and_unpack(&client, &tarball).await?;

    let mut entries: Vec<Entry> = Vec::new();
    let mut exports_line = String::new();
    for (rel, body) in &files {
        if rel == "package.json" {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(body) {
                let desc = pkg.get("description").and_then(|v| v.as_str()).unwrap_or("");
                let types = pkg
                    .get("types")
                    .or_else(|| pkg.get("typings"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                exports_line = format!(
                    "{}{}",
                    desc,
                    if types.is_empty() { String::new() } else { format!("\n类型入口: {types}") }
                );
            }
        } else if rel.to_ascii_lowercase().starts_with("readme") {
            extract_readme(body, &mut entries);
        } else {
            extract_dts(body, &mut entries);
        }
        if entries.len() >= MAX_ENTRIES_PER_PACKAGE {
            break;
        }
    }
    if !exports_line.trim().is_empty() {
        entries.push(Entry {
            kind: "package_readme",
            symbol: String::new(),
            title: format!("{name} 概述"),
            body: exports_line,
        });
    }

    let mut written = 0usize;
    for e in &entries {
        // 同包同版本同符号只留一条；重复抓取是 no-op。
        let res = sqlx::query(
            "INSERT INTO code_corpus (kind, ecosystem, name, version, symbol, title, body, source_url) \
             VALUES ($1,'npm',$2,$3,$4,$5,$6,$7) \
             ON CONFLICT (ecosystem, name, version, kind, symbol) DO NOTHING",
        )
        .bind(e.kind)
        .bind(name)
        .bind(&version)
        .bind(&e.symbol)
        .bind(&e.title)
        .bind(&e.body)
        .bind(&tarball)
        .execute(db)
        .await;
        if res.is_ok() {
            written += 1;
        }
    }

    let _ = sqlx::query(
        "INSERT INTO code_corpus_fetches (ecosystem, name, version, ok, entries, bytes) \
         VALUES ('npm',$1,$2,true,$3,$4) \
         ON CONFLICT (ecosystem, name, version) DO UPDATE \
           SET ok = true, entries = EXCLUDED.entries, bytes = EXCLUDED.bytes, \
               error = NULL, fetched_at = now()",
    )
    .bind(name)
    .bind(&version)
    .bind(written as i32)
    .bind(bytes as i64)
    .execute(db)
    .await;

    Ok(IngestReport { name: name.to_string(), version, entries: written, bytes })
}

/// 抓取失败也要留痕，否则同一个抓不到的包会被反复重抓，而原因只在日志里一闪而过。
pub async fn record_failure(db: &sqlx::PgPool, name: &str, version: &str, err: &str) {
    let _ = sqlx::query(
        "INSERT INTO code_corpus_fetches (ecosystem, name, version, ok, error) \
         VALUES ('npm',$1,$2,false,$3) \
         ON CONFLICT (ecosystem, name, version) DO UPDATE \
           SET ok = false, error = EXCLUDED.error, fetched_at = now()",
    )
    .bind(name)
    .bind(version)
    .bind(err.chars().take(500).collect::<String>())
    .execute(db)
    .await;
}

#[derive(serde::Serialize)]
pub struct CorpusHit {
    pub ecosystem: String,
    pub name: String,
    pub version: String,
    pub symbol: String,
    pub title: String,
    pub body: String,
    pub score: f32,
}

/// 把用户的话拆成词、用 `or` 连起来，交给 `websearch_to_tsquery`。
///
/// 为什么要这一步：`websearch_to_tsquery` 默认是 **AND**——「parse schema」要求两个词
/// 全中才算命中。对代码检索这太严了：真实提问几乎都是「一个动词 + 一个名词」的自然语言，
/// 而签名里往往只出现其中一个（实测 zod v4 里 `parse` 是 schema 对象上的方法，
/// 压根不是顶层导出，于是「parse schema」一条都召不回）。
///
/// 只做**取词**，不拼语法：过滤掉非字母数字，再用 websearch 自己认识的 `or` 连接。
/// 于是没有任何注入面——websearch_to_tsquery 对乱七八糟的输入也只会返回空查询，不会报错。
fn or_form(query: &str) -> String {
    let words: Vec<&str> = query
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
        .filter(|w| w.len() >= 2)
        .take(12)
        .collect();
    words.join(" or ")
}

/// 检索。内置全文排相关性；给了包名就把它限定住，避免同名符号跨包串味。
///
/// 召回用 OR 式、排序偏向 AND 式：两个词都中的排在只中一个的前面，
/// 而只中一个的仍然能被召回——这才是「找相关 API」要的行为。
pub async fn search(
    db: &sqlx::PgPool,
    query: &str,
    package: Option<&str>,
    limit: i64,
) -> Result<Vec<CorpusHit>> {
    let limit = limit.clamp(1, 20);
    let or_q = or_form(query);
    let rows: Vec<(String, String, String, String, String, String, f32)> = sqlx::query_as(
        "SELECT ecosystem, name, version, symbol, title, body, \
                (ts_rank(tsv, websearch_to_tsquery('english', $1)) * 2.0 \
                  + ts_rank(tsv, websearch_to_tsquery('english', $5)) \
                  + CASE WHEN symbol <> '' THEN similarity(symbol, $2) ELSE 0 END)::real AS score \
           FROM code_corpus \
          WHERE ($3 = '' OR name = $3) \
            AND ($5 <> '' AND tsv @@ websearch_to_tsquery('english', $5) \
                 OR (symbol <> '' AND symbol % $2)) \
          ORDER BY score DESC, length(body) ASC \
          LIMIT $4",
    )
    .bind(query)
    .bind(query)
    .bind(package.unwrap_or(""))
    .bind(limit)
    .bind(&or_q)
    .fetch_all(db)
    .await
    .context("code corpus search")?;

    Ok(rows
        .into_iter()
        .map(|(ecosystem, name, version, symbol, title, body, score)| CorpusHit {
            ecosystem,
            name,
            version,
            symbol,
            title,
            body,
            score,
        })
        .collect())
}

/// 这个包收录过没有。按需入库那条路先问它，再决定要不要现拉。
pub async fn have_package(db: &sqlx::PgPool, name: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM code_corpus WHERE ecosystem = 'npm' AND name = $1",
    )
    .bind(name)
    .fetch_one(db)
    .await
    .map(|n| n > 0)
    .unwrap_or(false)
}

/// 抓一个包并入库，按生态分派。已经收录过同一版本时是 no-op（唯一索引兜底）。
///
/// 三支的差别只有两件事：去哪拿包、从什么文件里抽签名。入库、去重、台账全共用。
pub async fn ingest(db: &sqlx::PgPool, eco: Eco, name: &str, version: Option<&str>) -> Result<IngestReport> {
    match eco {
        Eco::Npm => return ingest_npm(db, name, version).await,
        _ => {}
    }
    if !valid_simple_name(name) {
        return Err(anyhow!("not a valid package name"));
    }
    let client = http()?;
    let (url, version, files, bytes) = match eco {
        Eco::PyPI => {
            let (href, ver, is_wheel) = pypi_dist(&client, name).await?;
            let (files, bytes) = if is_wheel {
                fetch_and_unzip(&client, &href, eco).await?
            } else {
                fetch_and_untar(&client, &href, eco).await?
            };
            (href, ver, files, bytes)
        }
        Eco::Crates => {
            let (href, ver) = crates_dist(&client, name).await?;
            let (files, bytes) = fetch_and_untar(&client, &href, eco).await?;
            (href, ver, files, bytes)
        }
        Eco::Npm => unreachable!(),
    };

    let mut entries: Vec<Entry> = Vec::new();
    for (rel, body) in &files {
        let lower = rel.to_ascii_lowercase();
        if lower.split('/').next_back().is_some_and(|f| f.starts_with("readme")) {
            extract_readme(body, &mut entries);
        } else {
            match eco {
                Eco::PyPI => extract_python(body, &mut entries),
                Eco::Crates => extract_rust(body, &mut entries),
                Eco::Npm => unreachable!(),
            }
        }
        if entries.len() >= MAX_ENTRIES_PER_PACKAGE {
            break;
        }
    }

    let written = write_entries(db, eco, name, &version, &url, &entries).await;
    let _ = sqlx::query(
        "INSERT INTO code_corpus_fetches (ecosystem, name, version, ok, entries, bytes) \
         VALUES ($1,$2,$3,true,$4,$5) \
         ON CONFLICT (ecosystem, name, version) DO UPDATE \
           SET ok = true, entries = EXCLUDED.entries, bytes = EXCLUDED.bytes, error = NULL, fetched_at = now()",
    )
    .bind(eco.as_str()).bind(name).bind(&version)
    .bind(written as i32).bind(bytes as i64)
    .execute(db).await;

    Ok(IngestReport { name: name.to_string(), version, entries: written, bytes })
}

/// 下载并解 tar.gz（sdist / .crate 都是这个格式）。
async fn fetch_and_untar(client: &reqwest::Client, url: &str, eco: Eco) -> Result<(Vec<(String, String)>, u64)> {
    let bytes = download_capped(client, url).await?;
    let downloaded = bytes.len() as u64;
    let files = tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>> {
        let gz = flate2::read::GzDecoder::new(std::io::Cursor::new(bytes));
        let mut archive = tar::Archive::new(gz);
        let mut out = Vec::new();
        for entry in archive.entries().context("read tar entries")? {
            let mut entry = entry.context("bad tar entry")?;
            if !entry.header().entry_type().is_file() { continue; }
            let rel = entry.path().context("bad entry path")?.to_string_lossy().to_string();
            if !wanted_file_for(&rel, eco) { continue; }
            if entry.header().size().unwrap_or(0) as usize > MAX_ENTRY_BYTES { continue; }
            let mut buf = String::new();
            if entry.read_to_string(&mut buf).is_err() { continue; }
            out.push((rel, buf));
            if out.len() > 600 { break; }
        }
        Ok(out)
    })
    .await
    .context("untar task panicked")??;
    Ok((files, downloaded))
}

/// 入库。抽出来多少条就写多少条，重复的靠唯一索引挡掉。
async fn write_entries(
    db: &sqlx::PgPool, eco: Eco, name: &str, version: &str, url: &str, entries: &[Entry],
) -> usize {
    let mut written = 0usize;
    for e in entries {
        let res = sqlx::query(
            "INSERT INTO code_corpus (kind, ecosystem, name, version, symbol, title, body, source_url) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             ON CONFLICT (ecosystem, name, version, kind, symbol) DO NOTHING",
        )
        .bind(e.kind).bind(eco.as_str()).bind(name).bind(version)
        .bind(&e.symbol).bind(&e.title).bind(&e.body).bind(url)
        .execute(db).await;
        if res.is_ok() { written += 1; }
    }
    written
}

/// 包名字符集：PyPI / crates 的合法名比 npm 更窄，统一用这一条挡掉能注进 URL 的花样。
fn valid_simple_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// PyPI：元数据 → 优先 sdist（.tar.gz，带完整源码），没有就退而取 wheel（.whl，zip）。
async fn pypi_dist(client: &reqwest::Client, name: &str) -> Result<(String, String, bool)> {
    let url = format!("https://pypi.org/pypi/{name}/json");
    if !host_allowed(&url) {
        return Err(anyhow!("pypi host not allowed"));
    }
    let meta: serde_json::Value = client
        .get(&url).send().await.context("fetch pypi metadata")?
        .error_for_status().context("pypi returned an error status")?
        .json().await.context("pypi metadata is not JSON")?;
    let version = meta.pointer("/info/version").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let urls = meta.get("urls").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    // sdist 里有真源码和 docstring；wheel 常常只有编译产物，但总比没有强。
    let pick = urls.iter().find(|u| u.get("packagetype").and_then(|v| v.as_str()) == Some("sdist"))
        .or_else(|| urls.iter().find(|u| u.get("packagetype").and_then(|v| v.as_str()) == Some("bdist_wheel")));
    let Some(pick) = pick else { return Err(anyhow!("no downloadable artifact for {name}")) };
    let href = pick.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let is_wheel = href.ends_with(".whl");
    if !host_allowed(&href) {
        return Err(anyhow!("artifact host not allowed: {href}"));
    }
    Ok((href, version, is_wheel))
}

/// crates.io：`.crate` 就是个 tar.gz。
async fn crates_dist(client: &reqwest::Client, name: &str) -> Result<(String, String)> {
    let url = format!("https://crates.io/api/v1/crates/{name}");
    if !host_allowed(&url) {
        return Err(anyhow!("crates host not allowed"));
    }
    let meta: serde_json::Value = client
        .get(&url).send().await.context("fetch crates metadata")?
        .error_for_status().context("crates returned an error status")?
        .json().await.context("crates metadata is not JSON")?;
    let version = meta.pointer("/crate/max_stable_version")
        .or_else(|| meta.pointer("/crate/newest_version"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("crates metadata has no version"))?
        .to_string();
    let href = format!("https://static.crates.io/crates/{name}/{name}-{version}.crate");
    Ok((href, version))
}

/// 下载一个 zip（wheel）并取出想要的文本文件。
async fn fetch_and_unzip(client: &reqwest::Client, url: &str, eco: Eco) -> Result<(Vec<(String, String)>, u64)> {
    let bytes = download_capped(client, url).await?;
    let downloaded = bytes.len() as u64;
    let files = tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>> {
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("open wheel zip")?;
        let mut out = Vec::new();
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).context("read zip entry")?;
            if !f.is_file() { continue; }
            let rel = f.name().to_string();
            if !wanted_file_for(&rel, eco) { continue; }
            if f.size() as usize > MAX_ENTRY_BYTES { continue; }
            let mut buf = String::new();
            if std::io::Read::read_to_string(&mut f, &mut buf).is_err() { continue; }
            out.push((rel, buf));
            if out.len() > 600 { break; }
        }
        Ok(out)
    })
    .await
    .context("unzip task panicked")??;
    Ok((files, downloaded))
}

/// 带上限的下载。Content-Length 可以缺席或撒谎，所以拿到实体之后再量一次。
async fn download_capped(client: &reqwest::Client, url: &str) -> Result<bytes::Bytes> {
    if !host_allowed(url) {
        return Err(anyhow!("host not allowed"));
    }
    let resp = client.get(url).send().await.context("download")?
        .error_for_status().context("download returned an error status")?;
    if let Some(len) = resp.content_length() {
        if len > MAX_TARBALL_BYTES {
            return Err(anyhow!("artifact is {len} bytes, over the cap"));
        }
    }
    let bytes = resp.bytes().await.context("read body")?;
    if bytes.len() as u64 > MAX_TARBALL_BYTES {
        return Err(anyhow!("artifact is {} bytes, over the cap", bytes.len()));
    }
    Ok(bytes)
}

/// 这个文件对该生态有没有抽取价值。
fn wanted_file_for(rel: &str, eco: Eco) -> bool {
    let lower = rel.to_ascii_lowercase();
    if lower.contains("/test/") || lower.contains("/tests/") || lower.contains("/__tests__/") {
        return false;
    }
    match eco {
        Eco::Npm => wanted_file(rel),
        // Python：类型存根优先，其次是源码；README 给包级说明。
        Eco::PyPI => {
            lower.ends_with(".pyi")
                || (lower.ends_with(".py") && !lower.contains("/_vendor/"))
                || lower.split('/').next_back().is_some_and(|f| f.starts_with("readme"))
        }
        // Rust：只看 src 下的 .rs，README 同上。
        Eco::Crates => {
            (lower.ends_with(".rs") && lower.contains("/src/"))
                || lower.split('/').next_back().is_some_and(|f| f.starts_with("readme"))
        }
    }
}

/// PyPI：从 `.pyi` / `.py` 里抽顶层 `def` / `class` 签名和紧跟其后的 docstring。
///
/// 和 TS 不同，Python 的文档字符串在**声明之后**（缩进的三引号块），不是之前。
/// 只收顶层（零缩进）声明：类里的方法跟着类一起出现在签名行里，单独抽会把语料灌爆。
fn extract_python(src: &str, out: &mut Vec<Entry>) {
    let lines: Vec<&str> = src.lines().collect();
    let quotes = ["\"\"\"", "'''"];
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        let is_decl = line.starts_with("def ") || line.starts_with("class ") || line.starts_with("async def ");
        if is_decl {
            let kw = if line.starts_with("class ") { "class" } else { "def" };
            let after = line
                .trim_start_matches("async ")
                .trim_start_matches("def ")
                .trim_start_matches("class ");
            let name: String = after.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            // 私有的不进语料：`_x` 是约定俗成的「别用我」。
            if !name.is_empty() && !name.starts_with('_') {
                // 签名可能跨行（参数表换行），读到以 `:` 结尾的那一行为止。
                let mut sig = String::new();
                let mut j = i;
                while j < lines.len() && j < i + 12 {
                    sig.push_str(lines[j].trim_end());
                    sig.push('\n');
                    if lines[j].trim_end().ends_with(':') { break; }
                    j += 1;
                }
                // docstring 紧跟在声明之后。
                let mut doc = String::new();
                let mut k = j + 1;
                while k < lines.len() && lines[k].trim().is_empty() { k += 1; }
                if k < lines.len() {
                    let t = lines[k].trim();
                    if let Some(q) = quotes.iter().find(|q| t.starts_with(**q)) {
                        let mut body = t.trim_start_matches(*q).to_string();
                        if !body.trim_end().ends_with(q) {
                            let mut m = k + 1;
                            while m < lines.len() && !lines[m].contains(q) && m < k + 40 {
                                body.push('\n');
                                body.push_str(lines[m].trim());
                                m += 1;
                            }
                        }
                        doc = body.trim_end_matches(*q).trim().to_string();
                    }
                }
                let mut body = sig.trim().to_string();
                if !doc.is_empty() { body.push_str("\n\n"); body.push_str(&doc); }
                if body.chars().count() > MAX_BODY_CHARS {
                    body = body.chars().take(MAX_BODY_CHARS).collect();
                }
                out.push(Entry { kind: "package_api", symbol: name.clone(), title: format!("{kw} {name}"), body });
                if out.len() >= MAX_ENTRIES_PER_PACKAGE { return; }
            }
        }
        i += 1;
    }
}

/// Rust：抽 `pub fn` / `pub struct` / `pub trait` / `pub enum`，连同紧贴在上面的 `///` 文档。
fn extract_rust(src: &str, out: &mut Vec<Entry>) {
    let lines: Vec<&str> = src.lines().collect();
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim_start();
        let Some(after) = line.strip_prefix("pub ") else { continue };
        let after = after.strip_prefix("async ").unwrap_or(after);
        const KWS: &[&str] = &["fn ", "struct ", "trait ", "enum ", "type ", "const "];
        let Some(kw) = KWS.iter().find(|k| after.starts_with(**k)) else { continue };
        let name: String = after[kw.len()..]
            .chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if name.is_empty() { continue; }
        // 签名读到 `{`、`;` 或参数表收尾为止。
        let mut sig = String::new();
        for l in lines.iter().skip(i).take(8) {
            sig.push_str(l.trim_end());
            sig.push('\n');
            let t = l.trim_end();
            if t.ends_with('{') || t.ends_with(';') || t.ends_with(')') { break; }
        }
        // `///` 文档注释在声明**之前**，往上收；属性宏和空行不打断文档块。
        let mut doc_lines: Vec<&str> = Vec::new();
        let mut k = i;
        while k > 0 {
            k -= 1;
            let t = lines[k].trim();
            if let Some(d) = t.strip_prefix("///") { doc_lines.push(d.trim()); }
            else if t.starts_with("#[") || t.is_empty() { continue; }
            else { break; }
        }
        doc_lines.reverse();
        let mut body = sig.trim().to_string();
        if !doc_lines.is_empty() { body.push_str("\n\n"); body.push_str(&doc_lines.join("\n")); }
        if body.chars().count() > MAX_BODY_CHARS {
            body = body.chars().take(MAX_BODY_CHARS).collect();
        }
        out.push(Entry { kind: "package_api", symbol: name.clone(), title: format!("{} {}", kw.trim(), name), body });
        if out.len() >= MAX_ENTRIES_PER_PACKAGE { return; }
    }
}

/// 批量预热的种子词。
///
/// npm 的搜索接口按流行度排序，但**必须给一个 text**，没法裸枚举整个注册表。
/// 所以拿一组覆盖面广的词各翻几页，去重之后就是一份「常用包」清单。
/// 选词的标准是**生态切面**而不是具体库名：框架、运行时、构建、测试、数据、云、
/// 协议、语言工具——这样命中的是每个方向的头部包，而不是某一家的全家桶。
const SEED_TERMS: &[&str] = &[
    "react", "vue", "angular", "svelte", "next", "node", "express", "typescript",
    "webpack", "vite", "rollup", "babel", "eslint", "prettier", "jest", "vitest",
    "test", "cli", "http", "fetch", "axios", "database", "sql", "orm", "mongodb",
    "redis", "postgres", "auth", "jwt", "crypto", "date", "time", "lodash", "utility",
    "css", "tailwind", "styled", "animation", "chart", "table", "form", "validation",
    "state", "router", "graphql", "grpc", "websocket", "queue", "logger", "config",
    "aws", "azure", "google-cloud", "docker", "kubernetes", "stream", "parser",
    "markdown", "json", "yaml", "csv", "image", "video", "pdf", "email", "i18n",
];

/// 从 npm 官方搜索接口按流行度收集包名。
async fn discover_popular(client: &reqwest::Client, per_term: usize) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut names: BTreeSet<String> = BTreeSet::new();
    for term in SEED_TERMS {
        let mut from = 0usize;
        while from < per_term {
            let size = 250.min(per_term - from);
            let url = format!(
                "https://registry.npmjs.org/-/v1/search?text={term}&size={size}&from={from}\
                 &popularity=1.0&quality=0.0&maintenance=0.0"
            );
            if !host_allowed(&url) {
                break;
            }
            let Ok(resp) = client.get(&url).send().await else { break };
            let Ok(body) = resp.json::<serde_json::Value>().await else { break };
            let objects = body.get("objects").and_then(|v| v.as_array()).cloned().unwrap_or_default();
            if objects.is_empty() {
                break;
            }
            for o in &objects {
                if let Some(n) = o.pointer("/package/name").and_then(|v| v.as_str()) {
                    if valid_npm_name(n) {
                        names.insert(n.to_string());
                    }
                }
            }
            from += size;
            // 对官方接口客气一点：这是白嫖别人的服务，不是压测。
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    }
    names.into_iter().collect()
}

/// 这个包最近抓过吗（不管成功失败）。抓过就跳过——预热要能中断、能重跑，
/// 而不是每次从头再来一遍。
async fn recently_attempted(db: &sqlx::PgPool, name: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM code_corpus_fetches \
          WHERE ecosystem = 'npm' AND name = $1 AND fetched_at > now() - interval '30 days'",
    )
    .bind(name)
    .fetch_one(db)
    .await
    .map(|n| n > 0)
    .unwrap_or(false)
}

/// 批量预热：发现常用包 → 逐个抽取入库。
///
/// **串行 + 间隔**是刻意的：并发拉 npm 只会更快撞上限流，而这活本来就该慢慢跑在后台。
/// 已经抓过的直接跳过，所以中断之后重跑会接着上次的进度走。
pub async fn seed(db: sqlx::PgPool, per_term: usize, max_packages: usize) -> Result<()> {
    let client = http()?;
    let names = discover_popular(&client, per_term).await;
    tracing::info!(discovered = names.len(), "code corpus: seeding started");
    let mut done = 0usize;
    let mut failed = 0usize;
    for name in names.into_iter().take(max_packages) {
        if recently_attempted(&db, &name).await {
            continue;
        }
        match ingest_npm(&db, &name, None).await {
            Ok(r) => {
                done += 1;
                if done % 50 == 0 {
                    tracing::info!(done, failed, last = %r.name, "code corpus: seeding progress");
                }
            }
            Err(e) => {
                failed += 1;
                record_failure(&db, &name, "", &e.to_string()).await;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    tracing::info!(done, failed, "code corpus: seeding finished");
    Ok(())
}

/// crates.io 有官方的下载量排序，直接按热度翻页。
async fn discover_crates(client: &reqwest::Client, want: usize) -> Vec<String> {
    let mut names = Vec::new();
    let mut page = 1usize;
    while names.len() < want && page <= 100 {
        let url = format!("https://crates.io/api/v1/crates?sort=downloads&per_page=100&page={page}");
        if !host_allowed(&url) { break; }
        let Ok(resp) = client.get(&url).send().await else { break };
        let Ok(body) = resp.json::<serde_json::Value>().await else { break };
        let arr = body.get("crates").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        if arr.is_empty() { break; }
        for c in &arr {
            if let Some(n) = c.get("name").and_then(|v| v.as_str()) {
                if valid_simple_name(n) { names.push(n.to_string()); }
            }
        }
        page += 1;
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
    names.truncate(want);
    names
}

/// PyPI 没有官方的「按下载量排序」接口（BigQuery 那份不算公开 API），
/// 所以这一支用一份**明确的种子名单**打底：常用框架、数据科学、云 SDK、工具链。
/// 名单之外的包仍然会被「按需入库」补进来——预热只是让常用的开箱即有。
const PYPI_SEED: &[&str] = &[
    "requests","urllib3","numpy","pandas","scipy","matplotlib","pillow","pydantic","fastapi",
    "flask","django","starlette","uvicorn","gunicorn","httpx","aiohttp","sqlalchemy","alembic",
    "psycopg2-binary","pymongo","redis","celery","click","typer","rich","tqdm","attrs","cattrs",
    "pytest","hypothesis","mypy","ruff","black","isort","flake8","tox","poetry","setuptools",
    "boto3","botocore","google-cloud-storage","azure-storage-blob","paramiko","cryptography",
    "pyyaml","toml","jinja2","markupsafe","beautifulsoup4","lxml","scrapy","selenium","playwright",
    "openai","anthropic","transformers","torch","tensorflow","scikit-learn","xgboost","lightgbm",
    "polars","pyarrow","duckdb","dask","networkx","sympy","statsmodels","seaborn","plotly",
    "python-dateutil","pytz","arrow","pendulum","orjson","ujson","msgpack","protobuf","grpcio",
    "structlog","loguru","sentry-sdk","prometheus-client","opentelemetry-api","websockets",
];

/// 批量预热：三个生态一起。
///
/// # 为什么是串行 + 间隔
///
/// 用户明确要求「跑的过程中不影响使用」。并发拉包只会更快撞上注册表限流，而且会和真实
/// 请求抢这台机器的出网带宽和 CPU（解包是 CPU 活）。这活本来就该慢慢跑在后台——
/// 一次一个、每个之间歇一下，对前台几乎无感。
///
/// # 为什么能随时中断重跑
///
/// 抓过的（不管成功失败）30 天内不再抓，所以重启之后接着上次的进度走，不会从头再来。
///
/// # 入库即刻可用
///
/// 每个包抽完就 INSERT，检索读的是同一张表——不存在「攒够一批再生效」的窗口。
/// 正在跑的时候搜到刚入库的包，是预期行为。
pub async fn seed_all(db: sqlx::PgPool, npm_per_term: usize, per_eco_max: usize) -> Result<()> {
    let client = http()?;

    let npm = discover_popular(&client, npm_per_term).await;
    let crates = discover_crates(&client, per_eco_max).await;
    let pypi: Vec<String> = PYPI_SEED.iter().map(|s| s.to_string()).collect();
    tracing::info!(
        npm = npm.len(), crates = crates.len(), pypi = pypi.len(),
        "code corpus: seeding discovered candidates"
    );

    let plan: Vec<(Eco, Vec<String>)> = vec![
        (Eco::Npm, npm.into_iter().take(per_eco_max).collect()),
        (Eco::PyPI, pypi),
        (Eco::Crates, crates),
    ];

    for (eco, names) in plan {
        let (mut done, mut failed, mut skipped) = (0usize, 0usize, 0usize);
        for name in names {
            if recently_attempted_eco(&db, eco, &name).await {
                skipped += 1;
                continue;
            }
            match ingest(&db, eco, &name, None).await {
                Ok(r) => {
                    done += 1;
                    if done % 50 == 0 {
                        tracing::info!(eco = eco.as_str(), done, failed, skipped, last = %r.name,
                            "code corpus: seeding progress");
                    }
                }
                Err(e) => {
                    failed += 1;
                    record_failure_eco(&db, eco, &name, "", &e.to_string()).await;
                }
            }
            // 对前台让路。300ms 一个 ≈ 每小时 12000 个，跑满一轮几小时，代价是几乎无感。
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        tracing::info!(eco = eco.as_str(), done, failed, skipped, "code corpus: ecosystem finished");
    }
    tracing::info!("code corpus: seeding finished");
    Ok(())
}

async fn recently_attempted_eco(db: &sqlx::PgPool, eco: Eco, name: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM code_corpus_fetches \
          WHERE ecosystem = $1 AND name = $2 AND fetched_at > now() - interval '30 days'",
    )
    .bind(eco.as_str()).bind(name)
    .fetch_one(db).await.map(|n| n > 0).unwrap_or(false)
}

async fn record_failure_eco(db: &sqlx::PgPool, eco: Eco, name: &str, version: &str, err: &str) {
    let _ = sqlx::query(
        "INSERT INTO code_corpus_fetches (ecosystem, name, version, ok, error) \
         VALUES ($1,$2,$3,false,$4) \
         ON CONFLICT (ecosystem, name, version) DO UPDATE \
           SET ok = false, error = EXCLUDED.error, fetched_at = now()",
    )
    .bind(eco.as_str()).bind(name).bind(version)
    .bind(err.chars().take(500).collect::<String>())
    .execute(db).await;
}

/// 开机后自动开始预热。
///
/// `MICHAEL_CODE_CORPUS_SEED=0` 关掉。默认开：语料库空着对用户没有价值，
/// 而它对前台的影响被上面那条 300ms 间隔压到几乎为零。
pub fn spawn(db: sqlx::PgPool) {
    if std::env::var("MICHAEL_CODE_CORPUS_SEED").ok().as_deref() == Some("0") {
        tracing::info!("code corpus: seeding disabled by MICHAEL_CODE_CORPUS_SEED=0");
        return;
    }
    tokio::spawn(async move {
        // 让迁移和主要初始化先过去，别和启动抢资源。
        tokio::time::sleep(std::time::Duration::from_secs(45)).await;
        if let Err(err) = seed_all(db, 250, 20000).await {
            tracing::warn!(%err, "code corpus: background seeding failed");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_registry_hosts_are_fetchable() {
        // tarball 地址来自注册表返回的 JSON——那是外部输入。不挡就等于让人指使
        // 这台服务器去访问任意内网地址。
        assert!(host_allowed("https://registry.npmjs.org/zod/-/zod-3.0.0.tgz"));
        assert!(host_allowed("https://registry.yarnpkg.com/zod/-/zod-3.0.0.tgz"));
        for bad in [
            "http://registry.npmjs.org/x",          // 明文
            "https://169.254.169.254/latest/meta",  // 云元数据
            "https://localhost/x",
            "https://registry.npmjs.org.evil.com/x",
            "https://registry.npmjs.org:2375/x",    // 自定义端口（docker api）
            "file:///etc/passwd",
        ] {
            assert!(!host_allowed(bad), "must reject {bad}");
        }
    }

    #[test]
    fn package_names_that_could_escape_the_url_are_rejected() {
        for ok in ["zod", "react-dom", "@tanstack/react-query", "lodash.merge"] {
            assert!(valid_npm_name(ok), "{ok} should be valid");
        }
        for bad in ["", "../etc", "a/b/c", "@scope", "@/x", "UPPER", "pkg?x=1", ".hidden"] {
            assert!(!valid_npm_name(bad), "{bad} must be rejected");
        }
    }

    #[test]
    fn declarations_and_their_doc_comments_come_out_of_a_dts() {
        let dts = r#"
/**
 * Parse a value against this schema.
 * Throws when the value does not match.
 */
export declare function parse<T>(schema: Schema<T>, value: unknown): T;

export interface Schema<T> {
    _type: T;
}

declare function notExported(): void;
"#;
        let mut out = Vec::new();
        extract_dts(dts, &mut out);
        let names: Vec<&str> = out.iter().map(|e| e.symbol.as_str()).collect();
        assert!(names.contains(&"parse"), "导出的函数要抽出来：{names:?}");
        assert!(names.contains(&"Schema"), "导出的接口要抽出来：{names:?}");
        assert!(!names.contains(&"notExported"), "没导出的不该进语料");

        let parse = out.iter().find(|e| e.symbol == "parse").unwrap();
        assert!(parse.body.contains("schema: Schema<T>"), "签名要完整：{}", parse.body);
        assert!(parse.body.contains("Throws when the value does not match"),
            "紧贴声明的文档注释要一起收：{}", parse.body);
    }

    #[test]
    fn a_natural_language_query_is_or_joined_so_it_can_actually_recall() {
        // AND 语义是这套检索最容易踩的坑：实测「parse schema」在 zod v4 上一条都召不回，
        // 因为 parse 是 schema 对象上的方法、不是顶层导出，语料里根本没有这个词。
        assert_eq!(or_form("parse schema"), "parse or schema");
        // 标点和单字符不进查询；`$`/`_` 是标识符的一部分，要留。
        assert_eq!(or_form("z.string().min(1)"), "string or min");
        assert_eq!(or_form("$ZodType a"), "$ZodType");
        // 空查询要能安全地退化成「什么都不匹配」，而不是拼出坏语法。
        assert_eq!(or_form("   "), "");
        assert_eq!(or_form("!!! ???"), "");
    }

    #[test]
    fn python_declarations_and_their_docstrings_come_out() {
        // Python 的文档字符串在声明**之后**（缩进的三引号块），和 TS 正好相反——
        // 抽错方向就只剩签名、没有说明。
        let src = "def connect(dsn: str, *, timeout: int = 30) -> Connection:\n    \"\"\"Open a connection.\n\n    Raises TimeoutError when the server does not answer.\n    \"\"\"\n    ...\n\nclass Pool:\n    \"\"\"A pool of connections.\"\"\"\n    pass\n\ndef _private():\n    pass\n";
        let mut out = Vec::new();
        extract_python(src, &mut out);
        let names: Vec<&str> = out.iter().map(|e| e.symbol.as_str()).collect();
        assert!(names.contains(&"connect"), "顶层函数要抽出来：{names:?}");
        assert!(names.contains(&"Pool"), "顶层类要抽出来：{names:?}");
        assert!(!names.contains(&"_private"), "下划线开头是约定的私有，不进语料");
        let c = out.iter().find(|e| e.symbol == "connect").unwrap();
        assert!(c.body.contains("timeout: int = 30"), "签名要完整：{}", c.body);
        assert!(c.body.contains("Raises TimeoutError"), "docstring 要一起收：{}", c.body);
    }

    #[test]
    fn rust_pub_items_and_their_doc_comments_come_out() {
        let src = "/// Opens a store at `path`.\n///\n/// Returns an error when the path is not writable.\n#[inline]\npub fn open(path: &Path) -> Result<Store> {\n    todo!()\n}\n\npub struct Store {\n    inner: u8,\n}\n\nfn private_helper() {}\n";
        let mut out = Vec::new();
        extract_rust(src, &mut out);
        let names: Vec<&str> = out.iter().map(|e| e.symbol.as_str()).collect();
        assert!(names.contains(&"open"), "pub fn 要抽出来：{names:?}");
        assert!(names.contains(&"Store"), "pub struct 要抽出来：{names:?}");
        assert!(!names.contains(&"private_helper"), "非 pub 的不进语料");
        let o = out.iter().find(|e| e.symbol == "open").unwrap();
        assert!(o.body.contains("Returns an error when the path is not writable"),
            "/// 文档要一起收，属性宏不能打断文档块：{}", o.body);
    }

    #[test]
    fn each_ecosystem_only_unpacks_files_that_carry_its_api() {
        assert!(wanted_file_for("dist/index.d.ts", Eco::Npm));
        assert!(wanted_file_for("pkg/types.pyi", Eco::PyPI));
        assert!(wanted_file_for("pkg/client.py", Eco::PyPI));
        assert!(wanted_file_for("foo-1.0/src/lib.rs", Eco::Crates));
        // 测试目录在三个生态里都不是 API 表面。
        assert!(!wanted_file_for("foo/tests/test_x.py", Eco::PyPI));
        assert!(!wanted_file_for("foo-1.0/tests/it.rs", Eco::Crates));
        // 跨生态不串味：.py 不该被 npm 收，.d.ts 不该被 crates 收。
        assert!(!wanted_file_for("pkg/client.py", Eco::Npm));
        assert!(!wanted_file_for("dist/index.d.ts", Eco::Crates));
    }

    #[test]
    fn the_new_registries_are_on_the_allowlist_and_nothing_else_is() {
        for ok in [
            "https://pypi.org/pypi/requests/json",
            "https://files.pythonhosted.org/packages/x/requests-2.tar.gz",
            "https://crates.io/api/v1/crates/serde",
            "https://static.crates.io/crates/serde/serde-1.0.0.crate",
        ] {
            assert!(host_allowed(ok), "{ok} 应当放行");
        }
        for bad in ["https://pypi.org.evil.com/x", "http://crates.io/x", "https://crates.io:8080/x"] {
            assert!(!host_allowed(bad), "{bad} 必须拒绝");
        }
    }

    #[test]
    fn only_api_bearing_files_are_unpacked() {
        assert!(wanted_file("package.json"));
        assert!(wanted_file("README.md"));
        assert!(wanted_file("dist/index.d.ts"));
        // 实现、测试、二进制都不是 API 表面，抽它们只会稀释信噪比、撑大库。
        assert!(!wanted_file("dist/index.js"));
        assert!(!wanted_file("src/__tests__/x.d.ts"));
        assert!(!wanted_file("prebuilds/node.napi.node"));
    }
}
