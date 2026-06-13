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

self.MonacoEnvironment = {
  getWorker(_id, label) {
    if (label === "json") return new jsonWorker();
    if (label === "css" || label === "scss" || label === "less") return new cssWorker();
    if (label === "html" || label === "handlebars" || label === "razor") return new htmlWorker();
    if (label === "typescript" || label === "javascript") return new tsWorker();
    return new editorWorker();
  },
};

// ---- backend abstraction (Tauri when available, mock in a plain browser) ----
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
// Reserve room for the macOS traffic-light buttons only when running natively.
if (inTauri) document.body.classList.add("is-tauri");
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
    [ROOT + "/components/Button.js"]:
      'export function Button(label) {\n  const el = document.createElement("button");\n  el.textContent = label;\n  return el;\n}\n',
    [ROOT + "/components/Card.js"]:
      'export function Card(title) {\n  const el = document.createElement("div");\n  el.className = "card";\n  el.textContent = title;\n  return el;\n}\n',
  };

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
matchMedia("(prefers-color-scheme: dark)").addEventListener("change", (e) => {
  monaco.editor.setTheme(e.matches ? "vs-dark" : "vs");
  if (term) term.options.theme = termTheme();
});

/** path -> { model, name, dirty, viewState } */
const openFiles = new Map();
let activePath = null;

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
    return;
  }
  let content;
  try {
    content = await backend.readTextFile(path);
  } catch (e) {
    showToast(String(e));
    return;
  }
  const model = monaco.editor.createModel(content, extLang(name));
  model.onDidChangeContent(() => markDirty(path, true));
  openFiles.set(path, { model, name, dirty: false, viewState: null });
  renderTabs();
  activate(path);
}

function activate(path) {
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
  $("windowTitle").textContent = f.name + " — Michael IDE";
}

