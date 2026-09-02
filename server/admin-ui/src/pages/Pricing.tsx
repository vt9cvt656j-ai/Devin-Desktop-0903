import { useEffect, useMemo, useRef, useState, type ComponentProps, type ReactNode } from "react";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { SectionReveal } from "@/components/motion/section-reveal";
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
import { Select } from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Truncate,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Panel } from "@/components/Panel";
import { PlanHealthTab } from "@/components/PlanHealthTab";
import { api } from "@/lib/api";
import { cents, num, when } from "@/lib/format";

/**
 * 定价试算 — the pricing lab.
 *
 * What an operator does here: before changing a connection's 倍率 or putting a price on a quota
 * package, answer two questions with the server's own arithmetic. (1) 套餐额度: if a customer
 * burns a whole $N package on this model, what does the upstream channel actually cost me in ¥,
 * and what multiplier / price would hit my target margin? (2) Token 用量: for a concrete workload
 * (calls × tokens), what does the channel cost, what does the wallet deduct, and what does that
 * look like as visible user quota. Both call the real endpoints — /api/admin/quota-estimate and
 * /api/admin/model-estimate — so the price priority, cache-price fallbacks, connection multiplier
 * and 6.63:1 quota ratio are the server's, never re-implemented here.
 *
 * Its own route because it is a thinking tool, not a monitoring screen: nothing here polls, and
 * nothing here changes what a user is charged. In the old console it was two boxes buried at the
 * bottom of 模型系统, under the connection editor.
 *
 * Deliberately left out of the old version:
 *  - the 反向汇率 column (it was literally 1 ÷ the column next to it, printed to 6 decimals);
 *  - the 计算 button whose label changed between modes — the estimate is debounced and runs as
 *    you type, which is the whole point of a lab;
 *  - the ~15 inputs stacked in one wall: parameters now sit in a left column and the answer owns
 *    the top row and the right column;
 *  - window.confirm() for delete, and the toast-per-action pattern.
 *
 * Three rules this screen holds to, because it is the screen a price gets set from:
 *  - a number on display is either current or visibly dimmed — never silently stale;
 *  - "还没有…" is only ever said about a list that actually loaded;
 *  - a write that succeeded is never reported as a failure just because the refresh after it
 *    failed.
 */

// 「套餐体检」是默认页：它回答的是「额度给多少合适」，而那才是运营真正要问的。
// 另外两页是拿来核对单条的，属于工具而不是结论。
type Mode = "health" | "quota" | "token";

type ChannelRate = {
  id: string;
  name: string;
  /** 对应哪个中转域名。**空 = 假想渠道**，一条真实流量都没承载过。 */
  host?: string;
  usd_per_cny: number;
  note?: string | null;
  created_at?: string;
  updated_at?: string;
};

/** Only the fields this screen needs from GET /api/admin/models. */
type Connection = {
  id: string;
  label?: string;
  rate?: number;
  billing_mode?: string;
  enabled_models?: string[] | null;
  model_id?: string | null;
  model_names?: Record<string, string> | null;
};

/** Both estimate endpoints echo back what they resolved, so the result can name itself. */
type EstimateBase = {
  channel: { name: string; usd_per_cny: number };
  connection: { label: string; rate: number; billing_mode: string };
  model: { id: string; name: string };
};

type QuotaResult = EstimateBase & {
  visible_quota_usd: number;
  quota_raw_usd: number;
  quota_raw_usd_per_visible_usd: number;
  provider_usd_capacity: number;
  channel_cost_cny: number;
  sales_cny: number;
  profit_cny: number;
  margin_percent: number;
  break_even_sales_cny: number;
  target_sales_cny: number;
  break_even_multiplier: number;
  break_even_multiplier_rounded: number;
  target_margin_percent: number;
  target_multiplier: number;
  target_multiplier_rounded: number;
  safe_visible_quota_usd: number;
  /** "loss" | "below_target" | "healthy" — left wide on purpose, see `verdict`. */
  status?: string;
};

type TokenResult = EstimateBase & {
  calls: number;
  tokens_per_call: { input: number; output: number; cache_read: number; cache_creation: number };
  prices_per_million: {
    input: number;
    output: number;
    cache_read: number;
    cache_creation: number;
    source?: string;
  };
  provider_usd_per_call: number;
  provider_usd_total: number;
  channel_cost_cny: number;
  billed_cents_per_call: number;
  visible_quota_usd: number;
  quota_raw_usd_per_visible_usd: number;
  sales_cny?: number | null;
  profit_cny?: number | null;
  margin_percent?: number | null;
  break_even_cny: number;
};

type Opt = {
  key: string;
  connection_id: string;
  model_id: string;
  connection: string;
  display: string;
  rate: number;
  billing_mode: string;
};

/**
 * lib/format.ts owns $-from-cents and dates and is used for both below. It has no ¥ helper and no
 * high-precision USD, and this is the only screen that needs either — a channel cost can be
 * $0.00000123. They move into lib/format.ts the day a second screen needs them.
 *
 * `min` never exceeds `max` at any call site below; if that ever changes, toLocaleString throws
 * RangeError rather than misformatting, so the clamp here is the cheap insurance.
 */
const dec = (v: number | null | undefined, max = 2, min = 0) =>
  v == null || !Number.isFinite(v)
    ? "—"
    : v.toLocaleString("en-US", {
        minimumFractionDigits: Math.min(min, max),
        maximumFractionDigits: max,
      });
const cny = (v: number | null | undefined) =>
  v == null || !Number.isFinite(v) ? "—" : `${v < 0 ? "-" : ""}¥${dec(Math.abs(v), 2, 2)}`;
