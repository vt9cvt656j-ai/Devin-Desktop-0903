// Unit tests for the pure logic inside the (monolithic, export-less) src/main.js.
//
// main.js is a 23k-line browser module with no exports, so we can't `import` its
// helpers. Instead we EXTRACT each function's real source by name — brace-matched with a
// small scanner that skips string / template / regex / comment contents so their braces
// aren't miscounted — and eval it with its module-level dependencies injected as params.
// => these tests exercise the ACTUAL shipped code, not hand-copied duplicates that drift.
//
// Run:  node --test   (from ide/, or `npm test`)
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "../src/main.js"), "utf8");

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
      i--; // for-loop will ++
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
function extractFn(name) {
  const m = new RegExp(`function\\s+${name}\\s*\\(`).exec(SRC);
  if (!m) throw new Error(`function ${name} not found in main.js`);
  let i = SRC.indexOf("{", m.index), depth = 0;
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
// Build the real function with its module-level deps injected as parameters.
function load(name, deps = {}) {
  const keys = Object.keys(deps);
  const fn = new Function(...keys, `${extractFn(name)}\n;return ${name};`);
  return fn(...keys.map((k) => deps[k]));
}

// ---- tests ---------------------------------------------------------------------------
test("_toPosix normalizes Windows backslashes, no-op elsewhere", () => {
  const f = load("_toPosix");
  assert.equal(f("C:\\Users\\me\\proj"), "C:/Users/me/proj");
  assert.equal(f("/Users/me/proj"), "/Users/me/proj"); // mac untouched
  assert.equal(f("a\\b/c"), "a/b/c");
  assert.equal(f(null), null);
  assert.equal(f(42), 42);
});

test("_resolveRel resolves relatives to the workspace + passes absolutes through (incl. Windows)", () => {
  const f = load("_resolveRel", { _allRoots: () => ["/Users/me/proj"] });
  assert.equal(f("src/main.js"), "/Users/me/proj/src/main.js"); // plain relative → prepend root
  assert.equal(f("proj/src/x"), "/Users/me/proj/src/x");        // redundant root-name stripped
  assert.equal(f("/etc/hosts"), "/etc/hosts");                  // unix absolute → as-is
  assert.equal(f("C:\\Windows\\x"), "C:\\Windows\\x");          // windows absolute (backslash) → as-is
  assert.equal(f("C:/Windows/x"), "C:/Windows/x");              // windows absolute (fwd slash) → as-is
  assert.equal(f(""), "");
  // Windows workspace (posix-normalized root) resolves correctly:
  const fw = load("_resolveRel", { _allRoots: () => ["C:/Users/me/proj"] });
  assert.equal(fw("src/x.js"), "C:/Users/me/proj/src/x.js");
});

test("_resolveRel with no open root leaves the path unchanged", () => {
  const f = load("_resolveRel", { _allRoots: () => [] });
  assert.equal(f("src/x.js"), "src/x.js");
});

test("_lev computes edit distance", () => {
  const f = load("_lev");
  assert.equal(f("kitten", "kitten"), 0);
  assert.equal(f("kitten", "sitten"), 1);
  assert.equal(f("read_file", "readfile"), 1);
  assert.ok(f("run_cmd", "bash") >= 3);
});

test("_buildRepoMap builds a per-file symbol map from the index, query-boosted + bounded", () => {
  const idx = new Map([
    ["openFolder", [{ name: "openFolder", kind: "function", path: "src/main.js", line: 3567 }]],
    ["_resolveRel", [{ name: "_resolveRel", kind: "function", path: "src/main.js", line: 14329 }]],
    ["parseAuth", [{ name: "parseAuth", kind: "function", path: "src/auth.js", line: 10 }]],
    ["verifyToken", [{ name: "verifyToken", kind: "function", path: "src/auth.js", line: 40 }]],
  ]);
  const f = load("_buildRepoMap", { _symbolIndex: idx });
  const out = f("fix the auth token", 6000);
  assert.match(out, /项目符号地图/);
  assert.match(out, /src\/auth\.js: parseAuth, verifyToken/);   // both auth symbols listed
  assert.match(out, /src\/main\.js: openFolder, _resolveRel/);
  // query "auth token" should rank auth.js ABOVE main.js despite equal symbol counts:
  assert.ok(out.indexOf("src/auth.js") < out.indexOf("src/main.js"), "query-relevant file ranks first");
  // empty index → empty string (graceful before the background index builds):
  assert.equal(load("_buildRepoMap", { _symbolIndex: new Map() })("x", 6000), "");
});

test("_safeJsonLoose repairs malformed \\u escapes (the 'unexpected end of hex escape' bug)", () => {
  const f = load("_safeJsonLoose");
  // model put a literal \u (a regex) in content without double-escaping → broken JSON:
  const r = f('{"path":"a.js","content":"const re = /\\username/;"}');
  assert.ok(r && r.path === "a.js", "recovers the object");
  assert.match(r.content, /\\username/, "the literal \\u is preserved as text");
  // truncated \u12 right before the closing quote:
  const r2 = f('{"content":"tail\\u12"}');
  assert.ok(r2 && typeof r2.content === "string" && r2.content.includes("tail"));
  // a genuinely-valid ✓ must STILL decode to the checkmark (not get double-escaped):
  assert.equal(f('{"content":"ok \\u2713"}').content, "ok ✓");
  // an already-escaped \\u (literal backslash) must be left alone:
  assert.equal(f('{"content":"C:\\\\users"}').content, "C:\\users");
});

test("_modelSeesImages defaults TRUE (send real image) except known text-only models", () => {
  const f = load("_modelSeesImages", { MODEL_GROUPS: [] });
  // multimodal / unknown → assume it can see (the fix for '多模态读不懂图片'):
  for (const id of ["claude-opus-4-8", "gpt-5.5", "gemini-3-pro", "glm-4.6", "qwen-max",
                    "doubao-pro-32k", "hunyuan-turbo", "grok-4", "some-new-gateway-alias"]) {
    assert.equal(f(id), true, `${id} should be treated as vision-capable`);
  }
  // genuinely text-only / non-chat → bridge via transcription:
  for (const id of ["deepseek-chat", "deepseek-reasoner", "deepseek-coder", "o1-mini",
                    "text-embedding-3-large", "whisper-1", "codestral-latest"]) {
    assert.equal(f(id), false, `${id} should route through the text bridge`);
  }
  // deepseek's OWN vision model must NOT be denylisted:
  assert.equal(f("deepseek-vl2"), true);
});

test("_looksQuickAsk excludes project/multi-file scope (so it isn't crippled to a tiny budget)", () => {
  const f = load("_looksQuickAsk", { _looksUIBuildTask: () => false, _looksBugFixTask: () => false });
  // trivial conversational asks are still 'quick':
  assert.equal(f("什么是闭包？"), true);
  assert.equal(f("这个函数是什么意思"), true);
  // but anything project/codebase-scoped must NOT be quick — it needs real exploration:
  assert.equal(f("看一下我的项目"), false);
  assert.equal(f("帮我看看这几个文件"), false);
  assert.equal(f("分析一下整个代码库"), false);
  assert.equal(f("梳理一下这个工程的架构"), false);
});

test("developer community search is wired through schema, normalization, execution, and truthful fallback", () => {
  assert.match(SRC, /name: "developer_community_search"/);
  assert.match(SRC, /case "developer_community_search": return \{ type: "developer_community_search"/);
  assert.match(SRC, /backend\.invoke\(call\.type, _args\)/);
  assert.match(SRC, /成功响应不等于内容已被证实/);
  assert.match(SRC, /只调用工具或配置接口不等于成功/);

  const directoryDescription = SRC.match(/const _SEARCH_TOOLS_DESCRIPTION = `([^`]+)`;/)?.[1];
  assert.ok(directoryDescription, "search_tools should have a concise runtime description");
  assert.match(directoryDescription, /developer_community_search/);
  assert.match(directoryDescription, /当前支持/);
  assert.doesNotMatch(directoryDescription, /100%|十倍|全球最大|所有公开仓库|全部免费|秒回|绝不会丢/);
});

test("_sharedCtxDigest renders the shared run-context a sub-agent reads (真上下文协议)", () => {
  const f = load("_sharedCtxDigest");
  assert.equal(f(null), "", "no ctx → empty");
  assert.equal(f({}), "", "empty ctx → empty (nothing to share yet)");
  const ctx = {
    goal: "fix auth token refresh",
    done: ["read config", "found the bug"],
    modified: new Map([["auth.ts", "编辑"]]),
    filesRead: new Set(["src/auth.ts", "src/token.ts"]),
    findings: ["refreshToken() at auth.ts:42 never awaits"],
    errors: ["401 on retry"],
  };
  const s = f(ctx);
  assert.match(s, /主智能体已经掌握的上下文/);       // header so the child knows to reuse it
  assert.match(s, /fix auth token refresh/);          // goal
  assert.match(s, /auth\.ts\(编辑\)/);                // mutations w/ rationale
  assert.match(s, /src\/token\.ts/);                  // files already read (don't re-read)
  assert.match(s, /refreshToken\(\) at auth\.ts:42/); // prior findings
  assert.match(s, /401 on retry/);                    // open errors
});

test("_ceSerialize renders composer chips as space-delimited @refs so MULTIPLE drops all parse", () => {
  const f = load("_ceSerialize");
  // Fake the minimal DOM the walker touches: text nodes, chip elements, a plain element.
  const T = (v) => ({ nodeType: 3, nodeValue: v });
  const CHIP = (rel) => ({ nodeType: 1, classList: { contains: (c) => c === "composer-chip" }, dataset: { rel } });
  const ROOT = (...kids) => ({ childNodes: kids });
  // The send-path mention regex (must stay identical to main.js line ~8057).
  const refs = (s) => [...s.matchAll(/(?:^|\s)@([^\s]+)/g)].map((m) => m[1]);

  // Two chips dropped back-to-back ([chip][chip]) — the bug the user hit ("只能拖一个").
  const two = f(ROOT(CHIP("src/a.js"), CHIP("lib/b")));
  assert.equal(two, " @src/a.js  @lib/b ");
  assert.deepEqual(refs(two), ["src/a.js", "lib/b"], "BOTH refs must parse, not just one");

  // A chip dropped straight after a word, no space ("看这个[chip]"): the leading virtual
  // space is what rescues it — without it "看这个@rel" wouldn't match (?:^|\s)@.
  const adj = f(ROOT(T("看这个"), CHIP("dir1")));
  assert.equal(adj, "看这个 @dir1 ");
  assert.deepEqual(refs(adj), ["dir1"]);

  // Mixed: text + chip + text + chip, and a lone chip trims cleanly on send.
  const mixed = f(ROOT(T("先看"), CHIP("a"), T("再看"), CHIP("b/c"), T("对比")));
  assert.deepEqual(refs(mixed), ["a", "b/c"]);
  assert.equal(f(ROOT(CHIP("only/one"))).trim(), "@only/one");

  // The zero-width caret pad (U+200B) inserted after a dropped chip must be STRIPPED, so it never
  // reaches the sent text nor breaks the @ref (regression: "拖进来后光标看不到了" fix added the pad).
  const padded = f(ROOT(CHIP("src/x.js"), T("​")));
  assert.ok(!padded.includes("​"), "zero-width pad must not survive serialization");
  assert.deepEqual(refs(padded), ["src/x.js"]);
  // pad between two chips (drop, drop) still yields two clean refs:
  assert.deepEqual(refs(f(ROOT(CHIP("a"), T("​"), CHIP("b"), T("​")))), ["a", "b"]);
});

test("_dynamicChatChips predicts context-aware starters (not a fixed hardcoded list)", () => {
  const markers = (n) => Array.from({ length: n }, () => ({ severity: 8 })); // 8 = Monaco error
  const base = {
    activePath: "/ws/src/a.js",
    _pathToRel: (p) => p.replace(/^\/ws\//, ""),
    monacoEditor: { getSelection: () => ({ isEmpty: () => true }), getModel: () => ({ uri: {} }) },
    monaco: { editor: { getModelMarkers: () => [] } },
    openFiles: new Map([["/ws/src/a.js", { model: { uri: {} } }]]),
    _lastGitFiles: [],
    rootPath: "/ws",
    workspaceRoots: ["/ws"],
  };
  const run = (over) => load("_dynamicChatChips", { ...base, ...over })();
  const labels = (chips) => chips.map((c) => c.label).join(" | ");

  // errors in the open file → "修复报错 (N)" is ranked FIRST (the top prediction for right now)
  const errs = run({ monaco: { editor: { getModelMarkers: () => markers(3) } } });
  assert.match(errs[0].label, /修复报错 \(3\)/);
  assert.notEqual(errs[0].send, errs[0].label, "chip send is a full prompt, not just the label");

  // clean file, no git → generic file starters; NO commit-message chip (nothing to commit)
  const clean = run({});
  assert.ok(/解释/.test(labels(clean)) && /查找潜在 Bug/.test(labels(clean)));
  assert.ok(!/提交信息/.test(labels(clean)), "no git changes ⇒ no commit-message starter");

  // uncommitted changes ⇒ commit-message / review starters surface dynamically
  const dirty = run({ _lastGitFiles: [{ path: "src/a.js" }, { path: "src/b.js" }] });
  assert.ok(/写提交信息/.test(labels(dirty)));

  // a *.test.js file ⇒ "补充测试用例", never "编写单元测试"
  const tf = run({ activePath: "/ws/src/a.test.js", openFiles: new Map([["/ws/src/a.test.js", { model: { uri: {} } }]]) });
  assert.ok(/补充测试用例/.test(labels(tf)) && !/编写单元测试/.test(labels(tf)));

  // selecting code ⇒ "解释选中的代码" appears
  const sel = run({ monacoEditor: { getSelection: () => ({ isEmpty: () => false }), getModel: () => ({ uri: {} }) } });
  assert.ok(/解释选中的代码/.test(labels(sel)));

  // no file open but a project is ⇒ project-level starters (深挖这个项目 …)
  const proj = run({ activePath: "", openFiles: new Map() });
  assert.ok(/深挖这个项目/.test(labels(proj)));

  // always bounded to 6, and a normal file yields a full row of 6
  assert.ok(errs.length <= 6 && clean.length <= 6 && proj.length <= 6);
  assert.equal(clean.length, 6, "a normal code file should fill all 6 starter chips");
});

test("_flushChatHistorySync writes the shape restoreChatHistory reads (memory object, not history-object) — the '聊天内容全丢' bug", () => {
  const store = {};
  const localStorage = { setItem: (k, v) => { store[k] = v; }, getItem: (k) => (k in store ? store[k] : null) };
  const memJSON = { totalTurns: 3, recent: [{ role: "user", content: "hi" }, { role: "assistant", content: "yo" }], summaries: [], milestones: [] };
  const _chatSessions = [{ id: "s1", name: "Chat 1", mode: "chat", model: "m", project: "", created: 123, memory: { toJSON: () => memJSON } }];
  const flush = load("_flushChatHistorySync", { _chatSessions, localStorage, CHAT_STORE_KEY: "michael-ide.chat-sessions", _activeChatIdx: 0 });
  flush();
  const saved = JSON.parse(store["michael-ide.chat-sessions"]);
  const s0 = saved.sessions[0];
  // Memory must be persisted under `memory` as the serialized object — that's the ONLY object
  // shape restoreChatHistory accepts (`sData.memory`). The old code buried it under `history`
  // as an object, which restore silently dropped → the whole chat vanished on this sync path.
  assert.deepEqual(s0.memory, memJSON, "memory must persist under `memory` (object), readable by restore");
  assert.ok(
    !(s0.history && typeof s0.history === "object" && !Array.isArray(s0.history)),
    "must NOT store the memory object under `history` (unreadable by restore → total loss)"
  );
});
