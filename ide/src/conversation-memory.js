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
const PERSISTED_MEDIA_BUDGET = 3_200_000;
const MAX_ARCHIVE = 400;          // archived (compacted-away) turns kept for recall
const ARCHIVE_ENTRY_MAX = 1600;   // chars per archived turn
const MAX_MILESTONES = 60;
const MERGED_SUMMARY_MAX = 8000;  // merged summary text cap (head+tail keep)

// Tokenizer for archival search: latin words + CJK bigrams (MemGPT-style
// archival memory needs retrieval that works for Chinese without a segmenter).
function memSearchTokens(s) {
  const lower = String(s || '').toLowerCase();
  const out = new Set();
  for (const w of lower.match(/[a-z0-9_$./-]{2,}/g) || []) out.add(w);
  for (const run of lower.match(/[\u3400-\u9fff]+/g) || []) {
    if (run.length === 1) out.add(run);
    for (let i = 0; i < run.length - 1; i++) out.add(run.slice(i, i + 2));
  }
  return [...out].slice(0, 64);
}

function serializeLocationEvidence(evidence) {
  if (!evidence || typeof evidence !== 'object') return undefined;
  const latitude = evidence.latitude;
  const longitude = evidence.longitude;
  const hasCoordinates = typeof latitude === 'number' && Number.isFinite(latitude) && latitude >= -90 && latitude <= 90
    && typeof longitude === 'number' && Number.isFinite(longitude) && longitude >= -180 && longitude <= 180;
  const statuses = new Set([
    'embedded_location_absent',
    'embedded_location_unreadable',
    'embedded_gps',
    'embedded_gps_resolved',
    'embedded_gps_unresolved',
    'embedded_gps_reverse_failed',
  ]);
  const status = statuses.has(evidence.status) ? evidence.status : (hasCoordinates ? 'embedded_gps' : 'embedded_location_absent');
  const cleanText = (value, max = 500) => typeof value === 'string' ? value.slice(0, max) : null;
  const candidate = (value) => {
    if (!value || typeof value !== 'object') return null;
    const out = {};
    for (const key of ['source', 'label', 'house_number', 'road', 'neighborhood', 'suburb', 'city_district', 'city', 'state', 'country', 'country_code']) {
      const text = cleanText(value[key]);
      if (text) out[key] = text;
    }
    const lat = value.latitude, lon = value.longitude;
    if (typeof lat === 'number' && Number.isFinite(lat) && lat >= -90 && lat <= 90) out.latitude = lat;
    if (typeof lon === 'number' && Number.isFinite(lon) && lon >= -180 && lon <= 180) out.longitude = lon;
    return Object.keys(out).length ? out : null;
  };
  const out = {
    status,
    coordinateSource: hasCoordinates ? 'embedded_exif_gps' : null,
    metadataAuthenticity: 'not_verified',
  };
  if (['source_file_changed', 'missing_source_fingerprint'].includes(evidence.invalidatedReason)) {
    out.invalidatedReason = evidence.invalidatedReason;
  }
  if (hasCoordinates) {
    out.latitude = latitude;
    out.longitude = longitude;
    const accuracy = evidence.reportedAccuracyM;
    out.reportedAccuracyM = typeof accuracy === 'number' && Number.isFinite(accuracy) && accuracy >= 0 ? accuracy : null;
  }
  out.reverseGeocoding = (Array.isArray(evidence.reverseGeocoding) ? evidence.reverseGeocoding : [])
    .slice(0, 4).map(candidate).filter(Boolean);
  out.sourceStatuses = (Array.isArray(evidence.sourceStatuses) ? evidence.sourceStatuses : [])
    .slice(0, 4).map((value) => ({
      source: cleanText(value?.source, 120) || 'unknown',
      status: cleanText(value?.status, 40) || 'unknown',
      detail: cleanText(value?.detail, 500) || '',
    }));
  out.retrievedAt = typeof evidence.retrievedAt === 'number' && Number.isFinite(evidence.retrievedAt) ? evidence.retrievedAt : null;
  out.limitations = (Array.isArray(evidence.limitations) ? evidence.limitations : [])
    .slice(0, 10).map((value) => cleanText(value, 600)).filter(Boolean);
  return out;
}

