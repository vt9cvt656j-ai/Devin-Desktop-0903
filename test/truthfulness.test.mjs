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
const PLAN = readFileSync(join(HERE, "..", "..", "server", "prompts", "plan.txt"), "utf8");

/** 取一个具名函数的开头一段源码，用来对结构下断言。 */
function fnBody(name, len = 1400) {
  const i = SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `${name} 不见了`);
  return SRC.slice(i, i + len);
}

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

test("git_stash_list 命令失败不能吞成「堆栈为空」——那会让人以为改动丢了", () => {
  // 原来后端 `if !out.status.success() { return Ok(Vec::new()) }`，前端于是印
  // 「(stash 堆栈为空)」。用户刚 stash 完切分支回来看到这句会以为改动没了；更糟的是
  // 模型据此判定「没有需要恢复的东西」，跳过 stash_pop 直接在工作区上继续写。
  const GIT = readFileSync(join(HERE, "..", "src-tauri", "src", "git.rs"), "utf8");
  const i = GIT.indexOf("pub fn git_stash_list");
  assert.ok(i >= 0);
  const seg = GIT.slice(i, i + 900);
  assert.doesNotMatch(seg, /return Ok\(Vec::new\(\)\);/, "命令失败仍然被吞成空列表");
  assert.match(seg, /return Err\(format!\(/, "失败必须报错，让前端的 catch 接住");
});

test("docker_compose_up 的结论必须来自容器状态，不是 up -d 的退出码", () => {
  // `up -d` 退出 0 只代表容器被创建并启动过——里面的进程起来就崩时它照样是 0。
  // ps 的输出本来就已经取到了，原来只当装饰贴在成功文案后面。
  const i = SRC.indexOf("Docker Compose 启动成功");
  assert.ok(i >= 0);
  const seg = SRC.slice(Math.max(0, i - 3000), i);
  assert.match(seg, /_badStates/, "没有解析容器状态，结论仍然只看退出码");
  assert.match(seg, /未真正跑起来/, "容器没 running 时仍然会报成功");
  assert.match(seg, /不要告诉用户服务已经可用/, "没有明确阻止把「已启动」转述给用户");
});

test("get_diagnostics 对没有语言服务的语言不能报「无错误或警告」", () => {
  // 内置 worker 只覆盖 TS/JS/JSON/CSS/HTML。改完 Python/Rust 调它拿到全绿，
  // 然后向用户报告「已修复并验证通过」——而一条都没检查过。
  assert.match(SRC, /_BUILTIN_DIAG_LANGS/, "没有区分「有没有诊断提供方」");
  assert.match(SRC, /没有语言服务在给它出诊断/);
  assert.match(SRC, /不能当成「没有问题」/);
  // 原来那句免责说「分析可能略有延迟」，暗示再等等就准了——而真相是等到天亮也是空的。
  assert.match(SRC, /要验证它，跑项目自带的类型检查/, "没有给出真正能验证的替代路径");
});

// ── 全量工具面的「假成功」清扫（31 条 high）──────────────────────────
//
// 统一判据：没做成 / 只做了一部分 / 作用域比调用方以为的小，返回文案必须与「确认无事」
// 在字面上不同。下面按类钉住，避免任何一条被悄悄改回去。

test("裸 slice 一律换成留痕的截断", () => {
  // 「被截断」和「就这么多」同形，是最容易让人停止追查的一种假话。而更阴的是：同一段
  // 文案里已经有另一个截断标记（「响应体已截断到 5MB」），于是「没有标记」被读成「完整」。
  assert.match(SRC, /function _clip\(text, cap, what/, "共用的截断函数不见了");
  assert.match(SRC, /\[已截断\][\s\S]{0,80}不要当成全部/);
  // 三条 HTTP 通道 + 子智能体简报 + 检索结果都要走它。
  for (const site of ["响应内容", "重发的响应内容", "子智能体的简报", "检索结果"]) {
    assert.ok(SRC.includes(`, 8000, "${site}")`), `这处还在裸 slice：${site}`);
  }
});

test("gh / git：退出码就是结论，不能把原始输出原样丢回去", () => {
  assert.match(SRC, /CI 全绿（gh pr checks 退出码 0/, "gh_pr_checks 仍然不看退出码");
  assert.match(SRC, /没有任何检查.{0,30}不是「全绿」/, "「没有 CI」和「CI 通过」仍然同形");
  assert.match(SRC, /这不等于没有评论.{0,10}是没查成/, "gh_pr_review_comments 仍然不看退出码");
  assert.match(SRC, /最近 \$\{runs\.length\} 次 Actions run 里\*\*没有失败的\*\*/,
    "挑不到失败的 run 时仍然会拿一次成功的顶上");
  assert.match(SRC, /没有匹配到任何失败关键词/, "日志采样方式仍然不自报");
  assert.match(SRC, /git add -A 失败[\s\S]{0,120}没有提交/, "git_commit 仍然吞掉暂存失败");
  assert.match(SRC, /git diff 没有输出/, "git_diff 的空输出仍然说成「无改动」");
});

test("plan 模式不再被提示词逼着以「切到 Agent 模式」收尾", () => {
  // 回答一渲染完，正下方就挂着一个「用 Agent 执行此方案」的按钮——点一下自动切模式并
  // 发起实施。而提示词强制模型每次都写一句"做完了建议你切到 Agent 模式来实现"，
  // 讲的正是那个按钮已经在做的事。这是提示词层面**强制**产出的结尾废话。
  assert.doesNotMatch(PLAN, /suggest the user switch to Agent mode/i,
    "每一份方案的结尾都被逼着写一句纯套话");
  assert.match(SRC, /用 Agent 执行此方案/, "切模式的入口是那个按钮，它必须还在");
  // 方案本身「不改文件、不跑命令」的约束不能跟着一起删掉。
  assert.match(PLAN, /Plan only — do not change files, do not run commands\./);
});

test("检索类：说清楚「没找到」还是「没去找」", () => {
  // 这条原来钉的是 /扫描没走完/ —— 而那句话在 main.js 里**只剩注释里的一句引用**，
  // 真正的实现早就改成了两条具名的 notes。也就是说：功能整个删掉它照绿，只删一行注释
  // 它才红。断言被自己要守的那段代码的注释喂饱了，这个坑本仓踩过不止一次。
  // 改成剥掉注释之后钉**真实文案**，并顺手证明旧那句确实只活在注释里。
  const CODE = SRC.replace(/^\s*\/\/.*$/gm, "").replace(/\/\*[\s\S]*?\*\//g, "");
  assert.doesNotMatch(CODE, /扫描没走完/,
    "又把断言钉回一句只活在注释里的话了 —— 剥掉注释之后它必须不存在");
  assert.match(CODE, /只给了 \$\{out\.length\} 条就停了（上限 \$\{MAX\}）/,
    "find_files 收满上限时不说清「这不是字典序的前 N 个」");
  assert.match(CODE, /扫描到 \$\{MAX_SCAN\} 个条目的上限就停了，剩下的目录一个都没看过/,
    "find_files 扫满上限时不说剩下的目录没看过");
  assert.match(CODE, /无匹配文件——已完整遍历，确实不存在/,
    "遍历完整时也不敢说「确实不存在」，模型就没法据此收工");
  assert.match(CODE, /已看过的部分里无匹配/,
    "扫描被截断时说成了「无匹配文件」—— 那是「没去找」，不是「没找到」");
  assert.match(SRC, /符号索引只覆盖/, "find_symbol 不说索引作用域");
  assert.match(SRC, /在\*\*已建索引的部分\*\*里/, "semantic_search 不说索引作用域");
  // 这条原来钉的是一句**假话**：它和后端 skip_walk_entry 同 commit 写下，四小时后后端就
  // 改成「文件一律保留」（那边的注释写得很清楚：.env / .eslintrc / .gitignore 恰恰是最常被
  // 搜的那批配置），而这句话没人更新。.github 也从来不在跳过名单里。
  // 后果比「少说一句」更糟：模型据此认为「搜不到是因为范围不够」，被推去猜路径直接 read_file。
  assert.doesNotMatch(SRC, /扫描范围不含点开头的目录和文件/,
    "又把「点开头的目录和文件全不扫」这句假话写回去了 —— 后端只跳几个具名目录，文件一律保留");
  assert.match(SRC, /点开头的文件照常搜/, "search 没说清点开头的文件其实在扫描范围里");
  assert.match(SRC, /跳过的只有几个具名的构建\/缓存目录/, "search 没说清到底跳了什么");
  // 也不许换成另一句假话：search 根本不调 path_is_git_ignored。
  assert.doesNotMatch(SRC, /被 \.gitignore 忽略的.{0,12}不搜/,
    "用新假话替旧假话了 —— search 不查 .gitignore");
  // 后端的截断标志要真的被读。它有三道上限（单文件 50 处命中、整次 2000 处、扫 20000 个
  // 文件），触到就 break 并回报 truncated——而智能体那条搜索路径原来一次都没读过它。
  // 后果是假阴性：「把这个变量所有调用点都改掉」，某文件里 80 处引用只看到前 50 处，
  // 模型按"一共就这么多"改完收工。
  assert.match(SRC, /if \(scopedMatches && scopedMatches\.truncated\) _backendTruncated = true;/,
    "后端的截断标志在智能体搜索路径上被丢掉了");
  assert.match(SRC, /\*\*后端没搜完\*\*：单文件命中上限 50 处/,
    "没搜完要说出来，且要说清补救动作是缩小范围重搜");
  // 0 命中那条分支也要改口——「没找到」和「没去找」在这里最容易被当成同一件事。
  assert.match(SRC, /\*\*这次没搜完\*\*（触到后端的命中数\/扫描文件数上限就停了）/,
    "没搜完时的 0 命中仍然说成「无匹配」");
  // 两种截断分开说：前端是"我手上有更多没列"，后端是"根本没搜完"，补救动作不同。
  assert.match(SRC, /const _frontTruncated = hits < _totalHits/);
  assert.match(SRC, /这只是这一个域的结论/, "knowledge_search 不说它只查了一个域");
  assert.match(SRC, /环形缓冲的当前长度，不是本次抓包的总量/, "capture_flows 拿缓冲长度冒充总量");
});

test("动作没发生就不许报成功", () => {
  assert.match(SRC, /invalidMethod/, "computer 仍然把不认识的 method 静默降级");
  assert.match(SRC, /这次什么都没做/, "降级仍然没有变成显式失败");
  assert.match(SRC, /_batchDropped/, "browser batch 超限的步骤仍然被静默丢弃");
  assert.match(SRC, /后面 \$\{call\._batchDropped\} 步一个都没跑/);
  // 说出来还不够：这条文案的标记词必须能被权威失败判定认出来，否则一次被砍掉一半的批次
  // 仍然算「执行成功」，后面的交互验证照样盖章。「未全部执行」里没有连着的「未执行」三个字。
  assert.doesNotMatch(SRC, /\[未全部执行\]/, "标记词躲开了失败判定的关键词表");
  const failMatch = fnBody("_toolFailureMatch", 600);
  assert.ok(/未执行/.test(failMatch), "失败判定认「未执行」");
  assert.match(SRC, /\[未执行完\] 你给了/, "截断的批次要被判成失败");
  // 结构化的 ok 也要跟上——文案匹配只是兜底。
  assert.match(SRC, /ok: !call\._batchBroken && !\(call\._batchDropped > 0\)/,
    "截断的批次不能返回 ok:true");
  // 而「交互已验证」这道章：只能盖在真正跑过的那一段上，且截断时一律不盖。
  const passed = fnBody("_browserActionPassed", 900);
  assert.match(passed, /call\._batchBroken \|\| call\._batchDropped > 0/,
    "截断的批次不能certify交互验证");
  assert.match(passed, /Array\.isArray\(call\._executedSteps\) \? call\._executedSteps : call\.steps/,
    "要在执行过的步骤里找交互，不是模型发来的全部步骤");
  assert.match(SRC, /call\._executedSteps = steps;/, "执行过的那一段要挂出来给判定读");
  assert.match(SRC, /轮次用尽·未完成/, "子智能体轮次用尽仍然冒充最终简报");
  assert.match(SRC, /running 只表示这个终端标签页还开着/, "run_in_terminal 仍拿 PTY 当命令");
  assert.match(SRC, /这次没有查成.{0,10}不是「没有引用」/, "lsp_references 定位失败仍落到「未找到」");
});

test("Rust 侧：三处会被当成结论的空值", () => {
  const AX = readFileSync(join(HERE, "..", "src-tauri", "src", "accessibility.rs"), "utf8");
  const KN = readFileSync(join(HERE, "..", "src-tauri", "src", "knowledge.rs"), "utf8");
  const AI = readFileSync(join(HERE, "..", "src-tauri", "src", "ai.rs"), "utf8");
  const BR = readFileSync(join(HERE, "..", "src-tauri", "src", "browser.rs"), "utf8");
  assert.match(AX, /read_error: Option<String>/, "读屏失败仍和「这个 app 没有 AX 树」同形");
  assert.match(AX, /The UI-tree read DID NOT COMPLETE/);
  assert.doesNotMatch(KN, /totalResults"\]\.as_u64\(\)\.unwrap_or\(0\)/, "CVE 查询失败仍被当成「没有漏洞」");
  assert.match(KN, /registry 搜索接口本次失败/, "npm 降级到单点查询仍不说明");
  assert.match(AI, /本页正文共 \{total\} 字符/, "web_fetch 24000 截断仍无标记");
  assert.match(AI, /没有任何来源返回结果/, "web_search 表头仍写死三引擎合并");
  assert.match(BR, /上面的 JSON 很可能是半截的/, "browser eval 8000 截断仍无标记");
});

// ── 安全边界：clone 一个仓库不该等于交出这台机器 ──────────────────────
//
// 这三条是同一个根因的三种形态：**信任是按文件名猜的，不是按来源定的**。
// `.mrdayone/settings.json`、`.mrdayone/settings.local.json`、`.mcp.local.json` 默认都不在
// gitignore 里，都能被提交、跟着 clone 到受害者机器上。

test("仓库里的 permissions.allow 必须被丢弃——它会短路唯一那道高危弹窗", () => {
  const i = SRC.indexOf("const merged = { allow: [], ask: [], deny: [] }");
  assert.ok(i >= 0, "权限规则加载器不见了");
  const seg = SRC.slice(i, i + 2200);
  assert.match(seg, /bucket === "allow" && !trusted/, "allow 仍然接受工作区文件");
  // 放行必须由用户本人给出；收紧任何来源都算数。
  assert.match(SRC, /absorb\(localStorage\.getItem\("michael-ide\.permissions"\)[^)]*trusted: true/);
  // 工作区那几份文件必须以 trusted:false 读入。两处（权限规则、能力声明）都要。
  assert.equal((SRC.match(/trusted: false/g) || []).length, 2,
    "工作区文件仍被当成可信来源（应有两处：权限规则 + 能力声明）");
});

test("仓库里的能力声明只能关能力，不能开能力", () => {
  // 我上一版给 userhttp 设了 needsApproval: true 并写了「一律要审批」——**在默认模式下
  // 那句话是错的**：mustAsk 只在 mode === "approve" 时才看 needsApproval，而默认是 auto。
  // 于是 clone 一个仓库就等于给对方一个常驻出网通道。
  assert.match(SRC, /if \(trusted\) \{ scopes\.push\(one\); return; \}/, "仓库声明仍被整份采纳");
  assert.match(SRC, /跟着 git clone 下来的文件不能给自己加能力/, "没有把原因告诉用户");
  // disabled 仍然任何来源都认——那是收紧。
  assert.match(SRC, /disabled: one\.disabled/);
});

test("打开外部链接不许经过 shell", () => {
  // 市场里的「查看仓库」按钮，URL 来自第三方注册表（PulseMCP 的 source_code_url）。
  // 原来是 `taskRunCapture("/", 'open "' + url + '"')`——双引号 shell 串里 `"`、`$()`、
  // 反引号全部有效，点一下按钮就是任意命令执行，而且 cwd 传 "/" 连沙箱都不设防。
  // 剥注释再断言：这个文件和 main.js 里都有注释在**引用**那段旧代码来解释它为什么危险，
  // 不剥的话断言会被解释文字喂到，而真代码删没删都测不出来。（这个坑这轮踩过三次。）
  const codeOnly = SRC.split("\n").filter((l) => !/^\s*(\/\/|\*|\/\*)/.test(l)).join("\n");
  assert.doesNotMatch(codeOnly, /taskRunCapture\("\/", *'open "' *\+/, "又把 URL 拼进 shell 了");
  assert.match(SRC, /function _openExternalUrlSafe\(url\)/);
  assert.match(SRC, /u\.protocol === "http:" \|\| u\.protocol === "https:"/, "没有校验协议");
  // 仅剩的一处走 shell 打开的是本地证书路径，三个平台三条命令，但**都必须转义**。
  // 窗口要卡在这条语句本身，不能按字节数开——多开一点就会溢进下面那条通知文案，
  // 那里合法地用着 `${p}`，断言会被它喂到。（窗口越界这坑这轮也踩过。）
  const openStart = SRC.indexOf("const openCmd = _isWin");
  const openBlock = SRC.slice(openStart, SRC.indexOf("taskRunCapture(", openStart));
  assert.ok(openBlock.length > 50, "找不到打开证书那段");
  assert.match(openBlock, /shellQuote\(p\)/, "证书路径没转义就拼进了 shell");
  assert.match(openBlock, /cmd \/c start ""/, "Windows 上没有 open，要走 start");
  assert.match(openBlock, /xdg-open/, "Linux 上要走 xdg-open");
  // 路径来自我们自己的后端，但转义这件事不能因为"这次可控"就省掉——那是给后人留先例。
  assert.doesNotMatch(openBlock, /\$\{p\}/, "又把裸路径插进 shell 串了");
});

// 工具描述也算提示词，而且是**运行时会覆盖本地那份**的提示词：
// _applyCloudToolDescs 按名字用 server/prompts/tools.json 里的 description + parameters
// 顶掉 main.js 里的兜底。所以描述里的假话，比回执里的假话传得更远——模型在**调用之前**
// 就照着它规划。sync-tools-json.mjs 的 --check 只拦「两边写得不一样」，
// 两边写着同一句假话它是不会响的。
test("工具描述里的三句假话：stash 清空工作区 / auto_rig 一定出 glb / git_log 看全部分支", () => {
  const CATALOG = readFileSync(join(HERE, "..", "..", "server", "prompts", "tools.json"), "utf8");
  const both = [["main.js", SRC], ["tools.json", CATALOG]];

  for (const [where, text] of both) {
    // ① git stash **不带走未跟踪文件**，工作区不会被清空。实测：只有未跟踪文件时
    //    git 照样印 "No local changes to save"，文件原地不动。
    assert.doesNotMatch(text, /Stash the current working-tree changes onto the stash stack and clear the working tree/,
      `${where}: git_stash 又说自己会清空工作区 —— 未跟踪的新文件根本不会被带走`);
    assert.match(text, /\*\*Untracked \(new\) files are NOT taken and the working tree is not left clean\*\*/,
      `${where}: git_stash 没说清未跟踪文件不会被带走`);

    // ② auto_rig 的落盘扩展名是写死的 glb，而上游可能返回打包的 FBX；
    //    后端会按魔数改名，描述不能承诺一定是 .glb。
    assert.doesNotMatch(text, /outputs an animatable \.glb\./,
      `${where}: auto_rig 又承诺一定输出 .glb —— 上游给 zip/fbx 时文件会按真实格式改名`);
    assert.match(text, /go by the path in the receipt, not by the extension you expected/,
      `${where}: auto_rig 没告诉模型以回执里的路径为准`);

    // ③ git_log 现在只看当前分支（后端 all 默认 false）。
    assert.match(text, /Show recent commit history \*\*on the current branch\*\*/,
      `${where}: git_log 没说清只看当前分支 —— 模型会以为这是全仓最近的提交`);

    // ④ stash pop 撞冲突时 stash 条目会保留，回执里带完整冲突报告。
    assert.match(text, /git keeps the stash entry in place and the receipt carries the full conflict report/,
      `${where}: git_stash_pop 没说冲突时条目还留着，模型会重复 pop 或直接 drop`);
  }
});
