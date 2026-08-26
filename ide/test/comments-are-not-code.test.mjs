import test from "node:test";
// 这一对 2026-08-25 搬进了 src/agent/code-text.js —— 直接 import 真模块，
// 不再抠源码：抠源码验得到行为，验不到它在真实调用链上还在不在。
import { splitCodeAndComments as _splitCC, symbolPatternsFor as _symPat } from "../src/agent/code-text.js";
import assert from "node:assert/strict";
import { load, SRC } from "./helpers/source.mjs";

/*
 * 用户的原话：「不能光看注释，要代码一起看 —— 不然有时候注释会欺骗 IDE，
 * 有的还会用旧注释让 IDE 发现不了问题。」
 *
 * 两个方向都要防：
 *   ① 注释里的东西不许被当成代码。实测（改之前真会）：模型写
 *        // 老写法，已经废弃：
 *        // db.query("SELECT * FROM users WHERE id = " + id)
 *      然后被告知它写了 SQL 注入。既是噪音，也让整套机制显得不可信。
 *   ② 代码里的东西不许被注释盖住。写一句「这里已经参数化了」，拼接还是拼接。
 *
 * 判据只看代码部分，所以 ② 天然成立；这个文件把两个方向都钉住。
 */
const split = _splitCC;
const sink = load("_sinkRisksInWrite", { _splitCodeAndComments: split });
const amb = load("_ambiguousFailureInWrite", { _splitCodeAndComments: split });
const stub = load("_stubDeliveryFindings", {
  _CODE_FILE_RE: /\.(?:tsx?|jsx?|py|rs|go|java|rb|php|cs|swift|kt)$/i,
  _splitCodeAndComments: split,
});
const scanFile = (path, current, content = "") =>
  stub({ checkpoint: new Map([[path, { content, current }]]) }, 8);

// ── 一、拆分本身 ──────────────────────────────────────────────────────
test("串里的注释符号不是注释", () => {
  const { code, comments } = split(`const u = "https://x.com/a"; // 真注释`, "a.js");
  assert.match(code[0], /https:\/\/x\.com\/a/, "把字符串里的 // 当成注释起点了 —— URL 会被整段抹掉");
  assert.equal(comments[0].trim(), "真注释");
});

