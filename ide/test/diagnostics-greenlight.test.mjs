import test from "node:test";
import { SRC as SHARED_SRC } from "./helpers/source.mjs";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

/*
 * get_diagnostics 对 Python / Rust / Go **永远给不出一次绿灯**。
 *
 * 有两条判据前后脚写在一起。前面那条粗的：「非内置语言 + 没有诊断 = 一定没人在检查」。
 * 这个推理是错的——pyright 真的跑着、真的把文件看干净了，也是这个形状。于是模型改完
 * Python 调 get_diagnostics，拿到的永远是「一条都没检查」，只能每次都跑一遍完整构建。
 *
 * 而后面那条真判据（diagnosticsProviderReady：问语言服务器当前到底起没起来）被它挡成了
 * 死代码——只在 probs.length > 0 且服务没起来时才够得到，而有诊断就说明有人在出诊断，
 * 那是个自相矛盾的条件。
 *
 * 这条测试**真的跑那段判断**，不是断言源码里有某个词：删一个显示分支很容易顺手把模型的
 * 入口一起删掉，「调用点存在」不等于「路径会被走到」。
 */
// 源码文本用共享的那一份（helpers/source.mjs 的 SRC = main.js + src/agent/* 拼接）。
// 自己 readFileSync("src/main.js") 的话，每从 main.js 搬出一个模块就假红一次；
// 反方向更糟：「main.js 里不许出现 X」这类断言会在 X 搬进模块后恒绿，禁令悄悄失效。
const SRC = SHARED_SRC;

const START = "      const _diagLang = (() => {";
const END = `无错误或警告\${note}`;

function decide({ lang, ready, stopReason = "", probs = [], path = "/x.py" }) {
  const i = SRC.indexOf(START);
  assert.ok(i > 0, "诊断判断那段不见了");
  const j = SRC.indexOf(END, i);
  assert.ok(j > i, "结尾那句绿灯不见了");
  const body = SRC.slice(i, SRC.indexOf("};", j) + 2);

  const model = { getLanguageId: () => lang };
  const monaco = { editor: { getModels: () => [model] } };
  const lspManager = {
    diagnosticsProviderReady: (l) => l === lang && ready,
    lastStopReason: () => stopReason,
  };
  return new Function(
    "monaco", "lspManager", "_tempModel", "_modelForExactPath",
    "diagnosticPath", "probs", "note", "call",
    body + "\nreturn undefined;",
  )(monaco, lspManager, model, () => model, path, probs, "", { path });
}

test("语言服务真的跑着且文件干净 → 必须给出绿灯，而不是「一条都没检查」", () => {
  const r = decide({ lang: "python", ready: true });
  assert.ok(r, "什么都没返回");
  assert.match(r.content, /无错误或警告/,
    "pyright 跑着、文件干净，仍然被告知没有语言服务 —— Python/Rust/Go 永远拿不到一次绿灯，"
    + "模型只能每次都跑完整构建");
  assert.doesNotMatch(r.content, /一条都没检查/, "把有效的绿灯说成了没检查");
});

test("语言服务没起来 → 必须明说「一条都没检查」", () => {
  const r = decide({ lang: "python", ready: false });
  assert.match(r.content, /\*\*这次一条都没检查。\*\*/,
    "没装 pyright 时给了绿灯 —— 模型据此向用户报「已修复并验证通过」，而一行都没被检查过");
  assert.match(r.content, /pyright-langserver/, "没告诉模型该装什么");
  assert.match(r.content, /run_cmd/, "没给出路");
});

test("语言服务是崩掉的 → 说崩因，不要说「多半是没装」", () => {
  const r = decide({ lang: "python", ready: false, stopReason: "ModuleNotFoundError: no module named 'x'" });
  assert.match(r.content, /它启动过但崩了：ModuleNotFoundError/,
    "装了但崩了，还在让模型去装一遍 —— 装完还是不行，来回几轮");
  assert.doesNotMatch(r.content, /多半是没装/, "两种情况的出路完全不同，不能说同一句话");
});

test("内置语言（TS/JS/JSON/CSS/HTML）不受这道判据影响", () => {
  // 它们由 Monaco 自带 worker 出诊断，lspManager 里根本没有对应的 client。
  const r = decide({ lang: "typescript", ready: false, path: "/x.ts" });
  assert.match(r.content, /无错误或警告/, "内置语言被误判成「没有语言服务」");
});

test("那条粗判据不许再回来", () => {
  assert.doesNotMatch(SRC, /当前 IDE \*\*没有语言服务在给它出诊断\*\*/,
    "粗判据又回来了 —— 它会把真判据重新挡成死代码");
  // 顺序也要守：真判据必须在最后那句绿灯之前。
  const iReady = SRC.indexOf("const _diagReady");
  const iGreen = SRC.indexOf("无错误或警告${note}");
  assert.ok(iReady > 0 && iGreen > iReady, "真判据跑到绿灯后面去了，等于没有");
});
