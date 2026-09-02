import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
const STUB = "stub:";
const DEEP = new Set(["monaco-editor","@xterm/xterm","@xterm/addon-fit","@xterm/addon-webgl","@xterm/addon-web-links","three","3d-force-graph","globe.gl","three-spritetext","@neftaly/editcontext-polyfill"]);
export async function resolve(spec, ctx, next) {
  if (/\?(worker|raw|url|inline)$/.test(spec)) return { url: STUB + "worker", shortCircuit: true, format: "module" };
  // The tool executor does not depend on the optional React gallery island. Keep
  // this node-only harness focused on the production executor instead of asking
  // Node's ESM loader to parse JSX before the harness can install its hooks.
  if (spec === "./ui/mount-gallery.jsx" && /\/src\/main\.js$/.test(ctx.parentURL || "")) {
    return { url: STUB + "ui-gallery", shortCircuit: true, format: "module" };
  }
  if (/\.(css|scss|less|svg|png|jpg|jpeg|gif|woff2?|ttf)(\?.*)?$/.test(spec)) return { url: STUB + "asset", shortCircuit: true, format: "module" };
  for (const d of DEEP) if (spec === d || spec.startsWith(d + "/")) return { url: STUB + "deep:" + spec, shortCircuit: true, format: "module" };
  try { return await next(spec, ctx); }
  catch (e) { console.error("[hooks] UNRESOLVED:", spec, "from", ctx.parentURL, "->", e.code || e.message); throw e; }
}
export async function load(url, ctx, next) {
  if (url === STUB + "worker") return { format: "module", shortCircuit: true, source: "export default class StubWorker { constructor(){this.onmessage=null;} postMessage(){} terminate(){} addEventListener(){} removeEventListener(){} }" };
  if (url === STUB + "ui-gallery") return { format: "module", shortCircuit: true, source: "export {};" };
  if (url === STUB + "asset") return { format: "module", shortCircuit: true, source: "export default {}; export const __asset=true;" };
  if (url.startsWith(STUB + "deep:")) {
    const name = url.slice((STUB + "deep:").length);
    return { format: "module", shortCircuit: true, source:
      `import { mk, mkMonaco } from ${JSON.stringify(process.env.__DEEPSTUB_URL)};\n` +
      `const R = ${name === "monaco-editor" ? "mkMonaco()" : `mk(${JSON.stringify(name)})`};\n` +
      `export default R;\n` +
      `export const __ns = R;\n` +
      // named exports the app actually destructures
      `export const Terminal = R.Terminal, FitAddon = R.FitAddon, WebglAddon = R.WebglAddon, WebLinksAddon = R.WebLinksAddon;\n` +
      `export const editor = R.editor, languages = R.languages, Uri = R.Uri, Range = R.Range, Position = R.Position, KeyMod = R.KeyMod, KeyCode = R.KeyCode, MarkerSeverity = R.MarkerSeverity, Selection = R.Selection, Emitter = R.Emitter, CancellationTokenSource = R.CancellationTokenSource;\n` };
  }
  if (/\.json$/.test(url)) return { format: "module", shortCircuit: true, source: "export default " + readFileSync(fileURLToPath(url), "utf8") + ";" };
  const r = await next(url, ctx);
  if (/\/src\/main\.js$/.test(url) && r.source) {
    let s = r.source.toString();
    s = s.replace(/import\.meta\.glob/g, "globalThis.__viteGlob").replace(/import\.meta\.env/g, "globalThis.__viteEnv");
    // TEST HOOK: append module-scope export of the real executor. Loader-only, so the
    // shipped src/main.js is byte-for-byte unchanged in production.
    s += "\n\n/* __E2E_HOOK__ */\nif (globalThis.__E2E__) { globalThis.__testHooks = { _executeToolStepInner, backend, _normalizeFsPath, _resolveRel, _toolFailureKey, _resetToolFailure, setMode: (m)=>{ _currentAiMode = m; }, getMode: ()=>_currentAiMode };\n  try { globalThis.__testHooks.extra = { showToast, _fsDelta: typeof _fsDelta }; } catch (e) { globalThis.__testHookErr = e.message; } }\n";
    return { ...r, source: s };
  }
  return r;
}
