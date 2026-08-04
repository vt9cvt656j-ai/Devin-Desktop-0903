import { cn } from "@/lib/utils";

/**
 * 加载中的表格。改之前这些屏在加载时渲染的是"什么都没有"——网速一慢，一块空白卡片
 * 和"这个后台坏了"在屏幕上长得一模一样。
 *
 * 骨架按真实列宽排，不是一排等宽灰条：占位和真数据错位的话，数据落地那一下整张表会跳。
 * 行透明度往下递减，视觉重量集中在第一行，也顺手表达了"下面还有，别急"。
 *
 * 用 animate-pulse（Tailwind 自带，无依赖）；prefers-reduced-motion 下 index.css 把它关掉，
 * 骨架变成静态占位——占位的作用本来就不是动，是先把版面撑住。
 */
export function TableSkeleton({
  rows = 5,
  columns = ["28%", "12%", "16%", "18%", "10%"],
  label = "加载中",
  className,
}: {
  rows?: number;
  /** 每列宽度，直接照抄这张表真实的列比例。 */
  columns?: string[];
  label?: string;
  className?: string;
}) {
  return (
    <div
      role="status"
      aria-busy="true"
      aria-live="polite"
      className={cn("divide-y divide-border", className)}
    >
      <span className="sr-only">{label}</span>
      {Array.from({ length: rows }, (_, r) => (
        <div
          key={r}
          className="flex items-center gap-4 px-4 py-4"
          style={{ opacity: Math.max(0.35, 1 - r * 0.14) }}
        >
          {columns.map((width, c) => (
            <div
              key={c}
              aria-hidden
              className="h-3.5 animate-pulse rounded bg-secondary"
              style={{ width, animationDelay: `${(r * columns.length + c) * 40}ms` }}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
