import { useId, useState } from "react";
import { cn } from "@/lib/utils";

/**
 * Parts-of-whole for ORDERED categories (plan tiers run trial → ultra), so the ramp is
 * SEQUENTIAL zinc, not categorical — this design system has no categorical palette, and
 * inventing one would collide with the reserved success/destructive colours.
 *
 * Validated with the dataviz validator against the card surface: CVD separation PASS
 * (worst adjacent ΔE 15.9), normal-vision PASS. Its lightness-band and chroma FAILs are
 * CATEGORICAL criteria — a sequential ramp is judged on lightness monotonicity, which the zinc
 * scale satisfies by construction. The contrast WARN on the two lightest steps obligates relief,
 * which is why every slice is ALSO directly labelled in the legend with its count and share:
 * identity is never carried by colour alone.
 *
 * A 2px surface gap separates adjacent arcs (mark spec), and hover is per-slice.
 */
export type Slice = { label: string; value: number };

const RAMP = ["#18181b", "#3f3f46", "#71717a", "#a1a1aa", "#d4d4d8", "#e4e4e7"];

export function Donut({
  slices, total, centerLabel, centerValue, className,
}: {
  slices: Slice[]; total?: number;
  centerLabel?: string; centerValue?: string; className?: string;
}) {
  const uid = useId();
  const [hover, setHover] = useState<number | null>(null);
  const sum = total ?? slices.reduce((a, s) => a + s.value, 0);
  const R = 56, SW = 16, C = 2 * Math.PI * R;
  // 2px of surface between arcs; skip the gap when a slice is the entire ring.
  const GAP = 2;

  let offset = 0;
  const arcs = slices.map((s, i) => {
    const frac = sum > 0 ? s.value / sum : 0;
    const len = Math.max(0, frac * C - (frac < 1 ? GAP : 0));
    const arc = { ...s, i, frac, len, offset, color: RAMP[Math.min(i, RAMP.length - 1)] };
    offset += frac * C;
    return arc;
  });

  return (
    <div className={cn("flex items-center gap-6", className)}>
      <div className="relative shrink-0">
        <svg width={144} height={144} viewBox="0 0 144 144" role="img"
             aria-labelledby={`${uid}-t`} className="-rotate-90">
          <title id={`${uid}-t`}>
            {slices.map((s) => `${s.label} ${s.value}`).join("，")}
          </title>
          <circle cx={72} cy={72} r={R} fill="none" stroke="var(--color-secondary)" strokeWidth={SW} />
          {arcs.map((a) => (
            <circle
              key={a.label} cx={72} cy={72} r={R} fill="none"
              stroke={a.color} strokeWidth={SW}
              strokeDasharray={`${a.len} ${C - a.len}`}
              strokeDashoffset={-a.offset}
              className="transition-opacity duration-150"
              opacity={hover === null || hover === a.i ? 1 : 0.35}
              onMouseEnter={() => setHover(a.i)}
              onMouseLeave={() => setHover(null)}
            />
          ))}
        </svg>
        <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
          <span className="font-display text-2xl font-semibold">{centerValue ?? sum}</span>
          {centerLabel && <span className="type-eyebrow mt-0.5">{centerLabel}</span>}
        </div>
      </div>
      <ul className="min-w-0 flex-1 space-y-1.5">
        {arcs.map((a) => (
          <li
            key={a.label}
            onMouseEnter={() => setHover(a.i)} onMouseLeave={() => setHover(null)}
            className={cn(
              "flex items-center gap-2.5 rounded-md px-1.5 py-1 text-sm transition-colors",
              hover === a.i && "bg-secondary/60",
            )}
          >
            <span className="size-2.5 shrink-0 rounded-[3px] ring-1 ring-border"
                  style={{ background: a.color }} aria-hidden />
            <span className="min-w-0 flex-1 truncate text-muted-foreground">{a.label}</span>
            <span className="shrink-0 tabular-nums">{a.value}</span>
            <span className="w-11 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
              {sum > 0 ? `${Math.round(a.frac * 100)}%` : "—"}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
