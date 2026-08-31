// 拖文件/文件夹到文件树 = 复制进工作区（VS Code 那种），不是换项目。
//
// 用户报的：把文件、目录拖到「工作目录那块区域」，软件把项目重新打开了，而不是像
// VS Code 那样变成子目录 / 子文件。改法是按落点分工：文件树 → 复制进去；编辑器区 →
// 还是打开（文件夹才当"换项目"）。这里跑的是纯逻辑那半，全部真往返。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import {
  baseName, parentOf, joinPath, isInsideOrSame, splitExt, uniqueName,
  dropDirFor, planExplorerDrop, moveRejection, planMove, topLevelOf, chipBeside,
  addHidden, clearHidden, hiddenFor, isHidden,
} from "../src/agent/explorer-drop.js";
import { load } from "./helpers/source.mjs";
import { _mergeChatArchives as _mca } from "../src/agent/chat-archive.js";

const HERE = dirname(fileURLToPath(import.meta.url));

// main.js 那三段是 DOM 绑定的（getBoundingClientRect / elementFromPoint），搬不进模块，
// 但可以用 load() 注入假 DOM **真跑**——比 assert.match(SRC,…) 强：源码断言证明不了
// 「落点算得对不对」。下面这些造的都是最小假件。
const rect = (left, top, right, bottom) => ({ left, top, right, bottom, width: right - left, height: bottom - top });
const elWithRect = (r) => ({ getBoundingClientRect: () => r });
const _dropPointIn = load("_dropPointIn", {
  _dropCssPoint: load("_dropCssPoint", { _dropScale: 1, window: { devicePixelRatio: 1 }, viewportW: () => 4000, viewportH: () => 4000 }),
});

test("路径基本件：末尾斜杠不影响结果", () => {
  assert.equal(baseName("/a/b/c.txt"), "c.txt");
  assert.equal(baseName("/a/b/"), "b");
  assert.equal(parentOf("/a/b/c.txt"), "/a/b");
  assert.equal(parentOf("/a"), "/");
  assert.equal(joinPath("/a/b", "c.txt"), "/a/b/c.txt");
  assert.equal(joinPath("/a/b/", "c.txt"), "/a/b/c.txt");
});

test("落点解析：文件夹行进它自己，文件行进它所在目录，空白处进根", () => {
  assert.equal(dropDirFor({ rowPath: "/w/src", rowIsDir: true, rootPath: "/w" }), "/w/src");
  // 落在文件上不该变成「放进文件里」——要落到它所在的目录。
  assert.equal(dropDirFor({ rowPath: "/w/src/a.js", rowIsDir: false, rootPath: "/w" }), "/w/src");
  assert.equal(dropDirFor({ rowPath: "", rootPath: "/w" }), "/w");
});

test("自我拖入必须被拒——否则递归复制会自噬", () => {
  // 这条是这个功能最危险的输入：把文件夹拖进它自己（或它的子目录），
  // 后端的 copy_dir_recursive 会一边读一边往里写，无限长出嵌套目录。
  assert.equal(isInsideOrSame("/w/src", "/w/src"), true);
  assert.equal(isInsideOrSame("/w/src/deep", "/w/src"), true);
  assert.equal(isInsideOrSame("/w/other", "/w/src"), false);
  // 前缀相同但不是子目录：/w/srcx 不在 /w/src 里面。
  assert.equal(isInsideOrSame("/w/srcx", "/w/src"), false);

  const plan = planExplorerDrop({
    items: [{ path: "/w/src", isDir: true }],
    destDir: "/w/src/deep",
    existingNames: [],
  });
  assert.deepEqual(plan.copies, [], "把文件夹拖进自己的子目录，必须一条都不复制");
  assert.deepEqual(plan.skipped, [{ path: "/w/src", reason: "self" }]);
});

test("重名让路：Finder 写法，且扩展名要保住", () => {
  assert.deepEqual(splitExt("notes.txt"), { stem: "notes", ext: ".txt" });
  assert.deepEqual(splitExt("src", true), { stem: "src", ext: "" });
  // 开头的点不是扩展名，否则 .gitignore 会变成「 2.gitignore」。
  assert.deepEqual(splitExt(".gitignore"), { stem: ".gitignore", ext: "" });

  assert.equal(uniqueName("notes.txt", []), "notes.txt", "不重名就原样");
  assert.equal(uniqueName("notes.txt", ["notes.txt"]), "notes 2.txt");
  assert.equal(uniqueName("notes.txt", ["notes.txt", "notes 2.txt"]), "notes 3.txt");
  assert.equal(uniqueName("src", ["src"], true), "src 2", "文件夹不拆扩展名");
  assert.equal(uniqueName(".gitignore", [".gitignore"]), ".gitignore 2");
});

test("整批投放：同一批里的同名文件也要互相让路", () => {
  // 从两个不同目录各拖一个 a.txt 进来：第二个不能算出和第一个一样的目标，
  // 否则后一次 copyPath 会撞上前一次刚写下的文件而报错。
  const plan = planExplorerDrop({
    items: [{ path: "/x/a.txt" }, { path: "/y/a.txt" }, { path: "/z/a.txt" }],
    destDir: "/w",
    existingNames: [],
  });
  assert.deepEqual(plan.copies.map((c) => c.to), ["/w/a.txt", "/w/a 2.txt", "/w/a 3.txt"]);
  assert.equal(plan.copies[0].renamed, false);
  assert.equal(plan.copies[1].renamed, true);
});

test("整批投放：躲开目标目录里已有的名字", () => {
  const plan = planExplorerDrop({
    items: [{ path: "/x/README.md" }, { path: "/x/lib", isDir: true }],
    destDir: "/w",
    existingNames: ["README.md", "lib", "lib 2"],
  });
  assert.deepEqual(plan.copies.map((c) => c.to), ["/w/README 2.md", "/w/lib 3"]);
});

test("目标目录为空时不产出任何复制", () => {
  // 没打开工作区（rootPath 为空）时落进来，不能算出一个 "/name" 往根目录写。
  const plan = planExplorerDrop({ items: [{ path: "/x/a.txt" }], destDir: "", existingNames: [] });
  assert.deepEqual(plan.copies, []);
});

