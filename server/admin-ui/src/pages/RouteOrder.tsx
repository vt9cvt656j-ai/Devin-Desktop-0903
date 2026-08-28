import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, ArrowDown, ArrowUp, RotateCcw, Save } from "lucide-react";

import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { TableSkeleton } from "@/components/TableSkeleton";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Truncate } from "@/components/ui/table";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * 线路排序 —— 谁排在前面。
 *
 * # 这不是一个显示开关
 *
 * 服务端到处都是 `ORDER BY sort, created_at`，而挑主线路取的是 `candidates.first()`。
 * 也就是说：**同一个模型被两条线路开放时，排在前面的那条接单，用户按它的倍率付钱。**
 * 只当成「选择器里的先后」来改，会静默改变计费而界面上看不出来。
 *
 * 所以这一屏必须把「哪些模型真的会因此换线」当场算出来摆在旁边 —— 不摆的话，
 * 一次拖动的后果要等到月底对账才看得见。
 *
 * # 为什么给的是序号输入框而不是拖拽
 *
 * 用户点名要「输入序号」。而且九条线路里把第 7 条挪到第 2 位，输数字比拖两屏准。
 * 上下箭头也留着 —— 相邻交换用它更快。
 */
type Conn = {
  id: string;
  label?: string;
  base_url?: string;
  provider?: string;
  active?: boolean;
  sort?: number;
  rate?: number;
  power_route?: boolean;
  enabled_models?: string[];
  /** 派单实际认的模型集合（含出口自带的货）。服务端算的，别在前端重算。 */
  effective_models?: string[];
  created_at?: string;
};

