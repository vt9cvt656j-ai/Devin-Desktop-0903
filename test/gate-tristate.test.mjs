// 闸门的第三态：「裁决还没到」不等于「裁决说了不」。
//
// 2026-08-17 实测：完整意图裁决要 19.8 秒（生产网关 upstream_header_ms=19836），而第一个模型
// 回合往往在它之前就结束了。也就是说 run.engineering 为空是**常态**，不是边缘情况。
//
// 那天最贵的一个 bug 就长在这里：初始工具编排的闸门写的是 `!run.engineering?.applies`，
// 画像为空时为真 → 整轮工具编排不启动 → 128 个工具一个都进不来（没有 web_search、
// knowledge_search、git、db_query、browser）。用户的原话是「我让他干什么他什么都不知道」。
//
// 全量对账之后的结论值得写下来：**剩下的否决位全都倒向「少一道仪式」，而不是「少一样能力」**
// ——不要求计划、不注入验收契约、不算复杂任务。那个方向是对的，不该改。这个文件的作用不是
// 把它们全翻过来，而是把这一类**变成显式清单**：新长出来的否决位、或者改了形状的旧位，
// 都必须先在下面写清它往哪个方向倒，才能通过。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

// 按**代码文本**登记，不按行号：这个文件几万行，行号每天都在漂，按行号钉的清单第二天就是
// 一堆假红。改了那一行的文本 = 改了那处判断，本来就该重新过一遍。
//
// direction 只有两个值：
//   "ceremony" —— 裁决缺席时少做一道仪式（不要求计划、不注入契约……）。能力不受影响，方向正确。
//   "capability" —— 裁决缺席时**夺走一样能力**。这是那天那个 bug 的形状，一律要 intentSource 守卫。
const REVIEWED = new Map([
  ["const readOnly = !!run.engineering?.explicitReadOnly;",
    { direction: "ceremony", why: "缺席 → 不视为只读 → 允许更多动作，倒向放行" }],
  ["const complexReadOnly = !!run.engineering?.projectScope || !!run.engineering?.longTask;",
    { direction: "ceremony", why: "缺席 → 不算复杂只读任务 → 少一道计划仪式，不影响能力" }],
  ["return !!run.engineering?.requiresPlan;",
    { direction: "ceremony", why: "缺席 → 不强制先出计划 → 少一道仪式，倒向放行" }],
  ['return run.engineering?.applies ? "mutate" : "answer";',
    { direction: "ceremony", why: "缺席 → 按「答疑」判，收尾不强求交付物证据；宽松方向，不夺能力" }],
  ["const _quick = () => task.trim().length < 80 && !run.engineering?.applies && !_mustUseWorkspaceToolsNow();",
    { direction: "ceremony", why: "缺席 + 短消息 → 走轻量路径，不注入验收契约；只影响仪式" }],
  ["if (!run.engineering?.designKnowledgeRequired || !preflight.required) return false;",
    { direction: "ceremony", why: "这是**消费**预取结果的一侧；预取本身由正向闸门启动，缺席时压根没启动，这里返回 false 是自洽的" }],
  ["run._steeredWorkspaceRequired = !!run.engineering.explicitWorkspaceMutation;",
    { direction: "ceremony", why: "缺席 → 不额外声明写入义务；赋值而非否决" }],
]);

function denialSites(src) {
  const out = [];
  src.split("\n").forEach((line, index) => {
    const s = line.trim();
    if (!s || s.startsWith("//") || s.startsWith("*")) return;
    if (!/run\.engineering\??\.[a-zA-Z]/.test(s)) return;
    // 否决形状：对画像取非，或者拿画像字段做三元降级。
    const denies = /!\s*run\.engineering/.test(s) || /run\.engineering\??\.\w+\s*\?/.test(s);
    if (!denies) return;
    out.push({ line: index + 1, text: s.replace(/\s+/g, " "), guarded: /intentSource|_verdictLanded/.test(s) });
  });
  return out;
}

test("每一处「按画像否决」都要写明：裁决缺席时它倒向哪边", () => {
  const sites = denialSites(SRC);
  assert.ok(sites.length >= 5,
    `只扫出 ${sites.length} 处否决位——正则失效了，这条断言等于没跑`);

  const unreviewed = sites.filter((site) => !site.guarded && !REVIEWED.has(site.text));
  assert.deepEqual(unreviewed.map((s) => `${s.line}: ${s.text}`), [],
    "这些「按画像否决」的判断既没有 intentSource 守卫，也没登记方向。\n"
    + "完整裁决实测 19.8 秒，画像为空是常态——先想清楚它缺席时你希望倒向哪边：\n"
    + "  · 少一道仪式（不要求计划、不注入契约）→ 登记进 REVIEWED，direction: \"ceremony\"\n"
    + "  · 夺走一样能力（工具、检索、知识）→ 必须加 intentSource 守卫，不许登记");

  // 清单不能变成谎言：登记了却已经不在代码里的条目要删掉，否则下一个人以为它还在守着。
  const present = new Set(sites.map((s) => s.text));
  for (const [text, meta] of REVIEWED) {
    assert.ok(present.has(text), `REVIEWED 里这条已经不在代码里了，删掉它：\n  ${text}`);
    assert.equal(meta.direction, "ceremony",
      `direction 只允许 "ceremony"。倒向 "capability" 的判断不该靠登记放行，必须加 intentSource 守卫：\n  ${text}`);
    assert.ok(meta.why && meta.why.length >= 12, `这条要写清为什么这个方向是安全的：\n  ${text}`);
  }
});

test("工具编排的闸门必须区分「裁决未到」和「裁决说不适用」", () => {
  // 这是那天那个 bug 的原位，单独钉一条：它是**唯一**一处倒向 capability 的否决，
  // 也是「我让他干什么他什么都不知道」的直接成因。回退它不该只让上面那条泛化断言变红。
  const loop = SRC.slice(SRC.indexOf("const _startInitialToolRoutingAfterFirstTurn"));
  const gate = loop.slice(0, 1400);
  assert.match(gate, /const _verdictLanded = run\.engineering\?\.intentSource === "ai";/,
    "工具编排闸门不再区分「裁决未到」——画像为空时整轮 128 个工具都进不来");
  assert.match(gate, /\(_verdictLanded && !run\.engineering\.applies\)/,
    "只有**裁决真的到了且说不适用**才拦；「还没到」必须放行");
  assert.doesNotMatch(gate, /\|\|\s*!run\.engineering\?\.applies\s*\|\|/,
    "旧的无条件否决又长回来了");
});
