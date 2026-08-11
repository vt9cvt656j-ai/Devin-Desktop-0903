// The semantic profile decides which blocks the gateway puts in its system prefix, and the prefix
// sits at byte 0 of every request. Change it mid-session and the provider's cache misses on the
// prefix AND on the whole conversation behind it — the expensive part. This project already paid
// for that lesson once, with a date block that carried minutes and a measured 2% hit rate.
//
// Real functions are pulled out of src/main.js with acorn, following model-resume.test.mjs: a
// reimplementation here would only prove the copy is self-consistent.
import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs";
import * as acorn from "acorn";

const SRC = fs.readFileSync("src/main.js", "utf8");
const ast = acorn.parse(SRC, { ecmaVersion: "latest", sourceType: "module" });
function grab(name) {
  for (const n of ast.body) {
    if (n.type === "FunctionDeclaration" && n.id?.name === name) return SRC.slice(n.start, n.end);
  }
  throw new Error("missing " + name);
}
const load = (name, need) => new Function(need.map(grab).join("\n") + `\nreturn ${name};`)();

const stable = load("_sessionStableSemanticProfile", ["_sessionStableSemanticProfile"]);
const profileOf = load("_ideSemanticProfile", ["_ideSemanticProfile"]);

test("a turn that classifies narrower does not drop the session's blocks", () => {
  const session = {};

  // Turn 1, the real classifier shape for "fix the login page and run it".
  const t1 = stable(session, profileOf({ ui: true, workspaceAction: "modify", applies: true }));
  assert.equal(t1, "2.5:engineering,design,design_implementation,design_verification");

  // Turn 5 of the same session: "still broken, check why the window won't show". Nothing in that
  // sentence reads as UI work, so the per-turn classifier drops every design flag. Before this
  // was sticky, that rewrote the prefix — 34,973 bytes to 26,276 — and cost a full cache miss on
  // the entire conversation. It has to stay put: the user is still fixing the same login page.
  const t5 = stable(session, profileOf({ applies: true }));
  assert.equal(t5, t1, "a narrower turn rewrote the prefix mid-session");
});

test("a session that genuinely widens pays once and then settles", () => {
  const session = {};
  const t1 = stable(session, profileOf({ ui: true, workspaceAction: "modify", applies: true }));

  // The user now asks for a commit — git is real new capability, so the prefix legitimately grows.
  const t2 = stable(session, profileOf({ applies: true, git: true }));
  assert.notEqual(t2, t1, "a genuinely new capability should reach the gateway");
  assert.ok(t2.startsWith(t1), "widening must append, not reorder — reordering is a needless miss");
  assert.ok(t2.endsWith(",git"));

  // ...and every turn after that is byte-identical, which is the whole point.
  assert.equal(stable(session, profileOf({ applies: true })), t2);
  assert.equal(stable(session, profileOf({ ui: true, workspaceAction: "modify", applies: true })), t2);
  assert.equal(stable(session, profileOf({ applies: true, git: true })), t2);
});

test("stickiness is per session, so a new session starts focused", () => {
  const a = {};
  stable(a, profileOf({ ui: true, workspaceAction: "modify", applies: true, fullWebsite: true }));

  // Not a global accumulator: the full flag set assembles an 84KB prefix against 26KB here, and
  // blocks the model does not need are more instructions competing with the ones it does.
  const b = {};
  assert.equal(stable(b, profileOf({ applies: true })), "2.5:engineering");
});

test("no session (background/one-shot request) passes through unchanged", () => {
  const header = profileOf({ applies: true, git: true });
  assert.equal(stable(null, header), header);
  assert.equal(stable(undefined, ""), "");
});

