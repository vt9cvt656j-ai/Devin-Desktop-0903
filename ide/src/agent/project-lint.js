/*
 * 项目自己的 linter：**认出来它是哪一个、怎么只跑改动的那几个文件、输出怎么读**。
 *
 * 为什么需要它（写时正确性此前的缺口）：
 *   交错验证只有一条腿 —— Monaco 内建 worker 的语法/类型诊断，加上 LSP。那条腿认的是
 *   「这行编译不过」。而用户真正吃亏的一类 bug 在语法上全对、类型也全过：漏 await 的
 *   promise、该用 === 用了 ==、React hook 依赖不全、catch 了异常又吞掉、未使用的变量
 *   背后藏着一个拼错的名字。只有项目自己配的那份 linter 认得，而那份配置就躺在仓库里
 *   没人读。纯 JS 项目最极端：没有类型检查，那条腿只剩语法，等于唯一的写时正确性门
 *   几乎是空的。
 *
 * 三条纪律：
 *   1. **只按项目自己的配置认**，不猜、不塞默认规则。仓库没配 linter 就什么都不跑 ——
 *      拿一份我们选的规则去评判别人的代码，产出的全是噪音。
 *   2. **只报 error，不报 warning**。warning 是风格取向，阻断门里放风格就是把门玩坏。
 *   3. **「没跑成」和「跑了、干净」必须可分**。混为一谈的话，一个没装 eslint 的项目会
 *      得到「零错误」——而那恰恰是最该说一句的情形。
 *
 * 纯函数：不碰 DOM、无模块级可变状态，执行本身由调用方注入。
 */

/** 这次改动里，按扩展名分出的语言组。 */
function _extOf(rel) {
  return (String(rel || "").split(".").pop() || "").toLowerCase();
}

const _JS_EXT = new Set(["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts", "vue", "svelte"]);
const _PY_EXT = new Set(["py", "pyi"]);
const _GO_EXT = new Set(["go"]);

/**
 * 项目配了哪个 linter。
 *
 * `files` 是「仓库根下有哪些文件名」的集合（调用方从已有的项目扫描里拿，不额外读盘），
 * `manifest` 是 package.json 解析后的对象（没有就传 null）。
 *
 * 判据一律是**项目自己的配置文件或依赖声明**，不看有没有装、不看全局有没有这个命令——
 * 那是执行时才知道的事，猜在这里只会让判断和事实脱节。
 */
export function detectLinter(files, manifest = null) {
  const has = (name) => (files instanceof Set ? files.has(name) : Array.isArray(files) && files.includes(name));
  const deps = {
    ...(manifest?.dependencies || {}),
    ...(manifest?.devDependencies || {}),
  };
  const out = [];

  // ── JS/TS ────────────────────────────────────────────────────────────────
  // eslint 的配置可以在 package.json 的 eslintConfig 里，也可以是独立文件（新旧两代命名）。
  const eslintConfigured = !!manifest?.eslintConfig
    || ["eslint.config.js", "eslint.config.mjs", "eslint.config.cjs", "eslint.config.ts",
        ".eslintrc", ".eslintrc.js", ".eslintrc.cjs", ".eslintrc.json", ".eslintrc.yml", ".eslintrc.yaml"]
      .some(has);
  if (eslintConfigured || deps.eslint) {
    out.push({ id: "eslint", program: "npx", langs: _JS_EXT });
  } else if (has("biome.json") || has("biome.jsonc") || deps["@biomejs/biome"]) {
    out.push({ id: "biome", program: "npx", langs: _JS_EXT });
  } else if (has(".oxlintrc.json") || deps.oxlint) {
    out.push({ id: "oxlint", program: "npx", langs: _JS_EXT });
  }

  // ── Python ───────────────────────────────────────────────────────────────
  // ruff 可以配在 ruff.toml / .ruff.toml，也可以只在 pyproject.toml 的 [tool.ruff] 里。
  // 这里只认前两者和显式的 requirements 声明；pyproject 的内容由调用方解析后经
  // `manifest.pyprojectHasRuff` 告知（我们不在这儿写 TOML 解析器）。
  if (has("ruff.toml") || has(".ruff.toml") || manifest?.pyprojectHasRuff) {
    out.push({ id: "ruff", program: "ruff", langs: _PY_EXT });
  }

  // ── Go ───────────────────────────────────────────────────────────────────
  // go vet 是工具链自带的，有 go.mod 就一定能跑，不需要额外配置。
  // 它抓的正是这里要抓的那一类：Printf 参数对不上、结构体标签写错、锁被复制。
  if (has(".golangci.yml") || has(".golangci.yaml") || has(".golangci.toml")) {
    out.push({ id: "golangci-lint", program: "golangci-lint", langs: _GO_EXT });
  } else if (has("go.mod")) {
    out.push({ id: "go-vet", program: "go", langs: _GO_EXT });
  }

  return out;
}

