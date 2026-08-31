// "正在控制电脑" 红光覆盖层：什么时候该亮、什么时候不该亮。
//
// 两种错法代价都不小：
//   · 该亮不亮 —— agent 在无声地搬用户的窗口、点用户的菜单、回放录制好的鼠标序列，
//     用户完全不知道电脑正在被操作。这是这个功能存在的全部理由。
//   · 不该亮却亮 —— 只读调用（screen.info、read_screen、window.list）在自动化里每一步
//     都会发生，跟着亮就等于常亮，红光很快被无视，等于没有。
//
// 还有一类容易搞错的：browser.* 跑在 automation-server 自己拉起的独立 Chrome 里，
// 用户的鼠标键盘屏幕一概不碰，不该亮。
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
// 正向源码断言必须跑在**剥掉注释**的源码上。注释不是代码：把一条契约从代码里删掉、
// 只在注释里留一句，assert.match 照样绿——本仓库已经这样漏过一整组模型可见的工具契约。
// 所以 `SRC` 绑定的是 CODE（注释整段置空，行号与偏移和原文一字不差）；
// 真要匹配注释本身的断言显式用 RAW_SRC，并在那一行写清为什么。
import { CODE as SRC, SRC as RAW_SRC, fnSource, fnSource as extractFn, blockFrom } from "./helpers/source.mjs";

function decl(name) {
  const i = RAW_SRC.indexOf(`const ${name} = `);
  assert.ok(i >= 0, `找不到 ${name}`);
  return SRC.slice(i, RAW_SRC.indexOf("\n]);", i) >= 0 && RAW_SRC.indexOf("\n]);", i) < RAW_SRC.indexOf(";\n", i) + 2
    ? RAW_SRC.indexOf("\n]);", i) + 4
    : RAW_SRC.indexOf(";", RAW_SRC.indexOf(")", i)) + 1);
}

const drives = new Function(
  `${decl("_DESKTOP_READONLY_AUTOMATION")}\n${decl("_DESKTOP_DRIVING_SYSTEM_OPS")}\n${extractFn("_callDrivesDesktop")}\n;return _callDrivesDesktop;`,
)();

// —— 真的在动用户的电脑：必须亮 ——
const DRIVING = [
  ["合成点击", { type: "automation", method: "mouse.click" }],
  ["移动指针", { type: "automation", method: "mouse.move" }],
  ["拖拽", { type: "automation", method: "mouse.drag" }],
  ["滚动", { type: "automation", method: "mouse.scroll" }],
  ["敲键盘", { type: "automation", method: "keyboard.type" }],
  ["组合键", { type: "automation", method: "keyboard.combo" }],
  ["粘贴", { type: "automation", method: "keyboard.paste" }],
  ["回放录制的鼠标键盘序列", { type: "automation", method: "recorder.replay" }],
  ["把用户的窗口提到前台", { type: "automation", method: "window.activate" }],
  ["最小化用户的窗口", { type: "automation", method: "window.minimize" }],
  ["覆盖用户剪贴板", { type: "automation", method: "clipboard.set" }],
  ["打开/切换 App", { type: "system", op: "open" }],
  ["切换前台窗口", { type: "system", op: "focus" }],
  ["点菜单项", { type: "system", op: "menu" }],
  ["AX 按下控件", { type: "uiclick", ref: 3, action: "press" }],
  ["AX 填入文本", { type: "uiclick", ref: 3, action: "set_value" }],
];

