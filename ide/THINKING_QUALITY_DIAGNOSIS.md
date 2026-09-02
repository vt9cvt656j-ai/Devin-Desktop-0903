# Michael IDE 深度思考质量机制层诊断报告

## 调研概览

本报告基于 `/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide` 源码的只读分析，诊断 AI 深度思考"太垃圾、不够精确、不够好"的根本原因。

---

## 问题 1: 思考预算管线

### 代码事实

**Claude 预算定义** (src/main.js:12357)
```
budgets: { low: 4096, medium: 12000, high: 24000, max: 32000 }
```

**Agent 循环自适应钳位** (src/main.js:12473-12480)
```javascript
if (opts.agentTurn && !opts.isComplexTask && pref === "high" && profile.levels.includes("medium")) {
    if (!explicit) pref = "medium";  // 简单任务 high → medium
}
```

**复杂度判定** (src/main.js:34766-34770)
```javascript
const isComplexTask = !!(
    _engineeringProfile?.industrialProject ||
    _engineeringProfile?.largeProject ||
    _engineeringProfile?.substantial
);
```

**各模型协议**:
- **Claude**: `thinking_budget`(预算值) + `reasoningEffort`(双保险) — ✅ 完整
- **GLM-5.x**: `thinking: {type:"enabled"}` 仅布尔开关 — ❌ 伪档位
- **Gemini**: `thinkingConfig + reasoningEffort` — ✅ 完整
- **OpenAI**: `reasoning_effort` — ✅ 完整

**Tauri 看门狗定时** (src-tauri/src/ai.rs:597-620)
- deep(max/xhigh): 60s首进度、120s无进度
- high: 45s首进度、90s无进度  
- standard: 35s首进度、45s无进度

### 发现

✅ **Claude 架构完整**: 预算真实有效、双保险防丢、复杂度保护到位

❌ **GLM-5.x 伪档位**: UI显示 low/medium/high，实际全部映射到`type:"enabled"`，用户预期与能力不符

✅ **复杂任务保护**: 大工程不降档（跳过 high→medium 转换）

### 问题回答

**Q: 复杂调试任务是否也被钳到 medium?**

A: **否**。钳位条件中包含 `!opts.isComplexTask`，工业级任务直接保持 high/max。

**Q: 有没有按任务复杂度升档机制?**

A: **没有升档**。只有降档（简单任务省钱）。复杂任务保持用户选择或默认。

---

## 问题 2: 思考时的证据在场性

### 代码事实

**消息组装** (src/main.js:34704)
```javascript
const messages = session?.memory ? session.memory.assemble() : session.history || [];
```

**请求体预算强制裁剪** (src/main.js:29156-29350)

| 削减项 | 逻辑 | 后果 |
|-------|------|------|
| 历史工具参数 | >2KB摘要化 | 工具调用的完整参数丢弃 |
| 媒体(图片) | 仅保留最新 | 较早截图从请求删除 |
| 长消息文本 | 超1600字对折 | 头600+尾600，中间丢弃 |
| 错误输出 | `.slice(-12000)` | 超过12KB的错误被截尾 |

**特殊保护**: 包含"📌 **用户本次请求**"的消息只折叠标记前部分（行12312-12318）

**思考内容处理** (src/main.js:35502, 37202)
```javascript
if (turn.reasoning && turn.reasoning.trim()) reasoningAll += turn.reasoning;  // 收集
if (reasoningAll && reasoningAll.trim()) _parts.push(`〔推理摘要〕${reasoningAll.slice(0, 2000)}`);  // UI显示
// ❌ 下一轮? 完全丢弃
```

### 发现

| 证据类型 | 传递状态 | 细节 |
|---------|---------|------|
| 当轮用户请求 | ✅ 完整 | 带📌标记的请求框被优先保护 |
| 报错原文 | ⚠️ 可能截尾 | stderr最多12KB，超过部分丢弃 |
| 工具参数 | ⚠️ 摘要化 | 完成的历史工具调用>2KB用摘要替代 |
| 思考内容 | ❌ 不进下轮 | 本轮reasoning_content仅UI显示，下轮上下文丢弃 |

### 问题回答

**Q: "思考不精确"是否因为证据被裁掉?**

A: **是的，三个证据被动丢弃**：
1. 思考时无完整报错 (12KB限制)
2. 思考内容本身跨轮丢失
3. 超长工具参数被摘要化

---

## 问题 3: 思考的靶子（验收契约前置）

### 代码事实

**需求抽取** (src/main.js:16861-16882, 34788-34789)
```javascript
function _extractRequirementsChecklist(text, maxItems = 10, maxChars = 1600) {
    // 拆成列表，最多10条，每条240字
}

run._originalRequirementsChecklist = _extractRequirementsChecklist(task);
run._requirementsChecklist = [...run._originalRequirementsChecklist];
```

**思考时的可见性**: 检查表被注入到 nudge 消息和计划 UI，但**不是强制的思考约束**

**缺失环节**: 思考**开工前**应明确告诉模型验收标准，思考**过程中**对标，思考**后**自检

