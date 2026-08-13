/**
 * 把命令输出里的 ANSI 转义序列渲染成 HTML。
 *
 * 终端卡片以前是 `outEl.textContent = output`，于是任何带颜色的工具（cargo、pytest、
 * npm、eslint、docker）吐出来的 `ESC[31m` 都当成普通文字显示，一屏的 `[31m[1m`
 * 噪声；带进度条的命令（pip、curl、webpack）靠 `\r` 回车重写同一行，textContent 会
 * 把每一帧都留下来，几十行重复。这里两件事一起解决：
 *
 *   - SGR（颜色/加粗/下划线）→ 语义 class，颜色由 CSS 变量给，深浅色各一套；
 *   - 非 SGR 的 CSI / OSC 序列直接丢掉，`\r` 按真实的"回到行首覆盖写"处理。
 *
 * 输出是 HTML，所以文本必须经过 escapeHtml —— 这个模块是命令输出进入 innerHTML 的
 * 唯一入口，转义漏了就是一个注入点。
 */

import { escapeHtml } from "./escape.js";

/** xterm 256 色里 16-231 那段 6×6×6 立方的分量取值。 */
const CUBE = [0, 95, 135, 175, 215, 255];

/** 把 256 色索引转成 #rrggbb；0-15 不走这里（它们要用主题 class）。 */
export function xterm256Hex(n) {
  const i = Number(n);
  if (!Number.isInteger(i) || i < 16 || i > 255) return null;
  if (i < 232) {
    const k = i - 16;
    const r = CUBE[Math.floor(k / 36) % 6];
    const g = CUBE[Math.floor(k / 6) % 6];
    const b = CUBE[k % 6];
    return `#${[r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("")}`;
  }
  const v = 8 + (i - 232) * 10;
  const h = v.toString(16).padStart(2, "0");
  return `#${h}${h}${h}`;
}

const BASE_NAMES = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

function clampByte(v) {
  const n = Number(v);
  if (!Number.isFinite(n)) return 0;
  return Math.max(0, Math.min(255, Math.round(n)));
}

function rgbHex(r, g, b) {
  return `#${[r, g, b].map((v) => clampByte(v).toString(16).padStart(2, "0")).join("")}`;
}

/** 一次 SGR 参数序列 → 更新后的样式状态。 */
function applySgr(state, params) {
  // `ESC[m` 等价于 `ESC[0m`。
  const codes = params.length ? params : [0];
  for (let i = 0; i < codes.length; i++) {
    const c = codes[i];
    if (c === 0) {
      state.bold = false; state.dim = false; state.italic = false;
      state.underline = false; state.strike = false; state.inverse = false;
      state.fg = null; state.bg = null;
    } else if (c === 1) state.bold = true;
    else if (c === 2) state.dim = true;
    else if (c === 3) state.italic = true;
    else if (c === 4) state.underline = true;
    else if (c === 7) state.inverse = true;
    else if (c === 9) state.strike = true;
    else if (c === 21 || c === 22) { state.bold = false; state.dim = false; }
    else if (c === 23) state.italic = false;
    else if (c === 24) state.underline = false;
    else if (c === 27) state.inverse = false;
    else if (c === 29) state.strike = false;
    else if (c >= 30 && c <= 37) state.fg = { kind: "base", n: c - 30, bright: false };
    else if (c === 39) state.fg = null;
    else if (c >= 40 && c <= 47) state.bg = { kind: "base", n: c - 40, bright: false };
    else if (c === 49) state.bg = null;
    else if (c >= 90 && c <= 97) state.fg = { kind: "base", n: c - 90, bright: true };
    else if (c >= 100 && c <= 107) state.bg = { kind: "base", n: c - 100, bright: true };
    else if (c === 38 || c === 48) {
      // 38;5;N（256 色）/ 38;2;R;G;B（真彩）。参数不够就整条丢掉，不要把剩下的
      // 数字当成新的 SGR 码接着解释——那会渲染出一片本不存在的颜色。
      const target = c === 38 ? "fg" : "bg";
      const mode = codes[i + 1];
      if (mode === 5 && i + 2 < codes.length) {
        const n = codes[i + 2];
        state[target] = n >= 0 && n <= 15
          ? { kind: "base", n: n % 8, bright: n >= 8 }
          : { kind: "hex", hex: xterm256Hex(n) };
        if (state[target].kind === "hex" && !state[target].hex) state[target] = null;
        i += 2;
      } else if (mode === 2 && i + 4 < codes.length) {
        state[target] = { kind: "hex", hex: rgbHex(codes[i + 2], codes[i + 3], codes[i + 4]) };
        i += 4;
      } else {
        i = codes.length;
      }
    }
    // 其它 SGR 码（闪烁、字体切换等）忽略。
  }
  return state;
}

