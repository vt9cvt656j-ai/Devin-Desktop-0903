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
  return load("_readWorkspaceMcpDocument", {
    backend,
    _workspaceAncestorRoots,
    _readUserScopeMcpConfigs,
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
