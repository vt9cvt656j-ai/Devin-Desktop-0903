// Build-time IP strip: remove tool DESCRIPTION text from the shipped bundle.
//
// Why this is safe (zero runtime behavior change): with L0 forced on, every agent
// request already sends only tool NAMES (x-ide-tools header) and the gateway injects
// the full schema (descriptions + parameters) from its own tools.json. The client
// never transmits these description strings. They only sat in the bundle as static
// data an attacker could base64-decode out of the obfuscated file. Blanking them at
// build time closes that leak while the running app is unaffected.
//
// Scope: ONLY the `_buildAgentToolSchemas` function body, and within it only the
// `description:` string/template values on tool-schema and parameter lines. Tool
// NAMES, parameter NAMES, enums, types, and required arrays are preserved (the model
// receives full text from the gateway; the client keeps just enough structure to
// name-route and locally validate).

const FN_MARKER = "function _buildAgentToolSchemas";

/*
 * 三个派发工具的描述**不剥**。
 *
 * 剥除的安全前提是「客户端只发工具名，网关按自己那份 tools.json 回填 schema」。这三个
 * 是唯一的例外：用户声明了自定义角色时，_applyUserRoleEnums 要把角色名补进它们的 `role`
 * 枚举，补的是**客户端这份 schema**，所以 L0 那里会把它们从「只发名字」里摘出来、连整份
 * schema 一起发出去（网关那份目录里只有 11 个内置角色名，用户改不了）。
 *
 * 于是发布构建里模型收到的是 `{name: "run_subagent", description: ""}` 加一串同样被剥空
 * 的参数说明——用户把角色的提示词、工具矩阵、轮数全配好了，模型却看不懂这个工具是干
 * 什么的。整套自定义角色在发布版里是哑的，而开发版一切正常，所以本地测不出来。
 *
 * 泄露代价：141 条里的 3 条。而且性质上不是新增——src/tool-guides.js 的 TOOL_METADATA
 * （每个工具的场景 + 调用示例）本来就明确不在剥除范围内，release-tool-descriptions
 * 那条测试正面要求它别被剥。
 *
 * 更干净的做法是让网关来打这个补丁（客户端只多发一个「用户声明了哪些角色名」的头，
 * schema 仍旧由网关回填），那需要改 Rust 那边并单独部署，留作后续。
 */
const KEEP_DESCRIPTIONS = ["run_subagent", "run_worker", "spawn_multiple_agents"];
const keepsDescription = (line) => KEEP_DESCRIPTIONS.some((n) => line.includes(`name: "${n}"`));

// Replace the VALUE of every `description:` key on a single line with "". Handles
// double/single-quoted strings and template literals, respecting backslash escapes.
// Operates char-by-char so nested quotes/braces inside a description can't fool it.
function blankDescriptionsInLine(line) {
  const KEY = "description:";
  let out = "";
  let i = 0;
  while (i < line.length) {
    const at = line.indexOf(KEY, i);
    if (at === -1) {
      out += line.slice(i);
      break;
    }
    // Copy up to and including the key.
    out += line.slice(i, at + KEY.length);
    let j = at + KEY.length;
    // Skip whitespace between the key and its value.
    while (j < line.length && (line[j] === " " || line[j] === "\t")) {
      out += line[j];
      j++;
    }
    const q = line[j];
    if (q === '"' || q === "'" || q === "`") {
      // Consume the string literal, honoring escapes, and drop its contents.
      let k = j + 1;
      while (k < line.length) {
        if (line[k] === "\\") {
          k += 2;
          continue;
        }
        if (line[k] === q) break;
        k++;
      }
      out += q + q; // empty string of the same quote kind
      i = k + 1; // resume after the closing quote
    } else {
      // `description:` not followed by a string literal (unexpected) — leave as-is.
      i = j;
    }
  }
  return out;
}

export function stripToolIp(source) {
  const newline = source.includes("\r\n") ? "\r\n" : "\n";
  const lines = source.split(/\r?\n/);
  // 两个入口：main.js 的 `_buildAgentToolSchemas`（历史位置），和 src/agent/tool-catalog.js
  // 里的三段字面量（2026-08-25 从前者搬出去的）。**必须都认**——只认前者的话，
  // 目录搬家之后这一步会一个描述都剥不到，而它剥的是要保护的东西。
  // 好在插件那边有「剥到的行数少于阈值就让构建失败」的兜底，所以搬家当时是**响亮地**
  // 失败的，不是静默泄漏。这里把第二个入口补上。
  // 逐个区间收集，**不是首个匹配赢**：main.js 里 _buildAgentToolSchemas 仍有 31 条
  // 描述（用户声明派生的那些），目录模块里另有 145 条。first-match-wins 在两份源码
  // 拼在一起时只会剥前者，后者原样漏出去——发布产物里就是完整的工具描述。
  const regions = [];
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].startsWith(FN_MARKER)) {
      for (let j = i + 1; j < lines.length; j++) {
        if (lines[j] === "}") { regions.push([i, j]); i = j; break; }
      }
    } else if (/^const (BASE|READONLY_EXTERNAL|WRITE) = \[/.test(lines[i])) {
      for (let j = i + 1; j < lines.length; j++) {
        if (lines[j] === "];") { regions.push([i, j]); i = j; break; }
      }
    }
  }
  if (!regions.length) return { code: source, changed: 0, found: false };
  let changed = 0;
  for (const [start, end] of regions) {
    for (let i = start; i <= end; i++) {
      if (!lines[i].includes("description:")) continue;
      if (keepsDescription(lines[i])) continue;   // 见 KEEP_DESCRIPTIONS
      const next = blankDescriptionsInLine(lines[i]);
      if (next !== lines[i]) { changed++; lines[i] = next; }
    }
  }
  return { code: lines.join(newline), changed, found: true };
}
