import { useCallback, useEffect, useState } from "react";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Donut } from "@/components/viz/Donut";
import { Meter } from "@/components/viz/Meter";
import { TrendArea } from "@/components/viz/TrendArea";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { num, when } from "@/lib/format";
import { formatMoney, formatTotals, sumByCurrency } from "@/lib/money";
import { planKeys, useSettings } from "@/lib/settings";

/**
 * Leads with money. The old dashboard's three tiles were 总用户 / 今日新增 / 当前在线 — and
 * 当前在线 counted open ADMIN CONSOLE sockets (realtime.rs only calls touch_presence after
 * ws_authenticate returns an admin uid), so on a one-operator product it was permanently 1.
 * Revenue lived two tabs away on 收款, spend lived on 模型系统.
 *
 * 这一轮修掉的两件事：
 *
 *  1. 实时动态原本读的是 ev.detail —— events 表里没有这个列。realtime.rs 的 Event 是
 *     { id, user_id, kind, data, created_at }，真正的负载在 data 里（order_paid 带着
 *     email / amount_cents / by）。所以那一栏一直在走 `ev.detail || ev.kind` 的兜底分支，
 *     十行"login"、"user_updated"，谁做的、对谁做的，一个字都没有。这是登录那个
 *     {account} / {email} 的同一类错误：字段名照着界面猜，没有照着结构体读。
 *  2. 加载时四张牌直接渲染 $0.00 / 0 / 0。那不是"还没加载完"，那是"今天没收到钱"——
 *     两句话在屏幕上长得一样，但只有一句是真的。现在没读到就是"—"。
 *
 * GET /api/admin/orders 和 /api/admin/events 都是裸数组（pay.rs admin_list_orders →
 * Json<Vec<Order>>，realtime.rs recent_events → Json<Vec<Event>>），不是 { items } 信封。
 */

type User = {
  plan?: string;
  plan_expires_at?: string | null;
  created_at?: string;
  quota_window_cents?: number;
  quota_window_cap_cents?: number;
};

type Stats = {
  total_users?: number; today_users?: number; online?: number;
  // 下面三项是服务端的**全量聚合**（realtime.rs 的 stats）。加它们的原因：这一屏原来把
  // 「已收款 / 已付订单数 / 套餐构成」从两条带硬上限的列表里算（users LIMIT 500、
  // orders LIMIT 1000），用户过 500、订单过 1000 之后就静默变成"最近 N 条里的合计"，
  // 而紧挨着的「总用户」是真 count(*) —— 同一屏一个真一个截断，运营看不出来。
  paid_orders?: number;
  revenue_cents?: Record<string, number>;
  plan_mix?: Record<string, number>;
  // 近 24 小时「有 token、没收钱、也没扣免费点」的调用：上游的钱我们照付，两头都没进账。
  // 原因永远是这个模型在这条线路上三样价都没配（每模型价 / 官方目录 / 连接级），
  // compute_cost 只好返回 0。服务端已排除 mode:"free" 的模型 —— 那些是有意免费的。
  zero_priced_24h?: {
    calls: number;
    tokens: number;
    models: { model: string; calls: number; tokens: number }[];
  };
  // 免费额度池收了多少点，而那些调用按实时目录价**实际值多少钱**。
  // 池子扣的是售价，所以同一份「每日免费额度」在不同模型上不是同一个东西。
  free_pool_24h?: {
    milli_points: number;
    ref_micro_usd: number;
    /// 服务端按后台那个全局汇率折好的人民币分。别在前端写死汇率——它是可配置的。
    ref_cny_cents: number;
    unpriced_calls: number;
    models: {
      model: string; calls: number; milli_points: number;
      ref_micro_usd: number; should_milli_points: number; unpriced_calls: number;
    }[];
  };
};

/** pay.rs Order（只取这一屏用得上的字段）。email / amount_cents / status 在库里都是非空。 */
type Order = {
  id: string;
  email?: string;
  /** 目录标价，人民币分。「Power」是 18800。 */
  amount_cents?: number;
  /** Stripe 实收金额。手工发放和 20260827 之前的订单为 null。 */
  charged_cents?: number | null;
  status?: string;
  created_at?: string;
};

