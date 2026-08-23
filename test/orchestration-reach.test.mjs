// 编排族在真实用量里是**零调用**：用户机器 939 个回合的情景档案里，run_subagent /
// run_worker / spawn_multiple_agents 合计 0 次（2026-08-22 实测）。
//
// 不是模型不想派角色，是三道结构性的门叠在一起，任何一道单独就足以让它永远轮不到：
//   一、orchestrationMode 这个枚举在两条裁决链路里都只有枚举值、没有判据。同一段提示词
//      里 researchMode 有 250 字判据、domain 有判据。没判据的枚举恒等于默认值。
//   二、四个编排工具全在开局窗口外，要先花一轮 search_tools——而"自己往下读"永远更便宜。
//   三、run_subagent 的描述以 "Use only in structured-collaboration mode" 开头，而那是
//      内部画像旗标，模型观察不到自己在不在这个模式里。无法自查的前置条件读起来就是"别用"。
//
// 这个文件逐条守这三道门都还开着。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, fnSource as topLevelFn } from "./helpers/source.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));

// ── 一、枚举必须带判据 ──────────────────────────────────────────────────
//
// 判据的形状不是"多写点字"，而是**给出可判定的分界**：什么情况填 A、什么情况填 B，
// 以及反方向的代价（否则模型会为了显得强大而一律往上填）。
// 只取「枚举名之后、下一个字段之前」那一段——判据必须长在这个槽里。
// 从关键字本身开始切是错的：那样 staged_roles / parallel_roles 会被枚举名自己喂绿，
// 把整条判据删成一行枚举值，断言照样过（2026-08-22 变异实测确实如此）。
function criterionSlot(text, key, until) {
  const i = text.indexOf(key);
  assert.ok(i > 0, `提示词里找不到 ${key}`);
  const from = i + key.length;
  const end = text.indexOf(until, from);
  assert.ok(end > from, `${key} 后面找不到下一个字段 ${until}，取段失去边界`);
  const slot = text.slice(from, end);
  assert.ok(slot.length > 120,
    `${key} 后面只有 ${slot.length} 个字符——这个枚举实际上没有判据，恒等于默认值`);
  return slot;
}

test("完整裁决里 orchestrationMode 有判据，不只有一行枚举值", () => {
  const seg = criterionSlot(SRC, "orchestrationMode=solo/staged_roles/parallel_roles", "roleNeeds 只能从");
  assert.match(seg, /判据/, "只有枚举值没有判据——这种枚举恒等于默认值 solo，整道门结构性哑掉");
  assert.match(seg, /staged_roles/, "没说什么情况该填 staged_roles");
  assert.match(seg, /parallel_roles/, "没说什么情况该填 parallel_roles");
  // 反方向的代价必须同时写明，否则判据会变成单向的"多派角色"压力。
  // 不能只匹配"更慢"：parallel_roles 那一档本身就写着"串行读完会明显更慢"，
  // 拿它当兜底会把「派错的代价」这条整句删掉也喂绿（2026-08-22 变异实测）。
  assert.match(seg, /代价/, "没写派错的代价——模型会为了显得强大而一律往上填");
  assert.match(seg, /多花/, "没说清代价是什么：派一次角色要多花几轮模型调用");
  assert.match(seg, /子报告|文字不是代码/, "没说清子报告是文字不是代码，读它比自己读更慢也更差");
});

test("快通道那条链路也有同一条判据", () => {
  // 两条链路各判各的：完整裁决要 19.8 秒，第一轮通常只有快通道的结果。只补一边等于
  // 第一轮仍然恒定 solo——而"该派哪些角色"恰恰是第一轮就要决定的事。
  const fast = topLevelFn("_fastRoutingFlags", { code: true });
  assert.match(fast, /orchestrationMode=solo\|staged_roles\|parallel_roles/);
  assert.match(fast, /判据/, "快通道的 orchestrationMode 仍然只有枚举值");
  assert.match(fast, /staged_roles/);
});

test("判据说的是「契约定没定」，不是「活大不大」", () => {
  // 这条不变量要紧：按"活大不大"判，一个大重构会被判成要派角色，而它其实一条线程
  // 就能走到底——派出去只会更慢，子报告还是文字不是代码。
  const seg = criterionSlot(SRC, "orchestrationMode=solo/staged_roles/parallel_roles", "roleNeeds 只能从");
  assert.match(seg, /契约/, "判据没说清分界是「契约定没定」");
  assert.match(seg, /拿不准就 solo|哪怕改动很大也填 solo/,
    "没给出退回 solo 的出口——模型拿不准时会往上填");
});

