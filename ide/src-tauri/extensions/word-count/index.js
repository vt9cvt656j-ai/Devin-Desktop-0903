// Word Count — a sample Devin IDE extension.
//
// Demonstrates: registering a command, reading the active editor's text, and
// contributing a status-bar item. Runs inside the sandboxed extension worker;
// it can only touch the IDE through the `ide` API object it is given.

export function activate(ide) {
  async function refresh() {
    const text = (await ide.editor.getText()) ?? "";
    const words = (text.match(/\S+/g) || []).length;
    ide.window.setStatusBarItem("wordCount", {
      text: `Words ${words} · Chars ${text.length}`,
      tooltip: "Word Count — click to refresh",
      command: "wordCount.count",
    });
  }

  ide.commands.register("wordCount.count", async () => {
    await refresh();
    ide.window.showInformationMessage("Word Count refreshed");
  });

  // Seed the status-bar item as soon as the extension activates.
  refresh();
}
