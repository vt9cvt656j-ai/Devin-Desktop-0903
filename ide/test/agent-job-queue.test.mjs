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

test("JobQueue enforces maxConcurrent without polling races and starts queued jobs FIFO", async () => {
  const store = makeStore();
  const releases = new Map();
  const started = [];
  let running = 0;
  let peak = 0;
  const queue = new JobQueue({
    maxConcurrent: 2,
    historyTTL: 0,
    sharedStore: store,
    runner: async (job) => {
      started.push(job.id);
      running++;
      peak = Math.max(peak, running);
      await new Promise((resolve) => releases.set(job.id, resolve));
      running--;
      return { summary: job.id };
    },
  });

  for (let i = 1; i <= 5; i++) {
    await queue.submit({ id: `queued-${i}`, tool: "run_subagent", role: "test" });
  }
  await Promise.resolve();
  assert.deepEqual(started, ["queued-1", "queued-2"]);

  releases.get("queued-1")();
  await queue.waitForJob("queued-1", 1_000);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(started, ["queued-1", "queued-2", "queued-3"]);

  releases.get("queued-2")();
  releases.get("queued-3")();
  await Promise.all([queue.waitForJob("queued-2", 1_000), queue.waitForJob("queued-3", 1_000)]);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(started, ["queued-1", "queued-2", "queued-3", "queued-4", "queued-5"]);
  assert.equal(peak, 2);

  releases.get("queued-4")();
  releases.get("queued-5")();
  await Promise.all([queue.waitForJob("queued-4", 1_000), queue.waitForJob("queued-5", 1_000)]);
  await queue.stopAll();
});

test("JobQueue stopAll aborts queued jobs without leaving slot polling timers", async () => {
  const store = makeStore();
  let releaseRunning;
  let executions = 0;
  const queue = new JobQueue({
    maxConcurrent: 1,
    historyTTL: 0,
    sharedStore: store,
    runner: async () => {
      executions++;
      await new Promise((resolve) => { releaseRunning = resolve; });
      return { summary: "finished" };
    },
  });
  await queue.submit({ id: "running", tool: "run_subagent", role: "test" });
  await queue.submit({ id: "never-started", tool: "run_subagent", role: "test" });

  await queue.stopAll();
  assert.equal(queue.jobs.get("never-started").status, "aborted");
  assert.equal(queue.pendingJobs.length, 0);
  assert.equal(queue.monitorTimer, null);
  assert.equal(executions, 1);

  releaseRunning();
  await assert.rejects(queue.waitForJob("running", 1_000), /aborted/);
  assert.equal(queue.activeCount, 0);
});

test("JobQueue abort releases a slot even when the runner ignores AbortSignal", async () => {
  const store = makeStore();
  const started = [];
  const queue = new JobQueue({
    maxConcurrent: 1,
    historyTTL: 0,
    sharedStore: store,
    runner: async (job) => {
      started.push(job.id);
      if (job.id === "stuck") return new Promise(() => {});
      return { summary: "next completed" };
    },
  });

  await queue.submit({ id: "stuck", tool: "run_subagent", role: "test" });
  await queue.submit({ id: "next", tool: "run_subagent", role: "test" });
  await Promise.resolve();
  assert.deepEqual(started, ["stuck"]);

  assert.equal(await queue.abort("stuck"), true);
  await assert.rejects(queue.waitForJob("stuck", 1_000), /aborted/);
  assert.equal((await queue.waitForJob("next", 1_000)).summary, "next completed");
  assert.deepEqual(started, ["stuck", "next"]);
  assert.equal(queue.activeCount, 0);
  await queue.stopAll();
});

test("JobQueue stopAll completes when every running runner ignores cancellation", async () => {
  const store = makeStore();
  const queue = new JobQueue({
    maxConcurrent: 2,
    historyTTL: 0,
    sharedStore: store,
    runner: async () => new Promise(() => {}),
  });
  await queue.submit({ id: "stuck-a", tool: "run_subagent", role: "test" });
  await queue.submit({ id: "stuck-b", tool: "run_subagent", role: "test" });
  await Promise.resolve();

  await Promise.race([
    queue.stopAll(),
    new Promise((_, reject) => setTimeout(() => reject(new Error("stopAll hung")), 250)),
  ]);

  assert.equal(queue.activeCount, 0);
  assert.equal(queue.jobs.get("stuck-a").status, "aborted");
  assert.equal(queue.jobs.get("stuck-b").status, "aborted");
});
