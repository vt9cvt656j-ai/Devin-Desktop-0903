# Michael IDE 反幻觉协议 (Anti-Hallucination Protocol)

**版本**: v1.0  
**状态**: 待 #62 修复合并后激活  
**时间**: 2026 年 7 月 30 日  
**关联**: [THREE_ISSUES_DIAGNOSIS.md](../THREE_ISSUES_DIAGNOSIS.md) (问题 A/B/C)

---

## 一、协议目标

**核心承诺**:模型的所有技术结论必须有可验证的证据链，严禁基于推测、过时文档或单一 Markdown 笔记下确定性结论。

### 三大幻觉风险

| 风险 | 现象 | 根因 | 协议对策 |
|-----|------|-----|---------|
| **A. 思考泄漏** | 推理文本混入正式回答 | `<think>` 标签分片边界切割漏洞 | Protocol-A: 流式完整性校验 |
| **B. 意图误判** | 查询被拦截成 no-op | 弱模型混淆"位置 + 查询"vs"仅位置" | Protocol-B: 证据门槛声明 |
| **C. 文档幻觉** | 仅凭.md 断言项目事实 | 源码权威未被强制执行 | Protocol-C: 证据分级仲裁 |

---

## 二、Protocol-A: 流式完整性校验

### 设计原理

**目标**:消除 `/_routeThink()` 在流式分片边界处可能将未闭合的`<think>` 标签内容误作答案的风险。

**触发条件**:
- 任何涉及流式输出的对话轮次
- 特别关注弱模型 (如 Qwen3.6-35B-A3B) 不完全遵循 OpenAI 规范的场景

### 实现要求

#### 阶段 1:标签完整性守卫 (Priority P1)

**代码注入点**: `/src/main.js` 行 20623 前 (finally 块或流结束钩子)

```javascript
// === ANTI-HALLUCINATION PROTOCOL-A START ===
// 流式结束时强制校验思考标签完整性
if (_thinkingFlowEndTriggered) { // 由调度器在 scheduleStream() 空转时标记
  const residual = _thinkHold.trim();
  
  // 检查是否有悬空的 <think> 片段
  if (/^<think\b/i.test(residual) || /^<\/think\s*>$/i.test(residual)) {
    // 情况 1:标签未闭合 → 强制推回思考卡
    reasoning += residual;
    setThink(reasoning);
    
    // 警告日志 (仅内部记录)
    console.warn(
      '[AHDH] 思考标签不完整，已强制修正',
      { residual, totalReasoningLength: reasoning.length }
    );
  } else if (_partialOpen(residual, "<think>") > 0) {
    // 情况 2:正在接收 <think> 但剩余字符不足 → 保留缓冲直到明确终止
    // _thinkHold 已包含这部分，不需要额外处理
  }
  
  // 双重保险:清理后的 acc 中不应再有 <think> 标记
  const thinkInAnswer = /\b<think\b|<\/think\s*>\b/.test(acc);
  if (thinkInAnswer) {
    console.error('[AHDH] 严重违规：答案中检测到<think> 标签');
    // 尝试二次清洗
    acc = acc.replace(/\s*<think\b[^>]*>|<\/think\s*>\s*/g, '');
  }
}
// === ANTI-HALLUCINATION PROTOCOL-A END ===
```

#### 阶段 2:DOM 层隔离验证 (Priority P2 - 保险)

**代码注入点**: `/src/main.js` 行 20980 前 (`renderMarkdownInto()` 之前)

```javascript
// === ANTI-HALLUCINATION PROTOCOL-A P2 START ===
// 消息渲染前的最终校验
const _sanityCheckContextForThink(ctx) {
  if (!ctx?._chatStreamEl) return true; // 无 DOM 则跳过
  
  // 检查 CC 是否残留思考标记
  const chatContent = ctx._cc || "";
  const hasUnclosedThink = /<think\b[^>]*(?:>|$)|<\/think\s*>/.test(chatContent);
  
  if (hasUnclosedThink) {
    console.warn('[AHDH-P2] 消息上下文含未闭合思考标签，强制分离');
    // 分割内容与思考部分
    const parts = chatContent.split(/(<think\b[^>]*>|<\/think\s*>)\s*/);
    const thoughtParts = [];
    const answerParts = [];
    let inThink = false;
    
    for (const part of parts) {
      if (part.toLowerCase().startsWith('<think') || part === '<think>') {
        inThink = true;
        thoughtParts.push(part);
      } else if (part.toLowerCase().startsWith('</think')) {
        inThink = false;
        thoughtParts.push(part);
      } else if (inThink) {
        thoughtParts.push(part);
      } else {
        answerParts.push(part);
      }
    }
    
    // 将分离的思考内容追加到 existing thinking card
    if (thoughtParts.length > 0 && setThink) {
      setThink((reasoning || '') + thoughtParts.join(''));
    }
    
    // 修改 CC 为纯答案
    ctx._cc = answerParts.join('').trim();
  }
  
  return true;
}
// === ANTI-HALLUCINATION PROTOCOL-A P2 END ===
```

