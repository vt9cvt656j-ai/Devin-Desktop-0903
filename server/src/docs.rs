//! 用户文档，从管理台写，在官网读。
//!
//! 两类读者，两组接口。`list` 和 `get` 是 mrday.one/docs 读的：只有已发布的页，不需要登录 ——
//! 它是一份公开文档。其余全部 admin-only，因为往这里写就是发布。
//!
//! # 正文是 Markdown，这带来一个必须正面处理的问题
//!
//! changelog 那边刻意不渲染任何标记（模块注释里写着「网站按文本打印，所以一条日志不可能把
//! 标记带进一个持有会话的页面」）。文档做不到这一点 —— 没有标题、代码块和链接的文档不叫
//! 文档。所以标记必须渲染，而渲染就意味着：
//!
//! **这个页面和会话 cookie 同域。** `mide_token` 的 Domain 是 `.mrday.one`，官网的脚本读得到
//! 它。一次存储型 XSS = 一次会话泄露。
//!
//! 处理方式分两层，缺一不可：
//!
//! 1. **服务端**（这里）：正文只做长度和数量的界限，不做净化 —— 净化留给渲染的那一端，因为
//!    只有它知道自己支持什么语法。服务端假装能净化 HTML 是最危险的做法：它会让下游以为
//!    内容已经干净了。
//! 2. **渲染端**（`website/src/components/site/docs-page.tsx`）：只支持一个**白名单子集**，
//!    从转义后的文本拼 HTML，原始 HTML 一律当普通文字。见那边的说明。
//!
//! 作者只有管理员，所以这不是防外人，是**万一管理员账号被盗时不至于连带把所有访客的会话
//! 一起赔进去**。

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::auth::Claims;
use crate::error::{ApiResult, AppError};
use crate::AppState;

/// 界限。给散文留足空间，同时让这张表不至于被当成别的东西的存储。
const MAX_SLUG: usize = 80;
const MAX_TITLE: usize = 160;
const MAX_SECTION: usize = 60;
/// 一页 60KB 的 Markdown 已经是很长的一篇了；再长该拆页，而不是往一行里塞。
const MAX_BODY: usize = 60_000;

fn admin_only(claims: &Claims) -> ApiResult<()> {
    if claims.role != "admin" {
        return Err(AppError::forbidden("需要管理员权限"));
    }
    Ok(())
}

fn clean(raw: &str, max: usize) -> String {
    raw.trim().chars().take(max).collect()
}

/// slug 就是地址的一段，所以字符集要钉死。
///
/// 只允许小写字母、数字和连字符：它会出现在 URL 里，也会被前端拿去和路径比对。放开一点点
/// （比如允许点号或斜杠）就等于把路由的形状交给作者决定。
fn clean_slug(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(MAX_SLUG)
        .collect::<String>()
        // 连续的连字符压成一个，`a // b` 不该变成 `a----b`。
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ── 公开 ─────────────────────────────────────────────────────────────────────

/// `GET /api/docs` —— 侧栏。**不含正文**。
///
/// 正文单独取（见 `get`）：一份完整文档可能是几十万字，而侧栏只需要标题。把正文一起发下来，
/// 等于每次打开文档站都下载全部内容。
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    // 分组的先后由**组内最小的 sort** 决定，不是分组名的字典序。
    //
    // 按名字排会得到「使用 → 参考 → 开始」这种顺序 —— 中文分组名的字节序毫无意义，而
    // 「开始」显然该排第一。用最小 sort 意味着：作者在后台调条目次序时，分组顺序自然
    // 跟着走，不需要再多填一个「分组次序」字段（多一个字段就多一个会填错、会忘记填的地方）。
    let rows: Vec<(String, String, String, i32)> = sqlx::query_as(
        "SELECT slug, section, title, sort FROM doc_pages WHERE published \
         ORDER BY MIN(sort) OVER (PARTITION BY section), section, sort, title LIMIT 500",
    )
    .fetch_all(&state.db)
    .await?;

    let pages: Vec<_> = rows
        .into_iter()
        .map(|(slug, section, title, sort)| {
            json!({ "slug": slug, "section": section, "title": title, "sort": sort })
        })
        .collect();
    Ok(Json(json!({ "pages": pages })))
}

