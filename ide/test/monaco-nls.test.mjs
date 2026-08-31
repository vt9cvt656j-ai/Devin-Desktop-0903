// 编辑器自带的那套界面文案（右键菜单、查找框、悬浮提示、命令面板）跟着用户选的语言走。
//
// 用户原话：「我选中内容 右键 应该看看用户选择的什么语言，那么这个菜单内容就显示什么语言」。
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { monacoNlsLocale, loadMonacoNls } from "../src/monaco-nls.js";
import { GLOBAL_LANGUAGE_TAGS } from "../src/locales.js";

const require = createRequire(import.meta.url);
const HERE = new URL(".", import.meta.url);
const src = (p) => readFileSync(new URL(p, HERE), "utf8");

test("当前语言的判据和 initLocale 一致，且不依赖它先跑过", () => {
  // 这一步排在整个应用之前，那时 initLocale 还没执行；所以判据要自己算一遍：
  // 存的值 → 系统偏好 → 收敛到支持的语言。
  const fake = (v) => ({ getItem: () => v });
  assert.equal(monacoNlsLocale(fake("ja")), "ja");
  assert.equal(monacoNlsLocale(fake("zh-CN")), "zh-CN");
  assert.equal(monacoNlsLocale(fake("pt-BR")), "pt", "带地区的存值没有收敛到支持的语言");
  // 存的值坏了 / localStorage 读不了，都不能抛——它挡在启动路径上。
  assert.doesNotThrow(() => monacoNlsLocale(fake("")));
  assert.doesNotThrow(() => monacoNlsLocale({ getItem() { throw new Error("nope"); } }));
  assert.doesNotThrow(() => monacoNlsLocale(null));
  assert.ok(GLOBAL_LANGUAGE_TAGS.includes(monacoNlsLocale(fake("火星文"))), "坏值没有收敛到支持的语言");
});

test("英语不加载语言包（Monaco 的兜底本来就是英语）", async () => {
  assert.equal(await loadMonacoNls("en"), "", "给英语也去下一个语言包了");
  assert.equal(await loadMonacoNls("这不是语言"), "");
});

test("产品支持的每一种语言都指到一个真存在的 monaco 语言包", () => {
  // 拼错一个文件名，构建时是能过的（动态 import 的失败被 catch 吞掉），
  // 表现是那种语言的菜单**静默留在英文**。所以逐个去 node_modules 里确认文件在。
  const mod = src("../src/monaco-nls.js");
  const mapped = [...mod.matchAll(/^\s{2}"?([\w-]+)"?:\s*\(\)\s*=>\s*import\("monaco-editor\/esm\/nls\.messages\.([\w-]+)\.js"\)/gm)]
    .map((m) => [m[1], m[2]]);
  assert.ok(mapped.length >= 7, `语言表只解析出 ${mapped.length} 条，格式变了`);
  for (const [tag, file] of mapped) {
    assert.ok(GLOBAL_LANGUAGE_TAGS.includes(tag), `${tag} 不是产品支持的语言`);
    const p = require.resolve(`monaco-editor/esm/nls.messages.${file}.js`);
    const text = readFileSync(p, "utf8");
    assert.match(text, /globalThis\._VSCODE_NLS_MESSAGES\s*=\s*\[/,
      `${file} 语言包的形状变了——它的副作用不再是给那个全局赋值，整套就静默失效`);
  }
  // 除了英语，8 种语言一个都不能漏。
  const covered = new Set(mapped.map(([t]) => t));
  for (const tag of GLOBAL_LANGUAGE_TAGS) {
    if (tag === "en") continue;
    assert.ok(covered.has(tag), `${tag} 没有对应的 monaco 语言包，那种语言下菜单会留在英文`);
  }
});

test("语言包里真的有翻好的菜单项，索引也对得上", () => {
  // 光有文件不够：Monaco 的菜单是 `localize2(1065, "Go to Definition")`，1065 是**索引**。
  // 索引对不上就会显示成别的句子，比英文更糟。拿真包真查一次。
  const zh = readFileSync(require.resolve("monaco-editor/esm/nls.messages.zh-cn.js"), "utf8");
  const arr = JSON.parse(zh.slice(zh.indexOf("["), zh.lastIndexOf("]") + 1));
  const goto = readFileSync(
    require.resolve("monaco-editor/esm/vs/editor/contrib/gotoSymbol/browser/goToCommands.js"), "utf8");
  const idx = Number(/localize2\((\d+), "Go to Definition"\)/.exec(goto)?.[1]);
  assert.ok(Number.isFinite(idx), "monaco 那边「转到定义」的写法变了，这条守卫失去意义");
  assert.equal(arr[idx], "转到定义", "语言包的索引和 monaco 的调用对不上");
});

test("灌语言包必须排在 main.js 之前——晚一步菜单就已经是英文的了", () => {
  // 菜单标题在 monaco 模块**求值时**就定死了，而 main.js 顶上就是 `import * as monaco`。
  const boot = src("../src/boot.jsx");
  const nls = boot.indexOf("await loadMonacoNls()");
  const main = boot.indexOf('await import("./main.js")');
  assert.ok(nls > 0 && main > 0, "boot.jsx 里找不到这两步");
  assert.ok(nls < main, "语言包灌在 main.js 之后了——monaco 那时已经把英文标题注册完了");
  // main.js 必须仍然是**动态** import：静态依赖在依赖遍历时直接求值，不会等上面那个 await。
  assert.doesNotMatch(boot, /^import .*from "\.\/main\.js"/m,
    "main.js 变成静态 import 了——它不会等语言包，菜单又会是英文");
});

test("切语言时说清楚编辑器那套菜单要重启才跟上", () => {
  // 不说的话，用户看到软件都中文了、右键菜单还是英文，会以为切换没生效。
  const main = src("../src/main.js");
  assert.match(main, /feature\.settings\.localeEditorRestart/,
    "切语言的提示里没说编辑器菜单要重启才跟上");
  const i18n = src("../src/i18n.js");
  for (const lang of ["The editor", "编辑器自带"]) {
    assert.ok(i18n.includes(lang), `localeEditorRestart 少了一种语言的文案：${lang}`);
  }
});

test("真灌一次：全局表被填上，索引 1065 就是「转到定义」", async () => {
  // 前面几条守的是「接线对不对」；这条是**真跑**：调 loadMonacoNls，然后去看 monaco 待会儿
  // 会读的那个全局。语言包的形状变了、动态 import 被打包器改坏了、索引对不上，这里都会红。
  delete globalThis._VSCODE_NLS_MESSAGES;
  delete globalThis._VSCODE_NLS_LANGUAGE;
  assert.equal(await loadMonacoNls("zh-CN"), "zh-CN");
  assert.ok(Array.isArray(globalThis._VSCODE_NLS_MESSAGES), "全局表没被填上，菜单还会是英文");
  assert.equal(globalThis._VSCODE_NLS_LANGUAGE, "zh-cn");
  // monaco 那边写的是 localize2(<索引>, "Go to Definition")，索引现取现用，不写死。
  const goto = readFileSync(
    require.resolve("monaco-editor/esm/vs/editor/contrib/gotoSymbol/browser/goToCommands.js"), "utf8");
  const idx = Number(/localize2\((\d+), "Go to Definition"\)/.exec(goto)?.[1]);
  assert.equal(globalThis._VSCODE_NLS_MESSAGES[idx], "转到定义",
    "灌进去的表里，「转到定义」不在 monaco 要读的那个位置上");
  delete globalThis._VSCODE_NLS_MESSAGES;
  delete globalThis._VSCODE_NLS_LANGUAGE;
});
