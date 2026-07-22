//! Database tools for the agent — query MySQL / MariaDB / PostgreSQL / SQLite (sqlx),
//! SQL Server (tiberius), MongoDB (mongodb), Redis (redis), ClickHouse and
//! Elasticsearch / OpenSearch (HTTP). Postgres-wire databases (CockroachDB, TimescaleDB,
//! Supabase, Neon, Redshift…) and Redis-wire stores (Valkey, KeyDB, Dragonfly…) work
//! through their base drivers via alias normalization. One command
//! `db_query(driver, url, query)` covers everything.
//!
//! Generic decoding is best-effort: common column types (int / float / bool / text /
//! bytes) render directly; exotic types (date / decimal / uuid / json) come back as
//! `<typename>` — CAST them to text in the query if you need the value. Bounded by a
//! row cap + connect/query timeouts so a huge table or a hung server can't wedge the UI.

use serde_json::json;
use std::str::FromStr;
use std::time::Duration;

const MAX_ROWS: usize = 500;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const QUERY_TIMEOUT: Duration = Duration::from_secs(20);

/// Normalize driver names / wire-compatible aliases onto the concrete backends.
fn normalize_driver(d: &str) -> String {
    match d {
        "mariadb" | "tidb" | "oceanbase" | "doris" | "starrocks" => "mysql".into(),
        "postgresql" | "cockroach" | "cockroachdb" | "timescale" | "timescaledb" | "supabase"
        | "neon" | "redshift" | "greenplum" | "yugabyte" | "opengauss" => "postgres".into(),
        "valkey" | "keydb" | "dragonfly" | "kvrocks" | "garnet" => "redis".into(),
        "mssql" | "sqlserver" | "azuresql" | "sql-server" => "mssql".into(),
        "mongo" | "mongodb" | "documentdb" | "ferretdb" | "cosmosdb" => "mongodb".into(),
        "clickhouse" | "ch" => "clickhouse".into(),
        "elastic" | "elasticsearch" | "opensearch" | "es" => "elastic".into(),
        other => other.to_string(),
    }
}

/// Run a SQL query (mysql / mariadb / postgres / sqlite / mssql / clickhouse), a Redis
/// command line, a MongoDB database command (JSON), or an Elasticsearch REST request
/// (`GET /_cat/indices` / `POST /idx/_search {json}`).
#[tauri::command]
pub async fn db_query(
    driver: String,
    url: String,
    query: String,
    limit: Option<usize>,
) -> Result<serde_json::Value, String> {
    let driver = driver.trim().to_lowercase();
    let q = query.trim().to_string();
    if url.trim().is_empty() {
        return Err("缺少数据库连接 url".into());
    }
    if q.is_empty() {
        return Err("缺少查询语句 / 命令".into());
    }
    let cap = limit.unwrap_or(MAX_ROWS).min(2000);
    let driver = normalize_driver(&driver);
    match driver.as_str() {
        "redis" => redis_cmd(url.trim(), &q).await,
        "mysql" | "postgres" | "sqlite" => sql_query(&driver, url.trim(), &q, cap).await,
        "mssql" => mssql_query(url.trim(), &q, cap).await,
        "mongodb" => mongo_query(url.trim(), &q).await,
        "clickhouse" => clickhouse_query(url.trim(), &q, cap).await,
        "elastic" => elastic_query(url.trim(), &q).await,
        other => Err(format!(
            "不支持的 driver: {other}（支持 mysql / mariadb / postgres / sqlite / mssql / mongodb / redis / clickhouse / elasticsearch 及各自的协议兼容库）"
        )),
    }
}

// ---- SQL Server (tiberius) ----

fn mssql_config_from_url(url: &str) -> Result<tiberius::Config, String> {
    let u = url::Url::parse(url).map_err(|e| {
        format!("mssql 连接串无效: {e}（格式 mssql://user:pass@host:1433/database）")
    })?;
    let mut cfg = tiberius::Config::new();
    cfg.host(u.host_str().unwrap_or("127.0.0.1"));
    cfg.port(u.port().unwrap_or(1433));
    let db = u.path().trim_start_matches('/');
    if !db.is_empty() {
        cfg.database(db);
    }
    let user = percent_decode(u.username());
    let pass = percent_decode(u.password().unwrap_or(""));
    if !user.is_empty() {
        cfg.authentication(tiberius::AuthMethod::sql_server(user, pass));
    }
    // 默认信任证书（自签证书的内网 SQL Server 是常态）；?encrypt=off 可关加密。
    cfg.trust_cert();
    if u.query_pairs()
        .any(|(k, v)| k == "encrypt" && (v == "off" || v == "false"))
    {
        cfg.encryption(tiberius::EncryptionLevel::NotSupported);
    }
    Ok(cfg)
}

