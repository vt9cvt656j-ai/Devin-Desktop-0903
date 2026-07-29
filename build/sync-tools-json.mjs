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

// 光比工具**名**是不够的。
//
// 网关是按名字注入 schema 的：名字对上、参数却对不上，模型就会照客户端的定义去传参，
// 而网关拿着一份旧 schema 去校验/转发 —— 表现是那个工具一用就报错或参数被丢掉。历史上
// 已经踩过一次（线上 109 工具 vs 客户端 137，43 个工具一旦用就无 schema）。
//
// 只比**参数名集合**和 **required 集合**：描述文字本来就以目录为准（构建期还会被
// strip-tool-ip 清空），拿它做判据只会制造噪音。
const currentByName = new Map();
for (const tool of current) {
  const name = tool?.function?.name;
  if (name) currentByName.set(name, tool);
}

const paramShape = (tool) => {
  const params = tool?.function?.parameters || {};
  const props = Object.keys(params.properties || {}).sort();
  const required = [...(Array.isArray(params.required) ? params.required : [])].sort();
  return { props, required };
};

const drifted = [];
for (const [name, regTool] of registryByName) {
  const cur = currentByName.get(name);
  if (!cur) continue; // 缺失的由 missing 那条管
  const a = paramShape(regTool);
  const b = paramShape(cur);
  const diffs = [];
  // 方向是**单向**的：注册表有而目录没有的参数 = 真漂移（模型会发一个网关不认识的
  // 参数，被丢掉或直接报错）。反过来目录比注册表多是**正常的**——目录刻意暴露了更多
  // 动作/参数（browser 就是 51 vs 24），把它当漂移然后"修掉"等于把目录削成客户端的
  // 子集，那是数据丢失，不是同步。
  const added = a.props.filter((k) => !b.props.includes(k));
  if (added.length) diffs.push(`+${added.join("/")}`);
  if (a.required.join(",") !== b.required.join(",")) {
    diffs.push(`required: [${b.required.join(",")}] → [${a.required.join(",")}]`);
  }
  if (diffs.length) drifted.push(`${name} (${diffs.join("; ")})`);
}

const merged = current.concat(missing.map((name) => registryByName.get(name)));

const check = process.argv.includes("--check");
console.log(`registry tools:        ${registryByName.size}`);
console.log(`tools.json (before):   ${current.length}`);
console.log(`missing (to append):   ${missing.length}${missing.length ? "  -> " + missing.join(", ") : ""}`);
console.log(`only in tools.json:    ${onlyInCatalog.length}${onlyInCatalog.length ? "  -> " + onlyInCatalog.join(", ") : ""}`);
console.log(`tools.json (after):    ${merged.length}`);
console.log(`schema drift:          ${drifted.length}${drifted.length ? "\n  - " + drifted.join("\n  - ") : ""}`);

if (check) {
  let bad = false;
  if (missing.length) {
    console.error("tools.json is missing client-registry tools; run without --check to sync.");
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

// 修复漂移时**只补参数，不碰任何描述文字**。
//
// 上一版是整条 `return reg` 替换，结果把目录里更丰富的描述（browser 那条讲了 Shadow
// DOM / iframe 穿透）换成了客户端注册表里那条短的——测试当场抓到。描述以目录为准，
// 这在上面判据里就写着；修复路径也必须守同一条规矩，否则判据写了等于没写。
const driftedNames = new Set(drifted.map((d) => d.slice(0, d.indexOf(" ("))));
const synced = merged.map((tool) => {
  const name = tool?.function?.name;
  if (!name || !driftedNames.has(name)) return tool;
  const reg = registryByName.get(name);
  if (!reg) return tool;
  const regParams = reg.function?.parameters || {};
  const fn = tool.function;
  const params = fn.parameters || (fn.parameters = { type: "object", properties: {} });
  params.properties = params.properties || {};
  // 只补目录里没有的参数；两边都有的保留目录版本（它的描述更细）。
  for (const [key, def] of Object.entries(regParams.properties || {})) {
    if (!(key in params.properties)) params.properties[key] = def;
  }
  // required 必须听注册表的：客户端真正会发什么由它决定，目录多要一个参数就会
  // 把合法调用判成非法。
  if (Array.isArray(regParams.required)) params.required = regParams.required;
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
