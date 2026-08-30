import test from "node:test";
import assert from "node:assert/strict";
import { domainKnowledgeBrief, domainKnowledgeBullets } from "../src/agent/domain-knowledge-brief.js";
import { facetSummary, preflightSettleLabel } from "../src/agent/knowledge-preflight-card.js";

const S = (o) => ({ heading: "常见坑", bullets: [], ...o });

test("一次检索有三种结局，喂给模型的小抄必须说成三句不同的话", () => {
  // ① 检索失败 = **没有结论**（语料好端端摆着，只是这次没拿到）
  // ② 真零命中 = 有结论（这个域确实没这个主题，可以据此往下走）
  // ③ 命中 N 段但抽取器一条要点没压出来 = 也不是「没有」，是我们自己筛干净的
  // 原来三种全落到同一句「本域语料里没有」。①被说成②的后果最重：四条 rubric 全超时时
  // 模型收到「该领域语料不可用，不要编造该领域的规则」，据此判定这个域没有知识可用——
  // 而卡片 UI 那侧显示的是「检索失败 · 请求超时」，用户和模型看到的结论互相矛盾。
  const failed = domainKnowledgeBrief("healthcare", [S({ failed: true }), S({ failed: true })]);
  const zero   = domainKnowledgeBrief("healthcare", [S({ hits: 0 }), S({ hits: 0 })]);
  const dry    = domainKnowledgeBrief("healthcare", [S({ hits: 4 }), S({ hits: 2 })]);

  assert.match(failed, /没有拿到结果/, "全失败没说清是「没拿到」");
  assert.match(failed, /不等于.*库里没有|不\*\*等于\*\*库里没有/, "全失败必须点明它不等于库里没有");
  assert.doesNotMatch(failed, /语料不可用/, "全失败仍在说「语料不可用」——那是把①说成了②");

  assert.match(zero, /确实没有匹配/, "真零命中没给出「确实没有」这个结论");
  assert.match(dry, /没能压出可用要点/, "第三态没说清是抽不出，不是没有");
  assert.doesNotMatch(dry, /确实没有匹配/, "第三态被说成了真零命中");

  // 三句必须互不相同——只要有两种压成同一句，这条修复就白做了。
  assert.notEqual(failed, zero);
  assert.notEqual(zero, dry);
  assert.notEqual(failed, dry);
});

test("部分失败时，空栏要标明「这栏失败了」而不是「本域没有」", () => {
  const mixed = domainKnowledgeBrief("db", [
    { heading: "适用条件", bullets: ["用 Postgres"], hits: 3 },
    S({ heading: "常见坑", failed: true }),
  ]);
  assert.match(mixed, /适用条件/);
  assert.match(mixed, /这一栏这次\*\*没查到（检索失败）\*\*|没查到（检索失败）/,
    "失败的那一栏被写成了「本域语料里这一栏没有独立内容」");
});

test("卡面和结算标签同样要分开这三态", () => {
  assert.equal(facetSummary([S({ hits: 0 })]), "常见坑 0", "真零命中");
  assert.equal(facetSummary([S({ hits: 4 })]), "常见坑 0/4 段", "命中但没压出要点，写成裸 0 就和真零命中一样了");
  assert.equal(facetSummary([S({ failed: true })]), "常见坑 失败");
  assert.equal(preflightSettleLabel([S({ hits: 0 })]), "无可用命中");
  assert.equal(preflightSettleLabel([S({ hits: 4 })]), "命中 4 段 · 未压出要点");
  assert.equal(preflightSettleLabel([S({ failed: true })]), "", "全失败要返回空串交给唯一的失败判据");
});

test("抽要点：标题行/表格骨架/过短行照旧被丢掉（搬模块没改行为）", () => {
  const block = "【domain · Rate Limiting】\n## Rate Limiting\n|---|---|\n| 列 | 名 |\n短\n"
    + "Rate limiting must be applied per-tenant, not per-IP, or one noisy tenant starves the rest.";
  const got = domainKnowledgeBullets(block);
  assert.equal(got.length, 1, `应当只压出那一条真事实，实际: ${JSON.stringify(got)}`);
  assert.match(got[0], /per-tenant/);
  assert.doesNotMatch(got[0], /^Rate Limiting$/, "小节标题被当成了要点");
});

test("坏输入不抛——它在每轮的提示词拼装路径上", () => {
  for (const bad of [null, undefined, [], [null]]) {
    assert.doesNotThrow(() => domainKnowledgeBrief("d", bad));
  }
  for (const bad of [null, undefined, ""]) assert.doesNotThrow(() => domainKnowledgeBullets(bad));
});
