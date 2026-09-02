# Prompt graph

Every production IDE request is assembled from `prompt_graph.json`. The graph and its modules are
required deployment artifacts; a missing or invalid graph fails the request instead of restoring a
monolithic prompt.

## Runtime layers

- Chat/plan/explorer/reviewer: declared explicitly under `modes`
- Base: `agent_core.txt`, `reasoning.txt`, `truthfulness.txt`, `answer_quality.txt`
- Engineering: `agent_engineering.txt`
- Research: `agent_research.txt`
- Browser/desktop automation: `agent_automation.txt`
- Git: `git_guide.txt`
- UI base: `design_core.txt`
- UI implementation entry: `design_implementation.txt`
- UI components/tokens/icons/type: `design_components.txt`
- New-project framework wiring: `design_scaffold.txt`
- Content and real media: `design_content.txt`
- Persistent data and business states: `design_data.txt`
- UI engineering/accessibility/performance: `design_engineering.txt`
- UI review: `design_verification.txt`
- UI implementation verification: `design_verification.txt`
- Motion/full-site work: `design_motion.txt`

Intent is sticky across a bounded conversation window, so follow-ups such as "continue" keep the
same module set and upstream prompt-cache prefix. Browser automation that merely opens or logs into
a website does not activate UI design modules unless the request also asks for design work.

UI routing is scope-aware. A focused component/style change loads base + implementation +
components + engineering + verification and a small two-hit design evidence packet. A full site
also loads scaffold, content/media, data, motion, and a larger bounded blueprint packet. Product
category nouns such as finance, health, restaurant, travel, portfolio, or game do not activate the
research prompt unless the user actually asks to research, search, compare current facts, or find
evidence.

`knowledge/michael-design/` remains the full design corpus. UI assembly injects a bounded set of
relevant excerpts and leaves deeper retrieval to `knowledge_search`; the corpus files are not
rewritten or duplicated into prompts.

## Change rules

1. Add or reorder modules in `prompt_graph.json`.
2. Add every routed `.txt` module to `PROMPT_NAMES` in `src/prompts.rs` so prompt versions change.
3. Keep module selection stable across follow-up turns and avoid duplicating rules in multiple
   runtime modules.
4. Add new runtime detail to the narrowest routed module and extend the strength-contract test.
5. Never add a monolithic fallback path; graph/module failures must stay visible.
6. Run `cargo test prompts::tests -- --test-threads=1`, then the full server test suite.
