// 任务计划的自动推进：什么样的证据才算"这一步做完了"。
//
// 假完成比不推进糟得多：一个停在"进行中"的步骤只是看着慢，用户会自己等；
// 一个假勾会让用户以为事情已经做了，然后基于一个没发生的事实往下走。
//
// 真实事故：一份 7 步的建站计划里有 3 步的措辞没被动词表认出来
// （"配置 Tailwind…设计 tokens"、"面包店门店信息与地图"、"把首页跑起来给用户看"），
// 而当时的兜底是「分不出类 → 任何证据都算完成」，于是模型只是读了个文件，
// 这三步就在界面上显示成做完了。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

function extractFn(name) {
  const i = SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `找不到 ${name}`);
  let depth = 0, j = SRC.indexOf("{", SRC.indexOf(")", i));
  for (; j < SRC.length; j++) {
    const c = SRC[j];
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  return SRC.slice(i, j + 1);
}

const actionKind = new Function(`${extractFn("_planStepActionKind")}\n;return _planStepActionKind;`)();
const matches = new Function(
  `${extractFn("_planStepActionKind")}\n${extractFn("_planStepMatchesEvidence")}\n;return _planStepMatchesEvidence;`,
)();

// 用户那份真实计划，逐字照抄
const REAL_PLAN = [
  "创建 Vite + React + TypeScript 项目结构，配置 package.json 和依赖（Tailwind CSS、Framer Motion、Lucide React）",
  "配置 Tailwind CSS v4 CSS-first，设置面包店暖色调设计 tokens（stone + amber 配色）",
  "创建可复用基础组件（Button/Card/Container）使用 shadcn/ui 模式和语义 token",
  "实现 Navbar 组件（固定顶部、透明背景、logo + 导航链接 + CTA 按钮）",
  "实现 Hero section（全屏高度、背景图、标题动画）",
  "面包店门店信息与地图",
  "把首页跑起来给用户看",
];

test("一次读取不会把任何一步标成完成", () => {
  // read_file / list_dir / search 产出的是 investigate 证据。它只能推进"调查"型步骤，
  // 绝不该推进"实现"或"运行"型步骤——更不该推进一个连类型都判不出来的步骤。
  for (const step of REAL_PLAN) {
    assert.equal(matches({ content: step }, ["investigate"]), false,
      `一次读取就把这步标完成了：「${step.slice(0, 40)}」`);
  }
});

test("分不出类型的步骤一律不自动打勾", () => {
  // 旧兜底是「分不出类 → 有任何证据就算完成」，方向正好反了。
  // 判不出这一步要做什么，就没有资格断定它做完了；交回给模型自己的 update_plan。
  const unclassifiable = ["面包店门店信息与地图", "第二阶段", "收尾", "……"];
  for (const step of unclassifiable) {
    assert.equal(actionKind({ content: step }), "", `这条本该判不出类型：${step}`);
    for (const kinds of [["investigate"], ["implement"], ["verify"], ["execute"], ["implement", "verify"]]) {
      assert.equal(matches({ content: step }, kinds), false,
        `判不出类型却被 ${kinds.join("/")} 证据打勾了：${step}`);
    }
  }
});

test("对得上类型的真实证据仍然照常推进", () => {
  // 修复不能把自动推进整个废掉——那会让计划永远停在第一步。
  const cases = [
    ["实现 Navbar 组件", "implement"],
    ["创建 Vite + React + TypeScript 项目结构", "implement"],
    ["配置 Tailwind CSS v4，设置设计 tokens", "implement"],
    ["把首页跑起来给用户看", "execute"],
    ["部署到线上", "execute"],
    ["梳理现有代码结构", "investigate"],
    ["跑一遍测试确认没坏", "verify"],
  ];
  for (const [step, kind] of cases) {
    assert.equal(actionKind({ content: step }), kind, `分类错了：${step}`);
    assert.equal(matches({ content: step }, [kind]), true,
      `对应证据反而推不动了：${step}`);
  }
});

test("execute 型步骤接受验证类证据，但反过来不成立", () => {
  // 跑起来之后拿到构建/测试输出，算它跑过了；但"跑一下"不能算"验证过了"。
  assert.equal(matches({ content: "把首页跑起来" }, ["verify"]), true);
  assert.equal(matches({ content: "跑一遍测试确认没坏" }, ["execute"]), false);
});

