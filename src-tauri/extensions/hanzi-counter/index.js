// Hanzi Counter — Chinese character / word / total counts in the status bar.
export function activate(ide) {
  async function refresh() {
    const text = (await ide.editor.getText()) ?? "";
    const hanzi = (text.match(/[一-鿿]/g) || []).length;
    const words = (text.match(/[A-Za-z]+/g) || []).length;
    ide.window.setStatusBarItem("hanziCounter", {
      text: `汉字 ${hanzi} · 词 ${words} · 字符 ${text.length}`,
      tooltip: "Hanzi Counter — click to refresh",
      command: "hanzi.count",
    });
  }
  ide.commands.register("hanzi.count", async () => {
    await refresh();
    ide.window.showInformationMessage("Hanzi counts refreshed");
  });
  refresh();
}
