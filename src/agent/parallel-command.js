/**
 * 「这条 shell 命令能不能和同批的其它只读调用并发跑」——在权限判据之外**再减一层**。
 *
 * # 为什么不能直接用权限那份判据
 *
 * `_looksLikeReadOnlyCommand` 回答的是「要不要弹审批框」。它的白名单里有
 * `git branch`，因为「列分支」确实只读。但 `git branch feature-x` 是**建分支**，
 * 而 `attrib +r f` 是**改文件属性**——这两条在权限那一侧无伤大雅，在并发这一侧是另一回事：
 *
 * 同一个仓库里，`git` 的**工具类型**那一支早就踩过完全一样的坑（见 main.js 的
 * `_isReadOnlyParallel`）：切分支把整棵工作树换掉，同批并发的读拿到的是**另一个分支**
 * 的文件内容——而且读成功了、没有任何报错，模型据此往下推理。这是最难查的一类。
 *
 * 走 shell 的那条路当时没人管，因为 `cmd` 整个不参与并行。现在它参与了，
 * 同一个洞就跟着开了，所以这里补上。
 *
 * # 为什么是「减法」不是另写一份白名单
 *
 * 两份手写名单必然漂——这个仓库为此付过很多次账（gh 的只读 op、git 的只读 op、
 * 并行类型表与子体只读类型表，每一处的注释都在说同一件事）。所以结构解析
 * （`;` `<` `>` 反引号 `$()` `&`、`&&` 链、管道段）一律**复用**权限那一份，
 * 这里只表达「额外还要排掉什么」。
 */

/** 这一段是不是「权限上算只读、但并发起来会改变别人看到的世界」。 */
export function parallelUnsafeCommand(command) {
  const raw = String(command || "");
  // 按 && 和 | 切开逐段看：权限判据已经保证了整条命令的结构是干净的
  // （没有 `;` `<` `>` 反引号 `$()`），所以这里只需要按这两个分隔符切。
  const segments = raw.split(/\s*(?:&&|\|)\s*/).map((s) => s.trim()).filter(Boolean);
  return segments.some(segmentUnsafe);
}

function segmentUnsafe(segment) {
  // `git branch` 只有在**不带任何非选项参数**时才是「列分支」。带了名字就是建/改/删分支，
  // 会动 refs、`-f`/`-m`/`-d` 更是直接改。判据和 `git` 工具类型那一支对齐：
  // 不带名字＝列分支＝只读；带名字＝动东西。
  const gitBranch = /^git\s+branch\b(.*)$/i.exec(segment);
  if (gitBranch) {
    const rest = String(gitBranch[1] || "").trim();
    if (!rest) return false;                       // 光 `git branch` = 列出来
    // 只剩纯粹的列举类选项才算只读；出现任何位置参数（分支名）或写类选项都不算。
    return !rest.split(/\s+/).every((tok) =>
      /^-{1,2}(?:a|r|v|vv|all|list|remote|show-current|merged|no-merged|contains|no-contains|sort|format|color|no-color|column|no-column|i|ignore-case)$/i.test(tok));
  }
  // Windows 的 `attrib`：不带 +/- 是查看，带了就是改属性。
  if (/^attrib\b/i.test(segment)) return /\s[+-][a-z]/i.test(segment);
  // `set` 在 cmd 里带 `=` 就是赋值——只影响那个进程，不动磁盘，所以不排。
  return false;
}
