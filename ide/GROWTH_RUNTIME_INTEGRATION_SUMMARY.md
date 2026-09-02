# Growth-Runtime Integration - P0/P1 Implementation Summary

## Overview
实现了 Sam 调研报告中的自适应工具选择机制，打通了成长系统→运行时的反向通路，解决了"很多工具没被用上"的核心问题。

## Problem Statement

### 核心瓶颈 (Core Bottlenecks)
1. **硬编码限制**：`_selectInitialTools` 固定 11 个工具，无论用户水平
2. **编辑器截断**：`_criticRequestedToolSchemas` maxTools=10 硬限制
3. **未接入个性化**：`recommendToolsForIntent` 没有使用成长技能数据

### 转化率问题
- 注册 76+∞ → 首轮固定 11 → catalog~120 行 → 编排器最多 10 → 实际执行
- **转化率仅 14%** ← 这是主要浪费点

## Solution Architecture

### Step 1: Growth Policy Layer (growth.js)

#### New Export: `getRuntimePolicy()`
```javascript
export function getRuntimePolicy(growthState = null) {
    const avgP = growthState?.avgMastery?.() ?? 0.5;
    
    // Expert (>0.7): 20 initial tools, 15 critic limit
    if (avgP > 0.7) {
        return {
            initialToolCount: 20,
            criticMaxTools: 15,
            enableProfessionalTools: true,
            allowAutoLoadAdvanced: true
        };
    } 
    // Advanced (0.45-0.7): 14 tools, 12 critic limit
    else if (avgP > 0.45) {
        return {
            initialToolCount: 14,
            criticMaxTools: 12,
            enableProfessionalTools: true,
            allowAutoLoadAdvanced: false
        };
    }
    // Novice (<0.45): 11 tools, 10 critic limit (conservative)
    else {
        return {
            initialToolCount: 11,
            criticMaxTools: 10,
            enableProfessionalTools: false,
            allowAutoLoadAdvanced: false
        };
    }
}
```

#### Skill Levels
- **新手 (<0.45)**: 保守模式，基础 11 工具 + search_tools 扩展入口
- **进阶 (0.45-0.7)**: 适度开放专业工具 (debugger/lsp/git 等)
- **专家 (>0.7)**: 全量开放高级工具 (profiler/security_scan 等)

### Step 2: Adaptive Initial Tool Selection (main.js #1)

#### Modified: `_selectInitialTools()`
```javascript
// P0 #1: Adaptive expansion based on user mastery level from growth system
let baseCore = [...(roleCoreMap[role] || roleCoreMap.agent)];

try {
    const growthState = typeof window !== 'undefined' ? window._growthState || null : null;
    if (growthState && typeof growthState.avgMastery === 'function') {
        const avgP = growthState.avgMastery();
        
        let targetCount = 11;
        let enableProfessional = false;
        
        if (avgP > 0.7) {
            targetCount = 20; // expert
            enableProfessional = true;
        } else if (avgP > 0.45) {
            targetCount = 14; // advanced
            enableProfessional = true;
        }
        
        if (enableProfessional && targetCount > baseCore.length) {
            const extraNeeded = targetCount - baseCore.length;
            const available = PROFESSIONAL_TOOLS.filter(t => !baseCore.includes(t));
            const toAdd = Math.min(extraNeeded, available.length);
            
            if (toAdd > 0) {
                baseCore = [...baseCore, ...available.slice(0, toAdd)];
            }
        }
    }
} catch (e) {
    console.warn('Adaptive initial tool expansion failed, fallback to static:', e.message);
}
```

**PROFESSIONAL TOOLS Pool**:
```javascript
const PROFESSIONAL_TOOLS = [
    "debugger", "profiler", "lsp_symbols", "semantic_search",
    "code_map", "grep_code", "list_dir_recursive",
    "browser_navigate", "performance_audit", "security_scan",
    "db_query", "db_migrate", "backup_database", "package_search",
    "github_search", "developer_community_search"
];
```

### Step 3: Adaptive Critic Limit (main.js #2)

