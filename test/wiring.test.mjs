// The wiring linker.
//
// Every serious defect found while auditing this codebase was the same shape: code that
// EXISTS but nothing reaches. Not a crash, not a failing test — a silent hole that reads as
// working software.
//
//   * `_approveToolCall` returned `true` and had zero call sites, silently disabling the
//     entire permission layer beneath it (_APPROVE_TYPES, _requiresApproval, _sessionApproved,
//     _isDangerousCmd, the deny list). The settings switch had been REMOVED rather than the
//     code fixed.
//   * `run._diagnosticBlock` was assigned `""` at three sites and never anything else, so the
//     delivery gate reading `!run._diagnosticBlock` was permanently vacuous.
//   * `checkWorkspaceTrust(root)` was called and its verdict discarded, so declining the trust
//     prompt still launched every MCP server declared in the repo.
//   * `_missingRequiredEffects` recorded that nothing had been done, then let the run end.
//
// A type checker catches none of these: every one is well-typed and unreachable. This is the
// linker for that class, run in the normal suite so a refactor that severs a connection fails
// here rather than in production weeks later.
//
// Rules for anything added below:
//   1. Assert a CONNECTION, never an implementation. "X is reachable from Y" survives
//      refactoring; "X is written this way" is the source-text coupling that has already
//      broken this suite repeatedly.
//   2. Prefer under-reporting to false positives. A check that cries wolf gets deleted, and
//      then it protects nothing.
//   3. Every check names the real defect it would have caught.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(HERE, "../src/main.js"), "utf8");

// Deliberately NOT comment-stripped. A naive stripper is unsafe here: this file contains
// regex literals holding `/*` and `//`, and a stripper that mis-parses one silently deletes a
// huge span — which is exactly how the first version of this linker reported that
// `_approveToolCall` had no call sites at all. Every pattern below is chosen so that prose
// cannot satisfy it (`await X(`, `case "x"`), which makes stripping unnecessary.

/** Body of a top-level function, brace-matched. */
function fnBody(name) {
  const m = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).exec(SRC);
  if (!m) throw new Error(`function ${name} not found`);
  let p = SRC.indexOf("(", m.index), pd = 0;
  for (; p < SRC.length; p++) {
    const c = SRC[p];
    if (c === "(") pd++; else if (c === ")") { pd--; if (!pd) break; }
  }
  let i = SRC.indexOf("{", p), d = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i];
    if (c === "{") d++; else if (c === "}") { d--; if (!d) return SRC.slice(m.index, i + 1); }
  }
  throw new Error(`unbalanced braces in ${name}`);
}
/**
 * Evaluate a module-level `const NAME = <expr>;` out of main.js.
 *
 * The linker's whole point is checking the shipped literals rather than a copy of them, so a
 * table a test needs is read from source instead of restated here — a restatement is exactly the
 * kind of second copy that drifts. Brace/paren/bracket depth is tracked so the terminating `;`
 * is the declaration's own, not one inside the initialiser.
 */
function loadConst(name) {
  const m = new RegExp(`\\bconst\\s+${name}\\s*=`).exec(SRC);
  if (!m) throw new Error(`const ${name} not found in main.js`);
  let i = SRC.indexOf("=", m.index) + 1, depth = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i], d = SRC[i + 1];
    if (c === "/" && d === "/") { i = SRC.indexOf("\n", i); if (i < 0) i = SRC.length; continue; }
    if (c === "/" && d === "*") { i = SRC.indexOf("*/", i + 2) + 1; continue; }
    if (c === '"' || c === "'" || c === "`") {
      const q = c; i++;
      while (i < SRC.length && SRC[i] !== q) i += SRC[i] === "\\" ? 2 : 1;
      continue;
    }
    if (c === "(" || c === "[" || c === "{") depth++;
    else if (c === ")" || c === "]" || c === "}") depth--;
    else if (c === ";" && depth === 0) {
      return new Function(`${SRC.slice(m.index, i + 1)}\n;return ${name};`)();
    }
  }
  throw new Error(`unterminated declaration extracting ${name}`);
}
const uniq = (xs) => [...new Set(xs)];
const grab = (text, re) => uniq([...text.matchAll(re)].map((m) => m[1]));
const count = (text, re) => (text.match(re) || []).length;

// ─────────────────────────────────────────────────────────────────────────────
// 1. Tool chain: schema → mapper → handler
// ─────────────────────────────────────────────────────────────────────────────
//
// A tool the model can SEE but not RUN is the worst hole of all: the model calls it, gets
// nothing back, and burns turns retrying something that can never work.
//
// "Handled" is checked against the WHOLE file on purpose. Tools are dispatched from several
// layers — `_executeToolStepInner`, interception inside `_runAgenticLoop` (sub-agents,
// search_tools), and narrative-control paths (update_plan, ask_user). Scoping the search to
// two functions produced a false positive on `plan` in the first version. The invariant worth
// protecting is "something handles it", not "this specific function does".

const schemaNames = grab(SRC, /function:\s*\{\s*name:\s*"([a-z0-9_]+)"/g);
const mapperCases = grab(fnBody("_mapToolCall"), /case\s+"([a-z0-9_]+)"/g);
const producedTypes = grab(fnBody("_mapToolCall"), /type:\s*"([a-z0-9_]+)"/g);
const handledTypes = new Set([
  ...grab(SRC, /call\.type === "([a-z0-9_]+)"/g),
  ...grab(SRC, /it\.call\.type === "([a-z0-9_]+)"/g),
  ...grab(SRC, /\bcase "([a-z0-9_]+)":/g),
  ...grab(SRC, /"([a-z0-9_]+)"\]\.includes\(call\.type\)/g),
  ...grab(SRC, /\.has\(call\.type\)|_TYPES = new Set\(\[([^\]]*)\]/g),
]);

// Orchestration tools are dispatched on the TOOL NAME inside `_runAgenticLoop`, before the
// call ever reaches the type-based dispatcher — they spawn children rather than run a tool.
// Their `call.type` is therefore legitimately never matched, so they are checked by name
// instead. Listing them explicitly (rather than widening the type scan until it goes quiet)
// keeps the check meaningful: removing one of these names still fails.
const NAME_DISPATCHED = {
  spawnmulti: "spawn_multiple_agents",
  subagent: "run_subagent",
  worker: "run_worker",
  debate: "debate",
};

test("every tool the model can call is reachable end to end", () => {
  // Link 1: schema → mapper. No case means `{type:"unknown"}` — a tool that silently no-ops.
  const unmapped = schemaNames.filter((n) => !mapperCases.includes(n));
  assert.deepEqual(unmapped, [],
    `advertised to the model but _mapToolCall has no case: ${unmapped.join(", ")}`);

  // Link 2: mapper → some handler, by type or by name.
  const loop = fnBody("_runAgenticLoop");
  const orphans = producedTypes.filter((t) => {
    if (handledTypes.has(t)) return false;
    const byName = NAME_DISPATCHED[t];
    return !(byName && loop.includes(`"${byName}"`));
  });
  assert.deepEqual(orphans, [],
    `_mapToolCall produces these call.types but nothing handles them: ${orphans.join(", ")}`);
});

test("orchestration tools stay in the loop's dispatch set", () => {
  // These never reach the type dispatcher, so the type check above cannot protect them. If one
  // is dropped from the dispatch SET it becomes a tool the model can call that spawns nothing.
  //
  // Checked against the set itself, not the whole function. Mutation testing showed the
  // looser version was useless: `spawn_multiple_agents` appears 6 times inside
  // `_runAgenticLoop`, so deleting it from the dispatch set left the other 5 mentions and the
  // check stayed green while the tool was dead.
  const loop = fnBody("_runAgenticLoop");
  const m = /const subagentNames = new Set\(\[([^\]]*)\]\)/.exec(loop);
  assert.ok(m, "the loop's orchestration dispatch set (subagentNames) has moved or been renamed");
  const dispatched = new Set([...m[1].matchAll(/"([a-z0-9_]+)"/g)].map((x) => x[1]));
  for (const [type, name] of Object.entries(NAME_DISPATCHED)) {
    if (name === "run_worker") continue; // workers route through their own segment, not this set
    assert.ok(dispatched.has(name),
      `${name} (call.type "${type}") left the orchestration dispatch set — it now spawns nothing`);
  }
});

test("the tool catalog has not silently shrunk", () => {
  // Dropping a whole family during a refactor is easy to do and impossible to notice —
  // nothing fails, the model just quietly loses capability. A floor, not a target: raise it
  // deliberately, never lower it to make a red test green.
  //
  // Lowered ONCE, on purpose, from 140 → 128: eighteen single-site search tools were folded into
  // the two aggregators that already covered them (`developer_community_search` sources and
  // `package_search` ecosystems). The count is the weaker half of this test — the half that
  // actually protects capability is `retired search capability survives the fold` below, which
  // checks each retired name still reaches its replacement. Count guards accidents; that one
  // guards the thing the count was standing in for.
  assert.ok(schemaNames.length >= 128,
    `only ${schemaNames.length} tools registered — the catalog has lost capability`);
  for (const required of ["read_file", "write_file", "edit_file", "run_cmd", "search_tools", "update_plan"]) {
    assert.ok(schemaNames.includes(required), `nucleus tool ${required} is missing`);
  }
});

test("retired search capability survives the fold", () => {
  // Removing a tool from the catalogue is only safe while its capability is still reachable.
  // Each retired name must (a) be gone from the catalogue — otherwise the fold did nothing —
  // and (b) still resolve to the aggregator argument that reproduces it, so a mid-session
  // repeat of the old name executes instead of erroring.
  const EXPECTED = {
    devto_search: ["developer_community_search", "devto"],
    juejin_search: ["developer_community_search", "juejin"],
    v2ex_search: ["developer_community_search", "v2ex"],
    segmentfault_search: ["developer_community_search", "segmentfault"],
    gitlab_search: ["developer_community_search", "gitlab"],
    gitee_search: ["developer_community_search", "gitee"],
    codeberg_search: ["developer_community_search", "codeberg"],
    sourcegraph_search: ["developer_community_search", "sourcegraph"],
    infoq_search: ["developer_community_search", "infoq"],
    github_discussions_search: ["developer_community_search", "github_discussions"],
    github_trending: ["developer_community_search", "github_trending"],
    maven_search: ["package_search", "maven"],
    nuget_search: ["package_search", "nuget"],
    packagist_search: ["package_search", "packagist"],
    rubygems_search: ["package_search", "rubygems"],
    homebrew_search: ["package_search", "homebrew"],
    dockerhub_search: ["package_search", "dockerhub"],
    cdnjs_search: ["package_search", "cdnjs"],
  };
  const aliases = loadConst("_RETIRED_SEARCH_ALIASES");
  for (const [retired, [target, selector]] of Object.entries(EXPECTED)) {
    assert.ok(!schemaNames.includes(retired),
      `${retired} is still in the catalogue — the fold did not actually remove it`);
    const alias = aliases[retired];
    assert.ok(typeof alias === "function", `${retired} has no alias — an old call would fail hard`);
    const call = alias({ query: "q" });
    assert.equal(call.type, target, `${retired} must resolve to ${target}`);
    const got = target === "package_search" ? call.ecosystem : (call.sources || [])[0];
    assert.equal(got, selector, `${retired} must select the "${selector}" source/ecosystem`);
    assert.equal(call.query, "q", `${retired} must carry the query through`);
  }
  // Both aggregators must still be there — folding into a tool that is itself gone would
  // pass every assertion above and lose all 18 capabilities at once.
  for (const target of ["developer_community_search", "package_search"]) {
    assert.ok(schemaNames.includes(target), `${target} absorbed retired tools but is not registered`);
  }
});

