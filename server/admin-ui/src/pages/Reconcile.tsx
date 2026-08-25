import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  RefreshCw,
  TrendingDown,
  TrendingUp,
} from "lucide-react";

import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { VendorMark } from "@/components/VendorMark";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
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
 * 对账 —— 每个中转真实收了我们多少，我们从用户收回来多少。
 *
 * # 这一页只有真数，没有估算
 *
 * 上一版有一列「估算成本」，算法是 `收入 × 进价折扣 ÷ 计费倍率`。它的问题不是不准，
 * 是**它的前提不是事实**：进价折扣是多路由用来排序出口的旋钮，不是价格。用户点名
 * 要真实计算，那一列已经整个拿掉，服务端还有一条测试守着它不许回来。
 *
 * 现在的成本 = **真实 token × 真实单价**：
 *
 *   · token 来自上游自己的 usage 帧 —— 我们扣用户的钱就是按这个数算的，同源；
 *   · 单价是你从中转后台价目页抄进来的，一个模型一个出口一份。
 *
 * # 没录价就显示「未知」，绝不按 0 算
 *
 * 一个出口只要有一个用过的模型没录价，整行成本就是未知。把没录的那部分按 0 加进来，
 * 得到的是一个看起来很精确的错数字 —— 而且它会让那一行显示成高毛利，正好骗过
 * 「亏得最狠的排最前」那个排序。
 *
 * # 录价就在这一页
 *
 * 点开一行就是按模型的明细，价直接填在那儿。让人跑去另一页再回来，多数人会放弃，
 * 然后这一页永远只有横杠 —— 和多路由那边「发现新模型顺手定价」是同一个道理。
 */

type ModelRow = {
  model_id: string;
  calls: number;
  prompt_tokens: number;
  completion_tokens: number;
  cached_tokens: number;
  revenue_usd: number;
  cost_usd: number | null;
  margin_usd: number | null;
  input_per_mtok: number | null;
  output_per_mtok: number | null;
  cached_per_mtok: number | null;
  price_note: string;
};

type Row = {
  endpoint_id: string;
  route_id: string;
  route_label: string;
  label: string;
  vendor: string;
  is_own: boolean;
  active: boolean;
  calls: number;
  revenue_usd: number;
  cost_usd: number | null;
  margin_usd: number | null;
  margin_pct: number | null;
  unpriced_models: string[];
  cost_by_balance_usd: number | null;
  balance_basis: "used" | "remaining" | null;
  balance_note: string;
  models: ModelRow[];
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
    unpriced_models: number;
  };
};

const usd = (v: number | null | undefined) =>
  v === null || v === undefined ? "—" : `$${v.toFixed(2)}`;

const RANGES = [1, 7, 30] as const;

/**
 * 一页几行。
 *
 * 客户端分页：这张表一行是一个出口，量级是几十，随配置增长而不随流量增长。
 * 为几十行做服务端翻页，换来的是每翻一页多一个往返，以及排序要搬进 SQL。
 *
 * 合计**永远按全量算**（用接口回的 totals），翻页时头部四个数字一个都不变 ——
 * 那是分页在财务汇总上最容易出的错。
 */
const PAGE_SIZE = 12;

