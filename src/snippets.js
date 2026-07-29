import * as monaco from "monaco-editor";

const BUILTIN_SNIPPETS = {
  javascript: [
    { prefix: "log", body: "console.log($1);", description: "Console log" },
    { prefix: "fn", body: "function ${1:name}(${2:params}) {\n\t$0\n}", description: "Function declaration" },
    { prefix: "afn", body: "async function ${1:name}(${2:params}) {\n\t$0\n}", description: "Async function" },
    { prefix: "arrow", body: "const ${1:name} = (${2:params}) => {\n\t$0\n};", description: "Arrow function" },
    { prefix: "iife", body: "(() => {\n\t$0\n})();", description: "IIFE" },
    { prefix: "forof", body: "for (const ${1:item} of ${2:iterable}) {\n\t$0\n}", description: "for...of loop" },
    { prefix: "forin", body: "for (const ${1:key} in ${2:object}) {\n\t$0\n}", description: "for...in loop" },
    { prefix: "trycatch", body: "try {\n\t$1\n} catch (${2:err}) {\n\t$0\n}", description: "Try/catch block" },
    { prefix: "imp", body: "import { $1 } from '${2:module}';", description: "Import statement" },
    { prefix: "impd", body: "import $1 from '${2:module}';", description: "Default import" },
    { prefix: "class", body: "class ${1:Name} {\n\tconstructor(${2:params}) {\n\t\t$0\n\t}\n}", description: "Class" },
    { prefix: "prom", body: "new Promise((resolve, reject) => {\n\t$0\n});", description: "Promise" },
    { prefix: "setTimeout", body: "setTimeout(() => {\n\t$0\n}, ${1:delay});", description: "setTimeout" },
  ],
  typescript: [
    { prefix: "int", body: "interface ${1:Name} {\n\t$0\n}", description: "Interface" },
    { prefix: "type", body: "type ${1:Name} = $0;", description: "Type alias" },
    { prefix: "enum", body: "enum ${1:Name} {\n\t$0\n}", description: "Enum" },
    { prefix: "gen", body: "function ${1:name}<${2:T}>(${3:param}: ${2:T}): ${4:ReturnType} {\n\t$0\n}", description: "Generic function" },
  ],
  html: [
    { prefix: "!", body: '<!DOCTYPE html>\n<html lang="${1:en}">\n<head>\n\t<meta charset="UTF-8">\n\t<meta name="viewport" content="width=device-width, initial-scale=1.0">\n\t<title>${2:Document}</title>\n</head>\n<body>\n\t$0\n</body>\n</html>', description: "HTML5 boilerplate" },
    { prefix: "html5", body: '<!DOCTYPE html>\n<html lang="${1:en}">\n<head>\n\t<meta charset="UTF-8">\n\t<meta name="viewport" content="width=device-width, initial-scale=1.0">\n\t<title>${2:Document}</title>\n</head>\n<body>\n\t$0\n</body>\n</html>', description: "HTML5 boilerplate" },
    { prefix: "html:5", body: '<!DOCTYPE html>\n<html lang="${1:en}">\n<head>\n\t<meta charset="UTF-8">\n\t<meta name="viewport" content="width=device-width, initial-scale=1.0">\n\t<title>${2:Document}</title>\n</head>\n<body>\n\t$0\n</body>\n</html>', description: "HTML5 boilerplate" },
    { prefix: "div", body: "<div${1: class=\"$2\"}>\n\t$0\n</div>", description: "Div element" },
    { prefix: "div.class", body: '<div class="${1:name}">\n\t$0\n</div>', description: "Div with class" },
    { prefix: "div#id", body: '<div id="${1:name}">\n\t$0\n</div>', description: "Div with id" },
    { prefix: "a", body: '<a href="${1:#}">${2:link}</a>', description: "Anchor tag" },
    { prefix: "img", body: '<img src="${1:src}" alt="${2:alt}">', description: "Image tag" },
    { prefix: "input", body: '<input type="${1:text}" name="${2:name}" placeholder="${3:placeholder}">', description: "Input element" },
    { prefix: "button", body: '<button type="${1:button}">${2:Click}</button>', description: "Button element" },
    { prefix: "form", body: '<form action="${1:#}" method="${2:post}">\n\t$0\n</form>', description: "Form element" },
    { prefix: "ul>li", body: '<ul>\n\t<li>$1</li>\n\t<li>$2</li>\n\t<li>$0</li>\n</ul>', description: "Unordered list" },
    { prefix: "ol>li", body: '<ol>\n\t<li>$1</li>\n\t<li>$2</li>\n\t<li>$0</li>\n</ol>', description: "Ordered list" },
    { prefix: "table", body: '<table>\n\t<thead>\n\t\t<tr>\n\t\t\t<th>${1:Header}</th>\n\t\t</tr>\n\t</thead>\n\t<tbody>\n\t\t<tr>\n\t\t\t<td>$0</td>\n\t\t</tr>\n\t</tbody>\n</table>', description: "Table structure" },
    { prefix: "link", body: '<link rel="stylesheet" href="${1:style.css}">', description: "CSS link" },
    { prefix: "link:css", body: '<link rel="stylesheet" href="${1:style.css}">', description: "CSS link" },
    { prefix: "script", body: '<script src="${1:script.js}"></script>', description: "Script tag" },
    { prefix: "script:src", body: '<script src="${1:script.js}"></script>', description: "Script with src" },
    { prefix: "style", body: '<style>\n\t$0\n</style>', description: "Style tag" },
    { prefix: "meta:vp", body: '<meta name="viewport" content="width=device-width, initial-scale=1.0">', description: "Viewport meta" },
    { prefix: "section", body: '<section${1: class="$2"}>\n\t$0\n</section>', description: "Section element" },
    { prefix: "header", body: '<header${1: class="$2"}>\n\t$0\n</header>', description: "Header element" },
    { prefix: "footer", body: '<footer${1: class="$2"}>\n\t$0\n</footer>', description: "Footer element" },
    { prefix: "nav", body: '<nav${1: class="$2"}>\n\t$0\n</nav>', description: "Nav element" },
    { prefix: "main", body: '<main${1: class="$2"}>\n\t$0\n</main>', description: "Main element" },
    { prefix: "p", body: '<p>${1:text}</p>', description: "Paragraph" },
    { prefix: "span", body: '<span>${1:text}</span>', description: "Span" },
    { prefix: "h1", body: '<h1>${1:Heading}</h1>', description: "H1" },
    { prefix: "h2", body: '<h2>${1:Heading}</h2>', description: "H2" },
    { prefix: "h3", body: '<h3>${1:Heading}</h3>', description: "H3" },
    { prefix: "h4", body: '<h4>${1:Heading}</h4>', description: "H4" },
    { prefix: "h5", body: '<h5>${1:Heading}</h5>', description: "H5" },
    { prefix: "h6", body: '<h6>${1:Heading}</h6>', description: "H6" },
    { prefix: "article", body: '<article${1: class="$2"}>\n\t$0\n</article>', description: "Article" },
    { prefix: "aside", body: '<aside${1: class="$2"}>\n\t$0\n</aside>', description: "Aside" },
    { prefix: "figure", body: '<figure>\n\t<img src="${1:src}" alt="${2:alt}">\n\t<figcaption>${3:caption}</figcaption>\n</figure>', description: "Figure with caption" },
    { prefix: "video", body: '<video src="${1:video.mp4}" controls${2: autoplay}${3: muted}>\n\tYour browser does not support video.\n</video>', description: "Video element" },
    { prefix: "audio", body: '<audio src="${1:audio.mp3}" controls>\n\tYour browser does not support audio.\n</audio>', description: "Audio element" },
    { prefix: "iframe", body: '<iframe src="${1:url}" width="${2:600}" height="${3:400}" frameborder="0" allowfullscreen></iframe>', description: "Iframe" },
    { prefix: "select", body: '<select name="${1:name}">\n\t<option value="${2:value1}">${3:Option 1}</option>\n\t<option value="${4:value2}">${5:Option 2}</option>\n</select>', description: "Select dropdown" },
    { prefix: "textarea", body: '<textarea name="${1:name}" rows="${2:4}" cols="${3:50}" placeholder="${4:Enter text...}"></textarea>', description: "Textarea" },
    { prefix: "label", body: '<label for="${1:id}">${2:Label}</label>', description: "Label" },
    { prefix: "fieldset", body: '<fieldset>\n\t<legend>${1:Legend}</legend>\n\t$0\n</fieldset>', description: "Fieldset" },
    { prefix: "details", body: '<details>\n\t<summary>${1:Summary}</summary>\n\t$0\n</details>', description: "Details/Summary" },
    { prefix: "dialog", body: '<dialog id="${1:modal}">\n\t$0\n\t<button onclick="this.closest(\'dialog\').close()">Close</button>\n</dialog>', description: "Dialog" },
    { prefix: "template", body: '<template id="${1:tmpl}">\n\t$0\n</template>', description: "Template" },
    { prefix: "picture", body: '<picture>\n\t<source srcset="${1:image.webp}" type="image/webp">\n\t<img src="${2:image.jpg}" alt="${3:alt}">\n</picture>', description: "Picture element" },
    { prefix: "meta:og", body: '<meta property="og:title" content="${1:Title}">\n<meta property="og:description" content="${2:Description}">\n<meta property="og:image" content="${3:image.jpg}">\n<meta property="og:url" content="${4:url}">', description: "Open Graph meta" },
    { prefix: "link:icon", body: '<link rel="icon" type="image/${1:png}" href="${2:favicon.png}">', description: "Favicon" },
    { prefix: "link:font", body: '<link rel="preconnect" href="https://fonts.googleapis.com">\n<link href="https://fonts.googleapis.com/css2?family=${1:Inter}:wght@400;500;600;700&display=swap" rel="stylesheet">', description: "Google Fonts" },
    { prefix: "cdn:tailwind", body: '<script src="https://cdn.tailwindcss.com"></script>', description: "Tailwind CDN" },
    { prefix: "cdn:alpine", body: '<script defer src="https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/cdn.min.js"></script>', description: "Alpine.js CDN" },
    { prefix: "cdn:vue", body: '<script src="https://unpkg.com/vue@3/dist/vue.global.js"></script>', description: "Vue 3 CDN" },
    { prefix: "cdn:react", body: '<script crossorigin src="https://unpkg.com/react@18/umd/react.production.min.js"></script>\n<script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.production.min.js"></script>', description: "React CDN" },
    { prefix: "comment", body: '<!-- ${1:comment} -->', description: "HTML comment" },
    { prefix: "lorem", body: 'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.', description: "Lorem ipsum" },
  ],
  css: [
    { prefix: "flex", body: "display: flex;\nalign-items: ${1:center};\njustify-content: ${2:center};", description: "Flexbox center" },
    { prefix: "flex-col", body: "display: flex;\nflex-direction: column;\nalign-items: ${1:center};\ngap: ${2:16px};", description: "Flexbox column" },
    { prefix: "flex-between", body: "display: flex;\nalign-items: center;\njustify-content: space-between;", description: "Flexbox space-between" },
    { prefix: "grid", body: "display: grid;\ngrid-template-columns: ${1:repeat(3, 1fr)};\ngap: ${2:16px};", description: "CSS Grid" },
    { prefix: "grid-center", body: "display: grid;\nplace-items: center;\nmin-height: ${1:100vh};", description: "Grid center" },
    { prefix: "grid-auto", body: "display: grid;\ngrid-template-columns: repeat(auto-fill, minmax(${1:250px}, 1fr));\ngap: ${2:16px};", description: "Responsive grid" },
    { prefix: "media", body: "@media (max-width: ${1:768px}) {\n\t$0\n}", description: "Media query" },
    { prefix: "media:mobile", body: "@media (max-width: 480px) {\n\t$0\n}", description: "Mobile media query" },
    { prefix: "media:tablet", body: "@media (max-width: 768px) {\n\t$0\n}", description: "Tablet media query" },
    { prefix: "media:desktop", body: "@media (min-width: 1024px) {\n\t$0\n}", description: "Desktop media query" },
    { prefix: "media:dark", body: "@media (prefers-color-scheme: dark) {\n\t$0\n}", description: "Dark mode media query" },
    { prefix: "var", body: "var(--${1:name})", description: "CSS variable" },
    { prefix: "root", body: ":root {\n\t--${1:primary}: ${2:#3b82f6};\n\t--${3:bg}: ${4:#ffffff};\n\t--${5:text}: ${6:#1a1a1a};\n\t$0\n}", description: "CSS variables root" },
    { prefix: "reset", body: "*, *::before, *::after {\n\tbox-sizing: border-box;\n\tmargin: 0;\n\tpadding: 0;\n}\nbody {\n\tmin-height: 100vh;\n\tline-height: 1.5;\n\t-webkit-font-smoothing: antialiased;\n}", description: "CSS reset" },
    { prefix: "abs", body: "position: absolute;\ntop: ${1:0};\nleft: ${2:0};", description: "Position absolute" },
    { prefix: "abs-center", body: "position: absolute;\ntop: 50%;\nleft: 50%;\ntransform: translate(-50%, -50%);", description: "Absolute center" },
    { prefix: "fixed", body: "position: fixed;\ntop: ${1:0};\nleft: ${2:0};\nwidth: 100%;\nz-index: ${3:100};", description: "Position fixed" },
    { prefix: "sticky", body: "position: sticky;\ntop: ${1:0};\nz-index: ${2:10};", description: "Position sticky" },
    { prefix: "transition", body: "transition: ${1:all} ${2:0.3s} ${3:ease};", description: "Transition" },
    { prefix: "anim", body: "@keyframes ${1:name} {\n\t0% { $2 }\n\t100% { $0 }\n}", description: "Keyframe animation" },
    { prefix: "animation", body: "animation: ${1:name} ${2:0.3s} ${3:ease} ${4:forwards};", description: "Animation shorthand" },
    { prefix: "shadow", body: "box-shadow: ${1:0 4px 6px -1px} rgba(0, 0, 0, ${2:0.1});", description: "Box shadow" },
    { prefix: "shadow-lg", body: "box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -4px rgba(0, 0, 0, 0.1);", description: "Large shadow" },
    { prefix: "truncate", body: "overflow: hidden;\ntext-overflow: ellipsis;\nwhite-space: nowrap;", description: "Text truncate" },
    { prefix: "truncate-multi", body: "display: -webkit-box;\n-webkit-line-clamp: ${1:3};\n-webkit-box-orient: vertical;\noverflow: hidden;", description: "Multi-line truncate" },
    { prefix: "gradient", body: "background: linear-gradient(${1:135deg}, ${2:#667eea} 0%, ${3:#764ba2} 100%);", description: "Linear gradient" },
    { prefix: "bg-img", body: "background-image: url('${1:image.jpg}');\nbackground-size: cover;\nbackground-position: center;\nbackground-repeat: no-repeat;", description: "Background image" },
    { prefix: "scrollbar", body: "::-webkit-scrollbar {\n\twidth: ${1:8px};\n}\n::-webkit-scrollbar-track {\n\tbackground: ${2:#f1f1f1};\n}\n::-webkit-scrollbar-thumb {\n\tbackground: ${3:#888};\n\tborder-radius: 4px;\n}", description: "Custom scrollbar" },
    { prefix: "border", body: "border: ${1:1px} ${2:solid} ${3:#e5e7eb};", description: "Border" },
    { prefix: "radius", body: "border-radius: ${1:8px};", description: "Border radius" },
    { prefix: "container", body: "max-width: ${1:1200px};\nmargin: 0 auto;\npadding: 0 ${2:20px};", description: "Container" },
    { prefix: "aspect", body: "aspect-ratio: ${1:16} / ${2:9};", description: "Aspect ratio" },
    { prefix: "clamp", body: "font-size: clamp(${1:1rem}, ${2:2.5vw}, ${3:2rem});", description: "Clamp font size" },
    { prefix: "glass", body: "background: rgba(255, 255, 255, ${1:0.15});\nbackdrop-filter: blur(${2:10px});\n-webkit-backdrop-filter: blur(${2:10px});\nborder: 1px solid rgba(255, 255, 255, 0.2);", description: "Glassmorphism" },
    { prefix: "sr-only", body: "position: absolute;\nwidth: 1px;\nheight: 1px;\npadding: 0;\nmargin: -1px;\noverflow: hidden;\nclip: rect(0, 0, 0, 0);\nwhite-space: nowrap;\nborder-width: 0;", description: "Screen reader only" },
    { prefix: "hover", body: "&:hover {\n\t$0\n}", description: "Hover state" },
    { prefix: "focus", body: "&:focus {\n\toutline: 2px solid ${1:#3b82f6};\n\toutline-offset: 2px;\n}", description: "Focus state" },
    { prefix: "dark", body: ".dark & {\n\t$0\n}", description: "Dark mode" },
    { prefix: "btn", body: "display: inline-flex;\nalign-items: center;\njustify-content: center;\npadding: ${1:8px 16px};\nfont-size: ${2:14px};\nfont-weight: 500;\nborder-radius: ${3:6px};\nborder: none;\ncursor: pointer;\ntransition: all 0.2s;", description: "Button base" },
    { prefix: "card", body: "background: ${1:#fff};\nborder-radius: ${2:12px};\nbox-shadow: 0 1px 3px rgba(0,0,0,0.1);\npadding: ${3:24px};\ntransition: box-shadow 0.2s;", description: "Card base" },
    { prefix: "input-base", body: "width: 100%;\npadding: ${1:8px 12px};\nfont-size: ${2:14px};\nborder: 1px solid ${3:#d1d5db};\nborder-radius: ${4:6px};\noutline: none;\ntransition: border-color 0.2s;\n&:focus {\n\tborder-color: ${5:#3b82f6};\n}", description: "Input base" },
  ],
  rust: [
    { prefix: "fn", body: "fn ${1:name}(${2:params}) -> ${3:ReturnType} {\n\t$0\n}", description: "Function" },
    { prefix: "impl", body: "impl ${1:Type} {\n\t$0\n}", description: "Impl block" },
    { prefix: "struct", body: "#[derive(Debug)]\nstruct ${1:Name} {\n\t$0\n}", description: "Struct" },
    { prefix: "enum", body: "enum ${1:Name} {\n\t$0\n}", description: "Enum" },
    { prefix: "match", body: "match ${1:value} {\n\t${2:pattern} => $0,\n}", description: "Match expression" },
    { prefix: "test", body: "#[test]\nfn ${1:test_name}() {\n\t$0\n}", description: "Test function" },
  ],
  python: [
    { prefix: "def", body: "def ${1:name}(${2:params}):\n\t$0", description: "Function" },
    { prefix: "class", body: "class ${1:Name}:\n\tdef __init__(self${2:, params}):\n\t\t$0", description: "Class" },
    { prefix: "ifmain", body: 'if __name__ == "__main__":\n\t$0', description: "if __name__ == __main__" },
    { prefix: "with", body: "with ${1:expression} as ${2:var}:\n\t$0", description: "With statement" },
    { prefix: "try", body: "try:\n\t$1\nexcept ${2:Exception} as ${3:e}:\n\t$0", description: "Try/except" },
  ],
};

