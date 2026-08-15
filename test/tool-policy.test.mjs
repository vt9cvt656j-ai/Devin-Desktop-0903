// Tests for the tool policy registry.
//
// Note what these DON'T do: no `extractFn`, no `assert.match(SRC, /regex/)`. The module is
// pure, so it is imported and called like ordinary code. That is the whole point of moving it
// out of main.js — 1,221 assertions in logic.test.mjs are welded to the SHAPE of the source
// and break whenever the code is improved. Nothing here can break for that reason.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import {
  DEFAULT_POLICY,
  allPolicies,
  approvalTypes,
  blockedInReadOnlyMode,
  defineTool,
  fileEditTypes,
  fileMutationTypes,
  hookedTypes,
  isFileEdit,
  isFileMutation,
  mutatesWorkspace,
  needsApproval,
  readOnlyBlockedTypes,
  toolPolicy,
  workerScopeField,
  workerScopeTarget,
  workspaceMutatingTypes,
} from "../src/agent/tool-policy.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const MAIN = readFileSync(join(HERE, "../src/main.js"), "utf8");

const sorted = (set) => [...set].sort();

// ── The pinning tests ───────────────────────────────────────────────────────
//
// These encode "this refactor changed nothing". Each asserts the derived set has EXACTLY the
// membership the hand-written literal had before the registry existed. Without them, a
// migration like this is a leap of faith; with them it is verifiable.
//
// The expected lists are transcribed from the pre-refactor source and must never be "fixed"
// to match the code — if one fails, either the registry is wrong or a deliberate behaviour
// change is being made, and the second case belongs in its own commit with this list updated
// as part of it.

test("workspace-mutating set matches the pre-refactor literal exactly", () => {
  assert.deepEqual(sorted(workspaceMutatingTypes()), sorted(new Set([
    "write", "edit", "multiedit", "delete", "move", "mkdir", "copy", "format",
    "game_scaffold", "web_scaffold", "download", "download_asset", "genimage", "generate_3d",
    "generate_sound", "generate_music", "generate_voice", "auto_rig", "generate_motion",
    "generate_texture",
  ])));
  // The subtle one: a shell command may change the workspace but never REPORTS it, so it is
  // not in this set. Adding it would make `mutated === false` look like proof of a no-op.
  assert.equal(mutatesWorkspace("cmd"), false);
  assert.equal(mutatesWorkspace("termtask"), false);
});

test("approval set matches the pre-refactor literal exactly", () => {
  assert.deepEqual(sorted(approvalTypes()), sorted(new Set([
    "write", "edit", "multiedit", "delete", "move", "mkdir", "copy", "format",
    "cmd", "termtask", "automation", "uiclick", "download", "db", "mcp",
  ])));
});

test("hooked set matches the pre-refactor literal exactly, including format's absence", () => {
  assert.deepEqual(sorted(hookedTypes()), sorted(new Set([
    "write", "edit", "multiedit", "cmd", "termtask", "delete", "move", "mkdir", "copy",
  ])));
  // `format` writes content but is intentionally NOT hooked. It is the single element that
  // makes this set differ from the file-mutation family, and it was easy to lose.
  assert.equal(isFileEdit("format"), true);
  assert.equal(toolPolicy("format").hooked, false);
});

test("read-only-mode block matches the pre-refactor chain, plus the closed termtask gap", () => {
  assert.deepEqual(sorted(readOnlyBlockedTypes()), sorted(new Set([
    "write", "edit", "multiedit", "cmd", "delete", "move", "mkdir", "copy", "format",
    "uiclick", "mcp", "termtask",
  ])));
  // 上一版这里断言的是 `false`，并写着「补掉的时候这一行要在同一个提交里翻成 true」——
  // 这就是那个提交。termtask 就是 run_in_terminal，命令串由模型给出、原样执行，和 cmd
  // 是同一类能力；cmd 在只读模式被挡而它不被挡，等于换个工具名就绕过去了。
  assert.equal(blockedInReadOnlyMode("termtask"), true,
    "run_in_terminal is arbitrary shell — a read-only mode must not be able to start one");
});