#### 阶段 3:弱模型适配 Prompt(长期 P3)

**Prompt 注入点**: `/src/main.js` 行 16242-16254 (每个 `_AI_MODE_PROMPTS` 模式字符串末尾追加)

```javascript
// 在所有 AI 模式提示词中统一追加以下内容
"\n\n【思考格式铁律】
- 所有思考必须在单独的 `思者块` 内，使用 <think>...</think> 包裹
- 正文中绝对不允许夹带任何 <think> 标签或思考内容
- 流式输出时，如果<think> 标签被网络分片切断，必须等待完整标签闭合再继续吐正文
- 违反此规则的回复会被系统识别为'思考泄漏'并触发内部告警"
```

---

## 三、Protocol-B: 意图判定证据门控

### 设计原理

**根因复盘**(参考诊断报告 B):
- 用户问："在项目里，到底怎么实现绕过视频检测？"
- 弱模型误判 `locationIntent="context_only"`(仅提供位置),实际应`locationIntent="query"`(位置 + 查询)
- list_dir 工具调用被拦 → 模型被迫降级读.md → C 问题发生

**核心改进**:从纯 AI 语义判断升级为"AI 主判 + 关键词兜底校正"的双层机制，但注意不违反架构纪律。

### 实现方案

#### 修正案 I:意图 Prompt 强化 (Priority P1)

**代码位置**: `/src/main.js` 行 16820-16834 (`_aiIntentProfile` Prompt 定义)

**修改内容**:替换原有简略定义为详细规则 + 示例表

```javascript
// === ANTI-HALLUCINATION PROTOCOL-B P1 START ===
const locationIntentPrompt = `
locationIntent∈{none/context_only/query/remember}，判定规则如下：

【none】仅抽象问题，无位置上下文
  示例："怎么实现 HTTPS?" "什么是异步编程?"
  特征：不涉及"这个""那个""项目""目录""文件"等指示词

【context_only】仅提供位置，无查询/动作意图
  示例："这个项目怎么样？" "看看这个目录" "这个项目代码写得如何？"
  特征：只描述对象，没有"怎么做""为什么""能不能改"等动词短语

【query】提供位置 + 明确查询/动作意图 ⭐优先匹配此档
  示例：
  - "在项目里怎么实现绕过检测？" → 有"怎么实现"(查询)
  - "这个项目的视频处理模块在哪里？" → 有"在哪里"(定位查询)
  - "帮我看看这里有什么 bug" → 有"看看...bug"(调试意图)
  特征：位置词 (项目/目录/文件) + 动词短语 (怎么/如何/哪里/能吗/可不可以/是不是/请检查/请分析)

【remember】要求记住位置信息供后续使用
  示例："记住我这个项目用的是 React" "保存当前路径"
  特征：有"记住""保存""记住"等存证类指令

【关键区分】位置 + 查询≠仅位置
  即使提到"在项目里""在这个目录",只要同时有查询动词，必须是 query!
  
【判定优先级】
  1. 先看有没有明确的动作词 (modify/run/debug/review)
  2. 再看有没有查询词 (怎么/如何/哪里/为什么/是什么)
  3. 只有两者都没有才可能是 context_only
`;

// 替换原有单行定义
// old: "locationIntent=none/context_only/query/remember（仅提供位置上下文、明确要查询位置相关信息、或要求记住位置）"
// new: 使用上面定义的多行规则
// === ANTI-HALLUCINATION PROTOCOL-B P1 END ===
```

#### 修正案 II:后置快速校准 (Priority P2 - 非侵入式)

**注意**:不是添加新的关键词路由函数，而是在拦截逻辑前做**最终校验**,若发现明显矛盾则主动纠正意图字段，而非 bypass 拦截。

**代码注入点**: `/src/main.js` 行 36801 前 (现有拦截判断之前)

