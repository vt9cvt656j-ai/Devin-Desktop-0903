/**
 * 把一次 run 的时间线汇总成**能落盘的几个数**。
 *
 * # 为什么需要它
 *
 * 排查「慢」的时候撞到一堵墙：这个产品连「典型首字延迟是 3 秒还是 15 秒」都答不上来。
 * `_createAgentTimeline` 每一轮都记了 startedAt / requestStartedAt / firstProgressAt /
 * doneAt / kind，数据一直在内存里、喂给界面上那个跑秒表，**然后随 run 一起消失**。
 *
 * 同样答不了的还有「一条用户消息到底发几次模型请求」。我试过从服务端的 model_usage 反推，
 * 失败了——那边的 request_id 是**每个会话**一个（最长的一条跨了 22 小时），不是每条消息，
 * 所以按它分组量不出来。而客户端这边 `turns` 数组里本来就是逐轮的。
 *
 * 这个模块不改变任何行为，只是把已有的时间戳算成几个数交给情景档案。判据全是执行事实
 * （时间戳之差），没有任何推断。
 *
 * # 为什么每一个字段都不许抛
 *
 * `_recordEpisode` 整个包在 try 里、异常被吞掉——一个字段算炸，**整条情景记录会静默消失**，
 * 而不是少一个字段。所以这里对每一处取值都做了兜底，宁可返回 null 也不抛。
 */

const num = (v) => (Number.isFinite(Number(v)) ? Number(v) : null);

/** 两个时间戳之差，任何一个不可用就返回 null（而不是 0——0 是个会骗人的答案）。 */
function span(from, to) {
  const a = num(from), b = num(to);
  return a != null && b != null && b >= a ? b - a : null;
}

function pct(sorted, p) {
  if (!sorted.length) return null;
  return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * p))];
}

/**
 * @param {{startedAt?:number, turns?:Array}} timeline
 * @returns {object|null} 落盘用的紧凑记录；时间线不可用时返回 null（调用方据此不写这个字段）
 */
export function summarizeTiming(timeline) {
  try {
    const turns = Array.isArray(timeline?.turns) ? timeline.turns.filter(Boolean) : [];
    if (!turns.length) return null;

    // 首字：从**这一轮的请求真正发出**算到第一个进度信号。不从 startedAt 算——
    // 那之间还夹着准备工作，两件事混在一个数里就谁也说明不了。
    const ttfb = turns.map((t) => span(t.requestStartedAt ?? t.startedAt, t.firstProgressAt))
      .filter((v) => v != null).sort((a, b) => a - b);

    // 准备开销：run 开始 → 第一个请求发出。用户感受到的「按下回车之后的沉默」在这里。
    const prep = span(timeline?.startedAt, turns[0].requestStartedAt ?? turns[0].startedAt);

    // 每一轮的墙钟。慢在等模型还是慢在跑工具，靠它和 ttfb 的差值分。
    const wall = turns.map((t) => span(t.startedAt, t.endedAt ?? t.doneAt))
      .filter((v) => v != null).sort((a, b) => a - b);

    // 按轮次类型分。「一条消息到底发几次模型请求」这个问题的答案就在这里——
    // 而在此之前它在客户端和服务端两侧都取不到。
    const kinds = {};
    for (const t of turns) {
      const k = String(t.kind || "main").slice(0, 24);
      kinds[k] = (kinds[k] || 0) + 1;
    }

    const retries = turns.reduce((n, t) => n + (num(t.retryCount) || 0), 0);

    return {
      turns: turns.length,
      ...(prep != null ? { prepMs: Math.round(prep) } : {}),
      ...(ttfb.length ? { ttfbMs: Math.round(pct(ttfb, 0.5)), ttfbP90Ms: Math.round(pct(ttfb, 0.9)) } : {}),
      ...(wall.length ? { turnMs: Math.round(pct(wall, 0.5)) } : {}),
      ...(retries ? { retries } : {}),
      kinds,
    };
  } catch {
    return null;   // 算不出来就不写这个字段；绝不能让整条情景记录跟着消失
  }
}

/**
 * 意图裁决前台等待的**胜负记账**。
 *
 * `_INTENT_FOREGROUND_WAIT_MS` 这个数被反复调过：1500 → 8000 → 15000 → 6000，
 * 每一次的依据都是几条手工实测，而「它到底赢了几成」从来没有落过盘。
 *
 * 输赢的后果差得很远：赢＝网关按完整画像挂上工程 / 调研 / 设计各层（实测提示词 26951 字节），
 * 输＝只剩 agent.base 四块（18885 字节），模型手里既没有工程纪律也没有设计纪律——
 * 那正是「突然变弱智、工具也不用了」的物理成因。
 *
 * 记一笔，一周之后这个数就不用再靠赌。会话级累计，随情景档案落盘。
 */
export function recordIntentRace(session, verdictWon, ms) {
  try {
    if (!session) return;
    const r = (session._intentRace ||= { won: 0, lost: 0, wonMs: [], lostMs: [] });
    const n = Number.isFinite(Number(ms)) ? Math.max(0, Math.round(Number(ms))) : 0;
    if (verdictWon) { r.won++; if (r.wonMs.length < 20) r.wonMs.push(n); }
    else { r.lost++; if (r.lostMs.length < 20) r.lostMs.push(n); }
  } catch { /* 记账绝不能影响那道 race 本身 */ }
}

/**
 * 给一次 race 造一个记账笔。
 *
 * 做成工厂而不是让调用方每次拼参数，是因为它要塞进 `Promise.race` 的两条臂里——
 * 那是**记账绝不能弄坏被记的那件事**的地方：`adopted.then(...)` 里抛一下，
 * 派生的那条臂就拒绝、race 跟着拒绝、整轮挂死——写这一笔时真挂过一次，
 * 而且因为测试跑的是 `--test-timeout=0`，它不报错，只是永远不结束。
 * 所以这里吞掉一切异常；调用点那边还有一层 `typeof` 兜底
 * （被单独装进测试沙箱跑时，模块级符号不存在，裸引用直接抛 ReferenceError）。
 */
export function intentRaceMarker(session, startedAt) {
  return (won) => { try { recordIntentRace(session, won, Date.now() - startedAt); } catch {} };
}

/** 把会话级的胜负记账压成落盘用的一行；没有数据就返回 null。 */
export function summarizeIntentRace(session) {
  try {
    const r = session?._intentRace;
    if (!r || (!r.won && !r.lost)) return null;
    const med = (a) => (a?.length ? [...a].sort((x, y) => x - y)[a.length >> 1] : null);
    return {
      won: r.won | 0, lost: r.lost | 0,
      ...(med(r.wonMs) != null ? { wonMs: med(r.wonMs) } : {}),
      ...(med(r.lostMs) != null ? { lostMs: med(r.lostMs) } : {}),
    };
  } catch { return null; }
}
