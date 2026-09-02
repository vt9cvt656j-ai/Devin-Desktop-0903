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
// 2026-08-26 搬进了 src/agent/skill-doc.js —— 直接 import 真模块，不再抠源码
// 注入 parentDir（这个函数的外部依赖实测为零，那个注入早就是陈迹）。
import { parseSkillDocument as _parseSkillDoc } from "../src/agent/skill-doc.js";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
// 正向源码断言必须跑在**剥掉注释**的源码上。注释不是代码：把一条契约从代码里删掉、
// 只在注释里留一句，assert.match 照样绿——本仓库已经这样漏过一整组模型可见的工具契约。
// 所以 `SRC` 绑定的是 CODE（注释整段置空，行号与偏移和原文一字不差）；
// 真要匹配注释本身的断言显式用 RAW_SRC，并在那一行写清为什么。
import { CODE as SRC, SRC as RAW_SRC, fnSource as topLevelFn, blockFrom, load} from "./helpers/source.mjs";

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
  // 这里原来局部 `const SRC = readFileSync(main.js)`，把外层共享的 SRC 遮蔽掉了——
  // 下面几行注释记着它已经害出过一次「偏移混用」的 bug。遮蔽去掉，用共享那份。

  // ① 致命项：read_skill 的返回落在 8000 那一档时会被 _headTailModelText 从**中间**挖空，
  //    而官方技能的 SKILL.md 普遍 10–20KB —— 模型拿到头尾、丢掉中段的分步说明，
  //    同时工具卡片还显示「已全部读入」。两个互相矛盾的事实同时摆在用户和模型面前。
  const cap = SRC.slice(RAW_SRC.indexOf("function _toolMsgForModel"), RAW_SRC.indexOf("const rawMessage"));
  assert.match(cap, /_rt === "skill" \? 30000/,
    "技能正文又落回 8000 档，会被从中间挖空");

  // ② description 就是触发判据。官方技能的「什么时候该用我 / 什么时候别用我」写在
  //    400–900 字符处，解析期砍到 240 等于把触发条件本身切掉。
  // 2026-08-26：解析器搬进了 src/agent/skill-doc.js，这条从「切源码」改成**验行为**。
  //
  // 原来是 `SRC.slice(RAW_SRC.indexOf(锚点))`，而本函数第 128 行把 SRC 遮蔽成了 main.js
  // 的原文、indexOf 却用拼接后的 RAW_SRC —— 两套偏移混用。符号还在 main.js 里时恰好
  // 重合（main.js 排在拼接最前），搬走之后偏移落到 main.js 之外，切出来是别的地方。
  // 直接喂一份长描述进真解析器，比对着源码正则可靠得多。
  const longDesc = "触发条件".repeat(200); // 800 字符，远超旧的 240 上限
  const parsed = _parseSkillDoc(
    `---\nname: probe\ndescription: ${longDesc}\n---\n\n正文`,
    "/w/.mrdayone/skills/probe/SKILL.md",
  );
  assert.ok(parsed && parsed.desc, "解析器没吐出描述");
  assert.ok(parsed.desc.length > 700,
    `描述在解析期被砍到 ${parsed.desc.length} 字符——目录那边自有 6000 字预算和逐级压缩，`
    + "这里不该提前钉死上限（官方技能的「什么时候该用我」就写在 400–900 字符处）");

  // ③ allowed-tools 必须是真约束。原来它唯一的消费点是工具卡片上一枚灰色标签：
  //    一个写着 Read, Grep 的只读技能，启用后模型照样能 write_file、删文件。
  assert.match(SRC, /function _skillAllowedTools\(\)/, "allowed-tools 的收窄逻辑没了");
  // 区间原来是 `SRC.slice(RAW_SRC.indexOf("async function _approveToolCall"))` 再手工
  // `.slice(0, 1400)` / `.slice(0, 3200)`。两头都不成立：
  //   · 1400 / 3200 跟代码结构没有任何关系。_approveToolCall 实测 5053 字，两个目标分别
  //     落在偏移 318 和 1303；谁在它们前面补上千把字（代码或**注释**都算——CODE 把注释
  //     换成等长空格，长度照旧）就把目标顶出窗口；反过来函数一旦变短，窗口会溢出右花括号
  //     去看后面那个 _userDeniedToolResult，于是断言匹配到的可能根本不是这道闸。
  //   · 更要命的是这两条只验「这行字还在吗」。实测把 `.some(` 换成 `.every(`
  //     （同一次调用的几个名字要**全部**在白名单里才放行，run_cmd 立刻被自己声明它的
  //     技能拒掉），这两行字一个都没动，两条断言全绿。
  // 所以：源码断言只留「接线还在」这一层、区间按 AST 取整函数；判断本身改成真跑一遍。
  const gate = topLevelFn("_approveToolCall", { code: true });
  // 钉住**白名单的来源**，不只是那行条件文本：把 skillGate 直接改成 null，条件那行还在，
  // 只钉文本的话照样绿。
  assert.match(gate, /const skillGate = typeof _skillAllowedTools === "function" \? _skillAllowedTools\(\) : null;/,
    "审批闸没接技能白名单，allowed-tools 又变回纯装饰");
  // 形状变了（改成把这次调用的几个名字都试一遍，见本文件末尾那条行为测试），但意思不变：
  // 闸必须真的去查白名单。
  assert.match(gate, /_skillToolAllowed\(skillGate\.allow, n\)/, "闸没真的去查白名单");
  // 真跑这道闸。技能白名单排在权限规则**之前**，判定发生在任何 await 之前，所以
  // _noteRefusal 是同步被调到的——拿它当出口，这条用例就不用改成 async。
  // 闸后面的路径与本条无关：_permissionRuleVerdict 短路成 "allow"，于是 true/false
  // 唯一的分歧点就是技能白名单这一关。
  const _skillToolAllowedReal = load("_skillToolAllowed", ["_SKILL_TOOL_ALIASES", "_skillToolAllowed"]);
  const gateRefusals = (allowNames, call) => {
    const seen = [];
    const approve = load("_approveToolCall", {
      _callIsDestructive: () => false,
      _skillAllowedTools: () => (allowNames ? { allow: new Set(allowNames), names: ["只读技能"] } : null),
      _skillToolAllowed: _skillToolAllowedReal,
      showToast: () => {},
      _noteRefusal: (...a) => seen.push(a),
      _loadPermissionRules: async () => [],
      _permissionRuleVerdict: () => "allow",
      _permRuleSource: () => "",
    });
    approve(call, {}).catch(() => {}); // 闸之后是异步的，与本条无关，丢掉
    return seen;
  };
  assert.deepEqual(
    gateRefusals(["read"], { _toolName: "write_file", type: "write" }),
    [["skill", "只读技能", ["read"]]],
    "只声明了 read 的技能常驻着，write_file 却照样放行——allowed-tools 又变回纯装饰；"
    + "或者拒绝回执没把「这个技能到底允许什么」带给模型（它只能换个名字一轮轮试）");
  assert.deepEqual(
    gateRefusals(["run_cmd"], { _toolName: "run_cmd", type: "cmd", name: "某技能名" }),
    [],
    "技能自己声明的 run_cmd 被这道闸拒了——注册名/内部类型/call.name 三个名字里比对错了字段");
  assert.deepEqual(
    gateRefusals(["read", "read_skill"], { _toolName: "read_skill", type: "skill", name: "deploy-gateway" }),
    [],
    "read_skill 被拒——技能把自己锁死，模型连正文都读不到");
  assert.deepEqual(
    gateRefusals(null, { _toolName: "write_file", type: "write" }),
    [],
    "一个技能都没声明 allowed-tools 时这道闸也在拦——它只该在有人声明时收窄");
  // 取并集不取交集：启用两个技能时两边的工具都该可用，交集会让"启用越多能干越少"。
  //
  // 这三条原来是 `SRC.slice(RAW_SRC.indexOf("function _skillAllowedTools")).slice(0, 900)`
  // 上的文本匹配，两头都不成立：
  //   · 900 是拍脑袋的数，而 _skillAllowedTools 真身只有 765 字（fnSource 实测）——窗口
  //     比函数长 135 字，多出来的部分越过右花括号落到下一个声明上；另一头也没余量：
  //     `allow.add("read_skill")` 落在偏移 625，谁在它前面补 275 字（注释也算）就顶出去。
  //   · 更要命的是三条全是「这行字还在吗」。实测把筛选条件里的 `_activeSkillIds.has(s.id)`
  //     删掉（没常驻的技能也来贡献工具，白名单凭空变大、别人的技能反过来收窄了这一次调用），
  //     三行字一个都没动，三条断言全绿。
  // 这个函数的外部依赖只有三个模块级变量，能直接在 Node 里跑，所以整段换成真往返。
  const skillAllowed = (activeIds, skills) => load("_skillAllowedTools", {
    _activeSkillIds: new Set(activeIds),
    _loadSkillsLocal: () => [],
    _fileSkills: skills,
  })();
  const union = skillAllowed(["a", "b"], [
    { id: "a", name: "A", tools: ["Read"] },
    { id: "b", name: "B", tools: ["Bash"] },
    { id: "c", name: "C", tools: ["Write"] }, // 没常驻：一个字都不该进来
    { id: "d", name: "D" },                   // 常驻了但没声明：不表态
  ]);
  assert.deepEqual([...union.allow].sort(), ["bash", "read", "read_skill"],
    "白名单不是并集（交集会让「启用越多能干越少」）、或 read_skill 没被永久放行"
    + "（提示词要求模型用它读正文，挡掉等于技能自锁）、或没常驻的技能也进来收窄了");
  assert.deepEqual(union.names, ["A", "B"],
    "回执里的技能名不对——模型收到的拒绝理由会指错技能");
  // 没声明 allowed-tools 的技能不表态，不该缩小任何范围。
  assert.equal(skillAllowed(["d"], [{ id: "d", name: "D" }]), null,
    "没有声明的技能也参与收窄了");
  assert.equal(skillAllowed([], [{ id: "a", name: "A", tools: ["Read"] }]), null,
    "一个技能都没常驻时也去收窄了");

  // ④ 只扫**家目录技能库**这一处。这一条改过两次：最早断言"要扫到 Claude Code 的插件
  //    市场"，2026-08-18 按用户要求改成"只扫自己的目录（工作区 + 家目录）"，
  //    2026-08-22 再把工作区那条也删掉——技能是跨项目复用的能力，装进"当时打开的那个
  //    项目"意味着换个项目整批消失，那正是用户报的"装完无法使用"。
  const bases = topLevelFn("_skillDiscoveryBases", { code: true });
  assert.match(bases, /\$\{_STATE_DIR\}\/skills/, "家目录那份技能库没扫");
  assert.doesNotMatch(bases, /\$\{root\}/, "工作区那条发现路径又回来了");
  assert.doesNotMatch(bases, /_workspaceAncestorRoots/, "还在按工作区祖先展开");
  // .claude 现在也在这张黑名单里：技能只落自己的目录。
  for (const foreign of ["plugins", ".cursor", ".codex", ".agents", ".claude"]) {
    assert.ok(!bases.includes(foreign), `还在扫别的工具的目录：${foreign}`);
  }

  // ⑤ 往输入框填文本的那条路径是提示词模板语义，和 Agent Skills 是两回事，已删。
  assert.doesNotMatch(SRC, /function _useSkill\(/,
    "那条把技能正文填进输入框的死代码又回来了——它和技能是两种语义");
});

