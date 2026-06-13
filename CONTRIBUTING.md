# Contributing

Thanks for your interest in improving Devin Desktop! This repository contains
**two apps** that share one toolchain:

| Path | App | What it is |
| --- | --- | --- |
| repo root (`crates/`, `src-tauri/`, `src/`) | **Devin Desktop** | A token-protected local bridge that exposes one folder to cloud agents. |
| [`ide/`](ide) | **Devin IDE** | A macOS-style Monaco editor with an AI assistant sidebar. |

The two are **separate Cargo workspaces**, so Rust commands run from the relevant
directory (repo root for Devin Desktop, `ide/` for Devin IDE).

## Prerequisites

- **Rust** stable, **≥ 1.85**. The workspace crates are edition 2021, but a
  transitive dependency ships as edition 2024, which Cargo only supports on
  1.85+ (older toolchains fail to build the dependency tree).
- **Node** 18+ (CI pins Node 20 — see [`.nvmrc`](.nvmrc)).
- The [Tauri system dependencies](https://tauri.app/start/prerequisites/) for your OS.

If you use VS Code, install the recommended extensions (you'll be prompted from
[`.vscode/extensions.json`](.vscode/extensions.json)); ready-made debug
configurations live in [`.vscode/launch.json`](.vscode/launch.json).

## Common commands

Devin Desktop (repo root):

```bash
npm install                  # frontend deps
cargo test -p bridge-core    # core test-suite (no GUI needed)
npm run tauri dev            # run the app (Rust + Vite)
npm run build                # build the frontend only
```

Devin IDE (`ide/`):

```bash
cd ide
npm install
npm run dev                  # browser preview with a mock backend
npm run tauri dev            # native app
```

## Before you open a PR

Match what CI enforces (`.github/workflows/ci.yml` and `ide.yml`):

```bash
# Devin Desktop (repo root)
cargo fmt --all -- --check
cargo clippy -p bridge-core --all-targets -- -D warnings
cargo test -p bridge-core
npm run build

# Devin IDE
cd ide
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
npm run build
```

### Pre-commit hooks

This repo ships a [`.pre-commit-config.yaml`](.pre-commit-config.yaml) that runs
`rustfmt` and basic hygiene checks on every commit. Enable it once:

```bash
pip install pre-commit   # or: pipx install pre-commit
pre-commit install
```

Run against everything at any time with `pre-commit run --all-files`.

## Branches & commits

- Branch off the default branch; use a short, descriptive name.
- Keep commits focused; write imperative subject lines (e.g. `ide: add global search`).
- PRs should pass CI and keep `rustfmt`/`clippy` clean.

## Reporting security issues

Please do **not** open public issues for vulnerabilities — see [`SECURITY.md`](SECURITY.md).
