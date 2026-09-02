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
// 按名字取真源码只有一份实现：test/helpers/source.mjs 的 fnSource（acorn 按 AST 边界切）。
// 本文件的 SRC/CODE 还要同样处理 prompts.rs，所以那两个绑定保持本地不动。
import { fnSource, load } from "./helpers/source.mjs";

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


function numericConst(code, name) {
  const m = new RegExp(`const\\s+${name}\\s*(?::\\s*usize\\s*)?=\\s*([0-9_]+)\\s*(\\*\\s*[0-9_]+\\s*)?;`).exec(code);
  assert.ok(m, `找不到常量 ${name}`);
  const base = Number(m[1].replace(/_/g, ""));
  if (!m[2]) return base;
  return base * Number(m[2].replace(/[*\s]/g, "").replace(/_/g, ""));
}

// 「第一轮等意图裁决」那一整段的源文本。按那道 if 的判据取——它在 main.js 里唯一。
// 锚在等待门自己的收尾上：并行的两套实现一个用 _sessionFlags、一个把空判抽成
// _profileStillEmpty 常量——按表达式原文锚会命中 const 定义行，waitBlock 整个切错位置。
const WAIT_ANCHOR = "&& !sess._intentWaitPaid) {";
function waitBlock() {
  const i = CODE.indexOf(WAIT_ANCHOR);
  assert.ok(i > 0, `找不到第一轮等待那道 if（锚点 ${WAIT_ANCHOR}）——它是本文件全部断言的落点`);
  const start = CODE.lastIndexOf("if (", i);
  return CODE.slice(start, i + 1100);
}

test("第一轮等意图裁决的上限不许短于前台窗口——短一点这道等待就恒定失败", () => {
  const windowMs = numericConst(CODE, "_INTENT_FOREGROUND_WAIT_MS");
  // 这个上限**必须有界，而且不能大**。此前写的是 15000，理由是"抬高没有代价：race 的
  // 超时臂，裁决一落定就放行"。那条推理漏了一件事：上游拥堵时裁决**必然**赶不上窗口，
  // 于是这个数字变成每一轮实打实多付的墙钟时间。
  //
  // 2026-08-18 从生产网关日志量到的上游首响应头延迟（同一段时间、同一批请求）：
  //   claude-opus-5 平均 8.3s（最慢 28.0s）／gpt-5.5 平均 10.8s／gpt-5.6-sol 平均 18.4s
  //   而且 gpt-5.6-sol 有 45% 的请求以 502 结束，失败前平均还要耗 23.3s。
  // 裁决走的是用户选的同一个模型，所以它和正文一样慢。窗口 15 秒 + 旧的"只有落定才算
  // 付过"，合起来就是每条消息先干等十几秒——用户实拍："同一个 API 在 Claude Code 和
  // Codex 里飞快，在我软件里巨慢"。
  //
  // 等不到不等于丢掉：裁决在后台继续跑，_applyLateIntentIfLanded 在循环边界补上，
  // 行为闸门照样只认它。所以这里的取舍是"第一轮画像弱一点" vs "每轮多等十几秒"。
  assert.ok(windowMs > 0 && windowMs <= 8000,
    `前台窗口 ${windowMs}ms 不在合理区间：它只在「裁决比它慢」的轮次生效，而那正是要止损的`
    + `场景。定得太大 = 上游一慢，用户的每条消息都先被我们自己拖住十几秒。`);

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

  // 付款按「等过一次」算，不按「等到了」算。
  //
  // 旧写法是 `if (settled) paid = true`，看着更严谨，实际是复利陷阱：上游慢的时候裁决
  // 本来就赶不上窗口（本文件上面记着实测 19.8s，而窗口只有几秒），于是 settled 永远为假
  // → 永远记不上账 → **每一轮都再干等一个完整窗口**。它想防的是"一次网络抖动让整个会话
  // 永久失去画像"，但那件事根本不会发生：_applyLateIntentIfLanded 在循环边界会把迟到的
  // 裁决补上，run.engineering 和粘性画像都会更新。防的是个不存在的风险，代价是每轮实打实
  // 多等十几秒。
  assert.match(
    block,
    /sess\._intentWaitPaid\s*=\s*true/,
    "整个会话最多为这道等待付一次；按「等到了」记账会让上游一慢就每轮重付",
  );
  // 光断言"出现过这一行"是不够的：`if (settled) paid = true` 同样含这一行。
  // 明确禁掉条件式——那正是要防的那个写法。
  assert.doesNotMatch(
    block,
    /if\s*\(\s*_turnIntentState\.settled\s*\)\s*sess\._intentWaitPaid/,
    "记账又被挂回「裁决落定」上了：上游一慢裁决就永远赶不上窗口，于是每一轮都重付一次完整等待",
  );
});