test("失败的工具调用不产生任何证据", () => {
  // 这条守的是另一半：工具报错了，那一步当然不算做完。
  const kinds = extractFn("_planEvidenceKindsForTool");
  assert.match(kinds, /_toolExecutionSucceeded\(call, result\)/,
    "取证据前必须先确认这次调用真的成功了");
  assert.match(kinds, /return \[\]/, "没成功就返回空证据");
});

test("兜底方向写进了注释，避免以后又被改回去", () => {
  const fn = extractFn("_planStepMatchesEvidence");
  assert.match(fn, /if \(!kind\) return false;/, "分不出类必须直接返回 false");
  assert.match(fn, /假完成|不要自动打勾/, "要写明为什么，否则很容易被当成过严又改回去");
});

// ── 中断之后说「继续」：不该从头重来 ─────────────────────────────────────────
//
// 用户报的：中断修复过程后让它继续，它会重新从头读取文件、任务计划被打乱不遵守、
// 上下文丢失。查下来是三处断口叠在一起：
//
//   1. 计划从来没进过执行模型的提示词。unfinishedPlan 只喂给了**意图分类器**
//      （旁路小模型，判定任务维度用），真正干活的模型从没见过这份计划。
//   2. 新一轮的 run._planSteps 只有模型自己调 update_plan 才有值，中断重开时是空的——
//      界面上计划条还画着，模型和执行侧手里却什么都没有。
//   3. 历史是 text-only（工具调用和结果不入库，为了压缩和重放安全），所以"读过什么"
//      只能靠证据账本转达；不明说，模型就会重读。
function resumeBlock() {
  const i = SRC.indexOf("function _resumeHandoffBlock");
  const tail = '\n  } catch { return ""; }\n}';
  const end = SRC.indexOf(tail, i);
  assert.ok(i > 0 && end > i, "找不到 _resumeHandoffBlock");
  return new Function(SRC.slice(i, end + tail.length) + "\nreturn _resumeHandoffBlock;")();
}
const PLAN = [
  { content: "读取现有实现", status: "completed" },
  { content: "补上隔离", status: "completed" },
  { content: "给失败分支补卡片", status: "in_progress" },
  { content: "跑完整测试", status: "pending" },
];

test("中断之后要把计划和进度交接给模型——不然它只能重新规划", () => {
  const out = resumeBlock()({ _planSteps: PLAN, _lastRunState: { outcome: "partial", task: "把卡片做完", incompleteReason: "用户中断" } });
  assert.match(out, /接着上次继续——不是新任务/);
  assert.match(out, /已完成的步骤（别重做）：读取现有实现、补上隔离/);
  assert.match(out, /还没做的步骤（照这份走，别重新规划）：给失败分支补卡片、跑完整测试/);
  assert.match(out, /不要重新 search 定位、也不要整份重读/, "没有明说别重读，它就会重读");
});

test("正常收尾之后开的新任务，不能被上一轮的旧计划粘住", () => {
  assert.equal(resumeBlock()({ _planSteps: PLAN, _lastRunState: { outcome: "success" } }), "");
});

test("计划全做完 / 压根没有计划时，不输出这段", () => {
  const rb = resumeBlock();
  assert.equal(rb({ _planSteps: PLAN.map((x) => ({ ...x, status: "completed" })), _lastRunState: { outcome: "partial" } }), "");
  assert.equal(rb({ _lastRunState: { outcome: "failed" } }), "");
});

test("这段要真的进提示词，而且排在项目上下文之前", () => {
  // 它讲的是"这一轮该怎么接"，比项目背景更该被先读到。
  const at = SRC.indexOf("const _dynPreamble =");
  assert.ok(at > 0);
  const seg = SRC.slice(at, at + 500);
  assert.match(seg, /_resumeBlock \? _resumeBlock \+ "\\n\\n" : ""/, "交接块没有拼进提示词");
  assert.ok(seg.indexOf("_resumeBlock") < seg.indexOf("项目上下文"), "交接块排在了项目上下文后面");
});

test("新一轮要继承会话里那份没做完的计划", () => {
  const at = SRC.indexOf("run._sessionGen = session._runGen || 0;");
  assert.ok(at > 0);
  const seg = SRC.slice(at, at + 1400);
  assert.match(seg, /run\._planSteps = _prevPlan\.map/, "run 没继承会话计划，步骤推进会从零开始");
  assert.match(seg, /_prevOutcome === "failed" \|\| _prevOutcome === "partial"/,
    "继承没有限定在「确实没跑完」，正常收尾后的新任务会被旧计划粘住");
});
