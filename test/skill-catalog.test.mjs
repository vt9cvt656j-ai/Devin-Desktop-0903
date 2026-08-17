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
    const name = line.slice(2).split("：")[0].replace("（常驻）", "");
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

test("常驻技能仍然带标记，且不重复给全文", () => {
  const out = catalogWith([skill("a", "ui-ux-pro-max", "设计")], new Set(["ui-ux-pro-max"]));
  assert.match(out, /ui-ux-pro-max（常驻）/);
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

test("技能：正文不被腰斩、描述不被提前砍死、allowed-tools 真的收窄", () => {
  const SRC = readFileSync(join(HERE, "../src/main.js"), "utf8");

  // ① 致命项：read_skill 的返回落在 8000 那一档时会被 _headTailModelText 从**中间**挖空，
  //    而官方技能的 SKILL.md 普遍 10–20KB —— 模型拿到头尾、丢掉中段的分步说明，
  //    同时工具卡片还显示「已全部读入」。两个互相矛盾的事实同时摆在用户和模型面前。
  const cap = SRC.slice(SRC.indexOf("function _toolMsgForModel"), SRC.indexOf("const rawMessage"));
  assert.match(cap, /_rt === "skill" \? 30000/,
    "技能正文又落回 8000 档，会被从中间挖空");

  // ② description 就是触发判据。官方技能的「什么时候该用我 / 什么时候别用我」写在
  //    400–900 字符处，解析期砍到 240 等于把触发条件本身切掉。
  const parse = SRC.slice(SRC.indexOf("id: `file:${normalizedPath}`"));
  assert.match(parse.slice(0, 1400), /desc: desc\.replace\(\/\\s\+\/g, " "\)\.trim\(\)\.slice\(0, 1200\)/,
    "描述又在解析期被砍短了——目录那边自有 6000 字预算和逐级压缩，这里不该提前钉死上限");

  // ③ allowed-tools 必须是真约束。原来它唯一的消费点是工具卡片上一枚灰色标签：
  //    一个写着 Read, Grep 的只读技能，启用后模型照样能 write_file、删文件。
  assert.match(SRC, /function _skillAllowedTools\(\)/, "allowed-tools 的收窄逻辑没了");
  const gate = SRC.slice(SRC.indexOf("async function _approveToolCall"));
  // 钉住**白名单的来源**，不只是那行条件文本：把 skillGate 直接改成 null，条件那行还在，
  // 只钉文本的话照样绿。
  assert.match(gate.slice(0, 1400), /const skillGate = typeof _skillAllowedTools === "function" \? _skillAllowedTools\(\) : null;/,
    "审批闸没接技能白名单，allowed-tools 又变回纯装饰");
  assert.match(gate.slice(0, 1600), /!_skillToolAllowed\(skillGate\.allow,/, "闸没真的去查白名单");
  // 取并集不取交集：启用两个技能时两边的工具都该可用，交集会让"启用越多能干越少"。
  const allow = SRC.slice(SRC.indexOf("function _skillAllowedTools"));
  assert.match(allow.slice(0, 900), /for \(const s of declaring\) for \(const t of s\.tools\)/,
    "白名单不是并集");
  assert.match(allow.slice(0, 900), /allow\.add\("read_skill"\)/,
    "read_skill 没被永久放行——提示词要求模型用它读正文，挡掉等于技能自锁");
  // 没声明 allowed-tools 的技能不表态，不该缩小任何范围。
  assert.match(allow.slice(0, 900), /if \(!declaring\.length\) return null;/,
    "没有声明的技能也参与收窄了");

  // ④ 只扫自有目录。这一条以前是反的（断言"要扫到 Claude Code 的插件市场"）——
  //    按用户要求，别的工具的目录一个都不扫了。
  const bases = SRC.slice(SRC.indexOf("function _skillDiscoveryBases"), SRC.indexOf("function _skillDiscoveryBases") + 700);
  assert.match(bases, /\$\{root\}\/\.claude\/skills/, "工作区的 .claude/skills 是安装落点，必须扫");
  assert.match(bases, /\/\.mrdayone\/skills/, "家目录那份自有技能库没扫");
  for (const foreign of ["plugins", ".cursor", ".codex", ".agents"]) {
    assert.ok(!bases.includes(foreign), `还在扫别的工具的目录：${foreign}`);
  }

  // ⑤ 往输入框填文本的那条路径是提示词模板语义，和 Agent Skills 是两回事，已删。
  assert.doesNotMatch(SRC, /function _useSkill\(/,
    "那条把技能正文填进输入框的死代码又回来了——它和技能是两种语义");
});

const parseSkill = new Function(
  "parentDir",
  topLevelFn("_parseSkillDocument") + "\n;return _parseSkillDocument;",
)((path) => String(path).slice(0, String(path).lastIndexOf("/")));

test("技能正文进模型时剥掉 frontmatter，且开关的名字要说实话", () => {
  // ① frontmatter 不该进模型。name / description / allowed-tools 这三样解析期已经取走，
  //    description 更是早就在技能清单里；把它们原样留在正文里，等于同一段元数据在上下文
  //    出现三遍，占的还是最贵的那块预算（常驻 10k、read_skill 24k）。
  const doc = parseSkill(
    ["---", "name: docx", "description: 处理 Word 文档", "allowed-tools: Read, Grep", "---", "", "# 正文", "第一步：先读模板。"].join("\n"),
    "/w/.claude/skills/docx/SKILL.md",
  );
  assert.equal(doc.name, "docx");
  assert.equal(doc.desc, "处理 Word 文档");
  assert.deepEqual(doc.tools, ["Read", "Grep"]);
  assert.ok(doc.prompt.startsWith("# 正文"), `正文没剥干净：${JSON.stringify(doc.prompt.slice(0, 60))}`);
  for (const leaked of ["allowed-tools", "description:", "---"]) {
    assert.ok(!doc.prompt.includes(leaked), `${leaked} 还留在给模型的正文里：${doc.prompt}`);
  }
  // 但整份文件只有 frontmatter 时不能剥成空串——那会让这个技能从清单里凭空消失，
  // 比多几行元数据糟得多。
  const bare = parseSkill(["---", "name: 空的", "description: 没有正文", "---"].join("\n"), "/w/.claude/skills/bare/SKILL.md");
  assert.ok(bare.prompt.trim().length > 0, "剥成空串了，这个技能会从清单里消失");

  // ② 开关文案必须说实话。这几条打的是设置面板那一页（renderSkillsTool）——
  //    上面那个弹窗面板（openSkillsPanel）全仓零调用点，已经删掉了；在它身上改文案
  //    改了两次都没生效，因为用户根本点不到它。
  assert.doesNotMatch(SRC, /已启用技能：/, "旧文案还在：那句话说的不是这个开关做的事");
  assert.doesNotMatch(SRC, /会注入到对话/, "旧文案还在");
  assert.doesNotMatch(SRC, /如果只是不想让它生效，关掉开关就行/,
    "删除确认里那句话是假的：取消常驻之后模型照样看得见、照样能读");
  assert.doesNotMatch(SRC, /async function openSkillsPanel/,
    "那个零调用点的重复面板又回来了——它是「改完了却没生效」的来源");
});

test("「常驻」这个词在四个地方必须是同一个词——它们互相引用", () => {
  /*
   * 这不是文案洁癖，是一条真的交叉引用：
   *
   *   ① 技能清单给每个常驻技能打一个标记 `（常驻）`
   *   ② read_skill 的工具描述里写着「标着 X 的正文已经在系统提示词里，别重读」
   *   ③ 同一份描述在网关的 prompts/tools.json 里还有一份，而**运行时是网关那份说了算**
   *   ④ 界面上给用户看的也是同一个词
   *
   * ① 和 ② 用词不一致，模型就对不上号：它照着描述去找「已启用」，清单里写的是
   * 「常驻」，于是那条规则整条落空——每轮都把已经在上下文里的全文再 read_skill 读一遍。
   * ② 和 ③ 不一致更隐蔽：改了源码、跑起来没变化，因为模型读的根本是另一份。
   */
  const catalogMark = /_isSkillActive\(s\) \? "（常驻）"/;
  assert.match(SRC, catalogMark, "技能清单里的常驻标记被改了");
  assert.match(SRC, /标着「常驻」的/, "清单抬头对这个标记的解释和标记本身对不上");

  const readSkillDesc = SRC.slice(SRC.indexOf('name: "read_skill"'), SRC.indexOf('name: "read_skill"') + 1200);
  assert.match(readSkillDesc, /already marked 常驻/,
    "read_skill 的描述引用的标记和清单里打的不是同一个词——那条「别重读」的规则会整条落空");

  const gateway = JSON.parse(readFileSync(join(HERE, "..", "..", "server", "prompts", "tools.json"), "utf8"));
  const gwDesc = gateway.find((t) => t?.function?.name === "read_skill")?.function?.description || "";
  assert.ok(gwDesc, "网关目录里没有 read_skill");
  assert.match(gwDesc, /already marked 常驻/,
    "网关那份还写着旧词——运行时用的是它，改源码等于没改");
});

test("界面上不再有任何「启用 / 未启用」的说法", () => {
  // 技能从来没被"关"过：清单里的名称和描述始终在上下文里，模型随时能 read_skill 读它。
  // 「未启用」尤其糟——它是常驻显示在卡片上的**状态标签**，用户读到的是"这个技能是
  // 关着的"，于是他以为点一下就能停掉某个技能，实际什么都没停。
  const skillsPage = SRC.slice(SRC.indexOf('<h3>Skills 技能</h3>'), SRC.indexOf('<h3>Skills 技能</h3>') + 40000);
  for (const stale of ['"未启用"', '"已启用"', '保存并启用', '默认启用到模型请求里']) {
    assert.ok(!skillsPage.includes(stale), `设置面板的 Skills 页还留着「${stale}」`);
  }
  assert.match(skillsPage, /on \? "常驻" : "按需"/, "状态标签没改成说实话的那个");
  assert.match(skillsPage, /on \? "已常驻" : "设为常驻"/, "切换按钮还在说「启用」");
});
