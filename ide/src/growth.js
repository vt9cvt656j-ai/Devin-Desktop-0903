// ---------------------------------------------------------------------------
// Growth — an adaptive "learner model" that grows with the user.
//
// This is a small Intelligent-Tutoring-System (ITS) living inside the IDE:
//   • a *learner model* — a per-skill mastery estimate of THIS user, updated
//     from their real behaviour via Bayesian Knowledge Tracing (BKT);
//   • a *pedagogical layer* — it turns that model into adaptive teaching by
//     injecting a tailored block into the AI's system prompt (Lever 1).
//
// Design goals (grounded in the literature, see docs/growth-system.md):
//   • Low floor: a brand-new user starts "novice everywhere", so the AI
//     explains richly out of the box — beginners onboard with zero setup.
//   • High ceiling: as mastery rises, scaffolding *fades* (expertise-reversal
//     effect — explanations that help novices only annoy experts).
//   • "越勇越厉害" for real: the signals reward *cognitive engagement*
//     (reviewing diffs, catching bad edits, planning, writing your own code),
//     not blind delegation — directly countering AI-tool deskilling.
//   • Open Learner Model: the user can see and correct the model's beliefs,
//     which respects autonomy (Self-Determination Theory) and builds trust.
//
// Self-contained: persists to localStorage (works in the native app AND in
// `npm run dev` browser-mock mode), injects its own styles, and every public
// entry point is wrapped so a bug here can never break the chat hot-path.
// ---------------------------------------------------------------------------

const STORE_KEY = "michael-ide.learner-model.v1";

// The skill graph. Each skill is fed by real, captured behavioural signals
// (see applyEvent below) — no padded/dead bars. `coach` is the instruction the
// AI receives while this skill is still WEAK; it fades away once mastered.
const SKILLS = [
  {
    id: "reviewing",
    label: "审查 AI 产出",
    blurb: "看懂并判断 AI 改了什么、对不对",
    coach: "做完每个关键改动后，用一句话说清「改了什么 / 为什么」，并提示一个值得检查的点；不要让用户闭眼接受。",
  },
  {
    id: "authoring",
    label: "独立投入",
    blurb: "自己思考、不把活全外包给 AI",
    coach: "适当留白：对不太难的部分，鼓励用户自己动手，而不是直接把完整代码端上来。",
  },
  {
    id: "prompting",
    label: "表达需求",
    blurb: "把想要的东西讲清楚",
    coach: "当用户的需求含糊时，先用一句话复述你的理解、并补一个澄清问题，再动手。",
  },
  {
    id: "planning",
    label: "任务规划",
    blurb: "把复杂任务拆成步骤",
    coach: "面对多步任务，先给出简短的分步计划再实现，帮助用户建立拆解的习惯。",
  },
  {
    id: "tooling",
    label: "掌握 IDE 能力",
    blurb: "用上 plan / agent / @文件 等高级能力",
    coach: "在合适时机，顺带点出一个能更快完成此类任务的 IDE 功能（如 plan 模式、@文件）。",
  },
  {
    id: "verifying",
    label: "验证习惯",
    blurb: "改完会跑测试/诊断确认，而不是直接信",
    coach: "改完主动用 run_cmd 跑构建/测试/类型检查、或用 get_diagnostics 自检；没验证过的「做完了」别当完成。",
  },
];
const SKILL_IDS = SKILLS.map((s) => s.id);

// Export avgMastery for external access (runtime policy lookup)
export function getAvgMastery() {
  try { load(); return avgMastery(); } catch { return 0.5; }
}

// --- Bayesian Knowledge Tracing ------------------------------------------------
// Classic 4-parameter BKT: a latent "mastered?" probability updated by each
// observation. Interpretable and cheap — deep/LSTM knowledge tracing is overkill
// for a single local user.
const BKT = { pInit: 0.25, pLearn: 0.12, pSlip: 0.1, pGuess: 0.2, forgetPerDay: 0.015 };

function clamp(x, lo, hi) { return Math.max(lo, Math.min(hi, x)); }

function bktUpdate(p, correct) {
  const { pSlip, pGuess, pLearn } = BKT;
  const post = correct
    ? (p * (1 - pSlip)) / (p * (1 - pSlip) + (1 - p) * pGuess)
    : (p * pSlip) / (p * pSlip + (1 - p) * (1 - pGuess));
  return clamp(post + (1 - post) * pLearn, 0.02, 0.98);
}

// Forgetting: gentle decay toward the prior for skills not exercised lately
// (spacing / desirable-difficulty intuition — unused skills fade).
function applyForgetting(skill, now) {
  if (!skill.last) return;
  const days = (now - skill.last) / 86400000;
  if (days > 1) skill.p = clamp(skill.p - BKT.forgetPerDay * (days - 1), BKT.pInit * 0.6, 0.98);
}

