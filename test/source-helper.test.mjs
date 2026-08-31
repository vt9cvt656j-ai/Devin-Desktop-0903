// test/helpers/source.mjs 是「从 src/main.js 按名字取源码」的**唯一**一份提取器。
// 这个文件守住那个「唯一」，以及提取器自身的几条性质。
//
// 在此之前 test/ 下有 16 份手抄副本、4 种互不相同的语义。它们不是重复代码那么简单：
//   · 朴素计括号那几份不认字符串字面量。实测 capabilities-wiring 用它抽 _executeToolStepInner，
//     抽到 1,040,765 字节，真函数只有 406,715——main.js 里一句 `t.startsWith("{")` 让计数差 1，
//     于是多跑了一万四千行。那个用例当时仍然绿，纯属它断言的分支恰好落在真函数体内。
//   · 按 indexOf(`function 名(`) 定位的那几份会把 `async ` 前缀切掉：拼出来的代码里
//     await 是语法错误，或者更糟——签名断言看不见 async。
//   · 找 "\n}\n" 的那几份挑的是排版，不是语法。
//   · 于是修一个提取 bug 要同时改九个文件，历史上就出现过「只修了 mcp/skills 那两份」的分叉。
import test from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import * as acorn from "acorn";
import { SRC, CODE, OVERRIDES, fnSource, blockFrom, load, loadConst, stripComments } from "./helpers/source.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const TEST_FILES = readdirSync(HERE).filter((f) => f.endsWith(".test.mjs"));

// ---------------------------------------------------------------------------
// 1. 提取器本身
// ---------------------------------------------------------------------------

test("CODE 和 SRC 逐字节对齐：AST 的 offset 在两边通用", () => {
  // 这条不是形式主义。约 420 处断言的写法是「用 SRC(原文) 定位、在 CODE 上切片」，
  // 长度差一个字符，之后每一处切出来的都是错位的代码。用码点展开字符串（`[...SRC]`）
  // 就会踩这个坑：main.js 里有 emoji，acorn 的 offset 是 UTF-16 码元。
  assert.equal(CODE.length, SRC.length, "剥注释改变了长度，offset 全部作废");
  assert.equal(CODE.split("\n").length, SRC.split("\n").length, "剥注释吃掉了换行，行号对不上");
  let mismatched = 0;
  for (let i = 0; i < SRC.length; i++) if (CODE[i] !== " " && CODE[i] !== SRC[i]) mismatched++;
  assert.equal(mismatched, 0, "CODE 里的非空白字符和 SRC 对不上");
});

