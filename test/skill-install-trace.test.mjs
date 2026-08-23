// 「点击安装后就这样了」——用户原话。
//
// 实测（2026-08-23，用户机器）：~/.mrdayone/skills 目录建于当天 12:04（说明市场面板确实
// 跑过），里面一个技能都没有。而整条安装链路逐项实测**全是通的**：
//   · api.github.com / raw.githubusercontent.com 都 200，约 1 秒；
//   · anthropics/skills 仓库树 509 个节点拉得到，20 个技能带 SKILL.md；
//   · 那 20 个技能共 356 个文件，全部通过 Rust 侧的路径白名单；
//   · GitHub 匿名配额 59/60，没撞限流；市场探测走的是 CDN，本来就不吃配额；
//   · skills_dir / skills_write_at 都会 create_dir_all；按钮接线也正确。
//
// 缺的不是能力，是**这一次到底发生了什么没被记下来**：toast 一闪而过，磁盘上没东西，
// 面板上没标记。这个文件守两件事——失败要留痕、单个附属文件不许把整包作废。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, load, loadConst, fnSource as topLevelFn } from "./helpers/source.mjs";

function ledger() {
  const bag = new Map();
  const storage = {
    getItem: (k) => (bag.has(k) ? bag.get(k) : null),
    setItem: (k, v) => bag.set(k, String(v)),
  };
  const deps = {
    localStorage: storage,
    _SKILL_INSTALL_LOG_KEY: loadConst("_SKILL_INSTALL_LOG_KEY"),
    _SKILL_INSTALL_LOG_MAX: loadConst("_SKILL_INSTALL_LOG_MAX"),
    Date: { now: () => 1_700_000_000_000 },
  };
  return {
    append: load("_skillInstallLogAppend", deps),
    lastFail: load("_skillLastInstallFailure", deps),
    rows: () => JSON.parse(bag.get(deps._SKILL_INSTALL_LOG_KEY) || "[]"),
  };
}

// ── 一、失败要留痕 ──────────────────────────────────────────────────────
test("失败被记下来，而且能取回最近一次", () => {
  const L = ledger();
  L.append({ ok: false, tag: "o:skills/canvas-design", error: "SKILL.md 没能落盘：404" });
  const last = L.lastFail();
  assert.ok(last, "失败没留下任何痕迹——那正是「点了安装就这样了」");
  assert.match(String(last.error), /404/);
  assert.ok(Number.isFinite(last.at), "没记时间，用户无法判断是不是刚才那次");
});

test("成功之后，之前那条失败不再顶在面板上", () => {
  // 但历史仍在台账里——面板只显示"最近一次失败"，判据是**最近**，不是"有没有过"。
  const L = ledger();
  L.append({ ok: false, tag: "a", error: "旧的失败" });
  L.append({ ok: true, tag: "a", files: 12 });
  assert.equal(L.lastFail(), null, "装成功了还在报旧错误，用户会以为一直没成");
});

test("多次失败取最新那条", () => {
  const L = ledger();
  L.append({ ok: false, tag: "a", error: "第一次" });
  L.append({ ok: false, tag: "b", error: "第二次" });
  assert.match(String(L.lastFail().error), /第二次/);
});

test("台账有上限，不会无限涨", () => {
  const L = ledger();
  const max = loadConst("_SKILL_INSTALL_LOG_MAX");
  for (let i = 0; i < max + 20; i++) L.append({ ok: false, tag: `t${i}`, error: `e${i}` });
  // 断言**长度**。只断言"最新那条还在"是绿的摆设——没有上限时它当然也在。
  assert.equal(L.rows().length, max, `台账涨到 ${L.rows().length} 条，上限没生效`);
  assert.match(String(L.lastFail().error), new RegExp(`e${max + 19}`), "最新那条被自己挤掉了");
});

