/**
 * Hierarchical conversation memory with LLM-powered intelligent compression.
 * The LLM decides what's important based on CONTENT, not position — a key
 * decision from message #3 stays while a verbose tool dump from #48 gets
 * condensed. No fixed "keep last N" window.
 *
 * Structure: summaries (compressed context) + recent (new messages since
 * last compaction). LLM compaction is driven by main.js.
 */

const RECENT_WINDOW = 100;
const MAX_SUMMARIES = 8;
const SUMMARY_BATCH = 10;

export class ConversationMemory {
  constructor() {
    this.totalTurns = 0;
    this.recent = [];
    this.summaries = [];
    this.milestones = [];
    this.fileEvidence = [];
  }

  push(msg) {
    this.totalTurns++;
    this.recent.push(msg);
    if (this.recent.length > RECENT_WINDOW) {
      this._compressOldestBatch();
    }
  }

  markMilestone(event) {
    this.milestones.push({ turn: this.totalTurns, event });
  }

  recordFileEvidence(evidence) {
    if (!evidence || !evidence.root || !evidence.path || !evidence.signature) return;
    const next = {
      root: String(evidence.root),
      path: String(evidence.path),
      signature: String(evidence.signature),
      total: Math.max(0, Number(evidence.total) || 0),
      ranges: Array.isArray(evidence.ranges) ? evidence.ranges : [[evidence.from, evidence.to]],
      digest: String(evidence.digest || "").slice(0, 700),
      redacted: !!evidence.redacted,
      updatedAt: Number(evidence.updatedAt) || Date.now(),
    };
    next.ranges = next.ranges
      .map((range) => [Math.max(1, Number(range?.[0]) || 1), Math.max(0, Number(range?.[1]) || 0)])
      .filter((range) => range[1] >= range[0])
      .sort((a, b) => a[0] - b[0]);
    const index = this.fileEvidence.findIndex((item) => item.root === next.root && item.path === next.path);
    const current = index >= 0 ? this.fileEvidence[index] : null;
    if (current && current.signature === next.signature) {
      next.ranges.push(...(Array.isArray(current.ranges) ? current.ranges : []));
      if (!next.digest) next.digest = current.digest || "";
      next.redacted = next.redacted || !!current.redacted;
    }
    next.ranges.sort((a, b) => a[0] - b[0]);
    const merged = [];
    for (const range of next.ranges) {
      const last = merged[merged.length - 1];
      if (last && range[0] <= last[1] + 1) last[1] = Math.max(last[1], range[1]);
      else merged.push([...range]);
    }
    next.ranges = merged;
    next.complete = next.total > 0 && next.ranges.length === 1 && next.ranges[0][0] === 1 && next.ranges[0][1] >= next.total;
    if (index >= 0) this.fileEvidence.splice(index, 1);
    this.fileEvidence.push(next);
    if (this.fileEvidence.length > 80) this.fileEvidence.splice(0, this.fileEvidence.length - 80);
  }

  invalidateFileEvidence(root, path) {
    const r = String(root || ""), p = String(path || "");
    this.fileEvidence = this.fileEvidence.filter((item) => !(item.root === r && item.path === p));
  }

  fileEvidenceForRoot(root) {
    const r = String(root || "");
    return this.fileEvidence.filter((item) => item.root === r).map((item) => ({ ...item, ranges: (item.ranges || []).map((range) => [...range]) }));
  }

  assemble() {
    const result = [];
    if (this.milestones.length > 0) {
      const text = this.milestones
        .map(m => `[Turn ${m.turn}] ${m.event}`)
        .join('\n');
      result.push({ role: 'system', content: `📌 Key milestones from earlier:\n${text}` });
    }
    if (this.summaries.length > 0) {
      const merged = this.summaries.map(s => s.text).join('\n\n');
      result.push({ role: 'assistant', content: `[对话上下文摘要]\n${merged}` });
    }
    result.push(...this.recent);
    return result;
  }

