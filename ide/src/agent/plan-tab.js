// Plan 模式那一屏：虚拟页签 + 方案正文 + 底部「用不用这个方案」。
//
// 整块住在这里而不是 main.js：main.js 有行数闸（test/main-size-budget.test.mjs），
// 仓库规矩是「撞线先腾地方，再谈抬线」。依赖全部注入 —— 这一层不认识全局，
// 也因此能被单独测。
export function createPlanTab(deps) {
  const {
    editorContainer, renderMarkdownInto, sendPrompt,
    planCoreFromReply, planTitleFromReply,
    tabPath, tabName,
    openFiles, renderTabs, syncWelcome, activate, getActivePath,
    closeTab, onAccept, onDoc,
  } = deps;
  const state = { el: null, md: "", title: tabName };

  function ensure() {
    if (state.el) return state.el;
    const pane = document.createElement("div");
    pane.className = "planpane";
    pane.id = "planPane";
    pane.innerHTML = `
      <div class="planpane__head"><span class="planpane__title"></span></div>
      <div class="planpane__body"></div>
      <div class="planpane__foot">
        <span class="planpane__hint">也可以直接在对话框里说要改哪儿</span>
        <button type="button" class="planpane__btn" data-plan="dismiss">先不用</button>
        <button type="button" class="planpane__btn planpane__btn--go" data-plan="go">按这个方案执行</button>
      </div>`;
    pane.addEventListener("click", (e) => {
      const act = e.target.closest("[data-plan]")?.dataset.plan;
      if (act === "dismiss") { closeTab(); return; }
      if (act === "go") {
        // Plan 是只读模式，留在这儿它不会动手 —— 先切回 Agent，再把方案原文发回去。
        try { onAccept(); } catch {}
        sendPrompt(`按下面这个方案执行，逐条落地并自己验证：\n\n${state.md}`);
        closeTab();
      }
    });
    editorContainer.appendChild(pane);
    state.el = pane;
    return pane;
  }

  function render() {
    const pane = ensure();
    pane.querySelector(".planpane__title").textContent = state.title || tabName;
    const body = pane.querySelector(".planpane__body");
    while (body.firstChild) body.removeChild(body.firstChild);
    try { renderMarkdownInto(body, state.md); } catch { body.textContent = state.md; }
  }

  return {
    state,
    render,
    show() { ensure(); state.el.hidden = false; render(); },
    hide() { if (state.el) state.el.hidden = true; },
    /**
     * 用一份 Plan 回复打开/更新方案页签。
     *
     * **抽不出核心内容就不开**（返回 false）：空白页比没有更糟 ——
     * 「这一轮没有可执行内容」不该以一张空页的形式呈现。
     */
    openFromReply(markdown, { focus = true } = {}) {
      const core = planCoreFromReply(markdown);
      if (!core) return false;
      state.md = core;
      state.title = planTitleFromReply(markdown, tabName);
      try { onDoc(state.md, state.title); } catch {}
      if (!openFiles.has(tabPath)) {
        openFiles.set(tabPath, { model: null, name: tabName, dirty: false, viewState: null, isPlan: true });
        syncWelcome();
      }
      renderTabs();
      if (focus) activate(tabPath);
      else if (getActivePath() === tabPath) render();
      return true;
    },
  };
}
