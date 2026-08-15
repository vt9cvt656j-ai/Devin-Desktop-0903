// 声明接进去之后，链路是不是真的通。
//
// capabilities.test.mjs 测的是那个纯模块本身（一段 JSON → 一条工具声明）。这里测的是
// 另一半、也是更容易悄悄断掉的一半：**声明有没有真的走进工具目录、走进调用映射**。
//
// 这半边断掉的样子是最讨厌的一种——两边都编译、两边测试都绿，只是用户填了半天配置，
// 模型那边什么都没多出来，而且没有任何报错。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";
import { compileToolSchema, normalizeCapabilities } from "../src/agent/capabilities.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

// main.js 没有导出，按名抠函数源码再注入依赖执行——测的是真正发出去的那份代码。
function extractFn(name) {
  const i = SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `main.js 里找不到 ${name}`);
  let depth = 0;
  let j = SRC.indexOf("{", SRC.indexOf(")", i));
  for (; j < SRC.length; j++) {
    const c = SRC[j];
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  return SRC.slice(i, j + 1);
}

const CAPS = (raw) => normalizeCapabilities(raw, "测试配置");

/** 用给定的用户声明构建一次工具目录，返回工具名数组。 */
function toolNamesWith(caps) {
  const build = new Function(
    "inTauri", "_applyCloudToolDescs", "_userCapabilities", "compileToolSchema", "_withoutDisabledTools",
    `${extractFn("_withoutDisabledTools")}\n${extractFn("_buildAgentToolSchemas")}\n;return _buildAgentToolSchemas;`,
  )(
    true,
    (tools) => tools,
    () => caps,
    compileToolSchema,
    undefined, // 用上面抠出来的真实现，不注入桩
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
  const seg = src.slice(src.indexOf('call.type === "userhttp"'));
  assert.match(seg.slice(0, 1500), /buildHttpCall/, "没有用声明去合成请求");
});
