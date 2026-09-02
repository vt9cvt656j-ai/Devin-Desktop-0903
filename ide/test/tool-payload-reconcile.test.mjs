// ── 真实发送路那一跳的对账 ────────────────────────────────────────────────
//
// L0 把工具**一分为二**：内置的只发名字（`x-ide-tools` 头，网关按自己那份 tools.json
// 回填 schema），其余整份 schema 随请求体发出去（`body.tools`）。这一跳是全链最后一处
// 能悄悄弄丢一个工具的地方——丢了不报错，只是模型看不见它，然后"它怎么不会用这个功能"。
//
// 这个文件存在的理由：2026-08 那轮审计查出的 13 个缺陷里，**5 个断在这一跳**，而当时
// 没有任何测试站在这里。已知的四种丢法（每一种都真发生过）：
//   · 用户自己声明的能力（`user__*`）被当成内置工具交给网关回填 —— 网关那份目录里
//     根本没有它，整条丢掉；
//   · 用户声明了自定义角色，而三个派发工具的 role 枚举是**客户端**补的，交给网关回填
//     就退回内置的 11 个角色名 —— 用户配好的角色永远选不中；
//   · 发布构建把那三个派发工具的描述剥空了（它们的 schema 真会发出去，别的不会）；
//   · 名字进了 `x-ide-tools` 头，可网关那份目录里没有这个名字 —— 回填不出来，也是丢。
//
// 这里跑的是**源码里真实的那段分流代码**，不照抄一份：照抄的话改了源码它照样绿。
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { SRC } from "./helpers/source.mjs";

/** 从 _agentModelTurn 里抠出真实的分流块，注入依赖跑一遍，返回 { names, keep }。 */
function split({ tools, staticNames, roles = [], }) {
  const at = SRC.indexOf("        let _rolesDeclared = false;");
  assert.ok(at > 0, "L0 分流块的起点找不到了 —— 这个文件失去落点");
  const end = SRC.indexOf("        _turnConfig.ideMode =", at);
  assert.ok(end > at, "L0 分流块的终点找不到了");
  const body = SRC.slice(at, end);
  assert.ok(body.length < 3000, `分流块切出了 ${body.length} 字符 —— 边界划到外面去了`);
  const fn = new Function("toolSchemas", "_staticToolNames", "_userCapabilities",
    `${body}\nreturn { names: _names, keep: _keep };`);
  return fn(tools, () => new Set(staticNames), () => ({ roles, tools: [], commands: [], disabled: [], errors: [] }));
}

const t = (name) => ({ type: "function", function: { name, description: "d", parameters: {} } });
const DISPATCH = ["run_subagent", "run_worker", "spawn_multiple_agents"];
const BUILTIN = ["read_file", "write_file", "run_cmd", ...DISPATCH];

test("一分为二必须是**划分**：没有工具被丢掉，也没有被算两次", () => {
  // 这条是整个文件的核心。四种已知丢法全部表现为「这个等式不成立」。
  for (const roles of [[], [{ name: "无障碍审计" }]]) {
    for (const extra of [[], ["user__crm", "user__wiki"]]) {
      const tools = [...BUILTIN, ...extra].map(t);
      const { names, keep } = split({ tools, staticNames: BUILTIN, roles });
      const got = [...names, ...keep.map((x) => x.function.name)].sort();
      assert.deepEqual(got, [...BUILTIN, ...extra].sort(),
        `工具在分流时丢了或重了（roles=${roles.length}, 用户能力=${extra.length}）`);
    }
  }
});

test("用户自己声明的能力永远随请求体发出去 —— 网关目录里没有它", () => {
  const tools = [...BUILTIN, "user__crm"].map(t);
  // 就算 staticNames 里混进了同名（网关目录漂移/同名内置），也不许把它交出去。
  for (const staticNames of [BUILTIN, [...BUILTIN, "user__crm"]]) {
    const { names, keep } = split({ tools, staticNames });
    assert.ok(!names.includes("user__crm"),
      "user__* 被当成内置工具交给网关回填了 —— 网关那份目录里没有它，整条丢掉");
    assert.ok(keep.some((x) => x.function.name === "user__crm"), "user__* 的 schema 没随请求体发出去");
  }
});

