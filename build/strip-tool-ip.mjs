// Build-time IP strip: remove tool DESCRIPTION text from the shipped bundle.
//
// Why this is safe (zero runtime behavior change): with L0 forced on, every agent
// request already sends only tool NAMES (x-ide-tools header) and the gateway injects
// the full schema (descriptions + parameters) from its own tools.json. The client
// never transmits these description strings. They only sat in the bundle as static
// data an attacker could base64-decode out of the obfuscated file. Blanking them at
// build time closes that leak while the running app is unaffected.
//
// Scope: ONLY the `_buildAgentToolSchemas` function body, and within it only the
// `description:` string/template values on tool-schema and parameter lines. Tool
// NAMES, parameter NAMES, enums, types, and required arrays are preserved (the model
// receives full text from the gateway; the client keeps just enough structure to
// name-route and locally validate).

const FN_MARKER = "function _buildAgentToolSchemas";

// Replace the VALUE of every `description:` key on a single line with "". Handles
// double/single-quoted strings and template literals, respecting backslash escapes.
// Operates char-by-char so nested quotes/braces inside a description can't fool it.
function blankDescriptionsInLine(line) {
  const KEY = "description:";
  let out = "";
  let i = 0;
  while (i < line.length) {
    const at = line.indexOf(KEY, i);
    if (at === -1) {
      out += line.slice(i);
      break;
    }
    // Copy up to and including the key.
    out += line.slice(i, at + KEY.length);
    let j = at + KEY.length;
    // Skip whitespace between the key and its value.
    while (j < line.length && (line[j] === " " || line[j] === "\t")) {
      out += line[j];
      j++;
    }
    const q = line[j];
    if (q === '"' || q === "'" || q === "`") {
      // Consume the string literal, honoring escapes, and drop its contents.
      let k = j + 1;
      while (k < line.length) {
        if (line[k] === "\\") {
          k += 2;
          continue;
        }
        if (line[k] === q) break;
        k++;
      }
      out += q + q; // empty string of the same quote kind
      i = k + 1; // resume after the closing quote
    } else {
      // `description:` not followed by a string literal (unexpected) — leave as-is.
      i = j;
    }
  }
  return out;
}

export function stripToolIp(source) {
  const newline = source.includes("\r\n") ? "\r\n" : "\n";
  const lines = source.split(/\r?\n/);
  const start = lines.findIndex((l) => l.startsWith(FN_MARKER));
  if (start === -1) return { code: source, changed: 0, found: false };
  let end = -1;
  for (let i = start + 1; i < lines.length; i++) {
    if (lines[i] === "}") {
      end = i;
      break;
    }
  }
  if (end === -1) return { code: source, changed: 0, found: false };
  let changed = 0;
  for (let i = start; i <= end; i++) {
    if (!lines[i].includes("description:")) continue;
    const next = blankDescriptionsInLine(lines[i]);
    if (next !== lines[i]) {
      changed++;
      lines[i] = next;
    }
  }
  return { code: lines.join(newline), changed, found: true };
}
