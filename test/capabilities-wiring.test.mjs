// 声明接进去之后，链路是不是真的通。
//
// capabilities.test.mjs 测的是那个纯模块本身（一段 JSON → 一条工具声明）。这里测的是
// 另一半、也是更容易悄悄断掉的一半：**声明有没有真的走进工具目录、走进调用映射**。
//
// 这半边断掉的样子是最讨厌的一种——两边都编译、两边测试都绿，只是用户填了半天配置，
// 模型那边什么都没多出来，而且没有任何报错。
import { readFileSync } from "node:fs";
import { baseTools, readonlyExternalTools, writeTools } from "../src/agent/tool-catalog.js";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";
import { compileToolSchema, normalizeCapabilities } from "../src/agent/capabilities.js";

const HERE = dirname(fileURLToPath(import.meta.url));
// 正向源码断言必须跑在**剥掉注释**的源码上。注释不是代码：把一条契约从代码里删掉、
// 只在注释里留一句，assert.match 照样绿——本仓库已经这样漏过一整组模型可见的工具契约。
// 所以 `SRC` 绑定的是 CODE（注释整段置空，行号与偏移和原文一字不差）；
// 真要匹配注释本身的断言显式用 RAW_SRC，并在那一行写清为什么。
import { CODE as SRC, SRC as RAW_SRC, fnSource as extractFn, TOOL_CATALOG_SRC } from "./helpers/source.mjs";

// main.js 没有导出，按名抠函数源码再注入依赖执行——测的是真正发出去的那份代码。

const CAPS = (raw) => normalizeCapabilities(raw, "测试配置");

/** 用给定的用户声明构建一次工具目录，返回工具名数组。 */
function toolNamesWith(caps) {
  const build = new Function(
    "inTauri", "_applyCloudToolDescs", "_userCapabilities", "compileToolSchema", "_withoutDisabledTools", "baseTools", "readonlyExternalTools", "writeTools",
    `${extractFn("_applyUserRoleEnums")}\n${extractFn("_withoutDisabledTools")}\n${extractFn("_buildAgentToolSchemas")}\n;return _buildAgentToolSchemas;`,
  )(
    true,
    (tools) => tools,
    () => caps,
    compileToolSchema,
    undefined, // 用上面抠出来的真实现，不注入桩
  
    // 目录字面量已搬进 src/agent/tool-catalog.js —— 三个 getter 从模块注入。
    baseTools, readonlyExternalTools, writeTools,
  );
  return build(true, []).map((t) => String(t?.function?.name || "")).filter(Boolean);
}

test("填一段声明，模型的工具清单里就真的多出这个工具", () => {
  const caps = CAPS({
    tools: [{
      name: "acme_tickets",
      description: "查 ACME 内部工单",
      parameters: { query: { type: "string", required: true } },
      http: { url: "https://intra.acme.com/api/tickets?q={query}" },
    }],
  });
  const names = toolNamesWith(caps);
  assert.ok(names.includes("user__acme_tickets"), "声明没走进工具目录——用户填了配置，模型那边什么都没多");
  // 前缀不是装饰：靠它，这个名字才落在 _staticToolNames() 之外，schema 会随请求体发出，
  // 而不是指望网关那份产品目录（用户改不了那份）。
  assert.ok(names.filter((n) => n.startsWith("user__")).length === 1);
});

test("没有任何声明时，工具清单和以前一模一样", () => {
  const empty = toolNamesWith({ tools: [], commands: [], disabled: [], errors: [] });
  assert.ok(empty.length > 100, `内置工具数量不对：${empty.length}`);
  assert.equal(empty.filter((n) => n.startsWith("user__")).length, 0);
});

test("关掉一个内置工具，它从源头就不出现在清单里", () => {
  const before = toolNamesWith({ tools: [], commands: [], disabled: [], errors: [] });
  // 用户写的是**工具名**（generate_image），不是内部类型名（genimage）——旧的权限规则
  // 恰恰比的是内部类型名，所以用户写 web_search 根本不生效。这条测试同时钉住这个方向。
  assert.ok(before.includes("generate_image"), "前提不成立：generate_image 本来就不在清单里");
  const after = toolNamesWith(CAPS({ disabled: ["generate_image"] }));
  assert.ok(!after.includes("generate_image"), "关掉的工具还在清单里——那模型照样会调它，然后被执行前的门拒掉，白烧一轮");
  assert.equal(after.length, before.length - 1, "只该少这一个");
});

