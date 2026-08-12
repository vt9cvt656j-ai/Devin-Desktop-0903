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
import { ChangelogPage } from "@/components/site/changelog-page";
import { RankingsPage } from "@/components/site/rankings-page";

/**
 * 应用装配层。
 * 区块组件按访客旅程顺序挂载在 <main> 内,相邻区块的构图不要重复。
 */
/*
 * Standalone pages, chosen by path.
 *
 * nginx already falls back to index.html for unknown paths, so /changelog and /rankings
 * reach this bundle and are rendered here — no router dependency and no second build
 * entry. A table rather than a chain of conditions now that there is more than one: adding
 * a page is a line, and every one of them is visible in a single place. If any of these
 * ever grows sub-paths or needs history navigation, that is the point to bring in a real
 * router instead of teaching this to parse.
 */
const PAGES: { match: RegExp; title: string; render: () => React.ReactNode }[] = [
  { match: /^\/changelog\/?$/, title: "Update log", render: () => <ChangelogPage /> },
  { match: /^\/rankings\/?$/, title: "Rankings", render: () => <RankingsPage /> },
];

export default function App() {
  const page = PAGES.find((p) => p.match.test(location.pathname));
  if (page) {
    // These are pages people link to and keep open in a tab. Leaving the front page's
    // title on them labels every one of them "the AI-native code editor…", which tells
    // someone with six tabs open nothing about which one this is.
    document.title = `${page.title} — Mr. Day One`;
    return (
      <TooltipProvider delayDuration={150}>
        <div className="min-h-screen bg-background text-foreground antialiased">
          {page.render()}
          <Footer />
        </div>
      </TooltipProvider>
    );
  }

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
