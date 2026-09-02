// 页签条：让当前那个页签一定看得见。
//
// 从 main.js 搬出来 —— main.js 有行数闸，仓库规矩是「撞线先腾地方，再谈抬线」。
// 只依赖两个参数，没有 DOM 之外的全局，所以能单独测。
//
// # 为什么需要它
//
// 页签条是横向滚动容器，而 `renderTabs` 每次都重建 innerHTML —— scrollLeft 跟着归 0。
// 打开一个排在右边的文件时，内容出来了、页签却停在最左边，用户得自己横滑去找。
//
// # 为什么不用 scrollIntoView
//
// 它会顺带滚动**祖先容器**（编辑器区、整页），在这种嵌套布局里会把别处也带跑。
// 这里只该动页签条自己的 scrollLeft。
export function revealActiveTab(tabsEl, activePath) {
  if (!tabsEl || !activePath) return;
  const el = tabsEl.querySelector(`[data-path="${CSS.escape(activePath)}"]`);
  if (!el) return;
  const pad = 12; // 贴边等于看不全，留一点余量
  const left = el.offsetLeft;
  const right = left + el.offsetWidth;
  const viewLeft = tabsEl.scrollLeft;
  const viewRight = viewLeft + tabsEl.clientWidth;
  // 只在确实看不见时才动 —— 否则用户每点一次页签，条子都会自己跳一下。
  if (left < viewLeft + pad) tabsEl.scrollLeft = Math.max(0, left - pad);
  else if (right > viewRight - pad) tabsEl.scrollLeft = right - tabsEl.clientWidth + pad;
}
