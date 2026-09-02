// Polacode — image export needs DOM/canvas the sandbox doesn't expose, so this
// surfaces the framed selection so it can be copied into a screenshot tool.
export function activate(ide) {
  ide.commands.register("polacode.snippet", async () => {
    const sel = await ide.editor.getSelection();
    if (!sel) { ide.window.showInformationMessage("Select code to frame"); return; }
    const lines = sel.split("\n").length;
    ide.window.showInformationMessage(`Framed ${lines} line(s) — copy from the editor into your screenshot tool.`);
  });
  ide.window.setStatusBarItem("polacode", { text: "Shot", tooltip: "Polacode — frame selection", command: "polacode.snippet" });
}
