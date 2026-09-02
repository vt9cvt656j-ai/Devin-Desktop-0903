import { useCallback, useEffect, useState } from "react";
import { Image as ImageIcon, Loader2, Lock, Wallet } from "lucide-react";

import { Card, CardContent } from "@/components/ui/card";
import { api, type WithdrawState } from "@/lib/api";
import { ConnectCard } from "@/components/ConnectCard";
import { DICTS, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";

/*
 * Asking to be paid.
 *
 * This screen requests; it does not pay. Nothing in this service moves money to a person —
 * an operator reads the queue and sends it by whatever means they actually use, then marks
 * it done. The copy says so plainly rather than letting a button labelled 提现 imply a
 * transfer is already on its way, because the gap between those two readings is measured in
 * angry support messages.
 *
 * Only *settled* commission can be drawn. Pending commission is earned but not yet approved
 * by an operator, so it is shown separately with its own label rather than folded into the
 * balance and then refused at submit time.
 */

const usd = (cents: number) =>
  (cents / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });

const day = (iso: string, lang: Lang) => {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(lang === "en" ? "en-GB" : lang);
};

const STATUS_STYLE: Record<string, string> = {
  pending: "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-400",
  paid: "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400",
  rejected: "bg-muted text-muted-foreground",
};

export function Withdraw({ lang }: { lang: Lang }) {
  const t = DICTS[lang];
  const [data, setData] = useState<WithdrawState | null>(null);
  const [granted, setGranted] = useState<boolean | null>(null);
  const [failed, setFailed] = useState(false);

  const [amount, setAmount] = useState("");
  const [method, setMethod] = useState("alipay");
  const [account, setAccount] = useState("");
  /** 收款码，data: 图片。支付宝和微信是扫码转账的,只有一个账号字段等于让人到聊天里补图。 */
  const [qr, setQr] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);

  const load = useCallback(async () => {
    try {
      const [standing, state] = await Promise.all([api.referral(), api.withdrawals()]);
      setGranted(standing.granted);
      setData(state);
      setFailed(false);
    } catch {
      setFailed(true);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function submit() {
    if (!data) return;
    const cents = Math.round(Number(amount) * 100);
    if (!Number.isFinite(cents) || cents <= 0) {
      setNote({ text: t.withdrawBadAmount, ok: false });
      return;
    }
    setBusy(true);
    setNote(null);
    try {
      const r = await api.requestWithdrawal({
        amount_cents: cents,
        method,
        account: account.trim(),
        qr: qr || undefined,
      });
      // 自动打款成功和「排进队列等人处理」是两件完全不同的事,不能共用一句提示。
      setNote({ text: r.auto_paid ? t.withdrawPaidNow : t.withdrawSubmitted, ok: true });
      setAmount("");
      setQr("");
      await load();
    } catch (e) {
      // The gateway's refusals are already sentences a person can act on ("可提现余额
      // 只有 $x"), so they are shown as-is rather than replaced with a generic failure.
      setNote({ text: e instanceof Error ? e.message : t.withdrawFailed, ok: false });
    } finally {
      setBusy(false);
    }
  }

  const shell = (inner: React.ReactNode) => (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">{t.navWithdraw}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{t.withdrawLede}</p>
      </header>
      {inner}
    </div>
  );

  const full = "grid min-h-[calc(100vh-15rem)] place-items-center px-6 text-center text-sm text-muted-foreground";

  if (failed) return shell(<Card><CardContent className={full}>{t.referralFailed}</CardContent></Card>);
  if (!data || granted === null)
    return shell(<Card><CardContent className={full}>{t.loading}</CardContent></Card>);

  if (!granted) {
    return shell(
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
      </Card>,
    );
  }

  const enough = data.available_cents >= data.min_cents;

  return shell(
    <>
      <div className="grid gap-4 sm:grid-cols-2">
        <Card>
          <CardContent className="py-5">
            <div className="text-xs text-muted-foreground">{t.withdrawAvailable}</div>
            <div className="mt-1 text-3xl font-semibold tabular-nums">
              {usd(data.available_cents)}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="py-5">
            <div className="text-xs text-muted-foreground">{t.withdrawPending}</div>
            <div className="mt-1 text-3xl font-semibold tabular-nums text-muted-foreground">
              {usd(data.pending_commission_cents)}
            </div>
            <div className="mt-1 text-xs text-muted-foreground">{t.withdrawPendingNote}</div>
          </CardContent>
        </Card>
      </div>

      {/* 收款账户卡片现在由「邀请」页承载（提现页在自动打款下会被隐藏）。 */}
      <ConnectCard lang={lang} />

      <Card>
        <CardContent className="space-y-4 py-6">
          <div className="grid gap-4 sm:grid-cols-3">
            <label className="space-y-1.5">
              <span className="text-xs font-medium text-muted-foreground">{t.withdrawAmount}</span>
              <input
                inputMode="decimal"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder={(data.min_cents / 100).toFixed(2)}
                disabled={!enough}
                className="h-11 w-full rounded-lg border border-input bg-card px-3 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/25 disabled:opacity-50"
              />
            </label>

            <label className="space-y-1.5">
              <span className="text-xs font-medium text-muted-foreground">{t.withdrawMethod}</span>
              <select
                value={method}
                onChange={(e) => setMethod(e.target.value)}
                disabled={!enough}
                className="h-11 w-full rounded-lg border border-input bg-card px-3 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/25 disabled:opacity-50"
              >
                {data.methods.map((m) => (
                  <option key={m} value={m}>
                    {t[`withdrawMethod_${m}` as keyof typeof t] ?? m}
                  </option>
                ))}
              </select>
            </label>

            <label className="space-y-1.5">
              <span className="text-xs font-medium text-muted-foreground">{t.withdrawAccount}</span>
              <input
                value={account}
                onChange={(e) => setAccount(e.target.value)}
                placeholder={t.withdrawAccountHint}
                disabled={!enough}
                className="h-11 w-full rounded-lg border border-input bg-card px-3 text-sm outline-none transition-colors focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/25 disabled:opacity-50"
              />
            </label>
          </div>

          {/* 收款码是可选的:银行卡和 PayPal 填个账号就够了,扫码收款的才需要图。 */}
          <div className="flex flex-wrap items-center gap-3">
            <label
              className={cn(
                "flex cursor-pointer items-center gap-2 rounded-lg border border-dashed border-border px-3 py-2 text-xs text-muted-foreground transition-colors hover:border-foreground/25",
                !enough && "pointer-events-none opacity-50",
              )}
            >
              <input
                type="file"
                accept="image/png,image/jpeg,image/webp"
                className="hidden"
                onChange={async (e) => {
                  const file = e.target.files?.[0];
                  e.target.value = "";
                  if (!file) return;
                  // 4MB 上限在读之前拦一道:网关那边的上限是编码后的字符数,让人上传完
                  // 再被拒等于白等一次。
                  if (file.size > 4 * 1024 * 1024) {
                    setNote({ text: t.withdrawQrTooBig, ok: false });
                    return;
                  }
                  const reader = new FileReader();
                  reader.onload = () => setQr(String(reader.result ?? ""));
                  reader.readAsDataURL(file);
                }}
              />
              <ImageIcon className="size-3.5" />
              {qr ? t.withdrawQrChange : t.withdrawQrAdd}
            </label>
            {qr && (
              <span className="flex items-center gap-2">
                <img src={qr} alt="" className="size-10 rounded border border-border object-cover" />
                <button
                  type="button"
                  onClick={() => setQr("")}
                  className="text-xs text-muted-foreground underline-offset-2 hover:underline"
                >
                  {t.withdrawQrRemove}
                </button>
              </span>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <button
              type="button"
              onClick={submit}
              disabled={!enough || busy || !amount || !account.trim()}
              className="inline-flex items-center gap-2 rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
            >
              {busy && <Loader2 className="size-4 animate-spin" />}
              {t.withdrawSubmit}
            </button>
            <span
              className={cn(
                "min-w-0 flex-1 text-xs leading-relaxed",
                note ? (note.ok ? "text-emerald-600" : "text-destructive") : "text-muted-foreground",
              )}
            >
              {note?.text ??
                (enough
                  ? t.withdrawHow.replace("{min}", usd(data.min_cents))
                  : t.withdrawTooLittle.replace("{min}", usd(data.min_cents)))}
            </span>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-0">
          {data.rows.length === 0 ? (
            <div className="flex flex-col items-center gap-3 px-6 py-14 text-center">
              <span className="grid size-12 place-items-center rounded-full bg-muted">
                <Wallet className="size-5 text-muted-foreground" />
              </span>
              <p className="text-sm font-medium">{t.withdrawNone}</p>
            </div>
          ) : (
            <ul className="divide-y divide-border">
              {data.rows.map((w) => (
                <li key={w.id} className="flex flex-wrap items-center gap-x-4 gap-y-1 px-5 py-4">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-semibold tabular-nums">
                        {usd(w.amount_cents)}
                      </span>
                      <span
                        className={cn(
                          "rounded-full px-2 py-0.5 text-[11px] font-medium",
                          STATUS_STYLE[w.status] ?? "bg-muted text-muted-foreground",
                        )}
                      >
                        {t[`withdrawStatus_${w.status}` as keyof typeof t] ?? w.status}
                      </span>
                    </div>
                    <div className="mt-0.5 truncate text-xs text-muted-foreground">
                      {t[`withdrawMethod_${w.method}` as keyof typeof t] ?? w.method} · {w.account}
                      {w.note && ` · ${w.note}`}
                    </div>
                  </div>
                  <div className="text-xs text-muted-foreground">{day(w.created_at, lang)}</div>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <p className="text-xs leading-relaxed text-muted-foreground">{t.withdrawManual}</p>
    </>,
  );
}
