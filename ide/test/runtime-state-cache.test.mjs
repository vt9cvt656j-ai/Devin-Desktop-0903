// 运行状态块的缓存键不能叠终端的活动时间戳。
//
// 这一块（工作目录 / 终端 / git 现场 / 后端线索）每轮重扫整个项目：两次读文件、8 个 marker
// 逐个探存在、13 个路径各探目录+文件、再加一趟 depth-3 全树 walk，本仓库实测约 116 次串行
// IPC，最坏 1.6 秒被超时兜底整轮吃掉。所以它按「工作区变更 tick」做了缓存。
//
// 键里另一半叠的是终端的 lastActivityAt——而 PTY 的每一个输出块都会把它刷成 Date.now()。
// 只要工作区里有一个活着的 dev server 在打日志（Vite / express / watcher，模型用 browser
// 验页面时服务终端还会刷请求日志），两轮之间键必然不同，缓存永远不命中，这一块每轮照旧全量
// 重扫。而「起 dev server → 改 → 验」正是最常见的工作形态：缓存在最需要它的场景里整体失效。
//
// 另一半后果：重扫超过 1600ms 时兜底值 "" 会连同新键一起写进缓存，模型这一整轮拿不到执行
// 状态块，下一轮键再变又重扫——环境事实在「完整」和「空」之间来回翻。
//
// 修法：键里的终端分量换成「这一块真会渲染出来的终端事实」的签名；超时不写缓存。
import test from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, SRC as RAW_SRC, load } from "./helpers/source.mjs";

function stripJsComments(source) {
  return source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|[^:])\/\/[^\n]*/g, "$1");
}

/** _agentTerminalEntries() 的一条返回值的形状。 */
function entry(over = {}) {
  return {
    index: 0, label: "任务终端", task: true, status: "运行中",
    cwd: "/w/app", command: "npm run dev", recent: "vite ready in 300ms\n",
    urls: ["http://localhost:5173"], lastActivityAt: 1_700_000_000_000,
    ...over,
  };
}

const signatureOf = (entries) => load("_agentTerminalStateSignature", {
  _agentTerminalEntries: () => entries,
  _resultFingerprint: load("_resultFingerprint"),
})();

test("活着的服务一直刷日志，签名不动——这一轮不再白重扫一遍项目", () => {
  const before = signatureOf([entry()]);
  // 同一个终端，又滚出几百行请求日志，lastActivityAt 跟着刷新。
  const after = signatureOf([entry({
    recent: "vite ready in 300ms\n" + "GET /api/x 200 3ms\n".repeat(400),
    lastActivityAt: 1_700_000_090_000,
  })]);
  assert.equal(before, after, "运行中终端刷日志不改变这一块渲染出来的任何一个字");
});

test("服务崩了要立刻重建：状态一变签名就变", () => {
  const alive = signatureOf([entry()]);
  const dead = signatureOf([entry({ status: "已退出", recent: "vite ready\nError: EADDRINUSE\n" })]);
  assert.notEqual(alive, dead, "进程死了还给模型看「运行中」的旧快照，正是这条缓存的原罪");
  // 状态本身就是块里渲染的一个字段（`#1 · [运行中] · …`），不能只靠「已退出才附输出尾巴」
  // 间接兜住：启动中 → 运行中 这一跳两边都没有尾巴，签名里没有 status 就完全看不见。
  const booting = signatureOf([entry({ status: "启动中", recent: "", urls: [] })]);
  const running = signatureOf([entry({ status: "运行中", recent: "", urls: [] })]);
  assert.notEqual(booting, running, "签名里必须直接带 status");
});

test("已退出终端的输出尾巴进签名——同为已退出但报错内容变了也要重建", () => {
  const a = signatureOf([entry({ status: "已退出", recent: "Error: EADDRINUSE\n" })]);
  const b = signatureOf([entry({ status: "已退出", recent: "Error: cannot find module 'x'\n" })]);
  assert.notEqual(a, b, "块里贴的就是这 600 字尾巴，它变了缓存必须失效");
});

