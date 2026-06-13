---
name: testing-ide
description: How to run and manually test the ide/ sub-app (Michael IDE) — the Vite + Monaco + vanilla-JS AI-assistant editor. Use when verifying AI chat rendering, the titlebar/menus, branding, or theme parity.
---

# Testing the `ide/` sub-app (Michael IDE)

The `ide/` directory is a vanilla-JS + Vite + `monaco-editor` app (no framework, no markdown library — the markdown renderer in `src/markdown.js` is hand-rolled and DOM-safe). The right-hand AI assistant renders rich, card-based replies.

## Run it

```bash
cd ide
npm install        # first time
npm run dev        # Vite dev server on http://localhost:5173 (strictPort)
npm run build      # CI-critical production build (must exit 0)
npm run tauri dev  # native desktop (Tauri) shell
```

## Backend selection

`main.js` picks a backend at startup: native **Tauri** backend when `__TAURI_INTERNALS__` exists on `window`, otherwise a **mock** backend (this is what runs in a plain browser at `localhost:5173`). The mock streams a fixed, rich markdown sample after a ~750ms delay, so the "thinking" card is visible before the reply renders. This is ideal for testing rendering without any real API key.

## Sending a message (manual test)

- The send keybinding is **Ctrl+Enter** (⌘↩ on macOS). Plain Enter inserts a newline — it does NOT send.
- A send only fires if an AI config exists; otherwise the settings dialog opens. For browser testing, a dummy key (e.g. `sk-test`) saved in `localStorage` (key `devin-ide.ai-config`) is enough for the mock backend to respond.
- Quick path: click a starter chip (e.g. "Find potential bugs"), then click the send (↑) button or press Ctrl+Enter. The mock reply exercises headings, an ordered list with inline code, a code card (filename + Copy + Monaco syntax highlighting), a blockquote, a 3-column table, a task list, and a link.

## Model identity

The selected model (model picker in the composer) drives the assistant header, per-message avatar, and thinking orb, using real provider glyphs (`#i-brand-openai|anthropic|meta|qwen`) defined in `index.html`. `brandOf(id)` maps a model id to a brand.

## Dark-mode parity

There is no in-app theme toggle; dark mode follows `prefers-color-scheme`. To force dark in a headless/automation browser, emulate the media feature over CDP (`Emulation.setEmulatedMedia` with `prefers-color-scheme: dark`) and reload. Note the emulation reverts when the CDP client disconnects, so hold the connection open while viewing, then reload the page.

## What to verify

- Build: `npm run build` exits 0 (Rust under `src-tauri/` is untouched by frontend work).
- Browser console shows only Vite HMR lines — no errors.
- Rendering, titlebar menus (File/Edit/View/Help), logo placement, and branding look correct in BOTH light and dark.
