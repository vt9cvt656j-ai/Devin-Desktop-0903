import { TooltipProvider } from "@/components/ui/tooltip";
import { Navbar } from "@/components/site/navbar";
import { Hero } from "@/components/site/hero";
import { LanguageMarquee } from "@/components/site/language-marquee";
import { Features } from "@/components/site/features";
import { Architecture } from "@/components/site/architecture";
import { ToolGallerySection } from "@/components/site/tool-gallery";
import { Testimonials } from "@/components/site/testimonials";
import { Cta } from "@/components/site/cta";
import { Footer } from "@/components/site/footer";

/**
 * 应用装配层。
 * 区块组件按访客旅程顺序挂载在 <main> 内,相邻区块的构图不要重复。
 */
export default function App() {
  return (
    <TooltipProvider delayDuration={150}>
      <div className="min-h-screen bg-background text-foreground antialiased">
        <div aria-hidden className="scroll-progress" />
        <a
          href="#main"
          className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[60] focus:rounded-lg focus:bg-primary focus:px-4 focus:py-2 focus:text-sm focus:text-primary-foreground"
        >
          Skip to main content
        </a>

        <Navbar />
        <main id="main">
          <Hero />
          <LanguageMarquee />
          <Features />
          <Architecture />
          <ToolGallerySection />
          <Testimonials />
          <Cta />
        </main>
        <Footer />
      </div>
    </TooltipProvider>
  );
}