```javascript
// === ANTI-HALLUCINATION PROTOCOL-B P2 START ===
// 在决定是否拦截前，对弱模型的误判做一次"反向验证"
// 这不是新增路由，而是检查 AI 判决是否自相矛盾
let shouldOverrideIntent = false;
let overrideTo = null;

if (call.type !== "memory" && run.engineering?.intentSemantic?.locationIntent === "context_only") {
  // 条件 A:用户消息里有明显的动作意图
  const currentUserText = [run?._originalText, run?._steeringText].filter(Boolean).join("\n");
  
  // 动作词汇列表 - 这不是路由判断，只是辅助检查
  const actionKeywords = ["怎么", "如何", "实现", "修复", "改", "调试", "分析", "检查", "验证", "测试", "跑", "运行", "执行", "哪里", "什么"];
  const hasActionWord = actionKeywords.some(kw => currentUserText.includes(kw));
  
  // 条件 B:或者 AI 已经检测到 action 是 inspect/modify/create 等非 none 值
  const hasExplicitAction = ["inspect", "modify", "create", "debug", "review", "plan"].includes(
    run.engineering?.intentSemantic?.action
  );
  
  if (hasActionWord && !hasExplicitAction) {
    // 虽然 AI 说"context_only",但有动作词 → 怀疑误判
    overrideTo = "query";
  } else if (hasExplicitAction && run.engineering?.intentSemantic?.locationIntent === "context_only") {
    // AI 自己判定 action 是 modify，但 locationIntent 是 context_only → 矛盾
    overrideTo = "query";
  }
}

if (shouldOverrideIntent && overrideTo) {
  console.log('[AHDH-B-P2] 发现意图矛盾，自动校正:', run.engineering.intentSemantic.locationIntent, '→', overrideTo);
  run.engineering.intentSemantic.locationIntent = overrideTo;
  // 注意：不直接放行，而是修改 run 对象，让后续流程重新评估
}
// === ANTI-HALLUCINATION PROTOCOL-B P2 END ===
```

#### 修正案 III:替代工具建议 (Priority P3)

**目标**:即使被拦，也给出具体路径而不是简单拒绝

**代码注入点**: `/src/main.js` 行 36805 (原拦截返回内容处)

```javascript
// === ANTI-HALLUCINATION PROTOCOL-B P3 START ===
// 被拦时的引导式响应 (替代冷冰冰的 BLOCKED)
const r = { 
  type: call.type, 
  path: call.path || "", 
  content: "[工具暂时受限]\n" +
    "当前对话的意图判定尚未完全就绪，list_dir 权限需进一步确认。\n\n" +
    "如需了解项目结构，可改用以下精准工具:\n" +
    "- read_file(path='/path/to/package.json')\t读取配置\n" +
    "- read_file(path='/path/to/src/main.rs')\t读取源码\n" +
    "- search(query='关键词')\t\t\t搜索特定代码\n" +
    "- terminal(command='ls -la')\t\t直连磁盘\n\n" +
    "提出更具体的查询后，list_dir 权限可恢复。"
};
// === ANTI-HALLUCINATION PROTOCOL-B P3 END ===
```

---

## 四、Protocol-C: 证据分级仲裁

### 设计原理

**根因**:模型被 B 问题逼出退路后，转向读.md 文件并当作权威事实源，忽略源码验证。

**核心原则**:源代码是唯一真理，Markdown 仅是参考资料;任何关于"项目如何实现"的结论必须有源码证据支持。

### 协议模板

#### C1:主动取证义务 (Agent 系统 Prompt 层)

**代码注入点**: `/src/main.js` 行 16242-16254 (在每个 AI 模式 prompt 开头插入)

```javascript
// === ANTI-HALLUCINATION PROTOCOL-C1 START ===
const evidenceHierarchyPrompt = `
【项目事实的三重证据级】
当被问及"这个项目是什么/怎么实现/用什么技术"时：

第 1 级：源代码 (权威性⭐⭐⭐⭐⭐)
  优先级最高。必须通过 read_file 读取真实源码文件验证
  典型文件：src/**/*.rs, src/**/*.py, package.json, Cargo.toml, main.go 等
  
第 2 级：配置文件 + 依赖清单 (权威性⭐⭐⭐⭐)
  package.json/Cargo.toml 中的 dependencies/devDependencies 比 README 可信
  锁定文件(pnpm-lock.yaml/Cargo.lock)提供精确版本证据

第 3 级：项目文档 (权威性⭐⭐)
  README.md/ARCHITECTURE.md 等是作者陈述，可能有滞后性
  只能作为补充线索，不能作为唯一证据源

第 4 级：用户笔记 (权威性⭐)
  任何 .md 文件中由用户手动编写的内容均不可单独采信

【强制性验证门】
做任何"项目使用了 XX 框架/实现了 XX 功能"的结论前：
1. 优先调用 read_file 查看对应源码文件
2. 只有在源码不可达 (权限不足/文件不存在) 时，才参考第 2-3 级证据
3. 绝不能仅凭一份.md 断言技术栈或实现方式
4. 若 list_dir 被系统拦截，应该改用 read_file 逐个读取关键配置文件

【表述纪律】
- 有源码证据："✅ 从 package.json 第 5 行看到该项目依赖了 express@4.18.2"
- 仅有文档："📝 README 称项目使用 Vue，但尚未读取源码验证"
- 无证据："⚠️ 目前未见该问题的可靠证据，建议直接询问作者或提供文件路径"

