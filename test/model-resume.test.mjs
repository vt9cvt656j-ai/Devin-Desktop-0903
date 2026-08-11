// 断线续传：连接在出字过程中断掉时，应当从断点接着写，而不是把整轮判死。
// 这里直接从 src/main.js 里取真函数来跑（acorn 定位），不复刻逻辑——复刻出来的
// 测试只能证明复制品是对的。
import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs";
import * as acorn from "acorn";

const SRC = fs.readFileSync("src/main.js", "utf8");
const ast = acorn.parse(SRC, { ecmaVersion: "latest", sourceType: "module" });
function grab(name) {
  for (const n of ast.body) {
    if (n.type === "FunctionDeclaration" && n.id?.name === name) return SRC.slice(n.start, n.end);
    if (n.type === "VariableDeclaration")
      for (const d of n.declarations)
        if (d.id?.name === name && d.init) return "const " + name + " = " + SRC.slice(d.init.start, d.init.end) + ";";
  }
  throw new Error("missing " + name);
}
// 依赖既可能在构造时缺，也可能在**调用时**才缺（默认参数、函数体里的调用）。
// 所以既在构造时补，也在一次真实调用里补，直到不再抛 ReferenceError。
const load = async () => {
  // _modelEventHasProgress 在回调里被调用，它的 ReferenceError 会被吞成 attemptError
  // 字符串，探针看不见，所以直接点名。
  const need = ["_modelEventHasProgress", "_runModelRequestWithRetry"];
  const build = () => new Function(need.map(grab).join("\n") + "\nreturn _runModelRequestWithRetry;")();
  for (let i = 0; i < 60; i++) {
    let fn;
    try { fn = build(); } catch (e) {
      const m = /^(\w+) is not defined$/.exec(e.message);
      if (!m) throw e;
      need.unshift(m[1]); continue;
    }
    try {
      // 探针：走一遍"出过内容 → 断线 → 判定能否重试/续传"的完整分支
      await fn({
        invoke: (cb) => { cb({ kind: "token", delta: "p" }); cb({ kind: "error", message: "connection reset" }); return Promise.resolve(); },
        onEvent: () => {}, buildResumeInvoke: async () => null,
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
const run = await load();

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
  const turn = SRC.slice(SRC.indexOf("buildResumeInvoke: async ({ resume, resumeLimit })"));
  const body = turn.slice(0, turn.indexOf("onResume:"));
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
  assert.match(body, /\.\.\._l0Msgs, \{ role: "assistant", content: partial \}/,
    "有正文时靠 prefill 续写，不重放提示");
  assert.match(body, /replace\(\/\\s\+\$\/, ""\)/,
    "prefill 结尾不能留空白，否则上游直接 400");
});
