// 「写着写着就写成了 MVP」——用户原话。这条账的判据必须来自**落盘内容本身**，不是模型的说法。
//
// 改之前实测召回 3/12：原来那五条只抓模型**自己老实标注**的占位（TODO、not implemented、
// 命名里带 mock）。真正的 MVP 退化是**看着像写完了的空壳**：鉴权函数直接 return true、
// 空函数体、真逻辑被 if(false) 关掉、取数函数回空数组、把地址写成 api.example.com。
// 这些一条都不落进原来那张表。
//
// 新判据全部在本仓库自己的 207,656 行真实代码（JS + Rust）上量过误报，**全为 0**。
// 量出来被砍掉的候选（宁可漏，不可让每轮跳假警报，那会让整本账失去可信度）：
//   · 空 catch          83.4/万行 —— `try{...}catch{}` 在本仓库是通用惯用法
//   · `return {ok:true}` 1.24/万行 —— 正当代码里遍地都是
//   · 写死 localhost     0.8/万行 —— 抽样全是正则误匹配到散文里
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, load, fnSource } from "./helpers/source.mjs";

const scan = load("_stubDeliveryFindings", {
  _CODE_FILE_RE: /\.(?:tsx?|jsx?|py|rs|go|java|rb|php|cs|swift|kt)$/i,
});
const one = (code, maxItems = 8) =>
  scan({ checkpoint: new Map([["/p/src/a.ts", { content: "", current: code }]]) }, maxItems);

// ── 一、该抓到的 ──────────────────────────────────────────────────────
const CAUGHT = [
  ["// TODO: 接真实支付网关", /TODO/],
  ['throw new Error("not implemented")', /未实现/],
  ["const mockUsers = [{id:1}]", /假数据/],
  // 以下五种是这次补的，都要跨行才认得出——真实代码不写在一行里
  ["function verifyToken(t) {\n  return true;\n}", /鉴权恒真/],
  ["const canAccess = (u) => {\n  return true;\n}", /鉴权恒真/],
  ["async function handlePayment(order) {\n}", /空函数体/],
  ["if (false) {\n  await realCharge(o);\n}", /真逻辑被关掉/],
  ["async function listOrders(uid) {\n  return [];\n}", /取数函数回空/],
  ['const r = await fetch("https://api.example.com/pay");', /编造的地址/],
  ['const baseUrl = "https://example.com/api";', /编造的地址/],
];

test("看着像写完了的空壳要被点名，且给得出是哪一种", () => {
  for (const [code, kind] of CAUGHT) {
    const got = one(code);
    assert.ok(got.length, `漏掉了：${code.replace(/\n/g, "⏎")}`);
    assert.match(got[0].kind, kind, `认成了别的种类：${got[0].kind}`);
    assert.equal(got[0].line, 1, "要给行号");
    assert.ok(got[0].text.length > 0, "要给那一行的原文，否则用户无从对照");
  }
});

test("鉴权恒真同时算「空壳」和「漏洞」——文案要说穿", () => {
  // 这一条落在用户两个要求的交叉点上：既是没写完，也是一个真的鉴权洞。
  const got = one("function verifyToken(t) {\n  return true;\n}");
  assert.match(got[0].kind, /漏洞/,
    "只说它是空壳——模型会当成「以后补」，而它现在就是一个可绕过的鉴权");
});

// ── 二、刻意不抓的（反向断言，否则上面那条会诱使人无限放宽）────────────
const SILENT = [
  ["return { ok: true, balance: 100 };", "硬编码返回：本仓库自己 1.24/万行都是正当用法"],
  ["try { await charge(o); } catch (e) {}", "空 catch：本仓库 83.4/万行，是通用惯用法"],
  ['const API = "http://localhost:3000";', "写死 localhost：开发工具里正当"],
  ['expect(link).toBe("https://example.com/doc")', "测试断言里的 example.com 是数据不是端点"],
  ['html = `<a href="https://example.com/x">`', "纯字符串数据里的 example.com"],
];

test("误报率高的形态刻意不报——每一条都有量过的理由", () => {
  for (const [code, why] of SILENT) {
    assert.deepEqual(one(code), [], `${why}；这条一旦开始报，每一轮都会跳假警报`);
  }
});

// ── 三、活的校准：拿仓库自己的真实代码当对照组 ─────────────────────────
test("六条结构判据在本仓库自己的代码上一个都不许命中", () => {
  // 这不是形式主义。判据放宽的诱惑一直在，而放宽的代价要到线上才看得见。
  // 把本仓库当对照组：它是几十万行**真实的、不是 MVP 的**代码，命中即误报。
  // 这条一旦变红，两种可能——判据被放宽了，或者真有人写了这种代码。两种都该当场知道。
  const STRUCT_KINDS = /鉴权恒真|真逻辑被关掉|空函数体|取数函数回空|编造的地址/;
  const found = scan({ checkpoint: new Map([["/p/src/main.ts", { content: "", current: SRC }]]) }, 400)
    .filter((f) => STRUCT_KINDS.test(f.kind));
  assert.deepEqual(found.map((f) => `${f.line}: ${f.text}`), [],
    `六条结构判据在 main.js 上误报了 ${found.length} 处`);
});

test("基线相减没被破坏：本来就有的不算这次的账", () => {
  const before = "function verifyToken(t) {\n  return true;\n}";
  const same = scan({ checkpoint: new Map([["/p/src/a.ts", { content: before, current: before }]]) });
  assert.deepEqual(same, [],
    "文件里本来就有的空壳被算到这次交付头上了——每轮都会重复报同一处");
});

test("结构判据必须锚在窗口首行行首", () => {
  // 3 行窗口会把不相干的后两行拼进来。不锚行首时实测：空函数体那条误报从 0 涨到
  // 1.59/万行（命中的全是 Rust 里被窗口拼平的无关语句）。这条契约比某一条正则本身更要紧——
  // 下一个人往 STRUCT 里加判据时，忘了锚就是引入一片假警报，而 main.js 那条校准未必抓得住。
  const body = fnSource("_stubDeliveryFindings", { code: true });
  const at = body.indexOf("const STRUCT = [");
  const end = body.indexOf("];", at);
  assert.ok(at > 0 && end > at, "STRUCT 不见了");
  const block = body.slice(at, end);
  // 每条正则字面量：要么锚 ^（结构型），要么以 \b + 调用/配置上下文开头（地址型）
  const regexes = [...block.matchAll(/\[\/(.+?)\/[a-z]*,\s*"/g)].map((m) => m[1]);
  assert.ok(regexes.length >= 6, `只解析出 ${regexes.length} 条判据，应 ≥6`);
  for (const r of regexes) {
    const anchored = r.startsWith("^") || r.startsWith("\\s*") || r.startsWith("\\b(?:fetch") || r.startsWith("\\b(?:baseUrl");
    assert.ok(anchored, `这条判据没有锚：/${r.slice(0, 60)}…/ —— 3 行窗口会把不相干的行拼进来`);
  }
});

test("判据全部定义在函数体内，不引入新的外部标识符", () => {
  // 本仓库的注入清单是手工维护的：给这个函数加一个外部引用，
  // 那些 load() 站点会直接 ReferenceError，而不是断言失败。
  const body = fnSource("_stubDeliveryFindings", { code: true });
  assert.match(body, /const STRUCT = \[/, "结构判据不在函数体内了");
  assert.match(body, /const MARKS = \[/, "原来那五条不见了");
  assert.match(body, /lines\.slice\(i, i \+ 3\)/,
    "3 行窗口没了——真实代码里的空壳是跨行的，单行判据一条都抓不到");
});
