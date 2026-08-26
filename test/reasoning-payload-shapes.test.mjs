// ── 思考回执：三跳都可能把它悄悄弄丢 ──────────────────────────────────────
//
// 起因：用户用 grok-4.6 跑 agent，工具调用一切正常、**一张思考卡都没有**。
// 沿链取证的结论是「客户端把参数发对了」（网关入站遥测实测 reasoning_effort="xhigh"），
// 断点全在**回来的方向**，而且是三处各自独立、都不报错的静默丢弃：
//
//   D1 解析器只认字符串。`reasoning` 是对象、或走 OpenRouter 现行的 reasoning_details
//      数组时，as_str() 返回 None → 事件不发、不报错、不打日志。
//   D2 首个可见输出之后的思考被整段丢弃，而**工具调用**会置真那个标志。现代模型是
//      「想→调工具→再想」交错的，于是第一次工具调用之后的思考全没了。
//      更糟的是 return 发生在写 reasoningAcc/reasoningAll 之前：不上屏、不进历史、
//      不进崩溃草稿，三处同时没有。
//   D3 传输层一路传上来的 reasoning_tokens / thinking_chars，在客户端第一站被丢掉。
//      这条最要命的地方在于它是**尺子**：没有它，「模型没思考」和「思考了但我们丢了」
//      在界面上完全同形——查前两条时第一件想做的事就是量这个数，而它是个死数。
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { CODE as SRC, fnSource as extractFn } from "./helpers/source.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const AI_RS = readFileSync(join(HERE, "../src-tauri/src/ai.rs"), "utf8");
const I18N = readFileSync(join(HERE, "../src/i18n.js"), "utf8");

