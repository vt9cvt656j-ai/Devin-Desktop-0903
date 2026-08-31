/**
 * 工具卡片里那块内联 diff：行数统计、HTML 渲染、Monaco 着色。
 *
 * 从 `main.js` 原样搬出来。下面两个"看着像 bug"的行为是**故意保留**的：测试没覆盖它们，
 * 而改掉任何一个都会让产品里每一处非新建文件的 diff 渲染结果发生变化，同时测试全绿——
 * 这种改动值不值得做是另一回事，要做就单独一个提交配单独的测试。
 *
 * - **60 行上限在非新建分支下可能渲染出 63 行。** `rendered < cap` 只在循环头判一次，但
 *   一轮迭代最多吐 4 行（两行回填上下文 + 一行删 + 一行增）。测试只覆盖了新建分支，那边
 *   `rendered++` 写在 for 头里，计数是精确的。
 * - **第一个 hunk 会重复输出一行上下文。** `i=1` 时 `i - lastShown === 2`（`lastShown` 还是 -1）
 *   打印第 2 行；`i=2` 时 `lastShown < 0 && i > 0` 的回填又把第 1、2 行打印一遍。
 *
 * 另外两处也是靠人眼保住的：`if (html)` 的守卫（着色结果剥壳后为空时保留原文），以及
 * `^<div>` / `</div>$` 的**锚定**剥壳——Monaco 每行吐一个 `<div>`，不锚定的 replace 会把嵌套
 * span 打烂。
 */
import { escapeAttr, escapeHtml } from "./escape.js";
import { languageIdForPath } from "./language.js";

// diff-view.test.mjs 从这里 import 两个转义函数；它们真正的家在 escape.js。
export { escapeAttr, escapeHtml } from "./escape.js";

/**
 * 真实的增/删**行数**（不是"新文件总行数 / 旧文件总行数"）。
 *
 * 用行的多重集差：单纯挪了位置的行算没变，改过的行算一删一增——接近用户看 diff 时的直觉，
 * 而且是 O(n)。全新文件则每行都算新增。
 */
export function diffStat(oldText, newText) {
  const oldL = oldText ? oldText.split("\n") : [];
  const newL = newText ? newText.split("\n") : [];
  const counts = new Map();
  for (const l of oldL) counts.set(l, (counts.get(l) || 0) + 1);
  for (const l of newL) counts.set(l, (counts.get(l) || 0) - 1);
  let added = 0, removed = 0;
  for (const v of counts.values()) {
    if (v < 0) added += -v;       // 新文件里出现次数更多 → 新增
    else if (v > 0) removed += v; // 旧文件里出现次数更多 → 删除
  }
  return { added, removed };
}

/**
 * 把两份文本对齐成一串操作：`{ t: "ctx"|"del"|"add", o, n, s }`
 * （o/n = 旧/新文件里的 0-based 行号，取不到为 -1；s = 该行文本）。
 *
 * # 为什么必须做对齐
 *
 * 渲染这块 diff 的循环原来是**按同一个下标配对**（`oldL[i]` vs `newL[i]`）。这在"只改了
 * 某几行"时凑合，但只要有一行插入/删除，后面全部错位——实测在 40 行文件最前面插一行
 * import：徽章（diffStat 用行的多重集，算得对）显示 `+1`，而正下方那块 diff 画出
 * **30 增 30 删**。同一次写入，两个数字差 30 倍，用户不知道该信哪个。
 *
 * # 做法与规模守卫
 *
 * 先掐掉公共前后缀（真实编辑绝大多数是"中间改一小段"，这一步就把问题缩得很小），
 * 再对剩下的中段跑 LCS。中段太大时（乘积超过阈值）退回逐行配对——那是旧行为，
 * 不好但有界；这块 diff 只渲染 60 行，为一个几千行的整体重写去算 O(n·m) 不值得。
 */
