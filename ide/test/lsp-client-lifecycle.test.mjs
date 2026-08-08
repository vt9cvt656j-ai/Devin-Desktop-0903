import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const source = readFileSync(new URL("../src/lsp-client.js", import.meta.url), "utf8")
  .replace('import * as monaco from "monaco-editor";', "")
  .replace("export function createLspManager", "function createLspManager");

const monaco = {
  MarkerSeverity: { Error: 8, Warning: 4, Info: 2, Hint: 1 },
  languages: {
    CompletionItemKind: new Proxy({}, { get: (_target, key) => key }),
  },
  Uri: {
    file: (path) => ({ toString: () => `file://${path}` }),
    parse: (uri) => ({ toString: () => uri, fsPath: uri.replace(/^file:\/\//, "") }),
  },
  editor: {
    getModels: () => [],
    getModel: () => null,
    setModelMarkers: () => {},
  },
};

const { createLspManager } = new Function("monaco", source + "\nreturn { createLspManager };")(monaco);

function tick() {
  return new Promise((resolve) => setImmediate(resolve));
}

function createBackend() {
  const callbacks = [];
  return {
    callbacks,
    async lspStart(_config, callback) { callbacks.push(callback); },
    async lspSend(_lang, raw) {
      const message = JSON.parse(raw);
      if (message.id === undefined) return;
      const callback = callbacks.at(-1);
      queueMicrotask(() => callback({
        kind: "message",
        data: JSON.stringify({ jsonrpc: "2.0", id: message.id, result: { capabilities: {} } }),
      }));
    },
    async lspStop() {},
    async lspCheckAvailable() { return true; },
  };
}

test("LSP ignores a stopped event from the client replaced during restart", async () => {
  const backend = createBackend();
  const manager = createLspManager({ backend, isWorkspaceTrusted: () => true });

  assert.ok(await manager.startManual("rust"));
  const oldCallback = backend.callbacks[0];
  await manager.stop("rust");
  assert.ok(await manager.startManual("rust"));
  assert.equal(manager.isRunning("rust"), true);

  oldCallback({ kind: "stopped", lang: "rust" });
  await tick();

  assert.equal(manager.isRunning("rust"), true, "old channel must not remove the replacement client");
  await manager.stop("rust");
});
