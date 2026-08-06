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
  // className is applied to BOTH the wrapper and the <select>.
  //
  // The wrapper used to be hardcoded w-full, so a caller passing w-44 styled an element that was
  // still stretched to full width by its parent — which silently forced every toolbar containing
  // a Select to wrap onto its own row. Moving className to the wrapper fixed that but broke the
  // other half: callers also pass text-sm and padding, which only mean anything on the control.
  // Splitting them by guessing at prefixes would be worse than applying both — h-9/w-44 on the
  // wrapper sizes the box, the same classes on the select fill it exactly, and text/padding land
  // where they were meant to.
  return (
    <div className={cn("relative h-12 w-full", className)}>
      <select
        data-slot="select"
        className={cn(
          "h-full w-full cursor-pointer appearance-none rounded-lg border border-input bg-card pl-4 pr-10 text-base text-foreground transition-colors outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/25 disabled:cursor-not-allowed disabled:opacity-50 aria-[invalid=true]:border-destructive",
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
