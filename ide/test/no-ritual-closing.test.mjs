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
import { CODE as SRC } from "./helpers/source.mjs";

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
