// 构建期第二道剥离：把 src/tool-guides.js 里**面向模型的散文**从发布包里拿掉。
//
// 背景：build/strip-tool-ip.mjs 只覆盖 main.js 的 _buildAgentToolSchemas 和
// src/agent/tool-catalog.js 的三段目录字面量（实跑 31 + 143 条）。tool-guides.js
// **一条都没剥**——58 KB、143 个工具的「什么时候用、和谁比、怎么调」原样进包。
// 实测混淆产物里它的标记出现 95(表)+42(明文) 次，是包里体量最大的一块工具 IP。
//
// 剥什么：usage_note / example_call / use_cases / triggers —— 全是人写的模型指导语。
// 不剥什么：category / priority —— 枚举与序数，没有 IP，且 CATEGORY_LABELS 的分组
// 还要靠 category 才不塌。名字（键）当然留着：客户端要按名路由。
//
// 作用域：只在 `const TOOL_METADATA = Object.freeze({` 到与之配对的 `});` 之间。
// 文件后半段的函数（enrichedCatalogLine / toolCapabilityIndex / _capabilityNote）
// 里也有 'use_cases' 这类**标识符**，剥到那里会把代码改坏。

// 两个承载散文的字面量。**必须都在这里**：TOOL_EXAMPLES 是 2026 年才加的第二张表
// （109 条手写调用示例，6.7 KB），它在 TOOL_METADATA 的区间之外，只认第一个的话它
// 原样进包——实测混淆产物里 "official Vite migration guide" / "Inspect auth and return
// file:line evidence." 都在字符串表里，而**明文 grep 是 0**。
export const STRIP_REGIONS = ["TOOL_METADATA", "TOOL_EXAMPLES"];

// 显式豁免清单：**留在包里**的顶层字面量，每条都要写清为什么它不是 IP。
// 这不是装饰。test/tool-guides-drift.test.mjs 会枚举文件里所有顶层数据字面量，
// 凡是既不在 STRIP_REGIONS 也不在这里的，当场红——「有人新写了一张表」这件事
// 就是这样被抓住的，而不是等到发版之后。
export const ALLOWED_REGIONS = {
  FIELD_VALUES:
    "泛用占位参数值（'src/main.ts' / 'npm test' / 'https://example.com'）。按**参数名**取样例值，"
    + "和任何具体工具无关，换个产品照抄也没有价值。留着是因为 compactToolExampleArgs 要用它"
    + "把 schema 渲染成一行示例调用。",
  CATEGORY_LABELS:
    "21 个分类展示名（'规划与起步' / '符号与语义检索'）。名录一旦整体移到网关，这张表就是死重，"
    + "到那时连同 toolCapabilityIndex 一起删；在那之前它只是分组表头，不含「什么时候用哪个工具」。",
  _SELF_EVIDENT:
    "11 个「名字已经说完了」的工具名集合。里面只有工具名，没有一个字的说明。",
};

const REGION_OPENS = STRIP_REGIONS.map((n) => `const ${n} = Object.freeze({`);

// 每个键剥成什么：字符串键剥成空串，数组键剥成空数组。
const PROSE_KEYS = { usage_note: "str", example_call: "str", use_cases: "arr", triggers: "arr" };

function regionOf(source, marker) {
  const at = source.indexOf(marker);
  if (at < 0) return null;
  // 从 `Object.freeze({` 的 `{` 起做配对扫描（跳过字符串里的括号）。
  const open = source.indexOf("{", at + marker.length - 2);
  let depth = 0, quote = null;
  for (let i = open; i < source.length; i++) {
    const c = source[i];
    if (quote) { if (c === "\\") { i++; continue; } if (c === quote) quote = null; continue; }
    if (c === '"' || c === "'" || c === "`") { quote = c; continue; }
    if (c === "{") depth++;
    else if (c === "}" && --depth === 0) return [at, i];
  }
  return null;
}

