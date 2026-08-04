import { Fragment, useEffect, useState } from "react";
import { ChevronDown, Plus, RefreshCw } from "lucide-react";
import { Stat } from "@/components/Stat";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
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
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import { cents, num } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * 模型线路 — the provider connections every IDE request is routed through, and what each
 * enabled model costs.
 *
 * What an operator does here: watch which routes are live, pull a failing provider OUT of
 * rotation, add or re-key a connection, and price the models it exposes.
 *
 * THE FIX: `models.active` gates every routing query (models.rs:1078, 1788, 1849, 2286) and
 * `UpdateReq.active` has existed all along (models.rs:1635) — but the old edit form never sent
 * it (admin.html:1834-1850). So the only way to stop a bad provider was DELETE, which drops the
 * api key, the enabled-model set, the display names and every per-model price with it. Each row
 * now has 停用/启用, and it posts `{ active }` and nothing else: every other UpdateReq field is
 * an Option that keeps its stored value when absent (models.rs:1663-1756), so a toggle cannot
 * damage the connection. The delete dialog offers 停用 as the reversible alternative.
 *
 * PRICE PRIORITY — mirrored from the code that actually charges, compute_cost (models.rs:2666-
 * 2673): per-model override → built-in official catalogue → connection-level input/output price.
 * The connection price is a FALLBACK for models the catalogue doesn't know, not a default, so
 * this screen labels it 兜底 rather than showing it as the model's price. Note that
 * list_for_client (models.rs:1797-1805) hands the IDE a different order (override → connection →
 * catalogue) — the two disagree, and a screen whose job is "what does this cost" has to follow
 * the billing path. Worth reconciling server-side.
 *
 * SERVER TRAPS this screen has to work around, all of them worth fixing in models.rs:
 *  - admin_update re-runs its per-call price check on EVERY update (models.rs:1713), including
 *    the `{ active }`-only post — so a per-call connection with unpriced models REFUSES to be
 *    stopped until it is priced. setActive() says what to do when that 400 comes back.
 *  - admin_create never persists `per_call_micro_usd` (the INSERT at models.rs:1536-1553 omits
 *    the column), so a sub-half-cent fee entered at 新建 lands as 0 = free. Blocked client-side.
 *  - UpdateReq has no `model_id`, and allowed_ids() falls back to that legacy column when the
 *    enabled set is empty (models.rs:750) — unchecking every model cannot hide it. Flagged in
 *    the dialog when it applies.
 *
 * Deliberately left out of the old 模型系统 tab:
 *  - 渠道汇率 and 模型渠道利润计算器 — a pricing exercise, not a routing one; they move to 定价试算.
 *  - API 密钥 (apikeys) — belongs with customers, not with provider routes.
 *  - Vendor logo SVGs — six hand-pasted brand paths with hardcoded hex, none of them ours to
 *    ship; the provider is a word in the row instead.
 *  - 拉取模型 dumping the raw catalogue into a 300px scroller with seven controls per line —
 *    same controls, but the list starts at what is actually enabled.
 */

type PriceOverride = { in?: number; out?: number };
type BillingOverride = { mode?: string; per_call_cents?: number; per_call_micro_usd?: number };

/** Exactly the shape of GET /api/admin/models (models.rs:1146-1175). */
type Conn = {
  id: string;
  label?: string;
  provider?: string;
  base_url?: string;
  model_id?: string | null;
  api_key_masked?: string;
  has_key?: boolean;
  rate?: number;
  active?: boolean;
  sort?: number;
  created_at?: string;
  input_price?: number;
  output_price?: number;
  cache_read_price?: number;
  cache_create_price?: number;
  description?: string;
  enabled_models?: string[];
  billing_mode?: string;
  per_call_cents?: number;
  per_call_micro_usd?: number;
  model_names?: unknown;
  model_prices?: unknown;
  model_billing?: unknown;
  protocol?: string;
};

/** GET /api/admin/model-usage returns totals only (models.rs:1978-1991). */
type Usage = { calls?: number; spent_cents?: number };

const PROVIDERS = ["claude", "gpt", "deepseek", "gemini", "minimax", "glm", "other"];

/** The three JSON maps arrive as free-form serde_json::Value — never trust the shape. */
function asMap<T>(v: unknown): Record<string, T> {
  return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, T>) : {};
}

