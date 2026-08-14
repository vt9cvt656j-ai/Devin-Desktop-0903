// 技能清单：被丢掉的那几个技能，对模型来说等于根本不存在。
//
// 两个各自独立、叠在一起就要人命的毛病：
//   1. 按 skill.id 去重 —— 同一个技能名会从多个来源各来一份（家目录、工作区、
//      ~/.codex/plugins/cache），id 不同所以全都活下来。可模型只能**按名字**点技能
//      （read_skill → _findSkillByName 返回第一个匹配），重复那几条从一开始就够不着，
//      却照样占着清单的字数预算。
//   2. 装不下时整条丢技能 —— 一条条按 400 字满额往里塞，塞爆就把剩下的全砍掉。
//      代价完全不对等：说明短一点只是命中率降低，整条不见是这个能力对模型彻底不存在，
//      而且被丢的是谁只取决于磁盘扫描顺序。
//
// 结果就是用户那句「有时候它记得用技能，有时候完全不认识」。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

function topLevelFn(name) {
  const at = SRC.indexOf(`function ${name}(`);
  assert.ok(at > 0, `找不到 ${name}`);
  const end = SRC.indexOf("\n}\n", at);
  assert.ok(end > at, `${name} 没有行首收尾大括号`);
  return SRC.slice(at, end + 2);
}

// _skillCatalogBlock 读的是模块级的 _fileSkills / _loadSkillsLocal / _isSkillActive，
// 全部注入成桩，这样测的就是真实那段代码本身。
function catalogWith(skills, active = new Set()) {
  const fn = new Function(
    "SKILLS", "ACTIVE",
    "const _fileSkills = SKILLS;" +
    "const _loadSkillsLocal = () => [];" +
    "const _isSkillActive = (s) => ACTIVE.has(s.name);" +
    topLevelFn("_skillCatalogBlock") +
    "\n;return _skillCatalogBlock();",
  );
  return fn(skills, active);
}

const findByName = new Function(topLevelFn("_findSkillByName") + "\n;return _findSkillByName;")();

const skill = (id, name, desc) => ({ id, name, desc, prompt: "x" });

test("同名技能只占一个名额——重复那份本来就够不着", () => {
  // kc-autopilot 在这台机器上真的来了两份（~/.codex/plugins/cache 一份、工作区一份）。
  const out = catalogWith([
    skill("a", "kc-autopilot", "自动驾驶"),
    skill("b", "kc-autopilot", "同名的另一份"),
    skill("c", "writing-plans", "写实现计划"),
  ]);
  assert.equal((out.match(/kc-autopilot/g) || []).length, 1, "同名技能被列了不止一次，白占预算");
  assert.match(out, /writing-plans/, "重复项挤掉了别的技能");
});

test("去重用的归一化和 read_skill 查名字的完全一致", () => {
  // 两边不一致的话，会出现「清单里列着、read_skill 却找不到」或者反过来。
  const out = catalogWith([
    skill("a", "UI/UX Pro Max", "设计"),
    skill("b", "ui-ux-pro-max", "同一个技能的另一种写法"),
  ]);
  assert.equal((out.match(/^- /gm) || []).length, 1, "只是大小写/分隔符不同，应视为同一个技能");
});

test("列出来的每一个，都必须能被 read_skill 按名字取到", () => {
  const all = [
    skill("a", "systematic-debugging", "查 bug"),
    skill("b", "frontend-design", "做界面"),
    skill("c", "test-driven-development", "先写测试"),
  ];
  const out = catalogWith(all);
  for (const line of out.split("\n").filter((l) => l.startsWith("- "))) {
    const name = line.slice(2).split("：")[0].replace("（已启用）", "");
    assert.ok(findByName(all, name), `清单里有「${name}」，read_skill 却找不到它`);
  }
});

test("说明会先被压短，而不是直接把技能整条丢掉", () => {
  // 28 个技能 × 400 字说明 = 11k 字符，远超 6000 —— 老写法会静默丢掉后面几个，
  // 丢谁完全看磁盘扫描顺序。
  const many = [];
  for (let i = 0; i < 28; i++) {
    many.push(skill(`id${i}`, `skill-number-${i}`, "说".repeat(400)));
  }
  const out = catalogWith(many);
  assert.ok(out.length <= 6000, `超出 6000 字符上限：${out.length}`);
  for (let i = 0; i < 28; i++) {
    assert.match(out, new RegExp(`skill-number-${i}(?![0-9])`),
      `skill-number-${i} 没进清单——这个能力对模型等于不存在`);
  }
});

test("技能实在太多、压到最短也装不下时，要记账并给出补救路径", () => {
  const many = [];
  for (let i = 0; i < 400; i++) many.push(skill(`id${i}`, `a-very-long-skill-name-${i}`, "说明说明说明"));
  const out = catalogWith(many);
  assert.ok(out.length <= 6000);
  assert.match(out, /未列出/, "截断了却不记账，模型会把残缺清单当成全部");
  assert.match(out, /read_skill/,
    "只说「装不下」等于宣判那些技能不存在；要告诉模型 read_skill 是按名字查的，不受清单长度限制");
});

test("没有技能时返回空串，不占提示词", () => {
  assert.equal(catalogWith([]), "");
});

test("已启用的技能仍然带标记，且不重复给全文", () => {
  const out = catalogWith([skill("a", "ui-ux-pro-max", "设计")], new Set(["ui-ux-pro-max"]));
  assert.match(out, /ui-ux-pro-max（已启用）/);
});

test("输出稳定：同样的输入两次调用逐字节相同", () => {
  const all = [skill("a", "one", "第一"), skill("b", "two", "第二")];
  assert.equal(catalogWith(all), catalogWith(all));
});

test("不再存在「按满额说明一条条塞、塞爆就 break」的老写法", () => {
  const body = topLevelFn("_skillCatalogBlock");
  assert.doesNotMatch(body, /const PER_DESC = 400;[\s\S]*for \(const s of all\)/,
    "还是老的单遍满额填充——装不下就丢整条技能");
  assert.match(body, /\[400, 240, 140, 90, 60\]/, "缺少「先压说明再丢技能」的逐级收窄");
  assert.match(body, /byName/, "还是按 id 去重，同名技能会重复占位");
});
