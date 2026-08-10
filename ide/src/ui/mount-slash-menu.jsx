import { mountIsland, unmountIsland } from "./island.jsx";
import { SlashMenu } from "./slash-menu.jsx";

/**
 * vanilla → React seam for the `/` command palette.
 *
 * The JSX lives here rather than in main.js on purpose: vite.config.js runs the React plugin over
 * .jsx/.tsx only, so the 59k-line shell is never transformed and its source-text test assertions
 * keep matching. main.js calls these two plain functions and stays JSX-free — the same shape as
 * mount-gallery.jsx.
 */
export function renderSlashMenu(host, { items, activeIndex, onPick, onHover }) {
  if (!host) return;
  mountIsland(host, (
    <SlashMenu items={items} activeIndex={activeIndex} onPick={onPick} onHover={onHover} />
  ));
}

export function destroySlashMenu(host) {
  if (!host) return;
  try { unmountIsland(host); } catch { /* host already gone — nothing to release */ }
}
