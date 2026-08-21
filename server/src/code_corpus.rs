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
const MAX_ENTRIES_PER_PACKAGE: usize = 900;
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
        // 官方文档仓库（开源 markdown）的 tarball
        || host == "codeload.github.com"
        // PyPI 下载量排名快照（官方 BigQuery 数据集的第三方月度镜像）
        || host == "raw.githubusercontent.com"
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

impl Entry {
    /// 段内唯一锚点：有符号用符号，没符号（文档节 / README 节）用小节标题。
    ///
    /// 没有它，同一个源的所有无符号条目在唯一索引上撞成一条——实测 react 文档
    /// 970 节只留下 1 条，而且入库计数还报 970（冲突跳过时 sqlx 仍返回 Ok）。
    fn anchor(&self) -> String {
        let a = if self.symbol.is_empty() { &self.title } else { &self.symbol };
        a.chars().take(200).collect()
    }
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
    // 限流是**预期内**的（在白嫖公共注册表），退避重试三次；一次 429 不该让这个包
    // 进台账吃 30 天冷却——实测 crates 就是这么一口气丢掉 2115 个的。
    let meta = get_json_retry(client, &url)
        .await
        .ok_or_else(|| anyhow!("registry metadata unavailable (rate-limited or missing)"))?;

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
            let Ok(mut entry) = entry else { continue };
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
    // 先按重要性排序再抽：条目上限会被排在前面的文件吃光，不排序就等于让 tar 的
    // 内部顺序决定「这个包的哪部分进语料」。
    let mut files = files;
    files.sort_by_key(|(rel, _)| file_priority(rel));
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
            "INSERT INTO code_corpus (kind, ecosystem, name, version, symbol, anchor, title, body, source_url) \
             VALUES ($1,'npm',$2,$3,$4,$5,$6,$7,$8) \
             ON CONFLICT (ecosystem, name, version, kind, anchor) DO NOTHING",
        )
        .bind(e.kind)
        .bind(name)
        .bind(&version)
        .bind(&e.symbol)
        .bind(e.anchor())
        .bind(&e.title)
        .bind(&e.body)
        .bind(&tarball)
        .execute(db)
        .await;
        // 只数**真的写进去了**的。冲突跳过时 sqlx 一样返回 Ok，拿 is_ok() 计数
        // 会让日志报出一个从没发生过的数字（实测报 970、实入 1）。
        written += res.map(|r| r.rows_affected() as usize).unwrap_or(0);
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

/// 这个查询像不像「一个标识符」。
///
/// 决定排序该偏向谁：问 `useEffect` 要的是那个符号的签名；问「useEffect 为什么在开发环境
/// 跑两次」要的是官方文档里讲这件事的那一节。两者在同一个索引里争第一，靠权重分开。
fn looks_like_identifier(query: &str) -> bool {
    let q = query.trim();
    !q.is_empty()
        && q.len() <= 64
        && !q.contains(' ')
        && q.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$' || c == '.' || c == ':')
}

/// 检索。内置全文排相关性；给了包名就把它限定住，避免同名符号跨包串味。
///
/// # 权重是按查询形态换的
///
/// tsvector 里 name/symbol 是 A、title 是 B、body 是 C。这套权重对「查符号」是对的，
/// 对「问概念」是错的——实测问「useEffect 为什么在开发环境跑两次」，一个恰好导出
/// `useEffect` 的第三方小包（symbol 命中，权重 A）压过了 React 官方文档讲这件事的整节
/// 正文（权重 C）。答案在库里，只是排不上来。
///
/// `ts_rank` 的第一个参数就是权重数组 `{D,C,B,A}`，所以不用改索引、也不用拆表：
///   · 标识符查询 → A 主导（符号命中最重要）
///   · 自然语言查询 → C 主导（正文里讲清楚这件事的那一节最重要）
/// trigram 的符号相似度同理，只在标识符查询时加权——拿一整句话去和符号名比相似度是噪音。
pub async fn search(
    db: &sqlx::PgPool,
    query: &str,
    package: Option<&str>,
    limit: i64,
) -> Result<Vec<CorpusHit>> {
    let limit = limit.clamp(1, 20);
    let or_q = or_form(query);
    let ident = looks_like_identifier(query);
    // 查询里点了名的库要优先。
    //
    // 多词查询按正文权重排，而签名条目的正文往往只有一行——于是问
    // 「zustand create store selector」时，zustand 自己的 `create` 反而排不过一段
    // 正文冗长、只是碰巧提到 selector 的无关文档。用户明明把库名说出来了。
    // 取查询里的词去和包名精确比对，命中就加权：这是查询里最强的一个信号。
    let name_tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.' && c != '@' && c != '/')
        .filter(|w| w.len() >= 2)
        .map(|w| w.to_ascii_lowercase())
        .take(12)
        .collect();
    // {D, C, B, A}
    let weights: Vec<f32> = if ident {
        vec![0.1, 0.2, 0.4, 1.0]
    } else {
        vec![0.1, 1.0, 0.5, 0.15]
    };
    let rows: Vec<(String, String, String, String, String, String, f32)> = sqlx::query_as(
        "SELECT ecosystem, name, version, symbol, title, body, \
                (ts_rank($6::float4[], tsv, websearch_to_tsquery('english', $1)) * 2.0 \
                  + ts_rank($6::float4[], tsv, websearch_to_tsquery('english', $5)) \
                  + CASE WHEN $7 AND symbol <> '' THEN similarity(symbol, $2) * 2.0 ELSE 0 END \
                  + CASE WHEN lower(name) = ANY($8) THEN 3.0 ELSE 0 END)::real AS score \
           FROM code_corpus \
          WHERE ($3 = '' OR name = $3) \
            AND ($5 <> '' AND tsv @@ websearch_to_tsquery('english', $5) \
                 OR ($7 AND symbol <> '' AND symbol % $2)) \
          ORDER BY score DESC, length(body) ASC \
          LIMIT $4",
    )
    .bind(query)
    .bind(query)
    .bind(package.unwrap_or(""))
    .bind(limit)
    .bind(&or_q)
    .bind(&weights)
    .bind(ident)
    .bind(&name_tokens)
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
pub async fn have_package(db: &sqlx::PgPool, eco: Eco, name: &str) -> bool {
    // 生态必须带上。写死 'npm' 的后果实测过：问 PyPI 的 pandas 时它答「没有」，
    // 于是按默认生态去 npm 现拉，拉回来的是 npm 上那个占坑的 pandas@0.0.3，
    // 而真正的 pypi/pandas@3.0.5 就在库里躺着。
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM code_corpus WHERE ecosystem = $1 AND name = $2",
    )
    .bind(eco.as_str())
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
    let mut files = files;
    files.sort_by_key(|(rel, _)| file_priority(rel));
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
            let Ok(mut entry) = entry else { continue };
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
            "INSERT INTO code_corpus (kind, ecosystem, name, version, symbol, anchor, title, body, source_url) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) \
             ON CONFLICT (ecosystem, name, version, kind, anchor) DO NOTHING",
        )
        .bind(e.kind).bind(eco.as_str()).bind(name).bind(version)
        .bind(&e.symbol).bind(e.anchor()).bind(&e.title).bind(&e.body).bind(url)
        .execute(db).await;
        written += res.map(|r| r.rows_affected() as usize).unwrap_or(0);
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
    let meta = get_json_retry(client, &url)
        .await
        .ok_or_else(|| anyhow!("pypi metadata unavailable (rate-limited or missing)"))?;
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
    let meta = get_json_retry(client, &url)
        .await
        .ok_or_else(|| anyhow!("crates metadata unavailable (rate-limited or missing)"))?;
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

