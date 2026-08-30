/**
 * 专业域小抄的**一张卡**。
 *
 * # 为什么合成一张
 *
 * 这份预检对一个域发四次检索（适用条件 / 硬性约束 / 常见坑 / 必须做的检查），此前每次
 * 各出一张工具卡。用户实拍：一轮里连出四张一模一样的「知识检索」，把第一个模型回合之前
 * 的整片视野占满，而它们其实是**同一次预检的四个面**——一个操作，不是四个。
 *
 * # 但不做抽屉
 *
 * app.css 里记着一条已经被删过的设计：「Activity (N steps)」抽屉把卡片搬进另一个容器，
 * 「等于把内容藏到第二个地方让用户再去翻」。所以这里**不套容器、不藏内容**：四个面的
 * 命中数直接写在卡面上，展开时四段正文在同一个 viewport 里按顺序排开。
 *
 * 合并的判据是语义上的「这是一个操作」，不是「卡太多了收起来」——后者正是那条被删掉的抽屉。
 */

/**
 * 卡面上那一行：`适用条件 3 · 硬性约束 3 · 常见坑 0 · 必须做的检查 失败`。
 *
 * 失败的那一面写「失败」，不写 0 —— 这条和 `_knowledgeSettleLabel` 是同一个判据：
 * **零命中是一个结论**（这个域里确实没有这个主题，模型可以据此往下走），
 * **失败是没有结论**（语料好端端摆着，只是这次没拿到）。写成 0 就把后者伪装成前者。
 */
export function facetSummary(sections) {
  const list = Array.isArray(sections) ? sections : [];
  const parts = list
    .filter((s) => s && s.heading)
    .map((s) => {
      if (s.failed) return `${s.heading} 失败`;
      const n = Array.isArray(s.bullets) ? s.bullets.length : 0;
      // 第三态：命中了 N 段，但一条要点都没压出来（行级过滤器把标题行/表格骨架/过短行
      // 筛干净了，实测约 13.6% 的小节会这样）。写成裸的 0 就和「这个域确实没有这个主题」
      // 长得一模一样——而后者是可以据此往下走的结论，前者不是。写成 0/4 段。
      const hits = Number(s.hits) || 0;
      return `${s.heading} ${n === 0 && hits > 0 ? `0/${hits} 段` : n}`;
    });
  return parts.join(" · ");
}

/**
 * 结算标签。**四个面全失败时返回空串**，让调用方交给 `_knowledgeSettleLabel` 去说
 * 「检索失败 · <原因>」——那是全系统唯一负责区分失败与零命中的地方，这里不另抄一份。
 * 部分失败仍报条数，但把失败面数缀在后面：拿到一半也是拿到，不该整张卡说成失败。
 */
export function preflightSettleLabel(sections) {
  const list = Array.isArray(sections) ? sections : [];
  const failed = list.filter((s) => s?.failed).length;
  if (list.length && failed === list.length) return "";     // 交给 _knowledgeSettleLabel
  const total = list.reduce((n, s) => n + (Array.isArray(s?.bullets) ? s.bullets.length : 0), 0);
  if (!total) {
    if (failed) return `无可用命中 · ${failed} 面失败`;
    // 同上：检索命中了、只是没压出要点，不能和真零命中共用一句话。
    const hits = list.reduce((n, s) => n + (Number(s?.hits) || 0), 0);
    return hits > 0 ? `命中 ${hits} 段 · 未压出要点` : "无可用命中";
  }
  return failed ? `${total} 条 · ${list.length} 面 · ${failed} 面失败` : `${total} 条 · ${list.length} 面`;
}

/**
 * 展开后的正文：四段按固定顺序排开，各带小标题。
 *
 * 顺序取 sections 传进来的顺序（也就是 rubric 的定义顺序），不按命中数排——
 * 「适用条件 → 硬性约束 → 常见坑 → 必须做的检查」本身是一条阅读线，
 * 按数量重排会把它打乱。空的那一面也留着标题并写明没有，不静默消失：
 * 「这一面没查到」和「这一面不存在」是两件事。
 */
export function preflightBody(sections, cap = 6000) {
  const list = Array.isArray(sections) ? sections : [];
  const out = [];
  for (const s of list) {
    if (!s || !s.heading) continue;
    const bullets = Array.isArray(s.bullets) ? s.bullets : [];
    out.push(`【${s.heading}】${s.failed ? "（检索失败，不等于库里没有）" : (bullets.length ? "" : "（无可用命中）")}`);
    for (const b of bullets) out.push(`  · ${String(b).trim()}`);
    out.push("");
  }
  const text = out.join("\n").trimEnd();
  return text.length > cap ? text.slice(0, cap) + "\n…（已截断）" : text;
}

/**
 * 把已经落地的预检卡挪到思考卡**后面**。
 *
 * # 为什么要挪
 *
 * 专业域预检是**挡在第一个模型回合之前**跑的（见 main.js 那处 await），所以它的卡必然
 * 先于任何模型输出出现。用户看到的是：还没有任何「它在想」的迹象，先冒出一张知识检索卡。
 * 观感上像"它在瞎查东西"，而不是"它在为这一轮做准备"。
 *
 * 时间线上预检确实在前，但对读的人来说，知识是**这一轮答案的输入**，摆在思考旁边才读得通。
 * 所以只调整视觉顺序，不改任何执行顺序。
 *
 * # 判据
 *
 * 只挪带 `data-knowledge-preflight` 的卡，且只挪**在 anchor 之前**的那些——
 * anchor 之后新出的（下一轮预检）本来就在正确位置，再挪一次会把它推到更后面。
 */
