import { readFileSync } from "node:fs";
import * as acorn from "/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/node_modules/acorn/dist/acorn.mjs";
const src = readFileSync(join(__IDE, "src", "main.js"),"utf8");
const ast = acorn.parse(src, { ecmaVersion:"latest", sourceType:"module", locations:true, allowAwaitOutsideFunction:true });
let fn=null;
for (const n of ast.body) {
  const d = n.type==="ExportNamedDeclaration" ? n.declaration : n;
  if (d && d.type==="FunctionDeclaration" && d.id?.name==="_executeToolStepInner") fn=d;
}
if(!fn){ console.log("NOT FOUND at top level"); process.exit(1); }
console.log("signature: async=",fn.async, "params=", fn.params.map(p=>p.name||p.type).join(", "));
console.log("lines", fn.loc.start.line, "->", fn.loc.end.line, " (", fn.loc.end.line-fn.loc.start.line, "lines )");
const body = src.slice(fn.start, fn.end);
// member accesses on each param
for (const p of ["step","call","root","run"]) {
  const re = new RegExp("\\b"+p+"\\s*(?:\\?\\.|\\.)\\s*([A-Za-z_$][\\w$]*)","g");
  const s = new Map(); let m;
  while((m=re.exec(body))) s.set(m[1], (s.get(m[1])||0)+1);
  console.log("\n== "+p+" ("+s.size+" distinct) ==");
  console.log([...s.entries()].sort((a,b)=>b[1]-a[1]).map(([k,v])=>k+"("+v+")").join(" "));
}
console.log("\n== step.querySelector selectors ==");
const sel = new Set(); let m2;
const re2=/step\s*\??\.\s*querySelector(?:All)?\s*\(\s*(['"`])([^'"`]*)\1/g;
while((m2=re2.exec(body))) sel.add(m2[2]);
console.log([...sel].join("\n"));
console.log("\n== call.name string comparisons (tool names dispatched) ==");
const names=new Set(); let m3;
const re3=/(?:name|tool|t)\s*===?\s*"([a-z_0-9]+)"/g;
while((m3=re3.exec(body))) names.add(m3[1]);
console.log([...names].sort().join(" "));
