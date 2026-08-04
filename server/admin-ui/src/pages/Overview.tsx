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
import { cents, num, when } from "@/lib/format";

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

type Stats = { total_users?: number; today_users?: number; online?: number };

/** pay.rs Order（只取这一屏用得上的字段）。email / amount_cents / status 在库里都是非空。 */
type Order = { id: string; email?: string; amount_cents?: number; status?: string; created_at?: string };

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
  order_paid: "确认收款",
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
  const amount = typeof d?.amount_cents === "number" ? cents(d.amount_cents) : "";
  const action = str(d, "action");
  return [who, amount, action].filter(Boolean).join(" · ");
}

export function Overview() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [orders, setOrders] = useState<Order[] | null>(null);
  const [events, setEvents] = useState<Event[] | null>(null);
  const [users, setUsers] = useState<User[] | null>(null);
  const [err, setErr] = useState("");

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
  const revenue = paid.reduce((a, o) => a + (o.amount_cents || 0), 0);

  const loadedUsers = users ?? [];
  const active = (u: User) =>
    !!u.plan && u.plan !== "none" &&
    (!u.plan_expires_at || new Date(u.plan_expires_at).getTime() > Date.now());

  // Plan mix. Ordered tiers, so the ring uses a SEQUENTIAL ramp — see Donut for why this design
  // system cannot use a categorical one.
  const PLAN_ORDER = ["ultra", "power", "pro", "basic", "trial"];
  const planMix = (() => {
    const by = new Map<string, number>();
    for (const u of loadedUsers) {
      if (!active(u)) continue;
      const k = String(u.plan);
      by.set(k, (by.get(k) || 0) + 1);
    }
    const slices = PLAN_ORDER.filter((k) => by.get(k)).map((k) => ({ label: k, value: by.get(k)! }));
    const none = loadedUsers.length - slices.reduce((a, x) => a + x.value, 0);
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
          value={orders ? cents(revenue) : "—"}
          hint={orders ? `${paid.length} 笔` : "读取中"}
        />
        <Stat
          label="待确认订单"
          value={orders ? num(pending.length) : "—"}
          hint={!orders ? "读取中" : pending.length ? "需要处理" : "没有积压"}
        />
        <Stat label="总用户" value={stats ? num(stats.total_users) : "—"} />
        <Stat label="今日新增" value={stats ? num(stats.today_users) : "—"} />
      </SectionReveal>

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

      <div className="grid gap-6 lg:grid-cols-2">
        <SectionReveal as="section" delay={200}>
          <div className="rounded-xl border border-border bg-card">
            <header className="flex items-center justify-between border-b border-border px-5 py-3">
              <h2 className="text-sm font-semibold">待确认订单</h2>
              {pending.length > 0 && <Badge variant="outline">{pending.length}</Badge>}
            </header>
            {!orders ? (
              <TableSkeleton rows={4} columns={["46%", "18%"]} label="待确认订单读取中" />
            ) : (
              <div className="divide-y divide-border">
                {pending.slice(0, 6).map((o) => (
                  <div key={o.id} className="flex items-baseline justify-between gap-4 px-5 py-3 text-sm">
                    <span className="min-w-0 truncate" title={o.email || o.id}>
                      {o.email || o.id}
                    </span>
                    <span className="shrink-0 whitespace-nowrap tabular-nums">{cents(o.amount_cents)}</span>
                  </div>
                ))}
                {!pending.length && (
                  <EmptyState compact title="没有待确认订单" hint="有人付款后会出现在这里。" />
                )}
              </div>
            )}
          </div>
        </SectionReveal>

        <SectionReveal as="section" delay={210}>
          <div className="rounded-xl border border-border bg-card">
            <header className="border-b border-border px-5 py-3">
              <h2 className="text-sm font-semibold">实时动态</h2>
            </header>
            {!events ? (
              <TableSkeleton rows={4} columns={["52%", "14%"]} label="实时动态读取中" />
            ) : (
              <div className="divide-y divide-border">
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
