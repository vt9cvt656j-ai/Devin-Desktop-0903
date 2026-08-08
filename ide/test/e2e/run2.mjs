import { fileURLToPath as __f2u } from "node:url";
import { register } from "node:module";
import { pathToFileURL } from "node:url";
import * as fs from "node:fs";
import * as fsp from "node:fs/promises";
import { spawn } from "node:child_process";
import { mkdtempSync } from "node:fs";
import assert from "node:assert/strict";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
const __HERE = dirname(__f2u(import.meta.url));
const __IDE = join(__HERE, "..", "..");
const S = __HERE + "/";
process.env.__DEEPSTUB_URL = pathToFileURL(S + "deepstub.mjs").href;
register(pathToFileURL(S + "hooks.mjs"));
const { installGlobals } = await import(S + "globals.mjs");
installGlobals();
globalThis.__viteEnv = { DEV:false, PROD:true, MODE:"test", BASE_URL:"/" };
globalThis.__viteGlob = () => ({});
globalThis.__E2E__ = true;
globalThis.fetch = () => Promise.reject(new Error("E2E: network disabled"));
await import(join(__IDE, "src", "main.js"));
const H = globalThis.__testHooks;
console.log("current mode:", H.getMode());

// ---- real fs backend rooted at a temp dir ----
const ROOT = mkdtempSync(join(tmpdir(), "e2e-"));
console.log("ROOT:", ROOT);
const B = H.backend;
const before = Object.keys(B).length;
B.readTextFile = async (p) => fsp.readFile(p, "utf8");
B.writeTextFile = async (p, c) => { await fsp.mkdir(join(p, ".."), { recursive:true }); await fsp.writeFile(p, c); return true; };
// REAL signature: (path, expectedContent, content) -- content is 3rd, CAS on 2nd.
B.writeTextFileIfUnchanged = async (p, expectedContent, content) => {
  const existsNow = fs.existsSync(p);
  if (expectedContent == null ? existsNow : (!existsNow || fs.readFileSync(p,"utf8") !== expectedContent))
    throw new Error("[CONFLICT] file changed after it was read");
  await fsp.mkdir(join(p,".."),{recursive:true}); await fsp.writeFile(p, content); return true; };
