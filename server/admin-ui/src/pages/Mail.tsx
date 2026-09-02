import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, Loader2, Mail as MailIcon, Send, Users } from "lucide-react";

import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Panel } from "@/components/Panel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { api } from "@/lib/api";
import { planKeys, useSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";

/**
 * 群发邮件 —— 给全部用户、会员，或某个套餐。
 *
 * 两件事贯穿整页:
 *
 * 1. **先看人数,再发。** 收件人数是单独一次请求,选完范围就显示。"发给所有人"在
 *    12 个人和 12000 个人身上是完全不同的决定,不该发完才知道是哪一种。
 * 2. **发出去就收不回来。** 所以有一步明确的确认,并且确认框里写的是人数,不是
 *    "确定吗"。这是这一页唯一不可撤销的动作。
 *
 * 退订不在这里做开关:每封群发都自动带退订链接,退订过的人不会出现在收件人里。
 * 验证码那条路不受影响 —— 那是对方刚做的动作的回执,不是可以取消订阅的东西。
 */

type Campaign = {
  id: number;
  segment: string;
  plan: string;
  subject: string;
  html: boolean;
  total: number;
  sent: number;
  failed: number;
  status: string;
  created_by: string;
  created_at: string;
  finished_at: string | null;
};

type Audience = { count: number; opted_out: number; mail_configured: boolean };

const SEGMENTS = [
  { key: "all", label: "全部用户", hint: "所有没有退订的账号。" },
  { key: "members", label: "会员", hint: "套餐仍在有效期内的账号。" },
  { key: "plan", label: "指定套餐", hint: "只发给某一档。" },
  { key: "one", label: "单个邮箱", hint: "先发给自己看看效果。" },
] as const;

/**
 * 套餐清单**从服务端来**（lib/settings.ts 的 planKeys）。
 *
 * 以前这里写死 ["trial","basic","pro","power","ultra"]，而运营能在后台新建套餐 ——
 * 线上 plan_quotas 现在有 6 个，写死的那份漏掉了 `ceshi`。症状不是报错，
 * 是这个下拉框里根本没有那一档，运营会以为那个套餐坏了。
 */

const SEGMENT_LABEL: Record<string, string> = {
  all: "全部用户",
  members: "会员",
  plan: "套餐",
  one: "单个邮箱",
};

const STATUS_STYLE: Record<string, string> = {
  running: "bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-400",
  done: "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400",
  dev: "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-400",
};

const STATUS_LABEL: Record<string, string> = {
  running: "发送中",
  done: "已完成",
  dev: "未发送",
};

function when(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("zh-CN", { hour12: false });
}

export type MailView = "mail" | "mail-log";

export function Mail({ view }: { view: MailView }) {
  // 必须订阅：planKeys() 读的是快照，不订阅的话设置到货后这个组件不会重渲染，
  // 套餐下拉框会一直空着。
  useSettings();
  const [campaigns, setCampaigns] = useState<Campaign[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [segment, setSegment] = useState<string>("one");
  const [plan, setPlan] = useState<string>("pro");
  const [email, setEmail] = useState("");
  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");
  const [html, setHtml] = useState(false);

  const [audience, setAudience] = useState<Audience | null>(null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);

  // 发送记录只有那一屏用得上；写信的那屏拉它是白跑一次请求。
  const load = useCallback(async () => {
    if (view !== "mail-log") return;
    try {
      setCampaigns(await api.get<Campaign[]>("/api/admin/email/campaigns"));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    }
  }, [view]);

  useEffect(() => {
    void load();
  }, [load]);

  /*
   * 只要还有活跃的发送任务就轮询,发完自动停。
   *
   * 依赖里带上进度本身,所以每写进去一次都会重新排一次;没有活跃任务时这个 effect
   * 什么都不做,页面就安静下来了。
   */
  const running = campaigns?.some((c) => c.status === "running") ?? false;
  useEffect(() => {
    if (!running) return;
    const t = setTimeout(() => void load(), 2000);
    return () => clearTimeout(t);
  }, [running, campaigns, load]);

  // 收件人数随范围变,不随正文变 —— 所以只在这三个值上重新数。
  useEffect(() => {
    let alive = true;
    if (view !== "mail") return;
    if (segment === "one" && !email.includes("@")) {
      setAudience(null);
      return;
    }
    void (async () => {
      try {
        const a = await api.post<Audience>("/api/admin/email/audience", {
          target: segment,
          plan,
          email,
        });
        if (alive) setAudience(a);
      } catch {
        if (alive) setAudience(null);
      }
    })();
    return () => {
      alive = false;
    };
  }, [view, segment, plan, email]);

  const ready =
    subject.trim().length > 0 && body.trim().length > 0 && (audience?.count ?? 0) > 0;

  async function send() {
    const n = audience?.count ?? 0;
    // 唯一一处不可撤销的动作。确认框里写清楚发给多少人、什么范围 —— "确定吗"
    // 挡不住任何人,写了人数才挡得住。
    const who =
      segment === "one" ? email : `${SEGMENT_LABEL[segment]}${segment === "plan" ? ` · ${plan}` : ""}`;
    if (!confirm(`把《${subject}》发给 ${who},共 ${n} 个收件人?\n\n发出去就收不回来了。`)) {
      return;
    }
    setBusy(true);
    setNote(null);
    try {
      const r = await api.post<{ id: number; total: number; dev: boolean }>(
        "/api/admin/email/send",
        { target: segment, plan, email, subject, body, html },
      );
      setNote(
        r.dev
          ? {
              text: `没有配置邮件服务,${r.total} 封都没有真的发出去,只记了日志。`,
              ok: false,
            }
          : { text: `已开始发送,共 ${r.total} 封。下面能看到进度。`, ok: true },
      );
      setSubject("");
      setBody("");
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "发送失败", ok: false });
    } finally {
      setBusy(false);
    }
  }

  const seg = SEGMENTS.find((s) => s.key === segment);

  return (
    /*
     * 整页一个测量宽度，标题和面板共用一条左边缘。
     *
     * 之前是满宽的标题配满宽的面板，面板里再塞一个 mx-auto max-w-3xl 的表单 —— 于是
     * 表单浮在面板中间，左边空出一大条，看着像少画了一栏东西。
     */
    <div className="mx-auto w-full max-w-3xl space-y-6">
      <PageHeader
        title={view === "mail" ? "邮件 · 写一封" : "邮件 · 发送记录"}
        description={
          view === "mail"
            ? "给用户和会员群发消息。每封都带退订链接,退订过的人不会收到下一封;验证码这类回执不受影响。"
            : "发过的每一批邮件:发给了谁、多少封、成没成。还在发的会自己刷新进度。"
        }
      />

      {view === "mail" && audience && !audience.mail_configured && (
        <div className="flex items-start gap-2.5 rounded-lg border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-900 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-200">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" />
          <span>
            服务器上没有配置邮件服务(BREVO_API_KEY),现在点发送只会写日志,不会真的发出去。
          </span>
        </div>
      )}

      {view === "mail" && (
      <Panel
        bodyClassName="p-5"
        title="写一封"
        aside={
          audience && (
            <span className="flex items-center gap-1.5 text-sm text-muted-foreground">
              <Users className="size-3.5" />
              <span className="font-semibold text-foreground">{audience.count}</span> 个收件人
              {audience.opted_out > 0 && (
                <span className="text-muted-foreground/70">· {audience.opted_out} 人已退订</span>
              )}
            </span>
          )
        }
      >
        <div className="space-y-5">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="mail-segment">发给谁</Label>
              <Select
                id="mail-segment"
                className="text-sm"
                value={segment}
                onChange={(e) => setSegment(e.target.value)}
              >
                {SEGMENTS.map((s) => (
                  <option key={s.key} value={s.key}>
                    {s.label}
                  </option>
                ))}
              </Select>
              {seg && <p className="text-xs text-muted-foreground">{seg.hint}</p>}
            </div>

            {segment === "plan" && (
              <div className="space-y-2">
                <Label htmlFor="mail-plan">套餐</Label>
                <Select
                  id="mail-plan"
                  className="text-sm"
                  value={plan}
                  onChange={(e) => setPlan(e.target.value)}
                >
                  {planKeys().map((p) => (
                    <option key={p} value={p}>
                      {p}
                    </option>
                  ))}
                </Select>
              </div>
            )}

            {segment === "one" && (
              <div className="space-y-2">
                <Label htmlFor="mail-to">收件邮箱</Label>
                <Input
                  id="mail-to"
                  className="text-sm"
                  placeholder="you@example.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                />
              </div>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="mail-subject">主题</Label>
            <Input
              id="mail-subject"
              className="text-sm"
              placeholder="一句话说清这封信是干什么的"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="mail-body">正文</Label>
              <label className="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground">
                <input
                  type="checkbox"
                  className="size-3.5 accent-primary"
                  checked={html}
                  onChange={(e) => setHtml(e.target.checked)}
                />
                按 HTML 发送
              </label>
            </div>
            <textarea
              id="mail-body"
              rows={10}
              className="w-full rounded-lg border border-input bg-card px-4 py-3 text-sm leading-relaxed outline-none transition-colors focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/25"
              placeholder={
                html ? "<p>直接写 HTML。</p>" : "直接写纯文本,换行就是换行。"
              }
              value={body}
              onChange={(e) => setBody(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              退订链接会自动加在末尾,不用自己写。
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Button onClick={send} disabled={!ready || busy}>
              {busy ? <Loader2 className="animate-spin" /> : <Send />}
              发送
            </Button>
            {note ? (
              <span className={cn("text-sm", note.ok ? "text-emerald-600" : "text-destructive")}>
                {note.text}
              </span>
            ) : (
              <span className="text-sm text-muted-foreground">
                主题和正文都要填。发送前会再确认一次人数。
              </span>
            )}
          </div>
        </div>
      </Panel>
      )}

      {view === "mail-log" && (
      <Panel title="发送记录" aside={
          campaigns && (
            // 服务端是 ORDER BY id DESC LIMIT 50，所以这个数满 50 就不是"总共几条"，
            // 而是被截断后的条数。说清楚它是"最近多少条"，别当总数。
            <span className="text-sm text-muted-foreground">
              最近 {campaigns.length} 条{campaigns.length >= 50 && "（只保留最近 50 条）"}
            </span>
          )
        }>
        {/* 三种「还没有内容」的状态同高，页面不会先矮一下再蹿高。 */}
        {error ? (
          <div className="grid min-h-[20rem] place-items-center px-5">
            <ErrorState message={error} onRetry={() => void load()} />
          </div>
        ) : !campaigns ? (
          <EmptyState title="加载中…" className="min-h-[20rem] justify-center" />
        ) : campaigns.length === 0 ? (
          <EmptyState
            icon={MailIcon}
            title="还没发过邮件"
            hint="去「写一封」发一封，先用「单个邮箱」发给自己看看效果。"
            className="min-h-[20rem] justify-center"
          />
        ) : (
          <div className="divide-y divide-border">
            {campaigns.map((c) => {
              const pct = c.total > 0 ? Math.round(((c.sent + c.failed) / c.total) * 100) : 0;
              return (
                <div key={c.id} className="flex items-center gap-4 px-1 py-3.5">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="truncate text-sm font-medium">{c.subject}</span>
                      <Badge className={cn("border-0", STATUS_STYLE[c.status])}>
                        {STATUS_LABEL[c.status] ?? c.status}
                      </Badge>
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {SEGMENT_LABEL[c.segment] ?? c.segment}
                      {c.plan && ` · ${c.plan}`} · {when(c.created_at)} · {c.created_by}
                    </p>
                  </div>

                  <div className="shrink-0 text-right">
                    <div className="text-sm font-semibold tabular-nums">
                      {c.sent}
                      <span className="text-muted-foreground"> / {c.total}</span>
                    </div>
                    {c.failed > 0 && (
                      <div className="text-xs text-destructive">{c.failed} 封失败</div>
                    )}
                  </div>

                  {/* 只在还在发的时候画进度条 —— 发完了那条 100% 的横线不说明任何事。 */}
                  {c.status === "running" && (
                    <div className="h-1.5 w-24 shrink-0 overflow-hidden rounded-full bg-secondary">
                      <div
                        className="h-full rounded-full bg-primary transition-[width] duration-500"
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </Panel>
      )}
    </div>
  );
}
