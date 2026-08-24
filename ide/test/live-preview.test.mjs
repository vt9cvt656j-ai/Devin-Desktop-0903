// 实时预览页签：编辑器标签栏里的浏览器窗格。
//
// 这套用例守的是四类会静默失效的东西：
//   · 地址栏是「用户输入直达 iframe src」的通道，协议闸一旦漏了就是本应用源里的任意执行；
//   · 页签的键要经过 _normalizeFsPath，带双斜杠的伪路径会被折成另一个字符串；
//   · 预览页签背后没有文件，每一道「按文件类型分岔」的门都得放它过去，漏一道就是一个
//     看不出原因的崩溃或空白；
//   · 轮询只在窗格可见时该跑——聊天里那张老卡片就是在这儿漏的。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load, loadConst, fnSource, CODE } from "./helpers/source.mjs";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));

const normalize = load("_previewNormalizeUrl");
const isLocal = load("_previewIsLocalUrl");

test("地址栏认得人真会敲的几种写法", () => {
  assert.equal(normalize("3000"), "http://localhost:3000");
  assert.equal(normalize("8787"), "http://localhost:8787");
  // 端口号带路径。这条不是凑数：`5174/x.html` 不匹配任何本机地址的形状，会掉进
  // 「补 https://」那条分支，而 URL 解析器把纯数字主机名按 IPv4 整数解释——
  // 结果是 https://0.0.20.54/x.html。不报错、不空白，只是打开一个不相干的地址。
  assert.equal(normalize("5174/__icon-check.html"), "http://localhost:5174/__icon-check.html");
  assert.equal(normalize("3000/about?x=1"), "http://localhost:3000/about?x=1");
  assert.ok(!normalize("5174/x.html").includes("0.0.20.54"));
  // 0.0.0.0 是绑定地址不是目标地址：Windows 上连它无效。规范化时统一改写成 127.0.0.1，
  // 这样终端刮取 / 地址栏手敲 / agent 传入三条路一次覆盖。
  assert.equal(normalize("http://0.0.0.0:8000/x"), "http://127.0.0.1:8000/x");
  assert.equal(normalize("0.0.0.0:5173"), "http://127.0.0.1:5173/");
  assert.equal(normalize("http://localhost:5173"), "http://localhost:5173/");
  assert.equal(normalize("localhost:5173"), "http://localhost:5173/");
  assert.equal(normalize("127.0.0.1:8080"), "http://127.0.0.1:8080/");
  // 光一条路径 = 接在当前站点上
  assert.equal(normalize("/about", "http://localhost:3000/x/y"), "http://localhost:3000/about");
  // 省略协议的公网域名走 https，不是 http
  assert.equal(normalize("example.com"), "https://example.com/");
  assert.equal(normalize("https://example.com/a"), "https://example.com/a");
});

test("空输入和纯垃圾返回空串，而不是抛异常", () => {
  assert.equal(normalize(""), "");
  assert.equal(normalize("   "), "");
  assert.equal(normalize(null), "");
  assert.equal(normalize("://"), "");
});

test("只放行 http/https —— 这是地址栏那条注入路径的闸", () => {
  // javascript: 塞进新建 iframe 的 src 会在 about:blank 文档里执行，而那个文档
  // 继承的是本应用的源。这不是「预览一个奇怪的页面」，是在 IDE 自己的源里执行脚本。
  assert.equal(normalize("javascript:alert(1)"), "");
  assert.equal(normalize("JavaScript:alert(1)"), "");
  assert.equal(normalize("data:text/html,hi"), "");
  assert.equal(normalize("vbscript:msgbox(1)"), "");
  assert.equal(normalize("file:///etc/passwd"), "");
  assert.equal(normalize("blob:http://x/y"), "");
});

test("本地判定认全部回环写法", () => {
  for (const u of ["http://localhost:3000", "http://127.0.0.1:8080/x", "http://0.0.0.0:5173", "http://[::1]:9000"]) {
    assert.equal(isLocal(u), true, u);
  }
  for (const u of ["https://example.com", "http://192.168.1.9:3000", "not a url"]) {
    assert.equal(isLocal(u), false, u);
  }
});

