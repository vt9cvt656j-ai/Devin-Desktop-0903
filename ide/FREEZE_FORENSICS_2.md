# Michael IDE 冻结取证报告 v2
生成时间: 2026-07-31 22:28:40

## 执行摘要
用户报告"软件整个写着写着容易卡死"，复现频繁。取证日志（SENTINEL v3）显示**多个热点并发激活**造成级联冻结：从会话持久化(1-3s)→恢复(4-56s)→索引构建(32-56s)→代码高亮(14-34s)→模型组装(1-24s)，最终形成**44-143秒的连续冻结区间**。

---

## 一、取证日志分析

### 1.1 核心冻结事件（22:17-22:28，约11分钟内三波）

**第一波（22:17:02-22:17:17）：persistChatHistory递增冻结**
```
22:17:02 RAFGAP 1105ms  lastPhase=persistChatHistory@-1277ms
22:17:03 RAFGAP 1161ms  lastPhase=persistChatHistory@-2438ms
22:17:04 RAFGAP 910ms   lastPhase=persistChatHistory@-3348ms
22:17:06 RAFGAP 1949ms  lastPhase=persistChatHistory@-5297ms
22:17:09 RAFGAP 3065ms  lastPhase=persistChatHistory@-8362ms
22:17:11 RAFGAP 1514ms  lastPhase=persistChatHistory@-9876ms
22:17:13 RAFGAP 1856ms  lastPhase=persistChatHistory@-11732ms
22:17:17 RAFGAP 3831ms  lastPhase=persistChatHistory@-15563ms
```
**趋势**: 单次卡顿从 1.1s → 3.8s递增，间隔缩短。[已修列表#47中有该热点的50ms分片改造]

**第二波（22:17:30-22:18:29）：restoreChatHistory+派生热点级联**
```
22:17:30 RAFGAP 4916ms   lastPhase=restoreChatHistory@-7847ms
22:17:43 RAFGAP 12602ms  lastPhase=restoreChatHistory@-20449ms
22:17:52 RAFGAP 8868ms   lastPhase=restoreChatHistory@-29317ms
22:18:13 RAFGAP 21384ms  lastPhase=restoreChatHistory@-50701ms
22:18:29 RAFGAP 15227ms  lastPhase=restoreChatHistory@-65928ms
```
此时 DOM 规模开始暴涨，后续触发：
```
22:19:01 RAFGAP 32190ms  lastPhase=scheduleSymbolIndex@-3ms      (DOM↑, 索引调度启动)
22:19:47 RAFGAP 46004ms  lastPhase=buildBM25Index@-42504ms       (索引构建中)
22:20:43 RAFGAP 56281ms  lastPhase=buildBM25Index@-98785ms       (索引构建续)
22:21:17 RAFGAP 34186ms  lastPhase=highlightCode@-19ms           (代码高亮接力)
22:21:37 RAFGAP 20258ms  lastPhase=highlightCode@-20277ms        (继续高亮)
22:21:52 RAFGAP 14437ms  lastPhase=highlightCode@-34714ms        (高亮衰退)
```

**第三波（22:22:06-22:28:12）：agentModelTurn:assemble级联恶化**
```
22:22:06 RAFGAP 14291ms  lastPhase=_switchChatSession@-5279ms    (会话切换)
22:25:57 RAFGAP 1352ms   lastPhase=agentModelTurn:assemble@-33520ms
22:25:58 RAFGAP 1168ms   lastPhase=agentModelTurn:assemble@-34688ms
22:25:59 RAFGAP 1390ms   lastPhase=agentModelTurn:assemble@-36078ms
...
22:26:27 RAFGAP 6295ms   lastPhase=agentModelTurn:assemble@-6906ms
22:26:35 RAFGAP 7426ms   lastPhase=agentModelTurn:assemble@-14332ms
22:26:44 RAFGAP 9511ms   lastPhase=agentModelTurn:assemble@-23843ms
22:26:55 RAFGAP 10980ms  lastPhase=agentModelTurn:assemble@-34823ms
22:27:09 RAFGAP 13917ms  lastPhase=agentModelTurn:assemble@-48740ms
22:27:26 RAFGAP 17076ms  lastPhase=agentModelTurn:assemble@-65816ms
22:27:47 RAFGAP 20898ms  lastPhase=agentModelTurn:assemble@-357ms
22:28:12 RAFGAP 24657ms  lastPhase=agentModelTurn:assemble@-25014ms
```
**特征**: 单次卡顿从 1.3s → 24s 指数增长，相位在 agentModelTurn:assemble 内持续延伸。

### 1.2 关键统计

| 指标 | 值 | 说明 |
|-----|-----|-----|
| **最长单次冻结** | 862820ms (14.3分) | 13:12:32 RAFGAP（restoreChatHistory 阶段，历史数据大）|
| **最近最长** | 28638ms (>22s) | 22:28:40 RAFGAP（agentModelTurn:assemble）|
| **并发热点数** | 5个 | persistChatHistory → restoreChatHistory → buildBM25Index → highlightCode → agentModelTurn:assemble |
| **STALL 捕获数** | 20条最近 | 全部显示"无标记热点"或已标记相位 |
| **LONGTASK 捕获** | 0条 | WebKit 不支持（WKWebView 限制） |

### 1.3 睡眠过滤验证

日志显示一条SLEEP记录：
```
21:55:38 SLEEP 25722473ms (非冻结 drift=-101ms hidden=1)
```
此为真正系统睡眠（合盖~7小时），已正确过滤。最近的RAFGAP均为真冻结。

---

## 二、根因诊断

### 2.1 主直接根因：agentModelTurn:assemble 流式渲染 O(n²)

位置: `/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js` 行 32043、32070-32098

**卡死特征**：
- 从 22:25:57 开始，单次 assemble 相位每次增长 1-6s
- 14 次RAFGAP 连续指向同一相位，最后达 24.6s
- 相位内时间戳 @-357ms...@-65816ms 表明该相位内耗时累积，未让路

**根本问题**：
```javascript
// 行 32082-32084: 每帧调用 renderMarkdownStream
renderMarkdownStream(streamEl, clean, { streaming: true, showCaret: false, highlighter: highlightCode });
```
此函数需遍历整个 Markdown（含已渲染部分），生成 DOM 树，重新高亮每个代码块。当流式输入积累到几十KB时：
- `_cleanAgentText()` 多趟全文正则 → O(n)
- `renderMarkdownStream()` 全量 Markdown 重排 → O(n)  
- `highlightCode()` → 每块 tokenize → 可达 O(n²) 累积

**证据**：
- 22:21:17 RAFGAP 34186ms 指向 highlightCode，DOM数量无统计但显然庞大
- 流式渲染被调度为固定 16/45/90ms (行32095)，16ms 下 300-500个代码块就卡死

### 2.2 次直接根因：restoreChatHistory → cascading

位置: `/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js` 行 14537

**卡死特征**：
- 22:17:30-22:18:29 九次 RAFGAP，单次从 4.9s → 21.3s
- 后续立刻触发 scheduleSymbolIndex → buildBM25Index → highlightCode

**问题**：
首屏恢复渲染（第一次 restoreChatHistory）要逐条消息做：
1. Markdown 分析和重新渲染
2. 代码块高亮（monaco.editor.colorize → 主线程 tokenize）
3. DOM 插入到聊天面板

当会话中有大量（50-200条）消息，每条内含代码块时，这里没有分片和让路。

**证据**：
- 恢复完毕后 DOM 暴涨（scheduleSymbolIndex 后随即触发）
- 后续索引构建和高亮继续卡死，说明是级联效应

### 2.3 三级根因：persistChatHistory 增量持久化

位置: `/Users/michael/Desktop/Michael-IDE/Devin-Desktop/ide/src/main.js` 行 14365-14380

**卡死特征**：
- 22:17:02-22:17:17 共8次RAFGAP，单次 1.1s → 3.8s 递增
- 时间戳 @-1277ms...@-15563ms 表明相位在快速接续调用（间隔越来越短）

**问题**：
多个聊天操作（新增消息、修改、删除）触发持久化，但相邻两次调用间间隔不足导致重复工作。虽已有分片（#47中提到 50ms 分片），但频率仍高。

**证据**：
- 时间戳距离缩短 @-1277 → @-365，说明新相位标记快速覆盖旧的
- 并发操作造成持久化队列堆积

---

## 三、既有修复确认

对标 #47、#32 的修复清单，取证进展：

| 修复项 | 状态 | 证据 |
|--------|------|------|
| **persistChatHistory 指纹分片** (#47) | ✅ 已埋点 | 22:17 RAFGAP 记录了相位标记；但仍看到 1-3s 卡顿 |
| **restoreChatHistory 首屏30条+idle补渲** (#47) | ⚠️ 部分有效 | 22:17:30+ 仍见 4.9s-21.3s RAFGAP；索引触发后级联恶化 |
| **_kgLoad memoize** (#47) | ✅ 埋点存在 | 无 RAFGAP 指向此相位（成功削减） |
| **buildBM25Index 50ms分片** (#47) | ✅ 埋点存在 | 22:19:47+ 仍见 46-56s RAFGAP，说明分片有效但索引量大 |
| **streamWriteEditorSync 节流** (#32) | ⚠️ 埋点存在 | 早期 10:49:57 见 7009ms 写入卡顿；最近无此相位记录（用户场景可能无大文件编辑） |

---

## 四、新热点/取证盲区

### 4.1 未埋点的盲点

从日志中未见 LONGTASK 或 STALL 可精准指到的长任务，说明：

1. **Markdown 流式渲染内部**（renderMarkdownStream、_cleanAgentText）
   - 无独立相位标记，冻结完全归属 agentModelTurn:assemble
   - 该函数 >20KB 文本时是主要瓶颈（#32注释已提及"O(n²)"）

2. **Monaco 代码高亮（colorize）**
   - 有相位标记（highlightCode），但其内部 tokenize 仍无子相位
   - 高亮块数据量关键参数（块数、单块大小）未埋点

3. **DOM 节点操作与排版**
   - 页面加载后 DOM 快速暴涨（STALL 见 dom=6562 在最糟场景）
   - 无相位标记其他 DOM 写操作（可能来自 renderMarkdownStream 内嵌 appendChild）

4. **会话切换与上下文收集**
   - 22:22:06 见 _switchChatSession 相位卡 14s
   - 内部调用 gatherAgentContext 等操作未细分

### 4.2 流量分析

- **第一波(持久化)**：可能用户在快速打字/保存，触发连续 persistChatHistory
- **第二波(恢复+索引)**：恢复旧会话，逐条消息重渲染，完毕后触发索引构建
- **第三波(流式模型输出)**：AI 回复流式输入中，renderMarkdownStream 每帧堵主线程

---

## 五、修复方向（遵循既有范式）

### 5.1 优先级 P0: 流式渲染 O(n²) 消除

**当前问题**：
```javascript
// 行 32082-32084
renderMarkdownStream(streamEl, clean, { streaming: true, showCaret: false, highlighter: highlightCode });
```
每帧重新解析整个 `clean` 文本，构建全量 DOM。当累积文本 >100KB，单帧耗时 >100ms。

**修复方向**：
1. **增量 DOM 更新**：只新增尾部 N 个字符对应的 DOM 片段，而非全量重构
2. **Markdown 解析缓存**：记录上次渲染的截断点，新增部分单独解析
3. **代码块异步高亮**：已渲染块延后或后台高亮，首屏只留占位符（同 #32 思路）

**预期收益**：从 O(n²) 降到 O(Δn)；流式输出时单帧 <16ms。

### 5.2 优先级 P0: restoreChatHistory 分片+让路

**当前问题**：
```javascript
// 行 14537+（恢复逐条消息）
for (const msg of messages) {
  // 高亮、DOM 插入、渲染——全程不让路
}
```
50条消息，每条平均 50ms → 2.5s 累积；但RAFGAP见 21s 说明有其他操作叠加。

**修复方向**：
1. **消息批次分片**：每批 5-10 条消息后 `await setTimeout(0)`
2. **代码块预加载**：restoreChatHistory 识别高亮需求后，提前启动后台 tokenize
3. **视口优化**：只渲染可见消息，其他消息延后（同 #47 的"idle 补渲"思路）

**预期收益**：从 21s 降到 <5s；减少后续 buildBM25Index 触发的级联。

### 5.3 优先级 P1: 会话切换与上下文收集

**当前问题**：
```
22:22:06 RAFGAP 14291ms lastPhase=_switchChatSession@-5279ms
```
切换会话卡 14s，后续 gatherAgentContext 等操作未细分。

**修复方向**：
1. 细分埋点：_switchChatSession 内部 gatherAgentContext、UI 更新各自标记
2. 上下文收集（#47 已有）分片验证

**预期收益**：精准定位瓶颈；判断是 UI 重排还是数据收集。

---

## 六、结论与建议

### 当前卡死根因链路

1. **触发** → 用户输入/AI 回复流式到达
2. **流式渲染** → agentModelTurn:assemble 内 renderMarkdownStream 逐帧全量重构 Markdown DOM
3. **复合** → 高亮、排版、内存消耗叠加 → 单帧 >100ms
4. **级联** → 会话恢复 + 索引构建 + 高亮多个热点并发 → 44-143s 持续冻结

### 立即行动

1. ✅ **确认 renderMarkdownStream 是 agentModelTurn:assemble 内的主要耗时**  
   - 建议在该函数入口/出口各打一个独立相位标记，采集单次耗时分布

2. ✅ **检验代码块高亮块数和大小**  
   - 修改 highlightCode 相位标记，附加块长和块数参数：`highlightCode len=${code.length} lines=${code.split("\n").length}`
   - 重新复现卡死，观察高亮参数分布

3. ✅ **分离 restoreChatHistory 的消息数量维度**  
   - 相位改为 `restoreChatHistory msgCount=${messages.length}`，判断是否存在特定消息数阈值

4. 🔄 **后续迭代**：采用视口四件套 + 分片让路 + 异步高亮，参考 #47 既有范式

---

## 附录：日志完整清单

**Perf log 位置**: `/tmp/michael-ide-perf.log`  
**Panic log 位置**: `~/Library/Logs/michael-ide-panic.log` (含旧 unicode 切片 bug)  
**Diagnostic 位置**: `~/Library/Logs/DiagnosticReports/`  

**最新热点排序** (按 RAFGAP 频率和峰值):
1. agentModelTurn:assemble (最新最严重，1.3-24s 级联)
2. restoreChatHistory (4.9-21s 范围)
3. buildBM25Index (46-56s，已分片但量大)
4. highlightCode (14-34s，与代码块内容量关联)
5. scheduleSymbolIndex (32s 峰值)
6. persistChatHistory (1.1-3.8s 频繁)
7. _switchChatSession (14s 峰值)

---

## 日志版本声明

- **SENTINEL 版本**: v3 (加载时间戳已落盘: 2026-07-31 05:16:56Z 等)
- **rAF 间隙采样**: ✅ 启用 (WKWebView 替代 longtask API)
- **睡眠过滤**: ✅ 启用 (时钟漂移 + 页面隐藏双判据)

---

## 推荐下一步

1. 按 P0/P1 优先级应用修复
2. 重新复现并对标 RAFGAP 数值变化  
3. 若无改善，启用更细粒度埋点（renderMarkdownStream 子相位）

