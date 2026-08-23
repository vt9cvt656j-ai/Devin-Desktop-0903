import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

/*
 * 「没查成」和「没有」是两件事。
 *
 * LSP 的 request() 在四条完全不同的路上都 resolve(null)：超时、发送失败、服务器回
 * JSON-RPC error、以及服务器**真的**答了 null。而 null 在 LSP 协议里本身又是一个合法
 * 结论（"这里没有定义"）。调用方分不出来，就会把"没查成"说成"没有"。
 *
 * 后果不是理论上的：智能体问「谁调用了这个函数」，语言服务超时 → 回 []，模型读到的是
 * **「这个符号没人用」**，于是删掉它、或者改了签名不管调用点。main.js 里 63337 那段
 * 注释已经把这个后果写出来过（那次修的是"行号偏了"这条路），超时这条路一直没修。
 *
 * 判别放在 requestDetailed 一层，request() 只是它丢掉 reason 的薄封装 —— 对外契约
 * 不变（全仓几十处 await 依赖 resolve(null)）。
 */

const LSP = readFileSync(new URL("../src/lsp-client.js", import.meta.url), "utf8");
const MAIN = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");

// ── 把一个 agent* 方法单独抠出来跑 ──────────────────────────────────────────
function methodOf(name, deps = {}) {
  const seg = new RegExp(`async ${name}\\(([^)]*)\\) \\{[\\s\\S]*?\\n    \\},`).exec(LSP);
  assert.ok(seg, `${name} 不见了`);
  const names = ["_agentEnsureDoc", "LSP_SYMBOL_KIND_NAMES", "toMonacoRange", "monaco"];
  return (ctx) => new Function(...names, `return ({ ${seg[0].replace(/,$/, "")} });`)(
    async () => ctx,
    deps.LSP_SYMBOL_KIND_NAMES || {},
    deps.toMonacoRange || ((r) => r),
    deps.monaco || { Uri: { parse: (u) => ({ fsPath: String(u).replace(/^file:\/\//, "") }) } },
  )[name];
}

const ctxWith = (requestDetailed, extra = {}) => ({
  uri: "file:///x", model: { getValue: () => "", getLanguageId: () => "rust", ...(extra.model || {}) },
  client: { supports: () => true, requestDetailed },
});

const ok = (result) => async () => ({ ok: true, result, reason: "", detail: "" });
const bad = (reason) => async () => ({ ok: false, result: null, reason, detail: `x ${reason}` });

const shapeOf = (r) => (r && r.unanswered === true ? "unanswered" : Array.isArray(r) ? (r.length ? "some" : "empty") : r == null ? "none" : "value");

// ── ① 判别机制本身 ────────────────────────────────────────────────────────
test("requestDetailed 把四条路分开，而 request() 的契约一个字没变", async () => {
  const src = LSP.slice(LSP.indexOf("requestDetailed(method, params, opts)"), LSP.indexOf("async _initialize()"));
  for (const r of ["timeout", "error", "transport"]) {
    assert.match(src, new RegExp(`reason: "${r}"`), `${r} 这条路没有单独的 reason`);
  }
  // request() 必须是薄封装：不能自己再实现一遍超时/发送，否则两份逻辑迟早分叉。
  assert.match(src, /request\(method, params, opts\) \{\s*return this\.requestDetailed\([^)]*\)\.then\(\(r\) => \(r\.ok \? r\.result : null\)\);\s*\}/,
    "request() 不再是 requestDetailed 的薄封装 —— 两份实现会分叉");
  assert.doesNotMatch(src.slice(src.indexOf("request(method, params, opts)")), /setTimeout/,
    "request() 里又出现了自己的计时器");
});

test("服务器回 JSON-RPC error 走 reject，不和「答了 null」挤成同一个值", () => {
  const seg = LSP.slice(LSP.indexOf("_onMessage(raw)"), LSP.indexOf("_handleServerRequest(msg)"));
  assert.match(seg, /if \(msg\.error\) \{ if \(pending\.reject\) pending\.reject\(msg\.error\); else pending\.resolve\(null\); \}/,
    "error 又被压成 resolve(null) 了");
});

test("语言服务器停掉时，在等的请求算「没答上来」，不算「答了没有」", () => {
  const seg = LSP.slice(LSP.indexOf("shutdown() {"), LSP.indexOf("function clientCapabilities"));
  assert.match(seg, /if \(reject\) reject\(/, "shutdown 把待答请求 resolve(null) 了 —— 一次进程退出会被说成「这个符号没有引用」");
});

// ── ② 四个 agent 方法 ─────────────────────────────────────────────────────
test("agentLocate：超时/报错/断线/空应答一律是「没查成」，只有真的空数组才是「没有引用」", async () => {
  const run = (rd) => methodOf("agentLocate")(ctxWith(rd))("/x.rs", 1, 0, "references");
  assert.equal(shapeOf(await run(ok([]))), "empty", "服务器真回空数组 = 确实没有引用，这个结论不能丢");
  assert.equal(shapeOf(await run(ok([{ uri: "file:///a.rs", range: { start: { line: 3 } } }]))), "some", "有引用的情形被弄坏了");
  for (const r of ["timeout", "error", "transport"]) {
    assert.equal(shapeOf(await run(bad(r))), "unanswered", `${r} 被压成了「没有引用」—— 模型会据此删代码`);
  }
  assert.equal(shapeOf(await run(ok(null))), "unanswered", "服务器回 null 也按保守侧算没查成");
  assert.equal(shapeOf(await run(async () => { throw new Error("socket closed"); })), "unanswered", "抛异常也要算没查成");
  // reason 要带出来，否则上层没法说清是哪一种。
  const r = await run(bad("timeout"));
  assert.equal(r.reason, "timeout", "没把理由带上来");
});

test("agentHover：没答上来 ≠ 这个符号没有类型信息", async () => {
  const run = (rd) => methodOf("agentHover")(ctxWith(rd))("/x.rs", 1, 0);
  assert.equal(shapeOf(await run(ok({ contents: "fn main()" }))), "value", "正常情形被弄坏了");
  // hover 回空是**常态**（空行、注释），不能算没查成，否则模型白重试。
  assert.equal(shapeOf(await run(ok({ contents: "" }))), "none", "hover 真的没内容时不该说成「没查成」");
  assert.equal(shapeOf(await run(ok(null))), "none", "同上");
  for (const r of ["timeout", "error"]) {
    assert.equal(shapeOf(await run(bad(r))), "unanswered", `${r} 被说成了「这个符号没有类型信息」`);
  }
});

test("agentFormat：没答上来 ≠ 这个语言没有格式化器", async () => {
  const run = (rd) => methodOf("agentFormat")(ctxWith(rd, { model: { getValue: () => "a" } }))("/x.rs", {});
  assert.equal(await run(ok([])), "a", "服务器说无需改动时要回原文");
  assert.equal(shapeOf(await run(ok(null))), "none", "服务器明确说不支持格式化 = 无格式化器，这个结论要保留");
  for (const r of ["timeout", "error", "transport"]) {
    assert.equal(shapeOf(await run(bad(r))), "unanswered", `${r} 被说成了「这语言没有格式化器」—— 模型从此不再试`);
  }
});

test("agentDocumentSymbols 照旧，但理由现在是真的", async () => {
  const run = (rd) => methodOf("agentDocumentSymbols")(ctxWith(rd))("/x.rs");
  assert.equal(shapeOf(await run(ok([]))), "empty");
  assert.equal(shapeOf(await run(ok(null))), "unanswered");
  assert.equal((await run(bad("timeout"))).reason, "timeout", "以前只能笼统说「没答上来」，现在要能说出是超时");
});

// ── ③ 调用方：三处都得把这个对象摘出来 ─────────────────────────────────────
test("format 的「没查成」对象绝不能流到写盘那一步", () => {
  const at = MAIN.indexOf("let formatted = null;");
  const end = MAIN.indexOf("writeTextFileIfUnchanged(fp, old, formatted)", at);
  assert.ok(at > 0 && end > at, "format 那段不见了");
  const seg = MAIN.slice(at, end);
  const iGuard = seg.indexOf("formatted.unanswered === true");
  assert.ok(iGuard > 0, "没摘 unanswered —— 这个对象会被写进用户的文件，内容是 [object Object]");
  assert.ok(seg.indexOf("_diffStat(old, formatted)") > iGuard, "摘的位置在写盘之后就没用了");
  assert.match(seg, /\[未完成\][\s\S]{0,400}不等于这个语言没有格式化器/, "没把「没查成」和「没有格式化器」分开告诉模型");
});

test("references 的「没查成」必须明说不许据此删代码", () => {
  const at = MAIN.indexOf("let locs = null;");
  const end = MAIN.indexOf("const seen = new Set();", at);
  assert.ok(at > 0 && end > at, "locate 那段不见了");
  const seg = MAIN.slice(at, end);
  assert.match(seg, /locs\.unanswered === true/, "没摘 unanswered —— 这个对象是 truthy，会掉进 for...of 抛「不可迭代」");
  assert.match(seg, /绝对不要据此删除它、改它的签名或改它的行为/,
    "只说了「没查成」没说后果 —— 模型照样会当成「没人用」");
  assert.match(seg, /search\(/, "没给出「现在就要确定」时的替代路径");
});

test("hover 的「没查成」不许沿用那句「别再试第二次」", () => {
  const at = MAIN.indexOf('if (call.op === "hover") {');
  const end = MAIN.indexOf("let locs = null;", at);
  const seg = MAIN.slice(at, end);
  // 钉判据本身，不钉变量名：把条件改成 if (false) 时变量名照样在，名字守不住。
  const iUn = seg.indexOf("hover.unanswered === true");
  const iDead = seg.indexOf("换个位置或重试不会有不同结果");
  assert.ok(iUn > 0, "hover 没摘 unanswered —— 对象会被 String() 成 [object Object] 当作类型贴给模型");
  assert.ok(iDead > iUn, "「别再试第二次」排在了「没查成」前面 —— 那句话对超时是反的，重试恰恰是对的");
  assert.match(seg, /重试一次是有意义的/, "没告诉模型这种情况该重试");
});

test("交付事实里那条引用查询，不许把「没查成」写成「查不到任何引用」", () => {
  const at = MAIN.indexOf("const _tExt = (t.abs.split");
  const end = MAIN.indexOf("本文件之外查不到它的任何引用", at);
  assert.ok(at > 0 && end > at, "交付事实里的引用查询那段不见了");
  const seg = MAIN.slice(at, end);
  assert.match(seg, /if \(!Array\.isArray\(_refs\)\) continue;/,
    "守卫没了 —— 「没查成」会被当成事实写进模型看的那个块");
});

// ── ④ 语言服务器死了，得说出为什么 ─────────────────────────────────────────
//
// `LspEvent::Error` 以前是**声明了从来没构造过**的死变体（枚举上挂着 #[allow(dead_code)]
// 就是证据），语言服务器的 stderr 只进了 tracing::debug!，默认级别下谁都看不见。
// 于是 jdtls 找不到 JDK 起来就退、pyright 因为 node 太老崩掉，用户看到的都是一句
// 光秃秃的「已停止」，模型看到的是「这个语言没有可用的服务（可能未装）」——照着
// 「可能未装」去装一遍，装完还是不行。
const RS = readFileSync(new URL("../src-tauri/src/lsp.rs", import.meta.url), "utf8");

test("stderr 要真的送上来，Error 这一支不许再是死变体", () => {
  const en = RS.slice(RS.indexOf("pub enum LspEvent"), RS.indexOf("struct LspProcess"));
  assert.ok(en.length > 50, "LspEvent 不见了");
  assert.doesNotMatch(RS.slice(RS.indexOf("pub enum LspEvent") - 300, RS.indexOf("pub enum LspEvent")),
    /#\[allow\(dead_code\)\]/,
    "allow(dead_code) 又回来了 —— 那是这个变体从没被构造过的证据，撤掉它才能让编译器盯着");
  assert.match(RS, /LspEvent::Error \{/, "Error 变体还是没有任何构造点");
  // stderr 线程里构造，不是别处。
  const th = RS.slice(RS.indexOf("let reader = BufReader::new(stderr);"), RS.indexOf("let _ = on_event.send(LspEvent::Started"));
  // 钉真正的判据（那个 if 的条件），不钉出现过的词：改成 `if false {` 时
  // STDERR_FORWARD_MAX / looks_bad / LspEvent::Error 三个词照样都在。
  assert.match(th, /if forwarded < STDERR_FORWARD_MAX \|\| looks_bad \{/,
    "转发的判据被改掉了 —— 有上限（别把前端日志缓冲刷爆）但错误行不受上限限制（崩溃常常发生在很久以后）");
  assert.match(th, /LspEvent::Error \{/, "stderr 没有构造 Error 事件");
});

test("stopped 事件要带上死前的最后几行", () => {
  assert.match(RS, /Stopped \{ lang: String, tail: Vec<String> \}/, "Stopped 没带 tail");
  const stop = RS.slice(RS.indexOf("let mut tail = tail_for_stop"), RS.indexOf("LspEvent::Stopped { lang, tail }") + 40);
  assert.ok(stop.length > 40, "取 tail 那段不见了");
  assert.match(stop, /\.iter\(\)\.cloned\(\)\.collect/, "没真的把环形缓冲取出来");
  const loop = RS.slice(RS.indexOf("let mut reader = BufReader::new(stdout);"), RS.indexOf("LspEvent::Stopped { lang, tail }"));
  assert.match(loop, /sleep\(std::time::Duration::from_millis/,
    "stdout EOF 之后没等 stderr 收尾 —— 最能说明死因的那几行恰好赶不上这班车");
});

test("前端把死因说给用户，也留给模型", () => {
  const ev = LSP.slice(LSP.indexOf('case "stopped"'), LSP.indexOf('default:'));
  assert.match(ev, /Array\.isArray\(ev\.tail\)/, "没接 tail");
  assert.match(ev, /_handleStopped\(this\.lang, this, tail\)/, "tail 没往下传");

  const h = LSP.slice(LSP.indexOf("function _handleStopped"), LSP.indexOf("async function ensureServer"));
  assert.match(h, /showToast\(/, "用户那边一句话都没有");
  assert.match(h, /lastStopReason\.set\(langId, why\)/, "没留给模型 —— 工具回执还是只会说「可能未装」");
  // 起来了要清掉，否则拿上一次的失败解释这一次。
  assert.match(LSP, /lastStopReason\.delete\(langId\)/, "启动成功后没清掉旧死因");

  // 工具回执四条都要带上。
  for (const anchor of ["没有可用的符号服务", "没有可用的${label}服务", "没有可用的语言格式化器", "项目内的用 lsp_definition"]) {
    const i = MAIN.indexOf(anchor);
    assert.ok(i > 0, `回执「${anchor}」不见了`);
    const seg = MAIN.slice(i, i + 260);
    assert.match(seg, /\$\{_(?:lsp|fmt)DeadWhy\}/, `「${anchor}」这条回执没带死因，模型还是只知道「可能未装」`);
  }
});

// ── ⑤ 「取环境符号」那四条路一直没有工作区信任门 ────────────────────────────
//
// lsp_start 把它写得很仔细：未信任的工作区不把它的目录算进 PATH，只用系统装的语言
// 服务器。可同一个进程里另外四条路绕过了它：
//   · lsp_detect_python 直接执行 `<工作区>/.venv/bin/python`
//   · lsp_node_env_symbols 用 require() 加载工作区 node_modules 里的包（跑它的顶层代码）
//   · lsp_go_env_symbols 在别人的仓库里跑 go list（go.mod 的 toolchain 指令会下载并执行别的工具链）
//   · lsp_lang_env_symbols 在别人的仓库目录里跑 dart pub 之类
// 也就是说：clone 一个仓库、点开任意一个 .py 或 .ts 文件，仓库自带的可执行文件就跑起来了。
test("四条取环境符号的路都要过工作区信任门，且缺省 fail closed", () => {
  assert.match(RS, /fn workspace_trusted\(flag: Option<bool>\) -> bool \{\s*flag\.unwrap_or\(false\)/,
    "缺省不是 false —— 老客户端不传这个参数时就等于门不存在");

  const fnBody = (name) => {
    const i = RS.indexOf(`pub fn ${name}(`);
    assert.ok(i > 0, `${name} 不见了`);
    const j = RS.indexOf("\n}\n", i);
    return RS.slice(i, j);
  };

  // python：不信任就连工作区都不看（既不挑它的 venv，也不进 PATH）。
  const py = fnBody("lsp_detect_python");
  assert.match(py, /trust_workspace_binaries: Option<bool>/, "lsp_detect_python 没有信任参数");
  assert.match(py, /let scope = if trusted \{ ws \} else \{ None \};/,
    "不信任时还把工作区传给了 pick_python —— 那就会挑中仓库自带的 .venv/bin/python 并执行它");
  assert.match(py, /pick_python\(scope\)/, "pick_python 没走 scope");
  assert.match(py, /augmented_path\(scope\)/, "PATH 没走 scope");

  // node：列包名（读目录）安全，require 不安全。
  const nd = fnBody("lsp_node_env_symbols");
  assert.match(nd, /trust_workspace_binaries: Option<bool>/, "lsp_node_env_symbols 没有信任参数");
  assert.match(nd, /if !modules\.is_empty\(\) && trusted \{/,
    "require() 那步没过门 —— 它会执行工作区里那个包的顶层代码");
  assert.match(nd, /if let Ok\(entries\) = std::fs::read_dir\(&node_mods\)/,
    "把列包名也一起关掉了 —— 读目录名是安全的，不该跟着降级");

  for (const name of ["lsp_go_env_symbols", "lsp_lang_env_symbols"]) {
    const b = fnBody(name);
    assert.match(b, /trust_workspace_binaries: Option<bool>/, `${name} 没有信任参数`);
    assert.match(b, /if !workspace_trusted\(trust_workspace_binaries\) \{\s*return Ok\(/,
      `${name} 拿到参数却没用它挡住`);
  }
});

test("信任状态在 backend 包装那一层带上，不是在每个调用点", () => {
  // 调用点有六处，漏一处这道门就是虚的。
  for (const w of ["lsp_detect_python", "lsp_node_env_symbols", "lsp_go_env_symbols", "lsp_lang_env_symbols"]) {
    const i = MAIN.indexOf(`core.invoke("${w}"`);
    assert.ok(i > 0, `${w} 的包装不见了`);
    assert.match(MAIN.slice(i, i + 260), /trustWorkspaceBinaries: isWorkspaceTrusted\(\)/,
      `${w} 的包装没带信任状态 —— 后端 fail closed，这个功能会对所有人静默失效`);
  }
});

test("未信任导致的降级要说出来，不能让用户对着「无法解析」发呆", () => {
  assert.match(RS, /pub untrusted_fallback: bool/, "后端没把降级这件事报上来");
  const i = LSP.indexOf("await backend.lspDetectPython");
  assert.ok(i > 0);
  const seg = LSP.slice(i, i + 700);
  assert.match(seg, /info\.untrustedFallback/, "前端没接这个标记");
  assert.match(seg, /信任这个工作区/, "没告诉用户怎么解决 —— 这恰恰是他一键能解决的问题");
});

// ── ⑥ 读线程退出时必须摘掉自己，否则这门语言整个会话静默死亡 ──────────────
test("读循环退出前先把这门语言从 map 里摘掉，再发 stopped", () => {
  const loop = RS.slice(RS.indexOf("let mut reader = BufReader::new(stdout);"), RS.indexOf("LspEvent::Stopped { lang, tail }"));
  assert.ok(loop.length > 100, "读循环不见了");
  assert.match(loop, /reap\(&reap_map, &lang\);/,
    "读线程退出时没摘掉自己 —— 进程还活着 → prune_stopped 保留 → 下次 lsp_start 撞上 "
    + "already running → 前端静默 return。这门语言整个会话再也没有补全/诊断/跳转，"
    + "界面上一个字都没有，而那个几百 MB 的进程一直挂着");
  // 顺序：先摘再发事件。反了的话前端可能在记录还在时就去重启。
  assert.ok(loop.indexOf("reap(&reap_map") < loop.length, "reap 跑到发事件后面去了");
  // 非法帧的原因要带出去，不能只留一句「已停止」。
  assert.match(RS, /frame_error = format!\("协议帧读不懂/, "非法帧的原因被丢了");
  assert.match(RS, /if !frame_error\.is_empty\(\)\s*\{\s*tail\.push\(frame_error\);/, "原因没接进 stopped 的 tail");
});

test("Windows 也要走 resolve_command——npm 装的语言服务器全是 .cmd", () => {
  // Rust 的 Command 在 Windows 上走 CreateProcessW，只补 .exe、不查 PATHEXT。
  // 而 typescript-language-server / bash-language-server / yaml-language-server /
  // docker-langserver / vue-language-server / intelephense / graphql-lsp 全是 npm 装的 *.cmd。
  // 更难查的是表现：lsp_check_available 的 Windows 分支**会**扫 .cmd，于是返回「装了」，
  // 前端走的是「启动失败」而不是「去装一个」——用户看到「明明装好了却没有补全」。
  for (const [file, src] of [["lsp.rs", RS], ["debug.rs", readFileSync(new URL("../src-tauri/src/debug.rs", import.meta.url), "utf8")]]) {
    assert.doesNotMatch(src, /#\[cfg\(windows\)\]\s*\n\s*let resolved = command\.clone\(\);/,
      `${file}：Windows 分支又回到裸名字 spawn —— npm 装的适配器/语言服务器一个都起不来`);
    assert.match(src, /let resolved = process_util::resolve_command\(/, `${file}：resolve_command 没了`);
  }
});
