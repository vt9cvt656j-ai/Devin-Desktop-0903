> **⚠️ 这份文档有多处已经过期成假话（2026-08-25 核对）。**
>
> 计划文档比没有文档更危险的地方在于：下一个人会照着它做决定，而它说的「已完成」
> 可能从来没成立过。已核实的错处：
>
> - **阶段 2c 说「留下来是因为还有独立消费点」——那句话是错的。** 它点名保留的
>   `_addedRuntimeObligations` / `_addedExternalObligations`，唯一的读取点是
>   `_agentAllowsRuntimeKind` / `_agentAllowsExternalKind` 两个函数，而那两个函数
>   **自己零调用点**。两个 Set 从头到尾只写不读，靠一层死函数假装有消费方，
>   连「只写不读」那道守卫都被骗过去了。四样已于 2026-08-25 全部删除。
> - **阶段 4 的前提已经反转。** 它假设"machinery gone"之后循环会收敛，而实测
>   main.js 30 天从 52,537 行涨到 83,200 行（+58%）。在按这份计划推进之前，
>   先看 `test/main-size-budget.test.mjs`——尺寸闸和「撞线先搬模块」的规矩在那里。
> - 阶段 1 那份「静默轮只在这三种情况续跑」的清单与代码对不上；"Where main.js
>   stands (measured)"表格最后一行引用的是已删函数；阶段 1 引用的 agent_core.txt
>   里那句话 grep 不到。**这三处已于 2026-08-25 逐条对着代码更正**，更正内容就写在
>   各自出处的下面（真实的四条续跑门连同上限、`_missingRequiredEffects` 的下落、
>   agent_core.txt 那句英文原文）。
>
> 同目录的 `WHY_SINGLE_DISPATCH_DIAGNOSIS.md` 和另一份编排诊断整篇在建议
> **加回阶段 3 刚被所有者点名删掉的自动派发**。照着做等于把删掉的东西装回去。

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
| loop stop condition | "model called no tools" | "model called no tools"，外加 4 条**执行事实**门 |

> 最后一行 2026-08-25 更正。原文写的 `_missingRequiredEffects` + 6 branches 已经不成立：
> 那个函数在阶段 2b 删干净了（main.js 里只剩两条记录它被删的注释）。现在的静默轮
> 决策是「模型没调工具 = 它的收尾决定，默认成立」，只有四条门能推翻它，每条都由
> 机器产生的事实驱动，且各自有上限、共用一个 3 轮的全局池：
>
> | 门 | 触发 | 上限 |
> |---|---|---|
> | 用户插话 | `session._steerQueue` 里真有消息 | 无（优先于其余三门，并清空所有计数器） |
> | 新增诊断 | 诊断相对基线有增量 | `_diagnosticNudges < 2` |
> | 构建红了 | 模型声明为验证的命令退出码非零 | `buildFixAttempts < 2` |
> | 计划未完 | 模型自己调 update_plan 留下的未完成步骤 | `_planFinishNudges < 2` |
>
> 另有两道全局关闸：非 agent 模式 / 用户拒绝 / 只读拦截（R0），以及连续两轮静默（R2）。
> 阶段 1 正文里那份「三种情况」的清单同样过期——它列的「付费自动子智能体整合」那条腿
> 在阶段 3 随自动派发一起删了，而真实存在的**计划门**它从头到尾没提。

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
  intent. **2026-08-25 更正**：原文写「`agent_core.txt` now lists 跑/运行 among
  must-produce-a-result intents」——grep 不到，因为那个文件通篇是英文，从来没有过
  中文词。它真正的对应句是开头第一句："when they ask you to modify, create, run,
  or deploy, use tools to actually complete"。意思在，引文是编的；照原文去 grep
  会得出「这条已经没了」的错误结论。
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
   - **2b (done).** Removed the same two prediction labels from the cap/exception
     final-accounting path and deleted `_missingRequiredEffects` (its last consumer).
     Only execution-fact labels remain.
   - **2c (done).** The contract's last live consumer was the steer-message effect diff:
     after an interjection it recomputed `_requiredEffectContract`, and any newly added
     `external:` effect rewrote the run as `requiresPlan = true; substantial = true` — a
     prediction driving control flow, and redundant besides: the same block re-runs
     `_mergeAiIntentProfile` on the fresh steer verdict a few lines above, so the model has
     already declared whether a plan is needed. Removed that diff, then deleted
     `_requiredEffectContract`, `_runRequiredEffect`, `_runEffectTarget`,
     `_effectTargetForTask` and the already-dead `_planEffectForRun`. `_cancelledEffectKinds`
     went with them (the contract was its only reader; keeping it would leave write-only
     state). Kept what has independent consumers: `_addedRuntimeObligations` /
     `_addedExternalObligations` still feed the prompt's obligation list, and
     `run._steeredWorkspaceRequired` is still read by the write-obligation check.
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
