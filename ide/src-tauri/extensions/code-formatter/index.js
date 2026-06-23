// Code Formatter — basic formatting utilities that work without external tools.

export function activate(ide) {
  ide.commands.register("formatter.trimWhitespace", async () => {
    const text = await ide.editor.getText();
    if (!text) return;
    const trimmed = text.split("\n").map((line) => line.replace(/\s+$/, "")).join("\n");
    if (trimmed !== text) {
      await ide.editor.replaceText(
        { startLineNumber: 1, startColumn: 1, endLineNumber: text.split("\n").length, endColumn: text.split("\n").pop().length + 1 },
        trimmed,
      );
      ide.window.showInformationMessage("Trimmed trailing whitespace");
    } else {
      ide.window.showInformationMessage("No trailing whitespace found");
    }
  });

  ide.commands.register("formatter.sortLines", async () => {
    const selection = await ide.editor.getSelection();
    if (!selection) {
      ide.window.showInformationMessage("Select lines to sort");
      return;
    }
    const sorted = selection.split("\n").sort((a, b) => a.localeCompare(b)).join("\n");
    await ide.editor.insertText(sorted);
    ide.window.showInformationMessage("Lines sorted");
  });

  ide.commands.register("formatter.removeDuplicateLines", async () => {
    const selection = await ide.editor.getSelection();
    if (!selection) {
      ide.window.showInformationMessage("Select lines to deduplicate");
      return;
    }
    const lines = selection.split("\n");
    const seen = new Set();
    const unique = lines.filter((line) => {
      if (seen.has(line)) return false;
      seen.add(line);
      return true;
    });
    const removed = lines.length - unique.length;
    await ide.editor.insertText(unique.join("\n"));
    ide.window.showInformationMessage(`Removed ${removed} duplicate line${removed === 1 ? "" : "s"}`);
  });

  ide.commands.register("formatter.jsonPrettify", async () => {
    const text = await ide.editor.getText();
    if (!text) return;
    const lang = await ide.editor.getLanguage();
    if (lang !== "json") {
      ide.window.showInformationMessage("Current file is not JSON");
      return;
    }
    try {
      const obj = JSON.parse(text);
      const pretty = JSON.stringify(obj, null, 2) + "\n";
      const lines = text.split("\n");
      await ide.editor.replaceText(
        { startLineNumber: 1, startColumn: 1, endLineNumber: lines.length, endColumn: lines[lines.length - 1].length + 1 },
        pretty,
      );
      ide.window.showInformationMessage("JSON formatted");
    } catch (e) {
      ide.window.showInformationMessage("Invalid JSON: " + e.message);
    }
  });

  ide.commands.register("formatter.format", async () => {
    const lang = await ide.editor.getLanguage();
    if (lang === "json") {
      const text = await ide.editor.getText();
      try {
        const obj = JSON.parse(text);
        const pretty = JSON.stringify(obj, null, 2) + "\n";
        const lines = text.split("\n");
        await ide.editor.replaceText(
          { startLineNumber: 1, startColumn: 1, endLineNumber: lines.length, endColumn: lines[lines.length - 1].length + 1 },
          pretty,
        );
      } catch { /* not valid JSON */ }
    }
    const text = await ide.editor.getText();
    const trimmed = text.split("\n").map((line) => line.replace(/\s+$/, "")).join("\n");
    const final = trimmed.endsWith("\n") ? trimmed : trimmed + "\n";
    if (final !== text) {
      const lines = text.split("\n");
      await ide.editor.replaceText(
        { startLineNumber: 1, startColumn: 1, endLineNumber: lines.length, endColumn: lines[lines.length - 1].length + 1 },
        final,
      );
    }
    ide.window.showInformationMessage("Document formatted");
  });

  ide.window.setStatusBarItem("formatter", {
    text: "Format",
    tooltip: "Code Formatter — click to format document",
    command: "formatter.format",
  });
}
