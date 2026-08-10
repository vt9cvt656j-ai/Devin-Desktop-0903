import { useCallback, useEffect, useRef, useState } from "react";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "./components/dialog.jsx";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/tabs.jsx";
import { Button } from "./components/button.jsx";
import { cn } from "./lib/cn.js";

/**
 * Memory Center — project memory, global preferences, and the memory graph.
 *
 * The previous layout gave roughly 60% of the dialog to the 3D graph and squeezed the two
 * editors into a strip at the bottom, cut off mid-sentence. The graph is real functionality —
 * clicking a node selects that line in the editor — but it is something you look at
 * occasionally, while the editors are what you opened the panel to use. So they swap billing:
 * the editors are the default tab and get the full height, and the graph is one tab away at a
 * size where it is actually legible.
 *
 * The graph itself stays imperative (main.js owns _mcGlobeInit and its WebGL lifecycle). This
 * component just hands over a container node and tells main.js when the text changed so it can
 * rebuild. React owns layout; the widget owns itself.
 */
export function MemoryCenter({
  rootLabel,
  hasRoot,
  initialProject,
  initialGlobal,
  onGlobeMount,
  onTextChange,
  onSave,
  onClearProject,
  onClearGlobal,
  onClose,
}) {
  const [project, setProject] = useState(initialProject ?? "");
  const [global, setGlobal] = useState(initialGlobal ?? "");
  const [tab, setTab] = useState("memory");
  const globeHost = useRef(null);
  const globeHandle = useRef(null);
  const projectRef = useRef(null);
  const globalRef = useRef(null);

  // A callback ref rather than an effect: it fires exactly when the node attaches, and again
  // with null when it detaches. An effect keyed on the active tab looked equivalent and was not
  // — Radix attaches tab content on its own schedule, so the effect ran while the ref was still
  // null and the graph silently never got built.
  const attachGlobe = useCallback((node) => {
    globeHost.current = node;
    if (node) {
      globeHandle.current = onGlobeMount?.({
        host: node,
        projectEl: projectRef.current,
        globalEl: globalRef.current,
      });
    } else {
      try { globeHandle.current?.destroy?.(); } catch { /* already torn down */ }
      globeHandle.current = null;
    }
  }, [onGlobeMount]);

  // Tear the graph down with the dialog, not only when the tab changes.
  useEffect(() => () => {
    try { globeHandle.current?.destroy?.(); } catch { /* already torn down */ }
  }, []);

  const edit = useCallback((which, value) => {
    if (which === "project") setProject(value); else setGlobal(value);
    onTextChange?.(which, value);
  }, [onTextChange]);

  const editorClass = cn(
    "min-h-0 flex-1 resize-none rounded-lg border border-border bg-card px-3 py-2",
    "font-mono text-[12px] leading-6 text-foreground outline-none",
    "placeholder:text-muted-foreground focus:border-primary",
    "disabled:cursor-not-allowed disabled:opacity-60",
  );

  return (
    <Dialog defaultOpen onOpenChange={(open) => { if (!open) onClose?.(); }}>
      <DialogContent className="flex h-[78vh] max-w-4xl flex-col gap-0 overflow-hidden rounded-2xl p-0 shadow-xl">
        <DialogHeader className="shrink-0 space-y-1 border-b border-border px-5 pt-5 pb-4">
          <div className="flex items-baseline gap-3">
            <DialogTitle className="text-base">Memory</DialogTitle>
            <span
              className="truncate font-mono text-[11px] text-muted-foreground"
              data-i18n-skip
              title={rootLabel}
            >
              {rootLabel}
            </span>
          </div>
          <DialogDescription className="text-[12px]">
            One entry per line. Project memory applies here; preferences travel with you.
          </DialogDescription>
        </DialogHeader>

        <Tabs value={tab} onValueChange={setTab} className="flex min-h-0 flex-1 flex-col">
          <TabsList className="mx-5 mt-4 w-fit shrink-0">
            <TabsTrigger value="memory">Memory</TabsTrigger>
            <TabsTrigger value="graph">Graph</TabsTrigger>
          </TabsList>

          <TabsContent value="memory" className="mt-0 flex min-h-0 flex-1 gap-4 px-5 pt-4 pb-4">
            <section className="flex min-h-0 flex-1 flex-col gap-1.5">
              <h3 className="text-[12px] font-medium text-foreground">
                Project memory
                {!hasRoot ? (
                  <span className="ml-2 font-normal text-muted-foreground">no folder open</span>
                ) : null}
              </h3>
              <textarea
                ref={projectRef}
                className={editorClass}
                spellCheck={false}
                disabled={!hasRoot}
                value={project}
                onChange={(e) => edit("project", e.target.value)}
                placeholder="This project uses pnpm&#10;UI goes through shadcn/ui"
              />
            </section>
            <section className="flex min-h-0 flex-1 flex-col gap-1.5">
              <h3 className="text-[12px] font-medium text-foreground">Global preferences</h3>
              <textarea
                ref={globalRef}
                className={editorClass}
                spellCheck={false}
                value={global}
                onChange={(e) => edit("global", e.target.value)}
                placeholder="Answer truthfully&#10;Verify a fix before calling it done"
              />
            </section>
          </TabsContent>

          {/* forceMount: Radix otherwise unmounts the inactive tab, so the graph's host node does
              not exist until you switch — and the globe is imperative code that needs a real,
              sized node to attach to. Mounting it up front is also what the previous panel did,
              so the graph's own lifecycle is unchanged; only where it sits on screen is. */}
          <TabsContent
            value="graph"
            forceMount
            className="mt-0 min-h-0 flex-1 px-5 pt-4 pb-4 data-[state=inactive]:hidden"
          >
            <div className="relative h-full overflow-hidden rounded-xl border border-border bg-muted/30">
              <div ref={attachGlobe} className="h-full w-full" />
              <p className="pointer-events-none absolute bottom-2 right-3 text-[11px] text-muted-foreground">
                Drag to rotate · hover to inspect · click to jump to that line
              </p>
            </div>
          </TabsContent>
        </Tabs>

        <div className="flex shrink-0 items-center justify-end gap-2 border-t border-border px-5 py-3">
          {/* Destructive actions are quiet ghosts, not two red outlines competing with Save.
              Google puts weight on the affirmative action and leaves the rest as text. */}
          <Button
            variant="ghost"
            size="sm"
            disabled={!hasRoot}
            className="text-muted-foreground hover:text-destructive"
            onClick={() => { setProject(""); onClearProject?.(); }}
          >
            Clear project
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="text-muted-foreground hover:text-destructive"
            onClick={() => { setGlobal(""); onClearGlobal?.(); }}
          >
            Clear preferences
          </Button>
          <Button size="sm" onClick={() => onSave?.(project, global)}>Save</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
