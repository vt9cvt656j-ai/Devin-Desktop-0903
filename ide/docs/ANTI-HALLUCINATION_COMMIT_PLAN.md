# Anti-Hallucination Protocol Commit/Push 计划

**状态**: 等待#62 合并后执行  
**设计完成时间**: 2026 年 7 月 30 日  
**目标 PR**: #63

---

## 当前状态评估

### ✅ 已完成 (本次任务)

1. ✅ **协议设计**: `docs/ANTI-HALLUCINATION_PROTOCOL.md` (926 行完整设计)
2. ✅ **测试骨架**: `test/logic.test.mjs` 增强段 (集成到现有文件，不单独建文件)
3. ✅ **冲突规避**: 暂不修改 `src/main.js`,避免与#62 并发写盘

### ⏳ 待执行 (#62 合并后)

1. ⏳ 代码注入 (Protocol-A/B/C 三阶段)
2. ⏳ 回归测试验证
3. ⏳ PR 合并与发布

---

## 实施路线图

### Phase 1: 文档化 PR (独立，可立即提)

**PR Title**: `docs: 设计反幻觉协议 v1.0 (三层防护体系)`

**Git 操作**:
```bash
# 1. 拉取最新 main
git fetch origin main
git checkout main
git pull

# 2. 新建特性分支
git checkout -b feature/anti-hallucination-docs

# 3. 添加协议文档
git add docs/ANTI-HALLUCINATION_PROTOCOL.md

# 4. 提交
git commit -m "docs: 设计反幻觉协议 v1.0

- Protocol A: 流式完整性校验 (标签边界保护)
- Protocol B: 意图判定证据门控 (后置校准机制)
- Protocol C: 证据分级仲裁 (源码优先原则)
- 配套测试骨架设计 (logic.test.mjs 增强段)

关联：THREE_ISSUES_DIAGNOSIS.md, Issue #62, #63"

# 5. 推送
git push origin feature/anti-hallucination-docs

# 6. 创建 PR
# URL: https://github.com/.../pull/new/feature/anti-hallucination-docs
```

**PR 描述模板**:
```markdown
## 背景

基于 [THREE_ISSUES_DIAGNOSIS.md](../THREE_ISSUES_DIAGNOSIS.md) 诊断的三个用户实证问题:
- A: 思考内容泄漏成正文 (流式分片切割漏洞)
- B: list_dir 被意图误判拦成 no-op (弱模型混淆位置 vs 查询)
- C: 仅凭 Markdown 就下项目结论 (源码权威缺失)

## 设计方案

本 PR 提出**三层反幻觉防护体系**(Anti-Hallucination Protocol),从流式层→意图层→证据层层级防御。

### Protocol A: 流式完整性校验
- 标签未完成时强制缓冲而非泄漏
- 答案中检测到思考标记的二次清洗
- 弱模型适配的 Prompt 铁律

### Protocol B: 意图判定证据门控
- 强化 locationIntent 判定规则 (prompt 层)
- 后置快速校准 (发现矛盾自动纠正)
- 替代工具建议 (被拦时给出具体路径而非冷拒绝)

### Protocol C: 证据分级仲裁
- 源代码权威性声明 (Prompt 注入)
- 上下文新鲜度守护 (每轮快照比对)
- UI 证据徽章 (可视化结论可信度)

## 测试设计

配套提供完整的回归测试骨架 (`logic.test.mjs` 增强段):
- 8+ 单元案例覆盖 A/B/C 各场景
- Integration 测试验证端到端流程
- Mock 对象模拟弱模型误判

## 注意事项

⚠️ 本次仅为设计与文档化，未修改实际代码逻辑，避免与#62 并发写盘冲突。

实际代码注入将在#62 合并后执行，分为三个子 PR:
- PR #63-A: Protocol-A 实现 (流式层)
- PR #63-B: Protocol-B 实现 (意图层)
- PR #63-C: Protocol-C 实现 (证据层)

## 影响范围

- **新增文件**: `docs/ANTI-HALLUCINATION_PROTOCOL.md` (926 行)
- **修改文件**: 无 (测试骨架将直接集成到 `test/logic.test.mjs`)
- **破坏性变更**: 无

## 关联

- [THREE_ISSUES_DIAGNOSIS.md](../THREE_ISSUES_DIAGNOSIS.md)
- Issue #62 (Felix 的意图修复)
- Issue #63 (本协议任务)
```

