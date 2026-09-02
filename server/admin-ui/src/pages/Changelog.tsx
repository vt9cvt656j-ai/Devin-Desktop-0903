import { useCallback, useEffect, useState } from "react";
import { ExternalLink, Plus, RefreshCw, Trash2, Wrench, X } from "lucide-react";

import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Panel } from "@/components/Panel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * 更新日志 —— 写给用户看的那份，直接从这里发布。
 *
 * 条目落在 changelog_entries 表里，这个页面是它唯一的写入口，网站的 /changelog 只读
 * 已发布的条目。以前它是网站仓库里的一个 TypeScript 文件：加一条要改代码、重新构建、
 * 再部署一次。
 *
 * 这是**文档**，不是发布记录。GitHub release 里那句自动生成的
 * "Auto-built installers for macOS and Windows" 在六个版本上一模一样 —— 那是构建日志。
 */

type Kind = "added" | "fixed" | "changed";

type Entry = {
  id: string;
  date: string;
  product: string;
  title: string;
  version: string;
  changes: { kind: Kind; text: string }[];
  published: boolean;
};

const KIND_ICON = { added: Plus, fixed: Wrench, changed: RefreshCw } as const;
const KIND_LABEL: Record<Kind, string> = { added: "新增", fixed: "修复", changed: "调整" };
const KIND_STYLE: Record<Kind, string> = {
  added: "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400",
  fixed: "bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-400",
  changed: "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-400",
};

/** 和网站分组用的是同一组，刻意保持短。 */
const PRODUCTS = ["IDE", "Console", "Website", "Gateway"] as const;

const SITE = "https://mrday.one/changelog";

