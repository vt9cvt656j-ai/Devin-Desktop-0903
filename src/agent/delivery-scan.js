// 从 src/main.js 抽出。判据是内聚 + 边界干净：这一族对 main.js 其余部分的引用
// 只剩 code-text.js 的 splitCodeAndComments 和 paths.js 的 _CODE_FILE_RE 两个；调用点全部在族内，或经 import 显式接回。
//
// 本目录下的 import / export 必须写成**单物理行**：test/helpers/source.mjs 拼 SRC 时
// 按行过滤 `^\s*import`，多行写法只删得掉第一行，剩下的那半行会让顶层 acorn.parse
// 当场 SyntaxError —— 十几个测试文件一起在 import 期崩溃，而不是某条断言变红。
import { splitCodeAndComments as _splitCodeAndComments } from "./code-text.js";
import { _CODE_FILE_RE } from "./paths.js";

/// 落盘内容里**新引入**的占位实现。纯本地扫描，零模型调用。
///
/// 用户原话：「完成真正的产品而不是虚假的」。此前对"假交付"没有任何执行事实层面的判据——
/// 判文件性质的几个函数全都只看路径，从没有一处看过内容，而本轮每个文件的完整终态
/// 一直躺在 checkpoint 里（`content` 是改前原文，`current` 是智能体最后写进去的）。
/// 于是「禁止留 TODO/占位/假数据」在提示词里是一句无人对账的劝告。
///
/// 三条纪律，缺一条就会变成噪音或冤枉人：
///   · **基线相减**：只报 `current` 里有、而改前 `content` 里没有的那些行。文件里本来就有的
///     TODO 不算这次交付的账——和「诊断只认新增错误」同一套哲学。
///   · **只报有原文佐证的字面命中**：给得出文件+行号+这一行的原文。不做语义猜测，
///     不去判断"这个函数是不是真的实现了"。
///   · **只陈述，不发红灯也不发绿灯**：它进交付事实行和未完成原因，不参与验证学分。
/// 这一轮**删掉或改名**了哪些导出/函数声明，而且从没查过谁在用它们。
///
/// 「项目动不动被写坏」「细节处理不够」里，最常见也最可精确检测的一种：改一个文件时
/// 顺手把某个函数/导出去掉了，而它在别处被引用着。提示词里写着「改已有文件前必须知道
/// 它的调用方」，可全仓没有任何东西检查过这件事——lsp_references 摆在那儿，用不用全凭自觉。
///
/// 判据全部是执行事实，两条都必须成立才报：
///   · 改前的原文里有这条声明，改后没有了（基线相减，和诊断只认新增错误同一套哲学）；
///   · 本 run 里从没出现过这个名字的检索（search / lsp_references / find_symbol 的查询词）。
/// 单独一条都可能正当——删死代码是对的，查过之后删也是对的。两条同时成立才是"没看就删"。
export function _removedDeclarationsUnchecked(run, searchedTerms, maxItems = 6) {
  const cp = run?.checkpoint;
  if (!cp || typeof cp.forEach !== "function") return [];
  // **只认对外可见的声明**。文件内部的私有 helper 删掉是安全的——它只可能被这个文件用到，
  // 而这个文件的全文刚刚被智能体重写过。放进来会造成一类很常见的误报：读完整个文件、
  // 把只此一处用到的小函数内联掉、删了原声明——完全正当，却会被判成"没查引用就删"。
  // 一次误报把正常成功变成"未完成"，代价不比漏报小。
  //   JS/TS：必须带 export     Rust：必须带 pub     Python：必须是顶格 def（模块级即对外）
  const DECL = /^export\s+(?:async\s+)?(?:function|class)\s+([A-Za-z_$][\w$]*)|^export\s+(?:const|let|var)\s+([A-Za-z_$][\w$]*)|^pub\s+(?:async\s+)?fn\s+([A-Za-z_$][\w$]*)|^def\s+([A-Za-z_$][\w$]*)/;
  const names = (text) => {
    const out = new Set();
    for (const line of String(text || "").split("\n")) {
      const m = DECL.exec(line);
      if (m) out.add(m[1] || m[2] || m[3] || m[4]);
    }
    return out;
  };
  const searched = new Set([...(searchedTerms instanceof Set ? searchedTerms : [])].map((x) => String(x || "").toLowerCase()));
  const out = [];
  cp.forEach((snap, absPath) => {
    if (out.length >= maxItems) return;
    const cur = snap?.current;
    if (typeof cur !== "string" || !snap?.existed || !_CODE_FILE_RE.test(String(absPath))) return;
    const after = names(cur);
    for (const name of names(snap.content)) {
      if (out.length >= maxItems) break;
      if (after.has(name)) continue;
      // 查过就不算。判据放宽到"检索词里出现过这个名字"——精确匹配太严，
      // 模型常搜 "renderPlan(" 或 "调用 renderPlan" 这类形态。
      if ([...searched].some((q) => q.includes(name.toLowerCase()))) continue;
      out.push({ path: String(absPath).split("/").slice(-2).join("/"), name });
    }
  });
  return out;
}


