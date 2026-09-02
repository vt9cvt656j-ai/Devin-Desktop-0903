// Translate Helper — offline helpers for translation work (no network needed):
// analyze CJK/Latin ratio and wrap a selection in a translation marker.
export function activate(ide) {
  ide.commands.register("translate.analyze", async () => {
    const text = (await ide.editor.getText()) ?? "";
    const cjk = (text.match(/[一-鿿ぁ-ヿ]/g) || []).length;
    const latin = (text.match(/[A-Za-z]/g) || []).length;
    ide.window.showInformationMessage(`CJK ${cjk} · Latin ${latin} chars`);
  });
  ide.commands.register("translate.mark", async () => {
    const sel = await ide.editor.getSelection();
    if (!sel) { ide.window.showInformationMessage("Select text to mark"); return; }
    await ide.editor.insertText(`/* TODO translate: ${sel.replace(/\*\//g, "* /")} */`);
  });
  ide.window.setStatusBarItem("translateHelper", { text: "译", tooltip: "Translate Helper", command: "translate.analyze" });
}
