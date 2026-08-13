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

const SRC = fs.readFileSync("src/main.js", "utf8");

/** 从 main.js（无导出的浏览器单文件）里抠出一个函数体，和其它测试同一套做法。 */
function grab(name) {
  const i = SRC.indexOf(`function ${name}(`);
  if (i < 0) throw new Error(`function ${name} not found`);
  let depth = 0, started = false;
  for (let k = i; k < SRC.length; k++) {
    if (SRC[k] === "{") { depth++; started = true; }
    else if (SRC[k] === "}") { depth--; if (started && depth === 0) return SRC.slice(i, k + 1); }
  }
  throw new Error(`unbalanced braces in ${name}`);
}

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
test("琐碎轮把思考降到最浅档，但显式选过档位的用户不受影响", () => {
  // 不用 grab：这个函数的注释里有 {"type":"enabled","budget_tokens":N} 这种花括号，
  // 会把朴素的括号计数带偏、body 被截断。直接按位置切一段足够长的源码。
  const _start = SRC.indexOf("function _applyThinkingToConfig(");
  assert.ok(_start > 0, "_applyThinkingToConfig 没找到");
  const fn = SRC.slice(_start, _start + 6000);
  assert.match(fn, /opts\.lightTurn/, "要有 lightTurn 这一档降级");
  assert.match(fn, /\["minimal", "low", "medium"\]\.find/, "降到该模型有的最浅档");
  // 显式选择优先：和它下面那条 max→high 的钳位用同一套判据
  const light = fn.slice(fn.indexOf("opts.lightTurn"), fn.indexOf("opts.agentTurn"));
  assert.match(light, /_loadThinkingPrefs\(\)\[preferenceId\]/, "要读用户显式选过的档位");
  assert.match(light, /if \(!explicit/, "显式选过就不降");
  // 不能降到 off：Fable/Mythos 没有 off（显式 disabled 是 400）
  assert.match(light, /pref !== "off"/, "已经关掉思考的不要再动");

  // 接线：判定出琐碎轮之后必须真的重设一次，否则这一档等于没写
  assert.match(SRC, /if \(_agentLightTurn\) \{[\s\S]{0,400}lightTurn: true/,
    "_agentLightTurn 判定之后要重新套一次思考配置");
});