// 在一行里把 `key: <值>` 的值换掉。逐字符走，尊重转义，所以值里的引号/括号骗不了它。
function blankKeyInLine(line, key, kind) {
  const KEY = key + ":";
  // n = 剥掉的**键**个数；segs = 剥掉的**字面量**个数（拼接串一个键可能有好几段）。
  // 两个数分开报，否则 `usage_note: 'a'+'b'` 会让「键计数」多出 1，和朴素计数器对不上。
  let out = "", i = 0, n = 0, segs = 0;
  while (i < line.length) {
    const at = line.indexOf(KEY, i);
    if (at === -1) { out += line.slice(i); break; }
    // 必须是**键**，不能是 `meta.usage_note:` 之类的片段或更长标识符的尾巴。
    const prev = at > 0 ? line[at - 1] : "";
    if (/[\w$.]/.test(prev)) { out += line.slice(i, at + KEY.length); i = at + KEY.length; continue; }
    out += line.slice(i, at + KEY.length);
    let j = at + KEY.length;
    while (j < line.length && (line[j] === " " || line[j] === "\t")) { out += line[j]; j++; }
    if (kind === "str") {
      const q = line[j];
      if (q !== '"' && q !== "'" && q !== "`") { i = j; continue; }
      let k = j + 1;
      while (k < line.length) {
        if (line[k] === "\\") { k += 2; continue; }
        if (line[k] === q) break;
        k++;
      }
      if (k >= line.length) { i = j; continue; }          // 跨行字符串：不动，交给残留检查去红
      out += q + q; i = k + 1; n++; segs++;
      // **拼接串**：`usage_note: '前半'+'后半'`。只剥第一段的话后半截原样进包，
      // 而「第一个值已经是空串」会让残留检查误判为干净——本仓实测有 4 条正是这个形状
      // （probe_env / ui_extract / view_image / git_show），漏掉的全是【vs 替代】那半句。
      // build/strip-tool-ip.mjs 的同名实现有同一个洞，今天只是碰巧没有拼接串。
      for (;;) {
        let p = i;
        while (p < line.length && (line[p] === " " || line[p] === "\t")) p++;
        if (line[p] !== "+") break;
        p++;
        while (p < line.length && (line[p] === " " || line[p] === "\t")) p++;
        const q2 = line[p];
        if (q2 !== '"' && q2 !== "'" && q2 !== "`") break;
        let e = p + 1;
        while (e < line.length) {
          if (line[e] === "\\") { e += 2; continue; }
          if (line[e] === q2) break;
          e++;
        }
        if (e >= line.length) break;
        i = e + 1; segs++;                                // 整段 `+ '…'` 丢掉，不往 out 里写
      }
    } else {
      if (line[j] !== "[") { i = j; continue; }
      let k = j + 1, d = 1, q2 = null;
      while (k < line.length && d > 0) {
        const c = line[k];
        if (q2) { if (c === "\\") { k += 2; continue; } if (c === q2) q2 = null; k++; continue; }
        if (c === '"' || c === "'" || c === "`") { q2 = c; k++; continue; }
        if (c === "[") d++;
        else if (c === "]") d--;
        k++;
      }
      if (d !== 0) { i = j; continue; }                   // 跨行数组：同上
      out += "[]"; i = k; n++; segs++;
    }
  }
  return { line: out, n, segs };
}

/**
 * @returns {{code, changed, found, expected, residual}}
 *   changed  — 剥掉的**键**个数（逐字符走出来的）
 *   expected — 用一个**完全独立**的计数器数出来的应剥键数（朴素 split）
 *   literals — 剥掉的**字面量**个数。>= changed：`usage_note: 'a'+'b'` 是一个键两段。
 *   residual — 剥完之后区间里还剩几个非空的散文值（应当恒为 0）
 * 四个数互相独立，任何一个对不上都说明这一步坏了。见 vite.config.js 里的守卫。
 */