/// 抽取顺序的优先级（越小越先抽）。
///
/// # 为什么必须排序
///
/// 每包有条目上限（防一个巨型 SDK 灌满整张表），而抽取是按 tar 里的文件顺序走的——
/// 于是**上限被排在前面的文件吃光，真正的主 API 根本轮不到**。
///
/// 实测 pandas：382 条全是 `_libs/tslibs/*.pyi` 里的底层helper
/// （abbrev_to_npy_unit、periods_per_second…），而 `DataFrame` 一条都没有。
/// 「包在库里、但它最核心的 API 查不到」比没收录更糟——查的人会以为这个库收全了。
///
/// 判据只用路径形状，不猜语义：
///   · 入口文件最优先（index.d.ts / lib.rs / __init__.py / main）；
///   · 层级越浅越靠前（公开 API 通常在浅层，内部实现在深层）；
///   · 下划线开头的目录段、internal/、vendor/ 往后压——各语言都用这个约定表示「内部」。
fn file_priority(rel: &str) -> i32 {
    let lower = rel.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    let mut score = 0i32;

    // README 先抽：包级说明是最便宜的上下文。
    if file.starts_with("readme") {
        return -100;
    }
    // 入口文件：一个包的公开面基本都从这儿开始。
    if matches!(file, "index.d.ts" | "lib.rs" | "__init__.py" | "__init__.pyi" | "main.rs" | "mod.rs")
    {
        score -= 40;
    }
    // 层级：每深一层退一点。公开 API 在浅层是三个生态共同的惯例。
    score += 6 * lower.matches('/').count() as i32;
    // 内部/私有约定：下划线开头的目录段、internal、vendor、dist 里的打包产物。
    for seg in lower.split('/') {
        if seg.starts_with('_') && seg != "__init__.py" && seg != "__init__.pyi" {
            score += 60;
        }
        if seg == "internal" || seg == "vendor" || seg == "_vendor" || seg == "priv" {
            score += 60;
        }
    }
    score
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
    // 第二批种子词：把发现面从 ~3000 铺宽。仍然按**生态切面**选词而不是具体库名——
    // 这样命中的是每个方向的头部包，而不是某一家的全家桶。
    "solid", "qwik", "remix", "astro", "nest", "fastify", "koa", "hono", "trpc",
    "prisma", "drizzle", "sequelize", "typeorm", "knex", "sqlite", "mysql", "elasticsearch",
    "kafka", "rabbitmq", "grpc-web", "protobuf", "openapi", "swagger", "zod", "joi", "yup",
    "rxjs", "immer", "zustand", "redux", "mobx", "jotai", "recoil", "signals",
    "storybook", "cypress", "playwright", "puppeteer", "selenium", "mock", "faker",
    "monorepo", "turbo", "nx", "lerna", "changesets", "semver", "commander", "yargs",
    "inquirer", "chalk", "ora", "boxen", "dotenv", "cron", "scheduler", "worker",
    "canvas", "webgl", "three", "d3", "leaflet", "mapbox", "audio", "ffmpeg",
    "compression", "archive", "upload", "s3", "oauth", "passport", "bcrypt", "helmet",
    "rate-limit", "cors", "proxy", "ssr", "hydration", "islands", "pwa", "service-worker",
    "wasm", "napi", "ffi", "electron", "tauri", "capacitor", "expo", "react-native",
    "accessibility", "aria", "intl", "currency", "decimal", "uuid", "nanoid", "hash",
];

