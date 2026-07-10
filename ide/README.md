# Michael IDE

A native-feeling, macOS-style code editor with a built-in AI assistant sidebar — built with **Rust + Tauri** and the **Monaco** editor (the engine behind VS Code). Open a folder, edit files across tabs, and ask a centrally managed model about the code you have open.

> Companion to [Devin Desktop](https://github.com/fendoushaonian/Devin-Desktop). Devin Desktop securely exposes a folder to cloud agents; Michael IDE is a local editor you use directly.

## Features

- **Three-pane layout** — file explorer, tabbed Monaco editor, AI assistant.
- **Apple-style UI** — frosted titlebar, light/dark, SVG icons, native traffic-light window controls (overlay title bar).
- **Real editing** — syntax highlighting for many languages, dirty-state tabs, `⌘S` to save.
- **AI assistant** — streaming chat that automatically includes the open file (and any selection) as context.
- **Managed model gateway** — sign in once, choose an enabled model, and use centrally managed provider credentials and billing.
- **Extensions** — a lightweight, sandboxed extension system. Extensions add commands, command-palette entries, and status-bar items, and (with permission) read/write the editor and workspace. See [Writing a Michael IDE extension](docs/extensions.md).

## Architecture

```
┌── frontend (Vite + Monaco) ────────────────────────────┐
│  file tree │ tabbed editor │ AI chat (streaming)        │
└───────────────────────┬────────────────────────────────┘
                        │ Tauri IPC (invoke / Channel)
┌───────────────────────▼────────────────────────────────┐
│  src-tauri (Rust)                                       │
│   files.rs : read_dir / read_text_file / write_text_file│
│   ai.rs    : ai_chat → OpenAI-compatible SSE streaming  │
└─────────────────────────────────────────────────────────┘
```

The Rust side streams requests to the Michael gateway over HTTPS and returns tokens to the UI over a Tauri `Channel`.

## Develop

```bash
npm install
npm run tauri dev      # native app (requires macOS for the full experience)
npm run dev            # browser preview with a mock backend
```

`npm run dev` runs the UI in a plain browser with a mock filesystem and a mock AI echo, so the interface can be previewed without building the native shell.

## Configure the assistant

Sign in, then use the model picker in the chat composer. The desktop app connects
to `https://code.mrday.one`; `localStorage.michael_api` remains available only as
a development override.

Authentication tokens are stored locally and sent only to the Michael gateway.

## Extensions

Open the **Extensions** panel (puzzle icon in the title bar) to install bundled
samples or install your own from a `.zip`. Extensions run in a per-extension Web
Worker sandbox and only get the capabilities their manifest declares. To build
one, see **[Writing a Michael IDE extension](docs/extensions.md)**.

## License

MIT
