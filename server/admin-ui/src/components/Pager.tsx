import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { num } from "@/lib/format";

/**
 * 表格底部的翻页条。
 *
 * 抽出来是因为它已经在四张表上各抄了一遍（模型对账两张、客户、收款）。抄的东西会漂：
 * 其中一份的「共 N 条」漏了千分位、另一份的按钮在最后一页没置灰，都是抄出来的差异。
 *
 * 只有一页时**整个不渲染** —— 一个两端都置灰的翻页条只是噪音，还会让人以为翻不动是坏了。
 */
export function Pager({
  page,
  pages,
  total,
  unit,
  onPage,
}: {
  page: number;
  pages: number;
  total: number;
  /** 计数的量词：「位」「笔」「个」。 */
  unit: string;
  onPage: (next: number) => void;
}) {
  if (pages <= 1) return null;
  return (
    <div className="flex items-center justify-center gap-2 border-t border-border px-5 py-3 text-xs text-muted-foreground">
      <Button size="sm" variant="outline" disabled={page <= 1} onClick={() => onPage(Math.max(1, page - 1))}>
        <ChevronLeft className="h-3.5 w-3.5" /> 上一页
      </Button>
      <span className="tabular-nums">
        第 {page} / {pages} 页 · 共 {num(total)} {unit}
      </span>
      <Button size="sm" variant="outline" disabled={page >= pages} onClick={() => onPage(Math.min(pages, page + 1))}>
        下一页 <ChevronRight className="h-3.5 w-3.5" />
      </Button>
    </div>
  );
}

/** 一页多少行。四张表统一，改这里就是全改。 */
export const PAGE_SIZE = 20;

/** 从一个已经筛好的数组里取当页。页码越界时夹回最后一页，而不是给一页空表。 */
export function paginate<T>(rows: T[], page: number, size = PAGE_SIZE) {
  const pages = Math.max(1, Math.ceil(rows.length / size));
  const current = Math.min(Math.max(1, page), pages);
  return { pages, current, slice: rows.slice((current - 1) * size, current * size) };
}