function alignLines(oldL, newL) {
  let s = 0;
  while (s < oldL.length && s < newL.length && oldL[s] === newL[s]) s++;
  let e = 0;
  while (e < oldL.length - s && e < newL.length - s
         && oldL[oldL.length - 1 - e] === newL[newL.length - 1 - e]) e++;

  const ops = [];
  for (let i = 0; i < s; i++) ops.push({ t: "ctx", o: i, n: i, s: oldL[i] });

  const oMid = oldL.slice(s, oldL.length - e);
  const nMid = newL.slice(s, newL.length - e);
  const BUDGET = 4_000_000;   // 约 2000×2000，够覆盖真实编辑；再大就不值得算
  if (oMid.length * nMid.length > BUDGET) {
    // 退回旧的逐行配对，但只作用在中段（前后缀已经对齐了，比原来准）。
    const m = Math.max(oMid.length, nMid.length);
    for (let i = 0; i < m; i++) {
      const o = i < oMid.length ? oMid[i] : undefined;
      const n = i < nMid.length ? nMid[i] : undefined;
      if (o !== undefined && o === n) { ops.push({ t: "ctx", o: s + i, n: s + i, s: o }); continue; }
      if (o !== undefined) ops.push({ t: "del", o: s + i, n: -1, s: o });
      if (n !== undefined) ops.push({ t: "add", o: -1, n: s + i, s: n });
    }
  } else {
    // LCS 表（滚动两行即可，内存 O(min)）——但回溯需要整表，中段已被前后缀夹小，可接受。
    const L = Array.from({ length: oMid.length + 1 }, () => new Uint32Array(nMid.length + 1));
    for (let i = oMid.length - 1; i >= 0; i--) {
      for (let j = nMid.length - 1; j >= 0; j--) {
        L[i][j] = oMid[i] === nMid[j] ? L[i + 1][j + 1] + 1 : Math.max(L[i + 1][j], L[i][j + 1]);
      }
    }
    let i = 0, j = 0;
    while (i < oMid.length && j < nMid.length) {
      if (oMid[i] === nMid[j]) { ops.push({ t: "ctx", o: s + i, n: s + j, s: oMid[i] }); i++; j++; }
      else if (L[i + 1][j] >= L[i][j + 1]) { ops.push({ t: "del", o: s + i, n: -1, s: oMid[i] }); i++; }
      else { ops.push({ t: "add", o: -1, n: s + j, s: nMid[j] }); j++; }
    }
    while (i < oMid.length) { ops.push({ t: "del", o: s + i, n: -1, s: oMid[i] }); i++; }
    while (j < nMid.length) { ops.push({ t: "add", o: -1, n: s + j, s: nMid[j] }); j++; }
  }

  for (let k = 0; k < e; k++) {
    const oi = oldL.length - e + k, ni = newL.length - e + k;
    ops.push({ t: "ctx", o: oi, n: ni, s: oldL[oi] });
  }
  return ops;
}

