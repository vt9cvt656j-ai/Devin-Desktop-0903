// MCP 市场：可安装性判据、装不了的条目怎么呈现、装错了怎么自愈、失败怎么解释。
//
// 用户报的是：「我安装了 MCP 后 这个 MCP 连接失败了 很多都是这样」。
//
// 根因不是网络，是判据。市场的主数据源 PulseMCP（api.pulsemcp.com/v0beta/servers）
// 每条结果都带一个顶层 `url`，那是**它自己的目录页**（https://www.pulsemcp.com/servers/<slug>），
// 每条都有，和这个服务有没有远程端点毫无关系；真端点只在 `remotes[]` 里。而归一化那一步
// 写的是「有 url 且没有 package_name 就当远程」，于是：
//
//     { "command": "npx", "args": ["-y", "--", "mcp-remote", "https://www.pulsemcp.com/servers/0xkoda-ethereum-rpc"] }
//
// mcp-remote 连过去拿回一张 HTML 网页，握手必然失败。实测热度前 100 条里，23 条有包、
// 13 条有真远程端点、66 条两者皆无——那 66 条**全部**会被写成这样一条假 remote。
//
// 这组用例跑的是 main.js 里的真函数（helpers/source.mjs 按 AST 边界抠出来注入依赖），
// 且**不打真实网络**：PulseMCP 的响应形状写成夹具（字段清单取自真实 API）。
import test from "node:test";
import assert from "node:assert/strict";
import { load, fnSource, CODE } from "./helpers/source.mjs";
import { escapeAttr as _escAttr, escapeHtml as _escHtml } from "../src/agent/escape.js";

// ---------------------------------------------------------------------------
// 夹具：api.pulsemcp.com/v0beta/servers 一条结果的真实字段
// ---------------------------------------------------------------------------
// name, url, external_url, source_code_url, package_name, package_registry,
// remotes, github_stars, short_description, package_download_count,
// EXPERIMENTAL_ai_generated_description
const PULSE_WITH_PACKAGE = {
  name: "Filesystem",
  url: "https://www.pulsemcp.com/servers/modelcontextprotocol-filesystem",
  external_url: null,
  source_code_url: "https://github.com/modelcontextprotocol/servers/tree/main/src/filesystem",
  package_name: "@modelcontextprotocol/server-filesystem",
  package_registry: "npm",
  remotes: [],
  github_stars: 62000,
  short_description: "读写本机文件",
  package_download_count: 900000,
  EXPERIMENTAL_ai_generated_description: null,
};

const PULSE_WITH_REMOTE = {
  name: "DeepWiki",
  url: "https://www.pulsemcp.com/servers/deepwiki",
  external_url: "https://deepwiki.com",
  source_code_url: null,
  package_name: null,
  package_registry: null,
  remotes: [{ url: "https://mcp.deepwiki.com/mcp", transport_type: "streamable-http" }],
  github_stars: 0,
  short_description: "读公开 GitHub 仓库的文档",
  package_download_count: null,
  EXPERIMENTAL_ai_generated_description: null,
};

// 那 66 条的形状：只有源码，没有包、没有 remotes——但**有** url（目录页）。
const PULSE_SOURCE_ONLY = {
  name: "ethereum-rpc",
  url: "https://www.pulsemcp.com/servers/0xkoda-ethereum-rpc",
  external_url: null,
  source_code_url: "https://github.com/0xkoda/ethereum-rpc",
  package_name: null,
  package_registry: null,
  remotes: [],
  github_stars: 3,
  short_description: "以太坊 JSON-RPC",
  package_download_count: null,
  EXPERIMENTAL_ai_generated_description: "由 AI 生成的说明",
};