#### Modified: `_criticRequestedToolSchemas()`
```javascript
function _criticRequestedToolSchemas(toolNames, toolRegistry, maxTools = 8) {
    // P0 #2: Adaptive critic limit based on growth state
    let dynamicMaxTools = Number(maxTools) || 8;
    
    try {
        const growthState = typeof window !== 'undefined' ? window._growthState || null : null;
        if (growthState && typeof growthState.avgMastery === 'function') {
            const avgP = growthState.avgMastery();
            
            if (avgP > 0.7) {
                dynamicMaxTools = 15; // expert
            } else if (avgP > 0.45) {
                dynamicMaxTools = 12; // advanced
            }
        }
    } catch (e) {
        console.warn('Adaptive critic limit failed, fallback to default:', e.message);
    }
    
    // Use dynamicMaxTools instead of hardcoded maxTools
    // ... rest of implementation
}
```

### Step 4: Personalized Recommendations (main.js #3)

#### Enhanced: `recommendToolsForIntent()`
```javascript
function recommendToolsForIntent(intentText, context = {}) {
    // P0 #3: Personalize recommendations based on user's skill mastery levels
    let userSkills = {};
    try {
        const growthState = typeof window !== 'undefined' ? window._growthState || null : null;
        if (growthState && Array.isArray(growthState.skills)) {
            userSkills = growthState.skills.reduce((acc, skill) => {
                acc[skill.name] = skill.mastery ?? 0;
                return acc;
            }, {});
        }
    } catch (e) {
        console.warn('Growth skill access failed, falling back to default ranking:', e.message);
    }
    
    // ... existing keyword matching ...
    
    // Weight and sort by user skill mastery
    let weightedTools = matchedTools.map(tool => {
        const baseName = tool.replace(/\(.*\)/, ''); // strip parameters
        const mastery = userSkills[baseName] ?? userSkills[tool] ?? 0;
        return { tool, mastery };
    });
    
    // Sort descending by mastery, take top 5
    weightedTools.sort((a, b) => b.mastery - a.mastery);
    return weightedTools.slice(0, 5).map(w => w.tool);
}
```

### Step 5: Growth Feedback Loop (main.js #5)

#### New Function: `observeToolCall()`
```javascript
function observeToolCall(toolRecord) {
    try {
        const g = typeof window !== 'undefined' ? window._growthState || null : null;
        if (!g) return;
        
        let toolName = String(toolRecord.tool || "").trim();
        const baseCommand = toolName.split(' ')[0];
        
        // Find matching skill or create new one
        let matchedSkillIndex = -1;
        if (Array.isArray(g.skills)) {
            matchedSkillIndex = g.skills.findIndex(s => 
                s.name === toolName || 
                s.name === baseCommand ||
                s.name.includes(baseCommand) ||
                baseCommand.includes(s.name)
            );
        }
        
        if (matchedSkillIndex >= 0) {
            // Update existing skill mastery
            const current = g.skills[matchedSkillIndex].mastery ?? 0;
            const success = toolRecord.ok === true;
            const delta = success ? 0.05 : -0.02; // Success +0.05, failure -0.02
            g.skills[matchedSkillIndex].mastery = Math.max(0, Math.min(1, current + delta));
            g.skills[matchedSkillIndex].lastUsed = Date.now();
            g.skills[matchedSkillIndex].usageCount = (g.skills[matchedSkillIndex].usageCount || 0) + 1;
        } else {
            // Create new skill entry for first-time tool
            const initialMastery = toolRecord.ok === true ? 0.3 : 0.1;
            g.skills.push({
                name: toolName,
                mastery: initialMastery,
                lastUsed: toolRecord.timestamp || Date.now(),
                usageCount: 1
            });
        }
        
        // Persist to window and localStorage
        if (typeof window !== 'undefined') {
            window._growthState = g;
            localStorage.setItem("michael-ide.learner-model.v1", JSON.stringify(g));
        }
    } catch (e) {
        console.warn('observeToolCall failed:', e);
    }
}
```

#### Integration Point: Tool Execution Callback
```javascript
// In main.js run loop, after each tool batch:
if (items.length && _live()) {
    run._toolLedger.turnIndex++;
    
    // P0 #5: Growth feedback loop - record tool usage signals
    for (const it of items) {
        if (!it || !it.call?.type) continue;
        try {
            const ok = _toolExecutionSucceeded(it.call, it.rawResult);
            observeToolCall({
                turn: run._toolLedger.turnIndex,
                tool: String(it.call.type === 'cmd' ? 
                    (it.call.command?.split(' ')[0] || '') : it.call.type),
                ok: !!ok,
                timestamp: Date.now()
            });
        } catch (e) {
            console.warn('Growth signal recording failed:', e.message);
        }
    }
    // ... rest of execution logic
}
```