/**
 * 只跑这次改动的那几个文件的命令行。
 *
 * 返回 null 表示「这一批里没有这个 linter 管得着的文件」——调用方据此整个跳过，
 * 而不是跑一次全仓扫描。全仓扫描在交错验证里是不可接受的：几秒预算，几万个文件。
 */
/**
 * 落回仓库相对路径。
 *
 * 调用方给的路径**两种都有**：抓 baseline 那次是模型填的原始 `call.path`（通常相对），
 * 开阻断门那次是 `call._resolvedPath`（一定绝对）。两次给同一个 linter 喂不同形状的
 * 路径，轻则报告里路径不一致、baseline 抵扣对不上，重则像 go vet 那样直接拼出非法参数。
 */
function _relativeTo(root, path) {
  const p = String(path || "").replace(/\\/g, "/");
  const r = String(root || "").replace(/\\/g, "/").replace(/\/+$/, "");
  if (r && p.toLowerCase().startsWith(r.toLowerCase() + "/")) return p.slice(r.length + 1);
  return p;
}

export function lintCommand(linter, relPaths, root = "") {
  const files = (Array.isArray(relPaths) ? relPaths : [])
    .filter((rel) => linter?.langs?.has?.(_extOf(rel)))
    .map((rel) => _relativeTo(root, rel));
  if (!linter || !files.length) return null;
  switch (linter.id) {
    case "eslint":
      // --format json 让输出可解析；--no-error-on-unmatched-pattern 让「这个文件被
      // ignore 了」不至于变成一次失败（那不是代码问题）。
      return { program: "npx", args: ["--no-install", "eslint", "--format", "json",
        "--no-error-on-unmatched-pattern", ...files] };
    case "biome":
      return { program: "npx", args: ["--no-install", "biome", "lint", "--reporter=json", ...files] };
    case "oxlint":
      return { program: "npx", args: ["--no-install", "oxlint", "--format=json", ...files] };
    case "ruff":
      return { program: "ruff", args: ["check", "--output-format", "json", "--quiet", ...files] };
    case "golangci-lint":
      // **刻意不带格式标志。** v1 的 `--out-format json` 在 v2 里已经删掉（实测 2.13.1
      // 报 `unknown flag: --out-format`、退出码 3、stdout 空），而 v2 换成了
      // `--output.json.path`。两个版本都在用，写死任一个都会让另一半用户上恒不可用。
      // 默认文本输出 `file:line:col: message (linter)` 在**两个大版本上都一样**，
      // 而下面的行格式解析本来就认它 —— 少一个会随上游漂的判据。
      return { program: "golangci-lint", args: ["run", ...files] };
    case "go-vet":
      // go vet 吃的是包，不是文件；给它文件所在的目录（去重）。
      //
      // 传进来的可能是**绝对路径**（阻断门那次调用喂的是 `_resolvedPath`）。
      // 直接拼 `./` + 绝对路径会得到 `.//Users/...` 这种非法包模式，go 报
      // `lstat …: no such file or directory`，整条腿在所有 Go 仓库上恒不可用。
      // 所以这里必须先落回仓库相对路径 —— 见 `_relativeTo`。
      return { program: "go", args: ["vet", ...[...new Set(files.map((f) => {
        const at = String(f).lastIndexOf("/");
        return at > 0 ? `./${f.slice(0, at)}/...` : "./...";
      }))]] };
    default:
      return null;
  }
}

