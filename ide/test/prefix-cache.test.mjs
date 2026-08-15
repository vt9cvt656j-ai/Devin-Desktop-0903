// The semantic profile decides which blocks the gateway puts in its system prefix, and the prefix
// sits at byte 0 of every request. Change it mid-session and the provider's cache misses on the
// prefix AND on the whole conversation behind it — the expensive part. This project already paid
// for that lesson once, with a date block that carried minutes and a measured 2% hit rate.
//
// Real functions are pulled out of src/main.js with acorn, following model-resume.test.mjs: a
// reimplementation here would only prove the copy is self-consistent.
import assert from "node:assert/strict";
import test from "node:test";
import fs from "node:fs";
import * as acorn from "acorn";

const SRC = fs.readFileSync("src/main.js", "utf8");
const ast = acorn.parse(SRC, { ecmaVersion: "latest", sourceType: "module" });
function grab(name) {
  for (const n of ast.body) {
    if (n.type === "FunctionDeclaration" && n.id?.name === name) return SRC.slice(n.start, n.end);
  }
  throw new Error("missing " + name);
}
const load = (name, need) => new Function(need.map(grab).join("\n") + `\nreturn ${name};`)();

const stable = load("_sessionStableSemanticProfile", ["_sessionStableSemanticProfile"]);
const profileOf = load("_ideSemanticProfile", ["_ideSemanticProfile"]);

test("a turn that classifies narrower does not drop the session's blocks", () => {
  const session = {};

  // Turn 1, the real classifier shape for "fix the login page and run it".
  const t1 = stable(session, profileOf({ ui: true, workspaceAction: "modify", applies: true }));
  assert.equal(t1, "2.5:engineering,design,design_implementation,design_verification");

  // Turn 5 of the same session: "still broken, check why the window won't show". Nothing in that
  // sentence reads as UI work, so the per-turn classifier drops every design flag. Before this
  // was sticky, that rewrote the prefix — 34,973 bytes to 26,276 — and cost a full cache miss on
  // the entire conversation. It has to stay put: the user is still fixing the same login page.
  const t5 = stable(session, profileOf({ applies: true }));
  assert.equal(t5, t1, "a narrower turn rewrote the prefix mid-session");
});

test("a session that genuinely widens pays once and then settles", () => {
  const session = {};
  const t1 = stable(session, profileOf({ ui: true, workspaceAction: "modify", applies: true }));

  // The user now asks for a commit — git is real new capability, so the prefix legitimately grows.
  const t2 = stable(session, profileOf({ applies: true, git: true }));
  assert.notEqual(t2, t1, "a genuinely new capability should reach the gateway");
  assert.ok(t2.startsWith(t1), "widening must append, not reorder — reordering is a needless miss");
  assert.ok(t2.endsWith(",git"));

  // ...and every turn after that is byte-identical, which is the whole point.
  assert.equal(stable(session, profileOf({ applies: true })), t2);
  assert.equal(stable(session, profileOf({ ui: true, workspaceAction: "modify", applies: true })), t2);
  assert.equal(stable(session, profileOf({ applies: true, git: true })), t2);
});

test("stickiness is per session, so a new session starts focused", () => {
  const a = {};
  stable(a, profileOf({ ui: true, workspaceAction: "modify", applies: true, fullWebsite: true }));

  // Not a global accumulator: the full flag set assembles an 84KB prefix against 26KB here, and
  // blocks the model does not need are more instructions competing with the ones it does.
  const b = {};
  assert.equal(stable(b, profileOf({ applies: true })), "2.5:engineering");
});

test("no session (background/one-shot request) passes through unchanged", () => {
  const header = profileOf({ applies: true, git: true });
  assert.equal(stable(null, header), header);
  assert.equal(stable(undefined, ""), "");
});

