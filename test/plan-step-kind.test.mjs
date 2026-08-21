// 计划步骤的性质：模型声明优先，动词表只是老计划的兜底。
//
// 这处的判断有真实后果——它决定一步会不会被某次工具结果**自动打勾**。以前只有一条路：
// 把步骤文字丢进几张中英动词表里猜。猜不出来不打勾（这条是对的，注释里记着一次真实的
// 「假完成」事故：7 步里 3 步没被动词表认出来，模型只是读了个文件，那三步就显示做完了）。
// 但猜**错**类别同样有代价：一步会被不相干的证据勾掉。
//
// 现在 update_plan 让模型逐步声明 kind。声明是模型写计划时顺手就有的事实，比事后猜准。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
const GATEWAY = readFileSync(join(HERE, "..", "..", "server", "prompts", "tools.json"), "utf8");

function extractFn(name) {
  const i = SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `main.js 里找不到 ${name}`);
  let depth = 0;
  let j = SRC.indexOf("{", SRC.indexOf(")", i));
  for (; j < SRC.length; j++) {
    const c = SRC[j];
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  return SRC.slice(i, j + 1);
}

/// 取一条 const 声明的原文（到匹配的收尾括号为止）。
/// 和 extractFn 同一条原则：跑真源码，不在测试里另写一份会漂的字面量。
function extractConstDecl(name) {
  const i = SRC.indexOf(`const ${name} = `);
  assert.ok(i >= 0, `main.js 里找不到 const ${name}`);
  let j = SRC.indexOf("=", i) + 1, depth = 0;
  for (; j < SRC.length; j++) {
    const c = SRC[j];
    if (c === "(" || c === "[" || c === "{") depth++;
    else if (c === ")" || c === "]" || c === "}") depth--;
    else if (c === ";" && depth <= 0) break;
  }
  return SRC.slice(i, j + 1);
}

const KINDS = new Set(["investigate", "implement", "execute", "verify"]);
const kindOf = new Function(
  "_PLAN_STEP_KINDS",
  `${extractFn("_planStepActionKind")}\n;return _planStepActionKind;`,
)(KINDS);
// _normPlanSteps 取步骤文字时走 _planStepText（它带两张常量表）。少注入一个，
// 归一化会当场抛 ReferenceError —— 而那正是「模型照 schema 填的参数让这一步崩掉」。
const norm = new Function(
  "_PLAN_STEP_KINDS",
  [
    extractConstDecl("_PLAN_STEP_TEXT_KEYS"),
    extractConstDecl("_PLAN_STEP_META_KEYS"),
    extractFn("_planStepText"),
    extractFn("_normPlanSteps"),
  ].join("\n") + "\n;return _normPlanSteps;",
)(KINDS);

test("模型声明了 kind 就用声明的，不再去猜措辞", () => {
  // 这一步的文字里全是「测试」类动词，但模型声明它是 implement——以声明为准。
  // 猜测和声明冲突时选猜测，等于告诉模型「你说了不算」。
  assert.equal(kindOf({ content: "补充测试并验证构建通过", kind: "implement" }), "implement");
  assert.equal(kindOf({ content: "随便写点什么", kind: "verify" }), "verify");
  for (const k of KINDS) assert.equal(kindOf({ content: "x", kind: k }), k);
});

test("没声明的老计划照旧靠动词表兜底，行为不变", () => {
  assert.equal(kindOf({ content: "运行测试并检查退出码" }), "verify");
  assert.equal(kindOf({ content: "实现登录接口" }), "implement");
  // 认不出来仍然返回空 —— 这条不能动：分不出类就不打勾，否则一次 read_file 就能
  // 把三步勾掉，那就是「假完成」。
  assert.equal(kindOf({ content: "面包店门店信息与地图" }), "");
});

