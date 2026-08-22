// 需求账本与开场上下文：三条「测试全绿、功能是死的」。
//
// 1. 折叠开场消息时，保留段按一个已改名的标题找账本——永远找不到，账本整段被切掉。
//    账本压根不在磁盘上，折掉就再也回不来；用户看到的是长 run 前面听话、折叠之后像换了个人。
// 2. 「只发已掉出历史的」差集，两侧空白处理不一致：入账压成单空格，比对拿历史原文。
//    前 40 字里有一个换行（分行写需求、贴报错），这条明明还在对话里也被每轮重列一遍。
// 3. 冷启动首轮：MCP 名录在「等预热 1.5 秒」之前就算完了，等完也不重算——白等。
//
// 这里守的是行为：用源码里真实的装配表达式、真实的保留段、真实的差集判据跑一遍，
// 不手抄任何标题。
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const SRC = fs.readFileSync("src/main.js", "utf8");

// ---- source scanner (skip strings / templates / regex / comments) --------------------
function skipString(s, i, q) { i++; for (; i < s.length; i++) { if (s[i] === "\\") { i++; continue; } if (s[i] === q) return i; } return i; }
function skipRegex(s, i) { i++; let cls = false; for (; i < s.length; i++) { const c = s[i]; if (c === "\\") { i++; continue; } if (c === "[") cls = true; else if (c === "]") cls = false; else if (c === "/" && !cls) return i; } return i; }
function skipTemplate(s, i) {
  i++;
  for (; i < s.length; i++) {
    if (s[i] === "\\") { i++; continue; }
    if (s[i] === "`") return i;
    if (s[i] === "$" && s[i + 1] === "{") {
      i += 2; let depth = 1;
      for (; i < s.length && depth > 0; i++) {
        const c = s[i];
        if (c === "\\") { i++; continue; }
        if (c === "'" || c === '"') { i = skipString(s, i, c); continue; }
        if (c === "`") { i = skipTemplate(s, i); continue; }
        if (c === "{") depth++; else if (c === "}") depth--;
      }
      i--;
    }
  }
  return i;
}
function isRegexPos(s, i) {
  let j = i - 1; while (j >= 0 && /\s/.test(s[j])) j--;
  if (j < 0) return true;
  if ("=([,{;:!&|?+-*%<>~^".includes(s[j])) return true;
  return /(?:^|[^\w$])(return|typeof|case|in|of|do|else|void|delete|instanceof|yield|await)$/.test(s.slice(Math.max(0, j - 12), j + 1));
}
function extractConstDecl(name) {
  const m = new RegExp(`\\bconst\\s+${name}\\s*=`).exec(SRC);
  if (!m) throw new Error(`const ${name} not found in main.js`);
  let i = SRC.indexOf("=", m.index) + 1, depth = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i], d = SRC[i + 1];
    if (c === "/" && d === "/") { i = SRC.indexOf("\n", i); if (i < 0) i = SRC.length; continue; }
    if (c === "/" && d === "*") { i = SRC.indexOf("*/", i + 2) + 1; continue; }
    if (c === "'" || c === '"') { i = skipString(SRC, i, c); continue; }
    if (c === "`") { i = skipTemplate(SRC, i); continue; }
    if (c === "/" && isRegexPos(SRC, i)) { i = skipRegex(SRC, i); continue; }
    if (c === "(" || c === "[" || c === "{") depth++;
    else if (c === ")" || c === "]" || c === "}") depth--;
    else if (c === ";" && depth === 0) return SRC.slice(m.index, i + 1);
  }
  throw new Error(`unterminated declaration extracting ${name}`);
}
function loadConst(name) {
  return new Function(`${extractConstDecl(name)}\n;return ${name};`)();
}
function extractFn(name) {
  const m = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).exec(SRC);
  if (!m) throw new Error(`function ${name} not found in main.js`);
  let p = SRC.indexOf("(", m.index), pd = 0;
  for (; p < SRC.length; p++) {
    const c = SRC[p], d = SRC[p + 1];
    if (c === "/" && d === "/") { p = SRC.indexOf("\n", p); if (p < 0) p = SRC.length; continue; }
    if (c === "/" && d === "*") { p = SRC.indexOf("*/", p + 2) + 1; continue; }
    if (c === "'" || c === '"') { p = skipString(SRC, p, c); continue; }
    if (c === "`") { p = skipTemplate(SRC, p); continue; }
    if (c === "/" && isRegexPos(SRC, p)) { p = skipRegex(SRC, p); continue; }
    if (c === "(") pd++;
    else if (c === ")") { pd--; if (pd === 0) break; }
  }
  let i = SRC.indexOf("{", p), depth = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i], d = SRC[i + 1];
    if (c === "/" && d === "/") { i = SRC.indexOf("\n", i); if (i < 0) i = SRC.length; continue; }
    if (c === "/" && d === "*") { i = SRC.indexOf("*/", i + 2) + 1; continue; }
    if (c === "'" || c === '"') { i = skipString(SRC, i, c); continue; }
    if (c === "`") { i = skipTemplate(SRC, i); continue; }
    if (c === "/" && isRegexPos(SRC, i)) { i = skipRegex(SRC, i); continue; }
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (depth === 0) return SRC.slice(m.index, i + 1); }
  }
  throw new Error(`unbalanced braces extracting ${name}`);
}
// 注释里会引用被修掉的旧代码，所以凡是对源码文本的断言都先剥注释。按上下文逐字符扫，
// 认得字符串 / 模板串 / 正则字面量（两条正则式的剥法会把 `/\//` 当成行注释吃掉真代码）。
function stripJsComments(source) {
  const s = String(source);
  let out = "", i = 0, prev = "";
  const regexCanStart = (p) => !/[A-Za-z0-9_$)\]]/.test(p);
  while (i < s.length) {
    const c = s[i], d = s[i + 1];
    if (c === "/" && d === "/") { while (i < s.length && s[i] !== "\n") i++; continue; }
    if (c === "/" && d === "*") { const e = s.indexOf("*/", i + 2); i = e < 0 ? s.length : e + 2; continue; }
    if (c === '"' || c === "'" || c === "`") {
      const q = c; out += c; i++;
      while (i < s.length) {
        const ch = s[i]; out += ch;
        if (ch === "\\") { i++; if (i < s.length) out += s[i]; i++; continue; }
        i++;
        if (ch === q) break;
      }
      prev = q; continue;
    }
    if (c === "/" && regexCanStart(prev)) {
      out += c; i++;
      let inClass = false;
      while (i < s.length) {
        const ch = s[i]; out += ch;
        if (ch === "\\") { i++; if (i < s.length) out += s[i]; i++; continue; }
        i++;
        if (ch === "[") inClass = true;
        else if (ch === "]") inClass = false;
        else if (ch === "/" && !inClass) break;
        else if (ch === "\n") break;
      }
      prev = "/"; continue;
    }
    out += c;
    if (!/\s/.test(c)) prev = c;
    i++;
  }
  return out;
}

