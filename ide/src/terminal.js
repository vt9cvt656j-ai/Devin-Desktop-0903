import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";

let termPanel, termBody, editorwrapEl;
let backend, monacoEditor, getRootPath, t;

let termTabs = [];
let activeTermTab = -1;
let termSeq = 0;

function termTheme() {
  const isDark =
    document.documentElement.dataset.theme === "dark" ||
    (document.documentElement.dataset.theme !== "light" &&
      window.matchMedia("(prefers-color-scheme: dark)").matches);

  if (isDark) {
    return {
      background: "#0D1117", foreground: "#D6DEEB", cursor: "#7C5CFF",
      cursorAccent: "#0D1117", selectionBackground: "rgba(124, 92, 255, 0.28)",
      selectionForeground: "#FFFFFF",
      black: "#0B1020", red: "#FF6B7A", green: "#3DDC97", yellow: "#FFD166",
      blue: "#5B8DEF", magenta: "#C792EA", cyan: "#5DE4C7", white: "#D6DEEB",
      brightBlack: "#637083", brightRed: "#FF8FA3", brightGreen: "#63E6BE",
      brightYellow: "#FFE08A", brightBlue: "#7AA2FF", brightMagenta: "#D9A8FF",
      brightCyan: "#84F0D8", brightWhite: "#FFFFFF",
    };
  }

  return {
    background: "#0B1020", foreground: "#D6DEEB", cursor: "#0A84FF",
    cursorAccent: "#0B1020", selectionBackground: "rgba(10, 132, 255, 0.24)",
    selectionForeground: "#FFFFFF",
    black: "#0B1020", red: "#FF5F7A", green: "#35D49B", yellow: "#F7C948",
    blue: "#5B8DEF", magenta: "#B77CFF", cyan: "#4DDDD0", white: "#D6DEEB",
    brightBlack: "#6B7280", brightRed: "#FF87A0", brightGreen: "#6EE7B7",
    brightYellow: "#FFE082", brightBlue: "#8AB4FF", brightMagenta: "#D8B4FE",
    brightCyan: "#99F6E4", brightWhite: "#FFFFFF",
  };
}

const termIsOpen = () => termPanel && !termPanel.hidden;

const termResizeObserver = new ResizeObserver(() => {
  if (termIsOpen() && activeTermTab >= 0 && termTabs[activeTermTab]?.fit) {
    try { termTabs[activeTermTab].fit.fit(); } catch {}
  }
});

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
    btn.innerHTML = `<span class="term-tab__status"></span><span class="term-tab__label"></span><span class="term-tab__x">&times;</span>`;
    btn.querySelector(".term-tab__label").textContent = tab.label;
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

  let initDone = false;
  let initBuffer = "";
  const entry = { term, fit, container, label, backendId: null, opening: false };
  termTabs.push(entry);

  term.onData((d) => { if (entry.backendId != null) backend.termWrite(entry.backendId, d); });
  term.onResize(({ cols, rows }) => { if (entry.backendId != null) backend.termResize(entry.backendId, cols, rows); });

  switchTermTab(idx);

  entry.opening = true;
  try {
    entry.backendId = await backend.termOpen(
      { cwd: getRootPath() || undefined, cols: term.cols, rows: term.rows },
      (ev) => {
        if (ev.kind === "data") {
          if (!initDone) { initBuffer += ev.data; return; }
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

function cleanupAllTerminals() {
  for (const tab of termTabs) {
    if (tab.backendId != null) backend.termClose(tab.backendId);
  }
}

/**
 * Initialise the terminal module with required dependencies.
 *
 * @param {object} deps
 * @param {object} deps.backend     - Backend abstraction (termOpen/Write/Resize/Close)
 * @param {object} deps.editor      - Monaco editor instance (for layout calls)
 * @param {Function} deps.getRootPath - Returns the current workspace root path
 * @param {Function} deps.t          - i18n translation function
 */
export function initTerminal(deps) {
  backend = deps.backend;
  monacoEditor = deps.editor;
  getRootPath = deps.getRootPath;
  t = deps.t;

  const $ = (id) => document.getElementById(id);
  termPanel = $("terminalPanel");
  termBody = $("terminalBody");
  editorwrapEl = document.querySelector(".editorwrap");

  $("terminalClose")?.addEventListener("click", closeTerminal);
  $("termTrafficClose")?.addEventListener("click", closeTerminal);
  $("terminalBtn")?.addEventListener("click", toggleTerminal);
  window.addEventListener("beforeunload", cleanupAllTerminals);
}

export { openTerminal, closeTerminal, toggleTerminal, createTermTab };
