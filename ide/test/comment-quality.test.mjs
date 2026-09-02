import test from "node:test";
// 这一对 2026-08-25 搬进了 src/agent/code-text.js —— 直接 import 真模块，
// 不再抠源码：抠源码验得到行为，验不到它在真实调用链上还在不在。
import { splitCodeAndComments as _splitCC, symbolPatternsFor as _symPat } from "../src/agent/code-text.js";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { load, SRC } from "./helpers/source.mjs";

/*
 * 两条判据，回答用户的两句话：
 *   ·「AI 写代码不怎么写注释」            → _missingWhyInWrite
 *   ·「有的会用旧注释让 IDE 发现不了问题」 → _staleCommentFindings
 *
 * 两条的判据都是**量出来的**，被否掉的那些也记在下面——免得下次有人再想一遍。
 */
const split = _splitCC;
const why = load("_missingWhyInWrite", { _splitCodeAndComments: split });
const stale = load("_staleCommentFindings", {
  _splitCodeAndComments: split,
  _CODE_FILE_RE: /\.(?:tsx?|jsx?|mjs|py|rs|go|java|rb|php|cs|swift|kt)$/i,
});
const mkRun = (content, current, path = "/p/a.ts") =>
  ({ checkpoint: new Map([[path, { content, current }]]) });

// ══ 一、该说一句为什么的地方 ═════════════════════════════════════════
test("会改变结果的操作被静默吞掉，而一个字都没解释", () => {
  const bad = `async function save(p, d) {
  try { await writeFile(p, d); } catch {}
  return true;
}`;
  const hits = why(bad, "a.js");
  assert.equal(hits.length, 1, `没点名：${JSON.stringify(hits)}`);
  assert.equal(hits[0].kind, "静默吞掉一次真实操作");
  assert.equal(hits[0].line, 2);
  assert.match(hits[0].ask, /该发生的事没发生/, "只说了「有问题」，没说清后果");

  // 解释了就闭嘴——这条判据的全部意义就是逼出那一句话。
  const ok = `async function save(p, d) {
  // 落盘失败无所谓：这只是缓存，下次会重算。
  try { await writeFile(p, d); } catch {}
  return true;
}`;
  assert.deepEqual(why(ok, "a.js"), [], "解释过了还在念");

  // 不改变结果的操作不算——try { unsubscribe(); } catch {} 完全正常。
  assert.deepEqual(why(`function off(u) {\n  try { u(); } catch {}\n}`, "a.js"), [],
    "把「取消订阅失败无所谓」这种也报了 —— 本仓库 21 万行里这种有 923 处，报它等于刷屏");
});

test("抑制了一条检查却没写为什么", () => {
  const hits = why(`// @ts-ignore\nconst x = y.z;\n`, "a.ts");
  assert.equal(hits.length, 1);
  assert.equal(hits[0].kind, "抑制了检查却没说为什么");
  assert.match(hits[0].ask, /什么条件下[\s\S]{0,20}撤掉/, "没说清楚要写什么");
  assert.deepEqual(why(`// 上游类型定义漏了这个字段，已提 issue #421\n// @ts-ignore\nconst x = y.z;\n`, "a.ts"), [],
    "上面已经写了理由还在报");
});

test("具名的上限/超时常量没解释", () => {
  const hits = why(`const REQUEST_TIMEOUT_MS = 20000;\n`, "a.js");
  assert.equal(hits.length, 1);
  assert.equal(hits[0].kind, "这个数字是怎么定的");
  assert.match(hits[0].ask, /实测出来的、上游的硬限制、还是先拍一个/, "没给出该回答什么");
  assert.deepEqual(why(`// 上游网关 30s 断连，留 10s 余量\nconst REQUEST_TIMEOUT_MS = 20000;\n`, "a.js"), []);
  // 没有名字的裸数字不算——那条量下来每 495 行一处，太吵。
  assert.deepEqual(why(`if (n > 20000) return;\n`, "a.js"), []);
});

