import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
// 工具目录已搬进 src/agent/tool-catalog.js。schema 那一半从**数据结构**读
// （`.find(t => t.function.name === "ui_click")`），比正则可靠；执行分支那一半
// 仍在 main.js 里，所以两份都要。
const MAIN = readFileSync(join(ROOT, "src/main.js"), "utf8");
const CATALOG = readFileSync(join(ROOT, "src/agent/tool-catalog.js"), "utf8");
const ACC = readFileSync(join(ROOT, "src-tauri/src/accessibility.rs"), "utf8");
const TREE = readFileSync(join(ROOT, "automation-framework/src/platform/macos_tree.rs"), "utf8");

/**
 * ui_click 能做哪些动作，这件事在**五个地方**各写了一份，跨三个 crate/语言：
 *
 *   1. main.js 的工具 schema `enum`        —— 模型看到的合法取值
 *   2. main.js 的执行分支白名单            —— 前端第一道拦截
 *   3. accessibility.rs 的 `matches!`      —— Tauri 第二道拦截
 *   4. accessibility.rs 的 JXA 白名单      —— 老路（sidecar 不可用时）能不能生成脚本
 *   5. macos_tree.rs `act()` 的 match 分支 —— 快路（默认路）真正的实现
 *
 * 中间没有任何类型把它们绑在一起。漏在不同位置的后果各不相同，但没有一个是"报个错就完了"：
 *   · 漏在 1 → 模型根本不知道有这个动作；
 *   · 漏在 2/3 → 请求被拦成「这个动作不存在」，而实现明明写着（scroll_to 就这么漏过）；
 *   · 漏在 4 → 快路不可用时那个动作静默消失；
 *   · 漏在 5 → 退回老路，而老路的 ref 是另一套编号，**会点到别的元素上**。
 *
 * 所以判据是：五份必须**完全相同**。Rust 侧另有一条 cargo 测试守 3/4/5，
 * 这条是唯一同时看得见 JS 和 Rust 的。
 */

function schemaEnum() {
  const at = CATALOG.indexOf('name: "ui_click"');
  assert.ok(at > 0, "ui_click 的工具定义不见了");
  const m = CATALOG.slice(at, at + 4000).match(/enum: \[([^\]]+)\]/);
  assert.ok(m, "ui_click schema 里的 action enum 取不到");
  return new Set([...m[1].matchAll(/"([a-z_]+)"/g)].map((x) => x[1]));
}

function executorList() {
  const m = MAIN.match(/if \(!\[((?:\s*"[a-z_]+",?)+)\]\.includes\(call\.action\)\)/);
  assert.ok(m, "前端执行分支的动作白名单取不到");
  return new Set([...m[1].matchAll(/"([a-z_]+)"/g)].map((x) => x[1]));
}

function rustAllowed() {
  const at = ACC.indexOf("pub async fn ui_click(");
  assert.ok(at > 0, "ui_click 不见了");
  const start = ACC.indexOf("matches!(", at);
  const end = ACC.indexOf("\n    ) {", start);
  assert.ok(start > 0 && end > start, "Rust 放行清单的形状变了");
  return new Set([...ACC.slice(start, end).matchAll(/"([a-z_]+)"/g)].map((x) => x[1]));
}

function jxaWhitelist() {
  const start = ACC.indexOf("let act = match action {");
  assert.ok(start > 0, "JXA 白名单不见了");
  const end = ACC.indexOf("};", start);
  return new Set([...ACC.slice(start, end).matchAll(/"([a-z_]+)"/g)].map((x) => x[1]));
}

function fastPathArms() {
  const start = TREE.indexOf("pub fn act(");
  assert.ok(start > 0, "快路的 act() 不见了");
  // 切到 act() 结束，别把同文件的 #[cfg(test)] 也算进来——那里面有
  // `super::act(999_999, "press", None, None)`，会把断言喂绿。
  const end = TREE.indexOf("\n#[cfg(test)]", start);
  assert.ok(end > start, "找不到 act() 的结尾");
  const body = TREE.slice(start, end);
  // 钉 match 分支的形状，不是"这个词出现过"：每个分支的回执 JSON 里都写着
  // `"action": "press"`，只查子串的话把分支删掉照样绿。
  return new Set([...body.matchAll(/"([a-z_]+)" =>/g)].map((x) => x[1]));
}

test("ui_click 的动作清单，五个地方必须完全一致", () => {
  const lists = {
    "工具 schema 的 enum": schemaEnum(),
    "前端执行分支": executorList(),
    "Rust 放行清单": rustAllowed(),
    "JXA 白名单": jxaWhitelist(),
    "快路 act() 的分支": fastPathArms(),
  };

  // 任何一个解析器失灵都会得到空集合，而空集合之间"完全一致"——先钉地板。
  for (const [name, set] of Object.entries(lists)) {
    assert.ok(set.size >= 8, `${name} 只解析出 ${set.size} 个动作，解析判据坏了：${[...set]}`);
  }

  const base = lists["工具 schema 的 enum"];
  for (const [name, set] of Object.entries(lists)) {
    const missing = [...base].filter((a) => !set.has(a));
    const extra = [...set].filter((a) => !base.has(a));
    assert.deepEqual(
      { missing, extra },
      { missing: [], extra: [] },
      `${name} 和工具 schema 对不上：少了 [${missing}]，多了 [${extra}]。\n`
        + "五份清单必须完全相同——漏在快路会退回老路，而老路的 ref 是另一套编号，会点到别的元素上。",
    );
  }
});

/**
 * 工具描述里的承诺要和实现对得上。
 *
 * 这条描述是模型唯一的依据。原文写着 "pid + name are both checked"——而句柄表里只存了
 * pid，name 从来没参与过校验。承诺落空比不承诺更糟：模型会据此认为 ref 绝无可能落错窗口。
 */
test("ui_click 的描述不承诺它没做的校验", () => {
  const at = CATALOG.indexOf('name: "ui_click"');
  const desc = CATALOG.slice(at, at + 4000);
  assert.doesNotMatch(
    desc,
    /pid \+ name are both checked/,
    "描述仍然承诺同时校验 pid 和 name，而实现只比对 pid",
  );
  assert.match(
    desc,
    /pid/,
    "描述要说清 ref 带着它的目标进程，换个进程会被拒",
  );
  // 实现侧真的在比。描述改成只提 pid 之后，这条保证实现别又退回去什么都不查。
  const act = TREE.slice(TREE.indexOf("pub fn act("), TREE.indexOf("\n#[cfg(test)]", TREE.indexOf("pub fn act(")));
  assert.match(act, /expect_pid/, "快路的 act() 没有接收调用方的 pid");
  assert.match(act, /want != held\.pid/, "快路的 act() 收了 pid 却没比对");
});
