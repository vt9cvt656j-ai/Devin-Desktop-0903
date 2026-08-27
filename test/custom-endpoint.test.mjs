// 用户自己的模型端点（自己的 key、自建网关、Azure、本地 Ollama）。
//
// 这条链路以前不是没写过，是被**主动关掉**的：配置字段完整留着，但四道门把它拦死了，
// 而且顺序还是反的——先过会员门、再校验一组本轮根本用不到的网关字段，最后才把自定义
// 端点覆写进去。于是「用自己的 key 打自己的端点」也必须先登录本产品、账上还得有钱。
//
// 这里守两类性质：
//   1. 四道门确实拆了，而且**顺序**对（覆写在校验之前，否则只用自己端点的人会被
//      「请先登录账号」拦住）。
//   2. 该保留的东西没被顺手删掉——customModelId 仍要设置（思考档位偏好靠它绑定），
//      辅助调用仍然钉在网关上（拿网关的模型名去打用户端点必然失败、还烧用户的钱）。
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
import { CODE as SRC, SRC as RAW_SRC, fnSource as extractFn } from "./helpers/source.mjs";
import { load } from "./helpers/source.mjs";
import { cmProtocol, normalizeCustomModel } from "../src/agent/wire-protocol.js";

const READY = extractFn("_readyAiConfig");

test("自定义端点不再要求会员——钱是用户自己的", () => {
  // 这三句文案就是那三道会员门。它们还在，就说明门还在。
  assert.doesNotMatch(SRC, /自定义模型是会员专属功能/,
    "配置弹窗或发送前仍在按会员拦自定义端点");
  assert.doesNotMatch(SRC, /自定义模型为会员专属/,
    "模型选择器仍在按会员拦自定义端点");
});

test("覆写必须排在网关校验之前，否则只用自己端点的人会被「请先登录账号」拦住", () => {
  const override = READY.indexOf("config.baseUrl = _custom.baseUrl");
  const validate = READY.indexOf("if (!config.baseUrl || !config.apiKey)");
  assert.ok(override >= 0, "自定义端点的覆写不见了");
  assert.ok(validate >= 0, "网关字段校验不见了");
  assert.ok(override < validate,
    "顺序反了：仍然先校验网关字段再覆写，等于用一组本轮不会用到的字段去卡用户");
});

