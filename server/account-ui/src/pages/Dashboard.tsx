import { useEffect, useRef, useState } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api, probeDesktop, signOut, type DesktopSession, type Me, type Usage } from "@/lib/api";
import {
  formatDate,
  formatDateTime,
  planIsActive,
  planLabel,
  timeUntil,
  usd,
} from "@/lib/format";
import { DICTS, type Lang } from "@/lib/i18n";

/*
 * The release repo is private, so github.com/.../releases/latest 404s for every signed-in
 * user who clicks Download here. The marketing site's download section reads the gateway's
 * public update feed and shows either a real installer link or an honest "not published
 * yet" — either beats a 404.
 */
const RELEASES = "https://www.michaelide.xyz/#download";

export type Tab = "overview" | "usage" | "settings" | "integrations";

/**
 * Centred, not left-aligned. These sit in a row of three: ragged left-aligned blocks of
 * different lengths read as misaligned rather than as a set. Centring is applied here so
 * the Overview row and the Usage row cannot drift apart.
 */
function Stat({ label, value, sub }: { label: string; value: React.ReactNode; sub?: string }) {
  return (
    <Card className="items-center bg-muted p-6 text-center">
      <p className="text-xs text-muted-foreground">{label}</p>
      <div className="mt-1.5 text-[22px] font-semibold tracking-tight tabular-nums">{value}</div>
      {sub ? <div className="mt-1.5 text-xs text-muted-foreground">{sub}</div> : null}
    </Card>
  );
}

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <>
      <div className="flex items-baseline justify-between gap-5 py-3">
        <span className="text-[13.5px] text-muted-foreground">{k}</span>
        <span className="break-all text-right font-medium">{v}</span>
      </div>
      <Separator className="last:hidden" />
    </>
  );
}

/** Rows per page on the Usage tab. The gateway hands us the most recent 200. */
const USAGE_PAGE_SIZE = 20;

/**
 * Which page numbers to render: always the first and last, the current one and its
 * neighbours, and an ellipsis standing in for whatever is skipped. At 200 rows this
 * never needs to skip anything, but the list is not guaranteed to stay that size.
 */
function pageWindow(current: number, count: number): (number | "gap")[] {
  if (count <= 9) return Array.from({ length: count }, (_, i) => i + 1);
  const out: (number | "gap")[] = [1];
  const from = Math.max(2, current - 2);
  const to = Math.min(count - 1, current + 2);
  if (from > 2) out.push("gap");
  for (let p = from; p <= to; p += 1) out.push(p);
  if (to < count - 1) out.push("gap");
  out.push(count);
  return out;
}

/** Fill {name} placeholders — the dictionary holds strings, not functions. */
function fill(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (_, key) => String(values[key] ?? ""));
}

