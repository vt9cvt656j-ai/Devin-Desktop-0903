import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  ChevronDown,
  RefreshCw,
  TrendingDown,
  TrendingUp,
} from "lucide-react";

import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Pager } from "@/components/Pager";
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
 * 上一版有一列「估算成本」，算法是 `收入 × 进价倍率 ÷ 计费倍率`。它的问题不是不准，
 * 是**它的前提不是事实**：进价倍率是多路由用来排序出口的旋钮，不是价格。用户点名
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
  /** 写进缓存的 token。成本大头，而且以前既没记也没算。 */
  cache_creation_tokens: number;
  revenue_usd: number;
  cost_usd: number | null;
  margin_usd: number | null;
  input_per_mtok: number | null;
  output_per_mtok: number | null;
  cached_per_mtok: number | null;
  price_note: string;
  /** 这个价是推算的（OpenRouter 官方价 × 倍率），不是抓来的。 */
  price_derived: boolean;
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
  /** 折成人民币的三个数。页面显示的就是它们——`*_usd` 那三个不是同一种货币，不能相减。 */
  revenue_cny: number | null;
  cost_cny: number | null;
  margin_cny: number | null;
  cost_by_balance_cny: number | null;
  derived_cost_cny: number | null;
  /** 成本折不出来时说明是哪一家没填充值汇率。收入永远折得出来（走全局汇率）。 */
  fx_note: string;
  unpriced_models: string[];
  legacy_only: boolean;
  cost_by_balance_usd: number | null;
  balance_basis: "used" | "remaining" | null;
  balance_note: string;
  models: ModelRow[];
};

type Account = {
  base_url: string;
  routes: string[];
  spent_usd: number | null;
  user_tokens: number;
  probe_tokens: number;
  implied_per_mtok: number | null;
  predicted_usd: number | null;
  listed_per_mtok: number | null;
  gap_pct: number | null;
  note: string;
};

type Payload = {
  days: number;
  accounts: Account[];
  rows: Row[];
  totals: {
    revenue_usd: number;
    counted_revenue_usd: number;
    cost_usd: number;
    margin_usd: number;
    counted_rows: number;
    total_rows: number;
    unpriced_models: number;
    /** 合计成本里有多少是推算的。 */
    derived_cost_usd: number;
    revenue_cny: number;
    cost_cny: number;
    margin_cny: number;
    derived_cost_cny: number;
    fx_rows: number;
    fx_missing_rows: number;
  };
};

const usd = (v: number | null | undefined) =>
  v === null || v === undefined ? "—" : `$${v.toFixed(2)}`;

/**
 * 人民币。页面上的钱一律用它。
 *
 * 原来这一屏所有金额都打 `$`，而那两侧根本不是同一种「美元」：成本是**这个出口那家
 * 中转的余额面值**，收入是我们按售价算出来的美元。两边不折成人民币就没法相减，线上
 * 清衍那一行因此显示成「毛利 -144%」，而它实际是赚的。
 *
 * 2026-08-31 又改了一次，改的是**收入**用哪把尺子。此前收入也除以 `channel_rates`
 * 里那个站的 `usd_per_cny`，理由是「收入的量纲属于线路自带地址那家」——那个理由在
 * 给钱包加全局折算（c387e33）之后就不成立了：扣用户钱只走一条路、用的是后台那个
 * 全局汇率，供应商给我们的进货折扣跟用户付多少钱毫无关系。实测 7 天，同一批流水
 * 两把尺子折出来的收入差 40%，单 zyz 那条线路差 37.6 倍；而 api.hao.ai 一直是准的，
 * 因为它填的 0.14 恰好 ≈ 1/7.1023 —— 「碰巧对」正是这条错了没人发现的原因。
 * 折算在服务端做（`reconcile.rs`），这里只负责别再打错货币符号。
 */
const cny = (v: number | null | undefined) =>
  v === null || v === undefined ? "—" : `¥${v.toFixed(2)}`;

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

/**
 * 两个入口，一个组件。
 *
 * 值就是 NavKey 本身，不另起一套 "outlets" / "accounts" —— 多一层映射就多一处
 * 能对不上的地方。和「网关适配器」那两屏是同一个做法。
 */
export type ReconcileView = "routing-reconcile" | "routing-reconcile-accounts";