test("文件超过 60 行还一条注释都没有", () => {
  const noComment = Array.from({ length: 70 }, (_, i) => `const v${i} = ${i};`).join("\n");
  const hits = why(noComment, "a.js");
  assert.ok(hits.some((h) => h.kind === "整份文件零注释"), "没报");
  assert.match(hits.find((h) => h.kind === "整份文件零注释").ask, /146 个真实文件里没有一个/,
    "没说清这不是风格偏好 —— 这条判据在本仓库 146 个文件上命中 0 次，是标准本身");
  // 短文件不管。
  assert.deepEqual(why(Array.from({ length: 20 }, (_, i) => `const v${i} = ${i};`).join("\n"), "a.js"), []);
  // 有一条注释就够，不是要求密度。
  assert.deepEqual(why("// 这个文件负责把配置合并成运行时快照。\n" + noComment, "a.js")
    .filter((h) => h.kind === "整份文件零注释"), []);
});

test("被否掉的那几条判据不许偷偷回来", () => {
  // 全部量过：长函数零注释 每 437 行、裸空 catch 每 230 行、裸魔法数字 每 495 行、
  // 正则无注释 每 1505 行、导出函数无说明 每 1829 行 —— 都不合格。
  const longFn = "function f() {\n" + Array.from({ length: 40 }, (_, i) => `  const a${i} = ${i};`).join("\n") + "\n}\n";
  assert.deepEqual(why(longFn, "a.js").filter((h) => h.kind !== "整份文件零注释"), [],
    "「长函数零注释」又被加回来了 —— 每 437 行响一次，三天后没人再看这条提醒");
  assert.deepEqual(why("const re = /^(?:abc|def|ghi)+$/g;\n", "a.js"), [],
    "「正则无注释」又被加回来了");
});

// ══ 二、旧注释 ═══════════════════════════════════════════════════════
test("紧邻的常量改了，注释还写着旧的", () => {
  const before = `const _TOOL_PAYLOAD_MAX_TOOLS = 128;
const _TOOL_PAYLOAD_MAX_SCHEMA_BYTES = 512 * 1024;
// 仍远低于 128 / 512 KiB 的总窗口。`;
  const hits = stale(mkRun(before, before.replace("= 128;", "= 256;")));
  assert.equal(hits.length, 1, `没抓到：${JSON.stringify(hits)}`);
  assert.equal(hits[0].token, "128");
  assert.equal(hits[0].line, 3);
});

test("四种豁免：注释也改了 / 讲历史 / 新写的注释 / 离改动太远", () => {
  const before = `const _CAP = 128;\n// 上限是 128，够用。`;
  assert.deepEqual(stale(mkRun(before, before.replace("= 128;", "= 256;").replace("上限是 128", "上限是 256"))), [],
    "注释也改对了还在报");
  const hist = `const _CAP = 128;\n// 原来是 128，抬上来是因为目录已经 138 个。`;
  assert.deepEqual(stale(mkRun(hist, hist.replace("= 128;", "= 256;"))), [],
    "讲历史的注释被报了 —— 记录旧值正是它们的用途，这个仓库大量注释是事故复盘");
  assert.deepEqual(stale(mkRun(`const _CAP = 128;`, `const _CAP = 256;\n// 上限是 128`)), [],
    "这一轮新写的注释被算成了旧注释");
  const far = `const _CAP = 128;\n` + "x();\n".repeat(40) + `// 上限是 128`;
  assert.deepEqual(stale(mkRun(far, far.replace("= 128;", "= 256;"))), [],
    "隔着 40 行也报 —— 那不是「这次改动留下的旧注释」");
});

test("散文里的普通英文词不算标识符", () => {
  // 第一版什么词都认，标定时 Notification / Response / Items 全成了误报 ——
  // 它们是散文里的普通英文词，恰好也长得像标识符。
  // 这里必须让它**真的被声明过**，否则挡住它的是另一条判据（局部声明），测不到形态那条。
  const before = `class Notification {}\nconst n = new Notification();\n// 这里用的是 webview 的 Notification。`;
  const after = `class Toast {}\nconst n = new Toast();\n// 这里用的是 webview 的 Notification。`;
  assert.deepEqual(stale(mkRun(before, after)), [],
    "普通 CamelCase 英文词又被当成标识符了");
  // 反向：同样的形状，名字换成下划线打头的就该报——形态判据只挡英文散文词，不挡真标识符。
  const b2 = `const _notifyImpl = 1;\nconst n = _notifyImpl;\n// 走的是 _notifyImpl 那条。`;
  const a2 = `const _toastImpl = 1;\nconst n = _toastImpl;\n// 走的是 _notifyImpl 那条。`;
  assert.equal(stale(mkRun(b2, a2)).length, 1, "真标识符改名后旧注释没被抓到");
});

