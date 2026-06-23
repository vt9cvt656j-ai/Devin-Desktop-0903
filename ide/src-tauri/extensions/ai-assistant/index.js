// AI Code Assistant — drives the IDE's configured model (set in Settings) and
// streams answers into the assistant chat panel. The provider and API key live
// in the IDE host, never in this sandboxed extension.

export function activate(ide) {
  // Resolve the code a command should act on. When `needSelection` is true only
  // a selection is acceptable; otherwise we fall back to the whole file. Returns
  // null (after telling the user what to do) when there is nothing to work with.
  async function target(needSelection, emptyMsg) {
    const selection = await ide.editor.getSelection();
    if (needSelection) {
      if (!selection) {
        ide.window.showInformationMessage(emptyMsg);
        return null;
      }
      return selection;
    }
    const text = selection || (await ide.editor.getText());
    if (!text) {
      ide.window.showInformationMessage(emptyMsg);
      return null;
    }
    return text;
  }

  const fence = (lang, code) => "```" + (lang || "") + "\n" + code + "\n```";

  async function run(prompt) {
    ide.window.showInformationMessage("Asking the assistant…");
    await ide.assistant.ask(prompt);
  }

  ide.commands.register("ai.explain", async () => {
    const code = await target(true, "Select code to explain first");
    if (code == null) return;
    const lang = await ide.editor.getLanguage();
    await run(`Explain what the following ${lang} code does, clearly and concisely:\n\n${fence(lang, code)}`);
  });

  ide.commands.register("ai.refactor", async () => {
    const code = await target(true, "Select code to refactor first");
    if (code == null) return;
    const lang = await ide.editor.getLanguage();
    await run(
      `Refactor the following ${lang} code for readability and correctness without changing its behavior. ` +
        `Return the improved code in a fenced block, then briefly list what you changed:\n\n${fence(lang, code)}`,
    );
  });

  ide.commands.register("ai.findBugs", async () => {
    const code = await target(false, "Open a file or select code first");
    if (code == null) return;
    const lang = await ide.editor.getLanguage();
    await run(
      `Review the following ${lang} code for bugs, edge cases, and security issues. ` +
        `List each finding with the offending snippet and a suggested fix:\n\n${fence(lang, code)}`,
    );
  });

  ide.commands.register("ai.generateTests", async () => {
    const code = await target(false, "Open a file or select code to test");
    if (code == null) return;
    const lang = await ide.editor.getLanguage();
    await run(
      `Write thorough unit tests for the following ${lang} code, covering edge cases. ` +
        `Return only the test code in a fenced block:\n\n${fence(lang, code)}`,
    );
  });

  ide.commands.register("ai.addComments", async () => {
    const code = await target(true, "Select code to add comments to");
    if (code == null) return;
    const lang = await ide.editor.getLanguage();
    await run(
      `Add clear documentation comments to the following ${lang} code. ` +
        `Return the fully commented code in a fenced block:\n\n${fence(lang, code)}`,
    );
  });

  ide.window.setStatusBarItem("aiAssistant", {
    text: "AI Assistant",
    tooltip: "AI Code Assistant — explain, refactor, find bugs, generate tests, add comments",
    command: "ai.explain",
  });
}
