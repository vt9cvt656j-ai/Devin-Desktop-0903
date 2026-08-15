import { useCallback, useEffect, useRef, useState } from "react";
import { RefreshCw } from "lucide-react";

import { Card, CardContent } from "@/components/ui/card";
import { mseFetch } from "@/lib/mse";
import { cn } from "@/lib/utils";
import { formatDateTime } from "@/lib/format";
import type { Lang } from "@/lib/i18n";

/*
 * Reachability of every configured model route.
 *
 * Every figure here is measured by the prober in the gateway (health.rs): one HTTP round
 * trip to each route's own front door, once a minute. That is deliberately the honest and
 * free half of the picture — it says the network path and the provider are alive, and how
 * quickly they answer. It is NOT conversation latency, which cannot be measured without
 * paying for a completion against every model on every cycle, so this page does not claim
 * to show it. A number nobody measured has no place on a status page.
 */

type Sample = { ok: boolean; ms: number | null };

type ModelCard = {
  id: string;
  label: string;
  provider: string;
  model: string;
  state: "ok" | "degraded" | "error" | "unknown";
  ping_ms: number | null;
  /** Null until the route has been probed at least once — not 100%. */
  availability: number | null;
  window_days: number;
  checked_at: string | null;
  samples: Sample[];
};

type StatusPayload = {
  overall: "ok" | "degraded" | "error" | "unknown";
  window_days: number;
  probe_every_secs: number;
  models: ModelCard[];
};

const WINDOWS = [7, 15, 30] as const;

const STATE_STYLE: Record<ModelCard["state"], string> = {
  ok: "bg-emerald-50 text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400",
  degraded: "bg-amber-50 text-amber-700 dark:bg-amber-950/50 dark:text-amber-400",
  error: "bg-rose-50 text-rose-700 dark:bg-rose-950/50 dark:text-rose-400",
  unknown: "bg-muted text-muted-foreground",
};

const STATE_WORD: Record<ModelCard["state"], string> = {
  ok: "Healthy",
  degraded: "Degraded",
  error: "Unreachable",
  unknown: "No data yet",
};

function token(): string {
  try {
    const t = localStorage.getItem("michael_token");
    if (t) return t;
  } catch {
    /* storage blocked; the cookie below is the fallback */
  }
  const m = document.cookie.match(/(?:^|;\s*)mide_token=([^;]*)/);
  return m ? decodeURIComponent(m[1]) : "";
}

/** One bar per probe, oldest on the left. Height carries latency, colour carries outcome. */
function Sparkline({ samples }: { samples: Sample[] }) {
  const slowest = Math.max(300, ...samples.map((s) => s.ms ?? 0));
  return (
    <div className="flex h-10 items-end gap-[2px]" aria-hidden>
      {samples.map((s, i) => {
        // A failed probe still gets a full-height bar: an outage is the most important
        // thing on the strip and must not render as a barely-visible stub.
        const height = s.ok ? Math.max(12, ((s.ms ?? 0) / slowest) * 100) : 100;
        return (
          <span
            key={i}
            className={cn(
              "w-[3px] shrink-0 rounded-sm",
              !s.ok
                ? "bg-rose-500"
                : (s.ms ?? 0) > 2000
                  ? "bg-amber-400"
                  : "bg-emerald-500",
            )}
            style={{ height: `${height}%` }}
          />
        );
      })}
    </div>
  );
}

