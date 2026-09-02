import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const sprite = readFileSync(join(ROOT, "src/brand-sprite.js"), "utf8");
const html = readFileSync(join(ROOT, "index.html"), "utf8");
const main = readFileSync(join(ROOT, "src/main.js"), "utf8");

const spriteIds = new Set([...sprite.matchAll(/<symbol id=\\"(i-brand-[a-z0-9]+)\\"/g)].map((m) => m[1]));
const htmlIds = new Set([...html.matchAll(/<symbol id="(i-brand-[a-z0-9]+)"/g)].map((m) => m[1]));

/**
 * 同一个 symbol id 在文档里只能有一份。
 *
 * 这一条是实测踩出来的，不是预防性的：index.html 原本就定义了 i-brand-anthropic 等十个
 * 单色字形，新 sprite 用了同样的 id。SVG 的 `<use href="#x">` 取的是文档里**第一个**
 * 匹配的元素，而 index.html 比脚本先解析 —— 于是新的真 logo 被旧的假图标静默盖掉，
 * 症状是「图标换了但界面一点没变」，从代码上完全看不出问题。
 */
test("品牌 symbol 的 id 不能在 index.html 和 sprite 里各定义一份", () => {
  const clash = [...spriteIds].filter((id) => htmlIds.has(id));
  assert.deepEqual(
    clash,
    [],
    `这些 id 定义了两次，先解析的 index.html 会赢，真 logo 不会生效：${clash.join(", ")}`,
  );
});

/** sprite 自己内部也不能重复 —— 重复的话后面那个永远画不出来。 */
test("sprite 内部没有重复的 symbol id", () => {
  const all = [...sprite.matchAll(/<symbol id=\\"(i-brand-[a-z0-9]+)\\"/g)].map((m) => m[1]);
  const dupes = all.filter((id, i) => all.indexOf(id) !== i);
  assert.deepEqual([...new Set(dupes)], []);
});

/**
 * `brandOf` 引用的每个符号都得真的存在。
 *
 * 引用一个不存在的 symbol 不会报错，只会画出一块空白 —— 而这正是把符号改名
 * （i-brand-glm → i-brand-zhipu）时最容易漏掉的那一半。
 */
test("brandOf 里引用的品牌符号都在 sprite 里", () => {
  const start = main.indexOf("function brandOf(");
  assert.ok(start > 0, "brandOf 改名了");
  const body = main.slice(start, main.indexOf("\nfunction ", start + 10));
  const referenced = [...body.matchAll(/_brandMark\("([a-z0-9]+)"\)/g)].map((m) => m[1]);
  assert.ok(referenced.length >= 8, `brandOf 里的品牌分支只剩 ${referenced.length} 条，像是被改坏了`);
  const missing = referenced.filter((v) => !spriteIds.has(`i-brand-${v}`));
  assert.deepEqual(missing, [], `这些厂商没有对应的 symbol，会画成一块空白：${missing.join(", ")}`);
});

/** BRAND_SYM 映射到的也必须是存在的厂商标识。 */
test("BRAND_SYM 映射到的厂商都有图标", () => {
  const at = main.indexOf("const BRAND_SYM = {");
  assert.ok(at > 0, "BRAND_SYM 改名了");
  const block = main.slice(at, main.indexOf("};", at));
  const vendors = [...block.matchAll(/:\s*"([a-z0-9]+)"/g)].map((m) => m[1]);
  const missing = [...new Set(vendors)].filter((v) => !spriteIds.has(`i-brand-${v}`));
  assert.deepEqual(missing, [], `BRAND_SYM 指向了没有图标的厂商：${missing.join(", ")}`);
});

/** 启动时必须真的把 sprite 注进去，否则所有 `<use>` 都指向不存在的符号。 */
test("启动时安装了品牌 sprite", () => {
  assert.match(main, /^installBrandSprite\(\);$/m, "没有在模块顶层调用 installBrandSprite()");
  assert.match(main, /import \{[^}]*installBrandSprite[^}]*\} from "\.\/brand-sprite\.js"/);
});

/** 白色主体的标在白底上会整个隐形 —— 生成器该把它们换成单色版，这里兜一道。 */
test("没有纯白填充的图标（白底上会隐形）", () => {
  const whites = [...sprite.matchAll(/fill=\\"(#fff|#ffffff|white)\\"/gi)];
  assert.equal(whites.length, 0, "有图标的主体是白色的，画在浅色底片上会看不见");
});

/**
 * 网关说「还有没试过的出口」时，不能走 15 秒的限流退避。
 *
 * 那条长退避是为「所有上游都在限流」准备的。而多路由下常见的情况是：撞上的两个出口
 * 刚被记了让位，**重发一次就会换一个出口、多半立刻成功** —— 这时等 15 秒是白等。
 *
 * 判据必须走响应头，不能从文案里认：网关的错误措辞改一次，认文案那套就静默失效。
 */
