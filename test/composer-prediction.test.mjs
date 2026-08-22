// 输入框那条灰字预测（Tab 补全）的档位顺序。
//
// 起因：用户报「预测内容不会实时变动，而且内容老是错的」。查下来不是没重算——
// activate() → _refreshChatHintIfEmpty() → _renderComposerGhost() 这条链是通的，
// 诊断和选区变化也有 400ms 防抖重算。问题在**档位顺序**：
//
//   `_runStateNextActionSuggestions` 默认只信 5 分钟内的运行状态，但「继续上次」那一档
//   特意传了 maxAgeMs: Infinity。它永不过期，于是只要这个会话跑过一次东西，5 分钟后
//   前面几档静默之后，它就永远顶在最上面，把最后那档「基于当前文件」的预测彻底压住。
//   用户切文件、改选区、看着新代码，灰字纹丝不动，推的还是一件早就做完的事。
//
// 修法：那一档只在**这个会话里用户还没发过话**时成立——那才是"接着上次继续"真正
// 有意义的一刻（打开软件）。一旦用户开口，当前正在看的东西就是更新鲜的证据。
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
// 从 main.js（无导出的浏览器单文件）里按名字抠真源码：全仓唯一那份提取器。
import { fnSource as grab } from "./helpers/source.mjs";

const SRC = fs.readFileSync("src/main.js", "utf8");


// 只桩掉两个外部依赖，_predictionAlreadyUsed 用真身——它参与判定。
const deps = {
  _runStateNextActionSuggestions: (sess, { maxAgeMs = 5 * 60_000 } = {}) => {
    const run = sess?._lastRunState;
    if (!run || Date.now() - run.updatedAt > maxAgeMs) return [];
    if (run.outcome === "awaiting_user") return [];
    return [{ label: run.label, send: run.send }];
  },
  _dynamicChatChips: () => [{ label: "解释 database.py", send: "解释 database.py：整体职责、关键函数/类型、数据流。" }],
  _predictionAlreadyUsed: new Function(`return ${grab("_predictionAlreadyUsed")}`)(),
};
const keys = Object.keys(deps);
const _composerPrediction = new Function(
  ...keys,
  `${grab("_composerPrediction")}; return _composerPrediction;`,
)(...keys.map((k) => deps[k]));

const STALE = { updatedAt: Date.now() - 3 * 60 * 60 * 1000, label: "把爬虫接上数据库", send: "继续：把爬虫接上数据库" };

test("用户开口之后，预测跟着当前文件走，而不是被三小时前的运行状态钉死", () => {
  const sess = { id: "s1", _lastRunState: STALE, _recentSent: ["你好啊"] };
  const p = _composerPrediction(sess);
  assert.equal(p.src, "ctx", `期望跟随当前上下文，实际是 ${p.src}：${p.text}`);
  assert.match(p.send, /database\.py/);
});

test("刚打开软件、这个会话还没发过话时，「继续上次」必须还在", () => {
  const sess = { id: "s2", _lastRunState: STALE, _recentSent: [] };
  const p = _composerPrediction(sess);
  assert.equal(p.src, "resume");
  assert.match(p.text, /继续上次/);
});

test("刚跑完一轮（5 分钟内）仍然优先接着那一轮，别被当前文件抢走", () => {
  const sess = { id: "s3", _lastRunState: { ...STALE, updatedAt: Date.now() - 1000 }, _recentSent: ["做个爬虫"] };
  assert.equal(_composerPrediction(sess).src, "run");
});

test("举着问题等人回答时不给预测", () => {
  const sess = { id: "s4", _lastRunState: { ...STALE, updatedAt: Date.now() - 1000, outcome: "awaiting_user" }, _recentSent: [] };
  assert.equal(_composerPrediction(sess), null);
});

test("计划里还有未完成的步骤时，接着计划走", () => {
  const sess = {
    id: "s5",
    _planSteps: [{ status: "pending", content: "把 crawler 接上 SQLite" }],
    _recentSent: ["开始吧"],
  };
  const p = _composerPrediction(sess);
  assert.equal(p.src, "plan");
  assert.match(p.send, /^继续：/);
});