test("工具名 → 调用，走的是一个泛型类型，不是一个工具一个分支", () => {
  const caps = CAPS({
    tools: [{ name: "acme", description: "x", http: { url: "https://a.com/{id}" }, parameters: { id: { type: "string" } } }],
  });
  const map = new Function(
    "USER_TOOL_PREFIX", "userToolShortName", "_userCapabilities",
    "_applyToolArgDefaults", "_normalizeArgKeys", "_STR_ARG_KEYS", "_KNOWN_TOOLS",
    "_canonicalToolName", "_mcpToolMap", "_RETIRED_SEARCH_ALIASES",
    `${extractFn("_mapToolCall")}\n;return _mapToolCall;`,
  )(
    "user__", (n) => (String(n || "").startsWith("user__") ? String(n).slice(6) : ""),
    () => caps,
    undefined, (a) => a, new Set(), new Set(), () => "", new Map(), new Map(),
  );
  const call = map("user__acme", { id: "42" }, new Map());
  assert.equal(call.type, "userhttp", "没有映射成泛型类型 —— 那就意味着每加一个接口都要改代码");
  assert.equal(call.userName, "acme");
  assert.ok(call.userDef, "声明没被带上，执行时就拼不出请求");
  // GET 默认只读，于是 Plan / Explorer 这些只读模式里也能用它查东西。
  assert.equal(call.userReadOnly, true);
});

test("声明被删掉之后，同名调用不会拿着旧声明继续跑", () => {
  const map = new Function(
    "USER_TOOL_PREFIX", "userToolShortName", "_userCapabilities",
    "_applyToolArgDefaults", "_normalizeArgKeys", "_STR_ARG_KEYS", "_KNOWN_TOOLS",
    "_canonicalToolName", "_mcpToolMap", "_RETIRED_SEARCH_ALIASES",
    `${extractFn("_mapToolCall")}\n;return _mapToolCall;`,
  )(
    "user__", (n) => (String(n || "").startsWith("user__") ? String(n).slice(6) : ""),
    () => ({ tools: [], commands: [], disabled: [], errors: [] }),
    undefined, (a) => a, new Set(), new Set(), () => "", new Map(), new Map(),
  );
  const call = map("user__acme", {}, new Map());
  assert.equal(call.type, "userhttp");
  assert.equal(call.userDef, null, "声明已经没了，就必须带 null 出去，让执行侧如实报「找不到声明」");
});

test("执行侧确实复用了内置的 http 通道，没有另抄一份", () => {
  // 抄一份的代价不是重复代码，是行为分叉：幂等重试判定、重定向提示、错误文案会各走各的，
  // 而这些都是内置 http 分支里已经调好的东西。
  const src = extractFn("_executeToolStepInner");
  assert.match(src, /call\.type === "http" \|\| call\.type === "userhttp"/,
    "用户能力没有并进内置 http 分支");
  const seg = src.slice(src.indexOf('call.type === "http" || call.type === "userhttp"'));
  assert.match(seg.slice(0, 1500), /buildHttpCall/, "没有用声明去合成请求");
});

/**
 * settings.json 读不懂时必须报出来，不能静默吞掉。
 *
 * 这一份文件同时喂着能力声明和权限规则，而两处 absorb 原来都是 `catch { return; }`。
 * 后果不是"少一条声明"：用户配好的东西全部消失，而能力面板把 errors 排在最前面标红
 * ——那正是这个面板存在的首要理由——此时 errors 是空数组，用户看到的是「我从来没配过」。
 *
 * 同一个仓库里 hooks 加载器为同一种情况早就写好了正确的规矩（剥 BOM、真空才静默、
 * 否则吵）。这几条钉住 capabilities 这侧跟上了。
 */
test("settings.json 格式坏掉时，面板能拿到红色的原因", () => {
  const parse = extractFn("_parseSettingsJson");
  assert.ok(parse, "共用的解析函数不见了，这条断言失去落点");
  assert.doesNotMatch(SRC, /try \{ parsed = JSON\.parse\(raw \|\| "\{\}"\); \} catch \{ return; \}/,
    "静默吞掉又回来了——用户配的东西全没了，而面板显示「你从来没配过」");
  // 两个 absorb 都必须改过来（同一份文件同时喂能力和权限）。
  assert.match(SRC, /const \{ parsed \} = _parseSettingsJson\(raw, from \|\| "权限设置"\)/,
    "权限那侧还在静默吞");
  assert.match(SRC, /const \{ parsed, error \} = _parseSettingsJson\(raw, source\)/,
    "能力那侧还在静默吞");
  assert.match(SRC, /if \(error\) \{ scopes\.push\(\{ errors: \[error\] \}\); return; \}/,
    "报错没进 scopes —— 面板还是看不到");
});

