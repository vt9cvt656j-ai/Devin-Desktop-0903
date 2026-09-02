// 文档渲染器的攻击性测试。
//
// 这一页和会话 cookie 同域 —— `mide_token` 的 Domain 是 `.mrday.one`，本站脚本读得到它。
// 所以正文里一次存储型 XSS 就是一次会话泄露，而正文是**存在数据库里、由后台写入**的。
// 作者只有管理员，但这些断言防的不是外人：是万一管理员账号被盗时，不至于连带把每个访客的
// 会话一起赔进去。
//
// 每条用例都是一个真实的绕过手法，不是"随便造几个尖括号"。
import test from "node:test";
import assert from "node:assert/strict";
import { renderMarkdown, renderMarkdownBlocks, extractHeadings } from "./markdown.ts";

/**
 * 渲染结果里绝不该出现的东西。
 *
 * **只检查真正的标签内部**，不是整段字符串。理由：输入里的每个 `<` 都被转义成了 `&lt;`，
 * 所以正文里出现 ` onerror=` 这样的字样是**惰性文本**，恰恰说明转义生效了。按纯文本匹配
 * 会把这种正确输出判成危险 —— 第一版就是这么写的，红的是断言不是渲染器。
 *
 * 能构成危险的只有一种情况：属性真的落进了标签里。所以把 `<...>` 抠出来单独看。
 */
function assertInert(md, label) {
  const out = renderMarkdown(md);
  const tags = out.match(/<[^>]*>/g) || [];
  for (const tag of tags) {
    assert.ok(!/^<script/i.test(tag), `${label}: 出现了 <script> 标签 — ${out}`);
    assert.ok(!/^<iframe/i.test(tag), `${label}: 出现了 <iframe> 标签 — ${out}`);
    assert.ok(!/\son\w+\s*=/i.test(tag), `${label}: 标签里有事件属性 — ${tag}`);
    assert.ok(!/javascript:/i.test(tag), `${label}: 标签里有 javascript: — ${tag}`);
  }
  // 同时确认输入里的 `<` 确实被转义掉了 —— 没有它，上面的检查就没有意义。
  if (md.includes("<")) {
    assert.ok(out.includes("&lt;"), `${label}: 输入里的 < 没有被转义 — ${out}`);
  }
  return out;
}

test("原始 HTML 一律变成字面文字，不变成标签", () => {
  const out = assertInert("<script>alert(document.cookie)</script>", "script 标签");
  // 不只是"没执行"，而是原样显示给读者看。
  assert.match(out, /&lt;script&gt;/);

  assertInert('<img src=x onerror="alert(1)">', "img onerror");
  assertInert("<iframe src=//evil.test></iframe>", "iframe");
  assertInert('<svg><animate onbegin="alert(1)" /></svg>', "svg 事件");
  assertInert("<body onload=alert(1)>", "body onload");
});

