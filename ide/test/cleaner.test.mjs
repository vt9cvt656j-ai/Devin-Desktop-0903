// 旧版残留清理器 + 「新版被运行时盖回旧版」的那道闸。
//
// 用户的原话是「感觉一直是旧版，模型图标没变成真的，我一直感觉是旧版缓存」。
// 把每一层查完之后，真凶不是缓存：一个 2026-06-21 装的第三方中文语言包，
// 每次启动把 155 条界面文案整份覆盖回六月那版——包括产品名、助手名、
// 整个设置页的叙事。清任何缓存都没用，因为覆盖发生在运行时。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load, fnSource, CODE } from "./helpers/source.mjs";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
/** Rust 侧的源码不经过 acorn，直接读文件。 */
const fnSource_rust = () => readFileSync(join(HERE, "../src-tauri/src/cleanup.rs"), "utf8");

test("扩展提供的语言包只补空缺，绝不覆盖应用自带的文案", () => {
  // 这是那个症状的机制修法。默认 overwrite:true 会让第三方词典整份压过内置词典，
  // 于是界面文案被**永久冻结在那个包被写出来的那一版**：以后每次升级，新写的文案
  // 都会在启动时被按回去，而且不可能靠清缓存解决。
  assert.match(CODE, /registerLocale: \(locale, dict\) => registerLocale\(locale, dict, \{ overwrite: false \}\)/,
    "扩展的 registerLocale 又变回整份覆盖了——装过语言包的用户会永远停在旧文案");
});

test("覆盖语义本身：内置的赢，扩展只填空", () => {
  // 光钉住调用点不够，还得证明 overwrite:false 真是「内置优先」而不是反过来。
  // registerLocale 住在 i18n.js，不在 main.js —— fnSource 只读 main.js，这里直接抠源文件。
  const i18n = readFileSync(join(HERE, "../src/i18n.js"), "utf8");
  const m = i18n.match(/export function registerLocale[\s\S]*?\n}/);
  assert.ok(m, "抠不出 registerLocale 的源码，这条用例等于没跑");
  const src = m[0];
  const translations = { "zh-CN": { title: "Mr. Day One", only: "新的" } };
  const reg = new Function("EN", "translations", "isSupportedLocale", "coerceSupportedLocale",
    "let textAliasCache=null;" + src.replace("export function", "return function"))
    ({ title: "EN" }, translations, () => true, (x) => x);

  reg("zh-CN", { title: "Michael IDE", extra: "扩展补的" }, { overwrite: false });
  assert.equal(translations["zh-CN"].title, "Mr. Day One", "扩展把内置文案盖掉了");
  assert.equal(translations["zh-CN"].extra, "扩展补的", "扩展补的空缺没进去，语言包就白装了");

  // 反面对照：overwrite:true 确实会盖掉——证明上面那条断言是有区分度的
  reg("zh-CN", { title: "Michael IDE" }, { overwrite: true });
  assert.equal(translations["zh-CN"].title, "Michael IDE", "overwrite:true 竟然没盖掉，这条用例失去了参照");
});