test("函数体里失衡的 `}` 字符串字面量不会让提取跑飞", () => {
  // 这就是朴素计括号版翻车的那一处：main.js 里有 `t.startsWith("{")`，括号计数差 1。
  const real = fnSource("_executeToolStepInner");
  assert.ok(real.includes('startsWith("{")'), "挑错了样本：这个函数里已经没有失衡的括号字面量了");

  // 朴素版当场复现一遍，证明这条断言守的是真的东西，而不是一句空话。
  const i = SRC.indexOf("function _executeToolStepInner(");
  let depth = 0, j = SRC.indexOf("{", SRC.indexOf(")", i));
  for (; j < SRC.length; j++) {
    const c = SRC[j];
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  const naive = SRC.slice(i, j + 1);
  assert.ok(naive.length > real.length * 2,
    "朴素计括号版今天不再跑飞了——那这条对照断言该重挑样本，而不是删掉");

  // 真函数必须是一段能独立解析的完整代码。
  acorn.parse(real, { ecmaVersion: "latest", sourceType: "module" });
});

test("async 前缀不会被切掉，const 声明也认", () => {
  assert.match(fnSource("_readyAiConfig"), /^async function _readyAiConfig\(/,
    "async 前缀被切掉了：拼出来的代码里 await 会是语法错误");
  assert.match(fnSource("_INITIAL_MCP_MAX_TOOLS"), /^const _INITIAL_MCP_MAX_TOOLS = /);
});

test("{ code: true } 切的是剥了注释的那一份，边界一模一样", () => {
  const raw = fnSource("_readyAiConfig");
  const code = fnSource("_readyAiConfig", { code: true });
  assert.equal(code.length, raw.length, "两份的边界不一致，行号和 offset 就对不上了");
  assert.ok(raw.includes("//"), "挑错了样本：这个函数里没有注释，对照说明不了问题");
  assert.doesNotMatch(code, /\/\/ /, "注释没被剥掉");
});

test("找不到的名字要当场报错，不能返回一段别人的代码", () => {
  assert.throws(() => fnSource("_这个函数不存在_zzz"), /找不到/);
});

test("loadConst 取的是 main.js 里的真值", () => {
  const tools = loadConst("_INITIAL_MCP_MAX_TOOLS");
  assert.equal(typeof tools, "number");
  assert.match(SRC, new RegExp(`const _INITIAL_MCP_MAX_TOOLS = ${tools};`),
    "loadConst 求出来的值和源码里写的对不上");
});

// ---------------------------------------------------------------------------
// 2. OVERRIDES：把生产常量在依赖解析这一层换掉
// ---------------------------------------------------------------------------

test("退避间隔在测试里被换成 1ms，而生产值原样不动", () => {
  // model-resume 那条「没出过字 → 走重试」的用例守的是走哪条分支，和退避时长无关，
  // 却因为依赖是从 main.js 原样抓的而真睡一次随机 1–2 秒（equal jitter 取 [base/2, base]），
  // 是整个套件最慢的一条，而且每次时长都不一样。退避曲线本身在 logic.test.mjs 里另有专测。
  assert.match(SRC, /const _AI_MODEL_RETRY_DELAY_MS = 2_000;/,
    "生产常量被改了——OVERRIDES 里那条要么跟着更新，要么已经没有意义了");
  assert.equal(OVERRIDES._AI_MODEL_RETRY_DELAY_MS, 1);
  assert.equal(load("_AI_MODEL_RETRY_DELAY_MS", ["_AI_MODEL_RETRY_DELAY_MS"]), 1,
    "覆盖没有发生在依赖解析这一层");
  // 不在表里的名字照旧抓真源。
  assert.equal(
    load("_INITIAL_MCP_MAX_TOOLS", ["_INITIAL_MCP_MAX_TOOLS"]),
    loadConst("_INITIAL_MCP_MAX_TOOLS"),
  );
});

test("model-resume 的依赖解析走 helper 的 load，覆盖才够得着它", () => {
  const t = readFileSync(join(HERE, "model-resume.test.mjs"), "utf8");
  assert.match(t, /from "\.\/helpers\/source\.mjs"/, "又抄了一份本地提取器");
  assert.match(t, /load\("_runModelRequestWithRetry", need\)/,
    "构造改回了本地拼接：OVERRIDES 够不着，那条用例会重新真睡 1–2 秒");
});

// ---------------------------------------------------------------------------
// 3. 不许再抄一份
// ---------------------------------------------------------------------------

test("没有任何测试文件自己定义 extractFn / topLevelFn", () => {
  const offenders = [];
  for (const f of TEST_FILES) {
    const t = readFileSync(join(HERE, f), "utf8");
    for (const m of t.matchAll(/(?:^|\n)\s*(?:(?:async\s+)?function\s+|const\s+|let\s+)(extractFn|topLevelFn|extractTopLevelFn)\b(?!\s*[,;)}])/g)) {
      offenders.push(`${f}: ${m[1]}`);   // import 里的重命名（`fnSource as extractFn`）不算本地定义
    }
  }
  assert.deepEqual(offenders, [],
    "本地提取器回来了。想改提取行为就改 test/helpers/source.mjs 一处，别再各写各的");
});