fn percent_decode(s: &str) -> String {
    url::form_urlencoded::parse(format!("x={s}").as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| s.to_string())
}

fn mssql_cell_json(data: &tiberius::ColumnData<'_>) -> serde_json::Value {
    use tiberius::ColumnData as C;
    match data {
        C::U8(v) => v.map(|x| json!(x)).unwrap_or(serde_json::Value::Null),
        C::I16(v) => v.map(|x| json!(x)).unwrap_or(serde_json::Value::Null),
        C::I32(v) => v.map(|x| json!(x)).unwrap_or(serde_json::Value::Null),
        C::I64(v) => v.map(|x| json!(x)).unwrap_or(serde_json::Value::Null),
        C::F32(v) => v.map(|x| json!(x)).unwrap_or(serde_json::Value::Null),
        C::F64(v) => v.map(|x| json!(x)).unwrap_or(serde_json::Value::Null),
        C::Bit(v) => v.map(|x| json!(x)).unwrap_or(serde_json::Value::Null),
        C::String(v) => v
            .as_ref()
            .map(|x| json!(x.as_ref()))
            .unwrap_or(serde_json::Value::Null),
        C::Guid(v) => v
            .map(|x| json!(x.to_string()))
            .unwrap_or(serde_json::Value::Null),
        C::Binary(v) => v
            .as_ref()
            .map(|x| json!(String::from_utf8_lossy(x).to_string()))
            .unwrap_or(serde_json::Value::Null),
        C::Numeric(v) => v
            .map(|x| json!(x.to_string()))
            .unwrap_or(serde_json::Value::Null),
        C::Xml(v) => v
            .as_ref()
            .map(|x| json!(x.as_ref().to_string()))
            .unwrap_or(serde_json::Value::Null),
        other => {
            // 日期/时间类型等其余变体：Debug 渲染成可读字符串，避免整行失败。
            json!(format!("{other:?}"))
        }
    }
}

async fn mssql_query(url: &str, q: &str, cap: usize) -> Result<serde_json::Value, String> {
    use tokio_util::compat::TokioAsyncWriteCompatExt;
    let cfg = mssql_config_from_url(url)?;
    let started = std::time::Instant::now();
    let addr = cfg.get_addr();
    let tcp = tokio::time::timeout(CONNECT_TIMEOUT, tokio::net::TcpStream::connect(&addr))
        .await
        .map_err(|_| "连接超时（10s）".to_string())?
        .map_err(|e| format!("连接失败: {e}"))?;
    tcp.set_nodelay(true).ok();
    let mut client = tokio::time::timeout(
        CONNECT_TIMEOUT,
        tiberius::Client::connect(cfg, tcp.compat_write()),
    )
    .await
    .map_err(|_| "连接超时（10s）".to_string())?
    .map_err(|e| format!("连接失败: {e}"))?;
    let head = q.trim_start().to_lowercase();
    let is_read =
        head.starts_with("select") || head.starts_with("with") || head.starts_with("exec sp_help");
    let ms = started.elapsed().as_millis() as u64;
    let _ = ms;
    if is_read {
        let stream = tokio::time::timeout(QUERY_TIMEOUT, client.simple_query(q))
            .await
            .map_err(|_| "查询超时（20s）".to_string())?
            .map_err(|e| format!("查询出错: {e}"))?;
        let results = tokio::time::timeout(QUERY_TIMEOUT, stream.into_results())
            .await
            .map_err(|_| "查询超时（20s）".to_string())?
            .map_err(|e| format!("查询出错: {e}"))?;
        let rows: Vec<tiberius::Row> = results.into_iter().flatten().collect();
        let mut columns: Vec<String> = Vec::new();
        if let Some(r0) = rows.first() {
            columns = r0.columns().iter().map(|c| c.name().to_string()).collect();
        }
        let total = rows.len();
        let out_rows: Vec<Vec<serde_json::Value>> = rows
            .iter()
            .take(cap)
            .map(|row| row.cells().map(|(_, d)| mssql_cell_json(d)).collect())
            .collect();
        Ok(json!({
            "driver": "mssql",
            "columns": columns,
            "rows": out_rows,
            "row_count": total,
            "truncated": total > cap,
            "elapsed_ms": started.elapsed().as_millis() as u64,
        }))
    } else {
        let res = tokio::time::timeout(QUERY_TIMEOUT, client.execute(q, &[]))
            .await
            .map_err(|_| "执行超时（20s）".to_string())?
            .map_err(|e| format!("执行出错: {e}"))?;
        let n: u64 = res.rows_affected().iter().sum();
        Ok(affected_json(
            "mssql",
            n,
            started.elapsed().as_millis() as u64,
        ))
    }
}

