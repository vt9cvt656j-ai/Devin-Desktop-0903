import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * 出错了怎么说。两种形态，因为后台真的只有两种错：
 *
 *  - inline：屏还在，只是刚才那个动作失败了。一行 text-destructive，贴在标题下面。
 *  - block：这块内容根本没加载出来。占住表格的位置，带一个"重试"，不然操作员只能刷新整页。
 *
 * message 为空时返回 null，所以调用方可以直接 <ErrorState message={err} /> 写死在
 * 结构里，不用再套一层 {err && …} —— 少一个条件分支，就少一处"错误显示了但按钮还亮着"。
 *
 * 颜色只用 destructive：这套 token 里没有 warning/amber，也不该为了"半个错误"临时造一个。
 */
export function ErrorState({
  message,
  hint,
  onRetry,
  retryLabel = "重试",
  variant = "inline",
  className,
}: {
  message?: ReactNode;
  hint?: ReactNode;
  onRetry?: () => void;
  retryLabel?: string;
  variant?: "inline" | "block";
  className?: string;
}) {
  if (!message) return null;

  if (variant === "inline") {
    return (
      <p role="alert" className={cn("text-sm text-destructive", className)}>
        {message}
        {hint && <span className="ml-1 text-muted-foreground">{hint}</span>}
        {onRetry && (
          <button
            type="button"
            onClick={onRetry}
            className="ml-2 rounded underline underline-offset-4 transition-colors hover:text-foreground"
          >
            {retryLabel}
          </button>
        )}
      </p>
    );
  }

  return (
    <div
      role="alert"
      className={cn("flex flex-col items-center gap-2 px-5 py-12 text-center", className)}
    >
      <p className="text-sm font-medium text-destructive">{message}</p>
      {hint && <p className="type-measure text-sm text-muted-foreground">{hint}</p>}
      {onRetry && (
        <Button variant="outline" size="sm" className="mt-2" onClick={onRetry}>
          {retryLabel}
        </Button>
      )}
    </div>
  );
}