export function buildDiffView(oldText, newText, filePath) {
  oldText = oldText == null ? "" : String(oldText);
  newText = newText == null ? "" : String(newText);
  filePath = filePath == null ? "" : String(filePath);
  const oldL = oldText ? oldText.split("\n") : [];
  const newL = newText.split("\n");
  const monoLang = languageIdForPath(filePath);
  const isNew = !oldText;

  let h = '';
  // 内层这一格不是装饰：它是"每行都铺到最宽那行"的唯一支点。
  //
  // 外层 .atc-diff 是横向滚动容器，行的宽度 auto = **可视宽**，于是往右一滚，增/删的底色
  // 就只剩最长那一两行有，其余全白（用户实拍两次）。第一版想用单列网格
  // minmax(100%, max-content) 解决——**不成立**：轨道基准是 100%（可视宽），上限是
  // max-content，而轨道只有在容器里还有剩余空间时才会长向上限；容器正好等于可视宽，剩余
  // 空间为 0，于是轨道停在 280px，所有行仍然只有可视宽（浏览器里实测 gridTemplateColumns
  // 就是 "280px"）。
  //
  // 内层 width: max-content（长到最宽那行）+ min-width: 100%（内容窄时也不短于可视宽），
  // 行是块级、宽度 auto，自然铺满内层。实测四种写法只有这一种让每一行都等于 scrollWidth。
  h += `<div class="atc-diff" data-lang="${escapeHtml(monoLang)}"><div class="atc-diff-inner">`;

  const cap = 60;
  let rendered = 0;
  // 走到哪一**源行**为止。cap 数的是渲染出来的 DOM 行，而一处修改会渲染两行（- 和 +），
  // 上下文行也各占一行——rendered 和源行号完全不是一回事。下面的 footer 必须用这个变量
  // 算"还剩多少行没显示"：用 cap 算会少算一半，甚至算出负数导致 footer 干脆不显示
  // （一个 50 行整体重写的文件，rendered 撞到 cap=60 时其实只走到第 30 行，
  //  而 maxLen - cap = -10 → 用户看到半个 diff，下面什么提示都没有）。
  // 声明在 if/else 之外：两个分支都要写它，footer 在两个分支之后读它。
  let stoppedAt = 0;

  if (isNew) {
    for (let i = 0; i < newL.length && rendered < cap; i++, rendered++, stoppedAt = i) {
      h += `<div class="atc-diff-row atc-diff-row--add"><span class="atc-diff-ln">${i + 1}</span><span class="atc-diff-sign">+</span><span class="atc-diff-code" data-raw="${escapeAttr(newL[i])}">${escapeHtml(newL[i])}</span></div>`;
    }
  } else {
    // 按**对齐后的操作序列**渲染，不再按同一个下标配对（见 alignLines 的说明：
    // 那样只要有一行插入就整片错位，40 行文件插一行 import 会画成 30 增 30 删，
    // 而同一张卡上的徽章写着 +1）。
    const ops = alignLines(oldL, newL);
    const row = (cls, sign, ln, text) =>
      `<div class="atc-diff-row atc-diff-row--${cls}"><span class="atc-diff-ln">${ln}</span>`
      + `<span class="atc-diff-sign">${sign}</span>`
      + `<span class="atc-diff-code" data-raw="${escapeAttr(text)}">${escapeHtml(text)}</span></div>`;

    // 只画变更点附近；连续未变的大段折成一行 @@ N unchanged lines @@。
    const CTX = 2;
    const keep = new Set();
    for (let i = 0; i < ops.length; i++) {
      if (ops[i].t === "ctx") continue;
      for (let k = Math.max(0, i - CTX); k <= Math.min(ops.length - 1, i + CTX); k++) keep.add(k);
    }
    let skipped = 0;
    for (let i = 0; i < ops.length && rendered < cap; i++) {
      const op = ops[i];
      stoppedAt = Math.max(op.o, op.n) + 1;
      if (!keep.has(i)) { skipped++; continue; }
      if (skipped > 0) {
        h += `<div class="atc-diff-more">@@ ${skipped} unchanged line${skipped > 1 ? "s" : ""} @@</div>`;
        skipped = 0;
      }
      if (op.t === "ctx") { h += row("ctx", " ", op.n + 1, op.s); rendered++; }
      else if (op.t === "del") { h += row("del", "-", op.o + 1, op.s); rendered++; }
      else { h += row("add", "+", op.n + 1, op.s); rendered++; }
    }
  }

  if (rendered >= cap) {
    // 用真正走到的源行数算，不用 cap（见上面 stoppedAt 那段说明）。
    const remaining = Math.max(oldL.length, newL.length) - stoppedAt;
    if (remaining > 0) h += `<div class="atc-diff-more">… ${remaining} more lines not shown …</div>`;
  }
  h += "</div></div>";   // 关掉 .atc-diff-inner 和 .atc-diff
  return h;
}

/**
 * 用 Monaco 给已经渲染好的 diff 上色。
 *
 * `monaco` 和 `monacoLang` **必须由调用方注入，不能给 `globalThis` 兜底**。它们在 main.js 里
 * 是 ES module import，不挂在 window 上，所以 `globalThis.monaco` 在真实 App 里是 `undefined`；
 * 而下面的 try/catch 会把由此产生的 TypeError 吞掉——症状是高亮悄悄失效、diff 渲染成灰色纯文本、
 * 哪里都不报错。同样，这个模块自己也不能 `import * as monaco from "monaco-editor"`，
 * 否则 `node --test` 加载不了它。所以这里的解构故意不带默认值。
 */
export async function highlightDiffView(container, { monaco, monacoLang } = {}) {
  const diff = container.querySelector(".atc-diff");
  if (!diff) return;
  const lang = diff.dataset.lang;
  if (!lang || lang === "default") return;

  const monoId = monacoLang(lang);
  if (monoId === "plaintext") return;

  const codeEls = diff.querySelectorAll(".atc-diff-code[data-raw]");
  for (const el of codeEls) {
    const raw = el.dataset.raw;
    if (!raw || !raw.trim()) continue;
    try {
      let html = await monaco.editor.colorize(raw, monoId, { tabSize: 2 });
      html = html.replace(/<br\/?>\s*$/, "").replace(/^<div>/, "").replace(/<\/div>$/, "");
      if (html) el.innerHTML = html;
    } catch { /* 保留纯文本 */ }
  }
}
