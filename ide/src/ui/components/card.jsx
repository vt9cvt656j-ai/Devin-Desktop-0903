import { cn } from "../lib/cn.js";

/**
 * Card —— shadcn 的卡片族。
 *
 * 观感全部来自语义槽（bg-card / border-border / text-muted-foreground），而这些槽在
 * tailwind.css 里指向项目自己的 token，所以卡片在浅色下是 --panel-2 (#fbfbfd)、
 * 深色下是 #1f1f23，跟着 data-theme 自动切，配色一个都没新加。
 *
 * 结构照搬 shadcn：Card > CardHeader(CardTitle + CardDescription) > CardContent > CardFooter，
 * 这样以后从 shadcn 抄任何卡片布局都能直接用。
 */
export function Card({ className, ...props }) {
  return (
    <div
      data-slot="card"
      className={cn(
        "flex flex-col gap-6 rounded-xl border border-border bg-card py-6 text-card-foreground shadow-sm",
        className,
      )}
      {...props}
    />
  );
}

export function CardHeader({ className, ...props }) {
  return (
    <div
      data-slot="card-header"
      className={cn("flex flex-col gap-1.5 px-6", className)}
      {...props}
    />
  );
}

export function CardTitle({ className, ...props }) {
  return (
    <div
      data-slot="card-title"
      className={cn("font-semibold leading-none tracking-tight", className)}
      {...props}
    />
  );
}

export function CardDescription({ className, ...props }) {
  return (
    <div
      data-slot="card-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

export function CardAction({ className, ...props }) {
  return (
    <div
      data-slot="card-action"
      className={cn("col-start-2 row-span-2 row-start-1 self-start justify-self-end", className)}
      {...props}
    />
  );
}

export function CardContent({ className, ...props }) {
  return <div data-slot="card-content" className={cn("px-6", className)} {...props} />;
}

export function CardFooter({ className, ...props }) {
  return (
    <div
      data-slot="card-footer"
      className={cn("flex items-center px-6 [.border-t]:pt-6", className)}
      {...props}
    />
  );
}