function closeFile(path) {
  const f = openFiles.get(path);
  if (!f) return;
  f.model.dispose();
  openFiles.delete(path);
  if (activePath === path) {
    activePath = null;
    const next = [...openFiles.keys()].pop();
    if (next) activate(next);
    else {
      monacoEditor.setModel(monaco.editor.createModel("", "plaintext"));
      saveBtn.disabled = true;
      $("windowTitle").textContent = "Michael IDE";
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
    showToast("Saved " + f.name);
  } catch (e) {
    showToast(String(e));
  }
}

function renderTabs() {
  tabsEl.innerHTML = "";
  for (const [path, f] of openFiles) {
    const tab = document.createElement("div");
    tab.className = "tab" + (path === activePath ? " is-active" : "") + (f.dirty ? " dirty" : "");
    tab.innerHTML =
      `${iconImg(fileIconUrl(f.name))}<span class="label"></span>` +
      `<span class="x" title="Close"><span class="dot"></span><svg class="ic"><use href="#i-close" /></svg></span>`;
    tab.querySelector(".label").textContent = f.name;
    tab.addEventListener("click", () => activate(path));
    tab.querySelector(".x").addEventListener("click", (e) => {
      e.stopPropagation();
      closeFile(path);
    });
    tabsEl.appendChild(tab);
  }
}

// ---- file tree ----
let rootPath = null;

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

async function openFolder(path) {
  rootPath = path;
  rootNameEl.textContent = path.split("/").filter(Boolean).pop() || path;
  rootNameEl.title = path;
  dirNodes.clear();
  treeEl.innerHTML = "";
  rootContainer = document.createElement("div");
  treeEl.appendChild(rootContainer);
  setExplorerToolsEnabled(true);
  await renderChildren(path, rootContainer);
  renderTreeActive();
}

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
  if (path === rootPath) {
    dirNodes.clear();
    treeEl.innerHTML = "";
    rootContainer = document.createElement("div");
    treeEl.appendChild(rootContainer);
    await renderChildren(path, rootContainer);
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
    title: isDir ? "New Folder" : "New File",
    placeholder: isDir ? "folder name" : "file-name.ext",
    okLabel: "Create",
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
  const next = await ioPrompt({ title: "Rename", value: name, okLabel: "Rename" });
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
    title: "Delete " + (isDir ? "Folder" : "File"),
    message: `Are you sure you want to delete \u201C${name}\u201D? This cannot be undone.`,
    okLabel: "Delete",
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
      () => showToast("Copied path"),
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
    { label: "New File\u2026", icon: "i-new-file", action: () => newEntry(targetDir, false) },
    { label: "New Folder\u2026", icon: "i-new-folder", action: () => newEntry(targetDir, true) },
  ];
  if (!isRoot) {
    items.push(
      { sep: true },
      { label: "Rename\u2026", icon: "i-rename", action: () => renameEntry(entry.path, entry.name, isDir) },
      { label: "Delete", icon: "i-trash", danger: true, action: () => deleteEntry(entry.path, entry.name, isDir) },
    );
  }
  items.push({ sep: true }, { label: "Copy Path", icon: "i-copy", action: () => copyText(entry.path) });

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
    metaEl.textContent = "Open a folder to search.";
    return;
  }
  if (!query) {
    resultsEl.innerHTML = "";
    metaEl.textContent = "";
    return;
  }
  metaEl.textContent = "Searching\u2026";
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
    metaEl.textContent = "No results";
    return;
  }
  let total = 0;
  for (const f of files) total += f.matches.length;
  metaEl.textContent = `${total} result${total === 1 ? "" : "s"} in ${files.length} file${files.length === 1 ? "" : "s"}`;
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
  const isSearch = which === "search";
  $("viewExplorer").hidden = isSearch;
  $("viewSearch").hidden = !isSearch;
  $("tabExplorer").classList.toggle("is-active", !isSearch);
  $("tabSearch").classList.toggle("is-active", isSearch);
  const layout = document.querySelector(".layout");
  if (layout) layout.classList.remove("hide-explorer");
  if (isSearch) {
    const si = $("searchInput");
    si.focus();
    si.select();
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

// ---- AI assistant ----
const CFG_KEY = "devin-ide.ai-config";
function loadConfig() {
  try {
    return JSON.parse(localStorage.getItem(CFG_KEY)) || {};
  } catch {
    return {};
  }
}
function saveConfig(c) {
  localStorage.setItem(CFG_KEY, JSON.stringify(c));
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
  modelPickerLabel.textContent = c.model ? modelLabel(c.model) : "Select model";
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
    nameEl.textContent = "Assistant";
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
  cfg.innerHTML = `<svg class="ic"><use href="#i-gear" /></svg><span>Configure provider…</span>`;
  cfg.addEventListener("click", () => {
    closeModelMenu();
    openSettings();
  });
  modelMenu.appendChild(cfg);
}

function selectModel(model) {
  const c = loadConfig();
  saveConfig({ ...c, model });
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
    main.querySelector(".msg__who span").textContent = id ? modelLabel(id) : "Assistant";
    wrap.append(avatar, main);
    body = main.querySelector(".msg__body");
    if (text) renderMarkdownInto(body, text, { highlighter: highlightCode });
  } else {
    wrap.innerHTML = `<span class="msg__who"><span>You</span></span><div class="msg__body"></div>`;
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
    `<span class="thinking__text">Thinking</span>`;
  return t;
}

function showChatHint() {
  if (chatEl.children.length) return;
  const hint = document.createElement("div");
  hint.className = "chat-empty";
  hint.innerHTML =
    `<div class="chat-empty__icon"><svg class="ic"><use href="#i-monogram" /></svg></div>` +
    `<h3>Ask about your code</h3>` +
    `<p>The open file — and any text you select — is sent as context automatically.</p>` +
    `<div class="chat-empty__chips"></div>`;
  const chips = hint.querySelector(".chat-empty__chips");
  for (const s of ["Explain this file", "Find potential bugs", "Add doc comments", "Write a unit test"]) {
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
    showToast("Configure an AI provider first");
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
$("settingsForm").addEventListener("submit", (e) => {
  if (e.submitter && e.submitter.value === "save") {
    saveConfig({
      baseUrl: $("cfgBaseUrl").value.trim(),
      apiKey: $("cfgApiKey").value.trim(),
      model: $("cfgModel").value.trim(),
    });
    refreshModelBadge();
    showToast("AI settings saved");
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

const MENUS = [
  {
    label: "File",
    items: [
      { label: "Open Folder…", icon: "i-folder", hint: "⌘O", action: () => chooseFolder() },
      { label: "Save", icon: "i-save", hint: "⌘S", action: () => saveActive() },
      { sep: true },
      { label: "Close File", icon: "i-close", hint: "⌘W", action: () => activePath && closeFile(activePath) },
    ],
  },
  {
    label: "Edit",
    items: [
      { label: "Undo", icon: "i-undo", hint: "⌘Z", action: () => editorTrigger("undo") },
      { label: "Redo", icon: "i-redo", hint: "⇧⌘Z", action: () => editorTrigger("redo") },
      { sep: true },
      { label: "Find…", icon: "i-search", hint: "⌘F", action: () => editorAction("actions.find") },
      { label: "Replace…", icon: "i-replace", hint: "⌥⌘F", action: () => editorAction("editor.action.startFindReplaceAction") },
    ],
  },
  {
    label: "View",
    items: [
      { label: "Explorer", icon: "i-files", hint: "⇧⌘E", action: () => showSide("explorer") },
      { label: "Search", icon: "i-search", hint: "⇧⌘F", action: () => showSide("search") },
      { sep: true },
      { label: "Toggle Explorer", icon: "i-sidebar-left", action: () => togglePane("explorer") },
      { label: "Toggle Assistant", icon: "i-sidebar-right", action: () => togglePane("assistant") },
      { label: "Toggle Terminal", icon: "i-terminal", hint: "⌃`", action: () => toggleTerminal() },
      { sep: true },
      { label: "Command Palette…", icon: "i-command", hint: "⌘⇧P", action: () => editorAction("editor.action.quickCommand") },
    ],
  },
  {
    label: "Help",
    items: [
      { label: "Documentation", icon: "i-book", action: () => openExternal("https://github.com/fendoushaonian/Devin-Desktop") },
      { label: "AI Settings…", icon: "i-gear", action: () => openSettings() },
      { sep: true },
      { label: "About Michael IDE", icon: "i-info", action: () => showToast("Michael IDE — a macOS-style editor with a built-in AI assistant") },
    ],
  },
];

function buildMenubar() {
  const bar = $("menubar");
  if (!bar) return;
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

// ---- explorer tabs / tools / search ----
$("tabExplorer").addEventListener("click", () => showSide("explorer"));
$("tabSearch").addEventListener("click", () => showSide("search"));
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

window.addEventListener("keydown", (e) => {
  const mod = e.metaKey || e.ctrlKey;
  if (e.ctrlKey && e.key === "`") {
    e.preventDefault();
    toggleTerminal();
  } else if (mod && !e.shiftKey && e.key.toLowerCase() === "s") {
    e.preventDefault();
    saveActive();
  } else if (mod && e.shiftKey && e.key.toLowerCase() === "e") {
    e.preventDefault();
    showSide("explorer");
  } else if (mod && e.shiftKey && e.key.toLowerCase() === "f") {
    e.preventDefault();
    showSide("search");
  }
});

// ---- integrated terminal ----
let term = null;
let termFit = null;
let termId = null;
let termOpening = false;
const termPanel = $("terminalPanel");
const termBody = $("terminalBody");
const editorwrapEl = document.querySelector(".editorwrap");

function termTheme() {
  const dark = matchMedia("(prefers-color-scheme: dark)").matches;
  return dark
    ? { background: "#1e1e1e", foreground: "#d4d4d4", cursor: "#d4d4d4", selectionBackground: "#264f78" }
    : { background: "#ffffff", foreground: "#1f2328", cursor: "#1f2328", selectionBackground: "#b3d4fc" };
}

const termIsOpen = () => termPanel && !termPanel.hidden;

const termResizeObserver = new ResizeObserver(() => {
  if (termIsOpen() && termFit) {
    try {
      termFit.fit();
    } catch {
      /* container not measurable yet */
    }
  }
});

async function openTerminal() {
  if (!termPanel) return;
  if (termIsOpen()) {
    term?.focus();
    return;
  }
  termPanel.hidden = false;
  editorwrapEl?.classList.add("has-terminal");
  monacoEditor.layout();

  if (!term) {
    term = new Terminal({
      fontSize: 12,
      fontFamily: "SF Mono, ui-monospace, Menlo, monospace",
      theme: termTheme(),
      cursorBlink: true,
      scrollback: 5000,
    });
    termFit = new FitAddon();
    term.loadAddon(termFit);
    term.open(termBody);
    term.onData((d) => {
      if (termId != null) backend.termWrite(termId, d);
    });
    term.onResize(({ cols, rows }) => {
      if (termId != null) backend.termResize(termId, cols, rows);
    });
    termResizeObserver.observe(termBody);
  }

  requestAnimationFrame(() => {
    try {
      termFit.fit();
    } catch {
      /* ignore */
    }
  });

  if (termId == null && !termOpening) {
    termOpening = true;
    try {
      termId = await backend.termOpen(
        { cwd: rootPath || undefined, cols: term.cols, rows: term.rows },
        (ev) => {
          if (ev.kind === "data") term.write(ev.data);
          else if (ev.kind === "exit") {
            term.write("\r\n\x1b[2m[process exited — press ⌃` to reopen]\x1b[0m\r\n");
            termId = null;
          }
        },
      );
    } catch (err) {
      term.write("\r\n\x1b[31mFailed to start terminal: " + (err?.message || err) + "\x1b[0m\r\n");
    } finally {
      termOpening = false;
    }
  }
  term.focus();
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
$("terminalBtn")?.addEventListener("click", toggleTerminal);
// Clean up the backend shell process when the window goes away.
window.addEventListener("beforeunload", () => {
  if (termId != null) backend.termClose(termId);
});

buildMenubar();

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

const extHost = new ExtensionHost({
  getEditorText: () => monacoEditor.getModel()?.getValue() ?? "",
  getSelectionText: () => {
    const sel = monacoEditor.getSelection();
    const model = monacoEditor.getModel();
    return sel && model && !sel.isEmpty() ? model.getValueInRange(sel) : "";
  },
  insertText: insertAtCursor,
  showInformationMessage: (text) => showToast(text),
  setStatusBarItem,
  removeStatusBarItem,
  readFile: (path) => backend.readTextFile(path),
  writeFile: (path, content) => backend.writeTextFile(path, content),
});

const extManager = await createExtensionManager();
const extPanel = createExtensionsPanel({
  manager: extManager,
  host: extHost,
  showToast,
});

const palette = createCommandPalette({
  getCommands: () => [
    { id: "file.save", title: "Save File", category: "File", run: () => saveActive() },
    {
      id: "file.openFolder",
      title: "Open Folder\u2026",
      category: "File",
      run: () => chooseFolder(),
    },
    {
      id: "view.extensions",
      title: "Show Extensions",
      category: "View",
      run: () => extPanel.open(),
    },
    {
      id: "view.terminal",
      title: "Toggle Terminal",
      category: "View",
      run: () => toggleTerminal(),
    },
    {
      id: "ai.settings",
      title: "AI Provider Settings",
      category: "Preferences",
      run: () => openSettings(),
    },
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

refreshModelBadge();
showChatHint();
syncWelcome();
