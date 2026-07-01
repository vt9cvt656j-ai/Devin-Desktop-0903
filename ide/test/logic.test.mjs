// Unit tests for the pure logic inside the (monolithic, export-less) src/main.js.
//
// main.js is a 23k-line browser module with no exports, so we can't `import` its
// helpers. Instead we EXTRACT each function's real source by name — brace-matched with a
// small scanner that skips string / template / regex / comment contents so their braces
// aren't miscounted — and eval it with its module-level dependencies injected as params.
// => these tests exercise the ACTUAL shipped code, not hand-copied duplicates that drift.
//
// Run:  node --test   (from ide/, or `npm test`)
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "../src/main.js"), "utf8");

// ---- source scanner (skip strings / templates / regex / comments) --------------------
function skipString(s, i, q) { i++; for (; i < s.length; i++) { if (s[i] === "\\") { i++; continue; } if (s[i] === q) return i; } return i; }
function skipRegex(s, i) { i++; let cls = false; for (; i < s.length; i++) { const c = s[i]; if (c === "\\") { i++; continue; } if (c === "[") cls = true; else if (c === "]") cls = false; else if (c === "/" && !cls) return i; } return i; }
function skipTemplate(s, i) {
  i++;
  for (; i < s.length; i++) {
    if (s[i] === "\\") { i++; continue; }
    if (s[i] === "`") return i;
    if (s[i] === "$" && s[i + 1] === "{") {
      i += 2; let depth = 1;
      for (; i < s.length && depth > 0; i++) {
        const c = s[i];
        if (c === "\\") { i++; continue; }
        if (c === "'" || c === '"') { i = skipString(s, i, c); continue; }
        if (c === "`") { i = skipTemplate(s, i); continue; }
        if (c === "{") depth++; else if (c === "}") depth--;
      }
      i--; // for-loop will ++
    }
  }
  return i;
}
function isRegexPos(s, i) {
  let j = i - 1; while (j >= 0 && /\s/.test(s[j])) j--;
  if (j < 0) return true;
  if ("=([,{;:!&|?+-*%<>~^".includes(s[j])) return true;
  return /(?:^|[^\w$])(return|typeof|case|in|of|do|else|void|delete|instanceof|yield|await)$/.test(s.slice(Math.max(0, j - 12), j + 1));
}
function extractFn(name) {
  const m = new RegExp(`function\\s+${name}\\s*\\(`).exec(SRC);
  if (!m) throw new Error(`function ${name} not found in main.js`);
  let i = SRC.indexOf("{", m.index), depth = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i], d = SRC[i + 1];
    if (c === "/" && d === "/") { i = SRC.indexOf("\n", i); if (i < 0) i = SRC.length; continue; }
    if (c === "/" && d === "*") { i = SRC.indexOf("*/", i + 2) + 1; continue; }
    if (c === "'" || c === '"') { i = skipString(SRC, i, c); continue; }
    if (c === "`") { i = skipTemplate(SRC, i); continue; }
    if (c === "/" && isRegexPos(SRC, i)) { i = skipRegex(SRC, i); continue; }
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (depth === 0) return SRC.slice(m.index, i + 1); }
  }
  throw new Error(`unbalanced braces extracting ${name}`);
}
// Build the real function with its module-level deps injected as parameters.
function load(name, deps = {}) {
  const keys = Object.keys(deps);
  const fn = new Function(...keys, `${extractFn(name)}\n;return ${name};`);
  return fn(...keys.map((k) => deps[k]));
}

// ---- tests ---------------------------------------------------------------------------
test("_toPosix normalizes Windows backslashes, no-op elsewhere", () => {
  const f = load("_toPosix");
  assert.equal(f("C:\\Users\\me\\proj"), "C:/Users/me/proj");
  assert.equal(f("/Users/me/proj"), "/Users/me/proj"); // mac untouched
  assert.equal(f("a\\b/c"), "a/b/c");
  assert.equal(f(null), null);
  assert.equal(f(42), 42);
});

