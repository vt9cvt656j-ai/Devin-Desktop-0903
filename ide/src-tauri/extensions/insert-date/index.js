// Insert Date — a sample Mr. Day One extension.
//
// Demonstrates: registering a command and writing to the active editor through
// the sandboxed `ide` API.

export function activate(ide) {
  ide.commands.register("insertDate.iso", async () => {
    await ide.editor.insertText(new Date().toISOString());
    ide.window.showInformationMessage("Inserted current date");
  });
}
