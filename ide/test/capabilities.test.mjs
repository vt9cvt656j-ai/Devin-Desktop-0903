// 用户声明的能力。
//
// 这个模块是纯的，所以这里直接 import 它来跑，而不是从 main.js 里按名字抠源码——
// 抠源码的断言钉的是代码形状，改进一次就红一次（tool-policy.test.mjs 开头解释过）。
//
// 重点守两类性质：
//   1. **写错不会静默消失。** 用户手写 JSON 一定会写错，一个错字让整份配置无声无息
//      地不生效，是最难查的失败。坏的那条要被丢掉并报出来，好的那些照常生效。
//   2. **插值不能变成漏洞。** URL 里插模型给的参数必须 URL 编码，否则一个 `&admin=1`
//      就改写了查询串；而放 token 的 `${VAR}` 恰恰不能编码。两种插值规则不同，混了就错。
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  USER_TOOL_PREFIX,
  buildHttpCall,
  compileToolSchema,
  mergeCapabilities,
  normalizeCapabilities,
  userToolShortName,
} from "../src/agent/capabilities.js";

const ok = (raw, source = "~/.mrdayone/settings.json") => normalizeCapabilities(raw, source);

test("一段 JSON 就能加一个工具，不用改代码", () => {
  const caps = ok({
    capabilities: {
      tools: [{
        name: "acme_tickets",
        description: "查 ACME 内部工单",
        parameters: { query: { type: "string", description: "关键词", required: true } },
        http: { url: "https://intra.acme.com/api/tickets?q={query}", headers: { Authorization: "Bearer ${ACME_TOKEN}" } },
      }],
    },
  });
  assert.equal(caps.errors.length, 0, caps.errors.join(" / "));
  assert.equal(caps.tools.length, 1);
  const t = caps.tools[0];
  assert.equal(t.toolName, USER_TOOL_PREFIX + "acme_tickets");
  assert.equal(t.http.method, "GET", "没写 method 时默认 GET");
  const schema = compileToolSchema(t);
  assert.equal(schema.function.name, "user__acme_tickets");
  assert.deepEqual(schema.function.parameters.required, ["query"]);
  // 描述里要带上来源：模型据此知道这是用户特地接进来的，不是产品内置的。
  assert.match(schema.function.description, /查 ACME 内部工单/);
  assert.match(schema.function.description, /settings\.json/);
});

test("顶层直接写和包在 capabilities 下，两种写法都认", () => {
  const a = ok({ capabilities: { disabled: ["genimage"] } });
  const b = ok({ disabled: ["genimage"] });
  assert.deepEqual(a.disabled, ["genimage"]);
  assert.deepEqual(b.disabled, ["genimage"]);
});

test("写错的那一条被丢掉并报出来，写对的照常生效", () => {
  const caps = ok({
    tools: [
      { name: "好的", description: "x", http: { url: "https://a.com" } },        // 名字非法
      { name: "no_desc", http: { url: "https://a.com" } },                        // 缺描述
      { name: "bad_url", description: "x", http: { url: "ftp://a.com" } },        // 协议不对
      { name: "bad_method", description: "x", http: { url: "https://a.com", method: "TRACE" } },
      { name: "good_one", description: "这条是对的", http: { url: "https://a.com" } },
    ],
  });
  assert.equal(caps.tools.length, 1, "只有对的那条该活下来");
  assert.equal(caps.tools[0].name, "good_one");
  assert.equal(caps.errors.length, 4, "四条错误都要被报出来，不能静默吞掉");
  assert.ok(caps.errors.some((e) => e.includes("description")), "缺描述要说清楚");
});

test("完全不是配置的东西，不会让它炸", () => {
  for (const junk of [null, undefined, 42, "字符串", [], { tools: "不是数组" }, { tools: [null, 7] }]) {
    const caps = normalizeCapabilities(junk);
    assert.ok(Array.isArray(caps.tools) && Array.isArray(caps.errors));
  }
});

test("URL 里插模型给的参数必须 URL 编码——否则一个 & 就改写了查询串", () => {
  const [t] = ok({
    tools: [{
      name: "search", description: "搜",
      parameters: { q: { type: "string" } },
      http: { url: "https://a.com/s?q={q}&safe=1" },
    }],
  }).tools;
  const call = buildHttpCall(t, { q: "x&safe=0&admin=1" });
  assert.ok(!call.url.includes("admin=1&"), "参数没被编码，注入进了查询串");
  assert.match(call.url, /q=x%26safe%3D0%26admin%3D1/);
  assert.match(call.url, /&safe=1$/, "声明里自己的参数必须原样保留");
});