/** realtime.rs Event —— id 是 i64，data 是 serde_json::Value，没有 detail 这个字段。 */
type Event = {
  id?: number;
  kind?: string;
  data?: Record<string, unknown> | null;
  created_at?: string;
};

/** record_event 全部调用点的 kind（auth / pay / codes / update / email / commission）。 */
const EVENT_LABEL: Record<string, string> = {
  register: "注册",
  login: "登录",
  redeem: "兑换码",
  order_created: "下单",
  // 「确认收款」是人工确认时代的说法。这个事件现在只由 Stripe 那条线写（webhook / 对账器），
  // 没有人去确认它，所以它的意思就是「钱到了」。
  order_paid: "付款成功",
  user_updated: "客户变更",
  role_change: "角色变更",
  notify: "群发通知",
  commission_created: "佣金",
  commission_status: "佣金状态",
  ide_release_dispatched: "触发构建",
  ide_release_published: "版本发布",
  ide_release_cancelled: "取消构建",
};

const str = (data: Event["data"], key: string) => {
  const v = data?.[key];
  return typeof v === "string" && v ? v : "";
};

/** 一行动态说清楚"谁"。data 的形状按 kind 变，所以按优先级取第一个说得通的。 */
function subject(ev: Event) {
  const d = ev.data;
  const who = str(d, "email") || str(d, "by") || str(d, "tag");
  // 事件里带的 amount_cents 是目录的人民币标价（pay.rs / stripe.rs 记事件时绑的就是它）。
  const amount = typeof d?.amount_cents === "number" ? formatMoney(d.amount_cents, "cny") : "";
  const action = str(d, "action");
  return [who, amount, action].filter(Boolean).join(" · ");
}