  estimateRecentTokens() {
    let t = 0;
    for (const m of this.recent) {
      const c = typeof m.content === 'string' ? m.content : '';
      for (let i = 0; i < c.length; i++) t += c.charCodeAt(i) > 0x2E7F ? 1 : 0.25;
    }
    return Math.ceil(t);
  }

  /** Replace `count` oldest messages in recent with an LLM-generated summary.
   *  count can equal recent.length (compress everything). */
  compactRecent(count, summaryText) {
    if (count <= 0 || count > this.recent.length) return [];
    const removed = this.recent.splice(0, count);
    const startTurn = Math.max(1, this.totalTurns - this.recent.length - removed.length + 1);
    const endTurn = startTurn + removed.length - 1;
    this.summaries.push({
      range: `turns ${startTurn}–${endTurn}`,
      text: summaryText
    });
    while (this.summaries.length > MAX_SUMMARIES) {
      const a = this.summaries.shift();
      const b = this.summaries.shift();
      this.summaries.unshift({
        range: `${a.range}, ${b.range}`,
        text: a.text + '\n' + b.text
      });
    }
    return removed;
  }

  _compressOldestBatch() {
    if (this.recent.length < SUMMARY_BATCH) return;
    const batch = this.recent.splice(0, SUMMARY_BATCH);
    const startTurn = this.totalTurns - this.recent.length - batch.length;
    const endTurn = startTurn + batch.length - 1;
    this.summaries.push({ range: `turns ${startTurn}-${endTurn}`, text: this._summarizeBatch(batch) });
    if (this.summaries.length > MAX_SUMMARIES) {
      const a = this.summaries.shift();
      const b = this.summaries.shift();
      this.summaries.unshift({ range: `${a.range} + ${b.range}`, text: `${a.text}; ${b.text}` });
    }
  }

  _summarizeBatch(batch) {
    const actions = new Set(), files = new Set(), fixes = [], userReqs = [];
    for (const msg of batch) {
      if (msg.tool_calls) for (const tc of msg.tool_calls) {
        const n = tc.function?.name;
        if (n) { actions.add(n); if (['write_file','edit_file','read_file'].includes(n)) try { const a = JSON.parse(tc.function.arguments||'{}'); if (a.path) files.add(a.path); } catch {} }
      }
      if (msg.role === 'user' && msg.content) { const l = String(msg.content).split('\n')[0].slice(0,120); if (l.length > 5) userReqs.push(l); }
      if (msg.role === 'assistant' && /fixed|resolved|修复|已修复|解决|完成/i.test(msg.content)) { const f = msg.content.split('\n')[0]; if (f.length < 150) fixes.push(f); }
    }
    const p = [];
    if (userReqs.length) p.push(`User: ${userReqs.join('; ')}`);
    if (actions.size) p.push(`Actions: ${[...actions].join(', ')}`);
    if (files.size) p.push(`Files: ${[...files].join(', ')}`);
    if (fixes.length) p.push(`Outcomes: ${fixes.join('; ')}`);
    return p.length ? p.join(' | ') : 'Continued conversation.';
  }

  stats() {
    return { totalTurns: this.totalTurns, recentCount: this.recent.length, summaryCount: this.summaries.length, milestoneCount: this.milestones.length, recentTokens: this.estimateRecentTokens() };
  }

  toJSON() {
    return { totalTurns: this.totalTurns, recent: this.recent, summaries: this.summaries, milestones: this.milestones, fileEvidence: this.fileEvidence };
  }

  static fromJSON(obj) {
    const mem = new ConversationMemory();
    if (obj) {
      mem.totalTurns = obj.totalTurns || 0;
      mem.recent = obj.recent || [];
      mem.summaries = obj.summaries || [];
      mem.milestones = obj.milestones || [];
      mem.fileEvidence = Array.isArray(obj.fileEvidence) ? obj.fileEvidence.slice(-80) : [];
    }
    return mem;
  }
}