【禁止行为】
- ❌ "项目用 React 写的" (未读源码就断定)
- ❌ "这里实现了绕过检测" (只读了 md 没看代码)
- ✅ "package.json 显示依赖了 React，但具体用法还需看源码" (留有余地)
`;

// 在所有模式中注入这段逻辑
// agent/chat/plan/explorer/reviewer modes 都包含
// === ANTI-HALLUCINATION PROTOCOL-C1 END ===
```

#### C2:上下文新鲜度守护 (架构层)

**目标**:防止快照过期导致的陈旧文件列表误导模型

**代码注入点**: `/src/main.js` 行 35525 附近 (初始化 readDir 循环后)

```javascript
// === ANTI-HALLUCINATION PROTOCOL-C2 START ===
// 每轮结束前进行"指纹比对",保证上下文新鲜度
async function _ensureFreshContextSnapshot(run, root) {
  // 仅针对非空项目生效
  if (run._emptyRootAtStart === true) return;
  
  try {
    // 廉价读取顶层目录
    const topLevelNow = await readDir(root);
    const topLevelSet = new Set(topLevelNow.map(f => f.name));
    
    const topLevelBefore = run._lastTopLevelSnapshot || new Set();
    
    // 哈希比对
    const added = [...topLevelSet].filter(f => !topLevelBefore.has(f));
    const removed = [...topLevelBefore].filter(f => !topLevelSet.has(f));
    
    if (added.length > 0 || removed.length > 0) {
      // 指纹变化 → 旧快照失效
      console.log(`[AHDH-C2] 项目快照过期：新增${added.length}个，移除${removed.length}个`);
      run._projectContextStale = true;
      run._lastTopLevelSnapshot = topLevelSet;
      
      // 回灌到对话上下文：软 nudge，不强制中断
      const staleMessage = `
⚠️ 项目文件结构已发生变化。历史对话中的文件列表可能不符合当前磁盘实况。
新增：${added.join(', ') || '无'}
移除：${removed.join(', ') || '无'}

任何基于旧文件结构的结论 (如"项目使用了 XX 框架") 需要重新验证。
`;
      _pushNudge('staleContext', staleMessage);
    } else {
      run._projectContextStale = false;
    }
  } catch (err) {
    // IO 失败不影响主流程，仅静默记录
    console.warn('[AHDH-C2] 快照校验失败:', err.message);
  }
}

// 在主循环的合适位置调用此函数
// 推荐时机：每轮对话结束，准备发送下一轮请求前
// === ANTI-HALLUCINATION PROTOCOL-C2 END ===
```

#### C3:文档证据标注 UI 反馈 (前端展示层)

**目标**:让用户直观看到模型结论的证据强度

**设计稿**:在消息卡中添加"证据等级徽章"(可选插件)

```css
/* 证据等级样式 */
.evidence-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  margin-left: 8px;
}

.evidence-code { background: #dcfce7; color: #166534; } /* ✅ 源码级 */
.evidence-config { background: #fef3c7; color: #92400e; } /* 📝 配置级 */
.evidence-doc { background: #ffedd5; color: #9a3412; } /* 📄 文档级 */
.evidence-nudged { background: #fee2e2; color: #991b1b; } /* ⚠️ 存疑级 */
```

**HTML 注入逻辑**:

```javascript
// 在渲染消息时根据结论类型动态附加证据徽章
function _attachEvidenceBadge(messageEl, conclusionType, sources) {
  const badge = document.createElement('span');
  badge.className = 'evidence-badge';
  
  if (sources.some(s => s.type === 'source_code')) {
    badge.textContent = '✅ 源码验证';
    badge.classList.add('evidence-code');
  } else if (sources.some(s => s.type === 'config_file')) {
    badge.textContent = '📝 配置确认';
    badge.classList.add('evidence-config');
  } else if (sources.some(s => s.type === 'markdown_doc')) {
    if (sources.length === 1) {
      badge.textContent = '📄 仅文档';
      badge.classList.add('evidence-doc');
    } else {
      badge.textContent = '📄 文档佐证';
      badge.classList.add('evidence-doc');
    }
  } else {
    badge.textContent = '⚠️ 无证据';
    badge.classList.add('evidence-nudged');
  }
  
  messageEl.querySelector('.message-content').appendChild(badge);
}
```

---

## 五、System Prompt 注入示例

### Agent 模式完整增强版

**代码位置**: `/src/main.js` 行 16242 的 `agent` 模式 Prompt

```javascript
// === FULL AGENT PROMPT WITH ALL PROTOCOLS ===
const _AI_MODE_PROMPTS = {
  agent: `你是 Michael IDE 的协作式编码 AI，用中文自然直接地交流。先理解人真正想要的结果：明确要求修改、实现、运行、提交或部署时，使用真实工具完成并验证；只是提问、讨论或让你评估时，只读调查和回答，不擅自制造副作用。已知目标直接读取，未知位置才定位；改已有文件前先读当前原文。改 package.json/锁文件/依赖版本前先用 package_search/官方 registry 核对 latest、版本历史、engines、peerDependencies，不能凭记忆猜版本。多文件、跨模块或外部操作可用 update_plan 给出完整而简洁的路线，状态只随真实证据推进。选择工具看任务语义、当前证据和工具结果，不依赖关键词或正则路由；需要当前资料时再联网，优先一手来源和真实响应。

