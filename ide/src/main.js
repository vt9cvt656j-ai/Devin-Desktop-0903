// Devin IDE — editor + AI assistant orchestration.
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

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
    devinCreateSession: (config, prompt, title) =>
      core.invoke("devin_create_session", { config, prompt, title }),
    devinSendMessage: (config, sessionId, message) =>
      core.invoke("devin_send_message", { config, sessionId, message }),
    devinGetSession: (config, sessionId) =>
      core.invoke("devin_get_session", { config, sessionId }),
    openUrl: async (url) => {
      const opener = await import("@tauri-apps/plugin-opener");
      return opener.openUrl(url);
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
      const last = messages[messages.length - 1]?.content ?? "";
      const reply = `(preview mock) You said: "${last.slice(0, 80)}". Configure a real provider in settings to get live answers.`;
      for (const word of reply.split(" ")) {
        await new Promise((r) => setTimeout(r, 35));
        onEvent({ kind: "token", delta: word + " " });
      }
      onEvent({ kind: "done" });
    },
    devinCreateSession: async (_config, prompt) => {
      mockDevin = { id: "devin-mock", polls: 0, prompt };
      return { session_id: "devin-mock", url: "https://app.devin.ai/sessions/mock" };
    },
    devinSendMessage: async (_config, _sessionId, message) => {
      mockDevin = { id: "devin-mock", polls: 0, prompt: message };
    },
    devinGetSession: async (_config, sessionId) => {
      mockDevin.polls += 1;
      const done = mockDevin.polls >= 2;
      const messages = [{ type: "user_message", message: mockDevin.prompt, event_id: "u1" }];
      if (done) {
        messages.push({
          type: "devin_message",
          event_id: "d1",
          message:
            "(preview mock) This is where Devin's reply would stream in. Add your Devin API key in settings and run the desktop app to talk to a real session.",
        });
      }
      return {
        session_id: sessionId,
        status: done ? "finished" : "working",
        status_enum: done ? "finished" : "working",
        messages,
      };
    },
    openUrl: async (url) => window.open(url, "_blank"),
  };
}

let mockDevin = { id: null, polls: 0, prompt: "" };

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
  resetDevinSession(false);
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
  updateAssistantMode();
}

// ---- model picker (bottom-bar dropdown) ----
const MODEL_GROUPS = [
  {
    label: "Devin",
    models: [{ id: "devin", meta: "Agent · full session" }],
  },
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
  if (s === "devin") return { sym: "i-sparkle", cls: "brand--devin" };
  if (/^(gpt|o\d|chatgpt|text-|davinci)/.test(s)) return { sym: "i-brand-openai", cls: "brand--openai" };
  if (s.includes("claude")) return { sym: "i-brand-anthropic", cls: "brand--anthropic" };
  if (s.includes("llama")) return { sym: "i-brand-meta", cls: "brand--meta" };
  if (s.includes("qwen")) return { sym: "i-brand-qwen", cls: "brand--qwen" };
  return { sym: "i-cpu", cls: "" };
}

function isDevinMode() {
  return loadConfig().model === "devin";
}

function modelLabel(id) {
  return id === "devin" ? "Devin" : id || "Select model";
}

function syncModelPicker() {
  const c = loadConfig();
  modelPickerLabel.textContent = modelLabel(c.model);
  const b = brandOf(c.model);
  modelPickerBtnIcon.setAttribute("href", "#" + b.sym);
  modelPickerBtn.querySelector(".ic").setAttribute("class", "ic " + b.cls);
}

