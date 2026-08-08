import test from "node:test";
import assert from "node:assert/strict";
import CollaborationEngine from "../src/agent/collaboration-engine.js";
import { SharedStore } from "../src/agent/shared-store.js";

function makeStore() {
  return new SharedStore({ defaultTTL: 60_000, cleanupInterval: 60_000 });
}

test("CollaborationEngine reads real workspace snippets through its injected reader", async () => {
  const store = makeStore();
  const reads = [];
  const engine = new CollaborationEngine({
    store,
    readFile: async (path) => {
      reads.push(path);
      return "export function ready() { return true; }\n";
    },
    config: { maxContextSize: 8_000, fileSnippetsCount: 3 },
  });

  const context = await engine.enhanceContext("alpha", { task: "inspect" }, ["src/app.js"]);
  assert.deepEqual(reads, ["src/app.js"]);
  assert.equal(context.fileSnippets[0].path, "src/app.js");
  assert.match(context.fileSnippets[0].content, /function ready/);
});

test("shared-store collaboration broadcasts each local finding once without feedback loops", () => {
  const store = makeStore();
  store.createJob({ role: "research" }, "alpha");
  store.createJob({ role: "test" }, "beta");
  const engine = new CollaborationEngine({ store, mode: "shared_store" });
  engine.startSession("session-1", ["alpha", "beta"]);

  store.appendFinding("alpha", { type: "fact", content: "real evidence" });

  const alpha = store.get("jobs.alpha").findings;
  const beta = store.get("jobs.beta").findings;
  assert.equal(alpha.length, 1);
  assert.equal(beta.length, 1);
  assert.equal(beta[0].source, "alpha");
  assert.equal(beta[0].isExternal, true);
  assert.equal(engine.endSession("session-1"), true);

  store.appendFinding("alpha", { type: "fact", content: "after close" });
  assert.equal(store.get("jobs.beta").findings.length, 1, "closed sessions must release listeners");
});

test("eventbus collaboration delivers events to peer inboxes and reports the session id", () => {
  const store = makeStore();
  const engine = new CollaborationEngine({ store, mode: "eventbus" });
  engine.startSession("session-2", ["lead", "peer"]);
  const received = [];
  store.on("collab:session-2:peer", (event) => received.push(event));

  engine.activeCollaborations.get("session-2").publish("lead", "decision", { value: 42 });

  assert.equal(received.length, 1);
  assert.equal(received[0].sourceJobId, "lead");
  assert.deepEqual(received[0].payload, { value: 42 });
  assert.equal(engine.getActiveSessions()[0].sessionId, "session-2");
});

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
  const engine = new CollaborationEngine({ store, mode: "shared_store" });
  engine.startSession("sk-1", ["m1", "w1"]);

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
  engine.startSession("ctx-1", ["ctx_m", "ctx_r"]);
  engine.addSharedKnowledge("ctx-1", "api_url", "https://example.com/api");

  const enhanced = await engine.enhanceContext("ctx_r", { task: "investigate" }, []);
  assert.ok(enhanced.sharedKnowledge, "enhanced context should include shared knowledge");
  const flat = JSON.stringify(enhanced.sharedKnowledge);
  assert.ok(flat.includes("https://example.com/api"));
  engine.endSession("ctx-1");
});
