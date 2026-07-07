/**
 * Hierarchical conversation memory — prevents context overflow in ultra-long sessions
 * while preserving critical decisions/files/errors through LLM-powered compression:
 *   • Recent window (last N turns verbatim, short-term working memory)
 *   • Mid-term summaries (older messages summarized by LLM, stored as compact notes)
 *   • Milestones (key decisions/files/errors marked as permanent anchors)
 *
 * LLM compaction is driven by main.js (_compactHistoryIfHuge); rule-based fallback
 * fires only as a safety net at a high threshold.
 */

const RECENT_WINDOW = 100;    // Safety-net auto-compress threshold (LLM compact handles it earlier)
const SUMMARY_BATCH = 10;
const MAX_SUMMARIES = 8;

export class ConversationMemory {
  constructor() {
    this.totalTurns = 0;
    this.recent = [];           // Last N messages (verbatim)
    this.summaries = [];        // Mid-term compressed batches [{range, text}]
    this.milestones = [];       // Permanent anchors [{turn, event}]
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

  assemble() {
    const result = [];
    if (this.milestones.length > 0) {
      const text = this.milestones
        .map(m => `[Turn ${m.turn}] ${m.event}`)
        .join('\n');
      result.push({ role: 'system', content: `📌 Key milestones from earlier:\n${text}` });
    }
    for (const s of this.summaries) {
      result.push({ role: 'assistant', content: `[Summary of ${s.range}] ${s.text}` });
    }
    result.push(...this.recent);
    return result;
  }

  /** Estimate total tokens in the recent window. CJK ≈ 1 tok, ASCII ≈ 0.25 tok. */
  estimateRecentTokens() {
    let t = 0;
    for (const m of this.recent) {
      const c = typeof m.content === 'string' ? m.content : '';
      for (let i = 0; i < c.length; i++) t += c.charCodeAt(i) > 0x2E7F ? 1 : 0.25;
    }
    return Math.ceil(t);
  }

  /** Replace the oldest `count` messages in recent with an LLM-generated summary.
   *  Returns the removed messages. */
  compactRecent(count, summaryText) {
    if (count <= 0 || count >= this.recent.length) return [];
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
    const range = `turns ${startTurn}-${endTurn}`;
    const summaryText = this._summarizeBatch(batch);
    this.summaries.push({ range, text: summaryText });
    if (this.summaries.length > MAX_SUMMARIES) {
      const first = this.summaries.shift();
      const second = this.summaries.shift();
      this.summaries.unshift({
        range: `${first.range} + ${second.range}`,
        text: `${first.text}; ${second.text}`
      });
    }
  }

  _summarizeBatch(batch) {
    const actions = new Set();
    const files = new Set();
    const fixes = [];
    const userRequests = [];
    for (const msg of batch) {
      if (msg.tool_calls) {
        for (const tc of msg.tool_calls) {
          const name = tc.function?.name;
          if (name) {
            actions.add(name);
            if (['write_file', 'edit_file', 'read_file'].includes(name)) {
              try {
                const args = JSON.parse(tc.function.arguments || '{}');
                if (args.path) files.add(args.path);
              } catch {}
            }
          }
        }
      }
      if (msg.role === 'user' && msg.content) {
        const line = String(msg.content).split('\n')[0].slice(0, 120);
        if (line.length > 5) userRequests.push(line);
      }
      if (msg.role === 'assistant' && /fixed|resolved|修复|已修复|解决|完成/i.test(msg.content)) {
        const firstLine = msg.content.split('\n')[0];
        if (firstLine.length < 150) fixes.push(firstLine);
      }
    }
    const parts = [];
    if (userRequests.length > 0) parts.push(`User asked: ${userRequests.join('; ')}`);
    if (actions.size > 0) parts.push(`Actions: ${[...actions].join(', ')}`);
    if (files.size > 0) parts.push(`Files: ${[...files].join(', ')}`);
    if (fixes.length > 0) parts.push(`Outcomes: ${fixes.join('; ')}`);
    return parts.length > 0 ? parts.join(' | ') : 'Continued conversation.';
  }

  stats() {
    return {
      totalTurns: this.totalTurns,
      recentCount: this.recent.length,
      summaryCount: this.summaries.length,
      milestoneCount: this.milestones.length,
      recentTokens: this.estimateRecentTokens()
    };
  }

  toJSON() {
    return {
      totalTurns: this.totalTurns,
      recent: this.recent,
      summaries: this.summaries,
      milestones: this.milestones
    };
  }

  static fromJSON(obj) {
    const mem = new ConversationMemory();
    if (obj) {
      mem.totalTurns = obj.totalTurns || 0;
      mem.recent = obj.recent || [];
      mem.summaries = obj.summaries || [];
      mem.milestones = obj.milestones || [];
    }
    return mem;
  }
}