test("_resolveRel resolves relatives to the workspace + passes absolutes through (incl. Windows)", () => {
  const f = load("_resolveRel", { _allRoots: () => ["/Users/me/proj"] });
  assert.equal(f("src/main.js"), "/Users/me/proj/src/main.js"); // plain relative → prepend root
  assert.equal(f("proj/src/x"), "/Users/me/proj/src/x");        // redundant root-name stripped
  assert.equal(f("/etc/hosts"), "/etc/hosts");                  // unix absolute → as-is
  assert.equal(f("C:\\Windows\\x"), "C:\\Windows\\x");          // windows absolute (backslash) → as-is
  assert.equal(f("C:/Windows/x"), "C:/Windows/x");              // windows absolute (fwd slash) → as-is
  assert.equal(f(""), "");
  // Windows workspace (posix-normalized root) resolves correctly:
  const fw = load("_resolveRel", { _allRoots: () => ["C:/Users/me/proj"] });
  assert.equal(fw("src/x.js"), "C:/Users/me/proj/src/x.js");
});

test("_resolveRel with no open root leaves the path unchanged", () => {
  const f = load("_resolveRel", { _allRoots: () => [] });
  assert.equal(f("src/x.js"), "src/x.js");
});

test("_lev computes edit distance", () => {
  const f = load("_lev");
  assert.equal(f("kitten", "kitten"), 0);
  assert.equal(f("kitten", "sitten"), 1);
  assert.equal(f("read_file", "readfile"), 1);
  assert.ok(f("run_cmd", "bash") >= 3);
});

test("_buildRepoMap builds a per-file symbol map from the index, query-boosted + bounded", () => {
  const idx = new Map([
    ["openFolder", [{ name: "openFolder", kind: "function", path: "src/main.js", line: 3567 }]],
    ["_resolveRel", [{ name: "_resolveRel", kind: "function", path: "src/main.js", line: 14329 }]],
    ["parseAuth", [{ name: "parseAuth", kind: "function", path: "src/auth.js", line: 10 }]],
    ["verifyToken", [{ name: "verifyToken", kind: "function", path: "src/auth.js", line: 40 }]],
  ]);
  const f = load("_buildRepoMap", { _symbolIndex: idx });
  const out = f("fix the auth token", 6000);
  assert.match(out, /项目符号地图/);
  assert.match(out, /src\/auth\.js: parseAuth, verifyToken/);   // both auth symbols listed
  assert.match(out, /src\/main\.js: openFolder, _resolveRel/);
  // query "auth token" should rank auth.js ABOVE main.js despite equal symbol counts:
  assert.ok(out.indexOf("src/auth.js") < out.indexOf("src/main.js"), "query-relevant file ranks first");
  // empty index → empty string (graceful before the background index builds):
  assert.equal(load("_buildRepoMap", { _symbolIndex: new Map() })("x", 6000), "");
});

test("_safeJsonLoose repairs malformed \\u escapes (the 'unexpected end of hex escape' bug)", () => {
  const f = load("_safeJsonLoose");
  // model put a literal \u (a regex) in content without double-escaping → broken JSON:
  const r = f('{"path":"a.js","content":"const re = /\\username/;"}');
  assert.ok(r && r.path === "a.js", "recovers the object");
  assert.match(r.content, /\\username/, "the literal \\u is preserved as text");
  // truncated \u12 right before the closing quote:
  const r2 = f('{"content":"tail\\u12"}');
  assert.ok(r2 && typeof r2.content === "string" && r2.content.includes("tail"));
  // a genuinely-valid ✓ must STILL decode to the checkmark (not get double-escaped):
  assert.equal(f('{"content":"ok \\u2713"}').content, "ok ✓");
  // an already-escaped \\u (literal backslash) must be left alone:
  assert.equal(f('{"content":"C:\\\\users"}').content, "C:\\users");
});
