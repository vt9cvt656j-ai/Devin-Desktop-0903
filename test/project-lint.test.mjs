// 「业务逻辑错误没有任何机械检查」这条缺口。
//
// 交错验证此前只有一条腿：Monaco 内建 worker 的语法/类型诊断 + LSP。那条腿认的是
// 「这行编译不过」。而用户真正吃亏的一类 bug 在语法上全对、类型也全过：漏 await 的
// promise、该用 === 用了 ==、React hook 依赖不全、catch 了又吞掉、未使用变量背后藏着
// 拼错的名字。只有项目自己配的 linter 认得，而那份配置就躺在仓库里没人读。
// 纯 JS 项目最极端：没有类型检查，那条腿只剩语法。
//
// 这份守三件事：认得出项目配了什么、只跑改动的那几个文件、输出读得对（且只取 error）。
// 最后两条**真的把 eslint 跑起来**——解析器是照着输出格式写的，格式变了只有真跑才知道。
import test from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, rmSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { SRC, fnSource } from "./helpers/source.mjs";
import {
  detectLinter, lintCommand, parseLintErrors, lintRan, lintFindings,
  newFindings, findingKey, lintReport,
} from "../src/agent/project-lint.js";

test("只按项目自己的配置认——仓库没配就什么都不跑", () => {
  assert.deepEqual(detectLinter(new Set(["package.json", "src"]), { name: "x" }), [],
    "没配 linter 却跑了一个——拿我们选的规则评判别人的代码，产出的全是噪音");
  assert.deepEqual(detectLinter(new Set(), null), []);

  const flat = detectLinter(new Set(["eslint.config.js"]), null);
  assert.equal(flat[0]?.id, "eslint", "扁平配置（新一代命名）没认出来");
  const legacy = detectLinter(new Set([".eslintrc.json"]), null);
  assert.equal(legacy[0]?.id, "eslint", "旧命名没认出来——大量存量仓库还在用");
  // 配置也可以整个塞在 package.json 里。
  assert.equal(detectLinter(new Set(["package.json"]), { eslintConfig: { rules: {} } })[0]?.id, "eslint");
  // 只在 devDependencies 里声明、配置还没落盘的也算。
  assert.equal(detectLinter(new Set(), { devDependencies: { eslint: "^9" } })[0]?.id, "eslint");

  assert.equal(detectLinter(new Set(["ruff.toml"]), null)[0]?.id, "ruff");
  assert.equal(detectLinter(new Set(["pyproject.toml"]), { pyprojectHasRuff: true })[0]?.id, "ruff");
  assert.equal(detectLinter(new Set(["go.mod"]), null)[0]?.id, "go-vet",
    "go vet 是工具链自带的，有 go.mod 就一定跑得起来");
  assert.equal(detectLinter(new Set(["go.mod", ".golangci.yml"]), null)[0]?.id, "golangci-lint",
    "项目配了 golangci 就用它，别退回更弱的 go vet");

  // 一个仓库可以同时是 JS 和 Python 的。
  const both = detectLinter(new Set(["eslint.config.js", "ruff.toml"]), null).map((l) => l.id);
  assert.deepEqual(both.sort(), ["eslint", "ruff"]);
});

test("只跑改动的那几个文件——交错验证里没有全仓扫描的余地", () => {
  const eslint = detectLinter(new Set(["eslint.config.js"]), null)[0];
  const cmd = lintCommand(eslint, ["src/a.ts", "src/b.tsx", "README.md", "app.py"]);
  assert.equal(cmd.program, "npx");
  assert.ok(cmd.args.includes("src/a.ts") && cmd.args.includes("src/b.tsx"));
  assert.ok(!cmd.args.includes("README.md"), "md 不归 eslint 管");
  assert.ok(!cmd.args.includes("app.py"), "py 不归 eslint 管");
  assert.ok(!cmd.args.some((a) => a === "." || a === "src" || a.includes("**")),
    `命令里出现了全仓/通配目标：${cmd.args.join(" ")}`);

  // 这一批里一个它管得着的文件都没有 → 整个跳过，不跑一次空扫描。
  assert.equal(lintCommand(eslint, ["README.md", "go.mod"]), null);
  assert.equal(lintCommand(eslint, []), null);

  // go vet 吃的是包不是文件。
  const vet = detectLinter(new Set(["go.mod"]), null)[0];
  assert.deepEqual(lintCommand(vet, ["cmd/server/main.go", "cmd/server/util.go"]).args,
    ["vet", "./cmd/server/..."], "同一个包不该被列两遍");
});