// --- state --------------------------------------------------------------------

function freshState() {
  const skills = {};
  for (const id of SKILL_IDS) skills[id] = { p: BKT.pInit, n: 0, last: 0 };
  return {
    v: 1,
    skills,
    prefs: { explain: "auto", challenge: false },   // explain: auto | min | rich
    stats: { messages: 0, aiEdits: 0, reviews: 0, undos: 0, reverts: 0, blind: 0, predicts: 0, predictHits: 0 },
    trend: [],                                       // [{ msgs, avg, t }]
    sessionModes: [],                               // modes seen this run (tooling breadth)
    projects: {},                                    // root -> { name, first, last, turns, touched:{skill:count} }
  };
}

let state = null;
// The project (workspace root) the user is currently acting in. Set on every
// message-sent so that skill practice can be attributed across projects — the
// basis for "transferable" mastery (a skill proven in many projects is yours to
// keep; varied practice → better transfer, per Bjork).
let currentProject = null;
// Per-turn ledger, reconciled at the start of the next turn so that AI edits the
// user never looked at count as "blind accepts" (the deskilling signal).
let turn = { applied: 0, reviewed: 0, engaged: false };
// At most one "predict-first" challenge gate per turn, so it teaches without nagging.
let gatedThisTurn = false;

function load() {
  if (state) return state;
  try {
    const raw = localStorage.getItem(STORE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      const fresh = freshState();
      // Deep-merge the nested objects: a shallow spread would let an OLDER save's
      // partial `stats`/`prefs` replace the whole fresh object, leaving new fields
      // undefined — then `stats.blind += n` → NaN, which persists and poisons the
      // stats forever. Merge field-by-field so upgrades always have every key.
      state = {
        ...fresh,
        ...parsed,
        stats: { ...fresh.stats, ...(parsed.stats || {}) },
        prefs: { ...fresh.prefs, ...(parsed.prefs || {}) },
        skills: { ...fresh.skills, ...(parsed.skills || {}) },
      };
      // heal: ensure every known skill exists (graph may grow across versions)
      for (const id of SKILL_IDS) if (!state.skills[id]) state.skills[id] = { p: BKT.pInit, n: 0, last: 0 };
    }
  } catch { /* corrupt / unavailable — fall through to fresh */ }
  if (!state) state = freshState();
  // "This run" means THIS app launch — the persisted copy carries stale modes from
  // past sessions, which silenced the tooling signal forever (first-use credit could
  // never fire again). Reset on load so each launch re-credits feature breadth.
  state.sessionModes = [];
  return state;
}

function save() {
  // Synchronous: the payload is tiny and signals fire on user actions (not per
  // token), so there's no perf reason to debounce — and a debounce risks losing
  // the last turn if the app closes right after.
  try { localStorage.setItem(STORE_KEY, JSON.stringify(state)); } catch { /* quota / unavailable */ }
}

function observe(skillId, correct) {
  const s = state.skills[skillId];
  if (!s) return;
  const now = Date.now();
  applyForgetting(s, now);
  s.p = bktUpdate(s.p, correct);
  s.n += 1;
  s.last = now;
  // Attribute this practice to the current project, so we can tell how broadly
  // (across how many projects) a skill has been exercised.
  const proj = currentProject && state.projects[currentProject];
  if (proj) proj.touched[skillId] = (proj.touched[skillId] || 0) + 1;
}

function avgMastery() {
  const vals = SKILL_IDS.map((id) => state.skills[id].p);
  return vals.reduce((a, b) => a + b, 0) / vals.length;
}

// How many distinct projects a skill has been genuinely practiced in (≥2 reps,
// so a single incidental touch doesn't count) — its transfer breadth.
function skillBreadth(skillId) {
  let n = 0;
  for (const root in state.projects) if ((state.projects[root].touched[skillId] || 0) >= 2) n++;
  return n;
}
function projectCount() { return Object.keys(state.projects).length; }
// "Transferable" = both fairly mastered AND proven across ≥2 projects.
function isTransferable(skillId) { return state.skills[skillId].p >= 0.6 && skillBreadth(skillId) >= 2; }
function transferableCount() { return SKILL_IDS.filter(isTransferable).length; }

function snapshotTrend() {
  const t = state.trend;
  const point = { msgs: state.stats.messages, avg: +avgMastery().toFixed(3), t: Date.now() };
  const last = t[t.length - 1];
  if (!last || point.msgs !== last.msgs) t.push(point);
  if (t.length > 40) t.shift();
}

