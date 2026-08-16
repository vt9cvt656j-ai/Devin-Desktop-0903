// 「自动化用哪个浏览器」这个选择本身。
//
// 之前是写死的：候选表按固定优先级取第一个装了的，装了 Chrome 就只能用 Chrome。
// 想把自己的 Chrome 留给自己、让自动化去用 Edge，没有任何开关能做到。
//
// 这里守住的三条，每一条都对应一个「两边都绿、功能却是死的」的失败模式：
//   1. 常量的声明必须排在**立刻执行**的那段代码之前（否则应用一启动就白屏，而测试全绿）
//   2. 读状态和写状态必须是两个命令（用 setter 去读会把用户刚选的清掉）
//   3. 选择必须落盘并在启动时回灌（后端那份是进程内状态，重启就没了）
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { test } from "node:test";
import assert from "node:assert/strict";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
const RUST = readFileSync(join(HERE, "..", "src-tauri", "src", "browser.rs"), "utf8");
const CAPTURE = readFileSync(join(HERE, "..", "src-tauri", "src", "capture.rs"), "utf8");
const LIB = readFileSync(join(HERE, "..", "src-tauri", "src", "lib.rs"), "utf8");
const SHELL = readFileSync(join(HERE, "..", "src", "app", "Shell.jsx"), "utf8");
const I18N = readFileSync(join(HERE, "..", "src", "i18n.js"), "utf8");
const AUTOMATION = readFileSync(
  join(HERE, "..", "automation-framework", "src", "browser.rs"), "utf8");

test("图标常量必须声明在那段立刻执行的块之前，否则应用一启动就白屏", () => {
  // 能力菜单那段是块语句，模块求值时就跑。它读的常量如果声明在文件后面，
  // 就是 TDZ 报错——而且所有静态测试照样全绿，因为没人真的求值过 main.js。
  const decl = SRC.indexOf("const _ICON_BROWSER =");
  const use = SRC.indexOf("icon.innerHTML = _ICON_BROWSER");
  assert.ok(decl >= 0, "_ICON_BROWSER 没了");
  assert.ok(use >= 0, "菜单项不再设置图标");
  assert.ok(decl < use, "_ICON_BROWSER 声明排在使用之后 —— 这是 TDZ，应用起不来");
});

