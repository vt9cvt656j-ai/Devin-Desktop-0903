import { useCallback, useEffect, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  RefreshCw,
  ShieldAlert,
  ShieldCheck,
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
import { when } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * 网关适配器 —— 每家中转跑的是什么软件、我们从它那儿自动拿到了什么、还差什么。
 *
 * # 这一页回答的三个问题
 *
 *   · **认出来了吗** —— 是 sub2api / new-api / one-api 系 / OpenRouter，靠哪条指纹认的。
 *     认错比认不出更糟，所以「靠什么认的」必须写在脸上，而不是只给一个结论。
 *   · **拿到了什么** —— 拉到几条真实进价、余额读不读得到。
 *   · **哪些做不到真实记账，为什么** —— 这一条是重点。有些中转把价目接口关了，
 *     有些家族上游根本没有公开价目。那些行的成本在对账页上**永远**是未知，
 *     而这件事必须有名字、有原因、能被数出来，否则它会一直存在而没人当成待办。
 *
 * # 涨价自动停用为什么默认关
 *
 * 停掉唯一一条能服务某个模型的线路，等于用「不亏钱」换「直接断服」。那个取舍
 * 不该由一个后台任务替人做。所以看守默认关；就算开了，如果那是最后一条在转线路，
 * 也**不停**，只把告警升级并在这里显示「本来要停，因为会断服而没停」。
 */

type Row = {
  endpoint_id: string;
  route_id: string;
  label: string;
  base_url: string;
  active: boolean;
  rate: number;
  vendor: string;
  family: string;
  matched_by: string;
  note: string;
  priced_models: number;
  balance_ok: boolean;
  balance_text: string;
  accounting_ready: boolean;
  blocked_reason: string;
  auto_guard: boolean;
  margin_floor_pct: number;
  synced_at: string | null;
  quota_per_unit: number | null;
};

type Change = {
  endpoint_id: string;
  model_id: string;
  old_input: number | null;
  new_input: number | null;
  old_output: number | null;
  new_output: number | null;
  pct: number;
  acted: string;
  at: string;
};

type Plan = {
  endpoint_id: string;
  plan_key: string;
  plan_name: string;
  price: number;
  currency: string;
  granted: number | null;
  rate: number | null;
  raw: string;
};

type Topup = {
  endpoint_id: string;
  granted: number;
  matched_plan: string;
  price: number | null;
  currency: string;
  at: string;
};

type Payload = {
  rows: Row[];
  changes: Change[];
  topup_plans: Plan[];
  topups: Topup[];
};

/**
 * 处置文案。
 *
 * `drop` 和 `profitable` 是**两件不同的事**，不能合并成一句：前者是降价、根本没触发
 * 毛利重算；后者是涨了但重算过、仍然赚钱。合成一句的话，一行降价会被标成
 * 「重算后仍在赚钱」—— 一句声称做过、实际没做的事。这一版就是来修这个的。
 */
const ACTED: Record<string, string> = {
  drop: "降价（不触发重算）",
  none: "已记录",
  profitable: "涨价，重算后仍在赚钱",
  alarm: "已告警（这个出口的看守关着）",
  disabled: "已自动停用（重算后亏本）",
  // 这一条比 disabled 更急：钱还在流，而且不会自动重试。
  disable_failed: "⚠ 亏本但停用失败，线路仍在接单",
  // 上一版的判据留着能显示 —— 历史记录里还有这个值，映射不到会显示成生词。
  kept_last_route: "旧判据：因是最后一条线路而没停",
};

/**
 * 一页几行。
 *
 * 两张表分开翻页：适配器状态是「一行一个出口」，量级几十；价格异动是流水，
 * 会一直长。共用一个页码的话，切个标签页就跳到一张空表上。
 *
 * 25 而不是 10：10 行只占屏幕上半截，下面空一大片，而 60 条异动被切成 6 页 ——
 * 要翻 5 次才看得完一天的价格变化。副作用是「适配器状态」只剩一页，翻页条自己
 * 就不显示了（`pages <= 1` 直接 return null），那正是它该有的样子：10 个出口
 * 本来就不需要翻页。
 */
const PAGE_SIZE = 25;

/**
 * 翻页条。
 *
 * 用函数式更新（`p => p+1`）而不是 `onPage(page+1)`：连点两下时，两次计算会读到
 * 同一个 page，于是点两下只前进一页。
 */
function Pager({
  page,
  pages,
  total,
  unit,
  onPage,
}: {
  page: number;
  pages: number;
  total: number;
  unit: string;
  onPage: (f: (p: number) => number) => void;
}) {
  if (pages <= 1) return null;
  return (
    // 水平居中：翻页条是这张表的收尾，不是左栏的一部分。靠左放时它贴在
    // 「时间」那一列下面，看起来像又一行数据。
    <div className="flex items-center justify-center gap-2 border-t border-border px-5 py-3 text-xs text-muted-foreground">
      <Button size="sm" variant="outline" disabled={page <= 1} onClick={() => onPage((p) => Math.max(1, p - 1))}>
        <ChevronLeft className="h-3.5 w-3.5" /> 上一页
      </Button>
      <span className="tabular-nums">
        第 {page} / {pages} 页 · 共 {total} {unit}
      </span>
      <Button
        size="sm"
        variant="outline"
        disabled={page >= pages}
        onClick={() => onPage((p) => Math.min(pages, p + 1))}
      >
        下一页 <ChevronRight className="h-3.5 w-3.5" />
      </Button>
    </div>
  );
}

/**
 * 两个入口，一个组件。
 *
 * 值就是 NavKey 本身，不另起一套 "status" / "changes" —— 多一层映射就多一处
 * 能对不上的地方，而这个组件唯一的分支就是「侧栏点的是哪一项」。
 */
export type AdapterView = "routing-adapters" | "routing-adapters-changes";

export function Adapters({ view }: { view: AdapterView }) {
  const isStatus = view === "routing-adapters";
  const [data, setData] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [statusPage, setStatusPage] = useState(1);
  const [changePage, setChangePage] = useState(1);

  const load = useCallback(async () => {
    try {
      setData(await api.get<Payload>("/api/admin/relay-adapters"));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // 重新拉数据 = 换了一份数据，停在第 5 页多半是空的。两张表都回第一页。
  // 两个页码本身是独立的：切标签页不该把另一张表的位置也重置掉。
  useEffect(() => {
    if (data) {
      setStatusPage(1);
      setChangePage(1);
    }
  }, [data]);

  async function resync() {
    setBusy(true);
    try {
      await api.post("/api/admin/relay-adapters/sync", {});
      // 同步是后台跑的（十几个上游、几十秒），这里等一下再拉一次结果。
      // 不等的话立刻拉到的还是旧数据，看起来像「点了没反应」。
      setTimeout(() => void load(), 6000);
    } catch (e) {
      setError(e instanceof Error ? e.message : "同步失败");
    }
    setBusy(false);
  }

  async function toggleGuard(r: Row) {
    try {
      await api.post("/api/admin/relay-adapters/guard", {
        endpoint_id: r.endpoint_id,
        auto_guard: !r.auto_guard,
      });
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "改不动看守开关");
    }
  }

  const rows = data?.rows ?? [];
  // 顶部四个数**永远按全量算**，不跟着翻页变 —— 那是这一页的总账，
  // 翻到第二页就变一次的话，它就不是总账了。
  const known = rows.filter((r) => r.family && r.family !== "未知").length;
  // 「能真实记账」数的是**抓到真价**的那些。抓不到的现在也有成本（推算），
  // 但把两者合成一个数就分不出「实测」和「推的」—— 那正是这一列存在的意义。
  const ready = rows.filter((r) => r.accounting_ready).length;
  const blocked = rows.filter((r) => r.blocked_reason).length;

  const changes = data?.changes ?? [];
  const statusPages = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  const changePages = Math.max(1, Math.ceil(changes.length / PAGE_SIZE));
  // 数据变短时夹住，别显示空表。
  const sPage = Math.min(statusPage, statusPages);
  const cPage = Math.min(changePage, changePages);
  const shownRows = rows.slice((sPage - 1) * PAGE_SIZE, sPage * PAGE_SIZE);
  const shownChanges = changes.slice((cPage - 1) * PAGE_SIZE, cPage * PAGE_SIZE);

  // 价格异动只给出口 id，没有名字。没有这张对照表的话，同一个模型在不同出口上的
  // 涨跌看起来就是一堆重复行 —— 而它们其实是不同出口各自的价。
  const labelOf = new Map(rows.map((r) => [r.endpoint_id, r.label]));

  return (
    <div className="space-y-4">
      <PageHeader
        title={isStatus ? "网关适配器" : "价格异动"}
        description={
          isStatus
            ? "认出每家中转跑的是什么软件，自动把它的真实进价和余额拉过来。"
            : "每一轮同步都和上一轮比，涨到吃掉毛利的会被自动停用。"
        }
        actions={
          <Button size="sm" variant="outline" onClick={() => void resync()} disabled={busy}>
            <RefreshCw className={cn("mr-1.5 h-3.5 w-3.5", busy && "animate-spin")} />
            立刻同步
          </Button>
        }
      />

      {error && <ErrorState message={error} onRetry={() => void load()} />}

      <div className={cn("grid gap-3 sm:grid-cols-2 lg:grid-cols-4", !isStatus && "hidden")}>
        <Stat label="认出来的" value={`${known} / ${rows.length}`} hint="识别出软件家族" />
        <Stat
          label="能真实记账"
          value={`${ready} / ${rows.length}`}
          hint={ready === rows.length ? "全部" : "其余成本只能手工录"}
        />
        <Stat label="被自动停用" value={blocked} hint={blocked ? "见下方原因" : "没有"} />
        <Stat label="价格异动" value={data?.changes.length ?? 0} hint="最近记录" />
      </div>

      {!data && !error && <TableSkeleton rows={5} />}

      {data && isStatus && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <p className="text-[13px] text-muted-foreground">
                「靠什么认的」这一列不是装饰：<b>认错比认不出更糟</b>，因为认错会拉到一份
                别家的价目还很自信。这一列让你能一眼判断结论可不可信。
                <br />
                亏本看守的判据是<b>按新进价把最近 7 天的真实用量重算一遍</b>，不是涨幅百分比——
                涨 200% 但仍有 10 倍毛利的不会被停，涨 20% 就翻负的会被停。
              </p>
            </CardHeader>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>线路 / 出口</TableHead>
                    <TableHead>识别为</TableHead>
                    <TableHead>靠什么认的</TableHead>
                    <TableHead className="numeric">自动进价</TableHead>
                    <TableHead>余额</TableHead>
                    <TableHead>能否真实记账</TableHead>
                    <TableHead>亏本看守</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {shownRows.map((r) => (
                    <TableRow
                      key={r.endpoint_id}
                      className={cn(r.blocked_reason && "bg-destructive/5")}
                    >
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <VendorMark vendor={r.vendor} className="h-4 w-4 shrink-0" />
                          <div className="min-w-0">
                            <Truncate title={r.label}>{r.label}</Truncate>
                            <p className="text-[11px] text-muted-foreground">
                              <Truncate title={r.base_url}>{r.base_url}</Truncate>
                            </p>
                          </div>
                        </div>
                      </TableCell>
                      <TableCell>
                        {r.family ? (
                          <Badge variant={r.family === "未知" ? "outline" : "secondary"}>
                            {r.family}
                          </Badge>
                        ) : (
                          <span className="text-[12px] text-muted-foreground">还没同步</span>
                        )}
                        {r.quota_per_unit ? (
                          <p className="mt-0.5 text-[11px] text-muted-foreground">
                            额度单位 {r.quota_per_unit.toLocaleString()} = $1
                          </p>
                        ) : null}
                      </TableCell>
                      <TableCell className="max-w-[16rem]">
                        <span className="text-[12px] text-muted-foreground">
                          {r.matched_by || "—"}
                        </span>
                      </TableCell>
                      <TableCell className="numeric">
                        {r.priced_models > 0 ? (
                          <span className="font-medium text-emerald-600">{r.priced_models} 条</span>
                        ) : (
                          <span className="text-muted-foreground">—</span>
                        )}
                      </TableCell>
                      <TableCell>
                        {r.balance_ok ? (
                          <span className="text-[12px]">{r.balance_text}</span>
                        ) : (
                          <span className="text-[12px] text-muted-foreground">查不到</span>
                        )}
                      </TableCell>
                      <TableCell className="max-w-[20rem]">
                        {r.accounting_ready ? (
                          <span className="inline-flex items-center gap-1 text-[12px] text-emerald-600">
                            <CheckCircle2 className="h-3.5 w-3.5" /> 可以 · 抓到真价
                          </span>
                        ) : (
                          <div className="flex items-start gap-1.5">
                            {/*
                              抓不到价**不再等于算不出成本**：对账会按 OpenRouter 官方价 ×
                              这个出口的倍率推算。所以这里不能再画一个红叉配「只能手工录」——
                              那句话现在是假的，而一句过期的待办比没有待办更糟。
                              标黄不标红：有数，但不是实测的。
                            */}
                            <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" />
                            <span className="text-[12px] text-muted-foreground">
                              <b className="text-amber-600">推算</b>
                              {" · "}
                              {r.note || "拉不到价目，对账按 OpenRouter 官方价 × 倍率推算"}
                            </span>
                          </div>
                        )}
                        {r.blocked_reason && (
                          <p className="mt-1 flex items-start gap-1.5 text-[12px] text-destructive">
                            <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                            {r.blocked_reason}
                          </p>
                        )}
                      </TableCell>
                      <TableCell>
                        <Button
                          size="sm"
                          variant={r.auto_guard ? "default" : "outline"}
                          onClick={() => void toggleGuard(r)}
                        >
                          {r.auto_guard ? (
                            <>
                              <ShieldCheck className="mr-1 h-3.5 w-3.5" /> 开
                            </>
                          ) : (
                            <>
                              <ShieldAlert className="mr-1 h-3.5 w-3.5" /> 关
                            </>
                          )}
                        </Button>
                        <p className="mt-0.5 text-[11px] text-muted-foreground">
                          {r.auto_guard
                            ? `毛利低于 ${r.margin_floor_pct ?? 0}% 就停`
                            : "不自动停"}
                        </p>
                        <p className="text-[11px] text-muted-foreground">
                          {r.synced_at ? when(r.synced_at) : "未同步"}
                        </p>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
            <Pager
              page={sPage}
              pages={statusPages}
              total={rows.length}
              unit="个出口"
              onPage={setStatusPage}
            />
          </Card>
        </SectionReveal>
      )}

      {data && isStatus && (data.topup_plans.length > 0 || data.topups.length > 0) && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <h3 className="text-sm font-medium">充值比例</h3>
              <p className="text-[13px] text-muted-foreground">
                <b>这不是一个汇率，是逐套餐的。</b>实测这几家中转的前端整个没有汇率常量——
                每档充值各自定价，¥50 一档和 ¥200 一档的到账金额常常不成比例。所以能取的只有
                套餐表，取一个「平均汇率」会把这件事抹平。
                <br />
                套餐表要控制台令牌才拉得到；没有令牌时靠<b>余额跳升</b>兜底——那能拿到到账金额，
                但拿不到你付了多少，除非它对得上某一档套餐。
              </p>
            </CardHeader>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>来源</TableHead>
                    <TableHead>套餐 / 时间</TableHead>
                    <TableHead className="numeric">付款</TableHead>
                    <TableHead className="numeric">到账</TableHead>
                    <TableHead className="numeric">1 元买到</TableHead>
                    <TableHead>说明</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.topup_plans.map((p) => (
                    <TableRow key={`plan-${p.endpoint_id}-${p.plan_key}`}>
                      <TableCell>
                        <Badge variant="secondary">套餐表</Badge>
                      </TableCell>
                      <TableCell className="text-[13px]">
                        <Truncate title={p.plan_name || p.plan_key}>
                          {p.plan_name || p.plan_key}
                        </Truncate>
                      </TableCell>
                      <TableCell className="numeric">
                        {p.price.toFixed(2)} {p.currency}
                      </TableCell>
                      <TableCell className="numeric">
                        {p.granted === null ? "—" : `$${p.granted.toFixed(4)}`}
                      </TableCell>
                      <TableCell className="numeric font-medium">
                        {p.rate === null ? "—" : `$${p.rate.toFixed(4)}`}
                      </TableCell>
                      <TableCell className="text-[12px] text-muted-foreground">
                        {/* 认不出到账金额时把原文摆出来：「字段名不一样」和「这档不送余额」
                            在结果上都是横杠，在处理上完全不同。 */}
                        {p.granted === null && p.raw
                          ? `认不出到账字段：${p.raw.slice(0, 60)}`
                          : ""}
                      </TableCell>
                    </TableRow>
                  ))}
                  {data.topups.map((t2, i) => (
                    <TableRow key={`topup-${t2.endpoint_id}-${i}`}>
                      <TableCell>
                        <Badge variant="outline">余额跳升</Badge>
                      </TableCell>
                      <TableCell className="text-[13px] text-muted-foreground">
                        {when(t2.at)}
                      </TableCell>
                      <TableCell className="numeric">
                        {t2.price === null ? "—" : `${t2.price.toFixed(2)} ${t2.currency}`}
                      </TableCell>
                      <TableCell className="numeric">${t2.granted.toFixed(4)}</TableCell>
                      <TableCell className="numeric font-medium">
                        {t2.price === null || t2.price <= 0
                          ? "—"
                          : `$${(t2.granted / t2.price).toFixed(4)}`}
                      </TableCell>
                      <TableCell className="text-[12px] text-muted-foreground">
                        {t2.matched_plan
                          ? `匹配到套餐 ${t2.matched_plan}`
                          : "没匹配上任何套餐——多半是站外充值，付款金额要你自己补"}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          </Card>
        </SectionReveal>
      )}

      {data && !isStatus && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              {/* 标题在 PageHeader 上，这儿不再重复一遍 —— 只留判据。 */}
              <p className="text-[13px] text-muted-foreground">
                每一轮同步都和上一轮比。<b>只存当前价的话，涨价这件事在数据里根本不存在</b>——
                你只会看到一个新的价，而它看起来和一直是这个价没有区别。
              </p>
            </CardHeader>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>时间</TableHead>
                    <TableHead>出口</TableHead>
                    <TableHead>模型</TableHead>
                    <TableHead className="numeric">输入价</TableHead>
                    <TableHead className="numeric">输出价</TableHead>
                    <TableHead className="numeric">涨幅</TableHead>
                    <TableHead>处置</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {shownChanges.map((c, i) => (
                    <TableRow key={`${c.endpoint_id}-${c.model_id}-${i}`}>
                      <TableCell className="text-[12px] text-muted-foreground">
                        {when(c.at)}
                      </TableCell>
                      {/*
                        没有这一列的话，同一个模型在不同出口上的涨跌看起来就是一堆
                        重复行 —— 而它们其实是不同出口各自的价（同一个模型名在
                        自带地址、新挂的出口、不同分组下都有一份）。
                      */}
                      <TableCell className="text-[12px]">
                        <Truncate title={labelOf.get(c.endpoint_id) ?? c.endpoint_id}>
                          {labelOf.get(c.endpoint_id) ?? "（已删除的出口）"}
                        </Truncate>
                      </TableCell>
                      <TableCell className="font-mono text-[12px]">
                        <Truncate title={c.model_id}>{c.model_id}</Truncate>
                      </TableCell>
                      <TableCell className="numeric text-[12px]">
                        {c.old_input?.toFixed(3)} → {c.new_input?.toFixed(3)}
                      </TableCell>
                      <TableCell className="numeric text-[12px]">
                        {c.old_output?.toFixed(3)} → {c.new_output?.toFixed(3)}
                      </TableCell>
                      <TableCell className="numeric">
                        <span
                          className={cn(
                            "font-medium",
                            c.pct > 0 ? "text-destructive" : "text-emerald-600",
                          )}
                        >
                          {c.pct > 0 ? "+" : ""}
                          {c.pct.toFixed(0)}%
                        </span>
                      </TableCell>
                      <TableCell className="text-[12px]">
                        {ACTED[c.acted] ?? c.acted}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
            <Pager
              page={cPage}
              pages={changePages}
              total={changes.length}
              unit="条"
              onPage={setChangePage}
            />
          </Card>
        </SectionReveal>
      )}
    </div>
  );
}
