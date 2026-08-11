import { useCallback, useMemo, useState } from "react";
import { cn } from "./lib/cn.js";

/**
 * The archive panel, rebuilt as the browser that archive software actually is.
 *
 * What it replaced: one flat table of every entry in the archive, 1,652 rows deep for a real
 * build artefact, with the full path repeated on every line. Nothing to navigate, nothing to sort,
 * and the directory structure — the thing you open an archive to understand — was left for the
 * reader to reconstruct from slashes.
 *
 * 7-Zip, Keka and WinRAR all settled on the same shape long ago, and it is the right one: a path
 * bar you can climb, one level of contents at a time, sortable columns, and a status line that
 * totals what you are looking at. Folders aggregate the size of everything beneath them, so the
 * top level answers "what is big in here" without any expanding.
 *
 * Presentational. main.js owns the listing and both actions.
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

/** Immediate children of `cwd`, with folders aggregating everything beneath them. */
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
      if (!entry?.is_dir) files.push({ ...entry, label: rest });
    } else {
      // Anything deeper rolls up into the folder that contains it.
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
  { key: "size", label: "大小", align: "right", width: "w-24" },
  { key: "compressed", label: "压缩后", align: "right", width: "w-24" },
  { key: "ratio", label: "压缩率", align: "right", width: "w-20" },
];

export function ArchiveBrowser({ archive, onOpenEntry, onExtract }) {
  const [cwd, setCwd] = useState("");
  const [sort, setSort] = useState({ key: "label", dir: 1 });
  const entries = Array.isArray(archive?.entries) ? archive.entries : [];

  const rows = useMemo(() => {
    const level = levelAt(entries, cwd).map((row) => ({
      ...row,
      isDir: !!row.isDir,
      size: Number(row.size) || 0,
      compressed: Number(row.compressed ?? row.compressed_size) || 0,
    }));
    const { key, dir } = sort;
    return level.sort((a, b) => {
      // Folders first, always — the point of the view is structure.
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
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
  const totals = useMemo(() => {
    const files = rows.filter((r) => !r.isDir).length;
    const dirs = rows.length - files;
    const size = rows.reduce((n, r) => n + r.size, 0);
    return { files, dirs, size };
  }, [rows]);

  const toggleSort = useCallback((key) => {
    setSort((s) => (s.key === key ? { key, dir: -s.dir } : { key, dir: key === "label" ? 1 : -1 }));
  }, []);

  if (!entries.length) return null;

  return (
    <section className="ui-island flex min-h-0 flex-col rounded-xl border border-border bg-card">
      {/* Path bar. Climbing back out is one click per level, which is why archive software has
          always put the path here rather than making you re-open the file. */}
      <div className="flex items-center gap-1 border-b border-border px-3 py-2 text-[12px]">
        <button
          type="button"
          onClick={() => setCwd("")}
          className={cn(
            "rounded px-1.5 py-0.5 font-medium hover:bg-accent",
            cwd ? "text-primary" : "text-foreground",
          )}
          data-i18n-skip
        >
          {archive?.format || "压缩包"}
        </button>
        {crumbs.map((part, i) => (
          <span key={`${part}-${i}`} className="flex items-center gap-1 min-w-0">
            <span className="text-muted-foreground">›</span>
            <button
              type="button"
              onClick={() => setCwd(crumbs.slice(0, i + 1).join("/"))}
              className={cn(
                "truncate rounded px-1.5 py-0.5 hover:bg-accent",
                i === crumbs.length - 1 ? "text-foreground" : "text-primary",
              )}
              data-i18n-skip
            >
              {part}
            </button>
          </span>
        ))}
        <div className="ml-auto flex items-center gap-2">
          {archive?.encrypted ? (
            <span className="rounded bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground">🔒 含加密条目</span>
          ) : null}
          <button
            type="button"
            onClick={() => onExtract?.()}
            className="rounded-md bg-primary px-2.5 py-1 text-[12px] font-medium text-primary-foreground hover:opacity-90"
          >
            全部解压…
          </button>
        </div>
      </div>

      {archive?.note ? (
        <p className="border-b border-border bg-muted/40 px-3 py-2 text-[12px] leading-relaxed text-muted-foreground">
          {archive.note}
        </p>
      ) : null}

      <div className="min-h-0 max-h-[420px] overflow-auto">
        <table className="w-full border-collapse text-[12px]">
          <thead className="sticky top-0 z-10 bg-card">
            <tr className="border-b border-border">
              {COLUMNS.map((col) => (
                <th
                  key={col.key}
                  onClick={() => toggleSort(col.key)}
                  className={cn(
                    "cursor-pointer select-none px-3 py-1.5 font-medium text-muted-foreground hover:text-foreground",
                    col.align === "right" ? "text-right" : "text-left",
                    col.width,
                  )}
                >
                  {col.label}
                  {sort.key === col.key ? <span className="ml-1">{sort.dir > 0 ? "▲" : "▼"}</span> : null}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const ratio = row.size ? Math.round((row.compressed / row.size) * 100) : null;
              return (
                <tr
                  key={`${row.isDir ? "d" : "f"}:${row.label}`}
                  onDoubleClick={() => row.isDir && setCwd(cwd ? `${cwd}/${row.label}` : row.label)}
                  onClick={() => {
                    if (row.isDir) setCwd(cwd ? `${cwd}/${row.label}` : row.label);
                    else onOpenEntry?.(row.name);
                  }}
                  className="cursor-pointer border-b border-border/60 last:border-0 hover:bg-accent"
                  title={row.isDir ? `${row.count} 个文件` : row.name}
                >
                  <td className="max-w-0 truncate px-3 py-1.5">
                    <span className="mr-1.5" aria-hidden>{row.isDir ? "📁" : "📄"}</span>
                    <span data-i18n-skip>{row.label}</span>
                    {row.isDir ? (
                      <span className="ml-2 text-[11px] text-muted-foreground">{row.count}</span>
                    ) : null}
                  </td>
                  <td className="px-3 py-1.5 text-right tabular-nums text-muted-foreground">{formatBytes(row.size)}</td>
                  <td className="px-3 py-1.5 text-right tabular-nums text-muted-foreground">
                    {row.compressed ? formatBytes(row.compressed) : "—"}
                  </td>
                  <td className="px-3 py-1.5 text-right tabular-nums text-muted-foreground">
                    {ratio === null || !row.compressed ? "—" : `${ratio}%`}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* Status line. The archive's own totals, not this view's — a folder you have navigated into
          says how many entries the whole archive holds, the way every file manager does. */}
      <div className="flex items-center gap-3 border-t border-border px-3 py-1.5 text-[11px] text-muted-foreground">
        <span>{totals.dirs} 个文件夹 · {totals.files} 个文件</span>
        <span>本层 {formatBytes(totals.size)}</span>
        <span className="ml-auto tabular-nums" data-i18n-skip>
          {archive?.count_is_partial ? "至少 " : ""}{Number(archive?.total || 0).toLocaleString()} 项
          {archive?.truncated ? `（已载入 ${entries.length}）` : ""}
          {archive?.metadata_entries ? ` · ${archive.metadata_entries} 个附属条目` : ""}
        </span>
      </div>
    </section>
  );
}