test("every ecosystem the catalog advertises is one the backend actually dispatches", () => {
  // The fold moved seven registries from "their own tool" to "a value of package_search.ecosystem".
  // That turns a compile-time wiring into a STRING agreement across two languages: the enum the
  // model is shown lives in main.js, the match arms that serve it live in knowledge.rs. Drift is
  // silent in both directions — an enum value with no arm answers "Unknown ecosystem", an arm with
  // no enum value is a capability the model is never told about. So compare them directly.
  const rust = readFileSync(join(HERE, "../src-tauri/src/knowledge.rs"), "utf8");
  const declared = /pub const PACKAGE_ECOSYSTEMS: &\[&str\] = &\[([\s\S]*?)\];/.exec(rust);
  assert.ok(declared, "PACKAGE_ECOSYSTEMS has moved or been renamed in knowledge.rs");
  const canonical = new Set([...declared[1].matchAll(/"([a-z0-9_]+)"/g)].map((m) => m[1]));

  const body = /pub async fn package_search\([\s\S]*?\n\}/.exec(rust);
  assert.ok(body, "package_search has moved or been renamed in knowledge.rs");
  const served = new Set([...body[0].matchAll(/^\s*"([a-z0-9_" |]+)"?\s*=>/gm)]
    .flatMap((m) => [...m[1].matchAll(/([a-z0-9_]+)/g)].map((x) => x[1])));

  const line = /name:\s*"package_search"[\s\S]*?ecosystem:\s*\{[^}]*?enum:\s*\[([^\]]*)\]/.exec(SRC);
  const advertised = line ? [...line[1].matchAll(/"([a-z0-9_]+)"/g)].map((m) => m[1]) : null;
  assert.ok(Array.isArray(advertised) && advertised.length,
    "package_search must advertise its ecosystems as an enum — a free-text hint lets the model guess");

  for (const eco of advertised) {
    assert.ok(served.has(eco), `catalog offers ecosystem "${eco}" but package_search has no match arm for it`);
    assert.ok(canonical.has(eco), `ecosystem "${eco}" is advertised but missing from PACKAGE_ECOSYSTEMS`);
  }
  for (const eco of canonical) {
    assert.ok(advertised.includes(eco), `PACKAGE_ECOSYSTEMS lists "${eco}" but the model is never shown it`);
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. Gates must be reachable, and their verdicts consumed
// ─────────────────────────────────────────────────────────────────────────────

test("the permission gate is reachable from the single chokepoint", () => {
  // `_approveToolCall` once existed with zero callers. Everything under it died silently.
  assert.match(fnBody("_executeToolStep"), /await _approveToolCall\(/,
    "the permission gate must be invoked from _executeToolStep — the one path every tool takes");
  // …and that chokepoint must actually be what the loop calls.
  assert.match(fnBody("_runAgenticLoop"), /_executeToolStep\(/,
    "the loop must dispatch through the gated wrapper, not around it");
});

test("decision functions have their verdict consumed, not discarded", () => {
  // `await checkWorkspaceTrust(root);` as a bare statement: the dialog appeared, the user
  // could decline, and every MCP server in the repo launched anyway. A verdict evaluated for
  // its side effect alone is not a gate.
  for (const fn of ["checkWorkspaceTrust", "_approveToolCall", "_approveWorkspaceExecConfig"]) {
    const bare = new RegExp(`\\n\\s*await\\s+${fn}\\s*\\([^)]*\\)\\s*;`, "g");
    const hits = count(SRC, bare);
    assert.equal(hits, 0,
      `${fn}'s result is discarded at ${hits} site(s) — a decision nothing reads is not a gate`);
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. No NEW write-only run state
// ─────────────────────────────────────────────────────────────────────────────
//
// `run._diagnosticBlock` was assigned `""` three times and never read meaningfully, so the
// gate above it could never fire. The field, the gate and its tests all existed; only the
// connection was missing.
//
// This is baselined rather than enforced at zero. The 16 fields below are pre-existing — some
// are genuine debt, some are read through an alias (`execRun`, `_run`) that a text scan cannot
// follow. Fixing them all today would be a large unrelated change; letting the set GROW is
// how the next `_diagnosticBlock` gets in. So: the list may shrink freely, never grow.
// Names are stored WITHOUT the leading underscore: the capture group starts after `run._`.
const KNOWN_WRITE_ONLY = new Set([
  "michaelDesignPreflight", "michaelDesignBrief", "implementationGrounded", "compactedThisTurn",
  "emptyRootProbePending", "emptyRootProbe", "researchEvidence", "emptyBuildIntercepted",
  "toolRoutingState", "uiDeliveryAuditUnresolved", "capturePort",
]);

test("no NEW write-only run field is introduced", () => {
  const written = grab(SRC, /run\._([A-Za-z][A-Za-z0-9_]*)\s*=(?!=)/g);
  const fresh = [];
  for (const field of written) {
    // Reads include optional chaining (`run?._x`), which the first version missed and which
    // would have produced false positives on fields that ARE read.
    const all = count(SRC, new RegExp(`run\\??\\._${field}\\b`, "g"));
    const writes = count(SRC, new RegExp(`run\\._${field}\\s*=(?!=)`, "g"));
    if (all - writes === 0 && !KNOWN_WRITE_ONLY.has(field)) fresh.push(`run._${field}`);
  }
  assert.deepEqual(fresh, [],
    `new write-only run state — wire it up, or delete it: ${fresh.join(", ")}`);
});

test("the diagnostics gate field carries content, not only clears", () => {
  // The exact regression: three `= ""` assignments and no fourth, so the gate was vacuous.
  const clears = count(SRC, /run\._diagnosticBlock\s*=\s*""/g);
  const writes = count(SRC, /run\._diagnosticBlock\s*=(?!=)/g);
  assert.ok(writes > clears,
    "run._diagnosticBlock is only ever cleared — the diagnostics delivery gate cannot fire");
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. Bounded retries stay bounded
// ─────────────────────────────────────────────────────────────────────────────
//
// Each gate that sends the model back around must be latched. An unlatched `continue` in the
// quiet-turn branch is an infinite loop against a model that has decided to stop — and it
// bills for the whole time.

test("every quiet-turn re-entry is latched or counted", () => {
  const loop = fnBody("_runAgenticLoop");
  const start = loop.indexOf("if (!turn.toolCalls.length)");
  const end = loop.indexOf("// Render every tool step up front", start);
  const quiet = loop.slice(start, end);
  const continues = count(quiet, /\bcontinue;/g);
  assert.ok(continues > 0, "the quiet-turn branch should retain bounded recovery paths");
  // Every remaining re-entry is grounded in an OBSERVED fact or paid-for work, never in a
  // profile guess. The `_noWorkNudged` gate — the one place a classifier guess ("this task
  // should have changed something") still forced a turn — was removed in the loop rebuild
  // (AGENT_LOOP_REBUILD.md stage 1); its behaviour moved into agent_core.txt. If it comes
  // back as a `continue`, this list and the count below must stay honest.
  for (const [latch, why] of [
    [/run\._diagnosticNudges/, "new diagnostics, bounded"],
    [/buildFixAttempts/, "red build, bounded"],
    [/session\._steerQueue/, "user steering drains before the run ends"],
    [/run\._planFinishNudges/, "the plan still has open steps, bounded"],
  ]) {
    assert.match(quiet, latch, `a bounded re-entry lost its guard (${why})`);
  }
  assert.doesNotMatch(quiet, /_missingEffects\.includes\("workspace"\)[\s\S]{0,400}?continue;/,
    "a profile-derived 'you owe a change' guess must never force a re-entry again");
  // The IDE-dispatched sub-agent integration re-entry was removed with auto-dispatch itself
  // (AGENT_LOOP_REBUILD.md stage 3), dropping the quiet-turn re-entry ceiling from 4 to 3.
  assert.doesNotMatch(quiet, /_subAgentFinishIntercepted/,
    "the auto-dispatch integration leg must be gone — the IDE no longer spawns sub-agents");
  // Four now: the plan gate joined them. Every one of the other three is grounded in an observed
  // fact; so is this one — an open plan step is a fact the model itself recorded via update_plan,
  // not a classifier guess about what the task ought to have done.
  assert.ok(continues <= 4,
    `${continues} re-entry points in the quiet-turn branch — every one needs a latch listed above`);
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. The linker's own honesty check
// ─────────────────────────────────────────────────────────────────────────────
//
// A linker that passes proves nothing on its own — the first version of this file was fully
// green while `_approveToolCall` "had no call sites", because a broken comment-stripper had
// silently eaten the source it was searching. And its `spawn_multiple_agents` check stayed
// green through the very mutation it existed to catch, because the name appears six times in
// the loop and it searched all of them.
//
// So: re-introduce each historical defect into a COPY of the source and assert the relevant
// check goes red. This is what makes the file above trustworthy rather than decorative.
test("the linker catches the defects it was written for", () => {
  const mutate = (find, replace) => {
    const mutated = SRC.replace(find, replace);
    assert.notEqual(mutated, SRC, `mutation did not apply — the pattern is stale: ${find}`);
    return mutated;
  };
  // Each entry: a real bug from this codebase, and the check that must reject it.
  const cases = [
    {
      why: "S1 — the permission gate loses its only call site",
      src: mutate("if (!(await _approveToolCall(call, run))) {", "if (false) {"),
      check: (s) => /await _approveToolCall\(/.test(bodyOf(s, "_executeToolStep")),
    },
    {
      why: "B1 — the diagnostics gate becomes write-only again",
      src: mutate('run._diagnosticBlock = _noProgressVerify < 2 ? _d.report : "";', 'run._diagnosticBlock = "";'),
      check: (s) => (s.match(/run\._diagnosticBlock\s*=(?!=)/g) || []).length
        > (s.match(/run\._diagnosticBlock\s*=\s*""/g) || []).length,
    },
    {
      why: "S3 — a decision verdict is computed and discarded",
      src: mutate("const trusted = await checkWorkspaceTrust(root);",
        "await checkWorkspaceTrust(root);\n    const trusted = true;"),
      check: (s) => !/\n\s*await\s+checkWorkspaceTrust\s*\([^)]*\)\s*;/.test(s),
    },
    {
      why: "a registered tool loses its mapper case",
      src: mutate('case "run_cmd": {', 'case "run_cmd_DISABLED": {'),
      check: (s) => new Set([...bodyOf(s, "_mapToolCall").matchAll(/case\s+"([a-z0-9_]+)"/g)]
        .map((m) => m[1])).has("run_cmd"),
    },
    {
      why: "an orchestration tool leaves the loop's dispatch set",
      src: mutate('"spawn_multiple_agents", "debate"]', '"debate"]'),
      check: (s) => {
        const m = /const subagentNames = new Set\(\[([^\]]*)\]\)/.exec(bodyOf(s, "_runAgenticLoop"));
        return !!m && m[1].includes('"spawn_multiple_agents"');
      },
    },
  ];
  for (const { why, src, check } of cases) {
    assert.equal(check(SRC), true, `the check is wrong: it already rejects the CURRENT source (${why})`);
    assert.equal(check(src), false, `the linker would NOT catch: ${why}`);
  }
});

/** fnBody, but against arbitrary source — needed to run checks over mutated copies. */
function bodyOf(source, name) {
  const m = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).exec(source);
  if (!m) return "";
  let p = source.indexOf("(", m.index), pd = 0;
  for (; p < source.length; p++) {
    const c = source[p];
    if (c === "(") pd++; else if (c === ")") { pd--; if (!pd) break; }
  }
  let i = source.indexOf("{", p), d = 0;
  for (; i < source.length; i++) {
    const c = source[i];
    if (c === "{") d++; else if (c === "}") { d--; if (!d) return source.slice(m.index, i + 1); }
  }
  return "";
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. No NEW unreachable modules
// ─────────────────────────────────────────────────────────────────────────────
//
// The mirror image of a severed connection: a whole FILE nothing imports. It compiles, its
// tests pass, and it is never loaded — so the tests are measuring code the product does not
// run. `src/agent/job-queue.js` is 446 lines with a 218-line green test suite that main.js
// has never imported; the live `spawn_multiple_agents` is implemented inline instead. Two
// implementations, and the suite exercises the dead one.
//
// Baselined, like the write-only fields: the existing four may be deleted or wired up, but
// the set must never grow.
test("no NEW unreachable module appears under src/", async () => {
  const { readdirSync, statSync, existsSync } = await import("node:fs");
  const path = await import("node:path");
  const SRC_DIR = join(HERE, "../src");

  const all = [];
  (function walk(dir) {
    for (const e of readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) { if (!/node_modules|dist/.test(p)) walk(p); }
      else if (/\.(js|jsx|mjs)$/.test(e.name)) all.push(p);
    }
  })(SRC_DIR);

  const resolve = (from, spec) => {
    if (!spec.startsWith(".")) return null;
    // Vite suffixes (`./x.js?worker`, `?url`, `?raw`) are real imports — stripping the query
    // is what stops this check from calling live web workers dead, which the first scan did.
    const clean = spec.split("?")[0];
    const base = path.normalize(path.join(path.dirname(from), clean));
    for (const c of [base, `${base}.js`, `${base}.jsx`, `${base}.mjs`,
      path.join(base, "index.js"), path.join(base, "index.jsx")]) {
      if (existsSync(c) && statSync(c).isFile()) return c;
    }
    return null;
  };
  const seen = new Set();
  const visit = (f) => {
    if (seen.has(f) || !existsSync(f)) return;
    seen.add(f);
    const s = readFileSync(f, "utf8");
    for (const re of [/(?:import|from)\s+["']([^"']+)["']/g, /import\(\s*["']([^"']+)["']/g,
      /new Worker\(\s*new URL\(\s*["']([^"']+)["']/g]) {
      for (const m of s.matchAll(re)) { const r = resolve(f, m[1]); if (r) visit(r); }
    }
  };
  for (const entry of ["boot.jsx", "main.js"]) {
    const p = join(SRC_DIR, entry);
    if (existsSync(p)) visit(p);
  }

  // `_serve.mjs` is a standalone dev script, launched by hand rather than imported.
  const INTENTIONALLY_STANDALONE = new Set(["src/_serve.mjs"]);
  const KNOWN_DEAD = new Set([
    "src/agent/job-queue.js",        // 446 lines; live spawn_multiple_agents is inline in main.js
    "src/tools/spawn-multiple-agents.js", // 193 lines; imported by nothing at all
    "src/search-enhanced.js",        // 377 lines; none of its exports appear in main.js
    "src/terminal.js",               // 335 lines; main.js carries its own terminal implementation
  ]);

  const rel = (f) => path.relative(join(HERE, ".."), f).split(path.sep).join("/");
  const fresh = all.map(rel).filter((f) =>
    !seen.has(join(HERE, "..", f)) && !KNOWN_DEAD.has(f) && !INTENTIONALLY_STANDALONE.has(f));
  assert.deepEqual(fresh, [],
    `these modules are unreachable from the entry points — wire them up or delete them: ${fresh.join(", ")}`);

  // And the baseline must stay honest: if one gets wired up or deleted, shrink the list.
  for (const f of KNOWN_DEAD) {
    assert.ok(all.map(rel).includes(f) && !seen.has(join(HERE, "..", f)),
      `${f} is no longer dead — remove it from KNOWN_DEAD so the baseline keeps shrinking`);
  }
});

test("create_project 五个环节都接上了——少一环这个工具就是死的", () => {
  // 这是智能体唯一真正缺的能力：130 个工具里没有一个能新建或绑定工作区根，而 create_dir 走
  // require_inside_workspace（必须先有工作区才能建目录）。于是用户没打开文件夹时说「写个
  // telegram 机器人」，智能体只能把问题退回给用户——它不是不肯干，是真的做不到。
  //
  // 这条链五个环节，断任何一环工具就是死的，而且死得很安静：模型照样能看到它、照样会调，
  // 只是什么都不会发生。这个仓库已经有过太多这样的东西了。
  assert.match(SRC, /createProjectDir: \(name\) => core\.invoke\("create_project_dir", \{ name \}\)/,
    "① backend 包装");
  assert.match(SRC, /createProjectDir: async \(name\)/, "② 浏览器兜底（开发预览里一调就抛）");
  assert.match(SRC, /name: "create_project", description:/, "③ 工具定义，模型才看得见");
  assert.match(SRC, /case "create_project": return \{ type: "createproject"/, "④ 参数映射");
  assert.match(SRC, /call\.type === "createproject"/, "⑤ 执行分支");
  // 建完必须绑成活动工作区，否则后面每个文件工具照样 [BLOCKED]。
  assert.match(SRC, /setActiveWorkspaceRoot\(_abs\)/, "⑥ 绑定活动工作区");
  // 路径必须报给用户，否则他不知道东西建到哪了。
  assert.match(SRC, /在最终回复里把这个完整路径告诉用户/);

  // 后端命令要真的注册进 Tauri，否则前端 invoke 直接抛。
  const lib = readFileSync(join(HERE, "../src-tauri/src/lib.rs"), "utf8");
  assert.match(lib, /files::create_project_dir/, "⑦ Tauri 命令注册");

  // 网关那份目录才是运行时生效的（见 MEMORY：两套工具目录）。只改客户端等于部署后模型看不见。
  const gw = JSON.parse(readFileSync(join(HERE, "../../server/prompts/tools.json"), "utf8"));
  assert.ok(gw.some((t) => t.function.name === "create_project"),
    "⑧ 网关目录镜像——少了它，部署之后模型根本看不到这个工具");
});

test("施工请求不许被降级成反问：六处判题链和授权底线都要保持现状", () => {
  const P = (n) => readFileSync(join(HERE, "../../server/prompts/" + n), "utf8");

  // 「用户说做个 X 就直接做」这句话以前在整套提示里一次都没出现过，模型每轮只读到
  // 「先判题 → 先调研 → 先问方向」。这六处是同一条链的六个副本，重复本身才是它起作用的原因，
  // 所以任何一处退回去都要变红。
  assert.doesNotMatch(P("agent_core.txt"), /physical actions, missing credentials/,
    "缺密钥不是停下来找人的理由——Telegram bot 第一句话就缺 token");
  assert.match(P("agent_core.txt"), /A missing key or token is never a reason to stop/);

  assert.doesNotMatch(P("reasoning.txt"), /check whether a usable open-source implementation already exists/,
    '「build me an X」是施工单，不是调研题');
  assert.match(P("reasoning.txt"), /pick the mainstream stack from what you already know and start building/);

  assert.doesNotMatch(SRC, /Without the user explicitly asking for it, do not change the workspace/,
    "每轮契约要界定范围，而不是把装依赖/起服务整体列为未授权");
  assert.doesNotMatch(SRC, /Agent 模式不是无条件全自动/, "「问一句」不该是并列出口");
  assert.match(SRC, /用户说「写一个\/做一个\/搭一个\/接上 X」就是施工单/);
  assert.doesNotMatch(SRC, /只有要引入新包（npm install react\/pnpm add xxx）才需要用户明确要实现\/安装/,
    "功能需要的依赖属于实现本身");

  // ask_user 的范例决定了它的实际用法：范例的指令密度远高于告诫句。
  // 两份目录必须同步——运行时生效的是网关那份（见 MEMORY: 两套工具目录）。
  const gw = JSON.parse(readFileSync(join(HERE, "../../server/prompts/tools.json"), "utf8"));
  const askGw = gw.find((t) => t.function.name === "ask_user").function.description;
  for (const [label, text] of [["网关", askGw], ["客户端", SRC]]) {
    assert.doesNotMatch(text, /user says "build a login page" → ask/,
      `${label}：ask_user 的范例不能教「收到施工请求就问框架」`);
  }
  assert.match(askGw, /A build request whose stack is unspecified is NOT ambiguous/);

  // 授权底线一个字都不许少。放开自由度的同时把这五条一起"顺手简化"掉，才是真正会出事的改动。
  const truth = P("truthfulness.txt");
  for (const line of ["breaking into third parties", "stealing accounts",
                      "bypassing payment/risk controls", "exfiltrating data", "persistent control"]) {
    assert.ok(truth.includes(line), `授权底线缺失：${line}`);
  }
  // 补上的是禁令的对面，不是替代它。
  assert.match(truth, /owns or is\s+authorized to use is ordinary engineering — build it, no preamble/,
    "授权范围内的自动化/集成/写 bot 是普通工程，直接做");
  assert.match(truth, /about\s+unauthorized third parties, not third-party APIs/,
    "必须点明下面那条禁令针对的是未授权第三方，不是第三方 API——否则模型会把接 API 也当成越线");
});

test("the request-boundary markers the client emits are exactly the ones the gateway parses", () => {
  // The 📌 boundary is a wire protocol, not prompt text: main.js wraps the user's real request in
  // it, and prompts.rs slices the request back out of the surrounding project context. They are
  // two files in two languages agreeing on a literal string, so drift is silent — the gateway
  // stops finding the boundary, extract_marked_user_request fails closed, and the user's actual
  // ask silently stops being isolated from the context wrapped around it.
  //
  // The set is deliberately larger than one entry: a desktop build older than the English prompt
  // rewrite still emits the Chinese marker, and stored conversations still contain it, so both
  // spellings must stay parseable on the gateway side.
  const clientList = /const _REQUEST_MARKERS = \[([^\]]*)\];/.exec(SRC);
  assert.ok(clientList, "_REQUEST_MARKERS has moved or been renamed in main.js");
  const clientMarkers = [...clientList[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  assert.ok(clientMarkers.length >= 2,
    "the client must still recognize the pre-rewrite marker, or old conversations lose their boundary");

  const rust = readFileSync(join(HERE, "../../server/src/prompts.rs"), "utf8");
  const gatewayMarkers = [...rust.matchAll(
    /const (?:LEGACY_(?:CN_)?)?USER_REQUEST_MARKER: &str = "([^"]+)";/g)].map((m) => m[1]);
  assert.ok(gatewayMarkers.length >= 2, "the gateway's USER_REQUEST_MARKER constants have moved");

  // Rust escapes are not JS escapes, so compare on the marker's stable, escape-free core.
  const core = (s) => s.replace(/\\n/g, "").replace(/[━\s]+$/g, "").trim();
  for (const marker of clientMarkers) {
    assert.ok(gatewayMarkers.some((g) => core(g) === core(marker)),
      `main.js emits/recognizes ${JSON.stringify(marker)} but prompts.rs parses no such marker`);
  }

  // And the one it actually emits must be the gateway's current (non-legacy) marker, so a new
  // conversation is never routed down the compatibility path.
  const emitted = /📌 \*\*This turn's user request\*\*/.test(SRC);
  assert.ok(emitted, "main.js must emit the English request marker");
  const current = /\nconst USER_REQUEST_MARKER: &str = "([^"]+)";/.exec(rust);
  assert.ok(current && core(current[1]) === core(clientMarkers[0]),
    "the first entry of _REQUEST_MARKERS must be the gateway's current USER_REQUEST_MARKER");
});

test("every semantic flag the client declares is one the gateway accepts and routes", () => {
  // Same two-files-in-two-languages problem as the request marker, but it fails silently in a
  // worse way: the gateway filters the profile through an allow-list, so a flag main.js emits
  // that prompts.rs does not list is not an error anywhere — it is simply dropped, and the
  // prompt block it was supposed to route never loads. Nothing goes red; the model just stops
  // being told something. (Measured: the `defects` flag did exactly this until it was allowed.)
  const emit = /function _ideSemanticProfile\(profile\) \{[\s\S]*?\n\}/.exec(SRC);
  assert.ok(emit, "_ideSemanticProfile has moved or been renamed in main.js");
  const clientFlags = [...emit[0].matchAll(/\badd\("([a-z0-9_]+)"/g)].map((m) => m[1]);
  assert.ok(clientFlags.length >= 15, "the semantic profile's flag list has collapsed");

  const rust = readFileSync(join(HERE, "../../server/src/prompts.rs"), "utf8");
  const allowList = /const IDE_SEMANTIC_PROFILE_FLAGS: &\[&str\] = &\[([\s\S]*?)\];/.exec(rust);
  assert.ok(allowList, "IDE_SEMANTIC_PROFILE_FLAGS has moved or been renamed in prompts.rs");
  const accepted = new Set([...allowList[1].matchAll(/"([a-z0-9_]+)"/g)].map((m) => m[1]));

  for (const flag of clientFlags) {
    assert.ok(accepted.has(flag),
      `main.js emits the semantic flag "${flag}" but prompts.rs drops it — its prompt block never loads`);
  }

  // A flag that is accepted but consumed by nobody is the same defect seen from the other end:
  // the client spends a declaration on it every turn and the gateway does nothing with it. These
  // seven are the ones that were already inert when this test was written — protocol surface with
  // no block behind it yet. The baseline is here so no NEW one joins them unnoticed; shrinking it
  // is always allowed.
  const KNOWN_INERT = new Set([
    "official", "community", "collaboration_staged", "collaboration_parallel",
    "existing_project", "existing_website", "network_capture",
  ]);
  const graph = JSON.parse(readFileSync(join(HERE, "../../server/prompts/prompt_graph.json"), "utf8"));
  const routed = new Set([
    ...Object.keys(graph.agent), ...Object.keys(graph.design), ...Object.keys(graph.modes),
  ]);
  const consumed = (flag) => routed.has(flag) || new RegExp(`semantic\\("${flag}"\\)`).test(rust);
  for (const flag of clientFlags) {
    if (KNOWN_INERT.has(flag)) continue;
    assert.ok(consumed(flag),
      `the semantic flag "${flag}" is declared and accepted but selects no prompt module`);
  }
  for (const flag of KNOWN_INERT) {
    assert.ok(clientFlags.includes(flag) && !consumed(flag),
      `"${flag}" is no longer inert — take it out of KNOWN_INERT instead of leaving the baseline stale`);
  }
});

test("思考深度必须真的到达模型：三条链路一条都不能断", () => {
  const main = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
  const ai = readFileSync(new URL("../src-tauri/src/ai.rs", import.meta.url), "utf8");

  // 1) gpt-5.6 走的是 protocol="openai" 的透传线路，网关不改 reasoning_effort，
  //    所以 xhigh 是真的能到模型的。默认停在 high 等于让不动转盘的人永远浅一档——
  //    线上三天 44 次请求里 xhigh 零次，用户拿 opencode 跑同一个模型用的正是 xhigh。
  // 窗口要够宽：这一段的注释里记着实测数据，窄了会把 defaultLevel 切在外面。
  const gpt56 = main.slice(main.indexOf("gpt[-_.]?5\\.6"), main.indexOf("gpt[-_.]?5\\.6") + 2000);
  assert.match(gpt56, /levels:\s*\[[^\]]*"xhigh"[^\]]*\]/, "gpt-5.6 的档位里必须有 xhigh");
  assert.match(gpt56, /defaultLevel:\s*"xhigh"/, "gpt-5.6 的默认档位必须是 xhigh，不是 high");

  // 2) 「推理 N tokens」是用户能拿到的唯一硬证据——没有它，档位拨了也看不出生效。
  //    消费方一直都在（_recordUsage 读 reasoning_tokens），断的是两条传输层。
  assert.match(main, /reasoning_tokens:\s*reasoning/, "网页传输层要把 reasoning_tokens 发上去");
  assert.match(ai, /reasoning_tokens:\s*u32/, "AiEvent::Usage 要带 reasoning_tokens");
  assert.match(ai, /completion_tokens_details"\]\["reasoning_tokens"/, "ai.rs 要解析 OpenAI 形状");
  assert.match(ai, /output_tokens_details"\]\["reasoning_tokens"/, "ai.rs 要解析 Anthropic 形状");
  assert.match(ai, /reasoning_tokens:\s*reasoning as u32/, "解析出来的值要真的发出去");

  // 3) @model: 换族时先清后合。思考字段是按模型族成形的，Object.assign 不删多余键，
  //    从 Claude 切到 GPT 会把 thinking:{type:"adaptive"} 一起带过去。
  const routed = main.slice(main.indexOf("const routed = await _readyAiConfig"), main.indexOf("const routed = await _readyAiConfig") + 700);
  assert.match(routed, /delete config\[k\]/, "@model: 切模型前要先清掉上一族的思考字段");
  for (const k of ["reasoningEffort", "thinking", "thinkingConfig", "thinkingBudget"]) {
    assert.ok(routed.includes(`"${k}"`), `清理列表里少了 ${k}`);
  }
  assert.ok(routed.indexOf("delete config[k]") < routed.indexOf("Object.assign(config, routed)"),
    "必须先清再合，顺序反了等于没清");
});

test("两条线路的思考档位一真一假，代码里必须分开处理", () => {
  const main = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
  const rs = readFileSync(new URL("../../server/src/models.rs", import.meta.url), "utf8");

  // 2026-08-13 对真实上游实测：
  //   GPT 线路（protocol=openai，网关透传）—— 乱填的 effort 被拒
  //     （level "zzzz" not supported），low/medium 的输出量只有 high 的一半：档位是真的。
  //   Claude 线路（protocol=anthropic，网关翻译）—— banana / ULTRA / 12345 全部 200
  //     正常返回，high 之上没有任何梯度：档位是假的。
  // 同一个 "xhigh" 在两条线路上一真一假，所以只能分开处理——这条守卫钉的就是这个区分。

  // GPT：既然是真档位，就要有按钮、而且默认就该是它。
  const gpt56 = main.slice(main.indexOf("gpt[-_.]?5\\.6"), main.indexOf("gpt[-_.]?5\\.6") + 1600);
  assert.match(gpt56, /levels:\s*\[[^\]]*"xhigh"[^\]]*\]/);
  assert.match(gpt56, /defaultLevel:\s*"xhigh"/);

  // Claude：既然是假档位，就不能摆按钮——摆了就是用户抱怨的那个"和假的一样"。
  const claudeLevels = main.match(/levels: _alwaysThinks \? \[[^\]]*\] : \[[^\]]*\]/)[0];
  assert.doesNotMatch(claudeLevels, /xhigh/, "Claude 线路上 xhigh 不是真档位，不该有按钮");
  assert.match(rs, /\("xhigh", false\) \| \("max", false\) => "high",/, "网关默认仍要封顶");

  // 实测结论必须留在代码里，否则下一个人会照着文档把按钮加回去——这正是上一轮
  // 「转卖渠道会返回空 completion」那条没人验证过的推断留下的教训。
  assert.match(rs, /banana/, "网关注释里要留着控制组的实测数据");
  assert.match(gpt56, /valid levels/, "客户端注释里要留着上游报错的原文");
});

test("思考量在两条线路上都有东西可显示", () => {
  const main = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
  const ai = readFileSync(new URL("../src-tauri/src/ai.rs", import.meta.url), "utf8");
  const rs = readFileSync(new URL("../../server/src/models.rs", import.meta.url), "utf8");

  // Anthropic 不单独报思考 token（算进 output_tokens），GPT 那条上游实测也不报
  // completion_tokens_details.reasoning_tokens。两条路都没数 → 那半句永远不显示。
  // 网关本来就在逐帧数思考字符（原本只进遥测日志），把它一并报上来。
  assert.match(rs, /"thinking_chars": self\.thinking_telemetry\.thinking_utf8_chars/,
    "网关要把已经数出来的思考字符报给客户端");
  assert.match(ai, /thinking_chars: u32/, "桌面传输层要带这个字段");
  assert.match(ai, /usage\["thinking_chars"\]/, "桌面侧要解析它");
  assert.match(main, /thinking_chars: Number\(u\.thinking_chars\)/, "网页传输层也要带");
  assert.match(main, /_lastThinkChars \? ` · 思考 \$\{k\(_lastThinkChars\)\} 字`/,
    "没有 token 数时要退回字符数，不能只剩一个光秃秃的档位名");
});

test("/sessions 必须能看到内存装不下的那部分历史会话", () => {
  const main = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
  const rs = readFileSync(new URL("../src-tauri/src/conversation_store.rs", import.meta.url), "utf8");
  const lib = readFileSync(new URL("../src-tauri/src/lib.rs", import.meta.url), "utf8");

  // 起因：热副本 _closedChatSessions 被有意封在 80 条（每条常驻内存、带 recent/transcript），
  // Rust 侧回填也封在 200。但 conversation_sessions 表**永不删除**——用户开过的每个会话都在。
  // 以前 /sessions 只读热副本，于是关掉的会话过一阵就从历史里消失了。数据一直在，是清单没去看。

  // 1) 后端要有「全量轻量清单」和「按 id 取一条」两个命令，且都注册了。
  assert.match(rs, /pub async fn conversation_sessions_index/);
  assert.match(rs, /pub async fn conversation_session_load/);
  assert.match(rs, /SELECT session_id, session_json, is_closed, updated_at FROM conversation_sessions ORDER BY updated_at DESC/,
    "清单要按最近更新排序取全表");
  assert.doesNotMatch(
    rs.slice(rs.indexOf("pub async fn conversation_sessions_index")).slice(0, 1200),
    /LIMIT/,
    "清单查询不能带 LIMIT —— 那就又把历史砍掉了",
  );
  for (const cmd of ["conversation_sessions_index", "conversation_session_load"]) {
    assert.ok(lib.includes(`conversation_store::${cmd}`), `${cmd} 没在 lib.rs 注册`);
  }

  // 2) 前端要有绑定、要真的去查、点进去要能恢复。
  assert.match(main, /conversationSessionsIndex: \(\) => core\.invoke\("conversation_sessions_index"\)/);
  assert.match(main, /conversationSessionLoad: \(sessionId\) => core\.invoke\("conversation_session_load"/);
  assert.match(main, /async function _archivedSessionRows\(\)/);
  assert.match(main, /async function _restoreArchivedSession\(sessionId\)/);

  // 3) picker 必须把归档行拼进清单，并且点击时走归档恢复路径。
  const picker = main.slice(main.indexOf("async function _openSessionPicker"), main.indexOf("async function _openSessionPicker") + 3000);
  assert.match(picker, /await _archivedSessionRows\(\)/, "picker 要去取归档清单");
  assert.match(picker, /entries: \[\.\.\.rows, \.\.\.archivedRows\]/, "归档行要真的进 entries");
  assert.match(picker, /row\.state === "archived"/, "点归档行要走归档恢复");

  // 4) 索引查不到时必须退化成旧行为，不能让整个 /sessions 打不开。
  const idx = main.slice(main.indexOf("async function _archivedSessionRows"), main.indexOf("async function _restoreArchivedSession"));
  assert.match(idx, /return \[\]/, "取不到索引要返回空数组而不是抛");
  assert.match(idx, /catch/, "索引查询要被 try/catch 包住");
});

test("「要方案」和「要施工」两条规则必须成对存在，只留一半就是这次的 bug", () => {
  const main = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");
  const core = readFileSync(new URL("../../server/prompts/agent_core.txt", import.meta.url), "utf8");

  // 起因：为了治「让他做 X 他却反问」，提示词被推向「直接开工」。但只写了
  // 「不要把施工降级成方案」，没写反向那句，于是用户要方案时它给完方案顺手就开工了。
  // 两条是对称的，任何一次只改一半都会把天平推到另一边。

  // 施工方向（不能被这次修复削弱）
  assert.match(main, /就是施工单，直接开工，不要降级成方案/);

  // 方案方向（这次补的）
  assert.match(main, /不要把方案自动升级成施工/, "判题那处要有反向规则");
  assert.match(main, /\*\*用户要方案\*\*/, "协作边界要单独列出「要方案」这一类");
  assert.match(main, /给完就停|停下来等他发话/, "要方案必须明确说「停」");

  // 意图分类要把方案请求判成只读，否则计划门和评分都会按施工算
  assert.match(main, /action=plan、workspaceAction=inspect/, "方案请求要分类成只读");

  // 服务端那条也要在（客户端提示不一定每条路径都注入）
  assert.match(core, /A plan request ends at the plan/, "服务端 agent_core 要有同一条规则");
  assert.match(core, /never downgrade a build request into a plan|do\/fix\/change\/build\/run\/deploy must produce/,
    "施工那半边不能被删掉");
});

test("打包进 app 的 automation sidecar 必须带鉴权，且这道闸在构建期", () => {
  const build = readFileSync(new URL("../src-tauri/build.rs", import.meta.url), "utf8");

  // 事故形状（2026-08-13 实测）：rpc.rs 的两道闸 08-02 就加上了，但 binaries/ 里放的是
  // 手工构建、部分未跟踪的二进制。aarch64 是加固后编的（有鉴权），x86_64 和 universal
  // 虽然文件日期更晚却是加固**之前**的产物——零鉴权。Tauri 按目标三元组挑文件，于是
  // Apple Silicon 装到安全的那份、Intel Mac 和 Windows 装到零鉴权那份，全程无报错。
  // 换掉二进制不够：手工产物迟早再次和源码脱节，所以闸必须在构建期。
  assert.match(build, /fn assert_sidecar_is_authenticated/, "构建期要有 sidecar 鉴权闸");
  assert.match(build, /assert_sidecar_is_authenticated\(\);\s*\n\s*tauri_build::build\(\)/,
    "闸必须在 tauri_build::build() 之前跑");
  assert.match(build, /MICHAEL_AUTOMATION_TOKEN/);
  assert.match(build, /unauthorized/);
  assert.match(build, /panic!/, "缺鉴权要让构建失败，不能只打个警告");

  // 判据只能用会进 .rodata 的字符串。短比较字面量（strip_prefix("x-automation-token:")）
  // 在 release 下被内联成立即数，从**已加固**的源码新编出来也搜不到——拿它当判据只会
  // 得到假警报。今天在这上面栽过三次。
  assert.doesNotMatch(build, /has\("x-automation-token/, "别用会被内联的短字面量当判据");
});

test("官网的工具画廊必须还能从 main.js 里取出真目录", () => {
  // 事故形状（2026-08-16 实测）：`_buildAgentToolSchemas` 里先后多了 `_userCapabilities()`
  // 和 `_withoutDisabledTools()` 两个调用（提交 46398dc），而 website/scripts/extract-tools.mjs
  // 是把这个函数的**源文本**抠出来 `new Function` 求值的——注入的自由变量没跟着加。
  //
  // 后果不是"某个测试红了"。是 website/ 里的 `npm run build` 死在第一步 prebuild 上，
  // 官网整个发不出去；而已经生成好的 public/tools.json 就停在旧的 132 个工具，画廊少列
  // 四个真实存在的能力（git_show / probe_env / ui_extract / view_image）。两条链路都没有
  // 任何东西会喊——本地测试全绿，服务器也全绿，只有真去构建官网的那一刻才炸。
  //
  // 这里跑**真脚本**而不是在测试里重写一份提取逻辑：重写的那份自己就会漂，正好抓不到
  // 这一类缺陷。断言的是连接——"提取器仍然够得着 main.js 里的那个函数"。
  const WEB = join(HERE, "../website");
  const OUT = join(WEB, "public/tools.json");
  const before = readFileSync(OUT, "utf8");
  try {
    execFileSync(process.execPath, ["scripts/extract-tools.mjs"], { cwd: WEB, stdio: "pipe" });
  } catch (e) {
    assert.fail(
      "extract-tools.mjs 跑不起来了，官网 prebuild 会死在这里：\n" +
        String(e.stderr || e.message).trim() +
        "\n——_buildAgentToolSchemas 里多了一个提取器没注入的自由变量，去 extract-tools.mjs 的 new Function 参数里补上",
    );
  }
  const after = readFileSync(OUT, "utf8");
  // 生成是确定性的（generatedAt 刻意留空就是为了不churn），所以同步时这一步是空操作。
  // 不同步就把文件还原，测试只报告、不改仓库。
  if (after !== before) writeFileSync(OUT, before);
  assert.equal(after, before,
    "public/tools.json 和 main.js 的真目录不同步了——跑 website/ 的 `node scripts/extract-tools.mjs` 并提交结果");
});

test("强力版开关必须真的改变请求去向，而不只是一个会亮的图标", () => {
  // 这个开关的全部意义是把这一轮派到后台勾了「Claude 强力版」的线路上。
  // 只要下面任何一环断了，UI 照样亮、请求照走普通线路——正是这个文件专门盯的
  // 那种"存在但没人够得着"的缺陷。
  // 1) 按钮渲染进卡片头部（不是定义了一个没人调的函数）
  // 断言的是**调用点在卡片模板里**，不是"源码里出现过这个名字"——函数定义本身
  // 就含有这个名字，照着名字找等于自己喂饱自己。
  const tpl = SRC.slice(SRC.indexOf("card.innerHTML ="));
  assert.match(tpl.slice(0, 700), /_modelPowerToggleHtml\(/,
    "卡片模板没调强力版按钮，那个函数成了死代码");

  // 2) 点击真的落盘，而不是只切了个 class
  const clickBlock = SRC.slice(SRC.indexOf('.closest?.(".mic-power")'));
  assert.match(clickBlock.slice(0, 600), /_setPowerRoute\(/,
    "点了按钮没写进持久化状态，刷新/重开卡片就丢");

  // 3) 意图盖到了**所有**去后端的入口。轮次组装点有好几处，漏一处就会出现
  //    "开关亮着但请求走普通线路"。
  for (const cmd of ["ai_chat", "ai_chat_with_tools", "ai_complete"]) {
    const at = SRC.indexOf(`core.invoke("${cmd}",`);
    assert.notEqual(at, -1, `${cmd} 入口不见了`);
    assert.match(SRC.slice(at, at + 200), /_stampPowerRoute\(config\)/,
      `${cmd} 没盖强力版标记，从这个入口发的轮次会静默走普通线路`);
  }

  // 4) 盖上的标记最终变成请求头——网页端在 JS 里发，桌面端在 Rust 里发
  assert.match(SRC, /x-ide-power-route/,
    "网页端没把标记转成请求头，网关看不到");
  const rust = readFileSync(join(HERE, "../src-tauri/src/ai.rs"), "utf8");
  assert.match(rust, /ide_power_route/,
    "AiConfig 没有这个字段，serde 会把 JS 传的值直接丢掉（michaelCompression 就这么丢过一次）");
  assert.match(rust, /header\("x-ide-power-route"/,
    "桌面端没发这个头，桌面版的开关等于没接");

  // 5) 只有 Claude 一族有。用户明确要求过。
  // 这里**跑**这个判定而不是读它的源码：上一版按 /claude/i 去匹配源文本，结果被
  // 函数上面那段写着 Claude 的注释喂饱了——把限定改成 `return true` 都照样绿。
  const gate = SRC.slice(SRC.indexOf("function _modelSupportsPowerRoute"));
  const supports = new Function(`${gate.slice(0, gate.indexOf("\n}") + 2)}
    return _modelSupportsPowerRoute;`)();
  for (const yes of ["claude-opus-4-6", "claude-sonnet-4-5", "anthropic/claude-haiku-4-5"]) {
    assert.equal(supports(yes), true, `${yes} 是 Claude，应该有强力版按钮`);
  }
  for (const no of ["gpt-5.2", "gemini-3-pro", "glm-5.3", "deepseek-v4", "kimi-k3", ""]) {
    assert.equal(supports(no), false,
      `${no} 不是 Claude，却冒出了强力版按钮——用户明确要求过只有 Claude 有`);
  }
  const render = SRC.slice(SRC.indexOf("function _modelPowerToggleHtml"));
  assert.match(render.slice(0, 900), /if \(!_modelSupportsPowerRoute\(id\)\) return ""/,
    "渲染时没挡住非 Claude 模型");
});

test("强力版：网关说了没有强力线路，就不该把那个按钮画出来", () => {
  // 承接上一条。上一条钉的是"开关真的改变请求去向"，这一条钉的是"按钮只出现在它
  // 真的有用的地方"——用户在后台没配强力线路时，画一个点了必然报错的按钮比不画更糟。
  //
  // 三态是关键：网关明确说 false 才藏；拿不到（离线、目录没拉到、网关旧版）是"不知道"，
  // 这时候必须退回旧行为。压成布尔的话按钮会在离线时集体消失，而那不是"没有强力线路"。
  const mapAt = SRC.indexOf("(byGroup[label] ||= []).push({");
  assert.notEqual(mapAt, -1, "目录映射那个对象字面量没了");
  assert.match(SRC.slice(mapAt, mapAt + 2600), /powerRouteAvailable:/,
    "映射层没接这个字段——那个 push 是逐字段列举的白名单，不在里面就到不了按钮那儿");

  // 字段名必须和网关下发的逐字一致，跨仓库对不上就是静默失效。
  const rustList = readFileSync(join(HERE, "../../server/src/models.rs"), "utf8");
  assert.match(rustList, /"power_route_available"/,
    "网关没下发这个字段，客户端永远读到 undefined，等于这条闸不存在");
  assert.match(SRC.slice(mapAt, mapAt + 2600), /it\.power_route_available/,
    "客户端读的键名和网关下发的对不上");

  // 三态判定：只认布尔，其它一律 null（不知道）。
  const availSrc = SRC.slice(SRC.indexOf("function _powerRouteAvailable"));
  const avail = availSrc.slice(0, availSrc.indexOf("\n}") + 2);
  assert.match(avail, /typeof v === "boolean" \? v : null/,
    "没做成三态——离线/网关旧版会被当成'没有强力线路'，按钮集体消失");

  // 同一条闸必须同时管住**渲染**和**发送**。只藏按钮不拦请求头，用户会陷在一个
  // 每轮都报错、又找不到地方关掉的状态里。
  const send = SRC.slice(SRC.indexOf("function _powerRouteOn"));
  assert.match(send.slice(0, 900), /_powerRouteAvailable\(id\) === false/,
    "发送侧没拦——按钮藏了但请求头照发，用户没有关掉它的入口");
  const render = SRC.slice(SRC.indexOf("function _modelPowerToggleHtml"));
  assert.match(render.slice(0, 1200), /_powerRouteAvailable\(id\) === false/,
    "渲染侧没拦——按钮画在了没有强力线路的模型上，点了只会报错");
});

test("档位滑块：拖动要真的落到档位上，且拖出卡片边界不能把卡片收走", () => {
  // 滑块换掉分段按钮之后，有三条连接一断就会变成"能拖、但什么都没发生"或者
  // "拖到一半断掉"——三种都不报错，只是控件坏了。

  // 1) 两个控件都换成了滑块（不再是分段按钮那一套）
  const ctxFn = SRC.slice(SRC.indexOf("function _modelContextRows"));
  assert.match(ctxFn.slice(0, 2000), /_micSliderHtml\(/, "上下文没渲染成滑块");
  const thinkAt = SRC.indexOf("thinkEl.innerHTML =");
  assert.notEqual(thinkAt, -1, "思考深度那段渲染没了");
  assert.match(SRC.slice(thinkAt, thinkAt + 600), /_micSliderHtml\(/, "思考深度没渲染成滑块");

  // 2) 两个滑块都绑了处理器。只渲染不绑定 = 拖得动、但存不进去。
  // 窗口必须**切在两个滑块之间**：从上下文那段一路读到思考深度那段的话，
  // 上下文的绑定被剪掉了也照样能在隔壁读到 _bindMicSlider——自己喂饱自己。
  // 先后不重要（上下文那条已挪到 `if (supports)` 之外），各自成段才重要。
  const ctxAt = SRC.indexOf("const ctxSl =");
  const thinkAt2 = SRC.indexOf("const thinkSl =");
  const posAt = SRC.indexOf("// Position:", Math.max(ctxAt, thinkAt2));
  assert.ok(ctxAt !== -1 && thinkAt2 !== -1 && posAt > Math.max(ctxAt, thinkAt2),
    "两个滑块的绑定段没了");
  const seg = (from) => SRC.slice(from, Math.min(...[ctxAt, thinkAt2, posAt].filter((n) => n > from)));
  const bind = seg(ctxAt);
  assert.match(bind, /_bindMicSlider\(/, "上下文滑块没绑处理器，拖了不存");
  const tbind0 = seg(thinkAt2);
  assert.match(tbind0, /_bindMicSlider\(/, "思考深度滑块没绑处理器，拖了不存");
  assert.match(bind, /_setCtxChoice\(/, "上下文滑块没写入选择");
  assert.match(tbind0, /_setThinkingPref\(/, "思考深度滑块没写入选择");

  // 3) 拖动中不得重画整张卡片。重画会把正在被拖的那个 input 换成新节点，
  //    指针立刻丢掉目标，拖到一半就断——这正是分段按钮时代 showModelInfoCard() 的做法。
  const binder = SRC.slice(SRC.indexOf("function _bindMicSlider"));
  const binderBody = binder.slice(0, binder.indexOf("\n}\n") + 3);
  assert.doesNotMatch(binderBody, /showModelInfoCard\(/,
    "拖动处理里重画了整张卡片，拖动会断在半路");

  // 4) 指针拖出卡片边界时不能收卡片，否则 input 随卡片一起消失。
  const leaveAt = SRC.indexOf('el.addEventListener("mouseleave"');
  assert.notEqual(leaveAt, -1, "卡片的 mouseleave 处理没了");
  // 抑制必须是**无状态**的：读事件自带的 buttons，而不是一个要靠 pointerup 清掉的标志位。
  // 标志位那一版只要 pointerup 没送达（拖动中窗口失焦、指针捕获被中断、拖到屏幕外松手），
  // 就永远挂着，卡片从此再也不会自动收起——表现就是"调完滑块后卡片赖着好几秒"。
  const leaveBody = SRC.slice(leaveAt, SRC.indexOf("});", leaveAt));
  assert.match(leaveBody, /if \(ev\.buttons\) return;/,
    "拖滑块时卡片仍会因为 mouseleave 被收走，拖到最右端必断");
  assert.doesNotMatch(SRC, /_sliderDrag/,
    "又用回那个要靠 pointerup 清掉的标志位了——它一旦漏清，卡片就再也收不起来");
  // 松手时若指针已不在卡片上要补收一次；但这条即使没跑到，也只是少收一次，
  // 不会把后续所有收起一起堵死。
  assert.match(SRC.slice(leaveAt, leaveAt + 900), /window\.addEventListener\("pointerup"/,
    "没有在松手时补收卡片");

  // 5) 买不到的档位要弹回并解释，不能静默钉住（那看起来就是滑块坏了）。
  assert.match(bind, /showToast\(/, "锁定档位没有任何提示，用户只会觉得滑块坏了");
});

test("AI 助手开关：按钮、面板、分隔条、落盘、开机还原，缺一环都不算能用", () => {
  const shell = readFileSync(join(HERE, "../src/app/Shell.jsx"), "utf8");
  // 1) 按钮在标题栏里，且在调试图标**左边**——用户指定的位置。
  const btnAt = shell.indexOf('id="toggleAssistantBtn"');
  const dbgAt = shell.indexOf('id="debugBtn"');
  assert.notEqual(btnAt, -1, "标题栏里没有这个按钮");
  assert.ok(btnAt < dbgAt, "按钮跑到调试图标右边去了");
  // 用仓库里已有的 i-sidebar-right（就是标准的 panel-right 几何：外框 + 一条竖分隔），
  // 视图菜单里那条同功能的项用的也是它。另画一个新图标只会让同一个动作有两种长相。
  assert.match(shell.slice(btnAt, btnAt + 300), /#i-sidebar-right-on/, "按钮没用面板图标");
  const html = readFileSync(join(HERE, "../index.html"), "utf8");
  // 两个状态是**两个图标**，同一副几何、只差右段填不填（实心＝面板在，空框＝已收起）。
  // 同一个图标只换颜色的话，光看图标判断不出当前是开是关——得先记住"亮的是开还是关"，
  // 那就不叫状态指示了。
  for (const sym of ["i-sidebar-right", "i-sidebar-right-on"]) {
    assert.match(html, new RegExp(`<symbol id="${sym}"`), `${sym} 没定义，按钮会画成空白方块`);
  }
  assert.match(SRC, /icon\.setAttribute\("href", open \? "#i-sidebar-right-on" : "#i-sidebar-right"\)/,
    "切换时没换图标，两个状态长得一模一样");
  // 图标要和右边那几个一样大。尺寸规则是 `.titlebar__action-group .tbtn--icon .ic`
  // （20px）；按钮放在 action-group 外面就只能拿到通用的 16px，肉眼一眼看得出小一圈。
  const groupAt = shell.lastIndexOf("titlebar__action-group", btnAt);
  assert.ok(groupAt !== -1 && shell.slice(groupAt, btnAt).split("</div>").length === 1,
    "按钮不在 titlebar__action-group 里，图标会比旁边的小一圈（16px vs 20px）");

  // 2) 点击走的是**既有的** togglePane("assistant")，不是另起一套。视图菜单里本来就有
  //    这个开关，CSS 的 .layout.hide-assistant 也早就同时收掉面板和分隔条；各做一套的
  //    结果是两个入口互相不认账——从菜单关掉，标题栏按钮还亮着"已展开"。
  const clickAt = SRC.indexOf('$("toggleAssistantBtn")?.addEventListener');
  assert.notEqual(clickAt, -1, "按钮没绑点击，点了什么都不会发生");
  const click = SRC.slice(clickAt, clickAt + 420);
  assert.match(click, /togglePane\("assistant"\)/, "按钮没复用既有的面板开关");
  assert.doesNotMatch(SRC, /function _applyAssistantVisibility/,
    "又长出了第二套收放机制，会和视图菜单那条互相不认账");

  // 3) 按钮要跟着**实际状态**走，包括从视图菜单改的那次。
  assert.match(click, /_syncAssistantToggleBtn\(\)/, "点完没同步按钮状态");
  const sync = SRC.slice(SRC.indexOf("function _syncAssistantToggleBtn"));
  assert.match(sync.slice(0, 500), /paneIsOpen\("assistant"\)/,
    "按钮状态不是从布局真值读的，会和实际显示对不上");

  // 4) 状态落盘 + 开机还原。少了任何一半，用户收起来的面板重启后又弹回来。
  // 只切 togglePane 的**函数体**。往后多读几行就会读到紧邻的 function _savePaneState()
  // 定义，那样即便 togglePane 里根本没调它，按名字找的断言照样能通过。
  const tpAll = SRC.slice(SRC.indexOf("function togglePane"));
  const tp = tpAll.slice(0, tpAll.indexOf("\n}\n") + 3);
  assert.match(tp, /_savePaneState\(\)/, "开关状态没存，重开 IDE 就丢");
  assert.match(SRC, /_restorePaneState\(\);/, "启动时没还原，存了也等于没存");

  // 5) 布局变了要让编辑器重算宽度，否则代码区停在旧宽度上，右边空一大块。
  assert.match(click, /new Event\("resize"\)/,
    "收放面板后没触发重算，编辑器会停在旧宽度");
});

test("设置面板的下拉必须是自绘组件：菜单在控件正下方、同宽，且点击不留焦点环", () => {
  const css = readFileSync(join(HERE, "../src/styles/app.css"), "utf8");

  // 1) 面板内不许再有原生 <select>。原生控件的弹出菜单是系统画的：盖在控件上、
  //    宽度按最长选项算，位置和宽度 CSS 一行都管不着——"菜单在下方、同宽"这个要求
  //    在原生控件上根本无法满足，打扮得再像也没用。
  const selAt = SRC.indexOf('createElement("select")');
  assert.equal(selAt, -1, "又建原生 select 了，它的弹出菜单没法按要求定位");

  // 2) 两处入口都走同一个组件——否则同一个面板里两种下拉各弹各的。
  assert.match(SRC, /function buildSelectControl\(/, "自绘下拉组件没了");
  const bc = SRC.slice(SRC.indexOf("function buildSettingControl"));
  assert.match(bc.slice(0, 700), /buildSelectControl\(/, "通用设置行没用自绘下拉");
  const mk = SRC.slice(SRC.indexOf("const makeSelect = "));
  assert.match(mk.slice(0, 400), /buildSelectControl\(/, "自适应页没用自绘下拉");

  // 3) 菜单必须与控件同宽、贴在正下方。这三行是"对齐"这件事的全部实现。
  const open = SRC.slice(SRC.indexOf("const r = btn.getBoundingClientRect();"));
  assert.match(open.slice(0, 600), /menu\.style\.width = `\$\{r\.width\}px`/, "菜单没跟控件同宽");
  assert.match(open.slice(0, 600), /menu\.style\.left = `\$\{r\.left\}px`/, "菜单左缘没和控件对齐");
  assert.match(open.slice(0, 600), /r\.bottom \+ 4/, "菜单没贴在控件下方");

  // 4) 键盘要能用。原生 select 白送的东西，自绘就得自己补——少一样键盘用户就用不了。
  const kd = SRC.slice(SRC.indexOf('btn.addEventListener("keydown"'));
  for (const key of ["Escape", "ArrowDown", "ArrowUp", "Enter"]) {
    assert.match(kd.slice(0, 1200), new RegExp(key), `键盘少了 ${key}`);
  }

  // 4a) 高亮必须由 JS 自己挂类名，**不能**靠 :focus-visible。键盘移动时是程序化
  //     focus，而 focus-visible 由浏览器按"这次焦点是不是键盘引起的"启发式判定，
  //     程序化 focus 在 WKWebView 里经常不算——表现就是上下键走下去一路没有高亮。
  assert.match(SRC, /el\.classList\.add\("is-active"\)/, "选项高亮没有由 JS 挂类名");
  assert.doesNotMatch(css, /\.mselect__opt:focus-visible/,
    "又把选项高亮压在 :focus-visible 上了，键盘走下去会没有高亮");
  assert.match(css, /\.mselect__opt\.is-active\s*\{[^}]*background:/, "高亮没有可见的底色");
  // 菜单挂在 document.body 上，**不是** .feature-panel 的后代，所以取不到 --feature-*。
  // 用了又不写兜底的话，整条声明直接作废——类名挂上了、颜色是空的，表现就是"悬停毫无反应"。
  const menuCss = css.slice(css.indexOf(".mselect__menu {"), css.indexOf(".mselect__opt.is-on::after"));
  // 这个浮层挂在 document.body 上，不在任何面板的令牌作用域里。var() 的兜底只在变量
  // **未定义**时生效——定义了却对该属性无效时，整条声明作废、回到初始值（background
  // 的初始值是 transparent），表现就是"菜单透明、底下内容透出来"。表面色一律写字面值。
  // 只管**颜色**：--font 是 :root 上的字体栈，取不到也只是回退字体，不会让整块变透明。
  assert.doesNotMatch(menuCss, /(?:background|background-color|color|border|box-shadow)[^;]*var\(--(?!font)/,
    "浮层的颜色又绕回 CSS 变量了——它不在面板作用域里，取不到就整条作废、回到透明");
  assert.match(menuCss, /background-color:\s*#/, "菜单没有实色底，会透出底下的内容");
  assert.match(css, /:root\[data-theme="dark"\] \.mselect__menu\s*\{[^}]*background-color:/,
    "深色主题下菜单没有自己的底色");
  // 键盘走到视口外的项要带进来，否则高亮跑到看不见的地方。
  // 只在 setActive 的函数体里找——main.js 别处也有 scrollIntoView，扫全文会被喂饱。
  const sa = SRC.slice(SRC.indexOf("const setActive = (i) =>"));
  assert.match(sa.slice(0, sa.indexOf("\n  };") + 5), /scrollIntoView/,
    "键盘移动时没把当前项滚进视野，高亮会跑到看不见的地方");

  // 4b) 只有菜单**外面**的滚动才关菜单。直接把 close 挂在 window 捕获阶段的话，
  //     在菜单里滚滚轮同样会被捕获到，表现就是"菜单根本滚不动"。
  const sc = SRC.slice(SRC.indexOf("const onScroll ="));
  assert.match(sc.slice(0, 200), /menu\.contains\(ev\.target\)/,
    "菜单内部的滚动也会关掉菜单——菜单会滚不动");
  assert.doesNotMatch(SRC, /window\.addEventListener\("scroll", close/,
    "又把 close 直接挂到 scroll 上了");

  // 5) 鼠标点完不留灰描边：焦点环只给 :focus-visible。
  assert.match(css, /\.settings-input:focus \{[^}]*box-shadow:\s*none/,
    "鼠标点完还挂着一圈灰描边");
  assert.match(css, /\.settings-input:focus-visible \{[^}]*box-shadow:\s*0 0 0 3px/,
    "键盘焦点环没了，键盘用户看不出焦点在哪");

  // 6) 控件轨道固定宽度——右缘齐靠统一宽度，左缘齐靠固定轨道。
  assert.match(css, /\.settings-row__control\s*\{[^}]*flex:\s*0 0 220px/,
    "控件列不是固定轨道，左缘会参差不齐");
  assert.match(css, /\.settings-row__control > \.settings-toggle\s*\{[^}]*flex:\s*0 0 42px/,
    "开关被拉宽了");
});

test("数字设置项要有自绘步进器，而不是一个裸文本框", () => {
  // 之前为了去掉 macOS 那个系统步进器，把 <input type=number> 的原生箭头藏了——
  // 连带把"这个值可以加减"的提示也一起拿掉了，剩一个看不出能干嘛的文本框。
  const css = readFileSync(join(HERE, "../src/styles/app.css"), "utf8");
  assert.match(SRC, /function buildNumberControl\(/, "步进器组件没了");
  const bc = SRC.slice(SRC.indexOf("function buildSettingControl"));
  assert.match(bc.slice(0, 900), /buildNumberControl\(/, "数字设置项没用步进器");

  // 中间必须还是真的 number 输入框：键入、↑/↓、读屏器的数值语义都靠它。
  const nc = SRC.slice(SRC.indexOf("function buildNumberControl"));
  const body = nc.slice(0, nc.indexOf("\n}\n") + 3);
  assert.match(body, /inp\.type = "number"/, "中间不是真的数字输入框，键盘和读屏器会失去数值语义");
  // 到头置灰，否则用户会对着一个点了没反应的按钮反复点。
  assert.match(body, /dec\.disabled = v <= min/, "减到下限没置灰");
  assert.match(body, /inc\.disabled = v >= max/, "加到上限没置灰");
  // 越界要钳住，不能靠 UI 拦——用户可以直接把数字键进去。
  assert.match(body, /Math\.min\(max, Math\.max\(min,/, "直接键入的值没有钳位");

  // 外观上和下拉框共用同一套边框/圆角/高度，否则同一行里两种控件长相不一。
  assert.match(css, /\.mnum\s*\{[^}]*height:\s*32px/, "步进器高度和下拉框对不上");
  assert.match(css, /\.mnum\s*\{[^}]*border-radius:\s*var\(--feature-radius-control/, "圆角和下拉框对不上");
});

test("面板里的按钮不能出现蓝字压蓝底，页尾动作要居中", () => {
  const css = readFileSync(join(HERE, "../src/styles/app.css"), "utf8");
  // 上一版给面板内的 .btn 统一改了字色，却没管 .btn--primary 那个实心蓝底——
  // 蓝字压蓝底，整颗按钮看起来是**空白**的。两者必须一起收进同一副长相。
  const at = css.indexOf(".feature-panel__body .btn,");
  assert.notEqual(at, -1, "面板内的按钮样式没了");
  const block = css.slice(at, at + 500);
  assert.match(block, /\.feature-panel__body \.btn--primary/,
    "只改了 .btn 没管 .btn--primary——它的实心蓝底还在，字会看不见");
  assert.match(block, /background:\s*var\(--feature-control\)/, "按钮底色没跟着改");
  // 页尾动作居中，和上面居中的内容列对齐。
  assert.match(css, /\.settings-actions\s*\{[^}]*justify-content:\s*center/,
    "页尾按钮没居中，会和居中的内容列对不上");

  // 自适应页那颗「保存」按钮已删：上面每一项都是 onchange 就落盘的，它唯一真正保存的
  // 是同时被删掉的那个偏好编辑框。留着就是一颗按下去什么都不改变、却让人以为"不按就
  // 没生效"的按钮。
  const adaptive = SRC.slice(SRC.indexOf("function renderAdaptiveTool"), SRC.indexOf("const SETTINGS_SCHEMA"));
  assert.doesNotMatch(adaptive, /adaptive-notes/, "那个偏好编辑框不该回来——记忆中心才是这份数据的正主");
  assert.doesNotMatch(adaptive, /_saveKgText\(/, "保存按钮回来了，但它已经没有要保存的东西");
  assert.match(adaptive, /actions\.append\(reset, memory\)/, "页尾按钮不是预期的两颗");
});

test("快捷键显示必须分平台：Mac 用符号，Windows 用词并带加号", () => {
  // ⇧ ⏎ ⌫ 这些字形在 Windows 上既不是系统习惯，很多字体里还缺字，会渲染成方框。
  // 上一版只有 mod/ctrl/alt/meta 做了分支，shift/enter/backspace 无论什么平台都吐
  // Mac 符号；shortcutLabel 又是无分隔连写的，Windows 上会出现 "CtrlShiftP"。
  const cut = (n) => {
    const i = SRC.indexOf("function " + n + "(");
    assert.notEqual(i, -1, `${n} 没了`);
    let d = 0;
    for (let k = SRC.indexOf("{", i); k < SRC.length; k++) {
      if (SRC[k] === "{") d++;
      else if (SRC[k] === "}") { d--; if (!d) return SRC.slice(i, k + 1); }
    }
    return "";
  };
  const build = (platform) => new Function(
    "navigator",
    cut("isMacPlatform") + cut("formatCombo") + cut("shortcutLabel") + ";return shortcutLabel;",
  )({ platform });

  const mac = build("MacIntel");
  const win = build("Win32");
  assert.equal(mac("mod+shift+p"), "⌘⇧P", "Mac 上应该是符号连写");
  assert.equal(win("mod+shift+p"), "Ctrl+Shift+P", "Windows 上应该是词 + 加号");
  // 这三个是上一版漏掉分支的
  assert.equal(win("alt+enter"), "Alt+Enter", "Windows 上 enter 还在吐 Mac 的 ↩");
  assert.equal(win("mod+backspace"), "Ctrl+Backspace", "Windows 上 backspace 还在吐 ⌫");
  assert.equal(mac("mod+backspace"), "⌘⌫", "Mac 上应保持符号");
});

test("快捷键表要覆盖真正生效的键，且每个动作都得有实现", () => {
  // 之前设置页只登记了 15 条，而缩放、Markdown 预览、删除文件这些是各自挂 keydown 的
  // ——在设置页里既查不到也改不了。现在它们都进了同一张表。
  const labels = SRC.slice(SRC.indexOf("const ACTION_LABELS = {"));
  const labelBlock = labels.slice(0, labels.indexOf("\n};") + 3);
  const acts = SRC.slice(SRC.indexOf("const KB_ACTIONS = {"));
  const actBlock = acts.slice(0, acts.indexOf("\n};") + 3);
  const defs = SRC.slice(SRC.indexOf("function _defaultKeybindings()"));
  const defBlock = defs.slice(0, defs.indexOf("\n}\n") + 3);

  for (const id of ["view.markdownPreview", "view.zoomIn", "view.zoomOut", "view.zoomReset",
                    "file.deleteSelected", "view.extensions", "view.bookmarks", "memory.manage"]) {
    assert.ok(labelBlock.includes(`"${id}"`), `${id} 没有中文标签，设置页里不会出现`);
    assert.ok(defBlock.includes(`"${id}"`), `${id} 没有默认键位，用户不改键就永远按不出来`);
  }
  // 每个有标签的动作都必须有实现——只有标签没实现的话，设置页列出来、按下去什么都不发生。
  for (const m of labelBlock.matchAll(/"([\w.]+)":/g)) {
    assert.ok(actBlock.includes(`"${m[1]}"`), `${m[1]} 只有标签没有实现`);
  }

  // 那些自己挂 keydown 的都要拆掉，否则同一个键会走两条路（其中一条还改不了）。
  assert.doesNotMatch(SRC, /if \(\(e\.metaKey \|\| e\.ctrlKey\) && e\.key === "\."\)/,
    "Markdown 预览又自己挂 keydown 了，设置页里改不了它");
  assert.doesNotMatch(SRC, /k === "0"\) \{ e\.preventDefault\(\); _applyUiZoom\(1\)/,
    "缩放又自己挂 keydown 了");

  // 删除文件是分发器上唯一会破坏数据的动作：焦点在编辑器/终端/输入框里时必须让路，
  // 否则在聊天框里按退格会删掉磁盘上的文件。守卫必须在动作函数**自己**身上。
  // 这里**跑**这个函数，不是看它源码里有没有那几个字符串——把条件 `&& false` 掉，
  // 按名字找的断言照样通过，而守卫已经形同虚设。
  const del = SRC.slice(SRC.indexOf("function _deleteSelectedTreeItem"));
  const delBody = del.slice(0, del.indexOf("\n}\n") + 3);
  const runDelete = (activeElement) => {
    let deleted = false;
    new Function(
      "_treeSel", "document", "_deleteSelectedTree",
      delBody + ";return _deleteSelectedTreeItem;",
    )(
      new Set(["/x/a.txt"]),
      { activeElement },
      () => { deleted = true; },
    )();
    return deleted;
  };
  const el = (tag, closestHit = null) => ({
    tagName: tag,
    isContentEditable: false,
    closest: (sel) => (sel === closestHit ? {} : null),
  });
  assert.equal(runDelete(el("DIV")), true, "焦点在普通元素上时应该真的删除");
  assert.equal(runDelete(el("INPUT")), false, "焦点在输入框里还删文件——用户按的是退格删字");
  assert.equal(runDelete(el("TEXTAREA")), false, "焦点在多行输入框里还删文件");
  assert.equal(runDelete(el("DIV", ".monaco-editor")), false, "焦点在编辑器里还删文件");
  assert.equal(runDelete(el("DIV", ".xterm")), false, "焦点在终端里还删文件");

  // 平台分两套：Windows 的微软拼音会吃掉 Ctrl+.；Mac 惯例是 ⌘⌫ 而 Windows 是裸 Delete。
  assert.match(defBlock, /mac \? "mod\+\." : "mod\+shift\+v"/, "Markdown 预览没有分平台");
  assert.match(defBlock, /mac \? "mod\+backspace" : "delete"/, "删除键没有分平台");
});

test("MCP 页：已停用排在已装服务之后，卡片不靠整块染色表达状态", () => {
  const css = readFileSync(join(HERE, "../src/styles/app.css"), "utf8");
  // 一进 MCP 页第一眼该看到正在跑的服务。已停用是补救入口，不是主角。
  const at = SRC.indexOf("installedEl.innerHTML = brokenBanner + installedNames.map");
  assert.notEqual(at, -1, "已装服务列表的拼装变了");
  const stmt = SRC.slice(at, at + 6000);
  const joinAt = stmt.indexOf('}).join("")');
  assert.notEqual(joinAt, -1, "已装列表的收尾没了");
  assert.match(stmt.slice(joinAt, joinAt + 40), /\}\)\.join\(""\) \+ disabledRows/,
    "已停用又跑到已装服务前面去了");

  // 状态不靠整张卡染色。一屏七八张卡各有底色时页面像打翻调色盘，而且"绿底"和"选中"
  // 在这个面板别处是两个意思。
  const at2 = css.indexOf(".feature-panel__body .mcpfp-card--installed.is-on");
  assert.notEqual(at2, -1, "卡片状态色的统一规则没了");
  assert.match(css.slice(at2, at2 + 400), /background:\s*var\(--feature-card\)/,
    "卡片又按状态染整块底色了");
  // 状态徽标走语义令牌，深浅两套自动跟着变——原来是写死的 #34a853 / #d93025 那一套。
  // 选择器是个分组（… .is-on, … .mcpfp-badge--count { ），不能假设 `{` 紧跟其后。
  const okAt = css.indexOf(".feature-panel__body .mcpfp-row__status.is-on");
  assert.notEqual(okAt, -1, "状态徽标的规则没了");
  assert.match(css.slice(okAt, css.indexOf("}", okAt)), /var\(--feature-ok\)/,
    "状态徽标没走语义令牌，深色下不会跟着变");
  // 「安装」不再是整块实心主色。
  const btnAt = css.indexOf(".feature-panel__body .ctp-btn--primary");
  assert.notEqual(btnAt, -1, "市场里的主按钮没收进面板的按钮语言");
  assert.match(css.slice(btnAt, css.indexOf("}", btnAt)), /background:\s*var\(--feature-control\)/,
    "主按钮还是整块实心主色");
});

test("Skills：外部技能可删，但删之前必须把磁盘路径摆出来", () => {
  // 原来外部目录（用户 / 插件目录）的技能一律只能停用，卡片上连删除按钮都不画——
  // 用户看到的是"这一堆技能没有删除功能"。按所有者要求放开了，但这一下会删到工作区
  // **外面**的文件夹，所以必须先把完整路径摆给用户看。
  const can = SRC.slice(SRC.indexOf("function _skillCanDelete"));
  const canBody = can.slice(0, can.indexOf("\n}\n") + 3);
  assert.match(canBody, /return !!String\(skill\.baseDir \|\| ""\)\.trim\(\)/,
    "外部技能又不能删了");

  const del = SRC.slice(SRC.indexOf("async function _deleteSkillRecord"));
  const delBody = del.slice(0, del.indexOf("\n}\n") + 3);
  assert.match(delBody, /confirm\(/, "删工作区外的目录居然不确认");
  assert.match(delBody, /\$\{dir\}/, "确认框里没写清楚要删哪个目录");
  assert.match(delBody, /if \(!ok\) return;/, "用户点了取消还照删");
  // 确认只针对**工作区外**的；工作区里自己装的不必每次拦一道。
  assert.match(delBody, /if \(!_skillIsWorkspaceInstalled\(skill, skillRoot\)\) \{/,
    "把工作区内的删除也拦上了确认框，那是多余的摩擦");
  // 分区标题不能再说"不能在这里删除"——现在能删了，留着就是假话。
  assert.doesNotMatch(SRC, /只读，可开关但不能在这里删除/,
    "分区标题还写着不能删除，和实际行为对不上");
});

test("侧栏毛玻璃要真能透出后面，且不支持时必须退回实色", () => {
  const css = readFileSync(join(HERE, "../src/styles/app.css"), "utf8");
  // 玻璃要有效果，背后得有东西可糊。面板本体和 sheet 必须是透明的——它们只要有实色，
  // 侧栏糊的就是自己那层白底，等于白做。
  const panelAt = css.indexOf("\n.feature-panel {");
  assert.match(css.slice(panelAt, css.indexOf("}", panelAt)), /background:\s*transparent/,
    "面板本体不透明，侧栏的模糊糊的是它自己的底色，看不出任何效果");
  const sheetAt = css.indexOf("\n.feature-panel__sheet {");
  assert.match(css.slice(sheetAt, css.indexOf("}", sheetAt)), /background:\s*transparent/,
    "sheet 不透明，同上");
  // 内容区必须**仍然是实色**：正文压在模糊的代码上没法读。
  const bodyAt = css.indexOf("\n.feature-panel__body {");
  assert.match(css.slice(bodyAt, css.indexOf("}", bodyAt)), /background:\s*var\(--feature-bg\)/,
    "内容区跟着透明了，正文会压在工作区上");

  // 用行首锚定：@media (max-width:720px) 里也有一条同名规则（缩成图标栏那个），
  // 它缩进两格，indexOf 会先撞上它。
  const railAt = css.indexOf("\n.feature-panel__tabs {");
  const rail = css.slice(railAt, css.indexOf("}", railAt));
  assert.match(rail, /backdrop-filter:\s*blur/, "侧栏没有背景模糊");
  assert.match(rail, /-webkit-backdrop-filter/, "缺 -webkit- 前缀，WKWebView 上不生效");
  // 匹配**属性值本身**，不是"规则体里出现过 saturate 这个词"——解释它为什么重要的
  // 注释里也写着这个词，按词去找会被自己的注释喂饱。
  assert.match(rail, /backdrop-filter:\s*blur\([^)]*\)\s+saturate\(/,
    "只 blur 不提饱和度，背后内容会褪成一片脏灰");
  assert.match(rail, /background:\s*var\(--feature-glass\)/, "侧栏没有半透明底");
  // 玻璃是靠**边缘的高光**被认出来的，只有模糊时更像"背景没渲染好"。
  assert.match(rail, /box-shadow:\s*inset 1px 0 0 var\(--feature-glass-edge\);/,
    "侧栏少了玻璃的亮边，看着只是一块半透明色块");
  // 右缘不许再压内阴影：那道渐暗会让交界处看着像右边的内容盖在侧栏上面，
  // 而这两栏是平级的，分栏靠那条 1px 发丝线就够了。
  // 先剥注释：解释"原来那道内阴影长什么样"的注释里就写着它的值，不剥就会被自己喂饱。
  assert.doesNotMatch(rail.replace(/\/\*[\s\S]*?\*\//g, ""), /inset -\d+px 0 \d+px/,
    "交界处又加了内阴影，看着像右边内容压在侧栏上");
  // 底色不能太实。72% 那一版看不出效果——侧栏背后正好是纯色的文件树面板，
  // 模糊一块纯色出来还是那块纯色，得让背后的内容真的透上来一点。
  const glassAt = css.indexOf("--feature-glass: rgba(252");
  assert.notEqual(glassAt, -1, "浅色玻璃色没了");
  const alpha = Number(/rgba\([^)]*,\s*\.(\d+)\)/.exec(css.slice(glassAt, glassAt + 60))?.[1] || "99");
  assert.ok(alpha <= 60, `玻璃底太实（.${alpha}），背后的内容透不上来，看不出是玻璃`);
  // 深浅两套各自的玻璃色。
  for (const [sel, who] of [[".feature-panel {", "浅色"], [':root[data-theme="dark"] .feature-panel {', "深色"]]) {
    const at = css.indexOf("\n" + sel);
    assert.match(css.slice(at, css.indexOf("}", at)), /--feature-glass:/, `${who}主题没有玻璃色`);
  }
  // 不支持 backdrop-filter 时必须退回实色，否则文字直接压在没被模糊的代码上。
  assert.match(css, /@supports not \(\(backdrop-filter[\s\S]{0,220}background:\s*var\(--feature-rail\)/,
    "没有降级路径，不支持模糊的环境里侧栏文字会压在工作区上");
});

test("毛玻璃的祖先链不许出现切断 backdrop 的属性", () => {
  // 这条是补给上一条的**反向**断言，也是这次真正漏掉的那道缝。
  //
  // 上一条只检查侧栏自己写没写 backdrop-filter —— 它一直是绿的，而效果一直是坏的：
  // .feature-panel__sheet 上挂着 `animation: feature-fade … both`，而那个 keyframes 动的
  // 是 opacity。fill-mode:both 让这条 opacity 动画播完之后永久生效，浏览器据此把 sheet
  // 当成 backdrop root，后代的 backdrop-filter 只能采样这个 root 内部——而那里面三层
  // 全是透明的，等于对空气做模糊。
  const css = readFileSync(join(HERE, "../src/styles/app.css"), "utf8");
  const ruleBody = (sel) => {
    const at = css.indexOf("\n" + sel + " {");
    assert.notEqual(at, -1, `${sel} 规则没了`);
    return css.slice(at, css.indexOf("\n}", at));
  };
  // 注释里会提到这些词（正是解释它们为什么危险的那段），所以先把注释剥掉再断言。
  const strip = (t) => t.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const sel of [".feature-panel", ".feature-panel__sheet", ".feature-panel__main"]) {
    const body = strip(ruleBody(sel));
    for (const bad of ["opacity:", "filter:", "mask:", "clip-path:", "will-change:", "isolation:", "contain:"]) {
      assert.ok(!body.includes(bad),
        `${sel} 上出现了 ${bad} —— 它会成为 backdrop root，侧栏和顶栏的毛玻璃会当场变成一层白纱`);
    }
    // animation 同样危险：只要它引用的 keyframes 动了 opacity，效果和直接写 opacity 一样。
    const anim = /animation:\s*([\w-]+)/.exec(body);
    if (anim) {
      const kf = new RegExp(`@keyframes ${anim[1]}\\s*\\{[^}]*\\}[^}]*\\}`).exec(css)?.[0] || "";
      assert.ok(!/opacity/.test(kf),
        `${sel} 上的 animation ${anim[1]} 动了 opacity —— 和直接写 opacity 一个后果`);
    }
  }
  // 动画得挂在实体层自己身上：元素自己的 opacity 动画不会切断它自己的 backdrop-filter。
  for (const sel of [".feature-panel__head", ".feature-panel__tabs", ".feature-panel__body"]) {
    assert.match(ruleBody(sel), /animation:\s*feature-fade/, `${sel} 少了淡入，整屏淡入会缺一块`);
  }
});

test("拖滑块时不许每一帧都落盘", () => {
  // 这个仓库的性能记录里，localStorage.setItem 是最常见的多秒卡顿源（见 logic.test.mjs
  // 那条「空闲期卡死」：实测 120 次 2–60s 的卡顿里它出现得最多）。而 input 事件在拖动时
  // 每移动一点就触发一次，一次拖动几十上百次——每次都写一遍盘，松手后界面顿好几秒。
  const bind = SRC.slice(SRC.indexOf("function _bindMicSlider"));
  const body = bind.slice(0, bind.indexOf("\n}\n") + 3);
  assert.match(body, /addEventListener\("input",[\s\S]{0,80}resolve\(false\)/,
    "input 还在提交——拖动时每一帧都会落盘");
  assert.match(body, /addEventListener\("change",[\s\S]{0,80}resolve\(true\)/,
    "change 不提交的话，拖完根本存不下来");

  // 两个调用方都必须认这个 commit 参数：只要有一个不认，那一条滑块照旧每帧写盘。
  // 窗口必须**切在两条滑块之间**：从上下文那段一路读到思考深度那段的话，上下文的
  // `&& commit` 被删掉了也照样能在隔壁读到——自己喂饱自己。
  // 两段的**先后不重要**，重要的是各自成段：上下文那条已经被挪到 `if (supports)` 之外
  // （不支持思考深度的模型此前滑块画得出来却拖不动），所以顺序反过来了。按各自的起点
  // 到"下一个起点或收尾"来切，谁在前都能测。
  const ctxAt = SRC.indexOf("const ctxSl =");
  const thinkAt = SRC.indexOf("const thinkSl =");
  const endAt = SRC.indexOf("// Position:", Math.max(ctxAt, thinkAt));
  assert.ok(ctxAt !== -1 && thinkAt !== -1 && endAt > Math.max(ctxAt, thinkAt),
    "两条滑块的绑定段没了");
  const cut = (from) => SRC.slice(from, Math.min(...[ctxAt, thinkAt, endAt].filter((n) => n > from)));
  for (const [seg, who] of [[cut(ctxAt), "上下文"], [cut(thinkAt), "思考深度"]]) {
    assert.match(seg, /\(want, commit\) =>/, `${who}滑块没接 commit 参数`);
    assert.match(seg, /&& commit\)/, `${who}滑块拖动途中仍在落盘`);
  }
});

test("前端调的每个后端命令，Rust 侧都注册了", () => {
  // wiring.test 上面那几条守的是 schema → _mapToolCall → 处理器这条链。它管不到最后一跳：
  // 处理器里 `invoke("some_command")` 的那个命令名，Rust 的 generate_handler! 里到底有没有。
  // 少一个的表现是**只在运行时报错**——某个工具一调就失败，而全套测试照样全绿。
  //
  // 剥掉超长字符串字面量再扫：项目脚手架模板里带着示例 Rust + React 代码，里面有
  // `invoke("write_note")` 这种示例调用，它不是真调用（`write_note` 确实不该注册）。
  // 按"超长字面量"剥而不是写死排除名单，将来新增的模板也自动排除。
  const stripReal = (s) => s
    .replace(/'(?:[^'\\\n]|\\.){300,}'/g, "''")
    .replace(/"(?:[^"\\\n]|\\.){300,}"/g, '""');
  const invoked = [...new Set(
    [...stripReal(SRC).matchAll(/invoke\(\s*["']([a-z0-9_]+)["']/g)].map((m) => m[1]),
  )];
  assert.ok(invoked.length > 150, `只扫出 ${invoked.length} 个 invoke——正则失效了，这条断言等于没跑`);

  const LIB = readFileSync(join(HERE, "..", "src-tauri", "src", "lib.rs"), "utf8");
  const handler = /generate_handler!\s*\[([\s\S]*?)\]/.exec(LIB);
  assert.ok(handler, "找不到 generate_handler!——它被改名或挪走了，这条断言失去落点");
  const registered = new Set(
    [...handler[1].matchAll(/(?:[a-z0-9_]+::)*([a-z0-9_]+)/g)].map((m) => m[1]),
  );

  const missing = invoked.filter((c) => !registered.has(c));
  assert.deepEqual(missing, [],
    `前端会调、但 Rust 没注册的命令（调到就报错，且只有运行时才看得出来）：${missing.join(", ")}`);
});

test("发给上游的请求一律走 SSE——同步请求在中转那边是整段生成完才回", () => {
  // 用户实拍：中转（Sub2API）控制台里请求类型大多写着"同步"，只有一行"流式"。
  // 网关日志侧量到的正是那个形状：upstream_header_ms 8~40 秒，而
  // first_upstream_chunk_after_headers_ms 恒为 0（headers 一到正文就全在）。
  // 同一个 API 在 Claude Code / Codex 里飞快，就是因为那边是真流式。
  //
  // 一处漏改不会报错，只会表现成"这条路径特别慢"，所以按结构扫两棵源码树。
  const roots = [
    join(HERE, "..", "src-tauri", "src"),
    join(HERE, "..", "..", "server", "src"),
  ];
  const offenders = [];
  for (const root of roots) {
    for (const name of readdirSync(root)) {
      if (!name.endsWith(".rs")) continue;
      const text = readFileSync(join(root, name), "utf8");
      text.split("\n").forEach((line, i) => {
        if (/"stream"\s*:\s*false/.test(line)) offenders.push(`${name}:${i + 1}`);
      });
    }
  }
  assert.deepEqual(offenders, [],
    "这些出站请求还是同步的——中转会等整段生成完才回，用户那边就是干等：" + offenders.join(", "));
});

test("harness 的编排信封两侧必须是同一个字面量，否则「谁在说话」就量不出来", () => {
  // 和 📌 边界同一类：两个文件、两种语言约定同一个字符串，漂了没有任何东西会报错——
  // 网关只是从此统计到 0 条编排消息，看起来跟"这一轮 harness 很安静"一模一样。
  //
  // 为什么要量：2026-08-17 实测，用户的一句话 83 字节，组装后发出去 21,643 字节（1:260），
  // 而运行中还能继续插话的提醒有 25 类。每一段单看都有道理，合起来就把人挤出去了，
  // 却没有任何一处代码为"人的话占多少比重"负责。
  const client = /const _ORCH_NOTE = "([^"]+)"/.exec(SRC);
  assert.ok(client, "_ORCH_NOTE 在 main.js 里改名或挪走了——网关那侧的统计会静默归零");

  const rust = readFileSync(join(HERE, "../../server/src/prompts.rs"), "utf8");
  const marker = /const ORCH_NOTE_MARKER: &str = "([^"]+)";/.exec(rust);
  assert.ok(marker, "ORCH_NOTE_MARKER 在 prompts.rs 里改名或挪走了");

  assert.ok(client[1].startsWith(marker[1]),
    `网关认的前缀不是客户端信封的开头，统计会恒为 0：\n  客户端 ${client[1].slice(0, 24)}…\n  网关   ${marker[1]}`);
  assert.ok(marker[1].length >= 6, "前缀太短，会误伤正常正文里碰巧出现的字");

  // 统计必须真的记进那条装配日志，否则量了也看不到。
  assert.match(rust, /orch_msg_count,\s*\n\s*orch_bytes,/,
    "harness 话语量没有进 assembled IDE prompt request 那条日志——量了看不到等于没量");
  // 只统计结构，不记内容。
  assert.doesNotMatch(rust, /orch_(bytes|msg_count)\s*=\s*%/, "这两个字段只能记数字，不能记正文");
});
