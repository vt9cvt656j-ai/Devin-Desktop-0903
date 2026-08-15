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
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

function extractFn(name) {
  const i = SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `main.js 里找不到 ${name}`);
  let depth = 0;
  let j = SRC.indexOf("{", SRC.indexOf(")", i));
  for (; j < SRC.length; j++) {
    const c = SRC[j];
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  return SRC.slice(i, j + 1);
}

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
  const runtime = extractFn("_aiConfigForRuntime");
  assert.doesNotMatch(runtime, /customBaseUrl|customApiKey|customModel/,
    "运行时配置被放开到自定义端点了——内部辅助调用会开始烧用户的钱");
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
