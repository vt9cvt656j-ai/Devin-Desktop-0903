/**
 * Single source of truth for what a tool IS, so the harness stops re-deciding it.
 *
 * # The problem this replaces
 *
 * Every property the harness needs to know about a tool before running it — does it change
 * the workspace, does it need approval, may a read-only mode run it, does a worker's scope
 * apply, do repository hooks fire — was written as a literal list at the point of use. The
 * mutation family alone (`["write","edit","multiedit","delete","move","mkdir","copy","format"]`)
 * appeared **eleven times** in `main.js`, each an independent copy.
 *
 * That is the growth tax. Adding one tool meant finding eleven places and remembering the
 * right subset for each. Missing one is silent: a tool left out of the read-only-mode list is
 * quietly executable in Explorer/Plan/Reviewer, and nothing fails until a user notices.
 *
 * # The model
 *
 * A tool declares capability FLAGS once. Every named set the harness used to hard-code is a
 * derived query over those flags, so the sets can never disagree with each other again.
 *
 * Only tools whose policy differs from the default are listed. The default — read-only, no
 * approval, no hooks, runnable in every mode — is correct for the large majority of the
 * catalogue (every `*_search`, every lookup, every inspection tool), so listing them would be
 * noise that rots. A tool type that is not registered gets the default, which is also what
 * makes adding a read-only tool a zero-edit operation.
 *
 * # Adding a tool
 *
 *   defineTool("my_tool", { mutatesWorkspace: true, needsApproval: true });
 *
 * That is the whole change. No list hunting.
 *
 * This module is pure data + pure functions with no DOM, no I/O and no imports, so its tests
 * `import` it directly instead of scraping source text out of `main.js`.
 */

/** Policy for a tool type nobody has registered: safe, inert, universally available. */
export const DEFAULT_POLICY = Object.freeze({
  /**
   * The tool's result carries a trustworthy `mutated` boolean, so `mutated === false` is
   * evidence it really changed nothing (rather than absence of information).
   *
   * Deliberately NOT true for `cmd`/`termtask`: a shell command may well change the
   * workspace, but it does not report whether it did, and treating a missing flag as
   * "no-op" would mark every command as neutral.
   */
  mutatesWorkspace: false,
  /** One of the structured file operations (the family that shares read-before-edit, path
   *  binding, conflict handling). A generator that emits files mutates the workspace but is
   *  NOT a file operation, which is exactly the distinction the old lists kept blurring. */
  fileMutation: false,
  /** Writes file CONTENT specifically (write/edit/multi_edit/format) — the subset that
   *  participates in diagnostics and diff review. */
  fileEdit: false,
  /** Needs user approval when "approve before changes" is on. */
  needsApproval: false,
  /** Repository `pre_tool_use` / `post_tool_use` hooks fire around it. */
  hooked: false,
  /** Refused in the read-only modes (Explorer / Plan / Reviewer). */
  readOnlyModeBlocked: false,
  /** Which argument carries the path a worker sub-agent's scope is checked against.
   *  Empty string = this tool is not scope-checked. */
  scopeField: "",
  /** A [BLOCKED]/[CONFLICT] result is a recoverable policy stop rather than a hard failure,
   *  so it must not count toward the three-strike tool lockout. */
  recoverableBlock: false,
});

/** file-mutation defaults, shared by the eight structured file operations. */
const FILE_OP = {
  mutatesWorkspace: true,
  fileMutation: true,
  needsApproval: true,
  hooked: true,
  readOnlyModeBlocked: true,
  recoverableBlock: true,
};
/** the four that write content (and therefore reach diagnostics + diff review). */
const FILE_CONTENT_OP = { ...FILE_OP, fileEdit: true, scopeField: "path" };
/** a generator that lands assets in the workspace: mutating, but not a file operation. */
const GENERATOR = { mutatesWorkspace: true };
/** an execution tool: side effects the harness cannot verify, but no `mutated` report. */
const EXEC = { needsApproval: true, hooked: true };

const REGISTRY = new Map();

/**
 * Register or override a tool's policy. Unspecified fields fall back to the default, so a
 * declaration states only what is unusual about the tool.
 */
export function defineTool(type, policy = {}) {
  const name = String(type || "").trim();
  if (!name) throw new Error("defineTool requires a tool type");
  const unknown = Object.keys(policy).filter((k) => !(k in DEFAULT_POLICY));
  // A typo'd flag would silently do nothing — exactly the class of bug this module exists to
  // remove — so refuse it at declaration time instead of at 3am.
  if (unknown.length) throw new Error(`unknown tool policy field(s) for "${name}": ${unknown.join(", ")}`);
  REGISTRY.set(name, Object.freeze({ ...DEFAULT_POLICY, ...policy }));
  return REGISTRY.get(name);
}

