# Security Policy

Devin Desktop runs a local HTTP bridge that can read and write files on your
machine, so we take security seriously.

## Reporting a vulnerability

Please report security issues **privately** rather than opening a public issue.

- Preferred: use GitHub's private vulnerability reporting on this repository
  (the **Security** tab → **Report a vulnerability**).
- We aim to acknowledge reports within a few days and will coordinate a fix and
  disclosure timeline with you.

When reporting, include the affected app (Devin Desktop bridge or Devin IDE), a
description of the issue, and steps to reproduce if possible.

## Supported versions

This project is pre-1.0 and under active development. Security fixes target the
latest commit on the default branch; there is no long-term support for older
tags yet.

## Security model (Devin Desktop bridge)

The bridge is designed so that exposing it to a cloud agent is safe **by
default**:

- **Single shared folder.** All file operations are confined to one folder you
  pick. Absolute paths and `..` traversal are rejected, and symlinks that escape
  the folder are blocked.
- **Bearer-token auth.** Every request must carry an `Authorization: Bearer
  <token>` header. Tokens are generated with a CSPRNG and compared in
  constant time; restarting the bridge rotates the token.
- **Loopback only.** The server binds to `127.0.0.1`. Exposing it to the
  internet (e.g. via a tunnel) is an explicit, user-initiated step.
- **Read-only switch.** When read-only mode is on, all write/mkdir/delete
  endpoints return `400`.

If you find a way to bypass any of these guarantees, that is a security issue —
please report it as described above.

## Notes for the Devin IDE

The IDE stores the AI provider **API key in `localStorage`** and sends chat
requests from the Rust backend to the configured OpenAI-compatible endpoint.
Keys are never committed to the repo. Treat your configured endpoint and key as
sensitive.
