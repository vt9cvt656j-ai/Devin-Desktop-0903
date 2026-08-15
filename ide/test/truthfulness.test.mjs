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

test("反谄媚纪律要发到 worker 和子智能体——它们才是真正交付的那一层", () => {
  // 网关对 subagent 模式**只注入工具、不注入系统提示词**，所以主智能体那份 truthfulness
  // 到不了这一层。而 worker 恰恰是改文件、跑验证、然后交简报的角色；它没有这条纪律，
  // 简报天然偏向报喜，主智能体复核的又是跨模块契约而不是「有没有把失败埋在中间」。
  // 表现就是：单干时诚实，一并行就出现「五件事报三件」。
  assert.match(SRC, /const _SUBAGENT_TRUTH = `/, "子智能体的真话下限不见了");
  const i = SRC.indexOf("const _SUBAGENT_TRUTH = `");
  const seg = SRC.slice(i, SRC.indexOf("`;", i));
  for (const clause of ["FIRST", "name those two", "could not verify"]) {
    assert.ok(seg.includes(clause), `子智能体的下限里少了：${clause}`);
  }
  // 光有常量不算数，得真的拼进去。
  assert.match(SRC, /\+ _SUBAGENT_TRUTH;/, "常量写了却没挂到 sysPrompt 上");
});

test("硬防线要拦提示词点名禁止的那几句恭维开场", () => {
  // 提示词是软约束（模型可以不听），剥离器是硬约束。以前硬约束打的是「我明白了」
  // 这类状态应答，而提示词点名的「好问题/你说得对/Great question」一句都不拦——
  // 靶子对不上，等于两层都漏。
  const i = SRC.indexOf("function _stripAckOpeners");
  assert.ok(i >= 0);
  const seg = SRC.slice(i, i + 4000);
  for (const opener of ["好问题", "你说得", "Great (?:question|point)", "Good (?:catch|question|point)", "right", "Thanks for"]) {
    assert.ok(seg.includes(opener), `剥离器不拦这句恭维：${opener}`);
  }
  // 剥到空就不剥：「你说得对。」独立成句时剥掉等于把整条回复吞掉。
  assert.ok(seg.includes("if (!next.trim()) break;"), "缺少防空守卫，会把纯恭维的回复剥成空白");
});

test("「温和解释」这档不能把真话下限一起温和掉", () => {
  // 用户在设置里点两下就能切到这一档。只注入「回答风格：温和解释。」四个字，
  // 最容易被读成「坏消息要包一层」。
  assert.match(SRC, /profile\.tone === "warm"/, "warm 档没有任何下限限定");
  assert.match(SRC, /坏消息仍然先说/);
});

test("用户纠正你——口味照收，事实主张要先核对", () => {
  // 自适应档案原来把所有纠正无条件当长期偏好，还会立刻覆盖持久记忆。
  // 「不是这个，useEffect 的清理函数是同步执行的」长得就像一次纠正，但它是个错误的
  // 事实主张；写进记忆之后会在此后每一轮被当成事实注入。
  assert.match(SRC, /只对口味类纠正成立/);
  assert.match(SRC, /不要把这条错误主张写进记忆/);
});

test("方案本身有问题时要在动手前说——最常见的谄媚不是假话，是沉默", () => {
  assert.ok(TRUTH.includes("silently implementing a plan you believe is wrong"),
    "缺少「方案有问题要先说」这条——而它正是最常见的那种谄媚");
  // 必须写明它和「只做被要求的事」不冲突，否则两条规则会对撞，而让路的总是这条。
  assert.ok(TRUTH.includes('not unrequested honesty'));
  assert.ok(TRUTH.includes("Say it once"), "没写「说一次」会退化成每轮一段风险清单");
});
