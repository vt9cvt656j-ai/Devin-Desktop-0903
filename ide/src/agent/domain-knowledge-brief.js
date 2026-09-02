/**
 * 专业域小抄：从检索结果里抽要点，再压成一份给模型读的简报。
 *
 * # 为什么搬出来
 *
 * main.js 有一条尺寸闸，而这两个函数是纯的：给字符串出字符串，无 DOM、无网络、
 * 无模块级可变状态。判据和 mainlink / approval-label 那几次搬迁一致。
 *
 * # 这份简报最容易骗人的地方：三种状态被压成同一句话
 *
 * 一次检索有**三种**结局，含义完全不同：
 *   ① 检索失败（HTTP 非 2xx / 超时）—— 没有结论。语料好端端摆着，只是这次没拿到。
 *   ② 真零命中 —— 有结论：这个域里确实没有这个主题，模型可以据此往下走。
 *   ③ 命中了 N 段，但抽取器一条要点都没抽出来 —— 也不是「没有」，是我们自己筛干净的
 *      （行级过滤器会丢标题行、表格分隔、过短行；实测仍有约 13.6% 的小节抽不出要点）。
 *
 * 原来三种全落到同一句「本域语料里没有」。①被说成②的后果最重：四条 rubric 全超时时，
 * 模型收到「该领域语料不可用，不要编造该领域的规则」，于是判定这个域没有知识可用——
 * 而卡片 UI 那侧显示的是「检索失败 · 请求超时」。用户和模型看到的结论互相矛盾。
 *
 * 判据不是重新算的：`failed` 在 _runDomainKnowledgePreflight 里已经按**结构**判好了
 *（成功的结果带 knowledge 字段，两条失败路径都不带），这里只读不算。
 */

export const DOMAIN_KNOWLEDGE_BRIEF_BUDGET = 2500;

