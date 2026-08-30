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
        <button type="button" class="planpane__btn planpane__btn--go" data-plan="go">按这个方案执行</button>
        <button type="button" class="planpane__btn" data-plan="dismiss">先不用</button>
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
  // 一轮里模型分好几条消息说话（每跑一步一条），而 liveUpdate 每次只拿到**当前那条**的
  // 累积文本。所以「换了一条消息」必须往后接，不能顶掉前面那条 —— 否则收尾那条
  // 「方案交付完毕。给执行者的下一句话……」一到，整份方案就被两段话替换掉了，
  // 用户看到的正是「写的时候明明很多，写完只剩一点」。
  let liveDone = [];   // 本轮已经说完的那几条
  let liveCur = "";    // 当前这条累积到哪儿
  let liveShown = false; // 这一轮往面板里写过没有
  function accumulate(turn, text) {
    if (liveTurn !== turn) { liveTurn = turn; liveDone = []; liveCur = ""; liveShown = false; liveAt = 0; }
    if (text.startsWith(liveCur)) liveCur = text;                        // 同一条在长
    else { if (liveCur.trim()) liveDone.push(liveCur); liveCur = text; } // 换了一条
    return [...liveDone, liveCur].filter((s) => s.trim()).join("\n\n");
  }

  return {
    state,
    render,
    /** @param turn 这一轮的会话 id；@param md 到目前为止累积的回复文本 */
    liveUpdate(turn, md) {
      if (!isPlanMode()) return;
      const doc = accumulate(turn, String(md || ""));
      // **等到出现第一个小节标题再开页签**，不是一有字就开：模型开头总有一两句引子，
      // 那时开等于刚吐两个字就把编辑区抢走，比不写还烦。
      // 判据是「这一轮写过没有」，不是「页签开着没有」：页签开着但换了新一轮时，按后者
      // 会直接掉进下面的节流分支 —— 上一轮的方案要么被新一轮的第一个 token 顶掉，要么
      // 被节流挡住一直挂着不动。两种都不对。
      if (!liveShown) {
        if (!/^#{1,6}\s+\S/m.test(doc)) return;
        liveShown = true; liveAt = Date.now();
        // 第一次开才跟过去；页签已经在了就别抢焦点，用户可能正在别的文件里。
        this.openFromReply(doc, { focus: !openFiles.has(tabPath) });
        return;
      }
      const now = Date.now();
      if (now - liveAt < 300) return;
      liveAt = now;
      this.openFromReply(doc, { focus: false }); // 已经开着：只更新，不抢焦点
    },

    // 回合收尾。**照样过累积器**：收尾那条消息只是本轮的最后一段，不是全文，
    // 直接拿它开窗就等于把前面几段扔了。流式没跑过时 liveDone 是空的，
    // 这里退化成「就用这一条」—— 和以前的行为一样。
    commit(turn, md) {
      if (!isPlanMode()) return false;
      return this.openFromReply(accumulate(turn, String(md || "")));
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