test("声明了自定义角色，三个派发工具就要连 schema 一起发", () => {
  // 角色名是 _applyUserRoleEnums 补进**客户端这份** schema 的 role 枚举里的。
  // 交给网关回填就退回它自己目录里的 11 个内置角色名，用户配好的那个永远选不中。
  const tools = BUILTIN.map(t);
  const { names, keep } = split({ tools, staticNames: BUILTIN, roles: [{ name: "无障碍审计" }] });
  for (const d of DISPATCH) {
    assert.ok(!names.includes(d), `${d} 被交给网关回填了 —— 用户声明的角色枚举会丢`);
    assert.ok(keep.some((x) => x.function.name === d), `${d} 的 schema 没随请求体发出去`);
  }
});

test("没声明角色就一个字节都不多发", () => {
  // 代价只该落在真的声明了角色的用户身上。
  const tools = BUILTIN.map(t);
  const { names, keep } = split({ tools, staticNames: BUILTIN, roles: [] });
  assert.deepEqual(names.sort(), [...BUILTIN].sort(), "没声明角色时全部内置工具都该只发名字");
  assert.deepEqual(keep, [], "没声明角色却还有 schema 随体发出去");
});

test("只发名字的那些，网关目录里必须真有 —— 否则回填不出来也是丢", () => {
  // 判据取自源码：交出去的前提是 `_stat.has(_n)`。网关不认识的名字必须留在体里。
  const tools = [...BUILTIN, "brand_new_tool"].map(t);
  const { names, keep } = split({ tools, staticNames: BUILTIN });
  assert.ok(!names.includes("brand_new_tool"),
    "把一个网关目录里没有的名字交出去了 —— 网关回填不出 schema，模型看不到这个工具");
  assert.ok(keep.some((x) => x.function.name === "brand_new_tool"));
});

test("剥除的豁免名单 == 真正随体发出去的那批（两张表必须对得上）", () => {
  // 这条和 release-tool-descriptions.test.mjs 里那条是**同一条不变量的两半**：
  // 那边守「剥除时确实豁免了这几个」，这边守「豁免名单和真正随体发出去的集合一致」。
  //
  // 两边分开漂移过一次：L0 先把这三个摘出来随体发（为了用户声明的角色枚举），而
  // build/strip-tool-ip.mjs 还照旧把它们的描述剥空——发布版里模型收到
  // `{name:"run_subagent", description:""}`，整套自定义角色是哑的，而开发版一切正常。
  //
  // 判据取自两边的源码，不手抄名单：豁免名单少一个 → 那个工具在发布版里没描述；
  // 多一个 → 白白多泄露一条描述（它根本不随体发）。
  const buildSrc = readFileSync(new URL("../build/strip-tool-ip.mjs", import.meta.url), "utf8");
  const m = /const KEEP_DESCRIPTIONS = \[([^\]]*)\]/.exec(buildSrc);
  assert.ok(m, "build/strip-tool-ip.mjs 里找不到 KEEP_DESCRIPTIONS —— 豁免机制没了？");
  const exempt = [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1]).sort();

  // 真正随体发出去的那批 = 声明角色时留在 keep 里的内置工具（user__* 是动态编译的，
  // 不在剥除范围内，所以不参与这条对账）。
  const tools = BUILTIN.map(t);
  const { keep } = split({ tools, staticNames: BUILTIN, roles: [{ name: "无障碍审计" }] });
  const inBody = keep.map((x) => x.function.name).filter((n) => !n.startsWith("user__")).sort();

  assert.deepEqual(exempt, inBody,
    "剥除豁免名单和真正随体发出去的工具集对不上。少一个 → 那个工具在发布版里没有描述"
    + "（开发版正常，本地测不出来）；多一个 → 白白多泄露一条描述。\n"
    + `  豁免名单：${exempt.join(", ")}\n  真正随体发：${inBody.join(", ")}`);
});
