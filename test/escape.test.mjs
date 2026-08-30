import test from "node:test";
import assert from "node:assert/strict";
import { setBadgeText } from "../src/agent/escape.js";

test("状态徽章的文案必须包进 __t，并给长文案挂 title", () => {
  // .atc-result 是 flex 容器，它自己那条 text-overflow:ellipsis 对匿名文本项**恒不生效**。
  // 实测（浏览器里量的）：405px 的失败原因塞进 202px 的徽章，硬切、无省略号、无 tooltip，
  // 用户看到一句读不通的残句还不知道后面有内容。包进 .atc-result__t 才有正确规则。
  const mk = () => {
    const kids = [];
    return {
      _kids: kids, _title: undefined, textContent: "x",
      appendChild(c) { kids.push(c); },
      removeAttribute(n) { if (n === "title") this._title = undefined; },
      set title(v) { this._title = v; }, get title() { return this._title; },
      ownerDocument: { createElement: () => ({ className: "", textContent: "" }) },
    };
  };
  const long = "用户在 Agent 写入期间继续编辑；本次 Agent 写入已撤销，未覆盖用户缓冲区";
  const el = mk();
  assert.equal(setBadgeText(el, long), true);
  assert.equal(el.textContent, "", "裸文本没被清掉——省略号还是不会生效");
  assert.equal(el._kids.length, 1, "文案没被包进一个元素");
  assert.equal(el._kids[0].className, "atc-result__t", "包错了 class，省略号规则套不上");
  assert.equal(el._kids[0].textContent, long);
  assert.equal(el.title, long, "长文案没挂 title——被截掉的部分永远读不到");

  // 短文案不挂 title：每个徽章都带 tooltip 反而是噪音。
  const s = mk(); setBadgeText(s, "完成");
  assert.equal(s._kids[0].textContent, "完成");
  assert.equal(s.title, undefined, "短徽章不该挂 title");

  // null/undefined 要变成空串，不是字面量 "null"。
  const n = mk(); setBadgeText(n, null);
  assert.equal(n._kids[0].textContent, "", "null 渲染成了字面量");
  // 坏输入不抛：它跑在渲染路径上。
  for (const bad of [null, undefined, {}]) assert.doesNotThrow(() => setBadgeText(bad, "x"));
});
