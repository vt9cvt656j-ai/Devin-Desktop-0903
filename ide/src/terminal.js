import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";

let termPanel, termBody, editorwrapEl, termTabBar;
let backend, monacoEditor, getRootPath, t;

let termTabs = [];
let activeTermTab = -1;
let termSeq = 0;

function termTheme() {
  return {
    background: "#0e1116",
    foreground: "#e6edf3",
    cursor: "#58a6ff",
    cursorAccent: "#0e1116",
    selectionBackground: "rgba(56, 139, 253, 0.25)",
    selectionForeground: "#ffffff",
    black: "#0d1117",
    red: "#ff7b72",
    green: "#3fb950",
    yellow: "#d29922",
    blue: "#58a6ff",
    magenta: "#bc8cff",
    cyan: "#39d353",
    white: "#e6edf3",
    brightBlack: "#484f58",
    brightRed: "#ffa198",
    brightGreen: "#56d364",
    brightYellow: "#e3b341",
    brightBlue: "#79c0ff",
    brightMagenta: "#d2a8ff",
    brightCyan: "#56d364",
    brightWhite: "#ffffff",
  };
}

const termIsOpen = () => termPanel && !termPanel.hidden;

const termResizeObserver = new ResizeObserver(() => {
  if (termIsOpen() && activeTermTab >= 0 && termTabs[activeTermTab]?.fit) {
    try { termTabs[activeTermTab].fit.fit(); } catch {}
  }
});

function renderTermTabs() {
  if (!termTabBar) return;
  termTabBar.innerHTML = "";
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
    termTabBar.appendChild(btn);
  });
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

function initResize(resizeHandle) {
  if (!resizeHandle) return;
  let startY = 0;
  let startH = 0;
  let dragging = false;

  const onMouseMove = (e) => {
    if (!dragging) return;
    const delta = startY - e.clientY;
    const newH = Math.max(140, Math.min(window.innerHeight * 0.7, startH + delta));
    termPanel.style.flex = `0 0 ${newH}px`;
    requestAnimationFrame(() => {
      monacoEditor.layout();
      if (activeTermTab >= 0 && termTabs[activeTermTab]?.fit) {
        try { termTabs[activeTermTab].fit.fit(); } catch {}
      }
    });
  };

  const onMouseUp = () => {
    dragging = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    document.removeEventListener("mousemove", onMouseMove);
    document.removeEventListener("mouseup", onMouseUp);
  };

  resizeHandle.addEventListener("mousedown", (e) => {
    e.preventDefault();
    dragging = true;
    startY = e.clientY;
    startH = termPanel.getBoundingClientRect().height;
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  });
}

function initMaximize(maxBtn) {
  if (!maxBtn) return;
  let maximized = false;
  let savedFlex = "";

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
      if (activeTermTab >= 0 && termTabs[activeTermTab]?.fit) {
        try { termTabs[activeTermTab].fit.fit(); } catch {}
      }
    });
  });
}

/**
 * Initialise the terminal module with required dependencies.
 */
export function initTerminal(deps) {
  backend = deps.backend;
  monacoEditor = deps.editor;
  getRootPath = deps.getRootPath;
  t = deps.t;

  const $ = (id) => document.getElementById(id);
  termPanel = $("terminalPanel");
  termBody = $("terminalBody");
  termTabBar = $("termTabBar");
  editorwrapEl = document.querySelector(".editorwrap");

  $("terminalClose")?.addEventListener("click", closeTerminal);
  $("terminalBtn")?.addEventListener("click", toggleTerminal);
  $("termNewBtn")?.addEventListener("click", () => createTermTab());

  initResize($("terminalResize"));
  initMaximize($("termMaxBtn"));

  window.addEventListener("beforeunload", cleanupAllTerminals);
}

export { openTerminal, closeTerminal, toggleTerminal, createTermTab };