export function domainKnowledgeBullets(content, maxBullets = 3, maxChars = 190, seen = null) {
  const blocks = String(content || "").split("\n\n———\n\n");
  const bullets = [];
  for (const block of blocks) {
    const cut = block.indexOf("】");
    if (cut < 0) continue;
    const section = /·\s*([^｜】]+)】\s*$/.exec(block.slice(0, cut + 1))?.[1]?.trim() || "";
    const rawLines = block.slice(cut + 1).split("\n");
    let inFence = false;
    let prose = "";     // 首选：散文/表格数据行
    let fenced = "";    // 兜底：代码围栏里的第一行
    // seen：跨 rubric 去重。命中已见的就往下取**同一段的下一条**，而不是把这一栏丢空——
    // 四条 rubric 各查一次、共 16 个检索位，而多数域的小节数比这还少，重复是结构性的。
    const _fresh = (t) => !seen || !seen.has(String(t).trim());
    for (let li = 0; li < rawLines.length && !prose; li++) {
      const line = rawLines[li];
      if (/^\s*(?:```|~~~)/.test(line)) { inFence = !inFence; continue; }
      // markdown 标题行要在**剥 # 之前**判掉。
      //
      // 网关的索引器把小节标题写回了正文第一行（knowledge.rs 里 `cur_buf = format!("## {}\\n", …)`），
      // 而这里原来先剥 # 再按长度过滤——于是「Database Selection Decision Tree」这种 32 字符的
      // 标题顺利活过 `< 24` 那道门，成为首条要点。
      //
      // 实测（2026-08-23，真语料 + 真函数）：非设计域 683 个小节里 **416 段（60.9%）的首条
      // 要点就是它自己的小节标题**。而这份小抄的抬头对模型说的是「它是该领域的**既有事实**，
      // 不是灵感：先按它判断可行性、约束和验收」——给的却是一份标题清单。被这样吞掉的正是
      // 最该用上的那几条：服务拆分规则、数据库选型决策树、重构策略。
      //
      // 判据按**结构**不按字符串：跳所有 markdown 标题行，而不是拿正文和 section 名做前缀
      // 比较——后者会把「小节名 Rate Limiting + 首句 Rate Limiting must be applied per-tenant…」
      // 这种真事实一起误杀。
      if (/^\s*#{1,6}\s/.test(line)) continue;
      // 表格**不能**一刀切：这份语料里大量事实本身就是表（反模式清单、选型对照表都是），
      // 砍掉整类会让产不出要点的小节从 13.6% 飙到 51.7%（实测）。只砍两种不带事实的行：
      // 分隔行 `|---|---|`，以及紧跟着分隔行的那一行（那是表头）。
      if (/^\s*\|[\s:|-]+\|?\s*$/.test(line)) continue;
      if (/^\s*\|/.test(line) && /^\s*\|[\s:|-]+\|?\s*$/.test(rawLines[li + 1] || "")) continue;
      const text = line.replace(/^\s*(?:[-*•·]|\d+[.)])\s*/, "").trim();
      // 太短的行是空行或"示例："这类粘合词，进了小抄只占预算不带信息。
      if (text.length < 24) continue;
      // 代码围栏里的行**留作兜底**，不直接选中。一条孤立的 import 行说不出这个小节想说
      // 什么；但整节只有代码时，给一行真代码仍然远好过给空。一刀砍掉整类会让产不出要点的
      // 小节从 13.6% 涨到 49.2%（实测）——那是把一半语料静默扔掉。
      if (!_fresh(text)) continue;
      if (inFence) { if (!fenced) fenced = text; continue; }
      prose = text;
    }
    const picked = prose || fenced;
    if (!picked) continue;
    if (seen) seen.add(String(picked).trim());
    const one = picked.length > maxChars ? `${picked.slice(0, maxChars)}…` : picked;
    bullets.push(section ? `${section} → ${one}` : one);
    if (bullets.length >= maxBullets) break;
  }
  return bullets;
}

export function domainKnowledgeBrief(domain, sections) {
  const list = Array.isArray(sections) ? sections : [];
  const filled = list.filter((s) => s?.bullets?.length);
  // 三态里的第①种：一条都没成功过 = 没有结论。**绝不能**说成「本域语料里没有」——
  // 那是第②种的话，模型会据此断言这个域没有相关规则并继续往下走。
  const allFailed = list.length > 0 && list.every((s) => s?.failed);
  const head = `[DOMAIN_KNOWLEDGE_PRELOADED_BRIEF · ${domain}]
本轮在你开始规划前，IDE 已按「${domain}」这个专业领域从平台自有知识库真实检索并压缩出下面这份小抄。
它是该领域的既有事实，不是灵感：先按它判断可行性、约束和验收，再结合用户要求和项目证据动手。
没有列出的内容不代表不存在——需要更细的就自己调用 knowledge_search(domain="${domain}")；
但**不要把没命中的内容说成知识库结论**。`;
  if (allFailed) {
    return `${head}

本轮对「${domain}」的检索**没有拿到结果**（检索链路失败，例如超时或非 2xx）——这**不等于**库里没有。
不要据此断言该领域没有相关规则，也不要把「没查到」写进结论。需要就自己调
knowledge_search(domain="${domain}") 重试；在拿到之前，按用户约束与项目证据推进。`;
  }
  if (!filled.length) {
    // 第②/③种：检索是成功的。区分「真零命中」和「命中了但抽取器没抽出要点」——
    // 后者不是「这个域没有」，是我们自己的行级过滤器筛干净的，说成前者同样是误导。
    const hits = list.reduce((n, s) => n + (Number(s?.hits) || 0), 0);
    return `${head}

${hits > 0
  ? `本轮「${domain}」检索到 ${hits} 段内容，但都没能压出可用要点（多为标题行/表格骨架/过短行）。这**不是**「本域没有这个主题」——需要原文就自己调 knowledge_search(domain="${domain}") 取。`
  : `本轮「${domain}」检索成功但零命中：库里确实没有匹配这几个维度的内容。可以据此往下走，基于用户约束与项目证据继续，不要编造该领域的规则。`}`;
  }
  // 四栏是**问过的四个维度**，不是「有内容的那几栏」。空栏要如实说，不能整栏消失——
  // 消失会让模型以为那个维度压根没被问过，而事实是「问了，这个域没有独立的答案」。
  //（跨 rubric 去重之后这种情况会变多：同一小节对多条 rubric 都高分时，后面的栏
  // 拿到的是同段的下一条，取不到就空。）
  const body = list
    .map((s) => (s?.bullets?.length
      ? `【${s?.heading}】\n${s.bullets.map((b) => `- ${b}`).join("\n")}`
      : s?.failed
        ? `【${s?.heading}】\n- （这一栏这次**没查到（检索失败）**，不是本域没有；需要就自己调 knowledge_search(domain="${domain}") 重试。）`
        : `【${s?.heading}】\n- （本域语料里这一栏没有独立内容；需要就自己调 knowledge_search(domain="${domain}") 细查，别当成"没有要求"。）`))
    .join("\n\n");
  const full = `${head}\n\n${body}`;
  if (full.length <= DOMAIN_KNOWLEDGE_BRIEF_BUDGET) return full;
  // 记账要在**截断之后**按真实字数算，不是按"计划截多少"。给账目行预留位置，
  // 否则加完这行又超预算，等于预算没生效。
  const note = (dropped) => `\n…（本域小抄超出 ${DOMAIN_KNOWLEDGE_BRIEF_BUDGET} 字符预算，已截断 ${dropped} 字符；缺的部分用 knowledge_search(domain="${domain}") 自取，不要凭印象补。）`;
  const room = Math.max(0, DOMAIN_KNOWLEDGE_BRIEF_BUDGET - note(full.length).length);
  const kept = full.slice(0, room);
  return kept + note(full.length - kept.length);
}
