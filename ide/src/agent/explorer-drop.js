// 把 OS 文件/文件夹拖进**文件树**时的落点计算。
//
// 之前拖到侧栏一律走「打开」：文件夹 → openFolder() 直接换掉工作区（用户看到的就是
// 「怎么把我项目重新打开了」），文件 → 在编辑器里打开。VS Code 的分工不是这样的：
//   · 拖到资源管理器  → **复制进工作区**，成为子文件 / 子目录
//   · 拖到编辑器区    → 打开（文件夹才当成"换项目"）
// 这个模块只放能纯算的那部分：目标目录解析、重名让路、自我拖入防护、整批计划。
// DOM 命中测试（光标下面是哪一行）留在 main.js，模块里不碰 DOM、不留可变状态。
//
// 路径一律按 POSIX 处理：main.js 侧的 _toPosix 已经把 Windows 的反斜杠归一过了。

const trimSlash = (p) => String(p || "").replace(/\/+$/, "");

export function baseName(p) {
  const s = trimSlash(p);
  const i = s.lastIndexOf("/");
  return i >= 0 ? s.slice(i + 1) : s;
}

export function parentOf(p) {
  const s = trimSlash(p);
  const i = s.lastIndexOf("/");
  if (i > 0) return s.slice(0, i);
  return i === 0 ? "/" : "";
}

export function joinPath(dir, name) {
  const d = trimSlash(dir);
  return (d === "/" ? "" : d) + "/" + name;
}

// `a/b` 在 `a` 里面（或就是它自己）。用来挡住「把文件夹拖进它自己/它的子目录」——
// 那会让递归复制一边读一边写自己，无限长出嵌套目录，是这个功能最危险的一种输入。
export function isInsideOrSame(child, parent) {
  const c = trimSlash(child);
  const p = trimSlash(parent);
  if (!c || !p) return false;
  return c === p || c.startsWith(p + "/");
}

// 扩展名只对文件拆，且只认最后一个点。开头的点不算扩展名，否则 `.gitignore` 会被
// 拆成空主名 + `.gitignore`，让路后变成 ` 2.gitignore`。
export function splitExt(name, isDir = false) {
  const n = String(name || "");
  if (isDir) return { stem: n, ext: "" };
  const i = n.lastIndexOf(".");
  return i > 0 ? { stem: n.slice(0, i), ext: n.slice(i) } : { stem: n, ext: "" };
}

// 重名让路，跟 Finder 一个写法：`notes.txt` → `notes 2.txt`，文件夹 `src` → `src 2`。
// 后端 copy_path 在目标已存在时是直接报错的，所以这一步必须在调用之前算好。
export function uniqueName(name, existing, isDir = false) {
  const taken = existing instanceof Set ? existing : new Set(existing || []);
  if (!taken.has(name)) return name;
  const { stem, ext } = splitExt(name, isDir);
  for (let n = 2; n < 10_000; n++) {
    const candidate = `${stem} ${n}${ext}`;
    if (!taken.has(candidate)) return candidate;
  }
  return `${stem} ${stem.length}${ext}`;
}

// 光标停在哪一行 → 往哪个目录里放。停在文件夹行=进这个文件夹；停在文件行=进它所在的
// 目录（VS Code 就是这样，落在文件上不会变成"放进文件里"）；空白处 = 工作区根。
export function dropDirFor({ rowPath = "", rowIsDir = false, rootPath = "" } = {}) {
  if (!rowPath) return trimSlash(rootPath);
  return rowIsDir ? trimSlash(rowPath) : parentOf(rowPath);
}