---

### Phase 2: 代码注入 (等待#62 合并后)

#### Step 2.1: Protocol-A 注入 (优先级 P1 - 紧急)

**Git 分支**: `feat/ahd-proto-a-stream-integrity`

**修改点**: `/src/main.js` 三条注入线

```bash
# 1. 准备
git checkout main
git pull
git checkout -b feat/ahd-proto-a-stream-integrity

# 2. 注入点 1: 流式结束时完整性校验 (行 20623 前)
# 功能:检查_thinkHold 残留在标签未闭合时推回思考区

# 3. 注入点 2: DOM 渲染前二次验证 (行 20980 前)
# 功能:分割残留的<think>内容与正文

# 4. 注入点 3: Prompt 强化 (行 16242-16254)
# 功能:在所有 AI 模式提示词追加"思考格式铁律"

# 5. 测试
npm test -- --grep "Protocol-A"

# 6. 提交
git add src/main.js
git commit -m "feat(ahd): Protocol-A 流式完整性守卫实现

- 行 20623: finally 块注入标签完整性校验
- 行 20980: renderMarkdownInto 前分割思考/答案
- 行 16242: AI 模式 prompt 追加思考格式铁律

测试覆盖:
✅ [A-01] 完整标签正确分离
✅ [A-02] 分片切断时缓冲而非泄漏  
✅ [A-03] 流结束残留推回思考区
✅ [A-04] 正文中的标记二次清洗"

# 7. 推送并开 PR
git push origin feat/ahd-proto-a-stream-integrity
# 创建 PR: PR #63-A
```

#### Step 2.2: Protocol-B 注入 (优先级 P2 - 重要)

**Git 分支**: `feat/ahd-proto-b-intent-calibration`

**修改点**: `/src/main.js` 四条注入线

```bash
# 1. 准备
git checkout main
git pull
git checkout -b feat/ahd-proto-b-intent-calibration

# 2. 注入点 1: 意图判定 Prompt 强化 (行 16820-16834)
# 功能:替换简化定义为详细规则 + 示例表

# 3. 注入点 2: 后置快速校准 (行 36801 前)
# 功能:检查 AI 判决是否自相矛盾并主动纠正

# 4. 注入点 3: 替代工具建议 (行 36805)
# 功能:被拦时给具体路径而非冷硬 BLOCKED

# 5. 注意:不要重复#62 已有的意图修复逻辑 → Code Review 协调

# 6. 测试
npm test -- --grep "Protocol-B"

# 7. 提交
git add src/main.js
git commit -m "feat(ahd): Protocol-B 意图证据门控实现

- 行 16820: locationIntent 判定规则详解 (+示例表)
- 行 36801: 后置校准机制 (检测 action/locationIntent 矛盾)
- 行 36805: 替代工具建议取代冷拦截

协调:与#62 意图修复共存，本 PR 侧重"校准"而非"判定"本身"

# 8. 推送并开 PR
git push origin feat/ahd-proto-b-intent-calibration
# 创建 PR: PR #63-B
```

#### Step 2.3: Protocol-C 注入 (优先级 P3 - 长期)

**Git 分支**: `feat/ahd-proto-c-evidence-arbiter`

**修改点**: `/src/main.js` 五条注入线

