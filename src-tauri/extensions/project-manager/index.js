// Project Manager — surfaces the active file's project (parent folder) in the bar.
export function activate(ide) {
  async function refresh() {
    const path = (await ide.editor.getFilePath()) || "";
    const parts = path.split("/").filter(Boolean);
    const project = parts.length >= 2 ? parts[parts.length - 2] : (parts[0] || "—");
    ide.window.setStatusBarItem("projectManager", {
      text: `▸ ${project}`,
      tooltip: "Project Manager — current project",
      command: "project.info",
    });
  }
  ide.commands.register("project.info", async () => {
    const path = (await ide.editor.getFilePath()) || "(no file open)";
    ide.window.showInformationMessage("Current file: " + path);
    await refresh();
  });
  refresh();
}
