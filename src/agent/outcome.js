/**
 * 一次 agent 运行的**结局判定**。
 *
 * # 为什么抠成一个模块
 *
 * 这段判定原来是主循环里的一串三元表达式。任何关于「什么时候算做完了」的测试就只能
 * 拿正则去匹配源码文本——而本仓库有过「断言真实却守错了东西」的先例：源码长这样不代表
 * 跑起来是这样。抠成纯函数之后，结局判定可以在 Node 里**真往返**，变异测试打得红。
 *
 * 判据全部来自调用方算好的执行事实（落盘账本、退出码、评审结论），这里不读任何全局、
 * 不碰 DOM、没有模块级状态——所以它该住在这儿，也满足 main.js 尺寸闸对「能搬就搬」的判据。
 */

/**
 * 这一轮为什么不算完成。空串 = 完成。
 *
 * 分支顺序是**先到者胜**，和它搬出来之前一字不差：先到的那个才是真正促成 partial 的原因，
 * 顺序变了记下来的成因就会漂。
 */
export function partialCause(f = {}) {
  if (f.stoppedEarly) return "stopped_early";
  if (f.incompleteReason) return String(f.incompleteReason).slice(0, 60);
  if (f.hitCap) return "iteration_limit";
  if (f.didMutate && !f.verificationPassed && !f.nothingToVerify) return "unverified_change";
  if (f.didMutate && !f.uiVerificationPassed) return "unverified_ui";

  // ── 收尾评审的否决权 ───────────────────────────────────────────────────
  //
  // 收尾评审是一次**付费**调用：它拿到真实 diff、执行证据和验收契约，判断这一轮到底有没有
  // 实现用户要的东西。它的结论此前**只进一张建议卡**，对结局没有任何影响——评审说
  // 「登录接口仍然写死返回 true」，outcome 照样 success。
  //
  // 实测代价（用户机器 1217 条真实运行）：明确让它造/改东西的 273 条里，114 条一个文件
  // 都没碰，其中 **76 条被记成 success**。而 success 是学习回路的入口——只有非 success
  // 进纠错通道，只有 success 会被归纳成「工作流」注入以后每一轮。磁盘上因此长出了一条
  // uses=4 的「目录勘察后建站」：五步全是 list/read，最后一步是「先和用户确认再开始动手」。
  // **「让它干活它不干、光说」已经被总结成这类任务的标准做法，并且每轮喂给它自己看。**
  // 这条否决就是为了断掉那个回路。
  //
  // 放在链条**最后**是刻意的：它永远不会盖掉一个更具体的成因，只在其余判据都说
  // 「看起来完成了」时才出声。
  //
  // 第二个合取项防的是结论过期：评审之后又落过盘，说明它读的是旧版本的世界，这时候再否决
  // 就是拿旧结论冤枉新工作（同一个坑让建议卡叫用户去做一件已经做完的事）。
  if (f.wrapUpDone === false && (f.implOps | 0) <= reviewedAt(f)) return "wrapup_not_delivered";

  return "";
}

function reviewedAt(f) {
  return Number.isFinite(f.wrapUpReviewedAtImplOps) ? f.wrapUpReviewedAtImplOps : -1;
}

/** 结局标签。`cause` 由 `partialCause` 给，两者结构上不可能漂移。 */
export function runOutcome(f = {}, cause = partialCause(f)) {
  if (f.awaitingUserReply) return "awaiting_user";
  if (f.finalErr) return "failed";
  return cause ? "partial" : "success";
}

/**
 * 零交付的运行**也要被评审**。
 *
 * 原来的闸门第一个合取项是 `_mutatedCode`，于是「一个文件都没碰」的运行一次都进不去——
 * 恰好是最该被质疑的那一类，从来没人看过。判据全部是执行事实（落盘账本 + 这一轮真的
 * 干过事），不含任何分类器、不含模型自报的枚举：那类判据在这个仓库反复出现「裁决赶不上
 * 就恒等于默认值、整道门结构性哑掉」。
 *
 * 上界是一次。评审本身发的是**完整**的那一份，不是精简版——精简会把
 * 「问『项目是干嘛的』这类题必须真读过入口和核心模块」这条判据一起砍掉，
 * 而那正是零交付这一类最需要被抓住的地方。
 */
export function shouldReviewZeroDelivery(f = {}) {
  return f.mode === "agent" && !f.didMutate && (f.reviews | 0) === 0 && (f.steps | 0) >= 2;
}