/** Every declaration whose policy differs from the default. */
function seed() {
  // ── structured file operations ────────────────────────────────────────────
  for (const t of ["write", "edit", "multiedit"]) defineTool(t, FILE_CONTENT_OP);
  // `format` writes content like the other three, but repository hooks deliberately do NOT
  // fire for it: formatting is a mechanical rewrite of code the hooks already saw, and firing
  // a lint hook on every auto-format was noise.
  defineTool("format", { ...FILE_CONTENT_OP, hooked: false });
  defineTool("mkdir", { ...FILE_OP, scopeField: "path" });
  defineTool("copy", { ...FILE_OP, scopeField: "path" });
  // delete/move are refused outright for workers rather than scope-checked (a parallel child
  // deleting or relocating files is a conflict source no scope can make safe), so they carry
  // no scopeField — the executor's own worker guard rejects them earlier.
  defineTool("delete", FILE_OP);
  defineTool("move", FILE_OP);

  // ── command execution ─────────────────────────────────────────────────────
  defineTool("cmd", { ...EXEC, readOnlyModeBlocked: true });
  // KNOWN GAP, preserved on purpose. `termtask` (run_in_terminal) is absent from the
  // read-only-mode block, so Explorer / Plan / Reviewer can currently start a terminal task
  // even though those modes are meant to be read-only. It was invisible while the rule lived
  // in an eleven-term `||` chain; here it is one field, and closing it is a one-word change
  // (`readOnlyModeBlocked: true`). Left as-is so this refactor provably changes nothing —
  // a behaviour fix belongs in its own commit, not smuggled into a "no-op" migration.
  defineTool("termtask", EXEC);

  // ── other side-effecting tools ────────────────────────────────────────────
  defineTool("mcp", { needsApproval: true, readOnlyModeBlocked: true });
  defineTool("uiclick", { needsApproval: true, readOnlyModeBlocked: true });
  defineTool("automation", { needsApproval: true });
  defineTool("db", { needsApproval: true });
  defineTool("download", { mutatesWorkspace: true, needsApproval: true });

  // ── generators that land assets in the workspace ──────────────────────────
  for (const t of [
    "game_scaffold", "web_scaffold", "download_asset", "genimage", "generate_3d",
    "generate_sound", "generate_music", "generate_voice", "auto_rig", "generate_motion",
    "generate_texture",
  ]) defineTool(t, GENERATOR);
}
seed();

/** Resolved policy for a tool type. Never throws; unknown types get the safe default. */
export function toolPolicy(type) {
  return REGISTRY.get(String(type || "")) || DEFAULT_POLICY;
}

/** Every registered type whose policy satisfies `predicate`. */
export function typesWhere(predicate) {
  const out = new Set();
  for (const [type, policy] of REGISTRY) if (predicate(policy, type)) out.add(type);
  return out;
}

// ── The named sets the harness used to hard-code, now derived ───────────────
// Exported as live getters rather than frozen constants so a `defineTool` call at startup
// (a plugin, a future MCP-backed tool) is reflected everywhere instead of only in the
// call sites that happened to be evaluated after it.
export const workspaceMutatingTypes = () => typesWhere((p) => p.mutatesWorkspace);
export const fileMutationTypes = () => typesWhere((p) => p.fileMutation);
export const fileEditTypes = () => typesWhere((p) => p.fileEdit);
export const approvalTypes = () => typesWhere((p) => p.needsApproval);
export const hookedTypes = () => typesWhere((p) => p.hooked);
export const readOnlyBlockedTypes = () => typesWhere((p) => p.readOnlyModeBlocked);

// ── Predicates: what call sites should actually use ─────────────────────────
export const mutatesWorkspace = (type) => toolPolicy(type).mutatesWorkspace;
export const isFileMutation = (type) => toolPolicy(type).fileMutation;
export const isFileEdit = (type) => toolPolicy(type).fileEdit;
export const needsApproval = (type) => toolPolicy(type).needsApproval;
export const isHooked = (type) => toolPolicy(type).hooked;
export const blockedInReadOnlyMode = (type) => toolPolicy(type).readOnlyModeBlocked;
export const hasRecoverableBlock = (type) => toolPolicy(type).recoverableBlock;

/**
 * Which argument of `call` a worker sub-agent's scope is checked against, or "" when the tool
 * is not scope-checked. Returning the FIELD rather than a boolean keeps the executor from
 * re-deriving "which property holds the path for this tool", which is the same duplication in
 * a different costume.
 */
export const workerScopeField = (type) => toolPolicy(type).scopeField;

/** The concrete path a worker's scope applies to, or "" when the tool is unscoped. */
export function workerScopeTarget(call) {
  if (!call) return "";
  const field = workerScopeField(call.type);
  if (!field) return "";
  return String(call[field] || call.dest || call.to || "");
}

/** Snapshot of every declaration — for diagnostics and for the drift test. */
export function allPolicies() {
  return Object.fromEntries([...REGISTRY].map(([type, policy]) => [type, { ...policy }]));
}
