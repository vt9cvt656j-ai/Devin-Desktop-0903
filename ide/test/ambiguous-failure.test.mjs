import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join, extname } from "node:path";

/*
 * 「失败」和「没有」回同一个值 —— 写完之后维护不动的最典型一种。
 *
 * 写的时候两条路都 return null 看着再自然不过；出事时从调用方往回查，什么线索都没有，
 * 因为**信息在 return 的那一刻就丢了**。本仓库今天一天里连撞四处（agentLocate /
 * agentHover / agentFormat / agentDocumentSymbols），后果是模型把「查询超时」读成
 * 「这个函数没人调用」，然后把它删了。
 *
 * 判据是量出来的，不是拍的：拿仓库自己 15.7 万行真代码跑，命中 34 处（每 4600 行一处），
 * 抽查 8 处有 6 处是真的。同一轮量过的另外三条判据都没过关 —— 最接近的一条（会改变
 * 结果的操作被空 catch 吞掉）在 27 万行上命中 2081 处，收紧到 7 处后逐个看仍有七成
 * 误报（那些空 catch 大多在别处补偿过），所以没有采用。
 */
const SRC = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");

function loadDetector() {
  const i = SRC.indexOf("function _ambiguousFailureInWrite(");
  assert.ok(i > 0, "检测器不见了");
  const tail = SRC.indexOf("  flush();\n  return out;", i);
  assert.ok(tail > i, "检测器的收尾不见了");
  return new Function(SRC.slice(i, SRC.indexOf("\n}\n", tail) + 2) + "\nreturn _ambiguousFailureInWrite;")();
}
const detect = loadDetector();

test("抓得住今天那个真形状：catch 回 null + 正常路径也回 null", () => {
  const hits = detect(`async function agentLocate(path, line, kind) {
  const ctx = await ensureDoc(path);
  if (!ctx) return null;
  let result;
  try { result = await ctx.client.request("textDocument/references", params); }
  catch { return null; }
  return toLocations(result);
}`);
  assert.equal(hits.length, 1, "没抓到 —— 这正是让模型把有调用方的函数删掉的那个形状");
  assert.equal(hits[0].name, "agentLocate", "没点名是哪个函数");
  assert.equal(hits[0].line, 6, "没指到 catch 回 null 那一行");
  assert.equal(hits[0].other, 3, "没指出另一条也回 null 的路径在哪一行");
});

test("修好之后的形状不再报（否则修完还被念，这机制就废了）", () => {
  assert.deepEqual(detect(`async function agentLocate(path) {
  const ctx = await ensureDoc(path);
  if (!ctx) return null;
  let r;
  try { r = await ctx.client.requestDetailed("x", p); }
  catch (e) { return { unanswered: true, reason: "transport" }; }
  if (!r.ok) return { unanswered: true, reason: r.reason };
  return r.result;
}`), []);
});

test("只有一条路回 null 的不报 —— 那不含糊", () => {
  assert.deepEqual(detect(`async function readCfg(p) {
  try { return JSON.parse(await read(p)); } catch { return null; }
}`), []);
  assert.deepEqual(detect(`function find(xs, k) {
  for (const x of xs) if (x.k === k) return x;
  return null;
}`), []);
});

test("catch 换行写的也要认（一行写完的和换行写的是同一件事）", () => {
  const hits = detect(`async function load(p) {
  if (!p) return null;
  try {
    return await read(p);
  } catch {
    return null;
  }
}`);
  assert.equal(hits.length, 1, "catch 换行写就漏了 —— 那是更常见的写法");
});

test("两个函数各自判断，不许串块", () => {
  // A 只有 catch 那一条，B 只有正常那一条 —— 任何一个都不该报。
  assert.deepEqual(detect(`function a(p) {
  try { return read(p); } catch { return null; }
}
function b(xs) {
  if (!xs.length) return null;
  return xs[0];
}`), [], "把两个函数串成一块了 —— 那会凭空造出误报");
});

test("上限收得住，不刷屏", () => {
  const one = `function f%%(p) {
  if (!p) return null;
  try { return read(p); } catch { return null; }
}
`;
  const many = Array.from({ length: 9 }, (_, k) => one.replace("%%", k)).join("");
  assert.equal(detect(many).length, 3, "没有上限，一次写入会被糊一屏");
});

test("在真语料上的命中密度要维持在可接受范围", () => {
  // 这条是**活的标定**：判据一旦放宽（比如把「只有 catch 回 null」也算上），
  // 密度会立刻塌下来，这里就红。仓库自己的真代码就是标尺。
  const files = [];
  const walk = (d) => { for (const e of readdirSync(d, { withFileTypes: true })) {
    if (e.name === "node_modules" || e.name.startsWith(".")) continue;
    const p = join(d, e.name);
    if (e.isDirectory()) walk(p); else if ([".js", ".mjs"].includes(extname(e.name))) files.push(p);
  } };
  walk(new URL("../src", import.meta.url).pathname);
  let hits = 0, lines = 0;
  for (const f of files) {
    const s = readFileSync(f, "utf8");
    lines += s.split("\n").length;
    hits += detect(s, 1e9).length;
  }
  const per = lines / Math.max(hits, 1);
  assert.ok(lines > 90000, `语料太小（${lines} 行），标定不作数`);
  assert.ok(per > 2000, `每 ${Math.round(per)} 行就命中一处，太吵了 —— 判据被放宽了`);
  assert.ok(hits > 5, `只命中 ${hits} 处，判据被收死了 —— 已知这个仓库里真有这种形状`);
});

test("挂在写入建议那个唯一出口上，不另开调用点", () => {
  const adv = SRC.slice(SRC.indexOf("function _sinkRiskAdvice(call)"), SRC.indexOf("function _sinkRiskAdvice(call)") + 2200);
  assert.match(adv, /_ambiguousFailureInWrite\(body\)/, "没挂上去 —— 检测器写了没人调");
  // 危险汇聚点没命中时不能提前 return，否则把这条也一起吞掉。
  assert.doesNotMatch(adv.slice(0, adv.indexOf("_ambiguousFailureInWrite")), /if \(!risks\.length\) return "";/,
    "sink 没命中就早退了 —— 可维护性这条永远发不出去");
  assert.match(adv, /给失败一个\*\*不同的形状\*\*/, "只说了有问题，没给具体改法");
  assert.match(adv, /出事时从调用方往回查，什么线索都没有/, "没说清为什么这是维护问题而不是洁癖");
});
