// Devin IDE — editor + AI assistant orchestration.
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import { renderMarkdownInto } from "./markdown.js";

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
const backend = inTauri ? await tauriBackend() : mockBackend();

async function tauriBackend() {
  const core = await import("@tauri-apps/api/core");
  const dialog = await import("@tauri-apps/plugin-dialog");
  return {
    readDir: (path) => core.invoke("read_dir", { path }),
    readTextFile: (path) => core.invoke("read_text_file", { path }),
    writeTextFile: (path, content) => core.invoke("write_text_file", { path, content }),
    homeDir: () => core.invoke("home_dir"),
    pickFolder: () => dialog.open({ directory: true, multiple: false }),
    aiChat: (config, messages, onEvent) => {
      const channel = new core.Channel();
      channel.onmessage = onEvent;
      return core.invoke("ai_chat", { config, messages, onEvent: channel });
    },
  };
}

function mockBackend() {
  const FS = {
    "/Users/andrew/my-app": [
      { name: "src", path: "/Users/andrew/my-app/src", is_dir: true },
      { name: "README.md", path: "/Users/andrew/my-app/README.md", is_dir: false },
      { name: "package.json", path: "/Users/andrew/my-app/package.json", is_dir: false },
    ],
    "/Users/andrew/my-app/src": [
      { name: "main.js", path: "/Users/andrew/my-app/src/main.js", is_dir: false },
      { name: "styles.css", path: "/Users/andrew/my-app/src/styles.css", is_dir: false },
    ],
  };
  const FILES = {
    "/Users/andrew/my-app/README.md": "# my-app\n\nA sample project shown in the browser preview.\n",
    "/Users/andrew/my-app/package.json": '{\n  "name": "my-app",\n  "version": "1.0.0"\n}\n',
    "/Users/andrew/my-app/src/main.js":
      'function greet(name) {\n  return `Hello, ${name}!`;\n}\n\nconsole.log(greet("world"));\n',
    "/Users/andrew/my-app/src/styles.css": "body {\n  margin: 0;\n  font-family: sans-serif;\n}\n",
  };
  return {
    readDir: async (path) => FS[path] ?? [],
    readTextFile: async (path) => FILES[path] ?? "",
    writeTextFile: async (path, content) => {
      FILES[path] = content;
    },
    homeDir: async () => "/Users/andrew",
    pickFolder: async () => "/Users/andrew/my-app",
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
        `console.log(greet("Devin")); // "Hello, Devin!"`,
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
  $("windowTitle").textContent = f.name + " — Devin IDE";
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
      $("windowTitle").textContent = "Devin IDE";
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
    const fi = fileIcon(f.name);
    tab.innerHTML =
      `${iconSvg(fi.id, fi.cls)}<span class="label"></span>` +
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

/** Map a filename to an SVG glyph id + a color class. */
function fileIcon(name) {
  const ext = name.split(".").pop().toLowerCase();
  const code = {
    js: "js", jsx: "js", mjs: "js", cjs: "js", ts: "ts", tsx: "ts",
    rs: "rust", py: "py", go: "go", java: "java", c: "c", h: "c", cpp: "cpp",
    hpp: "cpp", cc: "cpp", rb: "ruby", php: "php", swift: "swift", kt: "kotlin",
    sh: "shell", bash: "shell",
  };
  const markup = { html: "html", htm: "html", xml: "html", svg: "img", vue: "html" };
  const style = { css: "style", scss: "style", less: "style", sass: "style" };
  const data = { json: "data", yml: "data", yaml: "data", toml: "data", ini: "data", sql: "data", lock: "data" };
  const doc = { md: "doc", markdown: "doc", txt: "doc", rst: "doc" };
  const image = { png: "img", jpg: "img", jpeg: "img", gif: "img", webp: "img", ico: "img", svg: "img", avif: "img" };
  if (ext in code) return { id: "i-file-code", cls: "ic--" + code[ext] };
  if (ext in style) return { id: "i-file-style", cls: "ic--style" };
  if (ext in data) return { id: "i-file-data", cls: "ic--data" };
  if (ext in image) return { id: "i-file-image", cls: "ic--img" };
  if (ext in markup) return { id: "i-file-code", cls: "ic--html" };
  if (ext in doc) return { id: "i-file-doc", cls: "ic--doc" };
  return { id: "i-file", cls: "ic--doc" };
}

async function openFolder(path) {
  rootPath = path;
  rootNameEl.textContent = path.split("/").filter(Boolean).pop() || path;
  rootNameEl.title = path;
  treeEl.innerHTML = "";
  const container = document.createElement("div");
  treeEl.appendChild(container);
  await renderChildren(path, container);
}

async function renderChildren(path, container) {
  let entries;
  try {
    entries = await backend.readDir(path);
  } catch (e) {
    showToast(String(e));
    return;
  }
  for (const entry of entries) {
    const row = document.createElement("div");
    row.className = "row";
    row.dataset.path = entry.path;
    if (entry.is_dir) {
      row.innerHTML = `<svg class="chev"><use href="#i-chevron" /></svg>${iconSvg("i-folder", "ic--folder")}<span class="name"></span>`;
    } else {
      const fi = fileIcon(entry.name);
      row.innerHTML = `<span class="chev-spacer"></span>${iconSvg(fi.id, fi.cls)}<span class="name"></span>`;
    }
    row.querySelector(".name").textContent = entry.name;
    container.appendChild(row);

    if (entry.is_dir) {
      const kids = document.createElement("div");
      kids.className = "children";
      kids.hidden = true;
      let loaded = false;
      container.appendChild(kids);
      row.addEventListener("click", async () => {
        row.classList.toggle("open");
        kids.hidden = !kids.hidden;
        if (!loaded && !kids.hidden) {
          loaded = true;
          await renderChildren(entry.path, kids);
        }
      });
    } else {
      row.addEventListener("click", () => openFile(entry.path, entry.name));
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
      { id: "gpt-4o", meta: "Most capable" },
      { id: "gpt-4o-mini", meta: "Fast · cheap" },
      { id: "gpt-4.1", meta: "" },
      { id: "o3-mini", meta: "Reasoning" },
    ],
  },
  {
    label: "Anthropic",
    models: [
      { id: "claude-3-7-sonnet", meta: "" },
      { id: "claude-3-5-sonnet", meta: "" },
      { id: "claude-3-5-haiku", meta: "Fast" },
    ],
  },
  {
    label: "Local",
    models: [
      { id: "llama3.1", meta: "Ollama" },
      { id: "qwen2.5-coder", meta: "Ollama" },
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

function syncModelPicker() {
  const c = loadConfig();
  modelPickerLabel.textContent = c.model || "Select model";
  const b = brandOf(c.model);
  modelPickerBtnIcon.setAttribute("href", "#" + b.sym);
  modelPickerBtn.querySelector(".ic").setAttribute("class", "ic " + b.cls);
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
      item.querySelector(".name").textContent = m.id;
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
    const avatar = document.createElement("div");
    avatar.className = "msg__avatar";
    avatar.innerHTML = `<svg class="ic"><use href="#i-sparkle" /></svg>`;
    const main = document.createElement("div");
    main.className = "msg__main";
    main.innerHTML = `<span class="msg__who"><span>Devin</span></span><div class="msg__body"></div>`;
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

// Devin-style "thinking" card shown while the first token is pending.
function thinkingCard() {
  const t = document.createElement("div");
  t.className = "thinking";
  t.innerHTML =
    `<span class="thinking__orb"><svg class="ic"><use href="#i-sparkle" /></svg></span>` +
    `<span class="thinking__text">Thinking</span>`;
  return t;
}

function showChatHint() {
  if (chatEl.children.length) return;
  const hint = document.createElement("div");
  hint.className = "chat-empty";
  hint.innerHTML =
    `<div class="chat-empty__icon"><svg class="ic"><use href="#i-sparkle" /></svg></div>` +
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
    { role: "system", content: "You are Devin IDE's coding assistant. Be concise and precise. Use fenced code blocks for code." },
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

// ---- wiring ----
async function chooseFolder() {
  const picked = await backend.pickFolder();
  if (picked) await openFolder(picked);
}
$("openFolderBtn").addEventListener("click", chooseFolder);
$("emptyOpenBtn").addEventListener("click", chooseFolder);
$("settingsBtn").addEventListener("click", openSettings);
$("saveBtn").addEventListener("click", saveActive);

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
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
    e.preventDefault();
    saveActive();
  }
});

refreshModelBadge();
showChatHint();
syncWelcome();