test("读 src/main.js 的测试不许自己手写按名字抠函数体的提取器", () => {
  // 判据是**结构**，不是名字：一个函数里同时出现「按名字找 `function 名(`」和「配平大括号
  // 或者找 \n}\n 收尾」，它就是又一份手抄提取器，叫什么都一样。
  //
  // 只按名字扫会漏，而且这次就是这么漏的：control-glow 的那份叫 extractTopLevelFn、
  // package-source 里那份叫 fnSrc、intent-timing 那份叫 extractSimpleFn 且用 new RegExp
  // 拼名字——按名字 grep 一个都扫不出来。
  const byName = /`function \$\{[\w.?]+\}\(`|function\\\\s\+\$\{[\w.?]+\}/;
  const balanced = [/"\\n\}\\n"/, /=== "\}"\)\s*\{?\s*depth--/, /depth--;\s*if \(!?depth/];
  const offenders = [];

  for (const f of TEST_FILES) {
    const t = readFileSync(join(HERE, f), "utf8");
    if (!t.includes("src/main.js") && !t.includes('"src", "main.js"') && !t.includes("helpers/source.mjs")) continue;
    const ast = acorn.parse(t, { ecmaVersion: "latest", sourceType: "module" });
    const seen = new Set();
    const walk = (node) => {
      if (!node || typeof node !== "object" || seen.has(node)) return;
      if (Array.isArray(node)) { for (const c of node) walk(c); return; }
      if (typeof node.type !== "string") return;
      seen.add(node);
      if (node.type === "FunctionDeclaration" || node.type === "FunctionExpression"
        || node.type === "ArrowFunctionExpression") {
        const body = t.slice(node.start, node.end);
        if (byName.test(body) && balanced.some((re) => re.test(body))) {
          offenders.push(`${f}:${t.slice(0, node.start).split("\n").length}`);
        }
      }
      for (const key of Object.keys(node)) {
        if (key === "type" || key === "start" || key === "end") continue;
        walk(node[key]);
      }
    };
    walk(ast);
  }

  // 这个文件自己有一份朴素版——它是对照样本，就写在「不会跑飞」那条断言里，不是副本。
  assert.deepEqual(offenders.filter((o) => !o.startsWith("source-helper.test.mjs")), [],
    "又出现了手抄的函数提取器。改用 test/helpers/source.mjs 的 fnSource——"
    + "手抄那几种要么不认字符串字面量里的括号、要么切掉 async 前缀，会静默抠出错的代码");
});

// ── blockFrom：fnSource 够不着的锚点（调度分支 / 回调注册 / 局部块）────────────
// 这些地方历来写成 `SRC.slice(RAW_SRC.indexOf(锚), RAW_SRC.indexOf(锚) + 2000)`。
// 两个毛病：固定字符数（被守的那段一变长，窗口尾部就滑出去，断言从此守别的东西）、
// 锚点不唯一时 indexOf 闷声挑第一个（本仓有个锚点出现 16 次）。

test("blockFrom 按 AST 边界取整块，长度不受固定窗口限制", () => {
  const browser = blockFrom('} else if (call.type === "browser") {', { code: true });
  assert.ok(browser.startsWith("{"), "取到的应当是这个分支的块本身");
  assert.ok(browser.length > 30000, `browser 分支有三万多字，取到 ${browser.length} 说明被截了`);
  // 相邻分支各取各的，不会串到一起。
  const mcp = blockFrom('} else if (call.type === "mcp") {', { code: true });
  assert.ok(mcp.length > 5000 && mcp.length < browser.length);
  assert.ok(!mcp.includes('call.type === "browser"'), "串到隔壁分支去了");
});

test("锚点不唯一就当场抛错，不许闷声挑第一个", () => {
  // `card.innerHTML =` 在源码里出现 16 次；indexOf 会挑第一个，而那多半不是你要的。
  assert.throws(() => blockFrom("card.innerHTML ="), /出现 \d+ 次/,
    "锚点不唯一却没抛错——这正是「按下标切源码」最会骗人的地方");
  // 确实要第 n 个时可以显式说。
  assert.ok(blockFrom("card.innerHTML =", { nth: 0 }).length > 0);
});

test("锚点找不到要抛错，不能返回一段别人的代码", () => {
  // indexOf 找不到会返回 -1，slice(-1) 只剩最后一个字符——断言随之变成守空气。
  assert.throws(() => blockFrom("这段源码里绝对不存在的锚点"), /找不到锚点/);
});

test("blockFrom 的 code:true 和 SRC 版边界一致（只是注释被抹成空格）", () => {
  const a = blockFrom('} else if (call.type === "system") {');
  const b = blockFrom('} else if (call.type === "system") {', { code: true });
  assert.equal(a.length, b.length, "两份文本逐字节对齐，边界必须一模一样");
});

// ── 「按下标切源码」的棘轮 ────────────────────────────────────────────────
// 判据只有一个、不需要判断：**它变多了没有**。
//
// 为什么是棘轮而不是白名单：这类写法现在有 167 处，散在 18 个文件。逐条写进白名单
// 要做 167 次「它到底守得住守不住」的判断，而那张表接下来只会烂掉。棘轮只回答
// 「有没有新增」，迁移过程中基线跟着往下走，方向是单调的，谁也不用维护理由。
//
// 为什么这类写法要往下走（三条实测）：
//   · 固定字符数：被守的那段一变长，窗口尾部滑出去。wiring 里守 _deliveryFactsLine
//     的那条就是这么红的——我往函数里加了几行注释，锚点从 1988 挪到 2028，窗口 2000。
//     **红了算走运**，同一形状悄悄绿着才是常态。
//   · 锚点不唯一：`card.innerHTML =` 在源码里出现 16 次，indexOf 闷声挑第一个。
//   · 锚点找不到：indexOf 回 -1，slice(-1) 只剩一个字符，断言从此守空气。
// 改法：函数用 fnSource(名字)，分支/回调/局部块用 blockFrom(锚点)——两个都按 AST
// 取边界，且锚点不唯一/找不到会**当场抛错**而不是猜。
const SLICE_BASELINE = {
  "ambiguous-failure.test.mjs": 1,
  "comment-quality.test.mjs": 2,
  "control-glow.test.mjs": 3,
  "desktop-automation.test.mjs": 2,
  "gate-tristate.test.mjs": 1,
  "interrupt-transcript.test.mjs": 1,
  "knowledge-preflight-card.test.mjs": 1,
  "knowledge-routing.test.mjs": 1,
  "logic.test.mjs": 89,
  
  "mcp.test.mjs": 7,
  "prefix-cache.test.mjs": 14,
  "runtime-state-cache.test.mjs": 1,
  "search-tools-routing.test.mjs": 1,
  "skill-catalog.test.mjs": 7,
  "skills.test.mjs": 2,
  "truthfulness.test.mjs": 1,
  "wiring.test.mjs": 26,
};

/** 每个测试文件里「SRC.slice(SRC.indexOf(…))」的条数。先剥注释——注释里会**讲**这个
 *  写法（本文件上面就有），不剥的话删一句注释都算"修好了一处"。 */
function sliceCensus() {
  const out = {};
  for (const f of readdirSync(HERE).filter((n) => n.endsWith(".mjs"))) {
    const s = stripComments(readFileSync(join(HERE, f), "utf8"));
    const n = (s.match(/(?:RAW_)?SRC\.slice\(\s*(?:RAW_)?SRC\.indexOf\(/g) || []).length;
    if (n) out[f] = n;
  }
  return out;
}

test("按下标切源码的写法只许减少，不许增加", () => {
  const now = sliceCensus();
  const grew = Object.entries(now)
    .filter(([f, n]) => n > (SLICE_BASELINE[f] || 0))
    .map(([f, n]) => `${f}: ${SLICE_BASELINE[f] || 0} → ${n}`);
  assert.deepEqual(grew, [],
    `这些文件新增了「按下标切源码」的守卫：\n  ${grew.join("\n  ")}\n`
    + "函数用 fnSource(名字)，分支/回调/局部块用 blockFrom(锚点)——按 AST 取边界，"
    + "锚点不唯一或找不到会当场抛错，而不是闷声守错地方。");

  // 反向：修好了就要把基线跟着降下来，否则那一格永远绿着，别人后来又加回两条也不会红。
  const stale = Object.entries(SLICE_BASELINE)
    .filter(([f, n]) => (now[f] || 0) < n)
    .map(([f, n]) => `${f}: 基线 ${n}，实际 ${now[f] || 0}`);
  assert.deepEqual(stale, [],
    `基线高于实际，说明这些已经迁过了，把 SLICE_BASELINE 调到实际值锁住成果：\n  ${stale.join("\n  ")}`);
});

test("这个普查器本身没坏（不许量出 0 条还报通过）", () => {
  const now = sliceCensus();
  const total = Object.values(now).reduce((a, b) => a + b, 0);
  assert.ok(total >= 100,
    `全仓只量到 ${total} 条——普查器坏了（正则或剥注释），而它坏掉的表现恰好是「一切干净」。`);
  // 阳性对照：一处**当前确实存在**的写法必须被数到。
  const wiring = stripComments(readFileSync(join(HERE, "wiring.test.mjs"), "utf8"));
  assert.match(wiring, /SRC\.slice\(\s*RAW_SRC\.indexOf\(/,
    "阳性对照不在剥注释后的文本里——普查结果不作数");
});