test("落点坐标：默认当 CSS 用，越出视口才认定是物理像素", () => {
  // 这块踩过两次，两次都表现为「放在下面，亮的是上面」：
  //  ① 「两套都试、先试除以 dpr」——侧栏又窄又高，减半后仍落在侧栏里，错的那个每次先命中。
  //  ② 「向窗口要 innerSize 除以 CSS 宽算比例」——界面缩放会改 clientWidth，比例又量歪。
  // 所以现在既不试两套也不测量：先当 CSS，观察到越出视口才锁定成 dpr。
  const el = elWithRect(rect(0, 0, 200, 1200));   // 又窄又高，正是当年翻车的形状

  // 逻辑点（wry 在 macOS 上报的就是这种）：一次都不许除。
  // 关键回归：810 减半是 405，同样落在这个矩形里——老写法会返回 405。
  const at = load("_dropPointIn", {
    _dropCssPoint: load("_dropCssPoint", { _dropScale: 1, window: { devicePixelRatio: 2 },
      viewportW: () => 1000, viewportH: () => 1200 }),
  });
  assert.equal(at({ x: 100, y: 810 }, el).y, 810,
    "坐标被多除了一次——高亮会出现在光标位置的一半处");

  // 物理像素：越出 CSS 视口 → 锁定 dpr 换算。
  const phys = load("_dropCssPoint", { _dropScale: 1, window: { devicePixelRatio: 2 },
    viewportW: () => 1000, viewportH: () => 1200 });
  assert.deepEqual(phys({ x: 200, y: 1620 }), { x: 100, y: 810 }, "越界后要按 dpr 换算");
  // 锁定之后对后续坐标一直有效（同一次拖动里不能一帧一个结论）。
  assert.deepEqual(phys({ x: 100, y: 200 }), { x: 50, y: 100 }, "锁定后要持续按 dpr 换算");

  // dpr=1 时没有歧义，任何坐标都原样使用。
  const one = load("_dropCssPoint", { _dropScale: 1, window: { devicePixelRatio: 1 },
    viewportW: () => 1000, viewportH: () => 1200 });
  assert.deepEqual(one({ x: 100, y: 1900 }), { x: 100, y: 1900 });

  assert.equal(at({ x: 100, y: 810 }, null), null, "元素不存在时不该炸");
});

test("落点落在哪一行 → 进哪个目录（真跑 elementFromPoint）", () => {
  const tree = elWithRect(rect(0, 0, 200, 400));
  // 假树行**照抄真实 DOM 形状**（见 main.js 的行渲染 + 工作区根行渲染）：
  //   普通目录行： <svg class="chev">…            → 直接子元素有 .chev
  //   文件行：     <span class="chev-spacer">…    → 直接子元素有 .chev-spacer
  //   工作区根行： <button class="workspace-root__toggle"><svg class="chev">…
  //                → .chev 被包在按钮里，**不是**直接子元素；也没有 .chev-spacer
  // 这三种形状是这条判据唯一会遇到的输入，缺一种就守不住。
  const mkRow = (path, kind) => {
    const has = (sel) =>
      (sel === ":scope > .chev" && kind === "dir") ||
      (sel === ":scope > .chev-spacer" && kind === "file");
    const row = { dataset: { path }, querySelector: (sel) => (has(sel) ? {} : null) };
    row.closest = (sel) => (sel === ".row" ? row : null);
    return row;
  };
  const at = (row) => load("_dropDirAt", {
    _dropPointIn, _treeEl: tree, rootPath: "/w", dropDirFor,
    document: { elementFromPoint: () => row },
  })({ position: { x: 50, y: 50 } });

  assert.equal(at(mkRow("/w/src", "dir")), "/w/src", "落在文件夹行 → 进这个文件夹");
  assert.equal(at(mkRow("/w/src/a.js", "file")), "/w/src", "落在文件行 → 进它所在的目录");
  assert.equal(at(null), "/w", "落在空白处 → 进工作区根");
  // 回归闸：根行的 .chev 在按钮里。判据要是反过来认 .chev，这里会算出 "/"（工作区
  // **外面**的父目录），后端一律拒绝，用户看到的是"复制失败"而不是文件进了根目录。
  assert.equal(at(mkRow("/w", "root")), "/w",
    "工作区根行必须当成目录——认 .chev 会把它当文件，目标算到工作区外面去");
});

test("拖动中的高亮必须能扛住树被重建", () => {
  // fs-watcher 会在拖动途中 reloadDir/renderWorkspaceRoots 重建整棵树。高亮如果存节点
  // 引用，就会挂在一个已被丢弃的节点上——既清不掉也贴不上。所以每帧按类名扫一遍再按
  // **路径**重贴，重建后下一帧自愈。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /querySelectorAll\("\.row\.is-drop-into"\)/,
    "清理必须按类名扫，不能按保存的节点引用——树重建后引用就失效了");
  assert.match(src, /\.row\[data-path="\$\{cssEscape\(path\)\}"\]/,
    "重贴必须按 data-path 找行（树重建后节点是新的）");
});

