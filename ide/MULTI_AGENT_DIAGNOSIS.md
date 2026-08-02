# Michael IDE 多智能体编排机制诊断与改进方案

## 调研概述

本报告针对用户反馈的"子智能体效果差、只能单派、主智能体阻塞等待"问题，对 Michael IDE 的子智能体/多角色编排机制进行系统性代码级调研。所有结论均基于 `src/main.js`（约 55700 行）的实际实现。

---

## 一、五个核心问题的机制事实

### 1.1 子智能体现状：run_subagent/_runSubAgent

**定义与实现位置：**
- **函数签名** (line 32882): `async function _runSubAgent({ config, description, prompt, root, container, run, write = false, scope = [], role = "", depth = 1, onMutation = null })`
- **调用路径**: line 36636 - 在主循环的工具段执行中被异步调用
- **本质**: **独立模型循环**,不是简单工具调用。它拥有自己的 tool schema(读/写两套)、自己的消息历史、最多 SUB_MAX 步(write=true 时 18 步，只读时 12 步) 的自主推理链。

**如何触发:**
- 模型通过 `run_subagent` 工具调用触发，tool schema 定义于 line 25348:
```javascript
subagent: { tools: ["run_subagent", "run_worker", "research_project", "generate_wiki"] }
```
- 工具调用映射：line 26337 `case "run_subagent": return { type: "subagent", ... }`
- UI 卡片渲染：line 32890 `card.className = "agent-tool-step agent-tool-step--subagent is-open"`

**同步 await 阻塞还是异步？**
- **当前是同步阻塞**。关键证据在 line 36636-36640:
```javascript
const report = await _runSubAgent({ 
  config, 
  description: it.call.description, 
  prompt: it.call.prompt, 
  root, 
  container: body, 
  run, 
  write: isWorker, 
  scope: it.call.scope || [], 
  role: it.call.role || "", 
  onMutation: (path) => { ... }
});
```
- 主循环中 `_runOrderedToolSegments` (line 31612) 处理并发段时，对于每个 item 调用 `execute(items[current], current)` (line 31627),而 execute 函数内部就是 await runSubagentItem(it) (line 36623)。
- **结论**: 即使不同 segment 间可以并行(Promise.all),但一个 run_subagent 本身在执行时**主循环卡死在 await 上不能继续其他工作**。

**能否并发多个？**
- **同一轮内，同类型子智能体串行，不同类型可以部分并行**。
- 并发控制逻辑在 `_runOrderedToolSegments` (line 31612-31632):
  ```javascript
  async function _runOrderedToolSegments(items, segmentKeyOf, execute, isLive = () => true) {
    for (let index = 0; index < items.length && isLive();) {
      const key = segmentKeyOf(items[index], index);
      if (!key) {
        await execute(items[index], index); // 硬屏障，串行
        index++;
        continue;
      }
      let end = index;
      const segment = [];
      while (end < items.length && segmentKeyOf(items[end], end) === key) {
        const current = end++;
        segment.push(execute(items[current], current)); // 收集到 Promise 数组
      }
      await Promise.all(segment); // 真正并行
      index = end;
    }
  }
  ```
- 段键语义由 `segmentKeyOf` 回调定义 (line 36700-36712):
  ```javascript
  (it, index) => {
    if (toolMsgs[index] || !it.call) return "";
    if (canRunInReadSegment(it)) {
      return it.call.type === "list" ? "recon" : "read";
    }
    if (it.call.type === "worker") return "worker";
    return "";
  }
  ```
- **关键点**: 
  - `run_subagent` 被 `canRunInReadSegment` 判断为可并行 (line 36667):
    ```javascript
    const canRunInReadSegment = (it) => !!it.call && (_isReadOnlyParallel(it.call)
      || ["run_subagent", "research_project", "design_research"].includes(it.tc.name));
    ```
  - 这意味着**同轮内的多个 run_subagent 可以被 Promise.all 并发执行**！前提是它们是连续的且都是只读 subagent。
  
**结果如何回传？**
- `_runSubAgent` 返回 report string (line 33113): `return report || (write ? "（worker 未产出简报）" : "（子智能体未产出简报）");`
- 这个 report 被包装成 tool result message (line 36657):
  ```javascript
  it.rawResult = { 
    type: isWorker ? "worker" : "subagent", 
    path: it._wikiPath || it.call.description || "", 
    content: message 
  };
  ```
