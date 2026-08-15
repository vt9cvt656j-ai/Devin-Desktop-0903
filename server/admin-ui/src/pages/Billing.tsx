import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
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
import { Textarea } from "@/components/ui/textarea";
import { Panel } from "@/components/Panel";
import { api } from "@/lib/api";
import { creditCentsFromRaw, rawCentsFromCreditDollars, useSettings } from "@/lib/settings";
import { useRowFlash } from "@/lib/flash";
import { cents, num, when } from "@/lib/format";

/**
 * 收款 — everything that brings money in, on one screen: 订单 / 商品 / 兑换码.
 *
 * An operator lives on the 订单 tab: a customer pays by hand (there is no gateway callback yet),
 * the operator confirms the order and the server grants the plan or the credits in the same
 * transaction (pay.rs admin_confirm_order). 商品 is where the price list that the IDE sells from
 * is maintained; 兑换码 is the same grant sold offline as a code. Three old tabs' worth of
 * navigation for one job, so they are three tabs of one screen and the pending count is on the
 * stat row, on the tab label, and pinned to the top of the table.
 *
 * Deliberately dropped from the old console:
 *  - products 状态 (在售/下架) as a column: prices.active defaults true and no route can change
 *    it, so the column only ever printed 在售 — a control that isn't one. But admin_list_prices
 *    returns rows regardless of active while the buy path requires active = true (pay.rs:171),
 *    so a row deactivated out-of-band would sit in this table looking sellable. A 已下架 badge
 *    renders only in that case, and 在售商品 counts only active rows — normally identical.
 *  - orders 方式: create_order hardcodes method='manual' (pay.rs:177). One value, every row.
 *  - the third stat tile (已支付订单 as its own number): it is the hint under 已收款.
 *  - the codes 使用者 email lookup, which fetched the entire /api/admin/users table to resolve a
 *    uuid. The short id links back to 客户; the full email belongs on that screen.
 */

type Grant = {
  kind?: string;
  plan?: string | null;
  duration_days?: number | null;
  credits_cents?: number | null;
};
type Price = Grant & {
  id: string;
  label?: string;
  amount_cents?: number;
  /** prices.active (migrations/0004_orders.sql) — the buy path refuses anything false. */
  active?: boolean;
  created_at?: string;
};
type Order = Grant & {
  id: string;
  email?: string;
  /** 目录上的标价，人民币分。「Power」是 18800。 */
  amount_cents?: number;
  /**
   * Stripe 实际收到的钱和币种。手工发放的订单、以及 20260827 之前的订单都是 null。
   *
   * 显示金额必须优先用这个：amount_cents 是人民币标价，按美元渲染会把一笔 $34.99 的
   * 交易显示成 $188.00，整体营收虚高五六倍。
   */
  charged_cents?: number | null;
  charged_currency?: string | null;
  status?: string;
  created_at?: string;
};

/** 一笔订单真正收到的钱。拿不到就退回标价，并让调用方知道这是退回来的。 */
const realAmount = (o: Order) =>
  typeof o.charged_cents === "number" ? o.charged_cents : (o.amount_cents || 0);

/** Stripe 的金额是所收币种的最小单位；标价那一路仍按老样子当分处理。 */
const money = (o: Order) => {
  if (typeof o.charged_cents !== "number") return cents(o.amount_cents);
  const ccy = (o.charged_currency || "usd").toUpperCase();
  return (o.charged_cents / 100).toLocaleString("en-US", {
    style: "currency",
    currency: ccy,
  });
};
type Code = Grant & {
  id: string;
  code?: string;
  note?: string;
  status?: string;
  used_by?: string | null;
  /** Not sent today; used if the server ever joins it, so the screen improves without a rewrite. */
  used_by_email?: string | null;
  created_at?: string;
};
type Ask = { title: string; desc: string; label: string; danger?: boolean; act: () => Promise<void> };

