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
const modelBadge = $("modelBadge");
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
    tab.innerHTML = `<span class="dot"></span><span class="label"></span><span class="x">×</span>`;
    tab.querySelector(".label").textContent = f.name;
    tab.querySelector(".label").addEventListener("click", () => activate(path));
    tab.querySelector(".dot").addEventListener("click", () => activate(path));
    tab.querySelector(".x").addEventListener("click", (e) => {
      e.stopPropagation();
      closeFile(path);
    });
    tabsEl.appendChild(tab);
  }
}

// ---- file tree ----
let rootPath = null;

function iconSvg(id) {
  return `<svg class="ic"><use href="#${id}" /></svg>`;
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
      row.innerHTML = `<svg class="chev"><use href="#i-chevron" /></svg>${iconSvg("i-folder")}<span class="name"></span>`;
    } else {
      row.innerHTML = `<span style="width:14px;flex:none"></span>${iconSvg("i-file")}<span class="name"></span>`;
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
  const c = loadConfig();
  modelBadge.textContent = c.model && c.apiKey ? c.model : "not configured";
}

const history = [];
let streaming = false;

function addMessage(role, text) {
  const wrap = document.createElement("div");
  wrap.className = "msg " + role;
  wrap.innerHTML = `<span class="msg__who"></span><div class="msg__body"></div>`;
  wrap.querySelector(".msg__who").textContent = role === "user" ? "You" : "Assistant";
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
