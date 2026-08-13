// 用户规则：跨项目的长期要求。
//
// 项目级约定这个 IDE 一直会读（AGENTS.md / CLAUDE.md / .cursorrules，见 _gatherAgentContext），
// 但**用户级的一直没有**。「我一律用 pnpm」「回答用中文」「改完先跑测试」这类要求跟着人走、
// 不跟着项目走，以前只能每个项目重抄一遍，或者每轮对话重复一次。
//
// 存 ~/.michael-ide/rules.md（和 mcp.json 同目录，0600），进每一轮的系统提示词。
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const SRC = fs.readFileSync("src/main.js", "utf8");

function extractFn(name) {
  const m = new RegExp(`(?:async\\s+)?function\\s+${name}\\s*\\(`).exec(SRC);
  if (!m) throw new Error(`function ${name} not found`);
  let i = SRC.indexOf("{", SRC.indexOf(")", m.index)), depth = 0;
  for (; i < SRC.length; i++) {
    const c = SRC[i], d = SRC[i + 1];
    if (c === "/" && d === "/") { i = SRC.indexOf("\n", i); if (i < 0) i = SRC.length; continue; }
    if (c === "/" && d === "*") { i = SRC.indexOf("*/", i + 2) + 1; continue; }
    if (c === "'" || c === '"' || c === "`") {
      const q = c;
      for (i++; i < SRC.length; i++) { if (SRC[i] === "\\") { i++; continue; } if (SRC[i] === q) break; }
      continue;
    }
    if (c === "{") depth++;
    else if (c === "}") { depth--; if (depth === 0) return SRC.slice(m.index, i + 1); }
  }
  throw new Error(`unbalanced braces in ${name}`);
}
const block = (text, max = 4000) =>
  new Function("_userRulesText", "_USER_RULES_MAX", `${extractFn("_userRulesBlock")};return _userRulesBlock();`)(text, max);

test("写了规则就进提示词，原文一字不改", () => {
  const out = block("回答用中文。\n包管理器一律用 pnpm。");
  assert.match(out, /回答用中文。/);
  assert.match(out, /包管理器一律用 pnpm。/);
});

test("没写规则时一个字节都不占——不能让空规则白付每轮的钱", () => {
  for (const empty of ["", "   ", "\n\n\t ", null, undefined]) {
    assert.equal(block(empty), "", `${JSON.stringify(empty)} 应该产出空串`);
  }
});

test("块里要说清它是谁写的、和项目约定谁大", () => {
  const out = block("x");
  assert.match(out, /用户规则/);
  assert.match(out, /高于项目约定/, "不说清优先级，模型碰上冲突只能瞎猜");
  assert.match(out, /本轮/, "本轮的明确指令必须能压过长期规则，否则用户改不动主意");
});

test("超长会截断并说明，而不是整段丢掉", () => {
  const out = block("A".repeat(50) + "B".repeat(50), 60);
  assert.ok(out.includes("A".repeat(50)), "前半段要保住");
  assert.ok(!out.includes("B".repeat(50)), "超出的部分不该进去");
  assert.match(out, /截断/);
  assert.match(out, /精简/, "要告诉用户怎么办，不能只说被截了");
});

