import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  // Served under /account/ by nginx, so asset URLs must be prefixed. Without this the built
  // index.html asks for /assets/... and gets the API's 404 instead of the bundle.
  base: "/account/",
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 5274,
    strictPort: true,
    proxy: { "/api": "http://127.0.0.1:8099" },
  },
});
