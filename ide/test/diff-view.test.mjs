import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  buildDiffView,
  diffStat,
  escapeAttr,
  escapeHtml,
  highlightDiffView,
} from "../src/agent/diff-view.js";

test("diffStat counts true line additions and removals", () => {
  assert.deepEqual(diffStat("a\nb\nc\nb", "b\na\nc\nd"), { added: 1, removed: 1 });
  assert.deepEqual(diffStat("", "one\ntwo"), { added: 2, removed: 0 });
  assert.deepEqual(diffStat("same\nsame", "same\nsame"), { added: 0, removed: 0 });
});

test("escape helpers are safe for text nodes and attributes", () => {
  assert.equal(escapeHtml('<script a="b">&</script>'), '&lt;script a="b"&gt;&amp;&lt;/script&gt;');
  assert.equal(escapeAttr('<script a="b">&</script>'), "&lt;script a=&quot;b&quot;&gt;&amp;&lt;/script&gt;");
});

// escapeHtml 原来是 DOM 实现（textContent → innerHTML），改成纯字符串版时有两条 HTML 片段
// 序列化规则必须手工搬过来，否则 313 个调用点会静默改变输出。上面那条断言只用 ASCII，
// 三次 replace 的朴素版本也能过——正是它守不住的地方。
test("escapeHtml carries over the two DOM serialization rules", () => {
  assert.equal(escapeHtml("a\u00a0b"), "a&nbsp;b");      // 不换行空格必须变实体
  assert.equal(escapeHtml("a\u00a0&b"), "a&nbsp;&amp;b"); // 注入的 & 不能被二次转义
  assert.equal(escapeHtml(null), "");                     // 不是字符串 "null"
  assert.equal(escapeHtml(undefined), "");
  assert.equal(escapeAttr(null), "");
});

test("buildDiffView renders new-file additions with escaped code and language", () => {
  const html = buildDiffView("", '<script a="b">&</script>', "src/app.js");

  assert.match(html, /<div class="atc-diff" data-lang="js">/);
  assert.match(html, /atc-diff-row--add/);
  assert.match(html, /data-raw="&lt;script a=&quot;b&quot;&gt;&amp;&lt;\/script&gt;"/);
  assert.match(html, /&lt;script a="b"&gt;&amp;&lt;\/script&gt;/);
});

test("buildDiffView renders deletes, adds, context, and skipped-line markers", () => {
  // 2026-08-30 夹具加长了中间那段：原来两处改动之间只隔 4 行，而上下文是各 2 行，
  // 折叠标记恰好折不出来（2+2 就把 4 行全占了）。原来那版渲染器按同一下标配对、
  // 上下文逻辑也不同，才在这个夹具上吐出 @@ 4 @@。现在中间隔 10 行，标记如实出现。
  const mid = Array.from({ length: 10 }, (_, i) => `mid${i}`);
  const oldText = ["same0", "same1", "old2", ...mid, "old7"].join("\n");
  const newText = ["same0", "same1", "new2", ...mid, "new7"].join("\n");
  const html = buildDiffView(oldText, newText, "src/app.ts");

  assert.match(html, /data-lang="ts"/);
  assert.match(html, /atc-diff-row--ctx/);
  assert.match(html, /atc-diff-row--del/);
  assert.match(html, /atc-diff-row--add/);
  assert.match(html, /@@ 6 unchanged lines @@/);
});

test("diff 视图和它上面那个 +N/-M 徽章必须给出同一个结论", () => {
  // 渲染循环原来按**同一个下标**配对 oldL[i] / newL[i]，于是只要有一行插入，后面全错位。
  // 实测：40 行文件最前面插一行 import → 徽章（diffStat 用行的多重集，算得对）显示 +1，
  // 正下方那块 diff 画出 30 增 30 删。同一次写入，两个数字差 30 倍，用户不知道信哪个。
  const count = (h, c) => (String(h).match(new RegExp(`atc-diff-row--${c}`, "g")) || []).length;
  const base = Array.from({ length: 40 }, (_, i) => `line ${i}`).join("\n");
  for (const [label, oldT, newT] of [
    ["头部插一行", base, "import x from 'y';\n" + base],
    ["尾部插一行", base, base + "\nexport {};"],
    ["中间改一行", base, base.split("\n").map((l, i) => (i === 9 ? "line NINE" : l)).join("\n")],
    ["删掉一行", base, base.split("\n").filter((_, i) => i !== 5).join("\n")],
    ["完全没变", base, base],
    // 交错编辑：中段既有插入又有修改。前后缀裁剪解决不了这种，必须真的跑 LCS——
    // 把预算调成 0 强制走退化路径时，这一条会给出 4 增 3 删（正确答案是 2 增 1 删），
    // 所以它是唯一能把「LCS 真的在跑」和「只靠裁剪蒙对」分开的用例。
    ["交错插入+修改",
      ["a", "b", "c", "d", "e", "f", "g", "h"].join("\n"),
      ["a", "b", "INSERT", "c", "d", "CHANGED", "f", "g", "h"].join("\n")],
  ]) {
    const st = diffStat(oldT, newT);
    const html = buildDiffView(oldT, newT, "a.ts") || "";
    assert.equal(count(html, "add"), st.added, `${label}：视图的新增行数和徽章对不上`);
    assert.equal(count(html, "del"), st.removed, `${label}：视图的删除行数和徽章对不上`);
  }
});

