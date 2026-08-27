// 前缀缓存被打碎的两个来源，以及修好之后要守住的不变量。
//
// 上游 prompt cache 按前缀逐字节匹配：历史中段每改一个字，其后全部 0 命中。这里守两件事：
//   1. 历史里的图片消息：投影只看这条消息自己，永远不随「本轮说了什么 / 有没有带图」改写；
//      本轮要回看的旧图附在**本轮** user 消息末尾（新后缀，不打碎任何前缀）。
//   2. 场景→工具直觉表 + 完整能力名录：字节稳定的纯字面量只有待在 system 末尾才有缓存收益；
//      拼在每轮 user 消息尾部是每轮重付，折叠开场消息时还会整段消失。
//
// 真函数从 src/main.js 里用 acorn 抠出来跑（test/helpers/source.mjs 的 fnSource），不复刻逻辑。
// 源码断言先剥注释再匹配：注释里引用旧代码就能把断言喂绿。
import assert from "node:assert/strict";
import test from "node:test";
import { fnSource, load, loadConst } from "./helpers/source.mjs";
import { toolCapabilityIndex } from "../src/tool-guides.js";

/** 剥了注释的函数源码：正向源码断言跑在这上面。 */
const grabCode = (name) => fnSource(name, { code: true });

// ---------------------------------------------------------------------------
// 1. 历史里的图片消息
// ---------------------------------------------------------------------------
function mediaHistoryLoader(calls) {
  return load("_memoryMessagesForModel", {
    _stripAckOpeners: (s) => s,
    _stripTeachingSections: (s) => s,
    _priorMediaForTurn: load("_priorMediaForTurn", {
      _isImageLocationRequest: (text, hasImageContext) => hasImageContext && /哪里|定位/.test(String(text)),
      _attachmentAwareContent: async (text, attachments, _config, _budget, forced) => {
        calls.push({ name: attachments[0].name, forced });
        return [{ type: "text", text }, { type: "image_url", image_url: { url: `data:${attachments[0].name}` } }];
      },
    }),
  });
}

const memory = { assemble: () => [
  { role: "user", content: "第一张截图", attachments: [{ kind: "image", name: "a.png" }] },
  { role: "assistant", content: "看过了 a" },
  { role: "user", content: "改一下间距" },
  { role: "assistant", content: "改好了" },
  { role: "user", content: "第二张截图", attachments: [{ kind: "image", name: "b.png" }] },
  { role: "assistant", content: "看过了 b" },
] };

test("the history projection is byte-identical whatever the current turn says or attaches", async () => {
  // 「截图、说一句、再截图」这种 UI 调试会话原来几乎每轮都翻：带图一轮 → 之前的图全变文本，
  // 下一轮纯文字 → 又全变回图片数组。同一条历史消息在相邻两轮形态不同，前缀从最早一张图处作废。
  const calls = [];
  const rebuild = mediaHistoryLoader(calls);
  const textTurn = await rebuild(memory, { model: "vision" }, "这里还是不对", false);
  const mediaTurn = await rebuild(memory, { model: "vision" }, "现在这样", true);
  const referencingTurn = await rebuild(memory, { model: "vision" }, "和上一张比一下", true);
  const allTurn = await rebuild(memory, { model: "vision" }, "把之前所有图片都对一遍", true);

  const shape = (r) => JSON.stringify(r.messages);
  assert.equal(shape(mediaTurn), shape(textTurn), "a turn with its own attachment rewrote history");
  assert.equal(shape(referencingTurn), shape(textTurn), "an explicit back-reference rewrote history");
  assert.equal(shape(allTurn), shape(textTurn));
  assert.ok(textTurn.messages.every((m) => typeof m.content === "string"), "history never carries image arrays");
  assert.equal(textTurn.messages.length, 6);
  assert.ok(!("attachments" in textTurn.messages[0]), "IDE-only attachment fields are stripped");
  assert.match(textTurn.messages[0].content, /^第一张截图\n（该轮曾附带 1 个媒体文件/);

  // 可见性判据没变——变的只是位置：选中的旧图在 priorMedia 里，由调用方附到本轮消息末尾。
  assert.deepEqual(textTurn.priorMedia.map((p) => p.image_url?.url).filter(Boolean), ["data:a.png", "data:b.png"]);
  assert.deepEqual(mediaTurn.priorMedia, []);
  assert.deepEqual(referencingTurn.priorMedia.map((p) => p.image_url?.url).filter(Boolean), ["data:b.png"]);
  assert.deepEqual(allTurn.priorMedia.map((p) => p.image_url?.url).filter(Boolean), ["data:a.png", "data:b.png"]);
  // 每段回看都指回原消息，模型才知道这是哪一轮的图，且说明它不是本轮新附件。
  assert.match(textTurn.priorMedia[0].text, /第一张截图/);
  assert.match(textTurn.priorMedia[0].text, /不是本轮新附件/);
});

