// package_source：读磁盘上装好的那份依赖，拿真实签名。
//
// 它治的是最难被发现的一类幻觉——**版本号是对的，写法是旧版的**：
// package_search 查过了说装的是 5.x，模型仍按训练记忆写 4.x 的调用形状，
// 而现有的检索层一个都拦不住，因为它们全都够不着 node_modules。
//
// 这个文件里的用例**跑在这个仓库自己的 node_modules 上**，不用夹具：真实的包布局
// （scoped、桶文件、几百个打包产物、269KB 的类型声明）才是会把实现坑掉的东西，
// 手写的夹具全都碰不到。
import test from "node:test";
import { baseTools, readonlyExternalTools, writeTools } from "../src/agent/tool-catalog.js";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..");
// 正向源码断言必须跑在**剥掉注释**的源码上。注释不是代码：把一条契约从代码里删掉、
// 只在注释里留一句，assert.match 照样绿——本仓库已经这样漏过一整组模型可见的工具契约。
// 所以 `SRC` 绑定的是 CODE（注释整段置空，行号与偏移和原文一字不差）；
// 真要匹配注释本身的断言显式用 RAW_SRC，并在那一行写清为什么。
import { CODE as SRC, SRC as RAW_SRC, fnSource as topLevelFn } from "./helpers/source.mjs";
const constLine = (name) => {
  const i = RAW_SRC.indexOf(`const ${name} =`);
  assert.ok(i > 0, `找不到常量 ${name}`);
  return SRC.slice(i, RAW_SRC.indexOf("\n", i) + 1);
};

// 真实文件系统当后端——被测的就是"能不能在真实包布局里找到东西"。
const backend = {
  readDir: async (p) => readdirSync(p).map((name) => ({
    name, path: `${p}/${name}`, is_dir: statSync(`${p}/${name}`).isDirectory(),
  })),
  readTextFile: async (p) => readFileSync(p, "utf8"),
};
const api = new Function("backend",
  constLine("_PKG_SRC_MAX_FILES") + constLine("_PKG_SRC_MAX_BYTES")
  + constLine("_PKG_SRC_MAX_TYPE_BYTES") + constLine("_pkgSrcMaxBytes")
  + topLevelFn("_packageBareName") + topLevelFn("_packageSourceCandidates")
  + topLevelFn("_pkgReadDir") + topLevelFn("_packageInstalledVersion")
  + topLevelFn("_resolvePackageDir") + topLevelFn("_collectPackageFiles")
  + topLevelFn("_extractExportNames")
  + ";return { _packageBareName, _resolvePackageDir, _packageInstalledVersion,"
  + " _collectPackageFiles, _extractExportNames, _pkgSrcMaxBytes };")(backend);

const hasDeps = existsSync(join(ROOT, "node_modules", "exifr"));

test("包名里的 @ ：作用域前缀 vs 版本分隔符", () => {
  // 一条正则想同时管两种，实际把 `@tauri-apps/api` 削成了 `@`——于是所有 @scope/xxx
  // 一律报"未安装"，而那正是前端项目里最常见的一半依赖。
  assert.equal(api._packageBareName("@tauri-apps/api"), "@tauri-apps/api");
  assert.equal(api._packageBareName("@scope/x@1.2.3"), "@scope/x");
  assert.equal(api._packageBareName("zod@3.22"), "zod");
  assert.equal(api._packageBareName("exifr"), "exifr");
});

test("类型声明文件单独一档字节上限", () => {
  // monaco 的 editor.api.d.ts 是 269KB，卡在普通上限外面，于是查 IRange 一无所获。
  // 声明文件恰恰是最该读的那类，而回给模型的只有命中处 ±20 行，读多大都不占上下文。
  assert.ok(api._pkgSrcMaxBytes("a/b/editor.api.d.ts") > 1_000_000);
  assert.ok(api._pkgSrcMaxBytes("a/b/types.pyi") > 1_000_000);
  assert.equal(api._pkgSrcMaxBytes("a/b/bundle.js"), 220_000);
});

