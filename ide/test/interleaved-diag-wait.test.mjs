import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

/*
 * 「改完自动检查」那道门的等待循环。
 *
 * 原来的退出条件是「**任一**目标出现 marker 就收」。一批里同时改了 src/app.ts 和
 * src/engine.rs：Monaco 自带的 TS worker 约 300ms 就为 app.ts 推出一条 marker → 循环
 * 立刻退出 → 此时 rust-analyzer 一条都还没到（刚被 didOpen 告知这个文件，冷启动几秒
 * 起步）→ engine.rs 的 markers 为空 → 新增错误数不含它 → 门放行，**engine.rs 里新
 * 引入的编译错误一句提示都没有**，智能体照常收尾报「已完成」。它还进不了 unchecked
 * 名单（满足 isRunning，走的是「查过了、是干净的」那条腿），连诚实兜底都没有。
 *
 * 这条测试真的跑那段循环（用假时钟，不真等）。
 */
const SRC = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");

// setTimeout 在 new Function 里拿的是全局那个；用假时钟推进时间即可，等待仍是真实的
// 150ms×N —— 为了不让测试变慢，把总闸和 TS 闸按毫秒缩小传进去。
function runFast({ targets, markersAt, hard = 400, ts = 120 }) {
  const i = SRC.indexOf("    const started = Date.now();\n    const hardDeadline");
  const tail = "      if (settled || now >= hardDeadline) break;\n    }";
  const body = SRC.slice(i, SRC.indexOf(tail, i) + tail.length);
  const monaco = { editor: { getModelMarkers: ({ resource }) => (Date.now() - t0 >= (markersAt[resource] ?? Infinity) ? [{ severity: 8 }] : []) } };
  const t0 = Date.now();
  const fn = new Function("targets", "_INTERLEAVED_DIAG_MAX_WAIT_MS", "_INTERLEAVED_DIAG_TS_WAIT_MS", "monaco",
    `return (async () => { ${body} })();`);
  return fn(targets, hard, ts, monaco).then(() => ({ elapsed: Date.now() - t0, targets }));
}

test("秒回的 TS 目标不许替慢的 rust/python 提前结束等待", async () => {
  const targets = [
    { rel: "src/app.ts", isTs: true, jsFamily: true, model: { uri: "ts" } },
    { rel: "src/engine.rs", isTs: false, jsFamily: false, model: { uri: "rs" } },
  ];
  // TS 30ms 就出结果；rust 要 250ms。
  const { targets: out } = await runFast({ targets, markersAt: { ts: 30, rs: 250 }, hard: 900, ts: 200 });
  assert.equal(out[1]._diagSettled, true,
    "rust 目标还没结算就退出了 —— 它新引入的编译错误一条都读不到，门直接放行");
});

test("慢目标一直不出结果时，各自超时，不把整批拖满总闸", async () => {
  const targets = [
    { rel: "a.ts", isTs: true, jsFamily: true, model: { uri: "ts" } },
    { rel: "b.rs", isTs: false, jsFamily: false, model: { uri: "rs" } },
  ];
  const { elapsed, targets: out } = await runFast({ targets, markersAt: {}, hard: 700, ts: 150 });
  assert.ok(out.every((t) => t._diagSettled), "有目标没结算");
  assert.ok(elapsed >= 650, `只等了 ${elapsed}ms —— 没等到非 TS 目标自己的期限`);
  assert.ok(elapsed < 1400, `等了 ${elapsed}ms，超过总闸太多`);
});

test("TS 目标用的是它自己那个短期限，不是总闸", async () => {
  const targets = [{ rel: "a.ts", isTs: true, jsFamily: true, model: { uri: "ts" } }];
  const { elapsed } = await runFast({ targets, markersAt: {}, hard: 3000, ts: 200 });
  assert.ok(elapsed < 900, `只有 TS 目标却等了 ${elapsed}ms —— TS 由 Monaco 自带 worker 出结果，几百毫秒的事，等久了是白等`);
});

test("所有目标都出结果就立刻收，不空等", async () => {
  const targets = [
    { rel: "a.ts", isTs: true, jsFamily: true, model: { uri: "ts" } },
    { rel: "b.rs", isTs: false, jsFamily: false, model: { uri: "rs" } },
  ];
  const { elapsed } = await runFast({ targets, markersAt: { ts: 20, rs: 40 }, hard: 3000, ts: 2000 });
  assert.ok(elapsed < 500, `都出结果了还等了 ${elapsed}ms`);
});

/**
 * 干净的 .js 不许等满总闸。
 *
 * 这是这条路径上单条最大的等待：`_jsFamily`（含 js/jsx/mjs/cjs）在上游算对了，
 * 但 targets 只带了 `isTs`，于是期限那一行看不见它，js 家族全部落进 LSP 那条
 * 4 秒的腿。而且是**必然**等满——tsconfig 那边 checkJs:false，.js 只出语法诊断，
 * 语法没错就是 0 个 marker，`has` 恒为假，唯一出路是 own(t) 到点。
 *
 * 判据钉在「.js 用的是 Monaco 那条短期限」，不钉具体毫秒。
 */
test("干净的 .js 走 Monaco 的短期限，不是 LSP 的总闸", async () => {
  const targets = [{ rel: "a.js", isTs: false, jsFamily: true, model: { uri: "js" } }];
  const { elapsed } = await runFast({ targets, markersAt: {}, hard: 3000, ts: 200 });
  assert.ok(elapsed < 900,
    `干净的 .js 等了 ${elapsed}ms —— 它的诊断来自 Monaco 自带 worker，不来自 LSP，`
    + "落进 4 秒那条腿是白等，而且因为 checkJs:false 它必然等满");
});

test("真 LSP 语言仍然拿满冷启动预算", async () => {
  // 反向：上面那条不许顺手把 .py/.rs/.go 也缩短——4 秒对真 LSP 是刻意的冷启动预算。
  const targets = [{ rel: "a.rs", isTs: false, jsFamily: false, model: { uri: "rs" } }];
  const { elapsed } = await runFast({ targets, markersAt: {}, hard: 700, ts: 150 });
  assert.ok(elapsed >= 650, `rust 只等了 ${elapsed}ms —— 冷启动预算被缩掉了`);
});