test("BOM 不算格式错误，真空才静默", () => {
  // Windows 上一份内容完全合法、只是带 BOM 的 settings.json，会让能力声明连同
  // 权限规则一起消失。hooks 那侧已经为一次实拍加了这个处理，这侧当时没跟上。
  // 去重用的模块级 let 要一起注进来（它在函数外面，抠函数抠不到）。
  const parse = new Function("showToast", `let _settingsParseWarned = "";\n${extractFn("_parseSettingsJson")}\n;return _parseSettingsJson;`)(() => {});
  const good = parse('\uFEFF{"capabilities":{"tools":[]}}', "settings.json");
  assert.equal(good.error, "", "带 BOM 的合法 JSON 被判成坏文件了");
  assert.ok(good.parsed && good.parsed.capabilities, "剥掉 BOM 之后没解析出内容");
  // 真空 → 静默，不报错（没配过不是错误）。
  assert.deepEqual(parse("\uFEFF   \n", "settings.json"), { parsed: null, error: "" });
  assert.deepEqual(parse("", "settings.json"), { parsed: null, error: "" });
  // 真的坏 → 有错，且文案要说清「权限规则也一起失效了」。
  const bad = parse('{"capabilities":', "settings.json");
  assert.match(bad.error, /settings\.json/, "报错里没说是哪份文件");
  assert.match(bad.error, /权限规则/, "没告诉用户权限规则也一起失效了——那是更危险的那一半");
  assert.equal(bad.parsed, null);
});

/**
 * 用户自己声明的能力，在**默认线路**上必须真的到达模型。
 *
 * 2026-08-26 体检查出的最严重一条。代码里那句注释写着「名字带 user__ 前缀，于是它天然
 * 落在 _staticToolNames() 之外，schema 会随请求体一起发出」——**那句是假的**：
 * `_staticToolNames()` 正是从 `_buildToolRegistry(true)` 建的，而它遍历的就是
 * `_buildAgentToolSchemas` 的返回值，用户工具就在那里面被推进来。前缀不构成任何豁免。
 *
 * 后果：走网关时 L0 会把它的 schema 丢掉、只发名字，而网关按**自己那份产品目录**回填
 * ——那份目录里没有用户的工具。于是用户在 settings.json 里接进来的能力，
 * 在默认线路上整条消失：模型看不见、也调不动。
 *
 * 那句注释此前**没有任何断言落点**，所以它一直假着也没人发现。这一组补上落点。
 */
test("用户声明的工具确实在静态目录里——所以豁免必须显式写，不能靠前缀「天然」成立", () => {
  // 先证伪那句老注释：user__ 工具就在 _buildAgentToolSchemas 的产出里。
  const build = new Function(
    "inTauri", "_applyCloudToolDescs", "_userCapabilities", "compileToolSchema",
    "_withoutDisabledTools", "_applyUserRoleEnums", "baseTools", "readonlyExternalTools", "writeTools",
    `${extractFn("_buildAgentToolSchemas")}\n;return _buildAgentToolSchemas;`,
  )(
    true, (t) => t,
    () => ({ tools: [{ name: "user__probe", description: "d", parameters: {} }], roles: [], commands: [], disabled: [], errors: [] }),
    (t) => ({ type: "function", function: { name: t.name, description: t.description, parameters: { type: "object", properties: {} } } }),
    (t) => t, (t) => t, baseTools, readonlyExternalTools, writeTools,
  );
  const names = build(true, []).map((t) => t?.function?.name).filter(Boolean);
  assert.ok(names.includes("user__probe"),
    "用户工具不在目录里了——那这一整条判据的前提变了，要重新核");
  // 而 _staticToolNames() 就是从这份目录建的，所以它必然也在里面。
  assert.match(extractFn("_staticToolNames"), /_buildToolRegistry\(true\)/,
    "静态名单换来源了——上面那条推理要重做");
});

test("L0 拆分显式豁免 user__，不把它交给网关回填", () => {
  const turn = extractFn("_agentModelTurn");
  assert.match(turn, /_userDeclared = typeof _n === "string" && _n\.startsWith\("user__"\)/,
    "没有显式豁免——用户声明的能力会被 L0 丢掉，而网关目录里没有它");
  assert.match(turn, /!_userDeclared/, "豁免算出来了却没接进 _delegate 的判据");
  // 反向：不许再退回「靠前缀天然成立」的说法。
  // 允许它出现在「以前写着…那句是假的」这种更正段落里，不允许被当成现状陈述。
  // （不这么写的话，这条断言会命中更正文本自己——本仓库为同一个形状栽过两次。）
  const src = readFileSync(join(HERE, "..", "src/main.js"), "utf8");
  const at = src.indexOf("天然落在 _staticToolNames() 之外");
  if (at >= 0) {
    const around = src.slice(Math.max(0, at - 300), at + 300);
    assert.match(around, /以前写着|那句是假的|已更正/,
      "那句假注释又被当成现状写回来了——它让人以为不需要豁免");
  }
});

test("声明变了要让静态名单失效，否则第一次调用就冻住整个进程", () => {
  const refresh = extractFn("_refreshUserCapabilities");
  assert.match(refresh, /__staticToolNames = null;/,
    "刷新能力之后没清 memo —— 开 app 时还没读到声明的话，之后读到了也永远进不去名单");
});
