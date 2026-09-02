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

/**
 * 输入框里那枚片**发送出去时**变成的短记号：`@code:<相对路径>#<起>-<止>`。
 *
 * 为什么不是把代码直接拼进正文：那样气泡里就是一大坨代码，而用户要的是"发出去的也是那枚片"。
 * 和 `@element:<id>` 同一条路子——可见文本和历史里只留这个短记号，真正的代码在**发送期**
 * 展开进上下文。区别是这里不需要另存一份快照：路径和行号就够把那几行从磁盘上取回来。
 *
 * 单行也写成 `#286-286`：标签那边可以省掉范围，记号不行——省了就得在解析处再分一支。
 */
export function selectionToken(rel, startLine, endLine) {
  const a = Math.max(1, Number(startLine) || 1);
  const b = Math.max(a, Number(endLine) || a);
  return `@code:${encodeRelForToken(rel)}#${a}-${b}`;
}

// 提及扫描是按**空白**切的（`@([^\s]+)`），所以记号里一个空格都不能有——路径里带空格的项目
// 很常见（用户自己那个项目就叫「cursor 反代」）。空白按 %XX 编码，`%` 自己也编，这样是可逆的。
const encodeRelForToken = (rel) => String(rel || "")
  .replace(/[\s%]/g, (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase().padStart(2, "0"));
const decodeRelFromToken = (rel) => String(rel || "")
  .replace(/%([0-9A-Fa-f]{2})/g, (_, h) => String.fromCharCode(parseInt(h, 16)));

/** 反过来解析。认不出就返回 null —— 调用方据此放行给别的分支，不许猜。 */
export function parseSelectionToken(token) {
  const m = /^@?code:(.+)#(\d+)-(\d+)$/.exec(String(token || ""));
  if (!m) return null;
  const a = Math.max(1, Number(m[2]) || 1);
  const b = Math.max(a, Number(m[3]) || a);
  return { rel: decodeRelFromToken(m[1]), startLine: a, endLine: b };
}

/**
 * 从整份文件里切出记号指的那几行。
 *
 * 行号按 1 起算、两端都含。越界不报错只取交集：文件在拖进来之后被改短了是常事，
 * 这时给出剩下的部分远好过整条丢掉。
 */
export function sliceLines(content, startLine, endLine) {
  const all = String(content ?? "").split("\n");
  const a = Math.max(1, Number(startLine) || 1);
  const b = Math.max(a, Number(endLine) || a);
  return all.slice(a - 1, b).join("\n");
}
