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
