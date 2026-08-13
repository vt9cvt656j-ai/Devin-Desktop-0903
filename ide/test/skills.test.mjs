// Skills：让模型知道有哪些技能，并且能按需读正文。
//
// 症结不是"技能没实现"，是**模型看不见它们**。在此之前技能只有一条路进上下文：用户在面板里
// 手动打开开关，然后那个技能的正文被塞进系统提示词。没打开的技能，模型从头到尾不知道存在。
//
// 拿这台机器的真实情况量过：~/.cursor/skills 下 11 个技能，
//   · 目录（每个技能一行 name + description）= 3108 字符 ≈ 970 token
//   · 全部正文加起来          = 105247 字符 ≈ 33k token
// 差 34 倍。所以目录常驻、正文按需——这正是 Anthropic Agent Skills 的渐进式披露。
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const SRC = fs.readFileSync("src/main.js", "utf8");

function extractFn(name) {
  const m = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).exec(SRC);
  if (!m) throw new Error(`function ${name} not found`);
  let i = SRC.indexOf("{", SRC.indexOf(")", m.index)), depth = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i], d = SRC[i + 1];
    if (c === "/" && d === "/") { i = SRC.indexOf("\n", i); if (i < 0) i = SRC.length; continue; }
    if (c === "/" && d === "*") { i = SRC.indexOf("*/", i + 2) + 1; continue; }
    if (c === "'" || c === '"' || c === "`") {
      const q = c;
      for (i++; i < SRC.length; i++) { if (SRC[i] === "\\") { i++; continue; } if (SRC[i] === q) break; }
      continue;
    }
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (depth === 0) return SRC.slice(m.index, i + 1); }
  }
  throw new Error(`unbalanced braces in ${name}`);
}
function load(name, deps = {}) {
  const keys = Object.keys(deps);
  return new Function(...keys, `${extractFn(name)}\n;return ${name};`)(...keys.map((k) => deps[k]));
}

const SKILLS = [
  { id: "file:/s/a/SKILL.md", name: "ui-ux-pro-max", desc: "UI/UX design intelligence: styles, palettes, font pairings.", prompt: "A".repeat(44_000) },
  { id: "file:/s/b/SKILL.md", name: "systematic-debugging", desc: "Root-cause a failure instead of guessing.", prompt: "B".repeat(9_000) },
  { id: "file:/s/c/SKILL.md", name: "writing-plans", desc: "", prompt: "C".repeat(500) },
];

const catalog = (active = []) => load("_skillCatalogBlock", {
  _loadSkillsLocal: () => [],
  _fileSkills: SKILLS,
  _isSkillActive: (s) => new Set(active).has(typeof s === "string" ? s : s?.id),
})();

test("模型不用任何手动操作就能看到全部技能——这是缺的那一半", () => {
  const out = catalog();
  for (const s of SKILLS) assert.ok(out.includes(s.name), `目录里少了 ${s.name}`);
  assert.match(out, /read_skill/, "要告诉模型正文怎么取");
});

test("目录只放 name + description，不放正文——正文贵 34 倍", () => {
  const out = catalog();
  assert.ok(!out.includes("A".repeat(200)), "正文不该出现在目录里");
  assert.ok(out.length < 1200, `目录不该这么大：${out.length}`);
  assert.match(out, /UI\/UX design intelligence/);
});

test("已启用的技能在目录里标出来，免得模型再去读一遍它已经拿到的东西", () => {
  const out = catalog(["file:/s/b/SKILL.md"]);
  assert.match(out, /systematic-debugging（已启用）/);
  assert.ok(!/ui-ux-pro-max（已启用）/.test(out), "没启用的不该带这个标记");
});

test("没写 description 的技能也列出来，不能因为缺字段就从清单里消失", () => {
  assert.match(catalog(), /writing-plans：（没写说明）/);
});

test("一个技能都没有时目录是空串，不往提示词里塞一个空标题", () => {
  assert.equal(load("_skillCatalogBlock", { _loadSkillsLocal: () => [], _fileSkills: [], _isSkillActive: () => false })(), "");
});

test("技能特别多时目录有上限，并说清楚少列了几个", () => {
  const many = Array.from({ length: 200 }, (_, i) => ({
    id: `file:/s/${i}`, name: `skill-${i}`, desc: "X".repeat(400), prompt: "p",
  }));
  const out = load("_skillCatalogBlock", { _loadSkillsLocal: () => [], _fileSkills: many, _isSkillActive: () => false })();
  assert.ok(out.length <= 6_400, `目录超预算：${out.length}`);
  assert.match(out, /另有 \d+ 个技能未列出/);
});

test("单条 description 再长也会被截住——有人写了 914 字符的", () => {
  const out = load("_skillCatalogBlock", {
    _loadSkillsLocal: () => [], _isSkillActive: () => false,
    _fileSkills: [{ id: "x", name: "n", desc: "Z".repeat(3000), prompt: "p" }],
  })();
  assert.ok(out.length < 700, `单条没截住：${out.length}`);
});

