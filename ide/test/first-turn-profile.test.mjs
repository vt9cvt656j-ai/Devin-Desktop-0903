// 会话第一轮的第一发模型调用，语义画像是空的——而那正是决定技术栈和目录结构的一发。
//
// 后果不是"少挂一个模块"：画像空掉时同时丢掉
//   · IDE 侧 2000 多字符的工程决策律（交付规格 / 先读懂再动手 / 变更半径 / 可维护升级）；
//   · 网关侧 agent_engineering 整块 13KB——里面逐字写着模块边界、反硬编码，以及
//     「Prefer a mature, mainstream solution over building your own」；
//   · 按领域限定的语料检索与专业域小抄。
// 等第二轮补上时，模型已经把栈和目录写死了。用户看到的就是「不懂架构、不用主流库」。
//
// 两处成因都是**时序**，不是缺规则：
//   一、打字空闲时的预热对「本进程第一条消息」永远不跑（凭证只在 sendPrompt 内部写）；
//   二、快通道几秒就回并赢下 race，把整场等待退掉，而完整裁决（唯一产出 domain /
//       architectureMode / researchMode 的那条腿）还有几秒**已经批过**的预算没用。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, fnSource as topLevelFn } from "./helpers/source.mjs";

const PREFETCH = topLevelFn("_prefetchIntentFromComposer", { code: true });

// ── 一、预热必须对第一条消息生效 ────────────────────────────────────────
test("预热不再只认「跑过一轮才有」的那份凭证", () => {
  // _lastGoodAiConfig 全仓只有一处写入，在 sendPrompt 内部。只认它 = 应用启动后的
  // 第一条消息一次都不预热，而那是唯一一条要付等待窗口的消息。
  assert.doesNotMatch(PREFETCH, /if \(!inTauri \|\| !_lastGoodAiConfig\) return;/,
    "还是只认跑过一轮才有的凭证——第一条消息永远不预热，而它恰恰是唯一需要预热的那条");
  assert.match(PREFETCH, /loadConfig\(\)/,
    "没有回落到持久化配置，第一条消息仍然拿不到凭证");
});

test("预热用的是纯读配置，绝不能走会弹登录门的那条", () => {
  // _readyAiConfig 会调 michaelAccessGate()。挂在「用户正在打字」这条空闲路径上，
  // 等于随时可能在打字中途弹一个登录框出来。
  assert.doesNotMatch(PREFETCH, /_readyAiConfig/,
    "预热走了会触发登录门的配置入口——用户打字打到一半会被弹框打断");
  assert.doesNotMatch(PREFETCH, /michaelAccessGate/, "同上");
});

test("凭证不全就不发：缺一样这次请求必然失败", () => {
  assert.match(PREFETCH, /baseUrl && [\w.]*\.?apiKey && [\w.]*\.?model|c\.baseUrl && c\.apiKey && c\.model/,
    "没检查三件套就发预热——凭证不全时是白发一次");
});

test("发出去的是新解析的那份凭证，不是原来那个只在跑过一轮后才有的变量", () => {
  // 只加了回落却仍然把 _lastGoodAiConfig 传下去，等于回落白做——这是最容易漏的一半。
  const call = /_aiIntentProfile\(t, ([A-Za-z_$][\w$]*), sess, ctx\)/.exec(PREFETCH);
  assert.ok(call, "预热的发起调用不见了");
  assert.notEqual(call[1], "_lastGoodAiConfig",
    "回落算出来的凭证没被用上——第一条消息仍然预热不了");
});

test("预热不额外多发一次请求", () => {
  // 单飞去重 + 同键缓存是这条改动「零成本」的全部依据。它们没了，预热就变成每次打字
  // 都多打一次付费请求。
  assert.match(PREFETCH, /_aiIntentCache\.get\(key\) \|\| _aiIntentInflight\.get\(key\)/,
    "去重那道判据没了——预热会变成额外的付费请求");
  assert.match(PREFETCH, /if \(t === _intentPrefetchedText\) return;/, "同一句话被重复预取");
});

// ── 二、快通道赢了 race 不等于这场等待该结束 ────────────────────────────
const SEND = topLevelFn("sendPrompt", { code: true });

test("快通道赢下 race 之后，剩余预算继续等完整裁决", () => {
  // domain / architectureMode / researchMode 三样只有完整裁决产得出，
  // 而快通道只回四个枚举加一批布尔旗标。
  const seg = SEND.slice(SEND.indexOf("_intentWaitPaid"), SEND.indexOf("_intentWaitPaid") + 4000);
  assert.match(seg, /_turnIntentExactPromise && !_turnIntentState\.settled/,
    "快通道一落定就把整场等待退掉了——完整裁决赶不上决定架构的那一发");
  assert.match(seg, /_FIRST_TURN_INTENT_WAIT_MS - \(Date\.now\(\) - _waitStartedAt\)/,
    "续等的额度不是从原窗口里扣的——那会变成真的多等一个窗口");
});

test("续等只花原来那笔预算，绝不新开一个窗口", () => {
  const seg = SEND.slice(SEND.indexOf("_waitStartedAt"), SEND.indexOf("_waitStartedAt") + 3000);
  // 上限必须仍是同一个常量。写成 setTimeout(..., _FIRST_TURN_INTENT_WAIT_MS) 就是
  // 在已经等过一截之后再等一整个窗口——用户那边量到的是首轮卡两倍时间。
  assert.doesNotMatch(seg, /_restTimer = setTimeout\(resolve, _FIRST_TURN_INTENT_WAIT_MS\)/,
    "续等又开了一个完整窗口——首轮会卡成两倍");
  assert.match(seg, /_left > 50/, "没有下限保护：只剩几毫秒时还去开一次定时器纯属浪费");
});

test("等不到照样往下走，迟到的旗标由循环边界补", () => {
  // 这条守的是「不能把它写成必须等到」：上游慢的时候会把每一轮都卡死。
  //
  // 取窗口必须从 `_left > 50` 之后开始：从 `_waitStartedAt` 开始的话，**外层那个**
  // Promise.race 也在窗口里，把续等改成死等照样能匹配到——那样这条就是绿着的摆设
  // （2026-08-23 变异实测确实如此）。
  const at = SEND.indexOf("_left > 50");
  assert.ok(at > 0, "续等那道下限保护不见了，这条断言失去落点");
  const rest = SEND.slice(at, at + 900);
  assert.match(rest, /Promise\.race\(/, "续等写成了 await 完整裁决——上游慢时会把这一轮卡死");
  assert.match(rest, /clearTimeout\(_restTimer\)/, "定时器没清，会吊住一个 handle");
});

test("续等只等完整裁决那条腿，不会把快通道再等一遍", () => {
  const seg = SEND.slice(SEND.indexOf("_waitStartedAt"), SEND.indexOf("_waitStartedAt") + 3000);
  const race2 = seg.slice(seg.indexOf("_left > 50"));
  assert.doesNotMatch(race2.slice(0, 500), /_fastRoute\b/,
    "把已经落定的快通道又放进第二场 race——它会立刻兑现，续等等于没写");
});

// ── 三、行为闸门的边界不许被这次改动动到 ────────────────────────────────
test("请求头可以用快通道，行为闸门仍然只认完整裁决", () => {
  // 这是本仓库写死的一条边界（test/intent-timing.test.mjs 正面守着）：
  // 给模型的**信息**可以走快通道，harness 自己的**闸门**只认完整裁决。
  assert.doesNotMatch(SRC, /_uiTurnEngineering = _fastRouteProfile|run\.engineering = _fastRouteProfile/,
    "快通道的结果被当成行为闸门的依据了");
});
