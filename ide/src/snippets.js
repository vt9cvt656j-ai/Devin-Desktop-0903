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
    { prefix: "html5", body: '<!DOCTYPE html>\n<html lang="${1:en}">\n<head>\n\t<meta charset="UTF-8">\n\t<meta name="viewport" content="width=device-width, initial-scale=1.0">\n\t<title>${2:Document}</title>\n</head>\n<body>\n\t$0\n</body>\n</html>', description: "HTML5 boilerplate" },
    { prefix: "div", body: "<div${1: class=\"$2\"}>\n\t$0\n</div>", description: "Div element" },
    { prefix: "a", body: '<a href="${1:#}">${2:link}</a>', description: "Anchor tag" },
    { prefix: "link", body: '<link rel="stylesheet" href="${1:style.css}">', description: "CSS link" },
    { prefix: "script", body: '<script src="${1:script.js}"></script>', description: "Script tag" },
  ],
  css: [
    { prefix: "flex", body: "display: flex;\nalign-items: ${1:center};\njustify-content: ${2:center};", description: "Flexbox center" },
    { prefix: "grid", body: "display: grid;\ngrid-template-columns: ${1:repeat(3, 1fr)};\ngap: ${2:16px};", description: "CSS Grid" },
    { prefix: "media", body: "@media (max-width: ${1:768px}) {\n\t$0\n}", description: "Media query" },
    { prefix: "var", body: "var(--${1:name})", description: "CSS variable" },
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

function snippetsForLang(langId) {
  const normalized = LANG_ALIAS[langId] || langId;
  const results = [...(BUILTIN_SNIPPETS[normalized] || [])];
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
      provideCompletionItems(model, position) {
        const word = model.getWordUntilPosition(position);
        const range = {
          startLineNumber: position.lineNumber,
          endLineNumber: position.lineNumber,
          startColumn: word.startColumn,
          endColumn: word.endColumn,
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
          })),
        };
      },
    });
  }
}