test("新服务起来（换命令 / 新 URL / 新终端）都会重建", () => {
  const base = signatureOf([entry()]);
  assert.notEqual(base, signatureOf([entry({ command: "npm run preview" })]));
  assert.notEqual(base, signatureOf([entry({ urls: ["http://localhost:5173", "http://localhost:8787"] })]));
  assert.notEqual(base, signatureOf([entry(), entry({ index: 1, label: "普通终端", task: false, command: "psql" })]));
  assert.notEqual(base, signatureOf([entry({ cwd: "/w/other" })]));
  assert.notEqual(base, signatureOf([]));
});

test("运行中终端的输出只通过 URL 影响这一块，所以新打出来的 URL 要重建", () => {
  const before = signatureOf([entry({ urls: [] })]);
  const after = signatureOf([entry({ urls: ["http://127.0.0.1:3000"] })]);
  assert.notEqual(before, after);
});

test("终端面板还没就绪时不抛出去", () => {
  const sig = load("_agentTerminalStateSignature", {
    _agentTerminalEntries: () => { throw new Error("termTabs not ready"); },
    _resultFingerprint: load("_resultFingerprint"),
  });
  assert.equal(typeof sig(), "string");
});

test("缓存键用的是签名，不是活动时间戳", () => {
  const at = RAW_SRC.indexOf("const _fsTickNow = ");
  assert.ok(at > 0, "运行状态块的缓存没了");
  const block = stripJsComments(SRC.slice(at - 400, at + 400));
  const keyExpr = /const _fsTickNow = ([^;]+);/.exec(block);
  assert.ok(keyExpr, "找不到缓存键的赋值");
  assert.match(keyExpr[1], /run\._fsMutTick/, "文件变更信号还得在");
  assert.match(keyExpr[1], /_termTick/, "终端信号还得在");
  assert.match(block, /_termTick = _agentTerminalStateSignature\(\)/,
    "终端分量必须是渲染事实的签名");
  assert.doesNotMatch(block, /lastActivityAt/,
    "活动时间戳回到键里 = 有 dev server 时缓存永远不命中");
});

test("超时兜底不写进缓存：不把一次超时固化成整轮无执行状态", () => {
  const at = RAW_SRC.indexOf("const _fsTickNow = ");
  const block = stripJsComments(SRC.slice(at, at + 900));
  assert.match(block, /_promiseOrFallbackWithin\(_agentRuntimeStateBlock\(root\), 1600, null\)/,
    "兜底值必须是可辨认的 null，不是会被当成「本轮没有状态块」的空串");
  const guarded = /if \(_rtFresh != null\) \{\s*run\._rtState = _rtFresh;\s*run\._rtStateTick = _fsTickNow;\s*\}/;
  assert.match(block, guarded, "只有真拿到新快照才更新缓存与键；超时就留着上一份、键不推进");
  assert.doesNotMatch(
    block.replace(/if \(run\._rtStateTick[\s\S]*?\n      \}/, ""),
    /_promiseOrFallbackWithin\(_agentRuntimeStateBlock/,
    "缓存外面还留着一次无条件重扫",
  );
});

test("签名只取块里真会渲染的字段", () => {
  const fn = SRC.slice(
    RAW_SRC.indexOf("function _agentTerminalStateSignature("),
    RAW_SRC.indexOf("function _terminalLogChunks("),
  );
  for (const field of ["it.index", "it.status", "it.label", "it.command", "it.cwd", "it.urls"]) {
    assert.ok(fn.includes(field), `签名漏了 ${field}——这一块会把它渲染给模型`);
  }
  assert.match(fn, /it\.status === "已退出"[\s\S]{0,160}slice\(-600\)/,
    "只有已退出终端的 600 字尾巴进块，也只有它该进签名");
});
