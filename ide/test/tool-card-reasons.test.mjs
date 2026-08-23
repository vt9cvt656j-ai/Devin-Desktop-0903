// 工具卡上写给用户的**理由**必须对得上事实。
//
// 2026-08-23 用户现场（他自己的 AICode 项目，一个 agent 回合）：
//   · 编辑 README.md          → 红徽章「先查一下再选」
//   · 运行 chmod +x …（删除线）→ 红徽章「前项真实失败 · 后续已停止」
//   · 知识检索 ×3             → 红徽章「无可用命中」
// 三张卡的理由全是错的：
//   ① 拦住那次编辑的是**计划门**（[BLOCKED_PLAN_FIRST]，要求先写计划），不是调研门。
//      两道门共用一句文案和同一个失败码，于是用户被告知去做一件他根本没被要求做的事。
//   ② 「前项真实失败」——前项是被门在运行前拦下的，一个字节都没写，没有任何失败输出可读。
//   ③ 红色只可能来自 `[失败]` 开头的正文，也就是说那三次是**真失败**（HTTP/异常），
//      却被写成「无可用命中」；模型据此会把「没拿到」当成「库里确实没有」写进交付说明。
//
// 拦截本身**不改**：门是「一个 run 只拦一次」，放行后续等于让那道要求彻底落空
// （logic.test.mjs 正面钉着前项 [BLOCKED] 时后续必须停）。这里改的只有「理由」。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, load, fnSource } from "./helpers/source.mjs";

const succeeded = load("_toolExecutionSucceeded", {
  _WORKSPACE_MUTATING_TYPES: new Set(["write", "edit", "multiedit"]),
  _toolFailureMatch: load("_toolFailureMatch"),
  _toolFailureMarkerAtHead: load("_toolFailureMarkerAtHead"),
});

// ── ① 两道门必须各说各的 ──────────────────────────────────────────────
const dispatch = fnSource("_runAgenticLoop", { code: true }) || SRC;

test("计划门和调研门给的是两句不同的话", () => {
  assert.match(SRC, /const _planFirst = techIssue\.startsWith\("\[BLOCKED_PLAN_FIRST\]"\)/,
    "两道门又合流了——用户会被告知去做一件他没被要求做的事");
  assert.match(SRC, /_planFirst \? "先写计划再动手" : "先查一下再选"/,
    "徽章文案没有按门分开");
  assert.match(SRC, /code: _planFirst \? "plan_first" : "tech_research"/,
    "失败码没有按门分开——恢复指令会走错分支");
});

test("拦截本身一个都没被放行（改文案不许顺带把门改松）", () => {
  // 这条是反向断言。只钉「文案分开了」是绿的摆设：把整段拦截删掉它照样绿。
  assert.match(SRC, /const techIssue = _planBeforeBuildIssue\(run, it\.call\) \|\| _newTechResearchIssue\(run, it\.call\)/,
    "门的调用点没了");
  assert.match(SRC, /if \(_planFirst\) run\._planStopUsed = true;\s*\n\s*else run\._techResearchStopUsed = true;/,
    "「一个 run 只拦一次」的记号没置——门会反复拦同一轮");
  assert.match(SRC, /it\.rawResult = blocked;/, "拦截结果没有落到 item 上");
});

// ── ② 「真实失败」要对得上事实 ────────────────────────────────────────
const isPreBlock = load("_isPreExecutionBlock", {
  _PRE_EXECUTION_BLOCK_CODES: new Set([
    "plan_first", "tech_research", "read_before_edit", "mutation_batch", "command_batch",
  ]),
});

test("预执行拦截这一族认得全，且不误收跑过之后才有的结局", () => {
  for (const code of ["plan_first", "tech_research", "read_before_edit", "mutation_batch", "command_batch"]) {
    assert.equal(isPreBlock({ failure: { code } }), true, `${code} 应算门拦`);
  }
  // 这两个是**跑过了**才有的结局：请求发出去了、监控起来了。把它们算成「没跑过」，
  // 卡片就会对一次真实的重定向/不可核查说「前项被门拦下」。
  for (const code of ["http_redirect", "monitor_uncheckable"]) {
    assert.equal(isPreBlock({ failure: { code } }), false, `${code} 不是预执行拦截`);
  }
  assert.equal(isPreBlock({ ok: false, content: "[BLOCKED]" }), false,
    "没有结构化失败码时不许靠猜——那会把真失败也说成门拦");
  assert.equal(isPreBlock(null), false);
  assert.equal(isPreBlock({}), false);
});

test("预执行拦截这一族的成员是钉死的", () => {
  // 上面那条行为断言吃的是**注入进去的**集合，改真集合它不会红（变异实测：往真集合里
  // 加 http_redirect，11 条测试全绿）。成员表必须对着源码本身钉。
  const m = SRC.match(/const _PRE_EXECUTION_BLOCK_CODES = new Set\(\[([\s\S]{0,400}?)\]\)/);
  assert.ok(m, "_PRE_EXECUTION_BLOCK_CODES 不见了");
  const codes = [...m[1].matchAll(/"([a-z_]+)"/g)].map((x) => x[1]).sort();
  assert.deepEqual(codes,
    ["command_batch", "mutation_batch", "plan_first", "read_before_edit", "tech_research"],
    "成员变了。每一个进来的都必须满足「门在运行前拦下、磁盘一个字节都没写」——"
    + "http_redirect / monitor_uncheckable 是**跑过了**才有的结局，收进来就会把一次真实结局"
    + "说成「前项被门拦下」，用户又一次读到错的理由");
});

