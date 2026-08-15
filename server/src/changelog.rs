//! The public changelog, written from the console.
//!
//! Two audiences, two endpoints. `list` is what mrday.one/changelog reads: published
//! entries, no drafts, no authentication — it is a public document. Everything else is
//! admin-only, because writing to it is publishing.
//!
//! Entries are prose. Nothing here renders markdown or HTML; the website prints the text
//! as text, so an entry cannot carry markup into a page that holds a session.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// Bounds on authored text. Generous for prose, small enough that the table cannot be
/// used as storage for something else.
const MAX_TITLE: usize = 200;
const MAX_PRODUCT: usize = 40;
const MAX_VERSION: usize = 40;
const MAX_CHANGE: usize = 600;
const MAX_CHANGES: usize = 20;

/// The three kinds the website has icons for. An unknown kind would render as a blank
/// marker, so it is rejected here rather than silently displayed wrong.
const KINDS: [&str; 3] = ["added", "fixed", "changed"];

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

fn clean(raw: &str, max: usize) -> String {
    raw.trim().chars().take(max).collect()
}

// ── GET /api/changelog ───────────────────────────────────────────────────────────────

/// Public. The website's changelog page reads this and nothing else.
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let rows: Vec<(uuid::Uuid, chrono::NaiveDate, String, String, String, serde_json::Value)> =
        sqlx::query_as(
            "SELECT id, entry_date, product, title, version, changes \
             FROM changelog_entries WHERE published \
             ORDER BY entry_date DESC, created_at DESC LIMIT 200",
        )
        .fetch_all(&state.db)
        .await?;

    Ok(Json(json!({
        "entries": rows
            .into_iter()
            .map(|(id, date, product, title, version, changes)| json!({
                "id": id,
                "date": date.to_string(),
                "product": product,
                "title": title,
                "version": version,
                "changes": changes,
            }))
            .collect::<Vec<_>>(),
    })))
}

// ── GET /api/admin/changelog ─────────────────────────────────────────────────────────

/// Admin. Includes drafts, which the public list deliberately hides.
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    type Row = (
        uuid::Uuid,
        chrono::NaiveDate,
        String,
        String,
        String,
        serde_json::Value,
        bool,
        chrono::DateTime<chrono::Utc>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT id, entry_date, product, title, version, changes, published, created_at \
         FROM changelog_entries ORDER BY entry_date DESC, created_at DESC LIMIT 500",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({
        "entries": rows
            .into_iter()
            .map(|(id, date, product, title, version, changes, published, created)| json!({
                "id": id,
                "date": date.to_string(),
                "product": product,
                "title": title,
                "version": version,
                "changes": changes,
                "published": published,
                "created_at": created,
            }))
            .collect::<Vec<_>>(),
    })))
}

// ── POST /api/admin/changelog ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ChangeReq {
    pub kind: String,
    pub text: String,
}

#[derive(Deserialize)]
pub struct EntryReq {
    /// ISO date, "2026-08-10".
    pub date: String,
    pub product: String,
    pub title: String,
    #[serde(default)]
    pub version: String,
    pub changes: Vec<ChangeReq>,
    #[serde(default = "yes")]
    pub published: bool,
}

fn yes() -> bool {
    true
}

pub async fn admin_create(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<EntryReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    let date = chrono::NaiveDate::parse_from_str(req.date.trim(), "%Y-%m-%d")
        .map_err(|_| AppError::bad("日期格式应为 YYYY-MM-DD"))?;

    let title = clean(&req.title, MAX_TITLE);
    if title.is_empty() {
        return Err(AppError::bad("标题不能为空"));
    }
    let product = clean(&req.product, MAX_PRODUCT);
    if product.is_empty() {
        return Err(AppError::bad("请填写所属产品"));
    }

    // An entry with no changes is a headline with nothing under it — the shape the page
    // cannot render usefully, so it is refused at the door rather than published empty.
    let mut changes = Vec::new();
    for c in req.changes.into_iter().take(MAX_CHANGES) {
        let text = clean(&c.text, MAX_CHANGE);
        if text.is_empty() {
            continue;
        }
        let kind = c.kind.trim().to_ascii_lowercase();
        if !KINDS.contains(&kind.as_str()) {
            return Err(AppError::bad("类型只能是 added / fixed / changed"));
        }
        changes.push(json!({ "kind": kind, "text": text }));
    }
    if changes.is_empty() {
        return Err(AppError::bad("至少写一条改动"));
    }

    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO changelog_entries (entry_date, product, title, version, changes, published) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
    )
    .bind(date)
    .bind(&product)
    .bind(&title)
    .bind(clean(&req.version, MAX_VERSION))
    .bind(serde_json::Value::Array(changes))
    .bind(req.published)
    .fetch_one(&state.db)
    .await?;

    crate::realtime::record_event(
        &state,
        None,
        "changelog_added",
        json!({ "id": id, "title": title }),
    )
    .await;

    Ok(Json(json!({ "id": id })))
}

// ── DELETE /api/admin/changelog/:id ──────────────────────────────────────────────────

pub async fn admin_delete(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    let done = sqlx::query("DELETE FROM changelog_entries WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if done.rows_affected() == 0 {
        return Err(AppError::bad("该条目不存在"));
    }

    crate::realtime::record_event(&state, None, "changelog_deleted", json!({ "id": id })).await;

    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The public list must never expose drafts.
    #[test]
    fn the_public_feed_shows_published_entries_only() {
        let src = include_str!("changelog.rs");
        let body = src.split("pub async fn list(").nth(1).expect("list");
        let body = &body[..body.find("\n// ──").unwrap_or(body.len())];
        assert!(
            body.contains("WHERE published"),
            "an unpublished entry is a draft and must not reach the public page"
        );
    }

    /// Writing is publishing; only an admin may do it.
    #[test]
    fn every_write_path_is_admin_gated() {
        let src = include_str!("changelog.rs");
        for handler in ["pub async fn admin_create(", "pub async fn admin_delete(", "pub async fn admin_list("] {
            let body = src.split(handler).nth(1).expect(handler);
            let head = &body[..body.find("\n}").unwrap_or(body.len())];
            assert!(
                head.contains("admin_only(&claims)?"),
                "{handler} must check the admin role before touching the table"
            );
        }
    }

    /// Only the three kinds the page can draw.
    #[test]
    fn unknown_change_kinds_are_refused() {
        assert_eq!(KINDS, ["added", "fixed", "changed"]);
        let src = include_str!("changelog.rs");
        assert!(
            src.contains("if !KINDS.contains(&kind.as_str())"),
            "an unrecognised kind would render as a blank marker on the site"
        );
    }
}
