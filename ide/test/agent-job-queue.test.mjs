import test from "node:test";
import assert from "node:assert/strict";
import { JobQueue } from "../src/agent/job-queue.js";
import { SharedStore } from "../src/agent/shared-store.js";

function makeStore() {
  return new SharedStore({ defaultTTL: 60_000, cleanupInterval: 60_000 });
}

test("JobQueue keeps the caller job id and settles a real injected runner", async () => {
  const store = makeStore();
  const queue = new JobQueue({ sharedStore: store, historyTTL: 60_000, runner: async (_job, ctx) => {
    ctx.reportProgress({ progress: 50, finding: { type: "evidence", content: "read" } });
    return { summary: "done", findings: [{ type: "evidence" }], tokensUsed: 3 };
  } });

  const jobId = await queue.submit({ id: "job-fixed", tool: "run_subagent", role: "research", args: {} });
  const result = await queue.waitForJob(jobId, 1_000);

  assert.equal(jobId, "job-fixed");
  assert.equal(result.summary, "done");
  assert.equal(queue.jobs.get(jobId)?.status, "completed");
  assert.equal(store.get(`jobs.${jobId}`)?.status, "completed");
  assert.equal(store.get(`jobs.${jobId}`)?.findings?.length, 2);
  assert.equal(queue.stats().totalTokens, 3);
});

test("JobQueue refuses to fabricate an agent result when no runner is injected", async () => {
  const store = makeStore();
  const queue = new JobQueue({ sharedStore: store, historyTTL: 60_000 });
  const jobId = await queue.submit({ id: "job-no-runner", tool: "run_subagent", role: "research", args: {} });

  await assert.rejects(queue.waitForJob(jobId, 1_000), /injected runner/);
  assert.equal(queue.jobs.get(jobId)?.status, "failed");
  assert.equal(store.get(`jobs.${jobId}`)?.status, "failed");
  assert.match(store.get(`jobs.${jobId}`)?.findings?.at(-1)?.data || "", /injected runner/);
});

test("SharedStore job updates remain one coherent record and clear with queue history", async () => {
  const store = makeStore();
  const queue = new JobQueue({ sharedStore: store, historyTTL: 60_000, runner: async () => ({ summary: "ok" }) });
  const jobId = await queue.submit({ id: "job-clear", tool: "run_subagent", role: "test", args: {} });
  await queue.waitForJob(jobId, 1_000);

  assert.deepEqual(store.getActiveJobs(), []);
  assert.equal(queue.clearHistory(), 1);
  assert.equal(store.get(`jobs.${jobId}`), undefined);
});

test("SharedStore keeps job records coherent and subscriptions alive", () => {
  const store = makeStore();
  const findingCounts = [];
  store.on("jobs.alpha.findings", (findings) => findingCounts.push(findings.length));

  assert.equal(store.createJob({ role: "research" }, "alpha"), "alpha");
  store.updateJobStatus("alpha", "running", 20);
  store.appendFinding("alpha", { type: "fact", content: "first" });
  store.appendFinding("alpha", { type: "fact", content: "second" });

  const active = store.getActiveJobs();
  assert.equal(active.length, 1);
  assert.equal(active[0].jobId, "alpha");
  assert.equal(active[0].status, "running");
  assert.equal(active[0].progress, 20);
  assert.equal(active[0].findings.length, 2);
  assert.deepEqual(findingCounts, [1, 2]);

  store.updateJobStatus("alpha", "completed", 100);
  assert.equal(store.getActiveJobs().length, 0);
  assert.equal(store.get("jobs.alpha").status, "completed");
});

test("JobQueue uses the injected runner and preserves completed history for wait/resume", async () => {
  const store = makeStore();
  const progress = [];
  const queue = new JobQueue({
    maxConcurrent: 1,
    historyTTL: 0,
    sharedStore: store,
    runner: async (job, runtime) => {
      runtime.reportProgress({ progress: 50 });
      progress.push(job.id);
      await Promise.resolve();
      runtime.reportProgress({ progress: 75, finding: { type: "fact", content: "real runner" } });
      return { summary: "done", findings: [{ type: "fact" }], tokensUsed: 12 };
    },
  });

  const jobId = await queue.submit({ id: "real-1", tool: "run_subagent", role: "research" });
  const result = await queue.waitForJob(jobId, 1_000);
  assert.equal(result.summary, "done");
  assert.deepEqual(progress, ["real-1"]);
  assert.equal(queue.jobs.get(jobId).status, "completed");
  assert.equal(queue.stats().totalTokens, 12);
  assert.equal(store.get("jobs.real-1").status, "completed");
  assert.equal(store.get("jobs.real-1").findings.length, 2);
});