function _sinkRisksInWrite(text, before = null, path = "") {
  const src = String(text || "");
  if (!src) return [];
  const RULES = [
    [/\b(?:SELECT|INSERT\s+INTO|UPDATE|DELETE\s+FROM)\b[\s\S]{0,160}?\b(?:WHERE|VALUES|SET|LIKE|HAVING)\b[^`"'\n]{0,40}(?:\$\{\s*[\w.\[\]]+\s*\}|\{\s*[\w.\[\]]+\s*\}|["']\s*\+\s*[\w.]+|%s["']\s*%\s*[\w(])/i,
      "SQL 拼接", "改成参数绑定（? / $1 / :name）。顺带确认这条查询按调用者的身份过滤了行——只按请求里的 id 取记录就是 IDOR。"],
    [/dangerouslySetInnerHTML|\bv-html\b|\.outerHTML\s*=\s*[`"'][^`"']*\$\{|insertAdjacentHTML\s*\(\s*[^,]+,\s*[`"'][^`"']*\$\{/,
      "HTML 汇聚", "这条路径上的内容只要有一段来自用户，就是存储型 XSS。要么改成 textContent，要么先过白名单净化（DOMPurify 一类），别自己写转义。"],
    [/\beval\s*\(|new\s+Function\s*\(/,
      "动态求值", "传进去的串只要有一段来自外部就是任意代码执行。换成显式的分支/查表；确实要沙箱执行的，用真沙箱而不是 eval。"],
    [/\b(?:execSync|exec|spawnSync|spawn|system|popen|Command::new)\s*\(\s*[`"'][^`"']*\$\{|shell\s*[:=]\s*true/,
      "命令注入", "别把变量拼进命令串。改成参数数组形式（execFile / spawn(cmd, [args])），并关掉 shell:true。"],
    [/\bpickle\.loads?\s*\(|yaml\.load\s*\((?![^)]*Safe)|Marshal\.load|ObjectInputStream/,
      "不安全反序列化", "这几个能在反序列化过程中直接执行代码。换 safe_load / JSON / 带类型白名单的解析器。"],
    [/Object\.assign\s*\(\s*\w+\s*,\s*(?:req|request|ctx)\.body|\{\s*\.\.\.\s*(?:req|request|ctx)\.body\s*\}/,
      "批量赋值", "整包绑上去，调用方就能自己设 role / isAdmin / ownerId。只挑这次允许改的字段。"],
  ];
  const seen = before instanceof Set ? before : null;
  const lines = src.split("\n");
  // 判据只看**代码**：注释掉的危险写法不算数（实测模型会在注释里贴一段"老写法，已废弃"，
  // 然后被告知它写了 SQL 注入）。反过来注释里的辩解也拦不住——注释一个字都不参与判定。
  const codeLines = _splitCodeAndComments(src, path).code;
  // 按**类别**去重、首次命中为准。两个理由：
  //   ① 3 行窗口会让同一处连报三次（窗口从第 1、2、3 行各命中一次），刷屏且全是同一条。
  //   ② 每类的处置办法是同一句，重复说没有任何新信息——真正的信息在「哪一行、原文是什么」。
  // 每个窗口要测**全部**规则、不 break：同一段代码常常同时踩两类（实测一段 7 行的
  // handler 里 SQL 拼接 + 批量赋值 + HTML 汇聚三类同时成立），先命中的那条不该把别的挡掉。
  const byKind = new Map();
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    // 整行只有注释时代码部分是空的，直接跳过——省掉一次注定不成立的窗口匹配。
    if (!codeLines[i].trim()) continue;
    if (!trimmed || (seen && seen.has(trimmed))) continue;
    const win = codeLines.slice(i, i + 3).map((l) => l.trim()).join(" ");
    for (const [re, kind, ask] of RULES) {
      if (byKind.has(kind) || !re.test(win)) continue;
      // 行号要指准。窗口是从第 i 行起的三行拼平，命中的往往是里面第 2、第 3 行——
      // 报窗口起点就差一两行，而「指到了行号」正是这条机制区别于泛泛提醒的全部所在。
      // 先看窗口里哪一行**自己**就成立，成立就报那一行；三行都不单独成立（真正跨行的
      // 那种，比如模板字符串里的多行 SQL）才退回窗口起点。
      let at = -1;
      for (let k = 0; k < 3 && i + k < lines.length; k++) {
        if (re.test(codeLines[i + k].trim())) { at = i + k; break; }
      }
      // 单行就成立 → 报那一行的原文；真正跨行的（模板字符串里的多行 SQL）→ 报窗口起点和
      // 拼平后的三行。只报起点那一行会给出 `const q = \`` 这种什么都没说的"原文"。
      // 回显用**原始行**（带注释），让模型一眼认出自己写的是哪一句；判定用的才是代码行。
      byKind.set(kind, at >= 0
        ? { line: at + 1, kind, ask, text: lines[at].trim().slice(0, 100) }
        : { line: i + 1, kind, ask, text: win.slice(0, 100) });
    }
    if (byKind.size >= RULES.length) break;
  }
  return [...byKind.values()].sort((a, b) => a.line - b.line);
}

/*
 * 同一个函数里，null 既表示「没有」又表示「没查成」。
 *
 * 这是本仓库今天一天里连撞四处的那个病（agentLocate / agentHover / agentFormat /
 * agentDocumentSymbols）：`catch { return null }` 和正常路径上的 `return null` 写在
 * 同一个函数里，调用方拿到 null 分不出是哪一种，于是把「超时」说成「没有引用」，
 * 模型据此把有调用方的函数删了。
 *
 * 它是「写完之后维护不动」最典型的一种：写的时候两条路都回 null 看着很自然，出事时
 * 从调用方往回查，什么线索都没有——因为**信息在返回的那一刻就丢了**。
 *
 * 判据拿这个仓库自己的 27 万行真代码量过：命中 23 处，约每一万两千行一处，抽查 4 处
 * 全是真的（其中 _readOwnMcpUserConfig 那处的后果是「用户停用过的 MCP 服务会悄悄复活」）。
 * 同一轮里量过的另外三条判据都没过关，最接近的一条（会改变结果的操作被空 catch 吞掉）
 * 收紧到 7 处之后逐个看仍有七成误报（那些空 catch 大多在别处补偿过），所以没有采用。
 */
function _ambiguousFailureInWrite(text, maxItems = 3, path = "") {
  // 同样只看代码：注释里写一句「拿不到就 return null」不是一条返回路径。
  const lines = _splitCodeAndComments(String(text || ""), path).code;
  if (lines.length < 3) return [];
  // 函数起点 + 缩进。没有 AST，用缩进划块——这里只需要"大致是同一个函数"，
  // 划错一两行不会把结论变反：两种 return 必须都在同一块里才报。
  const START = /^(\s*)(?:(?:export\s+)?(?:async\s+)?function\s+[\w$]+|(?:pub\s+)?(?:async\s+)?fn\s+\w+|(?:async\s+)?[\w$]+\s*\([^)]*\)\s*\{|[\w$]+:\s*(?:async\s*)?\([^)]*\)\s*=>|(?:const|let|var)\s+[\w$]+\s*=\s*(?:async\s*)?\([^)]*\)\s*=>)/;
  const NULLISH = "(?:null|undefined|None)";
  const CATCH_INLINE = new RegExp("\\bcatch\\s*(?:\\([^)]*\\))?\\s*\\{\\s*return\\s+" + NULLISH + "\\s*;?\\s*\\}");
  const CATCH_OPEN = /\bcatch\s*(?:\([^)]*\))?\s*\{\s*$/;
  const RET_NULL = new RegExp("^\\s*return\\s+" + NULLISH + "\\s*;?\\s*$");
  const PLAIN_RET = new RegExp("^\\s*(?:if\\s*\\([^)]*\\)\\s*)?return\\s+" + NULLISH + "\\s*;?\\s*$");

  const out = [];
  let blockStart = -1, blockIndent = -1, blockName = "";
  let failLine = -1, plainLine = -1;
  const flush = () => {
    if (failLine > 0 && plainLine > 0 && out.length < maxItems) {
      out.push({ line: failLine, other: plainLine, name: blockName || "这个函数" });
    }
    failLine = -1; plainLine = -1;
  };
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const m = START.exec(raw);
    if (m && (blockIndent < 0 || raw.search(/\S/) <= blockIndent)) {
      flush();
      blockStart = i; blockIndent = m[1].length;
      blockName = (/(?:function|fn)\s+([\w$]+)|([\w$]+)\s*[:(=]/.exec(raw.trim()) || [])[1]
        || (/(?:function|fn)\s+([\w$]+)|([\w$]+)\s*[:(=]/.exec(raw.trim()) || [])[2] || "";
    }
    if (blockStart < 0) continue;
    // catch 里回 null —— 一行写完的，和 catch 换行写的，都要认。
    if (CATCH_INLINE.test(raw)) { if (failLine < 0) failLine = i + 1; continue; }
    if (CATCH_OPEN.test(raw)) {
      for (let k = 1; k <= 2 && i + k < lines.length; k++) {
        if (RET_NULL.test(lines[i + k])) { if (failLine < 0) failLine = i + k + 1; break; }
        if (lines[i + k].trim() && !/^\s*\/\//.test(lines[i + k])) break;
      }
      continue;
    }
    if (PLAIN_RET.test(raw) && plainLine < 0) plainLine = i + 1;
  }
  flush();
  return out;
}

/*
 * 「这里得说一句为什么」——写代码时当场点名，不是事后念叨。
 *
 * 用户说的是「AI 写代码不怎么写注释」。但「注释不够」这个笼统判据**量下来不成立**：
 * 拿这个仓库 21.2 万行真代码跑，「长函数零注释」每 437 行一处、「空 catch 无注释」每
 * 230 行一处、「魔法数字无注释」每 495 行一处——那种提醒每写一个文件就跳七八次，
 * 三天之后没人会再看它。大量零注释的短函数本来就是对的。
 *
 * 真正该有注释的是**读的人会问「为什么」的那几个点**。收紧成四条，逐条量过：
 *   · 会改变结果的操作被空 catch 吞掉、且一个字都没解释   每 15,158 行一处
 *   · 抑制类指令（@ts-ignore / #[allow] / eslint-disable）没写理由  每 26,527 行一处
 *   · 具名的上限/超时常量没解释（REQUEST_TIMEOUT_MS = 20000 —— 为什么是 20 秒？）
 *                                                          每 3,723 行一处
 *   · 文件超过 60 行、全文一条注释都没有   **146 个文件里 0 处** —— 这条就是这个仓库
 *     的标准本身，它只会在别人破坏这个标准时才响。
 * 被否掉的几条也记在这儿，免得下次有人再想一遍：长函数零注释 / 裸魔法数字 / 裸空 catch
 * / 正则无注释 / 导出函数无说明，密度 230–1829 行一处，全部不合格。
 *
 * 措辞一律要「为什么」而不是「是什么」——`// 把 i 加一` 这种注释比没有更糟。
 */
function _missingWhyInWrite(text, path = "", maxItems = 4) {
  const src = String(text || "");
  if (!src) return [];
  const { code, comments } = _splitCodeAndComments(src, path);
  const lines = src.split("\n");
  const out = [];
  // `self` = 这一行自己的注释算不算解释。抑制类指令本身就写在注释里
  // （// @ts-ignore），它自己当然不能算成"已经解释过了"。
  const hasNote = (i, self = true) => {
    if (self && comments[i] && comments[i].trim()) return true;
    // 往上找三行：空行跳过，紧邻的注释才算解释。
    for (let k = 1; k <= 3 && i - k >= 0; k++) {
      if (!lines[i - k].trim()) continue;
      if (comments[i - k] && comments[i - k].trim()) return true;
      if (k > 1) break;
    }
    return false;
  };

  // ── ① 整份文件一条注释都没有 ──────────────────────────────────────
  const codeLineCount = code.filter((l) => l.trim()).length;
  if (codeLineCount > 60 && !comments.some((c) => c && c.trim())) {
    out.push({
      line: 1, kind: "整份文件零注释",
      text: (lines.find((l) => l.trim()) || "").trim().slice(0, 80),
      ask: "在开头写两三行：这个文件解决什么问题、它在整体里是哪一环、有什么不显然的取舍。"
        + "**146 个真实文件里没有一个超过 60 行还一条注释都没有的**——这不是风格偏好，是接手的人能不能上手的分界线。",
    });
  }

  const EFFECTFUL = /\b(?:await\s+)?(?:[\w.]*\.)?(?:writeFile|writeTextFile|writeFileSync|writeTextFileIfUnchanged|appendFile|copyFile|rename|unlink|fetch|axios|got|execSync|execFile|spawnSync|commit|rollback)\s*\(/;
  const SUPPRESS = /@ts-ignore|@ts-expect-error|eslint-disable|#\[allow\(|# noqa|# type:\s*ignore|@SuppressWarnings/;
  const NAMED_LIMIT = /(?:const|let|static|pub const|final)\s+([A-Za-z_$][\w$]*(?:MS|_MS|Ms|LIMIT|CAP|MAX|TIMEOUT|RETRIES|RETRY|THRESHOLD|BUDGET|INTERVAL|DELAY|SIZE|BYTES)[\w$]*)\s*(?::[^=]+)?=\s*(\d{2,})/;

  for (let i = 0; i < code.length && out.length < maxItems; i++) {
    const c = code[i];
    // 纯注释行也要过一遍：抑制类指令（// @ts-ignore）住在注释里。
    if (!c.trim() && !SUPPRESS.test(comments[i] || "")) continue;

    // ── ② 会改变结果的操作被完全吞掉，而没有一个字说为什么可以吞 ──
    if (/\bcatch\s*(?:\([^)]*\))?\s*\{\s*\}/.test(c) || /\bexcept[^:]*:\s*pass\s*$/.test(c)) {
      const win = code.slice(Math.max(0, i - 3), i + 1).join("\n");
      if (EFFECTFUL.test(win) && !hasNote(i)) {
        out.push({
          line: i + 1, kind: "静默吞掉一次真实操作", text: lines[i].trim().slice(0, 80),
          ask: "这里被吞掉的是一次真实的写入/请求/进程调用——失败了就意味着**该发生的事没发生**，"
            + "而调用方什么都不知道。要么把失败往上报，要么用一句话写清楚为什么这里可以吞"
            + "（是幂等的？别处补偿过？失败无所谓？）。半年后出事时，这一句就是唯一的线索。",
        });
        continue;
      }
    }

    // ── ③ 抑制类指令没写理由 ──
    //
    // 这一条要**同时看注释和代码**：JS/TS 的 `// @ts-ignore`、`// eslint-disable-next-line`
    // 本身就是注释（底座会把它剥进 comments），Rust 的 `#[allow(...)]` 才是代码。
    // 只看代码的话这条规则对 JS 永远不触发——第一版就是这么写的，测试当场抓到。
    const directive = SUPPRESS.exec(comments[i] || "") || SUPPRESS.exec(c);
    if (directive) {
      // 同一行写了理由也算解释：`eslint-disable-next-line no-x -- 上游类型定义漏了`。
      const rest = String(comments[i] || c).slice(directive.index + directive[0].length);
      const sameLineReason = /[^\s)\],;:]{4,}/.test(rest.replace(/^[\s)\],;:-]+/, ""));
      if (!sameLineReason && !hasNote(i, false)) {
        out.push({
          line: i + 1, kind: "抑制了检查却没说为什么", text: lines[i].trim().slice(0, 80),
          ask: "抑制一条检查等于把一个已知风险按下不表。写清楚为什么这里是安全的、以及什么条件下"
            + "这条抑制该被撤掉——不写的话，下一个人只能在「删了会不会炸」和「留着会不会掩盖 bug」之间猜。",
        });
      }
      continue;
    }

    // ── ④ 具名的上限/超时常量没解释 ──
    const m = NAMED_LIMIT.exec(c);
    if (m && !hasNote(i)) {
      out.push({
        line: i + 1, kind: "这个数字是怎么定的", text: `${m[1]} = ${m[2]}`,
        ask: `${m[1]} 为什么是 ${m[2]}？是实测出来的、上游的硬限制、还是先拍一个？`
          + "不写的话它就变成一个没人敢动的数——出问题时不知道能不能调，不出问题时也不知道还有没有余量。",
      });
    }
  }
  return out;
}

