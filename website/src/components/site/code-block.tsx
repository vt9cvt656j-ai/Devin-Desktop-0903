import { useId, useState } from "react";
import { Check, Copy } from "lucide-react";

/**
 * 文档里的代码块。
 *
 * # 为什么代码要走 React 而不是拼进 HTML 串
 *
 * 复制按钮需要状态和事件，DOM 得由 React 管。另一条路是渲染完再用 ref 遍历 DOM 往
 * `dangerouslySetInnerHTML` 的子树里塞按钮 —— 那是靠「React 恰好不 diff 那块」活着，而且
 * 将来要加语法高亮就必须写 `pre.innerHTML = 上色后的串`，等于在 markdown.ts 之外**新开
 * 第二个把正文拼成 HTML 的地方**。
 *
 * 走 React 之后安全性反而是**变强**的：`code` 作为文本子节点交给 React，它自己转义，
 * 代码内容从此完全不参与任何 HTML 拼接。少一次拼接就少一个拼错的机会。
 *
 * # 复制按钮常驻，不是 hover 才出现
 *
 * hover 才出现的按钮在平板和手机上永远不出现，而文档有相当一部分是在平板上读的；键盘用户
 * Tab 到一个看不见的按钮也很困惑。常驻但用低对比度，鼠标移上去才提亮。
 */
export function CodeBlock({ lang, title, code }: { lang: string; title: string; code: string }) {
  const [copied, setCopied] = useState(false);
  // useId 而不是从代码内容算 —— btoa 遇到非 Latin-1 会直接抛，而代码块里有中文注释是常态。
  const id = useId();

  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
    } catch {
      // 剪贴板可能被权限或非安全上下文拒绝。退回到选中整段，让人自己按 ⌘C ——
      // 比一个点了没反应的按钮好。
      const sel = window.getSelection();
      const el = document.getElementById(id);
      if (sel && el) {
        const r = document.createRange();
        r.selectNodeContents(el);
        sel.removeAllRanges();
        sel.addRange(r);
      }
      return;
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 1600);
  }

  const label = title || lang.toUpperCase();

  return (
    <figure className="doc-code group">
      <figcaption className="doc-code__bar">
        <span className={title ? "doc-code__file" : "doc-code__lang"}>{label || "CODE"}</span>
        <button type="button" onClick={() => void copy()} className="doc-code__copy" aria-label="复制代码">
          {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
          {copied ? "已复制" : "复制"}
        </button>
      </figcaption>
      {/*
        tabIndex 让键盘用户能滚动横向溢出的代码 —— 没有它，只用键盘的人读不到超出宽度的行。
        code 是文本子节点：React 转义它，这里不做任何 HTML 拼接。
      */}
      <pre tabIndex={0} role="region" aria-label={label ? `代码：${label}` : "代码"}>
        <code id={id}>{code}</code>
      </pre>
    </figure>
  );
}
