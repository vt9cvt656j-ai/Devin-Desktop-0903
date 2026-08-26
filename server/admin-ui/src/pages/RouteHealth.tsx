import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, Mail, PauseCircle, RefreshCw, Timer, Wallet } from "lucide-react";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { VendorMark } from "@/components/VendorMark";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Truncate } from "@/components/ui/table";
import { api } from "@/lib/api";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * 健康 —— 每个上游此刻是什么样。
 *
 * # 这一页要回答的是「现在能不能用、为什么、我该干什么」
 *
 * 别的页面回答「怎么配」，这一页只回答「出事了吗」。所以它把四个来源摆在同一行上：
 *
 *   · **调度状态** —— 系统此刻拿它怎么办（正常 / 限流让位 / 下架）
 *   · **真实流量** —— 用户的请求在它身上的结局（连败几次、上次成功多久前）
 *   · **主动探测** —— 我们自己敲门的结果（通不通、多少毫秒、失败原因）
 *   · **用量和余额** —— 它花了多少、还剩多少
 *
 * 这四个是**不同的东西**，经常互相矛盾，而矛盾本身就是信息：探测通但真实流量连败，
 * 说明问题出在某类请求上而不是这条线本身；真实流量正常但余额见底，说明该充值了
 * 但还没爆。合成一个「健康度」百分比会把这些全抹平。
 *
 * # 余额要点一下才查
 *
 * 各家中转的余额接口互不相同，查一遍是好几个网络往返。默认不查，按钮点了才查——
 * 不然每次刷新这一页都要等几秒，而多数时候你只是来看一眼状态。
 */

type Row = {
  endpoint_id: string;
  route_id: string;
  route_label: string;
  vendor: string;
  label: string;
  base_url: string;
  is_own: boolean;
  active: boolean;
  cost_ratio: number;
  capacity: number | null;
  sched: string;
  retry_in: number | null;
  live: string;
  consecutive_failures: number;
  last_ok_secs_ago: number | null;
  probe_ok: boolean | null;
  probe_ms: number | null;
  probe_note: string;
  calls_today: number;
  cost_today_usd: number;
  calls_7d: number;
  cost_7d_usd: number;
  cached_tokens_7d: number;
  balance: string | null;
};

type Body = {
  rows: Row[];
  // 全部可选：后端换版、部分失败、或者中间层塞了个别的东西，这里就会拿到不完整的形状。
  // 一个 `body.alarm.usable` 不加兜底，整个控制台会白屏 —— 而白屏时你连「哪一页坏了」
  // 都看不出来。这一页的用途恰恰是出事时来看，它自己不能是最脆的那个。
  alarm?: { usable?: number; total?: number };
  balance_included?: boolean;
};

function ago(secs: number | null): string {
  if (secs == null) return "从没成功过";
  if (secs < 90) return `${secs} 秒前`;
  if (secs < 5400) return `${Math.round(secs / 60)} 分钟前`;
  if (secs < 172800) return `${Math.round(secs / 3600)} 小时前`;
  return `${Math.round(secs / 86400)} 天前`;
}

function usd(v: number): string {
  if (v === 0) return "—";
  return v < 0.01 ? `<$0.01` : `$${v.toFixed(2)}`;
}

/** 调度器此刻拿它怎么办。三种理由分开显示——恢复动作完全不同。 */
function Sched({ sched, retryIn }: { sched: string; retryIn: number | null }) {
  if (sched === "saturated") {
    return (
      <Badge variant="outline" className="border-warning/40 text-warning">
        <Timer /> 限流让位中
      </Badge>
    );
  }
  if (sched === "no_quota" || sched === "auth") {
    const mins = retryIn == null ? null : Math.max(1, Math.round(retryIn / 60));
    return (
      <Badge variant="outline" className="border-destructive/40 text-destructive">
        <PauseCircle /> {sched === "no_quota" ? "已下架 · 没额度" : "已下架 · 密钥被拒"}
        {mins != null && ` · ${mins} 分后再试`}
      </Badge>
    );
  }
  return (
    <Badge variant="success">
      在用
    </Badge>
  );
}