/**
 * codes.rs PLANS —— 服务端只认这几个，别的会被拒。
 *
 * 名字跟着 2026Q3 的价目走：等级 key 是不能改的（用户身上存的就是它），但下拉框里只写 key
 * 的话，运营得自己记住 ultra 是「尊享」。`pro` 已经停售 —— 它没有对应商品，配额行留着只是
 * 为了还挂在这一档的老用户能查到。默认值曾经就是它，点一下「生成」就会发出一批
 * 没人买得到、名字也对不上的套餐码。
 */
const PLANS = [
  { key: "trial", label: "日卡" },
  { key: "basic", label: "入门" },
  { key: "power", label: "主力" },
  { key: "ultra", label: "尊享" },
  { key: "pro", label: "pro（已停售，仅供老用户补发）" },
] as const;

/**
 * User-facing credit balances are denominated at N real billing cents = $1.00 of visible
 * credit (default 663). Prices and revenue are real money and never take this conversion —
 * only credits_cents does. The formatting itself still goes through cents(); only the unit
 * conversion lives here.
 *
 * 分母来自服务端（lib/settings.ts → app_settings），不再在本文件写死。toCredits 是**写路径**：
 * 运营输入的美元乘以分母后作为 credits_cents 存库，服务端不做二次换算，所以这个数一旦
 * 和服务端不一致，商品和兑换码发出去的额度就会直接错掉。
 *
 * 注意：商品价格和兑换码里的额度在创建时就按当时的分母折算成真实分存下来了。之后修改
 * 分母**不会**追改已存在的商品、订单和未兑换的码——它们的真实分是冻结的，变的只是这些
 * 数字在页面上显示成多少面值美元。
 */
const creditUsd = (raw: number | null | undefined) =>
  raw == null ? cents(null) : cents(creditCentsFromRaw(raw));
const toCents = (v: string) => Math.round((parseFloat(v) || 0) * 100);
const toCredits = rawCentsFromCreditDollars;
const reason = (e: unknown, fallback: string) => (e instanceof Error ? e.message : fallback);


function Field({ id, label, children }: { id: string; label: string; children: ReactNode }) {
  return (
    <div>
      <Label htmlFor={id}>{label}</Label>
      {children}
    </div>
  );
}

/** What a plan / credits grant actually gives — shared by products, orders and codes. */
function content(g: Grant) {
  if (g.kind === "plan") {
    return (
      <span className="inline-flex items-center gap-2">
        <Badge variant="secondary">{g.plan || "—"}</Badge>
        <span className="text-muted-foreground tabular-nums">{num(g.duration_days)} 天</span>
      </span>
    );
  }
  return <span className="tabular-nums">额度 {creditUsd(g.credits_cents)}</span>;
}

function orderStatus(s?: string) {
  if (s === "paid") return <Badge variant="success">已支付</Badge>;
  if (s === "pending") return <Badge variant="outline">待确认</Badge>;
  if (s === "canceled") return <Badge variant="secondary">已取消</Badge>;
  return <Badge variant="secondary">{s || "—"}</Badge>;
}

