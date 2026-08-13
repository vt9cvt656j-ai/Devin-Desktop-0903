// MCP：配置作用域 + 权限门。
//
// 用户报的是两件事：「把我的 MCP 做成真实可以用的」和「不需要『始终允许』那些按钮
// 点击条件」。查下来它们其实是同一个设计缺口的两面：
//
//   1. 配置**只**来自工作区里的 .mcp.local.json / .mcp.json / .cursor/mcp.json。
//      换一个项目，配好的服务连同填进去的 API Key 全都不在了；没打开文件夹时
//      `_ensureMcpTools` 直接 early-return，一个 MCP 都没有。
//   2. mcp 类型一刀切 needsApproval: true，而"本会话总是允许"只活在内存的
//      `_sessionApproved` 里。于是每开一轮新会话、每换一个工具，都要重新点一次。
//
// 这组测试**跑真函数**（从 main.js 里抠出来注入依赖），不是对着源码做正则匹配——
// 那种断言换个写法就红，却挡不住行为回归。
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const SRC = fs.readFileSync("src/main.js", "utf8");

/** 抠出一个函数体（和 logic.test.mjs 同一套做法的精简版：跳过注释/字符串/模板/正则）。 */
function extractFn(name) {
  const m = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).exec(SRC);
  if (!m) throw new Error(`function ${name} not found in main.js`);
  let i = SRC.indexOf("{", SRC.indexOf(")", m.index));
  let depth = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i], d = SRC[i + 1];
    if (c === "/" && d === "/") { i = SRC.indexOf("\n", i); if (i < 0) i = SRC.length; continue; }
    if (c === "/" && d === "*") { i = SRC.indexOf("*/", i + 2) + 1; continue; }
    if (c === "'" || c === '"' || c === "`") {
      const quote = c;
      for (i++; i < SRC.length; i++) {
        if (SRC[i] === "\\") { i++; continue; }
        if (SRC[i] === quote) break;
      }
      continue;
    }
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (depth === 0) return SRC.slice(m.index, i + 1); }
  }
  throw new Error(`unbalanced braces extracting ${name}`);
}

/**
 * 剥掉注释再断言。
 *
 * 这个坑今天踩了两次：解释一处改动的注释里，几乎一定会原样引用它删掉或绕开的那段代码，
 * 于是"这段代码不该出现"和"A 要排在 B 前面"这类断言全在跟自己的注释较劲。
 * logic.test.mjs 顶部有同名工具，理由一模一样。
 */
function stripJsComments(source) {
  return String(source)
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/(^|[^:])\/\/[^\n]*/g, "$1");
}

function load(name, deps = {}) {
  const keys = Object.keys(deps);
  return new Function(...keys, `${extractFn(name)}\n;return ${name};`)(...keys.map((k) => deps[k]));
}

const _mcpServerIsRepoProvided = load("_mcpServerIsRepoProvided");
const _mcpServerApprovalMode = load("_mcpServerApprovalMode", { _mcpServerIsRepoProvided });
const _workspaceAncestorRoots = load("_workspaceAncestorRoots");

/**
 * 造一个假 backend：磁盘上的文件用 files 映射，用户级配置用 userConfigs 数组
 * （对应 Rust 侧 mcp_user_configs 的返回形状）。
 */
function makeDoc({ files = {}, userConfigs = [] } = {}) {
  const backend = {
    readTextFile: async (path) => {
      if (!(path in files)) throw new Error("ENOENT " + path);
      return files[path];
    },
    invoke: async (cmd) => {
      if (cmd === "mcp_user_configs") return userConfigs;
      throw new Error("unexpected invoke " + cmd);
    },
  };
  const _readUserScopeMcpConfigs = load("_readUserScopeMcpConfigs", { inTauri: true, backend });
  const _disabledMcpServers = load("_disabledMcpServers", { _readUserScopeMcpConfigs });
  return load("_readWorkspaceMcpDocument", {
    backend,
    _workspaceAncestorRoots,
    _readUserScopeMcpConfigs,
    _disabledMcpServers,
  });
}

