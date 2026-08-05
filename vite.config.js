import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import JavaScriptObfuscator from "javascript-obfuscator";
import { stripToolIp } from "./build/strip-tool-ip.mjs";

// Build-time IP strip: blank tool DESCRIPTION text in `_buildAgentToolSchemas` so the
// shipped bundle carries no tool-library prose. Runtime-neutral (L0 sends only tool
// names; the gateway injects full schemas). Build-only so dev keeps readable sources.
// Runs BEFORE the obfuscator (enforced by plugin order below) and FAILS the build if
// the target function moved/renamed, so a refactor can't silently re-leak the IP.
function stripToolIpPlugin() {
  return {
    name: "strip-tool-ip",
    apply: "build",
    enforce: "pre",
    transform(code, id) {
      if (!/\/src\/main\.js$/.test(id.replace(/\\/g, "/"))) return null;
      const { code: out, changed, found } = stripToolIp(code);
      if (!found) {
        throw new Error(
          "[strip-tool-ip] _buildAgentToolSchemas not found in src/main.js — the tool-IP " +
          "strip did not run. A rename/move would ship tool descriptions in the bundle. " +
          "Update build/strip-tool-ip.mjs before building.",
        );
      }
      if (changed < 40) {
        throw new Error(
          `[strip-tool-ip] only ${changed} description lines blanked (expected many more) — ` +
          "the strip likely missed tools. Refusing to ship a partially-stripped bundle.",
        );
      }
      return { code: out, map: null };
    },
  };
}

// L2 client hardening (anti-reverse): obfuscate OUR application code (`src/**`) in the
// production bundle so the agent's orchestration logic isn't readable in the shipped
// desktop app. Combined with L0 (prompts + tool schemas never leave the gateway), the
// client carries neither the IP text nor legible logic.
//
// Default ON (opt OUT with `OBFUSCATE=0 npm run build`).
// 2026-07-13 postmortem: the first default-on build shipped a DEAD desktop app. Cause:
// the obfuscator moved the `import("@tauri-apps/...")` specifier strings into its
// string array, so vite could no longer statically bundle those dynamic imports; at
// runtime the bare specifiers are unresolvable in the webview and the top-level
// `await tauriBackend()` threw, killing the whole module. Browser smoke tests missed
// it because only `inTauri` code paths execute those imports. Hence:
//   • reservedStrings keeps every "@tauri-apps/..." literal intact for the bundler;
//   • dynamicImportGuard() below FAILS THE BUILD if any emitted app chunk still
//     contains a runtime-computed import(...) specifier.
// Conservative settings only:
//   • renameGlobals OFF  — Tauri/`window` globals & ESM import/export bindings survive.
//   • controlFlowFlattening / selfDefending / debugProtection OFF — these are the
//     breaking + slow transforms; dial them up only after the app runs clean obfuscated.
// node_modules (Monaco / xterm / vendor) are excluded by the plugin's default matcher —
// never obfuscate the big third-party chunks (slow + can break them).
const OBFUSCATE = process.env.OBFUSCATE !== "0";

const obfuscatorOptions = {
  compact: true,
  identifierNamesGenerator: "hexadecimal",
  renameGlobals: false,           // keep chunk import/export bindings intact
  stringArray: true,
  stringArrayEncoding: ["base64"],
  stringArrayThreshold: 0.75,
  // splitStrings OFF: it splits EVERY string literal into concatenated chunks —
  // including the emitted dynamic-import chunk paths (`import("assets/x.js")`) — which
  // turns them into computed specifiers the bundler can't resolve → dead app in Tauri.
  // reservedStrings doesn't exempt strings from splitStrings, so it must stay off when
  // obfuscating a bundled chunk that contains dynamic imports.
  splitStrings: false,
  numbersToExpressions: true,
  simplify: true,
  transformObjectKeys: false,
  deadCodeInjection: false,
  controlFlowFlattening: false,
  debugProtection: false,
  selfDefending: false,
  disableConsoleOutput: false,
  unicodeEscapeSequence: false,
  // Keep literal every string the bundler still needs to resolve at runtime:
  //   ^@tauri-apps/  — bare module specifiers for dynamic import()
  //   \.js$          — emitted chunk paths (vite __vite__mapDeps + import("assets/x.js"))
  //   ^assets/ , ^\./ — same chunk-path strings in other forms
  // Without these the obfuscator would move them into its string array and the
  // dynamic import specifier becomes computed → dead app in Tauri.
  reservedStrings: ["^@tauri-apps/", "\\.js$", "^assets/", "^\\./"],
};

