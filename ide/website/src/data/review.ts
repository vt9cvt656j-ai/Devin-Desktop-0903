/**
 * A code review of this repository, written by the AI that worked in it.
 *
 * This is NOT a customer testimonial and must never be presented as one — the card
 * that renders it says so explicitly. It is published because it is a genuine
 * assessment of the codebase, criticisms included. If the criticisms are ever
 * removed, take the card down rather than leave a review that only praises.
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
    "The native claim holds: 38,900 lines of Rust across 35 focused modules, against 18 runtime dependencies.",
  ],
  criticisms: [
    "src/main.js is 59,700 lines. One file holds the entire frontend, which makes every change riskier than it needs to be.",
    "Eight interface languages ship, but roughly four thousand user-visible strings are still Chinese-only — including the Memory Center and the agent's own closing summary.",
  ],
};
