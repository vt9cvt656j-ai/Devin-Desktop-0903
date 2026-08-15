import { useCallback, useEffect, useState } from "react";
import { ExternalLink, Eye, EyeOff, Plus, RefreshCw, Trash2 } from "lucide-react";
import { api } from "@/lib/api";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Panel } from "@/components/Panel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

/**
 * 用户文档的编辑台。
 *
 * 网站上 mrday.one/docs 显示的就是这里写的东西 —— 内容存库，写完点发布就上线，不需要发版。
 *
 * # 一个表单做新建和修改
 *
 * 服务端按 `slug` 覆盖（`ON CONFLICT (slug) DO UPDATE`），所以这里不分"新建"和"编辑"两种
 * 模式：填上 slug 就是在写那一页。点左边的列表把它读进表单，改完保存就是更新。这样也顺带
 * 幂等 —— 连点两次保存不会多出一页。
 *
 * # 草稿
 *
 * `published` 关掉的页在网站上是 404（不是 403）—— 承认草稿存在本身就是信息。所以你可以先
 * 把下个版本的文档写完放着，发版那天再一起打开。
 */

type Row = {
  id: string;
  slug: string;
  section: string;
  title: string;
  sort: number;
  published: boolean;
  updated_at: string;
};

type Draft = {
  slug: string;
  section: string;
  title: string;
  body: string;
  sort: number;
  published: boolean;
};

const EMPTY: Draft = { slug: "", section: "", title: "", body: "", sort: 0, published: true };

