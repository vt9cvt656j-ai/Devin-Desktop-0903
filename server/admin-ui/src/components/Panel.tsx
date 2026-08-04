import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * A titled card. Defined identically in Pricing.tsx and Billing.tsx (differing only in whether
 * the header slot was called `aside` or `action`) and hand-inlined a third time — three copies of
 * the same eight lines, which is how the console's cards drifted apart in the first place.
 *
 * Deliberately not the vendored shadcn Card: it is imported by zero files here, its slot names
 * (CardHeader/CardTitle/CardContent) do not match how this console composes a titled panel, and
 * swapping 17 hand-rolled surfaces onto it is a bigger change than this one is meant to be. What
 * matters now is that there is ONE of these, not which primitive it wraps.
 */
export function Panel({
  title, aside, children, className, bodyClassName,
}: {
  title: ReactNode;
  /** Right side of the header — a count, a filter, an action. */
  aside?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
}) {
  return (
    <section className={cn("rounded-xl border border-border bg-card", className)}>
      <header className="flex items-center justify-between gap-4 border-b border-border px-5 py-3">
        <h2 className="text-sm font-semibold">{title}</h2>
        {aside}
      </header>
      <div className={bodyClassName}>{children}</div>
    </section>
  );
}