- 最终注入 messages 数组喂给下一轮 LLM (line 36741): `for (const m of toolMsgs) messages.push(m);`
- **重要上下文协议**: line 33109-33112 将 report 压缩 400 字塞进 `run.ctx.findings`:
  ```javascript
  if (run && run.ctx && Array.isArray(run.ctx.findings) && report && report.trim()) {
    run.ctx.findings.push(`【${write ? "worker" : "子"}:${String(description || "").slice(0, 24)}】${report.replace(/\s+/g, " ").slice(0, 400)}`);
    if (run.ctx.findings.length > 40) run.ctx.findings.splice(0, run.ctx.findings.length - 40);
  }
  ```

**有无超时/中断保护？**
- **有 live 检查机制** (line 32884-32885):
  ```javascript
  const _sess = run && run.session;
  const _subGenSnap = _sess ? (_sess._runGen || 0) : 0; // 代际快照：父 run 被新回合取代后，子代理下个检查点确定性退出
  const _live = () => !_sess || (_sess.streaming && (_sess._runGen || 0) === _subGenSnap);
  ```
- 子智能体的每步循环都有 `_live()` 检查 (line 33006: `if (!_live()) break;`),如果主 run 被中断或切换代际，子智能体会安全退出。
- 超时控制：没有显式的 timeout(),但依靠 SUB_MAX 步数限制 (最多 12/18 步)。

**上下文怎么传？**
- **上下文组装** (line 32941-32968):
  ```javascript
  const _childContext = _currentDateBlock() + `\n\n--- 项目上下文 ---\n` + (await _gatherAgentContext("", root));
  const _shared = _sharedCtxDigest(run && run.ctx);  // 【关键】主智能体已掌握的上下文摘要
  const _evidenceLedger = _sessionFileEvidenceBlock(_sess, root, 12);  // 会话文件证据账本
  let _designHandoff = "";
  if (["frontend", "design"].includes(String(role || "").toLowerCase())) {
    // frontend/design 角色会拿到压缩后的 michael-design 设计证据交接块
    const ev = run && run._michaelDesignEvidence;
    if (ev) { /* 压缩设计证据 */ }
  }
  const _handoff = [_shared, _evidenceLedger, _designHandoff].filter(Boolean).join("\n\n");
  const messages = [{ role: "system", content: sysPrompt }, 
                    { role: "user", content: (_handoff ? _handoff + "\n\n——————\n\n" : "") + _childContext + "\n\n——————\n\n" + prompt }];
  ```
- **_sharedCtxDigest 具体字段** (line 32858-32868):
  ```javascript
  function _sharedCtxDigest(ctx) {
    const p = [];
    if (ctx.goal) p.push(`· 总目标：${ctx.goal}`);
    if (ctx.requirements && ctx.requirements.length) p.push(`· 原始需求：...`);
    if (ctx.done && ctx.done.length) p.push(`· 主智能体已完成：${ctx.done.slice(-8).join(" → ")}`);
    if (ctx.modified && ctx.modified.size) p.push(`· 已改的文件：...`);
    if (ctx.filesRead && ctx.filesRead.size) p.push(`· 已读过（不必重读，除非你要改它）：${[...ctx.filesRead].slice(-40).join("、")}`);
    if (ctx.findings && ctx.findings.length) p.push(`· 已知关键发现：${ctx.findings.slice(-8).join("；")}`);
    if (ctx.errors && ctx.errors.length) p.push(`· 当前未解决的错误：${ctx.errors.slice(-3).join("；")}`);
    return p.length ? `【主智能体已经掌握的上下文——直接接着用，别从零重查】\n${p.join("\n")}` : "";
  }
  ```
- **问题分析**: 
  - `ctx.done` 只取最近 8 步
  - `ctx.filesRead` 只取最后 40 个文件路径 (**只有路径！没有内容摘要!**)
  - `ctx.findings` 只保留最近 8 条发现，每条 400 字
  - **这确实是"上下文喂得太少"的证据!**

---

### 1.2 run_worker / debate / worktree 现状

**run_worker:**
- **真实实现**: 通过 `_runSubAgent(write=true, scope=[...])` 复用同一函数，line 32886-32888:
  ```javascript
  // Worker mode: declared scope → root-relative; the worker may only modify files
  // inside it. Disjoint scopes across concurrent workers = no write conflict.
  const scopeRel = write ? (Array.isArray(scope) ? scope : [scope]).map((s) => _normRel(s, root)).filter(Boolean) : [];
  ```
- **能力**: 可以写文件，但必须声明 scope，且并行 worker 之间 scope 不能重叠 (line 32930-32935 有 scope overlap 检查)。
- **并发能力**: 与 run_subagent 一样，可以被 `_runOrderedToolSegments` 识别为 "worker" 段并 Promise.all 并发。
- **工具集**: line 32986 给予 write 模式额外权限：
  ```javascript
  const _allow = write
    ? [..._READ_TOOLS, "write_file", "edit_file", "multi_edit", "run_cmd", "format_file", "create_dir"]
    : _READ_TOOLS;
  ```
