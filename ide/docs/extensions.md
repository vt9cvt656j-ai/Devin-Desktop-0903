# Writing a Michael IDE extension

Michael IDE has its own lightweight extension format. It is **not** the VS Code
`.vsix` format — Michael IDE embeds Monaco (VS Code's editor component) but not the
VS Code Extension Host, so VS Code extensions do not run here. Instead, an
extension is a tiny package — a manifest plus one JavaScript module — that runs
in a sandboxed Web Worker and talks to the IDE through a small, permission-gated
API.

This guide shows you how to write, package, and install one.

## TL;DR

An extension is a folder (or `.zip`) containing:

```
my-extension/
├── extension.json   # manifest: id, name, permissions, contributed commands
└── index.js         # ES module that exports activate(ide)
```

```jsonc
// extension.json
{
  "id": "acme.hello",
  "name": "Hello",
  "version": "0.1.0",
  "description": "Says hello.",
  "author": "you",
  "main": "index.js",
  "permissions": ["editor"],
  "contributes": {
    "commands": [{ "id": "hello.say", "title": "Hello: Say Hello" }]
  }
}
```

```js
// index.js
export function activate(ide) {
  ide.commands.register("hello.say", async () => {
    await ide.editor.insertText("Hello from my extension!");
    ide.window.showInformationMessage("Hello ran");
  });
}
```

Zip it, open **Extensions** (the puzzle icon in the title bar) → **Install from
file…**, pick the `.zip`, and your command shows up in the command palette
(`Ctrl/Cmd+Shift+P`).

## The manifest (`extension.json`)

| Field | Required | Default | Notes |
|---|---|---|---|
| `id` | **yes** | — | Unique id, also used as the on-disk folder name. Letters, digits, `.`, `_`, `-` only; ≤ 100 chars; may not start with `.`. Convention: `publisher.name` (e.g. `acme.hello`). |
| `name` | **yes** | — | Human-readable display name (shown in the Extensions panel and used as the command-palette category). |
| `version` | no | `""` | Semver string, e.g. `0.1.0`. |
| `description` | no | `""` | One-line description. |
| `author` | no | `""` | Author / publisher. |
| `main` | no | `index.js` | Entry module, resolved relative to the package root. |
| `permissions` | no | `[]` | Capabilities the extension is granted — see [Permissions](#permissions). |
| `contributes.commands` | no | `[]` | Array of `{ "id", "title" }`. `title` is what shows in the command palette. |

Unknown fields are ignored, so the format can grow without breaking older
extensions.

## The entry module

The entry file is a standard ES module. It must export an `activate` function and
may export a `deactivate` function:

```js
export function activate(ide) {
  // Called once when the extension is enabled. Register commands, seed the
  // status bar, etc. May be async.
}

export function deactivate() {
  // Optional. Called when the extension is disabled or uninstalled.
}
```

`activate(ide)` receives the **`ide` capability object** — the only way an
extension can interact with the editor. There are no `window`, `document`,
filesystem, or network globals available to the extension (it runs in a Worker
sandbox), so everything goes through `ide`.

### Cleanup with `subscriptions`

Anything you push onto `ide.subscriptions` is disposed when the extension
deactivates — push either a function or an object with a `dispose()` method:

```js
export function activate(ide) {
  const timer = setInterval(() => {/* ... */}, 1000);
  ide.subscriptions.push(() => clearInterval(timer));
}
```

## The `ide` API

| Method | Permission | Description |
|---|---|---|
| `ide.commands.register(id, handler)` | — | Register a command handler. `handler` may be async. To appear in the palette, also list the command under `contributes.commands`. |
| `ide.window.showInformationMessage(text)` | — | Show a transient toast notification. |
| `ide.window.setStatusBarItem(id, opts)` | — | Add/update a status-bar item. `opts = { text, tooltip?, command? }`; if `command` is set, clicking the item runs that command. |
| `ide.window.removeStatusBarItem(id)` | — | Remove a status-bar item you added. |
| `ide.editor.getText()` | `editor` | Resolve to the full text of the active editor (`""` if none). |
| `ide.editor.getSelection()` | `editor` | Resolve to the currently selected text. |
| `ide.editor.insertText(text)` | `editor` | Insert `text` at the cursor. |
| `ide.workspace.readFile(path)` | `workspace-read` | Resolve to the contents of a file in the open workspace. |
| `ide.workspace.writeFile(path, content)` | `workspace-write` | Write `content` to a file in the open workspace. |

All methods that talk to the host return Promises — `await` them. Calling a
method whose permission you did **not** declare rejects with
`permission "<perm>" not granted to <id>`.

## Permissions

Extensions are deny-by-default. List only what you need in `permissions`:

| Permission | Grants |
|---|---|
| `editor` | `ide.editor.*` (read text/selection, insert text) |
| `workspace-read` | `ide.workspace.readFile` |
| `workspace-write` | `ide.workspace.writeFile` |

`ide.commands.*` and `ide.window.*` are always available and need no permission.

The Extensions panel shows each extension's declared permissions as badges so a
user can see what an extension can do before enabling it.

## How commands reach the palette

A command shows up in the command palette (`Ctrl/Cmd+Shift+P`) when it is both:

1. **registered at runtime** via `ide.commands.register(id, handler)`, and
2. **declared** in `contributes.commands` with a `title`.

The palette lists it as `title` under the category `name` from your manifest. If
a registered command has no matching `contributes.commands` entry, it still works
but is shown by its raw id.

## Packaging

Zip the package so the manifest is findable:

```bash
cd my-extension
zip -r ../my-extension.zip extension.json index.js   # extension.json at the zip root
```

A single wrapping top-level folder is also accepted (the installer finds the
folder that contains `extension.json`), so a zip produced by
`zip -r my-extension.zip my-extension/` works too.

## Installing

- **From the bundled registry:** Extensions panel → **Available** → **Install**.
  Michael IDE ships sample extensions (Word Count, Insert Date) compiled into the
  app.
- **From a file:** Extensions panel → **Install from file…** → choose your
  `.zip`.

Installed extensions are unpacked to the app data directory, one folder per id:

| OS | Path |
|---|---|
| Linux | `~/.local/share/ai.devin.ide/extensions/<id>/` |
| macOS | `~/Library/Application Support/ai.devin.ide/extensions/<id>/` |
| Windows | `%APPDATA%\ai.devin.ide\extensions\<id>\` |

A sibling `state.json` records which extensions are enabled. Enable/disable and
uninstall from the Extensions panel; disabling tears down the extension's worker
and removes its status-bar items.

## Security model

Extensions are untrusted code, so the system confines them:

- **Worker sandbox.** Each enabled extension runs in its own Web Worker — no DOM,
  no editor, no filesystem, no ambient network. It can only send messages to the
  extension host on the main thread.
- **Permission-gated RPC.** Every privileged call (`editor.*`, `workspace.*`) is
  checked host-side against the manifest's `permissions` before it runs.
- **No `eval`.** Extension code is loaded by dynamic `import()` of a blob URL
  under a CSP of `script-src 'self' blob:` / `worker-src 'self' blob:` (no
  `unsafe-eval`).
- **Safe install.** On install, every archive entry is validated for path
  traversal and against per-file (16 MiB) and total (64 MiB) size caps *before*
  any files are written, and an existing same-id install is only replaced after
  the new archive fully validates — a malformed or malicious package can't
  escape the extension directory or destroy an already-installed extension.

## Worked example: a status-bar word counter

```jsonc
// extension.json
{
  "id": "acme.word-count",
  "name": "Word Count",
  "version": "0.1.0",
  "main": "index.js",
  "permissions": ["editor"],
  "contributes": {
    "commands": [{ "id": "wordCount.count", "title": "Word Count: Count Active File" }]
  }
}
```

```js
// index.js
export function activate(ide) {
  async function refresh() {
    const text = (await ide.editor.getText()) ?? "";
    const words = (text.match(/\S+/g) || []).length;
    ide.window.setStatusBarItem("wordCount", {
      text: `Words ${words} · Chars ${text.length}`,
      tooltip: "Word Count — click to refresh",
      command: "wordCount.count",
    });
  }

  ide.commands.register("wordCount.count", async () => {
    await refresh();
    ide.window.showInformationMessage("Word Count refreshed");
  });

  refresh(); // seed on activation
}
```

The two bundled samples are the canonical reference — see
[`src-tauri/extensions/word-count`](../src-tauri/extensions/word-count) and
[`src-tauri/extensions/insert-date`](../src-tauri/extensions/insert-date).

## Current limitations (MVP)

- Capabilities are limited to **commands, the command palette, the status bar,
  toasts, editor text/selection, and workspace file read/write**. There are no
  custom UI panels/webviews, settings contributions, or editor decorations yet.
- Command handlers are currently invoked with no arguments.
- Distribution is via the **bundled registry + install-from-file**; there is no
  remote marketplace.