export function RouteOrder() {
  const [conns, setConns] = useState<Conn[] | null>(null);
  const [order, setOrder] = useState<Conn[]>([]);
  const [nums, setNums] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);

  const load = useCallback(async () => {
    try {
      const r = await api.get<Conn[] | { items?: Conn[] }>("/api/admin/models");
      const list = Array.isArray(r) ? r : (r.items ?? []);
      // 服务端已经按 sort, created_at 回好了 —— 这一屏的初始次序就是**线上真正在用的次序**，
      // 不另外排一遍。另排一遍就会出现「界面上第一条」和「真正接单的第一条」不是同一条。
      setConns(list);
      setOrder(list);
      setNums(Object.fromEntries(list.map((c, i) => [c.id, String(i + 1)])));
      setError(null);
      setSaved(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  /** 按输入框里的序号重排。没填/填重了的按当前位置兜底，绝不把谁弄丢。 */
  function applyNumbers() {
    const next = [...order]
      .map((c, i) => {
        const raw = (nums[c.id] ?? "").trim();
        const n = Number(raw);
        return { c, key: Number.isFinite(n) && raw !== "" ? n : i + 1, tie: i };
      })
      .sort((a, b) => a.key - b.key || a.tie - b.tie)
      .map((x) => x.c);
    setOrder(next);
    setNums(Object.fromEntries(next.map((c, i) => [c.id, String(i + 1)])));
  }

  function move(i: number, d: -1 | 1) {
    const j = i + d;
    if (j < 0 || j >= order.length) return;
    const next = [...order];
    [next[i], next[j]] = [next[j], next[i]];
    setOrder(next);
    setNums(Object.fromEntries(next.map((c, k) => [c.id, String(k + 1)])));
  }

  async function save() {
    setBusy(true);
    try {
      await api.post("/api/admin/models/sort", {
        // 从 10 开始、每档 10：以后想在两条之间插一条，不用把整张表重排一遍。
        order: order.map((c, i) => ({ id: c.id, sort: (i + 1) * 10 })),
      });
      setSaved(true);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "保存失败");
    }
    setBusy(false);
  }

  /*
   * 哪些模型**真的会因为次序换线**。
   *
   * 判据不是「有几条线路开放了它」，而是「有几条**普通**线路开放了它」：
   * 强力线路已经被服务端从普通请求里排除掉了（power_route 那道闸），
   * 把它算进来会报出一堆假的冲突，然后真正的冲突淹没在里面。
   */
  const live = order.filter((c) => c.active !== false);
  const plain = live.filter((c) => !c.power_route);
  const owners = new Map<string, Conn[]>();
  for (const c of plain) {
    // 用**服务端算的派单集合**，不是线路自己声明的 enabled_models ——
    // 出口可以带线路本身没有的货，少算就会得出「没有模型会换线」，
    // 而这一屏存在的理由正是「排序会静默改变用户按谁的倍率付钱」。
    for (const m of c.effective_models ?? c.enabled_models ?? []) {
      owners.set(m, [...(owners.get(m) ?? []), c]);
    }
  }
  const contested = [...owners.entries()]
    .filter(([, cs]) => cs.length > 1)
    .map(([model, cs]) => ({
      model,
      // 现在这个次序下谁接单 = 排在最前面的那条。
      winner: cs.reduce((a, b) => (order.indexOf(a) <= order.indexOf(b) ? a : b)),
      all: cs,
    }))
    .sort((a, b) => a.model.localeCompare(b.model));

  const dirty = order.some((c, i) => conns?.[i]?.id !== c.id);

  return (
    <div className="space-y-4">
      <PageHeader
        title="线路排序"
        description="谁排在前面。同一个模型被两条线路开放时，排在前面的那条接单。"
        actions={
          <div className="flex gap-2">
            <Button size="sm" variant="outline" onClick={() => void load()} disabled={busy}>
              <RotateCcw className="mr-1.5 h-3.5 w-3.5" /> 还原
            </Button>
            <Button size="sm" onClick={() => void save()} disabled={busy || !dirty}>
              <Save className="mr-1.5 h-3.5 w-3.5" />
              {busy ? "保存中…" : dirty ? "保存顺序" : "没有改动"}
            </Button>
          </div>
        }
      />

      {error && <ErrorState message={error} onRetry={() => void load()} />}
      {saved && !dirty && (
        <p className="rounded-lg border border-success/30 bg-success/5 px-4 py-2 text-sm text-success">
          顺序已保存，立刻生效。
        </p>
      )}

      {!conns && !error && <TableSkeleton rows={6} />}

      {conns && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <p className="text-[13px] text-muted-foreground">
                在左边的框里改序号，然后点<b>「按序号重排」</b>；相邻两条对调用右边的上下箭头更快。
                改完要点右上角<b>保存顺序</b>才写进去。
                <br />
                <b>这个次序不只是选择器里的先后。</b>服务端挑主线路取的是排在最前面的那条，
                所以<b>同一个模型被两条线路开放时，是这个次序决定用户按谁的倍率付钱</b>。
                下面「会换线的模型」那一块列的就是真正受影响的那几个 —— 没列出来的模型
                只有一条线路提供，怎么排都不影响它。
              </p>
            </CardHeader>

            <div className="divide-y divide-border">
              {order.map((c, i) => (
                <div
                  key={c.id}
                  className={cn(
                    "flex flex-wrap items-center gap-3 px-5 py-3",
                    c.active === false && "opacity-50",
                  )}
                >
                  <Input
                    className="h-8 w-14 shrink-0 text-center text-xs"
                    value={nums[c.id] ?? String(i + 1)}
                    onChange={(ev) => setNums({ ...nums, [c.id]: ev.target.value })}
                    onKeyDown={(ev) => {
                      if (ev.key === "Enter") applyNumbers();
                    }}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-sm font-medium">{c.label || "未命名"}</span>
                      {c.active === false && <Badge variant="outline">已停用</Badge>}
                      {c.power_route && (
                        <Badge variant="outline" title="强力线路不接普通请求，次序对它没有计费影响">
                          强力版
                        </Badge>
                      )}
                      <span className="text-[11px] text-muted-foreground">
                        按 Token ×{c.rate ?? 1} · {(c.effective_models ?? c.enabled_models ?? []).length} 个模型
                      </span>
                    </div>
                    <Truncate className="font-mono text-[11px] text-muted-foreground" title={c.base_url}>
                      {c.base_url || "—"}
                    </Truncate>
                  </div>
                  <div className="flex shrink-0 gap-1">
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label="上移"
                      disabled={i === 0}
                      onClick={() => move(i, -1)}
                    >
                      <ArrowUp className="h-3.5 w-3.5" />
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label="下移"
                      disabled={i === order.length - 1}
                      onClick={() => move(i, 1)}
                    >
                      <ArrowDown className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>

            <div className="border-t border-border px-5 py-3">
              <Button size="sm" variant="outline" onClick={applyNumbers}>
                按序号重排
              </Button>
              <span className="ml-2 text-[11px] text-muted-foreground">
                序号填重了或者没填的，按它当前的位置兜底 —— 不会把谁弄丢。
              </span>
            </div>
          </Card>
        </SectionReveal>
      )}

      {conns && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <h3 className="text-sm font-medium">
                会换线的模型
                <span className="ml-2 text-xs font-normal text-muted-foreground">
                  {contested.length} 个 —— 只有这些的计费会跟着次序变
                </span>
              </h3>
              <p className="text-[13px] text-muted-foreground">
                这些模型有<b>两条以上普通线路</b>都开放了。排在最前面的那条接单，用户就按它的
                倍率付钱。强力线路不算在内 —— 它已经被排除在普通请求之外，次序对它没有影响。
              </p>
            </CardHeader>
            {contested.length === 0 ? (
              <p className="px-5 pb-4 text-sm text-muted-foreground">
                没有。每个模型都只有一条普通线路提供，所以这个次序现在<b>纯粹是显示顺序</b>，
                怎么排都不会改变谁接单、按谁计费。
              </p>
            ) : (
              <ul className="divide-y divide-border">
                {contested.map((x) => (
                  <li key={x.model} className="flex flex-wrap items-center gap-x-3 gap-y-1 px-5 py-2 text-[12px]">
                    <span className="font-mono">{x.model}</span>
                    <span className="text-muted-foreground">
                      {x.all.length} 条线路都开放了
                    </span>
                    <span className="ml-auto">
                      现在走{" "}
                      <b className="text-success">{x.winner.label || "未命名"}</b>
                      <span className="text-muted-foreground">
                        {" "}
                        · 按 Token ×{x.winner.rate ?? 1}
                      </span>
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </Card>
        </SectionReveal>
      )}

      {conns && contested.length > 0 && (
        <p className="flex items-start gap-1.5 px-1 text-xs leading-relaxed text-muted-foreground">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-warning" />
          <span>
            改次序之前先看一眼上面那一块：把一条倍率高的线路挪到前面，这几个模型的用户
            账单会当场变贵，而账单上不会写「因为排序变了」。
          </span>
        </p>
      )}
    </div>
  );
}
