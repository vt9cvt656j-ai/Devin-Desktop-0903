// 写前取证门原来做对了大半，只差最后一步。
//
// 它的判据是**执行事实**（取证台账为空），在第一次真写盘那一刻触发，并且当场把
// package_search / github_repo / developer_community_search / web_search 的 schema
// 装进本轮工具窗口——这些都对。唯一缺的是**把查询参数也替模型算好**。
//
// 少了那一步的后果不是"少个便利"：模型此刻正忙着写文件，手上突然多了几个工具、
// 却还要自己想查什么词。最省事的出口就是不查。这正是 _depDocsCandidate /
// _verifyCandidate 那套「执行事实 → 代码预填候选 → 模型点头」要解决的同一件事。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, load, loadConst, fnSource as topLevelFn } from "./helpers/source.mjs";

const query = load("_researchGateQuery", { _RESEARCH_QUERY_STOP: loadConst("_RESEARCH_QUERY_STOP") });
const fill = load("_researchGateCandidateFill");

// ── 查询串：宁可不预填，也不预填一个坏查询 ──────────────────────────────
test("优先用裁决声明的 researchTopics", () => {
  const q = query({ engineering: { researchTopics: ["Zustand state management", "React 19 concurrent"] } });
  assert.match(q, /Zustand/);
  assert.match(q, /React/);
});

test("没有 researchTopics 就回落到用户原话", () => {
  assert.match(query({ _originalText: "help me add Stripe checkout to the app" }), /Stripe/);
});

test("只保留英文技术词——中文 query 会命中错答案而且分数还高", () => {
  // 语料和社区源（Stack Overflow / HN / 各语言官方论坛）都是英文的。
  const q = query({ _originalText: "帮我把 Stripe 支付接进来，用 webhook 收回调" });
  assert.match(q, /Stripe/);
  assert.match(q, /webhook/);
  assert.doesNotMatch(q, /[一-龥]/, "中文进了查询串");
});

test("一个技术词都没有时返回空串——不预填好过预填一个坏查询", () => {
  // 模型点头之后拿到一堆无关结果，比让它自己想查什么更糟。
  assert.equal(query({ _originalText: "帮我改一下这里" }), "");
  assert.equal(query({}), "");
  assert.equal(query(null), "");
});

test("摘掉没信息量的词，但不许把技术名词一起摘走", () => {
  const q = query({ _originalText: "please help me use the new Prisma migrate for this" });
  assert.match(q, /Prisma/);
  assert.doesNotMatch(q, /\bplease\b|\bhelp\b|\bthe\b/, "噪音词没摘干净");
  // "go" 既是停用词也是语言名——摘过头会把真正要查的东西摘走。
  const stop = loadConst("_RESEARCH_QUERY_STOP");
  for (const lang of ["go", "rust", "c", "r", "swift", "dart"]) {
    assert.ok(!stop.has(lang), `停用词表把语言名 ${lang} 收进去了——查这门语言时查询串会被掏空`);
  }
});

test("查询串有长度上限，不会把整段用户原话灌进去", () => {
  const long = Array.from({ length: 200 }, (_, i) => `Token${i}`).join(" ");
  assert.ok(query({ _originalText: long }).split(" ").length <= 6);
});

test("同一个词不重复", () => {
  const q = query({ _originalText: "Stripe Stripe stripe STRIPE webhook" });
  assert.equal(q.toLowerCase().split(" ").filter((w) => w === "stripe").length, 1);
});

// ── 代填器：只在模型点头时代填，且一次性消费 ──────────────────────────
const cand = () => ({ name: "package_search", args: { query: "Stripe webhook" } });

test("模型对该工具发空参数调用 = 点头，代码替它补上", () => {
  const run = { _researchGateCandidate: cand() };
  const call = { type: "package_search", args: {} };
  assert.deepEqual(fill(run, call), { query: "Stripe webhook" });
  assert.deepEqual(call.args, { query: "Stripe webhook" });
});

test("模型自己带了参数就一个字不动", () => {
  const run = { _researchGateCandidate: cand() };
  const call = { type: "package_search", args: { query: "别的东西" } };
  assert.equal(fill(run, call), null);
  assert.deepEqual(call.args, { query: "别的东西" }, "把模型自己想查的东西覆盖掉了");
  assert.ok(run._researchGateCandidate, "模型带参数时候选不该被消费掉");
});

test("调别的工具不触发", () => {
  const run = { _researchGateCandidate: cand() };
  assert.equal(fill(run, { type: "write_file", args: {} }), null);
  assert.equal(fill(run, { type: "developer_community_search", args: {} }), null);
});

test("一次性消费：点过一次头就不再代填", () => {
  const run = { _researchGateCandidate: cand() };
  fill(run, { type: "package_search", args: {} });
  assert.equal(run._researchGateCandidate, null);
  const second = { type: "package_search", args: {} };
  assert.equal(fill(run, second), null);
  assert.deepEqual(second.args, {}, "第二次还在代填——那就不是「点头」是「代跑」了");
});

test("没有候选 / run 为空时安静返回", () => {
  assert.equal(fill({}, { type: "package_search", args: {} }), null);
  assert.equal(fill(null, { type: "package_search", args: {} }), null);
  assert.equal(fill({ _researchGateCandidate: cand() }, null), null);
});

// ── 接线：候选必须真的被武装、且在授权检查之前被消费 ────────────────────
test("取证门里真的武装了候选，不只是装了 schema", () => {
  assert.match(SRC, /run\._researchGateCandidate = _preWriteResearchMissing\.includes\("official"\)/,
    "那道门还是只装 schema 不预填参数——模型手上多了工具却还要自己想查什么");
  assert.match(SRC, /const _gateQuery = _researchGateQuery\(run\);/);
  assert.match(SRC, /if \(_gateQuery\) \{/, "空查询也去武装候选，会让模型点头点到一个空查询上");
});

test("缺官方那栏走注册表，缺社区那栏走社区检索", () => {
  // 两栏都缺时先补官方：版本/API 事实是社区经验的前提。
  // 锚点要用唯一的那一句：`run._researchGateCandidate =` 在文件里更早还出现过一次
  // （代填器里的 `= null`，一次性消费），从那儿切 400 字什么都切不到。
  const at = SRC.indexOf('run._researchGateCandidate = _preWriteResearchMissing');
  assert.ok(at > 0, "武装候选那一句不见了");
  const seg = SRC.slice(at, at + 400);
  assert.match(seg, /package_search/);
  assert.match(seg, /developer_community_search/);
});

test("点头入口接在唯一授权检查点之前", () => {
  const wrapper = topLevelFn("_executeToolStep", { code: true });
  const fillAt = wrapper.indexOf("_researchGateCandidateFill(run, call)");
  const approveAt = wrapper.indexOf("_approveToolCall(call, run)");
  assert.ok(fillAt > 0, "代填器没接进执行包装器——候选武装了也没人消费");
  assert.ok(approveAt > 0 && fillAt < approveAt,
    "代填要发生在授权检查之前，否则用户确认框里看到的是一条空查询");
});

test("红线：预填本体绝不自己发网络请求", () => {
  for (const name of ["_researchGateQuery", "_researchGateCandidateFill"]) {
    const body = topLevelFn(name, { code: true });
    assert.doesNotMatch(body, /fetch\(|_invokeCapped\(|invoke\(|XMLHttpRequest|WebSocket/,
      `${name} 里出现了网络调用——预填只许「等点头」，IDE 从不自己代跑`);
  }
});
