/**
 * Single source of truth for what a tool IS, so the harness stops re-deciding it.
 *
 * # The problem this replaces
 *
 * Every property the harness needs to know about a tool before running it — does it change
 * the workspace, does it need approval, may a read-only mode run it, does a worker's scope
 * apply, do repository hooks fire — was written as a literal list at the point of use. The
 * mutation family alone (`["write","edit","multiedit","delete","move","mkdir","copy","format"]`)
 * appeared **eleven times** in `main.js`, each an independent copy.
 *
 * That is the growth tax. Adding one tool meant finding eleven places and remembering the
 * right subset for each. Missing one is silent: a tool left out of the read-only-mode list is
 * quietly executable in Explorer/Plan/Reviewer, and nothing fails until a user notices.
 *
 * # The model
 *
 * A tool declares capability FLAGS once. Every named set the harness used to hard-code is a
 * derived query over those flags, so the sets can never disagree with each other again.
 *
 * Only tools whose policy differs from the default are listed. The default — read-only, no
 * approval, no hooks, runnable in every mode — is correct for the large majority of the
 * catalogue (every `*_search`, every lookup, every inspection tool), so listing them would be
 * noise that rots. A tool type that is not registered gets the default, which is also what
 * makes adding a read-only tool a zero-edit operation.
 *
 * # Adding a tool
 *
 *   defineTool("my_tool", { mutatesWorkspace: true, needsApproval: true });
 *
 * That is the whole change. No list hunting.
 *
 * This module is pure data + pure functions with no DOM, no I/O and no imports, so its tests
 * `import` it directly instead of scraping source text out of `main.js`.
 */

/** Policy for a tool type nobody has registered: safe, inert, universally available. */
export const DEFAULT_POLICY = Object.freeze({
  /**
   * The tool's result carries a trustworthy `mutated` boolean, so `mutated === false` is
   * evidence it really changed nothing (rather than absence of information).
   *
   * Deliberately NOT true for `cmd`/`termtask`: a shell command may well change the
   * workspace, but it does not report whether it did, and treating a missing flag as
   * "no-op" would mark every command as neutral.
   */
  mutatesWorkspace: false,
  /** One of the structured file operations (the family that shares read-before-edit, path
   *  binding, conflict handling). A generator that emits files mutates the workspace but is
   *  NOT a file operation, which is exactly the distinction the old lists kept blurring. */
  fileMutation: false,
  /** Writes file CONTENT specifically (write/edit/multi_edit/format) — the subset that
   *  participates in diagnostics and diff review. */
  fileEdit: false,
  /** Needs user approval when "approve before changes" is on. */
  needsApproval: false,
  /** Repository `pre_tool_use` / `post_tool_use` hooks fire around it. */
  hooked: false,
  /** Refused in the read-only modes (Explorer / Plan / Reviewer). */
  readOnlyModeBlocked: false,
  /** Which argument carries the path a worker sub-agent's scope is checked against.
   *  Empty string = this tool is not scope-checked. */
  scopeField: "",
  /** A [BLOCKED]/[CONFLICT] result is a recoverable policy stop rather than a hard failure,
   *  so it must not count toward the three-strike tool lockout. */
  recoverableBlock: false,
});

/** file-mutation defaults, shared by the eight structured file operations. */
const FILE_OP = {
  mutatesWorkspace: true,
  fileMutation: true,
  needsApproval: true,
  hooked: true,
  readOnlyModeBlocked: true,
  recoverableBlock: true,
};
/** the four that write content (and therefore reach diagnostics + diff review). */
const FILE_CONTENT_OP = { ...FILE_OP, fileEdit: true, scopeField: "path" };
/** a generator that lands assets in the workspace: mutating, but not a file operation. */
const GENERATOR = { mutatesWorkspace: true, needsApproval: true };
/** an execution tool: side effects the harness cannot verify, but no `mutated` report. */
const EXEC = { needsApproval: true, hooked: true };

const REGISTRY = new Map();