const USER_FILE = "/home/me/.michael-ide/mcp.json";
const CURSOR_FILE = "/home/me/.cursor/mcp.json";

// ── ① 全局作用域：换项目还在，没打开文件夹也在 ──────────────────────────────

test("全局配置里的服务在任意项目里都能拿到——这就是「真实可以用」的那一半", async () => {
  const read = makeDoc({
    userConfigs: [{ path: USER_FILE, writable: true, servers: { memory: { command: "npx", args: ["-y", "server-memory"] } } }],
  });
  for (const root of ["/work/alpha", "/work/beta"]) {
    const doc = await read(root);
    const servers = JSON.parse(doc.text).mcpServers;
    assert.deepEqual(Object.keys(servers), ["memory"], `${root} 下丢了全局服务`);
    assert.equal(doc.serverScopes.memory, "user");
  }
});

test("一个文件夹都没打开时，全局服务照样在（以前这里直接 early-return 成空）", async () => {
  const read = makeDoc({
    userConfigs: [{ path: USER_FILE, writable: true, servers: { memory: { command: "npx" } } }],
  });
  const doc = await read("");
  assert.deepEqual(Object.keys(JSON.parse(doc.text).mcpServers), ["memory"]);
});

test("项目里的同名服务盖住全局的那个，而不是反过来", async () => {
  const read = makeDoc({
    files: { "/work/a/.mcp.local.json": JSON.stringify({ mcpServers: { db: { command: "项目版" } } }) },
    userConfigs: [{ path: USER_FILE, writable: true, servers: { db: { command: "全局版" } } }],
  });
  const doc = await read("/work/a");
  assert.equal(JSON.parse(doc.text).mcpServers.db.command, "项目版");
  assert.equal(doc.serverScopes.db, "local");
});

test("Claude Code / Cursor 里配好的服务直接就能用，不用再抄一遍", async () => {
  const read = makeDoc({
    userConfigs: [
      { path: USER_FILE, writable: true, servers: {} },
      { path: CURSOR_FILE, writable: false, servers: { "my-api": { command: "node", args: ["s.js"] } } },
    ],
  });
  const doc = await read("/work/a");
  assert.equal(JSON.parse(doc.text).mcpServers["my-api"].command, "node");
  assert.equal(doc.serverScopes["my-api"], "interop");
  assert.equal(doc.serverSources["my-api"], CURSOR_FILE);
});

test("每个服务都带着它的来源作用域——权限门和面板都靠这个分辨谁写的", async () => {
  const read = makeDoc({
    files: {
      "/work/a/.mcp.local.json": JSON.stringify({ mcpServers: { mine: { command: "x" } } }),
      "/work/a/.mcp.json": JSON.stringify({ mcpServers: { fromRepo: { command: "x" } } }),
    },
    userConfigs: [{ path: USER_FILE, writable: true, servers: { global: { command: "x" } } }],
  });
  const doc = await read("/work/a");
  assert.deepEqual(doc.serverScopes, { mine: "local", fromRepo: "repo", global: "user" });
});

// ── ② 权限门：谁写的命令行，谁决定要不要逐次问 ──────────────────────────────

test("仓库自带的 MCP 照旧逐次确认；用户自己配的直接跑", () => {
  assert.equal(_mcpServerApprovalMode("repo", {}), "ask");
  assert.equal(_mcpServerApprovalMode("user", {}), "auto");
  assert.equal(_mcpServerApprovalMode("local", {}), "auto");
  assert.equal(_mcpServerApprovalMode("interop", {}), "auto");
});

