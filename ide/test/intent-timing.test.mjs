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
import { spawnSync } from "node:child_process";
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
  // 下限不是「刚好覆盖实测」——8000 就是刚好覆盖，余量几百毫秒，实测结果是线上两种提示词
  // 字节数交替出现（26951 有各层 / 18885 只有 base），也就是五五开。抬高这个上限没有代价：
  // 它是 race 的超时臂，裁决一落定就放行，只在本来就会失败的轮次多等。
  assert.ok(windowMs >= 12000,
    `前台窗口 ${windowMs}ms 对实测裁决延迟（6931ms / 7599ms）余量太薄——这道等待会变成`
    + `五五开的赌博，赶不上的那些轮次模型手里没有工程/调研纪律`);

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

  // 先确认这个数字**就是模型真正拿到的那份**。
  //
  // 网关目录和 main.js 的注册表是两套东西，运行时以网关那份为准（release 构建还会把
  // 客户端描述整个剥掉）。只改一边是常态——就在今天，run_subagent 的并发文案在网关那份
  // 里错了至少两个提交周期，而所有测试全绿。那种状态下这条断言量的是一个和模型无关的
  // 数字：上限看着够，实际发出去的目录是另一副样子。
  const sync = spawnSync(process.execPath, [join(HERE, "..", "build", "sync-tools-json.mjs"), "--check"], {
    encoding: "utf8",
  });
  assert.equal(
    sync.status, 0,
    `两份工具目录不同步，下面的数量断言量的就不是模型真正收到的那份：\n${sync.stdout}\n${sync.stderr}`,
  );

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
    + "路径会从「数量」换到「字节」，症状一模一样。\n"
    + "撞上这条时先问一句：是目录真的该这么大，还是某个工具的描述在膨胀？"
    + "前者抬上限并把这里的实测值写进注释，后者去收那个描述——别默认抬数字。",
  );
});

