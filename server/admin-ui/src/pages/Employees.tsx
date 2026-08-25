import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  Check,
  Eye,
  Play,
  Plus,
  RefreshCw,
  ShieldAlert,
  Trash2,
  Wrench,
  X,
} from "lucide-react";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Truncate } from "@/components/ui/table";
import { api } from "@/lib/api";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * 智能员工。
 *
 * # 这一页的主角是「边界」，不是「能力」
 *
 * 它管的是一个在跑的生意：真实用户、真实收款、一台在服务的服务器。所以界面上最显眼的
 * 不该是「它能干多少事」，而是**每项能力的后果**和**哪些动作需要你点头**。
 *
 * 具体三个决定：
 *
 * 1. **能力按风险分组，勾选框旁边写的是后果不是名字。** 「改定价」旁边写的是
 *    「直接改变用户被扣多少钱」——一个人在勾这个框的时候，该看到的是这句话。
 * 2. **待批准的动作放在最上面**，不是藏在某个员工的详情里。它是这套系统里唯一
 *    会真的改变什么的地方，必须一进页面就看见。
 * 3. **「自己动手」的下拉只有两档**，没有「全自动」。服务端也只接受这两个值——
 *    界面不给的选项，接口也不收。
 */

type Cap = { id: string; name: string; what: string; tier: number };
type Emp = {
  id: string;
  name: string;
  role: string;
  model_route: string | null;
  model_id: string;
  capabilities: string[];
  autonomy: string;
  every_minutes: number;
  enabled: boolean;
  last_run_at: string | null;
  last_summary: string;
  pending: number;
};
type Run = {
  id: string;
  employee_id: string;
  trigger: string;
  status: string;
  summary: string;
  detail: string;
  used: string[];
  error: string;
  created_at: string;
};
type Action = {
  id: string;
  run_id: string;
  employee_id: string;
  capability: string;
  args: Record<string, unknown>;
  reason: string;
  tier: number;
  status: string;
  result: string;
  created_at: string;
};
type Route = { id: string; label: string; models?: string[] };

const TIERS: Record<number, { label: string; hint: string; tone: string }> = {
  0: { label: "看", hint: "读数据，永远自动", tone: "text-muted-foreground" },
  1: { label: "运维", hint: "可逆的操作，可以配成自动", tone: "text-foreground" },
  2: { label: "影响用户", hint: "永远要你点头", tone: "text-warning" },
  3: { label: "危险", hint: "系统不执行，只写建议给你", tone: "text-destructive" },
};

function TierBadge({ tier }: { tier: number }) {
  const t = TIERS[tier];
  if (tier === 0) return <Badge variant="outline">{t.label}</Badge>;
  if (tier === 1)
    return (
      <Badge variant="outline">
        <Wrench /> {t.label}
      </Badge>
    );
  return (
    <Badge
      variant="outline"
      className={cn(
        tier === 2 ? "border-warning/40 text-warning" : "border-destructive/40 text-destructive",
      )}
    >
      <ShieldAlert /> {t.label}
    </Badge>
  );
}

function when(iso: string | null): string {
  if (!iso) return "还没跑过";
  const d = Math.round((Date.now() - new Date(iso).getTime()) / 60000);
  if (d < 1) return "刚刚";
  if (d < 60) return `${d} 分钟前`;
  if (d < 1440) return `${Math.round(d / 60)} 小时前`;
  return `${Math.round(d / 1440)} 天前`;
}

const BLANK = {
  name: "",
  role: "",
  model_route: "",
  model_id: "",
  capabilities: [] as string[],
  autonomy: "none",
  every_minutes: 0,
  enabled: true,
};

