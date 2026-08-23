// 模型把裁决 JSON **拍平**时，整个工程半被静默丢掉——而且账本还记 ok。
//
// 实测（2026-08-23，跑真 _normalizeAiIntentVerdict）：
//   带 engineering 壳 → domain=healthcare / researchMode=official / architectureMode=design_new
//   同内容**扁平**    → domain=""        / researchMode=none     / architectureMode=none
//                       而 _halves.engineering 仍然报 true —— 一边说「工程半到了」，
//                       一边把 18 个枚举全退成默认值。
//   扁平且没有 dimensions → **整份 NULL**
//
// 成因是取值方式不对称：dimensions 和 semantic 都有扁平回退，只有 engineering 没有。
// 而发给模型的那句话是「**只输出这一个对象**，不要输出其它顶层字段」，示例里却有
// engineering 和 dimensions 两个顶层键——指令和示例互相打架，拍平是模型最自然的解法。
import { test } from "node:test";
import assert from "node:assert/strict";
import { CODE as SRC, load, loadConst, fnSource as topLevelFn } from "./helpers/source.mjs";

const FIELDS = loadConst("_AI_ENGINEERING_FIELDS");
const SEMANTIC_FIELDS = ["goal", "action", "target", "locationIntent", "continuation",
  "constraints", "successCriteria", "ambiguities"];

function normalizer() {
  const intentText = load("_aiIntentText");
  const deps = {
    _AI_INTENT_DIMENSIONS: loadConst("_AI_INTENT_DIMENSIONS"),
    _AI_ENGINEERING_FIELDS: FIELDS,
    _AI_INTENT_RELATIONS: new Set(["new", "continue", "correct", "replace", "clarify"]),
    _AI_PROJECT_STATES: new Set(["none", "existing", "greenfield", "unknown"]),
    _AI_DELIVERY_SURFACES: new Set(["answer", "code", "ui_component", "website", "web_app", "backend", "data", "cli", "desktop", "automation", "mixed"]),
    _AI_CHANGE_SCOPES: new Set(["none", "local", "module", "project", "system"]),
    _AI_ARCHITECTURE_MODES: new Set(["none", "follow_existing", "extend_existing", "design_new", "refactor_existing"]),
    _AI_DATA_STRATEGIES: new Set(["not_applicable", "none", "local", "server", "inspect_existing", "undecided"]),
    _AI_RESEARCH_MODES: new Set(["none", "official", "community", "official_and_community"]),
    _AI_DESIGN_MODES: new Set(["none", "michael_design_2_5_existing", "michael_design_2_5_greenfield"]),
    _AI_WORKSPACE_ACTIONS: new Set(["none", "inspect", "modify"]),
    _AI_CAPTURE_MODES: new Set(["none", "isolated_browser", "system", "background"]),
    _AI_BROWSER_GOALS: new Set(["none", "static", "interactive", "network_capture"]),
    _AI_ORCHESTRATION_MODES: new Set(["solo", "staged_roles", "parallel_roles"]),
    _AI_AGENT_ROLES: loadConst("_AI_AGENT_ROLES"),
    _AI_KNOWLEDGE_DOMAINS: loadConst("_AI_KNOWLEDGE_DOMAINS"),
    _aiIntentText: intentText,
    _aiIntentList: load("_aiIntentList", { _aiIntentText: intentText }),
    _aiIntentEnum: load("_aiIntentEnum", { _aiIntentText: intentText }),
    _aiIntentKnowledgeDomain: load("_aiIntentKnowledgeDomain", { _AI_KNOWLEDGE_DOMAINS: loadConst("_AI_KNOWLEDGE_DOMAINS") }),
    _userRoleMap: () => new Map(),
  };
  for (let i = 0; i < 40; i++) {
    try { return load("_normalizeAiIntentVerdict", deps); }
    catch (e) {
      const m = /(\w+) is not defined/.exec(String(e?.message));
      if (!m) throw e;
      try { deps[m[1]] = loadConst(m[1]); } catch { try { deps[m[1]] = load(m[1]); } catch { deps[m[1]] = () => null; } }
    }
  }
  throw new Error("装不起来");
}
const norm = normalizer();
const ENG = { projectState: "existing", domain: "healthcare", researchMode: "official", architectureMode: "design_new" };
const pick = (v) => v && { halves: v._halves, ...v.engineering };

// ── 一、拍平的和带壳的必须逐字段一致 ──────────────────────────────────
test("模型把工程半拍平，字段一个都不能丢", () => {
  const shaped = norm({ engineering: { ...ENG }, dimensions: { implementation: true } });
  const flat = norm({ ...ENG, dimensions: { implementation: true } });
  for (const k of ["domain", "researchMode", "architectureMode", "projectState"]) {
    assert.equal(flat.engineering[k], shaped.engineering[k],
      `拍平之后 ${k} 变了：带壳=${shaped.engineering[k]} 扁平=${flat.engineering[k]}`);
  }
});