// ── D1：解析器认全已知的载荷形状 ─────────────────────────────────────────
test("思考解析器认全载荷形状，不只认字符串", () => {
  assert.match(AI_RS, /fn reasoning_text_from_delta\(delta: &serde_json::Value\) -> Option<String>/,
    "按形状取思考文本的那个函数没了 —— 又退回只认字符串");
  const fn = AI_RS.slice(AI_RS.indexOf("fn reasoning_text_from_delta"));
  const body = fn.slice(0, fn.indexOf("\n}\n") + 3);
  assert.ok(body.length < 2000, `切出了 ${body.length} 字符 —— 边界划到函数外面去了`);
  for (const key of ["reasoning_content", "reasoning", "reasoning_details"]) {
    assert.ok(body.includes(`"${key}"`), `${key} 不在认的字段名里`);
  }
  // reasoning_details 是 OpenRouter 现行的规范载荷，而本仓库的模型目录正是抓 OpenRouter 的。
  assert.match(AI_RS, /reasoning_details/, "OpenRouter 的规范推理载荷形状没认");
  assert.match(body, /Value::Array\(items\)/, "数组（分片推理）形状没认");
  assert.match(body, /"text", "content", "reasoning", "summary"/, "对象形状取不到文本");
  // 调用点必须换成新函数，别留一份只认字符串的旧路。**先剥注释**——本文件的注释里
  // 就原样引用着那个旧写法，不剥的话这条断言会匹配到自己的说明文字。
  const RS_CODE = AI_RS.replace(/\/\/[^\n]*/g, "").replace(/\/\*[\s\S]*?\*\//g, "");
  assert.doesNotMatch(RS_CODE, /delta\["reasoning_content"\]\s*\n?\s*\.as_str\(\)\s*\n?\s*\.or_else\(\|\| delta\["reasoning"\]\.as_str\(\)\)/,
    "旧的「只认两个字符串字段」写法还在（两处：渲染路和停滞看门狗的进展谓词）");
  // 停滞看门狗的进展谓词也必须用同一份判据：形状不认 → 判成「没有有效内容」→
  // 看门狗把一条正在正常思考的流掐掉，报「模型在 N 秒内没有生成有效内容」。
  assert.match(RS_CODE, /fn delta_has_real_progress[\s\S]{0,400}reasoning_text_from_delta\(delta\)/,
    "进展谓词没走同一份思考判据 —— 对象/数组形状的思考会被看门狗当成停滞");
  assert.match(AI_RS, /if let Some\(rt\) = reasoning_text_from_delta\(delta\)/,
    "解析主路没有走新函数");
});

// ── D2：厂商通道不受内联标签那道闸约束 ───────────────────────────────────
test("厂商的 reasoning 通道不被工具调用掐断", () => {
  const turn = extractFn("_agentModelTurn");
  // 判据的两半：调用点传 trusted，且函数确实按 trusted 放行。
  assert.match(turn, /accepted = appendReasoning\(ev\.delta \|\| "", true\);/,
    "厂商 reasoning 事件又被套上内联标签的 answerStarted 闸了 —— "
    + "工具调用会置真它，「想→调工具→再想」的模型从第一次调用起思考全丢");
  assert.match(turn, /const appendReasoning = \(delta, trusted = false\) => \{\s*\n\s*if \(\(!trusted && !_canRenderPreAnswerReasoning\(_inlineThinkState\)\)/,
    "appendReasoning 的免闸判据变了");
  // 工具调用确实会置真那个标志——这是上面那条为什么必要的前提，一起钉住。
  assert.match(turn, /else if \(ev\.kind === "toolCall"\)[\s\S]{0,400}_inlineThinkState\.answerStarted = true;/,
    "工具调用不再置真 answerStarted 了？那上面那条免闸的理由要重写");
  // 内联标签那条路仍然受闸（它是启发式，误判有代价）。
  assert.match(SRC, /function _canRenderPreAnswerReasoning\(state\) \{\s*\n\s*return !state \|\| state\.answerStarted !== true;/,
    "内联标签的闸没了 —— 正文里打个尖括号就会被当成思考");
});

// ── D3：思考回执要被接住，它是判「有没有思考」的唯一尺子 ─────────────────
test("流式 usage 里的思考回执要接住，别在最后一寸落地", () => {
  const fn = extractFn("_recordStreamUsage");
  assert.match(fn, /_lastReasoningTok = _rt;/, "reasoning_tokens 没被接住");
  assert.match(fn, /_lastThinkChars = _tc;/, "thinking_chars 没被接住");
  // 三家的字段名都要认（Rust 侧已经认全了，这里是最后一跳）。
  for (const shape of ["reasoningTokens", "reasoning_tokens", "completion_tokens_details", "output_tokens_details"]) {
    assert.ok(fn.includes(shape), `${shape} 这种形状没认`);
  }
  // 另一条腿是死的：_recordUsage 拿的是网关结算对象，那里面根本没有这两个字段。
  // 所以这一处是唯一活口，钉住它别被"看起来重复"删掉。
  assert.match(SRC, /_recordUsage\(\{\s*\n\s*prompt_tokens: settlement\.promptTokens/,
    "_recordUsage 的入参形状变了 —— 若它开始带思考字段，这里的「唯一活口」说法要重写");
  // Rust 侧确实在发这两个字段（对不上就等于白接）。
  assert.match(AI_RS, /reasoning_tokens: reasoning as u32,/, "Rust 侧不发 reasoning_tokens 了");
  assert.match(AI_RS, /thinking_chars: thinking_chars as u32,/, "Rust 侧不发 thinking_chars 了");
  // 字段在线上是蛇形（enum 上只有 rename_all，没有 rename_all_fields）——接的时候两种都认。
  assert.doesNotMatch(AI_RS, /rename_all_fields/,
    "AiEvent 加了 rename_all_fields，线上字段变驼峰 —— 上面认的蛇形要跟着核一遍");
});

// ── grok 的档位提示必须说清「这条接口不回思考正文」 ──────────────────────
test("grok 的思考档位提示要说清这条接口不返回思考正文", () => {
  // 用户报「grok 不发送 thinking」时的实际形状：转盘写着「思考 High」，一张卡都不出，
  // 和坏了没有任何区别。而沿链取证的结论是**我们这边没丢**——
  //   · 网关入站遥测实测 reasoning_effort="xhigh"（参数确实发出去了）
  //   · xAI 官方对比页原文：Chat Completions API (Deprecated) 那一列写着
  //     「No reasoning content returned」；AWS Bedrock 的 Grok 4.6 model card 同样写法
  //   · 可读的思考摘要只在 xAI 的 Responses API 上给（response.reasoning_text.delta）
  //
  // 所以能做的不是"修好它"，是**别让界面继续假装**：档位是真生效的（它决定模型想多久、
  // 也决定用户付多少钱），但思考正文在这条线上结构性拿不到。这条守的就是那句话别被
  // 后人当废话删掉——删掉之后，下一个用户还会以为是 bug，下一个人还会再查一遍这条链。
  for (const key of ["grokReasoning", "grok45", "grok43"]) {
    const lines = I18N.split("\n").filter((l) => l.includes(`"model.thinking.reason.${key}"`));
    assert.equal(lines.length, 2, `${key} 应当在中英两份文案里各有一条`);
    for (const line of lines) {
      assert.match(line, /Chat Completions/,
        `${key} 的提示没点名这条接口 —— 用户会继续把「没有思考卡」当成 bug`);
      assert.match(line, /no thinking card will appear|不会出思考卡/,
        `${key} 的提示没说清不会出思考卡`);
    }
  }
  // 反面：别把「档位没生效」写进去——它是生效的，参数实测发出去了。
  const zh = I18N.split("\n").find((l) => l.includes('"model.thinking.reason.grokReasoning"') && /档位/.test(l));
  assert.match(zh, /档位是真生效的/, "别把「档位没生效」当成结论 —— 网关入站遥测实测它发出去了");
});
