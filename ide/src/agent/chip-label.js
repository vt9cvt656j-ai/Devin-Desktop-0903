// 「接下来」卡片上显示的短标签。纯函数，从 main.js 搬出来 —— main.js 有行数闸，
// 仓库规矩是「撞线先腾地方，再谈抬线」。

/** 卡片上显示的短标签。**点下去发的仍然是原文，title 里也是原文。**
 *
 * 「接下来」的文案是模型从自己答案里摘的，常常是一整段。实测用户面板宽度（367px）下，
 * 一条 80 字的建议要占 9 行、198px —— 三张卡就是 456px，等于把答案又贴了一遍，
 * 用户原话「不然的话就容易丑陋了」。想要的是 Claude Code 那种一行一个短标签、扫一眼就能挑。
 *
 * 不做定宽截断（那是上一版被否掉的做法：切在半句上，选项名都看不全）。改成**按语义切**：
 * 这些句子几乎都是「标题——解释」的形状，破折号/冒号前那一截本身就是完整的短标签。
 *
 *   没有流式输出——现在整段返回后才打印，体验上就是…   →  没有流式输出
 *   长会话不自动压缩——context 顶到上限就得手动 /clear… →  长会话不自动压缩
 *
 * 切不出来才退回逗号，再不行才硬截。本来就短的一句话原样通过。
 */
export function chipShortLabel(text) {
  const full = String(text || "").trim();
  if (!full) return full;
  const width = (t) => [...t].reduce((n, ch) => n + (/[\u3000-\u9fff\uff00-\uffef]/.test(ch) ? 2 : 1), 0);
  const MAX = 46; // ≈ 23 个汉字，367px 下正好一行出头
  if (width(full) <= MAX) return full;

  // 反引号必须成对：切在一对中间的话，行内代码会从断点一路吃到句尾。
  const balanced = (t) => ((t.match(/`/g) || []).length % 2 === 0);
  const pick = (t) => {
    const v = t.trim().replace(/[，,、；;：:]+$/, "");
    return v && balanced(v) && width(v) >= 6 ? v : null;
  };

  // 一级：标题/解释的分界。二级：分句。都按**第一个**出现的位置切。
  for (const re of [/[\u2014\u2015\u2500]{1,2}|[:：]|[。！？!?]|\n/, /[，,；;]/]) {
    const m = full.match(re);
    if (!m) continue;
    const head = pick(full.slice(0, m.index));
    if (head && width(head) <= MAX) return head;
  }
  // 都切不出来：按显示宽度硬截，收省略号。全文仍在 title 和点击发送的内容里。
  let out = "";
  for (const ch of full) {
    if (width(out + ch) > MAX - 1) break;
    out += ch;
  }
  out = out.replace(/[，,、；;：:\s`]+$/, "");
  if (!balanced(out)) out = out.replace(/`[^`]*$/, "").trim();
  return out + "\u2026";
}
