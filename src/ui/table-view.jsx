import { useCallback, useMemo, useState } from "react";
import { cn } from "./lib/cn.js";

/**
 * The CSV/TSV window — a spreadsheet, not a wall of characters.
 *
 * A `.csv` used to open in the code editor. Technically correct, practically useless: the header
 * is just the first line, quoted fields containing commas look broken, and the questions you open
 * a spreadsheet to answer — which columns, how many rows, what is in column 7 — all take counting.
 *
 * Same shape as the archive window, for the same reason: title, toolbar, column headers, the grid,
 * a status bar. Numbers right-align because that is how you compare them by eye. The dialect and
 * encoding the parser settled on are printed in the status bar, because they are guesses and the
 * reader is the only one who can tell whether they were right.
 *
 * Presentational — main.js owns the parse and the actions.
 */

const ALIGN = { number: "text-right tabular-nums", date: "tabular-nums", text: "" };

export function TableView({ table, file, iconFor, onOpenAsText, onReveal }) {
  const [sort, setSort] = useState(null); // null = file order, which is meaningful in a CSV
  const [query, setQuery] = useState("");

  const columns = Array.isArray(table?.columns) ? table.columns : [];
  const allRows = Array.isArray(table?.rows) ? table.rows : [];

  const rows = useMemo(() => {
    const q = query.trim().toLowerCase();
    let out = q ? allRows.filter((r) => r.some((cell) => String(cell).toLowerCase().includes(q))) : allRows;
    if (sort) {
      const { index, dir } = sort;
      const numeric = columns[index]?.kind === "number";
      out = [...out].sort((a, b) => {
        const x = a[index] ?? "";
        const y = b[index] ?? "";
        if (numeric) {
          const nx = parseFloat(String(x).replace(/[^0-9.eE+-]/g, ""));
          const ny = parseFloat(String(y).replace(/[^0-9.eE+-]/g, ""));
          if (Number.isNaN(nx) && Number.isNaN(ny)) return 0;
          if (Number.isNaN(nx)) return 1; // blanks sink, whichever way you sort
          if (Number.isNaN(ny)) return -1;
          return dir * (nx - ny);
        }
        return dir * String(x).localeCompare(String(y), undefined, { numeric: true });
      });
    }
    return out;
  }, [allRows, columns, sort, query]);

  // Cycles file order → ascending → descending → file order. A CSV's own order often carries
  // meaning, so it has to be reachable again without reopening the file.
  const cycleSort = useCallback((index) => {
    setSort((s) => {
      if (!s || s.index !== index) return { index, dir: 1 };
      if (s.dir === 1) return { index, dir: -1 };
      return null;
    });
  }, []);

  return (
    <div className="ui-island flex h-full min-h-0 flex-col bg-[var(--panel-solid)] text-foreground">
      <div className="flex flex-none items-center gap-3 border-b border-border px-4 py-2.5">
        {iconFor ? (
          <img src={iconFor(file?.name || "data.csv", false)} alt="" draggable={false} className="size-7 shrink-0" />
        ) : null}
        <div className="min-w-0 flex-1">
          <div className="truncate text-[13px] font-medium" title={file?.name} data-i18n-skip>
            {file?.name || "表格"}
          </div>
          <div className="truncate text-[11px] tabular-nums text-muted-foreground">
            {Number(table?.total_rows || 0).toLocaleString()} 行 × {columns.length} 列
            {table?.count_is_partial ? "（至少）" : ""}
          </div>
        </div>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="筛选…"
          className="h-9 w-44 flex-none rounded-full border border-border bg-transparent px-4 text-[13px] outline-none placeholder:text-muted-foreground focus:border-primary"
        />
        {onOpenAsText ? (
          <button
            type="button"
            onClick={onOpenAsText}
            className="h-9 flex-none rounded-full border border-border px-4 text-[13px] font-medium text-primary transition-colors hover:bg-primary/10"
          >
            以文本打开
          </button>
        ) : null}
        {onReveal ? (
          <button
            type="button"
            onClick={onReveal}
            className="h-9 flex-none rounded-full border border-border px-4 text-[13px] font-medium transition-colors hover:bg-accent"
          >
            在系统中显示
          </button>
        ) : null}
      </div>

      {table?.note ? (
        <p className="flex-none border-b border-border bg-muted/40 px-4 py-2 text-[12px] leading-relaxed text-muted-foreground">
          {table.note}
        </p>
      ) : null}

      <div className="min-h-0 flex-1 overflow-auto">
        <table className="w-max min-w-full border-collapse text-[12px]">
          <thead className="sticky top-0 z-10 bg-[var(--panel-solid)]">
            <tr className="border-b border-border">
              {/* Row numbers are the spreadsheet's own addressing — without them "the third row"
                  means counting, and a sorted view makes counting wrong. */}
              <th className="sticky left-0 z-20 w-12 bg-[var(--panel-solid)] px-3 py-2 text-right text-[11px] font-normal text-muted-foreground">
                #
              </th>
              {columns.map((col, i) => (
                <th
                  key={`${col.name}-${i}`}
                  onClick={() => cycleSort(i)}
                  title={`${col.name} · ${col.kind}`}
                  className={cn(
                    "cursor-pointer select-none whitespace-nowrap px-3 py-2 text-[12px] font-medium text-muted-foreground transition-colors hover:text-foreground",
                    col.kind === "number" ? "text-right" : "text-left",
                  )}
                >
                  <span data-i18n-skip>{col.name}</span>
                  <span className="ml-1 inline-block w-2">
                    {sort?.index === i ? (sort.dir > 0 ? "▲" : "▼") : ""}
                  </span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr>
                <td colSpan={columns.length + 1} className="px-4 py-10 text-center text-muted-foreground">
                  {query ? "没有匹配的行" : "这个表格是空的"}
                </td>
              </tr>
            ) : rows.map((row, r) => (
              <tr key={r} className="border-b border-border/40 last:border-0 hover:bg-foreground/[0.04]">
                <td className="sticky left-0 z-10 bg-[var(--panel-solid)] px-3 py-1.5 text-right text-[11px] tabular-nums text-muted-foreground">
                  {r + 1}
                </td>
                {columns.map((col, c) => (
                  <td
                    key={c}
                    className={cn("max-w-[420px] truncate px-3 py-1.5", ALIGN[col.kind] || "")}
                    title={row[c] || ""}
                    data-i18n-skip
                  >
                    {row[c]}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* The guesses go here, where they can be checked: a mojibake column or a one-column table
          is almost always one of these two being wrong. */}
      <div className="flex flex-none items-center gap-4 border-t border-border bg-muted/30 px-4 py-1.5 text-[11px] text-muted-foreground">
        <span className="tabular-nums">
          {query ? `${rows.length.toLocaleString()} / ` : ""}
          {Number(table?.total_rows || 0).toLocaleString()} 行
          {table?.truncated ? `（已载入 ${allRows.length.toLocaleString()}）` : ""}
        </span>
        <span className="ml-auto tabular-nums" data-i18n-skip>
          分隔符 {table?.delimiter} · {table?.encoding}
          {table?.has_header ? " · 首行为表头" : " · 无表头"}
        </span>
      </div>
    </div>
  );
}