// Keep enough recent visual context to survive a restart without letting base64
// media overflow localStorage. Raw videos are never persisted; their compressed
// key frames (or a stable local path) are the durable representation.
function serializeAttachment(attachment, budget) {
  if (!attachment || typeof attachment !== 'object') return null;
  const kind = attachment.kind === 'video' ? 'video' : 'image';
  let budgetOmitted = 0;
  const out = {
    id: String(attachment.id || '').slice(0, 160),
    kind,
    mime: String(attachment.mime || attachment.type || (kind === 'video' ? 'video/mp4' : 'image/png')).slice(0, 120),
    name: String(attachment.name || (kind === 'video' ? 'video' : 'image')).slice(0, 240),
    path: String(attachment.path || '').slice(0, 2048),
    sourceFingerprint: typeof attachment.sourceFingerprint === 'string'
      ? attachment.sourceFingerprint.slice(0, 120) : '',
    visionText: String(attachment.visionText || '').slice(0, 6000),
    locationVisionText: String(attachment.locationVisionText || '').slice(0, 6000),
    modelMediaSanitized: attachment.modelMediaSanitized === true
      ? true
      : attachment.modelMediaSanitized === false ? false : undefined,
    mediaSourceChanged: attachment.mediaSourceChanged === true,
    locationEvidence: serializeLocationEvidence(attachment.locationEvidence),
    frames: [],
  };
  const keepDataUrl = (value, prefix) => {
    const data = typeof value === 'string' && value.startsWith(prefix) ? value : '';
    if (!data) return '';
    if (data.length > budget.remaining) {
      budgetOmitted++;
      return '';
    }
    budget.remaining -= data.length;
    return data;
  };
  if (kind === 'image') {
    const dataUrl = keepDataUrl(attachment.dataUrl, 'data:image/');
    if (dataUrl) out.dataUrl = dataUrl;
  }
  for (const frame of Array.isArray(attachment.frames) ? attachment.frames.slice(0, 4) : []) {
    const data = keepDataUrl(frame, 'data:image/');
    if (data) out.frames.push(data);
  }
  const priorOmitted = !!attachment.omitted;
  const hasDurableMedia = !!(out.path || out.dataUrl || out.frames.length);
  if (budgetOmitted || priorOmitted || !hasDurableMedia) {
    out.omitted = true;
    out.omittedReason = budgetOmitted
      ? 'persistence_media_budget'
      : String(attachment.omittedReason || (kind === 'video' ? 'raw_video_not_persisted' : 'media_unavailable')).slice(0, 120);
    out.omittedCount = Math.max(1, Number(attachment.omittedCount) || 0, budgetOmitted);
  }
  return out;
}

export function serializeMessagesForPersistence(messages, mediaBudget = PERSISTED_MEDIA_BUDGET) {
  const list = Array.isArray(messages) ? messages : [];
  // Callers serializing several queues or sessions may pass one shared budget
  // object so the aggregate media payload stays bounded.
  const budget = mediaBudget && typeof mediaBudget === 'object'
    ? mediaBudget
    : { remaining: Math.max(0, Number(mediaBudget) || 0) };
  budget.remaining = Math.max(0, Number(budget.remaining) || 0);
  const out = new Array(list.length);
  // Spend the bounded media budget on the newest turns first.
  for (let i = list.length - 1; i >= 0; i--) {
    const message = list[i] && typeof list[i] === 'object' ? { ...list[i] } : list[i];
    if (message && Array.isArray(message.attachments)) {
      message.attachments = message.attachments
        .map((attachment) => serializeAttachment(attachment, budget))
        .filter(Boolean);
    }
    out[i] = message;
  }
  return out;
}

export class ConversationMemory {
  constructor() {
    this.totalTurns = 0;
    this.recent = [];
    this.summaries = [];
    this.milestones = [];
    this.fileEvidence = [];
    // Archival memory (MemGPT/Letta-style): compacted-away turns keep a bounded
    // text-only record here, searchable via searchArchive() / the recall tool,
    // so compression no longer means permanent loss of early details.
    this.archive = [];
    this._onMessagesRemoved = null;
  }

  setRemovalHandler(handler) {
    this._onMessagesRemoved = typeof handler === 'function' ? handler : null;
  }