test("对齐有规模守卫：超大整体重写不去算 O(n·m)", () => {
  // 中段乘积超预算时退回逐行配对——那是旧行为，不好但**有界**。这块 diff 只渲染 60 行，
  // 为一个几千行的整体重写去跑 LCS 不值得（而且会吃掉几十 MB 的表）。
  const big = (tag) => Array.from({ length: 2600 }, (_, i) => `${tag} ${i}`).join("\n");
  const t0 = Date.now();
  const html = buildDiffView(big("a"), big("b"), "big.ts");
  assert.ok(Date.now() - t0 < 3000, "超大重写把渲染卡住了");
  assert.ok(String(html).includes("atc-diff-row"), "退化路径也要能渲染出东西");
});

test("buildDiffView caps large previews", () => {
  const newText = Array.from({ length: 65 }, (_, index) => `line ${index + 1}`).join("\n");
  const html = buildDiffView("", newText, "README.md");

  assert.equal((html.match(/atc-diff-row--add/g) || []).length, 60);
  // 省略号是 U+2026，不是三个 ASCII 点：产品里的截断标记跟 `@@ N unchanged lines @@`
  // 是同一套排版，改成 ASCII 属于把界面改动混进重构里。
  assert.match(html, /… 5 more lines not shown …/);
});

test("highlightDiffView colorizes through injected Monaco dependencies", async () => {
  const codeEl = { dataset: { raw: "const x = 1;" }, innerHTML: "const x = 1;" };
  const blankEl = { dataset: { raw: "   " }, innerHTML: "" };
  const diff = {
    dataset: { lang: "js" },
    querySelectorAll(selector) {
      assert.equal(selector, ".atc-diff-code[data-raw]");
      return [codeEl, blankEl];
    },
  };
  const container = {
    querySelector(selector) {
      assert.equal(selector, ".atc-diff");
      return diff;
    },
  };
  const calls = [];
  const monaco = {
    editor: {
      async colorize(raw, lang, options) {
        calls.push({ raw, lang, options });
        return "<div><span>colored</span></div>";
      },
    },
  };

  await highlightDiffView(container, { monaco, monacoLang: (lang) => (lang === "js" ? "javascript" : lang) });

  assert.deepEqual(calls, [{ raw: "const x = 1;", lang: "javascript", options: { tabSize: 2 } }]);
  assert.equal(codeEl.innerHTML, "<span>colored</span>");
  assert.equal(blankEl.innerHTML, "");
});

test("增删行的底色要铺到最宽那行，横向滚动后右边不能是白的", () => {
  // 用户实拍：「被删除的和新增的代码那个背景颜色没覆盖全，后面还是白色的，右边那里」。
  //
  // 根因不是底色画少了。行是块级 flex，宽度 auto = 滚动容器的**可视宽**；只有最长那行被
  // min-width: fit-content 顶到自己的内容宽，于是它一个人撑出了滚动宽度，其余每一行都停在
  // 可视宽的边上。往右一滚，除了最长的那一两行，全是白的。
  //
  // width: 100% 在滚动容器里等于可视宽，治不了。单列网格才治得了：
  // 轨道 = minmax(可视宽, 最宽那行)，所有行按轨道拉伸。改前/改后各渲过一版图对照确认。
  // 先把注释剥掉再切：上面那几段注释里逐字写着 min-width: fit-content、overflow: hidden，
  // 不剥的话「这些写法不许回来」的断言会匹配到**说明它们的注释**，改回去照样绿。
  const css = readFileSync(new URL("../src/styles/app.css", import.meta.url), "utf8")
    .replace(/\/\*[\s\S]*?\*\//g, "");
  const rule = (sel) => {
    const i = css.indexOf(`\n${sel} {`);
    assert.ok(i > 0, `找不到规则 ${sel}`);
    return css.slice(i, css.indexOf("}", i));
  };
  const box = rule(".atc-diff");
  assert.match(box, /display:\s*grid/, "滚动容器不是网格——行宽又只剩可视宽");
  assert.match(box, /grid-template-columns:\s*minmax\(100%,\s*max-content\)/,
    "轨道不是 minmax(100%, max-content)：只写 max-content 时内容比容器窄就不铺满，只写 100% 就等于可视宽");
  const row = rule(".atc-diff-row");
  assert.doesNotMatch(row, /min-width:\s*fit-content/,
    "min-width: fit-content 回来了——它只让**这一行**长到自己的内容宽，别的行照旧停在可视宽");
  const code = rule(".atc-diff-code");
  assert.match(code, /flex:\s*1 0 auto/,
    "代码格被允许压缩了：压窄了这行的 max-content 就变小，网格轨道跟着变窄，最长那行反而被截");
  assert.doesNotMatch(code, /overflow:\s*hidden/,
    "overflow: hidden 把这一格的最小宽压成 0，等于把「这行有多宽」抹掉，轨道量不到最宽行");
  assert.doesNotMatch(code, /text-overflow:\s*ellipsis/,
    "横向能滚的容器里省略号轮不到出场，留着只会让人以为这行被截断了");
});
