// 工具卡的红绿必须和台账同源。
//
// _settleToolStep 是所有「执行器自己没画完」的卡片的兜底：unknown / skipped / interrupted /
// 异常，以及 search_tools、debate、批次拦截这些 harness 自产文案的路径。它原来对**整段正文**
// 扫裸词判失败：
//
//   const failed = /\[(?:ERROR|BLOCKED|DENIED|失败|不可用|interrupted|未执行)\]|失败|缺参数|未知工具|已停止/i
//
// 交替项里那几个不带方括号的裸词作用在整段正文上，于是 search_tools 走「无匹配」分支时，
// 文案里那句「这**不是检索失败**，是内置工具里没有专用的」命中裸词「失败」，一张成功卡被涂成
// 红色并加上 agent-tool-step--rejected（变淡 + 路径删除线）。同一次调用在 _toolExecutionSucceeded
// 那边记的是 ok=true、计划照常推进——界面说「被拒了」，台账说「成功了」，用户按界面排查，方向全反。
// 反向也漏：裸词表里没有 CONFLICT / NEEDS_REPO / 权限问题（首行标记表里有），那几种真失败
// 只要走到兜底就画成绿色「完成」。
//
// 这里守的是行为：红绿由结构化事实决定（failure.code / ok:false / cmd 退出码 / 正文**首行**的
// 方括号标记），和 _toolExecutionSucceeded 对同一次调用给出同一个结论；label 只管文字。
import test from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, load, fnSource } from "./helpers/source.mjs";
import { workspaceMutatingTypes } from "../src/agent/tool-policy.js";
import { setBadgeText } from "../src/agent/escape.js";

const settle = load("_settleToolStep", { _collapseSettledToolSteps: () => {}, _setBadge: setBadgeText });

/** 一张还在转圈的工具卡的最小替身；返回落定后的类名与 step 上加的类。 */
function settleCard(result, label = "") {
  let textContent = "";
  const classes = new Set();
  const resultEl = {
    className: "atc-result",
    querySelector: (selector) => (selector === ".atc-spin" && !textContent ? {} : null),
    get textContent() { return textContent; },
    set textContent(value) { textContent = value; },
  };
  const step = {
    dataset: {},
    classList: { add: (name) => classes.add(name) },
    querySelector: (selector) => (selector === ".atc-result" ? resultEl : null),
  };
  const changed = settle(step, result, label);
  return {
    changed,
    ok: resultEl.className.includes("atc-result--ok"),
    err: resultEl.className.includes("atc-result--err"),
    rejected: classes.has("agent-tool-step--rejected"),
    text: textContent,
  };
}

const succeeded = load("_toolExecutionSucceeded", {
  _WORKSPACE_MUTATING_TYPES: workspaceMutatingTypes(),
  _toolFailureMarkerAtHead: load("_toolFailureMarkerAtHead"),
  _toolFailureMatch: load("_toolFailureMatch"),
});

// 主循环里 search_tools 「无匹配」分支的原文（照抄，不改一个字）。
const NO_MATCH_CONTENT = "语义工具调度本次不可用，且模糊匹配无命中。这**不是检索失败，是内置工具里没有专用的**"
  + "——注册表是起手包，不是能力边界。别再换词空搜，按目标改走组合路径：读文件/搜索/命令。";

test("正文里出现「失败」二字不等于这次失败：卡片和台账必须给同一个结论", () => {
  const card = settleCard({ type: "search_tools", path: "", content: NO_MATCH_CONTENT }, "无匹配");
  assert.equal(card.ok, true, "「无匹配」是检索结论，不是执行失败——卡片不能画红");
  assert.equal(card.rejected, false, "更不能打上 rejected：那在界面上等于「这次调用被拒绝了」");
  assert.equal(card.text, "无匹配", "语义由 label 承担，和红绿解耦");
  // 台账侧对同一次调用的结论。
  assert.equal(succeeded({ type: "search_tools" }, { content: NO_MATCH_CONTENT }), true);
});

test("MCP 发现失败的检索结果同样是一次成功的检索", () => {
  const content = "没有找到匹配的新工具；部分 MCP 服务在后台发现时失败。\n\n· fs: 连接超时";
  const card = settleCard({ type: "search_tools", path: "", content }, "MCP 连接失败");
  assert.equal(card.err, false, "裸词「失败」不再决定红绿");
  assert.equal(succeeded({ type: "search_tools" }, { content }), true, "台账也判成功");
});

test("读到的文件正文/日志正文里的失败字样不该把成功调用画成红卡", () => {
  const fileBody = "function report() {\n  // 部署失败时要回滚\n  if (!ok) throw new Error('已停止');\n}";
  assert.equal(settleCard({ type: "read", path: "a.js", content: fileBody }).ok, true);
  const logBody = "〔外部数据〕\n2026-08-22 10:00:01 INFO started\n2026-08-22 10:00:02 [ERROR] retry\n";
  assert.equal(settleCard({ type: "logs", path: "app.log", content: logBody }).ok, true,
    "只认首行标记：日志正文里的 [ERROR] 是它的内容，不是它的结局");
});

