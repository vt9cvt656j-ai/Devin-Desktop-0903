// 跨会话工具经验：喂给**工具编排腿**（决定下一阶段装入哪些能力的那条认知腿）的
// 「过往同类场景经验」。它偏偏是最该随时间变聪明的一块，而它此前结构性地几乎恒为空。
//
// 旧结构：一份 **200 条的全局**原始事件数组，被所有场景签名和 141 个工具分摊；取用侧
// 还要求"同一签名同一工具 ≥3 条"才产生任何信号。一个忙碌的下午就能把之前的全挤光。
// 新结构：按 (场景签名, 工具) 聚合，一个工具的历史不会因为别的工具忙而被挤掉。
//
// 判据本身一个字没改，而且这里用**行为**守它，不是守实现形状：
// 45✓/3✗、最近全成的工具绝不能被打上"近期屡次失败"。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load, loadConst } from "./helpers/source.mjs";

function fakeStorage() {
  const bag = new Map();
  return {
    getItem: (k) => (bag.has(k) ? bag.get(k) : null),
    setItem: (k, v) => bag.set(k, String(v)),
    _raw: () => bag,
  };
}

const CLASSIFY = load("_classifyToolFailure");
const SIGNATURE = load("_buildScenarioSignature");
const CONST = {
  _TOOL_EXP_KEY: loadConst("_TOOL_EXP_KEY"),
  _TOOL_EXP_MAX_ENTRIES: loadConst("_TOOL_EXP_MAX_ENTRIES"),
  _TOOL_EXP_RECENT: loadConst("_TOOL_EXP_RECENT"),
};

function ledger(storage = fakeStorage(), now = { t: 1_700_000_000_000 }) {
  const fold = load("_toolExpFold", { ...CONST, _classifyToolFailure: CLASSIFY });
  const deps = {
    ...CONST,
    localStorage: storage,
    _perfPhase: () => {},
    _classifyToolFailure: CLASSIFY,
    _buildScenarioSignature: SIGNATURE,
    _toolExpFold: fold,
    Date: { now: () => now.t++ },
  };
  const loadTable = load("_toolExpLoad", deps);
  const full = { ...deps, _toolExpLoad: loadTable };
  return {
    record: load("_toolExpRecord", full),
    retrieve: load("_toolExpRetrieve", full),
    loadTable,
    storage,
  };
}

const PROFILE = { ui: true, bug: true, lang: "ts" };
const OTHER = { securityRisk: true, lang: "rust" };

test("同一场景里，成功与失败都记进同一条，取回来是聚合事实", () => {
  const L = ledger();
  for (let i = 0; i < 45; i++) L.record(PROFILE, "read_file", true);
  for (let i = 0; i < 3; i++) L.record(PROFILE, "read_file", false, "ENOENT: cannot stat");
  const text = L.retrieve(PROFILE);
  assert.match(text, /read_file: 45✓\/3✗/, "聚合计数不对");
});

test("45✓/3✗ 且最近全成的工具，绝不能被打成「近期屡次失败」", () => {
  // 这是那个老坑的正脸：曾经的分组键把成败拆开，fail 桶里「最近三条全失败」恒真，
  // 于是一个本来好用的工具被劝退，模型学到的是**反的**。
  const L = ledger();
  for (let i = 0; i < 3; i++) L.record(PROFILE, "search", false, "timeout");
  for (let i = 0; i < 45; i++) L.record(PROFILE, "search", true);
  const text = L.retrieve(PROFILE);
  assert.match(text, /search: 45✓\/3✗/);
  assert.doesNotMatch(text, /屡次失败/,
    "最近 45 次全成的工具被打成近期屡次失败——模型会绕开一个本来好用的工具");
});

test("最近三次真的全失败时，警告要出来", () => {
  const L = ledger();
  for (let i = 0; i < 20; i++) L.record(PROFILE, "web_fetch", true);
  for (let i = 0; i < 3; i++) L.record(PROFILE, "web_fetch", false, "network unreachable");
  assert.match(L.retrieve(PROFILE), /web_fetch[^\n]*屡次失败/,
    "连续三次失败没有被标出来——这条经验就白记了");
});

test("失败不足三次不下结论", () => {
  const L = ledger();
  L.record(PROFILE, "git_diff", true);
  L.record(PROFILE, "git_diff", false, "not a git repository");
  L.record(PROFILE, "git_diff", false, "not a git repository");
  assert.doesNotMatch(L.retrieve(PROFILE), /屡次失败/, "两次失败就下结论太急了");

  // 只有两条、而且两条都失败——这一例才真的量到那个 3。上面那例的尾部不是"全失败"，
  // 把阈值从 3 调到 1 它照样绿（变异实测），等于没守住这个数。
  const L2 = ledger();
  L2.record(PROFILE, "lsp_hover", false, "no language server");
  L2.record(PROFILE, "lsp_hover", false, "no language server");
  assert.doesNotMatch(L2.retrieve(PROFILE), /屡次失败/,
    "才两次失败就劝模型换方案——一个刚开始用的工具会被直接劝退");
});

test("失败类别与建议要带出来，模型才知道下一步换什么打法", () => {
  const L = ledger();
  for (let i = 0; i < 3; i++) L.record(PROFILE, "read_file", false, "ENOENT: cannot stat '/x'");
  const text = L.retrieve(PROFILE);
  assert.match(text, /失败 3 次/, "失败类别的次数没带出来");
  assert.match(text, /建议/, "只报了失败、没给下一步建议");
});

