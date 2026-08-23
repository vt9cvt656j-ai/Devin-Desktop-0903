/**
 * 用户声明的能力：把用户自己填的一段 JSON，变成 IDE 真正拥有的工具和命令。
 *
 * # 这是在解决什么
 *
 * 这个 IDE 对模型很开放，对**用户**是关死的：132 个工具是 `_buildAgentToolSchemas` 里
 * 一个手写的数组字面量，斜杠命令是三条字面量。用户想加一个自家能力——「查我们内网那个
 * 工单接口」——今天唯一的路是写一个 MCP server 并部署它。为了一次 HTTP GET 去搭一个
 * 常驻进程，成本高到没人会做；而且 MCP 那条链背着进程会话、按 root 隔离、连接状态检查，
 * 一个纯 HTTP 调用背不动这些。
 *
 * 另一半同样缺：**关不掉**。今天"禁用"只发生在执行前的检查点，schema 照发给模型、模型
 * 照调一次、再拿回一句"用户已拒绝"——白烧一轮，而且规则是按内部类型名匹配的，用户写
 * `web_search` 根本不生效（内部叫 `websearch`）。
 *
 * # 模型
 *
 * 一段声明 = 一个能力。声明放在已有的多作用域配置里（HOME 的个人配置、仓库里的项目
 * 配置），加一段就多一个能力，删掉那段就没了——不改代码、不发版、不重启。
 *
 * 本模块是纯数据 + 纯函数，没有 DOM、没有 I/O、没有 import，所以它的测试直接 `import`
 * 它来跑，而不是从 main.js 里按名字抠源码。
 *
 * # 边界
 *
 * 声明可以来自**仓库里**的配置文件，也就是说 clone 一个别人的仓库就可能带进来一条声明。
 * 因此：
 *   - 编译出来的工具一律要审批（和 MCP 同级），审批框里带上它来自哪个文件；
 *   - URL 只允许 http/https，参数插值一律 URL 编码，不允许拼出别的协议；
 *   - `disabled`（关掉某个内置工具）任何作用域都可以写——收紧永远放行，这和权限规则
 *     那套的方向一致。
 */

/** 用户工具在模型那边的名字前缀。有它才能一眼分出「这是用户自己加的」。 */
export const USER_TOOL_PREFIX = "user__";

/** 工具名：小写字母开头，字母数字下划线，2–48 字符。 */
const NAME_RE = new RegExp("^[a-z][a-z0-9_]{1,47}$");
/** 参数名：同样的字符集，但允许**单字符**——`q=` 是查询参数最自然的写法，
 *  拿工具名那条规则去卡参数名，等于逼用户把 `q` 写成 `query`。 */
const PARAM_NAME_RE = new RegExp("^[a-z][a-z0-9_]{0,47}$", "i");
/** 斜杠命令名：允许连字符，因为 /deploy-staging 这种写法很自然。 */
const CMD_RE = new RegExp("^[a-z][a-z0-9_-]{0,31}$");
/** `${VAR}` 占位符。写成 RegExp 而不是字面量，避免大括号打断按花括号计数的源码抽取器。 */
const PLACEHOLDER_RE = new RegExp("\\$\\{([A-Za-z_][A-Za-z0-9_]*)\\}", "g");
/** `{param}` —— URL 和 body 里插模型给的参数。 */
const PARAM_RE = new RegExp("\\{([A-Za-z_][A-Za-z0-9_]*)\\}", "g");

const ALLOWED_METHODS = new Set(["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"]);
const ALLOWED_PARAM_TYPES = new Set(["string", "number", "integer", "boolean"]);

/** 允许一次声明多少条，纯粹是防止一个坏文件把整个工具窗口挤爆。 */
export const MAX_TOOLS = 64;
export const MAX_COMMANDS = 64;
export const MAX_ROLES = 32;
/** 官方文档域名白名单的条数上限。同上，纯粹防一个坏文件把判定表撑爆。 */
export const MAX_DOC_HOSTS = 64;

const str = (v) => (typeof v === "string" ? v.trim() : "");

/** 主机名：字母数字段用点连接，最后一段是字母（TLD）。故意不认端口和路径。 */
const HOST_RE = new RegExp("^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*\\.[a-z]{2,}$");

/**
 * 一条用户写的「这个站是官方文档」→ 归一化的主机名。
 *
 * 用户会两种写法都用：`docs.acme.com` 和 `https://docs.acme.com/api/v2`。两种都收，
 * 统一成裸主机名并去掉 `www.`——判定那侧比对的就是这个形状。认不出来返回空串。
 */