test("页签的键在 _normalizeFsPath 下是不动点", () => {
  // 这条真踩过：`mrdayone://live-preview` 经过路径规范化会变成 `mrdayone:/live-preview`
  // （连续斜杠被折掉），于是 openFiles 的键和常量对不上，closeFile / activate 全部
  // 静默失配——页签关不掉、切过去是空白，而且哪里都不报错。
  const norm = load("_normalizeFsPath", ["_normalizeFsPath", "_toPosix"]);
  const path = loadConst("PREVIEW_TAB_PATH");
  assert.equal(norm(path), path, path + " 经过路径规范化变成了 " + norm(path));
  assert.equal(norm("mrdayone://live-preview"), "mrdayone:/live-preview"); // 反面：这就是当初那个坑
});

test("每一道按文件类型分岔的门都放预览页签过去", () => {
  // 预览页签没有 model、没有磁盘内容。漏掉任何一道都不会报错，只会表现成
  // 别的症状：跑到 Monaco 那条路会拿 null model 崩，跑到保存那条路会试着往
  // 一个伪路径写盘，跑到 LSP 那条路会给语言服务器发一个不存在的文件。
  const gates = [
    ["closeFile 关前保存", /!discardBuffer && f\.dirty && f\.model && [^\n]*!f\.isPreview/],
    ["closeFile 的 didClose", /if \(!f\.isInspection && !f\.isPreview\) lspManager\?\.didClose/],
    ["activate 的 runBtn", /runBtn\.disabled = !!\(f\.isImage[^)]*\|\| f\.isPreview\)/],
    ["activate 的装饰绘制", /!f\.isImage && !f\.isVideo && !f\.isPdf && !f\.isInspection && !f\.isPreview/],
  ];
  for (const [what, re] of gates) assert.match(CODE, re, what + " 这道门没放预览页签过去");
  // 分屏那两处是同一行文本，数量对上即可
  const splitGate = /if \(!f \|\| f\.isImage \|\| f\.isVideo \|\| f\.isPdf \|\| f\.isInspection \|\| f\.isPreview\) return;/g;
  assert.equal((CODE.match(splitGate) || []).length, 2, "分屏编辑器有两处类型门，预览页签没有全部被挡住");
});

test("activate 里预览分支排在最前，且切走时收起窗格", () => {
  const src = fnSource("activate", { code: true });
  assert.match(src, /hideLivePreviewPane\(\);/, "切走时没有收起预览窗格——轮询会在后台一直跑");
  // 两个锚点都要先确认存在。只写 indexOf(a) < indexOf(b) 的话，a 被整个删掉时
  // indexOf 返回 -1，恒小于——断言变成永远为真。变异测试就是在这儿抓到我的。
  const atPreview = src.indexOf("if (f.isPreview)");
  const atImage = src.indexOf("f.isImage) {");
  assert.ok(atPreview >= 0, "activate 里根本没有 isPreview 分支");
  assert.ok(atImage >= 0, "activate 里没有图片分支了——这条用例的参照物没了，需要重写");
  assert.ok(atPreview < atImage,
    "isPreview 分支必须在图片分支之前：预览条目没有 isImage 之类的字段，落到后面的分支会走进文本编辑器那条路");
});

test("iframe 不给 allow-top-navigation", () => {
  // 给了的话，预览页面一句 top.location=… 就能把整个 IDE 导航走。在桌面壳里
  // 这等于把应用窗口变成那个网站，而且回不来（没有地址栏可以敲回去）。
  const src = fnSource("_previewRenderStage", { code: true });
  const m = src.match(/setAttribute\("sandbox",\s*"([^"]+)"\)/);
  assert.ok(m, "iframe 没有设 sandbox");
  const tokens = m[1].split(/\s+/);
  assert.ok(!tokens.includes("allow-top-navigation"), "sandbox 里出现了 allow-top-navigation");
  assert.ok(!tokens.includes("allow-top-navigation-by-user-activation"), "sandbox 里出现了 allow-top-navigation-by-user-activation");
  // 真实应用要跑起来这几项是必须的，一并钉住，免得后来有人「收紧安全」把预览改哑
  for (const need of ["allow-scripts", "allow-same-origin", "allow-forms"]) {
    assert.ok(tokens.includes(need), "sandbox 少了 " + need + "，大部分真实页面会跑不起来");
  }
});