const LANG_ALIAS = {
  javascriptreact: "javascript",
  typescriptreact: "typescript",
  scss: "css",
  sass: "css",
  less: "css",
};

// User-defined snippets (from the Snippet editor). Grouped by normalized langId.
// Previously these were written to the "custom-snippets" store but NEVER read back,
// so a saved snippet never showed up in completion — setCustomSnippets fixes that.
let CUSTOM_SNIPPETS = {};
export function setCustomSnippets(list) {
  CUSTOM_SNIPPETS = {};
  for (const s of (Array.isArray(list) ? list : [])) {
    if (!s || !s.prefix || !s.body) continue;
    const l = LANG_ALIAS[s.lang] || s.lang || "javascript";
    (CUSTOM_SNIPPETS[l] ||= []).push({ prefix: s.prefix, body: s.body, description: s.description || "自定义片段" });
  }
}

function snippetsForLang(langId) {
  const normalized = LANG_ALIAS[langId] || langId;
  const results = [...(BUILTIN_SNIPPETS[normalized] || []), ...(CUSTOM_SNIPPETS[normalized] || [])];
  if (normalized === "typescript") {
    results.push(...(BUILTIN_SNIPPETS.javascript || []));
  }
  return results;
}

export function registerSnippetProviders() {
  const languages = new Set([
    ...Object.keys(BUILTIN_SNIPPETS),
    ...Object.keys(LANG_ALIAS),
  ]);

  for (const lang of languages) {
    monaco.languages.registerCompletionItemProvider(lang, {
      triggerCharacters: ["!"],
      provideCompletionItems(model, position) {
        const word = model.getWordUntilPosition(position);
        const lineContent = model.getLineContent(position.lineNumber);
        const textBefore = lineContent.slice(0, position.column - 1);

        let rangeStart = word.startColumn;
        const trimmed = textBefore.trimStart();
        if (trimmed === "!" || trimmed.endsWith("!")) {
          const excIdx = textBefore.lastIndexOf("!");
          rangeStart = excIdx + 1;
        }

        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: rangeStart,
          endColumn: position.column,
        };
        const snippets = snippetsForLang(lang);
        return {
          suggestions: snippets.map((s) => ({
            label: s.prefix,
            kind: monaco.languages.CompletionItemKind.Snippet,
            insertText: s.body,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            documentation: s.description,
            detail: "Snippet",
            range,
            sortText: s.prefix === "!" ? "0" : "1" + s.prefix,
          })),
        };
      },
    });
  }
}
