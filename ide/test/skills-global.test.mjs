// 技能是**跨项目复用的能力**，只存在一个地方：`~/.mrdayone/skills/<名字>/SKILL.md`。
//
// 用户报的病：「装完无法使用」。~/.mrdayone/skills 至今是空目录，而装出来的技能落进
// 「当时打开的那个项目」，换个项目整批消失。根因是**读写两半不对齐**——
//
//   读：_skillDiscoveryBases 把家目录排在最后一位，家目录那份是能扫到的；
//   写：save_skill、技能市场、_skillWorkspaceInstallRoot 三个入口全部钉死在工作区根，
//       全仓没有任何一处以 home 为基去写。
//
// 而且光把 JS 路径改到家目录会**更糟**：Rust 侧 files.rs 的 require_inside_workspace
// 明令拒绝"在 HOME 底下但不在已打开工作区里"的写入（正是它挡住 ~/.ssh、~/.bashrc），
// 用户点安装会当场看到 write denied。所以要照 mcp.rs 那对用户级命令的形状，另开一组
// **作用域钉死在技能库**的 Tauri 命令。
//
// 这份文件按链条逐环钉：落点 → 发现 → 权限登记 → 面板 → 文案 → 当轮可见 → 常驻迁移。
import { readFileSync } from "node:fs";
import { normalizeFsPath, pathIdentity, pathIsAtOrUnder } from "../src/agent/paths.js";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

import { CODE, SRC as RAW_SRC, fnSource, load, loadConst } from "./helpers/source.mjs";
import {
  mutatesWorkspace,
  workerScopeField,
  approvalTypes,
  readOnlyBlockedTypes,
  toolPolicy,
} from "../src/agent/tool-policy.js";

const HERE = dirname(fileURLToPath(import.meta.url));
const MCP_RS = readFileSync(join(HERE, "../src-tauri/src/mcp.rs"), "utf8");
const LIB_RS = readFileSync(join(HERE, "../src-tauri/src/lib.rs"), "utf8");

// ── ① 发现：只有家目录技能库这一条 ────────────────────────────────────────────

test("发现路径只有家目录技能库一条，工作区那条整条不在了", () => {
  const bases = load("_skillDiscoveryBases", { _STATE_DIR: loadConst("_STATE_DIR") });
  assert.deepEqual(bases("/home/tester"), ["/home/tester/.mrdayone/skills"]);
  // Rust 给的绝对路径胜出：app_dir() 会经 migrate_app_dir 回退到老的 `.michael-ide`，
  // JS 常量不知道这回事，两边指到不同目录就是"发现扫新的、写入写老的"。
  assert.deepEqual(bases("/home/tester", "/home/tester/.michael-ide/skills"),
    ["/home/tester/.michael-ide/skills"]);
  assert.deepEqual(bases(""), [], "连家目录都没有时给空清单，不要拼出一个 `/…/skills`");

  // 反向：函数体里不许再出现任何以工作区为基的拼装。
  const body = fnSource("_skillDiscoveryBases", { code: true });
  assert.doesNotMatch(body, /_workspaceAncestorRoots/);
  assert.doesNotMatch(body, /projectRoot/);
});

test("技能库的绝对路径来自 Rust，JS 只在拿不到时才按常量兜底", () => {
  const src = fnSource("_skillsHomeRoot", { code: true });
  assert.match(src, /backend\.invoke\("skills_dir"\)/,
    "又在 JS 里拼 ~/.mrdayone/skills 了——那会和 migrate_app_dir 的回退目录对不上");
  assert.match(src, /_skillDiscoveryBases\(home\)/, "兜底路径没走同一个拼装函数");
});

test("目录缓存键里没有项目路径——换文件夹不再让技能清单串台", () => {
  const src = fnSource("_refreshFileSkills", { code: true });
  assert.doesNotMatch(src, /const projectRoot/,
    "cacheKey 里又混进了项目路径：换 root 后 key 只是不相等、并不为空，"
    + "sendPrompt 那道 `!_fileSkillsCacheKey` 的等待会被跳过，这一轮用的是上一个项目的技能");
  assert.match(src, /const cacheKey = skillsRoot/);
});

