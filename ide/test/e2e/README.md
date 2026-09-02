# E2E harness — Phase 1

Drives the **real** `_executeToolStepInner` from `src/main.js` against a **real** temp directory
and asserts on **files on disk**. See `../E2E-HARNESS-SPEC.md` for why this exists.

    node test/e2e/run2.mjs

`src/main.js` is **unmodified**. The test hook is appended by a Node module-customization loader
(`hooks.mjs`) at load time, so production has zero knowledge of it. Import takes ~75ms.

## The rule that makes this trustworthy

**Drive it the way the product drives it.** The executor is not the product — the agent loop
supplies state and does bookkeeping the executor assumes. Omit that and the harness invents
failures nobody would ever hit.

This bit us immediately. The first run reported two "bugs" that were both the driver's fault:

| reported | actual cause |
|---|---|
| second write to the same path is blocked | run object had no `ctx.filesRead`, so `_runHasCurrentRead` returns false on its 3rd line — every write to an existing file blocked |
| negative path cache not cleared after write | that clear lives in the agent loop's post-mutation accounting, which driving the executor directly skips |

Both vanish once the driver supplies what the loop supplies. So: before believing a failure,
check whether the driver is being faithful. A harness that manufactures defects is worse than
no harness, because it becomes another green/red signal that means nothing.

Same discipline for the shims: `deepstub.mjs` layers **real** implementations over the proxy for
monaco APIs whose return values are consumed (`colorize` must return a real thenable). A blanket
deep proxy silently manufactures failures.

## Status

Scenarios 1, 2, 3, 5, 6, 7 run and pass against real disk.
Scenarios 4, 8, 9 are **not implemented** — they need `run_cmd` wired to real `child_process`.
No claim is made about them.