test("源码里那道闸真的在——去掉它这组测试就没意义了", () => {
  const fn = grab("_composerPrediction");
  assert.match(fn, /_recentSent[^\n]*\)\.length\) \{[\s\S]{0,200}maxAgeMs: Infinity/,
    "「继续上次」那一档必须被「本会话还没发过话」这个条件包住");
});

// ── 琐碎轮不该付深思考的钱 ────────────────────────────────────────────────
// 截图实证：一句"你好啊"，总耗时 21s、模型 5.6s 就开始出字，剩下十几秒全花在
// "要不要问他修 bug / 要不要列菜单"的反复推演上。这类轮次代码里本来就已经在省
// 系统提示词、跳过技能块和工作区预热了（_shouldUseLightweightAgentTurn），
// 唯独思考预算照付默认档（Claude 4.6 的 high = 24000 budget_tokens）。
test("琐碎轮不再偷偷把思考降档——用户选什么就发什么", () => {
  // 这一档降级删掉了。它写在"客户端还发不出 effort"的年代：当时 Claude 走
  // budget_tokens=24000，一句"你好啊"确实会烧掉整份预算。现在是 adaptive + effort，
  // 深浅由模型每轮自己定，前提没了，留下的只有"用户选的和实际发的不一致"。
  //
  // 后来轻量轮**整套删掉了**（见 sendPrompt 里那段说明）：它省下的提示词，代价是判错时
  // 模型手上一个工具都没有、还不知道自己缺了什么。这条用例现在只管一件事——
  // 思考档位不许被任何东西偷偷改写。
  const _start = SRC.indexOf("function _applyThinkingToConfig(");
  assert.ok(_start > 0, "_applyThinkingToConfig 没找到");
  const fn = SRC.slice(_start, _start + 6000);
  assert.doesNotMatch(fn, /opts\.lightTurn/, "轻量轮又在改写档位了");
  assert.doesNotMatch(fn, /\["minimal", "low", "medium"\]\.find/, "又在往最浅档压");

  // 轻量轮已删干净：留一个字都可能让下一个人以为还能靠它省点什么。
  assert.doesNotMatch(SRC, /_agentLightTurn|_shouldUseLightweightAgentTurn/,
    "轻量轮又回来了——它会在判错时把工具和技能一起拿走");
});

// ── 空工作区不能推「深挖整个项目」 ───────────────────────────────────────────
//
// 用户截图：工作区 小说 是个空文件夹，助手上一条刚说完「当前目录完全为空，没有任何
// 文件」，而输入框的灰字还在推「用 research_project 深挖整个项目，给我一份上手地图：
// 技术栈、目录结构、核心模块职责……」。预测和事实当场打架。
//
// 成因：那一档只判「有没有根目录」，从不看根目录里有没有东西。
// 空目录该问的是"做点什么"，不是"读懂什么"——这是两种完全不同的建议。
function chipsWith(entryCount) {
  const zh = (() => {
    const I = fs.readFileSync("src/i18n.js", "utf8");
    const at = I.indexOf("const ZH_CN");
    return Function("return {" + I.slice(I.indexOf("{", at) + 1, I.indexOf("\n};", at)) + "}")();
  })();
  const deps = {
    activePath: "", _pathToRel: (x) => x, t: (k) => zh[k] ?? k,
    monacoEditor: { getSelection: () => null, getModel: () => null },
    monaco: { editor: { getModelMarkers: () => [] } },
    openFiles: new Map(), _isGeneratedDependencyDiagnostic: () => false, _lastGitFiles: [],
    rootPath: "/w", workspaceRoots: ["/w"],
    _workspaceRootEntryCounts: new Map([["/w", entryCount]]), _treePath: (x) => x,
  };
  const keys = Object.keys(deps);
  return new Function(...keys, grab("_dynamicChatChips") + "\n;return _dynamicChatChips;")(...keys.map((k) => deps[k]))();
}