export function Docs() {
  const [rows, setRows] = useState<Row[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);
  const [busy, setBusy] = useState(false);
  const [draft, setDraft] = useState<Draft>(EMPTY);

  const load = useCallback(async () => {
    setError(null);
    try {
      const body = await api.get<{ pages: Row[] }>("/api/admin/docs");
      setRows(body.pages ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "读取失败");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function edit(slug: string) {
    setNote(null);
    try {
      const d = await api.get<Draft>(`/api/admin/docs/${encodeURIComponent(slug)}`);
      setDraft({
        slug: d.slug,
        section: d.section ?? "",
        title: d.title,
        body: d.body ?? "",
        sort: d.sort ?? 0,
        published: d.published,
      });
      window.scrollTo({ top: 0 });
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "读取失败", ok: false });
    }
  }

  async function save() {
    setBusy(true);
    setNote(null);
    try {
      await api.post("/api/admin/docs", draft);
      setNote({ text: `已保存 /docs/${draft.slug}`, ok: true });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "保存失败", ok: false });
    } finally {
      setBusy(false);
    }
  }

  async function remove(row: Row) {
    // 删除是不可逆的，而这一页可能已经被人收藏或从别处链接过来了。
    if (!confirm(`删除《${row.title}》？\n\n/docs/${row.slug} 会立刻变成 404。`)) return;
    try {
      await api.del(`/api/admin/docs/id/${row.id}`);
      if (draft.slug === row.slug) setDraft(EMPTY);
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "删除失败", ok: false });
    }
  }

  return (
    <>
      <PageHeader
        title="用户文档"
        description="写在这里，mrday.one/docs 直接显示。支持 Markdown：标题、列表、代码块、链接。"
        actions={
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={() => setDraft(EMPTY)}>
              <Plus className="size-4" /> 新建
            </Button>
            <Button variant="outline" size="sm" onClick={() => void load()}>
              <RefreshCw className="size-4" /> 刷新
            </Button>
            <Button variant="outline" size="sm" asChild>
              <a href="https://mrday.one/docs" target="_blank" rel="noreferrer">
                <ExternalLink className="size-4" /> 看网站
              </a>
            </Button>
          </div>
        }
      />

      {note ? (
        <p className={`mb-4 text-sm ${note.ok ? "text-success" : "text-destructive"}`}>{note.text}</p>
      ) : null}

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <Panel title={draft.slug ? `编辑 /docs/${draft.slug}` : "新建一页"}>
          <div className="grid gap-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div>
                <Label htmlFor="d-title">标题</Label>
                <Input
                  id="d-title"
                  value={draft.title}
                  placeholder="快速开始"
                  onChange={(e) => setDraft({ ...draft, title: e.target.value })}
                />
              </div>
              <div>
                <Label htmlFor="d-slug">地址（/docs/…）</Label>
                <Input
                  id="d-slug"
                  value={draft.slug}
                  placeholder="getting-started"
                  onChange={(e) => setDraft({ ...draft, slug: e.target.value })}
                />
                {/* 服务端会再洗一遍（只留字母数字和连字符），这里先说清楚，省得保存后地址变了让人意外。 */}
                <p className="mt-1 text-xs text-muted-foreground">
                  只保留字母、数字和连字符；中文会被去掉，请用英文。
                </p>
              </div>
              <div>
                <Label htmlFor="d-section">分组（侧栏标题）</Label>
                <Input
                  id="d-section"
                  value={draft.section}
                  placeholder="入门"
                  onChange={(e) => setDraft({ ...draft, section: e.target.value })}
                />
              </div>
              <div>
                <Label htmlFor="d-sort">组内次序</Label>
                <Input
                  id="d-sort"
                  type="number"
                  value={String(draft.sort)}
                  onChange={(e) => setDraft({ ...draft, sort: Number(e.target.value) || 0 })}
                />
                <p className="mt-1 text-xs text-muted-foreground">小的排在前面，相同时按标题。</p>
              </div>
            </div>

            <div>
              <Label htmlFor="d-body">正文（Markdown）</Label>
              <textarea
                id="d-body"
                value={draft.body}
                onChange={(e) => setDraft({ ...draft, body: e.target.value })}
                rows={22}
                spellCheck={false}
                placeholder={"# 一级标题\n\n正文段落。**加粗**、`行内代码`、[链接](https://mrday.one)。\n\n- 列表项\n- 列表项\n\n```bash\nnpm install\n```"}
                className="mt-1.5 w-full rounded-lg border border-border bg-background p-3 font-mono text-[13px] leading-relaxed outline-none focus:border-primary"
              />
              {/*
                写清楚限制，而不是等它被静默丢掉。渲染器只认一个白名单子集 —— 原始 HTML 会
                原样显示成文字，不会变成标签。这是有意的：这一页和会话 cookie 同域。
              */}
              <p className="mt-1 text-xs text-muted-foreground">
                支持标题、列表、代码块、引用、粗体斜体、链接。直接写 HTML 不会生效（会显示成文字）。
              </p>
            </div>

            <div className="flex items-center justify-between">
              <label className="flex cursor-pointer items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={draft.published}
                  onChange={(e) => setDraft({ ...draft, published: e.target.checked })}
                />
                发布（不勾就是草稿，网站上是 404）
              </label>
              <Button disabled={busy || !draft.slug.trim() || !draft.title.trim()} onClick={() => void save()}>
                {busy ? "保存中…" : "保存"}
              </Button>
            </div>
          </div>
        </Panel>

        <Panel title="所有文档">
          {error ? (
            <ErrorState message={error} onRetry={() => void load()} />
          ) : rows === null ? (
            <p className="text-sm text-muted-foreground">读取中…</p>
          ) : rows.length === 0 ? (
            <EmptyState title="还没有文档" hint="在左边写第一页。" />
          ) : (
            <ul className="space-y-1">
              {rows.map((r) => (
                <li
                  key={r.id}
                  className="flex items-center gap-2 rounded-lg px-2 py-2 hover:bg-muted"
                >
                  <button
                    type="button"
                    onClick={() => void edit(r.slug)}
                    className="min-w-0 flex-1 text-left"
                  >
                    <span className="block truncate text-sm font-medium">{r.title}</span>
                    <span className="block truncate text-xs text-muted-foreground">
                      {r.section ? `${r.section} · ` : ""}/docs/{r.slug}
                    </span>
                  </button>
                  {r.published ? (
                    <Badge variant="success" title="已发布">
                      <Eye className="size-3" />
                    </Badge>
                  ) : (
                    <Badge variant="secondary" title="草稿">
                      <EyeOff className="size-3" />
                    </Badge>
                  )}
                  <Button variant="ghost" size="icon" onClick={() => void remove(r)} title="删除">
                    <Trash2 className="size-4" />
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </Panel>
      </div>
    </>
  );
}