test("dev server 候选来自执行事实：IDE 自己的端口 + 终端里真打印过的地址", () => {
  const viteLine = "\u001b[36m  Local:   http://localhost:5173/\u001b[0m\n  ready in 320 ms\n";
  const detect = load("_previewDetectDevUrls", {
    _devServerPort: 8787,
    _devServerRunning: true,
    _stripAnsi: (s) => String(s).replace(/\u001b\[[0-9;]*m/g, ""),
    termTabs: [
      { title: "dev", recentOut: viteLine },
      { title: "api", recentOut: "控制台  http://127.0.0.1:8080\n" },
      { title: "空", recentOut: "" },
    ],
  });
  const urls = detect().map((c) => c.url);
  assert.ok(urls.includes("http://localhost:8787"), "IDE 自己起的预览服务没被列出来");
  assert.ok(urls.includes("http://localhost:5173/"), "终端里 vite 打印的地址没被认出来：" + urls.join(","));
  assert.ok(urls.includes("http://127.0.0.1:8080"), "终端里的 127.0.0.1 地址没被认出来");
  // 来源要说得出，用户才知道这个候选是哪来的
  assert.ok(detect().every((c) => c.from && c.from.length), "候选没有标注来源");
});

test("候选去掉终端画框粘上的标点", () => {
  const detect = load("_previewDetectDevUrls", {
    _devServerPort: null, _devServerRunning: false,
    _stripAnsi: (s) => s,
    // 关键是**带路径**的那两条：路径部分的字符类会把结尾的句号/逗号一起吃进来，
    // 而裸主机名那两条根本轮不到剥标点（端口正则本来就停在数字上）。第一版用例
    // 只举了裸主机名，把剥标点那行删掉照样绿——变异测试当场抓到。
    termTabs: [{ title: "t", recentOut: [
      "Server: http://localhost:3000.",
      "见 http://localhost:4000)",
      "打开 http://localhost:5173/dashboard.",
      "或者 http://127.0.0.1:8080/a/b，然后刷新",
    ].join("\n") }],
  });
  const urls = detect().map((c) => c.url);
  assert.ok(urls.includes("http://localhost:3000"), urls.join(","));
  assert.ok(urls.includes("http://localhost:4000"), urls.join(","));
  assert.ok(urls.includes("http://localhost:5173/dashboard"), "带路径的地址末尾句号没剥掉：" + urls.join(","));
  assert.ok(urls.includes("http://127.0.0.1:8080/a/b"), "带路径的地址末尾中文逗号没剥掉：" + urls.join(","));
  assert.ok(!urls.some((u) => /[.)，]$/.test(u)), "地址末尾粘着标点：" + urls.join(","));
});

test("终端读不到时不抛，已经拿到的候选照样交出去", () => {
  // 真实形状是「终端子系统还没起来 / 正在重建」，遍历当场炸。这时候不该整个函数
  // 报废：IDE 自己那个端口是另一个来源，已经确定了的东西要照样给出去。
  const detect = load("_previewDetectDevUrls", {
    _devServerPort: 8787, _devServerRunning: true,
    _stripAnsi: (s) => s,
    termTabs: { [Symbol.iterator]() { throw new Error("终端还没初始化"); } },
  });
  assert.deepEqual(detect().map((c) => c.url), ["http://localhost:8787"]);
});

