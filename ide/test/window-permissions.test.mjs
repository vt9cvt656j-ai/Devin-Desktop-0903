// 窗口命令的**权限**必须覆盖代码真正调用的那些。
//
// 2026-08-26：用户报「Windows 上放大/全屏报错，然后就没法用了」。真因不在窗口代码里，
// 在 Tauri 的能力清单里：`src-tauri/capabilities/default.json` 放行了 close / destroy /
// start_dragging / set_title 等等，**唯独没放行 minimize 和 toggle_maximize**——
// 而标题栏那三个按钮接的正是它们。调用被 ACL 拒掉 → promise 抛出 → 全局的
// unhandledrejection 处理器弹一句 `Error: …` → 按钮点了没反应。
//
// **为什么只有 Windows 上炸**：macOS 走 titleBarStyle:"Overlay"，系统红绿灯还在，
// 那排自绘按钮的 CSS 是 `body.is-win .titlebar__winctl { display: flex }` —— mac 用户
// 根本看不到它们。Windows 那边 decorations:false，自绘按钮是**唯一**的窗口控制，
// 于是「最小化点了没反应、最大化报错、只有关闭能用」。
//
// 判据不写死某几个命令名，而是**从源码里把调用捞出来**再和展开后的权限比对：
// 下次谁加一个 window.setSize() 而忘了配权限，这条会红，而不是等用户在 Windows 上撞见。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..");

/** JS 侧的方法名 → Tauri 的命令名（后者是 snake_case）。 */
const toCommand = (m) => m.replace(/[A-Z]/g, (c) => "_" + c.toLowerCase());

/**
 * 把能力清单展开成「最终放行了哪些命令」。
 *
 * 一条权限可能是叶子（commands.allow 直接列命令），也可能是权限集合（permission_sets），
 * 还可能是插件的 default_permission。三种都要递归，否则会把 core:default 这种一揽子
 * 条目算成「什么都没放行」，得出一堆假阳性。
 */
function allowedCommands() {
  const man = JSON.parse(readFileSync(join(ROOT, "src-tauri/gen/schemas/acl-manifests.json"), "utf8"));
  const cap = JSON.parse(readFileSync(join(ROOT, "src-tauri/capabilities/default.json"), "utf8"));
  const out = new Set();
  const seen = new Set();
  const expand = (ref, depth = 0) => {
    if (depth > 10) return;
    const id = typeof ref === "string" ? ref : (ref?.identifier || "");
    if (!id || seen.has(id + depth)) return;
    seen.add(id + depth);
    let plugin = "core", name = id;
    if (id.includes(":")) {
      const [a, ...rest] = id.split(":");
      plugin = a; name = rest.join(":");
      if (name.includes(":")) { const [sub, ...r2] = name.split(":"); plugin = `${plugin}:${sub}`; name = r2.join(":"); }
    }
    const m = man[plugin] || {};
    const ps = m.permissions || {};
    const sets = m.permission_sets || {};
    if (ps[name]) { for (const c of ps[name].commands?.allow || []) out.add(c); return; }
    if (sets[name]) { for (const r of sets[name].permissions || []) expand(String(r).includes(":") ? r : `${plugin}:${r}`, depth + 1); return; }
    if (name === "default") {
      for (const r of (m.default_permission?.permissions) || []) expand(String(r).includes(":") ? r : `${plugin}:${r}`, depth + 1);
    }
  };
  for (const p of cap.permissions || []) expand(p);
  return out;
}

/** 源码里对当前窗口句柄调用的方法名。事件订阅和纯 JS 数组方法不是 Tauri 命令，排掉。 */
function windowMethodsUsed() {
  const src = readFileSync(join(ROOT, "src/main.js"), "utf8");
  const NOT_COMMANDS = new Set([
    "onCloseRequested", "onResized", "onMoved", "onFocusChanged", "listen", "once",
    "map", "flatMap", "reduce", "slice", "some", "filter", "forEach", "join", "find",
  ]);
  const hits = new Set();
  for (const m of src.matchAll(/\bcurrentWindow\.([a-zA-Z]+)\(/g)) hits.add(m[1]);
  for (const m of src.matchAll(/getCurrentWindow\(\)\.([a-zA-Z]+)\(/g)) hits.add(m[1]);
  return [...hits].filter((n) => !NOT_COMMANDS.has(n)).sort();
}

test("代码调用的每一个窗口命令，能力清单里都必须放行", () => {
  const allowed = allowedCommands();
  assert.ok(allowed.size > 50,
    `展开后只放行了 ${allowed.size} 个命令，展开器坏了——这条会把所有调用都判成缺权限`);

  const used = windowMethodsUsed();
  assert.ok(used.length >= 4, `只从源码里捞到 ${used.length} 个窗口方法：${used}`);

  const missing = used.filter((m) => !allowed.has(toCommand(m)));
  assert.deepEqual(missing, [],
    `这些窗口方法代码在调、能力清单却没放行：${missing.map((m) => `${m}() → core:window:allow-${toCommand(m).replace(/_/g, "-")}`).join("、")}\n`
    + "调用会被 ACL 拒掉、抛出，然后被全局 unhandledrejection 变成一句 Error 提示。\n"
    + "**Windows 上尤其致命**：那边 decorations:false，自绘按钮是唯一的窗口控制；\n"
    + "而 macOS 保留了系统红绿灯、那排按钮 CSS 上就不显示，所以本机测不出来。");
});

test("最小化和最大化确实在放行名单里（用户报的就是这两个）", () => {
  const allowed = allowedCommands();
  for (const cmd of ["minimize", "toggle_maximize", "close", "start_dragging"]) {
    assert.ok(allowed.has(cmd), `core:window 的 ${cmd} 没放行——标题栏按钮会报错`);
  }
  // 只读查询那几个本来就在 core:default 里，钉住它们别被顺手删掉。
  for (const cmd of ["is_maximized", "is_fullscreen"]) {
    assert.ok(allowed.has(cmd), `${cmd} 没放行——按钮图标的最大化状态会一直是错的`);
  }
});

test("Windows 的自绘标题栏是唯一控制，所以按钮必须真的显示", () => {
  // 这条守的是「为什么 mac 上测不出来」那半：CSS 只在 is-win 时显示那排按钮。
  // 哪天有人把它改成两个平台都显示、或都不显示，上面两条的前提就变了。
  const css = readFileSync(join(ROOT, "src/styles/app.css"), "utf8");
  assert.match(css, /body\.is-win \.titlebar__winctl \{ display: flex; \}/,
    "Windows 上那排窗口按钮不显示了——decorations:false 之下就没有任何窗口控制了");
  const win = JSON.parse(readFileSync(join(ROOT, "src-tauri/tauri.windows.conf.json"), "utf8"));
  assert.equal(win.app.windows[0].decorations, false,
    "Windows 的 decorations 变了——它决定了自绘按钮是不是唯一控制，两条要一起看");
});
