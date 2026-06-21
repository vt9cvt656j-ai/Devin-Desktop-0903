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
import "@xterm/xterm/css/xterm.css";
import { renderMarkdownInto } from "./markdown.js";
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
    tasksList: (root) => core.invoke("tasks_list", { root }),
    taskRunCapture: (cwd, command) => core.invoke("task_run_capture", { cwd, command }),
    pickFolder: () => dialog.open({ directory: true, multiple: false }),
    aiChat: (config, messages, onEvent) => {
      const channel = new core.Channel();
      channel.onmessage = onEvent;
      return core.invoke("ai_chat", { config, messages, onEvent: channel });
    },
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
  fixedOverflowWidgets: true,
  fontSize: 13,
  fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
  minimap: { enabled: true, maxColumn: 80, renderCharacters: false },
  scrollBeyondLastLine: false,
  renderWhitespace: "selection",
  glyphMargin: true,
  padding: { top: 10 },
  bracketPairColorization: { enabled: true, independentColorPoolPerBracketType: true },
  guides: { bracketPairs: true, indentation: true, highlightActiveIndentation: true },
  smoothScrolling: true,
  cursorBlinking: "smooth",
  cursorSmoothCaretAnimation: "on",
  stickyScroll: { enabled: true },
  linkedEditing: true,
  colorDecorators: true,
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
  if (runBtn) runBtn.disabled = !f.isImage;
  $("windowTitle").textContent = f.name + " — Michael IDE";
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
      $("windowTitle").textContent = "Michael IDE";
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
  renderTabs();
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
lspManager.registerProviders();

const lspLogBuffers = new Map();
function lspLogSink(lang, line) {
  let buf = lspLogBuffers.get(lang);
  if (!buf) {
    buf = [];
    lspLogBuffers.set(lang, buf);
  }
  buf.push(line);
  if (buf.length > 400) buf.shift();
  document.dispatchEvent(new CustomEvent("lsp-log", { detail: { lang } }));
}