// 把上面那些做成挂在**这一次写入**上的一段话。没有命中就一个字都不发。
export function _sinkRiskAdvice(call) {
  // 名字是历史的：最早这里只有「危险汇聚点」。现在它是**这一次写入的质量提醒**的唯一
  // 出口——多一个出口就多一处要手工保持同步的接线，而这个仓库已经因为"手工维护的清单"
  // 栽过好几次。新增的检测器挂进来，不要另开调用点。
  const body = String(call?.content ?? call?.new_string ?? "");
  const path = String(call?.path || "").split("/").slice(-2).join("/");
  let out = "";

  const risks = _sinkRisksInWrite(body, null, call?.path || "");
  if (risks.length) {
    out += "\n\n⚠ 这次写入碰到了危险汇聚点，趁还在这一步先堵上（下面每条都指到了行号和原文，不是泛泛提醒）：\n"
      + risks.map((r) => `· ${path}:${r.line} ${r.kind} — \`${r.text}\`\n  ${r.ask}`).join("\n");
  }

  const why = _missingWhyInWrite(body, call?.path || "");
  if (why.length) {
    out += "\n\n✍ 这几处半年后没人看得懂为什么，趁现在补一句（要写**为什么**，不是写代码在干嘛）：\n"
      + why.map((w) => `· ${path}:${w.line} ${w.kind} — \`${w.text}\`\n  ${w.ask}`).join("\n");
  }

  const amb = _ambiguousFailureInWrite(body, 3, call?.path || "");
  if (amb.length) {
    out += "\n\n⚠ 这次写的代码里，失败和「没有」回的是同一个值——调用方分不出来：\n"
      + amb.map((a) => `· ${path}:${a.line} \`${a.name}\` 在 catch 里回 null，第 ${a.other} 行的正常路径也回 null。`
        + `\n  调用方拿到 null 只能猜。它会猜"没有"——于是把"超时"说成"查不到"、把"读失败"说成"是空的"，然后照着这个假结论往下做。`
        + `\n  给失败一个**不同的形状**：抛出去，或者回 { ok: false, reason } 这类能带上原因的值；"没有"才回 null。`
        + `\n  这不是洁癖：出事时从调用方往回查，什么线索都没有——信息在 return 的那一刻就丢了。`).join("\n");
  }
  return out;
}

