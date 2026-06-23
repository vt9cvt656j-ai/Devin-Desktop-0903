// Michael IDE — real Language Server Protocol client.
//
// The Rust backend (`lsp.rs`) only handles process spawning and Content-Length
// framing: it forwards each decoded LSP message to the frontend as a clean JSON
// string and writes whatever JSON we hand back. This module is the actual LSP
// *client*: it performs the initialize handshake, mirrors document lifecycle
// (didOpen/didChange/didSave/didClose), turns `publishDiagnostics` into Monaco
// markers, and registers Monaco language-feature providers (completion, hover,
// definition, references, rename, document symbols, signature help, formatting,
// code actions) that proxy to the running servers.
//
// Monaco already ships an excellent built-in language service for
// TS/JS/JSON/CSS/HTML, so this client targets the "gap" languages that have no
// in-browser intelligence (Rust, Python, Go, C/C++) and any custom server the
// user starts manually.
import * as monaco from "monaco-editor";

// Languages we auto-start + wire Monaco providers for. Monaco's bundled service
// covers ts/js/json/css/html, so we deliberately leave those to Monaco to avoid
// duplicate completions and diagnostics.
const MANAGED_LANGS = ["rust", "python", "go", "c", "cpp"];

// Monaco language id -> the `lang` key the backend's KNOWN_SERVERS table uses.
const SERVER_LANG = {
  rust: "rust",
  python: "python",
  go: "go",
  c: "c",
  cpp: "cpp",
  typescript: "typescript",
  javascript: "javascript",
  html: "html",
  css: "css",
  json: "json",
};

// Monaco language id -> the `languageId` reported in textDocument items. Most
// match 1:1; this exists for the few that differ.
const DOC_LANGUAGE_ID = {
  rust: "rust",
  python: "python",
  go: "go",
  c: "c",
  cpp: "cpp",
  typescript: "typescript",
  javascript: "javascript",
};

const Sev = monaco.MarkerSeverity;
const LSP_TO_SEVERITY = { 1: Sev.Error, 2: Sev.Warning, 3: Sev.Info, 4: Sev.Hint };

const CK = monaco.languages.CompletionItemKind;
const LSP_TO_COMPLETION_KIND = {
  1: CK.Text, 2: CK.Method, 3: CK.Function, 4: CK.Constructor, 5: CK.Field,
  6: CK.Variable, 7: CK.Class, 8: CK.Interface, 9: CK.Module, 10: CK.Property,
  11: CK.Unit, 12: CK.Value, 13: CK.Enum, 14: CK.Keyword, 15: CK.Snippet,
  16: CK.Color, 17: CK.File, 18: CK.Reference, 19: CK.Folder, 20: CK.EnumMember,
  21: CK.Constant, 22: CK.Struct, 23: CK.Event, 24: CK.Operator, 25: CK.TypeParameter,
};

const REQUEST_TIMEOUT_MS = 20000;

const PYTHON_SETTINGS = {
  pythonPath: "python3",
  analysis: {
    autoSearchPaths: true,
    diagnosticMode: "openFilesOnly",
    typeCheckingMode: "standard",
    useLibraryCodeForTypes: true,
    autoImportCompletions: true,
    extraPaths: [],
  },
};


// ---- coordinate conversions (LSP is 0-based, Monaco is 1-based) ----
function toMonacoPosition(p) {
  return { lineNumber: (p.line ?? 0) + 1, column: (p.character ?? 0) + 1 };
}
function toMonacoRange(r) {
  if (!r) return undefined;
  return {
    startLineNumber: (r.start.line ?? 0) + 1,
    startColumn: (r.start.character ?? 0) + 1,
    endLineNumber: (r.end.line ?? 0) + 1,
    endColumn: (r.end.character ?? 0) + 1,
  };
}
function fromMonacoPosition(pos) {
  return { line: pos.lineNumber - 1, character: pos.column - 1 };
}
function pathToUri(path) {
  return monaco.Uri.file(path).toString();
}
function baseName(p) {
  return p.replace(/[/\\]+$/, "").split(/[/\\]/).pop() || p;
}

function lspMarkupToString(content) {
  if (content == null) return "";
  if (typeof content === "string") return content;
  if (typeof content.value === "string") {
    // MarkedString { language, value } -> fenced code block.
    if (content.language) return "```" + content.language + "\n" + content.value + "\n```";
    return content.value;
  }
  return "";
}

function hoverContentsToMarkdown(contents) {
  const list = Array.isArray(contents) ? contents : [contents];
  return list
    .map(lspMarkupToString)
    .filter(Boolean)
    .map((value) => ({ value, isTrusted: false }));
}

function completionDocsToMonaco(doc) {
  if (doc == null) return undefined;
  if (typeof doc === "string") return doc;
  if (typeof doc.value === "string") return { value: doc.value };
  return undefined;
}

// ---- a single language server connection ----
class LspClient {
  constructor(lang, manager) {
    this.lang = lang;
    this.manager = manager;
    this.serverLang = SERVER_LANG[lang] || lang;
    this.nextId = 1;
    this.pending = new Map();
    this.capabilities = {};
    this.initialized = false;
    this.initPromise = null;
    this.openDocs = new Map(); // uri -> version
    this.disposed = false;
    this.logLines = [];
  }

  log(line) {
    this.logLines.push(line);
    if (this.logLines.length > 400) this.logLines.shift();
    this.manager.onLog?.(this.lang, line);
  }

  async start(custom) {
    if (this.initPromise) return this.initPromise;
    this.initPromise = this._startInner(custom);
    return this.initPromise;
  }

  async _startInner(custom) {
    const config = {
      lang: this.serverLang,
      command: custom?.command || "",
      args: custom?.args || [],
      rootUri: this.manager.primaryRootUri() || "",
    };
    try {
      await this.manager.backend.lspStart(config, (ev) => this._onEvent(ev));
    } catch (e) {
      this.initPromise = null;
      throw e;
    }
    await this._initialize();
    return this;
  }

  _onEvent(ev) {
    if (this.disposed) return;
    switch (ev?.kind) {
      case "message":
        this._onMessage(ev.data);
        break;
      case "started":
        this.log(`[started] ${ev.lang}`);
        break;
      case "error":
        this.log(`[error] ${ev.message}`);
        break;
      case "stopped":
        this.log(`[stopped] ${ev.lang}`);
        this.manager._handleStopped(this.lang);
        break;
      default:
        break;
    }
  }

  _onMessage(raw) {
    let msg;
    try {
      msg = JSON.parse(raw);
    } catch {
      return;
    }
    // Response to one of our requests.
    if (msg.id !== undefined && (msg.result !== undefined || msg.error !== undefined)) {
      const pending = this.pending.get(msg.id);
      if (pending) {
        this.pending.delete(msg.id);
        clearTimeout(pending.timer);
        if (msg.error) pending.resolve(null);
        else pending.resolve(msg.result);
      }
      return;
    }
    // Server -> client request (needs a reply).
    if (msg.method && msg.id !== undefined) {
      this._handleServerRequest(msg);
      return;
    }
    // Notification.
    if (msg.method) this._handleNotification(msg);
  }