test("a location follow-up forces evidence only for the latest image turn, at the tail", async () => {
  const calls = [];
  const rebuild = mediaHistoryLoader(calls);
  await rebuild(memory, { model: "vision" }, "这是在哪里拍的", false);
  assert.deepEqual(calls, [{ name: "a.png", forced: false }, { name: "b.png", forced: true }]);
});

test("prior media lands on this turn's user message, never back in history", () => {
  const append = load("_appendContentParts");
  // 纯文本的回看（文字模型的转写）贴在字符串后面，形态不变——开场折叠和文字模型都读字符串。
  assert.equal(append("请求", [{ type: "text", text: "转写" }]), "请求\n\n转写");
  // 带图就升成多模态数组，当前附件在前、回看在后。
  assert.deepEqual(
    append("请求", [{ type: "text", text: "回看" }, { type: "image_url", image_url: { url: "data:x" } }]),
    [{ type: "text", text: "请求" }, { type: "text", text: "回看" }, { type: "image_url", image_url: { url: "data:x" } }],
  );
  assert.deepEqual(
    append([{ type: "text", text: "请求" }, { type: "image_url", image_url: { url: "data:now" } }], [{ type: "image_url", image_url: { url: "data:old" } }]),
    [{ type: "text", text: "请求" }, { type: "image_url", image_url: { url: "data:now" } }, { type: "image_url", image_url: { url: "data:old" } }],
  );
  assert.equal(append("请求", []), "请求");

  const send = grabCode("sendPrompt");
  assert.match(send, /const _history = await _memoryMessagesForModel\(sess\.memory, config, text, attachments\.length > 0\);/,
    "the current turn's intent still decides which prior media is visible");
  assert.match(send, /for \(const m of _history\.messages\) messages\.push\(m\);/);
  assert.match(send, /_appendContentParts\(await _attachmentAwareContent\(_userText, attachments, config, 7_000_000, false, text\), _history\.priorMedia\)/,
    "the selected prior media must be appended to THIS turn's user content");
  // 历史投影函数自己不再渲染任何附件：它连 _attachmentAwareContent 都不该碰。
  assert.doesNotMatch(grabCode("_memoryMessagesForModel"), /_attachmentAwareContent|image_url/);
});

