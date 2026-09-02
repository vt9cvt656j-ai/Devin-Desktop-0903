// TODO Highlight — scans for TODO/FIXME/HACK/BUG/NOTE markers and highlights them.

const MARKERS = [
  { pattern: /\bTODO\b/g, className: "todo-marker todo-marker--todo", label: "TODO" },
  { pattern: /\bFIXME\b/g, className: "todo-marker todo-marker--fixme", label: "FIXME" },
  { pattern: /\bHACK\b/g, className: "todo-marker todo-marker--hack", label: "HACK" },
  { pattern: /\bBUG\b/g, className: "todo-marker todo-marker--bug", label: "BUG" },
  { pattern: /\bNOTE\b/g, className: "todo-marker todo-marker--note", label: "NOTE" },
];

export function activate(ide) {
  async function highlight() {
    const text = await ide.editor.getText();
    if (!text) return;

    const decorations = [];
    const lines = text.split("\n");
    let totalCount = 0;

    for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
      const line = lines[lineIdx];
      for (const marker of MARKERS) {
        marker.pattern.lastIndex = 0;
        let match;
        while ((match = marker.pattern.exec(line)) !== null) {
          decorations.push({
            range: {
              startLineNumber: lineIdx + 1,
              startColumn: match.index + 1,
              endLineNumber: lineIdx + 1,
              endColumn: match.index + match[0].length + 1,
            },
            inlineClassName: marker.className,
            hoverMessage: `${marker.label} marker on line ${lineIdx + 1}`,
          });
          totalCount++;
        }
      }
    }

    await ide.editor.setDecorations(decorations);
    ide.window.setStatusBarItem("todoCount", {
      text: `TODOs: ${totalCount}`,
      tooltip: `Found ${totalCount} TODO/FIXME/HACK/BUG/NOTE markers`,
      command: "todo.highlight",
    });
    ide.window.showInformationMessage(`Found ${totalCount} markers`);
  }

  ide.commands.register("todo.highlight", highlight);

  ide.commands.register("todo.clear", async () => {
    await ide.editor.clearDecorations();
    ide.window.removeStatusBarItem("todoCount");
    ide.window.showInformationMessage("TODO highlights cleared");
  });

  ide.commands.register("todo.list", async () => {
    const text = await ide.editor.getText();
    if (!text) return;
    const lines = text.split("\n");
    const found = [];
    for (let i = 0; i < lines.length; i++) {
      for (const marker of MARKERS) {
        marker.pattern.lastIndex = 0;
        if (marker.pattern.test(lines[i])) {
          found.push(`L${i + 1}: ${lines[i].trim()}`);
        }
      }
    }
    if (found.length === 0) {
      ide.window.showInformationMessage("No TODO markers found");
    } else {
      ide.window.showInformationMessage(found.slice(0, 5).join(" | "));
    }
  });

  highlight();
}