  _handleServerRequest(msg) {
    const reply = (result) => this._respond(msg.id, result);
    switch (msg.method) {
      case "workspace/configuration": {
        const items = msg.params?.items || [];
        reply(items.map((item) => this._getConfigForSection(item.section)));
        break;
      }
      case "workspace/applyEdit": {
        const ok = this.manager.applyWorkspaceEdit(msg.params?.edit);
        reply({ applied: ok });
        break;
      }
      case "client/registerCapability":
      case "client/unregisterCapability":
      case "window/workDoneProgress/create":
      case "workspace/semanticTokens/refresh":
      case "workspace/codeLens/refresh":
      case "workspace/inlayHint/refresh":
      case "workspace/diagnostic/refresh":
        reply(null);
        break;
      default:
        // Be lenient: reply null so servers never hang waiting on us.
        reply(null);
        break;
    }
  }

  _handleNotification(msg) {
    switch (msg.method) {
      case "textDocument/publishDiagnostics":
        this.manager.applyDiagnostics(this.lang, msg.params);
        break;
      case "window/logMessage":
      case "window/showMessage":
        if (msg.params?.message) this.log(`[srv] ${msg.params.message}`);
        break;
      default:
        break;
    }
  }

  _send(method, params) {
    const payload = JSON.stringify({ jsonrpc: "2.0", method, params });
    return this.manager.backend.lspSend(this.serverLang, payload).catch(() => {});
  }

  _respond(id, result) {
    const payload = JSON.stringify({ jsonrpc: "2.0", id, result });
    return this.manager.backend.lspSend(this.serverLang, payload).catch(() => {});
  }

  request(method, params) {
    const id = this.nextId++;
    const payload = JSON.stringify({ jsonrpc: "2.0", id, method, params });
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        resolve(null);
      }, REQUEST_TIMEOUT_MS);
      this.pending.set(id, { resolve, timer });
      this.manager.backend.lspSend(this.serverLang, payload).catch(() => {
        const p = this.pending.get(id);
        if (p) {
          this.pending.delete(id);
          clearTimeout(p.timer);
          resolve(null);
        }
      });
    });
  }

  async _initialize() {
    const roots = this.manager.workspaceRoots();
    const primary = roots[0] || null;
    const params = {
      processId: null,
      clientInfo: { name: "Michael IDE", version: "0.1.0" },
      locale: "en",
      rootUri: primary ? pathToUri(primary) : null,
      rootPath: primary || null,
      workspaceFolders: roots.length
        ? roots.map((r) => ({ uri: pathToUri(r), name: baseName(r) }))
        : null,
      capabilities: clientCapabilities(),
      initializationOptions: this._getInitOptions(),
    };
    const result = await this.request("initialize", params);
    this.capabilities = result?.capabilities || {};
    this._send("initialized", {});
    this._send("workspace/didChangeConfiguration", {
      settings: this._getLangSettings(),
    });
    this.initialized = true;
    this.manager.onStatus?.();
    this.log("[initialized]");
    return this;
  }

  _getInitOptions() {
    if (this.serverLang === "python") {
      return this.manager._pythonSettings || {};
    }
    return {};
  }

  _getLangSettings() {
    if (this.serverLang === "python") {
      return { python: this.manager._pythonSettings || {} };
    }
    return {};
  }

  _getConfigForSection(section) {
    if (this.serverLang === "python") {
      const pythonSettings = this.manager._pythonSettings || {};
      if (!section || section === "python") return pythonSettings;
      if (section === "python.analysis") return pythonSettings.analysis || {};
      const parts = section.split(".");
      let obj = pythonSettings;
      for (const part of parts.slice(parts[0] === "python" ? 1 : 0)) {
        obj = obj?.[part];
        if (obj === undefined) return {};
      }
      return obj ?? {};
    }
    return {};
  }

  supports(name) {
    const c = this.capabilities || {};
    switch (name) {
      case "completion": return !!c.completionProvider;
      case "hover": return !!c.hoverProvider;
      case "definition": return !!c.definitionProvider;
      case "references": return !!c.referencesProvider;
      case "rename": return !!c.renameProvider;
      case "documentSymbol": return !!c.documentSymbolProvider;
      case "signatureHelp": return !!c.signatureHelpProvider;
      case "formatting": return !!c.documentFormattingProvider;
      case "codeAction": return !!c.codeActionProvider;
      case "resolveCompletion": return !!(c.completionProvider && c.completionProvider.resolveProvider);
      case "inlayHint": return !!c.inlayHintProvider;
      case "semanticTokens": return !!(c.semanticTokensProvider);
      case "implementation": return !!c.implementationProvider;
      case "typeDefinition": return !!c.typeDefinitionProvider;
      case "declaration": return !!c.declarationProvider;
      case "callHierarchy": return !!c.callHierarchyProvider;
      case "codeLens": return !!c.codeLensProvider;
      case "documentHighlight": return !!c.documentHighlightProvider;
      case "colorProvider": return !!c.colorProvider;
      case "linkedEditingRange": return !!c.linkedEditingRangeProvider;
      case "foldingRange": return !!c.foldingRangeProvider;
      case "onTypeFormatting": return !!c.documentOnTypeFormattingProvider;
      case "rangeFormatting": return !!c.documentRangeFormattingProvider;
      case "selectionRange": return !!c.selectionRangeProvider;
      case "documentLink": return !!c.documentLinkProvider;
      case "workspaceSymbol": return !!c.workspaceSymbolProvider;
      default: return false;
    }
  }

  completionTriggerCharacters() {
    return this.capabilities?.completionProvider?.triggerCharacters || [];
  }
  signatureTriggerCharacters() {
    return this.capabilities?.signatureHelpProvider?.triggerCharacters || [];
  }

  didOpen(uri, languageId, version, text) {
    if (this.openDocs.has(uri)) return;
    this.openDocs.set(uri, version);
    this._send("textDocument/didOpen", {
      textDocument: { uri, languageId, version, text },
    });
  }

  didChange(uri, version, text) {
    if (!this.openDocs.has(uri)) return;
    this.openDocs.set(uri, version);
    this._send("textDocument/didChange", {
      textDocument: { uri, version },
      contentChanges: [{ text }],
    });
  }

  didSave(uri, text) {
    if (!this.openDocs.has(uri)) return;
    this._send("textDocument/didSave", {
      textDocument: { uri },
      text,
    });
  }

  didClose(uri) {
    if (!this.openDocs.has(uri)) return;
    this.openDocs.delete(uri);
    this._send("textDocument/didClose", { textDocument: { uri } });
  }

  shutdown() {
    this.disposed = true;
    for (const { timer, resolve } of this.pending.values()) {
      clearTimeout(timer);
      resolve(null);
    }
    this.pending.clear();
  }
}