function styleKey(s) {
  const fg = s.fg ? (s.fg.kind === "hex" ? s.fg.hex : `b${s.fg.n}${s.fg.bright ? "+" : ""}`) : "";
  const bg = s.bg ? (s.bg.kind === "hex" ? s.bg.hex : `b${s.bg.n}${s.bg.bright ? "+" : ""}`) : "";
  return [fg, bg, s.bold, s.dim, s.italic, s.underline, s.strike, s.inverse].join("|");
}

function isPlain(s) {
  return styleKey(s) === "||false|false|false|false|false|false";
}

/** 当前状态 → `<span …>` 开标签。 */
function openTag(s) {
  const cls = [];
  const style = [];
  // inverse（`ESC[7m`）交换前景/背景，diff 工具和 grep --color 常用。
  const fg = s.inverse ? s.bg : s.fg;
  const bg = s.inverse ? s.fg : s.bg;
  const paint = (c, prefix) => {
    if (!c) {
      // 反显但另一侧没设色时，用卡片自己的前景/背景色顶上，不然反显看不出来。
      if (s.inverse) cls.push(`ansi-${prefix}-default`);
      return;
    }
    if (c.kind === "hex") style.push(`${prefix === "fg" ? "color" : "background-color"}:${c.hex}`);
    else cls.push(`ansi-${prefix}-${c.bright ? "b" : ""}${BASE_NAMES[c.n]}`);
  };
  paint(fg, "fg");
  paint(bg, "bg");
  if (s.bold) cls.push("ansi-bold");
  if (s.dim) cls.push("ansi-dim");
  if (s.italic) cls.push("ansi-italic");
  if (s.underline) cls.push("ansi-underline");
  if (s.strike) cls.push("ansi-strike");
  const attrs = [];
  if (cls.length) attrs.push(`class="${cls.join(" ")}"`);
  if (style.length) attrs.push(`style="${style.join(";")}"`);
  return `<span${attrs.length ? " " + attrs.join(" ") : ""}>`;
}

// CSI：ESC [ 参数 中间字符 最终字符。SGR 的最终字符是 `m`，其余（光标移动、清屏、
// 私有模式 `?25l` 之类）解析出来直接丢。
const CSI = /\x1b\[([0-9;:?]*)([ -\/]*)([@-~])/y;
// OSC：ESC ] … BEL 或 ESC \。设置窗口标题、iTerm 图片、超链接都走这里。
const OSC = /\x1b\][\s\S]*?(?:\x07|\x1b\\)/y;

/**
 * 解析成"行 × cell"。HTML 和纯文本两个渲染器共用这一次解析，免得纯文本靠正则把
 * 标签再剥回来——那样一段本身就含 `<div>` 的命令输出会被剥掉。
 *
 * @param {string} input 原始命令输出（可以含转义序列）
 * @param {{ maxChars?: number }} [opts] 超长时截断的字符数（按解析后的可见字符算）
 * @returns {{ lines: Array<Array<{c: string, s: object}>>, truncated: boolean }}
 */