test("拍平且没有 dimensions 时也不能整份丢掉", () => {
  const v = norm({ ...ENG });
  assert.ok(v, "整份 NULL——模型明明答了，只是没套壳");
  assert.equal(v.engineering.domain, "healthcare");
});

test("只给一个工程字段也算工程半到场", () => {
  const v = norm({ domain: "healthcare" });
  assert.ok(v, "只回了 domain 就被整份丢掉");
  assert.equal(v._halves.engineering, true);
});

// ── 二、消歧不是放松：语义半不许被误判成工程半 ──────────────────────
test("纯语义半的扁平输入，工程半仍然判为缺席", () => {
  // 这是「消歧不是放松」的判据。放松了的话，一个只答了语义的回合会被记成
  // 「两半都到 = ai」，从而拿到完整裁决才配有的夺能力权限。
  const v = norm({ goal: "看看这个函数干嘛的", action: "inspect", continuation: "new" });
  assert.ok(v, "语义半自己应当仍然成立");
  assert.equal(v._halves.engineering, false,
    "语义半被误判成工程半了——半份裁决会冒充完整裁决");
});

test("把门的是**字段名单检查**，不是 rawEngineering 非空", () => {
  // 这条是「消歧不是放松」真正的落点。把 rawEngineering 的回退放宽成 `: value`，
  // 上面那条语义半断言照样绿——因为真正决定 _halves.engineering 的是字段名单那道检查
  // （2026-08-23 变异实测确认无害）。所以要钉的是这道检查本身。
  const body = topLevelFn("_normalizeAiIntentVerdict", { code: true });
  assert.match(body, /engineering: hasEngineeringInput \|\| hasDimensionInput/,
    "_halves.engineering 的来源变了——若改成 !!rawEngineering，语义半会冒充工程半");
  assert.match(body, /hasEngineeringInput = !!rawEngineering\s*\n?\s*&& _AI_ENGINEERING_FIELDS\.some/,
    "字段名单那道检查没了——顶层只要有任何键都会被当成工程半到场");
});

test("两份名单零重叠，这是上一条成立的前提", () => {
  const overlap = FIELDS.filter((f) => SEMANTIC_FIELDS.includes(f));
  assert.deepEqual(overlap, [],
    `名单重叠：${overlap.join(",")}——拿工程名单认顶层会把语义半误判成工程半`);
});

// ── 三、名单只此一份 ──────────────────────────────────────────────────
test("字段名单是模块级常量，源码里没有第二份手抄", () => {
  // 今天判定处和赋值处曾是两份手抄副本。漏跟一次就出现「读得到却判成没到」。
  const dup = SRC.match(/"projectState", "deliverySurface", "changeScope"/g) || [];
  assert.equal(dup.length, 1, `出现了 ${dup.length} 份字段名单——它们会漂`);
  assert.match(SRC, /const _AI_ENGINEERING_FIELDS = \[/);
  const body = topLevelFn("_normalizeAiIntentVerdict", { code: true });
  assert.match(body, /_AI_ENGINEERING_FIELDS\.some/, "判定处没用那份常量");
});

test("名单覆盖裁决真正会产出的工程字段", () => {
  for (const f of ["domain", "researchMode", "architectureMode", "orchestrationMode", "dataStrategy", "workspaceAction"]) {
    assert.ok(FIELDS.includes(f), `${f} 不在名单里——拍平时它会被丢掉`);
  }
});

// ── 四、数组不许混进合并 ────────────────────────────────────────────
test("解析器回数组时判死，不许 spread 进合并结果", () => {
  // _safeJsonLoose 在某些截断形状上会回数组，而数组是 truthy：
  // spread 之后 [] → {}、["a"] → {"0":"a"}，这一半必然归零，账本却照样记 ok。
  const at = SRC.indexOf("const _semObj =");
  assert.ok(at > 0, "数组守卫不见了");
  const seg = SRC.slice(at, at + 500);
  assert.match(seg, /!Array\.isArray\(_sem\)/, "语义半没挡数组");
  assert.match(seg, /!Array\.isArray\(_eng\)/, "工程半没挡数组");
  assert.match(seg, /_merged = \(_semObj \|\| _engObj\)/, "合并用的还是没过滤的那两个");
});

test("守卫只挡数组，不挡正常对象", () => {
  const v = norm({ engineering: { ...ENG } });
  assert.equal(v.engineering.domain, "healthcare", "守卫把正常对象也挡掉了");
});
