import { TooltipProvider } from "@/components/ui/tooltip";

/**
 * 应用装配层。
 * 区块组件按访客旅程顺序挂载在 <main> 内，相邻区块的构图不要重复。
 */
export default function App() {
  return (
    <TooltipProvider delayDuration={150}>
      <div className="min-h-screen bg-background text-foreground antialiased">
        <a
          href="#main"
          className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[60] focus:rounded-lg focus:bg-primary focus:px-4 focus:py-2 focus:text-sm focus:text-primary-foreground"
        >
          跳到主要内容
        </a>

        <main id="main">
          <section id="top" className="border-b border-border">
            <div className="mx-auto w-full max-w-7xl px-6 py-16 md:py-24">
              <p className="type-eyebrow">Michael Design</p>
              <h1 className="mt-4 text-4xl font-semibold md:text-6xl">
                在这里替换成你的标题
              </h1>
              <p className="type-measure mt-6 text-lg text-muted-foreground">
                这是脚手架的起始区块。删掉它，然后把你自己的区块组件挂进 main。
              </p>
            </div>
          </section>
        </main>
      </div>
    </TooltipProvider>
  );
}