function today(): string {
  // 用本地年月日拼，不用 toISOString()：格林尼治以西过了下午五点，后者给的是明天，
  // 网站那边已经栽过一次同样的 UTC 差一天。
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

const BLANK: { kind: Kind; text: string } = { kind: "added", text: "" };

export type ChangelogView = "changelog" | "changelog-list";

export function Changelog({ view }: { view: ChangelogView }) {
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [date, setDate] = useState(today());
  const [product, setProduct] = useState<string>("IDE");
  const [title, setTitle] = useState("");
  const [version, setVersion] = useState("");
  const [changes, setChanges] = useState<{ kind: Kind; text: string }[]>([{ ...BLANK }]);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);

  // 只有「已发布」那一屏用得上这份列表；写表单的那一屏拉它纯属浪费一次请求。
  const load = useCallback(async () => {
    if (view !== "changelog-list") return;
    try {
      const body = await api.get<{ entries: Entry[] }>("/api/admin/changelog");
      setEntries(body.entries);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    }
  }, [view]);

  useEffect(() => {
    void load();
  }, [load]);

  const filled = changes.filter((c) => c.text.trim());
  const ready = title.trim().length > 0 && filled.length > 0;

  async function publish() {
    setBusy(true);
    setNote(null);
    try {
      await api.post("/api/admin/changelog", {
        date,
        product,
        title,
        version,
        changes: filled,
        published: true,
      });
      setTitle("");
      setVersion("");
      setChanges([{ ...BLANK }]);
      setNote({ text: "已发布，网站上立即可见。", ok: true });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "发布失败", ok: false });
    } finally {
      setBusy(false);
    }
  }

  async function remove(entry: Entry) {
    // 删除立刻对外生效，没有撤销 —— 值得一次确认。
    if (!confirm(`删除「${entry.title}」？网站上会立刻消失。`)) return;
    try {
      await api.del(`/api/admin/changelog/${entry.id}`);
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "删除失败", ok: false });
    }
  }

  function patch(i: number, next: Partial<{ kind: Kind; text: string }>) {
    setChanges((cs) => cs.map((c, n) => (n === i ? { ...c, ...next } : c)));
  }

  return (
    /*
     * 整页收在同一个宽度里，标题和面板共用一条左边缘。
     *
     * 之前是三个不同的宽度套在一起：整栏满宽的标题、mx-auto max-w-4xl 的面板、面板里
     * 再套一个 mx-auto max-w-3xl 的表单。结果标题贴在最左边，面板浮在中间，两者看不出
     * 是一页上的东西 —— 那点「内边距」其实是两层居中挤出来的空隙，不是有意留的。
     */
    <div className="mx-auto w-full max-w-3xl space-y-6">
      <PageHeader
        title={view === "changelog" ? "更新日志 · 写一条" : "更新日志 · 已发布"}
        description={
          view === "changelog"
            ? "写给用户看的改动说明，发布后出现在 mrday.one/changelog。这是文档，不是构建记录 —— 每条都要说清楚对用户来说什么变了。"
            : "已经发布出去的条目，网站上的 /changelog 就是这一份。删除立刻对外生效。"
        }
      />

      {view === "changelog" && (
      <Panel
        bodyClassName="p-5"
        title="写一条"
        aside={
          <a
            href={SITE}
            target="_blank"
            rel="noreferrer"
            className="flex items-center gap-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground"
          >
            看看网站上的样子
            <ExternalLink className="size-3.5" />
          </a>
        }
      >
        {/*
          单栏。曾经右边挂过一个「网站上的样子」预览，但它把表单挤到左边去了 ——
          一块占掉三分之一宽度的参考图，代价是主要内容再也没法居中。想看发出去的样子，
          面板右上角那个链接直接开网站。
        */}
        <div className="space-y-5">
          {/* 日期、产品、版本都是短字段，并排一行；标题和改动是要读的句子，各占一整行。 */}
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
            <div className="space-y-1.5">
              <Label htmlFor="cl-date">日期</Label>
              <Input
                id="cl-date"
                type="date"
                value={date}
                onChange={(e) => setDate(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="cl-product">产品</Label>
              <Select
                id="cl-product"
                value={product}
                onChange={(e) => setProduct(e.target.value)}
              >
                {PRODUCTS.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="cl-version">版本</Label>
              <Input
                id="cl-version"
                value={version}
                placeholder="v0.3.48（可留空）"
                onChange={(e) => setVersion(e.target.value)}
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="cl-title">标题</Label>
            <Input
              id="cl-title"
              value={title}
              placeholder="一句话说清这次改了什么"
              onChange={(e) => setTitle(e.target.value)}
            />
          </div>

          <Separator />

          <div className="space-y-2.5">
            <div className="flex items-baseline justify-between">
              <Label>改动条目</Label>
              <span className="text-xs text-muted-foreground">
                {filled.length} 条 · 说变化，不说改了哪个文件
              </span>
            </div>

            {changes.map((c, i) => (
              <div key={i} className="flex items-center gap-2">
                <Select
                  aria-label="类型"
                  className="w-28 shrink-0 text-sm"
                  value={c.kind}
                  onChange={(e) => patch(i, { kind: e.target.value as Kind })}
                >
                  {(Object.keys(KIND_LABEL) as Kind[]).map((k) => (
                    <option key={k} value={k}>
                      {KIND_LABEL[k]}
                    </option>
                  ))}
                </Select>
                <Input
                  value={c.text}
                  placeholder="对用户来说变了什么"
                  onChange={(e) => patch(i, { text: e.target.value })}
                />
                {/* 只有一行时不给删除键：删光了没有意义，还得再点一次"再加一条"。 */}
                {/* 只有一行时不渲染，而不是 invisible —— invisible 仍然占位，
                    那一行的输入框就会比上面的标题短一截，看着像没对齐。 */}
                {changes.length > 1 && (
                  <Button
                    variant="ghost"
                    size="icon"
                    className="shrink-0"
                    onClick={() => setChanges((cs) => cs.filter((_, n) => n !== i))}
                    aria-label="删掉这一条"
                  >
                    <X className="size-4" />
                  </Button>
                )}
              </div>
            ))}

            <Button
              variant="outline"
              size="sm"
              onClick={() => setChanges((cs) => [...cs, { ...BLANK }])}
            >
              <Plus className="size-4" /> 再加一条
            </Button>
          </div>

          <div className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
            <Button onClick={() => void publish()} disabled={busy || !ready}>
              {busy ? "发布中…" : "发布"}
            </Button>
            {note && (
              <span
                className={cn(
                  "text-sm",
                  note.ok ? "text-emerald-600 dark:text-emerald-400" : "text-destructive",
                )}
              >
                {note.text}
              </span>
            )}
            {!ready && !note && (
              <span className="text-sm text-muted-foreground">标题和至少一条改动是必填的。</span>
            )}
          </div>
        </div>
      </Panel>
      )}

      {view === "changelog-list" && (
      <Panel
        title="已发布"
        aside={
          entries ? <span className="text-xs text-muted-foreground">{entries.length} 条</span> : null
        }
        bodyClassName="p-0"
      >
        {/* 读不到已发布列表不该把「写一条」也一起打掉 —— 那是两件事，
            而且写新条目并不需要先读到旧的。 */}
        {/* 三种「还没有内容」的状态同高，解析出结果时页面不会先矮一下再蹿高。 */}
        {error ? (
          <div className="grid min-h-[20rem] place-items-center px-5">
            <ErrorState message={error} onRetry={() => void load()} />
          </div>
        ) : !entries ? (
          <p className="grid min-h-[20rem] place-items-center px-5 text-sm text-muted-foreground">
            加载中…
          </p>
        ) : entries.length === 0 ? (
          <EmptyState
            title="还没有任何条目"
            hint="去「写一条」发布一条，它会同时出现在这里和网站上。"
            className="min-h-[20rem] justify-center"
          />
        ) : (
          <ul>
            {entries.map((e, i) => (
              <li key={e.id}>
                {i > 0 && <Separator />}
                <div className="group flex gap-4 px-5 py-4">
                  {/* 日期单独一栏并且等宽：条目按天读，让它们在左边对齐成一列。 */}
                  <time className="w-24 shrink-0 pt-0.5 font-mono text-xs text-muted-foreground">
                    {e.date}
                  </time>

                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium">{e.title}</span>
                      <Badge variant="outline">{e.product}</Badge>
                      {e.version && (
                        <code className="rounded bg-secondary px-1.5 py-0.5 text-xs">
                          {e.version}
                        </code>
                      )}
                      {!e.published && <Badge>草稿</Badge>}
                    </div>

                    <ul className="mt-2.5 space-y-1.5">
                      {e.changes.map((c, n) => {
                        const Icon = KIND_ICON[c.kind] ?? Plus;
                        return (
                          <li key={n} className="flex items-start gap-2 text-sm text-muted-foreground">
                            <span
                              title={KIND_LABEL[c.kind]}
                              className={cn(
                                "mt-0.5 grid size-5 shrink-0 place-items-center rounded-full",
                                KIND_STYLE[c.kind],
                              )}
                            >
                              <Icon className="size-3" strokeWidth={2.5} />
                            </span>
                            <span className="text-pretty leading-relaxed">{c.text}</span>
                          </li>
                        );
                      })}
                    </ul>
                  </div>

                  {/* 删除只在这一行上才出现：一列常驻的垃圾桶图标，读的时候比条目本身还显眼。 */}
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 shrink-0 self-start text-muted-foreground opacity-0 transition-opacity hover:text-destructive focus-visible:opacity-100 group-hover:opacity-100"
                    onClick={() => void remove(e)}
                    aria-label={`删除 ${e.title}`}
                  >
                    <Trash2 className="size-4" />
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </Panel>
      )}
    </div>
  );
}