test("只取 error；warning 一律丢掉", () => {
  const eslintOut = JSON.stringify([{
    filePath: "/repo/src/a.ts",
    messages: [
      { severity: 2, line: 12, ruleId: "no-floating-promises", message: "Promises must be awaited" },
      { severity: 1, line: 3, ruleId: "quotes", message: "Strings must use singlequote" },
      { fatal: true, line: 1, ruleId: null, message: "Parsing error: Unexpected token" },
    ],
  }]);
  const errs = parseLintErrors("eslint", eslintOut);
  assert.equal(errs.length, 2, "warning 被当成 error 了——阻断门里放风格就是把门玩坏");
  assert.equal(errs[0].line, 12);
  assert.equal(errs[0].rule, "no-floating-promises");
  assert.ok(errs[1].message.includes("Parsing error"), "fatal 必须当 error");

  // ruff 的形状不一样。
  const ruffErrs = parseLintErrors("ruff", JSON.stringify([
    { filename: "app.py", location: { row: 7 }, code: "F821", message: "Undefined name `reponse`" },
  ]));
  assert.deepEqual(ruffErrs, [{ rel: "app.py", line: 7, rule: "F821", message: "Undefined name `reponse`" }]);

  // go vet 不出 JSON，走行格式。
  const vetErrs = parseLintErrors("go-vet", "cmd/main.go:12:5: Printf format %d has arg s of wrong type string\n");
  assert.equal(vetErrs.length, 1);
  assert.equal(vetErrs[0].line, 12);

  // 读不懂就说读不懂，别猜。
  assert.deepEqual(parseLintErrors("eslint", "not json at all"), []);
  assert.deepEqual(parseLintErrors("eslint", ""), []);
});

test("仓库里本来就有的问题不算模型头上——否则门一开就永远关不上", () => {
  const before = [
    { rel: "src/a.ts", line: 3, rule: "no-unused-vars", message: "'x' is defined but never used" },
  ];
  const after = [
    // 同一条，只是编辑让它挪了两行 —— 行号不进身份，不该被当成新的。
    { rel: "src/a.ts", line: 5, rule: "no-unused-vars", message: "'x' is defined but never used" },
    { rel: "src/a.ts", line: 9, rule: "no-floating-promises", message: "Promises must be awaited" },
  ];
  const fresh = newFindings(before, after);
  assert.equal(fresh.length, 1, "老问题被算成新引入的了");
  assert.equal(fresh[0].rule, "no-floating-promises");

  // 绝对路径和相对路径指的是同一个文件。
  assert.equal(
    findingKey({ rel: "/Users/x/repo/src/a.ts", rule: "r", message: "m" }),
    findingKey({ rel: "repo/src/a.ts", rule: "r", message: "m" }),
  );
  // 同一条重复出现只报一次。
  assert.equal(newFindings([], [after[1], { ...after[1], line: 40 }]).length, 1);
  // baseline 为空（没抓到）时全部都算新的——这是调用方按 ran:false 决定要不要用的前提。
  assert.equal(newFindings([], after).length, 2);
});

test("阻断正文有界，且说清这是项目自己的规则", () => {
  assert.equal(lintReport("eslint", []), "", "没有 finding 时不发块");
  const many = Array.from({ length: 30 }, (_, i) => ({ rel: "a.ts", line: i, rule: "r", message: `m${i}` }));
  const text = lintReport("eslint", many);
  assert.ok(text.includes("还有 10 条同类"), "没有截断——三十条错误糊进上下文");
  assert.ok(text.includes("项目自己的规则"),
    "没说清规则来源，模型会以为是 harness 强加的口味，然后去改 lint 配置绕过它");
});