test("服务条目里显式写了 approve 就听它的，两个方向都要生效", () => {
  assert.equal(_mcpServerApprovalMode("user", { __michael: { approve: "ask" } }), "ask");
  assert.equal(_mcpServerApprovalMode("repo", { __michael: { approve: "auto" } }), "auto");
  // 写错的值不该被当成"关掉审批"
  assert.equal(_mcpServerApprovalMode("repo", { __michael: { approve: "随便写的" } }), "ask");
});

test("_mcpServerIsRepoProvided 只认 repo 这一种来源", () => {
  assert.equal(_mcpServerIsRepoProvided("repo"), true);
  for (const scope of ["user", "local", "interop", "", undefined]) {
    assert.equal(_mcpServerIsRepoProvided(scope), false, `${scope} 被误判成仓库自带`);
  }
});

// ── ③ 端到端：打开「改动前审批」之后还会不会弹窗 ────────────────────────────

function makeGate({ asked = [] } = {}) {
  const deps = {
    _permissionRuleVerdict: () => "",
    _loadPermissionRules: async () => ({}),
    _callIsDestructive: () => false,
    _dbCallIsDestructive: () => false,
    _callIsReadOnlyCommand: () => false,
    _currentAiPerm: "approve",                 // 用户把「改动前审批」打开了
    _requiresApproval: () => true,             // mcp 类型在策略表里就是 needsApproval
    _approvalKey: (call) => `mcp:${call.server}/${call.tool}`,
    _approvalLabel: () => ({ title: "执行 MCP 工具？", detail: "" }),
    _sessionApproved: new Set(),
    document: { body: {} },
    _toolApprovalDialog: async ({ title }) => { asked.push(title); return "once"; },
  };
  return load("_approveToolCall", deps);
}

test("打开「改动前审批」后，用户自己配的 MCP 工具不再弹窗——一次都不弹", async () => {
  const asked = [];
  const approve = makeGate({ asked });
  const call = { type: "mcp", server: "memory", tool: "search", mcpAutoApprove: true };
  for (let i = 0; i < 5; i++) assert.equal(await approve(call, { root: "/w" }), true);
  assert.deepEqual(asked, [], `不该弹窗，实际弹了 ${asked.length} 次`);
});

test("仓库自带的 MCP 工具照旧要问——这道门不能顺手拆掉", async () => {
  const asked = [];
  const approve = makeGate({ asked });
  const ok = await approve({ type: "mcp", server: "repoSvc", tool: "run", mcpAutoApprove: false }, { root: "/w" });
  assert.equal(ok, true);
  assert.equal(asked.length, 1, "仓库自带的服务必须弹一次");
});

test("工作区权限规则写了 ask，就算是自己配的服务也得问——事先写下的策略优先", async () => {
  const asked = [];
  const approve = load("_approveToolCall", {
    _permissionRuleVerdict: () => "ask",
    _loadPermissionRules: async () => ({}),
    _callIsDestructive: () => false,
    _dbCallIsDestructive: () => false,
    _callIsReadOnlyCommand: () => false,
    _currentAiPerm: "auto",
    _requiresApproval: () => true,
    _approvalKey: () => "k",
    _approvalLabel: () => ({ title: "执行 MCP 工具？", detail: "" }),
    _sessionApproved: new Set(),
    document: { body: {} },
    _toolApprovalDialog: async ({ title }) => { asked.push(title); return "once"; },
  });
  await approve({ type: "mcp", server: "memory", tool: "search", mcpAutoApprove: true }, { root: "/w" });
  assert.equal(asked.length, 1, "permissions.ask 必须压过自动放行");
});

test("规则写了 deny 就直接否决，自动放行不能把它顶掉", async () => {
  const approve = load("_approveToolCall", {
    _permissionRuleVerdict: () => "deny",
    _loadPermissionRules: async () => ({}),
    _callIsDestructive: () => false,
    _dbCallIsDestructive: () => false,
    _callIsReadOnlyCommand: () => false,
    _currentAiPerm: "auto",
    _requiresApproval: () => true,
    _approvalKey: () => "k",
    _approvalLabel: () => ({ title: "", detail: "" }),
    _sessionApproved: new Set(),
    document: { body: {} },
    _toolApprovalDialog: async () => "once",
  });
  assert.equal(await approve({ type: "mcp", mcpAutoApprove: true }, { root: "/w" }), false);
});

