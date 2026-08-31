// 编辑器里选中的一段代码拖进输入框。
//
// 用户原话：「我鼠标选中的内容 要能够直接拖拽到对话框那里 使用」。
// 纯的那一半（标签怎么写、正文怎么围栏、超长怎么截断）在这里真跑；鼠标那一半留在 main.js，
// 下面用锚点钉住它确实接上了。
import { test } from "node:test";
import assert from "node:assert/strict";
import { selectionLabel, selectionText } from "../src/agent/selection-drag.js";
import { SRC, blockFrom, fnSource } from "./helpers/source.mjs";

test("片上的标签：只留文件名 + 行号范围，单行不写范围", () => {
  // 和输入框里其它每一种片同一条规则：只显示最后一段名字（完整路径进 tooltip）。
  assert.equal(selectionLabel("cursor_proxy/extract_fields.py", 8, 17), "extract_fields.py:8-17");
  assert.equal(selectionLabel("a.py", 8, 8), "a.py:8", "单行还写成 8-8 只是噪音");
  assert.equal(selectionLabel("a.py", 8), "a.py:8");
  // 坏输入不许炸：它跑在拖放路径上。
  assert.equal(selectionLabel("", 0, 0), ":1");
  assert.equal(typeof selectionLabel(null), "string");
});

test("展开的正文带出处——模型要知道是哪个文件的哪几行才能改它", () => {
  const t = selectionText({ rel: "cursor_proxy/x.py", lang: "python", startLine: 8, endLine: 9, code: "a=1\nb=2" });
  assert.match(t, /引用 cursor_proxy\/x\.py 第 8-9 行/, "没带出处，模型只能猜这段在哪");
  assert.match(t, /```python\na=1\nb=2\n```/, "代码没有进围栏，会和用户自己的话糊在一起");
  assert.match(selectionText({ rel: "x.py", startLine: 3, endLine: 3, code: "a" }), /第 3 行/, "单行也要写清楚");
});

test("围栏按内容算：选区里自带 ``` 也不能把块提前关掉", () => {
  // markdown、文档字符串里嵌代码块都很常见。固定三个反引号的话，后半段代码会变成正文。
  const code = "前言\n```js\nconsole.log(1)\n```\n收尾";
  const t = selectionText({ rel: "a.md", lang: "markdown", code, startLine: 1, endLine: 5 });
  const fence = /\n(`{4,})markdown\n/.exec(t);
  assert.ok(fence, "围栏没有比内容里最长的那串反引号更长——代码块会提前关掉");
  assert.ok(t.endsWith(`${fence[1]}\n`), "结尾围栏和开头不一致");
  assert.ok(t.includes(code), "代码被改过——它必须原样进块里");
  // 再长一点的也要跟着长。
  const t2 = selectionText({ rel: "a.md", code: "````\nx\n````", startLine: 1, endLine: 3 });
  assert.match(t2, /\n`{5,}\n/, "内容里有四个反引号时围栏没跟着加长");
});

test("超长要明说截断了，不能默默少给几行", () => {
  // 默默少给会让模型以为自己看到了全部，然后基于半段代码下结论。
  const code = Array.from({ length: 500 }, (_, i) => `line ${i}`).join("\n");
  const t = selectionText({ rel: "a.py", code, startLine: 1, endLine: 500, maxLines: 10 });
  assert.match(t, /选区共 500 行，这里只带了前 10 行/, "截断了却没说");
  assert.ok(!t.includes("line 20"), "maxLines 没生效");
  // 字符上限同样要说。
  const wide = Array.from({ length: 5 }, () => "x".repeat(400)).join("\n");
  const t2 = selectionText({ rel: "a.py", code: wide, startLine: 1, endLine: 5, maxChars: 500 });
  assert.match(t2, /只带了前 \d+ 行/, "按字符截断时没说");
  // 没超就不许无中生有地说截断。
  assert.doesNotMatch(selectionText({ rel: "a.py", code: "a\nb", startLine: 1, endLine: 2 }), /只带了前/,
    "没截断却说截断了");
});

test("坏输入一律不抛——它跑在拖放路径上", () => {
  for (const bad of [undefined, null, {}, { code: null }, { code: 123 }]) {
    assert.doesNotThrow(() => selectionText(bad));
  }
});

test("main.js 真的接上了：按在选区里才算候选，落到输入框才插片", () => {
  const h = blockFrom("(function _wireSelectionDragToComposer() {");
  // 判据是「按下的位置在不在选区里」。少了这一条，编辑器里任何一次按下拖动都会变成拖代码，
  // 正常的框选就没法做了。
  assert.match(h, /sel\.containsPosition\(t\.position\)/,
    "没有判「按在选区里」——普通框选会被当成拖拽");
  assert.match(h, /monacoEditor\.getTargetAtClientPoint\(e\.clientX, e\.clientY\)/,
    "没有把鼠标位置换算成编辑器里的位置");
  // 阈值：不走够距离不算拖，否则点一下就插一枚片。
  assert.match(h, /< 6\) return;/, "没有拖拽阈值——点一下就会插片");
  // 只在落到输入框上时才插，且插的是 code 片 + 展开的正文。
  assert.match(h, /_insertRefAtCursor\(c\.rel, "code", _selectionLabel\([\s\S]{0,80}_selectionText\(/,
    "落点没有接到 code 片上");
  // 片自己带正文；_chipText 发送时读它，而不是把它当成 @ 引用去读整个文件。
  assert.match(fnSource("_chipText"), /if \(kind === "code"\) return chip\?\.dataset\?\.text \|\| "";/,
    "code 片没有直接展开成正文——会被当成 @ 引用，模型收到的是整个文件而不是选中那段");
  assert.match(fnSource("_insertRefAtCursor"), /if \(dataText\) chip\.dataset\.text = dataText;/,
    "插入时没有把正文挂到片上，发送出去会是空的");
  // 图标用这个文件真正的图标，且必须从 rel 算 —— code 片的 name 是「quota.py:275-284」，
  // 带着行号去查扩展名会落到兜底图标上（用户：「前面图标要用真实的文件图标」）。
  assert.match(SRC, /kind === "code"\s*\n?\s*\? iconImg\(fileIconUrl\(rel\.split\("\/"\)/,
    "code 片没有用真实的文件图标，或者图标是从带行号的 name 算的");
});
