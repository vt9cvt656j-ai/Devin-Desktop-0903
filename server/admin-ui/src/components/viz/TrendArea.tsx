import { useId } from "react";
import { cn } from "@/lib/utils";

/**
 * Trend over time for ONE series — so: area, single hue, no legend (the title names it), and a
 * recessive baseline. Deliberately not a line-with-markers: at 14 daily points markers would be
 * noise, and the reference reserves ≥8px markers for series you must pick individual points off.
 *
 * Selective labels only — first, last and the peak — never a number on every point.
 */
export function TrendArea({
  points, label, valueFormat = (n: number) => String(n), className,
}: {
  points: { t: string; v: number }[];
  label: string;
  valueFormat?: (n: number) => string;
  className?: string;
}) {
  const uid = useId();
  const W = 560, H = 120, PAD = 6;
  const max = Math.max(1, ...points.map((p) => p.v));
  const n = Math.max(1, points.length - 1);
  const x = (i: number) => PAD + (i / n) * (W - PAD * 2);
  const y = (v: number) => H - PAD - (v / max) * (H - PAD * 2);

  const line = points.map((p, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(p.v).toFixed(1)}`).join(" ");
  const area = points.length
    ? `${line} L${x(points.length - 1).toFixed(1)},${H - PAD} L${x(0).toFixed(1)},${H - PAD} Z`
    : "";
  const peak = points.reduce((best, p, i) => (p.v > points[best].v ? i : best), 0);

  return (
    <div className={cn("min-w-0", className)}>
      <svg viewBox={`0 0 ${W} ${H}`} className="h-28 w-full" role="img" aria-labelledby={`${uid}-t`}>
        <title id={`${uid}-t`}>{label}</title>
        <defs>
          <linearGradient id={`${uid}-g`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--color-foreground)" stopOpacity="0.14" />
            <stop offset="100%" stopColor="var(--color-foreground)" stopOpacity="0" />
          </linearGradient>
        </defs>
        {area && <path d={area} fill={`url(#${uid}-g)`} />}
        {points.length > 1 && (
          <path d={line} fill="none" stroke="var(--color-foreground)" strokeWidth={2}
                strokeLinecap="round" strokeLinejoin="round" />
        )}
        <line x1={PAD} y1={H - PAD} x2={W - PAD} y2={H - PAD}
              stroke="var(--color-border)" strokeWidth={1} />
        {points.length > 0 && max > 0 && (
          <>
            <circle cx={x(peak)} cy={y(points[peak].v)} r={3.5}
                    fill="var(--color-card)" stroke="var(--color-foreground)" strokeWidth={2} />
            <text x={x(peak)} y={Math.max(12, y(points[peak].v) - 8)} textAnchor="middle"
                  className="fill-muted-foreground text-[10px]">
              {valueFormat(points[peak].v)}
            </text>
          </>
        )}
      </svg>
      <div className="flex justify-between px-1.5 text-xs text-muted-foreground">
        <span>{points[0]?.t ?? ""}</span>
        <span>{points[points.length - 1]?.t ?? ""}</span>
      </div>
    </div>
  );
}
