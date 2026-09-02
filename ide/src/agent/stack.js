/**
 * 项目栈探测：一张表 + 三处派生判据 + 一段给模型的栈提示。
 *
 * 从 main.js 抽出来的第六块。判据和前几块一样，而且是这一批里最干净的：
 * AST 实测七个声明的**组外自由变量为零**——不是"依赖少"，是没有。
 * extractStackHints 吃的是一张已经读好的文件内容表（fileMap），不碰磁盘、不碰 DOM；
 * formatStackHint 是纯字符串拼接，连 t() 都不调。
 *
 * **_applyUserStackOverride 不在这里**，它留在 main.js：那个函数要读
 * _userCapabilities()（用户的实时声明），是状态不是逻辑。这条边界和 paths.js 把
 * coherentFilePath 退回 main.js 是同一条：模块里只放"给它参数就能算出答案"的东西。
 * 调用点仍然是 _applyUserStackOverride(extractStackHints(fileMap))，顺序不变。
 *
 * 命名去掉了下划线前缀（模块里不需要"这是私有的"这层暗示）；main.js 那边按原名
 * import，八个调用点一个都不用改。
 */

// Parse the workspace's key files (package.json, Cargo.toml, etc.) into a compact
// stack summary the model sees prominently at the top of the context block. Much
// more reliable than hoping the model derives "this is Next.js, run `npm test`"
// from raw JSON. The same info is cached for the agent loop's auto-test.
// 项目栈识别表。**唯一**一份「哪些文件是依赖清单」的名单——三处判据全部从它派生。
//
// 为什么要有这张表：漏掉一类构建描述文件，代价不是"少认一个栈"，是整块栈提示一个字都
// 不出（lang 为空 → formatStackHint 以前直接返回 ""），模型连一行「怎么跑测试」都收不到，
// 只能靠猜，或者干脆一条验证命令都不跑就交付。而这份名单此前在三个地方各写了一遍、
// 互相漂开：读取清单 12 条、"改它就是在选技术栈" 15 条、"清单只算文档不算源码" 13 条。
// 同一个 pubspec.yaml，在一处算依赖清单、在另一处不算——这种分叉最难查。
//
// bespoke: true 的那几行由下面的专用分支解析（要读 JSON、按依赖嗅框架、算复杂度）；
// 表里仍然列出，因为三份名单要从这里派生。其余的整条由表驱动：**加一门语言 = 加一行**，
// 不用改代码、不用等发版（见 [[config-over-hardcoding]]）。用户自己那一层再往上叠：
// _userCapabilities().stack 能覆盖任何一个字段——公司内部的构建包装器（`./bin/ci check`）、
// monorepo 里某个子包的专属命令，永远不可能进产品自带的表。
//
// guessed：这条命令是**猜的默认值**，没有任何项目文件声明过它，也没验证装没装。要如实
// 标给模型（退出 127 时它才知道该换命令，而不是当成代码错误），并被验证管线过滤掉。
export const stackTable = [
  { manifest: "package.json", lang: "JS/TS", bespoke: true },
  { manifest: "Cargo.toml", lang: "Rust", bespoke: true },
  { manifest: "pyproject.toml", lang: "Python", bespoke: true },
  { manifest: "requirements.txt", lang: "Python", bespoke: true },
  { manifest: "go.mod", lang: "Go", bespoke: true },
  // 读它是为了拿 `test:` 目标，但它是构建逻辑、不是依赖清单：改 Makefile 不等于在选技术栈，
  // 读 Makefile 也不等于"只读了一份依赖描述"。notManifest 就是这个区别。
  { manifest: "Makefile", bespoke: true, notManifest: true },
  { manifest: "pom.xml", lang: "Java", bespoke: true },
  { manifest: "build.gradle", lang: "JVM", bespoke: true },
  { manifest: "build.gradle.kts", lang: "JVM", bespoke: true },
  { manifest: "composer.json", lang: "PHP", bespoke: true },
  { manifest: "Gemfile", lang: "Ruby", bespoke: true },
  { manifest: "README.md", doc: true, notManifest: true },

  // ↓ 以下没有专用分支，整条由这张表驱动。
  { manifest: "deno.json", lang: "Deno", pkgMgr: "deno",
    cmds: { testCmd: "deno test -A", checkCmd: "deno check .", formatCmd: "deno fmt", lintCmd: "deno lint" } },
  { manifest: "deno.jsonc", lang: "Deno", pkgMgr: "deno",
    cmds: { testCmd: "deno test -A", checkCmd: "deno check .", formatCmd: "deno fmt", lintCmd: "deno lint" } },
  { manifest: "mix.exs", lang: "Elixir", pkgMgr: "mix",
    cmds: { testCmd: "mix test", checkCmd: "mix compile --warnings-as-errors", buildCmd: "mix compile", formatCmd: "mix format" },
    frameworks: [[/phoenix/i, "Phoenix"]] },
  { manifest: "pubspec.yaml", lang: "Dart", pkgMgr: "pub",
    cmds: { testCmd: "dart test", checkCmd: "dart analyze", buildCmd: "dart compile exe .", formatCmd: "dart format ." },
    frameworks: [[/^\s*flutter\s*:/m, "Flutter"]],
    // Flutter 项目的命令是另一套，嗅到就整组换掉。
    whenFramework: { Flutter: { testCmd: "flutter test", checkCmd: "flutter analyze", buildCmd: "flutter build", pkgMgr: "flutter pub" } } },
  { manifest: "Package.swift", lang: "Swift", pkgMgr: "SwiftPM",
    cmds: { testCmd: "swift test", checkCmd: "swift build", buildCmd: "swift build -c release" } },
  { manifest: "build.zig", lang: "Zig", pkgMgr: "zig",
    cmds: { testCmd: "zig build test", checkCmd: "zig build", buildCmd: "zig build -Doptimize=ReleaseSafe", formatCmd: "zig fmt ." } },
  { manifest: "build.sbt", lang: "Scala", pkgMgr: "sbt",
    cmds: { testCmd: "sbt test", checkCmd: "sbt compile", buildCmd: "sbt package" } },
  { manifest: "deps.edn", lang: "Clojure", pkgMgr: "deps.edn",
    cmds: { testCmd: "clojure -M:test", checkCmd: "clojure -M -e nil" }, guessed: ["clojure -M:test"] },
  { manifest: "stack.yaml", lang: "Haskell", pkgMgr: "stack",
    cmds: { testCmd: "stack test", checkCmd: "stack build --fast", buildCmd: "stack build" } },
  { manifest: "cabal.project", lang: "Haskell", pkgMgr: "cabal",
    cmds: { testCmd: "cabal test", checkCmd: "cabal build", buildCmd: "cabal build" } },
  { manifest: "CMakeLists.txt", lang: "C/C++", pkgMgr: "CMake",
    cmds: { checkCmd: "cmake --build build", buildCmd: "cmake --build build", testCmd: "ctest --test-dir build" },
    guessed: ["cmake --build build", "ctest --test-dir build"] },
  { manifest: "meson.build", lang: "C/C++", pkgMgr: "Meson",
    cmds: { checkCmd: "meson compile -C build", buildCmd: "meson compile -C build", testCmd: "meson test -C build" },
    guessed: ["meson compile -C build", "meson test -C build"] },
  { manifest: "Gopkg.toml", lang: "Go", pkgMgr: "dep" },
  { manifest: "Pipfile", lang: "Python", pkgMgr: "pipenv",
    cmds: { testCmd: "pipenv run pytest", checkCmd: "pipenv run python -m compileall -q ." }, guessed: ["pipenv run pytest"] },
  { manifest: "Podfile", lang: "Swift/ObjC", pkgMgr: "CocoaPods" },

  // 按扩展名认的（工程文件名跟着项目名走，没有固定文件名）。这一族要多一次根目录列举，
  // 命中的内容以扩展名为键放进 fileMap。
  { ext: ".csproj", lang: "C#", pkgMgr: "NuGet",
    cmds: { testCmd: "dotnet test", checkCmd: "dotnet build", buildCmd: "dotnet build -c Release", formatCmd: "dotnet format" } },
  { ext: ".fsproj", lang: "F#", pkgMgr: "NuGet",
    cmds: { testCmd: "dotnet test", checkCmd: "dotnet build", buildCmd: "dotnet build -c Release" } },
  { ext: ".sln", lang: "C#", pkgMgr: "NuGet",
    cmds: { testCmd: "dotnet test", checkCmd: "dotnet build" } },
  { ext: ".cabal", lang: "Haskell", pkgMgr: "cabal",
    cmds: { testCmd: "cabal test", checkCmd: "cabal build" } },
  { ext: ".gemspec", lang: "Ruby", pkgMgr: "Bundler" },
];

