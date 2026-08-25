import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, RefreshCw, TrendingDown, TrendingUp } from "lucide-react";

import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { VendorMark } from "@/components/VendorMark";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Truncate,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * 对账 —— 每个中转收我们多少，我们从用户收回来多少，差额是正是负。
 *
 * # 为什么成本有两列，而不是一个数
 *
 * 收入那一侧是确定的（逐笔累加，和计费同源）。成本那一侧没有任何接口能直接回答，
 * 所以这里给两个来源不同的数，让它们互相校验：
 *
 *   · **估算** = 收入 × 进价折扣 ÷ 计费倍率。两边同底（都是 tokens × 官方价），
 *     所以这个恒等式成立。好处是部署当天就有数。前提是进价折扣填对了。
 *   · **实测** = 中转账户余额（或「已用」）实际掉了多少。真金白银，不依赖任何人
 *     填对什么。代价是要等两次快照，而且有些中转不给余额接口。
 *
 * 两个数差得远，说明进价折扣填错了，或者上游在按另一份价目表收费 —— 那正是这一页
 * 要抓的东西。合并成一个数就再也看不出来了。
 *
 * # 为什么「没有数」不显示成 0
 *
 * 没攒够余额采样、这家不给余额接口、期间充过值 —— 这三种都是「算不出来」，
 * 不是「成本为零」。显示成 0 会让毛利凭空变好看，而且那一行会排到最上面
 * （按毛利升序）冒充亏损。所以一律显示横杠，并在同一行给出原因。
 */

type Row = {
  endpoint_id: string;
  route_id: string;
  route_label: string;
  label: string;
  vendor: string;
  is_own: boolean;
  active: boolean;
  cost_ratio: number;
  rate: number;
  calls: number;
  prompt_tokens: number;
  completion_tokens: number;
  cached_tokens: number;
  revenue_usd: number;
  cost_est_usd: number | null;
  cost_real_usd: number | null;
  cost_real_basis: "used" | "remaining" | null;
  balance_samples: number;
  margin_usd: number | null;
  margin_pct: number | null;
  margin_basis: "real" | "est" | null;
  note: string;
};

type Payload = {
  days: number;
  rows: Row[];
  totals: {
    revenue_usd: number;
    counted_revenue_usd: number;
    cost_usd: number;
    margin_usd: number;
    counted_rows: number;
    total_rows: number;
  };
};

const usd = (v: number | null | undefined) =>
  v === null || v === undefined ? "—" : `$${v.toFixed(2)}`;

const RANGES = [1, 7, 30] as const;