test("the session's flags survive a restart and are dropped on a rewind", () => {
  // Session persistence is an explicit whitelist, so a field nobody lists silently does not
  // survive — and a resumed session would then pay a full prefix miss on its first turn for
  // nothing. Two serializers and two rehydrators, matching how _intentState is handled.
  assert.equal((SRC.match(/semanticFlags: Array\.isArray\(/g) || []).length, 2,
    "both session serializers must write the flags");
  assert.equal((SRC.match(/if \(Array\.isArray\(sData\.semanticFlags\)\)/g) || []).length, 2,
    "both rehydrators must read them back");

  // Rewinding deletes a message and everything after it, so whatever capabilities that stretch
  // of the conversation needed stop counting. It is also the one free moment to re-derive:
  // truncation has already invalidated the cache, so nothing is lost by starting over.
  assert.match(SRC, /sess\._intentState = null; sess\._lastRunState = null; sess\._semanticProfileFlags = null;/,
    "a rewind must drop the accumulated flags with the conversation it discarded");
});

test("every profile write goes through the sticky merge", () => {
  // Three places assign config.ideSemanticProfile: the turn itself, a late intent verdict, and
  // steering mid-run. The last two also narrow, so a raw assignment anywhere reopens the bug.
  const assignments = SRC.match(/config\.ideSemanticProfile\s*=\s*[^;]+;/g) || [];
  assert.ok(assignments.length >= 3, "expected the known profile writes to still exist");
  for (const line of assignments) {
    assert.match(line, /_sessionStableSemanticProfile\(/, `unmerged profile write: ${line}`);
  }
});

// ---------------------------------------------------------------------------
// File tree — the explorer rows.
// ---------------------------------------------------------------------------
const APP_CSS = fs.readFileSync("src/styles/app.css", "utf8");
const REFINE_CSS = fs.readFileSync("src/styles/refine.css", "utf8");

test("a workspace root does not wear the file selection", () => {
  // The root row carries .is-active whenever it is the current workspace, so it rendered the
  // same filled highlight as the open file directly beneath it — two solid rows touching, which
  // is what made the explorer read as one clump instead of a tree.
  assert.match(APP_CSS, /\.workspace-root__row\.is-active::before \{ background: transparent; \}/,
    "the root must not paint the active-file highlight");
  assert.match(APP_CSS, /\.workspace-root__row\.is-active \.name \{ color: var\(--text\); \}/,
    "which root is current has to remain readable some other way");
});

test("the row highlight spans the panel instead of sitting in a pill", () => {
  // Indentation comes from nested .children wrappers, so a background on .row itself starts at
  // the indent. The highlight is a pseudo-element stretched past the left edge instead.
  assert.match(APP_CSS, /\.row::before \{[^}]*left: -1000px;/s, "the highlight must extend past the indent");
  assert.match(APP_CSS, /\.row:hover::before \{ background: var\(--hover\); \}/);
  assert.match(APP_CSS, /\.row\.is-active::before \{ background: var\(--sel\); \}/);
  // A background on .row would be drawn at the indented width and defeat the whole thing.
  const rowBlock = APP_CSS.slice(APP_CSS.indexOf("\n.row {"), APP_CSS.indexOf("\n.row::before"));
  assert.doesNotMatch(rowBlock, /background:/, ".row itself must stay transparent");
  assert.doesNotMatch(rowBlock, /border-radius:/, "no pill");
  assert.match(rowBlock, /height: 22px;/, "VS Code's row height");
});

test("the tree stylesheet has no selectors that match nothing", () => {
  // refine.css carried a .tree-item / .tree__row / #tree [role="treeitem"] block whose comment
  // claimed to have fixed the row spacing. None of those three exist in this DOM, so it never
  // applied — the comment described a fix that was not happening, which is worse than no comment.
  // Compare rules only; the note left in its place naturally mentions the dead names.
  const rules = REFINE_CSS.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const dead of [".tree-item", ".tree__row", 'role="treeitem"']) {
    assert.ok(!rules.includes(dead), `${dead} matches no element and must not be styled`);
  }

  assert.match(SRC, /document\.querySelector\(`\.row\[data-path=/,
    "revealInTree must query the class the rows actually have — `.tree-item` matched nothing, so " +
    "every caller, including the agent marking each file it touches, silently did nothing");
  assert.match(APP_CSS, /\.row\.flash::before \{ animation: tree-flash/,
    "and .flash needs a style behind it, which it never had either");
});

test("archive handling is reachable from the UI, not just implemented in the backend", () => {
  // The reader existed for a while before anything could call it. A capability nobody can invoke
  // is indistinguishable from one that does not exist.
  assert.match(SRC, /extractArchive: \(path, dest, budget\) => core\.invoke\("extract_archive"/,
    "the native seam must expose extraction");
  assert.match(SRC, /readArchiveEntry: \(path, entry, maxBytes\) =>\s*core\.invoke\("read_archive_entry"/,
    "and single-entry reads");
  // Both stubs must exist too, or the web build throws an obscure undefined-is-not-a-function
  // instead of saying the desktop app is required.
  assert.match(SRC, /extractArchive: async \(\) => \{ throw new Error\("解压需要桌面版应用。"\); \}/);

  // The panel is a React island now; the host and the mount are what make it reachable.
  assert.match(SRC, /data-archive-browser-host/, "the panel needs a host to mount into");
  assert.match(SRC, /mountArchiveBrowser\(host, \{/, "and something has to mount it");
  assert.match(SRC, /if \(info\?\.archive\) _mountArchiveBrowser\(path, info\.archive\);/,
    "mounted after each render — the inspector rebuilds its body with innerHTML, so the previous " +
    "host is gone every time");

  // Archive content is untrusted input; it is rendered as text, never as markup.
  const preview = SRC.slice(SRC.indexOf("function _showArchiveEntryPreview"), SRC.indexOf("function _inspectionArchiveHtml"));
  assert.match(preview, /querySelector\("pre"\)\.textContent = /,
    "entry content must go in as textContent — innerHTML here is a script-injection path straight " +
    "out of a downloaded archive");
  const template = preview.slice(preview.indexOf("wrap.innerHTML = `"), preview.indexOf("// textContent"));
  assert.doesNotMatch(template, /content[?.]*\.text/,
    "the entry's own bytes must never be interpolated into markup — only its name and size, both escaped");
});

test("the inspector header cannot be crushed by its own buttons", () => {
  // A fourth action button squeezed the title column to a few pixels, and CJK — which may break
  // between any two characters — rendered one character per line. min-width:0 was already there
  // and is what permitted it: the floor has to come from the basis, with the header wrapping.
  const header = APP_CSS.slice(APP_CSS.indexOf(".file-inspector__header {"), APP_CSS.indexOf(".file-inspector__icon {"));
  assert.match(header, /flex-wrap: wrap;/, "the header must wrap rather than compress the title");
  const heading = APP_CSS.slice(APP_CSS.indexOf(".file-inspector__heading {"), APP_CSS.indexOf(".file-inspector__eyebrow {"));
  assert.match(heading, /flex: 1 1 260px;/, "and the title needs a real basis, not flex-basis auto");
});
