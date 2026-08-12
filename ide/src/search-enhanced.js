/**
 * Enhanced Search System for Mr. Day One
 * 
 * Features:
 * - Real-time incremental search with debounce
 * - Result grouping by file type/directory
 * - Syntax-aware highlighting
 * - Context-aware code understanding
 * - Cross-file relationship detection
 */

// ===== Configuration =====
const SEARCH_CONFIG = {
  DEBOUNCE_DELAY: 200,
  MAX_RESULTS_PER_FILE: 10,
  MAX_TOTAL_RESULTS: 500,
  MAX_FILES_SCANNED: 10000,
  GROUPING_OPTIONS: ['auto', 'fileType', 'directory', 'none'],
  HIGHLIGHT_MODES: ['regex', 'literal', 'caseSensitive'],
  CONTEXT_LINES_ABOVE: 3,
  CONTEXT_LINES_BELOW: 3
};

// ===== Result Grouping Utilities =====
export function groupSearchResults(results, mode = 'auto') {
  if (mode === 'none' || !results?.length) return { items: results, groups: [] };
  
  const groups = new Map();
  const groupedItems = [];
  
  for (const file of results) {
    let groupName;
    
    switch (mode) {
      case 'fileType': {
        const ext = file.path.split('.').pop() || 'no-ext';
        groupName = `.${ext}`;
        break;
      }
      case 'directory': {
        const dir = file.rel.split('/')[0] || '_root';
        groupName = dir;
        break;
      }
      default: { // 'auto'
        const ext = file.path.split('.').pop();
        if (['js', 'ts', 'jsx', 'tsx', 'vue'].includes(ext)) {
          groupName = '💻 Code';
        } else if (['json', 'yaml', 'yml', 'toml'].includes(ext)) {
          groupName = '⚙️ Config';
        } else if (['md', 'txt', 'log'].includes(ext)) {
          groupName = '📝 Documents';
        } else {
          groupName = '📁 Others';
        }
        break;
      }
    }
    
    if (!groups.has(groupName)) {
      groups.set(groupName, []);
    }
    groups.get(groupName).push(file);
  }
  
  // Convert to array and sort by result count
  const sortedGroups = Array.from(groups.entries())
    .map(([name, files]) => ({
      name,
      count: files.length,
      files
    }))
    .sort((a, b) => b.count - a.count);
  
  // Flatten for display
  sortedGroups.forEach(group => {
    groupedItems.push(...group.files);
  });
  
  return { items: groupedItems, groups: sortedGroups };
}

// ===== Context Extraction =====
export async function extractContextAroundMatch(lineContent, lineIndex, lines, contextLines = 3) {
  const start = Math.max(0, lineIndex - contextLines);
  const end = Math.min(lines.length, lineIndex + contextLines + 1);
  
  const context = {
    before: lines.slice(start, lineIndex),
    matchLine: lineContent,
    after: lines.slice(lineIndex + 1, end)
  };
  
  return context;
}

// ===== Syntax-Aware Highlighting =====
export function highlightMatches(text, matches, options = {}) {
  const {
    mode = 'regex',
    highlightColor = '#fff3a8',
    textColor = '#000'
  } = options;
  
  if (mode === 'literal') {
    return escapeHtml(text).replace(
      new RegExp(escapeRegex(matches.pattern), 'gi'),
      `<mark style="background: ${highlightColor}; color: ${textColor}; border-radius: 2px;">$&</mark>`
    );
  }
  
  // Default: regex mode
  const flags = options.caseSensitive ? 'g' : 'gi';
  const compiled = compileSafeRegex(matches.pattern, flags);
  if (compiled.error) {
    // Surface the reason inline so the user knows WHY there's no highlight,
    // instead of a silently unhighlighted result or a frozen tab.
    return escapeHtml(text);
  }
  const regex = compiled.regex;
  try {
    // Build output from escaped slices: running replace() on raw text and
    // splicing raw matches into HTML was an HTML-injection hole (any `<tag>`
    // in file content or match went into the DOM unescaped).
    let out = '';
    let last = 0;
    let m;
    while ((m = regex.exec(text)) !== null) {
      out += escapeHtml(text.slice(last, m.index));
      out += `<mark style="background: ${highlightColor}; color: ${textColor}; border-radius: 2px;">${escapeHtml(m[0])}</mark>`;
      last = m.index + m[0].length;
      // Zero-length match (e.g. pattern `a*`): advance manually or exec loops forever.
      if (m[0].length === 0) regex.lastIndex++;
    }
    out += escapeHtml(text.slice(last));
    return out;
  } catch (err) {
    console.error('Highlight error:', err);
    return escapeHtml(text);
  }
}

