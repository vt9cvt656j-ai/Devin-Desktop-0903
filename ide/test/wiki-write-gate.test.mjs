import test from "node:test";
import { SRC as SHARED_SRC } from "./helpers/source.mjs";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { blockedInReadOnlyMode, needsApprovalFor, readOnlyBlockedTypes, approvalTypes } from "../src/agent/tool-policy.js";

/*
 * generate_wiki 会**真的写工作区文件**：路径由模型的 dest 参数给（默认 PRODUCT_WIKI.md，
 * 但传 "README.md" 就覆盖 README）。而这次落盘发生在主循环的结果处理里、不在工具执行器
 * 里——于是只读门和审批门从头到尾没被问过：Explorer / Plan / Reviewer 三个只读模式下它
 * 照样写盘，开着「改动前审批」时也一框不弹，而隔壁 write_file 写一个字节就要弹。
 *
 * tool-policy 那个模块的开篇注释写的就是这种事：「a tool left out of the read-only-mode
 * list is quietly executable in Explorer/Plan/Reviewer, and nothing fails until a user
 * notices」。这条测试就是那个 notice。
 */
// 源码文本用共享的那一份（helpers/source.mjs 的 SRC = main.js + src/agent/* 拼接）。
// 自己 readFileSync("src/main.js") 的话，每从 main.js 搬出一个模块就假红一次；
// 反方向更糟：「main.js 里不许出现 X」这类断言会在 X 搬进模块后恒绿，禁令悄悄失效。
const SRC = SHARED_SRC;

test("带 _wiki 的 subagent 调用要被只读模式挡下，纯调研的照常放行", () => {
  assert.equal(blockedInReadOnlyMode("subagent", { _wiki: true, wikiDest: "README.md" }), true,
    "只读模式下它还能把 README 覆盖掉");
  // 只读模式本来就靠这几个干活，绝不能一起挡掉。
  for (const call of [{}, { description: "调研" }, { _research: true }]) {
    assert.equal(blockedInReadOnlyMode("subagent", call), false,
      "把纯调研的 run_subagent / research_project 也挡了 —— 只读模式将无法工作");
  }
});

test("带 _wiki 的调用要过审批门，纯调研的不必弹框", () => {
  assert.equal(needsApprovalFor("subagent", { _wiki: true }), true, "写工作区文件却不弹框");
  assert.equal(needsApprovalFor("subagent", {}), false, "看一眼代码也弹框，那道门会被用户直接关掉");
});

test("按类型建的那两份集合不会因为函数值把纯调研也算进去", () => {
  // 这两个集合回答的是「这个工具**有可能**要审批 / 有可能被只读挡」，函数值在它们眼里
  // 恒为真。所以它们含 subagent 是对的，但判定必须走 *For 那两个按 call 判的函数。
  assert.ok(readOnlyBlockedTypes().has("subagent"));
  assert.ok(approvalTypes().has("subagent"));
});

test("检查点写在那次落盘前面，而不是只声明策略", () => {
  // 那条路径压根不经过工具执行器，光在 tool-policy 里声明是拦不住的。
  const i = SRC.indexOf("if (it.call._wiki && report");
  assert.ok(i > 0, "wiki 落盘那段不见了");
  const seg = SRC.slice(i, SRC.indexOf("it._wikiMutated = true", i));
  const iRo = seg.indexOf('blockedInReadOnlyMode("subagent", it.call)');
  const iAp = seg.indexOf("_approveToolCall(");
  const iWrite = seg.indexOf("_commitDiskTextIfUnchanged(");
  assert.ok(iRo > 0, "只读门没补 —— Explorer/Plan/Reviewer 下它照样写盘");
  assert.ok(iAp > 0, "审批门没补 —— 开着「改动前审批」也一框不弹");
  assert.ok(iRo < iWrite && iAp < iWrite, "门排在落盘后面，等于没有");
  // 拦下不等于白干：报告本身要照样交回去。
  assert.match(seg, /\*\*报告本身在下面，一个字都没少\*\*/,
    "拦下之后把报告也吞了 —— 模型辛苦跑完的调研全丢，它只会再跑一遍");
});
