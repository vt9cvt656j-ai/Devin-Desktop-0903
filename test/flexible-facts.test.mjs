// 两处「把世界写死在代码里」的表，被改成了随执行事实走的活数据。这个文件守它们真的活着。
//
// 为什么必须是行为测试而不是源码断言：这两处上线时**整块挖掉都没有一条测试变红**
// （变异实测 2026-08-22：`_applyUserRoleEnums` 直接 `return tools`、
// `_officialDocsHosts` 的 harvested 分支置空，全量套件 fail 数纹丝不动）。
// 源码断言只能守住"这段字还在"，守不住"它真的改变了发出去的东西"。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load, fnSource } from "./helpers/source.mjs";

// ── A. 用户自己声明的角色，要真的出现在模型看得见的 enum 里 ──────────────
//
// 角色枚举原来是三处硬编码的 `enum: ["architect","product","research",…]`。用户在
// 能力声明里加一个角色，模型的 schema 里根本没有这个值——填了就是非法参数，
// 于是"自定义角色"这个功能对模型是不存在的。
function applyRoles(roles) {
  const apply = load("_applyUserRoleEnums", {
    _userCapabilities: () => ({ roles: roles.map((name) => ({ name })) }),
  });
  const tools = [{
    type: "function",
    function: {
      name: "run_subagent",
      parameters: { type: "object", properties: {
        tasks: { type: "array", items: { type: "object", properties: {
          role: { type: "string", enum: ["architect", "product", "research"] },
        } } },
      } },
    },
  }];
  return { out: apply(tools), tools };
}
const roleEnum = (tools) =>
  tools[0].function.parameters.properties.tasks.items.properties.role.enum;

test("用户声明的角色真的进了 run_subagent 的 role 枚举", () => {
  const { out } = applyRoles(["security-auditor"]);
  assert.ok(roleEnum(out).includes("security-auditor"),
    "自定义角色没进 enum——模型填它就是非法参数，等于这个功能不存在");
  assert.ok(roleEnum(out).includes("architect"), "内置角色不能被顶掉");
});

test("没有自定义角色时，schema 一个字节都不变", () => {
  const { out, tools } = applyRoles([]);
  assert.equal(out, tools, "空声明必须原样返回，不该白拷一份");
  assert.deepEqual(roleEnum(out), ["architect", "product", "research"]);
});

test("绝不就地改 parameters——那个对象长期活在云端目录缓存里", () => {
  // `_applyCloudToolDescs` 把 function.parameters 直接指向 _remoteTools 里的对象，
  // 它跨调用存活。就地改会把角色名永久焊进缓存：用户删掉角色它还在，换个工作区也还在。
  const shared = { type: "string", enum: ["architect"] };
  const tools = [{
    type: "function",
    function: { name: "run_subagent", parameters: { type: "object", properties: { role: shared } } },
  }];
  const apply = load("_applyUserRoleEnums", { _userCapabilities: () => ({ roles: [{ name: "sre" }] }) });
  const out = apply(tools);
  assert.deepEqual(shared.enum, ["architect"], "原对象被就地改了——角色名会焊死在云端缓存里");
  assert.ok(out[0].function.parameters.properties.role.enum.includes("sre"), "返回的那份要有新角色");
});

