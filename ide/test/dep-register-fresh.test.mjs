// 依赖事实通道：解析上限 6 个、每轮预算只说 2 个，而登记表把 **6 个全部**登记成
// 「项目已声明」。第 3~6 位于是两头落空：既没被提醒过，又因为"已声明"把 import 那条
// 后备通道一起堵死了。
//
// 而那几位恰恰是最会因版本漂移写错 API 的新依赖——头部框架（react / react-dom / vite）
// 按 manifest 行序排在前面，先把 2 个预算用掉了。讽刺的是第 7 位往后**压根没被登记**，
// 反而能被 import 腿抓住，还带 package_source 的零网络真签名。
import { test } from "node:test";
import assert from "node:assert/strict";
import { load, fnSource as topLevelFn } from "./helpers/source.mjs";

// 注入方式照抄 dep-pitfalls：自动补依赖会把 _manifestDepAdditions 也打成桩，
// 那样解析出来永远是空的，测的就不是这条链了。
const storage = () => {
  const store = new Map();
  return { getItem: (k) => (store.has(k) ? store.get(k) : null), setItem: (k, v) => { store.set(k, String(v)); } };
};
const noteFn = () => load("_depPitfallNote", {
  _manifestDepAdditions: load("_manifestDepAdditions", {
    _manifestDepKind: load("_manifestDepKind"),
    _depRegistryUrl: load("_depRegistryUrl"),
  }),
  _depSeenTouch: load("_depSeenTouch", {
    localStorage: storage(), _DEP_SEEN_LS_KEY: "michael-ide.dep-register-fresh-test", _DEP_SEEN_MAX: 64,
  }),
  _undeclaredImportAdditions: () => [],
});

const PKG = (names) => JSON.stringify({
  dependencies: Object.fromEntries(names.map((n) => [n, "^1.0.0"])),
}, null, 2);

test("登记表只记本轮真正说出口的，不记被预算饿死的", () => {
  const note = noteFn();
  const run = {};
  const names = ["react", "react-dom", "yjs", "y-websocket", "socket.io", "socket.io-client"];
  note(run, "/p/package.json", "{}", PKG(names));
  const reg = run._declaredDeps;
  assert.ok(reg instanceof Set, "登记表没建起来");
  // 预算是 2：只有前两个说出口，也只有它们该被登记。
  const registered = names.filter((n) => reg.has(`npm:${n}`));
  assert.equal(registered.length, 2,
    `登记了 ${registered.length} 个（${registered.join(",")}）——被饿死的那几位也被登记成`
    + "「已声明」，于是 import 那条后备通道对它们也闭嘴了");
  assert.deepEqual(registered, ["react", "react-dom"]);
});

test("没说出口的包，import 腿仍然能抓住它", () => {
  // 这是上一条的正面后果：第 3~6 位仍然有一条通道。
  const note = noteFn();
  const run = {};
  note(run, "/p/package.json", "{}", PKG(["react", "react-dom", "yjs", "socket.io"]));
  const reg = run._declaredDeps;
  assert.ok(!reg.has("npm:yjs"), "yjs 没说出口却被登记成已声明——它的后备通道被堵死了");
  assert.ok(!reg.has("npm:socket.io"), "同上");
});

test("说出口的包不会被 import 腿再报一遍（原契约没丢）", () => {
  // 登记表存在的理由：模型刚写进 manifest 的包，工作区扫描不知道，
  // 不登记的话紧接着写的 import 会被当成未声明依赖再报一遍。
  const note = noteFn();
  const run = {};
  note(run, "/p/package.json", "{}", PKG(["react", "react-dom"]));
  assert.ok(run._declaredDeps.has("npm:react"), "说出口的包没被登记——它会被重复提醒");
  assert.ok(run._declaredDeps.has("npm:"), "裸生态前缀没登记");
});

test("import 触发的那条路不写登记表", () => {
  // 登记表的语义是「模型往 manifest 里写了这个依赖」。import 腿看到的是「用了但没声明」，
  // 那正好相反，写进去会把真问题掩盖掉。
  const body = topLevelFn("_depPitfallNote", { code: true });
  assert.match(body, /if \(fromManifest && fresh\.length\)/,
    "登记不再区分是哪条腿触发的——import 腿会把「用了但没声明」登记成「已声明」");
});

test("登记发生在预算结算之后", () => {
  const body = topLevelFn("_depPitfallNote", { code: true });
  const regAt = body.indexOf("reg.add(`${d.kind}:`)");
  const freshAt = body.indexOf("fresh.push(d)");
  assert.ok(regAt > 0 && freshAt > 0, "两处锚点都要在");
  assert.ok(freshAt < regAt,
    "登记又跑到预算结算之前了——那样它记的还是 adds 而不是 fresh");
});

test("登记的是 fresh 不是 adds", () => {
  const body = topLevelFn("_depPitfallNote", { code: true });
  assert.match(body, /for \(const d of fresh\) \{ reg\.add/,
    "登记循环遍历的还是 adds——被饿死的那几位又会被登记");
});
