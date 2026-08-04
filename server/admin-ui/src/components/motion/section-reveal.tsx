import { useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

type SectionRevealProps = {
  children: ReactNode;
  className?: string;
  /** 入场延迟（毫秒），用于同容器内多元素错峰 */
  delay?: number;
  /** 渲染为的标签，默认 div */
  as?: "div" | "section" | "li" | "article";
};

/**
 * 可复用分区入场：IntersectionObserver 触发一次，通过 data-reveal 切换状态，
 * 具体动效参数（位移/时长/曲线/reduced-motion 降级）集中在 index.css 的 [data-reveal] 里。
 *
 * 相对展示站那份，这里只动了观察器的两个参数，理由是这两处在后台会真的把内容弄没：
 *
 *  1. threshold 0.15 → 0。threshold 量的是"目标自身可见的比例"，展示站每个 section 都
 *     大约一屏高，0.15 永远够得着；后台一屏是一根很长的竖列，一段比视口高六倍以上时
 *     可见比例永远到不了 0.15 —— 观察器一次都不触发，那一段就永久停在 opacity: 0。
 *     一个动效参数不该有"把整块内容藏起来"这种失败模式。
 *  2. 没有 IntersectionObserver 就直接判定可见。同理：降级的结果只能是"不动"，
 *     不能是"看不见"。
 */
export function SectionReveal({
  children,
  className,
  delay = 0,
  as = "div",
}: SectionRevealProps) {
  const ref = useRef<HTMLElement | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setVisible(true);
            observer.unobserve(entry.target);
          }
        }
      },
      { threshold: 0, rootMargin: "0px 0px -10% 0px" },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const Tag = as;
  return (
    <Tag
      ref={ref as never}
      data-reveal={visible ? "visible" : "hidden"}
      style={delay ? ({ "--reveal-delay": `${delay}ms` } as React.CSSProperties) : undefined}
      className={cn(className)}
    >
      {children}
    </Tag>
  );
}