const normalizePulse = load("_mcpRegNormalizePulse", ["_mcpIsDirectoryPageUrl", "_MCP_DIRECTORY_PAGE_PATTERNS", "_mcpPickRemoteEndpoint", "_mcpRegNormalizePulse"]);
const normalizeOfficial = load("_mcpRegNormalizeOfficial", ["_mcpIsDirectoryPageUrl", "_MCP_DIRECTORY_PAGE_PATTERNS", "_mcpPickRemoteEndpoint", "_mcpRegNormalizeOfficial"]);
const installable = load("_mcpRegInstallable", ["_mcpRegInstallable"]);
const isDirectoryPage = load("_mcpIsDirectoryPageUrl", ["_MCP_DIRECTORY_PAGE_PATTERNS", "_mcpIsDirectoryPageUrl"]);
const toConfig = load("_mcpRegToConfig", [
  "_MCP_PKG_PATTERNS", "_mcpSafePkgId", "_MCP_DIRECTORY_PAGE_PATTERNS", "_mcpIsDirectoryPageUrl", "_mcpRegToConfig",
]);
const configRemoteUrl = load("_mcpConfigRemoteUrl", ["_mcpConfigRemoteUrl"]);
const spawnErrno = load("_mcpSpawnErrno", ["_mcpSpawnErrno"]);
const diagnose = load("_mcpDiagnoseFailure", [
  "_MCP_DIRECTORY_PAGE_PATTERNS", "_mcpIsDirectoryPageUrl", "_mcpConfigRemoteUrl",
  "_mcpConfigPackage", "_mcpSpawnErrno", "_mcpDiagnoseFailure",
]);

// ---------------------------------------------------------------------------
// 1. 判据换成真实字段
// ---------------------------------------------------------------------------

test("PulseMCP 的 url 是目录页，绝不能被当成远程 MCP 端点", () => {
  const s = normalizePulse(PULSE_SOURCE_ONLY);
  // 这一条就是用户 ~/.mrdayone/mcp.json 里躺着的那条的来源。
  assert.equal(s.remote, "", "只有源码的条目不许产生 remote");
  assert.equal(s.pkg, null);
  assert.equal(installable(s), false, "既没有包也没有真端点 = 装不了");
  // 目录页仍然留着，但只作为「查看条目」的链接。
  assert.equal(s.listing, PULSE_SOURCE_ONLY.url);
  assert.equal(s.repo, "https://github.com/0xkoda/ethereum-rpc");
});

test("真远程端点只从 remotes[] 里取，传输方式一并带上", () => {
  const s = normalizePulse(PULSE_WITH_REMOTE);
  assert.equal(s.remote, "https://mcp.deepwiki.com/mcp");
  assert.equal(s.remoteTransport, "streamable-http");
  assert.equal(installable(s), true);
  assert.notEqual(s.remote, PULSE_WITH_REMOTE.url);
});

test("有包的条目走包，url 一样不参与", () => {
  const s = normalizePulse(PULSE_WITH_PACKAGE);
  assert.deepEqual(s.pkg, { kind: "npm", id: "@modelcontextprotocol/server-filesystem" });
  assert.equal(s.remote, "");
  assert.equal(installable(s), true);
});

test("网页地址（external_url / source_code_url）也不许变成端点", () => {
  const s = normalizePulse({
    ...PULSE_SOURCE_ONLY,
    external_url: "https://example.com/docs/ethereum",
  });
  assert.equal(s.remote, "");
});

test("remotes[] 里混进目录页 / 非 https 也要被跳过，取下一条真端点", () => {
  const pick = load("_mcpPickRemoteEndpoint", ["_MCP_DIRECTORY_PAGE_PATTERNS", "_mcpIsDirectoryPageUrl", "_mcpPickRemoteEndpoint"]);
  assert.deepEqual(pick([
    { url: "https://www.pulsemcp.com/servers/foo", transport_type: "streamable-http" },
    { url: "ftp://nope", transport_type: "sse" },
    { url: "https://real.example.com/mcp", transport_type: "sse" },
  ]), { url: "https://real.example.com/mcp", transport: "sse" });
  assert.deepEqual(pick(undefined), { url: "", transport: "" });
  assert.deepEqual(pick([{ transport_type: "sse" }]), { url: "", transport: "" });
});

test("官方注册表那一支也走同一个取端点函数（transport 字段名不同）", () => {
  const s = normalizeOfficial({
    server: {
      name: "io.github.acme/thing",
      description: "x",
      packages: [],
      remotes: [{ url: "https://mcp.acme.com/mcp", transport: "sse" }],
    },
  });
  assert.equal(s.remote, "https://mcp.acme.com/mcp");
  assert.equal(s.remoteTransport, "sse");
});

test("目录站判据认整个 pulsemcp.com 的条目页，且只认能证明的那一个站", () => {
  assert.equal(isDirectoryPage("https://www.pulsemcp.com/servers/0xkoda-ethereum-rpc"), true);
  assert.equal(isDirectoryPage("https://pulsemcp.com/servers/foo"), true);
  assert.equal(isDirectoryPage("http://www.pulsemcp.com/servers/foo?x=1"), true);
  // 不猜别的站：一个真端点被误判成网页比现在这个 bug 更糟。
  assert.equal(isDirectoryPage("https://mcp.deepwiki.com/mcp"), false);
  assert.equal(isDirectoryPage("https://api.pulsemcp.com/v0beta/servers"), false);
  assert.equal(isDirectoryPage(""), false);
});

