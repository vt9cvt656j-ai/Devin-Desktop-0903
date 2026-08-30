import test from "node:test";
import assert from "node:assert/strict";
import { fnSource, SRC } from "./helpers/source.mjs";
import { facetSummary, preflightSettleLabel, preflightBody, movePreflightCardsAfter, attachFacetLine, createPreflightCard, settlePreflightCard }
  from "../src/agent/knowledge-preflight-card.js";

/**
 * 专业域小抄合成一张卡。
 *
 * 用户实拍：一轮里连出四张一模一样的「知识检索」，把第一个模型回合之前的整片视野占满。
 * 它们其实是同一次预检的四个面（适用条件/硬性约束/常见坑/必须做的检查）——一个操作。
 */

// 命中数**故意不按阅读顺序递减**（1/3/0/2）。写成 3/2/1/0 的话，「按命中数重排」这个
// 变异不会改变任何顺序，下面那条顺序断言就是恒真的——第一版就是这么写的，变异测试当场
// 抓住了它。测试数据本身要能把被测的那件事分开。
const S = [
  { heading: "适用条件", bullets: ["a"] },
  { heading: "硬性约束", bullets: ["b", "c", "d"] },
  { heading: "常见坑", bullets: [] },
  { heading: "必须做的检查", bullets: ["e", "f"] },
];

test("四个面的命中数直接写在卡面上，不用点开", () => {
  assert.equal(facetSummary(S), "适用条件 1 · 硬性约束 3 · 常见坑 0 · 必须做的检查 2");
  // 失败的那一面写「失败」，不写 0：零命中是一个结论（域里确实没这个主题），
  // 失败是没有结论（语料好端端摆着，只是没拿到）。写成 0 就把后者伪装成前者。
  const F = [{ heading: "常见坑", bullets: [], failed: true }, { heading: "硬性约束", bullets: ["x"] }];
  assert.equal(facetSummary(F), "常见坑 失败 · 硬性约束 1");
  assert.equal(preflightSettleLabel(F), "1 条 · 2 面 · 1 面失败", "部分失败还是要报拿到的条数");
  // 全失败 → 空串，交给 _knowledgeSettleLabel 去说「检索失败 · 原因」。
  assert.equal(preflightSettleLabel([{ heading: "a", bullets: [], failed: true }]), "",
    "全失败时没有交回给唯一那个区分失败/零命中的判据 —— 会显示成「无可用命中」，等于说库里没有");
  assert.equal(preflightSettleLabel(S), "6 条 · 4 面");
  // 全空时说「无可用命中」，别报一个骗人的 0——这条和 _knowledgeSettleLabel 同口径。
  assert.equal(preflightSettleLabel([{ heading: "x", bullets: [] }]), "无可用命中");
  assert.equal(preflightSettleLabel([]), "无可用命中");
});

test("展开的正文按 rubric 固定顺序排，空的那一面留着标题", () => {
  const body = preflightBody(S);
  const order = ["适用条件", "硬性约束", "常见坑", "必须做的检查"]
    .map((h) => body.indexOf(`【${h}】`));
  assert.ok(order.every((v, i) => v >= 0 && (i === 0 || v > order[i - 1])),
    "顺序被打乱了——「适用条件→硬性约束→常见坑→必须做的检查」本身是一条阅读线，不按命中数重排");
  assert.match(preflightBody([{ heading: "常见坑", bullets: [], failed: true }]),
    /【常见坑】（检索失败，不等于库里没有）/, "失败的那一面在正文里也被写成了零命中");
  assert.match(body, /【常见坑】（无可用命中）/,
    "空的那一面被静默丢掉了——「这一面没查到」和「这一面不存在」是两件事");
  assert.ok(preflightBody(S, 40).endsWith("…（已截断）"), "超长没截断");
});

test("坏输入一律不抛——它跑在渲染路径上", () => {
  for (const bad of [null, undefined, {}, [null], [{ bullets: null }], "x"]) {
    assert.doesNotThrow(() => facetSummary(bad));
    assert.doesNotThrow(() => preflightSettleLabel(bad));
    assert.doesNotThrow(() => preflightBody(bad));
  }
});

