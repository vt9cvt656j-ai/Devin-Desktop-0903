// 工具契约全量扫描：给 138 个工具**每一个**按自己的 schema 合成一次调用，真的喂进
// _mapToolCall，再检查参数有没有活着到达返回的调用对象。
//
// 为什么要有这个文件：git_clone 曾经在这一层坏掉——模型按 schema 填了 url，归一化却
// 认不出这个形状，于是工具"用不了"，而且报的错跟真正的原因毫无关系。那种 bug 单看
// 代码不会发现，写一条一条的用例也守不住（138 个工具，新加一个就漏一个）。这里改成
// 从目录本身长出用例：任何工具、任何以后新加的工具，只要"声明了必填参数却在归一化里
// 把它丢了"，就会在这里当场红掉。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";
import { compileToolSchema } from "../src/agent/capabilities.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

function extractFn(name) {
  const m = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).exec(SRC);
  assert.ok(m, `main.js 里找不到 ${name}`);
  let depth = 0;
  let j = SRC.indexOf("{", SRC.indexOf(")", m.index));
  for (; j < SRC.length; j++) {
    const c = SRC[j];
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  return SRC.slice(m.index, j + 1);
}

function extractConst(name) {
  const m = new RegExp(`const\\s+${name}\\s*=`).exec(SRC);
  if (!m) return "";
  let i = SRC.indexOf("=", m.index) + 1, depth = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i];
    if (c === "{" || c === "[" || c === "(") depth++;
    else if (c === "}" || c === "]" || c === ")") depth--;
    else if ((c === ";" || c === "\n") && depth <= 0) break;
  }
  return SRC.slice(m.index, i + 1).replace(/;?$/, ";");
}

const EMPTY_CAPS = { tools: [], commands: [], disabled: [], errors: [] };

/** 真实内置目录（不含用户声明——那有自己的测试）。 */
function buildCatalog() {
  const build = new Function(
    "inTauri", "_applyCloudToolDescs", "_userCapabilities", "compileToolSchema", "_withoutDisabledTools",
    `${extractFn("_withoutDisabledTools")}\n${extractFn("_buildAgentToolSchemas")}\n;return _buildAgentToolSchemas;`,
  )(true, (t) => t, () => EMPTY_CAPS, compileToolSchema, undefined);
  return build(true, []);
}

/**
 * 真正的 _mapToolCall，归一化链（_normalizeArgKeys / _applyToolArgDefaults）用真实现。
 * 只有三个**提示词构造器**和 shell 探测缓存注入成桩：它们产出的是给子智能体的自然语言，
 * 不参与参数映射，注入真源码只会把整份提示词文本拖进来。
 */
function buildMapToolCall(catalog) {
  const prelude = [
    extractConst("_COMPUTER_METHODS"),
    // save_skill 的归一化要拼出技能落点（<工作区>/<产品目录>/skills/…）。产品目录名在
    // main.js 里只有一份（_STATE_DIR），沙箱按需注入它，别在测试里另写一个字面量。
    extractConst("_STATE_DIR"),
    extractFn("_normalizeArgKeys"),
    extractFn("_applyToolArgDefaults"),
    extractFn("_canonicalToolName"),
    extractFn("_finiteNumberArg"),
    // _normPlanSteps 取步骤文字走 _planStepText（带两张常量表）——少一个就当场
    // ReferenceError，而这条测试的全部意义就是"照 schema 填的参数不许让映射崩掉"。
    extractConst("_PLAN_STEP_TEXT_KEYS"),
    extractConst("_PLAN_STEP_META_KEYS"),
    extractFn("_planStepText"),
    extractFn("_normPlanSteps"),
    extractFn("_shellKind"),
  ].filter(Boolean).join("\n");
  return new Function(
    "USER_TOOL_PREFIX", "userToolShortName", "_userCapabilities",
    "_STR_ARG_KEYS", "_KNOWN_TOOLS", "_mcpToolMap", "_RETIRED_SEARCH_ALIASES", "t",
    "_PLAN_STEP_KINDS", "_RESEARCH_PROMPT", "_DESIGN_RESEARCH_PROMPT", "_WIKI_PROMPT", "_shellPlanCache",
    `${prelude}\n${extractFn("_mapToolCall")}\n;return _mapToolCall;`,
  )(
    "user__",
    (n) => (String(n || "").startsWith("user__") ? String(n).slice(6) : ""),
    () => EMPTY_CAPS,
    new Set(),
    new Set(catalog.map((e) => e?.function?.name).filter(Boolean)),
    new Map(), new Map(),
    (k) => String(k),
    new Set(["read", "edit", "run", "verify"]),
    () => "研究提示词", () => "设计调研提示词", () => "Wiki 提示词",
    new Map(),
  );
}

