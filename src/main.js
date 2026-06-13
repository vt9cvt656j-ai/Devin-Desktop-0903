// Devin Desktop control-panel logic.
//
// Runs against the Tauri backend when available, and falls back to an in-memory
// mock when opened in a plain browser (useful for design preview / dev).

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Thin backend abstraction so the UI works with or without Tauri. */
const backend = inTauri ? await tauriBackend() : mockBackend();

async function tauriBackend() {
  const { invoke } = await import("@tauri-apps/api/core");
  const { open } = await import("@tauri-apps/plugin-dialog");
  return {
    getStatus: () => invoke("get_status"),
    start: (root, allowWrite) => invoke("start_bridge", { root, allowWrite }),
    stop: () => invoke("stop_bridge"),
    pickFolder: () => open({ directory: true, multiple: false }),
  };
}

function mockBackend() {
  let running = null;
  const token = () =>
    Array.from({ length: 40 }, () =>
      "ABCDEFGHJKMNPQRSTUVWXYZ23456789abcdefghijkmnpqrstuvwxyz"[
        Math.floor(Math.random() * 54)
      ]
    ).join("");
  return {
    getStatus: async () => running ?? stopped(),
    start: async (root, allowWrite) => {
      running = {
        running: true,
        addr: "127.0.0.1:53412",
        url: "http://127.0.0.1:53412",
        token: token(),
        root,
        allow_write: allowWrite,
      };
      return running;
    },
    stop: async () => {
      running = null;
      return stopped();
    },
    pickFolder: async () => "/Users/andrew/Projects/my-app",
  };
}

const stopped = () => ({
  running: false,
  addr: null,
  url: null,
  token: null,
  root: null,
  allow_write: false,
});

// ---- element refs ----
const el = (id) => document.getElementById(id);
const statusPill = el("statusPill");
const statusLabel = el("statusLabel");
const heroHeadline = el("heroHeadline");
const heroSub = el("heroSub");
const folderPath = el("folderPath");
const pickFolderBtn = el("pickFolderBtn");
const allowWrite = el("allowWrite");
const toggleBtn = el("toggleBtn");
const toggleLabel = el("toggleLabel");
const connectionCard = el("connectionCard");
const urlValue = el("urlValue");
const tokenValue = el("tokenValue");
const revealToken = el("revealToken");
const toast = el("toast");

let selectedFolder = null;
let currentToken = null;
let tokenRevealed = false;

// ---- rendering ----
function render(status) {
  const running = status.running;

  statusPill.classList.toggle("is-running", running);
  statusLabel.textContent = running ? "Running" : "Stopped";

  toggleBtn.classList.toggle("btn--primary", !running);
  toggleBtn.classList.toggle("btn--danger", running);
  toggleLabel.textContent = running ? "Stop bridge" : "Start bridge";
  toggleBtn.disabled = !running && !selectedFolder;

  pickFolderBtn.disabled = running;
  allowWrite.disabled = running;

  if (running) {
    selectedFolder = status.root;
    heroHeadline.textContent = "Bridge is live";
    heroSub.textContent =
      "Devin can reach the folder below once you point a tunnel at the local URL.";
  } else {
    heroHeadline.textContent = "Share a folder with Devin";
    heroSub.textContent =
      "Run a secure, token-protected bridge on your Mac so Devin can read and write files inside a single folder you choose.";
  }

  if (selectedFolder) {
    folderPath.textContent = selectedFolder;
    folderPath.title = selectedFolder;
    folderPath.classList.remove("is-empty");
  } else {
    folderPath.textContent = "Choose a folder to share…";
    folderPath.classList.add("is-empty");
  }

  connectionCard.hidden = !running;
  if (running) {
    urlValue.textContent = status.url;
    currentToken = status.token;
    renderToken();
  }
}

function renderToken() {
  if (!currentToken) return;
  tokenValue.textContent = tokenRevealed
    ? currentToken
    : "•".repeat(Math.min(currentToken.length, 24));
  tokenValue.classList.toggle("masked", !tokenRevealed);
}

let toastTimer;
function showToast(msg) {
  toast.textContent = msg;
  toast.classList.add("is-visible");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.remove("is-visible"), 1800);
}

// ---- actions ----
pickFolderBtn.addEventListener("click", async () => {
  try {
    const picked = await backend.pickFolder();
    if (picked) {
      selectedFolder = picked;
      render(stopped());
    }
  } catch (e) {
    showToast(`Could not open folder picker: ${e}`);
  }
});

toggleBtn.addEventListener("click", async () => {
  toggleBtn.disabled = true;
  try {
    const status = await backend.getStatus();
    if (status.running) {
      render(await backend.stop());
      showToast("Bridge stopped");
    } else {
      if (!selectedFolder) return;
      render(await backend.start(selectedFolder, allowWrite.checked));
      showToast("Bridge started");
    }
  } catch (e) {
    showToast(`${e}`);
    render(await backend.getStatus());
  }
});

revealToken.addEventListener("click", () => {
  tokenRevealed = !tokenRevealed;
  renderToken();
});

document.querySelectorAll("[data-copy]").forEach((btn) => {
  btn.addEventListener("click", async () => {
    const target = btn.getAttribute("data-copy");
    const text = target === "tokenValue" ? currentToken : el(target).textContent;
    try {
      await navigator.clipboard.writeText(text);
      showToast("Copied to clipboard");
    } catch {
      showToast("Copy failed");
    }
  });
});

// ---- boot ----
render(await backend.getStatus());
