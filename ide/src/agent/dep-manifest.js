// 从 src/main.js 抽出。判据是内聚 + 边界干净：这一族对 main.js 其余部分的引用
// 实测为零；调用点全部在族内，或经 import 显式接回。
//
// 本目录下的 import / export 必须写成**单物理行**：test/helpers/source.mjs 拼 SRC 时
// 按行过滤 `^\s*import`，多行写法只删得掉第一行，剩下的那半行会让顶层 acorn.parse
// 当场 SyntaxError —— 十几个测试文件一起在 import 期崩溃，而不是某条断言变红。

// ── 依赖坑前置：新依赖第一次出现在磁盘上那一刻，点名「它还没核对过真实 API」 ────────
//
// 触发点有两个，都是执行事实，都走同一条预填候选形状（下面 _depPitfallNote 是唯一出口）：
//
//   ① 写 manifest —— 见下面这一大段。这是原本唯一的触发点。
//   ② 写源码里的 import —— 因为常态恰恰是**先写 import 再补依赖**：模型直接
//      `import { z } from "zod"` 开写，package.json 是后面某一轮才补的（或者压根忘了补，
//      于是跑起来才炸）。只盯 manifest 的话，"写用法代码之前"这个唯一有用的时机每次都
//      正好被错过——而 ① 那段注释里"写 manifest 那一刻，正是写用法代码之前唯一确定存在
//      的时机"这句话，前提就只在"先声明后使用"的顺序下成立。
//      判据同样是新增行（checkpoint 基线相减，同一套手法）：新增行里出现 import/require/
//      use 语句，引入的是**第三方包名**（不是相对路径、不是内置模块），且这个包不在项目
//      已声明的依赖清单里。判不准就不说（宁可漏报，不许误报）——所以该生态的依赖清单
//      本身必须是已知的（_projectDeclaredDepIndex 返回 null 语义时一个字都不说）。
//
// 触发是执行事实，不是猜测：写工具这次真的往 manifest（package.json / Cargo.toml /
// requirements*.txt / pyproject.toml / go.mod）落了盘，且 checkpoint 基线相减的**新增行**
// 里解析得出依赖条目（与 _stubDeliveryFindings/_duplicateSymbolNote 同一套新增行手法）。
// 训练语料常落后于装进项目的版本，模型凭记忆写 API 是「写完才发现坑」的机器成因；
// 而写 manifest 的那一刻，正是「写用法代码之前」唯一确定存在的时机。
//
// 给的是机制不是劝诫：
//   · 连着 context7（run.mcpToolCache 里有 mcp__context7__query-docs）→ 把一条**参数已
//     预填**的查询挂成候选（run._depDocsCandidate，_verifyCandidate 的同款先例），模型
//     空参数调用那个工具即点头执行；
//   · 没有 context7 → 退为指路事实：registry 页面 URL 按 manifest 类型确定性拼出
//     （npm/crates.io/pypi/pkg.go.dev），模型可用 web_fetch 自行核对。
// 两条路的发起方都永远是模型——IDE 不代跑、不后台抓取（与 verifyNow 同一条红线）。
// 有界：每轮最多 2 条（run._depHintBudget，主循环每轮重置）；同一 (生态,名字,主版本)
// 跨 run 只提示一次（localStorage LRU，上限 64 条）。
function _manifestDepKind(path) {
  const base = String(path || "").split(/[\\/]/).pop() || "";
  if (/^package\.json$/i.test(base)) return "npm";
  if (/^cargo\.toml$/i.test(base)) return "crates";
  if (/^requirements[\w.-]*\.txt$/i.test(base)) return "pypi";
  if (/^pyproject\.toml$/i.test(base)) return "pypi";
  if (/^go\.mod$/i.test(base)) return "go";
  return "";
}

// 确定性映射，不是猜测：manifest 类型 → 该生态的官方 registry 页面。
function _depRegistryUrl(kind, name) {
  if (kind === "npm") return `https://www.npmjs.com/package/${name}`;
  if (kind === "crates") return `https://crates.io/crates/${name}`;
  if (kind === "pypi") return `https://pypi.org/project/${name}/`;
  if (kind === "go") return `https://pkg.go.dev/${name}`;
  return "";
}

