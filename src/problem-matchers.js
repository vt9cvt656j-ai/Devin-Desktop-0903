// Michael IDE — problem matchers.
//
// Turns raw compiler/linter output captured from a task run into structured
// diagnostics the editor can render in the Problems panel and as inline
// squiggles. Each matcher mirrors a well-known VS Code problem matcher.
//
// A parsed problem is:
//   { file, line, col, endLine?, endCol?, severity, message, code?, source }
// where `file` may be relative (resolved against the task cwd by the caller).

function normSeverity(s) {
  const v = String(s || "").toLowerCase();
  if (v.startsWith("err") || v === "fatal error" || v === "fatal") return "error";
  if (v.startsWith("warn")) return "warning";
  if (v === "info" || v === "information" || v === "note" || v === "hint") return "info";
  return "error";
}

const lines = (text) => String(text || "").split(/\r?\n/);

// --- TypeScript compiler (tsc) ---
// `file(line,col): error TS1234: message`  and  `file:line:col - error TS1234: message`
function matchTsc(text) {
  const out = [];
  const reParen = /^(.+?)\((\d+),(\d+)\):\s+(error|warning|info)\s+(TS\d+)\s*:\s+(.*)$/;
  const reColon = /^(.+?):(\d+):(\d+)\s*-\s*(error|warning|info)\s+(TS\d+)\s*:\s+(.*)$/;
  for (const line of lines(text)) {
    const m = reParen.exec(line) || reColon.exec(line);
    if (m) {
      out.push({
        file: m[1], line: +m[2], col: +m[3],
        severity: normSeverity(m[4]), code: m[5], message: m[6], source: "ts",
      });
    }
  }
  return out;
}

// --- Rust (cargo / rustc) ---
// A heading line carries severity + message; the following `--> file:line:col`
// line carries the location.
function matchRustc(text) {
  const out = [];
  const head = /^(error|warning)(?:\[([A-Za-z0-9]+)\])?:\s+(.*)$/;
  const loc = /^\s*-->\s+(.+?):(\d+):(\d+)\s*$/;
  let cur = null;
  for (const line of lines(text)) {
    const h = head.exec(line);
    if (h) {
      // Ignore aggregate summaries like "aborting due to N previous errors".
      if (/aborting due to|previous error|could not compile|generated \d+ warning/i.test(h[3])) {
        cur = null;
      } else {
        cur = { severity: normSeverity(h[1]), code: h[2] || undefined, message: h[3], source: "rustc" };
      }
      continue;
    }
    const l = loc.exec(line);
    if (l && cur) {
      out.push({ ...cur, file: l[1], line: +l[2], col: +l[3] });
      cur = null;
    }
  }
  // Defensive: if the last head had no matching loc line (truncated output),
  // don't let it pollute a future call's state — cur is a local, but hygiene
  // guarantees the dangling reference is dropped here explicitly.
  cur = null;
  return out;
}

// --- GCC / Clang / Make C-C++ ---
// `file:line:col: error: message`  (col optional)
function matchGcc(text) {
  const out = [];
  const re = /^(.+?):(\d+):(?:(\d+):)?\s+(fatal error|error|warning|note):\s+(.*)$/;
  for (const line of lines(text)) {
    const m = re.exec(line);
    if (m) {
      out.push({
        file: m[1], line: +m[2], col: m[3] ? +m[3] : 1,
        severity: normSeverity(m[4]), message: m[5], source: "gcc",
      });
    }
  }
  return out;
}

