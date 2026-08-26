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
import { Pager, paginate } from "@/components/Pager";
import { Textarea } from "@/components/ui/textarea";
import { Panel } from "@/components/Panel";
import { api } from "@/lib/api";
import { creditCentsFromRaw, rawCentsFromCreditDollars, useSettings } from "@/lib/settings";
import { useRowFlash } from "@/lib/flash";
import { cents, num, when } from "@/lib/format";
import { formatMoney, formatTotals, realCharge, sumByCurrency } from "@/lib/money";

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
  /** 人民币标价，分。 */
  amount_cents?: number;
  /** 美元标价，美分。控制台建的商品填不了它，所以经常是空 —— 见下面表格里的「未设」。 */
  amount_usd_cents?: number | null;
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
  /** 退过款的时间。库里**没有退了多少钱**，所以金额无法从营收里减掉，只能把笔数说出来。 */
  refunded_at?: string | null;
  /** 下单时打算按哪个币报价。**不代表真按这个币收到了钱** —— 那只看 charged_currency。 */
  resolved_currency?: string | null;
  quantity?: number | null;
  price_id?: string | null;
  status?: string;
  created_at?: string;
};

/**
 * 表格里那一格。
 *
 * **有实收就写实收**（charged_cents + charged_currency，Stripe 扣款成功后 webhook 写回来的事实）。
 * 判据是 `typeof === "number"` 而不是「非 0」：整单优惠券会产生一笔真实的 0 元，
 * 「收了 0 元」和「没记录收了多少」是两件事，用非 0 判会把前者错当成后者。
 *
 * 没实收就只能写标价，而标价**只有人民币这一个数**（amount_cents = 目录人民币价 × 份数，
 * stripe.rs 那行 INSERT 对美元买家也一样绑人民币价）。
 *
 * # 为什么不去把美元报价还原出来
 *
 * 试过：按 price_id 去 join `prices.amount_usd_cents`。不行，两个独立原因：
 *  1. 价目是**原地 UPDATE 同一行**改的（迁移里写死了这条纪律）。实测漂移：20260834 把
 *     daily_trial 设成 710，20260835 又把同一行改成 700 —— 这中间下的单，今天 join 出来
 *     已经不是当时那个数了。
 *  2. 就算没调过价，买家看到的也不是这一列：卡片上的价优先取 **Stripe 实时价**，本地两列
 *     只是 Stripe 查不到时的兜底。stripe.rs 里点名过同一款：目录写 2799，Stripe 实收 3499。
 *
 * 所以这里只说得出的事实：人民币标价是多少、以及当时**打算**按什么币报价（右边那行小字）。
 * 不换算、不 join、不编。
 */
const money = (o: Order) => {
  const real = realCharge(o);
  if (real) return formatMoney(real.minor, real.ccy);
  return `${formatMoney(o.amount_cents || 0, "cny")} 标价`;
};

/** 报价币种和人民币标价不是一回事时，单独标出来，绝不拿它当上面那个数的单位。 */
const quoteCcy = (o: Order) => {
  const c = (o.resolved_currency || "").toLowerCase();
  return c && c !== "cny" ? c.toUpperCase() : "";
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
  // 「待确认」是人工确认收款时代的说法 —— 那时这一行在等运营点一下。现在付没付由
  // Stripe 说了算，pending 的意思就是钱没到，没有人要去确认它。
  if (s === "pending") return <Badge variant="outline">未支付</Badge>;
  if (s === "canceled") return <Badge variant="secondary">已取消</Badge>;
  return <Badge variant="secondary">{s || "—"}</Badge>;
}