const CODE = stripJsComments(SRC);
const HEAD = loadConst("_DEMAND_LEDGER_HEAD");
const SEND = extractFn("sendPrompt");
const TRIM = extractFn("_trimMessagesIfHuge");
const norm = new Function(`${extractFn("_ledgerNorm")}\n;return _ledgerNorm;`)();
const MODES_WITH_TOOLS = loadConst("_MODES_WITH_TOOLS");

/** 用 sendPrompt 里真实的装配表达式拼出账本块——不手抄标题。 */
function ledgerBlockOf(faded) {
  const expr = /const _demandLedgerBlock = _fadedDemands\.length[\s\S]*?: "";/.exec(SEND);
  assert.ok(expr, "找不到账本块的装配表达式");
  return new Function("_fadedDemands", "_DEMAND_LEDGER_HEAD", `${expr[0]}\nreturn _demandLedgerBlock;`)(faded, HEAD);
}

/** _trimMessagesIfHuge 里真实的保留段（从 _head 到 _kept），按源码原样执行。 */
function keepOf(content) {
  const seg = /const _head = m\.content\.slice\(0, mk\);[\s\S]*?const _kept = [^\n]*\n/.exec(TRIM);
  assert.ok(seg, "保留段那块不见了，这条断言失去落点");
  // 边界的算法跟源码一致：网关按「━━━…\n📌」定位真实用户请求。
  const bk = content.search(/━{8,}\s*\n\s*📌/);
  const mk = bk >= 0 ? bk : content.indexOf("📌");
  return new Function("m", "mk", "_DEMAND_LEDGER_HEAD", `${seg[0]}\nreturn _kept;`)({ content }, mk, HEAD);
}

