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
  dropDirFor, planExplorerDrop, moveRejection, planMove,
} from "../src/agent/explorer-drop.js";
import { load } from "./helpers/source.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));

// main.js 那三段是 DOM 绑定的（getBoundingClientRect / elementFromPoint），搬不进模块，
// 但可以用 load() 注入假 DOM **真跑**——比 assert.match(SRC,…) 强：源码断言证明不了
// 「落点算得对不对」。下面这些造的都是最小假件。
const rect = (left, top, right, bottom) => ({ left, top, right, bottom, width: right - left, height: bottom - top });
const elWithRect = (r) => ({ getBoundingClientRect: () => r });
const _dropPointIn = load("_dropPointIn", { window: { devicePixelRatio: 1 } });

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

test("落区命中：物理像素和逻辑像素两套坐标都要认", () => {
  // Tauri 报的坐标可能是物理像素（HiDPI 上 = 逻辑像素 × dpr），浏览器路径给的是 client px。
  // 只认一套的话，Retina 上整个投放区会偏到一半位置去。
  const el = elWithRect(rect(0, 0, 200, 100));
  assert.deepEqual(_dropPointIn({ x: 50, y: 50 }, el), { x: 50, y: 50 }, "逻辑像素要命中");
  assert.equal(_dropPointIn({ x: 300, y: 50 }, el), null, "框外不该命中");
  assert.equal(_dropPointIn({ x: 50, y: 50 }, null), null, "元素不存在时不该炸");

  // dpr=2：物理坐标 (100,100) 落在逻辑 (50,50) → 命中，且**还回逻辑坐标**，
  // 因为后面 elementFromPoint 要的是 client 空间。
  const hi = load("_dropPointIn", { window: { devicePixelRatio: 2 } });
  assert.deepEqual(hi({ x: 100, y: 100 }, el), { x: 50, y: 50 },
    "HiDPI 下必须换算成逻辑坐标再还回去，否则命中测试和 elementFromPoint 对不上");
});

test("三个落区的分工，以及空工作区时不抢文件树", () => {
  const composer = elWithRect(rect(0, 500, 400, 600));
  const tree = elWithRect(rect(0, 0, 200, 400));
  const mk = (root) => load("_dragTargetAt", {
    _dropPointIn, _composerEl: composer, _explorerEl: tree, rootPath: root,
    window: { devicePixelRatio: 1 },
  });
  const open = mk("/w");
  assert.equal(open({ position: { x: 100, y: 550 } }), "composer", "落在输入框上是引用到对话");
  assert.equal(open({ position: { x: 100, y: 200 } }), "explorer", "落在侧栏上要走复制，不是换工作区");
  assert.equal(open({ position: { x: 800, y: 200 } }), "open", "落在编辑器区仍然是打开/换项目");
  // 没打开项目时树里是空状态：这时拖文件夹进来，用户要的是「打开这个项目」。
  assert.equal(mk("")({ position: { x: 100, y: 200 } }), "open",
    "空工作区时文件树不该抢落点——否则拖文件夹进来会算出空目标，什么都不发生");
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
    window: { devicePixelRatio: 1 },
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

test("浏览器路径的坐标要乘回 dpr，否则 Retina 上命中错行", () => {
  // _dropPointIn 先按物理像素试（Tauri 那条给的就是 PhysicalPosition）。浏览器路径给的却是
  // CSS 像素，dpr=2 时 (130,400) 除完变成 (65,200) —— 而侧栏又窄又高，除完**仍然落在框内**，
  // 于是分档答案对、坐标偏了半屏，行高亮会命中光标上方好几行。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  // 只看拖放那一段：树内部的 _rowDragCandidate 也存 clientX，但它不走 _dropPointIn。
  const zone = src.slice(src.indexOf("// Browser / non-Tauri path"), src.indexOf("tauri://drag-enter"));
  const m = zone.match(/position: \{ x: e\.clientX[^}]*\}/g) || [];
  assert.ok(m.length >= 2, "找不到浏览器路径的坐标构造——测试台锚点失效了");
  for (const line of m) {
    assert.match(line, /devicePixelRatio|_dpr/,
      `浏览器路径的坐标没有乘回 dpr：${line}`);
  }
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

test("文件夹落在项目根上要先问，而不是默默复制", () => {
  // VS Code 在这一刻弹框："Do you want to copy 'X' or add 'X' as a folder to the workspace?"
  // 我们把次选项换成「打开为新项目」——用户原来靠拖到侧栏换项目，改成复制后那条路没了。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  const blk = src.slice(src.indexOf("async function _copyIntoWorkspace"), src.indexOf("async function _handleDrop"));
  // 判据要和 VS Code 一样是「任意工作区根」（它的条件就是 e.isRoot）：多根工作区里
  // 往**非活动根**上放文件夹，只认 rootPath 的话会静默复制，问都不问。
  assert.match(blk, /workspaceRoots\.includes\(destDir\)/,
    "根落框只认活动根——多根工作区里往另一个根上放文件夹会被静默复制");
  assert.match(blk, /alt2Label: "打开为新项目"/, "用户丢掉的「替换整个项目」没有还回来");
  assert.match(blk, /pick === "alt2"[\s\S]{0,220}openFolder/, "选了「打开为新项目」没有真的去打开");
  assert.match(blk, /pick === "alt"[\s\S]{0,160}_addWorkspaceRoot/, "「添加到工作区」没有接上多根");
  assert.match(blk, /pick === "cancel"/, "取消必须什么都不做");
  // 多个文件夹时不能只处理第一个、把其余静默丢掉。
  assert.match(blk, /_dirs\.slice\(1\)[\s\S]{0,90}_addWorkspaceRoot/,
    "选了「打开为新项目」时，多余的文件夹被静默丢弃了");
  assert.match(blk, /_dirs = items\.filter\(\(x\) => x\.isDir\)/, "问的条件必须是「拖的是文件夹」");
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
  assert.match(drag.slice(0, 3000), /_paintDropRow\(onComposer \? "" : _treeDropDirAt\(/,
    "拖动中没有高亮将要接收它的那个目录");
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
  assert.match(src, /classList\.add\("io-confirm-actions--stack"\)/, "竖排类没有被挂上去");
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