- **实际使用**: user memory 中提到"相邻 run_worker 组成并行段真并行",证明已实现。

**debate:**
- **工具定义存在**: line 25134 定义了 debate 工具的 schema:
  ```javascript
  { type: "function", function: { name: "debate", description: "**辩论模式——重大技术决策/方案取舍时用**...", parameters: {...} } }
  ```
- **工具映射存在**: line 26335 `case "debate": return { type: "debate", question: ..., perspectives: ..., context: ... };`
- **显示逻辑存在**: line 34267 `case "debate": return "辩论：" + String(call.question || "").slice(0, 40);`
- **但无执行实现**: 在 `_executeToolStep` (line 39291) 中**完全没有** `call.type === "debate"` 的处理分支。
- **结论**: **debate 是 UI 装饰/未来规划，未被实现**。模型调用 debate 时会卡在 `_executeToolStep`,因为没有匹配 case 会抛出异常或被当作未知工具拒绝。

**worktree:**
- **Rust 后端命令存在**: line 316-318 定义了 `gitWorktreeAdd/list/remove` 三个 Rust IPC 命令。
- **工具定义存在**: line 25134 定义了 `worktree` 工具 (best-of-N 隔离)。
- **工具指南示例**: tool-guides.js line 331 `worktree: { action: "list" };`
- **执行层缺失**: 同样在 `_executeToolStep` 中找不到 `call.type === "worktree"` 的处理逻辑。
- **结论**: **worktree 也是 UI 装饰/Rust 命令未暴露为可执行工具**。

---

### 1.3 多角色机制：触发条件/并行方式/结果合并

**角色定义与纪律块:**
- `_AGENT_ROLE_BLOCKS` 定义于 line 32768-32817，包含:
  - architect, product, frontend, backend, database, security, test, devops, design, docs, research 共 11 个角色
  - 每个角色有专注领域和纪律约束 (如 frontend 强制 Tailwind 配色、禁裸 hex)
- 角色注入：line 32818-32820:
  ```javascript
  function _agentRoleBlock(role) {
    const key = String(role || "").trim().toLowerCase();
    return (_AGENT_ROLE_BLOCKS[key] ? "\n\n" + _AGENT_ROLE_BLOCKS[key] : "");
  }
  ```
- 在 `_runSubAgent` 中，sysPrompt 拼接角色块 (line 32938-32940):
  ```javascript
  const sysPrompt = (write ? _WORKER_SYSTEM : _SUBAGENT_SYSTEM) 
                    + _agentRoleBlock(role) 
                    + (run?.skillsBlock ?? _activeSkillsBlock());
  ```

**触发条件:**
- **由模型自主决定**: 没有自动检测引擎切分任务的机制。用户记忆提到"裁决提示词仅有克制条款，缺乏正向判据",说明当前没有自动 multi-role 触发器。
- **手动按名调用**: user memory 明确指出"solo 模式下需提供清晰升级路径：任务展开后发现需分角色，可直接按名调用 run_subagent（只读调研）或 run_worker（分 scope 写入）"。
- **编排者五步纪律** (memory 引用): 
  1. 先定架构与接口契约再拆
  2. worker prompt 自包含（目标/文件/契约/验证命令）
  3. scope 按领域切干净不重叠
  4. 返回后抽查关键产出 + 跑构建测试，挂了带报错重派该角色
  5. 如实汇总各角色完成度

**是否真并行？**
- **是**,通过 `_runOrderedToolSegments` 的 Promise.all (line 31629)。
- **前提**: 
  1. 多个 worker 必须是**连续**的 item
  2. 它们的 scope 必须**互不重叠** (line 32931-32934 有校验)
  3. 父 run 不能是 read-only 模式 (line 32922-32924 拦截)
- **历史问题**: 注释提到"此前 worker 被当硬屏障，同轮派 3 个 worker 实际串行跑，白白三倍耗时",证明已修复为真并行。

**每个 worker 的模型/工具/上下文配置:**
- **模型**: 继承父 run 的 config.model，没有单独指定 (line 33007: `await _agentModelTurn({ config, ... })`)。
- **工具**: 
  - read-only subagent: line 32983-32984 的 `_READ_TOOLS` (18 个只读工具)
  - worker: line 32985-32987 扩展为可写工具 + run_cmd
  - **递归禁止**: line 33020 注释"Neither can spawn a sub-agent/worker (not on the list) → recursion is structurally impossible"