export function normalizeDocsHost(value) {
  let s = str(value).toLowerCase();
  if (!s) return "";
  if (s.includes("://")) {
    try { s = new URL(s).hostname; } catch { return ""; }
  } else {
    s = s.split("/")[0].split("?")[0].split("#")[0];
    if (s.includes("@")) return "";        // 邮箱之类的，不是主机名
    if (s.includes(":")) s = s.split(":")[0]; // 顺手容忍 host:port
  }
  s = s.replace(/^www\./, "").replace(/\.$/, "");
  return HOST_RE.test(s) ? s : "";
}

/**
 * 项目栈覆盖：用户直接告诉我们这个项目怎么编、怎么测、测试放哪。
 *
 * 存在的理由是 `_STACK_TABLE` 再全也有边界——公司内部的构建包装器（`./bin/ci check`）、
 * 一个 monorepo 里某个子包的专属命令，永远不可能进产品自带的表。填一段就生效，不用等发版。
 *
 * 只收白名单里这几个键，且一律当**字符串**：这里出来的 checkCmd/testCmd 会进模型上下文，
 * 也会被 `_isRecognizedVerifierCommand` 认成项目自己声明的验证命令，所以形状必须是死的。
 * 「谁有资格写这几个键」不在本模块判——见 main.js 里 `_refreshUserCapabilities` 的 absorb()：
 * 跟着 git clone 下来的文件写的命令一律不生效。
 */
const STACK_KEYS = ["lang", "framework", "pkgMgr", "checkCmd", "testCmd", "buildCmd", "lintCmd", "formatCmd", "devCmd", "testDir"];
function normalizeStack(raw) {
  const bag = raw && typeof raw === "object" && !Array.isArray(raw) ? raw : null;
  const out = {};
  if (!bag) return out;
  for (const key of STACK_KEYS) {
    const v = str(bag[key]).slice(0, 400);
    if (v) out[key] = v;
  }
  return out;
}

/**
 * 把一份（可能是任何东西的）声明规整成 `{ tools, commands, disabled, errors }`。
 *
 * 永不抛异常：用户手写的 JSON 一定会写错，而一个错字让整份配置静默消失，是最难查的
 * 失败。所以坏的那条被丢掉并记进 `errors`，好的那些照常生效——`errors` 会显示给用户。
 *
 * @param {unknown} raw 已经 JSON.parse 过的对象（或任何东西）
 * @param {string} source 这份声明来自哪个文件，用于报错和审批框
 */
export function normalizeCapabilities(raw, source = "") {
  const out = { tools: [], commands: [], roles: [], disabled: [], stack: {}, officialDocsHosts: [], errors: [] };
  const bag = raw && typeof raw === "object" && !Array.isArray(raw) ? raw : null;
  if (!bag) return out;
  // 顶层允许直接写，也允许包在 `capabilities` 下——两种写法用户都会自然写出来。
  const caps = bag.capabilities && typeof bag.capabilities === "object" ? bag.capabilities : bag;

  const seen = new Set();
  for (const item of Array.isArray(caps.tools) ? caps.tools.slice(0, MAX_TOOLS) : []) {
    const { tool, error } = normalizeTool(item, source);
    if (error) { out.errors.push(error); continue; }
    if (seen.has(tool.name)) { out.errors.push(`工具名重复：${tool.name}`); continue; }
    seen.add(tool.name);
    out.tools.push(tool);
  }

  for (const item of Array.isArray(caps.knowledge) ? caps.knowledge.slice(0, MAX_TOOLS) : []) {
    const { tool, error } = normalizeKnowledge(item, source);
    if (error) { out.errors.push(error); continue; }
    if (seen.has(tool.name)) { out.errors.push(`名字和已有工具重复：${tool.name}`); continue; }
    seen.add(tool.name);
    out.tools.push(tool);
  }

  const seenCmd = new Set();
  for (const item of Array.isArray(caps.commands) ? caps.commands.slice(0, MAX_COMMANDS) : []) {
    const { command, error } = normalizeCommand(item, source);
    if (error) { out.errors.push(error); continue; }
    if (seenCmd.has(command.cmd)) { out.errors.push(`命令重复：${command.cmd}`); continue; }
    seenCmd.add(command.cmd);
    out.commands.push(command);
  }

  const seenRole = new Set();
  for (const item of Array.isArray(caps.roles) ? caps.roles.slice(0, MAX_ROLES) : []) {
    const { role, error } = normalizeRole(item, source);
    if (error) { out.errors.push(error); continue; }
    if (seenRole.has(role.name)) { out.errors.push(`角色重复：${role.name}`); continue; }
    seenRole.add(role.name);
    out.roles.push(role);
  }

  for (const name of Array.isArray(caps.disabled) ? caps.disabled : []) {
    const n = str(name);
    if (n) out.disabled.push(n);
  }

  out.stack = normalizeStack(caps.stack);

  // 「这些域名是官方文档」。产品自带的那张表只可能收录公开的大站，收不了公司内网的
  // Confluence、也收不了一个小众框架刚换的新域名——而判不出 official 的代价不是少一条
  // 提示，是取证提醒照发、收尾契约继续列为未完成，白烧一轮再给用户一行不实的「未验证」。
  const seenHost = new Set();
  for (const item of Array.isArray(caps.officialDocsHosts) ? caps.officialDocsHosts.slice(0, MAX_DOC_HOSTS) : []) {
    const host = normalizeDocsHost(item);
    if (!host) { out.errors.push(`officialDocsHosts 里这一条不是主机名：${str(item) || "(空)"}`); continue; }
    if (seenHost.has(host)) continue;
    seenHost.add(host);
    out.officialDocsHosts.push(host);
  }
  return out;
}

