// 同时到场、方向相反的指令。判据严格：必须是**同一次请求里会同时出现**的两处，
// 且**方向相反**（不是详略不同）。解法一律是删掉/改写其中一条、或合成一条带判据的，
// 不是再加一句「注意不要……」——那正是本仓库第一原则「修机制不是加劝诫」。
//
// 这批来自一次 20 智能体的排查 + 逐条对抗验证，22 条候选、11 条高危。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, SRC as RAW_SRC } from "./helpers/source.mjs";
import { readFileSync } from "node:fs";
const P = (n) => readFileSync(new URL(`../../server/prompts/${n}.txt`, import.meta.url), "utf8");

// ── 仪式与格式强制族（用户反复投诉的那段模板的直接来源）─────────────
test("小任务律不再无条件要求「验证/未验证」两段", () => {
  // 「完成后给结果和真实验证/未验证边界」是无条件的，而 answer_quality.txt:33 把
  // 「a rundown of what you checked」按**形状**禁掉、只有 FAILED check 才值一行。
  // 一边强制两段、一边按形状禁止，模型只能照强制的做——那正是投诉里的前两节。
  assert.match(SRC, /小任务律：直接用最短证据链完成；能一两个工具搞定就别升级成长流程。/,
    "尾巴又回来了");
  assert.doesNotMatch(SRC, /完成后给结果和真实验证\/未验证边界/, "无条件的验证两段又回来了");
});

test("不再强制把话拆成 bullet", () => {
  // answer_quality.txt:27「Turn two sentences into a bulleted list … do not」，
  // 而客户端两处风格块写着「要点用短 bullet，别用大段连续文字」。方向相反。
  assert.doesNotMatch(SRC, /结构化代替啰嗦/, "「用 bullet 别用连续文字」又回来了");
  assert.doesNotMatch(SRC, /正文用 Markdown 结构化/, "同上，第二处");
  assert.doesNotMatch(SRC, /能列表别长段/, "同上，第三处");
  // 留下的必须是**有判据**的版本，不是删干净
  assert.match(SRC, /只有条目\*\*真的平行\*\*时才用列表/, "删了却没留判据，模型会退回默认排版");
});

test("reviewer 的覆盖面说明挪到开头，不做仪式性收尾", () => {
  const r = P("reviewer");
  assert.doesNotMatch(r, /Finish with a one-line overview/,
    "又变回收尾行了——answer_quality.txt:33 里 a rundown of what you checked 就是这一行");
  assert.match(r, /Open with one line/, "挪走了却没给新位置");
  assert.match(r, /not in a closing summary/, "没说清为什么不能放结尾");
});

test("chat 模式结尾不再强制「给一两条下一步」", () => {
  assert.doesNotMatch(P("chat"), /finish by offering one or two things/,
    "一边 answer_quality 按形状禁止仪式结尾，一边 chat.txt 强制要求");
});

// ── 过度设计与范围蔓延族 ─────────────────────────────────────────
test("默认可维护不等于默认造扩展点", () => {
  // agent_engineering.txt:7「This is not a licence to build for imagined futures」，
  // 抽象层要等第三处真的需要时才抽；而可维护升级律写着默认就要「扩展点可替换」。
  const laws = SRC;
  assert.doesNotMatch(laws, /组件\/服务可复用、扩展点可替换/,
    "「默认就要可扩展」又回来了——单调用点的功能会被抽出接口 + 注册表 + 配置项");
  assert.match(laws, /可维护升级律/, "整条律被误删了——配置集中和禁止硬编码还要留");
  assert.match(laws, /禁止把业务规则、颜色、端口、密钥、路径和魔法值散落硬编码/,
    "该留的那半句丢了");
});

