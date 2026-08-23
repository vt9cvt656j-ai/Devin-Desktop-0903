// 用户反复投诉多次的那段固定收尾模板：
//
//   验证结果（真实命令输出）
//   · tsc --noEmit → TSC_OK；npm run build → BUILD_OK
//   未验证的一项：……
//   你要做的唯一一步
//   两点说明：……已知边界写在 README 里
//
// 它不是硬编码的，也不是模型爱写八股——是 harness 以 **user 角色**每轮递一份带小标题的
// 结构化执行记录（[本轮交付事实]）过去，却**从没说这块是给谁用的**。一条 user 消息里塞
// 一份清单，最自然的反应就是把它转述回给用户。answer_quality 里那条「仪式性结尾按形状禁止」
// 拦不住它：那是一条规矩，而这是一份摆在眼前的模板。修机制不是加劝诫——说清用途就够了。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, SRC as RAW_SRC, fnSource } from "./helpers/source.mjs";

const at = SRC.indexOf('const _facts = run.mode === "agent"');
const block = SRC.slice(at, at + 3400);

test("交付事实块必须说清它是给自己核对用的，不是给用户转述的", () => {
  assert.ok(at > 0, "交付事实块的注入点不见了");
  assert.match(block, /这块是给你自己核对用的，不是让你转述给用户/,
    "又变回一份没说用途的清单了——模型会继续把它抄成一个「验证结果」章节");
  assert.match(block, /拿它和你打算说的话对一遍/,
    "没说清怎么用它：核对自己的措辞，不是复述内容");
});

test("用户投诉过的那几个小标题要被逐字点名", () => {
  // 只说「不要仪式性结尾」是没用的——那条规矩 answer_quality 里已经有了，而它没拦住。
  // 点名才有判据：模型知道自己正要写的那个小标题就在名单上。
  for (const heading of ["验证结果", "未验证的一项", "你要做的下一步", "已知边界"]) {
    assert.ok(block.includes(heading),
      `没点名「${heading}」——用户实拍的模板里就有这一节，不点名等于没说`);
  }
  assert.match(block, /每次都长一样的固定章节/, "没说清禁的是**形状**不是措辞");
});

test("真有没验证的东西仍然要说——禁的是形状，不是诚实", () => {
  // 反向断言。把这一整块删掉也能让上面两条变绿的写法是存在的，
  // 而那会连「没跑过验证就别说已验证」一起删掉。
  assert.match(block, /没跑过验证就别说「已验证」/,
    "诚实那条被顺手删了——这不是要模型闭嘴，是要它别按模板说话");
  assert.match(block, /会改变用户接下来怎么做[\s\S]{0,40}用一句话说清/,
    "没给「该说的时候怎么说」——只禁不给替代，模型会退回模板");
});

