/**
 * 自动压缩时，喂给摘要器的对话转录 —— 以及**这次压缩到底能覆盖到第几条**。
 *
 * 两个数必须是同一个来源。原来的写法是整份拼起来再 `.slice(0, 80000)`，而删除范围
 * 另按 `snapshot.length` 算：超出预算的那些消息，摘要器一眼没看过，却照样被
 * compactRecent 删出 recent。实测 120 条各 ~3400 字的历史——摘要器只看到前 24 条，
 * 删掉 114 条，其中 90 条从没进过转录。
 *
 * 它们还躺在 archive 里（recall_conversation 捞得回来），所以不是永久销毁；真正的
 * 伤害是**摘要顶头写着「turns 1–114」**。模型没有任何理由认为中间少了 90 轮，也就
 * 不会去 recall——「没覆盖到」和「覆盖了但没什么可说」在这里是同一个值。
 *
 * 所以按预算逐条装，装得下几条就只压几条，剩下的下一轮再压。每条上限 4000 字，
 * 80000 的预算至少装得下 20 条，压缩一定有进展，不会卡住。
 */
const PER_MESSAGE_MAX = 4000;

export function buildCompactionTranscript(recent, budget = 80_000) {
  const list = Array.isArray(recent) ? recent : [];
  const parts = [];
  let used = 0;
  for (let i = 0; i < list.length; i++) {
    const piece = `[#${i}][${list[i]?.role}] ${String(list[i]?.content || "").slice(0, PER_MESSAGE_MAX)}`;
    // fitCount > 0 的例外：第一条就超预算也得装，否则永远压不动。
    if (used + piece.length > budget && parts.length > 0) break;
    parts.push(piece);
    used += piece.length + 2;
  }
  return { transcript: parts.join("\n\n"), fitCount: parts.length };
}
