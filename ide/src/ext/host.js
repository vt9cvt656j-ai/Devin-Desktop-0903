// Extension host: owns the sandbox workers and bridges their permission-gated
// requests to the IDE. Extension code never runs on the main thread.

import SandboxWorker from "./sandbox.worker.js?worker";

// RPC method -> permission required to call it (undefined = always allowed).
const PERMISSION_FOR = {
  "window.showInformationMessage": undefined,
  "window.setStatusBarItem": undefined,
  "window.removeStatusBarItem": undefined,
  "editor.getText": "editor",
  "editor.getSelection": "editor",
  "editor.insertText": "editor",
  "editor.replaceText": "editor",
  "editor.setDecorations": "editor",
  "editor.clearDecorations": "editor",
  "editor.getFilePath": "editor",
  "editor.getLanguage": "editor",
  "editor.getLineCount": "editor",
  "editor.getLine": "editor",
  "workspace.readFile": "workspace-read",
  "workspace.writeFile": "workspace-write",
  "workspace.listDir": "workspace-read",
  "network.fetch": "network",
  "diagnostics.set": "diagnostics",
  "diagnostics.clear": "diagnostics",
  "locale.registerLocale": "locale",
  "locale.setLocale": "locale",
};

// Collapse "." and ".." segments without touching the filesystem so a granted
// extension still can't escape the workspace via "../../../etc/passwd".
function normalizePath(p) {
  const isAbs = p.startsWith("/");
  const stack = [];
  for (const seg of p.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") {
      if (stack.length && stack[stack.length - 1] !== "..") stack.pop();
      else if (!isAbs) stack.push("..");
    } else {
      stack.push(seg);
    }
  }
  return (isAbs ? "/" : "") + stack.join("/");
}

export class ExtensionHost {
  /**
   * @param {object} ctx IDE bridge.
   * @param {() => string} ctx.getEditorText
   * @param {() => string} ctx.getSelectionText
   * @param {(text: string) => void} ctx.insertText
   * @param {(range: object, text: string) => void} ctx.replaceText
   * @param {(text: string) => void} ctx.showInformationMessage
   * @param {(key: string, opts: object, onClick: (()=>void)|null) => void} ctx.setStatusBarItem
   * @param {(key: string) => void} ctx.removeStatusBarItem
   * @param {(path: string) => Promise<string>} ctx.readFile
   * @param {(path: string, content: string) => Promise<void>} ctx.writeFile
   * @param {(path: string) => Promise<object[]>} ctx.listDir
   * @param {() => string|null} ctx.getFilePath
   * @param {() => string} ctx.getLanguage
   * @param {() => number} ctx.getLineCount
   * @param {(n: number) => string} ctx.getLine
   * @param {(decorations: object[]) => string} ctx.setDecorations
   * @param {(handle: string) => void} ctx.clearDecorations
   * @param {(url: string, opts: object) => Promise<object>} ctx.networkFetch
   * @param {(uri: string, diagnostics: object[]) => void} ctx.setDiagnostics
   * @param {(uri: string) => void} ctx.clearDiagnostics
   * @param {() => void} [ctx.onChange] called when commands/extensions change
   */
  constructor(ctx) {
    this.ctx = ctx;
    /** id -> { manifest, worker, commands:Set<string>, ready:Promise } */
    this.active = new Map();
    this.rpcSeq = 0;
    this.pending = new Map(); // reqId -> { resolve, reject }
  }

  notifyChange() {
    this.ctx.onChange?.();
  }

  /** Activate one installed+enabled extension. */
  async activate(installed, manager) {
    const { manifest } = installed;
    if (this.active.has(manifest.id)) return;
    let source;
    try {
      source = await manager.readAsset(manifest.id, manifest.main || "index.js");
    } catch (err) {
      console.error(`[extensions] failed to read ${manifest.id}:`, err);
      return;
    }
    const worker = new SandboxWorker();
    const entry = { manifest, worker, commands: new Set() };
    this.active.set(manifest.id, entry);
    worker.onmessage = (event) => this.handleMessage(manifest.id, event.data);
    worker.onerror = (err) =>
      console.error(`[extensions] worker error in ${manifest.id}:`, err.message || err);
    worker.postMessage({
      t: "activate",
      id: manifest.id,
      source,
      permissions: manifest.permissions || [],
    });
  }

  /** Tear down a running extension. */
  deactivate(id) {
    const entry = this.active.get(id);
    if (!entry) return;
    try {
      entry.worker.postMessage({ t: "deactivate" });
    } catch {
      /* ignore */
    }
    // Give the worker a moment to run deactivate(), then terminate.
    setTimeout(() => entry.worker.terminate(), 60);
    for (const key of [...this.statusKeysFor(id)]) this.ctx.removeStatusBarItem(key);
    this.active.delete(id);
    this.notifyChange();
  }

  // Resolve an extension-supplied path and guarantee it stays inside an open
  // workspace root. Returns the normalized absolute path or throws. This is the
  // last line of defense for the file APIs: extensions run in a Worker sandbox
  // and can only reach the filesystem through this host, so enforcing the
  // boundary here blocks path-traversal sandbox escapes.
  resolveWorkspacePath(rawPath) {
    const roots = (this.ctx.getWorkspaceRoots?.() || []).filter(Boolean).map(normalizePath);
    if (!roots.length) throw new Error("no workspace is open");
    const raw = String(rawPath ?? "");
    const candidates = raw.startsWith("/")
      ? [normalizePath(raw)]
      : roots.map((root) => normalizePath(root + "/" + raw));
    for (const abs of candidates) {
      if (roots.some((root) => abs === root || abs.startsWith(root + "/"))) return abs;
    }
    throw new Error(`access denied: "${raw}" is outside the workspace`);
  }