/**
 * 规整一条角色声明。
 *
 * 角色不只是一段提示词，它还带一份**工具矩阵**——一个 `design` 子智能体拿得到浏览器，
 * 一个 `database` 子智能体拿得到数据库查询。以前这三样（合法名单、提示词、工具矩阵）
 * 是三张写死的表，加一个角色要改三处代码。
 *
 * 这里只校验形状。「这个工具名真的存在吗」要在 main.js 里对着真实工具目录查——那份
 * 目录不在这个纯模块的视野里，而放行一个不存在的工具名，代价是子智能体被派出去之后
 * 才发现手里没有那件工具。
 */
function normalizeRole(item, source) {
  if (!item || typeof item !== "object") return { error: "角色声明必须是对象" };
  const name = str(item.name).toLowerCase();
  if (!NAME_RE.test(name)) return { error: `角色名不合法：${item.name || "(空)"}` };
  const prompt = str(item.prompt);
  if (!prompt) return { error: `角色 ${name} 缺少 prompt —— 那正是这个角色和别的角色的区别所在` };
  const tools = (Array.isArray(item.tools) ? item.tools : []).map(str).filter(Boolean);
  const types = (Array.isArray(item.types) ? item.types : []).map(str).filter(Boolean);
  return { role: { name, prompt, tools, types, source } };
}

/**
 * 规整一条知识库声明 → 一个可检索的工具。
 *
 * 「把我们团队的文档库接进来」这件事以前一条路都没有：knowledge_search 只会 POST 到
 * 写死的网关端点，语义检索看不见工作区之外的文档，用户规则那两份文件各有字数上限。
 *
 * 这里不另造一套：一个知识源就是一个**工具**，走和 HTTP 能力完全相同的那条轨——同样的
 * 名字前缀、同样的审批、同样的「删掉声明就没了」。区别只在 kind：执行时一个发 HTTP，
 * 一个在目录里检索。
 */
function normalizeKnowledge(item, source) {
  if (!item || typeof item !== "object") return { error: "知识库声明必须是对象" };
  const name = str(item.name);
  if (!NAME_RE.test(name)) return { error: `知识库名不合法：${item.name || "(空)"}` };
  const description = str(item.description);
  if (!description) return { error: `知识库 ${name} 缺少 description —— 模型是靠它决定该不该来这里查的` };
  const path = str(item.path);
  if (!path) return { error: `知识库 ${name} 缺少 path` };
  return {
    tool: {
      name,
      toolName: USER_TOOL_PREFIX + name,
      kind: "folder",
      description,
      // 检索型工具的参数是固定的一个 query，不让用户自己写——写错了反而查不了。
      parameters: {
        type: "object",
        properties: { query: { type: "string", description: "要查什么（关键词或短语）" } },
        required: ["query"],
      },
      folder: { path },
      // 检索永远是只读的，所以 Plan / Explorer 这些只读模式里查资料照样能用。
      readOnly: true,
      source,
    },
  };
}