function parseAnsi(input, opts = {}) {
  const text = String(input == null ? "" : input);
  const maxChars = Number.isInteger(opts.maxChars) ? opts.maxChars : Infinity;

  const state = {
    bold: false, dim: false, italic: false, underline: false,
    strike: false, inverse: false, fg: null, bg: null,
  };
  // 一行一行地攒：`\r` 要能回到行首覆盖写，所以行内必须是可寻址的 cell 数组，
  // 不能直接往输出字符串上追加。每个 cell 记录字符和它当时的样式。
  let line = [];
  let col = 0;
  let visible = 0;
  let truncated = false;
  const out = [];

  const flushLine = () => {
    out.push(line);
    line = [];
    col = 0;
  };

  let i = 0;
  while (i < text.length && !truncated) {
    const ch = text[i];
    if (ch === "\x1b") {
      CSI.lastIndex = i;
      const csi = CSI.exec(text);
      if (csi && csi.index === i) {
        if (csi[3] === "m" && !csi[1].includes("?")) {
          const params = csi[1]
            .split(";")
            .map((p) => (p === "" ? 0 : parseInt(p.split(":")[0], 10)))
            .map((n) => (Number.isFinite(n) ? n : 0));
          applySgr(state, params);
        }
        i = CSI.lastIndex;
        continue;
      }
      OSC.lastIndex = i;
      const osc = OSC.exec(text);
      if (osc && osc.index === i) {
        i = OSC.lastIndex;
        continue;
      }
      // 落单的 ESC（或者两字符序列如 ESC ( B）：吃掉 ESC 本身，别显示成 `←`。
      i += 1;
      continue;
    }
    if (ch === "\n") {
      flushLine();
      i += 1;
      continue;
    }
    if (ch === "\r") {
      // 回到行首。`\r\n` 是普通换行，别把上一行清掉。
      if (text[i + 1] === "\n") { flushLine(); i += 2; continue; }
      col = 0;
      i += 1;
      continue;
    }
    if (ch === "\b") {
      if (col > 0) col -= 1;
      i += 1;
      continue;
    }
    if (visible >= maxChars) { truncated = true; break; }
    // 覆盖写：col 落在已有内容上就替换，否则追加（中间空缺补空格）。
    while (line.length < col) line.push({ c: " ", s: { ...state } });
    line[col] = { c: ch, s: { ...state } };
    col += 1;
    visible += 1;
    i += 1;
  }
  flushLine();
  return { lines: out, truncated };
}

// 一个控制字符都没有 = 没什么可解析的。逐字符建 cell 会给每个字符分配一个对象，
// 2MB 的构建日志走全解析要几百毫秒；绝大多数命令输出是纯文本，直接短路掉。
const NEEDS_PARSE = /[\x1b\r\b]/;

/** ANSI 文本 → HTML 片段（每行之间一个 `\n`，靠 `white-space: pre-wrap` 排版）。 */
export function ansiToHtml(input, opts = {}) {
  const raw = String(input == null ? "" : input);
  if (!NEEDS_PARSE.test(raw)) {
    const max = Number.isInteger(opts.maxChars) ? opts.maxChars : Infinity;
    return raw.length > max
      ? escapeHtml(raw.slice(0, max)) + `<span class="ansi-dim">\n… 输出已截断</span>`
      : escapeHtml(raw);
  }
  const { lines, truncated } = parseAnsi(input, opts);
  const rendered = lines.map((line) => {
    let html = "";
    let openKey = null;
    for (const cell of line) {
      const key = styleKey(cell.s);
      if (key !== openKey) {
        if (openKey !== null) html += "</span>";
        openKey = isPlain(cell.s) ? null : key;
        if (openKey !== null) html += openTag(cell.s);
      }
      html += escapeHtml(cell.c);
    }
    if (openKey !== null) html += "</span>";
    return html;
  });
  let html = rendered.join("\n");
  if (truncated) html += `<span class="ansi-dim">\n… 输出已截断</span>`;
  return html;
}

/** 去掉转义序列、但保留 `\r` 覆盖语义的纯文本版（复制按钮、喂给模型时用）。 */
export function ansiToText(input, opts = {}) {
  const raw = String(input == null ? "" : input);
  if (!NEEDS_PARSE.test(raw)) {
    const max = Number.isInteger(opts.maxChars) ? opts.maxChars : Infinity;
    return raw.length > max ? raw.slice(0, max) + "\n… 输出已截断" : raw;
  }
  const { lines, truncated } = parseAnsi(input, opts);
  const text = lines.map((line) => line.map((cell) => cell.c).join("")).join("\n");
  return truncated ? text + "\n… 输出已截断" : text;
}