// ── ④ 仓库自带配置的确认框：答应过就别再问第二遍 ────────────────────────────

test("对仓库自带配置说了「允许」就记住，不必去点「始终允许」", async () => {
  const stored = new Set();
  const shown = [];
  const approve = load("_approveWorkspaceExecConfig", {
    _loadExecConfigApprovals: async () => stored,
    _toPosix: (p) => p,
    _fingerprint: (t) => "fp:" + t.length,
    _EXEC_CONFIG_APPROVALS_KEY: "k",
    getStore: async () => ({ set: async () => {} }),
    _toolApprovalDialog: async (opts) => { shown.push(opts); return "once"; },
  });
  const details = ["svc：npx -y thing"];
  assert.equal(await approve("MCP", "/w/.mcp.json", "TEXT", details), true);
  assert.equal(await approve("MCP", "/w/.mcp.json", "TEXT", details), true);
  assert.equal(shown.length, 1, `同一份配置只该问一次，实际问了 ${shown.length} 次`);
  // 那个"本会话总是允许"按钮在这个框里是个陷阱：点"允许"的人会被永远拦下去。收成两个按钮。
  assert.equal(shown[0].alwaysLabel, "", "这个确认框不该再出现「本会话总是允许」");
});

test("配置内容一改就重新问——记住的是这一份内容，不是这个路径", async () => {
  const stored = new Set();
  const shown = [];
  const approve = load("_approveWorkspaceExecConfig", {
    _loadExecConfigApprovals: async () => stored,
    _toPosix: (p) => p,
    _fingerprint: (t) => "fp:" + t,
    _EXEC_CONFIG_APPROVALS_KEY: "k",
    getStore: async () => ({ set: async () => {} }),
    _toolApprovalDialog: async (opts) => { shown.push(opts); return "once"; },
  });
  await approve("MCP", "/w/.mcp.json", "旧命令", ["a"]);
  await approve("MCP", "/w/.mcp.json", "换成了别的命令", ["b"]);
  assert.equal(shown.length, 2);
});

test("拒绝就是拒绝，不会被记成同意", async () => {
  const stored = new Set();
  const approve = load("_approveWorkspaceExecConfig", {
    _loadExecConfigApprovals: async () => stored,
    _toPosix: (p) => p,
    _fingerprint: () => "fp",
    _EXEC_CONFIG_APPROVALS_KEY: "k",
    getStore: async () => ({ set: async () => {} }),
    _toolApprovalDialog: async () => "deny",
  });
  assert.equal(await approve("MCP", "/w/.mcp.json", "T", ["a"]), false);
  assert.equal(stored.size, 0, "拒绝不该写进已批准名单");
});

// ── MCP 的两个 UI 入口 ──────────────────────────────────────────────────────
//
// MCP 有三类能力：tools / resources / prompts。这个 IDE 一直只把第一类摆到了台面上，
// 后两类虽然握手时就取回来了（_mcpResourceCache / _mcpPromptCache 里躺着），却只以
// "再包一个工具丢给模型"的形式存在——指望模型自己想起来去调，而它基本想不起来。
//
// 入口一：敲 `/` 的菜单里直接列出各服务的提示词模板（`服务:模板`）。
// 入口二：敲 `@` 的提及菜单里多一类 MCP 资源，选中后内容进这一轮上下文。

const _mcpSlashCommands = (cache) => load("_mcpSlashCommands", { _mcpPromptCache: cache })();

