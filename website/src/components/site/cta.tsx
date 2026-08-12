
import { DownloadButtons } from "@/components/site/download-buttons";
import { SectionReveal } from "@/components/motion/section-reveal";

export function Cta() {
  return (
    <section id="download" className="border-t border-border bg-primary py-24 text-primary-foreground">
      <div className="mx-auto max-w-3xl px-4 text-center sm:px-6">
        <SectionReveal>
          <h2 className="text-balance text-3xl font-semibold sm:text-4xl">
            Stop reviewing suggestions. Start reviewing verified diffs.
          </h2>
          <p className="mx-auto mt-4 max-w-xl text-primary-foreground/70">
            Mr. Day One is free while in preview.
          </p>
          <DownloadButtons variant="onDark" className="mt-8" />
        </SectionReveal>
      </div>
    </section>
  );
}