/**
 * 按 schema 造一个"模型会真的这么填"的值，并埋进可追踪的哨兵。
 * 两个讲究：数字取声明范围内（越界会被正当夹取，看起来就像丢了）；枚举取**最后一个**
 * （取第一个常常正好等于默认值，被丢了也看不出来）。
 */
function synthArg(prop, schema) {
  const type = Array.isArray(schema?.type) ? schema.type[0] : schema?.type;
  if (Array.isArray(schema?.enum) && schema.enum.length) return schema.enum[schema.enum.length - 1];
  const low = String(prop).toLowerCase();
  if (type === "number" || type === "integer") {
    const lo = Number.isFinite(schema?.minimum) ? schema.minimum : 1;
    const hi = Number.isFinite(schema?.maximum) ? schema.maximum : 8;
    return Math.min(hi, Math.max(lo, 3));
  }
  if (type === "boolean") return true;
  if (type === "array") return [synthArg(prop, schema.items || { type: "string" })];
  if (type === "object" || schema?.properties) {
    const obj = {};
    for (const [k, v] of Object.entries(schema.properties || {})) obj[k] = synthArg(k, v);
    if (!Object.keys(obj).length) obj[`k_${prop}`] = `SENT_${prop}_obj`;
    return obj;
  }
  if (/url|uri|href|endpoint|link/.test(low)) return `https://example.com/o/SENT_${prop}.git`;
  if (/^path$|paths|file|dir|folder|cwd|dest/.test(low)) return `/tmp/SENT_${prop}.txt`;
  return `SENT_${prop}`;
}

/** 一个值里所有可追踪的标量，按小写收集——归一化会 toLowerCase/toUpperCase，那不算丢。 */
function scalars(value, out = []) {
  if (value == null) return out;
  const t = typeof value;
  if (t === "string" || t === "number" || t === "boolean") out.push(String(value).toLowerCase());
  else if (Array.isArray(value)) value.forEach((v) => scalars(v, out));
  else if (t === "object") Object.values(value).forEach((v) => scalars(v, out));
  return out;
}

function sweep() {
  const catalog = buildCatalog();
  const mapToolCall = buildMapToolCall(catalog);
  const unmapped = [], threw = [], droppedRequired = [];
  for (const entry of catalog) {
    const name = entry?.function?.name;
    if (!name) continue;
    const params = entry.function.parameters || {};
    const required = new Set(params.required || []);
    const args = {};
    for (const [p, s] of Object.entries(params.properties || {})) args[p] = synthArg(p, s);

    let call;
    try {
      call = mapToolCall(name, args, new Map());
    } catch (e) {
      threw.push(`${name}: ${e.message}`);
      continue;
    }
    if (!call || typeof call !== "object") { unmapped.push(name); continue; }

    const haystack = scalars(call).join(String.fromCharCode(32));
    for (const p of required) {
      if (!(p in args)) continue;
      const needles = scalars(args[p]);
      if (!needles.length) continue;
      // 整块都不见了才算丢——部分子字段被裁剪是各工具自己的规则。
      if (needles.every((n) => !haystack.includes(n))) droppedRequired.push(`${name}.${p}`);
    }
  }
  return { catalog, unmapped, threw, droppedRequired };
}

test("目录里每个工具都能从自己的 schema 参数映射出一个可执行调用", () => {
  const { catalog, unmapped, threw } = sweep();
  assert.ok(catalog.length > 130, `目录只剩 ${catalog.length} 个工具，少了`);
  assert.deepEqual(threw, [], "归一化直接抛异常——模型照 schema 填的参数会让这一步崩掉");
  assert.deepEqual(unmapped, [],
    "工具在目录里、却映射不出调用对象——模型能看见它、调它，然后什么都不会发生");
});

test("必填参数不会在归一化里被静默丢掉", () => {
  // git_clone 就是死在这里：url 填了，归一化认不出形状，最后拿着空参数去执行，
  // 报的错还跟真正原因无关。这条断言按目录自动覆盖每一个工具的每一个必填参数。
  const { droppedRequired } = sweep();
  assert.deepEqual(droppedRequired, [],
    "必填参数在归一化后消失了——工具会拿着空参数去跑，错误信息还会指向别处");
});

