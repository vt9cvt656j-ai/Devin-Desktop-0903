// 自定义模型线协议：取值归一化、存量兼容、两侧目录不漂。
//
// 直接 import 产品代码验行为 —— 抠 main.js 源码文本验得到「代码长这样」，验不到「它还在
// 不在真实调用链上」。src/agent/wire-protocol.js 是纯函数模块，没有 DOM 依赖，可以这么验。
import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  CM_PROTOCOLS,
  CM_PROTOCOL_DEFAULT,
  CM_PROTOCOL_UI,
  cmProtocol,
  normalizeCustomModel,
} from "../src/agent/wire-protocol.js";

const HERE = path.dirname(fileURLToPath(import.meta.url));

test("cmProtocol：认不出的一律落 openai，认得出的大小写和空白都吃", () => {
  for (const raw of [undefined, null, "", "   ", "gemini", "bedrock", 0, 42, {}, [], "openai-compatible"]) {
    assert.equal(cmProtocol(raw), "openai", `${JSON.stringify(raw)} 应落回默认`);
  }
  for (const raw of ["anthropic", "ANTHROPIC", "  anthropic  ", "Anthropic"]) {
    assert.equal(cmProtocol(raw), "anthropic", `${JSON.stringify(raw)} 应认成 anthropic`);
  }
  assert.equal(cmProtocol("xai_responses"), "xai_responses");
  assert.equal(CM_PROTOCOL_DEFAULT, "openai");
});

test("存量条目零改动：没有 protocol 字段的老条目仍走 openai", () => {
  // 今天 localStorage 里就长这样 —— 一个 protocol 字段都没有。
  const legacy = { id: "custom:abc", group: "我的中转", name: "gpt-4o", baseUrl: "https://relay.example/v1", apiKey: "sk-x" };
  const n = normalizeCustomModel(legacy);
  assert.equal(n.protocol, "openai", "存量条目被改成别的协议 = 用户端点一夜之间全部打错");
  assert.equal(n.id, legacy.id);
  assert.equal(n.name, "gpt-4o");
  assert.equal(n.baseUrl, "https://relay.example/v1");
  assert.equal(n.apiKey, "sk-x");
  assert.equal(n.group, "我的中转");
});

test("normalizeCustomModel：组名空白落回默认、密钥缺失变空串", () => {
  const n = normalizeCustomModel({ id: "custom:x", group: "   ", name: "  m  ", baseUrl: " https://a/v1 ", apiKey: undefined, protocol: "anthropic" });
  assert.equal(n.group, "自定义模型");
  assert.equal(n.name, "m");
  assert.equal(n.baseUrl, "https://a/v1");
  assert.equal(n.apiKey, "");
  assert.equal(n.protocol, "anthropic");
});

test("文案表覆盖每一条协议 —— 漏一条 syncProto() 里 ui.ph 就抛，弹窗整个白掉", () => {
  assert.deepEqual(Object.keys(CM_PROTOCOL_UI).sort(), [...CM_PROTOCOLS].sort());
  for (const p of CM_PROTOCOLS) {
    const ui = CM_PROTOCOL_UI[p];
    assert.ok(ui.label && ui.label.length > 1, `${p} 缺 label`);
    assert.ok(ui.ph && ui.ph.startsWith("https://"), `${p} 的占位地址要是个真地址`);
    assert.ok(ui.hint && ui.hint.length > 20, `${p} 缺填写说明`);
    assert.ok(Array.isArray(ui.gaps), `${p} 缺 gaps 数组`);
  }
});

test("不许假装支持：非默认协议必须把能力缺口写出来", () => {
  for (const p of CM_PROTOCOLS.filter((x) => x !== CM_PROTOCOL_DEFAULT)) {
    const gaps = CM_PROTOCOL_UI[p].gaps;
    assert.ok(gaps.length > 0, `${p} 一条缺口都没写 —— 那是在假装它什么都支持`);
    for (const g of gaps) {
      assert.ok(g.length > 12, `${p} 的缺口说明太短，说不清用户会看到什么：${g}`);
    }
  }
});

test("桌面专属标记与 Rust 侧的能力一致：只有 openai 能在网页构建上跑", () => {
  assert.equal(CM_PROTOCOL_UI.openai.desktopOnly, false);
  assert.equal(CM_PROTOCOL_UI.anthropic.desktopOnly, true);
  assert.equal(CM_PROTOCOL_UI.xai_responses.desktopOnly, true);
});

test("两个目录不漂：JS 的取值集合必须是 Rust PROTOCOLS 的子集", () => {
  // 对不上时**没有任何报错**：Wire::of 认不出的字符串一律落 openai，表现成「界面选了
  // Anthropic、请求打到 /chat/completions」，回来一句 404，看着像地址填错了。
  const rs = fs.readFileSync(path.join(HERE, "..", "src-tauri", "src", "protocol.rs"), "utf8");
  const m = rs.match(/PROTOCOLS[^=]*=\s*(\[[\s\S]*?\]);/);
  assert.ok(m, "protocol.rs 里找不到 PROTOCOLS 常量 —— 改了名字就把这道门变成了摆设");
  const rustIds = new Set([...m[1].matchAll(/"([a-z_]+)"/g)].map((x) => x[1]));
  for (const p of CM_PROTOCOLS) {
    assert.ok(rustIds.has(p), `JS 有 "${p}" 而 Rust 侧没有 —— 选了它会静默落回 openai 打错端点`);
  }
});
