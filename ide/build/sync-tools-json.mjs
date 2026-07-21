// Regenerate server/prompts/tools.json from the client tool registry in src/main.js.
//
// The desktop registry `_buildAgentToolSchemas(true, [])` is the single source of
// truth for tool schemas. The server catalog (prompts/tools.json) had drifted:
// 29 tools the client offers were missing, so L0 assembly could not inject their
// schemas and the prompts ended up teaching "phantom" tools the model could not call.
//
// This extractor brace-matches the real function source out of main.js (skipping
// string/template/regex/comment contents) and evaluates it with the only two
// module-level dependencies it touches — `inTauri` (force true → full catalog) and
// `_applyCloudToolDescs` (identity: cloud descriptions come FROM this file, so there
// is nothing to overlay when generating it). The result is the exact runtime array.
//
// Merge policy is ADDITIVE: existing entries keep their position and content; only
// tools absent from tools.json are appended. This cannot silently drop a capability.
//
// Run: node build/sync-tools-json.mjs         (writes tools.json, prints a summary)
//      node build/sync-tools-json.mjs --check  (exit 1 if out of sync, writes nothing)
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const MAIN = join(HERE, "../src/main.js");
const TOOLS_JSON = join(HERE, "../../server/prompts/tools.json");
const SRC = readFileSync(MAIN, "utf8");

// ---- source scanner (skip strings / templates / regex / comments) ----
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
      i--;
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
  const m = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).exec(SRC);
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

// Evaluate the real registry with its two module-level deps injected.
const buildFn = new Function(
  "inTauri",
  "_applyCloudToolDescs",
  `${extractFn("_buildAgentToolSchemas")}\n;return _buildAgentToolSchemas;`,
)(true, (tools) => tools);

const registry = buildFn(true, []);
const registryByName = new Map();
for (const tool of registry) {
  const name = tool?.function?.name;
  if (name) registryByName.set(name, tool);
}

const current = JSON.parse(readFileSync(TOOLS_JSON, "utf8"));
const currentNames = new Set(current.map((t) => t?.function?.name).filter(Boolean));

const missing = [...registryByName.keys()].filter((name) => !currentNames.has(name));
const onlyInCatalog = [...currentNames].filter((name) => !registryByName.has(name));

const merged = current.concat(missing.map((name) => registryByName.get(name)));

const check = process.argv.includes("--check");
console.log(`registry tools:        ${registryByName.size}`);
console.log(`tools.json (before):   ${current.length}`);
console.log(`missing (to append):   ${missing.length}${missing.length ? "  -> " + missing.join(", ") : ""}`);
console.log(`only in tools.json:    ${onlyInCatalog.length}${onlyInCatalog.length ? "  -> " + onlyInCatalog.join(", ") : ""}`);
console.log(`tools.json (after):    ${merged.length}`);

if (check) {
  if (missing.length) { console.error("tools.json is missing client-registry tools; run without --check to sync."); process.exit(1); }
  console.log("in sync");
  process.exit(0);
}

writeFileSync(TOOLS_JSON, JSON.stringify(merged, null, 2) + "\n", "utf8");
console.log(`wrote ${TOOLS_JSON}`);