export function Reconcile({ view }: { view: ReconcileView }) {
  const accountsView = view === "routing-reconcile-accounts";
  const [data, setData] = useState<Payload | null>(null);
  const [days, setDays] = useState<number>(7);
  const [page, setPage] = useState(1);
  // 两张表各自翻页。共用一个页码的话，切个入口就跳到一张空表上。
  const [acctPage, setAcctPage] = useState(1);
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
    t && t.revenue_cny > 0 ? (t.margin_cny / t.revenue_cny) * 100 : null;

  const accounts = data?.accounts ?? [];
  const acctPages = Math.max(1, Math.ceil(accounts.length / PAGE_SIZE));
  // 数据变短时夹住，别显示一张空表。
  const acctCur = Math.min(acctPage, acctPages);
  const shownAccounts = accounts.slice((acctCur - 1) * PAGE_SIZE, acctCur * PAGE_SIZE);

  return (
    <div className="space-y-4">
      <PageHeader
        title={accountsView ? "账单核对" : "出口明细"}
        description={
          accountsView
            ? "按价目表算出来的消耗，和中转余额实际掉的钱，对不对得上。"
            : "金额一律折成人民币再比。收入按**用户实际被扣钱的那把尺子**（后台的全局汇率），成本按**这个出口那家的充值汇率**（¥1 在那儿买到多少面值）——两者问的不是同一个问题，供应商给我们的进货折扣和用户付多少钱无关。真实 token × 真实单价；抓不到价目的按「OpenRouter 官方价 × 倍率」推算并标出来，仍然算不出的显示未知，不按 0 算。"
        }
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
          {/*
            三个钱的数**必须同一个分母**。
            上一版「用户付了」画的是全量收入，而「中转收了」和「毛利」只统计
            算得出成本的那几个出口 —— 三个数摆成一排，读起来像一件事，实际是两件。
            线上实测：全量 $450.44，而算得出成本的只有 $77.90（17%），
            于是「毛利 79.7%」看着像在赚钱，其实它只描述了六分之一的生意。
            现在主数字统一用「算得出成本的那一批」，全量放进 hint 当背景。
          */}
          <Stat
            label="用户付了"
            value={cny(t.revenue_cny)}
            hint={
              t.fx_missing_rows > 0
                ? `最近 ${data?.days} 天 · 有 ${t.fx_missing_rows} 个出口因为充值汇率没填，成本折不出来，没算进去`
                : `最近 ${data?.days} 天 · ${t.fx_rows} 个出口`
            }
          />
          <Stat
            label="中转收了"
            value={cny(t.cost_cny)}
            hint={
              t.derived_cost_usd > 0
                ? `其中 ${cny(t.derived_cost_cny)} 是推算的 · 已录价 ${t.counted_rows}/${t.total_rows} 个出口`
                : `同一批出口 · 已录价 ${t.counted_rows}/${t.total_rows} 个`
            }
          />
          <Stat
            label="毛利"
            value={cny(t.margin_cny)}
            hint={
              totalPct === null
                ? "没有可比的收入"
                : t.fx_missing_rows > 0
                  ? `${totalPct.toFixed(1)}% · ${t.fx_missing_rows} 个出口充值汇率没填，没算进去`
                  : `${totalPct.toFixed(1)}%`
            }
          />
          <Stat
            label="待录单价"
            value={t.unpriced_models}
            hint={t.unpriced_models === 0 ? "全部已录" : "个模型，点开对应行填"}
          />
        </div>
      )}

      {/*
        这条提示只挂在「出口明细」上：它的操作指引是「点开行末的箭头，在明细里把价填上」，
        而账单核对那一屏没有那个箭头 —— 一条照做不了的指引比没有指引更糟。
      */}
      {!!t && !accountsView && t.unpriced_models > 0 && (
        <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/5 px-3 py-2 text-[13px]">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-500" />
          <span>
            还有 <b>{t.unpriced_models}</b> 个模型没录进价。用到它们的出口<b>整行</b>成本都算不出来
            —— 一行里漏一个模型，剩下那些乘出来的数字就不是这一行的成本了。
            点开行末的箭头，在明细里把价填上（抄中转后台的价目页，单位是每百万 token 美元）。
          </span>
        </div>
      )}

      {data && accountsView && data.accounts.length === 0 && (
        <p className="rounded-lg border border-border bg-muted/30 px-4 py-6 text-center text-sm text-muted-foreground">
          这段时间没有可核对的账户 —— 要么没有余额读数，要么这几个账户都没跑过。
        </p>
      )}

      {data && accountsView && data.accounts.length > 0 && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <h3 className="text-sm font-medium">账单核对（按账户）</h3>
              <p className="text-[13px] text-muted-foreground">
                <b>按价目表算出来的消耗，和余额实际掉的钱，对不对得上。</b>
                价目表是中转说的，余额差是它实际扣的——两个对不上只有两种可能：
                中转在按另一份价目收费，或者我们抄来的表过期了。
                <br />
                按<b>账户</b>算而不是按线路：同一个中转账户下常常挂着好几把密钥（你的 Claude 和
                GPT 就是），按线路算会把同一笔扣款重复计进两条。探活烧的 token 也算进消耗——
                那笔钱同样是从这个余额里出的。
                <br />
                最后一列的「混合费率」<b>不是单价，不能拿去乘别的用量</b>：真实计费是
                输入×输入价 + 输出×输出价，而输出价通常是输入价的 4~5 倍，所以那个数完全取决于
                这一段的输入输出配比。可比的是前面两列的<b>美元总额</b>。
              </p>
            </CardHeader>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>账户</TableHead>
                    <TableHead className="numeric">价目表算的</TableHead>
                    <TableHead className="numeric">余额实际掉的</TableHead>
                    <TableHead>对不对得上</TableHead>
                    <TableHead className="numeric">用户 token</TableHead>
                    <TableHead className="numeric">探活 token</TableHead>
                    <TableHead className="numeric">混合费率（不是单价）</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {shownAccounts.map((a) => {
                    // 偏差超过一成就点出来 —— 那是「中转按另一份价目收费」最直接的信号。
                    const off = a.gap_pct !== null && Math.abs(a.gap_pct) > 10;
                    return (
                      <TableRow key={a.base_url} className={cn(off && "bg-amber-500/5")}>
                        <TableCell>
                          <Truncate title={a.base_url}>{a.base_url}</Truncate>
                          <p className="text-[11px] text-muted-foreground">
                            {a.routes.join("、")}
                          </p>
                        </TableCell>
                        <TableCell className="numeric">{usd(a.predicted_usd)}</TableCell>
                        <TableCell className="numeric font-medium">{usd(a.spent_usd)}</TableCell>
                        {/*
                          note 和百分比要**并排**，不是二选一。
                          原来写成三元：有 gap 就只画数字。而服务端那句限定语
                          （「只比了最近 N 天」「N 个模型里有 M 个没抓到价」）恰恰是
                          解释这个百分比该怎么读的 —— 有百分比的行反而永远看不到它。
                        */}
                        <TableCell>
                          {a.gap_pct !== null && (
                            <span className={cn("text-[12px]", off && "font-medium text-amber-600")}>
                              {a.gap_pct > 0 ? "+" : ""}
                              {a.gap_pct.toFixed(0)}%
                              {off && " ← 中转扣的和它自己的价目表对不上"}
                            </span>
                          )}
                          {a.note && (
                            <div className="text-[12px] text-muted-foreground">{a.note}</div>
                          )}
                        </TableCell>
                        <TableCell className="numeric text-[12px]">{num(a.user_tokens)}</TableCell>
                        <TableCell className="numeric text-[12px] text-muted-foreground">
                          {num(a.probe_tokens)}
                        </TableCell>
                        {/*
                          混合费率放最后，而且标灰。
                          它 = 余额差 ÷ 总 token，**只反映这一段的输入输出配比**——
                          真实计费是 输入×输入价 + 输出×输出价，而输出价通常是输入价的
                          4~5 倍。拿这个数去乘别的用量会错得离谱，所以它只用来看量级，
                          不是单价。上面那两列（美元总额）才是可比的。
                        */}
                        <TableCell className="numeric text-[12px] text-muted-foreground">
                          {a.implied_per_mtok === null
                            ? "—"
                            : `$${a.implied_per_mtok.toFixed(3)}`}
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
            <Pager page={acctCur} pages={acctPages} total={accounts.length} unit="个账户" onPage={setAcctPage} />
          </Card>
        </SectionReveal>
      )}

      {!data && !error && <TableSkeleton rows={6} />}

      {data && !accountsView && (
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
                    const losing = r.margin_cny !== null && r.margin_cny < 0;
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
                        <TableCell className="numeric">{cny(r.revenue_cny)}</TableCell>
                        <TableCell className="numeric">{cny(r.cost_cny)}</TableCell>
                        <TableCell className="numeric">
                          {r.margin_cny === null ? (
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
                              {cny(r.margin_cny)}
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
                            {/*
                              「没跑过」和「跑过但那时没按模型记账」是两件事。
                              第一版把后者也说成「没跑过」—— 而它跑了几千次。
                              一句听起来正常、实际是假的话，比空白更糟：
                              它会让人以为这条线路闲着。
                            */}
                            {r.legacy_only && (
                              <span className="text-[12px] text-muted-foreground">
                                这段调用发生在「按模型记账」上线之前，没有模型维度，成本拆不出来
                              </span>
                            )}
                            {r.calls === 0 && !r.legacy_only && (
                              <span className="text-[12px] text-muted-foreground">
                                这段时间没跑过
                              </span>
                            )}
                            {r.cost_by_balance_cny !== null && (
                              <span className="text-[12px] text-muted-foreground">
                                余额口径 {cny(r.cost_by_balance_cny)}
                              </span>
                            )}
                            {/*
                              折不出来必须说是**哪一家**没填汇率，否则运维只看到一排「—」，
                              不知道该去填谁。这一行没有汇率就没有毛利可言，比「待录价」更硬。
                            */}
                            {r.fx_note !== "" && (
                              <span className="text-[12px] text-amber-600">{r.fx_note}</span>
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
            <Pager page={current} pages={pages} total={allRows.length} unit="条" onPage={setPage} />
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
      <div className="px-5 py-4">
        <p className="text-[13px] text-muted-foreground">
          这个出口在所选时间段里没有调用记录。
        </p>
        {/*
          没有调用记录 ≠ 不能录价。上一版这里就直接返回了，于是一个**还没跑过流量**
          的出口一条价都填不进去 —— 而自研网关（没有任何可拉的价目接口）恰恰只能靠
          手工录。「只能手工录」这句话必须在这里能落地，否则它是空的。
        */}
        <BulkPrices row={row} onSaved={onSaved} />
      </div>
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
            {/*
              缓存写单独一列。它是成本大头 —— 实测一次 claude-opus-5 调用
              新鲜输入 381、写入 61,634，那一笔的钱几乎全在写入上。
              不画出来的话，「输入才 2 个 token 怎么扣了 46 分」永远问不明白。
            */}
            <TableHead className="numeric">缓存写</TableHead>
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
      <BulkPrices row={row} onSaved={onSaved} />
    </div>
  );
}

/**
 * 标定进价：发两发真实请求，按**余额差**反推真实单价。
 *
 * 这条路不依赖中转公布任何东西 —— 只要能发请求、能查余额就能量。而且量到的是
 * **实际扣的钱**，天然含了分组倍率、活动折扣这些价目表上看不见的东西，比任何
 * 公布的价目都准。线上那六家不给价的中转，这是唯一不用人抄的路。
 *
 * **它会花钱**，所以：只在点按钮时跑、先把要花什么说清楚、花掉多少（两次余额差）
 * 原样回给界面。花了多少必须看得见。
 */
function Calibrate({ row, onSaved }: { row: Row; onSaved: () => void }) {
  const [busy, setBusy] = useState(false);
  const [res, setRes] = useState<CalResult[] | null>(null);
  const [err, setErr] = useState("");

  // 只标这个出口上**真的跑过**的模型：没跑过的标了也用不上，白花钱。
  // 一次最多 12 个，和服务端那道闸一致。
  const models = row.models.map((m) => m.model_id).slice(0, 12);

  async function run() {
    if (
      !window.confirm(
        `标定会给 ${models.length} 个模型各发两发真实请求（一发约 6000 个输入 token，` +
          `一发约 600 个输出 token），这些请求要花钱，从这个出口的余额里扣。\n\n` +
          `换来的是这些模型的真实进价（按余额差反推，比价目表准）。继续吗？`,
      )
    ) {
      return;
    }
    setBusy(true);
    setErr("");
    setRes(null);
    try {
      const r = await api.post<{ calibrated: number; results: CalResult[] }>(
        "/api/admin/endpoint-prices/calibrate",
        { endpoint_id: row.endpoint_id, models },
      );
      setRes(r.results);
      if (r.calibrated > 0) onSaved();
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (models.length === 0) {
    return null;
  }
  return (
    <>
      <Button size="sm" variant="outline" onClick={() => void run()} disabled={busy}>
        {busy ? "标定中…（要发真实请求，慢）" : `标定进价（${models.length} 个模型，会花钱）`}
      </Button>
      {err && <p className="w-full text-[12px] text-destructive">{err}</p>}
      {res && (
        <div className="w-full space-y-1 text-[12px]">
          {res.map((x) => (
            <p key={x.model} className={x.input_per_mtok == null ? "text-amber-600" : "text-emerald-600"}>
              {x.model}：
              {x.input_per_mtok == null
                ? x.why
                : `输入 ${x.input_per_mtok.toFixed(3)} / 输出 ${x.output_per_mtok?.toFixed(3)} 每百万 token` +
                  `（这两发花了 ${x.samples.map((v) => v.delta.toFixed(4)).join(" + ")}）`}
            </p>
          ))}
        </div>
      )}
    </>
  );
}

type CalResult = {
  model: string;
  input_per_mtok: number | null;
  output_per_mtok: number | null;
  samples: { input_tokens: number; output_tokens: number; delta: number }[];
  why: string;
};

/**
 * 从中转后台把整张价目表粘进来，一次录完。
 *
 * 一次一个模型的填法对**自研网关**是不可用的：它们没有任何可拉的价目接口
 * （线上 api.teamorouter.com 挂了 9 个出口，价目只在它自己前端里动态渲染，
 * 服务端拿不到），被人机校验挡住的面板同理。这些站手工录是唯一的路，
 * 而一张表几十个模型一个个填不现实 —— 那等于「只能手工录」没有落地。
 *
 * 单位在标签里写死：$ / 百万 token。单位靠猜是这套账里最贵的一类错。
 */
function BulkPrices({ row, onSaved }: { row: Row; onSaved: () => void }) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<
    { saved: number; skipped: string[]; failed: string[] } | null
  >(null);
  const [err, setErr] = useState("");

  async function submit() {
    setBusy(true);
    setErr("");
    setResult(null);
    try {
      const r = await api.post<{ saved: number; skipped: string[]; failed: string[] }>(
        "/api/admin/endpoint-prices/bulk",
        { endpoint_id: row.endpoint_id, text },
      );
      setResult(r);
      if (r.saved > 0) onSaved();
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!open) {
    return (
      <div className="mt-3 flex flex-wrap gap-2">
        <Button size="sm" variant="outline" onClick={() => setOpen(true)}>
          粘贴整张价目表
        </Button>
        <Calibrate row={row} onSaved={onSaved} />
      </div>
    );
  }
  return (
    <div className="mt-3 space-y-2 rounded-md border p-3">
      <p className="text-[12px] text-muted-foreground">
        一行一个模型：<code>模型名 输入价 输出价 [缓存读价] [缓存写价]</code>。
        单位是 <strong>美元 / 百万 token</strong>，和上面那一列一致。
        制表符、逗号、空格分隔都行，直接从中转后台框选复制过来即可。
        认不出的行会逐条告诉你是第几行、为什么 —— 不会闷头跳过。
      </p>
      <textarea
        className="h-40 w-full rounded-md border bg-background p-2 font-mono text-[12px]"
        placeholder={"claude-opus-5\t5\t25\ngpt-5.6-sol, 1.25, 10, 0.125\ndeepseek-v4-flash 0.28 0.42"}
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
      <div className="flex items-center gap-2">
        <Button size="sm" onClick={() => void submit()} disabled={busy || !text.trim()}>
          {busy ? "存入中…" : "存入"}
        </Button>
        <Button size="sm" variant="ghost" onClick={() => setOpen(false)}>
          收起
        </Button>
      </div>
      {err && <p className="text-[12px] text-destructive whitespace-pre-wrap">{err}</p>}
      {result && (
        <div className="space-y-1 text-[12px]">
          <p className="text-emerald-600">存好了 {result.saved} 个模型的价</p>
          {/* 认不出的和写失败的都要逐条列出来。只报「存了 12 条」而不说另外
              18 条去哪了，人会以为填完了，而对账会拿那些缺口按推算值顶上。 */}
          {result.skipped.map((x) => (
            <p key={x} className="text-amber-600">
              {x}
            </p>
          ))}
          {result.failed.map((x) => (
            <p key={x} className="text-destructive">
              写失败 {x}
            </p>
          ))}
        </div>
      )}
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
      <TableCell
        className={cn(
          "numeric text-[12px]",
          // 写入量大到压过新鲜输入时标出来 —— 那种行的成本几乎全来自这一列，
          // 而它以前既没记也没算。
          m.cache_creation_tokens > m.prompt_tokens && "font-medium text-warning",
        )}
        title="写进缓存的 token。上游按输入价的 1.25 倍收它 —— 常常是这一行成本的大头。"
      >
        {num(m.cache_creation_tokens)}
      </TableCell>
      <TableCell className="numeric text-[12px]">{num(m.completion_tokens)}</TableCell>
      <TableCell className="numeric">{usd(m.revenue_usd)}</TableCell>
      <TableCell className="numeric">
        {usd(m.cost_usd)}
        {/*
          推算的价必须一眼分得出来。混在实测数字里，一个假设就变成了事实 ——
          而这一页的全部价值就在于它说的是真数。
        */}
        {m.price_derived && (
          <span
            className="ml-1 rounded bg-warning/15 px-1 text-[10px] text-warning"
            title={m.price_note || "按 OpenRouter 官方价 × 这个出口的倍率推算，不是抓来的真价"}
          >
            推算
          </span>
        )}
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