  _notifyMessagesRemoved(messages) {
    if (!messages?.length || !this._onMessagesRemoved) return;
    try { this._onMessagesRemoved(messages); } catch {}
  }

  push(msg) {
    this.totalTurns++;
    this.recent.push(msg);
    // Adaptive compression: by count (long chats of short turns) OR by token
    // pressure (few turns but huge tool outputs). Threshold sits ABOVE the LLM
    // auto-compact trigger (~28k in main.js) so the smart compactor gets first
    // shot; this is the mechanical safety net.
    if (this.recent.length > RECENT_WINDOW
        || (this.recent.length >= SUMMARY_BATCH + 6 && this.estimateRecentTokens() > 48000)) {
      this._compressOldestBatch();
    }
  }

  markMilestone(event) {
    this.milestones.push({ turn: this.totalTurns, event });
    if (this.milestones.length > MAX_MILESTONES) {
      // Keep the earliest few (project framing) + the most recent ones.
      this.milestones = [...this.milestones.slice(0, 8), ...this.milestones.slice(-(MAX_MILESTONES - 8))];
    }
  }

  _archiveBatch(removed, startTurn) {
    for (let i = 0; i < removed.length; i++) {
      const msg = removed[i];
      const role = msg?.role === 'assistant' ? 'assistant' : msg?.role === 'user' ? 'user' : msg?.role === 'tool' ? 'tool' : 'system';
      let text = typeof msg?.content === 'string' ? msg.content.replace(/\s+/g, ' ').trim() : '';
      if (msg?.tool_calls?.length) {
        const names = msg.tool_calls.map((tc) => tc?.function?.name).filter(Boolean).join(', ');
        if (names) text = (text ? text + ' ' : '') + `[调用工具: ${names}]`;
      }
      if (!text) continue;
      this.archive.push({ turn: startTurn + i, role, text: text.slice(0, role === 'tool' ? 700 : ARCHIVE_ENTRY_MAX) });
    }
    if (this.archive.length > MAX_ARCHIVE) this.archive.splice(0, this.archive.length - MAX_ARCHIVE);
  }

