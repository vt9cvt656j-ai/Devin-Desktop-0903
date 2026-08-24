import { useCallback, useEffect, useState } from "react";
import { Check, CircleSlash, ListChecks, Plus, RefreshCw, Trash2, X, Zap } from "lucide-react";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { VendorMark, vendorName } from "@/components/VendorMark";
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
 * 多路由 —— 一条线路挂多个上游出口。
 *
 * # 这一屏的主角是「顺序」，所以按顺序排，不按表格排
 *
 * 加出口这件事本身只有三个输入（地址、密钥、几折），不值得一整屏。真正需要看见的是
 * **同一条线路下这几个出口谁先被用**，因为那是这个功能的全部意义：便宜的先用、坏的靠后。
 * 做成一张平表（每行一个出口、有个"优先级"列）就要读着数字在脑子里重排一遍；
 * 直接按生效顺序竖着列，第一眼就是答案。
 *
 * # 为什么把「只有前两个会被用到」画出来
 *
 * 网关一个请求最多换两个出口就收手（`CHAT_UPSTREAM_MAX_ROUTES_WHEN_ANSWERED = 2`，
 * 再多客户端等不起）。所以挂十个不等于十次机会 —— 第三个往后只有在前面的被探测判坏、
 * 掉到后面之后才轮得到。这件事不画出来，运维会以为自己配了十重保险，而实际上第三个
 * 之后基本躺着。宁可界面上多一条分割线，也不能让人对可用性有错误预期。
 *
 * # 密钥只写不读
 *
 * 服务端只回「有没有配密钥」，不回密钥本身，后台页面也不例外。所以编辑时密钥框是空的，
 * 空着保存 = 沿用原值。这不是省事，是不想让一份明文密钥多在一个地方出现。
 */

type Endpoint = {
  id: string;
  route_id: string;
  label: string;
  base_url: string;
  has_key: boolean;
  cost_ratio: number;
  active: boolean;
  note: string;
  probe_ok: boolean | null;
  probe_at: string | null;
  probe_ms: number | null;
  probe_note: string;
  enabled_models: string[];
  protocol: string;
  capacity: number | null;
  live: string;
};

type Route = {
  id: string;
  label: string;
  protocol: string;
  vendor: string;
  base_url: string;
  active: boolean;
  model_count: number;
  models: string[];
  live: string;
  endpoints: Endpoint[];
};

type Draft = {
  id?: string;
  route_id: string;
  label: string;
  base_url: string;
  api_key: string;
  cost_ratio: string;
  note: string;
  /// 空 = 跟线路一样。
  protocol: string;
  active: boolean;
  /// 空数组 = 承载线路的全部模型。
  enabled_models: string[];
  /// 空串 = 不填。
  capacity: string;
};

/** 网关一个请求最多试几个出口。和 models.rs 的常量对齐 —— 改那边要改这里。 */
const TRIED_PER_REQUEST = 2;

/** 探测结论 → 排序档位。和服务端 `order_key` 同一套判据。 */
function tier(probeOk: boolean | null): number {
  if (probeOk === true) return 0;
  if (probeOk === null) return 1;
  return 2;
}

/**
 * 按生效顺序排出这条线路的出口，线路自带地址算成本 1.0 的那个。
 *
 * 这里重算一遍而不是让服务端回排好的：这一屏要的是「**如果**我把这个改成三折，
 * 它会排到第几」，而那要在保存之前就看得到。判据和服务端是同一套。
 */
function ordered(r: Route): Array<Endpoint | null> {
  const rows: Array<{ k: [number, number]; v: Endpoint | null }> = [
    // 线路自带的地址按第 0 档算，不是「还没测过」：它是在任的那个，今天所有流量都从它走。
    // 走「还没测过」的话，一个原价的备用中转只要测通就会把直连顶掉 —— 同价位凭空多一跳。
    // 判据和服务端 own_order_key 一致。
    { k: [0, 1], v: null },
    ...r.endpoints
      .filter((e) => e.active)
      .map((e) => ({ k: [tier(e.probe_ok), e.cost_ratio] as [number, number], v: e })),
  ];
  rows.sort((a, b) => a.k[0] - b.k[0] || a.k[1] - b.k[1]);
  return rows.map((x) => x.v);
}