// ── 结构性那半：这才是这次改动真正修的东西 ────────────────────────────────
test("别的工具忙一整天，也挤不掉这个工具在这类场景的历史", () => {
  // 旧结构在这里必然失败：200 条全局缓冲，写 300 条别的工具就把前面的全挤光。
  const L = ledger();
  for (let i = 0; i < 3; i++) L.record(PROFILE, "package_search", false, "network unreachable");
  for (let i = 0; i < 300; i++) L.record(PROFILE, `noise_tool_${i % 40}`, true);
  assert.match(L.retrieve(PROFILE, 60), /package_search[^\n]*屡次失败/,
    "别的工具一忙，这个工具的历史就被挤没了——「过往同类场景经验」于是恒为空");
});

test("别的场景签名忙一整天，也挤不掉本场景的历史", () => {
  const L = ledger();
  for (let i = 0; i < 3; i++) L.record(PROFILE, "run_cmd", false, "permission denied");
  for (let i = 0; i < 300; i++) L.record(OTHER, `other_${i % 40}`, true);
  assert.match(L.retrieve(PROFILE, 60), /run_cmd[^\n]*屡次失败/,
    "换个场景忙一阵子，本场景的经验就没了");
});

test("场景之间不串味", () => {
  const L = ledger();
  for (let i = 0; i < 5; i++) L.record(PROFILE, "read_file", true);
  for (let i = 0; i < 5; i++) L.record(OTHER, "cargo_check", true);
  assert.doesNotMatch(L.retrieve(PROFILE), /cargo_check/, "别的场景的经验混进来了");
  assert.doesNotMatch(L.retrieve(OTHER), /read_file/);
});

test("表满了按最近用到的时间淘汰整对，不砍半截历史", () => {
  // 旧结构满了砍掉最老的**一条事件**，会把一个工具的历史砍成半截，聚合计数无声失真。
  const L = ledger();
  const cap = CONST._TOOL_EXP_MAX_ENTRIES;
  for (let i = 0; i < cap + 50; i++) L.record({ lang: `l${i}` }, "t", true);
  const table = L.loadTable();
  assert.ok(Object.keys(table).length <= cap, `表涨到 ${Object.keys(table).length} 条`);
  // 最后写进去的那个场景必须还在——淘汰的是最久没用到的。
  assert.match(L.retrieve({ lang: `l${cap + 49}` }), /^t: 1✓/m, "刚写的那条被自己淘汰了");
});

test("「最近用到」按时间算，不是按名字或写入顺序", () => {
  // 上一条量不到这个：那些 key 恰好字典序和时间序一致，把排序换成 keys.sort() 照样绿。
  // 这里让一个**早就写过**的场景在末尾被再次用到——它必须活下来。
  const L = ledger();
  const early = { lang: "aaa_first" };          // 字典序最小 = 最先被 keys.sort() 砍掉
  L.record(early, "t", true);
  const cap = CONST._TOOL_EXP_MAX_ENTRIES;
  for (let i = 0; i < cap - 2; i++) L.record({ lang: `zz${i}` }, "t", true);
  L.record(early, "t", true);                    // 再次用到 → 时间序上它最新
  for (let i = 0; i < 40; i++) L.record({ lang: `zzz_late${i}` }, "t", true); // 触发淘汰
  assert.match(L.retrieve(early), /^t: 2✓/m,
    "刚刚用过的场景被淘汰了——淘汰算的是名字或写入顺序，不是最近用到的时间");
});

test("旧格式（原始事件数组）能就地折叠迁移，不丢已有数据", () => {
  const storage = fakeStorage();
  const sig = SIGNATURE(PROFILE);
  storage.setItem(CONST._TOOL_EXP_KEY, JSON.stringify([
    { sig, tool: "read_file", ok: true, ts: 1 },
    { sig, tool: "read_file", ok: true, ts: 2 },
    { sig, tool: "read_file", ok: false, ts: 3, failCategory: "not_found", failDetail: "ENOENT" },
  ]));
  const L = ledger(storage);
  assert.match(L.retrieve(PROFILE), /read_file: 2✓\/1✗/, "升级把用户已有的经验丢了");
});

test("存储坏掉时安静退回，不能把编排腿带崩", () => {
  const storage = fakeStorage();
  storage.setItem(CONST._TOOL_EXP_KEY, "{ 这不是 JSON");
  const L = ledger(storage);
  assert.equal(L.retrieve(PROFILE), "");
  L.record(PROFILE, "read_file", true); // 不许抛
  assert.match(L.retrieve(PROFILE), /read_file: 1✓/, "坏数据之后写不进新的了");
});

test("limit 砍掉的是最不相关的，不是碰巧先写进表的", () => {
  const L = ledger();
  for (let i = 0; i < 10; i++) L.record(PROFILE, `tool_${i}`, true);
  const text = L.retrieve(PROFILE, 3);
  assert.equal(text.split("\n").length, 3);
  assert.match(text, /tool_9/, "最近用到的那个被 limit 砍掉了");
  assert.doesNotMatch(text, /tool_0\b/, "最久没用的那个反而留下来了");
});