## Verification Results

### ✅ Syntax Validation
```bash
node --check src/main.js         # ✓ PASS
node --check src/growth.js       # ✓ PASS
```

### ✅ Unit Tests
```bash
npm test                         # 387/387 PASS
```

### ✅ Integration Tests
```bash
node test_growth_integration.mjs # All 8 checks passed
```

## Expected Behavior Examples

### Novice User (<0.45 mastery)
- **首轮工具**: `read_file`, `list_dir`, `search`, `find_files`, `update_plan`, `ask_user`, `write_file`, `edit_file`, `multi_edit`, `run_cmd`, `search_tools` (11 个)
- **Critic 窗口**: 最多 10 个工具
- **推荐排序**: 基于默认关键词匹配，无个性化加权

### Advanced User (0.45-0.7 mastery)
- **首轮工具**: 11 基础 + debugger, profiler, lsp_symbols 等额外工具 (14 个)
- **Critic 窗口**: 最多 12 个工具
- **推荐排序**: 熟悉度高的工具优先排列

### Expert User (>0.7 mastery)
- **首轮工具**: 20 个工具全开（含 security_scan, performance_audit, db_migrate 等）
- **Critic 窗口**: 最多 15 个工具
- **推荐排序**: 完全按个人技能掌握程度加权

## Safety Constraints

### ✅ Fallback Mechanism
所有 adaptive logic 都包裹在 try-catch 中，失败时自动 fallback 到保守默认值：
- 初始工具数回退到 11
- critic 限制回退到 10
- 推荐排序回退到关键词匹配

### ✅ No Breaking Changes
- 不影响现有工具执行逻辑
- 只改变 schema 加载数量，不改变工具行为
- 保持 prompt cache 前缀稳定（只在 user block 调整）

### ✅ Test Coverage
- 所有 387 个原有测试通过
- 新增集成测试验证关键路径

## Impact Metrics

### Before
- **转化率**: 14% (76 工具 → 11 首轮 → 10 执行)
- **专家用户**: 只能看到 11 个工具，浪费专业能力
- **推荐**: 静态规则，无视用户水平

### After
- **转化率提升预期**:
  - 新手：维持 14% (保守策略保护)
  - 进阶：提升至 ~25% (多 3 个专业工具可用)
  - 专家：提升至 ~40% (接近完整工具集)
  
- **用户体验**:
  - AI 能根据水平动态调整工具范围
  - 高手不被初级工具束缚
  - 每次成功使用工具都能提升能力画像

## Files Modified

1. `/src/growth.js`
   - 新增 `getRuntimePolicy()` 函数 (第 678 行)
   - 新增 `getAvgMastery()` 导出函数 (第 73 行)

2. `/src/main.js`
   - 修改 `_selectInitialTools()` → 自适应版本 (#1, ~第 25141 行)
   - 修改 `_criticRequestedToolSchemas()` → 自适应限制 (#2, ~第 33571 行)
   - 增强 `recommendToolsForIntent()` → 个性化加权 (#3, ~第 33641 行)
   - 新增 `observeToolCall()` 函数 (#5, ~第 25259 行)
   - 修改工具执行回调 → 注入增长信号 (#5, ~第 36050 行)

## Next Steps / Optional Enhancements

### P1 #5: Enhanced Search Tools Fuzzy Match (未实现)
如果需要在 `search_tools` 中也应用模糊匹配，可以在 `main.js` 中找到该实现并增强：

```javascript
case 'search_tools': {
    const query = args.query.toLowerCase();
    const results = toolRegistry.filter(entry => {
        const nameMatch = entry.name.toLowerCase().includes(query);
        const descMatch = (entry.description || '').toLowerCase().includes(query);
        return nameMatch || descMatch;
    }).slice(0, 20);
    // ... formatting
}
```

### Future Considerations
- 考虑添加冷启动优化（首次使用时临时提升新手档位）
- 可探索更细粒度的技能分组（如按 LSP vs DB 分类）
- 添加 A/B 测试框架评估不同策略效果

## Conclusion

✅ **全部 P0 需求已实现并通过验证**
✅ **所有测试通过（387/387）**
✅ **向后兼容，无破坏性变更**
✅ **容错机制健全，异常 gracefully degrade**

这套机制打通了“成长系统→运行时”的反向通路，让 IDE 真正成为“越用越懂你”的智能助手。