- **上下文**: 见 1.1 节，包含 shared context digest + evidence ledger + design handoff。

**结果合并逻辑:**
- **run.ctx.findings 累积**: 每个子智能体/worker 完成后，400 字简报被追加到 `run.ctx.findings` (line 33109-33112)。
- **文件修改追踪**: worker 写的文件会被记录到 `run.ctx.modified` (line 33059-33064)。
- **主循环看到的事实**: 下一轮 LLM 调用时，messages 中包含了所有 tool messages (line 36741),run.ctx.findings 也被_contextBudgetScale 压缩后注入到后续 turn (line 32944)。
- **问题**: findings 仅保留 40 条最新，且每条 400 字截断，长任务容易丢失早期 worker 的详细结果。

---

### 1.4 主智能体阻塞点分析

**阻塞机制:**
- line 36636: `const report = await _runSubAgent({...})` —— 这是**真正的 await 阻塞点**。
- 在 `_runOrderedToolSegments` 中 (line 31629),虽然 Promise.all 让同段 items 并行，但每个 execute() 调用内部仍然是顺序 await。
- **这意味着**: 即使命令并行启动多个 subagent，主循环也在 `await Promise.all([subagent1, subagent2, ...])` 处卡住，无法执行其他逻辑。

**有没有异步消息/检查点机制？**
- **没有**。代码中没有发现类似 "callback/promise(resolve)/emit event"的机制让主智能体"先干别的，子智能体完成后再 merge"。
- live 检查 (line 32885) 只是为了在子智能体运行时允许主智能体**中断**它，而不是协作。
- **对比理想状态**: 应该是子智能体启动后立即返回一个 job ID，主智能体继续执行其他 tool calls，同时后台监控 job 进度，完成后再 pull 结果。

**为什么不能协同？**
- 根本原因是当前架构将 subagent 视为"同步工具调用"而非"异步作业"。
- 从调用栈看：主循环→executeScheduledItem→runSubagentItem→_runSubAgent→_agentModelTurn→_runLoop，这是一个完整的同步链，中间没有 yield 点让主循环插队。

---

### 1.5 并发的真实约束

**网关/模型通道并发限制:**
- 代码中**未发现显式的并发请求限流**。
- line 20836-20879 定义了命令并发的限制:
  ```javascript
  const _MAX_CONCURRENT_CMDS = 3;
  if (_runningTermCmds >= _MAX_CONCURRENT_CMDS) {
    return { code: 1, stdout: "", stderr: `Too many concurrent commands (${_runningTermCmds}/${_MAX_CONCURRENT_CMDS}). Wait for others to finish.` };
  }
  ```
  但这只是 shell 命令并发，不是 LLM 请求。
- **外部依赖**: Michael 网关可能有服务端限流，但客户端代码中没有暴露相关配置。

**成本记账 (_billingScope):**
- `_billingScopeId` 定义于 line 19927-19930，用于跟踪一轮 run 的所有 token 消耗。
- line 20305: `await _awaitBillableAiTasks(_billingScopeId);` 等账单结算逻辑存在于 run 结束时。
- **问题**: 没有看到 per-turn 或 per-subagent 的成本钳位。如果并发 10 个子智能体，可能同时烧穿大量 tokens，导致用户投诉"扣费太快"。

**多 worker 文件冲突防护:**
- **有 worktree 隔离**: line 32930-32935 的 `run._activeWorkerScopes` 机制:
  ```javascript
  run._activeWorkerScopes = run._activeWorkerScopes || [];
  if (run._activeWorkerScopes.some((s) => _scopesOverlap(s, scopeRel))) {
    res.className = "atc-result atc-result--err"; 
    res.textContent = "scope 重叠";
    return `[ERROR] 这个 worker 的 scope（${scopeRel.join(", ")}）与另一个正在运行的 worker 重叠。`;
  }
  run._activeWorkerScopes.push(scopeRel);
  ```
- **释放时机**: line 33075-33078 在 finally 块中移除 scope:
  ```javascript
  if (write && run._activeWorkerScopes) {
    const idx = run._activeWorkerScopes.indexOf(scopeRel);
    if (idx >= 0) run._activeWorkerScopes.splice(idx, 1);
  }
  ```
- **效果**: 保证并行 worker 的 scope 严格不相交，物理上避免文件冲突。

---

## 二、子智能体"效果差"的具体根因

### 2.1 上下文不足（主要因素）

**证据链:**
1. `_sharedCtxDigest` 中 `ctx.filesRead` 只存**文件路径**,不包含文件内容或摘要 (line 32865)
2. `ctx.findings` 只保留最近 8 条，每条截断至 400 字 (line 32866, 33110)
3. `ctx.done` 只取最后 8 步 (line 32863)
4. 整个_childContext 基于_gatherAgentContext(""),该函数会重新 scan 项目结构，但如果项目大，可能因为 token 预算被 truncation。