/// `GET /api/docs/:slug` —— 一页的正文。
///
/// 草稿在这里和不存在是同一个回答（404）：否则这个接口就成了「猜 slug 看未发布内容」的入口。
pub async fn get(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let slug = clean_slug(&slug);
    let row: Option<(String, String, String, String)> = sqlx::query_as(
        "SELECT slug, section, title, body FROM doc_pages WHERE slug = $1 AND published",
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?;

    let (slug, section, title, body) = row.ok_or_else(|| AppError::not_found("文档不存在"))?;
    Ok(Json(json!({
        "slug": slug, "section": section, "title": title, "body": body,
    })))
}

// ── 管理台 ───────────────────────────────────────────────────────────────────

/// `GET /api/admin/docs` —— 含草稿的全量列表（仍不含正文，正文用 get_admin 取单页）。
pub async fn admin_list(
    State(state): State<AppState>,
    claims: Claims,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let rows: Vec<(uuid::Uuid, String, String, String, i32, bool, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as(
            // 和公开列表同一个排序，否则后台看到的次序和网站上的不一样 —— 那会让人
            // 以为自己排错了，然后去改一个本来就对的东西。
            "SELECT id, slug, section, title, sort, published, updated_at \
             FROM doc_pages \
             ORDER BY MIN(sort) OVER (PARTITION BY section), section, sort, title LIMIT 500",
        )
        .fetch_all(&state.db)
        .await?;

    let pages: Vec<_> = rows
        .into_iter()
        .map(|(id, slug, section, title, sort, published, updated_at)| {
            json!({
                "id": id, "slug": slug, "section": section, "title": title,
                "sort": sort, "published": published, "updated_at": updated_at,
            })
        })
        .collect();
    Ok(Json(json!({ "pages": pages })))
}

/// `GET /api/admin/docs/:slug` —— 编辑器要读正文，草稿也要能读。
pub async fn admin_get(
    State(state): State<AppState>,
    claims: Claims,
    Path(slug): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let slug = clean_slug(&slug);
    let row: Option<(uuid::Uuid, String, String, String, String, i32, bool)> = sqlx::query_as(
        "SELECT id, slug, section, title, body, sort, published FROM doc_pages WHERE slug = $1",
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await?;

    let (id, slug, section, title, body, sort, published) =
        row.ok_or_else(|| AppError::not_found("文档不存在"))?;
    Ok(Json(json!({
        "id": id, "slug": slug, "section": section, "title": title,
        "body": body, "sort": sort, "published": published,
    })))
}

#[derive(Deserialize)]
pub struct SaveReq {
    pub slug: String,
    #[serde(default)]
    pub section: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub sort: i32,
    #[serde(default = "yes")]
    pub published: bool,
}
fn yes() -> bool {
    true
}

/// `POST /api/admin/docs` —— 新建或覆盖（按 slug 认人）。
///
/// 一个接口做两件事，而不是 create + update 两条：slug 是地址，作者心里的操作就是「写这一页」，
/// 而不是「这是新的还是旧的」。`ON CONFLICT` 让它幂等 —— 连点两次保存不会多出一页。
pub async fn admin_save(
    State(state): State<AppState>,
    claims: Claims,
    Json(req): Json<SaveReq>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;

    let slug = clean_slug(&req.slug);
    if slug.is_empty() {
        return Err(AppError::bad("slug 不能为空（只允许字母、数字和连字符）"));
    }
    let title = clean(&req.title, MAX_TITLE);
    if title.is_empty() {
        return Err(AppError::bad("标题不能为空"));
    }
    let section = clean(&req.section, MAX_SECTION);
    let body: String = req.body.chars().take(MAX_BODY).collect();

    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO doc_pages (slug, section, title, body, sort, published) \
         VALUES ($1,$2,$3,$4,$5,$6) \
         ON CONFLICT (slug) DO UPDATE SET \
             section = EXCLUDED.section, title = EXCLUDED.title, body = EXCLUDED.body, \
             sort = EXCLUDED.sort, published = EXCLUDED.published, updated_at = now() \
         RETURNING id",
    )
    .bind(&slug)
    .bind(&section)
    .bind(&title)
    .bind(&body)
    .bind(req.sort)
    .bind(req.published)
    .fetch_one(&state.db)
    .await?;

    tracing::info!(%id, %slug, "doc page saved");
    Ok(Json(json!({ "id": id, "slug": slug })))
}

/// `DELETE /api/admin/docs/:id`
pub async fn admin_delete(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<uuid::Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    admin_only(&claims)?;
    let done = sqlx::query("DELETE FROM doc_pages WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if done == 0 {
        return Err(AppError::not_found("文档不存在"));
    }
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 公开的两条只能看到已发布的页。
    ///
    /// 草稿泄露不是抽象风险：文档常常在发布前就写好了下一个版本的功能。`get` 尤其重要 ——
    /// 少了 `AND published`，它就成了「知道 slug 就能读草稿」的接口，而 slug 是能猜的。
    #[test]
    fn the_public_endpoints_only_ever_see_published_pages() {
        let src = include_str!("docs.rs");
        let body = &src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())];
        for f in ["pub async fn list(", "pub async fn get("] {
            let seg = body.split(f).nth(1).expect(f);
            let seg = &seg[..seg.find("\n}").unwrap_or(seg.len())];
            assert!(seg.contains("published"), "{f} 少了已发布过滤");
        }
        // 公开列表不带正文：一份文档站的全部内容不该在打开侧栏时被下载一遍。
        let seg = body.split("pub async fn list(").nth(1).unwrap();
        let seg = &seg[..seg.find("\n}").unwrap_or(seg.len())];
        assert!(!seg.contains("body"), "公开列表不应携带正文");
    }

    /// 写入口全部要管理员。
    #[test]
    fn every_writing_endpoint_is_admin_only() {
        let src = include_str!("docs.rs");
        let body = &src[..src.find("\n#[cfg(test)]").unwrap_or(src.len())];
        for f in [
            "pub async fn admin_list(",
            "pub async fn admin_get(",
            "pub async fn admin_save(",
            "pub async fn admin_delete(",
        ] {
            let seg = body.split(f).nth(1).expect(f);
            let seg = &seg[..seg.find("\n}").unwrap_or(seg.len())];
            assert!(seg.contains("admin_only(&claims)?"), "{f} 少了管理员校验");
        }
    }

    /// slug 会变成地址，字符集必须钉死。
    #[test]
    fn a_slug_can_only_be_letters_digits_and_hyphens() {
        assert_eq!(clean_slug("Getting Started"), "getting-started");
        assert_eq!(clean_slug("  快速开始  "), "");
        assert_eq!(clean_slug("a//b"), "a-b");
        assert_eq!(clean_slug("--x--"), "x");
        // 路径穿越、查询串、片段：一个都不许留下。
        assert_eq!(clean_slug("../../etc/passwd"), "etc-passwd");
        assert_eq!(clean_slug("a?b=1"), "a-b-1");
        assert_eq!(clean_slug("a#b"), "a-b");
        for s in ["../x", "a/b", "a?b", "a#b", "a b"] {
            let out = clean_slug(s);
            assert!(
                out.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "{s} 洗出了不该有的字符：{out}",
            );
        }
    }

    /// 保存是幂等的：连点两次不会多出一页。
    #[test]
    fn saving_the_same_slug_twice_updates_rather_than_duplicates() {
        let src = include_str!("docs.rs");
        let seg = src.split("pub async fn admin_save(").nth(1).expect("admin_save");
        assert!(
            seg.contains("ON CONFLICT (slug) DO UPDATE"),
            "按 slug 覆盖，否则同一个地址会有两行，而读的那条 SQL 只会拿到其中一条",
        );
    }
}