export function movePreflightCardsAfter(body, anchor) {
  try {
    if (!body?.querySelectorAll || !anchor?.parentNode) return 0;
    const cards = [...body.querySelectorAll('[data-knowledge-preflight="1"]')];
    let moved = 0, at = anchor;
    for (const card of cards) {
      // compareDocumentPosition：2 = card 在 anchor 之前（DOCUMENT_POSITION_PRECEDING）
      if (!(anchor.compareDocumentPosition(card) & 2)) continue;
      at.parentNode.insertBefore(card, at.nextSibling);
      at = card;                 // 多张时保持它们原来的相对顺序
      moved++;
    }
    return moved;
  } catch { return 0; }
}

/**
 * 把四个面那一行摆到卡的标题行下面。
 *
 * 摆在卡面上而不是藏进正文，是这次合并的**前提**：合成一张卡如果要点开才看得见四个面，
 * 那就是 app.css 里记着的那个被删掉的抽屉——「把内容藏到第二个地方让用户再去翻」。
 *
 * 复用已有的 `.atc-why` 样式（工具卡本来就用它显示「为什么调这个工具」那一行），
 * 不新开一套 class：新样式要在浅色/暗色两套主题里各配一遍，而这一行本来就是同一种东西。
 */
export function attachFacetLine(step, text, escapeHtml) {
  try {
    if (!step?.querySelector || !text) return false;
    const row = step.querySelector(".atc-action-row");
    if (!row?.parentNode) return false;
    const esc = typeof escapeHtml === "function" ? escapeHtml : (x) => String(x);
    const el = (step.ownerDocument || globalThis.document)?.createElement?.("div");
    if (!el) return false;
    el.className = "atc-why";
    el.innerHTML = `<span class="atc-why__v">${esc(text)}</span>`;
    row.parentNode.insertBefore(el, row.nextSibling);
    return true;
  } catch { return false; }
}

/**
 * 建卡 / 结算卡。整段搬到模块里，不是为了好看——main.js 有一条尺寸闸，
 * 而这段逻辑（判失败、算标签、写正文、摆四面）全部只依赖参数，没有一处读全局。
 *
 * `deps` 里那三个是 main.js 侧的现成函数，**必须注入而不是复制**：
 * `knowledgeSettleLabel` 是全系统唯一区分「检索失败」与「零命中」的地方，
 * 在这里另写一份判据，两份迟早漂开——这个仓库为此付过很多次账。
 */
export function createPreflightCard(body, domain, createToolStep) {
  try {
    if (!body?.appendChild || typeof createToolStep !== "function") return null;
    const step = createToolStep({
      type: "knowledge", domain, query: `${domain} · 四面预检`,
      topK: 4, corpus: false, _domainKnowledgePreflight: true,
    });
    if (!step) return null;
    if (step.dataset) step.dataset.knowledgePreflight = "1";   // 思考卡出现时据此挪下去
    body.appendChild(step);
    return step;
  } catch { return null; }
}

export function settlePreflightCard(step, sections, deps = {}) {
  try {
    if (!step) return false;
    const { settleToolStep, knowledgeSettleLabel, escapeHtml } = deps;
    const vp = step.querySelector?.(".atc-viewport");
    if (vp) vp.textContent = preflightBody(sections);
    attachFacetLine(step, facetSummary(sections), escapeHtml);
    // 全失败时 label 是空串 → 交回 knowledgeSettleLabel 说「检索失败 · 原因」。
    const label = preflightSettleLabel(sections);
    const list = Array.isArray(sections) ? sections : [];
    const result = label ? { type: "knowledge", ok: true }
      : (list.find((x) => x?.failResult)?.failResult || { type: "knowledge", content: "[失败] 预取异常" });
    if (typeof settleToolStep !== "function") return false;
    settleToolStep(step, result,
      typeof knowledgeSettleLabel === "function" ? knowledgeSettleLabel({ type: "knowledge" }, result, label) : label);
    return true;
  } catch { return false; }
}

/**
 * 把 michael-design 预检的三条结果摊成 `settlePreflightCard` 要的 sections。
 *
 * 搬到模块里不是为了好看：main.js 有一条尺寸闸，而这段只依赖参数——没有 DOM、没有全局、
 * 没有模块级可变状态，正是那条闸说的「能搬就搬」。
 *
 * `extractBullets` 必须注入而不是在这里另写一份：抽要点那套行级过滤器（丢标题行、丢表格
 * 分隔、丢过短行）在 main.js 里，专业域那条路用的就是它。两条路共用同一个抽取器，卡面上
 * 的条数才是同一个意思；各写一份迟早漂开。
 *
 * `failed` 的判据由调用方按**结构**算好传进来（`!result.knowledge`），这里只搬运不重判——
 * 「零命中」和「检索失败」的唯一判据在 _knowledgeSettleLabel，这个仓库为抄第二份付过账。
 */
export function designPreflightSections(results, extractBullets) {
  const list = Array.isArray(results) ? results : [];
  const pick = typeof extractBullets === "function" ? extractBullets : () => [];
  return list.map((item) => ({
    heading: String(item?.plan?.purpose || item?.plan?.id || "检索"),
    bullets: pick(String(item?.result?.content || "")),
    failed: !!item?.failed,
    failResult: item?.failResult || (item?.failed ? item?.result : null),
  }));
}
