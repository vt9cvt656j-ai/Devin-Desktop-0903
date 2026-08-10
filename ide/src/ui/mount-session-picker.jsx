import { mountIsland, unmountIsland } from "./island.jsx";
import { SessionPicker } from "./session-picker.jsx";

/**
 * vanilla → React seam for `/sessions`. Same shape as mount-gallery.jsx and
 * mount-slash-menu.jsx: the JSX stays out of main.js, which vite deliberately does not run
 * through the React plugin.
 */
const HOST_ID = "session-picker-host";

export function openSessionPickerIsland({ entries, resumableCount, onPick }) {
  const existing = document.getElementById(HOST_ID);
  if (existing) { close(existing); return; }

  const host = document.createElement("div");
  host.id = HOST_ID;
  document.body.appendChild(host);

  mountIsland(host, (
    <SessionPicker
      entries={entries}
      resumableCount={resumableCount}
      onPick={(entry) => { close(host); onPick?.(entry); }}
      onClose={() => close(host)}
    />
  ));
}

function close(host) {
  unmountIsland(host);
  // unmountIsland defers the unmount to a microtask; the host has to outlive it or React
  // unmounts against a node already detached from the document.
  queueMicrotask(() => queueMicrotask(() => host.remove()));
}
