// Plan 模式那一屏的「核心内容」。
//
// # 为什么要抽，而不是把回复原样搬过去
//
// 对话里那份是给人读的：先讲取证过程、再讲权衡、最后才是要做什么。而方案窗口是给人
// **照着做**的 —— 用户原话「AI 回复的是用户完整内容，但这个窗口要写的是核心逻辑那些」。
// 所以这里做减法：留下能执行、能核对的部分（目标、关键文件/接口、步骤、验证命令、风险），
// 把叙述性的取证过程去掉。
//
// 判据只用**结构**，不猜语义：Plan 模式的提示词规定了输出小节，标题里出现什么词就归到
// 哪一类。抽不出小节时退回「留列表和代码块、去掉大段散文」——那两样天然是可执行的部分。

/** 这些小节留下。命中的判据是标题里含任一关键词。 */
const KEEP = [
  ["目标", "非目标", "goal", "objective"],
  ["关键文件", "接口", "数据契约", "契约", "file", "interface", "contract"],
  ["计划", "步骤", "实施", "plan", "step"],
  ["验证", "verify", "test", "检查"],
  ["风险", "未知", "risk", "unknown"],
];
/** 这些小节去掉：它们是「为什么这么定」的过程，不是「要做什么」。 */
const DROP = [["证据", "取证", "现状", "背景", "evidence", "context", "background"]];

const hit = (title, groups) => {
  const t = String(title || "").toLowerCase();
  return groups.some((g) => g.some((w) => t.includes(String(w).toLowerCase())));
};

/** 把 markdown 按 ATX 标题切成小节。第一段没有标题时归到一个空标题的小节里。 */
function splitSections(md) {
  const lines = String(md || "").split("\n");
  const out = [];
  let cur = { title: "", level: 0, body: [] };
  let inFence = false;
  for (const line of lines) {
    // 代码块里的 # 不是标题 —— 不判这个的话，shell 注释会把方案切碎。
    if (/^\s*```/.test(line)) inFence = !inFence;
    const m = !inFence && /^(#{1,6})\s+(.*)$/.exec(line);
    if (m) {
      if (cur.title || cur.body.some((l) => l.trim())) out.push(cur);
      cur = { title: m[2].trim(), level: m[1].length, body: [] };
    } else {
      cur.body.push(line);
    }
  }
  if (cur.title || cur.body.some((l) => l.trim())) out.push(cur);
  return out;
}

/** 没有小节可依据时的兜底：只留列表项、代码块、表格 —— 散文段落去掉。 */
function actionableOnly(md) {
  const lines = String(md || "").split("\n");
  const out = [];
  let inFence = false;
  for (const line of lines) {
    if (/^\s*```/.test(line)) { inFence = !inFence; out.push(line); continue; }
    if (inFence) { out.push(line); continue; }
    if (/^\s*([-*+]|\d+[.)])\s+/.test(line) || /^\s*\|/.test(line) || !line.trim()) out.push(line);
  }
  return out.join("\n").replace(/\n{3,}/g, "\n\n").trim();
}

/**
 * 从一份 Plan 回复里抽出方案窗口要展示的核心内容。
 *
 * 抽不出任何东西时返回空串 —— 调用方据此**不开窗口**，而不是开一个空窗口。
 * 「没有可执行内容」和「有但抽错了」是两回事，前者不该以一个空白页的形式呈现。
 */
export function planCoreFromReply(md) {
  const text = String(md || "").trim();
  if (!text) return "";
  const sections = splitSections(text);
  const titled = sections.filter((s) => s.title);
  if (titled.length) {
    const kept = titled.filter((s) => hit(s.title, KEEP) && !hit(s.title, DROP));
    if (kept.length) {
      return kept
        .map((s) => `${"#".repeat(Math.min(3, Math.max(2, s.level)))} ${s.title}\n${s.body.join("\n").trim()}`)
        .join("\n\n")
        .replace(/\n{3,}/g, "\n\n")
        .trim();
    }
  }
  return actionableOnly(text);
}

/** 窗口标题：取第一个标题，没有就用默认名。太长的截断（标签栏放不下）。 */
export function planTitleFromReply(md, fallback = "方案") {
  const first = splitSections(String(md || "")).find((s) => s.title);
  const t = (first?.title || "").replace(/[*`#]/g, "").trim();
  if (!t) return fallback;
  return [...t].length > 14 ? [...t].slice(0, 14).join("") + "…" : t;
}
