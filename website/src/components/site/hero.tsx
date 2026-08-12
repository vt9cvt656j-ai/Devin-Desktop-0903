import { ArrowRight } from "lucide-react";
import { DownloadButtons } from "@/components/site/download-buttons";
import { SectionReveal } from "@/components/motion/section-reveal";
import { IdeWindow } from "@/components/site/ide-window";

export function Hero() {
  return (
    <section className="relative overflow-hidden">
      {/* 氛围层：品牌双色 aurora（css-effects #mesh-gradient + 慢漂移） */}
      <div aria-hidden className="pointer-events-none absolute inset-0">
        <div className="aurora aurora--one left-[8%] top-24 h-[26rem] w-[26rem]" />
        <div className="aurora aurora--two right-[6%] top-56 h-[30rem] w-[30rem]" />
      </div>

      <div className="relative mx-auto max-w-6xl px-4 pb-24 pt-20 text-center sm:px-6 sm:pt-28">
        <SectionReveal delay={80}>
          <h1 className="type-measure mx-auto text-balance text-4xl font-semibold sm:text-6xl">
            The AI-native editor that{" "}
            <span className="gradient-text">verifies its own work</span>
          </h1>
        </SectionReveal>

        <SectionReveal delay={160}>
          <p className="type-measure mx-auto mt-6 text-pretty text-lg text-muted-foreground">
            Mr. Day One wires your repo, terminal, and diagnostics into one agent loop. The AI
            edits the code, runs the tests, reads the errors, and re-checks the result — instead
            of handing you a suggestion and walking away.
          </p>
        </SectionReveal>

        <SectionReveal delay={240}>
          <div className="mt-8 flex flex-col items-center gap-4">
            <DownloadButtons />
            <a
              href="#architecture"
              className="group inline-flex items-center gap-1.5 text-sm font-medium text-foreground transition-colors hover:text-brand"
            >
              See the agent loop
              <ArrowRight className="size-4 transition-transform duration-200 group-hover:translate-x-0.5" />
            </a>
          </div>
        </SectionReveal>

        <SectionReveal delay={320} className="mt-16">
          <IdeWindow className="mx-auto max-w-5xl" />
        </SectionReveal>
      </div>
    </section>
  );
}
