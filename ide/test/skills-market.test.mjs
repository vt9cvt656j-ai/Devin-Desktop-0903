// 技能市场：可安装性判据前移到列表阶段，以及 GitHub 未鉴权限流怎么如实说。
//
// 两条真 bug：
//
// ① 列表不管装不装得上。市场的数据源是 GitHub 仓库搜索
//    `claude skill in:name,description,topics`，按 star 排序直接当「热门技能」列出来。
//    但「这个仓库到底有没有 SKILL.md」以前只在用户点安装那一刻才验——_skillInstallFromRepo
//    拉仓库树，找不到就 throw。而这个搜索式命中的是**名字/描述/话题里提到** "claude skill"
//    的仓库，很多只是在**讨论** Claude 技能，并不**是**一个技能包。用户看到一排热门、
//    点安装、报错，反复如此。
//
// ② GitHub 未鉴权额度是核心 60 次/小时、搜索 10 次/分钟
//    （curl https://api.github.com/rate_limit 可复现）。撞上之后共用的拉取函数
//    _mcpRegFetchJson 只 `throw new Error("HTTP 403")`，用户看到「获取失败：HTTP 403」
//    或一片空白：不知道为什么，也不知道等多久。而 x-ratelimit-remaining /
//    x-ratelimit-reset / retry-after 就在响应里，http_request 那条 IPC 路径本来就把整张
//    headers 表回传（src-tauri/src/net.rs 的 HttpResponse.headers）。
//
// 这组用例跑的是 main.js 里的真函数（helpers/source.mjs 按 AST 边界抠出来注入依赖），
// 且**不打真实网络**：GitHub 搜索响应、仓库树响应、403 限流响应（含响应头）全是夹具。
import test from "node:test";
import assert from "node:assert/strict";
import { load, fnSource, CODE } from "./helpers/source.mjs";
import { escapeAttr as _escAttr, escapeHtml as _escHtml } from "../src/agent/escape.js";

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

// api.github.com/search/repositories 一条结果的真实字段（只留这里用到的那些）。
const SEARCH_REAL_SKILL = {
  name: "pdf-filler",
  full_name: "acme/pdf-filler",
  description: "A Claude skill that fills PDF forms",
  stargazers_count: 812,
  owner: { login: "acme", avatar_url: "https://avatars.githubusercontent.com/u/1?v=4" },
  default_branch: "main",
  html_url: "https://github.com/acme/pdf-filler",
};

// 「只是在讨论」的那类：标题里有 claude skill，仓库里一个 SKILL.md 都没有。
const SEARCH_NOT_A_SKILL = {
  name: "awesome-claude-skill-notes",
  full_name: "someone/awesome-claude-skill-notes",
  description: "我整理的 Claude skill 学习笔记",
  stargazers_count: 4300,
  owner: { login: "someone", avatar_url: "https://avatars.githubusercontent.com/u/2?v=4" },
  default_branch: "master",
  html_url: "https://github.com/someone/awesome-claude-skill-notes",
};

// 合集仓库：SKILL.md 在子目录里，根目录没有。
const TREE_COLLECTION = {
  tree: [
    { path: "README.md", type: "blob", size: 900 },
    { path: "skills", type: "tree" },
    { path: "skills/docx/SKILL.md", type: "blob", size: 4000 },
    { path: "skills/docx/refs.md", type: "blob", size: 1200 },
    { path: "skills/pdf-filler/SKILL.md", type: "blob", size: 3000 },
  ],
};

const TREE_ROOT_SKILL = {
  tree: [
    { path: "SKILL.md", type: "blob", size: 2000 },
    { path: "scripts/run.py", type: "blob", size: 500 },
  ],
};

const TREE_NO_SKILL = {
  tree: [
    { path: "README.md", type: "blob", size: 9000 },
    { path: "notes/claude-skills.md", type: "blob", size: 5000 },
    { path: "notes/SKILL.md.draft", type: "blob", size: 100 },
  ],
};

// GitHub 撞限流时**真实**回的那组头（未鉴权核心桶）。
const RATE_LIMITED_HEADERS = {
  "content-type": "application/json; charset=utf-8",
  "x-ratelimit-limit": "60",
  "x-ratelimit-remaining": "0",
  "x-ratelimit-used": "60",
  "x-ratelimit-resource": "core",
  "x-ratelimit-reset": "1755870000", // epoch 秒
};

// 同样是 403，但**没有**额度耗尽的证据（例如被封禁）：不许被说成"等等就好"。
const FORBIDDEN_HEADERS = {
  "content-type": "application/json; charset=utf-8",
  "x-ratelimit-limit": "60",
  "x-ratelimit-remaining": "57",
};