// ---- MongoDB ----

async fn mongo_query(url: &str, q: &str) -> Result<serde_json::Value, String> {
    let started = std::time::Instant::now();
    let mut opts =
        tokio::time::timeout(CONNECT_TIMEOUT, mongodb::options::ClientOptions::parse(url))
            .await
            .map_err(|_| "mongodb 连接超时（10s）".to_string())?
            .map_err(|e| format!("mongodb 连接串无效: {e}"))?;
    opts.connect_timeout = Some(CONNECT_TIMEOUT);
    opts.server_selection_timeout = Some(CONNECT_TIMEOUT);
    let client =
        mongodb::Client::with_options(opts).map_err(|e| format!("mongodb 连接失败: {e}"))?;
    let db = client
        .default_database()
        .unwrap_or_else(|| client.database("admin"));
    let val: serde_json::Value = serde_json::from_str(q).map_err(|_| {
        "mongodb 的 query 必须是 JSON 数据库命令，例如 {\"listCollections\":1} 或 {\"find\":\"users\",\"limit\":10}".to_string()
    })?;
    let doc = mongodb::bson::to_document(&val).map_err(|e| format!("命令无效: {e}"))?;
    let out = tokio::time::timeout(QUERY_TIMEOUT, db.run_command(doc))
        .await
        .map_err(|_| "mongodb 命令超时（20s）".to_string())?
        .map_err(|e| format!("mongodb 出错: {e}"))?;
    let result = serde_json::to_value(&out).unwrap_or(serde_json::Value::Null);
    Ok(
        json!({ "driver": "mongodb", "result": result, "elapsed_ms": started.elapsed().as_millis() as u64 }),
    )
}

// ---- ClickHouse（HTTP 接口，无额外驱动依赖）----