test("在真实的 node_modules 里解析包、拿到已安装版本", { skip: !hasDeps && "没装依赖" }, async () => {
  for (const pkg of ["exifr", "@tauri-apps/api", "monaco-editor"]) {
    const hit = await api._resolvePackageDir(ROOT, pkg);
    assert.ok(hit, `${pkg} 没解析到——scoped 包和普通包都必须能找到`);
    const version = await api._packageInstalledVersion(hit.dir, hit.eco);
    assert.match(version, /^\d+\.\d+/, `${pkg} 没拿到已安装版本：${version}`);
  }
  // 不存在的包要明确说没有，不能瞎猜一个目录。
  assert.equal(await api._resolvePackageDir(ROOT, "肯定没有这个包-zzz"), null);
});

test("声明文件优先入列——否则大包的打包产物会把名额吃光", { skip: !hasDeps && "没装依赖" }, async () => {
  /*
   * monaco-editor 的遍历会先撞上 `dev/vs/` 底下几百个打包产物。一遍走 + 收完再排序
   * 救不了——排序只能排已经收进来的，而真正该看的 editor.api.d.ts 那时压根没进来。
   */
  const hit = await api._resolvePackageDir(ROOT, "monaco-editor");
  const files = await api._collectPackageFiles(hit.dir);
  assert.ok(files.some((f) => f.endsWith("editor.api.d.ts")),
    `monaco 的类型声明没被收进来：${files.slice(0, 5).join(" ")}`);
  assert.match(files[0], /\.d\.ts$/, `第一个文件该是类型声明，实际是 ${files[0]}`);
  // 而且必须正好是 package.json 里**官方声明的那个入口**——包里 .d.ts 往往有几十个，
  // "随便哪个 .d.ts 排第一"和"官方说的入口排第一"是两回事：入口描述的才是对外形状。
  const declared = JSON.parse(readFileSync(join(hit.dir, "package.json"), "utf8"));
  const entry = String(declared.types || declared.typings || "").replace(/^\.\//, "");
  assert.ok(entry, "monaco 的 package.json 该有 types 字段（用例前提变了就改这里）");
  assert.equal(files[0], `${hit.dir}/${entry}`,
    `官方类型入口没排在最前：期望 ${entry}，实际 ${files[0]}`);
});

test("符号模式在三种真实写法上都拿得到定义", { skip: !hasDeps && "没装依赖" }, async () => {
  // 三种写法各代表一类真实包：
  //   export function   —— exifr（直接导出）
  //   declare function  —— @tauri-apps/api（声明和导出分开写，.d.ts 里最常见）
  //   裸 interface      —— monaco（命名空间里的类型）
  const i = RAW_SRC.indexOf("const esc = wantSymbol.replace");
  assert.ok(i > 0, "找不到符号匹配那段");
  const buildRe = new Function("wantSymbol", SRC.slice(i, RAW_SRC.indexOf("\n      );", i) + 9) + ";return defRe;");

  const find = async (pkg, symbol) => {
    const hit = await api._resolvePackageDir(ROOT, pkg);
    const files = await api._collectPackageFiles(hit.dir);
    const re = buildRe(symbol);
    for (const f of files) {
      let text;
      try { text = readFileSync(f, "utf8"); } catch { continue; }
      if (!text || text.length > api._pkgSrcMaxBytes(f)) continue;
      const line = text.split(/\r?\n/).find((l) => re.test(l));
      if (line) return line.trim();
    }
    return null;
  };

  assert.match(await find("exifr", "parse") || "", /export function parse\(/);
  assert.match(await find("@tauri-apps/api", "invoke") || "", /declare function invoke</);
  assert.match(await find("monaco-editor", "IRange") || "", /interface IRange/);
  // 不存在的导出必须回空，不能拿个相似的糊弄过去。
  assert.equal(await find("exifr", "肯定没有这个导出Zzz"), null);
});

test("工具接进了注册表、意图映射和取证闸", () => {
  assert.match(SRC, /name: "package_source"/, "工具没注册");
  assert.match(SRC, /case "package_source": return \{ type: "package_source"/, "意图没映射");
  assert.match(SRC, /\} else if \(call\.type === "package_source"\)/, "没有执行分支");
  // 光有工具没用——仓库里已经吃过一次亏：context7 装好了、名录也报了，模型照旧不调。
  // 所以"什么时候用它"必须同时写在三处。
  assert.match(SRC, /第三方库的真实签名→package_source/, "工具直觉里没引导到它");
  assert.doesNotMatch(SRC, /陌生库初探→semantic_search/,
    "还在把陌生库引向 semantic_search——那个工具的索引跳过 node_modules，必然空手而归");
  assert.match(SRC, /"package_search", "package_source", "github_repo"/,
    "没进官方证据表，取证 gate 不会装载它");

  /*
   * 加一个工具要同步的地方不止注册表——这五处漏一个，工具就等于半死不活：
   * 少 TOOL_METADATA → 进不了能力名录，模型根本叫不出它的名字；
   * 少网关那份 → 正式构建走网关时描述漂移；
   * 少官网那份 → 工具画廊和真目录对不上。
   * 这一条把它们一次钉齐（本次就是被这几条守卫逐个抓出来的）。
   */
  const guides = readFileSync(join(ROOT, "src", "tool-guides.js"), "utf8");
  assert.match(guides, /package_source: \{ category: 'research'/, "缺 TOOL_METADATA");
  for (const [label, rel] of [
    ["网关", join(ROOT, "..", "server", "prompts", "tools.json")],
    ["官网", join(ROOT, "website", "public", "tools.json")],
  ]) {
    // 两份目录的形状不一样：网关那份是裸数组，官网那份是 { tools: [...] } 带元信息。
    const doc = JSON.parse(readFileSync(rel, "utf8"));
    const list = Array.isArray(doc) ? doc : (doc.tools || []);
    assert.ok(list.length > 100, `${label}那份目录读出来是空的`);
    assert.ok(
      list.some((t) => (t?.function?.name || t?.name) === "package_source"),
      `${label}那份工具目录里没有 package_source`,
    );
  }
});

test("read_file 读不到 node_modules 时不再塞假事实", () => {
  // 那句"可能还没装依赖（先跑 npm install）"是**必然**出现的：它依赖的查找函数
  // 忽略表第一项就是 node_modules，永远返回空。依赖装得好好的，却在最容易幻觉的
  // 那一秒告诉模型"没装"。
  assert.doesNotMatch(SRC, /node_modules 内找不到 \$\{_base\}。可能还没装依赖/,
    "那句假事实又回来了");
  assert.match(SRC, /搜索层刻意跳过 node_modules，所以这不代表没装/, "没说清为什么搜不到");
  assert.match(SRC, /package_source\(package="包名"\)/, "没指向真正能查的那个工具");
});

// ── lsp_hover / lsp_definition 回正文 ────────────────────────────────────────
//
// 跳转的全部意义就是拿到真实签名。只回 `path:line` 的话，模型还得再花一次 read_file
// 才知道那儿写了什么——而在"别磨蹭"的压力下它多半不再读，直接按记忆往下写。

test("lsp_definition 连正文一起回，引用则不带", () => {
  const at = RAW_SRC.indexOf("const rels = uniq.map(");
  assert.ok(at > 0, "找不到定义结果的拼装处");
  const block = SRC.slice(at, at + 1600);
  assert.match(block, /if \(call\.op !== "references"\)/,
    "没有把引用排除在外——引用动辄几十处，每处贴 20 行会把上下文冲垮");
  assert.match(block, /await backend\.readTextFile\(l\.path\)/, "没有真的去读正文");
  assert.match(block, /uniq\.slice\(0, 2\)/, "没有限制条数");
  assert.match(SRC, /\$\{rels\.join\("\\n"\)\}\$\{bodies\}/, "正文没有拼进返回值");
});

test("lsp_hover 从语言服务一直接到工具表", () => {
  const client = readFileSync(join(ROOT, "src", "lsp-client.js"), "utf8");
  // hover 在编辑器侧早就跑着，智能体侧一直没有入口——最省 token 的签名真相源反而没开。
  assert.match(client, /async agentHover\(path, line, character\) \{/, "管理器上没有 agentHover");
  assert.match(client, /ctx\.client\.supports\("hover"\)/, "没检查语言服务支持不支持");
  // hover 的返回可能是字符串 / MarkedString / MarkupContent，三种都要认，否则大半语言拿到空。
  assert.match(client, /if \(Array\.isArray\(node\)\)/, "没处理数组形态的 hover 内容");
  assert.match(client, /typeof node\.value === "string"/, "没处理 MarkupContent 形态");

  assert.match(SRC, /name: "lsp_hover"/, "工具没注册");
  assert.match(SRC, /case "lsp_hover": return \{ type: "lsp", op: "hover"/, "意图没映射");
  assert.match(SRC, /if \(call\.op === "hover"\) \{/, "执行分支没接");
  assert.match(SRC, /lspManager\.agentHover \? lspManager\.agentHover\(fp, line, character\)/, "没调到管理器");
  // 拿不到时必须说清"这不代表符号不存在"，并指向真正能查的那条路——
  // 否则模型会把"没有悬停信息"理解成"没有这个符号"，然后开始编。
  assert.match(SRC, /\*\*这不代表这个符号不存在\*\*/, "空结果的措辞会误导模型");
  assert.match(SRC, /package_source\(package="包名", symbol=/, "空结果没指向 package_source");
  // 只读工具，子智能体也该能用。
  assert.match(SRC, /"lsp_symbols", "lsp_hover", "lsp_definition"/, "子智能体的只读工具表里没有它");
  // 五个接线点里最容易漏的两个。
  const guides = readFileSync(join(ROOT, "src", "tool-guides.js"), "utf8");
  assert.match(guides, /lsp_hover: \{ category: 'search'/, "缺 TOOL_METADATA（或分类不在展示名表里）");
  const gateway = JSON.parse(readFileSync(join(ROOT, "..", "server", "prompts", "tools.json"), "utf8"));
  assert.ok(gateway.some((t) => t?.function?.name === "lsp_hover"), "网关目录里没有 lsp_hover");
});

test("已有的公共代码搜索要说清它是干什么的", () => {
  /*
   * `sourcegraph` 一直躺在 developer_community_search 的 sources 枚举里，后端打的就是
   * 公共代码搜索——但描述里一个字都没说它是干什么的，整个工具被描述成"查踩坑/技术选型/
   * 社区讨论"。于是这个能力等于不存在。
   *
   * 它治的是 package_source 治不了的那一层：**签名对了，但用法不对**——参数顺序、
   * 必需的初始化步骤、真实的调用惯例。文档常常不写，一千个仓库的实际用法里全都有。
   */
  // 目录搬进模块之后从**数据结构**取，别再 indexOf 切窗口：main.js 里还有一处
  // 同名的意图提示（`{ name: "developer_community_search", args: ... }`），
  // indexOf 先撞上它，5000 字窗口就完全落空——判据会静默失效。
  const tool = [...baseTools(), ...readonlyExternalTools(), ...writeTools()]
    .find((t) => t?.function?.name === "developer_community_search");
  assert.ok(tool, "找不到这个工具");
  const block = JSON.stringify(tool);
  assert.match(block, /`sourcegraph` is public CODE search across many repositories/,
    "描述里没说清 sourcegraph 是代码搜索");
  assert.match(SRC, /签名对了但不确定怎么用→developer_community_search/,
    "工具直觉里没有引导到它");
  // 这句活在一条**双引号**字符串里，内部再出现裸的 " 会把它提前截断（踩过一次，
  // 整个 main.js 语法都断了）。
  const hintAt = RAW_SRC.indexOf("签名对了但不确定怎么用");
  const hintLine = SRC.slice(RAW_SRC.lastIndexOf("\n", hintAt) + 1, RAW_SRC.indexOf("\n", hintAt));
  assert.doesNotMatch(hintLine, /sources=\["/, "又在双引号字符串里用了双引号");
});

// ── TS 的类型兜底：垫片不能盖住真类型 ────────────────────────────────────────
//
// 这条防线是唯一**检出**而非降低概率的机制：参数写错、方法不存在，tsc 当场红。
// 而它以前是断的——垫片无条件把每个用到的名字声明成 any，ambient module declaration
// 又会盖过 node_modules 的正常解析，于是第三方符号的实际类型永远是 any，诊断必然绿。

test("垫片只补真类型没覆盖到的名字", () => {
  const makeShim = new Function(extractShim() + ";return _makeInstalledPackageShim;")();
  const named = new Set(["defineConfig", "createServer"]);

  // 真类型什么都没读到 → 行为和从前一样（宁可多一个 any，也不要假红）。
  const full = makeShim("vite", { named, hasDefault: false }, null);
  assert.match(full, /export const defineConfig: any;/);
  assert.match(full, /export const createServer: any;/);

  // 真类型覆盖了一个 → 只补另一个。补了被覆盖的那个就会把真类型盖掉。
  const partial = makeShim("vite", { named, hasDefault: false }, new Set(["defineConfig"]));
  assert.doesNotMatch(partial, /defineConfig/, "把真类型已有的名字也垫了，等于盖掉它");
  assert.match(partial, /export const createServer: any;/);

  // 注入点传进来的是 _extractExportNames 的返回值——数组，不是 Set。写死 instanceof
  // 判断会让它默默退化成空集，过滤一个名字都不生效，而全 Set 的用例照样绿。
  assert.doesNotMatch(
    makeShim("vite", { named, hasDefault: false }, ["defineConfig"]),
    /defineConfig/,
    "数组形式的已知导出没被认出来，过滤等于没做",
  );

  // 全覆盖 → 返回空串，调用方据此整份跳过（加了照样遮蔽真解析）。
  assert.equal(makeShim("vite", { named, hasDefault: false }, new Set(["defineConfig", "createServer"])), "");
  // 默认导出同理。
  assert.equal(makeShim("x", { named: new Set(), hasDefault: true }, new Set(["default"])), "");
  assert.match(makeShim("x", { named: new Set(), hasDefault: true }, new Set()), /export default _default;/);

  function extractShim() {
    const at = RAW_SRC.indexOf("function _makeInstalledPackageShim(");
    return SRC.slice(at, RAW_SRC.indexOf("\n}\n", at) + 2);
  }
});

test("注入点：全覆盖时不加那份 ambient 声明", () => {
  const at = RAW_SRC.indexOf("const shim = _makeInstalledPackageShim(specifier, details");
  assert.ok(at > 0, "找不到垫片注入处");
  const block = SRC.slice(at, at + 500);
  assert.match(block, /realType\?\.exports \|\| null/, "没有把真类型的导出集传进去");
  assert.match(block, /if \(shim\) \{/, "空串时仍然会 addExtraLib——那还是会遮蔽真解析");
});

test("类型入口要跟随 re-export，否则导出集永远是空的", async () => {
  // 现代库的入口几乎全是再导出，入口文件里一个具体符号都没有。不跟的话
  // knownExports 恒空，上面那条"只补没覆盖的"就等于没做。
  const files = {
    "/p/node_modules/vite/package.json": JSON.stringify({ types: "./dist/index.d.ts" }),
    "/p/node_modules/vite/dist/index.d.ts":
      'export * from "./config";\nexport { createServer } from "./server";\n',
    "/p/node_modules/vite/dist/config.d.ts": "export declare function defineConfig(c: unknown): unknown;\n",
    "/p/node_modules/vite/dist/server.d.ts": "export declare function createServer(): unknown;\n",
  };
  const backend = { readTextFile: async (p) => { if (!(p in files)) throw new Error("ENOENT"); return files[p]; } };
  // TS_TYPE_REEXPORT_MAX_FILES 也得注进来：漏了它函数会抛 ReferenceError，而
  // _readPackageTypeEntry 的外层 try/catch 会把它吞成"没读到类型入口"——症状和真失败
  // 一模一样，最容易查错方向。
  const api = new Function("backend", "TS_PACKAGE_TYPE_MAX_BYTES", "TS_TYPE_REEXPORT_MAX_FILES",
    topLevelFn("_typeEntryCandidatesFromPackageJson") + topLevelFn("_followTypeReexports")
    + topLevelFn("_readPackageTypeEntry") + topLevelFn("_extractExportNames")
    + ";return _readPackageTypeEntry;")(backend, 256 * 1024, 8);

  const entry = await api("/p", "vite");
  assert.ok(entry, "没读到类型入口");
  assert.ok(entry.exports.includes("defineConfig"), `export * 没跟上：${entry.exports}`);
  assert.ok(entry.exports.includes("createServer"), `具名再导出没跟上：${entry.exports}`);
});
