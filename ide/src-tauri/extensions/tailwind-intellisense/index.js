// Tailwind Helper — cheatsheet + class counter (the sandbox API has no completion
// provider, so the most-used utilities are surfaced on demand instead).
const CHEATS = "flex grid · p-4 m-2 gap-2 · text-sm font-bold · bg-blue-500 text-white · rounded-lg shadow · w-full h-screen · hover: md: dark:";
export function activate(ide) {
  ide.commands.register("tailwind.cheatsheet", () => {
    ide.window.showInformationMessage("Tailwind: " + CHEATS);
  });
  ide.commands.register("tailwind.count", async () => {
    const text = (await ide.editor.getText()) ?? "";
    const tokens = (text.match(/class(Name)?="([^"]*)"/g) || [])
      .reduce((n, m) => n + (m.match(/[^"\s=]+/g) || []).length, 0);
    ide.window.showInformationMessage(`~${tokens} class tokens in this file`);
  });
  ide.window.setStatusBarItem("tailwindHelper", { text: "TW", tooltip: "Tailwind Helper", command: "tailwind.cheatsheet" });
}
