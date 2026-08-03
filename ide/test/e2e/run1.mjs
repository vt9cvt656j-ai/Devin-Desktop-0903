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
function makeStep() {
  const step = document.createElement("div");
  step.className = "agent-tool-call";
  step.innerHTML = `<div class="agent-tool-row"><span class="atc-name"></span></div>` +
                   `<div class="atc-viewport"></div><div class="atc-result"></div>`;
  return step;
}
const step = makeStep();
console.log("step.querySelector('.atc-viewport'):", !!step.querySelector(".atc-viewport"),
            " .atc-result:", !!step.querySelector(".atc-result"),
            " .agent-tool-row:", !!step.querySelector(".agent-tool-row"));

const run = { mode:"agent", session:{}, _toolStep:0, _failedPathAttempts:new Map(), _readSeen:new Map(),
              _readSig:new Map(), _dupReadN:0, _reqId:"e2e-1", _toolLedger:[], _redactionMap:new Map(),
              _gitRepoHints:new Set(), _browserOpLog:[], _subAgentJobs:[], _scope:null, engineering:{} };
const call = { type:"write", path:"hello.txt", content:"hello from the harness\n" };

console.log("\n--- calling _executeToolStepInner ---");
let out;
try { out = await H._executeToolStepInner(step, call, ROOT, run); }
catch (e) { console.log("EXECUTOR THREW:", e.constructor.name, e.message); console.log((e.stack||"").split("\n").slice(0,8).join("\n")); process.exit(2); }
console.log("returned:", JSON.stringify(out)?.slice(0,600));
const target = join(ROOT, "hello.txt");
console.log("\nFILE ON DISK exists:", fs.existsSync(target));
if (fs.existsSync(target)) console.log("CONTENT:", JSON.stringify(fs.readFileSync(target,"utf8")));
console.log("dir listing:", fs.readdirSync(ROOT));
process.exit(0);
