// 撞上人机验证墙时的行为。
//
// 在此之前后端根本不认这件事：验证页和目标页面一样，都只是「一张截图 + 一段文本」。
// 于是模型把挑战页当正文解析，或者原地把同一个地址重试到工具调用次数耗光——
// 用户看到的就是「全自动流程不完美」。
//
// 这里守住两条性质：
//   1. 撞墙时给模型的指令是**停下来交给人**，且明确禁止自行绕过；
//   2. Rust 那边发出来的字段名，和 main.js 这边读的字段名，不能各改各的。
//      第 2 条是这类改动真正会烂掉的地方：两侧都编译、都全绿，只是从此永远不触发。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
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

test("撞上人机验证时，给模型的指令是停下交给人，不是想办法过去", () => {
  const i = CODE.indexOf("state.blocked");
  assert.ok(i >= 0, "main.js 没有消费后端的撞墙信号——那这套检测等于没接上");
  const branch = CODE.slice(i, i + 2200);
  // 交接：窗口是可见的，人点一下就过了，模型要停下来等。
  assert.match(branch, /需要你本人操作/, "撞墙的结果必须一眼看出要人介入");
  assert.match(branch, /不要尝试绕过/, "必须明确禁止模型自己想办法绕过去");
  // 逐项禁掉那几种「聪明办法」——这几条正是绕过机器人检测，这个产品不做。
  for (const banned of ["User-Agent", "navigator.webdriver", "镜像"]) {
    assert.ok(branch.includes(banned), `没有明确禁止「${banned}」这条绕行路子`);
  }
  // 反复重试同一个地址是现在的实际表现，必须点名禁掉。
  assert.match(branch, /不要反复重试/, "必须禁止原地重试——那是现在真实发生的事");
  // 挑战页的截图和文本不是目标内容，不能被当成答案引用。
  assert.match(branch, /不要把下面这张截图和文本当成目标页面的内容/,
    "必须说明挑战页的内容不作数，否则模型会照着验证页胡编");
});

test("产品里没有任何绕过或反检测的实现", () => {
  // 这条是有意写死的边界：只做「发现并交接」，不做「解题或藏起来」。
  // 以后谁想顺手加个打码服务或者去掉自动化标志，这里会红。
  const rustCode = stripComments(RUST);
  // 注意区分「提到」和「真去做」：上一条测试要求禁令文本里**必须**写着
  // navigator.webdriver（那是发给模型的话），所以这里只能钉真正的实现手法。
  const implementations = [
    /--disable-blink-features=AutomationControlled/,
    /Object\.defineProperty\(\s*navigator\s*,\s*["']webdriver["']/,
    /delete\s+navigator\.webdriver/,
    /navigator\.webdriver\s*=/,
    /2captcha|anti-captcha|capsolver|deathbycaptcha/i,
  ];
  for (const re of implementations) {
    assert.doesNotMatch(rustCode, re, `browser.rs 里出现了绕过检测的实现：${re}`);
    assert.doesNotMatch(CODE, re, `main.js 里出现了绕过检测的实现：${re}`);
  }
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