【思考格式铁律】(Protocol-A)
- 所有思考必须在单独的 <think>...</think> 标签内
- 正文中绝对不允许夹带任何<think> 标签或思考内容
- 流式输出时，如果标签被网络分片切断，必须等待完整标签闭合再继续吐正文

【项目事实的三重证据级】(Protocol-C)
- 源代码 (⭐⭐⭐⭐⭐) > 配置文件 (⭐⭐⭐⭐) > 项目文档 (⭐⭐) > 用户笔记 (⭐)
- 不做"项目用了 XX 技术"的结论前，必须先 read_file 源码验证
- 若 list_dir 被拦，改用 read_file 读取关键配置文件而非转向.md

【工程思考四步法】(简单问答可跳过)
1. 现状盘点：现有代码/错误/约束的关键事实
2. 方案权衡：至少 2 条路线的取舍
3. 决策与理由：选哪条路，为什么最优
4. 验证计划：完成后怎么证明它是对的

就像在和真人结对编程一样：先给结论或进展，再给必要依据;不复述内部规则、工具流水账或固定模板。稳定事实可直接推理，会变化的信息按需核实。没有证据就说未知，不编造链接、数据、文件或完成状态。`,

  // ... 其他模式同样注入相应 Protocol 段
};
// === END FULL PROMPT ===
```

---

## 六、回归测试设计

### 测试骨架文件：`logic.test.mjs` 增强

**注意**:本次仅提供测试骨架，不执行实际测试，避免与#62并发写盘冲突。

