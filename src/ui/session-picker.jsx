import { useMemo, useState } from "react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "./components/dialog.jsx";
import { Input } from "./components/input.jsx";
import { cn } from "./lib/cn.js";

/**
 * `/sessions` — browse and resume conversations in this workspace.
 *
 * Modelled on Claude Code's resume list rather than the previous dialog: one line per session
 * carrying name, project, and counts, with the last message as a single muted line underneath.
 * The old version put a wall of eight running totals in a blue banner above the search box —
 * "已打开 1 · 可恢复 80 · 总计 333 轮 · 近期 320 条 · 历史摘要 0 段 · 文件证据 148 个 · 有效纠正
 * 17 条" — which is aggregate trivia you cannot act on, sitting where the thing you came to do
 * (find a conversation) should be. What survives is the one number that sets expectations
 * (how many are resumable) and the per-row counts that actually distinguish one session from
 * another.
 *
 * Purely presentational: main.js owns session state and hands in plain rows.
 */
export function SessionPicker({ entries = [], resumableCount = 0, onPick, onClose }) {
  const [query, setQuery] = useState("");

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return entries;
    return entries.filter((e) => (e.search || "").includes(q));
  }, [entries, query]);

  return (
    <Dialog defaultOpen onOpenChange={(open) => { if (!open) onClose?.(); }}>
      <DialogContent className="max-w-2xl gap-0 overflow-hidden rounded-2xl p-0 shadow-xl [&>[data-slot=dialog-close]]:right-3 [&>[data-slot=dialog-close]]:top-3 [&>[data-slot=dialog-close]]:grid [&>[data-slot=dialog-close]]:size-8 [&>[data-slot=dialog-close]]:place-items-center [&>[data-slot=dialog-close]]:rounded-full [&>[data-slot=dialog-close]]:text-muted-foreground [&>[data-slot=dialog-close]]:opacity-100 [&>[data-slot=dialog-close]]:hover:bg-accent">
        <DialogHeader className="space-y-1 px-5 pt-5 pb-3">
          <DialogTitle className="text-base">Sessions</DialogTitle>
          <DialogDescription className="text-[12px]">
            Older messages stay in context as summaries — this is not only what is still on screen.
          </DialogDescription>
        </DialogHeader>

        <div className="px-5 pb-3">
          <Input
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search sessions, projects, file evidence…"
            className="h-9 rounded-lg text-[13px]"
          />
        </div>

        <div className="max-h-[52vh] overflow-y-auto px-2 pb-2">
          {rows.length === 0 ? (
            <p className="px-3 py-10 text-center text-[13px] text-muted-foreground">
              {entries.length ? "No matching sessions" : "No sessions yet"}
            </p>
          ) : (
            rows.map((e) => (
              <button
                key={e.key}
                type="button"
                onClick={() => onPick?.(e)}
                className={cn(
                  "group flex w-full cursor-pointer flex-col gap-0.5 rounded-lg px-3 py-2 text-left transition-colors",
                  "hover:bg-accent focus-visible:bg-accent focus-visible:outline-none",
                  // Google marks the current item with the light-blue tint, not the neutral hover
                  // wash — otherwise "selected" and "the pointer happens to be here" look alike.
                  e.active && "bg-primary/10",
                )}
              >
                <span className="flex items-baseline gap-2">
                  <span
                    aria-hidden
                    className="size-1.5 shrink-0 self-center rounded-full"
                    style={{ background: e.dot }}
                  />
                  <span className="truncate text-[13px] font-medium text-foreground">{e.name}</span>
                  {e.project ? (
                    <span className="truncate font-mono text-[11px] text-muted-foreground">{e.project}</span>
                  ) : null}
                  <span className="ml-auto flex shrink-0 items-baseline gap-2">
                    {e.tag ? (
                      <span
                        className={cn(
                          "rounded px-1.5 py-px text-[10px] font-medium uppercase tracking-wide",
                          e.tag === "current"
                            ? "bg-primary/10 text-primary"
                            : "bg-muted text-muted-foreground",
                        )}
                      >
                        {e.tag}
                      </span>
                    ) : null}
                    <span className="text-[11px] tabular-nums text-muted-foreground">{e.meta}</span>
                  </span>
                </span>
                {e.preview ? (
                  <span className="truncate pl-3.5 text-[12px] text-muted-foreground">{e.preview}</span>
                ) : null}
              </button>
            ))
          )}
        </div>

        {entries.length > 0 ? (
          <div className="border-t border-border px-5 py-2 text-[11px] text-muted-foreground">
            {resumableCount} resumable
          </div>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
