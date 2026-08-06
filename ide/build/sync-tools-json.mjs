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
// Name membership is strict: the desktop registry is authoritative. Entries that
// exist only in tools.json are removed because the gateway would otherwise advertise
// a tool the desktop cannot discover or execute. Existing entries keep their order
// and descriptions; newly registered tools are appended.
//
// Run: node build/sync-tools-json.mjs         (writes tools.json, prints a summary)
//      node build/sync-tools-json.mjs --check  (exit 1 if out of sync, writes nothing)
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
// Tests may point the synchronizer at isolated fixtures. Production calls leave
// both variables unset and continue to use the repository paths below.
const MAIN = process.env.MICHAEL_IDE_MAIN_PATH || join(HERE, "../src/main.js");
const TOOLS_JSON = process.env.MICHAEL_IDE_TOOLS_JSON_PATH || join(HERE, "../../server/prompts/tools.json");
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

// 光比工具**名**或者顶层参数成员是不够的。网关按名字注入完整 schema；嵌套对象、数组、
// enum/union 或边界约束漂移，同样会让合法调用被拒绝或让无效参数漏过校验。
const currentByName = new Map();
for (const tool of current) {
  const name = tool?.function?.name;
  if (name) currentByName.set(name, tool);
}

const SCHEMA_ANNOTATIONS = new Set(["description", "title"]);
const SCHEMA_MAP_KEYWORDS = new Set([
  "properties",
  "patternProperties",
  "dependentSchemas",
  "definitions",
  "$defs",
]);
const SCHEMA_ARRAY_KEYWORDS = new Set(["allOf", "anyOf", "oneOf", "prefixItems"]);
const SCHEMA_SINGLE_KEYWORDS = new Set([
  "additionalProperties",
  "contains",
  "contentSchema",
  "else",
  "if",
  "items",
  "not",
  "propertyNames",
  "then",
  "unevaluatedItems",
  "unevaluatedProperties",
]);
const hasOwn = (value, key) => Object.prototype.hasOwnProperty.call(value, key);
const isObject = (value) => value !== null && typeof value === "object" && !Array.isArray(value);

// Return the validation-relevant schema only. Object key order is normalized so
// hand-edited JSON formatting cannot create false drift; array order remains exact
// because it can be meaningful for tuple items and union branch precedence.
function schemaShape(node) {
  if (!isObject(node)) return node;
  const shaped = {};
  for (const key of Object.keys(node).filter((key) => !SCHEMA_ANNOTATIONS.has(key)).sort()) {
    const value = node[key];
    if (SCHEMA_MAP_KEYWORDS.has(key) && isObject(value)) {
      shaped[key] = Object.fromEntries(
        Object.keys(value).sort().map((name) => [name, schemaShape(value[name])]),
      );
    } else if (SCHEMA_ARRAY_KEYWORDS.has(key) && Array.isArray(value)) {
      shaped[key] = value.map(schemaShape);
    } else if (SCHEMA_SINGLE_KEYWORDS.has(key)) {
      shaped[key] = Array.isArray(value) ? value.map(schemaShape) : schemaShape(value);
    } else if (key === "dependencies" && isObject(value)) {
      shaped[key] = Object.fromEntries(
        Object.keys(value).sort().map((name) => [
          name,
          isObject(value[name]) ? schemaShape(value[name]) : value[name],
        ]),
      );
    } else {
      shaped[key] = value;
    }
  }
  return shaped;
}

// Produce reviewable paths rather than a single opaque "schema changed" flag.
// This walks every JSON-schema keyword, including type/default/required, unions,
// items, ranges, patterns and additionalProperties, without maintaining a brittle
// allow-list that would miss a newly introduced constraint.
function collectSchemaDiffs(expected, actual, path = "parameters", diffs = []) {
  if (Object.is(expected, actual)) return diffs;
  if (Array.isArray(expected) || Array.isArray(actual)) {
    if (!Array.isArray(expected) || !Array.isArray(actual)) {
      diffs.push(`${path}: ${JSON.stringify(actual)} -> ${JSON.stringify(expected)}`);
      return diffs;
    }
    if (expected.length !== actual.length) {
      diffs.push(`${path}.length: ${actual.length} -> ${expected.length}`);
    }
    const commonLength = Math.min(expected.length, actual.length);
    for (let i = 0; i < commonLength; i++) {
      collectSchemaDiffs(expected[i], actual[i], `${path}[${i}]`, diffs);
    }
    return diffs;
  }
  if (isObject(expected) || isObject(actual)) {
    if (!isObject(expected) || !isObject(actual)) {
      diffs.push(`${path}: ${JSON.stringify(actual)} -> ${JSON.stringify(expected)}`);
      return diffs;
    }
    const keys = [...new Set([...Object.keys(expected), ...Object.keys(actual)])].sort();
    for (const key of keys) {
      const childPath = `${path}.${key}`;
      if (!hasOwn(expected, key)) diffs.push(`${childPath}: remove`);
      else if (!hasOwn(actual, key)) diffs.push(`${childPath}: add ${JSON.stringify(expected[key])}`);
      else collectSchemaDiffs(expected[key], actual[key], childPath, diffs);
    }
    return diffs;
  }
  diffs.push(`${path}: ${JSON.stringify(actual)} -> ${JSON.stringify(expected)}`);
  return diffs;
}