test("「服务端挂了设计层吗」必须按 design 旗标判，空画像的 2.5: 是 truthy 的", () => {
  const fn = fnSource("_serverDesignLayersRouted");
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

test("一轮都没跑的时候，那块执行状态整块都不许发", () => {
  // 两轮实拍才定的案。
  //
  // 第一轮：这块署名〔执行状态·不要从头重查〕、末尾〔接着完成尚未完成的动作〕，一轮没跑也发。
  //   → 模型复述里面的 express/better-sqlite3/data.db，得出「用户没有提出具体问题」，去 list_dir。
  // 第二轮（只改措辞、位置照旧留在最后一条）：
  //   → 模型思考原文：「我看到的只是系统注入的环境信息，没有看到用户的实际请求」
  //                  「最后有环境信息注入」「而最后的系统注入说『用户这一轮的请求就在上面的对话里』」
  //   → 网关字节数：marked_request_bytes=83（问题在），last_user_bytes=2335（最后一条是这块）。
  //
  // 结论：问题不在措辞，在**位置**。这个产品自己的注释写着 Put the user's ACTUAL request LAST
  // ——recency 是最高注意力位。所以一轮没跑时整块不发：环境事实是锦上添花，用户的问题在最后
  // 一位是刚需。真有活干到一半时它才有意义，那时它描述的是已经发生的事，不跟用户的话争位置。
  const i = CODE.indexOf("_hasRunActivity");
  assert.ok(i > 0, "执行状态块的活动判据不见了");
  const block = CODE.slice(Math.max(0, i - 200), i + 2600);

  // 判据里每一项都必须是「真的发生过某个动作」。崩溃恢复事实（run._resumeFact）
  // 算数：它每个会话最多出现一次，且它陈述的正是上一轮**真的落了盘**的文件——
  // 续跑的第一次决策此刻手里是张白纸，最容易把已经改好的文件从头再写一遍。
  // 被点名禁止的是 _runtimeStateBlock 那类**对任何真实项目都非空**的东西：
  // 放进来等于每轮必发，把用户的问题从最后一位挤走。
  const _actExpr = block.match(/const _hasRunActivity = [^;]+;/)?.[0] || "";
  assert.ok(_actExpr, "执行状态块的活动判据不见了");
  assert.doesNotMatch(
    _actExpr,
    /_runtimeStateBlock/,
    "运行时状态对任何真实项目都非空，放进活动判据等于每轮必发",
  );
  for (const term of ["_mutatedFiles.size", "_readFiles.size", "_evidenceBlock", "_latestDiagBlock", "_planLine"]) {
    assert.ok(_actExpr.includes(term), `活动判据里少了 ${term}`);
  }
  assert.match(
    block,
    /if \(_hasRunActivity\) \{/,
    "整块必须只在**真有动作**时才发。曾经的写法是 `if (_hasRunActivity || _runtimeStateBlock)`——"
    + "而运行时状态对任何真实项目都非空，等于每轮必发，把用户的问题从最后一位挤走",
  );
  // _planLine 是 2026-08-20 加进这个判据的第五项，理由要写清楚，因为它看起来像在放宽：
  //   · 它不是"环境事实"。有计划意味着模型自己调过 update_plan，或继承了一份未完成的计划——
  //     那本身就是已经发生过的动作，符合这块"描述已发生的事"的定位。
  //   · 而它恰恰在这块原本发不出去的那一刻最要紧：用户打"继续"的第一次模型调用，
  //     还没落盘、还没读文件，位置感为零——"不知道此刻该做什么"就是从这里开始的。
  //   · 当初那次事故（模型把这块当成用户的话，得出"用户没提出具体问题"）的根因是位置，
  //     补救是块尾无条件把用户原话带回最后一位。那条补救仍在，见下面那条断言。
  assert.match(block, /用户这一轮真正要的是下面这句/,
    "块尾必须把用户原话带回最后一位——这是上面那条判据能放宽的前提");

  // 那段"请求在上面的对话里"是上一版的补救，位置没动所以补救失败，且它本身把模型送去翻找。
  assert.doesNotMatch(block, /用户这一轮的请求就在上面的对话里/,
    "这句是上一版只改措辞不改位置的产物，模型照着它去翻找然后报告找不到——整块不发之后它就不该存在");

  // 有动作那一支保持原样：那时它描述的是已经发生的事实。
  assert.match(block, /接着完成尚未完成的动作/, "有动作时仍要催它接着做完");
  assert.match(block, /〔执行状态·不要从头重查〕/);
});
test("完整裁决要 19.8 秒，路由必须有第二条腿——而且那条腿只喂请求头", () => {
  // 生产网关实测（2026-08-17）：
  //   17:37:51 分类器发出 → 17:38:06 主回合发出（等待 15s 超时）→ 17:38:10 分类器才回
  //   upstream_header_ms=19836
  // 只有完整裁决这一条腿时，这道等待就是二选一：干等二十几秒，或者这一轮工程/调研/设计/
  // Git/协作/自动化/缺陷八个模块一个都不挂。两个都不能接受，所以拆快通道。
  assert.match(CODE, /async function _fastRoutingFlags\(/,
    "路由快通道没了——那这道等待又回到了「干等」和「没有模块」的二选一");

  // 用真正的函数提取器，不是固定字符窗口。窗口切法每次往这个函数里加一句注释或一行代码
  // 就会把函数收尾挤出去，于是断言不是变红、而是**静静地不再守着尾部那几条**——本文件
  // 已经因此把窗口从 3300 调到 3600 一次。fnSource 按 AST 边界取整个函数，加多少都不漂。
  const fn = fnSource("_fastRoutingFlags", { code: true });

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
  assert.match(wait, /Promise\.race\(\[\s*_turnIntentExactPromise,\s*_fastRoutingFlags\(|Promise\.race\(\[\s*_turnIntentExactPromise,\s*_fastRoute,/,
    "两条腿必须在同一个 race 里——串行等待就没有意义了");

  // 快通道的**启动点**已经搬到这道等待之外，而且必须在它之外。
  //
  // 原来它是等待块里的一个 const：既然生在块里，就只有块里那次 race 能用它，块外那行同步的
  // `_fastRouteProfile || _turnEngineeringResolved` 是它唯一的读者。而这条腿自己是一次完整的
  // 模型调用（生产首响应头 8~18 秒），它结构上赢不了 6 秒的窗口——赢不了就等于结果没有读者。
  // 生产 46/46 语义画像全空正是这个形状。启动点提到块外，结果才有机会在循环边界落地。
  const startAt = CODE.indexOf("const _fastRoute = ");
  const waitAt = CODE.indexOf(WAIT_ANCHOR);
  assert.ok(startAt > 0 && startAt < waitAt,
    "快通道的启动点必须在等待块之外——生在块里，它的结果就只有那一次赢不了的 race 能读到");
  assert.doesNotMatch(CODE.slice(startAt, startAt + 200), /_intentWaitPaid/,
    "快通道的启动判据不许挂在 _intentWaitPaid 上：那个标志记的是「这一轮等过」，"
    + "一置真整条会话就再也不发快通道了");
});

test("角色计划第一轮就要到，但只当指路用——闸门仍然只认完整裁决", () => {
  // 完整裁决实测 19.8 秒，第一轮通常没有。而"该派哪些角色"恰恰是第一轮就要决定的事：
  // 等到第二轮，活已经按 solo 干起来了，多角色那条路等于没通。
  //
  // 边界是死的：给模型的**信息**可以走快通道，harness 自己的**闸门**（计划门槛、写入义务、
  // 角色派发准入）仍然只认完整裁决。精简判断可以指路，不该管闸门。
  // 同上：AST 边界取整个函数，不用固定字符窗口（那种切法加几行注释就静静地不再守尾部）。
  const fn = fnSource("_fastRoutingFlags", { code: true });
  assert.match(fn, /orchestrationMode 不是 solo 时，再给 roleNeeds/,
    "快通道没问角色——第一轮就不知道该派谁");
  // 逐个对目录校验这条不放，但"目录"是两处：内置角色表 + 用户自己声明的角色
  // （_userRoleMap）。孪生的完整裁决分支一直是两处都认，快通道只认内置那一处，
  // 于是同一个自定义角色"有时候认有时候不认"——最难查的那类不一致。
  assert.match(fn, /\.filter\(\(role\) => _AI_AGENT_ROLES\.has\(role\) \|\| _userRoleMap\(\)\.has\(role\)\)/,
    "角色名必须逐个对着目录校验（内置表 + 用户自定义两处都要认）");
  assert.match(fn, /profile\.orchestrationMode !== "solo" && Array\.isArray\(raw\.roleNeeds\)/,
    "solo 时不该带角色清单");
  assert.match(fn, /\[\.\.\.new Set\(roles\)\]\.slice\(0, 5\)/, "角色要去重并封顶，别让它列一长串");

  // 临时契约必须**自报是临时的**：否则模型会把初判当最终结论，完整裁决到了也不改。
  // 这两段文本已经抽成各自的生成器（同一段话原来在两个分支里写了两遍）。
  const body = fnSource("_provisionalRolesText", { code: true })
    + "\n" + fnSource("_agentDecisionFrameBlock", { code: true });
  assert.match(body, /〔协作初判·完整意图裁决还在路上，这是快速判断〕/,
    "临时契约没有自报身份——模型会把它当成最终结论");
  assert.match(body, /完整裁决落定后会自动补全或纠正/, "要说清它会被修正，否则模型不敢改口");
  assert.match(body, /else if \(provisional && provisional\.orchestrationMode && provisional\.orchestrationMode !== "solo"\)/,
    "临时契约必须只在完整契约缺席、且真的要多角色时才出现——否则纯属噪音");
});


test("第 2 轮起本轮契约必然赶不上第一发——要把上一轮的契约带上去顶着", () => {
  // 那道等待窗口按会话只付一次（sess._intentWaitPaid，理由写在 sendPrompt 里）：
  // 第 2 轮起，本轮裁决必然赶不上第一次模型调用，契约要等循环边界的 late-adopt 才有，
  // 也就是**第二个模型回合**。而一轮里最关键的判断——要不要动手、动哪儿、算不算做完——
  // 就在第一发决定完了。上一轮收敛出来的契约躺在 sess._intentState.semantic 里，
  // 零延迟零成本，不用它纯属浪费。
  const body = fnSource("_agentDecisionFrameBlock", { code: true });
  const carry = fnSource("_priorContractText", { code: true });
  assert.match(body, /priorSemantic = null/, "没有接上一轮契约的入口");
  assert.match(carry, /上一轮已经收敛的契约（本轮裁决还在路上，先按它开工）/,
    "带过来了却不自报身份 —— 模型会把上一轮的目标当成这一轮的");
  assert.match(carry, /用户这一轮的原话优先/,
    "必须明说用户改主意时以他为准，否则这块会把用户的转向压掉");
  // 只带耐久的那几维，而且是**真的跑一遍**看渲染结果 —— 只按源码文本断言
  // `priorSemantic.constraints` 之类会被 else-if 的守卫喂到（那里也出现同一个属性名），
  // 于是删掉真正那行 push 照样全绿（实测漏网）。
  const frame = load("_agentDecisionFrameBlock", {
    // 真实实现太重（要整套画像），这里注一个**最小但真实形状**的契约生成器：
    // 它只回答「本轮契约在不在场」，而这条断言问的正是这个。
    _agentIntentExecutionBlock: (p) => (p?.intentSemantic?.goal
      ? `🎯 本轮意图契约\n目标：${p.intentSemantic.goal}` : ""),
    _engineeringProfileWithAiIntent: () => ({}),
    _priorContractText: load("_priorContractText"),
    _provisionalRolesText: load("_provisionalRolesText"),
  });
  const rendered = frame("再改改", {}, null, {
    goal: "做一个多租户后台",
    action: "实现",
    target: "计费模块",
    constraints: ["金额一律用分"],
    successCriteria: ["跑通回归"],
    // 下面三个是对**上一句话**的判断，一个字都不该出现在这一轮。
    restatedTask: "上一句话被复述成了这样",
    continuation: "continue",
    ambiguities: ["上一句里那个没搞清的点"],
  });
  for (const durable of ["做一个多租户后台", "计费模块", "金额一律用分", "跑通回归"]) {
    assert.ok(rendered.includes(durable), `耐久维度没渲染出来：${durable}`);
  }
  for (const perMessage of ["上一句话被复述成了这样", "上一句里那个没搞清的点"]) {
    assert.ok(!rendered.includes(perMessage),
      `${perMessage} 是对上一句话的判断，带到这一轮就是张冠李戴`);
  }
  // 上一轮什么都没收敛过（第 1 轮、或裁决从没落定）时整块不发，别凭空多一段。
  assert.ok(!frame("随便说点什么", {}, null, null).includes("上一轮已经收敛的契约"),
    "没有上一轮契约时不该凭空造一块");
  assert.ok(!frame("随便说点什么", {}, null, {}).includes("上一轮已经收敛的契约"),
    "空的语义帧同样不该发");

  // 完整契约在场时不许出现：那才是本轮真正的裁决，两份契约同时摆着必然打架。
  //
  // 这条原来只比两个 indexOf 的先后 —— 把 `else` 删掉（也就是它自己点名的那个缺陷）
  // 照样全绿，因为文本顺序没变。改成**真的跑一遍**：本轮契约在场时，上一轮那块必须缺席。
  const withCurrent = frame("再改改", {
    intentSemantic: { goal: "本轮真正的目标", restatedTask: "本轮的复述", confidence: 0.9 },
  }, null, { goal: "上一轮的目标", constraints: ["上一轮的约束"] });
  assert.ok(withCurrent.includes("本轮真正的目标"), "本轮契约本身没发出来");
  assert.ok(!withCurrent.includes("上一轮已经收敛的契约"),
    "本轮裁决已经在场，上一轮那块还在发 —— 两份契约同时摆着，模型必然打架");
  assert.ok(!withCurrent.includes("上一轮的目标"));
});