/// 从 npm 官方搜索接口按流行度收集包名。
async fn discover_popular(client: &reqwest::Client, per_term: usize) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut names: BTreeSet<String> = BTreeSet::new();
    // 被限流截断的种子词数。少收是可以接受的，**不知道自己少收了**不行。
    let mut truncated = 0usize;
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
            let Some(body) = get_json_retry(client, &url).await else {
                truncated += 1;
                break;
            };
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
    if truncated > 0 {
        tracing::warn!(truncated, total = SEED_TERMS.len(),
            "code corpus: npm discovery was rate-limited on some seed terms; coverage is short by that much");
    }
    names.into_iter().collect()
}

/// 文档仓库的下载超时。
///
/// 不是 30 秒能下完的量级：mdn/content 是 53 MB，实测从境外拉要 4 分钟。
/// 上一版整套用同一个 30 秒超时，于是 MDN **每次都在下载途中被掐**，
/// 台账里记的是「read docs tarball」——看着像网络抖动，其实是我自己的超时。
const DOC_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(900);

/// 流式解包时允许流过的总字节数。
///
/// 和「整份读进内存」的上限不是一回事：这里的字节是**流过**，不是**占住**——
/// 内存只被下面那个有界通道占着（≈8 MB），所以这个数可以给得大方些。
const MAX_DOC_STREAM_BYTES: u64 = 400 * 1024 * 1024;

/// 把异步字节流桥接成同步 `Read`。
///
/// tar 和 flate2 都是同步接口，而 reqwest 给的是异步流。整份 `.bytes().await` 下来最简单，
/// 但那意味着 195 MB 的 tailwind 要**整个占住内存**才能开始解包。
/// 用一个有界通道把两边接起来：下载在异步任务里推，解包在 spawn_blocking 里拉，
/// 内存占用就被通道容量钉死，和仓库大小无关。
struct ChannelReader {
    rx: std::sync::mpsc::Receiver<std::io::Result<bytes::Bytes>>,
    cur: bytes::Bytes,
    pos: usize,
}

impl std::io::Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.pos < self.cur.len() {
                let n = (self.cur.len() - self.pos).min(buf.len());
                buf[..n].copy_from_slice(&self.cur[self.pos..self.pos + n]);
                self.pos += n;
                return Ok(n);
            }
            match self.rx.recv() {
                Ok(Ok(chunk)) => {
                    self.cur = chunk;
                    self.pos = 0;
                }
                Ok(Err(e)) => return Err(e),
                // 发送端关闭 = 流正常结束。tar 见到 EOF 自己收尾。
                Err(_) => return Ok(0),
            }
        }
    }
}

/// 流式下载 + 解包，只留下 `prefix` 下的 markdown。
///
/// 返回 (文件列表, 流过的字节数)。内存占用与仓库大小无关——由通道容量决定。
async fn stream_docs_tarball(
    client: &reqwest::Client,
    url: &str,
    prefix: &str,
) -> Result<(Vec<(String, String)>, u64)> {
    use futures_util::StreamExt;

    if !host_allowed(url) {
        return Err(anyhow!("doc host not allowed"));
    }
    let resp = client
        .get(url).send().await.context("start docs download")?
        .error_for_status().context("docs download returned an error status")?;

    // 容量 8：每块通常 8~64 KB，所以驻留内存是几百 KB 级，和 195 MB 的仓库无关。
    let (tx, rx) = std::sync::mpsc::sync_channel::<std::io::Result<bytes::Bytes>>(8);
    let counted = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let counted_tx = counted.clone();

    let pump = tokio::spawn(async move {
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(b) => {
                    let total = counted_tx
                        .fetch_add(b.len() as u64, std::sync::atomic::Ordering::Relaxed)
                        + b.len() as u64;
                    if total > MAX_DOC_STREAM_BYTES {
                        let _ = tx.send(Err(std::io::Error::other("docs tarball over the stream cap")));
                        return;
                    }
                    // 接收端提前收工（解包出错/够了）时 send 会失败——停下即可。
                    if tx.send(Ok(b)).is_err() {
                        return;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(std::io::Error::other(e.to_string())));
                    return;
                }
            }
        }
    });

    let prefix_owned = prefix.to_string();
    let files = tokio::task::spawn_blocking(move || -> Result<Vec<(String, String)>> {
        let reader = ChannelReader { rx, cur: bytes::Bytes::new(), pos: 0 };
        let gz = flate2::read::GzDecoder::new(reader);
        let mut archive = tar::Archive::new(gz);
        let mut out = Vec::new();
        for entry in archive.entries().context("read docs tar")? {
            // 一个畸形条目不该把整份已经下载完的仓库作废。
            // 实测 prisma/docs 里就有这么一条，`?` 让整个源报 "bad docs tar entry"、
            // 457 节的 tailwind 能成而它 0 节——而问题只出在其中一个条目上。
            let Ok(mut entry) = entry else { continue };
            if !entry.header().entry_type().is_file() { continue; }
            let path = entry.path().context("bad docs entry path")?.to_string_lossy().to_string();
            // GitHub tarball 顶层是 `<repo>-<ref>/`，剥掉再比路径。
            let rel = match path.split_once('/') { Some((_, r)) => r.to_string(), None => continue };
            if !rel.starts_with(&prefix_owned) { continue; }
            let lower = rel.to_ascii_lowercase();
            if !(lower.ends_with(".md") || lower.ends_with(".mdx")) { continue; }
            if entry.header().size().unwrap_or(0) as usize > MAX_ENTRY_BYTES { continue; }
            let mut buf = String::new();
            if std::io::Read::read_to_string(&mut entry, &mut buf).is_err() { continue; }
            out.push((rel, buf));
            if out.len() > MAX_DOC_FILES { break; }
        }
        Ok(out)
    })
    .await
    .context("docs untar task panicked")??;

    // 解包可能提前 break，pump 会因为 send 失败自己退出；这里只是收尸。
    pump.abort();
    let bytes = counted.load(std::sync::atomic::Ordering::Relaxed);
    Ok((files, bytes))
}