test("MCP 提示词模板变成 `服务:模板` 形式的斜杠命令", () => {
  const rows = _mcpSlashCommands([
    { server: "github", name: "review-pr", description: "审查一个 PR", arguments: [{ name: "pr", required: true }] },
    { server: "ctx7", name: "docs", description: "", arguments: [] },
  ]);
  assert.deepEqual(rows.map((r) => r.cmd), ["ctx7:docs", "github:review-pr"], "要按名字排序");
  assert.equal(rows[1].desc, "审查一个 PR");
  assert.deepEqual(rows[1].mcp, { server: "github", prompt: "review-pr", args: [{ name: "pr", required: true }] });
});

test("模板没写说明时，退而告诉用户它要几个参数、几个必填", () => {
  const [row] = _mcpSlashCommands([
    { server: "db", name: "q", arguments: [{ name: "a", required: true }, { name: "b" }] },
  ]);
  assert.match(row.desc, /2 个参数/);
  assert.match(row.desc, /1 个必填/);
});

test("缺服务名或模板名的条目直接丢掉，不能变成一条点不动的命令", () => {
  assert.deepEqual(_mcpSlashCommands([
    { server: "", name: "x" }, { server: "y", name: "" }, { name: "z" }, null,
  ]), []);
});

test("没连 MCP 时不产生任何斜杠命令", () => {
  assert.deepEqual(_mcpSlashCommands([]), []);
  assert.deepEqual(_mcpSlashCommands(undefined), []);
});

// ── 斜杠菜单的匹配：跑真的 _updateSlashMenu ────────────────────────────────

function makeSlashMatcher(mcpRows) {
  const stub = {
    _SLASH: [{ cmd: "sessions", desc: "" }, { cmd: "memory", desc: "" }],
    _mcpSlashCommands: () => mcpRows,
    promptEl: { value: "", getBoundingClientRect: () => ({ left: 0, top: 0, width: 400 }) },
    _slashMenu: { style: {}, hidden: true },
    window: { innerHeight: 800 },
    _renderSlashActive: () => {},
  };
  const keys = Object.keys(stub);
  return new Function(...keys, `
    let _slashMatches = [], _slashActive = -1;
    function _hideSlash() { _slashMatches = []; }
    ${extractFn("_updateSlashMenu")}
    return (typed) => { _slashMatches = []; promptEl.value = typed; _updateSlashMenu(); return _slashMatches.map((s) => s.cmd); };
  `)(...keys.map((k) => stub[k]));
}

const MCP_ROWS = [
  { cmd: "github:review-pr", desc: "", mcp: {} },
  { cmd: "sequential-thinking:plan", desc: "", mcp: {} },
];

test("斜杠触发的正则放得下服务名里的连字符和 `服务:模板` 的冒号", () => {
  const match = makeSlashMatcher(MCP_ROWS);
  // 原来的 /^\/(\w*)$/ 到这两个都会直接不匹配 → 菜单根本不弹
  assert.deepEqual(match("/sequential-thinking"), ["sequential-thinking:plan"]);
  assert.deepEqual(match("/github:rev"), ["github:review-pr"]);
});

test("只记得模板名、想不起是哪个服务，也能搜到", () => {
  const match = makeSlashMatcher(MCP_ROWS);
  assert.deepEqual(match("/review"), ["github:review-pr"]);
  assert.deepEqual(match("/plan"), ["sequential-thinking:plan"]);
});

test("原有的内置命令一条都没少", () => {
  const match = makeSlashMatcher(MCP_ROWS);
  assert.deepEqual(match("/"), ["sessions", "memory", "github:review-pr", "sequential-thinking:plan"]);
  assert.deepEqual(match("/se"), ["sessions", "sequential-thinking:plan"]);
});

test("输入框里不止一个斜杠命令时不弹菜单（正则钉着整行）", () => {
  const match = makeSlashMatcher(MCP_ROWS);
  assert.deepEqual(match("帮我 /review 一下"), []);
  assert.deepEqual(match("/review 这个 PR"), []);
});

