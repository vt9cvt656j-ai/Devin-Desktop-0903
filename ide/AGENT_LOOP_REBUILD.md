# Agent loop rebuild — toward the Claude Code / opencode shape

Reference material studied: `~/Desktop/agent-reference/claude-code-analysis` and
`~/Desktop/agent-reference/OpencodeSrcChineseDoc`.

## The north star

Claude Code's entire loop is five steps (`src/query.ts`):

```
while (true) {
  turn = await streamModelTurn()          // call model, stream
  toolCalls = extractToolUse(turn)
  if (toolCalls.length === 0) break        // the model decides it is done, by stopping
  results = await runTools(toolCalls)      // partitioned concurrent / serial
  messages.push(...results)                // append, loop
}
```

opencode is identical in spirit. Neither has an intent classifier, an "obligation"
ledger, or a keyword floor anywhere in the loop. Behaviour is shaped by **the prompt**
and **the model**, plus a **declarative** permission layer (`allow`/`deny`/`ask` per
tool) and a tool registry where each tool declares `isReadOnly`/`isDestructive`/
`isConcurrencySafe` once. The one keyword regex in Claude Code (`wtf`, `this sucks`)
feeds a telemetry event and **touches no control flow**.

This is also what this project's own memory already says the architecture should be:
*judgment via model declarations + execution facts; regex only for permissions and
floors.* The loop drifted from it.

## Where `main.js` stands (measured)

| | Claude Code / opencode | `main.js` |
|---|---|---|
| intent-classifier functions | 0 | 27 |
| obligation / effect / contract refs | 0 | 51 |
| nudge / steer / gate functions | 0 | 55 |
| loop stop condition | "model called no tools" | `_missingRequiredEffects` + 6 branches |

## The line that matters

The loop's quiet-turn branch (model emitted no tool calls — the *same* signal Claude
Code treats as "done") currently runs an obligation ledger derived from a
keyword/profile guess of user intent, and can **force another turn** when that guess
says work is owed. That machine is:

- simultaneously over-built (hundreds of lines guessing intent) and full of holes —
  it had never heard of 运行, which is why "run my project" produced a paragraph;
- the thing every "just add a keyword" patch feeds.

## The distinction that governs every cut

Two kinds of re-entry trigger currently live in the stop decision. They are **not**
equal, and the whole rebuild rests on separating them:

- **Profile / keyword guess** — `_missingRequiredEffects`, `_missingResearchEvidence`,
  the `_noWorkNudged` gate. "The classifier thinks this task should have changed
  something." → **Remove.** The model decides when it is done; the prompt carries the
  intent (already: `agent_core.txt` now lists 跑/运行 among must-produce-a-result
  intents and says do it, don't offer to).
- **Observed execution fact** — `_diagnosticBlock` (fresh diagnostics the agent itself
  created), `_freshBuildFailure` (a command the model declared as verification exited
  nonzero). "A real, observed result is red." → **Keep.** This is the "execution facts"
  half of the declared architecture and matches Cursor's Agent Loop / Warp's verified
  terminal loop.

## Staged plan (each stage its own commit, full suite green before ship)

1. **Stop-decision pivot (load-bearing).** Quiet turn re-enters the loop only on an
   execution fact (open red build / fresh diagnostics), paid-for auto-subagent
   reconciliation, or a steer message. Remove the `_noWorkNudged` profile gate — the
   only place a profile guess still *forces* a turn. After this, the model stops when
   it stops. ← this commit.
2. **Demote the ledger to labeling, then delete it.** `_missingRequiredEffects` /
   `_missingResearchEvidence` / `_requiredEffectContract` / `_runRequiredEffect` /
   `_runEffectTarget` only record `_incompleteReason` after stage 1. Move honest final
   accounting to observed facts (did any mutation happen; did any declared verification
   run) and delete the functions. Untangle their secondary consumers (plan-quality,
   effect-cancel diff) first.
3. **Collapse the 27 intent functions.** Separate the two jobs they do today: (a)
   drive behaviour/obligations — remove; (b) rank tools + inject prompt context —
   keep, as a single small profile with no control-flow authority.
4. **Loop skeleton.** With the machinery gone, express the loop as the five-step
   skeleton above around the existing (good) partitioned tool executor, permission
   layer, and untrusted-content tagging.

## What is NOT touched

The parts that already match the reference architecture stay: the tool-policy
registry (`src/agent/tool-policy.js`), the permission gate (`_approveToolCall`),
untrusted-content tagging, the partitioned concurrent/serial tool executor, the
wiring linter (`test/wiring.test.mjs`).