test("业务逻辑做扎实 ≠ 顺手多写三个功能", () => {
  // agent_engineering.txt:5「Commercial grade is a quality standard — it is not licence to
  // expand the feature scope」；而业务逻辑律写着「必须覆盖取消、退款、…审计记录」。
  assert.doesNotMatch(SRC, /必须覆盖取消、退款、重复提交、超时、并发、历史数据和审计记录/,
    "扩范围那句又回来了——用户只要下单接口，会顺手把取消/退款/审计一起写了");
  assert.match(SRC, /\*\*你这次要写的那条流程上\*\*的重复提交、超时、并发、幂等/,
    "收窄到本次流程的那句没了");
  assert.match(SRC, /取消、退款、历史数据迁移、审计记录是另外的功能面/,
    "没说清哪些属于别的功能面，模型无从划界");
});

test("「先写红测试」要有判据，不是无条件", () => {
  // agent_engineering.txt:22 写的是 reproduce **or** read the real error。
  assert.match(SRC, /\*\*项目已经有测试体系、而这个 bug 落在它覆盖得到的范围里时\*\*/,
    "又变回无条件了——没有测试体系的项目会被逼着为一个空值判断从零搭一套");
  assert.match(SRC, /reproduce \*\*or\*\* read/, "没说清另一条路同样算数");
  assert.match(SRC, /先看着它红，再动手修/, "有体系时的先红后绿被误删了");
});

// ── 想 vs 干 ────────────────────────────────────────────────────
test("催写文件要看计划落地了没有", () => {
  // 同一份上下文里还有「计划落地前不写文件」和「先把不确定的地方查清楚再动手」。
  // 无条件催写文件时，需要计划的任务会被逼在没计划的情况下开写——而计划门那边还会
  // 因为「从零建的第一次落盘没有计划」把这次写入硬拦回去，两头空转。
  assert.match(SRC, /const _planLanded = Array\.isArray\(run\._planSteps\) && run\._planSteps\.length > 0;/,
    "行动门禁又变回无条件了");
  assert.match(SRC, /_planLanded \|\| !_runRequiresPlan\(run\)/, "判据算出来了却没接上");
  assert.match(SRC, /但\*\*还没有计划\*\*/, "需要计划那一支没有给出正确的下一步");
  // 反向：防「只思考不写代码」那条原意不能丢
  assert.match(SRC, /现在立即开始 write_file 创建第一批文件/, "催动手那句被整条删了");
});

test("从零建系统类项目要先读真实实现，建应用则直接开工", () => {
  // reasoning.txt 是基座、无条件挂载，原来无条件说「不要去查有没有人做过」；
  // 而外部知识律要求从零写版本控制/编译器/数据库时第一份代码前先查。合成一条带判据的。
  const r = P("reasoning");
  assert.match(r, /application-shaped/, "没有区分应用型和系统型");
  assert.match(r, /systems artifact whose design trade-offs are the work itself/,
    "系统型那一档没写出来");
  assert.match(r, /version control system, a compiler, a database/,
    "没给具体例子，判据落不了地");
  // 反向：应用型仍然直接开工，不许退回「先比较一圈技术选型」
  assert.match(r, /pick the mainstream stack from what you already know and start building/,
    "把「直接开建」也删了——那会让每个待办应用都先做一轮选型调研");
});

// ── 执行前提族 ──────────────────────────────────────────────────
test("用户说「这轮只出计划」时也不催他去做第一步", () => {
  assert.match(SRC, /run\.engineering\?\.explicitReadOnly === true/,
    "agent 模式里明说只规划时仍被催着执行——agent_core.txt:6：A plan request ends at the plan");
});

test("命令根本不存在时，不许指示去改代码", () => {
  // agent_engineering.txt:32：a verifier that cannot run asserts nothing about the code。
  assert.match(SRC, /command not found\|no such file or directory\|: not found/,
    "cmdFail 又不分「代码错了」和「命令没找到」了");
  assert.match(SRC, /\*\*不要\*\*去改代码找根因——它一行都没被执行过/,
    "认出来了却还给同一句指示");
  // 反向：真报错时那条正确的指示不许丢
  assert.match(SRC, /照它刚输出的那段真实报错定位根因、直接改对应的文件:行/,
    "真失败时的正确指示被整条删了");
});

