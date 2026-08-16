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
/**
 * Forget whichever project was open last.
 *
 * Each demo opens a different tree, and the IDE picks it up from localStorage when it
 * starts — so without this, switching from one panel to the next boots the new editor on
 * the previous one's state and it comes up empty. The theme is kept deliberately: that is
 * the reader's choice, not the previous project's leftovers.
 */
function clearOpenProject() {
  try {
    for (const key of Object.keys(localStorage)) {
      if (key.startsWith("michael-ide.") && key !== "michael-ide.theme") {
        localStorage.removeItem(key);
      }
    }
  } catch {
    /* private mode — nothing was stored, so nothing needs clearing */
  }
}

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

  /*
   * The editor must not mount until its slate has been wiped.
   *
   * Each demo opens a different project, and the IDE reads which one from localStorage at
   * startup. `launch()` cleared the previous project's keys before setting `active`, so a
   * click always produced a clean start. Auto-starting skipped that — the keys the last
   * tab left behind were still there when the next one booted, and the panel came up
   * empty. Clearing it from an effect would not help either: effects run after the iframe
   * is in the DOM, by which point it has already read the stale values.
   *
   * So `prepared` is the gate. `active` says the reader should see the editor; `prepared`
   * says the storage underneath it is ready. The iframe waits for both, which costs one
   * extra render and removes the race entirely.
   */
  const [prepared, setPrepared] = useState(false);

  useEffect(() => {
    if (!active) {
      setPrepared(false);
      setBooted(false);
      return;
    }
    clearOpenProject();
    seedIdePreferences();
    setPrepared(true);
  }, [active, demo]);

  /*
   * Starting the editor must not move the reader.
   *
   * The iframe below runs the real IDE, and the editor takes focus as it starts. Focusing
   * anything inside an iframe makes the browser scroll the *parent* document to reveal it
   * — so a section that auto-activates while someone is still at the top drags them down
   * to it. That is why the front page kept opening part-way down with a clean URL and
   * nothing in the page's own scroll code to blame; three fixes went looking in the wrong
   * place before the iframe turned out to be the one doing it.
   *
   * So for a few seconds after an embed starts, any scroll the reader did not ask for is
   * undone. Their own wheel, key, touch or pointer input cancels the guard immediately,
   * so this can never fight someone who is actually scrolling.
   */
  useEffect(() => {
    if (!active) return;

    const startedAt = window.scrollY;
    let readerMoved = false;
    const yield_ = () => {
      readerMoved = true;
    };
    const events = ["wheel", "touchstart", "keydown", "pointerdown"] as const;
    for (const name of events) window.addEventListener(name, yield_, { passive: true });

    const guard = window.setInterval(() => {
      if (readerMoved) return;
      if (Math.abs(window.scrollY - startedAt) > 4) {
        window.scrollTo({ top: startedAt, behavior: "instant" as ScrollBehavior });
      }
    }, 100);
    // Long enough to cover the editor booting, short enough that it is gone well before
    // anyone could be surprised by it.
    const release = window.setTimeout(() => window.clearInterval(guard), 4000);

    return () => {
      window.clearInterval(guard);
      window.clearTimeout(release);
      for (const name of events) window.removeEventListener(name, yield_);
    };
  }, [active]);

  /*
   * 遮罩什么时候撤掉。
   *
   * 以前只挂 `onLoad`，两个问题：
   *
   * 1. `load` 要等 iframe 里**所有**子资源下载完。里面是完整的 IDE —— Monaco、xterm、
   *    几十个分块、好几兆。编辑器早就能敲了，`load` 还没来；而只要其中任何一个分块慢
   *    或者卡住，遮罩就永远停在"starting the editor…"，看上去就是加载不出来。
   * 2. React 在 commit 阶段才把 `onLoad` 挂上去。走缓存时 `load` 可能在那之前就已经
   *    触发过一次 —— 事件错过了就不会再来，遮罩同样永远不消失。
   *
   * 所以改成盯真正要等的那件事：编辑器有没有出现。同源，读得到 iframe 的 document，
   * `.monaco-editor` 一出现就撤遮罩 —— 这比 `load` 早得多，也正是遮罩那句话的字面意思。
   * 同时补挂一次 load 监听并立刻查一遍 readyState，把上面第 2 条的竞态补掉。
   *
   * 最后还有一条兜底：到点无论如何都撤。宁可让人看见一个半成品编辑器，也不要一块永远
   * 转不完的灰底 —— 前者还能往下读，后者只会让人以为网站坏了。
   */
  useEffect(() => {
    if (!active || !prepared) return;
    const el = frame.current;
    if (!el) return;

    let done = false;
    const lift = () => {
      if (done) return;
      done = true;
      setBooted(true);
    };

    const editorIsUp = () => {
      try {
        return !!el.contentDocument?.querySelector(".monaco-editor");
      } catch {
        // 跨源就读不到（现在不会，但别让它抛出来把轮询打断）。
        return false;
      }
    };

    const poll = window.setInterval(() => {
      if (editorIsUp()) {
        window.clearInterval(poll);
        lift();
      }
    }, 120);

    // 事件可能已经错过了，所以监听之外还要立刻自查一次。
    el.addEventListener("load", lift);
    try {
      if (el.contentDocument?.readyState === "complete") lift();
    } catch {
      /* 读不到就等事件 */
    }

    // 兜底。给得比正常启动宽裕得多，只在真的出问题时才会用上。
    const giveUp = window.setTimeout(lift, 8000);

    return () => {
      window.clearInterval(poll);
      window.clearTimeout(giveUp);
      el.removeEventListener("load", lift);
    };
  }, [active, prepared, demo]);

  function launch() {
    // The preparation itself now happens in the effect above, which covers this path and
    // the automatic one alike. This only asks to be shown.
    onActivate();
  }

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-ide-bg shadow-2xl">
      <div className="relative aspect-[16/10] w-full">
        {/* Both, not just `active` — see the note on `prepared`. */}
        {active && prepared ? (
          <>
            <iframe
              ref={frame}
              src={`/app/index.html?demo=${demo}${play ? "&play=agent" : ""}`}
              title={`Mr. Day One — ${label}`}
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
              alt={`Mr. Day One with the ${label} project open`}
              className="absolute inset-0 size-full object-cover object-top dark:hidden"
            />
            <img
              src={posterDark}
              alt={`Mr. Day One with the ${label} project open`}
              className="absolute inset-0 hidden size-full object-cover object-top dark:block"
            />
            {/*
              Revealed on hover, and no longer a wall.

              The editor now starts by itself once the section is reached, so this is the
              way back in after pressing reset — not the thing standing between the reader
              and the panel. As a permanent dark scrim it obscured the very screenshot it
              was advertising and implied a click was required. `focus-within` keeps it
              reachable by keyboard, where there is no hover to depend on.
            */}
            <div
              className={cn(
                "absolute inset-0 flex items-center justify-center",
                "bg-zinc-950/45 backdrop-blur-[2px]",
                "opacity-0 transition-opacity duration-200 hover:opacity-100 focus-within:opacity-100",
              )}
            >
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
          {/* It starts on its own now, so "click to launch" only described the old
              behaviour. The inactive state is reached by pressing reset. */}
          {active ? "live — this is the application, not a video" : "stopped — hover to run it again"}
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