// —— 只读或不沾用户桌面：不该亮 ——
const PASSIVE = [
  ["读屏幕尺寸", { type: "automation", method: "screen.info" }],
  ["读指针位置", { type: "automation", method: "mouse.position" }],
  ["列窗口", { type: "automation", method: "window.list" }],
  ["读剪贴板", { type: "automation", method: "clipboard.get" }],
  ["保存录制", { type: "automation", method: "recorder.save" }],
  ["列录制", { type: "automation", method: "recorder.list" }],
  ["初始化", { type: "automation", method: "system.init" }],
  ["独立 Chrome 打开网页", { type: "automation", method: "browser.goto" }],
  ["独立 Chrome 点击", { type: "automation", method: "browser.click" }],
  ["独立 Chrome 输入", { type: "automation", method: "browser.type" }],
  ["独立 Chrome 截图", { type: "automation", method: "browser.screenshot" }],
  ["列 App", { type: "system", op: "apps" }],
  ["列某 App 的窗口", { type: "system", op: "windows" }],
  ["列菜单项", { type: "system", op: "menu_items" }],
  ["查前台是谁", { type: "system", op: "frontmost" }],
  ["system 默认 op", { type: "system" }],
  ["读文件", { type: "read", path: "a.js" }],
  ["跑命令", { type: "cmd", command: "ls" }],
  ["空调用", null],
  ["没有 method", { type: "automation" }],
];

for (const [what, call] of DRIVING) {
  test(`亮红光：${what}`, () => {
    assert.equal(drives(call), true,
      `${JSON.stringify(call)} 真的在操作用户的电脑，必须亮红光——否则用户不知道电脑被动了`);
  });
}

for (const [what, call] of PASSIVE) {
  test(`不亮红光：${what}`, () => {
    assert.equal(drives(call), false,
      `${JSON.stringify(call)} 不动用户桌面，亮红光会让指示灯常亮、失去意义`);
  });
}

test("大小写不影响判定", () => {
  assert.equal(drives({ type: "automation", method: "Mouse.Click" }), true);
  assert.equal(drives({ type: "automation", method: "BROWSER.GOTO" }), false);
  assert.equal(drives({ type: "automation", method: " mouse.click " }), true);
});

test("三个真驱动的执行分支都接上了红光", () => {
  // 光有判据没用，得真的在执行点调。以前只有 automation 那一条接了，而且判据是写死的
  // 正则 ^(mouse|keyboard)\.，recorder.replay / window.activate / system / ui_click 全漏。
  // 按 AST 取整个分支块，不要「开放式切片 + 固定字符数」：实测三个窗口分别只盖住
  // 目标分支的 29% / 35% / 76%（automation 3134 字给了 900、system 2548 给了 900、
  // uiclick 2109 给了 1600）。接线挪到分支后半截就照样绿——而这三条守的正是
  // 「红光真的在执行点被点亮」，用户看不见红光时根本不知道桌面正在被自动操作。
  const automation = blockFrom('} else if (call.type === "automation") {', { code: true });
  assert.match(automation, /_callDrivesDesktop\(call\) && \(await _showControlGlow\(\)|_callDrivesDesktop\(call\)\) _showControlGlow\(\)/,
    "automation 分支必须用统一判据，不能再用写死的正则");

  const system = blockFrom('} else if (call.type === "system") {', { code: true });
  assert.match(system, /_callDrivesDesktop\(call\)\) _showControlGlow\(\)/,
    "system 分支（open/focus/menu 会真的搬窗口点菜单）必须亮红光");

  const uiclick = blockFrom('} else if (call.type === "uiclick") {', { code: true });
  assert.match(uiclick, /_showControlGlow\(\)/,
    "ui_click 通过辅助功能真的操作前台 App，必须亮红光");
});

test("红光在运行收尾时一定会灭", () => {
  // 原来钉的是 `_hideControlGlow(); // 灭掉红光` —— 后半截是注释，删掉调用只留注释这条
  // 断言照样绿。改钉真代码里的相邻关系：收尾把流关掉的同一处，紧接着必须熄灯。
  assert.match(SRC, /_setStreaming\(session, false\);\s*_hideControlGlow\(\);/,
    "运行结束的收尾里必须紧跟着熄灯——灭不掉的红光比不亮更糟");
});

test("覆盖层本身是透明穿透的，且带着说明文字", () => {
  const overlay = readFileSync(join(HERE, "..", "overlay.html"), "utf8");
  assert.match(overlay, /正在控制电脑/, "必须告诉用户这红光是什么意思");
  assert.match(overlay, /background:\s*transparent/, "背景必须透明，否则会盖住整个屏幕");
  assert.match(overlay, /pointer-events:\s*none/, "必须点击穿透，不能挡住用户操作");
});

