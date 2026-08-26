/**
 * 源码文本的结构切分：给一段源码，找出它的符号定义位置和"哪些字节是注释"。
 *
 * 从 main.js 抽出来的第七块。两个函数是一件事的两面——都只吃字符串、只吐结构，
 * 不碰文件系统、不碰 DOM、没有模块级可变状态（AST 实测外部自由变量为零）。
 *
 * splitCodeAndComments 承担着这个仓库的一条硬规矩：**正向源码断言必须跑在剥掉注释
 * 的源码上**。注释不是代码——把一条契约从代码里删掉、只在注释里留一句，assert.match
 * 照样绿，这个仓库已经这样漏过一整组模型可见的工具契约。所以它的正确性不是"锦上添花"。
 */

// Symbol patterns by file extension. Each regex captures the symbol NAME in
// group 1 + the matched kind via the capturing group name (we tag externally).
// Loose by design — false positives are cheaper than missed definitions.
export function symbolPatternsFor(ext) {
  if (ext === "ts" || ext === "tsx" || ext === "js" || ext === "jsx" || ext === "mjs" || ext === "cjs") {
    return [
      [/^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_$][\w$]*)/, "function"],
      [/^\s*(?:export\s+)?class\s+([A-Za-z_$][\w$]*)/, "class"],
      [/^\s*(?:export\s+)?interface\s+([A-Za-z_$][\w$]*)/, "interface"],
      [/^\s*(?:export\s+)?type\s+([A-Za-z_$][\w$]*)/, "type"],
      [/^\s*(?:export\s+)?enum\s+([A-Za-z_$][\w$]*)/, "enum"],
      [/^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::|=\s*(?:async\s*)?\(|=\s*function)/, "const"],
    ];
  }
  if (ext === "py") {
    return [
      [/^\s*def\s+([A-Za-z_][\w]*)/, "function"],
      [/^\s*class\s+([A-Za-z_][\w]*)/, "class"],
      [/^\s*async\s+def\s+([A-Za-z_][\w]*)/, "function"],
    ];
  }
  if (ext === "rs") {
    return [
      [/^\s*(?:pub\s+(?:\(.+?\)\s+)?)?(?:async\s+)?fn\s+([A-Za-z_][\w]*)/, "function"],
      [/^\s*(?:pub\s+(?:\(.+?\)\s+)?)?struct\s+([A-Za-z_][\w]*)/, "struct"],
      [/^\s*(?:pub\s+(?:\(.+?\)\s+)?)?enum\s+([A-Za-z_][\w]*)/, "enum"],
      [/^\s*(?:pub\s+(?:\(.+?\)\s+)?)?trait\s+([A-Za-z_][\w]*)/, "trait"],
      [/^\s*impl(?:<[^>]+>)?\s+(?:[\w:<>, ]+\s+for\s+)?([A-Za-z_][\w]*)/, "impl"],
    ];
  }
  if (ext === "go") {
    return [
      [/^\s*func\s+(?:\([^)]+\)\s+)?([A-Za-z_][\w]*)/, "function"],
      [/^\s*type\s+([A-Za-z_][\w]*)\s+(?:struct|interface)/, "type"],
    ];
  }
  if (ext === "java" || ext === "kt" || ext === "scala") {
    return [
      [/^\s*(?:public|private|protected|internal)?\s*(?:static\s+)?(?:final\s+)?(?:abstract\s+)?class\s+([A-Za-z_][\w]*)/, "class"],
      [/^\s*(?:public|private|protected|internal)?\s*(?:static\s+)?(?:final\s+)?(?:abstract\s+)?interface\s+([A-Za-z_][\w]*)/, "interface"],
      [/^\s*fun\s+([A-Za-z_][\w]*)/, "function"],
      [/^\s*def\s+([A-Za-z_][\w]*)/, "function"],
    ];
  }
  if (ext === "c" || ext === "cc" || ext === "cpp" || ext === "cxx" || ext === "h" || ext === "hpp") {
    return [
      [/^\s*(?:static\s+)?(?:inline\s+)?(?:extern\s+)?[\w:<>*&\s]+\s+([A-Za-z_][\w]*)\s*\([^;]*\)\s*\{?\s*$/, "function"],
      [/^\s*(?:struct|class)\s+([A-Za-z_][\w]*)/, "class"],
    ];
  }
  if (ext === "rb") {
    return [[/^\s*def\s+(?:self\.)?([A-Za-z_][\w]*[?!]?)/, "method"], [/^\s*class\s+([A-Za-z_][\w]*)/, "class"]];
  }
  if (ext === "php") {
    return [[/^\s*function\s+([A-Za-z_][\w]*)/, "function"], [/^\s*class\s+([A-Za-z_][\w]*)/, "class"]];
  }
  if (ext === "sh") return [[/^\s*(?:function\s+)?([A-Za-z_][\w]*)\s*\(\s*\)\s*\{/, "function"]];
  return null;
}

/**
 * 这次**正在写**的代码碰到了哪些「不可信输入 → 危险汇聚点」。
 *
 * 用户原话：「实时的知道到底有没有漏洞那些，有的话写的不要写出漏洞」。
 *
 * 为什么不是把漏洞分类表挂上来：那张表（server/prompts/defect_hunting.txt，9.4KB）
 * **刻意**只在只读审计时挂载，写代码时不挂，理由写在 `add("defects", …)` 上面，而且
 * server/src/prompts.rs 里有一条测试正面钉着 `assemble("2.5:engineering")` 不许含它。
 * 那条契约是对的：写个登录功能不该平白背上整张表。
 *
 * 所以这里走另一条路——不按「这轮是什么任务」挂整表，按**这一次写进去的代码碰到了什么**
 * 递对应的那几条。判据是落盘内容本身（和 _stubDeliveryFindings 同源），不读用户措辞、
 * 不做意图推断；产物挂在 `_mutationAdvice` 上，跟着**这一次写入的工具结果**一起回给模型，
 * 所以是「写的当下」知道，不是收尾时才知道。
 *
 * 每一条都在本仓库自己的 207,820 行真实代码（JS + Rust）上量过误报：
 *   动态求值 / 命令注入 / 不安全反序列化 / 批量赋值 / HTML 汇聚  —— 全部 0
 *   SQL 拼接 —— 0.10/万行，而那 2 处是 `format!("UPDATE {table} SET {col} = $1 …")`，
 *   属于**真发现**（标识符插进 SQL），不是误报。
 * 量出来被砍掉的：裸 `.innerHTML =`（21.22/万行——界面密集的应用里遍地都是）。
 * 假警报比漏报贵：每次写文件都跳一条，模型和用户都会学会略过它。
 */
/*
 * 把每一行拆成「代码部分」和「注释部分」。
 *
 * 存在的理由是用户那句话：「不能光看注释，要代码一起看——注释会欺骗 IDE，
 * 有的还会用旧注释让 IDE 发现不了问题。」两个方向都要防：
 *
 *   ① 注释里的东西不许被当成代码。实测：模型写
 *        // 老写法，已经废弃：
 *        // db.query("SELECT * FROM users WHERE id = " + id)
 *      然后被告知它写了 SQL 注入。既是噪音，也让这套机制显得不可信。
 *   ② 代码里的东西不许被注释盖住。写一句「这里已经参数化了，不用担心注入」，
 *      拼接还是拼接——判据只看代码，注释一个字都不参与，所以这个方向天然成立。
 *
 * 行数和每行长度都保持不变（注释处填空格），行号/列号照旧对得上——这套机制
 * 区别于泛泛提醒的全部所在就是「指到了哪一行」。
 *
 * 注释语法按扩展名分。**默认不认 `#`**：JS 的私有字段 `this.#x` 会被它整段抹掉。
 */
export function splitCodeAndComments(text, path = "") {
  // 两张表放在函数**里面**：这个仓库的测试用 load("<名字>") 把单个函数抠出来跑，
  // 引用一个外部常量就是 ReferenceError（不是断言失败，排查方向完全不同，已经栽过）。
  const HASH_EXT = new Set(["py","rb","sh","bash","zsh","fish","yml","yaml","toml","tf","tfvars","pl","r","rake","gemspec","dockerfile","makefile","mk","conf","ini","env"]);
  const DASH_EXT = new Set(["sql","lua","hs","elm","ada"]);
  const ext = String(path || "").split("/").pop().split(".").pop().toLowerCase();
  const hash = HASH_EXT.has(ext);
  const dash = DASH_EXT.has(ext);
  const slash = !hash && !dash;
  const src = String(text || "");
  const lines = src.split("\n");
  const code = [];
  const comments = [];
  let inBlock = false;     // /* … */   （dash 那档是 --[[ … ]]，这里一并用它表示）
  /*
   * 跨行的字符串定界符（JS 的模板串、Python 的三引号）**必须跨行保持**，不能每行重置。
   *
   * 第一版就是每行重置的，代价很实：main.js 里那些几百行的 HTML 模板串，后续行被当成
   * 代码，串里随便一个 `//`（URL、JSX 注释、CSS 里的 //）就把那一行剩下的全判成注释。
   * 标定时它以「注释里点名了 120 个全仓不存在的下划线标识符」的形式露出来——那些名字
   * （_mpmDeleteConn / _mpmFilterDom …）其实好端端地住在模板串里的 onclick 上。
   */
  let multi = "";          // 跨行未闭合的定界符：反引号 或 三引号
  for (const line of lines) {
    let out = "";
    let note = "";
    let quote = multi;     // 上一行没闭合就接着算在串里
    multi = "";
    let i = 0;
    while (i < line.length) {
      const c = line[i];
      if (inBlock) {
        const end = dash ? line.indexOf("]]", i) : line.indexOf("*/", i);
        if (end < 0) { note += line.slice(i); out += " ".repeat(line.length - i); i = line.length; }
        else { const n = end + 2; note += line.slice(i, end); out += " ".repeat(n - i); i = n; inBlock = false; }
        continue;
      }
      if (quote) {
        out += c;
        if (c === "\\") { if (i + 1 < line.length) { out += line[i + 1]; i += 2; continue; } }
        else if (quote.length === 3) {
          if (line.slice(i, i + 3) === quote) { out += line.slice(i + 1, i + 3); i += 3; quote = ""; continue; }
        } else if (c === quote) quote = "";
        i++;
        continue;
      }
      // Python 的三引号也跨行。先试三引号再试单引号，否则它会被当成一个空串加一个引号。
      const triple = line.slice(i, i + 3);
      if (triple === '"""' || triple === "'''") { quote = triple; out += triple; i += 3; continue; }
      if (c === '"' || c === "'" || c === "`") { quote = c; out += c; i++; continue; }
      /*
       * 正则字面量要先认出来，否则它里面的斜杠会被当成注释起点。
       *
       * 这不是假想：main.js 里有一处 url.replace(正则, "")，那个正则以「反斜杠 斜杠 星号」
       * 结尾——在扫描器眼里就是块注释的起点，于是从那一行起**整片文件都被判成注释**，
       * 7440 行那个 `function _mpmDeleteConn(id) {` 直接进了 comments。所有只看代码的
       * 判据在那之后就全哑了，而且一声不响。标定时是以「注释里点名了 120 个全仓不存在的
       * 标识符」这种奇怪形式露出来的。
       *
       * 「这个 / 是正则还是除号」是 JS 词法的老问题，用标准那条启发式：看前一个有意义的
       * 字符——它是运算符、括号、逗号、分号、关键字结尾，那就是正则的开头；是标识符、
       * 数字、右括号，那就是除号。
       */
      if (slash && c === "/" && line[i + 1] !== "/" && line[i + 1] !== "*") {
        const prev = out.replace(/\s+$/, "").slice(-1);
        const prevWord = /[\w$)\]]/.test(prev);
        if (!prevWord) {
          let k = i + 1;
          let closed = false;
          let inClass = false;
          for (; k < line.length; k++) {
            const d = line[k];
            if (d === "\\") { k++; continue; }
            if (inClass) { if (d === "]") inClass = false; continue; }
            if (d === "[") { inClass = true; continue; }
            if (d === "/") { closed = true; break; }
          }
          if (closed) {
            // 连同结尾的标志位（g/i/m/s/u/y）一起原样带过去。
            let end = k + 1;
            while (end < line.length && /[gimsuyd]/.test(line[end])) end++;
            out += line.slice(i, end);
            i = end;
            continue;
          }
        }
      }
      if (slash && c === "/" && line[i + 1] === "/") { note += line.slice(i + 2); out += " ".repeat(line.length - i); break; }
      if (slash && c === "/" && line[i + 1] === "*") { inBlock = true; i += 2; out += "  "; continue; }
      if (dash && c === "-" && line[i + 1] === "-") {
        if (line[i + 2] === "[" && line[i + 3] === "[") { inBlock = true; i += 4; out += "    "; continue; }
        note += line.slice(i + 2); out += " ".repeat(line.length - i); break;
      }
      if (hash && c === "#") { note += line.slice(i + 1); out += " ".repeat(line.length - i); break; }
      out += c;
      i++;
    }
    // 只有跨行的定界符能带到下一行；普通引号在行尾未闭合是语法错误，不顺延。
    if (quote === "`" || quote.length === 3) multi = quote;
    code.push(out);
    comments.push(note);
  }
  return { code, comments };
}
