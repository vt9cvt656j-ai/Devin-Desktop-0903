import type { ReactNode } from "react";
import { RefreshCw, Search } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";

/**
 * The filter row every list screen needs: search, some filters, a count, refresh.
 *
 * Extracted because four pages were each hand-rolling it with slightly different heights,
 * gaps and orderings — the kind of drift that makes a console feel assembled rather than
 * designed. One row, one height, one place to change it.
 */
export function Toolbar({
  query, onQuery, placeholder, count, onRefresh, children, className,
}: {
  query?: string;
  onQuery?: (v: string) => void;
  placeholder?: string;
  /** Right-aligned count, e.g. "120 位" or "12 / 120". */
  count?: ReactNode;
  onRefresh?: () => void;
  /** Filters — Selects and the like. Each keeps its own width. */
  children?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-nowrap items-center gap-2 overflow-x-auto rounded-xl border border-border bg-card p-2",
        className,
      )}
    >
      {onQuery && (
        <div className="relative min-w-[12rem] flex-1">
          <Search
            aria-hidden
            className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            className="h-11 w-full border-transparent bg-transparent pl-9 focus-visible:border-ring"
            value={query ?? ""}
            onChange={(e) => onQuery(e.target.value)}
            placeholder={placeholder}
            aria-label={placeholder || "搜索"}
          />
        </div>
      )}
      {children && (
        <>
          <Separator orientation="vertical" className="hidden h-6 shrink-0 sm:block" />
          <div className="flex shrink-0 items-center gap-2">{children}</div>
        </>
      )}
      {count != null && (
        <span
          className="ml-auto shrink-0 whitespace-nowrap px-2 text-sm tabular-nums text-muted-foreground"
          aria-live="polite"
        >
          {count}
        </span>
      )}
      {onRefresh && (
        <Button
          variant="outline"
          className="h-11 w-11 shrink-0 p-0"
          onClick={onRefresh}
          aria-label="刷新"
          title="刷新"
        >
          <RefreshCw className="size-4" />
        </Button>
      )}
    </div>
  );
}