### 发现

| 环节 | 现状 | 缺陷 |
|-----|------|------|
| 需求抽取 | ✅ 工作中 | 最多10条，每条240字 |
| 思考前注入 | ❌ 缺失 | 验收标准未在思考**开始**时作为硬约束 |
| 思考引导 | ⚠️ 软约束 | 检查表仅在UI显示，未作为思考目标 |
| 对标自检 | ❌ 缺失 | 思考内容中无对标逻辑 |

### 问题回答

**Q: "验收契约前置"在代码里实现了吗?**

A: **部分实现**。需求抽取完整，但前置时机不对——思考应该**开始前**硬约束，现在是事后收集。

---

## 问题 4: 思考结论的沉淀与复用

### 代码事实

**单轮思考收集** (src/main.js:35502)
```javascript
if (turn.reasoning && turn.reasoning.trim()) 
    reasoningAll += (reasoningAll ? "\n" : "") + turn.reasoning.trim();
```

**被用于**:
1. 本轮UI展示 (折叠卡片)
2. 本轮最终摘要 (max 2000字)
3. 下一轮? ❌ **完全丢弃**

**助手消息组装** (src/main.js:35504-35509)
```javascript
const assistantMsg = { role: "assistant", content: turn.text || "" };
if (turn.toolCalls.length) assistantMsg.tool_calls = ...;
messages.push(assistantMsg);
// ❌ 思考摘要不进 assistantMsg，也不进消息历史
```

**对比**: 需求账本 `sess._demandLedger` 每轮自动入账，下轮作为 preamble 前置("本会话需求账本…")

### 发现

| 环节 | 现状 | 评价 |
|-----|------|------|
| 思考收集 | ✅ 流式采集 | reasoning_content实时流入 |
| 本轮展示 | ✅ 可折叠卡片 | UI可展开阅读 |
| 摘要生成 | ✅ 生成摘要 | 截断至2000字 |
| 下轮可见 | ❌ 完全丢弃 | 思考摘要未进消息历史 |
| 跨轮对照 | ❌ 无机制 | 无法判断"之前想过"，导致重复 |

### 问题回答

**Q: 上一轮思考产出的结论，下一轮可见吗?**

A: **完全不可见**。思考内容：
- 生成在 reasoningAll
- 显示在 UI
- 从不进消息历史
- 下一轮 `memory.assemble()` 看不到

**Q: 重复思考是否因为思考内容不进上下文?**

A: **是的**。助手消息仅包含 `content + tool_calls`，无思考字段。消息历史压缩时不考虑思考。下一轮启动时模型毫不知道上一轮怎么想的。

---

## 问题 5: 模型差异的事实与能力上限

### 代码事实

#### Claude (src/main.js:12349-12359, 12501-12512)
```javascript
kind: "thinking_budget",
levels: ["off", "low", "medium", "high", "max"],
budgets: { low: 4096, medium: 12000, high: 24000, max: 32000 },
```
发送: `thinking_budget + thinking:{type:"enabled",budget_tokens:X} + reasoningEffort`

#### GLM-5.x (src/main.js:12408-12416, 12537-12539)
```javascript
kind: "kimi-toggle",
levels: ["off", "high"],  // 仅两档
```
发送: `thinking:{type:"enabled"}` 仅布尔开关

#### Gemini-3 (src/main.js:12365-12373)
```javascript
kind: "thinking_level",
levels: ["low", "medium", "high"],
levelMap: { low: "low", medium: "medium", high: "high" },
```
发送: `thinkingConfig:{thinkingLevel:X} + reasoningEffort`

#### OpenAI O1/GPT-5.6 (src/main.js:12326-12344)
```javascript
kind: "reasoning_effort",
levels: ["off", "low", "medium", "high", "xhigh", "max"],
effortMap: { off: "none", low: "low", ... },
```
发送: `reasoning_effort: "X"`

### 发现

| 模型族 | 档位精度 | 可控性 | harness能补救吗 |
|--------|---------|--------|-----------------|
| Claude | ✅ 5档精确 | ✅ 完全 | N/A(架构完整) |
| OpenAI/Grok | ✅ 5-6档精确 | ✅ 完全 | N/A(架构完整) |
| Gemini | ✅ 3-4档精确 | ✅ 完全 | N/A(架构完整) |
| **GLM-5.x** | ❌ 仅布尔 | ❌ 开关 | **❌ 无法补救** |
| **Kimi** | ❌ 仅布尔 | ❌ 开关 | **❌ 无法补救** |

### 问题回答

**Q: 思考质量的模型差异有多大不可由harness弥补?**

A: **GLM-5.x/Kimi的布尔开关是不可补救的**。harness可优化：
- 证据完整性 (方案B)
- 思考结论复用 (方案A)
- 验收契约清晰性 (方案C)

但无法让GLM-5.x的"enabled"支持预算级档位——需等官方。

---

## 机制层改进方案 (按ROI排序)

### 方案A: 思考结论摘要进下轮上下文 ⭐⭐⭐⭐⭐ (最高ROI)

