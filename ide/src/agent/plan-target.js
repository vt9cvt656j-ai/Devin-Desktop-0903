/*
 * 计划步骤的**交付物**判据。
 *
 * 自动打勾此前只看动作类别（investigate / implement / verify / execute）：一步说
 * 「实现 src/pages/login.tsx 的表单校验」，模型去改了完全不相干的
 * `src/utils/date.ts`，证据类别照样是 implement，于是这一步被打成完成。用户看到的
 * 是进度条在走、活没干——「假完成」的另一种形状，比分不出类那种更难发现，因为它
 * 每一步都对得上类别。
 *
 * 这里只做一件事：**当步骤自己点了名**（写出了带扩展名的文件、带斜杠的路径、或反引号
 * 里的标识符），而这次工具调用**确实动了某个具体目标**，两边却一个都对不上时，说
 * 「对不上」。除此之外一律沉默——它只能**拒绝**一个勾，永远不会新增一个勾，因为这条
 * 判据要防的就是假完成。
 *
 * 纯字符串逻辑：不碰 DOM、不读模块级状态、只依赖参数。
 */

// 带扩展名的文件名（login.tsx / package.json / main.rs），或带斜杠的路径片段。
// 扩展名必须**以字母开头**：版本号（v1.2 / 1.5.0 / 第 2.1 版）的小数尾巴全是数字，
// 而真扩展名没有纯数字的（ts/tsx/json/rs/py/yml/lock…）。少了这条，「我在做第 2 版」
// 里的 v1.2 会被当成一个文件名，然后拿去和真实改动比对。
const _FILE_TOKEN = /[A-Za-z0-9_.\-@/]*[A-Za-z0-9_\-]\.[A-Za-z][A-Za-z0-9]{0,7}\b/g;
const _PATH_TOKEN = /[A-Za-z0-9_.\-@]+(?:\/[A-Za-z0-9_.\-@*]+)+/g;
// 反引号 / 单引号里被点名的东西：`useAuth`、'/api/v2/orders'。
const _QUOTED_TOKEN = /[`'"]([^`'"\n]{2,80})[`'"]/g;
// 「看着像文件、其实是库名/域名」的裸词。扩展名一侧列的是**包和域名后缀**，
// 不是源码扩展名 —— `app.ts` 不该被这条挡掉，`socket.io` 该。
const _LIBRARY_LIKE = /\.(?:js|ts|io|dev|com|net|org|cn|ai|sh|app|co|gg)$/i;

/** 一段路径里所有可比对的片段（含 basename）。全小写。 */
function _segments(path) {
  return String(path || "")
    .replace(/\\/g, "/")
    .toLowerCase()
    .split("/")
    .map((part) => part.trim())
    .filter(Boolean);
}

/**
 * 步骤正文里点名的交付目标。
 *
 * 返回的是**小写片段**，不是原文：两边比对必须走同一套归一化，否则
 * `src/Pages/Login.tsx` 和 `src/pages/login.tsx` 会被判成两回事。
 */
export function planStepTargets(step) {
  const text = String(
    (step && (step.content || step.title || step.description)) || step || "",
  );
  if (!text) return [];
  const out = new Set();
  const push = (raw) => {
    // 只剥**结尾**的标点，开头的点不剥：`.eslintrc.json` / `.env.local` 是真实文件名，
    // 剥掉之后和另一侧（toolTouchedTargets 走 _segments，不剥）就不对称了 ——
    // 改对了根目录点文件，反而被判成「动的是别处」。
    const value = String(raw || "").trim().replace(/[.,;:，。；：、)）】\]]+$/g, "").replace(/^\/+/, "");
    if (!value || value.length > 200) return;
    // 目录写法（src/、tests/）留下目录名本身。
    for (const seg of _segments(value)) {
      // 单字符片段（a、x）和纯数字片段（2、v1 里的 1）当不了判据，噪音大于信号。
      if (seg.length >= 2 && !/^\d+$/.test(seg)) out.add(seg);
    }
  };
  for (const m of text.matchAll(_FILE_TOKEN)) {
    // 带点的**库名和域名**不是文件：chart.js / socket.io / three.js / next.js /
    // react.dev / lodash.debounce / node.js —— 步骤描述里出现它们再正常不过
    // （「用 chart.js 画个折线图」），而一旦被当成点名的交付物，这一步就再也勾不上了。
    // 判据是「它带路径分隔符吗」：真要指一个文件时，人几乎总会写出所在目录
    // （src/chart.js），而说库名时不会。裸文件名只在它自己也出现在改动目标里时才成立，
    // 那种情况下面的 _PATH_TOKEN 那一支会覆盖到。
    if (!m[0].includes("/") && _LIBRARY_LIKE.test(m[0])) continue;
    push(m[0]);
  }
  for (const m of text.matchAll(_PATH_TOKEN)) push(m[0]);
  for (const m of text.matchAll(_QUOTED_TOKEN)) {
    // 反引号里常常是一整句中文说明，不是标识符。只收看起来像路径/标识符的。
    const inner = m[1].trim();
    if (/^[A-Za-z0-9_.\-@/]{2,80}$/.test(inner)) push(inner);
  }
  return [...out];
}

