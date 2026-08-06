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
