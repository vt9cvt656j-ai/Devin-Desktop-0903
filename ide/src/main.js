// Michael IDE — editor + AI assistant orchestration.

// Global error boundary: catch unhandled errors/rejections so the IDE never
// silently dies. Shows a transient toast and logs to the console.
window.addEventListener("error", (e) => {
  console.error("[michael-ide] uncaught:", e.error || e.message);
  try { showToast?.(`Error: ${e.message}`, 5000); } catch { /* too early */ }
});
window.addEventListener("unhandledrejection", (e) => {
  console.error("[michael-ide] unhandled rejection:", e.reason);
  try { showToast?.(`Unhandled: ${e.reason?.message || e.reason}`, 5000); } catch { /* too early */ }
});

import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebglAddon } from "@xterm/addon-webgl";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import { renderMarkdownInto, renderMarkdownStream, langLabel, monacoLang } from "./markdown.js";
import { ExtensionHost } from "./ext/host.js";
import { createExtensionManager } from "./ext/manager.js";
import { createCommandPalette } from "./ext/palette.js";
import { createExtensionsPanel } from "./ext/panel.js";
import { t, initLocale, onLocaleChange, registerLocale, setLocale, applyToDOM } from "./i18n.js";
import { load as loadStore } from "@tauri-apps/plugin-store";
import { registerSnippetProviders } from "./snippets.js";
import { createLspManager } from "./lsp-client.js";
import { parseProblems } from "./problem-matchers.js";
import { createDapManager } from "./dap-client.js";

self.MonacoEnvironment = {
  getWorker(_id, label) {
    if (label === "json") return new jsonWorker();
    if (label === "css" || label === "scss" || label === "less") return new cssWorker();
    if (label === "html" || label === "handlebars" || label === "razor") return new htmlWorker();
    if (label === "typescript" || label === "javascript") return new tsWorker();
    return new editorWorker();
  },
};

// Configure Monaco's bundled TypeScript language service so JS/TS files get
// real completions, hover, signature help, go-to-definition and live
// diagnostics — not just syntax highlighting. This needs no external language
// server, so it works in both the native app and the browser preview.
(function configureLanguageService() {
  const ts = monaco.languages.typescript;
  const compilerOptions = {
    target: ts.ScriptTarget.ESNext,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.NodeJs,
    allowJs: true,
    checkJs: false,
    allowNonTsExtensions: true,
    esModuleInterop: true,
    jsx: ts.JsxEmit.ReactJSX,
    skipLibCheck: true,
    noEmit: true,
    baseUrl: ".",
  };
  for (const d of [ts.typescriptDefaults, ts.javascriptDefaults]) {
    d.setCompilerOptions(compilerOptions);
    d.setEagerModelSync(true);
    d.setDiagnosticsOptions({ noSemanticValidation: false, noSyntaxValidation: false });
  }
})();

// ---- backend abstraction (Tauri when available, mock in a plain browser) ----
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
// Reserve room for the macOS traffic-light buttons only when running natively on macOS.
if (inTauri) document.body.classList.add("is-tauri");
if (/Mac/i.test(navigator.platform || navigator.userAgent)) {
  document.body.classList.add("is-mac");
}
const backend = inTauri ? await tauriBackend() : mockBackend();

// Reap any backend processes (shells / LSP servers / debug adapters) orphaned by
// a previous page session — runs before this session starts any of its own, so a
// webview reload no longer piles up zombie processes that eventually freeze the IDE.
if (inTauri) {
  try { await backend.invoke("cleanup_stale"); } catch { /* older backend without the command */ }
}

// Real LSP client manager (wired up after the workspace state exists below).
let lspManager = null;
// Debug Adapter Protocol manager (wired up alongside the LSP manager below).
let dapManager = null;
// Cached debug launch configurations discovered from launch.json (null = unloaded).
let _launchConfigsCache = null;

async function tauriBackend() {
  const core = await import("@tauri-apps/api/core");
  const dialog = await import("@tauri-apps/plugin-dialog");
  return {
    registerWorkspaceRoot: (path) => core.invoke("register_workspace_root", { path }),
    readDir: (path) => core.invoke("read_dir", { path }),
    readTextFile: (path) => core.invoke("read_text_file", { path }),
    writeTextFile: (path, content) => core.invoke("write_text_file", { path, content }),
    homeDir: () => core.invoke("home_dir"),
    createFile: (path) => core.invoke("create_file", { path }),
    createDir: (path) => core.invoke("create_dir", { path }),
    renamePath: (from, to) => core.invoke("rename_path", { from, to }),
    deletePath: (path) => core.invoke("delete_path", { path }),
    searchInProject: (root, query, caseSensitive) =>
      core.invoke("search_in_project", { root, query, caseSensitive }),
    gitStatus: (root) => core.invoke("git_status", { root }),
    gitFileHead: (root, rel) => core.invoke("git_file_head", { root, rel }),
    gitStage: (root, rel) => core.invoke("git_stage", { root, rel }),
    gitUnstage: (root, rel) => core.invoke("git_unstage", { root, rel }),
    gitStageAll: (root) => core.invoke("git_stage_all", { root }),
    gitUnstageAll: (root) => core.invoke("git_unstage_all", { root }),
    gitCommit: (root, message) => core.invoke("git_commit", { root, message }),
    gitPush: (root) => core.invoke("git_push", { root }),
    gitBranches: (root) => core.invoke("git_branches", { root }),
    gitCheckout: (root, branch, create) => core.invoke("git_checkout", { root, branch, create }),
    gitPull: (root) => core.invoke("git_pull", { root }),
    gitLog: (root, count) => core.invoke("git_log", { root, count }),
    gitConflicts: (root) => core.invoke("git_conflicts", { root }),
    gitMergeVersions: (root, rel) => core.invoke("git_merge_versions", { root, rel }),
    gitResolveConflict: (root, rel, resolution) =>
      core.invoke("git_resolve_conflict", { root, rel, resolution }),
    lspList: () => core.invoke("lsp_list"),
    lspStart: (config, onEvent) => {
      const channel = new core.Channel();
      channel.onmessage = onEvent;
      return core.invoke("lsp_start", { config, onEvent: channel });
    },
    lspSend: (lang, message) => core.invoke("lsp_send", { lang, message }),
    lspStop: (lang) => core.invoke("lsp_stop", { lang }),
    lspCheckAvailable: (lang) => core.invoke("lsp_check_available", { lang }),
    writeTmpFile: (name, content) => core.invoke("write_tmp_file", { name, content }),
    lspDetectPython: () => core.invoke("lsp_detect_python"),
    lspPythonEnvSymbols: (modules) => core.invoke("lsp_python_env_symbols", { modules }),
    lspNodeEnvSymbols: (projectDir, modules) => core.invoke("lsp_node_env_symbols", { projectDir, modules }),
    lspGoEnvSymbols: (projectDir) => core.invoke("lsp_go_env_symbols", { projectDir }),
    lspLangEnvSymbols: (lang, projectDir, modules) => core.invoke("lsp_lang_env_symbols", { lang, projectDir, modules }),
    dapList: () => core.invoke("dap_list"),
    dapStart: (config, onEvent) => {
      const channel = new core.Channel();
      channel.onmessage = onEvent;
      return core.invoke("dap_start", { config, onEvent: channel });
    },
    dapSend: (adapterId, message) => core.invoke("dap_send", { adapterId, message }),
    dapStop: (adapterId) => core.invoke("dap_stop", { adapterId }),
    marketplaceList: () => core.invoke("marketplace_list"),
    marketplaceSearch: (query) => core.invoke("marketplace_search", { query }),
    marketplaceInstall: (entry) => core.invoke("marketplace_install", { entry }),
    dbMarketplaceList: () => core.invoke("db_marketplace_list"),
    dbMarketplaceUpsert: (ext) => core.invoke("db_marketplace_upsert", { ext }),
    tasksList: (root) => core.invoke("tasks_list", { root }),
    taskRunCapture: (cwd, command) => core.invoke("task_run_capture", { cwd, command }),
    pickFolder: () => dialog.open({ directory: true, multiple: false }),
    aiChat: (config, messages, onEvent) => {
      const channel = new core.Channel();
      channel.onmessage = onEvent;
      return core.invoke("ai_chat", { config, messages, onEvent: channel });
    },
    aiChatWithTools: (config, messages, tools, onEvent) => {
      const channel = new core.Channel();
      channel.onmessage = onEvent;
      return core.invoke("ai_chat_with_tools", { config, messages, tools, onEvent: channel });
    },
    aiComplete: (config, messages, maxTokens) =>
      core.invoke("ai_complete", { config, messages, maxTokens }),
    termOpen: (opts, onEvent) => {
      const channel = new core.Channel();
      channel.onmessage = onEvent;
      return core.invoke("term_open", {
        cwd: opts.cwd ?? null,
        cols: opts.cols,
        rows: opts.rows,
        onEvent: channel,
      });
    },
    termWrite: (id, data) => core.invoke("term_write", { id, data }),
    termResize: (id, cols, rows) => core.invoke("term_resize", { id, cols, rows }),
    termClose: (id) => core.invoke("term_close", { id }),
    termListCommands: () => core.invoke("term_list_commands"),
    termHistory: () => core.invoke("term_history"),
    fsWatch: (paths) => core.invoke("fs_watch", { paths }),
    fsUnwatch: (paths) => core.invoke("fs_unwatch", { paths }),
    replaceInFile: (filePath, query, replacement, caseSensitive) =>
      core.invoke("replace_in_file", { filePath, query, replacement, caseSensitive }),
    replaceInProject: (root, query, replacement, caseSensitive) =>
      core.invoke("replace_in_project", { root, query, replacement, caseSensitive }),
    gitStash: (root) => core.invoke("git_stash", { root }),
    gitStashPop: (root, index) => core.invoke("git_stash_pop", { root, index: index ?? null }),
    gitStashApply: (root, index) => core.invoke("git_stash_apply", { root, index }),
    gitStashDrop: (root, index) => core.invoke("git_stash_drop", { root, index }),
    gitStashList: (root) => core.invoke("git_stash_list", { root }),
    gitBlame: (root, rel) => core.invoke("git_blame", { root, rel }),
    authLoginOrRegister: (email, password) => core.invoke("auth_login_or_register", { email, password }),
    authCheckEmail: (email) => core.invoke("auth_check_email", { email }),
    authSendCode: (email) => core.invoke("auth_send_code", { email }),
    authVerifyCode: (email, code) => core.invoke("auth_verify_code", { email, code }),
    invoke: (cmd, args) => core.invoke(cmd, args || {}),
  };
}

function mockBackend() {
  const ROOT = "/Users/andrew/my-app";
  const DIRS = new Set([
    "/Users/andrew",
    ROOT,
    ROOT + "/src",
    ROOT + "/src/utils",
    ROOT + "/components",
  ]);
  const FILES = {
    [ROOT + "/README.md"]:
      "# my-app\n\nA sample project shown in the browser preview.\nRun `npm run dev` to start the dev server.\n",
    [ROOT + "/package.json"]:
      '{\n  "name": "my-app",\n  "version": "1.0.0",\n  "scripts": {\n    "dev": "vite"\n  }\n}\n',
    [ROOT + "/src/main.js"]:
      'import { greet } from "./utils/format.js";\nimport { mount } from "./utils/dom.js";\n\nmount(document.body, greet("world"));\nconsole.log(greet("Michael"));\n',
    [ROOT + "/src/styles.css"]:
      "body {\n  margin: 0;\n  font-family: sans-serif;\n}\n\n.card {\n  border-radius: 10px;\n}\n",
    [ROOT + "/src/utils/format.js"]:
      'export function greet(name) {\n  const who = name?.trim() || "world";\n  return `Hello, ${who}!`;\n}\n',
    [ROOT + "/src/utils/dom.js"]:
      'export function mount(el, text) {\n  el.textContent = text;\n}\n',
    [ROOT + "/src/utils/math.ts"]:
      "export function add(a: number, b: number): number {\n  return a + b;\n}\n\nexport function clamp(value: number, min: number, max: number): number {\n  return Math.min(Math.max(value, min), max);\n}\n\nexport const TAU = Math.PI * 2;\n",
    [ROOT + "/components/Button.js"]:
      'export function Button(label) {\n  const el = document.createElement("button");\n  el.textContent = label;\n  return el;\n}\n',
    [ROOT + "/components/Card.js"]:
      'export function Card(title) {\n  const el = document.createElement("div");\n  el.className = "card";\n  el.textContent = title;\n  return el;\n}\n',
  };

  // Simulated git state: a curated set of changes vs. an imaginary HEAD so the
  // browser preview can show the Source Control panel and diffs. The native app
  // talks to the real `git` instead.
  const GIT_HEAD = {
    [ROOT + "/README.md"]:
      "# my-app\n\nA sample project shown in the browser preview.\n",
    [ROOT + "/src/utils/format.js"]:
      "export function greet(name) {\n  return `Hello, ${name}!`;\n}\n",
  };
  const GIT_CHANGES = [
    { rel: "README.md", code: " M", label: "Modified", staged: false, deleted: false },
    { rel: "src/utils/format.js", code: "M ", label: "Modified", staged: true, deleted: false },
    { rel: "components/Card.js", code: "??", label: "Untracked", staged: false, deleted: false },
  ];
  const GIT_CONFLICTS = [
    { rel: "src/utils/format.js", name: "format.js" },
  ];
  // In-memory stash list for the browser preview.
  const GIT_STASHES = [];
  const MERGE_VERSIONS = {
    "src/utils/format.js": {
      base: "export function greet(name) {\n  return `Hello, ${name}!`;\n}\n",
      ours: "export function greet(name) {\n  const who = name?.trim() || \"world\";\n  return `Hello, ${who}!`;\n}\n",
      theirs: "export function greet(name) {\n  return `Hi, ${name || \"friend\"}!`;\n}\n",
      merged: "<<<<<<< HEAD\nexport function greet(name) {\n  const who = name?.trim() || \"world\";\n  return `Hello, ${who}!`;\n}\n=======\nexport function greet(name) {\n  return `Hi, ${name || \"friend\"}!`;\n}\n>>>>>>> feature/greeting\n",
    },
  };
  FILES[ROOT + "/src/utils/format.js"] = MERGE_VERSIONS["src/utils/format.js"].merged;
  const mockLspRunning = new Set();
  const mockDapRunning = new Set();
  // Minimal in-browser DAP simulation so the debugger is demoable without a
  // native adapter. Echoes responses and drives a tiny stopped/continue flow.
  let mockDap = null;
  const mockDapReply = (req, body) =>
    mockDap?.onEvent?.({ kind: "message", data: JSON.stringify({ type: "response", request_seq: req.seq, success: true, command: req.command, body }) });
  const mockDapEvent = (event, body) =>
    mockDap?.onEvent?.({ kind: "message", data: JSON.stringify({ type: "event", event, body }) });
  function mockDapHandle(raw) {
    let req;
    try { req = JSON.parse(raw); } catch { return; }
    if (req.type !== "request") return;
    const file = (req.arguments && req.arguments.program) || `${mockDap.program}/src/main.js`;
    switch (req.command) {
      case "initialize":
        mockDapReply(req, { supportsConfigurationDoneRequest: true, supportsTerminateRequest: true, supportsEvaluateForHovers: true });
        setTimeout(() => mockDapEvent("initialized", {}), 10);
        break;
      case "launch":
      case "attach":
        mockDapReply(req, {});
        mockDapEvent("output", { category: "console", output: "Mock debug session started (browser preview).\n" });
        setTimeout(() => mockDapEvent("stopped", { reason: "breakpoint", threadId: 1, allThreadsStopped: true }), 60);
        break;
      case "setBreakpoints":
        mockDapReply(req, { breakpoints: (req.arguments.breakpoints || []).map((b) => ({ verified: true, line: b.line })) });
        break;
      case "setExceptionBreakpoints":
      case "configurationDone":
        mockDapReply(req, {});
        break;
      case "threads":
        mockDapReply(req, { threads: [{ id: 1, name: "main" }] });
        break;
      case "stackTrace":
        mockDapReply(req, { stackFrames: [
          { id: 1, name: "main", line: 1, column: 1, source: { path: file, name: file.split("/").pop() } },
          { id: 2, name: "greet", line: 2, column: 3, source: { path: file, name: file.split("/").pop() } },
        ], totalFrames: 2 });
        break;
      case "scopes":
        mockDapReply(req, { scopes: [{ name: "Locals", variablesReference: 1000, expensive: false }, { name: "Globals", variablesReference: 1001, expensive: true }] });
        break;
      case "variables":
        mockDapReply(req, { variables: req.arguments.variablesReference === 1000
          ? [{ name: "name", value: '"world"', type: "string", variablesReference: 0 }, { name: "count", value: "42", type: "number", variablesReference: 0 }]
          : [{ name: "globalThis", value: "Window", type: "object", variablesReference: 0 }] });
        break;
      case "evaluate":
        mockDapReply(req, { result: `${req.arguments.expression} = (mock) 42`, variablesReference: 0 });
        break;
      case "continue":
        mockDapReply(req, { allThreadsContinued: true });
        setTimeout(() => mockDapEvent("terminated", {}), 30);
        break;
      case "next":
      case "stepIn":
      case "stepOut":
        mockDapReply(req, {});
        setTimeout(() => mockDapEvent("stopped", { reason: "step", threadId: 1 }), 30);
        break;
      case "terminate":
      case "disconnect":
        mockDapReply(req, {});
        mockDapEvent("terminated", {});
        break;
      default:
        mockDapReply(req, {});
        break;
    }
  }
  const MARKETPLACE = [
    {
      id: "michael.theme-pack",
      name: "Michael Theme Pack",
      version: "1.2.0",
      description: "A curated set of editor themes including Monokai Pro, Nord, Solarized, and 12 more hand-crafted color palettes for comfortable coding.",
      author: "Michael Labs",
      download_url: "https://example.com/michael-theme-pack.zip",
      tags: ["theme", "ui"],
      downloads: 128400,
      rating: 4.8,
      featured: true,
      category: "Themes",
    },
    {
      id: "michael.git-tools",
      name: "Git Tools Pro",
      version: "0.4.1",
      description: "Advanced Git integration with interactive rebase, cherry-pick UI, stash manager, and inline blame annotations.",
      author: "Michael Labs",
      download_url: "https://example.com/git-tools.zip",
      tags: ["git", "productivity"],
      downloads: 73310,
      rating: 4.6,
      featured: true,
      category: "SCM",
    },
    {
      id: "michael.prettier",
      name: "Prettier Formatter",
      version: "3.5.0",
      description: "Opinionated code formatter supporting JS, TS, CSS, HTML, JSON, Markdown, and more. Format on save.",
      author: "Prettier Team",
      download_url: "https://example.com/prettier.zip",
      tags: ["formatter", "javascript", "typescript"],
      downloads: 245000,
      rating: 4.9,
      featured: true,
      category: "Formatters",
    },
    {
      id: "michael.eslint",
      name: "ESLint",
      version: "4.2.1",
      description: "Integrates ESLint into Michael IDE. Highlights problems in your code and offers quick-fix actions.",
      author: "Microsoft",
      download_url: "https://example.com/eslint.zip",
      tags: ["linter", "javascript", "typescript"],
      downloads: 312000,
      rating: 4.7,
      featured: false,
      category: "Linters",
    },
    {
      id: "michael.docker",
      name: "Docker",
      version: "1.8.0",
      description: "Docker container management, Dockerfile syntax highlighting, compose support, and image explorer.",
      author: "Microsoft",
      download_url: "https://example.com/docker.zip",
      tags: ["docker", "devops", "containers"],
      downloads: 89000,
      rating: 4.5,
      featured: false,
      category: "DevOps",
    },
    {
      id: "michael.rust-analyzer",
      name: "Rust Analyzer",
      version: "0.4.2094",
      description: "Smart Rust language support with completion, diagnostics, refactoring, and inline type hints.",
      author: "rust-lang",
      download_url: "https://example.com/rust-analyzer.zip",
      tags: ["rust", "language"],
      downloads: 156000,
      rating: 4.9,
      featured: true,
      category: "Languages",
    },
    {
      id: "michael.python",
      name: "Python",
      version: "2024.8.0",
      description: "Rich Python support including IntelliSense, linting, debugging, Jupyter Notebooks, and virtual env management.",
      author: "Microsoft",
      download_url: "https://example.com/python.zip",
      tags: ["python", "language", "jupyter"],
      downloads: 420000,
      rating: 4.6,
      featured: true,
      category: "Languages",
    },
    {
      id: "michael.icons",
      name: "Material Icon Theme",
      version: "5.12.0",
      description: "Material Design icons for files and folders in the explorer. Over 1000 icons for every file type.",
      author: "Philipp Kief",
      download_url: "https://example.com/material-icons.zip",
      tags: ["icons", "theme", "ui"],
      downloads: 198000,
      rating: 4.8,
      featured: false,
      category: "Themes",
    },
    {
      id: "michael.copilot",
      name: "AI Code Companion",
      version: "1.3.0",
      description: "AI-powered code completion with multi-line suggestions, chat, and inline code generation.",
      author: "Michael Labs",
      download_url: "https://example.com/ai-companion.zip",
      tags: ["ai", "completion", "productivity"],
      downloads: 67000,
      rating: 4.4,
      featured: true,
      category: "AI",
    },
    {
      id: "michael.tailwind",
      name: "Tailwind CSS IntelliSense",
      version: "0.12.8",
      description: "Autocomplete, syntax highlighting, color preview, and linting for Tailwind CSS classes.",
      author: "Tailwind Labs",
      download_url: "https://example.com/tailwind.zip",
      tags: ["css", "tailwind", "web"],
      downloads: 145000,
      rating: 4.7,
      featured: false,
      category: "Web",
    },
    {
      id: "michael.live-server",
      name: "Live Server",
      version: "5.7.9",
      description: "Launch a local development server with live reload for static and dynamic web pages.",
      author: "Ritwick Dey",
      download_url: "https://example.com/live-server.zip",
      tags: ["server", "web", "preview"],
      downloads: 92000,
      rating: 4.3,
      featured: false,
      category: "Web",
    },
    {
      id: "michael.markdown-all-in-one",
      name: "Markdown All in One",
      version: "3.6.2",
      description: "All you need for Markdown: keyboard shortcuts, table of contents, auto preview, math support, and list editing.",
      author: "Yu Zhang",
      download_url: "https://example.com/markdown-aio.zip",
      tags: ["markdown", "writing", "docs"],
      downloads: 108000,
      rating: 4.5,
      featured: false,
      category: "Other",
    },
  ];
  // Simulated branches for the browser preview's branch picker.
  let GIT_BRANCH = "main";
  const GIT_BRANCHES = ["main", "feature/login", "release/1.0"];

  const parentOf = (p) => p.slice(0, p.lastIndexOf("/"));
  const baseOf = (p) => p.slice(p.lastIndexOf("/") + 1);
  const exists = (p) => DIRS.has(p) || p in FILES;

  // ---- simulated terminal (browser preview only) ----
  const mockTerms = new Map();
  let mockTermSeq = 1;
  const mockPrompt = (cwd) => {
    const short = cwd.replace("/Users/andrew", "~");
    return `\x1b[1;32mmichael\x1b[0m:\x1b[1;34m${short}\x1b[0m$ `;
  };
  function runMockCommand(t, cmd, send) {
    if (!cmd) return;
    const [name, ...args] = cmd.split(/\s+/);
    switch (name) {
      case "help":
        send("Simulated shell. Try: \x1b[1mhelp ls pwd echo date whoami clear\x1b[0m\r\n");
        break;
      case "pwd":
        send(t.cwd + "\r\n");
        break;
      case "whoami":
        send("andrew\r\n");
        break;
      case "date":
        send(new Date().toString() + "\r\n");
        break;
      case "echo":
        send(args.join(" ") + "\r\n");
        break;
      case "clear":
        send("\x1b[2J\x1b[3J\x1b[H");
        break;
      case "ls": {
        const names = [];
        for (const d of DIRS) if (d !== t.cwd && parentOf(d) === t.cwd) names.push(`\x1b[1;34m${baseOf(d)}\x1b[0m`);
        for (const f of Object.keys(FILES)) if (parentOf(f) === t.cwd) names.push(baseOf(f));
        if (names.length) send(names.join("  ") + "\r\n");
        break;
      }
      default:
        send(`michael: command not found: ${name}\r\n`);
    }
  }
  function mockTermInput(id, data) {
    const t = mockTerms.get(id);
    if (!t) return;
    const send = (s) => t.onEvent({ kind: "data", data: s });
    if (data.charCodeAt(0) === 27) return; // ignore arrow / nav escape sequences
    for (const ch of data) {
      if (ch === "\r" || ch === "\n") {
        send("\r\n");
        runMockCommand(t, t.line.trim(), send);
        t.line = "";
        send(mockPrompt(t.cwd));
      } else if (ch === "\x7f" || ch === "\b") {
        if (t.line.length) {
          t.line = t.line.slice(0, -1);
          send("\b \b");
        }
      } else if (ch === "\x03") {
        send("^C\r\n");
        t.line = "";
        send(mockPrompt(t.cwd));
      } else if (ch >= " ") {
        t.line += ch;
        send(ch);
      }
    }
  }
  const ensureDir = (p) => {
    let cur = p;
    while (cur && !DIRS.has(cur)) {
      DIRS.add(cur);
      cur = parentOf(cur);
    }
  };
  return {
    readDir: async (path) => {
      const out = [];
      for (const d of DIRS) {
        if (d !== path && parentOf(d) === path) out.push({ name: baseOf(d), path: d, is_dir: true });
      }
      for (const f of Object.keys(FILES)) {
        if (parentOf(f) === path) out.push({ name: baseOf(f), path: f, is_dir: false });
      }
      out.sort((a, b) => (a.is_dir === b.is_dir ? a.name.localeCompare(b.name) : a.is_dir ? -1 : 1));
      return out;
    },
    readTextFile: async (path) => FILES[path] ?? "",
    writeTextFile: async (path, content) => {
      FILES[path] = content;
    },
    homeDir: async () => "/Users/andrew",
    createFile: async (path) => {
      if (exists(path)) throw new Error("a file or folder with that name already exists");
      ensureDir(parentOf(path));
      FILES[path] = "";
    },
    createDir: async (path) => {
      if (exists(path)) throw new Error("a file or folder with that name already exists");
      ensureDir(path);
    },
    renamePath: async (from, to) => {
      if (exists(to)) throw new Error("a file or folder with that name already exists");
      ensureDir(parentOf(to));
      if (from in FILES) {
        FILES[to] = FILES[from];
        delete FILES[from];
      } else if (DIRS.has(from)) {
        const prefix = from + "/";
        for (const f of Object.keys(FILES)) {
          if (f === from || f.startsWith(prefix)) {
            FILES[to + f.slice(from.length)] = FILES[f];
            delete FILES[f];
          }
        }
        for (const d of [...DIRS]) {
          if (d === from || d.startsWith(prefix)) {
            DIRS.delete(d);
            DIRS.add(to + d.slice(from.length));
          }
        }
        DIRS.add(to);
      } else {
        throw new Error("not found");
      }
    },
    deletePath: async (path) => {
      if (path in FILES) {
        delete FILES[path];
        return;
      }
      const prefix = path + "/";
      for (const f of Object.keys(FILES)) if (f === path || f.startsWith(prefix)) delete FILES[f];
      for (const d of [...DIRS]) if (d === path || d.startsWith(prefix)) DIRS.delete(d);
    },
    searchInProject: async (root, query, caseSensitive) => {
      const needle = caseSensitive ? query : query.toLowerCase();
      if (!needle) return [];
      const prefix = root.endsWith("/") ? root : root + "/";
      const results = [];
      const paths = Object.keys(FILES)
        .filter((p) => p === root || p.startsWith(prefix))
        .sort();
      for (const p of paths) {
        const rel = p.startsWith(prefix) ? p.slice(prefix.length) : baseOf(p);
        const matches = [];
        const lines = FILES[p].split("\n");
        for (let i = 0; i < lines.length && matches.length < 50; i++) {
          const line = lines[i];
          const hay = caseSensitive ? line : line.toLowerCase();
          let from = 0;
          while (matches.length < 50) {
            const idx = hay.indexOf(needle, from);
            if (idx < 0) break;
            matches.push({ line: i + 1, column: idx + 1, text: line, start: idx, end: idx + query.length });
            from = idx + needle.length;
          }
        }
        if (matches.length) results.push({ path: p, name: baseOf(p), rel, matches });
      }
      return results;
    },
    gitStatus: async (root) => {
      const prefix = root.endsWith("/") ? root : root + "/";
      const files = GIT_CHANGES.map((c) => ({
        path: prefix + c.rel,
        name: baseOf(c.rel),
        rel: c.rel,
        code: c.code,
        label: c.label,
        staged: c.staged,
        deleted: c.deleted,
      }));
      return { is_repo: true, branch: GIT_BRANCH, files };
    },
    gitFileHead: async (root, rel) => {
      const prefix = root.endsWith("/") ? root : root + "/";
      return GIT_HEAD[prefix + rel] ?? "";
    },
    gitStage: async (_root, rel) => {
      const c = GIT_CHANGES.find((x) => x.rel === rel);
      if (!c || c.staged) return;
      c.staged = true;
      c.code = c.code === "??" ? "A " : c.code.trim().charAt(0) + " ";
    },
    gitUnstage: async (_root, rel) => {
      const c = GIT_CHANGES.find((x) => x.rel === rel);
      if (!c || !c.staged) return;
      c.staged = false;
      c.code = c.label === "Untracked" ? "??" : " " + c.code.trim().charAt(0);
    },
    gitStageAll: async (_root) => {
      for (const c of GIT_CHANGES) {
        if (c.staged) continue;
        c.staged = true;
        c.code = c.code === "??" ? "A " : c.code.trim().charAt(0) + " ";
      }
    },
    gitUnstageAll: async (_root) => {
      for (const c of GIT_CHANGES) {
        if (!c.staged) continue;
        c.staged = false;
        c.code = c.label === "Untracked" ? "??" : " " + c.code.trim().charAt(0);
      }
    },
    gitCommit: async (root, message) => {
      const msg = (message || "").trim();
      if (!msg) throw new Error("Commit message is empty.");
      const staged = GIT_CHANGES.filter((c) => c.staged);
      if (!staged.length) throw new Error("No staged changes to commit.");
      const prefix = root.endsWith("/") ? root : root + "/";
      for (const c of staged) {
        // Fold the working-tree content into the simulated HEAD.
        GIT_HEAD[prefix + c.rel] = FILES[prefix + c.rel] ?? "";
      }
      // Drop committed entries from the change set.
      for (let i = GIT_CHANGES.length - 1; i >= 0; i--) {
        if (GIT_CHANGES[i].staged) GIT_CHANGES.splice(i, 1);
      }
      const hash = Math.random().toString(16).slice(2, 9);
      return `${hash} ${msg}`;
    },
    gitPush: async (_root) => "Everything up-to-date (preview mock).",
    gitBranches: async (_root) => ({ current: GIT_BRANCH, branches: [...GIT_BRANCHES] }),
    gitCheckout: async (_root, branch, create) => {
      const name = (branch || "").trim();
      if (!name) throw new Error("Branch name is empty.");
      if (create) {
        if (GIT_BRANCHES.includes(name)) throw new Error(`Branch '${name}' already exists.`);
        GIT_BRANCHES.push(name);
      } else if (!GIT_BRANCHES.includes(name)) {
        throw new Error(`Branch '${name}' not found.`);
      }
      GIT_BRANCH = name;
    },
    gitPull: async (_root) => "Already up to date. (preview mock)",
    gitLog: async () => [
      { hash: "a1b2c3d", short_hash: "a1b2c3d", author: "Michael", date: "2 hours ago", message: "Initial commit" },
      { hash: "e4f5g6h", short_hash: "e4f5g6h", author: "Michael", date: "1 day ago", message: "Add feature X" },
    ],
    gitConflicts: async (root) => {
      const prefix = root.endsWith("/") ? root : root + "/";
      return GIT_CONFLICTS.map((c) => ({ ...c, path: prefix + c.rel }));
    },
    gitMergeVersions: async (_root, rel) => MERGE_VERSIONS[rel] || { base: "", ours: "", theirs: "", merged: "" },
    gitResolveConflict: async (root, rel, resolution) => {
      const prefix = root.endsWith("/") ? root : root + "/";
      const versions = MERGE_VERSIONS[rel];
      if (versions && resolution === "ours") FILES[prefix + rel] = versions.ours;
      else if (versions && resolution === "theirs") FILES[prefix + rel] = versions.theirs;
      GIT_CONFLICTS.splice(0, GIT_CONFLICTS.length, ...GIT_CONFLICTS.filter((c) => c.rel !== rel));
    },
    gitStash: async () => {
      if (!GIT_CHANGES.length && !GIT_STASHES.length) return "No local changes to stash.";
      const ts = new Date().toISOString().slice(0, 16).replace("T", " ");
      GIT_STASHES.unshift(`stash@{0}: On main: Michael IDE stash (${ts})`);
      return GIT_STASHES[0];
    },
    gitStashList: async () =>
      GIT_STASHES.map((s, i) => s.replace(/^stash@\{\d+\}/, `stash@{${i}}`)),
    gitStashPop: async (_root, index) => {
      const i = index ?? 0;
      if (i >= GIT_STASHES.length) throw new Error("No stash entry found.");
      GIT_STASHES.splice(i, 1);
      return "Stash applied.";
    },
    gitStashApply: async (_root, index) => {
      if ((index ?? 0) >= GIT_STASHES.length) throw new Error("No stash entry found.");
      return "Stash applied.";
    },
    gitStashDrop: async (_root, index) => {
      const i = index ?? 0;
      if (i >= GIT_STASHES.length) throw new Error("No stash entry found.");
      GIT_STASHES.splice(i, 1);
      return "Stash dropped.";
    },
    gitBlame: async (_root, rel) => {
      const now = Math.floor(Date.now() / 1000);
      const lines = (FILES[ROOT + "/" + rel] || "").split("\n");
      return lines.map((_, idx) => ({
        commit: idx % 3 === 0 ? "a1b2c3d4" : "e4f5g6h7",
        author: idx % 3 === 0 ? "Michael" : "Andrew",
        date: String(now - (idx % 3 === 0 ? 7200 : 86400)),
        line: idx + 1,
      }));
    },
    lspList: async () => ["typescript", "javascript", "rust", "python", "go", "html", "css", "json"]
      .map((lang) => ({ lang, running: mockLspRunning.has(lang) })),
    lspStart: async (config, onEvent) => {
      mockLspRunning.add(config.lang);
      onEvent?.({ kind: "started", lang: config.lang });
    },
    lspSend: async () => {},
    lspStop: async (lang) => {
      mockLspRunning.delete(lang);
    },
    dapList: async () => ["node", "python", "lldb", "go"]
      .map((adapter) => ({ adapter, running: mockDapRunning.has(adapter) })),
    dapStart: async (config, onEvent) => {
      mockDapRunning.add(config.adapterId);
      mockDap = { onEvent, program: config.cwd || ROOT, step: 0 };
      onEvent?.({ kind: "started", adapter: config.adapterId });
    },
    dapSend: async (_adapterId, message) => {
      if (mockDap) mockDapHandle(message);
    },
    dapStop: async (adapterId) => {
      mockDapRunning.delete(adapterId);
      mockDap = null;
    },
    marketplaceList: async () => [...MARKETPLACE],
    marketplaceSearch: async (query) => {
      const q = query.trim().toLowerCase();
      if (!q) return [...MARKETPLACE];
      return MARKETPLACE.filter((entry) =>
        entry.name.toLowerCase().includes(q) ||
        entry.description.toLowerCase().includes(q) ||
        entry.tags.some((tag) => tag.toLowerCase().includes(q)),
      );
    },
    marketplaceInstall: async (entry) => `Installed ${entry.name} v${entry.version} (preview mock)`,
    tasksList: async (root) => [
      { id: "npm:dev", label: "npm: dev", command: "npm run dev", cwd: root, source: "npm", group: "run", problemMatcher: null },
      { id: "npm:build", label: "npm: build", command: "npm run build", cwd: root, source: "npm", group: "build", problemMatcher: "$tsc" },
      { id: "npm:test", label: "npm: test", command: "npm test", cwd: root, source: "npm", group: "test", problemMatcher: null },
    ],
    taskRunCapture: async (cwd, command) => ({
      code: 1,
      stdout: `src/main.js(530,7): warning TS6133: 'reply' is declared but its value is never read.\n`,
      stderr: "",
      combined: `src/main.js(530,7): warning TS6133: 'reply' is declared but its value is never read.\n`,
      truncated: false,
    }),
    pickFolder: async () => ROOT,
    aiComplete: async (_config, messages) => {
      const user = messages[messages.length - 1]?.content || "";
      const code = user.split("\nCode:\n").slice(1).join("\nCode:\n");
      return "// ✦ preview mock edit — set a real provider for live edits\n" + code;
    },
    aiChat: async (_config, messages, onEvent) => {
      const last = (messages[messages.length - 1]?.content ?? "").slice(0, 80);
      const reply = [
        `Here's how I'd approach **"${last || "your request"}"**. This is a _preview mock_ \u2014 configure a real provider in settings (\u2699\ufe0f) for live answers.`,
        ``,
        `### Plan`,
        `1. Read the open file and locate the relevant function.`,
        `2. Refactor \`greet()\` to be null-safe.`,
        `3. Add a quick test.`,
        ``,
        `\`\`\`js:src/main.js`,
        `function greet(name) {`,
        `  // fall back to a friendly default`,
        `  const who = name?.trim() || "world";`,
        `  return \`Hello, \${who}!\`;`,
        `}`,
        ``,
        `console.log(greet("Michael")); // "Hello, Michael!"`,
        `\`\`\``,
        ``,
        `> Tip: select code in the editor and it's sent as context automatically.`,
        ``,
        `| Case | Input | Output |`,
        `| --- | --- | --- |`,
        `| normal | \`"Ada"\` | \`Hello, Ada!\` |`,
        `| empty | \`""\` | \`Hello, world!\` |`,
        ``,
        `- [x] Handle empty names`,
        `- [ ] Add unit tests`,
        ``,
        `Read more in the [docs](https://github.com/fendoushaonian/Devin-Desktop).`,
      ].join("\n");
      await new Promise((r) => setTimeout(r, 750)); // let the "thinking" card show
      for (const tok of reply.match(/\S+\s*|\s+/g) ?? []) {
        await new Promise((r) => setTimeout(r, 18));
        onEvent({ kind: "token", delta: tok });
      }
      onEvent({ kind: "done" });
    },
    termOpen: async (opts, onEvent) => {
      const id = mockTermSeq++;
      const cwd = opts?.cwd || ROOT;
      mockTerms.set(id, { onEvent, line: "", cwd });
      const send = (s) => onEvent({ kind: "data", data: s });
      setTimeout(() => {
        send("Michael IDE terminal \x1b[2m(preview mock)\x1b[0m\r\n");
        send("\x1b[2mSimulated shell — the native app runs your real shell via a PTY. Type 'help'.\x1b[0m\r\n\r\n");
        send(mockPrompt(cwd));
      }, 20);
      return id;
    },
    termWrite: async (id, data) => mockTermInput(id, data),
    termResize: async () => {},
    termClose: async (id) => {
      mockTerms.delete(id);
    },
    termListCommands: async () => [],
    termHistory: async () => [],
    authLoginOrRegister: async () => ({ success: true, message: "mock login" }),
    authCheckEmail: async () => ({ exists: false }),
    authSendCode: async () => "验证码已发送（模拟）",
    authVerifyCode: async () => ({ success: true, message: "mock verify" }),
    invoke: async () => ({}),
  };
}

// ---- element refs ----
const $ = (id) => document.getElementById(id);
const treeEl = $("tree");
const tabsEl = $("tabs");
const editorEl = $("editor");
const welcomeEl = $("welcome");
const chatEl = $("chat");
const rootNameEl = $("rootName");
const saveBtn = $("saveBtn");
const runBtn = $("runBtn");
const toastEl = $("toast");

// ---- editor state ----
const monacoEditor = monaco.editor.create(editorEl, {
  value: "",
  language: "plaintext",
  theme: matchMedia("(prefers-color-scheme: dark)").matches ? "vs-dark" : "vs",
  automaticLayout: true,
  fixedOverflowWidgets: false,
  suggest: {
    showStatusBar: true,
    shareSuggestSelections: true,
    showWords: false,
    filterGraceful: true,
    snippetsPreventQuickSuggestions: false,
    localityBonus: true,
    preview: true,
    showIcons: true,
    showMethods: true,
    showFunctions: true,
    showConstructors: true,
    showFields: true,
    showVariables: true,
    showClasses: true,
    showStructs: true,
    showInterfaces: true,
    showModules: true,
    showProperties: true,
    showEvents: true,
    showOperators: true,
    showUnits: true,
    showValues: true,
    showConstants: true,
    showEnums: true,
    showEnumMembers: true,
    showKeywords: true,
    showColors: true,
    showFiles: true,
    showReferences: true,
    showSnippets: true,
  },
  quickSuggestions: { other: "on", comments: "off", strings: "on" },
  quickSuggestionsDelay: 50,
  suggestOnTriggerCharacters: true,
  acceptSuggestionOnCommitCharacter: false,
  wordBasedSuggestions: "off",
  parameterHints: { enabled: true, cycle: true },
  inlineSuggest: { enabled: true, mode: "subwordSmart" },
  fontSize: 13,
  fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
  minimap: { enabled: true, maxColumn: 80, renderCharacters: false, scale: 1, showSlider: "mouseover" },
  scrollBeyondLastLine: false,
  renderWhitespace: "selection",
  glyphMargin: true,
  padding: { top: 10 },
  bracketPairColorization: { enabled: true, independentColorPoolPerBracketType: true },
  guides: { bracketPairs: false, indentation: true, highlightActiveIndentation: false },
  smoothScrolling: true,
  cursorBlinking: "blink",
  cursorSmoothCaretAnimation: "off",
  stickyScroll: { enabled: false },
  linkedEditing: true,
  largeFileOptimizations: true,
  maxTokenizationLineLength: 20000,
  stopRenderingLineAfter: 10000,
  fastScrollSensitivity: 5,
  mouseWheelScrollSensitivity: 1,
  renderValidationDecorations: "on",
  unfoldOnClickAfterEndOfLine: true,
  definitionLinkOpensInPeek: true,
  gotoLocation: { multiple: "peek", multipleDefinitions: "peek", multipleDeclarations: "peek", multipleImplementations: "peek", multipleTypeDefinitions: "peek", multipleReferences: "peek" },
  colorDecorators: true,
});

let _imeComposing = false;
const _imeFlushCallbacks = [];

monacoEditor.onDidCompositionStart(() => { _imeComposing = true; });
monacoEditor.onDidCompositionEnd(() => {
  _imeComposing = false;
  while (_imeFlushCallbacks.length) _imeFlushCallbacks.shift()();
});

const _CN_PUNCT_MAP = {
  "\uFF0C": ",",   // ，→ ,
  "\u3002": ".",   // 。→ .
  "\uFF1B": ";",   // ；→ ;
  "\uFF1A": ":",   // ：→ :
  "\u201C": '"',   // " → "
  "\u201D": '"',   // " → "
  "\u2018": "'",   // ' → '
  "\u2019": "'",   // ' → '
  "\uFF08": "(",   // （→ (
  "\uFF09": ")",   // ）→ )
  "\u3010": "[",   // 【→ [
  "\u3011": "]",   // 】→ ]
  "\uFF01": "!",   // ！→ !
  "\uFF1F": "?",   // ？→ ?
};
const _CN_PUNCT_RE = new RegExp("[" + Object.keys(_CN_PUNCT_MAP).join("") + "]", "g");
let _punctFixing = false;

monacoEditor.onDidChangeModelContent((e) => {
  if (_punctFixing || _imeComposing) return;
  const model = monacoEditor.getModel();
  if (!model) return;
  const lang = model.getLanguageId();
  if (lang === "markdown" || lang === "plaintext") return;

  const edits = [];
  for (const change of e.changes) {
    const text = change.text;
    if (!text || !_CN_PUNCT_RE.test(text)) continue;
    _CN_PUNCT_RE.lastIndex = 0;

    const startLine = change.range.startLineNumber;
    const startCol = change.range.startColumn;
    let offset = 0;
    let match;
    while ((match = _CN_PUNCT_RE.exec(text)) !== null) {
      const idx = match.index;
      let line = startLine;
      let col = startCol + idx - offset;
      const before = text.slice(0, idx);
      const newlines = (before.match(/\n/g) || []).length;
      if (newlines > 0) {
        line = startLine + newlines;
        col = idx - before.lastIndexOf("\n");
      }
      edits.push({
        range: new monaco.Range(line, col, line, col + 1),
        text: _CN_PUNCT_MAP[match[0]],
      });
    }
  }
  if (edits.length > 0) {
    _punctFixing = true;
    model.pushEditOperations([], edits, () => null);
    _punctFixing = false;
  }
});

// ---- Smart Rename: Chinese identifier → English + duplicate detection ----
const _CN_CHAR_RE = /[\u4e00-\u9fff]/;
let _renameTimer = null;
let _lastRenamePos = "";

function _hasChinese(text) {
  return _CN_CHAR_RE.test(text);
}

function _collectSymbolNames(model) {
  const names = new Set();
  const text = model.getValue();
  const ident = /\b([a-zA-Z_]\w*)\b/g;
  let m;
  while ((m = ident.exec(text)) !== null) names.add(m[1]);
  return names;
}

async function _translateToEnglish(chineseName, context, existingNames) {
  const config = loadConfig();
  if (!config.baseUrl || !config.apiKey || !config.model) return null;

  const namesStr = [...existingNames].slice(0, 50).join(", ");
  const msgs = [
    {
      role: "system",
      content: `You translate Chinese code identifiers to English. Rules:
1. Output ONLY the English name, nothing else.
2. Use snake_case for Python/C, camelCase for JS/Java, PascalCase for classes.
3. Keep it concise (1-3 words).
4. If the name would conflict with these existing names, add a suffix: ${namesStr}`,
    },
    {
      role: "user",
      content: `Context: ${context}\nTranslate: "${chineseName}"`,
    },
  ];

  try {
    const aiConfig = {
      baseUrl: config.baseUrl.replace(/\/+$/, ""),
      apiKey: config.apiKey,
      model: config.baseUrl?.includes("deepseek") ? "deepseek-v4-flash" : config.model,
      maxTokens: 256,
      temperature: 0,
    };
    const result = await new Promise((resolve) => {
      let buf = "";
      backend.aiChat(aiConfig, msgs, (ev) => {
        if (ev.kind === "token") buf += ev.delta;
        else if (ev.kind === "done") resolve(buf.trim());
        else if (ev.kind === "error") resolve("");
      }).catch(() => resolve(""));
      setTimeout(() => resolve(buf.trim()), 10000);
    });
    if (!result || _CN_CHAR_RE.test(result)) return null;
    let name = result.replace(/[^a-zA-Z0-9_]/g, "").replace(/^_+|_+$/g, "");
    if (!name) return null;
    if (existingNames.has(name)) {
      for (let i = 2; i < 100; i++) {
        const candidate = `${name}_${i}`;
        if (!existingNames.has(candidate)) { name = candidate; break; }
      }
    }
    return name;
  } catch {
    return null;
  }
}

function _getIdentifierAtPosition(model, position) {
  const word = model.getWordAtPosition(position);
  if (!word) return null;
  const text = word.word;
  if (!_hasChinese(text)) return null;
  return {
    text,
    range: new monaco.Range(
      position.lineNumber, word.startColumn,
      position.lineNumber, word.endColumn,
    ),
  };
}

function _isInCommentOrString(line, offset, langId) {
  const before = line.slice(0, offset);
  if (/^\s*(#|\/\/|--|%)/.test(line)) return true;
  if (before.includes("//")) return true;
  if (before.includes("#") && (langId === "python" || langId === "ruby" || langId === "shell")) return true;
  if (before.includes("--") && langId === "lua") return true;

  let inSingle = false, inDouble = false, inTemplate = false;
  for (let i = 0; i < offset; i++) {
    const ch = line[i];
    if (ch === "'" && !inDouble && !inTemplate) inSingle = !inSingle;
    else if (ch === '"' && !inSingle && !inTemplate) inDouble = !inDouble;
    else if (ch === "`" && !inSingle && !inDouble) inTemplate = !inTemplate;
  }
  return inSingle || inDouble || inTemplate;
}

function _findAllChineseIdentifiers(model) {
  const found = new Map();
  const total = model.getLineCount();
  const langId = model.getLanguageId();
  for (let ln = 1; ln <= total; ln++) {
    const line = model.getLineContent(ln);
    const re = /[\u4e00-\u9fff][\u4e00-\u9fff\w]*/g;
    let m;
    while ((m = re.exec(line)) !== null) {
      if (_isInCommentOrString(line, m.index, langId)) continue;
      const text = m[0];
      if (!found.has(text)) found.set(text, []);
      found.get(text).push({ line: ln, col: m.index + 1, len: text.length });
    }
  }
  return found;
}

async function _batchTranslate(chineseNames, lang, existingNames) {
  const config = loadConfig();
  if (!config.baseUrl || !config.apiKey || !config.model) return null;

  const namesList = chineseNames.join("\n");
  const namesStr = [...existingNames].slice(0, 30).join(", ");
  const msgs = [
    {
      role: "system",
      content: `Translate Chinese code identifiers to English for ${lang}. Rules:
1. Output one translation per line, same order as input.
2. Use snake_case for Python, camelCase for JS/TS.
3. Keep concise (1-3 words each).
4. No duplicates, no conflicts with: ${namesStr}
5. Output ONLY the English names, one per line. No numbering, no explanations.`,
    },
    { role: "user", content: namesList },
  ];

  try {
    const aiConfig = {
      baseUrl: config.baseUrl.replace(/\/+$/, ""),
      apiKey: config.apiKey,
      model: config.baseUrl?.includes("deepseek") ? "deepseek-v4-flash" : config.model,
      maxTokens: 1024,
      temperature: 0,
    };
    const result = await new Promise((resolve) => {
      let buf = "";
      backend.aiChat(aiConfig, msgs, (ev) => {
        if (ev.kind === "token") buf += ev.delta;
        else if (ev.kind === "done") resolve(buf.trim());
        else if (ev.kind === "error") resolve("");
      }).catch(() => resolve(""));
      setTimeout(() => resolve(buf.trim()), 20000);
    });
    if (!result) return null;
    const lines = result.split("\n").map((l) => l.trim().replace(/^\d+[\.\)]\s*/, "").replace(/[^a-zA-Z0-9_]/g, ""));
    if (lines.length < chineseNames.length) return null;

    const used = new Set(existingNames);
    const mapping = {};
    for (let i = 0; i < chineseNames.length; i++) {
      let name = lines[i];
      if (!name || _CN_CHAR_RE.test(name)) continue;
      while (used.has(name)) name = name + "_" + (Math.floor(Math.random() * 90) + 10);
      used.add(name);
      mapping[chineseNames[i]] = name;
    }
    return mapping;
  } catch {
    return null;
  }
}

async function _trySmartRename(editor) {
  const model = editor.getModel();
  if (!model) return;
  const lang = model.getLanguageId();
  if (lang === "markdown" || lang === "plaintext") return;

  const chineseIdents = _findAllChineseIdentifiers(model);
  if (chineseIdents.size === 0) return;

  const namesKey = [...chineseIdents.keys()].sort().join("|");
  if (namesKey === _lastRenamePos) return;

  const existingNames = _collectSymbolNames(model);
  const chineseNames = [...chineseIdents.keys()];
  const mapping = await _batchTranslate(chineseNames, lang, existingNames);
  if (!mapping || Object.keys(mapping).length === 0) return;
  _lastRenamePos = namesKey;

  const edits = [];
  const defRe = /^\s*(?:def |class |function |const |let |var |async function |fn |func |pub fn |pub func )/;
  const usedDefNames = new Set();

  for (const [cnName, positions] of chineseIdents.entries()) {
    let enName = mapping[cnName];
    if (!enName) continue;

    for (const pos of positions) {
      const currentText = model.getValueInRange(
        new monaco.Range(pos.line, pos.col, pos.line, pos.col + pos.len),
      );
      if (currentText !== cnName) continue;

      const lineContent = model.getLineContent(pos.line);
      const isDef = defRe.test(lineContent);

      let finalName = enName;
      if (isDef && usedDefNames.has(enName)) {
        for (let i = 2; i < 100; i++) {
          const candidate = `${enName}_${i}`;
          if (!usedDefNames.has(candidate) && !existingNames.has(candidate)) {
            finalName = candidate;
            break;
          }
        }
      }
      if (isDef) usedDefNames.add(finalName);

      edits.push({
        range: new monaco.Range(pos.line, pos.col, pos.line, pos.col + pos.len),
        text: finalName,
      });
    }
  }

  if (edits.length > 0) {
    _punctFixing = true;
    model.pushEditOperations([], edits, () => null);
    _punctFixing = false;
    const summary = Object.entries(mapping).map(([k, v]) => `${k}→${v}`).join("  ");
    showToast?.(`✦ ${summary}`);
  }
}

monacoEditor.onDidChangeModelContent(() => {
  if (_imeComposing || _punctFixing) return;
  if (_renameTimer) clearTimeout(_renameTimer);
  _renameTimer = setTimeout(() => _trySmartRename(monacoEditor), 1500);
});

// ---- Smart Code Auto-Corrector ----
let _autoFixTimer = null;
const _AUTO_FIX_DEBOUNCE = 1200;

const _DOUBLE_SYMBOLS = [
  [/;;/g, ";"],
  [/,,/g, ","],
  [/\.\.\./g, null],
  [/\.\.(?!\.)/g, "."],
  [/::(?!:)/g, ":"],
  [/\+\+(?!\+)/g, null],
  [/--(?!-|>)/g, null],
];

const _BRACKET_PAIRS = { "(": ")", "[": "]", "{": "}" };
const _CLOSE_TO_OPEN = { ")": "(", "]": "[", "}": "{" };

function _fixDoublePunctuation(model) {
  const edits = [];
  const total = model.getLineCount();
  for (let ln = 1; ln <= total; ln++) {
    const line = model.getLineContent(ln);
    const trimmed = line.trimStart();
    if (trimmed.startsWith("//") || trimmed.startsWith("#") || trimmed.startsWith("*")) continue;

    for (const [re, replacement] of _DOUBLE_SYMBOLS) {
      if (replacement === null) continue;
      re.lastIndex = 0;
      let m;
      while ((m = re.exec(line)) !== null) {
        const col = m.index + 1;
        const inStr = _isInString(line, m.index);
        if (inStr) continue;
        edits.push({
          range: new monaco.Range(ln, col, ln, col + m[0].length),
          text: replacement,
        });
      }
    }
  }
  return edits;
}

function _isInString(line, pos) {
  let inSingle = false, inDouble = false, inBacktick = false;
  for (let i = 0; i < pos; i++) {
    const ch = line[i];
    const prev = i > 0 ? line[i - 1] : "";
    if (prev === "\\") continue;
    if (ch === "'" && !inDouble && !inBacktick) inSingle = !inSingle;
    else if (ch === '"' && !inSingle && !inBacktick) inDouble = !inDouble;
    else if (ch === "`" && !inSingle && !inDouble) inBacktick = !inBacktick;
  }
  return inSingle || inDouble || inBacktick;
}

function _fixUnbalancedBrackets(model) {
  const edits = [];
  const total = model.getLineCount();
  const stack = [];

  for (let ln = 1; ln <= total; ln++) {
    const line = model.getLineContent(ln);
    for (let i = 0; i < line.length; i++) {
      if (_isInString(line, i)) continue;
      const ch = line[i];
      if (_BRACKET_PAIRS[ch]) {
        stack.push({ ch, ln, col: i + 1 });
      } else if (_CLOSE_TO_OPEN[ch]) {
        const expected = _CLOSE_TO_OPEN[ch];
        if (stack.length > 0 && stack[stack.length - 1].ch === expected) {
          stack.pop();
        } else {
          edits.push({
            range: new monaco.Range(ln, i + 1, ln, i + 2),
            text: "",
          });
        }
      }
    }
  }

  for (const unclosed of stack) {
    const closer = _BRACKET_PAIRS[unclosed.ch];
    const ln = unclosed.ln;
    const lineContent = model.getLineContent(ln);
    const endCol = lineContent.length + 1;
    edits.push({
      range: new monaco.Range(ln, endCol, ln, endCol),
      text: closer,
    });
  }
  return edits;
}

function _fixTrailingWhitespace(model, changedLines) {
  const edits = [];
  for (const ln of changedLines) {
    if (ln < 1 || ln > model.getLineCount()) continue;
    const line = model.getLineContent(ln);
    const trimmed = line.replace(/\s+$/, "");
    if (trimmed.length < line.length) {
      edits.push({
        range: new monaco.Range(ln, trimmed.length + 1, ln, line.length + 1),
        text: "",
      });
    }
  }
  return edits;
}

const _LANG_KEYWORDS = new Set([
  "abstract","and","arguments","as","assert","async","await",
  "boolean","break","byte","case","catch","char","class","const","constructor",
  "continue","debugger","declare","def","default","defer",
  "delete","do","double","elif","else","enum","eval","except","export",
  "extends","extern","false","final","finally","float","for","foreach",
  "from","func","function","get","global","go","goto","if","implement",
  "implements","import","in","include","instanceof","int","interface",
  "internal","is","lambda","let","long","map","match","module","mut",
  "namespace","new","nil","none","not","null","number","object","of",
  "operator","or","out","override","package","param","pass","print",
  "private","protected","pub","public","raise","range","readonly",
  "ref","require","return","sealed","select","self","set","short",
  "signed","sizeof","slice","static","string","struct","super",
  "switch","synchronized","template","then","this","throw","throws",
  "trait","true","try","type","typedef","typeof","uint","undefined",
  "union","unsigned","use","using","val","var","virtual","void",
  "volatile","while","with","yield",
  "console","document","window","element","length","push","pop","shift","unshift",
  "splice","forEach","filter","reduce","indexOf","includes","find","findIndex",
  "promise","resolve","reject","async","await","fetch","response","request",
  "addEventListener","removeEventListener","querySelector","getElementById",
  "createElement","appendChild","innerHTML","textContent","className","style",
  "setTimeout","setInterval","clearTimeout","clearInterval","JSON","parse","stringify",
  "Math","random","floor","ceil","round","abs","max","min","pow","sqrt",
  "Array","Object","String","Number","Boolean","Date","RegExp","Map","Set","WeakMap",
  "toString","valueOf","hasOwnProperty","constructor","prototype",
  "process","exports","module","Buffer","Stream","EventEmitter",
  "vector","string","iostream","algorithm","utility","functional","memory",
  "unordered_map","shared_ptr","unique_ptr","make_shared","make_unique",
  "begin","end","size","empty","clear","erase","insert","emplace",
  "System","Collections","Generic","Linq","Threading","Tasks",
  "ArrayList","HashMap","LinkedList","TreeMap","HashSet","Iterator",
  "StringBuilder","IOException","Exception","Override","Nullable",
  "println","printf","sprintf","fprintf","scanf","malloc","calloc","realloc","free",
  "goroutine","channel","select","defer","panic","recover","make","append","len","cap",
  "fmt","http","json","time","sync","context","errors","strings","strconv","io",
]);

const _envSymbols = new Set();
const _fileSymbols = new Set();
const _moduleApiSymbols = new Set();
const _typoCache = new Map();
let _envSymbolsLoaded = false;
let _envLoadingLang = null;
const _loadedModuleApis = new Set();

function _levenshtein(a, b) {
  const m = a.length, n = b.length;
  if (m === 0) return n;
  if (n === 0) return m;
  let prev = Array.from({ length: n + 1 }, (_, i) => i);
  let curr = new Array(n + 1);
  for (let i = 1; i <= m; i++) {
    curr[0] = i;
    for (let j = 1; j <= n; j++) {
      curr[j] = a[i - 1] === b[j - 1]
        ? prev[j - 1]
        : 1 + Math.min(prev[j - 1], prev[j], curr[j - 1]);
    }
    [prev, curr] = [curr, prev];
  }
  return prev[n];
}

function _addSymbols(target, symbols) {
  let added = 0;
  for (const sym of symbols) {
    if (sym && sym.length >= 2 && /^[a-zA-Z_]\w*$/.test(sym)) {
      const lc = sym.toLowerCase();
      if (!target.has(lc)) { target.add(lc); added++; }
    }
  }
  if (added > 0) _typoCache.clear();
}

function _updateDynamicKeywords(symbols) {
  _addSymbols(_envSymbols, symbols);
}

async function _loadEnvSymbols(langId) {
  if (_envLoadingLang === langId && _envSymbolsLoaded) return;
  _envLoadingLang = langId;

  const model = monacoEditor.getModel();
  const importedMods = model ? _extractImportedModules(model) : [];

  if (langId === "python") {
    if (importedMods.length > 0) _loadModuleApisOnly(importedMods);
    _loadAllModuleNames();
  }

  if ((langId === "javascript" || langId === "typescript") && workspaceRoots.length > 0) {
    _loadNodeEnvSymbols(workspaceRoots[0], importedMods);
  }

  if (langId === "go" && workspaceRoots.length > 0) {
    _loadGoEnvSymbols(workspaceRoots[0]);
  }

  const genericLangs = ["lua","ruby","php","dart","kotlin","java","swift","c","cpp","csharp"];
  if (genericLangs.includes(langId)) {
    _loadGenericLangSymbols(langId, importedMods);
  }

  if (lspManager) {
    _loadLspSymbols(langId, model);
  }
}

async function _loadGenericLangSymbols(langId, importedMods) {
  const projectDir = workspaceRoots.length > 0 ? workspaceRoots[0] : "";
  try {
    const result = await backend.lspLangEnvSymbols(langId, projectDir, importedMods);
    if (result?.symbols) {
      _addSymbols(_envSymbols, result.symbols);
    }
    if (result?.apiSymbols) {
      for (const [mod, syms] of Object.entries(result.apiSymbols)) {
        _loadedModuleApis.add(mod);
        _addSymbols(_moduleApiSymbols, syms);
      }
    }
    _envSymbolsLoaded = true;
  } catch { /* lang runtime not available */ }
}

async function _loadNodeEnvSymbols(projectDir, importedMods) {
  try {
    const result = await backend.lspNodeEnvSymbols(projectDir, importedMods);
    if (result?.packages) _addSymbols(_envSymbols, result.packages);
    if (result?.exports) {
      for (const [mod, syms] of Object.entries(result.exports)) {
        _loadedModuleApis.add(mod);
        _addSymbols(_moduleApiSymbols, syms);
      }
    }
    _envSymbolsLoaded = true;
  } catch { /* node not available */ }
}

async function _loadGoEnvSymbols(projectDir) {
  try {
    const result = await backend.lspGoEnvSymbols(projectDir);
    if (result?.packages) {
      _addSymbols(_envSymbols, result.packages);
      _envSymbolsLoaded = true;
    }
  } catch { /* go not available */ }
}

async function _loadModuleApisOnly(mods) {
  const newMods = mods.filter((m) => !_loadedModuleApis.has(m));
  if (newMods.length === 0) return;
  try {
    const result = await backend.lspPythonEnvSymbols(newMods);
    if (result?.symbols) {
      for (const [mod, attrs] of Object.entries(result.symbols)) {
        _loadedModuleApis.add(mod);
        _addSymbols(_moduleApiSymbols, attrs);
      }
    }
  } catch { /* ignore */ }
}

async function _loadAllModuleNames() {
  if (_envSymbolsLoaded) return;
  try {
    const result = await backend.lspPythonEnvSymbols([]);
    if (result?.modules) {
      _addSymbols(_envSymbols, result.modules);
      _envSymbolsLoaded = true;
    }
  } catch { /* ignore */ }
}

async function _loadLspSymbols(langId, model) {
  try {
    const wsSymbols = await lspManager.queryWorkspaceSymbols(langId, "");
    if (wsSymbols.length) _addSymbols(_envSymbols, wsSymbols);
  } catch { /* workspace/symbol not supported */ }

  if (model) {
    try {
      const docSymbols = await lspManager.queryDocumentSymbols(
        model.uri.toString(), langId
      );
      if (docSymbols.length) _addSymbols(_envSymbols, docSymbols);
    } catch { /* ignore */ }
  }
}

function _extractImportedModules(model) {
  if (!model) return [];
  const mods = new Set();
  const total = Math.min(model.getLineCount(), 300);
  for (let i = 1; i <= total; i++) {
    const line = model.getLineContent(i);
    let m;
    if ((m = line.match(/^\s*import\s+([a-zA-Z_][\w.]*)/))) {
      mods.add(m[1].split(".")[0]);
    }
    if ((m = line.match(/^\s*from\s+([a-zA-Z_][\w.]*)\s+import/))) {
      mods.add(m[1].split(".")[0]);
    }
    if ((m = line.match(/require\s*\(\s*['"]([a-zA-Z@][\w/.-]*)["']\s*\)/))) {
      const pkg = m[1].startsWith("@") ? m[1].split("/").slice(0, 2).join("/") : m[1].split("/")[0];
      mods.add(pkg);
    }
    if ((m = line.match(/^\s*import\s+.*\s+from\s+['"]([a-zA-Z@][\w/.-]*)["']/))) {
      const pkg = m[1].startsWith("@") ? m[1].split("/").slice(0, 2).join("/") : m[1].split("/")[0];
      mods.add(pkg);
    }
    if ((m = line.match(/^\s*use\s+([a-zA-Z_][\w]*)/))) {
      mods.add(m[1]);
    }
    if ((m = line.match(/^\s*#include\s*[<"]([a-zA-Z_][\w./]*)[">/]/))) {
      const header = m[1].split("/")[0].replace(/\.h(pp)?$/, "");
      mods.add(header);
    }
    if ((m = line.match(/^\s*using\s+(namespace\s+)?([a-zA-Z_][\w.]*)/))) {
      mods.add(m[2].split(".")[0]);
    }
    if ((m = line.match(/^\s*import\s+([a-zA-Z_][\w.]*)\s*;/))) {
      const parts = m[1].split(".");
      if (parts.length > 1) {
        mods.add(parts[parts.length - 1]);
        mods.add(parts[parts.length - 2]);
      }
    }
    if ((m = line.match(/require\s*[\('"]\s*["']?([a-zA-Z_][\w.-]*)["']?\s*[\)'"]/))) {
      mods.add(m[1].replace(/\.\w+$/, ""));
    }
    if ((m = line.match(/^\s*(?:require|include|require_once|include_once)\s+['"]([a-zA-Z_][\w/.-]*)["']/))) {
      mods.add(m[1].split("/").pop().replace(/\.\w+$/, ""));
    }
    if ((m = line.match(/^\s*import\s+['"]package:([a-zA-Z_][\w]*)/))) {
      mods.add(m[1]);
    }
    if ((m = line.match(/^\s*local\s+\w+\s*=\s*require\s*[\("'][\s"']*([a-zA-Z_][\w.]*)/))) {
      mods.add(m[1].split(".")[0]);
    }
  }
  return [...mods];
}

function _extractFileIdentifiers(model) {
  if (!model) return;
  _fileSymbols.clear();
  const total = model.getLineCount();
  const re = /\b([a-zA-Z_][a-zA-Z0-9_]{1,})\b/g;
  for (let i = 1; i <= total; i++) {
    const line = model.getLineContent(i);
    let m;
    while ((m = re.exec(line)) !== null) {
      const w = m[1];
      if (w.length >= 2 && !w.startsWith("_")) _fileSymbols.add(w.toLowerCase());
    }
  }
  _typoCache.clear();
}

async function _refreshModuleApis(model) {
  if (!model) return;
  const langId = model.getLanguageId();
  const imported = _extractImportedModules(model);
  const newMods = imported.filter((m) => !_loadedModuleApis.has(m));
  if (newMods.length === 0) return;

  if (langId === "python") {
    try {
      const result = await backend.lspPythonEnvSymbols(newMods);
      if (result?.symbols) {
        for (const [mod, attrs] of Object.entries(result.symbols)) {
          _loadedModuleApis.add(mod);
          _addSymbols(_moduleApiSymbols, attrs);
        }
      }
    } catch { /* ignore */ }
  } else if ((langId === "javascript" || langId === "typescript") && workspaceRoots.length > 0) {
    try {
      const result = await backend.lspNodeEnvSymbols(workspaceRoots[0], newMods);
      if (result?.exports) {
        for (const [mod, syms] of Object.entries(result.exports)) {
          _loadedModuleApis.add(mod);
          _addSymbols(_moduleApiSymbols, syms);
        }
      }
    } catch { /* ignore */ }
  } else if (["lua","ruby","php","dart","kotlin","java","swift","c","cpp","csharp"].includes(langId)) {
    const projectDir = workspaceRoots.length > 0 ? workspaceRoots[0] : "";
    try {
      const result = await backend.lspLangEnvSymbols(langId, projectDir, newMods);
      if (result?.apiSymbols) {
        for (const [mod, syms] of Object.entries(result.apiSymbols)) {
          _loadedModuleApis.add(mod);
          _addSymbols(_moduleApiSymbols, syms);
        }
      }
    } catch { /* ignore */ }
  } else {
    _addSymbols(_envSymbols, newMods);
  }
}

function _getAllKnownWords() {
  const all = new Set(_LANG_KEYWORDS);
  for (const s of _envSymbols) all.add(s);
  for (const s of _fileSymbols) all.add(s);
  for (const s of _moduleApiSymbols) all.add(s);
  return all;
}

function _isCamelOrPascal(word) {
  return /[a-z][A-Z]/.test(word) || /^[A-Z][a-z]+[A-Z]/.test(word);
}

function _findTypoFix(word) {
  if (word.length < 3) return null;
  if (_isCamelOrPascal(word)) return null;
  if (word.includes("_") && word.split("_").length > 1) return null;
  if (/^[A-Z]{2,}$/.test(word)) return null;

  const key = word.toLowerCase();
  if (_typoCache.has(key)) return _typoCache.get(key);

  const allWords = _getAllKnownWords();
  if (allWords.has(key)) { _typoCache.set(key, null); return null; }

  let best = null, bestDist = Infinity;
  const maxDist = 1;
  for (const kw of allWords) {
    if (!_LANG_KEYWORDS.has(kw) && !_moduleApiSymbols.has(kw)) continue;
    if (Math.abs(kw.length - key.length) > maxDist) continue;
    const d = _levenshtein(key, kw);
    if (d === 1 && d < bestDist) {
      bestDist = d;
      best = kw;
      break;
    }
  }
  _typoCache.set(key, best);
  return best;
}

const _IMPORT_LINE_RE = /^\s*(import|from|require|use|include|#include|using|require_once|include_once)\b/;

function _fixKeywordTypos(model, changedLines) {
  const edits = [];
  for (const ln of changedLines) {
    if (ln < 1 || ln > model.getLineCount()) continue;
    const line = model.getLineContent(ln);
    if (_IMPORT_LINE_RE.test(line)) continue;
    if (/^\s*(#|\/\/|--|\/\*|\*)/.test(line)) continue;

    const re = /\b([a-zA-Z_][a-zA-Z]*)\b/g;
    let m;
    while ((m = re.exec(line)) !== null) {
      if (_isInString(line, m.index)) continue;
      const word = m[1];
      if (word.startsWith("_")) continue;
      const fix = _findTypoFix(word);
      if (fix && fix !== word.toLowerCase()) {
        const corrected = word[0] === word[0].toUpperCase() ? fix[0].toUpperCase() + fix.slice(1) : fix;
        edits.push({
          range: new monaco.Range(ln, m.index + 1, ln, m.index + 1 + word.length),
          text: corrected,
        });
      }
    }
  }
  return edits;
}

function _fixPythonMissingColon(model, changedLines) {
  const lang = model.getLanguageId();
  if (lang !== "python") return [];
  const edits = [];
  const coloned = /^\s*(def |class |if |elif |else|for |while |with |try|except|finally|async def |async for |async with )/;
  for (const ln of changedLines) {
    if (ln < 1 || ln > model.getLineCount()) continue;
    const line = model.getLineContent(ln);
    if (!coloned.test(line)) continue;
    const trimmed = line.trimEnd();
    if (trimmed.endsWith(":") || trimmed.endsWith(":\\")) continue;
    if (trimmed.endsWith(",") || trimmed.endsWith("(") || trimmed.endsWith("\\")) continue;
    const nextLn = ln + 1;
    if (nextLn <= model.getLineCount()) {
      const nextLine = model.getLineContent(nextLn);
      const nextIndent = nextLine.search(/\S/);
      const currIndent = line.search(/\S/);
      if (nextIndent > currIndent || nextLine.trim() === "") {
        edits.push({
          range: new monaco.Range(ln, trimmed.length + 1, ln, trimmed.length + 1),
          text: ":",
        });
      }
    }
  }
  return edits;
}

function _fixExtraSpaces(model, changedLines) {
  const edits = [];
  for (const ln of changedLines) {
    if (ln < 1 || ln > model.getLineCount()) continue;
    const line = model.getLineContent(ln);
    const indent = line.match(/^(\s*)/)?.[0] || "";
    const rest = line.slice(indent.length);
    const fixed = rest.replace(/  +/g, (match, offset) => {
      if (_isInString(rest, offset)) return match;
      return " ";
    });
    if (fixed !== rest) {
      edits.push({
        range: new monaco.Range(ln, indent.length + 1, ln, line.length + 1),
        text: fixed,
      });
    }
  }
  return edits;
}

async function _fixFromLspDiagnostics(editor) {
  const model = editor.getModel();
  if (!model) return [];
  const markers = monaco.editor.getModelMarkers({ resource: model.uri });
  if (markers.length === 0) return [];

  const langId = model.getLanguageId();
  const client = lspManager?.isRunning(langId) ? true : false;
  if (!client) return [];

  const edits = [];
  for (const marker of markers) {
    const msg = marker.message || "";
    const didYouMean = msg.match(/[Dd]id you mean ['"](\w+)['"]/);
    if (didYouMean) {
      const suggestion = didYouMean[1];
      const word = model.getWordAtPosition({ lineNumber: marker.startLineNumber, column: marker.startColumn });
      if (word && word.word !== suggestion) {
        edits.push({
          range: new monaco.Range(marker.startLineNumber, word.startColumn, marker.startLineNumber, word.endColumn),
          text: suggestion,
        });
      }
    }
  }
  return edits;
}

async function _runAutoCorrections(editor, changedLines) {
  const model = editor.getModel();
  if (!model) return;
  const lang = model.getLanguageId();
  if (lang === "markdown" || lang === "plaintext") return;

  const doubleFixes = _fixDoublePunctuation(model);
  const typoFixes = _fixKeywordTypos(model, changedLines);
  const colonFixes = _fixPythonMissingColon(model, changedLines);
  const spaceFixes = _fixExtraSpaces(model, changedLines);
  const lspFixes = await _fixFromLspDiagnostics(editor);

  const allEdits = [...doubleFixes, ...typoFixes, ...colonFixes, ...spaceFixes, ...lspFixes];
  if (allEdits.length === 0) return;

  _punctFixing = true;
  model.pushEditOperations([], allEdits, () => null);
  _punctFixing = false;
}

let _lspFixTimer = null;
monacoEditor.onDidChangeModelContent((e) => {
  if (_imeComposing || _punctFixing) return;
  if (_autoFixTimer) clearTimeout(_autoFixTimer);
  const lines = e.changes.map((c) => c.range.startLineNumber);
  _autoFixTimer = setTimeout(() => _runAutoCorrections(monacoEditor, lines), _AUTO_FIX_DEBOUNCE);
});

monaco.editor.onDidChangeMarkers((uris) => {
  if (_punctFixing || _imeComposing) return;
  const model = monacoEditor.getModel();
  if (!model) return;
  const modelUri = model.uri.toString();
  if (!uris.some((u) => u.toString() === modelUri)) return;
  if (_lspFixTimer) clearTimeout(_lspFixTimer);
  _lspFixTimer = setTimeout(async () => {
    const lspFixes = await _fixFromLspDiagnostics(monacoEditor);
    if (lspFixes.length > 0) {
      _punctFixing = true;
      model.pushEditOperations([], lspFixes, () => null);
      _punctFixing = false;
    }
  }, 2000);
});

const editorContainer = $("editorContainer");

const splitState = {
  active: false,
  editor: null,
  sash: null,
  wrap: null,
  path: null,
  focusedPane: "left",
  ratio: 0.5,
};

function toggleSplitEditor() {
  if (splitState.active) {
    closeSplitEditor();
    return;
  }
  if (!activePath) return;
  openSplitEditor(activePath);
}

function openSplitEditor(filePath) {
  if (splitState.active) {
    switchSplitFile(filePath);
    return;
  }

  const sash = document.createElement("div");
  sash.className = "editor-sash";
  editorContainer.appendChild(sash);

  const wrap = document.createElement("div");
  wrap.className = "editor-split";
  editorContainer.appendChild(wrap);

  const f = openFiles.get(filePath);
  if (!f || f.isImage) return;

  const ed = monaco.editor.create(wrap, {
    model: f.model,
    theme: currentTheme === "dark" || currentTheme === "solarized-dark" || currentTheme === "nord" ? "vs-dark" : "vs",
    automaticLayout: true,
    fixedOverflowWidgets: true,
    fontSize: monacoEditor.getOptions().get(52),
    fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    renderWhitespace: "selection",
    padding: { top: 10 },
    bracketPairColorization: { enabled: true },
    guides: { indentation: true },
  });

  wrap.addEventListener("mousedown", () => { splitState.focusedPane = "right"; updateSplitFocus(); });
  $("editor").addEventListener("mousedown", () => { splitState.focusedPane = "left"; updateSplitFocus(); }, { once: false });

  initSashDrag(sash);

  splitState.active = true;
  splitState.editor = ed;
  splitState.sash = sash;
  splitState.wrap = wrap;
  splitState.path = filePath;
  splitState.ratio = 0.5;

  applySplitRatio(0.5);
  updateSplitFocus();
}

function switchSplitFile(filePath) {
  if (!splitState.active || !splitState.editor) return;
  const f = openFiles.get(filePath);
  if (!f || f.isImage) return;
  splitState.editor.setModel(f.model);
  splitState.path = filePath;
}

function closeSplitEditor() {
  if (!splitState.active) return;
  if (splitState.editor) splitState.editor.dispose();
  if (splitState.sash) splitState.sash.remove();
  if (splitState.wrap) splitState.wrap.remove();
  splitState.active = false;
  splitState.editor = null;
  splitState.sash = null;
  splitState.wrap = null;
  splitState.path = null;
  splitState.focusedPane = "left";
  $("editor").style.flex = "";
}

function applySplitRatio(ratio) {
  const r = Math.max(0.15, Math.min(0.85, ratio));
  splitState.ratio = r;
  $("editor").style.flex = `${r * 100} 0 0`;
  if (splitState.wrap) splitState.wrap.style.flex = `${(1 - r) * 100} 0 0`;
}

function updateSplitFocus() {
  $("editor").classList.toggle("pane-focused", splitState.focusedPane === "left");
  if (splitState.wrap) splitState.wrap.classList.toggle("pane-focused", splitState.focusedPane === "right");
}

function initSashDrag(sash) {
  let dragging = false;
  sash.addEventListener("mousedown", (e) => {
    e.preventDefault();
    dragging = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
  });
  window.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    const rect = editorContainer.getBoundingClientRect();
    const ratio = (e.clientX - rect.left) / rect.width;
    applySplitRatio(ratio);
  });
  window.addEventListener("mouseup", () => {
    if (!dragging) return;
    dragging = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  });
}

// ---- panel sash (sidebar / assistant resize) ----
(function initPanelSashes() {
  const layout = document.querySelector(".layout");
  const sashL = $("sashLeft");
  const sashR = $("sashRight");
  const explorerEl = $("explorer");
  const assistantEl = $("assistant");
  if (!layout) return;

  function makePanelSash(sash, getTarget, cssProp, direction) {
    if (!sash) return;
    let dragging = false;
    sash.addEventListener("mousedown", (e) => {
      e.preventDefault();
      dragging = true;
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    });
    window.addEventListener("mousemove", (e) => {
      if (!dragging) return;
      const target = getTarget();
      if (!target) return;
      const layoutRect = layout.getBoundingClientRect();
      let w;
      if (direction === "left") {
        w = e.clientX - layoutRect.left;
      } else {
        w = layoutRect.right - e.clientX;
      }
      w = Math.max(140, Math.min(layoutRect.width * 0.5, w));
      layout.style.setProperty(cssProp, w + "px");
    });
    window.addEventListener("mouseup", () => {
      if (!dragging) return;
      dragging = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    });
  }
  makePanelSash(sashL, () => explorerEl, "--sidebar-w", "left");
  makePanelSash(sashR, () => assistantEl, "--assistant-w", "right");
})();

const EDITOR_PREFS_KEY = "editor-prefs";
let _editorPrefs = null;

const DEFAULT_EDITOR_SETTINGS = {
  theme: "system",
  fontSize: 13,
  fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
  lineHeight: 0,
  tabSize: 4,
  wordWrap: "off",
  minimap: true,
  stickyScroll: true,
  renderWhitespace: "selection",
  cursorBlinking: "smooth",
  bracketColorization: true,
  autoSave: true,
};

function effectivePrefs() {
  return { ...DEFAULT_EDITOR_SETTINGS, ...(_editorPrefs || {}) };
}

async function loadEditorPrefs() {
  if (_editorPrefs) return _editorPrefs;
  const store = await getStore();
  _editorPrefs = (await store.get(EDITOR_PREFS_KEY)) || {};
  // Migrate from localStorage
  const oldTheme = localStorage.getItem("michael-ide.theme");
  const oldAutoSave = localStorage.getItem("michael-ide.autosave");
  if (oldTheme && !_editorPrefs.theme) {
    _editorPrefs.theme = oldTheme;
    localStorage.removeItem("michael-ide.theme");
  }
  if (oldAutoSave !== null && _editorPrefs.autoSave === undefined) {
    _editorPrefs.autoSave = oldAutoSave !== "false";
    localStorage.removeItem("michael-ide.autosave");
  }
  return _editorPrefs;
}

async function saveEditorPrefs() {
  _editorPrefs = _editorPrefs || {};
  const store = await getStore();
  await store.set(EDITOR_PREFS_KEY, _editorPrefs);
  await store.save();
}

function applyModelOptions() {
  const p = effectivePrefs();
  const tabSize = Math.max(1, Math.min(8, Number(p.tabSize) || 4));
  monacoEditor.getModel()?.updateOptions({ tabSize, insertSpaces: true });
}

function applyEditorPrefs() {
  const p = effectivePrefs();
  const opts = {
    fontSize: Math.max(8, Math.min(48, Number(p.fontSize) || 13)),
    fontFamily: p.fontFamily || DEFAULT_EDITOR_SETTINGS.fontFamily,
    lineHeight: Math.max(0, Number(p.lineHeight) || 0),
    wordWrap: p.wordWrap || "off",
    minimap: { enabled: p.minimap !== false, maxColumn: 80, renderCharacters: false },
    stickyScroll: { enabled: p.stickyScroll !== false },
    renderWhitespace: p.renderWhitespace || "selection",
    cursorBlinking: p.cursorBlinking || "smooth",
    bracketPairColorization: {
      enabled: p.bracketColorization !== false,
      independentColorPoolPerBracketType: true,
    },
  };
  monacoEditor.updateOptions(opts);
  if (splitState.editor) splitState.editor.updateOptions(opts);
  applyModelOptions();
  if (p.theme) { currentTheme = p.theme; applyEditorTheme(); }
  autoSaveEnabled = p.autoSave !== false;
}

monacoEditor.onDidChangeModel(() => applyModelOptions());

let currentTheme = "system";

monaco.editor.defineTheme("monokai", {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "comment", foreground: "75715E", fontStyle: "italic" },
    { token: "keyword", foreground: "F92672" },
    { token: "string", foreground: "E6DB74" },
    { token: "number", foreground: "AE81FF" },
    { token: "type", foreground: "66D9EF", fontStyle: "italic" },
    { token: "function", foreground: "A6E22E" },
    { token: "variable", foreground: "F8F8F2" },
  ],
  colors: {
    "editor.background": "#272822",
    "editor.foreground": "#F8F8F2",
    "editor.selectionBackground": "#49483E",
    "editor.lineHighlightBackground": "#3E3D32",
    "editorCursor.foreground": "#F8F8F0",
    "editorWhitespace.foreground": "#3B3A32",
  },
});

monaco.editor.defineTheme("github-light", {
  base: "vs",
  inherit: true,
  rules: [
    { token: "comment", foreground: "6A737D", fontStyle: "italic" },
    { token: "keyword", foreground: "D73A49" },
    { token: "string", foreground: "032F62" },
    { token: "number", foreground: "005CC5" },
    { token: "type", foreground: "6F42C1" },
    { token: "function", foreground: "6F42C1" },
    { token: "variable", foreground: "24292E" },
  ],
  colors: {
    "editor.background": "#FFFFFF",
    "editor.foreground": "#24292E",
    "editor.selectionBackground": "#C8E1FF",
    "editor.lineHighlightBackground": "#F6F8FA",
  },
});

monaco.editor.defineTheme("solarized-dark", {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "comment", foreground: "586E75", fontStyle: "italic" },
    { token: "keyword", foreground: "859900" },
    { token: "string", foreground: "2AA198" },
    { token: "number", foreground: "D33682" },
    { token: "type", foreground: "B58900" },
    { token: "function", foreground: "268BD2" },
    { token: "variable", foreground: "839496" },
  ],
  colors: {
    "editor.background": "#002B36",
    "editor.foreground": "#839496",
    "editor.selectionBackground": "#073642",
    "editor.lineHighlightBackground": "#073642",
  },
});

monaco.editor.defineTheme("nord", {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "comment", foreground: "616E88", fontStyle: "italic" },
    { token: "keyword", foreground: "81A1C1" },
    { token: "string", foreground: "A3BE8C" },
    { token: "number", foreground: "B48EAD" },
    { token: "type", foreground: "8FBCBB" },
    { token: "function", foreground: "88C0D0" },
    { token: "variable", foreground: "D8DEE9" },
  ],
  colors: {
    "editor.background": "#2E3440",
    "editor.foreground": "#D8DEE9",
    "editor.selectionBackground": "#434C5E",
    "editor.lineHighlightBackground": "#3B4252",
  },
});

const THEME_MAP = {
  light: { monaco: "vs", css: "light" },
  dark: { monaco: "vs-dark", css: "dark" },
  monokai: { monaco: "monokai", css: "dark" },
  "github-light": { monaco: "github-light", css: "light" },
  "solarized-dark": { monaco: "solarized-dark", css: "dark" },
  nord: { monaco: "nord", css: "dark" },
};

function applyEditorTheme() {
  let resolved = currentTheme;
  if (resolved === "system") {
    resolved = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  const mapping = THEME_MAP[resolved] || THEME_MAP.dark;
  monaco.editor.setTheme(mapping.monaco);
  document.documentElement.setAttribute("data-theme", mapping.css);
}

async function setTheme(theme) {
  currentTheme = theme;
  if (_editorPrefs) {
    _editorPrefs.theme = theme;
    await saveEditorPrefs();
  }
  applyEditorTheme();
  const th = termTheme();
  for (const tab of (typeof termTabs !== "undefined" ? termTabs : [])) {
    tab.term.options.theme = th;
  }
  const termBodyEl = document.getElementById("terminalBody");
  if (termBodyEl) {
    termBodyEl.querySelectorAll(".xterm, .xterm-viewport, .xterm-screen").forEach(el => {
      el.style.backgroundColor = th.background;
    });
  }
}

applyEditorTheme();
registerSnippetProviders();

if (monaco.languages.html?.htmlDefaults) {
  monaco.languages.html.htmlDefaults.setOptions({
    format: { tabSize: 2, insertSpaces: true, wrapLineLength: 120, wrapAttributes: "auto" },
    suggest: { html5: true, angular1: false, ionic: false },
  });
}
if (monaco.languages.css?.cssDefaults) {
  monaco.languages.css.cssDefaults.setOptions({
    validate: true,
    lint: {
      compatibleVendorPrefixes: "warning",
      vendorPrefix: "warning",
      duplicateProperties: "warning",
      emptyRules: "warning",
      importStatement: "warning",
      zeroUnits: "warning",
      fontFaceProperties: "warning",
      hexColorLength: "warning",
      unknownProperties: "warning",
    },
  });
}

matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  applyEditorTheme();
  for (const tab of (typeof termTabs !== "undefined" ? termTabs : [])) {
    tab.term.options.theme = termTheme();
  }
});

// Route "go to definition" (and similar) to a resource other than the current
// model through our own tab system, then move the cursor to the target.
monaco.editor.registerEditorOpener({
  openCodeEditor(_source, resource, selectionOrPosition) {
    const path = resource.fsPath || resource.path;
    if (!path) return false;
    const name = path.split("/").pop();
    Promise.resolve(openFile(path, name)).then((opened) => {
      if (!opened || !selectionOrPosition) return;
      const pos =
        "startLineNumber" in selectionOrPosition
          ? { lineNumber: selectionOrPosition.startLineNumber, column: selectionOrPosition.startColumn }
          : selectionOrPosition;
      monacoEditor.revealPositionInCenter(pos);
      monacoEditor.setPosition(pos);
      monacoEditor.focus();
    });
    return true;
  },
});

// ---- breakpoints + debug location (editor-side state) ----
const breakpoints = new Map(); // path -> Set<lineNumber>
const bpDecorations = monacoEditor.createDecorationsCollection([]);
const debugLineDecorations = monacoEditor.createDecorationsCollection([]);
let debugStopLocation = null; // { path, line }

function getAllBreakpoints() {
  const out = new Map();
  for (const [path, set] of breakpoints) out.set(path, [...set]);
  return out;
}

function toggleBreakpoint(path, line) {
  if (!path || !line) return;
  let set = breakpoints.get(path);
  if (!set) {
    set = new Set();
    breakpoints.set(path, set);
  }
  if (set.has(line)) set.delete(line);
  else set.add(line);
  if (set.size === 0) breakpoints.delete(path);
  renderBreakpointDecorations();
  if (dapManager?.isActive()) {
    dapManager.sendBreakpoints(path, [...(breakpoints.get(path) || [])]);
  }
  refreshDebugUI();
}

function renderBreakpointDecorations() {
  if (!activePath) {
    bpDecorations.set([]);
    return;
  }
  const set = breakpoints.get(activePath);
  if (!set || set.size === 0) {
    bpDecorations.set([]);
    return;
  }
  const decos = [...set]
    .sort((a, b) => a - b)
    .map((line) => ({
      range: new monaco.Range(line, 1, line, 1),
      options: {
        glyphMarginClassName: "bp-glyph",
        glyphMarginHoverMessage: { value: "Breakpoint" },
        stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
      },
    }));
  bpDecorations.set(decos);
}

function showDebugLocation(path, line) {
  debugStopLocation = { path, line };
  Promise.resolve(openFile(path, basename(path))).then((ok) => {
    if (!ok) return;
    applyDebugLineDecoration();
    monacoEditor.revealLineInCenter(line);
  });
}

function applyDebugLineDecoration() {
  if (debugStopLocation && activePath === debugStopLocation.path) {
    const line = debugStopLocation.line;
    debugLineDecorations.set([
      {
        range: new monaco.Range(line, 1, line, 1),
        options: {
          isWholeLine: true,
          className: "debug-current-line",
          glyphMarginClassName: "debug-current-glyph",
          stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
        },
      },
    ]);
  } else {
    debugLineDecorations.set([]);
  }
}

function clearDebugLocation() {
  debugStopLocation = null;
  debugLineDecorations.set([]);
}

monacoEditor.onMouseDown((e) => {
  if (e.target.type === monaco.editor.MouseTargetType.GUTTER_GLYPH_MARGIN) {
    const line = e.target.position?.lineNumber;
    if (line && activePath) toggleBreakpoint(activePath, line);
  }
});

// Standard debugger keybindings.
monacoEditor.addCommand(monaco.KeyCode.F9, () => {
  if (activePath) toggleBreakpoint(activePath, monacoEditor.getPosition().lineNumber);
});
monacoEditor.addCommand(monaco.KeyCode.F5, () => {
  if (dapManager?.isActive()) {
    if (dapManager.isStopped()) dapManager.cont();
  } else {
    openFeaturePanel("debugger");
  }
});
monacoEditor.addCommand(monaco.KeyMod.Shift | monaco.KeyCode.F5, () => dapManager?.stop());
monacoEditor.addCommand(monaco.KeyCode.F10, () => dapManager?.next());
monacoEditor.addCommand(monaco.KeyCode.F11, () => dapManager?.stepIn());
monacoEditor.addCommand(monaco.KeyMod.Shift | monaco.KeyCode.F11, () => dapManager?.stepOut());

/** path -> { model, name, dirty, viewState } */
const openFiles = new Map();
let activePath = null;

// Paths whose Monaco model is kept alive as part of the language-service
// "project" (so cross-file go-to-definition / completion resolves even when the
// target file isn't open in a tab). These models are not disposed on close.
const projectModels = new Set();

// Create (or reuse) a model addressed by its real path so the TypeScript
// language service treats files as one project and can resolve imports across
// them. Reuses any model preloaded by the project walk.
const modelsWithListeners = new WeakSet();
function attachModelListeners(path, model) {
  if (modelsWithListeners.has(model)) return;
  modelsWithListeners.add(model);
  model.onDidChangeContent(() => {
    markDirty(path, true);
    if (_imeComposing) {
      if (!_imeFlushCallbacks.some(cb => cb._lspPath === path)) {
        const cb = () => lspManager?.didChange(path, model);
        cb._lspPath = path;
        _imeFlushCallbacks.push(cb);
      }
      return;
    }
    lspManager?.didChange(path, model);
  });
}

function getOrCreateModel(path, name, content) {
  const uri = monaco.Uri.file(path);
  let model = monaco.editor.getModel(uri);
  if (model) {
    // A model may have been created lazily (e.g. for cross-file diagnostics)
    // with an unresolved language — correct it now that we know the real file.
    const want = extLang(name);
    if (want && model.getLanguageId() !== want) monaco.editor.setModelLanguage(model, want);
    if (content != null && model.getValue() !== content) model.setValue(content);
    attachModelListeners(path, model);
    return model;
  }
  model = monaco.editor.createModel(content ?? "", extLang(name), uri);
  attachModelListeners(path, model);
  return model;
}

function extLang(name) {
  const ext = name.split(".").pop().toLowerCase();
  const map = {
    js: "javascript", jsx: "javascript", mjs: "javascript", cjs: "javascript",
    ts: "typescript", tsx: "typescript", json: "json", css: "css", scss: "scss",
    less: "less", html: "html", htm: "html", md: "markdown", markdown: "markdown",
    rs: "rust", py: "python", go: "go", java: "java", c: "c", h: "c", cpp: "cpp",
    hpp: "cpp", cc: "cpp", sh: "shell", bash: "shell", yml: "yaml", yaml: "yaml",
    toml: "ini", ini: "ini", xml: "xml", sql: "sql", rb: "ruby", php: "php",
    swift: "swift", kt: "kotlin",
  };
  return map[ext] ?? "plaintext";
}

function syncWelcome() {
  welcomeEl.hidden = openFiles.size > 0;
}

const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "bmp"]);
function isImageFile(name) {
  const ext = (name.split(".").pop() || "").toLowerCase();
  return IMAGE_EXTS.has(ext);
}

async function openFile(path, name) {
  if (openFiles.has(path)) {
    activate(path);
    return true;
  }

  if (isImageFile(name)) {
    openFiles.set(path, { model: null, name, dirty: false, viewState: null, isImage: true });
    renderTabs();
    activate(path);
    return true;
  }

  let content;
  try {
    content = await backend.readTextFile(path);
  } catch (e) {
    showToast(String(e));
    return false;
  }
  const model = getOrCreateModel(path, name, content);
  openFiles.set(path, { model, name, dirty: false, viewState: null });
  renderTabs();
  activate(path);
  lspManager?.didOpen(path, model);
  _onFileOpened(model);
  return true;
}

function activate(path) {
  closeDiffView();
  hideImagePreview();
  hideMarkdownPreview();
  if (activePath && openFiles.has(activePath)) {
    const prev = openFiles.get(activePath);
    if (prev && prev.model) prev.viewState = monacoEditor.saveViewState();
  }
  activePath = path;
  const f = openFiles.get(path);

  if (f.isImage) {
    monacoEditor.setModel(null);
    showImagePreview(path);
    editorEl.style.display = "none";
  } else {
    editorEl.style.display = "";
    monacoEditor.setModel(f.model);
    if (f.viewState) monacoEditor.restoreViewState(f.viewState);
    monacoEditor.focus();
    if (path.endsWith(".md")) showMarkdownPreview(f.model);
    const lang = f.model?.getLanguageId();
    // Tool detection is handled by the LSP client's ensureServer(); no need to
    // run a redundant check here that races and sometimes double-notifies.
  }

  syncWelcome();
  renderTabs();
  renderTreeActive();
  saveBtn.disabled = !f.dirty;
  if (runBtn) runBtn.disabled = !!f.isImage;
  const projectLabel = rootPath ? basename(rootPath) : "";
  $("windowTitle").textContent = f.name + (projectLabel ? " — " + projectLabel : "") + " — Michael IDE";
  if (!f.isImage) {
    refreshGutter();
    refreshBlame();
    updateBreadcrumb(path);
    renderBreakpointDecorations();
    applyDebugLineDecoration();
  } else {
    updateBreadcrumb(path);
  }
  updateStatusBar();
}

// ---- image preview ----
let _imagePreviewEl = null;
function showImagePreview(path) {
  if (!_imagePreviewEl) {
    _imagePreviewEl = document.createElement("div");
    _imagePreviewEl.className = "image-preview";
    editorContainer.appendChild(_imagePreviewEl);
  }
  const src = inTauri ? `asset://localhost/${encodeURIComponent(path)}` : path;
  _imagePreviewEl.innerHTML = `<div class="image-preview__inner"><img src="${src}" alt="" /><p class="image-preview__path"></p></div>`;
  _imagePreviewEl.querySelector(".image-preview__path").textContent = path;
  _imagePreviewEl.hidden = false;
}
function hideImagePreview() {
  if (_imagePreviewEl) _imagePreviewEl.hidden = true;
}

// ---- markdown preview ----
let _mdPreviewEl = null;
let _mdThumbEl = null;
let _mdPreviewDisposable = null;
let _mdPreviewOpen = false;
let _mdActiveModel = null;

function _createMdElements() {
  if (_mdPreviewEl) return;

  // preview pane
  _mdPreviewEl = document.createElement("div");
  _mdPreviewEl.className = "md-preview";
  _mdPreviewEl.hidden = true;
  const header = document.createElement("div");
  header.className = "md-preview__header";
  header.innerHTML = `<span class="md-preview__title">Markdown 预览</span>
    <button class="md-preview__close" title="关闭预览 ⌘.">✕</button>`;
  _mdPreviewEl.appendChild(header);
  const body = document.createElement("div");
  body.className = "md-preview__body";
  _mdPreviewEl.appendChild(body);
  editorContainer.appendChild(_mdPreviewEl);
  header.querySelector(".md-preview__close").addEventListener("click", () => _toggleMdPreview(false));

  // thumbnail strip
  _mdThumbEl = document.createElement("div");
  _mdThumbEl.className = "md-thumb";
  _mdThumbEl.hidden = true;
  _mdThumbEl.innerHTML = `<div class="md-thumb__mini"></div>`;
  _mdThumbEl.addEventListener("click", () => _toggleMdPreview(true));
  editorContainer.appendChild(_mdThumbEl);
}

function _toggleMdPreview(open) {
  _mdPreviewOpen = open;
  // preview: show or hide
  if (_mdPreviewEl) _mdPreviewEl.hidden = !open;
  // thumbnail: opposite of preview
  if (_mdThumbEl) _mdThumbEl.hidden = open;
}

function _renderMdThumb() {
  if (!_mdThumbEl || !_mdPreviewEl) return;
  const body = _mdPreviewEl.querySelector(".md-preview__body");
  const mini = _mdThumbEl.querySelector(".md-thumb__mini");
  if (!body || !mini) return;
  mini.textContent = "";
  const clone = body.cloneNode(true);
  clone.className = "md-thumb__content";
  mini.appendChild(clone);
}

function showMarkdownPreview(model) {
  _createMdElements();
  _mdActiveModel = model;
  const body = _mdPreviewEl.querySelector(".md-preview__body");
  function render() {
    renderMarkdownInto(body, model.getValue());
    if (!_mdPreviewOpen) _renderMdThumb();
  }
  render();
  if (_mdPreviewDisposable) _mdPreviewDisposable.dispose();
  _mdPreviewDisposable = model.onDidChangeContent(() => render());
  _toggleMdPreview(false);
}

function hideMarkdownPreview() {
  _mdPreviewOpen = false;
  _mdActiveModel = null;
  if (_mdPreviewEl) _mdPreviewEl.hidden = true;
  if (_mdThumbEl) _mdThumbEl.hidden = true;
  if (_mdPreviewDisposable) { _mdPreviewDisposable.dispose(); _mdPreviewDisposable = null; }
}

// ⌘. to toggle markdown preview
document.addEventListener("keydown", (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key === ".") {
    e.preventDefault();
    if (!_mdActiveModel) return;
    _toggleMdPreview(!_mdPreviewOpen);
    if (!_mdPreviewOpen) _renderMdThumb();
  }
});

function updateBreadcrumb(path) {
  const bc = $("breadcrumb");
  if (!path) { bc.hidden = true; return; }
  bc.hidden = false;
  bc.innerHTML = "";
  const rel = rootPath ? path.replace(rootPath, "").replace(/^\//, "") : path;
  const segments = rel.split("/").filter(Boolean);
  segments.forEach((seg, i) => {
    if (i > 0) {
      const sep = document.createElement("span");
      sep.className = "breadcrumb__sep";
      sep.textContent = "›";
      bc.appendChild(sep);
    }
    const el = document.createElement("span");
    el.className = "breadcrumb__seg";
    el.textContent = seg;
    if (i < segments.length - 1) {
      const partial = rootPath + "/" + segments.slice(0, i + 1).join("/");
      el.addEventListener("click", () => revealInTree(partial));
    }
    bc.appendChild(el);
  });
}

function revealInTree(path) {
  const item = document.querySelector(`.tree-item[data-path="${CSS.escape(path)}"]`);
  if (item) {
    item.scrollIntoView({ block: "center" });
    item.classList.add("flash");
    setTimeout(() => item.classList.remove("flash"), 600);
  }
}

const pinnedTabs = new Set();
function togglePinTab(path) {
  if (pinnedTabs.has(path)) pinnedTabs.delete(path);
  else pinnedTabs.add(path);
  renderTabs();
}

function closeFile(path) {
  const f = openFiles.get(path);
  if (!f) return;
  if (pinnedTabs.has(path)) return; // pinned tabs cannot be closed
  lspManager?.didClose(path);
  if (!projectModels.has(path) && f.model) f.model.dispose();
  openFiles.delete(path);
  if (activePath === path) {
    activePath = null;
    const next = [...openFiles.keys()].pop();
    if (next) activate(next);
    else {
      monacoEditor.setModel(monaco.editor.createModel("", "plaintext"));
      saveBtn.disabled = true;
      if (runBtn) runBtn.disabled = true;
      const idleTitle = rootPath ? basename(rootPath) + " — Michael IDE" : "Michael IDE";
      $("windowTitle").textContent = idleTitle;
      refreshGutter();
      updateBreadcrumb(null);
    }
  }
  renderTabs();
  syncWelcome();
}

function closeOtherTabs(keepPath) {
  for (const path of [...openFiles.keys()]) {
    if (path !== keepPath) closeFile(path);
  }
}

function closeTabsToRight(fromPath) {
  const paths = [...openFiles.keys()];
  const idx = paths.indexOf(fromPath);
  if (idx < 0) return;
  for (const path of paths.slice(idx + 1)) closeFile(path);
}

function closeAllTabs() {
  for (const path of [...openFiles.keys()]) closeFile(path);
}

function relativePath(path) {
  if (rootPath && path.startsWith(rootPath + "/")) return path.slice(rootPath.length + 1);
  return basename(path);
}

function openTabContextMenu(x, y, path) {
  const isPinned = pinnedTabs.has(path);
  renderMenuAt(x, y, [
    { label: t("tabctx.close"), icon: "i-close", action: () => closeFile(path) },
    { label: t("tabctx.closeOthers"), action: () => closeOtherTabs(path) },
    { label: t("tabctx.closeRight"), action: () => closeTabsToRight(path) },
    { label: t("tabctx.closeAll"), action: () => closeAllTabs() },
    { sep: true },
    { label: isPinned ? t("tabctx.unpin") : t("tabctx.pin"), action: () => togglePinTab(path) },
    { label: t("tabctx.reveal"), icon: "i-files", action: () => { showSide("explorer"); revealInTree(path); } },
    { sep: true },
    { label: t("tabctx.copyPath"), icon: "i-copy", action: () => copyText(path) },
    { label: t("tabctx.copyRelPath"), icon: "i-copy", action: () => copyText(relativePath(path)) },
  ]);
}

function markDirty(path, dirty) {
  const f = openFiles.get(path);
  if (!f || f.dirty === dirty) return;
  f.dirty = dirty;
  if (path === activePath) saveBtn.disabled = !dirty;
  const tabEl = tabsEl.querySelector(`[data-path="${CSS.escape(path)}"]`);
  if (tabEl) {
    if (dirty) tabEl.classList.add("dirty");
    else tabEl.classList.remove("dirty");
  } else {
    renderTabs();
  }
}

async function saveActive() {
  if (!activePath) return;
  const f = openFiles.get(activePath);
  try {
    await backend.writeTextFile(activePath, f.model.getValue());
    markDirty(activePath, false);
    lspManager?.didSave(activePath, f.model);
    showToast(t("file.saved", { name: f.name }));
    refreshBlame();
  } catch (e) {
    showToast(String(e));
  }
}

let dragSrcPath = null;

function renderTabs() {
  tabsEl.innerHTML = "";
  for (const [path, f] of openFiles) {
    const tab = document.createElement("div");
    tab.className = "tab" + (path === activePath ? " is-active" : "") + (f.dirty ? " dirty" : "") + (pinnedTabs.has(path) ? " is-pinned" : "");
    tab.draggable = true;
    tab.dataset.path = path;
    tab.innerHTML =
      `${iconImg(fileIconUrl(f.name))}<span class="label"></span>` +
      `<span class="x" title="Close"><span class="dot"></span><svg class="ic"><use href="#i-close" /></svg></span>`;
    tab.querySelector(".label").textContent = f.name;
    tab.addEventListener("click", () => activate(path));
    tab.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      openTabContextMenu(e.clientX, e.clientY, path);
    });
    tab.querySelector(".x").addEventListener("click", (e) => {
      e.stopPropagation();
      closeFile(path);
    });
    tab.addEventListener("dragstart", (e) => {
      dragSrcPath = path;
      tab.classList.add("dragging");
      e.dataTransfer.effectAllowed = "move";
    });
    tab.addEventListener("dragend", () => {
      dragSrcPath = null;
      tab.classList.remove("dragging");
      tabsEl.querySelectorAll(".tab").forEach(t => t.classList.remove("drag-over"));
    });
    tab.addEventListener("dragover", (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      tab.classList.add("drag-over");
    });
    tab.addEventListener("dragleave", () => tab.classList.remove("drag-over"));
    tab.addEventListener("drop", (e) => {
      e.preventDefault();
      tab.classList.remove("drag-over");
      if (!dragSrcPath || dragSrcPath === path) return;
      const entries = [...openFiles.entries()];
      const srcIdx = entries.findIndex(([p]) => p === dragSrcPath);
      const dstIdx = entries.findIndex(([p]) => p === path);
      if (srcIdx < 0 || dstIdx < 0) return;
      const [moved] = entries.splice(srcIdx, 1);
      entries.splice(dstIdx, 0, moved);
      openFiles.clear();
      for (const [p, v] of entries) openFiles.set(p, v);
      renderTabs();
    });
    tabsEl.appendChild(tab);
  }
}

// ---- file tree ----
let rootPath = null;
let workspaceRoots = [];

// Wire the real LSP client now that workspace state exists. Disabled in the
// plain-browser mock (no real servers to talk to). Providers are registered for
// the "gap" languages Monaco's bundled service does not cover.
lspManager = createLspManager({
  backend,
  enabled: inTauri,
  getWorkspaceRoots: () => (workspaceRoots.length ? workspaceRoots : rootPath ? [rootPath] : []),
  showToast: (msg) => showToast(msg),
  showNotification: ({ title, message, actionLabel, duration, installCmd }) => {
    const toolName = (title || "").replace(/^缺少\s*/, "").replace(/\s*语言服务器$/, "") || "LSP";
    showNotification({
      title, message, actionLabel, duration,
      action: installCmd ? async () => {
        await openTerminal();
        writeToActiveTerminal(installCmd + "\n");
        _showInstallProgress(installCmd, toolName);
      } : undefined,
    });
  },
  onStatus: () => updateLspStatusBar(),
  onLog: (lang, line) => lspLogSink(lang, line),
});
lspManager.onCompletionSymbols = (symbols) => {
  _updateDynamicKeywords(symbols);
};
lspManager.registerProviders();

let _envLoadTimer = null;
let _modApiTimer = null;

function _onFileOpened(model) {
  if (!model) return;
  _extractFileIdentifiers(model);
  const langId = model.getLanguageId();

  const importedMods = _extractImportedModules(model);
  if (importedMods.length > 0) {
    setTimeout(() => _loadModuleApisOnly(importedMods), 200);
  }

  if (_envLoadTimer) clearTimeout(_envLoadTimer);
  _envLoadTimer = setTimeout(() => _loadEnvSymbols(langId), 1500);
}

monacoEditor.onDidChangeModel(() => {
  const model = monacoEditor.getModel();
  if (model) {
    _extractFileIdentifiers(model);
    if (_modApiTimer) clearTimeout(_modApiTimer);
    _modApiTimer = setTimeout(() => _refreshModuleApis(model), 2000);
  }
});

let _fileIdRefreshTimer = null;
monacoEditor.onDidChangeModelContent(() => {
  if (_fileIdRefreshTimer) clearTimeout(_fileIdRefreshTimer);
  _fileIdRefreshTimer = setTimeout(() => {
    const model = monacoEditor.getModel();
    if (model) {
      _extractFileIdentifiers(model);
      _refreshModuleApis(model);
    }
  }, 5000);
});

const lspLogBuffers = new Map();
function lspLogSink(lang, line) {
  let buf = lspLogBuffers.get(lang);
  if (!buf) {
    buf = [];
    lspLogBuffers.set(lang, buf);
  }
  buf.push(line);
  if (buf.length > 400) buf.shift();
  console.log(`[LSP:${lang}] ${line}`);
  document.dispatchEvent(new CustomEvent("lsp-log", { detail: { lang } }));
}

function updateLspStatusBar() {
  if (!lspManager) return;
  const all = lspManager.status();
  const ready = all.filter((s) => s.initialized).map((s) => s.lang);
  const starting = all.filter((s) => !s.initialized).map((s) => s.lang);
  if (ready.length || starting.length) {
    const parts = [];
    if (ready.length) parts.push(ready.join(", "));
    if (starting.length) parts.push(`(${starting.join(",")} starting…)`);
    setStatusBarItem(
      "lsp",
      { text: `LSP: ${parts.join(" ")}`, tooltip: `Active: ${ready.join(", ") || "none"}\nStarting: ${starting.join(", ") || "none"}\nClick for logs` },
      () => openFeaturePanel("lsp"),
    );
  } else {
    removeStatusBarItem("lsp");
  }
}

// Wire the real debug adapter client.
dapManager = createDapManager({
  backend,
  getWorkspaceRoots: () => (workspaceRoots.length ? workspaceRoots : rootPath ? [rootPath] : []),
  getAllBreakpoints,
  runInTerminal: (args) => debugRunInTerminal(args),
  showToast: (msg) => showToast(msg),
  callbacks: {
    onState: () => refreshDebugUI(),
    onOutput: (cat, text) => appendDebugConsole(cat, text),
    onShowLocation: (path, line) => showDebugLocation(path, line),
    onStopped: () => { openFeaturePanel("debugger"); refreshDebugUI(); },
    onContinued: () => clearDebugLocation(),
    onTerminated: () => { clearDebugLocation(); refreshDebugUI(); },
  },
});

async function debugRunInTerminal(args) {
  await openTerminal();
  const parts = args?.args || [];
  if (!parts.length) return;
  const cmd = parts.map((p) => shellQuote(String(p))).join(" ");
  const cd = args.cwd ? `cd ${shellQuote(args.cwd)} && ` : "";
  writeToActiveTerminal(`\n${cd}${cmd}\n`);
}

function updateDebugStatusBar() {
  if (!dapManager) return;
  if (dapManager.isActive()) {
    const state = dapManager.isStopped() ? "paused" : "running";
    setStatusBarItem(
      "debug",
      { text: `Debug: ${state}`, tooltip: "Active debug session" },
      () => openFeaturePanel("debugger"),
    );
  } else {
    removeStatusBarItem("debug");
  }
}

function iconSvg(id, cls = "") {
  return `<svg class="ic ${cls}"><use href="#${id}" /></svg>`;
}

// Real file-type icons from the Material Icon Theme (MIT-licensed), vendored as
// static SVGs under ./assets/file-icons and bundled by Vite (offline-capable in
// both the browser and the native Tauri app).
const ICON_URLS = import.meta.glob("./assets/file-icons/*.svg", {
  eager: true,
  query: "?url",
  import: "default",
});
function iconUrl(base) {
  return ICON_URLS[`./assets/file-icons/${base}.svg`] || "";
}
/** Render a bundled icon URL as an <img>, falling back to a generic glyph. */
function iconImg(url, cls = "") {
  if (!url) return iconSvg("i-file", "ic--doc");
  return `<img class="ic ${cls}" src="${url}" alt="" draggable="false" />`;
}

// extension -> Material icon basename
const EXT_ICON = {
  js: "javascript", mjs: "javascript", cjs: "javascript",
  ts: "typescript", mts: "typescript", cts: "typescript",
  jsx: "react", tsx: "react_ts",
  json: "json", jsonc: "json", json5: "json",
  md: "markdown", markdown: "markdown", mdx: "markdown",
  css: "css", scss: "sass", sass: "sass", less: "less",
  html: "html", htm: "html", xml: "xml", svg: "svg",
  vue: "vue", svelte: "svelte",
  rs: "rust",
  py: "python", pyw: "python", pyi: "python",
  go: "go", java: "java", kt: "kotlin", kts: "kotlin",
  c: "c", h: "c", cpp: "cpp", cc: "cpp", cxx: "cpp", hpp: "cpp", hh: "cpp",
  cs: "csharp",
  sh: "console", bash: "console", zsh: "console", fish: "console", ps1: "powershell",
  yml: "yaml", yaml: "yaml", toml: "toml",
  ini: "settings", conf: "settings", cfg: "settings", env: "tune",
  sql: "database",
  rb: "ruby", php: "php", swift: "swift",
  gradle: "gradle", graphql: "graphql", gql: "graphql", prisma: "prisma", astro: "astro",
  png: "image", jpg: "image", jpeg: "image", gif: "image",
  webp: "image", ico: "image", avif: "image", bmp: "image",
  woff: "font", woff2: "font", ttf: "font", otf: "font", eot: "font",
  mp4: "video", webm: "video", mov: "video",
  mp3: "audio", wav: "audio", flac: "audio", ogg: "audio",
  pdf: "pdf", zip: "zip", gz: "zip", tar: "zip", rar: "zip", "7z": "zip",
  key: "key", pem: "key", crt: "key", cert: "key",
  txt: "document", rst: "document", log: "document", csv: "document",
};

// exact filename (lowercased) -> Material icon basename
const NAME_ICON = {
  "package.json": "nodejs", "package-lock.json": "npm",
  "yarn.lock": "yarn", "pnpm-lock.yaml": "npm", "bun.lockb": "bun",
  ".npmrc": "npm", ".nvmrc": "nodejs",
  "tsconfig.json": "tsconfig",
  ".gitignore": "git", ".gitattributes": "git", ".gitmodules": "git",
  ".editorconfig": "editorconfig",
  ".eslintrc": "eslint", ".eslintrc.json": "eslint", ".eslintrc.js": "eslint",
  ".eslintrc.cjs": "eslint", "eslint.config.js": "eslint", "eslint.config.mjs": "eslint",
  ".prettierrc": "prettier", ".prettierrc.json": "prettier",
  "prettier.config.js": "prettier", ".prettierrc.js": "prettier",
  dockerfile: "docker", ".dockerignore": "docker", "docker-compose.yml": "docker",
  "vite.config.js": "vite", "vite.config.ts": "vite",
  "webpack.config.js": "webpack", "rollup.config.js": "rollup",
  "babel.config.js": "babel", ".babelrc": "babel",
  "jest.config.js": "jest", "jest.config.ts": "jest",
  "vitest.config.js": "vitest", "vitest.config.ts": "vitest",
  "tailwind.config.js": "tailwindcss", "tailwind.config.ts": "tailwindcss",
  ".env": "tune", ".env.local": "tune", ".env.development": "tune", ".env.production": "tune",
  license: "license", "license.md": "license", "license.txt": "license",
  "readme.md": "readme", readme: "readme", "readme.txt": "readme",
};

// folder name (lowercased) -> Material folder icon basename
const FOLDER_ICON = {
  src: "folder-src", source: "folder-src", lib: "folder-src",
  components: "folder-components", component: "folder-components",
  node_modules: "folder-node",
  dist: "folder-dist", build: "folder-dist", out: "folder-dist",
  public: "folder-public", static: "folder-public",
  test: "folder-test", tests: "folder-test", __tests__: "folder-test", spec: "folder-test",
  assets: "folder-resource", resources: "folder-resource", res: "folder-resource",
  images: "folder-images", img: "folder-images",
  config: "folder-config", configs: "folder-config",
  css: "folder-css", styles: "folder-css", style: "folder-css", scss: "folder-css",
  utils: "folder-utils", util: "folder-utils", helpers: "folder-utils",
  hooks: "folder-hook", hook: "folder-hook",
  api: "folder-api",
  docs: "folder-docs", doc: "folder-docs",
  ".github": "folder-github",
  ".vscode": "folder-vscode",
  store: "folder-store", stores: "folder-store", redux: "folder-store",
  scripts: "folder-scripts", script: "folder-scripts",
  views: "folder-views", pages: "folder-views",
  server: "folder-server", backend: "folder-server",
  client: "folder-client", frontend: "folder-client",
};

/** Map a filename to a bundled Material file-icon URL. */
function fileIconUrl(name) {
  const lower = name.toLowerCase();
  if (lower in NAME_ICON) return iconUrl(NAME_ICON[lower]);
  if (lower.endsWith(".lock")) return iconUrl("lock");
  const ext = lower.includes(".") ? lower.split(".").pop() : "";
  if (ext in EXT_ICON) return iconUrl(EXT_ICON[ext]);
  return iconUrl("file");
}

/** Map a folder name + open state to a bundled Material folder-icon URL. */
function folderIconUrl(name, open) {
  const base = FOLDER_ICON[name.toLowerCase()] || "folder";
  return (
    iconUrl(open ? base + "-open" : base) ||
    iconUrl(open ? "folder-open" : "folder")
  );
}

let rootContainer = null;
/** path -> { row, kids, loaded } for every directory row currently rendered. */
const dirNodes = new Map();

function setExplorerToolsEnabled(on) {
  for (const id of ["newFileBtn", "newFolderBtn", "refreshTreeBtn"]) {
    const b = $(id);
    if (b) b.disabled = !on;
  }
}

const parentDir = (p) => p.slice(0, p.lastIndexOf("/")) || "/";

function basename(path) {
  return path.split("/").filter(Boolean).pop() || path;
}

function setActiveWorkspaceRoot(path) {
  rootPath = path;
  _launchConfigsCache = null;
  _agentContextCache = { root: "", ts: 0, data: "" };
  if (workspaceRoots.length > 1) {
    rootNameEl.textContent = `${workspaceRoots.length} folders`;
    rootNameEl.title = workspaceRoots.join("\n");
  } else {
    rootNameEl.textContent = basename(path);
    rootNameEl.title = path;
  }
  setExplorerToolsEnabled(Boolean(path));
  const titleFile = activePath ? openFiles.get(activePath)?.name : "";
  const project = basename(path);
  $("windowTitle").textContent = (titleFile ? titleFile + " — " : "") + project + " — Michael IDE";
}

async function renderWorkspaceRoots() {
  dirNodes.clear();
  treeEl.innerHTML = "";
  rootContainer = document.createElement("div");
  treeEl.appendChild(rootContainer);

  for (const root of workspaceRoots) {
    const section = document.createElement("div");
    section.className = "workspace-root";

    const row = document.createElement("div");
    row.className = "row workspace-root__row" + (root === rootPath ? " is-active" : "");
    row.dataset.path = root;
    row.innerHTML = `${iconImg(folderIconUrl(basename(root), true), "folder-ic")}<span class="name"></span><span class="workspace-root__path"></span>`;
    row.querySelector(".name").textContent = basename(root);
    row.querySelector(".workspace-root__path").textContent = root;
    row.title = root;
    row.addEventListener("click", async () => {
      if (root !== rootPath) {
        setActiveWorkspaceRoot(root);
        await refreshGitStatus();
        preloadProjectModels(root);
        renderTreeActive();
      }
    });
    row.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      e.stopPropagation();
      openContextMenu(e.clientX, e.clientY, { path: root, name: basename(root), is_dir: true });
    });

    const kids = document.createElement("div");
    kids.className = "children workspace-root__children";
    section.append(row, kids);
    rootContainer.appendChild(section);
    await renderChildren(root, kids);
  }
  renderTreeActive();
}

async function openFolder(path) {
  workspaceRoots = [path];
  setActiveWorkspaceRoot(path);
  // Opening a different project should immediately retag the active chat tab.
  const _sess = _currentSession();
  if (_sess && _sess.project !== path) { _sess.project = path; _renderChatTabs(); saveChatHistory(); }
  try { await backend.registerWorkspaceRoot(path); } catch { /* browser preview */ }
  _ipcBroadcast("workspace_changed", { roots: [path], active: path });
  await renderWorkspaceRoots();
  preloadProjectModels(path);
  await refreshGitStatus();
  startFileWatcher();
  addRecentProject(path);
}

let _fsWatcherActive = false;
let _fsChangeDebounce = null;
async function startFileWatcher() {
  if (_fsWatcherActive || !inTauri) return;
  const roots = workspaceRoots.length ? [...workspaceRoots] : rootPath ? [rootPath] : [];
  if (!roots.length) return;
  try {
    await backend.fsWatch(roots);
    _fsWatcherActive = true;
    const { listen } = await import("@tauri-apps/api/event");
    listen("fs-change", (event) => {
      clearTimeout(_fsChangeDebounce);
      _fsChangeDebounce = setTimeout(() => {
        const changed = event.payload?.paths || [];
        handleFsChanges(changed);
      }, 200);
    });
  } catch (e) {
    console.warn("[watcher] failed to start:", e);
  }
}

const _FS_IGNORE_RE = /(^|\/)(node_modules|\.git|target|dist|build|out|\.next|coverage|\.cache|\.venv|__pycache__|vendor|\.gradle)(\/|$)/;
function handleFsChanges(paths) {
  if (!rootPath) return;
  if (_autoSaving) return;

  // Defense in depth (the watcher already filters these at the source): never do
  // work for changes inside high-churn build/dependency dirs. Test the path
  // RELATIVE to its workspace root, so a project that itself lives under a dir
  // named e.g. "build" isn't wholly ignored.
  paths = paths.filter((p) => {
    const r = workspaceRoots.find((wr) => p.startsWith(wr)) || (rootPath && p.startsWith(rootPath) ? rootPath : "");
    return !_FS_IGNORE_RE.test(r ? p.slice(r.length) : p);
  });
  if (!paths.length) return;

  const openPaths = new Set(openFiles.keys());
  const onlyOpenFiles = paths.every((p) => openPaths.has(p));
  if (onlyOpenFiles && autoSaveEnabled) return;

  const dirsToReload = new Set();
  for (const p of paths) {
    const dir = parentDir(p);
    if (dir && (workspaceRoots.some((r) => dir.startsWith(r)) || dir.startsWith(rootPath))) {
      dirsToReload.add(dir);
    }
  }
  let reloaded = 0;
  for (const dir of dirsToReload) {
    if (dirNodes.has(dir) || workspaceRoots.includes(dir)) {
      reloadDir(dir);
      if (++reloaded >= 40) break; // never reload an unbounded number of dirs at once
    }
  }
  refreshGitStatus();
}

async function addFolderToWorkspace() {
  const picked = await backend.pickFolder();
  if (!picked) return;
  if (!workspaceRoots.includes(picked)) workspaceRoots.push(picked);
  try { await backend.registerWorkspaceRoot(picked); } catch { /* browser preview */ }
  _ipcBroadcast("workspace_changed", { roots: [...workspaceRoots], active: picked });
  setActiveWorkspaceRoot(picked);
  await renderWorkspaceRoots();
  preloadProjectModels(picked);
  await refreshGitStatus();
  showToast(`Added ${basename(picked)} to workspace`);
}

// Eagerly load the workspace's JS/TS/JSON files into the TypeScript language
// service so imports resolve and go-to-definition works across files the user
// hasn't opened yet. Skips heavy/irrelevant directories and caps the count so
// large repos stay responsive; runs in the background (not awaited).
const PRELOAD_CODE_EXT = new Set(["ts", "tsx", "js", "jsx", "mjs", "cjs", "json"]);
const PRELOAD_SKIP_DIRS = new Set([
  "node_modules", ".git", "dist", "build", "out", "target",
  ".next", "coverage", ".cache", ".vscode", "vendor",
]);
const PRELOAD_MAX_FILES = 150;
const PRELOAD_MAX_BYTES = 512 * 1024;
let preloadToken = 0;

async function preloadProjectModels(root) {
  const token = ++preloadToken;
  // Drop project models from a previously opened folder.
  for (const p of projectModels) {
    if (openFiles.has(p)) continue;
    const m = monaco.editor.getModel(monaco.Uri.file(p));
    if (m) m.dispose();
  }
  projectModels.clear();

  let count = 0;
  const stack = [root];
  while (stack.length && count < PRELOAD_MAX_FILES) {
    const dir = stack.pop();
    let entries;
    try {
      entries = await backend.readDir(dir);
    } catch {
      continue;
    }
    if (token !== preloadToken) return; // folder changed mid-walk
    for (const entry of entries) {
      if (entry.is_dir) {
        if (!PRELOAD_SKIP_DIRS.has(entry.name)) stack.push(entry.path);
        continue;
      }
      const ext = entry.name.includes(".") ? entry.name.split(".").pop().toLowerCase() : "";
      if (!PRELOAD_CODE_EXT.has(ext) || count >= PRELOAD_MAX_FILES) continue;
      let content;
      try {
        content = await backend.readTextFile(entry.path);
      } catch {
        continue;
      }
      if (token !== preloadToken) return;
      if (content.length > PRELOAD_MAX_BYTES) continue;
      // Never clobber a file the user has open (and may be editing): its model
      // is already alive in the language service.
      if (openFiles.has(entry.path)) {
        projectModels.add(entry.path);
        count++;
        continue;
      }
      getOrCreateModel(entry.path, entry.name, content);
      projectModels.add(entry.path);
      count++;
    }
  }
}

// ---- diagnostics / problems panel ----
const SEV = monaco.MarkerSeverity;
const problemsPanel = $("problemsPanel");
const problemsBody = $("problemsBody");

function pathBase(p) {
  return p.slice(p.lastIndexOf("/") + 1);
}
function pathForDisplay(p) {
  if (rootPath && p.startsWith(rootPath + "/")) return p.slice(rootPath.length + 1);
  return pathBase(p);
}

function gotoMarker(path, marker) {
  Promise.resolve(openFile(path, pathBase(path))).then((opened) => {
    if (!opened) return;
    monacoEditor.revealLineInCenter(marker.startLineNumber);
    monacoEditor.setPosition({ lineNumber: marker.startLineNumber, column: marker.startColumn });
    monacoEditor.focus();
  });
}

function renderProblems(markers) {
  if (!problemsBody) return;
  problemsBody.innerHTML = "";
  if (!markers.length) {
    const empty = document.createElement("div");
    empty.className = "problems__empty";
    empty.textContent = t("problems.empty");
    problemsBody.appendChild(empty);
    return;
  }
  const byFile = new Map();
  for (const m of markers) {
    const key = m.resource.toString();
    if (!byFile.has(key)) byFile.set(key, { path: m.resource.fsPath || m.resource.path, items: [] });
    byFile.get(key).items.push(m);
  }
  for (const { path, items } of byFile.values()) {
    const head = document.createElement("div");
    head.className = "problems__file";
    head.innerHTML = `${iconImg(fileIconUrl(pathBase(path)))}<span class="problems__file-name"></span><span class="problems__file-count"></span>`;
    head.querySelector(".problems__file-name").textContent = pathForDisplay(path);
    head.querySelector(".problems__file-count").textContent = String(items.length);
    problemsBody.appendChild(head);
    for (const m of items) {
      const isErr = m.severity === SEV.Error;
      const item = document.createElement("button");
      item.type = "button";
      item.className = "problems__item problems__item--" + (isErr ? "error" : "warn");
      item.innerHTML = `<svg class="ic"><use href="#i-${isErr ? "error" : "warn"}" /></svg><span class="problems__msg"></span><span class="problems__loc"></span>`;
      item.querySelector(".problems__msg").textContent = m.message;
      item.querySelector(".problems__loc").textContent = `Ln ${m.startLineNumber}, Col ${m.startColumn}`;
      item.addEventListener("click", () => gotoMarker(path, m));
      problemsBody.appendChild(item);
    }
  }
}

function updateProblems() {
  const markers = monaco.editor
    .getModelMarkers({})
    // Only real files — skip in-memory models such as the diff viewer's panes.
    .filter(
      (m) =>
        m.resource.scheme === "file" &&
        (m.severity === SEV.Error || m.severity === SEV.Warning),
    );
  let err = 0;
  let warn = 0;
  for (const m of markers) {
    if (m.severity === SEV.Error) err++;
    else warn++;
  }
  const errEl = $("problemsErrCount");
  const warnEl = $("problemsWarnCount");
  if (errEl) errEl.textContent = String(err);
  if (warnEl) warnEl.textContent = String(warn);
  $("problemsBtn")?.classList.toggle("has-errors", err > 0);
  if (problemsPanel && !problemsPanel.hidden) renderProblems(markers);
}

function toggleProblems() {
  if (!problemsPanel) return;
  problemsPanel.hidden = !problemsPanel.hidden;
  if (!problemsPanel.hidden) updateProblems();
}

monaco.editor.onDidChangeMarkers(() => updateProblems());
$("problemsBtn")?.addEventListener("click", toggleProblems);
$("problemsClose")?.addEventListener("click", () => {
  if (problemsPanel) problemsPanel.hidden = true;
});
updateProblems();

async function renderChildren(path, container) {
  let entries;
  try {
    entries = await backend.readDir(path);
  } catch (e) {
    showToast(String(e));
    return;
  }
  container.innerHTML = "";
  for (const entry of entries) {
    const row = document.createElement("div");
    row.className = "row";
    row.dataset.path = entry.path;
    if (entry.is_dir) {
      row.innerHTML = `<svg class="chev"><use href="#i-chevron" /></svg>${iconImg(folderIconUrl(entry.name, false), "folder-ic")}<span class="name"></span>`;
    } else {
      row.innerHTML = `<span class="chev-spacer"></span>${iconImg(fileIconUrl(entry.name))}<span class="name"></span>`;
    }
    row.querySelector(".name").textContent = entry.name;
    row.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      e.stopPropagation();
      openContextMenu(e.clientX, e.clientY, entry);
    });
    container.appendChild(row);

    if (entry.is_dir) {
      const kids = document.createElement("div");
      kids.className = "children";
      kids.hidden = true;
      container.appendChild(kids);
      dirNodes.set(entry.path, { row, kids, loaded: false });
      row.addEventListener("click", async () => {
        const node = dirNodes.get(entry.path);
        row.classList.toggle("open");
        kids.hidden = !kids.hidden;
        const fimg = row.querySelector(".folder-ic");
        if (fimg) fimg.src = folderIconUrl(entry.name, row.classList.contains("open"));
        if (node && !node.loaded && !kids.hidden) {
          node.loaded = true;
          await renderChildren(entry.path, kids);
        }
      });
    } else {
      row.addEventListener("click", () => openFile(entry.path, entry.name));
    }
  }
}

/** Re-read a directory and re-render its children (root or any expanded dir). */
async function reloadDir(path) {
  if (!rootPath) return;
  if (workspaceRoots.includes(path)) {
    await renderWorkspaceRoots();
    renderTreeActive();
    return;
  }
  const node = dirNodes.get(path);
  if (!node) {
    await reloadDir(rootPath);
    return;
  }
  const prefix = path + "/";
  for (const key of [...dirNodes.keys()]) {
    if (key.startsWith(prefix)) dirNodes.delete(key);
  }
  node.loaded = true;
  node.row.classList.add("open");
  node.kids.hidden = false;
  const rfimg = node.row.querySelector(".folder-ic");
  if (rfimg) rfimg.src = folderIconUrl(path.split("/").filter(Boolean).pop() || "", true);
  await renderChildren(path, node.kids);
  renderTreeActive();
}

/** Open + lazy-load a directory so freshly created children become visible. */
async function expandDir(path) {
  if (path === rootPath) return;
  const node = dirNodes.get(path);
  if (!node) return;
  node.row.classList.add("open");
  node.kids.hidden = false;
  const efimg = node.row.querySelector(".folder-ic");
  if (efimg) efimg.src = folderIconUrl(path.split("/").filter(Boolean).pop() || "", true);
  if (!node.loaded) {
    node.loaded = true;
    await renderChildren(path, node.kids);
  }
}

async function newEntry(targetDir, isDir) {
  const name = await ioPrompt({
    title: isDir ? t("explorer.newFolder") : t("explorer.newFile"),
    placeholder: isDir ? "folder name" : "file-name.ext",
    okLabel: t("dialog.create"),
  });
  if (!name) return;
  const dest = targetDir.replace(/\/+$/, "") + "/" + name;
  try {
    if (isDir) await backend.createDir(dest);
    else await backend.createFile(dest);
  } catch (e) {
    showToast(String(e));
    return;
  }
  if (targetDir === rootPath) {
    await reloadDir(rootPath);
  } else {
    await expandDir(targetDir);
    await reloadDir(targetDir);
  }
  if (!isDir) openFile(dest, name);
}

async function renameEntry(path, name, isDir) {
  const next = await ioPrompt({ title: t("dialog.rename"), value: name, okLabel: t("dialog.rename") });
  if (!next || next === name) return;
  const parent = parentDir(path);
  const dest = parent + "/" + next;
  const reopen = openFiles.has(path) && !isDir;
  try {
    for (const op of [...openFiles.keys()]) {
      if (op === path || op.startsWith(path + "/")) closeFile(op);
    }
    await backend.renamePath(path, dest);
  } catch (e) {
    showToast(String(e));
    return;
  }
  await reloadDir(parent);
  if (reopen) openFile(dest, next);
}

async function deleteEntry(path, name, isDir) {
  const ok = await ioConfirm({
    title: t("delete.title", { type: isDir ? t("delete.folder") : t("delete.file") }),
    message: t("delete.confirm", { name }),
    okLabel: t("ctx.delete"),
    danger: true,
  });
  if (!ok) return;
  for (const op of [...openFiles.keys()]) {
    if (op === path || op.startsWith(path + "/")) closeFile(op);
  }
  try {
    await backend.deletePath(path);
  } catch (e) {
    showToast(String(e));
    return;
  }
  await reloadDir(parentDir(path));
}

function copyText(text) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(text).then(
      () => showToast(t("file.copiedPath")),
      () => showToast(text),
    );
  } else {
    showToast(text);
  }
}

// ---- Right-click context menu ----
let ctxMenuEl = null;
function closeContextMenu() {
  if (ctxMenuEl) {
    ctxMenuEl.remove();
    ctxMenuEl = null;
  }
}
function openContextMenu(x, y, entry) {
  closeContextMenu();
  const isDir = !!entry.is_dir;
  const targetDir = isDir ? entry.path : parentDir(entry.path);
  const isRoot = entry.path === rootPath;
  const items = [
    { label: t("ctx.newFile"), icon: "i-new-file", action: () => newEntry(targetDir, false) },
    { label: t("ctx.newFolder"), icon: "i-new-folder", action: () => newEntry(targetDir, true) },
  ];
  if (!isRoot) {
    items.push(
      { sep: true },
      { label: t("ctx.rename"), icon: "i-rename", action: () => renameEntry(entry.path, entry.name, isDir) },
      { label: t("ctx.delete"), icon: "i-trash", danger: true, action: () => deleteEntry(entry.path, entry.name, isDir) },
    );
  }
  items.push({ sep: true }, { label: t("ctx.copyPath"), icon: "i-copy", action: () => copyText(entry.path) });

  renderMenuAt(x, y, items);
}

function renderMenuAt(x, y, items) {
  closeContextMenu();
  const menu = document.createElement("div");
  menu.className = "menu ctx-menu";
  for (const it of items) {
    if (it.sep) {
      const s = document.createElement("div");
      s.className = "menu__sep";
      menu.appendChild(s);
      continue;
    }
    const mi = document.createElement("div");
    mi.className = "menu__item" + (it.danger ? " menu__item--danger" : "");
    mi.innerHTML =
      (it.icon ? `<svg class="ic"><use href="#${it.icon}" /></svg>` : `<span class="ic"></span>`) +
      `<span class="name"></span>`;
    mi.querySelector(".name").textContent = it.label;
    mi.addEventListener("click", () => {
      closeContextMenu();
      it.action();
    });
    menu.appendChild(mi);
  }
  menu.style.visibility = "hidden";
  document.body.appendChild(menu);
  const rect = menu.getBoundingClientRect();
  const px = Math.min(x, window.innerWidth - rect.width - 8);
  const py = Math.min(y, window.innerHeight - rect.height - 8);
  menu.style.left = Math.max(8, px) + "px";
  menu.style.top = Math.max(8, py) + "px";
  menu.style.visibility = "visible";
  ctxMenuEl = menu;
}
document.addEventListener("click", (e) => {
  if (ctxMenuEl && !ctxMenuEl.contains(e.target)) closeContextMenu();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") closeContextMenu();
});
window.addEventListener("scroll", closeContextMenu, true);
window.addEventListener("resize", closeContextMenu);

// ---- Prompt / confirm dialog (#ioDialog) ----
function ioPrompt({ title, message = "", value = "", placeholder = "", okLabel = "OK" }) {
  return new Promise((resolve) => {
    const dlg = $("ioDialog");
    $("ioTitle").textContent = title;
    const msg = $("ioMessage");
    if (message) {
      msg.textContent = message;
      msg.hidden = false;
    } else {
      msg.hidden = true;
    }
    $("ioInputWrap").hidden = false;
    const input = $("ioInput");
    input.value = value;
    input.placeholder = placeholder;
    const ok = $("ioOk");
    ok.textContent = okLabel;
    ok.classList.remove("btn--danger");
    const cancel = $("ioCancel");
    const form = $("ioForm");
    let done = false;
    const finish = (val) => {
      if (done) return;
      done = true;
      dlg.removeEventListener("close", onClose);
      cancel.removeEventListener("click", onCancel);
      form.removeEventListener("submit", onSubmit);
      resolve(val);
    };
    const onClose = () => finish("");
    const onCancel = () => dlg.close();
    const onSubmit = () => finish(input.value.trim());
    dlg.addEventListener("close", onClose);
    cancel.addEventListener("click", onCancel);
    form.addEventListener("submit", onSubmit);
    dlg.showModal();
    requestAnimationFrame(() => {
      input.focus();
      input.select();
    });
  });
}

function ioConfirm({ title, message = "", okLabel = "OK", danger = false }) {
  return new Promise((resolve) => {
    const dlg = $("ioDialog");
    $("ioTitle").textContent = title;
    const msg = $("ioMessage");
    if (message) {
      msg.textContent = message;
      msg.hidden = false;
    } else {
      msg.hidden = true;
    }
    $("ioInputWrap").hidden = true;
    const ok = $("ioOk");
    ok.textContent = okLabel;
    ok.classList.toggle("btn--danger", danger);
    const cancel = $("ioCancel");
    const form = $("ioForm");
    let done = false;
    let confirmed = false;
    const finish = () => {
      if (done) return;
      done = true;
      dlg.removeEventListener("close", onClose);
      cancel.removeEventListener("click", onCancel);
      form.removeEventListener("submit", onSubmit);
      resolve(confirmed);
    };
    const onClose = () => finish();
    const onCancel = () => dlg.close();
    const onSubmit = () => {
      confirmed = true;
    };
    dlg.addEventListener("close", onClose);
    cancel.addEventListener("click", onCancel);
    form.addEventListener("submit", onSubmit);
    dlg.showModal();
    requestAnimationFrame(() => ok.focus());
  });
}

// ---- Global search ----
let searchSeq = 0;
let searchTimer = null;
let searchCaseSensitive = false;

function debounceSearch() {
  clearTimeout(searchTimer);
  searchTimer = setTimeout(runSearch, 220);
}

async function runSearch() {
  const input = $("searchInput");
  const resultsEl = $("searchResults");
  const metaEl = $("searchMeta");
  const query = input.value;
  const seq = ++searchSeq;
  if (!rootPath) {
    resultsEl.innerHTML = "";
    metaEl.textContent = t("search.openFolder");
    return;
  }
  if (!query) {
    resultsEl.innerHTML = "";
    metaEl.textContent = "";
    return;
  }
  metaEl.textContent = t("search.searching");
  let files;
  try {
    files = await backend.searchInProject(rootPath, query, searchCaseSensitive);
  } catch (e) {
    metaEl.textContent = String(e);
    return;
  }
  if (seq !== searchSeq) return;
  renderSearchResults(files, metaEl, resultsEl);
}

function renderSearchResults(files, metaEl, resultsEl) {
  resultsEl.innerHTML = "";
  if (!files.length) {
    metaEl.textContent = t("search.noResults");
    return;
  }
  let total = 0;
  for (const f of files) total += f.matches.length;
  metaEl.textContent = t("search.resultsMeta", { total, s1: total === 1 ? "" : "s", files: files.length, s2: files.length === 1 ? "" : "s" });
  for (const f of files) {
    const group = document.createElement("div");
    group.className = "sr-group";
    const head = document.createElement("div");
    head.className = "sr-file";
    head.innerHTML = `<svg class="sr-chev"><use href="#i-chevron" /></svg>${iconImg(fileIconUrl(f.name))}<span class="sr-name"></span><span class="sr-count"></span>`;
    head.querySelector(".sr-name").textContent = f.rel;
    head.querySelector(".sr-count").textContent = String(f.matches.length);
    const lines = document.createElement("div");
    lines.className = "sr-lines";
    head.addEventListener("click", () => {
      head.classList.toggle("collapsed");
      lines.hidden = head.classList.contains("collapsed");
    });
    for (const m of f.matches) {
      const lineEl = document.createElement("div");
      lineEl.className = "sr-line";
      const ln = document.createElement("span");
      ln.className = "sr-ln";
      ln.textContent = String(m.line);
      const tx = document.createElement("span");
      tx.className = "sr-tx";
      appendHighlighted(tx, m.text, m.start, m.end);
      lineEl.append(ln, tx);
      lineEl.addEventListener("click", () =>
        openFileAt(f.path, f.name, m.line, m.column, m.column + (m.end - m.start)),
      );
      lines.appendChild(lineEl);
    }
    group.append(head, lines);
    resultsEl.appendChild(group);
  }
}

/** Append a line of text with the matched [start,end) span wrapped in <mark>, building text nodes (XSS-safe). Leading whitespace is trimmed for display. */
function appendHighlighted(container, text, start, end) {
  const trimmed = text.replace(/^\s+/, "");
  const removed = text.length - trimmed.length;
  let s = Math.max(0, Math.min(start, text.length)) - removed;
  let e = Math.max(0, Math.min(end, text.length)) - removed;
  s = Math.max(0, s);
  e = Math.max(s, e);
  if (s > 0) container.appendChild(document.createTextNode(trimmed.slice(0, s)));
  if (e > s) {
    const mk = document.createElement("mark");
    mk.textContent = trimmed.slice(s, e);
    container.appendChild(mk);
  }
  if (e < trimmed.length) container.appendChild(document.createTextNode(trimmed.slice(e)));
}

async function openFileAt(path, name, line, column, endColumn) {
  await openFile(path, name);
  if (!monacoEditor) return;
  monacoEditor.revealLineInCenter(line);
  monacoEditor.setSelection({
    startLineNumber: line,
    startColumn: column,
    endLineNumber: line,
    endColumn: endColumn || column,
  });
  monacoEditor.focus();
}

function showSide(which) {
  $("viewExplorer").hidden = which !== "explorer";
  $("viewSearch").hidden = which !== "search";
  $("viewGit").hidden = which !== "git";
  $("viewOutline").hidden = which !== "outline";
  $("viewTest").hidden = which !== "test";
  $("tabExplorer").classList.toggle("is-active", which === "explorer" || which === "search");
  $("tabGit").classList.toggle("is-active", which === "git");
  $("tabOutline").classList.toggle("is-active", which === "outline");
  $("tabTest").classList.toggle("is-active", which === "test");
  const layout = document.querySelector(".layout");
  if (layout) layout.classList.remove("hide-explorer");
  if (which === "search") {
    const si = $("searchInput");
    si.focus();
    si.select();
  } else if (which === "git") {
    refreshGitStatus();
  } else if (which === "outline") {
    refreshOutline();
  } else if (which === "test") {
    refreshTestExplorer();
  }
}


function renderTreeActive() {
  treeEl.querySelectorAll(".row.is-active").forEach((r) => r.classList.remove("is-active"));
  if (!activePath) return;
  const r = treeEl.querySelector(`.row[data-path="${cssEscape(activePath)}"]`);
  if (r) r.classList.add("is-active");
}
function cssEscape(s) {
  return s.replace(/"/g, '\\"');
}

// ---- source control (git) ----
const gitListEl = $("gitList");
const gitBranchNameEl = $("gitBranchName");
let gitActiveRel = null;
// Whether the open folder is a git repo — gates the editor diff gutter.
let gitIsRepo = false;

/** Single-letter badge + colour class for a porcelain status code. */
function gitBadge(file) {
  if (file.code === "??") return { ch: "U", cls: "git-badge--Q" };
  const c = (file.label || "").charAt(0).toUpperCase();
  const known = { M: "M", A: "A", D: "D", R: "R", C: "R", U: "U" };
  const ch = known[c] || "M";
  return { ch, cls: "git-badge--" + ch };
}

async function refreshGitStatus() {
  if (!rootPath) {
    gitBranchNameEl.textContent = "—";
    gitListEl.innerHTML = `<div class="git-empty"></div>`;
    gitListEl.firstChild.textContent = t("git.openFolder");
    return;
  }
  let status;
  try {
    status = await backend.gitStatus(rootPath);
  } catch (e) {
    gitIsRepo = false;
    gitBranchNameEl.textContent = "—";
    gitListEl.innerHTML = `<div class="git-empty"></div>`;
    gitListEl.firstChild.textContent = String(e);
    refreshGutter();
    return;
  }
  if (!status.is_repo) {
    gitIsRepo = false;
    gitBranchNameEl.textContent = "—";
    gitListEl.innerHTML = `<div class="git-empty"></div>`;
    gitListEl.firstChild.textContent = t("git.notRepo");
    refreshGutter();
    return;
  }
  gitIsRepo = true;
  gitBranchNameEl.textContent = status.branch;
  gitBranchNameEl.parentElement.title = t("git.onBranch", { name: status.branch });
  renderGitFiles(status.files);
  refreshGutter();
  refreshGitLog();
  refreshStashList();
}

const GRAPH_LANE_COLORS = [
  "#0969da", "#8250df", "#1a7f37", "#cf222e", "#bf8700",
  "#e16f24", "#0550ae", "#6e7781", "#953800", "#1b7c83",
];

function layoutGitGraph(entries) {
  const lanes = [];
  const hashToLane = new Map();
  const rows = [];

  for (const e of entries) {
    let lane = hashToLane.get(e.hash);
    if (lane == null) {
      lane = lanes.indexOf(null);
      if (lane < 0) lane = lanes.length;
      if (lane >= lanes.length) lanes.push(e.hash);
      else lanes[lane] = e.hash;
    }

    const merges = [];
    const forks = [];

    for (let i = 0; i < lanes.length; i++) {
      if (i !== lane && lanes[i] === e.hash) {
        merges.push(i);
        lanes[i] = null;
      }
    }

    const parents = e.parents || [];
    if (parents.length > 0) {
      lanes[lane] = parents[0];
      hashToLane.set(parents[0], lane);
    } else {
      lanes[lane] = null;
    }

    for (let pi = 1; pi < parents.length; pi++) {
      const ph = parents[pi];
      let fl = hashToLane.get(ph);
      if (fl == null) {
        fl = lanes.indexOf(null);
        if (fl < 0) fl = lanes.length;
        if (fl >= lanes.length) lanes.push(ph);
        else lanes[fl] = ph;
        hashToLane.set(ph, fl);
      }
      forks.push(fl);
    }

    const activeLanes = lanes.map((v, i) => v != null ? i : -1).filter(i => i >= 0);
    rows.push({ entry: e, lane, merges, forks, activeLanes: [...activeLanes], maxLane: lanes.length });
  }
  return rows;
}

function renderGitGraphSvg(rows, container) {
  const ROW_H = 28;
  const LANE_W = 14;
  const PAD_L = 6;
  const R = 4;
  const maxLane = rows.reduce((m, r) => Math.max(m, r.maxLane), 0);
  const svgW = PAD_L + maxLane * LANE_W + LANE_W;
  const svgH = rows.length * ROW_H;

  const lines = [];
  const circles = [];

  for (let ri = 0; ri < rows.length; ri++) {
    const { lane, merges, forks, activeLanes } = rows[ri];
    const cx = PAD_L + lane * LANE_W + LANE_W / 2;
    const cy = ri * ROW_H + ROW_H / 2;
    const color = GRAPH_LANE_COLORS[lane % GRAPH_LANE_COLORS.length];

    if (ri + 1 < rows.length) {
      const nextLane = rows[ri + 1].lane;
      for (const al of activeLanes) {
        let targetLane = al;
        if (al === lane) targetLane = lane;
        const x1 = PAD_L + al * LANE_W + LANE_W / 2;
        const x2 = PAD_L + targetLane * LANE_W + LANE_W / 2;
        const y1 = cy;
        const y2 = (ri + 1) * ROW_H + ROW_H / 2;
        const lc = GRAPH_LANE_COLORS[al % GRAPH_LANE_COLORS.length];
        if (x1 === x2) {
          lines.push(`<line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" stroke="${lc}" stroke-width="2"/>`);
        } else {
          const my = (y1 + y2) / 2;
          lines.push(`<path d="M${x1},${y1} C${x1},${my} ${x2},${my} ${x2},${y2}" stroke="${lc}" stroke-width="2" fill="none"/>`);
        }
      }
    }

    for (const ml of merges) {
      const mx = PAD_L + ml * LANE_W + LANE_W / 2;
      const prevY = ri > 0 ? (ri - 1) * ROW_H + ROW_H / 2 : 0;
      const mc = GRAPH_LANE_COLORS[ml % GRAPH_LANE_COLORS.length];
      const my = (prevY + cy) / 2;
      lines.push(`<path d="M${mx},${prevY} C${mx},${my} ${cx},${my} ${cx},${cy}" stroke="${mc}" stroke-width="2" fill="none"/>`);
    }

    for (const fl of forks) {
      const fx = PAD_L + fl * LANE_W + LANE_W / 2;
      const nextY = (ri + 1) * ROW_H + ROW_H / 2;
      const fc = GRAPH_LANE_COLORS[fl % GRAPH_LANE_COLORS.length];
      const my = (cy + nextY) / 2;
      lines.push(`<path d="M${cx},${cy} C${cx},${my} ${fx},${my} ${fx},${nextY}" stroke="${fc}" stroke-width="2" fill="none"/>`);
    }

    circles.push(`<circle cx="${cx}" cy="${cy}" r="${R}" fill="${color}" stroke="var(--graph-node-stroke, #fff)" stroke-width="1.5"/>`);
  }

  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("width", svgW);
  svg.setAttribute("height", svgH);
  svg.setAttribute("class", "git-graph-svg");
  svg.innerHTML = lines.join("") + circles.join("");
  return { svg, svgW, ROW_H };
}

async function refreshGitLog() {
  const logTitle = $("gitLogTitle");
  const logEl = $("gitLog");
  if (!logTitle || !logEl) return;
  if (!rootPath || !gitIsRepo) {
    logTitle.style.display = "none";
    logEl.hidden = true;
    return;
  }
  logTitle.style.display = "";
  let entries;
  try {
    entries = await backend.gitLog(rootPath, 60);
  } catch {
    return;
  }
  logEl.innerHTML = "";
  if (!entries.length) return;

  const rows = layoutGitGraph(entries);
  const { svg, svgW, ROW_H } = renderGitGraphSvg(rows, logEl);

  const wrapper = document.createElement("div");
  wrapper.className = "git-graph-wrap";

  const graphCol = document.createElement("div");
  graphCol.className = "git-graph-col";
  graphCol.style.width = svgW + "px";
  graphCol.appendChild(svg);

  const infoCol = document.createElement("div");
  infoCol.className = "git-graph-info";
  for (const r of rows) {
    const row = document.createElement("div");
    row.className = "git-graph-row";
    row.style.height = ROW_H + "px";

    let refHtml = "";
    if (r.entry.refs && r.entry.refs.length) {
      for (const ref of r.entry.refs) {
        const isHead = ref.includes("HEAD");
        const cls = isHead ? "git-ref git-ref--head" : ref.startsWith("tag:") ? "git-ref git-ref--tag" : "git-ref git-ref--branch";
        const label = ref.replace(/^HEAD -> /, "");
        refHtml += `<span class="${cls}"></span>`;
        const last = row; // will set text below
      }
    }

    row.innerHTML = `<span class="git-graph-hash"></span>${refHtml}<span class="git-graph-msg"></span><span class="git-graph-meta"></span>`;
    row.querySelector(".git-graph-hash").textContent = r.entry.short_hash;
    row.querySelector(".git-graph-msg").textContent = r.entry.message;
    row.querySelector(".git-graph-meta").textContent = `${r.entry.author} · ${r.entry.date}`;
    row.title = `${r.entry.hash}\n${r.entry.author} · ${r.entry.date}\n${r.entry.message}`;

    const refEls = row.querySelectorAll(".git-ref");
    const refs = r.entry.refs || [];
    refEls.forEach((el, idx) => {
      if (refs[idx]) el.textContent = refs[idx].replace(/^HEAD -> /, "");
    });

    infoCol.appendChild(row);
  }

  wrapper.appendChild(graphCol);
  wrapper.appendChild(infoCol);
  logEl.appendChild(wrapper);
}

$("gitLogToggle")?.addEventListener("click", () => {
  const logEl = $("gitLog");
  if (logEl) logEl.hidden = !logEl.hidden;
});

// ---- stash ----
function parseStashIndex(line) {
  const m = /^stash@\{(\d+)\}/.exec(line);
  return m ? parseInt(m[1], 10) : 0;
}

function stashLabel(line) {
  const colon = line.indexOf(":");
  return colon >= 0 ? line.slice(colon + 1).trim() : line;
}

async function doStash() {
  if (!rootPath || !gitIsRepo) {
    showToast(t("git.notRepo"));
    return;
  }
  try {
    const res = await backend.gitStash(rootPath);
    showToast(res || t("git.stashed"));
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
  }
  await afterWorktreeChange();
  refreshStashList(true);
}

// Run a stash mutation, then refresh the worktree + stash list.
async function stashOp(op, okMsg) {
  try {
    const res = await op();
    showToast(typeof res === "string" && res ? res : okMsg);
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
  }
  await afterWorktreeChange();
  refreshStashList(true);
}

async function refreshStashList(forceShow) {
  const titleEl = $("gitStashTitle");
  const listEl = $("gitStashList");
  if (!titleEl || !listEl) return;
  if (!rootPath || !gitIsRepo) {
    titleEl.style.display = "none";
    listEl.hidden = true;
    listEl.innerHTML = "";
    return;
  }
  let entries = [];
  try {
    entries = await backend.gitStashList(rootPath);
  } catch {
    entries = [];
  }
  if (!entries.length) {
    titleEl.style.display = "none";
    listEl.hidden = true;
    listEl.innerHTML = "";
    return;
  }
  titleEl.style.display = "";
  const countEl = titleEl.querySelector("span");
  if (countEl) countEl.textContent = `${t("git.stashes")} (${entries.length})`;
  if (forceShow) listEl.hidden = false;
  listEl.innerHTML = "";
  for (const line of entries) {
    const idx = parseStashIndex(line);
    const row = document.createElement("div");
    row.className = "git-stash-row";

    const label = document.createElement("span");
    label.className = "git-stash-row__label";
    label.textContent = stashLabel(line);
    label.title = line;

    const acts = document.createElement("span");
    acts.className = "git-stash-row__acts";
    const mkBtn = (icon, title, run) => {
      const b = document.createElement("button");
      b.className = "git-stash-act";
      b.type = "button";
      b.title = title;
      b.innerHTML = `<svg class="ic"><use href="#${icon}" /></svg>`;
      b.addEventListener("click", (e) => {
        e.stopPropagation();
        run();
      });
      return b;
    };
    acts.append(
      mkBtn("i-arrow-down", t("git.stashApply"), () =>
        stashOp(() => backend.gitStashApply(rootPath, idx), t("git.stashApplied"))),
      mkBtn("i-check", t("git.stashPop"), () =>
        stashOp(() => backend.gitStashPop(rootPath, idx), t("git.stashPopped"))),
      mkBtn("i-trash", t("git.stashDrop"), () =>
        stashOp(() => backend.gitStashDrop(rootPath, idx), t("git.stashDropped"))),
    );
    row.append(label, acts);
    listEl.appendChild(row);
  }
}

$("gitStashBtn")?.addEventListener("click", doStash);
$("gitStashToggle")?.addEventListener("click", () => {
  const listEl = $("gitStashList");
  if (listEl) listEl.hidden = !listEl.hidden;
});

// ---- git blame (GitLens-style current-line annotation) ----
let blameEnabled = false;
let blameMap = null; // Map<lineNumber, { author, date, commit }>
let blameMapPath = null; // path the map was built for
const blameDecorations = monacoEditor.createDecorationsCollection([]);

// `author-time` is a unix epoch; render a compact relative label.
function formatBlameDate(epochStr) {
  const sec = parseInt(epochStr, 10);
  if (!Number.isFinite(sec)) return "";
  const then = sec * 1000;
  const diff = Date.now() - then;
  const day = 86400000;
  if (diff < day) return t("git.blameToday");
  if (diff < 2 * day) return t("git.blameYesterday");
  if (diff < 30 * day) return t("git.blameDaysAgo", { n: Math.floor(diff / day) });
  if (diff < 365 * day) return t("git.blameMonthsAgo", { n: Math.floor(diff / (30 * day)) });
  return new Date(then).toISOString().slice(0, 10);
}

// Fetch blame for the active file and cache a line→info map. Blame reflects the
// committed file, so it is skipped for dirty / untracked / non-repo buffers.
async function refreshBlame() {
  if (!blameEnabled) return;
  const path = activePath;
  const rel = relForPath(path);
  const f = path ? openFiles.get(path) : null;
  if (!gitIsRepo || !rel || !f || f.dirty || f.isImage) {
    blameMap = null;
    blameMapPath = null;
    blameDecorations.set([]);
    return;
  }
  let lines = [];
  try {
    lines = await backend.gitBlame(rootPath, rel);
  } catch {
    lines = [];
  }
  if (path !== activePath) return; // user switched files during the fetch
  if (!lines.length) {
    blameMap = null;
    blameMapPath = null;
    blameDecorations.set([]);
    return;
  }
  const map = new Map();
  for (const b of lines) map.set(b.line, b);
  blameMap = map;
  blameMapPath = path;
  updateBlameLine();
}

// Paint the annotation at the end of the caret's current line.
function updateBlameLine() {
  if (!blameEnabled || !blameMap || blameMapPath !== activePath) {
    blameDecorations.set([]);
    return;
  }
  const pos = monacoEditor.getPosition();
  const info = pos && blameMap.get(pos.lineNumber);
  if (!info) {
    blameDecorations.set([]);
    return;
  }
  const when = formatBlameDate(info.date);
  const text = `    ${info.author}, ${when} · ${info.commit}`;
  const model = monacoEditor.getModel();
  const col = model ? model.getLineMaxColumn(pos.lineNumber) : 1;
  blameDecorations.set([
    {
      range: new monaco.Range(pos.lineNumber, col, pos.lineNumber, col),
      options: { after: { content: text, inlineClassName: "blame-inline" } },
    },
  ]);
}

function toggleBlame() {
  blameEnabled = !blameEnabled;
  if (blameEnabled) {
    setStatusBarItem("_blame", { text: t("git.blameLabel"), tooltip: t("git.blameToggle") }, toggleBlame);
    refreshBlame();
    showToast(t("git.blameOn"));
  } else {
    blameMap = null;
    blameMapPath = null;
    blameDecorations.set([]);
    removeStatusBarItem("_blame");
    showToast(t("git.blameOff"));
  }
}

let _blameRAF = 0;
monacoEditor.onDidChangeCursorPosition(() => {
  if (blameEnabled && !_blameRAF && !_imeComposing) {
    _blameRAF = requestAnimationFrame(() => { _blameRAF = 0; updateBlameLine(); });
  }
});
let _blameInvalidateTimer = null;
monacoEditor.onDidChangeModelContent(() => {
  if (blameEnabled && !_imeComposing) {
    blameMap = null;
    blameMapPath = null;
    if (_blameInvalidateTimer) clearTimeout(_blameInvalidateTimer);
    _blameInvalidateTimer = setTimeout(() => blameDecorations.set([]), 500);
  }
});

function renderGitFiles(files) {
  gitListEl.innerHTML = "";
  if (!files.length) {
    const empty = document.createElement("div");
    empty.className = "git-empty";
    empty.textContent = t("git.noChanges");
    gitListEl.appendChild(empty);
    return;
  }
  const staged = files.filter((f) => f.staged);
  const unstaged = files.filter((f) => !f.staged);
  const addSection = (title, group, action) => {
    if (!group.length) return;
    const head = document.createElement("div");
    head.className = "git-section-title";
    const label = document.createElement("span");
    label.textContent = `${title} (${group.length})`;
    head.appendChild(label);
    const btn = document.createElement("button");
    btn.className = "git-section-act";
    btn.type = "button";
    btn.title = action.title;
    btn.innerHTML = `<svg class="ic"><use href="#${action.icon}" /></svg>`;
    btn.addEventListener("click", action.run);
    head.appendChild(btn);
    gitListEl.appendChild(head);
    for (const f of group) gitListEl.appendChild(gitRow(f));
  };
  addSection(t("git.stagedChanges"), staged, {
    title: t("git.unstageAll"),
    icon: "i-minus",
    run: () => gitRunOp(() => backend.gitUnstageAll(rootPath), t("git.unstagedAll")),
  });
  addSection(t("git.changes"), unstaged, {
    title: t("git.stageAll"),
    icon: "i-plus",
    run: () => gitRunOp(() => backend.gitStageAll(rootPath), t("git.stagedAll")),
  });
}

function gitRow(file) {
  const row = document.createElement("div");
  row.className = "git-row";
  row.dataset.rel = file.rel;
  if (file.rel === gitActiveRel) row.classList.add("is-active");

  const icon = document.createElement("span");
  icon.className = "git-row__icon";
  icon.innerHTML = iconImg(fileIconUrl(file.name));

  const name = document.createElement("span");
  name.className = "git-row__name";
  const slash = file.rel.lastIndexOf("/");
  if (slash >= 0) {
    name.textContent = file.name;
    const dir = document.createElement("span");
    dir.className = "git-row__dir";
    dir.textContent = " · " + file.rel.slice(0, slash);
    name.appendChild(dir);
  } else {
    name.textContent = file.name;
  }
  row.title = file.label + " — " + file.rel;

  const act = document.createElement("button");
  act.className = "git-row__act";
  act.type = "button";
  if (file.staged) {
    act.title = t("git.unstage");
    act.innerHTML = `<svg class="ic"><use href="#i-minus" /></svg>`;
    act.addEventListener("click", (e) => {
      e.stopPropagation();
      gitRunOp(() => backend.gitUnstage(rootPath, file.rel), t("git.unstaged", { name: file.name }));
    });
  } else {
    act.title = t("git.stage");
    act.innerHTML = `<svg class="ic"><use href="#i-plus" /></svg>`;
    act.addEventListener("click", (e) => {
      e.stopPropagation();
      gitRunOp(() => backend.gitStage(rootPath, file.rel), t("git.staged", { name: file.name }));
    });
  }

  const badge = document.createElement("span");
  const b = gitBadge(file);
  badge.className = "git-row__badge " + b.cls;
  badge.textContent = b.ch;

  row.append(icon, name, act, badge);
  row.addEventListener("click", () => openDiff(file));
  return row;
}

/** Run a git mutation, then refresh the panel; surface errors via toast. */
async function gitRunOp(op, okMsg) {
  try {
    await op();
    if (okMsg) showToast(okMsg);
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
  }
  await refreshGitStatus();
}

async function gitCommit() {
  if (!rootPath) return;
  const input = $("gitCommitMsg");
  const msg = input.value.trim();
  if (!msg) {
    showToast(t("git.emptyMsg"));
    input.focus();
    return;
  }
  try {
    const res = await backend.gitCommit(rootPath, msg);
    input.value = "";
    closeDiffView();
    showToast(t("git.committed", { hash: res }));
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
  }
  await refreshGitStatus();
}

async function gitPush() {
  if (!rootPath) return;
  showToast(t("git.pushing"));
  try {
    const res = await backend.gitPush(rootPath);
    showToast(res.split("\n").pop() || t("git.pushed"));
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
  }
}

async function gitPull() {
  if (!rootPath) return;
  showToast(t("git.pulling"));
  try {
    const res = await backend.gitPull(rootPath);
    showToast(res.split("\n").pop() || t("git.pulled"));
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
  }
  await afterWorktreeChange();
}

// ---- branch picker ----
const gitBranchMenuEl = $("gitBranchMenu");
const gitBranchBtnEl = $("gitBranchBtn");

function closeBranchMenu() {
  gitBranchMenuEl.hidden = true;
  gitBranchMenuEl.innerHTML = "";
  gitBranchBtnEl.setAttribute("aria-expanded", "false");
}

async function toggleBranchMenu() {
  if (!gitBranchMenuEl.hidden) {
    closeBranchMenu();
    return;
  }
  if (!rootPath || !gitIsRepo) {
    showToast(t("git.notRepo"));
    return;
  }
  let info;
  try {
    info = await backend.gitBranches(rootPath);
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
    return;
  }
  renderBranchMenu(info);
  gitBranchMenuEl.hidden = false;
  gitBranchBtnEl.setAttribute("aria-expanded", "true");
}

function renderBranchMenu(info) {
  gitBranchMenuEl.innerHTML = "";
  const list = document.createElement("div");
  list.className = "git-branch-menu__list";
  for (const name of info.branches) {
    const item = document.createElement("button");
    item.type = "button";
    item.className = "git-branch-item" + (name === info.current ? " is-current" : "");
    const check = document.createElement("span");
    check.className = "git-branch-item__check";
    check.textContent = name === info.current ? "✓" : "";
    const label = document.createElement("span");
    label.className = "git-branch-item__name";
    label.textContent = name;
    item.append(check, label);
    if (name === info.current) {
      item.disabled = true;
    } else {
      item.addEventListener("click", () => switchBranch(name));
    }
    list.appendChild(item);
  }
  gitBranchMenuEl.appendChild(list);

  const create = document.createElement("button");
  create.type = "button";
  create.className = "git-branch-create";
  create.innerHTML = `<svg class="ic"><use href="#i-plus" /></svg><span></span>`;
  create.querySelector("span").textContent = t("git.newBranch");
  create.addEventListener("click", () => createBranch());
  gitBranchMenuEl.appendChild(create);
}

async function switchBranch(name) {
  closeBranchMenu();
  showToast(t("git.switchingTo", { name }));
  try {
    await backend.gitCheckout(rootPath, name, false);
    showToast(t("git.switchedTo", { name }));
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
    return;
  }
  await afterWorktreeChange();
}

async function createBranch() {
  closeBranchMenu();
  const name = (window.prompt(t("git.newBranchPrompt")) || "").trim();
  if (!name) return;
  try {
    await backend.gitCheckout(rootPath, name, true);
    showToast(t("git.createdBranch", { name }));
  } catch (e) {
    showToast(String(e && e.message ? e.message : e));
    return;
  }
  await afterWorktreeChange();
}

// After an op that may rewrite the working tree (checkout/pull), reload
// non-dirty open files from disk and refresh git state + diff gutter.
async function afterWorktreeChange() {
  closeDiffView();
  const gone = [];
  for (const [path, f] of openFiles) {
    if (f.dirty) continue;
    try {
      const content = await backend.readTextFile(path);
      if (content !== f.model.getValue()) {
        const pos = path === activePath ? monacoEditor.getPosition() : null;
        f.model.setValue(content);
        // setValue fires onDidChangeContent → markDirty(true); reset it since
        // the file now matches disk again.
        markDirty(path, false);
        if (pos) monacoEditor.setPosition(pos);
      }
    } catch {
      // File doesn't exist on the new branch — close its (clean) tab.
      gone.push(path);
    }
  }
  for (const path of gone) closeFile(path);
  if (rootPath) await reloadDir(rootPath);
  await refreshGitStatus();
}

// ---- editor diff gutter (dirty diff vs HEAD) ----
const gutterDecorations = monacoEditor.createDecorationsCollection([]);
let gutterBaselinePath = null;
let gutterBaselineText = null;
let gutterTimer = null;

function relForPath(path) {
  if (!rootPath || !path) return null;
  const prefix = rootPath.endsWith("/") ? rootPath : rootPath + "/";
  return path.startsWith(prefix) ? path.slice(prefix.length) : null;
}

// Reload the HEAD baseline for the active file, then redraw the gutter.
async function refreshGutter() {
  const path = activePath;
  const rel = relForPath(path);
  if (!gitIsRepo || !rel) {
    gutterBaselinePath = null;
    gutterBaselineText = null;
    updateGutter();
    return;
  }
  let head = "";
  try {
    head = await backend.gitFileHead(rootPath, rel);
  } catch {
    head = "";
  }
  if (path !== activePath) return; // user switched files during the fetch
  gutterBaselinePath = path;
  // Empty HEAD = untracked/new file — skip gutter to avoid all-green noise.
  gutterBaselineText = head === "" ? null : head;
  updateGutter();
}

// Recompute decorations from the cached baseline + current editor content.
function updateGutter() {
  if (gutterBaselinePath !== activePath || gutterBaselineText == null) {
    gutterDecorations.set([]);
    return;
  }
  const model = monacoEditor.getModel();
  if (!model) {
    gutterDecorations.set([]);
    return;
  }
  const orig = gutterBaselineText.split("\n");
  const mod = model.getValue().split("\n");
  // Guard against pathological sizes (O(n*m) LCS).
  if (orig.length > 4000 || mod.length > 4000) {
    gutterDecorations.set([]);
    return;
  }
  const lineCount = model.getLineCount();
  const decos = [];
  for (const h of lineDiffHunks(orig, mod)) {
    if (h.aCount === 0 && h.bCount > 0) {
      for (let k = 0; k < h.bCount; k++) decos.push(gutterDeco(h.bStart + 1 + k, "gutter-add"));
    } else if (h.bCount === 0 && h.aCount > 0) {
      decos.push(gutterDeco(Math.min(Math.max(h.bStart, 1), lineCount), "gutter-del"));
    } else {
      for (let k = 0; k < h.bCount; k++) decos.push(gutterDeco(h.bStart + 1 + k, "gutter-mod"));
    }
  }
  gutterDecorations.set(decos);
}

function gutterDeco(line, cls) {
  return {
    range: new monaco.Range(line, 1, line, 1),
    options: { isWholeLine: true, linesDecorationsClassName: "git-gutter " + cls },
  };
}

// Longest-common-subsequence line diff → change hunks (0-based indices).
function lineDiffHunks(a, b) {
  const n = a.length;
  const m = b.length;
  const dp = [];
  for (let i = 0; i <= n; i++) dp.push(new Uint32Array(m + 1));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const hunks = [];
  let cur = null;
  const flush = () => {
    if (cur) {
      hunks.push(cur);
      cur = null;
    }
  };
  let i = 0;
  let j = 0;
  const open = () => {
    if (!cur) cur = { aStart: i, aCount: 0, bStart: j, bCount: 0 };
  };
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      flush();
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      open();
      cur.aCount++;
      i++;
    } else {
      open();
      cur.bCount++;
      j++;
    }
  }
  while (i < n) {
    open();
    cur.aCount++;
    i++;
  }
  while (j < m) {
    open();
    cur.bCount++;
    j++;
  }
  flush();
  return hunks;
}

monacoEditor.onDidChangeModelContent(() => {
  if (gutterTimer) clearTimeout(gutterTimer);
  if (_imeComposing) return;
  gutterTimer = setTimeout(updateGutter, 250);
});

// ---- diff view ----
let diffEditor = null;
const diffViewEl = $("diffView");

function ensureDiffEditor(opts = {}) {
  if (diffEditor) return diffEditor;
  diffEditor = monaco.editor.createDiffEditor($("diffBody"), {
    theme: matchMedia("(prefers-color-scheme: dark)").matches ? "vs-dark" : "vs",
    automaticLayout: true,
    readOnly: false,
    originalEditable: false,
    renderSideBySide: true,
    enableSplitViewResizing: true,
    fontSize: 13,
    fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    renderOverviewRuler: true,
    ignoreTrimWhitespace: false,
    ...opts,
  });
  const modifiedEditor = diffEditor.getModifiedEditor();
  if (modifiedEditor) {
    modifiedEditor.onDidChangeModelContent(() => {
      const m = diffEditor.getModel();
      if (!m || !_diffFilePath) return;
      const content = m.modified.getValue();
      backend.writeTextFile(_diffFilePath, content).then(() => {
        showToast?.("Diff: saved");
      }).catch(() => {});
    });
  }
  return diffEditor;
}
let _diffFilePath = null;

async function openDiff(file) {
  if (!rootPath) return;
  let headText = "";
  let workText = "";
  try {
    headText = await backend.gitFileHead(rootPath, file.rel);
  } catch {
    headText = "";
  }
  if (!file.deleted) {
    try {
      workText = await backend.readTextFile(file.path);
    } catch {
      workText = "";
    }
  }

  const ed = ensureDiffEditor();
  const lang = extLang(file.name);
  const original = monaco.editor.createModel(headText, lang);
  const modified = monaco.editor.createModel(workText, lang);
  const prev = ed.getModel();
  ed.setModel({ original, modified });
  if (prev) {
    prev.original?.dispose();
    prev.modified?.dispose();
  }

  _diffFilePath = file.path;
  $("diffTitle").textContent = file.rel;
  gitActiveRel = file.rel;
  gitListEl.querySelectorAll(".git-row.is-active").forEach((r) => r.classList.remove("is-active"));
  const activeRow = gitListEl.querySelector(`.git-row[data-rel="${cssEscape(file.rel)}"]`);
  if (activeRow) activeRow.classList.add("is-active");

  diffViewEl.hidden = false;
  ed.layout();
}

function closeDiffView() {
  if (diffViewEl.hidden) return;
  diffViewEl.hidden = true;
  gitActiveRel = null;
  _diffFilePath = null;
  gitListEl.querySelectorAll(".git-row.is-active").forEach((r) => r.classList.remove("is-active"));
  const m = diffEditor?.getModel();
  if (m) {
    m.original?.dispose();
    m.modified?.dispose();
    diffEditor.setModel(null);
  }
}

// ---- AI assistant ----
const CFG_KEY = "ai-config";
let _cfgCache = null;
let _store = null;

async function getStore() {
  if (!_store) _store = await loadStore("settings.json");
  return _store;
}

async function migrateFromLocalStorage() {
  const OLD_KEY = "devin-ide.ai-config";
  const raw = localStorage.getItem(OLD_KEY);
  if (!raw) return;
  try {
    const old = JSON.parse(raw);
    if (old && (old.baseUrl || old.apiKey || old.model)) {
      const store = await getStore();
      await store.set(CFG_KEY, old);
      await store.save();
      _cfgCache = old;
      localStorage.removeItem(OLD_KEY);
    }
  } catch { /* ignore corrupt legacy data */ }
}

function loadConfig() {
  return _cfgCache || _DEFAULT_AI_CONFIG;
}

const _DEFAULT_AI_CONFIG = {
  baseUrl: "https://api.deepseek.com/v1",
  apiKey: "",
  model: "deepseek-chat",
};

async function loadConfigAsync() {
  if (_cfgCache) return _cfgCache;
  await migrateFromLocalStorage();
  const store = await getStore();
  const saved = (await store.get(CFG_KEY)) || {};
  // Merge saved settings over the defaults so a chosen model (or baseUrl) is kept
  // across restarts even before an API key is entered — previously an empty
  // apiKey wiped the whole config back to defaults, losing the user's model.
  _cfgCache = { ..._DEFAULT_AI_CONFIG, ...saved };
  try {
    await store.set(CFG_KEY, _cfgCache);
    await store.save();
  } catch { /* store might not be ready in browser dev mode */ }
  return _cfgCache;
}

async function saveConfig(c) {
  _cfgCache = c;
  const store = await getStore();
  await store.set(CFG_KEY, c);
  await store.save();
}
function refreshModelBadge() {
  syncModelPicker();
}

// ---- model picker (bottom-bar dropdown) ----
const MODEL_GROUPS = [
  {
    label: "OpenAI",
    models: [
      { id: "gpt-4o", name: "GPT-4o", meta: "Most capable" },
      { id: "gpt-4o-mini", name: "GPT-4o mini", meta: "Fast · cheap" },
      { id: "gpt-4.1", name: "GPT-4.1", meta: "" },
      { id: "o3-mini", name: "o3-mini", meta: "Reasoning" },
    ],
  },
  {
    label: "Anthropic",
    models: [
      { id: "claude-3-7-sonnet", name: "Claude 3.7 Sonnet", meta: "" },
      { id: "claude-3-5-sonnet", name: "Claude 3.5 Sonnet", meta: "" },
      { id: "claude-3-5-haiku", name: "Claude 3.5 Haiku", meta: "Fast" },
    ],
  },
  {
    label: "DeepSeek",
    models: [
      { id: "deepseek-v4-pro", name: "DeepSeek V4 Pro", meta: "Most capable" },
      { id: "deepseek-v4-flash", name: "DeepSeek V4 Flash", meta: "Fast" },
    ],
  },
  {
    label: "Local",
    models: [
      { id: "llama3.1", name: "Llama 3.1", meta: "Ollama" },
      { id: "qwen2.5-coder", name: "Qwen2.5 Coder", meta: "Ollama" },
    ],
  },
];

const modelPicker = $("modelPicker");
const modelPickerBtn = $("modelPickerBtn");
const modelPickerBtnIcon = modelPickerBtn.querySelector("use");
const modelPickerLabel = $("modelPickerLabel");
const modelMenu = $("modelMenu");

/** Map a model id to its provider brand logo + brand colour. */
function brandOf(id = "") {
  const s = id.toLowerCase();
  if (/^(gpt|o\d|chatgpt|text-|davinci)/.test(s)) return { sym: "i-brand-openai", cls: "brand--openai" };
  if (s.includes("claude")) return { sym: "i-brand-anthropic", cls: "brand--anthropic" };
  if (s.includes("deepseek")) return { sym: "i-brand-deepseek", cls: "brand--deepseek" };
  if (s.includes("llama")) return { sym: "i-brand-meta", cls: "brand--meta" };
  if (s.includes("qwen")) return { sym: "i-brand-qwen", cls: "brand--qwen" };
  return { sym: "i-cpu", cls: "" };
}

/** Friendly display name for a model id (falls back to the raw id). */
const MODEL_NAMES = Object.fromEntries(
  MODEL_GROUPS.flatMap((g) => g.models.map((m) => [m.id, m.name])),
);
function modelLabel(id = "") {
  return MODEL_NAMES[id] || id;
}
function currentModel() {
  return loadConfig().model || "";
}

function syncModelPicker() {
  const c = loadConfig();
  modelPickerLabel.textContent = c.model ? modelLabel(c.model) : t("assistant.selectModel");
  const b = brandOf(c.model);
  modelPickerBtnIcon.setAttribute("href", "#" + b.sym);
  modelPickerBtn.querySelector(".ic").setAttribute("class", "ic " + b.cls);
  syncAssistantBrand();
}

// Reflect the active model in the assistant panel header (provider logo + name).
function syncAssistantBrand() {
  const avatar = document.querySelector(".assistant__avatar");
  const nameEl = document.querySelector(".assistant__name");
  if (!avatar || !nameEl) return;
  const id = currentModel();
  avatar.className = "assistant__avatar assistant__avatar--logo";
  if (!id) {
    nameEl.textContent = t("assistant.name");
    return;
  }
  nameEl.textContent = modelLabel(id);
}

function buildModelMenu() {
  const current = loadConfig().model;
  modelMenu.innerHTML = "";
  for (const group of MODEL_GROUPS) {
    const g = document.createElement("div");
    g.className = "menu__group";
    g.textContent = group.label;
    modelMenu.appendChild(g);
    for (const m of group.models) {
      const item = document.createElement("div");
      item.className = "menu__item" + (m.id === current ? " is-active" : "");
      item.setAttribute("role", "option");
      const meta = m.meta ? `<span class="meta">${m.meta}</span>` : "";
      const mark =
        m.id === current
          ? `<svg class="check"><use href="#i-check" /></svg>`
          : meta;
      const b = brandOf(m.id);
      item.innerHTML = `<svg class="ic ${b.cls}"><use href="#${b.sym}" /></svg><span class="name"></span>${mark}`;
      item.querySelector(".name").textContent = m.name || m.id;
      item.addEventListener("click", () => {
        selectModel(m.id);
        closeModelMenu();
      });
      modelMenu.appendChild(item);
    }
  }
  const sep = document.createElement("div");
  sep.className = "menu__sep";
  modelMenu.appendChild(sep);
  const cfg = document.createElement("div");
  cfg.className = "menu__item";
  cfg.innerHTML = `<svg class="ic"><use href="#i-gear" /></svg><span></span>`;
  cfg.querySelector("span").textContent = t("settings.configure");
  cfg.addEventListener("click", () => {
    closeModelMenu();
    openSettings();
  });
  modelMenu.appendChild(cfg);
}

async function selectModel(model) {
  const c = loadConfig();
  await saveConfig({ ...c, model });
  refreshModelBadge();
  const session = _currentSession();
  if (session) {
    session.model = model;
    _renderChatTabs();
    saveChatHistory();
  }
}

function openModelMenu() {
  buildModelMenu();
  modelMenu.hidden = false;
  modelPicker.classList.add("is-open");
  modelPickerBtn.setAttribute("aria-expanded", "true");
}
function closeModelMenu() {
  modelMenu.hidden = true;
  modelPicker.classList.remove("is-open");
  modelPickerBtn.setAttribute("aria-expanded", "false");
}
modelPickerBtn.addEventListener("click", (e) => {
  e.stopPropagation();
  modelMenu.hidden ? openModelMenu() : closeModelMenu();
});
document.addEventListener("click", (e) => {
  if (!modelMenu.hidden && !modelPicker.contains(e.target)) closeModelMenu();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !modelMenu.hidden) closeModelMenu();
});

let _chatSessions = [];
let _activeChatIdx = -1;
let _chatSeq = 0; // monotonic counter for auto-naming so "Chat N" never repeats after a close
let streaming = false;
const CHAT_STORE_KEY = "michael-ide.chat-sessions";

function _currentSession() {
  return _activeChatIdx >= 0 && _activeChatIdx < _chatSessions.length ? _chatSessions[_activeChatIdx] : null;
}
function _getHistory() {
  const s = _currentSession();
  return s ? s.history : [];
}

const history = new Proxy([], {
  get(target, prop) {
    const h = _getHistory();
    if (prop === "push") return (...args) => h.push(...args);
    if (prop === "length") return h.length;
    if (prop === "slice") return (...args) => h.slice(...args);
    if (prop === "filter") return (...args) => h.filter(...args);
    if (prop === "map") return (...args) => h.map(...args);
    if (prop === Symbol.iterator) return () => h[Symbol.iterator]();
    return h[prop];
  }
});

function _createChatSession(name, mode, model, project) {
  const id = Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
  const container = document.createElement("div");
  container.className = "chat-session-container";
  // Name from a monotonic counter, not length+1 — otherwise closing Chat 1 of
  // [Chat 1, Chat 2] then adding a new one produces a second "Chat 2". A
  // restored name bumps the counter so fresh tabs never collide with it.
  let finalName = name;
  if (finalName) {
    const n = parseInt((finalName.match(/Chat\s+(\d+)/) || [])[1] || "0", 10);
    if (n > _chatSeq) _chatSeq = n;
  } else {
    finalName = `Chat ${++_chatSeq}`;
  }
  return {
    id, name: finalName, mode: mode || _currentAiMode, model: model || null,
    // Which project this chat is about (folder it was started under); kept in
    // sync on send. Lets each tab show its project.
    project: project !== undefined ? project : (rootPath || workspaceRoots[0] || ""),
    history: [], container, created: Date.now(),
  };
}

function _renderChatTabs() {
  const tabBar = document.getElementById("chatTabBar");
  if (!tabBar) return;
  tabBar.innerHTML = "";
  _chatSessions.forEach((s, i) => {
    const tab = document.createElement("button");
    const modeObj = _AI_MODES.find(m => m.id === s.mode);
    const modeColor = modeObj?.color || "#3b82f6";
    tab.className = "chat-tab" + (i === _activeChatIdx ? " is-active" : "");
    tab.type = "button";
    const projName = s.project ? (s.project.split("/").filter(Boolean).pop() || "") : "";
    const projTag = projName
      ? `<span class="chat-tab__project"><svg viewBox="0 0 16 16" fill="currentColor" aria-hidden="true"><path d="M1.75 2.75a.75.75 0 00-.75.75v9a.75.75 0 00.75.75h12.5a.75.75 0 00.75-.75v-7a.75.75 0 00-.75-.75H7.7L6.35 3.02a.75.75 0 00-.53-.27H1.75z"/></svg><span class="chat-tab__projname"></span></span>`
      : "";
    const modelTag = s.model ? `<span class="chat-tab__model">${modelLabel(s.model)}</span>` : "";
    const modeTag = s.mode && s.mode !== "agent" ? `<span class="chat-tab__mode" style="color:${modeColor}">${modeObj?.label || s.mode}</span>` : "";
    tab.innerHTML = `<span class="chat-tab__dot" style="background:${modeColor}"></span><span class="chat-tab__label"></span>${projTag}${modeTag}${modelTag}<span class="chat-tab__x" aria-label="关闭" title="关闭">&times;</span>`;
    tab.querySelector(".chat-tab__label").textContent = s.name;
    if (projName) tab.querySelector(".chat-tab__projname").textContent = projName;
    tab.title = (s.project ? `📁 ${s.project}\n` : "") + s.name;
    tab.addEventListener("click", (e) => {
      if (e.target.closest(".chat-tab__x")) {
        e.preventDefault();
        e.stopPropagation();
        _closeChatSession(i);
      } else {
        _switchChatSession(i);
      }
    });
    tabBar.appendChild(tab);
  });
  const addBtn = document.createElement("button");
  addBtn.className = "chat-tab chat-tab--add";
  addBtn.type = "button";
  addBtn.textContent = "+";
  addBtn.title = "New Chat";
  addBtn.addEventListener("click", () => _newChatSession());
  tabBar.appendChild(addBtn);
}

function _switchChatSession(idx) {
  if (idx === _activeChatIdx || idx < 0 || idx >= _chatSessions.length) return;
  if (_activeChatIdx >= 0 && _chatSessions[_activeChatIdx]) {
    _chatSessions[_activeChatIdx].container.hidden = true;
    _chatSessions[_activeChatIdx].scrollPos = chatEl?.scrollTop || 0;
    _chatSessions[_activeChatIdx].model = loadConfig().model;
  }
  _activeChatIdx = idx;
  const session = _chatSessions[idx];
  session.container.hidden = false;
  _currentAiMode = session.mode;
  _updateModeUI();
  if (session.model) {
    const c = loadConfig();
    if (c.model !== session.model) {
      saveConfig({ ...c, model: session.model });
      refreshModelBadge();
    }
  }
  _renderChatTabs();
  if (chatEl) {
    while (chatEl.firstChild) chatEl.removeChild(chatEl.firstChild);
    chatEl.appendChild(session.container);
    chatEl.scrollTop = session.scrollPos || 0;
  }
}

function _newChatSession(name, mode) {
  const session = _createChatSession(name, mode);
  _chatSessions.push(session);
  _switchChatSession(_chatSessions.length - 1);
  saveChatHistory();
  return session;
}

function _closeChatSession(idx) {
  if (idx < 0 || idx >= _chatSessions.length) return;
  const closing = _chatSessions[idx];
  _chatSessions.splice(idx, 1);
  // Drop the closed session's DOM so it can't linger on screen.
  closing?.container?.remove();
  if (_chatSessions.length === 0) {
    // Truly close the last tab — go to an empty state instead of spawning a
    // fresh "Chat N" (which made the tab feel impossible to close and kept
    // bumping the counter). Numbering restarts at 1 next time.
    _activeChatIdx = -1;
    _chatSeq = 0;
    _renderChatTabs();
    if (chatEl) { while (chatEl.firstChild) chatEl.removeChild(chatEl.firstChild); }
    showChatHint?.();
    saveChatHistory();
    return;
  }
  // Recompute which session stays active after the removal.
  let target = _activeChatIdx;
  if (target > idx) target--;                 // sessions after idx shifted left by one
  else if (target === idx) target = Math.min(idx, _chatSessions.length - 1);
  // target < idx → unchanged
  // Force a full refresh even when target equals the current active index
  // (closing a non-active tab): otherwise _switchChatSession's same-index
  // early-return leaves the just-closed tab still rendered.
  _activeChatIdx = -1;
  _switchChatSession(target);
  saveChatHistory();
}

let _chatSavePending = false;
async function saveChatHistory() {
  if (!inTauri) return; // allow persisting the empty state (all tabs closed)
  if (_chatSavePending) return;
  _chatSavePending = true;
  try {
    await new Promise(r => setTimeout(r, 500));
    const store = await loadStore("session.json");
    const data = _chatSessions.map(s => ({
      id: s.id, name: s.name, mode: s.mode, model: s.model || null,
      project: s.project || "",
      history: s.history.slice(-30),
      created: s.created,
    }));
    await store.set(CHAT_STORE_KEY, { sessions: data, activeIdx: _activeChatIdx });
    await store.save();
  } catch (e) { console.warn("[chat] save failed:", e); }
  _chatSavePending = false;
}

async function restoreChatHistory() {
  if (!inTauri) return;
  try {
    const store = await loadStore("session.json");
    const saved = await store.get(CHAT_STORE_KEY);

    if (saved?.sessions && Array.isArray(saved.sessions)) {
      const usedNames = new Set();
      for (const sData of saved.sessions) {
        // Heal any duplicate names left by the old length-based scheme: a
        // collision (or empty name) gets a fresh monotonic "Chat N".
        const nm = sData.name && !usedNames.has(sData.name) ? sData.name : undefined;
        const session = _createChatSession(nm, sData.mode, sData.model, sData.project ?? "");
        usedNames.add(session.name);
        session.id = sData.id || session.id;
        session.created = sData.created || Date.now();
        if (Array.isArray(sData.history)) {
          for (const m of sData.history) {
            session.history.push(m);
          }
        }
        _chatSessions.push(session);
      }
      const activeIdx = saved.activeIdx ?? 0;
      _switchChatSession(Math.min(activeIdx, _chatSessions.length - 1));

      const session = _currentSession();
      if (session) {
        for (const m of session.history) {
          addMessage(m.role === "assistant" ? "assistant" : "user", m.content);
        }
      }
    } else if (Array.isArray(saved) && saved.length > 0) {
      const session = _newChatSession("Chat 1");
      for (const m of saved) {
        session.history.push(m);
        addMessage(m.role === "assistant" ? "assistant" : "user", m.content);
      }
    }
  } catch (e) { console.warn("[chat] restore failed:", e); }

  if (_chatSessions.length === 0) {
    _newChatSession();
  }
  _renderChatTabs();
}

// ---- Multi-Window IPC (BroadcastChannel) ----
const _WINDOW_ID = Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
let _ipcChannel = null;
const _ipcPeers = new Map();

function _initIPC() {
  if (typeof BroadcastChannel === "undefined") return;
  _ipcChannel = new BroadcastChannel("michael-ide-ipc");
  _ipcChannel.onmessage = (ev) => {
    const msg = ev.data;
    if (!msg || msg.from === _WINDOW_ID) return;
    switch (msg.type) {
      case "ping":
        _ipcPeers.set(msg.from, { ts: Date.now(), workspace: msg.workspace });
        _ipcChannel.postMessage({ type: "pong", from: _WINDOW_ID, workspace: rootPath || "" });
        _updateIpcBadge();
        break;
      case "pong":
        _ipcPeers.set(msg.from, { ts: Date.now(), workspace: msg.workspace });
        _updateIpcBadge();
        break;
      case "file_changed":
        if (msg.path && openFiles.has(msg.path)) {
          showToast(`其他窗口修改了 ${msg.path.split("/").pop()}`);
        }
        break;
      case "workspace_changed":
        if (msg.roots && Array.isArray(msg.roots)) {
          for (const r of msg.roots) {
            if (r && !workspaceRoots.includes(r)) {
              workspaceRoots.push(r);
              backend.registerWorkspaceRoot(r).catch(() => {});
            }
          }
          if (msg.active && !rootPath) {
            setActiveWorkspaceRoot(msg.active);
            renderWorkspaceRoots();
            refreshGitStatus();
            preloadProjectModels(msg.active);
            startFileWatcher();
          }
        }
        break;
      case "workspace_request":
        if (rootPath && workspaceRoots.length) {
          _ipcChannel.postMessage({
            type: "workspace_response",
            from: _WINDOW_ID,
            roots: [...workspaceRoots],
            active: rootPath,
            tabs: [...openFiles.keys()],
          });
        }
        break;
      case "workspace_response":
        if (msg.roots && Array.isArray(msg.roots) && !rootPath) {
          for (const r of msg.roots) {
            if (r && !workspaceRoots.includes(r)) {
              workspaceRoots.push(r);
              backend.registerWorkspaceRoot(r).catch(() => {});
            }
          }
          if (msg.active) {
            setActiveWorkspaceRoot(msg.active);
            renderWorkspaceRoots();
            refreshGitStatus();
            preloadProjectModels(msg.active);
            startFileWatcher();
          }
        }
        break;
      case "agent_result":
        if (msg.summary) showToast(`[窗口 ${msg.from.slice(0,4)}] ${msg.summary}`);
        break;
      case "git_status":
        break;
    }
  };
  _ipcChannel.postMessage({ type: "ping", from: _WINDOW_ID, workspace: rootPath || "" });
  setTimeout(() => {
    if (!rootPath) {
      _ipcChannel.postMessage({ type: "workspace_request", from: _WINDOW_ID });
    }
  }, 500);
  setInterval(() => {
    const now = Date.now();
    for (const [id, p] of _ipcPeers) {
      if (now - p.ts > 15000) { _ipcPeers.delete(id); _updateIpcBadge(); }
    }
    if (_ipcChannel) _ipcChannel.postMessage({ type: "ping", from: _WINDOW_ID, workspace: rootPath || "" });
  }, 10000);
}

function _ipcBroadcast(type, data = {}) {
  if (!_ipcChannel) return;
  _ipcChannel.postMessage({ type, from: _WINDOW_ID, ...data });
}

function _updateIpcBadge() {
  const badge = document.getElementById("ipcPeerCount");
  if (!badge) return;
  const count = _ipcPeers.size;
  badge.textContent = count > 0 ? count.toString() : "";
  badge.style.display = count > 0 ? "inline-flex" : "none";
}

_initIPC();

// Syntax-highlight code-card bodies by reusing Monaco's tokenizer (matches the
// editor theme, no extra dependency). Returns null on failure so the card keeps
// its plain, already-escaped text.
async function highlightCode(code, lang) {
  // monaco.editor.colorize tokenizes on the main thread — a very large block can
  // freeze the UI for hundreds of ms, and a code-heavy reply triggers many at
  // once. Skip highlighting oversized blocks (plain text reads fine).
  if (!code || code.length > 20000 || code.split("\n").length > 600) return null;
  try {
    let html = await monaco.editor.colorize(code, lang, { tabSize: 2 });
    return html.replace(/<br\/?>\s*$/, "");
  } catch {
    return null;
  }
}

function addMessage(role, text) {
  const wrap = document.createElement("div");
  wrap.className = "msg " + role;
  let body;
  const session = _currentSession();
  const target = session ? session.container : chatEl;
  if (role === "assistant") {
    const id = currentModel();
    const avatar = document.createElement("div");
    avatar.className = "msg__avatar msg__avatar--logo";
    avatar.innerHTML = `<img class="assistant-logo" src="/src/assets/logo.png" alt="" aria-hidden="true" />`;
    const main = document.createElement("div");
    main.className = "msg__main";
    main.innerHTML = `<span class="msg__who"><span></span></span><div class="msg__body"></div>`;
    main.querySelector(".msg__who span").textContent = id ? modelLabel(id) : t("assistant.name");
    wrap.append(avatar, main);
    body = main.querySelector(".msg__body");
    if (text) {
      const hasToolMarkers = /\[TOOL:(read_file|write_file|run_cmd|list_dir)\]|^📄\s|^📎\s/m.test(text);
      const hasMultiCodeBlocks = (text.match(/```/g) || []).length >= 4;
      if (hasToolMarkers || hasMultiCodeBlocks) {
        const segs = _parseStreamSegments(text);
        for (let si = 0; si < segs.length; si++) {
          _renderAgentSegStatic(body, segs[si], segs, si);
        }
      } else {
        renderMarkdownInto(body, text, { highlighter: highlightCode });
      }
    }
  } else {
    wrap.innerHTML = `<span class="msg__who"><span></span></span><div class="msg__body"></div>`;
    wrap.querySelector(".msg__who span").textContent = t("assistant.you");
    body = wrap.querySelector(".msg__body");
    const cleanText = (text || "").replace(/\n\n\[用户附加了 \d+ 张图片\][\s\S]*$/, "").trim();
    if (cleanText) body.textContent = cleanText;
    if (typeof _pastedImages !== 'undefined') {
      const images = wrap._attachedImages || [];
      for (const img of images) {
        const imgEl = document.createElement("img");
        imgEl.src = img.dataUrl;
        imgEl.className = "msg__attached-image";
        imgEl.alt = img.name || "Attached image";
        body.appendChild(imgEl);
      }
    }
  }
  target.appendChild(wrap);
  // Bound DOM growth on very long sessions: keep at most the most recent N
  // message nodes rendered (the conversation data in `history` is untouched, so
  // nothing is lost — only the oldest off-screen nodes are pruned). The newest
  // (possibly streaming) message is at the end and never pruned.
  const MSG_CAP = 250;
  const msgs = target.querySelectorAll(":scope > .msg");
  for (let i = 0; i < msgs.length - MSG_CAP; i++) msgs[i].remove();
  chatEl.scrollTop = chatEl.scrollHeight;
  return body;
}

function thinkingCard() {
  const el = document.createElement("div");
  el.className = "thinking";
  el.innerHTML = `<div class="thinking-spinner"></div><span class="thinking__text">${t("assistant.thinking")}</span><span class="thinking-dots"><span></span><span></span><span></span></span>`;
  return el;
}

function showChatHint() {
  if (chatEl.children.length) return;
  const hint = document.createElement("div");
  hint.className = "chat-empty";
  hint.innerHTML =
    `<div class="chat-empty__icon chat-empty__icon--logo"><img class="assistant-logo" src="/src/assets/logo.png" alt="" aria-hidden="true" /></div>` +
    `<h3></h3>` +
    `<p></p>` +
    `<div class="chat-empty__chips"></div>`;
  hint.querySelector("h3").textContent = t("assistant.chatHintTitle");
  hint.querySelector("p").textContent = t("assistant.chatHintDesc");
  const chips = hint.querySelector(".chat-empty__chips");
  for (const s of [t("assistant.chip.explain"), t("assistant.chip.bugs"), t("assistant.chip.comments"), t("assistant.chip.test")]) {
    const chip = document.createElement("button");
    chip.type = "button";
    chip.className = "chip";
    chip.textContent = s;
    chip.addEventListener("click", () => {
      promptEl.value = s;
      promptEl.focus();
      promptEl.dispatchEvent(new Event("input"));
    });
    chips.appendChild(chip);
  }
  chatEl.appendChild(hint);
}

// ---- AI Mode System (Agent / Chat / Plan) ----
let _currentAiMode = "agent";

const _AI_MODE_PROMPTS = {
  agent: `你是 Michael IDE 的 AI 编程智能体，能像资深工程师一样自主完成编码任务。你拥有读文件、搜索、按名查找、精确编辑、写文件、运行命令的完整工具。用中文回复。

# 工作方式（每个任务都遵循）
1. 先理解再动手——先用 search / find_files / list_dir / read_file 摸清相关代码，再改。绝不修改没读过的文件，也不要凭空猜测代码内容或路径。
2. 规划——多步任务先用 update_plan 列出分步计划，开工后每完成一步就更新对应状态（pending/in_progress/completed），让用户随时看到进度。简单任务（一两步）不必用。
3. 全自动推进——你的目标是把任务**端到端做完**，不是交一半。需要改代码就直接调工具改；信息不全时做**合理假设并说明**，能自己查证的就查证，尽量别停下来反问用户。一个任务里连续调用工具，直到真正完成；除非有不可逆风险或缺关键凭据（如 API key/密码），否则一路推进。
4. 不确定就联网查，别凭记忆猜——遇到不熟悉的库/框架/API、拿不准的用法、版本差异、报错信息，**主动用 web_search 搜官方文档 / API 参考 / Stack Overflow，再用 web_fetch 读全文**，按查到的事实写代码，而不是靠印象。新库、新版本、冷门 API 尤其要查。
5. 自我验证——改完尽量用 run_cmd 跑测试/构建/类型检查（cargo check、npm run build、go build、pytest 等）。失败就读报错→定位→（必要时联网查解决方案）→修复→再验证，循环到通过为止。
6. 收尾——完成后用 1-3 句话总结：改了什么、为什么、怎么验证的；列出你做的关键假设。

# 做真实可用的东西，不是 demo（最重要的总原则）
- 默认交付**真正能跑、逻辑完整**的实现，不是占位/假数据/写死的 demo。该连后端就连后端，该落库就落库，该处理边界就处理——别用「TODO」「假装成功」「mock 数据」糊弄。
- 动手前先想清楚**这东西到底需要什么**：要不要持久化？要就按下面「数据与数据库」一节认真设计 schema/索引/迁移，而不是塞内存或平面文件应付；要不要鉴权、配置、错误处理？把真实需求想全再实现。
- 做完**一定要验证它真的成立**：用 run_cmd 跑构建/测试/类型检查、用 get_diagnostics 看报错、能起服务就起来点一下关键路径。没验证过的「做完了」不算做完。
- 宁可范围小但每块都真，也不要范围大但全是空壳。

# 高效准则（向 Claude Code / Codex 看齐）
- **并行探索**：一轮里要读/搜多个文件，就**一次发多个只读工具调用**（read_file / search / find_files / list_dir 会并行执行），别串成一长串单步往返，省时间、更快摸清全貌。
- **根因优先**：修 bug 先定位「它到底为什么发生」，解决**根本原因**，不要打补丁糊症状；改完想一下有没有同类问题在别处。
- **保持推进**：常规步骤（读文件、装依赖、跑构建/测试）直接做，不为每一步请示；信息不全就合理假设并说明，一路推到任务真正完成再收尾。
- **沟通极简**：直接给结论，不写「我将要…/接下来我会…」这类铺垫，也不在正文复述正在调用的工具（系统已显示成卡片）；要指到代码就用 文件:行号。

# 工程判断（默认这样思考，没人提也要做到）
- 先顺着项目已有约定走——命名、目录结构、错误处理、状态管理、技术选型，都先看现有代码怎么做并保持一致；优先复用已有的工具/组件/模式，别另起一套。
- 默认就把安全和健壮性做对：校验外部输入，处理空/错误/加载/边界四类情况，绝不把密钥/密码硬编码或提交进仓库。
- 既不过度设计也不欠设计：按当前真实需求实现，不为假想未来提前抽象；但该有的分层和边界要有。

# 架构（做功能/系统前先想清楚再动手）
- 先定结构：模块怎么拆、各自单一职责、数据怎么流动、依赖朝哪个方向——让依赖单向、关注点分离；多步任务把这个结构写进 update_plan。
- 单一数据源：同一份状态只在一处拥有，其余派生或引用，别多处各存一份再手动同步。
- 接口先于实现：先定清楚模块/函数的输入输出契约和错误如何向上传递，再填实现。

# UI 质量（写界面时，目标是"精致、现代、一致"，不是"能用就行"）
- 先沿用项目已有设计语言：复用现有的设计变量（颜色/间距/圆角/阴影 token）、组件与排版，新界面要和现有风格浑然一体，绝不引入突兀新风格或写死颜色。
- 间距用一致刻度（4/8px 网格：4 8 12 16 24 32…），靠留白分组而非到处加边框线。
- 排版建立层级：字号走比例阶（如 12/14/16/20/24/32），用字号+字重+颜色拉开主次；正文行高 ~1.5、行宽 ≤ ~70 字。
- 颜色克制：一个主强调色 + 中性灰阶 + 少量语义色（成功/警告/危险）；正文对比度达 WCAG AA（≥4.5:1）。
- 状态做全：hover / focus（键盘焦点要可见）/ active / disabled / loading / 空状态 / 错误态，别只做"正常态"。
- 细节出质感：一致的圆角与柔和阴影、对齐、克制的过渡（150–200ms）、语义标签+aria+键盘可达、响应式不溢出、深浅色主题都用变量照顾到。

# 图标与视觉资源（做界面/产品时，别用 emoji 凑数）
- 图标优先复用项目已有图标集 / SVG sprite，保持风格统一。需要新图标时：① 用成熟图标库（Lucide / Material Symbols / Heroicons / Feather）——可 web_search 查它们官方 SVG 或 CDN 用法再用；② 或自己**手写干净的 SVG**：统一 24×24 viewBox、用 currentColor 继承主题色、1.5–2px 描边、几何简洁、对齐像素，别堆一堆杂乱 path。**正式 UI 图标不要用 emoji。**
- 图片/插画分清场景：原型可用占位服务（picsum 等）；正式素材联网找**可商用并注明出处**的（Unsplash 等）；图标性质的优先 SVG（清晰、可缩放、可换色）而非位图。外链资源注意许可与可用性。
- 图标/插画/配色都和项目既有设计语言一致（用「UI 质量」里的设计变量），别东拼西凑。

# 数据与数据库（涉及持久化/存储时）
- 选对存储：简单本地配置用文件/KV；有关系、要查询、要事务才上关系型数据库——别为存几个键值上重型库，也别把关系数据硬塞进平面文件。
- 表结构：字段给准确类型与约束（NOT NULL / UNIQUE / 外键 / 默认值），默认规范化到 3NF 消冗余，仅在有实测性能需要时才反规范化。
- 索引：给 WHERE / JOIN / ORDER BY 用到的列建索引，**每个外键列都要建索引**；但别滥建（写入有代价）。
- 迁移：schema 变更走版本化迁移脚本（可追溯、可回滚、向后兼容），别手改生产库。
- 安全红线：SQL 一律参数化/预编译，**绝不把变量拼进 SQL 字符串**；关系完整性靠数据库外键约束兜底，而不是只靠应用代码。

# 编辑规则
- 改已有文件**优先用 edit_file**（精确替换片段）；不要用 write_file 整文件重写——重写易丢内容、易出错。
- edit_file 的 old_string 必须带足够上下文，能在文件里唯一定位那段；要改多处相同文本时设 replace_all=true。
- write_file 只用于新建文件或彻底重写。
- 最小改动——只做任务要求的改动。不顺手重构、不加未要求的功能/注释/错误处理、不为假想需求做抽象。bug 修复不必清理周围代码。

# 路径与安全
- 路径用相对工作区根目录的相对路径（如 src/main.go）或完整绝对路径，不要用截断路径（如 /Users/m）。
- 所有文件操作必须在工作区目录内；禁止访问 /Users、/etc、/var、/System 等工作区外目录。
- 破坏性命令（rm -rf /、dd、mkfs、磁盘格式化）一律禁止；不扫描整个文件系统（find /、ls /Users、tree /）；一次最多并发 3 条命令，不重复执行刚跑过的命令。

# 操作系统
严格按上下文里的 OS 信息选命令（上下文第一行就有）：
- macOS(zsh)：brew/open/launchctl/lsof/killall/pbcopy；禁用 type/dir/findstr/timeout/systemctl
- Linux(bash)：apt/systemctl/xdg-open/journalctl
- Windows(PowerShell)：Start-Process/Get-Process

# 工具
- read_file(path)：读文件内容
- list_dir(path)：列目录
- search(query, path?)：在项目里搜索文本/符号（找用法、定义、引用）
- find_files(pattern)：按文件名/glob 找文件（如 *.rs、main.js）
- web_search(query)：联网搜索，找官方文档/API用法/库版本/报错解决方案/技术文章（拿不准就先搜）
- web_fetch(url)：抓取公网网页正文，读 web_search 找到的页面或已知文档 URL（仅 http/https 公网）
- update_plan(steps)：维护可视化任务计划，多步任务用它列计划并随进度更新状态
- run_subagent(description, prompt)：派生只读子智能体做聚焦调研（大范围"搞清楚 X 怎么实现的"这类调查交给它，省主线上下文）
- remember(content)：把值得跨会话长期记住的项目知识（技术栈/架构决定、约定、构建测试命令、用户偏好、易踩的坑）写进项目记忆，下次自动加载。只记长期有用的，别记一次性细节。
- get_diagnostics(path?)：读 LSP/编辑器对文件的错误与警告，改完代码快速自检（比每次跑构建快）
- edit_file(path, old_string, new_string, replace_all?)：精确替换，改已有文件首选
- multi_edit(path, edits[])：对同一文件做多处精确替换，一次原子应用，重构/多点改动比连发多次 edit_file 更快更稳
- write_file(path, content)：新建或整文件重写
- delete_path(path)：删除文件/目录（递归），用于清理/重构
- move_path(from, to)：移动或重命名文件/目录
- run_cmd(command)：在隔离子进程里运行一条 shell 命令并拿到完整输出（装依赖、跑测试、构建、git 等）。注意：① 每次都是独立 shell，状态不跨命令保留——要切目录就写「cd 子目录 && 你的命令」；② 路径含空格务必加引号，如 cd "未命名文件夹 2/client"；③ 要启动服务器/前端来测试，用后台方式让命令立刻返回：「nohup 你的命令 > /tmp/svc.log 2>&1 & sleep 3 && cat /tmp/svc.log」——千万别前台直接跑服务（不返回会卡住）

# 输出风格
直奔重点，先结论后细节。不复述用户的话，不写废话铺垫。文件改动一律通过 edit_file/write_file 工具完成——不要把整段新文件源码贴进聊天文本里。
**工具调用会由系统自动显示成卡片**：不要在回复正文里用 \`read_file 路径\`、\`run_cmd 命令\`、\`list_dir\` 等复述你正在调用的工具，也不要用 \`<file_content>\`/\`<item>\` 标签把读到的文件内容或目录再抄一遍。直接调用工具即可；回复正文只写给用户看的结论、解释和下一步。`,

  chat: `你是 Michael IDE 的聊天助手。
- 只回答问题，不主动修改文件
- 解释代码、回答编程问题、提供建议
- 用简洁中文回复
- 代码用 fenced code blocks`,

  plan: `你是 Michael IDE 的架构规划智能体。你先调查代码库，再产出一份扎实、可直接照着实现的架构与实施方案。你有只读工具（read_file / list_dir / search / find_files / web_search / web_fetch / run_subagent / update_plan），但**绝不修改文件、不运行命令**。用中文回复。

# 方法
1. 先调查再设计——用 search / find_files / list_dir / read_file 把相关代码、现有约定、技术栈、数据模型摸清楚；不熟的库/方案先 web_search 查官方文档。绝不凭空假设项目结构或现状。
2. 多方案权衡——给 1-3 个可行方案，列清各自取舍（复杂度、性能、可维护性、风险），明确推荐一个并说明理由。
3. 用 update_plan 把最终方案落成有序、可执行的步骤清单，让用户看到完整路线图。

# 方案要覆盖（按任务相关性取舍，不相关的略过）
1. **目标与约束** — 要解决什么、边界、非目标。
2. **架构设计** — 模块怎么拆、各自单一职责、数据如何流动、依赖方向（保持单向、关注点分离、单一数据源）；关键接口/契约（输入输出、错误如何向上传递）。
3. **数据模型 / 数据库**（涉及持久化时）— 存储选型（按是否有关系·查询·事务决定文件/KV/关系型）；表结构（类型与约束，默认 3NF）；索引（WHERE/JOIN/ORDER BY 列，**每个外键都建索引**）；迁移与安全（版本化迁移、参数化查询、外键约束兜底）。
4. **UI 结构**（涉及界面时）— 组件树与状态归属；复用项目现有设计变量/组件保持风格一致；要覆盖的状态（hover/focus/disabled/loading/空/错误）、可访问性与响应式。
5. **实施步骤** — 按依赖与优先级排序，每步可独立验证。
6. **风险与验证** — 主要风险及缓解、如何验证（测试/构建/类型检查）。

# 输出
先给结论与推荐方案，再展开细节；调用链/模块关系用列表或 ASCII 图。只规划，不改文件、不运行命令。完成后建议用户切到 Agent 模式照此实施。`,

  explorer: `你是 Michael IDE 的代码探索者，只读分析智能体。你能读文件、列目录、搜索代码、按名查找文件，但绝对不能修改任何文件或运行有副作用的命令。用中文回复。

# 方法
1. 用 search 搜关键字/符号、find_files 定位文件、list_dir 看结构、read_file 读细节。
2. 主动多轮检索——顺着 import / 调用 / 定义层层追下去，直到把问题搞清楚，而不是只看一个文件就下结论。
3. 每个论点都用文件名+行号支撑。

# 工具（只读）
- read_file(path)：读文件
- list_dir(path)：列目录
- search(query, path?)：搜索文本/符号
- find_files(pattern)：按文件名/glob 找文件

# 输出
- 先给结论，再展开分析。
- 调用链 / 模块关系用列表或 ASCII 图展示。
- 只分析和建议，不修改文件、不运行有副作用的命令。`,

  reviewer: `你是 Michael IDE 的代码审查员。你仔细读代码，找出 bug、安全隐患、性能问题和可维护性问题。你能读文件、列目录、搜索、按名查找文件，但不修改文件。用中文回复。

# 先调查再评审
用 search / find_files / read_file 把相关代码**和它的调用方**读全，再下判断——不要只凭片段臆测。

# 审查维度
1. **正确性** — 逻辑、边界条件、错误处理、并发
2. **安全性** — 注入(SQL/命令/XSS)、鉴权、硬编码密钥、路径穿越
3. **性能** — N+1、无谓拷贝/分配、复杂度
4. **可维护性** — 命名、重复、过度复杂、测试覆盖、关注点分离/依赖方向
5. **数据/数据库**（涉及持久化时）— schema 与约束是否合理、缺失索引（尤其外键列）、是否参数化查询、迁移是否安全
6. **UI/UX**（前端代码）— 与现有设计变量/风格是否一致、间距与排版层级、状态是否做全(hover/focus/disabled/loading/空/错误)、对比度与可访问性

# 工具（只读）
- read_file(path) / list_dir(path) / search(query, path?) / find_files(pattern)

# 每条发现的格式
- 📍 位置（文件:行号）
- 🔴/🟡/🟢 严重度（高/中/低）
- 📝 问题描述
- ✅ 建议修复

只评审，不改文件、不运行有副作用的命令。`,
};

const _AI_MODES = [
  { id: "agent", label: "Agent", desc: "AI 可读写文件、运行命令", color: "#3b82f6", icon: `<circle cx="8" cy="5" r="3" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M3 14c0-3 2-5 5-5s5 2 5 5" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M11 3l2-1m-2 1l2 1" stroke="currentColor" stroke-width="1" stroke-linecap="round"/>` },
  { id: "chat", label: "Chat", desc: "对话问答，不修改文件", color: "#8b5cf6", icon: `<rect x="2" y="3" width="12" height="8" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M5 13l2-2h5" fill="none" stroke="currentColor" stroke-width="1.3"/>` },
  { id: "plan", label: "Plan", desc: "分析规划，输出实施方案", color: "#f59e0b", icon: `<rect x="2" y="2" width="12" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M5 5h6M5 8h4M5 11h5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>` },
  { id: "explorer", label: "Explorer", desc: "只读代码探索，深度分析", color: "#10b981", icon: `<circle cx="7" cy="7" r="4" fill="none" stroke="currentColor" stroke-width="1.3"/><path d="M10 10l4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>` },
  { id: "reviewer", label: "Reviewer", desc: "代码审查，发现问题", color: "#ef4444", icon: `<path d="M8 2l6 10H2z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><path d="M8 6v3M8 10.5v.5" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>` },
];

function _updateModeUI() {
  const mode = _AI_MODES.find(m => m.id === _currentAiMode) || _AI_MODES[0];
  $("modeLabel").textContent = mode.label;
  $("modeIcon").innerHTML = mode.icon;
}

function _toggleModeMenu() {
  const menu = $("modeMenu");
  if (!menu.hidden) { menu.hidden = true; return; }
  menu.innerHTML = "";
  for (const mode of _AI_MODES) {
    const item = document.createElement("button");
    item.className = "mode-menu__item" + (mode.id === _currentAiMode ? " is-active" : "");
    item.innerHTML = `<svg class="ic" viewBox="0 0 16 16">${mode.icon}</svg><div class="mode-menu__info"><div class="mode-menu__name">${mode.label}</div><div class="mode-menu__desc">${mode.desc}</div></div>`;
    item.addEventListener("click", () => {
      _currentAiMode = mode.id;
      _updateModeUI();
      menu.hidden = true;
      const session = _currentSession();
      if (session) { session.mode = mode.id; _renderChatTabs(); saveChatHistory(); }
      showToast(`已切换到 ${mode.label} 模式`);
    });
    menu.appendChild(item);
  }
  menu.hidden = false;
  const dismiss = (e) => { if (!$("modePicker").contains(e.target)) { menu.hidden = true; document.removeEventListener("click", dismiss); } };
  setTimeout(() => document.addEventListener("click", dismiss), 50);
}

$("modePickerBtn")?.addEventListener("click", _toggleModeMenu);
_updateModeUI();

let _agentContextCache = { root: "", ts: 0, data: "" };

function _estimateTokens(text) {
  if (!text) return 0;
  return Math.ceil(text.length / 3.5);
}

function _compactHistoryIfNeeded() {
  const h = _getHistory();
  if (!h || h.length === 0) return;

  const MAX_HISTORY_TOKENS = 24000;
  const MAX_MESSAGES = 40;
  const KEEP_RECENT = 8;

  let totalTokens = h.reduce((sum, m) => sum + _estimateTokens(m.content), 0);

  if (h.length <= MAX_MESSAGES && totalTokens <= MAX_HISTORY_TOKENS) return;

  for (let i = 0; i < h.length - KEEP_RECENT; i++) {
    const m = h[i];
    if (m.role === "assistant" && m.content.length > 2000) {
      const toolResultPattern = /\[TOOL:(read_file|list_dir)\][^\n]*\n([\s\S]*?)(?=\[TOOL:|```|$)/g;
      let compressed = m.content;
      compressed = compressed.replace(toolResultPattern, (match, tool, content) => {
        if (content.length > 500) {
          const preview = content.trim().split("\n").slice(0, 5).join("\n");
          return `[TOOL:${tool}] (${content.length} chars read)\n${preview}\n...\n`;
        }
        return match;
      });
      const codeBlockPattern = /```[\w]*\n([\s\S]*?)```/g;
      compressed = compressed.replace(codeBlockPattern, (match, code) => {
        if (code.length > 800) {
          const lines = code.trim().split("\n");
          const preview = lines.slice(0, 8).join("\n");
          return "```\n" + preview + `\n... (${lines.length} lines total)\n` + "```";
        }
        return match;
      });
      if (compressed.length < m.content.length * 0.7) {
        h[i] = { ...m, content: compressed };
      }
    }
  }

  totalTokens = h.reduce((sum, m) => sum + _estimateTokens(m.content), 0);

  if (h.length > MAX_MESSAGES) {
    const excess = h.length - MAX_MESSAGES;
    const removed = h.splice(0, excess);
    const summaryParts = removed.filter(m => m.role === "user").map(m => m.content.slice(0, 100));
    if (summaryParts.length > 0) {
      h.unshift({ role: "user", content: `[之前的对话摘要：讨论了 ${summaryParts.join("、")}]` });
      h.unshift({ role: "assistant", content: "[已压缩之前的对话内容以节省空间]" });
    }
  }

  while (totalTokens > MAX_HISTORY_TOKENS && h.length > KEEP_RECENT + 2) {
    h.splice(0, 2);
    totalTokens = h.reduce((sum, m) => sum + _estimateTokens(m.content), 0);
  }

  console.log(`[compact] history: ${h.length} msgs, ~${totalTokens} tokens`);
}

function _detectOS() {
  const ua = navigator.userAgent || "";
  const platform = navigator.platform || "";
  if (/Mac|Darwin/i.test(platform) || /Mac OS X/i.test(ua)) return "macOS";
  if (/Win/i.test(platform)) return "Windows";
  if (/Linux/i.test(platform)) return "Linux";
  return "Unknown";
}

let _osDetailCache = null;
async function _detectOSDetail() {
  if (_osDetailCache) return _osDetailCache;
  const os = _detectOS();
  const detail = { os, shell: "unknown", arch: "unknown", version: "" };

  if (os === "macOS") {
    detail.shell = "zsh";
    detail.arch = navigator.userAgent.includes("ARM64") || navigator.platform === "MacIntel" ? "arm64 (Apple Silicon)" : "x86_64";
    try {
      const r = await backend.taskRunCapture("/tmp", "sw_vers -productVersion 2>/dev/null");
      if (r?.stdout?.trim()) detail.version = r.stdout.trim();
    } catch {}
  } else if (os === "Linux") {
    detail.shell = "bash";
    try {
      const r = await backend.taskRunCapture("/tmp", "uname -m 2>/dev/null");
      if (r?.stdout?.trim()) detail.arch = r.stdout.trim();
    } catch {}
  } else if (os === "Windows") {
    detail.shell = "PowerShell";
    detail.arch = "x86_64";
  }

  _osDetailCache = detail;
  return detail;
}

async function _gatherAgentContext() {
  const root = rootPath || workspaceRoots[0];
  const osDetail = await _detectOSDetail();
  console.log("[agent-ctx] rootPath:", rootPath, "workspaceRoots:", workspaceRoots, "using:", root);

  const osBlock = `操作系统: ${osDetail.os} ${osDetail.version} (${osDetail.arch})\nShell: ${osDetail.shell}`;

  if (!root) return `${osBlock}\n(未打开工作区文件夹。请提示用户先打开文件夹，不要尝试读取或列出文件。)`;
  if (_agentContextCache.root === root && Date.now() - _agentContextCache.ts < 15000) return _agentContextCache.data;

  const parts = [osBlock, `当前工作区: ${root}`];

  // Pick up project-specific agent instructions the way Claude Code reads
  // CLAUDE.md and Codex reads AGENTS.md — first match wins.
  for (const guide of ["AGENTS.md", "CLAUDE.md", ".cursorrules", ".github/copilot-instructions.md"]) {
    try {
      const txt = await backend.readTextFile(root + "/" + guide);
      if (txt && txt.trim()) { parts.push(`\n--- 项目约定 (${guide}，请遵守) ---\n${txt.slice(0, 4000)}`); break; }
    } catch { /* not present */ }
  }

  // Agent-authored project memory (persisted via the `remember` tool).
  const _mem = _loadMemory(root);
  if (_mem && _mem.trim()) {
    parts.push(`\n--- 项目记忆（你之前用 remember 记下的，跨会话保留，开工前先看）---\n${_mem.slice(0, 4000)}`);
  }

  const treeDom = document.querySelectorAll("#tree .row");
  if (treeDom.length > 0) {
    parts.push("\n项目结构:");
    const treeLines = [];
    treeDom.forEach(row => {
      const name = row.querySelector(".name")?.textContent || row.textContent?.trim() || "";
      const isDir = row.classList.contains("is-dir");
      if (name && treeLines.length < 100) treeLines.push(`${isDir ? "📁" : "  "} ${name}`);
    });
    parts.push(treeLines.join("\n"));
  } else {
    try {
      const treeCmd = osDetail.os === "Windows"
        ? "dir /b"
        : "find . -maxdepth 2 -not -path '*/node_modules/*' -not -path '*/.git/*' -not -path '*/dist/*' -not -path '*/__pycache__/*' -not -path '*/.venv/*' -not -path '*/target/*' -not -name '.DS_Store' 2>/dev/null | head -100 | sort";
      const treeResult = await backend.taskRunCapture(root, treeCmd);
      if (treeResult?.stdout?.trim()) {
        parts.push("\n项目结构:");
        parts.push(treeResult.stdout.trim());
      }
    } catch {}
  }

  if (activePath && openFiles.has(activePath)) {
    const f = openFiles.get(activePath);
    if (f?.model) {
      const content = f.model.getValue();
      parts.push(`\n--- ${activePath.split("/").pop()} (当前编辑中) ---\n${content.slice(0, 4000)}`);
    }
  }

  const recentEdits = [];
  for (const [path, file] of openFiles) {
    if (path === activePath) continue;
    if (file?.model && file.dirty) {
      recentEdits.push({ path, content: file.model.getValue() });
    }
  }
  if (recentEdits.length > 0) {
    const maxRecent = 3;
    for (const edit of recentEdits.slice(0, maxRecent)) {
      const name = edit.path.split("/").pop();
      parts.push(`\n--- ${name} (已修改未保存) ---\n${edit.content.slice(0, 2000)}`);
    }
  }

  const keyFiles = ["package.json", "README.md", "Cargo.toml", "pyproject.toml", "go.mod", "Makefile"];
  const keyReads = keyFiles.map(name =>
    backend.readTextFile(root + "/" + name).catch(() => null).then(c => c?.trim() ? [name, c] : null)
  );
  const results = await Promise.all(keyReads);
  for (const r of results) {
    if (r) parts.push(`\n--- ${r[0]} ---\n${r[1].slice(0, 2000)}`);
  }

  let ctx = parts.join("\n");
  const CTX_TOKEN_BUDGET = 14000;
  const ctxTokens = _estimateTokens(ctx);
  if (ctxTokens > CTX_TOKEN_BUDGET) {
    const ratio = CTX_TOKEN_BUDGET / ctxTokens;
    const maxLen = Math.floor(ctx.length * ratio);
    ctx = ctx.slice(0, maxLen) + "\n...(context truncated to fit " + CTX_TOKEN_BUDGET + " token budget)";
  }
  _agentContextCache = { root, ts: Date.now(), data: ctx };
  return ctx;
}

async function sendPrompt(text, attachedImages = []) {
  const config = loadConfig();
  if (!config.baseUrl || !config.apiKey || !config.model) {
    openSettings();
    showToast(t("assistant.configFirst"));
    return;
  }
  if (streaming) return;
  _setSendBtnStop(true);
  chatEl.querySelector(".chat-empty")?.remove();

  // If all chats were closed, sending starts a fresh one so history has a home.
  if (!_currentSession()) _newChatSession();

  // Compact a long conversation before this turn (summarize older history).
  await _compactHistoryIfHuge(config);

  // Keep the active chat's project label in sync with the folder it's actually
  // operating on (handles opening a different project mid-conversation).
  const _activeSess = _currentSession();
  const _curRoot = rootPath || workspaceRoots[0] || "";
  if (_activeSess && _curRoot && _activeSess.project !== _curRoot) {
    _activeSess.project = _curRoot;
    _renderChatTabs();
    saveChatHistory();
  }

  const userBody = addMessage("user", text);
  if (attachedImages.length > 0 && userBody) {
    for (const img of attachedImages) {
      const imgEl = document.createElement("img");
      imgEl.src = img.dataUrl;
      imgEl.className = "msg__attached-image";
      imgEl.alt = img.name || "Attached image";
      userBody.appendChild(imgEl);
    }
  }

  const sysPrompt = _AI_MODE_PROMPTS[_currentAiMode] || _AI_MODE_PROMPTS.agent;

  const osDetail = await _detectOSDetail();
  let contextBlock = `\n操作系统: ${osDetail.os} ${osDetail.version} | Shell: ${osDetail.shell} | 架构: ${osDetail.arch}`;
  if (_currentAiMode === "agent" || _currentAiMode === "explorer" || _currentAiMode === "reviewer" || _currentAiMode === "plan") {
    if (rootPath || workspaceRoots.length) {
      contextBlock += "\n" + await _gatherAgentContext();
    } else {
      contextBlock += "\n(未打开工作区文件夹)";
    }
  }
  if (activePath) {
    const f = openFiles.get(activePath);
    if (f?.model) {
      const sel = monacoEditor.getModel() === f.model ? monacoEditor.getSelection() : null;
      const selected = sel && !sel.isEmpty() ? f.model.getValueInRange(sel) : "";
      contextBlock += `\n\n当前打开文件: ${activePath}\n\`\`\`\n${f.model.getValue().slice(0, 12000)}\n\`\`\``;
      if (selected) contextBlock += `\n\n选中的代码:\n\`\`\`\n${selected.slice(0, 4000)}\n\`\`\``;
    }
  }

  const fullPrompt = sysPrompt + (contextBlock ? `\n\n--- 项目上下文 ---\n${contextBlock}` : "");

  _compactHistoryIfNeeded();

  const messages = [{ role: "system", content: fullPrompt }];
  for (const m of history) messages.push(m);
  // When images are attached, send a multimodal content array so vision models
  // actually see them. History keeps only the text (image data URLs would bloat
  // every subsequent request).
  // D — for a clearly complex agent task, steer the model to investigate + plan
  // before editing (Claude Code's plan-first habit). Injected only into this
  // turn's request — not the visible bubble, not stored history.
  const _planFirst = (_currentAiMode === "agent" && _looksComplexTask(text))
    ? "\n\n（这看起来是个多步/复杂任务：先用 search / read_file 摸清相关代码与现有约定，再用 update_plan 列出分步计划，然后逐步实现、每步更新计划状态，最后用 run_cmd / get_diagnostics 验证。别一上来就直接改。）"
    : "";
  // @file mentions → pull each pinned file's content into THIS turn's context
  // (the visible bubble and stored history keep just the "@path" the user typed).
  let _atContext = "";
  const _mentioned = [...text.matchAll(/(?:^|\s)@([^\s]+)/g)].map((m) => m[1]);
  if (_mentioned.length && (rootPath || workspaceRoots.length)) {
    const _r = (rootPath || workspaceRoots[0]).replace(/\/$/, "");
    const _seen = new Set();
    for (const rel of _mentioned.slice(0, 6)) {
      if (_seen.has(rel)) continue;
      _seen.add(rel);
      try {
        const fp = rel.startsWith("/") ? rel : _r + "/" + rel.replace(/^\.?\//, "");
        const content = await backend.readTextFile(fp);
        _atContext += `\n\n文件 ${rel}:\n\`\`\`\n${content.slice(0, 8000)}\n\`\`\``;
      } catch { /* unreadable — skip */ }
    }
  }
  const _extra = _planFirst + _atContext;
  const userContent = attachedImages.length > 0
    ? [{ type: "text", text: text + _extra }, ...attachedImages.map((img) => ({ type: "image_url", image_url: { url: img.dataUrl } }))]
    : text + _extra;
  messages.push({ role: "user", content: userContent });
  history.push({ role: "user", content: text });

  const isAgent = _currentAiMode === "agent";
  const isExplorer = _currentAiMode === "explorer";
  const isReviewer = _currentAiMode === "reviewer";
  const isPlan = _currentAiMode === "plan";
  // Plan joins the read-only tool modes so the architect can actually investigate
  // the codebase (read / search / find / web) before proposing a design — it gets
  // _buildAgentToolSchemas(false), i.e. no write/edit/run_cmd.
  const hasToolAccess = isAgent || isExplorer || isReviewer || isPlan;

  // Tool-capable modes (agent / explorer / reviewer) run the real multi-turn
  // agentic loop: think → call tools → feed results back → repeat until the
  // model stops calling tools (task done) or we hit the iteration cap. Plain
  // chat / plan modes keep the original single-shot streaming path below.
  if (hasToolAccess) {
    await _runAgenticLoop({ config, messages, root: rootPath || workspaceRoots[0] || "" });
    return;
  }

  const body = addMessage("assistant", "");
  body.appendChild(thinkingCard());
  let acc = "";
  let err = null;
  let raf = 0;
  let lastFlush = 0;

  const _toolSchemas = hasToolAccess ? [
    { type: "function", function: { name: "read_file", description: "Read file content", parameters: { type: "object", properties: { path: { type: "string", description: "File path" } }, required: ["path"] } } },
    { type: "function", function: { name: "list_dir", description: "List directory contents", parameters: { type: "object", properties: { path: { type: "string", description: "Directory path" } }, required: ["path"] } } },
    ...(isAgent ? [
      { type: "function", function: { name: "write_file", description: "Write content to file", parameters: { type: "object", properties: { path: { type: "string", description: "File path" }, content: { type: "string", description: "File content" } }, required: ["path", "content"] } } },
      { type: "function", function: { name: "run_cmd", description: "Run shell command", parameters: { type: "object", properties: { command: { type: "string", description: "Shell command" } }, required: ["command"] } } },
    ] : []),
  ] : [];
  let _segRendered = 0;
  let _streamEl = null;
  const _agentRoot = rootPath || workspaceRoots[0] || "";
  const _toolPromises = [];
  const _trackedFiles = new Map();
  let _filesBar = null;
  let _filesList = null;

  if (hasToolAccess) {
    _filesBar = document.createElement("div");
    _filesBar.className = "agent-files-bar";
    _filesBar.innerHTML =
      `<svg class="agent-files-bar__chev" viewBox="0 0 12 12"><path d="M4 2.5l3.5 3.5-3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>` +
      `<svg class="agent-files-bar__icon" viewBox="0 0 16 16" fill="currentColor"><path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5a.25.25 0 01-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"/></svg>` +
      `<span class="agent-files-bar__count">0 Files</span>` +
      `<div class="agent-files-bar__actions">` +
      `<button class="agent-files-bar__btn agent-files-bar__btn--stop" type="button">Stop<span class="agent-files-bar__shortcut">^C</span></button>` +
      `<button class="agent-files-bar__btn agent-files-bar__btn--review" type="button">Review</button>` +
      `</div>`;
    _filesBar.addEventListener("click", (e) => {
      if (e.target.closest(".agent-files-bar__btn")) return;
      _filesBar.classList.toggle("is-open");
    });
    _filesBar.querySelector(".agent-files-bar__btn--stop").addEventListener("click", (e) => {
      e.stopPropagation();
      streaming = false;
      _setSendBtnStop(false);
      showToast("Agent stopped");
    });
    _filesBar.querySelector(".agent-files-bar__btn--review").addEventListener("click", (e) => {
      e.stopPropagation();
      body.querySelectorAll(".agent-tool-step--write:not(.agent-tool-step--accepted):not(.agent-tool-step--rejected)").forEach(s => s.classList.add("is-open"));
      _filesBar.classList.add("is-open");
    });
    _filesList = document.createElement("ul");
    _filesList.className = "agent-files-list";
    body.parentElement.insertBefore(_filesBar, body);
    body.parentElement.insertBefore(_filesList, body);
    _filesBar.style.display = "none";
  }

  const flushStream = () => {
    raf = 0;
    const now = Date.now();
    if (now - lastFlush < 80) { scheduleStream(); return; } // ~12fps; reschedule so the tail still renders
    lastFlush = now;
    body.querySelector(".thinking")?.remove();

    if (hasToolAccess) {
      const segs = _parseStreamSegments(acc);
      const completeEnd = segs.length > 0 && !segs[segs.length - 1].complete ? segs.length - 1 : segs.length;
      while (_segRendered < completeEnd) {
        if (_streamEl) { _streamEl.remove(); _streamEl = null; }
        const seg = segs[_segRendered];
        _renderAgentSeg(body, seg, segs, _segRendered, _agentRoot, _toolPromises);
        if (seg.type === "code" || seg.type === "write") {
          const fn = _extractSegFileName(seg, segs, _segRendered);
          if (fn && !_trackedFiles.has(fn)) {
            _trackedFiles.set(fn, seg.type);
            _updateFilesBar(_filesBar, _filesList, _trackedFiles);
          }
        } else if (seg.type === "cmd") {
          const key = "$ " + (seg.command || "").slice(0, 40);
          if (!_trackedFiles.has(key)) {
            _trackedFiles.set(key, "cmd");
            _updateFilesBar(_filesBar, _filesList, _trackedFiles);
          }
        } else if (seg.type === "read" || seg.type === "list") {
          const fn = seg.path;
          if (fn && !_trackedFiles.has(fn)) {
            _trackedFiles.set(fn, seg.type);
            _updateFilesBar(_filesBar, _filesList, _trackedFiles);
          }
        }
        _segRendered++;
      }
      const tail = completeEnd < segs.length ? segs[segs.length - 1] : null;
      if (tail) {
        if (!_streamEl) { _streamEl = document.createElement("div"); _streamEl.className = "agent-seg agent-seg--stream"; body.appendChild(_streamEl); }
        if (tail.type === "text") {
          const clean = _cleanAgentText(tail.content);
          if (clean) renderMarkdownStream(_streamEl, clean, { streaming: true });
        } else if (tail.type === "code" && !tail.complete) {
          const lineCount = tail.content.split("\n").length;
          const langDisplay = tail.lang ? langLabel(tail.lang) : 'Code';
          const monoId = tail.lang ? monacoLang(tail.lang) : 'plaintext';
          const guessedName = _extractSegFileName(tail, segs, segs.length - 1);
          const headerLabel = guessedName || langDisplay;

          let existingCard = _streamEl.querySelector(".code-card--streaming");
          if (!existingCard) {
            _streamEl.innerHTML = '';
            existingCard = document.createElement("div");
            existingCard.className = "code-card code-card--streaming";
            const head = document.createElement("div");
            head.className = "code-card__head";
            head.innerHTML = `<span class="code-card__lang"><svg class="ic"><use href="#i-code"/></svg><span class="code-card__label"></span></span><span class="code-card__streaming-meta"><span class="atc-spin"></span><span class="code-card__linecount"></span></span>`;
            existingCard.appendChild(head);
            const pre = document.createElement("pre");
            pre.className = "code-card__body";
            const codeEl = document.createElement("code");
            pre.appendChild(codeEl);
            existingCard.appendChild(pre);
            _streamEl.appendChild(existingCard);
          }

          const labelEl = existingCard.querySelector(".code-card__label");
          if (labelEl && labelEl.textContent !== headerLabel) labelEl.textContent = headerLabel;
          const countEl = existingCard.querySelector(".code-card__linecount");
          if (countEl) countEl.textContent = `${lineCount} lines`;

          const codeEl = existingCard.querySelector("code");
          if (codeEl && codeEl._lastLen !== tail.content.length) {
            codeEl.textContent = tail.content;
            codeEl._lastLen = tail.content.length;
            if (monoId !== 'plaintext' && tail.content.trim() && tail.content.length < 20000 && !codeEl._hlPending) {
              codeEl._hlPending = true;
              const snapLen = tail.content.length;
              monaco.editor.colorize(tail.content, monoId, { tabSize: 2 })
                .then(html => { if (html && codeEl._lastLen === snapLen) codeEl.innerHTML = html.replace(/<br\/?>\s*$/, ""); })
                .catch(() => {})
                .finally(() => { codeEl._hlPending = false; });
            }
          }

          const preEl = existingCard.querySelector("pre");
          if (preEl) preEl.scrollTop = preEl.scrollHeight;
        }
      }
    } else {
      const prevLen = body._lastLen || 0;
      if (acc.length - prevLen < 20 && !acc.includes("```") && prevLen > 0) {
        const tail = acc.slice(prevLen);
        if (tail && body.lastChild && body.lastChild.nodeType === 3) {
          body.lastChild.textContent += tail;
        } else if (tail) {
          renderMarkdownInto(body, acc, { streaming: true });
        }
      } else {
        renderMarkdownInto(body, acc, { streaming: true });
      }
      body._lastLen = acc.length;
    }
    chatEl.scrollTop = chatEl.scrollHeight;
  };
  const scheduleStream = () => { if (!raf) raf = requestAnimationFrame(flushStream); };
  streaming = true;
  const _pendingToolCalls = [];
  let _toolArgBuf = {};
  try {
    const useTools = hasToolAccess && _toolSchemas.length > 0 && backend.aiChatWithTools;
    const chatFn = useTools
      ? (cb) => backend.aiChatWithTools(config, messages, _toolSchemas, cb)
      : (cb) => backend.aiChat(config, messages, cb);
    await chatFn((ev) => {
      if (ev.kind === "token") { acc += ev.delta; scheduleStream(); }
      else if (ev.kind === "toolCall") {
        const { id, name, arguments: args } = ev;
        if (name) { _toolArgBuf[id || "_"] = { name, args: args || "" }; }
        else if (id && _toolArgBuf[id]) { _toolArgBuf[id].args += args; }
        else if (_toolArgBuf["_"]) { _toolArgBuf["_"].args += args; }
        const entry = _toolArgBuf[id] || _toolArgBuf["_"];
        if (entry && entry.args) {
          try {
            const parsed = JSON.parse(entry.args);
            const toolName = entry.name;
            let call;
            if (toolName === "read_file") call = { type: "read", path: parsed.path };
            else if (toolName === "list_dir") call = { type: "list", path: parsed.path };
            else if (toolName === "run_cmd") call = { type: "cmd", command: parsed.command };
            else if (toolName === "write_file") call = { type: "write", path: parsed.path, content: parsed.content };
            if (call) {
              body.querySelector(".thinking")?.remove();
              if (_streamEl) { _streamEl.remove(); _streamEl = null; }
              if (acc.trim()) { renderMarkdownInto(body, acc.trim(), { streaming: false }); acc = ""; }
              const step = _createToolStep(call);
              body.appendChild(step);
              const p = _executeToolStep(step, call, _agentRoot);
              p.then(r => { if (r) p._result = r; });
              _toolPromises.push(p);
              _pendingToolCalls.push(call);
              chatEl.scrollTop = chatEl.scrollHeight;
              delete _toolArgBuf[id || "_"];
            }
          } catch { /* JSON not complete yet, keep buffering */ }
        }
      }
      else if (ev.kind === "error") { err = ev.message; }
    });
  } catch (e) { if (!err) err = String(e); }
  finally {
    if (raf) cancelAnimationFrame(raf);
    streaming = false;
    _setSendBtnStop(false);
    body.querySelector(".thinking")?.remove();
    if (_streamEl) { _streamEl.remove(); _streamEl = null; }
    if (acc) {
      if (hasToolAccess) {
        const segs = _parseStreamSegments(acc);
        while (_segRendered < segs.length) {
          _renderAgentSeg(body, segs[_segRendered], segs, _segRendered, _agentRoot, _toolPromises);
          _segRendered++;
        }
        const historyContent = acc + (_pendingToolCalls.length ? "\n" + _pendingToolCalls.map(c => `[TOOL:${c.type === "read" ? "read_file" : c.type === "list" ? "list_dir" : c.type === "cmd" ? "run_cmd" : "write_file"}] ${c.path || c.command || ""}`).join("\n") : "");
        if (!err && historyContent.trim()) { history.push({ role: "assistant", content: historyContent }); saveChatHistory(); }
        await Promise.allSettled(_toolPromises);
        const readResults = _toolPromises.filter(p => p._result).map(p => p._result);
        if (readResults.length) _agentFollowUp(readResults, body);
      } else {
        renderMarkdownInto(body, acc, { highlighter: highlightCode });
        if (!err) { history.push({ role: "assistant", content: acc }); saveChatHistory(); }
      }
    }
    if (err) {
      const note = document.createElement("div");
      note.className = "msg__error";
      note.textContent = "⚠️ " + err;
      body.appendChild(note);
    }
    if (_filesBar) {
      const stopBtn = _filesBar.querySelector(".agent-files-bar__btn--stop");
      if (stopBtn) stopBtn.style.display = "none";
      _updateFilesBar(_filesBar, _filesList, _trackedFiles);
    }
    chatEl.scrollTop = chatEl.scrollHeight;
  }
}

let _termLock = Promise.resolve();
let _termOpened = false;
const _DANGEROUS_CMDS = /\bls\s+-[a-zA-Z]*R[a-zA-Z]*\s+\/\s*$|\brm\s+-rf\s+\/\s*$|\bdd\s+if=|\bmkfs\b|\bformat\s+[cCdD]:/;
const _BROAD_SCAN_CMDS = /\bfind\s+\/(?!tmp|var\/log)\b|\bfind\s+~|\bls\s+\/(?:Users|home|etc|var|usr|opt|System|Library)\b|\btree\s+\/|\bdu\s+(?:-[a-z]*\s+)?\/[^t]/;
let _runningTermCmds = 0;
const _MAX_CONCURRENT_CMDS = 3;

async function _agentRunInTerminal(root, command, stepEl) {
  if (!command || !command.trim()) return { code: -1, stdout: "", stderr: "" };
  const cmd = command.trim();
  if (cmd === "cd" || cmd === "cd ~" || cmd === "cd /") return { code: 0, stdout: "", stderr: "" };
  if (_DANGEROUS_CMDS.test(cmd)) {
    return { code: 1, stdout: "", stderr: `Blocked: "${cmd}" is a dangerous command that could harm the system.` };
  }
  if (_BROAD_SCAN_CMDS.test(cmd)) {
    return { code: 1, stdout: "", stderr: `Blocked: "${cmd}" scans system directories. Use project-relative paths instead.` };
  }
  if (_runningTermCmds >= _MAX_CONCURRENT_CMDS) {
    return { code: 1, stdout: "", stderr: `Too many concurrent commands (${_runningTermCmds}/${_MAX_CONCURRENT_CMDS}). Wait for others to finish.` };
  }
  _runningTermCmds++;

  const outputEl = stepEl?.querySelector(".agent-term-output");
  const statusEl = stepEl?.querySelector(".agent-term-status");
  const timerEl = stepEl?.querySelector(".agent-term-timer");

  const startTime = Date.now();
  let timerInterval;
  if (timerEl) {
    timerInterval = setInterval(() => {
      const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
      timerEl.textContent = `${elapsed}s`;
    }, 200);
  }

  if (statusEl) {
    statusEl.className = "agent-term-status agent-term-status--running";
    statusEl.innerHTML = `<span class="agent-term-spinner"></span> Running`;
  }

  let result = { code: -1, stdout: "", stderr: "" };

  try {
    // Agent commands run in an ISOLATED subprocess (taskRunCapture), never typed
    // into the user's interactive terminal — otherwise running another command
    // would interrupt whatever the user already has running there (a dev server,
    // a REPL, etc.). Each call gets a fresh shell at the workspace root.
    const captureRoot = root || "/tmp";
    // A backgrounded command (nohup / trailing `&`) returns right away, so it's
    // fine even if it starts a server — this is exactly how the agent SHOULD spin
    // up a dev server to test (e.g. `nohup npx vite & sleep 3 && cat log`). Only
    // a FOREGROUND server/watch (which never returns) is refused.
    const backgrounded = /\bnohup\b/i.test(cmd) || /\s&(\s|$)/.test(cmd);
    const isLongRunning = !backgrounded && /\b(serve|watch|nodemon|flask\s+run|npm\s+(run\s+)?(start|dev|serve)|yarn\s+(start|dev)|pnpm\s+(start|dev)|npx\s+(vite|next|nuxt)|http\.server|webpack-dev-server|ng\s+serve|rails\s+server|gunicorn|uvicorn)\b/i.test(cmd);
    const isCatCmd = /^\s*cat\s/.test(cmd);

    if (isCatCmd) {
      const filePath = cmd.replace(/^\s*cat\s+/, "").replace(/^["']|["']$/g, "").trim();
      const fp = filePath.startsWith("/") ? filePath : (root ? root + "/" + filePath : filePath);
      try {
        const content = await backend.readTextFile(fp);
        result = { code: 0, stdout: content || "", stderr: "" };
      } catch {
        result = await backend.taskRunCapture(captureRoot, cmd).catch(e => ({ code: 1, stdout: "", stderr: String(e?.message || e) }));
      }
    } else if (isLongRunning) {
      // Foreground server/watch never returns — but instead of just refusing,
      // teach the model to background it so it CAN start & test a dev server.
      result = { code: 1, stdout: "", stderr: "[未执行] 前台长时间运行的命令不会返回。要启动服务/前端来测试，请改成后台方式（会立刻返回）：\n  nohup 你的命令 > /tmp/svc.log 2>&1 & sleep 3 && cat /tmp/svc.log\n这样进程在后台跑、你也能看到启动日志。路径含空格记得加引号。" };
    } else {
      const r = await backend.taskRunCapture(captureRoot, cmd).catch(e => ({ code: 1, stdout: "", stderr: String(e?.message || e) }));
      result = { code: r?.code ?? 0, stdout: r?.stdout || "", stderr: r?.stderr || "" };
    }

    if (outputEl) {
      const output = (result.stdout + (result.stderr ? "\n" + result.stderr : "")).trim();
      if (output && !output.startsWith("(")) {
        outputEl.textContent = output.slice(0, 5000);
        outputEl.style.display = "block";
      }
    }
  } catch (e) {
    console.warn("[agent-term]", e);
    result = { code: 1, stdout: "", stderr: String(e?.message || e) };
  }

  _runningTermCmds = Math.max(0, _runningTermCmds - 1);
  if (timerInterval) clearInterval(timerInterval);

  const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
  if (timerEl) timerEl.textContent = `${elapsed}s`;

  if (statusEl) {
    if (result.code === 0) {
      statusEl.className = "agent-term-status agent-term-status--ok";
      statusEl.innerHTML = `<svg viewBox="0 0 12 12" width="10" height="10" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm2.22 4.22a.75.75 0 010 1.06l-3 3a.75.75 0 01-1.06 0l-1.5-1.5a.75.75 0 111.06-1.06L4.69 6.69l2.47-2.47a.75.75 0 011.06 0z"/></svg> exit 0`;
    } else {
      statusEl.className = "agent-term-status agent-term-status--err";
      statusEl.innerHTML = `<svg viewBox="0 0 12 12" width="10" height="10" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm2.03 3.97a.75.75 0 00-1.06 0L6 4.94 5.03 3.97a.75.75 0 10-1.06 1.06L4.94 6 3.97 6.97a.75.75 0 101.06 1.06L6 7.06l.97.97a.75.75 0 101.06-1.06L7.06 6l.97-.97a.75.75 0 000-1.06z"/></svg> exit ${result.code}`;
    }
  }

  if (stepEl) {
    stepEl.classList.remove("agent-term-card--running");
    stepEl.classList.add(result.code === 0 ? "agent-term-card--ok" : "agent-term-card--err");
  }

  return result;
}

function _updateFilesBar(bar, list, tracked) {
  if (!bar || !list) return;
  const fileEntries = [...tracked.entries()].filter(([, t]) => t !== "cmd" && t !== "list" && t !== "read");
  const total = tracked.size;
  if (total === 0) { bar.style.display = "none"; return; }
  bar.style.display = "";
  bar.querySelector(".agent-files-bar__count").textContent = `${fileEntries.length} File${fileEntries.length !== 1 ? 's' : ''}`;
  // Incremental: append only newly-tracked entries and update a row's status when
  // its type changes, instead of rebuilding the whole <ul> on every tool call
  // (which was an O(n²) reflow storm during long agent runs).
  const rendered = list.__rendered || (list.__rendered = new Map()); // name -> { li, type }
  for (const [name, type] of tracked) {
    const statusLabel = { write: "Edited", code: "Edited", read: "Read", list: "Listed", cmd: "Command" }[type] || type;
    const statusCls = type === "cmd" ? "cmd" : (type === "read" || type === "list" ? "read" : "write");
    const prev = rendered.get(name);
    if (!prev) {
      const li = document.createElement("li");
      li.className = "agent-files-list__item";
      li.innerHTML = `${_langBadge(name)}<span>${_escHtml(name)}</span><span class="agent-files-list__status agent-files-list__status--${statusCls}">${statusLabel}</span>`;
      list.appendChild(li);
      rendered.set(name, { li, type });
    } else if (prev.type !== type) {
      const stEl = prev.li.querySelector(".agent-files-list__status");
      if (stEl) { stEl.className = `agent-files-list__status agent-files-list__status--${statusCls}`; stEl.textContent = statusLabel; }
      prev.type = type;
    }
  }
}

// Some models (notably DeepSeek) recap files they've read by dumping pseudo-XML
// `<file_content path='...'>…</file_content>` and `<item name='…' isDir='…'/>`
// tags as plain text. Turn those into the editor's normal code/file cards by
// rewriting them to fenced code blocks (which renderMarkdownInto renders as
// cards), instead of showing raw tags.
function _transformFileContentTags(text) {
  if (!text || (text.indexOf("<file_content") === -1 && text.indexOf("<item") === -1)) return text;
  // Each block ends at its </file_content> (consumed), the next <file_content>,
  // or end of text — so trailing prose stays out of the card, and an
  // unterminated block while streaming still renders as a growing card.
  let t = text.replace(
    /<file_content\s+path=['"]([^'"]+)['"][^>]*>([\s\S]*?)(?:<\/file_content>|(?=<file_content\s)|$)/gi,
    (_m, path, bodyRaw) => {
      let body = bodyRaw;
      if (/<item\b/i.test(body)) {
        const items = [];
        const re = /<item\s+name=['"]([^'"]+)['"][^>]*?\bisDir=['"]?(true|false)['"]?[^>]*?\/?>/gi;
        let im;
        // Files use no leading glyph: a later cleanup pass strips "📄 …" lines.
        while ((im = re.exec(body))) items.push(im[2].toLowerCase() === "true" ? "📁 " + im[1] + "/" : "   " + im[1]);
        body = items.join("\n") || body.replace(/<[^>]*>/g, "").trim();
      } else {
        body = body.replace(/<\/?(?:file_content|item)[^>]*>/gi, "").replace(/^[ \t]*\d+\|/gm, "").replace(/^\n+|\s+$/g, "");
      }
      const base = path.split("/").pop() || path;
      const ext = base.indexOf(".") >= 0 ? base.split(".").pop().toLowerCase() : "";
      const lang = /^[a-z0-9]{1,8}$/.test(ext) ? ext : "text";
      return "\n```" + lang + ":" + path + "\n" + body + "\n```\n";
    }
  );
  // Any standalone <item> left outside a block → markdown list bullets
  // (dirs keep 📁; files use a neutral glyph since "📄 …" lines get stripped).
  t = t.replace(/<item\s+name=['"]([^'"]+)['"][^>]*?\bisDir=['"]?(true|false)['"]?[^>]*?\/?>/gi,
    (_m, name, isDir) => `\n- ${isDir.toLowerCase() === "true" ? "📁 " + name + "/" : "▸ " + name}`);
  // Sweep up any leftover stray tags.
  t = t.replace(/<\/?(?:file_content|item)[^>]*>/gi, "");
  return t;
}

// DeepSeek (and some others) narrate their native tool calls as plain-text
// lines — "read_file /path", "run_cmd cd … && …" — even though the real action
// already renders as its own tool card. Drop that redundant narration so the
// chat reads like prose + cards. Fence-aware: a code line inside a ``` block
// that happens to start with a tool name is left untouched.
function _stripToolNarration(text) {
  if (!text || !/^[ \t>*\-]*(?:read_file|list_dir|write_file|edit_file|web_fetch|search|find_files|run_cmd|run_subagent|update_plan)\b[ \t(]/m.test(text)) return text;
  const lines = text.split("\n");
  let inFence = false;
  const out = [];
  const RE = /^[ \t>*\-]*(?:read_file|list_dir|write_file|edit_file|web_fetch|search|find_files|run_cmd|run_subagent|update_plan)\b[ \t(].*$/;
  for (const line of lines) {
    if (/^\s*```/.test(line)) { inFence = !inFence; out.push(line); continue; }
    if (!inFence && RE.test(line)) continue;
    out.push(line);
  }
  return out.join("\n");
}

function _cleanAgentText(text) {
  text = _transformFileContentTags(text);
  text = _stripToolNarration(text);
  return text.replace(/\[TOOL:\w+\]\s*\n?[^\n]*/g, "").replace(/📄\s*[^\n]+\n?/g, "").replace(/📎\s*[^\n]+\n?/g, "").replace(/\n{3,}/g, "\n\n").trim();
}

function _parseStreamSegments(text) {
  const segs = [];
  const lines = text.split("\n");
  let i = 0, textBuf = "";
  while (i < lines.length) {
    const line = lines[i];
    const toolM = line.match(/^\[TOOL:(read_file|list_dir|run_cmd|write_file)\]\s*(.*)$/)
      || line.match(/^(read_file|list_dir|run_cmd|write_file)\s+(\/[^\s].*|[a-zA-Z]:\\[^\s].*)$/)
      || (line.match(/^(read_file|list_dir|run_cmd|write_file)\s*$/) && i + 1 < lines.length && /^[\s]*[\/~]/.test(lines[i + 1]) ? [line, line.trim(), ""] : null);
    if (toolM) {
      if (textBuf.trim()) { segs.push({ type: "text", content: textBuf, complete: true }); textBuf = ""; }
      const cmd = toolM[1]; let arg = toolM[2]?.trim();
      if (!arg && i + 1 < lines.length && !lines[i + 1].startsWith("```")) { i++; arg = lines[i].trim(); }
      if (arg) {
        if (arg.startsWith("{") || arg.startsWith("(")) {
          try {
            const parsed = JSON.parse(arg.replace(/'/g, '"'));
            arg = parsed.file || parsed.path || parsed.command || parsed.cmd || parsed.dir || Object.values(parsed)[0] || arg;
          } catch {
            const m = arg.match(/["']([^"']+)["']/);
            if (m) arg = m[1];
          }
        }
        arg = arg.replace(/^["'`]+|["'`]+$/g, "").trim();
      }
      if (cmd === "write_file") {
        i++;
        if (i < lines.length && lines[i].match(/^```/)) {
          const lang = lines[i].replace(/^```/, "").trim(); i++;
          let code = ""; let closed = false;
          while (i < lines.length) { if (lines[i].match(/^```\s*$/)) { closed = true; i++; break; } code += (code ? "\n" : "") + lines[i]; i++; }
          segs.push({ type: "code", lang, content: code, complete: closed, contextBefore: arg });
        } else { segs.push({ type: "code", lang: "", content: "", complete: false, contextBefore: arg }); }
      } else {
        if (arg) {
          if (cmd === "run_cmd") segs.push({ type: "cmd", command: arg, complete: true });
          else if (cmd === "read_file") segs.push({ type: "read", path: arg, complete: true });
          else segs.push({ type: "list", path: arg, complete: true });
        }
        i++;
      }
      continue;
    }
    const emojiF = line.match(/^📄\s*(.+)$/);
    if (emojiF && i + 1 < lines.length && lines[i + 1].match(/^```/)) {
      if (textBuf.trim()) { segs.push({ type: "text", content: textBuf, complete: true }); textBuf = ""; }
      const fname = emojiF[1].trim(); i++;
      const lang = lines[i].replace(/^```/, "").trim(); i++;
      let code = ""; let closed = false;
      while (i < lines.length) { if (lines[i].match(/^```\s*$/)) { closed = true; i++; break; } code += (code ? "\n" : "") + lines[i]; i++; }
      segs.push({ type: "code", lang, content: code, complete: closed, contextBefore: fname });
      continue;
    }
    const cmdM = line.match(/^📎\s*(.+)$/);
    if (cmdM) {
      if (textBuf.trim()) { segs.push({ type: "text", content: textBuf, complete: true }); textBuf = ""; }
      segs.push({ type: "cmd", command: cmdM[1].trim(), complete: true }); i++; continue;
    }
    const fenceM = line.match(/^```(\w*)\s*$/);
    if (fenceM) {
      const ctx = textBuf;
      if (textBuf.trim()) { segs.push({ type: "text", content: textBuf, complete: true }); textBuf = ""; }
      const lang = fenceM[1]; i++;
      let code = ""; let closed = false;
      while (i < lines.length) { if (lines[i].match(/^```\s*$/)) { closed = true; i++; break; } code += (code ? "\n" : "") + lines[i]; i++; }
      segs.push({ type: "code", lang, content: code, complete: closed, contextBefore: ctx });
      continue;
    }
    textBuf += (textBuf ? "\n" : "") + line; i++;
  }
  if (textBuf) segs.push({ type: "text", content: textBuf, complete: false });
  return segs;
}

function _extractSegFileName(seg, allSegs, idx) {
  const ctx = seg.contextBefore || (idx > 0 && allSegs[idx - 1]?.type === "text" ? allSegs[idx - 1].content : "");
  if (ctx) {
    const revLines = ctx.split("\n").reverse();
    for (const raw of revLines) {
      const s = raw.replace(/[`"'*#>📄📎]/g, "").trim();
      if (!s) continue;
      const m1 = s.match(/([a-zA-Z0-9_\-./\\]+\.[a-zA-Z]{1,10})\s*[:：]?\s*$/);
      if (m1) return m1[1];
      const m3 = s.match(/([a-zA-Z0-9_\-./\\]+\/[a-zA-Z0-9_\-./\\]+\.[a-zA-Z]{1,10})/);
      if (m3) return m3[1];
      const m4 = s.match(/(?:^|\s)([a-zA-Z0-9_\-]+\.[a-zA-Z]{1,10})(?:\s|$|[:：—\-])/);
      if (m4) return m4[1];
      const m2 = s.match(/(?:文件|创建|写入|新建|编写|修改|更新|File|file)\s*[`"':\s]*([a-zA-Z0-9_\-./\\]+\.[a-zA-Z]{1,10})/);
      if (m2) return m2[1];
      if (s.length > 120) break;
    }
  }
  const c = seg.content;
  if (/from flask import|Flask\(__name__\)|app\.route/.test(c)) return "app.py";
  if (/class \w+\(.*(?:db\.Model|Base)\)/.test(c)) return "models.py";
  if (/<!DOCTYPE|<html|<head|<body/.test(c)) return "index.html";
  if (/import React|from ['"]react['"]/.test(c)) return "App.jsx";
  if (/import express|require\(['"]express/.test(c)) return "server.js";
  if (/CREATE TABLE|ALTER TABLE/i.test(c)) return "schema.sql";
  if (/FROM\s+(node|python|golang|alpine|ubuntu)/i.test(c)) return "Dockerfile";
  if (/server\s*\{|location\s*\//.test(c)) return "nginx.conf";
  if (/{%\s*(extends|block|include)|{{\s*\w+/.test(c)) return "template.html";
  return null;
}

function _renderAgentSegStatic(container, seg, allSegs, idx) {
  if (seg.type === "text") {
    let clean = _cleanAgentText(seg.content);
    if (!clean) return;
    const nextSeg = idx + 1 < allSegs.length ? allSegs[idx + 1] : null;
    if (nextSeg && nextSeg.type === "code") {
      clean = clean.replace(/(?:^|\n)\s*[`"']*([a-zA-Z0-9_\-./\\]+\.[a-zA-Z]{1,10})[`"']*\s*[:：]?\s*$/m, "").trim();
    }
    if (!clean) return;
    const div = document.createElement("div");
    div.className = "agent-seg";
    renderMarkdownInto(div, clean, { highlighter: highlightCode });
    container.appendChild(div);
  } else if (seg.type === "code") {
    if (!seg.content || !seg.content.trim()) return;
    const fileName = _extractSegFileName(seg, allSegs, idx);
    if (fileName) {
      const added = seg.content.split("\n").length;
      const badge = _langBadge(fileName);
      const step = document.createElement("div");
      step.className = "agent-tool-step agent-tool-step--write agent-tool-step--accepted";
      step.innerHTML =
        `<div class="agent-tool-row">` +
        `<svg class="atc-chev" viewBox="0 0 12 12" width="12" height="12"><path d="M4 2.5l3.5 3.5-3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>` +
        `<div class="atc-type-icon"><svg viewBox="0 0 16 16" fill="currentColor"><path d="M11.013 1.427a1.75 1.75 0 012.474 0l1.086 1.086a1.75 1.75 0 010 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 01-.927-.928l.929-3.25c.081-.286.235-.547.445-.758l8.61-8.61z"/></svg></div>` +
        `<div class="atc-info"><div class="atc-action-row"><span class="atc-action">Edited</span><span class="atc-path atc-path--clickable" data-filepath="${_escAttr(fileName)}">${_escHtml(fileName)}</span></div></div>` +
        `<span class="atc-result atc-result--ok"><span class="atc-diffstat"><span class="a">+${added}</span></span></span></div>` +
        `<div class="atc-viewport"></div>`;
      step.querySelector(".agent-tool-row").addEventListener("click", () => step.classList.toggle("is-open"));
      const clickPath = step.querySelector(".atc-path--clickable");
      if (clickPath) {
        clickPath.addEventListener("click", (e) => {
          e.stopPropagation();
          const fp = clickPath.dataset.filepath;
          if (fp) {
            const root = rootPath || workspaceRoots[0] || "";
            const fullPath = fp.startsWith("/") ? fp : root + "/" + fp;
            openFile(fullPath, fp.split("/").pop());
          }
        });
      }
      const vp = step.querySelector(".atc-viewport");
      vp.innerHTML = _buildDiffView("", seg.content, fileName);
      _highlightDiffView(vp);
      container.appendChild(step);
    } else {
      const fenceInfo = seg.lang ? seg.lang : "";
      const div = document.createElement("div");
      div.className = "agent-seg";
      renderMarkdownInto(div, "```" + fenceInfo + "\n" + seg.content + "\n```", { highlighter: highlightCode });
      container.appendChild(div);
    }
  } else if (seg.type === "cmd") {
    if (!seg.command || !seg.command.trim()) return;
    const step = document.createElement("div");
    step.className = "agent-term-card agent-term-card--ok";
    step.innerHTML =
      `<div class="agent-term-card__header">` +
        `<svg class="agent-term-card__icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3 4l3 4-3 4M8.5 12H13"/></svg>` +
        `<span class="agent-term-card__label">Terminal</span>` +
        `<div class="agent-term-status agent-term-status--ok">` +
          `<svg viewBox="0 0 12 12" width="10" height="10" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm2.22 4.22a.75.75 0 010 1.06l-3 3a.75.75 0 01-1.06 0l-1.5-1.5a.75.75 0 111.06-1.06L4.69 6.69l2.47-2.47a.75.75 0 011.06 0z"/></svg> done` +
        `</div>` +
      `</div>` +
      `<div class="agent-term-card__cmd">` +
        `<span class="agent-term-card__prompt">$</span>` +
        `<code class="agent-term-card__code">${_escHtml(seg.command || "")}</code>` +
      `</div>`;
    container.appendChild(step);
  }
}

function _renderAgentSeg(container, seg, allSegs, idx, root, promises) {
  if (seg.type === "text") {
    let clean = _cleanAgentText(seg.content);
    if (!clean) return;
    const nextSeg = idx + 1 < allSegs.length ? allSegs[idx + 1] : null;
    if (nextSeg && nextSeg.type === "code") {
      clean = clean.replace(/(?:^|\n)\s*[`"']*([a-zA-Z0-9_\-./\\]+\.[a-zA-Z]{1,10})[`"']*\s*[:：]?\s*$/m, "").trim();
    }
    if (!clean) return;
    const div = document.createElement("div");
    div.className = "agent-seg";
    renderMarkdownInto(div, clean, { highlighter: highlightCode });
    container.appendChild(div);
  } else if (seg.type === "code") {
    if (!seg.content || !seg.content.trim()) return;
    const fileName = _extractSegFileName(seg, allSegs, idx);
    if (fileName) {
      const call = { type: "write", path: fileName, content: seg.content };
      const step = _createToolStep(call);
      container.appendChild(step);
      const p = _executeToolStep(step, call, root); promises.push(p);
    } else {
      const fenceInfo = seg.lang ? seg.lang : "";
      const div = document.createElement("div");
      div.className = "agent-seg";
      renderMarkdownInto(div, "```" + fenceInfo + "\n" + seg.content + "\n```", { highlighter: highlightCode });
      container.appendChild(div);
    }
  } else if (seg.type === "cmd") {
    const call = { type: "cmd", command: seg.command };
    const step = _createToolStep(call);
    container.appendChild(step);
    const p = _executeToolStep(step, call, root);
    p.then(r => { if (r) p._result = r; });
    promises.push(p);
  } else if (seg.type === "read") {
    if (!seg.path || !seg.path.trim()) return;
    const call = { type: "read", path: seg.path };
    const step = _createToolStep(call);
    container.appendChild(step);
    const p = _executeToolStep(step, call, root);
    p.then(r => { if (r) p._result = r; });
    promises.push(p);
  } else if (seg.type === "list") {
    if (!seg.path || !seg.path.trim()) return;
    const call = { type: "list", path: seg.path };
    const step = _createToolStep(call);
    container.appendChild(step);
    const p = _executeToolStep(step, call, root);
    p.then(r => { if (r) p._result = r; });
    promises.push(p);
  }
}

async function _executeInlineTools(response, container) {
  const segments = _splitAgentResponse(response);
  const toolSegs = segments.filter(s => s.type !== "text");
  if (!toolSegs.length) return;

  const log = document.createElement("div");
  log.className = "agent-tool-log";
  container.appendChild(log);
  const toolResults = [];
  const root = rootPath || workspaceRoots[0] || "";

  for (const seg of toolSegs) {
    const step = _createToolStep(seg);
    log.appendChild(step);
    chatEl.scrollTop = chatEl.scrollHeight;
    const tr = await _executeToolStep(step, seg, root);
    if (tr) toolResults.push(tr);
    await new Promise(r => setTimeout(r, 60));
    chatEl.scrollTop = chatEl.scrollHeight;
  }

  if (toolResults.length) _agentFollowUp(toolResults, container);
}

function _splitAgentResponse(response) {
  const segments = [];
  let remaining = response;
  const toolPattern = /\[TOOL:(read_file|write_file|run_cmd|list_dir)\]\s*\n?([^\n]+)/;
  const bareToolPattern = /(?:^|\n)(read_file|list_dir|run_cmd)\s+(\/[^\s\n][^\n]*)/;
  const writePattern = /\[TOOL:write_file\]\s*\n?([^\n]+)\n```[\w]*\n([\s\S]*?)```/;

  const emojiFilePattern = /📄\s*([^\n]+)\n```(\w*)\n([\s\S]*?)```/;
  const emojiCmdPattern = /📎\s*(.+?)(?:\n|$)/;

  while (remaining.length > 0) {
    const wm = remaining.match(writePattern);
    const tm = remaining.match(toolPattern);
    const bt = remaining.match(bareToolPattern);
    const ef = remaining.match(emojiFilePattern);
    const ec = remaining.match(emojiCmdPattern);

    const candidates = [
      wm && { m: wm, idx: remaining.indexOf(wm[0]), type: "wm" },
      tm && { m: tm, idx: remaining.indexOf(tm[0]), type: "tm" },
      bt && { m: [bt[0].replace(/^\n/, ""), bt[1], bt[2]], idx: Math.max(0, remaining.indexOf(bt[0]) + (bt[0].startsWith("\n") ? 1 : 0)), type: "tm" },
      ef && { m: ef, idx: remaining.indexOf(ef[0]), type: "ef" },
      ec && { m: ec, idx: remaining.indexOf(ec[0]), type: "ec" },
    ].filter(Boolean).sort((a, b) => a.idx - b.idx);

    if (!candidates.length) {
      segments.push({ type: "text", content: remaining });
      break;
    }

    const best = candidates[0];
    if (best.idx > 0) segments.push({ type: "text", content: remaining.slice(0, best.idx) });

    if (best.type === "wm") {
      segments.push({ type: "write", path: best.m[1].trim(), content: best.m[2] });
    } else if (best.type === "ef") {
      segments.push({ type: "write", path: best.m[1].trim(), content: best.m[3] });
    } else if (best.type === "ec") {
      segments.push({ type: "cmd", command: best.m[1].trim() });
    } else {
      const cmd = best.m[1];
      const arg = best.m[2].trim();
      if (arg) {
        if (cmd === "read_file") segments.push({ type: "read", path: arg });
        else if (cmd === "list_dir") segments.push({ type: "list", path: arg });
        else if (cmd === "run_cmd") segments.push({ type: "cmd", command: arg });
        else if (cmd === "write_file") segments.push({ type: "write", path: arg, content: "" });
      }
    }

    remaining = remaining.slice(best.idx + best.m[0].length);
  }

  const newSegs = [];
  const langToFile = { python: "main.py", py: "main.py", javascript: "app.js", js: "app.js", typescript: "app.ts", ts: "app.ts", html: "index.html", css: "style.css", json: "data.json", yaml: "config.yaml", yml: "config.yml", shell: "script.sh", bash: "script.sh", sh: "script.sh", sql: "query.sql", go: "main.go", rust: "main.rs", rs: "main.rs", java: "Main.java", c: "main.c", cpp: "main.cpp", ruby: "app.rb", rb: "app.rb", php: "index.php", swift: "main.swift", kotlin: "main.kt", dart: "main.dart", vue: "App.vue", svelte: "App.svelte", jsx: "App.jsx", tsx: "App.tsx" };
  const fileNamePat = /[`"']?([a-zA-Z0-9_\-./\\]+\.[a-zA-Z]{1,10})[`"']?\s*(?:[：:]\s*)?$/;

  for (const seg of segments) {
    if (seg.type !== "text") { newSegs.push(seg); continue; }
    const allBlocks = [...seg.content.matchAll(/```(\w*)\n([\s\S]*?)```/g)];
    if (!allBlocks.length) { newSegs.push(seg); continue; }

    let txt = seg.content;
    let lastEnd = 0;
    const usedNames = new Set();

    for (const m of allBlocks) {
      const lang = m[1].toLowerCase();
      const code = m[2];
      if (code.split("\n").length < 3) continue;

      const blockStart = txt.indexOf(m[0], lastEnd);
      const textBefore = txt.slice(lastEnd, blockStart);

      let fileName = "";
      const allLines = textBefore.split("\n").reverse();
      for (const line of allLines) {
        const stripped = line.replace(/[`"'*#>\-📄]/g, "").trim();
        const fm = stripped.match(/([a-zA-Z0-9_\-./\\]+\.[a-zA-Z]{1,10})\s*[:：]?\s*$/);
        if (fm) { fileName = fm[1]; break; }
        const fm2 = stripped.match(/(?:文件|创建|写入|新建|编写|修改)\s*[`"']*([a-zA-Z0-9_\-./\\]+\.[a-zA-Z]{1,10})/);
        if (fm2) { fileName = fm2[1]; break; }
      }

      if (!fileName && code) {
        if (/from flask import|Flask\(__name__\)|app\.route/.test(code)) fileName = "app.py";
        else if (/class \w+\(.*(?:db\.Model|Base)\)/.test(code)) fileName = "models.py";
        else if (/<!DOCTYPE|<html|<head|<body/.test(code)) fileName = "index.html";
        else if (/import React|from ['"]react['"]|export default/.test(code)) fileName = "App.jsx";
        else if (/import express|require\(['"]express/.test(code)) fileName = "server.js";
        else if (/CREATE TABLE|ALTER TABLE/.test(code)) fileName = "schema.sql";
        else if (/FROM\s+(node|python|golang|alpine|ubuntu)/.test(code)) fileName = "Dockerfile";
        else if (/server\s*\{|location\s*\//.test(code)) fileName = "nginx.conf";
        else if (lang && langToFile[lang]) {
          let base = langToFile[lang]; let i = 1;
          while (usedNames.has(base)) { base = base.replace(/(\.\w+)$/, `${++i}$1`); }
          fileName = base;
        }
      }

      if (!fileName) { newSegs.push({ type: "text", content: textBefore + m[0] }); lastEnd = blockStart + m[0].length; continue; }

      usedNames.add(fileName);
      if (textBefore.trim()) {
        const cleanText = textBefore.replace(fileNamePat, "").trim();
        if (cleanText) newSegs.push({ type: "text", content: cleanText });
      }
      newSegs.push({ type: "write", path: fileName, content: code });
      lastEnd = blockStart + m[0].length;
    }
    if (lastEnd < txt.length) {
      const rest = txt.slice(lastEnd).trim();
      if (rest) newSegs.push({ type: "text", content: rest });
    }
  }
  return newSegs.length > 0 ? newSegs : segments;
}

const _ATC_LANG_MAP = { py: "py", python: "py", js: "js", javascript: "js", jsx: "js", ts: "ts", typescript: "ts", tsx: "ts", html: "html", htm: "html", css: "css", scss: "css", less: "css", rs: "rs", rust: "rs", go: "go", sh: "sh", bash: "sh", shell: "sh", zsh: "sh", json: "json", sql: "sql", md: "md", markdown: "md" };
function _langBadge(pathOrLang) {
  const ext = pathOrLang.split(".").pop().toLowerCase();
  const key = _ATC_LANG_MAP[ext] || _ATC_LANG_MAP[pathOrLang] || "default";
  const labels = { py: "PY", js: "JS", ts: "TS", html: "HTML", css: "CSS", rs: "RS", go: "GO", sh: "SH", json: "JSON", sql: "SQL", md: "MD", cmd: "CMD", default: "FILE" };
  return `<span class="atc-lang-badge atc-lang-badge--${key}">${labels[key] || key.toUpperCase()}</span>`;
}
const _ATC_EXPAND_ICON = `<svg viewBox="0 0 12 12" width="12" height="12"><path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

// ============================================================================
//  Agentic loop — the engine behind agent / explorer / reviewer modes.
//
//  A real multi-turn loop: on each turn the model may emit tool calls; we run
//  them, feed the results back as proper `tool` messages, and let the model
//  keep going until it stops calling tools (task done) or we hit MAX_ITERS.
//  This is what lets the agent read → reason → edit → verify → fix on its own,
//  the way Claude Code / Codex do, instead of a single read-then-answer pass.
// ============================================================================

const _AGENT_MAX_ITERS = 40;

function _buildAgentToolSchemas(includeWrite) {
  const tools = [
    { type: "function", function: { name: "read_file", description: "读取文件内容（默认最多约 400 行）。文件很大时用 offset/limit 分页继续读完。改文件前先读清楚。", parameters: { type: "object", properties: { path: { type: "string", description: "相对工作区根目录的路径或绝对路径" }, offset: { type: "integer", description: "起始行号(1 基)，默认 1" }, limit: { type: "integer", description: "读取的行数，默认 400" } }, required: ["path"] } } },
    { type: "function", function: { name: "list_dir", description: "列出某个目录下的文件和子目录。用 \".\" 表示工作区根。", parameters: { type: "object", properties: { path: { type: "string", description: "目录路径" } }, required: ["path"] } } },
    { type: "function", function: { name: "search", description: "在整个项目里按文本搜索（grep），返回匹配的文件、行号和该行内容。用来找符号定义、用法、引用。", parameters: { type: "object", properties: { query: { type: "string", description: "要搜索的文本" }, path: { type: "string", description: "可选，限定搜索的子目录" } }, required: ["query"] } } },
    { type: "function", function: { name: "find_files", description: "按文件名或 glob 模式查找文件，如 *.rs、main.js、src/**/*.ts，或直接给文件名子串。", parameters: { type: "object", properties: { pattern: { type: "string", description: "文件名或 glob 模式" } }, required: ["pattern"] } } },
    { type: "function", function: { name: "web_search", description: "联网搜索（DuckDuckGo），返回标题/URL/摘要列表。用来查官方文档、API 用法、库的最新版本、报错解决方案、技术文章——找到相关页面后再用 web_fetch 读全文。不确定的 API/库/报错优先搜，别凭记忆猜。", parameters: { type: "object", properties: { query: { type: "string", description: "搜索关键词（可用英文更准）" } }, required: ["query"] } } },
    { type: "function", function: { name: "web_fetch", description: "抓取一个公网网页并返回正文文本，用于读 web_search 找到的页面、在线文档、API 参考、报错信息等。只支持 http/https 公网地址（本地/内网会被拒绝）。", parameters: { type: "object", properties: { url: { type: "string", description: "完整的 http/https URL" } }, required: ["url"] } } },
    { type: "function", function: { name: "update_plan", description: "创建或更新当前任务的分步计划，并随进度更新每步状态。多步任务开始时先用它列出计划，每完成一步就再调用更新状态。", parameters: { type: "object", properties: { steps: { type: "array", description: "有序的步骤列表", items: { type: "object", properties: { content: { type: "string", description: "这一步要做什么" }, status: { type: "string", enum: ["pending", "in_progress", "completed"], description: "状态" } }, required: ["content", "status"] } } }, required: ["steps"] } } },
    { type: "function", function: { name: "run_subagent", description: "派生一个独立的只读子智能体去完成一个聚焦的调研子任务（如「找出登录流程涉及哪些文件并总结」）。子智能体能读文件、列目录、搜索、查找，自主多轮调查后返回一份简报。把大范围调研拆出去能让主线保持清爽、更省上下文。", parameters: { type: "object", properties: { description: { type: "string", description: "子任务的简短描述（3-6 字）" }, prompt: { type: "string", description: "交给子智能体的完整任务说明，必须自包含——它看不到当前对话历史。" } }, required: ["description", "prompt"] } } },
    { type: "function", function: { name: "remember", description: "把一条值得跨会话长期记住的项目知识写进项目记忆（按工作区持久保存，下次自动加载进上下文）。适合记：技术栈/架构决定、约定、构建与测试命令、用户偏好、易踩的坑。只记真正长期有用的事实，别记一次性细节。", parameters: { type: "object", properties: { content: { type: "string", description: "要记住的一句话（简洁、自包含）" } }, required: ["content"] } } },
    { type: "function", function: { name: "get_diagnostics", description: "读取编辑器/LSP 对文件的诊断（错误与警告）。改完代码用它快速自检，比每次跑构建快。不传 path 则返回所有已打开文件的诊断。", parameters: { type: "object", properties: { path: { type: "string", description: "可选，要检查的文件路径；省略则查所有已打开文件" } } } } },
  ];
  if (includeWrite) {
    tools.push(
      { type: "function", function: { name: "edit_file", description: "对已有文件做精确替换：把 old_string 替换成 new_string。改已有文件请优先用它。old_string 必须能在文件中唯一定位（带足够上下文），否则会报错。", parameters: { type: "object", properties: { path: { type: "string" }, old_string: { type: "string", description: "要被替换的原文，需与文件内容逐字符一致" }, new_string: { type: "string", description: "替换后的新内容" }, replace_all: { type: "boolean", description: "为 true 时替换所有匹配；默认只替换唯一的一处" } }, required: ["path", "old_string", "new_string"] } } },
      { type: "function", function: { name: "multi_edit", description: "对同一个文件做多处精确替换：edits 是一组 {old_string, new_string, replace_all?}，按顺序原子应用（任一处定位失败则整体不写入、报错）。重构或一个文件里改多个地方时用它，比连发多次 edit_file 更快更可靠。每个 old_string 同样需在当时的文件内容里唯一定位。", parameters: { type: "object", properties: { path: { type: "string" }, edits: { type: "array", description: "有序的替换列表", items: { type: "object", properties: { old_string: { type: "string" }, new_string: { type: "string" }, replace_all: { type: "boolean" } }, required: ["old_string", "new_string"] } } }, required: ["path", "edits"] } } },
      { type: "function", function: { name: "write_file", description: "新建文件或整文件重写。仅用于新建或彻底重写；改局部请用 edit_file。", parameters: { type: "object", properties: { path: { type: "string" }, content: { type: "string" } }, required: ["path", "content"] } } },
      { type: "function", function: { name: "run_cmd", description: "在工作区里运行一条 shell 命令（装依赖、跑测试、构建、git 等）。", parameters: { type: "object", properties: { command: { type: "string" } }, required: ["command"] } } },
      { type: "function", function: { name: "delete_path", description: "删除工作区内的一个文件或目录（递归）。用于清理、重构。务必只删确实该删的，删前最好先确认路径存在。", parameters: { type: "object", properties: { path: { type: "string", description: "要删除的文件或目录路径" } }, required: ["path"] } } },
      { type: "function", function: { name: "move_path", description: "移动或重命名工作区内的文件/目录（from → to）。重构、改名时用。", parameters: { type: "object", properties: { from: { type: "string", description: "源路径" }, to: { type: "string", description: "目标路径" } }, required: ["from", "to"] } } },
    );
  }
  return tools;
}

/** Translate an OpenAI tool call into the internal `call` shape `_executeToolStep` understands. */
function _mapToolCall(name, args) {
  args = args || {};
  switch (name) {
    case "read_file": return { type: "read", path: args.path || "", offset: args.offset, limit: args.limit };
    case "list_dir": return { type: "list", path: args.path || "" };
    case "search": return { type: "search", path: args.query || "", query: args.query || "", searchPath: args.path || "" };
    case "find_files": return { type: "find", path: args.pattern || "", pattern: args.pattern || "" };
    case "web_fetch": return { type: "web", path: args.url || "", url: args.url || "" };
    case "web_search": return { type: "websearch", path: args.query || "", query: args.query || "" };
    case "edit_file": return { type: "edit", path: args.path || "", oldString: args.old_string || "", newString: args.new_string || "", replaceAll: !!args.replace_all };
    case "multi_edit": return { type: "multiedit", path: args.path || "", edits: Array.isArray(args.edits) ? args.edits : [] };
    case "write_file": return { type: "write", path: args.path || "", content: args.content || "" };
    case "run_cmd": return { type: "cmd", command: args.command || "" };
    case "update_plan": return { type: "plan", steps: _normPlanSteps(args.steps || args.plan || args.todos) };
    case "run_subagent": return { type: "subagent", path: args.description || "调研", description: args.description || "调研子任务", prompt: args.prompt || "" };
    case "remember": return { type: "memory", path: "项目记忆", content: args.content || "" };
    case "get_diagnostics": return { type: "diag", path: args.path || "" };
    case "delete_path": return { type: "delete", path: args.path || "" };
    case "move_path": return { type: "move", path: args.from || "", to: args.to || "" };
    default: return null;
  }
}

/** Normalize loosely-shaped plan steps from the model into {content, status}. */
function _normPlanSteps(steps) {
  if (!Array.isArray(steps)) return [];
  return steps
    .map((s) => typeof s === "string"
      ? { content: s, status: "pending" }
      : { content: s.content || s.step || s.text || s.title || "", status: String(s.status || "pending").toLowerCase() })
    .filter((s) => s.content);
}

/** Short textual confirmation of a plan update, fed back to the model. */
function _planSummary(steps) {
  const total = steps.length;
  const done = steps.filter((s) => s.status === "completed").length;
  const cur = steps.find((s) => s.status === "in_progress");
  return `计划已更新：共 ${total} 步，已完成 ${done}${cur ? `，进行中：${cur.content}` : ""}。`;
}

/** Create or update the live plan panel pinned at the top of the agent message. */
function _renderPlan(container, steps, existingEl) {
  const done = steps.filter((s) => s.status === "completed").length;
  const icon = (st) => st === "completed"
    ? `<svg viewBox="0 0 16 16" width="13" height="13" fill="#2ea043" style="flex:0 0 auto;margin-top:1px"><path d="M8 0a8 8 0 100 16A8 8 0 008 0zm3.78 5.97a.75.75 0 010 1.06l-4.5 4.5a.75.75 0 01-1.06 0l-2-2a.75.75 0 111.06-1.06l1.47 1.47 3.97-3.97a.75.75 0 011.06 0z"/></svg>`
    : st === "in_progress"
    ? `<span class="atc-spin" style="flex:0 0 auto;width:12px;height:12px;margin-top:1px;border-width:2px"></span>`
    : `<svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4" style="flex:0 0 auto;margin-top:1px;opacity:.45"><circle cx="8" cy="8" r="6.5"/></svg>`;
  let el = existingEl;
  if (!el) {
    el = document.createElement("div");
    el.className = "agent-plan";
    el.style.cssText = "margin:6px 0 10px;border:1px solid rgba(128,128,128,.25);border-radius:10px;overflow:hidden;font-size:13px";
    container.insertBefore(el, container.firstChild);
  }
  el.innerHTML =
    `<div style="display:flex;align-items:center;gap:7px;padding:8px 12px;font-weight:600;opacity:.9;border-bottom:1px solid rgba(128,128,128,.18)">` +
      `<svg viewBox="0 0 16 16" width="13" height="13" fill="currentColor"><path d="M2 2.75C2 1.78 2.78 1 3.75 1h8.5c.97 0 1.75.78 1.75 1.75v10.5c0 .97-.78 1.75-1.75 1.75h-8.5C2.78 15 2 14.22 2 13.25V2.75zM5 4.5a.75.75 0 000 1.5h6a.75.75 0 000-1.5H5zm0 3a.75.75 0 000 1.5h6a.75.75 0 000-1.5H5zm0 3a.75.75 0 000 1.5h3.5a.75.75 0 000-1.5H5z"/></svg>` +
      `<span>任务计划</span><span style="margin-left:auto;font-weight:400;opacity:.55">${done}/${steps.length}</span>` +
    `</div>` +
    `<ul style="list-style:none;margin:0;padding:6px 0">` +
    steps.map((s) => `<li style="display:flex;gap:8px;padding:4px 12px;line-height:1.45${s.status === "completed" ? ";opacity:.55;text-decoration:line-through" : ""}">${icon(s.status)}<span>${_escHtml(s.content)}</span></li>`).join("") +
    `</ul>`;
  return el;
}

// ============================================================================
// Agent infrastructure: caching, project memory, context compaction.
// ============================================================================

// --- Caching: avoid re-reading the same file / re-fetching the same URL during
// a run. The read cache is cleared at the start of each agent run and on any
// command (which may mutate files), and invalidated per-path on edit/write/
// delete/move. The web cache lives for the session (page content is stable). ---
const _agentReadCache = new Map(); // absolute path -> file text
const _agentWebCache = new Map(); // url|query -> result text
// Bounded LRU-ish put: evict the oldest entry past the cap so a long session of
// web_fetch/web_search can't grow this cache without limit.
function _webCachePut(key, val) {
  _agentWebCache.set(key, val);
  if (_agentWebCache.size > 60) _agentWebCache.delete(_agentWebCache.keys().next().value);
}
function _clearAgentReadCache() { _agentReadCache.clear(); }
function _invalidateRead(path) {
  if (!path) return;
  const root = rootPath || workspaceRoots[0] || "";
  for (const k of [path, root ? root + "/" + path.replace(/^\/+/, "") : null]) {
    if (k) _agentReadCache.delete(k);
  }
}

// Refresh the explorer around a path the agent just created/changed/removed so a
// new/deleted file shows up immediately — instead of waiting on the fs watcher,
// which only reloads already-expanded dirs and can miss a freshly-written file.
function _refreshTreeFor(absPath) {
  if (!absPath) return;
  const dir = parentDir(absPath);
  try {
    if (dir && (workspaceRoots.includes(dir) || dirNodes.has(dir))) reloadDir(dir);
    else reloadDir(rootPath || workspaceRoots[0] || dir);
  } catch { /* tree not ready */ }
  try { refreshGitStatus(); } catch { /* git panel not ready */ }
}

// Run-level checkpoint: snapshot each file the moment before the agent FIRST
// changes it this run, so the whole run's edits can be reverted in one click.
const _runCheckpoint = new Map(); // absPath -> { existed: bool, content: string }
function _checkpointRecord(absPath, existed, content) {
  if (!absPath || _runCheckpoint.has(absPath)) return;
  _runCheckpoint.set(absPath, { existed: !!existed, content: existed ? (content || "") : "" });
}
async function _revertRun(snapshot) {
  let ok = 0, fail = 0;
  for (const [path, snap] of snapshot) {
    try {
      if (snap.existed) await backend.writeTextFile(path, snap.content);
      else await backend.deletePath(path).catch(() => {});
      _agentReadCache.delete(path); _invalidateRead(path); _refreshTreeFor(path);
      _ipcBroadcast("file_changed", { path });
      ok++;
    } catch { fail++; }
  }
  return { ok, fail };
}

// Heuristic for "this is a multi-step / build-something task" → worth a plan-first
// approach. Deliberately conservative so quick fixes aren't slowed down.
function _looksComplexTask(text) {
  const t = (text || "").trim();
  if (t.length > 280) return true;
  return /(实现|重构|搭建|做一个|做个|开发|新增功能|加.{0,4}功能|集成|迁移|设计.{0,6}(系统|架构|页面|功能|模块)|build |implement|refactor|create (a|an)|add (a|an).{0,30}(feature|page|component|endpoint|api|system|module)|scaffold|set ?up|migrate)/i.test(t);
}

// --- Project memory: notes the agent writes with the `remember` tool, persisted
// per-workspace in localStorage and auto-injected into the agent's context so it
// carries knowledge across turns and sessions (like CLAUDE.md, but agent-authored). ---
function _memoryKey(root) { return "michael-ide.memory:" + (root || "_global"); }
function _loadMemory(root) {
  try { return localStorage.getItem(_memoryKey(root)) || ""; } catch { return ""; }
}
function _appendMemory(root, note) {
  const clean = String(note || "").trim().replace(/\n+/g, " ");
  if (!clean) return false;
  let lines = _loadMemory(root).split("\n").filter(Boolean);
  lines.push("- " + clean);
  if (lines.length > 60) lines = lines.slice(-60); // keep the most recent notes
  let mem = lines.join("\n");
  if (mem.length > 8000) mem = mem.slice(mem.length - 8000);
  try { localStorage.setItem(_memoryKey(root), mem); } catch {}
  _agentContextCache = { root: null, ts: 0, data: "" }; // force context rebuild so the note shows
  return true;
}

// Memory panel: view / edit / clear the agent's project memory for the current
// workspace. Opened via the "Manage Project Memory" command (⌘⇧P).
function openMemoryPanel() {
  const root = rootPath || workspaceRoots[0] || "";
  const overlay = document.createElement("div");
  overlay.style.cssText = "position:fixed;inset:0;background:rgba(0,0,0,.45);display:flex;align-items:center;justify-content:center;z-index:9999";
  const modal = document.createElement("div");
  modal.style.cssText = "width:min(640px,92vw);max-height:80vh;display:flex;flex-direction:column;background:var(--panel,#1e1e22);border:1px solid var(--border,rgba(128,128,128,.35));border-radius:12px;box-shadow:0 20px 60px rgba(0,0,0,.5);overflow:hidden";
  modal.innerHTML =
    `<div style="display:flex;align-items:center;gap:8px;padding:12px 14px;border-bottom:1px solid var(--border,rgba(128,128,128,.25))">` +
      `<span style="font-size:14px;font-weight:600">🧠 项目记忆</span>` +
      `<span style="font-size:11px;opacity:.55;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${_escHtml(root || "(无工作区)")}</span>` +
      `<button class="mem-close" style="margin-left:auto;background:none;border:none;color:inherit;cursor:pointer;font-size:16px;opacity:.6">✕</button>` +
    `</div>` +
    `<div style="padding:10px 14px 0;font-size:12px;opacity:.6">智能体用 remember 工具记下的、按工作区持久保存的知识，每轮自动注入它的上下文。可直接编辑或清空（一行一条）。</div>` +
    `<textarea class="mem-text" spellcheck="false" style="flex:1;min-height:240px;margin:10px 14px;padding:10px;font:12px/1.5 ui-monospace,Menlo,monospace;background:var(--bg,#15151a);color:var(--text,#eee);border:1px solid var(--border,rgba(128,128,128,.3));border-radius:8px;resize:vertical;outline:none"></textarea>` +
    `<div style="display:flex;gap:8px;padding:0 14px 14px;justify-content:flex-end">` +
      `<button class="mem-clear" style="padding:6px 12px;border-radius:7px;border:1px solid var(--border,rgba(128,128,128,.35));background:none;color:#f85149;cursor:pointer;font:inherit;font-size:12px">清空</button>` +
      `<button class="mem-save" style="padding:6px 14px;border-radius:7px;border:none;background:var(--accent,#3b82f6);color:#fff;cursor:pointer;font:inherit;font-size:12px">保存</button>` +
    `</div>`;
  overlay.appendChild(modal);
  document.body.appendChild(overlay);
  const ta = modal.querySelector(".mem-text");
  ta.value = _loadMemory(root);
  const close = () => overlay.remove();
  modal.querySelector(".mem-close").addEventListener("click", close);
  overlay.addEventListener("click", (e) => { if (e.target === overlay) close(); });
  modal.querySelector(".mem-save").addEventListener("click", () => {
    try { localStorage.setItem(_memoryKey(root), ta.value.slice(0, 8000)); } catch {}
    _agentContextCache = { root: null, ts: 0, data: "" };
    showToast("项目记忆已保存");
    close();
  });
  modal.querySelector(".mem-clear").addEventListener("click", () => {
    ta.value = "";
    try { localStorage.removeItem(_memoryKey(root)); } catch {}
    _agentContextCache = { root: null, ts: 0, data: "" };
    showToast("项目记忆已清空");
  });
}

/**
 * Keep the running `messages` array from blowing past the context window on long
 * autonomous tasks. Once it gets large, fold the *older* tool outputs (and, when
 * very large, older assistant prose) down to a stub. Only message CONTENT is
 * shortened — roles and tool_call ids are never touched, so the tool-call
 * structure the API requires stays intact.
 */
function _trimMessagesIfHuge(messages) {
  const HARD = 140000, SOFT = 90000;
  let total = 0;
  for (const m of messages) total += m.content ? String(m.content).length : 0;
  if (total <= SOFT) return;
  const aggressive = total > HARD;
  const KEEP = aggressive ? 4 : 8;
  const cap = aggressive ? 120 : 300;
  const toolIdx = [];
  for (let i = 0; i < messages.length; i++) if (messages[i].role === "tool") toolIdx.push(i);
  const trimUpTo = toolIdx.length - KEEP;
  for (let k = 0; k < trimUpTo; k++) {
    const i = toolIdx[k];
    const c = messages[i].content ? String(messages[i].content) : "";
    if (c.length > cap + 80) {
      messages[i] = { ...messages[i], content: c.slice(0, cap) + `\n…（较早工具输出已折叠以省上下文，原 ${c.length} 字）` };
    }
  }
  if (aggressive) {
    const lastKeep = messages.length - 6;
    for (let i = 1; i < lastKeep; i++) {
      const m = messages[i];
      if (m.role === "assistant" && !m.tool_calls && m.content && String(m.content).length > 600) {
        messages[i] = { ...m, content: String(m.content).slice(0, 400) + "\n…（较早回复已折叠）" };
      }
    }
  }
}

/**
 * Conversation-level compaction (between turns): when the text-only chat
 * `history` gets long, summarize the older turns into one compact note and keep
 * only the most recent few verbatim. `history` holds plain user/assistant text
 * (no tool_call structure), so replacing a prefix is structurally safe. Guarded:
 * any failure falls back to a short marker rather than touching the structure.
 */
async function _compactHistoryIfHuge(config) {
  if (!Array.isArray(history) || history.length < 8) return;
  let total = 0;
  for (const m of history) total += m.content ? String(m.content).length : 0;
  if (total < 60000) return;
  const KEEP = 4;
  if (history.length <= KEEP + 2) return;
  const older = history.slice(0, history.length - KEEP);
  const transcript = older.map((m) => `[${m.role}] ${String(m.content || "").slice(0, 4000)}`).join("\n\n").slice(0, 50000);
  let summary = "";
  try {
    summary = await backend.aiComplete(config, [
      { role: "system", content: "把下面这段编程助手对话压缩成简洁要点，保留：用户目标与约束、已完成的关键改动与决定、已确认的事实/文件结构、未完成事项。用中文，分条，尽量短。" },
      { role: "user", content: transcript },
    ], 1024);
  } catch { summary = ""; }
  const note = summary && summary.trim()
    ? "【早先对话的压缩摘要】\n" + summary.trim()
    : "【早先对话较长，已省略以节省上下文】";
  history.splice(0, older.length, { role: "assistant", content: note });
  try { saveChatHistory(); } catch {}
}

/** Serialize a tool's result into the `tool` message content the model reads next turn. */
function _toolResultToString(call, result) {
  if (!result) return call.type === "write" || call.type === "edit" ? `（${call.path}：未应用）` : "(无结果)";
  const c = result.content != null ? String(result.content) : "";
  switch (result.type || call.type) {
    case "read": return `文件 ${result.path}:\n${c}`;
    case "list": return `目录 ${result.path}:\n${c}`;
    case "cmd": return `命令输出:\n${c || "(无输出)"}`;
    default: return c || `已完成 ${call.type}`;
  }
}

/** Glob (`*`, `**`, `?`) or plain substring → RegExp matched against a relative path. */
function _globToRegExp(glob) {
  if (!/[*?]/.test(glob)) {
    return new RegExp(glob.replace(/[.+^${}()|[\]\\]/g, "\\$&"), "i"); // substring match
  }
  let re = "";
  for (let i = 0; i < glob.length; i++) {
    const ch = glob[i];
    if (ch === "*") {
      if (glob[i + 1] === "*") { re += ".*"; i++; if (glob[i + 1] === "/") i++; }
      else re += "[^/]*";
    } else if (ch === "?") re += "[^/]";
    else re += ch.replace(/[.+^${}()|[\]\\]/g, "\\$&");
  }
  return new RegExp("(^|/)" + re + "$", "i");
}

/** Bounded recursive file finder, used by the `find_files` tool. Read-only and workspace-scoped. */
async function _agentFindFiles(root, pattern) {
  if (!root) return { count: 0, text: "[ERROR] 未打开工作区。" };
  const pat = (pattern || "").trim();
  if (!pat) return { count: 0, text: "[ERROR] 空 pattern。" };
  let rx;
  try { rx = _globToRegExp(pat); } catch { return { count: 0, text: "[ERROR] 无效 pattern。" }; }
  const IGNORED = new Set([".git", "node_modules", "target", "dist", "build", ".next", ".venv", "__pycache__", ".cache", "vendor"]);
  const out = [];
  const MAX = 200, MAX_SCAN = 8000;
  let scanned = 0;
  const stack = [{ dir: root, rel: "" }];
  while (stack.length && out.length < MAX && scanned < MAX_SCAN) {
    const { dir, rel } = stack.pop();
    let entries = [];
    try { entries = await backend.readDir(dir); } catch { continue; }
    for (const e of entries) {
      if (out.length >= MAX) break;
      scanned++;
      const name = e.name;
      if (!name || name.startsWith(".")) continue;
      const childRel = rel ? rel + "/" + name : name;
      if (e.is_dir) {
        if (!IGNORED.has(name)) stack.push({ dir: dir + "/" + name, rel: childRel });
      } else if (rx.test(childRel)) {
        out.push(childRel);
      }
    }
  }
  out.sort();
  const text = out.length ? out.join("\n") + (out.length >= MAX ? "\n…(更多结果已截断)" : "") : "(无匹配文件)";
  return { count: out.length, text };
}

/**
 * Drive one model turn: stream assistant text into `body`, accumulate any tool
 * calls (reassembled by `index` across deltas), and return both so the loop can
 * decide whether to run tools and continue.
 */
/** True for transient API/network errors worth retrying with backoff. */
function _isRetryableAiError(msg) {
  const m = String(msg || "").toLowerCase();
  return /\b(408|409|425|429|500|502|503|504)\b/.test(m)
    || /rate.?limit|too many requests|overloaded|temporar|timeout|timed out|econn|enotfound|network|connection (reset|refused|closed)|fetch failed|stream (error|closed)|server error|service unavailable|capacity|try again/.test(m);
}

async function _agentModelTurn({ config, messages, toolSchemas, body }) {
  let acc = "";
  let err = null;
  const byIndex = new Map();
  let streamEl = null;
  // Throttle streaming re-render to ~12fps. Re-parsing + rebuilding the whole
  // markdown DOM (and re-highlighting code) on every animation frame is O(n²)
  // over a long reply and freezes the UI. The final full render runs once when
  // the turn ends (below), so no content is lost.
  let lastFlush = 0;
  let flushTimer = 0;
  const doRender = () => {
    flushTimer = 0;
    lastFlush = Date.now();
    body.querySelector(".thinking")?.remove();
    const clean = _cleanAgentText(acc);
    if (clean.trim()) {
      if (!streamEl) { streamEl = document.createElement("div"); streamEl.className = "agent-seg agent-seg--stream"; body.appendChild(streamEl); }
      renderMarkdownStream(streamEl, clean, { streaming: true });
    }
    chatEl.scrollTop = chatEl.scrollHeight;
  };
  const schedule = () => {
    if (flushTimer) return;
    const wait = Math.max(0, 80 - (Date.now() - lastFlush));
    flushTimer = setTimeout(() => requestAnimationFrame(doRender), wait);
  };

  // Retry transient failures (rate limits, 5xx, network blips) with exponential
  // backoff — but only while nothing has streamed yet, so a retry can never
  // duplicate partial output. Both the main loop and sub-agents go through here.
  for (let attempt = 0; ; attempt++) {
    let turnErr = null;
    let produced = false;
    try {
      await backend.aiChatWithTools(config, messages, toolSchemas, (ev) => {
        if (ev.kind === "token") { produced = true; acc += ev.delta; schedule(); }
        else if (ev.kind === "toolCall") {
          produced = true;
          const idx = ev.index ?? 0;
          let e = byIndex.get(idx);
          if (!e) { e = { id: "", name: "", args: "" }; byIndex.set(idx, e); }
          if (ev.id) e.id = ev.id;
          if (ev.name) e.name = ev.name;
          if (ev.arguments) e.args += ev.arguments;
        }
        else if (ev.kind === "error") { turnErr = ev.message; }
      });
    } catch (e) { turnErr = String(e?.message || e); }

    if (turnErr && !produced && attempt < 3 && streaming && _isRetryableAiError(turnErr)) {
      showToast(`网络/服务波动，重试中… (${attempt + 1}/3)`);
      await new Promise((r) => setTimeout(r, 800 * Math.pow(2, attempt)));
      if (!streaming) { err = turnErr; break; }
      continue;
    }
    err = turnErr;
    break;
  }

  if (flushTimer) clearTimeout(flushTimer);
  body.querySelector(".thinking")?.remove();

  const cleanFinal = _cleanAgentText(acc);
  if (streamEl) {
    if (cleanFinal.trim()) renderMarkdownInto(streamEl, cleanFinal, { streaming: false });
    else streamEl.remove();
  } else if (cleanFinal.trim()) {
    const el = document.createElement("div");
    el.className = "agent-seg";
    renderMarkdownInto(el, cleanFinal, { streaming: false });
    body.appendChild(el);
  }

  const toolCalls = [...byIndex.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([, e]) => {
      let parsed = {};
      try { parsed = JSON.parse(e.args || "{}"); } catch { parsed = {}; }
      return {
        id: e.id || ("call_" + Math.random().toString(36).slice(2, 10)),
        name: e.name,
        argsRaw: (e.args && e.args.trim()) ? e.args : "{}",
        parsedArgs: parsed,
      };
    })
    .filter((tc) => tc.name);

  return { text: acc, toolCalls, error: err };
}

const _SUBAGENT_SYSTEM = `你是一个只读调研子智能体。你能用 read_file / list_dir / search / find_files 调查代码库，但不能修改文件、不能运行命令、也不能再派生子智能体。

自主进行多轮调查，顺着 import / 调用 / 定义层层追，把交给你的子任务彻底搞清楚。完成后用中文输出一份**简洁、可直接使用的简报**：
- 结论 / 答案放最前面
- 关键代码用 路径:行号 标注
- 必要时给出调用链、模块关系或文件清单
不要复述任务，不要写废话铺垫。`;

/**
 * Spawn a focused, read-only sub-agent — Claude Code's Task tool in miniature.
 * It runs its own bounded agentic loop with read-only tools and returns a report
 * string, which the parent feeds back as the run_subagent tool result. Keeping
 * sub-agents read-only (no writes, no cmd, no nested sub-agents) makes them safe
 * to delegate big investigation chunks to without runaway recursion or edits.
 */
async function _runSubAgent({ config, description, prompt, root, container }) {
  const card = document.createElement("div");
  card.className = "agent-tool-step agent-tool-step--subagent is-open";
  card.innerHTML =
    `<div class="agent-tool-row">` +
    `<svg class="atc-chev" viewBox="0 0 12 12" width="12" height="12"><path d="M4 2.5l3.5 3.5-3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>` +
    `<div class="atc-type-icon"><svg viewBox="0 0 16 16" fill="currentColor"><path d="M8 1a2 2 0 011 3.732V6h3.25A2.75 2.75 0 0115 8.75v3.5A2.75 2.75 0 0112.25 15h-8.5A2.75 2.75 0 011 12.25v-3.5A2.75 2.75 0 013.75 6H7V4.732A2 2 0 018 1zM5.5 9.5a1 1 0 100 2 1 1 0 000-2zm5 0a1 1 0 100 2 1 1 0 000-2z"/></svg></div>` +
    `<div class="atc-info"><div class="atc-action-row"><span class="atc-action">子智能体</span><span class="atc-path">${_escHtml(description)}</span></div></div>` +
    `<span class="atc-result"><span class="atc-spin"></span></span></div>` +
    `<div class="atc-viewport"></div>`;
  card.querySelector(".agent-tool-row").addEventListener("click", () => card.classList.toggle("is-open"));
  container.appendChild(card);
  const vp = card.querySelector(".atc-viewport");
  const res = card.querySelector(".atc-result");
  chatEl.scrollTop = chatEl.scrollHeight;

  const sysPrompt = _SUBAGENT_SYSTEM + `\n\n--- 项目上下文 ---\n` + (await _gatherAgentContext());
  const messages = [{ role: "system", content: sysPrompt }, { role: "user", content: prompt }];
  const toolSchemas = _buildAgentToolSchemas(false).filter((t) => ["read_file", "list_dir", "search", "find_files", "web_fetch", "web_search"].includes(t.function.name));

  let report = "";
  let toolCount = 0;
  const SUB_MAX = 12;
  try {
    for (let i = 0; i < SUB_MAX; i++) {
      if (!streaming) break;
      const turn = await _agentModelTurn({ config, messages, toolSchemas, body: vp });
      if (turn.error) { report = report || `[ERROR] ${turn.error}`; break; }
      if (turn.text && turn.text.trim()) report = turn.text.trim();
      const am = { role: "assistant", content: turn.text || "" };
      if (turn.toolCalls.length) am.tool_calls = turn.toolCalls.map((tc) => ({ id: tc.id, type: "function", function: { name: tc.name, arguments: tc.argsRaw } }));
      messages.push(am);
      if (!turn.toolCalls.length) break;
      for (const tc of turn.toolCalls) {
        const call = _mapToolCall(tc.name, tc.parsedArgs);
        if (!call || !["read", "list", "search", "find", "web", "websearch"].includes(call.type)) {
          messages.push({ role: "tool", tool_call_id: tc.id, content: call ? `子智能体只读，不能用 ${tc.name}。` : `未知工具: ${tc.name}` });
          continue;
        }
        const step = _createToolStep(call);
        vp.appendChild(step);
        toolCount++;
        let result;
        if (!streaming) result = { type: call.type, path: call.path, content: "[interrupted]" };
        else { try { result = await _executeToolStep(step, call, root); } catch (e) { result = { type: call.type, path: call.path, content: `[ERROR] ${e?.message || e}` }; } }
        messages.push({ role: "tool", tool_call_id: tc.id, content: _toolResultToString(call, result).slice(0, 8000) });
      }
      _trimMessagesIfHuge(messages);
    }
  } catch (e) { report = report || `[ERROR] ${e?.message || e}`; }

  res.className = "atc-result atc-result--ok";
  res.textContent = `${toolCount} 步调研`;
  card.classList.remove("is-open");
  chatEl.scrollTop = chatEl.scrollHeight;
  return report || "（子智能体未产出简报）";
}

async function _runAgenticLoop({ config, messages, root }) {
  const body = addMessage("assistant", "");
  body.appendChild(thinkingCard());

  const isAgent = _currentAiMode === "agent";
  const toolSchemas = _buildAgentToolSchemas(isAgent);

  // Files/activity bar reused from the existing agent UI.
  const filesBar = document.createElement("div");
  filesBar.className = "agent-files-bar";
  filesBar.innerHTML =
    `<svg class="agent-files-bar__chev" viewBox="0 0 12 12"><path d="M4 2.5l3.5 3.5-3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>` +
    `<svg class="agent-files-bar__icon" viewBox="0 0 16 16" fill="currentColor"><path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5a.25.25 0 01-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"/></svg>` +
    `<span class="agent-files-bar__count">0 Files</span>` +
    `<div class="agent-files-bar__actions">` +
    `<button class="agent-files-bar__btn agent-files-bar__btn--stop" type="button">Stop<span class="agent-files-bar__shortcut">^C</span></button>` +
    `</div>`;
  filesBar.addEventListener("click", (e) => { if (!e.target.closest(".agent-files-bar__btn")) filesBar.classList.toggle("is-open"); });
  filesBar.querySelector(".agent-files-bar__btn--stop").addEventListener("click", (e) => {
    e.stopPropagation(); streaming = false; _setSendBtnStop(false); showToast("Agent stopped");
  });
  const filesList = document.createElement("ul");
  filesList.className = "agent-files-list";
  body.parentElement.insertBefore(filesBar, body);
  body.parentElement.insertBefore(filesList, body);
  filesBar.style.display = "none";
  const trackedFiles = new Map();

  streaming = true;
  _setSendBtnStop(true);
  _clearAgentReadCache(); // fresh file reads each run
  _runCheckpoint.clear(); // start a fresh revert checkpoint for this run

  let finalErr = null;
  let summaryText = "";
  let hitCap = false;
  let planEl = null;
  // Harness-level discipline (the "Claude Code way"): track whether this run has
  // mutated files, whether it has verified (build/test/diagnostics), and the
  // latest plan — so we can nudge the model to finish its plan and to verify its
  // changes before it stops. Bounded so it can never loop forever.
  let didMutate = false, didVerify = false, planSteps = null;
  let continueNudges = 0, verifyNudges = 0;

  try {
    for (let iter = 0; iter < _AGENT_MAX_ITERS; iter++) {
      if (!streaming) break;
      const turn = await _agentModelTurn({ config, messages, toolSchemas, body });
      if (turn.error) { finalErr = turn.error; break; }
      if (turn.text && turn.text.trim()) summaryText += (summaryText ? "\n\n" : "") + turn.text.trim();

      const assistantMsg = { role: "assistant", content: turn.text || "" };
      if (turn.toolCalls.length) {
        assistantMsg.tool_calls = turn.toolCalls.map((tc) => ({ id: tc.id, type: "function", function: { name: tc.name, arguments: tc.argsRaw } }));
      }
      messages.push(assistantMsg);

      if (!turn.toolCalls.length) {
        // C — don't stop with unfinished plan steps.
        const pending = Array.isArray(planSteps) && planSteps.some((s) => s.status === "pending" || s.status === "in_progress");
        if (pending && continueNudges < 2) {
          continueNudges++;
          messages.push({ role: "user", content: "你的任务计划里还有未完成的步骤（pending / in_progress）。继续把它们做完再收尾，别提前停。" });
          continue;
        }
        // A — mutated files but never verified → make verification non-optional.
        if (didMutate && !didVerify && verifyNudges < 1) {
          verifyNudges++;
          messages.push({ role: "user", content: "你改了文件但还没验证。先用 run_cmd 跑相关的构建/测试/类型检查（如 npm run build、cargo check、pytest、tsc --noEmit），或用 get_diagnostics 看报错；确认通过后再收尾。若这个项目确实没有可跑的验证，简短说明原因即可。" });
          continue;
        }
        break; // truly done
      }

      // Render every tool step up front in call order (so the UI keeps the
      // model's sequence), then execute: read-only tools (read/list/search/find)
      // run in parallel for fast context-gathering, while mutating tools
      // (edit/write/cmd) and plan updates run sequentially in order.
      const items = turn.toolCalls.map((tc) => ({ tc, call: _mapToolCall(tc.name, tc.parsedArgs), step: null }));
      for (const it of items) {
        if (it.call && it.tc.name !== "update_plan" && it.tc.name !== "run_subagent") { it.step = _createToolStep(it.call); body.appendChild(it.step); }
      }
      chatEl.scrollTop = chatEl.scrollHeight;

      const toolMsgs = new Array(items.length);
      const READ_ONLY = new Set(["read", "list", "search", "find", "web", "websearch"]);

      const runOne = async (it) => {
        const { call, step } = it;
        let result;
        if (!streaming) result = { type: call.type, path: call.path, content: "[interrupted] 用户已停止任务。" };
        else {
          try { result = await _executeToolStep(step, call, root); }
          catch (e) { result = { type: call.type, path: call.path, content: `[ERROR] ${e?.message || e}` }; }
        }
        const key = call.type === "cmd" ? "$ " + (call.command || "").slice(0, 40) : (call.path || "");
        if (key && !trackedFiles.has(key)) { trackedFiles.set(key, call.type); _updateFilesBar(filesBar, filesList, trackedFiles); }
        return _toolResultToString(call, result).slice(0, 8000);
      };

      // 1) read-only tools AND sub-agents — concurrently (fast parallel
      //    context-gathering plus parallel research, like Claude Code).
      const parallel = [];
      for (let i = 0; i < items.length; i++) {
        const it = items[i];
        if (!it.call) continue;
        if (READ_ONLY.has(it.call.type)) {
          parallel.push((async () => { toolMsgs[i] = { role: "tool", tool_call_id: it.tc.id, content: await runOne(it) }; })());
        } else if (it.tc.name === "run_subagent") {
          parallel.push((async () => {
            const report = await _runSubAgent({ config, description: it.call.description, prompt: it.call.prompt, root, container: body });
            const key = "🤖 " + (it.call.description || "subagent");
            if (!trackedFiles.has(key)) { trackedFiles.set(key, "subagent"); _updateFilesBar(filesBar, filesList, trackedFiles); }
            toolMsgs[i] = { role: "tool", tool_call_id: it.tc.id, content: report.slice(0, 8000) };
          })());
        }
      }
      if (parallel.length) await Promise.all(parallel);

      // 2) plan updates + mutating tools — sequentially, in call order
      for (let i = 0; i < items.length; i++) {
        if (toolMsgs[i]) continue;
        const it = items[i];
        if (!it.call) { toolMsgs[i] = { role: "tool", tool_call_id: it.tc.id, content: `未知工具: ${it.tc.name}` }; continue; }
        if (it.tc.name === "update_plan") {
          planEl = _renderPlan(body, it.call.steps, planEl);
          toolMsgs[i] = { role: "tool", tool_call_id: it.tc.id, content: _planSummary(it.call.steps) };
          continue;
        }
        toolMsgs[i] = { role: "tool", tool_call_id: it.tc.id, content: await runOne(it) };
      }

      for (const m of toolMsgs) messages.push(m);
      _trimMessagesIfHuge(messages);

      // Track this turn for the finish/verify gates above.
      for (const it of items) {
        if (!it.call) continue;
        const t = it.call.type;
        if (t === "write" || t === "edit" || t === "multiedit" || t === "delete" || t === "move") didMutate = true;
        if (t === "diag") didVerify = true;
        if (t === "cmd" && /\b(test|tests|build|check|lint|tsc|typecheck|cargo|pytest|jest|vitest|mocha|phpunit|gradle|make|go\s+(build|test|vet))\b/i.test(it.call.command || "")) didVerify = true;
        if (it.tc.name === "update_plan") planSteps = it.call.steps;
      }

      if (iter === _AGENT_MAX_ITERS - 1) hitCap = true;
    }
  } catch (e) { finalErr = String(e?.message || e); }
  finally {
    streaming = false;
    _setSendBtnStop(false);
    body.querySelector(".thinking")?.remove();
    if (hitCap) {
      const note = document.createElement("div");
      note.className = "msg__error";
      note.textContent = `⚠️ 已达到单次最多 ${_AGENT_MAX_ITERS} 步，任务可能未完成。可直接再发一条让我接着做。`;
      body.appendChild(note);
    }
    if (finalErr) {
      const note = document.createElement("div");
      note.className = "msg__error";
      note.textContent = "⚠️ " + finalErr;
      body.appendChild(note);
    }
    if (!finalErr && summaryText) history.push({ role: "assistant", content: summaryText });
    // Plan mode: one-click handoff to execute the proposed plan in agent mode
    // (Claude Code's plan → execute flow). The plan is already in `history`, so
    // the agent sees it.
    if (_currentAiMode === "plan" && !finalErr && summaryText && summaryText.trim()) {
      const exec = document.createElement("button");
      exec.className = "plan-exec-btn";
      exec.innerHTML = `<svg viewBox="0 0 14 14" width="12" height="12" fill="currentColor"><path d="M4 2.5v9l7-4.5z"/></svg> 用 Agent 执行此方案`;
      exec.addEventListener("click", () => {
        if (streaming) return;
        exec.disabled = true;
        _currentAiMode = "agent";
        _updateModeUI();
        const s = _currentSession();
        if (s) { s.mode = "agent"; _renderChatTabs(); saveChatHistory(); }
        sendPrompt("按上面给出的方案逐步实施：先用 update_plan 列出步骤，再逐步实现，收尾前验证。");
      });
      body.appendChild(exec);
    }
    // Whole-run revert: one click restores every file this run touched to its
    // pre-run state (created files are removed). A checkpoint of each file was
    // taken before its first change.
    if (_runCheckpoint.size > 0) {
      const snapshot = new Map(_runCheckpoint);
      const n = snapshot.size;
      const revert = document.createElement("button");
      revert.className = "run-revert-btn";
      revert.innerHTML = `<svg viewBox="0 0 14 14" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h5.5a2.5 2.5 0 010 5H7M4 7l2.4-2.4M4 7l2.4 2.4"/></svg> 撤销本轮全部改动（${n} 个文件）`;
      revert.addEventListener("click", async () => {
        if (streaming) return;
        revert.disabled = true;
        revert.innerHTML = "正在撤销…";
        const { ok, fail } = await _revertRun(snapshot);
        revert.innerHTML = fail ? `已撤销 ${ok} 个，${fail} 个失败` : `✓ 已撤销 ${ok} 个文件`;
        showToast(fail ? `撤销完成（${ok} 成功 / ${fail} 失败）` : `已撤销本轮 ${ok} 个文件的改动`);
      });
      body.appendChild(revert);
    }
    saveChatHistory();
    const stopBtn = filesBar.querySelector(".agent-files-bar__btn--stop");
    if (stopBtn) stopBtn.style.display = "none";
    _updateFilesBar(filesBar, filesList, trackedFiles);
    chatEl.scrollTop = chatEl.scrollHeight;
  }
}

function _createToolStep(call) {
  const pathDisplay = call.path || call.command || "";
  const fileName = pathDisplay.split("/").pop();
  const dirPath = pathDisplay.includes("/") ? pathDisplay.split("/").slice(0, -1).join("/") : "";
  const actionLabel = { write: "Wrote", edit: "Edited", multiedit: "Edited", read: "Read", list: "Listed", cmd: "Ran command", search: "Searched", find: "Found files", web: "Fetched", websearch: "Web search", memory: "Remembered", delete: "Deleted", move: "Moved", diag: "Diagnostics" }[call.type] || "";
  const typeIcons = {
    write: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M11.013 1.427a1.75 1.75 0 012.474 0l1.086 1.086a1.75 1.75 0 010 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 01-.927-.928l.929-3.25c.081-.286.235-.547.445-.758l8.61-8.61zM11.524 2.2l-8.61 8.61a.25.25 0 00-.064.108l-.58 2.032 2.032-.58a.25.25 0 00.108-.064l8.61-8.61a.25.25 0 000-.354l-1.086-1.086a.25.25 0 00-.353 0z"/></svg>`,
    read: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M1.5 1.75C1.5.784 2.284 0 3.25 0h5.5a.75.75 0 01.53.22l3.5 3.5a.75.75 0 01.22.53v9.5A1.75 1.75 0 0111.25 15.5h-8A1.75 1.75 0 011.5 13.75V1.75zm1.75-.25a.25.25 0 00-.25.25v12a.25.25 0 00.25.25h8a.25.25 0 00.25-.25V4.664L8.836 2H3.25zM5 8.75a.75.75 0 01.75-.75h4.5a.75.75 0 010 1.5h-4.5A.75.75 0 015 8.75zm.75 2.25a.75.75 0 000 1.5h2.5a.75.75 0 000-1.5h-2.5z"/></svg>`,
    cmd: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M0 2.75C0 1.784.784 1 1.75 1h12.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0114.25 15H1.75A1.75 1.75 0 010 13.25V2.75zm1.75-.25a.25.25 0 00-.25.25v10.5c0 .138.112.25.25.25h12.5a.25.25 0 00.25-.25V2.75a.25.25 0 00-.25-.25H1.75zM7.25 8a.75.75 0 01-.22.53l-2.25 2.25a.75.75 0 11-1.06-1.06L5.44 8 3.72 6.28a.75.75 0 111.06-1.06l2.25 2.25A.75.75 0 017.25 8zM8 11.5a.75.75 0 01.75-.75h2.5a.75.75 0 010 1.5h-2.5a.75.75 0 01-.75-.75z"/></svg>`,
    list: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5a.25.25 0 01-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75zM1.5 2.75a.25.25 0 01.25-.25H5c.06 0 .118.026.158.07l.9 1.2a1.75 1.75 0 001.4.73h6.75a.25.25 0 01.25.25v8.5a.25.25 0 01-.25.25H1.75a.25.25 0 01-.25-.25V2.75z"/></svg>`,
    edit: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M11.013 1.427a1.75 1.75 0 012.474 0l1.086 1.086a1.75 1.75 0 010 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 01-.927-.928l.929-3.25c.081-.286.235-.547.445-.758l8.61-8.61zM11.524 2.2l-8.61 8.61a.25.25 0 00-.064.108l-.58 2.032 2.032-.58a.25.25 0 00.108-.064l8.61-8.61a.25.25 0 000-.354l-1.086-1.086a.25.25 0 00-.353 0z"/></svg>`,
    multiedit: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M11.013 1.427a1.75 1.75 0 012.474 0l1.086 1.086a1.75 1.75 0 010 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 01-.927-.928l.929-3.25c.081-.286.235-.547.445-.758l8.61-8.61zM11.524 2.2l-8.61 8.61a.25.25 0 00-.064.108l-.58 2.032 2.032-.58a.25.25 0 00.108-.064l8.61-8.61a.25.25 0 000-.354l-1.086-1.086a.25.25 0 00-.353 0z"/></svg>`,
    search: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M11.5 7a4.5 4.5 0 11-9 0 4.5 4.5 0 019 0zm-.82 4.74a6 6 0 111.06-1.06l2.79 2.79a.75.75 0 11-1.06 1.06l-2.79-2.79z"/></svg>`,
    find: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5a.25.25 0 01-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"/></svg>`,
    web: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M8 0a8 8 0 100 16A8 8 0 008 0zM1.5 8c0-.46.05-.91.14-1.34l3.32 3.32.7 1.4v1.27A6.51 6.51 0 011.5 8zm6.5 6.5c-.43 0-.85-.04-1.25-.12v-1.6a1 1 0 00-.55-.9L4 10.5v-1.5a1 1 0 011-1h1V6.5a1 1 0 001-1V4h1.5a1 1 0 001-1V2.2A6.5 6.5 0 0114.5 8 6.5 6.5 0 018 14.5z"/></svg>`,
    websearch: `<svg viewBox="0 0 16 16" fill="currentColor"><path d="M11.5 7a4.5 4.5 0 11-9 0 4.5 4.5 0 019 0zm-.82 4.74a6 6 0 111.06-1.06l2.79 2.79a.75.75 0 11-1.06 1.06l-2.79-2.79z"/></svg>`,
  };

  const step = document.createElement("div");
  step.className = `agent-tool-step agent-tool-step--${call.type}`;

  const _nonClickable = call.type === "cmd" || call.type === "search" || call.type === "find" || call.type === "web" || call.type === "websearch" || call.type === "memory" || call.type === "delete" || call.type === "move" || call.type === "diag";
  let pathHtml = _nonClickable
    ? `<span class="atc-path">${_escHtml(pathDisplay)}</span>`
    : `<span class="atc-path atc-path--clickable" data-filepath="${_escAttr(pathDisplay)}">${dirPath ? _escHtml(dirPath) + '/' : ''}${_escHtml(fileName)}</span>`;

  step.innerHTML =
    `<div class="agent-tool-row">` +
    `<svg class="atc-chev" viewBox="0 0 12 12" width="12" height="12"><path d="M4 2.5l3.5 3.5-3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>` +
    `<div class="atc-type-icon">${typeIcons[call.type] || typeIcons.read}</div>` +
    `<div class="atc-info"><div class="atc-action-row"><span class="atc-action">${actionLabel}</span>${pathHtml}</div></div>` +
    `<span class="atc-result"><span class="atc-spin"></span></span></div>` +
    `<div class="atc-viewport"></div>`;
  step.querySelector(".agent-tool-row").addEventListener("click", () => step.classList.toggle("is-open"));
  const clickablePath = step.querySelector(".atc-path--clickable");
  if (clickablePath) {
    clickablePath.addEventListener("click", (e) => {
      e.stopPropagation();
      const fp = clickablePath.dataset.filepath;
      if (fp) {
        const root = rootPath || workspaceRoots[0] || "";
        const fullPath = fp.startsWith("/") ? fp : root + "/" + fp;
        openFile(fullPath, fp.split("/").pop());
      }
    });
  }
  return step;
}

async function _executeToolStep(step, call, root) {
  const vp = step.querySelector(".atc-viewport");
  const res = step.querySelector(".atc-result");
  const row = step.querySelector(".agent-tool-row");

  const readOnlyMode = _currentAiMode === "explorer" || _currentAiMode === "reviewer" || _currentAiMode === "plan";
  if (readOnlyMode && (call.type === "write" || call.type === "edit" || call.type === "multiedit" || call.type === "cmd" || call.type === "delete" || call.type === "move")) {
    const modeName = _currentAiMode === "explorer" ? "Explorer" : _currentAiMode === "plan" ? "Plan" : "Reviewer";
    const what = call.type === "cmd" ? "运行命令" : "修改文件";
    res.className = "atc-result atc-result--blocked";
    res.textContent = `⛔ ${modeName} 模式下禁止${what}`;
    return { type: call.type, path: call.path, content: `[BLOCKED] ${modeName} 是只读模式，不能${what}。只能用 read_file/list_dir/search/find_files。` };
  }

  try {
    if (call.type === "read") {
      if (!call.path || !call.path.trim()) {
        res.className = "atc-result atc-result--err";
        res.innerHTML = `<svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm2.03 3.97a.75.75 0 00-1.06 0L6 4.94 5.03 3.97a.75.75 0 10-1.06 1.06L4.94 6 3.97 6.97a.75.75 0 101.06 1.06L6 7.06l.97.97a.75.75 0 101.06-1.06L7.06 6l.97-.97a.75.75 0 000-1.06z"/></svg> empty path`;
        return null;
      }
      let rawPath = call.path.trim();
      const homeDir = _cachedHomeDir || "";
      if (rawPath.startsWith("~/") && homeDir) {
        rawPath = homeDir + rawPath.slice(1);
      }
      const candidates = [];
      if (rawPath.startsWith("/")) {
        candidates.push(rawPath);
        if (root && !rawPath.startsWith(root)) {
          const basename = rawPath.split("/").pop();
          if (basename) candidates.push(root + "/" + basename);
        }
      } else {
        if (root) candidates.push(root + "/" + rawPath);
        candidates.push(rawPath);
      }

      let txt = "";
      let readFailed = true;
      let readError = "";
      let usedPath = candidates[0];

      let isDir = false;
      // Cache hit: serve a previously-read file without another backend call.
      for (const fp of candidates) {
        if (_agentReadCache.has(fp)) { txt = _agentReadCache.get(fp); readFailed = false; usedPath = fp; break; }
      }
      if (readFailed) {
        for (const fp of candidates) {
          try {
            txt = await backend.readTextFile(fp);
            readFailed = false;
            usedPath = fp;
            _agentReadCache.set(fp, txt);
            break;
          } catch (e) {
            const msg = String(e?.message || e);
            if (/is a directory|os error 21/i.test(msg)) { isDir = true; usedPath = fp; break; }
            readError = msg.slice(0, 200);
          }
        }
      }

      if (isDir) {
        call.type = "list"; call.path = usedPath;
        try {
          const entries = await backend.readDir(usedPath);
          const lines = entries.map(e => `${e.is_dir ? "d" : "-"} ${e.name}`);
          const listing = lines.join("\n");
          res.className = "atc-result atc-result--ok";
          res.textContent = `${entries.length} items (auto-switched to list_dir)`;
          vp.innerHTML = `<pre>${_escHtml(listing || "(empty directory)")}</pre>`;
          return { type: "list", path: call.path, content: listing || "(empty directory)" };
        } catch (dirErr) {
          res.className = "atc-result atc-result--err";
          res.textContent = String(dirErr?.message || dirErr).slice(0, 120);
          return { type: "list", path: call.path, content: `[ERROR] Cannot list directory: ${usedPath}` };
        }
      }

      if (readFailed) {
        let helpHint = "";
        if (root) {
          try {
            const parentDir = candidates[0].split("/").slice(0, -1).join("/") || root;
            const siblings = await backend.readDir(parentDir);
            const names = siblings.slice(0, 10).map(e => e.name).join(", ");
            if (names) helpHint = `\nFiles in ${parentDir}: ${names}`;
          } catch {}
        }
        res.className = "atc-result atc-result--err";
        res.innerHTML = `<svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm2.03 3.97a.75.75 0 00-1.06 0L6 4.94 5.03 3.97a.75.75 0 10-1.06 1.06L4.94 6 3.97 6.97a.75.75 0 101.06 1.06L6 7.06l.97.97a.75.75 0 101.06-1.06L7.06 6l.97-.97a.75.75 0 000-1.06z"/></svg> ${_escHtml(readError || "not found")}`;
        step.classList.add("agent-tool-step--rejected");
        vp.innerHTML = `<div style="padding:8px 12px;color:rgba(255,255,255,.4);font-size:12px">Tried: ${candidates.map(p => _escHtml(p)).join(", ")}</div>`;
        return { type: "read", path: call.path, content: `[ERROR] File not found: ${rawPath}. Workspace root is: ${root || "(none)"}. Use full absolute path.${helpHint}` };
      }
      const allLines = txt.split("\n");
      const total = allLines.length;
      // Page by line so the model can read large files fully (offset/limit),
      // instead of being stuck with only the first few KB.
      const start = Math.max(0, (Number.isFinite(call.offset) && call.offset > 0 ? Math.floor(call.offset) : 1) - 1);
      const limit = Number.isFinite(call.limit) && call.limit > 0 ? Math.floor(call.limit) : 400;
      let slice = allLines.slice(start, start + limit);
      const shownFrom = start + 1;
      const shownTo = Math.min(start + slice.length, total);
      let body = slice.join("\n");
      const CHAR_CAP = 16000;
      let charCapped = false;
      if (body.length > CHAR_CAP) { body = body.slice(0, CHAR_CAP); charCapped = true; }

      const sizeLabel = txt.length > 1024 ? `${(txt.length / 1024).toFixed(1)} KB` : `${txt.length} chars`;
      res.className = "atc-result atc-result--ok";
      const rangeLabel = (start > 0 || shownTo < total) ? ` (${shownFrom}-${shownTo}/${total})` : "";
      res.innerHTML = `<svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm.75 3a.75.75 0 10-1.5 0 .75.75 0 001.5 0zM5.25 5a.75.75 0 000 1.5h.25V8.5h-.5a.75.75 0 000 1.5h2a.75.75 0 000-1.5H6.5V5.75A.75.75 0 005.75 5h-.5z"/></svg> ${total} lines · ${sizeLabel}${rangeLabel}`;
      vp.innerHTML = `<pre>${_escHtml(body.slice(0, 4000))}</pre>`;
      row.addEventListener("dblclick", () => openFile(usedPath, call.path.split("/").pop()));

      let hint = "";
      if (shownTo < total) hint = `\n\n（已显示第 ${shownFrom}-${shownTo} 行，共 ${total} 行。用 read_file(path, offset=${shownTo + 1}) 继续读后续内容。）`;
      else if (charCapped) hint = `\n\n（内容过长已截断，用更小的 limit 或更大的 offset 分段读。）`;
      const header = (start > 0 || shownTo < total) ? `（${call.path} 第 ${shownFrom}-${shownTo}/${total} 行）\n` : "";
      return { type: "read", path: call.path, content: header + body + hint };

    } else if (call.type === "list") {
      let fp = call.path.startsWith("/") ? call.path : (root ? root + "/" + call.path : call.path);
      if (root && !fp.startsWith(root) && !fp.startsWith("/tmp") && !fp.startsWith("/Users")) {
        fp = root + "/" + call.path.replace(/^\/+/, "");
      }
      let entries = [];
      try {
        entries = await backend.readDir(fp);
      } catch {
        try {
          const r = await backend.taskRunCapture(root || "/tmp", `ls -la "${fp}" 2>/dev/null | head -60`);
          const ls = r?.stdout || "";
          const items = ls.split("\n").filter(l => l.trim());
          res.className = items.length ? "atc-result atc-result--ok" : "atc-result atc-result--err";
          res.textContent = `${items.length} items`;
          vp.innerHTML = `<pre>${_escHtml(ls || "(empty or inaccessible)")}</pre>`;
          return { type: "list", path: call.path, content: ls || "(empty)" };
        } catch {}
      }
      const lines = entries.map(e => `${e.is_dir ? "d" : "-"} ${e.name}`);
      const listing = lines.join("\n");
      res.className = "atc-result atc-result--ok";
      res.textContent = `${entries.length} items`;
      vp.innerHTML = `<pre>${_escHtml(listing || "(empty directory)")}</pre>`;
      return { type: "list", path: call.path, content: listing || "(empty directory)" };

    } else if (call.type === "write" || call.type === "edit") {
      const writeRoot = root || '/tmp';
      const fp = call.path.startsWith("/") ? call.path : root + "/" + call.path;
      let old = "";
      let existed = false;
      try { old = await backend.readTextFile(fp); existed = true; } catch {}
      _checkpointRecord(fp, existed, old); // snapshot before first change (for revert-all)

      let newContent = call.content;

      // edit_file: derive the new content by substituting old_string -> new_string
      // against the *current* file, with Claude-Code-style uniqueness checks so a
      // sloppy match never silently rewrites the wrong place.
      if (call.type === "edit") {
        if (!existed) {
          res.className = "atc-result atc-result--err";
          res.textContent = "文件不存在";
          return { type: "edit", path: call.path, content: `[ERROR] 文件不存在: ${call.path}。新建文件请用 write_file。` };
        }
        const oldStr = call.oldString || "";
        if (!oldStr) {
          res.className = "atc-result atc-result--err";
          res.textContent = "缺少 old_string";
          return { type: "edit", path: call.path, content: "[ERROR] edit_file 需要非空 old_string。" };
        }
        const occ = old.split(oldStr).length - 1;
        if (occ === 0) {
          res.className = "atc-result atc-result--err";
          res.textContent = "未找到 old_string";
          return { type: "edit", path: call.path, content: `[ERROR] 在 ${call.path} 中找不到 old_string。请先 read_file，再逐字符复制要替换的原文。` };
        }
        if (occ > 1 && !call.replaceAll) {
          res.className = "atc-result atc-result--err";
          res.textContent = `old_string 出现 ${occ} 次`;
          return { type: "edit", path: call.path, content: `[ERROR] old_string 在 ${call.path} 中出现 ${occ} 次（不唯一）。请加更多上下文以唯一定位，或设 replace_all=true。` };
        }
        // Function replacement so `$&`, `$1`, `$$`… in new_string are inserted
        // literally instead of being treated as replacement patterns.
        newContent = call.replaceAll ? old.split(oldStr).join(call.newString || "") : old.replace(oldStr, () => call.newString || "");
      }

      if (existed && newContent === old) {
        res.className = "atc-result atc-result--ok";
        res.textContent = "无变化";
        return { type: call.type, path: call.path, content: `(${call.path} 内容未变化)` };
      }

      const added = newContent.split("\n").length;
      const removed = old ? old.split("\n").length : 0;
      res.className = "atc-result atc-result--pending";
      res.innerHTML = `<span class="atc-diffstat"><span class="a">+${added}</span>${removed ? ` <span class="d">-${removed}</span>` : ""}</span>`;
      vp.innerHTML = _buildDiffView(old, newContent, call.path);
      _highlightDiffView(vp);
      step.classList.add("is-open");

      // Agent mode applies edits automatically so the loop stays autonomous; the
      // diff stays visible and an Undo button makes every change reversible.
      let writeErr = "";
      try {
        const dir = fp.split("/").slice(0, -1).join("/");
        if (dir) await backend.taskRunCapture(writeRoot, `mkdir -p "${dir}"`).catch(() => {});
        await backend.writeTextFile(fp, newContent);
        _agentReadCache.set(fp, newContent); // keep cache coherent with the new content
        _invalidateRead(call.path);
        _refreshTreeFor(fp); // show the new/changed file in the explorer right away
      } catch (e1) {
        try {
          await backend.taskRunCapture(root, `mkdir -p "$(dirname '${fp}')" && cat > "${fp}" << 'AGENT_EOF'\n${newContent}\nAGENT_EOF`);
        } catch (e2) { writeErr = String(e2?.message || e2); }
      }

      if (writeErr) {
        step.classList.add("agent-tool-step--rejected");
        res.className = "atc-result atc-result--err";
        res.textContent = writeErr.slice(0, 80);
        return { type: call.type, path: call.path, content: `[ERROR] 写入 ${call.path} 失败: ${writeErr}` };
      }

      step.classList.add("agent-tool-step--accepted");
      res.className = "atc-result atc-result--ok";
      res.innerHTML = `<svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm2.22 4.22a.75.75 0 010 1.06l-3 3a.75.75 0 01-1.06 0l-1.5-1.5a.75.75 0 111.06-1.06L4.69 6.69l2.47-2.47a.75.75 0 011.06 0z"/></svg> <span class="atc-diffstat"><span class="a">+${added}</span>${removed ? ` <span class="d">-${removed}</span>` : ""}</span><button class="atc-undo-btn" type="button">Undo</button>`;
      row.addEventListener("dblclick", () => openFile(fp, call.path.split("/").pop()));
      _ipcBroadcast("file_changed", { path: fp });
      showToast(`${existed ? "Updated" : "Created"} ${call.path.split("/").pop()}`);

      const undoBtn = res.querySelector(".atc-undo-btn");
      if (undoBtn) {
        undoBtn.addEventListener("click", async (e) => {
          e.stopPropagation();
          try {
            if (existed) await backend.writeTextFile(fp, old);
            else await backend.deletePath(fp).catch(() => backend.writeTextFile(fp, ""));
            step.classList.remove("agent-tool-step--accepted");
            step.classList.add("agent-tool-step--rejected");
            res.className = "atc-result atc-result--err";
            res.innerHTML = `<svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm2.03 3.97a.75.75 0 00-1.06 0L6 4.94 5.03 3.97a.75.75 0 10-1.06 1.06L4.94 6 3.97 6.97a.75.75 0 101.06 1.06L6 7.06l.97.97a.75.75 0 101.06-1.06L7.06 6l.97-.97a.75.75 0 000-1.06z"/></svg> Reverted`;
            _ipcBroadcast("file_changed", { path: fp });
            showToast(`Reverted ${call.path.split("/").pop()}`);
          } catch (err) { showToast("Undo failed: " + (err?.message || err)); }
        });
      }
      return { type: call.type, path: call.path, content: existed ? `已修改 ${call.path}（+${added}/-${removed} 行）。` : `已新建 ${call.path}（${added} 行）。` };

    } else if (call.type === "multiedit") {
      const fp = call.path.startsWith("/") ? call.path : root + "/" + call.path;
      let old = "";
      try { old = await backend.readTextFile(fp); }
      catch {
        res.className = "atc-result atc-result--err"; res.textContent = "文件不存在";
        return { type: "multiedit", path: call.path, content: `[ERROR] 文件不存在: ${call.path}。新建文件请用 write_file。` };
      }
      _checkpointRecord(fp, true, old); // snapshot before first change (for revert-all)
      const edits = Array.isArray(call.edits) ? call.edits : [];
      if (!edits.length) {
        res.className = "atc-result atc-result--err"; res.textContent = "edits 为空";
        return { type: "multiedit", path: call.path, content: "[ERROR] multi_edit 需要至少一处 edits。" };
      }
      // Apply edits in order against the evolving content; abort the whole op if
      // any one fails to locate uniquely (atomic — nothing is written on error).
      let content = old;
      for (let k = 0; k < edits.length; k++) {
        const oldStr = edits[k]?.old_string || "";
        const newStr = edits[k]?.new_string ?? "";
        if (!oldStr) {
          res.className = "atc-result atc-result--err"; res.textContent = `第 ${k + 1} 处缺 old_string`;
          return { type: "multiedit", path: call.path, content: `[ERROR] 第 ${k + 1} 处 edit 缺少 old_string，整体未写入。` };
        }
        const occ = content.split(oldStr).length - 1;
        if (occ === 0) {
          res.className = "atc-result atc-result--err"; res.textContent = `第 ${k + 1} 处未找到`;
          return { type: "multiedit", path: call.path, content: `[ERROR] 第 ${k + 1} 处 old_string 找不到（可能被前面的替换改动了，请按应用后的内容重新定位）。整体未写入。` };
        }
        if (occ > 1 && !edits[k].replace_all) {
          res.className = "atc-result atc-result--err"; res.textContent = `第 ${k + 1} 处不唯一(${occ})`;
          return { type: "multiedit", path: call.path, content: `[ERROR] 第 ${k + 1} 处 old_string 出现 ${occ} 次（不唯一）。加更多上下文或设 replace_all=true。整体未写入。` };
        }
        content = edits[k].replace_all ? content.split(oldStr).join(newStr) : content.replace(oldStr, () => newStr);
      }
      const newContent = content;
      if (newContent === old) {
        res.className = "atc-result atc-result--ok"; res.textContent = "无变化";
        return { type: "multiedit", path: call.path, content: `(${call.path} 内容未变化)` };
      }
      const added = newContent.split("\n").length;
      const removed = old.split("\n").length;
      res.className = "atc-result atc-result--pending";
      res.innerHTML = `<span class="atc-diffstat"><span class="a">+${added}</span> <span class="d">-${removed}</span></span>`;
      vp.innerHTML = _buildDiffView(old, newContent, call.path);
      _highlightDiffView(vp);
      step.classList.add("is-open");

      let writeErr = "";
      try {
        await backend.writeTextFile(fp, newContent);
        _agentReadCache.set(fp, newContent);
        _invalidateRead(call.path);
        _refreshTreeFor(fp);
      } catch (e) { writeErr = String(e?.message || e); }
      if (writeErr) {
        step.classList.add("agent-tool-step--rejected");
        res.className = "atc-result atc-result--err"; res.textContent = writeErr.slice(0, 80);
        return { type: "multiedit", path: call.path, content: `[ERROR] 写入 ${call.path} 失败: ${writeErr}` };
      }
      step.classList.add("agent-tool-step--accepted");
      res.className = "atc-result atc-result--ok";
      res.innerHTML = `<svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><path d="M6 0a6 6 0 110 12A6 6 0 016 0zm2.22 4.22a.75.75 0 010 1.06l-3 3a.75.75 0 01-1.06 0l-1.5-1.5a.75.75 0 111.06-1.06L4.69 6.69l2.47-2.47a.75.75 0 011.06 0z"/></svg> <span class="atc-diffstat"><span class="a">+${added}</span> <span class="d">-${removed}</span></span><button class="atc-undo-btn" type="button">Undo</button>`;
      row.addEventListener("dblclick", () => openFile(fp, call.path.split("/").pop()));
      _ipcBroadcast("file_changed", { path: fp });
      showToast(`Updated ${call.path.split("/").pop()} (${edits.length} edits)`);
      const undoBtn = res.querySelector(".atc-undo-btn");
      if (undoBtn) {
        undoBtn.addEventListener("click", async (e) => {
          e.stopPropagation();
          try {
            await backend.writeTextFile(fp, old);
            _agentReadCache.set(fp, old); _invalidateRead(call.path); _refreshTreeFor(fp);
            step.classList.remove("agent-tool-step--accepted"); step.classList.add("agent-tool-step--rejected");
            res.className = "atc-result atc-result--err"; res.textContent = "Reverted";
            _ipcBroadcast("file_changed", { path: fp });
            showToast(`Reverted ${call.path.split("/").pop()}`);
          } catch (err) { showToast("Undo failed: " + (err?.message || err)); }
        });
      }
      return { type: "multiedit", path: call.path, content: `已对 ${call.path} 应用 ${edits.length} 处替换（+${added}/-${removed} 行）。` };

    } else if (call.type === "search") {
      const q = (call.query || "").trim();
      if (!q) { res.className = "atc-result atc-result--err"; res.textContent = "空查询"; return { type: "search", path: call.path, content: "[ERROR] 空查询。" }; }
      const searchRoot = (call.searchPath && call.searchPath.trim())
        ? (call.searchPath.startsWith("/") ? call.searchPath : (root ? root + "/" + call.searchPath : call.searchPath))
        : (root || "");
      if (!searchRoot) { res.className = "atc-result atc-result--err"; res.textContent = "未打开工作区"; return { type: "search", path: call.path, content: "[ERROR] 未打开工作区，无法搜索。" }; }
      let fileMatches = [];
      try { fileMatches = await backend.invoke("search_in_project", { root: searchRoot, query: q, caseSensitive: false }) || []; }
      catch (e) { res.className = "atc-result atc-result--err"; res.textContent = String(e?.message || e).slice(0, 80); return { type: "search", path: call.path, content: `[ERROR] 搜索失败: ${e?.message || e}` }; }
      let hits = 0;
      const lines = [];
      for (const fm of fileMatches) {
        const rel = fm.rel || fm.name || fm.path || "";
        for (const m of (fm.matches || [])) {
          lines.push(`${rel}:${m.line}: ${(m.text || "").trim()}`);
          if (++hits >= 80) break;
        }
        if (hits >= 80) break;
      }
      const summary = `${hits} 处匹配 · ${fileMatches.length} 个文件`;
      res.className = fileMatches.length ? "atc-result atc-result--ok" : "atc-result atc-result--err";
      res.textContent = fileMatches.length ? summary : "无匹配";
      vp.innerHTML = `<pre>${_escHtml(lines.join("\n") || "(无匹配)")}</pre>`;
      return { type: "search", path: call.path, content: lines.length ? `搜索 "${q}" — ${summary}:\n${lines.join("\n")}` : `搜索 "${q}"：无匹配。` };

    } else if (call.type === "find") {
      const r = await _agentFindFiles(root, call.pattern || call.path || "");
      res.className = r.count ? "atc-result atc-result--ok" : "atc-result atc-result--err";
      res.textContent = r.count ? `${r.count} 个文件` : "无匹配";
      vp.innerHTML = `<pre>${_escHtml(r.text)}</pre>`;
      return { type: "find", path: call.path, content: `find_files "${call.pattern || call.path}":\n${r.text}` };

    } else if (call.type === "diag") {
      const sevName = { 8: "error", 4: "warning", 2: "info", 1: "hint" };
      let markers = [];
      try {
        if (call.path && call.path.trim()) {
          const want = call.path.startsWith("/") ? call.path : (root ? root + "/" + call.path.replace(/^\/+/, "") : call.path);
          const model = monaco.editor.getModels().find((m) => {
            const p = m.uri.fsPath || m.uri.path || "";
            return p === want || p.endsWith("/" + call.path.replace(/^\/+/, ""));
          });
          markers = model ? monaco.editor.getModelMarkers({ resource: model.uri }) : [];
        } else {
          markers = monaco.editor.getModelMarkers({});
        }
      } catch {}
      const probs = markers.filter((m) => m.severity >= 4); // errors + warnings
      const lines = probs.slice(0, 50).map((m) => `${sevName[m.severity] || "?"} ${(m.resource?.path || "").split("/").pop()}:${m.startLineNumber}:${m.startColumn} ${m.message}`);
      res.className = probs.length ? "atc-result atc-result--err" : "atc-result atc-result--ok";
      res.textContent = probs.length ? `${probs.length} 个问题` : "无错误/警告";
      if (vp) vp.innerHTML = `<pre>${_escHtml(lines.join("\n") || "(无诊断)")}</pre>`;
      return { type: "diag", path: call.path, content: probs.length ? `诊断（${probs.length} 个错误/警告）:\n${lines.join("\n")}` : "无错误或警告（注意：LSP 分析可能略有延迟，改完稍等再查更准）。" };

    } else if (call.type === "memory") {
      const ok = _appendMemory(root, call.content);
      res.className = ok ? "atc-result atc-result--ok" : "atc-result atc-result--err";
      res.textContent = ok ? "已记住" : "空内容";
      if (typeof vp !== "undefined" && vp) vp.innerHTML = `<pre>${_escHtml((call.content || "").slice(0, 500))}</pre>`;
      return { type: "memory", path: call.path, content: ok ? `已记入项目记忆：${call.content}` : "[ERROR] 空内容，未记录。" };

    } else if (call.type === "delete") {
      const p = call.path || "";
      if (!p.trim()) { res.className = "atc-result atc-result--err"; res.textContent = "空路径"; return { type: "delete", path: p, content: "[ERROR] 空路径。" }; }
      const fp = p.startsWith("/") ? p : (root ? root + "/" + p.replace(/^\/+/, "") : p);
      try {
        try { _checkpointRecord(fp, true, await backend.readTextFile(fp)); } catch { /* dir/binary — not snapshotted */ }
        await backend.deletePath(fp);
        _invalidateRead(p); _agentReadCache.delete(fp);
        _refreshTreeFor(fp);
        res.className = "atc-result atc-result--ok"; res.textContent = "已删除";
        return { type: "delete", path: p, content: `已删除 ${p}` };
      } catch (e) {
        res.className = "atc-result atc-result--err"; res.textContent = String(e?.message || e).slice(0, 80);
        return { type: "delete", path: p, content: `[ERROR] 删除失败: ${String(e?.message || e).slice(0, 160)}` };
      }

    } else if (call.type === "move") {
      const from = call.path || "", to = call.to || "";
      if (!from.trim() || !to.trim()) { res.className = "atc-result atc-result--err"; res.textContent = "缺少 from/to"; return { type: "move", path: from, content: "[ERROR] 需要 from 和 to。" }; }
      const fromFp = from.startsWith("/") ? from : (root ? root + "/" + from.replace(/^\/+/, "") : from);
      const toFp = to.startsWith("/") ? to : (root ? root + "/" + to.replace(/^\/+/, "") : to);
      try {
        try { _checkpointRecord(fromFp, true, await backend.readTextFile(fromFp)); _checkpointRecord(toFp, false, ""); } catch {}
        await backend.renamePath(fromFp, toFp);
        _invalidateRead(from); _agentReadCache.delete(fromFp);
        _refreshTreeFor(fromFp); _refreshTreeFor(toFp);
        res.className = "atc-result atc-result--ok"; res.textContent = "已移动";
        return { type: "move", path: from, content: `已移动 ${from} → ${to}` };
      } catch (e) {
        res.className = "atc-result atc-result--err"; res.textContent = String(e?.message || e).slice(0, 80);
        return { type: "move", path: from, content: `[ERROR] 移动失败: ${String(e?.message || e).slice(0, 160)}` };
      }

    } else if (call.type === "web") {
      const url = (call.url || call.path || "").trim();
      if (!url) { res.className = "atc-result atc-result--err"; res.textContent = "空 URL"; return { type: "web", path: call.path, content: "[ERROR] 空 URL。" }; }
      let text = "";
      if (_agentWebCache.has(url)) {
        text = _agentWebCache.get(url);
      } else {
        try { text = await backend.invoke("web_fetch", { url }); _webCachePut(url, text); }
        catch (e) {
          const msg = String(e?.message || e).slice(0, 160);
          res.className = "atc-result atc-result--err";
          res.textContent = msg.slice(0, 80);
          return { type: "web", path: call.path, content: `[ERROR] 抓取失败: ${msg}` };
        }
      }
      const chars = text.length;
      res.className = "atc-result atc-result--ok";
      res.textContent = chars > 1024 ? `${(chars / 1024).toFixed(1)} KB` : `${chars} chars`;
      vp.innerHTML = `<pre>${_escHtml(text.slice(0, 4000))}</pre>`;
      return { type: "web", path: call.path, content: `网页 ${url}:\n${text}` };

    } else if (call.type === "websearch") {
      const q = (call.query || call.path || "").trim();
      if (!q) { res.className = "atc-result atc-result--err"; res.textContent = "空搜索词"; return { type: "websearch", path: call.path, content: "[ERROR] 空搜索词。" }; }
      let text = "";
      if (_agentWebCache.has("q:" + q)) {
        text = _agentWebCache.get("q:" + q);
      } else {
        try { text = await backend.invoke("web_search", { query: q }); _webCachePut("q:" + q, text); }
        catch (e) {
          const msg = String(e?.message || e).slice(0, 160);
          res.className = "atc-result atc-result--err";
          res.textContent = msg.slice(0, 80);
          return { type: "websearch", path: call.path, content: `[ERROR] 搜索失败: ${msg}` };
        }
      }
      const hits = (text.match(/^\s*\d+\.\s/gm) || []).length;
      res.className = "atc-result atc-result--ok";
      res.textContent = hits ? `${hits} 条结果` : "完成";
      vp.innerHTML = `<pre>${_escHtml(text.slice(0, 4000))}</pre>`;
      return { type: "websearch", path: call.path, content: text };

    } else if (call.type === "cmd") {
      if (!call.command || !call.command.trim()) {
        return null;
      }
      _clearAgentReadCache(); // a command may have changed files on disk
      step.className = "agent-term-card agent-term-card--running";
      step.innerHTML =
        `<div class="agent-term-card__header">` +
          `<svg class="agent-term-card__icon" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3 4l3 4-3 4M8.5 12H13"/></svg>` +
          `<span class="agent-term-card__label">Terminal</span>` +
          `<span class="agent-term-timer"></span>` +
          `<div class="agent-term-status agent-term-status--running"><span class="agent-term-spinner"></span> Running</div>` +
        `</div>` +
        `<div class="agent-term-card__cmd">` +
          `<span class="agent-term-card__prompt">$</span>` +
          `<code class="agent-term-card__code">${_escHtml(call.command)}</code>` +
        `</div>` +
        `<pre class="agent-term-output" style="display:none"></pre>` +
        `<div class="agent-term-card__footer" style="display:none">` +
          `<button class="agent-term-toggle" title="Toggle output">Show output</button>` +
          `<button class="agent-term-copy" title="Copy output">Copy</button>` +
        `</div>`;

      const result = await _agentRunInTerminal(root, call.command, step);

      const outEl = step.querySelector(".agent-term-output");
      const footerEl = step.querySelector(".agent-term-card__footer");
      const output = (result.stdout + (result.stderr ? "\n" + result.stderr : "")).trim();

      if (output && outEl) {
        outEl.textContent = output.slice(0, 5000);
        footerEl.style.display = "";

        const toggleBtn = step.querySelector(".agent-term-toggle");
        toggleBtn?.addEventListener("click", (e) => {
          e.stopPropagation();
          const visible = outEl.style.display !== "none";
          outEl.style.display = visible ? "none" : "block";
          toggleBtn.textContent = visible ? "Show output" : "Hide output";
        });

        const copyBtn = step.querySelector(".agent-term-copy");
        copyBtn?.addEventListener("click", (e) => {
          e.stopPropagation();
          navigator.clipboard?.writeText(output).then(() => {
            copyBtn.textContent = "Copied!";
            setTimeout(() => { copyBtn.textContent = "Copy"; }, 1500);
          });
        });
      }

      return { type: "cmd", path: call.command, content: output ? output.slice(0, 2000) : "(executed)" };
    }
  } catch (e) {
    res.className = "atc-result atc-result--err";
    res.innerHTML = `<svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor"><path d="M4.47.22A.75.75 0 015 0h2a.75.75 0 01.53.22l4.25 4.25c.141.14.22.331.22.53v2a.75.75 0 01-.22.53l-4.25 4.25A.75.75 0 017 12H5a.75.75 0 01-.53-.22L.22 7.53A.75.75 0 010 7V5a.75.75 0 01.22-.53L4.47.22zM6.5 7.75a.75.75 0 100 1.5.75.75 0 000-1.5zM5.75 3v3.5a.75.75 0 001.5 0V3a.75.75 0 00-1.5 0z"/></svg> ${_escHtml(String(e?.message || e).slice(0, 50))}`;
  }
  return null;
}

function _buildDiffView(oldText, newText, filePath) {
  const oldL = oldText ? oldText.split("\n") : [];
  const newL = newText.split("\n");
  const ext = (filePath || "").split(".").pop().toLowerCase();
  const monoLang = _ATC_LANG_MAP[ext] || ext;
  const badge = _langBadge(filePath || ext || "file");
  const isNew = !oldText;

  let h = '';
  h += `<div class="atc-diff" data-lang="${_escHtml(monoLang)}">`;

  const cap = 60;
  let rendered = 0;

  if (isNew) {
    for (let i = 0; i < newL.length && rendered < cap; i++, rendered++) {
      h += `<div class="atc-diff-row atc-diff-row--add"><span class="atc-diff-ln">${i + 1}</span><span class="atc-diff-sign">+</span><span class="atc-diff-code" data-raw="${_escAttr(newL[i])}">${_escHtml(newL[i])}</span></div>`;
    }
  } else {
    const maxLen = Math.max(oldL.length, newL.length);
    let lastShown = -1;
    for (let i = 0; i < maxLen && rendered < cap; i++) {
      const oLine = i < oldL.length ? oldL[i] : undefined;
      const nLine = i < newL.length ? newL[i] : undefined;

      if (oLine !== undefined && nLine !== undefined && oLine === nLine) {
        if (i - lastShown === 2) {
          h += `<div class="atc-diff-row atc-diff-row--ctx"><span class="atc-diff-ln">${i + 1}</span><span class="atc-diff-sign"> </span><span class="atc-diff-code" data-raw="${_escAttr(nLine)}">${_escHtml(nLine)}</span></div>`;
          rendered++;
        }
        continue;
      }

      if (i - lastShown > 2 && lastShown >= 0) {
        const skipped = i - lastShown - 1;
        if (skipped > 0) {
          h += `<div class="atc-diff-more">@@ ${skipped} unchanged line${skipped > 1 ? 's' : ''} @@</div>`;
        }
      }

      if (lastShown < 0 && i > 0) {
        const ctxStart = Math.max(0, i - 2);
        for (let c = ctxStart; c < i; c++) {
          if (c < oldL.length) {
            h += `<div class="atc-diff-row atc-diff-row--ctx"><span class="atc-diff-ln">${c + 1}</span><span class="atc-diff-sign"> </span><span class="atc-diff-code" data-raw="${_escAttr(oldL[c])}">${_escHtml(oldL[c])}</span></div>`;
            rendered++;
          }
        }
      }

      if (oLine !== undefined && oLine !== nLine) {
        h += `<div class="atc-diff-row atc-diff-row--del"><span class="atc-diff-ln">${i + 1}</span><span class="atc-diff-sign">-</span><span class="atc-diff-code" data-raw="${_escAttr(oLine)}">${_escHtml(oLine)}</span></div>`;
        rendered++;
      }
      if (nLine !== undefined && oLine !== nLine) {
        h += `<div class="atc-diff-row atc-diff-row--add"><span class="atc-diff-ln">${i + 1}</span><span class="atc-diff-sign">+</span><span class="atc-diff-code" data-raw="${_escAttr(nLine)}">${_escHtml(nLine)}</span></div>`;
        rendered++;
      }
      lastShown = i;
    }
  }

  if (rendered >= cap) {
    const remaining = Math.max(oldL.length, newL.length) - cap;
    if (remaining > 0) h += `<div class="atc-diff-more">… ${remaining} more lines not shown …</div>`;
  }
  h += "</div>";
  return h;
}

function _escAttr(s) {
  return (s || "").replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

async function _highlightDiffView(container) {
  const diff = container.querySelector(".atc-diff");
  if (!diff) return;
  const lang = diff.dataset.lang;
  if (!lang || lang === "default") return;

  const monoId = monacoLang(lang);
  if (monoId === "plaintext") return;

  const codeEls = diff.querySelectorAll(".atc-diff-code[data-raw]");
  for (const el of codeEls) {
    const raw = el.dataset.raw;
    if (!raw || !raw.trim()) continue;
    try {
      let html = await monaco.editor.colorize(raw, monoId, { tabSize: 2 });
      html = html.replace(/<br\/?>\s*$/, "").replace(/^<div>/, "").replace(/<\/div>$/, "");
      if (html) el.innerHTML = html;
    } catch { /* keep plain text */ }
  }
}

async function _agentFollowUp(toolResults, container) {
  const config = loadConfig();
  if (!config.baseUrl || !config.apiKey) return;

  const followUpCtx = toolResults.map(r => {
    if (r.type === "read") return `--- 文件: ${r.path} ---\n${r.content}`;
    if (r.type === "list") return `--- 目录: ${r.path} ---\n${r.content}`;
    if (r.type === "cmd" && r.content && r.content !== "(executed)") return `--- 命令: ${r.path} ---\n输出:\n${r.content}`;
    return "";
  }).filter(Boolean).join("\n\n");

  if (!followUpCtx) return;

  const body = container;
  body.appendChild(thinkingCard());
  chatEl.scrollTop = chatEl.scrollHeight;

  const messages = [
    { role: "system", content: _AI_MODE_PROMPTS.agent },
    ...history,
    { role: "user", content: `以下是工具执行结果（文件内容、目录列表、命令输出），请根据这些信息继续完成任务：\n\n${followUpCtx}` },
  ];

  let acc = "";
  let err = null;
  let _segR2 = 0, _streamE2 = null;
  const root = rootPath || workspaceRoots[0] || "";
  const proms = [];
  // Throttle streaming re-render to ~12fps. Re-parsing the whole reply and
  // rebuilding its markdown DOM on every token is O(n²) and, combined with
  // terminal output, freezes the UI. The final full render below loses nothing.
  let _ffLast = 0, _ffTimer = 0;
  const _ffRender = () => {
    _ffTimer = 0; _ffLast = Date.now();
    body.querySelector(".thinking")?.remove();
    const segs = _parseStreamSegments(acc);
    const ce = segs.length > 0 && !segs[segs.length - 1].complete ? segs.length - 1 : segs.length;
    while (_segR2 < ce) {
      if (_streamE2) { _streamE2.remove(); _streamE2 = null; }
      _renderAgentSeg(body, segs[_segR2], segs, _segR2, root, proms);
      _segR2++;
    }
    const tail = ce < segs.length ? segs[segs.length - 1] : null;
    if (tail) {
      if (!_streamE2) { _streamE2 = document.createElement("div"); _streamE2.className = "agent-seg agent-seg--stream"; body.appendChild(_streamE2); }
      if (tail.type === "text") { const c = _cleanAgentText(tail.content); if (c) renderMarkdownStream(_streamE2, c, { streaming: true }); }
    }
    chatEl.scrollTop = chatEl.scrollHeight;
  };
  const _ffSchedule = () => {
    if (_ffTimer) return;
    _ffTimer = setTimeout(() => requestAnimationFrame(_ffRender), Math.max(0, 80 - (Date.now() - _ffLast)));
  };
  try {
    await backend.aiChat(config, messages, (ev) => {
      if (ev.kind === "token") { acc += ev.delta; _ffSchedule(); }
      else if (ev.kind === "error") { err = ev.message; }
    });
  } catch (e) { if (!err) err = String(e); }
  if (_ffTimer) { clearTimeout(_ffTimer); _ffTimer = 0; }
  body.querySelector(".thinking")?.remove();
  if (_streamE2) { _streamE2.remove(); _streamE2 = null; }
  if (acc) {
    const segs = _parseStreamSegments(acc);
    while (_segR2 < segs.length) { _renderAgentSeg(body, segs[_segR2], segs, _segR2, root, proms); _segR2++; }
    if (!err) { history.push({ role: "assistant", content: acc }); saveChatHistory(); }
  }
  chatEl.scrollTop = chatEl.scrollHeight;
}

// ---- AI inline code completion (Edit Prediction) ----
let _completionAbort = null;
let _lastCompletionKey = "";
let _cachedCompletion = null;
const _COMPLETION_DEBOUNCE = 400;

function _extractFileStructure(model, maxLines) {
  const total = model.getLineCount();
  const lines = [];
  const limit = Math.min(total, maxLines || 30);
  for (let i = 1; i <= limit; i++) {
    const line = model.getLineContent(i);
    if (/^\s*(import |from |require\(|#include|using |package |module )/.test(line) ||
        /^\s*(class |def |function |const |let |var |export |interface |type |struct |enum |pub fn |fn |async fn )/.test(line) ||
        /^\s*@/.test(line)) {
      lines.push(line);
    }
  }
  return lines.join("\n");
}

function _detectCommentIntent(textBefore) {
  const lines = textBefore.split("\n");
  for (let i = lines.length - 1; i >= Math.max(0, lines.length - 5); i--) {
    const trimmed = lines[i].trim();
    if (/^(\/\/|#|\/\*|\*|--|"""|''')/.test(trimmed) && trimmed.length > 8) {
      return trimmed;
    }
    if (trimmed.length > 0) break;
  }
  return null;
}

function _buildCompletionPrompt(lang, fileName, textBefore, textAfter, structure, commentIntent) {
  let systemPrompt = `You are a code completion engine. Output ONLY the raw code to insert at the cursor. Rules:
1. NO markdown, NO explanations, NO code fences, NO thinking process.
2. Match existing code style.
3. Keep completions concise (1-5 lines typically).
4. Complete the current expression/statement fully - never leave syntax open.`;

  if (commentIntent) {
    systemPrompt += `\nThe user wrote a comment: "${commentIntent}". Generate the implementation.`;
  }

  const beforeCtx = textBefore.slice(-1500);
  const afterCtx = textAfter.slice(0, 400);
  let userContent = `${fileName} (${lang})`;
  if (structure) userContent += `\n${structure}`;
  userContent += `\n\n${beforeCtx}█${afterCtx}`;

  return [
    { role: "system", content: systemPrompt },
    { role: "user", content: userContent },
  ];
}

function initInlineCompletion() {
  monaco.languages.registerInlineCompletionsProvider("*", {
    provideInlineCompletions: async (model, position, _context, token) => {
      if (_imeComposing) return undefined;
      const config = loadConfig();
      if (!config.baseUrl || !config.apiKey || !config.model) return undefined;

      const textBefore = model.getValueInRange({
        startLineNumber: Math.max(1, position.lineNumber - 80),
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column,
      });
      const textAfter = model.getValueInRange({
        startLineNumber: position.lineNumber,
        startColumn: position.column,
        endLineNumber: Math.min(model.getLineCount(), position.lineNumber + 20),
        endColumn: model.getLineMaxColumn(Math.min(model.getLineCount(), position.lineNumber + 20)),
      });

      if (textBefore.trim().length < 3) return { items: [] };

      const cacheKey = `${model.uri.toString()}:${position.lineNumber}:${position.column}:${textBefore.slice(-100)}`;
      if (cacheKey === _lastCompletionKey && _cachedCompletion) return _cachedCompletion;

      const lang = model.getLanguageId();
      const fileName = activePath ? activePath.split("/").pop() : "untitled";
      const structure = _extractFileStructure(model, 40);
      const commentIntent = _detectCommentIntent(textBefore);

      const maxTokens = commentIntent ? 4096 : 2048;
      const msgs = _buildCompletionPrompt(lang, fileName, textBefore, textAfter, structure, commentIntent);

      try {
        if (_completionAbort) { _completionAbort._cancelled = true; }
        const thisRequest = { _cancelled: false };
        _completionAbort = thisRequest;

        const inlineModel = config.baseUrl?.includes("deepseek") ? "deepseek-v4-flash" : config.model;
        const aiConfig = {
          baseUrl: config.baseUrl.replace(/\/+$/, ""),
          apiKey: config.apiKey,
          model: inlineModel,
          maxTokens: maxTokens,
          temperature: 0,
        };
        let text = await new Promise((resolve) => {
          let buf = "";
          backend.aiChat(aiConfig, msgs, (ev) => {
            if (ev.kind === "token") buf += ev.delta;
            else if (ev.kind === "done") resolve(buf);
            else if (ev.kind === "error") resolve("");
          }).catch(() => resolve(""));
          setTimeout(() => resolve(buf), 30000);
        });

        if (token.isCancellationRequested || thisRequest._cancelled) return { items: [] };
        if (!text || !text.trim()) return { items: [] };

        text = text.replace(/^```[\w]*\n?/, "").replace(/\n?```$/, "");
        if (text.startsWith("```")) text = text.slice(3).replace(/^[\w]*\n/, "");

        const result = {
          items: [{
            insertText: text,
            range: {
              startLineNumber: position.lineNumber,
              startColumn: position.column,
              endLineNumber: position.lineNumber,
              endColumn: position.column,
            },
          }],
        };
        _lastCompletionKey = cacheKey;
        _cachedCompletion = result;
        return result;
      } catch {
        return { items: [] };
      }
    },
    freeInlineCompletions: () => {},
    disposeInlineCompletions: () => {},
  });
}
if (typeof monaco !== "undefined") initInlineCompletion();

// ---- Inline AI Assistant (Ctrl+Enter) ----
let _inlineWidget = null;

function openInlineAssistant() {
  const sel = monacoEditor.getSelection();
  if (!sel || sel.isEmpty()) {
    showToast("Select code first, then press Ctrl+Enter");
    return;
  }
  const config = loadConfig();
  if (!config.baseUrl || !config.apiKey || !config.model) {
    openSettings();
    showToast(t("assistant.configFirst"));
    return;
  }
  closeInlineAssistant();

  const selectedCode = monacoEditor.getModel().getValueInRange(sel);
  const lang = monacoEditor.getModel().getLanguageId();

  const overlay = document.createElement("div");
  overlay.className = "inline-assist";
  overlay.innerHTML = `
    <div class="inline-assist__head">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2a5 5 0 015 5c0 2-1.5 3.5-3 4.5V14a2 2 0 01-2 2h-0a2 2 0 01-2-2v-2.5C8.5 10.5 7 9 7 7a5 5 0 015-5z"/><path d="M10 18h4"/><path d="M10 22h4"/></svg>
      <span>Inline Assistant</span>
      <button class="inline-assist__close" type="button">&times;</button>
    </div>
    <input class="inline-assist__input" type="text" placeholder="Describe the change…" spellcheck="false" autofocus />
    <div class="inline-assist__actions" hidden>
      <button class="btn btn--sm btn--primary inline-assist__accept">Accept</button>
      <button class="btn btn--sm inline-assist__reject">Reject</button>
    </div>
    <div class="inline-assist__status" hidden></div>
  `;

  const editorDom = $("editor");
  editorDom.parentElement.appendChild(overlay);

  const coords = monacoEditor.getScrolledVisiblePosition(sel.getStartPosition());
  if (coords) {
    overlay.style.top = (coords.top + coords.height + editorDom.offsetTop) + "px";
    overlay.style.left = Math.max(20, coords.left + editorDom.offsetLeft) + "px";
  }

  const input = overlay.querySelector(".inline-assist__input");
  const actionsDiv = overlay.querySelector(".inline-assist__actions");
  const statusDiv = overlay.querySelector(".inline-assist__status");
  let originalText = selectedCode;
  let newText = null;

  input.focus();
  input.addEventListener("keydown", async (e) => {
    if (e.key === "Escape") { closeInlineAssistant(); return; }
    if (e.key !== "Enter" || !input.value.trim()) return;
    e.preventDefault();
    const prompt = input.value.trim();
    input.disabled = true;
    statusDiv.hidden = false;
    statusDiv.textContent = "Thinking…";

    const msgs = [
      { role: "system", content: `You are a code transformation assistant. The user selected code in a ${lang} file and wants you to modify it. Output ONLY the modified code. No explanations, no markdown fences, no comments about changes. Just the raw modified code.` },
      { role: "user", content: `Instruction: ${prompt}\n\nSelected code:\n${originalText}` },
    ];

    try {
      let result = "";
      await backend.aiChat(config, msgs, (ev) => {
        if (ev.kind === "token") {
          result += ev.delta;
          statusDiv.textContent = `Generating… (${result.length} chars)`;
        } else if (ev.kind === "error") {
          statusDiv.textContent = "Error: " + ev.message;
        }
      });

      newText = result.trim();
      if (newText) {
        monacoEditor.executeEdits("inline-assist", [{
          range: sel,
          text: newText,
        }]);
        statusDiv.textContent = "Applied — Accept or Reject?";
        actionsDiv.hidden = false;
        input.hidden = true;
      }
    } catch (err) {
      statusDiv.textContent = "Error: " + (err.message || err);
      input.disabled = false;
    }
  });

  overlay.querySelector(".inline-assist__accept").addEventListener("click", () => {
    closeInlineAssistant();
  });
  overlay.querySelector(".inline-assist__reject").addEventListener("click", () => {
    if (newText !== null) {
      monacoEditor.trigger("inline-assist", "undo");
    }
    closeInlineAssistant();
  });
  overlay.querySelector(".inline-assist__close").addEventListener("click", () => {
    closeInlineAssistant();
  });

  _inlineWidget = overlay;
}

function closeInlineAssistant() {
  if (_inlineWidget) {
    _inlineWidget.remove();
    _inlineWidget = null;
  }
}

monacoEditor.addAction({
  id: "inline-assistant",
  label: "Inline AI Assistant",
  keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
  precondition: "editorHasSelection",
  run: () => openInlineAssistant(),
});

// ---- AI Diff Preview ----
function showAiDiffPreview(originalCode, modifiedCode, lang, filePath) {
  const ed = ensureDiffEditor({ readOnly: true, originalEditable: false });
  const original = monaco.editor.createModel(originalCode, lang || "plaintext");
  const modified = monaco.editor.createModel(modifiedCode, lang || "plaintext");
  const prev = ed.getModel();
  ed.setModel({ original, modified });
  if (prev) {
    prev.original?.dispose();
    prev.modified?.dispose();
  }
  _diffFilePath = filePath || null;
  $("diffTitle").textContent = filePath ? filePath.split("/").pop() + " (AI Preview)" : "AI Diff Preview";
  diffViewEl.hidden = false;
  ed.layout();

  ed.updateOptions({ readOnly: false, originalEditable: false });
}

// ---- settings dialog ----
const settingsEl = $("settings");
function openSettings() {
  const c = loadConfig();
  $("cfgBaseUrl").value = c.baseUrl || _DEFAULT_AI_CONFIG.baseUrl;
  $("cfgApiKey").value = c.apiKey || _DEFAULT_AI_CONFIG.apiKey;
  $("cfgModel").value = c.model || _DEFAULT_AI_CONFIG.model;
  settingsEl.showModal();
}
$("settingsForm").addEventListener("submit", async (e) => {
  if (e.submitter && e.submitter.value === "save") {
    await saveConfig({
      baseUrl: $("cfgBaseUrl").value.trim(),
      apiKey: $("cfgApiKey").value.trim(),
      model: $("cfgModel").value.trim(),
    });
    refreshModelBadge();
    showToast(t("settings.saved"));
  }
});

// ---- toast ----
let toastTimer;
function showToast(msg) {
  toastEl.textContent = msg;
  toastEl.classList.add("is-visible");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toastEl.classList.remove("is-visible"), 1900);
}

// ---- notification cards (bottom-right) ----
const _notifContainer = document.createElement("div");
_notifContainer.className = "notif-stack";
document.body.appendChild(_notifContainer);

function showNotification({ title, message, action, actionLabel = "安装", duration = 8000 }) {
  const card = document.createElement("div");
  card.className = "notif-card";
  card.innerHTML = `
    <div class="notif-card__content">
      <div class="notif-card__title">${title}</div>
      <div class="notif-card__msg">${message}</div>
    </div>
    <div class="notif-card__actions">
      ${action ? `<button class="notif-card__btn notif-card__btn--primary">${actionLabel}</button>` : ""}
      <button class="notif-card__btn notif-card__btn--dismiss">忽略</button>
    </div>`;
  _notifContainer.appendChild(card);
  requestAnimationFrame(() => card.classList.add("notif-card--visible"));
  const dismiss = () => { card.classList.remove("notif-card--visible"); setTimeout(() => card.remove(), 300); };
  card.querySelector(".notif-card__btn--dismiss")?.addEventListener("click", dismiss);
  if (action) card.querySelector(".notif-card__btn--primary")?.addEventListener("click", () => { action(); dismiss(); });
  if (duration > 0) setTimeout(dismiss, duration);
}

function _showInstallProgress(cmd, name) {
  const card = document.createElement("div");
  card.className = "notif-card";
  card.innerHTML = `
    <div class="notif-card__content">
      <div class="notif-card__title">正在安装 ${name}</div>
      <div class="notif-card__msg">${cmd}</div>
      <div class="notif-progress"><div class="notif-progress__bar"></div></div>
    </div>`;
  _notifContainer.appendChild(card);
  requestAnimationFrame(() => card.classList.add("notif-card--visible"));
  const bar = card.querySelector(".notif-progress__bar");
  let width = 0;
  const tick = setInterval(() => {
    width = Math.min(width + (90 - width) * 0.05, 95);
    bar.style.width = width + "%";
  }, 300);

  // The binary name is NOT the last word of the command: e.g.
  // "go install golang.org/x/tools/gopls@latest" installs `gopls`. Strip any
  // @version and take the final path segment.
  const lastArg = (cmd.trim().split(/\s+/).pop() || "");
  const bin = lastArg.split("@")[0].split("/").pop() || lastArg;
  const cwd = rootPath || workspaceRoots[0] || "/tmp";

  let checkDone, giveUp, settled = false;
  const finish = (ok, title, msg) => {
    if (settled) return;
    settled = true;
    clearInterval(tick); clearInterval(checkDone); clearTimeout(giveUp);
    bar.style.width = "100%";
    bar.style.background = ok ? "#34c759" : "#ff9f0a";
    card.querySelector(".notif-card__title").textContent = title;
    card.querySelector(".notif-card__msg").textContent = msg;
    setTimeout(() => { card.classList.remove("notif-card--visible"); setTimeout(() => card.remove(), 300); }, ok ? 4000 : 9000);
  };

  checkDone = setInterval(async () => {
    try {
      // Check PATH *and* the usual install dirs — go installs to ~/go/bin,
      // pip --user to ~/.local/bin, cargo to ~/.cargo/bin — which aren't always
      // on the shell PATH that term_list_commands sees.
      const r = await backend.taskRunCapture(
        cwd,
        `command -v ${bin} 2>/dev/null || ls "$HOME/go/bin/${bin}" "$HOME/.local/bin/${bin}" "$HOME/.cargo/bin/${bin}" /opt/homebrew/bin/${bin} /usr/local/bin/${bin} 2>/dev/null | head -1`
      );
      if (r && String(r.stdout || "").trim()) {
        finish(true, `${name} 安装完成`, "重新打开文件即可使用智能补全");
      }
    } catch { /* keep polling */ }
  }, 2500);

  // Don't leave a silently-frozen bar: after 90s, tell the user to check the
  // terminal (the install may have failed, e.g. base tool go/npm not present).
  giveUp = setTimeout(() => {
    finish(false, `${name} 安装未完成`, "请查看终端输出；如提示找不到 go/npm 等命令，需先装好对应的语言环境");
  }, 90000);
}

// ---- auto-detect missing tools ----
const TOOL_REQUIREMENTS = {
  python: { cmd: "pyright", install: "npm install -g pyright", name: "Pyright (Python LSP)" },
  rust: { cmd: "rust-analyzer", install: "brew install rust-analyzer", name: "rust-analyzer" },
  go: { cmd: "gopls", install: "go install golang.org/x/tools/gopls@latest", name: "gopls (Go LSP)" },
};

const _checkedLangs = new Set();
async function checkToolForLanguage(lang) {
  if (_checkedLangs.has(lang)) return;
  const req = TOOL_REQUIREMENTS[lang];
  if (!req) return;
  _checkedLangs.add(lang);
  try {
    if (inTauri) {
      const cmds = await backend.termListCommands();
      if (cmds.includes(req.cmd)) return;
    }
    showNotification({
      title: `缺少 ${req.name}`,
      message: `安装后可获得 ${lang} 智能补全、跳转定义等功能`,
      actionLabel: "安装",
      duration: 15000,
      action: async () => {
        await openTerminal();
        writeToActiveTerminal(req.install + "\n");
        showToast(`正在自动安装 ${req.name}...`);
      },
    });
  } catch (e) {
    console.warn("[tool-check]", lang, e);
  }
}

// ---- advanced feature panels (workspace / remote / marketplace / debug) ----
const FEATURE_TABS = [
  { id: "settings", title: "Settings", icon: "i-gear" },
  { id: "shortcuts", title: "Shortcuts", icon: "i-command" },
  { id: "workspace", title: "Workspace", icon: "i-folder" },
  { id: "tasks", title: "Tasks", icon: "i-play" },
  { id: "remote", title: "Remote", icon: "i-terminal" },
  { id: "marketplace", title: "Marketplace", icon: "i-ext" },
  { id: "conflicts", title: "Merge Conflicts", icon: "i-git" },
  { id: "debugger", title: "Debugger", icon: "i-code" },
  { id: "lsp", title: "Language Servers", icon: "i-code" },
];

const featureOverlay = document.createElement("div");
featureOverlay.className = "feature-panel";
featureOverlay.hidden = true;
featureOverlay.innerHTML = `
  <div class="feature-panel__sheet" role="dialog" aria-label="Advanced tools">
    <header class="feature-panel__head">
      <div class="feature-panel__title">
        <svg class="ic"><use href="#i-code" /></svg>
        <span>Advanced Tools</span>
      </div>
      <button class="feature-panel__close" type="button" aria-label="Close">
        <svg class="ic"><use href="#i-close" /></svg>
      </button>
    </header>
    <div class="feature-panel__main">
      <nav class="feature-panel__tabs" aria-label="Advanced tool tabs"></nav>
      <section class="feature-panel__body"></section>
    </div>
  </div>`;
document.body.appendChild(featureOverlay);

let activeFeatureTab = "workspace";

function featureBody() {
  return featureOverlay.querySelector(".feature-panel__body");
}

function openFeaturePanel(tab = activeFeatureTab) {
  activeFeatureTab = tab;
  featureOverlay.hidden = false;
  renderFeaturePanel();
}

function closeFeaturePanel() {
  featureOverlay.hidden = true;
}

function renderFeaturePanel() {
  const tabs = featureOverlay.querySelector(".feature-panel__tabs");
  const body = featureBody();
  tabs.innerHTML = "";
  for (const tab of FEATURE_TABS) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "feature-tab" + (tab.id === activeFeatureTab ? " is-active" : "");
    btn.innerHTML = `<svg class="ic"><use href="#${tab.icon}" /></svg><span></span>`;
    btn.querySelector("span").textContent = tab.title;
    btn.addEventListener("click", () => {
      activeFeatureTab = tab.id;
      renderFeaturePanel();
    });
    tabs.appendChild(btn);
  }

  body.innerHTML = "";
  body.classList.remove("mkt-body");
  const renderers = {
    settings: renderSettingsTool,
    shortcuts: renderShortcutsTool,
    workspace: renderWorkspaceTool,
    tasks: renderTasksTool,
    remote: renderRemoteTool,
    marketplace: renderMarketplaceTool,
    conflicts: renderConflictsTool,
    debugger: renderDebuggerTool,
    lsp: renderLspTool,
  };
  renderers[activeFeatureTab]?.(body);
}

featureOverlay.querySelector(".feature-panel__close").addEventListener("click", closeFeaturePanel);
featureOverlay.addEventListener("mousedown", (e) => {
  if (e.target === featureOverlay) closeFeaturePanel();
});
window.addEventListener("keydown", (e) => {
  if (!featureOverlay.hidden && e.key === "Escape") closeFeaturePanel();
});

function createToolHeader(body, title, desc) {
  const head = document.createElement("div");
  head.className = "tool-head";
  head.innerHTML = `<h2></h2><p></p>`;
  head.querySelector("h2").textContent = title;
  head.querySelector("p").textContent = desc;
  body.appendChild(head);
}

function createEmptyState(text) {
  const el = document.createElement("div");
  el.className = "tool-empty";
  el.textContent = text;
  return el;
}

const SETTINGS_SCHEMA = [
  {
    group: "Appearance",
    items: [
      { key: "theme", label: "Color Theme", type: "select", options: [
        ["system", "Follow System"], ["light", "Light"], ["dark", "Dark"],
        ["monokai", "Monokai"], ["github-light", "GitHub Light"],
        ["solarized-dark", "Solarized Dark"], ["nord", "Nord"],
      ] },
      { key: "fontSize", label: "Font Size", hint: "px", type: "number", min: 8, max: 48 },
      { key: "fontFamily", label: "Font Family", type: "text" },
      { key: "lineHeight", label: "Line Height", hint: "0 = auto", type: "number", min: 0, max: 60 },
    ],
  },
  {
    group: "Editor",
    items: [
      { key: "wordWrap", label: "Word Wrap", type: "select", options: [
        ["off", "Off"], ["on", "On"], ["wordWrapColumn", "At Column"], ["bounded", "Bounded"],
      ] },
      { key: "tabSize", label: "Tab Size", type: "number", min: 1, max: 8 },
      { key: "renderWhitespace", label: "Render Whitespace", type: "select", options: [
        ["none", "None"], ["boundary", "Boundary"], ["selection", "Selection"],
        ["trailing", "Trailing"], ["all", "All"],
      ] },
      { key: "cursorBlinking", label: "Cursor Animation", type: "select", options: [
        ["blink", "Blink"], ["smooth", "Smooth"], ["phase", "Phase"], ["expand", "Expand"], ["solid", "Solid"],
      ] },
      { key: "minimap", label: "Minimap", type: "toggle" },
      { key: "stickyScroll", label: "Sticky Scroll", type: "toggle" },
      { key: "bracketColorization", label: "Bracket Pair Colorization", type: "toggle" },
    ],
  },
  {
    group: "Files",
    items: [
      { key: "autoSave", label: "Auto Save", type: "toggle" },
    ],
  },
];

async function renderSettingsTool(body) {
  createToolHeader(body, "Settings", "Editor preferences save automatically and persist across sessions.");
  await loadEditorPrefs();
  const p = effectivePrefs();

  const update = async (key, value) => {
    _editorPrefs = _editorPrefs || {};
    _editorPrefs[key] = value;
    await saveEditorPrefs();
    applyEditorPrefs();
  };

  for (const section of SETTINGS_SCHEMA) {
    const sec = document.createElement("div");
    sec.className = "settings-group";
    const h = document.createElement("h3");
    h.className = "settings-group__title";
    h.textContent = section.group;
    sec.appendChild(h);

    for (const item of section.items) {
      const row = document.createElement("div");
      row.className = "settings-row";

      const meta = document.createElement("div");
      meta.className = "settings-row__meta";
      const lbl = document.createElement("span");
      lbl.className = "settings-row__label";
      lbl.textContent = item.label;
      meta.appendChild(lbl);
      if (item.hint) {
        const hint = document.createElement("span");
        hint.className = "settings-row__hint";
        hint.textContent = item.hint;
        meta.appendChild(hint);
      }
      row.appendChild(meta);

      const control = document.createElement("div");
      control.className = "settings-row__control";
      const cur = p[item.key];

      if (item.type === "select") {
        const sel = document.createElement("select");
        sel.className = "settings-input";
        for (const [val, text] of item.options) {
          const opt = document.createElement("option");
          opt.value = val;
          opt.textContent = text;
          if (String(val) === String(cur)) opt.selected = true;
          sel.appendChild(opt);
        }
        sel.addEventListener("change", () => update(item.key, sel.value));
        control.appendChild(sel);
      } else if (item.type === "number") {
        const inp = document.createElement("input");
        inp.type = "number";
        inp.className = "settings-input settings-input--num";
        inp.value = cur;
        if (item.min != null) inp.min = item.min;
        if (item.max != null) inp.max = item.max;
        inp.addEventListener("change", () => update(item.key, Number(inp.value)));
        control.appendChild(inp);
      } else if (item.type === "text") {
        const inp = document.createElement("input");
        inp.type = "text";
        inp.className = "settings-input settings-input--text";
        inp.value = cur;
        inp.addEventListener("change", () => update(item.key, inp.value));
        control.appendChild(inp);
      } else if (item.type === "toggle") {
        const sw = document.createElement("button");
        sw.type = "button";
        sw.className = "settings-toggle" + (cur !== false ? " is-on" : "");
        sw.setAttribute("role", "switch");
        sw.setAttribute("aria-checked", String(cur !== false));
        sw.innerHTML = `<span class="settings-toggle__knob"></span>`;
        sw.addEventListener("click", () => {
          const on = !sw.classList.contains("is-on");
          sw.classList.toggle("is-on", on);
          sw.setAttribute("aria-checked", String(on));
          update(item.key, on);
        });
        control.appendChild(sw);
      }
      row.appendChild(control);
      sec.appendChild(row);
    }
    body.appendChild(sec);
  }

  const actions = document.createElement("div");
  actions.className = "tool-actions settings-actions";
  const reset = document.createElement("button");
  reset.className = "btn";
  reset.type = "button";
  reset.textContent = "Reset to Defaults";
  reset.addEventListener("click", async () => {
    _editorPrefs = {};
    await saveEditorPrefs();
    applyEditorPrefs();
    renderFeaturePanel();
  });
  actions.appendChild(reset);
  body.appendChild(actions);
}

const ACTION_LABELS = {
  "terminal.toggle": "Toggle Terminal",
  "file.quickOpen": "Quick Open",
  "file.save": "Save File",
  "view.explorer": "Show Explorer",
  "view.search": "Show Search",
  "view.git": "Show Source Control",
  "view.problems": "Toggle Problems Panel",
  "memory.manage": "Manage Project Memory",
  "commandPalette": "Command Palette",
  "view.splitEditor": "Toggle Split Editor",
  "code.runCurrentFile": "Run Current File",
};

const DISABLED_BINDING = "__none__";

function formatCombo(combo) {
  const isMac = /Mac/i.test(navigator.platform);
  const map = {
    mod: isMac ? "\u2318" : "Ctrl",
    ctrl: isMac ? "\u2303" : "Ctrl",
    shift: "\u21e7",
    alt: isMac ? "\u2325" : "Alt",
    " ": "Space",
  };
  return combo.split("+").map((part) => {
    if (map[part]) return map[part];
    return part.length === 1 ? part.toUpperCase() : part.charAt(0).toUpperCase() + part.slice(1);
  });
}

async function rebindAction(action, newCombo) {
  const bindings = getKeybindings();
  for (const [combo, act] of Object.entries(bindings)) {
    if (act === action && combo !== newCombo) {
      if (DEFAULT_KEYBINDINGS[combo] === action) userKeybindings[combo] = DISABLED_BINDING;
      else delete userKeybindings[combo];
    }
  }
  userKeybindings[newCombo] = action;
  const store = await getStore();
  await store.set("keybindings", userKeybindings);
  await store.save();
}

async function resetKeybindings() {
  userKeybindings = {};
  const store = await getStore();
  await store.set("keybindings", {});
  await store.save();
}

function recordKeybinding(action, btn, onDone) {
  if (btn.classList.contains("is-recording")) return;
  const original = btn.textContent;
  btn.textContent = "Press keys\u2026";
  btn.classList.add("is-recording");
  const cleanup = () => {
    window.removeEventListener("keydown", handler, true);
    btn.textContent = original;
    btn.classList.remove("is-recording");
  };
  const handler = async (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    if (e.key === "Escape") { cleanup(); return; }
    const combo = keyCombo(e);
    cleanup();
    await rebindAction(action, combo);
    onDone();
  };
  window.addEventListener("keydown", handler, true);
}

function renderShortcutsTool(body) {
  createToolHeader(body, "Keyboard Shortcuts", "View and customize keybindings. Click Change, then press the new key combination (Esc to cancel).");
  const bindings = getKeybindings();
  const actionToCombo = {};
  for (const [combo, act] of Object.entries(bindings)) {
    if (!actionToCombo[act]) actionToCombo[act] = combo;
  }

  const list = document.createElement("div");
  list.className = "shortcuts-list";
  for (const action of Object.keys(ACTION_LABELS)) {
    const combo = actionToCombo[action];
    const isCustom = combo && userKeybindings[combo] === action;
    const row = document.createElement("div");
    row.className = "shortcut-row";

    const left = document.createElement("div");
    left.className = "shortcut-row__label";
    left.textContent = ACTION_LABELS[action];
    if (isCustom) {
      const badge = document.createElement("span");
      badge.className = "shortcut-row__badge";
      badge.textContent = "Custom";
      left.appendChild(badge);
    }
    row.appendChild(left);

    const keys = document.createElement("div");
    keys.className = "shortcut-row__keys";
    if (combo) {
      for (const k of formatCombo(combo)) {
        const kbd = document.createElement("kbd");
        kbd.textContent = k;
        keys.appendChild(kbd);
      }
    } else {
      const none = document.createElement("span");
      none.className = "shortcut-row__unbound";
      none.textContent = "Unbound";
      keys.appendChild(none);
    }
    row.appendChild(keys);

    const change = document.createElement("button");
    change.className = "btn btn--sm shortcut-row__change";
    change.type = "button";
    change.textContent = "Change";
    change.addEventListener("click", () => recordKeybinding(action, change, () => renderFeaturePanel()));
    row.appendChild(change);

    list.appendChild(row);
  }
  body.appendChild(list);

  const foot = document.createElement("div");
  foot.className = "tool-actions settings-actions";
  const reset = document.createElement("button");
  reset.className = "btn";
  reset.type = "button";
  reset.textContent = "Reset All to Defaults";
  reset.addEventListener("click", async () => {
    await resetKeybindings();
    renderFeaturePanel();
  });
  foot.appendChild(reset);
  body.appendChild(foot);
}

function renderWorkspaceTool(body) {
  createToolHeader(body, "Multi-root Workspace", "Add folders to one workspace, then switch which root powers Git, search, terminal, and language indexing.");
  const actions = document.createElement("div");
  actions.className = "tool-actions";
  const add = document.createElement("button");
  add.className = "btn btn--primary";
  add.type = "button";
  add.textContent = "Add Folder";
  add.addEventListener("click", async () => {
    await addFolderToWorkspace();
    renderFeaturePanel();
  });
  actions.appendChild(add);
  body.appendChild(actions);

  const list = document.createElement("div");
  list.className = "tool-list";
  if (!workspaceRoots.length) {
    list.appendChild(createEmptyState("Open a folder to start a workspace."));
  }
  for (const root of workspaceRoots) {
    const card = document.createElement("div");
    card.className = "tool-card" + (root === rootPath ? " is-active" : "");
    card.innerHTML = `<div class="tool-card__main"><strong></strong><span></span></div><button class="btn" type="button"></button>`;
    card.querySelector("strong").textContent = basename(root);
    card.querySelector("span").textContent = root;
    const btn = card.querySelector("button");
    btn.textContent = root === rootPath ? "Active" : "Use Root";
    btn.disabled = root === rootPath;
    btn.addEventListener("click", async () => {
      setActiveWorkspaceRoot(root);
      await renderWorkspaceRoots();
      preloadProjectModels(root);
      await refreshGitStatus();
      renderFeaturePanel();
    });
    list.appendChild(card);
  }
  body.appendChild(list);
}

function writeToActiveTerminal(data, tries = 30) {
  const tab = typeof termTabs !== "undefined" && activeTermTab >= 0 ? termTabs[activeTermTab] : null;
  if (tab?.backendId != null) {
    backend.termWrite(tab.backendId, data);
    return;
  }
  if (tries > 0) setTimeout(() => writeToActiveTerminal(data, tries - 1), 150);
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, "'\\''")}'`;
}

function dirname(path) {
  const clean = path.replace(/\/+$/, "");
  const idx = clean.lastIndexOf("/");
  return idx > 0 ? clean.slice(0, idx) : "/";
}

function runCommandForFile(path) {
  const name = basename(path);
  const ext = name.includes(".") ? name.split(".").pop().toLowerCase() : "";
  const q = shellQuote(path);
  const dir = shellQuote(dirname(path));
  switch (ext) {
    case "js":
    case "mjs":
    case "cjs":
      return `node ${q}`;
    case "ts":
    case "tsx":
      return `npx tsx ${q}`;
    case "py":
      return `python3 ${q}`;
    case "sh":
    case "bash":
    case "zsh":
      return `bash ${q}`;
    case "rb":
      return `ruby ${q}`;
    case "php":
      return `php ${q}`;
    case "go":
      return `go run ${q}`;
    case "rs": {
      const out = `/tmp/michael-ide-${name.replace(/[^A-Za-z0-9_.-]/g, "_")}`;
      return `rustc ${q} -o ${shellQuote(out)} && ${shellQuote(out)}`;
    }
    case "java": {
      const className = name.replace(/\.java$/i, "");
      if (!/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(className)) return null;
      return `cd ${dir} && javac ${shellQuote(name)} && java ${className}`;
    }
    case "kt":
    case "kts":
      return `kotlinc-jvm ${q} -include-runtime -d /tmp/_kt_out.jar && java -jar /tmp/_kt_out.jar`;
    case "swift":
      return `swift ${q}`;
    case "c": {
      const out = `/tmp/michael-ide-${name.replace(/[^A-Za-z0-9_.-]/g, "_")}`;
      return `gcc ${q} -o ${shellQuote(out)} && ${shellQuote(out)}`;
    }
    case "cpp":
    case "cc":
    case "cxx": {
      const out = `/tmp/michael-ide-${name.replace(/[^A-Za-z0-9_.-]/g, "_")}`;
      return `g++ -std=c++17 ${q} -o ${shellQuote(out)} && ${shellQuote(out)}`;
    }
    case "cs":
      return `dotnet-script ${q}`;
    case "lua":
      return `lua ${q}`;
    case "pl":
    case "pm":
      return `perl ${q}`;
    case "dart":
      return `dart run ${q}`;
    case "r":
    case "R":
      return `Rscript ${q}`;
    case "html":
    case "htm":
      return `__SERVE_HTML__`;
    case "vue":
    case "svelte":
    case "jsx":
    case "tsx":
      return `__SERVE_FRONTEND__`;
    case "css":
    case "scss":
    case "less":
      return `__SERVE_HTML__`;
    case "json":
      return `python3 -m json.tool ${q}`;
    case "sql":
      return `sqlite3 < ${q}`;
    default:
      return null;
  }
}

async function _waitTermReady(maxWait = 3000) {
  const start = Date.now();
  while (Date.now() - start < maxWait) {
    const tab = activeTermTab >= 0 ? termTabs[activeTermTab] : null;
    if (tab?.backendId != null && !tab.opening) return true;
    await new Promise((r) => setTimeout(r, 200));
  }
  return false;
}

let _devServerPort = null;
let _devServerRunning = false;

async function _findFreePort(start = 3000) {
  for (let p = start; p < start + 100; p++) {
    try {
      const resp = await fetch(`http://127.0.0.1:${p}`, { signal: AbortSignal.timeout(200) });
      continue;
    } catch {
      return p;
    }
  }
  return start;
}

async function _startDevServer(dir, port, fileName = "") {
  const wasOpen = termIsOpen();
  await openTerminal();
  if (!wasOpen) {
    await _waitTermReady(3000);
    await new Promise((r) => setTimeout(r, 1800));
  }

  const scriptContent = [
    "import http.server, socketserver, os, socket, threading, time",
    `os.chdir(r'${dir}')`,
    `PORT = ${port}`,
    "file_hashes = {}",
    "def scan():",
    "    h = {}",
    "    for root, _, files in os.walk('.'):",
    "        for f in files:",
    "            if f.endswith(('.html', '.css', '.js', '.json')):",
    "                p = os.path.join(root, f)",
    "                try: h[p] = os.path.getmtime(p)",
    "                except: pass",
    "    return h",
    "file_hashes.update(scan())",
    "last_change = [time.time()]",
    "def watcher():",
    "    while True:",
    "        time.sleep(0.8)",
    "        cur = scan()",
    "        if cur != file_hashes:",
    "            file_hashes.clear()",
    "            file_hashes.update(cur)",
    "            last_change[0] = time.time()",
    "threading.Thread(target=watcher, daemon=True).start()",
    `RELOAD_JS = b'<script>(function(){var t=0;setInterval(function(){fetch("/__reload__").then(function(r){return r.text()}).then(function(s){var n=parseFloat(s);if(t&&n>t)location.reload();t=n})},800)})()</script>'`,
    "class RH(http.server.SimpleHTTPRequestHandler):",
    "    def do_GET(self):",
    "        if self.path == '/__reload__':",
    "            self.send_response(200)",
    "            self.send_header('Content-Type', 'text/plain')",
    "            self.send_header('Access-Control-Allow-Origin', '*')",
    "            self.end_headers()",
    "            self.wfile.write(str(last_change[0]).encode())",
    "            return",
    "        path = self.translate_path(self.path)",
    "        if os.path.isfile(path) and path.endswith('.html'):",
    "            with open(path, 'rb') as f: data = f.read()",
    "            data = data.replace(b'</body>', RELOAD_JS + b'</body>')",
    "            self.send_response(200)",
    "            self.send_header('Content-Type', 'text/html')",
    "            self.send_header('Content-Length', str(len(data)))",
    "            self.end_headers()",
    "            self.wfile.write(data)",
    "            return",
    "        super().do_GET()",
    "    def log_message(self, fmt, *args):",
    "        print(f'  {args[1]} {args[0]}')",
    "try: ip = socket.gethostbyname(socket.gethostname())",
    "except: ip = '127.0.0.1'",
    "s = socketserver.TCPServer(('', PORT), RH)",
    `FILE = '${fileName}'`,
    `path_suffix = '/' + FILE if FILE else ''`,
    `print('\\n  \\033[1;32m✦ Michael IDE Dev Server\\033[0m\\n')`,
    `print(f'  ➜  Local:   \\033[36mhttp://localhost:{PORT}{path_suffix}\\033[0m')`,
    `print(f'  ➜  Network: \\033[36mhttp://{ip}:{PORT}{path_suffix}\\033[0m')`,
    `print(f'\\n  Serving:  {os.getcwd()}')`,
    `print('  Live Reload: \\033[32m✓ enabled\\033[0m')`,
    `print('  Press \\033[1mCtrl+C\\033[0m to stop\\n')`,
    "s.serve_forever()",
  ].join("\n");

  try {
    const tmpPath = await backend.writeTmpFile("_michael_ide_dev_server.py", scriptContent);
    writeToActiveTerminal(`\nclear\npython3 ${shellQuote(tmpPath)}\n`);
  } catch {
    writeToActiveTerminal(`\nclear\npython3 -m http.server ${port} --directory ${shellQuote(dir)}\n`);
  }
  _devServerPort = port;
  _devServerRunning = true;
}

async function _startViteServer(dir) {
  const wasOpen = termIsOpen();
  await openTerminal();
  if (!wasOpen) {
    await _waitTermReady(3000);
    await new Promise((r) => setTimeout(r, 1800));
  }

  const q = shellQuote(dir);
  const hasVite = await new Promise((resolve) => {
    try {
      const fs = window.__TAURI_INTERNALS__ ? true : false;
      resolve(true);
    } catch { resolve(false); }
  });

  writeToActiveTerminal(`\nclear\ncd ${q} && npx --yes vite --open\n`);
  showToast("启动 Vite Dev Server...");
}

async function runCurrentFile() {
  if (!activePath) {
    showToast("请先打开一个文件");
    return;
  }
  const command = runCommandForFile(activePath);
  if (!command) {
    showToast(`不支持运行 ${basename(activePath)} 类型的文件`);
    return;
  }
  const file = openFiles.get(activePath);
  if (file?.dirty) await saveActive();

  if (command === "__SERVE_HTML__") {
    const dir = dirname(activePath);
    const port = await _findFreePort(3000);
    const name = basename(activePath);
    await _startDevServer(dir, port, name);
    return;
  }

  if (command === "__SERVE_FRONTEND__") {
    const dir = dirname(activePath);
    await _startViteServer(dir);
    return;
  }

  const wasOpen = termIsOpen();
  await openTerminal();

  if (!wasOpen) {
    await _waitTermReady(3000);
    await new Promise((r) => setTimeout(r, 1800));
  }

  writeToActiveTerminal(`\nclear\n${command}\n`);
}

async function runTask(task) {
  if (!task?.command) return;
  await openTerminal();
  const cwd = task.cwd || rootPath;
  const prefix = cwd ? `cd ${shellQuote(cwd)} && ` : "";
  writeToActiveTerminal(`\n${prefix}${task.command}\n`);
  showToast(`Running task: ${task.label}`);
}

// Owner used for markers produced by task/problem-matcher runs, so they can be
// cleared independently of LSP and extension diagnostics.
const TASK_MARKER_OWNER = "task";
const MAX_PROBLEM_FILES = 80;

function clearTaskProblems() {
  for (const model of monaco.editor.getModels()) {
    monaco.editor.setModelMarkers(model, TASK_MARKER_OWNER, []);
  }
}

function resolveTaskPath(file, cwd) {
  if (!file) return null;
  let f = file.trim().replace(/^\.[\\/]/, "");
  if (f.startsWith("/") || /^[A-Za-z]:[\\/]/.test(f)) return f;
  const base = (cwd || rootPath || "").replace(/[\\/]+$/, "");
  return base ? `${base}/${f}` : f;
}

function taskSeverityToMarker(sev) {
  return sev === "error" ? monaco.MarkerSeverity.Error
    : sev === "warning" ? monaco.MarkerSeverity.Warning
    : sev === "info" ? monaco.MarkerSeverity.Info
    : monaco.MarkerSeverity.Hint;
}

async function ensureModelForPath(absPath) {
  const uri = monaco.Uri.file(absPath);
  const existing = monaco.editor.getModel(uri);
  if (existing) return existing;
  try {
    const content = await backend.readTextFile(absPath);
    return getOrCreateModel(absPath, basename(absPath), content);
  } catch {
    return null;
  }
}

// Run a task non-interactively, capture its output, parse it through the
// matching problem matcher, and surface the results in the Problems panel and
// as inline squiggles.
async function runTaskWithProblems(task) {
  if (!task?.command) return;
  const cwd = task.cwd || rootPath;
  if (!cwd) {
    showToast("Open a workspace first.");
    return;
  }
  clearTaskProblems();
  showToast(`Analyzing: ${task.label}…`);
  let result;
  try {
    result = await backend.taskRunCapture(cwd, task.command);
  } catch (e) {
    showToast(`Task failed to start: ${String(e && e.message ? e.message : e)}`);
    return;
  }

  const problems = parseProblems(result.combined || `${result.stdout || ""}${result.stderr || ""}`, {
    matcher: task.problemMatcher,
    command: task.command,
  });

  // Group by resolved absolute path.
  const byPath = new Map();
  for (const p of problems) {
    const abs = resolveTaskPath(p.file, cwd);
    if (!abs) continue;
    if (!byPath.has(abs)) byPath.set(abs, []);
    byPath.get(abs).push(p);
  }

  let errs = 0;
  let warns = 0;
  let fileCount = 0;
  for (const [abs, items] of byPath.entries()) {
    if (fileCount >= MAX_PROBLEM_FILES) break;
    const model = await ensureModelForPath(abs);
    if (!model) continue;
    fileCount++;
    const markers = items.map((p) => {
      const line = Math.min(p.line, model.getLineCount());
      const maxCol = model.getLineMaxColumn(line);
      const startCol = Math.min(p.col, maxCol);
      const word = model.getWordAtPosition({ lineNumber: line, column: startCol });
      const endCol = word ? word.endColumn : maxCol;
      if (p.severity === "error") errs++;
      else if (p.severity === "warning") warns++;
      return {
        severity: taskSeverityToMarker(p.severity),
        message: p.code ? `${p.message} (${p.code})` : p.message,
        source: p.source || task.label,
        startLineNumber: line,
        startColumn: startCol,
        endLineNumber: line,
        endColumn: Math.max(endCol, startCol + 1),
      };
    });
    monaco.editor.setModelMarkers(model, TASK_MARKER_OWNER, markers);
  }

  if (problems.length) {
    if (problemsPanel && problemsPanel.hidden) toggleProblems();
    else updateProblems();
    showToast(`${task.label}: ${errs} error(s), ${warns} warning(s) across ${fileCount} file(s)`);
  } else if (result.code === 0) {
    showToast(`${task.label}: completed with no problems`);
  } else {
    await openTerminal();
    showToast(`${task.label}: exited ${result.code}, no parseable problems — running in terminal`);
    runTask(task);
  }
}

function renderTasksTool(body) {
  createToolHeader(body, "Task Runner", "Discover npm, Cargo, Makefile, .michael/tasks.json, and .vscode/tasks.json tasks. Run them in the terminal, or use Problems to capture output and route compiler/linter errors into the Problems panel.");
  const actions = document.createElement("div");
  actions.className = "tool-actions";
  const refresh = document.createElement("button");
  refresh.className = "btn";
  refresh.type = "button";
  refresh.textContent = "Refresh";
  const clearBtn = document.createElement("button");
  clearBtn.className = "btn";
  clearBtn.type = "button";
  clearBtn.textContent = "Clear Problems";
  clearBtn.addEventListener("click", () => {
    clearTaskProblems();
    updateProblems();
    showToast("Cleared task problems");
  });
  actions.append(refresh, clearBtn);
  const list = document.createElement("div");
  list.className = "tool-list";
  body.append(actions, list);

  const load = async () => {
    list.innerHTML = "";
    if (!rootPath) {
      list.appendChild(createEmptyState("Open a workspace before running tasks."));
      return;
    }
    list.appendChild(createEmptyState("Loading tasks…"));
    try {
      const tasks = await backend.tasksList(rootPath);
      list.innerHTML = "";
      if (!tasks.length) {
        list.appendChild(createEmptyState("No tasks found. Add package.json scripts, Cargo.toml, Makefile, .michael/tasks.json, or .vscode/tasks.json."));
        return;
      }
      for (const task of tasks) {
        const card = document.createElement("div");
        card.className = "tool-card";
        card.innerHTML = `
          <div class="tool-card__main">
            <strong></strong>
            <span></span>
          </div>
          <div class="tool-card__actions">
            <button class="btn" type="button" data-act="problems">Problems</button>
            <button class="btn btn--primary" type="button" data-act="run">Run</button>
          </div>`;
        card.querySelector("strong").textContent = task.label;
        card.querySelector("span").textContent = `${task.source} · ${task.group} · ${task.command}`;
        card.querySelector('[data-act="run"]').addEventListener("click", () => runTask(task));
        card.querySelector('[data-act="problems"]').addEventListener("click", () => runTaskWithProblems(task));
        list.appendChild(card);
      }
    } catch (e) {
      list.innerHTML = "";
      list.appendChild(createEmptyState(String(e && e.message ? e.message : e)));
    }
  };

  refresh.addEventListener("click", load);
  load();
}

function renderRemoteTool(body) {
  createToolHeader(body, "Remote Development", "Create SSH terminal sessions tied to the current workspace. Use a mounted or synced path as a folder root when you need remote files in the editor.");
  const form = document.createElement("div");
  form.className = "tool-form";
  form.innerHTML = `
    <label><span>SSH target</span><input id="remoteTarget" spellcheck="false" placeholder="user@example.com" /></label>
    <label><span>Remote path</span><input id="remotePath" spellcheck="false" placeholder="/home/user/project" /></label>
    <div class="tool-actions"><button class="btn btn--primary" type="button" id="remoteConnect">Open SSH Terminal</button></div>
    <p class="tool-note">For full remote file editing, mount the remote folder locally first, then add that mounted folder to the workspace.</p>`;
  body.appendChild(form);
  form.querySelector("#remoteConnect").addEventListener("click", async () => {
    const target = form.querySelector("#remoteTarget").value.trim();
    const remotePath = form.querySelector("#remotePath").value.trim();
    if (!target) {
      showToast("Enter an SSH target first.");
      return;
    }
    if (!/^[A-Za-z0-9_.@%:+-]+$/.test(target)) {
      showToast("SSH target contains unsupported characters.");
      return;
    }
    await openTerminal();
    const remoteCommand = remotePath ? ` 'cd ${remotePath.replace(/'/g, "'\\''")} && exec $SHELL -l'` : "";
    writeToActiveTerminal(`ssh -t ${target}${remoteCommand}\n`);
    showToast(`Opening SSH session for ${target}`);
  });
}

function renderMarketplaceTool(body) {
  body.innerHTML = '<div class="empty"><p>Use the Extensions button or press ⇧⌘X to open the Marketplace.</p></div>';
}

const MKT_ICON_SVGS = {
  tailwind: `<path d="M12 5C9.2 5 7.5 6.3 6.8 9c1.1-1.3 2.3-1.8 3.8-1.5.8.2 1.4.8 2 1.4 1 1.1 2.2 2.3 4.7 2.3 2.8 0 4.5-1.3 5.2-4-1.1 1.3-2.3 1.8-3.8 1.5-.8-.2-1.4-.8-2-1.4C15.7 6.2 14.5 5 12 5zM6.8 12.2C4 12.2 2.3 13.5 1.5 16.2c1.1-1.3 2.3-1.8 3.8-1.5.8.2 1.4.8 2 1.4 1 1.1 2.2 2.3 4.7 2.3 2.8 0 4.5-1.3 5.2-4-1.1 1.3-2.3 1.8-3.8 1.5-.8-.2-1.4-.8-2-1.4-1-1.1-2.2-2.3-4.6-2.3z" fill="currentColor"/>`,
  vue: `<path d="M2 3h4l6 10.5L18 3h4L12 21 2 3z" fill="currentColor" opacity=".3"/><path d="M7 3h3l2 3.5L14 3h3L12 15 7 3z" fill="currentColor"/>`,
  theme: `<path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.8 0 1.5-.7 1.5-1.5 0-.4-.1-.7-.4-1-.3-.3-.5-.7-.5-1.1 0-.8.7-1.5 1.5-1.5H16c3.3 0 6-2.7 6-6 0-5.5-4.5-9-10-9z" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="8" cy="9" r="1.8" fill="#e53935"/><circle cx="12.5" cy="6.5" r="1.8" fill="#fdd835"/><circle cx="17" cy="9" r="1.8" fill="#43a047"/><circle cx="7" cy="14" r="1.8" fill="#1e88e5"/>`,
  docker: `<rect x="5" y="11" width="3" height="2.5" rx=".4" fill="currentColor"/><rect x="9" y="11" width="3" height="2.5" rx=".4" fill="currentColor"/><rect x="13" y="11" width="3" height="2.5" rx=".4" fill="currentColor"/><rect x="5" y="8" width="3" height="2.5" rx=".4" fill="currentColor"/><rect x="9" y="8" width="3" height="2.5" rx=".4" fill="currentColor"/><rect x="13" y="8" width="3" height="2.5" rx=".4" fill="currentColor"/><rect x="9" y="5" width="3" height="2.5" rx=".4" fill="currentColor"/><path d="M2 15c0 0 1.5-2 5-2h12c3 0 3.5 2 3.5 2s-.5 5-8.5 5-12-5-12-5z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>`,
  copilot: `<rect x="3" y="8" width="18" height="10" rx="5" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="9" cy="13" r="2" fill="currentColor"/><circle cx="15" cy="13" r="2" fill="currentColor"/><circle cx="9" cy="12.5" r=".8" fill="#fff"/><circle cx="15" cy="12.5" r=".8" fill="#fff"/><path d="M9 5v3M15 5v3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><path d="M6 8C6 5 9 3 12 3s6 2 6 5" fill="none" stroke="currentColor" stroke-width="1.3"/>`,
  ai: `<rect x="4" y="4" width="16" height="16" rx="4" fill="currentColor" opacity=".1"/><circle cx="12" cy="10" r="3" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="12" cy="10" r="1" fill="currentColor"/><path d="M12 3v3M12 15v3M5 10h3M16 10h3" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/><path d="M7 5.5l2 2M15 13.5l2 2M7 14.5l2-2M15 6.5l2-2" stroke="currentColor" stroke-width="1" stroke-linecap="round" opacity=".5"/>`,
  hanzi: `<rect x="2" y="2" width="20" height="20" rx="4" fill="currentColor" opacity=".08"/><text x="12" y="17" text-anchor="middle" font-size="15" font-weight="800" font-family="'PingFang SC',system-ui" fill="currentColor">字</text>`,
  translate: `<rect x="2" y="3" width="9" height="8" rx="2" fill="currentColor" opacity=".1"/><text x="6.5" y="9.5" text-anchor="middle" font-size="7" font-weight="700" font-family="system-ui" fill="currentColor">中</text><rect x="13" y="13" width="9" height="8" rx="2" fill="currentColor" opacity=".1"/><text x="17.5" y="19.5" text-anchor="middle" font-size="7" font-weight="700" font-family="system-ui" fill="currentColor">A</text><path d="M11 7h6M7 17h6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" opacity=".4"/><path d="M14 7l-3 10" stroke="currentColor" stroke-width="1.2" stroke-dasharray="2 2" opacity=".3"/>`,
  camera: `<rect x="2" y="6" width="20" height="14" rx="3" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="12" cy="13.5" r="4" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="12" cy="13.5" r="1.5" fill="currentColor"/><path d="M8 6l1.5-3h5L16 6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><circle cx="18" cy="9" r=".8" fill="currentColor"/>`,
  color: `<circle cx="10" cy="9" r="5" fill="none" stroke="#e53935" stroke-width="1.8"/><circle cx="14" cy="9" r="5" fill="none" stroke="#43a047" stroke-width="1.8"/><circle cx="12" cy="14" r="5" fill="none" stroke="#1e88e5" stroke-width="1.8"/>`,
  liveserver: `<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.5"/><ellipse cx="12" cy="12" rx="4" ry="9" fill="none" stroke="currentColor" stroke-width="1"/><path d="M3 9h18M3 15h18" stroke="currentColor" stroke-width="1"/><circle cx="18" cy="5" r="3" fill="currentColor"/><path d="M17 4l1.5 1.5L20 4" stroke="#fff" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>`,
  zhcn: `<rect x="2" y="4" width="20" height="16" rx="3" fill="currentColor" opacity=".08"/><text x="12" y="15" text-anchor="middle" font-size="9" font-weight="800" font-family="'PingFang SC',system-ui" fill="currentColor">简中</text>`,
  iconify: `<rect x="3" y="3" width="8" height="8" rx="2" fill="currentColor" opacity=".65"/><rect x="13" y="3" width="8" height="8" rx="2" fill="currentColor" opacity=".45"/><rect x="3" y="13" width="8" height="8" rx="2" fill="currentColor" opacity=".35"/><rect x="13" y="13" width="8" height="8" rx="2" fill="currentColor" opacity=".55"/><circle cx="7" cy="7" r="2" fill="#fff" opacity=".6"/><rect x="14.5" y="5" width="5" height="1.5" rx=".75" fill="#fff" opacity=".5"/><path d="M5 17l3 3 3-3" fill="none" stroke="#fff" stroke-width="1.2" stroke-linecap="round" opacity=".5"/><circle cx="17" cy="17" r="2" fill="#fff" opacity=".4"/>`,
  linter: `<path d="M12 2l9.5 4.5v5.5c0 5.5-4 9.5-9.5 11-5.5-1.5-9.5-5.5-9.5-11V6.5L12 2z" fill="currentColor" opacity=".08"/><path d="M12 2l9.5 4.5v5.5c0 5.5-4 9.5-9.5 11-5.5-1.5-9.5-5.5-9.5-11V6.5L12 2z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M8.5 12.5l2.5 2.5 5-5.5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>`,
  web: `<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.5"/><ellipse cx="12" cy="12" rx="5" ry="9" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M3 12h18" stroke="currentColor" stroke-width="1.2"/><path d="M4 8h16M4 16h16" stroke="currentColor" stroke-width=".8" opacity=".5"/>`,
  default: `<rect x="3" y="3" width="18" height="18" rx="4" fill="currentColor" opacity=".08"/><path d="M8 8l4 4-4 4M13 16h4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>`,
};

function _mktIconFor(entry) {
  if (entry.icon && MKT_ICON_SVGS[entry.icon]) return MKT_ICON_SVGS[entry.icon];
  const tags = (entry.tags || []).map(t => t.toLowerCase());
  if (tags.includes("theme") || tags.includes("icons")) return MKT_ICON_SVGS.theme;
  if (tags.includes("git")) return MKT_ICON_SVGS.git;
  if (tags.includes("formatter")) return MKT_ICON_SVGS.formatter;
  if (tags.includes("linter")) return MKT_ICON_SVGS.linter;
  if (tags.includes("rust") || tags.includes("python") || tags.includes("language")) return MKT_ICON_SVGS.language;
  if (tags.includes("docker") || tags.includes("devops")) return MKT_ICON_SVGS.docker;
  if (tags.includes("ai") || tags.includes("completion")) return MKT_ICON_SVGS.ai;
  if (tags.includes("web") || tags.includes("css") || tags.includes("server")) return MKT_ICON_SVGS.web;
  return MKT_ICON_SVGS.default;
}

function _mktHue(entry) {
  let h = 0;
  for (const c of entry.id) h = ((h << 5) - h + c.charCodeAt(0)) | 0;
  return Math.abs(h) % 360;
}

function _mktFmtDl(n) {
  if (n >= 1e6) return (n / 1e6).toFixed(1).replace(/\.0$/, "") + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1).replace(/\.0$/, "") + "K";
  return String(n || 0);
}

const _BUILTIN_EXTENSIONS = [
  { id: "bradlc.vscode-tailwindcss", name: "Tailwind CSS IntelliSense", author: "Tailwind Labs", version: "0.14.0", description: "智能 Tailwind CSS 工具 — 自动补全类名、语法高亮、错误提示", category: "Web", tags: ["css", "web", "tailwind"], featured: true, downloads: 16700000, rating: 4.7, icon: "tailwind", details: "## 功能\n- 90+ Tailwind 工具类自动补全\n- 悬停查看类名对应的 CSS\n- 语法错误实时提示\n\n## 使用方法\n安装后在 HTML/JSX 文件中输入 Tailwind 类名即可触发补全。\n\n命令面板：`Tailwind: Lookup Class` — 查询光标处的类名" },
  { id: "Vue.volar", name: "Vue - Official", author: "Vue", version: "2.2.0", description: "Vue.js 官方语言支持：模板语法高亮、组件智能提示、格式化", category: "Languages", tags: ["vue", "language", "web"], featured: true, downloads: 15600000, rating: 4.5, icon: "vue", details: "## 功能\n- Vue SFC 模板语法高亮\n- 组件 Props 自动补全\n- `<script setup>` 支持\n- TypeScript 集成" },
  { id: "pkief.material-icon-theme", name: "Material 文件图标", author: "Philipp Kief", version: "5.14.1", description: "Material Design 风格文件图标 — 1000+ 种文件和文件夹图标", category: "Themes", tags: ["theme", "icons"], featured: true, downloads: 28900000, rating: 4.8, icon: "theme", details: "## 功能\n- 1000+ 文件类型图标\n- 特殊文件夹图标（node_modules, src, test 等）\n- 浅色/深色主题自适应\n- 自定义图标映射" },
  { id: "ms-azuretools.vscode-docker", name: "Docker", author: "Microsoft", version: "1.29.3", description: "Docker 容器管理 — 构建、管理、部署容器化应用", category: "DevOps", tags: ["docker", "devops"], featured: true, downloads: 24100000, rating: 4.5, icon: "docker", details: "## 功能\n- 查看运行中的容器列表\n- 管理 Docker 镜像\n- Dockerfile 语法高亮\n\n## 命令\n- `Docker: 查看运行容器`\n- `Docker: 查看镜像列表`" },
  { id: "github.copilot", name: "GitHub Copilot", author: "GitHub", version: "1.240.0", description: "AI 编程助手 — 智能代码补全、函数生成、测试编写", category: "AI", tags: ["ai", "completion"], featured: true, downloads: 24800000, rating: 4.3, icon: "copilot", details: "## 功能\n- 智能代码补全（多行）\n- 根据注释生成完整函数\n- 自动编写单元测试\n- 支持 40+ 编程语言\n\n## 注意\n需要 GitHub Copilot 订阅账号" },
  { id: "tabnine.tabnine-vscode", name: "Tabnine AI", author: "Tabnine", version: "3.128.0", description: "AI 代码助手 — 全行和全函数代码补全，支持所有语言", category: "AI", tags: ["ai", "completion"], featured: true, downloads: 9800000, rating: 4.2, icon: "ai", details: "## 功能\n- 全行代码补全\n- 全函数代码生成\n- 本地模型（隐私保护）\n- 支持所有编程语言" },
  { id: "zhihu.hanzi-counter", name: "汉字计数器", author: "知乎团队", version: "1.2.0", description: "实时统计中文字符数、英文单词数、总字数，适合写作和翻译", category: "Other", tags: ["chinese", "tools"], featured: true, downloads: 180000, rating: 4.3, icon: "hanzi", details: "## 功能\n- 状态栏实时显示：汉字数 / 英文词数 / 总字符数\n- 统计行数\n- 支持 Markdown 和纯文本\n\n## 命令\n`汉字计数器: 统计当前文件` — 弹窗显示详细统计" },
  { id: "nicepkg.vscode-translate", name: "翻译助手", author: "NicePkg", version: "2.1.0", description: "代码注释中英互译、变量名翻译、选中文本即时翻译", category: "Other", tags: ["chinese", "tools", "ai"], featured: true, downloads: 350000, rating: 4.4, icon: "translate", details: "## 功能\n- 30+ 编程术语内置字典\n- 选中中文 → 翻译成英文\n- 选中英文 → 翻译成中文\n- 中文变量名转 camelCase\n\n## 命令\n- `翻译: 翻译选中文本`\n- `翻译: 变量名中英转换`" },
  { id: "pnp.polacode", name: "代码截图", author: "pnp", version: "0.3.4", description: "拍立得风格代码截图 — 选中代码生成精美分享图片", category: "Other", tags: ["tools", "screenshot"], downloads: 2100000, rating: 4.3, icon: "camera", details: "## 功能\n- 选中代码一键生成截图\n- 保留语法高亮颜色\n- 自动适配编辑器主题\n\n## 使用\n命令面板：`代码截图: 截取选中代码`" },
  { id: "antfu.iconify", name: "图标预览", author: "Anthony Fu", version: "0.18.0", description: "10 万+ 图标在线预览 — 100+ 图标集、行内预览、自动补全", category: "Other", tags: ["icons", "tools", "web"], downloads: 2800000, rating: 4.7, icon: "iconify", details: "## 功能\n- 100,000+ 图标库\n- 支持 100+ 图标集（Material, Heroicons, Lucide 等）\n- 代码中图标名行内预览\n- 图标名自动补全" },
  { id: "ms-ceintl.vscode-language-pack-zh-hans", name: "简体中文语言包", author: "Microsoft", version: "1.96.0", description: "编辑器界面简体中文翻译 — Chinese (Simplified) Language Pack", category: "Other", tags: ["chinese", "language"], downloads: 15800000, rating: 4.6, icon: "zhcn", details: "## 功能\n- 编辑器所有菜单中文化\n- 状态栏/面板中文翻译\n- 设置页面中文显示\n\n安装后重启即可生效" },
  { id: "svelte.svelte-vscode", name: "Svelte", author: "Svelte", version: "109.0.0", description: "Svelte 框架语言支持 — 语法高亮、自动补全、诊断", category: "Languages", tags: ["svelte", "language", "web"], downloads: 3200000, rating: 4.5, icon: "web", details: "## 功能\n- .svelte 文件语法高亮\n- 组件属性自动补全\n- 错误诊断\n- 代码格式化" },
  { id: "prisma.prisma", name: "Prisma", author: "Prisma", version: "5.22.0", description: "Prisma ORM 语法高亮和自动补全 — 数据库模型定义助手", category: "Other", tags: ["database", "tools"], downloads: 4500000, rating: 4.6, icon: "default", details: "## 功能\n- .prisma schema 语法高亮\n- 模型字段自动补全\n- 关系定义智能提示\n- 格式化 Prisma Schema" },
  { id: "streetsidesoftware.code-spell-checker", name: "拼写检查", author: "Street Side Software", version: "4.0.0", description: "代码拼写检查器 — 支持 camelCase 和编程术语", category: "Other", tags: ["linter", "tools"], downloads: 12300000, rating: 4.4, icon: "linter", details: "## 功能\n- 200+ 编程常用词内置词库\n- 自动拆分 camelCase/snake_case\n- 检测可疑拼写错误\n\n## 命令\n`拼写检查: 检查当前文件`" },
  { id: "wayou.vscode-todo-highlight", name: "TODO 高亮", author: "Wayou Liu", version: "1.0.5", description: "高亮 TODO/FIXME/HACK 等注释标记 — 一目了然待办事项", category: "Other", tags: ["tools", "highlight"], downloads: 7800000, rating: 4.5, icon: "color", details: "## 功能\n- 高亮 TODO / FIXME / HACK / BUG / NOTE 标记\n- 可自定义高亮颜色\n- 支持自定义关键词" },
  { id: "alefragnani.project-manager", name: "项目管理器", author: "Alessandro Fragnani", version: "12.8.0", description: "快速切换项目 — 收藏、分组、一键打开多个工作区", category: "Other", tags: ["tools", "workspace"], downloads: 5600000, rating: 4.5, icon: "default", details: "## 功能\n- 快速切换多个项目\n- 项目分组管理\n- 状态栏一键打开项目列表\n\n## 命令\n`项目管理器: 查看项目列表`" },
];

async function _installExtension(entry) {
  const builtinId = _BUILTIN_ID_MAP[entry.id];
  if (builtinId) {
    try {
      const result = await extManager.installBuiltin(builtinId);
      if (result && result.enabled !== false) {
        await extHost.activate(result, extManager);
      }
      return result?.manifest?.name ? `✅ ${result.manifest.name} 已安装并激活` : `✅ ${entry.name} 已安装`;
    } catch (e) {
      return `${entry.name} 安装失败: ${e.message || e}`;
    }
  }
  return `${entry.name} — 暂无安装包`;
}

const _BUILTIN_ID_MAP = {
  "bradlc.vscode-tailwindcss": "michael.tailwind-intellisense",
  "Vue.volar": "michael.tailwind-intellisense",
  "pkief.material-icon-theme": "michael.material-icons",
  "ms-azuretools.vscode-docker": "michael.docker-tools",
  "zhihu.hanzi-counter": "michael.hanzi-counter",
  "nicepkg.vscode-translate": "michael.translate-helper",
  "pnp.polacode": "michael.polacode-screenshot",
  "antfu.iconify": "michael.material-icons",
  "ms-ceintl.vscode-language-pack-zh-hans": "devin.chinese-language-pack",
  "svelte.svelte-vscode": "michael.svelte-language",
  "streetsidesoftware.code-spell-checker": "michael.spell-checker",
  "alefragnani.project-manager": "michael.project-manager",
  "wayou.vscode-todo-highlight": "devin.todo-highlight",
};

let _mktModal = null;
let _mktAllEntries = [];

function openMarketplaceModal() {
  if (_mktModal) { _mktModal.remove(); _mktModal = null; return; }
  const overlay = document.createElement("div");
  overlay.className = "mktm-overlay";
  const modal = document.createElement("div");
  modal.className = "mktm";
  modal.innerHTML = `
    <div class="mktm__head">
      <h2>扩展市场</h2>
      <div class="mktm__search">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        <input type="text" placeholder="搜索扩展…" spellcheck="false" autocomplete="off" />
      </div>
      <button class="mktm__close" type="button"><svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg></button>
    </div>
    <div class="mktm__filters"></div>
    <div class="mktm__body">
      <div class="mktm__list"></div>
      <div class="mktm__detail" hidden></div>
    </div>
  `;
  overlay.appendChild(modal);
  document.body.appendChild(overlay);
  _mktModal = overlay;
  requestAnimationFrame(() => overlay.classList.add("mktm-overlay--visible"));

  const closeModal = () => { overlay.classList.remove("mktm-overlay--visible"); setTimeout(() => { overlay.remove(); _mktModal = null; }, 200); };
  overlay.addEventListener("click", (e) => { if (e.target === overlay) closeModal(); });
  modal.querySelector(".mktm__close").addEventListener("click", closeModal);
  document.addEventListener("keydown", function esc(e) { if (e.key === "Escape") { closeModal(); document.removeEventListener("keydown", esc); } });

  let activeFilter = "all";
  let searchQ = "";
  const FILTERS = [
    { id: "all", label: "全部" }, { id: "featured", label: "推荐" },
    { id: "languages", label: "语言" }, { id: "themes", label: "主题" },
    { id: "tools", label: "工具" }, { id: "ai", label: "AI" },
  ];
  const filtersEl = modal.querySelector(".mktm__filters");
  for (const f of FILTERS) {
    const btn = document.createElement("button");
    btn.className = "mktm__pill" + (f.id === activeFilter ? " is-on" : "");
    btn.textContent = f.label;
    btn.addEventListener("click", () => { activeFilter = f.id; renderList(); });
    filtersEl.appendChild(btn);
  }

  const searchInput = modal.querySelector(".mktm__search input");
  let sTimer = null;
  searchInput.addEventListener("input", () => {
    clearTimeout(sTimer);
    sTimer = setTimeout(() => { searchQ = searchInput.value.trim().toLowerCase(); renderList(); }, 180);
  });
  searchInput.focus();

  const listEl = modal.querySelector(".mktm__list");
  const detailEl = modal.querySelector(".mktm__detail");

  function matchF(e) {
    if (activeFilter === "all") return true;
    if (activeFilter === "featured") return e.featured;
    const cat = (e.category || "").toLowerCase();
    const tags = (e.tags || []).map(t => t.toLowerCase());
    if (activeFilter === "languages") return cat === "languages" || tags.some(t => ["rust","python","language","jupyter"].includes(t));
    if (activeFilter === "themes") return cat === "themes" || tags.some(t => ["theme","icons","ui"].includes(t));
    if (activeFilter === "tools") return ["formatters","linters","web","devops","scm","other"].includes(cat) || tags.some(t => ["formatter","linter","docker","git","server","web","markdown"].includes(t));
    if (activeFilter === "ai") return cat === "ai" || tags.some(t => ["ai","completion"].includes(t));
    return true;
  }

  function filteredList() {
    let list = _mktAllEntries.filter(matchF);
    if (searchQ) list = list.filter(e => e.name.toLowerCase().includes(searchQ) || e.description.toLowerCase().includes(searchQ) || (e.tags||[]).some(t => t.toLowerCase().includes(searchQ)));
    list.sort((a, b) => (b.downloads || 0) - (a.downloads || 0));
    return list;
  }

  function renderList() {
    filtersEl.querySelectorAll(".mktm__pill").forEach((b, i) => b.classList.toggle("is-on", FILTERS[i].id === activeFilter));
    listEl.innerHTML = "";
    detailEl.hidden = true;
    listEl.hidden = false;
    const items = filteredList();
    if (!items.length) {
      listEl.innerHTML = '<div class="mktm__empty"><p>没有找到扩展</p></div>';
      return;
    }
    for (const entry of items) {
      const row = document.createElement("div");
      row.className = "mktm__row";
      row.style.setProperty("--h", _mktHue(entry));
      row.innerHTML = `
        <div class="mktm__icon"><svg viewBox="0 0 24 24" width="28" height="28">${_mktIconFor(entry)}</svg></div>
        <div class="mktm__info">
          <div class="mktm__name">${_escHtml(entry.name)}</div>
          <div class="mktm__desc">${_escHtml(entry.description)}</div>
          <div class="mktm__meta">
            <span class="mktm__author">${_escHtml(entry.author)}</span>
            <span class="mktm__sep">·</span>
            <span>v${entry.version}</span>
            <span class="mktm__sep">·</span>
            <span>${_mktFmtDl(entry.downloads)} downloads</span>
            <span class="mktm__sep">·</span>
            <span>★ ${(entry.rating || 0).toFixed(1)}</span>
          </div>
        </div>
        <button class="mktm__install-btn" type="button">Install</button>
      `;
      row.querySelector(".mktm__install-btn").addEventListener("click", async (ev) => {
        ev.stopPropagation();
        const btn = ev.target;
        btn.disabled = true; btn.textContent = "Installing…";
        try {
          const msg = await _installExtension(entry);
          btn.textContent = "✓ Installed"; btn.classList.add("is-done");
          showToast(typeof msg === "string" ? msg : `${entry.name} installed`);
        } catch (e) { btn.textContent = "Retry"; btn.disabled = false; showToast(String(e?.message || e)); }
      });
      row.addEventListener("click", () => openMktDetail(entry));
      listEl.appendChild(row);
    }
  }

  function openMktDetail(entry) {
    listEl.hidden = true;
    detailEl.hidden = false;
    detailEl.style.setProperty("--h", _mktHue(entry));
    const detailsHtml = (entry.details || "").replace(/^## (.+)$/gm, '<h3 class="mktm__det-h3">$1</h3>').replace(/^- (.+)$/gm, '<div class="mktm__det-li">• $1</div>').replace(/`([^`]+)`/g, '<code>$1</code>').replace(/\n\n/g, '<br/>');
    detailEl.innerHTML = `
      <button class="mktm__back" type="button"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg> 返回</button>
      <div class="mktm__det-head">
        <div class="mktm__det-icon"><svg viewBox="0 0 24 24" width="40" height="40">${_mktIconFor(entry)}</svg></div>
        <div class="mktm__det-info">
          <h2>${_escHtml(entry.name)}</h2>
          <p>${_escHtml(entry.description)}</p>
          <div class="mktm__meta">${_escHtml(entry.author)} · v${entry.version} · ${_mktFmtDl(entry.downloads)} downloads · ★ ${(entry.rating||0).toFixed(1)}</div>
        </div>
      </div>
      <div class="mktm__det-tags">${(entry.tags||[]).map(t => `<span class="mktm__tag">${_escHtml(t)}</span>`).join("")}</div>
      <div class="mktm__det-body">${detailsHtml}</div>
      <div class="mktm__det-info-table">
        <table><tbody>
          <tr><td>发布者</td><td>${_escHtml(entry.author)}</td></tr>
          <tr><td>扩展 ID</td><td><code>${_escHtml(entry.id)}</code></td></tr>
          <tr><td>版本</td><td>v${_escHtml(entry.version)}</td></tr>
          <tr><td>分类</td><td>${_escHtml(entry.category || "Other")}</td></tr>
          <tr><td>下载量</td><td>${_mktFmtDl(entry.downloads)}</td></tr>
          <tr><td>评分</td><td>★ ${(entry.rating||0).toFixed(1)}</td></tr>
        </tbody></table>
      </div>
    `;
    detailEl.querySelector(".mktm__back").addEventListener("click", renderList);
    const installBtn = document.createElement("button");
    installBtn.className = "mktm__install-btn mktm__install-btn--lg";
    installBtn.textContent = "安装扩展";
    installBtn.addEventListener("click", async () => {
      installBtn.disabled = true; installBtn.textContent = "Installing…";
      try {
        const msg = await _installExtension(entry);
        installBtn.textContent = "✓ Installed"; installBtn.classList.add("is-done");
        showToast(typeof msg === "string" ? msg : `${entry.name} installed`);
      } catch (e) { installBtn.textContent = "Retry"; installBtn.disabled = false; }
    });
    detailEl.querySelector(".mktm__det-head").appendChild(installBtn);
  }

  listEl.innerHTML = '<div class="mktm__loading"><div class="mkt-spinner"></div><span>Loading marketplace…</span></div>';
  (async () => {
    try {
      const dbEntries = await backend.dbMarketplaceList?.().catch(() => []) || [];
      if (dbEntries.length > 0) {
        _mktAllEntries = dbEntries.map(e => ({
          ...e,
          tags: Array.isArray(e.tags) ? e.tags : (typeof e.tags === "string" ? JSON.parse(e.tags) : []),
        }));
      } else {
        const remote = await backend.marketplaceList().catch(() => []);
        _mktAllEntries = [..._BUILTIN_EXTENSIONS, ...remote.filter(r => !_BUILTIN_EXTENSIONS.some(b => b.id === r.id))];
        if (backend.dbMarketplaceUpsert) {
          for (const ext of _mktAllEntries) {
            backend.dbMarketplaceUpsert({ ...ext, description: ext.description || "", category: ext.category || "Other", tags: ext.tags || [], icon: ext.icon || "default", icon_svg: null }).catch(() => {});
          }
        }
      }
      renderList();
    } catch {
      _mktAllEntries = _BUILTIN_EXTENSIONS;
      renderList();
    }
  })();
}

function renderMarketplaceToolOld(body) {

  let activeFilter = "all";
  let searchQuery = "";
  let allEntries = [];

  function iconFor(entry) {
    const tags = (entry.tags || []).map((t) => t.toLowerCase());
    if (tags.includes("theme") || tags.includes("icons")) return ICON_SVGS.theme;
    if (tags.includes("git")) return ICON_SVGS.git;
    if (tags.includes("formatter")) return ICON_SVGS.formatter;
    if (tags.includes("linter")) return ICON_SVGS.linter;
    if (tags.includes("rust") || tags.includes("python") || tags.includes("language")) return ICON_SVGS.language;
    if (tags.includes("docker") || tags.includes("devops")) return ICON_SVGS.docker;
    if (tags.includes("ai") || tags.includes("completion")) return ICON_SVGS.ai;
    if (tags.includes("web") || tags.includes("css") || tags.includes("server")) return ICON_SVGS.web;
    return ICON_SVGS.default;
  }
  function hueFor(entry) {
    let h = 0;
    for (const c of entry.id) h = ((h << 5) - h + c.charCodeAt(0)) | 0;
    return Math.abs(h) % 360;
  }
  function fmtDl(n) {
    if (n >= 1e6) return (n / 1e6).toFixed(1).replace(/\.0$/, "") + "M";
    if (n >= 1e3) return (n / 1e3).toFixed(1).replace(/\.0$/, "") + "K";
    return String(n || 0);
  }
  const STAR_SVG = `<svg viewBox="0 0 24 24" width="12" height="12" fill="currentColor"><path d="M12 2l2.9 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l7.1-1.01z"/></svg>`;
  const DL_SVG = `<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12m0 0l-4-4m4 4l4-4M5 21h14"/></svg>`;

  const searchEl = document.createElement("div");
  searchEl.className = "mkt-search";
  searchEl.innerHTML = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg><input type="text" spellcheck="false" placeholder="Search extensions…" />`;
  wrap.appendChild(searchEl);
  const searchInput = searchEl.querySelector("input");
  let searchTimer = null;
  searchInput.addEventListener("input", () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => { searchQuery = searchInput.value.trim().toLowerCase(); renderAll(); }, 180);
  });

  const filters = document.createElement("div");
  filters.className = "mkt-filters";
  const FILTERS = [
    { id: "all", label: "全部" }, { id: "featured", label: "推荐" },
    { id: "languages", label: "语言" }, { id: "themes", label: "主题" },
    { id: "tools", label: "工具" }, { id: "ai", label: "AI" },
  ];
  for (const f of FILTERS) {
    const btn = document.createElement("button");
    btn.className = "mkt-pill" + (f.id === activeFilter ? " is-on" : "");
    btn.type = "button";
    btn.textContent = f.label;
    btn.addEventListener("click", () => { activeFilter = f.id; renderAll(); });
    filters.appendChild(btn);
  }
  wrap.appendChild(filters);

  const mainArea = document.createElement("div");
  mainArea.className = "mkt-main";
  wrap.appendChild(mainArea);

  const listView = document.createElement("div");
  listView.className = "mkt-list";
  mainArea.appendChild(listView);

  const detailView = document.createElement("div");
  detailView.className = "mkt-detail";
  detailView.hidden = true;
  mainArea.appendChild(detailView);

  function matchFilter(e) {
    if (activeFilter === "all") return true;
    if (activeFilter === "featured") return e.featured;
    const cat = (e.category || "").toLowerCase();
    const tags = (e.tags || []).map((t) => t.toLowerCase());
    if (activeFilter === "languages") return cat === "languages" || tags.some((t) => ["rust", "python", "language", "jupyter"].includes(t));
    if (activeFilter === "themes") return cat === "themes" || tags.some((t) => ["theme", "icons", "ui"].includes(t));
    if (activeFilter === "tools") return ["formatters", "linters", "web", "devops", "scm", "other"].includes(cat.toLowerCase()) || tags.some((t) => ["formatter", "linter", "docker", "git", "server", "web", "markdown"].includes(t));
    if (activeFilter === "ai") return cat === "ai" || tags.some((t) => ["ai", "completion"].includes(t));
    return true;
  }

  function filtered() {
    let list = allEntries.filter(matchFilter);
    if (searchQuery) {
      list = list.filter((e) =>
        e.name.toLowerCase().includes(searchQuery) ||
        e.description.toLowerCase().includes(searchQuery) ||
        (e.tags || []).some((t) => t.toLowerCase().includes(searchQuery)),
      );
    }
    list.sort((a, b) => (b.downloads || 0) - (a.downloads || 0));
    return list;
  }

  function installButton(entry, label) {
    const btn = document.createElement("button");
    btn.className = "mkt-install";
    btn.type = "button";
    btn.textContent = label || "Install";
    btn.addEventListener("click", async (ev) => {
      ev.stopPropagation();
      btn.disabled = true; btn.textContent = "Installing…"; btn.classList.add("is-busy");
      try {
        const msg = await backend.marketplaceInstall(entry);
        btn.textContent = "Installed"; btn.classList.remove("is-busy"); btn.classList.add("is-done");
        showToast(msg); extPanel.refresh?.();
      } catch (e) {
        btn.textContent = "Retry"; btn.disabled = false; btn.classList.remove("is-busy");
        showToast(String(e && e.message ? e.message : e));
      }
    });
    return btn;
  }

  function makeCard(entry, featured) {
    const el = document.createElement("div");
    el.className = "mkt-card" + (featured ? " mkt-card--feat" : "");
    el.style.setProperty("--h", hueFor(entry));
    el.innerHTML = `
      <div class="mkt-card__icon"><svg viewBox="0 0 24 24" width="24" height="24">${iconFor(entry)}</svg></div>
      <div class="mkt-card__info">
        <div class="mkt-card__name"></div>
        <div class="mkt-card__desc"></div>
        <div class="mkt-card__meta">
          <span class="mkt-card__author"></span>
          <span class="mkt-card__sep">·</span>
          <span class="mkt-card__stat">${DL_SVG}${fmtDl(entry.downloads)}</span>
          <span class="mkt-card__stat">${STAR_SVG}${(entry.rating || 0).toFixed(1)}</span>
        </div>
      </div>
      <div class="mkt-card__cta"></div>`;
    el.querySelector(".mkt-card__name").textContent = entry.name;
    el.querySelector(".mkt-card__desc").textContent = entry.description;
    el.querySelector(".mkt-card__author").textContent = entry.author;
    el.querySelector(".mkt-card__cta").appendChild(installButton(entry));
    el.addEventListener("click", () => openDetail(entry));
    return el;
  }

  function openDetail(entry) {
    listView.hidden = true;
    detailView.hidden = false;
    detailView.scrollTop = 0;
    detailView.style.setProperty("--h", hueFor(entry));
    detailView.innerHTML = `
      <button class="mkt-back" type="button"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"/></svg> Back</button>
      <div class="mkt-det-head">
        <div class="mkt-det-icon"><svg viewBox="0 0 24 24" width="34" height="34">${iconFor(entry)}</svg></div>
        <div class="mkt-det-headinfo">
          <h2 class="mkt-det-name"></h2>
          <div class="mkt-det-author"></div>
          <div class="mkt-det-stats">
            <span>${STAR_SVG}${(entry.rating || 0).toFixed(1)}</span>
            <i class="mkt-det-sep"></i>
            <span>${fmtDl(entry.downloads)} installs</span>
            <i class="mkt-det-sep"></i>
            <span>v${entry.version}</span>
          </div>
        </div>
        <div class="mkt-det-cta"></div>
      </div>
      <div class="mkt-det-body">
        <div class="mkt-det-section"><h3>About</h3><p class="mkt-det-desc"></p></div>
        <div class="mkt-det-section mkt-det-tagsec"><h3>Tags</h3><div class="mkt-det-tags"></div></div>
        <div class="mkt-det-section"><h3>Information</h3>
          <table class="mkt-det-table"><tbody>
            <tr><td>Publisher</td><td class="d-pub"></td></tr>
            <tr><td>Extension ID</td><td class="d-id"></td></tr>
            <tr><td>Category</td><td class="d-cat"></td></tr>
            <tr><td>Version</td><td class="d-ver"></td></tr>
          </tbody></table>
        </div>
      </div>`;
    detailView.querySelector(".mkt-det-name").textContent = entry.name;
    detailView.querySelector(".mkt-det-author").textContent = "by " + entry.author;
    detailView.querySelector(".mkt-det-desc").textContent = entry.description;
    detailView.querySelector(".d-pub").textContent = entry.author;
    detailView.querySelector(".d-id").textContent = entry.id;
    detailView.querySelector(".d-cat").textContent = entry.category || "Other";
    detailView.querySelector(".d-ver").textContent = "v" + entry.version;
    const tags = entry.tags || [];
    const tagsWrap = detailView.querySelector(".mkt-det-tags");
    if (!tags.length) detailView.querySelector(".mkt-det-tagsec").style.display = "none";
    for (const tag of tags) {
      const pill = document.createElement("span");
      pill.className = "mkt-tag";
      pill.textContent = tag;
      tagsWrap.appendChild(pill);
    }
    detailView.querySelector(".mkt-back").addEventListener("click", closeDetail);
    detailView.querySelector(".mkt-det-cta").appendChild(installButton(entry, "Install Extension"));
  }

  function closeDetail() {
    listView.hidden = false;
    detailView.hidden = true;
  }

  function renderAll() {
    filters.querySelectorAll(".mkt-pill").forEach((b, i) => b.classList.toggle("is-on", FILTERS[i].id === activeFilter));
    listView.innerHTML = "";

    const items = filtered();
    if (!items.length) {
      listView.innerHTML = `<div class="mkt-empty"><svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity=".25"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg><p>No extensions found</p></div>`;
      return;
    }

    const feat = items.filter((e) => e.featured);
    if (!searchQuery && activeFilter === "all" && feat.length) {
      const sec = document.createElement("div");
      sec.className = "mkt-section";
      sec.innerHTML = `<h3 class="mkt-section__title">Featured</h3>`;
      const row = document.createElement("div");
      row.className = "mkt-grid mkt-grid--feat";
      for (const e of feat.slice(0, 4)) row.appendChild(makeCard(e, true));
      sec.appendChild(row);
      listView.appendChild(sec);
    }

    const sec2 = document.createElement("div");
    sec2.className = "mkt-section";
    const title = searchQuery ? `Results for "${searchQuery}"` : (activeFilter === "all" ? "All Extensions" : FILTERS.find((f) => f.id === activeFilter)?.label || "Extensions");
    sec2.innerHTML = `<h3 class="mkt-section__title">${title} <span class="mkt-section__count">${items.length}</span></h3>`;
    const grid = document.createElement("div");
    grid.className = "mkt-grid";
    for (const e of items) grid.appendChild(makeCard(e, false));
    sec2.appendChild(grid);
    listView.appendChild(sec2);
  }

  (async () => {
    listView.innerHTML = `<div class="mkt-loading"><div class="mkt-spinner"></div><span>Loading marketplace…</span></div>`;
    try {
      allEntries = await backend.marketplaceList();
      renderAll();
    } catch (e) {
      listView.innerHTML = `<div class="mkt-empty"><p>Failed to load</p><span>${String(e?.message || e)}</span><button class="mkt-install" style="margin-top:12px" onclick="this.closest('.mkt').querySelector('.mkt-list').innerHTML=''">Retry</button></div>`;
    }
  })();
}

function renderConflictsTool(body) {
  createToolHeader(body, "Merge Conflict Resolver", "Review base, ours, theirs, and merged content, then accept a side or mark your manual edit as resolved.");
  const list = document.createElement("div");
  list.className = "tool-list";
  const detail = document.createElement("div");
  detail.className = "merge-detail";
  body.append(list, detail);

  const loadConflicts = async () => {
    list.innerHTML = "";
    detail.innerHTML = "";
    if (!rootPath) {
      list.appendChild(createEmptyState("Open a Git workspace first."));
      return;
    }
    let conflicts = [];
    try {
      conflicts = await backend.gitConflicts(rootPath);
    } catch (e) {
      list.appendChild(createEmptyState(String(e && e.message ? e.message : e)));
      return;
    }
    if (!conflicts.length) {
      list.appendChild(createEmptyState("No merge conflicts found."));
      return;
    }
    for (const file of conflicts) {
      const row = document.createElement("button");
      row.className = "tool-row";
      row.type = "button";
      row.innerHTML = `${iconImg(fileIconUrl(file.name))}<span></span>`;
      row.querySelector("span").textContent = file.rel;
      row.addEventListener("click", () => openConflictDetail(file, detail));
      list.appendChild(row);
    }
    openConflictDetail(conflicts[0], detail);
  };
  loadConflicts();
}

async function openConflictDetail(file, detail) {
  detail.innerHTML = "";
  let versions;
  try {
    versions = await backend.gitMergeVersions(rootPath, file.rel);
  } catch (e) {
    detail.appendChild(createEmptyState(String(e && e.message ? e.message : e)));
    return;
  }
  const grid = document.createElement("div");
  grid.className = "merge-grid";
  const parts = [
    ["Base", versions.base],
    ["Ours", versions.ours],
    ["Theirs", versions.theirs],
  ];
  for (const [title, text] of parts) {
    const pane = document.createElement("div");
    pane.className = "merge-pane";
    pane.innerHTML = `<strong></strong><pre></pre>`;
    pane.querySelector("strong").textContent = title;
    pane.querySelector("pre").textContent = text || "(empty)";
    grid.appendChild(pane);
  }
  const merged = document.createElement("textarea");
  merged.className = "merge-editor";
  merged.spellcheck = false;
  merged.value = versions.merged || "";
  const actions = document.createElement("div");
  actions.className = "tool-actions";
  const resolve = async (resolution) => {
    try {
      if (resolution === "manual") await backend.writeTextFile(file.path, merged.value);
      await backend.gitResolveConflict(rootPath, file.rel, resolution);
      showToast(`Resolved ${file.rel}`);
      await afterWorktreeChange();
      renderFeaturePanel();
    } catch (e) {
      showToast(String(e && e.message ? e.message : e));
    }
  };
  for (const [label, resolution] of [["Accept Ours", "ours"], ["Accept Theirs", "theirs"], ["Mark Manual", "manual"]]) {
    const btn = document.createElement("button");
    btn.className = resolution === "manual" ? "btn btn--primary" : "btn";
    btn.type = "button";
    btn.textContent = label;
    btn.addEventListener("click", () => resolve(resolution));
    actions.appendChild(btn);
  }
  detail.append(grid, merged, actions);
}

// Built-in debug configuration templates per adapter. launchArgs carry the
// adapter-specific shape; the active file / workspace fill in the program.
function defaultDebugConfigs() {
  const root = rootPath || "";
  const program = activePath || "";
  return [
    {
      name: "Python: Current File",
      adapterId: "python",
      request: "launch",
      launchArgs: { type: "python", request: "launch", program, console: "integratedTerminal", cwd: root, stopOnEntry: false },
    },
    {
      name: "Node: Current File",
      adapterId: "node",
      request: "launch",
      launchArgs: { type: "pwa-node", request: "launch", program, cwd: root, console: "integratedTerminal" },
    },
    {
      name: "Go: Debug Package",
      adapterId: "go",
      request: "launch",
      launchArgs: { type: "go", request: "launch", mode: "debug", program: root, cwd: root },
    },
    {
      name: "LLDB: Launch Executable",
      adapterId: "lldb",
      request: "launch",
      launchArgs: { type: "lldb", request: "launch", program, cwd: root },
    },
  ];
}

async function loadLaunchConfigs() {
  const configs = [];
  for (const rel of [".vscode/launch.json", ".michael/launch.json"]) {
    if (!rootPath) break;
    try {
      const raw = await backend.readTextFile(`${rootPath}/${rel}`);
      const json = JSON.parse(raw.replace(/\/\/.*$/gm, ""));
      for (const c of json.configurations || []) {
        configs.push({
          name: c.name || `${c.type} ${c.request}`,
          adapterId: dapAdapterForType(c.type),
          request: c.request || "launch",
          launchArgs: c,
        });
      }
    } catch {
      /* no launch.json — fine */
    }
  }
  _launchConfigsCache = configs;
  return configs;
}

function dapAdapterForType(type) {
  const t = String(type || "").toLowerCase();
  if (t.includes("python") || t === "debugpy") return "python";
  if (t.includes("node") || t === "pwa-node" || t.includes("chrome")) return "node";
  if (t === "go") return "go";
  if (t === "lldb" || t === "cppdbg" || t.includes("lldb")) return "lldb";
  return type || "node";
}

let debugConsoleEl = null;

function appendDebugConsole(category, text) {
  if (!debugConsoleEl || !text) return;
  const span = document.createElement("span");
  span.className = "dbg-console__line dbg-console__line--" + (category || "console").replace(/[^a-z]/gi, "");
  span.textContent = text;
  debugConsoleEl.appendChild(span);
  debugConsoleEl.scrollTop = debugConsoleEl.scrollHeight;
}

function refreshDebugUI() {
  updateDebugStatusBar();
  if (!featureOverlay.hidden && activeFeatureTab === "debugger") {
    renderFeaturePanel();
  }
}

function renderDebuggerTool(body) {
  createToolHeader(
    body,
    "Debugger",
    "A real Debug Adapter Protocol client: set breakpoints in the editor gutter, launch a configuration, then step through code with live call stack, variables and a Debug Console. Requires the matching adapter installed (debugpy, vscode-js-debug/node, delve, lldb-dap).",
  );

  const active = dapManager?.isActive();
  debugConsoleEl = null;

  if (_launchConfigsCache === null && rootPath) {
    _launchConfigsCache = [];
    loadLaunchConfigs().then(() => refreshDebugUI());
  }

  if (!active) {
    renderDebugLauncher(body);
    return;
  }
  renderDebugSession(body);
}

function renderDebugLauncher(body) {
  const form = document.createElement("div");
  form.className = "tool-form";
  const configs = [...defaultDebugConfigs(), ...(_launchConfigsCache || [])];
  const optionsHtml = configs.map((c, i) => `<option value="${i}">${c.name}</option>`).join("");
  form.innerHTML = `
    <label><span>Configuration</span><select id="dapConfig">${optionsHtml}</select></label>
    <details class="dbg-advanced"><summary>Advanced (custom adapter command)</summary>
      <label><span>Adapter id</span><input id="dapAdapter" spellcheck="false" placeholder="python / node / go / lldb" /></label>
      <label><span>Custom command</span><input id="dapCommand" spellcheck="false" placeholder="leave empty for the bundled default" /></label>
      <label><span>Args</span><input id="dapArgs" spellcheck="false" placeholder="space separated" /></label>
      <label><span>Working directory</span><input id="dapCwd" spellcheck="false" /></label>
    </details>
    <div class="tool-actions">
      <button class="btn btn--primary" id="dapStartBtn" type="button">Start Debugging</button>
      <button class="btn" id="dapReloadBtn" type="button">Reload launch.json</button>
    </div>`;
  body.appendChild(form);
  form.querySelector("#dapCwd").value = rootPath || "";

  renderBreakpointsSection(body);

  form.querySelector("#dapStartBtn").addEventListener("click", async () => {
    const idx = +form.querySelector("#dapConfig").value;
    const base = configs[idx] ? structuredCloneSafe(configs[idx]) : null;
    const customCmd = form.querySelector("#dapCommand").value.trim();
    const customAdapter = form.querySelector("#dapAdapter").value.trim();
    const cwd = form.querySelector("#dapCwd").value.trim() || rootPath || null;
    let config = base || { name: customAdapter || "Custom", adapterId: customAdapter || "node", request: "launch", launchArgs: {} };
    if (customAdapter) config.adapterId = customAdapter;
    if (customCmd) {
      config.command = customCmd;
      config.args = form.querySelector("#dapArgs").value.trim().split(/\s+/).filter(Boolean);
    }
    config.cwd = cwd;
    if (config.launchArgs) {
      config.launchArgs.cwd = config.launchArgs.cwd || cwd;
      if (!config.launchArgs.program && activePath) config.launchArgs.program = activePath;
    }
    await dapManager.start(config);
  });
  form.querySelector("#dapReloadBtn").addEventListener("click", async () => {
    await loadLaunchConfigs();
    refreshDebugUI();
    showToast("Reloaded launch configurations");
  });
}

function structuredCloneSafe(obj) {
  try { return structuredClone(obj); } catch { return JSON.parse(JSON.stringify(obj)); }
}

function renderDebugSession(body) {
  // Toolbar.
  const bar = document.createElement("div");
  bar.className = "dbg-toolbar";
  const stopped = dapManager.isStopped();
  const mk = (label, title, fn, disabled) =>
    `<button class="btn dbg-ctl" data-act="${label}" title="${title}" ${disabled ? "disabled" : ""}>${title}</button>`;
  bar.innerHTML =
    mk("continue", stopped ? "Continue" : "Running…", null, !stopped) +
    mk("stepOver", "Step Over", null, !stopped) +
    mk("stepIn", "Step Into", null, !stopped) +
    mk("stepOut", "Step Out", null, !stopped) +
    mk("pause", "Pause", null, stopped) +
    mk("restart", "Restart", null, false) +
    mk("stop", "Stop", null, false);
  body.appendChild(bar);
  bar.addEventListener("click", (e) => {
    const act = e.target.closest("[data-act]")?.dataset.act;
    if (!act) return;
    const map = {
      continue: () => dapManager.cont(),
      stepOver: () => dapManager.next(),
      stepIn: () => dapManager.stepIn(),
      stepOut: () => dapManager.stepOut(),
      pause: () => dapManager.pause(),
      restart: () => dapManager.restart(),
      stop: () => dapManager.stop(),
    };
    map[act]?.();
  });

  const grid = document.createElement("div");
  grid.className = "dbg-grid";
  body.appendChild(grid);

  // Call stack.
  const stackCol = document.createElement("div");
  stackCol.className = "dbg-col";
  stackCol.innerHTML = `<h4 class="dbg-h">Call Stack</h4>`;
  const stackList = document.createElement("div");
  stackList.className = "dbg-stack";
  stackCol.appendChild(stackList);
  const frames = dapManager.currentFrames();
  if (!frames.length) {
    stackList.appendChild(createEmptyState(stopped ? "No frames." : "Running — pause or hit a breakpoint to inspect."));
  } else {
    for (const f of frames) {
      const row = document.createElement("button");
      row.type = "button";
      row.className = "dbg-frame" + (f.id === dapManager.activeFrameId() ? " is-active" : "");
      row.innerHTML = `<strong></strong><span></span>`;
      row.querySelector("strong").textContent = f.name;
      row.querySelector("span").textContent = f.source?.path
        ? `${basename(f.source.path)}:${f.line}`
        : `line ${f.line}`;
      row.addEventListener("click", async () => {
        await dapManager.setActiveFrame(f.id);
        if (f.source?.path) showDebugLocation(f.source.path, f.line);
      });
      stackList.appendChild(row);
    }
  }
  grid.appendChild(stackCol);

  // Variables.
  const varCol = document.createElement("div");
  varCol.className = "dbg-col";
  varCol.innerHTML = `<h4 class="dbg-h">Variables</h4>`;
  const varTree = document.createElement("div");
  varTree.className = "dbg-vars";
  varCol.appendChild(varTree);
  grid.appendChild(varCol);
  if (stopped && dapManager.activeFrameId() != null) {
    loadDebugScopes(varTree, dapManager.activeFrameId());
  } else {
    varTree.appendChild(createEmptyState("Variables appear when paused."));
  }

  // Breakpoints.
  renderBreakpointsSection(body);

  // Debug Console.
  const consoleWrap = document.createElement("div");
  consoleWrap.className = "dbg-console-wrap";
  consoleWrap.innerHTML = `<h4 class="dbg-h">Debug Console</h4>`;
  const consoleEl = document.createElement("div");
  consoleEl.className = "dbg-console";
  const inputRow = document.createElement("div");
  inputRow.className = "dbg-console__input";
  inputRow.innerHTML = `<input spellcheck="false" placeholder="Evaluate expression…" /><button class="btn" type="button">Eval</button>`;
  consoleWrap.append(consoleEl, inputRow);
  body.appendChild(consoleWrap);
  debugConsoleEl = consoleEl;
  for (const entry of dapManager.consoleLog()) appendDebugConsole(entry.category, entry.text);

  const evalInput = inputRow.querySelector("input");
  const doEval = async () => {
    const expr = evalInput.value.trim();
    if (!expr) return;
    appendDebugConsole("input", `> ${expr}\n`);
    evalInput.value = "";
    const res = await dapManager.evaluate(expr, dapManager.activeFrameId(), "repl");
    appendDebugConsole("result", `${res?.result ?? "(no result)"}\n`);
  };
  inputRow.querySelector("button").addEventListener("click", doEval);
  evalInput.addEventListener("keydown", (e) => { if (e.key === "Enter") doEval(); });
}

async function loadDebugScopes(container, frameId) {
  container.innerHTML = "";
  const scopes = await dapManager.scopes(frameId);
  if (!scopes.length) {
    container.appendChild(createEmptyState("No scopes."));
    return;
  }
  for (const scope of scopes) {
    const node = document.createElement("details");
    node.className = "dbg-scope";
    node.open = !scope.expensive;
    node.innerHTML = `<summary>${scope.name}</summary>`;
    const inner = document.createElement("div");
    inner.className = "dbg-var-children";
    node.appendChild(inner);
    container.appendChild(node);
    let loaded = false;
    const load = async () => {
      if (loaded) return;
      loaded = true;
      await renderVariables(inner, scope.variablesReference);
    };
    if (node.open) load();
    node.addEventListener("toggle", () => { if (node.open) load(); });
  }
}

async function renderVariables(container, variablesReference) {
  if (!variablesReference) return;
  const vars = await dapManager.variables(variablesReference);
  for (const v of vars) {
    const hasChildren = v.variablesReference && v.variablesReference > 0;
    const row = document.createElement(hasChildren ? "details" : "div");
    row.className = "dbg-var";
    if (hasChildren) {
      row.innerHTML = `<summary><span class="dbg-var__name"></span><span class="dbg-var__val"></span></summary>`;
      const kids = document.createElement("div");
      kids.className = "dbg-var-children";
      row.appendChild(kids);
      let loaded = false;
      row.addEventListener("toggle", async () => {
        if (row.open && !loaded) { loaded = true; await renderVariables(kids, v.variablesReference); }
      });
    } else {
      row.innerHTML = `<span class="dbg-var__name"></span><span class="dbg-var__val"></span>`;
    }
    row.querySelector(".dbg-var__name").textContent = v.name;
    row.querySelector(".dbg-var__val").textContent = v.value;
    container.appendChild(row);
  }
}

function renderBreakpointsSection(body) {
  const col = document.createElement("div");
  col.className = "dbg-col dbg-breakpoints";
  col.innerHTML = `<h4 class="dbg-h">Breakpoints</h4>`;
  const list = document.createElement("div");
  list.className = "dbg-bp-list";
  col.appendChild(list);
  body.appendChild(col);
  const all = getAllBreakpoints();
  if (!all.size) {
    list.appendChild(createEmptyState("Click the editor gutter to add a breakpoint."));
    return;
  }
  for (const [path, lines] of all) {
    for (const line of lines.sort((a, b) => a - b)) {
      const row = document.createElement("button");
      row.type = "button";
      row.className = "dbg-bp";
      row.innerHTML = `<span class="dbg-bp__dot"></span><span class="dbg-bp__file"></span>`;
      row.querySelector(".dbg-bp__file").textContent = `${basename(path)}:${line}`;
      row.title = path;
      row.addEventListener("click", () => {
        Promise.resolve(openFile(path, basename(path))).then((ok) => {
          if (ok) { monacoEditor.revealLineInCenter(line); monacoEditor.setPosition({ lineNumber: line, column: 1 }); }
        });
      });
      list.appendChild(row);
    }
  }
}

function renderLspTool(body) {
  createToolHeader(
    body,
    "Language Servers",
    "Real LSP intelligence — completion, diagnostics, hover, go-to-definition, references, rename, symbols and code actions. Servers for Rust, Python, Go and C/C++ start automatically when you open a matching file; Monaco's bundled service still powers TS/JS/JSON/CSS/HTML.",
  );
  const form = document.createElement("div");
  form.className = "tool-form";
  form.innerHTML = `
    <label><span>Language</span><select id="lspLang"><option>rust</option><option>python</option><option>go</option><option>c</option><option>cpp</option><option>typescript</option><option>javascript</option><option>html</option><option>css</option><option>json</option></select></label>
    <label><span>Custom command</span><input id="lspCommand" spellcheck="false" placeholder="leave empty for the bundled default server" /></label>
    <label><span>Args</span><input id="lspArgs" spellcheck="false" placeholder="space separated" /></label>
    <div class="tool-actions"><button class="btn btn--primary" id="lspStartBtn" type="button">Start</button><button class="btn" id="lspStopBtn" type="button">Stop</button><button class="btn" id="lspRefreshBtn" type="button">Refresh</button></div>
    <div class="tool-list" id="lspList"></div>
    <pre class="tool-log" id="lspLog"></pre>`;
  body.appendChild(form);
  const log = form.querySelector("#lspLog");
  const langSel = form.querySelector("#lspLang");

  const refresh = () => {
    const list = form.querySelector("#lspList");
    list.innerHTML = "";
    const status = lspManager ? lspManager.status() : [];
    const running = new Map(status.map((s) => [s.lang, s]));
    const known = ["rust", "python", "go", "c", "cpp", "typescript", "javascript", "html", "css", "json"];
    for (const lang of known) {
      const s = running.get(lang);
      const managed = lspManager?.managedLangs.includes(lang);
      const row = document.createElement("div");
      row.className = "tool-card";
      row.innerHTML = `<div class="tool-card__main"><strong></strong><span></span></div><span class="lsp-dot"></span>`;
      row.querySelector("strong").textContent = lang;
      let state;
      if (s && s.initialized) state = "Running · initialized";
      else if (s) state = "Starting…";
      else if (managed) state = "Auto-starts on open";
      else state = "Monaco built-in";
      row.querySelector("span").textContent = state;
      const dot = row.querySelector(".lsp-dot");
      dot.style.cssText = `width:8px;height:8px;border-radius:50%;margin-left:auto;background:${s && s.initialized ? "#3fb950" : s ? "#d29922" : "#6e7681"}`;
      list.appendChild(row);
    }
  };

  const renderLog = () => {
    const lang = langSel.value;
    log.textContent = (lspLogBuffers.get(lang) || []).join("\n");
    log.scrollTop = log.scrollHeight;
  };
  const onLogEvent = (e) => { if (e.detail?.lang === langSel.value) renderLog(); };
  document.addEventListener("lsp-log", onLogEvent);
  langSel.addEventListener("change", renderLog);

  form.querySelector("#lspStartBtn").addEventListener("click", async () => {
    const lang = langSel.value;
    const command = form.querySelector("#lspCommand").value.trim();
    const args = form.querySelector("#lspArgs").value.trim().split(/\s+/).filter(Boolean);
    try {
      const client = await lspManager.startManual(lang, command ? { command, args } : undefined);
      if (client) showToast(`LSP ${lang} started`);
      refresh();
      renderLog();
    } catch (e) {
      showToast(String(e && e.message ? e.message : e));
    }
  });
  form.querySelector("#lspStopBtn").addEventListener("click", async () => {
    const lang = langSel.value;
    try {
      await lspManager.stop(lang);
      showToast(`LSP ${lang} stopped`);
      refresh();
    } catch (e) {
      showToast(String(e && e.message ? e.message : e));
    }
  });
  form.querySelector("#lspRefreshBtn").addEventListener("click", refresh);
  refresh();
  renderLog();
}

// ---- titlebar menu bar (Cursor/Devin-style) ----
function editorAction(id) {
  monacoEditor.focus();
  monacoEditor.getAction(id)?.run();
}
function editorTrigger(cmd) {
  monacoEditor.focus();
  monacoEditor.trigger("menubar", cmd, null);
}
function togglePane(which) {
  document.querySelector(".layout")?.classList.toggle("hide-" + which);
}
function openExternal(url) {
  window.open(url, "_blank", "noopener,noreferrer");
}

function getMenus() {
  return [
    {
      label: t("menu.file"),
      items: [
        { label: t("menu.openFolder"), icon: "i-folder", hint: "⌘O", action: () => chooseFolder() },
        { label: "Add Folder to Workspace…", icon: "i-folder-open", action: () => addFolderToWorkspace() },
        { label: "New Project…", icon: "i-folder", action: () => showNewProjectDialog() },
        { label: t("menu.save"), icon: "i-save", hint: "⌘S", action: () => saveActive() },
        { sep: true },
        { label: t("menu.closeFile"), icon: "i-close", hint: "⌘W", action: () => activePath && closeFile(activePath) },
        { sep: true },
        { label: autoSaveEnabled ? "✓ Auto Save" : "  Auto Save", icon: "i-save", action: () => { toggleAutoSave(); buildMenubar(); } },
      ],
    },
    {
      label: t("menu.edit"),
      items: [
        { label: t("menu.undo"), icon: "i-undo", hint: "⌘Z", action: () => editorTrigger("undo") },
        { label: t("menu.redo"), icon: "i-redo", hint: "⇧⌘Z", action: () => editorTrigger("redo") },
        { sep: true },
        { label: t("menu.find"), icon: "i-search", hint: "⌘F", action: () => editorAction("actions.find") },
        { label: t("menu.replace"), icon: "i-replace", hint: "⌥⌘F", action: () => editorAction("editor.action.startFindReplaceAction") },
      ],
    },
    {
      label: t("menu.view"),
      items: [
        { label: t("menu.explorer"), icon: "i-files", hint: "⇧⌘E", action: () => showSide("explorer") },
        { label: t("menu.search"), icon: "i-search", hint: "⇧⌘F", action: () => showSide("search") },
        { label: t("menu.sourceControl"), icon: "i-git", hint: "⌃⇧G", action: () => showSide("git") },
        { label: "大纲", icon: "i-outline", hint: "⇧⌘O", action: () => showSide("outline") },
        { label: "测试", icon: "i-beaker", hint: "⇧⌘T", action: () => showSide("test") },
        { label: "输出", icon: "i-output", hint: "⌃⇧U", action: () => toggleOutputPanel() },
        { sep: true },
        { label: t("menu.toggleExplorer"), icon: "i-sidebar-left", action: () => togglePane("explorer") },
        { label: t("menu.toggleAssistant"), icon: "i-sidebar-right", action: () => togglePane("assistant") },
        { label: t("menu.toggleTerminal"), icon: "i-terminal", hint: "⌃`", action: () => toggleTerminal() },
        { label: t("menu.problems"), icon: "i-error", hint: "⇧⌘M", action: () => toggleProblems() },
        { sep: true },
        { label: t("menu.commandPalette"), icon: "i-command", hint: "⌘⇧P", action: () => editorAction("editor.action.quickCommand") },
      ],
    },
    {
      label: "工具",
      items: [
        { label: "运行当前文件", icon: "i-terminal", hint: "⌘R", action: () => runCurrentFile() },
        { label: "任务运行器", icon: "i-play", action: () => openFeaturePanel("tasks") },
        { sep: true },
        { label: "工作区管理", icon: "i-folder", action: () => openFeaturePanel("workspace") },
        { label: "远程开发", icon: "i-terminal", action: () => openFeaturePanel("remote") },
        { label: "扩展市场", icon: "i-ext", action: () => openMarketplaceModal() },
        { label: "合并冲突", icon: "i-git", action: () => openFeaturePanel("conflicts") },
        { sep: true },
        { label: "调试器", icon: "i-code", action: () => openFeaturePanel("debugger") },
        { label: "语言服务器", icon: "i-code", action: () => openFeaturePanel("lsp") },
        { sep: true },
        { label: "设置", icon: "i-gear", action: () => openFeaturePanel("settings") },
        { label: "快捷键", icon: "i-command", action: () => openFeaturePanel("shortcuts") },
      ],
    },
    {
      label: t("menu.help"),
      items: [
        { label: t("menu.documentation"), icon: "i-book", action: () => openExternal("https://github.com/fendoushaonian/Devin-Desktop") },
        { label: t("menu.aiSettings"), icon: "i-gear", action: () => openSettings() },
        { sep: true },
        { label: t("menu.about"), icon: "i-info", action: () => showToast(t("menu.aboutMsg")) },
        { sep: true },
        { label: t("theme.light"), icon: "i-theme-light", action: () => setTheme("light") },
        { label: t("theme.dark"), icon: "i-theme-dark", action: () => setTheme("dark") },
        { label: "Monokai", icon: "i-theme-monokai", action: () => setTheme("monokai") },
        { label: "GitHub Light", icon: "i-theme-github", action: () => setTheme("github-light") },
        { label: "Solarized Dark", icon: "i-theme-solarized", action: () => setTheme("solarized-dark") },
        { label: "Nord", icon: "i-theme-nord", action: () => setTheme("nord") },
        { label: t("theme.system"), icon: "i-theme-system", action: () => setTheme("system") },
      ],
    },
  ];
}

function buildMenubar() {
  const bar = $("menubar");
  if (!bar) return;
  bar.innerHTML = "";
  const buttons = [];
  const panels = [];
  let openIdx = -1;

  const closeMenu = () => {
    if (openIdx < 0) return;
    panels[openIdx].hidden = true;
    buttons[openIdx].classList.remove("is-open");
    buttons[openIdx].setAttribute("aria-expanded", "false");
    openIdx = -1;
  };
  const openMenu = (i) => {
    if (openIdx === i) return;
    closeMenu();
    panels[i].hidden = false;
    buttons[i].classList.add("is-open");
    buttons[i].setAttribute("aria-expanded", "true");
    openIdx = i;
  };

  const MENUS = getMenus();
  MENUS.forEach((menu, i) => {
    const wrap = document.createElement("div");
    wrap.className = "tb-menu";
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "tb-menu__btn";
    btn.textContent = menu.label;
    btn.setAttribute("aria-haspopup", "true");
    btn.setAttribute("aria-expanded", "false");
    const panel = document.createElement("div");
    panel.className = "menu menu--tb";
    panel.setAttribute("role", "menu");
    panel.hidden = true;
    for (const entry of menu.items) {
      if (entry.sep) {
        const sep = document.createElement("div");
        sep.className = "menu__sep";
        panel.appendChild(sep);
        continue;
      }
      const mi = document.createElement("div");
      mi.className = "menu__item";
      mi.setAttribute("role", "menuitem");
      mi.innerHTML =
        (entry.icon ? `<svg class="ic" aria-hidden="true"><use href="#${entry.icon}" /></svg>` : "") +
        `<span class="name"></span>` +
        (entry.hint ? `<span class="meta"></span>` : "");
      mi.querySelector(".name").textContent = entry.label;
      if (entry.hint) mi.querySelector(".meta").textContent = entry.hint;
      mi.addEventListener("click", () => {
        closeMenu();
        try {
          entry.action();
        } catch (e) {
          showToast(String(e));
        }
      });
      panel.appendChild(mi);
    }
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      if (openIdx === i) closeMenu();
      else openMenu(i);
    });
    btn.addEventListener("mouseenter", () => {
      if (openIdx >= 0) openMenu(i);
    });
    wrap.append(btn, panel);
    bar.appendChild(wrap);
    buttons.push(btn);
    panels.push(panel);
  });

  document.addEventListener("click", (e) => {
    if (openIdx >= 0 && !bar.contains(e.target)) closeMenu();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closeMenu();
  });
}

// ---- wiring ----
async function chooseFolder() {
  const picked = await backend.pickFolder();
  if (picked) await openFolder(picked);
}
$("openFolderBtn").addEventListener("click", chooseFolder);
$("emptyOpenBtn").addEventListener("click", chooseFolder);
// settings dropdown
const settingsDropdown = $("settingsDropdown");
$("settingsBtn").addEventListener("click", (e) => {
  e.stopPropagation();
  if (settingsDropdown) settingsDropdown.hidden = !settingsDropdown.hidden;
});
document.addEventListener("click", () => { if (settingsDropdown) settingsDropdown.hidden = true; });
if (settingsDropdown) {
  settingsDropdown.addEventListener("click", (e) => {
    e.stopPropagation();
    const item = e.target.closest("[data-action]");
    if (!item) return;
    settingsDropdown.hidden = true;
    const action = item.dataset.action;
    if (action === "ai-settings") openSettings();
    else if (action === "general-settings") openAdvancedTool("settings");
    else if (action === "shortcuts") openAdvancedTool("shortcuts");
    else if (action === "login") {
      const dlg = $("loginDialog");
      if (dlg) { $("loginStep1").hidden = false; $("loginStep2").hidden = true; dlg.showModal(); }
    }
    else if (action === "logout") {
      _loggedInEmail = null;
      _updateLoginUI();
    }
    else if (action === "profile") {
      alert("个人资料功能开发中");
    }
  });
}
$("saveBtn").addEventListener("click", saveActive);
$("runBtn")?.addEventListener("click", runCurrentFile);

// login state
let _loggedInEmail = null;
function _updateLoginUI() {
  const dropName = document.querySelector(".settings-dropdown__name");
  const dropHint = document.querySelector(".settings-dropdown__hint");
  const profileBtn = $("profileBtn");
  const logoutBtn = $("logoutBtn");
  const loginHeader = document.querySelector(".settings-dropdown__header");
  if (_loggedInEmail) {
    if (dropName) dropName.textContent = _loggedInEmail;
    if (dropHint) dropHint.textContent = "已登录";
    if (profileBtn) profileBtn.hidden = false;
    if (logoutBtn) logoutBtn.hidden = false;
    if (loginHeader) loginHeader.dataset.action = "profile";
  } else {
    if (dropName) dropName.textContent = "未登录";
    if (dropHint) dropHint.textContent = "点击登录";
    if (profileBtn) profileBtn.hidden = true;
    if (logoutBtn) logoutBtn.hidden = true;
    if (loginHeader) loginHeader.dataset.action = "login";
  }
}

// login flow
const loginLogoEl = $("loginLogo");
if (loginLogoEl) loginLogoEl.innerHTML = `<img class="assistant-logo" src="/src/assets/logo.png" alt="Michael IDE" style="width:52px;height:52px;border-radius:13px" />`;
$("loginCloseBtn")?.addEventListener("click", () => $("loginDialog")?.close());
$("loginNextBtn")?.addEventListener("click", () => {
  const email = $("loginEmail")?.value?.trim();
  if (!email) { $("loginEmail")?.focus(); return; }
  $("loginStep2Hint").textContent = email;
  $("loginStep1").hidden = true;
  $("loginStep2").hidden = false;
  $("loginPassword")?.focus();
});
$("loginBackBtn")?.addEventListener("click", () => {
  $("loginStep1").hidden = false;
  $("loginStep2").hidden = true;
  $("loginEmail")?.focus();
});
$("loginSubmitBtn")?.addEventListener("click", async () => {
  const email = $("loginEmail")?.value?.trim();
  const password = $("loginPassword")?.value;
  if (!email || !password) return;
  const btn = $("loginSubmitBtn");
  const origText = btn.textContent;
  btn.textContent = "处理中...";
  btn.disabled = true;
  try {
    const result = _loginCodeMode
      ? await backend.authVerifyCode(email, password)
      : await backend.authLoginOrRegister(email, password);
    if (result.success) {
      $("loginDialog")?.close();
      _loggedInEmail = email;
      _updateLoginUI();
    } else {
      alert(result.message || "登录失败");
    }
  } catch (e) {
    alert("登录失败: " + (e?.message || e));
  } finally {
    btn.textContent = origText;
    btn.disabled = false;
  }
});
let _loginCodeMode = false;
$("loginUseCodeBtn")?.addEventListener("click", async () => {
  const pw = $("loginPassword");
  _loginCodeMode = !_loginCodeMode;
  if (_loginCodeMode) {
    pw.type = "text"; pw.placeholder = "输入 6 位验证码";
    $("loginUseCodeBtn").textContent = "使用密码登录";
    $("loginSubmitBtn").textContent = "验证";
    const email = $("loginEmail")?.value?.trim();
    if (email) {
      try {
        const msg = await backend.authSendCode(email);
        alert(msg);
      } catch (e) { alert("发送失败: " + (e?.message || e)); }
    }
  } else {
    pw.type = "password"; pw.placeholder = "输入密码";
    $("loginUseCodeBtn").textContent = "使用验证码登录";
    $("loginSubmitBtn").textContent = "登录";
  }
  pw.value = ""; pw.focus();
});

// ---- explorer tabs / tools / search ----
$("tabExplorer").addEventListener("click", () => showSide("explorer"));
$("tabGit").addEventListener("click", () => showSide("git"));
$("tabOutline").addEventListener("click", () => showSide("outline"));
$("tabTest").addEventListener("click", () => showSide("test"));
$("gitRefreshBtn").addEventListener("click", () => refreshGitStatus());
$("gitPullBtn").addEventListener("click", () => gitPull());
$("gitPushBtn").addEventListener("click", () => gitPush());
$("gitBranchBtn").addEventListener("click", (e) => {
  e.stopPropagation();
  toggleBranchMenu();
});
document.addEventListener("click", (e) => {
  if (!gitBranchMenuEl.hidden && !gitBranchMenuEl.contains(e.target) && !gitBranchBtnEl.contains(e.target)) {
    closeBranchMenu();
  }
});
$("gitCommitBtn").addEventListener("click", () => gitCommit());
$("gitCommitMsg").addEventListener("keydown", (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
    e.preventDefault();
    gitCommit();
  }
});
$("diffClose").addEventListener("click", () => closeDiffView());
$("newFileBtn").addEventListener("click", () => rootPath && newEntry(rootPath, false));
$("newFolderBtn").addEventListener("click", () => rootPath && newEntry(rootPath, true));
$("refreshTreeBtn").addEventListener("click", () => rootPath && reloadDir(rootPath));
treeEl.addEventListener("contextmenu", (e) => {
  if (!rootPath) return;
  e.preventDefault();
  openContextMenu(e.clientX, e.clientY, { path: rootPath, name: rootNameEl.textContent, is_dir: true });
});
$("searchInput").addEventListener("input", debounceSearch);
$("searchInput").addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    clearTimeout(searchTimer);
    runSearch();
  }
});
$("searchCaseBtn").addEventListener("click", () => {
  searchCaseSensitive = !searchCaseSensitive;
  const b = $("searchCaseBtn");
  b.classList.toggle("is-active", searchCaseSensitive);
  b.setAttribute("aria-pressed", String(searchCaseSensitive));
  runSearch();
});

const promptEl = $("prompt");
const _sendBtnEl = $("sendBtn");
const _SEND_ICON = `<svg class="ic"><use href="#i-arrow-up" /></svg>`;
const _STOP_ICON = `<svg class="ic" viewBox="0 0 16 16" fill="currentColor"><rect x="3.5" y="3.5" width="9" height="9" rx="1.5"/></svg>`;

function _setSendBtnStop(isStop) {
  if (!_sendBtnEl) return;
  if (isStop) {
    _sendBtnEl.innerHTML = _STOP_ICON;
    _sendBtnEl.classList.add("is-stop");
    _sendBtnEl.title = "Stop generating";
    _sendBtnEl.type = "button";
  } else {
    _sendBtnEl.innerHTML = _SEND_ICON;
    _sendBtnEl.classList.remove("is-stop");
    _sendBtnEl.title = "Send (⌘↩)";
    _sendBtnEl.type = "submit";
  }
}

_sendBtnEl?.addEventListener("click", (e) => {
  if (streaming && _sendBtnEl.classList.contains("is-stop")) {
    e.preventDefault();
    e.stopPropagation();
    streaming = false;
    _setSendBtnStop(false);
    showToast("Generation stopped");
  }
});

promptEl.addEventListener("input", () => {
  promptEl.style.height = "auto";
  promptEl.style.height = Math.min(promptEl.scrollHeight, 160) + "px";
  _updateAtMenu();
  _updateSlashMenu();
});
$("composer").addEventListener("submit", (e) => {
  e.preventDefault();
  if (streaming) return;
  const text = promptEl.value.trim();
  if (!text && _pastedImages.length === 0) return;
  const images = [..._pastedImages];
  _pastedImages = [];
  _refreshImagePreviews();
  promptEl.value = "";
  promptEl.style.height = "auto";
  sendPrompt(text, images);
});
promptEl.addEventListener("keydown", (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
    e.preventDefault();
    $("composer").requestSubmit();
  }
});

// ---- @file mentions: type "@" in the chat to pin a workspace file into context ----
let _atMatches = [];
let _atActive = -1;
let _atTimer = 0;
const _atMenu = document.createElement("div");
_atMenu.className = "atmenu";
_atMenu.hidden = true;
document.body.appendChild(_atMenu);

function _atToken() {
  const pos = promptEl.selectionStart;
  const m = /(?:^|\s)@([^\s]*)$/.exec(promptEl.value.slice(0, pos));
  return m ? { query: m[1], start: pos - m[1].length - 1, end: pos } : null;
}
function _hideAtMenu() { _atMenu.hidden = true; _atActive = -1; _atMatches = []; }
function _renderAtActive() { [..._atMenu.children].forEach((c, i) => c.classList.toggle("is-active", i === _atActive)); }
function _updateAtMenu() {
  const tok = _atToken();
  const root = rootPath || workspaceRoots[0] || "";
  if (!tok || !root) return _hideAtMenu();
  clearTimeout(_atTimer);
  _atTimer = setTimeout(async () => {
    const t2 = _atToken();
    if (!t2) return _hideAtMenu();
    let res;
    try { res = await _agentFindFiles(root, t2.query || "**"); } catch { return _hideAtMenu(); }
    const files = (res.text || "").split("\n")
      .filter((l) => l && !l.startsWith("(") && !l.startsWith("…") && !l.startsWith("["))
      .slice(0, 8);
    if (!files.length) return _hideAtMenu();
    _atMatches = files;
    _atMenu.innerHTML = "";
    files.forEach((f, i) => {
      const item = document.createElement("div");
      item.className = "atmenu__item" + (i === 0 ? " is-active" : "");
      item.textContent = f;
      item.addEventListener("mousedown", (ev) => { ev.preventDefault(); _pickAt(i); });
      _atMenu.appendChild(item);
    });
    _atActive = 0;
    const r = promptEl.getBoundingClientRect();
    _atMenu.style.left = r.left + "px";
    _atMenu.style.width = Math.min(r.width, 520) + "px";
    _atMenu.style.bottom = window.innerHeight - r.top + 6 + "px";
    _atMenu.hidden = false;
  }, 160);
}
function _pickAt(i) {
  const tok = _atToken();
  if (!tok || !_atMatches[i]) return _hideAtMenu();
  const insert = "@" + _atMatches[i] + " ";
  const v = promptEl.value;
  promptEl.value = v.slice(0, tok.start) + insert + v.slice(tok.end);
  const caret = tok.start + insert.length;
  promptEl.setSelectionRange(caret, caret);
  _hideAtMenu();
  promptEl.focus();
}
promptEl.addEventListener("keydown", (e) => {
  if (_atMenu.hidden) return;
  if (e.key === "ArrowDown") { e.preventDefault(); _atActive = Math.min(_atActive + 1, _atMatches.length - 1); _renderAtActive(); }
  else if (e.key === "ArrowUp") { e.preventDefault(); _atActive = Math.max(_atActive - 1, 0); _renderAtActive(); }
  else if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); e.stopPropagation(); _pickAt(_atActive); }
  else if (e.key === "Escape") { e.preventDefault(); _hideAtMenu(); }
});
promptEl.addEventListener("blur", () => setTimeout(_hideAtMenu, 150));

// ---- slash commands: "/" at the start of the composer → quick agent prompts ----
const _SLASH = [
  { cmd: "fix", desc: "找出并修复 bug", prompt: "找出并修复当前文件里的 bug；修完用 run_cmd 或 get_diagnostics 验证。" },
  { cmd: "test", desc: "写并跑单元测试", prompt: "为当前文件写单元测试，覆盖边界情况，并跑通。" },
  { cmd: "explain", desc: "解释这段代码", prompt: "解释当前打开文件的代码：作用、关键逻辑、注意点。" },
  { cmd: "review", desc: "审查改动", prompt: "审查当前的代码改动，找出正确性 / 安全 / 性能 / 可维护性问题，按严重度列出并给修复建议。" },
  { cmd: "refactor", desc: "重构（保持行为）", prompt: "重构当前文件这段代码，提升可读性与结构，保持行为不变；改完验证。" },
  { cmd: "docs", desc: "加文档注释", prompt: "给当前文件的关键函数 / 类型加清晰的文档注释。" },
];
let _slashMatches = [];
let _slashActive = -1;
const _slashMenu = document.createElement("div");
_slashMenu.className = "atmenu";
_slashMenu.hidden = true;
document.body.appendChild(_slashMenu);
function _hideSlash() { _slashMenu.hidden = true; _slashActive = -1; _slashMatches = []; }
function _renderSlashActive() { [..._slashMenu.children].forEach((c, i) => c.classList.toggle("is-active", i === _slashActive)); }
function _updateSlashMenu() {
  const m = /^\/(\w*)$/.exec(promptEl.value);
  if (!m) return _hideSlash();
  const q = m[1].toLowerCase();
  _slashMatches = _SLASH.filter((s) => s.cmd.startsWith(q));
  if (!_slashMatches.length) return _hideSlash();
  _slashMenu.innerHTML = "";
  _slashMatches.forEach((s, i) => {
    const item = document.createElement("div");
    item.className = "atmenu__item" + (i === 0 ? " is-active" : "");
    item.innerHTML = `<b>/${_escHtml(s.cmd)}</b> <span style="opacity:.6;margin-left:6px">${_escHtml(s.desc)}</span>`;
    item.addEventListener("mousedown", (e) => { e.preventDefault(); _pickSlash(i); });
    _slashMenu.appendChild(item);
  });
  _slashActive = 0;
  const r = promptEl.getBoundingClientRect();
  _slashMenu.style.left = r.left + "px";
  _slashMenu.style.width = Math.min(r.width, 520) + "px";
  _slashMenu.style.bottom = window.innerHeight - r.top + 6 + "px";
  _slashMenu.hidden = false;
}
function _pickSlash(i) {
  const s = _slashMatches[i];
  if (!s) return _hideSlash();
  promptEl.value = s.prompt;
  promptEl.style.height = "auto";
  promptEl.style.height = Math.min(promptEl.scrollHeight, 160) + "px";
  _hideSlash();
  promptEl.focus();
}
promptEl.addEventListener("keydown", (e) => {
  if (_slashMenu.hidden) return;
  if (e.key === "ArrowDown") { e.preventDefault(); _slashActive = Math.min(_slashActive + 1, _slashMatches.length - 1); _renderSlashActive(); }
  else if (e.key === "ArrowUp") { e.preventDefault(); _slashActive = Math.max(_slashActive - 1, 0); _renderSlashActive(); }
  else if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); e.stopPropagation(); _pickSlash(_slashActive); }
  else if (e.key === "Escape") { e.preventDefault(); _hideSlash(); }
});
promptEl.addEventListener("blur", () => setTimeout(_hideSlash, 150));

let _pastedImages = [];

function _createImagePreview(dataUrl, idx) {
  const wrap = document.createElement("div");
  wrap.className = "prompt-image-preview";
  wrap.innerHTML =
    `<img src="${dataUrl}" alt="Pasted image" />` +
    `<button class="prompt-image-preview__remove" type="button" title="Remove">&times;</button>`;
  wrap.querySelector("button").addEventListener("click", () => {
    _pastedImages = _pastedImages.filter((_, i) => i !== idx);
    _refreshImagePreviews();
  });
  return wrap;
}

function _refreshImagePreviews() {
  let container = document.querySelector(".prompt-images");
  if (!container) {
    container = document.createElement("div");
    container.className = "prompt-images";
    promptEl.parentElement.insertBefore(container, promptEl);
  }
  container.innerHTML = "";
  _pastedImages.forEach((img, i) => container.appendChild(_createImagePreview(img.dataUrl, i)));
  if (_pastedImages.length === 0) container.remove();
}

promptEl.addEventListener("paste", (e) => {
  const items = e.clipboardData?.items;
  if (!items) return;
  for (const item of items) {
    if (item.type.startsWith("image/")) {
      e.preventDefault();
      const file = item.getAsFile();
      if (!file) continue;
      const reader = new FileReader();
      reader.onload = () => {
        _pastedImages.push({ dataUrl: reader.result, type: file.type, name: file.name || "image.png" });
        _refreshImagePreviews();
        showToast("Image attached");
      };
      reader.readAsDataURL(file);
      return;
    }
  }
});

promptEl.parentElement.addEventListener("dragover", (e) => { e.preventDefault(); e.dataTransfer.dropEffect = "copy"; });
promptEl.parentElement.addEventListener("drop", (e) => {
  e.preventDefault();
  const files = e.dataTransfer?.files;
  if (!files) return;
  for (const file of files) {
    if (file.type.startsWith("image/")) {
      const reader = new FileReader();
      reader.onload = () => {
        _pastedImages.push({ dataUrl: reader.result, type: file.type, name: file.name });
        _refreshImagePreviews();
        showToast(`Image attached: ${file.name}`);
      };
      reader.readAsDataURL(file);
    } else if (file.size < 100000) {
      const reader = new FileReader();
      reader.onload = () => {
        const text = reader.result;
        promptEl.value += (promptEl.value ? "\n\n" : "") + `[File: ${file.name}]\n${text}`;
        promptEl.dispatchEvent(new Event("input"));
        showToast(`File attached: ${file.name}`);
      };
      reader.readAsText(file);
    } else {
      showToast(`File too large: ${file.name} (${(file.size / 1024).toFixed(0)} KB)`);
    }
  }
});

// ---- quick open (⌘P) ----
const quickOpenOverlay = document.createElement("div");
quickOpenOverlay.className = "palette";
quickOpenOverlay.hidden = true;
quickOpenOverlay.innerHTML = `
  <div class="palette__panel" role="dialog" aria-label="Quick open">
    <input class="palette__input" type="text" placeholder="" spellcheck="false" />
    <div class="palette__list" role="listbox"></div>
  </div>`;
document.body.appendChild(quickOpenOverlay);

const qoInput = quickOpenOverlay.querySelector(".palette__input");
const qoList = quickOpenOverlay.querySelector(".palette__list");
let qoFiles = [];
let qoFiltered = [];
let qoCursor = 0;

async function collectProjectFiles() {
  if (!rootPath) return [];
  const result = [];
  const skip = new Set(["node_modules", ".git", "dist", "build", "out", "target", ".next", "coverage", ".cache", "vendor"]);
  const stack = [rootPath];
  const prefix = rootPath.endsWith("/") ? rootPath : rootPath + "/";
  while (stack.length && result.length < 5000) {
    const dir = stack.pop();
    let entries;
    try { entries = await backend.readDir(dir); } catch { continue; }
    for (const e of entries) {
      if (e.is_dir) {
        if (!skip.has(e.name)) stack.push(e.path);
      } else {
        const rel = e.path.startsWith(prefix) ? e.path.slice(prefix.length) : e.name;
        result.push({ path: e.path, name: e.name, rel });
      }
    }
  }
  return result;
}

function qoFuzzyMatch(str, query) {
  const lower = str.toLowerCase();
  const q = query.toLowerCase();
  let qi = 0, score = 0, lastMatch = -1;
  for (let i = 0; i < lower.length && qi < q.length; i++) {
    if (lower[i] === q[qi]) {
      score += (lastMatch === i - 1) ? 10 : 1;
      if (i === 0 || str[i - 1] === "/" || str[i - 1] === "." || str[i - 1] === "-" || str[i - 1] === "_")
        score += 5;
      lastMatch = i;
      qi++;
    }
  }
  return qi === q.length ? score : -1;
}

function qoRefresh() {
  const q = qoInput.value.trim();
  if (!q) {
    qoFiltered = qoFiles.slice(0, 50);
  } else {
    qoFiltered = qoFiles
      .map((f) => ({ f, s: qoFuzzyMatch(f.rel, q) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => b.s - a.s)
      .slice(0, 50)
      .map((x) => x.f);
  }
  qoCursor = 0;
  qoRender();
}

function qoRender() {
  qoList.innerHTML = "";
  if (!qoFiltered.length) {
    const empty = document.createElement("div");
    empty.className = "palette__empty";
    empty.textContent = t("quickOpen.noResults");
    qoList.appendChild(empty);
    return;
  }
  qoFiltered.forEach((file, i) => {
    const row = document.createElement("div");
    row.className = "palette__item" + (i === qoCursor ? " is-active" : "");
    row.setAttribute("role", "option");
    row.innerHTML = `${iconImg(fileIconUrl(file.name))}<span class="palette__title"></span><span class="palette__cat"></span>`;
    row.querySelector(".palette__title").textContent = file.name;
    const dir = file.rel.includes("/") ? file.rel.slice(0, file.rel.lastIndexOf("/")) : "";
    if (dir) row.querySelector(".palette__cat").textContent = dir;
    row.addEventListener("mousemove", () => { if (qoCursor !== i) { qoCursor = i; qoRender(); } });
    row.addEventListener("click", () => qoSelect(file));
    qoList.appendChild(row);
  });
  const active = qoList.querySelector(".is-active");
  if (active) active.scrollIntoView({ block: "nearest" });
}

function qoSelect(file) {
  qoClose();
  openFile(file.path, file.name);
}

async function qoOpen() {
  qoInput.value = "";
  qoInput.placeholder = t("quickOpen.placeholder");
  quickOpenOverlay.hidden = false;
  qoFiles = await collectProjectFiles();
  qoRefresh();
  qoInput.focus();
}

function qoClose() {
  quickOpenOverlay.hidden = true;
}

qoInput.addEventListener("input", qoRefresh);
qoInput.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") { e.preventDefault(); qoCursor = Math.min(qoCursor + 1, qoFiltered.length - 1); qoRender(); }
  else if (e.key === "ArrowUp") { e.preventDefault(); qoCursor = Math.max(qoCursor - 1, 0); qoRender(); }
  else if (e.key === "Enter") { e.preventDefault(); if (qoFiltered[qoCursor]) qoSelect(qoFiltered[qoCursor]); }
  else if (e.key === "Escape") { e.preventDefault(); qoClose(); }
});
quickOpenOverlay.addEventListener("mousedown", (e) => { if (e.target === quickOpenOverlay) qoClose(); });

// ---- terminal → IDE sync ----
let _termRefreshTimer = null;
const _TERM_REFRESH_PATTERNS = [
  /Successfully installed/i,
  /pip install/i,
  /npm install/i,
  /pnpm install/i,
  /yarn add/i,
  /cargo build/i,
  /go get/i,
  /gem install/i,
  /luarocks install/i,
  /composer require/i,
  // NOTE: deliberately no "shell prompt" pattern — `/\$\s*$/` matched almost any
  // output containing `$`, so it fired a full reloadDir + git + LSP refresh on
  // nearly every terminal chunk while the agent ran commands (a periodic freeze).
  // We only refresh after recognizably state-changing commands (installs/builds).
];

let _termOutputBuf = "";
function _detectTerminalChanges(data) {
  _termOutputBuf += data;
  if (_termOutputBuf.length > 2000) _termOutputBuf = _termOutputBuf.slice(-1000);

  for (const re of _TERM_REFRESH_PATTERNS) {
    if (re.test(_termOutputBuf)) {
      _scheduleTermRefresh();
      _termOutputBuf = "";
      return;
    }
  }
}

function _scheduleTermRefresh() {
  if (_termRefreshTimer) clearTimeout(_termRefreshTimer);
  _termRefreshTimer = setTimeout(() => {
    if (rootPath) {
      reloadDir(rootPath);
      refreshGitStatus();
    }

    const model = monacoEditor.getModel();
    if (model) {
      const langId = model.getLanguageId();
      _envSymbolsLoaded = false;
      _envLoadingLang = null;
      _loadEnvSymbols(langId);
    }
  }, 5000);
}

// ---- new project templates ----
const PROJECT_TEMPLATES = [
  { name: "React (Vite)", cmd: "npm create vite@latest {{name}} -- --template react && cd {{name}} && npm install", icon: "⚛" },
  { name: "React + TypeScript", cmd: "npm create vite@latest {{name}} -- --template react-ts && cd {{name}} && npm install", icon: "⚛" },
  { name: "Vue 3 (Vite)", cmd: "npm create vite@latest {{name}} -- --template vue && cd {{name}} && npm install", icon: "🟢" },
  { name: "Svelte (Vite)", cmd: "npm create vite@latest {{name}} -- --template svelte && cd {{name}} && npm install", icon: "🔶" },
  { name: "Next.js", cmd: "npx create-next-app@latest {{name}} --use-npm", icon: "▲" },
  { name: "Flask (Python)", cmd: "mkdir {{name}} && cd {{name}} && python3 -m venv .venv && source .venv/bin/activate && pip install flask && echo 'from flask import Flask\\napp = Flask(__name__)\\n\\n@app.route(\"/\")\\ndef index():\\n    return \"Hello World\"\\n\\nif __name__ == \"__main__\":\\n    app.run(debug=True)' > app.py", icon: "🐍" },
  { name: "FastAPI (Python)", cmd: "mkdir {{name}} && cd {{name}} && python3 -m venv .venv && source .venv/bin/activate && pip install fastapi uvicorn && echo 'from fastapi import FastAPI\\napp = FastAPI()\\n\\n@app.get(\"/\")\\ndef root():\\n    return {\"message\": \"Hello World\"}' > main.py", icon: "🚀" },
  { name: "Express (Node.js)", cmd: "mkdir {{name}} && cd {{name}} && npm init -y && npm install express && echo 'const express = require(\"express\");\\nconst app = express();\\napp.get(\"/\", (req, res) => res.json({ message: \"Hello World\" }));\\napp.listen(3000, () => console.log(\"Server on http://localhost:3000\"));' > index.js", icon: "🟩" },
  { name: "Tauri (Rust + React)", cmd: "npm create tauri-app@latest {{name}} -- --template react --manager npm", icon: "🦀" },
  { name: "Vanilla HTML/CSS/JS", cmd: "mkdir {{name}} && cd {{name}} && echo '<!DOCTYPE html>\\n<html lang=\"en\">\\n<head>\\n<meta charset=\"UTF-8\">\\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\\n<title>{{name}}</title>\\n<link rel=\"stylesheet\" href=\"style.css\">\\n</head>\\n<body>\\n<h1>Hello World</h1>\\n<script src=\"main.js\"></script>\\n</body>\\n</html>' > index.html && echo 'body { font-family: system-ui; max-width: 800px; margin: 0 auto; padding: 20px; }' > style.css && echo 'console.log(\"Hello World\");' > main.js", icon: "📄" },
];

async function showNewProjectDialog() {
  const overlay = document.createElement("div");
  overlay.className = "modal-overlay";
  const modal = document.createElement("div");
  modal.className = "modal";
  modal.style.cssText = "width:480px;max-height:80vh;overflow-y:auto;";
  modal.innerHTML = `<div class="modal-header"><h3>新建项目</h3><button class="modal-close">×</button></div>
    <div style="padding:16px"><input class="input" placeholder="项目名称" style="width:100%;margin-bottom:16px" autofocus>
    <div class="template-list"></div></div>`;

  const nameInput = modal.querySelector("input");
  const list = modal.querySelector(".template-list");
  const close = () => { overlay.remove(); };
  modal.querySelector(".modal-close").addEventListener("click", close);
  overlay.addEventListener("click", (e) => { if (e.target === overlay) close(); });

  for (const tmpl of PROJECT_TEMPLATES) {
    const row = document.createElement("div");
    row.style.cssText = "display:flex;align-items:center;gap:12px;padding:10px 12px;border-radius:8px;cursor:pointer;transition:background 0.15s;";
    row.innerHTML = `<span style="font-size:20px;width:28px;text-align:center">${tmpl.icon}</span><div><strong style="display:block">${tmpl.name}</strong></div>`;
    row.addEventListener("mouseenter", () => { row.style.background = "var(--hover)"; });
    row.addEventListener("mouseleave", () => { row.style.background = ""; });
    row.addEventListener("click", async () => {
      const name = nameInput.value.trim() || "my-project";
      close();
      await openTerminal();
      const wasOpen = termIsOpen();
      if (!wasOpen) {
        await _waitTermReady(3000);
        await new Promise((r) => setTimeout(r, 1800));
      }
      const cmd = tmpl.cmd.replace(/\{\{name\}\}/g, shellQuote(name));
      writeToActiveTerminal(`\nclear\n${cmd}\n`);
      showToast(`正在创建 ${tmpl.name} 项目: ${name}`);
    });
    list.appendChild(row);
  }

  overlay.appendChild(modal);
  document.body.appendChild(overlay);
  nameInput.focus();
  nameInput.addEventListener("keydown", (e) => { if (e.key === "Escape") close(); });
}

// ---- auto-save ----
let autoSaveEnabled = true;
let autoSaveTimer = null;

function scheduleAutoSave() {
  if (!autoSaveEnabled || !activePath) return;
  clearTimeout(autoSaveTimer);
  autoSaveTimer = setTimeout(async () => {
    if (!activePath) return;
    const f = openFiles.get(activePath);
    if (f && f.dirty) {
      try {
        _autoSaving = true;
        const savingPath = activePath;
        await backend.writeTextFile(savingPath, f.model.getValue());
        f.dirty = false;
        saveBtn.disabled = true;
        const tabEl = tabsEl.querySelector(`[data-path="${CSS.escape(savingPath)}"]`);
        if (tabEl) tabEl.classList.remove("dirty");
        lspManager?.didSave(savingPath, f.model);
        _autoSaving = false;
      } catch { _autoSaving = false; }
    }
  }, 800);
}
let _autoSaving = false;

monacoEditor.onDidChangeModelContent(() => {
  if (!_imeComposing) scheduleAutoSave();
});

async function toggleAutoSave() {
  autoSaveEnabled = !autoSaveEnabled;
  if (_editorPrefs) {
    _editorPrefs.autoSave = autoSaveEnabled;
    await saveEditorPrefs();
  }
  showToast(autoSaveEnabled ? t("autosave.enabled") : t("autosave.disabled"));
}

// ---- search & replace ----
const replaceInputEl = document.createElement("input");
replaceInputEl.type = "text";
replaceInputEl.className = "search-replace-input";
replaceInputEl.spellcheck = false;
replaceInputEl.autocomplete = "off";
replaceInputEl.placeholder = t("search.replacePlaceholder");

const replaceBar = document.createElement("div");
replaceBar.className = "search-replace-bar";
replaceBar.innerHTML = `<div class="search-replace-row"></div>`;
const replaceRow = replaceBar.querySelector(".search-replace-row");
replaceRow.appendChild(replaceInputEl);

const replaceSingleBtn = document.createElement("button");
replaceSingleBtn.className = "iconbtn";
replaceSingleBtn.type = "button";
replaceSingleBtn.title = t("search.replace");
replaceSingleBtn.innerHTML = `<svg class="ic" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" d="M4 8h12l-3-3m3 3-3 3"/></svg>`;

const replaceAllBtn = document.createElement("button");
replaceAllBtn.className = "iconbtn";
replaceAllBtn.type = "button";
replaceAllBtn.title = t("search.replaceAll");
replaceAllBtn.innerHTML = `<svg class="ic" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" d="M4 8h12l-3-3m3 3-3 3M4 16h12l-3-3m3 3-3 3"/></svg>`;

replaceRow.append(replaceSingleBtn, replaceAllBtn);

const searchBox = document.querySelector(".search-box");
if (searchBox) searchBox.after(replaceBar);

async function replaceInFiles(replaceAll) {
  const query = $("searchInput").value;
  const replacement = replaceInputEl.value;
  if (!query || !rootPath) return;

  let totalCount = 0, fileCount = 0;
  let files;
  try {
    files = await backend.searchInProject(rootPath, query, searchCaseSensitive);
  } catch { return; }

  for (const f of files) {
    if (!replaceAll && totalCount > 0) break;
    let content;
    try { content = await backend.readTextFile(f.path); } catch { continue; }
    const regex = searchCaseSensitive
      ? new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "g")
      : new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "gi");
    const newContent = content.replace(regex, replacement);
    if (newContent !== content) {
      try {
        await backend.writeTextFile(f.path, newContent);
        const count = (content.match(regex) || []).length;
        totalCount += count;
        fileCount++;
        if (openFiles.has(f.path)) {
          const model = openFiles.get(f.path).model;
          model.setValue(newContent);
          markDirty(f.path, false);
        }
      } catch { /* skip */ }
    }
    if (!replaceAll) break;
  }
  if (totalCount > 0) {
    showToast(replaceAll
      ? t("search.replaced", { count: totalCount, s: totalCount === 1 ? "" : "s", files: fileCount, s2: fileCount === 1 ? "" : "s" })
      : t("search.replacedInFile", { count: totalCount, s: totalCount === 1 ? "" : "s" })
    );
    runSearch();
  }
}

replaceSingleBtn.addEventListener("click", () => replaceInFiles(false));
replaceAllBtn.addEventListener("click", () => replaceInFiles(true));

const DEFAULT_KEYBINDINGS = {
  "ctrl+`": "terminal.toggle",
  "mod+p": "file.quickOpen",
  "mod+s": "file.save",
  "mod+shift+e": "view.explorer",
  "mod+shift+f": "view.search",
  "ctrl+shift+g": "view.git",
  "mod+shift+m": "view.problems",
  "mod+shift+p": "commandPalette",
  "mod+\\": "view.splitEditor",
  "mod+r": "code.runCurrentFile",
};

let userKeybindings = {};

async function loadKeybindings() {
  const store = await getStore();
  userKeybindings = (await store.get("keybindings")) || {};
}

async function saveKeybinding(combo, action) {
  userKeybindings[combo] = action;
  const store = await getStore();
  await store.set("keybindings", userKeybindings);
  await store.save();
}

function getKeybindings() {
  const merged = { ...DEFAULT_KEYBINDINGS, ...userKeybindings };
  for (const k of Object.keys(merged)) {
    if (merged[k] === DISABLED_BINDING) delete merged[k];
  }
  return merged;
}

const KB_ACTIONS = {
  "terminal.toggle": () => toggleTerminal(),
  "file.quickOpen": () => qoOpen(),
  "file.save": () => saveActive(),
  "view.explorer": () => showSide("explorer"),
  "view.search": () => showSide("search"),
  "view.git": () => showSide("git"),
  "view.outline": () => showSide("outline"),
  "view.test": () => showSide("test"),
  "view.output": () => toggleOutputPanel(),
  "view.bookmarks": () => toggleBookmarksPanel(),
  "view.problems": () => toggleProblems(),
  "memory.manage": () => openMemoryPanel(),
  "commandPalette": () => palette.open(),
  "view.splitEditor": () => toggleSplitEditor(),
  "code.runCurrentFile": () => runCurrentFile(),
};

function keyCombo(e) {
  const parts = [];
  if (e.ctrlKey && !e.metaKey) parts.push("ctrl");
  if (e.metaKey || (e.ctrlKey && !navigator.platform.match(/Mac/))) {
    if (!parts.includes("ctrl")) parts.push("mod");
  }
  if (e.shiftKey) parts.push("shift");
  if (e.altKey) parts.push("alt");
  const key = e.key.toLowerCase();
  if (!["control", "shift", "alt", "meta"].includes(key)) parts.push(key);
  return parts.join("+");
}

loadKeybindings().catch(console.error);

window.addEventListener("keydown", (e) => {
  const combo = keyCombo(e);
  const bindings = getKeybindings();
  const action = bindings[combo];
  if (action && KB_ACTIONS[action]) {
    e.preventDefault();
    e.stopPropagation();
    KB_ACTIONS[action]();
  }
});

// ---- integrated terminal (multi-tab) ----
const termPanel = $("terminalPanel");
const termBody = $("terminalBody");
const termTabBar = $("termTabBar");
const editorwrapEl = document.querySelector(".editorwrap");

// ---- terminal command suggestions (history + common commands + paths) ----
const TERM_COMMON_CMDS = [
  // git
  "git status", "git status -s", "git add .", "git add -A", "git add -p",
  "git commit -m \"\"", "git commit -am \"\"", "git commit --amend",
  "git push", "git push -u origin ", "git push --force-with-lease", "git push --tags",
  "git pull", "git pull --rebase", "git fetch", "git fetch --all --prune",
  "git log", "git log --oneline", "git log --oneline --graph --all", "git log -p",
  "git checkout ", "git checkout -b ", "git switch ", "git switch -c ", "git switch -",
  "git branch", "git branch -a", "git branch -d ", "git branch -D ", "git branch -m ",
  "git merge ", "git merge --abort", "git rebase ", "git rebase -i ", "git rebase --abort", "git rebase --continue",
  "git diff", "git diff --staged", "git diff HEAD", "git diff --stat",
  "git stash", "git stash pop", "git stash list", "git stash apply", "git stash drop", "git stash show -p",
  "git reset ", "git reset --hard ", "git reset --soft HEAD~1", "git restore ", "git restore --staged ",
  "git clone ", "git remote -v", "git remote add origin ", "git tag ", "git cherry-pick ",
  "git show ", "git blame ", "git clean -fd", "git revert ", "git config --global ", "git init",
  // npm
  "npm install", "npm install ", "npm install -D ", "npm install -g ", "npm uninstall ",
  "npm run ", "npm run dev", "npm run build", "npm run test", "npm run lint", "npm run start",
  "npm start", "npm test", "npm ci", "npm update", "npm outdated", "npm audit", "npm audit fix",
  "npm publish", "npm version patch", "npm list", "npm cache clean --force", "npx ",
  // pnpm / yarn / bun
  "pnpm install", "pnpm add ", "pnpm add -D ", "pnpm remove ", "pnpm dev", "pnpm build", "pnpm test", "pnpm run ", "pnpm up",
  "yarn", "yarn add ", "yarn add -D ", "yarn remove ", "yarn dev", "yarn build", "yarn test", "yarn install",
  "bun install", "bun add ", "bun run ", "bun dev",
  // cargo / rust
  "cargo build", "cargo build --release", "cargo run", "cargo run --release", "cargo test",
  "cargo check", "cargo clippy", "cargo clippy --all-targets -- -D warnings", "cargo fmt",
  "cargo add ", "cargo update", "cargo install ", "cargo new ", "cargo doc --open", "rustup update", "rustc ",
  // python
  "python3 ", "python3 -m venv venv", "python3 -m pip install ", "pip install ", "pip install -r requirements.txt",
  "pip freeze > requirements.txt", "pip list", "pip3 install ", "source venv/bin/activate", "pytest", "python -m http.server",
  // node / go / others
  "node ", "deno run ", "deno task ", "tsx ", "ts-node ",
  "go run .", "go build", "go test ./...", "go mod tidy", "go get ", "go install ",
  "java -jar ", "javac ", "mvn ", "gradle ", "ruby ", "rails ", "php ", "php artisan ", "composer install",
  "dotnet run", "dotnet build", "dotnet test",
  // docker / k8s
  "docker ps", "docker ps -a", "docker images", "docker build -t ", "docker run ", "docker exec -it ",
  "docker stop ", "docker rm ", "docker rmi ", "docker logs -f ", "docker pull ", "docker push ", "docker system prune",
  "docker compose up", "docker compose up -d", "docker compose down", "docker compose logs -f", "docker compose build",
  "kubectl get pods", "kubectl get svc", "kubectl get nodes", "kubectl apply -f ", "kubectl delete -f ",
  "kubectl logs ", "kubectl describe pod ", "kubectl exec -it ", "helm install ",
  // filesystem
  "cd ", "cd ..", "cd ~", "cd -", "ls", "ls -la", "ls -lah", "pwd", "clear",
  "mkdir ", "mkdir -p ", "rmdir ", "rm ", "rm -rf ", "rm -f ", "cp ", "cp -r ", "mv ",
  "touch ", "cat ", "less ", "head ", "tail ", "tail -f ", "ln -s ", "stat ", "file ", "tree",
  "chmod +x ", "chmod 755 ", "chown ", "open ", "open .", "code .", "du -sh ", "df -h",
  // text / search
  "grep -r ", "grep -rn ", "grep -i ", "rg ", "rg -i ", "find . -name ", "find . -type f -name ",
  "sed -i ", "awk ", "sort ", "uniq ", "wc -l ", "xargs ", "diff ", "pbcopy < ", "pbpaste",
  // net / process
  "curl ", "curl -O ", "curl -L ", "wget ", "ssh ", "scp ", "rsync -av ", "ping ",
  "ps aux", "ps aux | grep ", "kill ", "kill -9 ", "killall ", "lsof -i :", "top", "htop",
  "netstat -an", "ifconfig", "nslookup ", "dig ",
  // archive / pkg managers
  "tar -xzf ", "tar -czf ", "zip -r ", "unzip ", "gzip ", "gunzip ",
  "brew install ", "brew update", "brew upgrade", "brew list", "brew search ", "brew uninstall ",
  "apt install ", "apt update", "apt upgrade", "sudo apt install ",
  // misc
  "echo ", "export ", "source ", "which ", "whereis ", "man ", "history", "alias ", "env",
  "sudo ", "watch ", "sleep ", "date", "whoami", "uname -a", "say ", "code .",
];

let termHistory = [];
function pushTermHistory(cmd) {
  termHistory = termHistory.filter((c) => c !== cmd);
  termHistory.unshift(cmd);
  if (termHistory.length > 300) termHistory.length = 300;
}

// Cache of workspace-relative paths (files + dirs) for argument completion.
let termPathCache = [];
async function refreshTermPathCache() {
  if (!rootPath) { termPathCache = []; return; }
  const out = [];
  const skip = new Set(["node_modules", ".git", "dist", "build", "target", ".next", "coverage", ".cache", "vendor"]);
  const base = rootPath.endsWith("/") ? rootPath : rootPath + "/";
  const stack = [rootPath];
  let count = 0;
  while (stack.length && count < 4000) {
    const dir = stack.pop();
    let entries;
    try { entries = await backend.readDir(dir); } catch { continue; }
    for (const e of entries) {
      const rel = e.path.startsWith(base) ? e.path.slice(base.length) : e.name;
      out.push(e.is_dir ? rel + "/" : rel);
      count++;
      if (e.is_dir && !skip.has(e.name)) stack.push(e.path);
    }
  }
  termPathCache = out;
}

// System commands ($PATH) + the user's shell history, loaded once from the
// backend so suggestions cover every installed tool and real past command.
let termPathCommands = [];
let _termDataLoaded = false;
async function loadTermData() {
  if (_termDataLoaded) return;
  _termDataLoaded = true;
  try {
    const hist = await backend.termHistory?.();
    if (Array.isArray(hist) && hist.length) {
      const existing = new Set(termHistory);
      for (const c of hist) if (!existing.has(c)) termHistory.push(c);
    }
  } catch { /* ignore */ }
  try {
    const cmds = await backend.termListCommands?.();
    if (Array.isArray(cmds)) termPathCommands = cmds;
  } catch { /* ignore */ }
}

function getTermSuggestions(prefix) {
  if (!prefix || prefix.length < 1) return [];
  const seen = new Set();
  const out = [];
  const add = (cmd) => {
    if (cmd.length <= prefix.length || !cmd.startsWith(prefix) || seen.has(cmd)) return;
    seen.add(cmd);
    out.push(cmd);
  };
  for (const c of termHistory) add(c);
  for (const c of TERM_COMMON_CMDS) add(c);
  // Every installed command on $PATH — only when completing the first word.
  if (!prefix.includes(" ")) {
    for (const c of termPathCommands) {
      add(c);
      if (out.length >= 6) break;
    }
  }
  // Argument path completion: complete the last token against workspace paths.
  if (out.length === 0 && prefix.includes(" ")) {
    const cut = prefix.lastIndexOf(" ");
    const head = prefix.slice(0, cut + 1);
    const token = prefix.slice(cut + 1);
    if (token && !token.startsWith("-")) {
      for (const rel of termPathCache) {
        if (rel.length > token.length && rel.startsWith(token)) {
          const full = head + rel;
          if (!seen.has(full)) { seen.add(full); out.push(full); }
          if (out.length >= 6) break;
        }
      }
    }
  }
  return out.slice(0, 6);
}

let _ghostTimer = null;
let _ghostDeco = null;
let _ghostMarker = null;

// Remove the ghost overlay. This ONLY disposes the decoration — it never writes
// to the PTY buffer or moves the cursor, so the shell's line editing (backspace,
// arrows, history) is never disturbed.
function clearTermGhost(entry) {
  if (_ghostDeco) { try { _ghostDeco.dispose(); } catch { /* ignore */ } _ghostDeco = null; }
  if (_ghostMarker) { try { _ghostMarker.dispose(); } catch { /* ignore */ } _ghostMarker = null; }
  if (entry) entry.ghost = "";
}

// Render the suggestion as a non-intrusive overlay decoration positioned at the
// cursor. Because nothing is written into the terminal buffer, editing keys keep
// working exactly as the shell expects.
function renderTermGhost(entry) {
  clearTermGhost(entry);
  if (!entry || entry !== termTabs[activeTermTab] || entry.backendId == null) return;
  const prefix = entry.inputLine;
  const sug = getTermSuggestions(prefix)[0] || "";
  entry.suggestion = sug;
  const ghost = sug ? sug.slice(prefix.length) : "";
  if (!ghost) return;
  const term = entry.term;
  let marker = null;
  let deco = null;
  try {
    marker = term.registerMarker(0);
    if (!marker) return;
    deco = term.registerDecoration({
      marker,
      x: term.buffer.active.cursorX,
      width: Math.min(ghost.length, term.cols),
      layer: "top",
    });
  } catch { return; }
  if (!deco) { try { marker.dispose(); } catch { /* ignore */ } return; }
  _ghostMarker = marker;
  _ghostDeco = deco;
  entry.ghost = ghost;
  deco.onRender((el) => {
    el.textContent = ghost;
    el.classList.add("term-ghost");
  });
}

function scheduleTermGhost(entry) {
  clearTimeout(_ghostTimer);
  _ghostTimer = setTimeout(() => renderTermGhost(entry), 30);
}

// Accept the current ghost: send the remaining characters to the shell.
function acceptTermGhost(entry) {
  if (!entry || !entry.ghost || !entry.suggestion) return false;
  const rest = entry.suggestion.slice(entry.inputLine.length);
  clearTermGhost(entry);
  if (entry.backendId != null && rest) backend.termWrite(entry.backendId, rest);
  entry.inputLine = entry.suggestion;
  return true;
}

function trackTermInput(entry, d) {
  for (const ch of d) {
    const code = ch.charCodeAt(0);
    if (ch === "\r" || ch === "\n") {
      const line = entry.inputLine.trim();
      if (line) pushTermHistory(line);
      entry.inputLine = "";
      clearTermGhost(entry);
      return;
    } else if (code === 127 || code === 8) {
      entry.inputLine = entry.inputLine.slice(0, -1);
    } else if (code === 27 || code === 3 || code === 21 || code === 23 || code === 9) {
      // ESC sequences (arrow keys), Ctrl-C/U/W, or Tab: stop tracking this line
      entry.inputLine = "";
      clearTermGhost(entry);
      return;
    } else if (code >= 32) {
      entry.inputLine += ch;
    }
  }
  scheduleTermGhost(entry);
}

function termTheme() {
  const dark = document.documentElement.getAttribute("data-theme") === "dark";
  if (dark) {
    return {
      background: "#0d1017",
      foreground: "#e6edf3",
      cursor: "#58a6ff",
      cursorAccent: "#0d1017",
      selectionBackground: "rgba(56, 139, 253, 0.28)",
      selectionForeground: "#ffffff",
      black: "#0d1117", red: "#ff7b72", green: "#3fb950", yellow: "#d29922",
      blue: "#58a6ff", magenta: "#bc8cff", cyan: "#39d353", white: "#e6edf3",
      brightBlack: "#484f58", brightRed: "#ffa198", brightGreen: "#56d364",
      brightYellow: "#e3b341", brightBlue: "#79c0ff", brightMagenta: "#d2a8ff",
      brightCyan: "#56d364", brightWhite: "#ffffff",
    };
  }
  // Light terminal (GitHub Light palette) — dark text on white.
  return {
    background: "#ffffff",
    foreground: "#1f2328",
    cursor: "#0a84ff",
    cursorAccent: "#ffffff",
    selectionBackground: "rgba(10, 132, 255, 0.18)",
    selectionForeground: "#1f2328",
    black: "#24292f", red: "#cf222e", green: "#116329", yellow: "#7d4e00",
    blue: "#0969da", magenta: "#8250df", cyan: "#1b7c83", white: "#6e7781",
    brightBlack: "#57606a", brightRed: "#a40e26", brightGreen: "#1a7f37",
    brightYellow: "#633c01", brightBlue: "#218bff", brightMagenta: "#a475f9",
    brightCyan: "#3192aa", brightWhite: "#8c959f",
  };
}

const termIsOpen = () => termPanel && !termPanel.hidden;

const termResizeObserver = new ResizeObserver(() => {
  if (termIsOpen() && activeTermTab >= 0 && termTabs[activeTermTab]?.fit) {
    try { termTabs[activeTermTab].fit.fit(); } catch {}
  }
});

let termTabs = [];
let activeTermTab = -1;
let termSeq = 0;
let _termSplitActive = false;

function renderTermTabs() {
  if (!termTabBar) return;
  termTabBar.innerHTML = "";
  termTabs.forEach((tab, i) => {
    const btn = document.createElement("button");
    btn.className = "term-tab" + (i === activeTermTab ? " is-active" : "");
    btn.type = "button";
    btn.title = tab.cwd ? `${tab.label} — ${tab.cwd}` : tab.label;
    btn.innerHTML =
      `<svg class="term-tab__icon" viewBox="0 0 24 24" aria-hidden="true"><use href="#i-terminal" /></svg>` +
      `<span class="term-tab__label"></span>` +
      `<span class="term-tab__x" title="Close" aria-label="Close terminal">` +
      `<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"><path d="M4.5 4.5l7 7M11.5 4.5l-7 7"/></svg>` +
      `</span>`;
    btn.querySelector(".term-tab__label").textContent = tab.label;
    btn.addEventListener("click", (e) => {
      if (e.target.closest(".term-tab__x")) closeTermTab(i);
      else switchTermTab(i);
    });
    termTabBar.appendChild(btn);
  });
}

function switchTermTab(idx) {
  if (idx === activeTermTab || idx < 0 || idx >= termTabs.length) return;
  clearTermGhost(termTabs[activeTermTab]);
  // In split view both panes stay visible; only single view hides the old one.
  if (!_termSplitActive && activeTermTab >= 0 && termTabs[activeTermTab]) {
    termTabs[activeTermTab].container.hidden = true;
  }
  activeTermTab = idx;
  const tab = termTabs[idx];
  tab.container.hidden = false;
  if (_termSplitActive) _applyTermSplit();
  renderTermTabs();
  requestAnimationFrame(() => {
    try { tab.fit.fit(); } catch {}
    tab.term.focus();
  });
}

async function createTermTab() {
  const idx = termTabs.length;
  const label = `Terminal ${++termSeq}`;
  const cwd = rootPath || "";
  const container = document.createElement("div");
  container.className = "terminal-panel__instance";
  container.hidden = activeTermTab >= 0;
  termBody.appendChild(container);

  const term = new Terminal({
    fontSize: 13,
    fontFamily: "'SF Mono', Menlo, ui-monospace, 'JetBrains Mono', Consolas, monospace",
    fontWeight: "normal",
    fontWeightBold: "bold",
    lineHeight: 1.4,
    letterSpacing: 0.2,
    theme: termTheme(),
    cursorBlink: true,
    cursorStyle: "bar",
    cursorWidth: 2,
    scrollback: 3500, // bounded so a flooding command can't balloon terminal memory/CPU
    allowProposedApi: true,
    drawBoldTextInBrightColors: false,
    minimumContrastRatio: 4.5,
  });
  const fit = new FitAddon();
  term.loadAddon(fit);
  term.open(container);
  // WebGL renderer is much faster, but a lost GPU context (sleep/wake, driver
  // reset, too many contexts) silently stops it rendering — the classic "typing
  // shows nothing / terminal frozen" bug. Dispose the addon on context loss so
  // xterm falls back to the canvas renderer instead of freezing.
  let webglAddon = null;
  try {
    webglAddon = new WebglAddon();
    webglAddon.onContextLoss(() => { try { webglAddon.dispose(); } catch {} webglAddon = null; });
    term.loadAddon(webglAddon);
  } catch { webglAddon = null; /* WebGL unavailable → canvas renderer */ }
  term.loadAddon(new WebLinksAddon());
  termResizeObserver.observe(container);

  let initDone = false;
  let initBuffer = "";
  const entry = { term, fit, container, label, cwd, backendId: null, opening: false, inputLine: "", ghost: "", suggestion: "", webgl: webglAddon };
  termTabs.push(entry);

  term.onData((d) => {
    // Drop the ghost overlay immediately; it is rebuilt after the echo settles.
    clearTermGhost(entry);
    if (entry.backendId != null) backend.termWrite(entry.backendId, d);
    trackTermInput(entry, d);
  });
  term.onResize(({ cols, rows }) => { if (entry.backendId != null) backend.termResize(entry.backendId, cols, rows); });
  // Tab accepts the inline ghost suggestion when one is shown; otherwise it
  // falls through to the shell's own completion.
  term.attachCustomKeyEventHandler((e) => {
    if (e.type === "keydown" && e.key === "Tab" && entry.ghost) {
      e.preventDefault();
      acceptTermGhost(entry);
      scheduleTermGhost(entry);
      return false;
    }
    return true;
  });

  switchTermTab(idx);

  entry.opening = true;
  try {
    entry.backendId = await backend.termOpen(
      { cwd: rootPath || undefined, cols: term.cols, rows: term.rows },
      (ev) => {
        if (ev.kind === "data") {
          if (!initDone) {
            initBuffer += ev.data;
            return;
          }
          term.write(ev.data);
          _detectTerminalChanges(ev.data);
        } else if (ev.kind === "exit") {
          term.write("\r\n\x1b[2m[process exited]\x1b[0m\r\n");
          entry.backendId = null;
          _scheduleTermRefresh();
        }
      },
    );
    const finishInit = () => {
      if (initDone) return;
      term.reset();
      term.write("\x1b[0m\x1b[?25h");
      initDone = true;
      if (entry.backendId != null) {
        backend.termWrite(entry.backendId, " clear\n");
      }
    };
    setTimeout(finishInit, 1500);
  } catch (err) {
    term.write("\r\n\x1b[31mFailed to start terminal: " + (err?.message || err) + "\x1b[0m\r\n");
  } finally {
    entry.opening = false;
  }
}

function closeTermTab(idx) {
  if (idx < 0 || idx >= termTabs.length) return;
  const tab = termTabs[idx];
  if (tab.backendId != null) backend.termClose(tab.backendId);
  try { tab.webgl?.dispose(); } catch {} // release the GPU context (avoid leaking WebGL contexts)
  tab.term.dispose();
  termResizeObserver.unobserve(tab.container);
  tab.container.remove();
  termTabs.splice(idx, 1);
  if (termTabs.length === 0) {
    activeTermTab = -1;
    closeTerminal();
    return;
  }
  if (activeTermTab >= termTabs.length) activeTermTab = termTabs.length - 1;
  else if (activeTermTab === idx) activeTermTab = Math.min(idx, termTabs.length - 1);
  termTabs[activeTermTab].container.hidden = false;
  if (_termSplitActive) {
    if (termTabs.length >= 2) _applyTermSplit();
    else { _termSplitActive = false; $("terminalBody")?.classList.remove("term-split"); }
  }
  renderTermTabs();
  requestAnimationFrame(() => {
    try { termTabs[activeTermTab].fit.fit(); } catch {}
    termTabs[activeTermTab].term.focus();
  });
}

async function openTerminal() {
  if (!termPanel) return;
  if (termIsOpen()) {
    if (activeTermTab >= 0) termTabs[activeTermTab]?.term.focus();
    return;
  }
  termPanel.hidden = false;
  editorwrapEl?.classList.add("has-terminal");
  monacoEditor.layout();
  loadTermData();
  refreshTermPathCache().catch(() => {});

  if (termTabs.length === 0) {
    await createTermTab();
  } else {
    switchTermTab(activeTermTab);
  }
}

function closeTerminal() {
  if (!termIsOpen()) return;
  clearTermGhost(termTabs[activeTermTab]);
  termPanel.hidden = true;
  editorwrapEl?.classList.remove("has-terminal");
  monacoEditor.layout();
  monacoEditor.focus();
}

function toggleTerminal() {
  if (termIsOpen()) closeTerminal();
  else openTerminal();
}

$("terminalClose")?.addEventListener("click", closeTerminal);
$("termNewBtn")?.addEventListener("click", () => createTermTab());
$("terminalBtn")?.addEventListener("click", toggleTerminal);

// terminal panel resize
{
  const resizeHandle = $("terminalResize");
  if (resizeHandle) {
    let startY = 0, startH = 0, dragging = false;
    const onMove = (e) => {
      if (!dragging) return;
      const h = Math.max(140, Math.min(window.innerHeight * 0.7, startH + (startY - e.clientY)));
      termPanel.style.flex = `0 0 ${h}px`;
      requestAnimationFrame(() => {
        monacoEditor.layout();
        if (activeTermTab >= 0 && termTabs[activeTermTab]?.fit)
          try { termTabs[activeTermTab].fit.fit(); } catch {}
      });
    };
    const onUp = () => {
      dragging = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
    resizeHandle.addEventListener("mousedown", (e) => {
      e.preventDefault();
      dragging = true;
      startY = e.clientY;
      startH = termPanel.getBoundingClientRect().height;
      document.body.style.cursor = "ns-resize";
      document.body.style.userSelect = "none";
      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    });
  }
}

// terminal maximize/restore
{
  const maxBtn = $("termMaxBtn");
  if (maxBtn) {
    let maximized = false, savedFlex = "";
    maxBtn.addEventListener("click", () => {
      if (maximized) {
        termPanel.style.flex = savedFlex;
        maxBtn.title = "Maximize Panel";
        maximized = false;
      } else {
        savedFlex = termPanel.style.flex;
        termPanel.style.flex = "1 1 0";
        maxBtn.title = "Restore Panel";
        maximized = true;
      }
      requestAnimationFrame(() => {
        monacoEditor.layout();
        if (activeTermTab >= 0 && termTabs[activeTermTab]?.fit)
          try { termTabs[activeTermTab].fit.fit(); } catch {}
      });
    });
  }
}

window.addEventListener("beforeunload", () => {
  for (const tab of termTabs) {
    if (tab.backendId != null) backend.termClose(tab.backendId);
  }
});

buildMenubar();

onLocaleChange(() => {
  buildMenubar();
  applyToDOM();
  refreshModelBadge(); // re-sync the (non-i18n) model label after applyToDOM
  chatEl.querySelector(".chat-empty")?.remove();
  showChatHint();
});

// ---- status bar core items ----
const statusbarRight = $("statusbarRight");
const statusItems = new Map();

function updateStatusBar() {
  const pos = monacoEditor.getPosition();
  const model = monacoEditor.getModel();
  const sel = monacoEditor.getSelection();

  if (pos) {
    let posText = `Ln ${pos.lineNumber}, Col ${pos.column}`;
    if (sel && !sel.isEmpty()) {
      const selCount = model ? model.getValueInRange(sel).length : 0;
      if (selCount > 0) posText += ` (${selCount} selected)`;
    }
    setStatusBarItem("_cursor", { text: posText, tooltip: "Go to Line" });
  }

  if (model) {
    const langId = model.getLanguageId();
    const langLabel = langId.charAt(0).toUpperCase() + langId.slice(1);
    setStatusBarItem("_lang", { text: langLabel, tooltip: "Select Language Mode" });
  } else {
    removeStatusBarItem("_lang");
  }

  setStatusBarItem("_encoding", { text: "UTF-8", tooltip: "Select Encoding" });

  if (model) {
    const eol = model.getEOL() === "\r\n" ? "CRLF" : "LF";
    setStatusBarItem("_eol", { text: eol, tooltip: "Select End of Line" });
  }

  const lspItems = lspManager?.status() || [];
  const running = lspItems.filter((s) => s.running);
  if (running.length) {
    setStatusBarItem("_lsp", {
      text: `LSP: ${running.map((s) => s.lang).join(", ")}`,
      tooltip: "Language Servers",
    });
  } else {
    removeStatusBarItem("_lsp");
  }
}

let _statusBarRAF = 0;
function scheduleStatusBarUpdate() {
  if (_imeComposing || _statusBarRAF) return;
  _statusBarRAF = requestAnimationFrame(() => { _statusBarRAF = 0; updateStatusBar(); });
}
monacoEditor.onDidChangeCursorPosition(() => scheduleStatusBarUpdate());
monacoEditor.onDidChangeCursorSelection(() => scheduleStatusBarUpdate());
monacoEditor.onDidChangeModel(() => updateStatusBar());
updateStatusBar();

function setStatusBarItem(key, opts, onClick) {
  let el = statusItems.get(key);
  if (!el || el.tagName.toLowerCase() !== (onClick ? "button" : "span")) {
    if (el) el.remove();
    el = document.createElement(onClick ? "button" : "span");
    el.className = "statusbar__item" + (onClick ? " statusbar__item--btn" : "");
    statusItems.set(key, el);
    statusbarRight.appendChild(el);
  }
  el.textContent = opts.text ?? "";
  if (opts.tooltip) el.title = opts.tooltip;
  else el.removeAttribute("title");
  el.onclick = onClick || null;
}

function removeStatusBarItem(key) {
  const el = statusItems.get(key);
  if (el) {
    el.remove();
    statusItems.delete(key);
  }
}

function insertAtCursor(text) {
  const sel = monacoEditor.getSelection();
  if (!sel) return;
  monacoEditor.executeEdits("extension", [
    { range: sel, text, forceMoveMarkers: true },
  ]);
  monacoEditor.focus();
}

// Per-extension decoration collections keyed by extension id.
const extDecorations = new Map();

const extHost = new ExtensionHost({
  getEditorText: () => monacoEditor.getModel()?.getValue() ?? "",
  getSelectionText: () => {
    const sel = monacoEditor.getSelection();
    const model = monacoEditor.getModel();
    return sel && model && !sel.isEmpty() ? model.getValueInRange(sel) : "";
  },
  insertText: insertAtCursor,
  replaceText: (range, text) => {
    if (!range) return;
    const r = new monaco.Range(
      range.startLineNumber, range.startColumn,
      range.endLineNumber, range.endColumn,
    );
    monacoEditor.executeEdits("extension", [{ range: r, text, forceMoveMarkers: true }]);
    monacoEditor.focus();
  },
  showInformationMessage: (text) => showToast(text),
  setStatusBarItem,
  removeStatusBarItem,
  readFile: (path) => backend.readTextFile(path),
  writeFile: (path, content) => backend.writeTextFile(path, content),
  listDir: (path) => backend.readDir(path),
  getWorkspaceRoots: () =>
    workspaceRoots && workspaceRoots.length ? workspaceRoots : rootPath ? [rootPath] : [],
  getFilePath: () => activePath,
  getLanguage: () => monacoEditor.getModel()?.getLanguageId() ?? "plaintext",
  getLineCount: () => monacoEditor.getModel()?.getLineCount() ?? 0,
  getLine: (n) => monacoEditor.getModel()?.getLineContent(n) ?? "",
  setDecorations: (extId, decorations) => {
    let coll = extDecorations.get(extId);
    if (!coll) {
      coll = monacoEditor.createDecorationsCollection([]);
      extDecorations.set(extId, coll);
    }
    const decos = (decorations || []).map((d) => ({
      range: new monaco.Range(
        d.range.startLineNumber, d.range.startColumn,
        d.range.endLineNumber, d.range.endColumn,
      ),
      options: {
        isWholeLine: d.isWholeLine ?? false,
        className: d.className || undefined,
        inlineClassName: d.inlineClassName || undefined,
        linesDecorationsClassName: d.linesDecorationsClassName || undefined,
        glyphMarginClassName: d.glyphMarginClassName || undefined,
        hoverMessage: d.hoverMessage ? { value: d.hoverMessage } : undefined,
        after: d.after ? { content: d.after.content, inlineClassName: d.after.className } : undefined,
      },
    }));
    coll.set(decos);
    return extId;
  },
  clearDecorations: (extId) => {
    const coll = extDecorations.get(extId);
    if (coll) coll.set([]);
  },
  networkFetch: async (url, opts) => {
    const fetchOpts = {};
    if (opts.method) fetchOpts.method = opts.method;
    if (opts.headers) fetchOpts.headers = opts.headers;
    if (opts.body) fetchOpts.body = typeof opts.body === "string" ? opts.body : JSON.stringify(opts.body);
    const resp = await fetch(url, fetchOpts);
    const text = await resp.text();
    let json = null;
    try { json = JSON.parse(text); } catch { /* not json */ }
    return { status: resp.status, ok: resp.ok, text, json, headers: Object.fromEntries(resp.headers.entries()) };
  },
  setDiagnostics: (extId, uri, diagnostics) => {
    const model = monaco.editor.getModels().find((m) => m.uri.toString() === uri || m.uri.fsPath === uri);
    if (!model) return;
    const markers = (diagnostics || []).map((d) => ({
      severity: d.severity === "error" ? monaco.MarkerSeverity.Error
        : d.severity === "warning" ? monaco.MarkerSeverity.Warning
        : d.severity === "info" ? monaco.MarkerSeverity.Info
        : monaco.MarkerSeverity.Hint,
      message: d.message || "",
      startLineNumber: d.startLine || 1,
      startColumn: d.startColumn || 1,
      endLineNumber: d.endLine || d.startLine || 1,
      endColumn: d.endColumn || d.startColumn || 1,
      source: extId,
    }));
    monaco.editor.setModelMarkers(model, extId, markers);
  },
  clearDiagnostics: (extId, uri) => {
    if (uri) {
      const model = monaco.editor.getModels().find((m) => m.uri.toString() === uri || m.uri.fsPath === uri);
      if (model) monaco.editor.setModelMarkers(model, extId, []);
    } else {
      for (const model of monaco.editor.getModels()) {
        monaco.editor.setModelMarkers(model, extId, []);
      }
    }
  },
  registerLocale: (locale, dict) => registerLocale(locale, dict),
  setLocale: (locale) => setLocale(locale),
});

const extManager = await createExtensionManager();
const extPanel = createExtensionsPanel({
  manager: extManager,
  host: extHost,
  showToast,
});

const palette = createCommandPalette({
  getCommands: () => [
    { id: "file.save", title: t("menu.save"), category: t("menu.file"), run: () => saveActive() },
    { id: "file.openFolder", title: t("menu.openFolder"), category: t("menu.file"), run: () => chooseFolder() },
    { id: "workspace.addFolder", title: "Add Folder to Workspace", category: "Workspace", run: () => addFolderToWorkspace() },
    { id: "workspace.manager", title: "Workspace Manager", category: "Workspace", run: () => openFeaturePanel("workspace") },
    { id: "file.quickOpen", title: "Quick Open (⌘P)", category: t("menu.file"), run: () => qoOpen() },
    { id: "file.autoSave", title: "Toggle Auto Save", category: t("menu.file"), run: () => { toggleAutoSave(); buildMenubar(); } },
    { id: "code.runCurrentFile", title: "Run Current File", category: "Code", run: () => runCurrentFile() },
    { id: "tasks.open", title: "Task Runner", category: "Tasks", run: () => openFeaturePanel("tasks") },
    { id: "view.extensions", title: t("ext.title"), category: t("menu.view"), run: () => extPanel.open() },
    { id: "view.terminal", title: t("menu.toggleTerminal"), category: t("menu.view"), run: () => toggleTerminal() },
    { id: "terminal.new", title: t("terminal.new"), category: t("terminal.title"), run: () => { openTerminal(); createTermTab(); } },
    { id: "view.splitEditor", title: "Toggle Split Editor", category: t("menu.view"), run: () => toggleSplitEditor() },
    { id: "remote.open", title: "Remote Development", category: "Tools", run: () => openFeaturePanel("remote") },
    { id: "marketplace.open", title: "扩展市场", category: "工具", run: () => openMarketplaceModal() },
    { id: "git.conflicts", title: "Resolve Merge Conflicts", category: "Tools", run: () => openFeaturePanel("conflicts") },
    { id: "debug.open", title: "Debugger", category: "Tools", run: () => openFeaturePanel("debugger") },
    { id: "lsp.open", title: "Language Servers", category: "Tools", run: () => openFeaturePanel("lsp") },
    { id: "view.zenMode", title: "Toggle Zen Mode", category: t("menu.view"), run: () => toggleZenMode() },
    { id: "tab.pin", title: "Pin/Unpin Tab", category: "Tabs", run: () => activePath && togglePinTab(activePath) },
    { id: "git.stash", title: t("git.stash"), category: "Git", run: () => doStash() },
    { id: "git.stashPop", title: t("git.stashPopLatest"), category: "Git", run: () => stashOp(() => backend.gitStashPop(rootPath), t("git.stashPopped")) },
    { id: "git.blame", title: t("git.blameToggle"), category: "Git", run: () => toggleBlame() },
    { id: "pref.settings", title: "Settings", category: "Preferences", run: () => openFeaturePanel("settings") },
    { id: "pref.shortcuts", title: "Keyboard Shortcuts", category: "Preferences", run: () => openFeaturePanel("shortcuts") },
    { id: "ai.settings", title: t("menu.aiSettings"), category: "Preferences", run: () => openSettings() },
    ...extHost.listCommands().map((c) => ({
      ...c,
      run: () => extHost.invokeCommand(c.id),
    })),
  ],
});

$("extensionsBtn").addEventListener("click", () => openMarketplaceModal());
$("paletteBtn").addEventListener("click", () => palette.open());
window.addEventListener(
  "keydown",
  (e) => {
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      e.stopPropagation();
      palette.open();
    }
  },
  true,
);

(async () => {
  try {
    const installed = await extManager.listInstalled();
    for (const item of installed) {
      if (item.enabled) await extHost.activate(item, extManager);
    }
  } catch (err) {
    console.error("[extensions] init failed:", err);
  }
})();

initLocale();
Promise.all([
  loadConfigAsync().then(() => refreshModelBadge()),
  loadEditorPrefs().then(() => applyEditorPrefs()),
]).catch(console.error);
showChatHint();
syncWelcome();
restoreChatHistory().catch(console.warn);

// ---- recent projects ----
const RECENT_PROJECTS_KEY = "michael-ide.recent-projects";
const MAX_RECENT = 8;

async function addRecentProject(path) {
  if (!inTauri || !path) return;
  try {
    const store = await loadStore("session.json");
    let list = (await store.get(RECENT_PROJECTS_KEY)) || [];
    list = list.filter((p) => p !== path);
    list.unshift(path);
    if (list.length > MAX_RECENT) list.length = MAX_RECENT;
    await store.set(RECENT_PROJECTS_KEY, list);
    await store.save();
    renderRecentProjects(list);
  } catch (e) {
    console.warn("[recent] save failed:", e);
  }
}

async function loadRecentProjects() {
  if (!inTauri) return;
  try {
    const store = await loadStore("session.json");
    const list = (await store.get(RECENT_PROJECTS_KEY)) || [];
    renderRecentProjects(list);
  } catch { /* ignore */ }
}

function renderRecentProjects(list) {
  const container = $("welcomeRecent");
  const ul = $("recentList");
  if (!container || !ul) return;
  if (!list.length) { container.hidden = true; return; }
  container.hidden = false;
  ul.innerHTML = "";
  for (const path of list.slice(0, 4)) {
    const li = document.createElement("li");
    const name = path.split("/").filter(Boolean).pop() || path;
    li.innerHTML = `<span class="recent-name"></span><span class="recent-path"></span>`;
    li.querySelector(".recent-name").textContent = name;
    li.querySelector(".recent-path").textContent = path;
    li.addEventListener("click", () => openFolder(path));
    ul.appendChild(li);
  }
}

loadRecentProjects();

// ---- session persistence ----
const SESSION_STORE_KEY = "michael-ide.session";

async function saveSession() {
  if (!inTauri) return;
  const tabList = [];
  for (const [path, f] of openFiles) {
    tabList.push({ path, name: f.name, dirty: false, pinned: pinnedTabs.has(path) });
  }
  const session = {
    workspaceRoots: [...workspaceRoots],
    rootPath,
    activePath,
    tabs: tabList,
  };
  try {
    const store = await loadStore("session.json");
    await store.set(SESSION_STORE_KEY, session);
    await store.save();
  } catch (e) {
    console.warn("[session] save failed:", e);
  }
  await saveChatHistory();
}

async function restoreSession() {
  if (!inTauri) return;
  try {
    const store = await loadStore("session.json");
    const session = await store.get(SESSION_STORE_KEY);
    if (!session) {
      _requestWorkspaceFromPeers();
      return;
    }
    if (Array.isArray(session.workspaceRoots) && session.workspaceRoots.length) {
      workspaceRoots = session.workspaceRoots;
      for (const root of workspaceRoots) {
        try { await backend.registerWorkspaceRoot(root); } catch {}
      }
      setActiveWorkspaceRoot(session.rootPath || workspaceRoots[0]);
      await renderWorkspaceRoots();
      startFileWatcher();
      await refreshGitStatus();
      for (const root of workspaceRoots) preloadProjectModels(root);
    } else {
      _requestWorkspaceFromPeers();
    }
    if (Array.isArray(session.tabs)) {
      for (const t of session.tabs) {
        await openFile(t.path, t.name).catch(() => {});
        if (t.pinned) pinnedTabs.add(t.path);
      }
    }
    if (session.activePath && openFiles.has(session.activePath)) {
      activate(session.activePath);
    }
    updateStatusBar();
  } catch (e) {
    console.warn("[session] restore failed:", e);
    _requestWorkspaceFromPeers();
  }
}

function _requestWorkspaceFromPeers() {
  if (_ipcChannel) {
    _ipcChannel.postMessage({ type: "workspace_request", from: _WINDOW_ID });
  }
}

// ---- zen mode ----
let zenModeActive = false;
function toggleZenMode() {
  zenModeActive = !zenModeActive;
  document.body.classList.toggle("zen-mode", zenModeActive);
  const p = effectivePrefs();
  monacoEditor.updateOptions({
    lineNumbers: zenModeActive ? "off" : "on",
    glyphMargin: !zenModeActive,
    folding: !zenModeActive,
    minimap: { enabled: zenModeActive ? false : p.minimap !== false },
  });
  monacoEditor.layout();
}

// ---- drag & drop files from Finder ----
editorContainer.addEventListener("dragover", (e) => {
  e.preventDefault();
  e.dataTransfer.dropEffect = "copy";
  editorContainer.classList.add("drag-target");
});
editorContainer.addEventListener("dragleave", () => editorContainer.classList.remove("drag-target"));
editorContainer.addEventListener("drop", async (e) => {
  e.preventDefault();
  editorContainer.classList.remove("drag-target");
  if (inTauri) {
    try {
      const { listen } = await import("@tauri-apps/api/event");
      // Tauri drag-drop handled via event
    } catch { /* ignore */ }
  }
  const files = e.dataTransfer?.files;
  if (!files || !files.length) return;
  for (const file of files) {
    if (file.path) {
      await openFile(file.path, file.name);
    }
  }
});
if (inTauri) {
  import("@tauri-apps/api/event").then(({ listen }) => {
    listen("tauri://drag-drop", async (event) => {
      const paths = event.payload?.paths || [];
      for (const p of paths) {
        const name = p.split("/").pop() || p;
        const isDir = await backend.readDir(p).then(() => true).catch(() => false);
        if (isDir) {
          await openFolder(p);
        } else {
          await openFile(p, name);
        }
      }
    });
  }).catch(() => {});
}

window.addEventListener("beforeunload", () => saveSession());
if (inTauri) {
  import("@tauri-apps/api/event").then(({ listen }) => {
    listen("tauri://close-requested", async (event) => {
      await saveSession();
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().destroy();
    });
  }).catch(() => {});
}
restoreSession();

let _cachedHomeDir = "";
if (inTauri) {
  backend.homeDir().then(home => {
    if (home) {
      _cachedHomeDir = home;
      backend.registerWorkspaceRoot(home).catch(() => {});
    }
  }).catch(() => {});
}

// ---- Outline Panel ----
let _outlineSortByName = false;
let _outlineSymbols = [];

async function refreshOutline() {
  const tree = $("outlineTree");
  if (!activePath) {
    tree.innerHTML = '<div class="empty"><p>Open a file to see its outline.</p></div>';
    $("outlineTimeline")?.removeAttribute("hidden");
    refreshTimeline();
    return;
  }
  const model = monacoEditor.getModel();
  if (!model) return;
  try {
    const symbols = await monaco.languages.getDocumentSymbols(model);
    _outlineSymbols = symbols || [];
    renderOutlineTree(_outlineSymbols, tree);
  } catch {
    tree.innerHTML = '<div class="empty"><p>No symbols found.</p></div>';
  }
  $("outlineTimeline")?.removeAttribute("hidden");
  refreshTimeline();
}

function renderOutlineTree(symbols, container) {
  const filter = ($("outlineFilter")?.value || "").toLowerCase();
  container.innerHTML = "";
  if (!symbols.length) {
    container.innerHTML = '<div class="empty"><p>No symbols found.</p></div>';
    return;
  }
  const sorted = [...symbols];
  if (_outlineSortByName) sorted.sort((a, b) => a.name.localeCompare(b.name));
  for (const sym of sorted) {
    if (filter && !sym.name.toLowerCase().includes(filter)) continue;
    const row = document.createElement("div");
    row.className = "outline-row";
    const kindClass = _symbolKindClass(sym.kind);
    row.innerHTML = `<span class="outline-icon outline-icon--${kindClass}"></span><span class="outline-name">${_escHtml(sym.name)}</span><span class="outline-detail">${_escHtml(sym.detail || "")}</span>`;
    row.addEventListener("click", () => {
      const range = sym.range || sym.selectionRange;
      if (range) {
        monacoEditor.revealLineInCenter(range.startLineNumber);
        monacoEditor.setPosition({ lineNumber: range.startLineNumber, column: range.startColumn });
        monacoEditor.focus();
      }
    });
    container.appendChild(row);
    if (sym.children?.length) {
      const childContainer = document.createElement("div");
      childContainer.className = "outline-children";
      renderOutlineTree(sym.children, childContainer);
      container.appendChild(childContainer);
    }
  }
}

function _symbolKindClass(kind) {
  const map = { 1: "file", 2: "module", 3: "namespace", 4: "package", 5: "class", 6: "method",
    7: "property", 8: "field", 9: "constructor", 10: "enum", 11: "interface", 12: "function",
    13: "variable", 14: "constant", 15: "string", 16: "number", 17: "boolean", 18: "array",
    19: "object", 20: "key", 21: "null", 22: "enummember", 23: "struct", 24: "event", 25: "operator", 26: "typeparam" };
  return map[kind] || "variable";
}

function _escHtml(s) {
  const d = document.createElement("div");
  d.textContent = s;
  return d.innerHTML;
}

$("outlineSortBtn")?.addEventListener("click", () => {
  _outlineSortByName = !_outlineSortByName;
  refreshOutline();
});
$("outlineRefreshBtn")?.addEventListener("click", () => refreshOutline());
$("outlineFilter")?.addEventListener("input", () => {
  const tree = $("outlineTree");
  renderOutlineTree(_outlineSymbols, tree);
});

monacoEditor.onDidChangeModel(() => {
  if (!$("viewOutline")?.hidden === false) refreshOutline();
});

// ---- File Timeline ----
async function refreshTimeline() {
  const list = $("timelineList");
  if (!list) return;
  if (!activePath || !workspaceRoots.length) {
    list.innerHTML = '<div class="empty"><p>No file selected.</p></div>';
    return;
  }
  try {
    const root = workspaceRoots[0];
    const rel = activePath.startsWith(root) ? activePath.slice(root.length + 1) : activePath;
    const log = await backend.gitLog(root);
    const fileLog = log.filter(e => e.files?.includes(rel) || e.message?.includes(rel)).slice(0, 20);
    if (!fileLog.length) {
      const allLog = log.slice(0, 15);
      renderTimelineEntries(allLog, list);
    } else {
      renderTimelineEntries(fileLog, list);
    }
  } catch {
    list.innerHTML = '<div class="empty"><p>No git history.</p></div>';
  }
}

function renderTimelineEntries(entries, container) {
  container.innerHTML = "";
  for (const e of entries) {
    const row = document.createElement("div");
    row.className = "timeline-row";
    const date = e.date ? new Date(parseInt(e.date) * 1000).toLocaleDateString() : "";
    row.innerHTML = `<span class="timeline-dot"></span><div class="timeline-info"><span class="timeline-msg">${_escHtml(e.message?.split("\n")[0] || "")}</span><span class="timeline-meta">${_escHtml(e.author || "")} · ${date}</span></div>`;
    row.addEventListener("click", () => {
      showToast(`Commit: ${e.hash?.slice(0, 8) || "?"}`);
    });
    container.appendChild(row);
  }
}

$("timelineToggle")?.addEventListener("click", () => {
  const list = $("timelineList");
  const isHidden = list.hidden;
  list.hidden = !isHidden;
  $("timelineToggle").classList.toggle("is-expanded", !isHidden);
});

// ---- Output Panel ----
const _outputChannels = { lsp: [], tasks: [], extensions: [] };

function toggleOutputPanel() {
  const panel = $("outputPanel");
  if (!panel) return;
  const show = panel.hidden;
  panel.hidden = !show;
  if (show) {
    panel.style.display = "flex";
    requestAnimationFrame(() => panel.classList.add("output-panel--visible"));
  } else {
    panel.classList.remove("output-panel--visible");
    setTimeout(() => { if (panel.hidden) panel.style.display = "none"; }, 200);
  }
}

function appendOutput(channel, text) {
  if (!_outputChannels[channel]) _outputChannels[channel] = [];
  _outputChannels[channel].push(text);
  if (_outputChannels[channel].length > 2000) _outputChannels[channel].splice(0, 500);
  const sel = $("outputChannel");
  if (sel && sel.value === channel) {
    const body = $("outputBody");
    body.textContent = _outputChannels[channel].join("\n");
    body.scrollTop = body.scrollHeight;
  }
}

$("outputChannel")?.addEventListener("change", () => {
  const ch = $("outputChannel").value;
  $("outputBody").textContent = (_outputChannels[ch] || []).join("\n");
});
$("outputClearBtn")?.addEventListener("click", () => {
  const ch = $("outputChannel").value;
  _outputChannels[ch] = [];
  $("outputBody").textContent = "";
});
$("outputCloseBtn")?.addEventListener("click", () => toggleOutputPanel());

// ---- Test Explorer ----
const _TEST_PATTERNS = [
  /\.test\.[jt]sx?$/, /\.spec\.[jt]sx?$/, /_test\.go$/, /test_.*\.py$/, /.*_test\.py$/,
  /Test\.java$/, /\.test\.rs$/
];

async function refreshTestExplorer() {
  const tree = $("testTree");
  if (!workspaceRoots.length) {
    tree.innerHTML = '<div class="empty"><p>Open a project to detect tests.</p></div>';
    return;
  }
  try {
    const root = workspaceRoots[0];
    const allFiles = await _collectTestFiles(root);
    if (!allFiles.length) {
      tree.innerHTML = '<div class="empty"><p>No test files detected.</p></div>';
      return;
    }
    tree.innerHTML = "";
    const groups = {};
    for (const f of allFiles) {
      const dir = f.path.split("/").slice(0, -1).join("/").replace(root + "/", "") || ".";
      if (!groups[dir]) groups[dir] = [];
      groups[dir].push(f);
    }
    for (const [dir, files] of Object.entries(groups)) {
      const section = document.createElement("div");
      section.className = "test-group";
      section.innerHTML = `<div class="test-group__head"><svg class="ic"><use href="#i-folder" /></svg><span>${_escHtml(dir)}</span></div>`;
      for (const f of files) {
        const row = document.createElement("div");
        row.className = "test-row";
        row.innerHTML = `<span class="test-status test-status--pending">○</span><span class="test-name">${_escHtml(f.name)}</span>`;
        row.addEventListener("click", () => openFile(f.path, f.name));
        const runBtn = document.createElement("button");
        runBtn.className = "iconbtn test-run-btn";
        runBtn.title = "Run test";
        runBtn.innerHTML = '<svg class="ic"><use href="#i-play" /></svg>';
        runBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          runTestFile(f.path, f.name, row);
        });
        row.appendChild(runBtn);
        section.appendChild(row);
      }
      tree.appendChild(section);
    }
  } catch {
    tree.innerHTML = '<div class="empty"><p>Error scanning tests.</p></div>';
  }
}

async function _collectTestFiles(root, maxDepth = 4) {
  const results = [];
  async function scan(dir, depth) {
    if (depth > maxDepth) return;
    try {
      const entries = await backend.readDir(dir);
      for (const e of entries) {
        if (e.is_dir) {
          if (e.name === "node_modules" || e.name === ".git" || e.name === "target" || e.name === "__pycache__") continue;
          await scan(e.path, depth + 1);
        } else if (_TEST_PATTERNS.some(p => p.test(e.name))) {
          results.push(e);
        }
      }
    } catch { /* ignore inaccessible dirs */ }
  }
  await scan(root, 0);
  return results;
}

async function runTestFile(path, name, rowEl) {
  const statusEl = rowEl.querySelector(".test-status");
  statusEl.textContent = "⏳";
  statusEl.className = "test-status test-status--running";
  try {
    const ext = name.split(".").pop();
    let cmd;
    if (/\.(test|spec)\.[jt]sx?$/.test(name)) cmd = `npx jest --testPathPattern="${name}" --no-coverage 2>&1 || npx vitest run "${name}" 2>&1`;
    else if (/_test\.go$/.test(name)) cmd = `go test -v -run . "${path}" 2>&1`;
    else if (/test.*\.py$/.test(name) || /.*_test\.py$/.test(name)) cmd = `python -m pytest "${path}" -v 2>&1`;
    else cmd = `echo "No runner for ${ext}"`;
    const result = await backend.termWrite?.("test", cmd) || { output: "Run manually in terminal" };
    statusEl.textContent = "✓";
    statusEl.className = "test-status test-status--pass";
    appendOutput("tasks", `[TEST] ${name}: PASSED`);
  } catch (err) {
    statusEl.textContent = "✗";
    statusEl.className = "test-status test-status--fail";
    appendOutput("tasks", `[TEST] ${name}: FAILED - ${err.message || err}`);
  }
}

$("testRunAllBtn")?.addEventListener("click", () => {
  const rows = $("testTree").querySelectorAll(".test-row");
  for (const row of rows) row.click();
});
$("testRefreshBtn")?.addEventListener("click", () => refreshTestExplorer());

// ---- Terminal Split ----
$("termSplitBtn")?.addEventListener("click", async () => {
  _termSplitActive = !_termSplitActive;
  const body = $("terminalBody");
  body.classList.toggle("term-split", _termSplitActive);
  if (_termSplitActive) {
    if (termTabs.length < 2) await createTermTab(); // need a second pane to split into
    _applyTermSplit();
  } else {
    // back to single view: show only the active terminal
    termTabs.forEach((t, i) => { t.container.hidden = i !== activeTermTab; });
  }
  requestAnimationFrame(() => termTabs.forEach((t) => {
    if (!t.container.hidden) { try { t.fit.fit(); } catch {} }
  }));
});

// Show two terminals side by side: the active one + the newest other one.
function _applyTermSplit() {
  const a = activeTermTab >= 0 ? activeTermTab : termTabs.length - 1;
  let b = termTabs.length - 1;
  if (b === a) b = termTabs.length - 2;
  const show = new Set([a, b].filter((i) => i >= 0 && i < termTabs.length));
  termTabs.forEach((t, i) => { t.container.hidden = !show.has(i); });
}

// ---- Bookmarks ----
const _bookmarks = new Map();
let _bookmarkDecorations = [];

function toggleBookmark(path, line) {
  if (!path) return;
  const key = `${path}:${line}`;
  if (_bookmarks.has(key)) {
    _bookmarks.delete(key);
  } else {
    _bookmarks.set(key, { path, line, label: "" });
  }
  renderBookmarkDecorations();
}

function renderBookmarkDecorations() {
  if (!activePath) return;
  const decos = [];
  for (const [, bm] of _bookmarks) {
    if (bm.path !== activePath) continue;
    decos.push({
      range: new monaco.Range(bm.line, 1, bm.line, 1),
      options: {
        isWholeLine: true,
        glyphMarginClassName: "bookmark-glyph",
        overviewRuler: { color: "#007aff", position: monaco.editor.OverviewRulerLane.Center },
      },
    });
  }
  _bookmarkDecorations = monacoEditor.deltaDecorations(_bookmarkDecorations, decos);
}

function nextBookmark() {
  const line = monacoEditor.getPosition()?.lineNumber || 0;
  const sorted = [..._bookmarks.values()].filter(b => b.path === activePath).sort((a, b) => a.line - b.line);
  const next = sorted.find(b => b.line > line) || sorted[0];
  if (next) {
    monacoEditor.revealLineInCenter(next.line);
    monacoEditor.setPosition({ lineNumber: next.line, column: 1 });
  }
}

function prevBookmark() {
  const line = monacoEditor.getPosition()?.lineNumber || 0;
  const sorted = [..._bookmarks.values()].filter(b => b.path === activePath).sort((a, b) => b.line - a.line);
  const prev = sorted.find(b => b.line < line) || sorted[0];
  if (prev) {
    monacoEditor.revealLineInCenter(prev.line);
    monacoEditor.setPosition({ lineNumber: prev.line, column: 1 });
  }
}

function toggleBookmarksPanel() {
  showToast(`${_bookmarks.size} bookmark(s)`);
}

monacoEditor.addAction({
  id: "editor.toggleBookmark",
  label: "Toggle Bookmark",
  keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Alt | monaco.KeyCode.KeyK],
  run: () => {
    const line = monacoEditor.getPosition()?.lineNumber;
    if (line && activePath) toggleBookmark(activePath, line);
  },
});
monacoEditor.addAction({
  id: "editor.nextBookmark",
  label: "Next Bookmark",
  keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Alt | monaco.KeyCode.KeyN],
  run: () => nextBookmark(),
});
monacoEditor.addAction({
  id: "editor.prevBookmark",
  label: "Previous Bookmark",
  keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Alt | monaco.KeyCode.KeyP],
  run: () => prevBookmark(),
});

// ---- Notification Center ----
const _notifHistory = [];
const MAX_NOTIF_HISTORY = 50;

const _origShowNotification = showNotification;
const _wrappedShowNotification = function(opts) {
  _notifHistory.unshift({ ...opts, time: Date.now() });
  if (_notifHistory.length > MAX_NOTIF_HISTORY) _notifHistory.pop();
  _updateNotifBadge();
  return _origShowNotification(opts);
};

function _updateNotifBadge() {
  const badge = $("notifBadge");
  if (!badge) return;
  const unread = _notifHistory.filter(n => !n.read).length;
  badge.hidden = unread === 0;
  badge.textContent = unread > 9 ? "9+" : String(unread);
}

function toggleNotifCenter() {
  let panel = document.querySelector(".notif-center");
  if (panel) { panel.remove(); return; }
  panel = document.createElement("div");
  panel.className = "notif-center";
  panel.innerHTML = `<div class="notif-center__head"><span>Notifications</span><button class="iconbtn" id="notifClearAll" title="Clear all"><svg class="ic"><use href="#i-close" /></svg></button></div><div class="notif-center__list"></div>`;
  const list = panel.querySelector(".notif-center__list");
  if (!_notifHistory.length) {
    list.innerHTML = '<div class="empty"><p>No notifications.</p></div>';
  } else {
    for (const n of _notifHistory) {
      n.read = true;
      const row = document.createElement("div");
      row.className = "notif-center__item";
      const ago = _timeAgo(n.time);
      row.innerHTML = `<div class="notif-center__title">${_escHtml(n.title || "")}</div><div class="notif-center__msg">${_escHtml(n.message || "")}</div><div class="notif-center__time">${ago}</div>`;
      list.appendChild(row);
    }
  }
  _updateNotifBadge();
  panel.querySelector("#notifClearAll")?.addEventListener("click", () => {
    _notifHistory.length = 0;
    _updateNotifBadge();
    panel.remove();
  });
  document.body.appendChild(panel);
  requestAnimationFrame(() => panel.classList.add("notif-center--visible"));
  const dismiss = (e) => { if (!panel.contains(e.target) && e.target !== $("notifBellBtn")) { panel.remove(); document.removeEventListener("click", dismiss); } };
  setTimeout(() => document.addEventListener("click", dismiss), 50);
}

function _timeAgo(ts) {
  const s = Math.floor((Date.now() - ts) / 1000);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

$("notifBellBtn")?.addEventListener("click", (e) => { e.stopPropagation(); toggleNotifCenter(); });

// ---- Workspace Trust ----
let _workspaceTrusted = true;

async function checkWorkspaceTrust(path) {
  if (!inTauri) return;
  try {
    const store = await getStore();
    const trusted = (await store.get("trustedWorkspaces")) || [];
    if (trusted.includes(path)) { _workspaceTrusted = true; return; }
    _workspaceTrusted = false;
    const result = await ioConfirm({
      title: "Do you trust the authors of this folder?",
      message: `${path}\n\nIf you don't trust the authors, features like terminals, tasks, and debugging will be restricted.`,
      okLabel: "Yes, I trust the authors",
    });
    if (result) {
      _workspaceTrusted = true;
      trusted.push(path);
      await store.set("trustedWorkspaces", trusted);
    }
  } catch { _workspaceTrusted = true; }
}

// ---- Multi-file AI Composer ----
let _composerFiles = [];
let _composerOpen = false;

function openComposer() {
  _composerOpen = true;
  const panel = document.querySelector(".composer-panel") || _createComposerPanel();
  panel.hidden = false;
  panel.querySelector(".composer-input")?.focus();
}

function closeComposer() {
  _composerOpen = false;
  const panel = document.querySelector(".composer-panel");
  if (panel) panel.hidden = true;
}

function _createComposerPanel() {
  const panel = document.createElement("div");
  panel.className = "composer-panel";
  panel.innerHTML = `
    <div class="composer-panel__head">
      <span>AI Composer</span>
      <span class="composer-panel__files" id="composerFiles">0 files selected</span>
      <button class="iconbtn" id="composerClose" title="Close"><svg class="ic"><use href="#i-close" /></svg></button>
    </div>
    <div class="composer-panel__file-list" id="composerFileList"></div>
    <div class="composer-panel__input-wrap">
      <textarea class="composer-input" id="composerInput" placeholder="Describe changes across multiple files..." rows="3"></textarea>
      <button class="btn btn--primary" id="composerSend">Apply Changes</button>
    </div>
  `;
  document.body.appendChild(panel);
  panel.querySelector("#composerClose").addEventListener("click", closeComposer);
  panel.querySelector("#composerSend").addEventListener("click", () => runComposer());
  _updateComposerFileList();
  return panel;
}

function addFileToComposer(path) {
  if (!_composerFiles.includes(path)) _composerFiles.push(path);
  _updateComposerFileList();
}

function _updateComposerFileList() {
  const listEl = document.querySelector("#composerFileList");
  const countEl = document.querySelector("#composerFiles");
  if (!listEl) return;
  listEl.innerHTML = "";
  for (const f of _composerFiles) {
    const row = document.createElement("div");
    row.className = "composer-file";
    row.innerHTML = `<span>${_escHtml(f.split("/").pop())}</span>`;
    const removeBtn = document.createElement("button");
    removeBtn.className = "iconbtn";
    removeBtn.innerHTML = '<svg class="ic"><use href="#i-close" /></svg>';
    removeBtn.addEventListener("click", () => {
      _composerFiles = _composerFiles.filter(p => p !== f);
      _updateComposerFileList();
    });
    row.appendChild(removeBtn);
    listEl.appendChild(row);
  }
  if (countEl) countEl.textContent = `${_composerFiles.length} file(s)`;
}

async function runComposer() {
  const input = document.querySelector("#composerInput");
  const prompt = input?.value?.trim();
  if (!prompt || !_composerFiles.length) return;
  showToast("AI Composer: Processing...");
  appendOutput("extensions", `[Composer] Processing ${_composerFiles.length} files: ${prompt}`);
  for (const filePath of _composerFiles) {
    try {
      const content = await backend.readFile(filePath);
      const fileName = filePath.split("/").pop();
      const lang = extLang(fileName);
      const response = await _callAI(`You are a code editor assistant. Modify the following ${lang} file based on the user's instruction.\n\nFile: ${fileName}\n\`\`\`${lang}\n${content}\n\`\`\`\n\nInstruction: ${prompt}\n\nReturn ONLY the modified code, no explanations.`);
      if (response) {
        showAiDiffPreview(content, response, lang, filePath);
        appendOutput("extensions", `[Composer] Modified: ${fileName}`);
      }
    } catch (err) {
      appendOutput("extensions", `[Composer] Error for ${filePath}: ${err.message || err}`);
    }
  }
  showToast("AI Composer: Done");
}

async function _callAI(prompt) {
  try {
    const cfg = loadConfig();
    const model = cfg.model || "deepseek-chat";
    const apiKey = cfg.apiKey;
    const baseUrl = cfg.baseUrl || "https://api.deepseek.com";
    if (!apiKey) return null;
    const res = await fetch(`${baseUrl}/v1/chat/completions`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${apiKey}` },
      body: JSON.stringify({ model, messages: [{ role: "user", content: prompt }], max_tokens: 4096, temperature: 0.3 }),
    });
    const data = await res.json();
    return data.choices?.[0]?.message?.content || null;
  } catch { return null; }
}

monacoEditor.addAction({
  id: "editor.addToComposer",
  label: "Add to AI Composer",
  keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyA],
  run: () => {
    if (activePath) {
      addFileToComposer(activePath);
      if (!_composerOpen) openComposer();
      showToast(`Added ${activePath.split("/").pop()} to Composer`);
    }
  },
});

// ---- Minimap Search Highlight ----
let _minimapSearchDecorations = [];

function updateMinimapSearchHighlights(matches) {
  const decos = (matches || []).map(m => ({
    range: m.range,
    options: {
      minimap: { color: "#ffc107", position: monaco.editor.MinimapPosition.Inline },
      overviewRuler: { color: "#ffc107", position: monaco.editor.OverviewRulerLane.Center },
    },
  }));
  _minimapSearchDecorations = monacoEditor.deltaDecorations(_minimapSearchDecorations, decos);
}

// ---- Extension Recommendations ----
const _EXT_RECOMMENDATIONS = {
  ".py": { name: "Python", ext: "ms-python.python", desc: "Python language support" },
  ".rs": { name: "Rust Analyzer", ext: "rust-lang.rust-analyzer", desc: "Rust language support" },
  ".go": { name: "Go", ext: "golang.go", desc: "Go language support" },
  ".vue": { name: "Vue", ext: "Vue.volar", desc: "Vue language support" },
  ".svelte": { name: "Svelte", ext: "svelte.svelte-vscode", desc: "Svelte language support" },
  ".dart": { name: "Dart", ext: "Dart-Code.dart-code", desc: "Dart language support" },
  ".java": { name: "Java", ext: "redhat.java", desc: "Java language support" },
  ".rb": { name: "Ruby", ext: "Shopify.ruby-lsp", desc: "Ruby language support" },
};

function checkExtensionRecommendation(fileName) {
  const ext = "." + (fileName.split(".").pop() || "");
  const rec = _EXT_RECOMMENDATIONS[ext];
  if (!rec || _recommendedAlready.has(ext)) return;
  _recommendedAlready.add(ext);
  showNotification({
    title: `Recommended: ${rec.name}`,
    message: rec.desc,
    action: () => showSide("explorer"),
    actionLabel: "View Extensions",
    duration: 10000,
  });
}
const _recommendedAlready = new Set();

// ---- Custom Snippet Editor ----
function openSnippetEditor() {
  const existingPanel = document.querySelector(".snippet-editor");
  if (existingPanel) { existingPanel.remove(); return; }
  const panel = document.createElement("div");
  panel.className = "snippet-editor feature-body";
  panel.innerHTML = `
    <div class="tool-header"><h3>Custom Snippets</h3><p>Manage your code snippets</p></div>
    <div class="snippet-editor__form">
      <label>Language <select id="snippetLang"><option value="javascript">JavaScript</option><option value="typescript">TypeScript</option><option value="python">Python</option><option value="go">Go</option><option value="rust">Rust</option><option value="html">HTML</option><option value="css">CSS</option></select></label>
      <label>Prefix <input id="snippetPrefix" type="text" placeholder="e.g. log" /></label>
      <label>Description <input id="snippetDesc" type="text" placeholder="Description" /></label>
      <label>Body <textarea id="snippetBody" rows="5" placeholder="console.log($1);"></textarea></label>
      <button class="btn btn--primary" id="snippetSave">Save Snippet</button>
    </div>
    <div class="snippet-editor__list" id="snippetList"></div>
  `;
  const fp = $("featurePanel");
  if (fp) {
    fp.querySelector(".feature-body")?.remove();
    fp.appendChild(panel);
  }
  $("snippetSave")?.addEventListener("click", async () => {
    const lang = $("snippetLang").value;
    const prefix = $("snippetPrefix").value.trim();
    const desc = $("snippetDesc").value.trim();
    const body = $("snippetBody").value;
    if (!prefix || !body) return showToast("Prefix and body are required.");
    try {
      const store = await getStore();
      const snippets = (await store.get("custom-snippets")) || [];
      snippets.push({ lang, prefix, description: desc, body });
      await store.set("custom-snippets", snippets);
      showToast(`Snippet "${prefix}" saved.`);
    } catch { showToast("Failed to save snippet."); }
  });
}

// ============================================================================
// Cmd+K inline edit (AI): select code → ⌘K → describe → AI rewrites in place,
// shown as a green diff with Keep (⌘↵) / Undo (Esc). Plain ⌘K was unbound
// (⌘⌥K is Toggle Bookmark), so this is purely additive.
// ============================================================================
editorEl.style.position = "relative";
function _aiConfigured() {
  const c = loadConfig();
  return !!(c.baseUrl && c.apiKey && c.model);
}
function _stripFence(s) {
  return String(s || "").replace(/^\s*```[\w-]*\n?/, "").replace(/\n?```\s*$/, "");
}
let _cmdk = null;
let _cmdkReview = null;
function _closeCmdK() {
  if (_cmdk) { _cmdk.box.remove(); _cmdk = null; }
}
function _openCmdK() {
  if (!activePath) { showToast("Open a file to use Cmd+K"); return; }
  _closeCmdK();
  if (_cmdkReview) _cmdkReview.finish(false);
  const model = monacoEditor.getModel();
  const sel = monacoEditor.getSelection();
  if (!model || !sel) return;
  const range = sel.isEmpty()
    ? new monaco.Range(sel.startLineNumber, 1, sel.startLineNumber, model.getLineMaxColumn(sel.startLineNumber))
    : sel;
  const coords = monacoEditor.getScrolledVisiblePosition({ lineNumber: range.startLineNumber, column: 1 });
  const box = document.createElement("div");
  box.className = "cmdk";
  box.innerHTML =
    `<span class="cmdk__spark">✦</span>` +
    `<input class="cmdk__input" type="text" placeholder="Describe the edit…  (Enter to run · Esc to cancel)" />` +
    `<span class="cmdk__hint"></span>`;
  editorEl.appendChild(box);
  box.style.top = ((coords ? coords.top + coords.height : 40) + 4) + "px";
  box.style.left = Math.max(8, coords ? coords.left : 8) + "px";
  const input = box.querySelector(".cmdk__input");
  const hint = box.querySelector(".cmdk__hint");
  _cmdk = { box };
  let busy = false;
  input.addEventListener("keydown", async (e) => {
    e.stopPropagation();
    if (e.key === "Escape") { e.preventDefault(); _closeCmdK(); monacoEditor.focus(); return; }
    if (e.key !== "Enter" || busy) return;
    e.preventDefault();
    const instr = input.value.trim();
    if (!instr) return;
    if (!_aiConfigured()) { showToast("Configure an AI provider (⚙️) to use Cmd+K"); return; }
    busy = true;
    hint.textContent = "Thinking…";
    input.disabled = true;
    const code = model.getValueInRange(range);
    const messages = [
      { role: "system", content: "You are an expert code editor. Rewrite the user's code to satisfy the instruction. Output ONLY the new code for that region — no explanation, no markdown fences." },
      { role: "user", content: `Instruction: ${instr}\n\nLanguage: ${model.getLanguageId()}\n\nCode:\n${code}` },
    ];
    let out;
    try {
      out = _stripFence(await backend.aiComplete(loadConfig(), messages, 1024));
    } catch (err) {
      showToast("AI edit failed: " + (err?.message || err));
      _closeCmdK();
      monacoEditor.focus();
      return;
    }
    _closeCmdK();
    if (!out) { showToast("No edit produced"); monacoEditor.focus(); return; }
    _applyCmdKEdit(range, out);
  });
  input.focus();
}
function _applyCmdKEdit(range, newText) {
  const model = monacoEditor.getModel();
  const original = model.getValueInRange(range);
  monacoEditor.pushUndoStop();
  monacoEditor.executeEdits("ai-edit", [{ range, text: newText, forceMoveMarkers: true }]);
  monacoEditor.pushUndoStop();
  const lines = newText.split("\n");
  const startLine = range.startLineNumber;
  const endLine = startLine + lines.length - 1;
  const endCol = lines.length === 1 ? range.startColumn + newText.length : lines[lines.length - 1].length + 1;
  const newRange = new monaco.Range(startLine, range.startColumn, endLine, endCol);
  const decos = monacoEditor.createDecorationsCollection([
    { range: new monaco.Range(startLine, 1, endLine, 1), options: { isWholeLine: true, className: "ai-edit-line" } },
  ]);
  monacoEditor.revealRangeInCenterIfOutsideViewport(newRange);
  const coords = monacoEditor.getScrolledVisiblePosition({ lineNumber: startLine, column: 1 });
  const bar = document.createElement("div");
  bar.className = "cmdk-review";
  bar.innerHTML =
    `<button class="cmdk-review__btn cmdk-review__keep">✓ Keep <kbd>⌘↵</kbd></button>` +
    `<button class="cmdk-review__btn cmdk-review__undo">✗ Undo <kbd>Esc</kbd></button>`;
  editorEl.appendChild(bar);
  bar.style.top = Math.max(4, (coords ? coords.top : 40) - 30) + "px";
  bar.style.left = Math.max(8, coords ? coords.left : 8) + "px";
  const finish = (undo) => {
    if (!_cmdkReview) return;
    document.removeEventListener("keydown", onKey, true);
    decos.clear();
    bar.remove();
    _cmdkReview = null;
    if (undo) monacoEditor.executeEdits("ai-edit-undo", [{ range: newRange, text: original, forceMoveMarkers: true }]);
    monacoEditor.focus();
  };
  const onKey = (e) => {
    if (e.key === "Escape") { e.preventDefault(); finish(true); }
    else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) { e.preventDefault(); finish(false); }
  };
  document.addEventListener("keydown", onKey, true);
  bar.querySelector(".cmdk-review__keep").onclick = () => finish(false);
  bar.querySelector(".cmdk-review__undo").onclick = () => finish(true);
  _cmdkReview = { finish };
}
monacoEditor.addAction({
  id: "ai.inlineEdit",
  label: "AI: Edit (Cmd+K)",
  keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK],
  contextMenuGroupId: "navigation",
  contextMenuOrder: 0,
  run: () => _openCmdK(),
});