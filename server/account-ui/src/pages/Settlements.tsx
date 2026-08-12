import { useCallback, useEffect, useState } from "react";
import { ChevronLeft, ChevronRight, Lock, Wallet } from "lucide-react";

import { Card, CardContent } from "@/components/ui/card";
import { api, type SettlementList } from "@/lib/api";
import { DICTS, type Lang } from "@/lib/i18n";

/*
 * Commission that has actually been settled, and how.
 *
 * "Settled" means two different things depending on the mode the operator runs, and the
 * difference matters to whoever earned it: under automatic settlement the amount was added
 * to this account's credit balance, under manual an operator approved it and it becomes
 * withdrawable. Both say so on the row rather than leaving "已结算" to mean either.
 *
 * The customer's address is masked, as on the referrals screen — the referrer knows who
 * they invited, but this endpoint should not hand out a full address.
 */

const usd = (cents: number) =>
  (cents / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });

const day = (iso: string | null, lang: Lang) => {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(lang === "en" ? "en-GB" : lang);
};

export function Settlements({ lang }: { lang: Lang }) {
  const t = DICTS[lang];
  const [data, setData] = useState<SettlementList | null>(null);
  const [granted, setGranted] = useState<boolean | null>(null);
  const [failed, setFailed] = useState(false);
  const [page, setPage] = useState(1);

  const load = useCallback(async () => {
    try {
      // Both: an empty list means "nothing settled yet" for someone in the programme and
      // "you are not in it" for everyone else.
      const [standing, list] = await Promise.all([api.referral(), api.mySettlements(page)]);
      setGranted(standing.granted);
      setData(list);
      setFailed(false);
    } catch {
      setFailed(true);
    }
  }, [page]);

  useEffect(() => {
    void load();
  }, [load]);

  const shell = (inner: React.ReactNode) => (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">{t.navSettlements}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{t.settlementsLede}</p>
      </header>
      {inner}
    </div>
  );

  const full =
    "grid min-h-[calc(100vh-15rem)] place-items-center px-6 text-center text-sm text-muted-foreground";

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

  if (data.rows.length === 0) {
    return shell(
      <Card>
        <CardContent className="flex min-h-[calc(100vh-15rem)] flex-col items-center justify-center gap-5 px-6 py-20 text-center">
          <span className="grid size-20 place-items-center rounded-full bg-muted">
            <Wallet className="size-9 text-muted-foreground" />
          </span>
          <p className="text-2xl font-semibold tracking-tight">{t.settlementsEmptyTitle}</p>
          <p className="max-w-xl text-pretty text-base leading-relaxed text-muted-foreground">
            {t.settlementsEmptyBody}
          </p>
        </CardContent>
      </Card>,
    );
  }

  return shell(
    <>
      <Card>
        <CardContent className="py-5">
          <div className="text-xs text-muted-foreground">{t.settlementsTotal}</div>
          <div className="mt-1 text-3xl font-semibold tabular-nums">{usd(data.total_cents)}</div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-0">
          <ul className="divide-y divide-border">
            {data.rows.map((r) => {
              // 退款撤销的那笔要留在列表里,不能直接消失 —— 钱数对不上时,一行说明比一个
              // 少掉的条目好找得多。总额那张卡只算还算数的,所以两个数字自然对得上。
              const off = r.status === "reversed";
              return (
                <li
                  key={r.id}
                  className={`flex flex-wrap items-center gap-x-4 gap-y-1 px-5 py-4 ${off ? "opacity-60" : ""}`}
                >
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{r.customer_email}</div>
                    <div className="mt-0.5 text-xs text-muted-foreground">
                      {day(off ? r.reversed_at : r.settled_at, lang)} · {usd(r.amount_cents)} ·{" "}
                      {Number((r.rate_bps / 100).toFixed(2))}%
                    </div>
                  </div>
                  <div className="text-right">
                    <div
                      className={`text-sm font-semibold tabular-nums ${off ? "line-through" : ""}`}
                    >
                      {usd(r.commission_cents)}
                    </div>
                    {/* 「已结算」在两种模式下不是一回事,所以每行都说清是哪一种。 */}
                    <div className="text-xs text-muted-foreground">
                      {off
                        ? t.settlementsReversed
                        : r.settled_by === "auto"
                          ? t.settlementsAuto
                          : t.settlementsManual}
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>

          {data.pages > 1 && (
            <div className="flex items-center gap-2 border-t border-border px-5 py-3 text-xs text-muted-foreground">
              <button
                type="button"
                disabled={data.page <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                className="inline-flex items-center gap-1 rounded-lg border border-border px-2.5 py-1 transition-colors disabled:opacity-40 enabled:hover:bg-muted"
              >
                <ChevronLeft className="size-3.5" />
                {t.pagePrev}
              </button>
              <span className="tabular-nums">
                {t.showingPage.replace("{page}", String(data.page)).replace("{pages}", String(data.pages))}
              </span>
              <button
                type="button"
                disabled={data.page >= data.pages}
                onClick={() => setPage((p) => Math.min(data.pages, p + 1))}
                className="inline-flex items-center gap-1 rounded-lg border border-border px-2.5 py-1 transition-colors disabled:opacity-40 enabled:hover:bg-muted"
              >
                {t.pageNext}
                <ChevronRight className="size-3.5" />
              </button>
            </div>
          )}
        </CardContent>
      </Card>
    </>,
  );
}
