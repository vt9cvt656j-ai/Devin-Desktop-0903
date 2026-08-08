# Making this codebase easy to change

A working recipe, not a plan. The first migration is done and the tests prove it changed
nothing. Everything here is meant to be repeated a piece at a time, never as a big rewrite.

---

## The problem, measured

| | |
|---|---|
| `src/main.js` | 61,521 lines, top-level side effects, no exports |
| Test assertions coupled to source *text* | **1,221** (`extractFn` / `assert.match(SRC, …)`) |
| Copies of the mutation-family list | **11**, each maintained by hand |
| `_executeToolStepInner` | 4,461 lines, every tool in one `if / else if` |

These compound into one loop:

```
main.js can't be imported (top-level side effects)
   → tests must scrape source text to reach anything
      → 1,221 assertions welded to how the code LOOKS
         → improving code breaks tests that never tested behaviour
            → refactoring feels expensive, so the monolith grows
               → back to the top
```

That loop is what turns ordinary work into roadblocks. Nothing here is a skill problem — the
architecture makes local change impossible, so every change is a whole-system change.

---

## The exit: move a slice out, pin it, delete the duplication

Each migration is four steps and takes an afternoon. It is safe because step 2 proves the
refactor is a no-op *before* step 3 touches anything.

### 1. Write the module

A pure module under `src/agent/` — data and functions, no DOM, no I/O, no `main.js` imports.
Purity is the requirement that makes it importable, which is what kills the source-scraping.

### 2. Write pinning tests FIRST

Transcribe today's behaviour into assertions and run them against the new module *before*
changing `main.js`. They encode "this refactor changes nothing", which is the difference
between a safe migration and a leap of faith.

```js
test("approval set matches the pre-refactor literal exactly", () => {
  assert.deepEqual(sorted(approvalTypes()), sorted(new Set([
    "write", "edit", "multiedit", /* …transcribed from the old literal… */
  ])));
});
```

If one fails, either the module is wrong or you are making a real behaviour change — and the
second belongs in **its own commit**, never smuggled into a migration.

### 3. Replace the call sites

Import the module and delete the literals. Mechanical.

### 4. Add an anti-drift test

Stop the duplication growing back:

```js
const copies = (MAIN.match(/"write",\s*"edit",\s*"multiedit",\s*"delete"/g) || []).length;
assert.equal(copies, 0, "the family list must come from tool-policy.js");
```

---

## Migration 1 — tool policy (done)

**`src/agent/tool-policy.js`** + **`test/tool-policy.test.mjs`** (11 tests, all imported, zero
source scraping).

A tool declares capability flags once; every set the harness used to hard-code is a derived
query, so they can no longer disagree:

```js
defineTool("write", {
  mutatesWorkspace: true, fileMutation: true, fileEdit: true,
  needsApproval: true, hooked: true, readOnlyModeBlocked: true,
  recoverableBlock: true, scopeField: "path",
});
```

Removed from `main.js`: 8 hand-maintained lists, the 11-copy mutation family (now **0**), and
the eleven-term read-only `||` chain. Adding a read-only tool is now a **zero-edit** operation
(the default is correct); adding a side-effecting one is **one `defineTool` call** instead of
eleven edits.

Three things this immediately surfaced, which the flat lists had hidden:

- **`run_in_terminal` is not blocked in read-only modes.** Explorer / Plan / Reviewer can
  start a terminal task today. Invisible in an eleven-term chain; one obvious field in the
  registry. Preserved deliberately so the refactor stayed a no-op — closing it is
  `readOnlyModeBlocked: true` and should be its own commit.
- **`format` is not hooked**, unlike the other three content writers. Real, deliberate, and
  previously discoverable only by diffing two literals by eye.
- **`cmd` is not "workspace mutating"** — a command may change the workspace but never
  *reports* it, so trusting a missing `mutated` flag would mark every command a no-op.

It also made the test suite *more* faithful: extracted functions now receive the **real**
policy module instead of per-test stubs that could drift from the literal actually shipping.

---

## Next migrations, in order

Each is the same four steps. Ordered by pain removed per unit of risk.

**2. Tool result formatting** — `_toolResultToStringRaw`, `_toolMsgForModel`,
`_clipPreservingErrors`, the external-data tagging. Pure string transforms, heavily tested
today by source-shape assertions that would become real ones.

**3. Path & evidence helpers** — `_relCandidates`, `_resolveRel`, `_normRel`,
`_mergeReadRanges`, `_readRangeCovered`. Pure, self-contained, and the source of a large
share of the remaining 1,221 assertions.

**4. Command classification** — `_looksLikeReadOnlyCommand`, `_commandRiskKind`,
`_looksLikeVerificationCommand`, `_externalCommandKinds`. Pure predicates over strings;
currently untestable except by extraction.

**5. The tool execution registry** — the structural one. Give `defineTool` an `execute`, add a
front door in `_executeToolStep` that dispatches to a registered tool and **falls through to
the legacy dispatcher otherwise**. Then migrate tools one at a time, each a small verifiable
step, until `_executeToolStepInner` is empty. The policy layer already built is the half of
this that carries the risk; the rest is moving code.

---

## Two habits worth adopting

**Commit in coherent units.** At the start of this work the tree held 40 changed files and
8,614 uncommitted insertions across two days. That is the state where work gets lost and
where "what is actually true right now" stops being answerable.

**Stop writing diagnosis documents.** There are 40 in the repo root. Each was a real
investigation, but they are sediment, not knowledge — the same problems recur because the
findings land in prose beside the code instead of in structure. When something is worth
remembering, encode it as a test or a declaration. When it is not, let git history keep it.

The registry above replaced three such documents' worth of findings with one file and eleven
tests.