test("网关说还有别的出口时，走快速重发而不是限流长退避", () => {
  const src = readFileSync(join(ROOT, "src/main.js"), "utf8");
  // 用子串比而不是正则：这段代码里有 `?.`、`("`、`"1"` 一堆要转义的东西，
  // 正则写错的表现是**断言恒不成立**，看起来像功能坏了，其实是测试坏了。
  assert.ok(
    src.includes('retryElsewhere: resp.headers?.get?.("x-mide-retry-elsewhere") === "1"'),
    "没有从响应头取这个信号 —— 从错误文案里反解析的话，网关改一次措辞就失效",
  );
  assert.ok(
    src.includes("attemptRetryElsewhere = ev.retryElsewhere === true"),
    "信号没有被重试循环接住",
  );
  // 限流判据里必须排掉它，否则信号收到了也不起作用。
  const at = src.indexOf("const canWaitOutRateLimit");
  assert.ok(at > 0, "限流判据不见了");
  assert.ok(
    src.slice(at, at + 600).includes("!attemptRetryElsewhere"),
    "限流判据没有排掉「还有别的出口」这种情况 —— 会白等 15 秒",
  );
});

/**
 * sprite 容器**不许用 display:none**。
 *
 * WKWebView（Tauri）里 `display:none` 子树中的渐变解析不出来：`<use>` 克隆得到纯色
 * 路径，但 `fill="url(#…)"` 指向的 paint server 拿不到，画出来是空白。
 * 2026-08-26 实测：带渐变的 qwen / minimax 全空，纯色的 anthropic / xiaomimimo 正常。
 *
 * 症状极具误导性 —— 看起来像「这两家没有图标」，人会去翻图标库和厂商判定，
 * 而那两处都是好的。**判据是「带渐变的才空」。**
 *
 * 这条同时守住「sprite 里确实有带渐变的标」：哪天渐变标全没了，这条测试就该被重新审视，
 * 而不是继续挡着一个不再存在的问题。
 */
test("sprite 容器不能用 display:none，否则带渐变的图标画不出来", () => {
  // 沿用这个文件顶上已经读好的 `sprite`，不另开一份读法。
  const install = sprite.slice(sprite.indexOf("export function installBrandSprite"));
  assert.ok(
    !/style\.display\s*=\s*["']none["']/.test(install),
    "sprite 容器又用回 display:none 了 —— 带渐变的厂商标会全部变成空白，而且不报错",
  );
  assert.ok(
    install.includes("position:absolute") && install.includes("overflow:hidden"),
    "没有用留在渲染树里的隐藏方式，渐变仍然解析不出来",
  );
  // 确实有带渐变的标，这条测试才有意义。
  assert.ok(
    /linearGradient|radialGradient/.test(sprite),
    "sprite 里已经没有带渐变的标了 —— 这条测试守的问题可能已不存在，重新审视它",
  );
});

/**
 * IDE 的图标库必须和**后台的图标目录**一致。
 *
 * # 为什么要有这条
 *
 * `brand-sprite.js` 头上一直写着「由脚本从 VendorMark.tsx 生成」，而在 2026-08-26
 * 之前**那个脚本不在仓库里**。于是「在后台加一家图标」这件事，IDE 那边不会自动跟上，
 * 也不会报错 —— 表现只是某个模型没有图标，而人会去怀疑厂商判定（那里通常是好的）。
 *
 * 现在脚本补上了（`scripts/gen-brand-sprite.mjs`），这条守住「跑过了没有」。
 *
 * 后台那份不在这个仓库里时（只克隆 ide/ 的场景）跳过，而不是报一个查不下去的错。
 */
test("IDE 的图标库和后台的图标目录一致（漏跑生成脚本会红）", () => {
  const catalog = join(ROOT, "../server/admin-ui/src/components/VendorMark.tsx");
  if (!existsSync(catalog)) return; // 单独克隆 ide/ 时比不了，跳过。
  const tsx = readFileSync(catalog, "utf8");
  const body = tsx.slice(tsx.indexOf("const MARKS: Record<string, Mark> = {"));
  const inCatalog = new Set(
    [...body.matchAll(/^ {2}([a-z0-9]+): \{\n {4}name: "/gm)].map((m) => m[1]),
  );
  assert.ok(inCatalog.size > 100, `只从后台目录里认出 ${inCatalog.size} 个 —— 解析规则和它的排版对不上了`);

  const missing = [...inCatalog].filter((v) => !spriteIds.has(`i-brand-${v}`));
  assert.deepEqual(
    missing,
    [],
    `后台有这些图标而 IDE 没有：${missing.join("、")} —— 跑 \`node scripts/gen-brand-sprite.mjs\``,
  );
  const extra = [...spriteIds].filter((id) => !inCatalog.has(id.replace("i-brand-", "")));
  assert.deepEqual(
    extra,
    [],
    `IDE 有这些图标而后台已经没有了：${extra.join("、")} —— 同样跑一次生成脚本`,
  );
});
