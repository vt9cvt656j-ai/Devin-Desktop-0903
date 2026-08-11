import { useCallback, useMemo, useState } from "react";
import { cn } from "./lib/cn.js";

/**
 * The archive window. Opening a `.zip` gives you this and nothing else.
 *
 * What it replaced, twice over. First a flat table of every entry — 1,652 rows for a real build
 * artefact, full path repeated on each, no structure. Then the same table with a browser bolted
 * beneath a generic file report: MIME type, read-only flag, six fact cards, a row of chips, a hex
 * dump. None of that is what you opened an archive to find out. The hex panel had already learned
 * this lesson and dropped its own header for the same reason; archives had not.
 *
 * So it is laid out the way archive software has been laid out for thirty years, because that
 * shape is correct: title, toolbar, address bar, list, status bar. Folders aggregate everything
 * beneath them, so the first screen answers "what is big in here" without a single expansion.
 *
 * Presentational — main.js owns the listing and the actions.
 */

function formatBytes(n) {
  const v = Number(n) || 0;
  if (v < 1024) return `${v} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = v / 1024;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) { value /= 1024; i += 1; }
  return `${value >= 10 || Number.isInteger(value) ? Math.round(value) : value.toFixed(1)} ${units[i]}`;
}

/** Immediate children of `cwd`, folders rolling up everything beneath them. */
function levelAt(entries, cwd) {
  const prefix = cwd ? `${cwd}/` : "";
  const folders = new Map();
  const files = [];
  for (const entry of entries) {
    const name = String(entry?.name || "").replace(/\/+$/, "");
    if (!name || (prefix && !name.startsWith(prefix))) continue;
    const rest = name.slice(prefix.length);
    if (!rest) continue;
    const slash = rest.indexOf("/");
    if (slash === -1) {
      if (!entry?.is_dir) files.push({ ...entry, label: rest, isDir: false });
    } else {
      const label = rest.slice(0, slash);
      const folder = folders.get(label) || { label, isDir: true, size: 0, compressed: 0, count: 0 };
      folder.size += Number(entry?.size) || 0;
      folder.compressed += Number(entry?.compressed_size) || 0;
      if (!entry?.is_dir) folder.count += 1;
      folders.set(label, folder);
    }
  }
  return [...folders.values(), ...files];
}

const COLUMNS = [
  { key: "label", label: "名称", align: "left" },
  { key: "size", label: "大小", align: "right", width: "w-28" },
  { key: "compressed", label: "压缩后", align: "right", width: "w-28" },
  { key: "ratio", label: "压缩率", align: "right", width: "w-20" },
];

export function ArchiveBrowser({ archive, file, onOpenEntry, onExtract, onReveal }) {
  const [cwd, setCwd] = useState("");
  const [sort, setSort] = useState({ key: "label", dir: 1 });
  const [selected, setSelected] = useState(null);
  const entries = Array.isArray(archive?.entries) ? archive.entries : [];

  const rows = useMemo(() => {
    const level = levelAt(entries, cwd).map((row) => ({
      ...row,
      size: Number(row.size) || 0,
      compressed: Number(row.compressed ?? row.compressed_size) || 0,
    }));
    const { key, dir } = sort;
    return level.sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1; // folders first, always
      if (key === "label") return dir * a.label.localeCompare(b.label, undefined, { numeric: true });
      if (key === "ratio") {
        const ra = a.size ? a.compressed / a.size : 0;
        const rb = b.size ? b.compressed / b.size : 0;
        return dir * (ra - rb);
      }
      return dir * ((a[key] || 0) - (b[key] || 0));
    });
  }, [entries, cwd, sort]);

  const crumbs = cwd ? cwd.split("/") : [];
  const here = useMemo(() => {
    const files = rows.filter((r) => !r.isDir).length;
    return { files, dirs: rows.length - files, size: rows.reduce((n, r) => n + r.size, 0) };
  }, [rows]);

  const enter = useCallback((label) => { setCwd((c) => (c ? `${c}/${label}` : label)); setSelected(null); }, []);
  const toggleSort = useCallback((key) => {
    setSort((s) => (s.key === key ? { key, dir: -s.dir } : { key, dir: key === "label" ? 1 : -1 }));
  }, []);

  return (
    <div className="ui-island flex h-full min-h-0 flex-col bg-background text-foreground">
      {/* Title bar: the archive itself. Name, what it weighs, what it unpacks to. */}
      <div className="flex flex-none items-center gap-3 border-b border-border px-4 py-2.5">
        <span className="text-[15px]" aria-hidden>🗜️</span>
        <div className="min-w-0 flex-1">
          <div className="truncate text-[13px] font-medium" title={file?.name} data-i18n-skip>
            {file?.name || "压缩包"}
          </div>
          <div className="truncate text-[11px] text-muted-foreground tabular-nums">
            <span className="uppercase" data-i18n-skip>{archive?.format || "archive"}</span>
            {" · "}{formatBytes(file?.size)}
            {archive?.total_size ? ` · 解压后 ${formatBytes(archive.total_size)}` : ""}
            {archive?.encrypted ? " · 🔒 含加密条目" : ""}
          </div>
        </div>
        <div className="flex flex-none items-center gap-2">
          {onReveal ? (
            <button
              type="button"
              onClick={onReveal}
              className="h-8 rounded-lg border border-border px-3 text-[12px] hover:bg-accent"
            >
              在系统中显示
            </button>
          ) : null}
          <button
            type="button"
            onClick={() => onExtract?.()}
            className="h-8 rounded-lg bg-primary px-3.5 text-[12px] font-medium text-primary-foreground hover:opacity-90"
          >
            全部解压…
          </button>
        </div>
      </div>

      {/* Address bar. Climbing back out is one click per level. */}
      <div className="flex flex-none items-center gap-0.5 overflow-x-auto border-b border-border bg-muted/30 px-3 py-1.5 text-[12px]">
        <button
          type="button"
          onClick={() => { setCwd(""); setSelected(null); }}
          className={cn("flex-none rounded px-1.5 py-0.5 hover:bg-accent", cwd ? "text-primary" : "font-medium")}
        >
          根目录
        </button>
        {crumbs.map((part, i) => (
          <span key={`${part}-${i}`} className="flex flex-none items-center gap-0.5">
            <span className="text-muted-foreground">›</span>
            <button
              type="button"
              onClick={() => { setCwd(crumbs.slice(0, i + 1).join("/")); setSelected(null); }}
              className={cn(
                "max-w-[220px] truncate rounded px-1.5 py-0.5 hover:bg-accent",
                i === crumbs.length - 1 ? "font-medium" : "text-primary",
              )}
              data-i18n-skip
            >
              {part}
            </button>
          </span>
        ))}
      </div>

      {archive?.note ? (
        <p className="flex-none border-b border-border bg-muted/40 px-4 py-2 text-[12px] leading-relaxed text-muted-foreground">
          {archive.note}
        </p>
      ) : null}

      {/* The list fills whatever is left. */}
      <div className="min-h-0 flex-1 overflow-auto">
        <table className="w-full border-collapse text-[12px]">
          <thead className="sticky top-0 z-10 bg-background">
            <tr className="border-b border-border">
              {COLUMNS.map((col) => (
                <th
                  key={col.key}
                  onClick={() => toggleSort(col.key)}
                  className={cn(
                    "cursor-pointer select-none px-4 py-1.5 font-medium text-muted-foreground hover:text-foreground",
                    col.align === "right" ? "text-right" : "text-left",
                    col.width,
                  )}
                >
                  {col.label}
                  <span className="ml-1 inline-block w-2">{sort.key === col.key ? (sort.dir > 0 ? "▲" : "▼") : ""}</span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 ? (
              <tr><td colSpan={4} className="px-4 py-10 text-center text-muted-foreground">这一层是空的</td></tr>
            ) : rows.map((row) => {
              const key = `${row.isDir ? "d" : "f"}:${row.label}`;
              const ratio = row.size && row.compressed ? Math.round((row.compressed / row.size) * 100) : null;
              return (
                <tr
                  key={key}
                  onClick={() => setSelected(key)}
                  onDoubleClick={() => (row.isDir ? enter(row.label) : onOpenEntry?.(row.name))}
                  className={cn(
                    "cursor-default border-b border-border/50 last:border-0",
                    selected === key ? "bg-primary/10" : "hover:bg-accent",
                  )}
                  title={row.isDir ? `${row.count} 个文件 · 双击进入` : "双击预览"}
                >
                  <td className="max-w-0 truncate px-4 py-1.5">
                    <span className="mr-2" aria-hidden>{row.isDir ? "📁" : "📄"}</span>
                    <span data-i18n-skip>{row.label}</span>
                  </td>
                  <td className="px-4 py-1.5 text-right tabular-nums text-muted-foreground">{formatBytes(row.size)}</td>
                  <td className="px-4 py-1.5 text-right tabular-nums text-muted-foreground">
                    {row.compressed ? formatBytes(row.compressed) : "—"}
                  </td>
                  <td className="px-4 py-1.5 text-right tabular-nums text-muted-foreground">
                    {ratio === null ? "—" : `${ratio}%`}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* Status bar: this level on the left, the archive's own totals on the right. */}
      <div className="flex flex-none items-center gap-4 border-t border-border bg-muted/30 px-4 py-1.5 text-[11px] text-muted-foreground">
        <span className="tabular-nums">{here.dirs} 个文件夹 · {here.files} 个文件 · {formatBytes(here.size)}</span>
        <span className="ml-auto tabular-nums" data-i18n-skip>
          共 {archive?.count_is_partial ? "至少 " : ""}{Number(archive?.total || 0).toLocaleString()} 项
          {archive?.truncated ? `（已载入 ${entries.length}）` : ""}
          {archive?.metadata_entries ? ` · ${archive.metadata_entries} 个附属条目` : ""}
        </span>
      </div>
    </div>
  );
}
