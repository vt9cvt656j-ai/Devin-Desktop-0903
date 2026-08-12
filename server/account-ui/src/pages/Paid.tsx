import { useCallback, useEffect, useRef, useState } from "react";
import { CheckCircle2, Loader2 } from "lucide-react";

import { Card, CardContent } from "@/components/ui/card";
import { api, type PaymentResult } from "@/lib/api";
import { DICTS, type Lang } from "@/lib/i18n";

/*
 * 支付成功页。
 *
 * 用户从 Stripe 跳回来的第一屏。它要回答一个很具体的问题：**我刚花的钱，换到了什么。**
 * 所以这里不写「支付成功」四个字就完事，而是把套餐名、额度、有效期摆出来。
 *
 * 一个必须处理的时序问题：Stripe 先把浏览器重定向回来，webhook 是另一条链路，晚几百毫秒到
 * 几秒都正常。如果这一屏直接读账号，用户会在刚付完钱的瞬间看到「你还没有套餐」。所以：
 *
 *   · 后端那个接口在订单还挂着时会主动去 Stripe 核实并当场发放（和 webhook 同一条履约路径，
 *     先认领后发放，抢不到也不会重复发）；
 *   · 这一屏在拿到 paid=false 时会继续轮询，最多约 12 秒。
 *
 * 12 秒之后仍然没到账，就明确说「已收到付款，正在入账」并照常跳回后台 —— 那句话是真的：
 * 钱确实收到了，对账扫描每 10 分钟还会再补一次。绝不能显示成失败。
 */

const RETURN_AFTER_SECONDS = 5;
const POLL_EVERY_MS = 1500;
const POLL_LIMIT = 8;

export function Paid({ sessionId, lang }: { sessionId: string; lang: Lang }) {
  const t = DICTS[lang];
  const [result, setResult] = useState<PaymentResult | null>(null);
  const [failed, setFailed] = useState(false);
  const [left, setLeft] = useState(RETURN_AFTER_SECONDS);
  const tries = useRef(0);

  const leave = useCallback(() => {
    // 用 replace：返回键不该把人送回一个已经用过的支付回调地址。
    location.replace("/dashboard");
  }, []);

  useEffect(() => {
    let alive = true;
    let timer: number | undefined;

    const poll = async () => {
      try {
        const r = await api.paymentResult(sessionId);
        if (!alive) return;
        setResult(r);
        // 还没入账就再等等 —— 是 webhook 在路上，不是失败。
        if (!r.paid && tries.current < POLL_LIMIT) {
          tries.current += 1;
          timer = window.setTimeout(poll, POLL_EVERY_MS);
        }
      } catch {
        if (alive) setFailed(true);
      }
    };
    void poll();
    return () => {
      alive = false;
      if (timer) window.clearTimeout(timer);
    };
  }, [sessionId]);

  // 倒计时独立于轮询：无论入账查得怎么样，5 秒后都回后台。停在这一屏没有意义 ——
  // 真正的余额和有效期在后台，而且那边的数字才是权威的。
  useEffect(() => {
    const id = window.setInterval(() => {
      setLeft((n) => {
        if (n <= 1) {
          window.clearInterval(id);
          leave();
          return 0;
        }
        return n - 1;
      });
    }, 1000);
    return () => window.clearInterval(id);
  }, [leave]);

  const money = (cents: number | null | undefined, ccy: string | null | undefined) => {
    const v = (Number(cents) || 0) / 100;
    return v.toLocaleString(lang === "en" ? "en-US" : lang, {
      style: "currency",
      currency: (ccy || "usd").toUpperCase(),
    });
  };
  /** 原始计费分 → 用户看到的美元面值。除数由服务端下发，不写死。 */
  const credit = (raw: number | null | undefined, divisor: number) =>
    ((Number(raw) || 0) / (divisor || 663)).toLocaleString("en-US", {
      style: "currency",
      currency: "USD",
    });

  const day = (iso: string | null) => {
    if (!iso) return "—";
    const d = new Date(iso);
    return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(lang === "en" ? "en-GB" : lang);
  };

  const rows: { k: string; v: string }[] = [];
  if (result) {
    const isPlan = result.kind === "plan";
    if (isPlan) {
      rows.push({ k: t.paidPlan, v: result.label || result.plan || "—" });
      rows.push({
        k: t.paidQuota,
        v: credit(result.account.quota_total_cents, result.raw_cents_per_credit_usd),
      });
      rows.push({ k: t.paidUntil, v: day(result.account.plan_expires_at) });
    } else {
      rows.push({ k: t.paidProduct, v: result.label || "—" });
      rows.push({
        k: t.paidCreditsAdded,
        v: credit(result.credits_cents, result.raw_cents_per_credit_usd),
      });
      rows.push({
        k: t.paidBalance,
        v: credit(result.account.credits_cents, result.raw_cents_per_credit_usd),
      });
    }
    if (result.charged_cents) {
      rows.push({ k: t.paidCharged, v: money(result.charged_cents, result.charged_currency) });
    }
  }

  const settling = !!result && !result.paid;

  return (
    <div className="mx-auto flex min-h-[calc(100vh-8rem)] max-w-lg flex-col items-center justify-center px-4">
      <Card className="w-full">
        <CardContent className="flex flex-col items-center gap-5 px-6 py-10 text-center">
          <span className="grid size-16 place-items-center rounded-full bg-emerald-100 dark:bg-emerald-950">
            {result && !settling ? (
              <CheckCircle2 className="size-8 text-emerald-600 dark:text-emerald-400" />
            ) : (
              <Loader2 className="size-8 animate-spin text-emerald-600 dark:text-emerald-400" />
            )}
          </span>

          <div>
            <h1 className="text-2xl font-semibold tracking-tight">{t.paidTitle}</h1>
            <p className="mt-1.5 text-sm text-muted-foreground">
              {failed ? t.paidPending : settling ? t.paidSettling : t.paidSubtitle}
            </p>
          </div>

          {rows.length > 0 && (
            <ul className="w-full divide-y divide-border rounded-lg border border-border text-left">
              {rows.map((r) => (
                <li key={r.k} className="flex items-baseline justify-between gap-4 px-4 py-3">
                  <span className="text-sm text-muted-foreground">{r.k}</span>
                  <span className="text-sm font-semibold tabular-nums">{r.v}</span>
                </li>
              ))}
            </ul>
          )}

          <div className="flex w-full flex-col items-center gap-2">
            <button
              type="button"
              onClick={leave}
              className="w-full rounded-lg bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
            >
              {t.paidGoNow}
            </button>
            <p className="text-xs text-muted-foreground">
              {t.paidReturning.replace("{n}", String(left))}
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
