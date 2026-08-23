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

function facts(run, { evidenceForRoot = null } = {}) {
  const fn = load("_executionFactSemanticFlags", {
    _aiIntentWorkspaceEvidence: evidenceForRoot || (() => null),
  });
  return fn(run);
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

test("判据一个字没放宽：三个条件缺一不可", () => {
  // 放宽它等于凭空造旗标——本文件守的是「兜底腿够得着自己的输入」，不是「更容易点亮」。
  for (const over of [
    { hasWorkspace: false },
    { snapshotReady: false },
    { topLevel: [] },
  ]) {
    const out = facts({ root: "/p/app" }, { evidenceForRoot: () => EV(over) });
    assert.notEqual(out.existingProject, true,
      `判据被放宽了：${JSON.stringify(over)} 也点亮了 existing_project`);
  }
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