// Close out the previous turn: any AI edit the user never opened is a blind
// accept → weak negative evidence for "reviewing" (and disengagement for
// "authoring"). This is what makes over-reliance visibly cost mastery.
function reconcileTurn() {
  const blind = Math.max(0, turn.applied - turn.reviewed);
  if (blind > 0) {
    state.stats.blind += blind;
    const hits = Math.min(blind, 3);            // cap so one big run can't tank the estimate
    for (let i = 0; i < hits; i++) observe("reviewing", false);
    observe("authoring", false);
  } else if (turn.applied > 0 || turn.engaged) {
    observe("authoring", true);                 // they engaged with the work this turn
  }
  turn = { applied: 0, reviewed: 0, engaged: false };
  gatedThisTurn = false;                         // allow one predict-gate next turn
}

// --- public: record a behavioural signal --------------------------------------

export function signal(type, payload = {}) {
  try {
    load();
    switch (type) {
      case "message-sent": {
        reconcileTurn();                          // closes prior turn (still attributed to the prior project)
        state.stats.messages += 1;
        const proj = payload.project || "";
        if (proj) {
          currentProject = proj;
          const p = state.projects[proj] ||
            (state.projects[proj] = { name: payload.projectName || proj, first: Date.now(), last: 0, turns: 0, touched: {} });
          if (payload.projectName) p.name = payload.projectName;
          p.last = Date.now();
          p.turns += 1;
        }
        const len = payload.len || 0;
        // Sub-15-char turns are acks/continuations ("继续", "好"), not prompting
        // attempts — scoring them as failures unfairly tanked the mastery estimate.
        if (len >= 15) observe("prompting", len >= 60); // detailed asks vs one-liners
        if (payload.mode === "plan") { observe("planning", true); turn.engaged = true; }
        if (payload.complex && payload.mode === "agent") observe("planning", true); // 勇于接复杂任务
        if (payload.mode === "chat" || payload.mode === "plan") turn.engaged = true; // thinking, not autopiloting
        // tooling breadth — credit the first use of each mode/feature this run
        const feats = [payload.mode, ...(payload.usedAt ? ["at"] : [])].filter(Boolean);
        for (const f of feats) {
          if (!state.sessionModes.includes(f)) { state.sessionModes.push(f); observe("tooling", true); }
        }
        snapshotTrend();
        break;
      }
      case "review-diff":
        observe("reviewing", true);
        turn.reviewed += 1; turn.engaged = true;
        state.stats.reviews += 1;
        break;
      case "edit-applied":
        turn.applied += 1;
        state.stats.aiEdits += 1;
        break;
      case "undo-edit":
        observe("reviewing", true);               // caught a bad edit = strong engagement
        turn.reviewed += 1; turn.engaged = true;
        state.stats.undos += 1;
        break;
      case "run-complete":
        // An agent run that edited files: did it verify its own work (tests /
        // build / diagnostics) or ship unverified? The beneficial-usage signal.
        observe("verifying", !!payload.verified);
        break;
      case "predict":
        // "你先猜": the user committed to a guess before seeing the AI's diff —
        // retrieval practice + desirable difficulty, the strongest learning signal.
        observe("authoring", !!payload.hit);      // did their mental model match?
        observe("reviewing", true);               // they engaged & self-assessed
        turn.reviewed += 1; turn.engaged = true;
        state.stats.predicts += 1;
        if (payload.hit) state.stats.predictHits += 1;
        break;
    }
    save();
  } catch { /* a telemetry bug must never break the editor */ }
}

// --- public: the "你先猜" (predict-first) challenge gate -----------------------

export function challengeOn() {
  try { load(); return !!state.prefs.challenge; } catch { return false; }
}

// Overlay a just-rendered diff viewport with a "guess first, then reveal" gate.
// Purely additive DOM — the diff underneath is untouched, so if anything here
// throws the edit still shows normally. Caller passes the viewport element and
// the change size; we decide whether to actually gate (challenge on, not already
// gated this turn, change substantial enough to be worth predicting).
export function predictGate(viewportEl, opts = {}) {
  try {
    load();
    if (!state.prefs.challenge || gatedThisTurn || !viewportEl) return false;
    if ((opts.lines || 0) < 4) return false;     // trivial edits aren't worth a guess
    gatedThisTurn = true;
    injectStyles();

    viewportEl.style.position = viewportEl.style.position || "relative";
    const gate = document.createElement("div");
    gate.className = "growth-gate";
    gate.innerHTML =
      `<div class="growth-gate__inner">` +
      `<div class="growth-gate__t">🙈 你先猜</div>` +
      `<div class="growth-gate__s">先在脑子里想一遍：这段你会怎么改？想好了再看我的实现——自己先推一遍，记得更牢。</div>` +
      `<button type="button" class="growth-gate__reveal">揭晓答案</button>` +
      `</div>`;
    gate.querySelector(".growth-gate__reveal").addEventListener("click", (e) => {
      e.stopPropagation();
      gate.remove();
      const ask = document.createElement("div");
      ask.className = "growth-gate__check";
      ask.innerHTML =
        `<span>你的思路和我的接近吗？</span>` +
        `<button type="button" data-hit="1">👍 接近</button>` +
        `<button type="button" data-hit="0">👎 差挺多</button>`;
      ask.querySelectorAll("button").forEach((b) => {
        b.addEventListener("click", (ev) => {
          ev.stopPropagation();
          signal("predict", { hit: b.dataset.hit === "1" });
          ask.innerHTML = `<span class="growth-gate__done">已记下——${b.dataset.hit === "1" ? "稳。" : "没关系，正是这种「费点劲」最长本事。"}</span>`;
          setTimeout(() => ask.remove(), 2600);
        });
      });
      viewportEl.appendChild(ask);
    });
    viewportEl.appendChild(gate);
    return true;
  } catch { return false; }
}