test("不许默默把用户要的东西换成自己认定的真问题", () => {
  // truthfulness.txt:18 要求说清问题所在、**然后照用户要的做**，并把默默替换叫
  // silently implementing something else；而 agent_core.txt:5 原来允许
  // 「solve the real problem and say why」。用户的选择权不能被一句推断夺走。
  const c = P("agent_core");
  assert.doesNotMatch(c, /solve the real problem and say why/,
    "又允许默默换掉用户要的东西了");
  assert.match(c, /then build what they asked for/, "没说清正确做法是「说明 + 照做」");
  assert.match(c, /quietly substituting your own reading for theirs is not/,
    "没点名这就是 silently substituting");
});

// ── 死副本里那些「活版本确实缺」的规则，接回来了没有 ────────────────
//
// 8 个 .txt 在服务端躺着，客户端 _P() 查找键 100% 落空（接口只对 admin 且 ?full=1 返回正文，
// IDE 从不这么请求；生产 3495 次请求每次 body 恰好 54 字节，prompts 恒为空对象）。
// 于是永远回落到 main.js 的内联版——而那份**不是死副本的超集**，两边各有独家内容。
// 这里钉住的是「.txt 独有、活版本确实缺、且现在仍然正确」的那些，已经接进活版本。
const inlineOf = (name) => {
  const m = new RegExp("_P\\(\"" + name + "\",\\s*`([\\s\\S]*?)`\\)").exec(RAW_SRC);
  assert.ok(m, `${name} 的内联版定位不到了`);
  return m[1];
};

test("压缩摘要不再写死中文——它会替换掉真实历史成为之后每轮的上下文", () => {
  const t = inlineOf("compact");
  assert.doesNotMatch(t, /中文、分条/,
    "写死中文：一场英文对话被压缩后，历史里躺着一段中文，直接顶撞 agent_core 和五个模式提示词里"
    + "各写了一遍的「用与用户相同的语言，不要默认中文」");
  assert.match(t, /用这段对话本身的语言/, "改了却没给替代说法");
});

test("worker 人格补上它此前一条都没有的操作规则", () => {
  const t = inlineOf("worker_system");
  // worker 拿到的只有 _WORKER_SYSTEM + scope，拿不到主智能体的决策框，所以这些必须写在人格里
  assert.match(t, /Do not emit a huge file in one shot/, "死锁保护没了");
  assert.match(t, /never to one that already exists/,
    "少了那个关键区分——对已存在的文件「先写骨架」会把用户正在工作的代码截断成几十行");
  assert.match(t, /carry on from it rather than/, "没说要承接主智能体已有的上下文");
  assert.match(t, /watch for when integrating/, "交付里没有集成交接");
  assert.match(t, /comes back as \[BLOCKED\]/, "没说清越界写会怎样");
  assert.match(t, /run_cmd is not scope-limited/,
    "没堵住用命令绕过 scope——这是唯一一条能绕开作用域的路");
  assert.match(t, /belong to the main agent after every worker is done/,
    "没说跨模块接线和最终集成归谁");
});

test("subagent 人格补上能力边界与检索纪律", () => {
  const t = inlineOf("subagent_system");
  assert.match(t, /two independent pieces of evidence/, "关键结论只要一条证据就下了");
  assert.match(t, /Do not restate the task/, "没禁复述任务和开场白");
  assert.match(t, /you do not have the browser tool/,
    "这条边界一直是真的（只读集合里没有 browser），却从没说给模型听，它只能靠撞 [BLOCKED] 才知道");
  assert.match(t, /Think, then look/, "缺了「先想清楚缺哪一块再检索」");
  assert.match(t, /Follow the thread/, "缺了「顺着 import/调用/定义逐层追」");
});

test("项目摸底子体拿到可执行的判据和输出模板", () => {
  const t = inlineOf("research_prompt");
  assert.match(t, /技术栈只认清单文件和真实代码/, "技术栈还可以靠猜");
  assert.match(t, /读全/, "关键文件可以只读开头几行");
  assert.match(t, /## 目录地图/, "没有固定输出模板，交付形状随机");
  assert.match(t, /## 常见改动入口/, "同上，六段模板不全");
  assert.match(t, /不要只列文件名/, "会交回一份文件名清单");
});