export function Overview() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [orders, setOrders] = useState<Order[] | null>(null);
  const [events, setEvents] = useState<Event[] | null>(null);
  const [users, setUsers] = useState<User[] | null>(null);
  const [err, setErr] = useState("");
  // 必须订阅：planKeys() 读的是快照，不订阅的话设置到货后套餐圆环不会重画。
  useSettings();

  const load = useCallback(async (signal?: { alive: boolean }) => {
    try {
      const [s, o, e, u] = await Promise.all([
        api.get<Stats>("/api/admin/stats"),
        api.get<Order[]>("/api/admin/orders"),
        api.get<Event[]>("/api/admin/events"),
        // Plan mix, the signup trend and quota pressure all come from this one list, so the
        // dashboard stops being three counters and starts answering "what shape is the business".
        api.get<User[]>("/api/admin/users").catch(() => [] as User[]),
      ]);
      if (signal && !signal.alive) return;
      setStats(s || {});
      setOrders(Array.isArray(o) ? o : []);
      setEvents(Array.isArray(e) ? e : []);
      setUsers(Array.isArray(u) ? u : []);
      setErr("");
    } catch (e) {
      if (signal && !signal.alive) return;
      setErr(e instanceof Error ? e.message : "加载失败");
    }
  }, []);

  useEffect(() => {
    const signal = { alive: true };
    load(signal);
    const t = setInterval(() => load(signal), 30_000);
    return () => {
      signal.alive = false;
      clearInterval(t);
    };
  }, [load]);

  // null = 还没读到（不是"没有"）。这一屏所有"—"和空状态都从这里分叉。
  const loadedOrders = orders ?? [];
  const paid = loadedOrders.filter((o) => o.status === "paid");
  // status is three-valued (pending | paid | canceled — migrations/0004_orders.sql:28), so
  // "not paid" counts every CANCELLED order as needing action. The tile then shows a backlog
  // that never clears and disagrees with Billing, which filters correctly.
  const pending = loadedOrders.filter((o) => o.status === "pending");
  // 口径在 lib/money.ts，和收款页共用一份。以前这里是自己写的一份，只做到「优先用
  // charged_cents」，拿不到就退回 amount_cents（人民币标价）再按美元渲染 —— 于是没进账的
  // 手工单被算成钱，人民币被报成美元。两份抄在一起的东西只要有一处先改，另一处就会
  // 带着一句「Billing.tsx 已经改过」的注释继续错下去，这里原来就是那样。
  // 优先用服务端全量聚合；拿不到（老网关还没这个字段）才退回按列表合计，
  // 那时它只是"最近 1000 笔里的合计"，所以下面的 hint 会说清是哪一种。
  const revenueFull = stats?.revenue_cents;
  const receivedText = revenueFull
    ? formatTotals(revenueFull)
    : (orders ? formatTotals(sumByCurrency(paid)) : "—");
  const paidCount = typeof stats?.paid_orders === "number" ? stats.paid_orders : (orders ? paid.length : null);

  const loadedUsers = users ?? [];
  const active = (u: User) =>
    !!u.plan && u.plan !== "none" &&
    (!u.plan_expires_at || new Date(u.plan_expires_at).getTime() > Date.now());

  // Plan mix. Ordered tiers, so the ring uses a SEQUENTIAL ramp — see Donut for why this design
  // system cannot use a categorical one.
  // 档位次序跟服务端走（settings 里的 plans 已按 rank 排好）。
  //
  // 原来是前端一份白名单。它不只影响排序：下面 filter 之后用「总人数 − 各扇区之和」
  // 当「无会员」，所以任何**不在白名单里**的套餐（比如线上已经存在的 ceshi，
  // 或者运营以后新建的任何一档）的有效会员，都会被静默算进「无会员」。
  const PLAN_ORDER: string[] = planKeys().slice().reverse();
  const planMix = (() => {
    const by = new Map<string, number>();
    // 优先用服务端的全量分组（判据和下面 active() 一致：有套餐、不是 none、没过期）。
    // 退回按列表统计时，那只是"最近 500 位用户里的构成"——环心的「总客户」也跟着只算这 500 位，
    // 而它旁边的「总用户」是真 count(*)。两个数并排、一个真一个截断，正是这次要修的。
    const full = stats?.plan_mix;
    if (full) {
      for (const [k, n] of Object.entries(full)) if (n > 0) by.set(k, n);
    } else {
      for (const u of loadedUsers) {
        if (!active(u)) continue;
        const k = String(u.plan);
        by.set(k, (by.get(k) || 0) + 1);
      }
    }
    // PLAN_ORDER 之外的套餐（线上已存在的 ceshi、运营新建的任何一档）不能被静默吞进
    // 「无会员」——按 rank 排的先画，剩下的按名字补在后面。
    const ordered = PLAN_ORDER.filter((k) => by.get(k));
    const extra = [...by.keys()].filter((k) => !PLAN_ORDER.includes(k)).sort();
    const slices = [...ordered, ...extra].map((k) => ({ label: k, value: by.get(k)! }));
    // 分母也要用真总数，否则「无会员」= 500 − 有效会员，是个截断出来的数。
    const totalUsers = typeof stats?.total_users === "number" ? stats.total_users : loadedUsers.length;
    const none = totalUsers - slices.reduce((a, x) => a + x.value, 0);
    if (none > 0) slices.push({ label: "无会员", value: none });
    return slices;
  })();

  // Signups per day over the last 14 days, from created_at. Dates are bucketed on the LOCAL day
  // so the chart matches what the operator sees elsewhere in the console.
  const signups = (() => {
    const days: { t: string; v: number }[] = [];
    const key = (d: Date) => `${d.getMonth() + 1}/${d.getDate()}`;
    const bucket = new Map<string, number>();
    for (const u of loadedUsers) {
      if (!u.created_at) continue;
      const d = new Date(u.created_at);
      if (Number.isNaN(d.getTime())) continue;
      if (Date.now() - d.getTime() > 14 * 86_400_000) continue;
      bucket.set(key(d), (bucket.get(key(d)) || 0) + 1);
    }
    for (let i = 13; i >= 0; i--) {
      const d = new Date(Date.now() - i * 86_400_000);
      days.push({ t: key(d), v: bucket.get(key(d)) || 0 });
    }
    return days;
  })();

  // Fleet quota pressure: how much of the granted window is actually spent. A ratio, so a METER.
  const quota = loadedUsers.reduce(
    (a, u) => {
      const cap = u.quota_window_cap_cents || 0;
      if (cap <= 0) return a;
      const left = Math.max(0, Math.min(cap, u.quota_window_cents ?? cap));
      return { cap: a.cap + cap, used: a.used + (cap - left) };
    },
    { cap: 0, used: 0 },
  );

  return (
    <div className="space-y-8">
      <PageHeader
        title="总览"
        description="收入、待处理的事，和刚刚发生了什么。"
        actions={
          <Button variant="outline" size="sm" onClick={() => load()}>
            刷新
          </Button>
        }
      />

      <ErrorState message={err} onRetry={() => load()} />

      {/* 入场错峰：标题 0，往下每段 +70ms，最多四段（展示站 SectionReveal 的 Math.min(i,4)*70）。 */}
      <SectionReveal as="section" delay={70} className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat
          label="已收款"
          value={receivedText}
          hint={paidCount == null ? "读取中"
            : revenueFull ? `${paidCount} 笔（全量）`
            : `${paidCount} 笔（仅最近 ${loadedOrders.length} 笔）`}
        />
        <Stat
          label="未支付订单"
          value={orders ? num(pending.length) : "—"}
          hint={!orders ? "读取中" : pending.length ? "钱没到，Stripe 结清后自动转已支付" : "没有挂着的单"}
        />
        <Stat label="总用户" value={stats ? num(stats.total_users) : "—"} />
        <Stat label="今日新增" value={stats ? num(stats.today_users) : "—"} />
      </SectionReveal>

      {/* 只在真的有漏的时候出现。
          常驻一个写着 0 的卡片是噪音，看几天就没人看了；一条平时不在、一出现就意味着
          正在漏钱的横幅才是信号。实测抓到过两笔：grok-4.6 在 2026-08-28 一天里 717 次 /
          3403 万 token 收 0（新线路还没填每模型价），deepseek-v4-flash-vision-exp 在
          08-29 是 49/57 次。这两笔当时在后台任何一页上都看不见。 */}
      {!!stats?.zero_priced_24h?.calls && (
        <SectionReveal as="section" delay={90}>
          <div className="rounded-xl border border-amber-500/40 bg-amber-500/5 p-5">
            <div className="flex items-baseline justify-between gap-4">
              <h2 className="text-sm font-semibold text-amber-600 dark:text-amber-400">
                近 24 小时有 {num(stats.zero_priced_24h.calls)} 次调用没收到钱
              </h2>
              <span className="text-sm tabular-nums text-muted-foreground">
                {num(stats.zero_priced_24h.tokens)} token
              </span>
            </div>
            <p className="mt-1.5 text-sm text-muted-foreground">
              这些模型在它所在的线路上没有可用单价（每模型价、官方目录、连接级三样都是空或 0），
              于是按 0 收 —— 而上游的钱照付。去「模型」页给它们补一条每模型价格；
              如果本来就想免费，把它配成 <code className="text-xs">mode: "free"</code>，
              这样至少会走免费额度池，也不会再出现在这里。
            </p>
            <ul className="mt-3 space-y-1">
              {stats.zero_priced_24h.models.map((m) => (
                <li key={m.model} className="flex items-baseline justify-between text-sm">
                  <span className="font-mono text-xs">{m.model}</span>
                  <span className="tabular-nums text-muted-foreground">
                    {num(m.calls)} 次 · {num(m.tokens)} token
                  </span>
                </li>
              ))}
            </ul>
          </div>
        </SectionReveal>
      )}

      {/* 免费额度池：收了多少点 vs 实际值多少钱。
          只在池子今天真的动过时出现。这一格不改任何行为，它回答的是一个此前没人能回答
          的问题——「每天送出去的免费额度，成本到底是多少」。池子扣点扣的是**售价**，
          而售价可以被显式配成 0，于是 deepseek-v4-pro 那种模型每次只扣地板 1 毫点，
          4.5 万 token 和 45 个 token 一样。 */}
      {!!stats?.free_pool_24h?.milli_points && (
        <SectionReveal as="section" delay={100}>
          <div className="rounded-xl border border-border bg-card">
            <header className="flex items-baseline justify-between border-b border-border px-5 py-3">
              <h2 className="text-sm font-semibold">免费额度池 · 近 24 小时</h2>
              <span className="text-sm tabular-nums text-muted-foreground">
                收 {num(stats.free_pool_24h.milli_points)} 毫点 · 实际值 ¥
                {(stats.free_pool_24h.ref_cny_cents / 100).toFixed(2)}
              </span>
            </header>
            <div className="p-5">
              <table className="w-full text-sm">
                <thead className="text-left type-eyebrow text-muted-foreground">
                  <tr>
                    <th className="pb-2 font-normal">模型</th>
                    <th className="pb-2 text-right font-normal">调用</th>
                    <th className="pb-2 text-right font-normal">实扣毫点</th>
                    <th className="pb-2 text-right font-normal">按成本该扣</th>
                    <th className="pb-2 text-right font-normal">偏差</th>
                  </tr>
                </thead>
                <tbody>
                  {stats.free_pool_24h.models.map((m) => {
                    // 两边都要非零才谈得上倍数；否则只说明还没有参考价。
                    const r = m.milli_points > 0 && m.should_milli_points > 0
                      ? m.should_milli_points / m.milli_points
                      : null;
                    return (
                      <tr key={m.model} className="border-t border-border">
                        <td className="py-1.5 font-mono text-xs">{m.model}</td>
                        <td className="py-1.5 text-right tabular-nums">{num(m.calls)}</td>
                        <td className="py-1.5 text-right tabular-nums">{num(m.milli_points)}</td>
                        <td className="py-1.5 text-right tabular-nums">
                          {m.unpriced_calls === m.calls ? "—" : num(m.should_milli_points)}
                        </td>
                        <td className="py-1.5 text-right tabular-nums">
                          {r === null ? (
                            <span className="text-muted-foreground">—</span>
                          ) : r >= 2 ? (
                            <span className="text-amber-600">少扣 {r.toFixed(0)} 倍</span>
                          ) : r <= 0.5 ? (
                            <span className="text-sky-600">多扣 {(1 / r).toFixed(0)} 倍</span>
                          ) : (
                            <span className="text-muted-foreground">大致相当</span>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              <p className="mt-3 text-sm text-muted-foreground">
                池子扣的是<strong>售价</strong>：把某个模型的每模型价显式配成
                {" "}<code className="text-xs">{'{"in":0,"out":0}'}</code>{" "}
                之后，它每次只扣地板 1 毫点——4.5 万 token 和 45 个 token 一样。
                反过来，按次计价的模型每次扣固定点数，而它一次可能只值 $0.003。
                这一栏只报数，不改扣费：免费额度该值多少是定价决策。
                {stats.free_pool_24h.unpriced_calls > 0 && (
                  <>
                    {" "}其中 {num(stats.free_pool_24h.unpriced_calls)} 次还没有参考价
                    （实时目录里没有这个模型，或这一行写在参考价上线之前），没算进「按成本该扣」。
                  </>
                )}
              </p>
            </div>
          </div>
        </SectionReveal>
      )}

      {/* Two things the old dashboard could not answer: what SHAPE the customer base is, and
          whether signups are moving. Both come from the users list already being fetched. */}
      <div className="mb-6 grid gap-6 lg:grid-cols-2">
        <SectionReveal as="section" delay={120}>
          <div className="h-full rounded-xl border border-border bg-card">
            <header className="border-b border-border px-5 py-3">
              <h2 className="text-sm font-semibold">套餐构成</h2>
            </header>
            <div className="p-5">
              {!users ? (
                <div className="h-36 animate-pulse rounded-lg bg-secondary motion-reduce:animate-none" />
              ) : planMix.length ? (
                <Donut
                  slices={planMix}
                  centerValue={num(loadedUsers.length)}
                  centerLabel="总客户"
                />
              ) : (
                <EmptyState compact title="还没有客户" hint="有人注册后会出现在这里。" />
              )}
            </div>
          </div>
        </SectionReveal>

        <SectionReveal as="section" delay={160}>
          <div className="h-full rounded-xl border border-border bg-card">
            <header className="flex items-baseline justify-between border-b border-border px-5 py-3">
              <h2 className="text-sm font-semibold">近 14 天注册</h2>
              <span className="text-sm tabular-nums text-muted-foreground">
                共 {num(signups.reduce((a, d) => a + d.v, 0))} 位
              </span>
            </header>
            <div className="p-5">
              {!users ? (
                <div className="h-28 animate-pulse rounded-lg bg-secondary motion-reduce:animate-none" />
              ) : (
                <TrendArea points={signups} label="近 14 天每日注册数" />
              )}
              {quota.cap > 0 && (
                <div className="mt-5 border-t border-border pt-4">
                  <div className="mb-1.5 flex items-baseline justify-between">
                    <span className="type-eyebrow">全站时段额度已用</span>
                    <span className="text-sm tabular-nums">
                      {Math.round((quota.used / quota.cap) * 100)}%
                    </span>
                  </div>
                  <Meter used={quota.used} cap={quota.cap} />
                </div>
              )}
            </div>
          </div>
        </SectionReveal>
      </div>

      {/* Equal heights. The feed decided the row height and the orders panel was left short and
          floating — items-stretch plus h-full on both cards makes them match, and the feed's own
          list scrolls inside a fixed body rather than growing the card. */}
      <div className="grid items-stretch gap-6 lg:grid-cols-2">
        <SectionReveal as="section" delay={200} className="flex">
          <div className="flex w-full flex-col rounded-xl border border-border bg-card">
            <header className="flex items-center justify-between border-b border-border px-5 py-3">
              <h2 className="text-sm font-semibold">未支付订单</h2>
              {pending.length > 0 && <Badge variant="outline">{pending.length}</Badge>}
            </header>
            {!orders ? (
              <TableSkeleton rows={4} columns={["46%", "18%"]} label="未支付订单读取中" />
            ) : (
              <div className="flex flex-1 flex-col divide-y divide-border">
                {pending.slice(0, 6).map((o) => (
                  <div key={o.id} className="flex items-baseline justify-between gap-4 px-5 py-3 text-sm">
                    <span className="min-w-0 truncate" title={o.email || o.id}>
                      {o.email || o.id}
                    </span>
                    {/* 没付钱的单没有实收金额，这里只能显示标价 —— 而标价是人民币。 */}
                    <span className="shrink-0 whitespace-nowrap tabular-nums">
                      {formatMoney(o.amount_cents || 0, "cny")}
                    </span>
                  </div>
                ))}
                {!pending.length && (
                  <EmptyState compact title="没有未支付订单" hint="有人付款后会出现在这里。" />
                )}
              </div>
            )}
          </div>
        </SectionReveal>

        <SectionReveal as="section" delay={210} className="flex">
          <div className="flex w-full flex-col rounded-xl border border-border bg-card">
            <header className="border-b border-border px-5 py-3">
              <h2 className="text-sm font-semibold">实时动态</h2>
            </header>
            {!events ? (
              <TableSkeleton rows={4} columns={["52%", "14%"]} label="实时动态读取中" />
            ) : (
              <div className="max-h-80 flex-1 divide-y divide-border overflow-y-auto">
                {events.slice(0, 6).map((ev, i) => {
                  const who = subject(ev);
                  return (
                    <div
                      key={ev.id ?? i}
                      className="flex items-baseline justify-between gap-4 px-5 py-3 text-sm"
                    >
                      <span className="flex min-w-0 items-baseline gap-2">
                        <span className="shrink-0 font-medium">
                          {EVENT_LABEL[ev.kind || ""] || ev.kind || "—"}
                        </span>
                        {who && (
                          <span className="min-w-0 truncate text-muted-foreground" title={who}>
                            {who}
                          </span>
                        )}
                      </span>
                      <span className="shrink-0 whitespace-nowrap text-xs text-muted-foreground">
                        {when(ev.created_at)}
                      </span>
                    </div>
                  );
                })}
                {!events.length && <EmptyState compact title="暂无动态" />}
              </div>
            )}
          </div>
        </SectionReveal>
      </div>
    </div>
  );
}
