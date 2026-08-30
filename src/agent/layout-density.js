// 三栏布局的自适应档位：窗口越窄，两条侧栏让得越多，中间编辑区才留得住。
//
// # 为什么量的是物理宽度
//
// 界面缩放走 `documentElement.style.zoom`（见 main.js 的 _applyUiZoom），1 CSS px 会变成
// zoom 个物理像素。app.css 里三栏的宽度已经除以 `--ui-zoom`，各栏的**物理**宽度因此与
// 缩放无关 —— 缩放只把字和图标变大变小，谁占屏幕多少始终不变。
//
// 档位要是再按 CSS 像素分（媒体查询量的就是 CSS 像素 = 物理 / 缩放），等于把缩放算了
// 两次：放大一次、档位又缩一次，窄窗口里一放大侧栏反而更小。所以这里读的是
// `window.innerWidth` —— 实测它不随 zoom 变，就是物理可用宽度。
//
// # 为什么住在这里而不是 main.js
//
// main.js 有行数闸（test/main-size-budget.test.mjs），仓库规矩是「撞线先腾地方」。
// 这段只依赖参数、没有模块级可变状态，正好搬得动，也因此能被单独测。

/** [触发宽度(物理 px), 档位名]，从宽到窄。窗口宽度小于哪一档就取哪一档，取最后命中的。 */
export const LAYOUT_DENSITY_STEPS = [[1180, "narrow"], [980, "tight"], [780, "min"]];

/** 纯函数：给定物理宽度，算出档位名（宽窗口返回空串 = 不设档）。 */
export function layoutDensityStep(width, steps = LAYOUT_DENSITY_STEPS) {
  const w = Number(width) || 0;
  let step = "";
  if (w > 0) for (const [px, name] of steps) if (w < px) step = name;
  return step;
}

/**
 * 把档位打到根元素的 `data-layout` 上；CSS 那边拿 `max-width` 封顶。
 *
 * **封顶而不是改 `--sidebar-w`/`--assistant-w`**：那两个变量是拖分隔条存下来的（内联在
 * `.layout` 上），改掉等于把用户拖出来的宽度抹了，窗口再拉宽也回不来。封顶只是暂时压住，
 * 窗口一宽就自动松开。
 */
export function applyLayoutDensity(width, rootEl) {
  if (!rootEl || !rootEl.dataset) return "";
  const step = layoutDensityStep(width);
  if ((rootEl.dataset.layout || "") === step) return step;
  if (step) rootEl.dataset.layout = step;
  else delete rootEl.dataset.layout;
  return step;
}

/*
 * 视口尺寸有**两种口径，绝不能混用**：
 *
 *   · `window.innerWidth/Height` —— **物理**像素，不随界面缩放变（实测 2026-08-29）。
 *   · `documentElement.clientWidth/Height` —— **CSS** 像素 = 物理 / 缩放，也就是布局坐标系。
 *
 * `getBoundingClientRect()`、`clientX/Y`、`style.left/top` 全都是 CSS 像素。所以任何
 * 「把一个矩形夹进视口」的计算都必须用后者。
 *
 * 混用的后果就是用户看到的那一幕：放到 140% 时 innerWidth 比实际布局宽度大 40%，
 * `Math.min(r.left, innerWidth - w - 8)` 这个上界永远大于 r.left，夹取一次都不触发，
 * 模型菜单、悬浮卡、右键菜单直接飞出屏幕右下角 ——「放大缩小其他 UI 内容也都会乱飞」。
 *
 * 反过来，只有两处该用物理口径：缩放上限（问的是「这块屏幕有多大」）和上面的自适应
 * 档位（问的是「这块屏幕能放下几栏」）。这两处都在注释里写明了。
 */
export function viewportW() {
  const el = typeof document !== "undefined" ? document.documentElement : null;
  return (el && el.clientWidth) || (typeof window !== "undefined" ? window.innerWidth : 0) || 0;
}
export function viewportH() {
  const el = typeof document !== "undefined" ? document.documentElement : null;
  return (el && el.clientHeight) || (typeof window !== "undefined" ? window.innerHeight : 0) || 0;
}