function ratioText(v: number): string {
  if (v >= 1) return "原价";
  // 0.3 → 三折。中文习惯说折，小数点后一位足够；0.35 这种就直接给倍数。
  const tenth = Math.round(v * 100) / 10;
  return Number.isInteger(tenth) ? `${tenth} 折` : `${v}×`;
}

/**
 * 真实流量的结论。词表和服务端 `route_health::classify` 一致：
 * ok / degraded / error / unknown。自己另编一个词，就等于给自己造一条永远走不到的分支。
 */
function LiveDot({ live, className }: { live: string; className?: string }) {
  const map: Record<string, [string, string]> = {
    ok: ["bg-success", "真实流量最近成功过"],
    degraded: ["bg-warning", "最近成功过，但也在失败"],
    error: ["bg-destructive", "真实流量连续失败"],
  };
  const [color, title] = map[live] ?? ["bg-muted-foreground/40", "最近没有真实流量，不知道"];
  return (
    <span
      className={cn("size-2 shrink-0 rounded-full", color, className)}
      title={title}
      aria-label={title}
    />
  );
}

function ProbeBadge({ ok, ms, note }: { ok: boolean | null; ms: number | null; note: string }) {
  if (ok === null) {
    return (
      <Badge variant="outline" className="shrink-0">
        还没测
      </Badge>
    );
  }
  if (ok) {
    return (
      <Badge variant="success" className="shrink-0">
        <Check /> {ms ?? "—"}ms
      </Badge>
    );
  }
  // 失败原因是这一格最有价值的信息（密钥被拒 / 没有这个模型 / 连不上），
  // 别藏进 tooltip：藏起来就等于运维还得再点一次才知道要改什么。
  return (
    <Badge
      variant="outline"
      className="max-w-[16rem] shrink-0 border-destructive/40 text-destructive"
      title={note}
    >
      <X /> <span className="truncate">{note || "不通"}</span>
    </Badge>
  );
}