/// 一个文档源最多取多少个 markdown 文件。MDN 有好几万个，全收会把这张表灌爆。
const MAX_DOC_FILES: usize = 12000;


/// 官方文档源。
///
/// # 为什么是「文档」而不是「全网网页」
///
/// 抓整个互联网既做不到也没价值：模型缺的不是网页，是**权威且当下的事实**。
/// 官方文档恰好是这份东西的最高密度形态，而且这些仓库本身就是开源 markdown，
/// 可以整份取下来按节切开——不用爬、不用渲染、不用猜正文在哪。
///
/// # 选源标准
///
/// 只收**开源许可、以 markdown 形态发布**的官方文档。每一条都记下 source_url，
/// 检索结果能一路追回原文出处（这些内容各有各的许可，出处必须留着）。
///
/// (语料名, 仓库, 分支, 只取这个路径下的)
/// MDN 和 Node 现在收得进来了：抽取改成流式之后，内存占用与仓库大小无关，
/// 而且文档专用的长超时不会再把大仓掐在半路（实测 mdn/content 53 MB、要几分钟）。
const DOC_SOURCES: &[(&str, &str, &str, &str)] = &[
    ("react",      "reactjs/react.dev",           "main", "src/content/"),
    ("vue",        "vuejs/docs",                  "main", "src/"),
    ("svelte",     "sveltejs/svelte",             "main", "documentation/"),
    ("rust",       "rust-lang/book",              "main", "src/"),
    ("typescript", "microsoft/TypeScript-Website","v2",   "packages/documentation/copy/en/"),
    ("tailwind",   "tailwindlabs/tailwindcss.com","main", "src/docs/"),
    ("astro",      "withastro/docs",              "main", "src/content/docs/en/"),
    // 正文在 apps/docs/content/，不是根下的 content/ —— 上一版路径写错了，
    // 于是就算下载成功也一个文件都匹配不到。
    ("prisma",     "prisma/docs",                 "main", "apps/docs/content/"),
    ("fastapi",    "fastapi/fastapi",             "master", "docs/en/docs/"),
    // Web 平台那一整块：JS / CSS / HTML / HTTP / Web API。前端问题里占比最大的一块，
    // 而且是唯一权威出处。files/ 下是示例资源，只取 web/ 的正文。
    ("mdn",        "mdn/content",                 "main", "files/en-us/web/"),
    // Node 官方 API 文档。整仓是 Node 源码，流式解包只留 doc/api/ 那几十个 markdown。
    ("node",       "nodejs/node",                 "main", "doc/api/"),
    // 第二批：路径逐个核实过再加。写错路径的代价是「下载成功但一个文件都匹配不到」，
    // 而台账只会记 ok=true entries=0 —— 看着像收录了，其实是空的（prisma 踩过一次）。
    ("next",           "vercel/next.js",        "canary", "docs/"),
    ("nuxt",           "nuxt/nuxt",             "main",   "docs/"),
    ("angular",        "angular/angular",       "main",   "adev/src/content/"),
    ("vite",           "vitejs/vite",           "main",   "docs/"),
    ("playwright",     "microsoft/playwright",  "main",   "docs/src/"),
    ("kubernetes",     "kubernetes/website",    "main",   "content/en/docs/"),
    ("docker",         "docker/docs",           "main",   "content/"),
    ("supabase",       "supabase/supabase",     "master", "apps/docs/content/"),
    ("tanstack-query", "TanStack/query",        "main",   "docs/"),
    ("bun",            "oven-sh/bun",           "main",   "docs/"),
    ("deno",           "denoland/docs",         "main",   "runtime/"),
    ("go",             "golang/website",        "master", "_content/doc/"),
];

/// markdown 按 `##` 切节 —— 和手写语料库同一套切法，切出来的每一节都能独立读懂。
fn extract_markdown(path: &str, src: &str, out: &mut Vec<Entry>) {
    // front-matter 里常有真正的标题，比文件名好看。
    let mut doc_title = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".mdx")
        .trim_end_matches(".md")
        .to_string();
    let body_src = if let Some(rest) = src.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            for line in rest[..end].lines() {
                if let Some(t) = line.strip_prefix("title:") {
                    doc_title = t.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            }
            &rest[end + 5..]
        } else { src }
    } else { src };

    let mut section = String::new();
    let mut heading = doc_title.clone();
    let mut push = |heading: &str, body: &str, out: &mut Vec<Entry>| {
        let body = body.trim();
        // 太短的节（只有一个链接、一张图）没有检索价值。
        if body.len() < 80 || out.len() >= MAX_DOC_SECTIONS {
            return;
        }
        let body = if body.chars().count() > MAX_BODY_CHARS {
            body.chars().take(MAX_BODY_CHARS).collect()
        } else {
            body.to_string()
        };
        out.push(Entry {
            kind: "doc",
            symbol: String::new(),
            title: format!("{doc_title} · {heading}"),
            body,
        });
    };
    for line in body_src.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            push(&heading, &section, out);
            section.clear();
            heading = h.trim().to_string();
        } else {
            section.push_str(line);
            section.push('\n');
        }
    }
    push(&heading, &section, out);
}