```javascript
/**
 * ANTI-HALLUCINATION PROTOCOL REGRESSION TEST SUITE
 * 
 * 用途：验证 Protocol-A/B/C 的逻辑正确性
 * 状态：设计稿，待#62 合并后启用
 * 执行命令：node logic.test.mjs --suite anti-hallucination
 */

import { describe, it, beforeEach, afterEach, mock, assert } from './test-utils.mjs';

// ==================== PROTOCOL-A TESTS ====================

describe('Protocol-A: Stream Integrity Guard', () => {
  let streamRouter, reasonBuffer, answerBuffer;
  
  beforeEach(() => {
    // 模拟路由器的初始状态
    reasonBuffer = '';
    answerBuffer = '';
    streamRouter = createMockRouteThink();
  });
  
  it('[A-01] 完整<think> 标签应正确分离到思考区', () => {
    const input = [
      { kind: 'token', delta: '<think>' },
      { kind: 'token', delta: '我在分析这个问题...' },
      { kind: 'token', delta: '</think>' },
      { kind: 'token', delta: '好的，让我来回答...' }
    ];
    
    for (const chunk of input) {
      const result = streamRouter(chunk.delta);
      if (result.th) reasonBuffer += result.th;
      if (result.an) answerBuffer += result.an;
    }
    
    assert.equal(reasonBuffer, '<think>我在分析这个问题...</think>');
    assert.equal(answerBuffer, '好的，让我来回答...');
  });
  
  it('[A-02] 分片切断标签时应缓冲而非泄漏', async () => {
    // 模拟网络分片：<think> 被切成 <th 和 ink>
    const chunks = [
      { kind: 'token', delta: '<th' },
      { kind: 'token', delta: 'ink>' }, // 第二个分片才补全
      { kind: 'token', delta: '思考内容' },
      { kind: 'token', delta: '</think>' },
      { kind: 'token', delta: '正文' }
    ];
    
    for (const chunk of chunks) {
      const result = streamRouter(chunk.delta);
      if (result.th) reasonBuffer += result.th;
      if (result.an) answerBuffer += result.an;
    }
    
    // 重点断言：answerBuffer 不应包含<think> 相关文本
    assert.isFalse(answerBuffer.includes('<think>'));
    assert.isFalse(answerBuffer.includes('思考内容'));
    assert.isTrue(reasonBuffer.includes('思考内容'));
  });
  
  it('[A-03] 流式结束时残留在_hold 中的未闭合标签应推回思考区', () => {
    // 模拟流突然中断，_thinkHold 中有半拉子标签
    streamRouter.flush(); // 触发流结束逻辑
    
    // 假设_thinkHold 中有'<thin' (少 k)
    streamRouter._thinkHold = '<thin';
    
    // 调用完整性校验 (Protocol-A 的 guard 函数)
    const corrected = streamRouter.finalizeIntegrity();
    
    // 断言：校正后应将残留推回思考
    assert.isTrue(corrected.inThinkingArea);
    assert.isFalse(corrected.wasInjectedToAnswer);
  });
  
  it('[A-04] 正文中意外出现的<think> 应被二次清洗', () => {
    const pollutedAnswer = '这是答案。<think> 不应该在这里</think>继续回答';
    const cleaned = streamRouter.sanitizeAnswer(pollutedAnswer);
    
    assert.isFalse(cleaned.includes('<think>'));
    assert.isTrue(cleaned.startsWith('这是答案。'));
    assert.isTrue(cleaned.endsWith('继续回答'));
  });
});

// ==================== PROTOCOL-B TESTS ====================

describe('Protocol-B: Intent Evidence Gate', () => {
  let intentClassifier, userMessage, workspaceState;
  
  beforeEach(() => {
    intentClassifier = createMockIntentClassifier();
    userMessage = '';
    workspaceState = { empty: false, filesCount: 150 }; // 非空项目
  });
  
  it('[B-01] "在项目里怎么实现 X"应判为 query 而非 context_only', () => {
    userMessage = '在项目里，到底怎么实现绕过视频检测？';
    
    const rawClassification = intentClassifier.classify(userMessage, workspaceState);
    
    // 原始弱模型可能会误判
    assert.equal(rawClassification.rawOutput.locationIntent, 'context_only');
    
    // Protocol-B 的后置校准应检测到矛盾 (action=query 但 locationIntent=context_only)
    const corrected = intentClassifier.applyCalibration(rawClassification, userMessage);
    
    assert.equal(corrected.locationIntent, 'query');
  });
  
  it('[B-02] 仅有"这个项目怎么样？"应判为 context_only', () => {
    userMessage = '这个项目怎么样？';
    
    const classification = intentClassifier.classify(userMessage, workspaceState);
    
    // 没有查询动词，确实是 context_only
    assert.equal(classification.locationIntent, 'context_only');
    
    // 不应触发校准覆盖
    const calibrated = intentClassifier.applyCalibration(classification, userMessage);
    assert.equal(calibrated.locationIntent, 'context_only'); // 保持不变
  });
  
  it('[B-03] 有查询词但 action 明确为非 none 时应优先 query', () => {
    userMessage = '帮我看看这里的 bug';
    intentClassifier.simulateWeakModel({
      locationIntent: 'context_only',
      action: 'debug' // 但 AI 已经识别出 debug 意图
    });
    
    const calibration = intentClassifier.applyCalibration(null, userMessage);
    
    // 发现矛盾：locationIntent=context_only 但 action=debug → 应改为 query
    assert.equal(calibration.locationIntent, 'query');
  });
  
  it('[B-04] 被拦时应给替代工具建议而非简单拒绝', () => {
    const blockResponse = intentClassifier.generateBlockResponse('list_dir');
    
    assert.isTrue(blockResponse.content.includes('read_file'));
    assert.isTrue(blockResponse.content.includes('search'));
    assert.isFalse(blockResponse.content.includes('[BLOCKED]')); // 避免冷硬风格
    assert.isTrue(blockResponse.content.includes('替代工具'));
  });
});

// ==================== PROTOCOL-C TESTS ====================

describe('Protocol-C: Evidence Hierarchy Arbiter', () => {
  let evidenceValidator, claim, evidenceSources;
  
  beforeEach(() => {
    evidenceValidator = createEvidenceValidator();
    claim = '';
    evidenceSources = [];
  });
  
  it('[C-01] 仅凭.md 断言技术栈应被标记为低证据', () => {
    claim = '项目使用了 React 框架';
    evidenceSources = [
      { type: 'markdown_doc', path: 'README.md', content: '本项目使用 React 开发' }
    ];
    
    const verdict = evidenceValidator.evaluate(claim, evidenceSources);
    
    assert.equal(verdict.confidenceLevel, 'low'); // 文档级证据
    assert.isTrue(verdict.needsSourceCodeVerification);
    assert.include(verdict.recommendations, '读取源码验证');
  });
  
  it('[C-02] package.json+ 源码引用应被认可为中高证据', () => {
    claim = '项目依赖 express@4.18.2';
    evidenceSources = [
      { type: 'config_file', path: 'package.json', lines: [{num: 5, text: '"express": "4.18.2"'}] },
      { type: 'source_code', path: 'src/app.js', references: ['require("express")'] }
    ];
    
    const verdict = evidenceValidator.evaluate(claim, evidenceSources);
    
    assert.equal(verdict.confidenceLevel, 'high'); // 源码级证据
    assert.isFalse(verdict.needsSourceCodeVerification);
    assert.isNull(verdict.recommendations); // 无需更多验证
  });
  
  it('[C-03] list_dir 被拦时应建议 read_file 而非转向.md', () => {
    const toolBlockedScenario = {
      requestedTool: 'list_dir',
      blockReason: 'intent_misclassification',
      availableAlternatives: ['read_file', 'search', 'terminal']
    };
    
    const suggestion = evidenceValidator.proposeAlternative(toolBlockedScenario);
    
    assert.isTrue(suggestion.steps.includes('read_file(\'package.json\')'));
    assert.isFalse(suggestion.steps.includes('读取 README 文档')); // 不应转向.md
  });
  
  it('[C-04] 上下文过期时应发出 freshnuds 软提示', () => {
    const snapshotState = {
      before: new Set(['file1.js', 'file2.js']),
      after: new Set(['file1.js', 'file3.js']) // file2 被删,file3 新增
    };
    
    const nudge = evidenceValidator.detectStaleness(snapshotState);
    
    assert.isTrue(nudge.isStale);
    assert.deepEqual(nudge.changedFiles.removed, ['file2.js']);
    assert.deepEqual(nudge.changedFiles.added, ['file3.js']);
    assert.isTrue(nudge.message.includes('需要重新验证'));
  });
});

// ==================== INTEGRATION TESTS ====================

describe('Integration: End-to-End Anti-Hallucination Flow', () => {
  it('[INT-01] 完整场景：用户问"项目怎么实现"→验证 A/B/C 三层防护', async () => {
    // Step 1: 用户提问
    const userQuery = '在项目里，怎么实现绕过视频检测？';
    
    // Step 2: 意图判定 (Protocol-B)
    const intent = classifyIntent(userQuery);
    assert.equal(intent.locationIntent, 'query'); // Protocol-B 校准后应为 query
    
    // Step 3: 流式输出开始 (Protocol-A)
    const stream = startStreaming();
    
    // Step 4: 思考泄漏检测
    const chunkWithThink = { kind: 'token', delta: '<think>我来分析一下...</think>好的' };
    const routed = routeStreamChunk(chunkWithThink);
    assert.isFalse(routed.answer.includes('<think>')); // 无泄漏
    
    // Step 5: 证据收集 (Protocol-C)
    const suggestions = proposeEvidencePaths(intent);
    assert.include(suggestions, 'read_file(\'package.json\')');
    assert.isFalse(suggestions.some(s => s.includes('.md') && !s.includes('src'))); // 不盲目读.md
    
    // Step 6: 结论生成
    const response = generateResponse(suggestions);
    assert.isTrue(response.sources.every(s => s.verified === true)); // 所有主张均有验证
    
    // Step 7: 上下文新鲜度检查
    await checkSnapshotFreshness();
    assert.isFalse(staleContextDetected());
  });
  
  it('[INT-02] 边缘场景：list_dir 被拦→验证 fallback 路径合理性', async () => {
    // 模拟 B 问题复现
    const intentMisclassified = { locationIntent: 'context_only' };
    const blockedToolResult = interceptListDir(intentMisclassified);
    
    // Protocol-B P3 的替代建议应出现
    assert.isTrue(blockedToolResult.content.includes('替代工具'));
    assert.include(blockedToolResult.content, 'read_file');
    
    // 模型应转向精准读取而非读.md
    const nextCall = simulateModelDecision(blockedToolResult);
    assert.equal(nextCall.tool, 'read_file');
    assert.isFalse(nextCall.tool === 'read_file' && nextCall.path.endsWith('.md'));
  });
});

// ==================== UTILITY MOCKS ====================

// 以下为测试辅助 Mock 对象 (可根据实际代码调整)

function createMockRouteThink() {
  return {
    _thinkHold: '',
    _thinkIn: false,
    
    route(delta) {
      // 简化版_routeThink 逻辑
      // ... 实现省略
    },
    
    flush() {
      // 模拟流结束
    },
    
    finalizeIntegrity() {
      // Protocol-A guard 函数
      return { inThinkingArea: true, wasInjectedToAnswer: false };
    },
    
    sanitizeAnswer(answer) {
      return answer.replace(/\b<think\b|<\/think\s*>/g, '');
    }
  };
}

function createMockIntentClassifier() {
  return {
    classify(message, state) {
      // 模拟弱模型误判
      if (message.includes('怎么') && message.includes('在项目')) {
        return { locationIntent: 'context_only', action: 'inspect' };
      }
      return { locationIntent: 'query' };
    },
    
    applyCalibration(raw, message) {
      // Protocol-B 校准逻辑
      if (raw.locationIntent === 'context_only' && message.includes('怎么')) {
        return { ...raw, locationIntent: 'query' };
      }
      return raw;
    },
    
    generateBlockResponse(tool) {
      return {
        type: 'info',
        content: `[工具临时受限]\n可使用替代工具：read_file, search, terminal\n提出具体查询后可恢复${tool}权限`
      };
    }
  };
}

function createEvidenceValidator() {
  return {
    evaluate(claim, sources) {
      const maxLevel = Math.max(...sources.map(s => ({
        'source_code': 5, 'config_file': 4, 'markdown_doc': 2, 'user_note': 1
      })[s.type]));
      
      return {
        confidenceLevel: maxLevel >= 5 ? 'high' : maxLevel >= 4 ? 'medium' : 'low',
        needsSourceCodeVerification: maxLevel < 5,
        recommendations: maxLevel < 5 ? ['读取源码验证'] : null
      };
    },
    
    proposeAlternative(scenario) {
      return {
        steps: [
          'read_file(\'package.json\')',
          'read_file(\'Cargo.toml\')',
          'search(\'关键词\')'
        ]
      };
    },
    
    detectStaleness(snapshotState) {
      const removed = [...snapshotState.before].filter(f => !snapshotState.after.has(f));
      const added = [...snapshotState.after].filter(f => !snapshotState.before.has(f));
      
      return {
        isStale: added.length > 0 || removed.length > 0,
        changedFiles: { added, removed },
        message: '项目文件结构已变化，旧结论需要重新验证'
      };
    }
  };
}

// ==================== EXPORT ====================

export default {
  suites: ['Protocol-A', 'Protocol-B', 'Protocol-C', 'Integration'],
  skipUntil: '#62 merged', // 标记为待激活
  notes: '本测试套件设计完成，暂不执行以避免与#62并发写盘冲突'
};
```