test("首行的方括号标记仍然一律判红——带 label 的调用点一个都不许回退成绿勾", () => {
  const cases = [
    [{ content: "[ERROR] 未知工具: db_query。" }, "未知工具"],
    [{ content: "[ERROR] debate 需要 question 参数。" }, "缺少 question"],
    [{ content: "[BLOCKED] 只读子智能体只能运行纯探索命令。" }, "已拒绝"],
    [{ content: "[interrupted]" }, "已停止"],
    [{ content: "[未执行]" }, "未执行"],
    [{ content: "[失败] michael-design 预取异常: timeout" }, "预取失败"],
    [{ content: "[BLOCKED_MUTATION_BATCH] 同一补丁中前一个修改没有成功。", ok: false, failure: { code: "mutation_batch" } }, "前项真实失败 · 后续已停止"],
  ];
  for (const [result, label] of cases) {
    const card = settleCard(result, label);
    assert.equal(card.err, true, `${label} 必须是红卡`);
    assert.equal(card.rejected, true, `${label} 必须带 rejected 样式`);
    assert.equal(card.text, label, "红绿变了，文字仍然由 label 说了算");
  }
});

test("裸词表漏掉的那几种真失败，改判后不再画成绿色「完成」", () => {
  for (const head of [
    "[CONFLICT] 文件已被他人修改",
    "[NEEDS_REPO] 当前目录不是 git 仓库",
    "[权限问题] 无法写入 /etc/hosts",
    "[DENIED] 用户拒绝了这次调用",
  ]) {
    const card = settleCard({ type: "git", path: "", content: `${head}\n详情见上。` });
    assert.equal(card.err, true, `${head} 走到兜底时必须是红卡`);
  }
});

test("结构化结局优先于文案：ok:false / failure.code 一律红，哪怕正文一句失败都没有", () => {
  assert.equal(settleCard({ type: "http", path: "", ok: false, content: "响应已保存到缓存。" }).err, true);
  assert.equal(settleCard({ type: "write", path: "a.js", failure: { code: "read_before_edit" }, content: "请先读取当前文件。" }).err, true);
});

test("cmd 的退出码优先于文案，和 _toolExecutionSucceeded 同序", () => {
  const green = { type: "cmd", path: "npm test", code: 0, content: "PASS 42 tests\n[ERROR] deprecation warning printed by the runner" };
  assert.equal(settleCard(green).ok, true, "退出 0 的构建/测试在输出里印 [ERROR] 是家常便饭");
  assert.equal(succeeded({ type: "cmd" }, green), true);
  const red = { type: "cmd", path: "npm run build", code: 1, content: "done in 3s" };
  assert.equal(settleCard(red).err, true, "退出码非 0 就是失败，正文再干净也一样");
  assert.equal(succeeded({ type: "cmd" }, red), false);
});

test("跳过类结果不带任何标记词，仍然是绿卡", () => {
  const card = settleCard({ content: "[重复读取·已跳过]" }, "重复 · 已跳过");
  assert.equal(card.ok, true);
  assert.equal(card.rejected, false);
});

test("已经落定过的卡片不被兜底重画", () => {
  let textContent = "12 行 · 3.4 KB";
  const resultEl = {
    className: "atc-result atc-result--info",
    querySelector: () => null,
    get textContent() { return textContent; },
    set textContent(value) { textContent = value; },
  };
  const step = { dataset: {}, classList: { add: () => {} }, querySelector: (s) => (s === ".atc-result" ? resultEl : null) };
  assert.equal(settle(step, { type: "read", content: "……失败……" }), false);
  assert.equal(resultEl.className, "atc-result atc-result--info", "执行器自己画好的卡片不许被兜底覆盖");
});

test("卡片判据不再对整段正文扫裸词", () => {
  const src = fnSource("_settleToolStep", { code: true });
  assert.doesNotMatch(src, /\|失败\|缺参数\|未知工具\|已停止/,
    "裸子串判据回来了：正文里出现这些词的成功调用会被涂红");
  assert.match(src, /split\("\\n", 1\)/, "必须先切出首行再判标记");
  assert.match(src, /result\?\.failure && result\.failure\.code/, "结构化结局必须排在文案前面");
  assert.match(src, /result\?\.ok === false/);
});

test("内联的首行标记词表不许和 _toolFailureMarkerAtHead 漂移", () => {
  // _settleToolStep 会被 test/logic.test.mjs 用 load() 单独取出来跑（只注入
  // _collapseSettledToolSteps），所以它不能引用 _toolFailureMarkerAtHead，只能内联一份。
  // 内联就会漂：这条把两份词表钉成同一串。
  const words = (fn) => {
    const hit = /\(失败\|[^)]*\)/.exec(fn);
    assert.ok(hit, "找不到首行标记的词表");
    return hit[0];
  };
  assert.equal(
    words(fnSource("_settleToolStep", { code: true })),
    words(fnSource("_toolFailureMarkerAtHead", { code: true })),
    "卡片和台账认的失败标记词必须逐字相同",
  );
  // 两份实现对同一批正文必须给同一个答案。
  const atHead = load("_toolFailureMarkerAtHead");
  for (const content of [
    "[ERROR] x", "[BLOCKED] y", "[CONFLICT] z", "[NEEDS_REPO] w", "[权限问题] v",
    "〔外部数据〕\n[ERROR] 首行前面有抬头", "第一行没事\n[ERROR] 第二行才有",
    "[重复读取·已跳过]", "正文里写着失败两个字", "[interrupted]", "[未执行]",
  ]) {
    assert.equal(settleCard({ content }).err, atHead(content), `判定漂移: ${content.slice(0, 24)}`);
  }
});

test("这条判据只在兜底路径上生效，主批次的调用签名没变", () => {
  // test/logic.test.mjs 钉着 `_settleToolStep(step, result);` 的字面量（计划推进紧随其后），
  // 子体路径同形。判据换成结构化的同时不能顺手改签名。
  assert.match(SRC, /_settleToolStep\(step, result\);/);
  assert.match(SRC, /function _settleToolStep\(step, result, label = ""\) \{/);
});