/**
 * 这次工具调用真正动到/看到的目标。
 *
 * 只收**结构字段**（模型自己填的参数），不从命令行正文里猜：猜错会让一个本该打勾的
 * 步骤打不上，而打不上是没有提示的，比误勾更难查。
 */
export function toolTouchedTargets(call) {
  if (!call || typeof call !== "object") return [];
  const raw = [];
  const take = (value) => {
    if (typeof value === "string" && value.trim()) raw.push(value.trim());
  };
  // **判据是 `_resolvedPath`，不是 `call.path`。**
  //
  // `call.path` 在 _mapToolCall 里是**卡片标题槽位**，不是文件路径：`search` 往里塞
  // `args.query`、`web_search` 塞查询串、`web_fetch` 塞 URL、`ui_extract` 塞 glob、
  // `local_discovery` 塞地点、`design_research` 塞的干脆是「设计调研」四个字。拿它当路径
  // 比对，等于让一次搜索的关键词去和步骤点名的文件求交集 —— 必然对不上，于是一个点了名
  // 的步骤会被任意一次取证调用**永久挡住**（这条判据只会拒绝勾，所以后果是勾不上）。
  //
  // `_resolvedPath` 是文件操作的处理函数在真正落盘/读盘时写回的绝对路径（_resolveRel /
  // _resolveExisting 的结果），只有真的动了文件才有。这是结构性的，措辞和工具改名都动不了它。
  take(call._resolvedPath);
  if (Array.isArray(call.edits)) for (const e of call.edits) take(e?.path || e?.filePath || e?.file_path);
  // dest / to 只在**确实是文件操作**（有 _resolvedPath）时才读：move/copy 的目标端。
  // 别处的 `dest` 是别的意思（debug_control 的调试目标、worktree 的目录别名）。
  if (call._resolvedPath) { take(call.dest); take(call.to); }
  const out = new Set();
  for (const value of raw) for (const seg of _segments(value)) if (seg.length >= 2) out.add(seg);
  return [...out];
}

/** 带扩展名的那些片段 —— 一段路径里最具体的一层。 */
const _named = (list) => list.filter((token) => token.includes("."));

/**
 * 「这一步点了名，而这次调用动的是别处」——真的对不上才返回 true。
 *
 * 任何一侧为空都返回 false（＝不表态）：步骤没点名就没有交付物判据可言，调用没有
 * 结构化目标（跑命令、开浏览器）同样无从比对。这两种情形交回给动作类别那条判据。
 *
 * **比对要落在最具体的那一层。** 直接拿全部片段求交集是没用的：`src` 这种目录名几乎
 * 每条路径里都有，一比就命中，判据恒为「不冲突」。所以两边只要都点到了具体文件，
 * 就只比文件名；否则退回比目录名（更松，也就更沉默——这是对的，它只该拒绝勾）。
 */
export function targetsConflict(stepTargets, touchedTargets) {
  const wanted = Array.isArray(stepTargets) ? stepTargets : [];
  const touched = Array.isArray(touchedTargets) ? touchedTargets : [];
  if (!wanted.length || !touched.length) return false;
  const wantedFiles = _named(wanted);
  const touchedFiles = _named(touched);
  const [left, right] = wantedFiles.length && touchedFiles.length
    ? [wantedFiles, touchedFiles]
    : [wanted, touched];
  const set = new Set(right);
  return !left.some((target) => set.has(target));
}
