import { mountIsland, unmountIsland } from "./island.jsx";
import { ArchiveBrowser } from "./archive-browser.jsx";

/**
 * vanilla → React seam for the archive panel. Same shape as the other mounts: the JSX stays out of
 * main.js, which vite deliberately does not run through the React plugin.
 *
 * The inspector rebuilds its whole body with innerHTML on every render, so the host node is
 * replaced each time and the island has to be remounted against the new one rather than assumed
 * to still be attached.
 */
export function mountArchiveBrowser(host, props) {
  if (!host) return;
  mountIsland(host, <ArchiveBrowser {...props} />);
}

export function unmountArchiveBrowser(host) {
  if (host) unmountIsland(host);
}