function clientCapabilities() {
  return {
    textDocument: {
      synchronization: { dynamicRegistration: false, willSave: false, didSave: true },
      completion: {
        dynamicRegistration: false,
        contextSupport: true,
        completionItem: {
          snippetSupport: true,
          commitCharactersSupport: false,
          documentationFormat: ["markdown", "plaintext"],
          deprecatedSupport: true,
          preselectSupport: true,
          insertReplaceSupport: true,
          resolveSupport: { properties: ["documentation", "detail", "additionalTextEdits"] },
        },
        completionItemKind: { valueSet: Array.from({ length: 25 }, (_, i) => i + 1) },
      },
      hover: { dynamicRegistration: false, contentFormat: ["markdown", "plaintext"] },
      signatureHelp: {
        dynamicRegistration: false,
        signatureInformation: {
          documentationFormat: ["markdown", "plaintext"],
          parameterInformation: { labelOffsetSupport: true },
          activeParameterSupport: true,
        },
      },
      definition: { dynamicRegistration: false, linkSupport: true },
      declaration: { dynamicRegistration: false, linkSupport: true },
      implementation: { dynamicRegistration: false, linkSupport: true },
      typeDefinition: { dynamicRegistration: false, linkSupport: true },
      references: { dynamicRegistration: false },
      callHierarchy: { dynamicRegistration: false },
      codeLens: { dynamicRegistration: false },
      documentHighlight: { dynamicRegistration: false },
      colorProvider: { dynamicRegistration: false },
      linkedEditingRange: { dynamicRegistration: false },
      foldingRange: { dynamicRegistration: false, foldingRangeKind: { valueSet: ["comment", "imports", "region"] }, lineFoldingOnly: true },
      onTypeFormatting: { dynamicRegistration: false },
      rangeFormatting: { dynamicRegistration: false },
      selectionRange: { dynamicRegistration: false },
      documentLink: { dynamicRegistration: false, tooltipSupport: true },
      documentSymbol: {
        dynamicRegistration: false,
        hierarchicalDocumentSymbolSupport: true,
        symbolKind: { valueSet: Array.from({ length: 26 }, (_, i) => i + 1) },
      },
      formatting: { dynamicRegistration: false },
      rename: { dynamicRegistration: false, prepareSupport: false },
      publishDiagnostics: { relatedInformation: true, versionSupport: false },
      codeAction: {
        dynamicRegistration: false,
        codeActionLiteralSupport: {
          codeActionKind: {
            valueSet: ["", "quickfix", "refactor", "refactor.extract", "refactor.inline", "refactor.rewrite", "source", "source.organizeImports"],
          },
        },
        isPreferredSupport: true,
        resolveSupport: { properties: ["edit"] },
      },
      inlayHint: {
        dynamicRegistration: false,
        resolveSupport: { properties: ["tooltip", "textEdits", "label.tooltip", "label.location", "label.command"] },
      },
      semanticTokens: {
        dynamicRegistration: false,
        tokenTypes: [
          "namespace","type","class","enum","interface","struct","typeParameter",
          "parameter","variable","property","enumMember","event","function",
          "method","macro","keyword","modifier","comment","string","number",
          "regexp","operator","decorator",
        ],
        tokenModifiers: [
          "declaration","definition","readonly","static","deprecated",
          "abstract","async","modification","documentation","defaultLibrary",
        ],
        formats: ["relative"],
        requests: { full: { delta: false }, range: false },
        multilineTokenSupport: false,
        overlappingTokenSupport: false,
      },
    },
    workspace: {
      applyEdit: true,
      configuration: true,
      workspaceFolders: true,
      didChangeConfiguration: { dynamicRegistration: true },
      didChangeWatchedFiles: { dynamicRegistration: false },
      symbol: { dynamicRegistration: false },
      executeCommand: { dynamicRegistration: false },
    },
    window: { workDoneProgress: true, showMessage: {}, showDocument: { support: false } },
  };
}

