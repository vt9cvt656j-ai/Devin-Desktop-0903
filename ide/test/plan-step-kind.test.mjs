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

const KINDS = new Set(["investigate", "implement", "execute", "verify"]);
const kindOf = new Function(
  "_PLAN_STEP_KINDS",
  `${extractFn("_planStepActionKind")}\n;return _planStepActionKind;`,
)(KINDS);
const norm = new Function(
  "_PLAN_STEP_KINDS",
  `${extractFn("_normPlanSteps")}\n;return _normPlanSteps;`,
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
  assert.ok(GATEWAY.includes('"kind": {"type": "string", "enum": ["investigate", "implement", "execute", "verify"]'),
    "网关目录里 update_plan 没有 kind —— 正式构建下这个声明通道是死的");
});
