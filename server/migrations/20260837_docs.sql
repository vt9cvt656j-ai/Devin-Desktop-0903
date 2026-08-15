-- 用户文档：内容存库，由后台写，网站只读。
--
-- 版本号原本是 20260836，和另一处同时新增的迁移撞了。sqlx 按版本号记账，同号的第二个文件
-- 会被**静默跳过** —— 部署成功、日志无异常、表就是不存在。加迁移时先看一眼 migrations/
-- 的最后一个号。
--
-- 和 changelog_entries 同一个形状（发布位、公开索引只覆盖已发布行），因为它们是同一件事的
-- 两个面：运营在管理台写，访客在官网读。形状一致意味着后台用起来是同一个手感。
--
-- 与 changelog 的区别只有一个，但很关键：**正文是 Markdown**。日志条目是纯文本，网站按文本
-- 打印；文档需要标题、代码块、链接，必须渲染。渲染的那一层因此要限制成一个安全子集 ——
-- 这个页面和会话 cookie 同域（.mrday.one 下的脚本读得到 mide_token），一次存储型 XSS 就是
-- 一次会话泄露。见 website 的 docs-page.tsx。
CREATE TABLE IF NOT EXISTS doc_pages (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- URL 的最后一段：/docs/<slug>。唯一，因为它就是地址。
    slug       TEXT NOT NULL UNIQUE,

    -- 侧栏分组名。同一个 section 的页面排在一起。
    section    TEXT NOT NULL DEFAULT '',

    title      TEXT NOT NULL,
    -- Markdown 正文。
    body       TEXT NOT NULL DEFAULT '',

    -- 组内次序。小的在前；相同时按标题。
    sort       INTEGER NOT NULL DEFAULT 0,

    published  BOOLEAN NOT NULL DEFAULT true,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 公开列表只读已发布的行，索引也只覆盖它们 —— 草稿再多也不进这个索引。
CREATE INDEX IF NOT EXISTS idx_doc_pages_public
    ON doc_pages (section, sort, title)
    WHERE published;
