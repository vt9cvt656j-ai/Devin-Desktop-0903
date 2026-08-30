// Windows 上「代码缩略图（Monaco minimap）会丢失」的修复——行为断言。
//
// 根因：Monaco minimap 用 getContext('2d')（无 willReadFrequently）+ putImageData 脏矩形贴图，
// 这在 Windows WebView2 的 GPU 后端 canvas 上留白。修法是在 Windows 上把 2D canvas 默认切到
// CPU 后端（willReadFrequently:true）。这里在 Node 里造一个假 window 真跑补丁，验证四条行为，
// 而不是去 main.js 源码里 grep 字符串。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { isWindowsAgent, installWindowsCanvasFix } from "../src/agent/win-canvas-fix.js";

const HERE = dirname(fileURLToPath(import.meta.url));

// 造一个最小 window：一个带 getContext 的 HTMLCanvasElement.prototype，外加可注入的 navigator。
// getContext 把它收到的 (type, attrs) 记下来，好断言补丁到底往下传了什么。
function fakeWindow(ua, platform = "") {
  const calls = [];
  function getContext(type, attrs) {
    calls.push({ type, attrs });
    return { __type: type, __attrs: attrs };
  }
  return {
    __calls: calls,
    navigator: { userAgent: ua, platform },
    HTMLCanvasElement: { prototype: { getContext } },
  };
}

const WIN_UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120 Safari/537.36 Edg/120";
const MAC_UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)";

test("Windows 识别：UA / platform 命中即为 Windows", () => {
  assert.equal(isWindowsAgent({ userAgent: WIN_UA }), true);
  assert.equal(isWindowsAgent({ userAgent: "", platform: "Win32" }), true);
  assert.equal(isWindowsAgent({ userAgent: MAC_UA }), false);
  assert.equal(isWindowsAgent({ userAgent: "", platform: "MacIntel" }), false);
  assert.equal(isWindowsAgent(null), false);
});

test("Windows 上：2D canvas 的 getContext 被补上 willReadFrequently:true", () => {
  const w = fakeWindow(WIN_UA);
  assert.equal(installWindowsCanvasFix(w), true, "Windows 上应真的打补丁");

  // 无参调用（Monaco 就是这么调的）——必须补上 willReadFrequently:true。
  w.HTMLCanvasElement.prototype.getContext("2d");
  assert.deepEqual(w.__calls.at(-1), { type: "2d", attrs: { willReadFrequently: true } },
    "Monaco 的裸 getContext('2d') 没有被切到 CPU 后端——minimap 仍会在 Windows 留白");
});

test("不碰 webgl，只动 2d", () => {
  const w = fakeWindow(WIN_UA);
  installWindowsCanvasFix(w);
  w.HTMLCanvasElement.prototype.getContext("webgl", { antialias: true });
  assert.deepEqual(w.__calls.at(-1), { type: "webgl", attrs: { antialias: true } },
    "webgl 的上下文属性被动了——游戏那条 WebGL 路径不该受影响");
});

test("调用方显式表过态就尊重，不覆盖", () => {
  const w = fakeWindow(WIN_UA);
  installWindowsCanvasFix(w);
  w.HTMLCanvasElement.prototype.getContext("2d", { willReadFrequently: false });
  assert.equal(w.__calls.at(-1).attrs.willReadFrequently, false,
    "调用方显式关掉了 willReadFrequently，补丁不该强行改成 true");
  // 别原地改调用方的对象：给个带别的键的对象，willReadFrequently 应补进**副本**。
  const passed = { alpha: false };
  w.HTMLCanvasElement.prototype.getContext("2d", passed);
  assert.equal(passed.willReadFrequently, undefined, "不该原地污染调用方传入的属性对象");
  assert.equal(w.__calls.at(-1).attrs.willReadFrequently, true, "副本里应补上 willReadFrequently");
  assert.equal(w.__calls.at(-1).attrs.alpha, false, "副本要保留调用方原有的键");
});

test("非 Windows：完全不打补丁", () => {
  const w = fakeWindow(MAC_UA);
  const before = w.HTMLCanvasElement.prototype.getContext;
  assert.equal(installWindowsCanvasFix(w), false, "mac 上不该打补丁");
  assert.equal(w.HTMLCanvasElement.prototype.getContext, before, "mac 上 getContext 不该被替换");
  w.HTMLCanvasElement.prototype.getContext("2d");
  assert.equal("willReadFrequently" in (w.__calls.at(-1).attrs || {}), false,
    "mac 上不该注入任何属性");
});

test("幂等：装第二次不叠第二层", () => {
  const w = fakeWindow(WIN_UA);
  assert.equal(installWindowsCanvasFix(w), true);
  const patched = w.HTMLCanvasElement.prototype.getContext;
  assert.equal(installWindowsCanvasFix(w), false, "第二次安装应短路返回 false");
  assert.equal(w.HTMLCanvasElement.prototype.getContext, patched, "不该再包一层");
});

test("main.js 在建编辑器之前就安装了补丁", () => {
  // 光有模块没接上等于没修。钉两件事：main.js import 了它、且**调用**发生在第一处
  // monaco.editor.create 之前（否则第一个编辑器的 minimap canvas 已经拿到 GPU 上下文了）。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /import\s*\{[^}]*installWindowsCanvasFix[^}]*\}\s*from\s*["']\.\/agent\/win-canvas-fix\.js["']/,
    "main.js 没有 import installWindowsCanvasFix");
  const callAt = src.indexOf("installWindowsCanvasFix(");
  const createAt = src.indexOf("monaco.editor.create(");
  assert.ok(callAt >= 0, "main.js 没有调用 installWindowsCanvasFix()");
  assert.ok(createAt >= 0, "main.js 里找不到 monaco.editor.create——测试台锚点失效了");
  // 注意 callAt 命中的第一处是 import 语句里的名字；找**调用点**（import 之后的那次）。
  const importAt = src.indexOf("installWindowsCanvasFix }");
  const realCallAt = src.indexOf("installWindowsCanvasFix();");
  assert.ok(realCallAt >= 0, "找不到 installWindowsCanvasFix() 的调用点");
  assert.ok(realCallAt < createAt,
    "安装补丁必须发生在第一处 monaco.editor.create 之前，否则首个编辑器的 minimap 仍在 GPU 后端");
  void importAt;
});
