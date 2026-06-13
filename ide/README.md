# Devin IDE

A native-feeling, macOS-style code editor with a built-in AI assistant sidebar — built with **Rust + Tauri** and the **Monaco** editor (the engine behind VS Code). Open a folder, edit files across tabs, and ask the assistant (any OpenAI-compatible model) about the code you have open.

> Companion to [Devin Desktop](https://github.com/fendoushaonian/Devin-Desktop). Devin Desktop securely exposes a folder to cloud agents; Devin IDE is a local editor you use directly.

## Features

- **Three-pane layout** — file explorer, tabbed Monaco editor, AI assistant.
- **Apple-style UI** — frosted titlebar, light/dark, SVG icons, native traffic-light window controls (overlay title bar).
- **Real editing** — syntax highlighting for many languages, dirty-state tabs, `⌘S` to save.
- **AI assistant** — streaming chat that automatically includes the open file (and any selection) as context.
- **Bring your own model** — any OpenAI-compatible endpoint: OpenAI, gateways, or a local server such as Ollama (`http://localhost:11434/v1`). Keys are stored locally.

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

The Rust side performs the network call to the AI provider (avoids browser CORS) and streams tokens back to the UI over a Tauri `Channel`.

## Develop

```bash
npm install
npm run tauri dev      # native app (requires macOS for the full experience)
npm run dev            # browser preview with a mock backend
```

`npm run dev` runs the UI in a plain browser with a mock filesystem and a mock AI echo, so the interface can be previewed without building the native shell.

## Configure the assistant

Click the gear in the title bar and set:

- **Base URL** — e.g. `https://api.openai.com/v1`
- **API key** — your provider key (stored in `localStorage`, never committed)
- **Model** — e.g. `gpt-4o-mini`

## License

MIT