test("git_clone 认得模型真会发的各种参数形状", () => {
  // 被实拍到的那个 bug 就在这里：模型填了仓库地址，归一化认不出键名/形状，
  // 拿着空 source 去执行。schema 层的断言（required 是不是 ["source"]）看不出这个，
  // 只有真把参数喂进去才看得出来。
  const catalog = buildCatalog();
  const map = buildMapToolCall(catalog);
  const clone = (args) => map("git_clone", args, new Map());
  const REPO = "https://github.com/vercel/next.js.git";

  // 键名怎么叫都得认：模型不会永远写 source。
  for (const key of ["source", "url", "repo", "repository", "remote"]) {
    const call = clone({ [key]: REPO });
    assert.equal(call.source, REPO, `${key} 这个键名没被认出来`);
    assert.equal(call.target, "next.js", "落地目录没从仓库名推出来");
  }
  // 落地目录同样有一堆叫法，给了就必须尊重，不能被推断覆盖。
  for (const key of ["target", "dest", "destination", "dir", "directory", "to"]) {
    assert.equal(clone({ url: REPO, [key]: "myapp" }).target, "myapp", `${key} 指定的目录被忽略了`);
  }
  assert.equal(clone({ repository: "git@github.com:vercel/next.js.git" }).target, "next.js",
    "SSH 形式的地址推不出目录名");

  // 粘网页链接：git clone 不动这种地址，而且照最后一段推会推出 "main"。
  assert.deepEqual(
    (({ source, target }) => ({ source, target }))(clone({ url: "https://github.com/foo/bar/tree/main" })),
    { source: "https://github.com/foo/bar", target: "bar" },
  );
  assert.deepEqual(
    (({ source, target }) => ({ source, target }))(clone({ url: "https://github.com/foo/bar/blob/main/src/app.ts" })),
    { source: "https://github.com/foo/bar", target: "bar" },
  );
  // 但真有仓库就叫 tree —— 那种不能被截掉。
  assert.deepEqual(
    (({ source, target }) => ({ source, target }))(clone({ url: "https://github.com/foo/tree" })),
    { source: "https://github.com/foo/tree", target: "tree" },
  );
});

test("网关那份工具目录和客户端目录不漂移", () => {
  // 走网关的用户（绝大多数）看到的 schema 来自 server/prompts/tools.json，执行却在客户端。
  // 名字或参数漂了，就会出现"模型被告知能填某个参数、客户端根本没声明"这种沉默失效。
  const catalog = buildCatalog();
  const client = new Map(catalog.map((e) => [e.function.name, e.function]));
  const raw = JSON.parse(readFileSync(join(HERE, "../../server/prompts/tools.json"), "utf8"));
  const list = Array.isArray(raw) ? raw : (raw.tools || Object.values(raw)[0]);
  const server = new Map(list.map((e) => { const f = e.function || e; return [f.name, f]; }));

  const onlyServer = [...server.keys()].filter((n) => !client.has(n));
  assert.deepEqual(onlyServer, [], "网关会把这些工具发给模型，客户端却执行不了");
  const onlyClient = [...client.keys()].filter((n) => !server.has(n));
  assert.deepEqual(onlyClient, [], "客户端能执行、网关没告诉模型——等于这些工具不存在");

  const paramDrift = [], reqDrift = [];
  for (const [name, sf] of server) {
    const cf = client.get(name);
    const sp = Object.keys(sf.parameters?.properties || {});
    const cp = Object.keys(cf.parameters?.properties || {});
    for (const p of sp) if (!cp.includes(p)) paramDrift.push(`${name}.${p} 只在网关有`);
    for (const p of cp) if (!sp.includes(p)) paramDrift.push(`${name}.${p} 只在客户端有`);
    const sr = (sf.parameters?.required || []).slice().sort().join(",");
    const cr = (cf.parameters?.required || []).slice().sort().join(",");
    if (sr !== cr) reqDrift.push(`${name}: 网关[${sr}] vs 客户端[${cr}]`);
  }
  assert.deepEqual(paramDrift, [], "参数声明漂移");
  assert.deepEqual(reqDrift, [], "必填声明漂移——一边拒收、另一边以为可以省");
});