test("走自己的端点时跳过账号门，走网关时照旧要过", () => {
  // 判据必须是「这一轮是不是自定义端点」，而不是把门整个删掉——网关路径靠它鉴权和刷新
  // token，删了会让网关用户带着过期 token 一路发到底。
  assert.match(READY, /_customModelById\(_preCfg\?\.model\)/,
    "没有先判定本轮是不是自定义端点");
  assert.match(READY, /if \(!_customPre\) \{\s*\n\s*if \(!\(await michaelAccessGate\(\)\)\) return null;/,
    "账号门要么被整个删了、要么没有按「是否自定义端点」分流");
});

test("customModelId 仍然要设置——思考档位偏好靠它绑定", () => {
  assert.match(READY, /config\.customModelId = _custom\.id/,
    "顺手删掉了 customModelId：思考档位偏好会绑错，每轮读到的档位都是错的");
});

test("辅助调用仍然钉在网关上，不拿用户的额度去烧", () => {
  // 意图分类、记忆压缩这些内部调用用的是网关目录里的模型名，打到用户端点上必然失败，
  // 而且烧的是用户的钱。这条「钉死」是对的，不该被一起放开。
  //
  // 判据换成**正面**的：这个函数钉住网关靠的是无条件覆写 providerMode + baseUrl。
  // 早先那条 `doesNotMatch(/customBaseUrl|customApiKey|customModel/)` 抓不住真实回归 ——
  // 那三个名字是设置对象里被清空的遗留字段（main.js:13996/14028），这个函数从来没用过
  // 它们；把 `baseUrl: MICHAEL_API` 改成 `c.baseUrl` 才是真的放开，而那样改它照样绿。
  const runtime = extractFn("_aiConfigForRuntime");
  assert.match(runtime, /providerMode:\s*AI_PROVIDER_GATEWAY/,
    "运行时配置不再无条件走网关了——内部辅助调用会开始打用户端点、烧用户的钱");
  assert.match(runtime, /baseUrl:\s*MICHAEL_API/,
    "地址不再无条件是网关了——辅助调用会跟着用户的自定义端点跑");
  assert.doesNotMatch(runtime, /baseUrl:\s*(c|raw)\./,
    "地址被改成读入参了——那正是「放开到自定义端点」的样子");
});

test("必须如实告诉用户「走自己的端点会变弱」，否则这就是个陷阱", () => {
  // 工具描述由服务端按名回填、完整系统提示词也在服务端。第三方端点两头落空，
  // 智能体明显变弱，而界面上完全看不出来。不说清楚，放开这个开关等于交付一个坏功能。
  // 这句话原先是一条切端点时弹的 toast。用户要求去掉那条 toast（九秒的横幅、切一次弹
  // 一次），**但披露本身不能跟着消失** —— 那样就真成了「悄悄变弱」。它搬进了弹窗顶部的
  // 说明里：那是**填之前**就看得到的位置，比切完之后飘一条更早、也更该在那儿。
  //
  // 判据跟着搬到弹窗 markup 上。这里不验 _warnCustomEndpointOnce 了：它现在是个空壳。
  const dlg = extractFn("showCustomModelsDialog", { code: true });
  assert.match(dlg, /工具描述/, "弹窗里没说工具描述这件事");
  assert.match(dlg, /系统提示词/, "弹窗里没说系统提示词这件事");
  assert.match(dlg, /长上下文压缩/, "弹窗里没说长上下文压缩会关闭");
  assert.match(dlg, /弱一些/, "没说清「会变弱」这件事本身");
});

// ---- 线协议：六条真实调用链上的门 -----------------------------------------
// 全部用 fnSource 按 AST 取函数体，不用固定行窗口 —— 函数一变长，固定窗口就悄悄不再
// 守住尾部，而且仍然是绿的。

test("协议存得进也读得回——这是**往返**，不是「源码里有没有那行字」", () => {
  // 这条测试的前一版是 `assert.match(fnSource("_loadCustomModels"), /normalizeCustomModel/)`，
  // 它**恒真**：当时函数里还有一个定形的 .map（只列 id/group/name/baseUrl/apiKey），
  // normalizeCustomModel 接在它后面，拿到的 it.protocol 恒为 undefined。于是
  // 21 条测试全绿、协议功能整个是死的：选了 Anthropic 也照样打 /chat/completions。
  //
  // 教训是判据的形状不对，不是断言写得不够细：只要还在验「源码长什么样」，就永远
  // 验不到「它到底吐出什么」。所以这里把真函数取出来，配真的 localStorage 跑一遍往返。
  const store = {};
  const _loadCustomModels = load("_loadCustomModels", {
    localStorage: { getItem: (k) => store[k] ?? null, setItem: (k, v) => { store[k] = String(v); } },
    _CUSTOM_MODELS_KEY: "mrday.custom_models",
    _CUSTOM_MODEL_PREFIX: "custom:",
    cmProtocol,
    normalizeCustomModel,
  });

  const saved = [
    { id: "custom:a", group: "我的", name: "claude-sonnet-4-5", baseUrl: "https://api.anthropic.com", apiKey: "sk-a", protocol: "anthropic" },
    { id: "custom:b", group: "我的", name: "grok-4", baseUrl: "https://api.x.ai", apiKey: "sk-b", protocol: "xai_responses" },
    { id: "custom:c", group: "我的", name: "gpt-4o", baseUrl: "https://relay.example/v1", apiKey: "sk-c", protocol: "openai" },
    // 存量条目：根本没有 protocol 字段，必须落回 openai 且其余字段一个不丢
    { id: "custom:d", group: "老的", name: "gpt-4o-mini", baseUrl: "https://old.example/v1", apiKey: "sk-d" },
  ];
  store["mrday.custom_models"] = JSON.stringify(saved);

  const got = _loadCustomModels();
  assert.equal(got.length, 4, "过滤器把合法条目吃掉了");
  const by = Object.fromEntries(got.map((x) => [x.id, x]));

  assert.equal(by["custom:a"].protocol, "anthropic",
    "存 anthropic 读回来不是 anthropic —— 用户选了协议也不生效，而且全程零报错");
  assert.equal(by["custom:b"].protocol, "xai_responses", "xai_responses 在读取时被丢掉了");
  assert.equal(by["custom:c"].protocol, "openai");
  assert.equal(by["custom:d"].protocol, "openai", "存量条目没有 protocol 字段时必须落回 openai");

  // 其余字段一个都不能丢：保存/删除都走 _saveCustomModels(_loadCustomModels()…)，
  // 这里丢掉的字段会在下一次保存时被永久写没。
  assert.equal(by["custom:d"].name, "gpt-4o-mini");
  assert.equal(by["custom:d"].baseUrl, "https://old.example/v1");
  assert.equal(by["custom:d"].apiKey, "sk-d");
  assert.equal(by["custom:d"].group, "老的");
  assert.deepEqual(Object.keys(by["custom:a"]).sort(),
    ["apiKey", "baseUrl", "group", "id", "name", "protocol"],
    "读出来的形状变了 —— 多一个字段是脏数据，少一个是静默丢配置");
});

test("协议注入必须排在校验之前", () => {
  const i = READY.indexOf("config.protocol =");
  const j = READY.search(/if \(!config\.baseUrl \|\| !config\.apiKey\)/);
  assert.ok(i >= 0, "_readyAiConfig 没有注入 protocol——Rust 侧永远收不到，一律走 openai");
  assert.ok(j < 0 || i < j, "protocol 注入排在校验之后，提前 return 的分支上就注入不到");
});

test("@model: 切走时必须把 protocol 一起删掉", () => {
  // Object.assign 不删多余键。protocol 是完全同形的第七个键——不加进来，用 @model: 从
  // 一条自定义 Anthropic 条目切到**网关**模型时，残留的 protocol:"anthropic" 会把网关
  // 请求打到 {gateway}/v1/messages 带 x-api-key → 404，看着像网关挂了。下一轮又好了。
  //
  // 不匹配数组的字面排版（跑一次 formatter 就假红），只断言这个清键列表里有它。
  const m = SRC.match(/for \(const k of \[[^\]]*"customModelId"[^\]]*\]\)/);
  assert.ok(m, "找不到 @model: 的清键列表——这道门已经失效，别当它还在");
  assert.match(m[0], /"protocol"/,
    "清键列表漏了 protocol：从自定义 Anthropic 条目切到网关模型会 404，且下一轮自己好");
});

