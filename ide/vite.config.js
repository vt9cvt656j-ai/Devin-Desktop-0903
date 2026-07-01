import { defineConfig } from "vite";
import obfuscator from "vite-plugin-javascript-obfuscator";

// L2 client hardening (anti-reverse): obfuscate OUR application code (`src/**`) in the
// production bundle so the agent's orchestration logic isn't readable in the shipped
// desktop app. Combined with L0 (prompts + tool schemas never leave the gateway), the
// client carries neither the IP text nor legible logic.
//
// OPT-IN via `OBFUSCATE=1 npm run build` (default OFF → the normal build is byte-for-byte
// unchanged), so it's trivial to A/B if anything misbehaves; once the obfuscated build is
// confirmed working in the app, flip the default on. Conservative settings only:
//   • renameGlobals OFF  — Tauri/`window` globals & ESM import/export bindings survive.
//   • controlFlowFlattening / selfDefending / debugProtection OFF — these are the
//     breaking + slow transforms; dial them up only after the app runs clean obfuscated.
// node_modules (Monaco / xterm / vendor) are excluded by the plugin's default matcher —
// never obfuscate the big third-party chunks (slow + can break them).
const OBFUSCATE = !!process.env.OBFUSCATE;

const obfuscatorOptions = {
  compact: true,
  identifierNamesGenerator: "hexadecimal",
  renameGlobals: false,
  stringArray: true,
  stringArrayEncoding: ["base64"],
  stringArrayThreshold: 0.75,
  splitStrings: true,
  splitStringsChunkLength: 12,
  numbersToExpressions: true,
  simplify: true,
  transformObjectKeys: false,
  deadCodeInjection: false,
  controlFlowFlattening: false,
  debugProtection: false,
  selfDefending: false,
  disableConsoleOutput: false,
  unicodeEscapeSequence: false,
};

export default defineConfig({
  clearScreen: false,
  plugins: OBFUSCATE
    ? [
        obfuscator({
          apply: "build",
          include: ["src/**/*.js"],
          exclude: [/node_modules/],
          options: obfuscatorOptions,
        }),
      ]
    : [],
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
      // overlay.html = the red-glow "controlling the computer" overlay window. Second HTML
      // entry so it's emitted to dist/ for the production build (dev serves it directly).
      input: { main: "index.html", overlay: "overlay.html" },
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