// ---------------------------------------------------------------------------
// 2. 装不了的条目：不给一个必然失败的「安装」按钮
// ---------------------------------------------------------------------------

test("只有源码的条目转不出启动配置——宁可装不了，也不写一条必然失败的", () => {
  assert.equal(toConfig(normalizePulse(PULSE_SOURCE_ONLY)), null);
  assert.deepEqual(toConfig(normalizePulse(PULSE_WITH_PACKAGE)), {
    command: "npx", args: ["-y", "--", "@modelcontextprotocol/server-filesystem"],
  });
  // 传输方式不从注册表声明里钉死：mcp-remote 默认 http-first，声明过时时它还能自己退回
  // SSE；钉住等于拿一份第三方数据换掉这条自愈路径。
  assert.deepEqual(toConfig(normalizePulse(PULSE_WITH_REMOTE)), {
    command: "npx", args: ["-y", "--", "mcp-remote", "https://mcp.deepwiki.com/mcp"],
  });
});

test("注册表声明的传输方式记进已装元信息，但不参与拼启动参数", () => {
  const meta = load("_mcpInstallMetaFromRegistry", ["_mcpInstallMetaFromRegistry"])(normalizePulse(PULSE_WITH_REMOTE), "PulseMCP");
  assert.equal(meta.remoteTransport, "streamable-http");
  assert.equal(meta.badge, "remote");
});

test("即使上游把目录页塞进 remote 字段，也不许写出 mcp-remote <目录页>", () => {
  assert.equal(toConfig({ pkg: null, remote: "https://www.pulsemcp.com/servers/0xkoda-ethereum-rpc" }), null);
});

const marketCard = load("_mcpMarketCardHtml", {
  _mcpRegInstallable: installable,
  _MCP_SOURCE_ONLY_NOTE: load("_MCP_SOURCE_ONLY_NOTE", ["_MCP_SOURCE_ONLY_NOTE"]),
  _mcpRegIconHtml: () => "<span></span>",
  _mcpStarsText: (n) => String(n),
  _dbUiIconSvg: () => "<svg></svg>",
  _escAttr,
  _escHtml,
});

test("只有源码的卡片：标出来、给「查看仓库」，不给「安装」", () => {
  const html = marketCard(normalizePulse(PULSE_SOURCE_ONLY), { index: 7 });
  assert.doesNotMatch(html, /data-mcpfp-install/, "装不了就不许出现安装按钮");
  assert.match(html, /查看仓库/);
  assert.match(html, /data-mcpfp-repo="https:\/\/github\.com\/0xkoda\/ethereum-rpc"/);
  assert.match(html, /仅源码/);
  // 说清为什么：用户现在的体验是「点安装 → 看着装上了 → 连接失败 → 不知道为什么」。
  assert.match(html, /没法一键装/);
  assert.match(html, /添加服务/);
});

test("装得了的卡片照旧有安装按钮，索引原样带过去", () => {
  const html = marketCard(normalizePulse(PULSE_WITH_REMOTE), { index: 3 });
  assert.match(html, /data-mcpfp-install="3"/);
  assert.doesNotMatch(html, /查看仓库<\/button>/);
  const done = marketCard(normalizePulse(PULSE_WITH_PACKAGE), { index: 0, installed: true });
  assert.match(done, /已安装/);
  assert.match(done, /disabled/);
});

test("装不了的条目留在列表里（能搜到），不是被悄悄丢掉", () => {
  // 过滤条件写在 _mcpRegistryPage 里，靠网络才跑得动；这里守它的判据形状：
  // 唯一的门槛是「至少能指向点什么」，而不是「装得了」。
  const page = fnSource("_mcpRegistryPage", { code: true });
  assert.match(page, /_mcpRegInstallable\(s\) \|\| s\.repo \|\| s\.listing/);
  assert.doesNotMatch(page, /filter\(\(s\) => s\.name && \(s\.pkg \|\| s\.remote\)\)/);
});

// ---------------------------------------------------------------------------
// 3. 已经装进去的坏条目要能自愈
// ---------------------------------------------------------------------------

