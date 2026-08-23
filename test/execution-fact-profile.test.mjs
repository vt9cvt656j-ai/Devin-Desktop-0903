// 语义画像在生产上 144/144 全空，agent_engineering 挂载 0 次——那 13KB 的架构纪律和
// 「优先用成熟主流方案」一次都没到过模型手里（2026-08-23 实测，近 12 小时）。
//
// 系统里本来就有一条**不依赖模型声明**的兜底腿：按执行事实推旗标。它存在的全部理由就是
// 「分类器不可用时也要有旗标」——用户 97% 的请求跑在一个产不出大 JSON 的模型上，这条腿
// 是那种情况下唯一的来源。
//
// 而它原来唯一的输入 `run._intentState?.context?.workspaceEvidence` 恰恰长在**分类器**
// 那条链上：分类器没跑，兜底腿也跟着死。它在最需要它的那种情况下必然为空。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load, fnSource as topLevelFn } from "./helpers/source.mjs";

const EV = (over = {}) => ({ hasWorkspace: true, snapshotReady: true, topLevel: ["src", "package.json"], ...over });

function facts(run, { evidenceForRoot = null, ...opts } = {}) {
  const fn = load("_executionFactSemanticFlags", {
    _aiIntentWorkspaceEvidence: evidenceForRoot || (() => null),
  });
  return fn(run, Object.keys(opts).length ? opts : null);
}

test("分类器没跑时，工作区证据自己现算——这条腿的全部意义就在这", () => {
  const seen = [];
  const out = facts({ root: "/p/app" }, {
    evidenceForRoot: (root) => { seen.push(root); return EV(); },
  });
  assert.deepEqual(seen, ["/p/app"], "没有去现算——分类器一不可用，兜底腿就跟着死了");
  assert.equal(out.existingProject, true);
});

test("分类器跑了就用它那份，不重复计算", () => {
  let called = 0;
  const out = facts(
    { root: "/p/app", _intentState: { context: { workspaceEvidence: EV() } } },
    { evidenceForRoot: () => { called++; return EV(); } },
  );
  assert.equal(called, 0, "已经有证据了还去重算，纯浪费");
  assert.equal(out.existingProject, true);
});

test("没有工作区、或既无目录快照又无技术栈，都不点亮", () => {
  // 本文件守的是「兜底腿够得着自己的输入」，不是「更容易点亮」。
  for (const over of [
    { hasWorkspace: false },                       // 连工作区都没有
    { snapshotReady: false },                      // 快照冷、且这份证据里没有栈
    { topLevel: [] },                              // 快照热但目录空
    { snapshotReady: false, stack: {} },           // 栈字段在但全空
    { snapshotReady: false, stack: { lang: "" } }, // 空串不是「识别出了栈」
  ]) {
    const out = facts({ root: "/p/app" }, { evidenceForRoot: () => EV(over) });
    assert.notEqual(out.existingProject, true,
      `判据被放宽了：${JSON.stringify(over)} 也点亮了 existing_project`);
  }
});

test("目录快照过期、但技术栈识别得出来 → 照样算已有项目", () => {
  // 这是修的那一刀。_agentContextCache 带 5 分钟 TTL，会话第一发时它必然是冷的——
  // 而这条腿存在的全部理由就是第一发。_projectStacks 没有 TTL，且扫不出栈事实时会被
  // 显式 delete，所以「栈识别得出来」是比目录清单更强的证据，不是更弱的近似。
  for (const stack of [{ lang: "TypeScript" }, { framework: "React" }, { packageManager: "pnpm" }]) {
    const out = facts({ root: "/p/app" }, {
      evidenceForRoot: () => EV({ snapshotReady: false, topLevel: [], stack }),
    });
    assert.equal(out.existingProject, true,
      `${JSON.stringify(stack)}：栈已识别却还是没点亮——第一发仍然带空画像出门`);
  }
});

test("模型画像缺席时，engineering 由执行事实补上", () => {
  // engineering 在网关侧一面旗门着两样东西：agent_engineering（13KB 架构纪律 +
  // 「优先用成熟主流方案」）和整个 4.3MB 专业语料的自动注入。它此前唯一的来源是模型裁决，
  // 于是线上 317/383（83%）的装配两样一起够不着。
  const out = facts({ root: "/p/app" }, {
    evidenceForRoot: () => EV(),
    modelProfileMissing: true,
  });
  assert.equal(out.projectEngineering, true,
    "模型画像没到，事实腿也没补 engineering——那一轮就是零架构纪律、零语料");
});