function updateLspStatusBar() {
  if (!lspManager) return;
  const ready = lspManager.status().filter((s) => s.initialized).map((s) => s.lang);
  if (ready.length) {
    setStatusBarItem(
      "lsp",
      { text: `LSP: ${ready.join(", ")}`, tooltip: "Active language servers (real LSP)" },
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
  _launchConfigsCache = null; // re-discover launch.json for the new root
  if (workspaceRoots.length > 1) {
    rootNameEl.textContent = `${workspaceRoots.length} folders`;
    rootNameEl.title = workspaceRoots.join("\n");
  } else {
    rootNameEl.textContent = basename(path);
    rootNameEl.title = path;
  }
  setExplorerToolsEnabled(Boolean(path));
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
  try { await backend.registerWorkspaceRoot(path); } catch { /* browser preview */ }
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

function handleFsChanges(paths) {
  if (!rootPath) return;
  const dirsToReload = new Set();
  for (const p of paths) {
    const dir = parentDir(p);
    if (dir && (workspaceRoots.some((r) => dir.startsWith(r)) || dir.startsWith(rootPath))) {
      dirsToReload.add(dir);
    }
  }
  for (const dir of dirsToReload) {
    if (dirNodes.has(dir) || workspaceRoots.includes(dir)) {
      reloadDir(dir);
    }
  }
  refreshGitStatus();
}

async function addFolderToWorkspace() {
  const picked = await backend.pickFolder();
  if (!picked) return;
  if (!workspaceRoots.includes(picked)) workspaceRoots.push(picked);
  try { await backend.registerWorkspaceRoot(picked); } catch { /* browser preview */ }
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
  $("tabExplorer").classList.toggle("is-active", which === "explorer" || which === "search");
  $("tabGit").classList.toggle("is-active", which === "git");
  const layout = document.querySelector(".layout");
  if (layout) layout.classList.remove("hide-explorer");
  if (which === "search") {
    const si = $("searchInput");
    si.focus();
    si.select();
  } else if (which === "git") {
    refreshGitStatus();
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

monacoEditor.onDidChangeCursorPosition(() => {
  if (blameEnabled) updateBlameLine();
});
// Edits shift line numbers, so the cached blame becomes stale — hide until the
// file is saved again (refreshBlame re-runs from saveActive / activate).
monacoEditor.onDidChangeModelContent(() => {
  if (blameEnabled) {
    blameMap = null;
    blameMapPath = null;
    blameDecorations.set([]);
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
  gutterTimer = setTimeout(updateGutter, 250);
});

// ---- diff view ----
let diffEditor = null;
const diffViewEl = $("diffView");

function ensureDiffEditor() {
  if (diffEditor) return diffEditor;
  diffEditor = monaco.editor.createDiffEditor($("diffBody"), {
    theme: matchMedia("(prefers-color-scheme: dark)").matches ? "vs-dark" : "vs",
    automaticLayout: true,
    readOnly: true,
    renderSideBySide: true,
    fontSize: 13,
    fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
  });
  return diffEditor;
}

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
  return _cfgCache || {};
}

async function loadConfigAsync() {
  if (_cfgCache) return _cfgCache;
  await migrateFromLocalStorage();
  const store = await getStore();
  _cfgCache = (await store.get(CFG_KEY)) || {};
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

const history = [];
let streaming = false;
const CHAT_STORE_KEY = "michael-ide.chat-history";

async function saveChatHistory() {
  if (!inTauri || history.length === 0) return;
  try {
    const store = await loadStore("session.json");
    await store.set(CHAT_STORE_KEY, history.slice(-50));
    await store.save();
  } catch (e) { console.warn("[chat] save failed:", e); }
}

async function restoreChatHistory() {
  if (!inTauri) return;
  try {
    const store = await loadStore("session.json");
    const saved = await store.get(CHAT_STORE_KEY);
    if (!Array.isArray(saved) || saved.length === 0) return;
    for (const m of saved) {
      history.push(m);
      addMessage(m.role === "assistant" ? "assistant" : "user", m.content);
    }
  } catch (e) { console.warn("[chat] restore failed:", e); }
}

// Syntax-highlight code-card bodies by reusing Monaco's tokenizer (matches the
// editor theme, no extra dependency). Returns null on failure so the card keeps
// its plain, already-escaped text.
async function highlightCode(code, lang) {
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
    if (text) renderMarkdownInto(body, text, { highlighter: highlightCode });
  } else {
    wrap.innerHTML = `<span class="msg__who"><span></span></span><div class="msg__body"></div>`;
    wrap.querySelector(".msg__who span").textContent = t("assistant.you");
    body = wrap.querySelector(".msg__body");
    body.textContent = text;
  }
  chatEl.appendChild(wrap);
  chatEl.scrollTop = chatEl.scrollHeight;
  return body;
}

// Devin-style "thinking" card shown while the first token is pending.
function thinkingCard() {
  const t = document.createElement("div");
  t.className = "thinking";
  t.innerHTML =
    `<span class="thinking__orb thinking__orb--logo"><img class="assistant-logo" src="/src/assets/logo.png" alt="" aria-hidden="true" /></span>` +
    `<span class="thinking__text">${t("assistant.thinking")}</span>`;
  return t;
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

async function sendPrompt(text) {
  const config = loadConfig();
  if (!config.baseUrl || !config.apiKey || !config.model) {
    openSettings();
    showToast(t("assistant.configFirst"));
    return;
  }
  if (streaming) return;
  chatEl.querySelector(".chat-empty")?.remove();

  addMessage("user", text);

  // Build the request: system prompt, optional file context, history, prompt.
  const messages = [
    { role: "system", content: "You are Michael IDE's coding assistant. Be concise and precise. Use fenced code blocks for code." },
  ];
  if (activePath) {
    const f = openFiles.get(activePath);
    const sel = monacoEditor.getModel() === f.model ? monacoEditor.getSelection() : null;
    const selected = sel && !sel.isEmpty() ? f.model.getValueInRange(sel) : "";
    let ctx = `Open file: ${activePath}\n\n\`\`\`\n${f.model.getValue().slice(0, 12000)}\n\`\`\``;
    if (selected) ctx += `\n\nSelected text:\n\`\`\`\n${selected.slice(0, 4000)}\n\`\`\``;
    messages.push({ role: "user", content: ctx });
  }
  for (const m of history) messages.push(m);
  messages.push({ role: "user", content: text });
  history.push({ role: "user", content: text });

  const body = addMessage("assistant", "");
  body.appendChild(thinkingCard());
  let acc = "";
  let err = null;
  let raf = 0;
  const flushStream = () => {
    raf = 0;
    renderMarkdownInto(body, acc, { streaming: true });
    chatEl.scrollTop = chatEl.scrollHeight;
  };
  const scheduleStream = () => {
    if (!raf) raf = requestAnimationFrame(flushStream);
  };
  streaming = true;
  try {
    await backend.aiChat(config, messages, (ev) => {
      if (ev.kind === "token") {
        acc += ev.delta;
        scheduleStream();
      } else if (ev.kind === "error") {
        err = ev.message;
      }
    });
  } catch (e) {
    if (!err) err = String(e);
  } finally {
    if (raf) cancelAnimationFrame(raf);
    streaming = false;
    body.querySelector(".thinking")?.remove();
    if (acc) {
      renderMarkdownInto(body, acc, { highlighter: highlightCode });
      // Don't push a truncated/errored reply into history — it would feed
      // incomplete context into later requests.
      if (!err) { history.push({ role: "assistant", content: acc }); saveChatHistory(); }
    }
    if (err) {
      const note = document.createElement("div");
      note.className = "msg__error";
      note.textContent = "⚠️ " + err;
      body.appendChild(note);
    }
    chatEl.scrollTop = chatEl.scrollHeight;
  }
}

// ---- AI inline code completion (Edit Prediction) ----
let _completionTimer = null;
const COMPLETION_DEBOUNCE = 600;
let _completionAbort = null;

function initInlineCompletion() {
  monaco.languages.registerInlineCompletionsProvider("*", {
    provideInlineCompletions: async (model, position, _context, token) => {
      const config = loadConfig();
      if (!config.baseUrl || !config.apiKey || !config.model) return { items: [] };

      const textBefore = model.getValueInRange({
        startLineNumber: Math.max(1, position.lineNumber - 50),
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column,
      });
      const textAfter = model.getValueInRange({
        startLineNumber: position.lineNumber,
        startColumn: position.column,
        endLineNumber: Math.min(model.getLineCount(), position.lineNumber + 10),
        endColumn: model.getLineMaxColumn(Math.min(model.getLineCount(), position.lineNumber + 10)),
      });

      if (textBefore.trim().length < 3) return { items: [] };

      const lang = model.getLanguageId();
      const fileName = activePath ? activePath.split("/").pop() : "untitled";

      const msgs = [
        { role: "system", content: `You are a code completion engine. Complete the code at the cursor position. Output ONLY the completion text (the code that should be inserted at the cursor). No explanations, no markdown, no code fences. If no meaningful completion, output nothing.` },
        { role: "user", content: `File: ${fileName} (${lang})\n\n--- CODE BEFORE CURSOR ---\n${textBefore.slice(-2000)}\n--- CURSOR IS HERE ---\n--- CODE AFTER CURSOR ---\n${textAfter.slice(0, 500)}` },
      ];

      try {
        if (_completionAbort) _completionAbort.abort();
        const controller = new AbortController();
        _completionAbort = controller;

        const url = `${config.baseUrl.replace(/\/+$/, "")}/chat/completions`;
        const resp = await fetch(url, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${config.apiKey}`,
          },
          body: JSON.stringify({
            model: config.model,
            messages: msgs,
            max_tokens: 128,
            temperature: 0,
            stream: false,
          }),
          signal: controller.signal,
        });
        if (token.isCancellationRequested) return { items: [] };
        if (!resp.ok) return { items: [] };
        const data = await resp.json();
        const text = data?.choices?.[0]?.message?.content;
        if (!text || !text.trim()) return { items: [] };
        return {
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
      } catch {
        return { items: [] };
      }
    },
    freeInlineCompletions: () => {},
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

// ---- settings dialog ----
const settingsEl = $("settings");
function openSettings() {
  const c = loadConfig();
  $("cfgBaseUrl").value = c.baseUrl || "https://api.openai.com/v1";
  $("cfgApiKey").value = c.apiKey || "";
  $("cfgModel").value = c.model || "gpt-4o-mini";
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
  const checkDone = setInterval(async () => {
    try {
      const cmds = await backend.termListCommands();
      const toolCmd = cmd.split(/\s+/).pop();
      if (cmds.includes(toolCmd)) {
        clearInterval(tick); clearInterval(checkDone);
        bar.style.width = "100%";
        card.querySelector(".notif-card__title").textContent = `${name} 安装完成`;
        card.querySelector(".notif-card__msg").textContent = "重新打开文件即可使用智能补全";
        bar.style.background = "#34c759";
        setTimeout(() => { card.classList.remove("notif-card--visible"); setTimeout(() => card.remove(), 300); }, 4000);
      }
    } catch {}
  }, 3000);
  setTimeout(() => { clearInterval(tick); clearInterval(checkDone); card.remove(); }, 120000);
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
      return `cd ${shellQuote(dirname(path))} && javac ${shellQuote(name)} && java ${className}`;
    }
    case "json":
      return `python3 -m json.tool ${q}`;
    default:
      return null;
  }
}

async function runCurrentFile() {
  if (!activePath) {
    showToast("Open a file to run.");
    return;
  }
  const command = runCommandForFile(activePath);
  if (!command) {
    showToast(`No run command configured for ${basename(activePath)}.`);
    return;
  }
  const file = openFiles.get(activePath);
  if (file?.dirty) await saveActive();
  await openTerminal();
  writeToActiveTerminal(`\n${command}\n`);
  showToast(`Running ${basename(activePath)} in terminal`);
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
  body.classList.add("mkt-body");
  const wrap = document.createElement("div");
  wrap.className = "mkt";
  body.appendChild(wrap);

  // Per-extension hue drives a soft tile tint that adapts to light/dark in CSS,
  // keeping the gallery calm and cohesive instead of loud rainbow gradients.
  const ICON_SVGS = {
    theme: `<circle cx="13.5" cy="6.5" r="1.4" fill="currentColor"/><circle cx="17.3" cy="10.5" r="1.4" fill="currentColor"/><circle cx="8.4" cy="7.4" r="1.4" fill="currentColor"/><circle cx="6.6" cy="12.4" r="1.4" fill="currentColor"/><path d="M12 2.5C6.75 2.5 2.5 6.75 2.5 12S6.75 21.5 12 21.5c1.1 0 2-.9 2-2 0-.5-.2-.96-.5-1.3-.3-.34-.5-.8-.5-1.3 0-1.1.9-2 2-2h2.3c1.98 0 3.7-1.6 3.7-3.6 0-4.86-4.07-8.8-9-8.8z" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
    git: `<circle cx="7" cy="7" r="2.1" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="7" cy="17" r="2.1" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="17" cy="9" r="2.1" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7 9.1v5.8M17 11.1c0 3-3 3.7-6.4 3.7" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
    formatter: `<rect x="3.5" y="3.5" width="17" height="17" rx="3.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7.5 8.5h9M7.5 12h6M7.5 15.5h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>`,
    linter: `<path d="M12 3l8 3.6v5c0 4.3-3.4 7.5-8 8.9-4.6-1.4-8-4.6-8-8.9v-5L12 3z" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M9 12l2 2 4-4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>`,
    language: `<polyline points="15 17 20 12 15 7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/><polyline points="9 7 4 12 9 17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>`,
    docker: `<rect x="2.5" y="10" width="19" height="9" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M6 10V6.5a2 2 0 012-2h8a2 2 0 012 2V10" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
    ai: `<circle cx="12" cy="12" r="3.1" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M12 2.6v3M12 18.4v3M2.6 12h3M18.4 12h3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>`,
    web: `<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M3 12h18M12 3a14 14 0 010 18M12 3a14 14 0 000 18" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
    default: `<path d="M20 16.5v-9a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 002 7.5v9a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0020 16.5z" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M3.3 7.5L12 12.5l8.7-5M12 22V12.5" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
  };

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
    { id: "all", label: "All" }, { id: "featured", label: "Featured" },
    { id: "languages", label: "Languages" }, { id: "themes", label: "Themes" },
    { id: "tools", label: "Tools" }, { id: "ai", label: "AI" },
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
      label: "Tools",
      items: [
        { label: "Run Current File", icon: "i-terminal", hint: "⌘R", action: () => runCurrentFile() },
        { label: "Task Runner", icon: "i-play", action: () => openFeaturePanel("tasks") },
        { sep: true },
        { label: "Workspace Manager", icon: "i-folder", action: () => openFeaturePanel("workspace") },
        { label: "Remote Development", icon: "i-terminal", action: () => openFeaturePanel("remote") },
        { label: "Extension Marketplace", icon: "i-ext", action: () => openFeaturePanel("marketplace") },
        { label: "Merge Conflicts", icon: "i-git", action: () => openFeaturePanel("conflicts") },
        { sep: true },
        { label: "Debugger", icon: "i-code", action: () => openFeaturePanel("debugger") },
        { label: "Language Servers", icon: "i-code", action: () => openFeaturePanel("lsp") },
        { sep: true },
        { label: "Settings", icon: "i-gear", action: () => openFeaturePanel("settings") },
        { label: "Keyboard Shortcuts", icon: "i-command", action: () => openFeaturePanel("shortcuts") },
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
        { label: t("theme.light"), icon: "i-sparkle", action: () => setTheme("light") },
        { label: t("theme.dark"), icon: "i-sparkle", action: () => setTheme("dark") },
        { label: "Monokai", icon: "i-sparkle", action: () => setTheme("monokai") },
        { label: "GitHub Light", icon: "i-sparkle", action: () => setTheme("github-light") },
        { label: "Solarized Dark", icon: "i-sparkle", action: () => setTheme("solarized-dark") },
        { label: "Nord", icon: "i-sparkle", action: () => setTheme("nord") },
        { label: t("theme.system"), icon: "i-sparkle", action: () => setTheme("system") },
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
promptEl.addEventListener("input", () => {
  promptEl.style.height = "auto";
  promptEl.style.height = Math.min(promptEl.scrollHeight, 160) + "px";
});
$("composer").addEventListener("submit", (e) => {
  e.preventDefault();
  const text = promptEl.value.trim();
  if (!text) return;
  promptEl.value = "";
  promptEl.style.height = "auto";
  sendPrompt(text);
});
promptEl.addEventListener("keydown", (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
    e.preventDefault();
    $("composer").requestSubmit();
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
        await backend.writeTextFile(activePath, f.model.getValue());
        markDirty(activePath, false);
      } catch { /* silent */ }
    }
  }, 1500);
}

monacoEditor.onDidChangeModelContent(() => {
  scheduleAutoSave();
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
  "view.problems": () => toggleProblems(),
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
  if (activeTermTab >= 0 && termTabs[activeTermTab]) {
    termTabs[activeTermTab].container.hidden = true;
  }
  activeTermTab = idx;
  const tab = termTabs[idx];
  tab.container.hidden = false;
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
    scrollback: 10000,
    allowProposedApi: true,
    drawBoldTextInBrightColors: false,
    minimumContrastRatio: 4.5,
  });
  const fit = new FitAddon();
  term.loadAddon(fit);
  term.open(container);
  termResizeObserver.observe(container);

  let initDone = false;
  let initBuffer = "";
  const entry = { term, fit, container, label, cwd, backendId: null, opening: false, inputLine: "", ghost: "", suggestion: "" };
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
        } else if (ev.kind === "exit") {
          term.write("\r\n\x1b[2m[process exited]\x1b[0m\r\n");
          entry.backendId = null;
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

monacoEditor.onDidChangeCursorPosition(() => updateStatusBar());
monacoEditor.onDidChangeCursorSelection(() => updateStatusBar());
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
    { id: "marketplace.open", title: "Extension Marketplace", category: "Tools", run: () => openFeaturePanel("marketplace") },
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

$("extensionsBtn").addEventListener("click", () => extPanel.open());
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
  for (const path of list) {
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
    if (!session) return;
    if (Array.isArray(session.workspaceRoots) && session.workspaceRoots.length) {
      workspaceRoots = session.workspaceRoots;
      setActiveWorkspaceRoot(session.rootPath || workspaceRoots[0]);
      await renderWorkspaceRoots();
      startFileWatcher();
      await refreshGitStatus();
      for (const root of workspaceRoots) preloadProjectModels(root);
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