// ═══ 1. 折叠保留段认得账本的真实标题 ════════════════════════════════════════════

test("折叠开场消息时，账本按它真实的标题被原样带走（装配端 → 折叠端全链）", () => {
  const ledger = ledgerBlockOf(["帮我修登录", "记住：回复一律用中文"]);
  assert.ok(ledger.startsWith(HEAD), "账本块不以共享常量开头——折叠端就没有可对齐的标记");
  const dump = "X".repeat(500);
  const head = "操作系统：macOS 15\n\n" + ledger
    + "--- 项目上下文 ---\n--- 目录树 ---\nsrc/\n  a.ts\n--- 当前文件 ---\n" + dump + "\n";
  const kept = keepOf(head + "━━━━━━━━━━━━\n📌 用户请求：继续");
  assert.match(kept, /回复一律用中文/, "需求账本被折没了——账本压根不在磁盘上，折掉就再也回不来");
  assert.ok(kept.includes(HEAD), "账本的表头没跟着一起保留，模型不知道那几行是什么");
  // 目录树和当前文件转储**应该**被折掉：替换文案承诺的 list_dir / read_file 正是为它们写的。
  assert.ok(!kept.includes(dump), "当前文件转储没被折掉，这一刀就白挥了");
  assert.ok(!kept.includes("  a.ts"), "目录树没被折掉");
  // 没有账本的开场消息，不许凭空多出一段空壳。
  assert.equal(keepOf("操作系统：macOS\n--- 目录树 ---\nsrc/\n" + dump + "\n━━━━━━━━━━━━\n📌 继续"), "");
});

test("折叠 marker 列表里的每个前缀，源码里都有对应的装配端——改名只改一头就会红", () => {
  const trim = stripJsComments(TRIM);
  const list = /for \(const marker of (\[[^\]]*\])\)/.exec(trim);
  assert.ok(list, "找不到保留段的 marker 列表");
  const items = list[1].slice(1, -1).split(",").map((x) => x.trim()).filter(Boolean);
  assert.ok(items.length >= 3, "约定、子目录约定、账本三类至少要齐");
  for (const item of items) {
    if (/^["']/.test(item)) {
      const lit = new Function(`return ${item};`)();
      // 字面量 marker：装配端要么以 `\n<marker>` 起一段，要么以 `<marker>` 开模板。
      assert.ok(CODE.includes("\\n" + lit) || CODE.includes("`" + lit),
        `marker ${item} 在源码里找不到装配端——它只会保住一段从来不存在的分段`);
    } else {
      // 标识符 marker：装配端必须把同一个常量写进模板开头。
      assert.ok(CODE.includes("`${" + item + "}"),
        `marker ${item} 没有被任何模板当作分段起始——装配端另写了一份字面量就是漂移`);
    }
  }
  assert.ok(items.includes("_DEMAND_LEDGER_HEAD"), "账本那一类必须引用共享常量，不许手抄标题");
  // 标题字面量全源码只许出现一次（常量定义处）：第二份字面量就是下一次漂移的种子。
  assert.equal(CODE.split(HEAD).length - 1, 1, "账本标题在源码里有第二份手抄，改名时必然只改一头");
});

// ═══ 2. 差集两侧同一个归一化 ══════════════════════════════════════════════════════

/** sendPrompt 里真实的差集判据：_recentText + _fadedDemands。 */
function fadedOf(sess, effectiveMode = "agent") {
  const start = SEND.indexOf("const _recentText = (() => {");
  const fadedAt = SEND.indexOf("const _fadedDemands = (", start);
  const end = SEND.indexOf("\n    : [];", fadedAt);
  assert.ok(start > 0 && fadedAt > start && end > fadedAt, "找不到差集判据那段");
  const body = SEND.slice(start, end + "\n    : [];".length);
  return new Function("sess", "effectiveMode", "_MODES_WITH_TOOLS", "_ledgerNorm", `${body}\nreturn _fadedDemands;`)(
    sess, effectiveMode, MODES_WITH_TOOLS, norm,
  );
}

