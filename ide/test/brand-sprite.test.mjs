import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
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
