import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { Shell } from "./app/Shell.jsx";
import "./styles/app.css";
import "./styles/shadcn.css";
// 精修层排在 shadcn.css 之后、tailwind 之前：它重写的是 app.css 里的排版与间距，
// 靠源码顺序取胜，提前就被 app.css 盖回去了。
import "./styles/refine.css";
import "./ui/tailwind.css";

/**
 * 应用入口。顺序是这里唯一重要的东西。
 *
 * main.js 在**模块顶层**就抓 DOM：
 *
 *     const treeEl = $("tree");
 *     const tabsEl = $("tabs");
 *
 * 也就是说它一被 import，元素就必须已经在文档里了。React 18/19 的 `root.render()`
 * 默认是并发、异步提交的 —— 直接 `render(); import("./main.js")` 会让 main.js 抢在
 * commit 之前跑，那 159 个 ref 全是 null，整个 IDE 静默死掉。
 *
 * `flushSync` 强制同步提交：这一行返回时，DOM 已经在文档里了。这是全应用**唯一**
 * 需要 flushSync 的地方，代价只有首屏这一次，换来的是启动顺序确定。
 *
 * 然后才动态 import main.js —— 静态 import 会被提升到 flushSync 之前，那就前功尽弃。
 */
const host = document.getElementById("root");
if (!host) throw new Error("boot: #root 不存在，index.html 被改坏了");

const root = createRoot(host);
flushSync(() => {
  root.render(<Shell />);
});

// 到这里 159 个 ID 都在文档里了，main.js 可以安全地抓它的 ref。
await import("./main.js");
