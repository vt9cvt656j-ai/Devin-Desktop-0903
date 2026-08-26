// 「项目栈认不出来」的代价不是少认一个栈，是**整块栈提示一个字都不出**——模型连一行
// 「怎么跑测试」都收不到，于是只能猜，或者干脆一条验证命令都不跑就交付。
//
// 这个文件守三件事：
//   1. 那份"哪些文件是依赖清单"的名单只有一份（以前在三个地方各写了一遍并且互相漂开）；
//   2. 加一门语言真的只需要加一行数据，不用改代码；
//   3. 用户自己声明的构建命令真的能盖过探测结果——公司内部的 `./bin/ci check` 永远
//      不可能进产品自带的表。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load, loadConst, CODE, fnSource as topLevelFn } from "./helpers/source.mjs";
// 2026-08-25：这一族搬进了 src/agent/stack.js，于是这里从「按名字抠源码 + 手工注入
// 依赖」换成**真 import**。差别不是写法好看：抠源码验得到行为，验不到「这个函数在
// 真实调用链上还在不在」——搬出去之前只能那么写，因为它们绑在一个没法 import 的文件里。
import {
  stackTable as TABLE, manifestExtra as EXTRA, extractStackHints as extract,
  isManifestBaseName as isManifest, stackManifestNames, stackManifestExts,
  formatStackHint as fmt,
} from "../src/agent/stack.js";

const names = stackManifestNames();
const exts = stackManifestExts();

// ── 1. 一份名单，三处判据 ────────────────────────────────────────────────
test("三处判据全部从同一张表派生，源码里不许再有第二份硬编码清单", () => {
  // 以前：读取清单 12 条、"改它就是在选技术栈" 15 条、"清单只算文档" 13 条，各写各的。
  // 同一个 pubspec.yaml 在一处算依赖清单、在另一处不算——这种分叉最难查。
  const stray = CODE.match(/new Set\(\[[^\]]*"(?:package\.json|cargo\.toml|pyproject\.toml)"[^\]]*\]\)/gi) || [];
  assert.deepEqual(stray, [],
    `又出现了一份硬编码的清单名单：${stray.join(" / ")}——它会和 _STACK_TABLE 漂开`);
});

test("读取清单和「是不是依赖清单」用的是同一张表", () => {
  for (const n of names) {
    const row = TABLE.find((r) => r.manifest === n);
    if (row?.notManifest) continue;
    assert.ok(isManifest(n.toLowerCase()), `${n} 会被读，却不被认成依赖清单`);
  }
});

test("Makefile 和 README 会被读，但都不是依赖清单", () => {
  assert.ok(names.includes("Makefile"), "Makefile 要读——里面有 test: 目标");
  // 改 Makefile 不等于在选技术栈；读 Makefile 也不等于"只读了一份依赖描述"。
  assert.equal(isManifest("makefile"), false, "Makefile 是构建逻辑，不是依赖清单");
  assert.equal(isManifest("readme.md"), false, "README 是说明，不是依赖清单");
});

test("锁文件算依赖清单，但不算「在选技术栈」", () => {
  // 锁文件是解析结果，不是人做的选择：改 package-lock.json 不该触发一轮技术选型调研。
  assert.equal(isManifest("package-lock.json"), true);
  assert.equal(isManifest("package-lock.json", { includeLocks: false }), false);
});

// ── 2. 加一门语言 = 加一行 ───────────────────────────────────────────────
test("表驱动的语言真的能被识别出命令，不是摆在那儿好看", () => {
  const cases = [
    ["mix.exs", "Elixir", "mix test"],
    ["pubspec.yaml", "Dart", "dart test"],
    ["Package.swift", "Swift", "swift test"],
    ["build.zig", "Zig", "zig build test"],
    ["deno.json", "Deno", "deno test -A"],
    ["build.sbt", "Scala", "sbt test"],
    ["stack.yaml", "Haskell", "stack test"],
    ["CMakeLists.txt", "C/C++", "ctest --test-dir build"],
  ];
  for (const [manifest, lang, testCmd] of cases) {
    const out = extract({ [manifest]: "x" });
    assert.equal(out.lang, lang, `${manifest} 没被认成 ${lang}`);
    assert.equal(out.testCmd, testCmd, `${manifest} 认出来了却不知道怎么跑测试`);
  }
});

