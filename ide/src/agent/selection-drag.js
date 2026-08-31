/**
 * 编辑器里选中的代码拖进输入框。
 *
 * 这里只做纯粹的那一半：把「选了哪个文件的哪几行」压成输入框那枚片要显示的**标签**，
 * 以及发送时展开成的**正文**。行号、代码、语言全部从参数进，没有 DOM、没有 Monaco，
 * 所以测试是真跑出来的，不是比对源码文本。
 *
 * 鼠标那一半留在 main.js（要 getBoundingClientRect、要 Monaco 的命中测试），
 * 判据和 _wireTreeDragToComposer 一致。
 */

/**
 * 片上显示的标签：`文件名:起-止`。单行不写范围——`a.py:8-8` 只是噪音。
 *
 * 只取最后一段文件名，和输入框里其它每一种片同一条规则（完整路径进 tooltip）。
 */
export function selectionLabel(rel, startLine, endLine) {
  const name = String(rel || "").split("/").filter(Boolean).pop() || String(rel || "");
  const a = Math.max(1, Number(startLine) || 1);
  const b = Math.max(a, Number(endLine) || a);
  return b > a ? `${name}:${a}-${b}` : `${name}:${a}`;
}

/**
 * 发送时展开成的正文：一句出处 + 一个代码块。
 *
 * 三件事是有意为之：
 *  ① **围栏按内容算**——取代码里最长的一串反引号再加一位。选区本身含 ``` 的话（markdown、
 *     文档字符串里嵌代码块，都很常见），固定三个反引号会把块提前关掉，后半段代码变成正文。
 *  ② **带出处**——模型要知道这段是哪个文件的哪几行才能改它；只给一段裸代码，它只能猜。
 *  ③ **截断要明说**——超过上限只带前面一段，并写清共多少行、带了多少行。默默少给几行会让
 *     模型以为自己看到了全部，然后基于半段代码下结论。
 */
export function selectionText(opts) {
  // 解构的默认值只兜 undefined，兜不住 null —— 而这一层跑在拖放路径上，调用方一次取空
  // 就会在鼠标松开的那一刻抛出去。所以先自己兜一次。
  const { rel, lang, startLine, endLine, code, maxLines = 200, maxChars = 8000 } = opts || {};
  const all = String(code ?? "").split("\n");
  let kept = all.slice(0, Math.max(1, maxLines));
  let body = kept.join("\n");
  if (body.length > maxChars) {
    body = body.slice(0, Math.max(0, maxChars));
    kept = body.split("\n");
    body = kept.join("\n");
  }
  const longest = (body.match(/`+/g) || []).reduce((n, s) => Math.max(n, s.length), 0);
  const fence = "`".repeat(Math.max(3, longest + 1));
  const a = Math.max(1, Number(startLine) || 1);
  const b = Math.max(a, Number(endLine) || a);
  const where = b > a ? `第 ${a}-${b} 行` : `第 ${a} 行`;
  const dropped = all.length - kept.length;
  const tail = dropped > 0 ? `\n（选区共 ${all.length} 行，这里只带了前 ${kept.length} 行）` : "";
  return `\n引用 ${rel} ${where}：\n${fence}${lang || ""}\n${body}\n${fence}${tail}\n`;
}
