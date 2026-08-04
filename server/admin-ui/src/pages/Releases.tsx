import { useCallback, useEffect, useRef, useState } from "react";
import { Stat } from "@/components/Stat";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import { num, when } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * 版本发布 — the one screen where an operator ships a Michael IDE build: type an existing
 * vX.Y.Z tag to start the signed GitHub Actions build, watch it land as a Draft, then publish
 * the Draft once it carries a usable updater payload.
 *
 * The fact the old screen never showed: a Release is only useful to the auto-updater if it
 * carries latest.json (update.rs treats a Release without it as "no update available" — clients
 * silently stay on the old version; the per-platform signatures live inside that manifest, not in
 * the .sig assets beside it). So every row states that up front, and 校验并发布 is disabled on a
 * Draft with no latest.json, because the server would reject it anyway (download_release_manifest
 * → "Draft Release 尚未生成 latest.json").
 *
 * Deliberately left out of admin.html's version:
 *  - the second "发布 Draft" button beside the tag field. Publishing the thing you typed into a
 *    text box is how you publish the wrong version; publishing now happens only from the row of
 *    the Draft you are looking at, behind a confirm that names the tag.
 *  - the run-cancel button. It calls a real endpoint, but it belongs with the build logs on
 *    GitHub (linked in the header), not on the screen whose job is publishing.
 *  - window.confirm() for the consequential step, the connection "dot", the icon per cell, and
 *    the 10s polling of a page an operator visits for two minutes.
 */

type Asset = { id?: number; name?: string; size?: number; state?: string };

type Release = {
  id?: number;
  tag_name?: string;
  name?: string;
  draft?: boolean;
  prerelease?: boolean;
  html_url?: string;
  created_at?: string;
  published_at?: string;
  assets?: Asset[];
};

type Run = {
  id?: number;
  name?: string;
  display_title?: string;
  event?: string;
  head_branch?: string;
  head_sha?: string;
  status?: string;
  conclusion?: string;
  html_url?: string;
  created_at?: string;
  run_attempt?: number;
};

type Status = {
  configured?: boolean;
  connected?: boolean;
  repo?: string;
  workflow?: string;
  manifest_url?: string;
  actions_url?: string;
  releases_url?: string;
  runs?: Run[];
  releases?: Release[];
};

/** Same shape the server enforces (validate_release_tag): stable tags only, no pre-release refs. */
const TAG = /^v\d+\.\d+\.\d+$/;

const LINK =
  "text-sm text-muted-foreground underline-offset-4 transition-colors hover:text-foreground hover:underline";

/** Anchors get a URL straight out of the GitHub API response — pin it to github.com over https. */
function githubUrl(value?: string) {
  if (!value) return "";
  try {
    const url = new URL(value);
    return url.protocol === "https:" && url.hostname === "github.com" ? url.href : "";
  } catch {
    return "";
  }
}

