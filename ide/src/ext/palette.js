// A minimal command palette (Ctrl/Cmd+Shift+P) shared by built-in IDE actions
// and extension-contributed commands.

export function createCommandPalette({ getCommands }) {
  const overlay = document.createElement("div");
  overlay.className = "palette";
  overlay.hidden = true;
  overlay.innerHTML = `
    <div class="palette__panel" role="dialog" aria-label="Command palette">
      <input class="palette__input" type="text" placeholder="Type a command…" spellcheck="false" />
      <div class="palette__list" role="listbox"></div>
    </div>`;
  document.body.appendChild(overlay);

  const input = overlay.querySelector(".palette__input");
  const list = overlay.querySelector(".palette__list");
  let commands = [];
  let filtered = [];
  let cursor = 0;

  function score(cmd, q) {
    if (!q) return 0;
    const hay = `${cmd.category || ""} ${cmd.title} ${cmd.id}`.toLowerCase();
    const idx = hay.indexOf(q);
    return idx === -1 ? -1 : idx;
  }

  function refresh() {
    const q = input.value.trim().toLowerCase();
    filtered = commands
      .map((c) => ({ c, s: score(c, q) }))
      .filter((x) => x.s !== -1)
      .sort((a, b) => a.s - b.s)
      .map((x) => x.c);
    cursor = 0;
    render();
  }

  function render() {
    list.innerHTML = "";
    if (filtered.length === 0) {
      const empty = document.createElement("div");
      empty.className = "palette__empty";
      empty.textContent = "No matching commands";
      list.appendChild(empty);
      return;
    }
    filtered.forEach((cmd, i) => {
      const row = document.createElement("div");
      row.className = "palette__item" + (i === cursor ? " is-active" : "");
      row.setAttribute("role", "option");
      const cat = cmd.category ? `<span class="palette__cat"></span>` : "";
      row.innerHTML = `${cat}<span class="palette__title"></span>`;
      if (cmd.category) row.querySelector(".palette__cat").textContent = cmd.category;
      row.querySelector(".palette__title").textContent = cmd.title;
      row.addEventListener("mousemove", () => {
        if (cursor !== i) {
          cursor = i;
          render();
        }
      });
      row.addEventListener("click", () => run(cmd));
      list.appendChild(row);
    });
    const active = list.querySelector(".is-active");
    if (active) active.scrollIntoView({ block: "nearest" });
  }

  function run(cmd) {
    close();
    try {
      cmd.run();
    } catch (err) {
      console.error("[palette] command failed:", err);
    }
  }

  function open() {
    commands = getCommands();
    input.value = "";
    overlay.hidden = false;
    refresh();
    input.focus();
  }

  function close() {
    overlay.hidden = true;
  }

  input.addEventListener("input", refresh);
  input.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      cursor = Math.min(cursor + 1, filtered.length - 1);
      render();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      cursor = Math.max(cursor - 1, 0);
      render();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filtered[cursor]) run(filtered[cursor]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  });
  overlay.addEventListener("mousedown", (e) => {
    if (e.target === overlay) close();
  });

  return { open, close };
}