test("放 token 的 ${VAR} 取自环境变量，且不做 URL 编码", () => {
  const [t] = ok({
    tools: [{
      name: "tk", description: "x",
      http: { url: "https://a.com", headers: { Authorization: "Bearer ${TOKEN}" } },
    }],
  }).tools;
  const call = buildHttpCall(t, {}, { TOKEN: "abc/def+ghi=" });
  assert.equal(call.headers.Authorization, "Bearer abc/def+ghi=", "密钥被编码就用不了了");
  // 取不到这个变量时**不发请求**，而不是替换成空串照发。也绝不能把 `${TOKEN}` 原样
  // 发出去（那等于把占位符当密钥交给对端）。
  assert.throws(() => buildHttpCall(t, {}, {}), /未定义的环境变量/);
  assert.throws(() => buildHttpCall(t, {}, { TOKEN: "" }), /未定义的环境变量/,
    "空字符串和没设一样，都不该被当成真密钥发出去");
});

test("body 里的参数按 JSON 转义，不能把 JSON 撑破", () => {
  const [t] = ok({
    tools: [{
      name: "post", description: "x",
      parameters: { text: { type: "string" } },
      http: { url: "https://a.com", method: "POST", body: '{"q":"{text}"}' },
    }],
  }).tools;
  const call = buildHttpCall(t, { text: '他说"你好"\n换行' });
  assert.equal(call.method, "POST");
  const parsed = JSON.parse(call.body);
  assert.equal(parsed.q, '他说"你好"\n换行', "转义之后必须还能解析回原文");
});

test("插值之后如果 URL 不再是 http/https，拒绝发出去", () => {
  const [t] = ok({
    tools: [{ name: "based", description: "base 放在环境变量里", http: { url: "${BASE}/api" } }],
  }).tools;
  assert.ok(t, "url 以 ${VAR} 开头是合法写法，不该在声明期就被拒");
  assert.throws(() => buildHttpCall(t, {}, { BASE: "file:///etc" }), /不再是 http/);
  // BASE 没设时先撞未定义变量那道门——同样是拒发，只是把原因说得更准。
  assert.throws(() => buildHttpCall(t, {}, {}), /未定义的环境变量/);
});

test("斜杠命令：/ 可写可不写，缺 prompt 的被拒", () => {
  const caps = ok({
    commands: [
      { cmd: "/review", desc: "按团队清单评审", prompt: "请按 CONTRIBUTING.md 的清单评审这次改动" },
      { cmd: "deploy-staging", prompt: "跑 scripts/deploy.sh staging 并把输出贴回来" },
      { cmd: "broken" },
    ],
  });
  assert.equal(caps.commands.length, 2);
  // 不带前导斜杠——斜杠菜单比的是 `/` 后面那一截
  assert.deepEqual(caps.commands.map((c) => c.cmd), ["review", "deploy-staging"]);
  assert.equal(caps.commands[1].desc.length > 0, true, "没写 desc 时用 prompt 开头兜底");
  assert.ok(caps.errors.some((e) => e.includes("prompt")));
});

test("多作用域合并：先给的先占名字，disabled 取并集", () => {
  const home = ok({ tools: [{ name: "tk", description: "个人版", http: { url: "https://home" } }], disabled: ["a"] }, "home");
  const proj = ok({ tools: [{ name: "tk", description: "项目版", http: { url: "https://proj" } }], disabled: ["b"] }, "proj");
  const merged = mergeCapabilities([home, proj]);
  assert.equal(merged.tools.length, 1);
  assert.equal(merged.tools[0].http.url, "https://home", "先给的作用域优先");
  // 关掉是并集：一个作用域说「别用这个」，另一个不该把它打开。
  assert.deepEqual(merged.disabled.sort(), ["a", "b"]);
});

test("短名能从全名还原，非用户工具返回空", () => {
  assert.equal(userToolShortName("user__acme"), "acme");
  assert.equal(userToolShortName("read_file"), "");
  assert.equal(userToolShortName(""), "");
  assert.equal(userToolShortName(null), "");
});

test("一份坏文件不能把工具窗口挤爆", () => {
  const many = Array.from({ length: 500 }, (_, i) => ({
    name: `t${i}`, description: "x", http: { url: "https://a.com" },
  }));
  const caps = ok({ tools: many });
  assert.ok(caps.tools.length <= 64, `上限没生效，收了 ${caps.tools.length} 个`);
});
