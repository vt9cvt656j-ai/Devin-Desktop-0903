// Bracket Colorizer — highlights matching brackets with nesting-level colors.

const BRACKET_COLORS = [
  "bracket-color-1",
  "bracket-color-2",
  "bracket-color-3",
  "bracket-color-4",
  "bracket-color-5",
];

const OPEN = new Set(["(", "[", "{"]);
const CLOSE = new Set([")", "]", "}"]);
const MATCH = { ")": "(", "]": "[", "}": "{" };

export function activate(ide) {
  async function colorize() {
    const text = await ide.editor.getText();
    if (!text) return;

    const decorations = [];
    const stack = [];
    const lines = text.split("\n");

    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
      const line = lines[lineIdx];
      for (let col = 0; col < line.length; col++) {
        const ch = line[col];
        if (OPEN.has(ch)) {
          const level = stack.length;
          stack.push({ ch, line: lineIdx, col });
          decorations.push({
            range: {
              startLineNumber: lineIdx + 1,
              startColumn: col + 1,
              endLineNumber: lineIdx + 1,
              endColumn: col + 2,
            },
            inlineClassName: BRACKET_COLORS[level % BRACKET_COLORS.length],
          });
        } else if (CLOSE.has(ch)) {
          const expected = MATCH[ch];
          if (stack.length > 0 && stack[stack.length - 1].ch === expected) {
            stack.pop();
          }
          const level = stack.length;
          decorations.push({
            range: {
              startLineNumber: lineIdx + 1,
              startColumn: col + 1,
              endLineNumber: lineIdx + 1,
              endColumn: col + 2,
            },
            inlineClassName: BRACKET_COLORS[level % BRACKET_COLORS.length],
          });
        }
      }
    }

    await ide.editor.setDecorations(decorations);
    ide.window.showInformationMessage(`Colorized ${decorations.length} brackets`);
  }

  ide.commands.register("brackets.colorize", colorize);

  ide.commands.register("brackets.clear", async () => {
    await ide.editor.clearDecorations();
    ide.window.showInformationMessage("Bracket colors cleared");
  });

  ide.window.setStatusBarItem("brackets", {
    text: "Brackets",
    tooltip: "Bracket Colorizer — click to colorize",
    command: "brackets.colorize",
  });
}