const parseSkill = _parseSkillDoc;

test("技能正文进模型时剥掉 frontmatter，且开关的名字要说实话", () => {
  // ① frontmatter 不该进模型。name / description / allowed-tools 这三样解析期已经取走，
  //    description 更是早就在技能清单里；把它们原样留在正文里，等于同一段元数据在上下文
  //    出现三遍，占的还是最贵的那块预算（常驻 10k、read_skill 24k）。
  const doc = parseSkill(
    ["---", "name: docx", "description: 处理 Word 文档", "allowed-tools: Read, Grep", "---", "", "# 正文", "第一步：先读模板。"].join("\n"),
    "/w/.mrdayone/skills/docx/SKILL.md",
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
  const bare = parseSkill(["---", "name: 空的", "description: 没有正文", "---"].join("\n"), "/w/.mrdayone/skills/bare/SKILL.md");
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

  const readSkillDesc = SRC.slice(RAW_SRC.indexOf('name: "read_skill"'), RAW_SRC.indexOf('name: "read_skill"') + 1200);
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
  // 区间原来是 `SRC.slice(indexOf('<h3>Skills 技能</h3>'), … + 40000)`。40000 是个跟代码
  // 结构毫无关系的数：Skills 页那个单元 renderSkillsTool 实测只有 18271 字、锚点落在它
  // 内部偏移 366，于是窗口末尾还多盖了 22095 字、十几个跟 Skills 页毫不相干的函数。
  // 两个方向都坏：反向的 !includes 会被隔壁函数的字眼打成假红；正向的两条 assert.match
  // 只要那串字在窗口内**任何地方**出现就绿——把状态标签从 Skills 页整个搬到后面某个函数里
  // （实测：页面上改成「在线 / 离线」、原字符串挪到窗口内的另一个函数），旧写法照样全绿。
  // 按 AST 取整个 renderSkillsTool：区间正好是它自己，它长多少都盖得住，也一个字不越界。
  const skillsPage = topLevelFn("renderSkillsTool", { code: true });
  // 只把这四个反向检查收进 renderSkillsTool 是**放松**：老窗口是「锚点后 40000 字」，
  // 覆盖到函数结束之后那 22095 字，而这条用例的标题说的是「界面上」，不是「这个函数里」。
  // 实测这个变异：在 renderSkillsTool **之后**（`const FEATURE_TABS = [` 前面）加一个
  // `function _skillEditorButtons() { return \`<button …>保存并启用</button>\`; }`
  // ——页面本体一个字没动，旧写法红、只钉 renderSkillsTool 的写法绿。
  //
  // 所以改成按「这个词在源码里合法出现几次」来钉，不再依赖任何窗口：
  // 这三个词在整份源码（CODE，注释已置空）里今天是 0 次，直接对全文断言，
  // 严格强于原来那 40000 字窗口，而且 helper 搬到哪儿都跑不掉。
  for (const stale of ['"未启用"', '保存并启用', '默认启用到模型请求里']) {
    assert.ok(!SRC.includes(stale), `界面上又出现了「${stale}」`);
  }
  // 「已启用」不能对全文断言：MCP 服务器那条分支里真的有开/关这回事，合法地出现 2 次
  //（实测：mcpconfig 分支 4796 字，两处都在里面，全文也正好 2 处）。钉的是「它只出现在
  // 那条分支里」——技能这边、或任何别的页面冒出第三处，这条立刻红。
  const mcpCfgBranch = blockFrom('} else if (call.type === "mcpconfig") {', { code: true });
  assert.equal(SRC.split('"已启用"').length - 1, mcpCfgBranch.split('"已启用"').length - 1,
    "「已启用」跑到 MCP 服务器那条分支之外去了——技能从来没有「关着」这个状态");
  assert.ok(!skillsPage.includes('"已启用"'), "设置面板的 Skills 页还留着「已启用」");
  assert.match(skillsPage, /on \? "常驻" : "按需"/, "状态标签没改成说实话的那个");
  assert.match(skillsPage, /on \? "已常驻" : "设为常驻"/, "切换按钮还在说「启用」");
});

// 上面第 ③ 条钉的是「闸接上了没有」——全是源码文本断言，所以下面这个 bug 一直是绿的：
// 闸接上了，比对的字段却是错的。这条真的把闸跑一遍。
test("allowed-tools 比对的必须是工具注册名，不是映射后的内部类型", () => {
  const aliases = /const _SKILL_TOOL_ALIASES = \{[\s\S]*?\n\};/.exec(SRC);
  const fn = /function _skillToolAllowed\(allow, toolName\) \{[\s\S]*?\n\}/.exec(SRC);
  assert.ok(aliases && fn, "别名表或 _skillToolAllowed 改名/挪走了，这条断言失去落点");
  const allowed = new Function(`${aliases[0]}\n${fn[0]}\nreturn _skillToolAllowed;`)();
  const A = (...names) => new Set(names);

  // 病根：run_cmd 映射完是 { type: "cmd" }，没有 name。闸原来取 call.name || call.tool || call.type，
  // 于是拿「cmd」去比白名单里的「run_cmd」——对不上，静默拒。
  assert.equal(allowed(A("run_cmd"), "run_cmd"), true, "技能声明了 run_cmd 却放不行 run_cmd");
  assert.equal(allowed(A("run_cmd"), "cmd"), false,
    "「cmd」不该被放行——它证明当年传错字段时这道闸会把技能自己声明的工具拒掉");

  // 别名表的值是**注册名**，所以传 type 进来时别名这条路同样断掉。
  assert.equal(allowed(A("bash"), "run_cmd"), true, "Claude Code 风格的 bash 没映到 run_cmd");
  assert.equal(allowed(A("bash"), "run_in_terminal"), true, "bash 要能覆盖 IDE 终端，否则长驻服务起不来");
  assert.equal(allowed(A("write"), "multi_edit"), true, "write 覆盖不到 multi_edit，多处改写会被拒");
  assert.equal(allowed(A("read"), "read_file"), true);

  // read_skill 的 call.name 是**技能名**，所以「永远放行」那句以前一次都没兑现。
  assert.equal(allowed(A("read", "read_skill"), "read_skill"), true, "技能把自己锁死了，模型读不到正文");

  // 闸没有变松：没声明的工具照样拒。
  assert.equal(allowed(A("read"), "run_cmd"), false, "allowed-tools 不再是真约束了");

  // 钉实现特征：闸必须拿注册名去比，且不许再用 call.name（那对 read_skill 是技能名）。
  const gate = topLevelFn("_approveToolCall", { code: true });
  assert.match(gate, /\[call\._toolName, call\.tool, call\.type\]\.filter\(Boolean\)/,
    "闸又改回按单一字段比对了——run_cmd/write_file 会被自己声明它们的技能拒掉");
  assert.doesNotMatch(gate, /_skillToolAllowed\(skillGate\.allow, call\.name/,
    "call.name 对 read_skill 是技能名，拿它当工具名会让技能自锁");
});

// save_skill 写出来的 SKILL.md，必须能被**真正的**技能解析器读回来。
// 拼装和解析是两处独立代码：拼错一个字段名（allowed_tools vs allowed-tools、缺空行），
// 文件照样写成功、模型照样跟用户说「已经存好了」，而下一轮它在清单里是一条没有描述、
// 甚至根本解析不出来的废条目。所以这条把两头接起来跑一遍。
test("save_skill 写出的 SKILL.md 能被真正的技能解析器读回来", () => {
  // 区间原来是开放式的 `SRC.slice(RAW_SRC.indexOf('} else if (call.type === "saveskill") {'))`
  // ——一路切到拼接源码的结尾（约 467 万字），下面那条正则于是只是「从 saveskill 分支的
  // 起点往后找第一个 const _doc = [」。锚点除了给一个起始偏移之外没有任何约束力：把拼装
  // 代码从 save_skill 分支里搬走、换成调用别处的辅助函数（实测：辅助函数追加在 main.js
  // 末尾，分支里只剩一句调用），正则照样在文件后半截捞到那份拼装，连 assert.ok(docSrc)
  // 这道兜底也照样绿——于是这条验的是「文件某处有一份拼得对的代码」，不是「save_skill
  // 写出来的那份」。blockFrom 按 AST 把区间闭合在这个分支的块上（实测 2468 字），
  // 锚点在全文出现 1 次、不唯一时它当场抛错；拼装一旦搬出这个分支就立刻红。
  const exec = blockFrom('} else if (call.type === "saveskill") {', { code: true });
  const docSrc = /const _doc = \[[\s\S]*?\.join\("\\n"\);/.exec(exec);
  assert.ok(docSrc, "SKILL.md 的拼装代码改形状了，这条断言失去落点");
  const build = new Function("_name", "_desc", "_fmTools", "_body", `${docSrc[0]}\nreturn _doc;`);

  const doc = build("deploy-gateway", "改完网关之后怎么发布", ["run_cmd", "read_file"], "1. 先跑测试\n2. 再部署");
  const skill = parseSkill(doc, "/w/.mrdayone/skills/deploy-gateway/SKILL.md");
  assert.equal(skill.name, "deploy-gateway", "name 没被解析出来——清单里会是一条无名条目");
  assert.equal(skill.desc, "改完网关之后怎么发布", "desc 是模型以后唯一的判断依据（解析器把它放在 desc 而不是 description）");
  assert.deepEqual(skill.tools, ["run_cmd", "read_file"], "allowed-tools 没解析出来");
  assert.match(skill.prompt, /先跑测试/, "正文丢了");
  assert.doesNotMatch(skill.prompt, /^---/, "frontmatter 又混进正文了");

  // 不给 allowed_tools 时**不许**凭空写一行：它是收窄语义，凭空加上会让这个技能常驻时
  // 把别的工具全挡掉（本文件上面那条闸的测试就是这个后果）。
  const bare = build("release-notes", "怎么写发布说明", [], "照模板填");
  assert.doesNotMatch(bare, /allowed-tools/, "没声明工具却写了 allowed-tools —— 等于凭空给技能加了一道收窄");
  const bareSkill = parseSkill(bare, "/w/.mrdayone/skills/release-notes/SKILL.md");
  assert.equal(bareSkill.name, "release-notes");
  // 解析器只在真有 allowed-tools 时才带 tools 键（`...(tools.length ? { tools } : {})`），
  // 所以"没声明"的正确形状是这个键根本不存在，而不是一个空数组。
  assert.equal(bareSkill.tools, undefined, "没声明却带出了工具约束键");
});
