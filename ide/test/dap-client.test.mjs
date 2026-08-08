import test from "node:test";
import assert from "node:assert/strict";
import { createDapManager } from "../src/dap-client.js";

function tick() {
  return new Promise((resolve) => setImmediate(resolve));
}

function createBackend() {
  const callbacks = [];
  const requests = [];
  const stops = [];
  let stopGate = null;
  return {
    callbacks,
    requests,
    stops,
    holdNextStop() {
      let release;
      stopGate = new Promise((resolve) => { release = resolve; });
      return release;
    },
    async dapStart(_config, callback) {
      callbacks.push(callback);
    },
    async dapSend(_adapterId, raw) {
      const message = JSON.parse(raw);
      requests.push(message);
      if (message.type !== "request" || message.command === "stackTrace") return;
      const callback = callbacks.at(-1);
      queueMicrotask(() => callback({
        kind: "message",
        data: JSON.stringify({
          type: "response",
          request_seq: message.seq,
          success: true,
          body: message.command === "initialize" ? {} : {},
        }),
      }));
    },
    async dapStop(adapterId) {
      stops.push(adapterId);
      if (stopGate) {
        const gate = stopGate;
        stopGate = null;
        await gate;
      }
    },
  };
}

const config = { adapterId: "test-adapter", request: "launch", launchArgs: {} };

test("DAP ignores stale adapter events and survives termination during stack refresh", async () => {
  const backend = createBackend();
  const manager = createDapManager({ backend });
  assert.equal(await manager.start(config), true);
  const oldCallback = backend.callbacks[0];

  oldCallback({
    kind: "message",
    data: JSON.stringify({ type: "event", event: "stopped", body: { threadId: 7, reason: "breakpoint" } }),
  });
  await tick();
  assert.equal(backend.requests.at(-1).command, "stackTrace");
  oldCallback({
    kind: "message",
    data: JSON.stringify({ type: "event", event: "terminated", body: {} }),
  });
  await tick();
  assert.equal(manager.isActive(), false);

  assert.equal(await manager.start(config), true);
  oldCallback({
    kind: "message",
    data: JSON.stringify({ type: "event", event: "output", body: { output: "stale output" } }),
  });
  oldCallback({
    kind: "message",
    data: JSON.stringify({ type: "event", event: "terminated", body: {} }),
  });
  await tick();

  assert.equal(manager.isActive(), true);
  assert.equal(manager.consoleLog().some((entry) => entry.text.includes("stale output")), false);
  await manager.stop();
});

test("DAP waits for previous backend cleanup before starting the same adapter again", async () => {
  const backend = createBackend();
  const manager = createDapManager({ backend });
  assert.equal(await manager.start(config), true);
  const releaseStop = backend.holdNextStop();
  backend.callbacks[0]({
    kind: "message",
    data: JSON.stringify({ type: "event", event: "terminated", body: {} }),
  });
  await tick();

  const restarting = manager.start(config);
  await tick();
  assert.equal(backend.callbacks.length, 1);
  releaseStop();
  assert.equal(await restarting, true);
  assert.equal(backend.callbacks.length, 2);
  await manager.stop();
});