// --- ESLint (stylish reporter) ---
// A bare file path line, then indented `line:col  severity  message  rule` rows.
function matchEslint(text) {
  const out = [];
  let file = null;
  // Accept unix absolute (/x), dotted relative with either separator (./x ../x .\x ..\x)
  // and Windows drive-absolute (C:\x, C:/x) — the old `\.{0,2}\/` missed `.\` style
  // paths, silently dropping every ESLint diagnostic on Windows.
  const fileRe = /^(?:\/|\.{0,2}[\\/]|[A-Za-z]:[\\/]).+/;
  const rowRe = /^\s+(\d+):(\d+)\s+(error|warning)\s+(.*?)(?:\s{2,}([\w./-]+))?\s*$/;
  for (const line of lines(text)) {
    const r = rowRe.exec(line);
    if (r && file) {
      out.push({
        file, line: +r[1], col: +r[2],
        severity: normSeverity(r[3]), message: r[4].trim(), code: r[5], source: "eslint",
      });
      continue;
    }
    const trimmed = line.trim();
    if (
      trimmed &&
      !/^\s/.test(line) &&
      fileRe.test(trimmed) &&
      !/^\d+\s+problems?/.test(trimmed) &&
      !/[✖✓]/.test(trimmed)
    ) {
      file = trimmed;
    }
  }
  return out;
}

// --- Python (flake8 / ruff / mypy / pyright) ---
function matchPython(text) {
  const out = [];
  const flake = /^(.+?):(\d+):(\d+):\s+([A-Z]+\d+)\s+(.*)$/;
  const mypy = /^(.+?):(\d+):\s+(error|warning|note):\s+(.*)$/;
  const pyright = /^(.+?):(\d+):(\d+)\s*-\s*(error|warning|information):\s+(.*)$/;
  for (const line of lines(text)) {
    let m = flake.exec(line);
    if (m) {
      const c = m[4][0];
      out.push({
        file: m[1], line: +m[2], col: +m[3],
        severity: c === "E" || c === "F" ? "error" : "warning",
        code: m[4], message: m[5], source: "python",
      });
      continue;
    }
    m = pyright.exec(line);
    if (m) {
      out.push({
        file: m[1], line: +m[2], col: +m[3],
        severity: normSeverity(m[4]), message: m[5], source: "pyright",
      });
      continue;
    }
    m = mypy.exec(line);
    if (m) {
      out.push({
        file: m[1], line: +m[2], col: 1,
        severity: normSeverity(m[3]), message: m[4], source: "mypy",
      });
    }
  }
  return out;
}

const MATCHERS = { tsc: matchTsc, rustc: matchRustc, gcc: matchGcc, eslint: matchEslint, python: matchPython };

// Choose a matcher from an explicit hint (e.g. "$tsc") or, failing that, by
// sniffing the command. Returns "auto" to run every matcher and merge.
export function pickMatcher(name, command) {
  const n = String(name || "").toLowerCase();
  if (n.includes("tsc")) return "tsc";
  if (n.includes("rustc") || n.includes("cargo")) return "rustc";
  if (n.includes("eslint")) return "eslint";
  if (n.includes("gcc") || n.includes("clang") || n.includes("cpp")) return "gcc";
  if (/python|flake|mypy|pyright|ruff/.test(n)) return "python";

  const c = String(command || "").toLowerCase();
  if (/\bcargo\b|\brustc\b/.test(c)) return "rustc";
  if (/\btsc\b|vue-tsc|tspc/.test(c)) return "tsc";
  if (/eslint/.test(c)) return "eslint";
  if (/\bg\+\+\b|\bgcc\b|\bclang\b|\bclang\+\+\b|\bmake\b|\bcmake\b/.test(c)) return "gcc";
  if (/python|flake8|mypy|pyright|ruff|pytest/.test(c)) return "python";
  return "auto";
}

function dedupe(problems) {
  const seen = new Set();
  const out = [];
  for (const p of problems) {
    if (!p || !p.file || !p.line) continue;
    const sig = `${p.file}|${p.line}|${p.col}|${p.severity}|${p.message}`;
    if (seen.has(sig)) continue;
    seen.add(sig);
    out.push({
      file: p.file,
      line: Math.max(1, p.line | 0),
      col: Math.max(1, (p.col | 0) || 1),
      severity: normSeverity(p.severity),
      message: p.message || "",
      code: p.code,
      source: p.source,
    });
  }
  return out;
}

export function parseProblems(output, opts = {}) {
  const key = pickMatcher(opts.matcher, opts.command);
  if (key === "auto") {
    const merged = [];
    for (const fn of Object.values(MATCHERS)) merged.push(...fn(output));
    return dedupe(merged);
  }
  return dedupe(MATCHERS[key](output));
}