**影响**: 
- 子智能体开工时面对的是"主智能体读过的文件名列表"而非"这些文件的关键信息摘要"。
- 例如：主智能体读了 50 个文件，子智能体只知道文件名，不知道哪些是核心逻辑、哪些是配置文件，仍需自行 read_file 深入调查 → **重复劳动、效率低下、token 浪费**。

### 2.2 模型降级（潜在因素）

**证据链:**
- 子智能体**继承父 run 的 model 配置** (line 33007),没有专门指定为更强大的模型。
- 但在某些场景下，主智能体可能是旗舰模型，而子智能体若走另一套路由可能会降级。
- **需要核实**: 实际部署中，subagent 是否调用了不同的模型通道？

### 2.3 工具受限（次要因素）

**证据链:**
- read-only subagent 的工具集 line 32983 是固定的 18 个只读工具，包括:
  - 基础读取：read_file, list_dir, search, find_files
  - 语义检索：semantic_search, find_symbol, lsp_symbols, lsp_definition, lsp_references
  - 诊断工具：get_diagnostics, read_logs
  - 知识库：knowledge_search
  - Web 检索：web_fetch, web_search, screenshot, road_environment, shop_catalog
- **缺失能力**: 
  - 不能运行命令 (run_cmd) —— 无法真实测试代码/服务
  - 不能创建文件 —— 纯调研性质，不能"边查边试"
  - 不能 nested subagent —— 不能分层深化调研（比如先宏观再微观）

**结论**: 
- **效果差的核心是上下文传递不充分**,子智能体被迫重复主智能体已做的工作，且缺乏验证能力（不能跑命令）。
- 工具集本身较丰富，但对于"可执行调研"来说，缺少 run_cmd 是个硬伤（比如要验证一个 API 是否正常，只能靠 web_fetch 打 HTTP 请求，不能 curl localhost:3000）。

---

## 三、分级改进方案

### P0: 子智能体能力增强（2-3 天工作量）

#### P0.1 增强上下文传递

**改动点:**
1. **扩展 sharedCtxDigest 增加文件内容摘要** (line 32858-32868):
   ```javascript
   function _sharedCtxDigest(ctx) {
     // ... existing fields ...
     // NEW: 增量文件摘要
     if (ctx.fileSnippets && ctx.fileSnippets.size) {
       const snippets = [...ctx.fileSnippets].slice(0, 10).map(([path, snippet]) => 
         `${path}: ${snippet.slice(0, 200)}`
       ).join("\n");
       p.push(`· 关键片段:\n${snippets}`);
     }
     // NEW: 已验证的发现（带证据）
     if (ctx.verifiedFindings && ctx.verifiedFindings.length) {
       p.push(`· 已验证事实:`);
       ctx.verifiedFindings.slice(-5).forEach((f, i) => {
         p.push(`  ${i+1}. ${f.conclusion} (证据：${f.evidence})`);
       });
     }
   }
   ```

2. **扩大 findings 保留窗口** (line 33111):
   ```javascript
   if (run.ctx.findings.length > 60) run.ctx.findings.splice(0, run.ctx.findings.length - 60); // 从 40->60
   ```

3. **提高简报长度上限** (line 33110):
   ```javascript
   run.ctx.findings.push(`【${...}】${report.replace(/\s+/g, " ").slice(0, 800)}`); // 从 400->800
   ```

**预估规模**: 
- 修改 1 个函数 (20 行)
- 修改 2 处常数字面量 (2 行)
- **总计**: ~30 行改动，低风险

**风险**: 
- Token 消耗增加~50%,但对用户体验提升明显。
- 可能需要调整_michaelUser.michael_compression.max_input_tokens 档位以防超限。

#### P0.2 给子智能体赋予 run_cmd 权限

**改动点:**
1. **修改只读 subagent 工具集** (line 32983-32987):
   ```javascript
   const _READ_TOOLS = ["read_file", "list_dir", "search", "find_files", "semantic_search", "find_symbol", "lsp_symbols", "lsp_definition", "lsp_references", "get_diagnostics", "read_logs", "knowledge_search", "web_fetch", "web_search", "screenshot", "road_environment", "shop_catalog"];
   // ADD run_cmd but mark as dry-run only?
   // Better: add a new field to track if cmd is safe
   const _allow = write
     ? [..._READ_TOOLS, "write_file", "edit_file", "multi_edit", "run_cmd", "format_file", "create_dir"]
     : [..._READ_TOOLS, "run_cmd"]; // Grant run_cmd to read-only subagents too!
   ```

