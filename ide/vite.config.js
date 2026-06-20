import { defineConfig } from "vite";

// Tauri expects a fixed dev port and reads the production build from `dist/`.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    // Monaco's core (editor.api) is an inherently large, unsplittable vendor
    // chunk (~2.3 MB). Size the threshold to clear it while still flagging any
    // unexpected growth in the application bundle.
    chunkSizeWarningLimit: 2400,
    rollupOptions: {
      output: {
        manualChunks(id) {
          // Let Vite keep splitting Monaco languages into on-demand chunks —
          // returning undefined here preserves that lazy behaviour.
          if (id.includes("monaco-editor")) return;
          if (id.includes("@xterm")) return "xterm";
          if (id.includes("node_modules")) return "vendor";
        },
      },
    },
  },
});
