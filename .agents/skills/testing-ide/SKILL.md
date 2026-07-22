---
name: testing-ide
description: How to run and manually test the ide/ sub-app (Michael IDE), including chat rendering, agent verification gates, and rendered UI checks.
---

# Testing the `ide/` sub-app

## Baseline

```bash
cd ide
npm install                 # first run only
npm test                    # frontend logic tests
npm run build               # required production build
npm run dev                 # browser preview: http://localhost:5174
npm run tauri dev           # native desktop app
```

`vite.config.js` uses port `5174` with `strictPort`; stop the existing process if that port is occupied.

## Browser preview versus desktop

- The browser preview uses an in-memory mock filesystem, Git state, task runner, terminal, LSP, and debugger. It is suitable for checking layout, Monaco/chat rendering, menus, model branding, responsive behavior, and light/dark themes. Mock task or terminal output is not evidence that a real project built or passed tests.
- Browser AI is not a fixed canned reply: `aiChat` and `aiChatWithTools` stream from the configured gateway over HTTP/SSE. Sending requires a genuine Michael login token plus an active plan or credits and a reachable API (`VITE_API_TARGET` for the Vite `/api` proxy, or the `michael_api` development override). The preview login backend is mocked, so use an already valid session/token for AI tests; a dummy key such as `sk-test` does not bypass the access gate.
- The Tauri app uses the real local filesystem, Git, shell/task runner, MCP backend, and browser/screenshot commands. Use `npm run tauri dev` for any claim about agent edits, commands, builds, tests, MCP, or rendered UI validation. Browser automation requires an installed Chrome, Chromium, or Edge.

## Verify the agent engineering gate

Use a disposable real project with deterministic, non-watch `typecheck`, `test`, `lint`, and/or `build` scripts, then open that folder in the Tauri app and use Agent mode.

1. Run the project's commands once outside the agent to establish the expected exit codes.
2. Ask the agent for a small source change and a focused test. Confirm the file tool actually succeeded and the command cards contain real output from that project directory.
3. Temporarily make one verification script exit non-zero, then request another edit. The failure must remain visible and the final reply must not claim the project passed.
4. Restore the script, make a further edit, and confirm a successful verification runs after the latest mutation. Treat only exit code `0` from the real command as passing; prose, diagnostics alone, or a green-looking card is insufficient.

Also run `npm test` and `npm run build` in `ide/` after changes to the orchestration itself.

## Verify a UI task

In the Tauri app, have the agent start the target project's real dev server and require this sequence:

```text
Use browser navigate with fresh=true on the actual local URL; run browser check;
use nodes plus click/type and assert to verify the primary interaction; then use
screenshot at 1440x900 and 390x844 (mobile=true) to inspect desktop and mobile rendering.
Report the URL, assertions, console/network failures, and anything not verified.
```

`browser check` should report no unexplained console or network failures. Use `nodes`/`assert` for behavior and screenshots for visual layout; a screenshot alone does not prove an interaction works. Check that text is not clipped, controls do not overlap, and the primary workflow completes at both viewport sizes.

## IDE UI checklist

- Browser console has no application errors.
- File tree, tabs, Monaco, assistant cards, titlebar menus, logo, and provider glyphs render correctly.
- Ctrl+Enter sends on Windows/Linux and Command+Enter sends on macOS; plain Enter inserts a newline.
- Light and dark modes both work. Dark mode follows `prefers-color-scheme`; keep browser media emulation active while inspecting it.