// --- public: the adaptive teaching block for the system prompt ----------------

function levelLabel(p) { return p >= 0.7 ? "熟练" : p >= 0.4 ? "进阶" : "新手"; }

export function promptBlock(mode) {
  try {
    load();
    const explain = state.prefs.explain;
    const overall = avgMastery();
    const weak = SKILLS.filter((s) => state.skills[s.id].p < 0.45);
    const strong = SKILLS.filter((s) => state.skills[s.id].p >= 0.7);

    // Agent / tool modes: teaching must NOT pollute the DOING (telling it to
    // "explain every step" makes it ramble + under-deliver). So we give it a TIGHT
    // directive scoped to the WRAP-UP only — adaptive by skill level. This is the
    // "把新手提拔成高手 / 把高手伺候得更爽" lever, finally active in the mode the
    // user actually uses most (before, agent mode got zero adaptive teaching).
    if (mode === "agent" || mode === "explorer" || mode === "reviewer") {
      const al = ["--- 因人而教（**只作用于收尾总结**，干活过程照常高效、绝不啰嗦；据该用户能力画像自适应）---"];
      if (overall < 0.45) {
        al.push("该用户偏**新手**：干活时照常麻利、别中途解释；但**收尾**用大白话讲清「做了什么 / 为什么这么做 / 怎么用 / 改了哪些文件」，第一次出现的术语顺手一句话点破，并**额外给一个可迁移的原理 + 一个他自己能上手试的小下一步**——目标是把他一步步带成「会用、会判断、会自己写」，越用越长本事。别居高临下。");
      } else if (overall < 0.7) {
        al.push("该用户**进阶**：收尾简明说清改动与关键决策，点一个值得注意的点或更优做法；少铺垫、别从头讲基础。");
      } else {
        al.push("该用户是**高手**：对等、精简、直给——跳过一切基础讲解，收尾只点深层取舍 / 边界 / 风险 / 你做的关键假设；把他当资深同行，别教学、别复述显然的东西，能省则省。");
      }
      if (weak.length && overall < 0.7) al.push(`可顺带培养的弱项（**仅收尾点到为止**，绝不打断干活）：${weak.map((s) => s.label).join("、")}。`);
      if (strong.length) al.push(`其强项（${strong.map((s) => s.label).join("、")}）：直接给结论，别赘述。`);
      return "\n\n" + al.join("\n");
    }

    // base verbosity: pref override, else derived from overall mastery
    let density;
    if (explain === "min") density = "极简：只给改动与结论，几乎不解释。";
    else if (explain === "rich") density = "详尽：每步都讲清原理与坑。";
    else density = overall >= 0.7
      ? "精简：默认少解释、直接给改动；仅在不显然处补一句。"
      : overall >= 0.45
        ? "适中：关键处给简短的「为什么」，其余从简。"
        : "充分：把每个关键步骤讲清楚，多给「为什么」和示例。";

    const lines = [];
    lines.push("--- 成长性教学（依据对该用户的能力画像自适应，请据此调整讲解粒度）---");
    lines.push(`用户总体水平：${levelLabel(overall)}。讲解密度：${density}`);
    if (strong.length) lines.push(`其强项（${strong.map((s) => s.label).join("、")}）：别赘述，直接给结论。`);
    for (const s of weak) lines.push(`其弱项「${s.label}」：${s.coach}`);
    lines.push("通用：每次完成后用一句话点出一个可迁移的原理，让用户真正学到，而不是只复制结果（避免养成依赖）。务必简洁。");
    if (state.prefs.challenge) {
      lines.push("挑战模式：对有一定难度的改动，先别直接给完整答案——用一两句话引导用户先自己想/写，再给出你的实现并对比（合意difficulty，利于长期掌握）。");
    }
    return "\n\n" + lines.join("\n");
  } catch {
    return "";   // never block a chat send
  }
}

// --- public: the Open Learner Model panel -------------------------------------