const usd = (v: number | null | undefined, max = 2) =>
  v == null || !Number.isFinite(v) ? "—" : `$${dec(v, max, 2)}`;
const mul = (v: number | null | undefined, max = 2) =>
  v == null || !Number.isFinite(v) ? "—" : `×${dec(v, max)}`;
const pct = (v: number | null | undefined) =>
  v == null || !Number.isFinite(v) ? "—" : `${dec(v, 2)}%`;
// NOT an exchange rate — a relay PURCHASE rate: how many dollars of upstream credit ¥1 buys.
// The schema says so in words ("usd_per_cny means: 1 CNY buys this many raw channel USD") and the
// arithmetic (cost_cny = usd / rate) is correct for that reading. Only the wording was inherited
// from FX, and it was actively harmful: the field used to prefill "6.63", the CNY-per-USD FX
// number, which an operator would reasonably type — understating channel cost about 48x with no
// error and a plausible-looking result. new-api models these as two separate options for exactly
// this reason: Price (CNY per $1 of credit) and USDExchangeRate (display only).
const buyRate = (v: number | null | undefined) => `¥1 买 ${dec(v, 4)} 美元额度`;

/**
 * 价格来源的中文名。键必须和服务端 `effective_token_prices` 回的三个值逐字一致。
 *
 * 这张表漂过：服务端 2026-08-20 收敛三条价格阶梯时把 `official_catalog` / `connection_fallback`
 * 改成了 `catalog` / `backend`，这里没跟上，于是页面直接把英文原词显示出来 ——
 * 不报错，只是那一格突然变成 "catalog"。有测试钉着两边一致。
 */
const PRICE_SOURCE: Record<string, string> = {
  model_override: "单模型自定义价",
  catalog: "服务端官方价",
  backend: "连接兜底价",
};

const whole = (s: string) => {
  const v = Number(s);
  return Number.isFinite(v) ? Math.max(0, Math.trunc(v)) : 0;
};
/** Blank is not zero: an empty box means "unanswered", never a real parameter value. */
const positive = (s: string) => {
  const t = s.trim();
  if (!t) return null;
  const v = Number(t);
  return Number.isFinite(v) && v > 0 ? v : null;
};
const msg = (e: unknown, fallback: string) => (e instanceof Error ? e.message : fallback);


function Row({ label, value, hint }: { label: string; value: ReactNode; hint?: ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4 px-5 py-3 text-sm">
      <span className="min-w-0 text-muted-foreground">{label}</span>
      <span className="shrink-0 text-right">
        <span className="block font-medium tabular-nums">{value}</span>
        {hint && <span className="mt-0.5 block text-xs text-muted-foreground">{hint}</span>}
      </span>
    </div>
  );
}