test("file-mutation and file-edit families match their pre-refactor literals", () => {
  assert.deepEqual(sorted(fileMutationTypes()), sorted(new Set([
    "write", "edit", "multiedit", "delete", "move", "mkdir", "copy", "format",
  ])));
  assert.deepEqual(sorted(fileEditTypes()), sorted(new Set([
    "write", "edit", "multiedit", "format",
  ])));
  // A generator lands files in the workspace but is not a structured file operation — the
  // distinction the flat lists kept blurring.
  assert.equal(mutatesWorkspace("genimage"), true);
  assert.equal(isFileMutation("genimage"), false);
});

test("worker scope targets match the pre-refactor list", () => {
  const scoped = sorted(new Set(Object.keys(allPolicies()).filter((t) => workerScopeField(t))));
  assert.deepEqual(scoped, sorted(new Set([
    "write", "edit", "multiedit", "mkdir", "copy", "format",
  ])));
  // delete/move are refused for workers outright rather than scope-checked.
  assert.equal(workerScopeField("delete"), "");
  assert.equal(workerScopeField("move"), "");
  // The helper returns the concrete path, so the executor never re-derives "which field".
  assert.equal(workerScopeTarget({ type: "write", path: "src/a.ts" }), "src/a.ts");
  assert.equal(workerScopeTarget({ type: "copy", path: "", to: "src/b.ts" }), "src/b.ts");
  assert.equal(workerScopeTarget({ type: "read", path: "src/a.ts" }), "", "reads are unscoped");
  assert.equal(workerScopeTarget(null), "");
});

// ── Behaviour of the registry itself ────────────────────────────────────────

test("an unregistered tool gets the safe default, so read-only tools need no declaration", () => {
  // The large majority of the 126 call types are read-only lookups. Requiring a declaration
  // for each would be a list that rots; the default IS their policy.
  for (const t of ["npm_search", "arxiv_search", "read", "list", "current_time", "think", ""]) {
    assert.deepEqual(toolPolicy(t), DEFAULT_POLICY, `${t || "(empty)"} should default`);
  }
  assert.equal(needsApproval("some_tool_invented_tomorrow"), false);
  assert.equal(blockedInReadOnlyMode("some_tool_invented_tomorrow"), false);
});

test("adding a tool is one call, and it is reflected in every derived set at once", () => {
  // This is the property the whole module exists for: one declaration, not eleven edits.
  defineTool("__test_tool__", { mutatesWorkspace: true, needsApproval: true, readOnlyModeBlocked: true });
  assert.ok(workspaceMutatingTypes().has("__test_tool__"));
  assert.ok(approvalTypes().has("__test_tool__"));
  assert.ok(readOnlyBlockedTypes().has("__test_tool__"));
  assert.equal(hookedTypes().has("__test_tool__"), false, "unspecified flags stay at the default");
  // Re-declaring replaces cleanly, so a plugin can override a built-in.
  defineTool("__test_tool__", { needsApproval: true });
  assert.equal(mutatesWorkspace("__test_tool__"), false);
});

test("a typo'd policy field is rejected at declaration instead of silently doing nothing", () => {
  // A misspelled flag would be the exact bug class this module removes — a policy that looks
  // set and isn't. Fail loudly, at startup, where it is cheap to notice.
  assert.throws(() => defineTool("__bad__", { mutatesWorkspce: true }), /unknown tool policy field/);
  assert.throws(() => defineTool("", {}), /requires a tool type/);
});

test("policies are frozen so a call site cannot mutate shared policy by accident", () => {
  const p = toolPolicy("write");
  assert.throws(() => { "use strict"; p.needsApproval = false; }, TypeError);
  // allPolicies hands out copies, so diagnostics can't corrupt the registry either.
  const snapshot = allPolicies();
  snapshot.write.needsApproval = false;
  assert.equal(needsApproval("write"), true);
});

// ── The anti-drift test ─────────────────────────────────────────────────────