/** sendPrompt 里真实的入账段：从 _lt 到 if (!_isFiller) {...} 收尾。 */
function pushOf(text, sess) {
  const start = SEND.indexOf("const _lt = text.trim();");
  const tail = SEND.indexOf("_autoMemoryCapture(_identityRoot, _lt);", start);
  const end = SEND.indexOf("\n    }", tail);
  assert.ok(start > 0 && tail > start && end > tail, "找不到入账那段");
  const body = SEND.slice(start, end + "\n    }".length);
  new Function("text", "sess", "_ledgerNorm", "_applyExplicitMemoryCorrection", "_autoMemoryCapture", "_identityRoot", body)(
    text, sess, norm, () => true, () => {}, "",
  );
}

const MULTILINE_TEXTS = [
  // 分行写需求——最常见：第一行是要求，第二行是证据。
  "帮我修登录页的卡死\n报错是 TypeError: Cannot read properties of undefined (reading 'id')",
  // 贴的报错里带制表符。
  "修掉这个：\n\tat login.ts:42\n\tat app.ts:7，改完跑一下测试",
  // Windows 换行 + 连续空格。
  "先把  登录页\r\n的卡死修掉，再跑回归，别动现有视觉",
];

test("还在对话里的多行消息不再被当成「已折叠」每轮重列", () => {
  for (const text of MULTILINE_TEXTS) {
    const sess = { memory: { recent: [] }, _demandLedger: [] };
    pushOf(text, sess);
    assert.equal(sess._demandLedger.length, 1, `这条实质要求没入账：${JSON.stringify(text)}`);
    // 历史里存的是原文（conversation-memory 原样 push），不是入账时压过空白的那份。
    sess.memory.recent.push({ role: "user", content: text });
    for (const mode of MODES_WITH_TOOLS) {
      assert.deepEqual(fadedOf(sess, mode), [],
        `${mode} 模式下，明明还在对话里的多行消息被列进了「已折叠掉的历次要求」：${JSON.stringify(text)}`);
    }
  }
});

test("真正掉出历史的那条照样捞回来——差集不是被关掉了", () => {
  const text = MULTILINE_TEXTS[0];
  const sess = { memory: { recent: [] }, _demandLedger: [] };
  pushOf(text, sess);
  // 历史被压缩：这条不在 recent 里了。
  sess.memory.recent = [{ role: "user", content: "继续" }];
  const faded = fadedOf(sess, "agent");
  assert.equal(faded.length, 1, "掉出历史的要求没被捞回来");
  assert.equal(faded[0], norm(text).slice(0, 240));
  // 历史里被 _lexCompress 改过的消息（行尾空白被删）也算「还在」。
  sess.memory.recent = [{ role: "user", content: text.replace(/[ \t]+\n/g, "\n") + "   " }];
  assert.deepEqual(fadedOf(sess, "agent"), []);
});

