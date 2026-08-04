import { useEffect, useState } from "react";
import { Stat } from "@/components/Stat";
import { Badge } from "@/components/ui/badge";
import { api } from "@/lib/api";
import { cents, num, when } from "@/lib/format";

type Stats = { total_users?: number; today_users?: number; online?: number };
type Order = { id: string; email?: string; amount_cents?: number; status?: string; created_at?: string };
type Event = { id?: string; kind?: string; detail?: string; created_at?: string };

/**
 * Leads with money. The old dashboard's three tiles were 总用户 / 今日新增 / 当前在线 — and
 * 当前在线 counted open ADMIN CONSOLE sockets (realtime.rs only calls touch_presence after
 * ws_authenticate returns an admin uid), so on a one-operator product it was permanently 1.
 * Revenue lived two tabs away on 收款, spend lived on 模型系统.
 */
export function Overview() {
  const [stats, setStats] = useState<Stats>({});
  const [orders, setOrders] = useState<Order[]>([]);
  const [events, setEvents] = useState<Event[]>([]);
  const [err, setErr] = useState("");

  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        const [s, o, e] = await Promise.all([
          api.get<Stats>("/api/admin/stats").catch(() => ({})),
          api.get<Order[] | { items?: Order[] }>("/api/admin/orders").catch(() => []),
          api.get<Event[] | { items?: Event[] }>("/api/admin/events").catch(() => []),
        ]);
        if (!alive) return;
        setStats(s || {});
        setOrders(Array.isArray(o) ? o : o?.items || []);
        setEvents(Array.isArray(e) ? e : e?.items || []);
      } catch (e) {
        if (alive) setErr(e instanceof Error ? e.message : "加载失败");
      }
    };
    load();
    const t = setInterval(load, 30_000);
    return () => { alive = false; clearInterval(t); };
  }, []);

  const paid = orders.filter((o) => o.status === "paid");
  const pending = orders.filter((o) => o.status && o.status !== "paid");
  const revenue = paid.reduce((a, o) => a + (o.amount_cents || 0), 0);

  return (
    <div>
      <h1 className="font-display text-2xl font-semibold tracking-tight">总览</h1>
      <p className="type-measure mt-1 text-muted-foreground">收入、待处理的事，和刚刚发生了什么。</p>

      {err && <p role="alert" className="mt-4 text-sm text-destructive">{err}</p>}

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat label="已收款" value={cents(revenue)} hint={`${paid.length} 笔`} />
        <Stat label="待确认订单" value={num(pending.length)} hint={pending.length ? "需要处理" : "没有积压"} />
        <Stat label="总用户" value={num(stats.total_users)} />
        <Stat label="今日新增" value={num(stats.today_users)} />
      </div>

      <div className="mt-8 grid gap-6 lg:grid-cols-2">
        <section className="rounded-xl border border-border bg-card">
          <header className="flex items-center justify-between border-b border-border px-5 py-3">
            <h2 className="text-sm font-semibold">待确认订单</h2>
            {pending.length > 0 && <Badge variant="outline">{pending.length}</Badge>}
          </header>
          <div className="divide-y divide-border">
            {pending.slice(0, 6).map((o) => (
              <div key={o.id} className="flex items-center justify-between px-5 py-3 text-sm">
                <span className="min-w-0 truncate">{o.email || o.id}</span>
                <span className="ml-4 shrink-0 tabular-nums">{cents(o.amount_cents)}</span>
              </div>
            ))}
            {!pending.length && (
              <p className="px-5 py-8 text-center text-sm text-muted-foreground">没有待确认订单</p>
            )}
          </div>
        </section>

        <section className="rounded-xl border border-border bg-card">
          <header className="border-b border-border px-5 py-3">
            <h2 className="text-sm font-semibold">实时动态</h2>
          </header>
          <div className="divide-y divide-border">
            {events.slice(0, 6).map((ev, i) => (
              <div key={ev.id || i} className="flex items-baseline justify-between gap-4 px-5 py-3 text-sm">
                <span className="min-w-0 truncate">{ev.detail || ev.kind || "—"}</span>
                <span className="shrink-0 text-xs text-muted-foreground">{when(ev.created_at)}</span>
              </div>
            ))}
            {!events.length && (
              <p className="px-5 py-8 text-center text-sm text-muted-foreground">暂无动态</p>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