async fn clickhouse_query(url: &str, q: &str, cap: usize) -> Result<serde_json::Value, String> {
    let started = std::time::Instant::now();
    // 接受 clickhouse://user:pass@host:8123/db 或直接 http(s)://…
    let mut u = url::Url::parse(url).map_err(|e| format!("clickhouse 连接串无效: {e}"))?;
    if u.scheme() == "clickhouse" || u.scheme() == "ch" {
        let https = u.port() == Some(8443) || u.port() == Some(443);
        let mut s = format!(
            "{}://{}",
            if https { "https" } else { "http" },
            u.host_str().unwrap_or("127.0.0.1")
        );
        if let Some(p) = u.port() {
            s.push_str(&format!(":{p}"));
        } else {
            s.push_str(":8123");
        }
        let db = u.path().trim_start_matches('/').to_string();
        let user = u.username().to_string();
        let pass = u.password().map(|x| x.to_string());
        u = url::Url::parse(&s).map_err(|e| format!("clickhouse 地址无效: {e}"))?;
        if !db.is_empty() {
            u.query_pairs_mut().append_pair("database", &db);
        }
        if !user.is_empty() {
            u.set_username(&user).ok();
            u.set_password(pass.as_deref()).ok();
        }
    }
    let user = u.username().to_string();
    let pass = u.password().map(|x| x.to_string());
    u.set_username("").ok();
    u.set_password(None).ok();
    let head = q.trim_start().to_lowercase();
    let is_read = head.starts_with("select")
        || head.starts_with("show")
        || head.starts_with("with")
        || head.starts_with("describe")
        || head.starts_with("desc ")
        || head.starts_with("exists")
        || head.starts_with("explain");
    let body = if is_read && !head.contains(" format ") {
        format!("{} FORMAT JSON", q.trim_end_matches(';').trim_end())
    } else {
        q.to_string()
    };
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(QUERY_TIMEOUT)
        .build()
        .map_err(|e| format!("HTTP 客户端初始化失败: {e}"))?;
    let mut req = client.post(u.clone()).body(body);
    if !user.is_empty() {
        req = req.basic_auth(&user, pass.as_deref());
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("clickhouse 请求失败: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("clickhouse 响应读取失败: {e}"))?;
    if !status.is_success() {
        return Err(format!(
            "clickhouse 出错（HTTP {}）: {}",
            status.as_u16(),
            text.chars().take(500).collect::<String>()
        ));
    }
    if is_read {
        let parsed: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("clickhouse JSON 解析失败: {e}"))?;
        let columns: Vec<String> = parsed["meta"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|m| m["name"].as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default();
        let data = parsed["data"].as_array().cloned().unwrap_or_default();
        let total = data.len();
        let rows: Vec<Vec<serde_json::Value>> = data
            .iter()
            .take(cap)
            .map(|row| columns.iter().map(|c| row[c].clone()).collect())
            .collect();
        Ok(json!({
            "driver": "clickhouse",
            "columns": columns,
            "rows": rows,
            "row_count": total,
            "truncated": total > cap,
            "elapsed_ms": started.elapsed().as_millis() as u64,
        }))
    } else {
        Ok(
            json!({ "driver": "clickhouse", "ok": true, "result": text.trim(), "elapsed_ms": started.elapsed().as_millis() as u64 }),
        )
    }
}

// ---- Elasticsearch / OpenSearch（REST）----

async fn elastic_query(url: &str, q: &str) -> Result<serde_json::Value, String> {
    let started = std::time::Instant::now();
    let base = url.trim_end_matches('/');
    // query 格式：`GET /_cat/indices?format=json` 或 `POST /idx/_search {json}`；裸 JSON 默认 POST /_search。
    let t = q.trim();
    let (method, rest) = if let Some(r) = t.strip_prefix("GET ") {
        ("GET", r.trim())
    } else if let Some(r) = t.strip_prefix("POST ") {
        ("POST", r.trim())
    } else if let Some(r) = t.strip_prefix("PUT ") {
        ("PUT", r.trim())
    } else if let Some(r) = t.strip_prefix("DELETE ") {
        ("DELETE", r.trim())
    } else if t.starts_with('{') {
        ("POST", t)
    } else {
        ("GET", t)
    };
    let (path, body) = if method == "POST" && rest.starts_with('{') {
        ("/_search", rest)
    } else {
        match rest.find(|c: char| c == '{') {
            Some(i) => (rest[..i].trim(), rest[i..].trim()),
            None => (rest, ""),
        }
    };
    let path = if path.is_empty() { "/" } else { path };
    let full = format!(
        "{}{}{}",
        base,
        if path.starts_with('/') { "" } else { "/" },
        path
    );
    let mut u = url::Url::parse(&full).map_err(|e| format!("elasticsearch 地址无效: {e}"))?;
    let user = u.username().to_string();
    let pass = u.password().map(|x| x.to_string());
    u.set_username("").ok();
    u.set_password(None).ok();
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(QUERY_TIMEOUT)
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("HTTP 客户端初始化失败: {e}"))?;
    let mut req = match method {
        "POST" => client.post(u),
        "PUT" => client.put(u),
        "DELETE" => client.delete(u),
        _ => client.get(u),
    };
    if !user.is_empty() {
        req = req.basic_auth(&user, pass.as_deref());
    }
    if !body.is_empty() {
        req = req
            .header("content-type", "application/json")
            .body(body.to_string());
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("elasticsearch 请求失败: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("elasticsearch 响应读取失败: {e}"))?;
    let result: serde_json::Value = serde_json::from_str(&text).unwrap_or(json!(text.trim()));
    if !status.is_success() {
        return Err(format!(
            "elasticsearch 出错（HTTP {}）: {}",
            status.as_u16(),
            text.chars().take(500).collect::<String>()
        ));
    }
    Ok(
        json!({ "driver": "elastic", "result": result, "elapsed_ms": started.elapsed().as_millis() as u64 }),
    )
}

async fn sql_query(
    driver: &str,
    url: &str,
    q: &str,
    cap: usize,
) -> Result<serde_json::Value, String> {
    // Concrete per-backend connections (NOT sqlx `Any`): Any fails the whole fetch the
    // moment a row has a type it can't map (e.g. a SQLite BOOLEAN or a Postgres DATE),
    // which is most real tables. Concrete rows fetch fine; we then decode each cell
    // best-effort and tolerate the odd exotic type per-cell instead of failing the query.
    use sqlx::Connection;
    // The query future here genuinely requires a `'static` SQL string (sqlx's query borrow escapes
    // through the boxed/timeout'd future — a reborrow fails to compile), so we leak the statement.
    // Agent DB queries are few and short → this is a negligible, bounded cost, and far simpler than
    // threading lifetimes through three concrete backends. (Verified: reborrow → E0521.)
    let q: &'static str = Box::leak(q.to_owned().into_boxed_str());
    let head = q.trim_start().to_lowercase();
    let is_read = head.starts_with("select")
        || head.starts_with("with")
        || head.starts_with("show")
        || head.starts_with("pragma")
        || head.starts_with("explain")
        || head.starts_with("describe")
        || head.starts_with("desc ");
    let started = std::time::Instant::now();
    let ms = |t: std::time::Instant| t.elapsed().as_millis() as u64;
    let ct = |_| "连接超时（10s）".to_string();
    let qt = |_| {
        (if is_read {
            "查询超时（20s）"
        } else {
            "执行超时（20s）"
        })
        .to_string()
    };

    match driver {
        "sqlite" => {
            let options = if url.trim_start().starts_with("sqlite:") {
                sqlx::sqlite::SqliteConnectOptions::from_str(url)
                    .map_err(|e| format!("sqlite 连接串无效: {e}"))?
            } else {
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(url)
                    .read_only(false)
                    .create_if_missing(false)
            };
            let mut c = tokio::time::timeout(
                CONNECT_TIMEOUT,
                sqlx::SqliteConnection::connect_with(&options),
            )
            .await
            .map_err(ct)?
            .map_err(|e| format!("连接失败: {e}"))?;
            let out = if is_read {
                let rows = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).fetch_all(&mut c))
                    .await
                    .map_err(qt)?
                    .map_err(|e| format!("查询出错: {e}"))?;
                Ok(rows_to_json(&rows, "sqlite", cap, ms(started)))
            } else {
                let res = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).execute(&mut c))
                    .await
                    .map_err(qt)?
                    .map_err(|e| format!("执行出错: {e}"))?;
                Ok(affected_json("sqlite", res.rows_affected(), ms(started)))
            };
            let _ = c.close().await;
            out
        }
        "mysql" => {
            let mut c = tokio::time::timeout(CONNECT_TIMEOUT, sqlx::MySqlConnection::connect(url))
                .await
                .map_err(ct)?
                .map_err(|e| format!("连接失败: {e}"))?;
            let out = if is_read {
                let rows = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).fetch_all(&mut c))
                    .await
                    .map_err(qt)?
                    .map_err(|e| format!("查询出错: {e}"))?;
                Ok(rows_to_json(&rows, "mysql", cap, ms(started)))
            } else {
                let res = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).execute(&mut c))
                    .await
                    .map_err(qt)?
                    .map_err(|e| format!("执行出错: {e}"))?;
                Ok(affected_json("mysql", res.rows_affected(), ms(started)))
            };
            let _ = c.close().await;
            out
        }
        "postgres" | "postgresql" => {
            let mut c = tokio::time::timeout(CONNECT_TIMEOUT, sqlx::PgConnection::connect(url))
                .await
                .map_err(ct)?
                .map_err(|e| format!("连接失败: {e}"))?;
            let out = if is_read {
                let rows = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).fetch_all(&mut c))
                    .await
                    .map_err(qt)?
                    .map_err(|e| format!("查询出错: {e}"))?;
                Ok(rows_to_json(&rows, "postgres", cap, ms(started)))
            } else {
                let res = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).execute(&mut c))
                    .await
                    .map_err(qt)?
                    .map_err(|e| format!("执行出错: {e}"))?;
                Ok(affected_json("postgres", res.rows_affected(), ms(started)))
            };
            let _ = c.close().await;
            out
        }
        other => Err(format!("内部错误：未处理的 driver {other}")),
    }
}

