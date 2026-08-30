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
  dropDirFor, planExplorerDrop,
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
    _dropPointIn, _composerEl: composer, _treeEl: tree, rootPath: root,
    window: { devicePixelRatio: 1 },
  });
  const open = mk("/w");
  assert.equal(open({ position: { x: 100, y: 550 } }), "composer", "落在输入框上是引用到对话");
  assert.equal(open({ position: { x: 100, y: 200 } }), "tree", "落在文件树上要走复制，不是换工作区");
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

test("main.js 真的按落点分工，且复制路径接上了 copyPath", () => {
  // 纯逻辑对了但没接上等于没修。钉三件事：文件树是独立落区、树落点走复制而不是
  // openFolder、以及复制真的调了后端。
  const src = readFileSync(join(HERE, "..", "src", "main.js"), "utf8");
  assert.match(src, /import\s*\{[^}]*planExplorerDrop[^}]*\}\s*from\s*["']\.\/agent\/explorer-drop\.js["']/,
    "main.js 没有引入 explorer-drop 模块");
  assert.match(src, /return "tree";/,
    "拖放落区里没有「文件树」这一档——那它还是会走 open 分支去换工作区");
  assert.match(src, /target === "tree"/,
    "_handleDrop 没有为文件树落点分出复制分支");
  assert.match(src, /backend\.copyPath\(/,
    "复制分支没有真的调用 backend.copyPath");
  // 没打开项目时（rootPath 为空）文件树落点必须退回 open，否则空状态下拖文件夹进来
  // 会算出空目标、什么都不发生——而那时用户想要的正是「打开这个项目」。
  assert.match(src, /rootPath && _dropPointIn\(p, _treeEl\)/,
    "空工作区时文件树不该抢走落点——那时拖文件夹应该打开项目");
});