test("拖放事件必须按窗口订阅，否则两个窗口会各干一遍", () => {
  // event.js 的 listen() 不传 options 时 target 默认 {kind:"Any"}，而 Any 在 Rust 侧会短路掉
  // 标签过滤。这个应用能开第二个窗口（⇧⌘N / 文件菜单 / 命令面板），于是在 B 窗口拖一次，
  // A 窗口也会收到同一串事件：用 B 的坐标点亮 A 的高亮，松手时两边各跑一遍 _handleDrop
  // —— 文件被复制两份，或者一边复制、另一边把项目换掉。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const block = src.slice(src.indexOf("tauri://drag-enter") - 1500, src.indexOf("tauri://drag-drop") + 400);
  assert.match(block, /const listen = \(name, fn\) => getCurrentWebviewWindow\(\)\.listen\(name, fn\);/,
    "拖放事件还在用全局 listen()——多窗口下一次拖放会被两个窗口各执行一遍");
  // 别名之外不许再从 event 模块直接取 listen 来订阅拖放（那条默认 target 是 Any）。
  assert.doesNotMatch(block, /import\("@tauri-apps\/api\/event"\)\.then\(\(\{ listen \}\)/,
    "拖放这段又回到从 event 模块直接取 listen 了，默认 target 是 Any");
});

test("浏览器路径传原始 client 坐标，不许再乘 dpr", () => {
  // 之前为了抵消 _dropPointIn 里那次误除，在浏览器路径上乘了一次 dpr。现在换算只做一次
  // 且按实测比例（浏览器下 _dropScale=1），再乘就又偏了。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const zone = src.slice(src.indexOf("// Browser / non-Tauri path"), src.indexOf("tauri://drag-enter"));
  assert.match(zone, /position: \{ x: e\.clientX, y: e\.clientY \}/, "浏览器路径没有传原始 client 坐标");
  assert.doesNotMatch(zone, /clientX \* /, "浏览器路径又在乘 dpr 了");
  // 换算比例必须是**量出来的**，不是拿 devicePixelRatio 猜的。
  // 换算比例既不猜也不靠会被界面缩放污染的测量：默认当 CSS，越界才锁 dpr。
  assert.match(src, /if \(dpr > 1 && \(p\.x > viewportW\(\) \|\| p\.y > viewportH\(\)\)\) _dropScale = dpr;/,
    "坐标换算不是自校正的——界面缩放会把测量出来的比例带歪");
  assert.doesNotMatch(src, /innerSize\(\)/, "又回到靠 innerSize 测量比例了，缩放不为 1 时会量歪");
});

test("投放高亮照 VS Code：一块底色，不改文字，不加边框", () => {
  // 依据是 VS Code（Cursor 是它的分支）打包产物里的真实规则：
  //   .monaco-list-row.drop-target { background-color: <list.dropBackground>; color: inherit !important; }
  // 就这一条。list.dropBackground 实测色值：浅色 #D6EBFF、深色 #062F4A。
  // 上一版我自创了上下边框 + 左粗条 + 子树染色 + 文字变强调色加粗 + 跟随光标的标签 +
  // 编辑器区虚线框 + 侧栏压暗，用户的评价是「完全和 vscode 不一样」。这条守住不要再长回来。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  assert.match(css, /--drop-bg:\s*#D6EBFF/i, "浅色投放底色要和 VS Code 的 list.dropBackground 一致");
  assert.match(css, /--drop-bg:\s*#062F4A/i, "深色投放底色要和 VS Code 的一致");
  assert.match(css, /#tree \.row\.is-drop-into::before \{ background: var\(--drop-bg\); \}/,
    "投放高亮必须就是一块底色");
  // 自创的那几样都不许回来。
  for (const gone of ["drop-chip", "editor-dropzone", "drag-open--replace", "is-drop-into + .children"]) {
    assert.ok(!css.includes(gone), `自创视觉又回来了：${gone}——VS Code 没有这种东西`);
  }
  assert.ok(!/\.row\.is-drop-into::after/.test(css), "投放态不该再画边框/竖条");
  assert.ok(!/is-drop-into \.name/.test(css), "VS Code 明确 color:inherit——投放态不改文字颜色");
});

test("拖放进行中要关掉 hover 高亮", () => {
  // VS Code 的 hover 选择器带 :not(.drop-target)：两块高亮同时亮着，用户分不清哪个是落点。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  assert.match(css, /#tree\.is-dropping \.row:hover::before \{ background: transparent; \}/,
    "拖放中没有关掉 hover 高亮");
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /_treeEl\?\.classList\.toggle\("is-dropping"/, "没有在拖放时给树挂 is-dropping");
});

test("悬停折叠目录 500ms 自动展开，且只展开不收起", () => {
  // VS Code 的 autoExpand：实测就是 500ms，只在**目标行变化**时重新计时，且从不自动收起
  //（拖动中把列表收回去，落点会在脚下跳）。没有它，折叠的子目录根本没法作为落点。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /_springTimer = setTimeout\(/, "没有悬停自动展开——折叠目录无法作为落点");
  assert.match(src, /\}, 500\);/, "自动展开延时要和 VS Code 一致（500ms）");
  assert.doesNotMatch(src, /_treeSetExpanded\([^,]+, false\)/, "不许自动收起：拖动中列表跳动");
  // 工作区**根**行是另一套展开状态：_treeSetExpanded / expandDir 对根都直接 return，
  // 只走那条路的话，折叠的根永远展不开。
  assert.match(src, /_rootCollapsed[\s\S]{0,400}collapsedWorkspaceRoots\.delete\(dest\)/,
    "折叠的工作区根行没法被悬停展开——它走的是 collapsedWorkspaceRoots，不是 expandedTreeDirs");
});

test("落点反馈要覆盖整棵可见子树，不只是那一行", () => {
  // VS Code 的 onDragOver 结尾是 `feedback: L6(u, u + getListRenderCount(loc))` ——
  // 覆盖目标行**连同它已渲染的子孙**，而且是同一个颜色。只染一行说不清"东西进的是这个容器"。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const fn = src.slice(src.indexOf("function _paintDropRow"), src.indexOf("function _showDrop"));
  assert.match(fn, /nextElementSibling/, "没有取紧邻的 .children —— 子树不会被染");
  assert.match(fn, /classList\?\.contains\("children"\)/, "子树容器判据不对");
  assert.match(fn, /querySelectorAll\("\.row"\)[\s\S]{0,90}add\("is-drop-into"\)/,
    "子树里的行没有被加上同一个投放态");
});