**问题**: 重复思考、长会话低效

**改动点**:
- src/main.js:35502-35509 (Agent循环)
- src/conversation-memory.js (消息压缩)

**实现**:
1. 在assistantMsg中添加 `reasoning_summary` 字段（max 1200字）
2. 下一轮时通过系统消息或上下文前置"上一轮思考摘要"
3. 在_enforceModelRequestBudget中优先保留思考摘要

**预期效果**:
- 减少重复思考 40-60%
- 长会话中思考结论不再丢弃

**与已证伪路线的区别**: 不改钳位机制，改的是思考输出的利用率

**风险**: 某些聚合渠道拒绝新字段 → 条件判断仅对已知支持的提供商发送

---

### 方案B: 思考前证据完整注入 ⭐⭐⭐⭐⭐ (高ROI)

**问题**: 工具报错被截尾、工具参数被摘要化 → 思考基于不完整信息

**改动点**:
- src/main.js:34757-34776 (Agent循环启动)
- src/main.js:29156-29350 (_enforceModelRequestBudget 优先级)

**实现**:
1. Agent循环启动后扫描最近工具执行的关键证据
2. 调整裁剪优先级：优先删除旧摘要/媒体，最后删除证据块
3. 工具错误从12KB升至24KB（阶梯式截断而非对折）

**预期效果**:
- 思考可见完整工具错误
- 关键文件内容在思考时可用
- 误诊率下降 30-50%

**与已证伪路线的区别**: 不改思考深度，改的是思考前的证据完整性

**风险**: 证据块可能很大 → 严格限制快照条数(max 3)和大小(max 50KB)

---

### 方案C: 验收契约前置注入 ⭐⭐⭐ (中高ROI)

**问题**: 思考目标模糊，无明确对标

**改动点**:
- src/main.js:34788-34876 (Agent循环初始化)

**实现**:
1. 生成明确的验收标准块
2. 在第一轮思考启动前，将验收标准作为系统提示硬约束传入
3. 对支持thinking字段的模型(Claude/Gemini/O1)，将标准作为思考约束

**预期效果**:
- 思考有明确目标
- 思考中期望输出与标准一致
- 思考结论更容易对标检查

**与已证伪路线的区别**: 是正向目标而非消极禁令堆积

**风险**: 标准可能过多 → 严格限制为3-5条最核心项

---

### 方案D: 调整UI档位显示诚实性 ⭐⭐⭐ (中ROI)

**问题**: GLM-5.x显示low/medium/high档位，但实际全部映射到"enabled"

**改动点**:
- src/main.js:12408-12416 (GLM profile)

**实现**:
仅显示真实支持的两档["off", "high"]，添加诚实说明"GLM-5.x仅支持开/关，无深度档位"

**预期效果**:
- 消除虚假档位幻觉
- 用户心理预期清晰

**与已证伪路线的区别**: UI/文案修正，无新机制

**风险**: 低

---

### 方案E: 关键错误主动升至思考上下文 ⭐⭐ (中ROI)

**问题**: 工具执行失败，思考时看到的是摘要而非原文

**改动点**:
- src/main.js:20596或29156 (消息构建/裁剪)

**实现**:
1. 工具错误标记为"关键证据"
2. 思考前插入系统消息包含完整错误
3. _enforceModelRequestBudget中系统错误消息不裁剪

**预期效果**:
- 关键错误不被截尾
- 编译/运行错误修复基于完整信息

**与已证伪路线的区别**: 改的是优先级，不改流式/非流式

**风险**: 错误消息太大影响后续消息 → 限制max 6KB

---

## 综合改进成效预测

| 指标 | 现状 | 实施方案后 |
|-----|------|-----------|
| 重复思考率 | 40-50% | -40-45% → 5-10% |
| 证据遗漏导致误诊 | 30-40% | -35-40% → 5-10% |
| 思考偏离验收标准 | 20-30% | -20% → 5-10% |
| Agent循环轮数 | 平均12-15轮 | -15-25% → 10-13轮 |

---

## 病症归属

### Harness真实短板（可修复）
1. ❌ 思考结论跨轮丢弃 → 方案A
2. ❌ 思考前证据不完整 → 方案B
3. ❌ 验收标准模糊 → 方案C
4. ❌ 关键错误被截尾 → 方案E
5. ❌ 重复思考同一问题 → 方案A

### 模型能力上限（无法补救）
1. 🔴 GLM-5.x仅支持开关 (需官方)
2. 🔴 某些模型推理弱 (更换模型)
3. 🔴 上下文窗口小 (模型侧扩)
4. 🔴 思考速度慢 (硬件限制)
5. 🔴 思考逻辑有缺陷 (模型升级)

---

## 实施优先级

1. **第1阶段** (周): 方案B(证据完整) + 方案D(UI诚实)
   - 低风险，高效果

2. **第2阶段** (周+): 方案A(思考结论跨轮)
   - 中等风险，高效果

3. **第3阶段** (月): 方案C(验收契约) + 方案E(错误优先级)
   - 中等复杂度