/** 把一行/一条诊断归一成同一种形状。 */
function _finding(rel, line, rule, message) {
  return {
    rel: String(rel || ""),
    line: Number(line) || 0,
    rule: String(rule || ""),
    message: String(message || "").replace(/\s+/g, " ").trim().slice(0, 200),
  };
}

/**
 * 解析 linter 的输出，**只取 error 级别**。
 *
 * warning 一律丢掉：那是风格取向，不同项目取向不同，塞进阻断门等于拿别人的口味拦别人的活。
 * 解析不出来时返回空数组，并由调用方按 `ran:false` 处理——猜不出结构就别假装读懂了。
 */
export function parseLintErrors(linterId, stdout) {
  const text = String(stdout || "").trim();
  if (!text) return [];
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    // go vet 不出 JSON，走行格式：path/to/file.go:12:5: message
    if (linterId === "go-vet" || linterId === "golangci-lint") {
      return text.split("\n")
        .map((line) => /^(.+?):(\d+):\d+:\s*(.+)$/.exec(line.trim()))
        .filter(Boolean)
        .map((m) => _finding(m[1], m[2], "vet", m[3]));
    }
    return [];
  }
  const out = [];
  if (linterId === "eslint" && Array.isArray(parsed)) {
    for (const file of parsed) {
      for (const m of file?.messages || []) {
        // severity: 1=warning 2=error。fatal（解析失败）也当 error。
        if (m?.severity !== 2 && !m?.fatal) continue;
        out.push(_finding(file.filePath, m.line, m.ruleId, m.message));
      }
    }
    return out;
  }
  if (linterId === "ruff" && Array.isArray(parsed)) {
    // ruff 的 JSON 每条都是要修的问题（它没有 warning 级别），code 为空的是语法错。
    for (const d of parsed) {
      out.push(_finding(d?.filename, d?.location?.row, d?.code, d?.message));
    }
    return out;
  }
  if (linterId === "biome") {
    for (const d of parsed?.diagnostics || []) {
      if (String(d?.severity || "").toLowerCase() !== "error") continue;
      // **两个大版本的形状不一样，两边都认。** 实测：
      //   2.5.10 → location.path 是**裸字符串**，行号在 location.start.line，正文叫 message
      //   1.9.4  → location.path 是 {file}，location.span 是**字节偏移**二元组，正文叫 description
      // 上一版按 1.x 写、还把 span[0] 当行号（那在 1.x 上也是错的），于是在 2.x 上
      // 每条 error 解析成「空文件名 + 空正文 + 行号 0」，阻断门照开、模型收到一条
      // 定位不到也读不懂的错误；findingKey 还因此塌成「|规则|」，把新引入的真错静默抵扣掉。
      const loc = d?.location || {};
      const file = typeof loc.path === "string" ? loc.path : loc.path?.file;
      out.push(_finding(file, loc.start?.line, d?.category, d?.message ?? d?.description));
    }
    return out;
  }
  if (linterId === "oxlint") {
    for (const d of parsed?.diagnostics || []) {
      if (String(d?.severity || "").toLowerCase() !== "error") continue;
      // span 里 line/column/offset 三个字段都在（实测 oxlint 1.80.0）。
      // 上一版取的是 offset —— 字节偏移，量级是行号的几十倍：小文件里指向不存在的行，
      // 大文件里指向一个**存在但完全无关**的行，而模型被明确要求去那儿修。
      const _span = d?.labels?.[0]?.span || {};
      out.push(_finding(d?.filename, _span.line, d?.code, d?.message));
    }
    return out;
  }
  if (linterId === "golangci-lint") {
    for (const d of parsed?.Issues || []) {
      out.push(_finding(d?.Pos?.Filename, d?.Pos?.Line, d?.FromLinter, d?.Text));
    }
    return out;
  }
  return out;
}