test("透明窗口所需的构建开关是开的", () => {
  // tauri 在 macOS 上的透明窗口走私有 API。开关没开时 transparent:true 被静默忽略，
  // 覆盖层就变成一块不透明的全屏白窗——比不显示糟得多。
  const conf = readFileSync(join(HERE, "..", "src-tauri", "tauri.conf.json"), "utf8");
  assert.match(conf, /"macOSPrivateApi":\s*true/, "透明覆盖层需要 macOSPrivateApi");
  const cargo = readFileSync(join(HERE, "..", "src-tauri", "Cargo.toml"), "utf8");
  assert.match(cargo, /default\s*=\s*\[[^\]]*"macos-private-api"/,
    "对应的 cargo feature 必须默认开，不能指望 CLI 自动加");
});

test("桌面控制失败时会把真实的权限诊断带给模型", () => {
  // 光在 Rust 里算出诊断没用，得送到模型手上——否则用户听到的还是那句
  // 「去系统设置勾选 Mr. Day One」，而他看到的就是它已经勾着。
  assert.match(SRC, /async function _desktopPermissionNote\(scope = "ax"\)/,
    "必须有统一的诊断取用点，而且要**按能力分域**");
  const note = extractFn("_desktopPermissionNote");
  assert.match(note, /backend\.invoke\("permission_advice", \{ scope \}\)/,
    "诊断要问系统 API，而且要把 scope 带过去");
  assert.doesNotMatch(note, /invoke\("permission_status"\)/,
    "全量那份不能用在失败回执上：它只要三项缺一就出文案，而读屏/点击根本不需要屏幕录制——"
    + "一次「ref 已过期」会被贴上一整段「去把屏幕录制移除再重加」，用户照做一遍问题还在");
  assert.match(note, /原样转述/, "要明确要求模型别把诊断改写成「请去打开开关」");

  // 每一条失败路都要附诊断，而且**要传对域**。传错域不是"少提示"，是提示一件和这次
  // 失败无关的事：截屏失败去查辅助功能、读屏失败去查屏幕录制，两种都支使用户做无用功。
  for (const [what, marker, wantScope] of [
    ["automation", 'return { type: "automation", path: _m, content: `[失败]', /_desktopPermissionNote\(_permScopeForMethod\(_m\)\)/],
    ["system", "[系统控制失败]", /_desktopPermissionNote\("ax"\)/],
    ["uiclick", '[失败] ui_click: ${message}', /_desktopPermissionNote\("ax"\)/],
  ]) {
    const at = RAW_SRC.indexOf(marker);
    assert.ok(at > 0, `找不到 ${what} 的失败返回`);
    assert.match(SRC.slice(at - 400, at + 300), wantScope,
      `${what} 失败时必须附上**这一域**的权限诊断`);
  }

  // 分域函数本身要认得那三类，映射反了和不分域一样坏。
  const scope = extractFn("_permScopeForMethod");
  assert.match(scope, /screen\.capture[\s\S]*?return "capture"/, "截屏要归到屏幕录制那一域");
  assert.match(scope, /mouse\.[\s\S]*?return "input"/, "合成键鼠要归到辅助功能那一域");
  assert.match(scope, /return "ax"/, "其余（读屏 / 按 ref 操作 / system）归 AX 域");
});