2. **添加 run_cmd 的沙箱守卫** (line 33021-33038):
   ```javascript
   if (!call || !_execTypes.includes(call.type)) {
     // Special handling for run_cmd in read-only mode
     if (call.type === "cmd" && !write) {
       // Only allow safe read-only commands
       const SAFE_CMDS = /^(cat|grep|head|tail|wc|find|ls|git log|npm ls|pip show|cargo tree).*$/;
       if (!SAFE_CMDS.test(call.command)) {
         return { type: "cmd", content: `[BLOCKED] 只读子智能体只能运行安全的只读命令，你的命令 '${call.command}' 不安全。` };
       }
     } else {
       // Reject all mutating tools
       return { type: call.type, content: "[BLOCKED] ..." };
     }
   }
   ```

**预估规模**:
- 修改 1 处工具集定义 (1 行)
- 添加 1 个命令白名单检查 (~15 行)
- **总计**: ~20 行改动

**风险**: 
- 命令注入风险：必须严格白名单过滤。
- 建议初期默认关闭，通过配置开关_enableSubagentCmd: false 控制。

**替代方案**: 不让 subagent 直接跑命令，而是返回"建议验证命令清单",主智能体审核后执行，结果回传给 subagent 继续分析。

### P1: 并发多派 N 个子智能体（1-2 周工作量）

#### P1.1 可视化并行子智能体卡片

**现状问题**: 
- 用户看到"子智能体 设计+UI 架构调研"跑了 2m39s，**看不到内部进度**,以为是单个卡死的任务。
- 实际上可能有多个 subagent 在并行，但它们的效果混合在一个 card 里。

**改动点:**
1. **拆分子智能体卡片**: 当前 _runSubAgent 创建一个 card (line 32889-32897)。改为：
   - 父 card: 显示"子智能体集群：N 个并行任务"
   - 子卡片：每个 subagent 一个独立 card，带独立 progress bar
2. **实时进度反馈**: 在 subagent 循环中 (line 33004-33068),每执行一步就更新对应 card 的状态:
   ```javascript
   for (let i = 0; i < SUB_MAX; i++) {
     if (!_live()) break;
     // ... execute step ...
     // UPDATE PROGRESS CARD
     const progressEl = card.querySelector(".progress-bar");
     progressEl.textContent = `第 ${i+1}/${SUB_MAX} 步`;
   }
   ```

**预估规模**: 
- UI 组件重构：~200 行
- 状态管理：新增 SubAgentGroup 类管理集群 (~100 行)
- **总计**: ~300 行

**风险**: 
- DOM 操作复杂度高，可能引发性能问题（大量卡片）。
- 需要做好虚拟滚动或分页展示。

#### P1.2 智能合并策略

**问题**: 并行子智能体的结果如何 intelligently merge？

**方案:**
1. **阶段性归约**: 每个 subagent 每 N 步输出一次 intermediate result，主智能体做轻量 merge。
2. **最终聚合器**: 所有 subagent 完成后，调用一个专门的"merge agent"汇总所有 findings:
   ```javascript
   const mergedReport = await _runSubAgent({
     role: "orchestrator",
     prompt: `以下是 N 个子智能体的调研报告，请整合成一份统一文档：\n${reports.join("\n\n---\n\n")}`,
     // 只读模式
   });
   ```
3. **冲突检测**: 如果两个 worker 修改了同一模块，标记冲突让用户决策。

**预估规模**:
- Merge agent logic: ~100 行
- Conflict detector: ~80 行
- **总计**: ~200 行

**风险**:
- 合并过程可能丢失细节。
- 需要用户可干预 merge 结果（diff viewer）。

### P2: 主从协同：主智能体不阻塞（3-4 周工作量）

#### P2.1 异步作业架构

**核心改动**: 将 subagent 从"同步工具调用"改为"异步作业":

1. **启动即返回 JobID** (替换 line 36636):
   ```javascript
   // OLD: const report = await _runSubAgent(...);
   // NEW:
   const jobId = `subagent_${Date.now()}_${Math.random().toString(36).slice(2)}`;
   const jobPromise = _spawnSubagentAsync({ jobId, ...params });
   // 主循环继续！
   ```