test("用户实拍那份 9 步计划，每一步都要判对类型", () => {
  // 判错是双向的伤：该勾的勾不上（进度条永远难看），不该勾的可能被勾掉（假完成）。
  // 两边都会让计划失去可信度，然后模型和用户一起不再当真——用户原话：
  // 「任务规划内容是非常准确的，但是我的不根据任务规划去走」。
  const cases = [
    // 英文动词表原来没有词边界，`edit` 直接匹进 "Editor"，这一步被判成 implement。
    ["调研 Monaco Editor + Electron + React 集成方案与最佳实践", "investigate"],
    ["设计架构：确定 Electron 主进程/渲染进程通信、文件系统抽象层、状态管理方案", "implement"],
    // "搭建"是实现，可后半句的"构建"属于 verify 表，而 verify 原来排在前面先被测试，
    // 于是后半句抢走了整步的定性。
    ["搭建 Electron + React + Vite 项目脚手架与构建配置", "implement"],
    ["实现 Electron 主进程：窗口创建、菜单、文件操作 IPC 通信", "implement"],
    ["集成 Monaco Editor：编辑器实例、语法高亮、主题配置", "implement"],
    ["实现文件管理：文件树组件、打开/保存/新建文件、多 tab 管理", "implement"],
  ];
  for (const [content, want] of cases) {
    assert.equal(kindOf({ content }), want, `「${content.slice(0, 24)}」判成了 ${kindOf({ content }) || "空"}`);
  }
});

test("中文里最常见的调研类动词不能一个都不在表里", () => {
  // 原表只有"调查"，而计划第一步最常用的是"调研"，其次是研究/探索/对比/选型/评估。
  // 分不出类的步骤永远勾不上，进度条永远难看，模型很可能因此索性放弃维护计划。
  for (const verb of ["调研", "研究", "探索", "对比", "选型", "评估", "摸底"]) {
    assert.equal(kindOf({ content: `${verb}一下现有方案` }), "investigate", `${verb} 分不出类`);
  }
});

test("重叠时倒向更严的那一类，不许倒向更松的", () => {
  // execute 是四类里最宽松的（同时接受 execute 和 verify 证据），verify 只认 verify。
  // 「运行测试」按位置定性会得到 execute，那样一条 npm install 就能把它勾掉——假完成。
  assert.equal(kindOf({ content: "运行测试并检查退出码" }), "verify");
  assert.equal(kindOf({ content: "跑一遍测试" }), "verify");
  // 但主动词是实现时不受影响：implement 不比 verify 松，不该被覆盖。
  assert.equal(kindOf({ content: "搭建脚手架与构建配置" }), "implement");
});

test("乱写的 kind 当作没声明，退回猜测而不是让整步失效", () => {
  assert.equal(kindOf({ content: "运行测试", kind: "不存在的类别" }), "verify");
  assert.equal(kindOf({ content: "运行测试", kind: "" }), "verify");
  assert.equal(kindOf({ content: "运行测试", kind: 42 }), "verify");
});

test("归一化要把声明留住，否则声明走不到判定那一步", () => {
  // 这半边最容易悄悄断：schema 加了、模型也填了，但归一化把字段丢掉，于是永远走兜底，
  // 而且一点报错都没有。
  const [a, b, c] = norm([
    { content: "查一下现有结构", status: "pending", kind: "investigate" },
    { content: "写代码", status: "doing", kind: "IMPLEMENT" },
    { content: "没声明的一步", status: "pending", kind: "瞎写" },
  ]);
  assert.equal(a.kind, "investigate");
  assert.equal(b.kind, "implement", "大小写不该影响声明");
  assert.equal(b.status, "in_progress", "原有的状态归一化不受影响");
  assert.equal("kind" in c, false, "非法值不能留下来，留下来就成了永远匹配不上的类别");
  // 纯字符串步骤（最老的写法）仍然能用。
  assert.deepEqual(norm(["直接写一句"]), [{ content: "直接写一句", status: "pending" }]);
});

test("两份工具目录都要有 kind，否则正式构建走网关时等于没加", () => {
  // 工具描述有两份，运行时网关那份说了算。只改本地的话 dev 正常、装出来的包里这个
  // 字段根本不存在，模型永远不会填。
  assert.match(SRC, /kind: \{ type: "string", enum: \["investigate", "implement", "execute", "verify"\]/,
    "本地目录里 update_plan 没有 kind");
  // 钉**结构**不钉文本：同步脚本会重排 JSON 的空格，按字面量比对会假红一次
  // （这条就这么红过），而真正要保证的是"这个字段带着正确的枚举值在网关那份里"。
  const gatewayKind = JSON.parse(GATEWAY)
    .find((t) => t?.function?.name === "update_plan")
    ?.function?.parameters?.properties?.steps?.items?.properties?.kind;
  assert.ok(gatewayKind, "网关目录里 update_plan 没有 kind —— 正式构建下这个声明通道是死的");
  assert.deepEqual(gatewayKind.enum, ["investigate", "implement", "execute", "verify"],
    "网关那份的 kind 枚举和本地对不上");
});
