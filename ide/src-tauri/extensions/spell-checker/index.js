// Spell Checker — flags a built-in set of common misspellings via diagnostics.
const TYPOS = {
  teh: "the", recieve: "receive", seperate: "separate", occured: "occurred",
  definately: "definitely", neccessary: "necessary", accross: "across",
  wich: "which", thier: "their", untill: "until", begining: "beginning",
  enviroment: "environment", existance: "existence", lenght: "length",
  retreive: "retrieve", succesful: "successful", calender: "calendar",
};
export function activate(ide) {
  async function check() {
    const text = (await ide.editor.getText()) ?? "";
    const path = (await ide.editor.getFilePath()) || "file";
    const lines = text.split("\n");
    const diags = [];
    for (let i = 0; i < lines.length; i++) {
      const re = /[A-Za-z]+/g;
      let m;
      while ((m = re.exec(lines[i])) !== null) {
        const fix = TYPOS[m[0].toLowerCase()];
        if (fix) diags.push({
          severity: "warning",
          message: `"${m[0]}" may be a misspelling of "${fix}"`,
          startLine: i + 1, startColumn: m.index + 1,
          endLine: i + 1, endColumn: m.index + m[0].length + 1,
        });
      }
    }
    await ide.diagnostics.set(path, diags);
    ide.window.showInformationMessage(`Spell check: ${diags.length} issue(s)`);
  }
  ide.commands.register("spell.check", check);
  ide.commands.register("spell.clear", async () => {
    await ide.diagnostics.clear((await ide.editor.getFilePath()) || "file");
    ide.window.showInformationMessage("Spell check cleared");
  });
  ide.window.setStatusBarItem("spellChecker", { text: "Spell", tooltip: "Spell Checker — click to check", command: "spell.check" });
}