/** 表里所有固定文件名（读取用，保持原大小写）。 */
export const stackManifestNames = () => stackTable.filter((r) => r.manifest).map((r) => r.manifest);

/** 表里所有按扩展名认的后缀。 */
export const stackManifestExts = () => stackTable.filter((r) => r.ext).map((r) => r.ext);

/**
 * 「这个文件名是依赖清单吗」——小写基名判据，两处旧的硬编码 Set 都改用它。
 * `extra` 收锁文件之类：它们不该出现在读取清单里（读了也没信息量），但确实算清单。
 */
export const manifestExtra = ["go.sum", "gemfile.lock", "package-lock.json", "yarn.lock", "pnpm-lock.yaml",
  "cargo.lock", "poetry.lock", "composer.lock", "pubspec.lock", "package-swift"];

export function isManifestBaseName(base, { includeLocks = true, includeDocs = false } = {}) {
  // includeDocs 只在极少数「连说明文件也算」的场景放行；默认 notManifest 的行一律不算。
  const b = String(base || "").toLowerCase();
  if (!b) return false;
  for (const row of stackTable) {
    if (row.notManifest && !includeDocs) continue;
    if (row.manifest && row.manifest.toLowerCase() === b) return true;
    if (row.ext && b.endsWith(row.ext)) return true;
  }
  return includeLocks && manifestExtra.includes(b);
}