// ── 二、够得着 ──────────────────────────────────────────────────────────
test("run_subagent 在 agent 开局窗口里", () => {
  const table = /agent: \["read_file"[\s\S]*?\],\n  \};/.exec(SRC);
  assert.ok(table, "agent 核心表被改名或挪走了");
  const names = [...table[0].matchAll(/"([a-z_]+)"/g)].map((m) => m[1]);
  assert.ok(names.includes("run_subagent"),
    "编排整族要先花一轮 search_tools 才够得着——而『自己往下读』永远是更便宜的那条");
});

test("只放一个入口，没有把整族一起塞进窗口", () => {
  // 扩窗的判据是"结构性够不着"，不是"多多益善"。run_subagent 的 schema 自带 tasks
  // 数组和 wait，是整族单一入口；其余三个由它的描述指路，模型走到那一步时已经在编排了。
  const table = /agent: \["read_file"[\s\S]*?\],\n  \};/.exec(SRC)[0];
  const names = [...table.matchAll(/"([a-z_]+)"/g)].map((m) => m[1]);
  for (const t of ["run_worker", "spawn_multiple_agents", "await_subagent"]) {
    assert.ok(!names.includes(t), `${t} 也进窗口了——扩窗的理由是结构性够不着，不是把目录敞开`);
  }
});

// ── 三、描述不能以一句模型观察不到的前置条件开头 ──────────────────────────
function runSubagentDescription(text) {
  const m = /name: "run_subagent", description: "((?:[^"\\]|\\.)*)"/.exec(text);
  assert.ok(m, "run_subagent 的描述取不到");
  return m[1];
}

test("run_subagent 的描述先说这是干什么的，不以内部旗标当前置条件", () => {
  const desc = runSubagentDescription(SRC);
  assert.doesNotMatch(desc.slice(0, 120), /^Use only in structured-collaboration mode/,
    "开头是一句模型**观察不到**的前置条件（structured-collaboration mode 是内部画像旗标），"
    + "无法自查的条件读起来就是「别用」");
  assert.match(desc.slice(0, 160), /Dispatch a read-only specialist/,
    "开头应当先说清它是干什么的");
});

test("「什么时候别用」也要写明，别把它变成一个总该用的工具", () => {
  const desc = runSubagentDescription(SRC);
  assert.match(desc, /When NOT to use/, "只说什么时候用，模型就会在不该派的时候也派");
  assert.match(desc, /reading it yourself is both faster and better|faster and better/,
    "没说清反方向的代价：子报告是文字不是代码，一个人读几个文件更快也更好");
});

test("契约优先那条实质不能在改写里丢掉", () => {
  // 描述改写最容易顺手删掉的就是这句：staged_roles 先派**只读**角色收敛契约，
  // 然后才写。它不是修辞，是这个工具与 run_worker 的分界。
  const desc = runSubagentDescription(SRC);
  assert.match(desc, /staged_roles[\s\S]*read-only roles/,
    "契约优先（先派只读角色收敛契约再动手）这条实质被改没了");
});

test("网关那份目录和 main.js 里这份描述一字不差", () => {
  // 运行时以网关目录那份为准（见 [[two-tool-catalogs]]）：只改 main.js 而不同步
  // tools.json，等于改了个不生效的副本，而且两份会静静地越漂越远。
  const cloud = JSON.parse(readFileSync(join(HERE, "../../server/prompts/tools.json"), "utf8"));
  const list = Array.isArray(cloud) ? cloud : (cloud.tools || []);
  const hit = list.map((t) => t.function || t).find((f) => f?.name === "run_subagent");
  assert.ok(hit, "网关目录里没有 run_subagent");
  const local = runSubagentDescription(SRC).replace(/\\"/g, '"').replace(/\\n/g, "\n");
  assert.equal(hit.description, local,
    "网关目录和 main.js 的 run_subagent 描述漂了——运行时生效的是网关那份");
});

// ── 四、可证伪 ──────────────────────────────────────────────────────────
test("情景档案记下裁决判的编排模式，好让这次改动可以被证伪", () => {
  // 没有这个字段，"编排到底通没通"只能从 approach 的动词里反推，而那恰好分不清两种
  // 截然不同的情况：裁决压根没判成要编排（要改判据），和判了却没派出去（要改可达性）。
  const rec = topLevelFn("_recordEpisode", { code: true });
  assert.match(rec, /orch:/, "情景档案没记编排模式，这次改动就无法证伪");
  assert.match(rec, /orchestrationMode !== "solo"/,
    "solo 也记的话，这个字段在 90% 的回合里是噪音");
});