test("下一句预测：非 OpenAI 协议上宁可不发，也不发一个必然失败的请求", () => {
  // 预测是第三条发送路径，自己拼 /chat/completions + Bearer，不经过 Rust 的协议分叉。
  // 外层是 catch{}，连日志都没有——表现成「灰字预测在这个模型上不出现」，找不到原因。
  assert.match(SRC, /cmProtocol\(_cm\.protocol\) !== "openai"/,
    "预测路径没有协议守卫：Anthropic 端点上每轮都会白打一次 404，而且一声不吭");
});

test("网页构建：桌面专属协议要说清为什么，不能去打一个错端点", () => {
  const f = extractFn("_realAiFetch");
  assert.match(f, /cmProtocol\(config\.protocol\)/,
    "网页构建没有协议守卫：请求体/端点/鉴权头全是 OpenAI 形状，会换回一句看不懂的 404 或 CORS 错");
});

test("弹窗里协议选择器和缺口清单都在", () => {
  // 正面断言：这两个节点是 J5 那段 querySelector 的落点，缺任何一个 syncProto() 直接抛，
  // 弹窗整个白掉。不用 doesNotMatch 否定旧文案——改一个字它就绿，和协议是否真支持无关。
  assert.match(SRC, /class="cm-in-proto"/, "协议单选组不在——用户根本选不了协议");
  assert.match(SRC, /id="cmGapsProto"/, "缺口清单容器不在——「不许假装支持」在界面上就没有落点了");
  assert.match(SRC, /value="anthropic"/, "选项里没有 anthropic");
  assert.match(SRC, /value="xai_responses"/, "选项里没有 xai_responses");
});