test("模板串、私有字段、Python 串里的 # 都不许被误伤", () => {
  assert.match(split("const s = `a // 不是注释 ${x}`;", "a.js").code[0], /不是注释/);
  assert.match(split("this.#priv = 1; // x", "a.js").code[0], /this\.#priv = 1;/,
    "JS 的私有字段被 # 抹掉了 —— 默认不认 # 就是为了这个");
  assert.match(split(`url = "http://a#b"  # 真注释`, "a.py").code[0], /http:\/\/a#b/);
  assert.equal(split(`url = "http://a#b"  # 真注释`, "a.py").comments[0].trim(), "真注释");
});

test("行数和每行长度都不变——行号列号还要对得上", () => {
  const src = "a();\n// 注释\n/* 块\n   注释 */ b();\nc();";
  const { code, comments } = split(src, "a.js");
  const raw = src.split("\n");
  assert.equal(code.length, raw.length, "行数变了，行号就全错位了");
  for (let i = 0; i < raw.length; i++) {
    assert.equal(code[i].length, raw[i].length, `第 ${i + 1} 行长度变了，列号对不上`);
  }
  assert.match(code[3], /b\(\);/, "块注释结束后的代码被一起吃掉了");
  assert.match(comments.join("\n"), /块/, "块注释没被收进来");
});

test("按扩展名分注释语法", () => {
  assert.equal(split("x = 1 -- 注释", "a.sql").comments[0].trim(), "注释");
  assert.equal(split("x = 1 -- 不是注释", "a.js").comments[0], "", "JS 里 -- 不是注释");
  assert.equal(split("x = 1 # 注释", "a.rb").comments[0].trim(), "注释");
});

// ── 二、方向①：注释里的东西不许被当成代码 ──────────────────────────────
const COMMENTED_OUT = `function f(id) {
  // 老写法，已经废弃：
  // db.query("SELECT * FROM users WHERE id = " + id)
  return db.query("SELECT * FROM users WHERE id = ?", [id]);
}`;

test("注释掉的危险写法不报——那是模型在解释历史，不是它写的代码", () => {
  assert.deepEqual(sink(COMMENTED_OUT, null, "a.js"), [],
    "被注释掉的 SQL 拼接被报成了这次写入的漏洞 —— 既是噪音，也让这套机制显得不可信");
});

test("注释里的 return null 不算一条返回路径", () => {
  const code = `async function g(p) {
  // 两条路都 return null 的话就分不出来了
  if (!p) return null;
  try { return await read(p); } catch (e) { throw e; }
}`;
  assert.deepEqual(amb(code, 3, "a.js"), [],
    "把注释里那句 return null 当成了第二条返回路径");
});

test("注释里的 example.com / 空函数体不算这次交付的占位", () => {
  const hits = scanFile("/p/a.ts", `export const a = 1;
// 反面例子，别这么写：
// fetch("https://api.example.com/pay")
// function verifyToken(t) { return true; }
export function verifyToken(t) { return checkSignature(t); }`);
  const kinds = hits.map((h) => h.kind);
  assert.ok(!kinds.includes("编造的地址"), `注释里的示例地址被算成了占位：${JSON.stringify(hits)}`);
  assert.ok(!kinds.includes("鉴权恒真（空壳且是漏洞）"), `注释里的反面例子被算成了空壳：${JSON.stringify(hits)}`);
});

test("但自我招供的那几条照旧看注释——TODO 本来就写在注释里", () => {
  const hits = scanFile("/p/a.ts", `export const a = 1;\n// TODO: 接真实支付网关`);
  assert.equal(hits.length, 1, "TODO 不看注释就等于这条判据废了");
  assert.equal(hits[0].kind, "TODO 占位");
  assert.equal(hits[0].line, 2);
});

// ── 三、方向②：注释里的辩解拦不住判据 ────────────────────────────────
test("注释说「这里已经安全了」，拼接还是照报", () => {
  const code = `function f(id) {
  // 安全说明：这里已经做过参数化和转义了，不用担心注入
  return db.query("SELECT * FROM users WHERE id = " + id);
}`;
  const kinds = sink(code, null, "a.js").map((r) => r.kind);
  assert.deepEqual(kinds, ["SQL 拼接"],
    "注释里的一句辩解就让判据闭嘴了 —— 那正是「旧注释让 IDE 发现不了问题」");
});

test("回显给模型的是原始行（带注释），判定用的才是代码行", () => {
  const code = `function f(id) {\n  return db.query("SELECT * FROM t WHERE id = " + id); // 待重构\n}`;
  const [hit] = sink(code, null, "a.js");
  assert.ok(hit, "没报");
  assert.match(hit.text, /待重构/, "回显把注释砍掉了 —— 模型认不出自己写的是哪一句");
  assert.equal(hit.line, 2);
});

// ── 四、接线 ─────────────────────────────────────────────────────────
test("三个检测器都接了底座，且把路径传下去", () => {
  for (const [fn, tail] of [["_sinkRisksInWrite", "codeLines"], ["_ambiguousFailureInWrite", "_splitCodeAndComments"], ["_stubDeliveryFindings", "codeLines"]]) {
    const i = SRC.indexOf(`function ${fn}(`);
    assert.ok(i > 0, `${fn} 不见了`);
    assert.ok(SRC.slice(i, i + 4000).includes(tail), `${fn} 没用上代码行`);
  }
  // 路径决定按哪种注释语法拆，不传就按 JS 猜——Python 的 # 会被漏掉。
  assert.match(SRC, /_sinkRisksInWrite\(body, null, call\?\.path \|\| ""\)/, "sink 的调用点没传路径");
  assert.match(SRC, /_ambiguousFailureInWrite\(body, 3, call\?\.path \|\| ""\)/, "ambiguous 的调用点没传路径");
  assert.match(SRC, /_splitCodeAndComments\(cur, String\(absPath\)\)/, "占位检测没按文件的真实扩展名拆");
});
