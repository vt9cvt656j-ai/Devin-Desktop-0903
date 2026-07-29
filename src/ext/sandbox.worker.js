// Per-extension sandbox worker.
//
// Each enabled extension runs in its own Web Worker. The worker cannot touch
// the DOM, the editor, or the filesystem directly — it only talks to the
// extension host on the main thread through structured messages. Every
// privileged operation (editor/workspace access) is mediated and
// permission-checked by the host, so a malicious extension is confined to the
// capabilities its manifest declares.
//
// This file deliberately uses no static imports so it can be bundled as a
// plain worker; the extension module itself is loaded at runtime from a blob
// URL via dynamic import().

let registered = new Map(); // commandId -> handler
let rpcSeq = 0;
const pending = new Map(); // reqId -> { resolve, reject }

function postRpc(method, args, permission) {
  const reqId = `r${rpcSeq++}`;
  return new Promise((resolve, reject) => {
    pending.set(reqId, { resolve, reject });
    self.postMessage({ t: "rpc", reqId, method, args, permission });
  });
}

// The capability object handed to each extension's activate(ide).
function makeIde() {
  return {
    commands: {
      register(id, handler) {
        if (typeof id !== "string" || typeof handler !== "function") {
          throw new Error("commands.register(id, handler) expects a string id and a function");
        }
        registered.set(id, handler);
        self.postMessage({ t: "register", id });
      },
    },
    window: {
      showInformationMessage: (text) => postRpc("window.showInformationMessage", [String(text)]),
      setStatusBarItem: (id, opts) =>
        postRpc("window.setStatusBarItem", [String(id), opts || {}]),
      removeStatusBarItem: (id) => postRpc("window.removeStatusBarItem", [String(id)]),
    },
    editor: {
      getText: () => postRpc("editor.getText", [], "editor"),
      getSelection: () => postRpc("editor.getSelection", [], "editor"),
      insertText: (text) => postRpc("editor.insertText", [String(text)], "editor"),
      replaceText: (range, text) => postRpc("editor.replaceText", [range, String(text)], "editor"),
      setDecorations: (decorations) => postRpc("editor.setDecorations", [decorations], "editor"),
      clearDecorations: () => postRpc("editor.clearDecorations", [], "editor"),
      getFilePath: () => postRpc("editor.getFilePath", [], "editor"),
      getLanguage: () => postRpc("editor.getLanguage", [], "editor"),
      getLineCount: () => postRpc("editor.getLineCount", [], "editor"),
      getLine: (lineNumber) => postRpc("editor.getLine", [lineNumber], "editor"),
    },
    workspace: {
      readFile: (path) => postRpc("workspace.readFile", [String(path)], "workspace-read"),
      writeFile: (path, content) =>
        postRpc("workspace.writeFile", [String(path), String(content)], "workspace-write"),
      listDir: (path) => postRpc("workspace.listDir", [String(path)], "workspace-read"),
    },
    network: {
      fetch: (url, opts) => postRpc("network.fetch", [String(url), opts || {}], "network"),
    },
    diagnostics: {
      set: (uri, diagnostics) =>
        postRpc("diagnostics.set", [String(uri), diagnostics], "diagnostics"),
      clear: (uri) => postRpc("diagnostics.clear", [String(uri)], "diagnostics"),
    },
    locale: {
      registerLocale: (locale, translations) =>
        postRpc("locale.registerLocale", [String(locale), translations], "locale"),
      setLocale: (locale) =>
        postRpc("locale.setLocale", [String(locale)], "locale"),
    },
    subscriptions: [],
  };
}

let ide = null;
let extModule = null;

self.onmessage = async (event) => {
  const msg = event.data;
  if (!msg || typeof msg !== "object") return;

  switch (msg.t) {
    case "activate": {
      try {
        const blob = new Blob([msg.source], { type: "text/javascript" });
        const url = URL.createObjectURL(blob);
        try {
          extModule = await import(/* @vite-ignore */ url);
        } finally {
          URL.revokeObjectURL(url);
        }
        if (typeof extModule.activate !== "function") {
          throw new Error("extension has no exported activate(ide) function");
        }
        ide = makeIde();
        await extModule.activate(ide);
        self.postMessage({ t: "activated" });
      } catch (err) {
        self.postMessage({ t: "error", phase: "activate", message: String(err && err.message ? err.message : err) });
      }
      break;
    }
    case "invoke": {
      const handler = registered.get(msg.id);
      if (!handler) {
        self.postMessage({ t: "invokeResult", reqId: msg.reqId, ok: false, error: `command not registered: ${msg.id}` });
        return;
      }
      try {
        await handler(...(msg.args || []));
        self.postMessage({ t: "invokeResult", reqId: msg.reqId, ok: true });
      } catch (err) {
        self.postMessage({ t: "invokeResult", reqId: msg.reqId, ok: false, error: String(err && err.message ? err.message : err) });
      }
      break;
    }
    case "rpcResult": {
      const entry = pending.get(msg.reqId);
      if (!entry) return;
      pending.delete(msg.reqId);
      if (msg.ok) entry.resolve(msg.value);
      else entry.reject(new Error(msg.error || "host call failed"));
      break;
    }
    case "deactivate": {
      try {
        if (extModule && typeof extModule.deactivate === "function") {
          await extModule.deactivate();
        }
        for (const sub of ide?.subscriptions || []) {
          try {
            if (typeof sub === "function") sub();
            else if (sub && typeof sub.dispose === "function") sub.dispose();
          } catch {
            /* ignore subscription disposal errors */
          }
        }
      } catch {
        /* ignore deactivate errors */
      } finally {
        self.postMessage({ t: "deactivated" });
      }
      break;
    }
    default:
      break;
  }
};
