# End-to-end harness — spec

## Why this exists

`npm test` passes 626 tests while the IDE visibly fails in real use. Both are true because
every existing test asserts on **source text** or **pure predicates**. None of them runs the
agent loop, touches a real file, or checks whether a task completed.

So "all green" has never meant "it works", and every fix has been declared done on evidence
that could not detect the failures the user was actually seeing. This harness is the fix to
that. Until it exists, no change to the agent loop should be called verified.

## The one rule

**A change is done when the harness completes the task, not when a unit test passes.**

## Scope, in build order

### Phase 1 — real filesystem, real executor (the highest value, do this first)

Drive `_executeToolStepInner` against a real temp directory with a scripted tool-call
sequence, and assert on **files on disk**, not on returned strings.

Blocker to solve first: `_executeToolStepInner` is ~2k lines with a large free-identifier
surface and DOM access (`step.querySelector`). Two viable approaches, in preference order:

1. **Load `src/main.js` whole in Node** behind a shim module that provides `document`,
   `window`, `performance`, Monaco stubs, and a `backend` object whose fs methods are real
   `node:fs` calls rooted at a temp dir. Export a test hook (`globalThis.__testHooks`) guarded
   by an env var so production is unaffected. This is the real thing and is worth the effort.
2. If (1) proves impractical, extract with a **proper JS parser** (acorn is already a dep) —
   naive brace matching fails, it swallowed 312KB because braces appear inside strings,
   regexes and comments.

Scenarios to cover, all taken from failures observed in real use:

| # | Scenario | Passing means |
|---|----------|---------------|
| 1 | write a new file into an empty dir | file exists with exact content |
| 2 | read → edit → read | edit applied, no CONFLICT |
| 3 | write, then write the same path again | second write lands (no stale-preview block) |
| 4 | read a file, run a command that rewrites it, then write | write succeeds, CAS caught nothing spurious |
| 5 | create a dir, then read a file inside it | no `[SKIPPED_EMPTY_WORKSPACE]` |
| 6 | read a missing path twice, then write it, then read it | read succeeds (negative cache cleared) |
| 7 | write a file with a >55k single line | write allowed (coverage-impossible bypass) |
| 8 | `cd <dir> && npx vite build 2>&1` declared `verify` | earns verification credit |
| 9 | a command that writes files via a script | `_fsDelta` true, obligation armed |

### Phase 2 — the loop, with a scripted model

Replace the model call with a fixture that replays a recorded tool-call sequence. Asserts the
loop's *control flow*: gates fire the right number of times, nudges are delivered once, the run
terminates, `_incompleteReason` is correct. No network.

### Phase 3 — live model smoke (the real number)

A handful of tasks against a scratch repo with a real model, scored only on
**did it finish without human intervention**. This is the only thing that can answer
"how close are we to Claude Code" — everything above is necessary but not sufficient.

## Status board

The harness writes `test/e2e-status.json` each run: scenario → pass/fail → timestamp.
That file, not chat prose and not memory, is the answer to "what actually works right now".

## Known-open defects (verify each against the harness once it exists)

- auto-fan-out: one worker serially does N files instead of N workers in parallel
- `_resolveRel` cannot reach a self-named subdirectory (`rpa-site/rpa-site`)
- weak-model window convergence branch is unreachable (`routeToolNames.length > 10` on a
  list capped at 10)
- engineering intent profiles still derive from regex; migrate to declarations + facts
- `_looksLikeProjectExecutionCommand` still routes read-only/side-effect decisions
