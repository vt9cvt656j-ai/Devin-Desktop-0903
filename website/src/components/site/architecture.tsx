import { useRef, useState } from "react";
import { SectionReveal } from "@/components/motion/section-reveal";
import { IdeEmbed } from "@/components/site/ide-embed";
import { useNearViewport } from "@/lib/use-near-viewport";
import serviceLight from "@/assets/proj-service-light.png";
import serviceDark from "@/assets/proj-service-dark.png";

/*
 * How it works：不再用三张说明卡讲部署 —— 直接把产品本体放上来，
 * 让真实的 Monaco 自己把一段代码敲进真实的工程里（?play=1）。
 */
export function Architecture() {
  const ref = useRef<HTMLDivElement | null>(null);
  // Monaco is heavy, and starting it steals the reader's scroll position — so it waits
  // until this section is genuinely approaching, measured against the settled layout.
  // The observer this replaced fired while the page was still collapsing images into
  // place, which is how the front page ended up opening on this very section.
  const near = useNearViewport(ref, 200);
  // The reset button has to be able to stop it, and approaching the section again must
  // not silently restart what the reader just switched off.
  const [stopped, setStopped] = useState(false);
  const live = near && !stopped;

  return (
    <section id="architecture" className="py-24">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <SectionReveal className="mb-10 text-center">
          <p className="type-eyebrow mb-3">How it works</p>
          <h2 className="type-measure mx-auto text-balance text-3xl font-semibold sm:text-4xl">
            Watch it write code
          </h2>
          <p className="type-measure mx-auto mt-4 text-muted-foreground">
            {/*
              Reworded to what is actually true. This previously read "Every card, diff and
              exit code is the product's own — nothing here is a recording", and the exit
              code is not: the browser build runs against a mock backend, so the terminal
              and the test result are a fixture. There is no Python in a browser. A page
              that publishes its own code review, criticisms included, cannot afford a
              headline claim that does not survive being checked.
            */}
            Below is the real editor and the real agent loop, running in your browser
            against a sample project. The interface, the streaming and the tool calls are
            the application's own code — the project it works on, and its terminal, are a
            sandbox rather than a machine of yours.
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
            onActivate={() => setStopped(false)}
            onStop={() => setStopped(true)}
          />
          </div>
        </SectionReveal>

      </div>
    </section>
  );
}
