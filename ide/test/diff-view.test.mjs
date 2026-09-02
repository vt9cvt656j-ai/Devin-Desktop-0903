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
  // 用户实拍两次：「被删除的和新增的代码那个背景颜色没覆盖全，右边都是空白」。
  //
  // 行的宽度 auto 在横向滚动容器里等于**可视宽**；只有最长那行会把滚动宽度撑开，其余每行
  // 都停在可视宽的边上，往右一滚就全是白的。width: 100% 也治不了——100% 就是可视宽。
  //
  // 第一版用单列网格 minmax(100%, max-content)，**不成立**：轨道基准是 100%（可视宽），
  // 上限是 max-content，而轨道只有在容器里还有剩余空间时才会长向上限；容器正好等于可视宽，
  // 剩余为 0，轨道就停在可视宽上。浏览器里实测 gridTemplateColumns = "280px"，每行也都是
  // 280 —— 和没改一样。（教训：这条只能在真排版引擎里量，不能靠推。）
  //
  // 成立的是内层那一格：width: max-content 长到最宽那行，min-width: 100% 保证内容窄时也
  // 不短于可视宽；行是块级、宽度 auto，自然铺满它。四种写法在浏览器里逐个量过，只有这种
  // 让每一行都等于 scrollWidth。
  const css = readFileSync(new URL("../src/styles/app.css", import.meta.url), "utf8")
    // 先剥注释再断言：注释里逐字写着被废掉的那几种写法，不剥的话改回去照样绿。
    .replace(/\/\*[\s\S]*?\*\//g, "");
  const rule = (sel) => {
    const i = css.indexOf(`\n${sel} {`);
    assert.ok(i > 0, `找不到规则 ${sel}`);
    return css.slice(i, css.indexOf("}", i));
  };
  const inner = rule(".atc-diff-inner");
  assert.match(inner, /width:\s*max-content/, "内层没有长到最宽那行，短行的底色还是停在可视宽");
  assert.match(inner, /min-width:\s*100%/, "内层没有兜住可视宽，内容比容器窄时右边会露白");
  // 那套不成立的网格不许回来。
  assert.doesNotMatch(rule(".atc-diff"), /display:\s*grid/,
    "又用网格撑宽度了——minmax(100%, max-content) 的轨道在没有剩余空间时长不起来，实测等于没改");
  assert.doesNotMatch(rule(".atc-diff-row"), /min-width:\s*fit-content/,
    "min-width: fit-content 回来了——它只让**这一行**长到自己的内容宽，别的行照旧停在可视宽");
  const code = rule(".atc-diff-code");
  assert.match(code, /flex:\s*1 0 auto/,
    "代码格被允许压缩了：压窄了这行的 max-content 就变小，内层跟着变窄，最长那行反而被截");
  assert.doesNotMatch(code, /overflow:\s*hidden/,
    "overflow: hidden 把这一格的最小宽压成 0，等于把「这行有多宽」抹掉，内层量不到最宽行");
  assert.doesNotMatch(code, /text-overflow:\s*ellipsis/,
    "横向能滚的容器里省略号轮不到出场，留着只会让人以为这行被截断了");
});

test("每一行都在 .atc-diff-inner 里面——漏在外面的那一行就是白的那一行", () => {
  // 样式只管到内层里的行。页脚那条 @@ 提示也必须在里面：它有自己的底色和上边框，
  // 漏在外面就是横滚之后突然断掉的那一条。
  const html = buildDiffView("a\nb\n", "a\nc\nd\n", "x.py");
  const open = html.indexOf(`<div class="atc-diff-inner">`);
  assert.ok(open > 0, "没有内层容器——每行又只剩可视宽");
  assert.ok(open < html.indexOf(`class="atc-diff-row`), "第一行落在了内层外面");
  assert.ok(html.trimEnd().endsWith("</div></div>"), "内层没有被关掉");
  // 长文件会带页脚，页脚也要在内层里。
  const long = buildDiffView("", Array.from({ length: 90 }, (_, i) => `line ${i}`).join("\n"), "x.py");
  const more = long.indexOf(`class="atc-diff-more"`);
  assert.ok(more > 0, "没生成页脚——这条断言就守不住东西了");
  assert.ok(more > long.indexOf(`<div class="atc-diff-inner">`), "页脚漏在了内层外面");
  assert.ok(more < long.lastIndexOf("</div></div>"), "页脚跑到了内层关闭之后");
});