// ── ② 写入：三个入口全部落在技能库，且都走专用命令 ──────────────────────────

test("save_skill 落在技能库，不再要求打开工作区", () => {
  const at = RAW_SRC.indexOf('} else if (call.type === "saveskill") {');
  assert.ok(at > 0, "save_skill 的执行分支找不到了");
  const exec = CODE.slice(at, RAW_SRC.indexOf('} else if (call.type ===', at + 40));

  assert.match(exec, /const _skillsRoot = await _skillsHomeRoot\(\);/);
  assert.match(exec, /const fp = `\$\{_skillsRoot\}\/\$\{_slug\}\/SKILL\.md`;/);
  // 那条 BLOCKED 分支整条没了：它把一个全局能力绑在了"当时打开的那个项目"上。
  assert.doesNotMatch(exec, /\[BLOCKED\]/);
  assert.doesNotMatch(exec, /create_project/);
  assert.doesNotMatch(exec, /_resolveRel/, "又按工作区相对路径解析了");
  // 落盘只走技能库自己那条命令。
  assert.match(exec, /backend\.invoke\("skills_write_file", \{ name: _slug, rel: "SKILL\.md", text: _doc \}\)/);
  assert.doesNotMatch(exec, /backend\.writeTextFile/,
    "又走通用写文件了——require_inside_workspace 会把 HOME 底下的写入拒成 write denied");
});