export function Billing() {
  // 订阅面值分母：设置到货后金额要重算一次。
  useSettings();
  const [prices, setPrices] = useState<Price[]>([]);
  const [orders, setOrders] = useState<Order[]>([]);
  const [codes, setCodes] = useState<Code[]>([]);
  // 两种错误，寿命不同：loadErr 属于这一次轮询，下一次轮询就该被覆盖；err 属于操作员刚做的
  // 动作，必须一直留到他下一次动手为止 —— 否则 30 秒后的轮询会把"订单状态不是待支付"擦掉。
  const [loadErr, setLoadErr] = useState("");
  const [err, setErr] = useState("");
  const [ok, setOk] = useState("");
  const [busy, setBusy] = useState(false);
  // "还没读到"和"读到了，是空的"是两句话。分不开的话，第一次 render 会对着一张空表说
  // 「暂无订单」，操作员就以为今天没人下单。
  const [loaded, setLoaded] = useState(false);
  // 行级反馈：确认/取消之后，那一行自己亮一下（240ms，index.css 的 [data-flash]）。
  // 表格随后会重排（待确认永远置顶），亮的是这一行本身，不是它当时的位置。
  const { fire, toneOf } = useRowFlash();
  // busy 的 ref 影子：轮询的 setInterval 闭包读不到最新的 state。
  const busyRef = useRef(false);
  const mark = (v: boolean) => {
    busyRef.current = v;
    setBusy(v);
  };
  const [ask, setAsk] = useState<Ask | null>(null);

  const [orderFilter, setOrderFilter] = useState("");
  const [codeFilter, setCodeFilter] = useState("");

  // 商品表单
  const [pLabel, setPLabel] = useState("");
  const [pKind, setPKind] = useState("plan");
  const [pPlan, setPPlan] = useState("basic");
  const [pDays, setPDays] = useState("30");
  const [pCredit, setPCredit] = useState("10");
  const [pAmount, setPAmount] = useState("9.90");

  // 兑换码表单
  const [gKind, setGKind] = useState("plan");
  const [gPlan, setGPlan] = useState("basic");
  const [gDays, setGDays] = useState("30");
  const [gCredit, setGCredit] = useState("10");
  const [gCount, setGCount] = useState("10");
  const [gNote, setGNote] = useState("");
  const [generated, setGenerated] = useState<string[]>([]);

  const load = useCallback(async () => {
    const failed: string[] = [];
    // One endpoint being down should degrade one tab, not blank the screen — so each call
    // records its own failure and still resolves.
    const grab = async <T,>(path: string): Promise<T[]> => {
      try {
        const r = await api.get<T[] | { items?: T[] }>(path);
        return Array.isArray(r) ? r : r?.items || [];
      } catch (e) {
        failed.push(reason(e, "加载失败"));
        return [];
      }
    };
    const [p, o, c] = await Promise.all([
      grab<Price>("/api/admin/prices"),
      grab<Order>("/api/admin/orders"),
      grab<Code>("/api/admin/codes"),
    ]);
    return { prices: p, orders: o, codes: c, failed };
  }, []);

  const refresh = useCallback(async () => {
    const r = await load();
    setPrices(r.prices);
    setOrders(r.orders);
    setCodes(r.codes);
    setLoadErr(r.failed[0] || "");
    setLoaded(true);
  }, [load]);

  useEffect(() => {
    let alive = true;
    const tick = async () => {
      // 动作进行中不轮询：一次慢的轮询可能在 mutate 的 refresh() 之后才落地，
      // 把刚刚确认过的订单又画回"待确认"，操作员会以为没成功而再点一次。
      if (busyRef.current) return;
      const r = await load();
      if (!alive) return;
      setPrices(r.prices);
      setOrders(r.orders);
      setCodes(r.codes);
      setLoadErr(r.failed[0] || "");
      setLoaded(true);
    };
    tick();
    const t = setInterval(tick, 30_000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [load]);

  // 成功提示自己消失；错误留着，直到下一次动作。
  useEffect(() => {
    if (!ok) return;
    const t = setTimeout(() => setOk(""), 4_000);
    return () => clearTimeout(t);
  }, [ok]);

  /** Every mutation: one busy flag, one message line, one reload. */
  const mutate = async (done: string, fn: () => Promise<unknown>) => {
    mark(true);
    setErr("");
    setOk("");
    try {
      await fn();
      setOk(done);
      await refresh();
      return true;
    } catch (e) {
      setErr(reason(e, "操作失败"));
      // 失败最常见的原因是这一行已经在别处变了（订单已被确认、商品已被删除）。
      // 重新拉一次，否则表格会继续摆着一个刚刚失败的按钮等操作员再点一次。
      await refresh();
      return false;
    } finally {
      mark(false);
    }
  };

  const paid = orders.filter((o) => o.status === "paid");
  const pending = orders.filter((o) => o.status === "pending");
  // 只累加真收到的钱。混币种相加本来就不对,但在把标价当营收之后,这已经是更接近真相的
  // 版本;真要分币种统计,得先决定用哪天的汇率,那是另一件事。
  const revenue = paid.reduce((a, o) => a + realAmount(o), 0);
  const unused = codes.filter((c) => c.status === "unused");
  // 在售 = 客户真能买到的，也就是买入路径要求的 active = true（pay.rs:171）。
  const onSale = prices.filter((p) => p.active !== false);
  const shownErr = err || loadErr;

  // 待确认永远排在最前：它是这一屏唯一需要动手的东西。sort 是稳定的，组内仍是 created_at 倒序。
  const shownOrders = orders
    .filter((o) => !orderFilter || o.status === orderFilter)
    .slice()
    .sort((a, b) => (a.status === "pending" ? 0 : 1) - (b.status === "pending" ? 0 : 1));
  const shownCodes = codes.filter((c) => !codeFilter || c.status === codeFilter);

  const createPrice = () => {
    const label = pLabel.trim();
    if (!label) return setErr("请填写名称");
    if (toCents(pAmount) <= 0) return setErr("售价需大于 0");
    if (pKind === "plan" && (parseInt(pDays, 10) || 0) <= 0) return setErr("时长(天)需大于 0");
    if (pKind === "credits" && toCredits(pCredit) <= 0) return setErr("额度需大于 0");
    return mutate("已添加商品", async () => {
      await api.post<Price>("/api/admin/prices", {
        label,
        kind: pKind,
        amount_cents: toCents(pAmount),
        ...(pKind === "plan"
          ? { plan: pPlan, duration_days: parseInt(pDays, 10) || 0 }
          : { credits_cents: toCredits(pCredit) }),
      });
      setPLabel("");
    });
  };

  const generateCodes = () => {
    const count = parseInt(gCount, 10) || 0;
    if (count < 1 || count > 500) return setErr("数量需在 1 - 500 之间");
    if (gKind === "plan" && (parseInt(gDays, 10) || 0) <= 0) return setErr("时长(天)需大于 0");
    if (gKind === "credits" && toCredits(gCredit) <= 0) return setErr("额度需大于 0");
    return mutate(`已生成 ${count} 个兑换码`, async () => {
      const r = await api.post<{ codes?: string[]; count?: number }>("/api/admin/codes", {
        kind: gKind,
        count,
        note: gNote.trim(),
        ...(gKind === "plan"
          ? { plan: gPlan, duration_days: parseInt(gDays, 10) || 0 }
          : { credits_cents: toCredits(gCredit) }),
      });
      setGenerated(r?.codes || []);
    });
  };

  const copyGenerated = async () => {
    try {
      await navigator.clipboard.writeText(generated.join("\n"));
      setOk("已复制到剪贴板");
    } catch {
      setErr("浏览器拒绝了复制，请手动全选");
    }
  };

  const confirmOrder = (o: Order) =>
    setAsk({
      title: "确认收款",
      desc: `确认已收到 ${o.email || o.id} 的 ${money(o)}？确认后立即发放，且无法撤销。`,
      label: "确认收款并发放",
      act: async () => {
        const done = await mutate("已确认收款并发放", () =>
          api.post<{ ok?: boolean }>(`/api/admin/orders/${o.id}/confirm`),
        );
        fire(o.id, done ? "ok" : "error");
      },
    });

  const cancelOrder = (o: Order) =>
    setAsk({
      title: "取消订单",
      desc: `取消 ${o.email || o.id} 的这笔 ${money(o)} 订单？不会发放任何权益。`,
      label: "取消订单",
      danger: true,
      act: async () => {
        const done = await mutate("已取消订单", () =>
          api.post<{ ok?: boolean }>(`/api/admin/orders/${o.id}/cancel`),
        );
        fire(o.id, done ? "ok" : "error");
      },
    });

  const deletePrice = (p: Price) =>
    setAsk({
      title: "删除商品",
      desc: `删除「${p.label || p.id}」？已下单的订单不受影响，但用户将无法再购买它。`,
      label: "删除商品",
      danger: true,
      act: async () => {
        await mutate("已删除商品", () => api.del<{ ok?: boolean }>(`/api/admin/prices/${p.id}`));
      },
    });

  const deleteCode = (c: Code) =>
    setAsk({
      title: "删除兑换码",
      // 已兑换过的码，删除的后果完全不同：权益早已发出去、收不回来，删掉的只是
      // "谁在什么时候兑换了什么"这条记录。对已使用的码说"持有人将无法再兑换"是假的。
      desc:
        c.status === "used"
          ? `删除 ${c.code || c.id}？它已经被兑换，已发放的权益不会被收回；删除只会抹掉这条兑换记录。`
          : `删除 ${c.code || c.id}？如果它已经发出去，持有人将无法再兑换。`,
      label: "删除兑换码",
      danger: true,
      act: async () => {
        await mutate("已删除兑换码", () => api.del<{ ok?: boolean }>(`/api/admin/codes/${c.id}`));
      },
    });

  const dangerBtn = "text-destructive border-destructive/40 hover:bg-destructive/10";

  return (
    <div className="space-y-6">
      <PageHeader
        title="收款"
        description="商品、订单和兑换码。确认一笔订单会立刻给对应账号发放套餐或额度。"
      />

      <ErrorState message={shownErr} />
      {!shownErr && ok && (
        <p role="status" className="text-sm text-success">
          {ok}
        </p>
      )}

      {/* 入场错峰：标题 0，往下每段 +70ms（展示站 SectionReveal 的 Math.min(i,4)*70）。 */}
      <SectionReveal as="section" delay={70} className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat label="已收款" value={cents(revenue)} hint={`${paid.length} 笔已支付`} />
        <Stat
          label="待确认订单"
          value={num(pending.length)}
          hint={pending.length ? "需要处理" : "没有积压"}
        />
        <Stat
          label="在售商品"
          value={num(onSale.length)}
          hint={onSale.length === prices.length ? undefined : `共 ${prices.length} 个`}
        />
        <Stat label="未使用兑换码" value={num(unused.length)} hint={`共 ${codes.length} 个`} />
      </SectionReveal>

      <Tabs defaultValue="orders">
        <TabsList>
          <TabsTrigger value="orders">
            订单
            {pending.length > 0 && <span className="ml-1.5 tabular-nums">{pending.length}</span>}
          </TabsTrigger>
          <TabsTrigger value="products">商品</TabsTrigger>
          <TabsTrigger value="codes">兑换码</TabsTrigger>
        </TabsList>

        {/* ---------------- 订单 ---------------- */}
        <TabsContent value="orders">
          <Panel
            title={`订单 · ${orders.length}`}
            aside={
              <div className="w-36">
                <Select
                  aria-label="订单状态筛选"
                  className="h-9 text-sm"
                  value={orderFilter}
                  onChange={(e) => setOrderFilter(e.target.value)}
                >
                  <option value="">全部状态</option>
                  <option value="pending">待确认</option>
                  <option value="paid">已支付</option>
                  <option value="canceled">已取消</option>
                </Select>
              </div>
            }
          >
            {!loaded ? (
              <TableSkeleton
                rows={5}
                columns={["24%", "18%", "10%", "10%", "10%"]}
                label="订单读取中"
              />
            ) : shownOrders.length === 0 ? (
              <EmptyState
                title={orders.length ? "没有符合筛选的订单" : "暂无订单"}
                hint={
                  orders.length
                    ? "把状态筛选调回「全部状态」就能看到其余订单。"
                    : "客户在 IDE 里下单后，订单会出现在这里等你确认收款。"
                }
              />
            ) : (
              /* 六列写死宽度：买家是邮箱（可长到 80 字符），金额是右对齐等宽，操作列两个按钮不能换行。 */
              <Table className="min-w-[62rem]">
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[20rem]">买家</TableHead>
                    <TableHead className="w-56">内容</TableHead>
                    <TableHead numeric className="w-28">金额</TableHead>
                    <TableHead className="w-28">状态</TableHead>
                    <TableHead className="w-28">下单</TableHead>
                    <TableHead className="w-44 text-right">操作</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {shownOrders.map((o) => (
                    <TableRow
                      key={o.id}
                      data-flash={toneOf(o.id)}
                      className={o.status === "pending" ? "bg-muted/40" : undefined}
                    >
                      <TableCell className="max-w-[20rem]">
                        <Truncate className="font-medium">{o.email || o.id}</Truncate>
                      </TableCell>
                      <TableCell>{content(o)}</TableCell>
                      <TableCell numeric>{money(o)}</TableCell>
                      <TableCell>{orderStatus(o.status)}</TableCell>
                      <TableCell className="whitespace-nowrap text-muted-foreground">
                        {when(o.created_at)}
                      </TableCell>
                      <TableCell className="text-right">
                        {o.status === "pending" ? (
                          <div className="flex justify-end gap-2">
                            <Button size="sm" disabled={busy} onClick={() => confirmOrder(o)}>
                              确认收款
                            </Button>
                            <Button
                              size="sm"
                              variant="outline"
                              className={dangerBtn}
                              disabled={busy}
                              onClick={() => cancelOrder(o)}
                            >
                              取消
                            </Button>
                          </div>
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </Panel>
        </TabsContent>

        {/* ---------------- 商品 ---------------- */}
        <TabsContent value="products" className="space-y-6">
          <Panel title="添加商品">
            <form
              className="grid gap-4 p-5 sm:grid-cols-2 lg:grid-cols-4"
              onSubmit={(e) => {
                e.preventDefault();
                createPrice();
              }}
            >
              <Field id="pr-label" label="名称">
                <Input
                  id="pr-label"
                  value={pLabel}
                  onChange={(e) => setPLabel(e.target.value)}
                  placeholder="如 Pro 月卡"
                />
              </Field>
              <Field id="pr-kind" label="类型">
                <Select id="pr-kind" value={pKind} onChange={(e) => setPKind(e.target.value)}>
                  <option value="plan">会员套餐</option>
                  <option value="credits">额度</option>
                </Select>
              </Field>
              {pKind === "plan" ? (
                <>
                  <Field id="pr-plan" label="套餐">
                    <Select id="pr-plan" value={pPlan} onChange={(e) => setPPlan(e.target.value)}>
                      {PLANS.map((p) => (
                        <option key={p.key} value={p.key}>
                          {p.label}
                        </option>
                      ))}
                    </Select>
                  </Field>
                  <Field id="pr-days" label="时长（天）">
                    <Input
                      id="pr-days"
                      type="number"
                      min={1}
                      value={pDays}
                      onChange={(e) => setPDays(e.target.value)}
                    />
                  </Field>
                </>
              ) : (
                <Field id="pr-credit" label="赠送额度（$）">
                  <Input
                    id="pr-credit"
                    type="number"
                    min={0.01}
                    step={0.01}
                    value={pCredit}
                    onChange={(e) => setPCredit(e.target.value)}
                  />
                </Field>
              )}
              <Field id="pr-amount" label="售价（$）">
                <Input
                  id="pr-amount"
                  type="number"
                  min={0.01}
                  step={0.01}
                  value={pAmount}
                  onChange={(e) => setPAmount(e.target.value)}
                />
              </Field>
              <div className="flex items-end sm:col-span-2 lg:col-span-4">
                <Button type="submit" disabled={busy}>
                  添加商品
                </Button>
              </div>
            </form>
          </Panel>

          <Panel title={`商品 · ${prices.length}`}>
            {!loaded ? (
              <TableSkeleton rows={3} columns={["26%", "10%", "20%", "10%"]} label="商品读取中" />
            ) : prices.length === 0 ? (
              <EmptyState title="还没有商品" hint="在上面那张表单里添加一个，IDE 的购买页就会出现它。" />
            ) : (
              <Table className="min-w-[52rem]">
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[18rem]">名称</TableHead>
                    <TableHead className="w-24">类型</TableHead>
                    <TableHead className="w-56">内容</TableHead>
                    <TableHead numeric className="w-28">售价</TableHead>
                    <TableHead className="w-28 text-right">操作</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {prices.map((p) => (
                    <TableRow key={p.id}>
                      <TableCell className="max-w-[18rem] font-medium">
                        <span className="flex items-center gap-2">
                          <Truncate>{p.label || "—"}</Truncate>
                          {p.active === false && (
                            <Badge variant="secondary" className="shrink-0">已下架</Badge>
                          )}
                        </span>
                      </TableCell>
                      <TableCell className="text-muted-foreground">
                        {p.kind === "plan" ? "套餐" : "额度"}
                      </TableCell>
                      <TableCell>{content(p)}</TableCell>
                      <TableCell numeric>{cents(p.amount_cents)}</TableCell>
                      <TableCell className="text-right">
                        <Button
                          size="sm"
                          variant="outline"
                          className={dangerBtn}
                          disabled={busy}
                          onClick={() => deletePrice(p)}
                        >
                          删除
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </Panel>
        </TabsContent>

        {/* ---------------- 兑换码 ---------------- */}
        <TabsContent value="codes" className="space-y-6">
          <Panel title="生成兑换码">
            <div className="p-5">
              <form
                className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4"
                onSubmit={(e) => {
                  e.preventDefault();
                  generateCodes();
                }}
              >
                <Field id="gen-kind" label="类型">
                  <Select id="gen-kind" value={gKind} onChange={(e) => setGKind(e.target.value)}>
                    <option value="plan">会员套餐</option>
                    <option value="credits">额度</option>
                  </Select>
                </Field>
                {gKind === "plan" ? (
                  <>
                    <Field id="gen-plan" label="套餐">
                      <Select id="gen-plan" value={gPlan} onChange={(e) => setGPlan(e.target.value)}>
                        {PLANS.map((p) => (
                          <option key={p.key} value={p.key}>
                            {p.label}
                          </option>
                        ))}
                      </Select>
                    </Field>
                    <Field id="gen-days" label="时长（天）">
                      <Input
                        id="gen-days"
                        type="number"
                        min={1}
                        value={gDays}
                        onChange={(e) => setGDays(e.target.value)}
                      />
                    </Field>
                  </>
                ) : (
                  <Field id="gen-credit" label="额度（$）">
                    <Input
                      id="gen-credit"
                      type="number"
                      min={0.01}
                      step={0.01}
                      value={gCredit}
                      onChange={(e) => setGCredit(e.target.value)}
                    />
                  </Field>
                )}
                <Field id="gen-count" label="数量（1 - 500）">
                  <Input
                    id="gen-count"
                    type="number"
                    min={1}
                    max={500}
                    value={gCount}
                    onChange={(e) => setGCount(e.target.value)}
                  />
                </Field>
                <Field id="gen-note" label="备注（可选）">
                  <Input
                    id="gen-note"
                    value={gNote}
                    onChange={(e) => setGNote(e.target.value)}
                    placeholder="如 双十一活动"
                  />
                </Field>
                <div className="flex items-end sm:col-span-2 lg:col-span-4">
                  <Button type="submit" disabled={busy}>
                    生成
                  </Button>
                </div>
              </form>

              {generated.length > 0 && (
                <div className="mt-5 border-t border-border pt-5">
                  <Label htmlFor="gen-result">刚生成的 {generated.length} 个（只显示这一次）</Label>
                  <Textarea
                    id="gen-result"
                    readOnly
                    className="font-mono text-sm"
                    rows={6}
                    value={generated.join("\n")}
                  />
                  <div className="mt-3 flex gap-3">
                    <Button size="sm" variant="secondary" onClick={copyGenerated}>
                      复制全部
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => setGenerated([])}>
                      收起
                    </Button>
                  </div>
                </div>
              )}
            </div>
          </Panel>

          <Panel
            title={`兑换码 · ${codes.length}，未使用 ${unused.length}`}
            aside={
              <div className="w-36">
                <Select
                  aria-label="兑换码状态筛选"
                  className="h-9 text-sm"
                  value={codeFilter}
                  onChange={(e) => setCodeFilter(e.target.value)}
                >
                  <option value="">全部状态</option>
                  <option value="unused">未使用</option>
                  <option value="used">已使用</option>
                </Select>
              </div>
            }
          >
            {!loaded ? (
              <TableSkeleton
                rows={5}
                columns={["16%", "18%", "8%", "14%", "14%", "8%"]}
                label="兑换码读取中"
              />
            ) : shownCodes.length === 0 ? (
              <EmptyState
                title={codes.length ? "没有符合筛选的兑换码" : "暂无兑换码"}
                hint={
                  codes.length
                    ? "把状态筛选调回「全部状态」就能看到其余的码。"
                    : "在上面生成一批，生成之后只显示这一次。"
                }
              />
            ) : (
              <Table className="min-w-[68rem]">
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[13rem]">兑换码</TableHead>
                    <TableHead className="w-52">内容</TableHead>
                    <TableHead className="w-24">状态</TableHead>
                    <TableHead className="w-40">使用者</TableHead>
                    <TableHead className="w-[14rem]">备注</TableHead>
                    <TableHead className="w-24">创建</TableHead>
                    <TableHead className="w-24 text-right">操作</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {shownCodes.map((c) => (
                    <TableRow key={c.id}>
                      <TableCell className="max-w-[13rem] font-mono">
                        <Truncate>{c.code || "—"}</Truncate>
                      </TableCell>
                      <TableCell>{content(c)}</TableCell>
                      <TableCell>
                        {c.status === "unused" ? (
                          <Badge variant="outline">未使用</Badge>
                        ) : (
                          <Badge variant="secondary">已使用</Badge>
                        )}
                      </TableCell>
                      <TableCell className="max-w-[10rem] text-muted-foreground">
                        <Truncate>
                          {c.used_by_email || (c.used_by ? `${String(c.used_by).slice(0, 8)}…` : "—")}
                        </Truncate>
                      </TableCell>
                      <TableCell className="max-w-[14rem] text-muted-foreground">
                        <Truncate>{c.note || "—"}</Truncate>
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-muted-foreground">
                        {when(c.created_at)}
                      </TableCell>
                      <TableCell className="text-right">
                        <Button
                          size="sm"
                          variant="outline"
                          className={dangerBtn}
                          disabled={busy}
                          onClick={() => deleteCode(c)}
                        >
                          删除
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </Panel>
        </TabsContent>
      </Tabs>

      {/* 一个对话框负责所有不可撤销的动作：发放、取消、删除。 */}
      <Dialog open={!!ask} onOpenChange={(open) => !open && setAsk(null)}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{ask?.title}</DialogTitle>
            <DialogDescription>{ask?.desc}</DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-3">
            <Button variant="outline" onClick={() => setAsk(null)}>
              返回
            </Button>
            <Button
              variant={ask?.danger ? "outline" : "default"}
              className={ask?.danger ? dangerBtn : undefined}
              disabled={busy}
              onClick={() => {
                const a = ask;
                setAsk(null);
                a?.act();
              }}
            >
              {ask?.label}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