test("计划还欠着步骤时，「完成」两个字要当场撞上一条事实", () => {
  // 用户原话：「任务规划 没完成的内容 都会提前说完成 就很无语」。
  // harness 一直记着 plan_steps_pending，但那只是**记账**：结局卡片上写着「继续没做完的步骤」，
  // 而模型的正文早就说了完成。计划位置块（_PLAN_STATE_TAG）只说「你在第几步」，
  // 没有一句「那就别说完成」。
  assert.match(block, /st\?\.status === "pending" \|\| st\?\.status === "in_progress"/,
    "没有从计划里算出还欠几步");
  assert.match(block, /\*\*这一轮不是完成\*\*/,
    "算出来了却没说穿——记账不等于摆到模型面前");
  assert.match(block, /要么继续做，要么说清哪几步没做、为什么/,
    "只说「不是完成」不给出路，模型只能干耗或硬说完成");
  // 欠账要点名，不能只报个数字
  assert.match(block, /_openSteps\.slice\(0, 4\)\.map\(/, "没点名是哪几步");
});

test("纯问答、只读排查的回合一个字都不加", () => {
  // 自带闸门：没有交付事实、也没有欠账时不注入。
  // 这条守的是「不打扰」——每轮都塞一段，模型和用户都会学会略过它。
  assert.match(block, /if \(_facts \|\| _openLine\) \{/,
    "无条件注入了——纯问答的回合会被平白塞一段");
  assert.match(block, /const _openLine = _openSteps\.length\s*\n?\s*\?/,
    "没有计划时也发欠账那段");
});

// ── 模式契约不许被一句注入推翻 ──────────────────────────────────────
//
// plan.txt 三处写着 never modify files / never run commands，客户端 _AI_MODE_PROMPTS.plan
// 也写着「不修改文件或运行副作用命令」。而 harness 往同一份上下文里注入的两句话，
// 原来都不看模式：
//   · update_plan 的工具回执：「计划已收下，**现在去做第一步**」
//   · 每轮交付事实块：「还没做完的步骤有 N 个……要么继续做」
// 两句都在逼 plan 模式违约。多智能体审查把这两条都判成「高」，其中第二条是这次会话
// 自己引入的回归。
test("交付事实块里的「继续做」只对 agent 模式说", () => {
  assert.match(block, /const _openSteps = run\.mode === "agent"/,
    "又不分模式了——plan 模式的契约是只出方案不动手，对它说「要么继续做」正是逼它违约");
  // 注释断言要对**原文**：本文件的 SRC 是剥掉注释的 CODE（本仓库的老坑，
  // 另有记录说注释会把源码断言喂绿，所以两份要分清用途）。
  assert.match(RAW_SRC, /plan 模式的契约是「只出方案、不动手」/,
    "理由没写下来，下一个人会把它合并回去");
});

test("update_plan 的回执也按模式分开，且 plan 模式拿到的是正确的下一步", () => {
  assert.match(SRC, /const _goDo = run\.mode !== "agent"/,
    "「现在去做第一步」又不分模式了");
  assert.match(SRC, /本模式不执行其中任何一步/,
    "plan 模式没拿到替代的那一句——只删不给替代，模型手上就只剩「计划已更新」，"
    + "而它刚被要求先规划，最省事的下一步就是再规划一次（那条实测：开局连发 4~5 次，65% 跑不成）");
});

// ── 验证器起不来 ≠ 代码坏了 ─────────────────────────────────────────
test("退出 126/127 不再被当成「构建没过、代码跑不起来」", () => {
  // agent_engineering.txt:32 逐字写着 A verifier that cannot run is NOT evidence the code is
  // broken；而红构建门照单把 127 推成「代码现在跑不起来，先修根因」。用户现场就撞过：
  // `vhs demo.tape` 连着两次退出 127（工具没装），门却指示去修代码。
  const fn = fnSource("_freshBuildFailure", { code: true });
  assert.match(fn, /e\.exitCode === 127 \|\| e\.exitCode === 126/,
    "又把「命令没找到」当成代码坏了");
  assert.match(fn, /if \(_cannotRun\) continue;/, "认出来了却没跳过");
  // 判据必须是执行事实，不是正文里出现 not found（那可能是被测代码自己打印的）
  assert.match(fn, /e\.output \|\| e\.tail/, "没看运行器级输出");
});

// ── 诊断按新增算，不是按全工程 ───────────────────────────────────────
test("「先修掉 error 再收尾」对实时诊断要按新增算", () => {
  // 诊断块自己写着「这是**整个工程当前的全部诊断**，包含你动手之前就有的」，
  // 而那句无判据的「若实时诊断返回 error 就先修掉再收尾」等于要模型去清一堆本来就有的红线。
  // 收尾门走的是基线逐条抵扣（只报新增），这句原来是这条链上唯一没做基线的。
  assert.match(RAW_SRC, /实时诊断按\*\*新增\*\*算/, "又变回按全工程算了");
  assert.doesNotMatch(SRC, /若实时诊断、终端、日志、HTTP 或数据库返回 error，先解释根因并修掉再收尾/,
    "无判据的那句又回来了");
});
