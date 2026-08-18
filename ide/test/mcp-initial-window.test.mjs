// MCP 工具被结构性地"永远不会被选中"。
//
// 实测：装好 context7、把服务名录连同每个工具的说明都报进了上下文，问一句
// 「shadcn 的 Sidebar 现在怎么用？查最新官方文档，别凭记忆」——它照旧用内建的
// web_fetch 抓网页，context7 一次都没调。
//
// 不是模型笨，是这局本来就不公平：web_fetch / http_request 就在开局工具窗口里，
// 零成本可调；同样能办成这件事的 MCP 工具却要先花一轮 search_tools 取回 schema。
// 两条路结果差不多的时候，"少一轮往返"永远赢。挡在窗口外的代价不是"晚一轮加载"，
// 是**永远轮不到**。
//
// 但当初把 MCP 挡在外面的理由同样是真的：一个 35 个工具的服务会把开局窗口挤爆。
// 所以按服务整体放行，小的先进，进不下的整个服务留给 search_tools。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

function topLevelFn(name) {
  const at = SRC.indexOf(`function ${name}(`);
  assert.ok(at > 0, `找不到 ${name}`);
  const end = SRC.indexOf("\n}\n", at);
  assert.ok(end > at, `${name} 没有行首收尾大括号`);
  return SRC.slice(at, end + 2);
}

// 和 main.js 的 agent 核心表保持一致。2026-08-18 扩窗：取外部资源那五个进了核心
// （用户点名——不进窗口就意味着"要多花一轮取 schema"，模型在难任务上永远不会选它们）。
// 这个文件守的是"MCP 不许挤爆窗口"，那个保证与扩窗无关，仍然成立。
const CORE = ["read_file", "list_dir", "search", "find_files", "update_plan", "ask_user",
              "write_file", "edit_file", "multi_edit", "run_cmd",
              "web_search", "web_fetch", "github_search", "developer_community_search", "package_search"];

function selector() {
  return new Function(
    "CORE",
    "const _SEARCH_TOOLS_SCHEMA = { type: 'function', function: { name: 'search_tools' } };" +
    "const _INITIAL_MCP_MAX_TOOLS = 8;" +
    "const _INITIAL_MCP_MAX_BYTES = 12000;" +
    "const _buildAgentToolSchemas = (w, mcp) =>" +
    // web_fetch 已经在 CORE 里（扩窗后），这里再列一次会让过滤结果出现重复项。
    "  [...CORE, 'http_request', 'browser'].map((n) => ({type:'function',function:{name:n}}))" +
    "    .concat(mcp || []);" +
    topLevelFn("_utf8ByteLength") +
    topLevelFn("_mcpServersForInitialWindow") +
    topLevelFn("_selectInitialTools") +
    "\n;return _selectInitialTools;",
  )(CORE);
}

const select = selector();
const mcp = (name, schemaBytes = 0) => ({
  type: "function",
  function: { name, description: "x".repeat(schemaBytes) },
});
const names = (list) => list.map((t) => t.function.name);

test("没有 MCP 时，开局工具表和以前一模一样", () => {
  // 这条守的是"改动不外溢"：绝大多数用户一个 MCP 都没配，他们的首包不该变胖一个字节。
  const out = names(select(true, "随便什么任务", []));
  assert.deepEqual(out, [...CORE, "search_tools"]);
});

test("小体量服务（context7 两个工具）直接进开局窗口", () => {
  const out = names(select(true, "查 shadcn 最新官方文档", [
    mcp("mcp__context7__resolve-library-id"),
    mcp("mcp__context7__query-docs"),
  ]));
  assert.ok(out.includes("mcp__context7__query-docs"),
    "还是被挡在窗口外——那 web_fetch 永远比它便宜一轮，它就永远不会被选中");
  assert.ok(out.includes("search_tools"), "search_tools 必须还在");
  assert.ok(out.includes("read_file"), "核心工具不能被挤掉");
});

