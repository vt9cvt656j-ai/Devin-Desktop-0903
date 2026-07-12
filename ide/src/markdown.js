// Lightweight, dependency-free Markdown → DOM renderer for the AI assistant.
//
// Builds real DOM nodes (text is always set via textContent), so it is XSS-safe
// by construction — no innerHTML of model output, link hrefs are scheme-checked.
// Fenced code blocks render as Devin-style "cards" (language/filename header +
// copy button); an optional async `highlighter` paints syntax colors.

// ---- small DOM helpers ----
function el(tag, cls) {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  return n;
}
function txt(s) {
  return document.createTextNode(s);
}
function icon(id, cls = "") {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("class", "ic " + cls);
  const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
  use.setAttribute("href", "#" + id);
  svg.appendChild(use);
  return svg;
}

// ---- language display names + Monaco ids ----
const LANG_LABEL = {
  js: "JavaScript", javascript: "JavaScript", jsx: "JSX", mjs: "JavaScript",
  ts: "TypeScript", typescript: "TypeScript", tsx: "TSX",
  json: "JSON", html: "HTML", xml: "XML", css: "CSS", scss: "SCSS", less: "Less",
  md: "Markdown", markdown: "Markdown", rs: "Rust", rust: "Rust",
  py: "Python", python: "Python", go: "Go", java: "Java", c: "C", h: "C",
  cpp: "C++", "c++": "C++", cc: "C++", cs: "C#", csharp: "C#",
  rb: "Ruby", ruby: "Ruby", php: "PHP", swift: "Swift", kt: "Kotlin", kotlin: "Kotlin",
  sh: "Shell", bash: "Bash", zsh: "Shell", shell: "Shell", console: "Shell",
  yml: "YAML", yaml: "YAML", toml: "TOML", ini: "INI", sql: "SQL",
  dockerfile: "Dockerfile", diff: "Diff", text: "Text", txt: "Text", plaintext: "Text",
};
const LANG_MONACO = {
  js: "javascript", javascript: "javascript", jsx: "javascript", mjs: "javascript",
  ts: "typescript", typescript: "typescript", tsx: "typescript",
  json: "json", html: "html", xml: "xml", css: "css", scss: "scss", less: "less",
  md: "markdown", markdown: "markdown", rs: "rust", rust: "rust",
  py: "python", python: "python", go: "go", java: "java", c: "c", h: "c",
  cpp: "cpp", "c++": "cpp", cc: "cpp", cs: "csharp", csharp: "csharp",
  rb: "ruby", ruby: "ruby", php: "php", swift: "swift", kt: "kotlin", kotlin: "kotlin",
  sh: "shell", bash: "shell", zsh: "shell", shell: "shell", console: "shell",
  yml: "yaml", yaml: "yaml", toml: "ini", ini: "ini", sql: "sql",
  dockerfile: "dockerfile", diff: "plaintext",
};

export function langLabel(lang) {
  const k = (lang || "").toLowerCase();
  return LANG_LABEL[k] || (lang ? lang.toUpperCase() : "Text");
}
export function monacoLang(lang) {
  return LANG_MONACO[(lang || "").toLowerCase()] || "plaintext";
}

