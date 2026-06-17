// Color Preview — detects CSS colors and shows inline swatches via decorations.

const COLOR_RE = /#(?:[0-9a-fA-F]{3,4}){1,2}\b|rgb\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*\)|rgba\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*,\s*[\d.]+\s*\)|hsl\(\s*\d+\s*,\s*\d+%?\s*,\s*\d+%?\s*\)/g;

export function activate(ide) {
  async function preview() {
    const text = await ide.editor.getText();
    if (!text) return;

    const decorations = [];
    const lines = text.split("\n");
    let count = 0;

    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
      const line = lines[lineIdx];
      COLOR_RE.lastIndex = 0;
      let match;
      while ((match = COLOR_RE.exec(line)) !== null) {
        decorations.push({
          range: {
            startLineNumber: lineIdx + 1,
            startColumn: match.index + match[0].length + 1,
            endLineNumber: lineIdx + 1,
            endColumn: match.index + match[0].length + 1,
          },
          after: {
            content: " ■ ",
            className: "color-swatch",
          },
          hoverMessage: `Color: ${match[0]}`,
        });
        count++;
      }
    }

    await ide.editor.setDecorations(decorations);
    ide.window.showInformationMessage(`Found ${count} color${count === 1 ? "" : "s"}`);
  }

  ide.commands.register("color.preview", preview);

  ide.commands.register("color.clear", async () => {
    await ide.editor.clearDecorations();
    ide.window.showInformationMessage("Color previews cleared");
  });

  ide.window.setStatusBarItem("colorPreview", {
    text: "Colors",
    tooltip: "Color Preview — click to detect colors",
    command: "color.preview",
  });
}
