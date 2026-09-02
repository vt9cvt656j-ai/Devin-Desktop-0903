import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, CheckCircle2, HelpCircle, Scale } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Truncate } from "@/components/ui/table";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * 同步倍率 —— 把中转公开价目里的分组倍率对到我们填的进价倍率上。
 *
 * # 为什么这里是「对照表」而不是「一键写完」
 *
 * 用户要的是「点一下全部同步真实倍率，我不会亏钱」。而 2026-08-26 实测下来，
 * 「真实倍率」的两个来源都不能无条件相信：
 *
 *   · **公开价目的分组倍率** —— 私有 / 自建分组根本不在公开价目里。线上 Claude 那条的
 *     key 在 `CCMAX（自建）1x`，而公开价目里那几个模型只出现在 `claude_kiro 0.07x`。
 *     照抄就是把成本算成十四分之一 —— **正好是「让你以为在赚钱」的方向**。
 *
 *   · **余额反推** —— 最硬的执行事实，但它要求 token 记录完整，而按模型记账
 *     2026-08-26 05:48 才修好，在那之前分母残缺，反推出 1.58 倍（比任何分组都高）。
 *
 * 所以这一屏摆证据、标可信度，可信的默认勾上，剩下的让人自己判断。
 * **少填一个倍率最坏是排序不准；填错一个倍率是账目错到反向。**
 */
type Row = {
  endpoint_id: string;
  route_label: string;
  outlet_label: string;
  host: string;
  is_own: boolean;
  current: number;
  from_catalog: number | null;
  groups: string[];
  matched_models: number;
  total_models: number;
  confidence: "ok" | "partial" | "ambiguous" | "none";
  reason: string;
};

const num = (v: number) => Number(v.toFixed(4));