/**
 * 拖动**过程中**该给用户看什么：这一下的后果是什么、要高亮哪一行、标签写什么。
 *
 * 纯函数——不碰 DOM，调用方拿着结果去贴 class / 摆标签。单独拎出来是因为这段判断最容易
 * 错，而错了用户就会像这次一样「分不清会发生什么」，所以它必须能在 Node 里逐条断言。
 *
 * 入参：
 *   zone      —— "composer" | "explorer" | "open"（光标落在哪块区域）
 *   destDir   —— explorer 档算出的目标目录（落在文件行时已换成它的父目录）
 *   rootPath  —— 当前活动工作区根；空串 = 没打开项目
 *   items     —— [{ path, isDir }]，拖进来的东西。drag-enter 带 paths、over 不带，
 *                所以调用方要把 enter 那一帧的 paths 存下来复用。
 *
 * 返回 { kind, rowPath, title, sub }：
 *   kind      —— "copy" | "replace" | "openFile" | "ref" | "deny"，决定用哪套视觉
 *   rowPath   —— 要高亮的那一行（空串 = 没有行可高亮）
 *   title/sub —— 标签正文与副行；副行专门用来写**损失**（会关掉哪个项目）
 */
export function dropFeedback({ zone = "", destDir = "", rootPath = "", items = [] } = {}) {
  const list = Array.isArray(items) ? items.filter((x) => x && x.path) : [];
  const n = list.length;
  const many = n > 1 ? `${n} 项` : (n === 1 ? baseName(list[0].path) : "");
  if (zone === "composer") {
    return { kind: "ref", rowPath: "", title: n ? `引用到对话：${many}` : "引用到对话", sub: "只发给 AI，不写入磁盘" };
  }
  if (zone === "explorer") {
    const dest = trimSlash(destDir);
    if (!dest) return { kind: "deny", rowPath: "", title: "没有打开的项目", sub: "先打开一个文件夹" };
    // 把文件夹拖进它自己/它的子目录：递归复制会自噬，落地一定被拒——**拖动时**就说清楚，
    // 别等松手了才弹一句"复制失败"。
    const self = list.find((x) => x.isDir && isInsideOrSame(dest, x.path));
    if (self) return { kind: "deny", rowPath: dest, title: "不能放进它自己里面", sub: baseName(self.path) };
    const where = dest === trimSlash(rootPath) ? `${baseName(rootPath) || "项目"}（项目根）` : baseName(dest);
    return { kind: "copy", rowPath: dest, title: `复制到 ${where}`, sub: many };
  }
  // zone === "open"：文件 = 开个标签（无损）；文件夹 = 换掉整个工作区（有损，必须写明损失）。
  const anyDir = list.some((x) => x.isDir);
  if (!anyDir && n) return { kind: "openFile", rowPath: "", title: `在编辑器中打开：${many}`, sub: "" };
  const cur = baseName(rootPath);
  if (!cur) return { kind: "openFile", rowPath: "", title: n ? `打开项目：${many}` : "打开项目", sub: "" };
  return { kind: "replace", rowPath: "", title: n ? `打开为新项目：${many}` : "打开为新项目", sub: `当前「${cur}」会被关闭` };
}

/**
 * 整批投放计划：给定拖进来的若干路径和目标目录，算出「从哪儿复制到哪儿」。
 *
 * items: [{ path, isDir }]，existingNames: 目标目录里已有的名字。
 * 返回 { copies: [{from, to, name, renamed}], skipped: [{path, reason}] }。
 * 纯函数——不碰磁盘，调用方拿着 copies 逐条调 copyPath。
 */
export function planExplorerDrop({ items = [], destDir = "", existingNames = [] } = {}) {
  const dest = trimSlash(destDir);
  const copies = [];
  const skipped = [];
  if (!dest) return { copies, skipped };
  // 同一批里两个同名文件也要互相让路，所以 taken 是边算边长的。
  const taken = new Set(existingNames || []);
  for (const item of items) {
    const from = trimSlash(item?.path);
    const name = baseName(from);
    if (!from || !name) { skipped.push({ path: String(item?.path || ""), reason: "invalid" }); continue; }
    // 拖的是文件夹，而目标就在它自己里面 → 递归复制会自噬，直接拒。
    if (isInsideOrSame(dest, from)) { skipped.push({ path: from, reason: "self" }); continue; }
    const finalName = uniqueName(name, taken, !!item?.isDir);
    taken.add(finalName);
    copies.push({ from, to: joinPath(dest, finalName), name: finalName, renamed: finalName !== name });
  }
  return { copies, skipped };
}
