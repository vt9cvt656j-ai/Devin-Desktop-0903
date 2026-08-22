// search_tools：模型手里有 browser，却被告知"没这个能力"。
//
// 用户查的是 `browser_click` —— 模型最自然的查询形态，它就是照着工具命名法在猜名字。
// 三处叠起来的结果是一句彻底错误的结论：
//   ① 分词不拆 `_`/`-`，"browser_click" 是**一个** token，要求某个工具名逐字包含它才算
//      命中——恒不命中。模糊层零结果 → 快通道必空 → 等 MCP → 等编排器。
//   ② 编排器指向的 browser 已经在窗口里，adds 被 loaded 过滤成空；而「注册表里没有」
//      这条分支排在「编排器说够用」和「相关工具都已加载」**前面**，于是模型读到的是
//      「注册表没有 browser_click，按目标组合出来：web_search/web_fetch…」。
//      它据此放下手里的浏览器去抓网页，或者回用户「做不到」。
//   ③ 等完 MCP 发现之后只重算精确名，模糊层用的还是等待之前那份不全的目录——第一轮
//      问一个 MCP 服务名本可本地定下来，仍要再付一次上限 20 秒的编排器调用。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, SRC as RAW_SRC, fnSource, load, loadConst } from "./helpers/source.mjs";

const schema = (name, description = "") => ({ type: "function", function: { name, description } });
const registryOf = (...tools) => new Map(tools.map((t) => [t.function.name, t]));

const fuzzy = load("_searchToolsFuzzyMatch", {
  TOOL_METADATA: {
    browser: { triggers: ["在浏览器里点一下"], use_cases: ["操作真实网页"] },
    db_query: { triggers: ["查数据库"], use_cases: ["读工作区里的 sqlite"] },
  },
  autoEnrichToolMetadata: () => ({}),
});
const confident = load("_confidentFuzzyResolution");
const nearest = load("_nearestToolNames");

const REGISTRY = registryOf(
  schema("browser", "Drive a real browser: navigate, click, read the page"),
  schema("ui_click", "Click a coordinate on the desktop"),
  schema("web_fetch", "Fetch one URL"),
  schema("read_file", "Read a file"),
  schema("db_query", "Query a database"),
);

// ── ① 标识符形态的查询不再本地全盲 ────────────────────────────────────────

test("browser_click 这种查询要能在本地命中 browser", () => {
  const hits = fuzzy("browser_click", REGISTRY, new Set());
  assert.ok(hits.length, "含下划线的查询在本地零命中——快通道必空，只能去等 MCP + 编排器");
  assert.ok(hits.some((h) => h.name === "browser"), `命中里没有 browser：${hits.map((h) => h.name)}`);
});

test("连字符写法同样拆得开", () => {
  assert.ok(fuzzy("read-file", REGISTRY, new Set()).some((h) => h.name === "read_file"));
});

test("拆出来的子词是弱信号，不能顶成「判据明确」直接装工具", () => {
  // 片段按 1 分计，和 CJK 二元组同一档。整词命中才配 3 分——不然
  // "database_inspector" 里的 "database" 会把某个数据库工具直接顶成确定答案。
  const hits = fuzzy("database_inspector", REGISTRY, new Set());
  for (const hit of hits) {
    assert.ok(hit.score < 3, `${hit.name} 靠一个词片就拿到 ${hit.score} 分`);
  }
  assert.equal(confident(hits), null, "只靠词片就敢跳过语义编排，给错工具比慢更糟");
});

test("整词查询的分数一分没动", () => {
  // 排序逻辑一个字都不该改：拆子词只是补了原来根本不产生 token 的那一类查询。
  const spaced = fuzzy("browser click", REGISTRY, new Set()).find((h) => h.name === "browser");
  assert.ok(spaced.score >= 4, `整词命中掉到了 ${spaced.score} 分`);
  assert.ok(spaced.matchedOn.includes("name"));
});

