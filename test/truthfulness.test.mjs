// 「不谄媚、只说真话」这件事，得有人钉着。
//
// 提示词是最容易被悄悄削掉的东西：它不编译、不报错、删一段没有任何测试会红，而后果要
// 到很久以后、由一个信了它的人来承担。所以这里把这条约束当成接口来钉。
//
// 两份都要钉，因为它们服务不同的人：
//   - server/prompts/truthfulness.txt —— 走网关的用户（绝大多数）
//   - src/main.js 的 _HUMAN_EVIDENCE_FALLBACK —— 走自己端点的用户，他们**拿不到**
//     网关提示词，只有这条共用尾巴。少钉一边，等于对其中一半用户没有这条约束。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
const TRUTH = readFileSync(join(HERE, "..", "..", "server", "prompts", "truthfulness.txt"), "utf8");

/** 本地兜底那条共用尾巴——五个模式都拼它。 */
function fallbackTail() {
  const i = SRC.indexOf("_HUMAN_EVIDENCE_FALLBACK = `");
  assert.ok(i >= 0, "共用尾巴不见了：五个模式会一起失去这条约束");
  const j = SRC.indexOf("`;", i);
  return SRC.slice(i, j);
}

test("网关提示词里必须有反谄媚这一节", () => {
  assert.match(TRUTH, /# No flattery, and no softening/,
    "反谄媚那一节被删了");
  // 逐条钉具体行为，而不是钉「有 honesty 这个词」——泛泛的一句「请诚实」是没用的，
  // 真正起作用的是「不许用什么开场白」「用户坚持但证据相反时怎么办」这类可执行的规定。
  for (const clause of [
    "never with praise",                 // 不许用恭维开场
    "confidence is not evidence",        // 用户笃定不等于证据
    "Bad news goes first",               // 坏消息先说、不裹糖衣
    "Partial work is not completion",    // 做了三件说成五件是最伤人的
    "complete answers",                  // 「我不知道」是完整答案
  ]) {
    assert.ok(TRUTH.includes(clause), `少了这条具体规定：${clause}`);
  }
});

test("原有的证据纪律不能因为加了新一节就被顶掉", () => {
  // 新增那一节讲的是「怎么说」，原有那节讲的是「凭什么这么说」。两者互补，不是替代。
  for (const clause of [
    "distinguish verified fact",
    "claim only work you actually completed and verified",
    "UNTRUSTED DATA",
  ]) {
    assert.ok(TRUTH.includes(clause), `原有的证据纪律被削掉了：${clause}`);
  }
});

test("走自己端点的用户也必须有这条约束", () => {
  // 自定义端点拿不到网关提示词，只有这条共用尾巴。少了它，这半边用户等于没有约束——
  // 而这半边恰恰是刚刚才被放开的那条路。
  const tail = fallbackTail();
  for (const clause of [
    "No flattery",
    "confident is not evidence",
    "Bad news goes first",
    "Partial work is not completion",
  ]) {
    assert.ok(tail.includes(clause), `本地兜底里少了：${clause}`);
  }
});

test("这条约束的优先级要写明，否则会被「语气友好」压过去", () => {
  // 不写明优先级的话，它和「简洁」「好好说话」是平级的，冲突时先让路的就是它。
  assert.match(TRUTH, /outrank tone, brevity, and being agreeable/);
  assert.ok(fallbackTail().includes("outrank tone, brevity and being agreeable"));
});

