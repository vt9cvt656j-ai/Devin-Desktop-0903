// 注入到**被预览页面**里的调试桥。
//
// iframe 跨源，父窗口读不到里面的 console 和 DOM——只有页面自己送出来这一条路。
//
// # 它是怎么进到页面里的
//
// 由 src-tauri 里那个 `preview-bridge` 插件用 `js_init_script_on_all_frames` 注入。
// 那条路最终落到 WKWebView 的 `WKUserScript(forMainFrameOnly: false)`，于是**应用里
// 每一个帧**都会在文档解析前拿到它，包括跨源的 iframe。
//
// 这一点很重要：上一版是让 IDE 自带的 Python 预览服务把它插进 HTML，于是只有那一个
// 服务起的预览才有桥——用户自己的 vite / next / django 一律没有，「指元素」永远弹
// 「这个页面还没接调试桥」。现在不需要用户改自己的项目，任何本地 dev server 都能用。
//
// # 写法约束
//
// 这个文件同时被两边读：Rust 侧 `include_str!` 编进二进制，前端侧用 vite 的 `?raw`
// 导入。所以它必须是**能直接执行的一段脚本**，不是模块——不要 import/export。

(function () {
  // 位置守卫。这段由插件注入到**应用的每一个帧**，所以 IDE 自己那一帧、PDF 预览的
  // iframe、图片选择器的 iframe 都会执行到。只在真正被预览的网页里装：
  //   · tauri: / asset: / about: —— 应用自身和它的内部 iframe，一律不装
  //   · http / https —— 被预览的 dev server 页面，装
  // Tauri 的文档也反复强调注入全帧时要按 location 自守，理由就是这个。
  if (location.protocol !== "http:" && location.protocol !== "https:") return;
  if (window.__mrdayoneBridge) return;

  // 只在**子帧**里装。这段脚本由 Tauri 插件注入到应用的每一个帧，包括 IDE 自己那一帧；
  // 在主帧里挂 console 钩子会把 IDE 自身的日志也劫走，而拾取逻辑更是完全无意义。
  if (window.parent === window) return;

  window.__mrdayoneBridge = 1;

  var send = function (msg) {
    try { parent.postMessage(msg, "*"); } catch (e) {}
  };
  var log = function (level, text, src) {
    send({ __mrdayone: "preview-log", level: level, msg: String(text).slice(0, 600), src: src || "" });
  };

  // ---- 1. 控制台与未捕获错误 ----
  ["error", "warn"].forEach(function (lv) {
    var orig = console[lv];
    console[lv] = function () {
      try {
        log(lv, Array.prototype.map.call(arguments, function (a) {
          if (typeof a === "string") return a;
          try { return JSON.stringify(a); } catch (e) { return String(a); }
        }).join(" "));
      } catch (e) {}
      return orig.apply(console, arguments);
    };
  });
  window.addEventListener("error", function (e) {
    log("error", (e && e.message) || "script error",
        e && e.filename ? e.filename + ":" + (e.lineno || 0) : "");
  }, true);
  window.addEventListener("unhandledrejection", function (e) {
    var r = e && e.reason;
    log("error", "unhandledrejection: " + ((r && r.message) || r));
  });

  // ---- 2. 拾取元素 ----
  //
  // 高亮层用 position:fixed + pointer-events:none 盖在页面上，不改动页面本身的任何
  // 样式或结构——拾取模式退出后页面必须和进来之前一模一样。
  var box = null, picking = false, hovered = null;
  var ensureBox = function () {
    if (box) return box;
    box = document.createElement("div");
    box.style.cssText = "position:fixed;z-index:2147483647;pointer-events:none;border:2px solid #0a84ff;" +
      "background:rgba(10,132,255,.12);border-radius:3px;transition:all .05s;display:none";
    document.documentElement.appendChild(box);
    return box;
  };
  var paint = function (el) {
    if (!el) { if (box) box.style.display = "none"; return; }
    var r = el.getBoundingClientRect();
    var b = ensureBox();
    b.style.display = "block";
    b.style.left = r.left + "px"; b.style.top = r.top + "px";
    b.style.width = r.width + "px"; b.style.height = r.height + "px";
  };
  var cssPath = function (el) {
    // 短、稳、够用：优先 id，其次带一个类名的标签，再退到 nth-of-type。
    if (el.id) return "#" + el.id;
    var parts = [], node = el, depth = 0;
    while (node && node.nodeType === 1 && depth < 4) {
      var part = node.tagName.toLowerCase();
      var cls = (typeof node.className === "string" ? node.className : "").trim().split(/\\s+/)[0];
      if (cls) part += "." + cls;
      else {
        var p = node.parentElement;
        if (p) {
          var same = Array.prototype.filter.call(p.children, function (c) { return c.tagName === node.tagName; });
          if (same.length > 1) part += ":nth-of-type(" + (same.indexOf(node) + 1) + ")";
        }
      }
      parts.unshift(part);
      if (node.id) { parts[0] = "#" + node.id; break; }
      node = node.parentElement; depth++;
    }
    return parts.join(" > ");
  };
  var sourceOf = function (el) {
    // 框架的 dev 模式会把源码位置留在 DOM 上。找得到就给准确的文件:行，
    // 找不到就不编——**没有源码定位时必须说没有**，猜一个出来会让改文字那条路写错文件。
    var n = el;
    for (var i = 0; i < 6 && n; i++) {
      var v = n.getAttribute && (n.getAttribute("data-source") || n.getAttribute("data-v-inspector") ||
                                 n.getAttribute("data-inspector-file"));
      if (v) {
        var m = String(v).match(/^(.*?):(\\d+)(?::(\\d+))?$/);
        if (m) return { file: m[1], line: Number(m[2]) };
        return { file: String(v), line: 0 };
      }
      if (n.__reactFiber$ || n._debugSource) {
        var ds = (n._debugSource) || (n.__reactFiber$ && n.__reactFiber$._debugSource);
        if (ds && ds.fileName) return { file: ds.fileName, line: ds.lineNumber || 0 };
      }
      n = n.parentElement;
    }
    return null;
  };
  var describe = function (el) {
    var cs = getComputedStyle(el);
    var r = el.getBoundingClientRect();
    return {
      tag: el.tagName.toLowerCase(),
      selector: cssPath(el),
      text: (el.textContent || "").trim().slice(0, 200),
      cls: typeof el.className === "string" ? el.className : "",
      isLeaf: el.children.length === 0,
      source: sourceOf(el),
      color: cs.color, background: cs.backgroundColor,
      fontSize: cs.fontSize, fontWeight: cs.fontWeight,
      padding: cs.padding, margin: cs.margin, borderRadius: cs.borderRadius,
      size: Math.round(r.width) + "x" + Math.round(r.height),
      outerHTML: (el.outerHTML || "").slice(0, 400)
    };
  };
  var onMove = function (e) {
    if (!picking) return;
    hovered = e.target;
    paint(hovered);
  };
  var onClick = function (e) {
    if (!picking) return;
    e.preventDefault(); e.stopPropagation();
    stop();
    send({ __mrdayone: "preview-picked", el: describe(e.target) });
  };
  var onKey = function (e) { if (picking && e.key === "Escape") { stop(); send({ __mrdayone: "preview-picked", el: null }); } };
  var start = function () {
    if (picking) return;
    picking = true;
    document.documentElement.style.cursor = "crosshair";
    window.addEventListener("mousemove", onMove, true);
    window.addEventListener("click", onClick, true);
    window.addEventListener("keydown", onKey, true);
  };
  var stop = function () {
    picking = false;
    document.documentElement.style.cursor = "";
    paint(null);
    window.removeEventListener("mousemove", onMove, true);
    window.removeEventListener("click", onClick, true);
    window.removeEventListener("keydown", onKey, true);
  };
  window.addEventListener("message", function (e) {
    var d = e && e.data;
    if (!d || typeof d !== "object" || d.__mrdayone !== "preview-pick") return;
    if (d.on) start(); else stop();
  });

  // 桥装好了先报一声：父窗口据此知道这个页面有没有接桥，
  // 从而决定「指元素」能不能用、控制台面板该显示日志还是显示怎么接。
  log("info", "调试桥已接入");
})();
