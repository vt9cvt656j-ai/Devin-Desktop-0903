import assert from "node:assert/strict";
import test from "node:test";

import { SharedStore } from "../src/agent/shared-store.js";

function makeStore() {
  return new SharedStore({ defaultTTL: 60_000, cleanupInterval: 60_000 });
}

test("deleting a coherent record releases descendant subscriptions", () => {
  const store = makeStore();
  store.set("jobs.alpha", { status: "running", findings: [] });
  store.on("jobs.alpha", () => {});
  store.on("jobs.alpha.findings", () => {});
  store.on("jobs.alpha.*", () => {});
  store.on("jobs.alphabet.findings", () => {});

  store.delete("jobs.alpha");

  assert.equal(store.listenerCount(), 1);
  assert.deepEqual([...store.listeners.keys()], ["jobs.alphabet.findings"]);
});

test("TTL cleanup releases child listeners even when only the base key stores data", () => {
  const store = makeStore();
  store.set("jobs.expired", { status: "completed", findings: [] }, -1);
  store.on("jobs.expired.findings", () => {});

  assert.equal(store.cleanupExpired(), 1);
  assert.equal(store.listenerCount(), 0);
});

test("has observes TTL instead of exposing an expired in-memory record", () => {
  const store = makeStore();
  store.set("jobs.expired", { status: "completed" }, -1);
  store.on("jobs.expired.findings", () => {});

  assert.equal(store.has("jobs.expired"), false);
  assert.equal(store.listenerCount(), 0);
});

test("LRU eviction releases descendant listeners for the evicted base record", () => {
  const store = new SharedStore({ maxEntries: 1, defaultTTL: 60_000, cleanupInterval: 60_000 });
  store.set("jobs.old", { status: "completed" });
  store.on("jobs.old.findings", () => {});
  store.on("jobs.new.findings", () => {});

  store.set("jobs.new", { status: "running" });

  assert.equal(store.has("jobs.old"), false);
  assert.equal(store.listenerCount(), 1);
  assert.deepEqual([...store.listeners.keys()], ["jobs.new.findings"]);
});
