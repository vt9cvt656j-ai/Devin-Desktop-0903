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
    // registerProviders 会调一大票 registerXxxProvider —— 全部记下选择器，
    // 「.vue 用不用得上编辑器里的补全/跳转」就是靠这个选择器决定的。
    _selectors: [],
  },
  Uri: {
    file: (path) => ({ toString: () => `file://${path}` }),
    parse: (uri) => ({ toString: () => uri, fsPath: uri.replace(/^file:\/\//, "") }),
  },
  editor: {
    getModels: () => [],
    getModel: () => null,
    setModelMarkers: () => {},
    registerCommand: () => ({ dispose() {} }),
  },
};

monaco.languages = new Proxy(monaco.languages, {
  get(t, k) {
    if (typeof k === "string" && /^register\w+Provider$/.test(k)) {
      return (selector) => { t._selectors.push([k, selector]); return { dispose() {} }; };
    }
    if (k === "registerCompletionItemProvider") {
      return (selector) => { t._selectors.push([k, selector]); return { dispose() {} }; };
    }
    return t[k];
  },
});
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


// ── 「已经在跑了」不许静默认输 ──────────────────────────────────────────────
//
// 后端那条记录成孤儿时（读线程遇到一个非法帧就退出、却没把自己从 map 里摘掉），
// lsp_start 会返回「LSP for 'x' is already running」。这条分支原来只写一行日志就
// 静默 return —— 于是这门语言整个会话再也起不来，界面上一个字都没有。
test("后端说「已经在跑了」时，先真的停掉再重试一次，而不是静默放弃", async () => {
  let starts = 0, stops = 0;
  const callbacks = [];
  const backend = {
    async lspStart(_config, cb) {
      callbacks.push(cb);
      starts++;
      // 第一次：假装后端 map 里有一条孤儿记录。停过之后才让它起来。
      if (starts === 1 && stops === 0) throw new Error("LSP for 'rust' is already running");
    },
    async lspSend(_lang, raw) {
      const m = JSON.parse(raw);
      if (m.id === undefined) return;
      const cb = callbacks.at(-1);
      queueMicrotask(() => cb({ kind: "message", data: JSON.stringify({ jsonrpc: "2.0", id: m.id, result: { capabilities: {} } }) }));
    },
    async lspStop() { stops++; },
    async lspCheckAvailable() { return true; },
  };
  const toasts = [];
  const manager = createLspManager({ backend, isWorkspaceTrusted: () => true, showToast: (m) => toasts.push(m) });

  const client = await manager.startManual("rust");
  assert.ok(client, "「已经在跑了」之后就放弃了 —— 这门语言整个会话再也起不来，而且一声不吭");
  assert.equal(stops, 1, "没有先把那条孤儿记录停掉，重试必然还是同一个错");
  assert.equal(manager.isRunning("rust"), true, "重试起来了却没登记");
  await manager.stop("rust");
});

test("停掉之后仍然起不来，要说出来，不能静默", async () => {
  const backend = {
    async lspStart() { throw new Error("LSP for 'rust' is already running"); },
    async lspSend() {}, async lspStop() {}, async lspCheckAvailable() { return true; },
  };
  const toasts = [];
  const manager = createLspManager({ backend, isWorkspaceTrusted: () => true, showToast: (m) => toasts.push(m) });
  const client = await manager.startManual("rust");
  assert.equal(client, null);
  assert.ok(toasts.some((t) => /语言服务卡住了/.test(t)),
    "两次都失败却一个字都不说 —— 用户只会看到「代码智能没了」，无从下手");
  assert.equal(manager.isRunning("rust"), false, "失败了还留着 client，状态栏会显示它在跑");
});

// ── 智能体改了一个「没开标签页」的文件，语言服务器要跟上 ────────────────────
//
// 缺这一条的后果是「报错一直是旧版的、怎么改都不消失」：pyright 为一个没开标签页的
// models.py 推过诊断（跨文件诊断会给它建惰性 model 并 didOpen），智能体随后把那个类型
// 错误修好了 —— 磁盘写成功，但同步点因为这个文件既不在 openFiles 也不在 projectModels
// （.py 不在预载扩展名里）而直接返回 "closed"，didChange 一次都没发。服务器手里仍是旧
// 文本，继续推同一条旧错误 → 进 markers → 进每轮喂给模型的「实时诊断」块 → 模型看到
// 「没修上」，再改一遍同一行，循环。
test("改了没开标签页的文件，新内容要 didChange 给语言服务器", async () => {
  const sent = [];
  const backend = {
    async lspStart(_c, cb) { this._cb = cb; },
    async lspSend(_lang, raw) {
      const m = JSON.parse(raw);
      sent.push(m);
      if (m.id === undefined) return;
      queueMicrotask(() => this._cb({ kind: "message", data: JSON.stringify({ jsonrpc: "2.0", id: m.id, result: { capabilities: {} } }) }));
    },
    async lspStop() {}, async lspCheckAvailable() { return true; },
  };
  // 一份「跨文件诊断建出来的惰性 model」：没挂在编辑器上。
  let value = "old = 1\n";
  const model = {
    uri: { toString: () => "file:///p/models.py" },
    getLanguageId: () => "python",
    getValue: () => value,
    setValue: (v) => { value = v; },
    getVersionId: () => 2,
    isAttachedToEditor: () => false,
  };
  monaco.editor.getModel = () => model;

  const manager = createLspManager({ backend, isWorkspaceTrusted: () => true });
  assert.ok(await manager.startManual("python"));
  sent.length = 0;

  const synced = manager.syncFromDisk("/p/models.py", "new = 2\n");
  assert.equal(synced, true, "没同步 —— 服务器手里永远是旧文本，那条修好的错误会一直被推回来");
  assert.equal(value, "new = 2\n", "model 的内容没更新，lsp_* 查询会按改前的行号算");
  const notify = sent.find((m) => m.method === "textDocument/didOpen" || m.method === "textDocument/didChange");
  assert.ok(notify, "一条同步通知都没发给语言服务器");
  const text = JSON.stringify(notify.params);
  assert.match(text, /new = 2/, "发过去的还是旧内容");

  await manager.stop("python");
  monaco.editor.getModel = () => null;
});

test("用户正开着在编辑的缓冲区不许被覆盖", () => {
  let value = "用户刚敲的字，还没落盘\n";
  const model = {
    uri: { toString: () => "file:///p/a.py" },
    getLanguageId: () => "python",
    getValue: () => value,
    setValue: (v) => { value = v; },
    getVersionId: () => 1,
    isAttachedToEditor: () => true,      // ← 挂在编辑器上
  };
  monaco.editor.getModel = () => model;
  const manager = createLspManager({ backend: { async lspStart() {}, async lspSend() {}, async lspStop() {}, async lspCheckAvailable() { return true; } }, isWorkspaceTrusted: () => true });
  assert.equal(manager.syncFromDisk("/p/a.py", "磁盘上的旧内容"), false, "覆盖了正在编辑的缓冲区 —— 那是直接丢用户的字");
  assert.equal(value, "用户刚敲的字，还没落盘\n");
  monaco.editor.getModel = () => null;
});

// ── 换项目要把语言服务器一起换掉 ──────────────────────────────────────────
//
// rootUri / workspaceFolders 是 initialize 时一次性钉进去的，之后全仓没有一处改过它
// （也没有一处发 workspace/didChangeWorkspaceFolders）。打开 Rust 项目 A → rust-analyzer
// 按 A 的 Cargo workspace 起来；不重启应用直接切到项目 B，那个进程还在按 A 工作。
// 打开 B/src/main.rs → 不属于 A 认识的任何 crate → 补全、跳转、诊断全没了，而状态栏
// 仍然显示「LSP: rust」一切正常。唯一的恢复手段是重启应用。
test("换工作区时把在跑的语言服务器全停掉，并清掉 venv 探测结果", async () => {
  const stopped = [];
  let detects = 0;
  const backend = {
    async lspStart(_c, cb) { this._cb = cb; },
    async lspSend(_lang, raw) {
      const m = JSON.parse(raw);
      if (m.id === undefined) return;
      queueMicrotask(() => this._cb({ kind: "message", data: JSON.stringify({ jsonrpc: "2.0", id: m.id, result: { capabilities: {} } }) }));
    },
    async lspStop(lang) { stopped.push(lang); },
    async lspCheckAvailable() { return true; },
    async lspDetectPython() { detects++; return { pythonPath: "/a/.venv/bin/python", sitePackages: ["/a/sp"] }; },
  };
  const manager = createLspManager({ backend, isWorkspaceTrusted: () => true, getWorkspaceRoots: () => ["/a"] });
  assert.ok(await manager.startManual("rust"));
  assert.ok(await manager.startManual("python"));
  assert.equal(manager.isRunning("rust"), true);

  const langs = await manager.resetForNewWorkspace();

  assert.deepEqual(langs.sort(), ["python", "rust"], "没报告停了哪些");
  assert.equal(manager.isRunning("rust"), false, "rust-analyzer 还挂着 —— 它按上一个项目的 Cargo workspace 工作");
  assert.equal(manager.isRunning("python"), false);
  assert.deepEqual(stopped.sort(), ["python", "rust"], "没真的通知后端停掉进程");
  assert.equal(manager.status().length, 0, "状态栏还会显示它们在跑");
  // venv 探测结果只在第一次 ensureServer 时算一次。断言「行为」而不是那个内部字段：
  // 重置之后再起 python，必须**重新探测一次**——否则新项目的 pyright 会拿上一个项目的
  // 解释器去解析包，满屏「import X could not be resolved」。
  const before = detects;
  assert.ok(await manager.startManual("python"));
  assert.equal(detects, before + 1,
    "换项目后没重新探测 venv —— 新项目的 pyright 用的还是上一个项目的解释器");
  await manager.stop("python");
});

// ── .vue 整条链原来不可达 ────────────────────────────────────────────────
//
// 后端 KNOWN_SERVERS 里登记着 vue-language-server，但客户端把 .vue 的 Monaco languageId
// 写死成 "html"，于是**没有任何 model 的 languageId 会等于 "vue"** —— 那条登记项从来
// 没有机会被启动过。不改那个映射是有意的：改了，.vue 就失去 HTML 高亮和 html worker，
// 没装 vue-language-server 的用户会倒退。所以只在 LSP 这一层按扩展名认它。
test(".vue 按扩展名路由到 vue 服务器，真正的 .html 不受影响", async () => {
  const started = [];
  const backend = {
    async lspStart(config, cb) { started.push(config.lang); this._cb = cb; },
    async lspSend(_lang, raw) {
      const m = JSON.parse(raw);
      if (m.id === undefined) return;
      queueMicrotask(() => this._cb({ kind: "message", data: JSON.stringify({ jsonrpc: "2.0", id: m.id, result: { capabilities: {} } }) }));
    },
    async lspStop() {}, async lspCheckAvailable() { return true; },
  };
  const manager = createLspManager({ backend, isWorkspaceTrusted: () => true });
  assert.ok(await manager.startManual("vue"), "vue 服务器起不来");

  const mk = (uri, text) => {
    let v = text;
    return { uri: { toString: () => uri }, getLanguageId: () => "html",
      getValue: () => v, setValue: (x) => { v = x; }, getVersionId: () => 1, isAttachedToEditor: () => false };
  };
  // 观察「通知发给了谁」——lspSend 的第一个参数就是服务器那侧的 lang。
  const sentTo = [];
  const origSend = backend.lspSend.bind(backend);
  backend.lspSend = async (lang, raw) => { sentTo.push([lang, raw]); return origSend(lang, raw); };

  // .vue：Monaco 说它是 html，但 LSP 这一层要认成 vue。
  monaco.editor.getModel = () => mk("file:///p/App.vue", "<template/>");
  assert.equal(manager.syncFromDisk("/p/App.vue", "<template>x</template>"), true, ".vue 的 model 没被更新");
  assert.ok(sentTo.some(([lang, raw]) => lang === "vue" && /App\.vue/.test(raw)),
    ".vue 的改动没发给 vue 服务器 —— 那条链还是不可达（后端登记了 vue-language-server，"
    + "但 Monaco 里根本不存在 \"vue\" 这个 languageId）");

  // 真正的 .html 不许被路由到 vue 服务器，否则每个 html 文件都被喂给它。
  sentTo.length = 0;
  monaco.editor.getModel = () => mk("file:///p/index.html", "<html/>");
  assert.equal(manager.syncFromDisk("/p/index.html", "<html>x</html>"), true, "普通 html 的 model 也该被更新");
  assert.equal(sentTo.filter(([lang]) => lang === "vue").length, 0,
    "普通 .html 被喂给了 vue 服务器");
  monaco.editor.getModel = () => null;
  await manager.stop("vue");
});

test("编辑器里的补全/跳转选择器要覆盖到 .vue 所在的那个 Monaco 语言", () => {
  // provider 注册在 **Monaco 的 languageId** 上，而 .vue 的 languageId 是 "html"。
  // 选择器里没有 "html"，编辑器里的补全/悬停/跳转对 .vue 一次都不会触发 ——
  // 那样就只有智能体的 lsp_* 工具能用上 vue 服务器，用户自己敲代码时还是什么都没有。
  monaco.languages._selectors.length = 0;
  const manager = createLspManager({
    backend: { async lspStart() {}, async lspSend() {}, async lspStop() {}, async lspCheckAvailable() { return true; } },
    isWorkspaceTrusted: () => true,
  });
  manager.registerProviders();
  assert.ok(monaco.languages._selectors.length > 10, `只注册了 ${monaco.languages._selectors.length} 个 provider，桩没接上`);
  // registerWorkspaceSymbolProvider 是全局的，第一个参数就是 provider 本身，没有选择器。
  const withSelector = monaco.languages._selectors.filter(([, sel]) => Array.isArray(sel));
  assert.ok(withSelector.length > 10, `带选择器的 provider 只有 ${withSelector.length} 个`);
  for (const [name, sel] of withSelector) {
    assert.ok(sel.includes("vue"), `${name} 的选择器里没有 vue`);
    assert.ok(sel.includes("html"),
      `${name} 的选择器里没有 "html" —— .vue 的 Monaco languageId 就是 html，`
      + "少了它，用户在 .vue 里敲代码时补全/跳转一次都不会触发");
  }
  // 反向：不该把选择器扩成「所有语言」。
  const one = withSelector[0][1];
  assert.ok(one.length < 40, "选择器被扩得太宽了");
});
