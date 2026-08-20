// 工具可达性测试。
//
// 要守住的性质只有一条：**注册表里的每个工具都够得着**。
// 在此之前够不着的有 121 个——开局窗口只放 11 个，search_tools 又是精确名查找，
// 于是模型既叫不出剩下那些的名字、也搜不到它们；语义编排器要等第一次工具调用之后
// 才介入。三条路同时堵死，工具写了等于没写。
//
// 这里的断言分三层：
//   1. TOOL_METADATA 必须覆盖整个注册表（漂移守卫——以后加工具忘了配元数据会红）
//   2. 完整能力名录必须把每个名字都列出来，且字节稳定（能待在 prompt cache 前缀里）
//   3. 意图声明 → 开局能力包：给定 N 种真实任务画像，断言该出现的工具确实在开局窗口里
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";
import { toolCapabilityIndex, TOOL_METADATA, CATEGORY_LABELS } from "../src/tool-guides.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

// main.js 没有导出，按名抠函数源码再注入依赖执行——测的是真正发出去的那份代码。
function extractFn(name) {
  const i = SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `main.js 里找不到 ${name}`);
  let depth = 0, j = SRC.indexOf("{", SRC.indexOf(")", i));
  for (; j < SRC.length; j++) {
    const c = SRC[j], d = SRC[j + 1];
    if (c === "/" && d === "/") { j = SRC.indexOf("\n", j); if (j < 0) j = SRC.length; continue; }
    if (c === "/" && d === "*") { j = SRC.indexOf("*/", j + 2) + 1; continue; }
    if (c === '"' || c === "'" || c === "`") {
      const quote = c;
      for (j++; j < SRC.length; j++) {
        if (SRC[j] === "\\") { j++; continue; }
        if (SRC[j] === quote) break;
      }
      continue;
    }
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  return SRC.slice(i, j + 1);
}

function registeredToolNames() {
  // 用户声明给空：本文件测的是**内置**注册表的覆盖度，用户自己接进来的能力
  // 有自己的测试（test/capabilities.test.mjs）。
  const build = new Function(
    "inTauri", "_applyCloudToolDescs", "_userCapabilities", "compileToolSchema", "_withoutDisabledTools",
    `${extractFn("_buildAgentToolSchemas")}\n;return _buildAgentToolSchemas;`,
  )(
    true, (tools) => tools,
    () => ({ tools: [], commands: [], disabled: [], errors: [] }),
    (t) => t, (tools) => tools,
  );
  return build(true, []).map((t) => String(t?.function?.name || "")).filter(Boolean);
}

const REGISTERED = registeredToolNames();
const INDEX = toolCapabilityIndex();

// ---- 1. 漂移守卫 ----------------------------------------------------------

test("TOOL_METADATA 覆盖注册表里的每一个工具", () => {
  const missing = REGISTERED.filter((name) => !TOOL_METADATA[name]);
  assert.deepEqual(missing, [],
    `这些工具没有 TOOL_METADATA，因而不会出现在能力名录里，模型叫不出它们的名字：${missing.join(", ")}`);
});

test("TOOL_METADATA 不含注册表里不存在的工具", () => {
  // search_tools 的 schema 是单独的常量（_SEARCH_TOOLS_SCHEMA），不在 _buildAgentToolSchemas 里，
  // 但它确实可调用，所以它出现在元数据里是对的。
  const registered = new Set([...REGISTERED, "search_tools"]);
  const ghosts = Object.keys(TOOL_METADATA).filter((name) => !registered.has(name));
  assert.deepEqual(ghosts, [], `名录会向模型宣传这些并不存在的工具：${ghosts.join(", ")}`);
});

test("每个工具的 category 都配了展示名", () => {
  const unlabeled = [...new Set(Object.values(TOOL_METADATA).map((m) => m?.category).filter(Boolean))]
    .filter((c) => !CATEGORY_LABELS[c]);
  assert.deepEqual(unlabeled, [], `这些分类没有展示名，名录里只会露出裸分类键：${unlabeled.join(", ")}`);
});

// ---- 2. 完整能力名录 ------------------------------------------------------