export function extractStackHints(fileMap) {
  // checkCmd = a FAST compile/type-check (no test execution) — the cheapest strongest
  // per-change correctness signal. Used for non-JS/TS stacks (JS/TS gets live Monaco
  // diagnostics already). Run interleaved so a compile error surfaces the same turn.
  const out = { lang: "", pkgMgr: "", testCmd: "", lintCmd: "", formatCmd: "", devCmd: "", buildCmd: "", checkCmd: "", framework: "", complexity: "small", guessedCmds: [] };
  const pkg = fileMap["package.json"];
  if (pkg) {
    out.lang = "JS/TS";
    try {
      const j = JSON.parse(pkg);
      const scripts = j.scripts || {};
      const deps = { ...(j.dependencies || {}), ...(j.devDependencies || {}) };
      const declaredPm = j.packageManager ? String(j.packageManager).split("@")[0] : "";
      out.pkgMgr = declaredPm || (fileMap["pnpm-lock.yaml"] ? "pnpm" : fileMap["yarn.lock"] ? "yarn" : (fileMap["bun.lock"] || fileMap["bun.lockb"]) ? "bun" : "npm");
      const run = (name) => out.pkgMgr === "yarn" ? `yarn ${name}` : out.pkgMgr === "pnpm" ? `pnpm run ${name}` : out.pkgMgr === "bun" ? `bun run ${name}` : `npm run ${name}`;
      const direct = (name) => out.pkgMgr === "yarn" ? `yarn ${name}` : out.pkgMgr === "pnpm" ? `pnpm ${name}` : out.pkgMgr === "bun" ? `bun run ${name}` : `npm ${name}`;
      // Pick test command — prefer explicit "test" script, fall back to common variants.
      out.testCmd = scripts.test ? direct("test") :
                    scripts["test:unit"] ? run("test:unit") :
                    scripts.vitest ? run("vitest") : "";
      out.lintCmd = scripts.lint ? run("lint") : "";
      out.formatCmd = scripts.format ? run("format") : scripts.fmt ? run("fmt") : "";
      out.devCmd = scripts.dev ? run("dev") : scripts.start ? direct("start") : "";
      out.buildCmd = scripts.build ? run("build") : "";
      // A real "does it compile/typecheck" command so the mid-loop auto-check ACTUALLY fires for JS/TS
      // (was "" before → the biggest population got no real verification). Author-defined scripts only.
      out.checkCmd = scripts.typecheck ? run("typecheck") :
                     scripts["type-check"] ? run("type-check") :
                     scripts.check ? run("check") :
                     scripts.build ? run("build") :
                     scripts.lint ? run("lint") : "";
      // Framework detection by dependency.
      if (deps.next) out.framework = "Next.js";
      else if (deps.nuxt) out.framework = "Nuxt";
      else if (deps["@remix-run/react"]) out.framework = "Remix";
      else if (deps["@sveltejs/kit"]) out.framework = "SvelteKit";
      else if (deps.vite && deps.react) out.framework = "Vite + React";
      else if (deps.vite && deps.vue) out.framework = "Vite + Vue";
      else if (deps.vite) out.framework = "Vite";
      else if (deps["@tauri-apps/api"]) out.framework = "Tauri";
      else if (deps.electron) out.framework = "Electron";
      else if (deps.express) out.framework = "Express";
      else if (deps.fastify) out.framework = "Fastify";
      else if (deps.nestjs || deps["@nestjs/core"]) out.framework = "NestJS";
      else if (deps.react) out.framework = "React";
      else if (deps.vue) out.framework = "Vue";
      // Complexity heuristic: ≥30 deps or workspaces declared → large project.
      const depCount = Object.keys(deps).length;
      if (j.workspaces || depCount >= 60) out.complexity = "large";
      else if (depCount >= 25) out.complexity = "medium";
    } catch { /* malformed package.json — keep defaults */ }
  }
  if (fileMap["Cargo.toml"]) {
    out.lang = out.lang ? out.lang + " + Rust" : "Rust";
    out.pkgMgr = out.pkgMgr || "cargo";
    out.testCmd = out.testCmd || "cargo test";
    out.buildCmd = out.buildCmd || "cargo build";
    out.checkCmd = out.checkCmd || "cargo check"; // fast: type-check, no codegen
    out.formatCmd = out.formatCmd || "cargo fmt";
    out.lintCmd = out.lintCmd || "cargo clippy";
    out.framework = out.framework || "Rust";
    if (/tauri/i.test(fileMap["Cargo.toml"])) out.framework = "Tauri (Rust)";
    if (/axum|actix|warp|rocket/i.test(fileMap["Cargo.toml"])) out.framework = "Rust web (axum/actix)";
  }
  if (fileMap["pyproject.toml"] || fileMap["requirements.txt"]) {
    out.lang = out.lang ? out.lang + " + Python" : "Python";
    out.pkgMgr = out.pkgMgr || (fileMap["pyproject.toml"]?.includes("poetry") ? "poetry" : "pip");
    // 下面的 pytest / ruff 是**猜的默认值**——没有任何项目文件声明过它们，更没验证装没装。
    // 只作为给模型看的提示；绝不能进自动验证管线（实测：机器上没装 → 收尾门禁跑
    // `ruff check . && pytest` 退出 127 → 给用户甩一张"验证器不可用"红卡）。管线端由
    // _verificationCommandsForStack 按 guessedCmds 过滤，落回 _detectVerifyCmd 的存在性探测分支。
    if (!out.testCmd) { out.testCmd = "pytest"; out.guessedCmds.push("pytest"); }
    if (!out.checkCmd) { out.checkCmd = "ruff check ."; out.guessedCmds.push("ruff check ."); } // fast: catches syntax + undefined names
    out.lintCmd = out.lintCmd || "ruff check .";
    out.formatCmd = out.formatCmd || "ruff format .";
    if (/fastapi/i.test(fileMap["pyproject.toml"] || "") || /fastapi/i.test(fileMap["requirements.txt"] || "")) out.framework = "FastAPI";
    else if (/django/i.test(fileMap["pyproject.toml"] || "") || /django/i.test(fileMap["requirements.txt"] || "")) out.framework = "Django";
    else if (/flask/i.test(fileMap["pyproject.toml"] || "") || /flask/i.test(fileMap["requirements.txt"] || "")) out.framework = "Flask";
  }
  if (fileMap["go.mod"]) {
    out.lang = out.lang ? out.lang + " + Go" : "Go";
    out.pkgMgr = out.pkgMgr || "go mod";
    out.testCmd = out.testCmd || "go test ./...";
    out.buildCmd = out.buildCmd || "go build ./...";
    out.checkCmd = out.checkCmd || "go build ./..."; // fast: compiles, no test run
    out.formatCmd = out.formatCmd || "gofmt -w .";
    if (/gin-gonic\/gin/i.test(fileMap["go.mod"])) out.framework = "Gin (Go)";
    else if (/labstack\/echo/i.test(fileMap["go.mod"])) out.framework = "Echo (Go)";
  }
  if (fileMap["Makefile"] && !out.testCmd) {
    if (/^test:/m.test(fileMap["Makefile"])) out.testCmd = "make test";
  }
  if (fileMap["pom.xml"]) {
    out.lang = out.lang ? out.lang + " + Java" : "Java";
    out.pkgMgr = out.pkgMgr || "Maven";
    out.testCmd = out.testCmd || "mvn -q test";
    out.checkCmd = out.checkCmd || "mvn -q compile";
    out.buildCmd = out.buildCmd || "mvn -q package";
  }
  const _gradle = fileMap["build.gradle"] || fileMap["build.gradle.kts"];
  if (_gradle) {
    out.lang = out.lang ? out.lang + " + JVM" : (/kotlin/i.test(_gradle) ? "Kotlin" : "Java");
    out.pkgMgr = out.pkgMgr || "Gradle";
    out.testCmd = out.testCmd || "./gradlew test";
    out.checkCmd = out.checkCmd || "./gradlew build";
    out.buildCmd = out.buildCmd || "./gradlew build";
  }
  if (fileMap["composer.json"]) {
    out.lang = out.lang ? out.lang + " + PHP" : "PHP";
    out.pkgMgr = out.pkgMgr || "Composer";
    try {
      const scripts = JSON.parse(fileMap["composer.json"])?.scripts || {};
      if (scripts.test) out.testCmd = out.testCmd || "composer test";
    } catch {}
  }
  if (fileMap["Gemfile"]) {
    out.lang = out.lang ? out.lang + " + Ruby" : "Ruby";
    out.pkgMgr = out.pkgMgr || "Bundler";
    if (!out.testCmd) {
      // 没读到 Rakefile/spec 目录就只是常规猜测——如实标注，模型跑到 127 时才知道
      // 该换命令而不是当成代码错误。
      out.testCmd = /rspec/i.test(fileMap["Gemfile"]) ? "bundle exec rspec" : "bundle exec rake test";
      (out.guessedCmds = out.guessedCmds || []).push(out.testCmd);
    }
  }
  // 表驱动的其余语言。专用分支先跑，这里只**填空**（`||`），所以一个 JS/TS + Elixir 的
  // 混合仓不会被后来者把已经识别出来的命令顶掉；lang 走和上面完全相同的 " + X" 追加口径。
  for (const row of stackTable) {
    if (row.bespoke || row.doc) continue;
    const key = row.manifest || row.ext;
    const content = fileMap[key];
    if (content === undefined || content === null || content === "") continue;
    if (row.lang) out.lang = out.lang ? (out.lang.includes(row.lang) ? out.lang : out.lang + " + " + row.lang) : row.lang;
    if (row.pkgMgr) out.pkgMgr = out.pkgMgr || row.pkgMgr;
    // 框架嗅探先做：命中的框架可能整组换掉命令（Flutter 之于 Dart）。
    let framework = "";
    for (const [re, name] of row.frameworks || []) {
      if (re.test(String(content))) { framework = name; break; }
    }
    if (framework) out.framework = out.framework || framework;
    const cmds = { ...(row.cmds || {}), ...((framework && row.whenFramework?.[framework]) || {}) };
    if (cmds.pkgMgr) { out.pkgMgr = cmds.pkgMgr; delete cmds.pkgMgr; }
    const guessed = new Set(row.guessed || []);
    for (const [k, v] of Object.entries(cmds)) {
      if (out[k] || !v) continue;
      out[k] = v;
      // 框架专属那组是从项目文件里嗅出来的事实，不是猜测；只有 row.guessed 里列名的才是。
      if (guessed.has(v)) (out.guessedCmds = out.guessedCmds || []).push(v);
    }
  }
  return out;
}

