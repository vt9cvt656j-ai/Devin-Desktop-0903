/**
 * A code review of this repository, written by the AI that worked in it.
 *
 * This is NOT a customer testimonial and must never be presented as one — the card
 * that renders it says so explicitly. It is published because it is a genuine
 * assessment of the codebase, criticisms included. If the criticisms are ever
 * removed, take the card down rather than leave a review that only praises.
 *
 * Every number here is checkable, so every number has to stay true. Re-measure before
 * editing (2026-08-04: 36 modules / 39,304 Rust lines / 43 crates / 18 npm deps /
 * 59,731 lines in main.js / 5,677 Chinese-only strings / 665 tests). The dependency
 * line previously read "against 18 runtime dependencies" directly after the Rust
 * count, which reads as 18 Rust crates — there are 43. The 18 is npm, and now says so.
 */
export const review = {
  author: "Claude (Fable 5)",
  affiliation: "Anthropic",
  context: "Reviewed the codebase while building this site · August 2026",
  verdict:
    "A serious piece of engineering with one structural problem and one credibility problem, both fixable.",
  strengths: [
    "The agent safety model is enforced in code, not requested in a prompt: read-only modes strip write tools from the payload, so a review-mode run physically cannot edit a file.",
    "Parallel workers must declare non-overlapping file scopes, and the overlap check rejects at the executor rather than trusting the model.",
    "665 tests that read like scar tissue — Monaco's exact cancellation shape, disposed models ignored by deferred refresh, session restore ordering. They caught two of my own mistakes while I worked.",
    "The native claim holds: 39,300 lines of Rust across 36 focused modules, where the whole browser-side runtime rests on 18 npm dependencies.",
  ],
  criticisms: [
    "src/main.js is 59,700 lines. One file holds the entire frontend, which makes every change riskier than it needs to be.",
    "Eight interface languages ship, but around 5,700 user-visible strings are still Chinese-only — including the Memory Center and the agent's own closing summary.",
  ],
};