// ── @ 菜单里的 MCP 资源 ────────────────────────────────────────────────────

function makeAtMcpRows({ resources = [], connected = [] } = {}) {
  return load("_atMcpRows", {
    _mcpResourceCache: resources,
    _mcpConnected: connected,
    _pickMcpResource: () => {},
  });
}

test("资源列出来带服务名和说明，模板另作标记", () => {
  const rows = makeAtMcpRows({
    connected: ["pg"],
    resources: [
      { server: "pg", uri: "postgres://db/users", name: "users", description: "用户表" },
      { server: "fs", uriTemplate: "file:///{path}", name: "任意文件", template: true },
    ],
  })("");
  assert.equal(rows[0].name, "users");
  assert.match(rows[0].detail, /^pg · 用户表/);
  assert.equal(rows[1].name, "任意文件（模板）", "模板要标出来——它选中后还要填变量");
  assert.ok(rows.every((r) => r.kind === "mcp" && typeof r.onPick === "function"));
});

test("按服务名、资源名、uri 三样里的任意一样都能搜到", () => {
  const rows = makeAtMcpRows({
    connected: ["pg"],
    resources: [
      { server: "pg", uri: "postgres://db/users", name: "users" },
      { server: "notion", uri: "notion://page/42", name: "路线图" },
    ],
  });
  assert.deepEqual(rows("notion").map((r) => r.name), ["路线图"]);
  assert.deepEqual(rows("users").map((r) => r.name), ["users"]);
  assert.deepEqual(rows("db/us").map((r) => r.name), ["users"]);
});

test("空列表要说清是「没连服务」还是「连了但这些服务没有资源」——两句话不一样", () => {
  const none = makeAtMcpRows({ connected: [], resources: [] })("");
  assert.equal(none.length, 1);
  assert.match(none[0].name, /还没连上/);
  assert.match(none[0].detail, /高级设置/);

  const connectedNoRes = makeAtMcpRows({ connected: ["memory"], resources: [] })("");
  assert.match(connectedNoRes[0].name, /没有提供资源/);
  assert.match(connectedNoRes[0].detail, /可选能力/);
});

test("缺 server 或 uri 的条目丢掉，不能插出一个读不了的 chip", () => {
  const rows = makeAtMcpRows({ connected: ["x"], resources: [
    { server: "x" }, { uri: "a://b" }, null, { server: "x", uri: "a://b", name: "好的" },
  ] })("");
  assert.deepEqual(rows.map((r) => r.name), ["好的"]);
});

// ── 发送时把 @mcp: 换成真内容 ──────────────────────────────────────────────

test("@mcp: 的解析正则认得 服务/uri，并且去重、限量", () => {
  const grab = (text) => [...text.matchAll(/(?:^|\s)@mcp:([^\s@]+)/gi)]
    .map((m) => m[1]).filter((v, i, a) => a.indexOf(v) === i).slice(0, 4);
  assert.deepEqual(grab("看看 @mcp:pg/postgres://db/users 这个表"), ["pg/postgres://db/users"]);
  assert.deepEqual(grab("@mcp:a/x @mcp:a/x"), ["a/x"], "同一个资源只读一次");
  assert.deepEqual(grab("@mcp:a/1 @mcp:a/2 @mcp:a/3 @mcp:a/4 @mcp:a/5").length, 4, "最多四个，别把上下文撑爆");
  assert.deepEqual(grab("邮箱 a@mcp:b/c 不算"), [], "必须前面是行首或空白");
});

