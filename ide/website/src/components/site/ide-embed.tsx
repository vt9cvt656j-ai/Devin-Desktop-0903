import { useEffect, useRef, useState } from "react";
import { Play, RotateCcw } from "lucide-react";
import { cn } from "@/lib/utils";
import { seedIdePreferences } from "@/lib/seed-ide";


/*
 * 真机嵌入：iframe 里跑的就是 ide 的生产构建（vite build --base=/app/ → public/app）。
 * 不是仿制件 —— 样式、尺寸、行为都是产品本体，因为它就是产品本体。
 *
 * 同源，所以挂载前直接写 localStorage 把语言和主题交给它（IDE 在启动时读这两个键）。
 * 一次只允许一个实例存活：Monaco 很重，四个同时挂会拖垮页面。
 */
export function IdeEmbed({
  active,
  onActivate,
  onStop,
  label,
  demo,
  play,
  posterLight,
  posterDark,
}: {
  active: boolean;
  onActivate: () => void;
  onStop: () => void;
  label: string;
  /** 预览工程 id：交给 ide 的 mock 后端选样本项目 */
  demo: string;
  /** 开启后编辑器会自己把这段代码敲进去（真 Monaco，真高亮） */
  play?: boolean;
  posterLight: string;
  posterDark: string;
}) {
  const [booted, setBooted] = useState(false);
  const frame = useRef<HTMLIFrameElement | null>(null);

  useEffect(() => {
    if (!active) setBooted(false);
  }, [active]);

  // Sections that auto-activate never call launch(), so seed here too.
  useEffect(() => {
    if (active) seedIdePreferences();
  }, [active]);

  function launch() {
    try {
      // Drop any project the previous section opened so each starts on its own.
      for (const key of Object.keys(localStorage)) {
        if (key.startsWith("michael-ide.") && key !== "michael-ide.theme") {
          localStorage.removeItem(key);
        }
      }
    } catch {
      /* private mode */
    }
    seedIdePreferences();
    onActivate();
  }

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-ide-bg shadow-2xl">
      <div className="relative aspect-[16/10] w-full">
        {active ? (
          <>
            <iframe
              ref={frame}
              src={`/app/index.html?demo=${demo}${play ? "&play=agent" : ""}`}
              title={`Mr.day One — ${label}`}
              onLoad={() => setBooted(true)}
              className="absolute inset-0 size-full border-0"
            />
            {!booted && (
              <div className="absolute inset-0 flex items-center justify-center bg-ide-bg text-sm text-ide-text-dim">
                starting the editor…
              </div>
            )}
          </>
        ) : (
          <>
            <img
              src={posterLight}
              alt={`Mr.day One with the ${label} project open`}
              className="absolute inset-0 size-full object-cover object-top dark:hidden"
            />
            <img
              src={posterDark}
              alt={`Mr.day One with the ${label} project open`}
              className="absolute inset-0 hidden size-full object-cover object-top dark:block"
            />
            <div className="absolute inset-0 flex items-center justify-center bg-zinc-950/45 backdrop-blur-[2px]">
              <button
                type="button"
                onClick={launch}
                className="flex items-center gap-2 rounded-xl bg-white/95 px-5 py-3 text-sm font-semibold text-zinc-900 shadow-xl transition-transform hover:scale-[1.02] active:scale-[0.98]"
              >
                <Play className="size-4" />
                {play ? "Watch the agent work" : "Run the real editor here"}
              </button>
            </div>
          </>
        )}
      </div>

      <div className="flex items-center gap-2 border-t border-border bg-card px-3 py-2">
        <span className="font-mono text-[11px] text-muted-foreground">
          {active ? "live — this is the application, not a video" : "click to launch"}
        </span>
        {active && (
          <button
            type="button"
            onClick={onStop}
            className={cn(
              "ml-auto flex items-center gap-1 rounded-md border border-border px-2 py-1",
              "font-mono text-[11px] text-muted-foreground transition-colors hover:text-foreground",
            )}
          >
            <RotateCcw className="size-3" /> reset
          </button>
        )}
      </div>
    </div>
  );
}
