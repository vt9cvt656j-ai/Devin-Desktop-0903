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
import { readFileSync } from "node:fs";
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
