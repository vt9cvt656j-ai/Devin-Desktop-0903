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
    closeTab, onAccept, onDoc, isPlanMode,
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

  // 流式过程中就往这一屏写，不等回合跑完。300ms 节流 —— 跟得上眼睛，又不会每个
  // token 都重排一次 markdown。挂在 main.js 的 _streamDraftSave 上，因为那是流式
  // 过程中**带着累积文本反复被调用**的唯一一处；另找收尾点只会两边各说各话。
  let liveAt = 0;
  let liveTurn = null;

  return {
    state,
    render,
    /** @param turn 这一轮的会话 id；@param md 到目前为止累积的回复文本 */
    liveUpdate(turn, md) {
      if (!isPlanMode()) return;
      const text = String(md || "");
      // **等到出现第一个小节标题再开页签**，不是一有字就开：模型开头总有一两句引子，
      // 那时开等于刚吐两个字就把编辑区抢走，比不写还烦。
      if (!openFiles.has(tabPath)) {
        if (!/^#{1,6}\s+\S/m.test(text)) return;
        liveTurn = turn; liveAt = Date.now();
        this.openFromReply(text);           // 第一次开：跟过去，让用户看见它在长
        return;
      }
      if (liveTurn !== turn) return;        // 上一轮留下的页签，这一轮还没轮到它
      const now = Date.now();
      if (now - liveAt < 300) return;
      liveAt = now;
      this.openFromReply(text, { focus: false }); // 已经开着：只更新，不抢焦点
    },
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
