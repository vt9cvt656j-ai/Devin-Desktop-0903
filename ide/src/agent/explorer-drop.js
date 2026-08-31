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
 * 树**内部**拖动（把树里的文件/目录拖到另一个目录）的合法性判据。
 *
 * VS Code 的 handleDragOver 里对应这几条 return false：目标就是它自己、目标是它的子目录、
 * 源已经在目标目录里（拖了等于没拖）。前两条要是漏了，`rename` 会把目录搬进它自己，
 * 整棵子树当场消失。
 *
 * 返回空串 = 可以移动；否则是拒绝原因。
 */
export function moveRejection({ src = "", destDir = "" } = {}) {
  const s = trimSlash(src);
  const d = trimSlash(destDir);
  if (!s || !d) return "invalid";
  if (isInsideOrSame(d, s)) return "self";     // 拖进自己 / 自己的子目录
  if (parentOf(s) === d) return "same";        // 本来就在这个目录里
  return "";
}

/**
 * 一批内部移动的计划。paths 是被拖的那些路径（多选时不止一个）。
 * 返回 { moves: [{from, to, name}], skipped: [{path, reason}] }，纯计算不碰磁盘。
 * 同一批里若有两个同名，后一个会被标成 dup —— 后端 rename 在目标已存在时是直接报错的，
 * 与其让它半路失败，不如在计划期就说清楚。
 */
export function planMove({ paths = [], destDir = "" } = {}) {
  const dest = trimSlash(destDir);
  const moves = [];
  const skipped = [];
  const taken = new Set();
  for (const raw of paths) {
    const from = trimSlash(raw);
    const reason = moveRejection({ src: from, destDir: dest });
    if (reason) { skipped.push({ path: from, reason }); continue; }
    const name = baseName(from);
    if (taken.has(name)) { skipped.push({ path: from, reason: "dup" }); continue; }
    taken.add(name);
    moves.push({ from, to: joinPath(dest, name), name });
  }
  return { moves, skipped };
}

/**
 * 折叠嵌套选择：同时选中 `A/` 和 `A/x.txt` 时只保留 `A/`。
 *
 * 删除和移动都必须先过这一步。移动漏了它会**丢文件**：先把 A/ 搬走，再拿已经不存在的
 * A/x.txt 去 rename，整批停在半路，而 A/ 已经动了。
 *
 * treePath / isAtOrUnder 从参数传进来（main.js 侧那两个要读模块级状态），
 * 这样这里仍然是「给它字符串就能算出答案」的纯函数。
 */
export function topLevelOf(paths, { treePath = (p) => p, isAtOrUnder } = {}) {
  const under = isAtOrUnder || ((child, parent) => isInsideOrSame(child, parent));
  const out = [];
  const seen = new Set();
  for (const raw of paths || []) {
    const path = treePath(raw);
    if (!path || seen.has(path)) continue;
    seen.add(path);
    if (out.some((parent) => path !== parent && under(path, parent))) continue;
    for (let i = out.length - 1; i >= 0; i--) {
      if (out[i] !== path && under(out[i], path)) out.splice(i, 1);
    }
    out.push(path);
  }
  return out;
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

/**
 * 「移除」出来的隐藏清单。
 *
 * 注意它**不是删除**：文件仍然在磁盘上，只是不在文件树里显示。右键菜单里的「删除」
 * 才是真删。所以这里全程只跟路径字符串打交道，一次也不碰文件系统。
 *
 * 形状是 { [工作区根]: [被隐藏的绝对路径, ...] } —— 按项目分开，换个项目互不影响。
 */
export function hiddenFor(store, root) {
  const list = store && typeof store === "object" ? store[trimSlash(root)] : null;
  return Array.isArray(list) ? list.map(trimSlash).filter(Boolean) : [];
}

/** 加一条隐藏。返回新的 store（不改原对象）；已经在里面就原样返回。 */
export function addHidden(store, root, path) {
  const r = trimSlash(root);
  const p = trimSlash(path);
  if (!r || !p) return store || {};
  const cur = hiddenFor(store, r);
  if (cur.includes(p)) return store || {};
  return { ...(store || {}), [r]: [...cur, p] };
}

/** 清空某个项目的隐藏清单（「恢复已移除的项」）。 */
export function clearHidden(store, root) {
  const next = { ...(store || {}) };
  delete next[trimSlash(root)];
  return next;
}

/**
 * 这个条目该不该被藏起来。隐藏一个目录时**连同它底下的东西**一起藏——否则展开父目录
 * 时它又冒出来了。同前缀的兄弟（logs / logs2）不受影响。
 */
export function isHidden(hiddenList, path) {
  const p = trimSlash(path);
  if (!p) return false;
  return (hiddenList || []).some((h) => isInsideOrSame(p, h));
}

/** 从存储里读出整份隐藏清单；坏数据一律当空。storage 从参数传，模块本身不碰全局。 */
export function loadHidden(storage, key) {
  try { return JSON.parse(storage?.getItem?.(key) || "{}") || {}; } catch { return {}; }
}
/** 写回；写不进去（隐私模式、配额满）不该炸掉调用方。 */
export function saveHidden(storage, key, store) {
  try { storage?.setItem?.(key, JSON.stringify(store || {})); } catch { /* 存不了就算了 */ }
}

/**
 * 光标紧邻的那一侧是不是一个「片」（composer-chip）。
 *
 * 片是 contentEditable=false 的原子节点，WKWebView 不总能在它两侧摆放光标 —— 片若是输入框
 * 里第一个元素，按左键就"没反应"。两侧垫零宽空格不够（浏览器有时不把空文本节点当落脚点），
 * 所以要自己接管方向键，而"该不该接管"就是这个函数回答的。
 *
 * 纯 DOM 遍历，节点全部从参数进；isChip 也由调用方给，模块里不认 class 名。
 */
export function chipBeside({ container, node, offset = 0, left = true, isChip } = {}) {
  const bare = (t) => String(t || "").replace(/\u200b/g, "");
  if (!container || !node) return null;
  let cur = node;
  if (cur.nodeType === 3) {
    const txt = cur.nodeValue || "";
    // 这一侧还有真字符可走 → 交给浏览器，别抢。
    if (bare(left ? txt.slice(0, offset) : txt.slice(offset))) return null;
  } else if (cur.childNodes && cur.childNodes.length) {
    const i = Math.max(0, Math.min(offset, cur.childNodes.length - 1));
    cur = cur.childNodes[i] || cur;
  }
  let probe = cur;
  while (probe && probe !== container) {
    const sib = left ? probe.previousSibling : probe.nextSibling;
    if (sib) { probe = sib; break; }
    probe = probe.parentNode;
  }
  // 跳过垫在中间的零宽空格，再看过去是不是片。
  while (probe && probe.nodeType === 3 && !bare(probe.nodeValue)) {
    probe = left ? probe.previousSibling : probe.nextSibling;
  }
  return probe && probe.nodeType === 1 && isChip?.(probe) ? probe : null;
}
