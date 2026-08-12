/**
 * 渲染层共用的两个 HTML 转义函数。
 *
 * # 为什么要单独一个文件
 *
 * `escapeHtml` 原来是 DOM 实现（`main.js` 里的 `document.createElement("div")` +
 * `textContent` + 读 `innerHTML`）。那份实现在浏览器里完全正确，但 `node --test` 里没有
 * DOM，所以**任何 import 它的模块都会加载失败**——`diff-view.js` 和 `language.js` 都要用它，
 * 它们的测试也就跟着一起废掉。这里把它改写成纯字符串实现，两个模块才测得动。
 *
 * 同时它必须是一片独立的叶子：`language.js` 要用转义，`diff-view.js` 既要用转义又要用
 * `language.js`。转义函数如果挂在 `diff-view.js` 上，两个模块就成了循环依赖。
 * 依赖方向固定为 `escape.js` ← `language.js` ← `diff-view.js`。
 *
 * 纯函数、无 DOM、无 I/O、无 import，所以测试可以直接 `import` 它。
 */

/**
 * 转义**文本节点**内容。
 *
 * 引号是故意不转义的——这不是属性转义器，被它替换掉的那份 DOM 实现同样把引号原样输出。
 * 属性值请用 {@link escapeAttr}。
 *
 * 三条规则是从 HTML 片段序列化算法逐条搬过来的，缺一条就会让 319 个调用点悄悄改变输出：
 *
 * 1. `&` `<` `>` 转成实体——这条谁都记得。
 * 2. **U+00A0 转成 `&nbsp;`**。`innerHTML` 会把不换行空格序列化成实体；只做三次 replace 的
 *    朴素版本不会。丢掉这条，每一段带不换行空格的粘贴内容、模型输出、文件行都会吐出裸的
 *    U+00A0：肉眼几乎看不出来，但 `innerHTML` 字符串变了。
 * 3. **`null` 转成空串**。`textContent = null` 得到的是 `""`（WebIDL 的可空 DOMString 把
 *    `null`/`undefined` 都归一到 null，再清空子节点）；而 `String(null)` 得到的是 `"null"`。
 *    丢掉这条，本该空白的地方会在界面上显示出字面量 **null**。
 *
 * replace 的顺序是有意义的：`&` 必须第一个跑，否则我们为 `&nbsp;` 注入的那个 `&` 会被
 * 二次转义成 `&amp;nbsp;`。
 */
export function escapeHtml(s) {
  return String(s == null ? "" : s)
    .replace(/&/g, "&amp;")
    .replace(/\u00a0/g, "&nbsp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** 转义**带引号的属性值**。从 main.js 原样搬过来，一个字节都没动。 */
export function escapeAttr(s) {
  return String(s == null ? "" : s)
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
