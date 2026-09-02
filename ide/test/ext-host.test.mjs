import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import assert from "node:assert/strict";
import { test } from "node:test";

const here = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(here, "../src/ext/host.js"), "utf8")
  .replace('import SandboxWorker from "./sandbox.worker.js?worker";', "")
  .replace("export class ExtensionHost", "class ExtensionHost");
const ExtensionHost = new Function("SandboxWorker", source + "\nreturn ExtensionHost;")(class {});

class FakeWorker {
  constructor() { this.listeners = new Set(); this.messages = []; this.terminated = false; }
  addEventListener(type, listener) { if (type === "message") this.listeners.add(listener); }
  removeEventListener(type, listener) { if (type === "message") this.listeners.delete(listener); }
  postMessage(message) { this.messages.push(message); }
  emit(data) { for (const listener of [...this.listeners]) listener({ data }); }
  terminate() { this.terminated = true; }
}

function makeHost() {
  const notices = [];
  const removed = [];
  const host = new ExtensionHost({
    showInformationMessage: (message) => notices.push(message),
    removeStatusBarItem: (key) => removed.push(key),
  });
  const worker = new FakeWorker();
  const entry = { manifest: { id: "example", name: "Example", permissions: [] }, worker, commands: new Set(["example.run"]), pendingInvocations: new Map() };
  host.active.set("example", entry);
  return { host, worker, entry, notices, removed };
}

function withFakeTimers(fn) {
  const oldSet = globalThis.setTimeout, oldClear = globalThis.clearTimeout;
  const timers = new Map(); let nextId = 1;
  globalThis.setTimeout = (callback, ms) => { const id = nextId++; timers.set(id, { callback, ms }); return id; };
  globalThis.clearTimeout = (id) => { timers.delete(id); };
  try { return fn(timers); } finally { globalThis.setTimeout = oldSet; globalThis.clearTimeout = oldClear; }
}

test("deactivating an extension settles pending commands and clears resources immediately", async () => {
  await withFakeTimers(async (timers) => {
    const { host, worker, entry, notices } = makeHost();
    const pending = host.invokeCommand("example.run");
    assert.equal(entry.pendingInvocations.size, 1); assert.equal(worker.listeners.size, 1);
    host.deactivate("example");
    assert.equal(await pending, false);
    assert.equal(entry.pendingInvocations.size, 0); assert.equal(worker.listeners.size, 0);
    assert.equal(notices.length, 0, "deactivation must not emit a delayed timeout toast");
    assert.deepEqual([...timers.values()].map((timer) => timer.ms), [60]);
  });
});

test("extension invoke response settles exactly once and removes listener and timer", async () => {
  await withFakeTimers(async () => {
    const { host, worker, entry, notices } = makeHost();
    const pending = host.invokeCommand("example.run");
    const request = worker.messages.find((message) => message.t === "invoke");
    worker.emit({ t: "invokeResult", reqId: request.reqId, ok: true });
    assert.equal(await pending, true);
    worker.emit({ t: "invokeResult", reqId: request.reqId, ok: false, error: "late" });
    assert.equal(entry.pendingInvocations.size, 0); assert.equal(worker.listeners.size, 0);
    assert.deepEqual(notices, []);
  });
});

test("extension status keys are removed individually and discarded on every deactivate", () => {
  withFakeTimers(() => {
    const { host, entry, removed } = makeHost();
    host.rememberStatusKey("example", "example::one"); host.rememberStatusKey("example", "example::two");
    host.dispatch(entry, "window.removeStatusBarItem", ["one"]);
    assert.deepEqual([...host.statusKeysFor("example")], ["example::two"]);
    host.deactivate("example");
    assert.deepEqual(removed, ["example::one", "example::two"]);
    assert.equal(host._statusKeys?.has("example") ?? false, false);
    const next = { ...entry, worker: new FakeWorker(), pendingInvocations: new Map() };
    host.active.set("example", next); host.rememberStatusKey("example", "example::three"); host.deactivate("example");
    assert.equal(host._statusKeys?.has("example") ?? false, false);
  });
});