/** 真实流量的结论。和探测是两个来源，矛盾时两个都要看得见。 */
function Live({ live, fails, lastOk }: { live: string; fails: number; lastOk: number | null }) {
  const map: Record<string, [string, string]> = {
    ok: ["text-success", "真实流量最近成功过"],
    degraded: ["text-warning", "最近成功过，但也在失败"],
    error: ["text-destructive", "连续失败"],
  };
  const [tone, title] = map[live] ?? ["text-muted-foreground", "最近没有真实流量"];
  return (
    <div className="text-xs" title={title}>
      <span className={cn("font-medium", tone)}>
        {live === "ok"
          ? "正常"
          : live === "degraded"
            ? "时好时坏"
            : live === "error"
              ? "连败中"
              : "没数据"}
      </span>
      <span className="text-muted-foreground">
        {fails > 0 && ` · 连败 ${fails}`}
        {` · 上次成功 ${ago(lastOk)}`}
      </span>
    </div>
  );
}

export function RouteHealth() {
  const [body, setBody] = useState<Body | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [loadingBalance, setLoadingBalance] = useState(false);
  // 测试发信的逐条结果。「已发送」不等于「收到了」，所以要把每个地址分开报。
  const [mail, setMail] = useState<
    { sent?: { to: string; ok: boolean; error: string | null }[]; skipped?: number } | null
  >(null);
  const [mailing, setMailing] = useState(false);

  const load = useCallback(async (withBalance = false) => {
    setErr(null);
    if (withBalance) setLoadingBalance(true);
    try {
      const b = await api.get<Body>(
        `/api/admin/route-health${withBalance ? "?balance=1" : ""}`,
      );
      setBody(b);
    } catch (e) {
      setErr(e instanceof Error ? e.message : "读取失败");
    } finally {
      setLoadingBalance(false);
    }
  }, []);

  useEffect(() => {
    void load(false);
  }, [load]);

  async function testAlarm() {
    setMailing(true);
    setMail(null);
    setErr(null);
    try {
      setMail(await api.post("/api/admin/route-health/test-alarm", {}));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "发送失败");
    } finally {
      setMailing(false);
    }
  }

  const rows = body?.rows ?? [];
  const bad = rows.filter((r) => r.sched !== "live").length;
  const failing = rows.filter((r) => r.live === "error").length;
  const spend7d = rows.reduce((s, r) => s + r.cost_7d_usd, 0);

  // 按线路分组，组内保持接口给的顺序（自带地址在前）。
  const groups: { id: string; label: string; vendor: string; rows: Row[] }[] = [];
  for (const r of rows) {
    const last = groups[groups.length - 1];
    if (last && last.id === r.route_id) last.rows.push(r);
    else groups.push({ id: r.route_id, label: r.route_label, vendor: r.vendor, rows: [r] });
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="健康"
        description="每个上游此刻是什么样：系统拿它怎么办、用户的请求在它身上什么结局、我们自己敲门通不通、花了多少还剩多少。"
        actions={
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={() => void load(false)}>
              <RefreshCw /> 刷新
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={loadingBalance}
              onClick={() => void load(true)}
            >
              <Wallet /> {loadingBalance ? "查询中…" : "查余额"}
            </Button>
            <Button variant="outline" size="sm" disabled={mailing} onClick={() => void testAlarm()}>
              <Mail /> {mailing ? "发送中…" : "测试告警邮件"}
            </Button>
          </div>
        }
      />

      <ErrorState message={err} />

      {mail && (
        <div className="rounded-lg border border-border px-4 py-3 text-sm">
          <b>测试信已发出。</b>
          <span className="text-muted-foreground">
            {" "}
            服务端说「已发送」只代表投递出去了 —— 请去邮箱确认真的收到（先翻垃圾箱）。
            收不到的话是发件域在那家邮箱没过，得配 SPF/DKIM。
          </span>
          <ul className="mt-2 space-y-1">
            {(mail.sent ?? []).map((x) => (
              <li key={x.to} className="font-mono text-xs">
                <span className={x.ok ? "text-success" : "text-destructive"}>
                  {x.ok ? "已投递" : "失败"}
                </span>{" "}
                {x.to}
                {x.error && <span className="text-destructive"> — {x.error}</span>}
              </li>
            ))}
          </ul>
          {(mail.skipped ?? 0) > 0 && (
            <p className="mt-1 text-xs text-muted-foreground">
              另有 {mail.skipped} 个管理员账号的 email 填的不是邮箱，已跳过。
            </p>
          )}
        </div>
      )}

      {body && (body.alarm?.usable ?? 0) < (body.alarm?.total ?? 0) && (
        <div className="flex items-start gap-2 rounded-lg border border-warning/40 bg-warning/5 px-4 py-3 text-sm">
          <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warning" />
          <div>
            <b>有管理员收不到线路告警。</b>
            <span className="text-muted-foreground">
              {" "}
              {body.alarm?.total ?? 0} 个管理员账号里只有 {body.alarm?.usable ?? 0} 个填的是邮箱地址，
              其余填的是用户名 —— 线路挂了的通知发不到他们那儿。去「客户」页把邮箱补上。
            </span>
          </div>
        </div>
      )}

      {!body && <TableSkeleton rows={4} columns={["30%", "20%", "25%", "20%"]} label="读取中" />}

      {body && (
        <>
          <SectionReveal as="section" delay={70} className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <Stat label="上游总数" value={num(rows.length)} hint="含每条线路自带的直连" />
            <Stat label="不在轮转" value={num(bad)} hint={bad ? "限流让位或已下架" : "全都在用"} />
            <Stat label="连续失败" value={num(failing)} hint={failing ? "真实流量在报错" : "没有"} />
            <Stat label="近 7 天计费" value={usd(spend7d)} hint="按扣给用户的钱算" />
          </SectionReveal>

          <SectionReveal as="section" delay={140} className="space-y-4">
            {groups.map((g) => (
              <Card key={g.id}>
                <CardHeader>
                  <VendorMark vendor={g.vendor} />
                  <Truncate className="font-semibold">{g.label}</Truncate>
                  <span className="text-xs text-muted-foreground">
                    {g.rows.length} 个上游
                  </span>
                </CardHeader>
                <Separator />
                <div className="divide-y divide-border">
                  {g.rows.map((r) => (
                    <div
                      key={r.endpoint_id}
                      className={cn(
                        "grid gap-x-4 gap-y-2 px-5 py-3 lg:grid-cols-[minmax(0,1fr)_10rem_11rem_9rem]",
                        !r.active && "opacity-55",
                      )}
                    >
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <Truncate className="font-mono text-[13px]" title={r.base_url}>
                            {r.base_url || "—"}
                          </Truncate>
                          {r.is_own && <Badge variant="outline">直连</Badge>}
                          {!r.active && <Badge variant="outline">已停用</Badge>}
                        </div>
                        <p className="text-xs text-muted-foreground">
                          {r.label}
                          {/*
                            说「倍」不说「折」，而且 ≠1 就显示 —— 上一版是 `< 1` 才显示，
                            于是一个 1.5 倍的替补出口在这一屏和原价直连长得一模一样。
                          */}
                          {r.cost_ratio !== 1 &&
                            ` · 进价 ${Number(r.cost_ratio.toFixed(4))}×`}
                          {r.capacity != null && ` · 容量 ${r.capacity}`}
                        </p>
                      </div>

                      <div className="flex items-start">
                        <Sched sched={r.sched} retryIn={r.retry_in} />
                      </div>

                      <div>
                        <Live
                          live={r.live}
                          fails={r.consecutive_failures}
                          lastOk={r.last_ok_secs_ago}
                        />
                        {/* 主动探测和真实流量是两个来源。矛盾时两个都要看得见 ——
                            探测通但真实流量连败，说明问题出在某类请求上，不是这条线本身。 */}
                        {r.probe_ok != null && (
                          <p className="mt-0.5 text-xs text-muted-foreground">
                            探测{" "}
                            {r.probe_ok ? (
                              <span className="text-success">通 {r.probe_ms}ms</span>
                            ) : (
                              <span className="text-destructive">{r.probe_note || "不通"}</span>
                            )}
                          </p>
                        )}
                      </div>

                      <div className="text-xs tabular-nums">
                        <p>
                          今天 {num(r.calls_today)} 次 · {usd(r.cost_today_usd)}
                        </p>
                        <p className="text-muted-foreground">
                          7 天 {num(r.calls_7d)} 次 · {usd(r.cost_7d_usd)}
                        </p>
                        {/* 余额查不到就直说。显示成 0 会让人以为没钱了去充值，
                            而实际可能只是这家没有这个接口。 */}
                        {body.balance_included && (
                          <p className={cn("mt-0.5", r.balance ? "text-foreground" : "text-muted-foreground")}>
                            余额 {r.balance ?? "查不到"}
                          </p>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </Card>
            ))}
          </SectionReveal>

          <p className="text-xs leading-relaxed text-muted-foreground">
            「在用 / 限流让位 / 已下架」是<b>系统此刻拿它怎么办</b>；「正常 / 连败中」是
            <b>用户的请求在它身上的结局</b>；「探测」是<b>我们自己敲门的结果</b>。
            三者经常不一致，而不一致本身就是线索 —— 探测通但真实流量连败，
            多半是某一类请求的问题，不是这条线断了。
          </p>
        </>
      )}
    </div>
  );
}