test("文件夹落到项目根 = 直接换成这个项目，不再弹框", () => {
  // 用户原话：「直接替换工作区目录内容就行，不然的话没啥意义」。把一个项目文件夹拖到项目
  // 根上本来就只有这一个合理意图，先问一遍反而多一步。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const blk = src.slice(src.indexOf("async function _copyIntoWorkspace"), src.indexOf("async function _handleDrop"));
  assert.match(blk, /workspaceRoots\.includes\(destDir\)[\s\S]{0,400}openFolder\(_dirs\[0\]\.path\)/,
    "文件夹落到工作区根没有直接换项目");
  assert.doesNotMatch(blk, /ioConfirm\(/, "这一档不该再弹框");
  // 一次拖进来好几个：其余的挂成额外的根，别静默丢掉。
  assert.match(blk, /_dirs\.slice\(1\)[\s\S]{0,90}_addWorkspaceRoot/,
    "多个文件夹时，多余的被静默丢弃了");
  // 只对**文件夹**、且只在根上。拖文件、或把文件夹拖进子目录，照旧走复制。
  assert.match(blk, /_dirs = items\.filter\(\(x\) => x\.isDir\)/, "判据必须是「拖的是文件夹」");
});

test("移除 = 从树里藏起来，不是删除", () => {
  // 用户：「不是移除到废纸篓，而是移除 让用户看不见 而不是真正的删除，我这里写了删除按钮了都」。
  // 所以这条路一次都不许碰文件系统。
  let store = {};
  store = addHidden(store, "/w", "/w/logs");
  store = addHidden(store, "/w", "/w/a.txt");
  assert.deepEqual(hiddenFor(store, "/w"), ["/w/logs", "/w/a.txt"]);
  // 重复移除同一个不该堆两条
  assert.deepEqual(hiddenFor(addHidden(store, "/w", "/w/logs"), "/w"), ["/w/logs", "/w/a.txt"]);
  // 按项目分开：换个项目互不影响
  assert.deepEqual(hiddenFor(store, "/other"), []);

  const list = hiddenFor(store, "/w");
  assert.equal(isHidden(list, "/w/logs"), true, "被移除的目录本身要藏");
  assert.equal(isHidden(list, "/w/logs/x.log"), true, "它底下的东西也要藏，否则展开父目录又冒出来");
  assert.equal(isHidden(list, "/w/logs2"), false, "同前缀的兄弟目录不该被连坐");
  assert.equal(isHidden(list, "/w/src"), false);
  // 恢复
  assert.deepEqual(hiddenFor(clearHidden(store, "/w"), "/w"), []);

  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  // 菜单项的标签跟着选中的类型走
  assert.match(src, /isDir \? "移除目录" : "移除文件"/, "菜单没有按类型给标签");
  // 移除必须只改显示：这条路上不许出现任何文件系统调用
  const fn = src.slice(src.indexOf("async function _hideEntry"), src.indexOf("async function _restoreHidden"));
  assert.doesNotMatch(fn, /deletePath|trashPath|renamePath|remove_dir|backend\.\w*[Dd]elete/,
    "「移除」碰了文件系统——它只该改显示");
  assert.match(fn, /addHidden\(_hiddenStore, rootPath, path\)/, "没有把它加进隐藏清单");
  // 渲染时真的过滤掉了
  assert.match(src, /if \(_hidden\.length && isHidden\(_hidden, item\.path\)\) continue;/,
    "树渲染没有过滤掉被移除的条目");
  // 必须有回头路，否则「移除」是单向操作
  assert.match(src, /恢复已移除的 \$\{_nHidden\} 项/, "没有恢复入口——移除就找不回来了");
});

test("ioConfirm 的第三按钮是可选的，不影响老调用方", () => {
  // 加第三个按钮时改的是共享组件。不给 altLabel 时必须仍然 resolve 布尔，
  // 否则每一处 `if (await ioConfirm(...))` 都会因为拿到字符串 "cancel"（真值）而反转。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const fn = src.slice(src.indexOf("function ioConfirm({"), src.indexOf("// ---- Global search ----"));
  assert.match(fn, /altLabel = ""/, "altLabel 必须有默认空值");
  assert.match(fn, /resolve\(altLabel \? returnValue : confirmed\)/,
    "不给 altLabel 时必须仍返回布尔——否则老调用方的判断会反转");
});

test("树内拖动移动：非法情形必须在计划期就拒", () => {
  // VS Code 的 handleDragOver 对这三种一律 return false。前两条要是漏了，rename 会把目录
  // 搬进它自己，整棵子树当场消失。
  assert.equal(moveRejection({ src: "/w/src", destDir: "/w/src" }), "self", "拖到自己身上");
  assert.equal(moveRejection({ src: "/w/src", destDir: "/w/src/deep" }), "self", "拖进自己的子目录");
  assert.equal(moveRejection({ src: "/w/src/a.js", destDir: "/w/src" }), "same", "本来就在这个目录里");
  assert.equal(moveRejection({ src: "/w/src/a.js", destDir: "/w/lib" }), "", "正常移动要放行");
  // 前缀相同但不是子目录：/w/srcx 不在 /w/src 里
  assert.equal(moveRejection({ src: "/w/src", destDir: "/w/srcx" }), "", "同前缀的兄弟目录是合法目标");
});

test("树内拖动移动：整批计划与同名冲突", () => {
  const plan = planMove({ paths: ["/w/a.js", "/w/sub/a.js", "/w/b.js"], destDir: "/w/lib" });
  assert.deepEqual(plan.moves.map((m) => m.to), ["/w/lib/a.js", "/w/lib/b.js"]);
  // 同一批里两个 a.js：后端 rename 在目标已存在时直接报错，与其半路失败不如计划期就标出来。
  assert.deepEqual(plan.skipped, [{ path: "/w/sub/a.js", reason: "dup" }]);
  // 目标为空 → 一条都不产出（松手在树外面时就是这种情况）
  assert.deepEqual(planMove({ paths: ["/w/a.js"], destDir: "" }).moves, []);
});

test("树内拖动真的接上了移动，而不是只会 @ 引用", () => {
  // 这是用户报的核心问题：VS Code 里拖着文件就能挪进另一个目录，我们以前拖到别的文件夹上
  // 什么都不发生（_wireTreeDragToComposer 只处理"落在输入框"这一种）。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const drag = src.slice(src.indexOf("function _wireTreeDragToComposer"));
  assert.match(drag.slice(0, 3000), /_moveIntoDir\(candPaths\(\), _treeDropDirAt\(/,
    "松手落在树里没有触发移动");
  assert.match(drag.slice(0, 3000), /_treeDropDirAt\(e\.clientX, e\.clientY, candPaths\(\)\)[\s\S]{0,80}_paintDropRow\(/,
    "拖动中没有高亮将要接收它的那个目录");
  assert.match(drag.slice(0, 3000), /is-drop-root/, "树内拖动没有整树高亮——空白处拖动看不到反馈");
  // 移动本身要走 renamePath，并且照 renameEntry 的规矩先关掉已打开的文件。
  const mv = src.slice(src.indexOf("async function _moveIntoDir"), src.indexOf("(function _wireTreeDragToComposer"));
  assert.match(mv, /_closeOpenFilesUnder\(m\.from\)/, "移动前没有关掉已打开的文件");
  assert.match(mv, /backend\.renamePath\(m\.from, m\.to\)/, "没有真的调用后端移动");
  assert.match(mv, /_treeMoveExpansionSubtree\(m\.from, m\.to\)/, "目录的展开状态没有跟着搬");
  // 源目录和目标目录都要刷新——只刷一个的话，另一边的树还是旧的。
  assert.match(mv, /dirs\.add\(parentDir\(m\.from\)\)/, "源目录没有被加进待刷新集合");
  assert.match(mv, /for \(const d of dirs\)[\s\S]{0,60}reloadDir\(d\)/, "没有刷新所有受影响的目录");
});

test("拖住的行在多选里就带上整组", () => {
  // VS Code 的 getStatsFromDragAndDropData 同理：拖住的那项若在选区内，整个选区一起走。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /_treeSel\.has\(_rowDragCandidate\.path\) \? \[\.\.\._treeSel\] : \[_rowDragCandidate\.path\]/,
    "多选拖动没接上——拖一组文件只会移动其中一个");
});

test("正在被拖的行不能当自己的落点", () => {
  // 拖住一个文件夹时，光标底下就是它自己，目标会恒等于自己 → 整个拖动看起来"没反应"。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const fn = src.slice(src.indexOf("function _treeDropDirAt"), src.indexOf("async function _moveIntoDir"));
  assert.match(fn, /dragging\.includes\(dir\) \? "" : dir/, "没有把正在拖的行排除掉");
});

test("三个以上动作的弹框要竖排，标签不许折断", () => {
  // 用户截图：四个按钮挤在 420px 的卡片里，「添加到工作区」「打开为新项目」都被折成两行。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  assert.match(css, /\.io-confirm-actions \.btn \{[^}]*white-space: nowrap;/s, "按钮标签会被从中间折断");
  assert.match(css, /\.io-confirm-actions--stack \{[^}]*flex-direction: column-reverse;/s,
    "三个以上动作没有改成竖排");
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /if \(alt \|\| alt2\) overlay\.querySelector\("\.io-confirm-actions"\)\.classList\.add/,
    "三个动作（ok/alt/cancel）时没有竖排——中文标签会被挤成两行");
  // .btn 基础样式是**文字按钮**（透明无边框），竖排后三行蓝字完全不像按钮。
  assert.match(css, /\.io-confirm-actions--stack \.btn \{[^}]*border: 1px solid var\(--line-strong\);/s,
    "竖排按钮没有补上描边——看起来还是三行链接");
  assert.match(css, /\.io-confirm-actions--stack \.btn--primary \{[^}]*background: var\(--ask-accent\);/s,
    "主按钮没有实心底色");
  // 主色不用满屏都在用的 --accent：那个亮蓝一上大按钮就很吵。两套主题各一份。
  assert.equal((css.match(/--ask-accent:/g) || []).length, 2, "决策弹框主色缺深浅两套");
  // 点下之后按钮会被禁用防重复提交。基础 .btn:disabled 只有 opacity:.5，落在实心主按钮上
  // 就是一块灰底 + 几乎看不见的字（用户实拍过），所以竖排里要单独给禁用态。
  assert.match(css, /\.io-confirm-actions--stack \.btn:disabled \{[^}]*opacity: 1;/s,
    "禁用态还是靠 opacity 变淡——实心主按钮会糊成一块灰、字看不见");
  assert.match(css, /\.io-confirm-actions--stack \.btn--primary:disabled \{[^}]*color: #fff;/s,
    "禁用的主按钮没有保住文字对比");
  // 这张卡以前写死 #fff/#202124，深色主题下整块是白的。
  assert.match(css, /\.io-confirm-card \{[\s\S]{0,400}background: var\(--panel-solid\);/,
    "弹框卡片没走主题令牌——深色下会是白底黑字");
});

test("嵌套多选必须折叠，否则移动会丢文件", () => {
  // 同时选中 A/ 和 A/x.txt 一起拖：不折叠的话会先把 A/ 搬走，再拿已经不存在的 A/x.txt
  // 去 rename，整批停在半路——而 A/ 已经动了，用户只看到一句"移动失败"。
  // 删除那条路一直在用这个折叠（_treeTopLevelTargets），移动最初漏了。
  assert.deepEqual(topLevelOf(["/w/A", "/w/A/x.txt"]), ["/w/A"], "子路径没有被折叠掉");
  assert.deepEqual(topLevelOf(["/w/A/x.txt", "/w/A"]), ["/w/A"], "顺序反过来也要折叠");
  assert.deepEqual(topLevelOf(["/w/A", "/w/B"]), ["/w/A", "/w/B"], "互不包含的要都留着");
  assert.deepEqual(topLevelOf(["/w/A", "/w/A"]), ["/w/A"], "重复项要去掉");
  // 前缀相同但不是子路径：/w/AB 不在 /w/A 底下
  assert.deepEqual(topLevelOf(["/w/A", "/w/AB"]), ["/w/A", "/w/AB"]);

  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const mv = src.slice(src.indexOf("async function _moveIntoDir"), src.indexOf("(function _wireTreeDragToComposer"));
  assert.match(mv, /planMove\(\{ paths: _treeTopLevelTargets\(paths\)/,
    "移动没有先折叠嵌套选择——会丢文件");
});

test("移动之后要把原本打开着的文件重新打开", () => {
  // 移动前必须先关掉受影响的页签（否则 fs watcher 会当成「外部程序删了你的文件」弹框），
  // 但关了就得开回来——移动不该顺手把用户的页签关掉。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const mv = src.slice(src.indexOf("async function _moveIntoDir"), src.indexOf("(function _wireTreeDragToComposer"));
  assert.match(mv, /const wasOpen = openFiles\.has\(m\.from\);/, "没有记下哪些文件本来开着");
  assert.match(mv, /if \(wasOpen\) reopened\.push\(m\.to\);/, "没有记录要在新位置重开的路径");
  assert.match(mv, /for \(const rp of reopened\)[\s\S]{0,80}openFile\(rp/, "移动后没有把页签开回来");
});

test("拖进来的东西必须用 stat 探类型，不能借 readDir", () => {
  // 两个真实症状都出自「拿 readDir 当 is-directory 探针」：
  //  ① readDir 要求路径在工作区内 → 从 /Volumes、/Applications 拖进来的**文件夹被误判成
  //     文件**，接着 copyPath 报 "access denied: ... outside all workspace roots"；
  //  ② readDir 会把整个目录枚举一遍 → 拖一个大文件夹进来，光探测就卡好几秒。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.doesNotMatch(src, /backend\.readDir\(p\)\.then\(\(\) => true\)/,
    "又拿 readDir 当类型探针了——大目录会卡、工作区外的目录会被误判成文件");
  // 三条拖放路径（侧栏复制 / 编辑器区打开 / drag-enter 预判）都要走 stat。
  assert.ok((src.match(/backend\.pathKinds\(/g) || []).length >= 3,
    "还有拖放路径没换成 pathKinds");
  // 探测失败时要有兜底，不能整批崩掉。
  assert.match(src, /\.catch\(\(\) => \w+\.map\(\(\) => 1\)\)/, "pathKinds 失败时没有兜底");
});

test("从工作区外拖进来要能复制，但写入侧边界不许松", () => {
  // 用户实拍："复制失败: access denied: path '/Volumes/API for Cursor' is outside all
  // workspace roots"。根因是 copy_path 连**源**也要求在工作区内，而拖进来的东西按定义
  // 就在外面。新增的 import_path 只放开读侧，写入仍然必须落在已打开的工作区根里。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const fn = src.slice(src.indexOf("async function _copyIntoWorkspace"), src.indexOf("async function _handleDrop"));
  assert.match(fn, /backend\.importPath\(c\.from, c\.to\)/,
    "拖放导入还在用 copyPath——源在工作区外时会被拒");
  const rs = readFileSync(join(HERE, "..", "src-tauri", "src", "files.rs"), "utf8");
  const imp = rs.slice(rs.indexOf("pub fn import_path"), rs.indexOf("pub fn copy_path"));
  assert.match(imp, /require_inside_workspace\(&to, true\)/, "目标必须仍然受工作区约束");
  assert.doesNotMatch(imp, /require_inside_workspace\(&from/, "源不该再要求在工作区内");
  // path_kinds 只吐类型码，不泄漏目录内容。
  const pk = rs.slice(rs.indexOf("pub fn path_kinds"), rs.indexOf("pub fn import_path"));
  assert.match(pk, /std::fs::metadata\(p\)/, "类型探测要用 stat");
  assert.doesNotMatch(pk, /read_dir/, "类型探测不该枚举目录");
});

test("落点是项目根时整棵树一起亮", () => {
  // 用户实拍：项目是空的（树里只有一行根行），在一大片空白上拖文件夹进来，**什么反馈都没有**。
  // VS Code 在没有具体目标行时把 drop-target 加在列表本身上，整块列表都是投放色。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /classList\.toggle\("is-drop-root", !!dest && dest === rootPath\)/,
    "外部拖入没有整树高亮");
  // 收尾必须清掉，否则整棵树会一直亮着，看起来像坏了。
  assert.ok((src.match(/remove\("is-dropping", "is-drop-root"\)/g) || []).length >= 2,
    "两条拖动路径的收尾都要清掉整树高亮");
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  assert.match(css, /#tree\.is-drop-root \{ background: var\(--drop-bg\); \}/,
    "整树高亮要用和行高亮同一个投放色");
});

test("右键的那一行要标出来，菜单关掉再清掉", () => {
  // 用户：「被右键的那个内容 记得也要有被鼠标摸上那种效果，不然的话用户不知道点的是哪个
  // 项目或者文件」。以前只有工作区根行会被选中，普通文件/目录右键后菜单浮在旁边、行上毫无
  // 标记——而菜单里第一项之一就是不可逆的「删除」。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /function openContextMenu\([^)]*\) \{[\s\S]{0,200}_paintCtxTarget\(entry\?\.path \|\| ""\)/,
    "右键时没有标记目标行");
  assert.match(src, /function closeContextMenu\(\) \{[\s\S]{0,220}_paintCtxTarget\(""\)/,
    "菜单关掉后没有清掉标记——会留下一行看起来像被选中");
  // 存路径不存节点引用：菜单开着时 fs-watcher 仍可能重建树。
  const fn = src.slice(src.indexOf("function _paintCtxTarget"), src.indexOf("function openContextMenu"));
  assert.match(fn, /querySelectorAll\("\.row\.is-ctx-target"\)/, "清理要按类名扫，不能存节点引用");
  assert.match(fn, /\.row\[data-path="\$\{cssEscape\(_ctxTargetPath\)\}"\]/, "重贴要按路径找行");

  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  // --active 只比 --hover 深 3.5%，右键一下几乎看不出变化（用户报「没有灰色的选中样式，
  // 不知道自己选了哪个」）。要用更重的灰 + 一圈描边，因为菜单里有不可逆的「删除」。
  assert.match(css, /#tree \.row\.is-ctx-target::before \{ background: var\(--row-picked\); \}/,
    "右键目标行的高亮太淡——看不出来选的是哪一行");
  assert.equal((css.match(/--row-picked:/g) || []).length, 2, "灰色选中缺深浅两套");
  assert.match(css, /#tree \.row\.is-ctx-target::after \{[^}]*box-shadow: inset 0 0 0 1px var\(--line-strong\);/s,
    "右键目标行没有描边");
  // 不能用 accent：那是「当前打开的文件」的语汇，混在一起分不清。
  assert.doesNotMatch(css, /is-ctx-target::before \{ background: var\(--accent\)/, "别和 is-active 撞色");
});

test("右键菜单要窄", () => {
  // 用户：「让这个框宽度窄一点，不然的话太长了，不好看」。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  const blk = css.slice(css.indexOf(".ctx-menu {"), css.indexOf(".ctx-menu .menu__item"));
  const m = blk.match(/min-width:\s*(\d+)px/);
  assert.ok(m, "找不到 .ctx-menu 的 min-width");
  assert.ok(Number(m[1]) <= 150, `右键菜单还是太宽（${m[1]}px）`);
});

test("普通点击不留持久选中标记——屏幕上只该有一处高亮", () => {
  // 用户实拍：点了 logs（紫底）之后，pyrightconfig.json 还带着"当前打开文件"的蓝底，
  // 两处高亮同时在，分不清哪个才是"我选中的"。原话：「应该被选中的内容只能有一个才对」。
  // 根因是普通点击也会把那一行塞进多选集合 _treeSel。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const fn = src.slice(src.indexOf("function _treeSelectClick"), src.indexOf("async function _deleteSelectedTree"));
  assert.ok(fn, "找不到 _treeSelectClick——测试台锚点失效了");
  assert.doesNotMatch(fn, /_treeSel = new Set\(\[path\]\);/,
    "普通点击又把那一行塞进多选集合了——会留下第二处高亮");
  assert.match(fn, /_treeSel\.clear\(\);\n  _treeAnchor = path;/,
    "普通点击应当清空选区、只留 anchor 供 ⇧ 连选起头");
  // 多选本身要留着：批量删除和整组拖动都靠它。
  assert.match(fn, /if \(e\.metaKey \|\| e\.ctrlKey\)/, "⌘ 点选没了");
  assert.match(fn, /if \(e\.shiftKey && _treeAnchor\)/, "⇧ 连选没了");
});

test("选中高亮统一成蓝色，不再用紫色", () => {
  // 用户：「不要用紫色 用 下面那种蓝色是好看的」。原来多选是写死的紫色，
  // 和「当前打开的文件」(--sel，蓝) 是两套语汇，同屏出现像两种不同的"选中"。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  const sel = css.slice(css.indexOf(".row.is-selected::before"), css.indexOf(".row .chev,"));
  assert.doesNotMatch(sel, /124, 92, 255/, "树的选中高亮还在用写死的紫色");
  assert.match(sel, /\.row\.is-selected::before \{ background: color-mix\(in srgb, var\(--accent\)/,
    "多选高亮没有走强调色");
  // 叠加态（多选 + 当前文件）要更深，否则两者分不出层次。
  assert.match(sel, /\.row\.is-selected\.is-active::before \{ background: color-mix\(in srgb, var\(--accent\) 22%/,
    "多选叠加当前文件时没有加深");
});

test("内联实体片跟随主题，不再写死 Google 蓝", () => {
  // 输入框里的 @引用片和发出去之后消息里的那枚，原本都是写死的 #1a73e8 / #e8f0fe / #d2e3fc。
  // 深色主题、以及用户换强调色之后全都对不上，所以统一走 var(--accent) + color-mix。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  for (const sel of [".composer-chip {", ".msg-mention {"]) {
    const blk = css.slice(css.indexOf(sel), css.indexOf("}", css.indexOf(sel)));
    assert.ok(blk, `找不到 ${sel}`);
    assert.doesNotMatch(blk, /#1a73e8|#e8f0fe|#d2e3fc/, `${sel} 还在用写死的 Google 蓝`);
    assert.match(blk, /color: var\(--accent\)/, `${sel} 没有走强调色令牌`);
    // 全圆角胶囊读起来是"标签"；这是被引用的**对象**，用小圆角矩形。
    assert.match(blk, /border-radius: 6px/, `${sel} 还是 999px 的胶囊`);
  }
});

test("深色覆盖不许再盖住实体片", () => {
  // 那条 [data-theme=dark] 覆盖原本把 .composer-chip 和气泡上的 .msg-mention 一起改成
  // 淡蓝底——前者已经走令牌不需要覆盖，后者盖上去就又糊回蓝上加蓝了。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  assert.doesNotMatch(css, /\[data-theme="dark"\] \.composer-chip/, "深色又硬覆盖 composer-chip 了");
  assert.doesNotMatch(css, /\[data-theme="dark"\] \.msg__body \.msg-mention/, "深色又硬覆盖气泡上的片了");
  // 拖拽幽灵是写死的浅色卡片，那条覆盖要留着。
  assert.match(css, /\[data-theme="dark"\] \.row-drag-ghost/, "拖拽幽灵的深色覆盖被误删了");
});

test("发出去之后 @github:owner/repo 仍然是 GitHub 图标和全名", () => {
  // 用户实拍：输入框里那枚是对的（GitHub 图标 + owner/repo），发出去之后变成**文件夹图标**、
  // 名字还被截成 `ThesisX`。根因是消息端只按"有没有扩展名"猜文件/文件夹——发送后消息里
  // 只剩纯文本 `@github:owner/repo`，带前缀的引用根本不是本地路径。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const fn = src.slice(src.indexOf("function _renderMentionsToHtml"), src.indexOf("function _renderMentionsToHtml") + 2200);
  assert.match(fn, /\^\(github\|gitlab\|mcp\):/, "消息端没有识别带前缀的引用");
  assert.match(fn, /iconSvg\(`i-brand-\$\{kind\}`/, "没有用品牌图标");
  // 只显示仓库名，owner 收进 tooltip：组织名常常比仓库名还长，摆在气泡里挤掉正文。
  assert.match(fn, /pfx\[2\]\.split\("\/"\)\.filter\(Boolean\)\.pop\(\)/, "仓库名没有取最后一段");
  assert.match(fn, /title="\$\{relAttr\}"/, "完整的 owner/repo 要留在 tooltip 里");
  // 本地文件那条老路要留着
  assert.match(fn, /folderIconUrl\(name, false\) : fileIconUrl\(name\)/, "本地文件/文件夹的图标丢了");

  // 品牌图标符号必须真实存在，否则 <use> 会静默渲染成空白
  const html = readFileSync(join(HERE, "..", "index.html"), "utf8");
  for (const id of ["i-brand-github", "i-brand-gitlab"]) {
    assert.ok(html.includes(`id="${id}"`), `图标符号 ${id} 不存在——<use> 会静默渲染成空白`);
  }
});

test("蓝气泡上的片不能是纯白块", () => {
  // 用户：「样式也丑 还是纯白的」。白底对比够但等于在气泡上挖个洞。
  // 改成比气泡更深的一层 + 白字：片仍待在蓝色世界里，观感是"凹进去"而不是异物。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  const i = css.indexOf(".msg.user .msg-mention, .msg__body .msg-mention {");
  const blk = css.slice(i, css.indexOf("}", i));
  assert.ok(i > 0, "找不到气泡上的覆盖规则");
  // 三版都被否掉了：纯白块（挖洞）、22% 半透明白（糊住）、压深的黑（在蓝底上读成黑块）。
  // 它们的共同毛病是都在给片找一个**新的面**。现在靠描边成立，让气泡的蓝直接透上来。
  assert.doesNotMatch(blk, /background: #fff;/, "又变回纯白块了");
  assert.doesNotMatch(blk, /background: rgba\(0, 0, 0/, "又变回黑块了");
  assert.match(blk, /border-color: rgba\(255, 255, 255, \.55\)/, "描边不够清晰，片会立不住");
  assert.match(blk, /color: #fff;/, "字要白");
});


const _S = (id, n = 1) => ({ id, memory: { recent: Array(n).fill(0) } });

test("关掉的会话不许在重启后复活", () => {
  // 用户实拍：「明明我关闭的会话，他居然自己给我又恢复那些对话 tab 了」。
  // 存档有三层，退出时只有 localStorage 镜像能**同步**写完（beforeunload/pagehide 跑不了
  // 异步），SQLite 快照往往停在几分钟前、里面还留着后来被关掉的会话。原来两份按 id 求并集，
  // 于是关掉的全被并回来。
  const primary = { sessions: [_S("A", 3), _S("B", 5), _S("C", 2)], closedSessions: [], activeIdx: 0 };
  const mirror = { sessions: [_S("A", 4), _S("C", 2)], closedSessions: [_S("B", 5)], activeIdx: 0, savedAt: 2 };
  const r = _mca(primary, mirror);
  assert.deepEqual(r.sessions.map((x) => x.id), ["A", "C"], "被关掉的 B 又复活了");
  assert.deepEqual(r.closedSessions.map((x) => x.id), ["B"]);
  // 内容仍要取更全的那份——镜像被预算截断过，不能整份采用。
  assert.equal(r.sessions.find((x) => x.id === "A").memory.recent.length, 4, "会话内容没有取更全的那份");
});

test("新开的会话不能因为快照旧就丢掉", () => {
  // 反向：快照里没有、只在退出镜像里出现的会话（用户最后新开的那个）必须留下。
  const primary = { sessions: [_S("A", 2)], closedSessions: [], activeIdx: 0 };
  const mirror = { sessions: [_S("A", 2), _S("NEW", 1)], closedSessions: [], activeIdx: 0, savedAt: 9 };
  assert.deepEqual(_mca(primary, mirror).sessions.map((x) => x.id), ["A", "NEW"]);
});

test("缺 savedAt 时按「镜像更新」判——快照那份历史上不写时间戳", () => {
  // SQLite 快照没有 savedAt，镜像有。缺失一律当旧：镜像是退出瞬间写的，一定不比快照旧。
  const primary = { sessions: [_S("A"), _S("GONE")], closedSessions: [], activeIdx: 0 };
  const mirror = { sessions: [_S("A")], closedSessions: [], activeIdx: 0, savedAt: 1 };
  assert.deepEqual(_mca(primary, mirror).sessions.map((x) => x.id), ["A"],
    "镜像更新却没按它的成员资格来");
  // 两边都没有时间戳 → 谁也不比谁新，这时不该把 primary 独有的删掉（宁可多不可少）。
  const noTs = _mca(primary, { sessions: [_S("A")], closedSessions: [], activeIdx: 0 });
  assert.ok(noTs.sessions.length >= 1);
});

test("没有 id 的会话保留，不能被当成「不在打开列表里」删掉", () => {
  const primary = { sessions: [{ memory: { recent: [0] } }], closedSessions: [], activeIdx: 0 };
  const mirror = { sessions: [_S("A")], closedSessions: [], activeIdx: 0, savedAt: 5 };
  assert.equal(_mca(primary, mirror).sessions.length, 2, "没有 id 的会话被误删了");
});

test("输入框里的 GitHub 片也只显示仓库名", () => {
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /label: r\.full_name\.split\("\/"\)\.pop\(\)/, "输入框里的片还在显示 owner/repo");
  // value 仍然是全名——发送出去的文本要能唯一定位仓库。
  assert.match(src, /value: r\.full_name, label:/, "片的值不能跟着截短，否则定位不到仓库");
});

test("光标紧挨着片时，左右键能跨过去（直接跑模块函数）", () => {
  // 用户实拍两次：片是输入框里第一个元素时，按左键**没反应**。片是 contentEditable=false
  // 的原子节点，WKWebView 不总能在它两侧摆放光标；两侧垫零宽空格试过——不够，浏览器有时
  // 压根不把空文本节点当成可落脚位置。所以自己接管方向键，判据就是这个函数。
  const TXT = (v) => ({ nodeType: 3, nodeValue: v, previousSibling: null, nextSibling: null, parentNode: null });
  const CHIP = () => ({ nodeType: 1, _chip: true, previousSibling: null, nextSibling: null, parentNode: null, childNodes: [] });
  const link = (nodes, parent) => {
    nodes.forEach((n, k) => { n.parentNode = parent; n.previousSibling = nodes[k - 1] || null; n.nextSibling = nodes[k + 1] || null; });
    parent.childNodes = nodes; return parent;
  };
  const isChip = (n) => !!n._chip;

  // [片][零宽空格] —— 片就是第一个元素，光标在那个零宽空格的开头。
  const chip = CHIP(), pad = TXT("\u200b");
  const box = { nodeType: 1, childNodes: [] };
  link([chip, pad], box);
  assert.equal(chipBeside({ container: box, node: pad, offset: 0, left: true, isChip }), chip,
    "左边就是片，却没认出来——按左键会走不动");

  // 反向：片右边有片，按右键要越过去。
  const chip2 = CHIP(), lead = TXT("hi");
  const box2 = { nodeType: 1, childNodes: [] };
  link([lead, chip2], box2);
  assert.equal(chipBeside({ container: box2, node: lead, offset: 2, left: false, isChip }), chip2);

  // 这一侧还有真字符可走时**不许**接管，否则逐字移动会被吞掉。
  assert.equal(chipBeside({ container: box2, node: lead, offset: 1, left: false, isChip }), null,
    "文本中间也接管了——逐字移动会失灵");
  assert.equal(chipBeside({ container: box2, node: lead, offset: 1, left: true, isChip }), null);
  // 旁边不是片就放行。
  const plain = TXT("x"), box3 = { nodeType: 1, childNodes: [] };
  link([plain, TXT("y")], box3);
  assert.equal(chipBeside({ container: box3, node: box3.childNodes[1], offset: 0, left: true, isChip }), null);

  // 接线：处理器真的用了它，并且阻止了默认行为（否则浏览器那套走不动的行为仍然生效）。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const h = src.slice(src.indexOf("// 左右方向键跨过内联的片"), src.indexOf("// Enter 直接发送"));
  assert.match(h, /chipBeside\(\{ container: promptEl/, "方向键处理器没有接上判据");
  assert.match(h, /e\.preventDefault\(\)/, "没有阻止默认行为");
  assert.match(h, /to\.collapse\(left\)/, "左键要落到片之前、右键落到片之后");
});

test("片左右要留出间距，否则光标跨过来就贴在片上", () => {
  // 用户：「左键移动过来时候应该往右边空格2下，不然会挨在一起」。
  // 方向键接管之后光标会落在片的紧邻位置，原来左右各只有 1px，光标和片边贴在一起，
  // 看不出自己停在片的哪一侧。
  const css = readFileSync(join(HERE, "..", "src", "styles", "app.css"), "utf8");
  const i = css.indexOf(".composer-chip {");
  const blk = css.slice(i, css.indexOf("}", i));
  const m = blk.match(/margin:\s*0\s+(\d+)px/);
  assert.ok(m, "找不到 .composer-chip 的横向 margin");
  assert.ok(Number(m[1]) >= 4, `片左右间距太小（${m[1]}px），光标跨过来会贴在片上`);
});

test("main.js 真的按落点分工，且复制路径接上了 copyPath", () => {
  // 纯逻辑对了但没接上等于没修。钉三件事：文件树是独立落区、树落点走复制而不是
  // openFolder、以及复制真的调了后端。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /import\s*\{[^}]*planExplorerDrop[^}]*\}\s*from\s*["']\.\/agent\/explorer-drop\.js["']/,
    "main.js 没有引入 explorer-drop 模块");
  assert.match(src, /return "explorer";/,
    "拖放落区里没有「侧栏」这一档——那它还是会走 open 分支去换工作区");
  assert.match(src, /target === "explorer"/,
    "_handleDrop 没有为侧栏落点分出复制分支");
  assert.match(src, /backend\.copyPath\(/,
    "复制分支没有真的调用 backend.copyPath");
  // 没打开项目时（rootPath 为空）文件树落点必须退回 open，否则空状态下拖文件夹进来
  // 会算出空目标、什么都不发生——而那时用户想要的正是「打开这个项目」。
  assert.match(src, /rootPath && _dropPointIn\(p, _explorerEl\)/,
    "落区判据必须是整条侧栏而不是 #tree：Git 视图下 #tree 塌成 0 会退回 open 换掉项目；\n     且 rootPath 为空时不抢落点，那时拖文件夹应该打开项目");
});
