import type { ReactNode } from "react";
import { SectionReveal } from "@/components/motion/section-reveal";
import { cn } from "@/lib/utils";

/**
 * 每一屏的第一块。六个页面原本各自手写同一段 h1 + p，字号、间距、和右侧动作的对齐
 * 全靠复制粘贴保持一致 —— 复制得越多，改一次的成本越高。
 *
 * 它自己就是入场的第 0 段（delay 0），后面的段落用 delay={Math.min(i, 4) * 70} 排在它后面，
 * 沿用展示站 SectionReveal 的错峰写法。标题永远最先出现：操作员要先知道自己在哪一屏。
 */
export function PageHeader({
  title,
  description,
  actions,
  className,
}: {
  title: string;
  description?: ReactNode;
  /** 右上角的动作（刷新、新建…）。放在这里，页面就不用为了一个按钮再造一行 flex。 */
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <SectionReveal
      as="section"
      className={cn("flex flex-wrap items-start justify-between gap-x-6 gap-y-3", className)}
    >
      <div className="min-w-0">
        <h1 className="font-display text-2xl font-semibold tracking-tight">{title}</h1>
        {description && <p className="type-measure mt-1 text-muted-foreground">{description}</p>}
      </div>
      {actions && <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div>}
    </SectionReveal>
  );
}
