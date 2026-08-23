// 裁决还没到的时候，harness 不是「少说几句」，而是主动对模型下达**反向指令**。
//
// 实测（2026-08-23，跑真 _agentDecisionFrameBlock）：
//   pending 画像 → 906 字符，**含「小任务律：直接用最短证据链完成；能一两个工具搞定
//                  就别升级成长流程」**，而交付规格/先读懂再动手/架构质量/变更半径/
//                  可维护升级 五条工程律**一条都没有**
//   ai 画像      → 3242 字符，五条全在，不含小任务律
//
// 对「帮我做一个多人协作的待办应用」这种话，第一发照样说「这是小任务」。
// 成因：那七个否定项判的是「模型声明了它不是大活」，而 _mergeAiIntentProfile 在
// verdict 为 null 时把全部维度**强制 false**——七项必然同时成立，条件恒真。
// 而第一发必然是 pending（等待窗口 6 秒 vs 裁决 6.9~19.8 秒），也恰恰是决定技术栈
// 和目录结构的那一发。用户抱怨的「就喜欢写 MVP 不写正常的」，这里是字面成因。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load, loadConst, fnSource as topLevelFn } from "./helpers/source.mjs";

function frameFn() {
  const deps = {};
  for (let i = 0; i < 60; i++) {
    try { return load("_agentDecisionFrameBlock", deps); }
    catch (e) {
      const m = /(\w+) is not defined/.exec(String(e?.message));
      if (!m) throw e;
      try { deps[m[1]] = loadConst(m[1]); } catch { try { deps[m[1]] = load(m[1]); } catch { deps[m[1]] = () => ""; } }
    }
  }
  throw new Error("装不起来");
}
const frame = frameFn();
const TASK = "帮我做一个多人协作的待办应用";
const LAWS = ["交付规格", "先读懂再动手", "架构质量", "变更半径", "可维护升级"];

test("裁决没到时，绝不对模型说「这是小任务」", () => {
  const out = String(frame(TASK, { intentSource: "pending" }, null) || "");
  assert.ok(!out.includes("小任务律"),
    "第一发就告诉模型用最短证据链、别升级成长流程——而那一发正是决定技术栈的一发");
});

test("模型真的声明了这是小活时，照常说", () => {
  // 这条守的是「不是把它删掉」：小任务律本身是对的，错的是在拿不准时也说。
  const out = String(frame("把按钮文案改一下", {
    intentSource: "ai", applies: true,
  }, null) || "");
  assert.ok(out.includes("小任务律"),
    "连模型明确声明的小活都不说了——那是把一条有用的纪律删掉，不是修 bug");
});

test("快通道落定也算数，不必等完整裁决", () => {
  // 判据是「拿不准就少说一句」，不是夺能力，所以不用 === "ai" 那种严格闸。
  const out = String(frame("把按钮文案改一下", { intentSource: "fast" }, null) || "");
  assert.ok(out.includes("小任务律"), "快通道明明落定了却仍然当成拿不准");
});

test("裁决到场时那份决策框一个字节都没变", () => {
  const landed = {
    intentSource: "ai", architectureQuality: true, substantial: true, requiresPlan: true,
    projectEngineering: true, applies: true, architectureMode: "design_new", changeScope: "project",
  };
  const out = String(frame(TASK, landed, null) || "");
  for (const law of LAWS) {
    assert.ok(out.includes(law), `裁决到位时 ${law} 律不见了——这次改动越界了`);
  }
  assert.ok(!out.includes("小任务律"), "大活里混进了小任务律");
  assert.ok(out.length > 2500, `裁决到位那份只剩 ${out.length} 字符，被误伤了`);
});

test("判据用的是裁决到场状态，不是新造一个维度", () => {
  // 维度受一条元测试白名单管着；而 intentSource 是裁决的到场状态，两者性质不同。
  const body = topLevelFn("_agentDecisionFrameBlock", { code: true });
  assert.match(body, /p\.intentSource !== "pending"/,
    "判据没了或被换成了别的——pending 那一发又会拿到反向指令");
  assert.doesNotMatch(body, /p\.verdictPending|p\.isPending/,
    "新造了一个维度来判这件事——那会掉进维度白名单的元测试");
});

test("这一条只挡小任务律，不顺手挡别的", () => {
  // 反向断言：pending 时其余该出现的内容不能被一起吞掉，否则就是又造了一个空框。
  const pending = String(frame(TASK, { intentSource: "pending" }, null) || "");
  assert.ok(pending.length > 700,
    `pending 那份只剩 ${pending.length} 字符——把别的内容也一起挡掉了`);
});