test("35 个工具的服务没资格进开局窗口", () => {
  const many = [];
  for (let i = 0; i < 35; i++) many.push(mcp(`mcp__Michael-Cursor__tool_${i}`));
  const out = names(select(true, "随便什么任务", many));
  assert.deepEqual(out, [...CORE, "search_tools"], "大服务把开局窗口挤爆了");
});

test("大服务不会连累小服务——两个都在时，小的照样进", () => {
  // 「全有或全无」会造成一个很怪的耦合：用户一启用大服务，小服务的快车道就没了。
  const list = [mcp("mcp__context7__query-docs"), mcp("mcp__context7__resolve-library-id")];
  for (let i = 0; i < 35; i++) list.push(mcp(`mcp__Michael-Cursor__tool_${i}`));
  const out = names(select(true, "查文档", list));
  assert.ok(out.includes("mcp__context7__query-docs"), "小服务被大服务连累了");
  assert.ok(!out.some((n) => n.startsWith("mcp__Michael-Cursor__")), "大服务不该进来");
});

test("按服务整体放行，绝不放进半个服务", () => {
  // 只放进一部分工具，比一个都不放更糟：模型会把残缺的那半当成这个服务的全部能力。
  const list = [];
  for (let i = 0; i < 6; i++) list.push(mcp(`mcp__aaa__t${i}`));
  for (let i = 0; i < 6; i++) list.push(mcp(`mcp__bbb__t${i}`));
  const out = names(select(true, "任务", list));
  const a = out.filter((n) => n.startsWith("mcp__aaa__")).length;
  const b = out.filter((n) => n.startsWith("mcp__bbb__")).length;
  for (const [svc, n] of [["aaa", a], ["bbb", b]]) {
    assert.ok(n === 0 || n === 6, `服务 ${svc} 只进来了 ${n} 个工具——半个服务比不进更糟`);
  }
  assert.ok(a + b <= 8, "超出了条数预算");
});

test("schema 巨大的服务按字节预算拦下，不是只数条数", () => {
  // 两个工具、每个 10KB 的 schema，条数完全合规，字节上会把首包顶起来。
  const out = names(select(true, "任务", [
    mcp("mcp__fat__a", 10_000),
    mcp("mcp__fat__b", 10_000),
  ]));
  assert.ok(!out.some((n) => n.startsWith("mcp__fat__")), "只数条数拦不住体积");
});

test("放行是确定性的：同样输入、不同顺序，结果一致", () => {
  const list = [mcp("mcp__zzz__a"), mcp("mcp__aaa__a"), mcp("mcp__aaa__b")];
  const shuffled = [mcp("mcp__aaa__b"), mcp("mcp__zzz__a"), mcp("mcp__aaa__a")];
  assert.deepEqual(names(select(true, "任务", list)), names(select(true, "任务", shuffled)));
});

test("非 mcp__ 前缀的东西不会被这段逻辑误放进来", () => {
  // 举例不能用核心工具名：web_fetch 扩窗后进了 CORE，拿它当"假 MCP 工具"会把真正的
  // 核心项挤掉，测出来的就不是这条断言想守的东西了。
  const out = names(select(true, "任务", [mcp("plain_not_mcp"), mcp("mcp_no_double_underscore")]));
  assert.deepEqual(out, [...CORE, "search_tools"]);
});

test("上限是具名常量，不是散落的魔法数字", () => {
  assert.match(SRC, /const _INITIAL_MCP_MAX_TOOLS = \d+;/);
  assert.match(SRC, /const _INITIAL_MCP_MAX_BYTES = [\d_]+;/);
  const body = topLevelFn("_mcpServersForInitialWindow");
  assert.match(body, /_INITIAL_MCP_MAX_TOOLS/);
  assert.match(body, /_INITIAL_MCP_MAX_BYTES/);
});