/**
 * Two units that format.ts deliberately does not cover, kept together so nothing formats money
 * inline anywhere else. `cents()` is integer-cent money: it renders a $0.0035 per-call fee as
 * "$0.00", which is the exact bug models.rs:550-553 documents (the field appeared to revert on
 * save). Per-call fees are micro-USD and token prices are USD per 1M — different units, not
 * cents. Move both into format.ts the moment a second screen needs them.
 */
const fee = (usd: number) => {
  if (!(usd > 0)) return "$0.00";
  if (usd >= 0.01) return `$${usd.toFixed(2)}`;
  // Sub-cent fees are the whole point of per_call_micro_usd — never round one down to $0.00.
  // Micro-USD is the storage floor, so nothing smaller than 0.000001 can actually be saved.
  return usd < 0.000001 ? "<$0.000001" : `$${usd.toFixed(6).replace(/0+$/, "")}`;
};
/** USD per 1M tokens. Bounded so an f64 out of the DB can't print as $0.30000000000000004. */
const per1M = (usd: number | undefined) => `$${Number((usd || 0).toFixed(4))}`;

/** Mirrors allowed_ids() (models.rs:750): an empty enabled set falls back to the legacy model_id. */
function allowedIds(c: Conn): string[] {
  if (c.enabled_models?.length) return c.enabled_models;
  return c.model_id ? [c.model_id] : [];
}

/** `active` is NOT NULL in the schema; treat a missing field as live rather than silently dark. */
const isOn = (c: Conn) => c.active !== false;

const MODES = ["rate", "per_call", "free"];

/** A per-model mode override, normalised the way the server reads it (models.rs:624-628). */
const ovMode = (ov: BillingOverride) => {
  const m = String(ov.mode ?? "").trim().toLowerCase();
  return MODES.includes(m) ? m : "";
};

/** Effective billing mode for one model: its own override, else the connection's. */
const billingMode = (c: Conn, ov: BillingOverride) => ovMode(ov) || c.billing_mode || "rate";

/**
 * Per-call fee in micro-USD, resolved exactly as effective_billing_micro does (models.rs:601-618):
 * per-model micro → connection micro → per-model cents → connection cents. The order matters —
 * a connection-level micro fee outranks a legacy whole-cent per-model override.
 */
function perCallMicro(c: Conn, ov: BillingOverride): number {
  const ovMicro = Number(ov.per_call_micro_usd) || 0;
  if (ovMicro > 0) return ovMicro;
  if ((c.per_call_micro_usd || 0) > 0) return c.per_call_micro_usd || 0;
  const ovCents =
    typeof ov.per_call_cents === "number" && ov.per_call_cents >= 0 ? ov.per_call_cents : null;
  return (ovCents ?? c.per_call_cents ?? 0) * 10_000;
}

const channelFeeUsd = (c: Conn) => perCallMicro(c, {}) / 1_000_000;

/**
 * The models this connection would bill nothing for — the server's own check, resolved per model
 * exactly as models.rs:1713-1742 does it. A zero CHANNEL fee is fine when each model carries its
 * own price, so only flag models that end up charging zero.
 */
function unpriced(c: Conn): string[] {
  if (c.billing_mode !== "per_call") return [];
  if (channelFeeUsd(c) > 0) return [];
  const overrides = asMap<BillingOverride>(c.model_billing);
  return allowedIds(c).filter((id) => {
    const ov = overrides[id] || {};
    const mode = billingMode(c, ov);
    if (mode === "free" || mode === "rate") return false; // points-capped, or billed by tokens
    return perCallMicro(c, ov) <= 0;
  });
}

/** What one enabled model actually costs, with the same priority the gateway uses. */
function modelCost(c: Conn, id: string): string {
  const ov = asMap<BillingOverride>(c.model_billing)[id] || {};
  const mode = billingMode(c, ov);
  if (mode === "free") return "免费额度";
  if (mode === "per_call") {
    const usd = perCallMicro(c, ov) / 1_000_000;
    return usd > 0 ? `${fee(usd)} / 次` : "未计费";
  }
  const p = asMap<PriceOverride>(c.model_prices)[id] || {};
  if ((p.in || 0) > 0 || (p.out || 0) > 0) return `${per1M(p.in)} / ${per1M(p.out)} 每 1M`;
  // The built-in catalogue beats the connection price: compute_cost only falls back to the
  // connection's input/output when the model is NOT catalogued (models.rs:2666-2673).
  if ((c.input_price || 0) > 0 || (c.output_price || 0) > 0) {
    return `官方价 · 兜底 ${per1M(c.input_price)}/${per1M(c.output_price)}`;
  }
  return "内置官方价";
}

