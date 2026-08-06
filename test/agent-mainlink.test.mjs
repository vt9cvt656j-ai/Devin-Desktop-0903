// 主智能体 ↔ 子智能体的对齐。用 src/main.js 里的真函数 + 真 SharedStore 跑，不复刻逻辑。
import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs";
import * as acorn from "acorn";
import { SharedStore } from "../src/agent/shared-store.js";

const SRC = fs.readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
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
const { broadcast, drain } = new Function(
  "_globalSharedStore",
  grab("_broadcastMainAgentFinding") + "\n" + grab("_drainSubAgentCollaborationInbox") +
  "\nreturn { broadcast: _broadcastMainAgentFinding, drain: _drainSubAgentCollaborationInbox };",
)(null);

const mkRun = (store, jobs) => {
  const m = new Map();
  for (const [id, status] of jobs) {
    m.set(id, { status });
    store.set(`jobs.sm_${id}`, { status, findings: [] });
  }
  return { _subAgentJobs: m };
};

test("主智能体的发现会送达每一个还在跑的子智能体", () => {
  const store = new SharedStore();
  const run = mkRun(store, [["1", "running"], ["2", "running"]]);
  const sent = broadcast(run, "主智能体刚改了 src/a.ts", store);
  assert.equal(sent, 2, "两个在跑的子智能体都该收到");
  for (const id of ["1", "2"]) {
    const got = drain(store, id, 0);
    assert.match(got.message, /主智能体/, `子智能体 ${id} 的收件箱里要有主智能体这条`);
    assert.match(got.message, /src\/a\.ts/);
  }
});

test("已经落定的子智能体不再投递——它读不到了，投了只是白占位", () => {
  const store = new SharedStore();
  const run = mkRun(store, [["1", "running"], ["2", "completed"], ["3", "failed"]]);
  assert.equal(broadcast(run, "改了 src/b.ts", store), 1, "只有仍在跑的那个该收到");
  assert.equal(drain(store, "2", 0).message, "", "已完成的作业收件箱应为空");
});

test("游标只推进，不重复喂同一条", () => {
  const store = new SharedStore();
  const run = mkRun(store, [["1", "running"]]);
  broadcast(run, "第一条", store);
  const first = drain(store, "1", 0);
  assert.match(first.message, /第一条/);
  const again = drain(store, "1", first.cursor);
  assert.equal(again.message, "", "同一条不该被第二次读出来");
  broadcast(run, "第二条", store);
  const third = drain(store, "1", again.cursor);
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
  store.set("jobs.sm_9", { status: "running", findings: [] });
  store.appendFinding("sm_9", { source: "sm_4", channel: "collaboration", content: "同伴发现", isExternal: true });
  const run = { _subAgentJobs: new Map([["9", { status: "running" }]]) };
  broadcast(run, "主智能体发现", store);
  const got = drain(store, "9", 0);
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
  assert.match(seg, /_parts\.join\("\\n"\)\)\.slice\(0, 3200\) \+ _panel/,
    "原始报告必须仍然完整送达，归总是追加而不是替换");
});

test("会诊归总按角色署名，而不是按任务描述", () => {
  const SRCX = fs.readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
  assert.match(SRCX, /const job = \{ id: jobId, desc, role: spec\.role,/,
    "作业要记住自己的角色，归总里才能写成「后端说 X、安全说 Y」");
  assert.match(SRCX.slice(SRCX.indexOf("const _settled = _targets.filter")), /j\.role \|\| "专家"/,
    "没有角色时要有兜底称呼");
});
