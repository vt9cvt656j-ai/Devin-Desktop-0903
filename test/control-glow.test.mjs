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
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");

function extractFn(name) {
  const i = SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `main.js 里找不到 ${name}`);
  let depth = 0, j = SRC.indexOf("{", SRC.indexOf(")", i));
  for (; j < SRC.length; j++) {
    const c = SRC[j], d = SRC[j + 1];
    if (c === "/" && d === "/") { j = SRC.indexOf("\n", j); if (j < 0) j = SRC.length; continue; }
    if (c === "/" && d === "*") { j = SRC.indexOf("*/", j + 2) + 1; continue; }
    if (c === '"' || c === "'" || c === "`") {
      const q = c;
      for (j++; j < SRC.length; j++) { if (SRC[j] === "\\") { j++; continue; } if (SRC[j] === q) break; }
      continue;
    }
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (!depth) break; }
  }
  return SRC.slice(i, j + 1);
}

function decl(name) {
  const i = SRC.indexOf(`const ${name} = `);
  assert.ok(i >= 0, `找不到 ${name}`);
  return SRC.slice(i, SRC.indexOf("\n]);", i) >= 0 && SRC.indexOf("\n]);", i) < SRC.indexOf(";\n", i) + 2
    ? SRC.indexOf("\n]);", i) + 4
    : SRC.indexOf(";", SRC.indexOf(")", i)) + 1);
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
  const automation = SRC.slice(SRC.indexOf('} else if (call.type === "automation") {'));
  assert.match(automation.slice(0, 900), /_callDrivesDesktop\(call\) && \(await _showControlGlow\(\)|_callDrivesDesktop\(call\)\) _showControlGlow\(\)/,
    "automation 分支必须用统一判据，不能再用写死的正则");

  const system = SRC.slice(SRC.indexOf('} else if (call.type === "system") {'));
  assert.match(system.slice(0, 900), /_callDrivesDesktop\(call\)\) _showControlGlow\(\)/,
    "system 分支（open/focus/menu 会真的搬窗口点菜单）必须亮红光");

  const uiclick = SRC.slice(SRC.indexOf('} else if (call.type === "uiclick") {'));
  assert.match(uiclick.slice(0, 1600), /_showControlGlow\(\)/,
    "ui_click 通过辅助功能真的操作前台 App，必须亮红光");
});

test("红光在运行收尾时一定会灭", () => {
  assert.match(SRC, /_hideControlGlow\(\); \/\/ 灭掉红光/,
    "运行结束的 finally 里必须熄灯——灭不掉的红光比不亮更糟");
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
  assert.match(SRC, /async function _desktopPermissionNote\(\)/, "必须有统一的诊断取用点");
  assert.match(SRC, /backend\.invoke\("permission_status"\)/, "诊断要问系统 API，不能猜");
  const note = extractFn("_desktopPermissionNote");
  assert.match(note, /原样转述/, "要明确要求模型别把诊断改写成「请去打开开关」");

  for (const [what, marker] of [
    ["automation", 'return { type: "automation", path: _m, content: `[失败]'],
    ["system", "[系统控制失败]"],
    ["uiclick", '[失败] ui_click: ${message}'],
  ]) {
    const at = SRC.indexOf(marker);
    assert.ok(at > 0, `找不到 ${what} 的失败返回`);
    assert.match(SRC.slice(at - 400, at + 300), /_desktopPermissionNote\(\)/,
      `${what} 失败时必须附上权限诊断`);
  }
});

// 解释某段代码为什么被删的注释，往往会把被删的原文照抄一遍——那会让
// doesNotMatch 永远通不过。断言"这段文案不该再出现"之前必须先剥掉注释。
function stripJsComments(source) {
  return String(source)
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/(^|[^:])\/\/[^\n]*/g, "$1");
}

test("system 失败不再无条件叫用户去勾一个已经勾着的开关", () => {
  // 先切出 system 失败那一小段再剥注释。整份 main.js 不能直接剥——里面的正则字面量
  // 含 /* 序列，块注释规则会从那里一路吃掉几千行，断言就成了空对空。
  const at = SRC.indexOf("[系统控制失败]");
  assert.ok(at > 0, "找不到 system 失败返回");
  const region = stripJsComments(SRC.slice(at - 900, at + 400));
  assert.doesNotMatch(region, /勾选 Mr\. Day One 后重启/,
    "这句话在授权已失效时是错的：开关本来就勾着，重勾无效，只会把用户引向死路");
  assert.match(region, /_desktopPermissionNote\(\)/, "取而代之的必须是真实诊断");
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
// 顶层函数按"行首单独一个 }"收尾来切。上面那个括号匹配器不认正则字面量，
// 而 _dialogBodyHtml 里有 /[&<>"]/g —— 里面那个引号会被当成字符串开头，一路跑飞。
function extractTopLevelFn(name) {
  const i = SRC.indexOf(`function ${name}(`);
  assert.ok(i >= 0, `找不到 ${name}`);
  const end = SRC.indexOf("\n}\n", i);
  assert.ok(end > i, `${name} 没有行首收尾的大括号`);
  return SRC.slice(i, end + 2);
}
const dialogHtml = new Function(`${extractTopLevelFn("_dialogBodyHtml")}\n;return _dialogBodyHtml;`)();

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
  const dlg = extractTopLevelFn("_confirmDialog");
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