2. **后台监控队列** (新模块):
   ```javascript
   class SubagentJobQueue {
     constructor() {
       this.activeJobs = new Map(); // jobId -> promise
       this.pollInterval = 1000; // ms
       this._startPolling();
     }
     
     async _startPolling() {
       setInterval(async () => {
         for (const [jobId, promise] of this.activeJobs) {
           const status = await this._checkJobStatus(jobId);
           if (status.completed) {
             const result = await promise;
             this._notifyMainAgent(jobId, result); // Event callback
             this.activeJobs.delete(jobId);
           }
         }
       }, this.pollInterval);
     }
     
     _notifyMainAgent(jobId, result) {
       // Push result to main agent's next turn via run.ctx.findings or dedicated channel
       run.ctx.pendingSubagentResults.set(jobId, result);
     }
   }
   ```

3. **主循环检查 pending results** (line 36741 前):
   ```javascript
   // Before pushing tool messages, check for completed subagents
   if (run.ctx.pendingSubagentResults.size) {
     for (const [jobId, result] of run.ctx.pendingSubagentResults) {
       const summary = result.slice(0, 800);
       messages.push({
         role: "system",
         content: `[子智能体 ${jobId} 已完成]\n${summary}`
       });
     }
     run.ctx.pendingSubagentResults.clear();
   }
   ```

**预估规模**:
- JobQueue 类：~150 行
- Async spawn API: ~100 行
- Main loop integration: ~80 行
- **总计**: ~350 行

**风险**:
- 复杂性陡增，调试困难。
- 竞态条件风险（主 agent 和 subagent 同时写 run.ctx）。
- 需要 Rust 后端支持异步取消（register_cancel 已有，可复用）。

#### P2.2 双向通信通道

**理想状态**: 主智能体和子智能体可以**互相发消息**,像微服务间的 message bus。

**方案:**
1. **SharedStore 模式**: 使用 SQLite 或内存 Map 作为共享存储:
   ```javascript
   // Main agent sends task
   await store.put(`subagent:${jobId}:task`, {
     goal: "调研 auth 模块",
     scope: ["src/auth"],
     deadline: Date.now() + 300000 // 5 min
   });
   
   // Subagent reads task
   const task = await store.get(`subagent:${jobId}:task`);
   
   // Subagent writes progress
   await store.append(`subagent:${jobId}:log`, {
     ts: Date.now(),
     msg: "读了 5 个文件，发现 JWT 验证逻辑在 src/auth/jwt.js:45"
   });
   
   // Main agent polls progress
   const logs = await store.getRange(`subagent:${jobId}:log`, { from: lastLogIdx });
   ```

2. **EventEmitter 机制**: Node-style events 通知状态变化:
   ```javascript
   const subagentEvents = new EventEmitter();
   subagentEvents.on("subagent:result", (data) => {
     // Callback when subagent completes
     console.log(`Subagent done:`, data.result);
   });
   
   // In _spawnSubagentAsync
   const result = await _runSubAgent(...);
   subagentEvents.emit("subagent:result", { jobId, result });
   ```

**预估规模**:
- Store 抽象层：~200 行
- EventBus: ~100 行
- Subagent 改造：~150 行
- **总计**: ~450 行

**风险**:
- 过度工程化，对小项目不必要。
- 需要严格的事务保证，避免数据不一致。

---

## 四、可复用既有基础

### 4.1 已验证的并行编排机制

**_runOrderedToolSegments** (line 31612-31632) 已经实现了:
- ✅ 段键分组逻辑
- ✅ Promise.all 并行执行
- ✅ Live 检查与中断
- ✅ 连续相同 key 的 items 自动成段

**可复用性**: 
- P1 的多 subagent 并发**直接复用此函数**,只需确保 segmentKeyOf 正确返回 "subagent" 键。
- 当前的问题是这个函数已经被 run_worker 占用 ("worker" key)，需要扩展支持 "subagent" key。

### 4.2 Scope 隔离守卫

**_activeWorkerScopes** (line 32930-32935) 提供了:
- ✅ 并行 scope 冲突检测
- ✅ 自动注册/注销
- ✅ 友好的错误提示

**可扩展**:
- Subagent 也可以用类似的机制跟踪"调研范围",防止重复调查同一模块。

### 4.3 Context Digest 架构

**_sharedCtxDigest** (line 32858-32868) 的字段设计已经很合理:
- ✅ Goal/requirements 顶层目标
- ✅ Done/Modified 动作历史
- ✅ FilesRead 取证账本
- ✅ Findings 关键洞察
- ✅ Errors 待解决问题

**可复用**:
- P0 的上下文增强**只需扩展这个函数**,不需要推翻重来。

### 4.4 Live 检查与代际快照

**_subGenSnap** (line 32884-32885) 机制保证了:
- ✅ 父 run 切换时代后子代理能安全退出
- ✅ 避免 stale subagent 污染新回合

**可复用**:
- P2 的异步架构可以直接沿用这套 cancel 机制。

