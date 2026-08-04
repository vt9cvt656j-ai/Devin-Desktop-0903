import { useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

/**
 * One number that drives a decision. No sparklines, no decoration.
 *
 * 两处针对"装了真数据"的处理：
 *
 *  1. 数字只在真的变了的时候动一下。总览和收款每 30 秒重拉一次，如果每次回包都让四张牌
 *     跳一下，操作员两天之内就会学会无视这排数字——而这排数字恰恰是他唯一需要盯的东西。
 *     所以这里比较的是渲染出来的字符串：$12,345.00 → $12,345.00 什么都不发生，
 *     变成 $12,365.00 才闪一次（200ms，见 index.css 的 [data-bump]）。首帧不算变化。
 *  2. 八位数的钱塞不进 text-3xl 的卡片。超过 11 个字符自动降一档字号，再兜一层 truncate +
 *     title，宁可被截断也不要把卡片撑破或让 "$12,345,678.90" 折成两行。
 *
 * value 允许是 ReactNode（定价试算传的是带单位的节点），这时不做变化判定 ——
 * 判定不出"到底变没变"的时候，不动比乱动对。
 */
export function Stat({
  label,
  value,
  hint,
  className,
}: {
  label: string;
  value: ReactNode;
  hint?: ReactNode;
  className?: string;
}) {
  const text = typeof value === "string" || typeof value === "number" ? String(value) : null;
  const previous = useRef(text);
  // 用自增的 key 重挂一次值节点，动画才会在"连续第二次变化"时也重新播；
  // 只切一个布尔位的话，第二次 setState 拿到相同的值，不会重渲染，动画也就不播了。
  const [pulse, setPulse] = useState(0);

  useEffect(() => {
    if (text === null || previous.current === text) return;
    previous.current = text;
    setPulse((n) => n + 1);
  }, [text]);

  const long = (text?.length ?? 0) > 11;

  return (
    <div className={cn("rounded-xl border border-border bg-card p-5", className)}>
      <div className="type-eyebrow truncate" title={label}>
        {label}
      </div>
      <div
        key={pulse}
        data-bump={pulse ? "on" : undefined}
        title={text ?? undefined}
        className={cn(
          "mt-2 truncate font-display font-semibold tracking-tight tabular-nums",
          long ? "text-2xl" : "text-3xl",
        )}
      >
        {value}
      </div>
      {hint && (
        <div className="mt-1 truncate text-sm text-muted-foreground" title={typeof hint === "string" ? hint : undefined}>
          {hint}
        </div>
      )}
    </div>
  );
}
