import type { ComponentProps } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * A styled NATIVE <select>, matching Input's house style exactly.
 *
 * Deliberately not @radix-ui/react-select: that would be the only Radix package not already in
 * this project's dependencies, and it buys nothing here — the admin's 19 selects are all short
 * flat lists. A native select is also the better control on mobile and needs no portal, no focus
 * trap and no keyboard reimplementation.
 */
export function Select({ className, children, ...props }: ComponentProps<"select">) {
  return (
    <div className="relative w-full">
      <select
        data-slot="select"
        className={cn(
          "h-12 w-full appearance-none rounded-lg border border-input bg-card pl-4 pr-10 text-base text-foreground transition-colors outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/25 disabled:cursor-not-allowed disabled:opacity-50 aria-[invalid=true]:border-destructive",
          className,
        )}
        {...props}
      >
        {children}
      </select>
      <ChevronDown
        aria-hidden
        className="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
      />
    </div>
  );
}