test("台账坏掉时安静退回，绝不把安装面板带崩", () => {
  const bag = new Map();
  const storage = { getItem: () => "{ 这不是 JSON", setItem: (k, v) => bag.set(k, v) };
  const deps = {
    localStorage: storage,
    _SKILL_INSTALL_LOG_KEY: loadConst("_SKILL_INSTALL_LOG_KEY"),
    _SKILL_INSTALL_LOG_MAX: loadConst("_SKILL_INSTALL_LOG_MAX"),
    Date: { now: () => 1 },
  };
  assert.equal(load("_skillLastInstallFailure", deps)(), null);
  // 只断言"不许抛"是不够的：外层还有一层 try，把「这一条根本没写进去」也一起吞掉。
  // 要量的是**坏数据之后还能不能继续记账**——否则一次坏数据就让台账永久失效。
  const bag2 = new Map();
  let broken = true;
  const healing = {
    getItem: (k) => (broken ? "{ 这不是 JSON" : (bag2.has(k) ? bag2.get(k) : null)),
    setItem: (k, v) => { bag2.set(k, String(v)); broken = false; },
  };
  const d2 = { ...deps, localStorage: healing };
  load("_skillInstallLogAppend", d2)({ ok: false, tag: "t", error: "坏数据之后这条" });
  const back = load("_skillLastInstallFailure", d2)();
  assert.ok(back, "坏数据之后就再也记不进新的了——台账等于永久失效");
  assert.match(String(back.error), /坏数据之后这条/);
});

test("台账只记结果与原因，不记文件正文", () => {
  const body = topLevelFn("_skillInstallLogAppend", { code: true });
  assert.doesNotMatch(body, /\btext\b|content|body/,
    "把文件正文写进 localStorage 了——台账只该记「装了什么、为什么没成」");
});

// ── 二、接线：记账和显示都要真的发生 ────────────────────────────────────
test("成功和失败两条路都记账，且失败先记账再弹 toast", () => {
  // toast 会消失，台账不会。顺序反了的话，抛在记账之前就什么都不剩。
  const at = SRC.indexOf("const r = await run();");
  assert.ok(at > 0, "安装入口不见了");
  const seg = SRC.slice(at, at + 1200);
  assert.match(seg, /_skillInstallLogAppend\(\{ ok: true/, "成功没记账");
  assert.match(seg, /_skillInstallLogAppend\(\{ ok: false/, "失败没记账——那就还是「什么都不留」");
  const failLog = seg.indexOf('_skillInstallLogAppend({ ok: false');
  const failToast = seg.indexOf('showToast("安装失败');
  assert.ok(failLog > 0 && failLog < failToast, "记账要在弹 toast 之前");
});

test("最近一次失败常驻在面板上，不只是一闪而过", () => {
  // 要钉**调用点**：只写 /_skillLastInstallFailure\(\)/ 会被它自己的**定义行**喂绿，
  // 把调用改成 `const _lastFail = null` 照样匹配得到（2026-08-23 变异实测）。
  assert.match(SRC, /const _lastFail = _skillLastInstallFailure\(\);/,
    "面板没读台账——失败仍然只有 toast 一条痕迹");
  assert.match(SRC, /failEl\.hidden = false;/, "读了却没显示出来");
  assert.match(SRC, /data-skfp-lastfail/, "面板上没有承载它的节点");
});

// ── 三、单个附属文件不许把整包作废 ──────────────────────────────────────
test("SKILL.md 落不了盘是硬失败，且说清原因", () => {
  const body = topLevelFn("_skillInstallDir", { code: true });
  assert.match(body, /if \(rel === "SKILL\.md"\) throw new Error\(/,
    "SKILL.md 没落盘却当成装好了——那个技能是什么就无从谈起");
  assert.match(body, /SKILL\.md 没能落盘/, "硬失败没带上原因，用户还是不知道为什么");
});

test("附属文件抓不到只记一笔，整包照装", () => {
  const body = topLevelFn("_skillInstallDir", { code: true });
  assert.match(body, /skipped\.push\(\{ rel, why \}\)/,
    "附属文件一失败就把整包抛掉了——一个临时 404 就能让整次安装什么都不留");
  assert.match(body, /catch \(err\)/, "循环里没有逐文件的容错");
  assert.match(body, /return \{ destBase, fileCount: n, skipped,/, "缺了哪些文件没带出去");
});

test("缺文件要如实报给用户，不能装作整包都在", () => {
  const at = SRC.indexOf("const r = await run();");
  const seg = SRC.slice(at, at + 1200);
  assert.match(seg, /r\.skipped/, "没把缺的文件报出来——用户会以为整包都装上了");
  assert.match(seg, /没抓到/, "缺文件的提示没写清楚");
});

test("红线：安装台账不参与任何权限判断", () => {
  // 它只是可观测性。一旦被拿去当"这个技能可不可信"的判据，就成了一条能被写入影响的门。
  const body = topLevelFn("_skillLastInstallFailure", { code: true });
  assert.doesNotMatch(body, /approve|allow|trust|permission/i);
});
