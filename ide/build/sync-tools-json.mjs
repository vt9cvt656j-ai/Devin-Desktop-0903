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
// 源码里没有这个函数就不注入。测试用的合成 fixture main.js 只有一个注册表函数，
// 它的注册表也确实不会去调这个辅助函数——"没有就不注入"比塞一个 identity 桩好：桩会在
// 真文件里这个过滤器被改坏时替它兜底，把问题藏起来。
function extractIfPresent(name) {
  return new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).test(SRC) ? extractFn(name) : "";
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

// Evaluate the real registry with its module-level deps injected.
//
// `_userCapabilities` returns the tools THIS user declared in their own settings
// (~/.michael/settings.json). They are per-user runtime data and must never be baked
// into the shipped gateway catalog — one person's private HTTP tool would otherwise be
// advertised to everyone. An empty list is the correct answer here, not a stub.
//
// 这份注入清单和 _buildAgentToolSchemas 的依赖是**手工对齐**的，漏一个的后果不是少一个
// 工具，是整个脚本 ReferenceError 当场崩掉——于是目录再也同步不了，而两边就这么静静地
// 越漂越远。这正是它上次坏掉时发生的事：_userCapabilities 是后来加的，这里没跟上，
// 而唯一那条测试跑的是合成的 fixture main.js，真实那条路一次都没走过。
// 下面那条 --check 用例现在打的就是真文件。
const buildFn = new Function(
  "inTauri",
  "_applyCloudToolDescs",
  "_userCapabilities",
  // `_withoutDisabledTools` 抽真源码进来，不塞桩：它自己只依赖 _userCapabilities，
  // 而塞一个 identity 桩的话，将来这个过滤器改了逻辑这边不会跟着变，会静静地生成
  // 一份和运行时不一样的目录——那正是这个脚本存在的意义的反面。
  `${extractIfPresent("_withoutDisabledTools")}\n${extractFn("_buildAgentToolSchemas")}\n;return _buildAgentToolSchemas;`,
)(true, (tools) => tools, () => ({ tools: [], disabled: [] }));

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

// 描述漂移单独统计、只报不改。
//
// 这个脚本刻意不重写目录里的描述文字（网关那份可能是有意改过的），但"不重写"一路
// 滑成了"不比对"：main.js 的描述改完、测试全绿、提交发版，模型看到的还是旧文字。
// run_subagent 的并发上限就这样错了至少两个提交周期——注册表写着"4 个并发"，网关
// 那份仍写着"最多 2 个并发、其余排队"，等于一直在劝模型少扇出一半。
// 运行时以网关那份为准（release 构建还会把客户端描述整个剥掉），所以这条差异不是
// 文案问题，是**模型实际读到的指令**和源码不一致。
// 按 JSON 路径收集一份工具里所有的 description/title。
//
// 只比顶层是不够的——run_subagent 那句错了两个提交周期的并发说明就在
// `parameters.properties.tasks.description` 里，顶层一个字都没变。嵌套的说明恰恰是
// 最容易漂的：改行为的人改的是那一层，而同步脚本连看都不看。
function annotationMap(node, path = "", out = new Map()) {
  if (Array.isArray(node)) {
    node.forEach((item, index) => annotationMap(item, `${path}[${index}]`, out));
    return out;
  }
  if (!isObject(node)) return out;
  for (const key of SCHEMA_ANNOTATIONS) {
    if (typeof node[key] === "string") out.set(`${path}.${key}`, node[key]);
  }
  for (const [key, value] of Object.entries(node)) {
    if (SCHEMA_ANNOTATIONS.has(key)) continue;
    annotationMap(value, `${path}.${key}`, out);
  }
  return out;
}

const describedDrift = [];
for (const [name, regTool] of registryByName) {
  const cur = currentByName.get(name);
  if (!cur) continue;
  const a = annotationMap(regTool.function || {});
  const b = annotationMap(cur.function || {});
  const paths = new Set([...a.keys(), ...b.keys()]);
  const conflicts = [];
  const onlyOneSide = [];
  for (const path of [...paths].sort()) {
    const left = a.get(path);
    const right = b.get(path);
    if (left === right) continue;
    if (left != null && right != null) conflicts.push(path);
    else onlyOneSide.push(`${path}${left == null ? "（只有目录有）" : "（只有 main.js 有）"}`);
  }
  if (conflicts.length || onlyOneSide.length) {
    describedDrift.push({ name, conflicts, onlyOneSide });
  }
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

console.log(
  `description drift:     ${describedDrift.length}${
    describedDrift.length
      ? "\n  - " + describedDrift
        .map(({ name, conflicts, onlyOneSide }) => `${name}`
          + (conflicts.length ? ` 冲突: ${conflicts.slice(0, 3).join(", ")}${conflicts.length > 3 ? " …" : ""}` : "")
          + (onlyOneSide.length ? ` 缺失: ${onlyOneSide.slice(0, 3).join(", ")}${onlyOneSide.length > 3 ? " …" : ""}` : ""))
        .join("\n  - ")
      : ""
  }`,
);

if (check) {
  let bad = false;
  // 只有**冲突**才算失败。两边都写了却写的不一样，说明模型读到的和源码说的不是一回事
  // （run_subagent 的并发文案就是这么错了两个提交周期）。而"一边有一边没有"是遗漏——
  // 按 main.js 对齐会把只存在于目录里的指引删掉，那比不同步更糟，所以只报不拦。
  const conflicting = describedDrift.filter((d) => d.conflicts.length);
  if (conflicting.length) {
    console.error(
      "tools.json 和 main.js 的工具描述互相矛盾——运行时以目录那份为准，"
      + `请逐个确认哪边是对的再手工对齐：${conflicting.map((d) => d.name).join(", ")}`,
    );
    bad = true;
  }
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
