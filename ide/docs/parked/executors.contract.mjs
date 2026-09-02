/**
 * 【已封存 · 不是测试】原文件是 `test/executors.test.mjs`，2026-08-12 移到这里。
 *
 * 它跟 `diff-view` / `language` 那两份是同一批写出来的，但性质完全不同：那两份是**抽取**
 * ——被测的代码逐字存在于 main.js 里，抽出来就绿。这一份**三分之二是还没做的功能**，
 * 直接实现等于照着断言把代码编出来。放在 `test/` 下会被 `node --test test/*.test.mjs` 扫到，
 * 于是整个测试套件常红，所以先挪出来存档。逐条核实的结论：
 *
 * ## 真实存在、可以抽取的部分
 * - `current_time`（main.js:46851-46860）和 `system`（49623-49646）是真的，
 *   现有代码产出的每一个字符串都能满足对应断言。
 * - **Tailwind 调色板数据是真的，不是编的。** 这一条推翻了最初的判断：
 *   `node_modules/tailwindcss/theme.css` 里装的 Tailwind 4.3.3 正好是 **26 个色族 × 11 档**，
 *   含 `olive` 和 `mist`，`olive-500 = 58% 0.031 107.3`、`emerald-600 = 59.6% 0.145 163.225`
 *   与断言逐字节吻合。要做的话应该**从 theme.css codegen** 出来（node_modules 不进版本库，
 *   渲染进程运行时也读不到），而不是手抄。
 *
 * ## 拦路的三件事
 * 1. **`shadcn_ref` 和 `tailwind_palette` 这两个工具不存在。** 全仓搜索只命中这个文件本身和
 *    未接线的 `server/prompts/css_concrete_tokens.txt`；main.js 里没有工具定义、没有 dispatch
 *    分支，网关 `tools.json` 的 130 个工具里也没有。模型永远发不出这两个调用，
 *    所以模块里最大的那个分支生下来就是死的。要做就得配齐：工具定义 + dispatch + tools.json
 *    镜像到网关。
 * 2. **`shadcn_ref` 那段文案没有真值来源。** 五条断言全是标题匹配（`/shadcn\/ui Theming Reference/`
 *    之类），钉不住任何一个值。而 `src-tauri/src/web_scaffold.rs` 里发的是本项目自己的 token 词汇
 *    （`--bg`/`--surface`/`--text-faint`），不是 shadcn 的规范集合（`--background`/`--foreground`/
 *    `--card`/`--muted`/`--ring`/`--chart-*`/`--sidebar-*`）。凭模型记忆写一份"权威参考"再当成
 *    指导发给每个 CSS 任务，是在制造幻觉源——**不要照着断言编一份看起来像样的出来**。
 * 3. **有三条断言会删掉线上已有的行为**：
 *    - `deepEqual(calls[0][1], {name, workspace, framework})` 丢掉了 `style`（Material 3 / tdesign
 *      预设，dispatch 在 main.js:30339 解析）和 `tokensCss`（learn_design 的 token 自动接线，
 *      46885-46892）。
 *    - 注入 `refreshFileTree`，而真实代码走的是 `_clearRunEmptyRoot` + `refreshProjectCaches`。
 *    - `assert.match(out.content, /OKLCH/)`——真实返回串里没有 OKLCH 这个词。
 *    另外第 86 行钉的是**改名前**的产品名 `只能在 Michael IDE 桌面 App`，而 main.js 在
 *    commit f0696cf（2026-08-11）已经改成 `Mr. Day One`。满足它等于把改名单点回滚。
 *
 * ## 还有一层结构问题
 * `current_time` / `system` / `game_scaffold` / `web_scaffold` 和 7 个素材工具都不是独立函数，
 * 而是 `_executeToolStepInner(step, call, root, run)`（main.js:45542 起、4520 行、129 个
 * `call.type ===` 分支）里的 `else if` 分支，闭包着 12 个外层变量。从里面抠 2 个分支出来是**重构**，
 * 不是搬运。而且 `workspaceFor` 这个函数不存在——执行器用的是两级兜底
 * `(run?.session?.project) || root || ""`，这份契约写的是三级，多出来的 `list[0]` 会在没有活动项目时
 * **改变文件写到哪里**；契约自己也不自洽（web_scaffold 注入 `root:`，generate_3d 注入 `rootPath:`）。
 *
 * ## 结论
 * 想要这些能力，就当成一个**新功能**立项做全套（工具定义 + dispatch + tools.json 镜像 +
 * codegen 的调色板 + shadcn 文案的真实出处），别为了把这个文件点绿而把代码凑出来。
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  TAILWIND_OKLCH_PALETTE,
  buildCurrentTimeResult,
  buildTailwindPaletteResult,
  executeSystemTool,
} from "../src/agent/executors/system-tools.js";
import { executeGameTool, workspaceFor } from "../src/agent/executors/game-tools.js";

test("system executor handles current_time deterministically", () => {
  const now = new Date("2026-07-10T08:09:10.000Z");
  const res = {};
  const out = executeSystemTool({ type: "current_time" }, {
    res,
    now: () => now,
    resolveTimeZone: () => "UTC",
  });

  assert.equal(out.type, "current_time");
  assert.match(out.content, /当前时间: /);
  assert.match(out.content, /时区: UTC/);
  assert.match(out.content, /ISO: 2026-07-10T08:09:10.000Z/);
  assert.equal(res.className, "atc-result atc-result--title");
});

test("system executor exposes shadcn and Tailwind references", () => {
  const shadcn = executeSystemTool({ type: "shadcn_ref" }, { res: {} });
  assert.match(shadcn.content, /shadcn\/ui Theming Reference/);
  assert.match(shadcn.content, /OKLCH/);
  assert.match(shadcn.content, /npx shadcn@latest add/);
  assert.match(shadcn.content, /@theme inline/);
  assert.match(shadcn.content, /Dashboard \/ IDE/);

  const emerald = buildTailwindPaletteResult("emerald");
  assert.match(emerald, /emerald-500: oklch\(69.6% 0.17 162.48\)/);
  assert.match(emerald, /shadcn 语义 token 配方/);
  assert.match(emerald, /--primary: oklch\(59.6% 0.145 163.225\)/);
  assert.match(emerald, /ring-emerald-500/);

  const overview = buildTailwindPaletteResult("");
  assert.match(overview, /26 色族/);
  assert.match(overview, /olive/);
  assert.match(overview, /mist/);
  assert.match(overview, /neutral \+ accent/);
  assert.equal(TAILWIND_OKLCH_PALETTE.olive["500"], "58% 0.031 107.3");

  const unknown = buildTailwindPaletteResult("missing");
  assert.match(unknown, /未知色族/);

  const current = buildCurrentTimeResult(new Date("2026-01-02T03:04:05.000Z"), () => "UTC");
  assert.match(current.content, /Unix: /);
});

test("system executor routes desktop system controls through backend", async () => {
  const calls = [];
  const res = {};
  const vp = {};
  const backend = {
    async invoke(name, args) {
      calls.push([name, args]);
      return { ok: true, app: args?.name || "Michael" };
    },
  };

  const out = await executeSystemTool({ type: "system", op: "open", name: "Safari", background: true }, {
    inTauri: true,
    backend,
    res,
    vp,
    escapeHtml: (s) => s,
  });

  assert.deepEqual(calls[0], ["system_open_app", { name: "Safari", background: true }]);
  assert.equal(res.textContent, "system open");
  assert.match(vp.innerHTML, /Safari/);
  assert.match(out.content, /后台启动/);
});

test("system executor blocks desktop system controls outside Tauri", async () => {
  const out = await executeSystemTool({ type: "system", op: "apps" }, {
    inTauri: false,
    res: {},
  });

  assert.match(out.content, /只能在 Michael IDE 桌面 App/);
});

test("game executor blocks desktop-only tools outside Tauri", async () => {
  const res = {};
  const out = await executeGameTool({ type: "game_scaffold", name: "demo" }, { inTauri: false, res });

  assert.deepEqual(out, {
    type: "game_scaffold",
    path: "",
    content: "[不可用] 游戏脚手架只能在桌面 App 里用。",
  });
  assert.equal(res.textContent, "桌面专用");
});

test("game executor routes web scaffold through injected backend", async () => {
  const calls = [];
  let refreshed = false;
  const res = {};
  const vp = {};
  const backend = {
    async invoke(name, args) {
      calls.push([name, args]);
      return { path: "/ws/site", framework: "react", next: "npm run dev" };
    },
  };

  const out = await executeGameTool({ type: "web_scaffold", name: "site", framework: "react" }, {
    inTauri: true,
    backend,
    root: "/ws",
    res,
    vp,
    refreshFileTree: () => { refreshed = true; },
    escapeHtml: (s) => s,
  });

  assert.equal(calls[0][0], "web_scaffold");
  assert.deepEqual(calls[0][1], { name: "site", workspace: "/ws", framework: "react" });
  assert.equal(out.type, "web_scaffold");
  assert.equal(out.path, "/ws/site");
  assert.match(out.content, /OKLCH/);
  assert.equal(res.textContent, "/ws/site");
  assert.ok(vp.innerHTML.includes("framework"));
  assert.equal(refreshed, true);
});

test("game executor passes config and auth to asset generation", async () => {
  let seen = null;
  const backend = {
    async invoke(name, args) {
      seen = { name, args };
      return { path: "/ws/assets/model.glb", bytes: 123 };
    },
  };

  const out = await executeGameTool({ type: "generate_3d", prompt: "hero", name: "hero", style: "lowpoly" }, {
    inTauri: true,
    backend,
    rootPath: "/ws",
    loadConfig: () => ({ baseUrl: "https://api.example", apiKey: "cfg-key", model: "trellis" }),
    localStorage: { getItem: () => "user-token" },
    res: {},
  });

  assert.equal(seen.name, "generate_3d");
  assert.equal(seen.args.baseUrl, "https://api.example");
  assert.equal(seen.args.apiKey, "user-token");
  assert.equal(seen.args.workspace, "/ws");
  assert.equal(seen.args.model, "trellis");
  assert.equal(out.path, "/ws/assets/model.glb");
});

test("workspaceFor keeps executor workspace selection compatible", () => {
  assert.equal(workspaceFor("/active", "/root", ["/first"]), "/active");
  assert.equal(workspaceFor("", "/root", ["/first"]), "/root");
  assert.equal(workspaceFor("", "", ["/first"]), "/first");
  assert.equal(workspaceFor("", "", []), "");
});
