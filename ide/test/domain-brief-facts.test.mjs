// 域小抄的抬头对模型说：「它是该领域的**既有事实**，不是灵感：先按它判断可行性、
// 约束和验收」。而它实际给出的，六成是**小节标题的回声**。
//
// 实测（2026-08-23，真语料 683 个非设计域小节 + 真函数）：
//   416 段（60.9%）的首条要点 == 它自己的小节标题
// 成因：网关索引器把小节标题写回了 chunk 正文第一行，而这里先剥 `#` 再按长度过滤——
// 「Database Selection Decision Tree」这种 32 字符的标题顺利活过 `< 24` 那道门。
//
// 被这样吞掉的，正是最该用上的那几条：服务拆分规则、数据库选型决策树、各语言默认 ORM。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load } from "./helpers/source.mjs";

const bullets = load("_domainKnowledgeBullets");
const chunk = (section, body) => `【1·经验｜database/x · ${section}】\n## ${section}\n${body}`;

test("首条要点不再是小节标题的回声", () => {
  const out = bullets(chunk("Database Selection Decision Tree",
    "Follow top-down. Stop at the first match. 关系型优先，除非有明确反例，这一句足够长。"), 1, 190);
  assert.equal(out.length, 1);
  const text = out[0].split(" → ")[1];
  assert.notEqual(text, "Database Selection Decision Tree",
    "给的还是标题——那份「既有事实」小抄其实是一张标题清单");
  assert.match(text, /Follow top-down/);
});

test("判据按结构判，不拿正文和小节名做前缀比较", () => {
  // 前缀比较会把「小节名 Rate Limiting + 首句 Rate Limiting must be applied per-tenant」
  // 这种**真事实**一起误杀。
  const out = bullets(chunk("Rate Limiting",
    "Rate Limiting must be applied per-tenant, not per-IP: shared NAT makes per-IP useless."), 1, 190);
  assert.equal(out.length, 1, "以小节名开头的真事实被误杀了");
  assert.match(out[0], /per-tenant/);
});

test("各级标题都跳，不只是 H2", () => {
  // 标题行本身必须写够长度门（24 字），否则是长度门挡掉它、标题判据根本没被量到——
  // 那样这条断言就是绿的摆设（2026-08-23 变异实测：把判据收成只认 H2 它照样全绿）。
  for (const h of ["#", "##", "###", "####"]) {
    const out = bullets(chunk("X", `${h} 这是一个写够了二十四个字符的标题行，必须靠标题判据挡掉而不是靠长度门\n真正的事实句子放在这里，必须写够二十四个字符才会被选中，这就是那道长度门。`), 1, 190);
    assert.match(out[0], /真正的事实句子/, `${h} 级标题没被跳过`);
  }
});

// ── 表格：语料里大量事实本身就是表 ────────────────────────────────────
test("表格数据行是事实，要留下", () => {
  const out = bullets(chunk("Anti-Pattern Checklist",
    "| Category | Question |\n|---|---|\n| Wrong Cuts | Each service owns a single bounded context? |"), 1, 190);
  assert.equal(out.length, 1, "整类表格被砍掉了——这份语料里反模式清单、选型对照表都是表");
  assert.match(out[0], /Wrong Cuts/);
});

test("表头行和分隔行不算事实", () => {
  const out = bullets(chunk("X",
    "| Need | Structure | Example |\n|---|---|---|\n| 简单缓存 | String | 一条足够长的真实说明放在这里，凑够二十四个字符 |"), 1, 190);
  assert.doesNotMatch(out[0], /Need \| Structure/, "表头被当成了事实");
  assert.match(out[0], /简单缓存/);
});

// ── 代码围栏：兜底而不是首选 ──────────────────────────────────────────
test("有散文时不选代码", () => {
  const out = bullets(chunk("X",
    "```js\nimport { drizzle } from \"drizzle-orm/node-postgres\";\n```\n这一节真正想说的事实句子放在这里，同样要写够二十四个字符才过得了那道长度门。"), 1, 190);
  assert.match(out[0], /真正想说的事实/, "一条孤立的 import 行压过了散文事实");
});

test("整节只有代码时，给代码好过给空", () => {
  // 一刀砍掉整类围栏会让产不出要点的小节从 13.6% 涨到 49.2%（实测）——
  // 那是把一半语料静默扔掉。
  const out = bullets(chunk("X",
    "```js\nconst db = drizzle(pool, { schema }); // 这一行写够了二十四个字符，是这节唯一的内容\n```"), 1, 190);
  assert.equal(out.length, 1, "整节只有代码就什么都不给——半份语料会这样静默消失");
  assert.match(out[0], /drizzle/);
});

test("围栏配对：结束标记之后的行不再算围栏内", () => {
  const out = bullets(chunk("X",
    "```\n围栏里这一行虽然写够了二十四个字符，但它在围栏里，只能当兜底不能当首选\n```\n围栏外这一行才是这节的事实句子，同样写够二十四个字符，用来验证围栏状态有没有配对。"), 1, 190);
  assert.match(out[0], /围栏外这一行/, "围栏状态没配对，后面的散文被当成了代码");
});

// ── 边界 ──────────────────────────────────────────────────────────────
test("太短的行仍然跳过（空行、粘合词）", () => {
  const out = bullets(chunk("X", "示例：\n短\n这一条才是足够长的真实事实句子，写够二十四个字符之后才会被选中，前面两行都太短。"), 1, 190);
  assert.match(out[0], /才是足够长/);
});

test("一段都产不出时返回空，不编", () => {
  assert.deepEqual(bullets(chunk("X", "## 只有标题\n短"), 1, 190), []);
  assert.deepEqual(bullets("", 1, 190), []);
});

test("条数与长度上限都还在", () => {
  const many = Array.from({ length: 8 }, (_, i) =>
    `【${i}·经验｜d/x · S${i}】\n## S${i}\n这是第 ${i} 条足够长的真实事实句子，写够二十四个字符用于验证条数上限。`).join("\n\n———\n\n");
  assert.equal(bullets(many, 3, 190).length, 3);
  const long = bullets(chunk("X", "事" .repeat(500)), 1, 50)[0];
  assert.ok(long.length <= 50 + "X → ".length + 1, `没有按 maxChars 截断：${long.length}`);
});
