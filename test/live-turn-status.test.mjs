// 实时状态条的「等待上游首字节 / 接收中」必须说的是**当前这一轮**。
//
// 这两个标签原来读的是任务级首次事件（timeline.firstModelProgressAt / firstVisibleAt），
// 而这两个字段只在第一次有值时写入。于是第一轮首显之后它们永远非空，两个分支从第二轮起
// 再也进不去：模型请求发出后卡 10-30 秒没有首字节（中转不做流式转发、网关重试第 2/3 次），
// 界面上只有一只光跑的秒表，一个字都不说在等什么——而 agent 模式下绝大多数等待恰恰发生在
// 第二轮以后，这条标签在它最需要出现的地方恒定缺席。
//
// 更难查的是：同一份数据在 tooltip 里是逐轮全的（悬停能看到「#5 发起 1:02 · 响应头 - ·
// 模型进度 -」），可见标签却一个字没有——同一份事实两个出口说法不一致。
//
// 逐轮字段（requestStartedAt / firstProgressAt / firstVisibleAt / endedAt / attempts）
// timeline.turns 上早就齐了，这条修复不新增任何记录点，只是改成读它们。
import test from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, SRC as RAW_SRC, load } from "./helpers/source.mjs";

// _turnStatsText 必须保持自包含（test/logic.test.mjs 只注入这三个格式化函数就把它跑起来）。
const statsText = load("_turnStatsText", {
  _fmtElapsed: (ms) => `${Math.round(Number(ms) || 0)}ms`,
  _tokenShort: (n) => String(n),
  _dispUsd: (c) => `$${c}`,
});

const T0 = 1_700_000_000_000;
const line = (opts) => statsText(opts).html.replace(/<[^>]+>/g, "");

/** 一条模型轮次。缺省是「已发起、还没收到首字节、还没结束」。 */
function turn(over = {}) {
  return {
    stepIndex: 1, kind: "main", startedAt: T0,
    requestStartedAt: T0, responseHeadersAt: null, firstChunkAt: null,
    firstProgressAt: null, firstVisibleAt: null, endedAt: null, attempts: [{ index: 1 }],
    ...over,
  };
}
const timelineOf = (...turns) => ({
  startedAt: T0,
  firstModelProgressAt: turns.find((t) => t.firstProgressAt != null)?.firstProgressAt ?? null,
  firstVisibleAt: turns.find((t) => t.firstVisibleAt != null)?.firstVisibleAt ?? null,
  turns,
});

test("第一轮：请求已发出、上游还没开口时说明在等首字节", () => {
  const tl = timelineOf(turn());
  assert.match(line({ elapsedMs: 8000, timeline: tl, live: true }), /等待上游首字节/);
});

test("第一轮：开始收了但还没画出来时说「接收中」", () => {
  const tl = timelineOf(turn({ firstProgressAt: T0 + 900 }));
  const text = line({ elapsedMs: 8000, timeline: tl, live: true });
  assert.match(text, /接收中/);
  assert.doesNotMatch(text, /等待上游首字节/);
});

test("第二轮起同样要说话——这正是原来整条静默的地方", () => {
  // 第一轮完整跑完（任务级 firstModelProgressAt / firstVisibleAt 从此永远非空），
  // 第二轮刚发出去、还没首字节。
  const first = turn({ stepIndex: 1, firstProgressAt: T0 + 800, firstVisibleAt: T0 + 1200, endedAt: T0 + 5000 });
  const second = turn({ stepIndex: 2, startedAt: T0 + 9000, requestStartedAt: T0 + 9000 });
  const tl = timelineOf(first, second);
  assert.notEqual(tl.firstModelProgressAt, null, "任务级字段确实已经有值了（旧判据据此永远沉默）");
  const text = line({ elapsedMs: 30000, timeline: tl, live: true });
  assert.match(text, /等待上游首字节/, "第二轮卡在上游不开口，界面必须说出来");
  assert.match(text, /第 2 轮/, "多轮时要指明是哪一轮在等");
});

test("第二轮开始收内容但这一轮还没出字时是「接收中」", () => {
  const first = turn({ stepIndex: 1, firstProgressAt: T0 + 800, firstVisibleAt: T0 + 1200, endedAt: T0 + 5000 });
  const second = turn({ stepIndex: 2, startedAt: T0 + 9000, requestStartedAt: T0 + 9000, firstProgressAt: T0 + 12000 });
  const text = line({ elapsedMs: 30000, timeline: timelineOf(first, second), live: true });
  assert.match(text, /接收中/);
  assert.doesNotMatch(text, /等待上游首字节/);
});