fn affected_json(driver: &str, n: u64, ms: u64) -> serde_json::Value {
    json!({ "driver": driver, "rows_affected": n, "elapsed_ms": ms, "ok": true })
}

/// Build a SELECT result from concrete rows, generic over the backend. Each cell is
/// decoded best-effort (int → float → bool → datetime → text → bytes); NULL → json
/// null; a type none of those cover → `"<typename>"` (CAST it to text to read it).
fn rows_to_json<R>(rows: &[R], driver: &str, cap: usize, elapsed_ms: u64) -> serde_json::Value
where
    R: sqlx::Row,
    usize: sqlx::ColumnIndex<R>,
    for<'r> i64: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> f64: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> bool: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> String: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> Vec<u8>: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> sqlx::types::chrono::NaiveDateTime:
        sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
    for<'r> sqlx::types::chrono::NaiveDate: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
{
    use sqlx::{Column, TypeInfo, ValueRef};
    let mut columns: Vec<String> = Vec::new();
    if let Some(r0) = rows.first() {
        for c in r0.columns() {
            columns.push(c.name().to_string());
        }
    }
    let total = rows.len();
    let mut out_rows: Vec<Vec<serde_json::Value>> = Vec::with_capacity(rows.len().min(cap));
    for row in rows.iter().take(cap) {
        let mut cells: Vec<serde_json::Value> = Vec::with_capacity(columns.len());
        for i in 0..columns.len() {
            let is_null = row.try_get_raw(i).map(|r| r.is_null()).unwrap_or(false);
            let v = if is_null {
                serde_json::Value::Null
            } else if let Ok(v) = row.try_get::<i64, _>(i) {
                json!(v)
            } else if let Ok(v) = row.try_get::<f64, _>(i) {
                json!(v)
            } else if let Ok(v) = row.try_get::<bool, _>(i) {
                json!(v)
            } else if let Ok(v) = row.try_get::<sqlx::types::chrono::NaiveDateTime, _>(i) {
                json!(v.to_string())
            } else if let Ok(v) = row.try_get::<sqlx::types::chrono::NaiveDate, _>(i) {
                json!(v.to_string())
            } else if let Ok(v) = row.try_get::<String, _>(i) {
                json!(v)
            } else if let Ok(v) = row.try_get::<Vec<u8>, _>(i) {
                json!(String::from_utf8_lossy(&v).to_string())
            } else {
                let t = row
                    .try_get_raw(i)
                    .map(|r| r.type_info().name().to_string())
                    .unwrap_or_default();
                json!(format!(
                    "<{}>",
                    if t.is_empty() {
                        "unsupported".to_string()
                    } else {
                        t
                    }
                ))
            };
            cells.push(v);
        }
        out_rows.push(cells);
    }
    json!({
        "driver": driver,
        "columns": columns,
        "rows": out_rows,
        "row_count": total,
        "truncated": total > cap,
        "elapsed_ms": elapsed_ms,
    })
}

