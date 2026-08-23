// 域小抄的四条 rubric 各查一次（4 × topK 4 = 16 个检索位），而它们之间**没有共享的
// 去重集合**。多数域的小节总数比检索位还少——实测 21 个非设计域共 683 小节，其中
// systems-programming 9、healthcare 13、blockchain 14、ecommerce 15 都少于 16 个位，
// 抽屉原理保证重复；其余域也会在同一小节对多条 rubric 都高分时重复。
//
// 后果是 2500 字符的小抄预算有相当一部分花在逐字重复上，挤掉的是别的上下文。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, load, fnSource as topLevelFn } from "./helpers/source.mjs";

const bullets = load("_domainKnowledgeBullets");
const chunk = (sec, body) => `【1·经验｜d/x · ${sec}】\n## ${sec}\n${body}`;
const L = (t) => `${t}——这一句写够了二十四个字符，能通过那道长度门被选中。`;

test("共享 seen 时，第二次取同一段的**下一条**，不是空", () => {
  const doc = chunk("S", `${L("第一条")}\n${L("第二条")}`);
  const seen = new Set();
  const a = bullets(doc, 1, 190, seen);
  const b = bullets(doc, 1, 190, seen);
  assert.equal(a.length, 1);
  assert.equal(b.length, 1, "第二次整栏空了——只过滤不重取，比重复更糟");
  assert.notEqual(a[0], b[0], "两次拿到同一条，去重没生效");
  assert.match(b[0], /第二条/);
});

test("不传 seen 时行为一个字节没变", () => {
  const doc = chunk("S", `${L("第一条")}\n${L("第二条")}`);
  assert.deepEqual(bullets(doc, 1, 190), bullets(doc, 1, 190, null));
});

test("同一段被取空之后，如实返回空而不是硬凑", () => {
  const doc = chunk("S", L("唯一一条"));
  const seen = new Set();
  assert.equal(bullets(doc, 1, 190, seen).length, 1);
  assert.deepEqual(bullets(doc, 1, 190, seen), [], "取空了还在给东西——那只能是编的");
});

test("去重判据比的是要点正文，不是带小节名的整行", () => {
  // 同一句话挂在不同 rubric 标题下仍然是同一句话；比整行会让它看起来不一样。
  const seen = new Set();
  bullets(chunk("甲节", L("同一句")), 1, 190, seen);
  const second = bullets(chunk("乙节", L("同一句")), 1, 190, seen);
  assert.deepEqual(second, [], "换个小节名就当成新内容了——重复照旧");
});

// ── 接线：去重必须在并发之后、按固定顺序 ──────────────────────────────
const preflight = topLevelFn("_runDomainKnowledgePreflight", { code: true });

test("去重发生在 Promise.all 之后，不是塞进并发分支", () => {
  // 共享集合塞进并发分支的话，「谁占到某小节」取决于网络返回顺序——
  // 同一次查询跑两遍结果不同。
  const allAt = preflight.indexOf("const sections = await Promise.all");
  const dedupAt = preflight.indexOf("const _seenBullet = new Set();");
  assert.ok(allAt > 0 && dedupAt > 0, "两个锚点都要在");
  assert.ok(dedupAt > allAt, "去重集合建在并发之前——结果会随网络返回顺序变");
  const concurrent = preflight.slice(allAt, dedupAt);
  assert.doesNotMatch(concurrent, /_seenBullet/, "并发分支里引用了共享集合");
});

test("去重是**重算**不是过滤——四条 rubric 撞同一小节时后面的栏仍有内容", () => {
  // 这条量的是行为差异：只过滤的话，撞车的栏会整栏变空；重算会取到同段的下一条。
  const doc = chunk("S", `${L("第一条")}\n${L("第二条")}\n${L("第三条")}`);
  const seen = new Set();
  const cols = [bullets(doc, 1, 190, seen), bullets(doc, 1, 190, seen), bullets(doc, 1, 190, seen)];
  assert.deepEqual(cols.map((c) => c.length), [1, 1, 1],
    "四条 rubric 命中同一小节时，后面的栏空了——那是只过滤没重取");
  assert.equal(new Set(cols.map((c) => c[0])).size, 3, "三栏拿到了同一条");
});

test("并发分支把原文留下来，供之后重算", () => {
  assert.match(preflight, /raw: _raw/, "原文没留——之后只能过滤，不能重取下一条");
  assert.match(preflight, /_domainKnowledgeBullets\(sec\.raw, 3, 190, _seenBullet\)/,
    "没有用原文重算，退回成了只过滤");
  assert.match(preflight, /delete sec\.raw;/, "原文没清掉，会跟着 brief 一路带下去");
});

// ── 四栏结构 ──────────────────────────────────────────────────────────
const brief = load("_domainKnowledgeBrief", { _DOMAIN_KNOWLEDGE_BRIEF_BUDGET: 2500 });
const secs = (counts) => ["适用条件", "硬性约束", "常见坑", "必须做的检查"]
  .map((h, i) => ({ heading: h, bullets: Array.from({ length: counts[i] }, (_, k) => `S → 第 ${i}-${k} 条`) }));

test("brief 渲染的是**全部四栏**，不是有内容的那几栏", () => {
  // 源码层面钉住：`sections.filter(s => s.bullets.length)` 这种写法会让空栏整个消失。
  const src = topLevelFn("_domainKnowledgeBrief", { code: true });
  assert.match(src, /const body = sections\s*\n?\s*\.map/,
    "brief 又只渲染有内容的栏了——空栏消失会让模型以为那个维度没被问过");
  assert.doesNotMatch(src.slice(src.indexOf("const body =")), /filter\(\(s\) => s\.bullets\.length\)/,
    "渲染前又把空栏过滤掉了");
});

test("空栏如实说明，不整栏消失", () => {
  // 消失会让模型以为那个维度压根没被问过，而事实是「问了，这个域没有独立的答案」。
  const out = brief("healthcare", secs([2, 0, 0, 0]));
  for (const h of ["适用条件", "硬性约束", "常见坑", "必须做的检查"]) {
    assert.ok(out.includes(`【${h}】`), `「${h}」这一栏整个消失了`);
  }
  assert.match(out, /这一栏没有独立内容/, "空栏没说明为什么空");
  assert.match(out, /别当成"没有要求"/, "空栏没挡住「没列出来就是不用管」这种误读");
});

test("四栏全空时仍然走「整域不可用」那条兜底", () => {
  const out = brief("healthcare", secs([0, 0, 0, 0]));
  assert.match(out, /没有返回任何可用命中/, "全空时应当是整域不可用的说法，不是四个空栏");
});

test("有内容的栏一个字节没变", () => {
  const out = brief("healthcare", secs([2, 1, 1, 1]));
  assert.match(out, /- S → 第 0-0 条/);
  assert.doesNotMatch(out, /这一栏没有独立内容/, "有内容的栏被当成空的了");
});