function injectStyles() {
  if (document.getElementById("growth-styles")) return;
  const css = `
  .growth-wrap{padding:4px 2px 24px}
  .growth-banner{display:flex;gap:10px;align-items:flex-start;margin:6px 0 18px;padding:12px 14px;border-radius:10px;
    background:color-mix(in srgb, var(--accent,#3b82f6) 12%, transparent);border:1px solid color-mix(in srgb,var(--accent,#3b82f6) 30%,transparent);font-size:12.5px;line-height:1.5}
  .growth-banner.warn{background:color-mix(in srgb,#e0a000 14%,transparent);border-color:color-mix(in srgb,#e0a000 38%,transparent)}
  .growth-banner svg{flex:0 0 auto;margin-top:1px;opacity:.85}
  .growth-section-t{font-size:11px;text-transform:uppercase;letter-spacing:.06em;color:var(--text-dim,#888);margin:20px 2px 10px}
  .growth-skill{display:grid;grid-template-columns:130px 1fr auto;gap:12px;align-items:center;margin:10px 2px}
  .growth-skill__name{font-size:13px;font-weight:600}
  .growth-skill__name small{display:block;font-weight:400;font-size:11px;color:var(--text-dim,#888);margin-top:1px}
  .growth-bar{position:relative;height:8px;border-radius:5px;background:var(--hover,rgba(128,128,128,.18));overflow:hidden}
  .growth-bar__fill{position:absolute;inset:0 auto 0 0;border-radius:5px;background:linear-gradient(90deg,var(--accent,#3b82f6),color-mix(in srgb,var(--accent,#3b82f6) 60%,#22c55e));transition:width .5s cubic-bezier(.2,.7,.3,1)}
  .growth-skill__right{display:flex;align-items:center;gap:6px}
  .growth-skill__pct{font-variant-numeric:tabular-nums;font-size:12px;color:var(--text-dim,#888);min-width:62px;text-align:right}
  .growth-xfer{font-size:10.5px;padding:2px 7px;border-radius:10px;background:var(--hover,rgba(128,128,128,.16));color:var(--text-dim,#999);white-space:nowrap}
  .growth-xfer.is-xfer{background:color-mix(in srgb,#22c55e 22%,transparent);color:#3fb950;font-weight:600}
  .growth-profile{margin:6px 2px 4px;padding:14px;border-radius:14px;background:var(--panel-2,var(--hover,rgba(128,128,128,.1)));border:1px solid var(--atc-border,rgba(128,128,128,.2))}
  .growth-profile__cells{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:10px;align-items:stretch}
  .growth-profile__c{min-width:0;min-height:58px;padding:10px 12px;border-radius:12px;background:color-mix(in srgb,var(--hover,rgba(128,128,128,.1)) 62%,transparent);display:flex;flex-direction:column;align-items:center;justify-content:center;text-align:center}
  .growth-profile__c b{font-size:22px;font-variant-numeric:tabular-nums;display:block;line-height:1.08}
  .growth-profile__c span{font-size:11px;color:var(--text-dim,#888)}
  .growth-profile__line{margin:11px 0 0;font-size:12.5px;color:var(--text-dim,#999);line-height:1.5}
  .growth-nudge{width:22px;height:22px;border-radius:6px;border:1px solid var(--atc-border,rgba(128,128,128,.25));background:transparent;color:var(--text,#ddd);cursor:pointer;font-size:13px;line-height:1;display:grid;place-items:center}
  .growth-nudge:hover{background:var(--hover,rgba(128,128,128,.15))}
  .growth-ctl{display:flex;flex-wrap:wrap;gap:10px;align-items:center;margin:8px 2px}
  .growth-ctl label{font-size:12.5px;color:var(--text,#ddd)}
  .growth-ctl select{background:var(--panel-2,var(--panel,#222));color:var(--text,#ddd);border:1px solid var(--atc-border,rgba(128,128,128,.25));border-radius:7px;padding:4px 8px;font-size:12.5px}
  .growth-switch{display:inline-flex;align-items:center;gap:8px;cursor:pointer;user-select:none}
  .growth-switch input{appearance:none;width:34px;height:20px;border-radius:11px;background:var(--hover,rgba(128,128,128,.3));position:relative;cursor:pointer;transition:background .2s}
  .growth-switch input:checked{background:var(--accent,#3b82f6)}
  .growth-switch input::after{content:"";position:absolute;top:2px;left:2px;width:16px;height:16px;border-radius:50%;background:#fff;transition:transform .2s}
  .growth-switch input:checked::after{transform:translateX(14px)}
  .growth-stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(96px,1fr));gap:8px;margin:10px 2px}
  .growth-stat{background:var(--hover,rgba(128,128,128,.1));border-radius:9px;padding:9px 11px}
  .growth-stat b{display:block;font-size:18px;font-variant-numeric:tabular-nums}
  .growth-stat span{font-size:11px;color:var(--text-dim,#888)}
  .growth-reset{margin-top:14px;background:transparent;border:1px solid var(--atc-border,rgba(128,128,128,.25));color:var(--text-dim,#999);border-radius:7px;padding:6px 12px;font-size:12px;cursor:pointer}
  .growth-reset:hover{color:#e5484d;border-color:#e5484d}
  .growth-project{margin:6px 2px}
  .growth-project__intro{font-size:12.5px;color:var(--text-dim,#999);line-height:1.55;margin:0 0 8px}
  .growth-project__list{margin:0;padding-left:18px;display:flex;flex-direction:column;gap:5px}
  .growth-project__list li{font-size:12.5px;line-height:1.45}
  .growth-project__more{font-size:11.5px;color:var(--text-dim,#888);margin-top:6px}
  .growth-project__btn{margin-top:10px;background:transparent;border:1px solid var(--atc-border,rgba(128,128,128,.25));color:var(--text,#ddd);border-radius:7px;padding:5px 11px;font-size:12px;cursor:pointer}
  .growth-project__btn:hover{background:var(--hover,rgba(128,128,128,.15))}
  .growth-gate{position:absolute;inset:0;display:grid;place-items:center;padding:14px;border-radius:8px;background:color-mix(in srgb,var(--panel,#1c1c1c) 72%,transparent);backdrop-filter:blur(7px);-webkit-backdrop-filter:blur(7px);z-index:3;text-align:center}
  .growth-gate__inner{max-width:340px}
  .growth-gate__t{font-size:15px;font-weight:700;margin-bottom:4px}
  .growth-gate__s{font-size:12px;color:var(--text-dim,#999);line-height:1.5;margin-bottom:12px}
  .growth-gate__reveal{background:var(--accent,#3b82f6);color:#fff;border:0;border-radius:8px;padding:7px 18px;font-size:13px;font-weight:600;cursor:pointer}
  .growth-gate__reveal:hover{filter:brightness(1.08)}
  .growth-gate__check{display:flex;gap:8px;align-items:center;flex-wrap:wrap;padding:8px 10px;font-size:12.5px;border-top:1px dashed var(--atc-border,rgba(128,128,128,.25))}
  .growth-gate__check button{background:var(--hover,rgba(128,128,128,.15));border:1px solid var(--atc-border,rgba(128,128,128,.25));border-radius:7px;padding:4px 10px;cursor:pointer;color:var(--text,#ddd);font-size:12.5px}
  .growth-gate__check button:hover{background:var(--accent,#3b82f6);color:#fff}
  .growth-gate__done{color:var(--text-dim,#999)}
  `;
  const el = document.createElement("style");
  el.id = "growth-styles";
  el.textContent = css;
  document.head.appendChild(el);
}