test("空文件夹推的是「做点什么」，不是「深挖整个项目」", () => {
  const labels = chipsWith(0).map((c) => c.label).join(" | ");
  assert.ok(!/深挖/.test(labels), `空目录仍在推深挖：${labels}`);
  assert.match(labels, /开始做点什么|项目骨架/, `空目录该给可动手的建议：${labels}`);
});

test("有文件的项目照旧推「深挖整个项目」——别把好用的那档改坏了", () => {
  assert.match(chipsWith(37).map((c) => c.label).join(" | "), /深挖/);
});

test("还没扫完（undefined）不能当成空——刚打开大项目的头一秒不该推「新建」", () => {
  // 这个计数是异步扫出来的。把 undefined 当成 0，用户打开一个几千文件的仓库，
  // 第一眼看到的会是"这个文件夹是空的"。
  assert.match(chipsWith(undefined).map((c) => c.label).join(" | "), /深挖/);
});

test("扫完目录要通知预测重算，否则灰字停在扫描前那一版", () => {
  // 「预测不会实时变」的另一半：事实是异步到的，到了没人通知，预测就一直是旧的。
  const at = SRC.indexOf("_workspaceRootEntryCounts.set(_treePath(path), entries.length)");
  assert.ok(at > 0, "找不到计数写入点");
  assert.match(SRC.slice(at, at + 700), /_refreshChatHintIfEmpty\(\)/,
    "算出目录有多少东西之后没有触发预测重算");
});

test("四个新词条三本词典都要有，少一本那个语言就露出原始键名", () => {
  const I = fs.readFileSync("src/i18n.js", "utf8");
  for (const k of ["assistant.chip.startProject", "assistant.prompt.startProject",
                   "assistant.chip.scaffoldHere", "assistant.prompt.scaffoldHere",
                   "tool.action.skill"]) {
    const n = I.split(`"${k}"`).length - 1;
    assert.equal(n, 3, `${k} 只在 ${n} 本词典里`);
  }
});

// ── 预测的是「用户接下来会打什么」，不是「环境暗示该干什么」 ──────────────────
//
// 用户第二次纠正：「这里预测是预测用户接下来会问什么，具体要问的问题，而不是乱猜」。
// 上一轮我修的是"空目录别推深挖项目"——那只是把一个错答案换成另一个来源相同的答案。
// 真正的断层是：原有五档全部读**环境状态**（报错数、git 脏、开着哪个文件、目录空否），
// 一档都没读对话。助手刚分析完三个项目怎么变现，输入框却在推「这个文件夹是空的」，
// 两件事毫无关系——这就是"乱猜"。
//
// 所以加了一档真正读对话的，并且排在最前面。判据来自 Claude Code 那套：
// 预测"他会打什么"而不是"接下来该做什么"，说不准就沉默。
function rejectAsk() {
  const i = SRC.indexOf("function _rejectPredictedAsk");
  const tail = '\n  return "";\n}';
  const end = SRC.indexOf(tail, i);
  assert.ok(i > 0 && end > i, "找不到 _rejectPredictedAsk");
  return new Function(SRC.slice(i, end + tail.length) + "\nreturn _rejectPredictedAsk;")();
}

test("用户真会打的那些话要放行", () => {
  const rej = rejectAsk();
  for (const v of ["那先做压缩包大侠的上架", "帮我把自动化框架发到 crates.io",
                   "先补 README 的例子", "第 3 个，重点打中文市场", "这个能跑起来吗"]) {
    assert.equal(rej(v), "", `误杀了一句用户真会打的话：${v}`);
  }
});

test("助手口吻一律拦下——「我来帮你…」不是用户会打的字", () => {
  const rej = rejectAsk();
  assert.equal(rej("我来帮你发到 crates.io"), "assistant_voice");
  assert.equal(rej("让我先看看这个文件"), "assistant_voice");
});

test("反问用户、客套、空泛的一律拦下", () => {
  const rej = rejectAsk();
  assert.equal(rej("你想先做哪一个？"), "question_to_user");
  assert.equal(rej("不错"), "evaluative");
  assert.equal(rej("继续"), "too_generic", "「继续」任何时候都成立，等于没预测");
});

