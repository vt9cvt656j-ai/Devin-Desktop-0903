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

    // A reveal animation must NEVER be able to hide content. The previous version could, in two
    // ways that both showed up the moment the console got more than one page:
    //
    //  1. rootMargin "0px 0px -10% 0px" shrinks the observation box from the bottom. A section
    //     that mounts inside that bottom 10% on a page too short to scroll never intersects, so
    //     it sits at opacity 0 forever. On page switch this is the common case, not the edge one.
    //  2. The observer only ever fires on a CHANGE in intersection. Content that is already fully
    //     in view at mount does normally get a synthetic first callback — but it is delivered
    //     asynchronously, and if the element unmounts and remounts fast (exactly what tab
    //     switching does) the callback can land on a disconnected observer.
    //
    // So: no negative rootMargin, an immediate geometry check on mount, and a hard timeout that
    // forces visibility regardless. Degrading to "no animation" is fine. Degrading to "blank
    // screen" is not, and that is what the owner reported.
    const show = () => setVisible(true);

    const rect = el.getBoundingClientRect();
    const inView =
      rect.top < (window.innerHeight || document.documentElement.clientHeight) && rect.bottom > 0;
    if (inView || typeof IntersectionObserver === "undefined") {
      show();
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            show();
            observer.unobserve(entry.target);
          }
        }
      },
      { threshold: 0 },
    );
    observer.observe(el);

    // Last line of defence: whatever happens above, the content is on screen within 400ms.
    const failsafe = window.setTimeout(show, 400);
    return () => {
      window.clearTimeout(failsafe);
      observer.disconnect();
    };
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
