function _classifyToolFailure(resultText) {
  const t = String(resultText || "").toLowerCase();
  if (!t) return "other";
  if (/not found|no such file|enoent|404|does not exist|missing file|不存在|找不到/.test(t)) return "not_found";
  if (/permission|eacces|eperm|access denied|denied|forbidden|403|权限/.test(t)) return "permission";
  if (/timeout|timed out|etimedout|aborted|abortcontroller|超时/.test(t)) return "timeout";
  if (/invalid|bad request|400|malformed|required parameter|missing parameter|unexpected token|参数错误|schema/.test(t)) return "invalid_input";
  if (/econnrefused|econnreset|enotfound|network|fetch failed|dns|socket hang|502|503|连接失败|网络/.test(t)) return "network";
  return "other";
}

/**
 * 本次会话的工具成败台账，渲染成给模型看的一段文字。
 *
 * 三个消费方全部面向模型：工具编排腿（「本次会话工具成败账本」）、子智能体任务书
 * （「避开已知失败路径，复用已验证工具」）、收尾评审。
 *
 * **失败行必须优先。** 原来的写法是按 okCount 降序排完再 slice(0, 20)：一个只失败
 * 没成功的工具（0✓/4✗）**结构性地**永远排在最末、必被截掉。实测本 run 用过 21 个
 * 不同工具时，三个连败工具（run_in_terminal / browser / db_query，各 4 连败）全部
 * 从输出里消失，模型看到的是 20 行漂亮的 10✓/0✗，头上只挂一句「other类别: 0✓/12✗」
 * ——不说是哪个工具，也不说为什么。于是下一阶段照旧把 run_in_terminal 装进窗口，
 * 派出去的子体拿着一份标题写着「避开已知失败路径」、正文里一条失败路径都没有的
 * 交接块，把同一堵墙原样再撞四次。用户看到的就是「它一直在重复同一个失败的操作」。
 * 「工具真没用过」和「工具用了每次都炸」在这里被压成了同一件事。
 *
 * 排序也不再从渲染好的文本里正则抠数字（`\d+✓`）——那是拿自己刚格式化出来的字符串
 * 当数据源，改一下文案就静默失序。
 */
export function toolLedgerStats(entries) {
  if (!Array.isArray(entries) || entries.length === 0) return "";
  const agg = new Map();
  const catAgg = new Map(); // 按类别聚合：搜索工具失败了 5 次
  for (const e of entries) {
    const k = e.tool;
    if (!k) continue;
    if (!agg.has(k)) agg.set(k, { okCount: 0, failCount: 0, lastFailReason: "", failCats: new Map(), category: e.category || "other" });
    const v = agg.get(k);
    if (e.ok) v.okCount++;
    else {
      v.failCount++;
      if (e.reason) v.lastFailReason = String(e.reason).slice(0, 60);
      // 失败类别分布：优先用预计算的 failCategory，兆底用 _classifyToolFailure
      const cat = e.failCategory || _classifyToolFailure(e.reason || "");
      v.failCats.set(cat, (v.failCats.get(cat) || 0) + 1);
      // 类别聚合
      const toolCat = e.category || "other";
      if (!catAgg.has(toolCat)) catAgg.set(toolCat, { ok: 0, fail: 0 });
      catAgg.get(toolCat).fail++;
    }
    // 类别聚合（成功）
    if (e.ok) {
      const toolCat = e.category || "other";
      if (!catAgg.has(toolCat)) catAgg.set(toolCat, { ok: 0, fail: 0 });
      catAgg.get(toolCat).ok++;
    }
  }
  const lines = [];
  for (const [tool, data] of agg) {
    const total = data.okCount + data.failCount;
    if (total === 0) continue;
    let f = "";
    if (data.failCount > 0) {
      const cats = [...data.failCats.entries()].sort((a, b) => b[1] - a[1]).map(([c, n]) => `${c}\u00d7${n}`).join(",");
      f = ` (${cats}${data.lastFailReason ? `\uff1b最近失败：${data.lastFailReason}` : ""})`;
    }
    lines.push({ text: `${tool}[${data.category}]: ${data.okCount}\u2713/${data.failCount}\u2717${f}`,
      ok: data.okCount, fail: data.failCount });
  }
  // 类别汇总行：让编排器能看到“搜索工具失败了 5 次”
  const catLines = [];
  for (const [cat, data] of catAgg) {
    if (data.fail > 0) catLines.push(`${cat}\u7c7b\u522b: ${data.ok}\u2713/${data.fail}\u2717`);
  }
  // 失败行优先。原来按 okCount 降序再 slice(0, 20)：只失败没成功的工具结构性地
  // 永远排最末、必被截掉——而这本账两个消费方的标题正是「避开已知失败路径」。
  const failedRows = lines.filter((r) => r.fail > 0).sort((a, b) => b.fail - a.fail);
  const okRows = lines.filter((r) => r.fail === 0).sort((a, b) => b.ok - a.ok);
  const LIMIT = 20;
  const picked = [...failedRows, ...okRows].slice(0, LIMIT);
  // 截断要说出来：不说的话这份账读起来就是「本 run 只用过这些工具」。
  const dropped = lines.length - picked.length;
  const tail = dropped > 0 ? `\n\u5176\u4f59 ${dropped} \u4e2a\u5de5\u5177\u5168\u90e8\u6210\u529f\uff0c\u672a\u5217\u51fa\u3002` : "";
  const header = catLines.length ? `\u3010\u7c7b\u522b\u6c47\u603b\u3011${catLines.join(" | ")}\n` : "";
  return header + picked.map((r) => r.text).join("\n") + tail;
}