export function RouteEndpoints() {
  const [routes, setRoutes] = useState<Route[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [busy, setBusy] = useState(false);
  const [probing, setProbing] = useState<string | null>(null);
  // 「拉取」的结果：这家有哪些、缺哪些。缺的那部分才是运维真正要看的 ——
  // 它直接回答「这个出口能不能顶上」。
  const [fetched, setFetched] = useState<{ here: string[]; missing: string[] } | null>(null);
  const [fetching, setFetching] = useState(false);

  const load = useCallback(async () => {
    setErr(null);
    try {
      const body = await api.get<{ routes: Route[] }>("/api/admin/route-endpoints");
      setRoutes(body.routes ?? []);
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
      const r = await api.post<{ probe?: { ok: boolean; note: string; ms: number } }>(
        "/api/admin/route-endpoints",
        {
          id: draft.id,
          route_id: draft.route_id,
          label: draft.label,
          base_url: draft.base_url,
          api_key: draft.api_key,
          cost_ratio: Number(draft.cost_ratio) || 1,
          note: draft.note,
          protocol: draft.protocol,
          active: draft.active,
          enabled_models: draft.enabled_models,
          capacity: draft.capacity.trim() ? Number(draft.capacity) : null,
        },
      );
      // 保存后立刻回探测结论：填错密钥最想马上知道，而不是等它在候选池里躺 15 分钟。
      const p = r?.probe;
      setNote(
        p && !p.ok
          ? { text: `已保存，但探测没通过：${p.note}`, ok: false }
          : { text: p ? `已保存，探测通过（${p.ms}ms）` : "已保存", ok: true },
      );
      setDraft(null);
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "保存失败", ok: false });
    } finally {
      setBusy(false);
    }
  }

  async function probe(kind: "endpoint" | "route", id: string) {
    setProbing(id);
    setNote(null);
    try {
      const path =
        kind === "endpoint"
          ? `/api/admin/route-endpoints/${id}/probe`
          : `/api/admin/routes/${id}/probe`;
      const r = await api.post<{ ok: boolean; ms: number; note: string }>(path, {});
      setNote(r.ok ? { text: `通了，${r.ms}ms`, ok: true } : { text: `不通：${r.note}`, ok: false });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "探测失败", ok: false });
    } finally {
      setProbing(null);
    }
  }

  /// 问这个中转有哪些模型，并把线路开放但它没有的自动取消勾选。
  async function fetchModels() {
    if (!draft) return;
    setFetching(true);
    setNote(null);
    try {
      const r = await api.post<{ here: string[]; missing: string[]; upstream_total: number }>(
        "/api/admin/route-endpoints/available",
        {
          id: draft.id,
          route_id: draft.route_id,
          base_url: draft.base_url,
          api_key: draft.api_key,
        },
      );
      setFetched({ here: r.here ?? [], missing: r.missing ?? [] });
      // 直接把「它没有的」取消掉：拉取的意义就是省掉人工比对，
      // 只把结果显示出来还要人自己去点，等于没省。
      setDraft({ ...draft, enabled_models: r.here ?? [] });
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "拉取失败", ok: false });
    } finally {
      setFetching(false);
    }
  }

  async function remove(e: Endpoint) {
    if (!confirm(`删掉这个出口？\n\n${e.base_url}\n\n之后这条线路的流量不会再走它。`)) return;
    try {
      await api.del(`/api/admin/route-endpoints/${e.id}`);
      await load();
    } catch (x) {
      setNote({ text: x instanceof Error ? x.message : "删除失败", ok: false });
    }
  }

  const list = routes ?? [];
  /// 编辑出口时要知道它属于哪条线路 —— 那条线路开放的模型就是这个出口的可选范围。
  const routeOf = (id: string) => list.find((r) => r.id === id);
  const extra = list.reduce((n, r) => n + r.endpoints.length, 0);
  const broken = list.reduce(
    (n, r) => n + r.endpoints.filter((e) => e.probe_ok === false).length,
    0,
  );
  const untested = list.reduce(
    (n, r) => n + r.endpoints.filter((e) => e.probe_ok === null).length,
    0,
  );

  return (
    <div className="space-y-6">
      <PageHeader
        title="多路由"
        description="给一条线路挂几个不同的中转地址。模型、价格、账单全都跟着线路走——出口只决定这一次请求从哪儿发出去，换出口不会改用户被扣的钱。"
        actions={
          <Button variant="ghost" size="sm" onClick={() => void load()}>
            <RefreshCw /> 刷新
          </Button>
        }
      />

      <ErrorState message={err} />

      {note ? (
        <p className={cn("text-sm", note.ok ? "text-success" : "text-destructive")}>{note.text}</p>
      ) : null}

      {!routes && <TableSkeleton rows={3} columns={["30%", "20%", "20%", "20%"]} label="读取中" />}

      {routes && !list.length && (
        <EmptyState title="还没有线路" hint="先到「线路」建一条连接，再回来给它挂中转地址。" />
      )}

      {routes && list.length > 0 && (
        <>
          <SectionReveal as="section" delay={70} className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <Stat label="线路" value={num(list.length)} hint="每条各自计价" />
            <Stat label="额外出口" value={num(extra)} hint="线路自带的那个不算在内" />
            <Stat label="测下来不通" value={num(broken)} hint={broken ? "已自动排到最后" : "都通"} />
            <Stat label="还没测过" value={num(untested)} hint="排在通过的之后" />
          </SectionReveal>

          <SectionReveal as="section" delay={140} className="space-y-4">
            <p className="text-xs leading-relaxed text-muted-foreground">
              同一条线路下按 <b className="text-foreground">「能用的在前，便宜的在前」</b>{" "}
              自动排序。一个请求最多换 {TRIED_PER_REQUEST} 个出口就收手（再多客户端等不起），
              所以真正常用的是每条线路的前 {TRIED_PER_REQUEST} 个，再往后是它们都坏掉时的兜底。
            </p>

            {list.map((r) => {
              const rows = ordered(r);
              const vname = vendorName(r.vendor);
              return (
                <Card key={r.id} className={cn(!r.active && "opacity-60")}>
                  <CardHeader>
                    <VendorMark vendor={r.vendor} />
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <Truncate className="font-semibold">{r.label || "未命名"}</Truncate>
                        <LiveDot live={r.live} />
                        {!r.active && <Badge variant="outline">已停用</Badge>}
                      </div>
                      <p className="mt-0.5 text-xs text-muted-foreground">
                        {vname ? `${vname} · ` : ""}
                        {r.model_count} 个模型 · {r.protocol} 协议
                        {r.endpoints.length ? ` · ${r.endpoints.length} 个额外出口` : ""}
                      </p>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() =>
                        setDraft({
                          route_id: r.id,
                          label: "",
                          base_url: "",
                          api_key: "",
                          cost_ratio: "1",
                          note: "",
                          protocol: "",
                          active: true,
                          enabled_models: [],
                          capacity: "",
                        })
                      }
                    >
                      <Plus /> 加一个出口
                    </Button>
                  </CardHeader>

                  <Separator />

                  <ol className="divide-y divide-border">
                    {rows.map((e, i) => {
                      const beyond = i >= TRIED_PER_REQUEST;
                      const id = e?.id ?? r.id;
                      const showCut = i === TRIED_PER_REQUEST && rows.length > TRIED_PER_REQUEST;
                      return (
                        <li key={id}>
                          {showCut && (
                            <div className="flex items-center gap-2 bg-muted/40 px-5 py-1.5 text-[11px] text-muted-foreground">
                              <CircleSlash className="size-3" />
                              以下只有上面 {TRIED_PER_REQUEST} 个都失败并被判坏、掉到后面之后才轮得到
                            </div>
                          )}
                          <div
                            className={cn(
                              "flex flex-wrap items-center gap-x-3 gap-y-2 px-5 py-3 transition-colors hover:bg-accent/40",
                              // 用不到的压暗，但仍然可读、仍然可操作 —— 它们是兜底，不是垃圾。
                              beyond && "opacity-60",
                            )}
                          >
                            <span
                              className={cn(
                                "flex size-5 shrink-0 items-center justify-center rounded-md text-[11px] font-semibold tabular-nums",
                                beyond
                                  ? "bg-muted text-muted-foreground"
                                  : "bg-foreground text-background",
                              )}
                            >
                              {i + 1}
                            </span>
                            <LiveDot live={e ? e.live : r.live} />
                            <div className="min-w-0 flex-1">
                              <Truncate
                                className="font-mono text-[13px]"
                                title={e ? e.base_url : r.base_url}
                              >
                                {e ? e.base_url : r.base_url || "—"}
                              </Truncate>
                              <p className="text-xs text-muted-foreground">
                                {e ? (
                                  <>
                                    {e.label || "未命名出口"}
                                    {" · "}
                                    {e.has_key ? "自带密钥" : "用线路的密钥"}
                                    {/* 只承载一部分模型是个容易忘的设置：设完就再也看不见，
                                        然后某天有人问「为什么这个便宜出口没被用上」。 */}
                                    {e.enabled_models.length > 0 &&
                                      ` · 只有 ${e.enabled_models.length}/${r.model_count} 个模型`}
                                    {e.capacity != null && ` · 容量 ${e.capacity}`}
                                    {e.protocol ? ` · ${e.protocol} 协议` : ""}
                                    {!e.active ? " · 已停用" : ""}
                                    {e.note ? ` · ${e.note}` : ""}
                                  </>
                                ) : (
                                  "线路自带的地址"
                                )}
                              </p>
                            </div>
                            <Badge variant={e && e.cost_ratio < 1 ? "success" : "outline"}>
                              {ratioText(e ? e.cost_ratio : 1)}
                            </Badge>
                            {e ? (
                              <ProbeBadge ok={e.probe_ok} ms={e.probe_ms} note={e.probe_note} />
                            ) : (
                              <Badge variant="outline">直连</Badge>
                            )}
                            <div className="flex shrink-0 items-center gap-1">
                              <Button
                                size="sm"
                                variant="ghost"
                                disabled={probing === id}
                                onClick={() => void probe(e ? "endpoint" : "route", id)}
                              >
                                <Zap /> {probing === id ? "测…" : "测一下"}
                              </Button>
                              {e && (
                                <>
                                  <Button
                                    size="sm"
                                    variant="ghost"
                                    onClick={() =>
                                      setDraft({
                                        id: e.id,
                                        route_id: e.route_id,
                                        label: e.label,
                                        base_url: e.base_url,
                                        // 服务端不回密钥，所以这里必然是空的；空着保存 = 沿用。
                                        api_key: "",
                                        cost_ratio: String(e.cost_ratio),
                                        note: e.note,
                                        protocol: e.protocol,
                                        active: e.active,
                                        enabled_models: e.enabled_models,
                                        capacity: e.capacity == null ? "" : String(e.capacity),
                                      })
                                    }
                                  >
                                    编辑
                                  </Button>
                                  <Button
                                    size="sm"
                                    variant="ghost"
                                    aria-label="删掉这个出口"
                                    onClick={() => void remove(e)}
                                  >
                                    <Trash2 />
                                  </Button>
                                </>
                              )}
                            </div>
                          </div>
                        </li>
                      );
                    })}
                  </ol>
                </Card>
              );
            })}
          </SectionReveal>
        </>
      )}

      <Dialog
        open={!!draft}
        onOpenChange={(o) => {
          if (!o) {
            setDraft(null);
            // 不清的话，下次打开另一个出口会看到上一个的拉取结果 —— 一份看起来
            // 很可信、其实属于别人的名单。
            setFetched(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{draft?.id ? "改一个出口" : "加一个出口"}</DialogTitle>
            <DialogDescription>
              同一条线路的出口对用户完全等价——同样的模型、同样的账单，只有我的进价不同。
            </DialogDescription>
          </DialogHeader>
          {draft && (
            <div className="grid gap-4">
              <div>
                <Label htmlFor="e-url">中转地址</Label>
                <Input
                  id="e-url"
                  value={draft.base_url}
                  placeholder="https://xxx.com/v1"
                  onChange={(ev) => setDraft({ ...draft, base_url: ev.target.value })}
                />
              </div>
              <div>
                <Label htmlFor="e-key">密钥</Label>
                <Input
                  id="e-key"
                  type="password"
                  autoComplete="off"
                  value={draft.api_key}
                  placeholder={draft.id ? "留空 = 不改" : "sk-…"}
                  onChange={(ev) => setDraft({ ...draft, api_key: ev.target.value })}
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  留空就用线路自己的密钥。存进库时加密，之后任何页面都读不回来。
                </p>
              </div>
              <div className="grid gap-4 sm:grid-cols-3">
                <div>
                  <Label htmlFor="e-ratio">进价折扣</Label>
                  <Input
                    id="e-ratio"
                    value={draft.cost_ratio}
                    placeholder="0.3"
                    onChange={(ev) => setDraft({ ...draft, cost_ratio: ev.target.value })}
                  />
                  <p className="mt-1 text-xs text-muted-foreground">
                    0.3 = 三折。只决定先用谁，<b>不进用户账单</b>。
                  </p>
                </div>
                <div>
                  <Label htmlFor="e-cap">能扛多少</Label>
                  <Input
                    id="e-cap"
                    value={draft.capacity}
                    placeholder="留空 = 不填"
                    onChange={(ev) => setDraft({ ...draft, capacity: ev.target.value })}
                  />
                  {/*
                    只在「首选被限流、要挑替补」时起作用。平时所有流量都走最便宜那个，
                    这个数一点作用都没有 —— 所以不填是完全正常的默认。
                  */}
                  <p className="mt-1 text-xs text-muted-foreground">
                    同条线路下用同一把尺（RPM 或随便一个相对值）。只在别的出口被限流、
                    要挑替补时才用到。
                  </p>
                </div>
                <div>
                  <Label htmlFor="e-label">备注</Label>
                  <Input
                    id="e-label"
                    value={draft.label}
                    placeholder="转卖A"
                    onChange={(ev) => setDraft({ ...draft, label: ev.target.value })}
                  />
                </div>
              </div>
              <div>
                <Label htmlFor="e-proto">上游协议</Label>
                <Select
                  id="e-proto"
                  value={draft.protocol}
                  onChange={(ev) => setDraft({ ...draft, protocol: ev.target.value })}
                >
                  <option value="">跟线路一样</option>
                  <option value="anthropic">Anthropic 原生 /v1/messages</option>
                  <option value="openai">OpenAI 兼容 /chat/completions</option>
                </Select>
                {/*
                  协议是「这条线怎么说话」，可以和线路不同 —— 官方直连走 Anthropic 原生，
                  而最便宜的那批转卖往往只提供 OpenAI 兼容。没有这一项，那批就挂不上来。
                */}
                <p className="mt-1 text-xs text-muted-foreground">
                  便宜的转卖常常只有 OpenAI 兼容口，这里可以和线路不一样。
                </p>
              </div>

              <div>
                <div className="flex items-center justify-between">
                  <Label htmlFor="e-models">这个出口有哪些模型</Label>
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    disabled={fetching || !draft.base_url.trim()}
                    onClick={() => void fetchModels()}
                  >
                    <ListChecks /> {fetching ? "问它…" : "问它有什么"}
                  </Button>
                </div>
                {/*
                  只列线路已开放的模型：出口只能做减法。允许填线路之外的，等于从这里
                  开一个后门——那个模型没有价格、不在 IDE 列表里，但请求真会发出去。
                */}
                <div className="mt-1.5 max-h-44 overflow-y-auto rounded-lg border border-border">
                  {(routeOf(draft.route_id)?.models ?? []).map((m) => {
                    const on =
                      draft.enabled_models.length === 0 || draft.enabled_models.includes(m);
                    const absent = fetched?.missing.includes(m);
                    return (
                      <label
                        key={m}
                        className="flex cursor-pointer items-center gap-2 border-b border-border px-3 py-1.5 text-[13px] last:border-b-0 hover:bg-accent/40"
                      >
                        <input
                          type="checkbox"
                          checked={on}
                          onChange={(ev) => {
                            const all = routeOf(draft.route_id)?.models ?? [];
                            const cur = draft.enabled_models.length ? draft.enabled_models : all;
                            const next = ev.target.checked
                              ? [...cur, m]
                              : cur.filter((x) => x !== m);
                            setDraft({ ...draft, enabled_models: next });
                          }}
                        />
                        <span className="font-mono">{m}</span>
                        {absent && (
                          <Badge variant="outline" className="ml-auto border-destructive/40 text-destructive">
                            它没有
                          </Badge>
                        )}
                      </label>
                    );
                  })}
                </div>
                <p className="mt-1 text-xs text-muted-foreground">
                  全勾 = 承载这条线路的全部模型（以后线路加了新模型也自动跟着有）。
                  取消勾选的模型不会被派到这个出口——转卖商之间的货不一样，
                  派过去只会撞一个 404，而每个请求只有 2 次机会。
                </p>
              </div>

              <label className="flex cursor-pointer items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={draft.active}
                  onChange={(ev) => setDraft({ ...draft, active: ev.target.checked })}
                />
                投入轮转（取消勾选 = 留着配置但不接任何请求）
              </label>

              <div className="flex justify-end gap-2">
                <Button variant="ghost" onClick={() => setDraft(null)}>
                  取消
                </Button>
                <Button disabled={busy || !draft.base_url.trim()} onClick={() => void save()}>
                  {busy ? "保存并测试…" : "保存并测试"}
                </Button>
              </div>
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
