// Svelte Helper — scaffold a basic Svelte single-file component.
const TEMPLATE = `<script>
  let count = 0;
</script>

<button on:click={() => count++}>
  clicked {count} times
</button>

<style>
  button { font-weight: 600; }
</style>
`;
export function activate(ide) {
  ide.commands.register("svelte.scaffold", async () => {
    await ide.editor.insertText(TEMPLATE);
    ide.window.showInformationMessage("Inserted Svelte component scaffold");
  });
  ide.window.setStatusBarItem("svelteHelper", { text: "Svelte", tooltip: "Svelte Helper", command: "svelte.scaffold" });
}
