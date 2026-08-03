import { fileURLToPath as __f2u } from "node:url";
import { register } from "node:module";
import { pathToFileURL } from "node:url";
import * as fs from "node:fs";
import * as fsp from "node:fs/promises";
import { mkdtempSync } from "node:fs";
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
  // Replicate the agent loop's POST-MUTATION accounting. Driving the executor in isolation is
  // not the same as driving the product: several guarantees (clearing the negative path cache
  // after a successful write, advancing the tool batch) live in the loop, not the executor.
  // Omitting them makes the harness invent failures the user would never see — the exact way a
  // harness turns into a meaningless signal.
  if (out?.mutated) {
    for (const k of [call.path, out.path]) if (k) run._failedPathAttempts.delete(String(k).trim());
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

console.log("\nFINAL dir:", fs.readdirSync(ROOT).sort().join(", "));
process.exit(0);