test("「没跑成」和「跑了、干干净净」必须可分——这是整块最容易静默失效的地方", () => {
  // `npx --no-install eslint` 在没装 eslint 的项目里：正常启动、非零退出、stdout 空、
  // stderr 是 npm 的报错。进程跑起来了，所以没有 note；解析器拿到空串返回空数组。
  // 少了 lintRan 这一层，它就等于「零错误，干干净净」——用户以为写时检查在保护他。
  assert.equal(
    lintRan("eslint", { code: 1, stdout: "", stderr: "npm error npx canceled due to missing packages" }),
    false,
    "没装 eslint 被读成「零错误」了",
  );
  assert.equal(lintRan("eslint", { code: 0, stdout: "[]", stderr: "" }), true, "真跑过、真干净");
  assert.equal(lintRan("eslint", { code: 1, stdout: '[{"filePath":"a.js","messages":[]}]' }), true,
    "有合法 JSON 就是跑过了，退出码非零只说明它发现了问题");
  assert.equal(lintRan("eslint", { code: 1, stdout: "Cannot find module 'eslint'" }), false,
    "输出不是 JSON = 没跑成");

  // go vet：退出码 0 就是干净；非零但一条都解析不出来，说明是工具自己出错。
  assert.equal(lintRan("go-vet", { code: 0, stdout: "", stderr: "" }), true);
  assert.equal(lintRan("go-vet", { code: 1, stdout: "", stderr: "go: cannot find main module" }), false,
    "工具自己出错被当成「代码有问题」了");
  assert.equal(lintRan("go-vet", { code: 1, stdout: "", stderr: "a.go:7:2: Printf format %d has arg s of wrong type string" }), true);
});

test("诊断可能在 stderr——只读 stdout 的话 go vet 永远报告零问题", () => {
  const out = { code: 1, stdout: "", stderr: "cmd/main.go:7:2: Printf format %d has arg s of wrong type string" };
  const findings = lintFindings("go-vet", out);
  assert.equal(findings.length, 1, "stderr 里的诊断被丢了");
  assert.equal(findings[0].line, 7);
  // stdout 里有东西时以 stdout 为准（JSON 类都写 stdout）。
  assert.equal(lintFindings("eslint", {
    stdout: JSON.stringify([{ filePath: "a.ts", messages: [{ severity: 2, line: 1, ruleId: "r", message: "m" }] }]),
    stderr: "some warning noise",
  }).length, 1);
});

// ── 真的跑一遍 ───────────────────────────────────────────────────────────────
// 解析器是照着输出格式写的。格式在版本之间会变，而变了之后解析恒返回空数组、
// 整道门静默失效、测试全绿——正是这个仓库反复踩的那类坑。所以这一条真起一个进程。
const eslintAvailable = (() => {
  const probe = spawnSync("npx", ["--no-install", "eslint", "--version"], { encoding: "utf8", timeout: 60_000 });
  return probe.status === 0 && /\d+\./.test(String(probe.stdout || ""));
})();

