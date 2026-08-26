import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

/**
 * main.js 的尺寸闸。
 *
 * # 为什么要有它
 *
 * 实测增长：30 天前 52,537 行 → 7 天前 72,066 行 → 今天 83,384 行。**一个月 +59%**。
 * 这个文件里已经装着智能体主循环、141 个工具的 schema、全部工具执行分支、UI 渲染、
 * 会话管理、计费、终端、编辑器接线……任何一次排查都要先在五兆文本里定位。
 *
 * 这个仓库已经反复证明「往提示词里写劝诫」不解决结构问题——注意力预算那几条线用的是
 * **同一种机制**：钉一条线，撞线时必须给出理由才能抬，而抬线的注释本身成为账本。
 * 这条闸照抄那套：**要加东西可以，但得先腾出地方**。
 *
 * # 怎么用
 *
 * 撞线时不要直接抬数字。先问：这次新增的东西**能不能放进 src/agent/ 的一个模块**？
 * 判据是「边界干不干净」——只依赖参数、没有 DOM、没有模块级可变状态的，一律该搬。
 * 已经搬出去的：tool-policy / capabilities / shared-store / collaboration-engine /
 * job-queue / ansi / diff-view / escape / language / mainlink。
 *
 * 搬不动、又确实必须加在 main.js 里的（比如某个执行分支必须紧挨着主循环），
 * 才抬线，并在下面按格式补一行：日期、新值、实测值、**这一笔买到了什么**。
 * 抬线记录本身就是这个文件在长胖的证据链。
 *
 * # 抬线记录
 *
 * · 83_600（2026-08-25 首次设闸）：实测 83,384 行。同日刚把主↔子实时通道
 *   （_smRunToken / _drainSubAgentCollaborationInbox / _broadcastMainAgentFinding，
 *   101 行）搬进 src/agent/mainlink.js，作为「边界干净就该搬」的样板：那三个函数
 *   只依赖注入的 store 和一个 run 对象，搬完之后 agent-mainlink 那组测试从
 *   「用 acorn 抠源码文本再 new Function」改成**直接 import 产品代码**——
 *   前者验得到行为，验不到「这个函数还在不在真实调用链上」，而本仓库真出过
 *   「实现写好了、零调用点」。留 216 行余量给正在进行的修复，不是给新功能。
 * · 83_500（2026-08-25 第二次调整，**这次是往下调**）：实测 83,435 行。抽出第二块
 *   `src/agent/subagent-roles.js`（角色的工具矩阵 + 轮数预算，106 行）。判据和第一块
 *   一样：纯数据 + 纯函数、没有 DOM、没有模块级可变状态；唯一的外部依赖（用户自声明的
 *   角色表，它要读工具注册表）改成**从参数传**，和 mainlink 把 store 当参数是同一个规矩。
 *
 *   **抽完就把线收回来，这是这条闸的用法。** 抽出去腾的地方如果留着不收，下一次新增就
 *   直接填进去，等于白抽。所以规矩是：抬线要写清买到了什么，收线不用——收线永远是对的。
 *
 * 搬迁的附带收益（两次都一样，值得记）：原本靠「从 main.js 抠函数文本再 new Function」
 * 跑的测试可以改成直接 import 产品代码。抠源码验得到行为，验不到「这个函数在真实调用链上
 * 还在不在」——而本仓库真出过「实现写好了、零调用点」。这次改了三组、五处。
 * · 83_400（2026-08-25 第三次，仍在往下）：实测 83,3xx。抽出 `src/agent/paths.js`
 *   （路径规范化与比较，六个函数被引用近 250 次）。这一块比前两块难，两个教训值得记：
 *
 *   **判据要真的过一遍，不能看着像纯的就搬。** `pathIdentity` 读 `_remote` 全局、
 *   `coherentFilePath` 读编辑器打开的文件表——第一次搬进去两个都带着自由变量。
 *   前者改成从参数传（main.js 侧留薄壳），后者**退回 main.js**：模块里只放
 *  「给它字符串就能算出答案」的东西。
 *
 *   **而那条"没有未声明标识符"的守卫当时没抓到**，因为它的文件名单是手抄的、
 *   不含 src/agent/——每抽出去一个模块就逃出守卫一次。已改成自动发现该目录，
 *   加一行名单救不了下一个模块。
 *
 */
const MAIN_JS_MAX_LINES = 81_450;

test("main.js 不许再长胖——要加东西先腾地方", () => {
  const src = readFileSync(join(ROOT, "src/main.js"), "utf8");
  const lines = src.split("\n").length;
  assert.ok(
    lines <= MAIN_JS_MAX_LINES,
    `main.js 现在 ${lines} 行，超过上限 ${MAIN_JS_MAX_LINES}（超出 ${lines - MAIN_JS_MAX_LINES} 行）。\n`
      + "先看这次新增的东西能不能搬进 src/agent/ 的模块——判据是「只依赖参数、没有 DOM、\n"
      + "没有模块级可变状态」。确实搬不动才抬这条线，并在测试文件顶部按格式补一条抬线记录\n"
      + "（日期 / 新值 / 实测 / 这一笔买到了什么）。直接改数字不写理由的，下一个人无从判断。",
  );
});

/**
 * 闸不能只挡 main.js，否则会被"搬进另一个大文件"绕过去。
 *
 * 抽出去的模块要真的是**模块**：一个文件一件事。所以给 src/agent/ 下每个文件也设一条
 * 松得多的线——它挡的不是增长，是「把 main.js 的问题原样搬到隔壁」。
 */
const MODULE_MAX_LINES = 1_200;

test("抽出去的模块本身也不许长成第二个 main.js", () => {
  const dir = join(ROOT, "src/agent");
  const oversized = [];
  for (const name of readdirSync(dir)) {
    if (!name.endsWith(".js")) continue;
    const full = join(dir, name);
    if (!statSync(full).isFile()) continue;
    const n = readFileSync(full, "utf8").split("\n").length;
    if (n > MODULE_MAX_LINES) oversized.push(`${name}: ${n} 行`);
  }
  assert.deepEqual(
    oversized,
    [],
    `src/agent/ 下这些文件超过 ${MODULE_MAX_LINES} 行：\n  ${oversized.join("\n  ")}\n`
      + "拆成更小的模块，别把 main.js 的问题原样搬到隔壁。",
  );
});

/**
 * 这条闸本身要有效——数字得贴着现实，不能松到永远撞不上。
 *
 * 一条设在两倍现值的上限等于没有：它永远绿，而文件照样翻倍。所以反过来钉一条：
 * 上限不许比实际大太多。这条红了说明有人抬线抬过头了，或者刚做完一次大清理
 * 忘了把线收回来（那种情况把线收到新的实测值附近即可）。
 */
test("尺寸闸贴着现实，不是一条永远撞不上的线", () => {
  const lines = readFileSync(join(ROOT, "src/main.js"), "utf8").split("\n").length;
  const slack = MAIN_JS_MAX_LINES - lines;
  assert.ok(
    slack <= 3_000,
    `上限比实际大 ${slack} 行，这条闸基本不起作用了。`
      + `把 MAIN_JS_MAX_LINES 收到 ${lines + 500} 附近——闸的价值在于"下一次新增就会撞上"。`,
  );
});