test("main.js no longer hand-maintains the tool family lists", () => {
  // The literal that appeared ELEVEN times. Once the call sites are derived, a new copy
  // appearing is a regression toward the thing this module replaced — so fail on it.
  const literalCopies = (MAIN.match(/"write",\s*"edit",\s*"multiedit",\s*"delete",\s*"move",\s*"mkdir",\s*"copy",\s*"format"/g) || []).length;
  assert.equal(literalCopies, 0,
    "the mutation-family list must come from tool-policy.js, not be re-typed at the call site");
  // The eleven-term read-only chain likewise.
  assert.doesNotMatch(MAIN, /readOnlyMode && \(call\.type === "write" \|\| call\.type === "edit"/,
    "the read-only-mode rule must come from blockedInReadOnlyMode()");
  // And main.js must actually be importing the module rather than keeping a parallel copy.
  assert.match(MAIN, /import \{[^}]*\} from "\.\/agent\/tool-policy\.js"/,
    "main.js must consume the registry");
});

// ── 只读模式里的 MCP：按服务自己的声明逐次判，不整类一刀切 ────────────────────
//
// 以前 mcp 类型是 readOnlyModeBlocked: true，于是 Plan / Explorer / Reviewer 里
// 用户装的 MCP 服务一个都用不了。可"查官方文档、读表结构、看 issue"恰恰是
// 先调研再动手最需要的东西——调研这一半反而没工具。
// 但也不能反过来全放：MCP 规范里 readOnlyHint 是**可选**的，多数服务不写；
// 缺声明时必须按"可能有副作用"处理，否则只读模式会替用户改了东西。
test("只读门必须收到整个 call——少传一个实参，MCP 又被一刀切挡回去而且全绿", () => {
  // 这里破例用源码断言（本文件开头反对钉源码形状，但 anti-drift 小节是它自己写明的例外）：
  // MCP 的只读判定是**逐次**的，policy 里是个 lambda。调用点写成 blockedInReadOnlyMode(call.type)
  // 的话，lambda 收到 undefined → !undefined === true → 只读模式里所有 MCP 全被挡，
  // 而下面那些直接调函数的行为测试照样通过。钉的是元数，不是变量名。
  assert.doesNotMatch(MAIN, /blockedInReadOnlyMode\(\s*[A-Za-z_$][\w$]*\.type\s*\)/,
    "只读门只收到了 type，MCP 的逐次判定退化成一刀切");
  assert.match(MAIN, /blockedInReadOnlyMode\(\s*[A-Za-z_$][\w$]*\.type\s*,\s*[A-Za-z_$][\w$]*\s*\)/,
    "main.js 必须把整个 call 交给只读门");
});

test("声明了只读的 MCP 工具，在只读模式里可以用", () => {
  assert.equal(blockedInReadOnlyMode("mcp", { type: "mcp", mcpReadOnly: true }), false);
});

test("没声明只读的 MCP 工具照旧挡住——缺声明按有副作用处理", () => {
  assert.equal(blockedInReadOnlyMode("mcp", { type: "mcp", mcpReadOnly: false }), true);
  assert.equal(blockedInReadOnlyMode("mcp", { type: "mcp" }), true, "没有这个字段时必须挡");
  assert.equal(blockedInReadOnlyMode("mcp", undefined), true, "连 call 都没有时必须挡");
});

test("其它类型不受影响：写文件和跑命令在只读模式里照旧禁止", () => {
  for (const t of ["write", "edit", "multiedit", "cmd"]) {
    assert.equal(blockedInReadOnlyMode(t), true, `${t} 不该在只读模式里放行`);
  }
  assert.equal(blockedInReadOnlyMode("read"), false);
});

test("MCP 在只读模式里放行，不等于不用审批", () => {
  // 两道门是独立的：readOnlyModeBlocked 管"这个模式能不能做这件事"，
  // needsApproval 管"要不要问用户"。放行第一道不该顺手关掉第二道。
  assert.ok(needsApproval("mcp"), "mcp 不再需要审批了");
});