test("模型叙述自己在沉默也要拦——提示词说了它照样会漏", () => {
  // 软的那层（提示词里要求"不确定就输出空"）会漏，所以必须配一层硬的。
  const rej = rejectAsk();
  assert.equal(rej("（保持沉默）"), "meta_wrapped");
  assert.equal(rej("无法预测用户意图"), "meta_text");
});

test("长度按中文卡 32 字，不是照搬英文那套 100 字符", () => {
  const rej = rejectAsk();
  assert.equal(rej("帮我把这个项目从头到尾重新梳理一遍，包括技术栈选型、目录结构、核心模块职责和数据流"), "too_long");
  assert.equal(rej("先把自动化框架发到 crates.io。然后再补文档。"), "multiple_sentences",
    "用户不会一次打两句");
});

test("读过对话的那一档排在所有环境档之前", () => {
  const fn = grab("_composerPrediction");
  const askAt = fn.indexOf("sess._askPredict");
  const envAt = fn.indexOf("_runStateNextActionSuggestions(sess)");
  assert.ok(askAt > 0, "没有读对话的那一档");
  assert.ok(askAt < envAt, "对话档排在了环境档后面——环境档一命中就再也轮不到它");
});

test("对话往前走了，旧预测就作废", () => {
  // 不作废的话，用户发完新消息还会看到针对上一轮的那句预测。
  assert.match(grab("_composerPrediction"), /ask\.turnKey === \(\(sess\.memory && sess\.memory\.recent\) \|\| \[\]\)\.length/);
});

test("被拒的原因要记下来，「为什么这儿没有预测」得答得出来", () => {
  assert.match(SRC, /sess\._askPredictReject = why/);
  assert.match(SRC, /sess\._askPredictReject = "already_sent"/);
});

test("预测不能卡住收尾——只后台跑，不 await", () => {
  // 注意别匹配到函数定义那一处——第一个 _predictNextAsk(sess) 是 `async function` 的签名。
  assert.match(SRC, /void _predictNextAsk\(sess\)/,
    "收尾处没有以后台方式触发预测");
  assert.ok(!/await _predictNextAsk\(/.test(SRC),
    "await 了这次预测，模型慢一点整轮收尾就跟着卡");
});

test("没有预测时，要把「为什么没有」留下来——否则那句承诺是空的", () => {
  // _askPredictReject 记着读对话那一档被拒的具体原因（too_long / assistant_voice /
  // meta_text / already_sent …），但在接上读者之前**没有任何地方读它**：
  // 代码里那句「为什么这儿没有预测要答得出来」是句空话。
  const fn = grab("_renderComposerGhost");
  assert.match(fn, /sess\._askPredictReject/, "拒绝原因仍然没有读者");
  assert.match(fn, /本轮没有给出预测：/, "没有把原因呈现出来");
  // 有预测时要把上一次的原因清掉，否则 tooltip 会一直挂着一条过期的解释。
  assert.match(fn, /promptEl\.removeAttribute\("title"\)/, "有预测时没有清掉旧的解释");
});

test("默认底色是明写的，不是粘在别的规则行尾的碎片", () => {
  // 49 个工具类型里有 24 个没有专属配色，一直靠这两行上色；它原来粘在
  // --game_asset 那条规则的行尾，读起来像笔误，删掉那 24 个图标会变透明。
  // 先剥 CSS 注释：解释这条改动的注释里原样引用了 `{ … }`，不剥的话
  // 下面 [^}]* 会在注释里的那个花括号处断掉（这个仓库同一个坑踩过三次）。
  const APP_CSS = fs.readFileSync("src/styles/app.css", "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  assert.doesNotMatch(APP_CSS, /\.agent-tool-step--game_asset \.atc-type-icon \{[^}]*\}\s*\.atc-type-icon \{/,
    "无作用域的默认底色还粘在 --game_asset 行尾");
  assert.match(APP_CSS, /\.atc-type-icon \{[^}]*background: #ede7f6; color: #4527a0;[^}]*\}/s,
    "默认底色丢了——没有专属配色的 24 个工具图标会变成透明方块");
});
