import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { TooltipProvider } from "./components/tooltip.jsx";
import "./tailwind.css";

/**
 * vanilla ⇄ React 的唯一接缝。
 *
 * main.js 是 59,731 行命令式 DOM 代码，不会变成 React。需要 shadcn 的地方就在那块 DOM
 * 上挂一个岛：`mountIsland(node, <SettingsPanel/>)`。岛内是 React + Radix + Tailwind，
 * 岛外一切照旧。
 *
 * 为什么要有这个模块，而不是各处直接 createRoot：
 *
 *   1. **根要复用。** 对同一个节点重复 createRoot 会让 React 警告并泄漏上一棵树。这里
 *      用 WeakMap 记住节点 → root，重复挂载走 rerender，节点被回收时 WeakMap 自动放手。
 *   2. **卸载要有出口。** main.js 会整块重建 DOM（重渲染面板、切换项目）。节点被扔掉前
 *      不 unmount，React 的事件监听和 effect 就留在内存里 —— 一个长期开着的 IDE 里这是
 *      会累积的那种泄漏。`unmountIsland(node)` 是那条出口。
 *   3. **Provider 只写一遍。** Tooltip 需要 Provider 包裹，忘了包就静默不显示。放在这里，
 *      调用方不用记。
 */
const ROOTS = new WeakMap();

/**
 * 把 React 元素渲染进一个已存在的 DOM 节点。
 * @param {HTMLElement} node 宿主节点（由 vanilla 侧创建）
 * @param {React.ReactNode} element 要渲染的元素
 * @returns {import("react-dom/client").Root|null}
 */
export function mountIsland(node, element) {
  if (!node) return null;
  // 作用域类：给岛内一个字体/颜色基线，也方便日后需要时收紧样式边界。
  node.classList.add("ui-island");

  let root = ROOTS.get(node);
  if (!root) {
    root = createRoot(node);
    ROOTS.set(node, root);
  }
  root.render(
    <StrictMode>
      <TooltipProvider>{element}</TooltipProvider>
    </StrictMode>,
  );
  return root;
}

/**
 * 卸载一个岛。宿主节点被丢弃前**必须**调用，否则 React 的监听器与 effect 会留下来。
 *
 * unmount 排到微任务里：React 不允许在自己的渲染/commit 期间同步 unmount，而调用方
 * 常常正处在某个事件回调里。延一拍最省心，行为上没有区别。
 */
export function unmountIsland(node) {
  const root = node && ROOTS.get(node);
  if (!root) return;
  ROOTS.delete(node);
  queueMicrotask(() => {
    try { root.unmount(); } catch { /* 节点已被移除，无所谓 */ }
  });
}

/** 这个节点上是否已经有岛。 */
export function hasIsland(node) {
  return !!(node && ROOTS.has(node));
}