test("判据是局部的——大文件里「全文消失」那种判法抓不到任何东西", () => {
  const body = load("_staleCommentFindings", { _splitCodeAndComments: split, _CODE_FILE_RE: /x/ });
  const src = SRC.slice(SRC.indexOf("function _staleCommentFindings"), SRC.indexOf("function _hardcodedDeliveryFindings"));
  assert.match(src, /const NEAR = 12;/, "局部窗口没了");
  assert.match(src, /b\.code\.slice\(Math\.max\(0, j - NEAR\), j \+ NEAR \+ 1\)/, "改前那侧不再取局部窗口");
  assert.match(src, /localDecl/, "不再要求那个记号在局部被声明过");
  assert.ok(typeof body === "function");
});

// ── 活的标定：拿真实提交跑 ────────────────────────────────────────────
test("在真正引入漂移的那次提交上抓得到（活的标定）", () => {
  // 7690ef5 把 _TOOL_PAYLOAD_MAX_TOOLS 从 128 抬到 256，四行之外那句注释没跟上。
  // 这条漂移是靠人工取样复核出来的；这里断言机器也抓得到同一句。
  let before, after;
  try {
    // 用 fileURLToPath 而不是裸 .pathname：Windows 上 `new URL(...).pathname` 得到的是
    // `/C:/Users/...` —— **前面多一个斜杠**，那不是合法的 Win32 路径，execFileSync 的 cwd
    // 会直接失败。这是 import.meta.url 最经典的一条跨平台坑。
    const git = (...a) => execFileSync("git", a, { cwd: fileURLToPath(new URL("..", import.meta.url)), maxBuffer: 512 * 1024 * 1024 }).toString();
    before = git("show", "7690ef5^:src/main.js");
    after = git("show", "7690ef5:src/main.js");
  } catch {
    return; // 提交不可达（浅克隆 / 变基过）——不作为失败
  }
  const hits = stale(mkRun(before, after, "/x/src/main.js"), 10);
  assert.equal(hits.length, 1, `真实提交上命中 ${hits.length} 条，期望恰好 1`);
  assert.equal(hits[0].token, "128");
  assert.match(hits[0].text, /仍远低于 128 \/ 512 KiB/);
});

test("挂在交付事实那条路上，且说清了后果", () => {
  const i = SRC.indexOf("for (const c of _staleCommentFindings(run))");
  assert.ok(i > 0, "没接到交付事实里 —— 检测器写了没人调");
  const seg = SRC.slice(i - 400, i + 500);
  assert.match(seg, /照它理解会得出一个已经不成立的结论/, "只说了「注释旧了」，没说为什么要管");
  assert.match(SRC, /_missingWhyInWrite\(body, call\?\.path \|\| ""\)/, "缺注释那条没挂到写入建议出口上");
});

// ── 三、注释点名了一个这一轮被删掉的本地符号 ────────────────────────────
//
// 人工取样里这一类占 5/20：`_stopSessionRun` / `_warmupWorkspaceAgent` /
// `_predictComposerNext` / `_thinkingRequestParams` —— 注释理直气壮地说「走的是它那条」，
// 而那个函数全仓根本不存在。照它去理解，会顺着一条不存在的路径找半天。
test("注释点名的本地符号被这一轮删掉了", () => {
  const before = "function _oldHelper(x) { return x; }\n"
    + "// 走的是 `_oldHelper()` 那条路，不要绕开它。\n"
    + "export function run(x) { return _oldHelper(x); }";
  const after = "// 走的是 `_oldHelper()` 那条路，不要绕开它。\n"
    + "export function run(x) { return inline(x); }";
  const hits = stale(mkRun(before, after));
  assert.equal(hits.length, 1, `没抓到：${JSON.stringify(hits)}`);
  assert.equal(hits[0].token, "_oldHelper");
});