test("自动清扫的判据是「有更高版本」，不是键名", () => {
  // 按名字猜「这像是缓存」会把 michael_token（登录凭据）、chat-sessions（聊天镜像）
  // 一起删掉。判据必须是结构性证据：同一个基名存在更高版本 ⇒ 写新版的那段代码
  // 已经在用新的了，旧的再没人读。
  const store = new Map(Object.entries({
    "michael-ide.i18n-adhoc.zh-CN.v5": "旧",
    "michael-ide.i18n-adhoc.zh-CN.v6": "新",
    "michael-ide.mcp-registry.v3": "现役",
    "michael-ide.skill-registry.v1": "现役",
    "michael_token": "凭据",
    "michael-ide.chat-sessions": "聊天",
    "michael-ide.ctx-choice.v1": "旧",
    "michael-ide.ctx-choice.v2": "新",
    "michael_ctx_seen_v1": "现役",
  }));
  const localStorage = {
    get length() { return store.size; },
    key: (i) => [...store.keys()][i],
    getItem: (k) => store.get(k) ?? null,
    setItem: (k, v) => store.set(k, v),
    removeItem: (k) => store.delete(k),
  };
  const sweep = load("_sweepSupersededStorage", { localStorage, appPackage: { version: "0.4.13" }, console: { info() {} } });
  const removed = sweep().sort();

  assert.deepEqual(removed, ["michael-ide.ctx-choice.v1", "michael-ide.i18n-adhoc.zh-CN.v5"],
    "清掉的不是「被更高版本取代的那些」");
  // 现役的和没有版本后缀的一个都不能少
  for (const keep of ["michael-ide.i18n-adhoc.zh-CN.v6", "michael-ide.mcp-registry.v3",
                      "michael-ide.skill-registry.v1", "michael_token",
                      "michael-ide.chat-sessions", "michael-ide.ctx-choice.v2", "michael_ctx_seen_v1"]) {
    assert.ok(store.has(keep), `${keep} 被误删了`);
  }
  // 水位线是这套机制唯一自己写的键
  assert.equal(store.get("michael-ide.cleaner.last-run-version"), "0.4.13");
});

test("只有一个版本的键永远不动——哪怕它看起来很旧", () => {
  // 这是上一条的关键边界：孤零零一个 .v1 说明它就是现役的，不是残留。
  const store = new Map(Object.entries({
    "michael-ide.learner-model.v1": "现役",
    "michael-ide.browser.pref.v1": "现役",
    "lsp_install_prompted_v1": "现役",
  }));
  const localStorage = {
    get length() { return store.size; },
    key: (i) => [...store.keys()][i],
    getItem: (k) => store.get(k) ?? null,
    setItem: (k, v) => store.set(k, v),
    removeItem: (k) => store.delete(k),
  };
  const sweep = load("_sweepSupersededStorage", { localStorage, appPackage: { version: "0.4.13" }, console: { info() {} } });
  assert.deepEqual(sweep(), [], "没有更高版本的键被当成残留删掉了");
  assert.equal(store.size, 4, "现役键被动过（含水位线应为 4）");
});

test("存储被禁用时清扫不能挡住启动", () => {
  // 隐私模式 / 配额满的时候 localStorage 会抛。清扫是锦上添花，绝不能因此让应用起不来。
  const boom = new Proxy({}, { get() { throw new Error("storage disabled"); } });
  const sweep = load("_sweepSupersededStorage", { localStorage: boom, appPackage: { version: "x" }, console: { info() {} } });
  assert.doesNotThrow(() => sweep());
});

test("清理器的界面入口接上了", () => {
  assert.match(CODE, /\{ label: "清理旧版残留…", icon: "i-trash", action: \(\) => openCleanupDialog\(\) \}/,
    "工具菜单里没有清理器入口");
  assert.match(CODE, /id: "tools\.cleanup"[\s\S]{0,120}openCleanupDialog\(\)/,
    "命令面板里搜不到清理器");
  assert.match(CODE, /try \{ _sweepSupersededStorage\(\); \} catch/,
    "启动时没有跑自动清扫");
  // 两档必须分开问：auto 那档一次确认，manual 那档（旧版副本＝回滚点）单独再问
  const dlg = fnSource("openCleanupDialog", { code: true });
  assert.match(dlg, /i\.tier === "auto"/);
  assert.match(dlg, /_confirmDialog\("清理旧版副本"[\s\S]{0,80}true\)/,
    "旧版副本那档没有单独的、标红的确认——那是回滚点，清了回不去");
});

test("清理器说得清「清缓存不会让界面变新」", () => {
  // 用户会带着「我怀疑是缓存」的预期来点这个按钮。如果不当场说明白，
  // 他清完发现界面没变，会认为清理器坏了——而真正的原因在别处。
  const rs = fnSource_rust();
  assert.match(rs, /清它不会改变界面新旧|不会让界面变新/,
    "网络缓存那一项没有说清「清了也不会让界面变新」");
});

