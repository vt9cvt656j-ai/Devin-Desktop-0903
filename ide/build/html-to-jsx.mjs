/**
 * index.html 的 UI markup → JSX，机械转换。
 *
 * 为什么用生成而不是手写：这段 markup 有 159 个 ID 是 main.js 用 `$(id)` 直接抓的
 * （`const $ = (id) => document.getElementById(id)`）。手抄 855 行、漏掉一个 ID，
 * 表现是某个按钮**静默失灵** —— 不报错、不红、测试也未必抓得到。机械转换保证
 * 结构、ID、class、属性逐字保真，shadcn 的升级在这之后单独做，改哪一处清清楚楚。
 *
 * 图标 sprite（77 个 <symbol>）不转：它是静态 defs，留在 index.html 里由
 * `<use href="#i-x"/>` 引用，搬进 React 只会让首屏多解析一遍。
 */
import { readFileSync } from "node:fs";

const VOID = new Set([
  "area", "base", "br", "col", "embed", "hr", "img", "input",
  "link", "meta", "param", "source", "track", "wbr",
]);

/** HTML 属性名 → React 属性名。只列实际出现的，不做全量映射表。 */
const ATTR = {
  class: "className",
  for: "htmlFor",
  tabindex: "tabIndex",
  colspan: "colSpan",
  rowspan: "rowSpan",
  maxlength: "maxLength",
  minlength: "minLength",
  autocomplete: "autoComplete",
  autofocus: "autoFocus",
  readonly: "readOnly",
  spellcheck: "spellCheck",
  contenteditable: "contentEditable",
  crossorigin: "crossOrigin",
  datetime: "dateTime",
  enterkeyhint: "enterKeyHint",
  inputmode: "inputMode",
  novalidate: "noValidate",
  formnovalidate: "formNoValidate",
  accesskey: "accessKey",
  srcset: "srcSet",
  usemap: "useMap",
  autocapitalize: "autoCapitalize",

  // SVG 的连字符属性。React 只认驼峰，写成 stroke-width 会被当成未知 prop **丢掉**
  // —— 控制台报 "Invalid DOM property"，而图标画出来是错的（线宽/端点全没了）。
  // 这个 sprite 里 77 个 symbol 全靠这些属性。
  "stroke-width": "strokeWidth",
  "stroke-linecap": "strokeLinecap",
  "stroke-linejoin": "strokeLinejoin",
  "stroke-dasharray": "strokeDasharray",
  "stroke-dashoffset": "strokeDashoffset",
  "stroke-miterlimit": "strokeMiterlimit",
  "stroke-opacity": "strokeOpacity",
  "fill-rule": "fillRule",
  "fill-opacity": "fillOpacity",
  "clip-rule": "clipRule",
  "clip-path": "clipPath",
  "stop-color": "stopColor",
  "stop-opacity": "stopOpacity",
  "text-anchor": "textAnchor",
  "font-family": "fontFamily",
  "font-size": "fontSize",
  "font-weight": "fontWeight",
  "letter-spacing": "letterSpacing",
  "dominant-baseline": "dominantBaseline",
  "paint-order": "paintOrder",
  "vector-effect": "vectorEffect",
  "shape-rendering": "shapeRendering",
  "marker-end": "markerEnd",
  "marker-start": "markerStart",
  "marker-mid": "markerMid",
  "flood-color": "floodColor",
  "flood-opacity": "floodOpacity",
  "gradientunits": "gradientUnits",
  "gradienttransform": "gradientTransform",
  "patternunits": "patternUnits",
  "preserveaspectratio": "preserveAspectRatio",
  "viewbox": "viewBox",
};

/**
 * `checked` / `value` 在 React 里是**受控**属性：只给 checked 不给 onChange，React 会
 * 把字段变成只读并报警。这些控件是 main.js 用命令式代码驱动的（addEventListener +
 * 直接改 .checked），不是 React 状态，所以要的是非受控的初始值。
 */
const UNCONTROLLED = { checked: "defaultChecked", value: "defaultValue" };

/**
 * 只有 <input> / <textarea> 的 value 才是"受控属性"。
 *
 * <option value="lsp"> 的 value 是**选项的值**，不是受控输入 —— 把它换成 defaultValue，
 * 那个 option 就彻底没有 value 属性了，`select.value` 读出来是选项的文本而不是 "lsp"，
 * main.js 那边按 value 分支的逻辑会全部走错。<button value> 同理。
 * 所以受控转换必须看标签名，不能只看属性名。
 */