export function Reconcile() {
  const [data, setData] = useState<Payload | null>(null);
  const [days, setDays] = useState<number>(7);
  const [page, setPage] = useState(1);
  const [open, setOpen] = useState<string | null>(null);
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
    // 换区间等于换一份数据，停在第 3 页多半是空的。回第一页 —— 而且第一页正好
    // 是最该看的：整表按毛利升序，亏得最狠的排在最前。
    setPage(1);
  }, [load, days]);

  const t = data?.totals;
  const allRows = data?.rows ?? [];
  const pages = Math.max(1, Math.ceil(allRows.length / PAGE_SIZE));
  // 数据变短（换了区间、或出口被删）时当前页可能越界。夹住而不是显示空表。
  const current = Math.min(page, pages);
  const shown = allRows.slice((current - 1) * PAGE_SIZE, current * PAGE_SIZE);
  // 合计的分母要和分子取自同一批行，否则毛利率会拿全部收入去除一个只覆盖部分行的成本。
  const totalPct =
    t && t.counted_revenue_usd > 0 ? (t.margin_usd / t.counted_revenue_usd) * 100 : null;

  return (
    <div className="space-y-4">
      <PageHeader
        title="对账"
        description="真实 token × 真实单价。没有估算——单价没录就显示未知，不按 0 算。"
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
            hint={`已录价的 ${t.counted_rows}/${t.total_rows} 个出口`}
          />
          <Stat
            label="毛利"
            value={usd(t.margin_usd)}
            hint={totalPct === null ? "没有可比的收入" : `${totalPct.toFixed(1)}%`}
          />
          <Stat
            label="待录单价"
            value={t.unpriced_models}
            hint={t.unpriced_models === 0 ? "全部已录" : "个模型，点开对应行填"}
          />
        </div>
      )}

      {!!t && t.unpriced_models > 0 && (
        <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/5 px-3 py-2 text-[13px]">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-500" />
          <span>
            还有 <b>{t.unpriced_models}</b> 个模型没录进价。用到它们的出口<b>整行</b>成本都算不出来
            —— 一行里漏一个模型，剩下那些乘出来的数字就不是这一行的成本了。
            点开行末的箭头，在明细里把价填上（抄中转后台的价目页，单位是每百万 token 美元）。
          </span>
        </div>
      )}

      {!data && !error && <TableSkeleton rows={6} />}

      {data && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <p className="text-[13px] text-muted-foreground">
                成本 = 真实 token × 你录入的真实单价。token 来自上游自己的 usage 帧，
                和扣用户钱用的是同一个数。缓存命中的输入会按缓存价单独算——不减出来的话，
                命中率高的模型成本会被高估好几倍。
              </p>
            </CardHeader>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>出口</TableHead>
                    <TableHead className="numeric">调用</TableHead>
                    <TableHead className="numeric">用户付</TableHead>
                    <TableHead className="numeric">中转收</TableHead>
                    <TableHead className="numeric">毛利</TableHead>
                    <TableHead>说明</TableHead>
                    <TableHead />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {shown.map((r) => {
                    const losing = r.margin_usd !== null && r.margin_usd < 0;
                    const expanded = open === r.endpoint_id;
                    return [
                      <TableRow key={r.endpoint_id} className={cn(losing && "bg-destructive/5")}>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <VendorMark vendor={r.vendor} className="h-4 w-4 shrink-0" />
                            <div className="min-w-0">
                              <Truncate title={r.label}>{r.label}</Truncate>
                              <p className="text-[11px] text-muted-foreground">
                                {r.models.length} 个模型{!r.active && " · 已停用"}
                              </p>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="numeric">{num(r.calls)}</TableCell>
                        <TableCell className="numeric">{usd(r.revenue_usd)}</TableCell>
                        <TableCell className="numeric">{usd(r.cost_usd)}</TableCell>
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
                            {r.unpriced_models.length > 0 && (
                              <Badge
                                variant="outline"
                                className="border-amber-500/50 text-[11px] text-amber-600"
                              >
                                {r.unpriced_models.length} 个模型待录价
                              </Badge>
                            )}
                            {r.calls === 0 && (
                              <span className="text-[12px] text-muted-foreground">
                                这段时间没跑过
                              </span>
                            )}
                            {r.cost_by_balance_usd !== null && (
                              <span className="text-[12px] text-muted-foreground">
                                余额口径 {usd(r.cost_by_balance_usd)}
                              </span>
                            )}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => setOpen(expanded ? null : r.endpoint_id)}
                          >
                            <ChevronDown
                              className={cn(
                                "h-4 w-4 transition-transform",
                                expanded && "rotate-180",
                              )}
                            />
                          </Button>
                        </TableCell>
                      </TableRow>,
                      expanded ? (
                        <TableRow key={r.endpoint_id + "-detail"}>
                          <TableCell colSpan={7} className="bg-muted/30 p-0">
                            <ModelDetail row={r} onSaved={() => void load(days)} />
                          </TableCell>
                        </TableRow>
                      ) : null,
                    ];
                  })}
                </TableBody>
              </Table>
            </div>
            {pages > 1 && (
              <div className="flex items-center gap-2 border-t border-border px-5 py-3 text-xs text-muted-foreground">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={current <= 1}
                  onClick={() => setPage((p) => Math.max(1, p - 1))}
                >
                  <ChevronLeft className="h-3.5 w-3.5" /> 上一页
                </Button>
                <span className="tabular-nums">
                  第 {current} / {pages} 页 · 共 {allRows.length} 个出口
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={current >= pages}
                  onClick={() => setPage((p) => Math.min(pages, p + 1))}
                >
                  下一页 <ChevronRight className="h-3.5 w-3.5" />
                </Button>
                <span className="ml-auto">按毛利升序 · 亏得最狠的在第 1 页</span>
              </div>
            )}
          </Card>
        </SectionReveal>
      )}
    </div>
  );
}