const COMPUTED_IMPORT_RE = /import\(\s*[^"'`)\s]/;

// Obfuscate OUR app entry chunks AFTER the whole build is written to disk. This has
// to happen in `writeBundle`, not `transform`/`renderChunk`: under vite 8 / rolldown
// both earlier hooks get their output re-minified by rolldown's built-in pass, which
// stripped the obfuscation right back out (verified: shipped bundles had zero
// string-array/`_0x` markers even with the plugin "running"). writeBundle is the last
// word — nothing runs after it, so what we write here is exactly what ships.
// Scope: only main-*/overlay-* (our code). Never touch vendor/monaco/xterm chunks —
// obfuscating those is slow and breaks them. Fails the build if obfuscation would
// break a dynamic import (dead app in Tauri) or if it silently produced no markers.
function obfuscateAppChunks() {
  return {
    name: "obfuscate-app-chunks",
    apply: "build",
    enforce: "post",
    async writeBundle(options, bundle) {
      if (!OBFUSCATE) return;
      const fs = await import("node:fs");
      const path = await import("node:path");
      const outDir = options.dir || "dist";
      for (const fileName of Object.keys(bundle)) {
        const chunk = bundle[fileName];
        if (chunk.type !== "chunk") continue;
        if (!/^(main|overlay)-/.test(fileName.replace(/^assets\//, ""))) continue;
        const full = path.resolve(outDir, fileName);
        let src = fs.readFileSync(full, "utf8");
        // rolldown emits dynamic imports as TEMPLATE literals: import(`./x-hash.js`).
        // javascript-obfuscator's reservedStrings only exempts plain string literals,
        // not template literals, so it would transform these paths → computed import →
        // dead app. Convert the STATIC (no ${…}) ones to plain strings first; then the
        // reservedStrings patterns (^\./ , \.js$) protect them through obfuscation.
        src = src.replace(/import\(`([^`$]+)`\)/g, 'import("$1")');
        const out = JavaScriptObfuscator.obfuscate(src, obfuscatorOptions).getObfuscatedCode();
        if (!/_0x[0-9a-f]/.test(out)) {
          throw new Error(`[obfuscate] ${fileName}: obfuscation produced no markers — refusing to ship an unobfuscated bundle.`);
        }
        if (COMPUTED_IMPORT_RE.test(out)) {
          throw new Error(`[obfuscate] ${fileName}: obfuscation broke a dynamic import() specifier (would be a dead app in Tauri). Check reservedStrings.`);
        }
        fs.writeFileSync(full, out);
      }
    },
  };
}

// Build-time invariant: every dynamic import in OUR chunks must have a literal
// specifier ("..." or `...`), i.e. something the bundler already resolved. A
// computed specifier means a transform (obfuscator or otherwise) destroyed one —
// that ships as a dead app in Tauri, so fail the BUILD, not the user.
function dynamicImportGuard() {
  const COMPUTED_IMPORT_RE = /import\(\s*[^"'`)\s]/;
  return {
    name: "dynamic-import-guard",
    generateBundle(_options, bundle) {
      for (const [fileName, chunk] of Object.entries(bundle)) {
        if (chunk.type !== "chunk") continue;
        if (!/^assets\/(main|overlay)-/.test(fileName)) continue; // only our app entries
        if (COMPUTED_IMPORT_RE.test(chunk.code)) {
          throw new Error(
            `[dynamic-import-guard] ${fileName} contains a runtime-computed import(...) specifier — ` +
            "the bundler could not resolve it statically (an obfuscator/transform likely rewrote the " +
            "import string). This would ship a dead app in Tauri. Fix the transform (e.g. add the " +
            "specifier to obfuscatorOptions.reservedStrings) instead of shipping.",
          );
        }
      }
    },
  };
}

export default defineConfig({
  clearScreen: false,
  // The app's own version, so the desktop heartbeat can tell the gateway which build is
  // running. Read from package.json at build time rather than hardcoded, because a
  // second place to bump is a place that gets forgotten.
  define: {
    __APP_VERSION__: JSON.stringify(
      JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")).version,
    ),
  },
  plugins: [
    // enforce:"pre" → tool-IP strip runs before bundling.
    stripToolIpPlugin(),
    // React islands: shadcn components mounted into the existing vanilla shell.
    // Only touches .jsx/.tsx — src/main.js has no JSX, so the 59k-line shell is not
    // transformed and its 907 source-text test assertions keep matching.
    react(),
    // Tailwind v4. Preflight is deliberately NOT imported (see src/ui/tailwind.css):
    // its reset would restyle every element in 14,111 lines of existing CSS.
    tailwindcss(),
    // writeBundle → obfuscate the final emitted app chunks.
    // React/Radix land in `vendor` (see manualChunks), which this never touches.
    obfuscateAppChunks(),
    // generateBundle → assert dynamic imports survived (runs after obfuscation).
    dynamicImportGuard(),
  ],
  server: {
    port: 5174,
    strictPort: true,
    proxy: {
      "/api": {
        target: process.env.VITE_API_TARGET || "http://127.0.0.1:8080",
        changeOrigin: true,
      },
    },
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
