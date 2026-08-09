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
2. **Demote the ledger to labeling, then delete it.**
   - **2a (done).** Removed the ledger from the FINISH path: the quiet-turn accounting no
     longer sets the prediction-derived `required_effect_missing:` /
     `research_evidence_missing:` labels (only execution-fact labels remain:
     `code_delivered_unverified`, `semantic_runtime_review_missing`, `build_failing`,
     `new_diagnostics_unresolved`). Fact-grounded the "keeps searching, never acts" churn
     breaker: it triggered on `_requiredEffectContract` (which metric the classifier
     predicted mattered); now it triggers on total observed progress being zero
     (`_implOps + _runtimeEffects + _externalEffects === 0`), which needs no prediction and
     fires strictly less often. Dropped two ledger consumers.
   - **2b (todo).** Remove the same two prediction labels from the cap/exception
     final-accounting path, then delete `_missingRequiredEffects` (its last consumer).
   - **2c (todo).** Untangle `_requiredEffectContract`'s remaining consumers — the
     steer-message effect diff (sets `requiresPlan`) and plan-quality (`_planEffectForRun`)
     — then delete it and its feeders (`_runRequiredEffect`, `_runEffectTarget`,
     `_effectTargetForTask`).
   - The pre-write research gate (`_missingResearchEvidence` at the write site) is a
     non-blocking nudge + tool-loading, not a finish obligation — it belongs to stage 3's
     "inject context / rank tools" question, evaluated there, not rushed here.
3. **Collapse the intent functions (done — with a correction).** On audit, the "27
   functions" were mostly the RIGHT pattern, not the disease: the AI intent classifier IS
   the model making the judgment; the plan-quality grader is a fail-open fallback (AI
   review primary, keyword scorecard only when it is unavailable, output non-blocking);
   the rest rank tools / inject prompt context. Deleting those would remove correct code.
   The one genuinely anti-thesis piece was the orchestration subsystem — the harness
   *acting* on a prediction rather than informing. Removed in full (Michael's call):
   - the IDE auto-spawning up to four file-writing sub-agents the model never requested
     when a plan crossed ≥2 domains (parallel writers on one tree — the conflict class the
     worker guard exists to prevent, created automatically);
   - the three profile-driven nudges pushing the model to split/parallelize/not-single-
     dispatch (`_splitGateNudgeMessage`, `_inferOrchestrationFromPlan`,
     `_shouldDispatchSubagent`), plus the dead `_soloExecutionFactLine`;
   - the finish-time auto-integration leg and its bounded wait, which only ever served the
     auto-dispatched children (dead once they were gone).
   The "when to parallelize" decision moved to `agent_collaboration.txt` (the model owns it,
   with the tools named). Net −199 lines in the loop; 6 tests migrated (5 deleted with the
   machinery, 1 added asserting the capability lives in the prompt now).
4. **Loop skeleton.** With the machinery gone, express the loop as the five-step
   skeleton above around the existing (good) partitioned tool executor, permission
   layer, and untrusted-content tagging.

## What is NOT touched

The parts that already match the reference architecture stay: the tool-policy
registry (`src/agent/tool-policy.js`), the permission gate (`_approveToolCall`),
untrusted-content tagging, the partitioned concurrent/serial tool executor, the
wiring linter (`test/wiring.test.mjs`).