// 解释某段代码为什么被删的注释，往往会把被删的原文照抄一遍——那会让
// doesNotMatch 永远通不过。断言"这段文案不该再出现"之前必须先剥掉注释。
function stripJsComments(source) {
  // 必须**按上下文扫描**，不能用正则。
  //
  // 上一版是两条正则：先去行注释、再去块注释。它认不出**正则字面量**里的 `/`，于是
  // `!/Chrome|Chromium|Edg\//.test(ua)` 这样的真代码里那个 `\//` 被当成行注释开头，
  // 从那儿一路吃到行尾。实测在 main.js 上吃掉 21.7%（821KB）真代码。
  //
  // 后果是双向的，而且反向更危险：assert.match 会静默变红（还能发现），
  // 而 assert.doesNotMatch 在被吃掉的那片区域里**静默变绿** —— 一条本该守着的禁令
  // 等于没写。本仓库 70 处 doesNotMatch(stripJsComments(SRC), ...) 都建立在这上面。
  //
  // 现在逐字符扫，认得字符串 / 模板串 / 正则字面量三种上下文。`/` 前面若是标识符、数字、
  // `)` 或 `]`，那是除号；否则是正则开头 —— 这是 JS 词法里区分这两者的标准启发式。
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

test("system 失败不再无条件叫用户去勾一个已经勾着的开关", () => {
  // 先切出 system 失败那一小段再剥注释。整份 main.js 不能直接剥——里面的正则字面量
  // 含 /* 序列，块注释规则会从那里一路吃掉几千行，断言就成了空对空。
  const at = RAW_SRC.indexOf("[系统控制失败]");
  assert.ok(at > 0, "找不到 system 失败返回");
  const region = stripJsComments(SRC.slice(at - 900, at + 400));
  assert.doesNotMatch(region, /勾选 Mr\. Day One 后重启/,
    "这句话在授权已失效时是错的：开关本来就勾着，重勾无效，只会把用户引向死路");
  assert.match(region, /_desktopPermissionNote\("ax"\)/,
    "取而代之的必须是真实诊断，而且是 AX 那一域——system.* 和屏幕录制无关");
});

test("启动后有一个能点的授权入口", () => {
  // 后端算出诊断、前端不给入口，等于没做。而且必须是"用户点一下"才弹系统框：
  // 日常用的 AXIsProcessTrusted() 是不提示的变体，永远不会把 App 加进系统设置列表。
  const fn = extractFn("_checkDesktopPermissionsOnStart");
  assert.match(fn, /backend\.invoke\("permission_status"\)/, "先问系统真实状态");
  assert.match(fn, /_confirmDialog\(/, "要给用户一个可点的入口");
  assert.match(fn, /backend\.invoke\("request_accessibility"\)/,
    "点下去要触发带 prompt 的检查，让 macOS 自己把 App 加进列表");
  assert.match(fn, /settings_url/, "系统框不奏效时要能直接跳到对应的设置面板");
  // 只匹配名字会命中函数定义本身，"只定义不调用"照样通过。必须找真正的调用点。
  const calls = stripJsComments(SRC)
    .split("_checkDesktopPermissionsOnStart")
    .filter((_, i) => i > 0)
    .filter((tail, i) => i > 0 || true)
    .length;
  assert.ok(calls >= 2, "除了定义之外必须有调用点——光定义不调用等于没做");
  assert.match(stripJsComments(SRC), /void _checkDesktopPermissionsOnStart\(\);/,
    "启动流程里必须真的调它");
  // 权限齐全时不能打扰用户
  assert.match(fn, /if \(!advice\)/, "没缺权限就该安静退出");
});

// —— 对话框正文的排版 ——
// 这里要的是**剥掉注释**的那一份源码：_dialogBodyHtml 的注释里写着渲染前的原始写法，
// 不剥的话「渲染后不该还剩星号」之类的断言会看见注释里的星号。
const codeOf = (name) => fnSource(name, { code: true });
const dialogHtml = new Function(`${codeOf("_dialogBodyHtml")}\n;return _dialogBodyHtml;`)();

test("强调不会以星号残渣的形式漏到界面上", () => {
  const out = dialogHtml("开关**看起来是开的**，但已失效");
  assert.doesNotMatch(out, /\*/, `渲染后不该还剩星号：${out}`);
  assert.match(out, /<strong[^>]*>看起来是开的<\/strong>/, "**…** 要变成真正的强调");
});

test("换行和分段不会被 HTML 折成一坨", () => {
  const out = dialogHtml("第一段\n\n1. 步骤一\n2. 步骤二");
  assert.match(out, /<p[^>]*>第一段<\/p>/, "空行要分段");
  assert.match(out, /步骤一<br>2\. 步骤二/, "单换行要保留成换行，否则编号步骤会挤在一行");
});

test("正文仍然被转义，不能被内容注入", () => {
  const out = dialogHtml('<img src=x onerror=alert(1)> & "quoted"');
  assert.doesNotMatch(out, /<img/, `原始标签必须被转义：${out}`);
  assert.match(out, /&lt;img/);
  assert.match(out, /&amp;/);
});

test("「」括起来的词也当作强调", () => {
  const out = dialogHtml("缺少「辅助功能」权限");
  assert.match(out, /<strong[^>]*>「辅助功能」<\/strong>/);
});

test("空正文不产出空段落", () => {
  assert.equal(dialogHtml(""), "");
  assert.equal(dialogHtml(null), "");
  assert.equal(dialogHtml("\n\n\n"), "");
});

test("对话框真的用了这个渲染，而不是直接 esc 塞进去", () => {
  const dlg = codeOf("_confirmDialog");
  assert.match(dlg, /_dialogBodyHtml\(body\)/, "正文必须走排版函数");
  assert.doesNotMatch(dlg, /line-height:1\.6">\$\{esc\(body\)\}/, "不能再整段 esc 直塞");
});

test("构建不会悄悄退回 ad-hoc 签名", () => {
  // ad-hoc 签名让 macOS 只能用 cdhash 当签名要求，于是每编译一版，用户的辅助功能/
  // 录屏/自动化/完全磁盘访问授权全部作废——而系统设置里的开关还照样亮着。
  // 把身份写进配置，漏签时构建会直接失败，而不是静悄悄地把所有人的权限打掉。
  const conf = readFileSync(join(HERE, "..", "src-tauri", "tauri.conf.json"), "utf8");
  assert.doesNotMatch(conf, /"signingIdentity":\s*"-"/,
    'signingIdentity 不能是 "-"（ad-hoc）');
  assert.match(conf, /"signingIdentity":\s*"[^"-][^"]*"/,
    "必须指定一个真实的签名身份");
});

// —— michael-design：设计证据要赶在第一个模型回合之前到场 ——

test("设计证据在首个模型回合之前有界等待，不是发完就走", () => {
  // 原来的写法是 _startMichaelDesignPreflight(...) 之后紧跟同步的 _consumeMichaelDesignPreflight()，
  // 中间没有任何 await——检索请求不可能已经回来，brief 最早只能在第二个回合注入。而同一轮的
  // UI 律已经在告诉模型「先用本轮已预取的三轨证据」。模型于是照着不存在的证据定配色和结构。
  const code = stripJsComments(SRC);
  const start = code.indexOf("_startMichaelDesignPreflight({ run, body, isLive: _live });");
  assert.ok(start > 0, "找不到预检启动点");
  const consume = code.indexOf("_consumeMichaelDesignPreflight();", start);
  assert.ok(consume > start, "找不到消费点");
  const between = code.slice(start, consume);
  assert.match(between, /await Promise\.race\(/,
    "启动与消费之间必须有一次有界等待，否则 brief 赶不上第一个回合");
  assert.match(between, /_michaelDesignPreflightPromise/, "等的必须是预检那个 promise");
  assert.match(between, /setTimeout\(resolve, _MICHAEL_DESIGN_PREFLIGHT_WAIT_MS\)/,
    "等待必须有上界，不能无限期卡住首个 token");
  assert.match(between, /designKnowledgeRequired/,
    "只在真的需要设计知识时等，别拖慢所有任务");
});

test("界面活会自动去取设计蓝图，不用用户提醒", () => {
  // 这条进的是随 system 前缀发送、字节稳定的场景直觉表——每一轮都在，
  // 所以模型碰到界面活时的第一反应就是先取蓝图，而不是凭印象编色。
  const hint = codeOf("_buildToolHint");
  assert.match(hint, /michael-design/, "场景直觉表必须把界面活指向 michael-design");
  assert.match(hint, /界面|UI/, "要说清什么算界面活");
});