export function ModelStatus({ lang }: { lang: Lang }) {
  const [data, setData] = useState<StatusPayload | null>(null);
  const [days, setDays] = useState<number>(7);
  const [failed, setFailed] = useState(false);
  const [countdown, setCountdown] = useState(60);
  const timer = useRef<number | null>(null);

  const load = useCallback(async (window: number) => {
    try {
      // This page bypasses lib/api's request(), so it needs its own mseFetch — left on
      // plain fetch it would be the one screen still handing this account's Bearer token
      // to every hop in the clear, once a minute, for as long as the tab is open.
      const res = await mseFetch(`/api/models/status?days=${window}`, {
        headers: { Authorization: `Bearer ${token()}` },
        cache: "no-store",
      });
      if (!res.ok) throw new Error(String(res.status));
      const body = (await res.json()) as StatusPayload;
      setData(body);
      setFailed(false);
      setCountdown(body.probe_every_secs || 60);
    } catch {
      setFailed(true);
    }
  }, []);

  useEffect(() => {
    void load(days);
  }, [days, load]);

  // Refresh on the same cadence the prober writes at — polling faster would show the
  // same numbers again and suggest the page is more live than the data behind it.
  useEffect(() => {
    timer.current = window.setInterval(() => {
      setCountdown((n) => {
        if (n > 1) return n - 1;
        void load(days);
        return data?.probe_every_secs ?? 60;
      });
    }, 1000);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
  }, [days, load, data?.probe_every_secs]);

  if (failed && !data) {
    return <p className="p-8 text-center text-muted-foreground">Could not load model status.</p>;
  }
  if (!data) {
    return <p className="p-8 text-center text-muted-foreground">Checking routes…</p>;
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">Model routes</h2>
          <p className="text-sm text-muted-foreground">
            Reachability and response time of every route this gateway is configured to
            use, probed once a minute.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <div className="flex rounded-lg border border-border p-0.5">
            {WINDOWS.map((w) => (
              <button
                key={w}
                onClick={() => setDays(w)}
                className={cn(
                  "rounded-md px-3 py-1 text-xs font-medium transition-colors",
                  days === w
                    ? "bg-secondary text-foreground"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {w}d
              </button>
            ))}
          </div>

          <span
            className={cn(
              "rounded-full px-2.5 py-1 text-xs font-semibold uppercase tracking-wide",
              STATE_STYLE[data.overall],
            )}
          >
            {STATE_WORD[data.overall]}
          </span>

          <button
            onClick={() => void load(days)}
            className="flex items-center gap-1.5 rounded-lg border border-border px-2.5 py-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground"
            title="Check again now"
          >
            <RefreshCw className="size-3.5" />
            {countdown}s
          </button>
        </div>
      </div>

      {data.models.length === 0 ? (
        <p className="rounded-xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground">
          No active model routes are configured.
        </p>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {data.models.map((m) => (
            <Card key={m.id}>
              <CardContent className="space-y-4 p-5">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <p className="truncate font-semibold leading-tight">{m.label}</p>
                    <p className="mt-1 flex flex-wrap items-center gap-1.5 text-xs">
                      <span className="rounded bg-secondary px-1.5 py-0.5 font-medium text-foreground">
                        {m.provider}
                      </span>
                      <span className="truncate font-mono text-muted-foreground">{m.model}</span>
                    </p>
                  </div>
                  <span
                    className={cn(
                      "shrink-0 rounded-full px-2 py-0.5 text-[11px] font-semibold",
                      STATE_STYLE[m.state],
                    )}
                  >
                    {STATE_WORD[m.state]}
                  </span>
                </div>

                <div className="rounded-lg border border-border p-3">
                  <p className="text-[11px] uppercase tracking-wide text-muted-foreground">
                    Endpoint response
                  </p>
                  <p className="mt-0.5 text-2xl font-semibold tabular-nums">
                    {m.ping_ms == null ? "—" : m.ping_ms}
                    <span className="ml-1 text-sm font-normal text-muted-foreground">ms</span>
                  </p>
                </div>

                <div className="flex items-baseline justify-between border-t border-border pt-3">
                  <span className="text-xs text-muted-foreground">
                    Availability · {m.window_days}d
                  </span>
                  {/* Never a fabricated 100%: a route with no samples says so. */}
                  <span className="text-2xl font-semibold tabular-nums">
                    {m.availability == null ? (
                      <span className="text-sm font-normal text-muted-foreground">
                        not measured yet
                      </span>
                    ) : (
                      <>
                        {m.availability.toFixed(2)}
                        <span className="ml-0.5 text-sm font-normal text-muted-foreground">%</span>
                      </>
                    )}
                  </span>
                </div>

                <div>
                  <div className="mb-1.5 flex items-center justify-between text-[11px] text-muted-foreground">
                    <span>Last {m.samples.length} checks</span>
                    <span>{m.checked_at ? formatDateTime(m.checked_at, lang) : "—"}</span>
                  </div>
                  {m.samples.length === 0 ? (
                    <p className="text-xs text-muted-foreground">
                      The first probe runs within a minute of the gateway starting.
                    </p>
                  ) : (
                    <>
                      <Sparkline samples={m.samples} />
                      <div className="mt-1 flex justify-between text-[10px] uppercase tracking-wide text-muted-foreground">
                        <span>Past</span>
                        <span>Now</span>
                      </div>
                    </>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
