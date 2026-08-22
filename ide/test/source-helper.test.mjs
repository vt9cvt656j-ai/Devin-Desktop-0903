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
import { SRC, CODE, OVERRIDES, fnSource, load, loadConst } from "./helpers/source.mjs";

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
