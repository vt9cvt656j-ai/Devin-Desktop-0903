import { useEffect, useState } from "react";
import { ArrowLeft, ChevronLeft, ChevronRight, Lock } from "lucide-react";

import { authToken, GATEWAY } from "@/lib/account";
import { cn } from "@/lib/utils";

/*
 * Who consumed the most.
 *
 * Two currencies, two columns: money drawn from a balance or plan, and points drawn from
 * the free daily pool. They are not added together — there is no exchange rate between
 * them, and a merged number would be a made-up one.
 *
 * Signed in only, and that gate carries real weight: rows are labelled with the name the
 * account holder set, or their email address if they never set one, alongside the picture
 * from their account. Anyone who can load this page can read all of it.
 */

const FEED = `${GATEWAY}/api/rankings`;

type Row = {
  rank: number;
  name: string;
  /** The same `data:` picture the account console shows, or null if none was uploaded. */
  avatar?: string | null;
  you: boolean;
  cents: number;
  points: number;
  calls: number;
  share: number;
};

/** One column's current page. `page` is what the server served, which is not always what
    was asked for — an out-of-range request is clamped rather than refused. */
type Page = { rows: Row[]; page: number; pages: number; total: number };

type Payload = {
  window: string;
  days: number;
  money: Page;
  points: Page;
  per_page: number;
  total_cents: number;
  total_points: number;
};

const WINDOWS = [
  { key: "day", label: "Today" },
  { key: "week", label: "This week" },
  { key: "month", label: "This month" },
] as const;

const nf = new Intl.NumberFormat("en-US");

const money = (cents: number) =>
  (cents / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });

/** Points are fractional by design — a $0.003 call costs 0.06 of one. */
const points = (n: number) =>
  n.toLocaleString("en-US", { maximumFractionDigits: n < 10 ? 2 : 0 });

/**
 * The picture, if it is safe to put in an `<img>`.
 *
 * These are uploads: whatever the account holder chose, stored verbatim. A raster `data:`
 * URL is inert, but SVG is a document format that can carry script, so it is refused here
 * and the row falls back to a letter. `<img>` would not run that script today — SVG loaded
 * as an image is a passive context — but the guard costs one line and does not depend on
 * that staying true of every browser.
 */
function safePicture(src: string | null | undefined): string | null {
  if (!src) return null;
  const s = src.trim();
  if (!s.toLowerCase().startsWith("data:image/")) return null;
  if (s.toLowerCase().startsWith("data:image/svg")) return null;
  return s;
}

/** The initial shown when there is no picture — the console's rule, so one person's
    avatar looks the same wherever it appears. */
const letterOf = (name: string) => (name || "?").charAt(0).toUpperCase();

function Face({ row }: { row: Row }) {
  const src = safePicture(row.avatar);
  if (src) {
    return (
      // The ring is not decoration: a photo with a white or transparent background —
      // which provider avatars often have — otherwise dissolves into the page and the
      // row reads as though it has no picture at all.
      <img
        src={src}
        alt=""
        className="size-10 shrink-0 rounded-full object-cover ring-1 ring-inset ring-border"
        loading="lazy"
      />
    );
  }
  return (
    // `bg-primary` with `primary-foreground`, matching the console's sidebar avatar and
    // this site's own account badge exactly. It used to be a grey `bg-secondary` circle,
    // which meant the same person had one avatar in the console and a visibly different
    // one here — and while nobody has uploaded a picture, this fallback IS the avatar,
    // so a difference in it is a difference in the whole thing.
    <span
      aria-hidden
      className="grid size-10 shrink-0 place-items-center rounded-full bg-primary text-sm font-semibold text-primary-foreground"
    >
      {letterOf(row.name)}
    </span>
  );
}

