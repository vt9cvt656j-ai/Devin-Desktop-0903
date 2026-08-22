// 主智能体 ↔ 子智能体的对齐。用 src/main.js 里的真函数 + 真 SharedStore 跑，不复刻逻辑。
import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs";
import { SharedStore } from "../src/agent/shared-store.js";
// 按名字取真源码只有一份实现：test/helpers/source.mjs 的 fnSource（acorn 按 AST 边界切）。
import { fnSource as grab } from "./helpers/source.mjs";
// 黑板键带 run 前缀（_smRunToken）：jobId 是 run 内的编号，而 SharedStore 是全局的，
// 不带前缀两个标签页的 job#1 会写进同一条记录。这里注入真实实现，不用桩——
// 键怎么拼正是这组测试要守的东西。
const { broadcast, drain, runToken } = new Function(
  "_globalSharedStore",
  "let _smRunTokenSeq = 0;\n" + grab("_smRunToken") + "\n" +
  grab("_broadcastMainAgentFinding") + "\n" + grab("_drainSubAgentCollaborationInbox") +
  "\nreturn { broadcast: _broadcastMainAgentFinding, drain: _drainSubAgentCollaborationInbox, runToken: _smRunToken };",
)(null);

// 造一个 run，并按**真实的键**把它的作业登记进黑板。run.key(id) 给测试用来读同一条。
const mkRun = (store, jobs) => {
  const run = { _subAgentJobs: new Map() };
  const token = runToken(run);
  run.key = (id) => `${token}_${id}`;
  for (const [id, status] of jobs) {
    run._subAgentJobs.set(id, { status });
    store.set(`jobs.sm_${run.key(id)}`, { status, findings: [] });
  }
  return run;
};

test("主智能体的发现会送达每一个还在跑的子智能体", () => {
  const store = new SharedStore();
  const run = mkRun(store, [["1", "running"], ["2", "running"]]);
  const sent = broadcast(run, "主智能体刚改了 src/a.ts", store);
  assert.equal(sent, 2, "两个在跑的子智能体都该收到");
  for (const id of ["1", "2"]) {
    const got = drain(store, run.key(id), 0);
    assert.match(got.message, /主智能体/, `子智能体 ${id} 的收件箱里要有主智能体这条`);
    assert.match(got.message, /src\/a\.ts/);
  }
});

test("已经落定的子智能体不再投递——它读不到了，投了只是白占位", () => {
  const store = new SharedStore();
  const run = mkRun(store, [["1", "running"], ["2", "completed"], ["3", "failed"]]);
  assert.equal(broadcast(run, "改了 src/b.ts", store), 1, "只有仍在跑的那个该收到");
  assert.equal(drain(store, run.key("2"), 0).message, "", "已完成的作业收件箱应为空");
});

test("游标只推进，不重复喂同一条", () => {
  const store = new SharedStore();
  const run = mkRun(store, [["1", "running"]]);
  broadcast(run, "第一条", store);
  const first = drain(store, run.key("1"), 0);
  assert.match(first.message, /第一条/);
  const again = drain(store, run.key("1"), first.cursor);
  assert.equal(again.message, "", "同一条不该被第二次读出来");
  broadcast(run, "第二条", store);
  const third = drain(store, run.key("1"), again.cursor);
  assert.match(third.message, /第二条/, "新的一条要能读到");
  assert.doesNotMatch(third.message, /第一条/, "旧的不该混进来");
});

test("没有子智能体在跑时是彻底的空操作", () => {
  const store = new SharedStore();
  assert.equal(broadcast({ _subAgentJobs: new Map() }, "x", store), 0);
  assert.equal(broadcast(null, "x", store), 0);
  assert.equal(broadcast({ _subAgentJobs: new Map([["1", { status: "running" }]]) }, "   ", store), 0,
    "空白内容不该占用同伴的注意力");
});