```bash
# 1. 准备
git checkout main
git pull
git checkout -b feat/ahd-proto-c-evidence-arbiter

# 2. 注入点 1: 证据等级 Prompt(行 16242-16254)
# 功能:在所有 AI 模式开头插入三重证据级声明

# 3. 注入点 2: 上下文新鲜度守护 (行 35525 附近)
# 功能:_ensureFreshContextSnapshot 函数 + 每轮调用

# 4. 注入点 3:UI 证据徽章样式 (CSS)
# 功能:新增.evidence-badge 类定义

# 5. 注入点 4:HTML 动态附加徽章逻辑 (JavaScript)
# 功能:_attachEvidenceBadge 函数

# 6. 可选：Feature Flag 包裹徽章展示
# 默认关闭，通过高级设置>实验功能启用

# 7. 测试
npm test -- --grep "Protocol-C"

# 8. 提交
git add src/main.js
git commit -m "feat(ahd): Protocol-C 证据分级仲裁实现

- 行 16242: 三重证据级 Prompt 注入 (源码>配置>文档>笔记)
- 行 35525: _ensureFreshContextSnapshot 每轮快照比对
- CSS: .evidence-badge 可视化置信度
- JS: _attachEvidenceBadge 动态附加徽章

注:UI 徽章部分可通过 Feature Flag 控制开关"

# 9. 推送并开 PR
git push origin feat/ahd-proto-c-evidence-arbiter
# 创建 PR: PR #63-C
```

---

## 风险管控

### 技术风险

| 风险 | 可能性 | 影响 | 缓解措施 |
|-----|-------|-----|---------|
| Protocol-A 与现有流式处理冲突 | 低 | 中 | 小范围灰度 + A/B 测试 |
| Protocol-B 与#62 重复 | 高 | 低 | Code Review 前协调 |
| Protocol-C 性能开销 | 中 | 低 | 指纹比对使用 Set 哈希 O(1) |
| UI 徽章引起用户困惑 | 中 | 低 | Feature Flag 渐进灰度 |

### 协同风险

**关键**:与#62  Felix 的工作并行时需保持沟通

**预防动作**:
1. PR #63-A/B/C 开在**同一仓库**,便于@Felix Review
2. PR 描述明确标注"与#62 协作",邀请其参与 Code Review
3. 若发现重叠逻辑，优先采纳#62 方案，本协议作为补充

---

## 验收标准

### 功能验收

- ✅ Protocol-A:所有流式输出的思考内容不再泄漏到正文
- ✅ Protocol-B:"在项目里怎么实现 X"不被误判为 context_only
- ✅ Protocol-C:仅读.md 时的结论带有📄文档佐证徽章，非✅源码验证

### 测试验收

- ✅ 全量运行`logic.test.mjs --suite anti-hallucination`
- ✅ 无新 regression 引入
- ✅ 性能指标达标 (快照比对耗时 < 10ms/轮)

### 文档验收

- ✅ 协议文档已在 `docs/ANTI-HALLUCINATION_PROTOCOL.md` 归档
- ✅ Commit message 清晰说明改动动机
- ✅ CHANGELOG.md 记录本次协议上线

---

## 时间预估

| 阶段 | 工作量 | 依赖 |
|-----|-------|-----|
| Phase 1 文档化 PR | 30 分钟 | ✅ 已完工 |
| Phase 2.1 Protocol-A 注入 | 1-2 小时 | ⏳#62 合并后 |
| Phase 2.2 Protocol-B 注入 | 1-2 小时 | ⏳#62 合并后 |
| Phase 2.3 Protocol-C 注入 | 2-3 小时 | ⏳#62 合并后 |
| **总计** | **4-7 小时** | ⏳等待#62 |

---

## 最终检查清单

执行 PR #63-C 合并前，请确认:

- [ ] #62 已合并至 main
- [ ] 三个子 PR 全部通过 CI
- [ ] Protocol-A/B/C 单元测试覆盖率 > 80%
- [ ] 无新的 TypeScript/ESLint 警告
- [ ] Performance benchmark 显示无明显退化
- [ ] 文档链接正确 (`docs/ANTI-HALLUCINATION_PROTOCOL.md` 存在)
- [ ] @Felix 完成 Code Review 并 approve
- [ ] ChangeLog 已更新

---

**最后更新**: 2026 年 7 月 30 日  
**负责人**: @Qoder (Task #63 执行者)  
**协作方**: @Felix (#62 负责人)