B.deleteTextFileIfUnchanged = async (p, expectedContent) => { await fsp.rm(p, { force:true }); return true; };
B.deletePath = async (p) => { await fsp.rm(p, { recursive:true, force:true }); return true; };
B.homeDir = async () => ROOT;
B.createFile = async (p) => { await fsp.mkdir(join(p,".."),{recursive:true}); await fsp.writeFile(p, "", { flag:"a" }); return true; };
B.createDir = async (p) => { await fsp.mkdir(p, { recursive:true }); return true; };
B.readDir = async (p) => (await fsp.readdir(p, { withFileTypes:true })).map(d=>({ name:d.name, path:join(p,d.name), isDir:d.isDirectory(), is_dir:d.isDirectory() }));
B.inspectFile = async (p) => { try { const st=await fsp.stat(p); return { exists:true, isDir:st.isDirectory(), size:st.size }; } catch { return { exists:false }; } };
// Mirror the captured-command contract exposed by Tauri while keeping this
// harness inside its temporary workspace and using only short-lived shells.
B.taskRunCapture = (cwd, command, options = {}) => new Promise((resolve, reject) => {
  const child = spawn("/bin/sh", ["-lc", String(command)], {
    cwd,
    env: { ...process.env, CI: "1", TERM: "dumb" },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "", stderr = "", timedOut = false;
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  const timeout = setTimeout(() => { timedOut = true; child.kill("SIGKILL"); }, Math.max(1, Number(options.timeoutSecs) || 15) * 1000);
  child.once("error", (error) => { clearTimeout(timeout); reject(error); });
  child.once("close", (code) => {
    clearTimeout(timeout);
    resolve({ code: timedOut ? -1 : (code ?? -1), stdout, stderr, combined: stdout + stderr, truncated: false, timed_out: timedOut });
  });
});
console.log("backend patched, methods:", before, "->", Object.keys(B).length);


// ---- build a step element the way the real UI does ----
function makeStep(){ const s=document.createElement("div"); s.className="agent-tool-call";
  s.innerHTML=`<div class="agent-tool-row"><span class="atc-name"></span></div><div class="atc-viewport"></div><div class="atc-result"></div>`; return s; }
function newRun(){ return { mode:"agent", session:{}, _toolStep:0, _failedPathAttempts:new Map(), _readSeen:new Map(),
  _readSig:new Map(), _dupReadN:0, _reqId:"e2e", _toolLedger:[], _redactionMap:new Map(), _gitRepoHints:new Set(),
  _browserOpLog:[], _subAgentJobs:[], _scope:null, engineering:{},
  // Faithfulness, not convenience: the real loop always supplies these. Without
  // ctx.filesRead, _runHasCurrentRead short-circuits false on its 3rd line and EVERY
  // write to an existing file is blocked — a defect manufactured by the driver.
  ctx:{ filesRead:new Set() }, _readCoverage:new Map(), _knownCurrentContent:new Map(),
  _executionEvidence:[], _toolBatch:0, _eagerTurnBias:0 }; }
const run = newRun();
async function exec(call){
  const out = await H._executeToolStepInner(makeStep(), call, ROOT, run);
  // Replicate the production loop's POST-MUTATION accounting exactly enough for
  // direct executor calls. A miss is stored under its workspace-normalized
  // _toolFailureKey, not the raw relative path.
  if (out?.mutated) {
    for (const k of [call.path, out.path]) {
      if (!k) continue;
      const rawKey = String(k).trim();
      const failureKey = H._toolFailureKey({ type: "read", path: rawKey }, ROOT);
      run._failedPathAttempts.delete(rawKey);
      run._failedPathAttempts.delete(failureKey);
      H._resetToolFailure(failureKey);
    }
  }
  run._toolBatch = (run._toolBatch || 0) + 1;   // each exec() stands for one completed tool batch
  return out;
}
function show(tag,o){ const c=String(o?.content||"").replace(/\n/g," ").slice(0,110); console.log(tag.padEnd(34), "mutated="+(o?.mutated??"-"), "|", c); }

console.log("\n===== SCENARIO 1: write new file into empty dir =====");
show("write hello.txt", await exec({ type:"write", path:"hello.txt", content:"hello from the harness\n" }));
console.log("  DISK:", JSON.stringify(fs.readFileSync(join(ROOT,"hello.txt"),"utf8")));

console.log("\n===== SCENARIO 3: write the SAME path again =====");
show("write hello.txt (2nd)", await exec({ type:"write", path:"hello.txt", content:"second write\n" }));
console.log("  DISK:", JSON.stringify(fs.readFileSync(join(ROOT,"hello.txt"),"utf8")),
            "  => second write landed:", fs.readFileSync(join(ROOT,"hello.txt"),"utf8")==="second write\n");

console.log("\n===== SCENARIO 2: read -> edit -> read =====");
fs.writeFileSync(join(ROOT,"code.js"), "const a = 1;\nconst b = 2;\n");
show("read code.js", await exec({ type:"read", path:"code.js" }));
show("edit code.js", await exec({ type:"edit", path:"code.js", oldString:"const b = 2;", newString:"const b = 42;" }));
console.log("  DISK:", JSON.stringify(fs.readFileSync(join(ROOT,"code.js"),"utf8")));
show("read code.js again", await exec({ type:"read", path:"code.js" }));

console.log("\n===== SCENARIO 5: mkdir then read a file inside it =====");
show("mkdir sub", await exec({ type:"mkdir", path:"sub" }));
console.log("  dir exists:", fs.existsSync(join(ROOT,"sub")));
fs.writeFileSync(join(ROOT,"sub","in.txt"), "inside\n");
const r5 = await exec({ type:"read", path:"sub/in.txt" });
show("read sub/in.txt", r5);
console.log("  SKIPPED_EMPTY_WORKSPACE present:", /SKIPPED_EMPTY_WORKSPACE/.test(String(r5?.content||"")));

console.log("\n===== SCENARIO 7: write a >55k single line =====");
const big = "x".repeat(60000);
const r7 = await exec({ type:"write", path:"big.txt", content:big });
show("write big.txt (60k 1-line)", r7);
console.log("  DISK size:", fs.existsSync(join(ROOT,"big.txt")) ? fs.statSync(join(ROOT,"big.txt")).size : "MISSING");

console.log("\n===== SCENARIO 6: read missing path twice, then write it, then read =====");
show("read ghost.txt #1", await exec({ type:"read", path:"ghost.txt" }));
show("read ghost.txt #2", await exec({ type:"read", path:"ghost.txt" }));
show("write ghost.txt",   await exec({ type:"write", path:"ghost.txt", content:"now i exist\n" }));
const r6 = await exec({ type:"read", path:"ghost.txt" });
show("read ghost.txt #3", r6);
console.log("  negative cache cleared:", /now i exist/.test(String(r6?.content||"")));
assert.match(String(r6?.content || ""), /now i exist/, "a successful write must clear the normalized read miss key");

console.log("\n===== SCENARIO 8: OpenAPI local spec only exposes HTTP operations =====");
fs.writeFileSync(join(ROOT, "openapi.json"), JSON.stringify({
  openapi: "3.0.3",
  info: { title: "Harness API" },
  paths: {
    "/widgets": {
      parameters: [{ name: "trace", in: "header" }],
      get: { summary: "List widgets" },
      post: { summary: "Create widget" },
      "x-path-metadata": { owner: "platform" },
      "$ref": "#/components/pathItems/widgets"
    }
  }
}));
const r8 = await exec({ type:"openapi_parser", url:"openapi.json", outputFormat:"list" });
show("openapi_parser list", r8);
assert.match(String(r8?.content || ""), /GET \/widgets/);
assert.match(String(r8?.content || ""), /POST \/widgets/);
assert.doesNotMatch(String(r8?.content || ""), /X-PATH-METADATA/, "OpenAPI extensions are not HTTP methods");

console.log("\n===== SCENARIO 9: run_cmd captures real short-lived commands =====");
const cmdOk = await exec({ type:"cmd", command:"printf 'cmd-stdout-ok\n'", purpose:"verify" });
show("run_cmd stdout", cmdOk);
assert.equal(cmdOk.code, 0, "successful command keeps its exit code");
assert.match(String(cmdOk.stdout || ""), /cmd-stdout-ok/, "successful command returns stdout");

const cmdFail = await exec({ type:"cmd", command:"printf 'cmd-stderr-failure\n' >&2; exit 7", purpose:"verify" });
show("run_cmd stderr + exit 7", cmdFail);
assert.equal(cmdFail.code, 7, "failed command keeps its nonzero exit code");
assert.match(String(cmdFail.stderr || ""), /cmd-stderr-failure/, "failed command returns stderr");

const cmdCwd = await exec({ type:"cmd", command:"pwd; mkdir -p 'command dir'; printf 'created by command\n' > 'command dir/result file.txt'", purpose:"mutate" });
show("run_cmd cwd + spaced path", cmdCwd);
assert.equal(cmdCwd.code, 0, "workspace command exits successfully");
assert.ok(String(cmdCwd.stdout || "").includes(ROOT), "taskRunCapture runs in the supplied workspace cwd");
assert.equal(fs.readFileSync(join(ROOT, "command dir", "result file.txt"), "utf8"), "created by command\n",
  "quoted paths with spaces are passed to the real shell and mutate only the temporary workspace");

console.log("\nFINAL dir:", fs.readdirSync(ROOT).sort().join(", "));
process.exit(0);