test("入账两条路（sendPrompt / 插话）和比对走的是同一个归一化函数", () => {
  assert.equal(norm("a\n\tb  c\r\n"), "a b c");
  assert.equal(norm(null), "");
  const send = stripJsComments(SEND);
  assert.match(send, /sess\._demandLedger\.push\(_ledgerNorm\(_lt\)\.slice\(0, 240\)\)/,
    "sendPrompt 入账没走 _ledgerNorm");
  assert.match(send, /\.map\(\(m\) => _ledgerNorm\(m\.content\)\)\.join\("\\n"\)/,
    "历史那一侧没走 _ledgerNorm——差集又回到单边归一化");
  assert.match(send, /const key = _ledgerNorm\(d\)\.slice\(0, 40\);/,
    "账本那一侧没走 _ledgerNorm");
  const loop = stripJsComments(extractFn("_runAgenticLoop"));
  assert.match(loop, /const _sl = _ledgerNorm\(steerText\);/, "插话入账没走 _ledgerNorm");
  // 不许再出现第三种空白处理：任何 _demandLedger.push 的参数里都不能自带 replace(/\s+/g)。
  assert.doesNotMatch(CODE, /_demandLedger\.push\([^\n]*\.replace\(\/\\s\+\/g/,
    "又有一处入账自己压空白了——归一化必须只有 _ledgerNorm 一份");
});

// ═══ 3. 冷启动首轮：名录在预热等待之后算 ════════════════════════════════════════

/**
 * 跑 sendPrompt 里从诊断块之后到技能块之前的那一段真实代码：冷启动等待 + MCP 名录。
 * 返回拼完的 contextBlock。
 */
async function contextAfterWarmup(deps) {
  const anchor = "if (diagBlock) contextBlock += `\\n\\n${diagBlock}`;";
  const start = SEND.indexOf(anchor);
  const end = SEND.indexOf("const skillsBlock =", start);
  assert.ok(start > 0 && end > start, "找不到「诊断块 … 技能块」这段");
  const body = SEND.slice(start + anchor.length, end);
  const keys = Object.keys(deps);
  return new Function(...keys, `return (async () => { let contextBlock = ""; ${body}\nreturn contextBlock; })();`)(
    ...keys.map((k) => deps[k]),
  );
}

function mcpDeps({ effectiveMode = "agent", cacheKey = "", warm } = {}) {
  const state = { loaded: false, warmCalls: 0 };
  const deps = {
    effectiveMode,
    inTauri: true,
    _fileSkillsCacheKey: cacheKey,
    _curRoot: "/repo",
    _scheduleWorkspaceAgentWarmup: () => {},
    _refreshFileSkills: async () => {},
    _warmMcpTools: warm || (async () => { state.warmCalls++; await new Promise((r) => setTimeout(r, 5)); state.loaded = true; }),
    // 预热没落地时就是空快照——与 _readyMcpSnapshot 的真实行为一致。
    _readyMcpSnapshot: () => state.loaded
      ? { failed: [["broken", "connect ECONNREFUSED"]], toolCache: [{ function: { name: "mcp__svc__t" } }] }
      : { failed: [], toolCache: [] },
    _mcpAvailabilitySystemContext: (snap) => (snap.toolCache.length ? "【MCP 名录】" : ""),
    _mcpFailureSystemContext: (failed) => (failed?.length ? "【MCP 失败诊断】" : ""),
  };
  return { deps, state };
}

test("冷启动首轮：等完预热之后才算 MCP 名录，首轮就能看见有哪些服务", async () => {
  const { deps, state } = mcpDeps();
  const ctx = await contextAfterWarmup(deps);
  assert.equal(state.warmCalls, 1, "冷启动首轮没等 MCP 预热");
  assert.match(ctx, /【MCP 名录】/, "名录在预热之前就算完了——等到了也没人回头重算，这 1.5 秒对 MCP 是白等");
  assert.match(ctx, /【MCP 失败诊断】/, "连不上的服务同样要在等完之后才报得出来");
});

test("不是冷启动（已扫过）就不等，也不多一次预热；名录照常", async () => {
  const { deps, state } = mcpDeps({ cacheKey: "\0/Users/me" });
  state.loaded = true;
  const ctx = await contextAfterWarmup(deps);
  assert.equal(state.warmCalls, 0, "非首轮不该再等预热");
  assert.match(ctx, /【MCP 名录】/);
});

test("聊天模式移过去之后仍然不发名录、不发失败诊断", async () => {
  const { deps, state } = mcpDeps({ effectiveMode: "chat" });
  state.loaded = true;
  const ctx = await contextAfterWarmup(deps);
  assert.doesNotMatch(ctx, /【MCP 名录】|【MCP 失败诊断】/, "聊天模式收到了 MCP 名录——那段话会指使模型编造它做过的操作");
});

test("预热挂住也只等有上界的那一次：名录为空但这一轮照常发出", async () => {
  const never = new Promise(() => {});
  const { deps } = mcpDeps({ warm: () => never });
  const t0 = Date.now();
  const ctx = await Promise.race([
    contextAfterWarmup(deps),
    new Promise((_, reject) => setTimeout(() => reject(new Error("等待没有上界")), 4000)),
  ]);
  assert.ok(Date.now() - t0 < 4000);
  assert.doesNotMatch(ctx, /【MCP 名录】/, "预热没落地却凭空有了名录");
});

test("源码顺序：名录的计算点在冷启动等待之后、技能块之前", () => {
  const send = stripJsComments(SEND);
  const wait = send.indexOf("_warmMcpTools(_curRoot");
  const roster = send.indexOf("_mcpAvailabilitySystemContext(mcpSnapshot)");
  const skills = send.indexOf("const skillsBlock =");
  assert.ok(wait > 0 && roster > 0 && skills > 0, "锚点缺失");
  assert.ok(roster > wait, "MCP 名录在预热等待之前就算完了");
  assert.ok(roster < skills, "MCP 名录必须仍然在 contextBlock 收口之前拼进去");
  // 名录那行自己不许 await：预热拿不到就算了，下一轮自然就有。
  const line = send.slice(roster, send.indexOf("\n", roster));
  assert.doesNotMatch(line, /await/);
});
