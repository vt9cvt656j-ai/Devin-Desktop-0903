/**
 * 一个文件"是什么语言"——只回答问这个问题的那两处：diff 卡片的语言角标，和 diff 卡片的
 * `data-lang`。
 *
 * # 这不是编辑器的语言解析器
 *
 * `main.js` 里的 `extLang` 回答的是另一个问题：它返回**真正的 Monaco language id**
 * （`"javascript"` / `"ini"` / `"plaintext"`），喂给编辑器 model 和 `monaco.editor.colorize`。
 * 这个模块返回的是**短 token**（`"js"` / `"ts"` / `"toml"`）——它们是 CSS 类名后缀和角标查表
 * 的 key。`languageIdForPath` 的结果会再经过 `monacoLang()` 才轮到 Monaco 看见。
 *
 * 两者在无扩展名文件上的结论**故意不一致**：`extLang("Makefile")` 是 `"ini"`（这样编辑器能高亮），
 * 而 `langKey("Makefile")` 是 `"default"`（这样角标显示 FILE）。`styles/app.css` 里只有 11 个
 * 语言 token 加一个 `default` 有对应的 `.atc-lang-badge--*` 规则，把两者统一会渲染出一堆没有
 * 样式的裸角标。别去"修"这个不一致。
 *
 * 纯数据 + 纯函数，无 DOM，测试直接 import。
 */
import { escapeAttr, escapeHtml } from "./escape.js";

/**
 * 扩展名/语言别名 → 角标 token。
 *
 * 冻结是有原因的：`langKey` 的默认参数闭包到这个绑定上，导出一个可变对象等于允许任何
 * import 方永久改写整个会话的角标渲染，而且出问题时堆栈里根本不会指到这里。
 * 浅冻结就够——所有值都是字符串。
 */
export const AGENT_LANG_MAP = Object.freeze({
  py: "py", python: "py", js: "js", javascript: "js", jsx: "js", ts: "ts", typescript: "ts",
  tsx: "ts", html: "html", htm: "html", css: "css", scss: "css", less: "css", rs: "rs",
  rust: "rs", go: "go", sh: "sh", bash: "sh", shell: "sh", zsh: "sh", json: "json",
  sql: "sql", md: "md", markdown: "md",
});

/** 角标 token → 显示文字。只有 `default` 不是把 key 直接大写。 */
export const LANG_LABELS = Object.freeze({
  py: "PY", js: "JS", ts: "TS", html: "HTML", css: "CSS", rs: "RS", go: "GO", sh: "SH",
  json: "JSON", sql: "SQL", md: "MD", cmd: "CMD", default: "FILE",
});

/**
 * 路径或语言名 → 角标 token。
 *
 * 只按 `.` 切分，不做 basename 切分，所以 `"src/app.tsx"` 能解析纯粹是因为扩展名正好是最后
 * 一段。别为了对齐 `extLang` 而加上剥 `/` 的逻辑。
 *
 * 第二个分支 `langMap[s]`（整名命中）对内置表永远不会触发——无点输入时 `ext` 已经等于
 * `s.toLowerCase()`，而表里每个 key 都是小写且不含点。保留它是因为调用方传进来的表可以
 * 合理地按整名建 key，比如 `"go.mod"`。
 *
 * `Object.hasOwn` 两道判断是**修 bug**，不是搬运：原实现直接穿透到 `Object.prototype`，
 * 于是 `langKey("x.constructor")` 会返回 Object 构造函数本身，调用方紧接着抛
 * `key.toUpperCase is not a function`。见 {@link langBadge}。
 */
export function langKey(pathOrLang, { langMap = AGENT_LANG_MAP } = {}) {
  const s = String(pathOrLang == null ? "" : pathOrLang);
  const ext = (s.split(".").pop() || "").toLowerCase();
  if (Object.hasOwn(langMap, ext)) return langMap[ext];
  if (Object.hasOwn(langMap, s)) return langMap[s];
  return "default";
}

/**
 * diff 卡片的 `data-lang` token。
 *
 * 认不出的扩展名**原样透传**（`"Cargo.toml"` → `"toml"`），而不是回落到 `"plaintext"`——
 * 正是这一点让下游的 `monacoLang()` 还有机会做真正的解析。这个返回值不是 Monaco language id。
 */
export function languageIdForPath(filePath, { langMap = AGENT_LANG_MAP } = {}) {
  const ext = String(filePath == null ? "" : filePath).split(".").pop().toLowerCase();
  return Object.hasOwn(langMap, ext) ? langMap[ext] : ext;
}

/**
 * 角标 `<span>`。
 *
 * `labels` 是**叠加**在 {@link LANG_LABELS} 之上，不是替换。这一点是可观测的：如果是替换，
 * `langBadge("unknown.file", { labels: { custom: "X" } })` 得到 key `"default"`、查不到 label、
 * 于是渲染成 `DEFAULT`，而 CSS 里认的是 `.atc-lang-badge--default` + 文案 FILE。其他 key 在
 * 替换语义下"碰巧"没事，只是因为它们的 label 恰好等于 key 的大写形式；`default → FILE` 是
 * 唯一的例外。
 *
 * 两处转义在原实现里是没有的。原来 key 只可能是内置表里 12 个 `/^[a-z]+$/` 的编译期常量，
 * 所以不可利用；但**加上 `langMap` 参数之后这个约束就没了**——key 变成调用方的表说了算，
 * 而它正好落在 `class="...--${key}"` 里面，一个形如 `x" onmouseover=...` 的值就能从属性里
 * 逃出去。返回值最终是要进 `.innerHTML` 的。对内置表能产生的每一个值，两个转义函数都是恒等
 * 变换，所以线上两个调用点的输出逐字节不变。
 */
export function langBadge(pathOrLang, { langMap = AGENT_LANG_MAP, labels = {} } = {}) {
  const key = langKey(pathOrLang, { langMap });
  const table = { ...LANG_LABELS, ...labels };
  const text = table[key] || String(key).toUpperCase();
  return `<span class="atc-lang-badge atc-lang-badge--${escapeAttr(key)}">${escapeHtml(text)}</span>`;
}