// ── read_skill：按需取正文 ────────────────────────────────────────────────

test("按名字找技能：大小写、空格、连字符、斜杠都不挑", () => {
  const find = load("_findSkillByName");
  const hit = (q) => find(SKILLS, q)?.name || null;
  assert.equal(hit("ui-ux-pro-max"), "ui-ux-pro-max");
  assert.equal(hit("UI UX Pro Max"), "ui-ux-pro-max", "模型多半会照着说话的习惯写");
  assert.equal(hit("UI/UX_Pro_Max"), "ui-ux-pro-max");
  assert.equal(hit("debugging"), "systematic-debugging", "记住半个名字也该找得到");
});

test("找不到就是找不到，不能随便糊一个给模型", () => {
  const find = load("_findSkillByName");
  assert.equal(find(SKILLS, "根本不存在的技能"), null);
  assert.equal(find(SKILLS, ""), null);
  assert.equal(find(SKILLS, "u"), null, "一个字母不该命中 ui-ux-pro-max，那样等于乱猜");
  assert.equal(find([], "ui-ux-pro-max"), null);
});

test("模型把工具名写错也认——别让它卡在一个纯命名问题上", () => {
  // 切到声明结束为止，别用固定字符数——这张表的每一行都塞了十几个别名，很长。
  const aliasStart = SRC.indexOf("const _TOOL_ALIASES");
  const aliases = SRC.slice(aliasStart, SRC.indexOf("\n};", aliasStart));
  for (const alias of ["skill", "load_skill", "use_skill", "get_skill", "open_skill", "readskill"]) {
    assert.ok(aliases.includes(alias + ': "read_skill"'), `缺别名 ${alias}`);
  }
  assert.match(SRC, /case "read_skill": return \{ type: "skill", name:/);
});

test("read_skill 是只读工具：不弹审批，只读模式里也能用", () => {
  const readOnly = SRC.slice(SRC.indexOf("const _READ_ONLY_TYPES"), SRC.indexOf("const _READ_ONLY_TYPES") + 600);
  assert.match(readOnly, /"skill"/, "skill 类型必须在只读白名单里，否则 explorer/plan 模式用不了");
  const readTools = SRC.slice(SRC.indexOf("const _READ_TOOLS ="), SRC.indexOf("const _READ_TOOLS =") + 600);
  assert.match(readTools, /"read_skill"/, "子智能体也该能照着团队规范做事");
});

test("客户端和网关两份工具目录都得有 read_skill——只改一边等于没改", () => {
  assert.match(SRC, /name: "read_skill"/);
  const gateway = JSON.parse(fs.readFileSync("../server/prompts/tools.json", "utf8"));
  assert.ok(gateway.some((t) => t?.function?.name === "read_skill"),
    "网关 prompts/tools.json 里没有 read_skill：L0 模式下工具描述以网关那份为准");
});

test("技能被截断时要指出完整内容怎么取，不能只说一句「已截断」", () => {
  const block = load("_activeSkillsBlock", {
    _activeSkillIds: new Set(["big"]),
    _fileSkills: [],
    _loadSkillsLocal: () => [{ id: "big", name: "ui-ux-pro-max", prompt: "A".repeat(44_000) }],
    parentDir: (p) => p,
  })();
  assert.match(block, /read_skill/);
  assert.match(block, /ui-ux-pro-max/);
});

test("组合块 = 目录 + 已启用全文，两个注入点用的都是它", () => {
  assert.match(SRC, /function _skillsSystemBlock\(\) \{\s*return _skillCatalogBlock\(\) \+ _activeSkillsBlock\(\);/);
  assert.match(SRC, /const skillsBlock = _agentLightTurn \? "" : _skillsSystemBlock\(\)/);
  assert.match(SRC, /run\?\.skillsBlock \?\? _skillsSystemBlock\(\)/);
});

test("冷启动首轮不会漏掉磁盘上的技能", () => {
  // 文件技能靠 _idleRun 异步发现。开完工作区立刻提问时它还没跑完，技能目录里就只剩
  // localStorage 那批——模型看不见的技能等于不存在，且没有任何报错。这条钉住首轮的等待。
  const send = extractFn("sendPrompt");
  const at = send.indexOf("const skillsBlock =");
  assert.ok(at > 0, "找不到技能块的构建点");
  const before = send.slice(0, at);
  assert.match(before, /_fileSkillsCacheKey/,
    "构建技能块之前必须判断磁盘技能是否已经发现过");
  assert.match(before, /await Promise\.race\(\[\s*\n?\s*_refreshFileSkills\(/,
    "首轮必须等一次目录扫描，且要带超时——不能无限期阻塞这一轮");
  assert.match(before, /setTimeout\(resolve, \d+\)/,
    "等待必须有上界");
});