export function stripToolGuides(source) {
  const regions = [];
  for (const marker of REGION_OPENS) {
    const r = regionOf(source, marker);
    if (r) regions.push({ marker, span: r });
  }
  // 找不到**全部**区间就算 found=false。少认一个 = 那一整张表原样进包。
  if (regions.length !== REGION_OPENS.length) {
    return { code: source, changed: 0, found: false, expected: 0, residual: 0,
             missing: REGION_OPENS.filter((m) => !regions.some((r) => r.marker === m)) };
  }
  regions.sort((a, b) => a.span[0] - b.span[0]);

  const newline = source.includes("\r\n") ? "\r\n" : "\n";
  let out = "", cursor = 0, changed = 0, expected = 0, literals = 0;
  const bodies = [];
  for (const { span: [start, end] } of regions) {
    out += source.slice(cursor, start);
    const body = source.slice(start, end + 1);

    // 独立计数器 A：朴素 split，不共用下面那个逐字符实现。两者对不上 = 有一方漏了。
    for (const key of Object.keys(PROSE_KEYS)) {
      expected += body.split(new RegExp(`(?<![\\w$.])${key}:`)).length - 1;
    }
    // TOOL_EXAMPLES 是 `name: {...}` 形状，没有 PROSE_KEYS 的键；它整张表都要空掉，
    // 逐条剥成 {}。这一条单独计数（下面的 EXAMPLE_ENTRY 分支）。
    const isExamples = body.startsWith("const TOOL_EXAMPLES");
    let lines = body.split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      if (isExamples) {
        const r = blankExampleEntry(lines[i]);
        if (r.n) { lines[i] = r.line; changed += r.n; literals += r.n; expected += r.n; }
        continue;
      }
      for (const [key, kind] of Object.entries(PROSE_KEYS)) {
        if (!lines[i].includes(key + ":")) continue;
        const r = blankKeyInLine(lines[i], key, kind);
        if (r.n) { lines[i] = r.line; changed += r.n; literals += r.segs; }
      }
    }
    const sb = lines.join(newline);
    bodies.push({ isExamples, text: sb });
    out += sb;
    cursor = end + 1;
  }
  out += source.slice(cursor);

  // 独立计数器 B：剥完之后回头数「还剩多少个非空散文值」。这条不看剥了多少，只看剩没剩。
  // 它抓的是「剥了一半」——那正是 changed 阈值抓不到的形状。**逐区间用各自的判据**：
  // TOOL_METADATA 里 `name: {` 是每条工具的正常开头（category/priority 留着），
  // 拿 TOOL_EXAMPLES 的判据去量它会得到 99 个假残留。
  let residual = 0;
  for (const { isExamples, text } of bodies) {
    if (isExamples) {
      for (const m of text.matchAll(/^\s{2}([a-z_][\w]*):\s*\{(.)/gm)) if (m[2] !== "}") residual++;
      for (const m of text.matchAll(/^\s{2}([a-z_][\w]*):\s*(['"`])/gm)) residual++;   // 字符串形状的示例
      continue;
    }
    // **不看第一个字符，看整个值**。只看第一个字符的写法会把 `usage_note: ''+'后半句'`
    // 判成干净（首值确实是空串），而后半句原样进包 —— 那正是「断言真实却守错了东西」。
    for (const m of text.matchAll(/(?<![\w$.])(usage_note|example_call|use_cases|triggers):/g)) {
      const rest = text.slice(m.index + m[0].length);
      // 值的范围：到下一个「, <标识符>:」或该条目的 ` }` 为止。
      const stop = rest.search(/,\s*[A-Za-z_$][\w$]*\s*:|\s\}/);
      const value = (stop === -1 ? rest.slice(0, 200) : rest.slice(0, stop)).trim();
      // 干净的值只能是这三种形状之一（含拼接出来的空串链）。
      if (/^(?:''|""|``|\[\])(?:\s*\+\s*(?:''|""|``))*$/.test(value)) continue;
      residual++;
    }
  }
  return { code: out, changed, literals, found: true, expected, residual, missing: [] };
}

// TOOL_EXAMPLES 的一行： `  read_file: { path: "src/main.js" },` → `  read_file: {},`
function blankExampleEntry(line) {
  const m = /^(\s{2}[a-z_][\w]*:\s*)\{/.exec(line);
  if (!m) return { line, n: 0 };
  const open = line.indexOf("{", m[1].length - 1);
  let d = 0, q = null, k = open;
  for (; k < line.length; k++) {
    const c = line[k];
    if (q) { if (c === "\\") { k++; continue; } if (c === q) q = null; continue; }
    if (c === '"' || c === "'" || c === "`") { q = c; continue; }
    if (c === "{") d++;
    else if (c === "}" && --d === 0) break;
  }
  if (d !== 0 || k >= line.length) return { line, n: 0 };   // 跨行：不动，交给 residual 去红
  if (line.slice(open, k + 1) === "{}") return { line, n: 0 };
  return { line: line.slice(0, open) + "{}" + line.slice(k + 1), n: 1 };
}
