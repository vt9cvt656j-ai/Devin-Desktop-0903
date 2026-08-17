// 「突然变弱智、138 个工具也不用了」的成因，全在装配请求**之前**的那几毫秒里。
//
// 生产网关日志实测的一次用户回合：
//   14:39:05.654  发出意图裁决请求（claude-opus-5，思考键已剥离）
//   14:39:07.249  主回合发出（+1.6s），requested_tool_count=10
//                 prompt_blocks=["agent_core","reasoning","truthfulness","answer_quality"]
//   14:39:12.593  裁决的响应头才回来（upstream_header_ms=6931）
//
// 也就是说决定整个做法的那一轮，比裁决早发了 5.3 秒，带着**空画像**出门。网关按 flag 挂块，
// 空画像只命中 agent.base 四块——agent_engineering / git_guide / agent_research /
// agent_collaboration / agent_automation / defect_hunting 和整套 michael-design 设计层
// 一块都没有。代码里那道「第一轮等一下」的逻辑是对的，只是上限写了 1500ms，比实测短 5 倍，
// 于是 race 每次都由 timer 赢——一道恒定失败的等待，全绿、无日志、每轮都发生。
//
// 这个文件守住四条性质，都是"两侧都编译、都全绿，只是从此永远不触发"那一类：
//   1. 前台窗口和第一轮等待上限之间的大小关系（写反就是恒定失败的 race）；
//   2. 「只等一次」的判据不能只看 flag 是否为空（零 flag 的裁决是合法的）；
//   3. 「服务端挂了设计层吗」必须按旗标判，不能按画像字符串的 truthy 判；
//   4. 两侧的工具数量上限都必须容得下静态目录的真实规模。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
const RUST = readFileSync(join(HERE, "..", "..", "server", "src", "prompts.rs"), "utf8");
const CATALOG = JSON.parse(readFileSync(join(HERE, "..", "..", "server", "prompts", "tools.json"), "utf8"));

// 注释里出现的词不算数。这个文件尤其需要：上面那段解释逐字引用了 1500、128、64 和
// `!!config.ideSemanticProfile` —— 全是被修掉的旧代码。不剥注释的话，把修改整个回退掉
// 这些断言依然全绿。（这个坑在本仓库踩过六次，所以先剥再断。）
//
// 按**行**剥，不用 /\/\*[\s\S]*?\*\//：后者在 main.js 上会一口吃掉几十万字符的真代码，
// 因为 `/*` 也出现在字符串和正则字面量里。
function stripComments(src) {
  const out = [];
  let inBlock = false;
  for (const line of src.split("\n")) {
    const t = line.trim();
    if (inBlock) {
      if (t.includes("*/")) inBlock = false;
      continue;
    }
    if (t.startsWith("//")) continue;
    if (t.startsWith("/*")) {
      if (!t.includes("*/")) inBlock = true;
      continue;
    }
    out.push(line);
  }
  return out.join("\n");
}
const CODE = stripComments(SRC);
const RUST_CODE = stripComments(RUST);

// 取一个不含花括号字面量的短函数的源文本。本文件只用它取 _serverDesignLayersRouted，
// 那个函数里没有 `{`/`}` 出现在字符串或正则里，简单配对就够；复杂函数请用 logic.test.mjs
// 里那套带字符串/模板/正则跳过的 extractFn。
function extractSimpleFn(name) {
  const m = new RegExp(`function\\s+${name}\\s*\\(`).exec(SRC);
  assert.ok(m, `main.js 里找不到 ${name}`);
  let i = SRC.indexOf("{", SRC.indexOf(")", m.index));
  let depth = 0;
  for (; i < SRC.length; i++) {
    if (SRC[i] === "{") depth++;
    else if (SRC[i] === "}") { depth--; if (depth === 0) return SRC.slice(m.index, i + 1); }
  }
  throw new Error(`${name} 的函数体没有闭合`);
}

function numericConst(code, name) {
  const m = new RegExp(`const\\s+${name}\\s*(?::\\s*usize\\s*)?=\\s*([0-9_]+)\\s*(\\*\\s*[0-9_]+\\s*)?;`).exec(code);
  assert.ok(m, `找不到常量 ${name}`);
  const base = Number(m[1].replace(/_/g, ""));
  if (!m[2]) return base;
  return base * Number(m[2].replace(/[*\s]/g, "").replace(/_/g, ""));
}

// 「第一轮等意图裁决」那一整段的源文本。按那道 if 的判据取——它在 main.js 里唯一。
const WAIT_ANCHOR = "_semanticProfileFlags || []).length";
function waitBlock() {
  const i = CODE.indexOf(WAIT_ANCHOR);
  assert.ok(i > 0, `找不到第一轮等待那道 if（锚点 ${WAIT_ANCHOR}）——它是本文件全部断言的落点`);
  const start = CODE.lastIndexOf("if (", i);
  return CODE.slice(start, i + 1100);
}