test("取角色名不能走 _userRoleMap——那条路会无限递归", () => {
  // _userRoleMap() 为了校验工具名会回头调 _buildAgentToolSchemas，而本函数正在它的
  // 返回路径上。这条不是风格问题，是死循环。
  assert.doesNotMatch(fnSource("_applyUserRoleEnums", { code: true }), /_userRoleMap\s*\(/,
    "走 _userRoleMap 会经 _buildAgentToolSchemas 兜回本函数，无限递归");
});

test("声明读不到时安静退回，不能把整份工具表带崩", () => {
  const apply = load("_applyUserRoleEnums", { _userCapabilities: () => { throw new Error("配置坏了"); } });
  const tools = [{ type: "function", function: { name: "x", parameters: {} } }];
  assert.equal(apply(tools), tools, "读不到声明应原样返回");
});

// ── C. 官方证据认的是「这个包自己声明的站点」，不是一张 33 域的硬编码白名单 ─────
//
// 原判据是常量 _OFFICIAL_RESEARCH_HOSTS（33 个域名）。项目真实用着的依赖，官方站点
// 绝大多数不在这 33 个里——模型老实去读了官方文档，取证账上仍然是零。
const RESULT = (content) => ({ content });

test("从 package_search 结果里摘出这个包自己声明的主页/仓库", () => {
  const harvest = load("_declaredDocsHostsFromResult", ["_DECLARED_DOCS_KEY_RE", "_declaredDocsHostsFromResult"]);
  const hosts = harvest("package_search", RESULT(
    `tauri 2.1.0\n  homepage: https://tauri.app/\n  repository: https://github.com/tauri-apps/tauri\n`,
  ));
  assert.ok(hosts.includes("tauri.app"), "包自己声明的主页没被认出来");
});

test("不是取包信息的工具，一个主机都不许摘", () => {
  const harvest = load("_declaredDocsHostsFromResult", ["_DECLARED_DOCS_KEY_RE", "_declaredDocsHostsFromResult"]);
  // 论坛帖子里随手贴的链接不是"这个包声明的官方站"，摘进来等于让任何人都能伪造官方证据。
  assert.deepEqual(
    harvest("developer_community_search", RESULT("homepage: https://evil.example.com/")), [],
    "只有 package_search / package_source 的结果才是包声明事实",
  );
  assert.deepEqual(harvest("web_fetch", RESULT("homepage: https://evil.example.com/")), []);
});

test("读了包声明的官方站，算官方取证；同一次读换个站就不算", () => {
  const isOfficial = load("_isOfficialResearchUrl", ["_OFFICIAL_RESEARCH_HOSTS", "_isOfficialResearchUrl"]);
  assert.equal(isOfficial("https://tauri.app/start/", new Set(["tauri.app"])), true,
    "包声明的官方站没被认成官方证据——这正是「读了官方文档反被判没查」");
  assert.equal(isOfficial("https://v2.tauri.app/start/", new Set(["tauri.app"])), true,
    "子域也要认：包声明 tauri.app，模型读的常常是 v2.tauri.app");
  assert.equal(isOfficial("https://random.example.com/x", new Set(["tauri.app"])), false);
  assert.equal(isOfficial("https://nottauri.app/x", new Set(["tauri.app"])), false,
    "后缀匹配不能退化成子串匹配");
});

test("硬编码那 33 个降为兜底，没有额外主机时照样认", () => {
  const isOfficial = load("_isOfficialResearchUrl", ["_OFFICIAL_RESEARCH_HOSTS", "_isOfficialResearchUrl"]);
  assert.equal(isOfficial("https://developer.mozilla.org/en-US/docs/Web", null), true,
    "兜底表被这次改动弄丢了");
  // 兜底表这一侧同样必须是**后缀**匹配。退化成子串的话，任何人注册一个把官方域名
  // 塞进自己主机名的站点就能伪造官方证据——而证据账本是收尾判「查没查」的依据。
  assert.equal(isOfficial("https://developer.mozilla.org.attacker.example/x", null), false,
    "兜底表退化成子串匹配了：伪造官方证据只需要在自己域名里塞一段官方域名");
});

test("run 上摘到的主机会喂进证据判定，且有上限", () => {
  const record = load("_recordDeclaredDocsHosts", ["_DECLARED_DOCS_KEY_RE", "_declaredDocsHostsFromResult", "_recordDeclaredDocsHosts"]);
  const run = {};
  record(run, "package_search", RESULT("homepage: https://tauri.app/"));
  assert.ok(run._declaredDocsHosts instanceof Set && run._declaredDocsHosts.has("tauri.app"));
  // 上限：一次巨大的结果不能把这张表撑爆。
  // 每轮必须换一批主机：单次解析自带 12 条上限，重复喂同一批的话集合永远停在 12 条，
  // 64 这道闸门根本不会被碰到——那样这条断言就是绿着的摆设。
  for (let i = 0; i < 20; i++) {
    record(run, "package_search", RESULT(
      Array.from({ length: 12 }, (_, j) => `homepage: https://h${i}-${j}.example.com/`).join("\n"),
    ));
  }
  assert.ok(run._declaredDocsHosts.size <= 64, `表被撑到 ${run._declaredDocsHosts.size} 条`);
});

test("run 上的主机与用户配置的内部文档站合并成同一份判据", () => {
  // 这是取值那一层：只测 _isOfficialResearchUrl 的话，把 _officialDocsHosts 里
  // 「本次会话摘到的那半」整块删掉仍然全绿——实测确实如此，所以补这一条。
  const hostsOf = load("_officialDocsHosts", {
    _userCapabilities: () => ({ officialDocsHosts: ["docs.internal.corp"] }),
  });
  const hosts = hostsOf({ _declaredDocsHosts: new Set(["tauri.app"]) });
  assert.ok(hosts.has("tauri.app"), "本次会话从包声明里摘到的主机没进判据");
  assert.ok(hosts.has("docs.internal.corp"), "用户配置的内部文档站没进判据");

  // 配置读不到时，本次会话摘到的那半仍然要在——两个来源互不拖累。
  const brittle = load("_officialDocsHosts", { _userCapabilities: () => { throw new Error("坏了"); } });
  assert.ok(brittle({ _declaredDocsHosts: new Set(["tauri.app"]) }).has("tauri.app"),
    "配置读不到就把执行事实一起丢了");
  assert.equal(hostsOf({}).size, 1, "run 上没摘到东西时只剩用户配置那一份");
});

test("证据仍然必须来自真的读到了正文", () => {
  // 放宽主机判据**不能**放宽"真读到了东西"这条：否则一次失败的抓取也会记成官方取证。
  const category = load("_researchEvidenceCategory", [
    "_OFFICIAL_RESEARCH_EVIDENCE_TOOLS", "_COMMUNITY_RESEARCH_EVIDENCE_TOOLS",
    "_OFFICIAL_RESEARCH_HOSTS", "_isOfficialResearchUrl", "_researchResultHasEvidence",
    "_researchEvidenceCategory",
  ]);
  const hosts = new Set(["tauri.app"]);
  assert.equal(category("web_fetch", { url: "https://tauri.app/start/" },
    { content: "x".repeat(200) }, hosts), "official");
  assert.equal(category("web_fetch", { url: "https://tauri.app/start/" },
    { content: "[BLOCKED 403]" }, hosts), "", "抓取失败不能记成取证");
  assert.equal(category("web_fetch", { url: "https://tauri.app/start/" },
    { content: "" }, hosts), "", "空正文不能记成取证");
});