test("按扩展名认的那一族（.NET / F# / Haskell 工程文件）", () => {
  // 工程文件名跟着项目名走（MyApp.csproj），没有固定文件名可读——这整个语族此前
  // 一次都识别不出来，因为读取清单只会按固定文件名去读。
  assert.ok(exts.includes(".csproj") && exts.includes(".fsproj"));
  assert.equal(extract({ ".csproj": "<Project />" }).lang, "C#");
  assert.equal(extract({ ".csproj": "<Project />" }).checkCmd, "dotnet build");
  assert.equal(isManifest("myapp.csproj"), true, "按扩展名认的也要算依赖清单");
});

test("按扩展名那一族真的被读进来了，不只是表里列着", () => {
  // 这条守的是**接线**：_extractStackHints 是纯函数（只吃 fileMap），负责把 .csproj
  // 的内容放进 fileMap 的那段在 _gatherAgentContext 里。只测纯函数的话，把那段读取
  // 整块删掉仍然全绿——实测确实如此（变异 2026-08-22），所以补这一条源码断言。
  // 断言跑在 CODE（注释整段置空）上：注释里提一句不算实现。
  const ctx = topLevelFn("_gatherAgentContext", { code: true });
  assert.match(ctx, /_stackManifestExts\(\)/, "按扩展名认的那一族没人去读，整表等于摆设");
  assert.match(ctx, /readDir/, "不列目录就找不到 MyApp.csproj 这种跟着项目名走的文件名");
  assert.match(ctx, /fileMap\[hit\[0\]\] = hit\[1\]/,
    "读到了却没进 fileMap——_extractStackHints 看不见它");
});

test("框架嗅探能整组换掉命令：Flutter 不是 dart test", () => {
  const plain = extract({ "pubspec.yaml": "name: x\ndependencies:\n  http: ^1.0.0\n" });
  assert.equal(plain.testCmd, "dart test");
  const flutter = extract({ "pubspec.yaml": "name: x\ndependencies:\n  flutter:\n    sdk: flutter\n" });
  assert.equal(flutter.framework, "Flutter");
  assert.equal(flutter.testCmd, "flutter test", "Flutter 项目跑 dart test 是跑不起来的");
  assert.equal(flutter.checkCmd, "flutter analyze");
});

test("专用分支先跑，表驱动只填空——混合仓不会被后来者顶掉", () => {
  const mixed = extract({
    "package.json": JSON.stringify({ scripts: { test: "vitest" } }),
    "mix.exs": "defmodule X.MixProject do end",
  });
  assert.equal(mixed.testCmd, "npm test", "已经识别出来的命令被表驱动分支顶掉了");
  assert.match(mixed.lang, /JS\/TS/);
  assert.match(mixed.lang, /Elixir/, "第二种语言没被追加进 lang");
});

test("猜的命令要如实标成猜测，从项目文件嗅出来的不算猜", () => {
  const cmake = extract({ "CMakeLists.txt": "project(x)" });
  assert.ok((cmake.guessedCmds || []).includes("ctest --test-dir build"),
    "没标成猜测的话，模型跑到 127 会当成代码错误，验证管线也会拿它去跑");
  const flutter = extract({ "pubspec.yaml": "flutter:\n  sdk: flutter\n" });
  assert.ok(!(flutter.guessedCmds || []).includes("flutter test"),
    "从项目文件里嗅出来的是事实，标成猜测会让验证管线白白过滤掉它");
});

// ── 3. 用户声明盖过探测 ──────────────────────────────────────────────────
function withOverride(over, base) {
  const apply = load("_applyUserStackOverride", { _userCapabilities: () => ({ stack: over }) });
  return apply(base);
}