// ---- inline parsing ----
const SAFE_SCHEME = /^(https?:|mailto:|tel:)/i;
function safeHref(url) {
  const u = (url || "").trim();
  if (!u) return null;
  if (u.startsWith("#") || u.startsWith("/") || u.startsWith("./") || u.startsWith("../")) return u;
  if (SAFE_SCHEME.test(u)) return u;
  // bare "example.com/..." → treat as https
  if (/^[\w.-]+\.[a-z]{2,}([/?#].*)?$/i.test(u)) return "https://" + u;
  return null;
}

const IMAGE_EXT = /\.(?:avif|bmp|gif|ico|jpe?g|png|webp)(?:[?#].*)?$/i;
const VIDEO_EXT = /\.(?:m4v|mov|mp4|ogv|webm)(?:[?#].*)?$/i;
const SAFE_IMAGE_DATA = /^data:image\/(?:avif|bmp|gif|x-icon|vnd\.microsoft\.icon|jpeg|png|webp);base64,/i;
const SAFE_VIDEO_DATA = /^data:video\/(?:mp4|webm|ogg|quicktime|x-m4v);base64,/i;

function isRestrictedMediaHost(url) {
  try {
    const hostname = new URL(url).hostname.toLowerCase().replace(/^\[|\]$/g, "");
    if (hostname === "localhost"
      || hostname.endsWith(".localhost")
      || hostname.endsWith(".local")
      || hostname.endsWith(".internal")
      || hostname.endsWith(".lan")
      || hostname.endsWith(".home.arpa")) return true;
    // Literal IPv6 hosts are never needed for signed CDN media. Rejecting all of
    // them also covers loopback, ULA, link-local, multicast, and IPv4-mapped forms.
    if (hostname.includes(":")) return true;
    const match = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/.exec(hostname);
    if (!match) return false;
    const octets = match.slice(1).map(Number);
    if (octets.some((value) => value < 0 || value > 255)) return true;
    const [a, b] = octets;
    return a === 0
      || a === 10
      || a === 127
      || (a === 100 && b >= 64 && b <= 127)
      || (a === 169 && b === 254)
      || (a === 172 && b >= 16 && b <= 31)
      || (a === 192 && b === 168)
      || (a === 198 && (b === 18 || b === 19))
      || a >= 224;
  } catch {
    return true;
  }
}

/**
 * Return a media URL only when its source is safe to load inside the chat.
 * Remote media must use HTTPS; HTTP is limited to Tauri's internal asset host.
 * Data URLs are restricted to known raster/video MIME
 * types so HTML, SVG, and arbitrary executable payloads never become media.
 * Explicit Markdown media may use extensionless HTTPS URLs (for signed/CDN
 * endpoints); inferred media still requires a recognized file extension.
 */
export function safeMediaSrc(url, kind = "image", { explicit = false } = {}) {
  const u = String(url || "").trim();
  if (!u || u.startsWith("//")) return null;
  const expectedExtension = kind === "video" ? VIDEO_EXT : IMAGE_EXT;
  if (u.startsWith("/") || u.startsWith("./") || u.startsWith("../")) return expectedExtension.test(u) ? u : null;
  if (/^https:/i.test(u)) {
    if (isRestrictedMediaHost(u)) return null;
    return explicit || expectedExtension.test(u) ? u : null;
  }
  if (/^blob:/i.test(u) || /^asset:/i.test(u)) return u;
  if (/^http:\/\/asset\.localhost(?::\d+)?(?:\/|$)/i.test(u)) return u;
  if (kind === "video" ? SAFE_VIDEO_DATA.test(u) : SAFE_IMAGE_DATA.test(u)) return u;
  if (/^[\w.-]+\.[a-z]{2,}(?:[/?#].*)?$/i.test(u) && expectedExtension.test(u)) return "https://" + u;
  if (!/^[a-z][a-z0-9+.-]*:/i.test(u) && !/[\\\u0000-\u001f\u007f]/.test(u) && expectedExtension.test(u)) return u;
  return null;
}

/** Detect a media link without treating every ordinary URL as an image. */
export function mediaKindForUrl(url, label = "") {
  const u = String(url || "").trim();
  const hint = String(label || "").trim();
  if (/^data:video\//i.test(u) || VIDEO_EXT.test(u) || /^(?:video|视频)(?:\s*[:：-]|$)/i.test(hint)) return "video";
  if (/^data:image\//i.test(u) || IMAGE_EXT.test(u) || /^(?:image|img|图片|图像)(?:\s*[:：-]|$)/i.test(hint)) return "image";
  return null;
}

function hasExplicitMediaLabel(label) {
  return /^(?:video|视频|image|img|图片|图像)(?:\s*[:：-]|$)/i.test(String(label || "").trim());
}

function mediaNode(kind, url, label, { explicit = false } = {}) {
  const src = safeMediaSrc(url, kind, { explicit });
  const fallback = `${kind === "video" ? "Video" : "Image"}: ${label || url || "media"}`;
  if (!src) return txt(fallback);
  if (kind === "video") {
    const video = el("video", "md-media md-media--video");
    video.src = src;
    video.controls = true;
    video.preload = "metadata";
    video.playsInline = true;
    video.setAttribute("playsinline", "");
    video.setAttribute("referrerpolicy", "no-referrer");
    video.setAttribute("aria-label", label || "Video");
    video.style.maxWidth = "100%";
    video.style.display = "block";
    return video;
  }
  const image = el("img", "md-media md-media--image");
  image.src = src;
  image.alt = label || "Image";
  image.loading = "lazy";
  image.decoding = "async";
  image.referrerPolicy = "no-referrer";
  return image;
}

// Inline token matchers, tried by earliest start index (ties: order below).
const INLINE = [
  { // inline code `...`
    re: /(`+)([\s\S]*?[^`]|[^`])\1(?!`)/,
    make: (m) => {
      let code = m[2];
      if (code.length > 1 && code.startsWith(" ") && code.endsWith(" ") && code.trim()) {
        code = code.slice(1, -1);
      }
      const c = el("code");
      c.textContent = code;
      return c;
    },
  },
  { // strikethrough ~~...~~
    re: /~~(?=\S)([\s\S]*?\S)~~/,
    make: (m) => withChildren(el("del"), m[1]),
  },
  { // bold **...** or __...__
    re: /\*\*(?=\S)([\s\S]*?\S)\*\*|(?<![A-Za-z0-9])__(?=\S)([\s\S]*?\S)__(?![A-Za-z0-9])/,
    make: (m) => withChildren(el("strong"), m[1] ?? m[2]),
  },
  { // italic *...* or _..._
    re: /\*(?=\S)([\s\S]*?\S)\*|(?<![A-Za-z0-9])_(?=\S)([\s\S]*?\S)_(?![A-Za-z0-9])/,
    make: (m) => withChildren(el("em"), m[1] ?? m[2]),
  },
  { // image/video embed: ![alt](url); video is detected by extension or a "video:" label
    re: /!\[([^\]]*)\]\(\s*([^)\s]+)(?:\s+"[^"]*")?\s*\)/,
    make: (m) => {
      const kind = mediaKindForUrl(m[2], m[1]) || "image";
      return mediaNode(kind, m[2], m[1], { explicit: true });
    },
  },
  { // link [text](url)
    re: /\[([^\]]+)\]\(\s*([^)\s]+)(?:\s+"[^"]*")?\s*\)/,
    make: (m) => {
      const kind = mediaKindForUrl(m[2], m[1]);
      if (kind) return mediaNode(kind, m[2], m[1], { explicit: hasExplicitMediaLabel(m[1]) });
      const href = safeHref(m[2]);
      if (!href) return withChildren(el("span"), m[1]);
      const a = el("a");
      a.href = href;
      a.target = "_blank";
      a.rel = "noreferrer noopener";
      a.appendChild(frag(parseInline(m[1])));
      return a;
    },
  },
  { // autolink <https://...>
    re: /<((?:https?:\/\/|mailto:)[^>\s]+)>/,
    make: (m) => /^mailto:/i.test(m[1]) ? linkNode(m[1], m[1]) : urlCardNode(m[1]),
  },
  { // bare url
    re: /(?<![\w@/"'(=])((?:https?:\/\/)[^\s<]+[^\s<.,;:!?)\]}'"])/,
    make: (m) => urlCardNode(m[1]),
  },
];

function linkNode(text, url) {
  const href = safeHref(url);
  if (!href) return txt(text);
  const a = el("a");
  a.href = href;
  a.target = "_blank";
  a.rel = "noreferrer noopener";
  a.textContent = text;
  return a;
}

function urlCardNode(url) {
  const kind = mediaKindForUrl(url);
  if (kind) return mediaNode(kind, url, "");
  const href = safeHref(url);
  if (!href) return txt(url);
  let domain;
  try { domain = new URL(href).hostname.replace(/^www\./, ""); }
  catch { domain = href; }

  const card = el("a", "url-card");
  card.href = href;
  card.target = "_blank";
  card.rel = "noreferrer noopener";

  const ico = el("img", "url-card__ico");
  ico.src = `https://www.google.com/s2/favicons?domain=${encodeURIComponent(domain)}&sz=32`;
  ico.width = 16; ico.height = 16; ico.alt = "";
  ico.loading = "lazy";
  ico.onerror = function () { this.style.display = "none"; };
  card.appendChild(ico);

  const body = el("span", "url-card__body");
  const site = el("span", "url-card__site");
  site.textContent = domain;
  body.appendChild(site);
  let display = url;
  if (display.length > 60) display = display.slice(0, 57) + "…";
  const path = el("span", "url-card__path");
  path.textContent = display;
  body.appendChild(path);
  card.appendChild(body);

  const arrow = el("span", "url-card__arrow");
  arrow.textContent = "↗";
  card.appendChild(arrow);

  return card;
}
function withChildren(node, inner) {
  node.appendChild(frag(parseInline(inner)));
  return node;
}
function frag(nodes) {
  const f = document.createDocumentFragment();
  for (const n of nodes) f.appendChild(n);
  return f;
}

/** Parse inline markdown into an array of DOM nodes. Handles \n as <br>. */
function parseInline(text) {
  const out = [];
  let rest = text;
  while (rest.length) {
    let best = null;
    for (let p = 0; p < INLINE.length; p++) {
      const m = INLINE[p].re.exec(rest);
      if (m && (best === null || m.index < best.m.index)) {
        best = { p, m };
        if (m.index === 0) break;
      }
    }
    if (!best) {
      pushText(out, rest);
      break;
    }
    if (best.m.index > 0) pushText(out, rest.slice(0, best.m.index));
    out.push(INLINE[best.p].make(best.m));
    rest = rest.slice(best.m.index + best.m[0].length);
  }
  return out;
}
// Append text, turning escaped chars into literals and \n into <br>.
function pushText(out, s) {
  s = s.replace(/\\([\\`*_{}\[\]()#+\-.!~>|])/g, "$1");
  const parts = s.split("\n");
  parts.forEach((part, i) => {
    if (i > 0) out.push(el("br"));
    if (part) out.push(txt(part));
  });
}

// ---- block parsing ----
const RE_FENCE = /^(\s{0,3})(`{3,}|~{3,})(.*)$/;
const RE_HEADING = /^ {0,3}(#{1,6})(?:[ \t]+(.*?))?[ \t]*#*[ \t]*$/;
const RE_HR = /^ {0,3}([-*_])(?:[ \t]*\1){2,}[ \t]*$/;
const RE_UL = /^(\s*)[-*+][ \t]+(.*)$/;
const RE_OL = /^(\s*)(\d{1,9})[.)][ \t]+(.*)$/;
const RE_QUOTE = /^ {0,3}>[ \t]?(.*)$/;
const RE_TABLE_DELIM = /^ {0,3}\|?[ \t]*:?-{1,}:?[ \t]*(\|[ \t]*:?-{1,}:?[ \t]*)+\|?[ \t]*$/;

function indentOf(line) {
  const m = /^(\s*)/.exec(line);
  return m[1].replace(/\t/g, "    ").length;
}
function isBlank(line) {
  return line.trim() === "";
}
function startsBlock(line) {
  return (
    RE_FENCE.test(line) ||
    RE_HEADING.test(line) ||
    RE_HR.test(line) ||
    RE_UL.test(line) ||
    RE_OL.test(line) ||
    RE_QUOTE.test(line)
  );
}

/** Render an array of source lines into a DocumentFragment. */
function parseBlocks(lines, ctx) {
  const out = document.createDocumentFragment();
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];

    if (isBlank(line)) {
      i++;
      continue;
    }

    // fenced code
    const fence = RE_FENCE.exec(line);
    if (fence) {
      const marker = fence[2];
      const fenceChar = marker[0];
      const closeRe = new RegExp("^\\s{0,3}\\" + fenceChar + "{" + marker.length + ",}\\s*$");
      const info = fence[3].trim();
      const { lang, filename } = parseFenceInfo(info);
      const buf = [];
      i++;
      while (i < lines.length && !closeRe.test(lines[i])) {
        buf.push(lines[i]);
        i++;
      }
      if (i < lines.length) i++; // consume closing fence
      out.appendChild(codeCard(buf.join("\n"), lang, filename, ctx));
      continue;
    }

    // heading
    const h = RE_HEADING.exec(line);
    if (h) {
      const node = el("h" + h[1].length);
      node.appendChild(frag(parseInline((h[2] || "").trim())));
      out.appendChild(node);
      i++;
      continue;
    }

    // horizontal rule
    if (RE_HR.test(line)) {
      out.appendChild(el("hr"));
      i++;
      continue;
    }

    // table
    if (line.includes("|") && i + 1 < lines.length && RE_TABLE_DELIM.test(lines[i + 1])) {
      const res = parseTable(lines, i);
      out.appendChild(res.node);
      i = res.next;
      continue;
    }

    // blockquote
    if (RE_QUOTE.test(line)) {
      const inner = [];
      while (i < lines.length && (RE_QUOTE.test(lines[i]) || (!isBlank(lines[i]) && !startsBlock(lines[i]) && inner.length))) {
        const qm = RE_QUOTE.exec(lines[i]);
        inner.push(qm ? qm[1] : lines[i].trim());
        i++;
      }
      const bq = el("blockquote");
      bq.appendChild(parseBlocks(inner, ctx));
      out.appendChild(bq);
      continue;
    }

    // lists
    if (RE_UL.test(line) || RE_OL.test(line)) {
      const res = parseList(lines, i, ctx);
      out.appendChild(res.node);
      i = res.next;
      continue;
    }

    // paragraph
    const para = [];
    while (i < lines.length && !isBlank(lines[i]) && !startsBlock(lines[i]) && !(lines[i].includes("|") && i + 1 < lines.length && RE_TABLE_DELIM.test(lines[i + 1]))) {
      para.push(lines[i]);
      i++;
    }
    const p = el("p");
    p.appendChild(frag(parseInline(para.join("\n").trim())));
    out.appendChild(p);
  }
  return out;
}

function parseFenceInfo(info) {
  if (!info) return { lang: "", filename: "" };
  const first = info.split(/\s+/)[0];
  if (first.includes(":")) {
    const idx = first.indexOf(":");
    return { lang: first.slice(0, idx), filename: first.slice(idx + 1) };
  }
  const rest = info.slice(first.length).trim();
  const titleMatch = /title=("?)([^"]+)\1/.exec(rest);
  return { lang: first, filename: titleMatch ? titleMatch[2] : (rest && !rest.includes("=") ? rest : "") };
}

function parseTable(lines, start) {
  const aligns = lines[start + 1]
    .replace(/^\s*\|?/, "")
    .replace(/\|?\s*$/, "")
    .split("|")
    .map((c) => {
      const s = c.trim();
      const l = s.startsWith(":");
      const r = s.endsWith(":");
      return r && l ? "center" : r ? "right" : l ? "left" : "";
    });
  const header = splitRow(lines[start]);
  let i = start + 2;
  const rows = [];
  while (i < lines.length && !isBlank(lines[i]) && lines[i].includes("|")) {
    rows.push(splitRow(lines[i]));
    i++;
  }
  const wrap = el("div", "md-table");
  const table = el("table");
  const thead = el("thead");
  const htr = el("tr");
  header.forEach((cell, c) => {
    const th = el("th");
    if (aligns[c]) th.style.textAlign = aligns[c];
    th.appendChild(frag(parseInline(cell)));
    htr.appendChild(th);
  });
  thead.appendChild(htr);
  table.appendChild(thead);
  const tbody = el("tbody");
  for (const row of rows) {
    const tr = el("tr");
    for (let c = 0; c < header.length; c++) {
      const td = el("td");
      if (aligns[c]) td.style.textAlign = aligns[c];
      td.appendChild(frag(parseInline(row[c] || "")));
      tr.appendChild(td);
    }
    tbody.appendChild(tr);
  }
  table.appendChild(tbody);
  wrap.appendChild(table);
  return { node: wrap, next: i };
}
function splitRow(line) {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split(/(?<!\\)\|/)
    .map((c) => c.replace(/\\\|/g, "|").trim());
}

function parseList(lines, start, ctx) {
  const baseIndent = indentOf(lines[start]);
  const ordered = RE_OL.test(lines[start]) && !RE_UL.test(lines[start]);
  const list = el(ordered ? "ol" : "ul");
  if (ordered) {
    const sm = RE_OL.exec(lines[start]);
    if (sm && sm[2] !== "1") list.setAttribute("start", sm[2]);
  }
  let i = start;
  while (i < lines.length) {
    const line = lines[i];
    if (isBlank(line)) break;
    const ind = indentOf(line);
    if (ind < baseIndent) break;
    const m = RE_UL.exec(line) || RE_OL.exec(line);
    if (!m || ind > baseIndent) break;

    const isOl = !RE_UL.test(line);
    const content = isOl ? m[3] : m[2];
    const contentCol = line.length - content.length;

    const li = el("li");
    let firstText = content;
    const task = /^\[([ xX])\]\s+(.*)$/.exec(content);
    if (task) {
      li.classList.add("task");
      const box = el("input");
      box.type = "checkbox";
      box.disabled = true;
      box.checked = task[1].toLowerCase() === "x";
      li.appendChild(box);
      firstText = task[2];
    }

    const childLines = [];
    i++;
    while (i < lines.length) {
      const l2 = lines[i];
      if (isBlank(l2)) {
        let j = i;
        while (j < lines.length && isBlank(lines[j])) j++;
        if (j < lines.length && indentOf(lines[j]) > baseIndent) {
          for (let k = i; k < j; k++) childLines.push("");
          i = j;
          continue;
        }
        break;
      }
      if (indentOf(l2) > baseIndent) {
        childLines.push(dedent(l2, contentCol));
        i++;
      } else {
        break;
      }
    }
    while (childLines.length && childLines[childLines.length - 1] === "") childLines.pop();

    const subLines = [firstText, ...childLines];
    const onlyInline = !childLines.some((l) => startsBlock(l) || isBlank(l));
    if (!childLines.length) {
      li.appendChild(frag(parseInline(firstText)));
    } else if (onlyInline) {
      li.appendChild(frag(parseInline(subLines.join("\n").trim())));
    } else {
      li.appendChild(parseBlocks(subLines, ctx));
    }
    list.appendChild(li);
  }
  return { node: list, next: i };
}
function dedent(line, cols) {
  let n = 0;
  let removed = 0;
  while (n < line.length && removed < cols && (line[n] === " " || line[n] === "\t")) {
    removed += line[n] === "\t" ? 4 : 1;
    n++;
  }
  return line.slice(n);
}

// ---- code card ----
function codeCard(code, lang, filename, ctx) {
  const card = el("div", "code-card");
  card.dataset.lang = monacoLang(lang);

  const head = el("div", "code-card__head");
  const label = el("span", "code-card__lang");
  label.appendChild(icon(filename ? "i-file-code" : "i-code"));
  const labelText = el("span");
  labelText.textContent = filename || langLabel(lang);
  label.appendChild(labelText);
  head.appendChild(label);

  const copy = el("button", "code-card__copy");
  copy.type = "button";
  copy.appendChild(icon("i-copy"));
  const copyText = el("span");
  copyText.textContent = "";
  copy.appendChild(copyText);
  copy.addEventListener("click", () => {
    const write = navigator.clipboard?.writeText
      ? navigator.clipboard.writeText(code)
      : Promise.reject();
    write
      .then(() => flashCopied(copy, copyText))
      .catch(() => {
        try {
          const ta = document.createElement("textarea");
          ta.value = code;
          ta.style.position = "fixed";
          ta.style.opacity = "0";
          document.body.appendChild(ta);
          ta.select();
          document.execCommand("copy");
          ta.remove();
          flashCopied(copy, copyText);
        } catch {
          copyText.textContent = "Failed";
          setTimeout(() => (copyText.textContent = "Copy"), 1400);
        }
      });
  });
  head.appendChild(copy);
  card.appendChild(head);

  const pre = el("pre", "code-card__body");
  const codeEl = el("code");
  codeEl.textContent = code;
  pre.appendChild(codeEl);
  card.appendChild(pre);

  if (ctx && typeof ctx.highlighter === "function" && code.trim()) {
    const mono = monacoLang(lang);
    if (mono !== "plaintext") {
      ctx.highlighter(code, mono)
        .then((html) => {
          if (html) codeEl.innerHTML = html;
        })
        .catch(() => {});
    }
  }
  return card;
}
function flashCopied(btn, label) {
  btn.classList.add("is-copied");
  label.textContent = "Copied";
  clearTimeout(btn._t);
  btn._t = setTimeout(() => {
    btn.classList.remove("is-copied");
    label.textContent = "Copy";
  }, 1400);
}

// ---- public API ----
/**
 * Render markdown `text` into `container` (replacing its contents).
 * @param {HTMLElement} container
 * @param {string} text
 * @param {{ highlighter?: (code:string, lang:string)=>Promise<string>, streaming?: boolean }} [opts]
 */
export function renderMarkdownInto(container, text, opts = {}) {
  container.textContent = "";
  const lines = String(text ?? "").replace(/\r\n?/g, "\n").split("\n");
  container.appendChild(parseBlocks(lines, opts));
  if (opts.streaming) {
    const caret = el("span", "md-caret");
    container.appendChild(caret);
  }
}

// The end-offset of the last "settled" markdown block: the position after the
// last blank line that is NOT inside a code fence. Everything before it is made
// of complete blocks that won't change as more text streams in.
function _settledLen(text) {
  const lines = text.split("\n");
  let offset = 0, boundary = 0, inFence = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (/^\s{0,3}(```|~~~)/.test(line)) inFence = !inFence;
    if (!inFence && line.trim() === "" && i < lines.length - 1) boundary = offset + line.length + 1;
    offset += line.length + 1;
  }
  return boundary;
}

/**
 * Incremental streaming render. Settled blocks (before the last block boundary)
 * are parsed once and left untouched; only the still-growing tail block is
 * re-parsed on each call. This keeps per-frame cost ∝ the current block instead
 * of re-parsing/rebuilding the whole reply every token (which was O(n²) and, with
 * terminal output, froze the UI). Resets to a clean full render if the text
 * diverges from what was committed or the container was modified externally.
 * @param {HTMLElement} container
 * @param {string} text
 * @param {object} [opts]
 */
export function renderMarkdownStream(container, text, opts = {}) {
  text = String(text ?? "").replace(/\r\n?/g, "\n");
  let st = container.__mdStream;
  if (!st || !text.startsWith(st.src) || (st.tail && st.tail.parentNode !== container)) {
    container.textContent = "";
    st = container.__mdStream = { src: "", tail: null };
  }
  const settled = _settledLen(text);
  if (settled > st.src.length) {
    const chunk = text.slice(st.src.length, settled);
    if (chunk.trim()) {
      const node = parseBlocks(chunk.split("\n"), opts);
      if (st.tail) container.insertBefore(node, st.tail);
      else container.appendChild(node);
    }
    st.src = text.slice(0, settled);
  }
  const tailText = text.slice(st.src.length);
  if (!st.tail) { st.tail = el("div", "md-stream-tail"); container.appendChild(st.tail); }
  st.tail.textContent = "";
  if (tailText.trim()) st.tail.appendChild(parseBlocks(tailText.split("\n"), opts));
  if (opts.streaming) st.tail.appendChild(el("span", "md-caret"));
}