test("链接地址只放行 http/https、站内路径和锚点", () => {
  assertInert("[点我](javascript:alert(1))", "javascript 伪协议");
  assertInert("[点我](JaVaScRiPt:alert(1))", "大小写混写的伪协议");
  // data: 能塞 HTML 文档，同样不放行。
  const data = renderMarkdown("[点我](data:text/html,<script>alert(1)</script>)");
  assert.ok(!/href="data:/i.test(data), `data: 被放行了 — ${data}`);

  // 正常链接要照常工作，否则这个渲染器就没用了。
  const ok = renderMarkdown("见 [官网](https://mrday.one/) 说明");
  assert.match(ok, /<a href="https:\/\/mrday\.one\/"/);
  assert.match(ok, /rel="noreferrer noopener"/);
  // 站内相对路径和锚点是文档互相引用要用的。
  assert.match(renderMarkdown("[下一页](/docs/next)"), /href="\/docs\/next"/);
  assert.match(renderMarkdown("[本页](#anchor)"), /href="#anchor"/);
});

test("属性不能被引号提前闭合", () => {
  // 经典手法：在 href 里塞一个引号，指望它闭合 href、把后面的内容变成一个新属性。
  //
  // 这里有两种可接受的结局，都安全：链接压根没被识别（留成字面文字），或者被识别了但引号
  // 已经转义成 &quot; 关不掉属性。**不可接受的只有一种**：真的生成了带事件属性的 <a>。
  // 所以断言要看生成的标签，而不是整段文本 —— 转义后的 ` onmouseover=` 出现在正文里是
  // 惰性的，按文本匹配会把安全的输出误判成危险。
  const out = assertInert('[x](https://a.test" onmouseover="alert(1))', "引号闭合");
  for (const tag of out.match(/<a\b[^>]*>/gi) || []) {
    assert.ok(!/\son\w+\s*=/i.test(tag), `生成的链接带上了事件属性 — ${tag}`);
    const href = (tag.match(/href="([^"]*)"/) || [])[1] || "";
    assert.ok(/^(https?:\/\/|\/|#)/.test(href), `链接地址没过白名单 — ${href}`);
  }
});

test("代码块里的一切都是字面文本", () => {
  const out = assertInert("```html\n<script>alert(1)</script>\n```", "代码块内的 script");
  assert.match(out, /<pre><code class="lang-html">/);
  assert.match(out, /&lt;script&gt;/);

  // 行内代码同理，且不该被强调语法二次解析。
  const inline = renderMarkdown("用 `<b>*x*</b>` 这样写");
  assert.match(inline, /<code>&lt;b&gt;\*x\*&lt;\/b&gt;<\/code>/);
  assert.ok(!/<b>/.test(inline));
});

test("常规语法要真的能用", () => {
  // 标题现在带 id 和一个锚点链接（用来复制段落地址）。
  assert.match(renderMarkdown("# 标题"), /<h2 id="标题">.*标题<\/h2>/);
  assert.match(renderMarkdown("### 小标题"), /<h4 id="小标题">/);
  assert.match(renderMarkdown("**粗**"), /<strong>粗<\/strong>/);
  assert.match(renderMarkdown("*斜*"), /<em>斜<\/em>/);
  assert.match(renderMarkdown("- 一\n- 二"), /<ul>\n<li>一<\/li>\n<li>二<\/li>\n<\/ul>/);
  assert.match(renderMarkdown("1. 一\n2. 二"), /<ol>/);
  assert.match(renderMarkdown("> 引用"), /<blockquote>引用<\/blockquote>/);
  assert.match(renderMarkdown("---"), /<hr \/>/);
  assert.match(renderMarkdown("一段话"), /<p>一段话<\/p>/);
});

test("没闭合的代码块不会把后面的内容吞掉", () => {
  const out = renderMarkdown("```\n未闭合的代码");
  assert.match(out, /<pre><code>未闭合的代码<\/code><\/pre>/);
});

test("空输入和非字符串不会抛", () => {
  assert.equal(renderMarkdown(""), "");
  assert.equal(renderMarkdown(null), "");
  assert.equal(renderMarkdown(undefined), "");
});

test("表格能用，且单元格里的内容进不了标签", () => {
  const md = "| 你想要 | 用 |\n|---|---|\n| 把事做完 | Agent |\n| 先看方案 | Plan |";
  const out = renderMarkdown(md);
  assert.match(out, /<table>/);
  assert.match(out, /<th>你想要<\/th>/);
  assert.match(out, /<td>Agent<\/td>/);
  // 宽表格自己滚，不撑破页面。
  assert.match(out, /<div class="doc-table">/);

  // 单元格是注入的入口之一 —— 它同样必须走 esc。
  const evil = assertInert("| a | b |\n|---|---|\n| <img src=x onerror=alert(1)> | x |", "表格单元格");
  assert.match(evil, /&lt;img/);
});

test("正文里出现竖线不会被误判成表格", () => {
  // 少了分隔行就不是表格。只看当前行的话，一句普通的话就会被吃掉。
  const out = renderMarkdown("用 | 号分隔字段");
  assert.ok(!/<table>/.test(out), `普通句子被当成表格了 — ${out}`);
  assert.match(out, /<p>/);
});

test("标题 id 的字符集被约束死，构造不出属性注入", () => {
  // id 的安全性不来自转义，来自输出字符集 —— 引号/空格/等号在构造时就不可能存在。
  for (const md of [
    '# a" onmouseover=alert(1) "b',
    "## <img onerror=x>",
    "# 带 `代码` 和 [链接](https://x.test) 的标题",
  ]) {
    const [h] = extractHeadings(md);
    assert.ok(h, `没解析出标题：${md}`);
    assert.ok(/^[a-z0-9\u4e00-\u9fff-]+$/.test(h.id), `id 含白名单外字符：${h.id}`);
    for (const bad of ['"', "'", " ", "=", "<", ">", "&"]) {
      assert.ok(!h.id.includes(bad), `id 里出现了 ${bad}：${h.id}`);
    }
  }
  // `<img` 在正文里仍然是文字。
  assert.match(renderMarkdown("## <img onerror=x>"), /&lt;img/);
});

test("目录和正文用的是同一套 id —— 否则点了跳不动", () => {
  const md = "# 安装\n\n正文\n\n## 参数\n\n正文\n\n## 参数\n\n正文";
  const html = renderMarkdown(md);
  const hs = extractHeadings(md);
  assert.equal(hs.length, 3);
  for (const h of hs) {
    assert.ok(html.includes(`id="${h.id}"`), `正文里没有 ${h.id}，目录会点不动`);
  }
  // 同名标题必须去重，否则点第二个「参数」会跳到第一个。
  assert.notEqual(hs[1].id, hs[2].id);
});

test("代码块里的 # 不进目录", () => {
  const hs = extractHeadings("# 真标题\n\n```bash\n# 这是注释\n```\n\n## 另一个");
  assert.deepEqual(hs.map((h) => h.text), ["真标题", "另一个"]);
});

test("连续的引用行合成一个块", () => {
  const out = renderMarkdown("> 第一行\n> 第二行\n> 第三行");
  assert.equal((out.match(/<blockquote>/g) || []).length, 1);
});

test("站内链接不开新标签页，外链才开", () => {
  assert.ok(!/target="_blank"/.test(renderMarkdown("[x](/docs/y)")), "站内链接不该开新标签页");
  assert.ok(!/target="_blank"/.test(renderMarkdown("[x](#anchor)")), "锚点不该开新标签页");
  assert.match(renderMarkdown("[x](https://a.test)"), /target="_blank"/);
});

test("代码块单独成块，内容是未转义原文（只能交给 React 文本节点）", () => {
  const blocks = renderMarkdownBlocks("正文\n\n```ts src/main.ts\nconst a = 1 < 2;\n```\n\n结尾");
  const code = blocks.find((b) => b.kind === "code");
  assert.ok(code, "没切出代码块");
  assert.equal(code.lang, "ts");
  assert.equal(code.title, "src/main.ts");
  // 原文，不转义 —— React 渲染文本节点时自己会转义。
  assert.equal(code.code, "const a = 1 < 2;");
  // 前后的正文各自成块，顺序不能乱。
  assert.equal(blocks[0].kind, "html");
  assert.match(blocks[0].html, /正文/);
  assert.match(blocks[blocks.length - 1].html, /结尾/);
});
