import type { ComponentProps, ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * 密度：这几张表是照着空数据排的版，装上真行之后问题全在同一处 —— 一个 84 字符的邮箱、
 * 一个 40 字符的 model id、一个八位数的金额，会把某一列撑到把别的列挤没。三条对策，
 * 全部收在这一层，页面不用再各写各的：
 *
 *  - min-w-[46rem] 是"开始横向滚动"的下限。窄于它就滚动，而不是把每一格压成竖排单字。
 *    需要更宽的表（客户 7 列、模型线路 6 列）在自己那边 className 覆盖成更大的值。
 *  - numeric 一个属性同时给出右对齐 + 等宽数字 + 不换行。金额和次数必须右对齐才能一眼
 *    比大小，也不能因为多两位数就折行。
 *  - <Truncate> 把"截断"和"title"绑成一件事。分开写的结果一定是某天有人只写了 truncate，
 *    于是那个客户的邮箱在这台机器上永远看不全。
 */
export function Table({ className, ...props }: ComponentProps<"table">) {
  return (
    <div className="w-full overflow-x-auto">
      <table
        data-slot="table"
        className={cn("w-full min-w-[46rem] caption-bottom text-sm", className)}
        {...props}
      />
    </div>
  );
}

export function TableHeader({ className, ...props }: ComponentProps<"thead">) {
  return (
    <thead
      data-slot="table-header"
      className={cn("[&_tr]:border-b [&_tr]:border-border", className)}
      {...props}
    />
  );
}

export function TableBody({ className, ...props }: ComponentProps<"tbody">) {
  return (
    <tbody
      data-slot="table-body"
      className={cn("[&_tr:last-child]:border-0", className)}
      {...props}
    />
  );
}

export function TableRow({ className, ...props }: ComponentProps<"tr">) {
  return (
    <tr
      data-slot="table-row"
      className={cn(
        "border-b border-border transition-colors hover:bg-muted/70",
        className,
      )}
      {...props}
    />
  );
}

export function TableHead({
  className,
  numeric,
  ...props
}: ComponentProps<"th"> & { numeric?: boolean }) {
  return (
    <th
      data-slot="table-head"
      className={cn(
        "whitespace-nowrap px-4 py-3 text-left align-middle text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground",
        numeric && "text-right",
        className,
      )}
      {...props}
    />
  );
}

export function TableCell({
  className,
  numeric,
  ...props
}: ComponentProps<"td"> & { numeric?: boolean }) {
  return (
    <td
      data-slot="table-cell"
      className={cn(
        // leading-relaxed（1.625）是正文行高，装在双行单元格里会把行高撑到 76px。
        // 一屏能多看四行，就少翻一次。
        "px-4 py-3 align-middle leading-snug",
        numeric && "whitespace-nowrap text-right tabular-nums",
        className,
      )}
      {...props}
    />
  );
}

/**
 * 单行截断 + title。title 默认取纯文本 children，所以"截断了但鼠标悬停看不到全文"
 * 这件事在结构上就不会发生；children 不是纯文本时必须显式传 title。
 * 只在给了宽度上限的列里有效 —— 宽度约束是列的事（TableHead 上的 w-/max-w-），不是它的事。
 */
export function Truncate({
  children,
  title,
  className,
}: {
  children: ReactNode;
  title?: string;
  className?: string;
}) {
  const text =
    title ?? (typeof children === "string" || typeof children === "number" ? String(children) : undefined);
  return (
    <div className={cn("truncate", className)} title={text}>
      {children}
    </div>
  );
}
