// 「模型什么都没产出」和「产到一半被预算截断」在能力账本上都记 fail，而修法完全相反：
// 前者要切小输入面（加预算救不了），后者补预算就够。
//
// 分清这两种的唯一信号是 finish_reason。带工具的那条流早就在解析它了，只有 ai_complete
// 这条一直把它扔掉——`_billableAiComplete` 里那句注释逐字写着「抓不住『非空但截断』
// 那种形状——那需要 finish_reason，而 ai_complete 只回正文字符串」。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { CODE as SRC, load, fnSource as topLevelFn } from "./helpers/source.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const AI_RS = readFileSync(join(HERE, "../src-tauri/src/ai.rs"), "utf8");

// ── Rust 侧：流里取到 finish_reason 并带回 ─────────────────────────────
test("SSE 解析顺带取 finish_reason", () => {
  assert.match(AI_RS, /finish_reason"\]\.as_str\(\)/,
    "这条流没解析 finish_reason——它本来就在逐行解析 SSE，取它零成本");
  assert.match(AI_RS, /normalize_finish_reason\(fr\)/,
    "没走既有的归一函数：各家厂商的措辞不同（max_tokens / end_turn / stop_sequence）");
});

test("ai_complete 把它带回给客户端", () => {
  assert.match(AI_RS, /"text": content, "finishReason": finish/,
    "取到了却没带回去——客户端仍然分不清产不出和截断");
  assert.match(AI_RS, /pub async fn ai_complete\([\s\S]{0,200}-> Result<serde_json::Value, String>/,
    "返回类型还是裸字符串");
});

test("兜底那条路（中转无视 stream:true）也要给出形状一致的返回", () => {
  // 有的中转直接回普通 JSON body。那条路返回值形状不一致的话，客户端会拿到 undefined。
  const hits = AI_RS.match(/return Ok\(\(t\.to_string\(\), String::new\(\)\)\);/g) || [];
  assert.equal(hits.length, 2, `兜底返回点有 ${hits.length} 处形状跟上了，应为 2`);
});

// ── 客户端：两种形状都认，且截断走补预算那条路 ─────────────────────
const billable = topLevelFn("_billableAiComplete", { code: true });

test("新旧两种返回形状都认", () => {
  // 前端和 Rust 一起发版，但测试夹具和网页构建仍会喂裸字符串。
  assert.match(billable, /typeof o === "object" && !Array\.isArray\(o\) \? String\(o\.text/,
    "只认对象形状——喂裸串的地方会拿到 undefined");
  assert.match(billable, /\.then\(_text\)/,
    "调用方拿到的不再是正文字符串了——那会把每个调用点都改坏");
});

test("截断（finish_reason=length）走补预算重试", () => {
  assert.match(billable, /_finish\(out\) === "length"/,
    "截断没被识别——它会被当成「产不出」，而那两种的修法相反");
  const at = billable.indexOf('_finish(out) === "length"');
  const seg = billable.slice(at, at + 300);
  assert.match(seg, /_markModelNeedsAuxHeadroom\(model\)/, "识别了却没记进「这个模型要余量」");
  assert.match(seg, /send\(headroomCap\)/, "识别了却没真的补预算重试");
});

test("补预算只补一次，不许无上限往上加", () => {
  const at = billable.indexOf('_finish(out) === "length"');
  const seg = billable.slice(at, at + 200);
  assert.match(seg, /firstCap === maxTokens/,
    "已经带过余量的还要再补——那会无上限地涨");
});

test("空正文那条老路仍在（声明没覆盖到的模型靠事实学一次）", () => {
  assert.match(billable, /if \(_text\(out\)\.trim\(\) \|\| firstCap !== maxTokens \|\| !cap\) return out;/,
    "空正文兜底被这次改动弄丢了");
});

test("那句「抓不住截断」的旧注释必须删掉", () => {
  // 承诺一个不存在的能力比没有更糟；反过来，留着一句「这件事做不到」而其实已经做到了，
  // 会让下一个人绕开这条链去重造一遍。
  assert.doesNotMatch(billable, /抓不住「非空但截断」/,
    "注释还说抓不住——它现在抓得住了，留着会误导下一个人");
});

// ── 红线 ──────────────────────────────────────────────────────────────
test("finish_reason 只当判据，不进任何权限门", () => {
  const at = SRC.indexOf('_finish(out) === "length"');
  const seg = SRC.slice(Math.max(0, at - 400), at + 600);
  assert.doesNotMatch(seg, /approve|_approveToolCall|permission/i,
    "上游的返回状态被拿去当权限判据了——那是能被上游影响的一道门");
});