export function Reconcile() {
  const [data, setData] = useState<Payload | null>(null);
  const [days, setDays] = useState<number>(7);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const load = useCallback(async (d: number) => {
    setLoading(true);
    try {
      setData(await api.get<Payload>(`/api/admin/reconciliation?days=${d}`));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load(days);
  }, [load, days]);

  const t = data?.totals;
  // 合计只算「有成本数」的那些行。分母也必须跟着换成同一批行的收入，
  // 否则毛利率会拿全部收入去除一个只覆盖了部分行的成本，看起来永远很健康。
  const totalPct =
    t && t.counted_revenue_usd > 0 ? (t.margin_usd / t.counted_revenue_usd) * 100 : null;
  const partial = !!t && t.counted_rows < t.total_rows;

  return (
    <div className="space-y-4">
      <PageHeader
        title="对账"
        description="每个中转收我们多少、我们从用户收回来多少，差额是正是负。"
        actions={
          <div className="flex items-center gap-2">
            {RANGES.map((d) => (
              <Button
                key={d}
                size="sm"
                variant={days === d ? "default" : "outline"}
                onClick={() => setDays(d)}
              >
                {d === 1 ? "今天" : `${d} 天`}
              </Button>
            ))}
            <Button size="sm" variant="outline" onClick={() => void load(days)} disabled={loading}>
              <RefreshCw className={cn("mr-1.5 h-3.5 w-3.5", loading && "animate-spin")} />
              刷新
            </Button>
          </div>
        }
      />

      {error && <ErrorState message={error} onRetry={() => void load(days)} />}

      {t && (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <Stat label="用户付了" value={usd(t.revenue_usd)} hint={`最近 ${data?.days} 天`} />
          <Stat
            label="中转收了"
            value={usd(t.cost_usd)}
            hint={partial ? `只统计了 ${t.counted_rows}/${t.total_rows} 个出口` : "全部出口"}
          />
          <Stat
            label="毛利"
            value={usd(t.margin_usd)}
            hint={totalPct === null ? "没有可比的收入" : `${totalPct.toFixed(1)}%`}
          />
          <Stat
            label="亏钱的出口"
            value={data.rows.filter((r) => (r.margin_usd ?? 0) < 0).length}
            hint="毛利为负"
          />
        </div>
      )}

      {/*
        合计口径必须写在脸上。第一版把「还没攒够余额采样的出口」按零成本计入合计，
        毛利看着很健康 —— 而那正是最该被怀疑的几行。
      */}
      {partial && (
        <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/5 px-3 py-2 text-[13px]">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-500" />
          <span>
            上面的合计只包含 <b>{t?.counted_rows}</b> / {t?.total_rows} 个出口 ——
            其余的成本还算不出来（原因见每行末尾）。它们<b>没有</b>被当成零成本计入，
            否则毛利会凭空变好看。
          </span>
        </div>
      )}

      {!data && !error && <TableSkeleton rows={6} />}

      {data && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <p className="text-[13px] text-muted-foreground">
                成本有两列：<b>估算</b>按「收入 × 进价折扣 ÷ 计费倍率」推，部署当天就有数；
                <b>实测</b>是中转账户余额实际掉了多少，要等两次快照。
                两个数差得远，说明进价折扣填错了，或者上游在按另一份价目表收费。
              </p>
            </CardHeader>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>出口</TableHead>
                    <TableHead className="numeric">调用</TableHead>
                    <TableHead className="numeric">用户付</TableHead>
                    <TableHead className="numeric">成本(估)</TableHead>
                    <TableHead className="numeric">成本(实测)</TableHead>
                    <TableHead className="numeric">毛利</TableHead>
                    <TableHead>说明</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.rows.map((r) => {
                    const losing = r.margin_usd !== null && r.margin_usd < 0;
                    // 估算和实测差一倍以上就点出来 —— 这一页最有价值的信号就是这个矛盾。
                    const bothKnown = r.cost_est_usd !== null && r.cost_real_usd !== null;
                    const diverges =
                      bothKnown &&
                      Math.max(r.cost_est_usd!, r.cost_real_usd!) >
                        2 * Math.max(Math.min(r.cost_est_usd!, r.cost_real_usd!), 0.01);
                    return (
                      <TableRow key={r.endpoint_id} className={cn(losing && "bg-destructive/5")}>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <VendorMark vendor={r.vendor} className="h-4 w-4 shrink-0" />
                            <div className="min-w-0">
                              <Truncate title={r.label}>{r.label}</Truncate>
                              <p className="text-[11px] text-muted-foreground">
                                进价 ×{r.cost_ratio} · 计费 ×{r.rate}
                                {!r.active && " · 已停用"}
                              </p>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="numeric">{num(r.calls)}</TableCell>
                        <TableCell className="numeric">{usd(r.revenue_usd)}</TableCell>
                        <TableCell className="numeric text-muted-foreground">
                          {usd(r.cost_est_usd)}
                        </TableCell>
                        <TableCell className="numeric">
                          {usd(r.cost_real_usd)}
                          {r.cost_real_basis && (
                            <span className="ml-1 text-[11px] text-muted-foreground">
                              {r.cost_real_basis === "used" ? "已用" : "余额差"}
                            </span>
                          )}
                        </TableCell>
                        <TableCell className="numeric">
                          {r.margin_usd === null ? (
                            "—"
                          ) : (
                            <span
                              className={cn(
                                "inline-flex items-center gap-1 font-medium",
                                losing ? "text-destructive" : "text-emerald-600",
                              )}
                            >
                              {losing ? (
                                <TrendingDown className="h-3.5 w-3.5" />
                              ) : (
                                <TrendingUp className="h-3.5 w-3.5" />
                              )}
                              {usd(r.margin_usd)}
                              {r.margin_pct !== null && (
                                <span className="text-[11px] font-normal opacity-70">
                                  {r.margin_pct.toFixed(0)}%
                                </span>
                              )}
                            </span>
                          )}
                        </TableCell>
                        <TableCell>
                          <div className="flex flex-wrap items-center gap-1.5">
                            {/* 毛利是按哪个成本算的，必须标出来：估算和实测的可信度差一个量级。 */}
                            {r.margin_basis === "est" && (
                              <Badge variant="outline" className="text-[11px]">
                                按估算
                              </Badge>
                            )}
                            {diverges && (
                              <Badge
                                variant="outline"
                                className="border-destructive/50 text-[11px] text-destructive"
                              >
                                估算与实测差一倍以上
                              </Badge>
                            )}
                            {r.note && (
                              <span className="text-[12px] text-muted-foreground">{r.note}</span>
                            )}
                          </div>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          </Card>
        </SectionReveal>
      )}
    </div>
  );
}
