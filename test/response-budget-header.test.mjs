// 「客户端还能等多久」这件事，两个头必须成对存在。
//
// 起因是一条静默的、按机器发作的故障：`x-ide-response-deadline-ms` 是客户端 `Date.now()`
// 的绝对时间戳，网关拿**自己**的时钟去减它才能算出剩余量。机器时钟慢上一两分钟（NTP
// 被挡、虚拟机休眠唤醒、装机没对时），这个差就永远是负的，网关算出的预算恒为零——那台
// 机器上每一次请求都在发往上游之前就被判死，而且永远如此。两边日志里都只看得到"这个人
// 什么都发不出去"，看不出为什么。
//
// 解法是再发一个**相对**预算头，它就是本地定时器用的那个数，不牵涉任何时钟比对。网关
// 优先采信它，绝对时间戳只在两个时钟对得上时用来收紧（它的价值是把上传耗时也算了进去）。
//
// 这个文件守的是这套机制的三个断点，任何一个断了，时钟不准的机器就又没有出路了：
//   1. 网页/IDE 前端两个头都发；
//   2. 桌面端两个头都发；
//   3. 网关两个头都读。
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));

// 注释里出现的字符串不算数：本文件解释这条改动的注释就逐字写着两个头的名字，不剥的话
// 把三处实现全删掉这些断言依然全绿。按**行**剥，不用 /\/\*[\s\S]*?\*\//——后者在
// main.js 上会一口吃掉几十万字符的真代码，因为 `/*` 也出现在字符串和正则字面量里。
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

const WEB = stripComments(readFileSync(join(HERE, "../src/main.js"), "utf8"));
const DESKTOP = stripComments(readFileSync(join(HERE, "../src-tauri/src/ai.rs"), "utf8"));
const GATEWAY = stripComments(
  readFileSync(join(HERE, "../../server/src/models.rs"), "utf8"),
);

const DEADLINE = "x-ide-response-deadline-ms";
const BUDGET = "x-ide-response-budget-ms";

test("网页端两个头一起发，且相对预算取自本地常量而不是时钟", () => {
  assert.ok(WEB.includes(DEADLINE), "绝对时间戳仍然要发——网关靠它把上传耗时算进去");
  assert.ok(
    WEB.includes(BUDGET),
    "相对预算头没了：客户端时钟不准的机器会退回「预算恒为零」，一次请求都发不出去",
  );

  // 关键点：相对预算的值必须是那个本地常量，不能是 Date.now() 推出来的任何东西，
  // 否则它就又变成一个和时钟有关的量，等于什么也没修。
  const line = WEB.split("\n").find((l) => l.includes(BUDGET));
  assert.ok(line, "找不到设置相对预算头的那一行");
  assert.match(
    line,
    /_AI_RESPONSE_HEADERS_DEADLINE_MS/,
    `相对预算必须直接用本地定时器的常量，实际是：${line.trim()}`,
  );
  assert.doesNotMatch(line, /Date\.now\(\)/, "相对预算一旦掺进墙上时钟就失去了全部意义");
});

test("桌面端两个头一起发，且相对预算取自 deadline 自己的 budget", () => {
  assert.ok(DESKTOP.includes(DEADLINE));
  assert.ok(
    DESKTOP.includes(BUDGET),
    "桌面端是主要发行形态，它漏发的话这条修复对绝大多数用户等于不存在",
  );

  assert.match(
    DESKTOP,
    new RegExp(`const RESPONSE_BUDGET_HEADER: &str = "${BUDGET}";`),
    "常量名和实际发出的头必须对得上",
  );

  // `ResponseHeadersDeadline` 里 `unix_ms` 是墙上时钟、`budget` 是纯时长。
  // 相对预算头必须来自后者，而且要挂在统一的头装配处，否则某条调用路径会绕过它。
  const setter = DESKTOP.slice(
    DESKTOP.indexOf("fn with_response_deadline_header"),
    DESKTOP.indexOf("fn request_was_cancelled"),
  );
  assert.ok(setter.includes("RESPONSE_BUDGET_HEADER"), "相对预算头不在统一的头装配处");
  assert.match(
    setter,
    /budget[\s\S]*?as_millis\(\)/,
    "相对预算必须来自 deadline.budget（纯时长），不能来自 unix_ms（墙上时钟）",
  );
});

test("网关两个头都读，并且不再只靠绝对时间戳做判断", () => {
  assert.ok(GATEWAY.includes(DEADLINE));
  assert.ok(GATEWAY.includes(BUDGET), "网关不读相对预算的话，客户端发了也没用");

  // 时钟偏差必须留痕：否则「就他一台机器用不了」在服务端依然是查不到的。
  assert.match(
    GATEWAY,
    /ClockSkewed/,
    "时钟对不上要能在网关日志里认出来，而不是表现为某个用户莫名其妙发不出请求",
  );
  assert.match(
    GATEWAY,
    /AbsoluteUntrusted/,
    "尚未升级的客户端只有绝对时间戳，必须有合理性检查兜底",
  );
});