test("the session's flags survive a restart and are dropped on a rewind", () => {
  // Session persistence is an explicit whitelist, so a field nobody lists silently does not
  // survive — and a resumed session would then pay a full prefix miss on its first turn for
  // nothing. Two serializers and two rehydrators, matching how _intentState is handled.
  assert.equal((SRC.match(/semanticFlags: Array\.isArray\(/g) || []).length, 2,
    "both session serializers must write the flags");
  assert.equal((SRC.match(/if \(Array\.isArray\(sData\.semanticFlags\)\)/g) || []).length, 2,
    "both rehydrators must read them back");

  // Rewinding deletes a message and everything after it, so whatever capabilities that stretch
  // of the conversation needed stop counting. It is also the one free moment to re-derive:
  // truncation has already invalidated the cache, so nothing is lost by starting over.
  assert.match(SRC, /sess\._intentState = null; sess\._lastRunState = null; sess\._semanticProfileFlags = null;/,
    "a rewind must drop the accumulated flags with the conversation it discarded");
});

test("every profile write goes through the sticky merge", () => {
  // Three places assign config.ideSemanticProfile: the turn itself, a late intent verdict, and
  // steering mid-run. The last two also narrow, so a raw assignment anywhere reopens the bug.
  const assignments = SRC.match(/config\.ideSemanticProfile\s*=\s*[^;]+;/g) || [];
  assert.ok(assignments.length >= 3, "expected the known profile writes to still exist");
  for (const line of assignments) {
    assert.match(line, /_sessionStableSemanticProfile\(/, `unmerged profile write: ${line}`);
  }
});

test("a brand-new session waits, briefly and once, before sending an empty profile", () => {
  // 画像是同步算出来的，而全新会话没有 _intentState 也没有缓存命中，本地证据函数又只返回
  // URL 列表——所以第一轮的画像是空的：决定整个做法的那一轮拿不到 agent_engineering。
  // 而且粘性画像让这件事在缓存上也要付账：第一轮空、第二轮有 flag，等于每个会话必然在第二轮
  // 把整条对话的前缀作废一次——正是粘性本身要防的那个失败。
  assert.match(SRC, /const _FIRST_TURN_INTENT_WAIT_MS = \d+;/,
    "the first-turn wait must be one named bound, not a literal buried in the turn");
  const bound = Number(/const _FIRST_TURN_INTENT_WAIT_MS = (\d+);/.exec(SRC)[1]);
  assert.ok(bound > 0 && bound <= 2000,
    `the wait is paid on a real turn — ${bound}ms is not a bounded cost`);

  const guard = /if \(_turnIntentState && !\(sess\._semanticProfileFlags \|\| \[\]\)\.length\) \{/;
  assert.match(SRC, guard,
    "the wait must be gated on the session having no flags yet — otherwise every turn pays it");

  // 等待必须发生在画像组装之前，否则等了也白等。
  const waitAt = SRC.search(guard);
  const assignAt = SRC.indexOf("config.ideSemanticProfile = _sessionStableSemanticProfile(sess,");
  assert.ok(waitAt > 0 && assignAt > waitAt,
    "the wait must precede the profile assignment it exists to inform");

  // 超时也要照常发车：裁决迟到由循环边界的 _applyLateIntentIfLanded 兜底，不能把一轮卡死。
  assert.match(SRC, /_waitTimer = setTimeout\(resolve, _FIRST_TURN_INTENT_WAIT_MS\)/,
    "the race must have a timeout arm so a slow classifier cannot stall the turn");
  assert.match(SRC, /_applyLateIntentIfLanded\(run, config, task, session, body, _live, messages\);/,
    "the late-adopt path must remain as the fallback for a verdict that misses the window");
});

// ---------------------------------------------------------------------------
// File tree — the explorer rows.
// ---------------------------------------------------------------------------
const APP_CSS = fs.readFileSync("src/styles/app.css", "utf8");
const REFINE_CSS = fs.readFileSync("src/styles/refine.css", "utf8");

test("a workspace root does not wear the file selection", () => {
  // The root row carries .is-active whenever it is the current workspace, so it rendered the
  // same filled highlight as the open file directly beneath it — two solid rows touching, which
  // is what made the explorer read as one clump instead of a tree.
  assert.match(APP_CSS, /\.workspace-root__row\.is-active::before \{ background: transparent; \}/,
    "the root must not paint the active-file highlight");
  assert.match(APP_CSS, /\.workspace-root__row\.is-active \.name \{ color: var\(--text\); \}/,
    "which root is current has to remain readable some other way");
});

test("the row highlight spans the panel instead of sitting in a pill", () => {
  // Indentation comes from nested .children wrappers, so a background on .row itself starts at
  // the indent. The highlight is a pseudo-element stretched past the left edge instead.
  assert.match(APP_CSS, /\.row::before \{[^}]*left: -1000px;/s, "the highlight must extend past the indent");
  assert.match(APP_CSS, /\.row:hover::before \{ background: var\(--hover\); \}/);
  assert.match(APP_CSS, /\.row\.is-active::before \{ background: var\(--sel\); \}/);
  // A background on .row would be drawn at the indented width and defeat the whole thing.
  const rowBlock = APP_CSS.slice(APP_CSS.indexOf("\n.row {"), APP_CSS.indexOf("\n.row::before"));
  assert.doesNotMatch(rowBlock, /background:/, ".row itself must stay transparent");
  assert.doesNotMatch(rowBlock, /border-radius:/, "no pill");
  assert.match(rowBlock, /height: 22px;/, "VS Code's row height");
});

test("the tree stylesheet has no selectors that match nothing", () => {
  // refine.css carried a .tree-item / .tree__row / #tree [role="treeitem"] block whose comment
  // claimed to have fixed the row spacing. None of those three exist in this DOM, so it never
  // applied — the comment described a fix that was not happening, which is worse than no comment.
  // Compare rules only; the note left in its place naturally mentions the dead names.
  const rules = REFINE_CSS.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const dead of [".tree-item", ".tree__row", 'role="treeitem"']) {
    assert.ok(!rules.includes(dead), `${dead} matches no element and must not be styled`);
  }

  assert.match(SRC, /document\.querySelector\(`\.row\[data-path=/,
    "revealInTree must query the class the rows actually have — `.tree-item` matched nothing, so " +
    "every caller, including the agent marking each file it touches, silently did nothing");
  assert.match(APP_CSS, /\.row\.flash::before \{ animation: tree-flash/,
    "and .flash needs a style behind it, which it never had either");
});

test("archive handling is reachable from the UI, not just implemented in the backend", () => {
  // The reader existed for a while before anything could call it. A capability nobody can invoke
  // is indistinguishable from one that does not exist.
  assert.match(SRC, /extractArchive: \(path, dest, budget\) => core\.invoke\("extract_archive"/,
    "the native seam must expose extraction");
  assert.match(SRC, /readArchiveEntry: \(path, entry, maxBytes\) =>\s*core\.invoke\("read_archive_entry"/,
    "and single-entry reads");
  // Both stubs must exist too, or the web build throws an obscure undefined-is-not-a-function
  // instead of saying the desktop app is required.
  assert.match(SRC, /extractArchive: async \(\) => \{ throw new Error\("解压需要桌面版应用。"\); \}/);

  // An archive gets its own full panel, routed like the database and hex editors — not a section
  // appended below a generic file report of MIME type, read-only flag and fact cards.
  assert.match(SRC, /function _isArchiveInspection\(info\)/, "archives need their own classifier");
  assert.match(SRC, /if \(info && _isArchiveInspection\(info\)\) \{\s*_renderArchiveInspection/,
    "and must be routed to their own renderer before the generic path");
  assert.match(SRC, /_fileInspectionPreviewEl\.className = "file-inspector file-inspector--archive"/);
  assert.match(SRC, /data-archive-browser-host/, "the panel needs a host to mount into");
  assert.match(SRC, /mountArchiveBrowser\(host, \{/, "and something has to mount it");
  // The generic renderer must not also emit an archive section, or it comes back as a second copy.
  assert.equal((SRC.match(/_inspectionArchiveHtml/g) || []).length, 0,
    "the old inline archive section must be gone, not merely unused");

  // Archive content is untrusted input; it is rendered as text, never as markup.
  const preview = SRC.slice(SRC.indexOf("function _showArchiveEntryPreview"), SRC.indexOf("function _inspectionArchiveHtml"));
  assert.match(preview, /querySelector\("pre"\)\.textContent = /,
    "entry content must go in as textContent — innerHTML here is a script-injection path straight " +
    "out of a downloaded archive");
  const template = preview.slice(preview.indexOf("wrap.innerHTML = `"), preview.indexOf("// textContent"));
  assert.doesNotMatch(template, /content[?.]*\.text/,
    "the entry's own bytes must never be interpolated into markup — only its name and size, both escaped");
});

test("the inspector header cannot be crushed by its own buttons", () => {
  // A fourth action button squeezed the title column to a few pixels, and CJK — which may break
  // between any two characters — rendered one character per line. min-width:0 was already there
  // and is what permitted it: the floor has to come from the basis, with the header wrapping.
  const header = APP_CSS.slice(APP_CSS.indexOf(".file-inspector__header {"), APP_CSS.indexOf(".file-inspector__icon {"));
  assert.match(header, /flex-wrap: wrap;/, "the header must wrap rather than compress the title");
  const heading = APP_CSS.slice(APP_CSS.indexOf(".file-inspector__heading {"), APP_CSS.indexOf(".file-inspector__eyebrow {"));
  assert.match(heading, /flex: 1 1 260px;/, "and the title needs a real basis, not flex-basis auto");
});

test("the archive window fills its frame instead of floating in it", () => {
  const ARCHIVE_JSX = fs.readFileSync("src/ui/archive-browser.jsx", "utf8");
  // The base shell is a floating card: min(1180px) wide, 24px margin, 24px radius, a shadow. Left
  // inherited, the archive window became a rounded rectangle hovering over the inspector's tinted
  // gradient — corners and all, which is what read as pasted-on.
  const shell = APP_CSS.slice(APP_CSS.indexOf(".file-inspector__shell--archive {"), APP_CSS.indexOf(".file-inspector__shell--archive > "));
  for (const decl of ["margin: 0;", "border-radius: 0;", "box-shadow: none;", "border: 0;"]) {
    assert.ok(shell.includes(decl), `the archive shell must reset ${decl}`);
  }

  // One surface, named once. bg-background resolves to --bg; the shell paints --panel-solid. They
  // are different greys, and the join between them is visible.
  assert.match(ARCHIVE_JSX, /bg-\[var\(--panel-solid\)\]/, "the island must paint the shell's surface");
  assert.doesNotMatch(ARCHIVE_JSX, /bg-background/, "and must not paint a second one beside it");

  // Real icons, not emoji: a zip should look like the zip in the tree beside it.
  assert.match(ARCHIVE_JSX, /iconFor\?\.\(/, "rows take their icon from the tree's icon set");
  assert.match(SRC, /iconFor: \(name, isDir\) => \(isDir \? folderIconUrl/, "which main.js supplies");
});

test("CSV files open in the table window, with a way back to the editor", () => {
  const TABLE_JSX = fs.readFileSync("src/ui/table-view.jsx", "utf8");
  assert.match(SRC, /const TABLE_EXTS = new Set\(\["csv", "tsv", "tab"\]\)/, "csv/tsv route to the table");
  assert.match(SRC, /readTableFile: \(path, maxRows\) => core\.invoke\("read_table_file"/, "native seam");
  assert.match(SRC, /readTableFile: async \(\) => \{ throw new Error/, "and a stub for the web build");

  // A CSV is still a text file people hand-edit. Routing it to a read-only grid without an exit
  // would remove editing, so the escape hatch is part of the feature, not a nicety.
  assert.match(SRC, /_openAsTextPaths\.add\(path\)/, "opening as text must be possible");
  assert.match(SRC, /isTableFile\(name\) && !_openAsTextPaths\.has\(path\)/,
    "and must stick, or the next open bounces back to the grid");
  assert.match(TABLE_JSX, /onOpenAsText/, "the window needs the control for it");

  // A parse failure must fall back to text rather than leaving an empty panel.
  assert.match(SRC, /已按文本打开/, "a file that will not parse still has to open");

  // Same surface as its shell, the lesson from the archive panel's seam.
  assert.match(TABLE_JSX, /bg-\[var\(--panel-solid\)\]/);
  assert.doesNotMatch(TABLE_JSX, /bg-background/);

  // The overlay has to be hidden wherever its siblings are, or it covers the editor.
  assert.ok((SRC.match(/hideTablePreview\(\);/g) || []).length >= 3,
    "hidden alongside hidePdfPreview in every place that switches views");
});

test("a long filename does not take over the tab strip or the title bar", () => {
  // One real build artefact — 庄子视频处理系统V5.0-TS长视频版-Windows-x64-最新版-20260722.zip — made
  // its tab ~640px wide, pushing every other tab off the strip so the bar showed one file and a
  // fragment of another. .tab is white-space:nowrap and .tab .label had no rule at all, so the
  // tab simply grew to whatever the name needed.
  const tab = APP_CSS.slice(APP_CSS.indexOf("\n.tab {"), APP_CSS.indexOf(".tab:hover"));
  assert.match(tab, /max-width: 220px;/, "a tab needs a ceiling");
  // And a floor, which is the subtler half. .tabs is a flex row, so tabs shrink by default and
  // min-width:auto is the only thing that stops them. Setting min-width:0 to allow the ellipsis
  // removed that floor and every label collapsed to zero width — a strip of bare icons, worse
  // than the overflowing tab it replaced. Not shrinking is what lets .tabs scroll instead.
  assert.match(tab, /flex: 0 0 auto;/, "tabs must not shrink; the strip scrolls instead");
  assert.doesNotMatch(tab.slice(0, tab.indexOf(".tab .label")), /min-width: 0;/,
    "min-width:0 on the tab itself collapses every label");
  // No ellipsis anywhere on the tab label — the overflow fades instead.
  assert.doesNotMatch(APP_CSS.slice(APP_CSS.indexOf("\n.tab .label {")).slice(0, 400),
    /text-overflow: ellipsis;/, "the tab label must not ellipsise");
  assert.match(tab, /\.tab > \.ic,\s*\.tab > \.x \{ flex: none; \}/,
    "only the label may shrink — a close button that moves as you switch files is worse");
  assert.match(SRC, /tab\.title = f\.name;/, "a truncated label needs the full name on hover");

  // The title bar shortens from the middle: end-truncation drops the extension, which is the part
  // that says what the file is.
  const fn = SRC.match(/function _ellipsizeMiddle[\s\S]*?\n}/)[0];
  const ellipsize = new Function(`${fn}; return _ellipsizeMiddle;`)();
  const long = "庄子视频处理系统V5.0-TS长视频版-Windows-x64-最新版-20260722.zip";
  const short = ellipsize(long);
  assert.ok(short.length < long.length, "a long name must actually be shortened");
  assert.ok(short.endsWith(".zip"), "and must keep the extension");
  assert.ok(short.includes("…"));
  assert.equal(ellipsize("main.js"), "main.js", "short names are left alone");
});

test("a long tab name fades out rather than being cut with an ellipsis", () => {
  // Three attempts cut characters to fit a width: end-ellipsis ate the extension, middle-ellipsis
  // made the name unreadable, and a separate extension span left a gap between two boxes. All
  // three replaced real characters with dots. The label now holds the whole name and the overflow
  // is faded, so everything visible is true.
  const fill = SRC.slice(SRC.indexOf("function _fillFileLabel"), SRC.indexOf("function _markClippedTabs"));
  assert.match(fill, /el\.textContent = String\(name \|\| ""\);/, "the whole name goes in");
  assert.doesNotMatch(fill, /…|slice\(/, "nothing may be removed from the string");
  assert.match(fill, /el\.title = String\(name \|\| ""\);/);
  assert.doesNotMatch(fill, /\.innerHTML\s*=/);

  // No ellipsis on the tab label, and a fade in its place.
  // Slice forward from the rule: ".row .name {" also appears earlier in the file, so searching
  // from the start finds the wrong end and the slice comes out backwards.
  const labelStart = APP_CSS.indexOf("\n.tab .label {");
  const tabLabel = APP_CSS.slice(labelStart, APP_CSS.indexOf(".row .name {", labelStart));
  assert.doesNotMatch(tabLabel, /text-overflow: ellipsis;/, "the tab label must not ellipsise");
  assert.match(tabLabel, /mask-image: linear-gradient\(to right/, "it fades instead");
  assert.match(tabLabel, /-webkit-mask-image:/, "WKWebView needs the prefixed property");

  // The fade is conditional: applied to every label it would soften names that fit.
  assert.match(APP_CSS, /\.tab \.label\.is-clipped \{/);
  assert.match(SRC, /label\.classList\.toggle\("is-clipped", label\.scrollWidth > label\.clientWidth \+ 1\)/,
    "overflow cannot be detected in CSS, so it is measured");
  assert.match(SRC, /_markClippedTabs\(\);/, "and measured after the strip is built");

  // The tree keeps whole names — it is resizable, so its width is the user's to set.
  assert.match(SRC, /row\.querySelector\("\.name"\)\.textContent = item\.name;/,
    "the tree renders the full name, unshortened");
});

test("@ is a router: model, files, github, gitlab", () => {
  // It used to open straight onto a fuzzy list of every file, so the only thing you could point
  // at was a path.
  assert.match(SRC, /const _AT_CATEGORIES = \[/, "the categories must exist");
  for (const id of ["model", "files", "github", "gitlab"]) {
    assert.match(SRC, new RegExp(`id: "${id}"`), `${id} must be a category`);
  }

  // Typing a path still goes straight to files — a router that slows the common case is worse
  // than no router.
  assert.match(SRC, /query\.includes\("\/"\) \|\| query\.includes\("\."\)/,
    "a path-shaped query must skip the category step");

  // Escape backs out one level rather than closing everything.
  assert.match(SRC, /if \(_atMode\) \{ _atMode = null; _renderAtMenu\(\); \} else _hideAtMenu\(\);/);

  // Rows carry their own action, so the renderer and the picker cannot disagree about row 3.
  assert.match(SRC, /function _pickAt\(i\) \{[\s\S]*?row\.onPick\(\);/,
    "picking must call the row's own handler");
});

test("a @model: chip actually routes the turn to that model", () => {
  // The headline risk with this feature is a menu entry that inserts a token nothing reads —
  // the turn would run on the default model and look like it worked.
  const fn = SRC.match(/function _requestedModelFor[\s\S]*?\n}/)[0];
  const requested = new Function(
    `const _modelCatalogEntry = (id) => (id === "claude-opus-5" ? { id } : null);
     const _customModelById = () => null;
     ${fn}; return _requestedModelFor;`,
  )();
  assert.equal(requested("@model:claude-opus-5 重构这个模块"), "claude-opus-5");
  assert.equal(requested("重构这个模块"), "", "no token, no routing");
  // A model that is not in the catalogue must not silently fall back to the default.
  assert.equal(requested("@model:not-a-real-model do it"), "",
    "an unknown model must not quietly become the default");

  const strip = new Function(`${SRC.match(/function _withoutModelToken[\s\S]*?\n}/)[0]}; return _withoutModelToken;`)();
  assert.equal(strip("@model:claude-opus-5 重构这个模块"), "重构这个模块",
    "the routing token is an address, not part of the task");

  // Routed models go through the same gate as picked ones — membership, custom endpoints, the lot.
  assert.match(SRC, /const routed = await _readyAiConfig\(\{ \.\.\.loadConfig\(\), model: _routedModel \}\)/);
  assert.match(SRC, /async function _readyAiConfig\(overrideConfig = null\)/);
  assert.match(SRC, /const _rawCfg = overrideConfig \|\| loadConfig\(\);/);
});

test("an unlinked integration offers to connect rather than showing nothing", () => {
  // An empty list and "you have not connected this" look identical and mean opposite things.
  //
  // 「未连接」现在有两个来源都要排除掉才成立：本地粘贴的 PAT，以及用户在网页后台走 OAuth
  // 连好的那种（令牌在服务端，本地没有副本）。这个菜单一度只查前者，于是后台显示「已连接」
  // 的账号在 IDE 里仍被要求去粘贴令牌。
  assert.match(SRC, /if \(!token && _atRepoState\[kind\] !== "connected"\) \{\s*return \[\{\s*kind: "connect"/,
    "no token AND no gateway link must produce a connect row");
  assert.match(SRC, /`\$\{_michaelBase\(\)\}\/api\/integrations\/\$\{kind\}\/repos`/,
    "the gateway-side link must be consulted, not just localStorage");
  // 网关还没回话时不能先说「未连接」—— 那一闪读起来像是连接掉了。
  assert.match(SRC, /_atRepoState\[kind\] === "unknown" && !token/,
    "an unresolved state must render as loading, not as disconnected");
  assert.match(SRC, /localStorage\.setItem\(`michael-ide\.\$\{kind\}-token`/,
    "the token is stored locally, like the Figma one");
  // The chip serialiser and the caret arithmetic must share one definition of a chip's text.
  assert.equal((SRC.match(/_chipText\(c\)/g) || []).length, 2,
    "both the serialiser and the caret maths must call the same helper");
});

test("the @ categories use English names and icons that exist", () => {
  const INDEX = fs.readFileSync("index.html", "utf8");
  // i-sparkles was referenced and never existed, so Model rendered with no icon at all — a
  // missing <symbol> fails silently, which is why every icon name here is asserted present.
  for (const name of ["Model", "Files", "GitHub", "GitLab"]) {
    assert.match(SRC, new RegExp(`label: "${name}"`), `${name} must be the English label`);
  }
  assert.doesNotMatch(SRC, /label: "模型"|label: "文件"/, "no Chinese category labels remain");
  // As a value, not as a word: the comment explaining this bug necessarily names the icon, and
  // grepping prose is how a test ends up asserting against documentation. Twice today.
  assert.doesNotMatch(SRC, /"i-sparkles"/, "the icon that never existed must not be referenced");

  const icons = [...SRC.matchAll(/icon: "(i-[a-z-]+)"/g)].map((m) => m[1]);
  assert.ok(icons.length >= 4);
  for (const icon of new Set(icons)) {
    assert.ok(INDEX.includes(`id="${icon}"`), `${icon} has no <symbol> — it would render blank`);
  }
  // Real marks rather than a generic branch glyph for both hosts.
  assert.ok(INDEX.includes('id="i-brand-github"') && INDEX.includes('id="i-brand-gitlab"'));
  assert.match(SRC, /icon: "i-brand-github"/);
  assert.match(SRC, /icon: "i-brand-gitlab"/);
});

test("a folder in the @ menu opens instead of only inserting", () => {
  // Selecting a folder used to insert it and stop, so referencing one file deep in a tree meant
  // typing the path out.
  assert.match(SRC, /onPick: isDir\s*\? \(\) => \{ _atDir = clean; _renderAtMenu\(\); \}/,
    "a folder must open");
  assert.match(SRC, /name: `Use \$\{here\}\/`/, "inserting the folder itself stays one row away");
  assert.match(SRC, /name: "\.\.",/, "and there must be a way back up");
  // Inside a folder the list is its children, not a workspace-wide match that would undo the walk.
  assert.match(SRC, /if \(!f\.startsWith\(prefix\)\) continue;/);
  assert.match(SRC, /_atDir = "";  \/\/ and at the workspace root/, "reopening @ starts at the root");
});

test("the model mark depicts a model, not a sparkle", () => {
  const INDEX = fs.readFileSync("index.html", "utf8");
  const at = INDEX.indexOf('<symbol id="i-sparkle"');
  const mark = INDEX.slice(at, INDEX.indexOf("</symbol>", at));

  // Two rounds were spent resizing a sparkle, which is the wrong axis: a sparkle means "AI magic"
  // and says nothing about a model. The mark is a network converging on a core.
  assert.equal((mark.match(/<circle/g) || []).length, 4, "three peripheral nodes and a hub");
  assert.match(mark, /<path fill="none" stroke="currentColor"/, "edges connect them");

  // The hub must outweigh the peripheral nodes — the weight belongs where the computation is,
  // and that hierarchy is what the eye reads at 16px before it resolves any single node.
  const radii = [...mark.matchAll(/r="([\d.]+)"/g)].map((m) => parseFloat(m[1]));
  const hub = Math.max(...radii);
  const leaf = Math.min(...radii);
  assert.ok(hub > leaf * 1.3, `the hub (${hub}) must dominate the nodes (${leaf})`);

  // Round caps, or thin edges break up when the glyph is scaled down and antialiased.
  assert.match(mark, /stroke-linecap="round"/);
  // No opacity: it goes muddy on the tinted selected row and on dark.
  assert.doesNotMatch(mark, /opacity=/);
  // The spark is what stops a bare graph reading as "network" or "share".
  assert.match(mark, /M17\.9 3\.1c/, "the generative accent must remain");
});

test("Files can point @ at a folder the workspace does not have yet", () => {
  const INDEX = fs.readFileSync("index.html", "utf8");
  assert.match(SRC, /name: "Add folder…"/, "the row must exist");
  assert.match(SRC, /icon: "i-new-folder"/, "with its own icon");
  assert.ok(INDEX.includes('id="i-new-folder"'), "and that icon must be a real symbol");

  // It must go through _addWorkspaceRoot, which is what registers the folder with the backend
  // sandbox. A path this menu knew about but the backend had not granted would list fine and then
  // fail on the first read with "access denied" — worse than not offering the button.
  const fn = SRC.slice(SRC.indexOf("async function _atAddFolder"), SRC.indexOf("/** Which folder the Files branch is showing"));
  assert.match(fn, /await _addWorkspaceRoot\(picked\)/, "registration is not optional");
  assert.match(fn, /_atMode = "files";/, "and it lands you back in Files, inside the new folder");
  // The native chooser blurs the composer, and the blur handler hides this menu 150ms later,
  // clearing _atMode and _atDir. Setting them again after the dialog resolves renders into a menu
  // whose token is gone — nothing lists. Focus must return before the render.
  assert.match(fn, /promptEl\.focus\(\);/, "focus has to come back to the composer");
  assert.match(fn, /requestAnimationFrame/, "and the render must wait for the caret to land");
  assert.match(fn, /if \(!_atToken\(\)\)/, "a lost token must be reported, not silently ignored");
  // The row is offered only at the top level with no query — mid-search it would be noise.
  assert.match(SRC, /if \(!_atDir && !query\) \{\s*rows\.unshift\(\{\s*kind: "add-folder"/);
});

test("Directory is its own @ category, listing folders from the workspace", () => {
  const INDEX = fs.readFileSync("index.html", "utf8");
  assert.match(SRC, /\{ id: "directory", label: "Directory"/, "the category must exist");
  // Order matters: it sits below Files, which is where it was asked for.
  const cats = SRC.slice(SRC.indexOf("const _AT_CATEGORIES = ["), SRC.indexOf("];", SRC.indexOf("const _AT_CATEGORIES = [")));
  const order = [...cats.matchAll(/id: "(\w+)"/g)].map((m) => m[1]);
  // MCP 排在最后：它是 2026-08-13 新加的一类（已连接 MCP 服务提供的 resources），
  // 加在末尾就不会把上面几条的位置挪走——那几条的顺序是有人专门要求过的。
  assert.deepEqual(order, ["model", "files", "directory", "github", "gitlab", "mcp"]);
  assert.match(SRC, /if \(_atMode === "mcp"\) rows = _atMcpRows\(query\)/,
    "MCP 这一类也要接进路由，不能只列在表里");
  assert.ok(INDEX.includes('id="i-folder-open"'), "its icon must be a real symbol");
  assert.match(SRC, /else if \(_atMode === "directory"\) rows = await _atDirectoryRows\(query\)/,
    "and it must be routed, not just listed");

  // It reads the existing index rather than re-walking the disk — the index already records a
  // folder as its path plus a trailing slash.
  const fn = SRC.slice(SRC.indexOf("async function _atDirectoryRows"), SRC.indexOf("/** Turn workspace-relative paths into menu rows."));
  assert.match(fn, /\.filter\(\(f\) => f\.endsWith\("\/"\)\)/, "folders come from the index");
  assert.match(fn, /const depth = a\.split\("\/"\)\.length - b\.split\("\/"\)\.length;/,
    "shallow paths first — the folder you mean is usually near the root");
  assert.match(fn, /_insertAtChip\(\{ kind: "file", value: d/, "picking one inserts it as a reference");
  assert.match(fn, /No matching folder/, "an empty result must say so rather than closing the menu");
});

test("the @ menu is exempt from the runtime translator", () => {
  // The categories were authored in English and rendered as 模型 / 文件, because i18n.js translates
  // DOM text at runtime and caches per string — which is why the strings added in the same build
  // (Directory, "Reference a file") still showed English while older ones did not.
  assert.match(SRC, /_atMenu\.setAttribute\("data-i18n-skip", ""\)/,
    "the menu container must opt out");
  const I18N = fs.readFileSync("src/i18n.js", "utf8");
  assert.ok(I18N.includes('"[data-i18n-skip]"'),
    "and i18n.js must still honour that attribute — the opt-out is only as good as the selector");
});

test("each model in the @ menu wears its own vendor mark", () => {
  // Every row shared one generic glyph, so a list of Claude and GPT models looked identical.
  const brandOf = new Function(`${SRC.match(/function brandOf[\s\S]*?\n}/)[0]}; return brandOf;`)();
  assert.equal(brandOf("claude-opus-5").sym, "i-brand-anthropic");
  assert.equal(brandOf("gpt-5.5").sym, "i-brand-openai");
  assert.equal(brandOf("deepseek-v3").sym, "i-brand-deepseek");

  // It must be brandOf, not a second map: the picker already uses it, and a local copy would
  // drift the first time a vendor is added — one glyph in the picker, another in the menu.
  const rows = SRC.slice(SRC.indexOf("function _atModelRows"), SRC.indexOf("async function _atRepoRows") >= 0 ? SRC.indexOf("async function _atRepoRows") : SRC.indexOf("function _atRepoRows"));
  assert.match(rows, /const brand = brandOf\(id\);/, "model rows resolve the brand");
  assert.match(rows, /icon: brand\.sym/);
  assert.doesNotMatch(rows, /icon: "i-sparkle"/, "no hard-coded fallback glyph");

  // The brand class has to survive to the renderer, or every mark paints the same grey.
  assert.match(SRC, /iconSvg\(row\.icon \|\| "i-file", row\.iconClass \|\| "ic--doc"\)/);
  // And the chip keeps the mark after selection.
  assert.match(SRC, /iconSvg\(brandOf\(rel\)\.sym, brandOf\(rel\)\.cls\)/);
});

test("the profile card shows the account's real avatar when it has one", () => {
  // /api/me returns an avatar (auth.rs inserts it); the card ignored it and always drew an
  // initial on a gradient, so the dashboard and the app showed different people.
  assert.match(SRC, /u\.avatar\s*\n?\s*\?\s*`<img class="pf-av pf-av--img" src="\$\{esc2\(u\.avatar\)\}"/,
    "the real avatar must be used when present");
  assert.match(SRC, /onerror=/, "a dead image URL must fall back to the initial, not a broken icon");
  assert.match(SRC, /referrerpolicy="no-referrer"/,
    "an avatar is usually third-party hosted; do not leak where it is being rendered");
  assert.match(SRC, /\.pf-av--img\{object-fit:cover/, "and it must not stretch");
});

test("the account card shows the name when there is one, the email when there is not", () => {
  // One rule, shared by the card, the dropdown and the avatar initial. Three copies is how they
  // drift, which is exactly what happened before this was extracted.
  const identityOf = new Function(
    `const _michaelUser = null; ${SRC.match(/function _accountIdentity[\s\S]*?\n}/)[0]}; return _accountIdentity;`,
  )();
  assert.equal(identityOf({ first_name: "Michael", last_name: "Chen", email: "a@b.com" }), "Michael Chen");
  assert.equal(identityOf({ first_name: "Michael", last_name: "", email: "a@b.com" }), "Michael");
  // Whitespace-only names are not names — an account with " " set must still show its email.
  assert.equal(identityOf({ first_name: " ", last_name: "", email: "a@b.com" }), "a@b.com");
  assert.equal(identityOf({ email: "3266986273@qq.com" }), "3266986273@qq.com");
  // The dropdown knows the email before /api/me lands, so it may pass one in.
  assert.equal(identityOf(null, "fallback@x.com"), "fallback@x.com");

  // The email stays reachable on hover rather than being lost when a name exists.
  assert.match(SRC, /<div title="\$\{esc2\(u\.email \|\| ""\)\}"/);
  // And the initial follows the identity, or a named account shows the letter of its address.
  assert.match(SRC, /const av = \(identity \|\| "\?"\)\.slice\(0, 1\)\.toUpperCase\(\);/);
});

test("the account avatar matches the dashboard's, rule and appearance", () => {
  // The console renders <AvatarImage src={avatar}> with a fallback of
  // (name || email || "?").charAt(0).toUpperCase() on a flat primary fill. The app must be the
  // same account rendered the same way — a gradient here and a flat fill there is the same
  // person looking like two different avatars depending on which surface you opened.
  const SHELL = fs.readFileSync("../server/account-ui/src/components/Shell.tsx", "utf8");
  assert.match(SHELL, /\(name \|\| email \|\| "\?"\)\.charAt\(0\)\.toUpperCase\(\)/,
    "this test is only meaningful while the console still uses that rule");
  assert.match(SRC, /const av = \(identity \|\| "\?"\)\.slice\(0, 1\)\.toUpperCase\(\);/,
    "the app must use the same rule");
  assert.match(SRC, /const identity = _accountIdentity\(u\);/,
    "the card uses the shared rule rather than its own copy");
  assert.match(SRC, /const _identity = _accountIdentity\(_michaelUser, _loggedInEmail\);/,
    "and so does the dropdown");
  // Flat fill, no gradient, no shadow.
  assert.match(SRC, /\.pf-av\{[^}]*background:#1a73e8;/);
  assert.doesNotMatch(SRC.slice(SRC.indexOf('".pf-av{'), SRC.indexOf('".pf-av{') + 260), /linear-gradient|box-shadow/,
    "the card must not invent its own avatar styling");
});

test("the account dropdown shows the same identity and avatar as the card", () => {
  // It showed the raw email and a generic person outline, so the same account looked like two
  // different accounts depending on whether you opened the menu or the card.
  assert.match(SRC, /dropName\.textContent = _identity;/, "the dropdown shows the identity");
  assert.match(SRC, /dropName\.title = _loggedInEmail;/, "with the email still reachable on hover");
  assert.match(SRC, /_setDropdownAvatar\(_michaelUser, _identity\);/, "and the real avatar");

  const fn = SRC.slice(SRC.indexOf("function _setDropdownAvatar"), SRC.indexOf("function _accountIdentity"));
  assert.match(fn, /u\?\.avatar \? document\.createElement\("img"\)/, "an image when the account has one");
  assert.match(fn, /next\.referrerPolicy = "no-referrer";/, "third-party host learns nothing");
  assert.match(fn, /next\.onerror = \(\) => _setDropdownAvatar\(\{ \.\.\.u, avatar: "" \}, identity\);/,
    "a dead URL falls back to the letter rather than a broken frame");
  assert.match(fn, /\(identity \|\| "\?"\)\.slice\(0, 1\)\.toUpperCase\(\)/, "same initial rule as the card");
  assert.match(APP_CSS, /img\.settings-dropdown__avatar--filled \{ object-fit: cover; \}/);
});

const ctxInput = load("_contextInputTokens", ["_contextInputTokens"]);
const applyReading = load("_applyContextReading", ["_applyContextReading"]);
const readingForStorage = load("_ctxReadingForStorage", ["_ctxReadingForStorage"]);
const readingFromStorage = load("_ctxReadingFromStorage", ["_ctxReadingFromStorage"]);
const meterLimit = (native, choice) => new Function(
  "_modelContextLimit", "_ctxChoiceFor", "_nativeWindowsFor",
  `${grab("_contextMeterLimit")}\nreturn _contextMeterLimit("m");`,
)(() => native, () => choice, () => [native]);

test("context counts the cached prompt, which is where the whole conversation lives", () => {
  // This is why the meter looked frozen. Anthropic reports `input_tokens` EXCLUDING everything
  // served from cache, so a warm turn deep in a long conversation reports a few thousand tokens
  // no matter how much history is behind it — the uncached tail does not grow. Reading only that
  // number measured the last message, not the context.
  const warmTurn = { prompt: 4_600, cacheRead: 120_000, cacheWrite: 0 };
  assert.equal(ctxInput({ ...warmTurn, normalized: true }), 4_600,
    "a reading the transport already normalized is taken as-is");

  // A cache-write count only ever occurs on the shape that excludes cache, so the sum is exact.
  assert.equal(ctxInput({ prompt: 4_600, cacheRead: 90_000, cacheWrite: 30_000 }), 124_600);

  // Without one the two shapes are indistinguishable. Either answer is wrong for the other, but
  // only one of the two errors compounds: OpenAI's prompt total CONTAINS the cached part, so
  // adding would report a conversation as twice its real size and drive the gauge to full.
  assert.equal(ctxInput({ prompt: 120_000, cacheRead: 118_000, cacheWrite: 0 }), 120_000,
    "OpenAI shape: cached is a subset of prompt, never added to it");
  assert.equal(ctxInput({ prompt: 4_600, cacheRead: 120_000, cacheWrite: 0 }), 120_000,
    "Anthropic shape without a write: short by the uncached tail, never doubled");

  assert.equal(ctxInput({ prompt: 9_000, cacheRead: null, cacheWrite: 0 }), 9_000, "no cache reported");
  assert.equal(ctxInput({}), 0);
});

test("both transports normalize usage the same way before the meter sees it", () => {
  // The desktop goes through Rust and the web build through the fetch reader. They have to agree,
  // because the same conversation can be opened in either and the number must not change.
  const rust = fs.readFileSync("src-tauri/src/ai.rs", "utf8");
  assert.match(rust, /let prompt = if cache_read\.is_some\(\) \{\s*\n\s*prompt_raw \+ cached \+ cache_creation/,
    "Rust adds cache only on the shape that excludes it");
  assert.match(SRC, /u\.cache_read_input_tokens != null\s*\n\s*\?\s*promptRaw \+ \(cached \|\| 0\) \+ cacheWrite\s*\n\s*:\s*promptRaw/,
    "the browser transport applies the identical discriminator");

  // Usage is read ahead of the Stop gate in both loops: stopping a run does not un-spend tokens.
  const usageHandlers = SRC.match(/if \(ev && ev\.kind === "usage"\) \{/g) || [];
  assert.equal(usageHandlers.length, 2, "the agent loop and plain chat both consume stream usage");
  const agentTail = SRC.slice(SRC.indexOf('if (ev && ev.kind === "usage") {'));
  assert.ok(agentTail.indexOf("_recordStreamUsage") < agentTail.indexOf("if (!_live()) return false;"),
    "usage is recorded before the Stop gate, not after it");
});

test("a reading for the same request may only be corrected upward; a new one replaces it", () => {
  const session = {};

  // The stream reading lands first and is exact.
  assert.equal(applyReading(session, { input: 124_600, output: 900, requestId: "r1" }), true);
  assert.equal(session._ctxRealFloor.total, 125_500);

  // The billing settlement for that same request arrives later carrying the merged, shape-blind
  // numbers. It must not walk the meter backwards to the uncached tail.
  assert.equal(applyReading(session, { input: 4_600, output: 900, requestId: "r1" }), false);
  assert.equal(session._ctxRealFloor.total, 125_500);

  // The next turn replaces it — including downward. Compaction genuinely shrinks the context, and
  // a gauge that only ever climbed would hide the one moment the user most wants to see.
  assert.equal(applyReading(session, { input: 30_000, output: 500, requestId: "r2" }), true);
  assert.equal(session._ctxRealFloor.total, 30_500);

  assert.equal(applyReading(session, { input: 0, output: 0, requestId: "r3" }), false,
    "an empty reading is no reading; it must not blank a real one");
  assert.equal(session._ctxRealFloor.total, 30_500);
  assert.equal(applyReading(null, { input: 100, output: 1 }), false);
});

test("the reading survives a restart, including records written before the breakdown existed", () => {
  const session = { _ctxRealFloor: null };
  applyReading(session, { input: 124_600, output: 900, cacheRead: 120_000, cacheWrite: 0, model: "claude-opus-5", requestId: "r1" });

  const restored = readingFromStorage(JSON.parse(JSON.stringify(readingForStorage(session))));
  assert.equal(restored.total, 125_500);
  assert.equal(restored.input, 124_600);
  assert.equal(restored.output, 900);
  assert.equal(restored.cacheRead, 120_000);
  assert.equal(restored.model, "claude-opus-5");

  // 0.3.74 shipped a total and a timestamp and nothing else. Those conversations must still read
  // their real size rather than fall back to an estimate that cannot see the system prompt.
  const legacy = readingFromStorage({ total: 88_000, at: 1 });
  assert.equal(legacy.total, 88_000);
  assert.equal(legacy.input, 88_000, "with no breakdown the whole total is input");

  assert.equal(readingFromStorage({ total: 0 }), null);
  assert.equal(readingFromStorage(undefined), null);
  assert.equal(readingForStorage({}), undefined, "a session with no reading writes no key");

  // Both serializers and both rehydrators go through that one pair — the session record has lost
  // a field to a divergent copy twice already.
  assert.equal((SRC.match(/ctxFloor: _ctxReadingForStorage\(/g) || []).length, 2);
  assert.equal((SRC.match(/_ctxReadingFromStorage\(sData\.ctxFloor\)/g) || []).length, 2);
});

test("once the provider has reported, the local estimate is out of the loop", () => {
  // Mixing them was the earlier design. A local assemble() sees neither the system prompt nor the
  // tool schemas nor what the gateway compressed away, so it can only distort a number that is
  // already exact — and it re-derived the whole transcript on a keystroke timer to do it.
  const refresh = grab("_refreshContextMeterFromDraft");
  assert.match(refresh, /if \(Math\.max\(0, Number\(real\?\.total\) \|\| 0\) > 0\) \{[\s\S]*?_setContextMeterFromReading\(real, draftTokens\);\s*\n\s*return;/,
    "a real reading paints the meter and returns before the estimator is consulted");
  assert.ok(refresh.indexOf("_setContextMeterFromReading") < refresh.indexOf("_estimateActiveSessionContextTokens"),
    "the estimate is the fallback, not the default");
  assert.doesNotMatch(grab("_estimateActiveSessionContextTokens"), /_ctxRealFloor/,
    "the estimator no longer reaches for the real reading at all");
});

test("the percentage measures the window the model reads, so it can actually move", () => {
  // The membership tier is not a context window — it is how much history the gateway will hold
  // and compress down to fit one. Dividing by it put a full 200k request at 4% of a 5M
  // entitlement, and the ring sat on zero however long the conversation ran.
  assert.equal(meterLimit(200_000, 0), 200_000);
  assert.equal(200 * 125_500 / meterLimit(200_000, 0) / 2, 62.75, "125k of a 200k window reads 63%");

  // An explicit window choice is a real narrowing and counts; it cannot exceed what the model reads.
  assert.equal(meterLimit(200_000, 64_000), 64_000);
  assert.equal(meterLimit(200_000, 5_000_000), 200_000);
  assert.equal(meterLimit(0, 0), 1, "an unknown model still divides by something");

  // The tier stays visible, in the tooltip. An earlier build divided by the native window and a
  // 5M account saw a permanent "200.0k" — a working feature displayed as if it were off.
  assert.match(SRC, /const tierLimit = Math\.max\(0, Number\(_effectiveContextLimit\(model\)\) \|\| 0\);/);
  assert.match(SRC, /if \(tierLimit > state\.limit\) lines\.push\(`档位 可留存/);

  // The ring carries the percentage; the amount is on hover, first line, stated exactly.
  assert.match(SRC, /label\.textContent = pct >= 100 \? "满" : String\(pct\)/);
  assert.match(SRC, /const lines = \[`上下文 \$\{_tokenExact\(state\.total\)\} \/ \$\{k\(state\.limit\)\} · \$\{pct\}%`\];/);
  assert.match(APP_CSS, /\.cache-ring::after \{[^}]*content: attr\(data-tooltip\);/);
  assert.match(APP_CSS, /\.cache-ring::after \{[^}]*white-space: pre-line;/,
    "the tooltip is multi-line; without this it collapses into one unreadable run");
});

test("the hover panel says only what it has to say, on top of the conversation", () => {
  // Six lines fired unconditionally before this: the same figure in three vocabularies, zeros
  // where there was nothing to report, and a closing note restating the line above it. The box
  // ended up wide enough to cover the conversation it was floating over.
  const render = grab("_renderTokenMeter");
  const pushes = render.match(/lines\.push\(/g) || [];
  assert.ok(pushes.length >= 6, "the detail is still available");
  for (const guard of [
    /if \(state\.estimated\) \{/,
    /\} else if \(\(state\.prompt \|\| 0\) > 0 \|\| \(state\.completion \|\| 0\) > 0\) \{/,
    /if \(_tok\.anyReal\) \{/,
    /if \(tierLimit > state\.limit\)/,
    /if \(_activeThinkEffort && _activeThinkEffort !== "off"\)/,
    /if \(pct >= 100\)/,
  ]) assert.match(render, guard, "every line past the first is conditional");
  assert.equal((render.match(/const tooltip = lines\.join\("\\n"\);/g) || []).length, 1);

  // The cache split drops the parts that are zero rather than printing them.
  assert.match(render, /if \(state\.cached\) split\.push/);
  assert.match(render, /if \(cacheWrite\) split\.push/);
  assert.match(render, /if \(uncached\) split\.push/);
  assert.match(render, /split\.length \? `（\$\{split\.join\(" · "\)\}）` : ""/);

  // Nothing left over from the old string: a dead helper that still computes reads as if it
  // were displayed somewhere.
  assert.doesNotMatch(render, /_thinkShort|_thinkDetail|sourceText/,
    "the retired pieces are gone, not merely unreferenced");
  assert.doesNotMatch(render, /注：供应商尚未上报真实 usage/);

  // And it paints above the transcript. A message carries content-visibility, which promotes it
  // to its own stacking context, so DOM order alone did not keep chat text out of the box.
  assert.match(APP_CSS, /\.composer \{ position: relative; z-index: 60; \}/);
  assert.match(APP_CSS, /\.cache-ring::after \{[\s\S]*?z-index: 200;/);
  assert.match(APP_CSS, /\.cache-ring::after \{[\s\S]*?background: var\(--panel-solid, #fff\);/,
    "a missing custom property degrades to see-through, not to opaque");
});

const claudeGen = load("_claudeGeneration", ["_claudeGeneration"]);

test("the thinking switch splits by generation, not by named version", () => {
  // Naming versions one at a time is what left Sonnet 4.5, Opus 4.5 and Opus 4.1 on the adaptive
  // shape: only 4.6 was listed, so everything older fell through to the modern branch and got
  // sent `{"type":"adaptive"}` — a mode none of them supports, and on Sonnet 4.5 the effort
  // parameter errors outright.
  assert.equal(claudeGen("claude-opus-4-8"), 4.8);
  assert.equal(claudeGen("claude-sonnet-5"), 5);
  assert.equal(claudeGen("claude-opus-4-1"), 4.1);
  assert.equal(claudeGen("claude-opus-4-0"), 4);
  assert.equal(claudeGen("claude-fable-5"), 5);

  // A dated snapshot suffix is not a minor version, wherever the family name sits in the id.
  assert.equal(claudeGen("claude-3-7-sonnet-20250219"), 3.7);
  assert.equal(claudeGen("claude-opus-4-5-20251101"), 4.5);
  assert.equal(claudeGen("claude-sonnet-4-5-20250929"), 4.5);

  // Unrecognised reads as newer than the table — the adaptive shape, which is the direction the
  // API moved and the one a model released after this line will accept.
  assert.equal(claudeGen("some-unreleased-claude"), 0);
  assert.match(SRC, /const _gen = _claudeGeneration\(s\);\s*\n\s*if \(_gen > 0 && _gen <= 4\.6\) \{/,
    "zero must fall through to adaptive, not into the budget branch");

  // The gateway splits the same way, from the same number — the client's dial and the request
  // the gateway actually sends have to describe one contract.
  const RS = fs.readFileSync("../server/src/models.rs", "utf8");
  assert.match(RS, /if claude_generation\(&m\) > 0\.0 && claude_generation\(&m\) <= 4\.6 \{/);
  assert.match(RS, /fn claude_generation\(model_lower: &str\) -> f64 \{/);
});

test("every tier the model card offers is a tier the request can carry", () => {
  const profile = grab("_thinkingProfileFor");

  // `low` reaches the model as effort=low and always did once the client started sending effort;
  // it was dropped back when every level below `high` collapsed to the same request.
  assert.match(profile, /levels: _alwaysThinks \? \["low", "medium", "high", "max"\] : \["off", "low", "medium", "high", "max"\]/);

  // Fable/Mythos cannot be turned off — thinking is always on there and an explicit disable is a
  // 400. An off button would be a tier that cannot exist.
  assert.match(profile, /const _alwaysThinks = \/fable\|mythos\/\.test\(s\);/);

  // `xhigh` 在 Claude 这一族的档位里仍然缺席，但**理由变了**，所以断言也得跟着变。
  //
  // 以前的理由是「这条路是转卖渠道，它不认识的 effort 词会返回空 completion」——那是一条
  // 从未被真实探测验证过的推断，两个仓库各写了一条同样意思的注释、互相引用。
  // 现在网关把封顶做成了每条线路可配的开关（models.rs 的 anthropic_effort_word +
  // models 表的 effort_passthrough 列），默认关着 = 保持旧行为。
  //
  // 所以这里要钉的是：**开关默认关着的时候，前端不能摆一个到不了模型的按钮**。
  // 等某条线路真的把直通打开、确认上游认这个词了，再把按钮和这条断言一起改。
  const RS = fs.readFileSync("../server/src/models.rs", "utf8");
  assert.match(RS, /fn anthropic_effort_word\(requested: &str, passthrough: bool\)/,
    "封顶必须是可配的函数，不能又写回硬编码 match");
  assert.match(RS, /\("xhigh", false\) \| \("max", false\) => "high",/,
    "默认（passthrough=false）必须仍然封顶在 high —— 升级不改变任何现有流量");
  assert.match(RS, /\("xhigh", true\) => "xhigh",/, "直通打开时 xhigh 要真的发出去");
  // 深度梯子在两种配置下都不能倒挂：更深的档不能拿到更小的输出余量。
  assert.match(RS, /Some\("xhigh"\) if effort_passthrough => 52000,/);
  assert.match(RS, /Some\("high"\) \| Some\("xhigh"\) => 40000,/);
  const claudeLevels = profile.match(/levels: _alwaysThinks \? \[[^\]]*\] : \[[^\]]*\]/)[0];
  assert.doesNotMatch(claudeLevels, /xhigh/, "直通默认关着，就不该摆一个到不了模型的按钮");

  // 反过来：走 OpenAI 协议透传的那一族（gpt-5.6）网关一个字都不改，xhigh 是真的能到模型的，
  // 所以那边不但要有按钮，默认就该是 xhigh —— 参照实现（opencode / Claude Code）也是这么设的。
  // 窗口要够宽：那一段注释里记着 2026-08-13 的实测数据，窄了会把 defaultLevel 切在外面。
  const gpt56 = SRC.slice(SRC.indexOf('gpt[-_.]?5\\.6'), SRC.indexOf('gpt[-_.]?5\\.6') + 2000);
  assert.match(gpt56, /levels:\s*\[[^\]]*"xhigh"[^\]]*\]/);
  assert.match(gpt56, /defaultLevel:\s*"xhigh"/,
    "gpt-5.6 走透传线路，默认停在 high 等于让不动转盘的人永远浅一档");

  // The old hint said the dial only moved output headroom and timeouts. It sends effort.
  assert.doesNotMatch(profile, /档位只影响输出余量与超时分级/);
  assert.match(profile, /档位通过 output_config\.effort 送到模型/);
});

test("the native context window agrees end to end", () => {
  // 1M on the 4.6-and-later families, 200K before them — the client budgets requests against
  // this number and the gateway plans compression against its own copy, so a disagreement is a
  // silent 413 or a discarded three quarters of the window.
  const RS = fs.readFileSync("../server/src/models.rs", "utf8");
  const nativeLimit = load("_fallbackModelContextLimit", ["_fallbackModelContextLimit"]);
  for (const id of ["claude-opus-5", "claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6",
                    "claude-sonnet-5", "claude-sonnet-4-6", "claude-fable-5", "claude-mythos-5"]) {
    assert.equal(nativeLimit(id), 1_000_000, id);
  }
  for (const id of ["claude-opus-4-5", "claude-opus-4-1", "claude-haiku-4-5", "claude-sonnet-4-5"]) {
    assert.equal(nativeLimit(id), 200_000, id);
  }
  const table = RS.slice(RS.indexOf("fn official_contexts"), RS.indexOf("fn official_context("));
  for (const fam of ["opus-4-6", "opus-4-7", "opus-4-8", "opus-5", "sonnet-4-6", "sonnet-5", "fable-5", "mythos-5"]) {
    assert.ok(table.includes(`"${fam}"`), `the gateway's table must name ${fam} too`);
  }
  assert.match(table, /return vec!\[\(1_000_000, None\)\];/);
});

test("the paid tier stacks on the model's own window, in the client as in the gateway", () => {
  // The buttons have always been priced additively — native + tier, mirroring capacity_for_native
  // in the gateway (server/src/compression.rs). The effective limit took the LARGER of the two,
  // so on any 1M-native model every tier resolved back to 1M: the whole row was a no-op and the
  // top tier a subscriber pays for could never light up. The two lines disagreed by construction.
  const eff = (native, tierMax, choice) => new Function(
    "_modelContextLimit", "_nativeWindowsFor", "_michaelUser", "_gatewayHandlesCompression", "_ctxChoiceFor",
    `${grab("_effectiveContextLimit")}\nreturn _effectiveContextLimit("m");`,
  )(() => native, () => [native], { michael_compression: { max_input_tokens: tierMax } }, () => tierMax > 0, () => choice);

  assert.equal(eff(1_000_000, 5_000_000, 0), 6_000_000, "1M native + a 5M tier is 6M, not 5M");
  assert.equal(eff(1_000_000, 5_000_000, 6_000_000), 6_000_000, "the top tier must be reachable");
  assert.equal(eff(200_000, 1_000_000, 1_200_000), 1_200_000);
  assert.equal(eff(1_000_000, 0, 6_000_000), 1_000_000, "no membership → never a fictional window");

  const RS = fs.readFileSync("../server/src/compression.rs", "utf8");
  assert.match(RS, /pub fn capacity_for_native\(self, native: usize\) -> usize \{\s*\n\s*native\.saturating_add\(self\.max_input_tokens\(\)\)/,
    "the gateway grants native PLUS the tier; the client must compute the same ceiling");
});

test("a native window the user picked is the one that takes effect", () => {
  // The stored record kept only the KIND, so a model with two native windows — Sonnet 4.5 has
  // 200K by default and 1M behind the context-1m beta — always resolved back to the default and
  // the 1M button could never do anything.
  const choiceFor = (rec, windows, dflt) => new Function(
    "_ctxChoiceRecord", "_modelContextLimit", "_modelCatalogEntry",
    `${grab("_nativeWindowsFor")}\n${grab("_ctxChoiceFor")}\nreturn _ctxChoiceFor("m");`,
  )(() => rec, () => dflt, () => ({ contextWindows: windows.map((t) => ({ tokens: t })) }));

  const twoWindows = [200_000, 1_000_000];
  assert.equal(choiceFor({ kind: "native", tokens: 1_000_000 }, twoWindows, 200_000), 1_000_000);
  assert.equal(choiceFor({ kind: "native", tokens: 200_000 }, twoWindows, 200_000), 200_000);
  // A window the route has withdrawn must not strand a value nothing can deliver.
  assert.equal(choiceFor({ kind: "native", tokens: 1_000_000 }, [200_000], 200_000), 200_000);
  // Records written before the figure was stored still load.
  assert.equal(choiceFor({ kind: "native" }, twoWindows, 200_000), 200_000);

  assert.match(SRC, /if \(kind === "native"\) map\[key\] = \{ kind: "native", tokens: Math\.max\(0, Math\.round\(tokens\) \|\| 0\) \};/,
    "the click has to record WHICH window was clicked");
});

test("off is said out loud, because silence means the opposite on the 5 series", () => {
  // Opus 5 and Sonnet 5 run adaptive thinking when no thinking key is sent. Dropping the field
  // made the cheapest dial the deepest and most expensive one — and because the gateway grants
  // output headroom only to turns that announce thinking, that turn also ran on the bare default
  // while adaptive thinking consumed it, cutting the visible answer off mid-sentence.
  assert.match(grab("_applyThinkingToConfig"),
    /\} else if \(profile\.kind === "adaptive_thinking"\) \{[\s\S]{0,600}out\.thinking = \{ type: "disabled" \};/);

  const RS = fs.readFileSync("../server/src/models.rs", "utf8");
  assert.match(RS, /return default_is_on\.then\(\|\| json!\(\{"type":"disabled"\}\)\);/,
    "and the gateway must forward it rather than dropping the key again");
  assert.match(RS, /if t\.get\("type"\)\.and_then\(\|v\| v\.as_str\(\)\) == Some\("disabled"\) \{\s*\n\s*return "off";/,
    "an explicit disable must read as off, not as the bare-toggle high");
  // Absent stays absent: a caller that names no effort wants the model's own default.
  assert.match(RS, /None => return None,/);
});

test("how much a model can WRITE is carried, not guessed", () => {
  // The catalogue had a context window and nothing else, so the pipeline guessed twice: a flat
  // 128,000 clamp with no model in scope — which Haiku 4.5, capped at 64,000, rejects — and an
  // invented 8,192 default that truncated long answers on thinking-off turns.
  const RS = fs.readFileSync("../server/src/models.rs", "utf8");
  assert.match(RS, /fn official_max_output\(model_id: &str\) -> Option<i64> \{/);
  assert.match(RS, /"max_output_tokens": official_max_output\(&mid\),/, "and it must reach the client");
  assert.match(RS, /max_tokens\.clamp\(1, official_max_output\(model_str\)\.unwrap_or\(128000\)\)/);
  assert.doesNotMatch(RS, /max_tokens\.clamp\(1, 128000\)/, "no blanket ceiling for every model");

  assert.match(SRC, /function _modelMaxOutput\(modelId = ""\) \{/);
  assert.match(SRC, /maxOutput: Math\.max\(0, Math\.round\(Number\(it\.max_output_tokens \?\? it\.maxOutputTokens\) \|\| 0\)\),/);
  // Unknown says nothing rather than having a number invented for it.
  const helper = grab("_modelMaxOutput");
  assert.match(helper, /if \(!entry\) return 0;/);
  assert.match(SRC, /out > 0 \? ` · 单次输出上限 \$\{_tokenShort\(out\)\}` : ""/);
});

test("the browser preview does not invent a window the model does not have", () => {
  // Both preview seeds hardcoded 200,000 for Opus 4.6, whose real window is 1M — a fabricated
  // button in the native row and a context ring inflated five-fold.
  assert.equal((SRC.match(/contextLimit: 1000000, contextWindows: \[\{ tokens: 1000000, beta: null \}\]/g) || []).length, 2);
  assert.doesNotMatch(SRC, /contextLimit: 200000, contextWindows: \[\]/);
});

test("the paint optimization that blanks WebKit is granted, not assumed", () => {
  // content-visibility: auto is what keeps a long conversation cheap — it skips layout and paint
  // for everything off-screen. In WebKit it is also what leaves blank regions behind on scroll:
  // the element is skipped and its placeholder paints empty, so scrolling down can land on a
  // white page. macOS runs this app in WKWebView and cannot choose otherwise; Windows runs
  // WebView2, which is Chromium and does not have the defect.
  // Declarations only. Matching every line that mentions the property would also match the
  // comment explaining the gate, which is how a source-grep test ends up passing on its own prose.
  const guarded = APP_CSS.match(/^[^\n/*]*\{[^\n}]*content-visibility: auto[^\n]*$/gm) || [];
  assert.ok(guarded.length >= 4, "the known content-visibility rules must still be found");
  for (const rule of guarded) {
    assert.match(rule, /\.cv-safe /,
      `every content-visibility rule must be behind the engine gate: ${rule.trim()}`);
  }
  // Earned, not assumed: nothing sets the class until the engine has been identified, so a blank
  // page is never what happens while that line is still loading.
  assert.match(SRC, /if \(\/AppleWebKit\/\.test\(ua\) && !\/Chrome\|Chromium\|Edg\\\/\/\.test\(ua\)\) document\.body\.classList\.add\("is-webkit"\);\s*\n\s*else document\.body\.classList\.add\("cv-safe"\);/);
  assert.match(SRC, /catch \{ \/\* no navigator \(tests\) → stay on the safe side and skip the optimization \*\/ \}/);
});

test("an interrupted turn keeps what the model already wrote", () => {
  // The notice said "已生成的内容...都已保留" and none of it was. Three places threw the prose
  // away independently, so fixing any one of them alone changed nothing on screen.
  const turn = grab("_agentModelTurn");

  // 1. The return value blanked it whenever a tool call had started. That rule assumes a tool
  //    card to state the action and a following turn to write the conclusion — a dead turn
  //    gets neither.
  assert.match(turn, /text: \(_hasNonControlToolCall && !err\) \? "" : cleanFinal,/);

  // 2. The DOM element was removed the moment a tool name arrived, and the final render
  //    refused to put it back.
  assert.match(turn, /const _keepProse = \(!_hasNonControlToolCall \|\| !!err\) && cleanFinal\.trim\(\);/);
  assert.doesNotMatch(turn, /\} else if \(!_hasNonControlToolCall && cleanFinal\.trim\(\)\) \{/,
    "the fallback render path must use the same rule, or the two disagree");

  // 3. The run loop broke on the error BEFORE the line that accumulates the prose into the run
  //    summary — which is the only thing persisted to history. This is the one that made it
  //    survive a restart as well as a repaint.
  assert.match(SRC, /if \(turn\.error\) \{[\s\S]{0,900}summaryText \+= \(summaryText \? "\\n\\n" : ""\) \+ turn\.text\.trim\(\);[\s\S]{0,700}finalErr = turn\.error; break;/);
  // 正文之外，**已经落盘的文件**也要收账再走。流完即写是在流式阶段就真写磁盘的，
  // 而这条 break 走在批处理之前：不收的话，落了盘的文件在消息历史、run 摘要、账本里
  // 一个记录都没有——磁盘变了，所有记录都说没变。
  // 注意锚点：`if (turn.error) {` 在子智能体循环里也有一处，而且排在前面。
  // 用主循环那段独有的注释定位。
  const errBranch = SRC.slice(SRC.indexOf("Bank whatever the dying turn managed to write"));
  assert.match(errBranch.slice(0, 1400), /await _settleEagerWritesForBreak\(run\)/,
    "出错那一轮不收流完即写的账");

  // And the notice now describes what actually happened to the interrupted call, instead of
  // implying every file change landed.
  assert.match(SRC, /被打断的那次工具调用参数不完整，没有执行，也不会重放。/);
  assert.doesNotMatch(SRC, /已生成的内容和文件改动都已保留/);
});

test("a run does not end while its own plan has open steps", () => {
  const loop = grab("_runAgenticLoop");

  // The reported bug: the agent plans seven steps, does two, writes a paragraph, and stops.
  // A turn with no tool calls is the loop's main exit, and every condition that could force
  // another iteration there — diagnostics, a red build, queued user steering — read something
  // other than the plan. The two nudges that DO read the plan live in the tool-execution
  // branch, which a quiet turn never reaches. So nothing in the loop connected "the model
  // stopped" to "the list is unfinished".
  const quiet = loop.slice(loop.indexOf("if (!turn.toolCalls.length)"),
                           loop.indexOf("// Render every tool step up front"));
  assert.match(quiet, /run\._planSteps[\s\S]{0,300}status === "pending" \|\| step\?\.status === "in_progress"/);
  assert.match(quiet, /\(run\._planFinishNudges \|\| 0\) < 3/, "bounded, so an unfinishable plan converges");
  assert.match(quiet, /run\._incompleteReason = run\._incompleteReason \|\| `plan_steps_pending:/);

  // And the other exit: "第三步做完了，要不要我继续？" trips the wait-for-user boundary, which
  // also read nothing about the plan — and cleared the pending nudges on its way out.
  const boundary = loop.slice(loop.indexOf("if (_agentTurnMustWaitForUser(turn))"));
  assert.match(boundary.slice(0, 1600), /run\._planQuestionIntercepted/);
  assert.ok(boundary.indexOf("_planQuestionIntercepted") < boundary.indexOf("awaitingUserReply = true"),
    "the interception has to come before the run decides it is waiting");
});

test("agent turns are not demoted below the depth this route can deliver", () => {
  // The clamp was written when Claude ran on budget_tokens=24000 and would think for four
  // minutes without touching a tool — that was measured and the clamp was right. This family is
  // adaptive now: depth is chosen per turn by the model and interleaved with tool calls, so the
  // premise is gone and only the demotion was left. Anthropic's migration guide names xhigh as
  // the setting for coding and agentic work and the default in Claude Code; this route caps at
  // high, so high is already the deepest thing available here. Sending medium put the agent two
  // notches under the reference implementation.
  const apply = grab("_applyThinkingToConfig");
  assert.doesNotMatch(apply, /pref === "high"[\s\S]{0,400}pref = "medium"/,
    "an agent turn must not be demoted out of high");
  assert.match(apply, /opts\.agentTurn && !opts\.isComplexTask && pref === "max"[\s\S]{0,400}pref = "high"/,
    "only the max→high ceiling remains, and that trades output headroom, not depth");
  // An explicit choice still wins, which was the one part of the clamp that was never in doubt.
  assert.match(apply, /const saved = _loadThinkingPrefs\(\)\[preferenceId\];[\s\S]{0,200}if \(!explicit\)/);
});

test("the loop stops advertising continuation machinery it does not have", () => {
  // Seven counters were declared and never touched again — continueNudges among them. Reading
  // the loop, they answer "does anything push the model to keep going?" with a yes that is not
  // true anywhere in the file.
  // Declarations and assignments, not every mention: the comment that records this deletion names
  // them, and a test that counts bare occurrences fails on its own explanation.
  for (const dead of ["continueNudges", "effectNudges", "researchNudges", "verifyNudges",
                      "honestyNudges", "deepReadNudges", "codeVerifyNudges"]) {
    assert.doesNotMatch(SRC, new RegExp("(?:let|const|var)[^\\n;]*\\b" + dead + "\\b"),
      `${dead} is declared and never used — delete it or wire it`);
    assert.doesNotMatch(SRC, new RegExp("\\b" + dead + "\\s*(?:\\+\\+|=[^=])"),
      `${dead} is written to but never read`);
  }
  // The ones that survived are the ones that actually fire.
  for (const live of ["planGateNudges", "toolReminders", "recoveryNudges", "invalidArgNudges"]) {
    assert.ok((SRC.match(new RegExp("\\b" + live + "\\b", "g")) || []).length > 1, live);
  }
});

test("两处确认弹窗默认放行，但机制留着", () => {
  // 软件所有者 2026-08-11 明确要求：命令都能跑、文件夹始终可用，两个弹窗都不要。
  // 做成默认开启的开关而不是删掉机制 —— 它们挡的是真东西（沙箱挡"往工作区外写文件"，
  // 信任挡"执行仓库自带的程序"），对自己的项目是噪音，对刚 clone 下来没读过的代码不是。
  assert.match(SRC, /if \(_autoAllowSandboxEscape\(\)\) return true;/);
  assert.match(SRC, /if \(_autoTrustWorkspaces\(\)\) \{/);

  // 默认放行：只有显式写 "0" 才恢复询问，所以全新安装不会弹。
  for (const fn of ["_autoAllowSandboxEscape", "_autoTrustWorkspaces"]) {
    const body = grab(fn);
    assert.match(body, /!== "0"/, `${fn} 必须默认放行`);
    assert.match(body, /catch \{ return true; \}/, `${fn} 读不到存储时也不该开始弹窗`);
  }

  // 原来的弹窗路径必须完整保留 —— 关掉开关要能一步回到逐条确认，而不是回到一个残缺版本。
  assert.match(SRC, /title: "沙箱拦住了这条命令，要放开限制重跑吗？"/);
  assert.match(SRC, /title: "信任这个文件夹的作者吗？"/);
  assert.match(SRC, /const ok = decision !== "deny";/, "关掉开关后用户仍然能拒绝");
  // 沙箱本身没被拆：这两个开关只影响"被挡住之后要不要问"，不影响约束是否存在。
  assert.match(SRC, /async function _approveSandboxEscape\(command, root\)/);
});

test("表格文件有自己的图标，而不是通用文档图", () => {
  // 截图里 big.csv / euro.csv / gbk.csv / messy.csv 和 .txt/.log 长得一模一样，
  // 但它们在这个软件里会打开成表格窗口 —— 文件树上看不出哪个点开是表格。
  assert.match(SRC, /csv: "csv", tsv: "csv", tab: "csv",/);
  assert.doesNotMatch(SRC, /csv: "document"/);

  const svg = fs.readFileSync("src/assets/file-icons/csv.svg", "utf8");
  assert.match(svg, /<svg[^>]*viewBox="0 0 32 32"/, "与本套其它图标同一画布");
  assert.match(svg, /#34A853|#188038/, "沿用这套图标的配色语言");
  assert.doesNotMatch(svg, /<script|onload=/i);

  // 图标映射要覆盖真正会打开成表格的那组扩展名，不能只顺手加 csv。
  const tableExts = SRC.match(/const TABLE_EXTS = new Set\(\[([^\]]*)\]\)/)[1]
    .match(/"([a-z]+)"/g).map((q) => q.replace(/"/g, ""));
  for (const ext of tableExts) {
    assert.match(SRC, new RegExp(`\\b${ext}: "csv"`), `${ext} 会打开成表格，图标要跟上`);
  }
});

test("一轮里每个写完的文件都立刻落盘，而不是只放行第一个", () => {
  // 用户看到的：一轮发了三个 write_file，requirements.txt 写进去了，main.py（207 行内容俱全）
  // 和 crawler.py 挂着「等待执行」，磁盘上什么都没有。原因是即时写盘路径给自己设了
  // 「一轮只写一个」的上限，理由写着"和批处理那条路的同款规则保持一致"——而那条规则不存在：
  // 计数器全文件只有它自己两个读点，批处理从来没查过。
  // 只查代码，不查注释：记录这次删除的那句注释本身就写着这个名字，按裸出现次数断言会被
  // 自己的说明文字判死 —— 今天已经在另外两处踩过同一个坑。
  assert.doesNotMatch(SRC, /(?:function|let|const|var)[^\n;]*_(?:run|set)?EagerMutationCount/,
    "闸门的计数器不能还有定义");
  assert.doesNotMatch(SRC, /_runEagerMutationCount\s*\(|_setRunEagerMutationCount\s*\(/,
    "也不能还有调用点，否则下一个人照着注释又把上限加回来");
  assert.match(SRC, /entry\._eagerDone = true;/, "真正的执行标记留着");

  // 拆闸门有前置条件，顺序不能反 —— 否则把一个文件的风险放大成一整轮的。
  const turn = grab("_agentModelTurn");
  // ① 截断判定必须跑在流末即时写盘之前。上游按 max_tokens 砍断的一轮不算 error，
  //    流"正常"结束，于是半截参数会先落盘、几行之后才被判定为 truncated 并拒绝执行。
  // 断言的是顺序本身，不是两段之间隔了多少字符 —— 固定字宽的窗口会被后来人改一句注释就
  // 判死，而要守的从来是"先判断，再写盘"。
  const _truncAt = turn.indexOf('let truncatedByLimit = finishReason === "length"');
  const _notifyAt = turn.indexOf("if (!turnErr && !truncated) { for (const [, e] of byIndex)");
  assert.ok(_truncAt >= 0 && _notifyAt >= 0, "两段都要还在");
  assert.ok(_truncAt < _notifyAt,
    "截断判定必须跑在流末即时写盘之前，否则是先写盘再判断该不该写");
  // ② 判断"这一轮是不是真写过盘"要看真的执行标记。_eagerNotified 只表示钩子被叫过。
  assert.match(turn, /const eagerExecuted = \[\.\.\.byIndex\.values\(\)\]\.some\(\(e\) => e && e\._eagerDone\)/);
});

test("读取失败时说得出「这个目录是空的」", () => {
  // 空文件夹里，模型 list_dir 拿到 0 项，下一步就去读一个自己编的 demo_crawler。
  // 提示本来该在这时候说话，却因为 [] 是真值、names 是空串，被 if (names) 静静跳过了 ——
  // 唯一重要的那个事实，恰好在它就是全部答案的时候缺席。
  assert.match(SRC, /\} else if \(Array\.isArray\(siblings\)\) \{/,
    "空数组要走进来，不能靠真值判断");
  assert.match(SRC, /是空目录（0 个文件夹 · 0 个文件）。这个文件从来没存在过/);
  assert.doesNotMatch(SRC, /if \(names\) helpHint = `\\n\$\{parentDir\} 里实际有/,
    "旧的真值守卫会把空目录这一支吃掉");

  // 递归列目录走查失败时，绝不能宣布成空目录 —— 那句话会让模型在一个有内容的工作区里
  // 从零开建，而"开建"就是写文件。
  assert.match(SRC, /const _reallyEmpty = Array\.isArray\(entries\) && entries\.length === 0;/);
  assert.match(SRC, /\[ERROR\] 递归列目录失败：顶层有 \$\{entries\.length\} 项/);
  assert.match(SRC, /已达 500 行上限，列表被截断/, "截断要说出来，不能默默停");

  // 三处提示曾经承诺"IDE 会直接拦截"，而那个拦截早被删了 —— 模型一试就知道这句话是假的。
  assert.doesNotMatch(SRC, /IDE 会直接拦截/);
  assert.equal((SRC.match(/都只会失败并白白烧掉一轮/g) || []).length, 3);
});

test("传输层掉线要能被认出来，否则续传那一整套机制根本不会被调用", () => {
  // 用真函数跑，不是读正则 —— 这条判据决定 canResume 走不走，读错一个字就是整套机制静默失效。
  const isRetryable = load("_isRetryableAiError",
    ["_stripAiRetryPrefix", "_isRateLimitedAiError", "_isProviderGatewayStatusError", "_isRetryableAiError"]);

  // 网关校验出被截断的工具参数时，会把已经 200 的响应体中途 abort，桌面端因此发出这一句。
  // 它以前不匹配任何一条规则：network / connection reset 全是英文。
  assert.equal(isRetryable("连接中断（网络波动），已保留生成的部分。"), true);
  // 它的兄弟情况（干净 EOF）一直是可重试的 —— 同样是断线，两个相反的结论，中间没有理由。
  assert.equal(isRetryable("AI stream closed before data: [DONE]（连接提前结束）；响应可能被截断，已拒绝本轮结果，请重试。"), true);
  assert.equal(isRetryable("[tool-stream-retry-exhausted] 连接中断（网络波动），已保留生成的部分。"), true);
  // 真正不该重试的还是不重试：模型名写错重试一百次也是错。
  assert.equal(isRetryable("AI request failed (400): invalid model name"), false);
  assert.equal(isRetryable("AI request failed (429): rate limited"), false, "限流单独处理，不走重试");
});

test("续传成功了要画得出来，正文不能在工具开始时被删掉", () => {
  const turn = grab("_agentModelTurn");

  // _suppressNarrativeForTools 只被置真、从不复位。重来那条路把 acc / byIndex / 思考卡全清了，
  // 唯独留下这一位，于是渲染在入口就 return —— 续传即使成功也一个字都画不出来。
  assert.match(turn, /_suppressNarrativeForTools = false;/,
    "重来的路径必须把它复位，否则修好的续传是看不见的");
  assert.ok(turn.indexOf("_suppressNarrativeForTools = false;", turn.indexOf("let _suppressNarrativeForTools")) >
            turn.indexOf("let _suppressNarrativeForTools"),
    "复位要发生在重置块里，不只是初始化");

  // 工具一开始就删正文：对 write/edit 还有卡片顶上，对 read/search/list/cmd 就是纯粹的字没了。
  assert.doesNotMatch(turn, /_suppressNarrativeForTools = true;[\s\S]{0,400}streamEl\.remove\(\); streamEl = null;/,
    "不能再一见工具名就把已经读到的正文删掉");
  assert.match(turn, /streamEl\.classList\.remove\("agent-seg--stream"\); _degradedProseEl = streamEl; streamEl = null;/,
    "就地降级：留住节点，后续帧另开一段");
  // 松手之后必须留个引用：断线续传要复位渲染时得能撤掉这个孤儿节点，
  // 否则渲染器会把整个 acc 重画进新段落，同一段话出现两次。
  assert.match(turn, /let _degradedProseEl = null;/);
  // 两条续传分支都要复位并撤孤儿——rerequest 那条早就有复位，continue 那条一直没有：
  // 于是"从断点继续"成功之后，模型的整段回答一个字都画不出来。
  const contBranch = turn.slice(turn.indexOf('if (mode === "continue")'), turn.indexOf('mode === "rerequest"'));
  assert.match(contBranch, /_suppressNarrativeForTools = false;/,
    "continue 分支不复位，续传成功也画不出来");
  assert.match(contBranch, /_degradedProseEl\.remove\(\)/, "continue 分支不撤孤儿会重复正文");
});

test("中断和停止都不能把已经写出来的内容丢掉", () => {
  // agent：用户点停止那一轮的正文，以前从入账那行上面 break 走了。
  assert.match(SRC, /if \(!_live\(\)\) \{[\s\S]{0,700}summaryText \+= \(summaryText \? "\\n\\n" : ""\) \+ turn\.text\.trim\(\);[\s\S]{0,700}break;/,
    "按停止的那一轮也要入账");
  // 用户点停止时最需要知道的恰恰是"我按下去之前，它到底改了什么"。
  const stopBranch = SRC.slice(SRC.indexOf("入账那一行在下面几十行之外，这条 break 从它上面走掉"));
  assert.match(stopBranch.slice(0, 1200), /await _settleEagerWritesForBreak\(run\)/,
    "按停止那一轮不收流完即写的账");
  // 收账本身：要等在途写入落定（但有上限，不能让一个卡住的写入把 run 挂死），
  // 并且如实报告哪些成功、哪些失败、哪些没落定。
  const settle = SRC.slice(SRC.indexOf("async function _settleEagerWritesForBreak"));
  assert.match(settle.slice(0, 1600), /Promise\.race\(\[/, "不等在途写入，run 结束了它还在飞");
  assert.match(settle.slice(0, 1600), /setTimeout\(r, 8000\)/, "无上限地等会把 run 挂死");
  assert.match(settle.slice(0, 1600), /已经真实写入磁盘/);
  assert.match(settle.slice(0, 1600), /尝试写入但失败/);
  assert.match(settle.slice(0, 1600), /状态未知/, "没落定的要如实说未知，不能算成功");

  // 普通聊天：两处 push 都挂着 !err —— 答案画在屏幕上却从不落库，下一次渲染就没了。
  assert.doesNotMatch(SRC, /if \(!err && historyContent\.trim\(\)\)/);
  assert.doesNotMatch(SRC, /if \(!err && _cc\)/);
  assert.equal((SRC.match(/（本次回复因上游中断未完成）/g) || []).length, 2,
    "两条路都要存，并且如实标成半截的");
});

test("凭记忆拼 old_string 的许可不能只发给 Claude", () => {
  // 用户实测：编辑 sites/example.py 报「未找到 old_string」。文件真实存在、110 行、刚写完，
  // 所以不是编造文件名，是模型按自己记忆里的内容去改。原因在提示词分家：
  //   _GPT_TUNING  ：「改前必 read_file 看真内容，绝不凭印象拼 old_string」——只发给 GPT
  //   _CLAUDE_TUNING：「局部编辑可在已知上下文给出精确 old_string 直接改」——发给 Claude
  // 于是 claude-opus-5 只拿到宽松那条，严格那条它从来没收到过。而「已知上下文」恰恰是
  // 长 run 里最先被折叠掉的东西：十轮前自己写的文件早就不在上下文里了。
  const gpt = SRC.slice(SRC.indexOf("const _GPT_TUNING"), SRC.indexOf("const _DEEPSEEK_TUNING"));
  const claude = SRC.slice(SRC.indexOf("const _CLAUDE_TUNING"), SRC.indexOf("function _modelFamilyTuning"));

  assert.match(gpt, /绝不凭印象拼 old_string/, "GPT 那条严格规则还在");
  // Claude 那份现在也必须把这条说清楚，而且要说明白为什么记忆不算数。
  assert.match(claude, /逐字符来自\*\*这一轮看得见的真实文件内容\*\*/);
  assert.match(claude, /「我记得我刚写过这个文件」不算|"我记得我刚写过这个文件"不算/);
  assert.doesNotMatch(claude, /可在当前编辑器\/附件\/已知上下文给出精确 old_string 且工具能唯一命中时直接改/,
    "旧的宽松措辞不能留着 —— 它正是这次失败的许可来源");

  // 编辑失败的回执本来就很好（近似匹配、压缩产物识别、带行号回传最接近的真实内容），
  // 这条是防止它一开始就被用上。
  assert.match(SRC, /文件里\*\*最接近的真实内容\*\*是这几行（带真实行号）/);
});