test("第一轮等意图裁决的上限不许短于前台窗口——短一点这道等待就恒定失败", () => {
  const windowMs = numericConst(CODE, "_INTENT_FOREGROUND_WAIT_MS");
  assert.ok(windowMs >= 8000, `前台窗口 ${windowMs}ms 低于实测裁决延迟（6931ms / 7607ms），裁决永远赶不上`);

  // 关键是这两个值**同源**。分成两个独立字面量，就是上一次留下 1500 的方式：
  // 一边调了另一边没跟上，race 从此恒定由 timer 赢，而且没有任何东西会报错。
  const m = /const\s+_FIRST_TURN_INTENT_WAIT_MS\s*=\s*([^;]+);/.exec(CODE);
  assert.ok(m, "找不到 _FIRST_TURN_INTENT_WAIT_MS");
  const rhs = m[1].trim();
  assert.ok(
    rhs.includes("_INTENT_FOREGROUND_WAIT_MS"),
    `第一轮等待上限必须从前台窗口推导，现在是裸字面量 ${rhs}——两个数字会各自漂移，`
    + "而漂到「等待 < 窗口」时这道等待就完全失效且无声无息",
  );

  // 真的用上了：那个 race 必须拿这个常量当 timer，不是另一个数。
  // 锚点用那道 if 本身——`_turnIntentExactPromise` 的第一次出现是它的声明处，从那里切
  // 600 字符根本到不了 race，断言会永远为假（写这个文件时就先踩了一次）。
  assert.ok(
    waitBlock().includes("_FIRST_TURN_INTENT_WAIT_MS"),
    "第一轮那道 Promise.race 没有用 _FIRST_TURN_INTENT_WAIT_MS 当超时",
  );
});

test("零旗标的裁决是合法的，不能让此后每一轮都重付一次等待", () => {
  const block = waitBlock();
  const gate = block.slice(0, block.indexOf("{") + 1);

  assert.ok(
    gate.includes("_intentWaitPaid"),
    "这道等待只按 _semanticProfileFlags 是否为空来判。普通问答的裁决合法地返回零 flag"
    + "（action=answer、workspaceAction=none），于是 flag 永远是空的，纯聊天会话每一轮都要"
    + "白等一次完整窗口——被这个数字拖成逐轮卡顿。判据要落在「这个会话已经拿到过裁决」上。",
  );

  assert.match(
    block,
    /if\s*\(\s*_turnIntentState\.settled\s*\)\s*sess\._intentWaitPaid\s*=\s*true/,
    "记账必须只在裁决**落定**时发生：超时没落定的轮次下一轮还值得再等一次，"
    + "否则一次网络抖动就让整个会话永久失去画像",
  );
});

test("「服务端挂了设计层吗」必须按 design 旗标判，空画像的 2.5: 是 truthy 的", () => {
  const fn = extractSimpleFn("_serverDesignLayersRouted");
  const make = new Function("_l0On", `${fn}; return _serverDesignLayersRouted;`);
  const routed = make(() => true);

  // 这就是那个 bug 的最小复现：分类器迟到时画像是 "2.5:"，非空字符串。
  // 旧判据 !!config.ideSemanticProfile 为真 → 客户端撤掉自己那份约 4K token 的完整设计
  // 纪律；服务端因为没有 design flag，ui_intent 为假，design.base 及其下所有层一块没挂。
  // 设计纪律于是两边都没发，模型只能凭印象糊 UI。
  assert.equal(routed({ ideSemanticProfile: "2.5:" }), false, "空画像被当成「服务端已注入设计层」");
  assert.equal(routed({ ideSemanticProfile: "" }), false);
  assert.equal(routed({}), false);
  assert.equal(routed({ ideSemanticProfile: "2.5:engineering,git" }), false, "没有 design 旗标却认为设计层已挂");

  assert.equal(routed({ ideSemanticProfile: "2.5:engineering,design" }), true);
  // 服务端是 semantic_profile.contains("design") 的子串语义，design_* 都算命中；
  // 两边判同一件事，别一边子串一边精确。
  assert.equal(routed({ ideSemanticProfile: "2.5:design_motion" }), true);

  // 第三方直连（没有网关注入）时永远兜底发本地全量：零回退，宁重复不缺席。
  assert.equal(make(() => false)({ ideSemanticProfile: "2.5:design" }), false);
});

test("两侧的工具数量上限都必须容得下静态目录的真实规模", () => {
  const catalogCount = CATALOG.length;
  assert.ok(catalogCount > 0, "tools.json 是空的");

  const clientMax = numericConst(CODE, "_TOOL_PAYLOAD_MAX_TOOLS");
  assert.ok(
    clientMax >= catalogCount,
    `客户端工具窗口上限 ${clientMax} < 静态目录 ${catalogCount}。_agentModelTurn 那次调用不传`
    + " requestedSchemas，等于纯按上限裁剪当前窗口——窗口被编排器撑满后尾部工具会被静默"
    + "切掉，没有任何日志，表现为「工具明明装载了却调不到」",
  );

  const staticMax = numericConst(RUST_CODE, "MAX_STATIC_TOOLS_PER_REQUEST");
  const finalMax = numericConst(RUST_CODE, "MAX_FINAL_TOOLS_PER_REQUEST");
  assert.ok(staticMax >= catalogCount, `网关静态工具上限 ${staticMax} < 目录 ${catalogCount}`);
  assert.ok(
    finalMax >= catalogCount,
    `网关最终工具数量上限 ${finalMax} < 目录 ${catalogCount}。enforce_final_tool_budget 按输入`
    + "顺序保留，而运行时/MCP 工具排在前面——被挤掉的正是核心静态工具，且两侧都不报警",
  );

  // 字节上限才是真正兜住 payload 的那道闸，它必须仍然收得住完整目录。
  const byteMax = numericConst(RUST_CODE, "MAX_FINAL_TOOL_SCHEMA_BYTES");
  const catalogBytes = Buffer.byteLength(JSON.stringify(CATALOG));
  assert.ok(
    byteMax >= catalogBytes,
    `字节上限 ${byteMax} 收不住完整目录 ${catalogBytes}——数量上限放开之后，被静默丢工具的`
    + "路径会从「数量」换到「字节」，症状一模一样",
  );
});