function UsageTable({
  rows,
  lang,
  limit,
  pageSize,
}: {
  rows: Usage["recent"];
  lang: Lang;
  /** Show at most this many rows and stop. Ignored when pageSize is given. */
  limit?: number;
  /** Paginate at this many rows per page instead of truncating. */
  pageSize?: number;
}) {
  const t = DICTS[lang];
  const [page, setPage] = useState(1);
  const top = useRef<HTMLDivElement | null>(null);
  const painted = useRef(false);

  const total = rows.length;
  const pageCount = pageSize ? Math.max(1, Math.ceil(total / pageSize)) : 1;
  // Usage refetches. If the list comes back shorter, don't strand the reader on a page
  // past the end — derive the page rather than trusting the stored number.
  const current = Math.min(page, pageCount);

  // Put the top of the table back in view after a page change, but never on first
  // paint — that would yank the page around on load.
  useEffect(() => {
    if (!painted.current) {
      painted.current = true;
      return;
    }
    top.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [current]);

  if (!rows.length) return <p className="py-6 text-center text-[13.5px] text-muted-foreground">{t.noRequests}</p>;

  const start = pageSize ? (current - 1) * pageSize : 0;
  const visible = pageSize ? rows.slice(start, start + pageSize) : rows.slice(0, limit ?? total);

  return (
    <div ref={top}>
    <div className="overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{t.when}</TableHead>
            <TableHead>{t.model}</TableHead>
            <TableHead className="text-right">{t.tokensIn}</TableHead>
            <TableHead className="text-right">{t.tokensOut}</TableHead>
            <TableHead className="text-right">{t.cost}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {visible.map((r, i) => (
            <TableRow key={`${r.time}-${i}`}>
              <TableCell className="whitespace-nowrap">{formatDateTime(r.time, lang)}</TableCell>
              <TableCell className="font-medium">
                {r.model}
                {r.estimated ? (
                  <Badge variant="outline" className="ml-2">
                    {t.estimated}
                  </Badge>
                ) : null}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {r.prompt_tokens == null ? "—" : r.prompt_tokens.toLocaleString()}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {r.completion_tokens == null ? "—" : r.completion_tokens.toLocaleString()}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {r.free_points_spent > 0
                  ? `${Math.round(r.free_points_spent * 1000) / 1000} ${t.credits}`
                  : usd(r.cost_cents, 4)}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>

      {pageSize && pageCount > 1 ? (
        <div className="mt-4 flex flex-col-reverse items-center justify-between gap-3 sm:flex-row">
          <p className="text-xs tabular-nums text-muted-foreground">
            {fill(t.showingRange, {
              from: start + 1,
              to: start + visible.length,
              total,
            })}
          </p>
          <nav className="flex items-center gap-1" aria-label={t.requests}>
            <Button
              variant="outline"
              size="sm"
              className="px-2.5"
              disabled={current === 1}
              onClick={() => setPage(current - 1)}
              aria-label={t.pagePrev}
            >
              <ChevronLeft />
              <span className="hidden sm:inline">{t.pagePrev}</span>
            </Button>

            {/* Numbered jumps need room. Below sm they would wrap onto a second line,
                so the phone gets a plain position counter between the arrows instead. */}
            <div className="hidden items-center gap-1 sm:flex">
              {pageWindow(current, pageCount).map((p, i) =>
                p === "gap" ? (
                  <span key={`gap-${i}`} aria-hidden className="px-1 text-sm text-muted-foreground">
                    …
                  </span>
                ) : (
                  <Button
                    key={p}
                    size="sm"
                    variant={p === current ? "default" : "ghost"}
                    className="w-9 px-0 tabular-nums"
                    aria-current={p === current ? "page" : undefined}
                    aria-label={fill(t.goToPage, { page: p })}
                    onClick={() => setPage(p)}
                  >
                    {p}
                  </Button>
                ),
              )}
            </div>
            <span className="px-2 text-sm tabular-nums text-muted-foreground sm:hidden">
              {current} / {pageCount}
            </span>

            <Button
              variant="outline"
              size="sm"
              className="px-2.5"
              disabled={current === pageCount}
              onClick={() => setPage(current + 1)}
              aria-label={t.pageNext}
            >
              <span className="hidden sm:inline">{t.pageNext}</span>
              <ChevronRight />
            </Button>
          </nav>
        </div>
      ) : null}
    </div>
  );
}

export function Dashboard({ me, tab, lang }: { me: Me; tab: Tab; lang: Lang }) {
  const t = DICTS[lang];
  const [usage, setUsage] = useState<Usage | null>(null);
  const [desktop, setDesktop] = useState<DesktopSession | null | undefined>(undefined);
  const [modelCount, setModelCount] = useState<number | null>(null);

  useEffect(() => {
    // Additive: a failure here leaves the page usable.
    void api.usage().then(setUsage).catch(() => undefined);
    void api.models().then((m) => setModelCount(Array.isArray(m) ? m.length : null)).catch(() => undefined);
    void probeDesktop().then(setDesktop);
  }, []);

  const cap = me.quota_window_cap_cents ?? 0;
  const left = me.quota_window_cents ?? 0;
  const spent = Math.max(0, cap - left);
  const pct = cap > 0 ? Math.max(0, Math.min(100, (spent / cap) * 100)) : null;
  const refill = timeUntil(me.quota_window_reset_at);
  const active = planIsActive(me.plan, me.plan_expires_at);

  if (tab === "usage") {
    return (
      <div className="max-w-[1080px]">
        <h1 className="text-xl font-semibold tracking-tight">{t.usage}</h1>
        <p className="mb-6 mt-0.5 text-[13.5px] text-muted-foreground">{t.usageLede}</p>
        <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
          <Stat label={t.spentAllTime} value={usage ? usd(usage.total_spent_cents) : "—"} />
          <Stat
            label={t.requestsShown}
            value={usage ? usage.recent.length : "—"}
            sub={t.requestsShownSub}
          />
          <Stat label={t.creditBalance} value={usage ? usd(usage.credits_cents) : "—"} />
        </div>
        <h2 className="mb-3 mt-8 text-sm font-semibold">{t.requests}</h2>
        <UsageTable rows={usage?.recent ?? []} lang={lang} pageSize={USAGE_PAGE_SIZE} />
      </div>
    );
  }

  if (tab === "settings") {
    return (
      <div className="max-w-[1080px]">
        <h1 className="text-xl font-semibold tracking-tight">{t.settings}</h1>
        <p className="mb-6 mt-0.5 text-[13.5px] text-muted-foreground">{t.settingsLede}</p>

        <h2 className="mb-3 text-sm font-semibold">{t.account}</h2>
        <Card className="bg-muted px-6 py-1">
          <Row k={t.email} v={me.email} />
          <Row k={t.accountId} v={me.id} />
          <Row k={t.role} v={me.role === "admin" ? t.administrator : t.member} />
          <Row k={t.memberSince} v={formatDate(me.created_at, lang)} />
          <Row k={t.lastSignIn} v={formatDateTime(me.last_login_at, lang)} />
        </Card>

        <h2 className="mb-3 mt-8 text-sm font-semibold">{t.plan}</h2>
        <Card className="bg-muted px-6 py-1">
          <Row k={t.currentPlan} v={planLabel(me.plan)} />
          <Row k={t.expires} v={me.plan_expires_at ? formatDate(me.plan_expires_at, lang) : "—"} />
          <Row k={t.includedQuota} v={cap > 0 ? `${usd(cap)} ${t.perWindow}` : t.notIncluded} />
          <Row
            k={t.weeklyCap}
            v={me.quota_weekly_cap_cents > 0 ? `${usd(me.quota_weekly_cap_cents)} ${t.perWeek}` : t.noWeeklyCap}
          />
        </Card>

        <h2 className="mb-3 mt-8 text-sm font-semibold">{t.session}</h2>
        <Card className="bg-muted p-6">
          <p className="mb-4 text-[13.5px] leading-relaxed text-muted-foreground">{t.signOutNote}</p>
          <Button variant="outline" onClick={signOut} className="w-fit">
            {t.signOut}
          </Button>
        </Card>
      </div>
    );
  }

  if (tab === "integrations") {
    return (
      <div className="max-w-[1080px]">
        <h1 className="text-xl font-semibold tracking-tight">{t.integrations}</h1>
        <p className="mb-6 mt-0.5 text-[13.5px] text-muted-foreground">{t.integrationsLede}</p>

        <h2 className="mb-3 text-sm font-semibold">{t.desktopApp}</h2>
        <Card className="bg-muted p-6">
          <div className="mb-2.5 flex items-baseline gap-2.5">
            <span className="text-lg font-semibold">{t.desktopApp}</span>
            {desktop === undefined ? null : desktop === null ? (
              <Badge variant="outline">{t.notDetected}</Badge>
            ) : desktop.signedIn ? (
              <Badge variant="success">{t.connected}</Badge>
            ) : (
              <Badge variant="outline">{t.signedOut}</Badge>
            )}
          </div>
          <p className="mb-4 text-[13.5px] leading-relaxed text-muted-foreground">
            {desktop === undefined
              ? t.loading
              : desktop === null
                ? t.desktopMissing
                : desktop.signedIn
                  ? `${t.desktopConnected} ${desktop.email} (${t.desktopVersion} ${desktop.version}). ${t.desktopReuse}`
                  : t.desktopSignedOut}
          </p>
          <Button variant="outline" asChild className="w-fit">
            <a href={RELEASES} target="_blank" rel="noreferrer">
              {t.download}
            </a>
          </Button>
        </Card>

        <h2 className="mb-3 mt-8 text-sm font-semibold">{t.apiHeading}</h2>
        <Card className="bg-muted px-6 py-1">
          <Row k={t.baseUrl} v={location.origin} />
          <Row k={t.auth} v={t.authValue} />
          <Row k={t.modelsAvailable} v={modelCount == null ? "—" : `${modelCount} ${t.available}`} />
        </Card>
      </div>
    );
  }

  return (
    <div className="max-w-[1080px]">
      <h1 className="text-xl font-semibold tracking-tight">{t.overview}</h1>
      <p className="mb-6 mt-0.5 text-[13.5px] text-muted-foreground">{me.email}</p>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Card className="bg-muted p-6">
          <p className="mb-3.5 text-[13.5px] text-muted-foreground">{t.includedUsage}</p>
          {pct === null ? (
            <>
              <p className="mb-4 text-3xl font-semibold tracking-tight">
                {t.noneIncluded}{" "}
                <span className="text-[15px] font-medium text-muted-foreground">{t.onFreePlan}</span>
              </p>
              <p className="text-xs text-muted-foreground">{t.freeFallback}</p>
            </>
          ) : (
            <>
              {/* The amount spent used to sit here as "$0.00 of $45.25", which restated the
                  percentage beside it — two ways of saying nothing was used. Only the
                  headline keeps the reading; the allowance moves to the right, where it
                  labels the far end of the bar it belongs to. */}
              {/* flex-wrap so a long pair ("100% 已使用" + "$120.66 每时段") drops onto a
                  second line on a phone instead of crushing the headline. */}
              <div className="mb-4 flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
                <p className="text-3xl font-semibold tracking-tight tabular-nums">
                  {Math.round(pct)}% {t.used}
                </p>
                <p className="shrink-0 text-[15px] font-medium tabular-nums text-muted-foreground">
                  {usd(cap)} {t.perWindow}
                </p>
              </div>
              <div className="h-1 overflow-hidden rounded-full bg-accent">
                <div className="h-full rounded-full bg-foreground transition-all" style={{ width: `${pct}%` }} />
              </div>
              <p className="mt-3.5 text-xs text-muted-foreground">
                {refill.expired ? t.refillsNow : `${t.refillsIn} ${refill.text}`}
              </p>
            </>
          )}
        </Card>

        <Card className="items-center bg-muted p-6 text-center">
          <div className="mb-2.5 flex items-baseline justify-center gap-2.5">
            <span className="text-lg font-semibold">{planLabel(me.plan)}</span>
          </div>
          <p className="mb-5 text-[13.5px] leading-relaxed text-muted-foreground">
            {active && me.plan_expires_at
              ? `${t.until} ${formatDate(me.plan_expires_at, lang)}`
              : t.freeFallback}
          </p>
          <Button asChild className="mt-auto w-fit">
            <a href="/billing">{t.navBilling}</a>
          </Button>
        </Card>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4 md:grid-cols-3">
        <Stat label={t.creditBalance} value={usd(me.credits_cents)} sub={t.creditBalanceSub} />
        <Stat
          label={t.dailyFree}
          value={`${Math.round((me.free_points ?? 0) * 100) / 100} / ${me.free_points_daily ?? 0}`}
          sub={t.dailyFreeSub}
        />
        <Stat
          label={t.thisWeek}
          value={
            me.quota_weekly_cap_cents > 0
              ? `${usd(me.quota_week_used_cents)} / ${usd(me.quota_weekly_cap_cents)}`
              : usd(me.quota_week_used_cents)
          }
          sub={me.quota_week_reset_at ? `${t.resets} ${formatDate(me.quota_week_reset_at, lang)}` : undefined}
        />
      </div>

      <h2 className="mb-3 mt-8 text-sm font-semibold">{t.recentActivity}</h2>
      <UsageTable rows={usage?.recent ?? []} lang={lang} limit={6} />
    </div>
  );
}
