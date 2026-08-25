import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { workspaceMutatingTypes, approvalTypes } from "../src/agent/tool-policy.js";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SRC = readFileSync(join(ROOT, "src/main.js"), "utf8");

/**
 * 每个工具名对应的 type，从 mapCall 的 switch 里取。
 *
 * 判据不能写成一条大正则去匹配 `case "x": return { type: "y"` —— 那些分支的写法各不相同
 * （有的带 `{`、有的先声明局部变量、有的跨好几行），一条正则解析到的只是其中一部分，
 * 而**解析不到的会静默变成"没有这个工具"**，于是对账测试对它们恒绿。
 * 改成：先定位 `case "名字":`，再取它后面出现的第一个 `type: "..."`。
 */
function toolNameToType() {
  const map = new Map();
  for (const m of SRC.matchAll(/case "([a-z0-9_]+)":/g)) {
    const after = SRC.slice(m.index, m.index + 4000);
    const t = after.match(/type: "([a-z0-9_]+)"/);
    if (t) map.set(m[1], t[1]);
  }
  return map;
}

/** _STRICT_MUTATING_TOOL_NAMES 里实际列了哪些名字。 */
function strictNames() {
  const at = SRC.indexOf("const _STRICT_MUTATING_TOOL_NAMES");
  assert.ok(at > 0, "_STRICT_MUTATING_TOOL_NAMES 不见了");
  const block = SRC.slice(at, SRC.indexOf("]);", at));
  // 只取字符串字面量，注释里的中文和说明不会混进来（它们不带英文双引号）。
  return new Set([...block.matchAll(/"([a-z0-9_]+)"/g)].map((m) => m[1]));
}

/**
 * 「按调用判」的 type：策略里 readOnlyModeBlocked 写的是函数而不是 true/false。
 *
 * git / gh / browser / system / worktree 这些类型底下**读写混装**（git_diff 和
 * git_commit 同为 type "git"）。对它们要求"每个工具名都进 strict 名单"是错的：
 * 那会把 git_status / gh_pr_view 这些纯读取的也拖进严格校验，而 strict 名单的语义是
 *「参数要严格校验的**改动类**工具」。服务端那条同源对账
 *（server/src/prompts.rs 的 per_call_readonly_types）用的是同一条规矩。
 *
 * 抽成函数是因为下面两条测试都要用——各写一份就是这个仓库反复出问题的那个形状。
 */
function perCallTypes() {
  const policySrc = readFileSync(join(ROOT, "src/agent/tool-policy.js"), "utf8");
  const out = new Set();
  const marks = [...policySrc.matchAll(/defineTool\(/g)].map((m) => m.index);
  for (let i = 0; i < marks.length; i++) {
    const seg = policySrc.slice(marks[i], marks[i + 1] ?? policySrc.length);
    const nm = seg.match(/^defineTool\(\s*"([a-z0-9_]+)"/);
    const f = seg.indexOf("readOnlyModeBlocked:");
    if (!nm || f < 0) continue;
    const val = seg.slice(f + "readOnlyModeBlocked:".length).trimStart();
    if (!val.startsWith("true") && !val.startsWith("false")) out.add(nm[1]);
  }
  assert.ok(out.size >= 5, `按调用判的 type 只解析出 ${out.size} 个，解析判据坏了：${[...out]}`);
  return out;
}

/**
 * 声明有副作用的 type，它底下的每个工具名都必须在 strict 名单里。
 *
 * 这两份清单键不一样：tool-policy.js 按 **type** 声明，main.js 的 strict 闸按**工具名**。
 * 于是谁也发现不了谁漏了——实测漏了十个，其中 browser / capture_replay /
 * docker_compose_up / create_project 都是能造成真实副作用的。
 *
 * 漏掉不是"少一层保护"：模型把调用写成文本且信封被截断时，名单外的工具会被松散修复后
 * 照常执行，而这道闸存在的全部理由就是不让被截断的调用变成一次真实执行。
 *
 * **单向检查。** 反方向（在 strict 里但 type 没声明副作用）是合法的：git / gh / worker
 * 这些 type 底下读写混装（git_diff 和 git_commit 同为 type=git），type 级声明表达不了
 * 那个粒度，按工具名更严是对的。
 */
test("声明有副作用的 type，它的每个工具都要在 strict 闸的名单里", () => {
  const n2t = toolNameToType();
  // 解析器一旦失灵，下面的循环会在空集合上跑完并通过——先把地板钉住。
  assert.ok(n2t.size >= 100, `只解析出 ${n2t.size} 条 name→type，解析判据坏了`);

  const strict = strictNames();
  assert.ok(strict.size >= 40, `strict 名单只解析出 ${strict.size} 个，解析判据坏了`);

  const risky = new Set([...workspaceMutatingTypes(), ...approvalTypes()]);
  assert.ok(risky.size >= 20, `声明有副作用的 type 只有 ${risky.size} 个，导入判据坏了`);

  const perCall = perCallTypes();

  const missing = [];
  for (const [name, type] of n2t) {
    if (perCall.has(type)) continue;
    if (risky.has(type) && !strict.has(name)) missing.push(`${name}(type=${type})`);
  }
  assert.deepEqual(
    missing,
    [],
    `这些工具的 type 声明了有副作用，却不在 _STRICT_MUTATING_TOOL_NAMES 里：\n  ${missing.join("\n  ")}\n`
      + "被截断的文本调用会绕过 strict 闸被松散修复后执行。补进那份名单。",
  );
});

/**
 * 这条对账真的在对账，而不是在两个空集合之间做恒真断言。
 *
 * 上一条里的三个地板已经挡住"解析全失灵"，这里再证一次**耦合**是活的：
 * 随便挑一个已知有副作用的工具，把它从 strict 名单里拿掉，上一条必须能发现。
 */
test("对账不是空转：拿掉一个已知有副作用的工具就会被发现", () => {
  const n2t = toolNameToType();
  const risky = new Set([...workspaceMutatingTypes(), ...approvalTypes()]);
  // 和上一条同一套豁免：按调用判的 type（git / gh / browser …）不参与这条对账。
  const perCall = perCallTypes();
  const covered = [...n2t].filter(([, t]) => risky.has(t) && !perCall.has(t)).map(([n]) => n);
  assert.ok(
    covered.length >= 10,
    `只有 ${covered.length} 个工具落在"声明有副作用"的 type 上，耦合太弱，这条对账基本不起作用`,
  );

  const strict = strictNames();
  const notCovered = covered.filter((n) => !strict.has(n));
  assert.deepEqual(notCovered, [], "上一条已经保证为空，这里重复确认耦合方向没写反");
  // 反证：假装名单里少了第一个，missing 必须非空。
  const pretend = new Set(strict);
  pretend.delete(covered[0]);
  const missing = covered.filter((n) => !pretend.has(n));
  assert.deepEqual(missing, [covered[0]], "判据算不出缺失——这条对账是空转的");
});
