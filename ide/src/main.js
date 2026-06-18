// Michael IDE — editor + AI assistant orchestration.
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

async function tauriBackend() {
  const core = await import("@tauri-apps/api/core");
  const dialog = await import("@tauri-apps/plugin-dialog");
  return {
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
  const MARKETPLACE = [
    {
      id: "michael.theme-pack",
      name: "Michael Theme Pack",
      version: "1.2.0",
      description: "A curated set of editor themes for Michael IDE.",
      author: "Michael Labs",
      download_url: "https://example.com/michael-theme-pack.zip",
      tags: ["theme", "ui"],
      downloads: 12840,
    },
    {
      id: "michael.git-tools",
      name: "Git Tools",
      version: "0.4.1",
      description: "Extra source-control commands and status-bar actions.",
      author: "Michael Labs",
      download_url: "https://example.com/git-tools.zip",
      tags: ["git", "productivity"],
      downloads: 7331,
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
      onEvent?.({ kind: "started", adapter: config.adapterId });
    },
    dapSend: async () => {},
    dapStop: async (adapterId) => {
      mockDapRunning.delete(adapterId);
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
  fontSize: 13,
  fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
  renderWhitespace: "selection",
  padding: { top: 10 },
});

let splitEditor = null;
const editorContainer = $("editorContainer");

function toggleSplitEditor() {
  if (splitEditor) {
    splitEditor.dispose();
    const splitDiv = document.getElementById("editorSplit");
    if (splitDiv) splitDiv.remove();
    splitEditor = null;
    return;
  }
  if (!activePath) return;
  const splitDiv = document.createElement("div");
  splitDiv.id = "editorSplit";
  splitDiv.className = "editor-split";
  editorContainer.appendChild(splitDiv);
  const f = openFiles.get(activePath);
  if (!f) return;
  splitEditor = monaco.editor.create(splitDiv, {
    model: f.model,
    theme: matchMedia("(prefers-color-scheme: dark)").matches ? "vs-dark" : "vs",
    automaticLayout: true,
    fontSize: 13,
    fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    renderWhitespace: "selection",
    padding: { top: 10 },
  });
}

const EDITOR_PREFS_KEY = "editor-prefs";
let _editorPrefs = null;

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
  const store = await getStore();
  await store.set(EDITOR_PREFS_KEY, _editorPrefs);
  await store.save();
}

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
function getOrCreateModel(path, name, content) {
  const uri = monaco.Uri.file(path);
  let model = monaco.editor.getModel(uri);
  if (model) {
    if (content != null && model.getValue() !== content) model.setValue(content);
    return model;
  }
  model = monaco.editor.createModel(content ?? "", extLang(name), uri);
  model.onDidChangeContent(() => markDirty(path, true));
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

async function openFile(path, name) {
  if (openFiles.has(path)) {
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
  return true;
}

function activate(path) {
  closeDiffView();
  if (activePath && openFiles.has(activePath)) {
    openFiles.get(activePath).viewState = monacoEditor.saveViewState();
  }
  activePath = path;
  const f = openFiles.get(path);
  monacoEditor.setModel(f.model);
  if (f.viewState) monacoEditor.restoreViewState(f.viewState);
  monacoEditor.focus();
  syncWelcome();
  renderTabs();
  renderTreeActive();
  saveBtn.disabled = !f.dirty;
  if (runBtn) runBtn.disabled = false;
  $("windowTitle").textContent = f.name + " — Michael IDE";
  refreshGutter();
  updateBreadcrumb(path);
}

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

function closeFile(path) {
  const f = openFiles.get(path);
  if (!f) return;
  // Keep project models alive so language features still resolve this file.
  if (!projectModels.has(path)) f.model.dispose();
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
    showToast(t("file.saved", { name: f.name }));
  } catch (e) {
    showToast(String(e));
  }
}

let dragSrcPath = null;

function renderTabs() {
  tabsEl.innerHTML = "";
  for (const [path, f] of openFiles) {
    const tab = document.createElement("div");
    tab.className = "tab" + (path === activePath ? " is-active" : "") + (f.dirty ? " dirty" : "");
    tab.draggable = true;
    tab.dataset.path = path;
    tab.innerHTML =
      `${iconImg(fileIconUrl(f.name))}<span class="label"></span>` +
      `<span class="x" title="Close"><span class="dot"></span><svg class="ic"><use href="#i-close" /></svg></span>`;
    tab.querySelector(".label").textContent = f.name;
    tab.addEventListener("click", () => activate(path));
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
  await renderWorkspaceRoots();
  preloadProjectModels(path);
  await refreshGitStatus();
}

async function addFolderToWorkspace() {
  const picked = await backend.pickFolder();
  if (!picked) return;
  if (!workspaceRoots.includes(picked)) workspaceRoots.push(picked);
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
    mi.innerHTML = `<svg class="ic"><use href="#${it.icon}" /></svg><span class="name"></span>`;
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
  $("tabExplorer").classList.toggle("is-active", which === "explorer" || which === "search");
  $("tabGit").classList.toggle("is-active", which === "git");
  $("tabOutline")?.classList.toggle("is-active", which === "outline");
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
  }
}

// ---- outline panel ----
const outlineListEl = $("outlineList");

async function refreshOutline() {
  if (!outlineListEl) return;
  outlineListEl.innerHTML = "";
  if (!activePath) {
    const empty = document.createElement("div");
    empty.className = "outline-empty";
    empty.textContent = t("outline.noFile");
    outlineListEl.appendChild(empty);
    return;
  }
  const model = monacoEditor.getModel();
  if (!model) return;

  let symbols;
  try {
    symbols = await monaco.languages.getLanguages();
    const docSymbols = await getDocumentSymbols(model);
    if (!docSymbols || !docSymbols.length) {
      const empty = document.createElement("div");
      empty.className = "outline-empty";
      empty.textContent = t("outline.noSymbols");
      outlineListEl.appendChild(empty);
      return;
    }
    renderOutlineSymbols(docSymbols, outlineListEl, 0);
  } catch {
    const empty = document.createElement("div");
    empty.className = "outline-empty";
    empty.textContent = t("outline.noSymbols");
    outlineListEl.appendChild(empty);
  }
}

async function getDocumentSymbols(model) {
  const providers = monaco.languages.DocumentSymbolProviderRegistry?.all?.(model);
  if (!providers || !providers.length) {
    return fallbackSymbols(model);
  }
  try {
    const result = await providers[0].provideDocumentSymbols(model);
    return result && result.length ? result : fallbackSymbols(model);
  } catch {
    return fallbackSymbols(model);
  }
}

function fallbackSymbols(model) {
  const symbols = [];
  const text = model.getValue();
  const lines = text.split("\n");
  const funcRegex = /(?:export\s+)?(?:async\s+)?(?:function\s+(\w+)|(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s*)?\(?|class\s+(\w+))/;
  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(funcRegex);
    if (m) {
      const name = m[1] || m[2] || m[3];
      const kind = m[3] ? 4 : (m[1] ? 11 : 5);
      symbols.push({
        name,
        kind,
        range: { startLineNumber: i + 1, startColumn: 1, endLineNumber: i + 1, endColumn: lines[i].length + 1 },
        children: [],
      });
    }
  }
  return symbols;
}

const SYMBOL_ICONS = {
  4: "i-code",  // Class
  5: "i-code",  // Method
  11: "i-code", // Function
  12: "i-code", // Variable
  13: "i-code", // Constant
  1: "i-file",  // File
  2: "i-folder", // Module
  6: "i-code",  // Property
  7: "i-code",  // Field
};

function renderOutlineSymbols(symbols, container, depth) {
  for (const sym of symbols) {
    const item = document.createElement("div");
    item.className = "outline-item";
    item.style.paddingLeft = (12 + depth * 14) + "px";
    const iconId = SYMBOL_ICONS[sym.kind] || "i-code";
    item.innerHTML = `<svg class="ic"><use href="#${iconId}" /></svg><span class="outline-name"></span>`;
    item.querySelector(".outline-name").textContent = sym.name;
    item.addEventListener("click", () => {
      const line = sym.range?.startLineNumber || sym.selectionRange?.startLineNumber || 1;
      monacoEditor.revealLineInCenter(line);
      monacoEditor.setPosition({ lineNumber: line, column: 1 });
      monacoEditor.focus();
    });
    container.appendChild(item);
    if (sym.children && sym.children.length) {
      renderOutlineSymbols(sym.children, container, depth + 1);
    }
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
    entries = await backend.gitLog(rootPath, 30);
  } catch {
    return;
  }
  logEl.innerHTML = "";
  for (const e of entries) {
    const row = document.createElement("div");
    row.className = "git-log-row";
    row.innerHTML = `<span class="git-log-hash"></span><span class="git-log-msg"></span><span class="git-log-meta"></span>`;
    row.querySelector(".git-log-hash").textContent = e.short_hash;
    row.querySelector(".git-log-msg").textContent = e.message;
    row.querySelector(".git-log-meta").textContent = `${e.author} · ${e.date}`;
    row.title = `${e.hash}\n${e.author} · ${e.date}\n${e.message}`;
    logEl.appendChild(row);
  }
}

$("gitLogToggle")?.addEventListener("click", () => {
  const logEl = $("gitLog");
  if (logEl) logEl.hidden = !logEl.hidden;
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
  const use = avatar.querySelector("use");
  const id = currentModel();
  if (!id) {
    avatar.className = "assistant__avatar";
    use.setAttribute("href", "#i-sparkle");
    nameEl.textContent = t("assistant.name");
    return;
  }
  const b = brandOf(id);
  avatar.className = "assistant__avatar" + (b.cls ? " " + b.cls : "");
  use.setAttribute("href", "#" + b.sym);
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
    const b = brandOf(id);
    const sym = id ? b.sym : "i-sparkle";
    const avatar = document.createElement("div");
    avatar.className = "msg__avatar" + (id && b.cls ? " " + b.cls : "");
    avatar.innerHTML = `<svg class="ic"><use href="#${sym}" /></svg>`;
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

// Devin-style "thinking" card shown while the first token is pending. The orb
// matches the active model's provider so it feels like that model is replying.
function thinkingCard(brand) {
  const t = document.createElement("div");
  t.className = "thinking";
  const orbCls = brand && brand.cls ? "thinking__orb " + brand.cls : "thinking__orb";
  const sym = brand && brand.sym && brand.sym !== "i-cpu" ? brand.sym : "i-sparkle";
  t.innerHTML =
    `<span class="${orbCls}"><svg class="ic"><use href="#${sym}" /></svg></span>` +
    `<span class="thinking__text">${t("assistant.thinking")}</span>`;
  return t;
}

function showChatHint() {
  if (chatEl.children.length) return;
  const hint = document.createElement("div");
  hint.className = "chat-empty";
  hint.innerHTML =
    `<div class="chat-empty__icon"><svg class="ic"><use href="#i-monogram" /></svg></div>` +
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
  body.appendChild(thinkingCard(brandOf(currentModel())));
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
      if (!err) history.push({ role: "assistant", content: acc });
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

// ---- advanced feature panels (workspace / remote / marketplace / debug) ----
const FEATURE_TABS = [
  { id: "workspace", title: "Workspace", icon: "i-folder" },
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
  const renderers = {
    workspace: renderWorkspaceTool,
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
  createToolHeader(body, "Extension Marketplace", "Search the online registry and install extensions directly into Michael IDE.");
  const box = document.createElement("div");
  box.className = "tool-search";
  box.innerHTML = `<input spellcheck="false" placeholder="Search extensions…" /><button class="btn" type="button">Search</button>`;
  const list = document.createElement("div");
  list.className = "tool-list";
  body.append(box, list);

  const load = async () => {
    list.innerHTML = "";
    list.appendChild(createEmptyState("Loading marketplace…"));
    try {
      const query = box.querySelector("input").value.trim();
      const entries = query ? await backend.marketplaceSearch(query) : await backend.marketplaceList();
      list.innerHTML = "";
      if (!entries.length) list.appendChild(createEmptyState("No extensions found."));
      for (const entry of entries) {
        const card = document.createElement("div");
        card.className = "tool-card";
        card.innerHTML = `
          <div class="tool-card__main">
            <strong></strong>
            <span></span>
            <div class="tool-tags"></div>
          </div>
          <button class="btn btn--primary" type="button">Install</button>`;
        card.querySelector("strong").textContent = `${entry.name} ${entry.version}`;
        card.querySelector("span").textContent = `${entry.description} · ${entry.author} · ${entry.downloads || 0} downloads`;
        card.querySelector(".tool-tags").textContent = (entry.tags || []).join("  ");
        card.querySelector("button").addEventListener("click", async () => {
          try {
            const msg = await backend.marketplaceInstall(entry);
            showToast(msg);
            extPanel.refresh?.();
          } catch (e) {
            showToast(String(e && e.message ? e.message : e));
          }
        });
        list.appendChild(card);
      }
    } catch (e) {
      list.innerHTML = "";
      list.appendChild(createEmptyState(String(e && e.message ? e.message : e)));
    }
  };

  box.querySelector("button").addEventListener("click", load);
  box.querySelector("input").addEventListener("keydown", (e) => {
    if (e.key === "Enter") load();
  });
  load();
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

function renderDebuggerTool(body) {
  createToolHeader(body, "Debugger", "Start and stop DAP adapters. Custom command and args are supported when a default adapter is not enough.");
  const form = document.createElement("div");
  form.className = "tool-form";
  form.innerHTML = `
    <label><span>Adapter</span><select id="dapAdapter"><option>node</option><option>python</option><option>lldb</option><option>go</option></select></label>
    <label><span>Custom command</span><input id="dapCommand" spellcheck="false" placeholder="leave empty for default adapter" /></label>
    <label><span>Args</span><input id="dapArgs" spellcheck="false" placeholder="space separated" /></label>
    <label><span>Working directory</span><input id="dapCwd" spellcheck="false" /></label>
    <div class="tool-actions"><button class="btn btn--primary" id="dapStartBtn" type="button">Start</button><button class="btn" id="dapStopBtn" type="button">Stop</button><button class="btn" id="dapRefreshBtn" type="button">Refresh</button></div>
    <div class="tool-list" id="dapList"></div>
    <pre class="tool-log" id="dapLog"></pre>`;
  body.appendChild(form);
  form.querySelector("#dapCwd").value = rootPath || "";
  const log = form.querySelector("#dapLog");
  const refresh = async () => {
    const list = form.querySelector("#dapList");
    list.innerHTML = "";
    try {
      const adapters = await backend.dapList();
      for (const item of adapters) {
        const row = document.createElement("div");
        row.className = "tool-card";
        row.innerHTML = `<div class="tool-card__main"><strong></strong><span></span></div>`;
        row.querySelector("strong").textContent = item.adapter;
        row.querySelector("span").textContent = item.running ? "Running" : "Stopped";
        list.appendChild(row);
      }
    } catch (e) {
      list.appendChild(createEmptyState(String(e && e.message ? e.message : e)));
    }
  };
  form.querySelector("#dapStartBtn").addEventListener("click", async () => {
    const adapterId = form.querySelector("#dapAdapter").value;
    const command = form.querySelector("#dapCommand").value.trim();
    const args = form.querySelector("#dapArgs").value.trim().split(/\s+/).filter(Boolean);
    const cwd = form.querySelector("#dapCwd").value.trim() || null;
    try {
      await backend.dapStart({ adapterId, command, args, cwd }, (ev) => {
        log.textContent += JSON.stringify(ev) + "\n";
        log.scrollTop = log.scrollHeight;
      });
      showToast(`Debugger ${adapterId} started`);
      refresh();
    } catch (e) {
      showToast(String(e && e.message ? e.message : e));
    }
  });
  form.querySelector("#dapStopBtn").addEventListener("click", async () => {
    const adapterId = form.querySelector("#dapAdapter").value;
    try {
      await backend.dapStop(adapterId);
      showToast(`Debugger ${adapterId} stopped`);
      refresh();
    } catch (e) {
      showToast(String(e && e.message ? e.message : e));
    }
  });
  form.querySelector("#dapRefreshBtn").addEventListener("click", refresh);
  refresh();
}

function renderLspTool(body) {
  createToolHeader(body, "Language Servers", "Start and stop LSP servers for languages that need external intelligence beyond Monaco's built-in services.");
  const form = document.createElement("div");
  form.className = "tool-form";
  form.innerHTML = `
    <label><span>Language</span><select id="lspLang"><option>typescript</option><option>javascript</option><option>rust</option><option>python</option><option>go</option><option>html</option><option>css</option><option>json</option></select></label>
    <label><span>Custom command</span><input id="lspCommand" spellcheck="false" placeholder="leave empty for default server" /></label>
    <label><span>Args</span><input id="lspArgs" spellcheck="false" placeholder="space separated" /></label>
    <div class="tool-actions"><button class="btn btn--primary" id="lspStartBtn" type="button">Start</button><button class="btn" id="lspStopBtn" type="button">Stop</button><button class="btn" id="lspRefreshBtn" type="button">Refresh</button></div>
    <div class="tool-list" id="lspList"></div>
    <pre class="tool-log" id="lspLog"></pre>`;
  body.appendChild(form);
  const log = form.querySelector("#lspLog");
  const refresh = async () => {
    const list = form.querySelector("#lspList");
    list.innerHTML = "";
    try {
      const servers = await backend.lspList();
      for (const item of servers) {
        const row = document.createElement("div");
        row.className = "tool-card";
        row.innerHTML = `<div class="tool-card__main"><strong></strong><span></span></div>`;
        row.querySelector("strong").textContent = item.lang;
        row.querySelector("span").textContent = item.running ? "Running" : "Stopped";
        list.appendChild(row);
      }
    } catch (e) {
      list.appendChild(createEmptyState(String(e && e.message ? e.message : e)));
    }
  };
  form.querySelector("#lspStartBtn").addEventListener("click", async () => {
    const lang = form.querySelector("#lspLang").value;
    const command = form.querySelector("#lspCommand").value.trim();
    const args = form.querySelector("#lspArgs").value.trim().split(/\s+/).filter(Boolean);
    const rootUri = rootPath ? `file://${rootPath}` : "";
    try {
      await backend.lspStart({ lang, command, args, rootUri }, (ev) => {
        log.textContent += JSON.stringify(ev) + "\n";
        log.scrollTop = log.scrollHeight;
      });
      showToast(`LSP ${lang} started`);
      refresh();
    } catch (e) {
      showToast(String(e && e.message ? e.message : e));
    }
  });
  form.querySelector("#lspStopBtn").addEventListener("click", async () => {
    const lang = form.querySelector("#lspLang").value;
    try {
      await backend.lspStop(lang);
      showToast(`LSP ${lang} stopped`);
      refresh();
    } catch (e) {
      showToast(String(e && e.message ? e.message : e));
    }
  });
  form.querySelector("#lspRefreshBtn").addEventListener("click", refresh);
  refresh();
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
        { sep: true },
        { label: "Workspace Manager", icon: "i-folder", action: () => openFeaturePanel("workspace") },
        { label: "Remote Development", icon: "i-terminal", action: () => openFeaturePanel("remote") },
        { label: "Extension Marketplace", icon: "i-ext", action: () => openFeaturePanel("marketplace") },
        { label: "Merge Conflicts", icon: "i-git", action: () => openFeaturePanel("conflicts") },
        { sep: true },
        { label: "Debugger", icon: "i-code", action: () => openFeaturePanel("debugger") },
        { label: "Language Servers", icon: "i-code", action: () => openFeaturePanel("lsp") },
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
$("settingsBtn").addEventListener("click", openSettings);
$("saveBtn").addEventListener("click", saveActive);
$("runBtn")?.addEventListener("click", runCurrentFile);

// ---- explorer tabs / tools / search ----
$("tabExplorer").addEventListener("click", () => showSide("explorer"));
$("tabGit").addEventListener("click", () => showSide("git"));
$("tabOutline")?.addEventListener("click", () => showSide("outline"));
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
  return { ...DEFAULT_KEYBINDINGS, ...userKeybindings };
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
const editorwrapEl = document.querySelector(".editorwrap");

function termTheme() {
  const isDark = document.documentElement.dataset.theme === "dark" ||
    (document.documentElement.dataset.theme !== "light" && window.matchMedia("(prefers-color-scheme: dark)").matches);

  if (isDark) {
    return {
      background: "#1E1E1E", foreground: "#CCCCCC", cursor: "#AEAFAD",
      cursorAccent: "#1E1E1E", selectionBackground: "rgba(255,255,255,0.18)",
      selectionForeground: "#FFFFFF",
      black: "#000000", red: "#CD3131", green: "#0DBC79", yellow: "#E5E510",
      blue: "#2472C8", magenta: "#BC3FBC", cyan: "#11A8CD", white: "#E5E5E5",
      brightBlack: "#666666", brightRed: "#F14C4C", brightGreen: "#23D18B",
      brightYellow: "#F5F543", brightBlue: "#3B8EEA", brightMagenta: "#D670D6",
      brightCyan: "#29B8DB", brightWhite: "#F5F5F5",
    };
  }

  return {
    background: "#FFFFFF", foreground: "#383A42", cursor: "#526FFF",
    cursorAccent: "#FFFFFF", selectionBackground: "rgba(0,122,255,0.12)",
    selectionForeground: "#000000",
    black: "#383A42", red: "#E45649", green: "#50A14F", yellow: "#C18401",
    blue: "#4078F2", magenta: "#A626A4", cyan: "#0184BC", white: "#A0A1A7",
    brightBlack: "#4F525E", brightRed: "#E06C75", brightGreen: "#98C379",
    brightYellow: "#D19A66", brightBlue: "#61AFEF", brightMagenta: "#C678DD",
    brightCyan: "#56B6C2", brightWhite: "#FAFAFA",
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
  let tabBar = termPanel.querySelector(".term-tabs");
  if (!tabBar) {
    tabBar = document.createElement("div");
    tabBar.className = "term-tabs";
    const head = termPanel.querySelector(".terminal-panel__head");
    const titleSpan = head.querySelector(".terminal-panel__title");
    titleSpan.after(tabBar);
  }
  tabBar.innerHTML = "";
  termTabs.forEach((tab, i) => {
    const btn = document.createElement("button");
    btn.className = "term-tab" + (i === activeTermTab ? " is-active" : "");
    btn.type = "button";
    btn.innerHTML = `<span></span><span class="term-tab__x">&times;</span>`;
    btn.querySelector("span").textContent = tab.label;
    btn.addEventListener("click", (e) => {
      if (e.target.classList.contains("term-tab__x")) {
        closeTermTab(i);
      } else {
        switchTermTab(i);
      }
    });
    tabBar.appendChild(btn);
  });
  const addBtn = document.createElement("button");
  addBtn.className = "term-tab term-tab--add";
  addBtn.type = "button";
  addBtn.textContent = "+";
  addBtn.title = t("terminal.new");
  addBtn.addEventListener("click", () => createTermTab());
  tabBar.appendChild(addBtn);
}

function switchTermTab(idx) {
  if (idx === activeTermTab || idx < 0 || idx >= termTabs.length) return;
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
  const label = `${t("terminal.title")} ${++termSeq}`;
  const container = document.createElement("div");
  container.className = "terminal-panel__instance";
  container.hidden = activeTermTab >= 0;
  termBody.appendChild(container);

  const term = new Terminal({
    fontSize: 13,
    fontFamily: "'SF Mono', Menlo, ui-monospace, 'JetBrains Mono', Consolas, monospace",
    fontWeight: "normal",
    fontWeightBold: "bold",
    lineHeight: 1.35,
    letterSpacing: 0.3,
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

  let backendId = null;
  let initDone = false;
  let initBuffer = "";
  const entry = { term, fit, container, label, backendId, opening: false };
  termTabs.push(entry);

  term.onData((d) => { if (entry.backendId != null) backend.termWrite(entry.backendId, d); });
  term.onResize(({ cols, rows }) => { if (entry.backendId != null) backend.termResize(entry.backendId, cols, rows); });

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

  if (termTabs.length === 0) {
    await createTermTab();
  } else {
    switchTermTab(activeTermTab);
  }
}

function closeTerminal() {
  if (!termIsOpen()) return;
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
$("termTrafficClose")?.addEventListener("click", closeTerminal);
$("terminalBtn")?.addEventListener("click", toggleTerminal);
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

// ---- extensions ----
const statusbarRight = $("statusbarRight");
const statusItems = new Map();

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
    { id: "view.extensions", title: t("ext.title"), category: t("menu.view"), run: () => extPanel.open() },
    { id: "view.terminal", title: t("menu.toggleTerminal"), category: t("menu.view"), run: () => toggleTerminal() },
    { id: "terminal.new", title: t("terminal.new"), category: t("terminal.title"), run: () => { openTerminal(); createTermTab(); } },
    { id: "view.splitEditor", title: "Toggle Split Editor", category: t("menu.view"), run: () => toggleSplitEditor() },
    { id: "remote.open", title: "Remote Development", category: "Tools", run: () => openFeaturePanel("remote") },
    { id: "marketplace.open", title: "Extension Marketplace", category: "Tools", run: () => openFeaturePanel("marketplace") },
    { id: "git.conflicts", title: "Resolve Merge Conflicts", category: "Tools", run: () => openFeaturePanel("conflicts") },
    { id: "debug.open", title: "Debugger", category: "Tools", run: () => openFeaturePanel("debugger") },
    { id: "lsp.open", title: "Language Servers", category: "Tools", run: () => openFeaturePanel("lsp") },
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
  loadEditorPrefs().then((prefs) => {
    if (prefs.theme) { currentTheme = prefs.theme; applyEditorTheme(); }
    if (prefs.autoSave !== undefined) autoSaveEnabled = prefs.autoSave;
    if (prefs.fontSize) monacoEditor.updateOptions({ fontSize: prefs.fontSize });
    if (prefs.wordWrap) monacoEditor.updateOptions({ wordWrap: prefs.wordWrap });
  }),
]).catch(console.error);
showChatHint();
syncWelcome();
