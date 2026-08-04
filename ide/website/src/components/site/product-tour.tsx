import { useEffect, useRef, useState } from "react";
import { Check } from "lucide-react";
import { cn } from "@/lib/utils";
import { SectionReveal } from "@/components/motion/section-reveal";
import featOrchestrateLight from "@/assets/feat-orchestrate-light.png";
import featOrchestrateDark from "@/assets/feat-orchestrate-dark.png";
import featExecuteLight from "@/assets/feat-execute-light.png";
import featExecuteDark from "@/assets/feat-execute-dark.png";
import featReviewLight from "@/assets/feat-review-light.png";
import featReviewDark from "@/assets/feat-review-dark.png";

/*
 * Sticky scroll story（composition-repertoire #layout：左滚动步骤 + 右 sticky 画面，
 * 桌面签名构图，移动端平铺堆叠）。三个场景画面来自 capture-ai.mjs：真实应用的
 * 聊天管线渲染的会话（回复内容为脚本化演示流），跟随站点主题切换明暗档。
 */
const steps = [
  {
    eyebrow: "01 · Orchestrate",
    title: "One request, a team of agents",
    description:
      "Fan a task out to specialist agents that investigate in parallel — architecture, security, coverage — then join their findings into one answer.",
    bullets: [
      "Each agent runs read-only, in its own sandbox, with its own tool budget",
      "Nested cards show exactly which files every agent opened",
      "The parent waits on the whole fan-out before it reports",
    ],
    light: featOrchestrateLight,
    dark: featOrchestrateDark,
    alt: "Three specialist agents running in parallel inside one chat, each with its own nested tool cards",
  },
  {
    eyebrow: "02 · Execute",
    title: "A plan you can watch it work through",
    description:
      "Real work gets a real checklist. The agent writes the plan, then ticks each step off as it reads, edits and verifies — no black box.",
    bullets: [
      "Every tool call is a card: what ran, on which file, what came back",
      "Steps flip to done only when the work behind them actually landed",
      "Cancel any step mid-run — the plan is interactive, not decoration",
    ],
    light: featExecuteLight,
    dark: featExecuteDark,
    alt: "A task plan with completed steps beside the tool cards that produced them",
  },
  {
    eyebrow: "03 · Review",
    title: "Every edit lands as a reviewable diff",
    description:
      "Nothing is applied behind your back. Each change opens as a unified diff with a line count — and a one-click undo if you disagree.",
    bullets: [
      "Syntax-highlighted diffs, inline in the conversation",
      "Undo reverts the agent's write without touching your other edits",
      "The same diff opens in the editor and the Git panel",
    ],
    light: featReviewLight,
    dark: featReviewDark,
    alt: "An expanded unified diff of an agent edit, showing added and removed lines with an undo action",
  },
];

function PaneImage({
  step,
  className,
}: {
  step: (typeof steps)[number];
  className?: string;
}) {
  return (
    <>
      <img src={step.light} alt={step.alt} loading="lazy" className={cn("block dark:hidden", className)} />
      <img src={step.dark} alt={step.alt} loading="lazy" className={cn("hidden dark:block", className)} />
    </>
  );
}

export function ProductTour() {
  const [active, setActive] = useState(0);
  const stepRefs = useRef<Array<HTMLDivElement | null>>([]);

  // 中线触发：步骤穿过视口中部时切换右侧画面（IO，无滚动监听）
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          const index = stepRefs.current.indexOf(entry.target as HTMLDivElement);
          if (index >= 0) setActive(index);
        }
      },
      { rootMargin: "-45% 0px -45% 0px" },
    );
    for (const el of stepRefs.current) if (el) observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <div className="grid gap-12 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:gap-16">
      {/* 左：滚动步骤 */}
      <div>
        {steps.map((step, i) => (
          <div
            key={step.title}
            ref={(el) => {
              stepRefs.current[i] = el;
            }}
            className={cn(
              "border-l-2 py-10 pl-6 transition-colors duration-300 first:pt-2 last:pb-2 lg:min-h-[24rem]",
              active === i ? "border-brand" : "border-border",
            )}
          >
            <SectionReveal>
              <p className="type-eyebrow mb-3">{step.eyebrow}</p>
              <h3 className="text-2xl font-semibold">{step.title}</h3>
              <p className="mt-3 text-muted-foreground">{step.description}</p>
              <ul className="mt-5 space-y-2.5">
                {step.bullets.map((bullet) => (
                  <li key={bullet} className="flex gap-2.5 text-sm text-muted-foreground">
                    <Check className="mt-0.5 size-4 shrink-0 text-brand" />
                    {bullet}
                  </li>
                ))}
              </ul>
              {/* 移动端：画面跟在文案后（sticky story 的堆叠降级） */}
              <div className="mt-6 overflow-hidden rounded-xl border border-border shadow-lg lg:hidden">
                <PaneImage step={step} className="w-full" />
              </div>
            </SectionReveal>
          </div>
        ))}
      </div>

      {/* 右：sticky 画面，交叉淡切（opacity-only，reduced-motion 天然安全） */}
      <div className="hidden lg:block">
        <div className="sticky top-24">
          <div
            className="brand-ring relative overflow-hidden rounded-2xl border border-border bg-card shadow-xl"
            style={{
              boxShadow:
                "var(--ide-shadow), 0 24px 80px -24px color-mix(in srgb, var(--brand) 20%, transparent)",
            }}
          >
            {/* 第一张撑起容器高度，其余绝对定位叠放 */}
            {steps.map((step, i) => (
              <div
                key={step.title}
                aria-hidden={active !== i}
                className={cn(
                  "transition-opacity duration-500",
                  i === 0 ? "relative" : "absolute inset-0",
                  active === i ? "opacity-100" : "opacity-0",
                )}
              >
                <div className="flex h-full items-start justify-center bg-ide-bg p-6">
                  <PaneImage
                    step={step}
                    className="max-h-[34rem] w-auto max-w-full rounded-lg border border-ide-line shadow-md"
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