function Pager({
  page,
  pages,
  onPage,
}: {
  page: number;
  pages: number;
  onPage: (n: number) => void;
}) {
  // One page is not a pager, it is two dead buttons.
  if (pages <= 1) return null;
  const step = "rounded-lg border border-border px-2.5 py-1 transition-colors disabled:opacity-40 enabled:hover:border-foreground/25 enabled:hover:text-foreground";
  return (
    <div className="mt-6 flex items-center gap-2 text-xs text-muted-foreground">
      <button
        type="button"
        className={step}
        onClick={() => onPage(page - 1)}
        disabled={page <= 1}
        aria-label="Previous page"
      >
        <ChevronLeft className="size-3.5" />
      </button>
      <span className="tabular-nums">
        Page {page} of {pages}
      </span>
      <button
        type="button"
        className={step}
        onClick={() => onPage(page + 1)}
        disabled={page >= pages}
        aria-label="Next page"
      >
        <ChevronRight className="size-3.5" />
      </button>
    </div>
  );
}

function Column({
  title,
  note,
  data,
  amount,
  unit,
  onPage,
}: {
  title: string;
  note: string;
  data: Page;
  amount: (r: Row) => string;
  /** Rendered muted after the figure. Money carries its own "$"; points do not, and a
      bare number beside "$842.10" reads as dollars at a glance. */
  unit?: string;
  onPage: (n: number) => void;
}) {
  const rows = data.rows;
  return (
    <section>
      <h2 className="text-lg font-semibold">{title}</h2>
      <p className="mt-1 text-sm text-muted-foreground">
        {note}
        {data.total > 0 && (
          <span className="text-muted-foreground/70">
            {" "}
            {data.total} {data.total === 1 ? "account" : "accounts"}.
          </span>
        )}
      </p>

      {rows.length === 0 ? (
        <p className="mt-5 rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground">
          Nobody spent any in this period.
        </p>
      ) : (
        <ol className="mt-6 space-y-5">
          {rows.map((r) => (
            <li key={`${r.rank}-${r.name}`}>
              <div className="flex items-center gap-3">
                <span className="w-5 shrink-0 font-mono text-xs text-muted-foreground">
                  {r.rank}
                </span>
                <Face row={r} />
                <span
                  // Addresses are long and will clip; the full one is on hover rather
                  // than allowed to push the figures off the row.
                  title={r.name}
                  className={cn(
                    "min-w-0 flex-1 truncate text-[15px]",
                    r.you && "font-semibold",
                  )}
                >
                  {r.name}
                  {/* The viewer's own row, so they can find themselves in a list of names
                      that are otherwise other people's. */}
                  {r.you && (
                    <span className="ml-2 rounded-full bg-secondary px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">
                      You
                    </span>
                  )}
                </span>
                <span className="shrink-0 text-xs text-muted-foreground">
                  {nf.format(r.calls)} calls
                </span>
                <span className="w-24 shrink-0 text-right text-sm font-semibold tabular-nums">
                  {amount(r)}
                  {unit && (
                    <span className="ml-1 text-xs font-normal text-muted-foreground">
                      {unit}
                    </span>
                  )}
                </span>
              </div>
              {/* Indented to start under the name: rank (20px) + gap + face (40px) + gap. */}
              <div
                aria-hidden
                className="ml-[5.25rem] mt-2 h-[3px] overflow-hidden rounded-full bg-secondary"
              >
                <div
                  className="h-full rounded-full bg-brand"
                  style={{ width: `${Math.max(1.5, r.share)}%` }}
                />
              </div>
            </li>
          ))}
        </ol>
      )}

      <Pager page={data.page} pages={data.pages} onPage={onPage} />
    </section>
  );
}

function SignedOut() {
  return (
    <div className="mt-16 rounded-2xl border border-dashed border-border p-12 text-center">
      <Lock className="mx-auto size-6 text-muted-foreground" aria-hidden />
      <p className="mt-4 text-base font-medium">Sign in to see the rankings</p>
      <p className="mx-auto mt-2 max-w-sm text-pretty text-sm text-muted-foreground">
        This page reports what accounts have spent, so it is not shown to anonymous
        visitors.
      </p>
      <a
        href={`${GATEWAY}/gate`}
        className="mt-6 inline-flex rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90"
      >
        Log in
      </a>
    </div>
  );
}

