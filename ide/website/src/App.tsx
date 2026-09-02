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
import { DocsPage } from "@/components/site/docs-page";

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
const PAGES: {
  match: RegExp;
  title: string;
  render: () => React.ReactNode;
  /** 带站点导航渲染。文档要靠它 —— 见下面 chrome 分支的说明。 */
  chrome?: boolean;
  /** 内容更宽的页面，导航条跟着加宽，两条左边缘才对得齐。 */
  wide?: boolean;
}[] = [
  { match: /^\/changelog\/?$/, title: "Update log", render: () => <ChangelogPage /> },
  { match: /^\/rankings\/?$/, title: "Rankings", render: () => <RankingsPage /> },
  // /docs 和 /docs/<slug> 都走这一页 —— 它自己按地址挑要显示哪一篇，并用 pushState 在
  // 页面之间切换（所以每一页都能被收藏和分享）。
  //
  // chrome: 文档要带站点导航。没有它，从搜索直接落到某一篇的人看不到 Product / Download /
  // 主题切换，只有一个返回箭头 —— 那是「个人博客」和「正经文档站」最直观的分界线。
  // wide: 文档是三栏、比其它页宽，导航条不加宽的话 logo 会比侧栏左边缘缩进近 100px。
  { match: /^\/docs(\/[^/]*)?\/?$/, title: "文档", render: () => <DocsPage />, chrome: true, wide: true },
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
          {page.chrome ? <Navbar wide={page.wide} /> : null}
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