export function _stubDeliveryFindings(run, maxItems = 8) {
  const cp = run?.checkpoint;
  if (!cp || typeof cp.forEach !== "function") return [];
  const out = [];
  const MARKS = [
    [/\b(?:TODO|FIXME|XXX)\b/i, "TODO 占位"],
    [/not\s*implemented|NotImplementedError|未实现|尚未实现|待实现/i, "声明未实现"],
    [/lorem\s+ipsum/i, "lorem 假文案"],
    // 这条原来**没有 i 标志、且写死小驼峰**：mockData 命中，而 MOCK_DATA / mock_data /
    // fake_users / sample_response / mockUser（单数）全漏——Python、Go、Rust 的蛇形命名是
    // 主流写法，等于这条判据在那三门语言上结构性失效。放宽后误报率没变（本仓库 207,792 行
    // 实测：旧 0.10/万行、新 0.10/万行，那两处命中还是正则字面量自己和提示词里的一句话）；
    // 负向自检 6/6：sampleRate / mockingbird / dataList / resultSet / userList / sampled 静默。
    [/\b(?:mock|fake|dummy|stub|sample)[_-]?(?:data|list|items?|users?|products?|response|result|payload|records?|orders?)\b|假数据|模拟数据|示例数据/i, "写死的假数据"],
    [/\bplaceholder\b.*=.*['"`]|占位实现/i, "占位实现"],
  ];
  // 上面那五条只抓**模型自己老实标注**的占位（TODO、not implemented、命名里带 mock）。
  // 实测召回 3/12：真正的「写着写着变成 MVP」是**看着像写完了的空壳**，一条都不落进上面那张表。
  //
  // 下面这批按**结构**判，且要跨行——真实代码是
  //     function verifyToken(t) {
  //       return true;
  //     }
  // 三行，单行正则一条都抓不到。所以对每个起始行取一个 3 行窗口拼平再判，正则一律锚在
  // 窗口首行行首（不锚的话窗口会把不相干的下一行拼进来，实测空函数体那条误报从 0 涨到 1.59/万行）。
  //
  // 每一条都在本仓库自己的 207,656 行真实代码（JS + Rust）上量过误报，**全部为 0**；
  // 正向自检 6/6。量出来被砍掉的候选：空 catch（83.4/万行——`try{...}catch{}` 在本仓库是
  // 通用惯用法）、硬编码 `return {ok:true}`（1.24/万行）、写死 localhost（0.8/万行，抽样全是
  // 正则误匹配到散文里）。宁可漏，不可让每一轮都跳一堆假警报——那会让这本账整个失去可信度。
  const STRUCT = [
    // 鉴权/权限函数整个身子就是 return true —— 它同时是 MVP 空壳和一个真漏洞
    [/^(?:pub\s+)?(?:async\s+)?(?:fn|function)\s+(?:auth\w*|authorize\w*|authenticate\w*|verif\w*|permit\w*|permission\w*|canAccess|hasAccess|hasRole|hasPermission|isAdmin|isOwner|isAllowed|checkToken|checkAuth|validateToken|validateUser)\s*\([^)]*\)[^{]{0,40}\{\s*(?:return\s+)?true\s*;?\s*\}/i, "鉴权恒真（空壳且是漏洞）"],
    [/^(?:const|let|var)?\s*(?:auth\w*|authorize\w*|authenticate\w*|verif\w*|permit\w*|permission\w*|canAccess|hasAccess|hasRole|hasPermission|isAdmin|isOwner|isAllowed|checkToken|checkAuth|validateToken|validateUser)\s*[:=]\s*(?:async\s*)?\([^)]*\)\s*=>\s*\{?\s*(?:return\s+)?true\s*;?\s*\}?\s*;?$/i, "鉴权恒真（空壳且是漏洞）"],
    // 真逻辑被关掉
    [/^\s*if\s*\(\s*(?:false|0)\s*\)/, "真逻辑被关掉"],
    // 有名字、有参数，身子是空的
    [/^(?:pub\s+)?(?:async\s+)?(?:fn|function)\s+\w{3,}\s*\([^)]*\)[^{]{0,40}\{\s*\}\s*$/, "空函数体"],
    // 取数函数直接回空
    [/^(?:pub\s+)?(?:async\s+)?(?:fn|function)\s+(?:get|list|fetch|load|query|find|search)\w*\s*\([^)]*\)[^{]{0,40}\{\s*return\s*(?:\[\s*\]|null|None|\{\s*\})\s*;?\s*\}\s*$/i, "取数函数回空"],
    // 编出来的地址。**只在真会被调用/被配成端点的上下文里**才算——纯字符串数据和测试断言里
    // 出现 example.com 是正当的（收紧前它在本仓库误报 5.4/万行，全是网关的 HTML 解析夹具）。
    [/\b(?:fetch|axios(?:\.\w+)?|request|got|superagent|urlopen|requests\.\w+|http\.(?:Get|Post)|reqwest::\w+)\s*\(\s*["'`]https?:\/\/(?:[\w.-]*\.)?(?:example\.(?:com|org|net)|your-\w+|api\.example)\b/i, "编造的地址"],
    [/\b(?:baseUrl|baseURL|BASE_URL|apiUrl|API_URL|endpoint|ENDPOINT|apiHost|webhook\w*)\s*[:=]\s*["'`]https?:\/\/(?:[\w.-]*\.)?(?:example\.(?:com|org|net)|your-\w+|api\.example)\b/i, "编造的地址"],
  ];
  // 上限是**跨文件共享**的，而 cp.forEach 的顺序就是写入顺序。实测：一个文件里新增 12 条
  // TODO 就把 8 个名额全占光，这一轮真正新写的那个文件**一条都排不上**（返回 8 条、来自新
  // 文件的 0 条）——而后者恰恰是「这次交付」最该被点名的那一个。
  // 改成两轮分配：先每个动过的文件各出 1 条保证露面，再按顺序把剩下的名额填满。
  const perFile = [];
  cp.forEach((snap, absPath) => {
    const cur = String(snap?.current || "");
    if (!cur || !_CODE_FILE_RE.test(String(absPath))) return;
    // 改前原文按行建集合：只有新增的行才算这次交付引入的。
    const before = new Set(String(snap?.content || "").split("\n").map((l) => l.trim()));
    const lines = cur.split("\n");
    /*
     * 两组规则看的东西不一样，不能混：
     *   · MARKS（TODO / 未实现 / lorem / 假数据 / 占位实现）是**自我招供**，它本来就写在
     *     注释里——必须看原始行。
     *   · STRUCT（鉴权恒真、空函数体、取数回空、编造的地址）判的是**代码形状**，注释里
     *     贴一段"老写法，已废弃"不该被算成这次写出来的东西。
     * 用户的原话：不能光看注释，要代码一起看。这就是那条界线落在代码里的样子。
     */
    const codeLines = _splitCodeAndComments(cur, String(absPath)).code;
    const mine = [];
    for (let i = 0; i < lines.length && mine.length < maxItems; i++) {
      const raw = lines[i];
      const trimmed = raw.trim();
      if (!trimmed || before.has(trimmed)) continue;
      let matched = false;
      for (const [re, kind] of MARKS) {
        if (!re.test(trimmed)) continue;
        mine.push({ path: String(absPath).split("/").slice(-2).join("/"), line: i + 1, kind, text: trimmed.slice(0, 90) });
        matched = true;
        break;
      }
      if (matched) continue;
      // 3 行窗口：起始行必须是新增的（上面已判），后两行只用来补全结构。**只取代码部分**。
      if (!codeLines[i].trim()) continue;
      const win = codeLines.slice(i, i + 3).map((l) => l.trim()).join(" ");
      for (const [re, kind] of STRUCT) {
        if (!re.test(win)) continue;
        // 3 行窗口会让**同一处**连报两三次（窗口从它前面一两行就开始命中了）。
        // 实测：一行 fetch("https://api.example.com/…") 报了第 14 行和第 16 行两条，
        // 内容是同一句。同类且行距 <3 的当成同一处，只留先命中的那一条。
        // （_sinkRisksInWrite 那边是按类别整份去重；这边不能那么做——同一份文件里
        //   真的可能有多处独立的同类占位，那些每一处都该点名。）
        // 行号要指准：窗口起点常常不是真正命中的那一行（实测一行 fetch(...) 报成了它
        // 前面两行的行号）。先看窗口里哪一行**自己**就成立；三行都不单独成立（真跨行的
        // 那种，比如模板串里的多行函数签名）才退回窗口起点并给拼平后的三行。
        let at = -1;
        for (let k = 0; k < 3 && i + k < lines.length; k++) {
          if (re.test(codeLines[i + k].trim())) { at = i + k; break; }
        }
        const hitLine = at >= 0 ? at + 1 : i + 1;
        const hitText = at >= 0 ? lines[at].trim().slice(0, 90) : win.slice(0, 90);
        const dup = mine.some((f) => f.kind === kind && hitLine - f.line < 3);
        if (!dup) mine.push({ path: String(absPath).split("/").slice(-2).join("/"), line: hitLine, kind, text: hitText });
        break;
      }
    }
    if (mine.length) perFile.push(mine);
  });
  // 第一轮：每个动过的文件各出一条。第二轮：剩下的名额按文件顺序填满。
  for (const m of perFile) { if (out.length < maxItems) out.push(m[0]); }
  for (const m of perFile) {
    for (let k = 1; k < m.length && out.length < maxItems; k++) out.push(m[k]);
  }
  return out;
}

// ── 硬编码字面量：_stubDeliveryFindings 的姊妹扫描器 ─────────────────────────
//
// 同一套纪律：checkpoint 基线相减、只看**新增**行、只报有原文佐证的字面命中、
// 只陈述不拦截。规则族是硬编码一类：写死的端口/本机地址、写死的 URL、写死的绝对
// 路径、密钥形字面量。「不要散落硬编码」在提示词里写了三遍而用户照样抱怨——所以
// 这里给的是判据不是劝告：file:line + 那一行原文，命中 .env 里已有的 key 名时把
// 配置项名字一并点出（envKey 由 _envKeysByRoot 提供，只有名字、值从不缓存）。
/*
 * 旧注释：紧邻的代码改了，这句话还写着旧的。
 *
 * 用户的原话：「有的还会用旧注释让 IDE 发现不了问题。」在这个仓库里做了一轮专门取样，
 * 复核站得住 20 条。最典型的一条：常量从 128 抬到 256，四行之外那句「仍远低于
 * 128 / 512 KiB 的总窗口」没跟着改——而同一文件另一处写的是 256，两处自相矛盾。
 * 照旧的那句去理解，会回头排查一道早已不存在的闸。
 *
 * 判据必须是**局部**的。第一版按「这个记号从整份文件的代码里消失了」判，拿那次真实提交
 * （7690ef5，128→256 就是它干的）一跑：0 命中。原因很直白——`128` 在一个 5 MB 的文件里
 * 别处当然还有。所以改成看**注释周围那十几行**：这句话旁边的代码里，它提到的那个值/名字
 * 变了，而它一个字没动。
 *
 * 四个必要条件，缺一不可（每一条都是标定时逼出来的）：
 *   ① 形态够像代码：下划线开头、全大写常量名、或 ≥2 位数字。普通 CamelCase 英文词不认——
 *      第一版认，于是 `Notification` / `Response` / `Items` 这些散文词全成了误报。
 *   ② 在改前那段局部代码里是**被声明或被赋值**出来的，不是路过一次的字面量。
 *   ③ 改后同一段局部代码里**没有**它了。
 *   ④ 这条注释本身一个字没动（改前也是原样）；讲历史的（「原来 / 曾经 / was」）豁免——
 *      这个仓库大量注释是事故复盘，记录旧值正是它们的用途。
 *
 * 标定：最近 60 个提交里的 134 次真实代码改动，误报 0；同时抓得住 7690ef5 那次真实漂移。
 */
export function _staleCommentFindings(run, maxItems = 4) {
  const cp = run?.checkpoint;
  if (!cp || typeof cp.forEach !== "function") return [];
  const HISTORY = /原来|以前|曾经|旧的|旧值|改成|之前是|历史|was\s|used\s+to|previously|formerly|no\s+longer|不再/i;
  const NEAR = 12;
  const TOK = /(?:\b\d[\d_]{1,}\b)|(?:\b[A-Za-z_$][\w$]{3,}\b)/g;
  const CODEY = (t) => /^_/.test(t) || /^[A-Z][A-Z0-9_]{3,}$/.test(t) || /^\d[\d_]+$/.test(t);
  const tokensOf = (text) => {
    const set = new Set();
    for (const m of String(text).matchAll(TOK)) set.add(m[0]);
    return set;
  };
  const out = [];
  cp.forEach((snap, absPath) => {
    if (out.length >= maxItems) return;
    const before = String(snap?.content || "");
    const after = String(snap?.current || "");
    if (!before || !after || before === after) return;
    if (!_CODE_FILE_RE.test(String(absPath))) return;

    const b = _splitCodeAndComments(before, String(absPath));
    const a = _splitCodeAndComments(after, String(absPath));
    const afterCode = a.code.join("\n");

    // 改前的注释原文 → 它在改前的行号（用来把局部窗口对齐到改动之前的位置）。
    const beforeAt = new Map();
    for (let j = 0; j < b.comments.length; j++) {
      const t = (b.comments[j] || "").trim();
      if (t && !beforeAt.has(t)) beforeAt.set(t, j);
    }
    // 这一轮真正改动过的行（按行做集合差就够，只是用来划「附近」）。
    const beforeCodeLines = new Set(b.code.map((l) => l.trim()).filter(Boolean));
    const touched = [];
    for (let i = 0; i < a.code.length; i++) {
      const t = a.code[i].trim();
      if (t && !beforeCodeLines.has(t)) touched.push(i);
    }
    if (!touched.length) return;
    const touchedSet = new Set(touched);
    const nearTouched = (i) => {
      for (let k = -NEAR; k <= NEAR; k++) if (touchedSet.has(i + k)) return true;
      return false;
    };

    for (let i = 0; i < a.comments.length && out.length < maxItems; i++) {
      const note = (a.comments[i] || "").trim();
      if (!note || note.length < 6) continue;
      if (HISTORY.test(note)) continue;
      const j = beforeAt.get(note);
      if (j === undefined) continue;         // 这一轮新写/改过的注释不算
      if (!nearTouched(i)) continue;

      const beforeWin = b.code.slice(Math.max(0, j - NEAR), j + NEAR + 1).join("\n");
      const afterWin = a.code.slice(Math.max(0, i - NEAR), i + NEAR + 1).join("\n");
      const afterTokens = tokensOf(afterWin);
      // 局部窗口里被声明/被赋值出来的记号才算数。
      const localDecl = new Set();
      for (const m of beforeWin.matchAll(/(?:const|let|var|static|class|function|fn|struct|enum|type)\s+([A-Za-z_$][\w$]*)/g)) localDecl.add(m[1]);
      for (const m of beforeWin.matchAll(/[A-Za-z_$][\w$]*\s*[:=]\s*(\d[\d_]+)\b/g)) localDecl.add(m[1]);

      /*
       * 第二种形态：注释**点名**了一个下划线打头的本地符号（用反引号包着，或者写成
       * `_foo()`），而它在改后的这份文件里已经不存在了。
       *
       * 这一类在人工取样里占 5/20（`_stopSessionRun` / `_warmupWorkspaceAgent` /
       * `_predictComposerNext` / `_thinkingRequestParams` 全是这样）。要求「被点名」
       * 而不是「被提到」——散文里顺口提一个名字和用反引号点名它，是两回事；
       * 跨文件引用的误报也基本被这条挡住（那种多半是叙述，不是点名）。
       */
      const named = new Set();
      for (const m of note.matchAll(/`(_[A-Za-z][\w$]{3,})`|\b(_[A-Za-z][\w$]{3,})\(\)/g)) named.add(m[1] || m[2]);
      const fileTokens = tokensOf(afterCode);
      const beforeFileTokens = tokensOf(b.code.join("\n"));
      for (const t of named) {
        // 必须是**这一轮**弄没的：改前这份文件里有、改后没了。
        // 不加这条的话，测试/脚本文件里引用主文件的符号会全变成误报——标定时
        // 542 次真实改动里那 3 条误报（_KNOWN_TOOLS / _mcpServerApprovalMode / _live）
        // 全是这种跨文件引用，它们本来就不在这份文件里。
        if (fileTokens.has(t) || !beforeFileTokens.has(t)) continue;
        out.push({
          path: String(absPath).split("/").slice(-2).join("/"),
          line: i + 1, token: t, text: note.slice(0, 90),
        });
        break;
      }
      if (out.length >= maxItems) break;
      if (out.length && out[out.length - 1].line === i + 1) continue;

      for (const t of tokensOf(note)) {
        if (!CODEY(t) || afterTokens.has(t) || !localDecl.has(t)) continue;
        out.push({
          path: String(absPath).split("/").slice(-2).join("/"),
          line: i + 1, token: t, text: note.slice(0, 90),
        });
        break;
      }
    }
  });
  return out;
}

export function _hardcodedDeliveryFindings(run, envKeys, maxItems = 6) {
  const cp = run?.checkpoint;
  if (!cp || typeof cp.forEach !== "function") return [];
  const keys = Array.isArray(envKeys) ? envKeys : [];
  const _envFor = (cat) => {
    const want = cat === "port" ? /PORT/i
      : cat === "url" ? /(URL|HOST|ENDPOINT|API)/i
      : cat === "secret" ? /(KEY|SECRET|TOKEN|PASS)/i : null;
    return (want && keys.find((k) => want.test(k))) || "";
  };
  // 「是字面量还是表达式」——和 _redactSecrets 里那套被实测校准过的判据同一逻辑
  // （那份刻意定义在函数内部，不能引用；这里保持同样的四条规则）。
  const _looksLikeCodeValue = (v) => {
    if (!v) return false;
    if (/[(){}[\]<>\\]/.test(v)) return true;
    if (/^[+\-*/!~&|=]/.test(v)) return true;
    if (/^[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)+$/.test(v)) return true;
    if (/^[A-Za-z_$]+$/.test(v)) return true;
    return false;
  };
  const out = [];
  cp.forEach((snap, absPath) => {
    if (out.length >= maxItems) return;
    const cur = String(snap?.current || "");
    if (!cur || !_CODE_FILE_RE.test(String(absPath))) return;
    const before = new Set(String(snap?.content || "").split("\n").map((l) => l.trim()));
    const lines = cur.split("\n");
    for (let i = 0; i < lines.length && out.length < maxItems; i++) {
      const trimmed = lines[i].trim();
      if (!trimmed || before.has(trimmed)) continue;
      if (/^(?:\/\/|#|\*|\/\*|<!--)/.test(trimmed)) continue; // 注释行不算这次交付的行为
      const short = trimmed.slice(0, 90);
      const path = String(absPath).split("/").slice(-2).join("/");
      const sm = /(?:api[_-]?key|apikey|secret|token|passw(?:or)?d|credential)\s*[:=]\s*["'`]([^"'`]{8,})["'`]/i.exec(trimmed);
      if (sm && !_looksLikeCodeValue(sm[1])) {
        out.push({ path, line: i + 1, kind: "密钥形字面量写进了源码", text: short, envKey: _envFor("secret") });
        continue;
      }
      if (/(?:localhost|127\.0\.0\.1|0\.0\.0\.0):\d{2,5}\b/.test(trimmed) || /\bport\b["']?\s*[:=]\s*["']?\d{2,5}\b/i.test(trimmed)) {
        out.push({ path, line: i + 1, kind: "写死的端口/本机地址", text: short, envKey: _envFor("port") });
        continue;
      }
      if (/https?:\/\//.test(trimmed) && !/xmlns|w3\.org|schema\.org|localhost|127\.0\.0\.1/.test(trimmed)) {
        out.push({ path, line: i + 1, kind: "写死的 URL", text: short, envKey: _envFor("url") });
        continue;
      }
      if (/["'`(=\s](?:\/Users\/|\/home\/|[A-Za-z]:\\\\)/.test(trimmed)) {
        out.push({ path, line: i + 1, kind: "写死的绝对路径", text: short, envKey: "" });
      }
    }
  });
  return out;
}

// ── 这一轮**触及**（新增声明行、或声明行文本变了）的对外导出符号 ────────────────
//
// 给有界引用查询供靶子：每轮最多 maxItems 个、每 run 每个符号只查一次（seen 记在
// run._refQueriedSymbols 上）。只挑**还在文件里**的声明——被删掉的那类走
// _removedDeclarationsUnchecked（位置都没了，引用查询无处下手）；全新文件里的全新
// 符号天然没有调用方，也不在这份名单里（重名那一侧由符号索引查重覆盖）。
export function _touchedExportedDecls(run, absPaths, maxItems = 3) {
  const cp = run?.checkpoint;
  if (!cp || typeof cp.get !== "function") return [];
  const DECL = /^export\s+(?:async\s+)?(?:function|class)\s+([A-Za-z_$][\w$]*)|^export\s+(?:const|let|var)\s+([A-Za-z_$][\w$]*)|^pub\s+(?:async\s+)?fn\s+([A-Za-z_][\w]*)|^def\s+([A-Za-z_][\w]*)/;
  const seen = run._refQueriedSymbols || (run._refQueriedSymbols = new Set());
  const out = [];
  for (const abs of Array.isArray(absPaths) ? absPaths : []) {
    if (out.length >= maxItems) break;
    const snap = cp.get(abs);
    const cur = snap?.current;
    if (typeof cur !== "string" || !snap?.existed || !_CODE_FILE_RE.test(String(abs))) continue;
    const beforeDecl = new Map();
    for (const line of String(snap.content || "").split("\n")) {
      const m = DECL.exec(line);
      if (m) beforeDecl.set(m[1] || m[2] || m[3] || m[4], line.trim());
    }
    const lines = cur.split("\n");
    for (let i = 0; i < lines.length && out.length < maxItems; i++) {
      const m = DECL.exec(lines[i]);
      if (!m) continue;
      const name = m[1] || m[2] || m[3] || m[4];
      const prev = beforeDecl.get(name);
      // 全新符号不查引用（还没有调用方可言）；声明行原样没动的也不查。
      if (prev === undefined || prev === lines[i].trim()) continue;
      const key = `${name}@${abs}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push({ name, abs, line: i + 1, character: Math.max(0, lines[i].indexOf(name)) });
    }
  }
  return out;
}
