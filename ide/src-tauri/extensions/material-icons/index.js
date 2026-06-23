// Material Icons — file-icon theme. The sandboxed extension API can't contribute
// an icon theme directly, so this registers an informational entry; the IDE's
// built-in explorer icons already follow the Material set.
export function activate(ide) {
  ide.commands.register("materialIcons.info", () => {
    ide.window.showInformationMessage("Material file icons are built into the explorer.");
  });
  ide.window.setStatusBarItem("materialIcons", { text: "Icons", tooltip: "Material Icons", command: "materialIcons.info" });
}
