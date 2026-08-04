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
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setVisible(true);
            observer.unobserve(entry.target);
          }
        }
      },
      { threshold: 0.15, rootMargin: "0px 0px -10% 0px" },
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