---

## 五、实施路线图

### Phase 0: 立即修复（P0, 本周内）
1. **增强上下文传递** (P0.1)
   - 修改_sharedCtxDigest 增加 fileSnippets 和 verifiedFindings
   - 扩大 findings 窗口和简报长度
2. **赋予子智能体有限命令权** (P0.2)
   - 白名单 run_cmd 只读命令
   - 添加沙箱守卫

**预期效果**: 
- 子智能体减少~30% 重复劳动
- 验证能力增强，结论可信度提升

### Phase 1: 并发体验升级（P1, 2-3 周）
1. **可视化并行子智能体集群** (P1.1)
   - 父子卡片结构
   - 进度条实时更新
2. **智能结果合并** (P1.2)
   - Merge agent + 冲突检测

**预期效果**:
- 并行速度提升~2-3 倍
- 用户体验从"黑盒等待"变为"透明可控"

### Phase 2: 架构演进（P2, 1-2 月）
1. **异步作业架构** (P2.1)
   - JobQueue + 后台 polling
   - 主循环不再阻塞
2. **双向通信通道** (P2.2)
   - SharedStore + EventBus
   - 真正的协做事例

**预期效果**:
- 主智能体吞吐量翻倍（不再傻等）
- 支持复杂多阶段协作流程

---

## 六、风险与注意事项

### 技术风险
1. **Token 爆炸**: 并行 N 个子智能体可能导致 token 消耗指数增长。
   - **缓解**: 成本预警阈值、动态调整 N、上下文压缩优先。
2. **竞态条件**: 多 worker 同时写 run.ctx 可能覆盖彼此结果。
   - **缓解**: 使用 append-only 结构、乐观锁、merge-on-write。
3. **UI 性能**: 大量子卡片可能导致卡顿。
   - **缓解**: 虚拟滚动、按需加载、折叠默认展开。

### 产品风险
1. **用户困惑**: 太多并发任务反而让人眼花。
   - **缓解**: 提供"简化视图"/"高级模式"切换。
2. **成本敏感**: 并行=快速烧钱，用户可能不适应。
   - **缓解**: 明确的价格估算器、预算钳位开关。

### 工程风险
1. **复杂度失控**: P2 架构改动太大，可能引入新 bug。
   - **缓解**: 分阶段发布、充分回归测试、feature flag 渐进 rollout。
2. **维护负担**: 异步架构调试成本高。
   - **缓解**: 完善的日志系统、trace ID 透传、可视化作业图谱。

---

## 七、总结

### 核心发现
1. **子智能体效果差的主因是上下文不足**,不是模型或工具能力问题。
2. **当前支持并发但不可见**,用户体验感知差。
3. **主智能体确实在阻塞等待**,没有真正的异步协作机制。
4. **debate/worktree 是 UI 装饰**,未真正实现。

### 优先级排序
**P0 >> P1 >> P2**
- P0 成本最低、收益立竿见影，应该立即落地。
- P1 是中期优化，提升并发可见性和 merge 质量。
- P2 是大架构改动，需要慎重评估 ROI。

### 建议行动
1. **本周内**: 实施 P0.1+P0.2,改善子智能体上下文和验证能力。
2. **2 周内**: 启动 P1 开发，先做可视化再优化合并。
3. **1 个月后**: 评估 P2 必要性，优先考虑轻量级异步方案（如 promise + polling）而非完整 message bus。

---

## 附录：关键代码位置索引

| 功能 | 函数/变量 | 行号 | 备注 |
|------|----------|------|------|
| 子智能体主函数 | _runSubAgent | 32882 | 核心实现 |
| 子智能体最大步数 | SUB_MAX | 33003 | write=18, read=12 |
| 上下文摘要 | _sharedCtxDigest | 32858 | 应增强 |
| 角色纪律块 | _AGENT_ROLE_BLOCKS | 32768 | 11 个角色定义 |
| 角色注入函数 | _agentRoleBlock | 32818 | 拼接 sysPrompt |
| 段并行执行 | _runOrderedToolSegments | 31612 | Promise.all 关键 |
| Worker 范围守卫 | _activeWorkerScopes | 32930 | 防冲突 |
| Live 检查 | _live | 32885 | 中断机制 |
| 只读工具集 | _READ_TOOLS | 32983 | 18 个工具 |
| 工具映射 | case "run_subagent" | 26337 | 解析调用 |
| 结果回写 | run.ctx.findings | 33109 | 400 字截断 |

---

**报告完成时间**: 2026-07-30  
**调研深度**: 代码级逐行分析  
**验证方法**: grep + Read 双重确认  
**下次复查**: P0 实施后一周  