const RESET_MS = 1755870000 * 1000;

// ---------------------------------------------------------------------------
// 取真函数
// ---------------------------------------------------------------------------
const headerGet = load("_ghHeaderGet", ["_ghHeaderGet"]);
const rateFromHeaders = load("_ghRateFromHeaders", ["_ghHeaderGet", "_ghRateFromHeaders"]);
const waitText = load("_ghWaitText", ["_ghWaitText"]);
const rateVerdict = load("_ghRateLimitVerdict", [
  "_ghHeaderGet", "_ghRateFromHeaders", "_ghWaitText", "_ghRateLimitMessage", "_ghRateLimitVerdict",
]);
const httpError = load("_ghHttpError", [
  "_ghHeaderGet", "_ghRateFromHeaders", "_ghWaitText", "_ghRateLimitMessage", "_ghRateLimitVerdict", "_ghHttpError",
]);
const rateBucket = load("_ghRateBucket", ["_ghRateBucket"]);
const pickDir = load("_skillPickDirFromTree", ["_skillPickDirFromTree"]);

// _ghRate 是模块级可变状态：把 note/budget 一起抓进同一个作用域才能测到它们的联动。
const rateBudgetPair = new Function(`
  ${fnSource("_ghHeaderGet")}
  ${fnSource("_ghRateFromHeaders")}
  ${fnSource("_ghRate")}
  ${fnSource("_ghRateNote")}
  ${fnSource("_ghRateBudget")}
  return { note: _ghRateNote, budget: _ghRateBudget, state: () => _ghRate };
`)();

const marketCard = load("_skillMarketCardHtml", {
  _SKILL_NOT_A_SKILL_NOTE: load("_SKILL_NOT_A_SKILL_NOTE", ["_SKILL_NOT_A_SKILL_NOTE"]),
  _SKILL_UNVERIFIED_NOTE: load("_SKILL_UNVERIFIED_NOTE", ["_SKILL_UNVERIFIED_NOTE"]),
  _mcpStarsText: (n) => String(n),
  _dbUiIconSvg: () => "<svg></svg>",
  _escAttr,
  _escHtml,
});

const marketErrorHtml = load("_skillMarketErrorHtml", {
  _ghWaitText: waitText,
  _escHtml,
});

// ---------------------------------------------------------------------------
// 1. 判据：只认「有没有 SKILL.md」这个执行事实
// ---------------------------------------------------------------------------

test("判据是仓库树里的 SKILL.md，不是名字/描述里提到 claude skill", () => {
  // 4300 星、标题里明晃晃写着 claude skill —— 但它是一份笔记，不是技能包。
  assert.equal(pickDir(TREE_NO_SKILL, SEARCH_NOT_A_SKILL.name), null);
  // "notes/SKILL.md.draft" 不是 SKILL.md，别被前缀骗过去。
  assert.equal(pickDir({ tree: [{ path: "SKILL.md.bak", type: "blob" }] }, "x"), null);
  // 目录条目（type: "tree"）也不算。
  assert.equal(pickDir({ tree: [{ path: "a/SKILL.md", type: "tree" }] }, "x"), null);
});

test("根目录 SKILL.md 优先，其次同名目录，最后第一个", () => {
  assert.deepEqual(pickDir(TREE_ROOT_SKILL, "whatever"), { dir: "", dirs: [""] });
  assert.equal(pickDir(TREE_COLLECTION, "pdf-filler").dir, "skills/pdf-filler");
  assert.equal(pickDir(TREE_COLLECTION, "不存在的名字").dir, "skills/docx");
  assert.deepEqual(pickDir(TREE_COLLECTION, "docx").dirs, ["skills/docx", "skills/pdf-filler"]);
  assert.equal(pickDir(null, "x"), null);
  assert.equal(pickDir({}, "x"), null);
});

