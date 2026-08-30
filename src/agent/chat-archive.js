// 聊天存档的合并。从 main.js 搬出来的纯函数 —— 它们不碰 DOM、不读全局，
// 而 main.js 有行数闸（见 test/main-size-budget.test.mjs），撞线时先搬模块再谈抬线。
//
// helpers/source.mjs 会把 src/agent/ 下每个模块和 main.js 拼成同一份 SRC，
// 所以原来那些按源码断言的守卫照常有效。

// 两份存档按**会话**合并，而不是「谁先有内容用谁」。
//
// 存档三层，恢复时依次尝试：SQLite 快照 → session.json → localStorage 镜像。判据原本
// 只有 `hasSavedChats`（有内容就用），**不比新旧**。而退出路径里只有 `onCloseRequested`
// 会 await 快照写完；`beforeunload` / `pagehide`（Tauri 直接 destroy webview、强杀、
// 更新重启走的正是这几条）跑不了异步，只来得及同步写 localStorage。
//
// 于是「几分钟前的快照 + 退出瞬间的镜像」两份都非空时，**旧的排在前面就赢了**。
//
// 合并而不是「整份取新的那个」：镜像按 media/text 预算截断过，整份采用会把所有老会话的
// 图片和长文一起降级。逐个会话比消息条数取更全的那个，只在一边出现的原样保留。
const _archiveMsgCount = (sess) => {
  if (!sess) return 0;
  const recent = sess.memory && sess.memory.recent;
  if (Array.isArray(recent)) return recent.length;
  return Array.isArray(sess.history) ? sess.history.length : 0;
};
const _mergeArchiveList = (primaryList, mirrorList) => {
  const out = [];
  const at = new Map();
  for (const sess of (Array.isArray(primaryList) ? primaryList : [])) {
    if (!sess) continue;
    if (sess.id) at.set(sess.id, out.length);
    out.push(sess);
  }
  for (const sess of (Array.isArray(mirrorList) ? mirrorList : [])) {
    if (!sess) continue;
    const i = sess.id ? at.get(sess.id) : undefined;
    if (i === undefined) { out.push(sess); continue; }
    // 两边都有同一个会话：谁的消息多要谁。相等留主存档 —— 它没被预算截断过。
    if (_archiveMsgCount(sess) > _archiveMsgCount(out[i])) out[i] = sess;
  }
  return out;
};
const _archiveHasChats = (v) => !!(v && ((Array.isArray(v.sessions) && v.sessions.length) || (Array.isArray(v.closedSessions) && v.closedSessions.length)));
function _mergeChatArchives(primary, mirror) {
  if (!_archiveHasChats(primary)) return mirror;
  if (!_archiveHasChats(mirror)) return primary;
  const sessions = _mergeArchiveList(primary.sessions, mirror.sessions);
  const closedSessions = _mergeArchiveList(primary.closedSessions, mirror.closedSessions);
  // activeIdx 跟主存档：合并后主存档那些会话的下标原样不变，镜像的未必对得上。
  const idx = Number.isFinite(primary.activeIdx) ? primary.activeIdx : 0;
  return {
    sessions,
    closedSessions,
    activeIdx: Math.max(0, Math.min(idx, Math.max(0, sessions.length - 1))),
  };
}

export { _archiveMsgCount, _mergeArchiveList, _archiveHasChats, _mergeChatArchives };