test("预检卡挪到思考卡后面，且只挪在它前面的那些", () => {
  // 极简 DOM 替身：只实现这个函数真正用到的四件事。
  const mk = (tag = "div") => {
    const el = { tag, children: [], parentNode: null, dataset: {},
      insertBefore(node, ref) {
        if (node.parentNode) node.parentNode.children.splice(node.parentNode.children.indexOf(node), 1);
        const i = ref ? this.children.indexOf(ref) : this.children.length;
        this.children.splice(i < 0 ? this.children.length : i, 0, node);
        node.parentNode = this;
      },
      append(node) { this.insertBefore(node, null); },
      get nextSibling() {
        const c = this.parentNode?.children || []; return c[c.indexOf(this) + 1] || null;
      },
      querySelectorAll(sel) {
        assert.equal(sel, '[data-knowledge-preflight="1"]');
        return this.children.filter((c) => c.dataset.knowledgePreflight === "1");
      },
      compareDocumentPosition(other) {
        const c = this.parentNode?.children || [];
        return c.indexOf(other) < c.indexOf(this) ? 2 : 4;   // 2 = 在我之前
      } };
    return el;
  };
  const body = mk();
  const k1 = mk(); k1.dataset.knowledgePreflight = "1"; k1.id = "k1";
  const k2 = mk(); k2.dataset.knowledgePreflight = "1"; k2.id = "k2";
  const think = mk(); think.id = "think";
  const later = mk(); later.dataset.knowledgePreflight = "1"; later.id = "later";
  for (const n of [k1, k2, think, later]) body.append(n);

  assert.equal(movePreflightCardsAfter(body, think), 2, "在思考卡之前的两张没被挪");
  assert.deepEqual(body.children.map((c) => c.id), ["think", "k1", "k2", "later"],
    "顺序不对：两张要挪到思考卡后面且保持它们原来的相对顺序；思考卡之后的那张不许再挪");
  // 幂等：再挪一次不该变化（都已经在 anchor 之后了）。
  assert.equal(movePreflightCardsAfter(body, think), 0);
  assert.deepEqual(body.children.map((c) => c.id), ["think", "k1", "k2", "later"]);
  // 坏输入不抛。
  for (const [b, a] of [[null, think], [body, null], [body, {}], [undefined, undefined]]) {
    assert.doesNotThrow(() => movePreflightCardsAfter(b, a));
  }
});

test("调用点：一个域只建一张卡，且思考卡出现时会挪", () => {
  const pre = fnSource("_runDomainKnowledgePreflight", { code: true })
    || SRC.slice(SRC.indexOf("四面预检") - 3000, SRC.indexOf("四面预检") + 3000);
  assert.match(pre, /_createPreflightCard\(body, domain, _createToolStep\)/, "域级那张卡没了");
  assert.match(pre, /_settlePreflightCard\(_groupStep, sections/, "组卡没结算");
  // 建卡时必须打标记，否则挪位置那一步找不到它。判据在模块里，做真往返。
  const made = createPreflightCard(
    { appendChild() {} },
    "ui-ux",
    (call) => ({ _call: call, dataset: {} }),
  );
  assert.equal(made?.dataset?.knowledgePreflight, "1", "卡没打标记");
  assert.match(String(made?._call?.query || ""), /四面预检/);
  assert.equal(createPreflightCard(null, "x", () => ({})), null);
  assert.equal(createPreflightCard({ appendChild() {} }, "x", null), null);
  // 四条 rubric 各自不许再建卡（那正是「一轮连出四张」的成因）。
  assert.doesNotMatch(pre, /step = _createToolStep\(call\)/, "每条 rubric 又各建一张卡了");
  assert.match(SRC, /_movePreflightCardsAfter\(body, reasoningEl\)/,
    "思考卡出现时没有把预检卡挪下去");
  // 四个面必须摆在**卡面上**。要点开才看得见的话，这次合并就变成了 app.css 里
  // 记着的那个被删掉的抽屉：「把内容藏到第二个地方让用户再去翻」。
  // 四个面必须摆在卡面上（不是点开才见）——真往返验它。
  let settled = null;
  const okStep = { querySelector: (q) => (q === ".atc-viewport" ? vp : q === ".atc-action-row" ? row : null),
    ownerDocument: { createElement: () => ({ set className(v) { this._c = v; }, set innerHTML(v) { this._h = v; } }) } };
  const vp = { textContent: "" };
  const row = { parentNode: { insertBefore: (el) => { row._line = el._h; } } };
  settlePreflightCard(okStep, S, { settleToolStep: (_s, _r, l) => { settled = l; },
    knowledgeSettleLabel: (_c, _r, l) => l || "检索失败", escapeHtml: (x) => x });
  assert.match(String(row._line || ""), /适用条件 1/, "四个面没摆到卡面上");
  assert.equal(settled, "6 条 · 4 面");
  assert.match(vp.textContent, /【适用条件】/, "正文没写进 viewport");
  // 全失败 → 交回 knowledgeSettleLabel。
  settlePreflightCard(okStep, [{ heading: "a", bullets: [], failed: true }],
    { settleToolStep: (_s, _r, l) => { settled = l; }, knowledgeSettleLabel: () => "检索失败 · 超时", escapeHtml: (x) => x });
  assert.equal(settled, "检索失败 · 超时", "全失败没走那个唯一区分失败/零命中的判据");
  for (const bad of [null, {}]) assert.doesNotThrow(() => settlePreflightCard(bad, S, {}));
  for (const bad of [null, {}]) assert.doesNotThrow(() => attachFacetLine(bad, "x", null));
});