export function Billing() {
  // 订阅面值分母：设置到货后金额要重算一次。
  useSettings();
  const [prices, setPrices] = useState<Price[]>([]);
  const [orders, setOrders] = useState<Order[]>([]);
  const [orderPage, setOrderPage] = useState(1);
  const [codePage, setCodePage] = useState(1);
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

  // 换了筛选就回第一页：留在第 4 页上是「明明筛出来了却是空的」最常见的来源。
  useEffect(() => { setOrderPage(1); }, [orderFilter]);
  useEffect(() => { setCodePage(1); }, [codeFilter]);

  const paid = orders.filter((o) => o.status === "paid");
  // 按币种分桶。跨币种相加没有意义 —— 要加就得先定用哪天的汇率，那是另一件事，
  // 而在没定之前，把 ¥416 和 $12 加成 428 比不显示更糟。
  const receivedText = formatTotals(sumByCurrency(paid));
  // 老的手工发放单没有收款记录（charged_* 是 null）。它们确实发出去了权益，
  // 但没有一笔可以对账的进账，所以不进上面那个数，只在下面点一句。
  const paidWithoutReceipt = paid.filter((o) => !realCharge(o)).length;
  // 退款：库里只有「退过款」这个时间戳，没有「退了多少」。所以这些钱**仍然计在上面**，
  // 只把笔数说出来 —— 减一个猜出来的数，比标注一个已知的缺口更糟。
  const refunded = paid.filter((o) => !!o.refunded_at).length;
  const pending = orders.filter((o) => o.status === "pending");
  const unused = codes.filter((c) => c.status === "unused");
  // 在售 = 客户真能买到的，也就是买入路径要求的 active = true（pay.rs:171）。
  const onSale = prices.filter((p) => p.active !== false);
  const shownErr = err || loadErr;

  // 按时间倒序（服务端就是这么给的），不再把未支付的挑到最前 —— 那是人工确认收款时代
  // 的排法，因为那时它是这一屏唯一需要动手的东西。现在没有任何一行需要动手。
  const shownOrders = orders.filter((o) => !orderFilter || o.status === orderFilter);
  const shownCodes = codes.filter((c) => !codeFilter || c.status === codeFilter);
  // 翻页在**筛选之后**做：先筛后分页，页数才对得上筛出来的条数。
  const orderView = paginate(shownOrders, orderPage);
  const codeView = paginate(shownCodes, codePage);

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
        description="商品、订单和兑换码。付款状态由 Stripe 决定，这里只如实显示，没有人工确认这一步。"
      />

      <ErrorState message={shownErr} />
      {!shownErr && ok && (
        <p role="status" className="text-sm text-success">
          {ok}
        </p>
      )}

      {/* 入场错峰：标题 0，往下每段 +70ms（展示站 SectionReveal 的 Math.min(i,4)*70）。 */}
      <SectionReveal as="section" delay={70} className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat
          label="已收款"
          value={receivedText}
          hint={[
            paidWithoutReceipt
              ? `${paid.length - paidWithoutReceipt} 笔有收款记录，另 ${paidWithoutReceipt} 笔手工发放（无进账）`
              : `${paid.length} 笔已支付`,
            refunded ? `含 ${refunded} 笔已退款，金额未扣除` : "",
          ]
            .filter(Boolean)
            .join(" · ")}
        />
        <Stat
          label="未支付订单"
          value={num(pending.length)}
          hint={pending.length ? "钱没到，Stripe 结清后会自动转已支付" : "没有挂着的单"}
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
                    : "客户付款后，订单会自动出现在这里，不需要人工确认。"
                }
              />
            ) : (
              /* 六列写死宽度：买家是邮箱（可长到 80 字符），金额是右对齐等宽，操作列那个按钮不能换行。 */
              <Table className="min-w-[62rem]">
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[20rem]">买家</TableHead>
                    <TableHead className="w-56">内容</TableHead>
                    <TableHead numeric className="w-28">金额</TableHead>
                    <TableHead className="w-28">状态</TableHead>
                    <TableHead className="w-28">下单</TableHead>
                    <TableHead className="w-28 text-right">操作</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {orderView.slice.map((o) => (
                    <TableRow
                      key={o.id}
                      data-flash={toneOf(o.id)}
                      className={o.status === "pending" ? "bg-muted/40" : undefined}
                    >
                      <TableCell className="max-w-[20rem]">
                        <Truncate className="font-medium">{o.email || o.id}</Truncate>
                      </TableCell>
                      <TableCell>{content(o)}</TableCell>
                      <TableCell numeric>
                        {money(o)}
                        {!realCharge(o) && quoteCcy(o) && (
                          <div className="text-xs text-muted-foreground">报价币种 {quoteCcy(o)}</div>
                        )}
                      </TableCell>
                      <TableCell>
                        {orderStatus(o.status)}
                        {o.refunded_at && (
                          <div className="mt-1 text-xs text-muted-foreground">已退款</div>
                        )}
                      </TableCell>
                      <TableCell className="whitespace-nowrap text-muted-foreground">
                        {when(o.created_at)}
                      </TableCell>
                      <TableCell className="text-right">
                        {o.status === "pending" ? (
                          // 只剩「取消」：它把一笔没付的单关掉，不会凭空造出收入。
                          // 「确认收款」已经删了 —— 付没付由 Stripe 说了算。
                          <Button
                            size="sm"
                            variant="outline"
                            className={dangerBtn}
                            disabled={busy}
                            onClick={() => cancelOrder(o)}
                          >
                            取消
                          </Button>
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
            <Pager
              page={orderView.current}
              pages={orderView.pages}
              total={shownOrders.length}
              unit="笔"
              onPage={setOrderPage}
            />
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
              <Field id="pr-amount" label="售价（¥ 人民币）">
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
                    <TableHead numeric className="w-32">售价</TableHead>
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
                      <TableCell numeric>
                        {formatMoney(p.amount_cents || 0, "cny")}
                        <div
                          className="text-xs text-muted-foreground"
                          title="目录里存的美元价。结账卡片上优先显示 Stripe 的实时价，两者可能不一致（实测有一款目录写 27.99、Stripe 实收 34.99）。"
                        >
                          {typeof p.amount_usd_cents === "number"
                            ? `${formatMoney(p.amount_usd_cents, "usd")} 目录价`
                            : "美元价未设"}
                        </div>
                      </TableCell>
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
                  {codeView.slice.map((c) => (
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
            <Pager
              page={codeView.current}
              pages={codeView.pages}
              total={shownCodes.length}
              unit="个"
              onPage={setCodePage}
            />
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