function Field({
  id,
  label,
  hint,
  ...rest
}: ComponentProps<"input"> & { id: string; label: string; hint?: string }) {
  return (
    <div>
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} {...rest} />
      {hint && <p className="mt-1.5 text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

export function Pricing() {
  const [mode, setMode] = useState<Mode>("health");
  const [channels, setChannels] = useState<ChannelRate[]>([]);
  const [conns, setConns] = useState<Connection[]>([]);
  const [channelId, setChannelId] = useState("");
  const [optionKey, setOptionKey] = useState("");
  const [loadErr, setLoadErr] = useState("");
  // "The list is empty" and "the list never arrived" look identical on screen unless these are
  // tracked. Telling an operator 还没有渠道购买价 after a failed GET invites a duplicate channel.
  const [ratesLoaded, setRatesLoaded] = useState(false);
  const [modelsLoaded, setModelsLoaded] = useState(false);

  // 套餐额度 inputs
  // Defaults mirror a REAL plan (power) so the page does not open on a fabricated loss.
  // The old $1000-sold-for-¥288 pairing matches nothing in plan_spec and rendered -187.76%.
  // 这三个不写死。
  //
  // 原来是 "271" / "488" / "20"，注释说是「照抄主力档」，而主力档真值是面值 $210 / 售价 ¥295 ——
  // 抄的时候就不对，之后价目改过更不会跟着动。一进页面就用它们算出一个「净利润 ¥308」，
  // 那是一笔**不存在的生意**的利润，而它是这一屏最醒目的数。
  // 现在等真实套餐到货再播种（下面那个 effect），播不到就留空，让人自己填。
  const [quotaUsd, setQuotaUsd] = useState("");
  const [quotaSales, setQuotaSales] = useState("");
  const [targetMargin, setTargetMargin] = useState("20");
  // Token 用量 inputs
  const [calls, setCalls] = useState("1");
  const [inTok, setInTok] = useState("100000");
  const [outTok, setOutTok] = useState("10000");
  const [cacheRead, setCacheRead] = useState("0");
  const [cacheWrite, setCacheWrite] = useState("0");
  const [tokenSales, setTokenSales] = useState("");

  const [quota, setQuota] = useState<QuotaResult | null>(null);
  const [token, setToken] = useState<TokenResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [calcErr, setCalcErr] = useState("");
  const seq = useRef(0);

  // 渠道购买价 form + dialogs. The inline form and the two dialogs get their own error slots —
  // one shared string leaks a failed 创建 into the delete-confirm dialog and vice versa.
  const [crName, setCrName] = useState("");
  const [crRate, setCrRate] = useState("");
  const [crNote, setCrNote] = useState("");
  const [formErr, setFormErr] = useState("");
  const [dialogErr, setDialogErr] = useState("");
  const [crBusy, setCrBusy] = useState(false);
  const [editing, setEditing] = useState<ChannelRate | null>(null);
  const [editName, setEditName] = useState("");
  const [editRate, setEditRate] = useState("");
  const [editNote, setEditNote] = useState("");
  const [removing, setRemoving] = useState<ChannelRate | null>(null);

  /** Throws on failure; every caller decides for itself whether that is a write error. */
  const loadRates = async () => {
    const r = await api.get<ChannelRate[] | { items?: ChannelRate[] }>("/api/admin/channel-rates");
    setChannels(Array.isArray(r) ? r : r?.items || []);
    setRatesLoaded(true);
    setLoadErr("");
  };

  // allSettled, not all: a broken /api/admin/models must not also throw away the channel rates
  // that loaded fine next to it.
  // 用**真实套餐**给那两个输入框播种：面值额度和售价都来自 /api/admin/plan-health
  // （它就是 plan_quotas × prices 现算的）。挑最贵那一档，那是走量的位置。
  // 播不到就留空 —— 空着让人填，好过给一笔不存在的生意算利润。
  useEffect(() => {
    let alive = true;
    api.get<{ plans?: { visible_usd?: number; price_cny?: number | null }[] }>("/api/admin/plan-health")
      .then((r) => {
        if (!alive) return;
        const top = (r.plans || [])
          .filter((p) => (p.price_cny || 0) > 0 && (p.visible_usd || 0) > 0)
          .sort((a, b) => (b.price_cny || 0) - (a.price_cny || 0))[0];
        if (!top) return;
        setQuotaUsd((v) => v || String(Math.round(top.visible_usd!)));
        setQuotaSales((v) => v || String(top.price_cny!));
      })
      .catch(() => {});
    return () => { alive = false; };
  }, []);

  useEffect(() => {
    let alive = true;
    (async () => {
      const [rates, models] = await Promise.allSettled([
        api.get<ChannelRate[] | { items?: ChannelRate[] }>("/api/admin/channel-rates"),
        api.get<Connection[] | { items?: Connection[] }>("/api/admin/models"),
      ]);
      if (!alive) return;
      const failed: string[] = [];
      if (rates.status === "fulfilled") {
        const v = rates.value;
        setChannels(Array.isArray(v) ? v : v?.items || []);
        setRatesLoaded(true);
      } else {
        failed.push(msg(rates.reason, "渠道购买价加载失败"));
      }
      if (models.status === "fulfilled") {
        const v = models.value;
        setConns(Array.isArray(v) ? v : v?.items || []);
        setModelsLoaded(true);
      } else {
        failed.push(msg(models.reason, "模型连接加载失败"));
      }
      setLoadErr(failed.join(" · "));
    })();
    return () => {
      alive = false;
    };
  }, []);

  /** One row per (connection, opened model) — the same pairing the estimate endpoints expect. */
  const options = useMemo<Opt[]>(
    () =>
      conns.flatMap((c) => {
        const ids = (c.enabled_models?.length ? c.enabled_models : c.model_id ? [c.model_id] : [])
          .filter((id): id is string => !!id);
        return ids.map((id) => ({
          key: `${c.id}::${id}`,
          connection_id: c.id,
          model_id: id,
          connection: c.label || c.id,
          display: c.model_names?.[id] || id,
          rate: Number(c.rate) || 0,
          billing_mode: c.billing_mode || "rate",
        }));
      }),
    [conns],
  );

  const groups = useMemo(() => {
    const out: { label: string; items: Opt[] }[] = [];
    for (const o of options) {
      const group = out.find((g) => g.label === o.connection);
      if (group) group.items.push(o);
      else out.push({ label: o.connection, items: [o] });
    }
    return out;
  }, [options]);

  // Selection is derived, not synced in an effect: a deleted rate or a model that stopped being
  // exposed falls back to the first row on the same render, with no blank-select frame.
  // 默认**不选假想渠道**。
  //
  // 原来是 `channels[0]`，按 created_at 排第一的恰好是「渠道A · ¥1 买 $10」——
  // host 是空的（迁移原话：空 host 是假想渠道），生产 60 天一条流量都没走过它。
  // 而它的购买价比真实渠道好十倍，于是这一屏一打开就按最好的价算，永远显示赚钱。
  // 有 host 的才是真的在跑的，优先选它们里最贵的那条（保守）。
  const realChannels = channels.filter((c) => (c.host || "").trim());
  const defaultChannel =
    realChannels.slice().sort((a, b) => a.usd_per_cny - b.usd_per_cny)[0] || channels[0];
  const activeChannelId = channels.some((c) => c.id === channelId)
    ? channelId
    : defaultChannel?.id || "";
  const activeOptionKey = options.some((o) => o.key === optionKey) ? optionKey : options[0]?.key || "";

  const opt = options.find((o) => o.key === activeOptionKey);
  const perCall = opt?.billing_mode === "per_call";
  const quotaReady = !!opt && !perCall && opt.rate > 0;

  // Debounced re-estimate. Every value the server reads is in the dep list, including `channels`
  // (editing a rate must re-run the estimate even though only its id is sent).
  useEffect(() => {
    const channel = channels.find((c) => c.id === activeChannelId);
    const picked = options.find((o) => o.key === activeOptionKey);
    if (!channel || !picked) {
      setQuota(null);
      setToken(null);
      setCalcErr("");
      setBusy(false);
      return;
    }

    let body: Record<string, unknown>;
    let problem = "";
    if (mode === "quota") {
      const q = positive(quotaUsd);
      const s = positive(quotaSales);
      const marginText = targetMargin.trim();
      const m = Number(marginText);
      if (picked.billing_mode === "per_call" || picked.rate <= 0)
        problem = "套餐额度模式只支持倍率计费的模型，换一个模型或切到「Token 用量」。";
      else if (q == null) problem = "用户套餐额度必须是有效的正数。";
      else if (s == null) problem = "销售总价必须是有效的正数。";
      // A blank box is Number("") === 0, which passes the range check and would quietly price the
      // package at a 0% target margin. Blank means unanswered.
      else if (!marginText || !Number.isFinite(m) || m < 0 || m >= 100)
        problem = "目标利润率需在 0% 到 100% 之间。";
      body = {
        channel_rate_id: channel.id,
        connection_id: picked.connection_id,
        model_id: picked.model_id,
        visible_quota_usd: q,
        sales_cny: s,
        target_margin_percent: m,
      };
    } else {
      const c = Math.max(1, whole(calls));
      const tokens = {
        input_tokens: whole(inTok),
        output_tokens: whole(outTok),
        cache_read_tokens: whole(cacheRead),
        cache_creation_tokens: whole(cacheWrite),
      };
      const total =
        tokens.input_tokens + tokens.output_tokens + tokens.cache_read_tokens + tokens.cache_creation_tokens;
      body = {
        channel_rate_id: channel.id,
        connection_id: picked.connection_id,
        model_id: picked.model_id,
        calls: c,
        ...tokens,
      };
      if (c > 1_000_000) problem = "调用次数需在 1 到 1000000 之间。";
      else if (total <= 0) problem = "至少填写一种 Token 数量。";
      const text = tokenSales.trim();
      if (text) {
        const v = Number(text);
        if (!Number.isFinite(v) || v < 0) problem = "销售总价必须是有效的非负数。";
        else body.sales_cny = v;
      }
    }

    if (problem) {
      seq.current += 1;
      setCalcErr(problem);
      setQuota(null);
      setToken(null);
      setBusy(false);
      return;
    }

    const id = ++seq.current;
    setBusy(true);
    const timer = setTimeout(async () => {
      try {
        if (mode === "quota") {
          const r = await api.post<QuotaResult>("/api/admin/quota-estimate", body);
          if (seq.current !== id) return;
          setQuota(r);
        } else {
          const r = await api.post<TokenResult>("/api/admin/model-estimate", body);
          if (seq.current !== id) return;
          setToken(r);
        }
        setCalcErr("");
      } catch (e) {
        if (seq.current !== id) return;
        setCalcErr(msg(e, "推算失败"));
        setQuota(null);
        setToken(null);
      } finally {
        if (seq.current === id) setBusy(false);
      }
    }, 320);
    return () => clearTimeout(timer);
  }, [
    mode,
    channels,
    options,
    activeChannelId,
    activeOptionKey,
    quotaUsd,
    quotaSales,
    targetMargin,
    calls,
    inTok,
    outTok,
    cacheRead,
    cacheWrite,
    tokenSales,
  ]);

  async function createRate(e: React.FormEvent) {
    e.preventDefault();
    const name = crName.trim();
    const rate = Number(crRate);
    if (!name) {
      setFormErr("请填写渠道名称");
      return;
    }
    if (!Number.isFinite(rate) || rate <= 0) {
      setFormErr("渠道购买价必须是有效的正数");
      return;
    }
    setCrBusy(true);
    setFormErr("");
    let created: ChannelRate | null = null;
    try {
      created = await api.post<ChannelRate>("/api/admin/channel-rates", {
        name,
        usd_per_cny: rate,
        note: crNote.trim(),
      });
    } catch (err) {
      setFormErr(msg(err, "创建失败"));
      setCrBusy(false);
      return;
    }
    setCrName("");
    setCrRate("");
    setCrNote("");
    // The row exists from here on. A refresh that fails afterwards is a list problem, not a
    // create problem — saying 创建失败 here makes an operator create the channel a second time.
    try {
      await loadRates();
    } catch (err) {
      setLoadErr(msg(err, "渠道已创建，但列表刷新失败，请重新加载页面"));
    }
    if (created?.id) setChannelId(created.id);
    setCrBusy(false);
  }

  function openEdit(rate: ChannelRate) {
    setEditing(rate);
    setEditName(rate.name);
    setEditRate(String(rate.usd_per_cny));
    setEditNote(rate.note || "");
    setDialogErr("");
  }

  function openRemove(rate: ChannelRate) {
    setRemoving(rate);
    setDialogErr("");
  }

  async function saveEdit() {
    if (!editing) return;
    const name = editName.trim();
    const rate = Number(editRate);
    if (!name) {
      setDialogErr("请填写渠道名称");
      return;
    }
    if (!Number.isFinite(rate) || rate <= 0) {
      setDialogErr("渠道购买价必须是有效的正数");
      return;
    }
    setCrBusy(true);
    try {
      await api.post<ChannelRate>(`/api/admin/channel-rates/${editing.id}`, {
        name,
        usd_per_cny: rate,
        note: editNote.trim(),
      });
    } catch (err) {
      setDialogErr(msg(err, "保存失败"));
      setCrBusy(false);
      return;
    }
    setEditing(null);
    setDialogErr("");
    try {
      await loadRates();
    } catch (err) {
      setLoadErr(msg(err, "已保存，但列表刷新失败，请重新加载页面"));
    }
    setCrBusy(false);
  }

  async function confirmRemove() {
    if (!removing) return;
    setCrBusy(true);
    try {
      await api.del<{ ok?: boolean }>(`/api/admin/channel-rates/${removing.id}`);
    } catch (err) {
      setDialogErr(msg(err, "删除失败"));
      setCrBusy(false);
      return;
    }
    setRemoving(null);
    setDialogErr("");
    try {
      await loadRates();
    } catch (err) {
      setLoadErr(msg(err, "已删除，但列表刷新失败，请重新加载页面"));
    }
    setCrBusy(false);
  }

  const result = mode === "quota" ? quota : token;
  const profit = mode === "quota" ? quota?.profit_cny : token?.profit_cny;
  const margin = mode === "quota" ? quota?.margin_percent : token?.margin_percent;
  // Numbers on screen during a re-estimate belong to the previous parameters. Dim them rather
  // than letting a price get read off a stale tile.
  const fade = `transition-opacity duration-200 ${busy && result ? "opacity-60" : "opacity-100"}`;

  const verdict = (() => {
    if (!result) return null;
    if (mode === "quota" && quota) {
      if (quota.status === "loss")
        return {
          badge: <Badge variant="outline" className="border-destructive/40 text-destructive">亏损</Badge>,
          line: `当前 ${mul(quota.connection.rate, 4)} 会亏钱。保本至少要 ${mul(quota.break_even_multiplier_rounded)}，要做到 ${pct(quota.target_margin_percent)} 利润建议 ${mul(quota.target_multiplier_rounded)}。`,
        };
      if (quota.status === "below_target")
        return {
          badge: <Badge variant="outline">未达目标</Badge>,
          line: `当前倍率能赚钱，但没到 ${pct(quota.target_margin_percent)} 目标。把 ${mul(quota.connection.rate, 4)} 调到 ${mul(quota.target_multiplier_rounded)} 就够了。`,
        };
      // 达标 is claimed only when the server actually said healthy. A missing or unrecognised
      // status must not read as "不需要调整" on the screen a 倍率 gets set from.
      if (quota.status === "healthy")
        return {
          badge: <Badge variant="success">达标</Badge>,
          line: `当前 ${mul(quota.connection.rate, 4)} 已达到 ${pct(quota.target_margin_percent)} 目标，不需要调整。`,
        };
      return {
        badge: <Badge variant="outline">待确认</Badge>,
        line: `服务端没有给出这次试算的结论，先按下面的保本倍率 ${mul(quota.break_even_multiplier_rounded)} 和建议倍率 ${mul(quota.target_multiplier_rounded)} 判断。`,
      };
    }
    if (profit == null)
      return {
        badge: <Badge variant="outline">未定价</Badge>,
        line: `这段用量的渠道成本是 ${cny(token?.channel_cost_cny)}，填上销售总价就能看到利润。`,
      };
    if (profit < -1e-9)
      return {
        badge: <Badge variant="outline" className="border-destructive/40 text-destructive">亏损</Badge>,
        line: `这段用量会亏 ${cny(Math.abs(profit))}，保本价是 ${cny(token?.break_even_cny)}。`,
      };
    if (profit > 1e-9)
      return {
        badge: <Badge variant="success">盈利</Badge>,
        line: `这段用量赚 ${cny(profit)}，利润率 ${pct(margin)}，保本价 ${cny(token?.break_even_cny)}。`,
      };
    return { badge: <Badge variant="outline">保本</Badge>, line: "这段用量刚好打平。" };
  })();

  // One lab body, rendered inside whichever TabsContent is active. Two TabsContent elements
  // rather than one <TabsContent value={mode}>: with a single content node the inactive
  // trigger's aria-controls points at an id that is never in the DOM.
  const lab = (
    <>
      {mode === "quota" ? (
        <div className={`grid gap-4 sm:grid-cols-2 lg:grid-cols-4 ${fade}`} aria-busy={busy}>
          <Stat
            label="净利润"
            value={cny(quota?.profit_cny)}
            hint={quota ? `利润率 ${pct(quota.margin_percent)}` : "填好左边的参数"}
          />
          <Stat
            label="套餐用完的成本"
            value={cny(quota?.channel_cost_cny)}
            hint={quota ? `渠道消耗 ${usd(quota.provider_usd_capacity)}` : undefined}
          />
          <Stat
            label="建议倍率"
            value={mul(quota?.target_multiplier_rounded)}
            hint={
              quota
                ? `当前 ${mul(quota.connection.rate, 4)} · 保本 ${mul(quota.break_even_multiplier_rounded)}`
                : undefined
            }
          />
          <Stat
            label="目标售价"
            value={cny(quota?.target_sales_cny)}
            hint={quota ? `保本 ${cny(quota.break_even_sales_cny)}` : undefined}
          />
        </div>
      ) : (
        <div className={`grid gap-4 sm:grid-cols-2 lg:grid-cols-4 ${fade}`} aria-busy={busy}>
          <Stat
            label="渠道成本"
            value={cny(token?.channel_cost_cny)}
            hint={token ? usd(token.provider_usd_total, 8) : "填好左边的参数"}
          />
          <Stat
            label="净利润"
            value={token?.profit_cny == null ? "—" : cny(token.profit_cny)}
            hint={token?.margin_percent == null ? "填销售总价后计算" : `利润率 ${pct(token.margin_percent)}`}
          />
          <Stat
            label="钱包扣费"
            value={token ? cents(token.billed_cents_per_call * token.calls) : "—"}
            hint={token ? `${cents(token.billed_cents_per_call)} / 次` : undefined}
          />
          <Stat
            label="折合用户额度"
            value={usd(token?.visible_quota_usd, 4)}
            hint={token ? `${dec(token.quota_raw_usd_per_visible_usd, 2)} : 1` : undefined}
          />
        </div>
      )}

      {verdict && (
        <div
          className={`flex flex-wrap items-center gap-3 rounded-xl border border-border bg-card px-5 py-4 ${fade}`}
          aria-busy={busy}
        >
          {verdict.badge}
          <p className="min-w-0 text-sm">{verdict.line}</p>
        </div>
      )}

      <div className="grid gap-6 lg:grid-cols-5">
        <div className="lg:col-span-2">
          <Panel title="参数" aside={busy ? <span className="text-xs text-muted-foreground">计算中…</span> : undefined}>
            <div className="space-y-4 p-5">
              <div>
                <Label htmlFor="pr-channel">渠道</Label>
                <Select
                  id="pr-channel"
                  value={activeChannelId}
                  disabled={!channels.length}
                  onChange={(e) => setChannelId(e.target.value)}
                >
                  {!channels.length && (
                    <option value="">{ratesLoaded ? "还没有渠道购买价" : "渠道购买价没加载出来"}</option>
                  )}
                  {/* host 为空的是假想渠道，一条真实流量都没承载过 —— 标出来，
                      别让它看起来和真实中转一样。 */}
                  {channels.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name} · {buyRate(c.usd_per_cny)}
                      {(c.host || "").trim() ? "" : "（假想渠道，没有真实流量）"}
                    </option>
                  ))}
                </Select>
              </div>

              <div>
                <Label htmlFor="pr-model">模型</Label>
                <Select
                  id="pr-model"
                  value={activeOptionKey}
                  disabled={!options.length}
                  onChange={(e) => setOptionKey(e.target.value)}
                >
                  {!options.length && (
                    <option value="">{modelsLoaded ? "还没有开放的模型" : "模型连接没加载出来"}</option>
                  )}
                  {groups.map((g) => (
                    <optgroup key={g.label} label={g.label}>
                      {g.items.map((o) => (
                        <option key={o.key} value={o.key}>
                          {o.display} · {mul(o.rate, 4)}
                          {o.billing_mode === "per_call" ? " · 次数计费" : ""}
                        </option>
                      ))}
                    </optgroup>
                  ))}
                </Select>
                {!options.length && modelsLoaded && (
                  <p className="mt-1.5 text-xs text-muted-foreground">
                    先在「模型线路」里勾选要开放的模型。
                  </p>
                )}
              </div>

              {mode === "quota" ? (
                <div className="space-y-4">
                  <Field
                    id="pr-quota"
                    label="用户套餐额度（$）"
                    type="number"
                    min="0.01"
                    step="1"
                    value={quotaUsd}
                    onChange={(e) => setQuotaUsd(e.target.value)}
                    hint="套餐页上写给用户看的额度，不是真实扣费额度。"
                  />
                  <Field
                    id="pr-quota-sales"
                    label="销售总价（¥）"
                    type="number"
                    min="0.01"
                    step="0.01"
                    value={quotaSales}
                    onChange={(e) => setQuotaSales(e.target.value)}
                  />
                  <Field
                    id="pr-target"
                    label="目标利润率（%）"
                    type="number"
                    min="0"
                    max="99.99"
                    step="1"
                    value={targetMargin}
                    onChange={(e) => setTargetMargin(e.target.value)}
                  />
                  {!quotaReady && !!opt && (
                    <p className="text-xs text-muted-foreground">
                      {perCall
                        ? "这个模型按次计费，套餐额度模式算不了它。"
                        : "这个连接的倍率是 0，先去「模型线路」里设一个大于 0 的倍率。"}
                    </p>
                  )}
                </div>
              ) : (
                <div className="grid gap-4 sm:grid-cols-2">
                  <Field
                    id="pr-calls"
                    label="调用次数"
                    type="number"
                    min="1"
                    step="1"
                    value={calls}
                    onChange={(e) => setCalls(e.target.value)}
                  />
                  <Field
                    id="pr-sales"
                    label="销售总价（¥）"
                    type="number"
                    min="0"
                    step="0.01"
                    placeholder="可选"
                    value={tokenSales}
                    onChange={(e) => setTokenSales(e.target.value)}
                  />
                  <Field
                    id="pr-in"
                    label="输入 Token / 次"
                    type="number"
                    min="0"
                    step="100"
                    value={inTok}
                    onChange={(e) => setInTok(e.target.value)}
                  />
                  <Field
                    id="pr-out"
                    label="输出 Token / 次"
                    type="number"
                    min="0"
                    step="100"
                    value={outTok}
                    onChange={(e) => setOutTok(e.target.value)}
                  />
                  <Field
                    id="pr-cache-read"
                    label="缓存读取 Token / 次"
                    type="number"
                    min="0"
                    step="100"
                    value={cacheRead}
                    onChange={(e) => setCacheRead(e.target.value)}
                  />
                  <Field
                    id="pr-cache-write"
                    label="缓存写入 Token / 次"
                    type="number"
                    min="0"
                    step="100"
                    value={cacheWrite}
                    onChange={(e) => setCacheWrite(e.target.value)}
                  />
                </div>
              )}

              {calcErr && (
                <p role="alert" className="text-sm text-destructive">
                  {calcErr}
                </p>
              )}
            </div>
          </Panel>
        </div>

        <div className={`space-y-6 lg:col-span-3 ${fade}`} aria-busy={busy}>
          {mode === "quota" ? (
            <>
              <Panel
                title="推算过程"
                aside={
                  quota ? (
                    <span className="truncate text-xs text-muted-foreground">
                      {quota.connection.label} · {quota.model.name} · {quota.channel.name}
                    </span>
                  ) : undefined
                }
              >
                <div className="divide-y divide-border">
                  {quota ? (
                    <>
                      <Row
                        label="用户套餐额度"
                        value={usd(quota.visible_quota_usd)}
                        hint="用户看到的额度"
                      />
                      <Row
                        label="折算真实扣费额度"
                        value={usd(quota.quota_raw_usd)}
                        hint={`${dec(quota.quota_raw_usd_per_visible_usd, 2)} : 1`}
                      />
                      <Row
                        label="最多承载渠道消耗"
                        value={usd(quota.provider_usd_capacity, 4)}
                        hint={`÷ 当前倍率 ${mul(quota.connection.rate, 6)}`}
                      />
                      <Row
                        label="换成人民币成本"
                        value={cny(quota.channel_cost_cny)}
                        hint={`÷ 渠道购买价 ${buyRate(quota.channel.usd_per_cny)}`}
                      />
                      <Row label="销售收入" value={cny(quota.sales_cny)} />
                      <Row
                        label="净利润"
                        value={cny(quota.profit_cny)}
                        hint={`利润率 ${pct(quota.margin_percent)}`}
                      />
                    </>
                  ) : (
                    <EmptyState compact title="选好渠道和模型" hint="参数填齐之后结果会自动算出来。" />
                  )}
                </div>
              </Panel>

              <Panel title="定价建议">
                <div className="divide-y divide-border">
                  {quota ? (
                    <>
                      <Row
                        label="最低保本倍率"
                        value={mul(quota.break_even_multiplier_rounded)}
                        hint={`精确 ${mul(quota.break_even_multiplier, 6)}`}
                      />
                      <Row
                        label="目标利润建议倍率"
                        value={mul(quota.target_multiplier_rounded)}
                        hint={`目标 ${pct(quota.target_margin_percent)}`}
                      />
                      <Row label="当前倍率保本售价" value={cny(quota.break_even_sales_cny)} />
                      <Row label="目标利润售价" value={cny(quota.target_sales_cny)} />
                      <Row
                        label="当前售价的安全额度"
                        value={usd(quota.safe_visible_quota_usd)}
                        hint="额度开到这个数以内，才守得住目标利润"
                      />
                    </>
                  ) : (
                    <EmptyState compact title="还没有可用的建议" hint="先在左边选好渠道和模型。" />
                  )}
                </div>
              </Panel>
            </>
          ) : (
            <>
              <Panel
                title="价格来源"
                aside={
                  token ? (
                    <span className="truncate text-xs text-muted-foreground">
                      {token.connection.label} · {token.model.name} · {token.channel.name}
                    </span>
                  ) : undefined
                }
              >
                <div className="divide-y divide-border">
                  {token ? (
                    <>
                      <Row
                        label="输入 / 输出（$ / 1M）"
                        value={`${usd(token.prices_per_million.input, 6)} / ${usd(token.prices_per_million.output, 6)}`}
                        hint={
                          PRICE_SOURCE[token.prices_per_million.source || ""] ||
                          token.prices_per_million.source ||
                          "—"
                        }
                      />
                      <Row
                        label="缓存读 / 写（$ / 1M）"
                        value={`${usd(token.prices_per_million.cache_read, 6)} / ${usd(token.prices_per_million.cache_creation, 6)}`}
                      />
                      <Row
                        label="连接倍率"
                        value={mul(token.connection.rate, 6)}
                        hint={token.connection.billing_mode === "per_call" ? "按次计费" : "按 Token 计费"}
                      />
                    </>
                  ) : (
                    <EmptyState compact title="选好渠道和模型" hint="参数填齐之后结果会自动算出来。" />
                  )}
                </div>
              </Panel>

              <Panel title="推算过程">
                <div className="divide-y divide-border">
                  {token ? (
                    <>
                      <Row label="调用次数" value={`${num(token.calls)} 次`} />
                      <Row
                        label="每次 Token（入 / 出 / 读 / 写）"
                        value={`${num(token.tokens_per_call.input)} / ${num(token.tokens_per_call.output)} / ${num(token.tokens_per_call.cache_read)} / ${num(token.tokens_per_call.cache_creation)}`}
                      />
                      <Row
                        label="渠道美元成本"
                        value={usd(token.provider_usd_total, 8)}
                        hint={`${usd(token.provider_usd_per_call, 8)} / 次`}
                      />
                      <Row
                        label="人民币成本"
                        value={cny(token.channel_cost_cny)}
                        hint={buyRate(token.channel.usd_per_cny)}
                      />
                      <Row
                        label="服务端钱包扣费"
                        value={cents(token.billed_cents_per_call * token.calls)}
                        hint={`折合用户额度 ${usd(token.visible_quota_usd, 6)}`}
                      />
                      <Row
                        label="销售收入"
                        value={token.sales_cny == null ? "—" : cny(token.sales_cny)}
                        hint={`保本价 ${cny(token.break_even_cny)}`}
                      />
                      <Row
                        label="净利润"
                        value={token.profit_cny == null ? "—" : cny(token.profit_cny)}
                        hint={token.margin_percent == null ? undefined : `利润率 ${pct(token.margin_percent)}`}
                      />
                    </>
                  ) : (
                    <EmptyState compact title="选好渠道和模型" hint="参数填齐之后结果会自动算出来。" />
                  )}
                </div>
              </Panel>
            </>
          )}
        </div>
      </div>
    </>
  );

  return (
    <div className="space-y-6">
      <PageHeader
        title="定价试算"
        description="套餐给多少额度合适、卖这个价亏不亏。用的是真实用量和真实渠道构成，不是假设；算完不会改动任何模型或用户的扣费。"
      />

      <ErrorState message={loadErr} />

      {/* 入场错峰：标题 0，往下每段 +70ms（展示站 SectionReveal 的 Math.min(i,4)*70）。 */}
      <SectionReveal as="section" delay={70}>
      <Tabs value={mode} onValueChange={(v) => setMode(v as Mode)} className="gap-6">
        <TabsList>
          <TabsTrigger value="health">套餐体检</TabsTrigger>
          <TabsTrigger value="quota">单条试算</TabsTrigger>
          <TabsTrigger value="token">Token 用量</TabsTrigger>
        </TabsList>

        <TabsContent value="health" className="flex flex-col gap-6">
          <PlanHealthTab />
        </TabsContent>
        <TabsContent value="quota" className="flex flex-col gap-6">
          {lab}
        </TabsContent>
        <TabsContent value="token" className="flex flex-col gap-6">
          {lab}
        </TabsContent>
      </Tabs>
      </SectionReveal>

      <SectionReveal as="section" delay={140} className="rounded-xl border border-border bg-card">
        <header className="flex items-center justify-between gap-4 border-b border-border px-5 py-3">
          <h2 className="text-sm font-semibold">渠道购买价</h2>
          <span className="text-xs text-muted-foreground">只用于上面的试算</span>
        </header>

        <form onSubmit={createRate} className="grid gap-4 p-5 sm:grid-cols-[1fr_11rem_1fr_auto]">
          <div>
            <Label htmlFor="cr-name">渠道名称</Label>
            <Input
              id="cr-name"
              maxLength={80}
              placeholder="如 渠道 A"
              value={crName}
              onChange={(e) => setCrName(e.target.value)}
            />
          </div>
          <div>
            <Label htmlFor="cr-rate">1 CNY = ? USD</Label>
            <Input
              id="cr-rate"
              type="number"
              min="0"
              step="0.000001"
              value={crRate}
              onChange={(e) => setCrRate(e.target.value)}
            />
          </div>
          <div>
            <Label htmlFor="cr-note">备注</Label>
            <Input
              id="cr-note"
              maxLength={500}
              placeholder="可选"
              value={crNote}
              onChange={(e) => setCrNote(e.target.value)}
            />
          </div>
          <div className="flex items-end">
            <Button type="submit" disabled={crBusy || !crName.trim() || !crRate}>
              创建渠道
            </Button>
          </div>
          {formErr && (
            <p role="alert" className="text-sm text-destructive sm:col-span-4">
              {formErr}
            </p>
          )}
        </form>

        <div className="border-t border-border">
          {channels.length ? (
            <Table className="min-w-[52rem]">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[16rem]">渠道</TableHead>
                  <TableHead numeric className="w-56">渠道购买价</TableHead>
                  <TableHead className="w-[20rem]">备注</TableHead>
                  <TableHead className="w-28">更新</TableHead>
                  <TableHead className="w-32 text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {channels.map((c) => (
                  <TableRow key={c.id}>
                    <TableCell className="max-w-[16rem] font-medium">
                      <Truncate>{c.name}</Truncate>
                    </TableCell>
                    <TableCell numeric>{buyRate(c.usd_per_cny)}</TableCell>
                    <TableCell className="max-w-[20rem] text-muted-foreground">
                      <Truncate>{c.note || "—"}</Truncate>
                    </TableCell>
                    <TableCell className="whitespace-nowrap text-muted-foreground">
                      {when(c.updated_at || c.created_at)}
                    </TableCell>
                    <TableCell>
                      <div className="flex justify-end gap-2">
                        <Button size="sm" variant="outline" onClick={() => openEdit(c)}>
                          编辑
                        </Button>
                        <Button
                          size="sm"
                          variant="outline"
                          className="border-destructive/40 text-destructive hover:bg-destructive/10"
                          onClick={() => openRemove(c)}
                        >
                          删除
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          ) : ratesLoaded ? (
            <EmptyState
              title="还没有渠道购买价"
              hint="汇率表示 1 元人民币能换多少渠道原始美元，先建一个再试算。"
            />
          ) : loadErr ? (
            // 「还没有」只能对真的加载成功过的列表说。加载失败时说"还没有渠道购买价"，
            // 操作员会照着建一条重复的。
            <ErrorState
              variant="block"
              message="渠道购买价没有加载出来"
              hint="先解决上面的报错再操作，别在这里重复创建。"
            />
          ) : (
            <TableSkeleton rows={3} columns={["18%", "26%", "24%", "10%"]} label="渠道购买价读取中" />
          )}
        </div>
      </SectionReveal>

      <Dialog open={!!editing} onOpenChange={(open) => !open && setEditing(null)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>编辑渠道购买价</DialogTitle>
            <DialogDescription>改完之后，上面的试算会用新汇率重算一遍。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div>
              <Label htmlFor="cr-edit-name">渠道名称</Label>
              <Input
                id="cr-edit-name"
                maxLength={80}
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
              />
            </div>
            <div>
              <Label htmlFor="cr-edit-rate">1 CNY = ? USD</Label>
              <Input
                id="cr-edit-rate"
                type="number"
                min="0"
                step="0.000001"
                value={editRate}
                onChange={(e) => setEditRate(e.target.value)}
              />
            </div>
            <div>
              <Label htmlFor="cr-edit-note">备注</Label>
              <Input
                id="cr-edit-note"
                maxLength={500}
                value={editNote}
                onChange={(e) => setEditNote(e.target.value)}
              />
            </div>
            {dialogErr && (
              <p role="alert" className="text-sm text-destructive">
                {dialogErr}
              </p>
            )}
          </div>
          <div className="flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setEditing(null)} disabled={crBusy}>
              取消
            </Button>
            <Button onClick={saveEdit} disabled={crBusy}>
              保存
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={!!removing} onOpenChange={(open) => !open && setRemoving(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>删除渠道购买价</DialogTitle>
            <DialogDescription>
              删除「{removing?.name}」只影响试算，不会改动任何模型价格或用户扣费。
            </DialogDescription>
          </DialogHeader>
          {dialogErr && (
            <p role="alert" className="text-sm text-destructive">
              {dialogErr}
            </p>
          )}
          <div className="flex justify-end gap-3">
            <Button variant="ghost" onClick={() => setRemoving(null)} disabled={crBusy}>
              取消
            </Button>
            <Button
              variant="outline"
              className="border-destructive/40 text-destructive hover:bg-destructive/10"
              onClick={confirmRemove}
              disabled={crBusy}
            >
              删除
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
