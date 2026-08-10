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

test("every profile write goes through the sticky merge", () => {
  // Three places assign config.ideSemanticProfile: the turn itself, a late intent verdict, and
  // steering mid-run. The last two also narrow, so a raw assignment anywhere reopens the bug.
  const assignments = SRC.match(/config\.ideSemanticProfile\s*=\s*[^;]+;/g) || [];
  assert.ok(assignments.length >= 3, "expected the known profile writes to still exist");
  for (const line of assignments) {
    assert.match(line, /_sessionStableSemanticProfile\(/, `unmerged profile write: ${line}`);
  }
});
