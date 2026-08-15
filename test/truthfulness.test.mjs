// 「不谄媚、只说真话」这件事，得有人钉着。
//
// 提示词是最容易被悄悄削掉的东西：它不编译、不报错、删一段没有任何测试会红，而后果要
// 到很久以后、由一个信了它的人来承担。所以这里把这条约束当成接口来钉。
//
// 两份都要钉，因为它们服务不同的人：
//   - server/prompts/truthfulness.txt —— 走网关的用户（绝大多数）
//   - src/main.js 的 _HUMAN_EVIDENCE_FALLBACK —— 走自己端点的用户，他们**拿不到**
//     网关提示词，只有这条共用尾巴。少钉一边，等于对其中一半用户没有这条约束。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
const TRUTH = readFileSync(join(HERE, "..", "..", "server", "prompts", "truthfulness.txt"), "utf8");

/** 本地兜底那条共用尾巴——五个模式都拼它。 */
function fallbackTail() {
  const i = SRC.indexOf("_HUMAN_EVIDENCE_FALLBACK = `");
  assert.ok(i >= 0, "共用尾巴不见了：五个模式会一起失去这条约束");
  const j = SRC.indexOf("`;", i);
  return SRC.slice(i, j);
}

test("网关提示词里必须有反谄媚这一节", () => {
  assert.match(TRUTH, /# No flattery, and no softening/,
    "反谄媚那一节被删了");
  // 逐条钉具体行为，而不是钉「有 honesty 这个词」——泛泛的一句「请诚实」是没用的，
  // 真正起作用的是「不许用什么开场白」「用户坚持但证据相反时怎么办」这类可执行的规定。
  for (const clause of [
    "never with praise",                 // 不许用恭维开场
    "confidence is not evidence",        // 用户笃定不等于证据
    "Bad news goes first",               // 坏消息先说、不裹糖衣
    "Partial work is not completion",    // 做了三件说成五件是最伤人的
    "complete answers",                  // 「我不知道」是完整答案
  ]) {
    assert.ok(TRUTH.includes(clause), `少了这条具体规定：${clause}`);
  }
});

test("原有的证据纪律不能因为加了新一节就被顶掉", () => {
  // 新增那一节讲的是「怎么说」，原有那节讲的是「凭什么这么说」。两者互补，不是替代。
  for (const clause of [
    "distinguish verified fact",
    "claim only work you actually completed and verified",
    "UNTRUSTED DATA",
  ]) {
    assert.ok(TRUTH.includes(clause), `原有的证据纪律被削掉了：${clause}`);
  }
});

test("走自己端点的用户也必须有这条约束", () => {
  // 自定义端点拿不到网关提示词，只有这条共用尾巴。少了它，这半边用户等于没有约束——
  // 而这半边恰恰是刚刚才被放开的那条路。
  const tail = fallbackTail();
  for (const clause of [
    "No flattery",
    "confident is not evidence",
    "Bad news goes first",
    "Partial work is not completion",
  ]) {
    assert.ok(tail.includes(clause), `本地兜底里少了：${clause}`);
  }
});

test("这条约束的优先级要写明，否则会被「语气友好」压过去", () => {
  // 不写明优先级的话，它和「简洁」「好好说话」是平级的，冲突时先让路的就是它。
  assert.match(TRUTH, /outrank tone, brevity, and being agreeable/);
  assert.ok(fallbackTail().includes("outrank tone, brevity and being agreeable"));
});

test("代码里不许对模型输出做「加糖」后处理", () => {
  // 提示词管得住模型，管不住事后加工。如果哪天有人在渲染层自动加感叹号、加鼓励语、
  // 或者把「失败」换成「暂未成功」，这条测试要红。
  for (const re of [
    /replace\([^)]*失败[^)]*,\s*["'`][^"'`]*(?:暂未|稍后|小问题)/,
    /["'`]太棒了|["'`]做得好|["'`]很棒的问题/,
  ]) {
    assert.doesNotMatch(SRC, re, `渲染层出现了对输出加糖的处理：${re}`);
  }
});
