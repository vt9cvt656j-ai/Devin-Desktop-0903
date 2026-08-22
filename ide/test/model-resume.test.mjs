// 断线续传：连接在出字过程中断掉时，应当从断点接着写，而不是把整轮判死。
// 这里直接从 src/main.js 里取真函数来跑（acorn 定位），不复刻逻辑——复刻出来的
// 测试只能证明复制品是对的。
import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs";
// 按名字取真源码只有一份实现：test/helpers/source.mjs 的 fnSource（acorn 按 AST 边界切）。
// 这个文件后面还对源码原文下断言，所以 SRC 仍绑定 main.js 原文。
import { SRC, load } from "./helpers/source.mjs";
// 依赖既可能在构造时缺，也可能在**调用时**才缺（默认参数、函数体里的调用）。
// 所以既在构造时补，也在一次真实调用里补，直到不再抛 ReferenceError。
//
// 依赖是 helper 的 load() 按名字从 main.js 抓的真源码，只有一处例外：命中 OVERRIDES 的
// 名字换成字面量。这里唯一命中的是 _AI_MODEL_RETRY_DELAY_MS（生产值 2_000）——下面
// 「出字之前就失败的，仍然走重试」那条守的是走哪条分支，和退避时长毫无关系，却因为
// 依赖照抄生产常量而真睡一次 equal-jitter 的随机 1–2 秒，是整个套件最慢的一条，且每次
// 时长都不一样。退避曲线本身在 logic.test.mjs 里另有专测。
const resolveRealFn = async () => {
  // _modelEventHasProgress 在回调里被调用，它的 ReferenceError 会被吞成 attemptError
  // 字符串，探针看不见，所以直接点名。
  const need = ["_modelEventHasProgress", "_runModelRequestWithRetry"];
  const build = () => load("_runModelRequestWithRetry", need);
  for (let i = 0; i < 60; i++) {
    let fn;
    try { fn = build(); } catch (e) {
      const m = /^(\w+) is not defined$/.exec(e.message);
      if (!m) throw e;
      need.unshift(m[1]); continue;
    }
    try {
      // 探针一：走"出过内容 → 断线 → 判定能否重试/续传"这条分支
      await fn({
        invoke: (cb) => { cb({ kind: "token", delta: "p" }); cb({ kind: "error", message: "connection reset" }); return Promise.resolve(); },
        onEvent: () => {}, buildResumeInvoke: async () => null,
      });
      // 探针二：走"还没出字就失败 → 进重试分支"。这条和上面那条**不是同一段代码**——
      // 重试分支里有自己的依赖（比如两次重试之间的等待间隔），只探上面那条的话，
      // 缺的依赖会等到真正的测试跑起来才炸成 ReferenceError，而不是在这里被补上。
      // isLive 按**调用次数**放行，不能在 invoke 里直接置假：invoke 返回之后紧接着就有
      // 一个 `if (!isLive()) return`，那时候置假就直接返回了，重试分支根本进不去，
      // 探针等于白探。前三次放行（函数入口 / 循环顶 / invoke 之后），之后返回 false，
      // 让等待循环立刻退出——分支进得去，又不用真的干等一个间隔。
      let _probeCalls = 0;
      await fn({
        // 用和真实测试同一个错误文案：文案不可重试的话 canRetry 为假，分支同样不进。
        invoke: (cb) => { cb({ kind: "error", message: "fetch failed" }); return Promise.resolve(); },
        onEvent: () => {}, isLive: () => ++_probeCalls <= 3, retryLimit: 1,
      });
      return fn;
    } catch (e) {
      const m = /^(\w+) is not defined$/.exec(e.message);
      if (!m) throw e;
      need.unshift(m[1]);
    }
  }
  throw new Error("unresolved deps: " + need.join(","));
};
const run = await resolveRealFn();

// 一个会在出了 N 个 token 之后断线的假模型
const flaky = (failAfter, failures) => {
  let fails = 0;
  return (prefix = "") => (cb) => {
    for (let i = 0; i < failAfter; i++) cb({ kind: "token", delta: "x" });
    if (fails < failures) { fails++; cb({ kind: "error", message: "connection reset" }); return Promise.resolve(); }
    cb({ kind: "token", delta: "END" });
    cb({ kind: "done" });
    return Promise.resolve();
  };
};

test("断线后从断点续传，而不是报错收场", async () => {
  const f = flaky(3, 1);
  let resumeCalls = 0, sawText = "";
  const r = await run({
    invoke: f(),
    onEvent: (ev) => { if (ev.kind === "token") sawText += ev.delta; },
    buildResumeInvoke: async () => { resumeCalls++; return f(); },
  });
  assert.equal(r.error, "", "断线不该再作为最终错误抛出");
  assert.equal(resumeCalls, 1, "应当续传一次");
  assert.equal(r.resumes, 1);
  assert.ok(sawText.endsWith("END"), "续传后拿到了收尾内容");
});