test("拆子词不动精确名查询的契约：注册表里没有的复合名仍然不做词形猜测", () => {
  // _searchToolsLookup 是精确名通道，模糊只是它旁边的另一条路。这条边界不许被拆词模糊掉：
  // 一个查不到的复合名要落到语义编排，不能被拆成松散的词去撞出一个"差不多"的工具。
  const lookup = load("_searchToolsLookup", { _searchToolsExactQuery: load("_searchToolsExactQuery") });
  assert.deepEqual(lookup("local_discovery", registryOf(schema("web_fetch")), new Set()), []);
});

// ── ② 结论的顺序 ──────────────────────────────────────────────────────────

const RESULT = SRC.slice(
  RAW_SRC.indexOf("const rejectedNote = update.rejected.length"),
  RAW_SRC.indexOf("_settleToolStep(step, r, label)"),
);

test("「注册表里没有」要排在「编排器说够用」和「相关工具都已加载」之后", () => {
  assert.ok(RESULT.length > 500, "找不到 search_tools 的结果组装段");
  const at = (needle) => {
    const i = RESULT.indexOf(needle);
    assert.ok(i > 0, `结果组装段里找不到 ${needle}`);
    return i;
  };
  const miss = at("exact && !exact.schema");
  assert.ok(at("semanticDecision?.instruction") < miss,
    "编排器指向的工具已在窗口里时，模型读到的仍然是「注册表没有」——它会放下手里的工具去绕路");
  assert.ok(at("fuzzyHits.every((h) => h.alreadyLoaded)") < miss,
    "「相关工具均已加载」永远轮不到");
  assert.ok(at("adds.length") < miss, "「窗口装不下」也被这条挡在后面");
});

test("真的没有时先报名字相近的候选，只有连候选都没有才谈能力缺口", () => {
  assert.match(RESULT, /_nearestToolNames\(exact\.name, \[\.\.\.registry\.keys\(\)\]/,
    "没去注册表里找名字相近的工具，模型只会被推回去换个词再搜");
  assert.match(RESULT, /loaded\.has\(n\)/, "候选里已经在手上的那些没被标出来");
  assert.match(RESULT, /_near\.length[\s\S]{0,400}_CAPABILITY_ROUTES/,
    "候选为空这条路上没接换路清单");
});

test("候选取的是注册表里真实存在的名字", () => {
  assert.deepEqual(nearest("browser_click", [...REGISTRY.keys()], 5), ["browser"]);
  assert.deepEqual(nearest("readfile", [...REGISTRY.keys()], 5), ["read_file"]);
  // 给错方向比不给更糟：八竿子打不着的查询不许硬凑候选。
  assert.deepEqual(nearest("quantum_teleport", [...REGISTRY.keys()], 5), []);
});

// ── ③ 等完 MCP 发现之后要重跑快通道 ───────────────────────────────────────

test("等完 MCP 发现后用新目录重跑一次快通道，别白付一次编排器调用", () => {
  const loop = fnSource("_runAgenticLoop", { code: true });
  const at = loop.indexOf("await _waitForRunMcpDiscovery(run);");
  assert.ok(at > 0, "慢路径上等 MCP 那一步不见了");
  const block = loop.slice(at, at + 900);
  assert.match(block, /fastHits = _searchToolsFuzzyMatch\(call\.query, registry, loaded\)/,
    "只重算了精确名，模糊层用的还是等待之前那份不全的目录");
  assert.match(block, /_confidentFuzzyResolution\(fastHits\)[\s\S]{0,200}fastAdds =/,
    "重算了却没喂给同一套判据，命中也还是要去发那次编排器调用");
});

// ── ④ 文案里的数字对齐真实常量 ────────────────────────────────────────────

test("窗口装不下的说法引用真实上限，不是写死的 128", () => {
  const maxTools = loadConst("_TOOL_PAYLOAD_MAX_TOOLS");
  assert.equal(maxTools, 256, "上限变了，下面这两句话跟着变才有意义");
  assert.equal((RESULT.match(/\$\{_TOOL_PAYLOAD_MAX_TOOLS\} tools/g) || []).length, 2,
    "两处「窗口装不下」的文案要引用同一个常量");
  assert.ok(!RESULT.includes("128 tools"),
    "还在告诉模型窗口是 128 个工具——它会照着这个数去做没必要的裁剪");
});