test("the opener fold still reaches an opener that carries images", () => {
  // 旧图附在本轮消息末尾之后，纯文字轮的开场消息也可能是多模态数组；折叠只认字符串的话，
  // 这几轮的项目上下文（目录树 + 当前文件转储）在长 run 里就永远折不掉。
  const trim = load("_trimMessagesIfHuge", {
    _DEMAND_LEDGER_HEAD: loadConst("_DEMAND_LEDGER_HEAD"),
    // 折叠的保留段引用了账本标题常量（见 _DEMAND_LEDGER_HEAD）——从源码取真值注入，
    // 别手抄字符串：手抄过一次，标题改名后测试照绿而功能已经死了。
    _perfPhase: () => {},
    _gatewayHandlesCompression: () => false,
    _mcPrefixInvalidate: () => {},
    _msgSize: (m) => (Array.isArray(m?.content) ? m.content.map((p) => String(p?.text || p?.image_url?.url || "")).join("").length : String(m?.content || "").length),
    _estTokens: (msgs) => Math.round(msgs.reduce((n, m) => n + (Array.isArray(m?.content) ? m.content.map((p) => String(p?.text || "")).join("").length : String(m?.content || "").length), 0) / 4),
    _readEvidenceCovers: () => false,
    _REFETCHABLE: new Set(),
    _IMPORTANT_LINE: /error/i,
    _smartCompress: (c) => c.slice(0, 100),
    _foldAssistantText: (c) => c.slice(0, 100),
    _syncRunReadCoverageFromMessages: () => {},
  });
  const head = "--- 项目上下文 ---\n--- 目录树 ---\n" + "  file.ts\n".repeat(200) + "--- 当前文件 ---\n" + "const x = 1;\n".repeat(200);
  const opener = head + "━━━━━━━━━━━━\n📌 **This turn's user request**: \n\n还是不对";
  const calls = Array.from({ length: 4 }, (_, i) => ({ id: `c${i}`, type: "function", function: { name: "run_cmd", arguments: "{}" } }));
  const messages = [
    { role: "system", content: "sys" },
    { role: "user", content: [{ type: "text", text: opener }, { type: "image_url", image_url: { url: "data:prior" } }] },
    { role: "assistant", content: "", tool_calls: calls },
    { role: "tool", tool_call_id: "c0", content: "x".repeat(160_000) },
    ...calls.slice(1).map((c) => ({ role: "tool", tool_call_id: c.id, content: "ok" })),
  ];
  trim(messages, { root: "/repo" }, "/repo");
  const folded = messages[1].content;
  assert.ok(Array.isArray(folded), "the multimodal shape is preserved");
  assert.equal(folded[0].type, "text");
  assert.match(folded[0].text, /^\[较早的项目上下文/);
  assert.match(folded[0].text, /📌 \*\*This turn's user request\*\*: \n\n还是不对$/, "the real request survives the fold");
  assert.ok(!folded[0].text.includes("const x = 1;"), "the file dump is what the fold is for");
  assert.deepEqual(folded[1], { type: "image_url", image_url: { url: "data:prior" } }, "image parts are untouched");
});

// ---------------------------------------------------------------------------
// 2. 工具直觉表 + 能力名录
// ---------------------------------------------------------------------------
test("the tool hint is argument-free and byte-stable", () => {
  const build = load("_buildToolHint", { toolCapabilityIndex });
  const a = build();
  const b = build("跑起来了但看不到哪里错", { applies: true, bug: true });
  assert.equal(a, b, "anything the turn says must not change a byte of a prefix block");
  assert.ok(a.includes(toolCapabilityIndex()), "the full capability index rides with it");
  assert.match(a, /场景→工具直觉/);
  // 签名里不许再有会按轮求值的默认参数——那会在 system 装配点上被无参调用时跑一次分类器。
  assert.match(grabCode("_buildToolHint"), /^function _buildToolHint\(\)/);
});

test("the tool hint rides in the system prefix on both GATEWAY routes, never on a custom endpoint", () => {
  const send = grabCode("sendPrompt");
  // 非 L0：拼进开场 system 消息。fullPrompt 那一行的组成被别的测试钉着，所以追加在装配点。
  assert.match(send, /const _toolHint = \(effectiveMode === "agent"\) \? _buildToolHint\(\) : "";/);
  // 2026-08-27：这一行加了线路闸。工具直觉表 + 完整能力名录是内置 IP，**不出网关** ——
  // 走用户自己的中转时它会原样落在他的服务器日志里。缓存前缀这件事本身没变（仍然在
  // 开场 system 里、仍然一轮只 build 一次），变的是自定义端点那条路上它是空串。
  assert.match(send, /\{ role: "system", content: fullPrompt \+ \(_ipSafeRoute\(config\) \? _toolHint : ""\) \}/,
    "the hint must be part of the system message the whole run reuses（且不出网关）");
  // 当轮动态前导里不再有它：那是每轮的新后缀，放那儿永远不命中，折叠开场时还会整段消失。
  const preamble = /const _contextPreamble = ([^;]+);/.exec(send)?.[1] || "";
  assert.ok(preamble.length > 0);
  assert.doesNotMatch(preamble, /_toolHint|_buildToolHint/, "the hint must leave the per-turn preamble");
  assert.equal((send.match(/_buildToolHint\(/g) || []).length, 1, "one build per turn, at the system assembly point");

  // L0（默认线路）：_l0MessagesWithSkills 把原 system 整条丢掉、只用 clientBlocks 重建，
  // 没列进去就等于没发。
  const agent = grabCode("_agentModelTurn");
  const blocks = /const clientBlocks = ([^;]+);/.exec(agent)?.[1] || "";
  assert.match(blocks, /\(ideMode === "agent" \? _buildToolHint\(\) : ""\)/,
    "the L0 rebuild must carry the same hint, gated to agent mode like the non-L0 route");
  const l0Call = /_l0Msgs = _l0MessagesWithSkills\(providerMessages, skillsBlock, clientBlocks\)/;
  assert.match(agent, l0Call);
});
