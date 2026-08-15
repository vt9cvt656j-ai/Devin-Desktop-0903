import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "@/App";
import { configureMse, mseEnvConfig, mseReady } from "@/lib/mse";
import "@/index.css";

// Configure before anything renders: the first data fetch fires from App's mount effects,
// and a request that leaves before this call would be sealed against the default config
// (same-origin, unpinned) instead of the one this build was compiled with.
configureMse(mseEnvConfig());

// Warm-up only — deliberately not awaited. Fetching the gateway public key can fail
// (offline, gateway restarting, blocked network) and blocking render on it would turn a
// recoverable hiccup into a blank console. mseFetch bootstraps on its own when it finds no
// session, so the cost of a failed warm-up is one slower first request, nothing more.
void mseReady().catch(() => {
  /* the first mseFetch retries this */
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