test("轮询只在窗格可见且 CDP 引擎时开，其余情况一律停", () => {
  const timers = new Map();
  let seq = 0;
  const state = { el: { hidden: false }, engine: "cdp", url: "http://x", dockOpen: false, frameTimer: 0, logTimer: 0 };
  const sync = load("_previewSyncTimers", {
    _preview: state,
    activePath: "mrdayone:live-preview",
    PREVIEW_TAB_PATH: "mrdayone:live-preview",
    PREVIEW_CDP_FRAME_MS: 1200,
    PREVIEW_LOG_POLL_MS: 2000,
    _previewPumpFrame: () => {},
    _previewPumpLogs: () => {},
    setInterval: () => { const id = ++seq; timers.set(id, true); return id; },
    clearInterval: (id) => { timers.delete(id); },
  });

  sync();
  assert.ok(state.frameTimer, "CDP + 可见时没有开取帧轮询");
  assert.equal(state.logTimer, 0, "控制台面板没打开却在拉日志");

  state.dockOpen = true; sync();
  assert.ok(state.logTimer, "控制台面板打开后没有开日志轮询");

  // 切走 = 窗格不可见：两个轮询都必须停。这正是老卡片漏掉的那一步，
  // 结果是画面没人看，浏览器和 IPC 还在被占着。
  state.el.hidden = true; sync();
  assert.equal(state.frameTimer, 0, "窗格隐藏后取帧轮询还在跑");
  assert.equal(state.logTimer, 0, "窗格隐藏后日志轮询还在跑");
  assert.equal(timers.size, 0, "有定时器没被 clearInterval 收掉");

  // iframe 引擎本身就是活的，不需要任何轮询
  state.el.hidden = false; state.engine = "live"; sync();
  assert.equal(state.frameTimer, 0, "iframe 引擎下还开着取帧轮询");
  assert.equal(state.logTimer, 0, "iframe 引擎下还开着日志轮询");
});

test("设备预设缩放：放得下就不缩，放不下按短边等比缩", () => {
  const DEVICES = loadConst("PREVIEW_DEVICES");
  // 造一个最小可用的 DOM 替身：只需要这个函数真正读写的那几个属性
  const run = (device, w, h) => {
    const target = { style: {} };
    const stage = {
      clientWidth: w, clientHeight: h,
      classList: { _s: new Set(), add(c) { this._s.add(c); }, remove(c) { this._s.delete(c); }, has(c) { return this._s.has(c); } },
      querySelector: (sel) => (sel === "iframe" ? target : null),
    };
    const state = { device, el: { querySelector: () => stage } };
    load("_previewApplyDevice", { _preview: state, PREVIEW_DEVICES: DEVICES })();
    return { target, stage };
  };

  // 窗格很大：手机 390x844 原样放下，不缩放
  let r = run("phone", 1400, 1000);
  assert.equal(r.target.style.width, "390px");
  assert.equal(r.target.style.transform, "scale(1)");
  assert.ok(r.stage.classList.has("is-deviced"));

  // 窗格矮：按高度缩。844 高要塞进 (400-32)=368，比例 368/844
  r = run("phone", 1400, 400);
  const k = 368 / 844;
  assert.equal(r.target.style.transform, "scale(" + k + ")");
  // transform 不改变布局占位，缩小后底部会空出一大块，必须用负外边距扣回来
  assert.equal(r.target.style.marginBottom, Math.round(-844 * (1 - k)) + "px");

  // 自适应：不设尺寸、不缩放、不加设备样式
  r = run("auto", 1400, 1000);
  assert.equal(r.target.style.width, "");
  assert.equal(r.target.style.transform, "");
  assert.ok(!r.stage.classList.has("is-deviced"));
});

test("CDP 点击发的是完整指针事件序列，不是 element.click()", () => {
  // 只发一个 click 的话，React 合成事件、Radix/shadcn 那类监听 pointerdown 的组件
  // 全都没反应——表现是「点了没用」，而且没有任何报错。
  const js = load("_PREVIEW_CLICK_JS")(120, 240);
  assert.match(js, /elementFromPoint\(120, 240\)/);
  for (const type of ["pointerdown", "mousedown", "pointerup", "mouseup", "click"]) {
    assert.ok(js.includes("'" + type + "'"), "点击序列里少了 " + type);
  }
  assert.match(js, /clientX: 120/);
  assert.match(js, /clientY: 240/);
});

