// 发布构建把 141 条工具描述**全部剥空**（IP 保护）。网关按名注回只补 body.tools——
// 补不到**消息正文里的工具目录**。
//
// 而那份目录正是两条认知路径的唯一依据：
//   · 编排模型用它决定「这轮装哪些工具」
//   · 收尾评审用它判断「模型选对工具了吗」
// 于是线上跑的那个构建里，它们看到的是 141 个光名字加同一句「（无描述）」。
//
// 实测（跑真 stripToolIp + 用剥后源码重建注册表）：
//   dev      141 个工具，空描述 0 条，空参数描述 10/412
//   release  141 个工具，空描述 **141 条**，空参数描述 **411/412**
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { enrichedCatalogLine, compactToolGuide, TOOL_METADATA } from "../src/tool-guides.js";
import { stripToolIp } from "../build/strip-tool-ip.mjs";
import { CODE as SRC } from "./helpers/source.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
// 工具目录字面量已搬到 src/agent/tool-catalog.js —— 两份拼起来读，
// 否则所有按 schema 文本的断言会以「这条工具不见了」的形式假红。
const RAW = readFileSync(join(HERE, "../src/main.js"), "utf8")
  + "\n" + readFileSync(join(HERE, "../src/agent/tool-catalog.js"), "utf8");

function registryFrom(src) {
  const build = /function _buildAgentToolSchemas\([\s\S]*?\n\}/.exec(src);
  const dis = /function _withoutDisabledTools\([\s\S]*?\n\}/.exec(src);
  assert.ok(build, "_buildAgentToolSchemas 抠不出来");
  // 目录字面量已搬进 src/agent/tool-catalog.js。这里**不能** import 真模块：
  // 这条测试要的正是「剥除之后那份源码构建出来的注册表」，import 拿到的永远是开发版。
  // 所以从传进来的 src 里把目录连同三个 getter 一起抠出来，随 builder 一起注入。
  const catAt = src.indexOf("const BASE = [");
  assert.ok(catAt > 0, "工具目录（tool-catalog.js 的 BASE）抠不出来");
  const catalog = src.slice(catAt).replace(/^export /gm, "");
  const fn = new Function("inTauri", "_applyCloudToolDescs", "_userCapabilities", "compileToolSchema", "_applyUserRoleEnums",
    `${catalog}\n${dis ? dis[0] : "const _withoutDisabledTools = (t) => t;"}\n${build[0]}\n;return _buildAgentToolSchemas;`)
    (true, (t) => t, () => ({ tools: [], commands: [], roles: [], disabled: [], errors: [] }), (t) => t, (t) => t);
  return fn(true, []);
}

const stripOut = stripToolIp(RAW);
const STRIPPED = typeof stripOut === "string" ? stripOut : (stripOut.code ?? stripOut.source);

test("发布构建确实会把描述剥空——这是这条测试存在的前提", () => {
  const dev = registryFrom(RAW);
  const rel = registryFrom(STRIPPED);
  assert.equal(dev.length, rel.length, "剥除改变了工具数量——它只该改描述文本");
  const devEmpty = dev.filter((t) => !String(t.function?.description || "").trim()).length;
  const relEmpty = rel.filter((t) => !String(t.function?.description || "").trim()).length;
  assert.equal(devEmpty, 0, "开发构建里就有空描述，那是另一个问题");
  assert.ok(relEmpty > 100,
    `发布构建只剥空了 ${relEmpty} 条描述——剥除范围变了，这条测试的前提要重核`);
});

test("三个派发工具的描述不许被剥空 —— 它们的 schema 真会随请求体发出去", () => {
  // 剥除的安全前提是「客户端只发工具名，网关按自己那份 tools.json 回填 schema」。
  // 这三个是唯一的例外：用户声明了自定义角色时，_applyUserRoleEnums 要把角色名补进
  // 它们的 `role` 枚举，补的是客户端这份 schema，于是 L0 那里会把它们从「只发名字」
  // 里摘出来、连整份 schema 一起发出去（网关那份目录里只有 11 个内置角色名）。
  //
  // 剥空的后果是：用户把角色的提示词、工具矩阵、轮数全配好了，模型收到的却是
  // `{name: "run_subagent", description: ""}` —— 整套自定义角色在**发布版**里是哑的，
  // 而开发版一切正常，所以本地根本测不出来。
  //
  // 这条测试和上面那些不是重复：上面守的是「客户端自己的两条认知路径有 TOOL_METADATA
  // 兜底」，这条守的是「真发给模型的那份 schema 里有没有字」——兜底到不了那里。
  const rel = registryFrom(STRIPPED);
  for (const name of ["run_subagent", "run_worker", "spawn_multiple_agents"]) {
    const t = rel.find((x) => x.function?.name === name);
    assert.ok(t, `${name} 不在注册表里了`);
    assert.ok(String(t.function.description || "").trim().length > 100,
      `${name} 的描述在发布构建里被剥空了 —— 声明了自定义角色的用户会拿到一个哑工具`);
  }
  // 反向：别把豁免开成一张越来越长的白名单。除这三个之外，描述该空还得空。
  const stillFilled = rel
    .filter((t) => String(t.function?.description || "").trim())
    .map((t) => t.function.name)
    .filter((n) => !["run_subagent", "run_worker", "spawn_multiple_agents"].includes(n));
  assert.deepEqual(stillFilled, [],
    "这些工具的描述没被剥掉，而它们的 schema 并不会随请求体发出去：" + stillFilled.join(","));
});

