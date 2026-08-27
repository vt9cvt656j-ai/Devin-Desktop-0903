// 计划自动打勾的第二种「假完成」：类别对上了，**交付物不对**。
//
// 打勾判据此前只看动作类别（investigate / implement / verify / execute）。一步写着
// 「实现 src/pages/login.tsx 的表单校验」，模型去改了 src/utils/date.ts —— 类别照样是
// implement，于是这一步被打成完成。用户看到的是进度条在走、活没干。
//
// 分不出类那种假完成已经有专测（plan-advance.test.mjs）；这一份守的是**点了名却对不上**
// 那种，它更难发现：每一步的类别都对得上。
import test from "node:test";
import assert from "node:assert/strict";
import { load, fnSource } from "./helpers/source.mjs";
import { planStepTargets, toolTouchedTargets, targetsConflict } from "../src/agent/plan-target.js";

const PLAN_STEP_KINDS = new Set(["investigate", "implement", "execute", "verify"]);
const matches = load("_planStepMatchesEvidence", {
  _planStepActionKind: load("_planStepActionKind", { _PLAN_STEP_KINDS: PLAN_STEP_KINDS }),
  _planStepTargets: planStepTargets,
  _toolTouchedTargets: toolTouchedTargets,
  _planTargetsConflict: targetsConflict,
});

test("步骤点了名的文件，动了别处就不打勾", () => {
  const step = { content: "实现 src/pages/login.tsx 的表单校验" };
  assert.equal(
    matches(step, ["implement"], { type: "edit", path: "src/pages/login.tsx" }), true,
    "改的就是这一步点名的文件，必须打勾",
  );
  assert.equal(
    matches(step, ["implement"], { type: "edit", path: "src/utils/date.ts" }), false,
    "改的是别的文件却打了勾 —— 进度条在走、活没干",
  );
  // 大小写和目录写法不该造成分歧：两边走同一套归一化。
  assert.equal(
    matches(step, ["implement"], { type: "edit", path: "src/Pages/Login.tsx" }), true,
  );
  // multi_edit 的目标在 edits[] 里，不在 path 上。
  assert.equal(
    matches(step, ["implement"], { type: "multiedit", edits: [{ path: "src/pages/login.tsx" }] }), true,
  );
  assert.equal(
    matches(step, ["implement"], { type: "multiedit", edits: [{ path: "README.md" }] }), false,
  );
});

test("两边任一没点名就不表态——这条判据只能拒绝勾，不能新增勾", () => {
  // 步骤没写具体文件：交回给类别判据，行为和加这条之前一模一样。
  const vague = { content: "实现登录页" };
  assert.equal(matches(vague, ["implement"], { type: "edit", path: "src/utils/date.ts" }), true);
  assert.equal(matches(vague, ["investigate"], { type: "edit", path: "src/utils/date.ts" }), false,
    "类别判据照旧管用");

  // 调用没有结构化目标（跑命令、开浏览器）：同样不表态。
  const named = { content: "实现 src/pages/login.tsx 的表单校验" };
  assert.equal(matches(named, ["implement"], { type: "cmd", command: "npm create vite@latest app" }), true);

  // 完全不传 call（旧调用点、或没有 call 的场景）行为不变。
  assert.equal(matches(named, ["implement"]), true);
});

test("目标抽取只收像路径/标识符的东西，不把版本号和中文说明当文件", () => {
  assert.deepEqual(planStepTargets({ content: "我在做第 2 版，顺手把 v1.2 的坑填了" }), []);
  assert.deepEqual(planStepTargets({ content: "改 package.json" }), ["package.json"]);
  assert.deepEqual(
    planStepTargets({ content: "把 `src/api/orders.ts` 里的分页改掉" }).sort(),
    ["orders.ts", "src", "api"].sort(),
  );
  // 反引号里是一整句中文说明时不收。
  assert.deepEqual(planStepTargets({ content: "按 `先读懂再动手` 来做" }), []);
});

test("交付物这一关排在类别判据之前——顺序反了它就形同虚设", () => {
  // 类别判据在分不出类时返回 false 并 return，交付物那一关要是排在它后面，
  // 「点了名、类别也对得上、但动的是别处」这条路根本走不到。
  const src = fnSource("_planStepMatchesEvidence", { code: true });
  const conflictAt = src.indexOf("_planTargetsConflict");
  const kindAt = src.indexOf("const kind = _planStepActionKind(step)");
  assert.ok(conflictAt > 0 && kindAt > 0, "两条判据都要在场");
  assert.ok(conflictAt < kindAt, "交付物判据必须排在动作类别判据之前");
});

test("自动推进的两个调用点都要把 call 传进去——漏一个，那条路上这道门不存在", () => {
  const advance = fnSource("_advancePlanFromTool", { code: true });
  const calls = [...advance.matchAll(/_planStepMatchesEvidence\(([^)]*)\)/g)].map((m) => m[1]);
  assert.ok(calls.length >= 2, `只找到 ${calls.length} 个调用点，这条守卫失去落点`);
  for (const args of calls) {
    assert.ok(/,\s*call\s*$/.test(args.trim()),
      `调用点没带 call：${args} —— 这条路上交付物判据整个不生效`);
  }
});