/** 规整一条工具声明。返回 `{tool}` 或 `{error}`，两者必有其一。 */
function normalizeTool(item, source) {
  if (!item || typeof item !== "object") return { error: "工具声明必须是对象" };
  const name = str(item.name);
  if (!NAME_RE.test(name)) {
    return { error: `工具名不合法：${name || "(空)"}（只能小写字母开头，字母数字下划线，2–48 字符）` };
  }
  const description = str(item.description);
  // 没有描述的工具等于没加：模型是靠描述决定要不要用它的。
  if (!description) return { error: `工具 ${name} 缺少 description —— 模型是靠它决定要不要用这个工具的` };

  const http = item.http && typeof item.http === "object" ? item.http : null;
  if (!http) return { error: `工具 ${name} 缺少 http 声明（目前只支持 http 这一种）` };
  const url = str(http.url);
  // 允许以 `${VAR}` 开头——把 base 放进环境变量是最自然的写法（`${ACME_BASE}/api/x`）。
  // 真正的把关在 buildHttpCall：展开之后仍不是 http/https 就拒绝发出去。
  if (!/^https?:\/\//i.test(url) && !/^\$\{[A-Za-z_][A-Za-z0-9_]*\}/.test(url)) {
    return { error: `工具 ${name} 的 url 必须以 http:// 、https:// 或 \${环境变量} 开头` };
  }
  const method = (str(http.method) || "GET").toUpperCase();
  if (!ALLOWED_METHODS.has(method)) return { error: `工具 ${name} 的 method 不支持：${method}` };

  const headers = {};
  if (http.headers && typeof http.headers === "object") {
    for (const [k, v] of Object.entries(http.headers)) {
      const key = str(k);
      if (key && typeof v === "string") headers[key] = v;
    }
  }

  const { parameters, error } = normalizeParameters(item.parameters, name);
  if (error) return { error };

  return {
    tool: {
      name,
      toolName: USER_TOOL_PREFIX + name,
      kind: "http",
      description,
      parameters,
      http: { url, method, headers, body: typeof http.body === "string" ? http.body : "" },
      // 只读与否由**声明**决定，不是我们去猜：GET / HEAD 默认只读，用户可以显式覆盖
      // （有些接口用 POST 做查询）。只读模式（Plan / Explorer）据此逐次放行。
      readOnly: typeof item.readOnly === "boolean" ? item.readOnly : (method === "GET" || method === "HEAD"),
      source,
    },
  };
}

/** 参数声明 → JSON Schema 的 properties/required。 */
function normalizeParameters(raw, toolName) {
  const properties = {};
  const required = [];
  if (raw && typeof raw === "object" && !Array.isArray(raw)) {
    for (const [rawKey, rawSpec] of Object.entries(raw)) {
      const key = str(rawKey);
      if (!PARAM_NAME_RE.test(key)) return { error: `工具 ${toolName} 的参数名不合法：${key || "(空)"}` };
      const spec = rawSpec && typeof rawSpec === "object" ? rawSpec : {};
      const type = str(spec.type) || "string";
      if (!ALLOWED_PARAM_TYPES.has(type)) {
        return { error: `工具 ${toolName} 的参数 ${key} 类型不支持：${type}` };
      }
      properties[key] = { type, description: str(spec.description) || key };
      if (spec.required === true) required.push(key);
    }
  }
  return { parameters: { type: "object", properties, ...(required.length ? { required } : {}) } };
}