test("同伴与主智能体的消息共用一个收件箱，抬头不再谎称只来自同伴", () => {
  const store = new SharedStore();
  const run = mkRun(store, [["9", "running"]]);
  store.appendFinding(`sm_${run.key("9")}`, { source: "sm_4", channel: "collaboration", content: "同伴发现", isExternal: true });
  broadcast(run, "主智能体发现", store);
  const got = drain(store, run.key("9"), 0);
  assert.match(got.message, /同伴发现/);
  assert.match(got.message, /主智能体发现/);
  assert.doesNotMatch(got.message, /来自同批其他角色；/,
    "抬头写死成'只来自同伴'会让模型误判主智能体那条的权重");
});

test("多角色会诊：两份以上有内容的报告才归总，且归总不冒充证据", () => {
  const SRCX = fs.readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
  const i = SRCX.indexOf("const _settled = _targets.filter");
  assert.ok(i > 0, "await_subagent 里必须有会诊归总这一段");
  const seg = SRCX.slice(i, i + 3000);

  assert.match(seg, /_settled\.length >= 2/, "只有一份报告没什么可调和的，不该多花一次模型调用");
  assert.match(seg, /length > 80/, "空壳报告不参与归总");
  assert.match(seg, /\^\\\[\(\?:ERROR\|BLOCKED\|interrupted\)/, "报错的报告不该被当成一方意见");
  assert.match(seg, /run\._panelConfig && run\._panelBody/,
    "await 执行器拿不到 config/body，必须走派发时挂到 run 上的那份");
  assert.match(seg, /互相冲突的地方/, "归总必须先摆冲突——这正是主智能体自己做不好的部分");
  assert.match(seg, /没人覆盖到的缺口/, "缺口要点名，否则没人知道漏了什么");
  assert.match(seg, /以上是归总，不是新证据/,
    "归总是二手的：必须声明它不能盖过原始报告里的证据，否则模型会拿它当事实");
  // 守的是「追加而不是替换」，不是那个具体数字：原来写死 3200，而每份报告的预算后来
  // 对齐到了同步路的 8000（子体简报被压成一行 + 裁到 1200 字，是主 run 白烧轮次里最贵
  // 的一种）。总额小于单份预算的话，放大就白做了——所以下限跟着单份预算一起断言。
  const tail = /_parts\.join\("\\n"\)\)\.slice\(0, (\d+)\) \+ _panel/.exec(seg);
  assert.ok(tail, "原始报告必须仍然完整送达，归总是追加而不是替换");
  assert.ok(Number(tail[1]) >= 8000,
    `原始报告总额只有 ${tail[1]} 字——比单份报告的预算还小，等于又把报告砍回摘要`);
});

test("会诊归总按角色署名，而不是按任务描述", () => {
  const SRCX = fs.readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
  assert.match(SRCX, /const job = \{ id: jobId, desc, role: spec\.role,/,
    "作业要记住自己的角色，归总里才能写成「后端说 X、安全说 Y」");
  assert.match(SRCX.slice(SRCX.indexOf("const _settled = _targets.filter")), /j\.role \|\| "专家"/,
    "没有角色时要有兜底称呼");
});

test("两个 run 的同号作业不串台——黑板是全局的，jobId 不是", () => {
  // 这是没有 run 前缀时真会发生的事：A 标签页的 job#1 和 B 标签页的 job#1 写同一条记录，
  // 于是 A 的主智能体讲的话进了 B 的子智能体的收件箱——串台，也是跨项目的内容泄漏。
  const store = new SharedStore();
  const a = mkRun(store, [["1", "running"]]);
  const b = mkRun(store, [["1", "running"]]);
  assert.notEqual(a.key("1"), b.key("1"));
  broadcast(a, "A 项目改了 src/a.ts", store);
  assert.match(drain(store, a.key("1"), 0).message, /A 项目/);
  assert.equal(drain(store, b.key("1"), 0).message, "", "B 的子智能体读到了 A 的内容");
});
