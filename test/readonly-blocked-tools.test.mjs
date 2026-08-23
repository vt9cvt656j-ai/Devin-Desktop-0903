// 只读模式的注册表里有 3 把打不开门的钥匙：ui_click / create_project / save_skill 的
// 策略是 readOnlyModeBlocked，执行时 100% 回 [BLOCKED]，却照样发给模型——
// search_tools 取得回、编排器选得中，选中一次就是一轮白烧，还占掉工具窗口的配额。
//
// 更糟的是 create_project：没打开工作区时，harness **无条件**在上下文里写
// 「先调 create_project 建一个目录……别停下来问用户」。只读模式下那是一条必然被拒的指令。
//
// 这和本仓库已经修过的「网页版别发桌面专属工具」是同一个形状（那份注释逐字写着
// "don't even offer them there"），只是只读模式这一份漏了。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, fnSource as topLevelFn } from "./helpers/source.mjs";
import * as pol from "../src/agent/tool-policy.js";

function registry(includeWrite) {
  const build = /function _buildAgentToolSchemas\([\s\S]*?\n\}/.exec(SRC);
  const dis = /function _withoutDisabledTools\([\s\S]*?\n\}/.exec(SRC);
  const helper = /function _readOnlyBlockedTool\([\s\S]*?\n\}/.exec(SRC);
  assert.ok(build && helper, "锚点函数抠不出来");
  const fn = new Function("inTauri", "_applyCloudToolDescs", "_userCapabilities", "compileToolSchema",
    "_applyUserRoleEnums", "toolPolicy",
    `${dis ? dis[0] : "const _withoutDisabledTools=(t)=>t;"}\n${helper[0]}\n${build[0]}\n;return _buildAgentToolSchemas;`)
    (true, (t) => t, () => ({ tools: [], commands: [], roles: [], disabled: [], errors: [] }), (t) => t, (t) => t, pol.toolPolicy);
  return fn(includeWrite, []).map((t) => t.function?.name).filter(Boolean);
}

const blockedIn = (names) => names.filter((n) =>
  pol.toolPolicy(String(n).replace(/_/g, "").toLowerCase())?.readOnlyModeBlocked === true);

test("只读注册表里一把打不开门的钥匙都没有", () => {
  const bad = blockedIn(registry(false));
  assert.deepEqual(bad, [],
    `这些工具在只读模式里发给了模型，执行时却 100% 被拒：${bad.join(",")}——`
    + "选中一次就是一轮白烧，还占掉工具窗口配额");
});

test("可写模式一个工具都没少", () => {
  // 这条守的是「别修过头」：那三个在 Agent 模式下是正常能力。
  const rw = registry(true);
  for (const n of ["ui_click", "create_project", "save_skill"]) {
    assert.ok(rw.includes(n), `${n} 在可写模式里也被过滤掉了——那是把正常能力删了`);
  }
  assert.ok(rw.length > registry(false).length, "两个模式的注册表一样大，过滤没生效");
});

test("过滤掉的恰好就是策略说必被拒的那些，不多不少", () => {
  const rw = new Set(registry(true));
  const ro = new Set(registry(false));
  const removed = [...rw].filter((n) => !ro.has(n));
  // 只读模式本来就比可写模式少一批写工具；这里只看**本条改动**移除的那几个。
  const shouldRemove = [...rw].filter((n) =>
    pol.toolPolicy(String(n).replace(/_/g, "").toLowerCase())?.readOnlyModeBlocked === true);
  for (const n of shouldRemove) {
    assert.ok(removed.includes(n), `${n} 该被过滤却还在`);
  }
});

test("判据是严格布尔，不是真值判断", () => {
  // 策略表里有一族 readOnly 相关字段是**函数**（按这一次调用判，如 browser 看页面不弹框、
  // 替用户按按钮才弹）。真值判断会把那一族一并误杀。
  const body = topLevelFn("_readOnlyBlockedTool", { code: true });
  assert.match(body, /readOnlyModeBlocked === true/,
    "用了真值判断——函数型策略字段恒为真，会误杀一批本来能用的工具");
  assert.doesNotMatch(body, /readOnlyBlockedTypes\(\)/,
    "用了 readOnlyBlockedTypes()：那个集合按真值收，含函数型字段");
});

test("名字归一化和策略表的键一致", () => {
  const body = topLevelFn("_readOnlyBlockedTool", { code: true });
  assert.match(body, /replace\(\/_\/g, ""\)\.toLowerCase\(\)/,
    "没做下划线归一——策略表的键是 createproject 不是 create_project，核不上就恒为假");
});

test("查不到策略时放行，不是拦掉", () => {
  const body = topLevelFn("_readOnlyBlockedTool", { code: true });
  assert.match(body, /catch \{ return false; \}/,
    "策略查不到就拦——那会在策略表出问题时把整个只读模式的工具清空");
});

// ── harness 自己也别去点名一个必被拒的工具 ────────────────────────────
test("只读模式下不再指示模型去调 create_project", () => {
  const send = topLevelFn("sendPrompt", { code: true });
  const at = send.indexOf('先调 create_project 建一个目录');
  assert.ok(at > 0, "那条指令不见了（可能被整段删了，那也不对——Agent 模式下它是对的）");
  const before = send.slice(Math.max(0, at - 500), at);
  assert.match(before, /_readOnlyBlockedTool\("create_project"\)/,
    "那条指令仍然无条件下发——只读模式下它指使模型去调一个必然被拒的工具");
  assert.match(before, /\["explorer", "reviewer", "plan"\]\.includes/,
    "没有按模式分流");
});

test("Agent 模式下那条指令原样保留", () => {
  const send = topLevelFn("sendPrompt", { code: true });
  assert.match(send, /先调 create_project 建一个目录/,
    "把这条指令整个删了——Agent 模式下「用户说写个机器人就该看到机器人」那条产品判断没变");
});

test("只读模式给的是可执行的替代，不是一句拒绝", () => {
  const send = topLevelFn("sendPrompt", { code: true });
  assert.match(send, /先回答用户能回答的部分/, "只说了不行，没说能做什么");
  assert.match(send, /切到 Agent 模式或先打开一个文件夹/, "没告诉用户怎么才能真的建这个项目");
});