const CONTROLLED_TAGS = new Set(["input", "textarea", "select"]);

/** 布尔属性：JSX 里写成 `attr` 或 `attr={true}`，不能写成 attr="" */
const BOOL = new Set([
  "hidden", "disabled", "checked", "readonly", "required", "multiple",
  "selected", "autofocus", "novalidate", "formnovalidate", "open", "default",
  "controls", "loop", "muted", "playsinline", "autoplay", "reversed", "async", "defer",
]);

function styleToObject(css) {
  const out = [];
  for (const decl of css.split(";")) {
    const i = decl.indexOf(":");
    if (i < 0) continue;
    const prop = decl.slice(0, i).trim();
    const value = decl.slice(i + 1).trim();
    if (!prop || !value) continue;
    // --custom-prop 必须原样保留字符串键；其余转驼峰。
    const key = prop.startsWith("--")
      ? JSON.stringify(prop)
      : prop.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
    out.push(`${key}: ${JSON.stringify(value)}`);
  }
  return `{{ ${out.join(", ")} }}`;
}

function convertAttrs(raw, tagName) {
  const parts = [];
  const re = /([:@a-zA-Z_][-:a-zA-Z0-9_]*)(?:\s*=\s*("([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;
  let m;
  while ((m = re.exec(raw))) {
    const name = m[1];
    const hasValue = m[2] !== undefined;
    const value = m[3] ?? m[4] ?? m[5] ?? "";
    const lower = name.toLowerCase();

    if (lower === "style" && hasValue) { parts.push(`style=${styleToObject(value)}`); continue; }
    // 受控属性先转成非受控形式，再走布尔/普通分支。
    if (UNCONTROLLED[lower] && CONTROLLED_TAGS.has(tagName)) {
      const name2 = UNCONTROLLED[lower];
      parts.push(hasValue && value !== "" && value !== lower ? `${name2}=${JSON.stringify(value)}` : name2);
      continue;
    }
    if (BOOL.has(lower) && (!hasValue || value === "" || value === lower)) {
      parts.push(ATTR[lower] || lower);
      continue;
    }
    // data-* / aria-* 原样保留，React 直接支持
    const jsxName = lower.startsWith("data-") || lower.startsWith("aria-")
      ? lower
      : (ATTR[lower] || name);
    parts.push(hasValue ? `${jsxName}=${JSON.stringify(value)}` : jsxName);
  }
  return parts.length ? " " + parts.join(" ") : "";
}

/** JSX 文本节点里 { } 会被当表达式，必须转义；< > & 用实体已足够。 */
function escapeText(t) {
  if (!t.trim()) return t;
  return t.replace(/[{}]/g, (c) => `{"${c}"}`);
}

export function htmlToJsx(html) {
  let out = "";
  let i = 0;
  while (i < html.length) {
    if (html.startsWith("<!--", i)) {
      const end = html.indexOf("-->", i);
      const body = html.slice(i + 4, end < 0 ? html.length : end).trim();
      out += `{/* ${body.replace(/\*\//g, "*\\/")} */}`;
      i = end < 0 ? html.length : end + 3;
      continue;
    }
    if (html[i] === "<") {
      const close = html.indexOf(">", i);
      if (close < 0) { out += escapeText(html.slice(i)); break; }
      let tag = html.slice(i + 1, close);
      const selfClosed = tag.endsWith("/");
      if (selfClosed) tag = tag.slice(0, -1);
      const isEnd = tag.startsWith("/");
      const name = (isEnd ? tag.slice(1) : tag).split(/[\s/>]/)[0];

      if (isEnd) {
        out += `</${name}>`;
      } else {
        const attrs = convertAttrs(tag.slice(name.length), name.toLowerCase());
        out += VOID.has(name.toLowerCase()) || selfClosed
          ? `<${name}${attrs} />`
          : `<${name}${attrs}>`;
      }
      i = close + 1;
      continue;
    }
    const next = html.indexOf("<", i);
    const text = html.slice(i, next < 0 ? html.length : next);
    out += escapeText(text);
    i = next < 0 ? html.length : next;
  }
  return out;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [, , file] = process.argv;
  process.stdout.write(htmlToJsx(readFileSync(file, "utf8")));
}
