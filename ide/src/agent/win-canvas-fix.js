// Windows 上「代码缩略图（Monaco minimap）会丢失」的修复。
//
// 现象：同一份构建，mac 上 minimap 正常，Windows 上整条空白/看不见。
//
// 根因在渲染路径，不在我们的 CSS，也不在配置。Monaco 的 minimap 是这样画的
// （node_modules/monaco-editor/.../viewParts/minimap/minimap.js）：
//   const ctx = this._canvas.domNode.getContext('2d');            // 没有 willReadFrequently
//   ctx.putImageData(imageData, 0, 0, 0, dirtyY1, imageData.width, dirtyHeight);  // 带「脏矩形」
// 也就是先在 JS 里拼好一整块 ImageData，再用 putImageData 的**脏矩形**形式贴到 canvas 上。
//
// Windows 的 WebView2（Chromium 内核）默认把 2D canvas 放在 GPU 后端。GPU 后端的 canvas
// 在这种「putImageData + 脏矩形」的贴图上有一个长期存在的合成 bug：贴上去的像素不显示，
// 结果就是 minimap 一片空白。mac 的 WKWebView 不走这条 GPU 2D 合成路径，所以看不到问题——
// 这正是「只在 Windows 丢失」的来由。
//
// 修法：在 Windows 上，把 2D canvas 的 getContext 默认加上 { willReadFrequently: true }。
// 这个标志会让 Chromium 给该 canvas 选 **CPU（软件）后端**，于是 putImageData 不再经过
// 出问题的 GPU 合成路径，minimap 恢复。
//
// 为什么这样改是安全的：
//   · 只在 Windows 生效（靠 navigator 判 OS）；mac、以及浏览器构建都不受影响。
//   · 只动 type === "2d"，不碰 webgl / webgl2 / bitmaprenderer——游戏那条是 WebGL，无关。
//   · 调用方要是自己显式表过态（传了 willReadFrequently），一律尊重，不覆盖。
//   · 本应用自己的 2D canvas（图片降采样、视频抽帧）本来就要 toDataURL / getImageData 读回
//     像素，CPU 后端对「频繁读回」反而是更优选择，不会引入性能回退；Monaco 正文是 DOM 渲染
//     的，不是 canvas，所以正文性能也不受影响。
//
// 落点：必须在第一个编辑器（连带它的 minimap canvas）创建之前安装。main.js 在模块体里
// `monaco.editor.create(...)` 之前调用一次 installWindowsCanvasFix() 即可。

// 是否是 Windows。UA 在 WebView2 里形如 "... (Windows NT 10.0; Win64; x64) ...
// Edg/..."，platform 是 "Win32"。两者取其一命中即可。
export function isWindowsAgent(nav) {
  const n = nav ?? (typeof navigator !== "undefined" ? navigator : null);
  if (!n) return false;
  const ua = String(n.userAgent || "");
  const plat = String(n.platform || n.userAgentData?.platform || "");
  return /windows|win32|win64|win\b/i.test(ua) || /^win/i.test(plat);
}

// 在给定 window 上安装补丁。幂等；返回是否真的打了补丁（非 Windows / 环境缺失时返回 false）。
// win/nav 可注入，便于单测。
export function installWindowsCanvasFix(win, nav) {
  const w = win ?? (typeof window !== "undefined" ? window : null);
  if (!w) return false;
  const Canvas = w.HTMLCanvasElement;
  const proto = Canvas && Canvas.prototype;
  if (!proto || typeof proto.getContext !== "function") return false;
  if (proto.__winCanvasFixed) return false; // 已装过，别叠第二层
  if (!isWindowsAgent(nav ?? w.navigator)) return false;

  const original = proto.getContext;
  proto.getContext = function patchedGetContext(type, attrs) {
    if (type === "2d") {
      if (attrs == null || typeof attrs !== "object") {
        attrs = { willReadFrequently: true };
      } else if (!("willReadFrequently" in attrs)) {
        // 别原地改调用方传进来的对象；复制一份再补。
        attrs = { ...attrs, willReadFrequently: true };
      }
    }
    return original.call(this, type, attrs);
  };
  proto.__winCanvasFixed = true;
  return true;
}