test("剥空之后，编排器看到的目录行仍然带真内容", () => {
  const rel = registryFrom(STRIPPED);
  const bad = [];
  for (const t of rel) {
    const name = t.function?.name;
    const line = enrichedCatalogLine({ name, description: t.function?.description || "", inputs: [], required: [] });
    if (line.includes("（无描述）")) bad.push(name);
  }
  assert.deepEqual(bad, [],
    `${bad.length} 个工具在发布构建里只有名字没有说明——编排模型据此决定这轮装哪些工具：`
    + bad.slice(0, 8).join(","));
});

test("剥空之后，紧凑指南也不再是常量占位符", () => {
  const rel = registryFrom(STRIPPED);
  const bad = rel
    .map((t) => [t.function?.name, compactToolGuide(t)])
    .filter(([, g]) => String(g).includes("需要该能力时使用"))
    .map(([n]) => n);
  assert.deepEqual(bad, [],
    `${bad.length} 个工具的紧凑指南退化成常量：${bad.slice(0, 8).join(",")}`);
});

test("兜底数据源本身不在剥除范围内（否则这条修法是空的）", () => {
  const g = readFileSync(join(HERE, "../src/tool-guides.js"), "utf8");
  const out = stripToolIp(g);
  const changed = typeof out === "object" ? (out.changed ?? 0) : 0;
  assert.equal(changed, 0,
    "TOOL_METADATA 所在的文件也被剥了——兜底和被兜底的一起没了");
});

test("兜底覆盖全部注册工具，没有漏网的", () => {
  const rel = registryFrom(STRIPPED);
  const missing = rel
    .map((t) => t.function?.name)
    .filter((n) => !(Array.isArray(TOOL_METADATA[n]?.use_cases) && TOOL_METADATA[n].use_cases.length));
  assert.deepEqual(missing, [],
    `这些工具在 TOOL_METADATA 里没有 use_cases，兜底会退回占位符：${missing.join(",")}`);
});

test("收尾评审那份目录（在 main.js 里，另一处渲染点）也走同一个兜底", () => {
  // enrichedCatalogLine 在 tool-guides.js，_criticToolCatalog 在 main.js —— 两处各写一遍
  // 同样的兜底，改一处漏一处是本仓库反复踩过的形状。
  const at = SRC.indexOf('entry.description || ');
  assert.ok(at > 0, "收尾评审的目录行渲染不见了");
  const seg = SRC.slice(at, at + 120);
  assert.match(seg, /_toolScenarioFallback\(entry\.name\)/,
    "收尾评审那份目录还在用常量占位符——它是「模型选对工具了吗」的唯一依据");
  assert.doesNotMatch(seg, /"（无描述）"/, "常量兜底还在");
});

test("兜底是数据不是新写的文案", () => {
  // 新写 141 条文案会和同一行已有的【场景】【触发器】重叠——那是拿新缺口换旧缺口。
  assert.match(SRC, /function _toolScenarioFallback\(name\)/, "兜底函数不见了");
  const body = /function _toolScenarioFallback\([\s\S]*?\n\}/.exec(SRC)[0];
  assert.match(body, /TOOL_METADATA\[String\(name \|\| ""\)\]/, "兜底没走 TOOL_METADATA");
  assert.match(body, /use_cases/, "兜底没用 use_cases");
  assert.ok(body.length < 400, "兜底函数里出现了人工文案——它只该取数据");
  // 更硬的判据：兜底的返回值必须**逐字来自** TOOL_METADATA，不能是任何写死的句子。
  // 新写 141 条文案会和同一行已有的【场景】【触发器】重叠，属于拿新缺口换旧缺口。
  const fallback = new Function("TOOL_METADATA", `${body}\n;return _toolScenarioFallback;`)(TOOL_METADATA);
  for (const n of ["package_source", "run_subagent", "knowledge_search", "web_fetch"]) {
    assert.equal(fallback(n), String(TOOL_METADATA[n].use_cases[0]).trim(),
      `${n} 的兜底不是 TOOL_METADATA 里那句——中间被人塞了写死的文案`);
  }
  assert.equal(fallback("不存在的工具"), "（无描述）", "未知工具的兜底应当如实说没有，不许编");
});

test("开发构建一个字节都没变", () => {
  // 这条改的是**兜底**，不是描述本身。dev 里描述非空，兜底不该被走到。
  const dev = registryFrom(RAW);
  for (const t of dev.slice(0, 30)) {
    const line = enrichedCatalogLine({ name: t.function?.name, description: t.function?.description, inputs: [], required: [] });
    assert.ok(line.includes(String(t.function.description).split("。")[0].slice(0, 20)),
      `${t.function?.name} 的真实描述被兜底顶掉了`);
  }
});