test("轻量轮也带用户规则——闲聊轮才是最容易破规矩的地方", () => {
  // 轻量轮省的是系统提示词和工作区预热；「回答用中文」这种要求恰恰在寒暄时最该生效。
  assert.match(SRC, /const fullPrompt = _agentLightTurn \? \(sysPrompt \+ userRulesBlock \+/);
  assert.match(SRC, /: \(sysPrompt \+ _modelStyleTuning\(config\.model\) \+ userRulesBlock \+/);
});

test("保存后立刻生效，不用等重启", () => {
  const panel = SRC.slice(SRC.indexOf("async function openUserRulesPanel"), SRC.indexOf("async function openMcpPanel"));
  assert.match(panel, /_userRulesText = area\.value;/,
    "保存成功要同步更新内存里那份，否则用户改完发现没变化，会以为没存上");
  assert.match(panel, /user_rules_save/);
  assert.match(panel, /留空 = 不启用/);
});

// ── ⋮ 菜单 ────────────────────────────────────────────────────────────────

test("菜单里是「技能」和「用户规则」，MCP 那项已移走", () => {
  const shell = fs.readFileSync("src/app/Shell.jsx", "utf8");
  assert.match(shell, /capabilitySkillsItem/);
  assert.match(shell, /capabilityRulesItem/);
  assert.ok(!shell.includes("capabilityMcpItem"), "MCP 项已删（高级设置 → MCP 仍可达）");
  assert.match(SRC, /_rulesItem\.addEventListener\("click"[\s\S]{0,120}openUserRulesPanel/);
});

test("对齐算法：宽窗口正居中，贴边时夹进视口，绝不跑出屏幕", () => {
  const body = /const _alignCapabilitiesMenu = \(\) => \{[\s\S]*?\n  \};/.exec(SRC)[0];
  const place = (winW, btnCenter, menuW, wrapLeft) => {
    const _menu = { offsetWidth: menuW, style: {} };
    const _capBtn = { getBoundingClientRect: () => ({ left: btnCenter - 14, right: btnCenter + 14 }) };
    const _wrap = { getBoundingClientRect: () => ({ left: wrapLeft }) };
    new Function("_menu", "_capBtn", "_wrap", "window", `${body}; _alignCapabilitiesMenu();`)(
      _menu, _capBtn, _wrap, { innerWidth: winW });
    return parseInt(_menu.style.left, 10) + wrapLeft;
  };
  // 有地方就真的居中
  assert.equal(place(1600, 800, 148, 700) + 74, 800);
  // 按钮贴着右缘（这就是真实情况：实测按钮中心离窗口右缘只有 30px，居中会溢出 58px）
  const right = place(1280, 1250, 148, 1236);
  assert.ok(right + 148 <= 1280, `右边跑出去了：${right + 148}`);
  assert.ok(right > 1000, "也不该被甩到左边去");
  // 极端：视口比菜单还窄 / innerWidth 报 0。夹紧的上下界会反过来——顺序写错就会把菜单
  // 推到屏幕外（第一版就是这么错的）。
  assert.equal(place(100, 50, 148, 40), 8);
  assert.equal(place(0, 558, 148, 544), 8);
});

// ── 冷启动：规则必须在启动时读一次 ────────────────────────────────────────
//
// 这是上一版真实存在的 bug，也是「内容要真实实现」最要命的一处：`_refreshUserRules()`
// 写了但**一个调用者都没有**。`_userRulesText` 初值是空串，唯一填它的地方是打开规则面板
// 时那段内联读取。于是重启 App 之后，用户上周写的规则一轮都没进过提示词——而且
// `_userRulesBlock()` 只会安静返回空串，不报错、没有任何迹象。菜单上那句"每轮都遵守"
// 在冷启动后是假的。

test("启动时必须读一次用户规则，否则重启后规则一轮都不生效", () => {
  const calls = [...SRC.matchAll(/_refreshUserRules\(\)/g)];
  assert.ok(calls.length >= 2,
    `_refreshUserRules 除了定义之外必须有调用点，实际只出现 ${calls.length} 次`);
  // 就在启动段里，和 initLocale 一起
  assert.match(SRC, /initLocale\(\);[\s\S]{0,600}void _refreshUserRules\(\);/,
    "要在启动段调用——放在别处（比如首轮发送时）会让第一轮仍然是空的");
});

test("菜单打开时把状态画到行上——删掉副标题后这是唯一能看出状态的地方", () => {
  assert.match(SRC, /_skillItem\?\.classList\.toggle\("is-active", _activeSkillIds\.size > 0\)/);
  assert.match(SRC, /_rulesItem\?\.classList\.toggle\("is-active", String\(_userRulesText \|\| ""\)\.trim\(\)\.length > 0\)/);
  // 必须在菜单显示之前刷，否则第一次打开看到的是上一次的状态
  assert.match(SRC, /_paintCapabilityState\(\); _menu\.hidden = false;/);
  const css = fs.readFileSync("src/styles/app.css", "utf8");
  assert.match(css, /\.assistant-capability__item\.is-active \{ color: var\(--accent\); \}/);
});

test("role=\"menu\" 要配得上这个名字：方向键能走", () => {
  // 声明了 role="menu" 却只处理 Escape，屏幕阅读器会播报一个"菜单"然后发现它不像菜单。
  assert.match(SRC, /const _moveCapabilityFocus = \(delta\) => \{/);
  for (const key of ["ArrowDown", "ArrowUp", "Home", "End"]) {
    assert.ok(SRC.includes(`ev.key === "${key}"`), `菜单键盘契约缺 ${key}`);
  }
});

// ── 版式：单行，向自家其它下拉看齐 ────────────────────────────────────────

test("菜单收窄变矮，且不再是全应用唯一的两行排版", () => {
  const css = fs.readFileSync("src/styles/app.css", "utf8");
  const menu = /\.assistant-capability__menu\s*\{([^}]*)\}/.exec(css);
  assert.ok(menu, "找不到菜单规则");
  const width = /min-width:\s*(\d+)px/.exec(menu[1]);
  assert.ok(width && Number(width[1]) <= 120, `菜单还是太宽：${width?.[1]}px`);

  const item = /\.assistant-capability__item\s*\{([^}]*)\}/.exec(css);
  const pad = /padding:\s*(\d+)px/.exec(item[1]);
  assert.ok(pad && Number(pad[1]) <= 7, `每项还是太高：padding ${pad?.[1]}px`);

  // 副标题那一层整个不存在了——三个 class 在 CSS / JSX / JS 里都不该再出现
  const shell = fs.readFileSync("src/app/Shell.jsx", "utf8");
  for (const cls of ["__item-sub", "__item-main", "__item-title"]) {
    assert.ok(!css.includes(`assistant-capability${cls}`), `CSS 里还留着 ${cls}`);
    assert.ok(!shell.includes(`assistant-capability${cls}`), `JSX 里还留着 ${cls}`);
  }
});

test("菜单文案走真正的 i18n 键，不靠启发式翻译；三份字典都齐", () => {
  const shell = fs.readFileSync("src/app/Shell.jsx", "utf8");
  assert.match(shell, /data-i18n="assistant\.capability\.skills"/);
  assert.match(shell, /data-i18n="assistant\.capability\.rules"/);
  // 裸中文文本节点会走 localizeLooseTextNode 的启发式通道——那条路曾经被幻觉翻译污染过。
  const i18n = fs.readFileSync("src/i18n.js", "utf8");
  for (const key of ["assistant.capability.skills", "assistant.capability.rules"]) {
    const n = [...i18n.matchAll(new RegExp(`"${key.replace(/\./g, "\\.")}"\\s*:`, "g"))].length;
    assert.equal(n, 3, `${key} 应在 EN/ZH/JA 三份字典里各一条，实际 ${n} 条`);
  }
  // 同名键重复会被后写的覆盖，等于改了个寂寞
  assert.ok(!i18n.includes("assistant.capability.skillsSub"), "副标题的键该随排版一起删掉");
  assert.ok(!i18n.includes("assistant.capability.mcpSub"), "MCP 项已移出菜单，它的键也该走");
  // 按钮说明词不能还写着 MCP
  assert.ok(!/capabilities\.open": "[^"]*MCP/.test(i18n), "按钮说明还写着 MCP，但菜单里早就没有了");
});

test("第一项叫「技能」而不是「全局技能」——那个「全局」是句错话", () => {
  // 技能面板同时会扫项目目录（_skillDiscoveryBases 走工作区、父仓库、用户目录），
  // 点进去第一眼就能看见项目技能，「所有项目通用」是假的。
  const i18n = fs.readFileSync("src/i18n.js", "utf8");
  assert.match(i18n, /"assistant\.capability\.skills": "技能"/);
  assert.ok(!i18n.includes("全局技能"), "「全局技能」名不副实");
  assert.match(SRC, /_skillDiscoveryBases/, "技能发现确实覆盖项目级目录，所以名字不能带「全局」");
});