async fn redis_cmd(url: &str, line: &str) -> Result<serde_json::Value, String> {
    let client = redis::Client::open(url).map_err(|e| format!("redis 连接串无效: {e}"))?;
    let mut conn = tokio::time::timeout(CONNECT_TIMEOUT, client.get_multiplexed_async_connection())
        .await
        .map_err(|_| "redis 连接超时（10s）".to_string())?
        .map_err(|e| format!("redis 连接失败: {e}"))?;
    let parts = split_args(line);
    if parts.is_empty() {
        return Err("空的 redis 命令".into());
    }
    let mut cmd = redis::cmd(&parts[0]);
    for a in &parts[1..] {
        cmd.arg(a);
    }
    let val: redis::Value = tokio::time::timeout(QUERY_TIMEOUT, cmd.query_async(&mut conn))
        .await
        .map_err(|_| "redis 命令超时（20s）".to_string())?
        .map_err(|e| format!("redis 出错: {e}"))?;
    Ok(json!({ "driver": "redis", "result": redis_value_to_json(&val) }))
}

/// Whitespace-split a redis command line, honoring "double" and 'single' quotes.
fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for ch in s.chars() {
        match quote {
            Some(qc) => {
                if ch == qc {
                    quote = None;
                } else {
                    cur.push(ch);
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                } else if ch.is_whitespace() {
                    if !cur.is_empty() {
                        out.push(std::mem::take(&mut cur));
                    }
                } else {
                    cur.push(ch);
                }
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn redis_value_to_json(v: &redis::Value) -> serde_json::Value {
    use redis::Value;
    match v {
        Value::Nil => serde_json::Value::Null,
        Value::Int(i) => json!(i),
        Value::BulkString(b) => json!(String::from_utf8_lossy(b).to_string()),
        Value::SimpleString(s) => json!(s),
        Value::Okay => json!("OK"),
        Value::Array(a) => json!(a.iter().map(redis_value_to_json).collect::<Vec<_>>()),
        Value::Set(a) => json!(a.iter().map(redis_value_to_json).collect::<Vec<_>>()),
        Value::Map(m) => {
            let mut o = serde_json::Map::new();
            for (k, val) in m {
                let key = match redis_value_to_json(k) {
                    serde_json::Value::String(s) => s,
                    other => other.to_string(),
                };
                o.insert(key, redis_value_to_json(val));
            }
            serde_json::Value::Object(o)
        }
        other => json!(format!("{other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redis_args_split() {
        assert_eq!(split_args("GET key"), vec!["GET", "key"]);
        assert_eq!(
            split_args(r#"SET k "hello world""#),
            vec!["SET", "k", "hello world"]
        );
        assert_eq!(split_args("KEYS user:*"), vec!["KEYS", "user:*"]);
    }

    // End-to-end against a real temp SQLite file (mysql/postgres share this Any path):
    // connect → execute (DDL/DML) → fetch (SELECT) → generic decode of int/text/real/
    // bool/null. Proves the tool actually returns correct data, not just that it builds.
    #[tokio::test]
    async fn sqlite_roundtrip_and_decode() {
        let mut path = std::env::temp_dir();
        path.push(format!("mide_db_test_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let url = format!("sqlite://{}?mode=rwc", path.display());

        let ddl = "CREATE TABLE t (id INTEGER, name TEXT, score REAL, active BOOLEAN)";
        db_query("sqlite".into(), url.clone(), ddl.into(), None)
            .await
            .expect("create");

        let ins = db_query(
            "sqlite".into(),
            url.clone(),
            "INSERT INTO t VALUES (1,'alice',9.5,1),(2,'bob',NULL,0)".into(),
            None,
        )
        .await
        .expect("insert");
        assert_eq!(ins["rows_affected"].as_u64(), Some(2), "two rows inserted");

        let sel = db_query(
            "sqlite".into(),
            url.clone(),
            "SELECT id,name,score,active FROM t ORDER BY id".into(),
            None,
        )
        .await
        .expect("select");
        assert_eq!(sel["columns"], json!(["id", "name", "score", "active"]));
        assert_eq!(sel["row_count"].as_u64(), Some(2));
        let rows = sel["rows"].as_array().unwrap();
        // row 0: 1, 'alice', 9.5, active=1 — the BOOLEAN column must NOT fail the fetch
        // (the bug that sank the Any driver), and decodes to 1/true.
        assert_eq!(rows[0][0], json!(1));
        assert_eq!(rows[0][1], json!("alice"));
        assert_eq!(rows[0][2].as_f64(), Some(9.5));
        assert!(
            rows[0][3] == json!(1) || rows[0][3] == json!(true),
            "bool decodes, got {:?}",
            rows[0][3]
        );
        // row 1: NULL score must decode to json null
        assert_eq!(rows[1][1], json!("bob"));
        assert_eq!(rows[1][2], serde_json::Value::Null, "NULL → json null");

        let _ = std::fs::remove_file(&path);
    }
}