test("代码里不许对模型输出做「加糖」后处理", () => {
  // 提示词管得住模型，管不住事后加工。如果哪天有人在渲染层自动加感叹号、加鼓励语、
  // 或者把「失败」换成「暂未成功」，这条测试要红。
  for (const re of [
    /replace\([^)]*失败[^)]*,\s*["'`][^"'`]*(?:暂未|稍后|小问题)/,
    /["'`]太棒了|["'`]做得好|["'`]很棒的问题/,
  ]) {
    assert.doesNotMatch(SRC, re, `渲染层出现了对输出加糖的处理：${re}`);
  }
});

test("反谄媚纪律要发到 worker 和子智能体——它们才是真正交付的那一层", () => {
  // 网关对 subagent 模式**只注入工具、不注入系统提示词**，所以主智能体那份 truthfulness
  // 到不了这一层。而 worker 恰恰是改文件、跑验证、然后交简报的角色；它没有这条纪律，
  // 简报天然偏向报喜，主智能体复核的又是跨模块契约而不是「有没有把失败埋在中间」。
  // 表现就是：单干时诚实，一并行就出现「五件事报三件」。
  assert.match(SRC, /const _SUBAGENT_TRUTH = `/, "子智能体的真话下限不见了");
  const i = SRC.indexOf("const _SUBAGENT_TRUTH = `");
  const seg = SRC.slice(i, SRC.indexOf("`;", i));
  for (const clause of ["FIRST", "name those two", "could not verify"]) {
    assert.ok(seg.includes(clause), `子智能体的下限里少了：${clause}`);
  }
  // 光有常量不算数，得真的拼进去。
  assert.match(SRC, /\+ _SUBAGENT_TRUTH;/, "常量写了却没挂到 sysPrompt 上");
});

test("硬防线要拦提示词点名禁止的那几句恭维开场", () => {
  // 提示词是软约束（模型可以不听），剥离器是硬约束。以前硬约束打的是「我明白了」
  // 这类状态应答，而提示词点名的「好问题/你说得对/Great question」一句都不拦——
  // 靶子对不上，等于两层都漏。
  const i = SRC.indexOf("function _stripAckOpeners");
  assert.ok(i >= 0);
  const seg = SRC.slice(i, i + 4000);
  for (const opener of ["好问题", "你说得", "Great (?:question|point)", "Good (?:catch|question|point)", "right", "Thanks for"]) {
    assert.ok(seg.includes(opener), `剥离器不拦这句恭维：${opener}`);
  }
  // 剥到空就不剥：「你说得对。」独立成句时剥掉等于把整条回复吞掉。
  assert.ok(seg.includes("if (!next.trim()) break;"), "缺少防空守卫，会把纯恭维的回复剥成空白");
});

test("「温和解释」这档不能把真话下限一起温和掉", () => {
  // 用户在设置里点两下就能切到这一档。只注入「回答风格：温和解释。」四个字，
  // 最容易被读成「坏消息要包一层」。
  assert.match(SRC, /profile\.tone === "warm"/, "warm 档没有任何下限限定");
  assert.match(SRC, /坏消息仍然先说/);
});

test("用户纠正你——口味照收，事实主张要先核对", () => {
  // 自适应档案原来把所有纠正无条件当长期偏好，还会立刻覆盖持久记忆。
  // 「不是这个，useEffect 的清理函数是同步执行的」长得就像一次纠正，但它是个错误的
  // 事实主张；写进记忆之后会在此后每一轮被当成事实注入。
  assert.match(SRC, /只对口味类纠正成立/);
  assert.match(SRC, /不要把这条错误主张写进记忆/);
});

test("方案本身有问题时要在动手前说——最常见的谄媚不是假话，是沉默", () => {
  assert.ok(TRUTH.includes("silently implementing a plan you believe is wrong"),
    "缺少「方案有问题要先说」这条——而它正是最常见的那种谄媚");
  // 必须写明它和「只做被要求的事」不冲突，否则两条规则会对撞，而让路的总是这条。
  assert.ok(TRUTH.includes('not unrequested honesty'));
  assert.ok(TRUTH.includes("Say it once"), "没写「说一次」会退化成每轮一段风险清单");
});

test("部署失败不能报成功——这是唯一会跨出 IDE 变成对外承诺的假成功", () => {
  // 原来是 `curl -sS`（没有 --fail），网关返回 401/413/500 一律退出 0，set -e 不触发，
  // 紧跟着那句「可直接访问分享」是**无条件**打印的。模型读到它就告诉用户「部署好了，
  // 链接给你」——而那是个 404，用户把它发给别人之后才发现。
  const i = SRC.indexOf("mi-deploy.tar.gz");
  assert.ok(i >= 0, "deploy_site 的命令不见了");
  const cmd = SRC.slice(SRC.lastIndexOf("`", i - 200), SRC.indexOf("`;", i) + 1);
  assert.match(cmd, /-w '%\{http_code\}'/, "没有取回 HTTP 状态码，就无从判断成没成");
  assert.match(cmd, /if \[ "\$code" != "200" \]/, "没有按状态码判定成败");
  assert.match(cmd, /exit 1/, "失败时必须非零退出，否则上游仍会当成功");
  // 成功文案必须在判定之后，否则又回到无条件打印。
  assert.ok(cmd.indexOf('if [ "$code" != "200" ]') < cmd.indexOf("可直接访问分享"),
    "成功文案排在状态码判定之前——失败时照样会打印");
  assert.match(cmd, /不要把链接给别人/, "失败时要明说没有可用地址，否则模型仍可能给出链接");
});

test("红灯和绿灯必须用同一套判据，否则「不声明 + 跑个失败的测试」是最省事的过关路径", () => {
  // 发绿灯的 _evidenceCertifies 只看执行期盖上的 verifierRecognized，不看 purpose；
  // 而判红灯的 _freshBuildFailure 原来额外要求 purpose === "verify"。于是模型跑
  // `npm test` 不声明 purpose：过了拿满学分，挂了被直接跳过、照常宣布完成。
  const red = SRC.slice(SRC.indexOf("function _freshBuildFailure"), SRC.indexOf("function _freshBuildFailure") + 1800);
  const green = SRC.slice(SRC.indexOf("function _evidenceCertifies"), SRC.indexOf("function _evidenceCertifies") + 1800);
  assert.doesNotMatch(red, /e\.purpose !== "verify"/,
    "判红灯又要求声明 purpose 了——绿灯不要求，这个不对称就是一条过关捷径");
  for (const seg of [red, green]) {
    assert.match(seg, /verifierRecognized !== true/, "两侧都必须只认执行期盖的 verifierRecognized");
    assert.match(seg, /implementationVersion !== implOps/, "两侧都必须要求证据比最后一次改动新");
  }
});

test("验证义务的扩展名表要覆盖这个 IDE 最常改的那几类", () => {
  // 这张表是两道验证门唯一的入口。漏掉 html/css/json 等于：改一版 CSS 那一轮
  // **整轮零验证义务**——而界面恰恰是这个 IDE 最常做的交付。
  const m = SRC.match(/const _CODE_FILE_RE = \/([^/]+)\//);
  assert.ok(m, "_CODE_FILE_RE 不见了——两道验证门会一起失去入口");
  for (const ext of ["html", "css", "scss", "json", "yaml", "toml"]) {
    assert.ok(m[1].includes(ext), `验证义务漏掉了 .${ext}`);
  }
});

test("「改了代码没验证」只记账不补回合——这是刻意的，别再改回去", () => {
  // 我一度把它改成推提醒并续跑，被两条测试拦下，而它们是对的：红构建是**观测到失败**，
  // 是「已完成」为假的直接证据；「没验证」观测到的是**缺席**，缺席不等于工作是坏的。
  // 拿缺席去覆盖模型的收尾判断，就是用 harness 的偏好压过它的判断。
  const loop = SRC.slice(SRC.indexOf("function _runAgenticLoop"));
  assert.match(loop, /run\._incompleteReason = "code_delivered_unverified"/,
    "缺席必须记账，否则这一轮看起来就像验证过了");
  assert.ok(!/_pushNudge\("codeVerify"/.test(loop),
    "又把它改成强制补回合了——见 logic.test.mjs 里那两条守卫");
  // 记了账就必须到得了用户：outcome 变 partial，然后作为一枚建议按钮出现。
  assert.match(SRC, /code_delivered_unverified: "跑一遍验证刚才的改动"/,
    "标签没有对应的人话，用户看到的会是「继续完成剩余部分」这种废话");
});

test("中文查询必须搜得到——原来整段汉字是一个 token，几乎必然落空", () => {
  // 这个 IDE 的用户大多用中文提问。原来 `[一-鿿]+` 把「用户登录校验在哪」当成一个
  // token，它和注释里的「登录校验」永远不相等，BM25 里 df===0 直接跳过——
  // 也就是说「搜不到」和「不存在」在中文上长得完全一样，而这正是最容易骗到人的一种。
  const i = SRC.indexOf("function _tokenize");
  assert.ok(i >= 0);
  const tokenize = new Function("_BM25_STOP",
    `${SRC.slice(i, (() => { let d = 0, j = SRC.indexOf("{", SRC.indexOf(")", i)); for (; j < SRC.length; j++) { const c = SRC[j]; if (c === "{") d++; else if (c === "}") { d--; if (!d) break; } } return j + 1; })())}\nreturn _tokenize;`,
  )(new Set());

  const q = tokenize("用户登录校验在哪");
  const doc = tokenize("这里做登录校验");
  const shared = q.filter((t) => doc.includes(t));
  assert.ok(shared.length >= 2, `中文查询和文档没有共同 token（${shared.join("/") || "空"}）——搜索对中文是瞎的`);
  assert.ok(shared.includes("登录") && shared.includes("校验"), "二元切分没覆盖到实际词");
  // 整段那个 token 要保留：完整短语命中时它仍然是一次强匹配。
  assert.ok(q.includes("用户登录校验在哪"));
  // ASCII 的行为一个字都不能变。
  assert.deepEqual(tokenize("getUserPermission"), ["getuserpermission", "get", "user", "permission"]);
});

test("检索截断必须说出来——「搜到上限」和「一共就这么多」不能同形", () => {
  // 原来摘要写「${hits} 处匹配」，而 hits 被 HIT_CAP 封在 150：真有 500 处时它照样说
  // 「150 处匹配」。调用方读到的是一个完整答案，于是停止追查——这是最容易让人停下来的
  // 一种假话。
  const i = SRC.indexOf("const HIT_CAP = 150");
  assert.ok(i >= 0, "搜索的上限常量不见了");
  const seg = SRC.slice(i, i + 4200);
  assert.match(seg, /已截断/, "达到上限时没有任何提示");
  assert.match(seg, /不要当成全部结果/, "没有明确告诉调用方还有没看到的");
  assert.match(seg, /_totalHits/, "没有统计真实总数，就无从判断有没有被截断");
});

test("检索结果按命中数排序——字母序会把最相关的文件埋掉，而截断从末尾砍", () => {
  // 后端刚按命中数排好（files.rs 里注释写明「以前是纯字母序，会把 30 处命中的文件埋在
  // 一个偶然命中 1 处的文件下面」），前端原来一行 localeCompare 把那次修复整个撤销了。
  const i = SRC.indexOf("const fileMatches = [...matchesByPath.values()]");
  assert.ok(i >= 0);
  const seg = SRC.slice(i, i + 400);
  assert.match(seg, /_hitsOf\(b\) - _hitsOf\(a\)/, "又变回按路径字母序了");
  assert.match(seg, /localeCompare/, "同分时仍需稳定次序，否则结果不可复现");
});

test("get_diagnostics 不带 path 时，「什么都没查」不能说成「没有问题」", () => {
  // 带 path 那条腿半年前就修对了（读不到就明说未被分析），而不带 path 时
  // getProblemMarkers() 只报**已打开**文件的标记——一个文件都没开时返回「无错误或警告」，
  // 那句话的真实含义是「我一个文件都没看」。而不带 path 恰恰是推荐的整体自检用法。
  assert.match(SRC, /当前没有任何文件处于语言服务分析中/, "「什么都没查」仍然和「没问题」同形");
  assert.match(SRC, /这不等于「项目没有问题」/);
  assert.match(SRC, /这份快照只覆盖这 \$\{_openFiles\} 个文件，不是整个项目/,
    "有打开文件时也必须说清作用域，否则会被当成全项目结论");
});