test("从已装配置里取回真实远程地址：mcp-remote 垫片和原生 url 两种写法都认", () => {
  assert.equal(
    configRemoteUrl({ command: "npx", args: ["-y", "--", "mcp-remote", "https://www.pulsemcp.com/servers/0xkoda-ethereum-rpc"] }),
    "https://www.pulsemcp.com/servers/0xkoda-ethereum-rpc",
  );
  assert.equal(configRemoteUrl({ url: "https://mcp.deepwiki.com/mcp" }), "https://mcp.deepwiki.com/mcp");
  assert.equal(configRemoteUrl({ command: "npx", args: ["-y", "--", "some-pkg"] }), "", "本地包不是远程服务");
  assert.equal(configRemoteUrl(null), "");
});

test("指向目录页的那条：诊断直说装错了、建议删除，而不是丢一串底层错误", () => {
  const d = diagnose({
    error: "Error POSTing to endpoint (HTTP 200): <!DOCTYPE html><html lang=\"en\">…",
    config: { command: "npx", args: ["-y", "--", "mcp-remote", "https://www.pulsemcp.com/servers/0xkoda-ethereum-rpc"] },
  });
  assert.equal(d.kind, "directory-page");
  assert.equal(d.action, "delete", "这条配置本身就是错的，留着不会自己好");
  assert.match(d.message, /不是 MCP 端点/);
  assert.match(d.message, /删除/);
  assert.match(d.message, /pulsemcp\.com\/servers\/0xkoda-ethereum-rpc/);
  // 结论必须在最前面：这些串会被 _mcpFailureSystemContext 截到 180 字节喂给模型。
  assert.doesNotMatch(d.message.slice(0, 40), /DOCTYPE|HTTP 200/);
});

test("目录页这条判据不需要联网——探测都还没跑就该给出结论", () => {
  const d = diagnose({
    error: "x",
    config: { url: "https://pulsemcp.com/servers/whatever" },
    probe: null,
  });
  assert.equal(d.kind, "directory-page");
});

// ---------------------------------------------------------------------------
// 4. 失败诊断说人话，判据是执行事实
// ---------------------------------------------------------------------------

test("HTML 响应 → 这个地址不是 MCP 端点", () => {
  const d = diagnose({
    error: "unexpected token < in JSON",
    config: { url: "https://example.com/docs" },
    probe: { reached: true, status: 200, contentType: "text/html; charset=utf-8", authHeader: false },
  });
  assert.equal(d.kind, "not-endpoint");
  assert.equal(d.action, "delete");
  assert.match(d.message, /HTML 网页/);
});

test("401 / WWW-Authenticate → 需要鉴权（不是「地址错了」）", () => {
  const byStatus = diagnose({
    error: "",
    config: { url: "https://mcp.acme.com/mcp" },
    probe: { reached: true, status: 401, contentType: "application/json", authHeader: false },
  });
  assert.equal(byStatus.kind, "auth");
  assert.equal(byStatus.action, "configure");

  const byHeader = diagnose({
    error: "",
    config: { url: "https://mcp.acme.com/mcp" },
    probe: { reached: true, status: 400, contentType: "application/json", authHeader: true },
  });
  assert.equal(byHeader.kind, "auth");

  const forbidden = diagnose({
    error: "",
    config: { url: "https://mcp.acme.com/mcp" },
    probe: { reached: true, status: 403, contentType: "application/json", authHeader: false },
  });
  assert.equal(forbidden.kind, "auth");
  assert.match(forbidden.message, /凭据/);
});

test("404 / 405 → 这个地址上没有 MCP 端点", () => {
  for (const status of [404, 405, 410]) {
    const d = diagnose({
      error: "",
      config: { url: "https://mcp.acme.com/mcp" },
      probe: { reached: true, status, contentType: "text/plain", authHeader: false },
    });
    assert.equal(d.kind, "not-endpoint", `HTTP ${status}`);
  }
});

test("请求根本没发出去 → 网络不通，重试有意义", () => {
  const d = diagnose({
    error: "fetch failed",
    config: { url: "https://mcp.acme.com/mcp" },
    probe: { reached: false, status: 0, contentType: "", authHeader: false, error: "dns error: failed to lookup address" },
  });
  assert.equal(d.kind, "network");
  assert.equal(d.action, "retry");
  assert.match(d.message, /网络/);
});