test("模型画像到了就以模型为准，事实腿不许盖上去", () => {
  // 模型判「这不是工程活」是**一个判断**，不是缺席。事实腿只在缺席时兜底。
  const out = facts({ root: "/p/app" }, { evidenceForRoot: () => EV() });
  assert.notEqual(out.projectEngineering, true,
    "模型画像在场时事实腿仍然强点 engineering——这会覆盖模型自己的判断");
});

test("没有工作区就不补 engineering（不许凭空造）", () => {
  const out = facts({ root: "/p/app" }, {
    evidenceForRoot: () => EV({ hasWorkspace: false }),
    modelProfileMissing: true,
  });
  assert.notEqual(out.projectEngineering, true,
    "连工作区都没有也补 engineering——那是凭空造旗标");
});

test("循环边界那条腿也带着「模型画像到没到」", () => {
  // 第一发补上了不等于全程补上。一轮里模型始终没回（线上那 159 次上游限流就是这个形状），
  // 循环边界每一步装配都要重新给出这个条件，否则中途每一次又退回零纪律、零语料。
  // 变异实测：把这个参数从循环边界拿掉，本文件其余 12 条测试全绿——所以这条必须单独钉。
  const body = topLevelFn("_applyExecutionFactProfile", { code: true });
  assert.match(body, /_executionFactSemanticFlags\(\s*run\s*,/,
    "循环边界调用事实腿时没传第二个参数——那个条件整轮都丢了");
  assert.match(body, /modelProfileMissing/, "没把「模型画像到没到」传下去");
  assert.match(body, /_semanticProfileFromModel/,
    "条件不是从「模型来源的画像」来的——换成别的近似判据，事实腿就会冒充模型判过了");
});

test("补出来的事实确实变成网关认得的 engineering 旗标", () => {
  // 光在事实对象上点一个字段没用：网关门的是**旗标名**。这条把「事实 → 旗标」这一跳钉住，
  // 免得字段改名后整条链静默断掉（本仓库踩过同款：调用点在、路径不会被走到）。
  const profile = load("_ideSemanticProfile", {});
  const out = facts({ root: "/p/app" }, {
    evidenceForRoot: () => EV(),
    modelProfileMissing: true,
  });
  const flags = profile(out).split(":")[1].split(",").filter(Boolean);
  assert.ok(flags.includes("engineering"),
    `事实腿产出的字段没映射成 engineering 旗标，网关那边等于没收到：${flags.join(",")}`);
  assert.ok(flags.includes("existing_project"), `existing_project 也丢了：${flags.join(",")}`);
});

test("空目录（从零建）不点 existing_project——那正是它和已有项目的分界", () => {
  const out = facts({ root: "/p/empty" }, { evidenceForRoot: () => EV({ topLevel: [] }) });
  assert.notEqual(out.existingProject, true);
});

test("现算抛异常时安静退回，不能把整条画像带崩", () => {
  const out = facts({ root: "/p/app" }, {
    evidenceForRoot: () => { throw new Error("扫描失败"); },
  });
  assert.deepEqual(out, {}, "现算失败应当返回空事实，而不是抛出去");
});

test("落过盘的一轮就是工程活——这条不依赖任何证据来源", () => {
  const out = facts({ root: "", _writeLedger: [{ path: "a.ts", ok: true }] });
  assert.equal(out.implementation, true);
});

test("什么都没发生时一个旗标都不给（不许凭空造）", () => {
  assert.deepEqual(facts({ root: "" }), {});
  assert.deepEqual(facts(null), {});
  assert.deepEqual(facts({ root: "/p/app", _writeLedger: [] }, { evidenceForRoot: () => null }), {});
});

test("一个字的用户措辞都不看", () => {
  // 分类器不可用时**不许**退回词表猜意图（这条另有测试正面钉着）。
  // 这条腿只认已经发生的事，和猜测是两回事。
  const body = topLevelFn("_executionFactSemanticFlags", { code: true });
  // 正则要贴着**标识符边界**：写成 /text/ 会被 `context` 里的 "text" 喂红，
  // 那样这条断言在干净代码上就是红的（2026-08-23 实测）。
  assert.doesNotMatch(body, /\b(_originalText|userText|taskText|query|prompt)\b/,
    "这条腿开始读用户措辞了——那是猜意图，不是执行事实");
  // 词表猜意图的形态也一并挡掉：分类器不可用时不许退回关键词匹配（另有测试正面钉着）。
  assert.doesNotMatch(body, /\.test\(|includes\(["'\u0060]|match\(/,
    "出现了词表/正则匹配——那正是这条腿刻意不做的事");
});
