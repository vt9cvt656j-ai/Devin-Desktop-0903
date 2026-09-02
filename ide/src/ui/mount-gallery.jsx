import { mountIsland, unmountIsland } from "./island.jsx";
import { Gallery } from "./gallery.jsx";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "./components/dialog.jsx";

/**
 * 从 vanilla 侧打开组件长廊：控制台里敲 `showUIGallery()`。
 *
 * 这个文件就是"岛"的完整用法示例，一共三步：建宿主节点 → mountIsland → 关掉时 unmountIsland。
 * 之后把任何一个现有面板换成 shadcn，都是同样三步。
 */
function GalleryDialog({ onClose }) {
  return (
    <Dialog defaultOpen onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>shadcn / ui — 你的配色</DialogTitle>
          <DialogDescription>
            全部组件走项目自己的 token。切换深浅色（⌘+Shift+L 或设置里）看两套配色下的表现。
          </DialogDescription>
        </DialogHeader>
        <Gallery />
      </DialogContent>
    </Dialog>
  );
}

export function showUIGallery() {
  const existing = document.getElementById("ui-gallery-host");
  if (existing) { close(existing); return; }

  const host = document.createElement("div");
  host.id = "ui-gallery-host";
  document.body.appendChild(host);
  mountIsland(host, <GalleryDialog onClose={() => close(host)} />);
}

function close(host) {
  unmountIsland(host);
  // unmountIsland 把 unmount 排进了微任务，宿主节点要等它之后再移除，
  // 否则 React 卸载时节点已经不在文档里。
  queueMicrotask(() => queueMicrotask(() => host.remove()));
}

if (typeof window !== "undefined") window.showUIGallery = showUIGallery;
