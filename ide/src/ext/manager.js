// Extension storage layer: talks to the Rust backend under Tauri, and keeps an
// in-memory store when running in a plain browser preview.
//
// The bundled sample extensions are imported at build time so the browser
// preview has something to install and the source matches what the backend
// ships byte-for-byte.

import wordCountManifest from "../../src-tauri/extensions/word-count/extension.json";
import wordCountSource from "../../src-tauri/extensions/word-count/index.js?raw";
import insertDateManifest from "../../src-tauri/extensions/insert-date/extension.json";
import insertDateSource from "../../src-tauri/extensions/insert-date/index.js?raw";
import aiAssistantManifest from "../../src-tauri/extensions/ai-assistant/extension.json";
import aiAssistantSource from "../../src-tauri/extensions/ai-assistant/index.js?raw";
import codeFormatterManifest from "../../src-tauri/extensions/code-formatter/extension.json";
import codeFormatterSource from "../../src-tauri/extensions/code-formatter/index.js?raw";
import bracketColorizerManifest from "../../src-tauri/extensions/bracket-colorizer/extension.json";
import bracketColorizerSource from "../../src-tauri/extensions/bracket-colorizer/index.js?raw";
import todoHighlightManifest from "../../src-tauri/extensions/todo-highlight/extension.json";
import todoHighlightSource from "../../src-tauri/extensions/todo-highlight/index.js?raw";
import colorPickerManifest from "../../src-tauri/extensions/color-picker/extension.json";
import colorPickerSource from "../../src-tauri/extensions/color-picker/index.js?raw";
import chineseLangManifest from "../../src-tauri/extensions/chinese-language-pack/extension.json";
import chineseLangSource from "../../src-tauri/extensions/chinese-language-pack/index.js?raw";
// NOTE: only extensions whose source is committed may be bundled here — these
// `?raw` imports are resolved at build time, so referencing an uncommitted
// extension breaks the Vite build on a clean checkout.

const BUILTIN = [
  { manifest: wordCountManifest, source: wordCountSource },
  { manifest: insertDateManifest, source: insertDateSource },
  { manifest: aiAssistantManifest, source: aiAssistantSource },
  { manifest: codeFormatterManifest, source: codeFormatterSource },
  { manifest: bracketColorizerManifest, source: bracketColorizerSource },
  { manifest: todoHighlightManifest, source: todoHighlightSource },
  { manifest: colorPickerManifest, source: colorPickerSource },
  { manifest: chineseLangManifest, source: chineseLangSource },
];

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function createExtensionManager() {
  return inTauri ? await tauriManager() : mockManager();
}

async function tauriManager() {
  const core = await import("@tauri-apps/api/core");
  const dialog = await import("@tauri-apps/plugin-dialog");
  return {
    listInstalled: () => core.invoke("ext_list_installed"),
    availableBuiltin: () => core.invoke("ext_available_builtin"),
    installBuiltin: (id) => core.invoke("ext_install_builtin", { id }),
    setEnabled: (id, enabled) => core.invoke("ext_set_enabled", { id, enabled }),
    uninstall: (id) => core.invoke("ext_uninstall", { id }),
    readAsset: (id, rel) => core.invoke("ext_read_asset", { id, rel }),
    installFromFile: async () => {
      const path = await dialog.open({
        multiple: false,
        directory: false,
        filters: [{ name: "Extension package", extensions: ["zip"] }],
      });
      if (!path) return null;
      return core.invoke("ext_install_from_path", { archivePath: path });
    },
  };
}

function mockManager() {
  // id -> { manifest, enabled, files: { rel -> source } }
  const installed = new Map();
  const builtinById = new Map(BUILTIN.map((b) => [b.manifest.id, b]));

  const toInstalled = (rec) => ({ manifest: rec.manifest, enabled: rec.enabled });

  return {
    listInstalled: async () =>
      [...installed.values()]
        .map(toInstalled)
        .sort((a, b) => a.manifest.name.toLowerCase().localeCompare(b.manifest.name.toLowerCase())),
    availableBuiltin: async () => BUILTIN.map((b) => b.manifest),
    installBuiltin: async (id) => {
      const b = builtinById.get(id);
      if (!b) throw new Error(`unknown built-in extension: ${id}`);
      const files = { [b.manifest.main || "index.js"]: b.source };
      const rec = { manifest: b.manifest, enabled: true, files };
      installed.set(id, rec);
      return toInstalled(rec);
    },
    setEnabled: async (id, enabled) => {
      const rec = installed.get(id);
      if (!rec) throw new Error("extension is not installed");
      rec.enabled = enabled;
    },
    uninstall: async (id) => {
      installed.delete(id);
    },
    readAsset: async (id, rel) => {
      const rec = installed.get(id);
      const src = rec?.files?.[rel];
      if (src == null) throw new Error(`asset not found: ${rel}`);
      return src;
    },
    installFromFile: async () => {
      throw new Error("Installing from a file is only available in the desktop app.");
    },
  };
}
