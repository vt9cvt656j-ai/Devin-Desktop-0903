#!/usr/bin/env node
/**
 * 从后台的图标目录生成 IDE 的品牌 sprite。
 *
 *   node scripts/gen-brand-sprite.mjs          # 重新生成
 *   node scripts/gen-brand-sprite.mjs --check  # 只检查有没有漂（CI/测试用）
 *
 * # 为什么要有这个脚本
 *
 * `src/brand-sprite.js` 头上一直写着「由脚本从 VendorMark.tsx 生成」，而**那个脚本
 * 不在仓库里**。于是「在后台加一家图标」这件事，IDE 那边不会自动跟上，也不会报错 ——
 * 表现只是某个模型没有图标，而人会去怀疑厂商判定（那里通常是好的）。
 *
 * # 转换做了什么
 *
 * 后台那份是 JSX，IDE 这份是一个 SVG 字符串。差别只有两处：
 *   · 去掉 `<>…</>` 片段包装；
 *   · React 的驼峰属性还原成 SVG 的连字符写法（fillRule → fill-rule …）。
 *
 * 属性映射表是**白名单**，不是「凡是驼峰就转」：后者会把 `viewBox`、`gradientUnits`
 * 这类**本来就该是驼峰**的 SVG 属性一起改坏，而改坏之后图还是能画出来一部分，
 * 最难发现。
 */
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SRC = join(ROOT, "../server/admin-ui/src/components/VendorMark.tsx");
const OUT = join(ROOT, "src/brand-sprite.js");

/** React 驼峰 → SVG 连字符。**白名单**，见文件头。 */
const ATTR = {
  fillRule: "fill-rule",
  clipRule: "clip-rule",
  fillOpacity: "fill-opacity",
  strokeWidth: "stroke-width",
  strokeLinecap: "stroke-linecap",
  strokeLinejoin: "stroke-linejoin",
  strokeOpacity: "stroke-opacity",
  stopColor: "stop-color",
  stopOpacity: "stop-opacity",
  clipPath: "clip-path",
  xlinkHref: "xlink:href",
};

function parseMarks(tsx) {
  const start = tsx.indexOf("const MARKS: Record<string, Mark> = {");
  if (start < 0) throw new Error("VendorMark.tsx 里找不到 MARKS —— 它的形状变了");
  const body = tsx.slice(start);
  const out = [];
  // 一条 = `  key: {\n    name: "…",\n    mono: bool,\n    art: <>…</>,\n  },`
  const re = /^ {2}([a-z0-9]+): \{\n {4}name: "([^"]*)",\n {4}mono: (true|false),\n {4}art: <>([\s\S]*?)<\/>,\n {2}\},$/gm;
  let m;
  while ((m = re.exec(body))) {
    out.push({ key: m[1], name: m[2], mono: m[3] === "true", art: m[4] });
  }
  return out;
}

function toSvg(art) {
  let s = art;
  for (const [from, to] of Object.entries(ATTR)) {
    s = s.replaceAll(`${from}=`, `${to}=`);
  }
  // `style={{ mixBlendMode: "screen" }}` → `style="mix-blend-mode:screen"`。
  // 目录里只有 luma 用了内联样式，但这一支得留着 —— 下次再有一个，
  // 没有这一支的话脚本会直接抛，而抛在生成阶段总比生成出一个画不出来的图强。
  s = s.replace(/style=\{\{([^}]*)\}\}/g, (_, decls) => {
    const css = decls
      .split(",")
      .map((d) => d.trim())
      .filter(Boolean)
      .map((d) => {
        const [k, v] = d.split(":").map((x) => x.trim());
        const prop = k.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`);
        return `${prop}:${v.replace(/^["']|["']$/g, "")}`;
      })
      .join(";");
    return `style="${css}"`;
  });
  // 到这里还有花括号 = 目录里出现了这个脚本不认识的 JSX 表达式。
  // **抛出来，不要静默留着** —— 留着会生成一个浏览器画不出来的属性，
  // 而症状只是「这一家的图标是空的」。
  if (s.includes("{")) throw new Error(`图形里有认不出的 JSX 表达式：${s.slice(0, 120)}`);
  return s;
}

const tsx = readFileSync(SRC, "utf8");
const marks = parseMarks(tsx);
if (marks.length < 100) {
  // 解析规则和目录排版对不上时会**静默少认**，而少认的结果是「悄悄删掉一批图标」。
  throw new Error(`只解析出 ${marks.length} 个图标 —— 解析规则和 VendorMark.tsx 的排版对不上了`);
}

const sprite = marks
  .map((m) => `<symbol id="i-brand-${m.key}" viewBox="0 0 24 24">${toSvg(m.art)}</symbol>`)
  .join("");
const mono = marks.filter((m) => m.mono).map((m) => m.key).sort();
const all = marks.map((m) => m.key).sort();

// **按行替换，不要用 String.replace 拼整个文件。**
//
// 第一版是 `header.replace(/const SPRITE = "[\s\S]*?";\n/, …)`，两个坑一次踩齐：
// 替换串里的 `$` 在 String.replace 里有特殊含义，而那个非贪婪正则会停在文件里
// 第一个 `";\n`。结果是把 207KB 的 sprite 写成 2.6KB、149 个图标变成 0 个 ——
// **而且脚本打印的是「已生成 149 个图标」**。
//
// 这三个常量各自占一整行，逐行换掉最省事，也天然保住了文件里其它手写的东西
// （比如 installBrandSprite 里那个 display:none 的修复）。
const cur = readFileSync(OUT, "utf8");
const lines = cur.split("\n");
const put = (prefix, text) => {
  const at = lines.findIndex((l) => l.startsWith(prefix));
  if (at < 0) throw new Error(`brand-sprite.js 里找不到 \`${prefix}\` —— 它的形状变了`);
  lines[at] = text;
};
put("const SPRITE = ", `const SPRITE = ${JSON.stringify(sprite)};`);
// 数组按 Prettier 的风格排（元素间 `", "`）。不这么写的话，每次重新生成都会
// 制造一次纯格式的改动，而那种噪音会让人不敢跑这个脚本。
const arr = (xs) => JSON.stringify(xs).replaceAll('","', '", "');
put("export const MONO_BRANDS = ", `export const MONO_BRANDS = new Set(${arr(mono)});`);
put("export const BRANDS = ", `export const BRANDS = new Set(${arr(all)});`);
const file = lines.join("\n");

// 自检：写出去之前确认没把文件写小。上面那个 bug 就是靠这条才该被当场拦住。
const symbols = (file.match(/<symbol id=/g) || []).length;
if (symbols !== all.length) {
  throw new Error(`生成结果里只有 ${symbols} 个 symbol，应当是 ${all.length} 个 —— 不写盘`);
}
if (file.length < cur.length * 0.5) {
  throw new Error(`生成结果只有 ${file.length} 字节、原来 ${cur.length} —— 像是写坏了，不写盘`);
}

if (process.argv.includes("--check")) {
  const norm = (t) => t.replace(/\s+/g, " ").trim();
  if (norm(cur) !== norm(file)) {
    console.error(
      "brand-sprite.js 和后台的图标目录不一致 —— 跑 `node scripts/gen-brand-sprite.mjs` 重新生成",
    );
    process.exit(1);
  }
  console.log(`brand-sprite.js 与后台目录一致（${all.length} 个图标）`);
} else {
  writeFileSync(OUT, file);
  console.log(`已生成 ${all.length} 个图标（其中单色 ${mono.length} 个）→ src/brand-sprite.js`);
}