test("续传不消耗重试配额", async () => {
  const f = flaky(2, 2);
  let retries = 0, resumes = 0;
  const r = await run({
    invoke: f(),
    onEvent: () => {},
    onRetry: () => { retries++; },
    buildResumeInvoke: async () => { resumes++; return f(); },
    retryLimit: 0,          // 一次重试都不给
  });
  assert.equal(retries, 0, "没有走重试");
  assert.equal(resumes, 2, "续传照常进行，不受 retryLimit 限制");
  assert.equal(r.error, "");
});

test("拒绝续传（例如工具参数流到一半）时，维持原来的报错行为", async () => {
  const f = flaky(3, 5);
  const r = await run({ invoke: f(), onEvent: () => {}, buildResumeInvoke: async () => null });
  assert.match(r.error, /connection reset/);
  assert.equal(r.resumes, 0);
});

test("续传次数有上限，不会无限续", async () => {
  const f = flaky(1, 99);
  let resumes = 0;
  const r = await run({
    invoke: f(),
    onEvent: () => {},
    buildResumeInvoke: async () => { resumes++; return f(); },
    resumeLimit: 3,
  });
  assert.equal(resumes, 3, "到达上限后停止");
  assert.match(r.error, /connection reset/);
});

// 用户实拍的 7 分 10 秒就是这条：桌面端停滞看门狗抛出「模型连续 90 秒没有继续生成有效
// 内容」，这句话又正好被 _isRetryableAiError 的最后一条正则认领，于是每次超时都触发一次
// 续传、每次续传又开一个全新的 90 秒空窗。三个名额烧完 = 4 × 90 秒，全程只产出四句
// "我这就开始写"，一个文件都没有。
//
// 掉线值得续（连接没了，重新接上多半就好）；停滞不值得续三次（上游正卡着不出字，
// 带同一份上下文再问一遍它大概率继续卡）。给 1 次机会，不给 3 次。
test("停滞不是掉线：最多只续一次，不把 90 秒空窗重复四遍", async () => {
  const stall = "模型连续 90 秒没有继续生成有效内容，已停止本轮，请重试。";
  const stallInvoke = (cb) => {
    cb({ kind: "token", delta: "我这就开始写" });
    cb({ kind: "error", message: stall });
    return Promise.resolve();
  };
  let resumes = 0;
  const r = await run({
    invoke: stallInvoke,
    onEvent: () => {},
    buildResumeInvoke: async () => { resumes++; return stallInvoke; },
    resumeLimit: 3,
  });
  assert.equal(resumes, 1, "停滞类只给一次续传机会——给三次就是把同一个空窗重复四遍");
  assert.match(r.error, /没有继续生成有效内容/);

  // 对照：真正的掉线仍然用满三个名额，这道闸不许误伤它。
  const dropInvoke = (cb) => {
    cb({ kind: "token", delta: "正文" });
    cb({ kind: "error", message: "连接中断（网络波动），已保留生成的部分。" });
    return Promise.resolve();
  };
  let dropResumes = 0;
  const r2 = await run({
    invoke: dropInvoke,
    onEvent: () => {},
    buildResumeInvoke: async () => { dropResumes++; return dropInvoke; },
    resumeLimit: 3,
  });
  assert.equal(dropResumes, 3, "掉线该续满——别把停滞那道闸开到掉线身上");
});

test("出字之前就失败的，仍然走重试（重放是安全的）", async () => {
  let n = 0;
  const invoke = (cb) => { n++; if (n === 1) { cb({ kind: "error", message: "fetch failed" }); return Promise.resolve(); } cb({ kind: "token", delta: "ok" }); cb({ kind: "done" }); return Promise.resolve(); };
  let retries = 0, resumes = 0;
  const r = await run({ invoke, onEvent: () => {}, onRetry: () => retries++, buildResumeInvoke: async () => { resumes++; return invoke; } });
  assert.equal(retries, 1, "没有内容出去过 → 重放安全，走重试");
  assert.equal(resumes, 0, "不该动用续传");
  assert.equal(r.error, "");
});

test("不可重试的错误不触发续传", async () => {
  const invoke = (cb) => { cb({ kind: "token", delta: "hi" }); cb({ kind: "error", message: "invalid api key" }); return Promise.resolve(); };
  let resumes = 0;
  const r = await run({ invoke, onEvent: () => {}, buildResumeInvoke: async () => { resumes++; return invoke; } });
  assert.equal(resumes, 0);
  assert.match(r.error, /invalid api key/);
});