test("真跑一次 eslint，解析器读得懂它今天的输出", { skip: !eslintAvailable && "本机没有 eslint（跳过，不是通过）" }, () => {
  const dir = mkdtempSync(join(tmpdir(), "mrday-lint-"));
  try {
    mkdirSync(join(dir, "src"), { recursive: true });
    writeFileSync(join(dir, "eslint.config.js"),
      'export default [{ files: ["**/*.js"], rules: { eqeqeq: "error", "no-unused-vars": "warn" } }];\n');
    writeFileSync(join(dir, "package.json"), '{"name":"t","type":"module"}\n');
    // eqeqeq 是 error，no-unused-vars 是 warn —— 正好验证「只取 error」。
    writeFileSync(join(dir, "src/a.js"), 'const unused = 1;\nexport const f = (a, b) => a == b;\n');

    const linter = detectLinter(new Set(["eslint.config.js", "package.json"]), null)[0];
    assert.equal(linter.id, "eslint");
    const cmd = lintCommand(linter, ["src/a.js"]);
    const out = spawnSync(cmd.program, cmd.args, { cwd: dir, encoding: "utf8", timeout: 120_000 });

    const findings = parseLintErrors("eslint", out.stdout);
    assert.equal(findings.length, 1,
      `解析器读不懂 eslint 今天的输出（拿到 ${findings.length} 条）。原始输出：\n${out.stdout}\n${out.stderr}`);
    assert.equal(findings[0].rule, "eqeqeq");
    assert.equal(findings[0].line, 2);
    assert.ok(findings[0].rel.endsWith("src/a.js"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

const goAvailable = spawnSync("go", ["version"], { encoding: "utf8", timeout: 60_000 }).status === 0;

test("真跑一次 go vet，解析器读得懂它今天的输出", { skip: !goAvailable && "本机没有 go（跳过，不是通过）" }, () => {
  const dir = mkdtempSync(join(tmpdir(), "mrday-vet-"));
  try {
    mkdirSync(join(dir, "cmd"), { recursive: true });
    writeFileSync(join(dir, "go.mod"), "module example.com/t\n\ngo 1.21\n");
    // 这段**编译得过、类型也对**，只有 vet 认得出 %d 配了个 string ——
    // 正是「语法和类型那条腿看不见的那一类」。
    writeFileSync(join(dir, "cmd/main.go"),
      'package main\n\nimport "fmt"\n\nfunc main() {\n\ts := "x"\n\tfmt.Printf("%d\\n", s)\n}\n');

    const linter = detectLinter(new Set(["go.mod"]), null)[0];
    assert.equal(linter.id, "go-vet");
    const cmd = lintCommand(linter, ["cmd/main.go"]);
    const out = spawnSync(cmd.program, cmd.args, { cwd: dir, encoding: "utf8", timeout: 180_000 });
    // go vet 把诊断写在 stderr。
    const findings = parseLintErrors("go-vet", `${out.stdout}\n${out.stderr}`);
    assert.ok(findings.length >= 1,
      `解析器读不懂 go vet 今天的输出。原始：\n${out.stdout}\n${out.stderr}`);
    const hit = findings.find((f) => /Printf|format/i.test(f.message));
    assert.ok(hit, `没抓到那条格式串错误：${JSON.stringify(findings)}`);
    assert.ok(hit.rel.endsWith("main.go"));
    assert.equal(hit.line, 7);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// ── 接线 ─────────────────────────────────────────────────────────────────────
test("这条腿真的接在那道阻断门上，而不是算完就扔", () => {
  const loop = fnSource("_runAgenticLoop", { code: true });
  assert.ok(loop.includes("_projectLintFindings("),
    "循环里根本没调用项目 linter —— 模块写了没人用");
  assert.match(loop, /run\._diagnosticBlock\s*=\s*_noProgressVerify\s*<\s*2\s*\?\s*_lintReportText/,
    "lint 结果没进 _diagnosticBlock —— 那是收尾门唯一会读的地方，不进去就是只写不读");
  assert.ok(loop.includes("[BLOCKING_NEW_DIAGNOSTICS]"),
    "没有推给模型的阻断提示");

  // baseline 必须在**改动之前**抓：抓晚了，仓库里原有的问题会被算成模型引入的，
  // 门一开就永远关不上。
  const before = loop.indexOf("_projectLintFindings(_newBaselinePaths");
  const after = loop.indexOf("_projectLintFindings([...run._diagnosticCheckPaths]");
  assert.ok(before > 0, "没有在改动前抓 baseline");
  assert.ok(after > before, "两次调用的先后顺序反了");

  // 「配了却跑不起来」必须如实说出去，不能静默当成零问题。
  assert.ok(SRC.includes("lintless:"), "没跑成时没有任何事实交代 —— 会被当成「零错误」");
  // 这两层在执行那一侧（_projectLintFindings），不在循环里。
  const exec = fnSource("_projectLintFindings", { code: true });
  assert.ok(exec.includes("_lintRan(linter.id, out)"),
    "少了这一层，npx 找不到 eslint 的非零退出会被读成「零错误、干干净净」");
  assert.ok(exec.includes("_lintFindings(linter.id, out)"),
    "只读 stdout 的话 go vet 永远报告零问题（它的诊断在 stderr）");
});