test("面向模型的判断一律跟随用户选的模型——廉价降级这条路已经整个删掉", () => {
  // 用户的决定：输入框的回复建议用他当前选的那个模型，不要降级。降级版此前的失败方式
  // 很难看——挑中的永远是目录里最便宜那条线路，它长期坏掉（实测 gpt-5.4-mini 24 小时
  // 6 次调用 6 次 502）建议就永久不出现、界面没有任何解释、每轮还白烧一次上游请求。
  assert.doesNotMatch(CODE, /_pickCheapModel/,
    "廉价模型降级又长回来了。全项目的约定是：任何面向模型的判断都跟随用户选择的模型");

  // 锚点用这个函数独有的局部变量——`_ASK_PREDICT_SYSTEM` 的第一次出现是它的**定义处**，
  // 从那里切窗口根本到不了发请求那段（写这个文件时踩了两次同样的坑）。
  const i = CODE.indexOf("_predictCfg = {");
  assert.ok(i > 0, "找不到回复建议那次请求的配置组装");
  const call = CODE.slice(Math.max(0, i - 600), i + 2600);
  assert.match(call, /model: _predictCfg\.model/,
    "建议请求没有用用户当前选的模型");

  // 自定义端点也照这个规矩：用户选了自己的模型，建议就该出自那个模型。三个字段的覆写
  // 与主发送路径同款。
  assert.match(call, /_customModelById\(cfg\.model\)/, "自定义模型没有被解析出来");
  assert.match(call, /baseUrl: _cm\.baseUrl[\s\S]{0,60}apiKey: _cm\.apiKey[\s\S]{0,60}model: _cm\.name/,
    "自定义端点必须整组覆写地址、密钥和真实模型名——只换模型名会拿着网关的名字去问别人的端点");

  // 只用自己端点的用户压根不需要登录，此前那道 token 闸把他们的建议整个挡掉了。
  assert.match(call, /if \(_predictCfg\.viaGateway\) \{[\s\S]{0,200}michael_token/,
    "michael_token 只该是网关那条路的前置条件");
  // 计费关联头是我们自己的东西，别发给第三方端点。
  assert.match(call, /_predictCfg\.viaGateway && \/\^\[-_A-Za-z0-9\]\{8,128\}\$\/\.test\(rid\)/,
    "x-ide-request-id 必须只对网关发");
});

test("一轮都没跑的时候，不许在最高注意力位摆一条「接着完成尚未完成的动作」", () => {
  // 线上实拍：用户问「deepseek 最近有什么进展，和 Claude Code / Codex 比」——一个纯外部
  // 知识问题。模型的思考逐条复述了 express + better-sqlite3、data.db、node src/server.js，
  // 然后得出「用户没有提出具体问题，只是打开了这个项目」，转头 list_dir + 读源码。
  //
  // 用户的话确实发出去了（网关计费 prompt=2 / cache_create=4735，那 4735 就是真实内容）。
  // 盖住它的是 harness 每轮追加在**消息尾部**的那条合成消息：署名「执行状态·不要从头重查」、
  // 末尾「接着完成尚未完成的动作」。它的触发条件里有 _runtimeStateBlock，而依赖/启动脚本/
  // 数据库文件对任何真实项目都非空 —— 于是它每轮必发，包括一轮都还没跑的第一轮。
  const i = CODE.indexOf("_hasRunActivity");
  assert.ok(i > 0,
    "执行状态块又变回「有没有活动」和「项目长什么样」共用一个条件了 —— 那会让它第一轮就发");
  const block = CODE.slice(Math.max(0, i - 200), i + 2600);

  // 触发条件必须把「真实动作」和「静态环境事实」分开算。
  assert.match(
    block,
    /const _hasRunActivity = !!\(_mutatedFiles\.size \|\| _readFiles\.size \|\| _evidenceBlock \|\| _latestDiagBlock\)/,
    "「本次运行做过事」的判据里不能含 _runtimeStateBlock：它是静态事实，第一轮就非空",
  );

  // 「接着完成尚未完成的动作」只能出现在真的有动作的那一支。
  const tailIdx = block.indexOf("接着完成尚未完成的动作");
  assert.ok(tailIdx > 0, "找不到那句收尾指令，锚点已失效");
  const guard = block.slice(Math.max(0, tailIdx - 220), tailIdx);
  assert.match(guard, /_hasRunActivity/,
    "「接着完成尚未完成的动作」没有被 _hasRunActivity 守住 —— 一轮没跑就凭空断言有活干到一半");

  // 没动作那一支必须反过来把注意力推回用户的话上。
  assert.match(block, /用户这一轮的请求就在上面的对话里/,
    "facts-only 那支要明确把模型指回用户真正说的话，否则它照着环境事实去摸项目");
  assert.match(block, /本次运行还没有任何动作/,
    "facts-only 那支的标题仍在自称「执行状态」，模型会当成有进度可续");
});

test("完整裁决要 19.8 秒，路由必须有第二条腿——而且那条腿只喂请求头", () => {
  // 生产网关实测（2026-08-17）：
  //   17:37:51 分类器发出 → 17:38:06 主回合发出（等待 15s 超时）→ 17:38:10 分类器才回
  //   upstream_header_ms=19836
  // 只有完整裁决这一条腿时，这道等待就是二选一：干等二十几秒，或者这一轮工程/调研/设计/
  // Git/协作/自动化/缺陷八个模块一个都不挂。两个都不能接受，所以拆快通道。
  assert.match(CODE, /async function _fastRoutingFlags\(/,
    "路由快通道没了——那这道等待又回到了「干等」和「没有模块」的二选一");

  const fn = CODE.slice(CODE.indexOf("async function _fastRoutingFlags("), CODE.indexOf("async function _fastRoutingFlags(") + 3200);

  // 快的全部原因就是输出短。max_tokens 一放开，它就跟完整裁决一样慢，这条腿白加。
  assert.match(fn, /_billableAiComplete\(cfg, \[\{ role: "user", content: prompt \}\], 200\)/,
    "快通道的 max_tokens 被放开了——输出一长它就不快了，加这条腿的意义就没了");

  // 仍然用用户选的模型（全项目唯一约定），且不继承深度思考预算。
  assert.doesNotMatch(fn, /_pickCheapModel|gpt-|claude-3|mini/,
    "快通道不许换模型：面向模型的判断一律跟随用户选择的模型");
  assert.match(fn, /for \(const key of \["reasoningEffort", "thinkingBudget", "thinking", "thinkingConfig", "thinkingEffort"\]\) delete cfg\[key\]/,
    "快通道必须剥掉深度思考预算，否则它会和完整裁决一样慢");

  // 宁缺毋滥：判不准就省略，靠单调并集让完整裁决补齐。
  assert.match(fn, /raw\[k\] === true/,
    "只接受显式为 true 的旗标——把缺省当真会让整轮带上不相干的纪律");
  assert.match(fn, /return meaningful \? profile : null/,
    "一个旗标都没点亮时要返回 null，别把空画像当成「判过了」");

  // 两条腿必须真的并行 race，否则快通道等于没接。
  const wait = waitBlock();
  assert.match(wait, /_fastRoutingFlags\(text, config, sess/, "快通道没有在等待块里启动");
  assert.match(wait, /Promise\.race\(\[\s*_turnIntentExactPromise,\s*_fastRoute,/,
    "两条腿必须在同一个 race 里——串行等待就没有意义了");
});