test("技能市场装到技能库，没打开文件夹也能装", () => {
  const src = fnSource("_skillInstallDir", { code: true });
  assert.doesNotMatch(src, /先打开一个工作区文件夹/, "市场又要求先开文件夹了");
  assert.doesNotMatch(src, /rootPath \|\| workspaceRoots/, "又去读当前工作区了");
  assert.match(src, /const skillsRoot = await _skillsHomeRoot\(\);/);
  assert.match(src, /const destBase = `\$\{skillsRoot\}\/\$\{destName\}`;/);
  assert.match(src, /backend\.invoke\("skills_write_file"/);
  assert.doesNotMatch(src, /backend\.writeTextFile|backend\.createDir/,
    "又走通用写文件/建目录了——那两条对 HOME 底下的技能库都是 write denied");

  // 按钮不许再因为"没打开文件夹"而变灰：那条路上市场整页是死的，且不说为什么。
  const panel = fnSource("renderSkillsTool", { code: true });
  assert.doesNotMatch(panel, /installed \|\| installing \|\| !root/,
    "市场卡片又按有没有工作区置灰了");
  assert.doesNotMatch(panel, /if \(s && root\) doInstall/,
    "点击分支又加了一道 root 判断——按钮能点却静默 no-op，比灰着还糟");
  assert.match(panel, /if \(s\) doInstall/);
});

test("删除走技能库自己的命令，并且每次都确认（它是跨项目的）", () => {
  const src = fnSource("_deleteSkillRecord", { code: true });
  assert.match(src, /backend\.invoke\("skills_delete", \{ name: dirName \}\)/);
  assert.doesNotMatch(src, /backend\.deletePath/,
    "通用删除对 HOME 底下的技能库是 write denied");
  assert.match(src, /if \(!ok\) return;/, "用户点了取消还照删");
  assert.doesNotMatch(src, /_skillIsWorkspaceInstalled/);
});

// ── ③ Rust 侧：那道墙不许挖开，另开一条作用域钉死的窄路 ──────────────────────

test("技能库的三条命令已注册，且路径不接受调用方输入", () => {
  for (const cmd of ["skills_dir", "skills_write_file", "skills_delete"]) {
    assert.match(LIB_RS, new RegExp(`mcp::${cmd},`), `${cmd} 没注册进 invoke_handler，前端调它是 unknown command`);
    assert.match(MCP_RS, new RegExp(`pub fn ${cmd}\\(`), `${cmd} 不存在`);
  }
  // 命令签名里**只有**技能名和目录内相对路径，没有任何一个收绝对路径/根目录的参数。
  assert.match(MCP_RS, /pub fn skills_write_file\(name: String, rel: String, text: String\)/);
  assert.match(MCP_RS, /pub fn skills_delete\(name: String\)/);
  // 根由 app_dir() 拼，不由调用方给。
  assert.match(MCP_RS, /fn skills_root\(\) -> Result<std::path::PathBuf, String> \{\s*Ok\(app_dir\(\)\?\.join\(SKILLS_DIR\)\)/);
});

test("require_inside_workspace 那道 HOME 写入墙一个字节都没动", () => {
  const files = readFileSync(join(HERE, "../src-tauri/src/files.rs"), "utf8");
  assert.match(files, /write denied: '\{\}' is under HOME but not inside any opened workspace\./,
    "为了让技能能写家目录，把挡住 ~\/.ssh、~\/.bashrc 的那道墙拆了");
  assert.match(files, /let under_workspace = roots\s*\n?\s*\.iter\(\)\s*\n?\s*\.any\(\|root\| resolved\.starts_with\(root\) && !is_home\(root\)\);/);
});

test("路径校验是白名单，`..` 和分隔符逐段拒", () => {
  // 这是这组命令**唯一**的安全边界，所以它必须在 Rust 里逐段判，而不是靠前端自觉。
  const seg = MCP_RS.slice(MCP_RS.indexOf("fn skill_segment("), MCP_RS.indexOf("fn skill_file_path("));
  assert.match(seg, /seg\.is_empty\(\) \|\| seg == "\." \|\| seg == "\.\."/);
  assert.match(seg, /'\/' \| '\\\\' \| ':'/);
  assert.match(seg, /c\.is_control\(\)/);
  const build = MCP_RS.slice(MCP_RS.indexOf("fn skill_file_path("), MCP_RS.indexOf("pub fn skills_dir("));
  assert.match(build, /skill_segment\(name\)\?;/);
  // 绝对路径要**报错**，不是剥掉斜杠悄悄当成相对的：那样 `/etc/passwd` 会落成
  // `<技能>/etc/passwd`，安全但不是调用方要的东西，静默改写比拒绝更难查。
  assert.match(build, /rel\.starts_with\('\/'\) \|\| rel\.starts_with\('\\\\'\)/);
  assert.match(build, /for seg in &segs \{\s*\n\s*skill_segment\(seg\)\?;/);
  assert.match(build, /if !path\.starts_with\(root\)/, "逐段校验之后没有再验一次落点");
});

// ── ④ 权限登记：技能库不是工作区 ────────────────────────────────────────────

test("saveskill 从工作区改动 / worker scope / 工作区钩子三张表里摘掉，但审批和只读挡住留着", () => {
  assert.equal(mutatesWorkspace("saveskill"), false,
    "它一个工作区文件都不碰，报成「改了工作区」会让 mutated 不再是证据");
  assert.equal(workerScopeField("saveskill"), "",
    "worker 的 scope 是工作区内的相对路径，技能库的绝对路径必然在它之外——子智能体存技能会被整条拒掉");
  assert.equal(toolPolicy("saveskill").hooked, false,
    "工作区的 pre_tool_use 钩子对一个不落在这个项目里的写入没有管辖权");
  assert.equal(approvalTypes().has("saveskill"), true, "在用户家目录建文件，审批不许丢");
  assert.equal(readOnlyBlockedTypes().has("saveskill"), true, "只读模式不许留下持久化写入");
});

test("saveskill 的显示路径不再是一个会被当成工作区相对路径的字符串", () => {
  const src = fnSource("_mapToolCall", { code: true });
  const at = src.indexOf('case "save_skill":');
  assert.ok(at > 0, "save_skill 的归一化分支找不到了");
  const branch = src.slice(at, at + 900);
  assert.match(branch, /`~\/\$\{_STATE_DIR\}\/skills\/\$\{_slug\}\/SKILL\.md`/,
    "又拼成工作区相对路径了——那会被 _resolveRel / scope 检查当成项目里的文件");
});

// ── ⑤ 装完 / 存完当轮就能被发现，且不再自动置常驻 ────────────────────────────

test("save_skill 写完不只是清缓存，还要主动重扫——否则同一个 run 里 read_skill 读不回来", () => {
  const at = RAW_SRC.indexOf('} else if (call.type === "saveskill") {');
  const exec = CODE.slice(at, RAW_SRC.indexOf('} else if (call.type ===', at + 40));
  const clear = exec.indexOf('_fileSkillsCacheKey = "";');
  const rescan = exec.indexOf("await _refreshFileSkills()");
  assert.ok(clear > 0, "没清技能目录缓存");
  assert.ok(rescan > clear,
    "只清了 key 没重扫：read_skill 查的是模块级 _fileSkills，run 内没有别的地方会触发重扫，"
    + "模型照着回执那句「可以用 read_skill 读全文」去验会拿到「没有名为 X 的技能」");
});

test("市场装完只刷新目录，不再偷偷把技能置为常驻", () => {
  const src = fnSource("_skillPostInstall", { code: true });
  assert.match(src, /_fileSkillsCacheKey = "";/);
  assert.match(src, /await _refreshFileSkills\(\)/);
  /*
   * 自动置常驻是一条**静默收窄整个 IDE** 的路：_skillAllowedTools 对每个"常驻且声明了
   * allowed-tools"的技能取白名单并集，而这道闸排在权限规则之前，不在名单里的工具直接
   * return false。装一个声明了 allowed-tools 的技能，写文件/搜索/跑命令就可能被静默
   * 拒掉——用户既没选过常驻，也无处查看被收窄成了什么。
   */
  assert.doesNotMatch(src, /_toggleSkillActive/,
    "装完又自动置常驻了——一个声明了 allowed-tools 的技能会当场把别的工具全挡掉");
  assert.doesNotMatch(src, /_isSkillActive/);
  // 反向：那道闸本身要留着（它是技能声明的真约束），只是不再由安装自动打开。
  assert.match(CODE, /function _skillAllowedTools\(\)/);
});

// ── ⑥ 常驻状态跟着技能搬家 ──────────────────────────────────────────────────

test("升级时按目录名把常驻 id 迁到技能库，只迁一次", () => {
  const store = new Map();
  const active = new Set([
    "file:/repo/.mrdayone/skills/docx/SKILL.md",       // 老的工作区落点
    "file:/home/me/.mrdayone/skills/pdf/SKILL.md",     // 已经在技能库里的，不动
    "s1738",                                           // 自定义技能的 id，不是路径
  ]);
  let saved = 0;
  const migrate = load("_migrateActiveSkillIdsToHome", {
    _SKILLS_HOME_MIGRATED_KEY: loadConst("_SKILLS_HOME_MIGRATED_KEY"),
    localStorage: {
      getItem: (k) => (store.has(k) ? store.get(k) : null),
      setItem: (k, v) => store.set(k, String(v)),
    },
    _activeSkillIds: active,
    _saveActiveSkills: () => { saved++; },
  });

  migrate("/home/me/.mrdayone/skills");
  assert.deepEqual([...active].sort(), [
    "file:/home/me/.mrdayone/skills/docx/SKILL.md",
    "file:/home/me/.mrdayone/skills/pdf/SKILL.md",
    "s1738",
  ].sort(), "老 id 没按目录名映射过来——用户钉住的常驻技能升级后会全部变回「按需」");
  assert.equal(saved, 1, "迁完没落盘");

  // 第二次调用是 no-op：带标记，不会把用户之后自己取消的常驻又翻回来。
  active.delete("file:/home/me/.mrdayone/skills/docx/SKILL.md");
  migrate("/home/me/.mrdayone/skills");
  assert.ok(!active.has("file:/home/me/.mrdayone/skills/docx/SKILL.md"),
    "迁移又跑了一遍，把用户取消掉的常驻翻回来了");
  assert.equal(saved, 1);
});

// ── ⑦ 面板与文案：说的落点要和真落点一致 ────────────────────────────────────

test("面板按技能库判「已安装」，不再按当前工作区", () => {
  // 路径这一簇已搬进 src/agent/paths.js —— 从模块注入，不再从 main.js 抠。
  // pathIdentity 收 remote 参数；这里按「本机、无远程」跑，和原来的语义一致。
  const _local = { active: false, platform: "" };
  const isInstalled = new Function(
    "_normalizeFsPath", "_pathIdentity", "_pathIsAtOrUnder",
    `${fnSource("_skillIsLibraryInstalled")}\n;return _skillIsLibraryInstalled;`,
  )(normalizeFsPath, (x) => pathIdentity(x, _local), (a, b) => pathIsAtOrUnder(a, b, _local));
  const root = "/home/me/.mrdayone/skills";
  assert.equal(isInstalled({ baseDir: "/home/me/.mrdayone/skills/docx" }, root), true);
  assert.equal(isInstalled({ baseDir: "/repo/.mrdayone/skills/docx" }, root), false);
  assert.equal(isInstalled({ baseDir: root }, root), false, "技能库根自己不是一个技能");
  assert.equal(isInstalled({}, root), false);
  assert.equal(isInstalled({ baseDir: "/home/me/.mrdayone/skills/docx" }, ""), false);
});

test("三处面向用户/模型的落点文案都改成技能库，不再说工作区", () => {
  // ① 能力缺口时的换路清单：模型正是在这一刻被指去存技能的。
  const routes = loadConst("_CAPABILITY_ROUTES");
  assert.match(routes, /~\/\.mrdayone\/skills\//, "换路清单还在指工作区");
  assert.doesNotMatch(routes, /<工作区>\/\.mrdayone\/skills/);

  // ② 面板副标题：用户读到的落点必须和真落点一致。
  const panel = fnSource("renderSkillsTool", { code: true });
  assert.match(panel, /跨项目复用/, "副标题没说清技能是跨项目的");
  assert.doesNotMatch(panel, /装在工作区 <code>/, "副标题还写着「装在工作区」");
  assert.match(panel, /memory\.md/, "没说清项目里的 .mrdayone 是干什么的");

  // ③ 安装成功的 toast 打绝对落点：项目里也有一个同名的 .mrdayone，相对路径看不出是哪个。
  assert.match(panel, /已安装（\$\{r\.fileCount\} 个文件 → \$\{r\.destBase\}）/);
  assert.doesNotMatch(panel, /已安装并启用/, "装完不置常驻了，文案不能还说「并启用」");
});

test("save_skill 的工具描述说的是技能库，且不再要求先建项目", () => {
  const at = RAW_SRC.indexOf('name: "save_skill"');
  assert.ok(at > 0, "save_skill 的 schema 找不到了");
  const desc = RAW_SRC.slice(at, at + 1600);
  assert.match(desc, /~\/\.mrdayone\/skills\/<name>\/SKILL\.md/,
    "描述还写着 <workspace>/… —— 模型按描述行事，落点改了它照样往项目里存");
  assert.doesNotMatch(desc, /Needs an open workspace/,
    "还在要求打开工作区：模型会先去 create_project 建一个目录");
  assert.doesNotMatch(desc, /<workspace>/);
  assert.match(desc, /Skills are GLOBAL/,
    "没说清技能是跨项目的");
  assert.match(desc, /belongs in remember/,
    "没给出「这个项目专属的事写哪」的出口——那正是项目 .mrdayone/memory.md 的用途");

  /*
   * 同一份描述在网关的 prompts/tools.json 里还有一份，而**运行时是网关那份说了算**。
   * 改了源码、跑起来没变化，因为模型读的根本是另一份。build/sync-tools-json.mjs --check
   * 守着这条（test/logic.test.mjs 与 test/intent-timing.test.mjs 各跑一次），所以这里
   * 只钉「源码这份说对了」，两份是否同步由那道检查负责。
   */
});
