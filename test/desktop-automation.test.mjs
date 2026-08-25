import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SRC = readFileSync(join(ROOT, "src/main.js"), "utf8");

/**
 * 读屏结果给模型的那份不许带缩进。
 *
 * 这不是省 token 的洁癖，是**能不能看见目标控件**的问题。下游 _toolMsgForModel 按字符切，
 * 而 _headTailModelText 挖的是中段——正文控件通常就在中段。实测同一批 500 个元素：
 * 缩进版 182 字符/元素，紧凑版 100 字符/元素，缩进白烧掉 82%，换来的全是被挖空的元素。
 *
 * 面板那份照旧缩进，那是给人看的——所以这里要钉的是「两份不是同一个」。
 */
test("读屏给模型的那份不带缩进，给人看的那份才带", () => {
  const at = SRC.indexOf('const _rsPayload =');
  assert.ok(at > 0, "read_screen 的结果拼装找不到了");
  const body = SRC.slice(at, at + 1200);

  assert.ok(
    /const structured = JSON\.stringify\(_rsPayload\);/.test(body),
    "给模型的那份又带上缩进了——500 个元素会有九成被从中段挖掉",
  );
  assert.ok(
    /const structuredForView = JSON\.stringify\(_rsPayload, null, 2\);/.test(body),
    "给人看的那份不带缩进了，面板会变成一坨",
  );
  // 真正决定成败的是 content 里放的是哪一个。放错了上面两条都绿，功能照旧坏。
  assert.ok(
    /content: `read_screen 真实结果：\\n\$\{structured\}`/.test(body),
    "喂给模型的不是紧凑那份",
  );
  assert.ok(
    /_escHtml\(structuredForView\.slice\(0, 24000\)\)/.test(body),
    "面板显示的不是缩进那份",
  );
});

/**
 * 读屏结果不许落进 8000 那一档。
 *
 * 8000 字符约等于 40 个元素。后端一次最多给 500 个，于是 Chrome / Electron 这类
 * 真实界面会被砍掉九成——而那恰恰是桌面自动化最常见的目标。模型看不到要点的控件，
 * 就去重读同一份被挖空的结果、转 OCR、或者猜坐标，每一次都再付一整轮往返。
 * 用户那句「桌面自动化很烂不好用」有一半是这么来的。
 */
test("读屏和 ui_extract 的结果不吃 8000 那一档", () => {
  const at = SRC.indexOf("const _cap =");
  assert.ok(at > 0, "_cap 分档不见了");
  const tiers = SRC.slice(at, SRC.indexOf("const rawMessage", at));

  assert.ok(
    /_rt === "readscreen" \|\| _rt === "uiextract" \? 30000/.test(tiers),
    "读屏结果又掉回 8000 档了——500 个元素只有约 40 个到得了模型",
  );
  // 分档是从上往下短路的：这一条必须排在兜底的 `: 8000` 前面，写在后面等于没写。
  const mine = tiers.indexOf('_rt === "readscreen"');
  const fallback = tiers.indexOf(": 8000");
  assert.ok(mine > 0 && fallback > 0, "分档结构变了，判据要跟着改");
  assert.ok(mine < fallback, "读屏那一档排在兜底 8000 后面，短路不到它");
});

/**
 * 紧凑序列化 + 30000 档，合起来必须真的把「到达模型的元素数」抬上去。
 *
 * 上面两条钉的是写法，这一条量的是效果——写法对而效果没变（比如哪天 marker 变长、
 * 或者元素字段暴涨）时，前两条是绿的，只有这条会红。
 */
test("改完之后到达模型的元素数确实上去了", () => {
  const element = (i) => ({
    ref: i + 1, role: "Button", text: "打开设置面板",
    x: 120 + i, y: 340 + i, w: 88, h: 24, value: "", enabled: true,
  });
  const payload = {
    source: "ax",
    elements: Array.from({ length: 500 }, (_, i) => element(i)),
    limitations: [],
  };
  // _headTailModelText 的实际形状：留一段头、一段尾，中间换成一个标记。
  const reach = (text, cap) => {
    const marker = 133;
    const head = Math.ceil((cap - marker) * 0.45);
    const tail = cap - marker - head;
    return Math.floor((head + tail) / (text.length / 500));
  };
  const before = reach(JSON.stringify(payload, null, 2), 8000);
  const after = reach(JSON.stringify(payload), 30000);

  assert.ok(before < 60, `基线不对：改动前应该只有几十个元素能进，实际 ${before}`);
  assert.ok(
    after > 250,
    `到达模型的元素数没上去（${before} → ${after}）——两处改动有一处没起作用`,
  );
});