test("跨文件引用不算——测试和脚本本来就在引别的文件的符号", () => {
  // 标定时这是唯一的误报来源：542 次真实改动里那 3 条（_KNOWN_TOOLS /
  // _mcpServerApprovalMode / _live）全是 test/ 和 scripts/ 在点名主文件里的符号。
  const before = "// 走的是 `_otherFile()` 那条路。\nexport function run(x) { return x + 1; }";
  assert.deepEqual(stale(mkRun(before, before.replace("x + 1", "x + 2"))), [],
    "本来就不在这份文件里的名字被当成了「这一轮删掉的」");
});

test("全文件那条分支只认「被点名」，不认「顺口提到」", () => {
  /*
   * 两条分支的严格程度不一样，这是有意的：
   *   · 局部分支（值/名字在注释旁边那十几行里变了）——顺口提到也算，因为「旁边刚改过」
   *     本身就是很强的证据。上面那条 `_oldHelper` 被删的用例走的就是它。
   *   · 全文件分支（这个名字整份文件里都没有了）——只认反引号包着或写成 `_foo()` 的
   *     **点名**。散文里提一句和点名它是两回事，标定时那 3 条误报全出在这上面。
   */
  const src = SRC.slice(SRC.indexOf("function _staleCommentFindings"), SRC.indexOf("function _hardcodedDeliveryFindings"));
  assert.match(src, /const named = new Set\(\);/, "全文件分支没了");
  assert.match(src, /note\.matchAll\(\/`\(_\[A-Za-z\]/, "点名的判据被放宽成「提到就算」了");
  assert.match(src, /fileTokens\.has\(t\) \|\| !beforeFileTokens\.has\(t\)/,
    "「这一轮弄没的」这条没了 —— 跨文件引用会全变成误报");
});

// ── 四、底座：跨行字符串与正则字面量 ────────────────────────────────────
test("正则字面量里的斜杠不是注释起点", () => {
  /*
   * 这是已上线代码里的一个真 bug，标定时才发现：main.js 里有一处 url.replace(正则, "")，
   * 那个正则以「反斜杠 斜杠 星号」结尾——在扫描器眼里就是块注释的起点，于是从那一行起
   * **整片文件都被判成注释**，7440 行那个 function 直接进了 comments。所有只看代码的
   * 判据在那之后全哑了，而且一声不响。
   */
  const src = String.raw`const a = url.replace(/^sqlite:\/*/, "");` + "\nfunction _later() { return 1; }";
  const { code, comments } = split(src, "a.js");
  assert.match(code[1], /function _later/, "正则之后的整片代码被判成了注释");
  assert.equal(comments[1], "", "代码跑到 comments 里去了");
  // 真的除号不能被当成正则开头。
  assert.match(split("const r = a / b; // 注释", "a.js").code[0], /a \/ b;/);
  assert.equal(split("const r = a / b; // 注释", "a.js").comments[0].trim(), "注释");
  // 字符类里的 / 不算收尾：/[^/]+/ 的第一个 / 在方括号里，扫描器提前收尾的话
  // 后面那段代码会整片被吃掉。
  const cls = split(String.raw`const m = s.match(/[^/]+\//); // 注释` + "\nconst z = 1;", "a.js");
  assert.match(cls.code[0], /\[\^\/\]\+/, "字符类里的斜杠把正则提前收尾了");
  assert.equal(cls.comments[0].trim(), "注释");
  assert.match(cls.code[1], /const z = 1;/, "正则之后的代码被吃掉了");
});

test("跨行的模板串和三引号要跨行保持", () => {
  const html = ["const h = `", '  <a onclick="_go(1)">x</a>', "  // 这不是注释", "`;", "// 这才是"].join("\n");
  const r = split(html, "a.js");
  assert.match(r.code[2], /这不是注释/, "模板串里的 // 被当成了注释起点");
  assert.equal(r.comments[2], "", "模板串内容跑到 comments 里去了");
  assert.equal(r.comments[4].trim(), "这才是");
  const py = split(['s = """', "多行 # 不是注释", '"""', "x = 1  # 真注释"].join("\n"), "a.py");
  assert.match(py.code[1], /不是注释/, "Python 三引号没跨行");
  assert.equal(py.comments[3].trim(), "真注释");
});