/** 一个出口按模型展开：每个模型花了多少、单价是多少，就地能改。 */
function ModelDetail({ row, onSaved }: { row: Row; onSaved: () => void }) {
  if (row.models.length === 0) {
    return (
      <p className="px-5 py-4 text-[13px] text-muted-foreground">
        这个出口在所选时间段里没有调用记录。
      </p>
    );
  }
  return (
    <div className="px-5 py-3">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>模型</TableHead>
            <TableHead className="numeric">调用</TableHead>
            <TableHead className="numeric">输入 / 其中缓存</TableHead>
            <TableHead className="numeric">输出</TableHead>
            <TableHead className="numeric">用户付</TableHead>
            <TableHead className="numeric">中转收</TableHead>
            <TableHead>进价（$ / 百万 token）</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {row.models.map((m) => (
            <PriceRow key={m.model_id} endpointId={row.endpoint_id} m={m} onSaved={onSaved} />
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

function PriceRow({
  endpointId,
  m,
  onSaved,
}: {
  endpointId: string;
  m: ModelRow;
  onSaved: () => void;
}) {
  const [inp, setInp] = useState(m.input_per_mtok?.toString() ?? "");
  const [out, setOut] = useState(m.output_per_mtok?.toString() ?? "");
  const [cache, setCache] = useState(m.cached_per_mtok?.toString() ?? "");
  const [saving, setSaving] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  // 输入和输出必须都填。只填一个存下去，成本会按另一项为 0 算出来 —— 一个偏低的
  // 真数字比一个明显的空白更危险，因为它看起来是有效的。
  const ready = inp.trim() !== "" && out.trim() !== "";

  async function save() {
    setSaving(true);
    setErr(null);
    try {
      await api.post("/api/admin/endpoint-prices", {
        endpoint_id: endpointId,
        model_id: m.model_id,
        input_per_mtok: Number(inp),
        output_per_mtok: Number(out),
        // 空 = 这家不单独计缓存价，后端按输入价算（保守方向）。
        cached_per_mtok: cache.trim() === "" ? null : Number(cache),
      });
      onSaved();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "保存失败");
    }
    setSaving(false);
  }

  const unpriced = m.cost_usd === null;
  return (
    <TableRow className={cn(unpriced && "bg-amber-500/5")}>
      <TableCell className="font-mono text-[12px]">
        <Truncate title={m.model_id}>{m.model_id}</Truncate>
      </TableCell>
      <TableCell className="numeric">{num(m.calls)}</TableCell>
      <TableCell className="numeric text-[12px]">
        {num(m.prompt_tokens)}
        <span className="text-muted-foreground"> / {num(m.cached_tokens)}</span>
      </TableCell>
      <TableCell className="numeric text-[12px]">{num(m.completion_tokens)}</TableCell>
      <TableCell className="numeric">{usd(m.revenue_usd)}</TableCell>
      <TableCell className="numeric">
        {usd(m.cost_usd)}
        {m.margin_usd !== null && (
          <span
            className={cn(
              "ml-1 text-[11px]",
              m.margin_usd < 0 ? "text-destructive" : "text-muted-foreground",
            )}
          >
            ({usd(m.margin_usd)})
          </span>
        )}
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-1.5">
          <Input
            className="h-7 w-20 text-xs"
            placeholder="输入"
            value={inp}
            onChange={(e) => setInp(e.target.value)}
          />
          <Input
            className="h-7 w-20 text-xs"
            placeholder="输出"
            value={out}
            onChange={(e) => setOut(e.target.value)}
          />
          <Input
            className="h-7 w-20 text-xs"
            placeholder="缓存(选填)"
            value={cache}
            onChange={(e) => setCache(e.target.value)}
          />
          <Button size="sm" variant="outline" disabled={!ready || saving} onClick={() => void save()}>
            {saving ? "…" : "保存"}
          </Button>
          {err && <span className="text-[11px] text-destructive">{err}</span>}
        </div>
      </TableCell>
    </TableRow>
  );
}