// Clone the registry schema (the structural source of truth) while carrying over
// catalog prose at the corresponding node. Generic recursion handles properties,
// items, anyOf/oneOf/allOf and future schema keywords uniformly.
function mergeSchemaAnnotations(registryNode, catalogNode) {
  if (!isObject(registryNode)) return registryNode;

  const mergedNode = {};
  for (const [key, value] of Object.entries(registryNode)) {
    const catalogValue = isObject(catalogNode) ? catalogNode[key] : undefined;
    if (SCHEMA_MAP_KEYWORDS.has(key) && isObject(value)) {
      mergedNode[key] = Object.fromEntries(
        Object.entries(value).map(([name, childSchema]) => [
          name,
          mergeSchemaAnnotations(childSchema, isObject(catalogValue) ? catalogValue[name] : undefined),
        ]),
      );
    } else if (SCHEMA_ARRAY_KEYWORDS.has(key) && Array.isArray(value)) {
      mergedNode[key] = value.map((childSchema, index) =>
        mergeSchemaAnnotations(childSchema, Array.isArray(catalogValue) ? catalogValue[index] : undefined),
      );
    } else if (SCHEMA_SINGLE_KEYWORDS.has(key)) {
      mergedNode[key] = Array.isArray(value)
        ? value.map((childSchema, index) =>
            mergeSchemaAnnotations(childSchema, Array.isArray(catalogValue) ? catalogValue[index] : undefined),
          )
        : mergeSchemaAnnotations(value, catalogValue);
    } else if (key === "dependencies" && isObject(value)) {
      mergedNode[key] = Object.fromEntries(
        Object.entries(value).map(([name, dependency]) => [
          name,
          isObject(dependency)
            ? mergeSchemaAnnotations(dependency, isObject(catalogValue) ? catalogValue[name] : undefined)
            : dependency,
        ]),
      );
    } else {
      mergedNode[key] = value;
    }
  }
  if (isObject(catalogNode)) {
    for (const key of SCHEMA_ANNOTATIONS) {
      if (hasOwn(catalogNode, key)) mergedNode[key] = catalogNode[key];
    }
  }
  return mergedNode;
}

const drifted = [];
for (const [name, regTool] of registryByName) {
  const cur = currentByName.get(name);
  if (!cur) continue; // 缺失的由 missing 那条管
  const diffs = collectSchemaDiffs(
    schemaShape(regTool.function?.parameters || {}),
    schemaShape(cur.function?.parameters || {}),
  );
  if (diffs.length) drifted.push({ name, diffs });
}

const merged = current
  .filter((tool) => registryByName.has(tool?.function?.name))
  .concat(missing.map((name) => registryByName.get(name)));

const check = process.argv.includes("--check");
console.log(`registry tools:        ${registryByName.size}`);
console.log(`tools.json (before):   ${current.length}`);
console.log(`missing (to append):   ${missing.length}${missing.length ? "  -> " + missing.join(", ") : ""}`);
console.log(`only in tools.json:    ${onlyInCatalog.length}${onlyInCatalog.length ? "  -> remove " + onlyInCatalog.join(", ") : ""}`);
console.log(`tools.json (after):    ${merged.length}`);
console.log(
  `schema drift:          ${drifted.length}${
    drifted.length
      ? "\n  - " + drifted.map(({ name, diffs }) => `${name} (${diffs.join("; ")})`).join("\n  - ")
      : ""
  }`,
);

if (check) {
  let bad = false;
  if (missing.length) {
    console.error("tools.json is missing client-registry tools; run without --check to sync.");
    bad = true;
  }
  if (onlyInCatalog.length) {
    console.error("tools.json advertises tools absent from the client registry; run without --check to remove them.");
    bad = true;
  }
  if (drifted.length) {
    console.error(`${drifted.length} tool(s) have drifted parameter schemas; run without --check to sync.`);
    bad = true;
  }
  if (bad) process.exit(1);
  console.log("in sync");
  process.exit(0);
}

// 修复漂移时以客户端注册表的参数成员为准，不碰工具描述文字。
//
// 上一版是整条 `return reg` 替换，结果把目录里更丰富的描述（browser 那条讲了 Shadow
// DOM / iframe 穿透）换成了客户端注册表里那条短的——测试当场抓到。描述以目录为准，
// 这在上面判据里就写着；修复路径也必须守同一条规矩，否则判据写了等于没写。
const driftedNames = new Set(drifted.map(({ name }) => name));
const synced = merged.map((tool) => {
  const name = tool?.function?.name;
  if (!name || !driftedNames.has(name)) return tool;
  const reg = registryByName.get(name);
  if (!reg) return tool;
  const regParams = reg.function?.parameters || {};
  const fn = tool.function;
  fn.parameters = mergeSchemaAnnotations(regParams, fn.parameters);
  return tool;
});

// 保持文件原有的排版风格。这个文件在仓库里是**单行压缩 JSON**；无脑 pretty-print 会
// 产生四千多行的 diff，把真正的 schema 修改整个淹没掉（也让 review 变得没意义）。
const wasMinified = !readFileSync(TOOLS_JSON, "utf8").slice(0, 4096).includes("\n");
writeFileSync(
  TOOLS_JSON,
  wasMinified ? JSON.stringify(synced) : JSON.stringify(synced, null, 2) + "\n",
  "utf8",
);
console.log(`wrote ${TOOLS_JSON}`);
