// 撞上人机验证墙时的行为。
//
// 在此之前后端根本不认这件事：验证页和目标页面一样，都只是「一张截图 + 一段文本」。
// 于是模型把挑战页当正文解析，或者原地把同一个地址重试到工具调用次数耗光——
// 用户看到的就是「全自动流程不完美」。
//
// 这里守住两条性质：
//   1. 撞墙时事实告知模型当前在验证页上，且给出处理选项（自动点击/交给用户/换路径）；
//   2. Rust 那边发出来的字段名，和 main.js 这边读的字段名，不能各改各的。
//      第 2 条是这类改动真正会烂掉的地方：两侧都编译、都全绿，只是从此永远不触发。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
// 正向源码断言必须跑在**剥掉注释**的源码上。注释不是代码：把一条契约从代码里删掉、
// 只在注释里留一句，assert.match 照样绿——本仓库已经这样漏过一整组模型可见的工具契约。
// 所以 `SRC` 绑定的是 CODE（注释整段置空，行号与偏移和原文一字不差）；
// 真要匹配注释本身的断言显式用 RAW_SRC，并在那一行写清为什么。
import { CODE as SRC, SRC as RAW_SRC } from "./helpers/source.mjs";
const RUST = readFileSync(join(HERE, "..", "src-tauri", "src", "browser.rs"), "utf8");

// 注释里出现的词不算数——不然「不要改 navigator.webdriver」这句解释本身就能让断言变绿，
// 而真正的代码删掉了也照样通过。（这个坑踩过，所以先剥注释再断言。）
//
// 按**行**剥，不用 /\/\*[\s\S]*?\*\//：那个写法在 main.js 上会一口吃掉 40 万字符的真代码，
// 因为 `/*` 也出现在字符串和正则字面量里，非贪婪匹配会从那里一路跨到很远处的 `*/`。
// 整行注释才是本文件要排除的东西，按行处理就不会误伤行内的字面量。
function stripComments(src) {
  const out = [];
  let inBlock = false;
  for (const line of src.split("\n")) {
    const t = line.trim();
    if (inBlock) {
      if (t.includes("*/")) inBlock = false;
      continue;
    }
    if (t.startsWith("//")) continue;
    if (t.startsWith("/*")) {
      if (!t.includes("*/")) inBlock = true;
      continue;
    }
    out.push(line);
  }
  return out.join("\n");
}
const CODE = stripComments(SRC);

test("撞上人机验证时，告知模型事实并给出处理选项", () => {
  const i = CODE.indexOf("state.blocked");
  assert.ok(i >= 0, "main.js 没有消费后端的撞墙信号——那这套检测等于没接上");
  const branch = CODE.slice(i, i + 2200);
  // 事实告知：让模型知道当前在验证页上
  assert.match(branch, /人机验证/, "撞墙的结果必须一眼看出当前是验证页");
  // 给出可选路径：自动点击、交给用户、换路径
  assert.match(branch, /自动化点击/, "必须提供自动点击验证的选项");
  assert.match(branch, /告诉用户/, "必须提供交给用户的选项");
  // 挑战页的截图和文本不是目标内容，不能被当成答案引用。
  assert.match(branch, /验证页面本身.*不是目标页面/,
    "必须说明挑战页的内容不作数，否则模型会照着验证页胡编");
});

test("字段契约：state.blocked 被消费后告知模型验证状态", () => {
  const i = CODE.indexOf("state.blocked");
  assert.ok(i >= 0, "main.js 必须消费 state.blocked");
  const branch = CODE.slice(i, i + 800);
  assert.match(branch, /验证/, "blocked 分支必须提到验证");
});

test("Rust 发的字段名和前端读的字段名必须一致", () => {
  // 两侧各改各的时候，两边都编译、两边测试都绿，只是这个功能从此永不触发。
  // 所以直接拿另一侧的源码当断言依据。
  for (const field of ["blocked", "session_note"]) {
    assert.match(RUST, new RegExp(`\\n\\s+${field}: Option<String>,`),
      `BrowserState 上没有 ${field} 字段了，前端读的是个 undefined`);
    assert.ok(CODE.includes(`state.${field}`),
      `main.js 不再读 state.${field}——后端算了也没人用`);
  }
});

test("会话说明只在浏览器刚起来时发一次", () => {
  // 用 take() 而不是 clone()：每个动作都带一遍「为什么多了个 Chrome 图标」的话，
  // 会把模型上下文填满同一段文字。
  assert.match(RUST, /SESSION_NOTE\.lock\(\)\.ok\(\)\.and_then\(\|mut slot\| slot\.take\(\)\)/,
    "会话说明必须是取走一次，不能每次快照都重发");
});
