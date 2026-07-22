// The "Extensions" sheet: install bundled extensions, install from a file,
// enable/disable, and uninstall. Mirrors the look of the settings dialog.

import { t } from "../i18n.js";

export function createExtensionsPanel({ manager, host, showToast }) {
  const dialog = document.createElement("dialog");
  dialog.className = "sheet sheet--ext";
  dialog.innerHTML = `
    <div class="sheet__body">
      <div class="sheet__icon"><svg viewBox="0 0 24 24"><use href="#i-ext" /></svg></div>
      <h2 data-i18n="ext.title"></h2>
      <p class="sheet__sub" data-i18n="ext.sub"></p>
      <div class="ext-actions">
        <button class="btn" id="extInstallFile" type="button" data-i18n="ext.installFile"></button>
        <button class="btn" id="extClose" type="button" value="cancel" data-i18n="ext.done"></button>
      </div>
      <div class="ext-section">
        <div class="ext-section__title" data-i18n="ext.installed"></div>
        <div class="ext-list" id="extInstalled"></div>
      </div>
      <div class="ext-section">
        <div class="ext-section__title" data-i18n="ext.available"></div>
        <div class="ext-list" id="extAvailable"></div>
      </div>
    </div>`;
  document.body.appendChild(dialog);

  const installedEl = dialog.querySelector("#extInstalled");
  const availableEl = dialog.querySelector("#extAvailable");

  function permBadges(perms) {
    if (!perms || perms.length === 0) return "";
    return `<span class="ext-perms">${perms
      .map(() => `<span class="ext-perm"></span>`)
      .join("")}</span>`;
  }

  function fillPerms(node, perms) {
    const spans = node.querySelectorAll(".ext-perm");
    perms.forEach((p, i) => {
      if (spans[i]) spans[i].textContent = p;
    });
  }

  const EXT_ICONS = {
    theme: `<circle cx="13.5" cy="6.5" r="1.4" fill="currentColor"/><circle cx="17.3" cy="10.5" r="1.4" fill="currentColor"/><circle cx="8.4" cy="7.4" r="1.4" fill="currentColor"/><circle cx="6.6" cy="12.4" r="1.4" fill="currentColor"/><path d="M12 2.5C6.75 2.5 2.5 6.75 2.5 12S6.75 21.5 12 21.5c1.1 0 2-.9 2-2 0-.5-.2-.96-.5-1.3-.3-.34-.5-.8-.5-1.3 0-1.1.9-2 2-2h2.3c1.98 0 3.7-1.6 3.7-3.6 0-4.86-4.07-8.8-9-8.8z" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
    git: `<circle cx="7" cy="7" r="2.1" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="7" cy="17" r="2.1" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="17" cy="9" r="2.1" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7 9.1v5.8M17 11.1c0 3-3 3.7-6.4 3.7" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
    language: `<path d="M3.5 5.5h8M7.5 4v1.5c0 3.6-2 6.7-5 8.4M5.5 8.2c0 2 1.9 3.9 4.7 4.9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><path d="M13 20.5l4-9 4 9M14.6 17.2h4.8" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>`,
    ai: `<circle cx="12" cy="12" r="3.1" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M12 2.6v3M12 18.4v3M2.6 12h3M18.4 12h3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>`,
    format: `<rect x="3.5" y="3.5" width="17" height="17" rx="3.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7.5 8.5h9M7.5 12h6M7.5 15.5h4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>`,
    web: `<circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M3 12h18M12 3a14 14 0 010 18M12 3a14 14 0 000 18" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
    default: `<path d="M20 16.5v-9a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 002 7.5v9a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0020 16.5z" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M3.3 7.5L12 12.5l8.7-5M12 22V12.5" fill="none" stroke="currentColor" stroke-width="1.5"/>`,
  };
  function extHue(id) {
    let h = 0;
    for (const c of id || "x") h = ((h << 5) - h + c.charCodeAt(0)) | 0;
    return Math.abs(h) % 360;
  }
  function extIcon(manifest) {
    const id = (manifest.id || "").toLowerCase();
    const perms = (manifest.permissions || []).map((p) => String(p).toLowerCase());
    if (/theme|icon|color/.test(id)) return EXT_ICONS.theme;
    if (/git|scm/.test(id)) return EXT_ICONS.git;
    if (/lang|locale|i18n/.test(id)) return EXT_ICONS.language;
    if (/ai|assistant|copilot/.test(id) || perms.includes("network")) return EXT_ICONS.ai;
    if (/format|count|date|lint|prettier/.test(id)) return EXT_ICONS.format;
    if (/web|html|css|server|http/.test(id)) return EXT_ICONS.web;
    return EXT_ICONS.default;
  }

  function card(manifest) {
    const el = document.createElement("div");
    el.className = "ext-card";
    el.style.setProperty("--h", extHue(manifest.id));
    el.innerHTML = `
      <div class="ext-card__icon"><svg viewBox="0 0 24 24" width="22" height="22">${extIcon(manifest)}</svg></div>
      <div class="ext-card__main">
        <div class="ext-card__name"></div>
        <div class="ext-card__desc"></div>
        <div class="ext-card__meta"><span class="ext-card__ver"></span>${permBadges(manifest.permissions)}</div>
      </div>
      <div class="ext-card__actions"></div>`;
    el.querySelector(".ext-card__name").textContent = manifest.name;
    el.querySelector(".ext-card__desc").textContent = manifest.description || "";
    el.querySelector(".ext-card__ver").textContent =
      `${manifest.id}${manifest.version ? " · v" + manifest.version : ""}`;
    fillPerms(el, manifest.permissions || []);
    return el;
  }

  async function render() {
    let installed = [];
    let available = [];
    try {
      [installed, available] = await Promise.all([
        manager.listInstalled(),
        manager.availableBuiltin(),
      ]);
    } catch (err) {
      showToast(String(err));
      return;
    }
    const installedIds = new Set(installed.map((x) => x.manifest.id));

    installedEl.innerHTML = "";
    if (installed.length === 0) {
      const emptyEl = document.createElement("div");
      emptyEl.className = "ext-empty";
      emptyEl.textContent = t("ext.noInstalled");
      installedEl.appendChild(emptyEl);
    }
    for (const item of installed) {
      const el = card(item.manifest);
      const actions = el.querySelector(".ext-card__actions");

      const toggle = document.createElement("button");
      toggle.className = "btn ext-btn";
      toggle.textContent = item.enabled ? t("ext.disable") : t("ext.enable");
      toggle.addEventListener("click", async () => {
        try {
          await manager.setEnabled(item.manifest.id, !item.enabled);
          if (item.enabled) host.deactivate(item.manifest.id);
          else await host.activate(item, manager);
          await render();
        } catch (err) {
          showToast(String(err));
        }
      });

      const remove = document.createElement("button");
      remove.className = "btn ext-btn ext-btn--danger";
      remove.textContent = t("ext.uninstall");
      remove.addEventListener("click", async () => {
        try {
          host.deactivate(item.manifest.id);
          await manager.uninstall(item.manifest.id);
          await render();
        } catch (err) {
          showToast(String(err));
        }
      });

      if (!item.enabled) el.classList.add("is-disabled");
      actions.append(toggle, remove);
      installedEl.appendChild(el);
    }

    availableEl.innerHTML = "";
    const notInstalled = available.filter((m) => !installedIds.has(m.id));
    if (notInstalled.length === 0) {
      const allEl = document.createElement("div");
      allEl.className = "ext-empty";
      allEl.textContent = t("ext.allInstalled");
      availableEl.appendChild(allEl);
    }
    for (const manifest of notInstalled) {
      const el = card(manifest);
      const actions = el.querySelector(".ext-card__actions");
      const install = document.createElement("button");
      install.className = "btn btn--primary ext-btn";
      install.textContent = t("ext.install");
      install.addEventListener("click", async () => {
        try {
          const item = await manager.installBuiltin(manifest.id);
          await host.activate(item, manager);
          showToast(t("ext.installedMsg", { name: manifest.name }));
          await render();
        } catch (err) {
          showToast(String(err));
        }
      });
      actions.appendChild(install);
      availableEl.appendChild(el);
    }
  }

  dialog.querySelector("#extInstallFile").addEventListener("click", async () => {
    try {
      const item = await manager.installFromFile();
      if (!item) return;
      await host.activate(item, manager);
      showToast(t("ext.installedMsg", { name: item.manifest.name }));
      await render();
    } catch (err) {
      showToast(String(err));
    }
  });
  dialog.querySelector("#extClose").addEventListener("click", () => dialog.close());

  function applyI18n() {
    for (const el of dialog.querySelectorAll("[data-i18n]")) {
      const key = el.getAttribute("data-i18n");
      if (key) el.textContent = t(key);
    }
  }

  async function open() {
    applyI18n();
    await render();
    dialog.showModal();
  }

  return { open, render };
}