test("读状态和写状态是两个命令，不能拿 setter 去读", () => {
  // setter 传空 = 「自动选」。拿它来读当前状态，会把用户刚选的浏览器清掉，
  // 而且只在「打开面板看一眼再关掉」时才出现。
  assert.match(RUST, /pub fn browser_get_preference\(\)/);
  assert.match(RUST, /pub fn browser_set_preference\(/);
  assert.match(SRC, /backend\.invoke\("browser_get_preference"\)/,
    "面板读状态必须用 getter");
  // 两个命令都要在 lib.rs 注册，否则前端调用直接报「命令不存在」。
  for (const cmd of ["browser_set_preference", "browser_get_preference"]) {
    assert.ok(LIB.includes(`browser::${cmd}`), `${cmd} 没在 lib.rs 注册`);
  }
});

test("选择要落盘并在启动时回灌，不然重启一次就变回去了", () => {
  // 后端的偏好是进程内 static，不落盘。只写后端的话，用户选了 Edge、
  // 重启一次又悄悄回到 Chrome —— 一个只在下次启动才出现的 bug。
  assert.match(SRC, /_BROWSER_PREF_KEY = "michael-ide\.browser\.pref\.v1"/);
  assert.match(SRC, /localStorage\.setItem\(_BROWSER_PREF_KEY/);
  assert.match(SRC, /_restoreBrowserPref\(\)/, "启动时必须回灌");
});

test("菜单项三种语言都要有，否则英文界面会冒出中文", () => {
  assert.match(SHELL, /id="capabilityBrowserItem"/);
  assert.match(SHELL, /data-i18n="assistant\.capability\.browser"/);
  const labels = [...I18N.matchAll(/"assistant\.capability\.browser": "([^"]*)"/g)].map((m) => m[1]);
  assert.equal(labels.length, 3, `三张语言表各要一条，实际 ${labels.length} 条`);
  assert.equal(new Set(labels).size, 3, "三种语言的文案不该完全相同——多半是漏译");
});

test("浏览器目录只有一处，前后端不各写一份", () => {
  // 「有哪些浏览器」写成两份就必然漂移。Rust 侧由 capture.rs 的目录派生，
  // 前端不许再硬编码一份名单，它只渲染后端返回的 installed。
  assert.match(CAPTURE, /pub const BROWSER_KINDS: &\[BrowserKind\]/);
  assert.match(SRC, /state\.installed/, "前端必须渲染后端返回的名单");
  assert.doesNotMatch(SRC, /\[\s*"chrome"\s*,\s*"edge"\s*,\s*"brave"/,
    "前端又硬编码了一份浏览器名单");
});

test("automation-framework 不再写死 Chrome 的绝对路径", () => {
  // 这条链路（automation 工具的 browser.*）以前把 /Applications/Google Chrome.app
  // 的绝对路径写死了：没装 Chrome 的机器上整条链路直接起不来，
  // 而同一台机器上 IDE 自己的 browser 工具跑得好好的，因为那边会探测。
  assert.doesNotMatch(AUTOMATION, /chrome_executable\("\/Applications/,
    "又把 Chrome 的绝对路径写回去了");
  assert.match(AUTOMATION, /fn find_browser\(\)/);
  // Linux/Windows 上那个 macOS 路径永远不存在，所以三个平台都要有候选表。
  for (const os of ["macos", "windows"]) {
    assert.ok(AUTOMATION.includes(`target_os = "${os}"`), `${os} 没有候选表`);
  }
});

test("浏览器弹窗用托盘式选择列表，且颜色不依赖面板令牌", () => {
  const css = readFileSync(join(HERE, "../src/styles/app.css"), "utf8");
  const rule = (sel) => {
    const at = css.indexOf("\n" + sel + " {");
    assert.notEqual(at, -1, `${sel} 规则没了`);
    return css.slice(at, css.indexOf("\n}", at));
  };

  // 这里只有两三个选项，是一次**选择**，不是一屏几十项的设置表。M3 对这种场景给的是
  // 托盘式列表：项与项之间用留白而不是分隔线，选中项整块铺一层淡色容器。
  // 上一版照搬了设置页那套"一张卡 + 发丝线"，密排语汇用在三个选项上显得又挤又闷。
  assert.doesNotMatch(css, /\.bp-row \+ \.bp-row\s*\{[^}]*border-top/,
    "又给选项之间加分隔线了——三个选项的选择列表不该用密排表格的语汇");
  assert.match(rule(".bp-list"), /gap:\s*\dpx/, "项与项之间没有留白");
  assert.match(rule(".bp-row"), /border-radius:\s*14px/, "选项自己不是圆角容器，选中态铺不出来");
  assert.match(css, /\.bp-row\.is-on\s*\{[^}]*background:\s*#e8f0fe/,
    "选中态不是 tonal 容器");
  // 左边的首字母图块：认牌子比读一行路径快得多。
  assert.match(rule(".bp-mono"), /border-radius:/, "首字母图块没了");
  assert.match(SRC, /<span class="bp-mono"/, "没渲染首字母图块");

  // 单选圈是自绘的：原生 radio 在这套界面里是唯一一处系统件。
  assert.match(css, /\.bp-row input\[type="radio"\]\s*\{[^}]*opacity:\s*0/, "原生 radio 没藏起来");
  assert.match(css, /\.bp-row\.is-on \.bp-radio::after/, "选中没有自绘的实心点");

  // 深色必须单独覆盖：选中态和图块用的是写死的浅色，深色下不覆盖会白底黑字压深底。
  // 带上花括号精确匹配：光按前缀找会被 `.is-on:hover`、`.is-on .bp-mono` 这些同前缀的
  // 规则喂饱——把真正那条删掉也照样通过。
  for (const need of [".bp-row.is-on", ".bp-mono", ".bp-warn"]) {
    assert.ok(css.includes(`:root[data-theme="dark"] ${need} {`), `${need} 缺深色覆盖`);
  }

  // 这个弹窗**不在** .feature-panel 里，取不到 --feature-* 令牌；用了又不写兜底的话，
  // 整条声明会作废（毛玻璃那次就是这么栽的）。
  for (const sel of [".bp-lede", ".bp-sec", ".bp-list", ".bp-row", ".bp-ext"]) {
    assert.doesNotMatch(rule(sel), /var\(--feature-/,
      `${sel} 用了面板作用域的令牌，在这个弹窗里取不到`);
  }
  assert.match(rule(".bp-ext"), /var\(--text,\s*#/, "颜色没写兜底值");
});

