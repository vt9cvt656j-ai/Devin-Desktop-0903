import { useCallback, useEffect, useState } from "react";
import { Link2, Lock, Users } from "lucide-react";

import { Card, CardContent } from "@/components/ui/card";
import { api, type MyReferral } from "@/lib/api";
import { DICTS, type Lang } from "@/lib/i18n";

/*
 * The people this account brought in.
 *
 * Addresses arrive masked from the gateway (`h***0@gmail.com`). A referrer knows perfectly
 * well who they invited, so blanking them out entirely would be theatre — but the endpoint
 * should not hand a full customer address to anyone who managed to get a link clicked,
 * which is a different thing from knowing your own friend's email.
 */

const usd = (cents: number) =>
  (cents / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });

const day = (iso: string, lang: Lang) => {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString(lang === "en" ? "en-GB" : lang);
};

export function Referrals({ lang }: { lang: Lang }) {
  const t = DICTS[lang];
  const [rows, setRows] = useState<MyReferral[] | null>(null);
  const [granted, setGranted] = useState<boolean | null>(null);
  const [failed, setFailed] = useState(false);

  const load = useCallback(async () => {
    try {
      // Both, because the empty list means two different things: "nobody yet" for someone
      // in the programme, and "you are not in it" for everyone else.
      const [standing, list] = await Promise.all([api.referral(), api.myReferrals()]);
      setGranted(standing.granted);
      setRows(list);
      setFailed(false);
    } catch {
      setFailed(true);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const shell = (inner: React.ReactNode) => (
    <div className="space-y-6">
      <header>
        <h1 className="text-2xl font-semibold tracking-tight">{t.navReferrals}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{t.referralsLede}</p>
      </header>
      {inner}
    </div>
  );

  if (failed) {
    return shell(
      <Card>
        <CardContent className="grid min-h-[calc(100vh-15rem)] place-items-center px-6 text-center text-sm text-muted-foreground">
          {t.referralFailed}
        </CardContent>
      </Card>,
    );
  }

  if (rows === null || granted === null) {
    return shell(
      <Card>
        <CardContent className="grid min-h-[calc(100vh-15rem)] place-items-center px-6 text-center text-sm text-muted-foreground">
          {t.loading}
        </CardContent>
      </Card>,
    );
  }

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

  if (rows.length === 0) {
    return shell(
      <Card>
        <CardContent className="flex min-h-[calc(100vh-15rem)] flex-col items-center justify-center gap-5 px-6 py-20 text-center">
          <span className="grid size-20 place-items-center rounded-full bg-muted">
            <Users className="size-9 text-muted-foreground" />
          </span>
          <p className="text-2xl font-semibold tracking-tight">{t.referralsEmptyTitle}</p>
          <p className="max-w-xl text-pretty text-base leading-relaxed text-muted-foreground">
            {t.referralsEmptyBody}
          </p>
        </CardContent>
      </Card>,
    );
  }

  return shell(
    <Card>
      <CardContent className="p-0">
        {/* 一行一个人，不做成表格：手机上四列会被压成竖排单字，而这张列表在手机上看的
            次数不会比在电脑上少。 */}
        <ul className="divide-y divide-border">
          {rows.map((r, i) => (
            <li key={`${r.who}-${i}`} className="flex flex-wrap items-center gap-x-4 gap-y-1 px-5 py-4">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  {r.source === "link" && (
                    <Link2 className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />
                  )}
                  <span className="truncate text-sm font-medium">{r.who}</span>
                </div>
                <div className="mt-0.5 text-xs text-muted-foreground">
                  {day(r.created_at, lang)} · {Number((r.rate_bps / 100).toFixed(2))}%
                </div>
              </div>

              <div className="text-right">
                <div className="text-sm font-semibold tabular-nums">{usd(r.earned_cents)}</div>
                <div className="text-xs text-muted-foreground">
                  {r.active
                    ? t.referralsUntil.replace("{date}", day(r.expires_at, lang))
                    : t.referralsEnded}
                </div>
              </div>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>,
  );
}