test("聊天里不再产出旧的内嵌预览卡", () => {
  // 预览搬进标签栏之后，消息体里只留一行指路条。旧的 .mi-live-preview 结构
  // 不该再被任何代码创建（样式还留着，因为历史会话的 HTML 快照里有它）。
  assert.ok(!CODE.includes("mi-live-preview"), "main.js 里还在创建旧的内嵌预览卡");
  const src = fnSource("_ensureLiveBrowserPreview", { code: true });
  assert.match(src, /openLivePreview\(/, "浏览器工具没有把预览页签接管到当前地址");
  assert.match(src, /mi-preview-jump/, "对话里没有留下指向页签的入口");
});

test("agent 后台导航不把正在编辑的人拽走", () => {
  const src = fnSource("_ensureLiveBrowserPreview", { code: true });
  // 判据必须是「页签刚建 或 用户本来就停在预览上」，不能无条件 focus
  assert.match(src, /focus: firstTime \|\| wasActive/,
    "agent 每导航一次就把编辑器切到预览页——正在改代码的人会被反复打断");
  assert.match(src, /activePath === PREVIEW_TAB_PATH/);
});

test("内置 dev server 注入的调试桥是合法 JS，且只在被嵌时说话", () => {
  // iframe 跨源读不到页面的 console，只有页面自己送出来这一条路。这段脚本躺在
  // 一个 Python bytes 字面量里、又躺在 JS 模板串里，两层引号任何一层写错都只会
  // 表现成「预览的控制台永远是空的」，不会有人报错。
  const src = fnSource("_startDevServer", { code: true });
  const m = src.match(/BRIDGE_JS = b'<script>([\s\S]*?)<\/script>'/);
  assert.ok(m, "内置 dev server 没有注入调试桥");
  const js = m[1];
  assert.doesNotThrow(() => new Function(js), "注入的调试桥不是合法 JS");
  assert.ok(js.includes("window.parent===window)return"), "没做「不在 iframe 里就一个字都不发」的判断");
  assert.ok(js.includes("__mrdayone:\"preview-log\""), "消息没有带识别标记，接收端认不出来");
  assert.ok(!js.includes("'"), "这段要嵌进 Python 的单引号字面量里，不能出现单引号");
  // 注入点也要真的接上——常量定义了但没拼进响应，等于没写
  assert.match(src, /RELOAD_JS \+ BRIDGE_JS/, "调试桥定义了却没被注入到页面里");
});

test("预览页签的伪路径不会被当成「当前文件」交出去", () => {
  // 这个坑只在**没打开文件夹**时露头：有根目录时那几处的前缀判断顺手挡住了它，
  // 所以它是那种"偶尔才复现、看起来像模型犯傻"的形状——模型收到一个
  // mrdayone:live-preview，然后真的去 read_file 它。
  const realPath = load("_realFilePath", { PREVIEW_TAB_PATH: loadConst("PREVIEW_TAB_PATH") });
  assert.equal(realPath(loadConst("PREVIEW_TAB_PATH")), "");
  assert.equal(realPath("/repo/src/main.js"), "/repo/src/main.js");
  assert.equal(realPath(""), "");
  assert.equal(realPath(null), "");

  // 四个调用点都要真的走这道闸，漏一个就等于没修
  const chips = fnSource("_dynamicChatChips", { code: true });
  assert.match(chips, /const path = _realFilePath\(activePath\);/,
    "对话起手提示还在直接用 activePath——会渲染出「解释 mrdayone:live-preview」");
  const intent = (CODE.match(/activePath: _realFilePath\(activePath\) &&/g) || []).length;
  assert.equal(intent, 3,
    "意图分类的上下文有三处、必须逐字段一致（不一致会导致指纹对不上、预取白跑），现在只有 " + intent + " 处走了闸");
});

test("重新加载不依赖 requestAnimationFrame", () => {
  // 第一版是「先置 about:blank，再用一次 rAF 把地址切回去」。实测 rAF 在窗口被遮挡、
  // 标签页在后台或系统省电时会被节流甚至根本不触发——预览就永久停在空白页，
  // 而地址栏里地址还在，看起来像是页面自己挂了。
  const src = fnSource("_previewReload", { code: true });
  assert.ok(!src.includes("requestAnimationFrame"), "重新加载又回到了靠 rAF 恢复地址的写法");
  assert.ok(!src.includes("about:blank"), "还在用 about:blank 中转，一旦恢复那一步没跑就是白屏");
  assert.match(src, /stage\.querySelector\("iframe"\)\?\.remove\(\);/, "没有换掉 iframe 元素，src 不变就不会真的重新请求");
  assert.match(src, /_previewRenderStage\(\)/);
});

test("地址栏敲同一个地址回车会重新加载", () => {
  // 不特判的话 _previewNavigate 发现地址没变就什么都不做，按了回车毫无反应——
  // 而这恰恰是最常用的一个动作（改完代码回车刷一下）。
  const src = fnSource("_previewWirePane", { code: true });
  assert.match(src, /if \(next === _preview\.url\) \{ _previewReload\(\); return; \}/);
});

test("调试桥的 message 监听器只挂一次，不跟着窗格重建", () => {
  // 它必须挂在 window 上（消息是从 iframe post 上来的）。挂在「建窗格」里的话，
  // 每次关掉预览页签再打开都会再挂一个，而它们读同一份 _preview 状态——
  // 控制台里每条日志按打开次数翻倍。开三次就是三份，而且没有任何报错。
  const wire = fnSource("_previewWirePane", { code: true });
  assert.ok(!/window\.addEventListener\("message"/.test(wire),
    "message 监听器又回到了「每建一次窗格挂一个」的写法");
  // 模块级只有一处
  const n = (CODE.match(/window\.addEventListener\("message", \(ev\) => \{/g) || []).length;
  assert.equal(n, 1, "预览的 message 监听器出现了 " + n + " 处");
  // 关页签走的是"卸掉页面、留下壳"，不是销毁重建
  assert.match(CODE, /if \(f\.isPreview\) _previewTeardown\(\);/);
  const down = fnSource("_previewTeardown", { code: true });
  assert.ok(!down.includes("_preview.el = null"), "又开始销毁窗格了，监听器会重新开始叠加");
  assert.match(down, /stage\.textContent = "";/,
    "被预览的页面没卸掉——它的定时器/轮询/WebSocket 会在一个看不见的 iframe 里继续跑");
  assert.match(down, /_previewStopTimers\(\);/);
});

test("只收当前预览源发来的桥消息", () => {
  // 被预览的页面里可能还嵌着第三方 iframe，它们也能 postMessage 到 IDE 这一层。
  const src = CODE.slice(CODE.indexOf('window.addEventListener("message", (ev) => {'));
  assert.match(src.slice(0, 900), /new URL\(_preview\.url\)\.origin !== ev\.origin/,
    "没有校验消息来源，任何嵌套 iframe 都能往控制台面板里写东西");
  assert.match(src.slice(0, 900), /d\.__mrdayone !== "preview-log"/);
});

test("桌面壳的 CSP 必须放行实时预览真正要嵌的来源", () => {
  // 这条守的是一整类 bug：开发时跑 vite dev server，**那里没有任何 CSP**，所以
  // 功能在浏览器里怎么试都是好的；打包成桌面应用之后 CSP 才生效，被它挡住的东西
  // 不报错、不提示，只是静默什么都不发生。
  //
  // 实测踩过：frame-src 里没有 localhost，于是实时预览在桌面端永远白屏，
  // dev server 那边一条请求日志都收不到——从界面上完全看不出是谁拦的。
  const conf = JSON.parse(readFileSync(join(HERE, "../src-tauri/tauri.conf.json"), "utf8"));
  const csp = String(conf?.app?.security?.csp || "");
  assert.ok(csp.length > 40, "读不到 CSP —— 这条用例等于没跑");
  const directive = (name) => {
    const hit = csp.split(";").map((s) => s.trim()).filter((s) => s.startsWith(name + " "));
    assert.equal(hit.length, 1, `CSP 里 ${name} 出现了 ${hit.length} 次`);
    return hit[0].split(/\s+/).slice(1);
  };

  // 实时引擎用 iframe 直嵌本机 dev server。三种回环写法都要放行——
  // vite 默认打印 localhost，python 的 http.server 打印 127.0.0.1，IPv6 环境是 [::1]。
  const frame = directive("frame-src");
  // 判据不是「凭感觉列几个」，而是 **_previewIsLocalUrl 认哪些主机名，CSP 就必须放行哪些**。
  // 漏掉的那个会变成最恶劣的一种：IDE 自己从终端里扫出候选、推荐给用户点，点了必然白屏。
  // 0.0.0.0 尤其要有——`vite --host` / `python manage.py runserver 0.0.0.0:8000` /
  // `rails s -b 0.0.0.0` / docker-compose 打印的都是它。
  // HOSTS 不是手写的：直接从 _previewIsLocalUrl 的源码里把那张主机名表抠出来。
  // 手写白名单的话，以后有人往本机判定里加一个主机名（比如 host.docker.internal），
  // 这条用例照样绿，而 CSP 没跟上——那正是这个 bug 的原始形态。
  const localSrc = fnSource("_previewIsLocalUrl", { code: true });
  const alt = localSrc.match(/\/\^\(([^)]+)\)\$\/i/);
  assert.ok(alt, "抠不出 _previewIsLocalUrl 的主机名表——这条用例的判据坏了，等于没跑");
  const HOSTS = alt[1].split("|").map((h) => h.replace(/\\/g, ""))
    .filter((h) => h !== "::1");   // 裸 ::1 不是合法的 URL 主机写法，CSP 里对应的是 [::1]
  assert.ok(HOSTS.length >= 4, `只抠出 ${HOSTS.length} 个主机名，判据可疑`);
  const isLocalSrc = load("_previewIsLocalUrl");
  for (const h of HOSTS) {
    assert.equal(isLocalSrc(`http://${h}:8000`), true, `_previewIsLocalUrl 不再认 ${h}，这条用例的判据需要重写`);
    assert.ok(frame.includes(`http://${h}:*`),
      `frame-src 少了 http://${h}:* —— _previewIsLocalUrl 把它当本机、会塞进 iframe，CSP 却挡住，结果是白屏且不报错`);
  }
  // 但**不许**放行整个 https:——给桌面应用开放"任意站点都能嵌"是实打实的攻击面，
  // 外部站点走 CDP 那条引擎（见 _previewNavigate 里的自动切换）。
  assert.ok(!frame.includes("https:"), "frame-src 放开了整个 https: —— 外部站点该走「浏览器」引擎，不该嵌进应用窗口");

  // 失败信号要能从应用这一层探到本机服务，否则"连不上"和"不让被嵌"分不开。
  // 探针要能到达 iframe 能到达的每一个来源，否则「连不上」这个结论对某些主机名恒为真——
  // 那比白屏更坏：界面会理直气壮地叫用户去查一个根本没问题的 dev server。
  const connect = directive("connect-src");
  for (const h of HOSTS) {
    assert.ok(connect.includes(`http://${h}:*`),
      `connect-src 少了 http://${h}:* —— 探针被挡住，预览会对着一个活着的服务说「连不上」`);
  }
});

test("实时引擎遇到外部地址要自己切走，而不是白屏", () => {
  const src = fnSource("_previewNavigate", { code: true });
  assert.match(src, /_preview\.engine === "live" && !_previewIsLocalUrl\(next\)/,
    "没有判断「实时引擎 + 外部地址」——CSP 会把它拦成一片白，用户看不出原因");
  assert.match(src, /_previewSetEngine\("cdp"\)/);
});

test("iframe 加载失败要说得出是哪一种失败", () => {
  // 三种原因在页面这侧长得一模一样（都是一片白）：服务没跑 / 被 CSP 拦（请求都没发出去）/
  // 页面不让被嵌。分不开的话，用户只能看到"预览坏了"。
  const src = fnSource("_previewWatchLiveLoad", { code: true });
  assert.match(src, /fetch\(url, \{ mode: "no-cors"/, "没有从应用这层探服务在不在");
  assert.match(src, /addEventListener\("load"/, "没有等 iframe 的 load 事件");
  assert.match(src, /seq !== _preview\.loadSeq \|\| loaded/, "异步结论没有对导航序号，换了地址还会弹旧的提示");
  assert.match(src, /X-Frame-Options/, "「不让被嵌」这一种没有单独的说法");
  // 两种结论必须真的分岔，不能都说同一句
  assert.match(src, /reachable\s*\?/);
});