test("物理重试要标出来：用户看到的不是一次长等待，而是第几次重来", () => {
  const t = turn({ stepIndex: 3, attempts: [{ index: 1 }, { index: 2 }, { index: 3 }] });
  const first = turn({ stepIndex: 1, firstProgressAt: T0 + 800, firstVisibleAt: T0 + 900, endedAt: T0 + 1000 });
  const text = line({ elapsedMs: 60000, timeline: timelineOf(first, t), live: true });
  assert.match(text, /重试 #3/);
  assert.match(text, /等待上游首字节/);
});

test("轮次都结束了（正在跑工具）就不加标签——工具卡自己在转", () => {
  const done = turn({ firstProgressAt: T0 + 800, firstVisibleAt: T0 + 1200, endedAt: T0 + 5000 });
  const text = line({ elapsedMs: 30000, timeline: timelineOf(done), live: true });
  assert.doesNotMatch(text, /等待上游首字节|接收中/);
});

test("异常留下的陈迹不许一直亮着：它后面还有已结束的轮次就当它不存在", () => {
  const stale = turn({ stepIndex: 1, endedAt: null });          // 结束时机漏写
  const done = turn({ stepIndex: 2, startedAt: T0 + 6000, requestStartedAt: T0 + 6000, firstProgressAt: T0 + 6100, firstVisibleAt: T0 + 6200, endedAt: T0 + 9000 });
  const text = line({ elapsedMs: 40000, timeline: timelineOf(stale, done), live: true });
  assert.doesNotMatch(text, /等待上游首字节|接收中/, "倒序扫到已结束的轮次就停，不再往前翻");
});

test("并行子体和主轮同时未结束时，说的是主轮", () => {
  const main = turn({ stepIndex: 4, kind: "main" });
  const sub = turn({ stepIndex: 5, kind: "subagent", startedAt: T0 + 100, requestStartedAt: T0 + 100, firstProgressAt: T0 + 200 });
  const text = line({ elapsedMs: 20000, timeline: timelineOf(main, sub), live: true });
  assert.match(text, /等待上游首字节/, "主轮还在等首字节，就报主轮");
  assert.doesNotMatch(text, /子体/);
});

test("只有子体在跑时如实说是子体，不冒充第 N 轮", () => {
  const done = turn({ stepIndex: 1, firstProgressAt: T0 + 800, firstVisibleAt: T0 + 900, endedAt: T0 + 1000 });
  const sub = turn({ stepIndex: 2, kind: "subagent", startedAt: T0 + 3000, requestStartedAt: T0 + 3000 });
  const text = line({ elapsedMs: 20000, timeline: timelineOf(done, sub), live: true });
  assert.match(text, /子体 · 等待上游首字节/);
});

test("还没开过任何一轮时退回任务级判据（首轮开跑前的等待照样有字）", () => {
  const tl = { startedAt: T0, firstModelProgressAt: null, firstVisibleAt: null, turns: [] };
  assert.match(line({ elapsedMs: 3000, timeline: tl, live: true }), /等待上游首字节/);
  assert.match(line({ elapsedMs: 3000, timeline: null, live: true }), /等待上游首字节/);
});

test("收尾（非 live）不加等待标签，任务级的「模型/首显」两项不变", () => {
  const done = turn({ firstProgressAt: T0 + 800, firstVisibleAt: T0 + 1200, endedAt: T0 + 5000 });
  const text = line({ elapsedMs: 30000, timeline: timelineOf(done), live: false });
  assert.doesNotMatch(text, /等待上游首字节|接收中/);
  assert.match(text, /模型 800ms/, "任务级首个模型事件仍按任务级显示");
  assert.match(text, /首显 1200ms/);
});

test("判据不再是任务级首次事件", () => {
  const at = RAW_SRC.indexOf("function _turnStatsText(");
  const fn = SRC.slice(at, RAW_SRC.indexOf("function _turnStatsTitle", at));
  const liveAt = fn.indexOf("if (live) {");
  assert.ok(liveAt > 0, "找不到 live 分支");
  const liveBlock = fn.slice(liveAt, fn.indexOf("if (firstProgressMs != null)", liveAt));
  assert.match(liveBlock, /timeline\?\.turns/, "live 分支必须看逐轮数据");
  assert.match(liveBlock, /endedAt/, "必须按「这一轮结束没有」判定");
  // 任务级字段只允许在「一轮都还没开过」那条退路上出现。
  const perTurn = liveBlock.slice(0, liveBlock.indexOf("} else if (!turns.length)"));
  assert.doesNotMatch(perTurn, /firstProgressMs|firstVisibleMs/,
    "逐轮判定里不许再读任务级首次事件——那正是第二轮起整条静默的原因");
  // 这个函数必须继续自包含：它被单独取出来跑，引入新 helper 会当场 ReferenceError。
  assert.doesNotMatch(liveBlock, /_timelineElapsed\(/, "逐轮判据直接看字段是否为空即可");
});
