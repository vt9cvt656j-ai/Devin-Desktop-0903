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

/**
 * 往工具卡右侧那个状态徽章 `.atc-result` 里写文案。
 *
 * **不能直接 `res.textContent = label`**：`.atc-result` 是 flex 容器（spinner / svg / diffstat
 * 都靠它对齐），而 `text-overflow: ellipsis` 只作用于**块容器**——裸文本在 flex 里是匿名
 * flex 项，规则套不上去。于是那条 CSS 写着却恒不生效：实测一句 405px 的失败原因塞进 202px
 * 的徽章（它最宽只有行宽的 40%），被硬切在半个汉字上，既没有省略号提示「后面还有」，
 * 也没有 tooltip 能看全。用户看到的是一句读不通的残句，根本不知道自己漏了内容。
 *
 * 包进 `.atc-result__t`（它本身带正确的 `min-width:0 / overflow / text-overflow`）省略号就
 * 生效了，再把全文放进 `title` 供悬停。两件事一起做才完整：省略号告诉他「被截了」，
 * title 让他能读到被截掉的部分。
 *
 * 放在 escape.js 是因为它和这里的两个转义函数同族——都是渲染层的叶子工具，纯 DOM 操作、
 * 不读全局、不依赖 main.js 的任何状态；而 main.js 有一条尺寸闸，这段本来就该住在模块里。
 */
export function setBadgeText(el, text) {
  try {
    if (!el) return false;
    const s = String(text ?? "");
    el.textContent = "";
    const doc = el.ownerDocument || globalThis.document;
    const span = doc?.createElement?.("span");
    if (!span) { el.textContent = s; return false; }
    span.className = "atc-result__t";
    span.textContent = s;
    el.appendChild(span);
    // 短到根本不会被截的就别挂 title：每个徽章都带 tooltip 反而是噪音。
    if (s.length > 12) el.title = s; else el.removeAttribute?.("title");
    return true;
  } catch { return false; }
}