const blockedEarlier = load("_preExecutionBlockedEarlier", { _isPreExecutionBlock: isPreBlock });

test("同批更早的门拦认得出来", () => {
  const items = [
    { rawResult: { ok: false, failure: { code: "plan_first" } } },
    { call: { type: "cmd" } },
  ];
  assert.equal(blockedEarlier(items, 1), true);
  assert.equal(blockedEarlier([{ rawResult: { ok: false, code: 1, content: "npm ERR!" } }, {}], 1), false,
    "真跑过并失败的前项不该被说成门拦");
  assert.equal(blockedEarlier(items, 0), false, "第一项前面没有任何项");
  assert.equal(blockedEarlier(null, 1), false);
});

test("批量停止的徽章按前项性质分成两句", () => {
  assert.match(SRC, /_preExecutionBlockedEarlier\(items, index\) \? "前项被门拦下 · 后续已停止" : "前项真实失败 · 后续已停止"/,
    "又变回一句「真实失败」了——用户会去找一个并不存在的失败输出");
});

// ── ③ 知识检索：零命中 ≠ 检索失败 ─────────────────────────────────────
const label = load("_knowledgeSettleLabel", { _toolExecutionSucceeded: succeeded });
const CALL = { type: "knowledge", domain: "michael-design", query: "卡片布局" };

test("有命中时照常报条数", () => {
  assert.equal(label(CALL, { knowledge: { hitCount: 3 }, content: "…" }, "3 段 · 已注入"), "3 段 · 已注入");
});

test("真零命中说「无可用命中」——那是一个结论", () => {
  const zero = { type: "knowledge", knowledge: { hitCount: 0, domains: [] },
    content: "知识库的「michael-design」这一个域里没有「卡片布局」相关的内容。确认该主题确实不可用后，基于用户明确要求与项目证据继续。" };
  assert.equal(label(CALL, zero, ""), "无可用命中");
  // 顺带钉住：这段正文里有「不可用」三个字，而失败判据的词表里也有它。
  // 判据要求它出现在**方括号内**，所以这段正文不该被判成失败——一旦哪天判据放宽成裸词，
  // 每一次正常的零命中都会被涂红。
  assert.equal(succeeded(CALL, zero), true, "零命中被判成失败了——正常检索会整片变红");
});

test("检索失败必须说失败，并带上真实原因", () => {
  for (const [content, want] of [
    ["[失败] 知识库查询 HTTP 429: rate limited", /^检索失败 · HTTP 429/],
    ["[失败] 知识库查询 HTTP 401: unauthorized", /^检索失败 · HTTP 401/],
    ["[失败] michael-design 预取异常: fetch failed", /^检索失败 · michael-design 预取异常/],
  ]) {
    const got = label(CALL, { type: "knowledge", content }, "");
    assert.match(got, want, `失败被写成了别的：${got}`);
    assert.doesNotMatch(got, /无可用命中/,
      "一次真实失败仍被写成「没查到内容」——模型会把「没拿到」当成「库里确实没有」");
  }
});

test("两个检索落定点都走同一个判据（漏一个就还有半边是错的）", () => {
  // 只数**调用点**：`_knowledgeSettleLabel(call, result,` 也会匹配到函数自己的定义行
  // （这条断言第一次写就被自己的定义喂到 3）。
  const hits = (SRC.match(/_settleToolStep\(step, result, _knowledgeSettleLabel\(/g) || []).length;
  assert.equal(hits, 2,
    `只有 ${hits} 处走了新判据，应为 2（michael-design 预检 + 领域知识预检）`);
  assert.doesNotMatch(SRC, /_settleToolStep\(step, result, evidence \? [^\n]*: "无可用命中"\)/,
    "michael-design 那处还在直接写死「无可用命中」");
});

// ── ④ 简报里对模型说的话 ─────────────────────────────────────────────
test("检索失败时，简报不许对模型说「知识库没有」", () => {
  const brief = fnSource("_michaelDesignBrief", { code: true });
  assert.match(brief, /const _failedTracks = \(Array\.isArray\(results\) \? results : \[\]\)\.filter\(\(item\) => item\?\.failed\)/,
    "简报没有区分「失败」和「零命中」");
  assert.match(brief, /\*\*这不等于知识库里没有内容\*\*/,
    "失败时给模型的话里没有把这条说穿——它会把「没拿到」写成「库里没有」");
  assert.match(brief, /\$\{excerpts \|\| _noHitNote\}/, "兜底句没有接上");
});

test("「这次是失败还是零命中」用结构字段判，不用文案匹配", () => {
  const pre = fnSource("_runMichaelDesignPreflight", { code: true });
  assert.match(pre, /failed: !result\?\.knowledge/,
    "判据不是结构字段了——文案一改，这条链就静默失效");
  // 反向：确认这个结构差别真的存在于生产方
  const search = fnSource("_searchKnowledgeBase", { code: true });
  assert.match(search, /knowledge: \{ hitCount: 0, domains: \[\] \}/, "零命中分支不再带 knowledge 字段");
  assert.match(search, /content: `\[失败\] 知识库查询 HTTP \$\{response\.status\}/, "HTTP 失败分支变了");
});
