/**
 * Hierarchical conversation memory — prevents context overflow in ultra-long sessions
 * while preserving critical decisions/files/errors through three-tier compression:
 *   • Recent window (last 20 turns verbatim, short-term working memory)
 *   • Mid-term summaries (older messages grouped & summarized every 10 turns)
 *   • Milestones (key decisions/files/errors marked as permanent anchors)
 *
 * Token budget stays bounded while the agent can reference turn-1 decisions at turn 100+.
 */

const RECENT_WINDOW = 20;     // Keep last N turns verbatim
const SUMMARY_BATCH = 10;      // Group older messages into summaries every K turns
const MAX_SUMMARIES = 5;       // Cap mid-term memory (oldest summaries get merged)

export class ConversationMemory {
  constructor() {
    this.totalTurns = 0;
    this.recent = [];           // Last RECENT_WINDOW messages (verbatim)
    this.summaries = [];        // Mid-term compressed batches [{range, text}]
    this.milestones = [];       // Permanent anchors [{turn, event}]
  }

  /** Add a new message. Auto-compress when recent window overflows. */
  push(msg) {
    this.totalTurns++;
    this.recent.push(msg);

    // When recent overflows, compress oldest batch into summary
    if (this.recent.length > RECENT_WINDOW) {
      this._compressOldestBatch();
    }
  }

  /** Mark a milestone (user says "记住这个" or agent detects breakthrough). */
  markMilestone(event) {
    this.milestones.push({ turn: this.totalTurns, event });
  }

  /** Assemble full context: milestones + summaries + recent. */
  assemble() {
    const result = [];

    // 1) Inject milestones as synthetic system message (permanent anchors)
    if (this.milestones.length > 0) {
      const text = this.milestones
        .map(m => `[Turn ${m.turn}] ${m.event}`)
        .join('\n');
      result.push({
        role: 'system',
        content: `📌 Key milestones from earlier:\n${text}`
      });
    }

    // 2) Inject mid-term summaries as synthetic assistant messages
    for (const s of this.summaries) {
      result.push({
        role: 'assistant',
        content: `[Summary of ${s.range}] ${s.text}`
      });
    }

    // 3) Append recent verbatim messages
    result.push(...this.recent);

    return result;
  }

  /** Compress oldest SUMMARY_BATCH messages from recent into a summary. */
  _compressOldestBatch() {
    if (this.recent.length < SUMMARY_BATCH) return;

    const batch = this.recent.splice(0, SUMMARY_BATCH);
    const startTurn = this.totalTurns - this.recent.length - batch.length;
    const endTurn = startTurn + batch.length - 1;
    const range = `turns ${startTurn}-${endTurn}`;

    const summaryText = this._summarizeBatch(batch);

    this.summaries.push({ range, text: summaryText });

    // If too many summaries, merge the two oldest
    if (this.summaries.length > MAX_SUMMARIES) {
      const first = this.summaries.shift();
      const second = this.summaries.shift();
      const merged = {
        range: `${first.range} + ${second.range}`,
        text: `${first.text}; ${second.text}`
      };
      this.summaries.unshift(merged);
    }
  }

  /** Rule-based summarization: extract key actions/files/errors. */
  _summarizeBatch(batch) {
    const actions = new Set();
    const files = new Set();
    const fixes = [];

    for (const msg of batch) {
      // Extract tool calls (files read/written, commands run)
      if (msg.tool_calls) {
        for (const tc of msg.tool_calls) {
          const name = tc.function?.name;
          if (name) {
            actions.add(name);
            // Extract file paths from write_file/edit_file
            if (['write_file', 'edit_file', 'read_file'].includes(name)) {
              try {
                const args = JSON.parse(tc.function.arguments || '{}');
                if (args.path) files.add(args.path);
              } catch {}
            }
          }
        }
      }

      // Detect error fixes (heuristic: messages mentioning "fixed", "resolved", "修复")
      if (msg.role === 'assistant' &&
          /fixed|resolved|修复|已修复|解决/i.test(msg.content)) {
        const firstLine = msg.content.split('\n')[0];
        if (firstLine.length < 150) fixes.push(firstLine);
      }
    }

    const parts = [];
    if (actions.size > 0) parts.push(`Actions: ${[...actions].join(', ')}`);
    if (files.size > 0) parts.push(`Files: ${[...files].join(', ')}`);
    if (fixes.length > 0) parts.push(`Fixes: ${fixes.join('; ')}`);

    return parts.length > 0 ? parts.join(' | ') : 'Continued conversation.';
  }

  /** Get current stats (for debugging/UI display). */
  stats() {
    return {
      totalTurns: this.totalTurns,
      recentCount: this.recent.length,
      summaryCount: this.summaries.length,
      milestoneCount: this.milestones.length
    };
  }

  /** Serialize to plain object (for localStorage persistence). */
  toJSON() {
    return {
      totalTurns: this.totalTurns,
      recent: this.recent,
      summaries: this.summaries,
      milestones: this.milestones
    };
  }

  /** Restore from plain object (from localStorage). */
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
