/**
 * 文件路径的规范化与比较。**这是全仓路径逻辑的唯一真相。**
 *
 * 从 main.js 抽出来的第三块。这一簇六个函数被引用近 250 次，却一直和主循环、UI、
 * 计费挤在同一个五兆文件里——而它们是最纯的那种：只吃字符串、只吐字符串/布尔，
 * 没有 DOM、没有 I/O、没有模块级可变状态。
 *
 * 抽出来的直接好处是**它终于能被单独测**：以前想验一条路径比较，得先把 main.js 那份
 * 沙箱搭起来。现在 import 就能跑。
 *
 * 命名去掉了下划线前缀（模块里不需要"这是私有的"这层暗示）；main.js 那边按原名 import，
 * 近 250 个调用点一个都不用改。
 */

// Normalize a filesystem path to forward slashes. Windows dialogs / read_dir return
// `C:\a\b`, but ALL path logic here joins & compares with "/" (rootPath + "/" + rel,
// startsWith(rootPath + "/"), split("/")…). A backslash root then matches NOTHING →
// relative paths don't resolve to the workspace ("不看当前工作目录"). Windows accepts
// "/" in paths, so posix-normalizing at every boundary makes it all consistent.
// No-op on macOS/Linux (they have no backslashes in paths).
export function toPosix(p) { return typeof p === "string" ? p.replace(/\\/g, "/") : p; }

export function normalizeFsPath(path) {
  if (typeof path !== "string") return path;
  // A filesystem name may legally end in whitespace on POSIX. Trimming the whole
  // path turns `/repo/name ` into a different file (`/repo/name`) and defeats the
  // invisible-whitespace recovery used by read_file. Call sites validate empty
  // model arguments separately; path normalization itself must preserve identity.
  let value = toPosix(path);
  if (!value) return value;
  let prefix = "";
  if (/^[A-Za-z]:\//.test(value)) {
    prefix = value.slice(0, 2) + "/";
    value = value.slice(3);
  } else if (value.startsWith("//")) {
    prefix = "//";
    value = value.replace(/^\/+/, "");
  } else if (value.startsWith("/")) {
    prefix = "/";
    value = value.replace(/^\/+/, "");
  }
  const parts = [];
  for (const part of value.split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") {
      if (parts.length && parts[parts.length - 1] !== "..") parts.pop();
      else if (!prefix) parts.push(part);
      continue;
    }
    parts.push(part);
  }
  return prefix + parts.join("/");
}

export function isAbsoluteFsPath(path) {
  const normalized = normalizeFsPath(String(path || ""));
  return !!normalized && (normalized.startsWith("/") || normalized.startsWith("//") || /^[A-Za-z]:\//.test(normalized));
}

/**
 * 比较用的路径身份。大小写敏感与否取决于**文件系统在哪台机器上**。
 *
 * `remote` 从参数传进来，模块自己不去读那个全局——那是 main.js 的连接状态。
 * 这是这一簇唯一一处非纯的地方，所以必须从签名上看得见：不传就按本机判断
 * （navigator 是浏览器真全局，模块里用 typeof 守着，在 node 测试里也安全）。
 * main.js 侧的薄壳把 `_remote` 传进来，近 60 个调用点一个都不用改。
 */
export function pathIdentity(path, remote = null) {
  const normalized = normalizeFsPath(path);
  const remoteActive = !!(remote && remote.active);
  const remoteCaseInsensitive = remoteActive && /windows|darwin|mac(?:os)?/i.test((remote && remote.platform) || "");
  const localCaseInsensitive = !remoteActive && typeof navigator !== "undefined"
    && /Mac|Win/i.test((navigator.platform || "") + " " + (navigator.userAgent || ""));
  return typeof normalized === "string" && (remoteCaseInsensitive || localCaseInsensitive || /^[A-Za-z]:\//.test(normalized) || normalized.startsWith("//"))
    ? normalized.toLowerCase()
    : normalized;
}

export function pathIsAtOrUnder(candidate, parent, remote = null) {
  // remote 要透传下去：不传的话跨机器比较会悄悄退回本机的大小写判据。
  const childIdentity = pathIdentity(candidate, remote);
  const parentIdentity = pathIdentity(parent, remote);
  if (!childIdentity || !parentIdentity) return false;
  return childIdentity === parentIdentity
    || childIdentity.startsWith(parentIdentity.endsWith("/") ? parentIdentity : parentIdentity + "/");
}

// coherentFilePath **不在这里**：它要读编辑器打开的文件表（openFiles / projectModels /
// _openingFiles），那是 main.js 的状态，不是路径逻辑。留在 main.js 里。
// 判据就是这条：模块里只放"给它字符串就能算出答案"的东西。