/// 一份文档最多收多少节。防一个巨型文档站把整张表灌满。
const MAX_DOC_SECTIONS: usize = 4000;

/// 抓一份官方文档并入库。
pub async fn ingest_doc_source(
    db: &sqlx::PgPool,
    slug: &str,
    repo: &str,
    git_ref: &str,
    prefix: &str,
) -> Result<IngestReport> {
    let url = format!("https://codeload.github.com/{repo}/tar.gz/refs/heads/{git_ref}");
    // 文档仓单独一个客户端：包那套 30 秒超时下不完一份文档站。
    // mdn/content 是 53 MB，实测拉一次要几分钟——上一版就是被自己的超时掐掉的，
    // 而台账里记成「read docs tarball」，看着像网络抖动。
    let client = reqwest::Client::builder()
        .timeout(DOC_HTTP_TIMEOUT)
        .user_agent("michael-ide-code-corpus/1.0")
        .build()
        .context("build docs http client")?;
    // 流式解包：内存占用由通道容量决定，和仓库大小无关。
    let (files, downloaded) = stream_docs_tarball(&client, &url, prefix).await?;

    let mut written = 0usize;
    for (rel, body) in &files {
        let mut entries: Vec<Entry> = Vec::new();
        extract_markdown(rel, body, &mut entries);
        // 出处要能一路追回原文——这些内容各有各的许可，来源不能丢。
        let src_url = format!("https://github.com/{repo}/blob/{git_ref}/{rel}");
        for e in &entries {
            let res = sqlx::query(
                "INSERT INTO code_corpus (kind, ecosystem, name, version, symbol, anchor, title, body, source_url) \
                 VALUES ($1,'docs',$2,$3,'',$4,$5,$6,$7) \
                 ON CONFLICT (ecosystem, name, version, kind, anchor) DO NOTHING",
            )
            .bind(e.kind).bind(slug).bind(git_ref)
            // 文档的锚点要带上路径：不同文件里同名的 `## Usage` 是不同的节。
            .bind(format!("{rel}#{}", e.anchor()).chars().take(200).collect::<String>())
            .bind(&e.title).bind(&e.body).bind(&src_url)
            .execute(db).await;
            written += res.map(|r| r.rows_affected() as usize).unwrap_or(0);
        }
    }

    let _ = sqlx::query(
        "INSERT INTO code_corpus_fetches (ecosystem, name, version, ok, entries, bytes) \
         VALUES ('docs',$1,$2,true,$3,$4) \
         ON CONFLICT (ecosystem, name, version) DO UPDATE \
           SET ok = true, entries = EXCLUDED.entries, bytes = EXCLUDED.bytes, error = NULL, fetched_at = now()",
    )
    .bind(slug).bind(git_ref).bind(written as i32).bind(downloaded as i64)
    .execute(db).await;

    Ok(IngestReport { name: slug.to_string(), version: git_ref.to_string(), entries: written, bytes: downloaded })
}

