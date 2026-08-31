// Monaco 自己那套界面文案（右键菜单、查找框、悬浮提示、命令面板）跟着用户选的语言走。
//
// # 为什么必须在 import monaco **之前**做
//
// 这份 npm 包发的 esm 已经是"编译过"的形态：菜单项写成 `localize2(1065, "Go to Definition")`
// —— 数字是索引，英文只是兜底。索引查的是 `globalThis._VSCODE_NLS_MESSAGES`，而这些
// localize 调用发生在**模块求值时**（动作和菜单项在那时注册，标题当场定死）。
// 所以晚一步设这个全局，菜单就已经是英文的了，之后再改全局也没用。
//
// boot.jsx 里 main.js 本来就是 `await import(...)` 进来的，把这一步排在它前面就成立。
// 换成静态 import 是不成立的：同一层的静态依赖会在依赖遍历时直接求值，不会等这边的 await。
//
// # 语言表
//
// 左边是产品支持的 8 种语言（src/locales.js），右边是 monaco 发的语言包文件名。
// en 不列：Monaco 的兜底本来就是英文，设了反而多下一个包。
// pt 指到 pt-br —— 它是 Monaco 唯一的葡语包，而且是个几乎空的表（大多数条目是 null），
// 查不到时 lookupMessage 会退回英文兜底，不会炸。
import { coerceSupportedLocale, systemPreferredLocale } from "./locales.js";

const MONACO_NLS = {
  "zh-CN": () => import("monaco-editor/esm/nls.messages.zh-cn.js"),
  ja: () => import("monaco-editor/esm/nls.messages.ja.js"),
  ko: () => import("monaco-editor/esm/nls.messages.ko.js"),
  de: () => import("monaco-editor/esm/nls.messages.de.js"),
  es: () => import("monaco-editor/esm/nls.messages.es.js"),
  pt: () => import("monaco-editor/esm/nls.messages.pt-br.js"),
  ru: () => import("monaco-editor/esm/nls.messages.ru.js"),
};

/** Monaco 那边用的语言标签（`_VSCODE_NLS_LANGUAGE`），和上表一一对应。 */
const MONACO_TAG = { "zh-CN": "zh-cn", ja: "ja", ko: "ko", de: "de", es: "es", pt: "pt-br", ru: "ru" };

/**
 * 当前语言。判据和 initLocale 完全一致（存的值 → 系统偏好 → 收敛到支持的 8 种之一），
 * 但**不依赖 initLocale 已经跑过**：这一步排在整个应用之前。
 */
export function monacoNlsLocale(storage = globalThis.localStorage) {
  let saved = "";
  try { saved = storage?.getItem?.("michael-ide-locale") || ""; } catch { saved = ""; }
  return coerceSupportedLocale(saved || systemPreferredLocale());
}

/**
 * 把对应的语言包灌进去。失败一律吞掉——语言包没加载上只是菜单还是英文，
 * 不能让整个应用起不来。
 */
export async function loadMonacoNls(locale = monacoNlsLocale()) {
  const load = MONACO_NLS[locale];
  if (!load) return "";                      // en 或没覆盖到的语言：留着英文兜底
  try {
    await load();                            // 语言包的副作用就是给全局赋值
    globalThis._VSCODE_NLS_LANGUAGE = MONACO_TAG[locale] || "";
    return locale;
  } catch { return ""; }
}
