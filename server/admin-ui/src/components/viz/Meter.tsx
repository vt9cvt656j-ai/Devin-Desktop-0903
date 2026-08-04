import { cn } from "@/lib/utils";

/**
 * A single ratio against a limit. The dataviz reference is explicit that this is a METER, not a
 * two-slice pie — so the ring is reserved for genuine parts-of-whole and ratios get a track.
 *
 * The unfilled track is a lighter step of the SAME ramp so state reads across the whole bar, and
 * the fill carries severity: neutral until it matters, then the reserved destructive token once
 * the limit is effectively spent. Severity is never colour-alone — the value is always printed.
 */
export function Meter({
  used, cap, label, className,
}: { used: number; cap: number; label?: string; className?: string }) {
  const pct = cap > 0 ? Math.min(100, Math.max(0, (used / cap) * 100)) : 0;
  const spent = cap > 0 && used >= cap;
  const tight = pct >= 90;
  return (
    <div className={cn("min-w-0", className)}>
      {label && <div className="type-eyebrow mb-1">{label}</div>}
      <div
        className="h-1.5 w-full overflow-hidden rounded-full bg-secondary"
        role="progressbar" aria-valuenow={Math.round(pct)} aria-valuemin={0} aria-valuemax={100}
      >
        <div
          className={cn(
            "h-full rounded-full transition-[width] duration-300 motion-reduce:transition-none",
            spent || tight ? "bg-destructive" : "bg-foreground",
          )}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}
