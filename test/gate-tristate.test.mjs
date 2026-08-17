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

// ── 提醒的淘汰顺序 ───────────────────────────────────────────────────────
//
// 同轮最多挂 2 条提醒（这个上限本身是对的：并排挂五条，模型会逐条表态，输出又长又自我横跳）。
// 但淘汰原来是"清最旧的那条"——而最旧和最不重要没有关系。一条 [BUILD_FAILED]、一条
// "你改了从没读过的文件"、一条子智能体带回来的 3200 字结果，都可能被一条"建议先调研"挤掉。
// 挤掉的是**事实**，留下的是建议：事实丢了模型就按错误图景继续干活，建议丢了只少一句提点。
test("提醒按重要性淘汰，不是按先来后到", () => {
  const factsSrc = /const _NUDGE_FACTS = new Set\(\[[\s\S]*?\]\);/.exec(SRC);
  const rankSrc = /const _nudgeRank = \(cat\) => [^;]+;/.exec(SRC);
  assert.ok(factsSrc && rankSrc, "分级表或 _nudgeRank 被改名/挪走了——淘汰会退回按先来后到");
  const rank = new Function(`${factsSrc[0]}\n${rankSrc[0]}\nreturn _nudgeRank;`)();

  assert.equal(rank("steer"), 0, "用户实时插话必须永远最高优先级");
  for (const fact of ["buildFix", "diag", "blindEdit", "subagentResult", "toolRepair", "recovery"]) {
    assert.equal(rank(fact), 1, `${fact} 是事实类，丢了模型会按错误图景干活`);
  }
  for (const advice of ["researchFirst", "planNudge", "midSummary", "stuck", "askBudget"]) {
    assert.equal(rank(advice), 2, `${advice} 是建议类，可以被事实挤掉`);
  }
  assert.equal(rank("someBrandNewNudge"), 2,
    "没登记的新提醒必须默认按建议类——要保命就得显式登记，不能靠默认捡到便宜");

  // 拿**源码里真实的那段淘汰循环**跑，不照抄一份：照抄的话我改了源码它照样绿。
  const loopSrc = /while \(_nudgeReg\.size >= 2\) \{[\s\S]*?\n    \}/.exec(SRC);
  assert.ok(loopSrc, "淘汰循环的形状变了，这条断言失去落点");
  assert.match(loopSrc[0], /_nudgeRank\(key\) > _nudgeRank\(worst\)/,
    "淘汰没有按 _nudgeRank 挑——又回到了按先来后到");

  const evict = new Function("_nudgeReg", "messages", "cat", "_nudgeRank", `${loopSrc[0]}\nreturn [..._nudgeReg.keys()];`);
  // 先进来一条事实、再一条建议，然后第三条要挤掉一个：该走的是建议，不是先来的那条事实。
  const reg = new Map([["buildFix", { c: "fact" }], ["researchFirst", { c: "advice" }]]);
  const msgs = [reg.get("buildFix"), reg.get("researchFirst")];
  const left = evict(reg, msgs, "diag", rank);
  assert.deepEqual(left, ["buildFix"], "被挤掉的应当是建议类，事实类要留下");
  assert.equal(msgs.length, 1, "被淘汰的那条也要从消息列表里摘掉，不能只从注册表删");

  // steer 永远不被挤。
  const reg2 = new Map([["steer", { c: "steer" }], ["researchFirst", { c: "advice" }]]);
  const msgs2 = [reg2.get("steer"), reg2.get("researchFirst")];
  assert.deepEqual(evict(reg2, msgs2, "diag", rank), ["steer"], "用户实时插话被挤掉了");
});

