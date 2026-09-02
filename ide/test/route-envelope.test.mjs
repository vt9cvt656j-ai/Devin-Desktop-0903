// 路由层根本不需要那份 43 字段的大裁决——它需要的是一份小表，而那份小表恰好是弱模型
// 唯一实测产得出的那条腿。
//
// 实测差集（2026-08-23）：把「_ideSemanticProfile 能点亮的全部旗标」和「快通道键表能
// 点亮的」逐个比对，快通道**只差 domain_* 这一族**。补上 domain 就是 21/21 全覆盖。
// 用户 97% 的请求跑在一个产不出大 JSON 的模型上，这条差集决定了他能不能拿到语料路由、
// 工程纪律、取证要求——今天这些在生产上是 0。
//
// 顺带修掉一个 fail-open：design_data 用黑名单判 dataStrategy，而快通道以前不产这个
// 字段，`![...].includes(undefined)` 恒为真 → 每一个 uiProject 回合都误亮。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, load, loadConst, fnSource as topLevelFn } from "./helpers/source.mjs";

const prof = load("_ideSemanticProfile");
const KEYS = loadConst("_FAST_ROUTING_KEYS");
const DOMAINS = loadConst("_AI_KNOWLEDGE_DOMAINS");
const flagsOf = (p) => new Set(prof(p).split(":")[1].split(",").filter(Boolean));

// 「快通道能产出的一切」：键表里的布尔 + 它会解析的枚举
const fastAll = () => {
  const o = {};
  for (const k of KEYS) o[k] = true;
  return {
    ...o,
    workspaceAction: "modify",
    designMode: "michael_design_2_5_greenfield",
    orchestrationMode: "staged_roles",
    changeScope: "project",
    dataStrategy: "server",
    domain: "healthcare",
  };
};
// 「整个画像能点亮的一切」：把 _ideSemanticProfile 读的字段全给上
const everything = () => ({
  ...fastAll(),
  applies: true, projectEngineering: true, desktopAutomation: true, capture: true, git: true,
  debugProject: true, securityRisk: true, explicitReadOnly: true, existingProject: true,
  existingWebsite: true, designKnowledgeRequired: true, ui: true, uiProject: true,
  fullWebsite: true, richMediaRequired: true, motionDesignRequired: true,
  needsReferences: true, needsOfficialResearch: true, needsCommunityResearch: true,
});

// ── 一、覆盖率：这是整件事的支点 ────────────────────────────────────────
test("快通道能点亮的旗标，覆盖画像能点亮的全部", () => {
  const missing = [...flagsOf(everything())].filter((f) => !flagsOf(fastAll()).has(f));
  assert.deepEqual(missing, [],
    `快通道够不到这些旗标：${missing.join(",")}——而它是弱模型唯一产得出的那条腿，`
    + "够不到就等于这些模块在那台机器上永远不挂载");
});

test("domain 就是那块曾经缺掉的拼图", () => {
  // 反向证明：把 domain 拿掉，覆盖率必须立刻破。否则上面那条断言是绿的摆设。
  const noDomain = { ...fastAll(), domain: "" };
  const missing = [...flagsOf(everything())].filter((f) => !flagsOf(noDomain).has(f));
  assert.ok(missing.some((f) => f.startsWith("domain_")),
    "拿掉 domain 覆盖率却没破——说明这条覆盖率断言根本没在量 domain");
});

// ── 二、快通道真的会产出并解析这两个字段 ────────────────────────────────
const fastSrc = topLevelFn("_fastRoutingFlags", { code: true });

test("快通道的提示词里真的要了 domain 和 dataStrategy", () => {
  assert.match(fastSrc, /domain=这件事属于哪个\*\*业务领域\*\*/,
    "提示词没要 domain——解析侧写得再对也收不到");
  assert.match(fastSrc, /dataStrategy=not_applicable\|none\|local\|server/);
  // 判据必须写清「按业务领域判，不按技术判」——否则 healthcare 会被填成 web-frontend。
  assert.match(fastSrc, /不按用的技术判/, "没写清按业务领域判，模型会按技术栈填");
});