export function Routing() {
  const [conns, setConns] = useState<Conn[]>([]);
  const [usage, setUsage] = useState<Usage>({});
  const [err, setErr] = useState("");
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState("");
  const [expanded, setExpanded] = useState("");
  // null = closed, { conn: null } = 新建, { conn } = 编辑.
  const [dialog, setDialog] = useState<{ conn: Conn | null } | null>(null);
  const [confirmDel, setConfirmDel] = useState<Conn | null>(null);
  // Delete failures have to land INSIDE the modal — a page-level error behind the overlay
  // reads as "nothing happened" on the one action that cannot be undone.
  const [delErr, setDelErr] = useState("");

  // No polling here, unlike 总览: this screen is edited, and a 30s refresh would yank the table
  // out from under an open dialog. Every mutation reloads instead.
  const load = async () => {
    try {
      const [m, u] = await Promise.all([
        api.get<Conn[] | { items?: Conn[] }>("/api/admin/models"),
        api.get<Usage>("/api/admin/model-usage").catch(() => ({}) as Usage),
      ]);
      setConns(Array.isArray(m) ? m : m?.items || []);
      setUsage(u || {});
      setErr("");
    } catch (e) {
      setErr(e instanceof Error ? e.message : "加载失败");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  async function setActive(c: Conn, active: boolean) {
    setBusyId(c.id);
    setErr("");
    try {
      // ONLY `active` — see the header comment. Sending the rest would risk overwriting fields
      // this row never loaded.
      await api.post(`/api/admin/models/${c.id}`, { active });
      await load();
    } catch (e) {
      const msg = e instanceof Error ? e.message : "操作失败";
      // admin_update runs its per-call price check on every update (models.rs:1713), so the
      // rescue action fails with a PRICING error on exactly the connections that need rescuing.
      setErr(
        unpriced(c).length
          ? `${msg}｜停用被服务端的计费校验挡住了：先进「编辑」给这些模型填「次费$」，或填渠道级「每次调用收费」，再停用。`
          : msg,
      );
    } finally {
      setBusyId("");
    }
  }

  async function remove(c: Conn) {
    setBusyId(c.id);
    setDelErr("");
    try {
      await api.del(`/api/admin/models/${c.id}`);
      setConfirmDel(null);
      await load();
    } catch (e) {
      setDelErr(e instanceof Error ? e.message : "删除失败");
    } finally {
      setBusyId("");
    }
  }

  const live = conns.filter(isOn);
  const exposed = live.reduce((a, c) => a + allowedIds(c).length, 0);

  return (
    <div>
      <h1 className="font-display text-2xl font-semibold tracking-tight">模型线路</h1>
      <p className="type-measure mt-1 text-muted-foreground">
        供应商连接和每个模型的价格。线路出问题就停用它——密钥、开放的模型和定价都留着，随时能再开。
      </p>

      {err && <p role="alert" className="mt-4 text-sm text-destructive">{err}</p>}

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat label="在转线路" value={num(live.length)} hint={`共 ${conns.length} 条连接`} />
        <Stat label="开放模型" value={num(exposed)} hint="IDE 里能选到的" />
        <Stat label="累计调用" value={num(usage.calls)} />
        {/* cost_cents 是「按倍率结算后从用户扣掉的钱」(models.rs:2715)，不是渠道成本。 */}
        <Stat label="累计计费" value={cents(usage.spent_cents)} hint="已含倍率" />
      </div>

      <section className="mt-8 rounded-xl border border-border bg-card">
        <header className="flex flex-wrap items-center justify-between gap-3 border-b border-border px-5 py-3">
          <h2 className="text-sm font-semibold">供应商连接</h2>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={load}>
              <RefreshCw /> 刷新
            </Button>
            <Button size="sm" onClick={() => setDialog({ conn: null })}>
              <Plus /> 新建连接
            </Button>
          </div>
        </header>

        {loading && <p className="px-5 py-10 text-center text-sm text-muted-foreground">加载中…</p>}

        {!loading && !conns.length && (
          <p className="px-5 py-10 text-center text-sm text-muted-foreground">
            还没有连接。新建一条，再进「编辑」勾选要开放的模型。
          </p>
        )}

        {!loading && conns.length > 0 && (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>连接</TableHead>
                <TableHead>状态</TableHead>
                <TableHead>开放模型</TableHead>
                <TableHead>计费</TableHead>
                <TableHead>密钥</TableHead>
                <TableHead className="text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {conns.map((c) => {
                const ids = allowedIds(c);
                const names = asMap<string>(c.model_names);
                const dead = unpriced(c);
                const open = expanded === c.id;
                return (
                  <Fragment key={c.id}>
                    <TableRow className={cn(!isOn(c) && "opacity-60")}>
                      <TableCell>
                        <div className="font-medium">{c.label || "未命名"}</div>
                        <div className="truncate text-xs text-muted-foreground" title={c.base_url}>
                          {c.provider || "other"} · {c.base_url || "—"}
                        </div>
                      </TableCell>
                      <TableCell>
                        {isOn(c) ? (
                          <Badge variant="success">在转</Badge>
                        ) : (
                          <Badge variant="outline">已停用</Badge>
                        )}
                      </TableCell>
                      <TableCell>
                        <button
                          type="button"
                          onClick={() => setExpanded(open ? "" : c.id)}
                          aria-expanded={open}
                          className="inline-flex items-center gap-1.5 rounded-md px-1.5 py-0.5 text-sm transition-colors hover:bg-secondary"
                        >
                          <ChevronDown
                            aria-hidden
                            className={cn(
                              "size-3.5 text-muted-foreground transition-transform",
                              open && "rotate-180",
                            )}
                          />
                          {ids.length ? `${ids.length} 个` : "未选择"}
                        </button>
                      </TableCell>
                      <TableCell className="text-sm">
                        {c.billing_mode === "per_call" ? (
                          <span className="tabular-nums">按次 {fee(channelFeeUsd(c))}/次</span>
                        ) : (
                          <span className="tabular-nums">
                            按 Token ×{c.rate ?? 1}
                            {((c.input_price || 0) > 0 || (c.output_price || 0) > 0) && (
                              <span
                                className="text-muted-foreground"
                                title="只用于内置价目表没收录的模型；收录的模型按官方价计费"
                              >
                                {" "}· 兜底 {per1M(c.input_price)}/{per1M(c.output_price)}
                              </span>
                            )}
                          </span>
                        )}
                        {dead.length > 0 && (
                          <div className="mt-0.5 text-xs text-destructive" title={dead.join("、")}>
                            {dead.length} 个模型不计费
                          </div>
                        )}
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground">
                        {c.has_key === false ? "未配置" : c.api_key_masked || "—"}
                      </TableCell>
                      <TableCell>
                        <div className="flex justify-end gap-2">
                          <Button
                            size="sm"
                            variant="secondary"
                            disabled={busyId === c.id}
                            onClick={() => setActive(c, !isOn(c))}
                          >
                            {isOn(c) ? "停用" : "启用"}
                          </Button>
                          <Button size="sm" variant="outline" onClick={() => setDialog({ conn: c })}>
                            编辑
                          </Button>
                          <Button
                            size="sm"
                            variant="outline"
                            className="border-destructive/40 text-destructive hover:bg-destructive/10"
                            onClick={() => {
                              setDelErr("");
                              setConfirmDel(c);
                            }}
                          >
                            删除
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>

                    {open && (
                      <TableRow className="hover:bg-transparent">
                        <TableCell colSpan={6} className="bg-muted/50">
                          {ids.length ? (
                            <ul className="grid gap-2 sm:grid-cols-2">
                              {ids.map((id) => (
                                <li
                                  key={id}
                                  className="flex items-baseline justify-between gap-3 rounded-lg border border-border bg-card px-3 py-2"
                                >
                                  <span className="min-w-0">
                                    <span className="block truncate font-mono text-xs" title={id}>
                                      {id}
                                    </span>
                                    {names[id] && (
                                      <span className="block truncate text-xs text-muted-foreground">
                                        显示为 {names[id]}
                                      </span>
                                    )}
                                  </span>
                                  <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                                    {modelCost(c, id)}
                                  </span>
                                </li>
                              ))}
                            </ul>
                          ) : (
                            <p className="text-sm text-muted-foreground">
                              这条连接还没勾选模型，IDE 的模型列表里看不到它。进「编辑」拉取并勾选。
                            </p>
                          )}
                        </TableCell>
                      </TableRow>
                    )}
                  </Fragment>
                );
              })}
            </TableBody>
          </Table>
        )}
      </section>

      {dialog && (
        <ConnectionDialog
          key={dialog.conn?.id || "new"}
          conn={dialog.conn}
          onClose={() => setDialog(null)}
          onSaved={() => {
            setDialog(null);
            load();
          }}
        />
      )}

      {confirmDel && (
        <Dialog open onOpenChange={(o) => !o && setConfirmDel(null)}>
          <DialogContent className="max-w-md">
            <DialogHeader>
              <DialogTitle>删除「{confirmDel.label || "未命名"}」？</DialogTitle>
              <DialogDescription>
                API Key、{allowedIds(confirmDel).length} 个已开放模型、它们的显示名和单独定价会一起消失，
                无法恢复。只是想让它别再接单的话，用「停用」——线路立刻撤出轮转，配置原样留着。
              </DialogDescription>
            </DialogHeader>
            {delErr && <p role="alert" className="text-sm text-destructive">{delErr}</p>}
            <div className="flex flex-wrap justify-end gap-3">
              <Button variant="ghost" onClick={() => setConfirmDel(null)}>
                取消
              </Button>
              {isOn(confirmDel) && (
                <Button
                  variant="secondary"
                  disabled={busyId === confirmDel.id}
                  onClick={() => {
                    const c = confirmDel;
                    setConfirmDel(null);
                    setActive(c, false);
                  }}
                >
                  改为停用
                </Button>
              )}
              <Button
                variant="outline"
                className="border-destructive/40 text-destructive hover:bg-destructive/10"
                disabled={busyId === confirmDel.id}
                onClick={() => remove(confirmDel)}
              >
                仍然删除
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}

/** One editable line of the enabled-model list. Four parallel maps flattened into one row. */
type Row = { id: string; on: boolean; name: string; pin: string; pout: string; mode: string; fee: string };

function initialRows(c: Conn | null): Row[] {
  if (!c) return [];
  const names = asMap<string>(c.model_names);
  const prices = asMap<PriceOverride>(c.model_prices);
  const billing = asMap<BillingOverride>(c.model_billing);
  return allowedIds(c).map((id) => {
    const p = prices[id] || {};
    const b = billing[id] || {};
    // This box is the model's OWN fee. Never prefill it from the connection — saving would
    // then write the channel fee back as a per-model override that stops following it.
    const micro = Number(b.per_call_micro_usd) || (Number(b.per_call_cents) || 0) * 10_000;
    return {
      id,
      on: true,
      name: names[id] || "",
      pin: p.in ? String(p.in) : "",
      pout: p.out ? String(p.out) : "",
      mode: ovMode(b),
      fee: micro > 0 ? String(micro / 1_000_000) : "",
    };
  });
}

const nz = (s: string) => {
  const n = parseFloat(s);
  return Number.isFinite(n) && n > 0 ? n : 0;
};

function ConnectionDialog({
  conn,
  onClose,
  onSaved,
}: {
  conn: Conn | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const editing = conn !== null;
  const [label, setLabel] = useState(conn?.label || "");
  const [provider, setProvider] = useState((conn?.provider || "deepseek").toLowerCase());
  const [baseUrl, setBaseUrl] = useState(conn?.base_url || "");
  const [protocol, setProtocol] = useState(conn?.protocol || "anthropic");
  const [apiKey, setApiKey] = useState("");
  const [active, setActiveField] = useState(conn ? isOn(conn) : true);
  const [description, setDescription] = useState(conn?.description || "");
  const [mode, setMode] = useState(conn?.billing_mode === "per_call" ? "per_call" : "rate");
  const [rate, setRate] = useState(String(conn?.rate ?? 1));
  const [inPrice, setInPrice] = useState(String(conn?.input_price ?? 0));
  const [outPrice, setOutPrice] = useState(String(conn?.output_price ?? 0));
  const [cacheRead, setCacheRead] = useState(String(conn?.cache_read_price ?? 0));
  const [cacheCreate, setCacheCreate] = useState(String(conn?.cache_create_price ?? 0));
  const [perCall, setPerCall] = useState(String(conn ? channelFeeUsd(conn) : 0.2));
  const [rows, setRows] = useState<Row[]>(() => initialRows(conn));
  const [hint, setHint] = useState("");
  const [fetching, setFetching] = useState(false);
  const [busy, setBusy] = useState(false);
  const [formErr, setFormErr] = useState("");

  const patch = (id: string, part: Partial<Row>) =>
    setRows((prev) => prev.map((r) => (r.id === id ? { ...r, ...part } : r)));

  /** Ask the provider what this key can actually call, then merge — never discard local edits. */
  async function fetchCatalog() {
    if (!conn) return;
    setFetching(true);
    setFormErr("");
    try {
      const r = await api.get<{ models?: string[]; enabled?: string[] }>(
        `/api/admin/models/${conn.id}/available`,
      );
      const stored = new Set(r.enabled || []);
      setRows((prev) => {
        const seen = new Set(prev.map((x) => x.id));
        const added = (r.models || [])
          .filter((id) => !seen.has(id))
          .map<Row>((id) => ({
            id,
            on: stored.has(id),
            name: "",
            pin: "",
            pout: "",
            mode: "",
            fee: "",
          }));
        return [...prev, ...added];
      });
      setHint(`供应商返回 ${(r.models || []).length} 个模型`);
    } catch (e) {
      setFormErr(e instanceof Error ? e.message : "拉取失败");
    } finally {
      setFetching(false);
    }
  }

  async function save() {
    if (!label.trim() || !baseUrl.trim()) {
      setFormErr("名称和 baseUrl 必填");
      return;
    }
    const usd = nz(perCall);
    // admin_create's INSERT omits per_call_micro_usd (models.rs:1536-1553), so on 新建 the fee
    // survives only as whole cents — anything under half a cent is stored as 0 = free.
    if (!conn && mode === "per_call" && usd > 0 && Math.round(usd * 100) === 0) {
      setFormErr(`新建时「每次调用收费」只能存到分，${fee(usd)} 会被存成 0（等于不收费）。先填 ≥ $0.01 创建，再进「编辑」改成精确金额。`);
      return;
    }
    const on = rows.filter((r) => r.on);
    // A one-sided price override is not "half configured", it is $0 for the other side:
    // compute_cost takes the pair as soon as either number is > 0 (models.rs:2669-2673).
    const half = on.find((r) => (nz(r.pin) > 0) !== (nz(r.pout) > 0));
    if (conn && half) {
      setFormErr(`「${half.id}」只填了入价或出价——覆盖价是成对生效的，另一边会按 $0 计费。两个都填，或都留空用官方价。`);
      return;
    }
    setBusy(true);
    setFormErr("");
    // A blank 倍率 must not become 0: compute_cost multiplies the whole bill by it
    // (models.rs:2715), so 0 makes every token call free. Blank → 1; an explicit 0 is honoured.
    const parsedRate = parseFloat(rate);
    const rateVal = Number.isFinite(parsedRate) && parsedRate >= 0 ? parsedRate : 1;
    const base = {
      label: label.trim(),
      provider,
      base_url: baseUrl.trim(),
      description: description.trim(),
      billing_mode: mode,
      rate: rateVal,
      input_price: nz(inPrice),
      output_price: nz(outPrice),
      cache_read_price: nz(cacheRead),
      cache_create_price: nz(cacheCreate),
      // Both units stay in sync: the paid path still settles whole cents, free models read micro.
      per_call_micro_usd: Math.round(usd * 1_000_000),
      per_call_cents: Math.max(0, Math.round(usd * 100)),
    };
    try {
      if (!conn) {
        // ModelReq (models.rs:1503-1519) — no protocol, no active, no enabled set on create.
        await api.post("/api/admin/models", { ...base, api_key: apiKey.trim() });
      } else {
        const body: Record<string, unknown> = {
          ...base,
          active,
          protocol,
          enabled_models: on.map((r) => r.id),
          // These three replace the whole stored map, so only keep entries for models that are
          // still exposed — config for a model you unchecked is dead weight.
          model_names: Object.fromEntries(on.filter((r) => r.name.trim()).map((r) => [r.id, r.name.trim()])),
          model_prices: Object.fromEntries(
            on
              .filter((r) => nz(r.pin) > 0 || nz(r.pout) > 0)
              .map((r) => [r.id, { in: nz(r.pin), out: nz(r.pout) }]),
          ),
          model_billing: Object.fromEntries(
            on
              .filter((r) => r.mode || nz(r.fee) > 0)
              .map((r) => {
                const micro = Math.round(nz(r.fee) * 1_000_000);
                const entry: BillingOverride = {};
                if (r.mode) entry.mode = r.mode;
                if (micro > 0) {
                  entry.per_call_micro_usd = micro;
                  entry.per_call_cents = Math.max(1, Math.round(micro / 10_000));
                }
                return [r.id, entry];
              }),
          ),
        };
        if (apiKey.trim()) body.api_key = apiKey.trim();
        await api.post(`/api/admin/models/${conn.id}`, body);
      }
      onSaved();
    } catch (e) {
      setFormErr(e instanceof Error ? e.message : "保存失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>{editing ? `编辑：${conn?.label || "未命名"}` : "新建供应商连接"}</DialogTitle>
          <DialogDescription>
            {editing
              ? "密钥留空就是不改。停用只影响路由，下面的价格和模型都保留。"
              : "先建连接，保存后再进「编辑」拉取并勾选要开放的模型。新连接默认在转。"}
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 sm:grid-cols-2">
          <div>
            <Label htmlFor="cd-label">名称</Label>
            <Input
              id="cd-label"
              value={label}
              autoFocus
              onChange={(e) => setLabel(e.target.value)}
              placeholder="如 官方 DeepSeek"
            />
          </div>
          <div>
            <Label htmlFor="cd-provider">品牌</Label>
            <Select id="cd-provider" value={provider} onChange={(e) => setProvider(e.target.value)}>
              {PROVIDERS.map((p) => (
                <option key={p} value={p}>
                  {p}
                </option>
              ))}
            </Select>
          </div>
          <div className="sm:col-span-2">
            <Label htmlFor="cd-base">baseUrl</Label>
            <Input
              id="cd-base"
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="https://api.deepseek.com/v1"
            />
          </div>
          <div>
            <Label htmlFor="cd-key">API Key{editing && "（留空=不改）"}</Label>
            <Input
              id="cd-key"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={editing ? conn?.api_key_masked || "sk-…" : "sk-…"}
              autoComplete="off"
            />
          </div>
          {editing && (
            <div>
              <Label htmlFor="cd-protocol">上游协议</Label>
              <Select id="cd-protocol" value={protocol} onChange={(e) => setProtocol(e.target.value)}>
                <option value="anthropic">Anthropic 原生 /v1/messages</option>
                <option value="openai">OpenAI 兼容 /chat/completions</option>
              </Select>
            </div>
          )}
          <div className="sm:col-span-2">
            <Label htmlFor="cd-desc">描述（IDE 选模型时显示）</Label>
            <Input
              id="cd-desc"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="如 擅长复杂推理与编程"
            />
          </div>
        </div>

        {editing && (
          <label className="flex items-start gap-3 rounded-lg border border-border p-4">
            <Checkbox
              className="mt-1"
              checked={active}
              onChange={(e) => setActiveField(e.target.checked)}
            />
            <span className="text-sm">
              <span className="font-medium">投入轮转</span>
              <span className="mt-0.5 block text-muted-foreground">
                取消勾选＝这条线路不再接任何请求，IDE 的模型列表也不再出现它的模型。密钥和定价原样保留。
              </span>
            </span>
          </label>
        )}

        <div className="rounded-lg border border-border p-4">
          <Label htmlFor="cd-mode">计费方式</Label>
          <Select id="cd-mode" value={mode} onChange={(e) => setMode(e.target.value)}>
            <option value="rate">按 Token（真实成本 × 倍率）</option>
            <option value="per_call">按次（每次调用固定收费）</option>
          </Select>
          {mode === "rate" ? (
            <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              <div>
                <Label htmlFor="cd-rate">倍率</Label>
                <Input id="cd-rate" type="number" min="0" step="0.1" value={rate} onChange={(e) => setRate(e.target.value)} />
              </div>
              <div>
                <Label htmlFor="cd-in">输入价 $/1M</Label>
                <Input id="cd-in" type="number" min="0" step="0.01" value={inPrice} onChange={(e) => setInPrice(e.target.value)} />
              </div>
              <div>
                <Label htmlFor="cd-out">输出价 $/1M</Label>
                <Input id="cd-out" type="number" min="0" step="0.01" value={outPrice} onChange={(e) => setOutPrice(e.target.value)} />
              </div>
              <div>
                <Label htmlFor="cd-cr">缓存读取 $/1M</Label>
                <Input id="cd-cr" type="number" min="0" step="0.01" value={cacheRead} onChange={(e) => setCacheRead(e.target.value)} />
              </div>
              <div>
                <Label htmlFor="cd-cc">缓存写入 $/1M</Label>
                <Input id="cd-cc" type="number" min="0" step="0.01" value={cacheCreate} onChange={(e) => setCacheCreate(e.target.value)} />
              </div>
            </div>
          ) : (
            <div className="mt-4 sm:max-w-56">
              <Label htmlFor="cd-percall">每次调用收费 $</Label>
              <Input
                id="cd-percall"
                type="number"
                min="0"
                step="0.000001"
                value={perCall}
                onChange={(e) => setPerCall(e.target.value)}
              />
            </div>
          )}
          <p className="mt-3 text-xs text-muted-foreground">
            倍率是加价，3 = 按真实成本的 3 倍收；留空按 1 算，填 0 就是一分不收。
            输入价 / 输出价是「兜底价」：内置价目表收录的模型一律按官方价算，只有没收录的才用这里的数。
            缓存读取默认 0.1× 输入价，缓存写入默认 1.25× 输入价。
          </p>
        </div>

        {editing && (
          <div>
            <div className="mb-2 flex flex-wrap items-center justify-between gap-2">
              <Label className="mb-0">开放的模型</Label>
              <div className="flex items-center gap-3">
                {hint && <span className="text-xs text-muted-foreground">{hint}</span>}
                <Button
                  variant="outline"
                  size="sm"
                  disabled={fetching || conn?.has_key === false}
                  onClick={fetchCatalog}
                >
                  {fetching ? "拉取中…" : "拉取可用模型"}
                </Button>
              </div>
            </div>
            <div className="max-h-72 overflow-auto rounded-lg border border-border p-1.5">
              {rows.length ? (
                rows.map((r) => (
                  <div key={r.id} className="flex min-w-[38rem] items-center gap-2 rounded-md px-2 py-1.5 hover:bg-secondary/60">
                    <Checkbox
                      checked={r.on}
                      aria-label={`开放 ${r.id}`}
                      onChange={(e) => patch(r.id, { on: e.target.checked })}
                    />
                    <span className="min-w-0 flex-1 truncate font-mono text-xs" title={r.id}>
                      {r.id}
                    </span>
                    <Input
                      className="h-9 w-28 shrink-0 px-2.5 text-sm"
                      value={r.name}
                      placeholder="显示名"
                      aria-label={`${r.id} 显示名`}
                      onChange={(e) => patch(r.id, { name: e.target.value })}
                    />
                    <Input
                      className="h-9 w-20 shrink-0 px-2.5 text-sm"
                      type="number"
                      min="0"
                      step="0.01"
                      value={r.pin}
                      placeholder="入价"
                      aria-label={`${r.id} 输入价`}
                      onChange={(e) => patch(r.id, { pin: e.target.value })}
                    />
                    <Input
                      className="h-9 w-20 shrink-0 px-2.5 text-sm"
                      type="number"
                      min="0"
                      step="0.01"
                      value={r.pout}
                      placeholder="出价"
                      aria-label={`${r.id} 输出价`}
                      onChange={(e) => patch(r.id, { pout: e.target.value })}
                    />
                    <div className="w-28 shrink-0">
                      <Select
                        className="h-9 pl-2.5 pr-8 text-sm"
                        value={r.mode}
                        aria-label={`${r.id} 计费方式`}
                        onChange={(e) => patch(r.id, { mode: e.target.value })}
                      >
                        <option value="">跟随渠道</option>
                        <option value="rate">按 Token</option>
                        <option value="per_call">按次</option>
                        <option value="free">免费</option>
                      </Select>
                    </div>
                    <Input
                      className="h-9 w-20 shrink-0 px-2.5 text-sm"
                      type="number"
                      min="0"
                      step="0.000001"
                      value={r.fee}
                      placeholder="次费$"
                      aria-label={`${r.id} 每次收费`}
                      onChange={(e) => patch(r.id, { fee: e.target.value })}
                    />
                  </div>
                ))
              ) : (
                <p className="px-2 py-6 text-center text-sm text-muted-foreground">
                  还没有模型。点「拉取可用模型」，勾上要开放进 IDE 的。
                </p>
              )}
            </div>
            {conn?.model_id && !rows.some((r) => r.on) && (
              <p role="alert" className="mt-2 text-xs text-destructive">
                一个都不勾也关不掉这条线路：勾选集为空时，服务端会退回旧的单模型字段「{conn.model_id}」，
                IDE 里仍然看得到它。要彻底停掉，请用列表里的「停用」。
              </p>
            )}
            <p className="mt-2 text-xs text-muted-foreground">
              「拉取可用模型」用的是已保存的密钥，刚填的新密钥要先保存才能拉。
              显示名只改 IDE 里的叫法，调用仍用原始 id。入价 / 出价要么都填、要么都留空——填了就以你填的为准（倍率照样叠加），
              留空才按内置官方价。选「按次」的模型必须填次费，否则服务端会拒绝保存；「免费」按次费折算成每日免费点数扣，不动钱包。
            </p>
          </div>
        )}

        {formErr && <p role="alert" className="text-sm text-destructive">{formErr}</p>}

        <div className="flex justify-end gap-3">
          <Button variant="ghost" onClick={onClose}>
            取消
          </Button>
          <Button disabled={busy} onClick={save}>
            {busy ? "保存中…" : editing ? "保存" : "创建"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