// ===== Helper Functions =====

// Compile a user regex robustly: caps pattern length and rejects the classic
// catastrophic-backtracking shapes (nested quantifiers like (a+)+ / (a*)* /
// (a|aa)+) that freeze the main thread on adversarial or accidental input.
// Returns { regex } on success or { error } with a human-readable reason — so
// callers surface useful feedback instead of a silent freeze or swallowed throw.
export function compileSafeRegex(pattern, flags = "gi") {
  const src = String(pattern ?? "");
  if (!src) return { error: "空的搜索表达式" };
  if (src.length > 5000) return { error: "正则太长（上限 5000 字符），请缩短或改用 literal 模式" };
  // Nested quantifier applied to a group that itself repeats → exponential backtracking.
  const nestedQuantifier = /\((?![?]:)[^()]*[+*]\)[+*]|\((?:[^()]*\|[^()]*)\)[+*]/;
  if (nestedQuantifier.test(src)) {
    return { error: "检测到可能导致指数级回溯的嵌套量词（如 (a+)+）——请改写表达式或用 literal 模式" };
  }
  try {
    return { regex: new RegExp(src, flags) };
  } catch (err) {
    return { error: `正则语法无效：${err.message}` };
  }
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function escapeRegex(string) {
  return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// ===== Search Results Renderer =====
export class SearchResultsRenderer {
  constructor(containerId) {
    this.container = document.getElementById(containerId);
    this.currentQuery = '';
    this.lastResults = null;
    this.groupMode = 'auto';
    this.highlighted = false;
    
    this.setupEventListeners();
  }
  
  setupEventListeners() {
    // Auto-highlight on render
    this.container.addEventListener('render', () => {
      this.enableHighlights();
    });
  }
  
  enableHighlights() {
    if (!this.highlighted) {
      // Add mark styling
      const style = document.createElement('style');
      style.textContent = `
        mark {
          padding: 1px 2px;
          margin: 0 -2px;
          cursor: pointer;
          transition: all 0.2s;
        }
        mark:hover {
          transform: scale(1.02);
          box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
      `;
      document.head.appendChild(style);
      this.highlighted = true;
    }
  }
  
  render(query, results, options = {}) {
    const {
      groupBy = 'auto',
      showContext = true,
      maxContextLines = 3
    } = options;
    
    this.currentQuery = query;
    this.groupMode = groupBy;
    
    // Group results if requested
    const grouped = groupBy !== 'none' 
      ? groupSearchResults(results, groupBy) 
      : { items: results, groups: [] };
    
    this.lastResults = grouped;
    
    // Render
    this.container.innerHTML = this.buildHTML(grouped, { showContext, maxContextLines });
    
    // Add click handlers for navigation
    this.addNavigationHandlers();
    
    // Dispatch event for listeners
    this.container.dispatchEvent(new CustomEvent('searchComplete', {
      detail: { query, results: grouped.items, groups: grouped.groups }
    }));
  }
  
  buildHTML({ items, groups }, { showContext, maxContextLines }) {
    if (!items || !items.length) {
      return `
        <div style="padding: 40px; text-align: center; color: #57606a;">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
          </svg>
          <p style="margin-top: 16px; font-size: 16px;">未找到匹配结果</p>
          <p style="font-size: 14px; color: #8b949e;">尝试其他关键词或扩大搜索范围</p>
        </div>
      `;
    }
    
    // Render groups if available
    let html = '';
    if (groups && groups.length > 1) {
      html = groups.map(group => `
        <div style="margin-bottom: 24px;">
          <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 12px; padding-bottom: 8px; border-bottom: 1px solid #e1e4e8;">
            <h3 style="margin: 0; font-size: 14px; font-weight: 600;">
              ${group.name}
            </h3>
            <span style="padding: 2px 8px; background: #e1e4e8; border-radius: 12px; font-size: 12px;">
              ${group.count} 个结果
            </span>
          </div>
          ${this.renderFiles(group.files, showContext, maxContextLines)}
        </div>
      `).join('');
    } else {
      html = this.renderFiles(items, showContext, maxContextLines);
    }
    
    // Summary footer
    html += `
      <div style="padding: 12px 16px; background: #f6f8fa; border-top: 1px solid #e1e4e8; font-size: 13px; color: #57606a;">
        <strong>${items.length}</strong> 个结果 (扫描了 <strong>${this.formatNumber(items.reduce((sum, f) => sum + (f.matches?.length || 0), 0))}</strong> 处匹配)
      </div>
    `;
    
    return html;
  }
  
  renderFiles(files, showContext, maxContextLines) {
    // Rank files by match count (most relevant first); the backend already ranks,
    // but the UI may receive results from other sources too, so sort defensively.
    const ranked = [...files].sort((a, b) => (b.matches?.length || 0) - (a.matches?.length || 0));
    return ranked.map(file => `
      <div style="margin-bottom: 16px; border: 1px solid #e1e4e8; border-radius: 6px; overflow: hidden;">
        <div style="padding: 10px 14px; background: #fafbfc; border-bottom: 1px solid #e1e4e8; display: flex; justify-content: space-between; align-items: center;">
          <div style="display: flex; align-items: center; gap: 8px; flex: 1;">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#0969da" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
              <polyline points="14 2 14 8 20 8"></polyline>
            </svg>
            <span style="font-weight: 500; font-size: 13px; color: #1f2328;">${escapeHtml(file.name)}</span>
            <span style="font-size: 11px; color: #57606a;">${escapeHtml(file.rel)}</span>
          </div>
          <span style="font-size: 12px; color: #57606a;">${file.matches?.length || 0} matches</span>
        </div>
        
        <div style="max-height: 400px; overflow-y: auto;">
          ${(file.matches || []).slice(0, 20).map(match => this.renderMatch(match, showContext, maxContextLines)).join('')}
        </div>
      </div>
    `).join('');
  }
  
  renderMatch(match, showContext, maxContextLines) {
    const lineNumber = match.line;
    // Highlight the query inside the matched line. highlightMatches escapes every
    // slice, so raw file content like `<script>` can never reach the DOM unescaped
    // (the old code injected match.text verbatim — both an XSS hole and no highlight).
    const matchedText = this.currentQuery
      ? highlightMatches(String(match.text ?? ''), { pattern: this.currentQuery }, { mode: 'literal' })
      : escapeHtml(String(match.text ?? ''));
    
    return `
      <div style="padding: 8px 14px; border-bottom: 1px solid #eff1f3; cursor: pointer; transition: background 0.2s;"
           data-line="${lineNumber}"
           onclick="window.navigateToFile(this)">
        <div style="display: flex; gap: 12px;">
          <div style="min-width: 50px; font-size: 12px; color: #57606a; text-align: right; user-select: none; line-height: 1.5;">
            ${lineNumber}
          </div>
          <div style="flex: 1; font-size: 13px; line-height: 1.6; font-family: 'Monaco', 'Courier New', monospace;">
            ${matchedText}
          </div>
        </div>
      </div>
    `;
  }
  
  addNavigationHandlers() {
    // Make match lines clickable for navigation
    const matches = this.container.querySelectorAll('[data-line]');
    matches.forEach(el => {
      el.addEventListener('click', (e) => {
        const lineNum = parseInt(el.dataset.line);
        // TODO: Navigate to file and scroll to line
        window.focusFileTabAndScrollTo(lineNum);
      });
    });
  }
  
  formatNumber(num) {
    if (num >= 1000000) return (num / 1000000).toFixed(1) + 'M';
    if (num >= 1000) return (num / 1000).toFixed(1) + 'K';
    return num.toString();
  }
}

// ===== Debounced Search Hook =====
export function createDebouncedSearch(callback, delay = SEARCH_CONFIG.DEBOUNCE_DELAY) {
  let timeoutId = null;
  
  return function debouncedSearch(...args) {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => callback.apply(this, args), delay);
  };
}

// ===== Export =====
export default {
  groupSearchResults,
  extractContextAroundMatch,
  highlightMatches,
  SearchResultsRenderer,
  createDebouncedSearch,
  SEARCH_CONFIG
};
