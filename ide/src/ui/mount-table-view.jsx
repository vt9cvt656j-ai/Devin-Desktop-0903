import { mountIsland, unmountIsland } from "./island.jsx";
import { TableView } from "./table-view.jsx";

/**
 * vanilla → React seam for the CSV/TSV window. Same shape as mount-archive-browser.jsx: the JSX
 * stays out of main.js, which vite deliberately does not run through the React plugin.
 */
export function mountTableView(host, props) {
  if (!host) return;
  mountIsland(host, <TableView {...props} />);
}

export function unmountTableView(host) {
  if (host) unmountIsland(host);
}