test("Claude 别名认不出代次时不发思考参数", () => {
  // sonnet-latest 这类别名 _claudeGeneration 返回 0，旧代码会让它掉进 adaptive 分支，
  // 于是给一个可能是 4.5 的模型发 {"type":"adaptive"} + output_config.effort —— 硬 400，
  // 而界面上「保存成功、档位可调」，用户看到的只是整轮失败。
  const f = extractFn("_builtinThinkingProfileFor");
  assert.match(f, /_fromCustom && _claudeGeneration\(s\) === 0/,
    "别名保护没了：自定义条目上填 sonnet-latest 会发出必然 400 的思考参数");
});

test("列表行要印出协议——选错了不点开编辑根本看不出来", () => {
  // 协议选错的表现是一句 404，看着像地址填错。列表上直接标出来，一眼能对。
  // 落点是 renderList 这个嵌套箭头函数，按名字切不出来；SRC 本身就是剥了注释的那份
  // （文件顶部 `import { CODE as SRC }`），所以直接在它上面断言不会被注释喂绿。
  assert.match(SRC, /CM_PROTOCOL_UI\[it\.protocol\]\.label/,
    "列表行不印协议：用户在一堆条目里分不出哪条是 Anthropic，选错只能靠 404 反推");
});

test("三条认知腿在非 OpenAI 协议上必须走协议分叉，而不是打一个不存在的端点", async () => {
  // 工具编排 / 收尾评审 / 离线蒸馏此前各自拼 _chatCompletionsUrl + Bearer，绕过 Rust 的
  // 协议分叉。用户选了 Anthropic 原生之后，这三样 100% 打到
  // https://api.anthropic.com/v1/chat/completions → 404，而三处都是 catch{}，界面零提示。
  //
  // 这条是**行为**测试：把真函数取出来跑，看它到底调了谁。源码文本断言在这个仓库栽过
  // 一次（读取侧那条），不再用。
  const calls = [];
  const _cognitiveLegComplete = load("_cognitiveLegComplete", {
    cmProtocol,
    _fetchCompletionText: (url) => { calls.push(["openai直发", url]); return Promise.resolve("{}"); },
    _chatCompletionsUrl: (b) => String(b).replace(/\/+$/, "") + "/chat/completions",
    _billableAiComplete: (cfg) => { calls.push(["走Rust协议分叉", cfg.protocol]); return Promise.resolve("{}"); },
  });

  const body = { model: "m", messages: [{ role: "user", content: "x" }] };

  // openai：逐字走老路（铁律 1 —— 存量条目行为一个字节不变）
  calls.length = 0;
  await _cognitiveLegComplete({ baseUrl: "https://relay.example/v1", apiKey: "k", protocol: "openai" }, body, 100);
  assert.deepEqual(calls, [["openai直发", "https://relay.example/v1/chat/completions"]],
    "openai 那条路被改动了 —— 存量自定义端点的三条腿会跟着变");

  // 没有 protocol 字段的存量条目，同样走老路
  calls.length = 0;
  await _cognitiveLegComplete({ baseUrl: "https://relay.example/v1", apiKey: "k" }, body, 100);
  assert.equal(calls[0][0], "openai直发", "存量条目（无 protocol）不该被改道");

  for (const p of ["anthropic", "xai_responses"]) {
    calls.length = 0;
    await _cognitiveLegComplete({ baseUrl: "https://api.anthropic.com", apiKey: "k", protocol: p }, body, 100);
    assert.equal(calls[0][0], "走Rust协议分叉",
      `${p} 上仍在直发 OpenAI 形状的 /chat/completions —— 这三条腿会 100% 静默失败`);
    assert.equal(calls[0][1], p, "协议没被透传下去，Rust 侧会落回 openai");
  }
});