/** Reflect the active backend in the assistant header + composer hints. */
function updateAssistantMode() {
  const devin = isDevinMode();
  $("assistantName").textContent = devin ? "Devin" : "Assistant";
  $("newSessionBtn").hidden = !devin;
  promptEl.placeholder = devin
    ? "Ask Devin to work on your project…"
    : "Ask about the open file…";
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

function addMessage(role, text, who) {
  const wrap = document.createElement("div");
  wrap.className = "msg " + role;
  const whoIcon = role === "assistant" ? `<svg class="ic"><use href="#i-sparkle" /></svg>` : "";
  wrap.innerHTML = `<span class="msg__who">${whoIcon}<span></span></span><div class="msg__body"></div>`;
  const name = who || (role === "user" ? "You" : isDevinMode() ? "Devin" : "Assistant");
  wrap.querySelector(".msg__who span").textContent = name;
  const body = wrap.querySelector(".msg__body");
  body.textContent = text;
  chatEl.appendChild(wrap);
  chatEl.scrollTop = chatEl.scrollHeight;
  return body;
}

function showChatHint() {
  if (chatEl.children.length) return;
  const hint = document.createElement("div");
  hint.className = "hint";
  hint.textContent =
    "Ask the assistant about your code. The currently open file is sent as context automatically.";
  chatEl.appendChild(hint);
}

async function sendPrompt(text) {
  if (isDevinMode()) return sendDevinPrompt(text);
  const config = loadConfig();
  if (!config.baseUrl || !config.apiKey || !config.model) {
    openSettings();
    showToast("Configure an AI provider first");
    return;
  }
  if (streaming) return;
  chatEl.querySelector(".hint")?.remove();

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
  let acc = "";
  streaming = true;
  try {
    await backend.aiChat(config, messages, (ev) => {
      if (ev.kind === "token") {
        acc += ev.delta;
        body.textContent = acc;
        chatEl.scrollTop = chatEl.scrollHeight;
      } else if (ev.kind === "error") {
        body.textContent = "⚠️ " + ev.message;
      }
    });
  } catch (e) {
    if (!acc) body.textContent = "⚠️ " + String(e);
  } finally {
    streaming = false;
    if (acc) history.push({ role: "assistant", content: acc });
  }
}

// ---- Devin session backend ----
let devinSessionId = null;
let devinSessionUrl = null;
let devinBusy = false;
const devinSeen = new Set();
const TERMINAL = new Set(["finished", "blocked", "expired"]);

/** Open file + selection as context, appended to the very first prompt only. */
function projectContext() {
  const parts = [];
  if (rootPath) parts.push(`Project folder: ${rootPath}`);
  if (activePath) {
    const f = openFiles.get(activePath);
    const sel = monacoEditor.getModel() === f.model ? monacoEditor.getSelection() : null;
    const selected = sel && !sel.isEmpty() ? f.model.getValueInRange(sel) : "";
    parts.push(`Open file: ${activePath}\n\n\`\`\`\n${f.model.getValue().slice(0, 12000)}\n\`\`\``);
    if (selected) parts.push(`Selected text:\n\`\`\`\n${selected.slice(0, 4000)}\n\`\`\``);
  }
  return parts.join("\n\n");
}

function devinConfig() {
  const c = loadConfig();
  return { apiKey: (c.devinApiKey || "").trim(), baseUrl: (c.devinBaseUrl || "").trim() };
}

function devinStatusText(s) {
  const map = {
    working: "Devin is working…",
    blocked: "Devin is waiting for your reply.",
    finished: "Devin finished.",
    expired: "Session expired.",
    suspended: "Session suspended.",
  };
  return map[s] || `Devin: ${s}`;
}

function resetDevinSession(announce) {
  devinSessionId = null;
  devinSessionUrl = null;
  devinSeen.clear();
  if (announce && isDevinMode()) {
    chatEl.innerHTML = "";
    showChatHint();
  }
}

/** A status row with a live link to the running Devin session. */
function addDevinStatus() {
  const row = document.createElement("div");
  row.className = "devin-status";
  row.innerHTML = `<span class="devin-status__dot"></span><span class="devin-status__text"></span><a class="devin-status__link" target="_blank" hidden>View session ↗</a>`;
  const link = row.querySelector(".devin-status__link");
  link.addEventListener("click", (e) => {
    e.preventDefault();
    if (devinSessionUrl) backend.openUrl(devinSessionUrl);
  });
  chatEl.appendChild(row);
  chatEl.scrollTop = chatEl.scrollHeight;
  return {
    set(text, opts = {}) {
      row.querySelector(".devin-status__text").textContent = text;
      row.classList.toggle("is-done", !!opts.done);
      if (devinSessionUrl) link.hidden = false;
    },
  };
}

async function sendDevinPrompt(text) {
  const cfg = devinConfig();
  if (!cfg.apiKey) {
    openSettings();
    showToast("Add your Devin API key first");
    return;
  }
  if (devinBusy) return;
  chatEl.querySelector(".hint")?.remove();
  addMessage("user", text);
  const status = addDevinStatus();

  try {
    if (!devinSessionId) {
      status.set("Starting a Devin session…");
      const ctx = projectContext();
      const prompt = ctx ? `${text}\n\n---\nContext from my editor:\n${ctx}` : text;
      const ref = await backend.devinCreateSession(cfg, prompt, text.slice(0, 60));
      devinSessionId = ref.session_id;
      devinSessionUrl = ref.url;
      status.set("Devin is working…");
    } else {
      status.set("Sending to Devin…");
      await backend.devinSendMessage(cfg, devinSessionId, text);
      status.set("Devin is working…");
    }
    devinBusy = true;
    await pollDevin(cfg, status);
  } catch (e) {
    status.set("⚠️ " + String(e), { done: true });
  } finally {
    devinBusy = false;
  }
}

async function pollDevin(cfg, status) {
  const started = Date.now();
  const TIMEOUT_MS = 10 * 60 * 1000;
  while (true) {
    let session;
    try {
      session = await backend.devinGetSession(cfg, devinSessionId);
    } catch (e) {
      status.set("⚠️ " + String(e), { done: true });
      return;
    }
    for (const m of session.messages || []) {
      const id = m.event_id || `${m.type}:${m.timestamp}`;
      if (devinSeen.has(id)) continue;
      devinSeen.add(id);
      if (m.type === "devin_message" && m.message) addMessage("assistant", m.message, "Devin");
    }
    const state = session.status_enum || session.status;
    if (TERMINAL.has(state)) {
      status.set(devinStatusText(state), { done: true });
      return;
    }
    if (Date.now() - started > TIMEOUT_MS) {
      status.set("Still working — open the session to follow along.", { done: true });
      return;
    }
    status.set(devinStatusText(state));
    await new Promise((r) => setTimeout(r, 3000));
  }
}

// ---- settings dialog ----
const settingsEl = $("settings");
function openSettings() {
  const c = loadConfig();
  $("cfgDevinKey").value = c.devinApiKey || "";
  $("cfgDevinBaseUrl").value = c.devinBaseUrl || "";
  $("cfgBaseUrl").value = c.baseUrl || "https://api.openai.com/v1";
  $("cfgApiKey").value = c.apiKey || "";
  $("cfgModel").value = c.model && c.model !== "devin" ? c.model : "gpt-4o-mini";
  settingsEl.showModal();
}
$("settingsForm").addEventListener("submit", (e) => {
  if (e.submitter && e.submitter.value === "save") {
    const c = loadConfig();
    const cfgModel = $("cfgModel").value.trim();
    saveConfig({
      ...c,
      devinApiKey: $("cfgDevinKey").value.trim(),
      devinBaseUrl: $("cfgDevinBaseUrl").value.trim(),
      baseUrl: $("cfgBaseUrl").value.trim(),
      apiKey: $("cfgApiKey").value.trim(),
      // The active backend is chosen in the composer's model menu; keep the
      // Devin selection intact and otherwise fall back to the OpenAI model.
      model: c.model === "devin" ? "devin" : cfgModel || c.model,
    });
    refreshModelBadge();
    showToast("Assistant settings saved");
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
$("newSessionBtn").addEventListener("click", () => {
  resetDevinSession(true);
  showToast("Started a new Devin session");
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
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "s") {
    e.preventDefault();
    saveActive();
  }
});

refreshModelBadge();
showChatHint();
syncWelcome();
