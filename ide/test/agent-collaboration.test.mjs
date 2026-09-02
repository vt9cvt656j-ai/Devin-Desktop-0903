// 2026-08-25：删掉了三条测死代码的用例。
//
// shared_store 和 eventbus 两种模式、以及 extractFileSnippets 那一族，在生产里
// **零调用点**——唯一的 startSession 写死 lead_follower，enhanceContext 的
// filesToInclude 恒为空数组。它们连同实现一起删了。
//
// 这三条原来是全绿的，而它们测的正是产品不跑的那半：测试全绿不代表功能在跑，
// 这恰恰是 wiring.test.mjs 那条 KNOWN_DEAD 基线要防的事。

import test from "node:test";
import assert from "node:assert/strict";
import CollaborationEngine from "../src/agent/collaboration-engine.js";
import { SharedStore } from "../src/agent/shared-store.js";

function makeStore() {
  return new SharedStore({ defaultTTL: 60_000, cleanupInterval: 60_000 });
}

test("per-session mode: shared_store engine can start a lead_follower session", () => {
  const store = makeStore();
  store.createJob({ role: "main" }, "lead");
  store.createJob({ role: "research" }, "f1");
  store.createJob({ role: "security" }, "f2");
  const engine = new CollaborationEngine({ store, mode: "shared_store" });
  engine.startSession("lf-1", ["lead", "f1", "f2"], { mode: "lead_follower", leadJobId: "lead" });

  const session = engine.activeCollaborations.get("lf-1");
  assert.equal(session.mode, "lead_follower");

  const leadJob = store.get("jobs.lead");
  leadJob.decision = "use pattern X";
  leadJob.status = "phase_complete";
  leadJob.progress = 50;
  store.set("jobs.lead", leadJob);

  const f1 = store.get("jobs.f1");
  const f2 = store.get("jobs.f2");
  assert.ok(f1.findings.some((f) => f.channel === "lead_decision"), "follower f1 should receive lead decision");
  assert.ok(f2.findings.some((f) => f.channel === "lead_decision"), "follower f2 should receive lead decision");
  assert.equal(engine.endSession("lf-1"), true);
});

test("addSharedKnowledge propagates to all jobs in the session", () => {
  const store = makeStore();
  store.createJob({ role: "main" }, "m1");
  store.createJob({ role: "backend" }, "w1");
  const engine = new CollaborationEngine({ store });
  // 引擎现在只剩 lead_follower 一种模式，leadJobId 是必需的（生产那个调用点一直在传）。
  engine.startSession("sk-1", ["m1", "w1"], { mode: "lead_follower", leadJobId: "m1" });

  const ok = engine.addSharedKnowledge("sk-1", "db_schema", "users table has role column");
  assert.equal(ok, true);

  const session = engine.activeCollaborations.get("sk-1");
  assert.equal(session.knowledge.db_schema, "users table has role column");

  const w1 = store.get("jobs.w1");
  assert.ok(w1.findings.some((f) => f.channel === "knowledge_update" && f.data?.key === "db_schema"));
  engine.endSession("sk-1");
});

test("enhanceContext includes shared knowledge from active sessions", async () => {
  const store = makeStore();
  store.createJob({ role: "main" }, "ctx_m");
  store.createJob({ role: "research" }, "ctx_r");
  const engine = new CollaborationEngine({ store, mode: "shared_store" });
  engine.startSession("ctx-1", ["ctx_m", "ctx_r"], { mode: "lead_follower", leadJobId: "ctx_m" });
  engine.addSharedKnowledge("ctx-1", "api_url", "https://example.com/api");

  const enhanced = await engine.enhanceContext("ctx_r", { task: "investigate" }, []);
  assert.ok(enhanced.sharedKnowledge, "enhanced context should include shared knowledge");
  const flat = JSON.stringify(enhanced.sharedKnowledge);
  assert.ok(flat.includes("https://example.com/api"));
  engine.endSession("ctx-1");
});

/**
 * 同伴发现不能跨 run 串味。
 *
 * SharedStore 是**全局**的、跨 run 也跨标签页：上一轮任务的子体、另一个项目窗口里正在
 * 跑的子体，findings 全躺在同一块黑板上。原来 collectRelatedFindings 无差别扫 `jobs.*`，
 * 于是新派的子智能体一开工就被喂进一段「其他角色已发现：…」，内容却来自一个它从来
 * 没参与过的任务——它会把那些结论当成本次调查的既有证据接着往下推。
 *
 * 键的形状是 `sm_<runToken>_<jobId>`，同一次派发的同伴共享那个前缀。
 */
test("别的 run 的发现不会被当成同伴发现喂进来", async () => {
  const store = new SharedStore();
  const engine = new CollaborationEngine({ store });

  // 本次派发：run A 的两个子体
  store.set("jobs.sm_A_1", { findings: [] });
  store.set("jobs.sm_A_2", { findings: [] });
  store.appendFinding("sm_A_2", { content: "同伴的发现·应该看得到", type: "finding" });
  // 另一个 run（可能是另一个标签页、另一个项目）
  store.set("jobs.sm_B_9", { findings: [] });
  store.appendFinding("sm_B_9", { content: "别人的发现·绝不该出现", type: "finding" });

  const got = await engine.collectRelatedFindings("sm_A_1");
  const texts = got.map((f) => String(f.content || ""));
  assert.ok(
    texts.some((t) => t.includes("同伴的发现")),
    `同一次派发的同伴发现应该收得到，实际拿到：${JSON.stringify(texts)}`,
  );
  assert.ok(
    !texts.some((t) => t.includes("别人的发现")),
    `别的 run 的发现串进来了：${JSON.stringify(texts)}——子体会把它当成本次调查的既有证据`,
  );
});

/** 上下文超预算时的降级不该反着损：降序数组要取头部，取末尾等于只留最旧的。 */
test("上下文超预算降级时，留下的是最新的发现不是最旧的", async () => {
  const store = new SharedStore();
  // maxContextSize 压到很小，强制走裁剪分支
  const engine = new CollaborationEngine({ store, config: { maxContextSize: 400 } });
  store.set("jobs.sm_A_1", { findings: [] });
  store.set("jobs.sm_A_2", { findings: [] });
  for (let i = 1; i <= 12; i++) {
    store.appendFinding("sm_A_2", { content: `发现编号 ${i}`, type: "finding" });
  }
  const enhanced = await engine.enhanceContext("sm_A_1", {}, []);
  const kept = (enhanced.relatedFindings || []).map((f) => String(f.content || ""));
  assert.ok(kept.length, "裁剪之后不该一条都不剩");
  assert.ok(
    kept.some((t) => /发现编号 1[12]/.test(t)),
    `裁剪后留下的应该是最新的几条，实际留下：${JSON.stringify(kept)}`,
  );
  assert.ok(
    !kept.some((t) => /发现编号 [12]$/.test(t)),
    `裁剪后还留着最早的几条——降序数组取了末尾：${JSON.stringify(kept)}`,
  );
});