test("@文件 扫描要跳过 mcp: 前缀——否则拿它去读本地文件，白占一个提及名额", () => {
  // 锚点钉在循环头上，不要用 "const _mentioned" —— 工作树里 `const _mentionedAll` 排在
  // 它前面，indexOf 会先命中那一个，窗口就偏到别处去了。
  const loopStart = SRC.indexOf("for (const rel of _mentioned.slice(0, 8))");
  assert.ok(loopStart > 0, "找不到 @文件 的扫描循环");
  const loop = stripJsComments(SRC.slice(loopStart, loopStart + 1200));
  assert.match(loop, /if \(\/\^mcp:\/i\.test\(rel\)\) continue;/,
    "文件扫描循环里必须把 mcp: 摘出去");
  assert.ok(loop.indexOf("/^mcp:/i") < loop.indexOf("readTextFile"),
    "跳过要发生在 readTextFile 之前，否则等于没跳");
});

test("MCP 资源的读取不依赖工作区根目录——没打开文件夹时它照样该能用", () => {
  // 窗口切到自己这一段为止：后面紧跟着的就是 @文件 那个 if，它本来就该带 _contextRoot，
  // 窗口开大一点这条断言就变成在骂邻居。
  const blockStart = SRC.indexOf("const _mcpRefs");
  assert.ok(blockStart > 0, "找不到 @mcp: 预取块");
  const blockEnd = SRC.indexOf("if (!_agentLightTurn && _mentioned.length", blockStart);
  assert.ok(blockEnd > blockStart, "找不到 @mcp: 预取块的结尾");
  const block = SRC.slice(blockStart, blockEnd);
  assert.match(block, /if \(!_agentLightTurn && _mcpRefs\.length && inTauri\)/);
  assert.doesNotMatch(block, /_contextRoot/, "MCP 资源不属于任何工作区，不能被 _contextRoot 挡住");
  assert.match(block, /catch[\s\S]{0,120}读取失败/, "读失败要在上下文里留一行，不能静默吞掉");
});

// ── 「删除」在别人的配置上也要真的有用：停用清单 ────────────────────────────
//
// 用户报的是「mcp 这里内容没法真正删除」。查下来那个垃圾桶按钮对**不属于本 IDE 的**服务
// （从 Cursor / Claude Code 读来的、仓库自带的）是 disabled 的——点下去毫无反应，只有悬停
// 才看得到一句说明。等于一个坏掉的按钮。
//
// 但那些服务住在**别人的**配置文件里，Day One 只读不写：总不能因为用户想在这儿少加载一个
// 服务，就跑去改 Cursor 的配置。所以「删除」的正确语义是**停用**——记进自己的
// ~/.michael-ide/mcp.json 的 `disabled: []`，可恢复，一个字节都不碰对方的文件。

const CURSOR_SVC = { path: CURSOR_FILE, writable: false, servers: { "Michael-Cursor": { command: "node" } } };

test("停用之后那个服务不再进合并结果——也就不会连、不会出现在工具里", async () => {
  const read = makeDoc({
    userConfigs: [
      { path: USER_FILE, writable: true, servers: {}, disabled: ["Michael-Cursor"] },
      CURSOR_SVC,
    ],
  });
  const doc = await read("/work/a");
  assert.deepEqual(Object.keys(JSON.parse(doc.text).mcpServers), []);
  assert.ok(doc.disabled.has("michael-cursor"), "停用清单要跟着返回，面板才能给出恢复入口");
  assert.equal(doc.disabled.get("michael-cursor"), "Michael-Cursor",
    "显示要保留用户写的大小写——面板上显示成 michael-cursor 会像是另一个服务");
});

test("没停用时照常在——别把功能反过来了", async () => {
  const read = makeDoc({
    userConfigs: [{ path: USER_FILE, writable: true, servers: {}, disabled: [] }, CURSOR_SVC],
  });
  assert.deepEqual(Object.keys(JSON.parse((await read("/work/a")).text).mcpServers), ["Michael-Cursor"]);
});

test("服务名大小写不一致也认得出来", async () => {
  const read = makeDoc({
    userConfigs: [
      { path: USER_FILE, writable: true, servers: {}, disabled: ["  MICHAEL-cursor  "] },
      CURSOR_SVC,
    ],
  });
  assert.deepEqual(Object.keys(JSON.parse((await read("/work/a")).text).mcpServers), []);
});