export function createLspManager(options) {
  const {
    backend,
    enabled = true,
    getWorkspaceRoots = () => [],
    showToast = () => {},
    showNotification = null,
    onLog = null,
    onStatus = null,
  } = options;

  const clients = new Map(); // monaco lang id -> LspClient
  const changeTimers = new Map(); // uri -> { timer, langId, modelUri }
  const lazyModels = new Set(); // uris of models we created for cross-file diagnostics
  let executeCommandRegistered = false;

  const manager = {
    backend,
    onLog,
    onStatus,
    workspaceRoots,
    primaryRootUri,
    applyDiagnostics,
    applyWorkspaceEdit,
    _handleStopped,
    _pythonSettings: null,
  };

  function workspaceRoots() {
    const roots = getWorkspaceRoots() || [];
    return Array.isArray(roots) ? roots.filter(Boolean) : [];
  }
  function primaryRootUri() {
    const r = workspaceRoots()[0];
    return r ? pathToUri(r) : "";
  }

  function isManaged(langId) {
    return MANAGED_LANGS.includes(langId);
  }

  function clientForModel(model) {
    if (!model) return null;
    return clients.get(model.getLanguageId()) || null;
  }

  function _handleStopped(langId) {
    const client = clients.get(langId);
    if (client) {
      client.shutdown();
      clients.delete(langId);
    }
    // Clear any markers owned by this server.
    for (const m of monaco.editor.getModels()) {
      monaco.editor.setModelMarkers(m, "lsp:" + langId, []);
    }
    onStatus?.();
  }

  async function ensureServer(langId, custom) {
    if (!enabled) return null;
    let client = clients.get(langId);
    if (client) {
      if (client.initPromise) {
        try { await client.initPromise; } catch { /* fall through */ }
      }
      return clients.get(langId) || null;
    }
    if (langId === "python" && !manager._pythonSettings) {
      try {
        const info = await backend.lspDetectPython();
        if (info && info.pythonPath) {
          manager._pythonSettings = {
            pythonPath: info.pythonPath,
            analysis: {
              ...PYTHON_SETTINGS.analysis,
              extraPaths: info.sitePackages || [],
            },
          };
        } else {
          manager._pythonSettings = PYTHON_SETTINGS;
        }
      } catch {
        manager._pythonSettings = PYTHON_SETTINGS;
      }
    }
    client = new LspClient(langId, manager);
    clients.set(langId, client);
    try {
      await client.start(custom);
      onStatus?.();
      showToast(`Language server started: ${langId}`);
      return client;
    } catch (e) {
      clients.delete(langId);
      const msg = String(e && e.message ? e.message : e);
      const alreadyRunning = /already running/i.test(msg);
      if (alreadyRunning) { onLog?.(`[lsp] ${langId}: ${msg}`); return null; }
      const installHints = {
        python: "npm i -g pyright",
        rust: "rustup component add rust-analyzer",
        go: "go install golang.org/x/tools/gopls@latest",
        c: "brew install llvm (clangd)",
        cpp: "brew install llvm (clangd)",
      };
      const names = { python: "Pyright", rust: "rust-analyzer", go: "gopls", c: "clangd", cpp: "clangd" };
      const hint = installHints[langId];
      let toolExists = false;
      try { toolExists = await backend.lspCheckAvailable(langId); } catch { /* ignore */ }
      if (!toolExists && hint && showNotification) {
        showNotification({
          title: `缺少 ${names[langId] || langId} 语言服务器`,
          message: `安装后可获得智能补全、跳转定义、悬停文档等功能`,
          actionLabel: "安装",
          duration: 20000,
          installCmd: hint,
        });
      } else if (toolExists) {
        showToast(`${names[langId] || langId} 启动失败: ${msg}`);
      } else {
        showToast(`LSP ${langId}: ${msg}`);
      }
      onLog?.(`[lsp] ${langId}: ${msg}`);
      return null;
    }
  }

  // ---- document lifecycle ----
  function didOpen(path, model) {
    if (!enabled || !model) return;
    const langId = model.getLanguageId();
    if (!isManaged(langId)) return;
    const uri = model.uri.toString();
    const docLang = DOC_LANGUAGE_ID[langId] || langId;
    const version = model.getVersionId();
    const text = model.getValue();
    ensureServer(langId).then((client) => {
      if (client) client.didOpen(uri, docLang, version, text);
    });
  }

  function didChange(path, model) {
    if (!enabled || !model) return;
    const langId = model.getLanguageId();
    if (!isManaged(langId)) return;
    const client = clients.get(langId);
    if (!client || !client.initialized) return;
    const uri = model.uri.toString();
    const existing = changeTimers.get(uri);
    if (existing) clearTimeout(existing.timer);
    const timer = setTimeout(() => {
      changeTimers.delete(uri);
      const live = monaco.editor.getModel(model.uri);
      if (!live) return;
      client.didChange(uri, live.getVersionId(), live.getValue());
    }, 180);
    changeTimers.set(uri, { timer, langId, modelUri: model.uri });
  }

  function didSave(path, model) {
    if (!enabled || !model) return;
    const langId = model.getLanguageId();
    if (!isManaged(langId)) return;
    const client = clients.get(langId);
    if (!client) return;
    client.didSave(model.uri.toString(), model.getValue());
  }

  function didClose(path) {
    if (!enabled || !path) return;
    const uri = pathToUri(path);
    for (const client of clients.values()) client.didClose(uri);
  }

  function flushPendingChange(uri) {
    const entry = changeTimers.get(uri);
    if (!entry) return;
    clearTimeout(entry.timer);
    changeTimers.delete(uri);
    const live = monaco.editor.getModel(entry.modelUri);
    if (!live) return;
    const client = clients.get(entry.langId);
    if (client && client.initialized) {
      client.didChange(uri, live.getVersionId(), live.getValue());
    }
  }

  // ---- diagnostics ----
  async function applyDiagnostics(langId, params) {
    if (!params?.uri) return;
    const uri = params.uri;
    let model = findModelByUri(uri);
    if (!model) {
      // Create a lightweight model so cross-file diagnostics still surface in
      // the Problems panel. Capped to avoid model explosion on huge outputs.
      const diags = params.diagnostics || [];
      if (!diags.length) return;
      if (lazyModels.size > 150) return;
      model = await lazilyCreateModel(uri);
      if (!model) return;
    }
    const owner = "lsp:" + langId;
    const markers = (params.diagnostics || []).map((d) => diagnosticToMarker(d));
    monaco.editor.setModelMarkers(model, owner, markers);
  }

  function diagnosticToMarker(d) {
    const r = toMonacoRange(d.range) || { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 };
    const related = (d.relatedInformation || []).map((ri) => ({
      resource: monaco.Uri.parse(ri.location.uri),
      message: ri.message,
      startLineNumber: (ri.location.range.start.line ?? 0) + 1,
      startColumn: (ri.location.range.start.character ?? 0) + 1,
      endLineNumber: (ri.location.range.end.line ?? 0) + 1,
      endColumn: (ri.location.range.end.character ?? 0) + 1,
    }));
    return {
      severity: LSP_TO_SEVERITY[d.severity] ?? Sev.Error,
      message: d.message || "",
      code: typeof d.code === "object" ? d.code?.value : d.code,
      source: d.source,
      startLineNumber: r.startLineNumber,
      startColumn: r.startColumn,
      endLineNumber: r.endLineNumber,
      endColumn: r.endColumn,
      relatedInformation: related.length ? related : undefined,
      tags: (d.tags || []).map((tg) => (tg === 1 ? monaco.MarkerTag.Unnecessary : monaco.MarkerTag.Deprecated)),
    };
  }

  function findModelByUri(uri) {
    const parsed = monaco.Uri.parse(uri);
    return monaco.editor.getModel(parsed) || monaco.editor.getModels().find((m) => m.uri.toString() === uri) || null;
  }

  async function lazilyCreateModel(uri) {
    try {
      const parsed = monaco.Uri.parse(uri);
      const path = parsed.fsPath;
      const content = await backend.readTextFile(path);
      let model = monaco.editor.getModel(parsed);
      if (!model) {
        model = monaco.editor.createModel(content ?? "", undefined, parsed);
        lazyModels.add(uri);
      }
      return model;
    } catch {
      return null;
    }
  }

  // ---- workspace edits (rename, code actions, server applyEdit) ----
  function applyWorkspaceEdit(edit) {
    if (!edit) return false;
    const byUri = new Map();
    if (edit.changes) {
      for (const [uri, edits] of Object.entries(edit.changes)) byUri.set(uri, edits);
    }
    if (Array.isArray(edit.documentChanges)) {
      for (const dc of edit.documentChanges) {
        if (dc.textDocument && dc.edits) byUri.set(dc.textDocument.uri, dc.edits);
      }
    }
    let applied = false;
    for (const [uri, edits] of byUri.entries()) {
      const model = findModelByUri(uri);
      if (!model) continue;
      const ops = edits.map((e) => ({
        range: toMonacoRange(e.range),
        text: e.newText,
        forceMoveMarkers: true,
      }));
      model.pushEditOperations([], ops, () => null);
      applied = true;
    }
    return applied;
  }

  function toMonacoWorkspaceEdit(edit) {
    const edits = [];
    const pushEdits = (uri, list) => {
      const resource = monaco.Uri.parse(uri);
      for (const e of list) {
        edits.push({ resource, textEdit: { range: toMonacoRange(e.range), text: e.newText }, versionId: undefined });
      }
    };
    if (edit?.changes) {
      for (const [uri, list] of Object.entries(edit.changes)) pushEdits(uri, list);
    }
    if (Array.isArray(edit?.documentChanges)) {
      for (const dc of edit.documentChanges) {
        if (dc.textDocument && dc.edits) pushEdits(dc.textDocument.uri, dc.edits);
      }
    }
    return { edits };
  }

  // ---- Monaco provider registration ----
  function registerProviders() {
    if (!enabled) return;
    registerExecuteCommand();

    monaco.languages.registerCompletionItemProvider(MANAGED_LANGS, {
      triggerCharacters: [".", ":", ">", "<", "\"", "'", "/", "@", "(", "#", "$", "*", "&"],
      async provideCompletionItems(model, position, context) {
        const client = clientForModel(model);
        if (!client || !client.supports("completion")) return { suggestions: [] };
        flushPendingChange(model.uri.toString());
        const lspTrigger = (context.triggerKind || 0) + 1;
        const lspContext = { triggerKind: lspTrigger };
        if (context.triggerCharacter) lspContext.triggerCharacter = context.triggerCharacter;
        const result = await client.request("textDocument/completion", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
          context: lspContext,
        });
        if (!result) return { suggestions: [] };
        const items = Array.isArray(result) ? result : result.items || [];
        const isIncomplete = !Array.isArray(result) && !!result.isIncomplete;
        if (items.length > 0 && manager.onCompletionSymbols) {
          const labels = items.map((it) => typeof it.label === "string" ? it.label : it.label?.label).filter(Boolean);
          try { manager.onCompletionSymbols(labels); } catch { /* ignore */ }
        }
        const word = model.getWordUntilPosition(position);
        const defaultRange = new monaco.Range(position.lineNumber, word.startColumn, position.lineNumber, word.endColumn);
        return {
          incomplete: isIncomplete,
          suggestions: items.map((it) => completionToMonaco(it, defaultRange, client.lang)),
        };
      },
      async resolveCompletionItem(item) {
        const client = clients.get(item.__lspLang);
        if (!client || !client.supports("resolveCompletion") || !item.__lspItem) return item;
        const resolved = await client.request("completionItem/resolve", item.__lspItem);
        if (!resolved) return item;
        if (resolved.documentation) item.documentation = completionDocsToMonaco(resolved.documentation);
        if (resolved.detail) item.detail = resolved.detail;
        if (Array.isArray(resolved.additionalTextEdits)) {
          item.additionalTextEdits = resolved.additionalTextEdits.map((e) => ({
            range: toMonacoRange(e.range),
            text: e.newText,
          }));
        }
        return item;
      },
    });

    monaco.languages.registerHoverProvider(MANAGED_LANGS, {
      async provideHover(model, position) {
        const client = clientForModel(model);
        if (!client || !client.supports("hover")) return null;
        flushPendingChange(model.uri.toString());
        const result = await client.request("textDocument/hover", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
        });
        if (!result || !result.contents) return null;
        const contents = hoverContentsToMarkdown(result.contents);
        if (!contents.length) return null;
        return { range: toMonacoRange(result.range), contents };
      },
    });

    monaco.languages.registerDefinitionProvider(MANAGED_LANGS, {
      async provideDefinition(model, position) {
        const client = clientForModel(model);
        if (!client || !client.supports("definition")) return null;
        const result = await client.request("textDocument/definition", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
        });
        return locationsToMonaco(result);
      },
    });

    monaco.languages.registerImplementationProvider(MANAGED_LANGS, {
      async provideImplementation(model, position) {
        const client = clientForModel(model);
        if (!client || !client.supports("implementation")) return null;
        const result = await client.request("textDocument/implementation", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
        });
        return locationsToMonaco(result);
      },
    });

    monaco.languages.registerTypeDefinitionProvider(MANAGED_LANGS, {
      async provideTypeDefinition(model, position) {
        const client = clientForModel(model);
        if (!client || !client.supports("typeDefinition")) return null;
        const result = await client.request("textDocument/typeDefinition", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
        });
        return locationsToMonaco(result);
      },
    });

    monaco.languages.registerDeclarationProvider(MANAGED_LANGS, {
      async provideDeclaration(model, position) {
        const client = clientForModel(model);
        if (!client || !client.supports("declaration")) return null;
        const result = await client.request("textDocument/declaration", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
        });
        return locationsToMonaco(result);
      },
    });

    monaco.languages.registerReferenceProvider(MANAGED_LANGS, {
      async provideReferences(model, position, context) {
        const client = clientForModel(model);
        if (!client || !client.supports("references")) return null;
        const result = await client.request("textDocument/references", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
          context: { includeDeclaration: context?.includeDeclaration ?? true },
        });
        return locationsToMonaco(result);
      },
    });

    monaco.languages.registerRenameProvider(MANAGED_LANGS, {
      async provideRenameEdits(model, position, newName) {
        const client = clientForModel(model);
        if (!client || !client.supports("rename")) {
          return { edits: [], rejectReason: "Rename is not supported by this language server." };
        }
        const result = await client.request("textDocument/rename", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
          newName,
        });
        if (!result) return { edits: [], rejectReason: "Rename failed." };
        return toMonacoWorkspaceEdit(result);
      },
    });

    monaco.languages.registerDocumentSymbolProvider(MANAGED_LANGS, {
      async provideDocumentSymbols(model) {
        const client = clientForModel(model);
        if (!client || !client.supports("documentSymbol")) return [];
        const result = await client.request("textDocument/documentSymbol", {
          textDocument: { uri: model.uri.toString() },
        });
        return symbolsToMonaco(result);
      },
    });

    monaco.languages.registerSignatureHelpProvider(MANAGED_LANGS, {
      signatureHelpTriggerCharacters: ["(", ","],
      signatureHelpRetriggerCharacters: [")"],
      async provideSignatureHelp(model, position) {
        const client = clientForModel(model);
        if (!client || !client.supports("signatureHelp")) return null;
        flushPendingChange(model.uri.toString());
        const result = await client.request("textDocument/signatureHelp", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
        });
        if (!result || !Array.isArray(result.signatures) || !result.signatures.length) return null;
        return { value: signatureHelpToMonaco(result), dispose() {} };
      },
    });

    monaco.languages.registerDocumentFormattingEditProvider(MANAGED_LANGS, {
      async provideDocumentFormattingEdits(model, formatOptions) {
        const client = clientForModel(model);
        if (!client || !client.supports("formatting")) return [];
        const result = await client.request("textDocument/formatting", {
          textDocument: { uri: model.uri.toString() },
          options: {
            tabSize: formatOptions?.tabSize ?? 4,
            insertSpaces: formatOptions?.insertSpaces ?? true,
            trimTrailingWhitespace: true,
            insertFinalNewline: true,
          },
        });
        if (!Array.isArray(result)) return [];
        return result.map((e) => ({ range: toMonacoRange(e.range), text: e.newText }));
      },
    });

    monaco.languages.registerCodeActionProvider(MANAGED_LANGS, {
      async provideCodeActions(model, range, context) {
        const client = clientForModel(model);
        if (!client || !client.supports("codeAction")) return { actions: [], dispose() {} };
        const diagnostics = (context?.markers || []).map(markerToLspDiagnostic);
        const result = await client.request("textDocument/codeAction", {
          textDocument: { uri: model.uri.toString() },
          range: {
            start: fromMonacoPosition({ lineNumber: range.startLineNumber, column: range.startColumn }),
            end: fromMonacoPosition({ lineNumber: range.endLineNumber, column: range.endColumn }),
          },
          context: { diagnostics, only: undefined },
        });
        const actions = (Array.isArray(result) ? result : [])
          .map((a) => codeActionToMonaco(a, client.lang))
          .filter(Boolean);
        return { actions, dispose() {} };
      },
    });

    monaco.languages.registerInlayHintsProvider(MANAGED_LANGS, {
      async provideInlayHints(model, range) {
        const client = clientForModel(model);
        if (!client || !client.supports("inlayHint")) return { hints: [], dispose() {} };
        const result = await client.request("textDocument/inlayHint", {
          textDocument: { uri: model.uri.toString() },
          range: {
            start: fromMonacoPosition({ lineNumber: range.startLineNumber, column: range.startColumn }),
            end: fromMonacoPosition({ lineNumber: range.endLineNumber, column: range.endColumn }),
          },
        });
        if (!Array.isArray(result)) return { hints: [], dispose() {} };
        const hints = result.map((h) => ({
          position: toMonacoPosition(h.position),
          label: typeof h.label === "string" ? h.label : Array.isArray(h.label) ? h.label.map((p) => p.value).join("") : "",
          kind: h.kind === 1 ? monaco.languages.InlayHintKind.Type : h.kind === 2 ? monaco.languages.InlayHintKind.Parameter : undefined,
          paddingLeft: h.paddingLeft,
          paddingRight: h.paddingRight,
        }));
        return { hints, dispose() {} };
      },
    });

    // ---- CodeLens ----
    monaco.languages.registerCodeLensProvider(MANAGED_LANGS, {
      async provideCodeLenses(model) {
        const client = clientForModel(model);
        if (!client || !client.supports("codeLens")) return { lenses: [], dispose() {} };
        const result = await client.request("textDocument/codeLens", {
          textDocument: { uri: model.uri.toString() },
        });
        if (!Array.isArray(result)) return { lenses: [], dispose() {} };
        const lenses = result.map((cl) => {
          const lens = { range: toMonacoRange(cl.range) };
          if (cl.command) {
            lens.command = {
              id: "michael.lsp.executeCommand",
              title: cl.command.title || "",
              arguments: [client.lang, cl.command],
            };
          }
          lens.__lspItem = cl;
          lens.__lspLang = client.lang;
          return lens;
        });
        return { lenses, dispose() {} };
      },
      async resolveCodeLens(_model, codeLens) {
        const client = clients.get(codeLens.__lspLang);
        if (!client || !codeLens.__lspItem) return codeLens;
        const cap = client.capabilities?.codeLensProvider;
        if (!cap?.resolveProvider) return codeLens;
        const resolved = await client.request("codeLens/resolve", codeLens.__lspItem);
        if (!resolved) return codeLens;
        if (resolved.command) {
          codeLens.command = {
            id: "michael.lsp.executeCommand",
            title: resolved.command.title || "",
            arguments: [client.lang, resolved.command],
          };
        }
        return codeLens;
      },
    });

    // ---- Document Highlight (highlight all occurrences of selected symbol) ----
    monaco.languages.registerDocumentHighlightProvider(MANAGED_LANGS, {
      async provideDocumentHighlights(model, position) {
        const client = clientForModel(model);
        if (!client || !client.supports("documentHighlight")) return [];
        const result = await client.request("textDocument/documentHighlight", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
        });
        if (!Array.isArray(result)) return [];
        return result.map((h) => ({
          range: toMonacoRange(h.range),
          kind: h.kind === 2 ? monaco.languages.DocumentHighlightKind.Read
            : h.kind === 3 ? monaco.languages.DocumentHighlightKind.Write
            : monaco.languages.DocumentHighlightKind.Text,
        }));
      },
    });

    // ---- Color Provider (inline color swatches in CSS/code) ----
    monaco.languages.registerColorProvider(MANAGED_LANGS, {
      async provideDocumentColors(model) {
        const client = clientForModel(model);
        if (!client || !client.supports("colorProvider")) return [];
        const result = await client.request("textDocument/documentColor", {
          textDocument: { uri: model.uri.toString() },
        });
        if (!Array.isArray(result)) return [];
        return result.map((c) => ({
          range: toMonacoRange(c.range),
          color: { red: c.color.red, green: c.color.green, blue: c.color.blue, alpha: c.color.alpha },
        }));
      },
      async provideColorPresentations(model, colorInfo) {
        const client = clientForModel(model);
        if (!client || !client.supports("colorProvider")) return [];
        const result = await client.request("textDocument/colorPresentation", {
          textDocument: { uri: model.uri.toString() },
          color: colorInfo.color,
          range: {
            start: fromMonacoPosition({ lineNumber: colorInfo.range.startLineNumber, column: colorInfo.range.startColumn }),
            end: fromMonacoPosition({ lineNumber: colorInfo.range.endLineNumber, column: colorInfo.range.endColumn }),
          },
        });
        if (!Array.isArray(result)) return [];
        return result.map((p) => {
          const pres = { label: p.label };
          if (p.textEdit) {
            pres.textEdit = { range: toMonacoRange(p.textEdit.range), text: p.textEdit.newText };
          }
          if (Array.isArray(p.additionalTextEdits)) {
            pres.additionalTextEdits = p.additionalTextEdits.map((e) => ({
              range: toMonacoRange(e.range), text: e.newText,
            }));
          }
          return pres;
        });
      },
    });

    // ---- Linked Editing Ranges (rename HTML open/close tags together) ----
    monaco.languages.registerLinkedEditingRangeProvider(MANAGED_LANGS, {
      async provideLinkedEditingRanges(model, position) {
        const client = clientForModel(model);
        if (!client || !client.supports("linkedEditingRange")) return null;
        const result = await client.request("textDocument/linkedEditingRange", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
        });
        if (!result || !Array.isArray(result.ranges)) return null;
        return {
          ranges: result.ranges.map(toMonacoRange),
          wordPattern: result.wordPattern ? new RegExp(result.wordPattern) : undefined,
        };
      },
    });

    // ---- Folding Range Provider ----
    monaco.languages.registerFoldingRangeProvider(MANAGED_LANGS, {
      async provideFoldingRanges(model) {
        const client = clientForModel(model);
        if (!client || !client.supports("foldingRange")) return [];
        const result = await client.request("textDocument/foldingRange", {
          textDocument: { uri: model.uri.toString() },
        });
        if (!Array.isArray(result)) return [];
        return result.map((fr) => ({
          start: fr.startLine + 1,
          end: fr.endLine + 1,
          kind: fr.kind === "comment" ? monaco.languages.FoldingRangeKind.Comment
            : fr.kind === "imports" ? monaco.languages.FoldingRangeKind.Imports
            : monaco.languages.FoldingRangeKind.Region,
        }));
      },
    });

    // ---- On-Type Formatting ----
    const onTypeCandidates = [";", "}", "\n", ":", ")"];
    monaco.languages.registerOnTypeFormattingEditProvider(MANAGED_LANGS, {
      autoFormatTriggerCharacters: onTypeCandidates,
      async provideOnTypeFormattingEdits(model, position, ch, formatOptions) {
        const client = clientForModel(model);
        if (!client || !client.supports("onTypeFormatting")) return [];
        const serverChars = [
          client.capabilities?.documentOnTypeFormattingProvider?.firstTriggerCharacter,
          ...(client.capabilities?.documentOnTypeFormattingProvider?.moreTriggerCharacter || []),
        ].filter(Boolean);
        if (!serverChars.includes(ch)) return [];
        const result = await client.request("textDocument/onTypeFormatting", {
          textDocument: { uri: model.uri.toString() },
          position: fromMonacoPosition(position),
          ch,
          options: { tabSize: formatOptions?.tabSize ?? 4, insertSpaces: formatOptions?.insertSpaces ?? true },
        });
        if (!Array.isArray(result)) return [];
        return result.map((e) => ({ range: toMonacoRange(e.range), text: e.newText }));
      },
    });

    // ---- Range Formatting ----
    monaco.languages.registerDocumentRangeFormattingEditProvider(MANAGED_LANGS, {
      async provideDocumentRangeFormattingEdits(model, range, formatOptions) {
        const client = clientForModel(model);
        if (!client || !client.supports("rangeFormatting")) return [];
        const result = await client.request("textDocument/rangeFormatting", {
          textDocument: { uri: model.uri.toString() },
          range: {
            start: fromMonacoPosition({ lineNumber: range.startLineNumber, column: range.startColumn }),
            end: fromMonacoPosition({ lineNumber: range.endLineNumber, column: range.endColumn }),
          },
          options: {
            tabSize: formatOptions?.tabSize ?? 4,
            insertSpaces: formatOptions?.insertSpaces ?? true,
            trimTrailingWhitespace: true,
            insertFinalNewline: true,
          },
        });
        if (!Array.isArray(result)) return [];
        return result.map((e) => ({ range: toMonacoRange(e.range), text: e.newText }));
      },
    });

    // ---- Call Hierarchy (only if Monaco exposes the API) ----
    if (typeof monaco.languages.registerCallHierarchyProvider === "function") {
      monaco.languages.registerCallHierarchyProvider(MANAGED_LANGS, {
        async prepareCallHierarchy(model, position) {
          const client = clientForModel(model);
          if (!client || !client.supports("callHierarchy")) return [];
          const result = await client.request("textDocument/prepareCallHierarchy", {
            textDocument: { uri: model.uri.toString() },
            position: fromMonacoPosition(position),
          });
          if (!Array.isArray(result) || !result.length) return [];
          return result.map((item) => callHierarchyItemToMonaco(item, client.lang));
        },
        async provideIncomingCalls(item) {
          const client = clients.get(item.__lspLang);
          if (!client) return [];
          const result = await client.request("callHierarchy/incomingCalls", { item: item.__lspItem });
          if (!Array.isArray(result)) return [];
          return result.map((call) => ({
            from: callHierarchyItemToMonaco(call.from, client.lang),
            fromRanges: (call.fromRanges || []).map(toMonacoRange),
          }));
        },
        async provideOutgoingCalls(item) {
          const client = clients.get(item.__lspLang);
          if (!client) return [];
          const result = await client.request("callHierarchy/outgoingCalls", { item: item.__lspItem });
          if (!Array.isArray(result)) return [];
          return result.map((call) => ({
            to: callHierarchyItemToMonaco(call.to, client.lang),
            fromRanges: (call.fromRanges || []).map(toMonacoRange),
          }));
        },
      });
    }

    // ---- Selection Range ----
    monaco.languages.registerSelectionRangeProvider(MANAGED_LANGS, {
      async provideSelectionRanges(model, positions) {
        const client = clientForModel(model);
        if (!client || !client.supports("selectionRange")) return [];
        const result = await client.request("textDocument/selectionRange", {
          textDocument: { uri: model.uri.toString() },
          positions: positions.map(fromMonacoPosition),
        });
        if (!Array.isArray(result)) return [];
        return result.map((sr) => {
          const ranges = [];
          let cur = sr;
          while (cur) {
            ranges.push({ range: toMonacoRange(cur.range) });
            cur = cur.parent;
          }
          return ranges;
        });
      },
    });

    // ---- Document Link ----
    monaco.languages.registerLinkProvider(MANAGED_LANGS, {
      async provideLinks(model) {
        const client = clientForModel(model);
        if (!client || !client.supports("documentLink")) return { links: [] };
        const result = await client.request("textDocument/documentLink", {
          textDocument: { uri: model.uri.toString() },
        });
        if (!Array.isArray(result)) return { links: [] };
        return {
          links: result.map((l) => ({
            range: toMonacoRange(l.range),
            url: l.target,
            tooltip: l.tooltip,
          })),
        };
      },
    });

    // ---- Workspace Symbol (only if Monaco exposes the API) ----
    if (typeof monaco.languages.registerWorkspaceSymbolProvider === "function") {
      monaco.languages.registerWorkspaceSymbolProvider({
        async provideWorkspaceSymbols(query) {
          const results = [];
          for (const [langId, client] of clients) {
            if (!client.initialized || !client.supports("workspaceSymbol")) continue;
            const symbols = await client.request("workspace/symbol", { query });
            if (!Array.isArray(symbols)) continue;
            for (const s of symbols) {
              results.push({
                name: s.name,
                kind: (s.kind ?? 1) - 1,
                containerName: s.containerName || "",
                location: {
                  uri: monaco.Uri.parse(s.location.uri),
                  range: toMonacoRange(s.location.range),
                },
              });
            }
          }
          return results;
        },
      });
    }
  }

  function registerExecuteCommand() {
    if (executeCommandRegistered) return;
    executeCommandRegistered = true;
    monaco.editor.registerCommand("michael.lsp.executeCommand", (_accessor, langId, command) => {
      const client = clients.get(langId);
      if (!client || !command) return;
      client.request("workspace/executeCommand", {
        command: command.command,
        arguments: command.arguments || [],
      });
    });
  }

  // ---- LSP -> Monaco translation helpers ----
  function completionToMonaco(it, defaultRange, langId) {
    let insertText = it.insertText ?? it.label ?? "";
    let range = defaultRange;
    const edit = it.textEdit;
    if (edit) {
      insertText = edit.newText ?? insertText;
      if (edit.range) {
        range = toMonacoRange(edit.range);
      } else if (edit.insert && edit.replace) {
        range = { insert: toMonacoRange(edit.insert), replace: toMonacoRange(edit.replace) };
      }
    }
    const monacoItem = {
      label: typeof it.label === "string" ? it.label : it.label?.label ?? "",
      kind: LSP_TO_COMPLETION_KIND[it.kind] ?? CK.Text,
      insertText,
      range,
      detail: it.detail,
      documentation: completionDocsToMonaco(it.documentation),
      sortText: it.sortText,
      filterText: it.filterText,
      preselect: it.preselect,
      commitCharacters: it.commitCharacters,
    };
    if (it.insertTextFormat === 2) {
      monacoItem.insertTextRules = monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet;
    }
    if (Array.isArray(it.additionalTextEdits)) {
      monacoItem.additionalTextEdits = it.additionalTextEdits.map((e) => ({
        range: toMonacoRange(e.range),
        text: e.newText,
      }));
    }
    if (it.tags?.includes(1) || it.deprecated) {
      monacoItem.tags = [monaco.languages.CompletionItemTag.Deprecated];
    }
    const isCallable = it.kind === 2 || it.kind === 3 || it.kind === 4;
    if (isCallable && it.insertTextFormat !== 2 && !insertText.includes("(")) {
      monacoItem.insertText = insertText + "($1)";
      monacoItem.insertTextRules = monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet;
    }
    monacoItem.__lspItem = it;
    monacoItem.__lspLang = langId;
    return monacoItem;
  }

  function locationsToMonaco(result) {
    if (!result) return null;
    const list = Array.isArray(result) ? result : [result];
    const out = [];
    for (const loc of list) {
      if (loc.targetUri) {
        out.push({
          uri: monaco.Uri.parse(loc.targetUri),
          range: toMonacoRange(loc.targetSelectionRange || loc.targetRange),
        });
      } else if (loc.uri) {
        out.push({ uri: monaco.Uri.parse(loc.uri), range: toMonacoRange(loc.range) });
      }
    }
    return out;
  }

  function symbolsToMonaco(result) {
    if (!Array.isArray(result)) return [];
    const convert = (s) => {
      // DocumentSymbol (hierarchical) vs SymbolInformation (flat).
      if (s.location) {
        return {
          name: s.name,
          detail: s.containerName || "",
          kind: (s.kind ?? 1) - 1,
          tags: [],
          range: toMonacoRange(s.location.range),
          selectionRange: toMonacoRange(s.location.range),
        };
      }
      return {
        name: s.name,
        detail: s.detail || "",
        kind: (s.kind ?? 1) - 1,
        tags: (s.tags || []).map(() => monaco.languages.SymbolTag.Deprecated),
        range: toMonacoRange(s.range),
        selectionRange: toMonacoRange(s.selectionRange || s.range),
        children: Array.isArray(s.children) ? s.children.map(convert) : [],
      };
    };
    return result.map(convert);
  }

  function signatureHelpToMonaco(result) {
    return {
      activeSignature: result.activeSignature ?? 0,
      activeParameter: result.activeParameter ?? 0,
      signatures: result.signatures.map((sig) => ({
        label: sig.label,
        documentation: completionDocsToMonaco(sig.documentation),
        parameters: (sig.parameters || []).map((p) => ({
          label: p.label,
          documentation: completionDocsToMonaco(p.documentation),
        })),
        activeParameter: sig.activeParameter,
      })),
    };
  }

  function markerToLspDiagnostic(m) {
    return {
      range: {
        start: { line: m.startLineNumber - 1, character: m.startColumn - 1 },
        end: { line: m.endLineNumber - 1, character: m.endColumn - 1 },
      },
      severity: m.severity === Sev.Error ? 1 : m.severity === Sev.Warning ? 2 : m.severity === Sev.Info ? 3 : 4,
      message: m.message,
      code: m.code,
      source: m.source,
    };
  }

  function codeActionToMonaco(a, langId) {
    if (!a) return null;
    // Plain Command (no CodeAction wrapper).
    if (a.command && !a.title && !a.edit) {
      return {
        title: a.command.title || a.title || "Command",
        command: { id: "michael.lsp.executeCommand", title: a.command.title || "", arguments: [langId, a.command] },
      };
    }
    const action = {
      title: a.title || "Code action",
      kind: a.kind,
      isPreferred: a.isPreferred,
      diagnostics: undefined,
    };
    if (a.edit) action.edit = toMonacoWorkspaceEdit(a.edit);
    if (a.command) {
      const cmd = typeof a.command === "string" ? { command: a.command, title: a.title } : a.command;
      action.command = { id: "michael.lsp.executeCommand", title: cmd.title || a.title || "", arguments: [langId, cmd] };
    }
    return action;
  }

  const LSP_SYMBOL_KIND_MAP = {
    1: 0, 2: 10, 3: 4, 4: 2, 5: 22, 6: 10, 7: 10, 8: 2, 9: 11,
    10: 3, 11: 7, 12: 5, 13: 0, 14: 14, 15: 17, 16: 17, 17: 9,
    18: 8, 19: 6, 20: 1, 21: 12, 22: 13, 23: 18, 24: 23, 25: 24, 26: 25,
  };

  function callHierarchyItemToMonaco(item, langId) {
    return {
      name: item.name,
      kind: LSP_SYMBOL_KIND_MAP[item.kind] ?? 0,
      detail: item.detail || "",
      uri: monaco.Uri.parse(item.uri),
      range: toMonacoRange(item.range),
      selectionRange: toMonacoRange(item.selectionRange),
      tags: (item.tags || []).map(() => 1),
      __lspItem: item,
      __lspLang: langId,
    };
  }

  // ---- public surface ----
  return {
    registerProviders,
    didOpen,
    didChange,
    didSave,
    didClose,
    ensureServer,
    async startManual(langId, custom) {
      return ensureServer(langId, custom);
    },
    async stop(langId) {
      const client = clients.get(langId);
      if (client) {
        client.shutdown();
        clients.delete(langId);
      }
      try { await backend.lspStop(SERVER_LANG[langId] || langId); } catch { /* ignore */ }
      for (const m of monaco.editor.getModels()) {
        monaco.editor.setModelMarkers(m, "lsp:" + langId, []);
      }
      onStatus?.();
    },
    status() {
      return [...clients.entries()].map(([lang, client]) => ({
        lang,
        running: true,
        initialized: client.initialized,
      }));
    },
    isRunning(langId) {
      return clients.has(langId);
    },
    logFor(langId) {
      return clients.get(langId)?.logLines.join("\n") || "";
    },
    managedLangs: MANAGED_LANGS.slice(),
    onCompletionSymbols: null,
    async queryDocumentSymbols(uri, langId) {
      const client = clients.get(langId);
      if (!client || !client.initialized || !client.supports("documentSymbol")) return [];
      const result = await client.request("textDocument/documentSymbol", {
        textDocument: { uri },
      });
      if (!Array.isArray(result)) return [];
      const names = [];
      const walk = (items) => {
        for (const item of items) {
          const name = item.name || item.label;
          if (name) names.push(name);
          if (Array.isArray(item.children)) walk(item.children);
        }
      };
      walk(result);
      return names;
    },
    async queryWorkspaceSymbols(langId, query = "") {
      const client = clients.get(langId);
      if (!client || !client.initialized) return [];
      const cap = client.capabilities || {};
      if (!cap.workspaceSymbolProvider) return [];
      const result = await client.request("workspace/symbol", { query });
      if (!Array.isArray(result)) return [];
      return result.map((s) => s.name).filter(Boolean);
    },
  };
}