/** 规整一条斜杠命令声明。 */
function normalizeCommand(item, source) {
  if (!item || typeof item !== "object") return { error: "命令声明必须是对象" };
  const cmd = str(item.cmd).replace(/^\//, "").toLowerCase();
  if (!CMD_RE.test(cmd)) return { error: `命令名不合法：${item.cmd || "(空)"}` };
  const prompt = str(item.prompt);
  if (!prompt) return { error: `命令 /${cmd} 缺少 prompt（它就是这条命令要发出去的话）` };
  // `cmd` **不带**前导斜杠：斜杠菜单匹配的是用户敲下 `/` 之后的那一截，内置的
  // `_SLASH` 和 MCP 模板都是这个形状，混进一个带斜杠的会永远匹配不上。
  return { command: { cmd, desc: str(item.desc) || prompt.slice(0, 40), prompt, source } };
}

/** 一条声明 → 发给模型的 OpenAI function schema。 */
export function compileToolSchema(tool) {
  return {
    type: "function",
    function: {
      name: tool.toolName,
      // 明确告诉模型这是本机用户自己接进来的能力，不是产品内置的——它据此判断
      // 该不该优先用（用户特地接进来的东西，通常就是他想让你用的那个）。
      description: `${tool.description}\n（用户自己接入的${tool.kind === "folder" ? "知识库检索" : "能力"}，来自 ${tool.source || "本机配置"}）`,
      parameters: tool.parameters,
    },
  };
}

/**
 * 把声明 + 模型给的参数，合成一次真实的 HTTP 调用。
 *
 * 两种插值，规则不同，混了就是漏洞：
 *   - `${VAR}` 取自环境变量，用来放 token —— **不做** URL 编码（它是密钥或整段头值），
 *     并且只在 headers 和 body 里认；URL 里也认，但同样按环境值原样拼。
 *   - `{param}` 取自模型给的参数 —— 在 URL 里**一律 encodeURIComponent**。模型给的是
 *     不可信输入，不编码的话一个 `&admin=1` 就能改写查询串。
 */
export function buildHttpCall(tool, args = {}, env = {}) {
  const val = (k) => {
    const v = args?.[k];
    return v === undefined || v === null ? "" : String(v);
  };
  const missingEnv = new Set();
  const expandEnv = (s) => String(s).replace(PLACEHOLDER_RE, (_m, k) => {
    if (env && Object.prototype.hasOwnProperty.call(env, k) && env[k] != null && env[k] !== "") return String(env[k]);
    missingEnv.add(k);
    return "";
  });
  const url = expandEnv(tool.http.url).replace(PARAM_RE, (_m, k) => encodeURIComponent(val(k)));
  const headers = {};
  for (const [k, v] of Object.entries(tool.http.headers || {})) {
    headers[k] = expandEnv(v).replace(PARAM_RE, (_m, key) => val(key));
  }
  // body 不做 URL 编码：它通常是 JSON，编码会把它写坏。JSON 转义交给声明里自己写的
  // 引号结构（`{"q": "{query}"}`），并在这里对引号做最小转义，免得参数把 JSON 撑破。
  let body = "";
  if (tool.http.body) {
    body = expandEnv(tool.http.body).replace(PARAM_RE, (_m, k) =>
      val(k).replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n"));
  }
  if (missingEnv.size) {
    throw new Error(`未定义的环境变量 ${[...missingEnv].map((k) => "${" + k + "}").join("、")}`
      + "（写在这个能力声明的 url/headers/body 里）。从 Finder/Dock 启动时读不到登录 shell 的变量，"
      + "把它写进 IDE 设置里的环境变量，或从终端启动 IDE。");
  }
  if (!/^https?:\/\//i.test(url)) throw new Error("插值之后 URL 不再是 http/https，已拒绝");
  return { method: tool.http.method, url, headers, body: body || null };
}

/** 从工具全名还原出声明里的短名（`user__acme` → `acme`），不是用户工具则返回 ""。 */
export function userToolShortName(fullName) {
  const n = String(fullName || "");
  return n.startsWith(USER_TOOL_PREFIX) ? n.slice(USER_TOOL_PREFIX.length) : "";
}

/**
 * 合并多个作用域的声明。顺序即优先级：先给的先占名字。
 *
 * `disabled` 是并集而不是覆盖——任何一层想关掉某个工具都算数，这和权限规则那套
 * 「收紧永远赢」的方向一致：一个作用域说"别用这个"，另一个作用域不该把它打开。
 */
export function mergeCapabilities(list) {
  const out = { tools: [], commands: [], roles: [], disabled: [], stack: {}, officialDocsHosts: [], errors: [] };
  const seenTool = new Set();
  const seenRole = new Set();
  const seenCmd = new Set();
  for (const one of list || []) {
    if (!one) continue;
    for (const t of one.tools || []) {
      if (seenTool.has(t.name)) continue;
      seenTool.add(t.name);
      out.tools.push(t);
    }
    for (const c of one.commands || []) {
      if (seenCmd.has(c.cmd)) continue;
      seenCmd.add(c.cmd);
      out.commands.push(c);
    }
    for (const r of one.roles || []) {
      if (seenRole.has(r.name)) continue;
      seenRole.add(r.name);
      out.roles.push(r);
    }
    // 栈覆盖按**字段**取先给的那一份：先给的作用域填了 testCmd、后一个只填了 testDir，
    // 两条都该生效。整块覆盖的话，越靠近项目的那份会把个人配置里的其余字段一起抹掉。
    for (const [k, v] of Object.entries(one.stack || {})) if (!out.stack[k]) out.stack[k] = v;
    // 域名是并集：多一个作用域说"这个站是官方的"，不该被另一个作用域抹掉（和 disabled 同向）。
    for (const h of one.officialDocsHosts || []) if (!out.officialDocsHosts.includes(h)) out.officialDocsHosts.push(h);
    for (const d of one.disabled || []) if (!out.disabled.includes(d)) out.disabled.push(d);
    for (const e of one.errors || []) out.errors.push(e);
  }
  return out;
}
