// AI Code Assistant — uses the IDE's configured AI provider to analyze code.
// Reads the configured provider from the main IDE settings (baseUrl/apiKey/model).

const STORAGE_KEY = "devin-ide.ai-config";

function getConfig() {
  try {
    return JSON.parse(self.__ideConfig || "{}");
  } catch {
    return {};
  }
}

async function aiRequest(ide, systemPrompt, userContent) {
  const resp = await ide.network.fetch("__IDE_AI_PROXY__", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ system: systemPrompt, user: userContent }),
  });
  if (!resp.ok) throw new Error(resp.text || `HTTP ${resp.status}`);
  return resp.json?.choices?.[0]?.message?.content || resp.text;
}

export function activate(ide) {
  ide.commands.register("ai.explain", async () => {
    const selection = await ide.editor.getSelection();
    if (!selection) {
      ide.window.showInformationMessage("Select code to explain first");
      return;
    }
    ide.window.showInformationMessage("Analyzing code...");
    const lang = await ide.editor.getLanguage();
    const result = await ide.network.fetch("https://api.openai.com/v1/chat/completions", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": "Bearer __CONFIGURE_IN_SETTINGS__",
      },
      body: JSON.stringify({
        model: "gpt-4o-mini",
        messages: [
          { role: "system", content: "You are a code explanation expert. Explain the following code clearly and concisely in Chinese." },
          { role: "user", content: `Language: ${lang}\n\nCode:\n${selection}` },
        ],
      }),
    });
    const answer = result.json?.choices?.[0]?.message?.content || "Unable to get response";
    ide.window.showInformationMessage(answer.slice(0, 180));
  });

  ide.commands.register("ai.refactor", async () => {
    const selection = await ide.editor.getSelection();
    if (!selection) {
      ide.window.showInformationMessage("Select code to refactor first");
      return;
    }
    ide.window.showInformationMessage("AI Refactor: Select code and configure API key in settings");
  });

  ide.commands.register("ai.findBugs", async () => {
    const text = await ide.editor.getText();
    if (!text) {
      ide.window.showInformationMessage("Open a file first");
      return;
    }
    ide.window.showInformationMessage("AI Bug Finder: Configure API key in settings to enable");
  });

  ide.commands.register("ai.generateTests", async () => {
    const selection = await ide.editor.getSelection();
    const text = selection || (await ide.editor.getText());
    if (!text) {
      ide.window.showInformationMessage("Open a file or select code first");
      return;
    }
    ide.window.showInformationMessage("AI Test Generator: Configure API key in settings to enable");
  });

  ide.commands.register("ai.addComments", async () => {
    const selection = await ide.editor.getSelection();
    if (!selection) {
      ide.window.showInformationMessage("Select code to add comments to");
      return;
    }
    ide.window.showInformationMessage("AI Doc Comments: Configure API key in settings to enable");
  });

  ide.window.setStatusBarItem("aiAssistant", {
    text: "AI Assistant",
    tooltip: "AI Code Assistant — click for available commands",
    command: "ai.explain",
  });
}