test("仓库自带的服务同样可以停用（它也不是本 IDE 的文件）", async () => {
  const read = makeDoc({
    files: { "/work/a/.mcp.json": JSON.stringify({ mcpServers: { fromRepo: { command: "x" } } }) },
    userConfigs: [{ path: USER_FILE, writable: true, servers: {}, disabled: ["fromRepo"] }],
  });
  assert.deepEqual(Object.keys(JSON.parse((await read("/work/a")).text).mcpServers), []);
});

test("只认自己那份配置里的 disabled——别的客户端的文件里没这个概念", async () => {
  const read = makeDoc({
    userConfigs: [
      { path: USER_FILE, writable: true, servers: { mine: { command: "x" } }, disabled: [] },
      // 假装 Cursor 的文件里也有个 disabled，不该被采纳
      { path: CURSOR_FILE, writable: false, servers: {}, disabled: ["mine"] },
    ],
  });
  assert.deepEqual(Object.keys(JSON.parse((await read("/work/a")).text).mcpServers), ["mine"]);
});

test("停用写回自己的全局配置，绝不碰别人的文件", async () => {
  const saved = [];
  const backend = {
    invoke: async (cmd, args) => {
      if (cmd === "mcp_user_configs") {
        return [{ path: USER_FILE, writable: true, servers: { keepMe: { command: "x" } }, disabled: ["old"] },
                { path: CURSOR_FILE, writable: false, servers: { "Michael-Cursor": { command: "node" } } }];
      }
      if (cmd === "mcp_save_user_config") { saved.push(JSON.parse(args.text)); return USER_FILE; }
      throw new Error("unexpected " + cmd);
    },
  };
  const _readUserScopeMcpConfigs = load("_readUserScopeMcpConfigs", { inTauri: true, backend });
  const set = load("_setMcpServerDisabled", { inTauri: true, backend, _readUserScopeMcpConfigs });

  await set("Michael-Cursor", true);
  assert.deepEqual(saved.at(-1).disabled, ["old", "Michael-Cursor"]);
  assert.deepEqual(Object.keys(saved.at(-1).mcpServers), ["keepMe"], "停用不该动到已配置的服务");
  // 只写了自己那一份
  assert.equal(saved.length, 1);

  await set("old", false);
  // 断言字段**不存在**，而不是 `?? []` —— 那样写的话"留了个空数组"也能蒙混过去，
  // 等于这条断言什么都没管住（变异验证时就是它先漏掉的）。
  assert.ok(!("disabled" in saved.at(-1)), `清空后不该留空数组字段：${JSON.stringify(saved.at(-1))}`);
});

test("重复停用同一个不会写出两条", async () => {
  const saved = [];
  const backend = {
    invoke: async (cmd, args) => {
      if (cmd === "mcp_user_configs") return [{ path: USER_FILE, writable: true, servers: {}, disabled: ["dup"] }];
      if (cmd === "mcp_save_user_config") { saved.push(JSON.parse(args.text)); return USER_FILE; }
      throw new Error("unexpected " + cmd);
    },
  };
  const _readUserScopeMcpConfigs = load("_readUserScopeMcpConfigs", { inTauri: true, backend });
  await load("_setMcpServerDisabled", { inTauri: true, backend, _readUserScopeMcpConfigs })("dup", true);
  assert.deepEqual(saved.at(-1).disabled, ["dup"]);
});

test("面板要给出恢复入口，否则停用完就再也找不回来了", () => {
  assert.match(SRC, /已停用 · 不会加载/);
  assert.match(SRC, /_setMcpServerDisabled\(off, false\)/);
  // 编辑仍然不给——那是别人的文件
  assert.match(SRC, /editBtn\.disabled = true;/);
});