test("默认完整交付、先读懂再动手、每一步先想", () => {
  // 用户的原话：「随便写 MVP 结构糊弄用户」「动不动就把别人代码写烂」「必须先把全部项目读懂
  // ……才能去修改代码」「每写一个文件每做一步就需要去思考」。
  // 补之前全仓搜 MVP / 最小可用 / 糊弄，服务端提示词和客户端**零命中**——从来没有任何一条
  // 规则要求它别缩水。没有规则，模型缩到能跑就交，而且缩水本身不会被说出来。
  const frame = SRC.slice(SRC.indexOf("function _agentDecisionFrameBlock"));
  const laws = frame.slice(0, 8000);

  // ① 不降级。需求大就拆切片，不是砍功能。
  assert.match(laws, /交付规格律：默认按\*\*完整可用\*\*交付，不降级、不做演示版/,
    "交付规格律不见了——没有它，模型默认就是能跑就交");
  assert.match(laws, /需求过大就拆成\*\*有序的完整切片\*\*逐个交付/,
    "缺了「拆切片而不是砍功能」——否则「完整」遇到大需求就没法执行");
  for (const banned of ["TODO", "占位实现", "假数据"]) {
    assert.ok(laws.includes(banned), `禁止项少了「${banned}」——"完整"没有具体所指就会被解释掉`);
  }
  // ask_user 的用途被写死：问不懂的地方，不是拿来推卸交付标准。
  assert.match(laws, /ask_user 只在你真的读不懂用户要什么时才用/,
    "ask_user 的用途没写死，模型会拿它去问「要不要先做个简版」");
  // 只禁**肯定式**的那条路径。律文里那句「不要拿它去问『要不要先做个简版』」本身含这个词，
  // 按裸词去禁会把禁令自己判红（第一版就是这么误伤的）。
  assert.doesNotMatch(laws, /用 ask_user 把「完整实现」和「先做最小可用版」两条路/,
    "降级那条路又回来了——用户明确说过不需要降级");
  assert.match(laws, /不要拿它去问「要不要先做个简版」/,
    "要把这条路显式堵死，光不提它模型还是会自己想出来");

  // ② 先读懂再动手：依赖、入口、边界、真实成因，且要改的文件必须先读过。
  assert.match(laws, /先读懂再动手律/, "缺了「先读懂再动手」");
  assert.match(laws, /bug 的\*\*真实成因\*\*（不是症状）/, "要求找真实成因，不是照着症状改");
  assert.match(laws, /必须先 read_file 读过，不靠记忆和猜测下手/,
    "「要改的文件必须先读过」是「把别人代码写烂」的直接对策");

  // ③ 每一步先想 + 立刻验证。
  assert.match(laws, /逐步思考律/, "缺了「每一步先想」");
  assert.match(laws, /写完立刻用真实结果（诊断\/命令退出码\/真实输出）验证再走下一步/,
    "想完还要立刻验证，否则「思考」落不到地");

  // 已有代码的写法要沿用——用户抱怨的另一半。
  assert.match(laws, /沿用现存的分层、命名、错误处理和测试方式/, "缺了「沿用现有约定」");
});

// ── 项目本地记忆落到文件 ───────────────────────────────────────────────────
//
// 用户："能创建 Mr. Day One 项目本地记忆……可以实时扶正项目内容、锁定项目目标不偏航、
// 也能把遇到的错误放进去保持自己不会再犯。"
//
// 补之前：记忆只活在 localStorage（key = michael-ide.kg:<root>）——**不在项目里**。用户看不见、
// 改不动、换台机器就没了、清一次应用数据就丢，而里面存的恰恰是最不该丢的那些东西。
test("项目记忆要落到项目里的文件，而不是只活在浏览器存储里", () => {
  assert.match(SRC, /const _PROJECT_MEMORY_REL = "\.michael\/memory\.md";/,
    "项目记忆没有对应的文件——清一次应用数据就全丢了");

  // 分节沿用已有的 _kgClassify，不新造一套分类：两套分类必然漂。
  const sections = /const _PM_SECTIONS = \[([\s\S]*?)\];/.exec(SRC);
  assert.ok(sections, "分节表不见了");
  for (const must of ["锁定，防偏航", "实时扶正", "不再犯"]) {
    assert.ok(sections[1].includes(must), `分节少了「${must}」——那是用户点名要的三件事之一`);
  }
  assert.match(sections[1], /"pitfall"/, "「踩过的坑」要接到 _kgClassify 已有的 pitfall 分类上");

  // 渲染：只写活的笔记，被作废的不能再出现在文件里（否则手改文件的人会照着过期结论走）。
  const md = /function _projectMemoryMarkdown\(root\)[\s\S]*?\n\}/.exec(SRC);
  assert.ok(md, "渲染函数不见了");
  assert.match(md[0], /superseded\.has\(note\.id\)/, "被纠错作废的笔记不该写进文件");

  // 写入：记完就落盘，但不 await——落盘失败不该让「已记住」变成失败。
  assert.match(SRC, /if \(ok && !isGlobal\) void _mirrorProjectMemoryFile\(root\);/,
    "项目记忆写入后没有落盘");

  // 读回：只在本地存储确实为空时导入，否则一份旧文件会把刚记的东西抹掉。
  const imp = /async function _importProjectMemoryFile\(root\)[\s\S]*?\n\}/.exec(SRC);
  assert.ok(imp, "读回函数不见了");
  assert.match(imp[0], /if \(_kgLoad\(root\)\.length\) return 0;/,
    "必须只在本地记忆为空时导入——否则旧文件会静默回退掉新记的内容，那是最难查的一类问题");
  assert.match(imp[0], /if \(!line\.startsWith\("- "\)\) continue;/,
    "只认列表项：标题和注释不是记忆");
  assert.match(SRC, /void _importProjectMemoryFile\(path\);/, "打开项目时没有读回");
});