test("三条腿都必须**经过**那个 helper —— 上一条只测 helper 自己，某条腿改回直发它抓不到", () => {
  // 变异实测：把 _semanticToolOrchestrator 改回 _fetchCompletionText(_chatCompletionsUrl(…))，
  // 上一条行为测试**照样绿**。所以需要这一条守「调用点还在不在」。
  //
  // 判据是**否定式**的：不许再出现 _chatCompletionsUrl —— 那是绕过协议分叉的唯一形状。
  // 用 { code: true } 从剥了注释的源码里取，否则 helper 的说明文字（里面就写着
  // _chatCompletionsUrl）会把断言喂反。
  for (const fn of ["_semanticToolOrchestrator", "_wrapUpCritic", "_offlineDistillIfDue"]) {
    const src = extractFn(fn, { code: true });
    assert.match(src, /_cognitiveLegComplete\(/,
      `${fn} 不再走 _cognitiveLegComplete —— 非 OpenAI 协议上它会 100% 静默失败`);
    assert.doesNotMatch(src, /_chatCompletionsUrl\(/,
      `${fn} 又自己拼 /chat/completions 了 —— Anthropic 端点上这是必然 404，而外层是 catch{}`);
  }
});

test("自己端点的 401 不许把用户从本产品登出——这条会连锁掐掉网关、智能体和工具", async () => {
  // 用户报的真实症状：「用自定义模型 → 自动闪退登录 → 接着就是用不了 → 智能体、工具
  // 那些也用不了」。根因是 _recoverFromAiFailure 无条件把 401 当成**本产品**的登录过期：
  // 清 michael_token、弹登录框。而 401 来自他自己的中转站（key 写错/余额空/被限流），
  // 和他在 Mr. Day One 的账号毫无关系。token 一清，网关模型、智能体、工具全线失效 ——
  // 用户看到的是「选了自定义模型之后整个软件都用不了了」，找不到任何因果。
  const removed = [];
  const toasts = [];
  let loginOpened = 0, billingOpened = 0;
  const mk = () => load("_recoverFromAiFailure", {
    localStorage: { removeItem: (k) => removed.push(k) },
    _loggedInEmail: null,
    _setMichaelUserProfile: () => {},
    _updateLoginUI: () => {},
    openLoginDialog: () => { loginOpened++; },
    _showBillingPanel: () => { billingOpened++; },
    showToast: (t) => toasts.push(String(t)),
  });

  // 自定义端点：一步都不许动账号
  const r1 = mk()("auth", { custom: true });
  assert.equal(r1, true, "自定义端点上也要有反馈，不能一声不吭");
  assert.deepEqual(removed, [], "清掉了 michael_token —— 这一步就是「闪退登录」，而且会连锁");
  assert.equal(loginOpened, 0, "弹了登录框 —— 用户的登录根本没问题");
  assert.ok(toasts.some((t) => t.includes("401")), "没告诉用户 401 是他自己端点给的");

  const r2 = mk()("payment", { custom: true });
  assert.equal(r2, true);
  assert.equal(billingOpened, 0, "打开了本产品的充值页 —— 空的是那个中转站的余额");

  // 网关线路：老行为一个字节不变（清 token + 弹登录，这是对的）
  removed.length = 0; toasts.length = 0; loginOpened = 0; billingOpened = 0;
  assert.equal(mk()("auth"), true);
  assert.deepEqual(removed, ["michael_token"], "网关 401 不清 token 的话，下一轮还拿同一个过期 token 再撞一次");
  assert.equal(loginOpened, 1, "网关 401 必须把登录框摆到人面前");
  assert.equal(mk()("payment"), true);
  assert.equal(billingOpened, 1, "网关 402 必须打开充值页");
});

test("调用点要把「本轮是不是自己的端点」传下去——不传的话上面那条门等于没有", () => {
  // 行为测试测的是 _recoverFromAiFailure 本身；调用点漏传 custom 它抓不到（变异实测）。
  const loop = extractFn("_runAgenticLoop", { code: true });
  assert.match(loop, /_recoverFromAiFailure\([\s\S]{0,200}?customModelId/,
    "调用点没传 custom —— 自己端点的 401 照样会把用户登出，这条门形同虚设");
});