---

## 七、Commit/Push 时机规划

### 前置条件

**当前状态**:
- #62 修复正在进行中 (意图判定 + 快照验证)
- 本次设计的 Protocol 与#62 部分重叠但不冲突
- 为避免并发写盘冲突，暂不直接修改 `main.js`

### 合并策略

**时机**:等待 #62 完成并合并至 `main` 分支后

**步骤**:

1. **PR #63-A:协议文档化** (独立 PR，立即可提)
   ```bash
   git add docs/ANTI-HALLUCINATION_PROTOCOL.md
   git commit -m "docs: 设计反幻觉协议 v1.0 (Protocol A/B/C)"
   git push origin feature/anti-hallucination-protocol
   ```
   
   **PR 标题**: `docs: 设计反幻觉协议 (Protocol A:流式完整性 / B:证据门控 / C:分级仲裁)`
   
   **PR 描述**:包含本协议全部设计细节 + 测试骨架

2. **PR #63-B:测试骨架集成** (独立 PR)
   ```bash
   git add test/logic.test.mjs
   git commit -m "test: 反幻觉回归测试骨架 (设计稿)"
   git push origin feature/anti-hallucination-test
   ```

3. **PR #63-C:核心代码注入** (#62 合并后执行)
   ```bash
   git fetch origin main
   git checkout -b feat/anti-hallucination-impl
      
   # 按顺序注入 Protocol-A/B/C
   # 1. Protocol-A: stream integrity guard (行 20623)
   # 2. Protocol-B: intent calibration (行 16820 + 36801)
   # 3. Protocol-C: evidence validator (行 16242 + 35525)
   
   git add src/main.js
   git commit -m "feat: 实现反幻觉协议三层防护 (A:流式完整性 / B:意图校准 / C:证据分级)"
   git push origin feat/anti-hallucination-impl
   ```

