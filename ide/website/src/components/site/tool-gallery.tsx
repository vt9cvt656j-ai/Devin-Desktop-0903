import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { visualFor } from "@/components/site/tool-visuals";
import { cn } from "@/lib/utils";
import { SectionReveal } from "@/components/motion/section-reveal";
import { seedIdePreferences } from "@/lib/seed-ide";
import { useNearViewport } from "@/lib/use-near-viewport";

/*
 * Tool coverflow：目录来自 public/tools.json —— scripts/extract-tools.mjs 从
 * ide/src/main.js 的 _buildAgentToolSchemas 直接抽出（挂在 website 的 prebuild 上），
 * 网站不维护第二份清单：加了新工具重新构建，这里自动就有，而且排在最前面。
 *
 * 排序 = 声明顺序取反，所以最新加的工具在正中，越旧的越往后退。
 */
type Tool = { name: string; group: string; desktopOnly?: boolean };
type Catalog = { count: number; groups: string[]; tools: Tool[] };

/**
 * Where the live catalogue is read from. The site is served from mrday.one and the
 * gateway answers on code.mrday.one, so this is cross-origin by design — the gateway
 * sends permissive CORS and the endpoint returns names only, nothing authenticated.
 */
const GATEWAY = "https://code.mrday.one";

function ToolTile({ tool, focused }: { tool: Tool; focused: boolean }) {
  const v = visualFor(tool.name);
  const Icon = v.icon;
  return (
    <div
      className={cn(
        "flex w-[13rem] flex-col items-center gap-2 rounded-xl border bg-card px-4 py-4 text-center transition-shadow",
        focused ? "border-brand/40 shadow-xl" : "border-border shadow-sm",
      )}
    >
      <span
        className="flex size-10 items-center justify-center rounded-lg"
        style={{ background: `color-mix(in srgb, ${v.tint} 16%, transparent)` }}
      >
        <Icon className="size-5" style={{ color: v.tint }} />
      </span>
      <span className="w-full truncate font-mono text-[12px] font-medium text-foreground">
        {tool.name}
      </span>
      <span className="text-[10px] uppercase tracking-widest text-muted-foreground">
        {tool.group}
      </span>
      {tool.desktopOnly && (
        <span className="rounded-full bg-secondary px-2 py-0.5 text-[9px] uppercase tracking-widest text-muted-foreground">
          desktop only
        </span>
      )}
    </div>
  );
}

