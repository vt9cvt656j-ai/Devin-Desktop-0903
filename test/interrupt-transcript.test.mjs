// 中断/停止时的**成绩单**：显示侧和状态侧各归各的。
//
// 用户实拍：一轮跑到一半被中断，聊天记录末尾多出一条
//   〔中断前的写入结果〕本次运行已经真实写入磁盘：…server.py、…session.py
// 这条是 harness 写的字，被当成模型正文拼进了可见消息。用户的原话：
//   「即使中断也要显示完整的聊天记录，而不是就说中断了然后 xxx 了，这样是不正确的。」
//
// 定的两条性质：
//   1. 显示侧：中断那一轮的**真实流式正文**（turn.text）照旧先入账再 break，一个字不丢；
//      而那条〔中断前的写入结果〕页脚**不再**拼进可见消息（summaryText）。
//   2. 状态侧：「中断前改了哪些文件」这件事没丢——它记成状态（run._breakWriteFact →
//      _lastRunState.breakWriteFact → 续跑时的 run._resumeFact），照样喂给模型，
//      让它「别重写已落盘的文件」。删显示不等于砍掉模型的通道。
//
// 中断路径深埋在 _runAgenticLoop 里、没法在 Node 里真跑，所以这里按源码锚点断言
// （与 prefix-cache.test.mjs 里那组同源）。锚点尽量挑实现特征，不挑说明词。
import test from "node:test";
import assert from "node:assert/strict";
import { SRC } from "./helpers/source.mjs";

test("中断/停止：真实正文仍先入账再 break（完整聊天记录不丢）", () => {
  // 用户点停止那一轮：turn.text 必须在 break **之前**收进 run 摘要。
  assert.match(SRC,
    /if \(!_live\(\)\) \{[\s\S]{0,900}summaryText \+= \(summaryText \? "\\n\\n" : ""\) \+ turn\.text\.trim\(\);[\s\S]{0,900}break;/,
    "按停止那一轮的真实正文没有先入账——中断后聊天记录里会少掉这段");
});

test("〔中断前的写入结果〕页脚不再拼进可见消息", () => {
  // 关键回归闸：那条页脚是 _settleEagerWritesForBreak 造的，以前三处 break/插话都把它
  // `summaryText += … + _eagerNote` 进可见消息。现在一处都不许有。
  assert.doesNotMatch(SRC, /summaryText \+= \(summaryText \? "\\n\\n" : ""\) \+ _eagerNote/,
    "又把〔中断前的写入结果〕页脚拼回可见消息了——它是 harness 写的字，不该冒充模型正文");
  // 更广的闸：任何给 summaryText 赋值的行都不许带上写盘页脚（不管换成 _breakWriteFact
  // 还是直接拼字面量）。summaryText 是可见/落库的助手消息，页脚只能进状态。
  const badLine = SRC.split("\n").find((ln) =>
    /summaryText\s*(\+?=)/.test(ln) && /_eagerNote|_breakWriteFact|中断前的写入结果/.test(ln));
  assert.equal(badLine, undefined,
    `有一行把写盘页脚拼进了可见消息 summaryText：${badLine || ""}`);
});

test("写盘事实改记成状态：run._breakWriteFact（不是可见正文）", () => {
  // 三处 break/插话结算点都把 note 挂到 run._breakWriteFact 上。
  assert.ok((SRC.match(/if \(_eagerNote\) run\._breakWriteFact = _eagerNote;/g) || []).length >= 3,
    "三处结算点没有都把写盘事实记成 run._breakWriteFact 状态");
  // 结算调用本身要留着：它得等在途的「流完即写」落定（带 8s 上限），这条不能被顺手删掉。
  assert.ok((SRC.match(/await _settleEagerWritesForBreak\(run\)/g) || []).length >= 3,
    "结算在途写入的调用被删了——落了盘的文件会既不入账也等不到落定");
});

test("状态只在真没跑完时留存，正常收尾（哪怕插过话）不留", () => {
  // _lastRunState.breakWriteFact 用「本轮确实没跑完」门控，否则新任务会莫名收到
  // 「上一轮中断前已落盘」。门控用的就是收尾判据里现成的那三个信号。
  assert.match(SRC,
    /breakWriteFact: \(finalErr \|\| _stoppedEarly \|\| run\._incompleteReason\)[\s\S]{0,160}run\._breakWriteFact/,
    "_lastRunState.breakWriteFact 没有按「确实没跑完」门控，正常收尾也会误带写盘事实");
});

test("续跑时写盘事实照样喂给模型（删了显示没砍掉模型通道）", () => {
  // 同会话「停止→继续」走不到崩溃重启那条 _wfInterrupted，所以另补一条 run._resumeFact，
  // 从 _lastRunState.breakWriteFact 取，走每轮必注入的〔执行状态〕。crash 路已置则不覆盖。
  const seed = SRC.slice(SRC.indexOf("session?._lastRunState?.breakWriteFact"));
  assert.ok(seed, "找不到同会话续跑的写盘事实种子——续跑时模型会不知道哪些文件已落盘");
  assert.match(seed.slice(0, 400), /run\._resumeFact = /,
    "写盘事实没有接到 run._resumeFact 这条每轮必注入的通道上");
  assert.match(seed.slice(0, 400), /不要重做|别重写|不要整份重写/,
    "续跑提示没有告诉模型「别重写已落盘文件」");
  // 不能盖掉崩溃重启那条（那条更准，带步号）。
  assert.match(SRC, /!run\._resumeFact && session\?\._lastRunState\?\.breakWriteFact/,
    "同会话续跑的种子没有让崩溃重启那条优先");
});