test("解析侧收下这两个字段，且 domain 对着真实目录名归一", () => {
  assert.match(fastSrc, /"changeScope", "dataStrategy"/, "dataStrategy 没进枚举白名单");
  assert.match(fastSrc, /_aiIntentKnowledgeDomain\(raw\.domain\)/,
    "domain 没做归一——目录名带连字符，走枚举那条归一会把它改坏");
  assert.doesNotMatch(fastSrc, /_aiIntentEnum\(raw\.domain/,
    "domain 走了枚举归一：web-frontend 会被改成 web_frontend，之后核不上任何目录");
});

test("只判出 domain 也算「判过了」，不能被当成空画像丢掉", () => {
  // meaningful 判据漏了 domain 的话，一个纯问答轮里模型判出了领域也会被整份丢弃。
  assert.match(fastSrc, /\|\| !!profile\.domain/,
    "只给出 domain 时快通道返回 null——那块拼图刚补上就又丢了");
});

test("输出样例要带上这两个键，否则模型不知道该产", () => {
  assert.match(fastSrc, /"dataStrategy":"not_applicable"/);
  assert.match(fastSrc, /"domain":""/);
});

test("域名单从真表展开，不是手抄一份", () => {
  // 手抄一份就会漂：语料目录随运营增删，写死的名单是下一次静默失效。
  assert.match(fastSrc, /_AI_KNOWLEDGE_DOMAINS/, "域名单被手抄进提示词了");
  assert.doesNotMatch(fastSrc, /healthcare\/finance\/legal\/security\/penetration/,
    "提示词里出现了手抄的域名单——它和真表会漂");
});

// ── 三、design_data 的 fail-open ────────────────────────────────────────
test("dataStrategy 缺席时不点 design_data（fail-closed）", () => {
  assert.ok(!flagsOf({ uiProject: true }).has("design_data"),
    "字段缺席就误亮——黑名单写法 ![...].includes(undefined) 恒为真");
  assert.ok(!flagsOf({ uiProject: true, dataStrategy: "not_applicable" }).has("design_data"));
  assert.ok(!flagsOf({ uiProject: true, dataStrategy: "none" }).has("design_data"));
});

test("确实声明了要数据时照常点亮", () => {
  for (const d of ["local", "server", "inspect_existing", "undecided"]) {
    assert.ok(flagsOf({ uiProject: true, dataStrategy: d }).has("design_data"),
      `dataStrategy=${d} 没点亮 design_data——设计侧的数据层指引整块丢了`);
  }
});

test("判据是白名单，且白名单和枚举的取值域对得上", () => {
  const src = topLevelFn("_ideSemanticProfile", { code: true });
  assert.match(src, /add\("design_data", p\.uiProject && \["local", "server", "inspect_existing", "undecided"\]\.includes/,
    "又改回黑名单了——字段缺席时会 fail-open");
  // 白名单必须是枚举取值域减去两个"不要数据"的档，多一个少一个都是错。
  const enumVals = /dataStrategy=([a-z_|]+)/.exec(fastSrc);
  assert.ok(enumVals, "快通道的 dataStrategy 枚举说明不见了");
  const all = enumVals[1].split("|");
  const white = ["local", "server", "inspect_existing", "undecided"];
  assert.deepEqual([...all].filter((v) => !["not_applicable", "none"].includes(v)).sort(), [...white].sort(),
    "白名单和枚举取值域漂了——某个档会永远点不亮或永远误亮");
});

// ── 四、域名必须是真实语料目录 ──────────────────────────────────────────
test("提示词里排除 michael-design：它有自己的路由，不该混进业务域", () => {
  assert.match(fastSrc, /!== "michael-design"/,
    "michael-design 混进业务域选项了——同一份语料会被注两遍");
  assert.ok(DOMAINS.has("michael-design"), "真表里应该有它，只是不进这份选项");
});

// ── 五、架构模式：这一轮要不要做架构决定 ──────────────────────────────
test("快通道也判 architectureMode，且判据说清了分界", () => {
  // 它不点亮任何请求头旗标，但决策框直接读它：填错会让模型在已有项目里另起一套架构、
  // 或在空目录里去「沿用」一个不存在的架构。而这两件事都发生在第一发。
  assert.match(fastSrc, /architectureMode=none\|follow_existing\|extend_existing\|design_new\|refactor_existing/,
    "快通道不判架构模式——那条腿在弱模型上是唯一跑得通的");
  assert.match(fastSrc, /判据是\*\*这一轮要不要做架构决定\*\*/,
    "只有枚举值没有判据——这种字段恒等于默认值 none");
  assert.match(fastSrc, /空目录或第一次建这类东西/, "没说清 design_new 的分界");
  assert.match(fastSrc, /证据要求整体重构才 refactor_existing/, "没给重构那一档设门槛");
});

test("解析侧收下它，且只判出它也算「判过了」", () => {
  assert.match(fastSrc, /"dataStrategy", "architectureMode"\]/, "没进枚举白名单");
  assert.match(fastSrc, /profile\.architectureMode && profile\.architectureMode !== "none"/,
    "只给出 architectureMode 时快通道返回 null——判出来了又被当成空画像丢掉");
  assert.match(fastSrc, /"architectureMode":"none"/, "输出样例没带它，模型不知道该产");
});

test("六个枚举的形状逐字一致", () => {
  // 形状漂了会让某一个枚举被静默丢掉——这份键数组是手工维护的。
  const m = /for \(const k of \[([^\]]+)\]\)/.exec(fastSrc);
  assert.ok(m, "枚举白名单取不到");
  const keys = [...m[1].matchAll(/"([a-zA-Z]+)"/g)].map((x) => x[1]);
  assert.deepEqual(keys.sort(), [
    "architectureMode", "changeScope", "dataStrategy", "designMode", "orchestrationMode", "workspaceAction",
  ], "枚举白名单和提示词里声明的那几个漂了");
  for (const k of keys) {
    assert.ok(new RegExp(`${k}=`).test(fastSrc), `${k} 在白名单里却没在提示词里声明`);
  }
});
