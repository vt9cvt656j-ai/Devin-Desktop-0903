import { useEffect, useState } from "react";
import { ArrowLeft, Plus, RefreshCw, Wrench } from "lucide-react";

import type { ChangeKind, ChangelogEntry } from "@/data/changelog";
import { cn } from "@/lib/utils";

/*
 * The changelog, as its own page rather than a section of the front page.
 *
 * Standalone because it is a document people arrive at directly and link to, not a stop on
 * the marketing scroll — and because it should stay readable long after it has outgrown
 * anything that would fit inside another page.
 */

const KIND_LABEL: Record<ChangeKind, string> = {
  added: "New",
  fixed: "Fixed",
  changed: "Changed",
};

/*
 * A glyph, not a word in a pill.
 *
 * The labels used to be text in fixed-width capsules, which set three different words in
 * a box sized for the longest — so every row carried a stretched lozenge with the word
 * floating in it, and the three lined up as ragged blobs down the left of the page. A
 * single round icon is one size by construction, so the sentences align, and it reads as
 * a marker rather than as a second heading competing with the change itself.
 *
 * The word is still there for anyone who needs it: `title` on hover, and text for screen
 * readers, which an icon alone would have taken away.
 */
const KIND_ICON: Record<ChangeKind, typeof Plus> = {
  added: Plus,
  fixed: Wrench,
  changed: RefreshCw,
};

const KIND_STYLE: Record<ChangeKind, string> = {
  added: "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400",
  fixed: "bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-400",
  changed: "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-400",
};

/**
 * Written out rather than localised: the page is English, and so are the entries.
 *
 * Built from the parts rather than `new Date(iso)`. A bare "2026-08-10" is parsed as UTC
 * midnight, so west of Greenwich it renders as the ninth — every entry showed a day
 * earlier than it was written.
 */
function readableDate(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  if (!y || !m || !d) return iso;
  return new Date(y, m - 1, d).toLocaleDateString("en-GB", {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

/**
 * Entries come from the gateway, not from this repository.
 *
 * They used to be a constant in `data/changelog.ts`, which meant publishing an entry took
 * a rebuild and a deploy. They now live in the database and are written from the admin
 * console, so this page reads them at runtime — the console is the only place an entry is
 * created or removed, and there is exactly one source of truth.
 */
const FEED = "https://code.mrday.one/api/changelog";

export function ChangelogPage() {
  const [entries, setEntries] = useState<ChangelogEntry[] | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let alive = true;
    void (async () => {
      try {
        const res = await fetch(FEED, { cache: "no-store" });
        if (!res.ok) throw new Error(String(res.status));
        const body = (await res.json()) as { entries: ChangelogEntry[] };
        if (alive) setEntries(body.entries ?? []);
      } catch {
        if (alive) setFailed(true);
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  return (
    <main id="main" className="mx-auto max-w-3xl px-4 py-16 sm:px-6 sm:py-24">
      <a
        href="/"
        className="group mb-10 inline-flex items-center gap-1.5 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
      >
        <ArrowLeft className="size-4 transition-transform duration-200 group-hover:-translate-x-0.5" />
        Mr. Day One
      </a>

      <p className="type-eyebrow mb-3">Update log</p>
      <h1 className="text-balance text-4xl font-semibold sm:text-5xl">What changed</h1>
      <p className="type-measure mt-4 text-pretty text-muted-foreground">
        Notable changes across the editor, the gateway, the account console and this site.
        Written by hand — this is not a list of builds.
      </p>

      {/* A key, because a coloured glyph is only obvious once you have been told. */}
      <div className="mt-6 flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-muted-foreground">
        {(Object.keys(KIND_LABEL) as ChangeKind[]).map((kind) => {
          const Icon = KIND_ICON[kind];
          return (
            <span key={kind} className="flex items-center gap-1.5">
              <span
                className={cn("grid size-5 place-items-center rounded-full", KIND_STYLE[kind])}
              >
                <Icon className="size-3" strokeWidth={2.5} aria-hidden />
              </span>
              {KIND_LABEL[kind]}
            </span>
          );
        })}
      </div>

      {failed ? (
        <p className="mt-16 rounded-xl border border-dashed border-border p-10 text-center text-sm text-muted-foreground">
          Could not load the update log just now.
        </p>
      ) : !entries ? (
        <p className="mt-16 text-center text-sm text-muted-foreground">Loading…</p>
      ) : entries.length === 0 ? (
        <p className="mt-16 rounded-xl border border-dashed border-border p-10 text-center text-sm text-muted-foreground">
          Nothing published yet.
        </p>
      ) : (
        <div className="mt-14 space-y-14">
          {entries.map((entry) => (
            <article key={`${entry.date}-${entry.title}`}>
              <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
                <time
                  dateTime={entry.date}
                  className="font-mono text-xs uppercase tracking-widest text-muted-foreground"
                >
                  {readableDate(entry.date)}
                </time>
                <span className="rounded-full bg-secondary px-2 py-0.5 text-[11px] font-semibold text-foreground">
                  {entry.product}
                </span>
                {entry.version && (
                  <code className="rounded bg-secondary px-1.5 py-0.5 font-mono text-[11px]">
                    {entry.version}
                  </code>
                )}
              </div>

              <h2 className="mt-3 text-balance text-xl font-semibold sm:text-2xl">
                {entry.title}
              </h2>

              <ul className="mt-4 space-y-3">
                {entry.changes.map((c) => {
                  const Icon = KIND_ICON[c.kind];
                  return (
                    <li key={c.text} className="flex gap-3">
                      <span
                        title={KIND_LABEL[c.kind]}
                        className={cn(
                          "mt-[3px] grid size-5 shrink-0 place-items-center rounded-full",
                          KIND_STYLE[c.kind],
                        )}
                      >
                        <Icon className="size-3" strokeWidth={2.5} aria-hidden />
                        <span className="sr-only">{KIND_LABEL[c.kind]}</span>
                      </span>
                      <span className="text-pretty text-[15px] leading-relaxed text-muted-foreground">
                        {c.text}
                      </span>
                    </li>
                  );
                })}
              </ul>
            </article>
          ))}
        </div>
      )}
    </main>
  );
}
