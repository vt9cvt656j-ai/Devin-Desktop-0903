# Devin Desktop

A small macOS app that runs a **secure, token-protected bridge** on your machine
so Devin (and other AI agents) can **read and write files inside a single folder
you choose** — nothing more.

It is built with **Rust** (the bridge + app core) and a **Tauri** shell with an
Apple-style control panel. All SVG artwork is vector and theme-aware (light/dark).

| Stopped | Running |
| --- | --- |
| Pick a folder, choose read-only or read/write, hit **Start**. | A loopback URL + access token appear, ready to connect. |

## Why

Devin runs in the cloud and cannot reach your laptop's `localhost` directly.
Devin Desktop closes that gap **safely**:

- You explicitly pick **one** folder. The bridge can never touch anything outside it.
- Every request must carry a **bearer token** that you can rotate by restarting.
- The server binds to **loopback** only. You decide when/whether to expose it to
  the cloud via a tunnel (e.g. Cloudflare Tunnel or ngrok).
- A **read-only** switch disables all write/delete endpoints.

## Architecture

```
┌─────────────────────────── your Mac ───────────────────────────┐
│                                                                 │
│   Tauri shell (src-tauri)          bridge-core (Rust lib)       │
│   ┌──────────────────┐  start/stop ┌───────────────────────┐   │
│   │  Apple-style UI   │───────────▶│  axum HTTP server      │   │
│   │  (src/, dist/)    │            │  + bearer-token auth   │   │
│   └──────────────────┘            │  + ScopedFs (1 folder) │   │
│                                    └───────────┬───────────┘   │
│                                       127.0.0.1:<port>         │
└────────────────────────────────────────────────┼──────────────┘
                                                   │  (your tunnel)
                                                   ▼
                                          Devin / MCP client
```

The security-sensitive logic lives in the platform-independent
[`crates/bridge-core`](crates/bridge-core) crate so it can be unit-tested in
isolation (path-traversal protection, auth, file ops). The Tauri app embeds it
and manages the server lifecycle.

## HTTP API

All endpoints require `Authorization: Bearer <token>`. Paths are **relative to
the shared folder**; absolute paths and `..` traversal are rejected.

| Method | Path | Body / Query | Purpose |
| --- | --- | --- | --- |
| `GET`  | `/api/health` | — | Liveness + root + mode |
| `GET`  | `/api/list`   | `?path=` | List a directory |
| `GET`  | `/api/read`   | `?path=` | Read a file (base64) |
| `GET`  | `/api/search` | `?q=&path=&content=&limit=` | Search names / contents |
| `POST` | `/api/write`  | `{ path, content_base64 }` | Write a file |
| `POST` | `/api/mkdir`  | `{ path }` | Create a directory |
| `POST` | `/api/delete` | `{ path }` | Delete a file/directory |

Write/mkdir/delete return `400` when the bridge is in read-only mode.

Example:

```bash
TOKEN=...        # shown in the app
BASE=http://127.0.0.1:53412
curl -H "Authorization: Bearer $TOKEN" "$BASE/api/list?path="
```

## Development

Prerequisites: **Rust** (stable, ≥ 1.85 for the 2024 edition deps), **Node 18+**,
and the [Tauri system dependencies](https://tauri.app/start/prerequisites/) for
your OS.

```bash
# install frontend deps
npm install

# run the core test-suite (no GUI needed)
cargo test -p bridge-core

# run the app in dev mode (Rust + Vite)
npm run tauri dev      # or: cargo tauri dev

# produce a release bundle (.app / .dmg on macOS)
npm run tauri build
```

App icons are generated from `src-tauri/icons/source-icon.png`:

```bash
npx tauri icon src-tauri/icons/source-icon.png
```

## Connecting Devin to the bridge

1. Start the bridge in the app and copy the **Local URL** and **token**.
2. Expose loopback to the internet with a tunnel, e.g.:
   ```bash
   cloudflared tunnel --url http://127.0.0.1:53412
   ```
3. Give Devin the **public tunnel URL** and the **token**. Devin talks to the
   HTTP API above to read/write inside your shared folder.

## Roadmap

- Native **MCP server** adapter (stdio + HTTP) so MCP-aware clients get file
  tools without bespoke HTTP glue.
- Built-in tunnel management (start/stop a Cloudflare Tunnel from the UI).
- Menu-bar / tray mode and multiple named shares.

## License

MIT © 2026 weiligon
