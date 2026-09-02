import { defineConfig } from "vite";

// Tauri reads the built assets from `../dist` (see src-tauri/tauri.conf.json).
export default defineConfig({
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
  },
});