export function Employees() {
  const [emps, setEmps] = useState<Emp[] | null>(null);
  const [caps, setCaps] = useState<Cap[]>([]);
  const [routes, setRoutes] = useState<Route[]>([]);
  const [runs, setRuns] = useState<Run[]>([]);
  const [actions, setActions] = useState<Action[]>([]);
  const [err, setErr] = useState<string | null>(null);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);
  const [draft, setDraft] = useState<(typeof BLANK & { id?: string }) | null>(null);
  const [busy, setBusy] = useState(false);
  const [running, setRunning] = useState<string | null>(null);
  const [openRun, setOpenRun] = useState<Run | null>(null);

  const load = useCallback(async () => {
    setErr(null);
    try {
      const [a, b, c] = await Promise.all([
        api.get<{ employees: Emp[]; capabilities: Cap[] }>("/api/admin/employees"),
        api.get<{ runs: Run[]; actions: Action[] }>("/api/admin/employees/runs"),
        api.get<{ routes: Route[] }>("/api/admin/route-endpoints"),
      ]);
      setEmps(a.employees ?? []);
      setCaps(a.capabilities ?? []);
      setRuns(b.runs ?? []);
      setActions(b.actions ?? []);
      setRoutes(c.routes ?? []);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "读取失败");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function save() {
    if (!draft) return;
    setBusy(true);
    setNote(null);
    try {
      await api.post("/api/admin/employees", {
        ...draft,
        model_route: draft.model_route || null,
        every_minutes: Number(draft.every_minutes) || 0,
      });
      setDraft(null);
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "保存失败", ok: false });
    } finally {
      setBusy(false);
    }
  }

  async function runNow(id: string) {
    setRunning(id);
    setNote(null);
    try {
      await api.post(`/api/admin/employees/${id}/run`, {});
      setNote({ text: "跑完了，下面能看到工作记录。", ok: true });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "没跑成", ok: false });
    } finally {
      setRunning(null);
    }
  }

  async function decide(id: string, approve: boolean) {
    try {
      await api.post(`/api/admin/employees/actions/${id}/decide`, { approve });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "操作失败", ok: false });
    }
  }

  async function remove(e: Emp) {
    if (!confirm(`删掉「${e.name}」？它的工作记录也会一起删。`)) return;
    try {
      await api.del(`/api/admin/employees/${e.id}`);
      await load();
    } catch (x) {
      setNote({ text: x instanceof Error ? x.message : "删除失败", ok: false });
    }
  }

  const pending = useMemo(() => actions.filter((a) => a.status === "pending"), [actions]);
  const advice = useMemo(() => actions.filter((a) => a.status === "advice"), [actions]);
  const list = emps ?? [];
  const byId = (id: string) => list.find((e) => e.id === id)?.name ?? "已删除的员工";
  const capOf = (id: string) => caps.find((c) => c.id === id);
  const grouped = useMemo(() => {
    const g: Record<number, Cap[]> = { 0: [], 1: [], 2: [], 3: [] };
    for (const c of caps) g[c.tier]?.push(c);
    return g;
  }, [caps]);

  return (
    <div className="space-y-6">
      <PageHeader
        title="智能员工"
        description="给它一份职责和一组能力，它就替你盯着。能力分四档——看、运维、影响用户、危险；后两档它只能提，做不做由你点头。"
        actions={
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={() => void load()}>
              <RefreshCw /> 刷新
            </Button>
            <Button size="sm" onClick={() => setDraft({ ...BLANK })}>
              <Plus /> 招一个
            </Button>
          </div>
        }
      />

      <ErrorState message={err} />
      {note && (
        <p className={cn("text-sm", note.ok ? "text-success" : "text-destructive")}>{note.text}</p>
      )}

      {!emps && <TableSkeleton rows={3} columns={["35%", "25%", "25%"]} label="读取中" />}

      {emps && (
        <>
          <SectionReveal as="section" delay={70} className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <Stat label="员工" value={num(list.length)} hint={`${list.filter((e) => e.enabled).length} 个在岗`} />
            <Stat label="等你批准" value={num(pending.length)} hint={pending.length ? "下面第一块" : "没有积压"} />
            <Stat label="给你的建议" value={num(advice.length)} hint="系统不执行的那一档" />
            <Stat label="工作记录" value={num(runs.length)} hint="最近 60 条" />
          </SectionReveal>

          {/* 待批准放最上面：这是整套系统里唯一会真的改变什么的地方。 */}
          {pending.length > 0 && (
            <SectionReveal as="section" delay={110}>
              <Card className="border-warning/40">
                <CardHeader>
                  <AlertTriangle className="size-4 text-warning" />
                  <span className="font-semibold">等你批准（{pending.length}）</span>
                  <span className="text-xs text-muted-foreground">
                    批准之后才会真的去做。看那句「为什么」再决定。
                  </span>
                </CardHeader>
                <Separator />
                <div className="divide-y divide-border">
                  {pending.map((a) => (
                    <div key={a.id} className="flex flex-wrap items-start gap-3 px-5 py-3">
                      <TierBadge tier={a.tier} />
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium">
                          {capOf(a.capability)?.name ?? a.capability}
                          <span className="ml-2 text-xs font-normal text-muted-foreground">
                            {byId(a.employee_id)} 提的
                          </span>
                        </p>
                        <p className="text-sm text-muted-foreground">{a.reason || "（没写理由）"}</p>
                        {Object.keys(a.args ?? {}).length > 0 && (
                          <pre className="mt-1 overflow-x-auto rounded bg-muted/50 p-2 font-mono text-[11px]">
                            {JSON.stringify(a.args, null, 1)}
                          </pre>
                        )}
                      </div>
                      <div className="flex shrink-0 gap-1">
                        <Button size="sm" onClick={() => void decide(a.id, true)}>
                          <Check /> 批准
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => void decide(a.id, false)}>
                          <X /> 否决
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              </Card>
            </SectionReveal>
          )}

          {/* T3：系统不执行，只是写给你看的。和待批准分开——它没有「批准」这个动作。 */}
          {advice.length > 0 && (
            <SectionReveal as="section" delay={130}>
              <Card>
                <CardHeader>
                  <ShieldAlert className="size-4 text-destructive" />
                  <span className="font-semibold">给你的建议（{advice.length}）</span>
                  <span className="text-xs text-muted-foreground">
                    这一档系统<b>不会执行</b>。命令你自己看过再跑。
                  </span>
                </CardHeader>
                <Separator />
                <div className="divide-y divide-border">
                  {advice.slice(0, 8).map((a) => (
                    <div key={a.id} className="px-5 py-3">
                      <p className="text-sm">
                        <span className="text-muted-foreground">{byId(a.employee_id)}：</span>
                        {a.reason}
                      </p>
                      <pre className="mt-1 overflow-x-auto rounded bg-code-surface p-2 font-mono text-[11px] text-code-foreground">
                        {typeof a.args?.command === "string"
                          ? a.args.command
                          : typeof a.args?.sql === "string"
                            ? a.args.sql
                            : JSON.stringify(a.args, null, 1)}
                      </pre>
                    </div>
                  ))}
                </div>
              </Card>
            </SectionReveal>
          )}

          {!list.length && (
            <EmptyState
              title="还没有员工"
              hint="先招一个只会「看」的——让它盯着线路健康，每小时跑一次，有事发邮件给你。等你信得过它了再放开运维权限。"
              action={
                <Button size="sm" onClick={() => setDraft({ ...BLANK })}>
                  <Plus /> 招一个
                </Button>
              }
            />
          )}

          <SectionReveal as="section" delay={160} className="space-y-4">
            {list.map((e) => (
              <Card key={e.id} className={cn(!e.enabled && "opacity-60")}>
                <CardHeader>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold">{e.name}</span>
                      {!e.enabled && <Badge variant="outline">已停用</Badge>}
                      {e.pending > 0 && (
                        <Badge variant="outline" className="border-warning/40 text-warning">
                          {e.pending} 个待批准
                        </Badge>
                      )}
                      <Badge variant="outline">
                        {e.autonomy === "t1" ? "可自己做运维" : "只提建议"}
                      </Badge>
                      <span className="text-xs text-muted-foreground">
                        {e.every_minutes > 0 ? `每 ${e.every_minutes} 分钟` : "只手动"} ·{" "}
                        {when(e.last_run_at)}
                      </span>
                    </div>
                    <p className="mt-0.5 text-sm text-muted-foreground">
                      {e.last_summary || e.role.slice(0, 80) || "（还没写职责）"}
                    </p>
                  </div>
                  <div className="flex shrink-0 gap-1">
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={running === e.id}
                      onClick={() => void runNow(e.id)}
                    >
                      <Play /> {running === e.id ? "干活中…" : "立刻跑一次"}
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() =>
                        setDraft({
                          id: e.id,
                          name: e.name,
                          role: e.role,
                          model_route: e.model_route ?? "",
                          model_id: e.model_id,
                          capabilities: e.capabilities,
                          autonomy: e.autonomy,
                          every_minutes: e.every_minutes,
                          enabled: e.enabled,
                        })
                      }
                    >
                      编辑
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => void remove(e)}>
                      <Trash2 />
                    </Button>
                  </div>
                </CardHeader>
                <Separator />
                <div className="flex flex-wrap gap-1.5 px-5 py-3">
                  {e.capabilities.length === 0 && (
                    <span className="text-xs text-muted-foreground">
                      没给它任何能力 —— 它连数据都看不到。
                    </span>
                  )}
                  {e.capabilities.map((c) => (
                    <Badge key={c} variant="outline" className="text-xs">
                      {capOf(c)?.name ?? c}
                    </Badge>
                  ))}
                </div>
                {runs.filter((r) => r.employee_id === e.id).length > 0 && (
                  <>
                    <Separator />
                    <div className="divide-y divide-border">
                      {runs
                        .filter((r) => r.employee_id === e.id)
                        .slice(0, 3)
                        .map((r) => (
                          <button
                            key={r.id}
                            className="flex w-full items-start gap-3 px-5 py-2.5 text-left transition-colors hover:bg-accent/40"
                            onClick={() => setOpenRun(r)}
                          >
                            <Eye className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
                            <div className="min-w-0 flex-1">
                              <Truncate className="text-[13px]">
                                {r.status === "failed" ? (
                                  <span className="text-destructive">{r.error || "没跑成"}</span>
                                ) : (
                                  r.summary || "（没有结论）"
                                )}
                              </Truncate>
                            </div>
                            <span className="shrink-0 text-xs text-muted-foreground">
                              {when(r.created_at)}
                            </span>
                          </button>
                        ))}
                    </div>
                  </>
                )}
              </Card>
            ))}
          </SectionReveal>
        </>
      )}

      {/* 工作记录详情 */}
      <Dialog open={!!openRun} onOpenChange={(o) => !o && setOpenRun(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{openRun?.summary || "工作记录"}</DialogTitle>
            <DialogDescription>
              {openRun && (
                <>
                  {byId(openRun.employee_id)} · {openRun.trigger === "manual" ? "手动" : "定时"} ·{" "}
                  {when(openRun.created_at)}
                  {openRun.used.length > 0 && ` · 看了 ${openRun.used.length} 类数据`}
                </>
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="max-h-[50vh] overflow-y-auto whitespace-pre-wrap text-sm leading-relaxed">
            {openRun?.error && <p className="mb-2 text-destructive">{openRun.error}</p>}
            {openRun?.detail || "（它没写详细过程）"}
          </div>
        </DialogContent>
      </Dialog>

      {/* 招人 / 编辑 */}
      <Dialog open={!!draft} onOpenChange={(o) => !o && setDraft(null)}>
        <DialogContent className="max-h-[86vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{draft?.id ? "编辑员工" : "招一个员工"}</DialogTitle>
            <DialogDescription>
              职责写得越具体，它的判断越贴谱。能力按最小够用给——没勾的它连数据都看不到。
            </DialogDescription>
          </DialogHeader>
          {draft && (
            <div className="grid gap-4">
              <div className="grid gap-4 sm:grid-cols-2">
                <div>
                  <Label htmlFor="e-name">名字</Label>
                  <Input
                    id="e-name"
                    value={draft.name}
                    placeholder="线路管家"
                    onChange={(ev) => setDraft({ ...draft, name: ev.target.value })}
                  />
                </div>
                <div>
                  <Label htmlFor="e-every">多久跑一次（分钟）</Label>
                  <Input
                    id="e-every"
                    type="number"
                    value={String(draft.every_minutes)}
                    onChange={(ev) =>
                      setDraft({ ...draft, every_minutes: Number(ev.target.value) || 0 })
                    }
                  />
                  <p className="mt-1 text-xs text-muted-foreground">0 = 只在你点「立刻跑」时干活。</p>
                </div>
              </div>

              <div>
                <Label htmlFor="e-role">职责</Label>
                <textarea
                  id="e-role"
                  rows={5}
                  value={draft.role}
                  placeholder={
                    "盯着所有线路和上游。发现有连续失败、被限流、或者没额度的，判断严不严重。\n" +
                    "真出问题了给我发邮件说清楚是哪个、什么现象、我该做什么。\n" +
                    "一切正常就说一句正常，别为了显得有用编建议。"
                  }
                  onChange={(ev) => setDraft({ ...draft, role: ev.target.value })}
                  className="mt-1.5 w-full rounded-lg border border-border bg-background p-3 text-[13px] leading-relaxed outline-none focus:border-primary"
                />
              </div>

              <div className="grid gap-4 sm:grid-cols-2">
                <div>
                  <Label htmlFor="e-route">用哪条线路干活</Label>
                  <Select
                    id="e-route"
                    value={draft.model_route}
                    onChange={(ev) => setDraft({ ...draft, model_route: ev.target.value })}
                  >
                    <option value="">选一条…</option>
                    {routes.map((r) => (
                      <option key={r.id} value={r.id}>
                        {r.label}
                      </option>
                    ))}
                  </Select>
                  <p className="mt-1 text-xs text-muted-foreground">
                    用你自己的线路，不额外接一家。
                  </p>
                </div>
                <div>
                  <Label htmlFor="e-model">模型</Label>
                  <Select
                    id="e-model"
                    value={draft.model_id}
                    onChange={(ev) => setDraft({ ...draft, model_id: ev.target.value })}
                  >
                    <option value="">用这条线路的第一个</option>
                    {(routes.find((r) => r.id === draft.model_route)?.models ?? []).map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </Select>
                </div>
              </div>

              <div>
                <Label htmlFor="e-auto">它能自己动手到什么程度</Label>
                <Select
                  id="e-auto"
                  value={draft.autonomy}
                  onChange={(ev) => setDraft({ ...draft, autonomy: ev.target.value })}
                >
                  <option value="none">只提建议 —— 什么都要我点头</option>
                  <option value="t1">可以自己做运维 —— 下架/恢复/探测/发邮件给我</option>
                </Select>
                {/*
                  刻意只有两档。没有「全自动」——那个开关一旦存在，迟早会在某个着急的
                  晚上被打开，然后一次误判直接落到用户账上。服务端也只接受这两个值。
                */}
                <p className="mt-1 text-xs text-muted-foreground">
                  影响用户（改价、加额度、群发）和危险动作（服务器命令、改数据的
                  SQL）<b>不管怎么配都要你点头</b>，后者系统根本不执行。
                </p>
              </div>

              <div>
                <Label>能给它什么能力</Label>
                <div className="mt-1.5 space-y-3">
                  {[0, 1, 2, 3].map((tier) =>
                    grouped[tier]?.length ? (
                      <div key={tier} className="rounded-lg border border-border">
                        <div className="flex items-center gap-2 border-b border-border px-3 py-1.5">
                          <TierBadge tier={tier} />
                          <span className="text-xs text-muted-foreground">{TIERS[tier].hint}</span>
                        </div>
                        {grouped[tier].map((c) => (
                          <label
                            key={c.id}
                            className="flex cursor-pointer items-start gap-2 border-b border-border px-3 py-2 last:border-b-0 hover:bg-accent/40"
                          >
                            <input
                              type="checkbox"
                              className="mt-0.5"
                              checked={draft.capabilities.includes(c.id)}
                              onChange={(ev) =>
                                setDraft({
                                  ...draft,
                                  capabilities: ev.target.checked
                                    ? [...draft.capabilities, c.id]
                                    : draft.capabilities.filter((x) => x !== c.id),
                                })
                              }
                            />
                            <div className="min-w-0">
                              <p className="text-[13px] font-medium">{c.name}</p>
                              {/* 写后果，不写名字的同义反复——勾这个框的人该看到的是这句。 */}
                              <p className="text-xs text-muted-foreground">{c.what}</p>
                            </div>
                          </label>
                        ))}
                      </div>
                    ) : null,
                  )}
                </div>
              </div>

              <label className="flex cursor-pointer items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={draft.enabled}
                  onChange={(ev) => setDraft({ ...draft, enabled: ev.target.checked })}
                />
                在岗（取消勾选 = 留着配置但不干活）
              </label>

              <div className="flex justify-end gap-2">
                <Button variant="ghost" onClick={() => setDraft(null)}>
                  取消
                </Button>
                <Button disabled={busy || !draft.name.trim()} onClick={() => void save()}>
                  {busy ? "保存中…" : "保存"}
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