// checkpoint 基线相减、只看新增行，从中解析 (名字, 版本)。行文本级判据：版本换行
// （^17→^18）也算「这个版本的条目是新的」——大版本跳变恰恰是坑最密的时刻，缓存按
// (名字,主版本) 去重，同主版本不会重复提示。本地 path/git 依赖不在 registry 上，跳过。
export function _manifestDepAdditions(path, oldText, newText, maxItems = 6) {
  const kind = _manifestDepKind(path);
  if (!kind) return [];
  // 行归一化剥掉尾逗号：往 JSON/TOML 数组已有条目后面追加时，前一行只多出一个 `,`——
  // 那不是新依赖，是标点。不剥的话每次追加都会把邻居误报成新增，白烧每轮预算。
  const _normLine = (l) => String(l).trim().replace(/,\s*$/, "");
  const before = new Set(String(oldText || "").split("\n").map(_normLine));
  const lines = String(newText || "").split("\n");
  const out = [];
  const seenNames = new Set();
  const push = (name, version) => {
    name = String(name || "").trim();
    const ver = String(version || "").trim();
    const lower = name.toLowerCase();
    if (!name || seenNames.has(lower) || out.length >= maxItems) return;
    seenNames.add(lower);
    const digits = /(\d+)/.exec(ver);
    out.push({ kind, name, version: ver, major: digits ? digits[1] : "?", registry: _depRegistryUrl(kind, name) });
  };
  if (kind === "npm") {
    let inDeps = false;
    for (const raw of lines) {
      const t = raw.trim();
      if (!inDeps) {
        if (/^"(?:dependencies|devDependencies|peerDependencies|optionalDependencies)"\s*:\s*\{/.test(t)) inDeps = true;
        continue;
      }
      if (t.startsWith("}")) { inDeps = false; continue; }
      const m = /^"((?:@[\w.-]+\/)?[\w.-]+)"\s*:\s*"([^"]*)"/.exec(t);
      if (m && !before.has(_normLine(t))) push(m[1], m[2]);
    }
  } else if (kind === "crates") {
    let inDepTable = false; // [dependencies] 一族：一行一个依赖
    let entryName = "";     // [dependencies.foo] 一族：整节描述一个依赖
    let entryNew = false;
    let entryVersion = "";
    const flushEntry = () => {
      if (entryName && entryNew) push(entryName, entryVersion);
      entryName = ""; entryNew = false; entryVersion = "";
    };
    for (const raw of lines) {
      const t = raw.trim();
      const isNew = !!t && !before.has(_normLine(t));
      const header = /^\[([^\]]+)\]/.exec(t);
      if (header) {
        flushEntry();
        const sec = header[1].trim();
        const dm = /^(?:workspace\.|target\.[^\]]*?\.)?(?:dev-|build-)?dependencies(?:\.(.+))?$/.exec(sec);
        inDepTable = !!dm && !dm[1];
        if (dm && dm[1]) { entryName = dm[1].replace(/^["']|["']$/g, ""); entryNew = isNew; }
        continue;
      }
      if (entryName) {
        const vm = /^version\s*=\s*["']([^"']+)["']/.exec(t);
        if (vm) { entryVersion = vm[1]; if (isNew) entryNew = true; }
        if (/^(?:path|git)\s*=/.test(t)) entryName = ""; // 本地/git 依赖不在 crates.io 上
        continue;
      }
      if (!inDepTable || !isNew || t.startsWith("#")) continue;
      const m = /^([A-Za-z0-9_-]+)\s*=\s*(.+)$/.exec(t);
      if (!m) continue;
      const rhs = m[2];
      if (/(?:^|[{,\s])(?:path|git)\s*=/.test(rhs)) continue;
      const vm = /^["']([^"']*)["']/.exec(rhs) || /version\s*=\s*["']([^"']*)["']/.exec(rhs);
      push(m[1], vm ? vm[1] : "");
    }
    flushEntry();
  } else if (kind === "pypi") {
    const isPyproject = /^pyproject\.toml$/i.test(String(path || "").split(/[\\/]/).pop() || "");
    // PEP 508 形状的一条 spec（extras 允许，环境标记/注释允许跟在后面）。
    const spec = (text) => {
      const m = /^([A-Za-z0-9][\w.-]*)(?:\[[^\]]*\])?\s*(?:(?:===|==|>=|<=|~=|!=|>|<)\s*([^\s;,#'"]+))?/.exec(String(text || "").trim());
      if (!m) return null;
      const rest = String(text || "").trim().slice(m[0].length);
      if (rest && !/^[\s;,#]/.test(rest)) return null;
      return { name: m[1], version: m[2] || "" };
    };
    if (!isPyproject) {
      for (const raw of lines) {
        const t = raw.trim();
        if (!t || /^[#-]/.test(t) || /:\/\//.test(t) || before.has(_normLine(t))) continue;
        const s = spec(t);
        if (s) push(s.name, s.version);
      }
    } else {
      let section = "";
      let inArray = false;
      for (const raw of lines) {
        const t = raw.trim();
        const isNew = !!t && !before.has(_normLine(t));
        const header = /^\[([^\]]+)\]/.exec(t);
        if (header) { section = header[1].trim(); inArray = false; continue; }
        if (/^tool\.poetry\.(?:dev-)?dependencies$|^tool\.poetry\.group\.[\w.-]+\.dependencies$/.test(section)) {
          if (!isNew) continue;
          const m = /^([A-Za-z0-9][\w.-]*)\s*=\s*(.+)$/.exec(t);
          if (!m || m[1].toLowerCase() === "python") continue;
          const rhs = m[2];
          if (/(?:^|[{,\s])(?:path|git|url)\s*=/.test(rhs)) continue;
          const vm = /^["']([^"']*)["']/.exec(rhs) || /version\s*=\s*["']([^"']*)["']/.exec(rhs);
          push(m[1], vm ? vm[1] : "");
          continue;
        }
        if (inArray) {
          if (isNew) {
            const q = /^["']([^"']+)["']/.exec(t);
            const s = q && spec(q[1]);
            if (s) push(s.name, s.version);
          }
          if (t.includes("]")) inArray = false;
          continue;
        }
        // 依赖数组只认这两处：[project] 的 dependencies=[…] 和 [project.optional-dependencies]
        // 的任意 key=[…]。别处的字符串数组（keywords/classifiers）不是依赖清单。
        const opensDeps = (section === "project" && /^dependencies\s*=\s*\[/.test(t))
          || (section === "project.optional-dependencies" && /^[\w.-]+\s*=\s*\[/.test(t));
        if (opensDeps) {
          if (t.includes("]")) { // 单行数组：整行为新才解析
            if (isNew) for (const q of t.matchAll(/["']([^"']+)["']/g)) { const s = spec(q[1]); if (s) push(s.name, s.version); }
          } else inArray = true;
        }
      }
    }
  } else if (kind === "go") {
    let inReq = false;
    for (const raw of lines) {
      const t = raw.trim();
      const isNew = !!t && !before.has(_normLine(t));
      if (/^require\s*\($/.test(t)) { inReq = true; continue; }
      if (inReq && t.startsWith(")")) { inReq = false; continue; }
      if (/\/\/\s*indirect\b/.test(t)) continue; // go mod tidy 带进来的间接依赖不是模型的选择
      let m = /^require\s+(\S+)\s+(v\S+)/.exec(t);
      if (!m && inReq) m = /^([^\s()]+)\s+(v\S+)/.exec(t);
      if (m && isNew) push(m[1], m[2]);
    }
  }
  return out;
}

// ── 触发点 ②：源码新增行里的 import 引进了一个项目还没声明的第三方包 ──────────────
//
// 内置模块名单与相对路径判断覆盖 JS/TS、Python、Rust、Go 四种源码形态（第五种形态是
// manifest 本身，走上面那条）。判不准就不说是硬纪律：这条提示的价值全在"你正要照记忆
// 写一个没核对过的 API"，误报一次的代价是每次写业务代码都被念一遍，那比漏报糟得多。
const _NODE_BUILTIN_MODULES = new Set([
  "assert", "async_hooks", "buffer", "child_process", "cluster", "console", "constants", "crypto",
  "dgram", "diagnostics_channel", "dns", "domain", "events", "fs", "http", "http2", "https",
  "inspector", "module", "net", "os", "path", "perf_hooks", "process", "punycode", "querystring",
  "readline", "repl", "stream", "string_decoder", "sys", "timers", "tls", "trace_events", "tty",
  "url", "util", "v8", "vm", "wasi", "worker_threads", "zlib",
]);
// Python 标准库的顶层模块。只需要覆盖到"常见到会出现在 import 行里"的程度：没收进来的
// 冷门标准库模块会被当成第三方误报一次——所以宁可多列。
const _PY_STDLIB_MODULES = new Set([
  "__future__", "abc", "argparse", "array", "ast", "asyncio", "base64", "binascii", "bisect",
  "builtins", "bz2", "calendar", "cmath", "cmd", "collections", "colorsys", "concurrent",
  "configparser", "contextlib", "contextvars", "copy", "copyreg", "csv", "ctypes", "curses",
  "dataclasses", "datetime", "decimal", "difflib", "dis", "email", "encodings", "enum", "errno",
  "faulthandler", "filecmp", "fileinput", "fnmatch", "fractions", "ftplib", "functools", "gc",
  "getopt", "getpass", "gettext", "glob", "graphlib", "gzip", "hashlib", "heapq", "hmac", "html",
  "http", "imaplib", "importlib", "inspect", "io", "ipaddress", "itertools", "json", "keyword",
  "linecache", "locale", "logging", "lzma", "mailbox", "math", "mimetypes", "mmap", "multiprocessing",
  "netrc", "numbers", "operator", "os", "pathlib", "pickle", "pickletools", "pkgutil", "platform",
  "plistlib", "poplib", "posixpath", "pprint", "profile", "pstats", "pty", "pwd", "py_compile",
  "queue", "quopri", "random", "re", "readline", "reprlib", "resource", "runpy", "sched", "secrets",
  "select", "selectors", "shelve", "shlex", "shutil", "signal", "site", "smtplib", "socket",
  "socketserver", "sqlite3", "ssl", "stat", "statistics", "string", "stringprep", "struct",
  "subprocess", "symtable", "sys", "sysconfig", "tarfile", "tempfile", "termios", "textwrap",
  "threading", "time", "timeit", "tkinter", "token", "tokenize", "tomllib", "trace", "traceback",
  "tracemalloc", "tty", "types", "typing", "unicodedata", "unittest", "urllib", "uuid", "venv",
  "warnings", "wave", "weakref", "webbrowser", "xml", "xmlrpc", "zipapp", "zipfile", "zipimport",
  "zlib", "zoneinfo",
]);
// Rust：`use` 路径的首段只能是 extern crate 名、crate/self/super，或这几个内置 crate。
const _RUST_BUILTIN_CRATES = new Set(["std", "core", "alloc", "crate", "self", "super", "proc_macro", "test"]);

// 源码文件 → 生态。按扩展名判，认不出就返回空（调用方一个字都不说）。
function _sourceImportKind(path) {
  const base = String(path || "").split(/[\\/]/).pop() || "";
  if (/\.(?:[mc]?[jt]sx?)$/i.test(base)) return "npm";
  if (/\.pyi?$/i.test(base)) return "pypi";
  if (/\.rs$/i.test(base)) return "crates";
  if (/\.go$/i.test(base)) return "go";
  return "";
}

// import 说明符 → npm 包名。相对路径、协议前缀、路径别名（@/、~/、#internal）、内置模块
// 一律返回空。`@/components/Button` 这类别名会被包名正则挡掉：npm 的 scope 不能为空。
function _npmPackageFromSpecifier(spec) {
  const s = String(spec || "").trim();
  if (!s || /^[./#~]/.test(s) || s.includes("://") || /^[A-Za-z]:[\\/]/.test(s)) return "";
  if (/^[a-z][a-z0-9+.-]*:/i.test(s)) return ""; // node: / bun: / data: / file: …
  const parts = s.split("/");
  const name = s.startsWith("@") ? parts.slice(0, 2).join("/") : parts[0];
  if (!/^(?:@[a-z0-9][\w.-]*\/)?[a-z0-9][\w.-]*$/i.test(name)) return "";
  if (_NODE_BUILTIN_MODULES.has(name.toLowerCase())) return "";
  return name;
}

// registry 直链只在**源码里的标识符就是 registry 上的标识符**时才给。
//   · npm：包名逐字相同，给。
//   · go：pkg.go.dev 认完整 import path，给。
//   · pypi / crates：不给 —— `import yaml` 的发行名是 PyYAML、`use tokio_util` 的 crate 名
//     是 tokio-util，从源码标识符推 registry 名不是确定性映射。拼一个大概率 404 的 URL 是
//     "自信地给错答案"，正是"判不准就不说"要禁的那种。这两种改走工具路线（package_source
//     按 import 名读本机装的那一份；package_search 按名字在注册表里搜）。
function _importRegistryUrl(kind, name) {
  if (kind === "npm" || kind === "go") return _depRegistryUrl(kind, name);
  return "";
}

// 项目**已声明**的依赖索引：`${生态}:${小写名}`，外加每种已存在 manifest 的裸前缀
// `${生态}:`（"这个生态的清单我们读到过"）。两个来源都是执行事实：
//   · run.stack.declaredDeps —— 工作区扫描真读到的那几份 manifest（_declaredDepsFromFileMap）；
//   · run._declaredDeps —— 本 run 里模型自己往 manifest 写进去的（全新项目那份就是它写的）。
function _projectDeclaredDepIndex(run) {
  const idx = new Set();
  const add = (k) => {
    const s = String(k || "").trim().toLowerCase();
    if (s.includes(":")) idx.add(s);
  };
  const fromStack = run?.stack?.declaredDeps;
  if (Array.isArray(fromStack)) for (const k of fromStack) add(k);
  if (run?._declaredDeps instanceof Set) for (const k of run._declaredDeps) add(k);
  return idx;
}

// 从工作区扫描读到的 manifest 原文里抽出已声明依赖。复用 _manifestDepAdditions（基线传空
// ⇒ 每一条依赖行都算"新增"），不另写第二份解析器。
export function _declaredDepsFromFileMap(fileMap) {
  const out = new Set();
  if (!fileMap || typeof fileMap !== "object") return [];
  for (const [name, text] of Object.entries(fileMap)) {
    if (typeof text !== "string" || !text.trim() || text === "[present]") continue;
    const kind = _manifestDepKind(name);
    if (!kind) continue;
    out.add(`${kind}:`);
    for (const d of _manifestDepAdditions(name, "", text, 400)) out.add(`${kind}:${d.name.toLowerCase()}`);
    // go.mod 的 module 行：本模块自己的 import path 前缀。没有它，`import "github.com/me/app/db"`
    // （首段带点 ⇒ 看着像第三方）会被当成外部依赖误报。
    if (kind === "go") {
      const m = /^\s*module\s+(\S+)/m.exec(text);
      if (m) out.add(`go:${m[1].toLowerCase()}`);
    }
  }
  return [...out];
}

// 新增行里 import 进来、却不在已声明依赖清单里的第三方包。零网络：纯字符串判断。
export function _undeclaredImportAdditions(run, path, oldText, newText, maxItems = 6) {
  const kind = _sourceImportKind(path);
  if (!kind) return [];
  const declared = _projectDeclaredDepIndex(run);
  // 该生态的依赖清单没读到过 ⇒ "在不在依赖里"这个问题答不了 ⇒ 一个字都不说。
  let kindKnown = false;
  for (const key of declared) { if (key.startsWith(kind + ":")) { kindKnown = true; break; } }
  if (!kindKnown) return [];
  const norm = (l) => String(l).trim();
  const before = new Set(String(oldText || "").split("\n").map(norm));
  const out = [];
  const seen = new Set();
  const isDeclared = (name) => {
    const lower = name.toLowerCase();
    if (declared.has(`${kind}:${lower}`)) return true;
    // go：import path 落在某个已声明 module path 之下就算已声明（本模块自己的包也走这条）。
    if (kind !== "go") return false;
    for (const key of declared) {
      if (!key.startsWith("go:")) continue;
      const mod = key.slice(3);
      if (mod && (lower === mod || lower.startsWith(mod + "/"))) return true;
    }
    return false;
  };
  const push = (name) => {
    name = String(name || "").trim();
    const lower = name.toLowerCase();
    if (!name || seen.has(lower) || out.length >= maxItems) return;
    seen.add(lower);
    if (isDeclared(name)) return;
    out.push({ kind, name, version: "", major: "?", registry: _importRegistryUrl(kind, name), viaImport: true });
  };

  let goInImportBlock = false;
  for (const raw of String(newText || "").split("\n")) {
    const t = norm(raw);
    if (!t) continue;
    if (kind === "go") {
      // import 块的边界要跟着走完整份文件，否则新增行落在块里也认不出来。
      if (/^import\s*\($/.test(t)) { goInImportBlock = true; continue; }
      if (goInImportBlock && t.startsWith(")")) { goInImportBlock = false; continue; }
    }
    if (before.has(t)) continue; // 不是这次新增的行
    if (kind === "npm") {
      if (/^\s*(?:\/\/|\/\*|\*)/.test(t)) continue;
      const specs = [];
      const bare = /^import\s+["'`]([^"'`]+)["'`]/.exec(t);
      if (bare) specs.push(bare[1]);
      for (const m of t.matchAll(/\b(?:import|export)\b[^;]*?\bfrom\s*["'`]([^"'`]+)["'`]/g)) specs.push(m[1]);
      for (const m of t.matchAll(/\b(?:require|import)\s*\(\s*["'`]([^"'`]+)["'`]\s*\)/g)) specs.push(m[1]);
      for (const spec of specs) {
        const name = _npmPackageFromSpecifier(spec);
        if (name) push(name);
      }
    } else if (kind === "pypi") {
      if (t.startsWith("#")) continue;
      const mf = /^from\s+([.\w]+)\s+import\b/.exec(t);
      if (mf) {
        if (mf[1].startsWith(".")) continue; // 相对导入
        const top = mf[1].split(".")[0];
        if (top && !_PY_STDLIB_MODULES.has(top.toLowerCase())) push(top);
        continue;
      }
      const mi = /^import\s+(.+)$/.exec(t);
      if (!mi) continue;
      for (const piece of mi[1].split(",")) {
        const mod = piece.trim().split(/\s+as\s+/)[0].trim();
        if (!mod || mod.startsWith(".")) continue;
        const top = mod.split(".")[0];
        if (/^[A-Za-z_]\w*$/.test(top) && !_PY_STDLIB_MODULES.has(top.toLowerCase())) push(top);
      }
    } else if (kind === "crates") {
      if (t.startsWith("//")) continue;
      // Rust 2018 起，`use` 路径的首段只能是 extern crate 名（本 crate 的模块必须写
      // crate::/self::/super::），所以裸首段就是外部 crate。2015 edition 的老代码里
      // 裸首段也可能是本地模块——那种会误报，代价由下面"已声明依赖"那道过滤兜住。
      const mu = /^(?:pub\s+)?use\s+([A-Za-z_]\w*)\s*(?:::|;|\s+as\b)/.exec(t);
      const me = /^(?:pub\s+)?extern\s+crate\s+([A-Za-z_]\w*)/.exec(t);
      const name = (mu && mu[1]) || (me && me[1]) || "";
      if (name && !_RUST_BUILTIN_CRATES.has(name)) push(name);
    } else if (kind === "go") {
      if (t.startsWith("//")) continue;
      const single = /^import\s+(?:[\w.]+\s+)?["`]([^"`]+)["`]/.exec(t);
      const inBlock = goInImportBlock ? /^(?:[\w.]+\s+)?["`]([^"`]+)["`]/.exec(t) : null;
      const spec = (single && single[1]) || (inBlock && inBlock[1]) || "";
      // 标准库的 import path 首段不含点（fmt / net/http / encoding/json）；第三方一定含域名。
      if (spec && spec.split("/")[0].includes(".")) push(spec);
    }
  }
  return out;
}
