import { useCallback, useEffect, useState } from "react";
import { Check, Copy, Lock, Share2 } from "lucide-react";

import { Card, CardContent } from "@/components/ui/card";
import { ConnectCard } from "@/components/ConnectCard";
import { api, type Referral as Standing } from "@/lib/api";
import { DICTS, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";

/*
 * Your invite code and link, and what they have earned.
 *
 * Referring is a privilege an admin grants, so `granted: false` is the ordinary answer and
 * gets a real screen rather than an error — most accounts will see it, and "ask an admin"
 * is a next step, whereas a red failure box is a dead end.
 *
 * The terms shown are the ones in force for referrals bound from now on. Anyone already
 * bound keeps the rate and expiry they were bound under, which is why this says "现在"
 * rather than stating them as permanent.
 */

const usd = (cents: number) =>
  (cents / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });

function CopyField({
  label,
  value,
  mono,
  copied,
  copyLabel,
  copiedLabel,
  onCopy,
}: {
  label: string;
  value: string;
  mono?: boolean;
  copied: boolean;
  /* 按钮上的字也要跟着界面语言走 —— 之前写死了中文，英文界面上就冒出两个「复制」。 */
  copyLabel: string;
  copiedLabel: string;
  onCopy: () => void;
}) {
  return (
    <div className="space-y-1.5">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <div className="flex items-stretch gap-2">
        <div
          className={cn(
            "min-w-0 flex-1 truncate rounded-lg border border-border bg-muted/40 px-3 py-2.5 text-sm",
            mono && "font-mono tracking-wide",
          )}
          title={value}
        >
          {value}
        </div>
        <button
          type="button"
          onClick={onCopy}
          className="inline-flex shrink-0 items-center gap-1.5 rounded-lg border border-border px-3 text-sm font-medium transition-colors hover:bg-muted"
        >
          {copied ? <Check className="size-4 text-emerald-600" /> : <Copy className="size-4" />}
          {copied ? copiedLabel : copyLabel}
        </button>
      </div>
    </div>
  );
}

export function Invite({ lang }: { lang: Lang }) {
  const t = DICTS[lang];
  const [data, setData] = useState<Standing | null>(null);
  const [failed, setFailed] = useState(false);
  const [copied, setCopied] = useState<"code" | "link" | null>(null);

  const load = useCallback(async () => {
    try {
      setData(await api.referral());
      setFailed(false);
    } catch {
      setFailed(true);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function copy(what: "code" | "link", value: string) {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(what);
      setTimeout(() => setCopied(null), 1600);
    } catch {
      /* clipboard refused — the value is selectable on screen either way */
    }
  }

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">{t.navInvite}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{t.referralLede}</p>
      </header>

      {/* 三种"还没有内容"的状态高度一致,页面解析出结果时不会先矮一下再蹿高。 */}
      {failed ? (
        <Card>
          <CardContent className="grid min-h-[calc(100vh-15rem)] place-items-center px-6 text-center text-sm text-muted-foreground">
            {t.referralFailed}
          </CardContent>
        </Card>
      ) : !data ? (
        <Card>
          <CardContent className="grid min-h-[calc(100vh-15rem)] place-items-center px-6 text-center text-sm text-muted-foreground">
            {t.loading}
          </CardContent>
        </Card>
      ) : !data.granted ? (
        /*
         * 没开通不是错误,是这个账号现在的状态 —— 给一个说明和下一步,不是一个红框。
         *
         * 这一屏只有这一块内容,所以它得撑得住整页 —— 高度直接跟着视口走,而不是钉死一个
         * 数字。钉死的那版(26rem)在 1330px 高的窗口里仍然只占上面三分之一,下面一大片空白,
         * 看着还是一张没放稳的小卡片。减去的 15rem 是标题区加外边距。
         */
        <Card>
          <CardContent className="flex min-h-[calc(100vh-15rem)] flex-col items-center justify-center gap-5 px-6 py-20 text-center">
            <span className="grid size-20 place-items-center rounded-full bg-muted">
              <Lock className="size-9 text-muted-foreground" />
            </span>
            <p className="text-2xl font-semibold tracking-tight">{t.referralLockedTitle}</p>
            <p className="max-w-xl text-pretty text-base leading-relaxed text-muted-foreground">
              {t.referralLockedBody}
            </p>
          </CardContent>
        </Card>
      ) : (
        <>
          <Card>
            <CardContent className="space-y-5 py-6">
              <CopyField
                label={t.referralCode}
                value={data.code ?? ""}
                mono
                copied={copied === "code"}
                copyLabel={t.referralCopy}
                copiedLabel={t.referralCopied}
                onCopy={() => void copy("code", data.code ?? "")}
              />
              <CopyField
                label={t.referralLink}
                value={data.link ?? ""}
                copied={copied === "link"}
                copyLabel={t.referralCopy}
                copiedLabel={t.referralCopied}
                onCopy={() => void copy("link", data.link ?? "")}
              />
              <p className="text-xs leading-relaxed text-muted-foreground">
                {t.referralHow
                  .replace("{rate}", String(Number((data.rate_bps / 100).toFixed(2))))
                  .replace("{days}", String(data.window_days))}
              </p>
              {/* 计划整体关掉的时候要说一声，否则分享出去的链接绑不上人也没人知道为什么。 */}
              {!data.enabled && (
                <p className="rounded-lg bg-amber-50 px-3 py-2 text-xs leading-relaxed text-amber-900 dark:bg-amber-950/40 dark:text-amber-200">
                  {t.referralPaused}
                </p>
              )}
            </CardContent>
          </Card>

          {/*
            * 收款账户。放在这里而不是「提现」页：开了自动打款之后提现页是藏起来的，
            * 而绑定 Stripe 恰恰是自动打款唯一的前提 —— 没绑账户，每一轮打款都会以
            * 「未连接收款账户」跳过。入口不能长在一个会消失的页面上。
            */}
          <ConnectCard lang={lang} />

          <div className="grid gap-4 sm:grid-cols-3">
            <Card>
              <CardContent className="py-5">
                <div className="text-xs text-muted-foreground">{t.referralInvited}</div>
                <div className="mt-1 text-2xl font-semibold tabular-nums">
                  {data.invited ?? 0}
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-5">
                <div className="text-xs text-muted-foreground">{t.referralPending}</div>
                <div className="mt-1 text-2xl font-semibold tabular-nums">
                  {usd(data.pending_cents ?? 0)}
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="py-5">
                <div className="text-xs text-muted-foreground">{t.referralSettled}</div>
                <div className="mt-1 text-2xl font-semibold tabular-nums">
                  {usd(data.settled_cents ?? 0)}
                </div>
              </CardContent>
            </Card>
          </div>

          <p className="flex items-start gap-2 text-xs leading-relaxed text-muted-foreground">
            <Share2 className="mt-0.5 size-3.5 shrink-0" />
            {t.referralPayout}
          </p>
        </>
      )}
    </div>
  );
}