export function RatioSync({ open, onClose, onApplied }: {
  open: boolean;
  onClose: () => void;
  onApplied: () => void;
}) {
  const [rows, setRows] = useState<Row[] | null>(null);
  const [picked, setPicked] = useState<Record<string, boolean>>({});
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<number | null>(null);

  const load = useCallback(async () => {
    setRows(null);
    setDone(null);
    try {
      const r = await api.get<{ rows: Row[] }>("/api/admin/ratio-sync");
      setRows(r.rows);
      // **只有 `ok` 默认勾上。** partial / ambiguous 是「有数但说不准」，
      // 默认帮人写进去就等于替他做了一个可能反向的决定。
      setPicked(
        Object.fromEntries(
          r.rows
            .filter(
              (x) =>
                !x.is_own &&
                x.confidence === "ok" &&
                x.from_catalog != null &&
                Math.abs(x.from_catalog - x.current) > 1e-9,
            )
            .map((x) => [x.endpoint_id, true]),
        ),
      );
      setErr(null);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "读不到");
    }
  }, []);

  useEffect(() => {
    if (open) void load();
  }, [open, load]);

  const list = rows ?? [];
  const chosen = list.filter((r) => picked[r.endpoint_id] && r.from_catalog != null);

  async function apply() {
    setBusy(true);
    try {
      const r = await api.post<{ changed: number }>("/api/admin/ratio-sync", {
        items: chosen.map((x) => ({ endpoint_id: x.endpoint_id, ratio: x.from_catalog })),
      });
      setDone(r.changed);
      onApplied();
      await load();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "保存失败");
    }
    setBusy(false);
  }

  const mark = (c: Row["confidence"]) =>
    c === "ok" ? (
      <Badge variant="success" className="whitespace-nowrap">
        <CheckCircle2 className="h-3 w-3" /> 可信
      </Badge>
    ) : c === "none" ? (
      <Badge variant="outline" className="whitespace-nowrap border-border">
        <HelpCircle className="h-3 w-3" /> 看不到
      </Badge>
    ) : (
      <Badge variant="outline" className="whitespace-nowrap border-warning/40 text-warning">
        <AlertTriangle className="h-3 w-3" /> {c === "partial" ? "只对上一部分" : "有歧义"}
      </Badge>
    );

  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="max-w-5xl gap-4 p-5">
        <DialogHeader>
          <DialogTitle>同步倍率</DialogTitle>
          <DialogDescription>
            拿中转<b>公开价目里的分组倍率</b>对一遍我们填的进价倍率。
            <b className="text-foreground">不会替你做主</b>——只有「这条线路开放的模型
            全部落在同一个分组里」才算可信、默认勾上。
          </DialogDescription>
        </DialogHeader>

        <div className="rounded-lg border border-warning/40 bg-warning/5 px-3 py-2 text-[12px] leading-relaxed">
          <b>私有 / 自建分组中转不公开，这里就看不到。</b>
          线上就有这个形状：Claude 那条的 key 在「CCMAX（自建）1x」，而公开价目里那几个模型
          只出现在「claude_kiro 0.07x」——照抄会把成本算成十四分之一，
          <b>正好是让你以为在赚钱的方向</b>。所以标着「看不到」的那些请照中转后台手填。
        </div>

        {err && <p className="text-sm text-destructive">{err}</p>}
        {done !== null && (
          <p className="rounded-lg border border-success/30 bg-success/5 px-3 py-2 text-sm text-success">
            已同步 {done} 条，立刻生效。
          </p>
        )}

        {!rows && !err && <p className="py-8 text-center text-sm text-muted-foreground">读取中…</p>}

        {rows && (
          <div className="max-h-[52vh] overflow-y-auto rounded-lg border border-border">
            <table className="w-full text-[12px]">
              <thead className="sticky top-0 bg-muted text-[11px] text-muted-foreground">
                <tr>
                  <th className="w-10 px-3 py-1.5"></th>
                  <th className="px-2 py-1.5 text-left font-normal">线路 / 出口</th>
                  <th className="w-20 px-2 py-1.5 text-right font-normal">现在</th>
                  <th className="w-20 px-2 py-1.5 text-right font-normal">价目里的</th>
                  <th className="w-28 px-2 py-1.5 text-left font-normal">可信度</th>
                  <th className="px-3 py-1.5 text-left font-normal">依据</th>
                </tr>
              </thead>
              <tbody>
                {list.map((r) => {
                  const same =
                    r.from_catalog != null && Math.abs(r.from_catalog - r.current) < 1e-9;
                  // 自带地址的倍率恒为 1（「我按官方价进货」的定义），改不了。
                  const locked = r.is_own || r.from_catalog == null || same;
                  return (
                    <tr key={r.endpoint_id} className="border-t border-border/60">
                      <td className="px-3 py-2">
                        <input
                          type="checkbox"
                          disabled={locked}
                          checked={!!picked[r.endpoint_id]}
                          onChange={(e) =>
                            setPicked({ ...picked, [r.endpoint_id]: e.target.checked })
                          }
                        />
                      </td>
                      <td className="px-2 py-2">
                        <div>{r.route_label}</div>
                        <div className="text-[11px] text-muted-foreground">
                          {r.outlet_label} · <span className="font-mono">{r.host}</span>
                          {r.is_own && " · 恒为 1，改不了"}
                        </div>
                      </td>
                      <td className="px-2 py-2 text-right tabular-nums">{num(r.current)}×</td>
                      <td
                        className={cn(
                          "px-2 py-2 text-right tabular-nums",
                          !same && r.from_catalog != null && "font-semibold text-foreground",
                        )}
                      >
                        {r.from_catalog == null ? "—" : `${num(r.from_catalog)}×`}
                        {same && (
                          <span className="block text-[10px] text-muted-foreground">一致</span>
                        )}
                      </td>
                      <td className="px-2 py-2">{mark(r.confidence)}</td>
                      <td className="px-3 py-2 text-[11px] text-muted-foreground">
                        <Truncate title={r.reason}>{r.reason}</Truncate>
                        {r.matched_models > 0 && (
                          <span className="block">
                            命中 {r.matched_models}/{r.total_models} 个模型
                          </span>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}

        <div className="flex flex-wrap items-center justify-between gap-3 border-t border-border pt-4">
          <span className="text-[12px] text-muted-foreground">
            <Scale className="mr-1 inline h-3.5 w-3.5" />
            少填一个倍率最坏是排序不准；<b>填错一个是账目错到反向</b>，所以拿不准的没有默认勾上。
          </span>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={onClose}>
              关闭
            </Button>
            <Button disabled={busy || chosen.length === 0} onClick={() => void apply()}>
              {busy ? "写入中…" : `同步选中的 ${chosen.length} 条`}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