test("列表校验和真正安装必须是同一段代码得出的结论", () => {
  // 判据前移的全部意义在这：列表说能装、点下去真能装。两边各写一份 SKILL.md 扫描逻辑，
  // 迟早分叉回今天这个 bug。
  const install = fnSource("_skillInstallFromRepo", { code: true });
  assert.match(install, /_skillPickDirFromTree\(/, "安装那一步必须走同一个目录挑选函数");
  assert.doesNotMatch(install, /SKILL\\\.md\$/, "安装那一步不许自己再写一遍 SKILL.md 扫描");
  const probe = fnSource("_skillProbeTree", { code: true });
  assert.match(probe, /_skillPickDirFromTree\(/, "列表校验也走同一个");
});

test("装不了的结论会落盘——下次打开市场这条不再冒充能装的技能", () => {
  const install = fnSource("_skillInstallFromRepo", { code: true });
  assert.match(install, /_skillVerdictSet\(item\.full,\s*branch,\s*"no"/, "点了发现没有 SKILL.md，要把这条标成 no");
  assert.match(install, /_skillVerdictSet\(item\.full,\s*branch,\s*"yes"/, "装成功也要记，省下下次的树请求");
});

// ---------------------------------------------------------------------------
// 2. 呈现：装不了的标出来 + 给「查看仓库」，不从列表里悄悄丢掉
// ---------------------------------------------------------------------------

const asServer = (raw, verdict, skillDir = "") => ({
  name: raw.name,
  full: raw.full_name,
  desc: raw.description,
  stars: raw.stargazers_count,
  avatar: raw.owner.avatar_url,
  owner: raw.owner.login,
  branch: raw.default_branch,
  url: raw.html_url,
  verdict,
  skillDir,
});

test("验过没有 SKILL.md 的卡片：标出来、给「查看仓库」、说清为什么，不给「安装」", () => {
  const html = marketCard(asServer(SEARCH_NOT_A_SKILL, "no"), { index: 3 });
  assert.doesNotMatch(html, /data-skfp-install/, "装不了就不许出现安装按钮");
  assert.match(html, /非技能包/);
  assert.match(html, /查看仓库/);
  assert.match(html, /data-skfp-repo="https:\/\/github\.com\/someone\/awesome-claude-skill-notes"/);
  assert.match(html, /没有 SKILL\.md/, "要说清为什么，而不是让用户点了才知道");
  // 仍然在列表里：悄悄丢掉会让用户搜不到这个名字，以为市场坏了。
  assert.match(html, /awesome-claude-skill-notes/);
});

test("验过有 SKILL.md 的卡片：正常「安装」，并标出 SKILL.md 在哪个目录", () => {
  const html = marketCard(asServer(SEARCH_REAL_SKILL, "yes", "skills/pdf-filler"), { index: 0 });
  assert.match(html, /data-skfp-install="0"/);
  assert.match(html, /ctp-btn--primary/);
  assert.match(html, />安装</);
  assert.match(html, /skills\/pdf-filler/);
  assert.doesNotMatch(html, /未验证/);
});

test("还没验的卡片如实说没验，按钮是「检查并安装」而不是「安装」", () => {
  const html = marketCard(asServer(SEARCH_REAL_SKILL, "unknown"), { index: 1 });
  assert.match(html, /未验证/);
  // 按按钮本身断言：「检查并安装」这几个字在说明文案里也有一份，光 match 全文抓不到
  // 按钮标签有没有被换回「安装」。
  assert.match(html, /data-skfp-install="1"[^>]*>检查并安装</, "按钮标签必须是「检查并安装」");
  assert.doesNotMatch(html, /data-skfp-install="1"[^>]*>安装</, "没验过就不许说「安装」");
  assert.doesNotMatch(html, /ctp-btn--primary/, "没验过就不该是主按钮——它不是一条已确认能装的技能");
});

test("verdict 缺失/乱填一律按「还没验」处理，绝不当成能装", () => {
  for (const v of [undefined, "", "maybe", null, 1]) {
    const html = marketCard({ ...asServer(SEARCH_REAL_SKILL, v) }, { index: 2 });
    assert.match(html, /未验证/, `verdict=${String(v)} 应回落成未验证`);
    assert.doesNotMatch(html, />安装</);
  }
});

test("已安装 / 安装中仍然把按钮锁上", () => {
  assert.match(marketCard(asServer(SEARCH_REAL_SKILL, "yes"), { index: 0, installed: true }), /disabled/);
  assert.match(marketCard(asServer(SEARCH_REAL_SKILL, "yes"), { index: 0, installing: true }), /安装中/);
});

test("搜索结果落进列表时判据字段是空的——那一步没有任何能证明 SKILL.md 的字段", () => {
  const page = fnSource("_skillRegistryPage", { code: true });
  assert.match(page, /verdict:\s*"unknown"/, "别按 name/description 猜一个 yes 出来");
  assert.match(page, /_skillApplyVerdicts\(/, "已经判过的仓库要从缓存里盖回来，别重复花额度");
});

test("落点名按 SKILL.md 所在目录算，不是仓库名", () => {
  // 合集仓库装的是 skills/pdf-filler → 技能库目录叫 pdf-filler；按仓库名算会让「已安装」
  // 徽标永远不亮，用户重复点、反复覆盖。
  const render = fnSource("renderSkillsTool", { code: true });
  assert.match(render, /s\.skillDir\s*\?\s*s\.skillDir\.split\("\/"\)\.pop\(\)\s*:\s*s\.name/);
});

// ---------------------------------------------------------------------------
// 3. 限流：结论来自响应头，不来自状态码猜测
// ---------------------------------------------------------------------------

test("响应头两种形状都认（Tauri 回普通对象，webview fetch 回 Headers），大小写不敏感", () => {
  assert.equal(headerGet(RATE_LIMITED_HEADERS, "X-RateLimit-Remaining"), "0");
  const asHeaders = { get: (k) => (k === "x-ratelimit-remaining" ? "0" : null) };
  assert.equal(headerGet(asHeaders, "X-RateLimit-Remaining"), "0");
  assert.equal(headerGet(null, "x"), "");
  assert.equal(headerGet({}, "x"), "");
});

test("额度从响应头量出来：缺字段就是 null，不拿默认值冒充事实", () => {
  const now = RESET_MS - 12 * 60_000;
  const r = rateFromHeaders(RATE_LIMITED_HEADERS, now);
  assert.equal(r.remaining, 0);
  assert.equal(r.limit, 60);
  assert.equal(r.resetAt, RESET_MS, "x-ratelimit-reset 是 epoch 秒，要折成毫秒");
  assert.equal(r.retryAfterMs, 0);

  const bare = rateFromHeaders({}, now);
  assert.equal(bare.remaining, null);
  assert.equal(bare.limit, null);
  assert.equal(bare.resetAt, 0);

  // 只有 retry-after（429 常见）时，按「从现在起」折算重置点。
  const ra = rateFromHeaders({ "retry-after": "45" }, 1000);
  assert.equal(ra.retryAfterMs, 45_000);
  assert.equal(ra.resetAt, 46_000);

  // 非数字不许被 Number() 悄悄变成 NaN 塞进结论。
  assert.equal(rateFromHeaders({ "x-ratelimit-remaining": "unknown" }).remaining, null);
});

test("403 + x-ratelimit-remaining:0 → 限流，并算出还要等多久", () => {
  const now = RESET_MS - 12 * 60_000;
  const v = rateVerdict(403, RATE_LIMITED_HEADERS, now);
  assert.equal(v.rateLimited, true);
  assert.equal(v.waitMs, 12 * 60_000);
  assert.equal(v.resetAt, RESET_MS);
  assert.match(v.message, /GitHub 接口限流/);
  assert.match(v.message, /约 12 分钟后恢复/, "要说等多久——用户现在既不知道为什么、也不知道等多久");
  assert.match(v.message, /60/, "把额度是多少也摆出来");
});

test("429 + retry-after 也是限流", () => {
  const v = rateVerdict(429, { "retry-after": "90" }, 0);
  assert.equal(v.rateLimited, true);
  assert.match(v.message, /约 2 分钟后恢复/);
});

test("没有额度耗尽证据的 403 不许被说成限流——否则用户白等一小时", () => {
  const v = rateVerdict(403, FORBIDDEN_HEADERS, Date.now());
  assert.equal(v.rateLimited, false);
  assert.equal(v.message, "");
  // 404 / 500 更不是。
  assert.equal(rateVerdict(404, {}, Date.now()).rateLimited, false);
  assert.equal(rateVerdict(500, RATE_LIMITED_HEADERS, Date.now()).rateLimited, false);
});

test("等待时长的措辞按量级走，且不许出现负数", () => {
  assert.equal(waitText(0), "稍后自动");
  assert.equal(waitText(-5000), "稍后自动");
  assert.equal(waitText(30_000), "约 30 秒后");
  assert.equal(waitText(59_000), "约 59 秒后");
  assert.equal(waitText(60_000), "约 1 分钟后");
  assert.equal(waitText(3540_000), "约 59 分钟后");
});

test("拉取失败抛出的错误带结论：限流时是那句话 + retryAt，其余保持 HTTP <code>", () => {
  const now = RESET_MS - 60_000;
  const limited = httpError(403, RATE_LIMITED_HEADERS, now);
  assert.equal(limited.rateLimited, true);
  assert.equal(limited.status, 403);
  assert.equal(limited.retryAt, RESET_MS);
  assert.match(limited.message, /GitHub 接口限流/);
  assert.doesNotMatch(limited.message, /HTTP 403/, "「HTTP 403」不是用户能据以行动的信息");

  const plain = httpError(404, {}, now);
  assert.equal(plain.rateLimited, false);
  assert.equal(plain.message, "HTTP 404");
  assert.equal(plain.retryAt, 0);
});

test("两个额度桶分开记：/search/ 是 10 次/分钟，其余是核心 60 次/小时", () => {
  assert.equal(rateBucket("https://api.github.com/search/repositories?q=x"), "search");
  assert.equal(rateBucket("https://api.github.com/repos/a/b/git/trees/main?recursive=1"), "core");
  assert.equal(rateBucket("https://api.pulsemcp.com/v0beta/servers"), "core");
});

test("余额驱动退避：量到 0 就是 0，没量过是 null（不知道，别据此拦人）", () => {
  const { note, budget } = rateBudgetPair;
  const now = RESET_MS - 5 * 60_000;
  assert.equal(budget("core", { now }), null, "还没量过 → 不知道");

  note("core", RATE_LIMITED_HEADERS, now);
  assert.equal(budget("core", { now }), 0);
  // 重置点过了就当没量过——下一次请求会重新量，而不是永远认为没额度。
  assert.equal(budget("core", { now: RESET_MS + 1 }), null);

  // reserve 是留给用户主动动作（点安装）的头寸：后台批量校验不许把它花光。
  note("core", { "x-ratelimit-limit": "60", "x-ratelimit-remaining": "20", "x-ratelimit-reset": String(RESET_MS / 1000) }, now);
  assert.equal(budget("core", { now }), 20);
  assert.equal(budget("core", { reserve: 12, now }), 8);
  assert.equal(budget("core", { reserve: 50, now }), 0, "不许回负数");
});

test("不带额度头的响应不许覆盖已经量到的余额", () => {
  const { note, budget, state } = rateBudgetPair;
  const now = RESET_MS - 5 * 60_000;
  note("search", { "x-ratelimit-limit": "10", "x-ratelimit-remaining": "3", "x-ratelimit-reset": String(RESET_MS / 1000) }, now);
  assert.equal(budget("search", { now }), 3);
  note("search", { "content-type": "application/json" }, now);
  assert.equal(budget("search", { now }), 3, "PulseMCP 之类的响应不该把 GitHub 的余额抹掉");
  assert.equal(state().search.limit, 10);
});

// ---------------------------------------------------------------------------
// 4. 界面：限流时直说，并给重试入口
// ---------------------------------------------------------------------------

test("限流时界面直说「约 X 分钟后恢复」并留一个重试按钮", () => {
  const now = RESET_MS - 12 * 60_000;
  const msg = rateVerdict(403, RATE_LIMITED_HEADERS, now).message;
  const html = marketErrorHtml(msg, { retryAt: RESET_MS, now });
  assert.match(html, /约 12 分钟后恢复/);
  assert.match(html, /data-skfp="refresh"/, "要有重试入口");
  assert.match(html, />重试</);
  // 令牌是可选项，不是必须走的步骤；而且任何时候都不许把令牌本身印出来。
  assert.match(html, /访问令牌/);
  assert.doesNotMatch(html, /ghp_|github_pat_/);
});

test("不是限流的失败就不要编一个恢复时间出来", () => {
  const html = marketErrorHtml("HTTP 404", { retryAt: 0, now: Date.now() });
  assert.match(html, /HTTP 404/);
  assert.doesNotMatch(html, /分钟后|秒后/);
  assert.match(html, /data-skfp="refresh"/);
});

test("错误文案原样转义，不许把响应里的东西当 HTML 注进面板", () => {
  const html = marketErrorHtml('<img src=x onerror="alert(1)">', {});
  assert.doesNotMatch(html, /<img/);
  assert.match(html, /&lt;img/);
});

// ---------------------------------------------------------------------------
// 5. 机制在源码里真的接上了（不是写了个函数没人调）
// ---------------------------------------------------------------------------

test("两条拉取路径都把响应头交出去——不然 x-ratelimit-* 根本拿不到", () => {
  const fetchJson = fnSource("_mcpRegFetchJson", { code: true });
  // http_request 那条 IPC 路径本来就回 headers（net.rs 的 HttpResponse.headers），
  // 以前这里把它丢了。
  assert.match(fetchJson, /_ghRateNote\(bucket,\s*r\.headers\)/, "Tauri 那条路要记额度");
  assert.match(fetchJson, /_ghRateNote\(bucket,\s*res\.headers\)/, "webview fetch 那条路也要记");
  assert.match(fetchJson, /_ghHttpError\(r\.status,\s*r\.headers\)/);
  assert.match(fetchJson, /_ghHttpError\(res\.status,\s*res\.headers\)/);
  assert.doesNotMatch(fetchJson, /new Error\(`HTTP \$\{res\.status\}`\)/, "别再抛一个光秃秃的 HTTP 403");
});

test("Rust 侧确实回传响应头——这个判据的原料是那张表", async () => {
  const { readFileSync } = await import("node:fs");
  const { fileURLToPath } = await import("node:url");
  const net = readFileSync(new URL("../src-tauri/src/net.rs", import.meta.url), "utf8");
  void fileURLToPath;
  assert.match(net, /pub async fn http_request/);
  assert.match(net, /for \(k, v\) in resp\.headers\(\)\.iter\(\)/, "http_request 必须把整张响应头表带回来");
  assert.match(net, /headers: hmap/);
});

test("预取和批量校验都按余额退避，撞上限流就停", () => {
  const prefetch = fnSource("_skillRegPrefetch", { code: true });
  assert.match(prefetch, /_ghRateBudget\("search"/, "搜索桶只有 10 次/分钟，没剩下就别预取");

  const verify = fnSource("_skillVerifyPage", { code: true });
  assert.match(verify, /_ghRateBudget\("core"/, "花多少树请求由响应头里的剩余额度说了算");
  assert.match(verify, /_SKILL_VERIFY_RESERVE/, "要给用户的主动点击留头寸");
  assert.match(verify, /rateLimited\)\s*break/, "中途撞上限流立刻停，别把剩下的额度也撞光");
});

test("便宜那层走 raw CDN，不吃 api.github.com 的 60 次/小时", () => {
  const probe = fnSource("_skillProbeRootSkillMd", { code: true });
  assert.match(probe, /raw\.githubusercontent\.com/);
  assert.doesNotMatch(probe, /api\.github\.com/);
  // 没命中**不等于**没有（可能在子目录里）：这里只回"有没有命中"，不许直接判 no。
  assert.doesNotMatch(probe, /"no"/);
  const verify = fnSource("_skillVerifyPage", { code: true });
  assert.match(verify, /_skillProbeRootSkillMd/);
  assert.match(verify, /_skillPool\(/, "一页 30 条要走并发池，不是一起丢出去");
});

test("有现成令牌就用，但不许新增强制步骤，也不许把令牌写进文案", () => {
  const auth = fnSource("_ghAuthHeaders", { code: true });
  // 用的是设置里那份已有的 localStorage 令牌（和 @ 选仓库共用），没有就匿名跑。
  assert.match(auth, /_atIntegrationToken\("github"\)/);
  assert.match(auth, /if \(tok\)/, "没令牌要照常返回，不许在这里拦住整个市场");
  assert.match(auth, /api\\?\.github\\?\.com|raw\\?\.githubusercontent\\?\.com/, "只发给 GitHub 自己的域名");
  // 任何面向用户的文案里都不许出现令牌。
  const msg = fnSource("_ghRateLimitMessage", { code: true });
  assert.doesNotMatch(msg, /tok|Authorization|Bearer/);
  const errHtml = fnSource("_skillMarketErrorHtml", { code: true });
  assert.doesNotMatch(errHtml, /_ghAuthHeaders|_atIntegrationToken|Bearer/);
});

test("判据缓存分开存 yes/no 的有效期，且不许无限增长成一份陈年数据", () => {
  const ttl = load("_SKILL_VERDICT_TTL", ["_SKILL_VERDICT_TTL"]);
  assert.ok(ttl.yes > ttl.no, "no 要存得短一点：仓库随时可能补上 SKILL.md");
  assert.ok(ttl.no <= 24 * 3600_000);
  const loadFn = fnSource("_skillVerdictsLoad", { code: true });
  assert.match(loadFn, /_SKILL_VERDICT_TTL\[v\.v\]/, "读回来要按各自的有效期筛一遍");
});

test("市场标题栏说清判据，别让用户以为一排热门都能装", () => {
  assert.match(CODE, /只有确认含 SKILL\.md 的才给「安装」/);
});

// ---------------------------------------------------------------------------
// 6. 真跑一遍编排：不打网络，夹具喂到底
// ---------------------------------------------------------------------------

// 整条校验链的依赖清单。localStorage / _skillFetchText（raw CDN）/ _mcpRegFetchJson
// （api.github.com）全从外面注入，所以一次真实请求都不会发出去。
const VERIFY_CHAIN = [
  "_ghHeaderGet", "_ghRateFromHeaders", "_ghRate", "_ghRateNote", "_ghRateBudget",
  "_SKILL_VERDICT_KEY", "_SKILL_VERDICT_TTL", "_skillVerdicts",
  "_skillVerdictKey", "_skillVerdictsLoad", "_skillVerdictsSave",
  "_skillVerdictGet", "_skillVerdictSet", "_skillApplyVerdicts",
  "_skillPickDirFromTree", "_skillProbeRootSkillMd", "_skillProbeTree", "_skillPool",
  "_SKILL_VERIFY_CONCURRENCY", "_SKILL_VERIFY_RESERVE", "_SKILL_VERIFY_TREE_MAX",
  "_skillVerifyPage",
];

function buildVerify({ localStorage, fetchText, fetchJson }) {
  const body = VERIFY_CHAIN.map(fnSource).join("\n");
  return new Function("localStorage", "_skillFetchText", "_mcpRegFetchJson",
    `let _skillVerdictsLoaded = false;\n${body}\n;return { verify: _skillVerifyPage, note: _ghRateNote, budget: _ghRateBudget, dump: () => localStorage.getItem(_SKILL_VERDICT_KEY) };`
  )(localStorage, fetchText, fetchJson);
}

function makeVerifyHarness({ rateHeaders = null, treeThrows = null } = {}) {
  const calls = { raw: [], api: [] };
  const store = new Map();
  const localStorage = {
    getItem: (k) => (store.has(k) ? store.get(k) : null),
    setItem: (k, v) => store.set(k, String(v)),
  };
  // 只有这一个仓库根目录上有 SKILL.md；其余靠拉树才能有结论。
  const ROOT_SKILL_REPOS = new Set(["acme/pdf-filler"]);
  const TREES = {
    "someone/awesome-claude-skill-notes": TREE_NO_SKILL,
    "org/skill-pack": TREE_COLLECTION,
    "org/another-pack": TREE_COLLECTION,
  };
  const fetchText = async (url) => {
    calls.raw.push(url);
    const m = String(url).match(/raw\.githubusercontent\.com\/([^/]+\/[^/]+)\//);
    if (m && ROOT_SKILL_REPOS.has(m[1])) return "---\nname: pdf-filler\n---\n正文";
    throw new Error("HTTP 404");
  };
  const fetchJson = async (url) => {
    calls.api.push(url);
    if (treeThrows) throw treeThrows();
    const m = String(url).match(/repos\/([^/]+\/[^/]+)\/git\/trees/);
    const t = m && TREES[m[1]];
    if (!t) throw Object.assign(new Error("HTTP 404"), { status: 404 });
    return t;
  };
  const api = buildVerify({ localStorage, fetchText, fetchJson });
  if (rateHeaders) api.note("core", rateHeaders, Date.now());
  return { ...api, calls, store };
}

const samplePage = () => ([
  { name: "pdf-filler", full: "acme/pdf-filler", branch: "main", url: "https://github.com/acme/pdf-filler" },
  { name: "awesome-claude-skill-notes", full: "someone/awesome-claude-skill-notes", branch: "master", url: "https://github.com/someone/awesome-claude-skill-notes" },
  { name: "skill-pack", full: "org/skill-pack", branch: "main", url: "https://github.com/org/skill-pack" },
]);

test("跑完一页：能装的判 yes、只是在讨论的判 no、三条都还在列表里", async () => {
  const h = makeVerifyHarness();
  const list = await h.verify(samplePage());
  assert.deepEqual(list.map((s) => s.verdict), ["yes", "no", "yes"]);
  assert.equal(list[0].skillDir, "", "根目录 SKILL.md");
  // 合集仓库里没有和仓库同名的目录，按规则落到第一个（skills/docx）——重点是它指到了
  // 一个**真的有 SKILL.md** 的目录，而不是仓库根。
  assert.equal(list[2].skillDir, "skills/docx", "合集仓库要指到具体那个目录");
  assert.equal(list.length, 3, "装不了的不许从列表里消失");
  // 根目录命中的那条**没有**花 API 额度：它走的是 raw CDN。
  assert.ok(!h.calls.api.some((u) => u.includes("acme/pdf-filler")), "根目录命中就不该再拉树");
  assert.equal(h.calls.api.length, 2, "一页 3 条只花了 2 次核心额度，不是 3 次");
});

test("判过的仓库不再判：第二次一次请求都不发", async () => {
  const h = makeVerifyHarness();
  await h.verify(samplePage());
  const rawBefore = h.calls.raw.length;
  const apiBefore = h.calls.api.length;
  const again = await h.verify(samplePage());
  assert.deepEqual(again.map((s) => s.verdict), ["yes", "no", "yes"]);
  assert.equal(h.calls.raw.length, rawBefore, "缓存命中就别再探根目录");
  assert.equal(h.calls.api.length, apiBefore, "更别再拉一次树");
  assert.match(String(h.dump()), /awesome-claude-skill-notes/, "判据要落盘，翻页/重开都算数");
});

test("额度快见底时后台不再拉树，条目停在「未验证」而不是编一个结论", async () => {
  // 剩 12 次、reserve 正好 12 → 后台的树请求预算为 0。
  const h = makeVerifyHarness({
    rateHeaders: {
      "x-ratelimit-limit": "60",
      "x-ratelimit-remaining": "12",
      "x-ratelimit-reset": String(Math.floor(Date.now() / 1000) + 900),
    },
  });
  const list = await h.verify(samplePage());
  assert.equal(list[0].verdict, "yes", "raw CDN 那层不吃 API 额度，照常跑");
  assert.deepEqual(list.slice(1).map((s) => s.verdict), ["unknown", "unknown"]);
  assert.equal(h.calls.api.length, 0, "预算为 0 就一次树请求都不发");
});

test("撞上限流当场停手，不把剩下的额度也撞光", async () => {
  const h = makeVerifyHarness({
    treeThrows: () => Object.assign(new Error("GitHub 接口限流，约 30 分钟后恢复。"), { rateLimited: true, status: 403 }),
  });
  const list = await h.verify(samplePage());
  assert.equal(h.calls.api.length, 1, "第一次就撞上限流，剩下的树请求全部不发");
  assert.equal(list[0].verdict, "yes");
  assert.deepEqual(list.slice(1).map((s) => s.verdict), ["unknown", "unknown"], "撞了就停在未验证，不编结论");
});


test("装不了的条目必须留在列表里——丢掉会让用户搜不到，以为市场坏了", async () => {
  // 跑真的 _skillRegistryPage：GitHub 搜索响应是夹具，一次网络都不打。
  const store = new Map();
  const localStorage = {
    getItem: (k) => (store.has(k) ? store.get(k) : null),
    setItem: (k, v) => store.set(k, String(v)),
  };
  const SEARCH_RESPONSE = {
    total_count: 2,
    items: [SEARCH_NOT_A_SKILL, SEARCH_REAL_SKILL],
  };
  const chain = [
    "_SKILL_VERDICT_KEY", "_SKILL_VERDICT_TTL", "_skillVerdicts",
    "_skillVerdictKey", "_skillVerdictsLoad", "_skillVerdictsSave",
    "_skillVerdictGet", "_skillVerdictSet", "_skillApplyVerdicts",
    "_SKILL_REG_CACHE_KEY", "_SKILL_REG_PAGE_SIZE", "_skillRegPageCache", "_skillRegInflight",
    "_skillRegPageKey", "_skillRegistryPage",
  ].map(fnSource).join("\n");
  const registryPage = new Function("localStorage", "_mcpRegFetchJson",
    `let _skillVerdictsLoaded = false;\n${chain}\n;return _skillRegistryPage;`
  )(localStorage, async () => SEARCH_RESPONSE);

  const { servers } = await registryPage("pdf", 1);
  assert.equal(servers.length, 2, "两条都要在，包括那条只是在讨论 Claude 技能的");
  assert.deepEqual(servers.map((s) => s.full), ["someone/awesome-claude-skill-notes", "acme/pdf-filler"]);
  // 判据这一步还没有任何证据，所以两条都是 unknown——不是 yes，也不是被删掉。
  assert.deepEqual(servers.map((s) => s.verdict), ["unknown", "unknown"]);

  // 判过一次 no 之后，这条**依然**在列表里，只是判据变了。
  const store2 = new Map();
  const ls2 = { getItem: (k) => (store2.has(k) ? store2.get(k) : null), setItem: (k, v) => store2.set(k, String(v)) };
  const built = new Function("localStorage", "_mcpRegFetchJson",
    `let _skillVerdictsLoaded = false;\n${chain}\n;return { page: _skillRegistryPage, set: _skillVerdictSet };`
  )(ls2, async () => SEARCH_RESPONSE);
  built.set("someone/awesome-claude-skill-notes", "master", "no", "");
  const after = await built.page("pdf", 1);
  assert.equal(after.servers.length, 2, "标成「非技能包」不等于从列表里删掉");
  assert.equal(after.servers[0].verdict, "no");
});
