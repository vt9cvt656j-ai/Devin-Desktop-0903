import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * 空表格里那一行字。六个页面写过八种不同的版本（px-5 py-8 / py-10 / py-12，
 * 有的居中有的不居中），说的却都是同一件事。
 *
 * 一条规矩：title 说"现在是什么状况"，hint 说"那该怎么办"。只有 title 的空状态
 * 等于把人晾在原地 —— "暂无兑换码"之后应该跟着"在上面生成一批"。
 */
export function EmptyState({
  title,
  hint,
  action,
  icon: Icon,
  compact,
  className,
}: {
  title: ReactNode;
  hint?: ReactNode;
  /** 通常是一个把人送去下一步的按钮，不是装饰。 */
  action?: ReactNode;
  icon?: LucideIcon;
  /** 面板里的小列表用 compact，整屏的空表格用默认。 */
  compact?: boolean;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center gap-1.5 px-5 text-center",
        compact ? "py-8" : "py-12",
        className,
      )}
    >
      {Icon && <Icon aria-hidden className="mb-1 size-5 text-muted-foreground/60" />}
      <p className="text-sm font-medium">{title}</p>
      {hint && <p className="type-measure text-sm text-muted-foreground">{hint}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}
