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

// ── Windows 上装语言服务器 ────────────────────────────────────────────────────
//
// 用户报的是「安装语言那些都用不了，安装不成功」。三处独立的原因，全在 Windows：
//
//   1. 22 个语言里 11 个的安装命令写的是 `brew install` —— Windows 上没有 brew。
//   2. Pyright 那条带 `2>/dev/null`；cmd 里那不是"丢弃错误输出"，是往 `.\dev\null`
//      这个不存在的目录写文件。
//   3. **最要命的一条**：进度卡片判断"装好了没"用的是
//      `command -v X || ls "$HOME/go/bin/X" … /opt/homebrew/bin/X`，POSIX/macOS 专用。
//      Windows 上它永远返回空——于是即使 pip 真的装成功了，卡片也会转满 90 秒然后
//      报「安装超时」。用户看到的就是"根本用不了"。
//
// 这三条都不能在这台 mac 上复现，所以守卫钉的是源码本身。

test("安装命令按平台分：Windows 上不许出现 brew，也不许出现 POSIX 重定向", () => {
  const table = source.slice(source.indexOf("const CROSS = {"), source.indexOf("const installHints ="));
  assert.ok(table.length > 200, "找不到安装命令表");

  // 跨平台那张表里一条都不许有 brew / POSIX 重定向 —— 它两个平台共用。
  const cross = table.slice(table.indexOf("const CROSS = {"), table.indexOf("const MAC_ONLY = {"));
  assert.doesNotMatch(cross, /brew /, "跨平台表里混进了 brew");
  assert.doesNotMatch(cross, /2>\/dev\/null/, "跨平台表里还留着 POSIX 重定向");
  assert.match(cross, /python: "pip install pyright"/, "Pyright 那条没改干净");

  // Windows 那张表里同样不许有 brew。
  const win = table.slice(table.indexOf("const WIN_ONLY = {"));
  assert.doesNotMatch(win, /brew /, "Windows 表里出现了 brew——那条命令在 Windows 上必然失败");

  // 选表这一步真的按平台走。
  assert.match(source, /const installHints = isWindows\(\)\s*\?\s*\{ \.\.\.CROSS, \.\.\.WIN_ONLY \}\s*:\s*\{ \.\.\.CROSS, \.\.\.MAC_ONLY \}/,
    "没有按平台选表");
  assert.match(source, /function isWindows\(\)/, "缺少平台判定");
});

test("这个平台上没有一键包时，不给一条注定失败的命令", () => {
  // 给了就是让用户点一下、看它报错，再等 90 秒进度条转完告诉他"安装超时"。
  const block = source.slice(source.indexOf("if (!toolExists && showNotification"), source.indexOf("} else if (toolExists)"));
  assert.ok(block.length > 100, "找不到那段通知逻辑");
  assert.match(block, /actionLabel: hint \? "安装" : undefined/,
    "没有命令时还挂着「安装」按钮");
  assert.match(block, /hint\s*\?[\s\S]{0,120}:\s*`这个平台上它没有一键安装的包/,
    "没有命令时文案没说清该怎么办");
  // 没有命令也要提示（以前 hint 为空整条通知都不弹，用户完全不知道缺什么）。
  assert.match(block, /!_lspAlreadyPrompted\(langId\) && names\[langId\]/,
    "缺服务器这件事本身也该告诉用户，不该因为没有安装命令就整条不提");
});

test("langId 要一路传到进度卡片——它是跨平台探测的入口", () => {
  assert.match(source, /installCmd: hint,\n[\s\S]{0,300}langId,/,
    "通知里没带 langId，进度卡片就没法问后端「装好了没」");
});

