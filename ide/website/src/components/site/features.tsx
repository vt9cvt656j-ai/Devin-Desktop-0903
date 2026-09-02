import { SectionReveal } from "@/components/motion/section-reveal";
import { ProductTour } from "@/components/site/product-tour";
import { Showcase } from "@/components/site/showcase";

export function Features() {
  return (
    <section id="features" className="border-y border-border bg-muted py-24">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <SectionReveal className="mb-14 text-center">
          <p className="type-eyebrow mb-3">Product</p>
          <h2 className="type-measure mx-auto text-balance text-3xl font-semibold sm:text-4xl">
            An agent that works your repository, not a chat box beside it
          </h2>
          <p className="type-measure mx-auto mt-4 text-muted-foreground">
            Every panel on this page is captured from the running app — nothing here is a mockup.
          </p>
        </SectionReveal>

        <ProductTour />
        <Showcase />
      </div>
    </section>
  );
}
