import test from "node:test";
import assert from "node:assert/strict";

import { ansiToHtml, ansiToText, xterm256Hex } from "../src/agent/ansi.js";

const E = "\x1b";

test("SGR 颜色变成语义 class，而不是原样打出 [31m", () => {
  const html = ansiToHtml(`${E}[31m错误${E}[0m 正常`);
  assert.equal(html, '<span class="ansi-fg-red">错误</span> 正常');
  assert.ok(!html.includes("[31m"), "转义码不能作为可见文本漏出来");
});

test("亮色、加粗、下划线各自有 class", () => {
  assert.match(ansiToHtml(`${E}[91mx`), /ansi-fg-bred/);
  assert.match(ansiToHtml(`${E}[1mx`), /ansi-bold/);
  assert.match(ansiToHtml(`${E}[4mx`), /ansi-underline/);
  assert.match(ansiToHtml(`${E}[42mx`), /ansi-bg-green/);
  assert.match(ansiToHtml(`${E}[100mx`), /ansi-bg-bblack/);
});

test("同一段样式只开一个 span，不是每个字符一个", () => {
  const html = ansiToHtml(`${E}[32m通过通过通过${E}[0m`);
  assert.equal(html.match(/<span/g).length, 1);
});

test("256 色与真彩色走内联样式", () => {
  assert.match(ansiToHtml(`${E}[38;5;208m橙`), /style="color:#ff8700"/);
  assert.match(ansiToHtml(`${E}[38;2;18;52;86m色`), /style="color:#123456"/);
  assert.match(ansiToHtml(`${E}[48;2;255;0;0m底`), /style="background-color:#ff0000"/);
});

test("256 色的前 16 个索引仍然用主题 class（跟着深浅色走）", () => {
  assert.match(ansiToHtml(`${E}[38;5;1mx`), /ansi-fg-red/);
  assert.match(ansiToHtml(`${E}[38;5;9mx`), /ansi-fg-bred/);
});

test("xterm256Hex 覆盖立方体与灰阶两段", () => {
  assert.equal(xterm256Hex(16), "#000000");
  assert.equal(xterm256Hex(231), "#ffffff");
  assert.equal(xterm256Hex(232), "#080808");
  assert.equal(xterm256Hex(255), "#eeeeee");
  assert.equal(xterm256Hex(15), null, "0-15 该走主题 class，不是硬编码 hex");
  assert.equal(xterm256Hex(256), null);
});

test("参数不全的 38 不会把后面的数字当成新的 SGR 码", () => {
  // `38;5` 少一个索引：整条丢掉，后面的 31 也不该被当成红色。
  const html = ansiToHtml(`${E}[38;5m文字`);
  assert.equal(html, "文字");
});

test("光标移动、清屏、隐藏光标这些非 SGR 序列直接丢掉", () => {
  assert.equal(ansiToText(`${E}[2J${E}[H${E}[?25l干净${E}[?25h`), "干净");
  assert.equal(ansiToText(`${E}[1A${E}[2K重画`), "重画");
});

test("OSC（改窗口标题 / 超链接）被丢掉，不会把标题打进正文", () => {
  assert.equal(ansiToText(`${E}]0;某个标题\x07正文`), "正文");
  assert.equal(ansiToText(`${E}]8;;https://x.dev${E}\\链接${E}]8;;${E}\\`), "链接");
});

test("\\r 是回到行首覆盖写，不是换行——进度条只留最后一帧", () => {
  assert.equal(ansiToText("下载 10%\r下载 100%"), "下载 100%");
  assert.equal(ansiToText("abcdef\rXY"), "XYcdef");
  // \r\n 是普通换行，不能把上一行清掉。
  assert.equal(ansiToText("第一行\r\n第二行"), "第一行\n第二行");
});

test("退格键回退一格", () => {
  assert.equal(ansiToText("abc\b\bX"), "aXc");
});

