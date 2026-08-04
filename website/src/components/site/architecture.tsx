import { useEffect, useRef, useState } from "react";
import { SectionReveal } from "@/components/motion/section-reveal";
import { IdeEmbed } from "@/components/site/ide-embed";
import serviceLight from "@/assets/proj-service-light.png";
import serviceDark from "@/assets/proj-service-dark.png";

/*
 * How it works：不再用三张说明卡讲部署 —— 直接把产品本体放上来，
 * 让真实的 Monaco 自己把一段代码敲进真实的工程里（?play=1）。
 */
export function Architecture() {
  const ref = useRef<HTMLDivElement | null>(null);
  const [live, setLive] = useState(false);

  // Monaco is heavy: don't boot it until this section is actually on screen.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setLive(true);
          observer.disconnect();
        }
      },
      { rootMargin: "200px 0px" },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <section id="architecture" className="py-24">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <SectionReveal className="mb-10 text-center">
          <p className="type-eyebrow mb-3">How it works</p>
          <h2 className="type-measure mx-auto text-balance text-3xl font-semibold sm:text-4xl">
            Watch it write code
          </h2>
          <p className="type-measure mx-auto mt-4 text-muted-foreground">
            Below is the application running one request end to end: the assistant reads the
            router, writes a new endpoint into the file, and runs the test suite. Every card,
            diff and exit code is the product's own — nothing here is a recording.
          </p>
        </SectionReveal>

        <SectionReveal>
          <div ref={ref}>
          <IdeEmbed
            label="agent run"
            demo="service"
            play
            posterLight={serviceLight}
            posterDark={serviceDark}
            active={live}
            onActivate={() => setLive(true)}
            onStop={() => setLive(false)}
          />
          </div>
        </SectionReveal>

        <SectionReveal delay={120}>
          <div className="mx-auto mt-10 grid max-w-4xl gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {[
              ["No keys on laptops", "Provider credentials live in the gateway, never in a dotfile."],
              ["Centrally enabled models", "Administrators choose what appears in the picker."],
              ["One bill, not twelve", "Usage is metered per account, so spend is visible per team."],
              ["TLS end to end", "The Rust core streams to the gateway over HTTPS."],
            ].map(([title, body]) => (
              <div key={title} className="rounded-xl border border-border bg-card p-4">
                <p className="text-sm font-semibold">{title}</p>
                <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{body}</p>
              </div>
            ))}
          </div>
        </SectionReveal>
      </div>
    </section>
  );
}