test("续传的安全线是「是否已执行」，而非「是否有工具在途」（源码级守卫）", () => {
  // 这条守卫在 _agentModelTurn 里，那个函数太大没法抽出来单独跑，所以钉源码。
  // 旧行为：只要 byIndex.size>0（有任何工具调用在途）就拒绝续传——于是流式写大文件中途断线
  // 会把整轮报废（就是用户报的“输出中断，本轮不重放”）。真正危险的只有一种：eager write
  // （write_file 参数流完立刻落盘）**已经执行**——重发那次调用会二次写盘。而半截参数的调用
  // 其实没执行（JSON 没闭合、eager 没触发），丢弃即可，不必弃疗整轮。判据从此看“执行没执行”。
  // 锚点必须钉在 **agent** 那条分支上。indexOf 取到的是 chat 模式那条（它在文件里更靠前），
  // 而 chat 的 buildResumeInvoke 后面跟的是 onEvent 不是 onResume，于是切片一路跨过两条分支
  // ——下面每一条断言都可能是被另一条分支满足的，测试看着绿其实什么都没守住。
  // 用 agent 分支独有的 eagerExecuted 定位，再往回找它自己的那个 buildResumeInvoke。
  const _agentAt = SRC.indexOf("const eagerExecuted = [...byIndex.values()]");
  assert.ok(_agentAt > 0, "agent 那条续传分支不见了");
  const turn = SRC.slice(SRC.lastIndexOf("buildResumeInvoke: async ({ resume, resumeLimit })", _agentAt));
  const body = turn.slice(0, turn.indexOf("onResume:"));
  assert.ok(body.length > 200 && body.length < 6000, `切片大小不对（${body.length}）——多半又跨分支了`);
  assert.equal((body.match(/buildResumeInvoke:/g) || []).length, 1, "切片里只能有一条 buildResumeInvoke");
  assert.doesNotMatch(body, /if \(byIndex\.size > 0\) return null;/,
    "不再因为「有工具在途」就整轮弃疗");
  // 这条注释本来就写着"真的执行过"，钉住的却是 _eagerNotified —— 那个标记只表示钩子被叫过
  // 一次。钩子内部一串前置条件（意图未落定、有排队的插话、参数 parse 不过……）任何一条不满足
  // 就直接 return，一个字节都没写。真正代表落盘的是 _eagerDone，注释说的一直是它。
  assert.match(body, /const eagerExecuted = \[\.\.\.byIndex\.values\(\)\]\.some\(\(e\) => e && e\._eagerDone\)/,
    "停的判据是 eager write 真的落了盘（_eagerDone），不是钩子被叫过（_eagerNotified）");
  assert.match(SRC, /entry\._eagerDone = true;/, "_eagerDone 只在真正进入执行时才置位");
  assert.match(body, /if \(mode === "stop"\) return null;/,
    "只有 eager write 已落盘这一种情况才停下把决定权交回用户");
  assert.match(body, /byIndex\.clear\(\);/,
    "没执行过的半截工具调用直接丢弃，续传时由模型重新干净发出，不拿半截参数猜");
  // 原始消息原样带上（不重放），assistant prefill 压在**末位**——中间允许插续写指令，
  // 但末位必须是它，否则原生 Anthropic 线路上的 prefill 语义就没了。
  assert.match(body, /\.\.\._l0Msgs,[\s\S]{0,600}?\{ role: "assistant", content: partial \}\]/,
    "有正文时靠 prefill 续写，不重放提示");
  {
    const _ctor = body.slice(body.indexOf("[..._l0Msgs,"));
    const _end = _ctor.indexOf("];");
    assert.ok(_end > 0, "找不到 resumeMsgs 的结尾");
    const _b = _ctor.slice(0, _end);
    assert.ok(_b.lastIndexOf('role: "assistant"') > _b.lastIndexOf('role: "user"'),
      "assistant prefill 必须是最后一条");
  }
  // prefill 语义只在原生 Anthropic 线路成立。走 OpenAI 兼容透传时末条 assistant 是一条
  // 已完成的历史轮次，模型会回复它而不是接着写（实拍："明白，你在等我完成上一轮的任务"）。
  assert.match(body, /\[断点续写\]/,
    "续传必须显式要求接着写——只靠 prefill 约定，非 Anthropic 线路上模型会重新开场");
  assert.match(body, /replace\(\/\\s\+\$\/, ""\)/,
    "prefill 结尾不能留空白，否则上游直接 400");
});
