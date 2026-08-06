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