export function RankingsPage() {
  const [data, setData] = useState<Payload | null>(null);
  /** "loading" until the first answer, so nothing flashes the wrong state. */
  const [state, setState] = useState<"loading" | "ok" | "anon" | "failed">("loading");
  const [window_, setWindow] = useState<string>("week");
  /** The two columns page independently — they are different lengths, so one control
      would strand the shorter one on a blank page. */
  const [moneyPage, setMoneyPage] = useState(1);
  const [pointsPage, setPointsPage] = useState(1);

  useEffect(() => {
    const t = authToken();
    if (!t) {
      setState("anon");
      return;
    }
    let alive = true;
    void (async () => {
      try {
        const res = await fetch(
          `${FEED}?window=${window_}&money_page=${moneyPage}&points_page=${pointsPage}`,
          {
            headers: { Authorization: `Bearer ${t}` },
            cache: "no-store",
          },
        );
        // An expired or revoked token is a signed-out visitor, not a broken page.
        if (res.status === 401 || res.status === 403) {
          if (alive) setState("anon");
          return;
        }
        if (!res.ok) throw new Error(String(res.status));
        const body = (await res.json()) as Payload;
        if (alive) {
          setData(body);
          setState("ok");
        }
      } catch {
        if (alive) setState("failed");
      }
    })();
    return () => {
      alive = false;
    };
  }, [window_, moneyPage, pointsPage]);

  return (
    // Wider than the changelog on purpose: rows are labelled by address for anyone who
    // never set a name, and at the narrower measure every one of those clipped to an
    // ellipsis. Matches the navbar's width, so the page still lines up with the site.
    <main id="main" className="mx-auto max-w-6xl px-4 py-16 sm:px-6 sm:py-24">
      <a
        href="/"
        className="group mb-10 inline-flex items-center gap-1.5 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
      >
        <ArrowLeft className="size-4 transition-transform duration-200 group-hover:-translate-x-0.5" />
        Mr. Day One
      </a>

      <p className="type-eyebrow mb-3">Rankings</p>
      <h1 className="text-balance text-4xl font-semibold sm:text-5xl">Who used the most</h1>
      <p className="type-measure mt-4 text-pretty text-muted-foreground">
        Accounts ranked by what they consumed, counted from real usage through the gateway.
        Money and free points are ranked separately — they are two different balances, not
        one total. Six to a page, and the bar is each account's share of its whole column,
        not of the page.
      </p>

      {state === "anon" ? (
        <SignedOut />
      ) : (
        <>
          <div className="mt-8 flex flex-wrap items-center gap-2">
            {WINDOWS.map((w) => (
              <button
                key={w.key}
                onClick={() => {
                  setWindow(w.key);
                  // A different window is a different list. Staying on page 3 of a
                  // ranking that now has one page reads as an empty result.
                  setMoneyPage(1);
                  setPointsPage(1);
                }}
                className={cn(
                  "rounded-full border px-4 py-1.5 text-sm font-medium transition-colors",
                  window_ === w.key
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-border text-muted-foreground hover:border-foreground/25 hover:text-foreground",
                )}
              >
                {w.label}
              </button>
            ))}
            {data && (
              <span className="ml-auto text-xs text-muted-foreground">
                {money(data.total_cents)} and {points(data.total_points)} points{" "}
                {data.days === 1 ? "in the last 24 hours" : `in the last ${data.days} days`}
              </span>
            )}
          </div>

          {state === "failed" ? (
            <p className="mt-14 rounded-xl border border-dashed border-border p-10 text-center text-sm text-muted-foreground">
              Could not load the rankings just now.
            </p>
          ) : !data ? (
            <p className="mt-14 text-center text-sm text-muted-foreground">Loading…</p>
          ) : (
            <div className="mt-14 grid gap-14 lg:grid-cols-2">
              <Column
                title="By money spent"
                note="Drawn from a balance or a plan."
                data={data.money}
                amount={(r) => money(r.cents)}
                onPage={setMoneyPage}
              />
              <Column
                title="By points spent"
                note="Drawn from the free daily pool."
                data={data.points}
                amount={(r) => points(r.points)}
                unit="pts"
                onPage={setPointsPage}
              />
            </div>
          )}
        </>
      )}
    </main>
  );
}