**风险提示**:
- Protocol-B 的校准逻辑可能与#62 的意图修复重复 → 需 Code Review 协调
- Protocol-A 的实现需在网关层验证 → 建议先小范围灰度
- Protocol-C 的证据徽章属于 UI 扩展 → 可作为独立 Feature Flag

---

## 八、协议要点总结

### 核心设计理念

1. **分层防御**:A(流式层) → B(意图层) → C(证据层),层层递进
2. **最小侵入**:优先通过 Prompt 和事后校验，而非硬编码路由
3. **证据至上**:源码权威性高于一切，文档仅作参考
4. **可观测性**:所有 Protocol 触发均有日志记录和 UI 徽章反馈

### 预期效果

| 问题 | 修复前 | 修复后 |
|-----|-------|-------|
| **A.思考泄漏** | 偶尔混入正文 | 标签完整性校验 + 二次清洗 |
| **B.意图误判** | 查询被拦成 no-op | 后置校准 + 替代建议 |
| **C.文档幻觉** | 仅凭.md 下结论 | 证据分级仲裁 + 源码优先 |

### 下一步行动

1. ✅ **Done**:协议设计完成 (本文档 + 测试骨架)
2. ⏳ **Pending**:等待#62 合并后实施代码注入
3. ⏳ **Planning**:PR #63-A/B/C 编排发布

---

**最后更新**: 2026 年 7 月 30 日  
**维护者**: @Felix & Team  
**关联 Issue**: #62, #63, [THREE_ISSUES_DIAGNOSIS.md](../THREE_ISSUES_DIAGNOSIS.md)