// Format the extracted stack into a tight, model-friendly hint block. Goes at the
// TOP of the context (before raw file dumps) so the model sees it first.
export function formatStackHint(s) {
  // 认不出语言 ≠ 什么都不知道：套件位置、用户自己声明的构建命令、已声明依赖都可能在。
  // 以前这里一个 `!s.lang` 就把整块提示吞掉，模型连一行「怎么跑测试」都收不到，
  // 于是只能猜，或者干脆一条验证命令都不跑就交付。
  if (!s) return "";
  const _known = s.lang || s.testDir || s.checkCmd || s.testCmd || s.buildCmd || s.devCmd
    || s.lintCmd || s.formatCmd || s.declaredKeys?.length;
  if (!_known) return "";
  const lines = [`📦 项目栈: ${s.lang || "未识别（下面这些是从项目文件或你的声明里读到的事实）"}${s.framework ? " · " + s.framework : ""}${s.pkgMgr ? " · 包管理 " + s.pkgMgr : ""}${s.complexity === "large" ? " · ⚠️ 大项目" : s.complexity === "medium" ? " · 中型项目" : ""}`];
  // 猜测命令要如实标注——模型自己跑到 127 时才知道该换命令而不是当成代码错误。
  const _guessed = new Set(s.guessedCmds || []);
  const _unverified = (cmd) => _guessed.has(cmd) ? "（猜测默认、未验证已安装；退出 127 就换 venv 内工具或 python -m compileall，别当代码错误）" : "";
  // 这两行原来向模型断言：「agent 会每改几个文件自动跑」「失败 agent 会自动注入失败报告」。
  // **那套机器根本不存在**——`_runApprovedVerification` 只有定义零调用点，它包着的
  // `_interleavedTest`（唯一会真去 taskRunCapture 跑命令的那个）因此也只在死代码里可达；
  // 那两个"待校验文件"账本从落盘第一天起就没有过读者，写入处的注释还写着"推迟到收尾门"
  // ——收尾门里也没有。账本已删除（2026-08-20）。
  //
  // 后果不是"少了个功能"，是**机器主动给了模型一个错误的世界模型**：它每轮都在上下文
  // 顶部读到这句断言，于是理性地把跑构建/测试外包给 IDE，改完就收尾。什么都没跑。
  // 收尾时 `_hasVerifyEvidence` 为假，只记一行 `code_delivered_unverified`（那条路刻意
  // 只记账不补回合），用户拿到的是一份没编译过的代码加一行小字。
  // 这正是"写出来的代码很容易用不了"最直接的一条机器原因。
  //
  // 改成祈使句：验证是**模型自己的活**，而且说清"没人替你跑"。
  if (s.checkCmd) lines.push(`✅ 快速校验: \`${s.checkCmd}\`（编译/类型检查）——**改完必须你自己跑这条**——IDE 只会在你被提醒之后仍然不跑时兜底跑一次，每落出新的一版重新算，全程最多 3 次，兜出来的红字同样算你的账，别把它当默认路径。退出码非 0 就是结论——这条没过；退出码 0 只说明这条检查通过了，不等于用户要的事做成了，还要看真实输出和目标后置状态。${_unverified(s.checkCmd)}`);
  if (s.testCmd) lines.push(`🧪 测试: \`${s.testCmd}\`——**改完必须你自己跑**——同样只在你被提醒过之后才兜、且全程最多 3 次，别指望失败报告自动送到你面前。${_unverified(s.testCmd)}`);
  // 有套件就说清位置。只给命令不给位置，模型就会在根目录另起一个脚本——实测如此。
  if (s.testDir) {
    lines.push(`📁 测试套件在 \`${s.testDir}/\`${s.testSubs?.length ? `（${s.testSubs.join(" / ")}）` : ""}`
      + `——新测试**写进这个目录**、沿用它现有的组织方式和命名，不要在项目根目录另起 test_*.py / *_test.js 之类的散文件。`
      + `临时验证用的一次性脚本可以随便写，但**用完必须删掉**，别留在仓库里。`);
  }
  if (s.devCmd) lines.push(`🚀 启动 dev: \`${s.devCmd}\``);
  if (s.buildCmd) lines.push(`🔨 构建: \`${s.buildCmd}\``);
  if (s.lintCmd) lines.push(`🔧 lint: \`${s.lintCmd}\``);
  if (s.formatCmd) lines.push(`🎨 格式化: \`${s.formatCmd}\``);
  return lines.join("\n");
}