/**
 * 这次 linter **到底跑成了没有**。
 *
 * 这是整个模块最容易出错的地方，而且错了不会有任何征兆：
 *   · `npx --no-install eslint` 在项目没装 eslint 时会**正常启动、非零退出**，
 *     stdout 空、stderr 是一段 npm 的报错。进程跑起来了，所以 Rust 那边没有 note，
 *     解析器拿到空串返回空数组 —— 于是「这个项目没装 lint」被读成「零错误，干干净净」。
 *     用户会以为写时检查在保护他，实际那道门从没开过。
 *   · `go vet` 把诊断写在 **stderr**，stdout 是空的。只读 stdout 的话，它永远报告零问题。
 *
 * 判据分两种，按输出格式定：
 *   · JSON 类：输出必须真的解析成 JSON。解析不了就是没跑成 —— 无论退出码是几。
 *   · 行格式类（go vet / golangci 的纯文本）：退出码 0 就是干净；非零则必须**至少解析出
 *     一条**诊断，否则那个非零是「工具自己出错了」，不是「代码有问题」。
 */
export function lintRan(linterId, out) {
  const stdout = String(out?.stdout || "");
  const stderr = String(out?.stderr || "");
  const code = Number(out?.code);
  if (linterId === "go-vet" || linterId === "golangci-lint") {
    if (code === 0) return true;
    return parseLintErrors(linterId, `${stdout}\n${stderr}`).length > 0;
  }
  // JSON 类：空输出只有在退出码 0 时才可信（有些 linter 干净时什么都不打）。
  const text = stdout.trim();
  if (!text) return code === 0;
  try {
    JSON.parse(text);
    return true;
  } catch {
    return false;
  }
}

/** 诊断可能在 stdout 也可能在 stderr（go vet 只写 stderr）。两边都读，别只读一边。 */
export function lintFindings(linterId, out) {
  const fromOut = parseLintErrors(linterId, out?.stdout);
  if (fromOut.length) return fromOut;
  return parseLintErrors(linterId, out?.stderr);
}

/** 一条 finding 的身份：同一个文件同一条规则同一句话算同一条（行号会随编辑漂移）。 */
export function findingKey(finding) {
  const rel = String(finding?.rel || "").replace(/\\/g, "/").split("/").slice(-3).join("/").toLowerCase();
  return `${rel}|${finding?.rule || ""}|${finding?.message || ""}`;
}

/**
 * 这次改动**新引入**的那些。
 *
 * 和诊断那条腿同一个道理：仓库里本来就有的问题不能算到模型头上，否则门一开就永远
 * 关不上，而模型会被推去改一堆跟本轮任务无关的代码。行号不进身份（编辑会让它漂），
 * 所以「同一个问题挪了两行」不会被当成新的。
 */
export function newFindings(baseline, current) {
  const seen = new Set((Array.isArray(baseline) ? baseline : []).map(findingKey));
  const out = [];
  const emitted = new Set();
  for (const finding of Array.isArray(current) ? current : []) {
    const key = findingKey(finding);
    if (seen.has(key) || emitted.has(key)) continue;
    emitted.add(key);
    out.push(finding);
  }
  return out;
}

/** 给模型看的阻断正文。空数组时返回空串（不发无内容的块）。 */
export function lintReport(linterId, findings) {
  const list = (Array.isArray(findings) ? findings : []).slice(0, 20);
  if (!list.length) return "";
  const lines = list.map((f) => {
    const where = f.line ? `${f.rel}:${f.line}` : f.rel;
    return `· ${where}  ${f.rule ? `[${f.rule}] ` : ""}${f.message}`;
  });
  const more = findings.length > list.length ? `\n（还有 ${findings.length - list.length} 条同类）` : "";
  return `${linterId} 报出这次改动新引入的 ${findings.length} 个错误（项目自己的规则，不是我加的）：\n${lines.join("\n")}${more}`;
}