export function ToolGallery() {
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [i, setI] = useState(0);
  const [ready, setReady] = useState(false);
  const wheelLock = useRef(0);
  const stage = useRef<HTMLDivElement | null>(null);
  const frame = useRef<HTMLIFrameElement | null>(null);

  // Seed language/theme before the editor iframe below ever loads.
  const [seeded, setSeeded] = useState(false);
  useEffect(() => {
    seedIdePreferences();
    setSeeded(true);
  }, []);

  /*
   * The editor below does not boot until you are nearly at it.
   *
   * It used to mount with the page, and that is why the site opened part-way down: this
   * iframe loads the whole IDE, something inside it takes focus as it starts, and a
   * browser scrolls the *parent* document to bring a newly focused iframe into view. So
   * every visit ended up parked on this section, with no hash in the address bar and
   * nothing in the page's own scroll code to explain it. Three earlier attempts went
   * looking in the wrong place because the URL looked innocent.
   *
   * Waiting for the section to approach fixes that outright — there is nothing to steal
   * focus while you are at the top — and it stops a landing page from booting an entire
   * editor for a section most readers never scroll to.
   */
  const embedBox = useRef<HTMLDivElement | null>(null);
  const embedNear = useNearViewport(embedBox);

  /*
   * The bundled catalogue first, then the gateway's live one on top.
   *
   * tools.json is generated from the IDE registry at build time, which is accurate the
   * day it is built and drifts the moment the catalogue changes without a site rebuild —
   * exactly what happened: this page advertised 147 tools for a week after the product
   * dropped to 130, including 17 the gateway could no longer inject. The build-time copy
   * is kept because it carries the grouping and the desktop-only flags, and because a
   * gateway that is unreachable should degrade to a slightly stale list rather than to an
   * empty page. The live call decides only *membership* — which of those tools still
   * exist — so the page can never again name something the product does not have.
   */
  useEffect(() => {
    let cancelled = false;

    (async () => {
      let bundled: Catalog | null = null;
      try {
        bundled = (await (await fetch("/tools.json")).json()) as Catalog;
      } catch {
        // Not fatal on its own; the live list below can still carry the section.
      }
      if (cancelled) return;
      if (bundled) setCatalog(bundled);

      let live: string[] | null = null;
      try {
        const r = await fetch(`${GATEWAY}/api/tools/catalog`, { cache: "no-store" });
        if (r.ok) live = ((await r.json()) as { tools: string[] }).tools ?? null;
      } catch {
        // Offline, or an older gateway without the endpoint. Keep the bundled list.
      }
      if (cancelled || !live || live.length === 0) return;

      const known = new Map((bundled?.tools ?? []).map((t) => [t.name, t]));
      // Declaration order is what puts the newest tools in the middle of the coverflow,
      // and the gateway preserves it — so the live array's order is used as-is, with the
      // bundled entry supplying group and flags where we have one.
      const tools: Tool[] = live.map(
        (name) => known.get(name) ?? { name, group: "Knowledge" },
      );
      setCatalog({
        count: tools.length,
        groups: [...new Set(tools.map((t) => t.group))].sort(),
        tools,
      });
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  // The embedded editor tells us when its bridge is listening.
  useEffect(() => {
    const onMsg = (e: MessageEvent) => {
      if (e.data?.type === "mrday:ready") setReady(true);
    };
    window.addEventListener("message", onMsg);
    return () => window.removeEventListener("message", onMsg);
  }, []);

  const tools = useMemo(() => (catalog ? [...catalog.tools].reverse() : []), [catalog]);
  const last = tools.length - 1;
  const current = tools[i];

  const step = useCallback(
    (delta: number) => setI((n) => Math.min(last, Math.max(0, n + delta))),
    [last],
  );

  // Ask the real editor to run whichever tool is in front.
  const run = useCallback(() => {
    if (!ready || !current) return;
    frame.current?.contentWindow?.postMessage(
      { type: "mrday:run-tool", tool: current.name },
      window.location.origin,
    );
  }, [ready, current]);

  useEffect(() => {
    if (!ready || !current) return;
    const t = window.setTimeout(run, 260); // debounce while spinning the wheel
    return () => window.clearTimeout(t);
  }, [ready, current, run]);

  useEffect(() => {
    const el = stage.current;
    if (!el || !tools.length) return;
    const onWheel = (e: WheelEvent) => {
      const d = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
      const forward = d > 0;
      if ((forward && i >= last) || (!forward && i <= 0)) return;
      e.preventDefault();
      const now = Date.now();
      if (now < wheelLock.current) return;
      wheelLock.current = now + 110;
      step(forward ? 1 : -1);
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [i, last, step, tools.length]);

  if (!catalog) {
    return <p className="text-center text-sm text-muted-foreground">Loading the tool catalog…</p>;
  }

  return (
    <div>
      {/* picker */}
      <div
        ref={stage}
        tabIndex={0}
        role="group"
        aria-label="Tool picker — scroll or use the arrow keys"
        onKeyDown={(e) => {
          if (e.key === "ArrowRight") { e.preventDefault(); step(1); }
          if (e.key === "ArrowLeft") { e.preventDefault(); step(-1); }
        }}
        className="relative h-36 overflow-hidden rounded-2xl outline-none ring-offset-4 ring-offset-muted focus-visible:ring-2 focus-visible:ring-ring"
        style={{ perspective: "1200px" }}
      >
        {tools.map((tool, n) => {
          const offset = n - i;
          const far = Math.abs(offset) > 4;
          return (
            <div
              key={tool.name}
              aria-hidden={offset !== 0}
              className="absolute left-1/2 top-1/2 transition-all duration-400 ease-out"
              style={{
                transform: `translate(-50%, -50%) translateX(${offset * 150}px) translateZ(${-Math.abs(offset) * 120}px) rotateY(${offset === 0 ? 0 : offset > 0 ? -24 : 24}deg)`,
                opacity: far ? 0 : 1 - Math.abs(offset) * 0.24,
                zIndex: 100 - Math.abs(offset),
                pointerEvents: far ? "none" : "auto",
              }}
              onClick={() => offset !== 0 && step(offset)}
            >
              <ToolTile tool={tool} focused={offset === 0} />
            </div>
          );
        })}
      </div>

      <div className="mt-4 flex flex-wrap items-center justify-center gap-3">
        <button
          type="button"
          onClick={() => step(-1)}
          disabled={i === 0}
          aria-label="Previous tool"
          className="flex size-8 items-center justify-center rounded-full border border-border bg-card text-muted-foreground transition-colors hover:text-foreground disabled:opacity-30"
        >
          <ChevronLeft className="size-4" />
        </button>
        <p className="min-w-[18rem] text-center text-xs text-muted-foreground">
          <span className="font-mono text-sm font-semibold text-foreground">{current?.name}</span>
          <span className="ml-2">{i + 1} of {catalog.count}</span>
        </p>
        <button
          type="button"
          onClick={() => step(1)}
          disabled={i === last}
          aria-label="Next tool"
          className="flex size-8 items-center justify-center rounded-full border border-border bg-card text-muted-foreground transition-colors hover:text-foreground disabled:opacity-30"
        >
          <ChevronRight className="size-4" />
        </button>
      </div>

      {/* the real editor, running whatever is selected */}
      {/*
        The fixed height is on the wrapper, not only on the iframe, so the space is
        reserved before the editor mounts. Without it the page would grow by 34rem the
        moment you scrolled near, shifting everything under it.
      */}
      <div
        ref={embedBox}
        className="mt-6 h-[34rem] overflow-hidden rounded-xl border border-border shadow-2xl"
      >
        {seeded && embedNear && (
          <iframe
            ref={frame}
            src="/app/index.html?demo=service&play=tools"
            title="Mr. Day One running the selected tool"
            className="block h-full w-full border-0"
          />
        )}
      </div>
      <p className="mt-3 text-center text-xs text-muted-foreground">
        Scroll the row to pick a tool — the editor below runs it for real and shows the card it
        produces. Tools marked <span className="font-medium text-foreground">desktop only</span> are
        withheld from the browser build, so those report exactly that.
      </p>
    </div>
  );
}

export function ToolGallerySection() {
  return (
    <section id="extensions" className="border-y border-border bg-muted py-24">
      <div className="mx-auto max-w-6xl px-4 sm:px-6">
        <SectionReveal className="mb-12 text-center">
          <p className="type-eyebrow mb-3">The toolbelt</p>
          <h2 className="type-measure mx-auto text-balance text-3xl font-semibold sm:text-4xl">
            Everything the agent can reach for
          </h2>
          <p className="type-measure mx-auto mt-4 text-muted-foreground">
            Read straight from the application's own schema, newest first — add a tool and this
            gallery has it on the next build.
          </p>
        </SectionReveal>

        <SectionReveal>
          <ToolGallery />
        </SectionReveal>
      </div>
    </section>
  );
}