test("HTML 被转义——命令输出是不可信内容", () => {
  assert.equal(ansiToHtml("<script>x</script>"), "&lt;script&gt;x&lt;/script&gt;");
  const img = ansiToHtml('<img src=x onerror="alert(1)">');
  assert.ok(!img.includes("<img"), img);
  assert.ok(img.startsWith("&lt;img"), img);
});

test("即使颜色码把 span 属性拼出来，注入的也只能是我们自己的 class", () => {
  // 攻击面：SGR 参数只允许数字，正则不接受引号，拼不出属性来。
  const html = ansiToHtml(`${E}[31;"onload=alert(1)"m x`);
  assert.ok(!html.includes("onload"), html);
});

test("maxChars 按可见字符截断，转义码不占额度", () => {
  const long = `${E}[31m` + "字".repeat(50);
  const out = ansiToText(long, { maxChars: 10 });
  assert.ok(out.startsWith("字".repeat(10)), out);
  assert.match(out, /输出已截断/);
  // 没超过就不该有截断提示。
  assert.equal(ansiToText("短", { maxChars: 10 }), "短");
});

test("中文、emoji、制表符原样保留", () => {
  const s = "中文\t测试 ✅ 🚀";
  assert.equal(ansiToText(s), s);
});

test("空输入不炸", () => {
  assert.equal(ansiToHtml(""), "");
  assert.equal(ansiToHtml(null), "");
  assert.equal(ansiToHtml(undefined), "");
  assert.equal(ansiToText(""), "");
});

test("反显（ESC[7m）会交换前景背景", () => {
  const html = ansiToHtml(`${E}[7m反显`);
  assert.match(html, /ansi-fg-default/);
  assert.match(html, /ansi-bg-default/);
  const swapped = ansiToHtml(`${E}[31;7m反显`);
  assert.match(swapped, /ansi-bg-red/, "31 是前景红，反显后应该变成背景红");
});

test("真实的 cargo/pytest 片段整段渲染得出来", () => {
  const real =
    `${E}[0m${E}[1m${E}[32m   Compiling${E}[0m michael-ide v0.3.94\n` +
    `${E}[0m${E}[1m${E}[33mwarning${E}[0m${E}[0m${E}[1m: unused variable\n` +
    `${E}[0m${E}[1m${E}[31merror${E}[0m: 1 个错误\n`;
  const text = ansiToText(real);
  assert.ok(text.includes("Compiling"), text);
  assert.ok(text.includes("1 个错误"), text);
  assert.ok(!text.includes("\x1b"), "不能有转义字节残留");
  const html = ansiToHtml(real);
  assert.match(html, /ansi-fg-green/);
  assert.match(html, /ansi-fg-yellow/);
  assert.match(html, /ansi-fg-red/);
});

test("纯文本走短路快路径，结果和全解析一致", () => {
  const plain = "line one\nline two\t中文 ✅";
  assert.equal(ansiToText(plain), plain);
  assert.equal(ansiToHtml(plain), plain);
  // 快路径也要转义。
  assert.equal(ansiToHtml("a<b>c"), "a&lt;b&gt;c");
  // 快路径的截断行为要和慢路径一样。
  assert.match(ansiToText("x".repeat(50), { maxChars: 10 }), /^x{10}\n… 输出已截断$/);
  assert.match(ansiToHtml("x".repeat(50), { maxChars: 10 }), /输出已截断/);
});

test("2MB 构建日志不会把界面卡住", () => {
  const big = ("cargo: compiling crate number 12345\n").repeat(60000);
  const t0 = process.hrtime.bigint();
  const out = ansiToText(big);
  const ms = Number(process.hrtime.bigint() - t0) / 1e6;
  assert.equal(out.length, big.length);
  assert.ok(ms < 200, `纯文本 ${(big.length / 1e6).toFixed(1)}MB 花了 ${ms.toFixed(0)}ms`);
});