test("能力名录列出注册表里的每一个工具名", () => {
  // 名录现在是 `name(何时用)`，取名字要剥掉注解。
  const listed = new Set(INDEX.split("\n").flatMap((line) =>
    (line.split(": ")[1]?.split(" ") || []).map((entry) => entry.replace(/\(.*$/, ""))));
  const missing = [...REGISTERED, "search_tools"].filter((name) => !listed.has(name));
  assert.deepEqual(missing, [], `这些工具在名录里没有名字，模型无从调用：${missing.join(", ")}`);
});

test("能力名录字节稳定，不会击穿 prompt cache", () => {
  assert.equal(toolCapabilityIndex(), INDEX);
  // 名录随 system 前缀进缓存。只要它含有任务文本、时间戳或随机顺序，每轮前缀都不同，
  // 缓存全程失效——省下的那点 token 远抵不上重算整个前缀的代价。
  assert.doesNotMatch(INDEX, /\d{4}-\d{2}-\d{2}|\bMath\.random\b/);
  const lines = INDEX.split("\n");
  for (const line of lines) {
    const names = line.split(": ")[1]?.split(" ") || [];
    assert.deepEqual(names, names.slice().sort(), `${line.split(":")[0]} 内的工具名必须有序，否则输出不稳定`);
  }
});

test("能力名录进了随 system 前缀发送的工具提示", () => {
  const hint = extractFn("_buildToolHint");
  assert.match(hint, /toolCapabilityIndex\(\)/, "_buildToolHint 必须把完整名录带上");
  assert.match(hint, /自动装载|search_tools/,
    "名录必须同时告诉模型：没在开局窗口里也能直接按名调用");
});

test("能力名录的成本仍在预算内", () => {
  // 立场变过一次，理由写在这里。
  //
  // 原来是「只该有名字，不该有描述」，全量 133 个名字约 560 token。但光有名字，
  // `probe_env` / `ui_extract` / `remote` / `system` / `capture_start` 这种模型看了
  // 也不知道什么时候该伸手——于是那些能力结构性地永远轮不到，只能一个一个硬塞进
  // 开局窗口（窗口从 11 涨到 20，每个都按轮收注意力税，而下一处死胡同照样冒出来）。
  //
  // 现在每个名字后面带一句 ≤16 字的「何时用」，文件系统那几个名字自解释的不带。
  // 约 2000 token，换来 140 个能力都能被想到——不到硬塞那 20 个 schema
  // （约 7500 token）的三分之一。
  //
  // 4200 这条线守的是「只许一句话，不许把整段描述塞进来」：注解上限是 16 字，
  // 撞线说明有人在往里灌正文，那才是真的击穿预算。
  assert.ok(INDEX.length < 4200,
    `能力名录 ${INDEX.length} 字符，太大了——每条注解只该 ≤16 字，不该是整段描述`);
  // 名字自解释的那几个不许带注解，否则就是白花钱。
  assert.doesNotMatch(INDEX, /\bread_file\(/);
  assert.doesNotMatch(INDEX, /\blist_dir\(/);
  // 而名字说不清的那些必须带上，否则这次改动等于没做。
  for (const name of ["probe_env", "ui_extract", "save_skill", "find_symbol"]) {
    if (!INDEX.includes(name)) continue;
    assert.match(INDEX, new RegExp(`\\b${name}\\(`), `${name} 光看名字想不到什么时候用，必须带注解`);
  }
});

// ---- 3. 开局窗口保持最小 --------------------------------------------------

test("开局窗口不因画像而膨胀", () => {
  // 这是既有的、有意的立场：第一轮只发角色核心工具，其余由语义编排器和自愈装载
  // 按需装入。名录负责让模型"叫得出名字"，不负责把 schema 提前塞进每一轮载荷。
  const sel = extractFn("_selectInitialTools");
  assert.doesNotMatch(sel.replace(/\/\/[^\n]*/g, ""), /\bprofile\b[^)]*\.(intentEngineering|browserGoal|dataStrategy)/,
    "开局选择不得依据画像扩张工具集");
  const at = SRC.indexOf("const initialTools = _selectInitialTools(");
  assert.match(SRC.slice(at, at + 200), /run\.mode, null\)/,
    "开局选择必须保持画像无关");
});

test("运行中途的窗口重协调同样不带画像", () => {
  const at = SRC.indexOf("const desired = _selectInitialTools(");
  assert.ok(at > 0, "找不到重协调钩子里的 _selectInitialTools 调用");
  assert.match(SRC.slice(at, at + 200), /run\.mode, null\)/,
    "重协调钩子必须传 null，否则画像会在运行中途剪掉工具");
});

test("自愈装载兜住按名直调的未装载工具", () => {
  // 名录承诺"按名就能用"，兑现它的是这段自愈装载。它要是没了，名录就成了空头支票。
  const loop = extractFn("_runAgenticLoop");
  const heal = loop.slice(loop.indexOf("工具不丢失（自愈加载）"));
  assert.ok(heal.length > 0, "自愈装载块必须在场——名录的承诺全靠它兑现");
  assert.match(heal.slice(0, 1200), /_reg\.has\(_canonicalName\)/,
    "未装载但真实注册的工具必须从完整注册表里补进窗口");
});