/**
 * Register or override a tool's policy. Unspecified fields fall back to the default, so a
 * declaration states only what is unusual about the tool.
 */
export function defineTool(type, policy = {}) {
  const name = String(type || "").trim();
  if (!name) throw new Error("defineTool requires a tool type");
  const unknown = Object.keys(policy).filter((k) => !(k in DEFAULT_POLICY));
  // A typo'd flag would silently do nothing — exactly the class of bug this module exists to
  // remove — so refuse it at declaration time instead of at 3am.
  if (unknown.length) throw new Error(`unknown tool policy field(s) for "${name}": ${unknown.join(", ")}`);
  REGISTRY.set(name, Object.freeze({ ...DEFAULT_POLICY, ...policy }));
  return REGISTRY.get(name);
}

/** Every declaration whose policy differs from the default. */
function seed() {
  // ── structured file operations ────────────────────────────────────────────
  for (const t of ["write", "edit", "multiedit"]) defineTool(t, FILE_CONTENT_OP);
  // `format` writes content like the other three, but repository hooks deliberately do NOT
  // fire for it: formatting is a mechanical rewrite of code the hooks already saw, and firing
  // a lint hook on every auto-format was noise.
  defineTool("format", { ...FILE_CONTENT_OP, hooked: false });
  defineTool("mkdir", { ...FILE_OP, scopeField: "path" });
  defineTool("copy", { ...FILE_OP, scopeField: "path" });
  // delete/move are refused outright for workers rather than scope-checked (a parallel child
  // deleting or relocating files is a conflict source no scope can make safe), so they carry
  // no scopeField — the executor's own worker guard rejects them earlier.
  defineTool("delete", FILE_OP);
  defineTool("move", FILE_OP);

  // ── command execution ─────────────────────────────────────────────────────
  defineTool("cmd", { ...EXEC, readOnlyModeBlocked: true });
  // 缺口已补（原来这里留着一段注释说"故意先不改，behaviour fix 该单独一个提交"——
  // 这就是那个提交）。`termtask` 就是 run_in_terminal：命令串由模型给出、原样执行，
  // 和 `cmd` 是同一类能力，只是多了个长驻终端。它不在只读封禁名单里，意味着
  // Explorer / Plan / Reviewer 这三个**声称只读**的模式可以起任意 shell——
  // 而 `cmd` 在它们那儿是被挡住的。同一件事换个工具名就绕过去了，那道门等于虚设。
  defineTool("termtask", { ...EXEC, readOnlyModeBlocked: true });

  // ── other side-effecting tools ────────────────────────────────────────────
  // 只读模式里按**单次调用**判：服务自己声明了 readOnlyHint 的放行，没声明的照挡。
  // 每一次调用仍然过 needsApproval 那道门，所以放行的也不是无人看管。
  defineTool("mcp", { needsApproval: true, readOnlyModeBlocked: (call) => !call?.mcpReadOnly });
  // 用户自己声明接进来的 HTTP 能力。一律要审批，和 MCP 同级——声明可能来自 clone 来的
  // 仓库，而它能往任意 http(s) 地址发请求。只读判定同样**逐次**看这一次调用：声明里写的
  // 方法是 GET/HEAD 就当只读（那是用户自己写下的事实，不是我们猜的），于是 Plan /
  // Explorer 这些只读模式里，「查一下我们内网的工单」这类事照样能做。
  defineTool("userhttp", { needsApproval: true, readOnlyModeBlocked: (call) => !call?.userReadOnly });
  // 用户接进来的本地知识库。检索永远是只读的，所以只读模式一律放行——「先查资料再动手」
  // 恰恰是 Plan / Explorer 最需要的事。仍然要审批：它读的是用户机器上的一个目录。
  defineTool("userfolder", { needsApproval: true });
  // git worktree：add 在磁盘上建目录并新建分支，remove 连未提交的改动一起删。它一直
  // **完全没登记**，于是拿的是默认值（不审批、只读模式不挡）——Plan / Explorer / Reviewer
  // 这三个声称只读的模式里，模型可以建目录、删目录。这是漏登记，不是有意放行。
  //
  // 只读判定按**这一次调用**来：list 是纯读取，只读模式里该能用（"先看看有哪些候选"正是
  // Plan 要做的事）；add / remove 动磁盘，挡住。
  // 不设 needsApproval：它只在 <root>/.michael/worktrees/ 下面动，是 IDE 自己的目录，
  // 每建一个候选都弹一次窗会把 best-of-N 这件事变得没法用。真正的数据风险（重名时
  // --force 销毁上一个候选）已经在 git.rs 的 git_worktree_add 里从根上去掉了。
  defineTool("worktree", {
    mutatesWorkspace: true,
    readOnlyModeBlocked: (call) => String(call?.action || "list") !== "list",
  });
  defineTool("uiclick", { needsApproval: true, readOnlyModeBlocked: true });
  defineTool("automation", { needsApproval: true });
  defineTool("db", { needsApproval: true });
  defineTool("download", { mutatesWorkspace: true, needsApproval: true });
  // create_project 一直没有声明：它会在用户主目录下真的建出 ~/MrDayOne/<name>，
  // 并把左侧文件树整个切到那个新目录——只读模式里也能干，"改动前审批"也不弹。
  // 用户原来打开的项目就这么被顶掉，而模式标签一直写着「只读」。
  defineTool("createproject", { mutatesWorkspace: true, needsApproval: true, readOnlyModeBlocked: true });
  // capture_start：mode='system' / system_proxy=true 会改掉**操作系统级**代理设置，
  // 整台机器的流量（浏览器、邮件、其他 App）一起被切到本地 mitmproxy 上，接着还要
  // 用户 sudo 装一张根证书。这不该是一句「我顺手开了抓包」就发生的事。
  defineTool("capture_start", { needsApproval: true });

  // ── 有真实外部副作用、却一直没登记的四个 ──────────────────────────────────
  //
  // 这四个是审计（2026-08-17）挖出来的：它们和上面 uiclick / automation 是同一类东西
  // （越过工作区去动真实世界），但从来没进过这张表。没进表 = needsApproval 取默认值
  // false = 「改动前审批」开着也一次框都不弹，只读模式也不拦。而防漂移守卫只看得见
  // **已登记且 mutatesWorkspace** 的类型，对完全没登记的工具恒绿——所以漏了很久没人发现。
  //
  // browser：跑在一个常驻登录态的自动化 profile 上。action:"eval" 是任意 JS，
  //   "cookies"/"storage" 直接读走会话，"upload" 收的是**本机绝对路径**（模型可以填
  //   ~/.ssh/id_rsa），"autofill"+提交能替用户按下不可撤销的按钮。同一时刻写一个文件
  //   要弹框，这些不弹——这不是权衡，是漏登记。只读模式下同样要拦：读 cookie 不是"只读"。
  defineTool("browser", {
    needsApproval: true,
    // 纯导航/截图/读页面是观察，不该在只读模式里被一刀切；动到会话、文件、执行和提交的
    // 那几个 action 才是副作用。审批门则一律要过——观察别人的登录态浏览器也该让人知情。
    readOnlyModeBlocked: (call) => !["navigate", "screenshot", "read", "text", "back", "forward", "close"]
      .includes(String(call?.action || "")),
  });
  // docker_compose_up：直接起一整套容器（`docker compose up -d`），占端口、挂卷、
  //   长期后台运行，停不停得掉不归本轮管。这是执行，不是读。
  defineTool("docker_compose_up", { ...EXEC, readOnlyModeBlocked: true });
  // capture_replay：可以指定任意 method / url / body 直接发出去，而且**不要求真有一条
  //   抓包记录**——等于绕开 http_request 那道审批门的一条完整旁路。同门同待遇。
  defineTool("capture_replay", { needsApproval: true, readOnlyModeBlocked: true });
  // system：开 App、切前台窗口、触发任意 App 的菜单项。main.js 另一处早已把它判成
  //   「有外部副作用」，只有这张表不知道。
  defineTool("system", { needsApproval: true, readOnlyModeBlocked: true });

  // ── generators that land assets in the workspace ──────────────────────────
  for (const t of [
    "game_scaffold", "web_scaffold", "download_asset", "genimage", "generate_3d",
    "generate_sound", "generate_music", "generate_voice", "auto_rig", "generate_motion",
    "generate_texture",
  ]) defineTool(t, GENERATOR);
}
seed();

