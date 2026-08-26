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

// awaitStop 是给模型用的那条腿：它得能回答"停了没、停在哪、还是根本没停"。
//
// 这个假后端只假装**传输层**（Content-Length 拆包本来就在 Rust 里），每一条 DAP 语义
// 都按协议真答：stackTrace 真回栈帧。所以 frames 是走完真实 refreshStack 的结果，不是编
// 出来的常量 —— 它产不出 `expr = (mock) 42` 那种假绿，因为它根本不参与 evaluate。
// 真适配器那一侧只能手工验（这台机器上 debugpy 和 js-debug-adapter-stdio 都没装），
// 判据写在提交信息里：evaluate 的值必须随 continue 变化，不变就说明接的不是真适配器。
function backendAnsweringStackTrace() {
  const callbacks = [];
  return {
    callbacks,
    async dapStart(_c, cb) { callbacks.push(cb); },
    async dapSend(_id, raw) {
      const m = JSON.parse(raw);
      if (m.type !== "request") return;
      const body = m.command === "stackTrace"
        ? { stackFrames: [{ id: 11, name: "handler", line: 42, column: 1, source: { path: "/repo/app.py" } }] }
        : {};
      queueMicrotask(() => callbacks.at(-1)({ kind: "message", data: JSON.stringify({ type: "response", request_seq: m.seq, success: true, body }) }));
    },
    async dapStop() {},
  };
}

test("await_stop 是三态，而且都不是 null", async () => {
  const b = backendAnsweringStackTrace();
  const m = createDapManager({ backend: b });
  // ① 根本没有会话
  assert.deepEqual(await m.awaitStop({ timeoutMs: 1000 }), { state: "terminated", reason: "no session" });
  assert.equal(await m.start(config), true);
  // ② 等到 stopped，且带着真栈帧
  const waiting = m.awaitStop({ timeoutMs: 20000 });
  b.callbacks[0]({ kind: "message", data: JSON.stringify({ type: "event", event: "stopped", body: { threadId: 7, reason: "breakpoint" } }) });
  const hit = await waiting;
  assert.equal(hit.state, "stopped");
  assert.equal(hit.reason, "breakpoint");
  assert.equal(hit.threadId, 7);
  assert.deepEqual(hit.frames.map((f) => `${f.name}:${f.line}`), ["handler:42"]);
  // 已经停着时立刻回 stopped，不再等一轮
  assert.equal((await m.awaitStop({ timeoutMs: 20000 })).state, "stopped");
  await m.stop();
});

test("进程结束是 terminated，不是 timeout 也不是 null", async () => {
  const b = backendAnsweringStackTrace();
  const m = createDapManager({ backend: b });
  await m.start(config);
  const waiting = m.awaitStop({ timeoutMs: 20000 });
  b.callbacks[0]({ kind: "message", data: JSON.stringify({ type: "event", event: "terminated", body: {} }) });
  const r = await waiting;
  assert.equal(r.state, "terminated");
  assert.notEqual(r, null, "三态的意义就在于 null 说不清是哪一种");
});

test("预算耗尽是 timeout，且下限被夹到 1000ms", async () => {
  const b = backendAnsweringStackTrace();
  const m = createDapManager({ backend: b });
  await m.start(config);
  const r = await m.awaitStop({ timeoutMs: 5 });
  assert.equal(r.state, "timeout");
  assert.equal(r.waitedMs, 1000);
  await m.stop();
});