  /** Keyword search over archived (compacted-away) turns. Returns newest-biased
   *  best matches: [{turn, role, text}]. */
  searchArchive(query, k = 6) {
    const q = String(query || '').trim().toLowerCase();
    if (!q || !this.archive.length) return [];
    const terms = memSearchTokens(q);
    if (!terms.length) return [];
    const scored = [];
    for (let i = 0; i < this.archive.length; i++) {
      const entry = this.archive[i];
      const text = entry.text.toLowerCase();
      let score = 0;
      for (const t of terms) {
        let idx = 0, n = 0;
        while (n < 8 && (idx = text.indexOf(t, idx)) !== -1) { n++; idx += t.length; }
        score += n * Math.max(1, t.length);
      }
      if (text.includes(q)) score += q.length * 4; // exact-phrase bonus
      if (score > 0) scored.push({ entry, i, score });
    }
    scored.sort((a, b) => b.score - a.score || b.i - a.i);
    return scored.slice(0, Math.max(1, Math.min(20, k))).map((s) => ({ ...s.entry }));
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
      const recallHint = this.archive.length
        ? '\n\n（以上是早期对话的压缩摘要；需要某段早期对话的原文细节时，用 recall_conversation 工具按关键词检索归档）'
        : '';
      result.push({ role: 'assistant', content: `[对话上下文摘要]\n${merged}${recallHint}` });
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
    this._archiveBatch(removed, startTurn);
    this._notifyMessagesRemoved(removed);
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
        text: this._mergeSummaryText(a.text, b.text)
      });
    }
    return removed;
  }

  // Merged summaries used to grow without bound (plain concatenation). Cap with a
  // head+tail keep so the oldest framing and the newest facts both survive.
  _mergeSummaryText(a, b) {
    const merged = `${a}\n${b}`;
    if (merged.length <= MERGED_SUMMARY_MAX) return merged;
    const head = merged.slice(0, Math.floor(MERGED_SUMMARY_MAX * 0.6));
    const tail = merged.slice(-Math.floor(MERGED_SUMMARY_MAX * 0.35));
    return `${head}\n…（更早摘要已折叠，原文可用 recall_conversation 检索）…\n${tail}`;
  }

  _compressOldestBatch() {
    if (this.recent.length < SUMMARY_BATCH) return;
    const batch = this.recent.splice(0, SUMMARY_BATCH);
    const startTurn = this.totalTurns - this.recent.length - batch.length + 1;
    this._archiveBatch(batch, startTurn);
    this._notifyMessagesRemoved(batch);
    const endTurn = startTurn + batch.length - 1;
    this.summaries.push({ range: `turns ${startTurn}-${endTurn}`, text: this._summarizeBatch(batch) });
    if (this.summaries.length > MAX_SUMMARIES) {
      const a = this.summaries.shift();
      const b = this.summaries.shift();
      this.summaries.unshift({ range: `${a.range} + ${b.range}`, text: this._mergeSummaryText(a.text, b.text) });
    }
  }

  _summarizeBatch(batch) {
    const actions = new Set(), files = new Set(), lines = [];
    const clean = (value, max) => String(value || "")
      .replace(/\s+/g, " ")
      .trim()
      .slice(0, max);
    for (const msg of batch) {
      if (msg.tool_calls) for (const tc of msg.tool_calls) {
        const n = tc.function?.name;
        if (n) {
          actions.add(n);
          if (['write_file','edit_file','read_file'].includes(n)) {
            try {
              const a = JSON.parse(tc.function.arguments || '{}');
              if (a.path) files.add(String(a.path));
            } catch {}
          }
        }
      }
      const role = msg?.role === 'assistant' ? 'assistant'
        : msg?.role === 'user' ? 'user'
        : msg?.role === 'tool' ? 'tool'
        : 'system';
      const text = clean(msg?.content, role === 'user' ? 700 : role === 'assistant' ? 520 : 320);
      if (text) lines.push(`[${role}] ${text}`);
      const attachments = Array.isArray(msg?.attachments) ? msg.attachments : [];
      if (attachments.length) {
        const kinds = attachments.map((a) => a?.kind || "media").slice(0, 4).join(", ");
        lines.push(`[attachments] ${attachments.length} item(s): ${kinds}`);
      }
    }
    const p = [];
    if (lines.length) p.push(lines.join('\n'));
    if (actions.size) p.push(`Actions: ${[...actions].join(', ')}`);
    if (files.size) p.push(`Files: ${[...files].join(', ')}`);
    return p.length ? p.join('\n') : 'Continued conversation.';
  }

  stats() {
    return { totalTurns: this.totalTurns, recentCount: this.recent.length, summaryCount: this.summaries.length, milestoneCount: this.milestones.length, archiveCount: this.archive.length, recentTokens: this.estimateRecentTokens() };
  }

  toJSON(mediaBudget = PERSISTED_MEDIA_BUDGET) {
    // JSON.stringify calls toJSON with a string property key. Treat only an
    // explicit number/shared-budget object as the serializer override.
    const effectiveBudget = typeof mediaBudget === 'number' || (mediaBudget && typeof mediaBudget === 'object')
      ? mediaBudget
      : PERSISTED_MEDIA_BUDGET;
    return { totalTurns: this.totalTurns, recent: serializeMessagesForPersistence(this.recent, effectiveBudget), summaries: this.summaries, milestones: this.milestones, fileEvidence: this.fileEvidence, archive: this.archive };
  }

  static fromJSON(obj) {
    const mem = new ConversationMemory();
    if (obj) {
      mem.totalTurns = obj.totalTurns || 0;
      mem.recent = Array.isArray(obj.recent) ? obj.recent : [];
      mem.summaries = obj.summaries || [];
      mem.milestones = obj.milestones || [];
      mem.fileEvidence = Array.isArray(obj.fileEvidence) ? obj.fileEvidence.slice(-80) : [];
      mem.archive = Array.isArray(obj.archive)
        ? obj.archive.slice(-MAX_ARCHIVE)
          .filter((e) => e && typeof e.text === 'string' && e.text)
          .map((e) => ({ turn: Math.max(0, Number(e.turn) || 0), role: String(e.role || 'assistant'), text: String(e.text).slice(0, ARCHIVE_ENTRY_MAX) }))
        : [];
    }
    return mem;
  }
}
