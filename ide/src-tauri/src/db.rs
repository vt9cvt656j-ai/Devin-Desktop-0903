//! Database tools for the agent — query MySQL / PostgreSQL / SQLite (via sqlx's
//! `Any` driver) and Redis (via the `redis` crate). This lets the agent work with
//! REAL data — inspect schemas, read rows, check cache keys, run migrations — instead
//! of guessing. One command `db_query(driver, url, query)` covers all four.
//!
//! Generic decoding is best-effort: common column types (int / float / bool / text /
//! bytes) render directly; exotic types (date / decimal / uuid / json) come back as
//! `<typename>` — CAST them to text in the query if you need the value. Bounded by a
//! row cap + connect/query timeouts so a huge table or a hung server can't wedge the UI.

use serde_json::json;
use std::time::Duration;

const MAX_ROWS: usize = 500;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const QUERY_TIMEOUT: Duration = Duration::from_secs(20);

/// Run a SQL query (mysql / postgres / sqlite) or a Redis command. `driver` is one of
/// {mysql, postgres, sqlite, redis}. For SQL, `query` is the statement; for Redis,
/// `query` is a command line like `GET mykey` or `KEYS user:*`.
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
    match driver.as_str() {
        "redis" => redis_cmd(url.trim(), &q).await,
        "mysql" | "postgres" | "postgresql" | "sqlite" => sql_query(&driver, url.trim(), &q, cap).await,
        other => Err(format!(
            "不支持的 driver: {other}（支持 mysql / postgres / sqlite / redis）"
        )),
    }
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
    let qt = |_| (if is_read { "查询超时（20s）" } else { "执行超时（20s）" }).to_string();

    match driver {
        "sqlite" => {
            let mut c = tokio::time::timeout(CONNECT_TIMEOUT, sqlx::SqliteConnection::connect(url))
                .await.map_err(ct)?.map_err(|e| format!("连接失败: {e}"))?;
            let out = if is_read {
                let rows = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).fetch_all(&mut c))
                    .await.map_err(qt)?.map_err(|e| format!("查询出错: {e}"))?;
                Ok(rows_to_json(&rows, "sqlite", cap, ms(started)))
            } else {
                let res = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).execute(&mut c))
                    .await.map_err(qt)?.map_err(|e| format!("执行出错: {e}"))?;
                Ok(affected_json("sqlite", res.rows_affected(), ms(started)))
            };
            let _ = c.close().await;
            out
        }
        "mysql" => {
            let mut c = tokio::time::timeout(CONNECT_TIMEOUT, sqlx::MySqlConnection::connect(url))
                .await.map_err(ct)?.map_err(|e| format!("连接失败: {e}"))?;
            let out = if is_read {
                let rows = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).fetch_all(&mut c))
                    .await.map_err(qt)?.map_err(|e| format!("查询出错: {e}"))?;
                Ok(rows_to_json(&rows, "mysql", cap, ms(started)))
            } else {
                let res = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).execute(&mut c))
                    .await.map_err(qt)?.map_err(|e| format!("执行出错: {e}"))?;
                Ok(affected_json("mysql", res.rows_affected(), ms(started)))
            };
            let _ = c.close().await;
            out
        }
        "postgres" | "postgresql" => {
            let mut c = tokio::time::timeout(CONNECT_TIMEOUT, sqlx::PgConnection::connect(url))
                .await.map_err(ct)?.map_err(|e| format!("连接失败: {e}"))?;
            let out = if is_read {
                let rows = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).fetch_all(&mut c))
                    .await.map_err(qt)?.map_err(|e| format!("查询出错: {e}"))?;
                Ok(rows_to_json(&rows, "postgres", cap, ms(started)))
            } else {
                let res = tokio::time::timeout(QUERY_TIMEOUT, sqlx::query(q).execute(&mut c))
                    .await.map_err(qt)?.map_err(|e| format!("执行出错: {e}"))?;
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
    for<'r> sqlx::types::chrono::NaiveDateTime: sqlx::Decode<'r, R::Database> + sqlx::Type<R::Database>,
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
                let t = row.try_get_raw(i).map(|r| r.type_info().name().to_string()).unwrap_or_default();
                json!(format!("<{}>", if t.is_empty() { "unsupported".to_string() } else { t }))
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
        assert_eq!(split_args(r#"SET k "hello world""#), vec!["SET", "k", "hello world"]);
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
        db_query("sqlite".into(), url.clone(), ddl.into(), None).await.expect("create");

        let ins = db_query(
            "sqlite".into(),
            url.clone(),
            "INSERT INTO t VALUES (1,'alice',9.5,1),(2,'bob',NULL,0)".into(),
            None,
        )
        .await
        .expect("insert");
        assert_eq!(ins["rows_affected"].as_u64(), Some(2), "two rows inserted");

        let sel = db_query("sqlite".into(), url.clone(), "SELECT id,name,score,active FROM t ORDER BY id".into(), None)
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
        assert!(rows[0][3] == json!(1) || rows[0][3] == json!(true), "bool decodes, got {:?}", rows[0][3]);
        // row 1: NULL score must decode to json null
        assert_eq!(rows[1][1], json!("bob"));
        assert_eq!(rows[1][2], serde_json::Value::Null, "NULL → json null");

        let _ = std::fs::remove_file(&path);
    }
}
