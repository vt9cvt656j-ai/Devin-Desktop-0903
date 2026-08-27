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
  assert.match(SRC, /function _warnCustomEndpointOnce\(/, "没有任何知情提示");
  const warn = extractFn("_warnCustomEndpointOnce");
  assert.match(warn, /工具描述/, "提示里没说工具描述这件事");
  assert.match(warn, /系统提示词/, "提示里没说系统提示词这件事");
  assert.match(warn, /localStorage/, "提示没有去重，会变成每轮骚扰");
  assert.match(READY, /_warnCustomEndpointOnce\(_custom\)/, "提示函数写了却没被调用");
});

// ---- 线协议：六条真实调用链上的门 -----------------------------------------
// 全部用 fnSource 按 AST 取函数体，不用固定行窗口 —— 函数一变长，固定窗口就悄悄不再
// 守住尾部，而且仍然是绿的。

test("读取侧必须归一化，否则「选了协议也不生效」且全程零报错", () => {
  // _loadCustomModels 里那个 .map 是**定形**的：不在里面列出来的字段每次读取都被静默
  // 丢掉。只在保存侧写 protocol 而不改这里，表现是「弹窗选了 Anthropic、提示保存成功、
  // 列表也刷新了，发出去的还是 /chat/completions」。这是本次改动的头号静默失效点。
  // { code: true } = 从剥掉注释的那份里切。上面那段注释里就写着 normalizeCustomModel，
  // 不剥的话这条断言是被自己的注释喂绿的 —— 把 .map(normalizeCustomModel) 整行删掉它
  // 照样过。这个仓库吃过这个亏，helpers/source.mjs 的这个开关就是为它准备的。
  const load = extractFn("_loadCustomModels", { code: true });
  assert.match(load, /\.map\(normalizeCustomModel\)/,
    "读取侧没有归一化——保存侧写进去的 protocol 每次读取都会被那个定形 .map 丢掉");
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
