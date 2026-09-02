# File-type icons

These SVG icons are from the **Material Icon Theme** for VS Code, vendored here so
the file explorer works offline in both the browser and the native Tauri app.

- Project: https://github.com/material-extensions/vscode-material-icon-theme
- License: MIT
- Copyright (c) Material Extensions and contributors

Only a curated subset of the icon set is included (common languages, config files,
and folder icons). To add more, copy the corresponding `<name>.svg` from the
upstream `icons/` directory and map it in `src/main.js`
(`EXT_ICON` / `NAME_ICON` / `FOLDER_ICON`).
