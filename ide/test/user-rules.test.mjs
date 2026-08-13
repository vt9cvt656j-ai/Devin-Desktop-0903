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

test("菜单里是「全局技能」和「用户规则」，MCP 那项已移走", () => {
  const shell = fs.readFileSync("src/app/Shell.jsx", "utf8");
  assert.match(shell, /全局技能/);
  assert.match(shell, /用户规则/);
  assert.match(shell, /capabilityRulesItem/);
  assert.ok(!shell.includes("capabilityMcpItem"), "MCP 项已删（高级设置 → MCP 仍可达）");
  // 不能用 [^)]*：中间隔着 _closeCapabilitiesMenu() 的那对括号。
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
