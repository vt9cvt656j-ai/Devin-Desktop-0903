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

export function buildDiffView(oldText, newText, filePath) {
  oldText = oldText == null ? "" : String(oldText);
  newText = newText == null ? "" : String(newText);
  filePath = filePath == null ? "" : String(filePath);
  const oldL = oldText ? oldText.split("\n") : [];
  const newL = newText.split("\n");
  const monoLang = languageIdForPath(filePath);
  const isNew = !oldText;

  let h = '';
  h += `<div class="atc-diff" data-lang="${escapeHtml(monoLang)}">`;

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
    const maxLen = Math.max(oldL.length, newL.length);
    let lastShown = -1;
    for (let i = 0; i < maxLen && rendered < cap; i++, stoppedAt = i) {
      const oLine = i < oldL.length ? oldL[i] : undefined;
      const nLine = i < newL.length ? newL[i] : undefined;

      if (oLine !== undefined && nLine !== undefined && oLine === nLine) {
        if (i - lastShown === 2) {
          h += `<div class="atc-diff-row atc-diff-row--ctx"><span class="atc-diff-ln">${i + 1}</span><span class="atc-diff-sign"> </span><span class="atc-diff-code" data-raw="${escapeAttr(nLine)}">${escapeHtml(nLine)}</span></div>`;
          rendered++;
        }
        continue;
      }

      if (i - lastShown > 2 && lastShown >= 0) {
        const skipped = i - lastShown - 1;
        if (skipped > 0) {
          h += `<div class="atc-diff-more">@@ ${skipped} unchanged line${skipped > 1 ? 's' : ''} @@</div>`;
        }
      }

      if (lastShown < 0 && i > 0) {
        const ctxStart = Math.max(0, i - 2);
        for (let c = ctxStart; c < i; c++) {
          if (c < oldL.length) {
            h += `<div class="atc-diff-row atc-diff-row--ctx"><span class="atc-diff-ln">${c + 1}</span><span class="atc-diff-sign"> </span><span class="atc-diff-code" data-raw="${escapeAttr(oldL[c])}">${escapeHtml(oldL[c])}</span></div>`;
            rendered++;
          }
        }
      }

      if (oLine !== undefined && oLine !== nLine) {
        h += `<div class="atc-diff-row atc-diff-row--del"><span class="atc-diff-ln">${i + 1}</span><span class="atc-diff-sign">-</span><span class="atc-diff-code" data-raw="${escapeAttr(oLine)}">${escapeHtml(oLine)}</span></div>`;
        rendered++;
      }
      if (nLine !== undefined && oLine !== nLine) {
        h += `<div class="atc-diff-row atc-diff-row--add"><span class="atc-diff-ln">${i + 1}</span><span class="atc-diff-sign">+</span><span class="atc-diff-code" data-raw="${escapeAttr(nLine)}">${escapeHtml(nLine)}</span></div>`;
        rendered++;
      }
      lastShown = i;
    }
  }

  if (rendered >= cap) {
    // 用真正走到的源行数算，不用 cap（见上面 stoppedAt 那段说明）。
    const remaining = Math.max(oldL.length, newL.length) - stoppedAt;
    if (remaining > 0) h += `<div class="atc-diff-more">… ${remaining} more lines not shown …</div>`;
  }
  h += "</div>";
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