function trendVerdict() {
  const t = state.trend;
  if (t.length < 4) return null;
  const cur = t[t.length - 1];
  const prev = t.find((p) => cur.msgs - p.msgs >= 6) || t[0];
  const dMsgs = cur.msgs - prev.msgs;
  const dAvg = cur.avg - prev.avg;
  if (dMsgs < 6) return null;
  if (dAvg <= 0.005 && state.stats.aiEdits >= 4)
    return { warn: true, text: "你用得越来越多，但能力掌握度没怎么涨——当心把思考外包给 AI（“元认知惰性”）。试试多展开 diff 看看、或打开下面的「挑战模式」让自己先上手。" };
  if (dAvg >= 0.02)
    return { warn: false, text: `保持住——最近你的整体掌握度在上升（+${Math.round(dAvg * 100)} 点）。越勇越厉害。` };
  return null;
}

export function renderPanel(body, ctx = {}) {
  try {
    load();
    injectStyles();

    const head = document.createElement("div");
    head.className = "tool-head";
    head.innerHTML = `<h2></h2><p></p>`;
    head.querySelector("h2").textContent = "成长";
    head.querySelector("p").textContent = "这是你的开发者成长档案——跨所有项目积累、换项目也带着走。越战越勇：一项能力在越多样的项目里练过，就越是你的。你可以随时纠正我。";
    body.appendChild(head);

    const wrap = document.createElement("div");
    wrap.className = "growth-wrap";
    body.appendChild(wrap);

    const rerender = () => { body.innerHTML = ""; renderPanel(body, ctx); };

    // trend banner
    const v = trendVerdict();
    if (v) {
      const b = document.createElement("div");
      b.className = "growth-banner" + (v.warn ? " warn" : "");
      b.innerHTML = `<svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1a7 7 0 100 14A7 7 0 008 1zm0 3.2a.9.9 0 01.9.9v3.6a.9.9 0 01-1.8 0V5.1A.9.9 0 018 4.2zm0 7.2a1 1 0 110-2 1 1 0 010 2z"/></svg><span></span>`;
      b.querySelector("span").textContent = v.text;
      wrap.appendChild(b);
    }

    // cross-project growth headline — this capability is YOURS, not this app's;
    // breadth across projects is what makes a skill transferable ("越战越勇").
    {
      const np = projectCount();
      const nt = transferableCount();
      const strip = document.createElement("div");
      strip.className = "growth-profile";
      strip.innerHTML =
        `<div class="growth-profile__cells">` +
          `<div class="growth-profile__c"><b>${np}</b><span>实战项目</span></div>` +
          `<div class="growth-profile__c"><b>${nt}/${SKILLS.length}</b><span>可迁移能力</span></div>` +
          `<div class="growth-profile__c"><b>${state.stats.messages}</b><span>累计轮次</span></div>` +
        `</div><p class="growth-profile__line"></p>`;
      strip.querySelector(".growth-profile__line").textContent = np <= 1
        ? "目前只在 1 个项目里实战。多上手几个不同项目，你的能力会被「跨项目」证明、迁移性更强——越战越勇。"
        : `已在 ${np} 个项目里实战，其中 ${nt} 项能力已跨项目验证、换新项目也带得走。这才是真正的成长。`;
      wrap.appendChild(strip);
    }

    // skills (the open learner model — each row is inspectable AND editable)
    const st = document.createElement("div");
    st.className = "growth-section-t";
    st.textContent = "能力档位（点 ± 可纠正我的判断；标签 = 跨项目迁移度）";
    wrap.appendChild(st);

    for (const sk of SKILLS) {
      const m = state.skills[sk.id];
      const pct = Math.round(m.p * 100);
      const breadth = skillBreadth(sk.id);
      const xfer = isTransferable(sk.id);
      const xferTxt = xfer ? `可迁移 · ${breadth}` : breadth >= 1 ? `${breadth} 个项目` : "未跨项目";
      const row = document.createElement("div");
      row.className = "growth-skill";
      row.innerHTML =
        `<div class="growth-skill__name">${sk.label}<small>${sk.blurb}</small></div>` +
        `<div class="growth-bar"><div class="growth-bar__fill" style="width:${pct}%"></div></div>` +
        `<div class="growth-skill__right">` +
          `<span class="growth-xfer${xfer ? " is-xfer" : ""}" title="在 ${breadth} 个不同项目里练过">${xferTxt}</span>` +
          `<span class="growth-skill__pct">${levelLabel(m.p)} · ${pct}%</span>` +
          `<button class="growth-nudge" data-d="-1" title="我比这更弱">−</button>` +
          `<button class="growth-nudge" data-d="1" title="我比这更强">＋</button>` +
        `</div>`;
      row.querySelectorAll(".growth-nudge").forEach((btn) => {
        btn.addEventListener("click", () => {
          const d = +btn.dataset.d;
          m.p = clamp(m.p + d * 0.08, 0.02, 0.98);
          m.last = Date.now();
          save();
          rerender();
        });
      });
      wrap.appendChild(row);
    }

    // project understanding — the "越来越懂你的项目" half of the model. Reuses the
    // agent's `remember` project memory, passed in by the host (main.js).
    if (ctx.projectMemory !== undefined) {
      const facts = String(ctx.projectMemory || "")
        .split("\n").map((s) => s.replace(/^[-*]\s*/, "").trim()).filter(Boolean);
      const pt = document.createElement("div");
      pt.className = "growth-section-t";
      pt.textContent = "我对这个项目的理解" + (ctx.projectName ? ` · ${ctx.projectName}` : "");
      wrap.appendChild(pt);

      const box = document.createElement("div");
      box.className = "growth-project";
      const intro = document.createElement("p");
      intro.className = "growth-project__intro";
      if (facts.length) {
        intro.textContent = `已记住 ${facts.length} 条关于这个项目的知识（架构 / 约定 / 命令 / 易踩的坑），每轮自动带进上下文——所以我越用越懂你的项目。`;
        box.appendChild(intro);
        const ul = document.createElement("ul");
        ul.className = "growth-project__list";
        for (const f of facts.slice(-4).reverse()) {
          const li = document.createElement("li");
          li.textContent = f.length > 120 ? f.slice(0, 120) + "…" : f;
          ul.appendChild(li);
        }
        box.appendChild(ul);
        if (facts.length > 4) {
          const more = document.createElement("div");
          more.className = "growth-project__more";
          more.textContent = `…还有 ${facts.length - 4} 条`;
          box.appendChild(more);
        }
      } else {
        intro.textContent = ctx.projectName
          ? "还没攒下关于这个项目的长期知识。随着一起干活，我会用 remember 把架构、约定、构建/测试命令记下来，下次自动带上。"
          : "打开一个工作区文件夹后，我会开始记住这个项目的知识。";
        box.appendChild(intro);
      }
      if (ctx.onOpenMemory) {
        const btn = document.createElement("button");
        btn.className = "growth-project__btn";
        btn.textContent = "管理项目记忆 →";
        btn.addEventListener("click", () => { try { ctx.onOpenMemory(); } catch { /* host gone */ } });
        box.appendChild(btn);
      }
      wrap.appendChild(box);
    }

    // controls
    const ct = document.createElement("div");
    ct.className = "growth-section-t";
    ct.textContent = "教学偏好";
    wrap.appendChild(ct);

    const ctl1 = document.createElement("div");
    ctl1.className = "growth-ctl";
    ctl1.innerHTML = `<label for="growth-explain">AI 讲解密度</label>`;
    const sel = document.createElement("select");
    sel.id = "growth-explain";
    for (const [val, lbl] of [["auto", "自动（随能力渐隐）"], ["rich", "总是详尽"], ["min", "总是极简"]]) {
      const o = document.createElement("option");
      o.value = val; o.textContent = lbl;
      if (state.prefs.explain === val) o.selected = true;
      sel.appendChild(o);
    }
    sel.addEventListener("change", () => { state.prefs.explain = sel.value; save(); });
    ctl1.appendChild(sel);
    wrap.appendChild(ctl1);

    const ctl2 = document.createElement("div");
    ctl2.className = "growth-ctl";
    const sw = document.createElement("label");
    sw.className = "growth-switch";
    sw.innerHTML = `<input type="checkbox"><span>挑战模式 — 难的地方让我先自己想，AI 再揭晓（练得更扎实）</span>`;
    const cb = sw.querySelector("input");
    cb.checked = !!state.prefs.challenge;
    cb.addEventListener("change", () => { state.prefs.challenge = cb.checked; save(); });
    ctl2.appendChild(sw);
    wrap.appendChild(ctl2);

    // stats
    const stt = document.createElement("div");
    stt.className = "growth-section-t";
    stt.textContent = "行为信号";
    wrap.appendChild(stt);

    const s = state.stats;
    const blindRate = s.aiEdits ? Math.round((s.blind / s.aiEdits) * 100) : 0;
    const grid = document.createElement("div");
    grid.className = "growth-stats";
    const cells = [
      [s.messages, "提问轮次"],
      [s.aiEdits, "AI 改动"],
      [s.reviews, "你审过的 diff"],
      [s.undos, "你撤回的改动"],
      [blindRate + "%", "未看就接受率"],
      [s.predicts ? Math.round((s.predictHits / s.predicts) * 100) + "%" : "—", "你先猜命中率"],
    ];
    for (const [val, lbl] of cells) {
      const cell = document.createElement("div");
      cell.className = "growth-stat";
      cell.innerHTML = `<b></b><span></span>`;
      cell.querySelector("b").textContent = val;
      cell.querySelector("span").textContent = lbl;
      grid.appendChild(cell);
    }
    wrap.appendChild(grid);

    const reset = document.createElement("button");
    reset.className = "growth-reset";
    reset.textContent = "重置我的成长档案";
    reset.addEventListener("click", () => {
      if (!confirm("确定清空「成长」对你的全部画像与统计？此操作不可撤销。")) return;
      state = freshState();
      turn = { applied: 0, reviewed: 0, engaged: false };
      save();
      rerender();
    });
    wrap.appendChild(reset);
  } catch (e) {
    body.innerHTML = `<div style="padding:20px;color:var(--text-dim,#888)">「成长」面板渲染失败：${(e && e.message) || e}</div>`;
  }
}

// ===== ADAPTIVE RUNTIME POLICY =====
// 根据 avgMastery 输出工具加载策略

export function getRuntimePolicy(growthState = null) {
    const g = growthState || state || {};
    const avgP = typeof g.avgMastery === 'function' ? g.avgMastery() : 
                  (g.avg_mastery ?? 0.5); // fallback to medium
    
    // 三档策略：新手/进阶/专家
    if (avgP > 0.7) {
        // 专家模式：开放更多工具窗口
        return {
            initialToolCount: 20,
            criticMaxTools: 15,
            catalogDensity: 'minimal', // 减少冗余描述节省空间
            enableProfessionalTools: true,
            allowAutoLoadAdvanced: true
        };
    } else if (avgP > 0.45) {
        // 进阶模式：适度扩展
        return {
            initialToolCount: 14,
            criticMaxTools: 12,
            catalogDensity: 'balanced',
            enableProfessionalTools: true,
            allowAutoLoadAdvanced: false
        };
    } else {
        // 新手模式：保守
        return {
            initialToolCount: 11,
            criticMaxTools: 10,
            catalogDensity: 'rich', // 新手需要更详细的提示
            enableProfessionalTools: false,
            allowAutoLoadAdvanced: false
        };
    }
}