/** Asset sizes are neither money nor time, so they don't belong in format.ts. */
function bytes(value?: number) {
  if (value == null) return "";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1048576).toFixed(1)} MB`;
}

/** What the Tauri updater needs out of a Release: the manifest, and signed installers. */
function updater(release: Release) {
  const assets = release.assets || [];
  return {
    assets,
    manifest: assets.some((a) => a.name === "latest.json"),
    signatures: assets.filter((a) => (a.name || "").endsWith(".sig")).length,
  };
}

function assetTitle(assets: Asset[]) {
  return assets.map((a) => `${a.name || "?"}  ${bytes(a.size)}`).join("\n");
}

const RUN_LABEL: Record<string, string> = {
  success: "成功",
  failure: "失败",
  cancelled: "已取消",
  timed_out: "超时",
  action_required: "需处理",
  skipped: "已跳过",
  neutral: "已结束",
  stale: "已失效",
};

function RunState({ run }: { run: Run }) {
  if (run.status && run.status !== "completed") {
    return <Badge variant="outline">{run.status === "queued" ? "等待中" : "构建中"}</Badge>;
  }
  if (run.conclusion === "success") return <Badge variant="success">成功</Badge>;
  const failed = run.conclusion === "failure" || run.conclusion === "timed_out";
  return (
    <Badge variant="outline" className={failed ? "text-destructive" : undefined}>
      {RUN_LABEL[run.conclusion || ""] || "已结束"}
    </Badge>
  );
}

export function Releases() {
  const [status, setStatus] = useState<Status | null>(null);
  const [err, setErr] = useState("");
  const [flash, setFlash] = useState<{ ok: boolean; text: string } | null>(null);
  const [tag, setTag] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirming, setConfirming] = useState<Release | null>(null);
  const alive = useRef(true);

  const load = useCallback(async () => {
    try {
      const data = await api.get<Status>("/api/admin/ide-releases");
      if (!alive.current) return;
      setStatus(data || {});
      setErr("");
    } catch (e) {
      if (!alive.current) return;
      setErr(e instanceof Error ? e.message : "无法读取发布状态");
    }
  }, []);

  useEffect(() => {
    alive.current = true;
    load();
    const timer = setInterval(load, 15_000);
    return () => {
      alive.current = false;
      clearInterval(timer);
    };
  }, [load]);

  const s = status || {};
  const releases = s.releases || [];
  const runs = s.runs || [];
  const drafts = releases.filter((r) => r.draft);
  // GitHub returns newest first. /releases/latest — the only thing latest_via_github_api reads —
  // is the newest release that is neither a draft nor a pre-release, so mirror that exactly.
  // A pre-release must NOT count as live: with only pre-releases published, /releases/latest 404s,
  // update.rs caches NoUpdate, and every client is told "已是最新". Falling back to a pre-release
  // here would report 就绪 on the one screen whose job is to catch that.
  const live = releases.find((r) => !r.draft && !r.prerelease);
  // Published, but nothing the updater will ever serve.
  const stranded = !live && releases.some((r) => !r.draft);
  // latest.json alone decides this: update.rs answers "no update" for a Release without it, and
  // the per-platform signatures live inside the manifest, not in the .sig assets beside it.
  const ready = !!live && updater(live).manifest;
  const running = runs.filter((r) => r.status && r.status !== "completed");
  const canAct = !!s.configured && !busy;
  const actionsUrl = githubUrl(s.actions_url);
  const releasesUrl = githubUrl(s.releases_url);
  const target = confirming ? updater(confirming) : null;

  async function dispatch() {
    const wanted = tag.trim();
    if (!TAG.test(wanted)) {
      setFlash({ ok: false, text: "tag 需要是 vX.Y.Z 格式，并且已经推到仓库" });
      return;
    }
    setBusy(true);
    setFlash(null);
    try {
      await api.post("/api/admin/ide-releases/dispatch", { tag: wanted });
      setFlash({ ok: true, text: `已触发 ${wanted} 的签名构建，完成后会出现一个草稿` });
      await load();
      // GitHub needs a moment to register the run; look again so the build shows up by itself.
      setTimeout(() => {
        if (alive.current) load();
      }, 3000);
    } catch (e) {
      setFlash({ ok: false, text: e instanceof Error ? e.message : "触发构建失败" });
    } finally {
      if (alive.current) setBusy(false);
    }
  }

  async function publish(release: Release) {
    const wanted = release.tag_name || "";
    setBusy(true);
    setFlash(null);
    try {
      const result = await api.post<{ published?: boolean; already_published?: boolean }>(
        "/api/admin/ide-releases/publish",
        { tag: wanted },
      );
      setFlash({
        ok: true,
        text: result?.already_published
          ? `${wanted} 已经是发布状态`
          : `${wanted} 已发布，客户端下一次检查更新就能收到`,
      });
      await load();
    } catch (e) {
      setFlash({ ok: false, text: e instanceof Error ? e.message : "发布失败" });
    } finally {
      if (alive.current) {
        setBusy(false);
        setConfirming(null);
      }
    }
  }

  return (
    <div>
      <h1 className="font-display text-2xl font-semibold tracking-tight">版本发布</h1>
      <p className="type-measure mt-1 text-muted-foreground">
        触发签名构建，确认草稿带齐更新清单后再发布。发布即上线，撤不回来。
      </p>

      {err && (
        <p role="alert" className="mt-4 text-sm text-destructive">
          {err}
        </p>
      )}
      {flash && (
        <p
          role={flash.ok ? "status" : "alert"}
          className={cn("mt-4 text-sm", flash.ok ? "text-success" : "text-destructive")}
        >
          {flash.text}
        </p>
      )}

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat
          label="线上版本"
          value={live?.tag_name || "—"}
          hint={
            live
              ? `发布于${when(live.published_at || live.created_at)}`
              : stranded
                ? "只有预发布版本，更新器不认"
                : "还没有已发布版本"
          }
        />
        <Stat
          label="待发布草稿"
          value={num(drafts.length)}
          hint={drafts.length ? "等待校验并发布" : "没有积压"}
        />
        <Stat
          label="构建中"
          value={num(running.length)}
          hint={running.length ? "GitHub Actions 正在跑" : "没有进行中的构建"}
        />
        <Stat
          label="自动更新"
          value={!status ? "—" : ready ? "就绪" : "不可用"}
          hint={
            !status
              ? "读取中"
              : ready
                ? "线上版本带 latest.json"
                : live
                  ? "线上版本没有 latest.json，客户端收不到"
                  : stranded
                    ? "预发布不会推给客户端"
                    : "等第一个版本发布"
          }
        />
      </div>

      <section className="mt-8 rounded-xl border border-border bg-card">
        <header className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-3">
          <div className="min-w-0">
            <h2 className="text-sm font-semibold">发布流水线</h2>
            <p className="mt-0.5 truncate text-xs text-muted-foreground">
              {s.repo || "未设置仓库"} · {s.workflow || "未设置工作流"}
            </p>
          </div>
          <div className="flex shrink-0 items-center gap-3">
            {status &&
              (s.connected ? (
                <Badge variant="success">GitHub 已连接</Badge>
              ) : (
                <Badge variant="outline">GitHub 未连接</Badge>
              ))}
            {actionsUrl && (
              <a className={LINK} href={actionsUrl} target="_blank" rel="noopener noreferrer">
                Actions
              </a>
            )}
            {releasesUrl && (
              <a className={LINK} href={releasesUrl} target="_blank" rel="noopener noreferrer">
                Releases
              </a>
            )}
          </div>
        </header>

        <div className="px-5 py-4">
          {status && !s.configured ? (
            <p className="type-measure text-sm text-muted-foreground">
              服务器还没有配置{" "}
              <code className="rounded bg-muted px-1.5 py-0.5 text-xs">IDE_RELEASE_GITHUB_TOKEN</code>
              ，配置后重启服务就能从这里发版。Token 只在服务端使用，不会进浏览器。
            </p>
          ) : (
            <>
              <div className="flex flex-wrap items-end gap-3">
                <div className="w-full max-w-52">
                  <Label htmlFor="release-tag">版本 tag</Label>
                  <Input
                    id="release-tag"
                    className="h-11"
                    value={tag}
                    placeholder="v0.3.16"
                    autoComplete="off"
                    spellCheck={false}
                    disabled={!canAct}
                    onChange={(e) => setTag(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") dispatch();
                    }}
                  />
                </div>
                <Button onClick={dispatch} disabled={!canAct || !tag.trim()}>
                  开始构建
                </Button>
                <Button variant="outline" onClick={() => load()} disabled={busy}>
                  刷新
                </Button>
              </div>
              <p className="type-measure mt-3 text-sm text-muted-foreground">
                只接受仓库里已经存在的 vX.Y.Z tag。构建产出一个草稿，草稿里必须有 latest.json
                才能发布——每个平台的签名写在这个清单里，不看旁边的 .sig 文件。
              </p>
            </>
          )}
        </div>
      </section>

      <section className="mt-6 rounded-xl border border-border bg-card">
        <header className="flex items-center justify-between border-b border-border px-5 py-3">
          <h2 className="text-sm font-semibold">Release</h2>
          {drafts.length > 0 && <Badge variant="outline">{drafts.length} 个草稿</Badge>}
        </header>
        {releases.length ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>版本</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>更新包</TableHead>
                <TableHead>时间</TableHead>
                <TableHead className="text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {releases.map((r, i) => {
                const u = updater(r);
                const url = githubUrl(r.html_url);
                return (
                  <TableRow key={r.id ?? r.tag_name ?? i}>
                    <TableCell>
                      <div className="font-medium tabular-nums">{r.tag_name || r.name || "未命名"}</div>
                      {r.name && r.name !== r.tag_name && (
                        <div className="mt-0.5 max-w-64 truncate text-xs text-muted-foreground">
                          {r.name}
                        </div>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap items-center gap-1.5">
                        {r.draft ? (
                          <Badge variant="outline">草稿</Badge>
                        ) : (
                          <Badge variant="success">已发布</Badge>
                        )}
                        {r.prerelease && <Badge variant="outline">预发布</Badge>}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap items-center gap-1.5">
                        {u.manifest ? (
                          <Badge variant="success">latest.json</Badge>
                        ) : (
                          <Badge variant="outline" className="text-destructive">
                            无 latest.json
                          </Badge>
                        )}
                        {u.signatures > 0 && <Badge variant="outline">{u.signatures} 个签名</Badge>}
                      </div>
                      <div
                        className="mt-1.5 max-w-72 truncate text-xs text-muted-foreground"
                        title={assetTitle(u.assets)}
                      >
                        {u.assets.length
                          ? `${u.assets.length} 个文件：${u.assets.map((a) => a.name || "?").join("、")}`
                          : "尚无构建文件"}
                      </div>
                    </TableCell>
                    <TableCell className="whitespace-nowrap text-muted-foreground">
                      {when(r.published_at || r.created_at)}
                    </TableCell>
                    <TableCell className="text-right">
                      {r.draft ? (
                        <Button
                          size="sm"
                          onClick={() => setConfirming(r)}
                          disabled={!canAct || !r.tag_name || !u.manifest}
                          title={u.manifest ? undefined : "缺 latest.json，服务端会拒绝发布"}
                        >
                          校验并发布
                        </Button>
                      ) : url ? (
                        <a className={LINK} href={url} target="_blank" rel="noopener noreferrer">
                          查看
                        </a>
                      ) : (
                        <span className="text-sm text-muted-foreground">—</span>
                      )}
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        ) : (
          <p className="px-5 py-8 text-center text-sm text-muted-foreground">
            {status ? "还没有 Release" : "读取中…"}
          </p>
        )}
      </section>

      <section className="mt-6 rounded-xl border border-border bg-card">
        <header className="flex items-center justify-between border-b border-border px-5 py-3">
          <h2 className="text-sm font-semibold">构建记录</h2>
          {running.length > 0 && <Badge variant="outline">{running.length} 个进行中</Badge>}
        </header>
        {runs.length ? (
          <Table className="min-w-[34rem]">
            <TableHeader>
              <TableRow>
                <TableHead>构建</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>触发</TableHead>
                <TableHead className="text-right">日志</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {runs.map((run, i) => {
                const url = githubUrl(run.html_url);
                return (
                  <TableRow key={run.id ?? i}>
                    <TableCell>
                      <div className="max-w-64 truncate font-medium">
                        {run.head_branch || run.display_title || run.name || `#${run.id ?? "—"}`}
                      </div>
                      <div className="mt-0.5 text-xs text-muted-foreground">
                        #{run.id ?? "—"} · 第 {run.run_attempt || 1} 次
                        {run.head_sha ? ` · ${String(run.head_sha).slice(0, 7)}` : ""}
                      </div>
                    </TableCell>
                    <TableCell>
                      <RunState run={run} />
                    </TableCell>
                    <TableCell className="whitespace-nowrap text-muted-foreground">
                      {when(run.created_at)}
                    </TableCell>
                    <TableCell className="text-right">
                      {url ? (
                        <a className={LINK} href={url} target="_blank" rel="noopener noreferrer">
                          查看
                        </a>
                      ) : (
                        <span className="text-sm text-muted-foreground">—</span>
                      )}
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        ) : (
          <p className="px-5 py-8 text-center text-sm text-muted-foreground">
            {status ? "还没有构建记录" : "读取中…"}
          </p>
        )}
      </section>

      <Dialog
        open={!!confirming}
        onOpenChange={(open) => {
          if (!open && !busy) setConfirming(null);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>发布 {confirming?.tag_name || ""}？</DialogTitle>
            <DialogDescription>
              发布后这个版本立刻成为最新版，所有 Michael IDE
              客户端下一次检查更新都会下载它。后台没有回退按钮。
            </DialogDescription>
          </DialogHeader>

          <div className="rounded-lg border border-border bg-muted/50 p-4 text-sm leading-relaxed text-muted-foreground">
            服务端会先校验 latest.json：版本号要和 {confirming?.tag_name || "该 tag"} 一致，macOS 与
            Windows 两个平台都要在清单里，每个平台都要有 HTTPS 下载地址和签名。有一项不通过就会拒绝，
            草稿保持原样。
          </div>

          {target && (
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="outline">{target.assets.length} 个文件</Badge>
              {target.manifest ? (
                <Badge variant="success">latest.json</Badge>
              ) : (
                <Badge variant="outline" className="text-destructive">
                  无 latest.json
                </Badge>
              )}
              <Badge variant="outline">{target.signatures} 个签名</Badge>
            </div>
          )}

          <div className="flex justify-end gap-3">
            <Button variant="outline" onClick={() => setConfirming(null)} disabled={busy}>
              取消
            </Button>
            <Button onClick={() => confirming && publish(confirming)} disabled={busy}>
              {busy ? "发布中…" : "确认发布"}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