test("用户声明的构建命令盖过探测结果", () => {
  const out = withOverride({ checkCmd: "./bin/ci check" }, extract({ "CMakeLists.txt": "project(x)" }));
  assert.equal(out.checkCmd, "./bin/ci check", "填了不生效，等于这个功能不存在");
  assert.equal(out.lang, "C/C++", "没被覆盖的字段不该动");
});

test("被用户覆盖掉的猜测命令要从「猜测」名单里摘掉", () => {
  // 留着的话它会一直被标注成"未验证已安装"，也会被验证管线当猜测过滤掉——等于白填。
  const base = extract({ "CMakeLists.txt": "project(x)" });
  assert.ok(base.guessedCmds.includes("ctest --test-dir build"));
  const out = withOverride({ testCmd: "./bin/ci test" }, base);
  assert.equal(out.testCmd, "./bin/ci test");
  assert.ok(!out.guessedCmds.includes("ctest --test-dir build"),
    "用户声明的是事实，不是猜测；旧的猜测标记必须跟着摘掉");
});

test("没有声明时一个字段都不动", () => {
  const base = extract({ "mix.exs": "x" });
  assert.deepEqual(withOverride(undefined, { ...base }), base);
  assert.deepEqual(withOverride({}, { ...base }), base);
});

test("声明读不到时安静退回，不能把栈识别整个带崩", () => {
  const apply = load("_applyUserStackOverride", { _userCapabilities: () => { throw new Error("坏了"); } });
  const base = extract({ "mix.exs": "x" });
  assert.equal(apply(base), base);
});

// ── 4. 认不出语言 ≠ 什么都不说 ───────────────────────────────────────────
// fmt 现在从模块直接 import（见文件头）。原来要注入一个 `t` 桩，而 formatStackHint
// 根本不调 t —— 那是早就失效的陈迹，一起去掉。

test("认不出语言时，已知的事实照样要说出来", () => {
  // 以前一个 `!s.lang` 就把整块提示吞掉：套件位置、用户自己声明的构建命令、
  // 已声明依赖会**一起**消失，模型连一行「怎么跑测试」都收不到。
  const hint = fmt({ lang: "", testDir: "tests", checkCmd: "./bin/ci check", declaredKeys: ["checkCmd"] });
  assert.notEqual(hint, "", "认不出语言就整块噤声——这正是要修的那个洞");
  assert.match(hint, /tests/, "套件位置没说");
  assert.match(hint, /\.\/bin\/ci check/, "用户自己声明的命令没说");
});

test("真的什么都不知道时才闭嘴", () => {
  assert.equal(fmt({}), "");
  assert.equal(fmt(null), "");
  assert.equal(fmt({ lang: "", complexity: "small" }), "");
});

/**
 * 搬进模块买到的东西：这条测的是**真实调用链上的那份代码**。
 *
 * 搬出去之前，这一族只能靠「按名字从 main.js 抠源码 + 手工注入依赖」来测——那验得到
 * 行为，验不到「这个函数还在不在真实调用链上」。整个文件删掉一半、只要那个函数体
 * 还在源码里，抠源码那种写法照样绿。现在是 import，删了就报错。
 */
test("直接 import 真模块，栈探测在真实的 package.json 上给出可执行的命令", () => {
  const hints = extract({
    "package.json": JSON.stringify({
      scripts: { test: "vitest", build: "vite build" },
      devDependencies: { vite: "^5.0.0" },
    }),
  });
  assert.equal(hints.testCmd, "npm test", "认出了 vitest 却没给出能跑的命令");
  assert.equal(hints.buildCmd, "npm run build");
  assert.equal(hints.lang, "JS/TS");
  // 提示文本是模型唯一的依据，必须真的把命令带出去。
  const hint = fmt(hints);
  assert.match(hint, /npm test|npm run build/, "栈提示里一条可执行命令都没有");
  // 认不出的栈不许编一个出来。
  assert.equal(extract({ "随便一个文件.xyz": "" }).lang || "", "");
});