/// 把所有官方文档源过一遍。
pub async fn seed_docs(db: &sqlx::PgPool) {
    for (slug, repo, git_ref, prefix) in DOC_SOURCES {
        if recently_attempted_eco_named(db, "docs", slug).await {
            continue;
        }
        match ingest_doc_source(db, slug, repo, git_ref, prefix).await {
            Ok(r) => tracing::info!(source = %r.name, sections = r.entries, bytes = r.bytes,
                "code corpus: doc source ingested"),
            Err(e) => {
                let msg = e.to_string();
                tracing::warn!(source = %slug, error = %msg, "code corpus: doc source failed");
                let _ = sqlx::query(
                    "INSERT INTO code_corpus_fetches (ecosystem, name, version, ok, error) \
                     VALUES ('docs',$1,$2,false,$3) \
                     ON CONFLICT (ecosystem, name, version) DO UPDATE \
                       SET ok = false, error = EXCLUDED.error, fetched_at = now()",
                )
                .bind(slug).bind(git_ref).bind(msg.chars().take(500).collect::<String>())
                .execute(db).await;
            }
        }
        // 对 GitHub 客气一点。
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

/// 这个源近期处理过吗。
///
/// 成功的 14 天不重来；**失败的也要退避 3 天**——否则一个永远太大的源会在每次重启时
/// 重新下载几十 MB 再被上限拒掉，白烧带宽。3 天比 14 天短，是给临时故障留重试的机会。
async fn recently_attempted_eco_named(db: &sqlx::PgPool, eco: &str, name: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM code_corpus_fetches \
          WHERE ecosystem = $1 AND name = $2 \
            AND fetched_at > now() - (CASE WHEN ok THEN interval '14 days' ELSE interval '3 days' END)",
    )
    .bind(eco).bind(name)
    .fetch_one(db).await.map(|n| n > 0).unwrap_or(false)
}

/// 带重试的 JSON GET。
///
/// 发现阶段原来是「一失败就 break」——注册表限流时那一整个种子词/那一页之后的全部丢掉，
/// **而且完全静默**：日志上只看到候选数变少，看不出是被限流截断的。实测 npm 只发现 2469、
/// crates 只发现 2600（预期各上万），就是这么少的。
///
/// 限流是**预期内**的（我们在白嫖别人的公共接口），所以按预期处理：退避重试，
/// 三次都不行才放弃这一页，并且如实计数、最后报出来。
async fn get_json_retry(client: &reqwest::Client, url: &str) -> Option<serde_json::Value> {
    for attempt in 0..3u32 {
        if attempt > 0 {
            // 1s → 4s。限流恢复通常只要几秒，不值得等更久。
            tokio::time::sleep(std::time::Duration::from_millis(1000 * (1 << (2 * (attempt - 1))))).await;
        }
        let Ok(resp) = client.get(url).send().await else { continue };
        // 429/5xx 值得重试；4xx（包名不存在之类）不值得，直接放弃。
        let status = resp.status();
        if status.is_client_error() && status.as_u16() != 429 {
            return None;
        }
        if !status.is_success() {
            continue;
        }
        if let Ok(v) = resp.json::<serde_json::Value>().await {
            return Some(v);
        }
    }
    None
}

/// crates.io 有官方的下载量排序，直接按热度翻页。
async fn discover_crates(client: &reqwest::Client, want: usize) -> Vec<String> {
    let mut names = Vec::new();
    let mut page = 1usize;
    while names.len() < want && page <= 100 {
        let url = format!("https://crates.io/api/v1/crates?sort=downloads&per_page=100&page={page}");
        if !host_allowed(&url) { break; }
        let Some(body) = get_json_retry(client, &url).await else {
            tracing::warn!(page, collected = names.len(),
                "code corpus: crates discovery stopped early (rate-limited)");
            break;
        };
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

/// PyPI 的兜底种子名单。
///
/// 正常走 `discover_pypi`（真实下载量排名，15000 个）；那份取不到时才用这个，
/// 保证离线或数据源挂掉时仍有常用包可收。
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

/// PyPI 按真实下载量排名的包名。
///
/// PyPI **不提供**按下载量排序的公开接口——官方数据在 Google BigQuery 的公共数据集里，
/// 要凭证才能查。`hugovk/top-pypi-packages` 是从那份官方数据集每月生成的快照，
/// 是这件事实际上的标准来源。**出处要说清楚**：它是官方数据的第三方镜像，不是官方接口；
/// 取不到时回落到本文件里那份手写种子名单，预热照常进行，只是覆盖窄一些。
async fn discover_pypi(client: &reqwest::Client, want: usize) -> Vec<String> {
    let url = "https://raw.githubusercontent.com/hugovk/top-pypi-packages/main/top-pypi-packages.json";
    let fallback = || -> Vec<String> {
        tracing::warn!("code corpus: pypi ranking unavailable, falling back to the seed list");
        PYPI_SEED.iter().map(|s| s.to_string()).collect()
    };
    if !host_allowed(url) {
        return fallback();
    }
    let Ok(resp) = client.get(url).send().await else { return fallback() };
    let Ok(body) = resp.json::<serde_json::Value>().await else { return fallback() };
    let rows = body
        .get("rows")
        .or_else(|| body.get("data"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let names: Vec<String> = rows
        .iter()
        .filter_map(|r| r.get("project").and_then(|v| v.as_str()))
        .filter(|n| valid_simple_name(n))
        .map(|n| n.to_string())
        .take(want)
        .collect();
    if names.is_empty() { fallback() } else { names }
}

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
    let pypi = discover_pypi(&client, per_eco_max).await;
    tracing::info!(
        npm = npm.len(), crates = crates.len(), pypi = pypi.len(),
        "code corpus: seeding discovered candidates"
    );

    // 文档先跑：十几份官方文档就覆盖了最常问的框架，比几千个包更快见效，
    // 而且它是「全网知识库」那一半的主体。
    seed_docs(&db).await;

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
            // 对前台让路，也对上游客气。crates.io 的限流明显比 npm 严
            // （实测一口气 429 了 2115 个），单独放慢一档。
            let pace = if eco == Eco::Crates { 900 } else { 300 };
            tokio::time::sleep(std::time::Duration::from_millis(pace)).await;
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
    fn every_doc_source_declares_a_path_that_could_actually_match() {
        // 路径写错的代价是**静默的**：下载成功、一个文件都匹配不到，台账记 ok=true entries=0，
        // 看着像收录了其实是空的。prisma 就这么空过一轮（正文在 apps/docs/content/，
        // 而清单里写的是根下的 content/）。这里只能钉住形状，真实性靠上线后看 entries。
        assert!(DOC_SOURCES.len() >= 20, "文档源太少了：{}", DOC_SOURCES.len());
        let mut seen = std::collections::HashSet::new();
        for (slug, repo, git_ref, prefix) in DOC_SOURCES {
            assert!(seen.insert(*slug), "文档源 slug 重复：{slug}");
            assert!(repo.contains('/') && !repo.starts_with('/'), "{slug}: 仓库名要是 owner/name");
            assert!(!git_ref.is_empty(), "{slug}: 缺分支");
            assert!(
                prefix.ends_with('/') && !prefix.starts_with('/'),
                "{slug}: 路径前缀必须以 / 结尾、不以 / 开头（要和 tar 里剥掉顶层后的相对路径对齐），实际 {prefix:?}"
            );
        }
    }

    #[test]
    fn pypi_falls_back_to_the_seed_list_rather_than_collecting_nothing() {
        // 排名数据是官方 BigQuery 数据集的第三方月度镜像——不是官方接口，会挂。
        // 挂了要退回种子名单继续预热，而不是让整个 PyPI 支收零个。
        assert!(PYPI_SEED.len() >= 50, "兜底名单太短：{}", PYPI_SEED.len());
        assert!(PYPI_SEED.iter().all(|n| valid_simple_name(n)), "兜底名单里有非法包名");
        for must in ["requests", "numpy", "fastapi", "django"] {
            assert!(PYPI_SEED.contains(&must), "兜底名单缺了 {must}");
        }
        let src = include_str!("code_corpus.rs");
        let prod = &src[..src.find("mod tests").expect("tests module")];
        assert_eq!(prod.matches("fallback()").count(), 4,
            "discover_pypi 的每条失败路径都要回落，不能有一条直接返回空");
        // 逐包抓取也必须走重试：只给发现阶段加重试时，crates 一口气 429 了 2115 个，
        // 而它们随即进台账吃 30 天冷却——一次限流换来一个月的覆盖缺口。
        assert_eq!(prod.matches("get_json_retry(client, &url)").count(), 5,
            "三个生态的元数据抓取 + npm/crates 两处发现，五处都要带退避重试");
    }

    #[test]
    fn the_public_api_is_extracted_before_internal_helpers() {
        // 实测事故：pandas 收了 382 条，全是 _libs/tslibs/*.pyi 里的底层 helper，
        // 而 DataFrame 一条都没有——每包条目上限被 tar 里排在前面的内部存根吃光了。
        // 「包在库里、但最核心的 API 查不到」比没收录更糟：查的人会以为它收全了。
        let mut files = vec![
            "pandas/_libs/tslibs/timedeltas.pyi",
            "pandas/core/frame.py",
            "pandas/__init__.py",
            "README.md",
            "pandas/core/internals/blocks.py",
        ];
        files.sort_by_key(|f| file_priority(f));
        assert_eq!(files[0], "README.md", "包级说明最先");
        assert_eq!(files[1], "pandas/__init__.py", "入口文件排在实现之前");
        assert!(
            files.iter().position(|f| *f == "pandas/core/frame.py").unwrap()
                < files.iter().position(|f| f.contains("_libs")).unwrap(),
            "公开 API 必须排在下划线内部目录之前：{files:?}"
        );
        assert!(
            files.iter().position(|f| *f == "pandas/core/frame.py").unwrap()
                < files.iter().position(|f| f.contains("internals")).unwrap(),
            "internal/ 段要往后压：{files:?}"
        );

        // 三个生态的入口文件都要享受同一条优先级。
        for entry in ["index.d.ts", "lib.rs", "__init__.py"] {
            assert!(file_priority(entry) < file_priority("a/b/c/deep.rs"),
                "{entry} 应当优先于深层文件");
        }

        // 三条抽取路径都要先排序再抽，漏一条就会以完全一样的方式静默抽错东西。
        let src = include_str!("code_corpus.rs");
        let prod = &src[..src.find("mod tests").expect("tests module")];
        assert_eq!(prod.matches("files.sort_by_key(|(rel, _)| file_priority(rel));").count(), 2,
            "包的两条抽取路径（npm / pypi+crates）都要先排序");
    }

    #[test]
    fn one_malformed_tar_entry_must_not_discard_the_whole_archive() {
        // 实测 prisma/docs 里有一个 tar 条目会让 `?` 直接中断——结果是整份已经下载完的
        // 仓库 0 节入库，而问题只出在其中一个条目上。同一轮里 tailwind 靠流式解包成功
        // 收了 457 节，prisma 却因为这一条全废。
        let src = include_str!("code_corpus.rs");
        let prod = &src[..src.find("mod tests").expect("tests module")];
        assert!(!prod.contains(r#"entry.context("bad tar entry")?"#),
            "坏条目不许中断整份归档（npm 路径）");
        assert!(!prod.contains(r#"entry.context("bad docs tar entry")?"#),
            "坏条目不许中断整份归档（文档路径）");
        // 三条解包路径（npm tar / 通用 tar / 文档流式）都要跳过而不是中断。
        assert_eq!(prod.matches("let Ok(mut entry) = entry else { continue };").count(), 3,
            "三条解包路径都要对坏条目容错");
    }

    #[test]
    fn the_stream_bridge_reassembles_chunks_and_reports_errors() {
        use std::io::Read;
        // tar/flate2 是同步接口，reqwest 给的是异步流。桥接错了会静默地把包解坏，
        // 所以这里把三件事钉死：跨块拼接、EOF、错误传播。
        let (tx, rx) = std::sync::mpsc::sync_channel::<std::io::Result<bytes::Bytes>>(4);
        tx.send(Ok(bytes::Bytes::from_static(b"hello "))).unwrap();
        tx.send(Ok(bytes::Bytes::from_static(b"world"))).unwrap();
        drop(tx);
        let mut r = ChannelReader { rx, cur: bytes::Bytes::new(), pos: 0 };
        let mut s = String::new();
        r.read_to_string(&mut s).unwrap();
        assert_eq!(s, "hello world", "跨块内容必须原样拼回来");

        // 一次 read 只给一小段时，剩下的要留着下次接着给（不能丢）。
        let (tx, rx) = std::sync::mpsc::sync_channel::<std::io::Result<bytes::Bytes>>(4);
        tx.send(Ok(bytes::Bytes::from_static(b"abcdef"))).unwrap();
        drop(tx);
        let mut r = ChannelReader { rx, cur: bytes::Bytes::new(), pos: 0 };
        let mut two = [0u8; 2];
        assert_eq!(r.read(&mut two).unwrap(), 2);
        assert_eq!(&two, b"ab");
        let mut rest = String::new();
        r.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "cdef", "块内剩余部分不能丢");

        // 发送端报错要变成 Read 错误，而不是被当成正常 EOF —— 否则半份包会被当完整包解。
        let (tx, rx) = std::sync::mpsc::sync_channel::<std::io::Result<bytes::Bytes>>(4);
        tx.send(Ok(bytes::Bytes::from_static(b"partial"))).unwrap();
        tx.send(Err(std::io::Error::other("boom"))).unwrap();
        drop(tx);
        let mut r = ChannelReader { rx, cur: bytes::Bytes::new(), pos: 0 };
        let mut buf = Vec::new();
        assert!(r.read_to_end(&mut buf).is_err(), "中途出错必须报错，不能当 EOF");
    }

    #[test]
    fn identifier_queries_and_prose_questions_rank_by_different_weights() {
        // 查符号：一个词、没有空格。
        assert!(looks_like_identifier("useEffect"));
        assert!(looks_like_identifier("z.string"));
        assert!(looks_like_identifier("std::vec::Vec"));
        // 问概念：多个词。拿一整句话去和符号名比相似度是噪音，也不该让 symbol 的
        // 权重 A 压过正文——实测就是这么让一个第三方小包压过 React 官方文档的。
        assert!(!looks_like_identifier("why does useEffect run twice"));
        assert!(!looks_like_identifier(""));
        assert!(!looks_like_identifier("a".repeat(80).as_str()));

        // 权重必须真的按形态换，而且 trigram 只在标识符查询时参与。
        let src = include_str!("code_corpus.rs");
        let prod = &src[..src.find("mod tests").expect("tests module")];
        assert!(prod.contains("ts_rank($6::float4[], tsv"),
            "排序必须用可切换的权重数组，而不是默认权重");
        assert!(prod.contains("CASE WHEN $7 AND symbol <> '' THEN similarity"),
            "符号相似度只该在标识符查询时加权");
        // 用户把库名说出来了，那是查询里最强的信号——不加权就会被冗长的无关正文压过去。
        assert!(prod.contains("CASE WHEN lower(name) = ANY($8) THEN 3.0"),
            "查询里点名的库必须加权，否则问 zustand 排不出 zustand");
    }

    #[test]
    fn sectioned_entries_get_distinct_anchors_or_they_collapse_to_one_row() {
        // 唯一索引里无符号条目全靠锚点区分。少了它，同一个源的几百节撞成一条——
        // 实测 react 文档抽出 970 节、实际入库 1 条，而日志还报 970（冲突跳过时
        // sqlx 一样返回 Ok，用 is_ok() 计数就会报出一个没发生过的数字）。
        let a = Entry { kind: "doc", symbol: String::new(), title: "useEffect · Reference".into(), body: "x".into() };
        let b = Entry { kind: "doc", symbol: String::new(), title: "useEffect · Caveats".into(), body: "y".into() };
        assert_ne!(a.anchor(), b.anchor(), "同一文档的两节必须有不同锚点");
        assert!(!a.anchor().is_empty(), "锚点不能为空，空的就等于没分开");

        // 有符号的仍然以符号为锚——符号名比标题稳定。
        let f = Entry { kind: "package_api", symbol: "parse".into(), title: "function parse".into(), body: "z".into() };
        assert_eq!(f.anchor(), "parse");

        // 超长标题要截断，否则撑爆索引键。
        let long = Entry { kind: "doc", symbol: String::new(), title: "标".repeat(500), body: "z".into() };
        assert!(long.anchor().chars().count() <= 200);

        // 生产里三条 INSERT 都必须按锚点冲突、并且按真实影响行数计。
        let src = include_str!("code_corpus.rs");
        let prod = &src[..src.find("mod tests").expect("tests module")];
        assert_eq!(prod.matches("ON CONFLICT (ecosystem, name, version, kind, anchor)").count(), 3,
            "三条写入路径都要按锚点去重");
        assert!(!prod.contains("if res.is_ok() { written += 1; }"),
            "不许再用 is_ok() 当入库计数——冲突跳过时它也是 Ok");
        assert_eq!(prod.matches("rows_affected() as usize").count(), 3,
            "三条写入路径都要按真实影响行数计");
    }
    #[test]
    fn markdown_is_chunked_by_heading_and_keeps_its_front_matter_title() {
        let src = "---\ntitle: \"useEffect\"\nslug: reference/react/useEffect\n---\n\nIntro paragraph that is long enough to be worth indexing in the corpus at all.\n\n## Reference\n\n`useEffect(setup, dependencies?)` lets you synchronize a component with an external system and run side effects after render.\n\n## Caveats\n\nEffects only run on the client, never during server rendering, which surprises people often.\n";
        let mut out = Vec::new();
        extract_markdown("src/content/reference/react/useEffect.md", src, &mut out);
        assert!(out.len() >= 2, "至少切出 Reference 和 Caveats 两节：{}", out.len());
        // front-matter 的 title 要当文档名，比文件名可读。
        assert!(out.iter().all(|e| e.title.starts_with("useEffect · ")),
            "标题要带上文档名：{:?}", out.iter().map(|e| &e.title).collect::<Vec<_>>());
        assert!(out.iter().any(|e| e.body.contains("synchronize a component")));
        assert!(out.iter().all(|e| e.kind == "doc"));
        // 太短的节不进语料——只有一个链接或一张图的节没有检索价值。
        let mut tiny = Vec::new();
        extract_markdown("x.md", "## A\n\nshort\n", &mut tiny);
        assert!(tiny.is_empty(), "过短的节不该入库");
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