  statusKeysFor(id) {
    return this._statusKeys?.get(id) || new Set();
  }

  rememberStatusKey(id, key) {
    this._statusKeys = this._statusKeys || new Map();
    if (!this._statusKeys.has(id)) this._statusKeys.set(id, new Set());
    this._statusKeys.get(id).add(key);
  }

  handleMessage(extId, msg) {
    const entry = this.active.get(extId);
    if (!entry || !msg) return;
    switch (msg.t) {
      case "register":
        entry.commands.add(msg.id);
        this.notifyChange();
        break;
      case "activated":
        this.notifyChange();
        break;
      case "error":
        console.error(`[extensions] ${extId} ${msg.phase}: ${msg.message}`);
        this.ctx.showInformationMessage?.(`Extension "${entry.manifest.name}" error: ${msg.message}`);
        break;
      case "rpc":
        this.handleRpc(entry, msg);
        break;
      default:
        break;
    }
  }

  async handleRpc(entry, msg) {
    const { worker } = entry;
    const reply = (ok, value, error) =>
      worker.postMessage({ t: "rpcResult", reqId: msg.reqId, ok, value, error });

    const required = PERMISSION_FOR[msg.method];
    if (required && !(entry.manifest.permissions || []).includes(required)) {
      reply(false, undefined, `permission "${required}" not granted to ${entry.manifest.id}`);
      return;
    }

    try {
      const value = await this.dispatch(entry, msg.method, msg.args || []);
      reply(true, value);
    } catch (err) {
      reply(false, undefined, String(err && err.message ? err.message : err));
    }
  }

  async dispatch(entry, method, args) {
    switch (method) {
      case "window.showInformationMessage":
        this.ctx.showInformationMessage(args[0]);
        return null;
      case "window.setStatusBarItem": {
        const [itemId, opts] = args;
        const key = `${entry.manifest.id}::${itemId}`;
        const command = opts?.command;
        const onClick = command ? () => this.invokeCommand(command) : null;
        this.ctx.setStatusBarItem(key, opts || {}, onClick);
        this.rememberStatusKey(entry.manifest.id, key);
        return null;
      }
      case "window.removeStatusBarItem": {
        const key = `${entry.manifest.id}::${args[0]}`;
        this.ctx.removeStatusBarItem(key);
        return null;
      }
      case "editor.getText":
        return this.ctx.getEditorText();
      case "editor.getSelection":
        return this.ctx.getSelectionText();
      case "editor.insertText":
        this.ctx.insertText(args[0]);
        return null;
      case "editor.replaceText":
        this.ctx.replaceText(args[0], args[1]);
        return null;
      case "editor.setDecorations": {
        const handle = this.ctx.setDecorations(entry.manifest.id, args[0]);
        return handle;
      }
      case "editor.clearDecorations":
        this.ctx.clearDecorations(entry.manifest.id);
        return null;
      case "editor.getFilePath":
        return this.ctx.getFilePath();
      case "editor.getLanguage":
        return this.ctx.getLanguage();
      case "editor.getLineCount":
        return this.ctx.getLineCount();
      case "editor.getLine":
        return this.ctx.getLine(args[0]);
      case "workspace.readFile":
        return this.ctx.readFile(this.resolveWorkspacePath(args[0]));
      case "workspace.writeFile":
        await this.ctx.writeFile(this.resolveWorkspacePath(args[0]), args[1]);
        return null;
      case "workspace.listDir":
        return this.ctx.listDir(this.resolveWorkspacePath(args[0]));
      case "network.fetch":
        return this.ctx.networkFetch(args[0], args[1]);
      case "diagnostics.set":
        this.ctx.setDiagnostics(entry.manifest.id, args[0], args[1]);
        return null;
      case "diagnostics.clear":
        this.ctx.clearDiagnostics(entry.manifest.id, args[0]);
        return null;
      case "locale.registerLocale":
        this.ctx.registerLocale(args[0], args[1]);
        return null;
      case "locale.setLocale":
        this.ctx.setLocale(args[0]);
        return null;
      default:
        throw new Error(`unknown host method: ${method}`);
    }
  }

  /** Invoke a command registered by any active extension. */
  invokeCommand(commandId) {
    for (const entry of this.active.values()) {
      if (entry.commands.has(commandId)) {
        const reqId = `i${this.rpcSeq++}`;
        return new Promise((resolve) => {
          const onMsg = (event) => {
            const m = event.data;
            if (m && m.t === "invokeResult" && m.reqId === reqId) {
              entry.worker.removeEventListener("message", onMsg);
              if (!m.ok) this.ctx.showInformationMessage?.(`Command failed: ${m.error}`);
              resolve(m.ok);
            }
          };
          entry.worker.addEventListener("message", onMsg);
          entry.worker.postMessage({ t: "invoke", id: commandId, reqId, args: [] });
        });
      }
    }
    this.ctx.showInformationMessage?.(`No extension handles command: ${commandId}`);
    return Promise.resolve(false);
  }

  /** Commands contributed by active extensions, with display titles. */
  listCommands() {
    const out = [];
    for (const entry of this.active.values()) {
      const titles = new Map(
        (entry.manifest.contributes?.commands || []).map((c) => [c.id, c.title])
      );
      for (const id of entry.commands) {
        out.push({
          id,
          title: titles.get(id) || id,
          category: entry.manifest.name,
        });
      }
    }
    return out;
  }
}