/** Resolved policy for a tool type. Never throws; unknown types get the safe default. */
export function toolPolicy(type) {
  return REGISTRY.get(String(type || "")) || DEFAULT_POLICY;
}

/** Every registered type whose policy satisfies `predicate`. Internal: the derived
 *  views below are the public surface. */
function typesWhere(predicate) {
  const out = new Set();
  for (const [type, policy] of REGISTRY) if (predicate(policy, type)) out.add(type);
  return out;
}

// ── The named sets the harness used to hard-code, now derived ───────────────
// Exported as live getters rather than frozen constants so a `defineTool` call at startup
// (a plugin, a future MCP-backed tool) is reflected everywhere instead of only in the
// call sites that happened to be evaluated after it.
export const workspaceMutatingTypes = () => typesWhere((p) => p.mutatesWorkspace);
export const fileMutationTypes = () => typesWhere((p) => p.fileMutation);
export const fileEditTypes = () => typesWhere((p) => p.fileEdit);
export const approvalTypes = () => typesWhere((p) => p.needsApproval);
export const hookedTypes = () => typesWhere((p) => p.hooked);
export const readOnlyBlockedTypes = () => typesWhere((p) => p.readOnlyModeBlocked);

// ── Predicates: what call sites should actually use ─────────────────────────
export const mutatesWorkspace = (type) => toolPolicy(type).mutatesWorkspace;
export const isFileMutation = (type) => toolPolicy(type).fileMutation;
export const isFileEdit = (type) => toolPolicy(type).fileEdit;
export const needsApproval = (type) => toolPolicy(type).needsApproval;
/*
 * 只读模式（Plan / Explorer / Reviewer）里这个调用要不要挡。
 *
 * `call` 是可选的第二个参数，为的是让 MCP 能按**这一次调用**判断，而不是整个类型
 * 一刀切。一刀切的代价是：用户装的 MCP 服务在只读模式里全都用不了——查文档、读数据库
 * 结构这类纯读取的事，恰恰是 Plan 模式最需要的，而它们和"写文件"被归成了同一类。
 *
 * 判据用调用上带的 mcpReadOnly（来自服务自己声明的 readOnlyHint）。声明缺失时按
 * **可能有副作用**处理——MCP 规范里这个提示是可选的，多数服务不写，宁可挡住也不能
 * 在只读模式里替用户改了东西。
 */
export const blockedInReadOnlyMode = (type, call) => {
  const blocked = toolPolicy(type).readOnlyModeBlocked;
  if (typeof blocked === "function") return blocked(call);
  return blocked;
};

/**
 * Which argument of `call` a worker sub-agent's scope is checked against, or "" when the tool
 * is not scope-checked. Returning the FIELD rather than a boolean keeps the executor from
 * re-deriving "which property holds the path for this tool", which is the same duplication in
 * a different costume.
 */
export const workerScopeField = (type) => toolPolicy(type).scopeField;

/** The concrete path a worker's scope applies to, or "" when the tool is unscoped. */
export function workerScopeTarget(call) {
  if (!call) return "";
  const field = workerScopeField(call.type);
  if (!field) return "";
  return String(call[field] || call.dest || call.to || "");
}

/** Snapshot of every declaration — for diagnostics and for the drift test. */
export function allPolicies() {
  return Object.fromEntries([...REGISTRY].map(([type, policy]) => [type, { ...policy }]));
}