test("包注册表回 404 → 包不存在，不是网络问题", () => {
  const d = diagnose({
    error: "npm ERR! 404",
    config: { command: "npx", args: ["-y", "--", "no-such-mcp-pkg"] },
    pkgProbe: { reached: true, status: 404 },
  });
  assert.equal(d.kind, "missing-package");
  assert.equal(d.action, "delete");
  assert.match(d.message, /包不存在/);
  assert.match(d.message, /no-such-mcp-pkg/);

  const ok = diagnose({
    error: "npm ERR! 404",
    config: { command: "npx", args: ["-y", "--", "no-such-mcp-pkg"] },
    pkgProbe: { reached: true, status: 200 },
  });
  assert.notEqual(ok.kind, "missing-package", "包在，就不许说包不存在");
});

test("spawn 的 errno 是数字，不是那句会随系统语言变的描述", () => {
  assert.equal(spawnErrno("启动 MCP 服务失败（uvx）: No such file or directory (os error 2)"), 2);
  assert.equal(spawnErrno("启动 MCP 服务失败（uvx）: 系统找不到指定的文件。 (os error 2)"), 2);
  assert.equal(spawnErrno("permission denied (os error 13)"), 13);
  assert.equal(spawnErrno("MCP 连接超时（initialize）"), 0);

  const d = diagnose({ error: "启动 MCP 服务失败（uvx）: No such file or directory (os error 2)", config: { command: "uvx", args: ["--", "mcp-thing"] } });
  assert.equal(d.kind, "missing-command");
  assert.match(d.message, /uvx/);
  assert.match(d.message, /PATH/);
});

test("分类不许 grep 错误文案：探测说端点是好的，就不能因为串里写着 401/Not Found 而改判", () => {
  // 这是「靠关键词猜」和「靠执行事实判」的分水岭。底层错误串来自 mcp-remote / npx
  // 这些第三方进程，换个版本措辞就变；它里面出现什么词都不该改变结论。
  const noisy = "Error: 401 Unauthorized / 404 Not Found / ENOTFOUND / html";
  const d = diagnose({
    error: noisy,
    config: { url: "https://mcp.acme.com/mcp" },
    probe: { reached: true, status: 200, contentType: "application/json", authHeader: false },
  });
  assert.equal(d.kind, "unknown");
  assert.equal(d.action, "");
  assert.equal(d.message, noisy, "判不出来就原样把底层错误交出去，不编");
});

test("装不了 / 转不出配置时，安装按钮要说出理由，不是静默 return", () => {
  // 原来是 `if (!s || !conf) return;`：按钮按下去什么都不发生，比装错还费解。
  const at = CODE.indexOf('const idx = e.target.closest("[data-mcpfp-install]")');
  assert.ok(at > 0, "找不到市场安装处理器");
  const handler = CODE.slice(at, at + 1200);
  assert.doesNotMatch(handler, /if \(!s \|\| !conf\) return;/, "又回到静默 return 了");
  assert.match(handler, /if \(!conf\) \{[\s\S]*showToast\([\s\S]*_MCP_SOURCE_ONLY_NOTE/);
});

test("诊断结论要真的进面板和模型上下文，不是算完就丢", () => {
  const ensure = fnSource("_ensureMcpTools", { code: true });
  assert.match(ensure, /_mcpDiagnoseFailure\(/, "失败分支必须走诊断");
  assert.match(ensure, /_mcpFailures\.set\(serverName, \(diag\.message/,
    "存进 _mcpFailures 的必须是诊断后的那句话——面板和 _mcpFailureSystemContext 读的都是它");

  // 探测只在**失败之后**跑：正常路径一次网络都不许多花。
  const beforeCatch = ensure.slice(0, ensure.indexOf("} catch (error) {"));
  assert.doesNotMatch(beforeCatch, /_mcpProbeRemoteEndpoint|_mcpProbePackage/,
    "探测出现在了成功路径上");
  assert.match(ensure.slice(ensure.indexOf("} catch (error) {")), /_mcpProbeRemoteEndpoint/);

  // 失败那一句要看得见，不能只活在状态徽章的 title 里。
  const at = CODE.indexOf("const failure = live ? (_mcpFailures.get(name)");
  assert.ok(at > 0, "找不到已装卡片里读 _mcpFailures 的那一段");
  assert.match(CODE.slice(at, at + 3000), /\$\{failure \? `<p class="mcpfp-card__err"/);
});
